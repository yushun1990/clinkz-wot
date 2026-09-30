//! Connect the established frozen view prototype to a real external crate.
//! This adapter delegates queries; it introduces no TD semantic rule.
extern crate std;

use super::{Operation, Snapshot, ValidatedFormHref, ValidatedFormHrefError, ValidatedThingView};
use crate::validate::{Validate, ValidationLevel};
use alloc::vec;
use validated_thing_planning_handoff_probe as handoff;

impl Snapshot {
    fn contract_form(&self, property: u32, form: u32) -> super::ValidatedFormView<'_> {
        ValidatedThingView::new(self)
            .properties()
            .nth(property as usize)
            .unwrap()
            .form(form)
            .unwrap()
    }
}

impl handoff::ViewSource for Snapshot {
    fn id(&self) -> Option<&str> {
        ValidatedThingView::new(self).id()
    }
    fn property_count(&self) -> usize {
        ValidatedThingView::new(self).properties().len()
    }
    fn property_name(&self, property: u32) -> &str {
        ValidatedThingView::new(self)
            .properties()
            .nth(property as usize)
            .unwrap()
            .name()
    }
    fn form_count(&self, property: u32) -> usize {
        ValidatedThingView::new(self)
            .properties()
            .nth(property as usize)
            .unwrap()
            .forms()
            .len()
    }
    fn href(&self, property: u32, form: u32) -> handoff::ValidatedFormHref<'_> {
        convert_href(self.contract_form(property, form).href())
    }
    fn resolved_href(
        &self,
        property: u32,
        form: u32,
    ) -> Result<handoff::ValidatedFormHref<'_>, handoff::ValidatedFormHrefError> {
        self.contract_form(property, form)
            .resolved_href()
            .map(convert_href)
            .map_err(|error| match error {
                ValidatedFormHrefError::TemplateBase => {
                    handoff::ValidatedFormHrefError::TemplateBase
                }
                ValidatedFormHrefError::Resolution => handoff::ValidatedFormHrefError::Resolution,
            })
    }
    fn content_type(&self, property: u32, form: u32) -> &str {
        self.contract_form(property, form).content_type()
    }
    fn content_coding(&self, property: u32, form: u32) -> Option<&str> {
        self.contract_form(property, form).content_coding()
    }
    fn subprotocol(&self, property: u32, form: u32) -> Option<&str> {
        self.contract_form(property, form).subprotocol()
    }
    fn scope_count(&self, property: u32, form: u32) -> usize {
        self.contract_form(property, form).scopes().len()
    }
    fn scope(&self, property: u32, form: u32, index: usize) -> &str {
        self.contract_form(property, form)
            .scopes()
            .nth(index)
            .unwrap()
    }
    fn operation_count(&self, property: u32, form: u32) -> usize {
        self.contract_form(property, form)
            .effective_operations()
            .len()
    }
    fn operation(&self, property: u32, form: u32, index: usize) -> handoff::Operation {
        // Unit-test TD and the dependency TD are distinct crate instances.
        // This exhaustive vocabulary bridge changes no operation semantics.
        match self
            .contract_form(property, form)
            .effective_operations()
            .nth(index)
            .unwrap()
        {
            Operation::ReadProperty => handoff::Operation::ReadProperty,
            Operation::WriteProperty => handoff::Operation::WriteProperty,
            Operation::ObserveProperty => handoff::Operation::ObserveProperty,
            Operation::UnobserveProperty => handoff::Operation::UnobserveProperty,
            Operation::InvokeAction => handoff::Operation::InvokeAction,
            Operation::QueryAction => handoff::Operation::QueryAction,
            Operation::CancelAction => handoff::Operation::CancelAction,
            Operation::SubscribeEvent => handoff::Operation::SubscribeEvent,
            Operation::UnsubscribeEvent => handoff::Operation::UnsubscribeEvent,
            Operation::ReadAllProperties => handoff::Operation::ReadAllProperties,
            Operation::WriteAllProperties => handoff::Operation::WriteAllProperties,
            Operation::ReadMultipleProperties => handoff::Operation::ReadMultipleProperties,
            Operation::WriteMultipleProperties => handoff::Operation::WriteMultipleProperties,
            Operation::ObserveAllProperties => handoff::Operation::ObserveAllProperties,
            Operation::UnobserveAllProperties => handoff::Operation::UnobserveAllProperties,
            Operation::QueryAllActions => handoff::Operation::QueryAllActions,
            Operation::SubscribeAllEvents => handoff::Operation::SubscribeAllEvents,
            Operation::UnsubscribeAllEvents => handoff::Operation::UnsubscribeAllEvents,
        }
    }
    fn security_count(&self, property: u32, form: u32) -> usize {
        self.contract_form(property, form)
            .effective_security()
            .len()
    }
    fn security(&self, property: u32, form: u32, index: usize) -> &str {
        self.contract_form(property, form)
            .effective_security()
            .nth(index)
            .unwrap()
    }
    fn security_definition(&self, name: &str) -> Option<(&str, &str)> {
        ValidatedThingView::new(self)
            .security_definition(name)
            .map(|definition| (definition.name(), definition.scheme()))
    }
}

fn convert_href(href: ValidatedFormHref<'_>) -> handoff::ValidatedFormHref<'_> {
    match href {
        ValidatedFormHref::Reference(value) => handoff::ValidatedFormHref::Reference(value),
        ValidatedFormHref::Template(value) => handoff::ValidatedFormHref::Template(value),
    }
}

fn assert_static<T: 'static>(_: &T) {}

#[test]
fn external_planning_owned_output_survives_input_snapshot_and_registration_drop() {
    let mut thing = super::super::typed_corpus_shared::typed_corpus();
    // Alpha is present but not readable; a later Property supplies a second
    // owned plan. Zeta's first Form is filtered out, so original Form index 1
    // must remain 1 even though its retained plan/artifact slot is zero.
    let other = super::alpha_property_mut(&mut thing).clone();
    thing
        .properties
        .as_mut()
        .unwrap()
        .insert("zz_other".into(), other);
    super::alpha_property_mut(&mut thing)._interaction.forms[0].op =
        Some(vec![Operation::WriteProperty]);
    super::zeta_form_mut(&mut thing, 0).op = Some(vec![Operation::WriteProperty]);
    super::zeta_form_mut(&mut thing, 1).op = None;
    super::zeta_form_mut(&mut thing, 1).content_coding = Some("gzip".into());
    super::zeta_form_mut(&mut thing, 1).subprotocol = Some("fixture-subprotocol".into());
    super::zeta_form_mut(&mut thing, 1).scopes = Some(vec!["scope-b".into(), "scope-a".into()]);
    assert!(super::validate_security_references(&thing).is_ok());
    thing.validate_with_level(ValidationLevel::Basic).unwrap();
    let snapshot = Snapshot::normalize(&thing);
    // Compatibility input is actually destroyed before Planning even starts.
    drop(thing);
    let source_bytes = snapshot.arena.footprint().retained_requested_bytes as usize;
    assert_eq!(snapshot.arena.footprint().retained_allocation_count, 3);
    assert_eq!(snapshot.arena.ledger().live_bytes(), source_bytes as u64);
    let registration = handoff::registration();
    assert_static(&registration);

    let ((), query) = super::trace_allocations(|| {
        let view = handoff::ValidatedThingView::from_source(&snapshot);
        let selected = handoff::query_non_first_property_form(view);
        assert_eq!(selected.property_ordinal, 1);
        assert_eq!(selected.property_count, 3);
        assert_eq!(selected.form_original_index, 1);
        assert_eq!(
            selected.resolved_href,
            handoff::ValidatedFormHref::Reference("https://example.org/things/zeta/second")
        );
        assert_eq!(selected.content_coding, Some("gzip"));
        assert_eq!(selected.subprotocol, Some("fixture-subprotocol"));
        assert_eq!(selected.scope_count, 2);
        assert_eq!(selected.security_scheme, "nosec");
    });
    assert_eq!(query.allocation_calls, 0);

    let (draft, build) = super::trace_allocations(|| {
        let view = handoff::ValidatedThingView::from_source(&snapshot);
        handoff::seal(view, &registration)
    });
    assert_static(&draft);
    let footprint = draft.footprint();
    assert_eq!(footprint.properties, 3);
    assert_eq!(footprint.plans, 2);
    assert!(build.allocation_calls > 0);
    assert_eq!(build.live_change, footprint.owned_requested_bytes as i64);
    assert!(build.peak_additional_bytes >= footprint.owned_requested_bytes);
    let overlap_peak = source_bytes + build.peak_additional_bytes;
    assert!(overlap_peak >= source_bytes + footprint.owned_requested_bytes);

    // All nested views and compiler/registration input borrows ended at seal.
    // Drop the real three-arena owner while the full output remains resident.
    let sites = [
        snapshot.arena.nodes().as_ptr() as usize,
        snapshot.arena.edges().as_ptr() as usize,
        snapshot.arena.bytes().as_ptr() as usize,
    ];
    let ((), release) = super::trace_allocations_watching(sites, || drop(snapshot));
    assert_eq!(release.allocation_calls, 0);
    assert_eq!(release.deallocation_calls, 3);
    assert_eq!(release.watched_deallocations, [1, 1, 1]);
    assert_eq!(release.deallocated_bytes, source_bytes);
    assert_eq!(release.live_change, -(source_bytes as i64));

    let ((), selection) = super::trace_allocations(|| {
        let selected = draft.select("zeta", Some(1)).unwrap();
        assert_eq!(
            selected.plan.thing_id().as_str(),
            "urn:example:typed-corpus"
        );
        assert_eq!(selected.plan.property_name(), "zeta");
        assert_eq!(selected.plan.form_index(), 1);
        assert_eq!(selected.artifact_ref.artifact_slot().get(), 0);
        assert_eq!(
            selected.plan.plan_id(),
            selected.artifact_ref.identity().plan_id()
        );
        assert!(selected.artifact.route_reservation().is_none());
        assert_eq!(selected.facts.property_ordinal, 1);
        assert_eq!(selected.facts.raw_href.as_ref(), "zeta/second");
        assert_eq!(
            selected.plan.resolved_target(),
            "https://example.org/things/zeta/second"
        );
        assert_eq!(
            selected.artifact.artifact().payload().target(),
            Some(selected.plan.resolved_target())
        );
        assert_eq!(selected.plan.content_type(), Some("application/json"));
        assert_eq!(selected.plan.subprotocol(), Some("fixture-subprotocol"));
        assert_eq!(selected.facts.content_coding.as_deref(), Some("gzip"));
        assert_eq!(selected.facts.scopes[0].as_ref(), "scope-b");
        assert_eq!(selected.facts.scopes[1].as_ref(), "scope-a");
        assert_eq!(selected.facts.security_name.as_ref(), "none");
        assert_eq!(selected.facts.security_scheme.as_ref(), "nosec");
        assert_eq!(selected.candidate.registration_ordinal(), 0);
        assert_eq!(selected.candidate.candidate_order(), 0);
        selected
            .check_registration(registration.identity())
            .unwrap();
        assert_eq!(draft.select("zeta", None).unwrap().plan.form_index(), 1);
        assert!(matches!(
            draft.select("zeta", Some(0)),
            Err(handoff::SelectionError::StrictSelectionMismatch)
        ));
        let later = draft.select("zz_other", None).unwrap();
        assert_eq!(later.plan.property_name(), "zz_other");
        assert_eq!(later.facts.property_ordinal, 2);
        assert_eq!(later.plan.form_index(), 0);
        assert_eq!(later.artifact_ref.artifact_slot().get(), 1);
        assert!(matches!(
            draft.select("zeta", Some(9)),
            Err(handoff::SelectionError::StrictSelectionMismatch)
        ));
        assert!(matches!(
            draft.select("alpha", None),
            Err(handoff::SelectionError::NoFormSupportsOperation)
        ));
        assert!(matches!(
            draft.select("missing", None),
            Err(handoff::SelectionError::AffordanceMissing)
        ));
        assert_eq!(draft.footprint(), footprint);
        assert_eq!(
            footprint.artifact_bytes,
            draft
                .output()
                .artifacts()
                .iter()
                .map(|artifact| artifact.artifact().footprint().retained_bytes())
                .sum::<u64>()
        );
        let old = registration.identity();
        let stale = handoff::BindingRegistrationIdentity::new(
            old.binding_id(),
            old.binding_generation().checked_next().unwrap(),
            old.configuration(),
            old.artifact_compatibility(),
            0,
        );
        assert_eq!(
            selected.check_registration(stale),
            Err(handoff::SelectionError::RegistrationMismatch)
        );
    });
    assert_eq!(selection.allocation_calls, 0);
    assert_eq!(selection.deallocation_calls, 0);

    // The draft also has no hidden registration borrow; it can outlive that
    // owner even though runtime publication would retain it separately.
    drop(registration);
    let ((), after_registration) = super::trace_allocations(|| {
        let selected = draft.select("zeta", Some(1)).unwrap();
        assert_eq!(
            selected.artifact_ref.identity(),
            selected.artifact.identity()
        );
        assert_eq!(
            selected.artifact.artifact().payload().target(),
            Some(selected.plan.resolved_target())
        );
        assert_eq!(draft.output().logical_plans().len(), footprint.plans);
    });
    assert_eq!(after_registration.allocation_calls, 0);
    assert_eq!(after_registration.deallocation_calls, 0);
    let ((), output_release) = super::trace_allocations(|| drop(draft));
    assert_eq!(output_release.allocation_calls, 0);
    assert_eq!(
        output_release.deallocated_bytes,
        footprint.owned_requested_bytes
    );
    assert_eq!(
        output_release.live_change,
        -(footprint.owned_requested_bytes as i64)
    );
    std::eprintln!(
        "handoff requested-byte witness: source={source_bytes}, owned={}, build_overlap_peak={overlap_peak}, source_release_sites={:?}",
        footprint.owned_requested_bytes,
        release.watched_deallocations
    );
}
