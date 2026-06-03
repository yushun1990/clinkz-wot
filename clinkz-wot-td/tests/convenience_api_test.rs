use clinkz_wot_td::{
    affordance::{ActionAffordance, InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
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

    assert_eq!(nosec.scheme(), "nosec");
    assert_eq!(basic.scheme(), "basic");
    assert_eq!(apikey.scheme(), "apikey");
    nosec.validate().unwrap();
    basic.validate().unwrap();
    apikey.validate().unwrap();
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
        .form(Form::builder("/status").build().unwrap())
        .build()
        .unwrap();
    let action = ActionAffordance::builder()
        .input(DataSchema::object().property("value", DataSchema::integer()))
        .output(DataSchema::boolean())
        .form(
            Form::builder("/toggle")
                .op([Operation::InvokeAction])
                .build()
                .unwrap(),
        )
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
fn thing_model_builder_adds_nosec_security() {
    let model = ThingModel::builder("Lamp model").nosec().build().unwrap();

    model.validate().unwrap();
    assert_eq!(model.security.unwrap(), ["nosec"]);
    assert!(model.security_definitions.unwrap().contains_key("nosec"));
}
