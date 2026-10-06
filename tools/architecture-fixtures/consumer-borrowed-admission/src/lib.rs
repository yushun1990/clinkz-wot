//! Non-production external consumer of a TD-private proof, never a raw Thing.
//!
//! The paid proof has no unchecked constructor or raw source getter:
//! ```compile_fail,E0616
//! use consumer_borrowed_admission_probe::td::Validated;
//! fn raw(proof:Validated<'_>){let _=proof.thing;}
//! ```
//! ```compile_fail,E0599
//! use consumer_borrowed_admission_probe::td::Validated;
//! use td_candidate::thing::Thing;
//! fn forge(thing:&Thing){let _=Validated::new(thing);}
//! ```
//! Planning cannot substitute the current raw production PlanBuildInput:
//! ```compile_fail,E0308
//! use consumer_borrowed_admission_probe::planning::{Build,Calls};
//! use clinkz_wot_core::BindingCandidate;
//! use clinkz_wot_property_read_binding_fixture::MockCompiler;
//! use td_candidate::thing::Thing;
//! fn bypass(t:&Thing,c:&MockCompiler,calls:&Calls,candidate:BindingCandidate){
//!     let _=Build::new(t,c,calls,candidate,1000,1000,None);
//! }
//! ```
//! A derived result can only be borrowed for one step; it cannot survive moving
//! the cursor that owns its storage:
//! ```compile_fail,E0505
//! use consumer_borrowed_admission_probe::td::{Validated,Read};
//! use clinkz_wot_foundation::WorkBudget;
//! fn self_reference(proof:Validated<'_>){
//!     let mut read=Read::new(proof);
//!     let event=read.step(&mut WorkBudget::new(),false).unwrap();
//!     let moved=read;
//!     core::hint::black_box(event);
//! }
//! ```
//! The external Thing cannot be destroyed while its continuation is retained:
//! ```compile_fail,E0505
//! use consumer_borrowed_admission_probe::td::{Validation,Policy,Limits};
//! use td_candidate::thing::Thing;
//! fn dangling(t:Thing){
//!     let cursor=Validation::new(&t,Policy::check(Limits::default()).unwrap());
//!     drop(t);core::hint::black_box(cursor.trace());
//! }
//! ```
#![no_std]
extern crate alloc;
pub use td_candidate::borrowed_admission as td;
pub mod planning;
