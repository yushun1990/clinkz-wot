//! Directory publisher side — lease/revision-aware registration
//! (baseline v4.0 §6 / phase-p1 §1.7).
//!
//! The engine's frozen-TD lifecycle calls only `register` (expose) and
//! `unregister` (destroy); `update`/`renew` exist for directory-service backends
//! and manual registry maintenance (external/admin operators maintaining leases,
//! revisions, and patches against TDs the engine does not own).

use alloc::boxed::Box;

use clinkz_wot_core::ThingId;

use crate::{
    DirectoryPatch, DirectoryRegistration, DiscoveryResult, LeaseState, LeaseToken,
    RegistrationAck, Revision,
};

/// Publisher-side directory service: register/update/unregister TDs with
/// lease and revision tracking.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait DirectoryPublisher: Send + Sync {
    /// Registers a TD (create or replace). Returns the assigned id/revision
    /// and optional lease state.
    async fn register(&self, r: DirectoryRegistration) -> DiscoveryResult<RegistrationAck>;

    /// Renews a lease by its token.
    async fn renew(&self, lease: LeaseToken) -> DiscoveryResult<LeaseState>;

    /// Applies a Merge-Patch to an existing TD, returning the new revision.
    async fn update(&self, id: &ThingId, patch: DirectoryPatch) -> DiscoveryResult<Revision>;

    /// Removes a TD by id.
    async fn unregister(&self, id: &ThingId) -> DiscoveryResult<()>;
}
