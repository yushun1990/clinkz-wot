//! Validates machine-readable design artifacts that require structured parsing.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table};

const ACTIVE_DESIGN_REVISION: &str = "4.8";
const WP000_EVIDENCE_REVISION: &str = "4.6";
const ARCHITECTURE_CLOSURE_REVIEW: &str = "architecture-closure-2026-07-16-v4.8";
const ARCHITECTURE_REVIEW_02_OPEN: &str = "architecture-review-02-2026-07-16-open";

const REQUIRED_MACHINES: &[&str] = &[
    "binding-emission-slot",
    "binding-route",
    "directory-process",
    "emission-coordinator",
    "expose",
    "handler-async-execution",
    "handler-step-execution",
    "handler-sync-execution",
    "in-flight",
    "producer-subscription",
    "subscription",
    "subscription-driver-slot",
];
const REQUIRED_COMPOSITIONS: &[&str] = &[
    "handler-cancellation-response",
    "handler-direct-response",
    "producer-late-start-result-transfer",
    "producer-prepublication-failure-response",
    "producer-setup-abort",
    "producer-start-publication",
    "producer-start-result-transfer",
    "producer-teardown-handoff",
    "producer-teardown-result-and-response",
    "producer-terminal-replay-and-release",
];
const REQUIRED_WORK_PACKAGES: &[&str] = &[
    "WP-000", "WP-100", "WP-200", "WP-300", "WP-400", "WP-500", "WP-600", "WP-700",
];
const REQUIRED_WORK_PACKAGE_HEADINGS: &[&str] = &[
    "Scope",
    "Requirements",
    "Crates and Feature Cells",
    "Public API and Data Migration",
    "State and Ownership Migration",
    "Old API Removal",
    "Evidence",
    "Performance Workloads",
    "Completion Conditions",
];
const REQUIRED_REFACTOR_GATES: &[&str] =
    &["GATE-1", "GATE-2", "GATE-3", "GATE-4", "GATE-5", "GATE-6"];
const HANDLER_ENTRYPOINT: &str = "WP-100-HANDLER-ENTRY";
const HANDLER_FOUNDATION_TRANCHE: &str = "WP-100-FOUNDATION-REFRESH";

#[derive(Debug)]
struct Transition {
    from: String,
    event: String,
    to: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HandlerOperation {
    operation: &'static str,
    method: &'static str,
    result: &'static str,
    target: &'static str,
}

const HANDLER_OPERATIONS: &[HandlerOperation] = &[
    HandlerOperation {
        operation: "ReadProperty",
        method: "read_property",
        result: "InteractionOutput",
        target: "Property",
    },
    HandlerOperation {
        operation: "WriteProperty",
        method: "write_property",
        result: "InteractionOutput",
        target: "Property",
    },
    HandlerOperation {
        operation: "ObserveProperty",
        method: "observe_property",
        result: "SubscriptionAcceptance",
        target: "Property",
    },
    HandlerOperation {
        operation: "UnobserveProperty",
        method: "unobserve_property",
        result: "InteractionOutput",
        target: "Property",
    },
    HandlerOperation {
        operation: "InvokeAction",
        method: "invoke_action",
        result: "InteractionOutput",
        target: "Action",
    },
    HandlerOperation {
        operation: "QueryAction",
        method: "query_action",
        result: "InteractionOutput",
        target: "Action",
    },
    HandlerOperation {
        operation: "CancelAction",
        method: "cancel_action",
        result: "InteractionOutput",
        target: "Action",
    },
    HandlerOperation {
        operation: "SubscribeEvent",
        method: "subscribe_event",
        result: "SubscriptionAcceptance",
        target: "Event",
    },
    HandlerOperation {
        operation: "UnsubscribeEvent",
        method: "unsubscribe_event",
        result: "InteractionOutput",
        target: "Event",
    },
    HandlerOperation {
        operation: "ReadAllProperties",
        method: "read_all_properties",
        result: "InteractionOutput",
        target: "Thing",
    },
    HandlerOperation {
        operation: "WriteAllProperties",
        method: "write_all_properties",
        result: "InteractionOutput",
        target: "Thing",
    },
    HandlerOperation {
        operation: "ReadMultipleProperties",
        method: "read_multiple_properties",
        result: "InteractionOutput",
        target: "Thing",
    },
    HandlerOperation {
        operation: "WriteMultipleProperties",
        method: "write_multiple_properties",
        result: "InteractionOutput",
        target: "Thing",
    },
    HandlerOperation {
        operation: "ObserveAllProperties",
        method: "observe_all_properties",
        result: "SubscriptionAcceptance",
        target: "Thing",
    },
    HandlerOperation {
        operation: "UnobserveAllProperties",
        method: "unobserve_all_properties",
        result: "InteractionOutput",
        target: "Thing",
    },
    HandlerOperation {
        operation: "QueryAllActions",
        method: "query_all_actions",
        result: "InteractionOutput",
        target: "Thing",
    },
    HandlerOperation {
        operation: "SubscribeAllEvents",
        method: "subscribe_all_events",
        result: "SubscriptionAcceptance",
        target: "Thing",
    },
    HandlerOperation {
        operation: "UnsubscribeAllEvents",
        method: "unsubscribe_all_events",
        result: "InteractionOutput",
        target: "Thing",
    },
];

fn main() {
    if let Err(error) = run() {
        eprintln!("design structure check: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let command = env::args().nth(1).unwrap_or_else(|| "check".to_owned());
    let root = repository_root()?;
    match command.as_str() {
        "check" => {
            check_state_machines(&root)?;
            println!("design structure check: state machines valid");
            check_handler_contract(&root)?;
            println!("design structure check: handler API matrix valid");
            check_work_packages(&root, false)?;
            println!("design structure check: work-package DAG valid");
            check_governance(&root, false)?;
            println!("design structure check: gate governance valid");
        }
        "check-state" => {
            check_state_machines(&root)?;
            println!("design structure check: state machines valid");
        }
        "check-work-packages" => {
            check_work_packages(&root, false)?;
            println!("design structure check: work-package DAG valid");
        }
        "check-handler" => {
            check_handler_contract(&root)?;
            println!("design structure check: handler API matrix valid");
        }
        "check-governance" => {
            check_governance(&root, false)?;
            println!("design structure check: gate governance valid");
        }
        "check-refactor-ready" => {
            check_governance(&root, true)?;
            println!("design structure check: all refactor gates ready");
        }
        "check-handler-entry" => {
            check_governance(&root, true)?;
            check_work_packages(&root, true)?;
            println!("design structure check: WP-100 handler entry ready");
        }
        _ => {
            return Err(format!(
                "unknown command {command:?}; expected check, check-state, check-handler, \
                 check-work-packages, check-governance, check-refactor-ready, or \
                 check-handler-entry"
            ));
        }
    }
    Ok(())
}

fn check_handler_contract(root: &Path) -> Result<(), String> {
    let amendment_path = root.join("docs/amendments/WP-100-handler-api-v1.md");
    let amendment = fs::read_to_string(&amendment_path)
        .map_err(|error| format!("cannot read {}: {error}", amendment_path.display()))?;
    let table_start = amendment
        .find("| Operation | Sync trait | Async trait | Step trait | Success value | Target |")
        .ok_or_else(|| "handler amendment has no exact operation matrix".to_owned())?;
    let table = &amendment[table_start..];
    let table_end = table
        .find("\n\n### Synchronous traits")
        .ok_or_else(|| "handler operation matrix has no closing section".to_owned())?;

    let mut actual = BTreeSet::new();
    for line in table[..table_end].lines().skip(2) {
        if !line.starts_with('|') {
            continue;
        }
        let columns: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|value| value.trim().trim_matches('`').to_owned())
            .collect();
        if columns.len() != 6 {
            return Err(format!(
                "handler operation row has {} columns: {line}",
                columns.len()
            ));
        }
        actual.insert((
            columns[0].clone(),
            columns[1].clone(),
            columns[2].clone(),
            columns[3].clone(),
            columns[4].clone(),
            columns[5].clone(),
        ));
    }

    let expected: BTreeSet<_> = HANDLER_OPERATIONS
        .iter()
        .map(|operation| {
            (
                operation.operation.to_owned(),
                format!("{}Handler", operation.operation),
                format!("Async{}Handler", operation.operation),
                format!("Step{}Handler", operation.operation),
                operation.result.to_owned(),
                operation.target.to_owned(),
            )
        })
        .collect();
    if actual != expected {
        let missing: Vec<_> = expected.difference(&actual).collect();
        let unexpected: Vec<_> = actual.difference(&expected).collect();
        return Err(format!(
            "handler operation matrix mismatch; missing {missing:?}, unexpected {unexpected:?}"
        ));
    }

    let ownership_path = root.join("docs/api-ownership.csv");
    let ownership = fs::read_to_string(&ownership_path)
        .map_err(|error| format!("cannot read {}: {error}", ownership_path.display()))?;
    let mut rows = BTreeMap::new();
    for (index, line) in ownership.lines().enumerate().skip(1) {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 14 {
            return Err(format!(
                "api ownership line {} has {} columns",
                index + 1,
                fields.len()
            ));
        }
        if rows.insert(fields[0], fields).is_some() {
            return Err(format!(
                "duplicate API ownership item on line {}",
                index + 1
            ));
        }
    }

    for operation in HANDLER_OPERATIONS {
        check_handler_ownership_row(
            &rows,
            &format!("{}Handler", operation.operation),
            "clinkz-wot-core",
            "handler",
            &format!("clinkz_wot_core::{}Handler", operation.operation),
            "no-default|async-no-std|std",
        )?;
        check_handler_ownership_row(
            &rows,
            &format!("Async{}Handler", operation.operation),
            "clinkz-wot-core",
            "handler",
            &format!("clinkz_wot_core::Async{}Handler", operation.operation),
            "async-no-std|std-async",
        )?;
        check_handler_ownership_row(
            &rows,
            &format!("Step{}Handler", operation.operation),
            "clinkz-wot-core",
            "handler",
            &format!("clinkz_wot_core::Step{}Handler", operation.operation),
            "no-default|async-no-std|std",
        )?;

        for (prefix, cells) in [
            ("set_", "std"),
            ("set_async_", "std-async"),
            ("set_step_", "std"),
            ("clear_", "std"),
        ] {
            let item = format!("{prefix}{}_handler", operation.method);
            check_handler_ownership_row(
                &rows,
                &item,
                "clinkz-wot-servient",
                "runtime",
                &format!("clinkz_wot_servient::ExposedThingHandle::{item}"),
                cells,
            )?;
        }
    }

    for item in ["HostHandlerFuture", "HostAsyncAdapter"] {
        let row = rows
            .get(item)
            .ok_or_else(|| format!("missing API ownership row {item:?}"))?;
        if row[2] != "clinkz-wot-core"
            || row[3] != "handler"
            || row[4] != "crate"
            || row[5] != "-"
            || row[6] != "std-async"
            || row[13] != "frozen"
        {
            return Err(format!(
                "invalid crate-private host adapter ownership for {item:?}"
            ));
        }
    }
    if rows.contains_key("HandlerFuture") {
        return Err("HandlerFuture must not be a public or separately frozen API item".to_owned());
    }
    for (item, action) in [
        ("AffordanceKind", "relocate"),
        ("AffordanceTarget", "replace"),
    ] {
        let row = rows
            .get(item)
            .ok_or_else(|| format!("missing API ownership row {item:?}"))?;
        if row[2] != "clinkz-wot-core"
            || row[3] != "interaction"
            || row[4] != "public"
            || row[5] != format!("clinkz_wot_core::{item}")
            || row[11] != "core/src/thing.rs"
            || row[12] != action
            || row[13] != "frozen"
        {
            return Err(format!(
                "invalid Affordance ownership migration for {item:?}"
            ));
        }
    }
    let result_sink = rows
        .get("HandlerResultSink")
        .ok_or_else(|| "missing HandlerResultSink ownership".to_owned())?;
    if result_sink[2] != "clinkz-wot-core"
        || result_sink[3] != "handler"
        || result_sink[4] != "crate"
        || result_sink[5] != "-"
        || result_sink[13] != "frozen"
    {
        return Err("invalid HandlerResultSink ownership".to_owned());
    }

    let failure_header =
        "| Boundary | Condition | Error category | Phase | Retry class | Observable result |";
    let failure_start = amendment
        .find(failure_header)
        .ok_or_else(|| "handler amendment has no public failure matrix".to_owned())?;
    let mut failure_rows = Vec::new();
    for line in amendment[failure_start..].lines().skip(2) {
        if !line.starts_with('|') {
            break;
        }
        let columns: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|value| value.trim().trim_matches('`').to_owned())
            .collect();
        if columns.len() != 6 {
            return Err(format!(
                "handler failure row has {} columns: {line}",
                columns.len()
            ));
        }
        failure_rows.push(columns);
    }
    for expected in [
        [
            "Context construction",
            "HandlerContext::try_new",
            "CoreError::Validation",
            "ErrorPhase::Validate",
            "RetryClass::Never",
            "No context is constructed",
        ],
        [
            "Registration or clear",
            "does not exist",
            "CoreError::NotFound",
            "ErrorPhase::Admission",
            "RetryClass::Never",
            "No slot or counter changes",
        ],
        [
            "Registration or clear",
            "operation was not admitted",
            "CoreError::UnsupportedOperation",
            "ErrorPhase::Admission",
            "RetryClass::Never",
            "No slot or counter changes",
        ],
        [
            "Dispatch",
            "no handler flavor",
            "CoreError::UnsupportedOperation",
            "ErrorPhase::Handler",
            "RetryClass::Never",
            "Application code is not entered",
        ],
        [
            "Registration or static admission",
            "subscription_bytes",
            "CoreError::Validation",
            "ErrorPhase::Admission",
            "RetryClass::Never",
            "rejected without changing a slot or counter",
        ],
        [
            "Registration or replacement",
            "projected total exceeds",
            "CoreError::LimitExceeded",
            "ErrorPhase::Admission",
            "RetryClass::Never",
            "old published generation",
        ],
        [
            "Replacement",
            "third live generation",
            "CoreError::Backpressure",
            "ErrorPhase::Admission",
            "RetryClass::Safe",
            "old published generation",
        ],
        [
            "Clear",
            "no handler flavor",
            "None",
            "None",
            "None",
            "Ok(false)",
        ],
    ] {
        let matched = failure_rows.iter().any(|row| {
            row.iter()
                .zip(expected)
                .all(|(actual, fragment)| actual.contains(fragment))
        });
        if !matched {
            return Err(format!(
                "handler public failure matrix is missing contract row {expected:?}"
            ));
        }
    }

    Ok(())
}

fn check_handler_ownership_row(
    rows: &BTreeMap<&str, Vec<&str>>,
    item: &str,
    package: &str,
    module: &str,
    path: &str,
    cells: &str,
) -> Result<(), String> {
    let row = rows
        .get(item)
        .ok_or_else(|| format!("missing API ownership row {item:?}"))?;
    if row[2] != package
        || row[3] != module
        || row[4] != "public"
        || row[5] != path
        || row[6] != cells
        || row[13] != "frozen"
    {
        return Err(format!(
            "API ownership row {item:?} does not match the handler contract"
        ));
    }
    Ok(())
}

fn repository_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "cannot resolve repository root".to_owned())
}

fn check_state_machines(root: &Path) -> Result<(), String> {
    let path = root.join("docs/state-machines.toml");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;

    require_integer(document.get("schema_version"), "schema_version", 1)?;
    require_string(
        document.get("design_revision"),
        "design_revision",
        ACTIVE_DESIGN_REVISION,
    )?;
    let machines = document
        .get("machine")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "state artifact has no [[machine]] entries".to_owned())?;
    let design = fs::read_to_string(root.join("docs/design.md"))
        .map_err(|error| format!("cannot read docs/design.md: {error}"))?;

    let mut ids = BTreeSet::new();
    let mut machine_transitions = BTreeMap::new();
    for machine in machines {
        let (id, transitions) = check_machine(machine, &design, &mut ids)?;
        machine_transitions.insert(id, transitions);
    }

    let expected: BTreeSet<String> = REQUIRED_MACHINES
        .iter()
        .map(|value| (*value).to_owned())
        .collect();
    if ids != expected {
        return Err(format!(
            "state machine set mismatch; expected {expected:?}, found {ids:?}"
        ));
    }
    check_compositions(&document, &design, &machine_transitions)?;
    Ok(())
}

#[derive(Debug)]
struct GovernanceReview {
    design_revision: String,
    status: String,
    review_type: String,
    basis_revision: Option<String>,
    gates: BTreeSet<String>,
    artifacts: BTreeSet<String>,
    checks: BTreeSet<String>,
    impact: String,
}

fn check_governance(root: &Path, require_ready: bool) -> Result<(), String> {
    let registered_artifacts = load_artifact_registry(root)?;
    if !registered_artifacts.contains("docs/governance.toml") {
        return Err("docs/governance.toml is not registered as an active artifact".to_owned());
    }

    let governance_path = root.join("docs/governance.toml");
    let source = fs::read_to_string(&governance_path)
        .map_err(|error| format!("cannot read {}: {error}", governance_path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", governance_path.display()))?;
    require_integer(
        document.get("schema_version"),
        "governance schema_version",
        1,
    )?;
    require_string(
        document.get("design_revision"),
        "governance design_revision",
        ACTIVE_DESIGN_REVISION,
    )?;

    let aliases = document
        .get("artifact_alias")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "governance artifact has no [[artifact_alias]] records".to_owned())?;
    let expected_aliases: BTreeMap<&str, (&str, &str)> = [
        (
            "tools/performance-harness",
            ("tools/performance-harness/Cargo.toml", "package-root"),
        ),
        (
            "docs/future/directory-service.md",
            ("docs/governance.toml", "deferred-review-input"),
        ),
    ]
    .into_iter()
    .collect();
    let mut artifact_aliases = BTreeMap::new();
    for alias in aliases {
        let path = string_field(alias, "path", "artifact alias")?;
        validate_relative_path(&path, "artifact alias")?;
        if registered_artifacts.contains(&path) {
            return Err(format!(
                "artifact alias {path:?} duplicates a directly registered artifact"
            ));
        }
        if !root.join(&path).exists() {
            return Err(format!("artifact alias target {path:?} does not exist"));
        }
        let registered_by = string_field(alias, "registered_by", &path)?;
        if !registered_artifacts.contains(&registered_by) {
            return Err(format!(
                "artifact alias {path:?} is registered by unknown artifact {registered_by:?}"
            ));
        }
        let kind = string_field(alias, "kind", &path)?;
        let expected = expected_aliases
            .get(path.as_str())
            .ok_or_else(|| format!("unreviewed artifact alias {path:?}"))?;
        if (registered_by.as_str(), kind.as_str()) != *expected {
            return Err(format!(
                "artifact alias {path:?} mismatch; expected {expected:?}, found \
                 {:?}",
                (registered_by, kind)
            ));
        }
        if artifact_aliases
            .insert(path.clone(), registered_by)
            .is_some()
        {
            return Err(format!("duplicate artifact alias {path:?}"));
        }
    }
    let actual_aliases: BTreeSet<&str> = artifact_aliases.keys().map(String::as_str).collect();
    let expected_alias_paths: BTreeSet<&str> = expected_aliases.keys().copied().collect();
    if actual_aliases != expected_alias_paths {
        return Err(format!(
            "artifact alias set mismatch; expected {expected_alias_paths:?}, found \
             {actual_aliases:?}"
        ));
    }

    let check_statuses = check_governance_checks(&document, &registered_artifacts)?;
    let resolved_artifacts: BTreeSet<String> = registered_artifacts
        .iter()
        .cloned()
        .chain(artifact_aliases.keys().cloned())
        .collect();
    let reviews = check_governance_reviews(&document, &resolved_artifacts, &check_statuses)?;

    let known_requirements = load_requirement_ids(root)?;
    let gate_path = root.join("docs/refactor-gates.csv");
    let gate_source = fs::read_to_string(&gate_path)
        .map_err(|error| format!("cannot read {}: {error}", gate_path.display()))?;
    let mut lines = gate_source.lines();
    let expected_header = "gate,status,requirements,artifacts,checks,review_evidence";
    if lines.next() != Some(expected_header) {
        return Err(format!("{} has an invalid header", gate_path.display()));
    }
    let expected_gates = owned_set(REQUIRED_REFACTOR_GATES);
    let mut gate_ids = BTreeSet::new();
    let mut referenced_reviews = BTreeSet::new();
    let mut open_gates = Vec::new();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!(
                "{} has a blank data row on line {}",
                gate_path.display(),
                offset + 2
            ));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 6 {
            return Err(format!(
                "{} line {} has {} columns; expected 6",
                gate_path.display(),
                offset + 2,
                fields.len()
            ));
        }
        let gate = fields[0];
        if !expected_gates.contains(gate) || !gate_ids.insert(gate.to_owned()) {
            return Err(format!("unknown or duplicate refactor gate {gate:?}"));
        }
        let status = fields[1];
        if !matches!(status, "open" | "closed") {
            return Err(format!("{gate} has invalid status {status:?}"));
        }
        let requirements = pipe_set(fields[2], gate, "requirements")?;
        check_known_values(gate, "requirement", &requirements, &known_requirements)?;
        let artifacts = pipe_set(fields[3], gate, "artifacts")?;
        check_known_values(gate, "artifact", &artifacts, &resolved_artifacts)?;
        let checks = pipe_set(fields[4], gate, "checks")?;
        check_known_values(
            gate,
            "check",
            &checks,
            &check_statuses.keys().cloned().collect(),
        )?;
        let (expected_requirements, expected_artifacts, expected_checks) =
            expected_gate_contract(gate)
                .ok_or_else(|| format!("no frozen contract for refactor gate {gate:?}"))?;
        if requirements != expected_requirements {
            return Err(format!(
                "{gate} requirement set mismatch; expected {expected_requirements:?}, found {requirements:?}"
            ));
        }
        if artifacts != expected_artifacts {
            return Err(format!(
                "{gate} artifact set mismatch; expected {expected_artifacts:?}, found {artifacts:?}"
            ));
        }
        if checks != expected_checks {
            return Err(format!(
                "{gate} check set mismatch; expected {expected_checks:?}, found {checks:?}"
            ));
        }
        let review_id = fields[5];
        if review_id.is_empty() {
            return Err(format!("{gate} has no review evidence id"));
        }
        let review = reviews
            .get(review_id)
            .ok_or_else(|| format!("{gate} references unknown review {review_id:?}"))?;
        referenced_reviews.insert(review_id.to_owned());
        if !review.gates.contains(gate) {
            return Err(format!(
                "review {review_id:?} does not declare its use by {gate}"
            ));
        }
        if !artifacts.is_subset(&review.artifacts) {
            let missing: Vec<_> = artifacts.difference(&review.artifacts).collect();
            return Err(format!(
                "review {review_id:?} does not cover {gate} artifacts {missing:?}"
            ));
        }
        if !checks.is_subset(&review.checks) {
            let missing: Vec<_> = checks.difference(&review.checks).collect();
            return Err(format!(
                "review {review_id:?} does not cover {gate} checks {missing:?}"
            ));
        }

        if status == "closed" {
            if review.design_revision != ACTIVE_DESIGN_REVISION || review.status != "passed" {
                return Err(format!(
                    "{gate} closure requires a passed v{ACTIVE_DESIGN_REVISION} review; \
                     {review_id:?} is status {:?} at revision {:?}",
                    review.status, review.design_revision
                ));
            }
            for check in &checks {
                if check_statuses.get(check).map(String::as_str) != Some("executable") {
                    return Err(format!(
                        "{gate} cannot close with non-executable check {check:?}"
                    ));
                }
            }
        } else {
            open_gates.push(gate.to_owned());
        }

        if gate != "GATE-3"
            && (status != "open"
                || review_id != ARCHITECTURE_REVIEW_02_OPEN
                || review.review_type != "audit"
                || review.design_revision != ACTIVE_DESIGN_REVISION
                || review.status != "blocking"
                || !review.artifacts.contains("docs/review/review-02.org"))
        {
            return Err(format!(
                "{gate} must remain open under the blocking v{ACTIVE_DESIGN_REVISION} \
                 Architecture Review 02 record"
            ));
        }

        if gate == "GATE-3"
            && (status != "closed"
                || review_id != "directory-client-v4.6-review"
                || review.review_type != "carry-forward"
                || review.basis_revision.as_deref() != Some("4.6")
                || review.design_revision != ACTIVE_DESIGN_REVISION
                || review.status != "passed"
                || review.impact.trim().is_empty())
        {
            return Err(
                "GATE-3 must use the passed v4.8 carry-forward review based on v4.6".to_owned(),
            );
        }
    }
    if gate_ids != expected_gates {
        return Err(format!(
            "refactor gate set mismatch; expected {expected_gates:?}, found {gate_ids:?}"
        ));
    }
    let review_ids: BTreeSet<String> = reviews.keys().cloned().collect();
    if referenced_reviews != review_ids {
        let unused: Vec<_> = review_ids.difference(&referenced_reviews).collect();
        return Err(format!(
            "governance contains unreferenced reviews {unused:?}"
        ));
    }
    if require_ready && !open_gates.is_empty() {
        return Err(format!(
            "refactor admission blocked: gates remain open {}",
            open_gates.join(", ")
        ));
    }
    Ok(())
}

type GateContract = (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>);

fn expected_gate_contract(gate: &str) -> Option<GateContract> {
    let contract = match gate {
        "GATE-1" => (
            owned_set(&[
                "API-OWNERSHIP-001",
                "API-SURFACE-001",
                "API-PAYLOAD-001",
                "HANDLER-API-001",
                "HANDLER-SUB-001",
                "HANDLER-CANCEL-001",
                "HANDLER-CANCEL-002",
                "ERR-TAXONOMY-001",
                "ERR-RETRY-001",
                "CLEANUP-RECORD-001",
            ]),
            owned_set(&[
                "docs/design.md",
                "docs/api-ownership.csv",
                "docs/amendments/WP-100-error-cleanup-v1.md",
                "docs/amendments/WP-100-error-disposition-v1.md",
                "docs/amendments/WP-100-interaction-output-api-v1.md",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADR/core.org",
                "docs/ADR/0001-crate-and-module-boundaries.org",
                "docs/ADR/0002-producer-emission-dispatch.org",
                "docs/ADR/0003-subscription-driver-ownership.org",
                "docs/ADR/0004-collection-subscriptions.org",
                "docs/ADR/0005-outbound-request.org",
            ]),
            owned_set(&[
                "api-ownership-check",
                "architecture-adr-check",
                "wp100-amendment-check",
                "wp100-handler-amendment-check",
            ]),
        ),
        "GATE-2" => (
            owned_set(&[
                "LIFE-EXPOSE-003",
                "BIND-PROGRESS-001",
                "STATE-EXPOSE-001",
                "STATE-SUB-001",
                "STATE-BIND-001",
                "STATE-INFLIGHT-001",
                "HANDLE-DROP-001",
                "PRODUCER-EMIT-001",
                "PLAN-INDEX-001",
                "FORM-OWNER-001",
                "BIND-IO-001",
                "BIND-OUT-001",
                "HANDLER-API-001",
                "HANDLER-SUB-001",
                "HANDLER-CANCEL-001",
                "HANDLER-CANCEL-002",
                "HANDLER-STORAGE-001",
                "API-SECURITY-001",
                "CLEANUP-RECORD-001",
            ]),
            owned_set(&[
                "docs/design.md",
                "docs/state-machines.toml",
                "docs/amendments/WP-100-error-cleanup-v1.md",
                "docs/amendments/WP-100-error-disposition-v1.md",
                "docs/amendments/WP-100-interaction-output-api-v1.md",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADR/core.org",
                "docs/ADR/0001-crate-and-module-boundaries.org",
                "docs/ADR/0002-producer-emission-dispatch.org",
                "docs/ADR/0003-subscription-driver-ownership.org",
                "docs/ADR/0004-collection-subscriptions.org",
                "docs/ADR/0005-outbound-request.org",
            ]),
            owned_set(&[
                "architecture-adr-check",
                "state-machine-check",
                "wp100-amendment-check",
                "wp100-handler-amendment-check",
            ]),
        ),
        "GATE-3" => (
            owned_set(&[
                "DIR-SCOPE-001",
                "DIR-CONTRACT-001",
                "DIR-AUTH-001",
                "DIR-SNAPSHOT-001",
                "DIR-WATCH-001",
            ]),
            owned_set(&["docs/design.md", "docs/future/directory-service.md"]),
            owned_set(&["directory-client-scope-check"]),
        ),
        "GATE-4" => (
            owned_set(&[
                "RES-LIMIT-001",
                "RES-PROFILE-001",
                "API-RESOURCE-001",
                "HANDLER-STORAGE-001",
                "HANDLER-CANCEL-002",
                "CLEANUP-RECORD-001",
            ]),
            owned_set(&[
                "docs/resource-limits.csv",
                "docs/performance/constrained.toml",
                "docs/performance/gateway.toml",
                "docs/performance/directory.toml",
                "docs/amendments/WP-100-error-cleanup-v1.md",
                "docs/amendments/WP-100-error-disposition-v1.md",
                "docs/amendments/WP-100-interaction-output-api-v1.md",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADR/core.org",
                "docs/ADR/0001-crate-and-module-boundaries.org",
                "docs/ADR/0002-producer-emission-dispatch.org",
                "docs/ADR/0003-subscription-driver-ownership.org",
                "docs/ADR/0004-collection-subscriptions.org",
                "docs/ADR/0005-outbound-request.org",
            ]),
            owned_set(&[
                "architecture-adr-check",
                "resource-profile-check",
                "wp100-amendment-check",
                "wp100-handler-amendment-check",
            ]),
        ),
        "GATE-5" => (
            owned_set(&[
                "PERF-BENCH-001",
                "PERF-BENCH-002",
                "PERF-BUDGET-001",
                "PERF-SCALE-001",
                "PERF-ALLOC-001",
                "PERF-FANOUT-001",
                "PERF-FANOUT-002",
                "BIND-PROGRESS-001",
                "PRODUCER-EMIT-001",
                "HANDLER-STORAGE-001",
                "HANDLER-CANCEL-001",
                "HANDLER-CANCEL-002",
            ]),
            owned_set(&[
                "docs/state-machines.toml",
                "docs/performance/manifest.schema.json",
                "docs/performance/result.schema.json",
                "docs/performance/fixtures.lock.toml",
                "docs/performance/fixture-generator.md",
                "docs/performance/constrained.toml",
                "docs/performance/gateway.toml",
                "docs/performance/directory.toml",
                "tools/performance-harness",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADR/core.org",
                "docs/ADR/0001-crate-and-module-boundaries.org",
                "docs/ADR/0002-producer-emission-dispatch.org",
                "docs/ADR/0003-subscription-driver-ownership.org",
                "docs/ADR/0004-collection-subscriptions.org",
                "docs/ADR/0005-outbound-request.org",
            ]),
            owned_set(&[
                "architecture-adr-check",
                "state-machine-check",
                "performance-contract-check",
                "wp100-handler-amendment-check",
            ]),
        ),
        "GATE-6" => (
            owned_set(&[
                "IMPL-CONFORM-001",
                "HANDLER-API-001",
                "HANDLER-SUB-001",
                "HANDLER-CANCEL-001",
                "HANDLER-CANCEL-002",
                "HANDLER-STORAGE-001",
                "PRODUCER-EMIT-001",
            ]),
            owned_set(&[
                "docs/design.md",
                "docs/work-packages/index.toml",
                "docs/work-packages",
                "docs/amendments/WP-100-interaction-output-api-v1.md",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADR/core.org",
                "docs/ADR/0001-crate-and-module-boundaries.org",
                "docs/ADR/0002-producer-emission-dispatch.org",
                "docs/ADR/0003-subscription-driver-ownership.org",
                "docs/ADR/0004-collection-subscriptions.org",
                "docs/ADR/0005-outbound-request.org",
            ]),
            owned_set(&[
                "architecture-adr-check",
                "work-package-dag-check",
                "wp100-amendment-check",
                "wp100-handler-amendment-check",
            ]),
        ),
        _ => return None,
    };
    Some(contract)
}

fn load_artifact_registry(root: &Path) -> Result<BTreeSet<String>, String> {
    let relative_path = "docs/artifacts.csv";
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = source.lines();
    let expected_header = "path,role,normativity,design_revision,schema_version,requirement_source";
    if lines.next() != Some(expected_header) {
        return Err(format!("{relative_path} has an invalid header"));
    }
    let mut artifacts = BTreeSet::new();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!(
                "{relative_path} has a blank data row on line {}",
                offset + 2
            ));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 6 || fields.iter().any(|value| value.is_empty()) {
            return Err(format!(
                "{relative_path} line {} must have six non-empty columns",
                offset + 2
            ));
        }
        let artifact = fields[0];
        validate_relative_path(artifact, "artifact registry")?;
        if !artifacts.insert(artifact.to_owned()) {
            return Err(format!("duplicate registered artifact {artifact:?}"));
        }
        if !root.join(artifact).exists() {
            return Err(format!("registered artifact {artifact:?} does not exist"));
        }
        let revision = fields[3];
        let valid_revision = if artifact == "docs/evidence/WP-000.toml" {
            revision == WP000_EVIDENCE_REVISION
        } else {
            revision == ACTIVE_DESIGN_REVISION
        };
        if !valid_revision {
            return Err(format!(
                "registered artifact {artifact:?} has invalid design revision {revision:?}"
            ));
        }
        let schema_version = fields[4].parse::<u32>().map_err(|error| {
            format!("registered artifact {artifact:?} has invalid schema version: {error}")
        })?;
        if schema_version == 0 {
            return Err(format!(
                "registered artifact {artifact:?} has zero schema version"
            ));
        }
        if artifact == "docs/performance/result.schema.json" && schema_version != 2 {
            return Err(
                "performance result schema registry row must declare schema version 2".to_owned(),
            );
        }
        if fields[5] != "docs/design.md" {
            return Err(format!(
                "registered artifact {artifact:?} has unknown requirement source {:?}",
                fields[5]
            ));
        }
    }
    if artifacts.is_empty() {
        return Err(format!("{relative_path} has no data rows"));
    }
    Ok(artifacts)
}

fn check_governance_checks(
    document: &DocumentMut,
    registered_artifacts: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>, String> {
    let checks = document
        .get("check")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "governance artifact has no [[check]] records".to_owned())?;
    let expected_ids = owned_set(&[
        "api-ownership-check",
        "architecture-adr-check",
        "wp100-amendment-check",
        "state-machine-check",
        "directory-client-scope-check",
        "resource-profile-check",
        "performance-contract-check",
        "work-package-dag-check",
        "wp100-handler-amendment-check",
        "wp100-foundation-refresh-check",
    ]);
    let mut statuses = BTreeMap::new();
    for check in checks {
        let id = string_field(check, "id", "governance check")?;
        let (expected_artifact, expected_command, allowed_statuses) =
            expected_check_mapping(&id)
                .ok_or_else(|| format!("unknown governance check {id:?}"))?;
        let status = string_field(check, "status", &id)?;
        if !allowed_statuses.contains(&status.as_str()) {
            return Err(format!(
                "governance check {id:?} has invalid status {status:?}; expected one of \
                 {allowed_statuses:?}"
            ));
        }
        let artifact = string_field(check, "artifact", &id)?;
        if artifact != expected_artifact || !registered_artifacts.contains(&artifact) {
            return Err(format!(
                "governance check {id:?} has invalid or unregistered artifact {artifact:?}"
            ));
        }
        let command = string_vec(array_field(check, "command", &id)?, &id, "command")?;
        let expected: Vec<String> = expected_command
            .iter()
            .map(|value| (*value).to_owned())
            .collect();
        if command != expected {
            return Err(format!(
                "governance check {id:?} command mismatch; expected {expected:?}, found \
                 {command:?}"
            ));
        }
        if statuses.insert(id.clone(), status).is_some() {
            return Err(format!("duplicate governance check {id:?}"));
        }
    }
    let actual_ids: BTreeSet<String> = statuses.keys().cloned().collect();
    if actual_ids != expected_ids {
        return Err(format!(
            "governance check set mismatch; expected {expected_ids:?}, found {actual_ids:?}"
        ));
    }
    Ok(statuses)
}

type CheckMapping = (
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
);

fn expected_check_mapping(id: &str) -> Option<CheckMapping> {
    match id {
        "api-ownership-check" => Some((
            "tools/check-api-ownership.sh",
            &["tools/check-api-ownership.sh"],
            &["executable"],
        )),
        "architecture-adr-check" => Some((
            "tools/check-architecture-adrs.sh",
            &["tools/check-architecture-adrs.sh"],
            &["executable"],
        )),
        "wp100-amendment-check" => Some((
            "tools/check-wp100-amendment.sh",
            &["tools/check-wp100-amendment.sh"],
            &["executable"],
        )),
        "state-machine-check" => Some((
            "tools/design-check/Cargo.toml",
            &[
                "cargo",
                "run",
                "--locked",
                "--quiet",
                "--manifest-path",
                "tools/design-check/Cargo.toml",
                "--",
                "check-state",
            ],
            &["executable"],
        )),
        "directory-client-scope-check" => Some((
            "tools/check-directory-client-scope.sh",
            &["tools/check-directory-client-scope.sh"],
            &["executable"],
        )),
        "resource-profile-check" => Some((
            "tools/check-resource-limits.sh",
            &["tools/check-resource-limits.sh"],
            &["executable"],
        )),
        "performance-contract-check" => Some((
            "tools/performance-harness/Cargo.toml",
            &[
                "cargo",
                "run",
                "--locked",
                "--quiet",
                "--manifest-path",
                "tools/performance-harness/Cargo.toml",
                "--",
                "verify",
            ],
            &["executable"],
        )),
        "work-package-dag-check" => Some((
            "tools/design-check/Cargo.toml",
            &[
                "cargo",
                "run",
                "--locked",
                "--quiet",
                "--manifest-path",
                "tools/design-check/Cargo.toml",
                "--",
                "check-work-packages",
            ],
            &["executable"],
        )),
        "wp100-handler-amendment-check" => Some((
            "tools/check-wp100-handler-amendment.sh",
            &["tools/check-wp100-handler-amendment.sh"],
            &["executable"],
        )),
        "wp100-foundation-refresh-check" => Some((
            "tools/check-wp100-foundation-refresh.sh",
            &["tools/check-wp100-foundation-refresh.sh"],
            &["pending", "executable"],
        )),
        _ => None,
    }
}

fn check_governance_reviews(
    document: &DocumentMut,
    resolved_artifacts: &BTreeSet<String>,
    check_statuses: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, GovernanceReview>, String> {
    let review_tables = document
        .get("review")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "governance artifact has no [[review]] records".to_owned())?;
    let known_gates = owned_set(REQUIRED_REFACTOR_GATES);
    let known_checks: BTreeSet<String> = check_statuses.keys().cloned().collect();
    let mut reviews = BTreeMap::new();
    for table in review_tables {
        let id = string_field(table, "id", "governance review")?;
        let design_revision = string_field(table, "design_revision", &id)?;
        let status = string_field(table, "status", &id)?;
        if !matches!(status.as_str(), "blocking" | "passed") {
            return Err(format!("review {id:?} has invalid status {status:?}"));
        }
        let review_type = string_field(table, "review_type", &id)?;
        if !matches!(review_type.as_str(), "audit" | "carry-forward" | "closure") {
            return Err(format!("review {id:?} has invalid type {review_type:?}"));
        }
        let basis_revision = table
            .get("basis_revision")
            .and_then(Item::as_str)
            .map(str::to_owned);
        if review_type == "carry-forward" && basis_revision.is_none() {
            return Err(format!("carry-forward review {id:?} has no basis_revision"));
        }
        let gates = package_string_set(table, "gates", &id)?;
        check_known_values(&id, "gate", &gates, &known_gates)?;
        let artifacts = package_string_set(table, "artifacts", &id)?;
        check_known_values(&id, "artifact", &artifacts, resolved_artifacts)?;
        let checks = string_set(array_field(table, "checks", &id)?, &id, "checks")?;
        check_known_values_allow_empty(&id, "check", &checks, &known_checks)?;
        if status == "passed" {
            if design_revision != ACTIVE_DESIGN_REVISION {
                return Err(format!(
                    "passed review {id:?} must target revision {ACTIVE_DESIGN_REVISION:?}"
                ));
            }
            if checks.is_empty() {
                return Err(format!("passed review {id:?} has no executable checks"));
            }
            for check in &checks {
                if check_statuses.get(check).map(String::as_str) != Some("executable") {
                    return Err(format!(
                        "passed review {id:?} uses non-executable check {check:?}"
                    ));
                }
            }
        }
        if id == ARCHITECTURE_REVIEW_02_OPEN {
            let expected_gates = owned_set(&["GATE-1", "GATE-2", "GATE-4", "GATE-5", "GATE-6"]);
            let expected_checks = owned_set(&[
                "api-ownership-check",
                "architecture-adr-check",
                "wp100-amendment-check",
                "state-machine-check",
                "resource-profile-check",
                "performance-contract-check",
                "work-package-dag-check",
                "wp100-handler-amendment-check",
            ]);
            if status != "blocking"
                || review_type != "audit"
                || design_revision != ACTIVE_DESIGN_REVISION
                || basis_revision.is_some()
                || gates != expected_gates
                || checks != expected_checks
                || !artifacts.contains("docs/review/review-02.org")
            {
                return Err(format!(
                    "Architecture Review 02 does not match the frozen blocking \
                     v{ACTIVE_DESIGN_REVISION} record"
                ));
            }
        }
        if review_type == "closure" {
            let expected_gates = owned_set(&["GATE-1", "GATE-2", "GATE-4", "GATE-5", "GATE-6"]);
            let expected_checks = owned_set(&[
                "api-ownership-check",
                "architecture-adr-check",
                "wp100-amendment-check",
                "state-machine-check",
                "resource-profile-check",
                "performance-contract-check",
                "work-package-dag-check",
                "wp100-handler-amendment-check",
            ]);
            if id != ARCHITECTURE_CLOSURE_REVIEW
                || status != "passed"
                || basis_revision.is_some()
                || gates != expected_gates
                || checks != expected_checks
                || !artifacts.contains("docs/review/review-02.org")
            {
                return Err(format!(
                    "closure review {id:?} does not match the frozen v{ACTIVE_DESIGN_REVISION} \
                     Architecture Review 02 record"
                ));
            }
        }
        let impact = string_field(table, "impact", &id)?;
        let record = GovernanceReview {
            design_revision,
            status,
            review_type,
            basis_revision,
            gates,
            artifacts,
            checks,
            impact,
        };
        if reviews.insert(id.clone(), record).is_some() {
            return Err(format!("duplicate governance review {id:?}"));
        }
    }
    Ok(reviews)
}

fn load_governance_check_statuses(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let path = root.join("docs/governance.toml");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    let checks = document
        .get("check")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "governance artifact has no [[check]] records".to_owned())?;
    let mut statuses = BTreeMap::new();
    for check in checks {
        let id = string_field(check, "id", "governance check")?;
        let status = string_field(check, "status", &id)?;
        if statuses.insert(id.clone(), status).is_some() {
            return Err(format!("duplicate governance check {id:?}"));
        }
    }
    Ok(statuses)
}

fn check_work_packages(root: &Path, require_handler_entry: bool) -> Result<(), String> {
    let path = root.join("docs/work-packages/index.toml");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    require_integer(
        document.get("schema_version"),
        "work-package schema_version",
        1,
    )?;
    require_string(
        document.get("design_revision"),
        "work-package design_revision",
        ACTIVE_DESIGN_REVISION,
    )?;
    require_string(
        document.get("requirement"),
        "work-package requirement",
        "IMPL-CONFORM-001",
    )?;
    let root_status = document
        .get("status")
        .and_then(Item::as_str)
        .ok_or_else(|| "work-package index has no string status".to_owned())?;

    let entry_gates = root_string_set(&document, "implementation_entry_gates")?;
    let known_gates = load_first_column(root, "docs/refactor-gates.csv")?;
    if entry_gates != known_gates {
        return Err(format!(
            "implementation entry gates mismatch; expected {known_gates:?}, found {entry_gates:?}"
        ));
    }
    let required_ids = root_string_set(&document, "required_package_ids")?;
    let expected_ids = owned_set(REQUIRED_WORK_PACKAGES);
    if required_ids != expected_ids {
        return Err(format!(
            "required work-package ids mismatch; expected {expected_ids:?}, found {required_ids:?}"
        ));
    }
    let headings = root_string_set(&document, "required_headings")?;
    let expected_headings = owned_set(REQUIRED_WORK_PACKAGE_HEADINGS);
    if headings != expected_headings {
        return Err(format!(
            "required work-package headings mismatch; expected {expected_headings:?}, found {headings:?}"
        ));
    }

    let known_requirements = load_requirement_ids(root)?;
    let known_workloads = load_performance_ids(root)?;
    let packages = document
        .get("package")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "work-package index has no [[package]] entries".to_owned())?;
    let allowed_statuses = owned_set(&["planned", "in-progress", "complete"]);
    if !allowed_statuses.contains(root_status) {
        return Err(format!("invalid work-package index status {root_status:?}"));
    }
    let mut package_statuses = BTreeMap::new();
    for package in packages {
        let id = string_field(package, "id", "work package")?;
        let status = string_field(package, "status", &id)?;
        if !allowed_statuses.contains(&status) {
            return Err(format!("{id} has invalid status {status:?}"));
        }
        if package_statuses.insert(id.clone(), status).is_some() {
            return Err(format!("duplicate work package {id:?}"));
        }
    }
    let expected_root_status = if package_statuses.values().all(|status| status == "planned") {
        "planned"
    } else if package_statuses.values().all(|status| status == "complete") {
        "complete"
    } else {
        "in-progress"
    };
    if root_status != expected_root_status {
        return Err(format!(
            "work-package index status must be {expected_root_status:?}; found {root_status:?}"
        ));
    }
    let expected_dependencies = expected_work_package_dependencies();
    let expected_sequences: BTreeMap<&str, i64> = [
        ("WP-000", 0),
        ("WP-100", 100),
        ("WP-200", 200),
        ("WP-300", 300),
        ("WP-400", 400),
        ("WP-500", 500),
        ("WP-600", 600),
        ("WP-700", 700),
    ]
    .into_iter()
    .collect();
    let allowed_owners = owned_set(&[
        "clinkz-wot",
        "clinkz-wot-core",
        "clinkz-wot-discovery",
        "clinkz-wot-foundation",
        "clinkz-wot-protocol-bindings",
        "clinkz-wot-protocol-bindings-zenoh",
        "clinkz-wot-servient",
        "clinkz-wot-td",
        "workspace",
    ]);
    let allowed_cells = owned_set(&["no-default", "async-no-std", "std"]);
    let mut ids = BTreeSet::new();
    let mut covered_requirements = BTreeSet::new();
    let mut covered_workloads = BTreeSet::new();
    let mut evidence_keys = BTreeSet::new();
    let mut package_evidence = BTreeMap::new();
    let mut documents = BTreeSet::new();

    for package in packages {
        let id = string_field(package, "id", "work package")?;
        if !ids.insert(id.clone()) || !expected_ids.contains(&id) {
            return Err(format!("duplicate or unknown work package {id:?}"));
        }
        let expected_sequence = expected_sequences
            .get(id.as_str())
            .ok_or_else(|| format!("no expected sequence for {id:?}"))?;
        let sequence = integer_field(package, "sequence", &id)?;
        if sequence != *expected_sequence {
            return Err(format!(
                "{id} sequence mismatch; expected {expected_sequence}, found {sequence}"
            ));
        }
        string_field(package, "title", &id)?;
        let status = package_statuses
            .get(&id)
            .ok_or_else(|| format!("{id} has no registered status"))?;
        let dependencies = string_set(array_field(package, "depends_on", &id)?, &id, "depends_on")?;
        let expected = expected_dependencies
            .get(id.as_str())
            .ok_or_else(|| format!("no expected dependencies for {id:?}"))?;
        if &dependencies != expected {
            return Err(format!(
                "{id} dependency mismatch; expected {expected:?}, found {dependencies:?}"
            ));
        }
        for dependency in &dependencies {
            let dependency_sequence = expected_sequences
                .get(dependency.as_str())
                .ok_or_else(|| format!("{id} has unknown dependency {dependency:?}"))?;
            if dependency_sequence >= expected_sequence {
                return Err(format!("{id} dependency {dependency:?} is not earlier"));
            }
            if status != "planned"
                && package_statuses.get(dependency).map(String::as_str) != Some("complete")
            {
                return Err(format!(
                    "{id} cannot be {status:?} before dependency {dependency:?} is complete"
                ));
            }
        }

        let requirements = package_string_set(package, "requirements", &id)?;
        check_known_values(&id, "requirement", &requirements, &known_requirements)?;
        covered_requirements.extend(requirements.iter().cloned());
        let owners = package_string_set(package, "owner_packages", &id)?;
        check_known_values(&id, "owner package", &owners, &allowed_owners)?;
        let cells = package_string_set(package, "feature_cells", &id)?;
        check_known_values(&id, "feature cell", &cells, &allowed_cells)?;
        if id == "WP-600" && cells != allowed_cells {
            return Err(format!(
                "WP-600 feature cells must be exactly {allowed_cells:?}; found {cells:?}"
            ));
        }
        let evidence = package_string_set(package, "evidence_keys", &id)?;
        package_evidence.insert(id.clone(), evidence.clone());
        for key in &evidence {
            if !evidence_keys.insert(key.clone()) {
                return Err(format!("evidence key {key:?} is assigned more than once"));
            }
        }
        let workload_expressions = package_string_set(package, "performance_workloads", &id)?;
        let workloads = expand_expressions(&workload_expressions)?;
        check_known_values(&id, "performance workload", &workloads, &known_workloads)?;
        covered_workloads.extend(workloads.iter().cloned());

        let document_path = string_field(package, "document", &id)?;
        if !documents.insert(document_path.clone()) {
            return Err(format!("work-package document {document_path:?} is reused"));
        }
        check_work_package_document(
            root,
            &document_path,
            &id,
            status,
            &dependencies,
            &requirements,
            &owners,
            &evidence,
            &workload_expressions,
        )?;
        if status == "complete" {
            check_work_package_evidence(root, &id, &requirements, &cells, &evidence)?;
        }
    }
    if ids != expected_ids {
        return Err(format!(
            "work-package set mismatch; expected {expected_ids:?}, found {ids:?}"
        ));
    }
    if covered_requirements != known_requirements {
        let missing: Vec<_> = known_requirements
            .difference(&covered_requirements)
            .cloned()
            .collect();
        return Err(format!(
            "work-package DAG does not cover requirements {missing:?}"
        ));
    }
    if covered_workloads != known_workloads {
        let missing: Vec<_> = known_workloads
            .difference(&covered_workloads)
            .cloned()
            .collect();
        return Err(format!(
            "work-package DAG does not cover performance workloads {missing:?}"
        ));
    }
    check_work_package_tranches(
        root,
        &document,
        &known_requirements,
        &allowed_owners,
        &allowed_cells,
        &package_evidence,
        require_handler_entry,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn check_work_package_tranches(
    root: &Path,
    document: &DocumentMut,
    known_requirements: &BTreeSet<String>,
    allowed_owners: &BTreeSet<String>,
    allowed_cells: &BTreeSet<String>,
    package_evidence: &BTreeMap<String, BTreeSet<String>>,
    require_handler_entry: bool,
) -> Result<(), String> {
    let entrypoints = document
        .get("entrypoint")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "work-package index has no [[entrypoint]] records".to_owned())?;
    if entrypoints.len() != 1 {
        return Err(format!(
            "work-package index must define exactly one handler entrypoint; found {}",
            entrypoints.len()
        ));
    }
    let entrypoint = entrypoints
        .iter()
        .next()
        .ok_or_else(|| "handler entrypoint record is missing".to_owned())?;
    require_table_string(entrypoint, "id", HANDLER_ENTRYPOINT, "handler entrypoint")?;
    require_table_string(entrypoint, "work_package", "WP-100", HANDLER_ENTRYPOINT)?;
    let entry_dependencies =
        package_string_set(entrypoint, "depends_on_tranches", HANDLER_ENTRYPOINT)?;
    if entry_dependencies != owned_set(&[HANDLER_FOUNDATION_TRANCHE]) {
        return Err(format!(
            "{HANDLER_ENTRYPOINT} dependency mismatch; expected {HANDLER_FOUNDATION_TRANCHE:?}, \
             found {entry_dependencies:?}"
        ));
    }

    let tranches = document
        .get("tranche")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "work-package index has no [[tranche]] records".to_owned())?;
    if tranches.len() != 1 {
        return Err(format!(
            "work-package index must define exactly one handler prerequisite tranche; found {}",
            tranches.len()
        ));
    }
    let tranche = tranches
        .iter()
        .next()
        .ok_or_else(|| "handler prerequisite tranche record is missing".to_owned())?;
    require_table_string(tranche, "id", HANDLER_FOUNDATION_TRANCHE, "handler tranche")?;
    require_table_string(
        tranche,
        "work_package",
        "WP-100",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    let sequence = integer_field(tranche, "sequence", HANDLER_FOUNDATION_TRANCHE)?;
    if sequence != 110 {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} sequence mismatch; expected 110, found {sequence}"
        ));
    }
    let status = string_field(tranche, "status", HANDLER_FOUNDATION_TRANCHE)?;
    if !matches!(status.as_str(), "pending" | "complete") {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} has invalid status {status:?}"
        ));
    }
    let dependencies = string_set(
        array_field(tranche, "depends_on", HANDLER_FOUNDATION_TRANCHE)?,
        HANDLER_FOUNDATION_TRANCHE,
        "depends_on",
    )?;
    if !dependencies.is_empty() {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} must not depend on another tranche"
        ));
    }
    let blocked = package_string_set(tranche, "blocks_entrypoints", HANDLER_FOUNDATION_TRANCHE)?;
    if blocked != owned_set(&[HANDLER_ENTRYPOINT]) {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} must block only {HANDLER_ENTRYPOINT}"
        ));
    }
    let expected_requirements = owned_set(&[
        "API-RESOURCE-001",
        "API-SURFACE-001",
        "CONSTRAINED-PROGRESS-001",
        "CONSTRAINED-WORK-001",
        "CLEANUP-RECORD-001",
        "HANDLER-CANCEL-002",
        "HANDLER-STORAGE-001",
        "HANDLER-SUB-001",
        "RES-LIMIT-001",
        "RES-PROFILE-001",
    ]);
    let requirements = package_string_set(tranche, "requirements", HANDLER_FOUNDATION_TRANCHE)?;
    check_known_values(
        HANDLER_FOUNDATION_TRANCHE,
        "requirement",
        &requirements,
        known_requirements,
    )?;
    if requirements != expected_requirements {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} requirement set mismatch; expected \
             {expected_requirements:?}, found {requirements:?}"
        ));
    }
    let owners = package_string_set(tranche, "owner_packages", HANDLER_FOUNDATION_TRANCHE)?;
    check_known_values(
        HANDLER_FOUNDATION_TRANCHE,
        "owner package",
        &owners,
        allowed_owners,
    )?;
    if owners != owned_set(&["clinkz-wot-core", "clinkz-wot-foundation"]) {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} owner package set is not frozen"
        ));
    }
    let cells = package_string_set(tranche, "feature_cells", HANDLER_FOUNDATION_TRANCHE)?;
    check_known_values(
        HANDLER_FOUNDATION_TRANCHE,
        "feature cell",
        &cells,
        allowed_cells,
    )?;
    if cells != owned_set(&["no-default", "async-no-std", "std"]) {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} feature-cell set is not frozen"
        ));
    }
    let expected_resource_limit_count = integer_field(
        tranche,
        "expected_resource_limit_count",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    if expected_resource_limit_count != 139 {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} must freeze 139 resource-limit rows"
        ));
    }
    let expected_additive_fields = owned_set(&[
        "handler_slots_per_thing_max",
        "handler_slots_global_max",
        "handler_state_bytes_per_thing_max",
        "handler_state_bytes_global_max",
        "pending_handler_calls_per_thing_max",
        "pending_handler_calls_global_max",
        "handler_generations_per_slot_max",
        "handler_drain_timeout_millis_max",
        "handler_drain_steps_max",
        "producer_residual_records_global_max",
        "producer_residual_record_bytes_max",
        "producer_residual_bytes_global_max",
        "binding_emission_slots_per_binding_max",
        "binding_emission_slots_global_max",
        "collection_subscription_sources_per_subscription_max",
        "host_emission_lanes_per_binding_max",
        "host_emission_lanes_global_max",
        "pending_client_calls_per_binding_max",
        "pending_client_calls_per_thing_max",
        "pending_client_calls_global_max",
        "host_binding_cancel_drain_timeout_millis_max",
    ]);
    let additive_fields = package_string_set(
        tranche,
        "expected_additive_resource_fields",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    if additive_fields != expected_additive_fields {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} additive resource field set mismatch; expected \
             {expected_additive_fields:?}, found {additive_fields:?}"
        ));
    }
    let resource_fields = load_first_column(root, "docs/resource-limits.csv")?;
    if resource_fields.len()
        != usize::try_from(expected_resource_limit_count)
            .map_err(|error| format!("invalid resource-limit count: {error}"))?
    {
        return Err(format!(
            "resource limit row count mismatch; expected {expected_resource_limit_count}, found {}",
            resource_fields.len()
        ));
    }
    if !additive_fields.is_subset(&resource_fields) {
        let missing: Vec<_> = additive_fields.difference(&resource_fields).collect();
        return Err(format!(
            "resource limit schema is missing additive fields {missing:?}"
        ));
    }
    let evidence_key = string_field(tranche, "evidence_key", HANDLER_FOUNDATION_TRANCHE)?;
    if evidence_key != "handler-foundation-refresh"
        || !package_evidence
            .get("WP-100")
            .is_some_and(|keys| keys.contains(&evidence_key))
    {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} has an unregistered evidence key {evidence_key:?}"
        ));
    }
    let evidence_path = string_field(tranche, "evidence_path", HANDLER_FOUNDATION_TRANCHE)?;
    if evidence_path != "docs/evidence/WP-100-foundation-refresh.toml" {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} evidence path is not frozen"
        ));
    }
    validate_relative_path(&evidence_path, "tranche evidence")?;
    let verification_check =
        string_field(tranche, "verification_check", HANDLER_FOUNDATION_TRANCHE)?;
    let check_statuses = load_governance_check_statuses(root)?;
    let check_status = check_statuses.get(&verification_check).ok_or_else(|| {
        format!("{HANDLER_FOUNDATION_TRANCHE} references unknown check {verification_check:?}")
    })?;

    let evidence_exists = root.join(&evidence_path).is_file();
    if status == "complete" {
        if !evidence_exists {
            return Err(format!(
                "{HANDLER_FOUNDATION_TRANCHE} is complete but {evidence_path} is missing"
            ));
        }
        check_tranche_evidence(root, &evidence_path, &evidence_key, &verification_check)?;
    } else if evidence_exists && tranche_evidence_is_passed(root, &evidence_path)? {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} remains pending while {evidence_path} claims passed"
        ));
    }

    if require_handler_entry {
        if status != "complete" {
            return Err(format!(
                "handler entry blocked: {HANDLER_FOUNDATION_TRANCHE} is {status:?}"
            ));
        }
        if check_status != "executable" {
            return Err(format!(
                "handler entry blocked: check {verification_check:?} is {check_status:?}"
            ));
        }
        let checker = root.join("tools/check-wp100-foundation-refresh.sh");
        let status = Command::new(&checker)
            .current_dir(root)
            .status()
            .map_err(|error| {
                format!(
                    "cannot execute handler foundation verification {}: {error}",
                    checker.display()
                )
            })?;
        if !status.success() {
            return Err(format!(
                "handler entry blocked: foundation verification exited with {status}"
            ));
        }
    }
    Ok(())
}

fn check_tranche_evidence(
    root: &Path,
    relative_path: &str,
    evidence_key: &str,
    verification_check: &str,
) -> Result<(), String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    require_integer(
        document.get("schema_version"),
        "tranche evidence schema_version",
        1,
    )?;
    require_string(
        document.get("design_revision"),
        "tranche evidence design_revision",
        ACTIVE_DESIGN_REVISION,
    )?;
    require_string(
        document.get("work_package"),
        "tranche evidence work_package",
        "WP-100",
    )?;
    require_string(
        document.get("tranche"),
        "tranche evidence tranche",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    require_string(document.get("status"), "tranche evidence status", "passed")?;
    require_string(
        document.get("evidence_key"),
        "tranche evidence evidence_key",
        evidence_key,
    )?;
    require_string(
        document.get("verification_check"),
        "tranche evidence verification_check",
        verification_check,
    )?;
    for field in ["implementation_ref", "recorded_on", "verification_command"] {
        let value = document
            .get(field)
            .and_then(Item::as_str)
            .ok_or_else(|| format!("{relative_path} has no string field {field:?}"))?;
        if value.trim().is_empty() {
            return Err(format!("{relative_path} has empty field {field:?}"));
        }
    }
    Ok(())
}

fn tranche_evidence_is_passed(root: &Path, relative_path: &str) -> Result<bool, String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    Ok(document.get("status").and_then(Item::as_str) == Some("passed"))
}

fn expected_work_package_dependencies() -> BTreeMap<&'static str, BTreeSet<String>> {
    [
        ("WP-000", owned_set(&[])),
        ("WP-100", owned_set(&["WP-000"])),
        ("WP-200", owned_set(&["WP-100"])),
        ("WP-300", owned_set(&["WP-200"])),
        ("WP-400", owned_set(&["WP-300"])),
        ("WP-500", owned_set(&["WP-300"])),
        ("WP-600", owned_set(&["WP-300"])),
        ("WP-700", owned_set(&["WP-400", "WP-500", "WP-600"])),
    ]
    .into_iter()
    .collect()
}

fn load_first_column(root: &Path, relative_path: &str) -> Result<BTreeSet<String>, String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let values: BTreeSet<String> = source
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split(',').next())
        .map(str::to_owned)
        .collect();
    if values.is_empty() {
        return Err(format!("{} has no data rows", path.display()));
    }
    Ok(values)
}

fn load_requirement_ids(root: &Path) -> Result<BTreeSet<String>, String> {
    let expressions = load_first_column(root, "docs/requirements.csv")?;
    let components: BTreeSet<String> = expressions
        .iter()
        .flat_map(|expression| expression.split('|'))
        .map(str::to_owned)
        .collect();
    expand_expressions(&components)
}

fn load_performance_ids(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for relative_path in [
        "docs/performance/gateway.toml",
        "docs/performance/directory.toml",
        "docs/performance/constrained.toml",
    ] {
        let path = root.join(relative_path);
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let document = source
            .parse::<DocumentMut>()
            .map_err(|error| format!("invalid {}: {error}", path.display()))?;
        for kind in ["workload", "contention"] {
            let Some(tables) = document.get(kind).and_then(Item::as_array_of_tables) else {
                continue;
            };
            for table in tables {
                let id = string_field(table, "id", relative_path)?;
                if !ids.insert(id.clone()) {
                    return Err(format!("duplicate performance id {id:?}"));
                }
            }
        }
    }
    Ok(ids)
}

fn expand_expressions(expressions: &BTreeSet<String>) -> Result<BTreeSet<String>, String> {
    let mut values = BTreeSet::new();
    for expression in expressions {
        let Some((first, last)) = expression.split_once("..") else {
            if !values.insert(expression.clone()) {
                return Err(format!("duplicate identity expression {expression:?}"));
            }
            continue;
        };
        if first.len() < 4 || last.len() != 3 {
            return Err(format!("invalid identity range {expression:?}"));
        }
        let (prefix, first_number) = first.split_at(first.len() - 3);
        let first_number = first_number
            .parse::<u16>()
            .map_err(|error| format!("invalid identity range {expression:?}: {error}"))?;
        let last_number = last
            .parse::<u16>()
            .map_err(|error| format!("invalid identity range {expression:?}: {error}"))?;
        if first_number > last_number {
            return Err(format!("descending identity range {expression:?}"));
        }
        for number in first_number..=last_number {
            let value = format!("{prefix}{number:03}");
            if !values.insert(value.clone()) {
                return Err(format!("identity {value:?} is covered more than once"));
            }
        }
    }
    Ok(values)
}

fn check_known_values(
    context: &str,
    kind: &str,
    values: &BTreeSet<String>,
    known: &BTreeSet<String>,
) -> Result<(), String> {
    if values.is_empty() {
        return Err(format!("{context} has no {kind} entries"));
    }
    let unknown: Vec<&String> = values.difference(known).collect();
    if !unknown.is_empty() {
        return Err(format!("{context} has unknown {kind} entries {unknown:?}"));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn check_work_package_document(
    root: &Path,
    relative_path: &str,
    id: &str,
    status: &str,
    dependencies: &BTreeSet<String>,
    requirements: &BTreeSet<String>,
    owners: &BTreeSet<String>,
    evidence: &BTreeSet<String>,
    workloads: &BTreeSet<String>,
) -> Result<(), String> {
    if !relative_path.starts_with("docs/work-packages/")
        || !relative_path.ends_with(".md")
        || relative_path.contains("..")
    {
        return Err(format!("{id} has unsafe document path {relative_path:?}"));
    }
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if !source.starts_with(&format!("# {id} ")) {
        return Err(format!("{relative_path} title does not start with {id}"));
    }
    let status_metadata = match status {
        "planned" => "Status: Planned",
        "in-progress" => "Status: In Progress",
        "complete" => "Status: Complete",
        _ => return Err(format!("{id} has unsupported document status {status:?}")),
    };
    for metadata in [
        status_metadata,
        "Design revision: v4.8",
        "Depends on:",
        "Required gates:",
        "Owner packages:",
    ] {
        if !source.contains(metadata) {
            return Err(format!("{relative_path} is missing metadata {metadata:?}"));
        }
    }
    let mut previous_heading = 0;
    for heading in REQUIRED_WORK_PACKAGE_HEADINGS {
        let marker = format!("## {heading}");
        let position = source
            .find(&marker)
            .ok_or_else(|| format!("{relative_path} is missing heading {heading:?}"))?;
        if position < previous_heading {
            return Err(format!("{relative_path} headings are out of order"));
        }
        previous_heading = position;
    }
    let lower = source.to_ascii_lowercase();
    for placeholder in ["tbd", "todo", "to be decided"] {
        if lower.contains(placeholder) {
            return Err(format!(
                "{relative_path} contains unresolved placeholder {placeholder:?}"
            ));
        }
    }
    for value in dependencies
        .iter()
        .chain(requirements)
        .chain(owners)
        .chain(evidence)
        .chain(workloads)
    {
        if !source.contains(value) {
            return Err(format!("{relative_path} does not identify {value:?}"));
        }
    }
    Ok(())
}

fn check_work_package_evidence(
    root: &Path,
    id: &str,
    requirements: &BTreeSet<String>,
    cells: &BTreeSet<String>,
    expected_keys: &BTreeSet<String>,
) -> Result<(), String> {
    let relative_path = format!("docs/evidence/{id}.toml");
    let path = root.join(&relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {relative_path}: {error}"))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {relative_path}: {error}"))?;
    require_integer(document.get("schema_version"), "evidence schema_version", 1)?;
    let expected_revision = if id == "WP-000" {
        WP000_EVIDENCE_REVISION
    } else {
        ACTIVE_DESIGN_REVISION
    };
    require_string(
        document.get("design_revision"),
        "evidence design_revision",
        expected_revision,
    )?;
    require_string(document.get("work_package"), "evidence work_package", id)?;
    require_string(document.get("status"), "evidence status", "passed")?;
    for field in ["implementation_ref", "recorded_on", "verification_command"] {
        let value = document
            .get(field)
            .and_then(Item::as_str)
            .ok_or_else(|| format!("{relative_path} has no string field {field:?}"))?;
        if value.trim().is_empty() {
            return Err(format!("{relative_path} has empty field {field:?}"));
        }
    }

    let records = document
        .get("evidence")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{relative_path} has no [[evidence]] records"))?;
    let allowed_profiles = owned_set(&[
        "application-static",
        "gateway-default-v1",
        "directory-client-default-v1",
    ]);
    let mut keys = BTreeSet::new();
    for record in records {
        let key = string_field(record, "key", id)?;
        if !keys.insert(key.clone()) {
            return Err(format!("{relative_path} duplicates evidence key {key:?}"));
        }
        let record_requirements = package_string_set(record, "requirement_ids", &key)?;
        check_known_values(&key, "requirement", &record_requirements, requirements)?;
        let record_cells = package_string_set(record, "compilation_cells", &key)?;
        check_known_values(&key, "compilation cell", &record_cells, cells)?;
        let profiles = package_string_set(record, "resource_profiles", &key)?;
        check_known_values(&key, "resource profile", &profiles, &allowed_profiles)?;
        package_string_set(record, "coverage", &key)?;
    }
    if &keys != expected_keys {
        return Err(format!(
            "{relative_path} evidence keys mismatch; expected {expected_keys:?}, found {keys:?}"
        ));
    }
    Ok(())
}

fn root_string_set(document: &DocumentMut, field: &str) -> Result<BTreeSet<String>, String> {
    let array = document
        .get(field)
        .and_then(Item::as_array)
        .ok_or_else(|| format!("work-package index has no {field:?} array"))?;
    string_set(array, "work-package index", field)
}

fn package_string_set(
    table: &Table,
    field: &str,
    context: &str,
) -> Result<BTreeSet<String>, String> {
    let values = string_set(array_field(table, field, context)?, context, field)?;
    if values.is_empty() {
        return Err(format!("{context} has no {field:?} entries"));
    }
    Ok(values)
}

fn integer_field(table: &Table, field: &str, context: &str) -> Result<i64, String> {
    table
        .get(field)
        .and_then(Item::as_integer)
        .ok_or_else(|| format!("{context:?} has no integer field {field:?}"))
}

fn owned_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn check_machine(
    machine: &Table,
    design: &str,
    ids: &mut BTreeSet<String>,
) -> Result<(String, Vec<Transition>), String> {
    let id = string_field(machine, "id", "machine")?;
    if !ids.insert(id.clone()) {
        return Err(format!("duplicate state machine {id:?}"));
    }

    let states = string_set(array_field(machine, "states", &id)?, &id, "states")?;
    let terminals = string_set(array_field(machine, "terminal", &id)?, &id, "terminal")?;
    if terminals.is_empty() || !terminals.is_subset(&states) {
        return Err(format!("machine {id:?} has invalid terminal states"));
    }
    let initial = string_field(machine, "initial", &id)?;
    if !states.contains(&initial) {
        return Err(format!(
            "machine {id:?} initial state {initial:?} is unknown"
        ));
    }

    for requirement in string_set(
        array_field(machine, "requirements", &id)?,
        &id,
        "requirements",
    )? {
        if !design.contains(&format!("`{requirement}`:")) {
            return Err(format!(
                "machine {id:?} references unknown requirement {requirement:?}"
            ));
        }
    }

    let reusable_terminal_events = match machine.get("reusable_terminal_events") {
        Some(item) => string_set(
            item.as_array().ok_or_else(|| {
                format!("machine {id:?} reusable_terminal_events is not an array")
            })?,
            &id,
            "reusable_terminal_events",
        )?,
        None => BTreeSet::new(),
    };
    let transition_tables = machine
        .get("transition")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("machine {id:?} has no transitions"))?;
    let transitions = parse_transitions(
        &id,
        transition_tables,
        &states,
        &terminals,
        &reusable_terminal_events,
    )?;
    check_required_events(machine, &id, &transitions)?;
    check_reachability(&id, &initial, &states, &terminals, &transitions)?;
    if id == "producer-subscription" {
        check_producer_subscription_contract(machine)?;
    }
    Ok((id, transitions))
}

fn check_producer_subscription_contract(machine: &Table) -> Result<(), String> {
    for (field, expected) in [
        (
            "obligation_bits",
            ["SetupCancellation", "GuardClose", "ApplicationTeardown"].as_slice(),
        ),
        (
            "cleanup_order",
            ["SetupCancellation", "GuardClose", "ApplicationTeardown"].as_slice(),
        ),
    ] {
        let actual = string_vec(
            array_field(machine, field, "producer-subscription")?,
            "producer-subscription",
            field,
        )?;
        if actual != expected {
            return Err(format!(
                "producer-subscription {field:?} mismatch; expected {expected:?}, found {actual:?}"
            ));
        }
    }

    let expected_terminal_preconditions = owned_set(&[
        "start-response-terminal",
        "all-teardown-response-views-terminal",
        "SetupCancellation-absent-or-complete-or-residual",
        "ApplicationTeardown-absent-or-complete-or-residual",
        "GuardClose-absent-or-complete-or-residual",
        "cleanup-owner-absent-or-acknowledged",
        "local-view-count-zero",
    ]);
    let actual_terminal_preconditions = string_set(
        array_field(
            machine,
            "terminal_release_preconditions",
            "producer-subscription",
        )?,
        "producer-subscription",
        "terminal_release_preconditions",
    )?;
    if actual_terminal_preconditions != expected_terminal_preconditions {
        return Err(format!(
            "producer-subscription terminal release preconditions mismatch; expected {expected_terminal_preconditions:?}, found {actual_terminal_preconditions:?}"
        ));
    }

    for field in [
        "setup_cancellation_obligation_semantics",
        "obligation_creation",
        "cleanup_claim_semantics",
        "residual_record_semantics",
        "join_view_semantics",
    ] {
        string_field(machine, field, "producer-subscription")?;
    }
    Ok(())
}

fn check_compositions(
    document: &DocumentMut,
    design: &str,
    machine_transitions: &BTreeMap<String, Vec<Transition>>,
) -> Result<(), String> {
    let compositions = document
        .get("composition")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "state artifact has no [[composition]] entries".to_owned())?;
    let mut ids = BTreeSet::new();

    for composition in compositions {
        let id = string_field(composition, "id", "composition")?;
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate state composition {id:?}"));
        }
        let requirements = string_set(
            array_field(composition, "requirements", &id)?,
            &id,
            "requirements",
        )?;
        if requirements.is_empty() {
            return Err(format!("composition {id:?} has no requirements"));
        }
        for requirement in requirements {
            if !design.contains(&format!("`{requirement}`:")) {
                return Err(format!(
                    "composition {id:?} references unknown requirement {requirement:?}"
                ));
            }
        }
        let participants = string_set(
            array_field(composition, "participants", &id)?,
            &id,
            "participants",
        )?;
        if participants.len() < 2 {
            return Err(format!(
                "composition {id:?} must name at least two participants"
            ));
        }
        string_field(composition, "trigger", &id)?;

        for field in required_composition_fields(&id)? {
            require_nonempty_composition_field(composition, &id, field)?;
        }
        check_composition_policy(composition, &id)?;

        let mut transition_references = 0_usize;
        for (field, item) in composition.iter() {
            if !field.ends_with("_transitions") {
                continue;
            }
            let references = item
                .as_array()
                .ok_or_else(|| format!("composition {id:?} field {field:?} is not an array"))?;
            if references.is_empty() {
                return Err(format!(
                    "composition {id:?} field {field:?} has no transition references"
                ));
            }
            for reference in references {
                let reference = reference.as_str().ok_or_else(|| {
                    format!("composition {id:?} field {field:?} contains a non-string value")
                })?;
                check_composition_transition(reference, machine_transitions)
                    .map_err(|error| format!("composition {id:?} {error}"))?;
                transition_references += 1;
            }
        }
        if transition_references == 0 {
            return Err(format!(
                "composition {id:?} has no machine transition references"
            ));
        }
    }

    let expected = owned_set(REQUIRED_COMPOSITIONS);
    if ids != expected {
        return Err(format!(
            "state composition set mismatch; expected {expected:?}, found {ids:?}"
        ));
    }
    Ok(())
}

fn required_composition_fields(id: &str) -> Result<&'static [&'static str], String> {
    match id {
        "handler-direct-response" => Ok(&[
            "response_claim_policy",
            "preconditions",
            "atomic_transitions",
            "failure_transitions",
            "ownership",
        ]),
        "producer-start-result-transfer" => Ok(&[
            "error_response_policy",
            "preconditions",
            "success_transitions",
            "deliverable_error_transitions",
            "unavailable_error_transitions",
            "error_response_semantics",
            "success_in_flight_effect",
            "ownership",
        ]),
        "producer-late-start-result-transfer" => Ok(&[
            "late_acceptance_policy",
            "preconditions",
            "success_transitions",
            "error_transitions",
            "ownership",
        ]),
        "producer-setup-abort" => Ok(&[
            "incomplete_cleanup_policy",
            "setup_obligation_semantics",
            "cancel_create_transitions",
            "cancel_claim_transitions",
            "cancel_pending_transitions",
            "clean_async_transitions",
            "clean_step_transitions",
            "late_step_start_error_transitions",
            "failed_step_transitions",
            "transferred_step_transitions",
            "residual_step_transitions",
            "ownership",
        ]),
        "producer-start-publication" => Ok(&[
            "success_response_claim",
            "preconditions",
            "atomic_transitions",
            "failure_transitions",
            "ownership",
        ]),
        "producer-prepublication-failure-response" => Ok(&[
            "error_response_policy",
            "deliverable_validation_transitions",
            "unavailable_validation_transitions",
            "deliverable_install_transitions",
            "unavailable_install_transitions",
            "atomicity",
            "ownership",
        ]),
        "handler-cancellation-response" => Ok(&[
            "first_cause_policy",
            "response_transitions",
            "handler_effect",
            "producer_effect",
            "producer_teardown_exception",
            "race_rule",
        ]),
        "producer-teardown-handoff" => Ok(&[
            "active_pending_call_reservation",
            "application_teardown_admission",
            "preconditions",
            "atomic_handoff",
            "call_admission",
            "result_transitions",
            "cleanup_rule",
        ]),
        "producer-teardown-result-and-response" => Ok(&[
            "wire_response_owner",
            "wire_success_transitions",
            "wire_error_transitions",
            "local_transitions",
            "late_transitions",
            "async_no_result_transitions",
            "step_cancel_complete_transitions",
            "step_cancel_error_transitions",
            "step_cancel_transfer_transitions",
            "step_cancel_residual_transitions",
            "owner_abort_response_transitions",
            "no_result_response_precondition",
            "no_result_semantics",
            "response_unavailable",
            "ownership",
        ]),
        "producer-terminal-replay-and-release" => Ok(&[
            "replay_payload",
            "release_condition",
            "join_transitions",
            "replay_transitions",
            "local_replay_transitions",
            "in_flight_release_transitions",
            "start_response_ack_transitions",
            "teardown_view_ack_transitions",
            "local_view_release_transitions",
            "start_response_failure_transitions",
            "release_transitions",
            "local_view_accounting",
            "ack_atomicity",
            "tombstone_eviction_policy",
            "ownership",
        ]),
        _ => Err(format!("unknown state composition {id:?}")),
    }
}

fn require_nonempty_composition_field(
    composition: &Table,
    id: &str,
    field: &str,
) -> Result<(), String> {
    let item = composition
        .get(field)
        .ok_or_else(|| format!("composition {id:?} has no field {field:?}"))?;
    if let Some(value) = item.as_str() {
        if !value.is_empty() {
            return Ok(());
        }
    } else if let Some(values) = item.as_array()
        && !values.is_empty()
    {
        return Ok(());
    }
    Err(format!(
        "composition {id:?} field {field:?} is empty or has the wrong type"
    ))
}

fn check_composition_policy(composition: &Table, id: &str) -> Result<(), String> {
    let (field, expected) = match id {
        "handler-direct-response" => ("response_claim_policy", "atomic-with-handler-release"),
        "producer-start-result-transfer" => (
            "error_response_policy",
            "claim-before-failed-or-same-generation-terminal",
        ),
        "producer-late-start-result-transfer" => {
            ("late_acceptance_policy", "create-teardown-before-discard")
        }
        "producer-setup-abort" => (
            "incomplete_cleanup_policy",
            "complete-or-transfer-or-residual-before-terminal",
        ),
        "producer-start-publication" => (
            "success_response_claim",
            "atomic-with-installed-and-published",
        ),
        "producer-prepublication-failure-response" => (
            "error_response_policy",
            "claim-before-rollback-or-same-generation-terminal",
        ),
        "handler-cancellation-response" => ("first_cause_policy", "immutable"),
        "producer-teardown-handoff" => ("active_pending_call_reservation", "none"),
        "producer-teardown-result-and-response" => ("wire_response_owner", "initiating-view-only"),
        "producer-terminal-replay-and-release" => {
            ("replay_payload", "status-or-error-summary-only")
        }
        _ => return Err(format!("unknown state composition {id:?}")),
    };
    require_string(
        composition.get(field),
        &format!("composition {id} {field}"),
        expected,
    )?;

    if id == "producer-teardown-handoff" {
        require_string(
            composition.get("application_teardown_admission"),
            "producer teardown application admission",
            "acquire-count-and-bytes-at-claim",
        )?;
    } else if id == "producer-terminal-replay-and-release" {
        require_string(
            composition.get("release_condition"),
            "producer terminal release condition",
            "all-terminal-preconditions",
        )?;
    }
    Ok(())
}

fn check_composition_transition(
    reference: &str,
    machine_transitions: &BTreeMap<String, Vec<Transition>>,
) -> Result<(), String> {
    let (source, target_expression) = reference
        .split_once("->")
        .ok_or_else(|| format!("has malformed transition reference {reference:?}"))?;
    let mut source_parts = source.splitn(3, ':');
    let machine_expression = source_parts
        .next()
        .ok_or_else(|| format!("has malformed transition reference {reference:?}"))?;
    let from_expression = source_parts
        .next()
        .ok_or_else(|| format!("has malformed transition reference {reference:?}"))?;
    let event_expression = source_parts
        .next()
        .ok_or_else(|| format!("has malformed transition reference {reference:?}"))?;

    let machine_names: Vec<&str> = if machine_expression == "handler" {
        vec![
            "handler-sync-execution",
            "handler-async-execution",
            "handler-step-execution",
        ]
    } else {
        machine_expression.split('|').collect()
    };
    let from_states: Vec<&str> = from_expression.split('|').collect();
    let events: Vec<&str> = event_expression.split('|').collect();
    let target_states: Vec<&str> = target_expression.split('|').collect();

    for machine_name in machine_names {
        let transitions = machine_transitions
            .get(machine_name)
            .ok_or_else(|| format!("references unknown machine {machine_name:?}"))?;
        for from in &from_states {
            for event in &events {
                let expected_targets: Vec<&str> = if target_states == ["same"] {
                    vec![from]
                } else {
                    target_states.clone()
                };
                for to in expected_targets {
                    if !transitions.iter().any(|transition| {
                        transition.from == *from
                            && transition.event == *event
                            && transition.to == to
                    }) {
                        return Err(format!(
                            "references missing machine transition {machine_name}:{from}:{event}->{to}"
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_transitions(
    machine: &str,
    tables: &ArrayOfTables,
    states: &BTreeSet<String>,
    terminals: &BTreeSet<String>,
    reusable_terminal_events: &BTreeSet<String>,
) -> Result<Vec<Transition>, String> {
    for identity in reusable_terminal_events {
        let (state, event) = identity.split_once(':').ok_or_else(|| {
            format!("machine {machine:?} has malformed reusable terminal event {identity:?}")
        })?;
        if !terminals.contains(state) || event.is_empty() {
            return Err(format!(
                "machine {machine:?} reusable terminal event {identity:?} does not name a \
                 terminal state and non-empty event"
            ));
        }
    }

    let mut transitions = Vec::new();
    let mut identities = BTreeSet::new();
    let mut actual_reusable_terminal_events = BTreeSet::new();
    for table in tables {
        let from = string_field(table, "from", machine)?;
        let event = string_field(table, "event", machine)?;
        let to = string_field(table, "to", machine)?;
        if !states.contains(&from) || !states.contains(&to) {
            return Err(format!(
                "machine {machine:?} transition {from:?}:{event:?}->{to:?} uses an unknown state"
            ));
        }
        let identity = format!("{from}:{event}");
        if terminals.contains(&from) && !reusable_terminal_events.contains(&identity) {
            return Err(format!(
                "machine {machine:?} terminal state {from:?} has transition {event:?}"
            ));
        }
        if terminals.contains(&from) {
            actual_reusable_terminal_events.insert(identity);
        }
        if !identities.insert((from.clone(), event.clone())) {
            return Err(format!(
                "machine {machine:?} duplicates transition {from:?}:{event:?}"
            ));
        }
        for field in ["owner", "linearization", "outcome", "retryability"] {
            string_field(table, field, machine)?;
        }
        transitions.push(Transition { from, event, to });
    }
    if &actual_reusable_terminal_events != reusable_terminal_events {
        let missing: Vec<_> = reusable_terminal_events
            .difference(&actual_reusable_terminal_events)
            .collect();
        return Err(format!(
            "machine {machine:?} declares reusable terminal events without exact transitions \
             {missing:?}"
        ));
    }
    Ok(transitions)
}

fn check_required_events(
    machine: &Table,
    id: &str,
    transitions: &[Transition],
) -> Result<(), String> {
    let required = string_set(
        array_field(machine, "required_events", id)?,
        id,
        "required_events",
    )?;
    let actual: BTreeSet<String> = transitions
        .iter()
        .map(|transition| format!("{}:{}", transition.from, transition.event))
        .collect();
    let missing: Vec<&String> = required.difference(&actual).collect();
    if !missing.is_empty() {
        return Err(format!(
            "machine {id:?} is missing required events {missing:?}"
        ));
    }
    match machine.get("transition_contract").and_then(Item::as_str) {
        None | Some("minimum") => {}
        Some("exact") => {
            let unexpected: Vec<&String> = actual.difference(&required).collect();
            if !unexpected.is_empty() {
                return Err(format!(
                    "machine {id:?} has transitions outside its exact contract {unexpected:?}"
                ));
            }
        }
        Some(value) => {
            return Err(format!(
                "machine {id:?} has invalid transition_contract {value:?}"
            ));
        }
    }
    Ok(())
}

fn check_reachability(
    machine: &str,
    initial: &str,
    states: &BTreeSet<String>,
    terminals: &BTreeSet<String>,
    transitions: &[Transition],
) -> Result<(), String> {
    let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut incoming: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for transition in transitions {
        outgoing
            .entry(&transition.from)
            .or_default()
            .push(&transition.to);
        incoming
            .entry(&transition.to)
            .or_default()
            .push(&transition.from);
    }

    let reachable = traverse(initial, &outgoing);
    let unreachable: Vec<&String> = states
        .iter()
        .filter(|state| !reachable.contains(state.as_str()))
        .collect();
    if !unreachable.is_empty() {
        return Err(format!(
            "machine {machine:?} has unreachable states {unreachable:?}"
        ));
    }

    let mut terminal_reachable = BTreeSet::new();
    let mut queue: VecDeque<&str> = terminals.iter().map(String::as_str).collect();
    while let Some(state) = queue.pop_front() {
        if !terminal_reachable.insert(state) {
            continue;
        }
        if let Some(previous) = incoming.get(state) {
            queue.extend(previous.iter().copied());
        }
    }
    let trapped: Vec<&String> = states
        .iter()
        .filter(|state| !terminals.contains(*state))
        .filter(|state| !terminal_reachable.contains(state.as_str()))
        .collect();
    if !trapped.is_empty() {
        return Err(format!(
            "machine {machine:?} has nonterminal states with no terminal path {trapped:?}"
        ));
    }
    Ok(())
}

fn traverse<'a>(start: &'a str, edges: &BTreeMap<&'a str, Vec<&'a str>>) -> BTreeSet<&'a str> {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([start]);
    while let Some(state) = queue.pop_front() {
        if !visited.insert(state) {
            continue;
        }
        if let Some(next) = edges.get(state) {
            queue.extend(next.iter().copied());
        }
    }
    visited
}

fn array_field<'a>(table: &'a Table, field: &str, context: &str) -> Result<&'a Array, String> {
    table
        .get(field)
        .and_then(Item::as_array)
        .ok_or_else(|| format!("{context:?} has no {field:?} array"))
}

fn require_table_string(
    table: &Table,
    field: &str,
    expected: &str,
    context: &str,
) -> Result<(), String> {
    let actual = table.get(field).and_then(Item::as_str);
    if actual != Some(expected) {
        return Err(format!(
            "{context:?} field {field:?} mismatch; expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn validate_relative_path(path: &str, context: &str) -> Result<(), String> {
    let path_value = Path::new(path);
    if path.is_empty()
        || path_value.is_absolute()
        || path_value
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!("{context} has unsafe repository path {path:?}"));
    }
    Ok(())
}

fn pipe_set(value: &str, context: &str, field: &str) -> Result<BTreeSet<String>, String> {
    if value.is_empty() {
        return Err(format!("{context:?} has an empty {field:?} field"));
    }
    let mut values = BTreeSet::new();
    for entry in value.split('|') {
        if entry.is_empty() || !values.insert(entry.to_owned()) {
            return Err(format!(
                "{context:?} {field:?} contains an empty or duplicate value {entry:?}"
            ));
        }
    }
    Ok(values)
}

fn string_vec(array: &Array, context: &str, field: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    for value in array {
        let value = value
            .as_str()
            .ok_or_else(|| format!("{context:?} {field:?} contains a non-string value"))?;
        if value.is_empty() {
            return Err(format!("{context:?} {field:?} contains an empty string"));
        }
        values.push(value.to_owned());
    }
    if values.is_empty() {
        return Err(format!("{context:?} has no {field:?} entries"));
    }
    Ok(values)
}

fn string_field(table: &Table, field: &str, context: &str) -> Result<String, String> {
    let value = table
        .get(field)
        .and_then(Item::as_str)
        .ok_or_else(|| format!("{context:?} has no string field {field:?}"))?;
    if value.is_empty() {
        return Err(format!("{context:?} has an empty {field:?}"));
    }
    Ok(value.to_owned())
}

fn string_set(array: &Array, context: &str, field: &str) -> Result<BTreeSet<String>, String> {
    let mut values = BTreeSet::new();
    for value in array {
        let value = value
            .as_str()
            .ok_or_else(|| format!("{context:?} {field:?} contains a non-string value"))?;
        if value.is_empty() || !values.insert(value.to_owned()) {
            return Err(format!(
                "{context:?} {field:?} contains an empty or duplicate value {value:?}"
            ));
        }
    }
    Ok(values)
}

fn check_known_values_allow_empty(
    context: &str,
    kind: &str,
    values: &BTreeSet<String>,
    known: &BTreeSet<String>,
) -> Result<(), String> {
    let unknown: Vec<&String> = values.difference(known).collect();
    if !unknown.is_empty() {
        return Err(format!("{context} has unknown {kind} entries {unknown:?}"));
    }
    Ok(())
}

fn require_integer(item: Option<&Item>, field: &str, expected: i64) -> Result<(), String> {
    let actual = item.and_then(Item::as_integer);
    if actual != Some(expected) {
        return Err(format!(
            "{field} mismatch; expected {expected}, found {actual:?}"
        ));
    }
    Ok(())
}

fn require_string(item: Option<&Item>, field: &str, expected: &str) -> Result<(), String> {
    let actual = item.and_then(Item::as_str);
    if actual != Some(expected) {
        return Err(format!(
            "{field} mismatch; expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use toml_edit::DocumentMut;

    use super::{
        check_producer_subscription_contract, expand_expressions,
        expected_work_package_dependencies, parse_transitions,
    };

    fn producer_contract(obligation_bits: &str) -> DocumentMut {
        format!(
            r#"
obligation_bits = [{obligation_bits}]
cleanup_order = ["SetupCancellation", "GuardClose", "ApplicationTeardown"]
terminal_release_preconditions = [
    "start-response-terminal",
    "all-teardown-response-views-terminal",
    "SetupCancellation-absent-or-complete-or-residual",
    "ApplicationTeardown-absent-or-complete-or-residual",
    "GuardClose-absent-or-complete-or-residual",
    "cleanup-owner-absent-or-acknowledged",
    "local-view-count-zero",
]
setup_cancellation_obligation_semantics = "exact"
obligation_creation = "exact"
cleanup_claim_semantics = "exact"
residual_record_semantics = "exact"
join_view_semantics = "exact"
"#
        )
        .parse()
        .expect("test Producer contract must parse")
    }

    #[test]
    fn identity_ranges_expand_inclusively() {
        let expressions = BTreeSet::from(["PERF-GW-001..003".to_owned()]);
        let expanded = expand_expressions(&expressions).expect("range must be valid");
        assert_eq!(
            expanded,
            BTreeSet::from([
                "PERF-GW-001".to_owned(),
                "PERF-GW-002".to_owned(),
                "PERF-GW-003".to_owned(),
            ])
        );
    }

    #[test]
    fn final_package_joins_all_parallel_branches() {
        let dependencies = expected_work_package_dependencies();
        for package in ["WP-400", "WP-500", "WP-600"] {
            assert_eq!(
                dependencies.get(package),
                Some(&BTreeSet::from(["WP-300".to_owned()])),
                "{package} must remain an unordered sibling after WP-300"
            );
        }
        assert_eq!(
            dependencies.get("WP-700"),
            Some(&BTreeSet::from([
                "WP-400".to_owned(),
                "WP-500".to_owned(),
                "WP-600".to_owned(),
            ]))
        );
    }

    #[test]
    fn producer_contract_requires_pre_acceptance_cleanup_bit() {
        let exact =
            producer_contract("\"SetupCancellation\", \"GuardClose\", \"ApplicationTeardown\"");
        assert!(check_producer_subscription_contract(exact.as_table()).is_ok());

        let obsolete = producer_contract("\"GuardClose\", \"ApplicationTeardown\"");
        let error = check_producer_subscription_contract(obsolete.as_table())
            .expect_err("the pre-acceptance setup bit must be mandatory");
        assert!(error.contains("obligation_bits"));
    }

    #[test]
    fn reusable_terminal_transitions_require_an_exact_declaration() {
        let document: DocumentMut = r#"
[[transition]]
from = "Empty"
event = "start"
to = "Active"
owner = "Owner"
linearization = "claim"
outcome = "active"
retryability = "not-applicable"
"#
        .parse()
        .expect("test transitions must parse");
        let transitions = document["transition"]
            .as_array_of_tables()
            .expect("test transitions must be an array of tables");
        let states = BTreeSet::from(["Active".to_owned(), "Empty".to_owned()]);
        let terminals = BTreeSet::from(["Empty".to_owned()]);

        let exact = BTreeSet::from(["Empty:start".to_owned()]);
        assert!(parse_transitions("reusable", transitions, &states, &terminals, &exact).is_ok());

        let undeclared = BTreeSet::new();
        let error = parse_transitions("reusable", transitions, &states, &terminals, &undeclared)
            .expect_err("a terminal transition must be declared");
        assert!(error.contains("terminal state"));

        let unmatched = BTreeSet::from(["Empty:restart".to_owned()]);
        let error = parse_transitions("reusable", transitions, &states, &terminals, &unmatched)
            .expect_err("the declaration and transition must match exactly");
        assert!(error.contains("terminal state"));
    }
}
