//! Pure HTTP Action lowering. No socket, behavior or logical permission is
//! obtained here; every described operation has its own selected definition.
use crate::{HttpActionDefinition, HttpProducerConfig, Target, producer::producer_unsupported};
use clinkz_wot_core::{BindingGeneration, BindingId, CoreResult, Operation};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_planning::{
    producer_composition::{FormSite, ProducerCapability, ProducerOperationInput},
    type_schema::prepare_type_schema,
};
use clinkz_wot_td::semantic::{FormOwner, SemanticScope, SemanticValue, UriVariableScope};
use std::{any::Any, sync::Arc};

/// Actual HTTP registration, independent of any particular logical Thing.
pub struct HttpActionProducer {
    listen: HttpProducerConfig,
    identity: (BindingId, BindingGeneration),
}
impl HttpActionProducer {
    pub fn new(listen: HttpProducerConfig, id: BindingId, generation: BindingGeneration) -> Self {
        Self {
            listen,
            identity: (id, generation),
        }
    }

    fn lower(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<HttpActionDefinition>> {
        let FormSite::Action(name) = input.site else {
            return Ok(None);
        };
        let (method, control) = match input.operation {
            Operation::InvokeAction => ("POST", false),
            Operation::QueryAction => ("GET", true),
            Operation::CancelAction => ("DELETE", true),
            _ => return Ok(None),
        };
        // Eligibility is native and must precede HTTP-specific schema/security
        // refusals. Another registered Binding may legitimately own a different
        // Action Form (or a different control mapping of this same Action).
        let base = if control {
            input.target.strip_suffix("{?id}").unwrap_or(input.target)
        } else {
            input.target
        };
        let Ok(target) = Target::parse(base) else {
            return Ok(None);
        };
        if target.address != self.listen.listen_address() {
            return Ok(None);
        }
        if control && !input.target.ends_with("{?id}") {
            return Err(producer_unsupported());
        }
        budget
            .consume(WorkClass::BindingPolls, 4096)
            .map_err(|_| producer_unsupported())?;
        let doc = input.document;
        let form = doc
            .form(FormOwner::Action(name), input.form)
            .ok_or_else(producer_unsupported)?;
        if form.effective_operations != [input.operation]
            || form.content_type != "application/json"
            || form.subprotocol.is_some()
            || form.content_coding.is_some()
            || form.scopes.is_some()
            || form.additional_responses
            || form
                .response_content_type
                .is_some_and(|m| m != "application/json")
            || doc.action_synchronous(name) == Some(Some(true))
            || doc.uri_variables(UriVariableScope::Thing).is_some()
            || doc.scope(SemanticScope::Thing).is_none_or(|n| {
                n.children()
                    .any(|n| !crate::producer_compiler::class_assertion(n))
            })
        {
            return Err(producer_unsupported());
        }
        let action_scope = doc
            .scope(SemanticScope::Action(name))
            .ok_or_else(producer_unsupported)?;
        for role in ["input", "output"] {
            let iri = if role == "input" {
                "https://www.w3.org/2019/wot/td#input"
            } else {
                "https://www.w3.org/2019/wot/td#output"
            };
            if action_scope.children().filter(|n| n.name().is(iri)).count()
                != usize::from(
                    doc.affordance_schema(FormOwner::Action(name), role)
                        .is_some(),
                )
            {
                return Err(producer_unsupported());
            }
        }
        if action_scope.children().any(|n| {
            !crate::producer_compiler::class_assertion(n)
                && !n.name().is("https://www.w3.org/2019/wot/td#input")
                && !n.name().is("https://www.w3.org/2019/wot/td#output")
        }) {
            return Err(producer_unsupported());
        }
        match doc.uri_variables(UriVariableScope::Action) {
            None if !control => {}
            Some(vars) => {
                let mut vars = vars;
                let Some(("id", schema)) = vars.next() else {
                    return Err(producer_unsupported());
                };
                if vars.next().is_some()
                    || prepare_type_schema(schema)?
                        != clinkz_wot_core::value_type::TypeSchema::Type(
                            clinkz_wot_core::value_type::ValueType::String,
                        )
                {
                    return Err(producer_unsupported());
                }
            }
            _ => return Err(producer_unsupported()),
        }
        let [security_name] = form.effective_security else {
            return Err(producer_unsupported());
        };
        let security = doc
            .security(security_name)
            .map_err(|_| producer_unsupported())?;
        if security.scheme != "nosec"
            || security.proxy.is_some()
            || security
                .semantics
                .children()
                .any(|n| !crate::producer_compiler::class_assertion(n))
        {
            return Err(producer_unsupported());
        }
        let input_schema = doc
            .affordance_schema(FormOwner::Action(name), "input")
            .map(prepare_type_schema)
            .transpose()?;
        let output_schema = doc
            .affordance_schema(FormOwner::Action(name), "output")
            .map(prepare_type_schema)
            .transpose()?;
        let form_scope = doc
            .scope(SemanticScope::AffordanceForm {
                owner: FormOwner::Action(name),
                index: input.form,
            })
            .ok_or_else(producer_unsupported)?;
        let mut seen_method = false;
        for node in form_scope.children() {
            if seen_method
                || !node.name().is("http://www.w3.org/2011/http#methodName")
                || node.value() != SemanticValue::Text(method)
            {
                return Err(producer_unsupported());
            }
            seen_method = true;
        }
        if let Some(response) = doc.scope(SemanticScope::AffordanceResponse {
            owner: FormOwner::Action(name),
            index: input.form,
        }) {
            let mut seen = false;
            for node in response.children() {
                if seen
                    || !node
                        .name()
                        .is("http://www.w3.org/2011/http#statusCodeValue")
                    || !matches!(node.value(), SemanticValue::Number(n) if n.as_u64() == Some(if control {200} else {202}))
                {
                    return Err(producer_unsupported());
                }
                seen = true;
            }
        }
        let path = target.uri.path_and_query().map_or("/", |p| p.as_str());
        if path.len() > 64 || (control && path.contains('?')) {
            return Err(producer_unsupported());
        }
        Ok(Some(HttpActionDefinition {
            instance_binding: self.identity,
            action: Arc::from(name.as_str()),
            operation: input.operation,
            form: input.form,
            address: target.address,
            path: Arc::from(path),
            input_schema,
            output_schema,
            control,
        }))
    }
}
impl ProducerCapability for HttpActionProducer {
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.identity
    }
    fn definition_bytes(&self, definition: &dyn Any) -> Option<usize> {
        definition.downcast_ref::<HttpActionDefinition>().map(|d| {
            std::mem::size_of::<HttpActionDefinition>()
                + d.action.len()
                + d.path.len()
                + 4 * std::mem::size_of::<usize>()
        })
    }
    fn prepare(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>> {
        Ok(self
            .lower(input, budget)?
            .map(|d| Box::new(d) as Box<dyn Any>))
    }
    fn supports(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool> {
        Ok(self.lower(input, budget)?.is_some())
    }
    fn final_check_temporary_bytes(&self) -> Option<usize> {
        Some(8192)
    }
    fn check_final(
        &self,
        input: ProducerOperationInput<'_>,
        definition: &dyn Any,
        budget: &mut WorkBudget,
    ) -> CoreResult<bool> {
        let Some(expected) = definition.downcast_ref::<HttpActionDefinition>() else {
            return Ok(false);
        };
        Ok(self.lower(input, budget)?.as_ref() == Some(expected))
    }
}
