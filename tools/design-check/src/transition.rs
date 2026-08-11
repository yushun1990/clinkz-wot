//! Generic validation for declarative candidate/review/admission/completion transitions.
//!
//! This module intentionally owns only transition data and topology. Package-specific
//! ownership, lifecycle, resource, provenance, and runtime behavior remain in their
//! focused validators and fixtures while those validators act as parallel oracles.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use toml_edit::{DocumentMut, Item, Table};

use super::{
    ACTIVE_AUTHORITY_REVISION, PROPERTY_READ_GATE_MANIFEST, array_field, check_candidate_commit,
    check_candidate_paths, check_git_commit_is_ancestor, check_property_read_handler_audit_state,
    check_scoped_review_attestation, git_changed_paths_between, git_commit_changed_paths,
    git_output_bytes, git_single_parent, git_text, load_artifact_registry, owned_set,
    package_string_set, parse_scoped_review_attestation, require_exact_table_fields,
    require_full_commit_id, require_git_ancestor, require_git_single_parent, require_integer,
    require_string, resolve_unattested_candidate_ref, string_field, string_set,
    validate_relative_path,
};

const CONFIG_TABLE: &str = "transition_validation";
const VALIDATOR_CHECK: &str = "transition-record-check";
const REVIEW_MARKER: &str = "register-after-review";
const CONVERGENCE_CLAIM: &str = "D48-TRANSITION-VALIDATOR-CONVERGENCE";

#[derive(Clone, Debug, Eq, PartialEq)]
struct GovernanceCheck {
    status: String,
    artifact: String,
    command: Vec<String>,
}

#[derive(Debug)]
struct CompletionProjection {
    implementation_ref: String,
}

pub(crate) fn check_repository(root: &Path) -> Result<(), String> {
    let path = root.join(PROPERTY_READ_GATE_MANIFEST);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let manifest = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    let registered_artifacts = load_artifact_registry(root)?;
    check_manifest(root, &manifest, &registered_artifacts)
}

pub(crate) fn check_manifest(
    root: &Path,
    manifest: &DocumentMut,
    registered_artifacts: &BTreeSet<String>,
) -> Result<(), String> {
    let context = "declarative transition validation";
    let config = manifest
        .get(CONFIG_TABLE)
        .and_then(Item::as_table)
        .ok_or_else(|| format!("property-read gate has no [{CONFIG_TABLE}] table"))?;
    require_exact_table_fields(
        config,
        context,
        &[
            "schema_version",
            "design_revision",
            "status",
            "check",
            "record_ids",
            "oracle_checks",
            "parity_dimensions",
            "review_attestation",
            "review_attestation_ref",
            "review_commit_paths",
            "record",
        ],
    )?;
    require_integer(
        config.get("schema_version"),
        "transition validation schema_version",
        1,
    )?;
    require_string(
        config.get("design_revision"),
        "transition validation design_revision",
        ACTIVE_AUTHORITY_REVISION,
    )?;
    require_string(
        config.get("check"),
        "transition validation check",
        VALIDATOR_CHECK,
    )?;

    let status = string_field(config, "status", context)?;
    if !matches!(status.as_str(), "review-pending" | "reviewed" | "passed") {
        return Err(format!(
            "transition validation has unsupported status {status:?}"
        ));
    }
    let expected_parity = owned_set(&[
        "valid-state",
        "negative-mutation",
        "commit-topology",
        "current-completion-evidence",
    ]);
    let parity = package_string_set(config, "parity_dimensions", context)?;
    if parity != expected_parity {
        return Err(format!(
            "transition validation parity mismatch; expected {expected_parity:?}, found {parity:?}"
        ));
    }

    let governance = load_governance_checks(root)?;
    let validator = executable_check(&governance, VALIDATOR_CHECK, context)?;
    if validator.artifact != "tools/design-check/Cargo.toml"
        || validator.command.last().map(String::as_str) != Some("check-transitions")
    {
        return Err(format!(
            "{VALIDATOR_CHECK} must execute tools/design-check check-transitions"
        ));
    }

    let records = config
        .get("record")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{context} has no [[transition_validation.record]] entries"))?;
    let declared_record_ids = package_string_set(config, "record_ids", context)?;
    let mut actual_record_ids = BTreeSet::new();
    let mut actual_oracles = BTreeSet::new();
    for record in records {
        let id = string_field(record, "id", "transition record")?;
        if !actual_record_ids.insert(id.clone()) {
            return Err(format!("duplicate transition record {id:?}"));
        }
        let oracles = check_record(
            root,
            manifest,
            record,
            &id,
            registered_artifacts,
            &governance,
        )?;
        actual_oracles.extend(oracles);
    }
    if actual_record_ids != declared_record_ids {
        return Err(format!(
            "transition record set mismatch; declared {declared_record_ids:?}, found {actual_record_ids:?}"
        ));
    }

    let declared_oracles = package_string_set(config, "oracle_checks", context)?;
    if declared_oracles != actual_oracles {
        return Err(format!(
            "transition oracle set mismatch; expected {actual_oracles:?}, found {declared_oracles:?}"
        ));
    }
    for oracle in &declared_oracles {
        executable_check(&governance, oracle, context)?;
    }

    let review_paths = package_string_set(config, "review_commit_paths", context)?;
    for path in &review_paths {
        validate_relative_path(path, context)?;
    }
    check_convergence_review(
        root,
        config,
        &status,
        &declared_record_ids,
        &declared_oracles,
        &parity,
        &review_paths,
        registered_artifacts,
    )?;
    Ok(())
}

fn check_record(
    root: &Path,
    manifest: &DocumentMut,
    record: &Table,
    id: &str,
    registered_artifacts: &BTreeSet<String>,
    governance: &BTreeMap<String, GovernanceCheck>,
) -> Result<BTreeSet<String>, String> {
    require_exact_table_fields(
        record,
        id,
        &[
            "id",
            "schema_version",
            "design_revision",
            "owner_kind",
            "owner_id",
            "candidate_ref_mode",
            "implementation_ref_source",
            "implementation_parent_topology",
            "review_commit_paths",
            "pre_source_paths",
            "support_implementation_paths",
            "unchanged_implementation_paths",
            "candidate",
            "source_boundary",
        ],
    )?;
    require_integer(
        record.get("schema_version"),
        &format!("{id} transition schema_version"),
        1,
    )?;
    let design_revision = string_field(record, "design_revision", id)?;
    if design_revision != ACTIVE_AUTHORITY_REVISION {
        return Err(format!(
            "{id} transition design revision must be {ACTIVE_AUTHORITY_REVISION:?}"
        ));
    }

    let owner_kind = string_field(record, "owner_kind", id)?;
    let owner_id = string_field(record, "owner_id", id)?;
    let owner = transition_owner(manifest, &owner_kind, &owner_id)?;
    let owner_status = string_field(owner, "status", &owner_id)?;
    let admission_status = string_field(owner, "admission_status", &owner_id)?;
    if !transition_status_pair_is_valid(&owner_status, &admission_status) {
        return Err(format!(
            "{id} owner has invalid status/admission pair {owner_status:?}/{admission_status:?}"
        ));
    }

    require_string(
        record.get("candidate_ref_mode"),
        &format!("{id} candidate_ref_mode"),
        "review-attestation-or-checkout",
    )?;
    require_string(
        record.get("implementation_ref_source"),
        &format!("{id} implementation_ref_source"),
        "completion-evidence",
    )?;

    let implementation_paths = package_string_set(owner, "implementation_paths", &owner_id)?;
    let support_paths = optional_string_set(record, "support_implementation_paths", id)?;
    for path in implementation_paths.iter().chain(&support_paths) {
        validate_relative_path(path, id)?;
    }
    let candidate_paths = package_string_set(owner, "candidate_paths", &owner_id)?;
    check_candidate_paths(
        &owner_id,
        root,
        &candidate_paths,
        &implementation_paths,
        registered_artifacts,
    )?;

    let prechecks = package_string_set(owner, "pre_implementation_checks", &owner_id)?;
    for check in &prechecks {
        executable_check(governance, check, id)?;
    }
    let precheck_refs: Vec<&str> = prechecks.iter().map(String::as_str).collect();

    let candidate_base_ref = string_field(owner, "candidate_base_ref", &owner_id)?;
    require_full_commit_id(&candidate_base_ref, &format!("{id} candidate_base_ref"))?;
    check_git_commit_is_ancestor(root, &candidate_base_ref, &format!("{id} candidate base"))?;
    let candidate_mode = string_field(owner, "candidate_ref", &owner_id)?;
    if candidate_mode != "resolved-by-review-attestation" {
        return Err(format!(
            "{id} owner candidate_ref must remain the review-attestation resolution mode"
        ));
    }

    let review_attestation = string_field(owner, "review_attestation", &owner_id)?;
    let effective_candidate_ref = resolve_effective_candidate(
        root,
        &owner_id,
        &candidate_base_ref,
        &review_attestation,
        &precheck_refs,
        &design_revision,
    )?;
    check_candidate_history(
        root,
        record,
        id,
        &candidate_base_ref,
        &effective_candidate_ref,
        &candidate_paths,
    )?;

    let review_commit_paths = package_string_set(record, "review_commit_paths", id)?;
    let review_attestation_ref = string_field(owner, "review_attestation_ref", &owner_id)?;
    if !root.join(&review_attestation).is_file()
        || !registered_artifacts.contains(&review_attestation)
    {
        return Err(format!(
            "{id} review attestation is not present and registered"
        ));
    }
    let mut required_review_paths = owned_set(&["docs/artifacts.csv"]);
    required_review_paths.insert(review_attestation.clone());
    if !required_review_paths.is_subset(&review_commit_paths) {
        return Err(format!(
            "{id} review paths must include {required_review_paths:?}"
        ));
    }
    let additional_review_paths: Vec<&str> = review_commit_paths
        .difference(&required_review_paths)
        .map(String::as_str)
        .collect();
    check_scoped_review_attestation(
        root,
        &review_attestation,
        &review_attestation_ref,
        &owner_id,
        &precheck_refs,
        &candidate_base_ref,
        &effective_candidate_ref,
        &candidate_paths,
        id,
        &design_revision,
        &additional_review_paths,
    )?;

    let admission_review = string_field(owner, "admission_review", &owner_id)?;
    if !root.join(&admission_review).is_file() || !registered_artifacts.contains(&admission_review)
    {
        return Err(format!(
            "{id} admission review is not present and registered"
        ));
    }
    check_property_read_handler_audit_state(root, &admission_review, &admission_status)?;

    let entry_check = string_field(owner, "entry_check", &owner_id)?;
    let completion_check = string_field(owner, "completion_check", &owner_id)?;
    executable_check(governance, &entry_check, id)?;
    let completion_registration = executable_check(governance, &completion_check, id)?;
    let mut oracles = BTreeSet::from([entry_check, completion_check.clone()]);

    let contract_artifacts = package_string_set(owner, "contract_artifacts", &owner_id)?;
    for artifact in &contract_artifacts {
        validate_relative_path(artifact, id)?;
        if !root.join(artifact).is_file() {
            return Err(format!("{id} contract artifact {artifact:?} is missing"));
        }
        let crate_registered = artifact == "tools/design-check/src/main.rs"
            && registered_artifacts.contains("tools/design-check/Cargo.toml");
        if !registered_artifacts.contains(artifact) && !crate_registered {
            return Err(format!(
                "{id} contract artifact {artifact:?} is not registered"
            ));
        }
    }

    let evidence_path = string_field(owner, "completion_evidence_path", &owner_id)?;
    if !root.join(&evidence_path).is_file() || !registered_artifacts.contains(&evidence_path) {
        return Err(format!(
            "{id} completion evidence is not present and registered"
        ));
    }
    let completion = check_completion_evidence(
        root,
        owner,
        &owner_id,
        &design_revision,
        &evidence_path,
        &completion_check,
        completion_registration,
    )?;

    let admission_base_ref = string_field(owner, "admission_base_ref", &owner_id)?;
    require_full_commit_id(&admission_base_ref, &format!("{id} admission_base_ref"))?;
    check_git_commit_is_ancestor(root, &admission_base_ref, &format!("{id} admission base"))?;
    require_git_ancestor(
        root,
        &review_attestation_ref,
        &admission_base_ref,
        &format!("{id} review/admission ancestry"),
    )?;

    let pre_source_paths = package_string_set(record, "pre_source_paths", id)?;
    let unchanged = optional_string_set(record, "unchanged_implementation_paths", id)?;
    let mut allowed_implementation_paths = implementation_paths.clone();
    allowed_implementation_paths.extend(support_paths);
    if !unchanged.is_subset(&allowed_implementation_paths) {
        return Err(format!(
            "{id} unchanged implementation paths are outside the registered implementation scope"
        ));
    }
    let expected_implementation_paths: BTreeSet<String> = allowed_implementation_paths
        .difference(&unchanged)
        .cloned()
        .collect();
    check_implementation_topology(
        root,
        id,
        record,
        &admission_base_ref,
        &completion.implementation_ref,
        &pre_source_paths,
        &expected_implementation_paths,
    )?;

    check_source_boundaries(
        root,
        record,
        id,
        &effective_candidate_ref,
        &completion.implementation_ref,
    )?;

    // Keep the return type extensible for a future record that declares more than
    // an entry/completion oracle pair without adding record-specific engine code.
    oracles.insert(completion_check);
    Ok(oracles)
}

fn transition_owner<'a>(
    manifest: &'a DocumentMut,
    owner_kind: &str,
    owner_id: &str,
) -> Result<&'a Table, String> {
    match owner_kind {
        "tranche" => manifest
            .get("tranche")
            .and_then(Item::as_array_of_tables)
            .ok_or_else(|| "property-read gate has no [[tranche]] records".to_owned())?
            .iter()
            .find(|table| table.get("id").and_then(Item::as_str) == Some(owner_id))
            .ok_or_else(|| format!("transition owner tranche {owner_id:?} is missing")),
        "gate" => {
            if manifest.get("id").and_then(Item::as_str) != Some(owner_id) {
                return Err(format!("transition owner gate {owner_id:?} is missing"));
            }
            Ok(manifest.as_table())
        }
        other => Err(format!(
            "transition owner {owner_id:?} has unsupported kind {other:?}"
        )),
    }
}

fn resolve_effective_candidate(
    root: &Path,
    owner_id: &str,
    candidate_base_ref: &str,
    review_attestation: &str,
    prechecks: &[&str],
    design_revision: &str,
) -> Result<String, String> {
    let path = root.join(review_attestation);
    if path.is_file() {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        return Ok(parse_scoped_review_attestation(
            &source,
            owner_id,
            prechecks,
            owner_id,
            design_revision,
        )?
        .reviewed_ref);
    }
    resolve_unattested_candidate_ref(root, candidate_base_ref, owner_id)
}

fn check_candidate_history(
    root: &Path,
    record: &Table,
    id: &str,
    candidate_base_ref: &str,
    effective_candidate_ref: &str,
    current_candidate_paths: &BTreeSet<String>,
) -> Result<(), String> {
    let candidates = record
        .get("candidate")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{id} has no [[transition_validation.record.candidate]] history"))?;
    if candidates.is_empty() {
        return Err(format!("{id} candidate history is empty"));
    }

    let mut labels = BTreeSet::new();
    let mut previous_ref: Option<String> = None;
    let mut last_parent = None;
    let mut last_paths = None;
    for (index, candidate) in candidates.iter().enumerate() {
        require_exact_table_fields(
            candidate,
            &format!("{id} candidate history entry"),
            &["label", "ref", "parent_ref", "paths"],
        )?;
        let label = string_field(candidate, "label", id)?;
        if !labels.insert(label.clone()) {
            return Err(format!("{id} candidate history duplicates label {label:?}"));
        }
        let declared_ref = string_field(candidate, "ref", &label)?;
        let candidate_ref = match declared_ref.as_str() {
            "current" if index + 1 == candidates.len() => effective_candidate_ref.to_owned(),
            "current" => {
                return Err(format!(
                    "{id} uses the current candidate marker before the final history entry"
                ));
            }
            _ => {
                require_full_commit_id(&declared_ref, &format!("{id} {label} ref"))?;
                declared_ref
            }
        };
        let parent_ref = string_field(candidate, "parent_ref", &label)?;
        require_full_commit_id(&parent_ref, &format!("{id} {label} parent_ref"))?;
        if let Some(previous_ref) = &previous_ref {
            if !declared_chain_parent_is_valid(previous_ref, &parent_ref) {
                return Err(format!(
                    "{id} candidate {label:?} parent {parent_ref:?} does not continue {previous_ref:?}"
                ));
            }
        }
        let paths = package_string_set(candidate, "paths", &label)?;
        check_candidate_commit(id, root, &parent_ref, &candidate_ref, &paths)?;
        previous_ref = Some(candidate_ref);
        last_parent = Some(parent_ref);
        last_paths = Some(paths);
    }

    if previous_ref.as_deref() != Some(effective_candidate_ref) {
        return Err(format!(
            "{id} candidate history does not terminate at reviewed candidate {effective_candidate_ref:?}"
        ));
    }
    if last_parent.as_deref() != Some(candidate_base_ref) {
        return Err(format!(
            "{id} current candidate parent does not match owner candidate_base_ref"
        ));
    }
    if last_paths.as_ref() != Some(current_candidate_paths) {
        return Err(format!(
            "{id} current candidate paths do not match the declarative history"
        ));
    }
    Ok(())
}

fn check_completion_evidence(
    root: &Path,
    owner: &Table,
    owner_id: &str,
    design_revision: &str,
    relative_path: &str,
    completion_check: &str,
    registration: &GovernanceCheck,
) -> Result<CompletionProjection, String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    require_exact_table_fields(
        document.as_table(),
        &format!("{owner_id} completion evidence"),
        &[
            "schema_version",
            "design_revision",
            "work_package",
            "tranche",
            "implementation_ref",
            "recorded_on",
            "verification_command",
            "verification_check",
            "status",
            "evidence_key",
            "requirement_ids",
            "compilation_cells",
            "coverage",
        ],
    )?;
    require_integer(
        document.get("schema_version"),
        &format!("{owner_id} completion evidence schema_version"),
        1,
    )?;
    require_string(
        document.get("design_revision"),
        &format!("{owner_id} completion evidence design_revision"),
        design_revision,
    )?;
    let work_package = string_field(owner, "work_package", owner_id)?;
    require_string(
        document.get("work_package"),
        &format!("{owner_id} completion evidence work_package"),
        &work_package,
    )?;
    require_string(
        document.get("tranche"),
        &format!("{owner_id} completion evidence tranche"),
        owner_id,
    )?;
    require_string(
        document.get("status"),
        &format!("{owner_id} completion evidence status"),
        "passed",
    )?;
    require_string(
        document.get("verification_check"),
        &format!("{owner_id} completion evidence verification_check"),
        completion_check,
    )?;
    let expected_command = registration.command.join(" ");
    require_string(
        document.get("verification_command"),
        &format!("{owner_id} completion evidence verification_command"),
        &expected_command,
    )?;

    let evidence_keys = package_string_set(owner, "completion_evidence_keys", owner_id)?;
    let evidence_key = document
        .get("evidence_key")
        .and_then(Item::as_str)
        .ok_or_else(|| format!("{relative_path} has no evidence_key"))?;
    if !evidence_keys.contains(evidence_key) {
        return Err(format!(
            "{owner_id} completion evidence key {evidence_key:?} is not owned by the tranche"
        ));
    }
    let requirements = package_string_set(owner, "requirements", owner_id)?;
    let evidence_requirements = package_string_set(
        document.as_table(),
        "requirement_ids",
        &format!("{owner_id} completion evidence"),
    )?;
    if evidence_requirements != requirements {
        return Err(format!(
            "{owner_id} completion evidence requirement mismatch"
        ));
    }
    let feature_cells = package_string_set(owner, "feature_cells", owner_id)?;
    let evidence_cells = package_string_set(
        document.as_table(),
        "compilation_cells",
        &format!("{owner_id} completion evidence"),
    )?;
    if evidence_cells != feature_cells {
        return Err(format!(
            "{owner_id} completion evidence compilation-cell mismatch"
        ));
    }
    let coverage = package_string_set(
        document.as_table(),
        "coverage",
        &format!("{owner_id} completion evidence"),
    )?;
    if coverage.is_empty() {
        return Err(format!("{owner_id} completion evidence has empty coverage"));
    }
    let recorded_on = document
        .get("recorded_on")
        .and_then(Item::as_str)
        .ok_or_else(|| format!("{relative_path} has no recorded_on"))?;
    if recorded_on.trim().is_empty() {
        return Err(format!("{relative_path} has empty recorded_on"));
    }

    let implementation_ref = document
        .get("implementation_ref")
        .and_then(Item::as_str)
        .ok_or_else(|| format!("{relative_path} has no implementation_ref"))?
        .to_owned();
    require_full_commit_id(
        &implementation_ref,
        &format!("{owner_id} implementation_ref"),
    )?;
    check_git_commit_is_ancestor(
        root,
        &implementation_ref,
        &format!("{owner_id} implementation"),
    )?;
    Ok(CompletionProjection { implementation_ref })
}

fn check_implementation_topology(
    root: &Path,
    id: &str,
    record: &Table,
    admission_base_ref: &str,
    implementation_ref: &str,
    expected_pre_source_paths: &BTreeSet<String>,
    expected_implementation_paths: &BTreeSet<String>,
) -> Result<(), String> {
    let topology = string_field(record, "implementation_parent_topology", id)?;
    let pre_source_ref = match topology.as_str() {
        "direct-pre-source" => {
            let pre_source_ref = git_single_parent(
                root,
                implementation_ref,
                &format!("{id} implementation commit"),
            )?;
            require_git_single_parent(
                root,
                &pre_source_ref,
                admission_base_ref,
                &format!("{id} pre-source checkpoint"),
            )?;
            pre_source_ref
        }
        "tree-equivalent-admission-merge" => {
            let admission_merge_ref = git_single_parent(
                root,
                implementation_ref,
                &format!("{id} implementation commit"),
            )?;
            let merge = git_text(
                root,
                &["rev-list", "--parents", "-n", "1", &admission_merge_ref],
                &format!("{id} admission merge"),
            )?;
            let fields: Vec<&str> = merge.split_whitespace().collect();
            if fields.len() != 3
                || fields[0] != admission_merge_ref
                || fields[1] != admission_base_ref
            {
                return Err(format!(
                    "{id} admission merge has invalid parent topology {fields:?}"
                ));
            }
            let pre_source_ref = fields[2].to_owned();
            require_git_single_parent(
                root,
                &pre_source_ref,
                admission_base_ref,
                &format!("{id} pre-source checkpoint"),
            )?;
            let wrapper_diff = git_changed_paths_between(
                root,
                &pre_source_ref,
                &admission_merge_ref,
                &format!("{id} admission merge tree"),
            )?;
            if !wrapper_diff.is_empty() {
                return Err(format!(
                    "{id} admission merge tree differs from pre-source checkpoint: {wrapper_diff:?}"
                ));
            }
            pre_source_ref
        }
        other => {
            return Err(format!(
                "{id} has unsupported implementation_parent_topology {other:?}"
            ));
        }
    };

    let pre_source_paths = git_commit_changed_paths(
        root,
        &pre_source_ref,
        &format!("{id} pre-source checkpoint"),
    )?;
    if &pre_source_paths != expected_pre_source_paths {
        return Err(format!(
            "{id} pre-source paths mismatch; expected {expected_pre_source_paths:?}, found {pre_source_paths:?}"
        ));
    }
    let implementation_paths = git_commit_changed_paths(
        root,
        implementation_ref,
        &format!("{id} implementation commit"),
    )?;
    if &implementation_paths != expected_implementation_paths {
        return Err(format!(
            "{id} implementation paths mismatch; expected {expected_implementation_paths:?}, found {implementation_paths:?}"
        ));
    }
    Ok(())
}

fn check_source_boundaries(
    root: &Path,
    record: &Table,
    id: &str,
    candidate_ref: &str,
    implementation_ref: &str,
) -> Result<(), String> {
    let boundaries = record
        .get("source_boundary")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{id} has no source-boundary records"))?;
    if boundaries.is_empty() {
        return Err(format!("{id} source-boundary set is empty"));
    }
    let mut paths = BTreeSet::new();
    for boundary in boundaries {
        require_exact_table_fields(
            boundary,
            &format!("{id} source boundary"),
            &[
                "path",
                "kind",
                "candidate_expectation",
                "implementation_expectation",
                "markers",
            ],
        )?;
        let path = string_field(boundary, "path", id)?;
        validate_relative_path(&path, id)?;
        if !paths.insert(path.clone()) {
            return Err(format!("{id} duplicates source boundary {path:?}"));
        }
        let kind = string_field(boundary, "kind", &path)?;
        let candidate_expectation = string_field(boundary, "candidate_expectation", &path)?;
        let implementation_expectation =
            string_field(boundary, "implementation_expectation", &path)?;
        let markers = optional_string_set(boundary, "markers", &path)?;
        let candidate = git_file_at_ref(root, candidate_ref, &path)?;
        if !boundary_matches(
            candidate.as_deref(),
            &kind,
            &candidate_expectation,
            &markers,
        )? {
            return Err(format!(
                "{id} candidate boundary {path:?} does not satisfy {candidate_expectation:?}"
            ));
        }
        let implementation = git_file_at_ref(root, implementation_ref, &path)?;
        if !boundary_matches(
            implementation.as_deref(),
            &kind,
            &implementation_expectation,
            &markers,
        )? {
            return Err(format!(
                "{id} implementation boundary {path:?} does not satisfy {implementation_expectation:?}"
            ));
        }
    }
    Ok(())
}

fn git_file_at_ref(root: &Path, reference: &str, path: &str) -> Result<Option<Vec<u8>>, String> {
    let object = format!("{reference}:{path}");
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", &object])
        .output()
        .map_err(|error| format!("cannot inspect {object:?}: {error}"))?;
    if output.status.success() {
        return Ok(Some(output.stdout));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("does not exist in") || stderr.contains("exists on disk, but not in") {
        return Ok(None);
    }
    Err(format!("cannot inspect {object:?}: {stderr}"))
}

fn boundary_matches(
    source: Option<&[u8]>,
    kind: &str,
    expectation: &str,
    markers: &BTreeSet<String>,
) -> Result<bool, String> {
    match kind {
        "file" => {
            if !markers.is_empty() {
                return Err("file source boundary must not declare markers".to_owned());
            }
            match expectation {
                "absent" => Ok(source.is_none()),
                "present" => Ok(source.is_some()),
                other => Err(format!("unsupported file-boundary expectation {other:?}")),
            }
        }
        "text" => {
            if markers.is_empty() {
                return Err("text source boundary must declare markers".to_owned());
            }
            let Some(source) = source else {
                return Ok(expectation == "none");
            };
            let source = std::str::from_utf8(source)
                .map_err(|error| format!("source boundary is not UTF-8: {error}"))?;
            let present = markers
                .iter()
                .filter(|marker| source.contains(*marker))
                .count();
            match expectation {
                "none" => Ok(present == 0),
                "all" => Ok(present == markers.len()),
                "not-all" => Ok(present < markers.len()),
                other => Err(format!("unsupported text-boundary expectation {other:?}")),
            }
        }
        other => Err(format!("unsupported source-boundary kind {other:?}")),
    }
}

#[allow(clippy::too_many_arguments)]
fn check_convergence_review(
    root: &Path,
    config: &Table,
    status: &str,
    record_ids: &BTreeSet<String>,
    oracle_checks: &BTreeSet<String>,
    parity_dimensions: &BTreeSet<String>,
    review_commit_paths: &BTreeSet<String>,
    registered_artifacts: &BTreeSet<String>,
) -> Result<(), String> {
    let relative_path = string_field(config, "review_attestation", CONFIG_TABLE)?;
    let attestation_ref = string_field(config, "review_attestation_ref", CONFIG_TABLE)?;
    if status == "review-pending" {
        if relative_path != REVIEW_MARKER || attestation_ref != REVIEW_MARKER {
            return Err(format!(
                "review-pending transition validation must keep both review markers unresolved"
            ));
        }
        return Ok(());
    }
    if relative_path == REVIEW_MARKER || !root.join(&relative_path).is_file() {
        return Err("reviewed transition validation lacks its review attestation".to_owned());
    }
    if !registered_artifacts.contains(&relative_path) {
        return Err(format!(
            "transition validation review {relative_path:?} is not registered"
        ));
    }
    let source = fs::read_to_string(root.join(&relative_path))
        .map_err(|error| format!("cannot read {relative_path}: {error}"))?;
    let review = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid transition validation review: {error}"))?;
    require_exact_table_fields(
        review.as_table(),
        "transition validation review",
        &[
            "schema_version",
            "design_revision",
            "claim",
            "status",
            "reviewer_attestation_kind",
            "reviewer_id",
            "reviewed_ref",
            "validator_check",
            "record_ids",
            "oracle_checks",
            "parity_dimensions",
            "result",
            "case",
        ],
    )?;
    require_integer(
        review.get("schema_version"),
        "transition validation review schema_version",
        1,
    )?;
    require_string(
        review.get("design_revision"),
        "transition validation review design_revision",
        ACTIVE_AUTHORITY_REVISION,
    )?;
    require_string(
        review.get("claim"),
        "transition validation review claim",
        CONVERGENCE_CLAIM,
    )?;
    require_string(
        review.get("status"),
        "transition validation review status",
        "passed",
    )?;
    require_string(
        review.get("validator_check"),
        "transition validation review validator_check",
        VALIDATOR_CHECK,
    )?;
    require_string(
        review.get("result"),
        "transition validation review result",
        "equivalent-with-parallel-oracles-retained",
    )?;
    let reviewer_kind = review
        .get("reviewer_attestation_kind")
        .and_then(Item::as_str)
        .ok_or_else(|| "transition validation review has no reviewer kind".to_owned())?;
    let reviewer_id = review
        .get("reviewer_id")
        .and_then(Item::as_str)
        .ok_or_else(|| "transition validation review has no reviewer id".to_owned())?;
    match reviewer_kind {
        "independent-root-session" if reviewer_id == "codex-agent:/root" => {}
        "separate-agent-task"
            if reviewer_id.starts_with("codex-agent:/root/")
                && reviewer_id != "codex-agent:/root/" => {}
        _ => {
            return Err(format!(
                "transition validation review has invalid reviewer {reviewer_kind:?}/{reviewer_id:?}"
            ));
        }
    }
    let reviewed_ref = review
        .get("reviewed_ref")
        .and_then(Item::as_str)
        .ok_or_else(|| "transition validation review has no reviewed_ref".to_owned())?;
    require_full_commit_id(reviewed_ref, "transition validation reviewed_ref")?;
    check_git_commit_is_ancestor(root, reviewed_ref, "transition validation reviewed_ref")?;
    for (field, expected) in [
        ("record_ids", record_ids),
        ("oracle_checks", oracle_checks),
        ("parity_dimensions", parity_dimensions),
    ] {
        let actual = package_string_set(review.as_table(), field, "transition validation review")?;
        if &actual != expected {
            return Err(format!(
                "transition validation review {field} mismatch; expected {expected:?}, found {actual:?}"
            ));
        }
    }
    check_review_cases(
        review.as_table(),
        record_ids,
        oracle_checks,
        parity_dimensions,
    )?;

    if status == "reviewed" {
        if attestation_ref != REVIEW_MARKER {
            return Err(
                "reviewed transition validation must defer its review commit ref".to_owned(),
            );
        }
        return Ok(());
    }
    require_full_commit_id(
        &attestation_ref,
        "transition validation review_attestation_ref",
    )?;
    check_git_commit_is_ancestor(
        root,
        &attestation_ref,
        "transition validation review_attestation_ref",
    )?;
    require_git_single_parent(
        root,
        &attestation_ref,
        reviewed_ref,
        "transition validation review commit",
    )?;
    let actual_review_paths = git_commit_changed_paths(
        root,
        &attestation_ref,
        "transition validation review commit",
    )?;
    if &actual_review_paths != review_commit_paths {
        return Err(format!(
            "transition validation review paths mismatch; expected {review_commit_paths:?}, found {actual_review_paths:?}"
        ));
    }
    let object = format!("{attestation_ref}:{relative_path}");
    let committed_source = git_output_bytes(
        root,
        &["show", &object],
        "read transition validation review",
    )?;
    if committed_source != source.as_bytes() {
        return Err(
            "worktree transition validation review differs from its review commit".to_owned(),
        );
    }
    Ok(())
}

fn check_review_cases(
    review: &Table,
    record_ids: &BTreeSet<String>,
    oracle_checks: &BTreeSet<String>,
    parity_dimensions: &BTreeSet<String>,
) -> Result<(), String> {
    let cases = review
        .get("case")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "transition validation review has no [[case]] evidence".to_owned())?;
    let mut case_ids = BTreeSet::new();
    let mut covered_dimensions = BTreeSet::new();
    let mut valid_records = BTreeSet::new();
    for case in cases {
        require_exact_table_fields(
            case,
            "transition validation review case",
            &[
                "id",
                "dimension",
                "target_record",
                "mutation",
                "generic_result",
                "oracle_result",
                "oracle_checks",
            ],
        )?;
        let id = string_field(case, "id", "transition validation review case")?;
        if !case_ids.insert(id.clone()) {
            return Err(format!(
                "transition validation review duplicates case {id:?}"
            ));
        }
        let dimension = string_field(case, "dimension", &id)?;
        if !parity_dimensions.contains(&dimension) {
            return Err(format!(
                "transition validation review case {id:?} has unknown dimension {dimension:?}"
            ));
        }
        covered_dimensions.insert(dimension.clone());
        let target = string_field(case, "target_record", &id)?;
        if !record_ids.contains(&target) {
            return Err(format!(
                "transition validation review case {id:?} has unknown target {target:?}"
            ));
        }
        let mutation = string_field(case, "mutation", &id)?;
        let generic_result = string_field(case, "generic_result", &id)?;
        let oracle_result = string_field(case, "oracle_result", &id)?;
        if dimension == "valid-state" {
            if mutation != "none" || generic_result != "passed" || oracle_result != "passed" {
                return Err(format!(
                    "transition validation valid-state case {id:?} must record an unmutated parallel pass"
                ));
            }
            valid_records.insert(target);
        } else if mutation.trim().is_empty()
            || mutation == "none"
            || generic_result != "rejected"
            || oracle_result != "rejected"
        {
            return Err(format!(
                "transition validation mutation case {id:?} must record parallel rejection"
            ));
        }
        let case_oracles = package_string_set(case, "oracle_checks", &id)?;
        if case_oracles.is_empty() || !case_oracles.is_subset(oracle_checks) {
            return Err(format!(
                "transition validation review case {id:?} has invalid oracle coverage {case_oracles:?}"
            ));
        }
    }
    if &covered_dimensions != parity_dimensions {
        return Err(format!(
            "transition validation review case dimensions mismatch; expected {parity_dimensions:?}, found {covered_dimensions:?}"
        ));
    }
    if &valid_records != record_ids {
        return Err(format!(
            "transition validation valid-state records mismatch; expected {record_ids:?}, found {valid_records:?}"
        ));
    }
    Ok(())
}

fn load_governance_checks(root: &Path) -> Result<BTreeMap<String, GovernanceCheck>, String> {
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
    let mut result = BTreeMap::new();
    for check in checks {
        let id = string_field(check, "id", "governance check")?;
        let status = string_field(check, "status", &id)?;
        let artifact = string_field(check, "artifact", &id)?;
        let command = array_field(check, "command", &id)?
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("governance check {id:?} has a non-string command"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if command.is_empty() {
            return Err(format!("governance check {id:?} has an empty command"));
        }
        if result
            .insert(
                id.clone(),
                GovernanceCheck {
                    status,
                    artifact,
                    command,
                },
            )
            .is_some()
        {
            return Err(format!("duplicate governance check {id:?}"));
        }
    }
    Ok(result)
}

fn executable_check<'a>(
    governance: &'a BTreeMap<String, GovernanceCheck>,
    id: &str,
    context: &str,
) -> Result<&'a GovernanceCheck, String> {
    let check = governance
        .get(id)
        .ok_or_else(|| format!("{context} references unknown governance check {id:?}"))?;
    if check.status != "executable" {
        return Err(format!("{context} check {id:?} is not executable"));
    }
    Ok(check)
}

fn optional_string_set(
    table: &Table,
    field: &str,
    context: &str,
) -> Result<BTreeSet<String>, String> {
    match table.get(field) {
        Some(item) => item
            .as_array()
            .ok_or_else(|| format!("{context} field {field:?} is not an array"))
            .and_then(|array| string_set(array, context, field)),
        None => Ok(BTreeSet::new()),
    }
}

fn transition_status_pair_is_valid(status: &str, admission_status: &str) -> bool {
    matches!(
        (status, admission_status),
        ("pending", "review-pending")
            | ("pending", "approved")
            | ("in-progress", "approved")
            | ("complete", "approved")
    )
}

fn declared_chain_parent_is_valid(previous_ref: &str, parent_ref: &str) -> bool {
    previous_ref == parent_ref
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use toml_edit::DocumentMut;

    use super::{
        boundary_matches, check_review_cases, declared_chain_parent_is_valid,
        transition_status_pair_is_valid,
    };

    const REVIEW_CASES: &str = r#"
[[case]]
id = "route-valid"
dimension = "valid-state"
target_record = "route"
mutation = "none"
generic_result = "passed"
oracle_result = "passed"
oracle_checks = ["route-entry", "route-completion"]

[[case]]
id = "servient-valid"
dimension = "valid-state"
target_record = "servient"
mutation = "none"
generic_result = "passed"
oracle_result = "passed"
oracle_checks = ["servient-entry", "servient-completion"]

[[case]]
id = "negative"
dimension = "negative-mutation"
target_record = "route"
mutation = "remove one candidate path"
generic_result = "rejected"
oracle_result = "rejected"
oracle_checks = ["route-entry"]

[[case]]
id = "topology"
dimension = "commit-topology"
target_record = "servient"
mutation = "replace the admission base"
generic_result = "rejected"
oracle_result = "rejected"
oracle_checks = ["servient-completion"]

[[case]]
id = "evidence"
dimension = "current-completion-evidence"
target_record = "route"
mutation = "replace the completion evidence key"
generic_result = "rejected"
oracle_result = "rejected"
oracle_checks = ["route-completion"]
"#;

    #[test]
    fn transition_status_pairs_are_closed() {
        for (status, admission, valid) in [
            ("pending", "review-pending", true),
            ("pending", "approved", true),
            ("in-progress", "approved", true),
            ("complete", "approved", true),
            ("complete", "review-pending", false),
            ("revoked", "approved", false),
        ] {
            assert_eq!(
                transition_status_pair_is_valid(status, admission),
                valid,
                "unexpected result for {status}/{admission}"
            );
        }
    }

    #[test]
    fn text_boundaries_distinguish_none_not_all_and_all() {
        let markers = BTreeSet::from(["alpha".to_owned(), "beta".to_owned()]);
        assert_eq!(
            boundary_matches(Some(b"plain"), "text", "none", &markers),
            Ok(true)
        );
        assert_eq!(
            boundary_matches(Some(b"alpha"), "text", "not-all", &markers),
            Ok(true)
        );
        assert_eq!(
            boundary_matches(Some(b"alpha beta"), "text", "all", &markers),
            Ok(true)
        );
        assert_eq!(
            boundary_matches(Some(b"alpha"), "text", "all", &markers),
            Ok(false)
        );
    }

    #[test]
    fn file_boundaries_require_exact_presence() {
        let markers = BTreeSet::new();
        assert_eq!(boundary_matches(None, "file", "absent", &markers), Ok(true));
        assert_eq!(
            boundary_matches(Some(b"source"), "file", "present", &markers),
            Ok(true)
        );
    }

    #[test]
    fn candidate_history_must_be_contiguous() {
        assert!(declared_chain_parent_is_valid("candidate-a", "candidate-a"));
        assert!(!declared_chain_parent_is_valid(
            "candidate-a",
            "unrelated-base"
        ));
    }

    #[test]
    fn review_cases_cover_every_record_and_parity_dimension() {
        let review = REVIEW_CASES.parse::<DocumentMut>().unwrap();
        let records = BTreeSet::from(["route".to_owned(), "servient".to_owned()]);
        let oracles = BTreeSet::from([
            "route-entry".to_owned(),
            "route-completion".to_owned(),
            "servient-entry".to_owned(),
            "servient-completion".to_owned(),
        ]);
        let dimensions = BTreeSet::from([
            "valid-state".to_owned(),
            "negative-mutation".to_owned(),
            "commit-topology".to_owned(),
            "current-completion-evidence".to_owned(),
        ]);
        assert_eq!(
            check_review_cases(review.as_table(), &records, &oracles, &dimensions),
            Ok(())
        );

        let invalid = REVIEW_CASES
            .replacen(
                "generic_result = \"rejected\"",
                "generic_result = \"passed\"",
                1,
            )
            .parse::<DocumentMut>()
            .unwrap();
        assert!(check_review_cases(invalid.as_table(), &records, &oracles, &dimensions).is_err());
    }
}
