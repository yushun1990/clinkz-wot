use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const EXPECTED_HEADER: &str = "field,resource_kind,unit,scope,capability_roles,zero_semantics,gateway_default_v1,directory_client_default_v1,benchmark_static_reference_v1,requirements";
const EXPECTED_RESOURCE_LIMIT_COUNT: usize = 116;

#[derive(Debug)]
struct LimitRow {
    field: String,
    variant: String,
    category: String,
    unit: String,
    scope: String,
    roles: String,
    zero_semantics: String,
    gateway: Option<u64>,
    directory: Option<u64>,
    constrained: Option<u64>,
}

fn main() {
    if let Err(error) = generate() {
        panic!("cannot generate resource limit schema: {error}");
    }
}

fn generate() -> Result<(), String> {
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| "CARGO_MANIFEST_DIR is not set".to_owned())?,
    );
    let schema_path = manifest_dir.join("../docs/resource-limits.csv");
    println!("cargo:rerun-if-changed={}", schema_path.display());
    let source = fs::read_to_string(&schema_path)
        .map_err(|error| format!("cannot read {}: {error}", schema_path.display()))?;
    let mut lines = source.lines();
    let header = lines
        .next()
        .ok_or_else(|| "resource limit schema is empty".to_owned())?;
    if header != EXPECTED_HEADER {
        return Err("resource limit schema header does not match revision v4.6".to_owned());
    }

    let mut rows = Vec::new();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let columns: Vec<&str> = line.split(',').collect();
        if columns.len() != 10 {
            return Err(format!(
                "schema line {} has {} columns; expected 10",
                offset + 2,
                columns.len()
            ));
        }
        validate_snake_case(columns[0], offset + 2)?;
        let row = LimitRow {
            field: columns[0].to_owned(),
            variant: upper_camel_case(columns[0]),
            category: nonempty(columns[1], "resource_kind", offset + 2)?.to_owned(),
            unit: nonempty(columns[2], "unit", offset + 2)?.to_owned(),
            scope: nonempty(columns[3], "scope", offset + 2)?.to_owned(),
            roles: nonempty(columns[4], "capability_roles", offset + 2)?.to_owned(),
            zero_semantics: nonempty(columns[5], "zero_semantics", offset + 2)?.to_owned(),
            gateway: parse_limit(columns[6], offset + 2)?,
            directory: parse_limit(columns[7], offset + 2)?,
            constrained: parse_limit(columns[8], offset + 2)?,
        };
        if rows.iter().any(|existing: &LimitRow| {
            existing.field == row.field || existing.variant == row.variant
        }) {
            return Err(format!("duplicate resource field on line {}", offset + 2));
        }
        rows.push(row);
    }
    if rows.len() != EXPECTED_RESOURCE_LIMIT_COUNT {
        return Err(format!(
            "revision v4.6 requires {EXPECTED_RESOURCE_LIMIT_COUNT} resource fields; found {}",
            rows.len()
        ));
    }

    let generated = render(&rows)?;
    let output_dir =
        PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| "OUT_DIR is not set".to_owned())?);
    write_generated(&output_dir.join("resource_limits.rs"), &generated)
}

fn render(rows: &[LimitRow]) -> Result<String, String> {
    let mut output = String::new();
    writeln!(
        output,
        "/// Number of fields in the revision v4.6 resource-limit schema.\npub const RESOURCE_LIMIT_COUNT: usize = {};",
        rows.len()
    )
    .map_err(|error| error.to_string())?;
    output.push_str(
        "\n/// Stable identity of one field in `docs/resource-limits.csv`.\n\
         #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]\n\
         #[repr(u16)]\n\
         pub enum ResourceKind {\n",
    );
    for row in rows {
        writeln!(output, "    {},", row.variant).map_err(|error| error.to_string())?;
    }
    output.push_str("}\n\nimpl ResourceKind {\n    /// Every schema field in stable CSV order.\n    pub const ALL: [Self; RESOURCE_LIMIT_COUNT] = [\n");
    for row in rows {
        writeln!(output, "        Self::{},", row.variant).map_err(|error| error.to_string())?;
    }
    output.push_str(
        "    ];\n\n    /// Returns the stable zero-based schema index.\n    pub const fn index(self) -> usize { self as usize }\n\n\
         /// Returns the CSV field name.\n    pub const fn field_name(self) -> &'static str { RESOURCE_FIELD_NAMES[self.index()] }\n\n\
         /// Returns the resource category.\n    pub const fn category(self) -> &'static str { RESOURCE_CATEGORIES[self.index()] }\n\n\
         /// Returns the accounting unit.\n    pub const fn unit(self) -> &'static str { RESOURCE_UNITS[self.index()] }\n\n\
         /// Returns the accounting scope.\n    pub const fn scope(self) -> &'static str { RESOURCE_SCOPES[self.index()] }\n\n\
         /// Returns the applicable capability-role expression.\n    pub const fn capability_roles(self) -> &'static str { RESOURCE_CAPABILITY_ROLES[self.index()] }\n\n\
         /// Returns the documented meaning of a configured zero.\n    pub const fn zero_semantics(self) -> &'static str { RESOURCE_ZERO_SEMANTICS[self.index()] }\n}\n",
    );
    render_string_array(&mut output, "RESOURCE_FIELD_NAMES", rows, |row| &row.field)?;
    render_string_array(&mut output, "RESOURCE_CATEGORIES", rows, |row| {
        &row.category
    })?;
    render_string_array(&mut output, "RESOURCE_UNITS", rows, |row| &row.unit)?;
    render_string_array(&mut output, "RESOURCE_SCOPES", rows, |row| &row.scope)?;
    render_string_array(&mut output, "RESOURCE_CAPABILITY_ROLES", rows, |row| {
        &row.roles
    })?;
    render_string_array(&mut output, "RESOURCE_ZERO_SEMANTICS", rows, |row| {
        &row.zero_semantics
    })?;
    render_limit_array(&mut output, "GATEWAY_DEFAULT_VALUES", rows, |row| {
        row.gateway
    })?;
    render_limit_array(
        &mut output,
        "DIRECTORY_CLIENT_DEFAULT_VALUES",
        rows,
        |row| row.directory,
    )?;
    render_limit_array(
        &mut output,
        "BENCHMARK_STATIC_REFERENCE_VALUES",
        rows,
        |row| row.constrained,
    )?;

    output.push_str("\nimpl ResourceLimits {\n");
    for row in rows {
        writeln!(
            output,
            "    /// Returns the configured `{}` value, or `None` when it is not applicable.\n    pub const fn {}(&self) -> Option<u64> {{ self.get(ResourceKind::{}) }}\n",
            row.field, row.field, row.variant
        )
        .map_err(|error| error.to_string())?;
    }
    output.push_str("}\n");
    Ok(output)
}

fn render_string_array<'a>(
    output: &mut String,
    name: &str,
    rows: &'a [LimitRow],
    value: impl Fn(&'a LimitRow) -> &'a str,
) -> Result<(), String> {
    writeln!(output, "\nconst {name}: [&str; RESOURCE_LIMIT_COUNT] = [")
        .map_err(|error| error.to_string())?;
    for row in rows {
        writeln!(output, "    {:?},", value(row)).map_err(|error| error.to_string())?;
    }
    output.push_str("];\n");
    Ok(())
}

fn render_limit_array(
    output: &mut String,
    name: &str,
    rows: &[LimitRow],
    value: impl Fn(&LimitRow) -> Option<u64>,
) -> Result<(), String> {
    writeln!(
        output,
        "\npub(crate) const {name}: [Option<u64>; RESOURCE_LIMIT_COUNT] = ["
    )
    .map_err(|error| error.to_string())?;
    for row in rows {
        match value(row) {
            Some(value) => writeln!(output, "    Some({value}),"),
            None => writeln!(output, "    None,"),
        }
        .map_err(|error| error.to_string())?;
    }
    output.push_str("];\n");
    Ok(())
}

fn write_generated(path: &Path, source: &str) -> Result<(), String> {
    fs::write(path, source).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn parse_limit(value: &str, line: usize) -> Result<Option<u64>, String> {
    if value == "NA" {
        return Ok(None);
    }
    value
        .parse::<u64>()
        .map(Some)
        .map_err(|error| format!("invalid profile limit {value:?} on line {line}: {error}"))
}

fn nonempty<'a>(value: &'a str, field: &str, line: usize) -> Result<&'a str, String> {
    if value.is_empty() {
        return Err(format!("empty {field} on line {line}"));
    }
    Ok(value)
}

fn validate_snake_case(value: &str, line: usize) -> Result<(), String> {
    let valid = !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if !valid || value.contains("__") {
        return Err(format!("invalid snake_case field {value:?} on line {line}"));
    }
    Ok(())
}

fn upper_camel_case(value: &str) -> String {
    let mut output = String::new();
    for word in value.split('_') {
        let mut characters = word.chars();
        if let Some(first) = characters.next() {
            output.extend(first.to_uppercase());
            output.extend(characters);
        }
    }
    output
}
