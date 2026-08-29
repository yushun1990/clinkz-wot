#![allow(dead_code)]

use std::mem::MaybeUninit;

/// Non-production storage-topology proof for workspace/0063.
///
/// The semantic transaction state is one safe Rust enum. Host and constrained
/// profiles differ only in who provides the storage: Host owns a heap cell;
/// application-static receives one exclusive caller-provided cell. Neither
/// representation changes the state graph or exposes a replaceable cursor.
#[derive(Debug, Eq, PartialEq)]
enum AdmissionState {
    Captured { source_token: u32 },
    Validating { source_token: u32, cursor: u32 },
    Enumerating { source_token: u32, coordinate_count: u32 },
    Identified { coordinate_count: u32, identity_count: u32 },
    Bounded { identity_count: u32, required_bytes: u64 },
    Reserved { identity_count: u32, reserved_bytes: u64 },
    Building { identity_count: u32, built: u32 },
    Reconciling { identity_count: u32, built: u32 },
    Frozen { identity_count: u32, built: u32 },
    Aborting { identity_count: u32, cleanup_remaining: u32 },
    FailedSettled,
}

trait AdmissionStorage {
    fn state(&self) -> &AdmissionState;
    fn state_mut(&mut self) -> &mut AdmissionState;
}

struct HostAdmissionStorage {
    state: Box<AdmissionState>,
}

impl HostAdmissionStorage {
    fn new(initial: AdmissionState) -> Self {
        Self {
            state: Box::new(initial),
        }
    }
}

impl AdmissionStorage for HostAdmissionStorage {
    fn state(&self) -> &AdmissionState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut AdmissionState {
        &mut self.state
    }
}

/// Caller-owned constrained cell. The future production representation may be
/// an arena/table slot instead; this fixture proves only that an exclusive
/// bounded cell can host the same semantic enum without self-reference or
/// `union + ManuallyDrop` requirements.
struct StaticAdmissionCell {
    slot: MaybeUninit<AdmissionState>,
    initialized: bool,
}

impl StaticAdmissionCell {
    const fn uninit() -> Self {
        Self {
            slot: MaybeUninit::uninit(),
            initialized: false,
        }
    }

    fn initialize(&mut self, state: AdmissionState) -> StaticAdmissionStorage<'_> {
        assert!(!self.initialized, "exclusive static cell may be initialized once");
        self.slot.write(state);
        self.initialized = true;
        StaticAdmissionStorage { cell: self }
    }
}

impl Drop for StaticAdmissionCell {
    fn drop(&mut self) {
        if self.initialized {
            // SAFETY: `initialized` is set only after `slot.write` and is reset
            // by `StaticAdmissionStorage::release` after dropping the value.
            unsafe { self.slot.assume_init_drop() };
            self.initialized = false;
        }
    }
}

struct StaticAdmissionStorage<'a> {
    cell: &'a mut StaticAdmissionCell,
}

impl AdmissionStorage for StaticAdmissionStorage<'_> {
    fn state(&self) -> &AdmissionState {
        // SAFETY: this owner is created only by `initialize`, which writes the
        // slot before returning the exclusive borrow.
        unsafe { self.cell.slot.assume_init_ref() }
    }

    fn state_mut(&mut self) -> &mut AdmissionState {
        // SAFETY: `StaticAdmissionStorage` holds the unique borrow of the cell.
        unsafe { self.cell.slot.assume_init_mut() }
    }
}

impl StaticAdmissionStorage<'_> {
    fn release(mut self) {
        assert!(self.cell.initialized);
        // SAFETY: the slot is initialized and this owner has unique access.
        unsafe { self.cell.slot.assume_init_drop() };
        self.cell.initialized = false;
    }
}

fn drive_same_semantics(storage: &mut impl AdmissionStorage) {
    assert_eq!(
        storage.state(),
        &AdmissionState::Captured { source_token: 7 }
    );
    *storage.state_mut() = AdmissionState::Validating {
        source_token: 7,
        cursor: 1,
    };
    *storage.state_mut() = AdmissionState::Enumerating {
        source_token: 7,
        coordinate_count: 2,
    };
    *storage.state_mut() = AdmissionState::Identified {
        coordinate_count: 2,
        identity_count: 2,
    };
    *storage.state_mut() = AdmissionState::Bounded {
        identity_count: 2,
        required_bytes: 128,
    };
    *storage.state_mut() = AdmissionState::Reserved {
        identity_count: 2,
        reserved_bytes: 128,
    };
    *storage.state_mut() = AdmissionState::Building {
        identity_count: 2,
        built: 2,
    };
    *storage.state_mut() = AdmissionState::Reconciling {
        identity_count: 2,
        built: 2,
    };
    *storage.state_mut() = AdmissionState::Frozen {
        identity_count: 2,
        built: 2,
    };
}

#[test]
fn host_and_static_backends_preserve_the_same_semantic_state_graph() {
    let initial = || AdmissionState::Captured { source_token: 7 };

    let mut host = HostAdmissionStorage::new(initial());
    drive_same_semantics(&mut host);
    assert_eq!(
        host.state(),
        &AdmissionState::Frozen {
            identity_count: 2,
            built: 2
        }
    );

    let mut cell = StaticAdmissionCell::uninit();
    let mut static_owner = cell.initialize(initial());
    drive_same_semantics(&mut static_owner);
    assert_eq!(
        static_owner.state(),
        &AdmissionState::Frozen {
            identity_count: 2,
            built: 2
        }
    );
    static_owner.release();
    assert!(!cell.initialized);
}

#[test]
fn aborting_is_a_real_state_in_both_storage_backends() {
    fn drive_abort(storage: &mut impl AdmissionStorage) {
        *storage.state_mut() = AdmissionState::Aborting {
            identity_count: 2,
            cleanup_remaining: 1,
        };
        *storage.state_mut() = AdmissionState::FailedSettled;
    }

    let mut host = HostAdmissionStorage::new(AdmissionState::Building {
        identity_count: 2,
        built: 1,
    });
    drive_abort(&mut host);
    assert_eq!(host.state(), &AdmissionState::FailedSettled);

    let mut cell = StaticAdmissionCell::uninit();
    let mut static_owner = cell.initialize(AdmissionState::Building {
        identity_count: 2,
        built: 1,
    });
    drive_abort(&mut static_owner);
    assert_eq!(static_owner.state(), &AdmissionState::FailedSettled);
    static_owner.release();
}
