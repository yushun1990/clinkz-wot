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
    FormOwner, SemanticName, SemanticNode, SemanticScope, SemanticValue, UriVariableScope,
};

impl HttpProducerConfig {
    /// Lowers completely covered resolved input. Refusal never opens a socket.
    /// Native preparation alone supplies no runtime authority.
    pub fn compile_property_read(
        &self,
        input: ResolvedProducerInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<HttpProducerProgram>> {
        if self.secure
            || input.operation() != Operation::ReadProperty
            || input.document().property().observable
        {
            return Err(producer_unsupported());
        }
        self.compile_property(input, budget)
    }

    /// Compile one effective Read/Write operation. Each operation retains its
    /// own definition even when TD defaults select both on the same Form.
    pub(crate) fn compile_property(
        &self,
        input: ResolvedProducerInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<HttpProducerProgram>> {
        self.compile_mapping(
            clinkz_wot_planning::ResolvedBindingInput {
                operation: input.operation(),
                owner: FormOwner::Property(input.property()),
                property: input.property(),
                form_index: input.form_index(),
                target: input.target(),
                content_type: None,
                subprotocol: None,
                document: input.document(),
            },
            budget,
            None,
        )
    }

    pub(crate) fn compile_mapping(
        &self,
        input: clinkz_wot_planning::ResolvedBindingInput<'_>,
        budget: &mut WorkBudget,
        expected: Option<&HttpProducerProgram>,
    ) -> CoreResult<Option<HttpProducerProgram>> {
        if budget.consume(WorkClass::BindingPolls, 4096).is_err() {
            return Ok(None);
        }
        let document = input.document;
        let facts = document
            .form(input.owner, input.form_index)
            .ok_or_else(producer_unsupported)?;
        let collection = input.owner == FormOwner::Thing
            && matches!(
                input.operation,
                Operation::ReadAllProperties
                    | Operation::ReadMultipleProperties
                    | Operation::WriteAllProperties
                    | Operation::WriteMultipleProperties
            );
        if !(collection
            || matches!(
                (input.owner, input.operation),
                (
                    FormOwner::Property(_),
                    Operation::ReadProperty | Operation::WriteProperty
                )
            ))
            || !facts.effective_operations.contains(&input.operation)
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
        {
            return Err(producer_unsupported());
        }
        if !collection && {
            let flags = document.property().schema.facts();
            (input.operation == Operation::ReadProperty && flags.write_only)
                || (input.operation == Operation::WriteProperty && flags.read_only)
        } {
            return Err(producer_unsupported());
        }
        let security = clinkz_wot_planning::security::prepare_security(
            document,
            input.owner,
            input.form_index,
        )?;
        if (self.secure
            && (security != crate::security::BASIC
                || collection
                || !matches!(
                    input.operation,
                    Operation::ReadProperty | Operation::WriteProperty
                )))
            || (!self.secure && security != clinkz_wot_core::security::SecurityRequirement::NOSEC)
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
        let mut schema_work = 0;
        if collection {
            let writing = matches!(
                input.operation,
                Operation::WriteAllProperties | Operation::WriteMultipleProperties
            );
            for property in document
                .collection_properties()
                .ok_or_else(producer_unsupported)?
            {
                let facts = property.schema.facts();
                if if writing {
                    !facts.read_only
                } else {
                    !facts.write_only
                } {
                    schema_work +=
                        clinkz_wot_planning::type_schema::preparation_work(property.schema)?;
                }
            }
        } else {
            schema_work =
                clinkz_wot_planning::type_schema::preparation_work(document.property().schema)?;
        }
        if budget
            .consume(WorkClass::BindingPolls, schema_work)
            .is_err()
        {
            return Ok(None);
        }
        let property_map = if collection {
            Some(if let Some(original) = expected {
                let original = original
                    .property_map
                    .as_ref()
                    .ok_or_else(producer_unsupported)?;
                let writing = matches!(
                    input.operation,
                    Operation::WriteAllProperties | Operation::WriteMultipleProperties
                );
                if !clinkz_wot_planning::type_schema::matches_property_schema(
                    document, original, writing,
                )? {
                    return Err(producer_unsupported());
                }
                Arc::clone(original)
            } else {
                Arc::new(
                    if matches!(
                        input.operation,
                        Operation::WriteAllProperties | Operation::WriteMultipleProperties
                    ) {
                        clinkz_wot_planning::type_schema::prepare_writable_property_schema(
                            document,
                        )?
                    } else {
                        clinkz_wot_planning::type_schema::prepare_readable_property_schema(
                            document,
                        )?
                    },
                )
            })
        } else {
            None
        };
        let value_schema = if let Some(original) = expected {
            let agrees = if collection {
                original.value_schema.as_ref()
                    == &clinkz_wot_core::data_schema::PreparedSchema::Type(
                        clinkz_wot_core::data_schema::ValueType::Object,
                    )
            } else {
                clinkz_wot_planning::type_schema::matches_schema(
                    document.property().schema,
                    &original.value_schema,
                )?
            };
            if !agrees {
                return Err(producer_unsupported());
            }
            Arc::clone(&original.value_schema)
        } else {
            Arc::new(if collection {
                clinkz_wot_core::data_schema::PreparedSchema::Type(
                    clinkz_wot_core::data_schema::ValueType::Object,
                )
            } else {
                clinkz_wot_planning::type_schema::prepare_schema(document.property().schema)?
            })
        };
        let form = document
            .scope(if collection {
                SemanticScope::AffordanceForm {
                    owner: FormOwner::Thing,
                    index: input.form_index,
                }
            } else {
                SemanticScope::Form {
                    property: input.property,
                    index: input.form_index,
                }
            })
            .ok_or_else(producer_unsupported)?;
        let multiple = input.operation == Operation::ReadMultipleProperties;
        let mut method = false;
        for node in form.children() {
            if node.name().is("http://www.w3.org/2011/http#methodName") {
                let expected = if multiple {
                    "POST"
                } else if !matches!(
                    input.operation,
                    Operation::WriteProperty
                        | Operation::WriteAllProperties
                        | Operation::WriteMultipleProperties
                ) {
                    "GET"
                } else {
                    "PUT"
                };
                if method
                    || facts.declared_operations.is_some_and(|ops| ops.len() > 1)
                    || node.value() != SemanticValue::Text(expected)
                {
                    return Err(producer_unsupported());
                }
                method = true;
            } else {
                return Err(producer_unsupported());
            }
        }
        if multiple && !method {
            return Err(producer_unsupported());
        }
        if let Some(response) = document.scope(if collection {
            SemanticScope::AffordanceResponse {
                owner: FormOwner::Thing,
                index: input.form_index,
            }
        } else {
            SemanticScope::Response {
                property: input.property,
                index: input.form_index,
            }
        }) {
            let mut status = false;
            for node in response.children() {
                if status
                    || !node
                        .name()
                        .is("http://www.w3.org/2011/http#statusCodeValue")
                    || !matches!(node.value(), SemanticValue::Number(n) if n.as_u64() == Some(if !matches!(input.operation, Operation::WriteProperty | Operation::WriteAllProperties | Operation::WriteMultipleProperties) { 200 } else { 204 }))
                {
                    return Err(producer_unsupported());
                }
                status = true;
            }
        }
        let target =
            Target::parse_for(input.target, self.secure).map_err(|_| producer_unsupported())?;
        if target.address != self.address {
            return Err(producer_unsupported());
        }
        let authority_end =
            if self.secure { 8 } else { 7 } + target.uri.authority().unwrap().as_str().len();
        let origin = target.uri.path_and_query().map_or("/", |p| p.as_str());
        if origin.len() > 64 {
            return Err(producer_unsupported());
        }
        let mut program = HttpProducerProgram {
            operation: input.operation,
            basic: self.secure,
            security_registration: None,
            form: input.form_index,
            address: self.address,
            target: [0; 64],
            target_len: input.target.len() as u8,
            authority_end: authority_end as u8,
            origin: [0; 64],
            origin_len: origin.len() as u8,
            value_schema,
            property_map,
        };
        program.target[..input.target.len()].copy_from_slice(input.target.as_bytes());
        program.origin[..origin.len()].copy_from_slice(origin.as_bytes());
        Ok(Some(program))
    }
}
pub(crate) fn class_assertion(node: SemanticNode<'_>) -> bool {
    node.name() == SemanticName::Keyword("@type")
        && (matches!(node.value(), SemanticValue::Iri(_))
            || node.value() == SemanticValue::Array
                && node
                    .children()
                    .all(|n| matches!(n.value(), SemanticValue::Iri(_))))
}
