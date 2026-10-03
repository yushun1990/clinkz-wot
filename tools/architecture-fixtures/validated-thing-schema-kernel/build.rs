// Compile current TD source with shared private field and Basic seams.
// No generated source is committed or used by the production TD crate.
use std::{env, fs, path::Path};

fn copy_source(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    println!("cargo:rerun-if-changed={}", source.display());
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_source(&entry.path(), &destination);
        } else {
            println!("cargo:rerun-if-changed={}", entry.path().display());
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn main() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fixture.join("../../../td/src").canonicalize().unwrap();
    let output = env::var_os("OUT_DIR").unwrap();
    let candidate = Path::new(&output).join("candidate");
    copy_source(&source, &candidate);

    let lib = fs::read_to_string(candidate.join("lib.rs")).unwrap();
    let mut lib = lib.replace("//!", "//").replace("#![no_std]", "");
    for (module, file) in [
        ("schema_kernel", "kernel.rs"),
        ("schema_access", "typed_access.rs"),
        ("schema_diagnostics", "schema_diagnostics.rs"),
        ("basic_kernel", "basic_kernel.rs"),
        ("basic_typed", "basic_typed.rs"),
        ("basic_diagnostics", "basic_diagnostics.rs"),
        ("schema_fields", "schema_fields.rs"),
        ("schema_serde", "schema_serde.rs"),
    ] {
        let path = fixture.join("src").join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        lib.push_str(&format!(
            "\n#[path = {path:?}]\n#[allow(dead_code)]\nmod {module};\n"
        ));
    }
    let path = fixture.join("src/schema_arena.rs");
    println!("cargo:rerun-if-changed={}", path.display());
    lib.push_str(&format!(
        "\n#[cfg(feature = \"validated-thing\")]\n#[path = {path:?}]\npub mod schema_arena;\n"
    ));
    lib = lib.replace("mod schema_fields;", "pub mod schema_fields;");
    lib = lib.replace("mod schema_kernel;", "pub mod schema_kernel;");
    fs::write(candidate.join("lib.rs"), lib).unwrap();

    let path = candidate.join("components/data_schema.rs");
    let schema = fs::read_to_string(&path).unwrap();
    let seam = "impl Validate for DataSchema {";
    assert_eq!(schema.matches(seam).count(), 1, "schema seam changed");
    let (types_and_builders, rules) = schema.split_once(seam).unwrap();
    assert!(rules.ends_with("}\n"), "schema rule boundary changed");
    // Fail visibly if builders begin using a helper currently owned by this
    // final validation region, rather than silently replacing more source.
    for helper in [
        "validate_schema_context(",
        "validate_numeric_bounds(",
        "validate_ordered(",
        "format_schema_path(",
    ] {
        assert!(
            !types_and_builders.contains(helper),
            "new caller outside schema seam"
        );
    }
    let mut types_and_builders = types_and_builders.to_string();
    replace_region(
        &mut types_and_builders,
        "    fn expected_data_type",
        "\n}\n\n",
        "",
    );
    types_and_builders = types_and_builders.replace(
        "BTreeMap, format, string::String",
        "BTreeMap, string::String",
    );
    project_schema_fields(&mut types_and_builders);
    fs::write(
        &path,
        format!(
            "{types_and_builders}\n{}",
            include_str!("src/public_adapter.rs")
        ),
    )
    .unwrap();
    println!("cargo:rerun-if-changed=src/public_adapter.rs");

    // The same declarative metadata rows produce the unchanged owning serde
    // declaration and the literal arena reader. No second field table.
    let path = candidate.join("core/data_type/metadata.rs");
    let mut metadata = fs::read_to_string(&path).unwrap();
    replace_region(
        &mut metadata,
        "#[serde_as]\n#[skip_serializing_none]",
        "impl Metadata {",
        r#"
macro_rules! define_metadata {
    ($( $name:ident, $field:ident, $wire:literal, $ty:ty, $assoc:ident, $read:ident,
        [$($attribute:tt)*]; )*) => {
        #[serde_as]
        #[skip_serializing_none]
        #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Metadata {
            $(#[serde(rename = $wire)] $($attribute)* pub $name: Option<$ty>,)*
        }
        pub(crate) const METADATA_KEYS: &[&str] = &[$($wire,)*];
    };
}
crate::schema_fields::metadata_fields!(define_metadata);

"#,
    );
    fs::write(path, metadata).unwrap();

    let path = candidate.join("flat.rs");
    let mut flat = fs::read_to_string(&path).unwrap();
    replace_region(
        &mut flat,
        "/// Removes `key` from `map` and deserializes its value into `Option<Vec<T>>`",
        "/// Removes a required `key`",
        "",
    );
    flat = flat.replace("use alloc::vec::Vec;\n", "");
    fs::write(path, flat).unwrap();

    // The candidate's actual public Basic entry delegates to the same body as
    // the Snapshot. Profile/Full keep their original additional checks.
    let path = candidate.join("thing.rs");
    let thing = fs::read_to_string(&path).unwrap();
    let seam = "impl Validate for Thing {";
    assert_eq!(thing.matches(seam).count(), 1, "Thing seam changed");
    let (before, after) = thing.split_once(seam).unwrap();
    let point = "        // Profile/Full: @context must contain a standard WoT context URI.";
    assert_eq!(
        after.matches(point).count(),
        1,
        "Thing Basic boundary changed"
    );
    let after = after.replacen(point, &format!(
        "        if matches!(level, ValidationLevel::Basic) {{\n            return crate::basic_kernel::validate(&crate::basic_typed::TypedBasicAccess(Some(self)), &crate::basic_diagnostics::PublicSink {{ document: true }});\n        }}\n\n{point}"
    ), 1);
    fs::write(path, format!("{before}{seam}{after}")).unwrap();

    // Standalone affordance/security APIs share the same component rules too.
    // These components have no additional Profile/Full predicates.
    let path = candidate.join("components/affordance.rs");
    let mut affordances = fs::read_to_string(&path).unwrap();
    for (component, variant, kind) in [
        ("PropertyAffordance", "Property", "Property"),
        ("ActionAffordance", "Action", "Action"),
        ("EventAffordance", "Event", "Event"),
    ] {
        let start = format!("impl Validate for {component} {{");
        let end = format!("impl {component} {{");
        let replacement = format!(
            "impl Validate for {component} {{\n    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {{\n        if matches!(level, ValidationLevel::Minimal) {{ return Ok(()); }}\n        crate::basic_kernel::validate_affordance(\n            &crate::basic_typed::TypedBasicAccess(None),\n            crate::basic_typed::Affordance::{variant}(self),\n            crate::basic_kernel::Owner {{ kind: crate::basic_kernel::OwnerKind::{kind}, ordinal: 0 }},\n            &crate::basic_diagnostics::PublicSink {{ document: false }},\n        )\n    }}\n}}\n\n"
        );
        replace_region(&mut affordances, &start, &end, &replacement);
    }
    // The now-orphaned final helper contains only the former component rules.
    let helper = "fn validate_interaction_schemas(";
    assert_eq!(
        affordances.matches(helper).count(),
        1,
        "interaction helper acquired another caller"
    );
    affordances.truncate(affordances.find(helper).unwrap());
    affordances = affordances.replace("ValidationLevel, schema_error_message", "ValidationLevel");
    fs::write(path, affordances).unwrap();

    let path = candidate.join("components/security_scheme.rs");
    let mut security = fs::read_to_string(&path).unwrap();
    replace_region(
        &mut security,
        "impl Validate for SecurityScheme {",
        "fn validate_combo_members(",
        "impl Validate for SecurityScheme {\n    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {\n        if matches!(level, ValidationLevel::Minimal) { return Ok(()); }\n        crate::basic_kernel::validate_security_scheme(\n            &crate::basic_typed::TypedBasicAccess(None), self,\n            crate::basic_kernel::Owner { kind: crate::basic_kernel::OwnerKind::SecurityDefinition, ordinal: 0 },\n            &crate::basic_diagnostics::PublicSink { document: false },\n        )\n    }\n}\n\n",
    );
    // Keep the original named-reference helper for Profile/Full callers, but
    // remove unreachable copies of the extracted local security predicates.
    replace_region(
        &mut security,
        "fn validate_combo_members(",
        "fn validate_combo_references(",
        "",
    );
    replace_region(
        &mut security,
        "fn validate_oauth2_scheme(",
        "fn string_array_field(",
        "",
    );
    replace_region(
        &mut security,
        "    fn string_field(",
        "    fn one_of_references(",
        "",
    );
    replace_region(
        &mut security,
        "    fn apikey_name(",
        "\n}\n\nimpl Validate",
        "",
    );
    fs::write(path, security).unwrap();

    // Reuse the existing query witness's helpers in the public-source candidate
    // too. This is a seam extraction, without validating inferred operations.
    let path = candidate.join("td_defaults.rs");
    let mut defaults = fs::read_to_string(&path).unwrap();
    replace_region(
        &mut defaults,
        "const PROPERTY_READ_WRITE_OPERATIONS",
        "const ACTION_OPERATIONS",
        "",
    );
    replace_region(
        &mut defaults,
        "pub fn effective_form_security",
        "/// Returns the content type that applies",
        "pub fn effective_form_security<'a>(thing: &'a Thing, form: &'a Form) -> &'a [String] {\n    crate::basic_kernel::inherited_security(thing.security.as_slice(), form.security.as_deref()).0\n}\n\n",
    );
    replace_region(
        &mut defaults,
        "fn default_property_operations",
        "fn schema_context",
        "fn default_property_operations(property: &PropertyAffordance) -> &'static [Operation] {\n    let schema = schema_context(&property._schema);\n    crate::basic_kernel::default_property_operations((schema.read_only, schema.write_only))\n}\n\n",
    );
    fs::write(path, defaults).unwrap();
}

fn project_schema_fields(source: &mut String) {
    replace_region(
        source,
        "impl<'de> Deserialize<'de> for DataSchemaContext {",
        "impl Serialize for DataSchemaContext {",
        r#"impl<'de> Deserialize<'de> for DataSchemaContext {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = crate::flat::deserialize_map(deserializer)?;
        crate::schema_fields::context(crate::schema_serde::Source::new(map))
            .map(crate::schema_serde::context_into_td).map_err(serde::de::Error::custom)
    }
}

"#,
    );
    for (name, variant) in [
        ("ArraySchema", "Array"),
        ("BooleanSchema", "Boolean"),
        ("ObjectSchema", "Object"),
        ("StringSchema", "String"),
        ("NullSchema", "Null"),
        ("$name", "$variant"),
    ] {
        let start = format!("impl<'de> Deserialize<'de> for {name} {{");
        let end = format!("impl Serialize for {name} {{");
        let body = format!(
            r#"impl<'de> Deserialize<'de> for {name} {{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {{
        let map = crate::flat::deserialize_map(deserializer)?;
        let decoded = crate::schema_fields::variant(crate::schema_serde::Source::new(map), crate::schema_kernel::SchemaKind::{variant})
            .map_err(serde::de::Error::custom)?;
        let DataSchema::{variant}(value) = crate::schema_serde::into_td(decoded) else {{ unreachable!() }};
        Ok(value)
    }}
}}

"#
        );
        replace_region(source, &start, &end, &body);
    }
    // RawValue and TypePeek stay in place, including ordinary conversion and
    // duplicate handling. Only the variant policy is extracted.
    source.replace_range(
        source.find("match peek.r#type.as_deref() {").unwrap()
            ..source.find("match peek.r#type.as_deref() {").unwrap()
                + "match peek.r#type.as_deref() {".len(),
        "match crate::schema_fields::dispatch(peek.r#type.as_deref()) {",
    );
    for variant in [
        "array", "boolean", "number", "integer", "object", "string", "null",
    ] {
        let name = format!("{}{}", variant[..1].to_uppercase(), &variant[1..]);
        *source = source.replace(
            &format!("Some(\"{variant}\") => serde_json::from_str"),
            &format!("crate::schema_kernel::SchemaKind::{name} => serde_json::from_str"),
        );
    }
    // Object now also represents the fallback and is exhaustive.
    replace_region(
        source,
        "            // A DataSchema without a recognized",
        "        }\n        .map_err",
        "",
    );
    // The flexible scalar visitor is still reused by both representations.
    replace_region(
        source,
        "/// Deserialize adapter carrying the flexible-bool",
        "/// Shared base for every",
        "",
    );
    *source = source.replace("use super::util::deserialize_bool_flexible;\n", "");
    *source = source.replace(
        "ExtensionMap, METADATA_KEYS, Metadata",
        "ExtensionMap, Metadata",
    );
}

fn replace_region(source: &mut String, start: &str, end: &str, replacement: &str) {
    assert_eq!(
        source.matches(start).count(),
        1,
        "component seam changed: {start}"
    );
    let first = source.find(start).unwrap();
    let last = first + source[first..].find(end).expect("component end changed");
    *source = format!("{}{}{}", &source[..first], replacement, &source[last..]);
}
