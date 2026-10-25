//! Optional pure preparation. HTTP instructions never enter Engine code.
use super::{producer::*, *};
use clinkz_wot_planning::ResolvedProducerInput;
use clinkz_wot_planning::producer_execution::{ProducerBindingPreparation, ProducerCompiler};

impl ProducerCompiler<HttpProducerExecutor> for HttpProducerConfig {
    fn configuration(&self) -> &HttpProducerConfig {
        self
    }
    fn compile(
        &self,
        input: ResolvedProducerInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<HttpProducerProgram>> {
        self.compile_property_read(input, budget)
    }
}
/// Pure Producer-only registration. All actual inputs survive pairing refusal.
// Keep complete inputs inline: refusal must not require a new allocation.
#[allow(clippy::type_complexity)]
pub fn try_producer_registration(
    config: HttpProducerConfig,
    id: BindingId,
    generation: BindingGeneration,
    ordinal: u32,
) -> Result<
    ProducerBindingPreparation<HttpProducerConfig, HttpProducerExecutor>,
    BindingInputRejection<(HttpProducerConfig, HttpProducerExecutor, HttpProducerConfig)>,
> {
    ProducerBindingPreparation::try_new(
        config,
        producer_execution(config, id, generation, ordinal),
        config,
    )
}
use clinkz_wot_td::semantic::{
    SemanticName, SemanticNode, SemanticScope, SemanticValue, UriVariableScope,
};

impl HttpProducerConfig {
    /// Lowers completely covered resolved input. Refusal never opens a socket.
    /// Native preparation alone supplies no runtime authority.
    pub fn compile_property_read(
        &self,
        input: ResolvedProducerInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<HttpProducerProgram>> {
        if budget.consume(WorkClass::BindingPolls, 4096).is_err() {
            return Ok(None);
        }
        let document = input.document();
        let facts = document
            .form_facts(input.property(), input.form_index())
            .ok_or_else(producer_unsupported)?;
        if input.operation() != Operation::ReadProperty
            || facts.effective_operations != [Operation::ReadProperty]
            || facts.content_type != "application/json"
            || facts.subprotocol.is_some()
            || facts.content_coding.is_some()
            || facts.scopes.is_some()
            || facts.additional_responses
            || facts
                .response_content_type
                .is_some_and(|m| m != "application/json")
            || document.uri_variables(UriVariableScope::Thing).is_some()
            || document.uri_variables(UriVariableScope::Property).is_some()
            || document.property().observable
        {
            return Err(producer_unsupported());
        }
        let [name] = facts.effective_security else {
            return Err(producer_unsupported());
        };
        let security = document
            .security(name)
            .map_err(|_| producer_unsupported())?;
        if security.proxy.is_some()
            || security.semantics.children().any(|n| !class_assertion(n))
            || document
                .scope(SemanticScope::Thing)
                .ok_or_else(producer_unsupported)?
                .children()
                .any(|n| !class_assertion(n))
        {
            return Err(producer_unsupported());
        }
        // Shared DataSchema meaning is prepared once. HTTP supplies JSON
        // decoding; accepting extension-free schemas is not validation support.
        let response_schema =
            clinkz_wot_planning::type_schema::prepare_type_schema(document.property().schema)?;
        let form = document
            .scope(SemanticScope::Form {
                property: input.property(),
                index: input.form_index(),
            })
            .ok_or_else(producer_unsupported)?;
        let mut method = false;
        for node in form.children() {
            if node.name().is("http://www.w3.org/2011/http#methodName") {
                if method || node.value() != SemanticValue::Text("GET") {
                    return Err(producer_unsupported());
                }
                method = true;
            } else {
                return Err(producer_unsupported());
            }
        }
        if let Some(response) = document.scope(SemanticScope::Response {
            property: input.property(),
            index: input.form_index(),
        }) {
            let mut status = false;
            for node in response.children() {
                if status
                    || !node
                        .name()
                        .is("http://www.w3.org/2011/http#statusCodeValue")
                    || !matches!(node.value(), SemanticValue::Number(n) if n.as_u64() == Some(200))
                {
                    return Err(producer_unsupported());
                }
                status = true;
            }
        }
        let target = Target::parse(input.target()).map_err(|_| producer_unsupported())?;
        if target.address != self.address {
            return Err(producer_unsupported());
        }
        let authority_end = 7 + target.uri.authority().unwrap().as_str().len();
        let origin = target.uri.path_and_query().map_or("/", |p| p.as_str());
        if origin.len() > 64 {
            return Err(producer_unsupported());
        }
        let mut program = HttpProducerProgram {
            address: self.address,
            target: [0; 64],
            target_len: input.target().len() as u8,
            authority_end: authority_end as u8,
            origin: [0; 64],
            origin_len: origin.len() as u8,
            response_schema,
        };
        program.target[..input.target().len()].copy_from_slice(input.target().as_bytes());
        program.origin[..origin.len()].copy_from_slice(origin.as_bytes());
        Ok(Some(program))
    }
}
fn class_assertion(node: SemanticNode<'_>) -> bool {
    node.name() == SemanticName::Keyword("@type")
        && (matches!(node.value(), SemanticValue::Iri(_))
            || node.value() == SemanticValue::Array
                && node
                    .children()
                    .all(|n| matches!(n.value(), SemanticValue::Iri(_))))
}
