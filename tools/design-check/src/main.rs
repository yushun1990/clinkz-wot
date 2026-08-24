//! Small structural checks for current machine-readable technical authority.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{Array, DocumentMut, Item, Table};

const PACKAGE_IDS: &[&str] = &[
    "WP-000", "WP-100", "WP-200", "WP-300", "WP-400", "WP-500", "WP-600", "WP-700",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("design check: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = repository_root()?;
    match std::env::args().nth(1).as_deref().unwrap_or("check") {
        "check" => {
            check_authority(&root)?;
            check_state_machines(&root)?;
            check_work_packages(&root)?;
            println!("design check: current authority, state machines, and package DAG valid");
        }
        "check-authority" => {
            check_authority(&root)?;
            println!("design check: current v5 authority valid");
        }
        "check-state" => {
            check_state_machines(&root)?;
            println!("design check: state-machine graphs valid");
        }
        "check-work-packages" => {
            check_work_packages(&root)?;
            println!("design check: package and Property Read dependency DAGs valid");
        }
        command => {
            return Err(format!(
                "unknown command {command:?}; expected check, check-authority, check-state, or check-work-packages"
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

fn parse_toml(root: &Path, relative: &str) -> Result<DocumentMut, String> {
    let path = root.join(relative);
    fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {relative}: {error}"))?
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {relative}: {error}"))
}

fn check_authority(root: &Path) -> Result<(), String> {
    let relative = "docs/spec/v5-authority-reset.toml";
    let document = parse_toml(root, relative)?;
    require_root_string(&document, "current_design_revision", "5.0", relative)?;
    require_root_string(&document, "status", "active", relative)?;

    let classification = document
        .get("classification")
        .and_then(Item::as_table)
        .ok_or_else(|| format!("{relative} has no classification table"))?;
    let mut classified = BTreeSet::new();
    let mut active = BTreeSet::new();
    for (name, item) in classification {
        let table = item
            .as_table()
            .ok_or_else(|| format!("classification.{name} is not a table"))?;
        let status = table_string(table, "authority_status", &format!("classification.{name}"))?;
        let requirements = table_strings(table, "requirements", &format!("classification.{name}"))?;
        let expected = table_integer(table, "expected_count", &format!("classification.{name}"))?;
        if expected != requirements.len() as i64 {
            return Err(format!(
                "classification.{name} expects {expected} requirements but declares {}",
                requirements.len()
            ));
        }
        for requirement in requirements {
            if !classified.insert(requirement.clone()) {
                return Err(format!(
                    "requirement {requirement:?} is classified more than once"
                ));
            }
            if status == "active" {
                active.insert(requirement);
            }
        }
    }

    let expected_classified = root_integer(&document, "classified_requirement_count", relative)?;
    let expected_active = root_integer(&document, "active_requirement_count", relative)?;
    if classified.len() as i64 != expected_classified || active.len() as i64 != expected_active {
        return Err(format!(
            "authority counts differ: classified={} active={}, expected classified={expected_classified} active={expected_active}",
            classified.len(),
            active.len()
        ));
    }
    let indexed = load_requirement_ids(root)?;
    if indexed != classified {
        return Err(set_difference(
            "authority classification",
            &classified,
            "requirements index",
            &indexed,
        ));
    }

    let sources = document
        .get("active_source")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{relative} has no active_source records"))?;
    let mut sourced = BTreeSet::new();
    for source in sources {
        let path = table_string(source, "path", "active_source")?;
        validate_relative_path(&path, "active_source path")?;
        let source_text = fs::read_to_string(root.join(&path))
            .map_err(|error| format!("cannot read active authority source {path}: {error}"))?;
        let requirements = table_strings(source, "requirements", &format!("active_source {path}"))?;
        let expected = table_integer(source, "expected_count", &format!("active_source {path}"))?;
        if expected != requirements.len() as i64 {
            return Err(format!(
                "active_source {path} has a mismatched expected_count"
            ));
        }
        for requirement in requirements {
            if !active.contains(&requirement) {
                return Err(format!(
                    "active_source {path} owns inactive requirement {requirement}"
                ));
            }
            if !sourced.insert(requirement.clone()) {
                return Err(format!(
                    "active requirement {requirement} has multiple sources"
                ));
            }
            let marker = format!("`{requirement}`:");
            if !source_text.contains(&marker) {
                return Err(format!("active source {path} does not define {marker}"));
            }
        }
    }
    if sourced != active {
        return Err(set_difference(
            "active classification",
            &active,
            "active sources",
            &sourced,
        ));
    }
    Ok(())
}

fn load_requirement_ids(root: &Path) -> Result<BTreeSet<String>, String> {
    let source = fs::read_to_string(root.join("docs/requirements.csv"))
        .map_err(|error| format!("cannot read docs/requirements.csv: {error}"))?;
    let mut lines = source.lines();
    let header = lines
        .next()
        .ok_or_else(|| "requirements index is empty".to_owned())?;
    if !header.starts_with("requirement,") {
        return Err("requirements index has an unexpected header".to_owned());
    }
    let mut requirements = BTreeSet::new();
    for line in lines.filter(|line| !line.trim().is_empty()) {
        let field = line.split(',').next().unwrap_or_default();
        for expression in field.split('|') {
            for id in expand_requirement(expression)? {
                if !requirements.insert(id.clone()) {
                    return Err(format!("duplicate indexed requirement {id}"));
                }
            }
        }
    }
    Ok(requirements)
}

fn expand_requirement(expression: &str) -> Result<Vec<String>, String> {
    let Some((first, last)) = expression.split_once("..") else {
        if expression.is_empty() {
            return Err("empty requirement expression".to_owned());
        }
        return Ok(vec![expression.to_owned()]);
    };
    if first.len() < 4 || last.len() != 3 {
        return Err(format!("invalid requirement range {expression:?}"));
    }
    let (prefix, first) = first.split_at(first.len() - 3);
    let first = first
        .parse::<u16>()
        .map_err(|_| format!("invalid range {expression:?}"))?;
    let last = last
        .parse::<u16>()
        .map_err(|_| format!("invalid range {expression:?}"))?;
    if first > last {
        return Err(format!("descending requirement range {expression:?}"));
    }
    Ok((first..=last)
        .map(|number| format!("{prefix}{number:03}"))
        .collect())
}

fn check_work_packages(root: &Path) -> Result<(), String> {
    let relative = "docs/work-packages/index.toml";
    let document = parse_toml(root, relative)?;
    require_root_string(&document, "design_revision", "5.0", relative)?;
    let packages = document
        .get("package")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{relative} has no package records"))?;
    let mut graph = BTreeMap::new();
    for package in packages {
        let id = table_string(package, "id", "package")?;
        if graph.contains_key(&id) {
            return Err(format!("duplicate work package {id}"));
        }
        let status = table_string(package, "status", &id)?;
        if !matches!(status.as_str(), "complete" | "in-progress" | "planned") {
            return Err(format!("work package {id} has invalid status {status:?}"));
        }
        let document = table_string(package, "document", &id)?;
        require_file(root, &document, &format!("work package {id} document"))?;
        for owner in table_strings(package, "owner_crates", &id)? {
            require_file(
                root,
                &format!("{owner}/Cargo.toml"),
                &format!("work package {id} owner"),
            )?;
        }
        graph.insert(
            id,
            table_strings(package, "depends_on", "work package dependency")?,
        );
    }
    let expected: BTreeSet<String> = PACKAGE_IDS.iter().map(|id| (*id).to_owned()).collect();
    let actual: BTreeSet<String> = graph.keys().cloned().collect();
    if actual != expected {
        return Err(set_difference(
            "required packages",
            &expected,
            "registered packages",
            &actual,
        ));
    }
    check_dag(&graph, "work-package")?;

    let gate_path = root_string(&document, "integration_gate_manifest", relative)?;
    let gate = parse_toml(root, &gate_path)?;
    if root_integer(&gate, "schema_version", &gate_path)? != 3 {
        return Err(format!(
            "{gate_path} is not aggregate gate schema version 3"
        ));
    }
    require_root_string(&gate, "design_revision", "5.0", &gate_path)?;
    require_root_string(&gate, "id", "PROPERTY-READ-ARCHITECTURE", &gate_path)?;
    let gate_document = root_string(&gate, "document", &gate_path)?;
    require_file(root, &gate_document, "Property Read gate document")?;
    let status = root_string(&gate, "status", &gate_path)?;
    if !matches!(status.as_str(), "ready" | "passed") {
        return Err(format!("Property Read gate has invalid status {status:?}"));
    }
    let fixture_roots: BTreeSet<String> = item_strings(
        gate.get("fixture_roots")
            .ok_or_else(|| format!("{gate_path} has no fixture_roots"))?,
        &format!("{gate_path}.fixture_roots"),
    )?
    .into_iter()
    .collect();
    let expected_fixture_roots = BTreeSet::from([
        "tools/architecture-fixtures/property-read-binding".to_owned(),
        "tools/architecture-fixtures/property-read-runner".to_owned(),
    ]);
    if fixture_roots != expected_fixture_roots {
        return Err(set_difference(
            "Property Read fixture roots",
            &expected_fixture_roots,
            "registered fixture roots",
            &fixture_roots,
        ));
    }
    for fixture_root in &fixture_roots {
        require_file(
            root,
            &format!("{fixture_root}/Cargo.toml"),
            "Property Read fixture root",
        )?;
    }
    let aggregate_evidence = item_strings(
        gate.get("evidence")
            .ok_or_else(|| format!("{gate_path} has no aggregate evidence"))?,
        &format!("{gate_path}.evidence"),
    )?;
    if status == "passed" && aggregate_evidence.is_empty() {
        return Err("passed Property Read gate has no aggregate evidence".to_owned());
    }
    for evidence in aggregate_evidence {
        require_file(root, &evidence, "Property Read aggregate evidence")?;
    }
    let test_commands = item_strings(
        gate.get("test_commands")
            .ok_or_else(|| format!("{gate_path} has no test_commands"))?,
        &format!("{gate_path}.test_commands"),
    )?;
    for required in [
        "cargo check --locked -p clinkz-wot-property-read-binding-fixture --no-default-features",
        "cargo check --locked -p clinkz-wot-property-read-architecture-runner --no-default-features --features async",
        "cargo test --locked -p clinkz-wot-property-read-architecture-runner",
    ] {
        if !test_commands.iter().any(|command| command == required) {
            return Err(format!(
                "{gate_path} does not register required aggregate command {required:?}"
            ));
        }
    }
    let dependencies = gate
        .get("dependency")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{gate_path} has no dependency records"))?;
    let mut ids = BTreeSet::new();
    for dependency in dependencies {
        let id = table_string(dependency, "id", "Property Read dependency")?;
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate Property Read dependency {id}"));
        }
        let owner = table_string(dependency, "owner_package", &id)?;
        if !graph.contains_key(&owner) {
            return Err(format!(
                "Property Read dependency {id} has unknown owner {owner}"
            ));
        }
        if table_string(dependency, "status", &id)? != "covered" {
            return Err(format!("Property Read dependency {id} is not covered"));
        }
        for evidence in table_strings(dependency, "evidence", &id)? {
            require_file(
                root,
                &evidence,
                &format!("Property Read dependency {id} evidence"),
            )?;
        }
    }
    if ids.is_empty() {
        return Err("Property Read gate has no current technical dependencies".to_owned());
    }
    Ok(())
}

fn check_dag(graph: &BTreeMap<String, Vec<String>>, context: &str) -> Result<(), String> {
    let mut incoming: BTreeMap<String, usize> = graph.keys().map(|id| (id.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, dependencies) in graph {
        let mut seen = BTreeSet::new();
        for dependency in dependencies {
            if !graph.contains_key(dependency) {
                return Err(format!("{context} {id} depends on unknown {dependency}"));
            }
            if !seen.insert(dependency) {
                return Err(format!("{context} {id} repeats dependency {dependency}"));
            }
            *incoming.get_mut(id).expect("registered node") += 1;
            outgoing
                .entry(dependency.clone())
                .or_default()
                .push(id.clone());
        }
    }
    let mut ready: VecDeque<String> = incoming
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(id.clone()))
        .collect();
    let mut visited = 0;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        for dependent in outgoing.get(&id).into_iter().flatten() {
            let count = incoming.get_mut(dependent).expect("registered dependent");
            *count -= 1;
            if *count == 0 {
                ready.push_back(dependent.clone());
            }
        }
    }
    if visited != graph.len() {
        return Err(format!("{context} dependency graph contains a cycle"));
    }
    Ok(())
}

fn check_state_machines(root: &Path) -> Result<(), String> {
    let relative = "docs/state-machines.toml";
    let document = parse_toml(root, relative)?;
    let known_requirements = load_requirement_ids(root)?;
    let machines = document
        .get("machine")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{relative} has no machine records"))?;
    let mut machine_ids = BTreeSet::new();
    for machine in machines {
        let id = table_string(machine, "id", "machine")?;
        if !machine_ids.insert(id.clone()) {
            return Err(format!("duplicate state machine {id}"));
        }
        check_requirement_references(machine, &known_requirements, &id)?;
        let states: BTreeSet<String> = table_strings(machine, "states", &id)?.into_iter().collect();
        let initial = table_string(machine, "initial", &id)?;
        if !states.contains(&initial) {
            return Err(format!("machine {id} has unknown initial state {initial}"));
        }
        for terminal in table_strings(machine, "terminal", &id)? {
            if !states.contains(&terminal) {
                return Err(format!(
                    "machine {id} has unknown terminal state {terminal}"
                ));
            }
        }
        let transitions = machine
            .get("transition")
            .and_then(Item::as_array_of_tables)
            .ok_or_else(|| format!("machine {id} has no transitions"))?;
        let mut events = BTreeSet::new();
        let mut edges: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for transition in transitions {
            let from = table_string(transition, "from", &format!("machine {id} transition"))?;
            let event = table_string(transition, "event", &format!("machine {id} transition"))?;
            let to = table_string(transition, "to", &format!("machine {id} transition"))?;
            if !states.contains(&from) || !states.contains(&to) {
                return Err(format!(
                    "machine {id} transition {from}:{event}->{to} uses an unknown state"
                ));
            }
            if !events.insert(format!("{from}:{event}")) {
                return Err(format!("machine {id} repeats event {from}:{event}"));
            }
            edges.entry(from).or_default().push(to);
        }
        for required in table_strings(machine, "required_events", &id)? {
            if !events.contains(&required) {
                return Err(format!("machine {id} misses required event {required}"));
            }
        }
        let mut reachable = BTreeSet::from([initial.clone()]);
        let mut queue = VecDeque::from([initial]);
        while let Some(state) = queue.pop_front() {
            for target in edges.get(&state).into_iter().flatten() {
                if reachable.insert(target.clone()) {
                    queue.push_back(target.clone());
                }
            }
        }
        if reachable != states {
            return Err(set_difference(
                &format!("machine {id} states"),
                &states,
                "reachable states",
                &reachable,
            ));
        }
    }

    let compositions = document
        .get("composition")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{relative} has no composition records"))?;
    let mut composition_ids = BTreeSet::new();
    for composition in compositions {
        let id = table_string(composition, "id", "composition")?;
        if !composition_ids.insert(id.clone()) {
            return Err(format!("duplicate state-machine composition {id}"));
        }
        check_requirement_references(composition, &known_requirements, &id)?;
        if table_strings(composition, "participants", &id)?.is_empty() {
            return Err(format!("composition {id} has no participants"));
        }
        for (field, item) in composition {
            if field.ends_with("_transitions") {
                for transition in item_strings(item, &format!("composition {id}.{field}"))? {
                    if !transition.contains(':') || !transition.contains("->") {
                        return Err(format!(
                            "composition {id}.{field} has invalid transition {transition:?}"
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_requirement_references(
    table: &Table,
    known: &BTreeSet<String>,
    context: &str,
) -> Result<(), String> {
    for requirement in table_strings(table, "requirements", context)? {
        if !known.contains(&requirement) {
            return Err(format!(
                "{context} references unknown requirement {requirement}"
            ));
        }
    }
    Ok(())
}

fn require_file(root: &Path, relative: &str, context: &str) -> Result<(), String> {
    validate_relative_path(relative, context)?;
    if !root.join(relative).is_file() {
        return Err(format!("{context} does not exist: {relative}"));
    }
    Ok(())
}

fn validate_relative_path(path: &str, context: &str) -> Result<(), String> {
    if path.is_empty() || path.starts_with('/') || path.split('/').any(|part| part == "..") {
        return Err(format!(
            "{context} is not a repository-relative path: {path:?}"
        ));
    }
    Ok(())
}

fn require_root_string(
    document: &DocumentMut,
    field: &str,
    expected: &str,
    context: &str,
) -> Result<(), String> {
    let actual = root_string(document, field, context)?;
    if actual != expected {
        return Err(format!(
            "{context} {field} is {actual:?}; expected {expected:?}"
        ));
    }
    Ok(())
}

fn root_string(document: &DocumentMut, field: &str, context: &str) -> Result<String, String> {
    document
        .get(field)
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{context} has no string {field}"))
}

fn root_integer(document: &DocumentMut, field: &str, context: &str) -> Result<i64, String> {
    document
        .get(field)
        .and_then(Item::as_integer)
        .ok_or_else(|| format!("{context} has no integer {field}"))
}

fn table_string(table: &Table, field: &str, context: &str) -> Result<String, String> {
    table
        .get(field)
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{context} has no string {field}"))
}

fn table_integer(table: &Table, field: &str, context: &str) -> Result<i64, String> {
    table
        .get(field)
        .and_then(Item::as_integer)
        .ok_or_else(|| format!("{context} has no integer {field}"))
}

fn table_strings(table: &Table, field: &str, context: &str) -> Result<Vec<String>, String> {
    let item = table
        .get(field)
        .ok_or_else(|| format!("{context} has no {field}"))?;
    item_strings(item, &format!("{context}.{field}"))
}

fn item_strings(item: &Item, context: &str) -> Result<Vec<String>, String> {
    let array: &Array = item
        .as_array()
        .ok_or_else(|| format!("{context} is not an array"))?;
    array
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{context} contains a non-string value"))
        })
        .collect()
}

fn set_difference(
    left_name: &str,
    left: &BTreeSet<String>,
    right_name: &str,
    right: &BTreeSet<String>,
) -> String {
    let missing: Vec<_> = left.difference(right).cloned().collect();
    let extra: Vec<_> = right.difference(left).cloned().collect();
    format!("{left_name} and {right_name} differ; missing={missing:?}, extra={extra:?}")
}
