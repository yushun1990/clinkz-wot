//! Non-production external Planning handoff witness for WP-100 item 4.
//!
//! The fixture-only view backend connects the existing TD test Snapshot to
//! frozen-API-shaped views. It is not a proposed production storage SPI.
//!
//! The positive external contract compiles with an arbitrary owned source and
//! complete registration destroyed before output selection:
//!
//! ```
//! use validated_thing_planning_handoff_probe::{ViewSource, ValidatedThingView, registration, seal};
//! fn after_drop<S: ViewSource>(source: S) {
//!     let registration = registration();
//!     let draft = seal(ValidatedThingView::from_source(&source), &registration);
//!     drop(source);
//!     drop(registration);
//!     let _ = draft.select("zeta", Some(1));
//! }
//! ```
//!
//! Retaining a nested Form borrow across source destruction is rejected:
//!
//! ```compile_fail,E0505
//! use validated_thing_planning_handoff_probe::{ViewSource, ValidatedThingView};
//! fn dangling_form<S: ViewSource>(source: S) {
//!     let view = ValidatedThingView::from_source(&source);
//!     let form = view.property("zeta").unwrap().form(1).unwrap();
//!     drop(source);
//!     core::hint::black_box(form.resolved_href());
//! }
//! ```
//!
//! A raw/resolved string hidden in an artifact is still a source borrow. A
//! lifetime-free generic envelope alone would not establish an owned payload:
//!
//! ```compile_fail,E0521
//! use validated_thing_planning_handoff_probe::{ValidatedThingView, ValidatedFormHref};
//! use clinkz_wot_core::{BindingArtifact, BindingArtifactCompatibility, BindingArtifactFootprint};
//! fn require_owned<T: 'static>(_: T) {}
//! fn borrowed_artifact(view: ValidatedThingView<'_>) {
//!     let form = view.property("zeta").unwrap().form(1).unwrap();
//!     let ValidatedFormHref::Reference(target) = form.resolved_href().unwrap() else { return };
//!     let artifact = BindingArtifact::new(BindingArtifactCompatibility::new([0; 16]), BindingArtifactFootprint::new(1, 0), target);
//!     require_owned(artifact);
//! }
//! ```
//!
//! A registration callback hidden in an artifact also fails the owned boundary:
//!
//! ```compile_fail,E0521
//! use validated_thing_planning_handoff_probe::Registration;
//! use clinkz_wot_core::{BindingArtifact, BindingArtifactCompatibility, BindingArtifactFootprint};
//! fn require_owned<T: 'static>(_: T) {}
//! fn borrowed_registration(registration: &Registration) {
//!     let callback = move || registration.identity();
//!     let artifact = BindingArtifact::new(BindingArtifactCompatibility::new([0; 16]), BindingArtifactFootprint::new(1, 0), callback);
//!     require_owned(artifact);
//! }
//! ```
//!
//! External consumers cannot retrieve the facade's backend or storage handles:
//!
//! ```compile_fail,E0616
//! use validated_thing_planning_handoff_probe::ValidatedThingView;
//! fn raw_source(view: ValidatedThingView<'_>) {
//!     let _ = view.source;
//! }
//! ```
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod planning;
mod registration;
mod view;

// Compile #111's borrowed query in this external crate as well.
#[path = "../../validated-thing-arena-layout/planning_view_consumer.rs"]
mod borrowed_planning;
pub use borrowed_planning::{PlanningSelection, query_non_first_property_form};

pub use clinkz_wot_core::BindingRegistrationIdentity;
pub use clinkz_wot_td::data_type::Operation;
pub use planning::{Draft, Footprint, FormFacts, Selection, SelectionError, seal};
pub use registration::{Registration, registration};
pub use view::{
    ValidatedFormHref, ValidatedFormHrefError, ValidatedFormView, ValidatedPropertyView,
    ValidatedSecuritySchemeView, ValidatedThingView, ViewSource,
};
