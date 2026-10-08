// Non-production source projection. Read today's implementations; change only
// imports/signatures/storage observations. No predicate body is maintained here.
use std::{env, fs, path::PathBuf};

fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    assert_eq!(
        source.matches(start).count(),
        1,
        "source seam changed: {start}"
    );
    assert_eq!(source.matches(end).count(), 1, "source seam changed: {end}");
    let start = source.find(start).unwrap();
    let end = source.find(end).unwrap();
    assert!(start < end, "source seam order changed");
    &source[start..end]
}

fn replace_once(source: &str, before: &str, after: &str) -> String {
    assert_eq!(
        source.matches(before).count(),
        1,
        "adapter seam changed: {before}"
    );
    source.replace(before, after)
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let read = |name: &str| {
        let path = root.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        fs::read_to_string(path).unwrap()
    };
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let write = |name: &str, source: String| fs::write(out.join(name), source).unwrap();

    let identity = read("core/src/identity.rs");
    let identity = format!(
        "{}{}",
        between(
            &identity,
            "//! Opaque identity",
            "/// Canonical Thing identity."
        ),
        between(&identity, "/// Opaque, core-owned token", "#[cfg(test)]")
    );
    write(
        "identity.rs",
        replace_once(&identity, "use alloc::string::String;\n", "").replace("//!", "//"),
    );

    let error = read("core/src/error.rs");
    write(
        "error.rs",
        replace_once(
            between(&error, "use core::fmt;", "#[cfg(test)]"),
            "use clinkz_wot_td::data_type::Operation;",
            "use crate::Operation;",
        ),
    );

    let operation = read("td/src/core/data_type/operation.rs");
    let operation = replace_once(&operation, "use serde::{Deserialize, Serialize};\n", "");
    let operation = replace_once(&operation, ", Serialize, Deserialize", "");
    write(
        "operation.rs",
        replace_once(&operation, "#[serde(rename_all = \"lowercase\")]\n", "").replace("//!", "//"),
    );

    let interaction = read("core/src/interaction.rs");
    write(
        "metadata.rs",
        format!(
            "use crate::*;\nuse clinkz_wot_foundation::ResourceLimits;\n{}",
            between(
                &interaction,
                "/// Normalized completion status",
                "/// Output returned by an interaction handler"
            )
        ),
    );

    let artifact = read("core/src/binding_compiler.rs");
    write(
        "artifact.rs",
        format!(
            "use crate::*;\nuse clinkz_wot_foundation::SlotIndex;\n{}{}{}",
            between(
                &artifact,
                "/// Stable compatibility identity",
                "/// Measured retained lifetime footprint"
            ),
            between(
                &artifact,
                "/// Complete generation-qualified identity",
                "/// Read-only resolved input"
            ),
            between(
                &artifact,
                "/// Compact reference to one immutable artifact slot.",
                "/// Typed compiler component"
            )
        ),
    );

    let response = read("core/src/response.rs");
    let response = between(
        &response,
        "pub(crate) fn validate_property_read_binding_output(",
        "#[cfg(test)]",
    );
    let response = replace_once(
        response,
        "pub(crate) fn validate_property_read_binding_output(",
        "pub fn validate_property_read_binding_output<'p>(",
    );
    let response = replace_once(
        &response,
        "output: InteractionOutput,",
        "output: InteractionOutput<'p>,",
    );
    let response = replace_once(
        &response,
        ") -> CoreResult<InteractionOutput> {",
        ") -> CoreResult<InteractionOutput<'p>> {",
    );
    write("response.rs", format!("use crate::*;\n{response}"));

    let planning = read("planning/src/property_read.rs");
    let selector = between(
        &planning,
        "pub fn select_consumer_property_read<A>(",
        "\nfn finish_property_read_build",
    );
    let selector = replace_once(
        selector,
        "pub fn select_consumer_property_read<A>(",
        "pub fn select_consumer_property_read(",
    );
    let selector = replace_once(
        &selector,
        "output: &PlanBuildOutput<A>,",
        "output: &FrozenPlanView<'_>,",
    );
    let error = between(
        &planning,
        "fn selection_error(reason:",
        "\nfn compiler_contract_error",
    );
    write(
        "selection.rs",
        format!(
            "use crate::*;\nuse crate as clinkz_wot_core;\nuse clinkz_wot_foundation::SlotIndex;\n{selector}\n{error}"
        ),
    );
}
