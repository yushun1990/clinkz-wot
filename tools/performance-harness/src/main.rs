//! Deterministic fixture generator and performance manifest orchestrator.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use toml_edit::{DocumentMut, Item, Table, value};

const GENERATOR: &str = "clinkz-wot-fixture-generator-v3";
const MANIFESTS: &[&str] = &[
    "docs/performance/gateway.toml",
    "docs/performance/directory.toml",
    "docs/performance/constrained.toml",
];
const RECIPE_KEYS: &[&str] = &[
    "actors",
    "binding_artifacts",
    "bindings",
    "call_bytes",
    "collection_sources",
    "document_bytes",
    "driver_bytes",
    "extension_bytes",
    "forms",
    "handler_slots",
    "ingress_bytes",
    "ingress_items",
    "page_entries",
    "page_item_bytes",
    "payload_bytes",
    "plan_sets",
    "readiness_tokens",
    "routes",
    "schema_nodes",
    "security_branches",
    "slot_state_bytes",
    "string_bytes",
    "subscribers",
    "td_nodes",
    "uri_template_bytes",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GateComparator {
    AtLeast,
    AtMost,
    ExactlyOne,
}

#[derive(Clone, Debug)]
struct GateSpec {
    source_field: String,
    metric: String,
    comparator: GateComparator,
    threshold: f64,
}

#[derive(Clone, Debug)]
struct CoverageSpec {
    cell_count: u64,
    sha256: String,
}

#[derive(Clone, Debug)]
struct RunnerFingerprint {
    values: BTreeMap<String, String>,
    sha256: String,
}

#[derive(Clone, Debug)]
struct Fixture {
    id: String,
    profile: String,
    harness_case: String,
    version: u64,
    seed: u64,
    recipe: String,
    content_sha256: String,
    forms_per_context_max: usize,
}

#[derive(Clone, Debug)]
struct FixtureProfileLimits {
    values: BTreeMap<String, Option<u64>>,
}

#[derive(Clone, Debug)]
struct Case {
    manifest_path: PathBuf,
    manifest_sha256: String,
    profile: String,
    resource_profile: String,
    feature_set: String,
    target: String,
    toolchain: String,
    allocator: String,
    runner: String,
    runner_fingerprint: RunnerFingerprint,
    id: String,
    version: u64,
    fixture_id: String,
    harness_case: String,
    measurement_sha256: String,
    gating: bool,
    expected_sample_count: u64,
    coverage: Option<CoverageSpec>,
    activation_trace_oracle: Option<String>,
    activation_cases: Option<Vec<String>>,
    gates: Vec<GateSpec>,
    report_metrics: BTreeSet<String>,
}

#[derive(Debug)]
struct Manifest {
    path: PathBuf,
    fixture_digest: String,
    cases: Vec<Case>,
}

impl Case {
    fn total_sample_count(&self) -> Result<u64, String> {
        self.expected_sample_count
            .checked_mul(
                self.coverage
                    .as_ref()
                    .map_or(1, |coverage| coverage.cell_count),
            )
            .ok_or_else(|| format!("case {:?} total sample count overflows", self.id))
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("performance harness: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = repository_root()?;
    let mut arguments = env::args().skip(1);
    match arguments.next().as_deref().unwrap_or("verify") {
        "verify" => {
            reject_extra(arguments)?;
            let (fixtures, manifests) = load_contract(&root, false)?;
            verify_digests(&fixtures, &manifests)?;
            println!(
                "performance harness: {} fixtures and {} cases verified",
                fixtures.len(),
                manifests
                    .iter()
                    .map(|manifest| manifest.cases.len())
                    .sum::<usize>()
            );
        }
        "list" => {
            reject_extra(arguments)?;
            let (fixtures, manifests) = load_contract(&root, false)?;
            verify_digests(&fixtures, &manifests)?;
            for case in sorted_cases(&manifests) {
                let fixture = fixtures
                    .get(&case.fixture_id)
                    .ok_or_else(|| format!("unknown fixture {:?}", case.fixture_id))?;
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    case.id,
                    case.version,
                    case.profile,
                    case.resource_profile,
                    case.fixture_id,
                    fixture.content_sha256,
                    case.harness_case,
                    case.measurement_sha256
                );
            }
        }
        "digest-lines" => {
            reject_extra(arguments)?;
            let (fixtures, manifests) = load_contract(&root, true)?;
            for fixture in fixtures.values() {
                println!(
                    "fixture\t{}\tsha256:{}",
                    fixture.id,
                    fixture_digest(fixture)?
                );
            }
            for manifest in &manifests {
                println!(
                    "manifest\t{}\tsha256:{}",
                    display_path(&root, &manifest.path),
                    manifest_fixture_digest(manifest, &fixtures)?
                );
            }
        }
        "refresh-lock" => {
            reject_extra(arguments)?;
            refresh_digests(&root)?;
            let (fixtures, manifests) = load_contract(&root, false)?;
            verify_digests(&fixtures, &manifests)?;
            println!(
                "performance harness: refreshed {} fixtures and {} manifest digests",
                fixtures.len(),
                manifests.len(),
            );
        }
        "fixture" => {
            let id = required_argument(&mut arguments, "fixture id")?;
            reject_extra(arguments)?;
            let (fixtures, _) = load_contract(&root, true)?;
            let fixture = fixtures
                .get(&id)
                .ok_or_else(|| format!("unknown fixture {id:?}"))?;
            io::stdout()
                .write_all(&generate_fixture(fixture)?)
                .map_err(|error| format!("cannot write fixture: {error}"))?;
        }
        "run" => {
            let workload_id = required_argument(&mut arguments, "workload id")?;
            let adapter = PathBuf::from(required_argument(&mut arguments, "adapter")?);
            reject_extra(arguments)?;
            run_adapter(&root, &workload_id, &adapter)?;
        }
        command => {
            return Err(format!(
                "unknown command {command:?}; expected verify, list, digest-lines, refresh-lock, fixture, or run"
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

fn load_contract(
    root: &Path,
    allow_pending_digests: bool,
) -> Result<(BTreeMap<String, Fixture>, Vec<Manifest>), String> {
    validate_json_schema(
        root,
        "docs/performance/manifest.schema.json",
        "performance-manifest-v5",
    )?;
    validate_json_schema(
        root,
        "docs/performance/result.schema.json",
        "performance-result-v2",
    )?;
    let requirements = load_requirements(root)?;
    let profile_limits = load_fixture_profile_limits(root)?;
    let fixtures = load_fixtures(root, &profile_limits, allow_pending_digests)?;
    let mut case_ids = BTreeSet::new();
    let mut fixture_uses = BTreeSet::new();
    let mut manifests = Vec::new();
    for relative_path in MANIFESTS {
        manifests.push(load_manifest(
            root,
            relative_path,
            &requirements,
            &fixtures,
            &mut case_ids,
            &mut fixture_uses,
            allow_pending_digests,
        )?);
    }
    let locked: BTreeSet<&str> = fixtures.keys().map(String::as_str).collect();
    let used: BTreeSet<&str> = fixture_uses.iter().map(String::as_str).collect();
    if locked != used {
        let unused: Vec<_> = locked.difference(&used).copied().collect();
        let missing: Vec<_> = used.difference(&locked).copied().collect();
        return Err(format!(
            "fixture lock/reference mismatch; unused={unused:?}, missing={missing:?}"
        ));
    }
    validate_serving_activation_contract(&manifests)?;
    Ok((fixtures, manifests))
}

fn validate_serving_activation_contract(manifests: &[Manifest]) -> Result<(), String> {
    let find_case = |id: &str| {
        manifests
            .iter()
            .flat_map(|manifest| &manifest.cases)
            .find(|case| case.id == id)
            .ok_or_else(|| format!("missing serving activation case {id}"))
    };
    let gateway = find_case("PERF-GW-030")?;
    let constrained = find_case("PERF-CS-022")?;
    for case in [gateway, constrained] {
        if case.version != 2 {
            return Err(format!(
                "{} must use serving activation workload version 2",
                case.id
            ));
        }
        if case.activation_trace_oracle.as_deref() != Some("serving-activation-v1") {
            return Err(format!(
                "{} does not select the serving-activation-v1 trace oracle",
                case.id
            ));
        }
        let gate_fields: BTreeSet<&str> = case
            .gates
            .iter()
            .map(|gate| gate.source_field.as_str())
            .collect();
        for field in [
            "activation_authorities_max",
            "duplicate_concurrent_accept_claims_max",
            "lost_committed_route_guards_max",
            "nth_commit_failure_publications_max",
            "partial_route_admissions_max",
            "post_drain_accept_claims_max",
            "prepublication_requests_admitted_max",
            "permits_without_accept_claim_max",
            "require_atomic_activation_publication",
            "require_exclusive_accept_lease_borrow",
            "require_exactly_one_activation_authority",
            "require_borrowed_permit_nonretention",
            "require_closed_ingress_policy_coverage",
            "require_serving_activation_trace_oracle",
            "retained_permit_bytes_max",
            "stale_permits_mutating_state_max",
        ] {
            if !gate_fields.contains(field) {
                return Err(format!("{} is missing activation gate {field}", case.id));
            }
        }
    }
    if gateway.activation_cases != constrained.activation_cases {
        return Err(
            "Gateway and constrained activation cases do not share one ordered trace oracle"
                .to_owned(),
        );
    }
    let gateway_coverage = gateway
        .coverage
        .as_ref()
        .ok_or_else(|| "PERF-GW-030 has no activation coverage matrix".to_owned())?;
    let constrained_coverage = constrained
        .coverage
        .as_ref()
        .ok_or_else(|| "PERF-CS-022 has no activation coverage matrix".to_owned())?;
    if gateway_coverage.cell_count != constrained_coverage.cell_count
        || gateway_coverage.sha256 != constrained_coverage.sha256
    {
        return Err(
            "Gateway and constrained activation coverage matrices are not identical".to_owned(),
        );
    }
    Ok(())
}

fn validate_json_schema(root: &Path, relative_path: &str, identity: &str) -> Result<(), String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let schema: JsonValue = serde_json::from_str(&source)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    let schema_id = schema
        .get("$id")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| format!("{} has no $id", path.display()))?;
    if !schema_id.contains(identity) {
        return Err(format!(
            "{} has unexpected $id {schema_id:?}",
            path.display()
        ));
    }
    Ok(())
}

fn load_requirements(root: &Path) -> Result<BTreeSet<String>, String> {
    let path = root.join("docs/requirements.csv");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = source.lines();
    let header = lines
        .next()
        .ok_or_else(|| "requirements registry is empty".to_owned())?;
    if !header.starts_with("requirement,") {
        return Err("requirements registry has an unexpected header".to_owned());
    }
    let mut requirements = BTreeSet::new();
    for line in lines.filter(|line| !line.trim().is_empty()) {
        let expression = line
            .split(',')
            .next()
            .ok_or_else(|| "requirements registry has an empty row".to_owned())?;
        for component in expression.split('|') {
            for id in expand_requirement(component)? {
                if !requirements.insert(id.clone()) {
                    return Err(format!("duplicate requirement {id:?}"));
                }
            }
        }
    }
    Ok(requirements)
}

fn expand_requirement(expression: &str) -> Result<Vec<String>, String> {
    let Some((first, last)) = expression.split_once("..") else {
        return Ok(vec![expression.to_owned()]);
    };
    if first.len() < 4 || last.len() != 3 {
        return Err(format!("invalid requirement range {expression:?}"));
    }
    let (prefix, first_number) = first.split_at(first.len() - 3);
    let first_number = first_number
        .parse::<u16>()
        .map_err(|error| format!("invalid requirement range {expression:?}: {error}"))?;
    let last_number = last
        .parse::<u16>()
        .map_err(|error| format!("invalid requirement range {expression:?}: {error}"))?;
    if first_number > last_number {
        return Err(format!("descending requirement range {expression:?}"));
    }
    Ok((first_number..=last_number)
        .map(|number| format!("{prefix}{number:03}"))
        .collect())
}

fn load_fixture_profile_limits(
    root: &Path,
) -> Result<BTreeMap<String, FixtureProfileLimits>, String> {
    let path = root.join("docs/resource-limits.csv");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = source.lines();
    let header = lines
        .next()
        .ok_or_else(|| "resource limit schema is empty".to_owned())?;
    if header.split(',').collect::<Vec<_>>().len() != 10 {
        return Err("resource limit schema has an unexpected header".to_owned());
    }
    let profiles = [
        ("GatewayDefaultV1", 6_usize),
        ("DirectoryClientDefaultV1", 7_usize),
        ("BenchmarkStaticReferenceV1", 8_usize),
    ];
    let mut limits: BTreeMap<String, FixtureProfileLimits> = profiles
        .iter()
        .map(|(profile, _)| {
            (
                (*profile).to_owned(),
                FixtureProfileLimits {
                    values: BTreeMap::new(),
                },
            )
        })
        .collect();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let columns: Vec<&str> = line.split(',').collect();
        if columns.len() != 10 {
            return Err(format!(
                "resource limit schema line {} has {} columns",
                offset + 2,
                columns.len()
            ));
        }
        for (profile, column) in profiles {
            let value = match columns[column] {
                "NA" => None,
                value => Some(value.parse::<u64>().map_err(|error| {
                    format!(
                        "invalid resource limit for {profile} field {:?}: {error}",
                        columns[0]
                    )
                })?),
            };
            limits
                .get_mut(profile)
                .expect("the profile map was initialized")
                .values
                .insert(columns[0].to_owned(), value);
        }
    }
    Ok(limits)
}

fn load_fixtures(
    root: &Path,
    profile_limits: &BTreeMap<String, FixtureProfileLimits>,
    allow_pending_digests: bool,
) -> Result<BTreeMap<String, Fixture>, String> {
    let path = root.join("docs/performance/fixtures.lock.toml");
    let document = parse_toml(&path)?;
    require_integer(document.get("schema_version"), "fixture schema_version", 1)?;
    require_string(
        document.get("design_revision"),
        "fixture design_revision",
        "4.9",
    )?;
    require_string(document.get("generator"), "fixture generator", GENERATOR)?;
    let tables = document
        .get("fixture")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "fixture lock has no [[fixture]] entries".to_owned())?;
    let mut fixtures = BTreeMap::new();
    for table in tables {
        let id = string_field(table, "id", "fixture")?;
        let profile = string_field(table, "profile", &id)?;
        let limits = profile_limits
            .get(&profile)
            .ok_or_else(|| format!("fixture {id:?} has unknown profile {profile:?}"))?;
        let recipe = string_field(table, "recipe", &id)?;
        let parsed_recipe = parse_recipe(&recipe)?;
        validate_fixture_recipe(&id, &parsed_recipe, limits)?;
        let forms_per_context_max = profile_limit(limits, "forms_per_context_max", &id)?
            .ok_or_else(|| format!("fixture {id:?} has no applicable forms-per-context limit"))?;
        let fixture = Fixture {
            id: id.clone(),
            profile,
            harness_case: string_field(table, "harness_case", &id)?,
            version: positive_integer_field(table, "version", &id)?,
            seed: nonnegative_integer_field(table, "seed", &id)?,
            recipe,
            content_sha256: string_field(table, "content_sha256", &id)?,
            forms_per_context_max: usize::try_from(forms_per_context_max).map_err(|_| {
                format!("fixture {id:?} forms-per-context limit exceeds the host address space")
            })?,
        };
        if !allow_pending_digests && !is_sha256(&fixture.content_sha256) {
            return Err(format!("fixture {id:?} has an invalid content_sha256"));
        }
        if fixtures.insert(id.clone(), fixture).is_some() {
            return Err(format!("duplicate fixture {id:?}"));
        }
    }
    if fixtures.is_empty() {
        return Err("fixture lock is empty".to_owned());
    }
    Ok(fixtures)
}

fn validate_fixture_recipe(
    id: &str,
    recipe: &BTreeMap<String, u64>,
    limits: &FixtureProfileLimits,
) -> Result<(), String> {
    for (recipe_key, limit_field) in [
        ("binding_artifacts", "binding_artifacts_per_thing_max"),
        ("bindings", "bindings_global_max"),
        ("call_bytes", "host_binding_call_bytes_per_item_max"),
        (
            "collection_sources",
            "collection_subscription_sources_per_subscription_max",
        ),
        ("document_bytes", "document_bytes_max"),
        (
            "driver_bytes",
            "host_subscription_driver_bytes_per_item_max",
        ),
        ("extension_bytes", "extension_bytes_max"),
        ("forms", "forms_per_thing_max"),
        ("handler_slots", "handler_slots_per_thing_max"),
        ("ingress_bytes", "binding_ingress_bytes_per_route_max"),
        ("ingress_items", "binding_ingress_items_per_route_max"),
        ("page_entries", "directory_page_entries_max"),
        ("page_item_bytes", "directory_page_item_bytes_max"),
        ("payload_bytes", "payload_bytes_max"),
        ("plan_sets", "plan_sets_per_thing_max"),
        ("readiness_tokens", "route_readiness_tokens_per_thing_max"),
        ("routes", "binding_routes_per_thing_max"),
        ("schema_nodes", "schema_nodes_per_document_max"),
        ("security_branches", "security_branches_per_plan_max"),
        ("slot_state_bytes", "binding_slot_state_bytes_per_item_max"),
        ("string_bytes", "string_bytes_max"),
        ("subscribers", "subscriptions_per_thing_max"),
        ("td_nodes", "json_value_nodes_per_document_max"),
        ("uri_template_bytes", "uri_template_source_bytes_max"),
    ] {
        let requested = recipe.get(recipe_key).copied().unwrap_or(0);
        match profile_limit(limits, limit_field, id)? {
            Some(limit) if requested <= limit => {}
            Some(limit) => {
                return Err(format!(
                    "fixture {id:?} recipe {recipe_key:?} requests {requested}, exceeding {limit_field}={limit}"
                ));
            }
            None if requested == 0 => {}
            None => {
                return Err(format!(
                    "fixture {id:?} recipe {recipe_key:?} is not applicable to its profile"
                ));
            }
        }
    }

    let forms = recipe.get("forms").copied().unwrap_or(0);
    let forms_per_context = profile_limit(limits, "forms_per_context_max", id)?
        .ok_or_else(|| format!("fixture {id:?} has no forms-per-context limit"))?;
    let affordance_limit = profile_limit(limits, "affordances_per_thing_max", id)?
        .ok_or_else(|| format!("fixture {id:?} has no affordance limit"))?;
    let contexts = if forms == 0 {
        0
    } else {
        forms.div_ceil(forms_per_context)
    };
    if contexts > affordance_limit {
        return Err(format!(
            "fixture {id:?} needs {contexts} form contexts, exceeding affordances_per_thing_max={affordance_limit}"
        ));
    }
    if forms > 0
        && recipe.get("document_bytes").copied().unwrap_or(0) == 0
        && recipe.get("page_entries").copied().unwrap_or(0) == 0
    {
        return Err(format!("fixture {id:?} declares forms without a document"));
    }
    let page_entries = recipe.get("page_entries").copied().unwrap_or(0);
    let page_item_bytes = recipe.get("page_item_bytes").copied().unwrap_or(0);
    if page_entries > 0 {
        let page_bytes = page_entries
            .checked_mul(page_item_bytes)
            .ok_or_else(|| format!("fixture {id:?} page byte count overflows"))?;
        let limit = profile_limit(limits, "directory_page_bytes_max", id)?
            .ok_or_else(|| format!("fixture {id:?} has no directory-page byte limit"))?;
        if page_bytes > limit {
            return Err(format!(
                "fixture {id:?} page bytes {page_bytes} exceed directory_page_bytes_max={limit}"
            ));
        }
    }
    Ok(())
}

fn profile_limit(
    limits: &FixtureProfileLimits,
    field: &str,
    id: &str,
) -> Result<Option<u64>, String> {
    limits
        .values
        .get(field)
        .copied()
        .ok_or_else(|| format!("fixture {id:?} requires unknown resource field {field:?}"))
}

#[allow(clippy::too_many_arguments)]
fn load_manifest(
    root: &Path,
    relative_path: &str,
    requirements: &BTreeSet<String>,
    fixtures: &BTreeMap<String, Fixture>,
    case_ids: &mut BTreeSet<String>,
    fixture_uses: &mut BTreeSet<String>,
    allow_pending_digests: bool,
) -> Result<Manifest, String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    require_integer(document.get("schema_version"), "manifest schema_version", 5)?;
    require_string(
        document.get("fixture_generator"),
        "fixture_generator",
        GENERATOR,
    )?;
    require_string(
        document.get("fixture_lock"),
        "fixture_lock",
        "docs/performance/fixtures.lock.toml",
    )?;

    let profile = root_string(&document, "profile", relative_path)?;
    let tag = match profile.as_str() {
        "GatewayDefaultV1" => "GW",
        "DirectoryClientDefaultV1" => "DIR",
        "BenchmarkStaticReferenceV1" => "CS",
        _ => return Err(format!("{relative_path} has unknown profile {profile:?}")),
    };
    let resource_profile = root_string(&document, "resource_profile", relative_path)?;
    let feature_set = root_string(&document, "feature_set", relative_path)?;
    let target = root_string(&document, "environment", relative_path)?;
    let toolchain = root_string(&document, "toolchain", relative_path)?;
    let allocator = root_string(&document, "allocator", relative_path)?;
    let runner = root_string(&document, "runner", relative_path)?;
    let runner_fingerprint = load_runner_fingerprint(&document, &runner, relative_path)?;
    let expected_sample_count = positive_root_integer(&document, "sample_count", relative_path)?;
    nonnegative_root_integer(&document, "fixture_seed", relative_path)?;
    for table in ["gate_policy", "scales"] {
        if document.get(table).and_then(Item::as_table).is_none() {
            return Err(format!("{relative_path} has no {table} table"));
        }
    }
    let fixture_digest = root_string(&document, "fixture_digest", relative_path)?;
    if !allow_pending_digests && !is_prefixed_sha256(&fixture_digest) {
        return Err(format!("{relative_path} has an invalid fixture_digest"));
    }
    let operation_boundaries = document
        .get("operation_boundary")
        .and_then(Item::as_table)
        .ok_or_else(|| format!("{relative_path} has no operation_boundary table"))?;
    let measurement_source = document
        .get("measurement")
        .and_then(Item::as_table)
        .ok_or_else(|| format!("{relative_path} has no measurement table"))?
        .to_string();
    let manifest_sha256 = sha256_hex(source.as_bytes());
    let mut cases = Vec::new();
    for (kind, require_report, require_boundary) in
        [("workload", true, true), ("contention", false, false)]
    {
        let Some(tables) = document.get(kind).and_then(Item::as_array_of_tables) else {
            continue;
        };
        for table in tables {
            let id = string_field(table, "id", kind)?;
            validate_case_id(&id, "PERF", tag)?;
            if !case_ids.insert(id.clone()) {
                return Err(format!("duplicate performance case {id:?}"));
            }
            let fixture_id = string_field(table, "fixture_id", &id)?;
            validate_case_id(&fixture_id, "FX", tag)?;
            if !fixture_uses.insert(fixture_id.clone()) {
                return Err(format!(
                    "fixture {fixture_id:?} is referenced more than once"
                ));
            }
            let fixture = fixtures
                .get(&fixture_id)
                .ok_or_else(|| format!("{id} references unknown fixture {fixture_id:?}"))?;
            let name = string_field(table, "name", &id)?;
            let harness_case = string_field(table, "harness_case", &id)?;
            if harness_case != name {
                return Err(format!("{id} harness_case must equal its stable name"));
            }
            if fixture.profile != profile || fixture.harness_case != harness_case {
                return Err(format!(
                    "{id} fixture identity does not match profile and harness case"
                ));
            }
            validate_axis_contract(table, fixture, &id)?;
            let version = positive_integer_field(table, "version", &id)?;
            if fixture.version != version {
                return Err(format!("{id} and {fixture_id} versions differ"));
            }
            let case_requirements = string_array_field(table, "requirements", &id)?;
            if case_requirements.is_empty() {
                return Err(format!("{id} has no requirements"));
            }
            let mut unique_requirements = BTreeSet::new();
            for requirement in &case_requirements {
                if !requirements.contains(requirement) {
                    return Err(format!(
                        "{id} references unknown requirement {requirement:?}"
                    ));
                }
                if !unique_requirements.insert(requirement) {
                    return Err(format!("{id} repeats requirement {requirement:?}"));
                }
            }
            let gating = bool_field(table, "gating", &id)?;
            let characterization = table.get("characterization").and_then(Item::as_bool);
            if (!gating && characterization != Some(true))
                || (gating && characterization == Some(true))
            {
                return Err(format!(
                    "{id} has inconsistent gating/characterization markers"
                ));
            }
            let gates = parse_gate_specs(table, &id)?;
            if gating && gates.is_empty() {
                return Err(format!(
                    "{id} has no absolute numeric or deterministic gate"
                ));
            }
            let coverage = parse_coverage_spec(table, &id)?;
            let activation_trace_oracle = table
                .get("activation_trace_oracle")
                .map(|_| string_field(table, "activation_trace_oracle", &id))
                .transpose()?;
            let activation_cases = table
                .get("activation_cases")
                .map(|_| string_array_field(table, "activation_cases", &id))
                .transpose()?;
            if activation_trace_oracle.is_some() != activation_cases.is_some() {
                return Err(format!(
                    "{id} must define activation_trace_oracle and activation_cases together"
                ));
            }
            let report_metrics = if require_report {
                let metrics = string_array_field(table, "report", &id)?;
                if metrics.is_empty() {
                    return Err(format!("{id} has no report metrics"));
                }
                let unique: BTreeSet<String> = metrics.iter().cloned().collect();
                if unique.len() != metrics.len() {
                    return Err(format!("{id} repeats a report metric"));
                }
                unique
            } else {
                BTreeSet::new()
            };
            let boundary = if require_boundary {
                operation_boundaries
                    .get(&name)
                    .and_then(Item::as_str)
                    .ok_or_else(|| format!("{id} has no operation boundary for {name:?}"))?
                    .to_owned()
            } else {
                format!("contention:{name}")
            };
            let measurement_sha256 = sha256_hex(
                format!(
                    "measurement-v1\0{profile}\0{id}\0{harness_case}\0{measurement_source}\0{boundary}"
                )
                .as_bytes(),
            );
            cases.push(Case {
                manifest_path: path.clone(),
                manifest_sha256: manifest_sha256.clone(),
                profile: profile.clone(),
                resource_profile: resource_profile.clone(),
                feature_set: feature_set.clone(),
                target: target.clone(),
                toolchain: toolchain.clone(),
                allocator: allocator.clone(),
                runner: runner.clone(),
                runner_fingerprint: runner_fingerprint.clone(),
                id,
                version,
                fixture_id,
                harness_case,
                measurement_sha256,
                gating,
                expected_sample_count,
                coverage,
                activation_trace_oracle,
                activation_cases,
                gates,
                report_metrics,
            });
        }
    }
    if cases.is_empty() {
        return Err(format!("{relative_path} has no performance cases"));
    }
    validate_code_size(&document, relative_path)?;
    Ok(Manifest {
        path,
        fixture_digest,
        cases,
    })
}

fn validate_code_size(document: &DocumentMut, context: &str) -> Result<(), String> {
    let Some(code_size) = document.get("code_size").and_then(Item::as_table) else {
        return Ok(());
    };
    let characterization = code_size
        .get("characterization")
        .and_then(Item::as_bool)
        .unwrap_or(false);
    let absolute =
        code_size.get("text_bytes_max").is_some() || code_size.get("total_bytes_max").is_some();
    if !characterization && !absolute {
        return Err(format!(
            "{context} code_size must be characterization or have an absolute budget"
        ));
    }
    Ok(())
}

fn load_runner_fingerprint(
    document: &DocumentMut,
    runner: &str,
    context: &str,
) -> Result<RunnerFingerprint, String> {
    let table = document
        .get("runner_fingerprint")
        .and_then(Item::as_table)
        .ok_or_else(|| format!("{context} has no runner_fingerprint table"))?;
    let required: BTreeSet<String> = [
        "board",
        "class",
        "clock_source",
        "cpu_model",
        "frequency_policy",
        "kernel_or_runtime",
        "memory",
        "os_or_firmware",
        "physical_cores",
        "threads_per_core",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let mut values = BTreeMap::new();
    for (key, item) in table.iter() {
        let value = item.as_str().ok_or_else(|| {
            format!("{context} runner fingerprint field {key:?} must be a string")
        })?;
        if value.is_empty() {
            return Err(format!(
                "{context} runner fingerprint field {key:?} is empty"
            ));
        }
        values.insert(key.to_owned(), value.to_owned());
    }
    let actual: BTreeSet<String> = values.keys().cloned().collect();
    if actual != required {
        return Err(format!(
            "{context} runner fingerprint fields mismatch; expected {required:?}, found {actual:?}"
        ));
    }
    if values.get("class").map(String::as_str) != Some(runner) {
        return Err(format!(
            "{context} runner must equal runner_fingerprint.class"
        ));
    }
    let mut hasher = Sha256::new();
    hasher.update(b"clinkz-wot-runner-fingerprint-v1\0");
    for (key, value) in &values {
        hasher.update(key.as_bytes());
        hasher.update([0]);
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    Ok(RunnerFingerprint {
        values,
        sha256: format!("sha256:{}", hex::encode(hasher.finalize())),
    })
}

fn validate_axis_contract(table: &Table, fixture: &Fixture, context: &str) -> Result<(), String> {
    if table.get("axes").is_none() {
        return Ok(());
    }
    if table
        .get("require_vary_one_axis_at_a_time")
        .and_then(Item::as_bool)
        != Some(true)
    {
        return Err(format!(
            "{context} declares scaling axes without the executable one-axis gate"
        ));
    }
    let axes = string_array_field(table, "axes", context)?;
    if axes.is_empty() {
        return Err(format!("{context} has no scaling axes"));
    }
    let recipe = parse_recipe(&fixture.recipe)?;
    for axis in axes {
        let required_keys: &[&str] = match axis.as_str() {
            "bindings" => &["bindings"],
            "document-bytes" | "td-bytes" => &["document_bytes"],
            "extension-bytes" => &["extension_bytes"],
            "forms" | "td-forms" => &["forms"],
            "page-entries" => &["page_entries", "page_item_bytes"],
            "schema-nodes" => &["schema_nodes"],
            "security-branches" => &["security_branches"],
            "string-bytes" => &["string_bytes"],
            "td-nodes" => &["td_nodes"],
            "uri-template-bytes" => &["uri_template_bytes"],
            _ => return Err(format!("{context} has unknown scaling axis {axis:?}")),
        };
        for key in required_keys {
            if recipe.get(*key).copied().unwrap_or(0) == 0 {
                return Err(format!(
                    "{context} scaling axis {axis:?} has no nonzero fixture section {key:?}"
                ));
            }
        }
    }
    Ok(())
}

fn validate_case_id(id: &str, prefix: &str, tag: &str) -> Result<(), String> {
    let expected = format!("{prefix}-{tag}-");
    let Some(number) = id.strip_prefix(&expected) else {
        return Err(format!("case identity {id:?} must start with {expected:?}"));
    };
    if number.len() != 3 || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("case identity {id:?} must end in three digits"));
    }
    Ok(())
}

fn parse_gate_specs(table: &Table, context: &str) -> Result<Vec<GateSpec>, String> {
    let mut gates = Vec::new();
    let mut metrics = BTreeSet::new();
    for (key, item) in table.iter() {
        let (metric, comparator, threshold) = if let Some(metric) = key.strip_suffix("_max") {
            let threshold = numeric_item(item)
                .ok_or_else(|| format!("{context} gate {key:?} must be a numeric threshold"))?;
            (metric, GateComparator::AtMost, threshold)
        } else if let Some(metric) = key.strip_suffix("_min") {
            let threshold = numeric_item(item)
                .ok_or_else(|| format!("{context} gate {key:?} must be a numeric threshold"))?;
            (metric, GateComparator::AtLeast, threshold)
        } else if key.starts_with("require_") && item.as_bool() == Some(true) {
            (key, GateComparator::ExactlyOne, 1.0)
        } else {
            continue;
        };
        if !threshold.is_finite() || threshold < 0.0 {
            return Err(format!(
                "{context} gate {key:?} must have a finite nonnegative threshold"
            ));
        }
        if metric.is_empty() || !metrics.insert(metric.to_owned()) {
            return Err(format!(
                "{context} has an empty or duplicate gate metric {metric:?}"
            ));
        }
        gates.push(GateSpec {
            source_field: key.to_owned(),
            metric: metric.to_owned(),
            comparator,
            threshold,
        });
    }
    Ok(gates)
}

fn parse_coverage_spec(table: &Table, context: &str) -> Result<Option<CoverageSpec>, String> {
    if table.get("coverage_dimensions").is_none() {
        return Ok(None);
    }
    let dimension_names = string_array_field(table, "coverage_dimensions", context)?;
    if dimension_names.is_empty() {
        return Err(format!("{context} has an empty coverage dimension list"));
    }
    let mut seen = BTreeSet::new();
    let mut dimensions = Vec::new();
    let mut cell_count = 1_u64;
    for name in dimension_names {
        if !seen.insert(name.clone()) {
            return Err(format!("{context} repeats coverage dimension {name:?}"));
        }
        let values = string_array_field(table, &name, context)?;
        if values.is_empty() {
            return Err(format!("{context} coverage dimension {name:?} is empty"));
        }
        let unique: BTreeSet<&str> = values.iter().map(String::as_str).collect();
        if unique.len() != values.len() {
            return Err(format!(
                "{context} coverage dimension {name:?} repeats a value"
            ));
        }
        let value_count = u64::try_from(values.len())
            .map_err(|_| format!("{context} coverage dimension {name:?} is too large"))?;
        cell_count = cell_count
            .checked_mul(value_count)
            .ok_or_else(|| format!("{context} coverage cell count overflows"))?;
        dimensions.push((name, values));
    }
    let sha256 = coverage_sha256(&dimensions, cell_count)?;
    Ok(Some(CoverageSpec { cell_count, sha256 }))
}

fn coverage_sha256(
    dimensions: &[(String, Vec<String>)],
    cell_count: u64,
) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update(b"clinkz-wot-coverage-v1\0");
    for ordinal in 0..cell_count {
        let mut remainder = ordinal;
        let mut indices = vec![0_usize; dimensions.len()];
        for (index, (_, values)) in dimensions.iter().enumerate().rev() {
            let radix = u64::try_from(values.len())
                .map_err(|_| "coverage dimension exceeds u64".to_owned())?;
            indices[index] = usize::try_from(remainder % radix)
                .map_err(|_| "coverage index exceeds usize".to_owned())?;
            remainder /= radix;
        }
        for ((name, values), index) in dimensions.iter().zip(indices) {
            hasher.update(name.as_bytes());
            hasher.update([0]);
            hasher.update(values[index].as_bytes());
            hasher.update([0]);
        }
        hasher.update([0xff]);
    }
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}

fn numeric_item(item: &Item) -> Option<f64> {
    item.as_integer()
        .map(|value| value as f64)
        .or_else(|| item.as_float())
}

fn verify_digests(
    fixtures: &BTreeMap<String, Fixture>,
    manifests: &[Manifest],
) -> Result<(), String> {
    for fixture in fixtures.values() {
        let actual = fixture_digest(fixture)?;
        if actual != fixture.content_sha256 {
            return Err(format!(
                "fixture {:?} digest mismatch; expected {}, found {}",
                fixture.id, fixture.content_sha256, actual
            ));
        }
    }
    for manifest in manifests {
        let actual = format!("sha256:{}", manifest_fixture_digest(manifest, fixtures)?);
        if actual != manifest.fixture_digest {
            return Err(format!(
                "{} fixture digest mismatch; expected {}, found {}",
                manifest.path.display(),
                manifest.fixture_digest,
                actual
            ));
        }
    }
    Ok(())
}

fn refresh_digests(root: &Path) -> Result<(), String> {
    let (fixtures, manifests) = load_contract(root, true)?;
    let lock_path = root.join("docs/performance/fixtures.lock.toml");
    let mut lock = parse_toml(&lock_path)?;
    let tables = lock
        .get_mut("fixture")
        .and_then(Item::as_array_of_tables_mut)
        .ok_or_else(|| "fixture lock has no [[fixture]] entries".to_owned())?;
    for table in tables.iter_mut() {
        let id = string_field(table, "id", "fixture")?;
        let fixture = fixtures
            .get(&id)
            .ok_or_else(|| format!("fixture lock contains unknown fixture {id:?}"))?;
        table["content_sha256"] = value(fixture_digest(fixture)?);
    }
    fs::write(&lock_path, lock.to_string())
        .map_err(|error| format!("cannot write {}: {error}", lock_path.display()))?;

    for manifest in &manifests {
        let source = fs::read_to_string(&manifest.path)
            .map_err(|error| format!("cannot read {}: {error}", manifest.path.display()))?;
        let mut document = source
            .parse::<DocumentMut>()
            .map_err(|error| format!("invalid {}: {error}", manifest.path.display()))?;
        document["fixture_digest"] = value(format!(
            "sha256:{}",
            manifest_fixture_digest(manifest, &fixtures)?,
        ));
        fs::write(&manifest.path, document.to_string())
            .map_err(|error| format!("cannot write {}: {error}", manifest.path.display()))?;
    }
    Ok(())
}

fn manifest_fixture_digest(
    manifest: &Manifest,
    fixtures: &BTreeMap<String, Fixture>,
) -> Result<String, String> {
    let mut cases: Vec<&Case> = manifest.cases.iter().collect();
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    let mut hasher = Sha256::new();
    for case in cases {
        let fixture = fixtures
            .get(&case.fixture_id)
            .ok_or_else(|| format!("unknown fixture {:?}", case.fixture_id))?;
        hasher.update(fixture.id.as_bytes());
        hasher.update([0]);
        hasher.update(generate_fixture(fixture)?);
        hasher.update([0]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn fixture_digest(fixture: &Fixture) -> Result<String, String> {
    Ok(sha256_hex(&generate_fixture(fixture)?))
}

fn generate_fixture(fixture: &Fixture) -> Result<Vec<u8>, String> {
    let recipe = parse_recipe(&fixture.recipe)?;
    let mut output = b"clinkz-wot-fixture-v3\0".to_vec();
    append_section(&mut output, "fixture-id", fixture.id.as_bytes())?;
    append_section(&mut output, "profile", fixture.profile.as_bytes())?;
    append_section(&mut output, "harness-case", fixture.harness_case.as_bytes())?;
    append_section(&mut output, "recipe", fixture.recipe.as_bytes())?;

    let document_bytes = recipe_value(&recipe, "document_bytes")?;
    let forms = recipe_value(&recipe, "forms")?;
    if document_bytes > 0 {
        let document = generate_document(&fixture.id, document_bytes, fixture.seed)?;
        append_section(&mut output, "document", &document)?;
    }
    if forms > 0 {
        let form_contexts =
            generate_form_contexts(&fixture.id, forms, fixture.forms_per_context_max)?;
        append_section(&mut output, "form-contexts", &form_contexts)?;
    }

    append_deterministic_bytes(
        &mut output,
        "extension-bytes",
        recipe_value(&recipe, "extension_bytes")?,
        fixture.seed ^ 0x4558_5445_4e53_494f,
    )?;
    append_deterministic_bytes(
        &mut output,
        "string-bytes",
        recipe_value(&recipe, "string_bytes")?,
        fixture.seed ^ 0x5354_5249_4e47_0001,
    )?;
    append_deterministic_bytes(
        &mut output,
        "uri-template-bytes",
        recipe_value(&recipe, "uri_template_bytes")?,
        fixture.seed ^ 0x5552_4954_454d_5001,
    )?;
    append_deterministic_bytes(
        &mut output,
        "call-bytes",
        recipe_value(&recipe, "call_bytes")?,
        fixture.seed ^ 0x4341_4c4c_0000_0001,
    )?;
    append_deterministic_bytes(
        &mut output,
        "driver-bytes",
        recipe_value(&recipe, "driver_bytes")?,
        fixture.seed ^ 0x4452_4956_4552_0001,
    )?;
    append_deterministic_bytes(
        &mut output,
        "ingress-bytes",
        recipe_value(&recipe, "ingress_bytes")?,
        fixture.seed ^ 0x494e_4752_4553_5301,
    )?;
    append_deterministic_bytes(
        &mut output,
        "slot-state-bytes",
        recipe_value(&recipe, "slot_state_bytes")?,
        fixture.seed ^ 0x534c_4f54_5354_0001,
    )?;

    let payload_bytes = recipe_value(&recipe, "payload_bytes")?;
    if payload_bytes > 0 {
        let payload = deterministic_bytes(payload_bytes, fixture.seed ^ 0x5041_594c_4f41_4401)?;
        append_section(&mut output, "payload", &payload)?;
    }

    let page_entries = recipe_value(&recipe, "page_entries")?;
    let page_item_bytes = recipe_value(&recipe, "page_item_bytes")?;
    if page_entries > 0 && page_item_bytes == 0 {
        return Err(format!(
            "fixture {:?} has page_entries without page_item_bytes",
            fixture.id
        ));
    }
    for index in 0..page_entries {
        let id = format!("{}-page-{index:06}", fixture.id);
        let item = generate_document(
            &id,
            page_item_bytes,
            fixture.seed.wrapping_add(index as u64),
        )?;
        append_section(&mut output, "page-item", &item)?;
    }

    append_fixed_records(
        &mut output,
        "actor",
        recipe_value(&recipe, "actors")?,
        fixture.seed ^ 0x4143_544f_5200_0001,
    )?;
    append_fixed_records(
        &mut output,
        "binding-artifact",
        recipe_value(&recipe, "binding_artifacts")?,
        fixture.seed ^ 0x4152_5449_4641_4354,
    )?;
    append_fixed_records(
        &mut output,
        "binding",
        recipe_value(&recipe, "bindings")?,
        fixture.seed ^ 0x4249_4e44_494e_4701,
    )?;
    append_fixed_records(
        &mut output,
        "collection-source",
        recipe_value(&recipe, "collection_sources")?,
        fixture.seed ^ 0x434f_4c4c_4543_5401,
    )?;
    append_fixed_records(
        &mut output,
        "handler-slot",
        recipe_value(&recipe, "handler_slots")?,
        fixture.seed ^ 0x4841_4e44_4c45_5201,
    )?;
    append_fixed_records(
        &mut output,
        "ingress-item",
        recipe_value(&recipe, "ingress_items")?,
        fixture.seed ^ 0x494e_4752_4553_5302,
    )?;
    append_fixed_records(
        &mut output,
        "plan-set",
        recipe_value(&recipe, "plan_sets")?,
        fixture.seed ^ 0x504c_414e_5345_5401,
    )?;
    append_fixed_records(
        &mut output,
        "readiness-token",
        recipe_value(&recipe, "readiness_tokens")?,
        fixture.seed ^ 0x5245_4144_594e_0001,
    )?;
    append_fixed_records(
        &mut output,
        "route",
        recipe_value(&recipe, "routes")?,
        fixture.seed ^ 0x524f_5554_4500_0001,
    )?;
    append_fixed_records(
        &mut output,
        "schema-node",
        recipe_value(&recipe, "schema_nodes")?,
        fixture.seed ^ 0x5343_4845_4d41_0001,
    )?;
    append_fixed_records(
        &mut output,
        "security-branch",
        recipe_value(&recipe, "security_branches")?,
        fixture.seed ^ 0x5345_4355_5249_5401,
    )?;
    append_fixed_records(
        &mut output,
        "subscriber",
        recipe_value(&recipe, "subscribers")?,
        fixture.seed ^ 0x5355_4253_4352_0001,
    )?;
    append_fixed_records(
        &mut output,
        "td-node",
        recipe_value(&recipe, "td_nodes")?,
        fixture.seed ^ 0x5444_4e4f_4445_0001,
    )?;
    Ok(output)
}

fn parse_recipe(recipe: &str) -> Result<BTreeMap<String, u64>, String> {
    let mut values = BTreeMap::new();
    let mut previous = None::<&str>;
    if recipe.is_empty() {
        return Err("fixture recipe is empty".to_owned());
    }
    for part in recipe.split(';') {
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| format!("invalid recipe component {part:?}"))?;
        if !RECIPE_KEYS.contains(&key) {
            return Err(format!("unknown recipe key {key:?}"));
        }
        if previous.is_some_and(|previous| previous >= key) {
            return Err(format!("recipe keys are not strictly sorted at {key:?}"));
        }
        previous = Some(key);
        let value = value
            .parse::<u64>()
            .map_err(|error| format!("invalid recipe value for {key:?}: {error}"))?;
        if values.insert(key.to_owned(), value).is_some() {
            return Err(format!("duplicate recipe key {key:?}"));
        }
    }
    Ok(values)
}

fn recipe_value(recipe: &BTreeMap<String, u64>, key: &str) -> Result<usize, String> {
    usize::try_from(recipe.get(key).copied().unwrap_or(0))
        .map_err(|_| format!("recipe value for {key:?} exceeds the host address space"))
}

fn generate_document(id: &str, target_bytes: usize, seed: u64) -> Result<Vec<u8>, String> {
    let mut prefix = format!(
        "{{\"@context\":\"https://www.w3.org/2022/wot/td/v1.1\",\"id\":\"urn:clinkz:fixture:{id}\",\"title\":\"Fixture-{seed:016x}\""
    );
    prefix.push_str(",\"padding\":\"");
    let suffix = "\"}";
    let minimum = prefix.len() + suffix.len();
    if target_bytes < minimum {
        return Err(format!(
            "document {id:?} requires at least {minimum} bytes, recipe requests {target_bytes}"
        ));
    }
    let mut document = prefix.into_bytes();
    document.resize(target_bytes - suffix.len(), b'x');
    document.extend_from_slice(suffix.as_bytes());
    debug_assert_eq!(document.len(), target_bytes);
    Ok(document)
}

fn generate_form_contexts(
    id: &str,
    forms: usize,
    forms_per_context_max: usize,
) -> Result<Vec<u8>, String> {
    if forms == 0 || forms_per_context_max == 0 {
        return Err(format!(
            "form section {id:?} requires nonzero forms and context capacity"
        ));
    }
    let mut source = String::from("{");
    let mut remaining = forms;
    let mut form_index = 0_usize;
    let mut context_index = 0_usize;
    while remaining > 0 {
        if context_index > 0 {
            source.push(',');
        }
        source.push_str(&format!(
            "\"property-{context_index:06}\":{{\"type\":\"string\",\"forms\":["
        ));
        let context_forms = remaining.min(forms_per_context_max);
        for offset in 0..context_forms {
            if offset > 0 {
                source.push(',');
            }
            source.push_str(&format!(
                "{{\"href\":\"coap://fixture.invalid/{id}/{form_index}\",\"op\":\"readproperty\"}}"
            ));
            form_index += 1;
        }
        source.push_str("]}");
        remaining -= context_forms;
        context_index += 1;
    }
    source.push('}');
    Ok(source.into_bytes())
}

fn append_deterministic_bytes(
    output: &mut Vec<u8>,
    section: &str,
    length: usize,
    seed: u64,
) -> Result<(), String> {
    if length > 0 {
        append_section(output, section, &deterministic_bytes(length, seed)?)?;
    }
    Ok(())
}

fn deterministic_bytes(length: usize, seed: u64) -> Result<Vec<u8>, String> {
    if length > 64 * 1024 * 1024 {
        return Err(format!("fixture byte section is too large: {length}"));
    }
    let mut state = seed.max(1);
    let mut bytes = Vec::with_capacity(length);
    for _ in 0..length {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bytes.push((state >> 24) as u8);
    }
    Ok(bytes)
}

fn append_fixed_records(
    output: &mut Vec<u8>,
    section: &str,
    count: usize,
    seed: u64,
) -> Result<(), String> {
    let byte_count = count
        .checked_mul(16)
        .ok_or_else(|| format!("{section} record count overflows"))?;
    let mut records = Vec::with_capacity(byte_count);
    for index in 0..count {
        records.extend_from_slice(&(index as u64).to_le_bytes());
        records.extend_from_slice(&seed.wrapping_add(index as u64).to_le_bytes());
    }
    if !records.is_empty() {
        append_section(output, section, &records)?;
    }
    Ok(())
}

fn append_section(output: &mut Vec<u8>, name: &str, bytes: &[u8]) -> Result<(), String> {
    let name_length = u16::try_from(name.len())
        .map_err(|_| format!("fixture section name is too long: {name:?}"))?;
    let byte_length =
        u64::try_from(bytes.len()).map_err(|_| format!("fixture section is too long: {name:?}"))?;
    output.extend_from_slice(&name_length.to_le_bytes());
    output.extend_from_slice(name.as_bytes());
    output.extend_from_slice(&byte_length.to_le_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

fn run_adapter(root: &Path, workload_id: &str, adapter: &Path) -> Result<(), String> {
    let (fixtures, manifests) = load_contract(root, false)?;
    verify_digests(&fixtures, &manifests)?;
    let case = sorted_cases(&manifests)
        .into_iter()
        .find(|case| case.id == workload_id)
        .ok_or_else(|| format!("unknown workload id {workload_id:?}"))?;
    let fixture = fixtures
        .get(&case.fixture_id)
        .ok_or_else(|| format!("unknown fixture {:?}", case.fixture_id))?;
    let nonce = format!("{}-{}", std::process::id(), case.id.to_ascii_lowercase());
    let temporary = env::temp_dir();
    let fixture_path = temporary.join(format!("clinkz-wot-{nonce}.fixture"));
    let result_path = temporary.join(format!("clinkz-wot-{nonce}.result.json"));
    fs::write(&fixture_path, generate_fixture(fixture)?)
        .map_err(|error| format!("cannot write {}: {error}", fixture_path.display()))?;
    if result_path.exists() {
        fs::remove_file(&result_path)
            .map_err(|error| format!("cannot clear {}: {error}", result_path.display()))?;
    }
    let status = Command::new(adapter)
        .arg("--manifest")
        .arg(&case.manifest_path)
        .arg("--workload-id")
        .arg(&case.id)
        .arg("--fixture")
        .arg(&fixture_path)
        .arg("--result")
        .arg(&result_path)
        .status()
        .map_err(|error| format!("cannot execute adapter {}: {error}", adapter.display()));
    let outcome = match status {
        Ok(status) if status.success() => validate_result(case, fixture, &result_path),
        Ok(status) => Err(format!("adapter exited with {status}")),
        Err(error) => Err(error),
    };
    let _ = fs::remove_file(&fixture_path);
    if outcome.is_ok() {
        println!(
            "performance harness: result accepted at {}",
            result_path.display()
        );
    }
    outcome
}

fn validate_result(case: &Case, fixture: &Fixture, path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read adapter result {}: {error}", path.display()))?;
    let result: JsonValue = serde_json::from_str(&source)
        .map_err(|error| format!("invalid adapter result {}: {error}", path.display()))?;
    let object = result
        .as_object()
        .ok_or_else(|| "adapter result must be a JSON object".to_owned())?;
    let allowed_fields = [
        "schema_version",
        "workload_id",
        "workload_version",
        "profile",
        "resource_profile",
        "manifest_sha256",
        "fixture_id",
        "fixture_sha256",
        "harness_case",
        "measurement_sha256",
        "feature_set",
        "target",
        "toolchain",
        "allocator",
        "runner_class",
        "runner_fingerprint",
        "runner_fingerprint_sha256",
        "status",
        "sample_count",
        "failed_samples",
        "coverage_sha256",
        "coverage_cell_count",
        "samples_per_cell",
        "metrics",
        "notes",
    ];
    if let Some(field) = object
        .keys()
        .find(|field| !allowed_fields.contains(&field.as_str()))
    {
        return Err(format!("adapter result has unknown field {field:?}"));
    }
    expect_json_u64(object.get("schema_version"), "schema_version", 2)?;
    expect_json_string(object.get("workload_id"), "workload_id", &case.id)?;
    expect_json_u64(
        object.get("workload_version"),
        "workload_version",
        case.version,
    )?;
    expect_json_string(object.get("profile"), "profile", &case.profile)?;
    expect_json_string(
        object.get("resource_profile"),
        "resource_profile",
        &case.resource_profile,
    )?;
    expect_json_string(
        object.get("manifest_sha256"),
        "manifest_sha256",
        &case.manifest_sha256,
    )?;
    expect_json_string(object.get("fixture_id"), "fixture_id", &case.fixture_id)?;
    expect_json_string(
        object.get("fixture_sha256"),
        "fixture_sha256",
        &fixture.content_sha256,
    )?;
    expect_json_string(
        object.get("harness_case"),
        "harness_case",
        &case.harness_case,
    )?;
    expect_json_string(
        object.get("measurement_sha256"),
        "measurement_sha256",
        &case.measurement_sha256,
    )?;
    expect_json_string(object.get("feature_set"), "feature_set", &case.feature_set)?;
    expect_json_string(object.get("target"), "target", &case.target)?;
    for (field, expected) in [
        ("toolchain", case.toolchain.as_str()),
        ("allocator", case.allocator.as_str()),
        ("runner_class", case.runner.as_str()),
    ] {
        expect_manifest_json_string(object.get(field), field, expected)?;
    }
    validate_runner_fingerprint(case, object)?;
    let status = object
        .get("status")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| "adapter result has no status".to_owned())?;
    let allowed = ["passed", "failed", "characterization", "unavailable"];
    if !allowed.contains(&status) {
        return Err(format!("adapter result has unknown status {status:?}"));
    }
    for field in ["sample_count", "failed_samples"] {
        if object.get(field).and_then(JsonValue::as_u64).is_none() {
            return Err(format!(
                "adapter result has no nonnegative integer {field:?}"
            ));
        }
    }
    let sample_count = object["sample_count"]
        .as_u64()
        .ok_or_else(|| "adapter result sample_count is invalid".to_owned())?;
    let failed_samples = object["failed_samples"]
        .as_u64()
        .ok_or_else(|| "adapter result failed_samples is invalid".to_owned())?;
    if failed_samples > sample_count {
        return Err("adapter result failed_samples exceeds sample_count".to_owned());
    }
    validate_result_coverage(case, object)?;
    let metrics = object
        .get("metrics")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| "adapter result has no metrics object".to_owned())?;
    let numeric_metrics: BTreeMap<&str, f64> = metrics
        .iter()
        .map(|(name, value)| {
            let value = value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("adapter result metric {name:?} must be a finite number"))?;
            Ok((name.as_str(), value))
        })
        .collect::<Result<_, String>>()?;
    validate_result_outcome(case, status, sample_count, failed_samples, &numeric_metrics)?;
    if let Some(notes) = object.get("notes") {
        let notes = notes
            .as_array()
            .ok_or_else(|| "adapter result notes must be an array".to_owned())?;
        if notes.iter().any(|note| note.as_str().is_none()) {
            return Err("adapter result notes must contain only strings".to_owned());
        }
    }
    Ok(())
}

fn validate_runner_fingerprint(
    case: &Case,
    object: &serde_json::Map<String, JsonValue>,
) -> Result<(), String> {
    expect_json_string(
        object.get("runner_fingerprint_sha256"),
        "runner_fingerprint_sha256",
        &case.runner_fingerprint.sha256,
    )?;
    let actual = object
        .get("runner_fingerprint")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| "adapter result has no runner_fingerprint object".to_owned())?;
    if actual.len() != case.runner_fingerprint.values.len() {
        return Err(format!(
            "adapter runner fingerprint field count mismatch; expected {}, found {}",
            case.runner_fingerprint.values.len(),
            actual.len()
        ));
    }
    for (field, expected) in &case.runner_fingerprint.values {
        expect_json_string(actual.get(field), field, expected)?;
    }
    Ok(())
}

fn validate_result_coverage(
    case: &Case,
    object: &serde_json::Map<String, JsonValue>,
) -> Result<(), String> {
    let fields = ["coverage_sha256", "coverage_cell_count", "samples_per_cell"];
    match &case.coverage {
        Some(coverage) => {
            expect_json_string(
                object.get("coverage_sha256"),
                "coverage_sha256",
                &coverage.sha256,
            )?;
            expect_json_u64(
                object.get("coverage_cell_count"),
                "coverage_cell_count",
                coverage.cell_count,
            )?;
            expect_json_u64(
                object.get("samples_per_cell"),
                "samples_per_cell",
                case.expected_sample_count,
            )
        }
        None => {
            if let Some(field) = fields.iter().find(|field| object.contains_key(**field)) {
                return Err(format!(
                    "case {:?} has unexpected coverage field {field:?}",
                    case.id
                ));
            }
            Ok(())
        }
    }
}

fn validate_result_outcome(
    case: &Case,
    status: &str,
    sample_count: u64,
    failed_samples: u64,
    metrics: &BTreeMap<&str, f64>,
) -> Result<(), String> {
    if case.gating {
        if status != "passed" {
            return Err(format!(
                "gating case {:?} must report passed, found {status:?}",
                case.id
            ));
        }
    } else if !["characterization", "unavailable"].contains(&status) {
        return Err(format!(
            "characterization case {:?} must report characterization or unavailable, found {status:?}",
            case.id
        ));
    }

    if status == "unavailable" {
        if sample_count != 0 || failed_samples != 0 || !metrics.is_empty() {
            return Err(format!(
                "unavailable case {:?} must have zero samples and no metrics",
                case.id
            ));
        }
        return Ok(());
    }
    let expected_sample_count = case.total_sample_count()?;
    if sample_count != expected_sample_count {
        return Err(format!(
            "case {:?} sample_count mismatch; expected {expected_sample_count}, found {sample_count}",
            case.id
        ));
    }
    if failed_samples != 0 {
        return Err(format!(
            "case {:?} has {failed_samples} failed samples",
            case.id
        ));
    }
    if metrics.is_empty() {
        return Err(format!("case {:?} has no measured metrics", case.id));
    }
    for metric in &case.report_metrics {
        if !metrics.contains_key(metric.as_str()) {
            return Err(format!(
                "case {:?} is missing report metric {metric:?}",
                case.id
            ));
        }
    }
    for gate in &case.gates {
        let actual = metrics.get(gate.metric.as_str()).ok_or_else(|| {
            format!(
                "case {:?} is missing gate metric {:?} for manifest field {:?}",
                case.id, gate.metric, gate.source_field
            )
        })?;
        let passed = match gate.comparator {
            GateComparator::AtLeast => *actual >= gate.threshold,
            GateComparator::AtMost => *actual <= gate.threshold,
            GateComparator::ExactlyOne => *actual == 1.0,
        };
        if !passed {
            return Err(format!(
                "case {:?} gate {:?} failed: metric {:?} is {}, threshold is {}",
                case.id, gate.source_field, gate.metric, actual, gate.threshold
            ));
        }
    }
    Ok(())
}

fn parse_toml(path: &Path) -> Result<DocumentMut, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn root_string(document: &DocumentMut, field: &str, context: &str) -> Result<String, String> {
    let value = document
        .get(field)
        .and_then(Item::as_str)
        .ok_or_else(|| format!("{context} has no string field {field:?}"))?;
    if value.is_empty() {
        return Err(format!("{context} has an empty {field:?}"));
    }
    Ok(value.to_owned())
}

fn nonnegative_root_integer(
    document: &DocumentMut,
    field: &str,
    context: &str,
) -> Result<u64, String> {
    let value = document
        .get(field)
        .and_then(Item::as_integer)
        .ok_or_else(|| format!("{context} has no integer field {field:?}"))?;
    u64::try_from(value).map_err(|_| format!("{context} {field:?} must be nonnegative"))
}

fn positive_root_integer(
    document: &DocumentMut,
    field: &str,
    context: &str,
) -> Result<u64, String> {
    let value = nonnegative_root_integer(document, field, context)?;
    if value == 0 {
        return Err(format!("{context} {field:?} must be positive"));
    }
    Ok(value)
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

fn bool_field(table: &Table, field: &str, context: &str) -> Result<bool, String> {
    table
        .get(field)
        .and_then(Item::as_bool)
        .ok_or_else(|| format!("{context:?} has no Boolean field {field:?}"))
}

fn positive_integer_field(table: &Table, field: &str, context: &str) -> Result<u64, String> {
    let value = nonnegative_integer_field(table, field, context)?;
    if value == 0 {
        return Err(format!("{context:?} {field:?} must be positive"));
    }
    Ok(value)
}

fn nonnegative_integer_field(table: &Table, field: &str, context: &str) -> Result<u64, String> {
    let value = table
        .get(field)
        .and_then(Item::as_integer)
        .ok_or_else(|| format!("{context:?} has no integer field {field:?}"))?;
    u64::try_from(value).map_err(|_| format!("{context:?} {field:?} must be nonnegative"))
}

fn string_array_field(table: &Table, field: &str, context: &str) -> Result<Vec<String>, String> {
    let array = table
        .get(field)
        .and_then(Item::as_array)
        .ok_or_else(|| format!("{context:?} has no array field {field:?}"))?;
    array
        .iter()
        .map(|item| {
            item.as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("{context:?} {field:?} contains a non-string value"))
        })
        .collect()
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

fn sorted_cases(manifests: &[Manifest]) -> Vec<&Case> {
    let mut cases: Vec<&Case> = manifests
        .iter()
        .flat_map(|manifest| manifest.cases.iter())
        .collect();
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    cases
}

fn required_argument(
    arguments: &mut impl Iterator<Item = String>,
    description: &str,
) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("missing {description}"))
}

fn reject_extra(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    if let Some(argument) = arguments.next() {
        return Err(format!("unexpected argument {argument:?}"));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_prefixed_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(is_sha256)
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn expect_json_string(item: Option<&JsonValue>, field: &str, expected: &str) -> Result<(), String> {
    let actual = item.and_then(JsonValue::as_str);
    if actual != Some(expected) {
        return Err(format!(
            "adapter result {field:?} mismatch; expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn expect_manifest_json_string(
    item: Option<&JsonValue>,
    field: &str,
    expected: &str,
) -> Result<(), String> {
    if expected == "record-in-result" {
        if item
            .and_then(JsonValue::as_str)
            .is_some_and(|value| !value.is_empty())
        {
            return Ok(());
        }
        return Err(format!(
            "adapter result {field:?} must record a nonempty value"
        ));
    }
    expect_json_string(item, field, expected)
}

fn expect_json_u64(item: Option<&JsonValue>, field: &str, expected: u64) -> Result<(), String> {
    let actual = item.and_then(JsonValue::as_u64);
    if actual != Some(expected) {
        return Err(format!(
            "adapter result {field:?} mismatch; expected {expected}, found {actual:?}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use super::{
        Case, CoverageSpec, Fixture, GateComparator, GateSpec, RunnerFingerprint, coverage_sha256,
        generate_document, generate_fixture, generate_form_contexts, parse_recipe,
        validate_result_outcome,
    };

    fn gating_case() -> Case {
        Case {
            manifest_path: PathBuf::from("manifest.toml"),
            manifest_sha256: "0".repeat(64),
            profile: "TestProfileV1".to_owned(),
            resource_profile: "TestProfileV1".to_owned(),
            feature_set: "test".to_owned(),
            target: "test".to_owned(),
            toolchain: "test".to_owned(),
            allocator: "test".to_owned(),
            runner: "test".to_owned(),
            runner_fingerprint: RunnerFingerprint {
                values: BTreeMap::new(),
                sha256: format!("sha256:{}", "0".repeat(64)),
            },
            id: "PERF-GW-999".to_owned(),
            version: 1,
            fixture_id: "FX-GW-999".to_owned(),
            harness_case: "test_case".to_owned(),
            measurement_sha256: "1".repeat(64),
            gating: true,
            expected_sample_count: 100,
            coverage: None,
            activation_trace_oracle: None,
            activation_cases: None,
            gates: vec![GateSpec {
                source_field: "allocations_max".to_owned(),
                metric: "allocations".to_owned(),
                comparator: GateComparator::AtMost,
                threshold: 0.0,
            }],
            report_metrics: BTreeSet::from(["p95".to_owned()]),
        }
    }

    #[test]
    fn generated_document_is_valid_json_at_exact_size() {
        let document = generate_document("FX-TEST-001", 4096, 46)
            .expect("the requested document size is sufficient");
        assert_eq!(document.len(), 4096);
        let _: serde_json::Value =
            serde_json::from_slice(&document).expect("generated document must be valid JSON");
        let form_contexts = generate_form_contexts("FX-TEST-001", 8, 4)
            .expect("form contexts must generate independently");
        let value: serde_json::Value = serde_json::from_slice(&form_contexts)
            .expect("generated form contexts must be valid JSON");
        let properties = value
            .as_object()
            .expect("forms are distributed over property contexts");
        assert_eq!(properties.len(), 2);
        assert_eq!(
            properties
                .values()
                .map(|property| property["forms"].as_array().map_or(0, Vec::len))
                .sum::<usize>(),
            8
        );
        assert!(properties.values().all(|property| {
            property["forms"]
                .as_array()
                .is_some_and(|forms| forms.len() <= 4)
        }));
    }

    #[test]
    fn fixture_generation_is_deterministic() {
        let fixture = Fixture {
            id: "FX-TEST-001".to_owned(),
            profile: "TestProfileV1".to_owned(),
            harness_case: "test_case".to_owned(),
            version: 1,
            seed: 46,
            recipe: "actors=2;document_bytes=1024;forms=1;payload_bytes=64;subscribers=2"
                .to_owned(),
            content_sha256: String::new(),
            forms_per_context_max: 16,
        };
        let first = generate_fixture(&fixture).expect("fixture must generate");
        let second = generate_fixture(&fixture).expect("fixture must generate again");
        assert_eq!(first, second);
        assert!(first.starts_with(b"clinkz-wot-fixture-v3\0"));
    }

    #[test]
    fn fixture_generation_emits_planning_and_binding_sections() {
        let fixture = Fixture {
            id: "FX-TEST-002".to_owned(),
            profile: "TestProfileV1".to_owned(),
            harness_case: "planning_binding_sections".to_owned(),
            version: 1,
            seed: 47,
            recipe: concat!(
                "binding_artifacts=2;call_bytes=8;driver_bytes=8;",
                "ingress_bytes=8;ingress_items=2;plan_sets=2;",
                "readiness_tokens=2;routes=2;slot_state_bytes=8",
            )
            .to_owned(),
            content_sha256: String::new(),
            forms_per_context_max: 16,
        };
        let generated = generate_fixture(&fixture).expect("fixture must generate");
        for section in [
            "binding-artifact",
            "call-bytes",
            "driver-bytes",
            "ingress-bytes",
            "ingress-item",
            "plan-set",
            "readiness-token",
            "route",
            "slot-state-bytes",
        ] {
            assert!(
                generated
                    .windows(section.len())
                    .any(|window| window == section.as_bytes()),
                "fixture is missing section {section:?}",
            );
        }
    }

    #[test]
    fn recipe_rejects_noncanonical_key_order() {
        let error = parse_recipe("forms=1;document_bytes=1024")
            .expect_err("noncanonical key order must be rejected");
        assert!(error.contains("strictly sorted"));
    }

    #[test]
    fn gating_result_requires_passed_samples_and_all_metrics() {
        let case = gating_case();
        let valid = BTreeMap::from([("allocations", 0.0), ("p95", 4.0)]);
        assert!(validate_result_outcome(&case, "passed", 100, 0, &valid).is_ok());
        assert!(validate_result_outcome(&case, "failed", 100, 0, &valid).is_err());
        assert!(validate_result_outcome(&case, "unavailable", 0, 0, &BTreeMap::new()).is_err());
        assert!(validate_result_outcome(&case, "passed", 0, 0, &valid).is_err());
        assert!(validate_result_outcome(&case, "passed", 100, 1, &valid).is_err());
        assert!(
            validate_result_outcome(&case, "passed", 100, 0, &BTreeMap::from([("p95", 4.0)]))
                .is_err()
        );
        assert!(
            validate_result_outcome(
                &case,
                "passed",
                100,
                0,
                &BTreeMap::from([("allocations", 1.0), ("p95", 4.0)])
            )
            .is_err()
        );
    }

    #[test]
    fn matrix_coverage_requires_samples_for_every_cartesian_cell() {
        let dimensions = vec![
            (
                "handler_kind_pairs".to_owned(),
                vec!["sync-sync".to_owned(), "step-step".to_owned()],
            ),
            (
                "transaction_cases".to_owned(),
                vec!["active".to_owned(), "stop".to_owned(), "replay".to_owned()],
            ),
        ];
        let mut case = gating_case();
        case.coverage = Some(CoverageSpec {
            cell_count: 6,
            sha256: coverage_sha256(&dimensions, 6).expect("coverage digest must generate"),
        });
        let metrics = BTreeMap::from([("allocations", 0.0), ("p95", 4.0)]);
        assert!(validate_result_outcome(&case, "passed", 600, 0, &metrics).is_ok());
        assert!(validate_result_outcome(&case, "passed", 100, 0, &metrics).is_err());
    }
}
