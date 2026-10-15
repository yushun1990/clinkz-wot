//! ADR-0023 controlled boundary discriminators, not successor production proof.
//! Harness containers/RCs are baseline: this is no whole-process memory census.
use clinkz_wot_core::Payload;
use clinkz_wot_foundation::{Generation, ResourceAccount, ResourceKind, SlotIndex};
use std::{cell::RefCell, rc::Rc};

type Bank = Rc<RefCell<ResourceAccount>>;

fn bank(limit: u64) -> Bank {
    Rc::new(RefCell::new(ResourceAccount::new(
        SlotIndex::new(0),
        Generation::INITIAL,
        ResourceKind::EngineLiveBytesGlobalMax,
        limit,
    )))
}

// The same linear numeric primitive can implement an optional private policy.
// A private bank is deliberately distinct from the engine's real parent bank.
struct Charge {
    local: ResourceAccount,
    parent: Bank,
    bytes: u64,
}
impl Charge {
    fn acquire(parent: &Bank, owner: u32, bytes: u64) -> Option<Self> {
        let mut local = ResourceAccount::new(
            SlotIndex::new(owner),
            Generation::INITIAL,
            ResourceKind::EngineLiveBytesGlobalMax,
            bytes,
        );
        parent.borrow_mut().try_reserve(bytes)?.commit();
        local.try_reserve(bytes).unwrap().commit();
        Some(Self {
            local,
            parent: parent.clone(),
            bytes,
        })
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        assert!(self.local.release_committed(self.bytes));
        assert!(self.parent.borrow_mut().release_committed(self.bytes));
    }
}

struct Storage {
    bytes: Box<[u8]>,
    // Field destruction order releases actual backing before local/parent charge.
    _charge: Charge,
}
impl Storage {
    fn try_new(parent: &Bank, owner: u32, bytes: usize) -> Option<Self> {
        let charge = Charge::acquire(parent, owner, bytes as u64)?;
        Some(Self {
            bytes: vec![0; bytes].into_boxed_slice(),
            _charge: charge,
        })
    }
}

struct Session {
    storage: Option<Storage>,
    retired: Rc<RefCell<bool>>,
}
impl Drop for Session {
    fn drop(&mut self) {
        drop(self.storage.take());
        *self.retired.borrow_mut() = true;
    }
}
struct Binding {
    id: u32,
    session: Rc<Session>,
    private: Bank,
}
impl Binding {
    fn start(&self, ordinary: &Bank, cleanup: &Bank, request: Box<u8>) -> Result<Call, Box<u8>> {
        let Some(carrier) = Storage::try_new(ordinary, self.id, 16) else {
            return Err(request);
        };
        let Some(destination) = Storage::try_new(cleanup, self.id, 8) else {
            return Err(request);
        };
        Ok(Call {
            owner: self.id,
            private: self.private.clone(),
            carrier,
            destination,
            request,
            result: Some(42),
            protocol: None,
            session: self.session.clone(),
        })
    }
}
struct Call {
    // Protocol and shared-owner leases retire before engine carrier/fallback.
    protocol: Option<Storage>,
    session: Rc<Session>,
    owner: u32,
    private: Bank,
    request: Box<u8>,
    result: Option<u8>,
    carrier: Storage,
    destination: Storage,
}
impl Call {
    fn grow(&mut self, bytes: usize) -> bool {
        // Reserve replacement plus still-live old backing before allocation.
        let Some(next) = Storage::try_new(&self.private, self.owner, bytes) else {
            return false;
        };
        self.protocol = Some(next);
        true
    }
}

fn bindings() -> (Binding, Binding, Bank, Rc<RefCell<bool>>) {
    let private = bank(16);
    let retired = Rc::new(RefCell::new(false));
    let session = Rc::new(Session {
        storage: Some(Storage::try_new(&private, 99, 8).unwrap()),
        retired: retired.clone(),
    });
    (
        Binding {
            id: 1,
            session: session.clone(),
            private: private.clone(),
        },
        Binding {
            id: 2,
            session,
            private: private.clone(),
        },
        private,
        retired,
    )
}

#[test]
fn private_pressure_retains_live_call_result_and_funded_cleanup_then_reuses_capacity() {
    let (a, b, private, _) = bindings();
    let ordinary = bank(32);
    let cleanup = bank(16);
    let mut first = a.start(&ordinary, &cleanup, Box::new(1)).unwrap();
    let mut second = b.start(&ordinary, &cleanup, Box::new(2)).unwrap();
    assert!(first.grow(8));
    assert!(!second.grow(1));
    assert_eq!(second.result, Some(42));
    assert_eq!(second.carrier.bytes.len(), 16);
    assert_eq!(second.destination.bytes.len(), 8);
    assert_eq!(*second.request, 2);
    assert_eq!(ordinary.borrow().used(), 32);
    assert_eq!(cleanup.borrow().used(), 16);
    assert_eq!(private.borrow().used(), 16);
    drop(first);
    assert_eq!(ordinary.borrow().used(), 16);
    assert_eq!(cleanup.borrow().used(), 8);
    assert!(second.grow(8));
    drop(second);
    assert_eq!(ordinary.borrow().used(), 0);
    assert_eq!(cleanup.borrow().used(), 0);
    assert_eq!(private.borrow().used(), 8); // Only the real shared session remains.
}

#[test]
fn engine_rejection_and_cleanup_saturation_preserve_exact_input_and_parent_totals() {
    let (a, b, _, _) = bindings();
    let ordinary = bank(32);
    let cleanup = bank(8);
    let call = a.start(&ordinary, &cleanup, Box::new(1)).unwrap();
    let input = Box::new(2);
    let address = &*input as *const u8;
    let returned = b.start(&ordinary, &cleanup, input).err().unwrap();
    assert_eq!(&*returned as *const u8, address);
    assert_eq!(ordinary.borrow().used(), 16); // Partial ordinary reservation rolled back.
    assert_eq!(cleanup.borrow().used(), 8);
    // Saturating ordinary work cannot consume the cleanup bank.
    let ordinary_pressure = Storage::try_new(&ordinary, 3, 16).unwrap();
    let input = Box::new(3);
    let address = &*input as *const u8;
    let returned = b.start(&ordinary, &cleanup, input).err().unwrap();
    assert_eq!(&*returned as *const u8, address);
    assert_eq!(call.destination.bytes.len(), 8);
    drop(ordinary_pressure);
    drop(call);
    assert!(b.start(&ordinary, &cleanup, returned).is_ok());
}

#[test]
fn private_growth_accounts_replacement_overlap_and_rejects_without_losing_old_backing() {
    let (a, _, private, _) = bindings();
    let mut call = a.start(&bank(16), &bank(8), Box::new(1)).unwrap();
    assert!(call.grow(6));
    let address = call.protocol.as_ref().unwrap().bytes.as_ptr();
    assert!(!call.grow(4)); // 8 shared + 6 old + 4 new exceeds 16.
    assert_eq!(call.protocol.as_ref().unwrap().bytes.as_ptr(), address);
    assert_eq!(private.borrow().used(), 14);
    assert!(call.grow(2)); // Exact overlap limit; old six released afterward.
    assert_eq!(private.borrow().used(), 10);
}

#[test]
fn shared_infrastructure_is_charged_once_and_one_cancel_does_not_retire_another_user() {
    let (a, b, private, retired) = bindings();
    let ordinary = bank(32);
    let cleanup = bank(16);
    let first = a.start(&ordinary, &cleanup, Box::new(1)).unwrap();
    let second = b.start(&ordinary, &cleanup, Box::new(2)).unwrap();
    assert!(Rc::ptr_eq(&first.session, &second.session));
    assert_eq!(private.borrow().used(), 8);
    drop(a);
    drop(first); // Controlled immediate local cancellation, no remote claim.
    assert!(!*retired.borrow());
    assert_eq!(second.result, Some(42));
    drop(b);
    assert!(!*retired.borrow());
    drop(second);
    assert!(*retired.borrow());
    assert_eq!(private.borrow().used(), 0);
}

// This prototype checks the real media representation, not a length-only fake.
// Body Arc layout/control-block proof remains a production completion obligation.
fn supported_payload(payload: &Payload, body: usize, media: usize) -> bool {
    payload.body.len() <= body
        && payload.content_type.capacity() <= media
        && payload
            .content_coding
            .as_ref()
            .is_none_or(|s| s.capacity() <= media)
}

#[test]
fn handoff_refuses_spare_media_capacity_and_excess_body_bytes() {
    let mut media = String::with_capacity(128);
    media.push('x');
    let payload = Payload::new(vec![1_u8], media);
    assert_eq!(payload.content_type.len(), 1);
    assert!(!supported_payload(&payload, 1, 127));
    assert!(supported_payload(&payload, 1, 128));
    assert!(!supported_payload(&payload, 0, 128));
    let mut coding = String::with_capacity(256);
    coding.push('x');
    assert!(!supported_payload(
        &payload.with_content_coding(coding),
        1,
        128
    ));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Contract {
    StrictV1,
    ConsumerOwnedV2,
    ConsumerOwnedHostV2,
}

#[test]
fn contract_matching_and_host_disposal_applicability_are_distinct() {
    fn eligible(policy: Contract, registration: Contract, host: bool, allocating: bool) -> bool {
        policy == registration
            && (!allocating || (host && policy == Contract::ConsumerOwnedHostV2))
            && (host || policy != Contract::ConsumerOwnedHostV2)
    }
    assert!(!eligible(
        Contract::StrictV1,
        Contract::ConsumerOwnedV2,
        true,
        false
    ));
    assert!(!eligible(
        Contract::ConsumerOwnedV2,
        Contract::StrictV1,
        true,
        false
    ));
    assert!(!eligible(
        Contract::ConsumerOwnedV2,
        Contract::ConsumerOwnedV2,
        true,
        true
    ));
    assert!(!eligible(
        Contract::ConsumerOwnedHostV2,
        Contract::ConsumerOwnedHostV2,
        false,
        true
    ));
    assert!(eligible(
        Contract::ConsumerOwnedHostV2,
        Contract::ConsumerOwnedHostV2,
        true,
        true
    ));
}

#[test]
fn host_allocating_local_disposal_observes_precommitted_residual_and_live_engine_funding() {
    struct Sdk {
        residual: Rc<RefCell<bool>>,
        disposed: Rc<RefCell<bool>>,
        ordinary: Bank,
        cleanup: Bank,
    }
    impl Drop for Sdk {
        fn drop(&mut self) {
            assert!(*self.residual.borrow());
            assert_eq!(self.ordinary.borrow().used(), 16);
            assert_eq!(self.cleanup.borrow().used(), 8);
            // One finite SDK-local allocation, distinct from engine status commit.
            let local = Box::new([0_u8; 8]);
            std::hint::black_box(&local);
            drop(local);
            *self.disposed.borrow_mut() = true;
        }
    }
    let (a, _, _, _) = bindings();
    let ordinary = bank(16);
    let cleanup = bank(8);
    let call = a.start(&ordinary, &cleanup, Box::new(1)).unwrap();
    let residual = Rc::new(RefCell::new(false));
    let disposed = Rc::new(RefCell::new(false));
    let sdk = Sdk {
        residual: residual.clone(),
        disposed: disposed.clone(),
        ordinary: ordinary.clone(),
        cleanup: cleanup.clone(),
    };
    *residual.borrow_mut() = true; // Fixed prebacked engine commitment, no SDK call.
    drop(sdk);
    assert!(*disposed.borrow());
    drop(call);
    assert_eq!(ordinary.borrow().used(), 0);
    assert_eq!(cleanup.borrow().used(), 0);
}
