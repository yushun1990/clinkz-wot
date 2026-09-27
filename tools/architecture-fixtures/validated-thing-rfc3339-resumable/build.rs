//! Build two views of the current private TD RFC3339 owner.
//!
//! `existing.rs` is unchanged apart from documentation comments and is the
//! semantic oracle. `prototype.rs` retains the real module's serde adapter,
//! formatter, errors, and tests while replacing only its synchronous parser
//! region with the source-level resumable candidate checked into this probe.

use std::{env, fs, path::PathBuf};

const PARSER_START: &str = "/// Parses an RFC 3339 date-time string into an [`OffsetDateTime`].\n";
const ERROR_START: &str = "/// Parse failures for [`parse_rfc3339`].\n";

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let production = manifest.join("../../../td/src/rfc3339.rs");
    let replacement = manifest.join("src/resumable_parser.rs");
    println!("cargo:rerun-if-changed={}", production.display());
    println!("cargo:rerun-if-changed={}", replacement.display());

    let source = fs::read_to_string(&production).unwrap();
    assert_eq!(
        source.matches(PARSER_START).count(),
        1,
        "parser start moved"
    );
    assert_eq!(source.matches(ERROR_START).count(), 1, "error start moved");
    assert_eq!(
        source.matches("fn parse_rfc3339(input: &str)").count(),
        1,
        "production parser shape changed"
    );
    assert_eq!(
        source.matches("fn optional_fraction(&mut self)").count(),
        1,
        "production fraction scanner changed"
    );

    // `include!` cannot accept inner documentation comments in this position.
    let existing = source.replace("//!", "//");
    let parser_start = existing.find(PARSER_START).unwrap();
    let error_start = existing.find(ERROR_START).unwrap();
    assert!(parser_start < error_start);

    let candidate_parser = fs::read_to_string(&replacement).unwrap();
    let mut prototype = String::with_capacity(existing.len() + candidate_parser.len());
    prototype.push_str(&existing[..parser_start]);
    prototype.push_str(&candidate_parser);
    prototype.push('\n');
    prototype.push_str(&existing[error_start..]);
    prototype = prototype.replacen(
        "#[derive(Debug)]\nenum ParseError",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub(crate) enum ParseError",
        1,
    );
    assert!(prototype.contains("pub(crate) struct Decoder"));
    assert!(!prototype.contains("fn optional_fraction(&mut self)"));

    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(output.join("existing.rs"), existing).unwrap();
    fs::write(output.join("prototype.rs"), prototype).unwrap();
}
