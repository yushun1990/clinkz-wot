use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Table};

const REVIEWED_CANDIDATE: &str = "3b133ebfe3c870102931982d6c056595f9d44255";
const REQUIREMENTS_HEADER: &str = "requirement,compilation_cells,execution_models,resource_profiles,capability_roles,owner_packages,evidence_kinds,evidence_key,source_path";
const API_OWNERSHIP_HEADER: &str = "item,kind,defining_package,defining_module,visibility,public_path,compilation_cells,execution_models,resource_profiles,capability_roles,requirements,current_path,migration_action,status";

#[test]
fn current_authority_projection_is_exact() {
    let root = repository_root();
    let manifest = parse_toml(&root, "docs/spec/v5-authority-reset.toml");

    assert_eq!(root_integer(&manifest, "schema_version"), 1);
    assert_eq!(root_string(&manifest, "current_design_revision"), "5.1");
    assert!(
        manifest.get("target_design_revision").is_none(),
        "active v5.1 manifest must not retain target_design_revision"
    );
    assert_eq!(root_string(&manifest, "status"), "active");
    assert_eq!(
        root_string(&manifest, "decision"),
        "docs/ADRs/0019-consumer-one-shot-authority-entry.org"
    );
    assert_eq!(
        root_string(&manifest, "workspace_basis"),
        "workspace/0061-consumer-one-shot-domain-entry.md"
    );
    assert_eq!(
        root_string(&manifest, "reviewed_candidate_commit"),
        REVIEWED_CANDIDATE
    );
    assert!(
        manifest.get("candidate_decision").is_none()
            && manifest.get("candidate_workspace_basis").is_none(),
        "active manifest must not retain candidate selector fields"
    );
    assert_eq!(root_integer(&manifest, "classified_requirement_count"), 121);
    assert_eq!(root_integer(&manifest, "active_requirement_count"), 65);

    let classification = manifest
        .get("classification")
        .and_then(Item::as_table)
        .expect("manifest has no classification table");

    let mut classified = BTreeSet::new();
    let mut active = BTreeSet::new();
    for (name, item) in classification {
        let table = item
            .as_table()
            .unwrap_or_else(|| panic!("classification.{name} is not a table"));
        let requirements = table_strings(table, "requirements", &format!("classification.{name}"));
        let expected = table_integer(table, "expected_count", &format!("classification.{name}"));
        let status = table_string(table, "authority_status", &format!("classification.{name}"));
        assert_eq!(
            expected as usize,
            requirements.len(),
            "classification.{name} expected_count does not match"
        );
        for requirement in requirements {
            assert!(
                classified.insert(requirement.clone()),
                "requirement {requirement} is classified more than once"
            );
            if status == "active" {
                active.insert(requirement);
            }
        }
    }

    assert_eq!(classified.len(), 121);
    assert_eq!(active.len(), 65);

    let consumer = classification
        .get("consumer_one_shot")
        .and_then(Item::as_table)
        .expect("classification.consumer_one_shot is missing");
    assert_eq!(
        table_strings(consumer, "requirements", "classification.consumer_one_shot"),
        vec![
            "PLAN-REQUEST-001".to_owned(),
            "BIND-OUT-001".to_owned(),
            "API-OPTIONS-001".to_owned(),
        ],
        "consumer_one_shot must contain exactly the three reviewed identities"
    );

    let deferred = classification
        .get("v1_deferred")
        .and_then(Item::as_table)
        .expect("classification.v1_deferred is missing");
    assert_eq!(
        table_string(deferred, "authority_status", "classification.v1_deferred"),
        "inactive-domain-entry-review-required"
    );
    assert_eq!(
        table_integer(deferred, "expected_count", "classification.v1_deferred"),
        31
    );

    let requirement_rows = requirement_rows(&root);
    let indexed: BTreeSet<_> = requirement_rows.keys().cloned().collect();
    assert_eq!(indexed, classified, "authority classification and requirements index differ");

    require_metadata(
        &requirement_rows,
        "PLAN-REQUEST-001",
        Some("docs/spec/planning.md"),
        Some("consumer"),
        Some("consumer-request-static-data"),
    );
    require_metadata(
        &requirement_rows,
        "BIND-OUT-001",
        Some("docs/spec/binding-spi.md"),
        Some("consumer"),
        None,
    );
    require_metadata(
        &requirement_rows,
        "API-OPTIONS-001",
        Some("docs/spec/interaction-core.md"),
        Some("consumer"),
        Some("consumer-selection-options"),
    );
    require_metadata(
        &requirement_rows,
        "BIND-DELIVERY-001",
        None,
        Some("consumer"),
        None,
    );

    require_consumer_response_validator_ownership(&root);

    let sources = manifest
        .get("active_source")
        .and_then(Item::as_array_of_tables)
        .expect("manifest has no active_source records");
    let expected_source_counts = BTreeMap::from([
        ("docs/spec/interaction-core.md", 11_i64),
        ("docs/spec/planning.md", 9_i64),
        ("docs/spec/binding-spi.md", 12_i64),
    ]);
    let mut sourced = BTreeSet::new();
    for source in sources {
        let path = table_string(source, "path", "active_source");
        assert!(
            !Path::new(path).is_absolute() && !Path::new(path).components().any(|part| matches!(part, std::path::Component::ParentDir)),
            "invalid active_source path {path}"
        );
        let requirements = table_strings(source, "requirements", &format!("active_source {path}"));
        let expected = table_integer(source, "expected_count", &format!("active_source {path}"));
        assert_eq!(
            expected as usize,
            requirements.len(),
            "active_source {path} expected_count does not match"
        );
        if let Some(exact) = expected_source_counts.get(path) {
            assert_eq!(expected, *exact, "active_source {path} has wrong v5.1 active count");
        }
        let text = fs::read_to_string(root.join(path))
            .unwrap_or_else(|error| panic!("cannot read active authority source {path}: {error}"));
        for requirement in requirements {
            assert!(active.contains(&requirement), "active_source {path} owns inactive requirement {requirement}");
            assert!(
                sourced.insert(requirement.clone()),
                "active requirement {requirement} has multiple sources"
            );
            let marker = format!("`{requirement}`:");
            assert!(text.contains(&marker), "active source {path} does not define {marker}");
        }
    }
    assert_eq!(sourced, active, "active classification and active sources differ");

    assert!(
        !root
            .join("docs/work-packages/CONSUMER-PROPERTY-READ-V5.1-CANDIDATE.md")
            .exists(),
        "candidate package projection must be migrated and removed after activation"
    );
    for relative in [
        "docs/work-packages/WP-100-core.md",
        "docs/work-packages/WP-200-planning.md",
        "docs/work-packages/WP-300-bindings.md",
        "docs/work-packages/WP-400-servient.md",
    ] {
        let text = fs::read_to_string(root.join(relative))
            .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"));
        let marker = "## v5.1 Consumer Property Read entry slice";
        assert_eq!(
            text.matches(marker).count(),
            1,
            "{relative} must contain exactly one activated Consumer entry slice"
        );
    }

    assert!(
        !root.join("tools/check-v5.1-authority-candidate.py").exists(),
        "candidate authority checker must be removed after activation"
    );
    assert!(
        !root.join("tools/check-architecture-adrs-candidate.sh").exists(),
        "candidate ADR checker must be removed after activation"
    );
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("cannot resolve repository root")
        .to_path_buf()
}

fn parse_toml(root: &Path, relative: &str) -> DocumentMut {
    fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"))
        .parse::<DocumentMut>()
        .unwrap_or_else(|error| panic!("invalid {relative}: {error}"))
}

fn root_string<'a>(document: &'a DocumentMut, key: &str) -> &'a str {
    document
        .get(key)
        .and_then(Item::as_str)
        .unwrap_or_else(|| panic!("manifest field {key} is missing or not a string"))
}

fn root_integer(document: &DocumentMut, key: &str) -> i64 {
    document
        .get(key)
        .and_then(Item::as_integer)
        .unwrap_or_else(|| panic!("manifest field {key} is missing or not an integer"))
}

fn table_string<'a>(table: &'a Table, key: &str, context: &str) -> &'a str {
    table
        .get(key)
        .and_then(Item::as_str)
        .unwrap_or_else(|| panic!("{context}.{key} is missing or not a string"))
}

fn table_integer(table: &Table, key: &str, context: &str) -> i64 {
    table
        .get(key)
        .and_then(Item::as_integer)
        .unwrap_or_else(|| panic!("{context}.{key} is missing or not an integer"))
}

fn table_strings(table: &Table, key: &str, context: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(Item::as_array)
        .unwrap_or_else(|| panic!("{context}.{key} is missing or not an array"))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("{context}.{key} contains a non-string value"))
                .to_owned()
        })
        .collect()
}

fn expand_requirement(expression: &str) -> Vec<String> {
    let Some((first, last)) = expression.split_once("..") else {
        assert!(!expression.is_empty(), "empty requirement expression");
        return vec![expression.to_owned()];
    };
    assert!(first.len() >= 4 && last.len() == 3, "invalid requirement range {expression}");
    let (prefix, first) = first.split_at(first.len() - 3);
    let first = first
        .parse::<u16>()
        .unwrap_or_else(|_| panic!("invalid requirement range {expression}"));
    let last = last
        .parse::<u16>()
        .unwrap_or_else(|_| panic!("invalid requirement range {expression}"));
    assert!(first <= last, "descending requirement range {expression}");
    (first..=last)
        .map(|number| format!("{prefix}{number:03}"))
        .collect()
}

fn csv_rows(root: &Path, relative: &str, header: &str, columns: usize) -> Vec<Vec<String>> {
    let source = fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"));
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(header), "{relative} has an unexpected header");
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<String> = line.split(',').map(str::to_owned).collect();
            assert_eq!(fields.len(), columns, "{relative} row has an unexpected column count: {line}");
            fields
        })
        .collect()
}

fn requirement_rows(root: &Path) -> BTreeMap<String, Vec<String>> {
    let mut result = BTreeMap::new();
    for row in csv_rows(root, "docs/requirements.csv", REQUIREMENTS_HEADER, 9) {
        for expression in row[0].split('|') {
            for requirement in expand_requirement(expression) {
                assert!(
                    result.insert(requirement.clone(), row.clone()).is_none(),
                    "duplicate indexed requirement {requirement}"
                );
            }
        }
    }
    result
}

fn require_metadata(
    rows: &BTreeMap<String, Vec<String>>,
    requirement: &str,
    source: Option<&str>,
    role: Option<&str>,
    evidence: Option<&str>,
) {
    let row = rows
        .get(requirement)
        .unwrap_or_else(|| panic!("missing metadata row for {requirement}"));
    if let Some(source) = source {
        assert_eq!(row[8], source, "{requirement} source_path is wrong");
    }
    if let Some(evidence) = evidence {
        assert_eq!(row[7], evidence, "{requirement} evidence_key is wrong");
    }
    if let Some(role) = role {
        assert!(
            row[4].split('|').any(|candidate| candidate == role),
            "{requirement} metadata does not include capability role {role}"
        );
    }
}

fn require_consumer_response_validator_ownership(root: &Path) {
    let rows = csv_rows(root, "docs/api-ownership.csv", API_OWNERSHIP_HEADER, 14);
    let matches: Vec<&Vec<String>> = rows
        .iter()
        .filter(|row| row[0] == "validate_untrusted_binding_output")
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "api ownership must contain exactly one validate_untrusted_binding_output row"
    );
    let row = matches[0];
    for (index, expected) in [
        (1, "function"),
        (2, "clinkz-wot-core"),
        (3, "response"),
        (4, "public"),
        (5, "clinkz_wot_core::validate_untrusted_binding_output"),
        (6, "no-default|async-no-std|std"),
        (7, "all"),
        (8, "all"),
        (9, "consumer"),
        (10, "API-PAYLOAD-001|BIND-OUT-001"),
        (11, "absent"),
        (12, "add"),
        (13, "frozen"),
    ] {
        assert_eq!(
            row[index], expected,
            "validate_untrusted_binding_output has wrong field at column {index}"
        );
    }
    assert_ne!(row[2], "clinkz-wot-planning");
    assert!(
        !row[5].starts_with("clinkz_wot_planning::"),
        "Consumer response validator must not be Planning-owned"
    );
}
