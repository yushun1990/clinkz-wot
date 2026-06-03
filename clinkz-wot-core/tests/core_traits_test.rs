use std::borrow::Cow;

use clinkz_wot_core::{
    AffordanceTarget, BindingRequest, CodecInput, CoreError, CoreResult, InteractionInput,
    InteractionOutput, Payload, PayloadCodec, ProtocolBinding, TransportAdapter, TransportRequest,
    TransportResponse,
};
use clinkz_wot_td::{
    affordance::InteractionHelper,
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    security_scheme::{NoSecurityScheme, SecurityScheme},
    thing::Thing,
    validate::Validate,
};

struct EchoCodec;

impl PayloadCodec for EchoCodec {
    fn content_type(&self) -> Cow<'_, str> {
        "application/octet-stream".into()
    }

    fn encode(&self, input: CodecInput<'_>) -> CoreResult<Payload> {
        Ok(Payload::new(
            input.body.to_vec(),
            self.content_type().into_owned(),
        ))
    }

    fn decode(&self, payload: &Payload) -> CoreResult<Vec<u8>> {
        Ok(payload.body.clone())
    }
}

struct EchoTransport;

impl TransportAdapter for EchoTransport {
    fn exchange(&mut self, request: TransportRequest) -> CoreResult<TransportResponse> {
        Ok(TransportResponse {
            metadata: request.metadata,
            payload: request.payload,
        })
    }
}

struct EchoBinding {
    transport: EchoTransport,
}

impl ProtocolBinding for EchoBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        form.content_type == "application/octet-stream" && operation == Operation::InvokeAction
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        let payload = request.input.payload;
        let response = self.transport.exchange(
            TransportRequest::new(request.form.href.as_str(), "invoke").with_payload(payload),
        )?;
        Ok(InteractionOutput {
            payload: response.payload,
        })
    }
}

trait RequestPayloadExt {
    fn with_payload(self, payload: Option<Payload>) -> Self;
}

impl RequestPayloadExt for TransportRequest {
    fn with_payload(mut self, payload: Option<Payload>) -> Self {
        self.payload = payload;
        self
    }
}

#[test]
fn codec_round_trips_payload_bytes() {
    let codec = EchoCodec;

    let payload = codec
        .encode(CodecInput {
            body: b"hello",
            data_type: None,
        })
        .unwrap();

    assert_eq!(payload.content_type, "application/octet-stream");
    assert_eq!(codec.decode(&payload).unwrap(), b"hello");
}

#[test]
fn binding_invokes_selected_form_without_protocol_assumptions() {
    let form = Form::builder("wot://thing/actions/ping")
        .content_type("application/octet-stream")
        .op([Operation::InvokeAction])
        .build()
        .unwrap();
    let action = clinkz_wot_td::affordance::ActionAffordance::builder()
        .form(form.clone())
        .input(DataSchema::String(DataSchema::string().build()))
        .build()
        .unwrap();
    let thing = Thing::builder("Lamp")
        .security(SecurityScheme::NoSec(
            NoSecurityScheme::builder().build().unwrap(),
        ))
        .action("ping", action)
        .build()
        .unwrap();
    thing.validate().unwrap();

    let mut binding = EchoBinding {
        transport: EchoTransport,
    };
    assert!(binding.supports(&form, Operation::InvokeAction));

    let output = binding
        .invoke(BindingRequest {
            thing: &thing,
            target: AffordanceTarget::Action("ping"),
            operation: Operation::InvokeAction,
            form: &form,
            input: InteractionInput::with_payload(Payload::new(
                b"payload".to_vec(),
                "application/octet-stream",
            )),
        })
        .unwrap();

    assert_eq!(output.payload.unwrap().body, b"payload");
}

#[test]
fn core_error_display_is_english() {
    let err = CoreError::UnsupportedBinding("no matching form".into());

    assert_eq!(err.to_string(), "Unsupported binding: no matching form");
}
