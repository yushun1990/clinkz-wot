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
            println!("design check: current v6 authority valid");
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
    let document = parse_toml(root, "docs/spec/authority.toml")?;
    check_authority_document(root, &document)
}

fn check_authority_document(root: &Path, document: &DocumentMut) -> Result<(), String> {
    let relative = "docs/spec/authority.toml";
    require_root_string(document, "current_design_revision", "6.0", relative)?;
    require_root_string(document, "status", "active", relative)?;
    require_root_string(
        document,
        "implementation_acceptance",
        "not-transferred",
        relative,
    )?;
    require_root_string(
        document,
        "activation",
        "atomic-integration-of-independently-reviewed-authority-tree",
        relative,
    )?;
    require_file(
        root,
        &root_string(document, "decision", relative)?,
        "authority decision",
    )?;
    require_file(
        root,
        &root_string(document, "workspace_basis", relative)?,
        "authority investigation",
    )?;
    let predecessor = parse_toml(root, "docs/spec/v5-authority-reset.toml")?;
    require_root_string(&predecessor, "status", "superseded", "predecessor")?;
    require_root_string(&predecessor, "successor", relative, "predecessor")?;
    let metadata_text =
        fs::read_to_string(root.join("docs/requirements.csv")).map_err(|e| e.to_string())?;
    let mut metadata = BTreeMap::new();
    for line in metadata_text.lines().skip(1) {
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 11 || fields.iter().any(|f| f.is_empty()) {
            return Err("requirements metadata must have 11 nonempty columns".to_owned());
        }
        if metadata.insert(fields[0].to_owned(), fields).is_some() {
            return Err("duplicate requirement metadata".to_owned());
        }
    }
    let sources = document
        .get("active_source")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "authority has no active sources".to_owned())?;
    let mut active = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for source in sources {
        let path = table_string(source, "path", "active_source")?;
        require_file(root, &path, "active source")?;
        if !paths.insert(path.clone()) {
            return Err(format!("duplicate active source {path}"));
        }
        let text = fs::read_to_string(root.join(&path)).map_err(|e| e.to_string())?;
        let requirements = table_strings(source, "requirements", &path)?;
        if requirements.is_empty() {
            return Err(format!("empty active source {path}"));
        }
        for id in requirements {
            if !active.insert(id.clone()) {
                return Err(format!("multiple active owners for {id}"));
            }
            let marker = format!("`{id}`:");
            if text.matches(&marker).count() != 1 {
                return Err(format!("{path} must define {id} exactly once"));
            }
            let row = metadata
                .get(&id)
                .ok_or_else(|| format!("unindexed active requirement {id}"))?;
            if row[8] != path || row[9] != "active" || row[10] != "-" {
                return Err(format!("active source/metadata mismatch for {id}"));
            }
        }
    }
    if active.len() as i64 != root_integer(document, "active_requirement_count", relative)? {
        return Err("active requirement count differs from source map".to_owned());
    }
    if active != load_requirement_ids(root)? {
        return Err("active metadata differs from source map".to_owned());
    }
    let records = document
        .get("prior_requirement")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "authority has no predecessor dispositions".to_owned())?;
    let mut prior = BTreeSet::new();
    for record in records {
        let id = table_string(record, "id", "prior_requirement")?;
        if !prior.insert(id.clone()) {
            return Err(format!("duplicate prior disposition {id}"));
        }
        let disposition = table_string(record, "disposition", &id)?;
        let successors = table_strings(record, "successors", &id)?;
        if table_string(record, "reason", &id)?.is_empty()
            || table_string(record, "prior_status", &id)?.is_empty()
        {
            return Err(format!("missing prior disposition rationale {id}"));
        }
        let row = metadata
            .get(&id)
            .ok_or_else(|| format!("unindexed prior identity {id}"))?;
        match disposition.as_str() {
            "reaffirmed" if active.contains(&id) && successors == [id.clone()] => {}
            "superseded"
                if !active.contains(&id) && !successors.is_empty() && row[9] == "superseded" => {}
            "retired" if !active.contains(&id) && successors.is_empty() && row[9] == "retired" => {}
            _ => return Err(format!("invalid predecessor disposition for {id}")),
        }
        if successors.iter().any(|r| !active.contains(r)) {
            return Err(format!("inactive successor for {id}"));
        }
        if disposition != "reaffirmed"
            && row[10]
                != if successors.is_empty() {
                    "-".to_owned()
                } else {
                    successors.join("|")
                }
        {
            return Err(format!("successor metadata differs for {id}"));
        }
    }
    if prior.len() as i64 != root_integer(document, "predecessor_requirement_count", relative)? {
        return Err("incomplete predecessor dispositions".to_owned());
    }
    let indexed: BTreeSet<_> = metadata.keys().cloned().collect();
    if indexed != prior.union(&active).cloned().collect() {
        return Err("unclassified requirement metadata".to_owned());
    }
    for scoped in document
        .get("scoped_source")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "authority has no scoped source dispositions".to_owned())?
    {
        let path = table_string(scoped, "path", "scoped_source")?;
        validate_relative_path(&path, "scoped source")?;
        if !root.join(&path).exists() || table_string(scoped, "scope", &path)?.is_empty() {
            return Err(format!("invalid scoped source {path}"));
        }
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
        let fields: Vec<_> = line.split(',').collect();
        if fields.get(9) != Some(&"active") {
            continue;
        }
        let field = fields[0];
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
    if root_integer(&document, "schema_version", relative)? != 7 {
        return Err(format!("{relative} is not work-package schema version 7"));
    }
    require_root_string(&document, "design_revision", "6.0", relative)?;
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
    if document.get("tranche").is_some() {
        check_tranche_registry(root, &document, &graph)?;
    }

    // Completed v5 tranches/gates are historical provenance, not current dependencies.
    let historical = document
        .get("historical_tranche")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "missing historical tranche dispositions".to_owned())?;
    let mut historical_graph = BTreeMap::new();
    for record in historical {
        let id = table_string(record, "id", "historical tranche")?;
        if table_string(record, "successor_admission", &id)? != "none"
            || table_string(record, "v6_disposition", &id)?.is_empty()
        {
            return Err(format!(
                "historical tranche {id} cannot grant successor admission"
            ));
        }
        if historical_graph
            .insert(id.clone(), table_strings(record, "depends_on", &id)?)
            .is_some()
        {
            return Err(format!("duplicate historical tranche {id}"));
        }
        let owner = table_string(record, "work_package", &id)?;
        if !graph.contains_key(&owner) {
            return Err(format!("unknown historical package {owner}"));
        }
        require_file(
            root,
            &table_string(record, "completion_evidence_path", &id)?,
            "historical evidence",
        )?;
    }
    check_dag(&historical_graph, "historical tranche")?;
    for field in [
        "historical_producer_gate_manifest",
        "historical_consumer_gate_manifest",
    ] {
        let path = root_string(&document, field, relative)?;
        let gate = parse_toml(root, &path)?;
        require_root_string(
            &gate,
            "authority_scope",
            "historical-property-read-only",
            &path,
        )?;
        require_root_string(&gate, "successor_acceptance", "not-transferred", &path)?;
    }
    Ok(())
}

fn check_tranche_registry(
    root: &Path,
    document: &DocumentMut,
    package_graph: &BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    let relative = "docs/work-packages/index.toml";
    let tranches = document
        .get("tranche")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{relative} has no tranche admission records"))?;
    if tranches.is_empty() {
        return Err(format!("{relative} has no tranche admission records"));
    }

    let known_requirements = load_requirement_ids(root)?;
    let packages = document
        .get("package")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{relative} has no package records"))?;
    let mut package_status = BTreeMap::new();
    for package in packages {
        package_status.insert(
            table_string(package, "id", "package")?,
            table_string(package, "status", "package")?,
        );
    }

    let mut graph = BTreeMap::new();
    let mut tranche_state = BTreeMap::new();
    for tranche in tranches {
        let id = table_string(tranche, "id", "tranche")?;
        if graph.contains_key(&id) {
            return Err(format!("duplicate tranche {id}"));
        }
        let work_package = table_string(tranche, "work_package", &id)?;
        if !package_graph.contains_key(&work_package) {
            return Err(format!(
                "tranche {id} has unknown work package {work_package}"
            ));
        }
        let status = table_string(tranche, "status", &id)?;
        if !matches!(
            status.as_str(),
            "planned" | "in-progress" | "complete" | "reopened"
        ) {
            return Err(format!("tranche {id} has invalid status {status:?}"));
        }
        let admission_status = table_string(tranche, "admission_status", &id)?;
        if !matches!(admission_status.as_str(), "candidate" | "admitted") {
            return Err(format!(
                "tranche {id} has invalid admission status {admission_status:?}"
            ));
        }
        let impact_status = table_string(tranche, "impact_status", &id)?;
        if !matches!(
            impact_status.as_str(),
            "current" | "review-required" | "reopened"
        ) {
            return Err(format!(
                "tranche {id} has invalid impact status {impact_status:?}"
            ));
        }
        if !valid_tranche_impact_state(&status, &admission_status, &impact_status) {
            return Err(format!(
                "tranche {id} has inconsistent lifecycle/admission/impact state \
                 {status:?}/{admission_status:?}/{impact_status:?}"
            ));
        }
        if impact_status != "current" {
            if table_string(tranche, "impact_review", &id)?.is_empty() {
                return Err(format!("tranche {id} has no impact review location"));
            }
            let disposition = table_string(tranche, "completion_evidence_disposition", &id)?;
            let expected = if impact_status == "review-required" {
                "under-review"
            } else {
                "superseded"
            };
            if disposition != expected {
                return Err(format!(
                    "tranche {id} has completion evidence disposition {disposition:?}; \
                     expected {expected:?} for impact state {impact_status:?}"
                ));
            }
        }
        tranche_state.insert(id.clone(), (status.clone(), impact_status.clone()));

        for dependency in table_strings(tranche, "depends_on_packages", &id)? {
            match package_status.get(&dependency).map(String::as_str) {
                Some("complete") => {}
                Some(state) => {
                    return Err(format!(
                        "tranche {id} depends on incomplete package {dependency} ({state})"
                    ));
                }
                None => {
                    return Err(format!(
                        "tranche {id} depends on unknown package {dependency}"
                    ));
                }
            }
        }
        for requirement in table_strings(tranche, "requirements", &id)? {
            if !known_requirements.contains(&requirement) {
                return Err(format!(
                    "tranche {id} references unknown requirement {requirement}"
                ));
            }
        }
        let feature_cells = table_strings(tranche, "feature_cells", &id)?;
        if feature_cells.is_empty()
            || feature_cells
                .iter()
                .any(|cell| !matches!(cell.as_str(), "no-default" | "async-no-std" | "std"))
        {
            return Err(format!("tranche {id} has invalid or empty feature cells"));
        }
        for artifact in table_strings(tranche, "authoritative_artifacts", &id)? {
            require_file(root, &artifact, &format!("tranche {id} authority"))?;
        }
        for implementation_path in table_strings(tranche, "implementation_paths", &id)? {
            validate_relative_path(
                &implementation_path,
                &format!("tranche {id} implementation path"),
            )?;
        }
        if table_strings(tranche, "pre_implementation_checks", &id)?.is_empty() {
            return Err(format!("tranche {id} has no pre-implementation checks"));
        }
        if table_string(tranche, "admission_review", &id)?.is_empty() {
            return Err(format!("tranche {id} has no independent review location"));
        }
        if table_strings(tranche, "completion_evidence_keys", &id)?.is_empty() {
            return Err(format!("tranche {id} has no completion evidence keys"));
        }
        let evidence_path = table_string(tranche, "completion_evidence_path", &id)?;
        validate_relative_path(
            &evidence_path,
            &format!("tranche {id} completion evidence path"),
        )?;
        if matches!(status.as_str(), "complete" | "reopened") {
            require_file(
                root,
                &evidence_path,
                &format!("tranche {id} completion evidence"),
            )?;
            let evidence = parse_toml(root, &evidence_path)?;
            require_root_string(&evidence, "tranche", &id, &evidence_path)?;
            let evidence_status = root_string(&evidence, "status", &evidence_path)?;
            let expected_evidence_status = if impact_status == "reopened" {
                "superseded"
            } else {
                "passed"
            };
            if evidence_status != expected_evidence_status {
                return Err(format!(
                    "tranche {id} has {impact_status:?} impact but completion evidence \
                     status {evidence_status:?}; expected {expected_evidence_status:?}"
                ));
            }
        }
        for field in [
            "state_machines",
            "resource_changes",
            "old_api_removals",
            "performance_workloads",
        ] {
            let _ = table_strings(tranche, field, &id)?;
        }
        graph.insert(
            id,
            table_strings(tranche, "depends_on", "tranche dependency")?,
        );
    }
    check_tranche_dependency_graph(&graph, &tranche_state)
}

fn valid_tranche_impact_state(status: &str, admission: &str, impact: &str) -> bool {
    match (status, admission, impact) {
        ("reopened", "candidate", "reopened") | ("complete", "admitted", "review-required") => true,
        ("reopened", _, _) | (_, _, "reopened" | "review-required") => false,
        (_, _, "current") => true,
        _ => false,
    }
}

fn check_tranche_dependency_graph(
    graph: &BTreeMap<String, Vec<String>>,
    tranche_state: &BTreeMap<String, (String, String)>,
) -> Result<(), String> {
    for (id, dependencies) in graph {
        let (status, impact) = tranche_state
            .get(id)
            .ok_or_else(|| format!("tranche {id} has no lifecycle/impact state"))?;
        for dependency in dependencies {
            match tranche_state.get(dependency) {
                Some((dependency_status, dependency_impact))
                    if valid_tranche_dependency_edge(
                        impact,
                        dependency_status,
                        dependency_impact,
                    ) => {}
                Some((dependency_status, dependency_impact)) => {
                    return Err(format!(
                        "tranche {id} ({status}, impact {impact}) cannot depend on tranche \
                         {dependency} ({dependency_status}, impact {dependency_impact})"
                    ));
                }
                None => {
                    return Err(format!(
                        "tranche {id} depends on unknown tranche {dependency}"
                    ));
                }
            }
        }
    }
    check_dag(graph, "tranche")
}

fn valid_tranche_dependency_edge(
    dependent_impact: &str,
    dependency_status: &str,
    dependency_impact: &str,
) -> bool {
    match dependent_impact {
        "current" => dependency_status == "complete" && dependency_impact == "current",
        // These states preserve the pre-existing dependency graph while impact
        // review or corrective work is active. They do not make either tranche
        // executable or satisfy a current tranche's dependency.
        "review-required" | "reopened" => matches!(
            (dependency_status, dependency_impact),
            ("complete", "current" | "review-required") | ("reopened", "reopened")
        ),
        _ => false,
    }
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        check_tranche_dependency_graph, parse_toml, repository_root, valid_tranche_impact_state,
    };

    #[test]
    fn successor_rejects_duplicate_owner_and_missing_predecessor_disposition() {
        let root = repository_root().unwrap();
        let authority = parse_toml(&root, "docs/spec/authority.toml").unwrap();
        super::check_authority_document(&root, &authority).unwrap();
        let mut duplicate = authority.clone();
        let source = duplicate["active_source"]
            .as_array_of_tables()
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .clone();
        duplicate["active_source"]
            .as_array_of_tables_mut()
            .unwrap()
            .push(source);
        assert!(
            super::check_authority_document(&root, &duplicate)
                .unwrap_err()
                .contains("duplicate active source")
        );
        let mut missing = authority.clone();
        missing["prior_requirement"]
            .as_array_of_tables_mut()
            .unwrap()
            .remove(0);
        assert!(
            super::check_authority_document(&root, &missing)
                .unwrap_err()
                .contains("predecessor dispositions")
        );
    }

    #[test]
    fn successor_rejects_inactive_replacement_and_inherited_acceptance() {
        let root = repository_root().unwrap();
        let authority = parse_toml(&root, "docs/spec/authority.toml").unwrap();
        let mut invalid = authority.clone();
        invalid["prior_requirement"]
            .as_array_of_tables_mut()
            .unwrap()
            .iter_mut()
            .next()
            .unwrap()["successors"] = toml_edit::value({
            let mut a = toml_edit::Array::new();
            a.push("PLAN-SET-001");
            a
        });
        assert!(super::check_authority_document(&root, &invalid).is_err());
        let mut inherited = authority.clone();
        inherited["implementation_acceptance"] = toml_edit::value("passed-by-inheritance");
        assert!(
            super::check_authority_document(&root, &inherited)
                .unwrap_err()
                .contains("implementation_acceptance")
        );
    }

    #[test]
    fn tranche_impact_states_distinguish_current_review_and_reopening() {
        assert!(valid_tranche_impact_state(
            "complete", "admitted", "current"
        ));
        assert!(valid_tranche_impact_state(
            "complete",
            "admitted",
            "review-required"
        ));
        assert!(valid_tranche_impact_state(
            "reopened",
            "candidate",
            "reopened"
        ));

        assert!(!valid_tranche_impact_state(
            "reopened", "admitted", "reopened"
        ));
        assert!(!valid_tranche_impact_state(
            "complete", "admitted", "reopened"
        ));
        assert!(!valid_tranche_impact_state(
            "planned",
            "candidate",
            "review-required"
        ));
    }

    #[test]
    fn impact_graph_retains_transitively_non_current_dependencies() {
        let graph = BTreeMap::from([
            ("current-root".to_owned(), vec![]),
            ("reopened-root".to_owned(), vec!["current-root".to_owned()]),
            (
                "reopened-dependent".to_owned(),
                vec!["reopened-root".to_owned()],
            ),
            (
                "review-on-reopened".to_owned(),
                vec!["reopened-root".to_owned()],
            ),
            (
                "review-on-review".to_owned(),
                vec!["review-on-reopened".to_owned()],
            ),
        ]);
        let states = BTreeMap::from([
            (
                "current-root".to_owned(),
                ("complete".to_owned(), "current".to_owned()),
            ),
            (
                "reopened-root".to_owned(),
                ("reopened".to_owned(), "reopened".to_owned()),
            ),
            (
                "reopened-dependent".to_owned(),
                ("reopened".to_owned(), "reopened".to_owned()),
            ),
            (
                "review-on-reopened".to_owned(),
                ("complete".to_owned(), "review-required".to_owned()),
            ),
            (
                "review-on-review".to_owned(),
                ("complete".to_owned(), "review-required".to_owned()),
            ),
        ]);

        assert!(check_tranche_dependency_graph(&graph, &states).is_ok());
    }

    #[test]
    fn impact_graph_rejects_current_dependencies_on_non_current_tranches() {
        for (dependency_status, dependency_impact) in
            [("reopened", "reopened"), ("complete", "review-required")]
        {
            let graph = BTreeMap::from([
                ("dependency".to_owned(), vec![]),
                (
                    "current-dependent".to_owned(),
                    vec!["dependency".to_owned()],
                ),
            ]);
            let states = BTreeMap::from([
                (
                    "dependency".to_owned(),
                    (dependency_status.to_owned(), dependency_impact.to_owned()),
                ),
                (
                    "current-dependent".to_owned(),
                    ("complete".to_owned(), "current".to_owned()),
                ),
            ]);

            let error = check_tranche_dependency_graph(&graph, &states)
                .expect_err("a current tranche must reject a non-current dependency");
            assert!(error.contains("current-dependent"));
            assert!(error.contains("cannot depend on tranche dependency"));
        }
    }
}
