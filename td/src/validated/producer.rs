//! Whole-description Producer lending. No supported-leaf selection occurs here.
use super::*;
use crate::{
    affordance::{ActionAffordance, EventAffordance},
    td_defaults::{FormContext, default_operations_for_context},
};

/// Original declaration scope, including affordances with no Forms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedProducerScope<'a> {
    Thing,
    Property(&'a str),
    Action(&'a str),
    Event(&'a str),
}
#[derive(Clone, Copy)]
enum Owner<'a> {
    Thing,
    Property(&'a str, &'a PropertyAffordance),
    Action(&'a str, &'a ActionAffordance),
    Event(&'a str, &'a EventAffordance),
}
impl<'a> Owner<'a> {
    fn scope(self) -> ValidatedProducerScope<'a> {
        match self {
            Self::Thing => ValidatedProducerScope::Thing,
            Self::Property(n, _) => ValidatedProducerScope::Property(n),
            Self::Action(n, _) => ValidatedProducerScope::Action(n),
            Self::Event(n, _) => ValidatedProducerScope::Event(n),
        }
    }
    fn context(self) -> FormContext<'a> {
        match self {
            Self::Thing => FormContext::Thing,
            Self::Property(_, p) => FormContext::Property(p),
            Self::Action(_, a) => FormContext::Action(a),
            Self::Event(_, e) => FormContext::Event(e),
        }
    }
    fn forms(self, thing: &'a Thing) -> Option<&'a [Form]> {
        match self {
            Self::Thing => thing.forms.as_deref(),
            Self::Property(_, p) => Some(&p._interaction.forms),
            Self::Action(_, a) => Some(&a._interaction.forms),
            Self::Event(_, e) => Some(&e._interaction.forms),
        }
    }
}
enum Entries<'a> {
    Properties(btree_map::Iter<'a, String, PropertyAffordance>),
    Actions(btree_map::Iter<'a, String, ActionAffordance>),
    Events(btree_map::Iter<'a, String, EventAffordance>),
}
impl<'a> Entries<'a> {
    fn next(&mut self) -> Option<Owner<'a>> {
        match self {
            Self::Properties(i) => i.next().map(|(n, p)| Owner::Property(n, p)),
            Self::Actions(i) => i.next().map(|(n, a)| Owner::Action(n, a)),
            Self::Events(i) => i.next().map(|(n, e)| Owner::Event(n, e)),
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Start,
    ScopePay,
    Scope,
    Forms,
    Resolve,
    Allocate,
    Uri,
    Finish,
    Form,
    Next,
    Done,
}

/// Move-only continuation of complete Basic admission, with the same ledger
/// and nonrenewable source-work allowance. Ready loans require acknowledgement.
pub struct ValidatedProducerCursor<'td> {
    owner: ValidatedThingCursor<'td>,
    stage: Stage,
    cause: Option<Cause>,
    category: u8,
    entries: Option<Entries<'td>>,
    scope: Owner<'td>,
    ordinal: usize,
    form: usize,
    last: Option<(Owner<'td>, u32)>,
    alias: Option<&'td str>,
    resolver: Option<uri::Resolution<'td>>,
    scratch: UriBlock,
    effective: u64,
}
pub enum ValidatedProducerStep<'a> {
    Pending,
    Ready(ValidatedProducerEvent<'a>),
    Done,
}
pub enum ValidatedProducerEvent<'a> {
    Scope {
        scope: ValidatedProducerScope<'a>,
        forms: Option<usize>,
    },
    Form(ValidatedProducerForm<'a>),
}
/// Effective operations retain the complete set, including explicitly empty
/// sets. Security overrides also preserve explicit emptiness.
pub struct ValidatedProducerForm<'a> {
    pub scope: ValidatedProducerScope<'a>,
    pub index: u32,
    pub declared_operations: Option<&'a [Operation]>,
    pub effective_operations: &'a [Operation],
    pub effective_security: &'a [String],
    pub resolved_href: &'a str,
}
impl<'td> ValidatedThing<'td> {
    pub fn into_producer(mut self) -> ValidatedProducerCursor<'td> {
        self.owner.phase = Phase::Semantics;
        let effective = self.owner.counts.content;
        ValidatedProducerCursor {
            owner: self.owner,
            stage: Stage::Start,
            cause: None,
            category: 0,
            entries: None,
            scope: Owner::Thing,
            ordinal: 0,
            form: 0,
            last: None,
            alias: None,
            resolver: None,
            scratch: UriBlock::default(),
            effective,
        }
    }
}
impl<'td> ValidatedProducerCursor<'td> {
    pub fn acknowledge(&mut self) {
        match self.stage {
            Stage::Scope => self.stage = Stage::Forms,
            Stage::Form => {
                self.form += 1;
                self.stage = Stage::Forms;
            }
            _ => {}
        }
    }
    pub fn step(
        &mut self,
        budget: &mut WorkBudget,
        cancel: bool,
    ) -> Result<ValidatedProducerStep<'_>, Cause> {
        if let Some(cause) = self.cause {
            return Err(cause);
        }
        if cancel {
            let cause = Cause::Cancelled {
                phase: Phase::Semantics,
            };
            self.cause = Some(cause);
            return Err(cause);
        }
        if !matches!(self.stage, Stage::Scope | Stage::Form | Stage::Done) {
            if budget.is_exhausted() {
                return Ok(ValidatedProducerStep::Pending);
            }
            if let Err(cause) = self.tick(budget) {
                self.cause = Some(cause);
                return Err(cause);
            }
        }
        Ok(match self.stage {
            Stage::Done => ValidatedProducerStep::Done,
            Stage::Scope => ValidatedProducerStep::Ready(ValidatedProducerEvent::Scope {
                scope: self.scope.scope(),
                forms: self.scope.forms(self.owner.thing).map(<[Form]>::len),
            }),
            Stage::Form => {
                let form = self.current_form();
                ValidatedProducerStep::Ready(ValidatedProducerEvent::Form(ValidatedProducerForm {
                    scope: self.scope.scope(),
                    index: self.form as u32,
                    declared_operations: form.op.as_deref(),
                    effective_operations: form
                        .op
                        .as_deref()
                        .unwrap_or_else(|| default_operations_for_context(self.scope.context())),
                    effective_security: crate::td_defaults::effective_form_security(
                        self.owner.thing,
                        form,
                    ),
                    resolved_href: self.alias.unwrap_or_else(|| self.scratch.text()),
                }))
            }
            _ => ValidatedProducerStep::Pending,
        })
    }
    fn current_form(&self) -> &'td Form {
        &self.scope.forms(self.owner.thing).unwrap()[self.form]
    }
    fn tick(&mut self, budget: &mut WorkBudget) -> Result<(), Cause> {
        let nodes = if self.stage == Stage::Next {
            // Pay iterator creation as well as advance across category changes.
            iter_cost(
                self.owner
                    .thing
                    .properties
                    .as_ref()
                    .map_or(0, BTreeMap::len)
                    .max(self.owner.thing.actions.as_ref().map_or(0, BTreeMap::len))
                    .max(self.owner.thing.events.as_ref().map_or(0, BTreeMap::len)),
            )
        } else {
            1
        };
        let key_bytes = if matches!(self.stage, Stage::ScopePay | Stage::Finish) {
            match self.scope {
                Owner::Thing => 0,
                Owner::Property(n, _) | Owner::Action(n, _) | Owner::Event(n, _) => n.len() as u64,
            }
        } else {
            0
        };
        let costs = match self.stage {
            Stage::Uri => [
                (W::UriBytes, self.resolver.as_ref().unwrap().work()),
                (
                    W::CodecOutputBytes,
                    self.resolver.as_ref().unwrap().output_work(),
                ),
            ],
            Stage::Allocate => [(W::DocumentNodes, nodes), (W::CleanupItems, 1)],
            _ => [(W::DocumentNodes, nodes), (W::CodecInputBytes, key_bytes)],
        };
        if !self.owner.pay(budget, &costs)? {
            return Ok(());
        }
        match self.stage {
            Stage::Start => self.stage = Stage::ScopePay,
            Stage::ScopePay => self.stage = Stage::Scope,
            Stage::Forms => {
                if self.form == self.scope.forms(self.owner.thing).map_or(0, <[Form]>::len) {
                    if self.category != 0 {
                        self.ordinal += 1;
                    }
                    self.stage = Stage::Next;
                } else {
                    self.stage = Stage::Resolve;
                }
            }
            Stage::Next => {
                if let Some(owner) = self.entries.as_mut().and_then(Entries::next) {
                    self.scope = owner;
                    self.form = 0;
                    self.stage = Stage::ScopePay;
                } else {
                    self.category += 1;
                    self.ordinal = 0;
                    self.entries = match self.category {
                        1 => self
                            .owner
                            .thing
                            .properties
                            .as_ref()
                            .map(|m| Entries::Properties(m.iter())),
                        2 => self
                            .owner
                            .thing
                            .actions
                            .as_ref()
                            .map(|m| Entries::Actions(m.iter())),
                        3 => self
                            .owner
                            .thing
                            .events
                            .as_ref()
                            .map(|m| Entries::Events(m.iter())),
                        _ => {
                            self.stage = Stage::Done;
                            None
                        }
                    };
                }
            }
            Stage::Resolve => {
                match uri::Target::new(self.owner.thing.base.as_ref(), &self.current_form().href)
                    .map_err(|_| self.uri_error())?
                {
                    uri::Target::Alias(raw) => {
                        self.alias = Some(raw);
                        self.stage = Stage::Finish;
                    }
                    uri::Target::Resolve(r) => {
                        self.alias = None;
                        self.resolver = Some(r);
                        self.stage = Stage::Allocate;
                    }
                }
            }
            Stage::Allocate => {
                let href = self.current_form().href.as_str();
                self.owner
                    .allocate_uri_block(&mut self.scratch, href, producer_inline_bytes())?;
                self.stage = Stage::Uri;
            }
            Stage::Uri => {
                if self.resolver.as_mut().unwrap().tick(&mut self.scratch) {
                    self.stage = Stage::Finish;
                }
            }
            Stage::Finish => {
                let length = self.alias.map_or(self.scratch.len, str::len) as u64;
                self.owner
                    .policy
                    .check(R::UriTemplateSourceBytesMax, length, Phase::Semantics)?;
                if self.alias.is_none() {
                    self.effective = add(self.effective, length, Phase::Semantics)?;
                }
                self.owner.policy.check(
                    R::GeneratedEffectiveDocumentBytesMax,
                    self.effective,
                    Phase::Semantics,
                )?;
                self.last = Some((self.scope, self.form as u32));
                self.stage = Stage::Form;
            }
            Stage::Scope | Stage::Form | Stage::Done => unreachable!(),
        }
        Ok(())
    }
    fn uri_error(&self) -> Cause {
        let kind = match self.scope {
            Owner::Thing => b::OwnerKind::Thing,
            Owner::Property(..) => b::OwnerKind::Property,
            Owner::Action(..) => b::OwnerKind::Action,
            Owner::Event(..) => b::OwnerKind::Event,
        };
        let mut site = b::Site::new(
            b::Owner {
                kind,
                ordinal: self.ordinal,
            },
            b::Field::FormHref,
        );
        site.index = self.form;
        Cause::Invalid(ValidatedThingInvalid {
            kind: ValidatedThingInvalidKind::InvalidUri,
            phase: Phase::Semantics,
            node_ordinal: self.ordinal as u64,
            site,
            rule: Rule::Uri,
        })
    }

    /// Resolves the current paid Property Form (or the final one after Done).
    /// The callback cannot retain a source/arena loan. This is semantic input,
    /// not an installation permit or certification of a Binding's support.
    /// Arena backing is charged to this original ledger until physical Drop,
    /// including callback failure or unwinding.
    #[cfg(feature = "std")]
    pub fn with_property_semantics<T: 'static>(
        &mut self,
        property: &str,
        index: u32,
        budget: &mut WorkBudget,
        lower: impl FnOnce(&crate::semantic::ResolvedSemanticDocument<'_>, &str, &mut WorkBudget) -> T,
    ) -> Result<Option<T>, ValidatedSemanticError> {
        use crate::semantic::{ResolvedSemanticDocument as Document, SemanticError};
        if let Some(cause) = self.cause {
            return Err(ValidatedSemanticError::Admission(cause));
        }
        if !matches!(self.stage, Stage::Form | Stage::Done) {
            return Err(ValidatedSemanticError::Resolution(
                SemanticError::UnsupportedDependency,
            ));
        }
        let key_bytes = match self.last {
            Some((Owner::Property(name, _), _)) => name.len() as u64,
            _ => 0,
        };
        if !self
            .owner
            .pay(
                budget,
                &[
                    (W::DocumentNodes, crate::semantic::RESOLUTION_WORK),
                    (W::CleanupItems, 2),
                    (W::CodecInputBytes, key_bytes),
                ],
            )
            .map_err(ValidatedSemanticError::Admission)?
        {
            return Ok(None);
        }
        if !matches!(self.last, Some((Owner::Property(name, _), i)) if name == property && i == index)
        {
            return Err(ValidatedSemanticError::Resolution(
                SemanticError::UnsupportedDependency,
            ));
        }
        let bytes = Document::capacity_bytes();
        let live = add(
            self.owner.stack.ledger.live_bytes(),
            bytes,
            Phase::Semantics,
        )
        .and_then(|n| add(n, producer_inline_bytes(), Phase::Semantics))
        .map_err(ValidatedSemanticError::Admission)?;
        for kind in [
            R::AdmissionTemporaryBytesPerOperationMax,
            R::AdmissionTemporaryBytesGlobalMax,
            R::PeakLiveBytesPerAdmissionMax,
            R::AdmissionPeakLiveBytesGlobalMax,
            R::EngineLiveBytesGlobalMax,
        ] {
            self.owner
                .policy
                .check(kind, live, Phase::Semantics)
                .map_err(ValidatedSemanticError::Admission)?;
        }
        self.owner
            .policy
            .check(
                R::LargestContiguousAllocationBytesMax,
                bytes,
                Phase::Semantics,
            )
            .map_err(ValidatedSemanticError::Admission)?;
        let reservation = self
            .owner
            .stack
            .ledger
            .try_reserve_temporary(R::AdmissionTemporaryBytesPerOperationMax, bytes)
            .ok_or(ValidatedSemanticError::Resolution(
                SemanticError::Allocation,
            ))?;
        let (document, used) = Document::resolve_metered(self.owner.thing, property, index);
        let unused = crate::semantic::RESOLUTION_WORK - used;
        budget.set_remaining(
            W::DocumentNodes,
            budget.remaining(W::DocumentNodes) + unused,
        );
        self.owner.remaining += unused;
        let document = document.map_err(ValidatedSemanticError::Resolution)?;
        let result = lower(
            &document,
            self.alias.unwrap_or_else(|| self.scratch.text()),
            budget,
        );
        drop(document);
        drop(reservation);
        Ok(Some(result))
    }
}
fn producer_inline_bytes() -> u64 {
    2 * mem::size_of::<ValidatedProducerCursor<'static>>() as u64
}
impl Drop for ValidatedProducerCursor<'_> {
    fn drop(&mut self) {
        self.scratch.release(&mut self.owner.stack.ledger);
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use clinkz_wot_foundation::{GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile};
    fn work() -> WorkBudget {
        W::ALL
            .into_iter()
            .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
    }
    fn producer(thing: &Thing, temporary: u64) -> ValidatedProducerCursor<'_> {
        let config =
            ValidatedThingAdmissionConfig::try_from_limits(&GatewayDefaultV1::LIMITS).unwrap();
        let ledger = AdmissionLedger::new(
            SlotIndex::new(1),
            Generation::INITIAL,
            0,
            temporary,
            0,
            0,
            0,
            0,
        );
        let mut c = ValidatedThingCursor::from_thing(thing, &config, ledger);
        let mut cursor = loop {
            match c.step(&mut work(), false) {
                ValidatedThingProgress::Pending(next) => c = next,
                ValidatedThingProgress::Complete(proof) => break proof.into_producer(),
                ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
            }
        };
        loop {
            match cursor.step(&mut work(), false).unwrap() {
                ValidatedProducerStep::Done => break,
                ValidatedProducerStep::Ready(_) => cursor.acknowledge(),
                _ => {}
            }
        }
        cursor
    }
    #[test]
    fn arena_and_uri_release_keep_original_ledger_on_return_refusal_and_unwind() {
        let thing: Thing = serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"paid",
          "base":"http://127.0.0.1:38080/base/","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},
          "properties":{"":{"type":"number","readOnly":true,"forms":[{"href":"../value"}]}}}"#).unwrap();
        let mut c = producer(&thing, 8 << 20);
        let baseline = c.owner.stack.ledger.live_bytes();
        assert!(baseline > 0);
        let remaining = c.owner.remaining;
        assert!(
            c.with_property_semantics::<()>("", 0, &mut WorkBudget::new(), |_, _, _| panic!())
                .unwrap()
                .is_none()
        );
        assert_eq!(c.owner.remaining, remaining);
        let result = c
            .with_property_semantics("", 0, &mut work(), |doc, target, _| {
                assert_eq!(doc.property().name, "");
                assert_eq!(target, "http://127.0.0.1:38080/value");
                false
            })
            .unwrap();
        assert_eq!(result, Some(false));
        assert_eq!(c.owner.stack.ledger.live_bytes(), baseline);
        assert!(c.owner.remaining < remaining);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            c.with_property_semantics::<()>("", 0, &mut work(), |_, _, _| panic!("lowering failed"))
        }));
        assert!(panic.is_err());
        assert_eq!(c.owner.stack.ledger.live_bytes(), baseline);
        let uri_bytes = c.scratch.capacity as u64;
        c.scratch.release(&mut c.owner.stack.ledger);
        assert_eq!(c.owner.stack.ledger.live_bytes(), baseline - uri_bytes);
        assert!(c.step(&mut work(), true).is_err());
        assert!(matches!(
            c.with_property_semantics::<()>("", 0, &mut work(), |_, _, _| panic!()),
            Err(ValidatedSemanticError::Admission(Cause::Cancelled {
                phase: Phase::Semantics
            }))
        ));

        // The source was admitted under this exact small original account;
        // the larger semantic arena must not borrow an unrelated fresh ledger.
        let mut c = producer(
            &thing,
            baseline + crate::semantic::ResolvedSemanticDocument::capacity_bytes() - 1,
        );
        assert!(
            c.with_property_semantics::<()>("", 0, &mut work(), |_, _, _| panic!("quota bypass"))
                .is_err()
        );
        assert_eq!(c.owner.stack.ledger.live_bytes(), baseline);
    }

    #[test]
    fn every_suspended_uri_and_scope_action_is_inert_at_zero_credit() {
        let thing: Thing = serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"paid",
          "base":"http://127.0.0.1:38080/a/b/","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},
          "properties":{"x":{"type":"number","forms":[{"href":"../../value?q=1"}]}}}"#).unwrap();
        let config =
            ValidatedThingAdmissionConfig::try_from_limits(&GatewayDefaultV1::LIMITS).unwrap();
        let ledger = AdmissionLedger::new(
            SlotIndex::new(1),
            Generation::INITIAL,
            0,
            8 << 20,
            0,
            0,
            0,
            0,
        );
        let mut c = ValidatedThingCursor::from_thing(&thing, &config, ledger);
        let mut c = loop {
            match c.step(&mut work(), false) {
                ValidatedThingProgress::Pending(next) => c = next,
                ValidatedThingProgress::Complete(proof) => break proof.into_producer(),
                ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
            }
        };
        for _ in 0..10_000 {
            let before = (
                c.stage,
                c.owner.remaining,
                c.owner.stack.ledger.live_bytes(),
            );
            assert!(matches!(
                c.step(&mut WorkBudget::new(), false).unwrap(),
                ValidatedProducerStep::Pending
            ));
            assert!(
                before
                    == (
                        c.stage,
                        c.owner.remaining,
                        c.owner.stack.ledger.live_bytes()
                    )
            );
            match c.step(&mut work(), false).unwrap() {
                ValidatedProducerStep::Done => return,
                ValidatedProducerStep::Ready(_) => c.acknowledge(),
                _ => {}
            }
        }
        panic!("Producer did not progress");
    }
}
