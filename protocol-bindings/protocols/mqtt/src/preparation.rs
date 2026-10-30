//! Interpret MQTT vocabulary only here, over common TD-resolved semantics.
use super::*;
use clinkz_wot_core::CoreResult;
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_planning::{
    producer_composition::{FormSite, ProducerCapability, ProducerOperationInput},
    type_schema::prepare_type_schema,
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
        let Some((topic, schema)) =
            lower_notification(self.registration.config.address, input, budget)?
        else {
            return Ok(None);
        };
        let target = match input.site {
            FormSite::Property(n) => AffordanceTarget::Property(Arc::from(n.as_str())),
            FormSite::Event(n) => AffordanceTarget::Event(Arc::from(n.as_str())),
            _ => return Ok(None),
        };
        Ok(Some(MqttNotificationDefinition {
            registration: Rc::clone(&self.registration),
            target,
            topic,
            form: input.form,
            operation: input.operation,
            schema,
        }))
    }
}
/// Common bounded MQTT vocabulary interpretation, with no physical owners.
pub(crate) fn lower_notification(
    configured_address: SocketAddr,
    input: ProducerOperationInput<'_>,
    budget: &mut WorkBudget,
) -> CoreResult<Option<(Arc<str>, TypeSchema)>> {
    let (name, owner, kind, variables) = match input.site {
        FormSite::Property(n) => (
            n,
            FormOwner::Property(n),
            NotificationKind::PropertyObservation,
            UriVariableScope::Property,
        ),
        FormSite::Event(n) => (
            n,
            FormOwner::Event(n),
            NotificationKind::Event,
            UriVariableScope::Event,
        ),
        _ => return Ok(None),
    };
    if !input.target.starts_with("mqtt:") {
        return Ok(None);
    }
    // Selected MQTT declarations must be completely understood. A native
    // address outside this registration remains eligible for another one.
    let address: SocketAddr = input
        .target
        .strip_prefix("mqtt://")
        .ok_or_else(unsupported)?
        .trim_end_matches('/')
        .parse()
        .map_err(|_| unsupported())?;
    if address != configured_address {
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
    let schema = match kind {
        NotificationKind::PropertyObservation => {
            let property = doc.property();
            if !property.observable {
                return Err(unsupported());
            }
            // The Property DataSchema includes extension assertions. Unknown
            // semantics refuse in the shared type-only validator.
            prepare_type_schema(property.schema)?
        }
        NotificationKind::Event => {
            let event_scope = doc
                .scope(SemanticScope::Event(name))
                .ok_or_else(unsupported)?;
            // MQTT control ACKs are never Event subscription/cancel/dataResponse.
            if event_scope
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
            prepare_type_schema(
                doc.affordance_schema(owner, "data")
                    .ok_or_else(unsupported)?,
            )?
        }
    };
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
    if topic.is_empty()
        || topic.len() > MAX_TOPIC_BYTES
        || topic.starts_with('$')
        || topic
            .bytes()
            .any(|b| !b.is_ascii_graphic() || matches!(b, b'+' | b'#'))
        || explicit_topic.is_some_and(|t| t != topic)
    {
        return Err(unsupported());
    }
    Ok(Some((Arc::from(topic), schema)))
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
                + target_name(&d.target).len()
                + d.topic.len()
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
        Some(8192)
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
        Ok(self.lower(input, b)?.as_ref() == Some(expected))
    }
}
