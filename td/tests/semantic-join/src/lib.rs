//! External production TD -> Core plan/artifact witness; not a product coordinator.
//!
//! Raw typed input cannot enter the join, even after synchronous Basic validation:
//! ```compile_fail
//! use clinkz_wot_td::{thing::Thing, validate::Validate};
//! use clinkz_wot_td_semantic_join::{Build, Calls, Limits, registration};
//! let source = Thing::builder("source").id("urn:source").nosec().build().unwrap();
//! source.validate().unwrap();
//! let registration = registration::registration();
//! let calls = Calls::default();
//! let _ = Build::new(&source, &registration, &calls, Limits::default());
//! ```
//!
//! Pending owns the input and registration loans. Only consuming completion (or
//! destruction/terminal failure) can end them:
//! ```compile_fail
//! use clinkz_wot_td::ValidatedPropertyReadCursor;
//! use clinkz_wot_td_semantic_join::{Build, Calls, Limits, registration::Registration};
//! fn invalid<'a>(cursor: ValidatedPropertyReadCursor<'a>, registration: Registration) {
//!     let calls = Calls::default();
//!     let build = Build::new(cursor, &registration, &calls, Limits::default()).unwrap();
//!     drop(registration);
//!     drop(build);
//! }
//! ```
//!
//! The actual production proof carries the caller's input loan into Pending:
//! ```compile_fail
//! use clinkz_wot_foundation::{AdmissionLedger, WorkBudget};
//! use clinkz_wot_td::{thing::Thing, ValidatedThingAdmissionConfig,
//!     ValidatedThingCursor, ValidatedThingProgress};
//! use clinkz_wot_td_semantic_join::{Build, Calls, Limits, registration::Registration};
//! fn invalid(source: Thing, config: &ValidatedThingAdmissionConfig,
//!            ledger: AdmissionLedger, registration: &Registration, calls: &Calls) {
//!     let cursor = ValidatedThingCursor::from_thing(&source, config, ledger);
//!     let proof = match cursor.step(&mut WorkBudget::new(), false) {
//!         ValidatedThingProgress::Complete(proof) => proof,
//!         _ => return,
//!     };
//!     let build = Build::new(proof.into_property_read(), registration, calls, Limits::default()).unwrap();
//!     drop(source);
//!     drop(build);
//! }
//! ```
#![no_std]
extern crate alloc;

// Reuse only the existing complete Core registration authoring adapter. It has
// no TD/Planning dependency, semantic algorithm, input loan or publication path.
#[path = "../../../../tools/architecture-fixtures/validated-thing-planning-handoff/src/registration.rs"]
pub mod registration;

mod join;
pub use join::*;
