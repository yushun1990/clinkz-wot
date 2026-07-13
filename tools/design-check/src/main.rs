//! Validates machine-readable design artifacts that require structured parsing.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table};

const REQUIRED_MACHINES: &[&str] = &[
    "binding-route",
    "directory-process",
    "expose",
    "in-flight",
    "subscription",
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

#[derive(Debug)]
struct Transition {
    from: String,
    event: String,
    to: String,
}

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
            check_work_packages(&root)?;
            println!("design structure check: work-package DAG valid");
        }
        "check-state" => {
            check_state_machines(&root)?;
            println!("design structure check: state machines valid");
        }
        "check-work-packages" => {
            check_work_packages(&root)?;
            println!("design structure check: work-package DAG valid");
        }
        _ => {
            return Err(format!(
                "unknown command {command:?}; expected check, check-state, or check-work-packages"
            ));
        }
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
    require_string(document.get("design_revision"), "design_revision", "4.6")?;
    let machines = document
        .get("machine")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "state artifact has no [[machine]] entries".to_owned())?;
    let design = fs::read_to_string(root.join("docs/design.md"))
        .map_err(|error| format!("cannot read docs/design.md: {error}"))?;

    let mut ids = BTreeSet::new();
    for machine in machines {
        check_machine(machine, &design, &mut ids)?;
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
    Ok(())
}

fn check_work_packages(root: &Path) -> Result<(), String> {
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
        "4.6",
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
        let evidence = package_string_set(package, "evidence_keys", &id)?;
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
    Ok(())
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
        "Design revision: v4.6",
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
    require_string(
        document.get("design_revision"),
        "evidence design_revision",
        "4.6",
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

fn check_machine(machine: &Table, design: &str, ids: &mut BTreeSet<String>) -> Result<(), String> {
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

    let transition_tables = machine
        .get("transition")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("machine {id:?} has no transitions"))?;
    let transitions = parse_transitions(&id, transition_tables, &states, &terminals)?;
    check_required_events(machine, &id, &transitions)?;
    check_reachability(&id, &initial, &states, &terminals, &transitions)?;
    Ok(())
}

fn parse_transitions(
    machine: &str,
    tables: &ArrayOfTables,
    states: &BTreeSet<String>,
    terminals: &BTreeSet<String>,
) -> Result<Vec<Transition>, String> {
    let mut transitions = Vec::new();
    let mut identities = BTreeSet::new();
    for table in tables {
        let from = string_field(table, "from", machine)?;
        let event = string_field(table, "event", machine)?;
        let to = string_field(table, "to", machine)?;
        if !states.contains(&from) || !states.contains(&to) {
            return Err(format!(
                "machine {machine:?} transition {from:?}:{event:?}->{to:?} uses an unknown state"
            ));
        }
        if terminals.contains(&from) {
            return Err(format!(
                "machine {machine:?} terminal state {from:?} has transition {event:?}"
            ));
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

    use super::{expand_expressions, expected_work_package_dependencies};

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
        assert_eq!(
            dependencies.get("WP-700"),
            Some(&BTreeSet::from([
                "WP-400".to_owned(),
                "WP-500".to_owned(),
                "WP-600".to_owned(),
            ]))
        );
    }
}
