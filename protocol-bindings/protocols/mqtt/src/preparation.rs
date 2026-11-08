//! Interpret MQTT vocabulary only here, over common TD-resolved semantics.
use super::*;
use clinkz_wot_core::{CoreResult, notification::NotificationSchemas};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_planning::{
    producer_composition::{FormSite, ProducerCapability, ProducerOperationInput},
    type_schema::{
        matches_notification_schemas, notification_preparation_work, prepare_notification_schemas,
    },
};
use clinkz_wot_td::semantic::{
    FormOwner, SemanticName, SemanticNode, SemanticScope, SemanticValue, UriVariableScope,
};
use std::any::Any;

const TD_DATA: &str = "https://www.w3.org/2019/wot/td#data";
impl MqttNotificationProducer {
    fn lower(
        &self,
        input: ProducerOperationInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<MqttNotificationDefinition>> {
        let Some(kind) = NotificationKind::for_operation(input.operation) else {
            return Ok(None);
        };
        if !matches_address(self.registration.config.address, input.target)? {
            return Ok(None);
        }
        let target = match input.site {
            FormSite::Thing => AffordanceTarget::Thing,
            FormSite::Property(n) => AffordanceTarget::Property(Arc::from(n.as_str())),
            FormSite::Event(n) => AffordanceTarget::Event(Arc::from(n.as_str())),
            _ => return Ok(None),
        };
        budget
            .consume(
                WorkClass::PlanningItems,
                notification_preparation_work(input.document, &target, kind)?,
            )
            .map_err(|_| busy())?;
        let schemas = Arc::new(prepare_notification_schemas(input.document, &target, kind)?);
        let Some(topic) =
            lower_notification(self.registration.config.address, input, &schemas, budget)?
        else {
            return Ok(None);
        };
        Ok(Some(MqttNotificationDefinition {
            registration: Rc::clone(&self.registration),
            target,
            topic,
            form: input.form,
            operation: input.operation,
            schemas,
        }))
    }
}
/// Common bounded MQTT vocabulary interpretation, with no physical owners.
pub(crate) fn lower_notification(
    configured_address: SocketAddr,
    input: ProducerOperationInput<'_>,
    schemas: &NotificationSchemas,
    budget: &mut WorkBudget,
) -> CoreResult<Option<Arc<str>>> {
    let (name, owner, kind, variables) = match input.site {
        FormSite::Thing => (
            "",
            FormOwner::Thing,
            match NotificationKind::for_operation(input.operation) {
                Some(
                    k @ (NotificationKind::ObserveAllProperties
                    | NotificationKind::SubscribeAllEvents),
                ) => k,
                _ => return Ok(None),
            },
            UriVariableScope::Thing,
        ),
        FormSite::Property(n) => (
            n.as_str(),
            FormOwner::Property(n),
            NotificationKind::PropertyObservation,
            UriVariableScope::Property,
        ),
        FormSite::Event(n) => (
            n.as_str(),
            FormOwner::Event(n),
            NotificationKind::Event,
            UriVariableScope::Event,
        ),
        _ => return Ok(None),
    };
    if !matches_address(configured_address, input.target)? {
        return Ok(None);
    }
    budget
        .consume(WorkClass::BindingPolls, 4096)
        .map_err(|_| unsupported())?;
    if (input.operation != kind.start_operation() && input.operation != kind.stop_operation())
        || name.len() > 64
    {
        return Err(unsupported());
    }
    let doc = input.document;
    let form = doc.form(owner, input.form).ok_or_else(unsupported)?;
    if form.content_type != "application/json"
        || form.content_coding.is_some()
        || form.subprotocol.is_some()
        || form.scopes.is_some()
        || form.additional_responses
        || form.response_content_type.is_some()
        || doc.uri_variables(UriVariableScope::Thing).is_some()
        || doc.uri_variables(variables).is_some()
        || doc
            .scope(SemanticScope::Thing)
            .is_none_or(|s| s.children().any(|n| !annotation(n)))
    {
        return Err(unsupported());
    }
    if schemas.kind() != kind || !schemas.supports(&MqttJsonData) {
        return Err(unsupported());
    }
    match schemas {
        NotificationSchemas::SubscribeAllEvents(schema) => {
            if form.effective_operations.len() != 2
                || !form.effective_operations.contains(&kind.start_operation())
                || !form.effective_operations.contains(&kind.stop_operation())
                || schema.members().is_empty()
            {
                return Err(unsupported());
            }
            for m in schema.members() {
                if m.name.is_empty()
                    || m.name.len() > 64
                    || m.name
                        .bytes()
                        .any(|b| !b.is_ascii_graphic() || matches!(b, b'/' | b'+' | b'#' | b'$'))
                    || m.subscription.is_some()
                    || m.cancellation.is_some()
                    || m.data_response.is_some()
                    || m.data.is_none()
                    || m.schemas().any(|s| !s.is_type_only())
                    || doc.scope(SemanticScope::Event(&m.name)).is_none_or(|s| {
                        s.children()
                            .any(|n| !annotation(n) && !n.name().is(TD_DATA))
                            || s.children().filter(|n| n.name().is(TD_DATA)).count() != 1
                    })
                {
                    return Err(unsupported());
                }
            }
        }
        NotificationSchemas::ObserveAllProperties(schema) => {
            if form.effective_operations.len() != 2
                || !form.effective_operations.contains(&kind.start_operation())
                || !form.effective_operations.contains(&kind.stop_operation())
                || schema.members().is_empty()
            {
                return Err(unsupported());
            }
        }
        NotificationSchemas::PropertyValue(_) => {
            if !doc.property().observable || doc.property().schema.facts().write_only {
                return Err(unsupported());
            }
        }
        NotificationSchemas::Event {
            subscription,
            data,
            data_response,
            cancellation,
        } => {
            let event_scope = doc
                .scope(SemanticScope::Event(name))
                .ok_or_else(unsupported)?;
            // MQTT ACKs certify no application subscription/cancel/response role.
            if subscription.is_some()
                || cancellation.is_some()
                || data_response.is_some()
                || data.is_none()
                || event_scope
                    .children()
                    .any(|n| !annotation(n) && !n.name().is(TD_DATA))
                || event_scope
                    .children()
                    .filter(|n| n.name().is(TD_DATA))
                    .count()
                    != 1
            {
                return Err(unsupported());
            }
        }
    }
    let [security_name] = form.effective_security else {
        return Err(unsupported());
    };
    let security = doc.security(security_name).map_err(|_| unsupported())?;
    if security.scheme != "nosec"
        || security.proxy.is_some()
        || security.semantics.children().any(|n| !annotation(n))
    {
        return Err(unsupported());
    }
    let mut topic = None;
    let mut explicit_topic = None;
    let mut seen = 0u8;
    let form_scope = doc
        .scope(match owner {
            FormOwner::Property(property) => SemanticScope::Form {
                property,
                index: input.form,
            },
            _ => SemanticScope::AffordanceForm {
                owner,
                index: input.form,
            },
        })
        .ok_or_else(unsupported)?;
    for node in form_scope.children() {
        if annotation(node) {
            continue;
        }
        let (index, term) = [
            ("filter", "https://www.w3.org/2019/wot/mqtt#filter"),
            ("topic", "https://www.w3.org/2019/wot/mqtt#topic"),
            ("qos", "https://www.w3.org/2019/wot/mqtt#qos"),
            ("retain", "https://www.w3.org/2019/wot/mqtt#retain"),
            (
                "controlPacket",
                "https://www.w3.org/2019/wot/mqtt#controlPacket",
            ),
        ]
        .iter()
        .enumerate()
        .find(|(_, (_, iri))| node.name().is(iri))
        .map(|(i, (term, _))| (i, *term))
        .ok_or_else(unsupported)?;
        if seen & (1 << index) != 0 {
            return Err(unsupported());
        }
        seen |= 1 << index;
        match (term, node.value()) {
            ("filter", SemanticValue::Text(s)) => topic = Some(s),
            ("topic", SemanticValue::Text(s)) => explicit_topic = Some(s),
            ("qos", SemanticValue::Text("0")) => {}
            ("qos", SemanticValue::Number(n)) if n.as_u64() == Some(0) => {}
            ("retain", SemanticValue::Bool(false)) => {}
            ("controlPacket", SemanticValue::Text(s))
                if form.effective_operations.len() == 1
                    && s == if input.operation == kind.start_operation() {
                        "subscribe"
                    } else {
                        "unsubscribe"
                    } => {}
            _ => return Err(unsupported()),
        }
    }
    let topic = topic.ok_or_else(unsupported)?;
    let filter_body = if kind == NotificationKind::SubscribeAllEvents {
        let prefix = topic.strip_suffix("/+").ok_or_else(unsupported)?;
        let NotificationSchemas::SubscribeAllEvents(s) = schemas else {
            return Err(unsupported());
        };
        if prefix.is_empty()
            || explicit_topic.is_some()
            || s.members()
                .iter()
                .any(|m| prefix.len() + 1 + m.name.len() > MAX_TOPIC_BYTES)
        {
            return Err(unsupported());
        }
        prefix
    } else {
        topic
    };
    if topic.is_empty()
        || topic.len() > MAX_TOPIC_BYTES
        || topic.starts_with('$')
        || filter_body
            .bytes()
            .any(|b| !b.is_ascii_graphic() || matches!(b, b'+' | b'#'))
        || explicit_topic.is_some_and(|t| t != topic)
    {
        return Err(unsupported());
    }
    Ok(Some(Arc::from(topic)))
}
fn matches_address(configured: SocketAddr, target: &str) -> CoreResult<bool> {
    if !target.starts_with("mqtt:") {
        return Ok(false);
    }
    let address: SocketAddr = target
        .strip_prefix("mqtt://")
        .ok_or_else(unsupported)?
        .trim_end_matches('/')
        .parse()
        .map_err(|_| unsupported())?;
    Ok(address == configured)
}
fn busy() -> CoreError {
    CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
fn annotation(node: SemanticNode<'_>) -> bool {
    node.name() == SemanticName::Keyword("@type")
        && (matches!(node.value(), SemanticValue::Iri(_))
            || node.value() == SemanticValue::Array
                && node
                    .children()
                    .all(|n| matches!(n.value(), SemanticValue::Iri(_))))
}
impl ProducerCapability for MqttNotificationProducer {
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.registration.binding
    }
    fn definition_bytes(&self, d: &dyn Any) -> Option<usize> {
        let d = d.downcast_ref::<MqttNotificationDefinition>()?;
        Some(
            std::mem::size_of::<MqttNotificationDefinition>()
                + target_name_bytes(&d.target)
                + d.topic.len()
                + schema_backing(&d.schemas)
                + 4 * std::mem::size_of::<usize>(),
        )
    }
    fn prepare(
        &self,
        input: ProducerOperationInput<'_>,
        b: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>> {
        Ok(self.lower(input, b)?.map(|d| Box::new(d) as Box<dyn Any>))
    }
    fn supports(&self, input: ProducerOperationInput<'_>, b: &mut WorkBudget) -> CoreResult<bool> {
        Ok(self.lower(input, b)?.is_some())
    }
    fn final_check_temporary_bytes(&self) -> Option<usize> {
        Some(
            8192 + (4 * PreparedSchema::BACKING_BOUND)
                .max(clinkz_wot_core::collection::EventCollectionSchema::backing_bound())
                .max(clinkz_wot_core::collection::PropertyCollectionSchema::backing_bound())
                + std::mem::size_of::<NotificationSchemas>()
                + 2 * std::mem::size_of::<usize>(),
        )
    }
    fn check_final(
        &self,
        input: ProducerOperationInput<'_>,
        d: &dyn Any,
        b: &mut WorkBudget,
    ) -> CoreResult<bool> {
        let Some(expected) = d.downcast_ref::<MqttNotificationDefinition>() else {
            return Ok(false);
        };
        if !Rc::ptr_eq(&expected.registration, &self.registration)
            || input.site.site() != target_site(&expected.target)
            || input.form != expected.form
            || input.operation != expected.operation
        {
            return Ok(false);
        }
        b.consume(
            WorkClass::PlanningItems,
            notification_preparation_work(
                input.document,
                &expected.target,
                expected.schemas.kind(),
            )?,
        )
        .map_err(|_| busy())?;
        if !matches_notification_schemas(input.document, &expected.target, &expected.schemas)? {
            return Ok(false);
        }
        Ok(lower_notification(
            self.registration.config.address,
            input,
            &expected.schemas,
            b,
        )?
        .as_ref()
            == Some(&expected.topic))
    }
}
