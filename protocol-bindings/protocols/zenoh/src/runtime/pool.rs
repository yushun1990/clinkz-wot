//! Per-authority zenoh session aggregation.
//!
//! Implements the binding-template §5.2 `ZenohSessionPool`: lazily opens one
//! `zenoh::Session` per distinct TD-resolved authority, caches it, and routes
//! each interaction to the session for its plan's authority. This lets a
//! Consumer reach Things on multiple zenoh routers from one Servient.
//!
//! Available only with the `zenoh` feature.

use alloc::{boxed::Box, format, string::String, sync::Arc};

use std::collections::HashMap;

use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, InteractionOutput, RetryClass, Subscription,
    SubscriptionGuard,
};
use zenoh::Wait;

use crate::{ZenohOperationPlan, ZenohSessionTransport, ZenohTransport, ZenohTransportRequest};

const DEFAULT_REPLY_TIMEOUT: core::time::Duration = core::time::Duration::from_secs(5);

/// Builds a [`zenoh::Config`] for a planned zenoh operation.
///
/// Implementations translate the resolved transport and authority into a
/// connect-mode configuration ready for [`zenoh::open`]. Applications inject a
/// custom policy to add TLS, credentials, or custom locators (binding template
/// §5.4).
pub trait ZenohSessionPolicy: Send + Sync {
    /// Returns a zenoh config that connects to the router named by
    /// `plan.transport` / `plan.authority`.
    fn config_for(&self, plan: &ZenohOperationPlan) -> CoreResult<zenoh::Config>;
}

/// Default policy: plain `<transport>/<authority>` connect (e.g.
/// `tcp/router-a:7447`), no authentication.
///
/// Suitable for development and `nosec` deployments. Applications that need
/// TLS, PSK, or mTLS provide their own [`ZenohSessionPolicy`] implementation.
#[derive(Debug, Default, Clone)]
pub struct DefaultSessionPolicy;

impl ZenohSessionPolicy for DefaultSessionPolicy {
    fn config_for(&self, plan: &ZenohOperationPlan) -> CoreResult<zenoh::Config> {
        let mut config = zenoh::Config::default();
        let locator = format!("{}/{}", plan.transport, plan.authority);
        config
            .insert_json5("connect/endpoints", &format!("[\"{locator}\"]"))
            .map_err(|_| validation_error())?;
        Ok(config)
    }
}

/// Aggregates zenoh sessions keyed by router authority.
///
/// Replaces the single-session [`ZenohSessionTransport`] as the default
/// multi-router `std` backend. Each distinct authority encountered in a
/// consumed TD's resolved form target gets its own lazily-opened session;
/// subsequent interactions with the same authority reuse the cached session
/// (binding template §5.2).
pub struct ZenohSessionPool {
    sessions: std::sync::Mutex<HashMap<String, Arc<ZenohSessionTransport>>>,
    policy: Arc<dyn ZenohSessionPolicy>,
    reply_timeout: core::time::Duration,
}

impl core::fmt::Debug for ZenohSessionPool {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ZenohSessionPool")
            .field("reply_timeout", &self.reply_timeout)
            .finish_non_exhaustive()
    }
}

impl ZenohSessionPool {
    /// Creates a pool with the given session policy and a default 5 s reply
    /// timeout.
    pub fn new(policy: Arc<dyn ZenohSessionPolicy>) -> Self {
        Self {
            sessions: std::sync::Mutex::new(HashMap::new()),
            policy,
            reply_timeout: DEFAULT_REPLY_TIMEOUT,
        }
    }

    /// Sets the maximum time to wait for one query or subscription reply on
    /// every session opened by this pool.
    pub fn with_reply_timeout(mut self, timeout: core::time::Duration) -> Self {
        self.reply_timeout = timeout;
        self
    }

    /// Returns the number of cached sessions.
    pub fn len(&self) -> usize {
        self.sessions.lock().expect("pool mutex poisoned").len()
    }

    /// Returns `true` if no sessions are cached.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the cached session for `authority`, or lazily opens one via the
    /// policy. Thread-safe: concurrent callers for the same authority share a
    /// single session; a losing racer drops its duplicate.
    fn get_or_open(&self, plan: &ZenohOperationPlan) -> CoreResult<Arc<ZenohSessionTransport>> {
        let authority = &plan.authority;

        // Fast path: read-lock the map and return a cached session.
        {
            let sessions = self.sessions.lock().expect("pool mutex poisoned");
            if let Some(transport) = sessions.get(authority) {
                return Ok(Arc::clone(transport));
            }
        }

        // Slow path: build config and open a new session.
        let config = self.policy.config_for(plan)?;
        let session = zenoh::open(config).wait().map_err(|_| binding_error())?;
        let transport =
            Arc::new(ZenohSessionTransport::new(session).with_reply_timeout(self.reply_timeout));

        // Insert under the lock, but re-check: another thread may have won the
        // race and inserted first. The loser's duplicate session drops on the
        // floor (zenoh::Session closes itself on drop).
        let mut sessions = self.sessions.lock().expect("pool mutex poisoned");
        if let Some(existing) = sessions.get(authority) {
            return Ok(Arc::clone(existing));
        }
        sessions.insert(authority.clone(), Arc::clone(&transport));
        Ok(transport)
    }

    /// Explicitly closes and removes the cached session for `authority`.
    pub fn shutdown_authority(&self, authority: &str) {
        let mut sessions = self.sessions.lock().expect("pool mutex poisoned");
        sessions.remove(authority);
    }

    /// Closes and removes all cached sessions.
    pub fn shutdown_all(&self) {
        let mut sessions = self.sessions.lock().expect("pool mutex poisoned");
        sessions.clear();
    }
}

fn validation_error() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Prepare, RetryClass::Never))
}

fn binding_error() -> CoreError {
    CoreError::Binding(ErrorContext::new(ErrorPhase::Binding, RetryClass::Safe))
}

impl ZenohTransport for ZenohSessionPool {
    fn execute(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        let transport = self.get_or_open(&request.plan)?;
        transport.execute(request)
    }

    fn open_subscription(
        &self,
        request: ZenohTransportRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        let transport = self.get_or_open(&request.plan)?;
        transport.open_subscription(request)
    }
}
