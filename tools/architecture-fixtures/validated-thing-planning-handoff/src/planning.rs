//! Source-independent owned output. TD is observed solely through the view.
use alloc::{boxed::Box, vec::Vec};
use clinkz_wot_core::{
    BindingArtifactEnvelope, BindingArtifactIdentity, BindingArtifactRef, BindingArtifactRole,
    BindingCandidate, BindingCompilerExtension, BindingCompilerInput, BindingCompilerStep,
    BindingRegistrationIdentity, LogicalInteractionPlan, PlanId, PlanSetGeneration, ThingId,
};
use clinkz_wot_foundation::{Generation, SlotIndex};
use clinkz_wot_planning::PlanBuildOutput;
use clinkz_wot_property_read_binding_fixture::MockArtifact;

use crate::{Operation, Registration, ValidatedFormHref, ValidatedThingView};

#[derive(Debug, Eq, PartialEq)]
pub struct FormFacts {
    pub property_ordinal: u32,
    pub raw_href: Box<str>,
    pub content_coding: Option<Box<str>>,
    pub scopes: Box<[Box<str>]>,
    pub security_name: Box<str>,
    pub security_scheme: Box<str>,
}

struct LookupRow {
    name: Box<str>,
    start: usize,
    end: usize,
}

/// Requested bytes of this fixed witness, not the future production PlanFootprint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Footprint {
    pub properties: usize,
    pub plans: usize,
    pub artifact_bytes: u64,
    pub owned_requested_bytes: usize,
}

/// A closed concrete owner: no lifetime, source pointer, storage range, callback,
/// registration owner, or generic borrowed artifact payload.
pub struct Draft {
    output: PlanBuildOutput<MockArtifact>,
    candidates: Box<[BindingCandidate]>,
    facts: Box<[FormFacts]>,
    lookup: Box<[LookupRow]>,
    footprint: Footprint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionError {
    AffordanceMissing,
    NoFormSupportsOperation,
    StrictSelectionMismatch,
    RegistrationMismatch,
}

pub struct Selection<'a> {
    pub plan: &'a LogicalInteractionPlan,
    pub candidate: BindingCandidate,
    pub artifact: &'a BindingArtifactEnvelope<MockArtifact>,
    pub artifact_ref: BindingArtifactRef,
    pub facts: &'a FormFacts,
}

impl Draft {
    pub const fn footprint(&self) -> Footprint {
        self.footprint
    }
    pub fn output(&self) -> &PlanBuildOutput<MockArtifact> {
        &self.output
    }

    /// Uses only sealed lookup ranges and owned plan/artifact data.
    pub fn select(
        &self,
        property: &str,
        form_index: Option<u32>,
    ) -> Result<Selection<'_>, SelectionError> {
        let row = self
            .lookup
            .iter()
            .find(|row| row.name.as_ref() == property)
            .ok_or(SelectionError::AffordanceMissing)?;
        if row.start == row.end {
            return Err(SelectionError::NoFormSupportsOperation);
        }
        let slot = (row.start..row.end)
            .find(|&index| {
                form_index.is_none_or(|original| {
                    self.output.logical_plans()[index].form_index() == original
                })
            })
            .ok_or(SelectionError::StrictSelectionMismatch)?;
        let artifact_ref = self.output.artifact_refs()[slot];
        let artifact = &self.output.artifacts()[slot];
        assert_eq!(artifact_ref.identity(), artifact.identity());
        assert_eq!(
            artifact_ref.artifact_slot(),
            SlotIndex::new(slot.try_into().unwrap())
        );
        Ok(Selection {
            plan: &self.output.logical_plans()[slot],
            candidate: self.candidates[slot],
            artifact,
            artifact_ref,
            facts: &self.facts[slot],
        })
    }
}

impl Selection<'_> {
    /// Separate registration owner is joined only by copied identity/generation.
    pub fn check_registration(
        &self,
        identity: BindingRegistrationIdentity,
    ) -> Result<(), SelectionError> {
        let captured = self.artifact_ref.identity();
        if captured.binding_id() != identity.binding_id()
            || captured.binding_generation() != identity.binding_generation()
            || captured.configuration() != identity.configuration()
            || captured.compatibility() != identity.artifact_compatibility()
        {
            return Err(SelectionError::RegistrationMismatch);
        }
        Ok(())
    }
}

fn concrete(href: ValidatedFormHref<'_>) -> &str {
    match href {
        ValidatedFormHref::Reference(value) => value,
        ValidatedFormHref::Template(_) => panic!("fixed witness uses concrete targets"),
    }
}

/// Fixed-corpus ownership witness, not an admitted aggregate builder. It does
/// not implement preflight, resource reservations, progress, or publication.
pub fn seal(view: ValidatedThingView<'_>, registration: &Registration) -> Draft {
    let thing_id = view.id().expect("witness has an ID");
    let identity = registration.identity();
    assert!(
        registration
            .capabilities()
            .supports_consumer_property_read()
    );
    let compiler = registration.compiler().compiler();
    let mut plans = Vec::new();
    let mut artifacts = Vec::new();
    let mut refs = Vec::new();
    let mut candidates = Vec::new();
    let mut facts = Vec::new();
    let mut lookup = Vec::new();
    let mut artifact_bytes = 0;

    for property in view.properties() {
        let start = plans.len();
        for form in property.forms() {
            if !form
                .effective_operations()
                .any(|operation| operation == Operation::ReadProperty)
            {
                continue;
            }
            let mut names = form.effective_security();
            assert_eq!(names.len(), 1);
            let name = names.next().unwrap();
            let definition = view
                .security_definition(name)
                .expect("TD resolves effective security");
            assert_eq!(definition.scheme(), "nosec");
            let plan_id = PlanId::new(
                SlotIndex::new(plans.len().try_into().unwrap()),
                Generation::INITIAL,
            );
            let plan = LogicalInteractionPlan::try_property_read(
                plan_id,
                ThingId::from(thing_id),
                Box::from(property.name()),
                form.original_index(),
                Box::from(concrete(form.resolved_href().unwrap())),
                Some(Box::from(form.content_type())),
                form.subprotocol().map(Box::from),
            )
            .unwrap();
            let candidate = BindingCandidate::new(
                identity.binding_id(),
                identity.binding_generation(),
                identity.configuration(),
                identity.artifact_compatibility(),
                0,
                0,
            );
            let input =
                BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
            let bounds = compiler.bounds(&input).unwrap();
            let admitted = bounds.artifact();
            let mut budget = bounds.into_work();
            let cursor = compiler.start(&input).unwrap();
            let artifact = match compiler.step(&input, cursor, &mut budget) {
                BindingCompilerStep::Complete(output) => output.into_artifact(),
                _ => panic!("existing fixed mock compiler must complete in its declared bound"),
            };
            artifact_bytes += artifact.footprint().retained_bytes();
            let artifact_identity = BindingArtifactIdentity::new(
                PlanSetGeneration::INITIAL,
                plan_id,
                candidate.binding_id(),
                candidate.binding_generation(),
                candidate.configuration(),
                candidate.compatibility(),
                BindingArtifactRole::ConsumerCall,
            );
            artifacts.push(
                BindingArtifactEnvelope::try_new(artifact_identity, admitted, artifact).unwrap(),
            );
            refs.push(BindingArtifactRef::new(
                artifact_identity,
                SlotIndex::new(plans.len().try_into().unwrap()),
            ));
            candidates.push(candidate);
            facts.push(FormFacts {
                property_ordinal: property.ordinal(),
                raw_href: Box::from(concrete(form.href())),
                content_coding: form.content_coding().map(Box::from),
                scopes: form.scopes().map(Box::from).collect(),
                security_name: Box::from(definition.name()),
                security_scheme: Box::from(definition.scheme()),
            });
            plans.push(plan);
        }
        lookup.push(LookupRow {
            name: Box::from(property.name()),
            start,
            end: plans.len(),
        });
    }

    let candidates: Box<[_]> = candidates.into_boxed_slice();
    let facts: Box<[_]> = facts.into_boxed_slice();
    let lookup: Box<[_]> = lookup.into_boxed_slice();
    let mut owned_requested_bytes = plans.capacity()
        * core::mem::size_of::<LogicalInteractionPlan>()
        + artifacts.capacity() * core::mem::size_of::<BindingArtifactEnvelope<MockArtifact>>()
        + refs.capacity() * core::mem::size_of::<BindingArtifactRef>()
        + core::mem::size_of_val(candidates.as_ref())
        + core::mem::size_of_val(facts.as_ref())
        + core::mem::size_of_val(lookup.as_ref());
    for plan in &plans {
        owned_requested_bytes += plan.thing_id().as_str().len()
            + plan.property_name().len()
            + plan.resolved_target().len()
            + plan.content_type().map_or(0, str::len)
            + plan.subprotocol().map_or(0, str::len);
    }
    for artifact in &artifacts {
        owned_requested_bytes += artifact.artifact().payload().target().unwrap().len();
    }
    for fact in &facts {
        owned_requested_bytes += fact.raw_href.len()
            + fact.content_coding.as_deref().map_or(0, str::len)
            + core::mem::size_of_val(fact.scopes.as_ref())
            + fact.scopes.iter().map(|scope| scope.len()).sum::<usize>()
            + fact.security_name.len()
            + fact.security_scheme.len();
    }
    owned_requested_bytes += lookup.iter().map(|row| row.name.len()).sum::<usize>();
    let footprint = Footprint {
        properties: lookup.len(),
        plans: plans.len(),
        artifact_bytes,
        owned_requested_bytes,
    };
    Draft {
        output: PlanBuildOutput::new(plans, artifacts, refs),
        candidates,
        facts,
        lookup,
        footprint,
    }
}
