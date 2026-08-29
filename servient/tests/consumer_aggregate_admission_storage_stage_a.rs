#![allow(dead_code)]

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
/// bounded cell can host the same semantic enum without self-reference,
/// `union + ManuallyDrop`, or unsafe projection requirements.
struct StaticAdmissionCell {
    slot: Option<AdmissionState>,
}

impl StaticAdmissionCell {
    const fn empty() -> Self {
        Self { slot: None }
    }

    fn initialize(&mut self, state: AdmissionState) -> StaticAdmissionStorage<'_> {
        assert!(self.slot.is_none(), "exclusive static cell may be initialized once");
        self.slot = Some(state);
        StaticAdmissionStorage { cell: self }
    }

    fn is_vacant(&self) -> bool {
        self.slot.is_none()
    }
}

struct StaticAdmissionStorage<'a> {
    cell: &'a mut StaticAdmissionCell,
}

impl AdmissionStorage for StaticAdmissionStorage<'_> {
    fn state(&self) -> &AdmissionState {
        self.cell
            .slot
            .as_ref()
            .expect("static admission cell is initialized while owned")
    }

    fn state_mut(&mut self) -> &mut AdmissionState {
        self.cell
            .slot
            .as_mut()
            .expect("static admission cell is initialized while owned")
    }
}

impl StaticAdmissionStorage<'_> {
    fn release(self) {
        let previous = self.cell.slot.take();
        assert!(previous.is_some(), "static admission cell is already vacant");
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

    let mut cell = StaticAdmissionCell::empty();
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
    assert!(cell.is_vacant());
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

    let mut cell = StaticAdmissionCell::empty();
    let mut static_owner = cell.initialize(AdmissionState::Building {
        identity_count: 2,
        built: 1,
    });
    drive_abort(&mut static_owner);
    assert_eq!(static_owner.state(), &AdmissionState::FailedSettled);
    static_owner.release();
    assert!(cell.is_vacant());
}
