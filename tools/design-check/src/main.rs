//! Small structural checks for current machine-readable technical authority.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use toml_edit::{Array, DocumentMut, Item, Table};

const PACKAGE_IDS: &[&str] = &[
    "WP-000", "WP-100", "WP-200", "WP-300", "WP-400", "WP-500", "WP-600", "WP-700",
];
const PREDECESSOR_COMMIT: &str = "ed09f059e383b7a1b9d22902ee29aa218b4471a6";
const PREDECESSOR_MANIFEST: &str = "docs/spec/v5-authority-reset.toml";
const PACKAGE_INDEX: &str = "docs/work-packages/index.toml";
const CRITICAL_STATE_TRANSITIONS: &[(&str, &str, &str)] = &[
    (
        "notification-conversation",
        "Reserved:cancel-or-fail-before-native-start",
        "CleanupPending",
    ),
    (
        "native-infrastructure",
        "Reconnecting:terminal-reconnect-failure-fence-and-report",
        "Retiring",
    ),
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
    let predecessor = parse_toml(root, PREDECESSOR_MANIFEST)?;
    let pinned = pinned_predecessor(root)?;
    let expected_prior = check_predecessor_lineage(document, &predecessor, &pinned)?;
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
        let (expected_status, expected_source) = expected_prior
            .get(&id)
            .ok_or_else(|| format!("prior identity {id} is absent from pinned predecessor"))?;
        if table_string(record, "prior_status", &id)? != *expected_status
            || table_string(record, "prior_source", &id)? != *expected_source
        {
            return Err(format!(
                "prior status/source differs from pinned predecessor for {id}"
            ));
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
    let expected_prior_ids: BTreeSet<_> = expected_prior.keys().cloned().collect();
    if prior != expected_prior_ids {
        return Err(set_difference(
            "pinned predecessor identities",
            &expected_prior_ids,
            "dispositions",
            &prior,
        ));
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

fn pinned_predecessor(root: &Path) -> Result<DocumentMut, String> {
    pinned_document(root, PREDECESSOR_MANIFEST)
}

fn pinned_document(root: &Path, path: &str) -> Result<DocumentMut, String> {
    // The historical tree is the independent owner of v5.1 lineage. CI checks
    // out full history; do not replace the pinned object with current metadata.
    let object = format!("{PREDECESSOR_COMMIT}:{path}");
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", &object])
        .output()
        .map_err(|error| format!("cannot load pinned predecessor {object}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot load pinned predecessor {object} (full Git history required): {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("invalid UTF-8 in pinned predecessor {object}: {error}"))?
        .parse()
        .map_err(|error| format!("invalid pinned predecessor {object}: {error}"))
}

fn check_predecessor_lineage(
    successor: &DocumentMut,
    tombstone: &DocumentMut,
    pinned: &DocumentMut,
) -> Result<BTreeMap<String, (String, String)>, String> {
    let relative = "docs/spec/authority.toml";
    require_root_string(
        successor,
        "predecessor_commit",
        PREDECESSOR_COMMIT,
        relative,
    )?;
    require_root_string(
        successor,
        "predecessor_manifest",
        PREDECESSOR_MANIFEST,
        relative,
    )?;
    require_root_string(tombstone, "status", "superseded", PREDECESSOR_MANIFEST)?;
    require_root_string(tombstone, "successor", relative, PREDECESSOR_MANIFEST)?;
    require_root_string(
        tombstone,
        "historical_commit",
        PREDECESSOR_COMMIT,
        PREDECESSOR_MANIFEST,
    )?;
    require_root_string(
        tombstone,
        "historical_path",
        PREDECESSOR_MANIFEST,
        PREDECESSOR_MANIFEST,
    )?;
    require_root_string(pinned, "status", "active", "pinned predecessor")?;
    require_root_string(
        pinned,
        "current_design_revision",
        "5.1",
        "pinned predecessor",
    )?;
    require_root_string(
        tombstone,
        "current_design_revision",
        "5.1",
        PREDECESSOR_MANIFEST,
    )?;

    let classifications = pinned
        .get("classification")
        .and_then(Item::as_table)
        .ok_or_else(|| "pinned predecessor has no classifications".to_owned())?;
    let mut identities = BTreeMap::new();
    for (name, item) in classifications {
        let table = item
            .as_table()
            .ok_or_else(|| format!("pinned classification.{name} is not a table"))?;
        let status = table_string(table, "authority_status", name)?;
        let requirements = table_strings(table, "requirements", name)?;
        if requirements.len() as i64 != table_integer(table, "expected_count", name)? {
            return Err(format!("pinned classification.{name} count mismatch"));
        }
        for id in requirements {
            if identities
                .insert(
                    id.clone(),
                    (status.clone(), PREDECESSOR_MANIFEST.to_owned()),
                )
                .is_some()
            {
                return Err(format!("duplicate pinned predecessor identity {id}"));
            }
        }
    }
    if identities.len() as i64
        != root_integer(pinned, "classified_requirement_count", "pinned predecessor")?
        || identities.len() as i64
            != root_integer(successor, "predecessor_requirement_count", relative)?
    {
        return Err("pinned predecessor requirement count mismatch".to_owned());
    }

    let sources = pinned
        .get("active_source")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "pinned predecessor has no active sources".to_owned())?;
    let mut owned = BTreeSet::new();
    for source in sources {
        let path = table_string(source, "path", "pinned active_source")?;
        let requirements = table_strings(source, "requirements", &path)?;
        if requirements.len() as i64 != table_integer(source, "expected_count", &path)? {
            return Err(format!("pinned active source {path} count mismatch"));
        }
        for id in requirements {
            if !owned.insert(id.clone()) {
                return Err(format!("duplicate pinned active owner for {id}"));
            }
            let (status, recorded_source) = identities
                .get_mut(&id)
                .ok_or_else(|| format!("unclassified pinned active identity {id}"))?;
            if status != "active" {
                return Err(format!("inactive pinned identity has active owner: {id}"));
            }
            *recorded_source = path.clone();
        }
    }
    if owned.len() as i64 != root_integer(pinned, "active_requirement_count", "pinned predecessor")?
        || identities
            .iter()
            .any(|(id, (status, _))| (status == "active") != owned.contains(id))
    {
        return Err("pinned predecessor active source mismatch".to_owned());
    }
    Ok(identities)
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
    let document = parse_toml(root, PACKAGE_INDEX)?;
    check_work_packages_document(root, &document)
}

fn check_work_packages_document(root: &Path, document: &DocumentMut) -> Result<(), String> {
    let relative = PACKAGE_INDEX;
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
    check_historical_tranche_lineage(root, document, historical)?;
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
    Ok(())
}

fn check_historical_tranche_lineage(
    root: &Path,
    current: &DocumentMut,
    historical: &toml_edit::ArrayOfTables,
) -> Result<(), String> {
    require_root_string(
        current,
        "historical_basis_commit",
        PREDECESSOR_COMMIT,
        PACKAGE_INDEX,
    )?;
    let pinned = pinned_document(root, PACKAGE_INDEX)?;
    if root_integer(&pinned, "schema_version", "pinned work packages")? != 6 {
        return Err("pinned work-package schema revision differs".to_owned());
    }
    require_root_string(&pinned, "design_revision", "5.1", "pinned work packages")?;
    let original = pinned
        .get("tranche")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "pinned work packages have no tranches".to_owned())?;
    let mut expected = BTreeMap::new();
    for tranche in original {
        let id = table_string(tranche, "id", "pinned tranche")?;
        if expected.insert(id.clone(), tranche).is_some() {
            return Err(format!("duplicate pinned tranche {id}"));
        }
    }
    let mut actual = BTreeSet::new();
    for record in historical {
        let id = table_string(record, "id", "historical tranche")?;
        if !actual.insert(id.clone()) {
            return Err(format!("duplicate historical tranche {id}"));
        }
        let original = expected
            .get(&id)
            .ok_or_else(|| format!("historical tranche {id} is absent from pinned baseline"))?;
        require_table_string(
            record,
            "recorded_design_revision",
            &root_string(&pinned, "design_revision", "pinned work packages")?,
            &id,
        )?;
        for field in [
            "work_package",
            "status",
            "admission_status",
            "impact_status",
            "admission_review",
            "completion_evidence_path",
        ] {
            require_table_string(record, field, &table_string(original, field, &id)?, &id)?;
        }
        for field in [
            "impact_review",
            "impact_disposition",
            "decision_review",
            "completion_evidence_disposition",
        ] {
            let previous = optional_table_string(original, field, &id)?;
            let retained = optional_table_string(record, field, &id)?;
            if previous != retained {
                return Err(format!(
                    "historical tranche {id} {field} differs from pinned baseline"
                ));
            }
        }
        if table_integer(record, "sequence", &id)? != table_integer(original, "sequence", &id)? {
            return Err(format!(
                "historical tranche {id} sequence differs from pinned baseline"
            ));
        }
        for field in ["depends_on", "completion_evidence_keys"] {
            if table_strings(record, field, &id)? != table_strings(original, field, &id)? {
                return Err(format!(
                    "historical tranche {id} {field} differs from pinned baseline"
                ));
            }
        }
        let path = table_string(record, "completion_evidence_path", &id)?;
        let evidence = parse_toml(root, &path)?;
        let original_evidence = pinned_document(root, &path)?;
        check_historical_evidence_lineage(&path, &evidence, &original_evidence)?;
    }
    let expected_ids = expected.keys().cloned().collect();
    if actual != expected_ids {
        return Err(set_difference(
            "pinned tranches",
            &expected_ids,
            "historical tranches",
            &actual,
        ));
    }

    for (old_field, historical_field) in [
        (
            "integration_gate_manifest",
            "historical_producer_gate_manifest",
        ),
        (
            "consumer_integration_gate_manifest",
            "historical_consumer_gate_manifest",
        ),
    ] {
        let path = root_string(&pinned, old_field, PACKAGE_INDEX)?;
        require_root_string(current, historical_field, &path, PACKAGE_INDEX)?;
        let gate = parse_toml(root, &path)?;
        let original_gate = pinned_document(root, &path)?;
        check_historical_gate_lineage(&path, &gate, &original_gate)?;
    }
    Ok(())
}

fn check_historical_evidence_lineage(
    path: &str,
    current: &DocumentMut,
    pinned: &DocumentMut,
) -> Result<(), String> {
    require_root_string(
        current,
        "authority_scope",
        "historical-property-read-only",
        path,
    )?;
    require_root_string(current, "successor_acceptance", "not-transferred", path)?;
    if root_integer(current, "schema_version", path)?
        != root_integer(pinned, "schema_version", path)?
    {
        return Err(format!(
            "historical evidence {path} schema_version differs from pinned baseline"
        ));
    }
    for field in ["design_revision", "work_package", "tranche", "status"] {
        require_root_string(current, field, &root_string(pinned, field, path)?, path)?;
    }
    if let Some(keys) = pinned.get("evidence_keys") {
        if root_strings(current, "evidence_keys", path)?
            != item_strings(keys, &format!("{path}.evidence_keys"))?
        {
            return Err(format!(
                "historical evidence {path} evidence_keys differs from pinned baseline"
            ));
        }
    } else if current.get("evidence_keys").is_some() {
        return Err(format!(
            "historical evidence {path} adds unpinned evidence_keys"
        ));
    }
    Ok(())
}

fn check_historical_gate_lineage(
    path: &str,
    current: &DocumentMut,
    pinned: &DocumentMut,
) -> Result<(), String> {
    require_root_string(
        current,
        "authority_scope",
        "historical-property-read-only",
        path,
    )?;
    require_root_string(current, "successor_design_revision", "6.0", path)?;
    require_root_string(current, "successor_acceptance", "not-transferred", path)?;
    if root_integer(current, "schema_version", path)?
        != root_integer(pinned, "schema_version", path)?
    {
        return Err(format!(
            "historical gate {path} schema_version differs from pinned baseline"
        ));
    }
    for field in ["id", "status", "design_revision", "document"] {
        require_root_string(current, field, &root_string(pinned, field, path)?, path)?;
    }
    for field in ["fixture_roots", "evidence", "evidence_cases"] {
        if root_strings(current, field, path)? != root_strings(pinned, field, path)? {
            return Err(format!(
                "historical gate {path} {field} differs from pinned baseline"
            ));
        }
    }
    let current_dependencies = current
        .get("dependency")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("historical gate {path} has no dependencies"))?;
    let pinned_dependencies = pinned
        .get("dependency")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("pinned gate {path} has no dependencies"))?;
    let mut expected = BTreeMap::new();
    for dependency in pinned_dependencies {
        let id = table_string(dependency, "id", path)?;
        if expected.insert(id.clone(), dependency).is_some() {
            return Err(format!("pinned gate {path} repeats dependency {id}"));
        }
    }
    let mut actual = BTreeSet::new();
    for dependency in current_dependencies {
        let id = table_string(dependency, "id", path)?;
        if !actual.insert(id.clone()) {
            return Err(format!("historical gate {path} repeats dependency {id}"));
        }
        let original = expected
            .get(&id)
            .ok_or_else(|| format!("historical gate {path} has unpinned dependency {id}"))?;
        for field in ["owner_package", "status"] {
            require_table_string(dependency, field, &table_string(original, field, &id)?, &id)?;
        }
        if table_strings(dependency, "evidence", &id)? != table_strings(original, "evidence", &id)?
        {
            return Err(format!(
                "historical gate {path} dependency {id} evidence differs from pinned baseline"
            ));
        }
    }
    let expected_ids = expected.keys().cloned().collect();
    if actual != expected_ids {
        return Err(set_difference(
            "pinned gate dependencies",
            &expected_ids,
            "historical gate dependencies",
            &actual,
        ));
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
    check_state_machines_document(root, &document)
}

fn check_state_machines_document(root: &Path, document: &DocumentMut) -> Result<(), String> {
    let relative = "docs/state-machines.toml";
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
        let mut events = BTreeMap::new();
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
            if events
                .insert(format!("{from}:{event}"), to.clone())
                .is_some()
            {
                return Err(format!("machine {id} repeats event {from}:{event}"));
            }
            edges.entry(from).or_default().push(to);
        }
        let required_events: BTreeSet<_> = table_strings(machine, "required_events", &id)?
            .into_iter()
            .collect();
        for required in &required_events {
            if !events.contains_key(required) {
                return Err(format!("machine {id} misses required event {required}"));
            }
        }
        for &(_, required, target) in CRITICAL_STATE_TRANSITIONS
            .iter()
            .filter(|(machine, _, _)| *machine == id)
        {
            if !required_events.contains(required)
                || events.get(required).map(String::as_str) != Some(target)
            {
                return Err(format!(
                    "machine {id} misses required transition {required}->{target}"
                ));
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
    for &(id, _, _) in CRITICAL_STATE_TRANSITIONS {
        if !machine_ids.contains(id) {
            return Err(format!("missing critical state machine {id}"));
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

fn root_strings(document: &DocumentMut, field: &str, context: &str) -> Result<Vec<String>, String> {
    let item = document
        .get(field)
        .ok_or_else(|| format!("{context} has no {field}"))?;
    item_strings(item, &format!("{context}.{field}"))
}

fn table_string(table: &Table, field: &str, context: &str) -> Result<String, String> {
    table
        .get(field)
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{context} has no string {field}"))
}

fn optional_table_string(
    table: &Table,
    field: &str,
    context: &str,
) -> Result<Option<String>, String> {
    table
        .get(field)
        .map(|_| table_string(table, field, context))
        .transpose()
}

fn require_table_string(
    table: &Table,
    field: &str,
    expected: &str,
    context: &str,
) -> Result<(), String> {
    let actual = table_string(table, field, context)?;
    if actual != expected {
        return Err(format!(
            "historical tranche/gate {context} {field} differs from pinned baseline: {actual:?} != {expected:?}"
        ));
    }
    Ok(())
}

fn table_strings(table: &Table, field: &str, context: &str) -> Result<Vec<String>, String> {
    let item = table
        .get(field)
        .ok_or_else(|| format!("{context} has no {field}"))?;
    item_strings(item, &format!("{context}.{field}"))
}

fn table_integer(table: &Table, field: &str, context: &str) -> Result<i64, String> {
    table
        .get(field)
        .and_then(Item::as_integer)
        .ok_or_else(|| format!("{context} has no integer {field}"))
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
                .contains("pinned predecessor identities")
        );
    }

    #[test]
    fn predecessor_lineage_rejects_substituted_identity_status_and_source() {
        let root = repository_root().unwrap();
        let authority = parse_toml(&root, "docs/spec/authority.toml").unwrap();
        // Both IDs have the same successor and metadata disposition. The old
        // count/metadata checks accepted this swap despite changing lineage.
        let mut swapped = authority.clone();
        let records = swapped["prior_requirement"]
            .as_array_of_tables_mut()
            .unwrap();
        let bound = records
            .iter()
            .position(|record| record["id"].as_str() == Some("PLAN-BOUND-001"))
            .unwrap();
        let index = records
            .iter()
            .position(|record| record["id"].as_str() == Some("PLAN-INDEX-001"))
            .unwrap();
        records.get_mut(bound).unwrap()["id"] = toml_edit::value("PLAN-INDEX-001");
        records.get_mut(index).unwrap()["id"] = toml_edit::value("PLAN-BOUND-001");
        assert!(
            super::check_authority_document(&root, &swapped)
                .unwrap_err()
                .contains("status/source differs")
        );

        for (field, replacement, expected) in [
            ("id", "SUBSTITUTED-001", "absent from pinned predecessor"),
            (
                "prior_status",
                "inactive-historical-input",
                "status/source differs",
            ),
            (
                "prior_source",
                "docs/spec/planning.md",
                "status/source differs",
            ),
        ] {
            let mut altered = authority.clone();
            altered["prior_requirement"]
                .as_array_of_tables_mut()
                .unwrap()
                .iter_mut()
                .next()
                .unwrap()[field] = toml_edit::value(replacement);
            let error = super::check_authority_document(&root, &altered).unwrap_err();
            assert!(error.contains(expected), "{field}: {error}");
        }
        let mut inactive_source = authority.clone();
        let record = inactive_source["prior_requirement"]
            .as_array_of_tables_mut()
            .unwrap()
            .iter_mut()
            .find(|record| record["id"].as_str() == Some("API-CODEC-001"))
            .unwrap();
        record["prior_source"] = toml_edit::value("docs/spec/interaction-core.md");
        assert!(
            super::check_authority_document(&root, &inactive_source)
                .unwrap_err()
                .contains("status/source differs")
        );
    }

    #[test]
    fn predecessor_lineage_rejects_changed_revision_and_pinned_reference() {
        let root = repository_root().unwrap();
        let authority = parse_toml(&root, "docs/spec/authority.toml").unwrap();
        let tombstone = parse_toml(&root, super::PREDECESSOR_MANIFEST).unwrap();
        let pinned = super::pinned_predecessor(&root).unwrap();
        super::check_predecessor_lineage(&authority, &tombstone, &pinned).unwrap();

        let mut altered = authority.clone();
        altered["predecessor_commit"] = toml_edit::value("another-revision");
        assert!(
            super::check_predecessor_lineage(&altered, &tombstone, &pinned)
                .unwrap_err()
                .contains("predecessor_commit")
        );
        let mut altered = tombstone.clone();
        altered["historical_commit"] = toml_edit::value("another-revision");
        assert!(
            super::check_predecessor_lineage(&authority, &altered, &pinned)
                .unwrap_err()
                .contains("historical_commit")
        );
        let mut altered = tombstone.clone();
        altered["current_design_revision"] = toml_edit::value("5.0");
        assert!(
            super::check_predecessor_lineage(&authority, &altered, &pinned)
                .unwrap_err()
                .contains("current_design_revision")
        );
    }

    #[test]
    fn critical_failure_paths_cannot_be_removed_or_redirected() {
        let root = repository_root().unwrap();
        let document = parse_toml(&root, "docs/state-machines.toml").unwrap();
        super::check_state_machines_document(&root, &document).unwrap();
        for &(id, key, target) in super::CRITICAL_STATE_TRANSITIONS {
            let (from, event) = key.split_once(':').unwrap();
            for remove_requirement in [false, true] {
                let mut altered = document.clone();
                let machine = altered["machine"]
                    .as_array_of_tables_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|machine| machine["id"].as_str() == Some(id))
                    .unwrap();
                let transitions = machine["transition"].as_array_of_tables_mut().unwrap();
                let index = transitions
                    .iter()
                    .position(|t| {
                        t["from"].as_str() == Some(from) && t["event"].as_str() == Some(event)
                    })
                    .unwrap();
                transitions.remove(index);
                if remove_requirement {
                    machine["required_events"] = toml_edit::value(toml_edit::Array::new());
                }
                let error = super::check_state_machines_document(&root, &altered).unwrap_err();
                assert!(error.contains("required"), "{id}: {error}");
            }
            let mut altered = document.clone();
            let machine = altered["machine"]
                .as_array_of_tables_mut()
                .unwrap()
                .iter_mut()
                .find(|machine| machine["id"].as_str() == Some(id))
                .unwrap();
            let transition = machine["transition"]
                .as_array_of_tables_mut()
                .unwrap()
                .iter_mut()
                .find(|t| t["event"].as_str() == Some(event))
                .unwrap();
            assert_eq!(transition["to"].as_str(), Some(target));
            transition["to"] = toml_edit::value(from);
            let error = super::check_state_machines_document(&root, &altered).unwrap_err();
            assert!(error.contains("required transition"), "{id}: {error}");
        }
    }

    #[test]
    fn historical_tranches_reject_missing_identity_or_changed_status_revision_and_evidence() {
        let root = repository_root().unwrap();
        let index = parse_toml(&root, super::PACKAGE_INDEX).unwrap();
        super::check_work_packages_document(&root, &index).unwrap();
        let mut missing = index.clone();
        let historical = missing["historical_tranche"]
            .as_array_of_tables_mut()
            .unwrap();
        historical.remove(historical.len() - 1);
        assert!(
            super::check_work_packages_document(&root, &missing)
                .unwrap_err()
                .contains("pinned tranches")
        );

        let id = "WP-200-PREPARATION-EXECUTION-BOUNDARY";
        for (field, replacement) in [
            ("status", "complete"),
            ("admission_status", "admitted"),
            ("recorded_design_revision", "6.0"),
            (
                "completion_evidence_path",
                "docs/evidence/WP-600-http-consumer-property-read.toml",
            ),
        ] {
            let mut altered = index.clone();
            let tranche = altered["historical_tranche"]
                .as_array_of_tables_mut()
                .unwrap()
                .iter_mut()
                .find(|tranche| tranche["id"].as_str() == Some(id))
                .unwrap();
            tranche[field] = toml_edit::value(replacement);
            let error = super::check_work_packages_document(&root, &altered).unwrap_err();
            assert!(error.contains(field), "{field}: {error}");
        }
        let mut altered = index.clone();
        let tranche = altered["historical_tranche"]
            .as_array_of_tables_mut()
            .unwrap()
            .iter_mut()
            .find(|tranche| tranche["id"].as_str() == Some(id))
            .unwrap();
        tranche["completion_evidence_keys"] = toml_edit::value({
            let mut keys = toml_edit::Array::new();
            keys.push("different-evidence");
            keys
        });
        assert!(
            super::check_work_packages_document(&root, &altered)
                .unwrap_err()
                .contains("completion_evidence_keys")
        );
        let mut altered = index.clone();
        altered["historical_basis_commit"] = toml_edit::value("other-revision");
        assert!(
            super::check_work_packages_document(&root, &altered)
                .unwrap_err()
                .contains("historical_basis_commit")
        );
        let mut altered = index.clone();
        altered["historical_producer_gate_manifest"] =
            toml_edit::value("docs/work-packages/consumer-property-read-architecture-gate.toml");
        assert!(
            super::check_work_packages_document(&root, &altered)
                .unwrap_err()
                .contains("historical_producer_gate_manifest")
        );
    }

    #[test]
    fn historical_gates_reject_changed_status_revision_evidence_and_dependencies() {
        let root = repository_root().unwrap();
        for path in [
            "docs/work-packages/property-read-architecture-gate.toml",
            "docs/work-packages/consumer-property-read-architecture-gate.toml",
        ] {
            let gate = parse_toml(&root, path).unwrap();
            let pinned = super::pinned_document(&root, path).unwrap();
            super::check_historical_gate_lineage(path, &gate, &pinned).unwrap();
            for (field, replacement) in [("status", "ready"), ("design_revision", "6.0")] {
                let mut altered = gate.clone();
                altered[field] = toml_edit::value(replacement);
                let error =
                    super::check_historical_gate_lineage(path, &altered, &pinned).unwrap_err();
                assert!(error.contains(field), "{path}: {error}");
            }
            let mut altered = gate.clone();
            altered["evidence"] = toml_edit::value({
                let mut evidence = toml_edit::Array::new();
                evidence.push("unrelated-evidence");
                evidence
            });
            assert!(
                super::check_historical_gate_lineage(path, &altered, &pinned)
                    .unwrap_err()
                    .contains("evidence differs")
            );
            let mut altered = gate.clone();
            altered["dependency"]
                .as_array_of_tables_mut()
                .unwrap()
                .remove(0);
            assert!(
                super::check_historical_gate_lineage(path, &altered, &pinned)
                    .unwrap_err()
                    .contains("pinned gate dependencies")
            );
        }
    }

    #[test]
    fn historical_evidence_rejects_changed_status_revision_and_keys() {
        let root = repository_root().unwrap();
        for (path, changed_status) in [
            (
                "docs/evidence/WP-200-preparation-execution-boundary.toml",
                "passed",
            ),
            (
                "docs/evidence/WP-600-http-consumer-property-read.toml",
                "candidate",
            ),
        ] {
            let evidence = parse_toml(&root, path).unwrap();
            let pinned = super::pinned_document(&root, path).unwrap();
            super::check_historical_evidence_lineage(path, &evidence, &pinned).unwrap();
            for (field, replacement) in [("status", changed_status), ("design_revision", "6.0")] {
                let mut altered = evidence.clone();
                altered[field] = toml_edit::value(replacement);
                let error =
                    super::check_historical_evidence_lineage(path, &altered, &pinned).unwrap_err();
                assert!(error.contains(field), "{path}: {error}");
            }
            if evidence.get("evidence_keys").is_some() {
                let mut altered = evidence.clone();
                altered["evidence_keys"] = toml_edit::value({
                    let mut keys = toml_edit::Array::new();
                    keys.push("unreviewed-evidence");
                    keys
                });
                assert!(
                    super::check_historical_evidence_lineage(path, &altered, &pinned)
                        .unwrap_err()
                        .contains("evidence_keys")
                );
            }
        }
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
