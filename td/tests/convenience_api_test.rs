use clinkz_wot_td::{
    affordance::{ActionAffordance, InteractionHelper, PropertyAffordance},
    data_schema::{ContextHelper as DataSchemaContextHelper, DataSchema},
    data_type::Operation,
    form::Form,
    security_scheme::{APIKeySecurityScheme, NoSecurityScheme, SecurityScheme},
    thing::Thing,
    thing_model::ThingModel,
    validate::Validate,
};

#[test]
fn security_scheme_convenience_constructors_create_valid_schemes() {
    let nosec = SecurityScheme::nosec();
    let basic = SecurityScheme::basic("Authorization");
    let apikey = SecurityScheme::apikey("api_key");
    let digest = SecurityScheme::digest("Authorization");
    let bearer = SecurityScheme::bearer("Authorization");
    let bearer_auth =
        SecurityScheme::bearer_authorization("Authorization", "https://auth.example/token")
            .unwrap();
    let psk = SecurityScheme::psk("device-1");
    let oauth2_code = SecurityScheme::oauth2_code(
        "https://auth.example/authorize",
        "https://auth.example/token",
    )
    .unwrap();
    let oauth2_client = SecurityScheme::oauth2_client();
    let oauth2_device = SecurityScheme::oauth2_device();

    assert_eq!(nosec.scheme(), "nosec");
    assert_eq!(basic.scheme(), "basic");
    assert_eq!(apikey.scheme(), "apikey");
    assert_eq!(digest.scheme(), "digest");
    assert_eq!(bearer.scheme(), "bearer");
    assert_eq!(bearer_auth.scheme(), "bearer");
    assert_eq!(psk.scheme(), "psk");
    assert_eq!(oauth2_code.scheme(), "oauth2");
    assert_eq!(oauth2_client.scheme(), "oauth2");
    assert_eq!(oauth2_device.scheme(), "oauth2");
    nosec.validate().unwrap();
    basic.validate().unwrap();
    apikey.validate().unwrap();
    digest.validate().unwrap();
    bearer.validate().unwrap();
    bearer_auth.validate().unwrap();
    psk.validate().unwrap();
    oauth2_code.validate().unwrap();
    oauth2_client.validate().unwrap();
    oauth2_device.validate().unwrap();
}

#[test]
fn combo_security_convenience_constructors_create_valid_schemes() {
    let thing = Thing::builder("Lamp")
        .basic_security("basic", "Authorization")
        .apikey_security("api", "api_key")
        .combo_one_of_security("combo", ["basic", "api"])
        .build()
        .unwrap();

    thing.validate().unwrap();
    assert_eq!(thing.security, ["basic", "api", "combo"]);
    assert_eq!(
        thing.security_definitions.get("combo").unwrap().scheme(),
        "combo"
    );
}

#[test]
fn concrete_security_schemes_convert_into_security_scheme() {
    let nosec: SecurityScheme = NoSecurityScheme::builder().build().unwrap().into();
    let apikey: SecurityScheme = APIKeySecurityScheme::builder()
        .name("api_key")
        .build()
        .unwrap()
        .into();

    assert_eq!(nosec.scheme(), "nosec");
    assert_eq!(apikey.scheme(), "apikey");
}

#[test]
fn schema_builders_convert_into_data_schema() {
    let property = PropertyAffordance::builder(DataSchema::string().min_length(1))
        .uri_variable("locale", DataSchema::string())
        .form(Form::read_property("/status").build().unwrap())
        .build()
        .unwrap();
    let action = ActionAffordance::builder()
        .input(DataSchema::object().property("value", DataSchema::integer()))
        .output(DataSchema::boolean())
        .form(Form::invoke_action("/toggle").build().unwrap())
        .build()
        .unwrap();

    let thing = Thing::builder("Lamp")
        .nosec()
        .property("status", property)
        .action("toggle", action)
        .build()
        .unwrap();

    thing.validate().unwrap();
}

#[test]
fn data_schema_nested_convenience_accepts_concrete_builders() {
    let schema = DataSchema::object()
        .properties([
            ("enabled", DataSchema::from(DataSchema::boolean())),
            ("count", DataSchema::from(DataSchema::integer().minimum(0))),
        ])
        .one_of([
            DataSchema::from(DataSchema::null()),
            DataSchema::from(DataSchema::string().min_length(1)),
        ])
        .build();
    let array = DataSchema::array().items([DataSchema::string().min_length(1)]);

    DataSchema::from(schema).validate().unwrap();
    DataSchema::from(array).validate().unwrap();
}

#[test]
fn form_builder_operation_shortcuts_set_expected_operations() {
    let read = Form::read_property("/status").build().unwrap();
    let invoke = Form::builder("/toggle").invoke_action().build().unwrap();
    let subscribe = Form::subscribe_event("/changed").build().unwrap();
    let meta = Form::builder("/properties")
        .read_all_properties()
        .write_multiple_properties()
        .build()
        .unwrap();

    assert_eq!(read.op.unwrap(), [Operation::ReadProperty]);
    assert_eq!(invoke.op.unwrap(), [Operation::InvokeAction]);
    assert_eq!(subscribe.op.unwrap(), [Operation::SubscribeEvent]);
    assert_eq!(
        meta.op.unwrap(),
        [
            Operation::ReadAllProperties,
            Operation::WriteMultipleProperties
        ]
    );
}

#[test]
fn thing_builder_supports_named_security_definitions() {
    let thing = Thing::builder("Lamp")
        .basic_security("auth", "Authorization")
        .build()
        .unwrap();

    thing.validate().unwrap();
    assert_eq!(thing.security, ["auth"]);
    assert_eq!(
        thing.security_definitions.get("auth").unwrap().scheme(),
        "basic"
    );
}

#[test]
fn thing_builder_supports_extended_security_shortcuts() {
    let thing = Thing::builder("Lamp")
        .bearer_authorization_security("bearer", "Authorization", "https://auth.example/token")
        .psk_security("psk", "device-1")
        .oauth2_code_security(
            "oauth",
            "https://auth.example/authorize",
            "https://auth.example/token",
        )
        .build()
        .unwrap();

    thing.validate().unwrap();
    assert_eq!(thing.security, ["bearer", "psk", "oauth"]);
}

#[test]
fn thing_builder_security_shortcuts_keep_uri_errors() {
    let error = Thing::builder("Lamp")
        .oauth2_code_security("oauth", "not a uri", "https://auth.example/token")
        .build()
        .unwrap_err();

    assert!(error.to_string().contains("authorization"));
}

#[test]
fn thing_model_builder_adds_nosec_security() {
    let model = ThingModel::builder("Lamp model").nosec().build().unwrap();

    model.validate().unwrap();
    assert_eq!(model.security.unwrap(), ["nosec"]);
    assert!(model.security_definitions.unwrap().contains_key("nosec"));
}

#[test]
fn thing_model_builder_supports_extended_security_shortcuts() {
    let model = ThingModel::builder("Lamp model")
        .basic_security("basic", "Authorization")
        .apikey_security("api", "api_key")
        .combo_all_of_security("combo", ["basic", "api"])
        .oauth2_client_security("oauth")
        .build()
        .unwrap();

    model.validate().unwrap();
    assert_eq!(model.security.unwrap(), ["basic", "api", "combo", "oauth"]);
}
