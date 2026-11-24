//! Source-independent facts used by the Host publication decision.
use crate::{CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass};
use alloc::{sync::Arc, vec::Vec};
use std::sync::{OnceLock, RwLock, RwLockReadGuard};

/// A configuration or local readiness fact with one writer at a time. Owners
/// expose shared reads and replace the value under their own synchronization.
/// Every replacement invalidates observations, including a change back to an
/// earlier value. Interior changes affecting publication need their own fact.
pub struct PublicationState<T> {
    value: T,
    validity: OnceLock<Arc<RwLock<bool>>>,
}

impl<T> PublicationState<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            validity: OnceLock::new(),
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn replace(&mut self, value: T) -> T {
        self.invalidate();
        core::mem::replace(&mut self.value, value)
    }

    fn invalidate(&mut self) {
        if let Some(validity) = self.validity.take() {
            // The only overlapping readers here are in the callback-free local
            // decision. No provider or native call is made under those locks.
            *validity.write().unwrap() = false;
        }
    }

    pub fn observe(&self, facts: &mut PublicationObserver<'_>) -> CoreResult<()> {
        let validity = self.validity.get_or_init(|| Arc::new(RwLock::new(true)));
        if !facts.observed.iter().any(|old| Arc::ptr_eq(old, validity)) {
            facts.observed.try_reserve(1).map_err(|_| busy())?;
            facts.observed.push(validity.clone());
        }
        Ok(())
    }
}

impl<T> Drop for PublicationState<T> {
    fn drop(&mut self) {
        self.invalidate();
    }
}

impl<T: Default> Default for PublicationState<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// Every mutable dependency which contributed to one qualification. Observe
/// the owner before taking its first answer. Fixed registrations and exclusively
/// owned operands require no observation.
#[derive(Default)]
pub struct PublicationFacts {
    observed: Vec<Arc<RwLock<bool>>>,
}

/// Append-only loan to one contributing registration or native owner.
pub struct PublicationObserver<'a> {
    observed: &'a mut Vec<Arc<RwLock<bool>>>,
}

impl PublicationFacts {
    /// Start a new qualification with the same observed versions. The original
    /// proof can be retained across a short or refused qualification turn.
    pub fn try_clone(&self) -> CoreResult<Self> {
        let mut observed = Vec::new();
        observed
            .try_reserve_exact(self.observed.len())
            .map_err(|_| busy())?;
        observed.extend(self.observed.iter().cloned());
        Ok(Self { observed })
    }

    pub fn observer(&mut self) -> PublicationObserver<'_> {
        PublicationObserver {
            observed: &mut self.observed,
        }
    }

    /// Additional storage for this proof and its simultaneous commit guards.
    pub fn backing_bytes(&self) -> Option<usize> {
        let references = self
            .observed
            .capacity()
            .checked_mul(size_of::<Arc<RwLock<bool>>>())?;
        let per_version = size_of::<RwLockReadGuard<'_, bool>>()
            + size_of::<RwLock<bool>>()
            + 2 * size_of::<usize>();
        references.checked_add(self.observed.len().checked_mul(per_version)?)
    }

    /// Acquire only at a callback-free decision. All writers of observed state
    /// must pass these same locks to replace the effective value.
    pub fn lock_current<'a>(groups: &[&'a Self]) -> CoreResult<PublicationGuards<'a>> {
        let count = groups
            .iter()
            .try_fold(0usize, |n, g| n.checked_add(g.observed.len()))
            .ok_or_else(busy)?;
        let mut guards = Vec::new();
        guards.try_reserve_exact(count).map_err(|_| busy())?;
        for observed in groups.iter().flat_map(|group| &group.observed) {
            let guard = observed.try_read().map_err(|_| busy())?;
            if !*guard {
                return Err(CoreError::StaleHandle(context()));
            }
            guards.push(guard);
        }
        Ok(PublicationGuards { _guards: guards })
    }
}

#[must_use]
pub struct PublicationGuards<'a> {
    _guards: Vec<RwLockReadGuard<'a, bool>>,
}

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{TryLockError, mpsc},
        thread,
    };

    #[test]
    fn commit_pins_all_observed_versions_and_replacing_or_dropping_an_owner_invalidates_them() {
        let mut selected = PublicationState::new(1);
        let competitor = PublicationState::new(false);
        let mut preparation = PublicationFacts::default();
        let mut installation = PublicationFacts::default();
        selected.observe(&mut preparation.observer()).unwrap();
        competitor.observe(&mut installation.observer()).unwrap();

        let commit = PublicationFacts::lock_current(&[&preparation, &installation]).unwrap();
        thread::scope(|scope| {
            let (attempted, waiting) = mpsc::channel();
            let (updated, done) = mpsc::channel();
            let selected = &mut selected;
            let competitor = &competitor;
            scope.spawn(move || {
                for owner in [
                    selected.validity.get().unwrap(),
                    competitor.validity.get().unwrap(),
                ] {
                    assert!(matches!(owner.try_write(), Err(TryLockError::WouldBlock)));
                }
                attempted.send(()).unwrap();
                selected.replace(2);
                selected.replace(1);
                updated.send(()).unwrap();
            });
            waiting.recv().unwrap();
            assert!(matches!(done.try_recv(), Err(mpsc::TryRecvError::Empty)));
            drop(commit);
            done.recv().unwrap();
        });
        assert!(matches!(
            PublicationFacts::lock_current(&[&preparation]),
            Err(CoreError::StaleHandle(_))
        ));
        drop(competitor);
        assert!(matches!(
            PublicationFacts::lock_current(&[&installation]),
            Err(CoreError::StaleHandle(_))
        ));
    }
}
