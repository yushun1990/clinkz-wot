//! Validates machine-readable design artifacts that require structured parsing.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use quote::ToTokens;
use syn::visit::{self, Visit};
use syn::{Attribute, Fields, ImplItem, Item as SynItem, ItemImpl, Visibility};
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table};

const ACTIVE_DESIGN_REVISION: &str = "4.9";
const REJECTED_DESIGN_REVISION: &str = "4.8";
const WP000_EVIDENCE_REVISION: &str = "4.6";
const ARCHITECTURE_INTERIM_REVIEW: &str = "architecture-review-03-v4.9-interim";
const ARCHITECTURE_REVIEW_02_PREDECESSOR: &str = "architecture-review-02-v4.8-rejected";

const REQUIRED_MACHINES: &[&str] = &[
    "active-route-driver",
    "binding-call",
    "binding-emission-slot",
    "binding-route",
    "cleanup-task",
    "compiled-plan-set",
    "directory-process",
    "emission-coordinator",
    "expose",
    "handler-async-execution",
    "handler-step-execution",
    "handler-sync-execution",
    "in-flight",
    "lazy-binding-artifact",
    "producer-subscription",
    "route-lifecycle-call",
    "route-readiness",
    "serving-activation-authority",
    "subscription",
    "subscription-driver-slot",
];
const REQUIRED_COMPOSITIONS: &[&str] = &[
    "active-route-acceptance",
    "binding-call-cleanup-transfer",
    "binding-route-lifecycle",
    "binding-route-readiness",
    "consumer-plan-publication",
    "emission-delivery-ownership",
    "handler-cancellation-response",
    "handler-direct-response",
    "producer-late-start-result-transfer",
    "producer-plan-drain",
    "producer-plan-serving-publication",
    "producer-prepublication-failure-response",
    "producer-setup-abort",
    "producer-start-publication",
    "producer-start-result-transfer",
    "producer-teardown-handoff",
    "producer-teardown-result-and-response",
    "producer-terminal-replay-and-release",
    "response-delivery-ownership",
    "subscription-process-cleanup",
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
const HANDLER_VALUE_PRIMITIVES_TRANCHE: &str = "WP-100-HANDLER-VALUE-PRIMITIVES";
const HANDLER_TIME_BLOCKING_SCOPE: &str = "TIME-DOMAIN-AND-DEADLINE";
const HANDLER_VALUE_ENTRY_CHECK: &str = "wp100-handler-value-primitives-entry-check";
const HANDLER_VALUE_REVIEW_ATTESTATION: &str =
    "docs/audits/WP-100-handler-value-primitives-review.toml";
const HANDLER_VALUE_COMPLETION_EVIDENCE: &str =
    "docs/evidence/WP-100-handler-value-primitives.toml";
const HANDLER_VALUE_ADMISSION_REVIEW: &str = "docs/audits/WP-100-handler-value-primitives-entry.md";
const HANDLER_VALUE_PRECHECKS: &[&str] = &[
    "api-ownership-check",
    "architecture-adr-check",
    "resource-profile-check",
    "work-package-dag-check",
    "wp100-amendment-check",
    "wp100-handler-amendment-check",
];

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
        "check-handler-value-primitives-source" => {
            check_handler_value_primitives_source(&root)?;
            println!("design structure check: handler value-primitives source valid");
        }
        "check-handler-value-primitives-entry-state" => {
            let mode = env::args().nth(2).ok_or_else(|| {
                "check-handler-value-primitives-entry-state requires candidate or \
                 admission-ready"
                    .to_owned()
            })?;
            check_handler_value_primitives_entry_state(&root, &mode)?;
            println!("design structure check: handler value-primitives {mode} state valid");
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
                 check-handler-value-primitives-source, \
                 check-handler-value-primitives-entry-state, check-work-packages, \
                 check-governance, check-refactor-ready, or check-handler-entry"
            ));
        }
    }
    Ok(())
}

const HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT: &str = r#"
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CancellationView {
    #[default]
    Active,
    Requested,
}

impl CancellationView {
    pub const fn is_requested(self) -> bool {
        matches!(self, Self::Requested)
    }
}

#[derive(Debug, Eq, PartialEq)]
#[must_use = "a successful acceptance must be consumed by the subscription transaction"]
pub struct SubscriptionAcceptance {
    response: InteractionOutput,
}

impl SubscriptionAcceptance {
    pub const fn new(response: InteractionOutput) -> Self {
        Self { response }
    }

    pub const fn response(&self) -> &InteractionOutput {
        &self.response
    }

    pub fn into_response(self) -> InteractionOutput {
        self.response
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HandlerFootprint {
    retained_bytes: u64,
    pending_call_bytes: u64,
    subscription_bytes: u64,
}

impl HandlerFootprint {
    pub const fn new(
        retained_bytes: u64,
        pending_call_bytes: u64,
        subscription_bytes: u64,
    ) -> Self {
        Self {
            retained_bytes,
            pending_call_bytes,
            subscription_bytes,
        }
    }

    pub const fn retained_bytes(self) -> u64 {
        self.retained_bytes
    }

    pub const fn pending_call_bytes(self) -> u64 {
        self.pending_call_bytes
    }

    pub const fn subscription_bytes(self) -> u64 {
        self.subscription_bytes
    }
}

#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub enum HandlerStep<R> {
    Pending,
    Ready(CoreResult<R>),
}

pub struct StaticHandlerRegistration<'h, H> {
    slot_id: HandlerSlotId,
    handler: &'h H,
    footprint: HandlerFootprint,
}

impl<'h, H> StaticHandlerRegistration<'h, H> {
    pub const fn new(
        slot_id: HandlerSlotId,
        handler: &'h H,
        footprint: HandlerFootprint,
    ) -> Self {
        Self {
            slot_id,
            handler,
            footprint,
        }
    }

    pub const fn slot_id(&self) -> HandlerSlotId {
        self.slot_id
    }

    pub const fn handler(&self) -> &'h H {
        self.handler
    }

    pub const fn footprint(&self) -> HandlerFootprint {
        self.footprint
    }
}

impl<H> Copy for StaticHandlerRegistration<'_, H> {}

impl<H> Clone for StaticHandlerRegistration<'_, H> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<H> core::fmt::Debug for StaticHandlerRegistration<'_, H> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("StaticHandlerRegistration")
            .field("slot_id", &self.slot_id)
            .field("footprint", &self.footprint)
            .finish_non_exhaustive()
    }
}
"#;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HandlerValueSourceProjection {
    values: Vec<HandlerValueTypeProjection>,
    impls: Vec<HandlerValueImplProjection>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
enum HandlerValueTypeProjection {
    Struct {
        name: String,
        visibility: String,
        attributes: Vec<String>,
        generics: String,
        fields: Vec<HandlerValueFieldProjection>,
    },
    Enum {
        name: String,
        visibility: String,
        attributes: Vec<String>,
        generics: String,
        variants: Vec<HandlerValueVariantProjection>,
    },
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HandlerValueFieldProjection {
    name: Option<String>,
    visibility: String,
    attributes: Vec<String>,
    ty: String,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HandlerValueVariantProjection {
    name: String,
    attributes: Vec<String>,
    fields: Vec<HandlerValueFieldProjection>,
    discriminant: Option<String>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HandlerValueImplProjection {
    attributes: Vec<String>,
    defaultness: String,
    unsafety: String,
    generics: String,
    trait_path: Option<String>,
    negative: bool,
    self_ty: String,
    items: Vec<HandlerValueImplItemProjection>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HandlerValueImplItemProjection {
    attributes: Vec<String>,
    visibility: String,
    defaultness: String,
    signature: String,
}

fn check_handler_value_primitives_source(root: &Path) -> Result<(), String> {
    let path = root.join("core/src/handler.rs");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    validate_handler_value_primitives_source(&source)
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn validate_handler_value_primitives_source(source: &str) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| format!("invalid Rust source: {error}"))?;
    let mut forbidden = HandlerValueForbiddenIdentifier::default();
    forbidden.visit_file(&file);
    if !forbidden.identifiers.is_empty() {
        return Err(format!(
            "forbidden allocation/runtime/queue/callback identifiers: {:?}",
            forbidden.identifiers
        ));
    }

    let actual = project_handler_value_source(&file)?;
    let expected_file = syn::parse_file(HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT)
        .map_err(|error| format!("internal handler contract does not parse: {error}"))?;
    let expected = project_handler_value_source(&expected_file)?;
    if actual != expected {
        return Err(format!(
            "five-value source projection mismatch\nexpected: {expected:#?}\nfound: {actual:#?}"
        ));
    }
    Ok(())
}

fn project_handler_value_source(file: &syn::File) -> Result<HandlerValueSourceProjection, String> {
    let mut values = Vec::new();
    let mut impls = Vec::new();
    for item in &file.items {
        match item {
            SynItem::Use(item) if matches!(item.vis, Visibility::Inherited) => {}
            SynItem::Struct(item) => values.push(HandlerValueTypeProjection::Struct {
                name: item.ident.to_string(),
                visibility: token_string(&item.vis),
                attributes: contract_attributes(&item.attrs),
                generics: token_string(&item.generics),
                fields: project_handler_fields(&item.fields),
            }),
            SynItem::Enum(item) => values.push(HandlerValueTypeProjection::Enum {
                name: item.ident.to_string(),
                visibility: token_string(&item.vis),
                attributes: contract_attributes(&item.attrs),
                generics: token_string(&item.generics),
                variants: item
                    .variants
                    .iter()
                    .map(|variant| HandlerValueVariantProjection {
                        name: variant.ident.to_string(),
                        attributes: contract_attributes(&variant.attrs),
                        fields: project_handler_fields(&variant.fields),
                        discriminant: variant
                            .discriminant
                            .as_ref()
                            .map(|(_, expression)| token_string(expression)),
                    })
                    .collect(),
            }),
            SynItem::Impl(item) => impls.push(project_handler_impl(item)?),
            other => {
                return Err(format!(
                    "handler value module contains prohibited top-level item {}",
                    handler_syn_item_kind(other)
                ));
            }
        }
    }
    values.sort();
    impls.sort();
    Ok(HandlerValueSourceProjection { values, impls })
}

fn project_handler_fields(fields: &Fields) -> Vec<HandlerValueFieldProjection> {
    fields
        .iter()
        .map(|field| HandlerValueFieldProjection {
            name: field.ident.as_ref().map(ToString::to_string),
            visibility: token_string(&field.vis),
            attributes: contract_attributes(&field.attrs),
            ty: token_string(&field.ty),
        })
        .collect()
}

fn project_handler_impl(item: &ItemImpl) -> Result<HandlerValueImplProjection, String> {
    let mut items = Vec::new();
    for impl_item in &item.items {
        match impl_item {
            ImplItem::Fn(function) => items.push(HandlerValueImplItemProjection {
                attributes: contract_attributes(&function.attrs),
                visibility: token_string(&function.vis),
                defaultness: token_string(&function.defaultness),
                signature: token_string(&function.sig),
            }),
            _ => {
                return Err(
                    "handler value impl contains an extra associated const/type/macro".to_owned(),
                );
            }
        }
    }
    let (trait_path, negative) = match &item.trait_ {
        Some((negative, path, _)) => (Some(token_string(path)), negative.is_some()),
        None => (None, false),
    };
    Ok(HandlerValueImplProjection {
        attributes: contract_attributes(&item.attrs),
        defaultness: token_string(&item.defaultness),
        unsafety: token_string(&item.unsafety),
        generics: token_string(&item.generics),
        trait_path,
        negative,
        self_ty: token_string(item.self_ty.as_ref()),
        items,
    })
}

fn contract_attributes(attributes: &[Attribute]) -> Vec<String> {
    attributes
        .iter()
        .filter(|attribute| !attribute.path().is_ident("doc"))
        .map(|attribute| token_string(&attribute.meta))
        .collect()
}

fn token_string(tokens: &impl ToTokens) -> String {
    tokens.to_token_stream().to_string()
}

fn handler_syn_item_kind(item: &SynItem) -> &'static str {
    match item {
        SynItem::Const(_) => "const",
        SynItem::ExternCrate(_) => "extern crate",
        SynItem::Fn(_) => "function",
        SynItem::ForeignMod(_) => "extern block",
        SynItem::Macro(_) => "macro",
        SynItem::Mod(_) => "module",
        SynItem::Static(_) => "static",
        SynItem::Trait(_) => "trait",
        SynItem::TraitAlias(_) => "trait alias",
        SynItem::Type(_) => "type alias",
        SynItem::Union(_) => "union",
        SynItem::Use(_) => "public use",
        _ => "unsupported item",
    }
}

#[derive(Default)]
struct HandlerValueForbiddenIdentifier {
    identifiers: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for HandlerValueForbiddenIdentifier {
    fn visit_ident(&mut self, ident: &'ast syn::Ident) {
        let value = ident.to_string();
        let lower = value.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "alloc" | "std" | "arc" | "box" | "vec" | "string" | "tokio" | "async_std" | "smol"
        ) || lower.contains("runtime")
            || lower.contains("executor")
            || lower.contains("queue")
            || lower.contains("callback")
        {
            self.identifiers.insert(value);
        }
        visit::visit_ident(self, ident);
    }
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
    let known_requirements = load_requirement_ids(root)?;

    let mut ids = BTreeSet::new();
    let mut machine_transitions = BTreeMap::new();
    for machine in machines {
        let (id, transitions) = check_machine(machine, &known_requirements, &mut ids)?;
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
    check_compositions(&document, &known_requirements, &machine_transitions)?;
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
        if status == "closed" {
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
            // A blocking audit remains the reason an open gate cannot close. Corrective
            // artifacts added after that audit deliberately remain outside its historical
            // coverage until a later same-revision closure review evaluates the current set.
            open_gates.push(gate.to_owned());
        }

        if gate != "GATE-3"
            && (status != "open"
                || review_id != ARCHITECTURE_INTERIM_REVIEW
                || review.review_type != "audit"
                || review.design_revision != ACTIVE_DESIGN_REVISION
                || review.status != "blocking"
                || !review.artifacts.contains("docs/reviews/review-03.org"))
        {
            return Err(format!(
                "{gate} must remain open under the blocking v{ACTIVE_DESIGN_REVISION} \
                 Architecture Review 03 interim audit"
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
                "GATE-3 must use the passed v4.9 carry-forward review based on v4.6".to_owned(),
            );
        }
    }
    if gate_ids != expected_gates {
        return Err(format!(
            "refactor gate set mismatch; expected {expected_gates:?}, found {gate_ids:?}"
        ));
    }
    let review_ids: BTreeSet<String> = reviews.keys().cloned().collect();
    let expected_review_ids = owned_set(&[
        ARCHITECTURE_REVIEW_02_PREDECESSOR,
        ARCHITECTURE_INTERIM_REVIEW,
        "directory-client-v4.6-review",
    ]);
    if review_ids != expected_review_ids {
        return Err(format!(
            "governance review set mismatch; expected {expected_review_ids:?}, found \
             {review_ids:?}"
        ));
    }
    let expected_referenced_reviews =
        owned_set(&[ARCHITECTURE_INTERIM_REVIEW, "directory-client-v4.6-review"]);
    if referenced_reviews != expected_referenced_reviews {
        return Err(format!(
            "current gate review evidence mismatch; expected {expected_referenced_reviews:?}, \
             found {referenced_reviews:?}"
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
                "HANDLER-VALUE-001",
                "HANDLER-CANCEL-001",
                "HANDLER-CANCEL-002",
                "ERR-TAXONOMY-001",
                "ERR-RETRY-001",
                "CLEANUP-RECORD-001",
                "PLAN-COST-001",
                "PLAN-INDEX-001",
                "PLAN-SET-001",
                "PLAN-ARTIFACT-001",
                "FORM-FINALIZE-001",
                "FORM-OWNER-001",
                "LIFE-EXPOSE-002",
                "BIND-REG-001",
                "BIND-ROUTE-001",
                "BIND-STORAGE-001",
                "BIND-MEM-001",
                "BIND-DELIVERY-001",
                "BIND-IO-001",
                "BIND-OUT-001",
                "BIND-CALL-CANCEL-001",
                "BIND-HOST-CANCEL-001",
            ]),
            owned_set(&[
                "docs/design.md",
                "docs/spec/planning.md",
                "docs/spec/binding-spi.md",
                "docs/architecture/README.md",
                "docs/architecture/20-module-boundaries.md",
                "docs/architecture/30-compiled-plan-lifecycle.md",
                "docs/architecture/40-protocol-binding-spi-and-deployment.md",
                "docs/architecture/50-servient-runtime-lifecycle.md",
                "docs/api-ownership.csv",
                "docs/amendments/WP-100-error-cleanup-v1.md",
                "docs/amendments/WP-100-error-disposition-v1.md",
                "docs/amendments/WP-100-interaction-output-api-v1.md",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADRs/core.org",
                "docs/ADRs/0001-crate-and-module-boundaries.org",
                "docs/ADRs/0002-producer-emission-dispatch.org",
                "docs/ADRs/0003-subscription-driver-ownership.org",
                "docs/ADRs/0004-collection-subscriptions.org",
                "docs/ADRs/0005-outbound-request.org",
                "docs/ADRs/0006-host-binding-call-cancellation.org",
                "docs/ADRs/0007-normative-document-hierarchy.org",
                "docs/ADRs/0008-compiled-plan-lifecycle.org",
                "docs/ADRs/0009-protocol-binding-integration-and-deployment.org",
                "docs/ADRs/0010-server-route-lifecycle.org",
                "docs/ADRs/0011-cleanup-reservation-and-transfer.org",
                "docs/ADRs/0012-serving-activation-permit.org",
                "docs/ADRs/0015-borrowed-resource-profiles-and-linear-work-budgets.org",
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
                "LIFE-EXPOSE-002",
                "LIFE-EXPOSE-003",
                "BIND-PROGRESS-001",
                "STATE-EXPOSE-001",
                "STATE-SUB-001",
                "STATE-BIND-001",
                "STATE-INFLIGHT-001",
                "HANDLE-DROP-001",
                "PRODUCER-EMIT-001",
                "PLAN-COST-001",
                "PLAN-INDEX-001",
                "PLAN-SET-001",
                "PLAN-ARTIFACT-001",
                "FORM-OWNER-001",
                "BIND-ROUTE-001",
                "BIND-STORAGE-001",
                "BIND-DELIVERY-001",
                "BIND-IO-001",
                "BIND-OUT-001",
                "BIND-CALL-CANCEL-001",
                "BIND-HOST-CANCEL-001",
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
                "docs/spec/planning.md",
                "docs/spec/binding-spi.md",
                "docs/architecture/README.md",
                "docs/architecture/10-primary-data-flows.md",
                "docs/architecture/30-compiled-plan-lifecycle.md",
                "docs/architecture/40-protocol-binding-spi-and-deployment.md",
                "docs/architecture/50-servient-runtime-lifecycle.md",
                "docs/state-machines.toml",
                "docs/amendments/WP-100-error-cleanup-v1.md",
                "docs/amendments/WP-100-error-disposition-v1.md",
                "docs/amendments/WP-100-interaction-output-api-v1.md",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADRs/core.org",
                "docs/ADRs/0001-crate-and-module-boundaries.org",
                "docs/ADRs/0002-producer-emission-dispatch.org",
                "docs/ADRs/0003-subscription-driver-ownership.org",
                "docs/ADRs/0004-collection-subscriptions.org",
                "docs/ADRs/0005-outbound-request.org",
                "docs/ADRs/0006-host-binding-call-cancellation.org",
                "docs/ADRs/0007-normative-document-hierarchy.org",
                "docs/ADRs/0008-compiled-plan-lifecycle.org",
                "docs/ADRs/0009-protocol-binding-integration-and-deployment.org",
                "docs/ADRs/0010-server-route-lifecycle.org",
                "docs/ADRs/0011-cleanup-reservation-and-transfer.org",
                "docs/ADRs/0012-serving-activation-permit.org",
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
            owned_set(&[
                "docs/design.md",
                "docs/architecture/00-system-goals-and-context.md",
                "docs/architecture/20-module-boundaries.md",
                "docs/future/directory-service.md",
                "docs/ADRs/0001-crate-and-module-boundaries.org",
                "docs/ADRs/0007-normative-document-hierarchy.org",
                "docs/ADRs/0009-protocol-binding-integration-and-deployment.org",
            ]),
            owned_set(&["directory-client-scope-check"]),
        ),
        "GATE-4" => (
            owned_set(&[
                "RES-LIMIT-001",
                "RES-PROFILE-001",
                "API-RESOURCE-001",
                "ADMIT-MEM-001",
                "PLAN-COST-001",
                "PLAN-SET-001",
                "PLAN-ARTIFACT-001",
                "HANDLER-STORAGE-001",
                "HANDLER-CANCEL-002",
                "LIFE-EXPOSE-002",
                "BIND-REG-001",
                "BIND-ROUTE-001",
                "BIND-STORAGE-001",
                "BIND-MEM-001",
                "BIND-DELIVERY-001",
                "BIND-CALL-CANCEL-001",
                "BIND-HOST-CANCEL-001",
                "CLEANUP-RECORD-001",
            ]),
            owned_set(&[
                "docs/spec/planning.md",
                "docs/spec/binding-spi.md",
                "docs/architecture/30-compiled-plan-lifecycle.md",
                "docs/architecture/40-protocol-binding-spi-and-deployment.md",
                "docs/architecture/50-servient-runtime-lifecycle.md",
                "docs/resource-limits.csv",
                "docs/performance/constrained.toml",
                "docs/performance/gateway.toml",
                "docs/performance/directory.toml",
                "docs/amendments/WP-100-error-cleanup-v1.md",
                "docs/amendments/WP-100-error-disposition-v1.md",
                "docs/amendments/WP-100-interaction-output-api-v1.md",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADRs/core.org",
                "docs/ADRs/0001-crate-and-module-boundaries.org",
                "docs/ADRs/0002-producer-emission-dispatch.org",
                "docs/ADRs/0003-subscription-driver-ownership.org",
                "docs/ADRs/0004-collection-subscriptions.org",
                "docs/ADRs/0005-outbound-request.org",
                "docs/ADRs/0006-host-binding-call-cancellation.org",
                "docs/ADRs/0007-normative-document-hierarchy.org",
                "docs/ADRs/0008-compiled-plan-lifecycle.org",
                "docs/ADRs/0009-protocol-binding-integration-and-deployment.org",
                "docs/ADRs/0010-server-route-lifecycle.org",
                "docs/ADRs/0011-cleanup-reservation-and-transfer.org",
                "docs/ADRs/0012-serving-activation-permit.org",
                "docs/ADRs/0015-borrowed-resource-profiles-and-linear-work-budgets.org",
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
                "PLAN-COST-001",
                "PLAN-INDEX-001",
                "PLAN-SET-001",
                "PLAN-ARTIFACT-001",
                "LIFE-EXPOSE-002",
                "BIND-REG-001",
                "BIND-ROUTE-001",
                "BIND-STORAGE-001",
                "BIND-MEM-001",
                "BIND-DELIVERY-001",
                "BIND-PROGRESS-001",
                "BIND-CALL-CANCEL-001",
                "BIND-HOST-CANCEL-001",
                "PRODUCER-EMIT-001",
                "HANDLER-STORAGE-001",
                "HANDLER-CANCEL-001",
                "HANDLER-CANCEL-002",
            ]),
            owned_set(&[
                "docs/spec/planning.md",
                "docs/spec/binding-spi.md",
                "docs/architecture/10-primary-data-flows.md",
                "docs/architecture/30-compiled-plan-lifecycle.md",
                "docs/architecture/40-protocol-binding-spi-and-deployment.md",
                "docs/architecture/50-servient-runtime-lifecycle.md",
                "docs/state-machines.toml",
                "docs/resource-limits.csv",
                "docs/performance/manifest.schema.json",
                "docs/performance/result.schema.json",
                "docs/performance/fixtures.lock.toml",
                "docs/performance/fixture-generator.md",
                "docs/performance/constrained.toml",
                "docs/performance/gateway.toml",
                "docs/performance/directory.toml",
                "tools/performance-harness",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADRs/core.org",
                "docs/ADRs/0001-crate-and-module-boundaries.org",
                "docs/ADRs/0002-producer-emission-dispatch.org",
                "docs/ADRs/0003-subscription-driver-ownership.org",
                "docs/ADRs/0004-collection-subscriptions.org",
                "docs/ADRs/0005-outbound-request.org",
                "docs/ADRs/0006-host-binding-call-cancellation.org",
                "docs/ADRs/0007-normative-document-hierarchy.org",
                "docs/ADRs/0008-compiled-plan-lifecycle.org",
                "docs/ADRs/0009-protocol-binding-integration-and-deployment.org",
                "docs/ADRs/0010-server-route-lifecycle.org",
                "docs/ADRs/0011-cleanup-reservation-and-transfer.org",
                "docs/ADRs/0012-serving-activation-permit.org",
            ]),
            owned_set(&[
                "architecture-adr-check",
                "state-machine-check",
                "resource-profile-check",
                "performance-contract-check",
                "wp100-handler-amendment-check",
            ]),
        ),
        "GATE-6" => (
            owned_set(&[
                "IMPL-CONFORM-001",
                "PLAN-COST-001",
                "PLAN-INDEX-001",
                "PLAN-SET-001",
                "PLAN-ARTIFACT-001",
                "FORM-OWNER-001",
                "LIFE-EXPOSE-002",
                "BIND-REG-001",
                "BIND-ROUTE-001",
                "BIND-STORAGE-001",
                "BIND-MEM-001",
                "BIND-DELIVERY-001",
                "BIND-IO-001",
                "BIND-OUT-001",
                "BIND-CALL-CANCEL-001",
                "BIND-HOST-CANCEL-001",
                "HANDLER-API-001",
                "HANDLER-SUB-001",
                "HANDLER-VALUE-001",
                "HANDLER-CANCEL-001",
                "HANDLER-CANCEL-002",
                "HANDLER-STORAGE-001",
                "PRODUCER-EMIT-001",
            ]),
            owned_set(&[
                "docs/design.md",
                "docs/spec/planning.md",
                "docs/spec/binding-spi.md",
                "docs/architecture/README.md",
                "docs/architecture/10-primary-data-flows.md",
                "docs/architecture/20-module-boundaries.md",
                "docs/architecture/30-compiled-plan-lifecycle.md",
                "docs/architecture/40-protocol-binding-spi-and-deployment.md",
                "docs/architecture/50-servient-runtime-lifecycle.md",
                "docs/work-packages/index.toml",
                "docs/work-packages",
                "docs/amendments/WP-100-interaction-output-api-v1.md",
                "docs/amendments/WP-100-handler-api-v1.md",
                "docs/ADRs/core.org",
                "docs/ADRs/0001-crate-and-module-boundaries.org",
                "docs/ADRs/0002-producer-emission-dispatch.org",
                "docs/ADRs/0003-subscription-driver-ownership.org",
                "docs/ADRs/0004-collection-subscriptions.org",
                "docs/ADRs/0005-outbound-request.org",
                "docs/ADRs/0006-host-binding-call-cancellation.org",
                "docs/ADRs/0007-normative-document-hierarchy.org",
                "docs/ADRs/0008-compiled-plan-lifecycle.org",
                "docs/ADRs/0009-protocol-binding-integration-and-deployment.org",
                "docs/ADRs/0010-server-route-lifecycle.org",
                "docs/ADRs/0011-cleanup-reservation-and-transfer.org",
                "docs/ADRs/0012-serving-activation-permit.org",
                "docs/ADRs/0013-work-package-scoped-implementation-admission.org",
                "docs/ADRs/0014-transitional-normative-ownership.org",
                "docs/ADRs/0015-borrowed-resource-profiles-and-linear-work-budgets.org",
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
    let known_requirement_sources = load_requirement_source_paths(root)?;
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
        if artifact == "docs/work-packages/index.toml" && schema_version != 2 {
            return Err("work-package index registry row must declare schema version 2".to_owned());
        }
        if !known_requirement_sources.contains(fields[5]) {
            return Err(format!(
                "registered artifact {artifact:?} has unknown requirement source {:?}",
                fields[5]
            ));
        }
    }
    if artifacts.is_empty() {
        return Err(format!("{relative_path} has no data rows"));
    }
    if !known_requirement_sources.is_subset(&artifacts) {
        let missing: Vec<_> = known_requirement_sources.difference(&artifacts).collect();
        return Err(format!(
            "requirement sources are not registered as active artifacts: {missing:?}"
        ));
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
        "wp100-handler-value-primitives-check",
        HANDLER_VALUE_ENTRY_CHECK,
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
        "wp100-handler-value-primitives-check" => Some((
            "tools/check-wp100-handler-value-primitives.sh",
            &["tools/check-wp100-handler-value-primitives.sh"],
            &["pending", "executable"],
        )),
        HANDLER_VALUE_ENTRY_CHECK => Some((
            "tools/check-wp100-handler-value-primitives-entry.sh",
            &[
                "tools/check-wp100-handler-value-primitives-entry.sh",
                "--admission-ready",
            ],
            &["executable"],
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
        if id == ARCHITECTURE_REVIEW_02_PREDECESSOR {
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
                || design_revision != REJECTED_DESIGN_REVISION
                || basis_revision.is_some()
                || gates != expected_gates
                || checks != expected_checks
                || !artifacts.contains("docs/reviews/review-02.org")
            {
                return Err(format!(
                    "Architecture Review 02 does not match the rejected \
                     v{REJECTED_DESIGN_REVISION} predecessor record"
                ));
            }
        }
        if id == ARCHITECTURE_INTERIM_REVIEW {
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
            let required_artifacts = owned_set(&[
                "docs/artifacts.csv",
                "docs/governance.toml",
                "docs/refactor-gates.csv",
                "docs/requirements.csv",
                "docs/spec/README.md",
                "docs/spec/planning.md",
                "docs/spec/binding-spi.md",
                "docs/api-ownership.csv",
                "docs/state-machines.toml",
                "docs/resource-limits.csv",
                "docs/performance/manifest.schema.json",
                "docs/performance/result.schema.json",
                "docs/performance/fixtures.lock.toml",
                "docs/performance/fixture-generator.md",
                "docs/performance/constrained.toml",
                "docs/performance/gateway.toml",
                "docs/performance/directory.toml",
                "tools/performance-harness",
                "docs/work-packages/index.toml",
                "docs/work-packages",
                "docs/ADRs/0010-server-route-lifecycle.org",
                "docs/ADRs/0011-cleanup-reservation-and-transfer.org",
                "docs/architecture/README.md",
                "docs/reviews/review-03.org",
            ]);
            if status != "blocking"
                || review_type != "audit"
                || design_revision != ACTIVE_DESIGN_REVISION
                || basis_revision.is_some()
                || gates != expected_gates
                || checks != expected_checks
                || !required_artifacts.is_subset(&artifacts)
            {
                return Err(format!(
                    "Architecture Review 03 does not match the blocking modular \
                     v{ACTIVE_DESIGN_REVISION} interim audit record"
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
        2,
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

    require_string(
        document.get("admission_policy"),
        "work-package admission_policy",
        "adr-0013-tranche-scoped",
    )?;

    let entry_gates = root_string_set(&document, "global_convergence_gates")?;
    let known_gates = load_first_column(root, "docs/refactor-gates.csv")?;
    if entry_gates != known_gates {
        return Err(format!(
            "global convergence gates mismatch; expected {known_gates:?}, found {entry_gates:?}"
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
        "clinkz-wot-planning",
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

#[derive(Debug)]
struct HandlerBlockingScope {
    requirements: BTreeSet<String>,
    shared_meta_requirements: BTreeSet<String>,
    items: BTreeSet<String>,
    is_blocking: bool,
}

#[derive(Debug)]
struct HandlerValueTrancheState {
    status: String,
    verification_check: String,
    check_status: String,
    requirements: BTreeSet<String>,
    api_items: BTreeSet<String>,
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
    let handler_admission_status =
        string_field(entrypoint, "admission_status", HANDLER_ENTRYPOINT)?;
    if !matches!(handler_admission_status.as_str(), "blocked" | "approved") {
        return Err(format!(
            "{HANDLER_ENTRYPOINT} has invalid admission status {handler_admission_status:?}"
        ));
    }
    let entry_dependencies =
        package_string_set(entrypoint, "depends_on_tranches", HANDLER_ENTRYPOINT)?;
    let expected_entry_dependencies =
        owned_set(&[HANDLER_FOUNDATION_TRANCHE, HANDLER_VALUE_PRIMITIVES_TRANCHE]);
    if entry_dependencies != expected_entry_dependencies {
        return Err(format!(
            "{HANDLER_ENTRYPOINT} dependency mismatch; expected \
             {expected_entry_dependencies:?}, found {entry_dependencies:?}"
        ));
    }
    let entry_blocking_scopes =
        package_string_set(entrypoint, "blocking_scopes", HANDLER_ENTRYPOINT)?;
    if entry_blocking_scopes != owned_set(&[HANDLER_TIME_BLOCKING_SCOPE]) {
        return Err(format!(
            "{HANDLER_ENTRYPOINT} blocking scope mismatch; expected only \
             {HANDLER_TIME_BLOCKING_SCOPE:?}, found {entry_blocking_scopes:?}"
        ));
    }

    let registered_artifacts = load_artifact_registry(root)?;
    let blocking_scope = check_handler_blocking_scope(
        root,
        document,
        known_requirements,
        allowed_owners,
        package_evidence,
        &registered_artifacts,
    )?;

    let tranches = document
        .get("tranche")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "work-package index has no [[tranche]] records".to_owned())?;
    if tranches.len() != 2 {
        return Err(format!(
            "work-package index must define exactly two handler prerequisite tranches; found {}",
            tranches.len()
        ));
    }
    let tranche = tranches
        .iter()
        .find(|table| table.get("id").and_then(Item::as_str) == Some(HANDLER_FOUNDATION_TRANCHE))
        .ok_or_else(|| format!("{HANDLER_FOUNDATION_TRANCHE} tranche record is missing"))?;
    require_table_string(tranche, "id", HANDLER_FOUNDATION_TRANCHE, "handler tranche")?;
    require_table_string(
        tranche,
        "work_package",
        "WP-100",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    for forbidden_field in ["implementation_paths", "contract_artifacts", "entry_check"] {
        if tranche.contains_key(forbidden_field) {
            return Err(format!(
                "{HANDLER_FOUNDATION_TRANCHE} must not define value-tranche field \
                 {forbidden_field:?}"
            ));
        }
    }
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
    let admission_status = string_field(tranche, "admission_status", HANDLER_FOUNDATION_TRANCHE)?;
    if !matches!(
        admission_status.as_str(),
        "review-pending" | "approved" | "revoked"
    ) {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} has invalid admission status {admission_status:?}"
        ));
    }
    require_table_string(
        tranche,
        "impact_status",
        "current",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
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
        "CONSTRAINED-STORAGE-002",
        "CONSTRAINED-WORK-001",
        "RES-LIMIT-001",
        "RES-LIMIT-002",
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
    let artifacts = package_string_set(
        tranche,
        "authoritative_artifacts",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    let expected_artifacts = owned_set(&[
        "docs/ADRs/0013-work-package-scoped-implementation-admission.org",
        "docs/ADRs/0014-transitional-normative-ownership.org",
        "docs/ADRs/0015-borrowed-resource-profiles-and-linear-work-budgets.org",
        "docs/amendments/WP-100-handler-api-v1.md",
        "docs/api-ownership.csv",
        "docs/design.md",
        "docs/resource-limits.csv",
        "docs/work-packages/WP-100-core.md",
        "docs/work-packages/index.toml",
    ]);
    if artifacts != expected_artifacts {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} authoritative artifact set mismatch; expected \
             {expected_artifacts:?}, found {artifacts:?}"
        ));
    }
    check_known_values(
        HANDLER_FOUNDATION_TRANCHE,
        "authoritative artifact",
        &artifacts,
        &registered_artifacts,
    )?;

    let api_items = package_string_set(tranche, "api_items", HANDLER_FOUNDATION_TRANCHE)?;
    let expected_api_items = owned_set(&[
        "PendingWorkClass",
        "ResourceKind",
        "ResourceLimits",
        "StaticResourceProfile",
        "WorkBudget",
        "WorkClass",
    ]);
    if api_items != expected_api_items {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} API item set mismatch; expected \
             {expected_api_items:?}, found {api_items:?}"
        ));
    }
    let ownership_items = load_first_column(root, "docs/api-ownership.csv")?;
    check_known_values(
        HANDLER_FOUNDATION_TRANCHE,
        "API item",
        &api_items,
        &ownership_items,
    )?;

    for empty_field in [
        "state_machines",
        "old_api_removals",
        "performance_workloads",
    ] {
        let values = string_set(
            array_field(tranche, empty_field, HANDLER_FOUNDATION_TRANCHE)?,
            HANDLER_FOUNDATION_TRANCHE,
            empty_field,
        )?;
        if !values.is_empty() {
            return Err(format!(
                "{HANDLER_FOUNDATION_TRANCHE} must have an empty {empty_field:?} scope"
            ));
        }
    }

    let pre_checks = package_string_set(
        tranche,
        "pre_implementation_checks",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    let expected_pre_checks = owned_set(&[
        "api-ownership-check",
        "architecture-adr-check",
        "resource-profile-check",
        "work-package-dag-check",
        "wp100-amendment-check",
        "wp100-handler-amendment-check",
    ]);
    if pre_checks != expected_pre_checks {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} pre-implementation check set mismatch; expected \
             {expected_pre_checks:?}, found {pre_checks:?}"
        ));
    }
    let check_statuses = load_governance_check_statuses(root)?;
    for check in &pre_checks {
        if check_statuses.get(check).map(String::as_str) != Some("executable") {
            return Err(format!(
                "{HANDLER_FOUNDATION_TRANCHE} pre-implementation check {check:?} is not executable"
            ));
        }
    }

    let admission_review = string_field(tranche, "admission_review", HANDLER_FOUNDATION_TRANCHE)?;
    if admission_review != "docs/audits/WP-100-foundation-refresh-entry.md" {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} admission review path is not frozen"
        ));
    }
    validate_relative_path(&admission_review, "tranche admission review")?;
    if admission_status == "approved" {
        if !registered_artifacts.contains(&admission_review) {
            return Err(format!(
                "{HANDLER_FOUNDATION_TRANCHE} approved review {admission_review:?} is not registered"
            ));
        }
        check_tranche_admission_review(root, &admission_review, &artifacts, &pre_checks)?;
    }
    let expected_resource_limit_count = integer_field(
        tranche,
        "expected_resource_limit_count",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    if expected_resource_limit_count != 195 {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} must freeze 195 resource-limit rows"
        ));
    }
    let historical_resource_limit_prefix_count = integer_field(
        tranche,
        "historical_resource_limit_prefix_count",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    if historical_resource_limit_prefix_count != 139 {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} must preserve the 139-field v4.8 prefix"
        ));
    }
    let expected_additive_fields = owned_set(&[
        "plan_sets_per_thing_max",
        "plan_sets_global_max",
        "plan_pins_per_plan_set_max",
        "plan_pins_global_max",
        "logical_plan_bytes_per_thing_max",
        "binding_artifacts_per_thing_max",
        "binding_artifacts_global_max",
        "binding_artifact_bytes_per_item_max",
        "binding_artifact_bytes_per_thing_max",
        "binding_artifact_bytes_global_max",
        "lazy_artifact_negative_bytes_per_item_max",
        "lazy_artifact_negative_bytes_global_max",
        "binding_compiler_cursor_bytes_per_item_max",
        "binding_compiler_cursor_bytes_global_max",
        "lazy_artifact_waiters_per_slot_max",
        "lazy_artifact_waiters_global_max",
        "plan_compile_work_units_per_step_max",
        "plan_reclaim_bytes_per_step_max",
        "binding_routes_per_thing_max",
        "binding_routes_global_max",
        "route_guard_bytes_per_item_max",
        "route_guard_bytes_per_thing_max",
        "route_guard_bytes_global_max",
        "route_readiness_tokens_per_thing_max",
        "route_readiness_tokens_global_max",
        "route_readiness_token_bytes_per_item_max",
        "route_readiness_token_bytes_global_max",
        "route_readiness_timeout_millis_max",
        "route_readiness_steps_max",
        "binding_ingress_items_per_route_max",
        "binding_ingress_items_per_binding_max",
        "binding_ingress_items_global_max",
        "binding_ingress_bytes_per_route_max",
        "binding_ingress_bytes_per_binding_max",
        "binding_ingress_bytes_global_max",
        "host_binding_call_bytes_per_item_max",
        "host_binding_call_bytes_per_binding_max",
        "host_binding_call_bytes_per_thing_max",
        "host_binding_call_bytes_global_max",
        "host_subscription_driver_bytes_per_item_max",
        "host_subscription_driver_bytes_per_thing_max",
        "host_subscription_driver_bytes_global_max",
        "binding_slot_state_bytes_per_item_max",
        "binding_slot_state_bytes_per_thing_max",
        "binding_slot_state_bytes_global_max",
        "binding_poll_temporary_bytes_per_call_max",
        "binding_poll_temporary_bytes_global_max",
        "binding_response_buffer_bytes_per_route_max",
        "binding_response_buffer_bytes_global_max",
        "binding_cancel_buffer_bytes_per_call_max",
        "binding_cancel_buffer_bytes_global_max",
        "cleanup_transfer_slots_global_max",
        "cleanup_transfer_bytes_global_max",
        "binding_wake_leases_global_max",
        "binding_reactor_queue_items_per_binding_max",
        "binding_reactor_queue_bytes_per_binding_max",
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
    let completion_evidence = package_string_set(
        tranche,
        "completion_evidence_keys",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    if completion_evidence != owned_set(&["handler-foundation-refresh"])
        || !package_evidence
            .get("WP-100")
            .is_some_and(|keys| completion_evidence.is_subset(keys))
    {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} has unregistered completion evidence \
             {completion_evidence:?}"
        ));
    }
    let evidence_path = string_field(
        tranche,
        "completion_evidence_path",
        HANDLER_FOUNDATION_TRANCHE,
    )?;
    if evidence_path != "docs/evidence/WP-100-foundation-refresh.toml" {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} evidence path is not frozen"
        ));
    }
    validate_relative_path(&evidence_path, "tranche evidence")?;
    let verification_check = string_field(tranche, "completion_check", HANDLER_FOUNDATION_TRANCHE)?;
    if verification_check != "wp100-foundation-refresh-check" {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} completion check is not frozen"
        ));
    }
    let check_status = check_statuses.get(&verification_check).ok_or_else(|| {
        format!("{HANDLER_FOUNDATION_TRANCHE} references unknown check {verification_check:?}")
    })?;

    let evidence_exists = root.join(&evidence_path).is_file();
    if status == "complete" {
        if check_status != "executable" {
            return Err(format!(
                "{HANDLER_FOUNDATION_TRANCHE} is complete while check \
                 {verification_check:?} is {check_status:?}"
            ));
        }
        if !evidence_exists {
            return Err(format!(
                "{HANDLER_FOUNDATION_TRANCHE} is complete but {evidence_path} is missing"
            ));
        }
        if admission_status != "approved" {
            return Err(format!(
                "{HANDLER_FOUNDATION_TRANCHE} is complete without approved admission"
            ));
        }
        check_tranche_evidence(
            root,
            &evidence_path,
            HANDLER_FOUNDATION_TRANCHE,
            "handler-foundation-refresh",
            &verification_check,
        )?;
    } else if evidence_exists && tranche_evidence_is_passed(root, &evidence_path)? {
        return Err(format!(
            "{HANDLER_FOUNDATION_TRANCHE} remains pending while {evidence_path} claims passed"
        ));
    }

    let value_primitives_tranche = tranches
        .iter()
        .find(|table| {
            table.get("id").and_then(Item::as_str) == Some(HANDLER_VALUE_PRIMITIVES_TRANCHE)
        })
        .ok_or_else(|| format!("{HANDLER_VALUE_PRIMITIVES_TRANCHE} tranche record is missing"))?;
    let value_state = check_handler_value_primitives_tranche(
        root,
        value_primitives_tranche,
        known_requirements,
        allowed_owners,
        allowed_cells,
        package_evidence,
        &registered_artifacts,
        &check_statuses,
        &status,
        &admission_status,
        check_status,
    )?;

    check_handler_scope_partition(
        &value_state.requirements,
        &value_state.api_items,
        &blocking_scope.requirements,
        &blocking_scope.shared_meta_requirements,
        &blocking_scope.items,
    )?;

    if require_handler_entry {
        if blocking_scope.is_blocking {
            return Err(format!(
                "handler entry blocked: scope {HANDLER_TIME_BLOCKING_SCOPE} remains blocking"
            ));
        }
        if handler_admission_status != "approved" {
            return Err(format!(
                "handler entry blocked: {HANDLER_ENTRYPOINT} admission is \
                 {handler_admission_status:?}"
            ));
        }
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
        if value_state.status != "complete" {
            return Err(format!(
                "handler entry blocked: {HANDLER_VALUE_PRIMITIVES_TRANCHE} is \
                 {:?}",
                value_state.status
            ));
        }
        if value_state.check_status != "executable" {
            return Err(format!(
                "handler entry blocked: check {:?} is {:?}",
                value_state.verification_check, value_state.check_status
            ));
        }
        let checker = root.join("tools/check-wp100-handler-value-primitives.sh");
        let status = Command::new(&checker)
            .current_dir(root)
            .status()
            .map_err(|error| {
                format!(
                    "cannot execute handler value-primitives verification {}: {error}",
                    checker.display()
                )
            })?;
        if !status.success() {
            return Err(format!(
                "handler entry blocked: value-primitives verification exited with {status}"
            ));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn check_handler_blocking_scope(
    root: &Path,
    document: &DocumentMut,
    known_requirements: &BTreeSet<String>,
    allowed_owners: &BTreeSet<String>,
    package_evidence: &BTreeMap<String, BTreeSet<String>>,
    registered_artifacts: &BTreeSet<String>,
) -> Result<HandlerBlockingScope, String> {
    let scopes = document
        .get("blocking_scope")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "work-package index has no [[blocking_scope]] record".to_owned())?;
    if scopes.len() != 1 {
        return Err(format!(
            "work-package index must define exactly one handler blocking scope; found {}",
            scopes.len()
        ));
    }
    let scope = scopes
        .iter()
        .next()
        .ok_or_else(|| "handler blocking scope record is missing".to_owned())?;
    require_exact_table_fields(
        scope,
        HANDLER_TIME_BLOCKING_SCOPE,
        &[
            "id",
            "work_package",
            "status",
            "record_kind",
            "impact_status",
            "blocks_entrypoints",
            "affected_requirements",
            "shared_meta_requirements",
            "affected_owner_packages",
            "affected_items",
            "blocking_topic",
            "impact_review",
            "impacted_evidence_path",
            "impacted_evidence_key",
            "future_tranche_identity",
            "future_tranche_ownership",
            "future_tranche_dependencies",
            "future_tranche_completion_contract",
            "evidence_disposition",
        ],
    )?;
    require_table_string(
        scope,
        "id",
        HANDLER_TIME_BLOCKING_SCOPE,
        "handler blocking scope",
    )?;
    require_table_string(scope, "work_package", "WP-100", HANDLER_TIME_BLOCKING_SCOPE)?;
    require_table_string(scope, "status", "discussing", HANDLER_TIME_BLOCKING_SCOPE)?;
    require_table_string(
        scope,
        "record_kind",
        "impact-placeholder",
        HANDLER_TIME_BLOCKING_SCOPE,
    )?;
    require_table_string(
        scope,
        "impact_status",
        "blocking",
        HANDLER_TIME_BLOCKING_SCOPE,
    )?;

    let entrypoints = package_string_set(scope, "blocks_entrypoints", HANDLER_TIME_BLOCKING_SCOPE)?;
    if entrypoints != owned_set(&[HANDLER_ENTRYPOINT]) {
        return Err(format!(
            "{HANDLER_TIME_BLOCKING_SCOPE} must block only {HANDLER_ENTRYPOINT}"
        ));
    }

    let requirements =
        package_string_set(scope, "affected_requirements", HANDLER_TIME_BLOCKING_SCOPE)?;
    check_known_values(
        HANDLER_TIME_BLOCKING_SCOPE,
        "affected requirement",
        &requirements,
        known_requirements,
    )?;
    let expected_requirements = owned_set(&[
        "API-SOURCE-TIME-001",
        "API-SURFACE-001",
        "CLEANUP-RECORD-001",
        "HANDLER-CANCEL-001",
        "HANDLER-CANCEL-002",
        "TIME-001",
    ]);
    if requirements != expected_requirements {
        return Err(format!(
            "{HANDLER_TIME_BLOCKING_SCOPE} affected requirement set mismatch; expected \
             {expected_requirements:?}, found {requirements:?}"
        ));
    }
    let shared_meta_requirements = package_string_set(
        scope,
        "shared_meta_requirements",
        HANDLER_TIME_BLOCKING_SCOPE,
    )?;
    if shared_meta_requirements != owned_set(&["API-SURFACE-001"])
        || !shared_meta_requirements.is_subset(&requirements)
    {
        return Err(format!(
            "{HANDLER_TIME_BLOCKING_SCOPE} shared meta requirements must be exactly \
             API-SURFACE-001"
        ));
    }

    let owners = package_string_set(
        scope,
        "affected_owner_packages",
        HANDLER_TIME_BLOCKING_SCOPE,
    )?;
    check_known_values(
        HANDLER_TIME_BLOCKING_SCOPE,
        "affected owner package",
        &owners,
        allowed_owners,
    )?;
    if owners != owned_set(&["clinkz-wot-core", "clinkz-wot-foundation"]) {
        return Err(format!(
            "{HANDLER_TIME_BLOCKING_SCOPE} affected owner package set is not frozen"
        ));
    }

    let items = package_string_set(scope, "affected_items", HANDLER_TIME_BLOCKING_SCOPE)?;
    let expected_items = owned_set(&[
        "ClockId",
        "MonotonicInstant",
        "RuntimeClock",
        "SourceTimestamp",
        "Deadline",
        "CleanupRecord timing validation",
    ]);
    if items != expected_items {
        return Err(format!(
            "{HANDLER_TIME_BLOCKING_SCOPE} affected item set mismatch; expected \
             {expected_items:?}, found {items:?}"
        ));
    }
    let ownership_items = load_first_column(root, "docs/api-ownership.csv")?;
    let ownership_backed_items = owned_set(&[
        "ClockId",
        "MonotonicInstant",
        "RuntimeClock",
        "SourceTimestamp",
        "Deadline",
    ]);
    check_known_values(
        HANDLER_TIME_BLOCKING_SCOPE,
        "affected API item",
        &ownership_backed_items,
        &ownership_items,
    )?;

    for (field, expected) in [
        (
            "blocking_topic",
            "workspace/0007-time-domain-and-deadline.md",
        ),
        ("impact_review", "docs/reviews/review-06.org"),
        ("impacted_evidence_path", "docs/evidence/WP-000.toml"),
    ] {
        let value = string_field(scope, field, HANDLER_TIME_BLOCKING_SCOPE)?;
        if value != expected {
            return Err(format!(
                "{HANDLER_TIME_BLOCKING_SCOPE} {field:?} mismatch; expected {expected:?}, \
                 found {value:?}"
            ));
        }
        validate_relative_path(&value, HANDLER_TIME_BLOCKING_SCOPE)?;
        if !registered_artifacts.contains(&value) || !root.join(&value).is_file() {
            return Err(format!(
                "{HANDLER_TIME_BLOCKING_SCOPE} {field:?} is not registered and present: \
                 {value:?}"
            ));
        }
    }

    let evidence_key = string_field(scope, "impacted_evidence_key", HANDLER_TIME_BLOCKING_SCOPE)?;
    if evidence_key != "time-and-generation-api"
        || !package_evidence
            .get("WP-000")
            .is_some_and(|keys| keys.contains(&evidence_key))
    {
        return Err(format!(
            "{HANDLER_TIME_BLOCKING_SCOPE} impacted evidence key is unknown: {evidence_key:?}"
        ));
    }
    check_work_package_evidence_key(root, "docs/evidence/WP-000.toml", &evidence_key)?;

    for field in [
        "future_tranche_identity",
        "future_tranche_ownership",
        "future_tranche_dependencies",
        "future_tranche_completion_contract",
        "evidence_disposition",
    ] {
        require_table_string(scope, field, "not-frozen", HANDLER_TIME_BLOCKING_SCOPE)?;
    }

    Ok(HandlerBlockingScope {
        requirements,
        shared_meta_requirements,
        items,
        is_blocking: true,
    })
}

fn check_handler_scope_partition(
    value_requirements: &BTreeSet<String>,
    value_api_items: &BTreeSet<String>,
    blocking_requirements: &BTreeSet<String>,
    shared_meta_requirements: &BTreeSet<String>,
    blocking_items: &BTreeSet<String>,
) -> Result<(), String> {
    let requirement_overlap: BTreeSet<String> = value_requirements
        .intersection(blocking_requirements)
        .cloned()
        .collect();
    if &requirement_overlap != shared_meta_requirements {
        return Err(format!(
            "handler value/time requirement intersection must equal declared shared meta \
             requirements {shared_meta_requirements:?}; found {requirement_overlap:?}"
        ));
    }
    let item_overlap: BTreeSet<String> = value_api_items
        .intersection(blocking_items)
        .cloned()
        .collect();
    if !item_overlap.is_empty() {
        return Err(format!(
            "handler value and time blocking API scopes overlap at {item_overlap:?}"
        ));
    }
    Ok(())
}

fn check_work_package_evidence_key(
    root: &Path,
    relative_path: &str,
    expected_key: &str,
) -> Result<(), String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    let records = document
        .get("evidence")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("{relative_path} has no [[evidence]] records"))?;
    let matches = records
        .iter()
        .filter(|record| record.get("key").and_then(Item::as_str) == Some(expected_key))
        .count();
    if matches != 1 {
        return Err(format!(
            "{relative_path} must contain exactly one evidence key {expected_key:?}; found \
             {matches}"
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn check_handler_value_primitives_tranche(
    root: &Path,
    tranche: &Table,
    known_requirements: &BTreeSet<String>,
    allowed_owners: &BTreeSet<String>,
    allowed_cells: &BTreeSet<String>,
    package_evidence: &BTreeMap<String, BTreeSet<String>>,
    registered_artifacts: &BTreeSet<String>,
    check_statuses: &BTreeMap<String, String>,
    foundation_status: &str,
    foundation_admission_status: &str,
    foundation_check_status: &str,
) -> Result<HandlerValueTrancheState, String> {
    require_table_string(
        tranche,
        "id",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
        "handler value-primitives tranche",
    )?;
    require_table_string(
        tranche,
        "work_package",
        "WP-100",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;

    let sequence = integer_field(tranche, "sequence", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    if sequence != 120 {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} sequence mismatch; expected 120, found \
             {sequence}"
        ));
    }
    let status = string_field(tranche, "status", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    let admission_status = string_field(
        tranche,
        "admission_status",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if !handler_value_status_pair_is_valid(&status, &admission_status) {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} has invalid status/admission pair \
             {status:?}/{admission_status:?}"
        ));
    }
    require_table_string(
        tranche,
        "impact_status",
        "current",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;

    let dependencies = package_string_set(tranche, "depends_on", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    if dependencies != owned_set(&[HANDLER_FOUNDATION_TRANCHE]) {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} must depend only on \
             {HANDLER_FOUNDATION_TRANCHE}"
        ));
    }
    let blocked = package_string_set(
        tranche,
        "blocks_entrypoints",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if blocked != owned_set(&[HANDLER_ENTRYPOINT]) {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} must block only {HANDLER_ENTRYPOINT}"
        ));
    }

    let expected_requirements = owned_set(&["API-SURFACE-001", "HANDLER-VALUE-001"]);
    let requirements =
        package_string_set(tranche, "requirements", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    check_known_values(
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
        "requirement",
        &requirements,
        known_requirements,
    )?;
    if requirements != expected_requirements {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} requirement set mismatch; expected \
             {expected_requirements:?}, found {requirements:?}"
        ));
    }

    let owners = package_string_set(tranche, "owner_packages", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    check_known_values(
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
        "owner package",
        &owners,
        allowed_owners,
    )?;
    if owners != owned_set(&["clinkz-wot-core"]) {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} owner package set is not frozen"
        ));
    }

    let cells = package_string_set(tranche, "feature_cells", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    check_known_values(
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
        "feature cell",
        &cells,
        allowed_cells,
    )?;
    if cells != owned_set(&["no-default", "async-no-std", "std"]) {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} feature-cell set is not frozen"
        ));
    }

    let artifacts = package_string_set(
        tranche,
        "authoritative_artifacts",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    let expected_artifacts = owned_set(&[
        "docs/ADRs/0013-work-package-scoped-implementation-admission.org",
        "docs/ADRs/0014-transitional-normative-ownership.org",
        "docs/amendments/WP-100-handler-api-v1.md",
        "docs/api-ownership.csv",
        "docs/design.md",
        "docs/requirements.csv",
        "docs/work-packages/WP-100-core.md",
        "docs/work-packages/index.toml",
    ]);
    if artifacts != expected_artifacts {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} authoritative artifact set mismatch; expected \
             {expected_artifacts:?}, found {artifacts:?}"
        ));
    }
    check_known_values(
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
        "authoritative artifact",
        &artifacts,
        registered_artifacts,
    )?;

    let api_items = package_string_set(tranche, "api_items", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    let expected_api_items = owned_set(&[
        "CancellationView",
        "HandlerFootprint",
        "HandlerStep",
        "StaticHandlerRegistration",
        "SubscriptionAcceptance",
    ]);
    if api_items != expected_api_items {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} API item set mismatch; expected \
             {expected_api_items:?}, found {api_items:?}"
        ));
    }
    let ownership_items = load_first_column(root, "docs/api-ownership.csv")?;
    check_known_values(
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
        "API item",
        &api_items,
        &ownership_items,
    )?;

    let implementation_paths = package_string_set(
        tranche,
        "implementation_paths",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    let expected_implementation_paths = owned_set(&["core/src/handler.rs", "core/src/lib.rs"]);
    if implementation_paths != expected_implementation_paths {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} implementation path set mismatch; expected \
             {expected_implementation_paths:?}, found {implementation_paths:?}"
        ));
    }
    for path in &implementation_paths {
        validate_relative_path(path, "handler value implementation path")?;
    }

    let contract_artifacts = package_string_set(
        tranche,
        "contract_artifacts",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    let expected_contract_artifacts = owned_set(&[
        "tools/check-wp100-handler-value-primitives-entry.sh",
        "tools/check-wp100-handler-value-primitives.sh",
        "tools/design-check/Cargo.toml",
        "tools/design-check/src/main.rs",
        "tools/compile-contracts/wp100-handler-value-primitives/Cargo.toml",
        "tools/compile-contracts/wp100-handler-value-primitives/Cargo.lock",
        "tools/compile-contracts/wp100-handler-value-primitives/src/lib.rs",
        "tools/compile-contracts/wp100-handler-value-primitives/tests/semantics.rs",
        "tools/compile-contracts/wp100-handler-value-primitives/ui/private-subscription-acceptance.rs",
        "tools/compile-contracts/wp100-handler-value-primitives/ui/private-handler-footprint.rs",
        "tools/compile-contracts/wp100-handler-value-primitives/ui/private-static-handler-registration.rs",
        "tools/compile-contracts/wp100-handler-value-primitives/ui/must-use-subscription-acceptance.rs",
        "tools/compile-contracts/wp100-handler-value-primitives/ui/must-use-handler-step.rs",
    ]);
    if contract_artifacts != expected_contract_artifacts {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} contract artifact set mismatch; expected \
             {expected_contract_artifacts:?}, found {contract_artifacts:?}"
        ));
    }
    check_known_values(
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
        "contract artifact",
        &contract_artifacts,
        registered_artifacts,
    )?;
    for artifact in &contract_artifacts {
        if !root.join(artifact).is_file() {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} contract artifact is missing: {artifact:?}"
            ));
        }
    }

    let candidate_base_ref = string_field(
        tranche,
        "candidate_base_ref",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if candidate_base_ref != "8c89e9346f424923ef3247dd1c402d5ab141c203" {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} candidate base ref is not frozen"
        ));
    }
    check_git_commit_is_ancestor(root, &candidate_base_ref, "candidate base ref")?;
    let candidate_paths =
        package_string_set(tranche, "candidate_paths", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    check_handler_candidate_paths(
        root,
        &candidate_paths,
        &implementation_paths,
        registered_artifacts,
    )?;

    for empty_field in [
        "state_machines",
        "old_api_removals",
        "performance_workloads",
    ] {
        let values = string_set(
            array_field(tranche, empty_field, HANDLER_VALUE_PRIMITIVES_TRANCHE)?,
            HANDLER_VALUE_PRIMITIVES_TRANCHE,
            empty_field,
        )?;
        if !values.is_empty() {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} must have an empty {empty_field:?} scope"
            ));
        }
    }

    let pre_checks = package_string_set(
        tranche,
        "pre_implementation_checks",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    let expected_pre_checks = owned_set(HANDLER_VALUE_PRECHECKS);
    if pre_checks != expected_pre_checks {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} pre-implementation check set mismatch; \
             expected {expected_pre_checks:?}, found {pre_checks:?}"
        ));
    }
    for check in &pre_checks {
        if check_statuses.get(check).map(String::as_str) != Some("executable") {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} pre-implementation check {check:?} is not \
                 executable"
            ));
        }
    }

    let entry_check = string_field(tranche, "entry_check", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    if entry_check != HANDLER_VALUE_ENTRY_CHECK
        || check_statuses.get(&entry_check).map(String::as_str) != Some("executable")
    {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} entry check must be the executable \
             {HANDLER_VALUE_ENTRY_CHECK:?}"
        ));
    }

    let admission_review = string_field(
        tranche,
        "admission_review",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if admission_review != HANDLER_VALUE_ADMISSION_REVIEW {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} admission review path is not frozen"
        ));
    }
    validate_relative_path(&admission_review, "tranche admission review")?;
    if !registered_artifacts.contains(&admission_review) || !root.join(&admission_review).is_file()
    {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} admission review is not registered and \
             present: {admission_review:?}"
        ));
    }
    check_handler_value_audit_state(root, &admission_review, &admission_status)?;
    let attestation_ref = if admission_status == "approved" {
        if foundation_status != "complete"
            || foundation_admission_status != "approved"
            || foundation_check_status != "executable"
        {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} approval requires a complete, approved, \
                 current foundation predecessor with executable evidence check"
            ));
        }
        check_handler_value_primitives_admission_review(
            root,
            &admission_review,
            &artifacts,
            &pre_checks,
        )?;
        let attestation_path = string_field(
            tranche,
            "review_attestation",
            HANDLER_VALUE_PRIMITIVES_TRANCHE,
        )?;
        if attestation_path != HANDLER_VALUE_REVIEW_ATTESTATION
            || !registered_artifacts.contains(&attestation_path)
            || !root.join(&attestation_path).is_file()
        {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} review attestation must be registered at \
                 {HANDLER_VALUE_REVIEW_ATTESTATION:?}"
            ));
        }
        let attestation_ref = string_field(
            tranche,
            "review_attestation_ref",
            HANDLER_VALUE_PRIMITIVES_TRANCHE,
        )?;
        check_handler_review_attestation(
            root,
            &attestation_path,
            &attestation_ref,
            &candidate_base_ref,
            &candidate_paths,
        )?;
        Some(attestation_ref)
    } else {
        for forbidden_field in ["review_attestation", "review_attestation_ref"] {
            if tranche.contains_key(forbidden_field) {
                return Err(format!(
                    "{HANDLER_VALUE_PRIMITIVES_TRANCHE} review-pending state must not define \
                     {forbidden_field:?}"
                ));
            }
        }
        None
    };

    let admission_ref = if status == "pending" {
        if tranche.contains_key("admission_ref") {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} pending state must not define \
                 admission_ref"
            ));
        }
        None
    } else {
        let admission_ref = string_field(
            tranche,
            "admission_ref",
            HANDLER_VALUE_PRIMITIVES_TRANCHE,
        )?;
        let attestation_ref = attestation_ref.as_deref().ok_or_else(|| {
            format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} non-pending state has no review \
                 attestation ref"
            )
        })?;
        check_handler_admission_commit(root, &admission_ref, attestation_ref)?;
        if status == "in-progress" {
            check_handler_progress_checkpoint_state(root, &admission_ref, &implementation_paths)?;
        }
        Some(admission_ref)
    };

    let completion_evidence = package_string_set(
        tranche,
        "completion_evidence_keys",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if completion_evidence != owned_set(&["handler-value-primitives"])
        || !package_evidence
            .get("WP-100")
            .is_some_and(|keys| completion_evidence.is_subset(keys))
    {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} has unregistered completion evidence \
             {completion_evidence:?}"
        ));
    }
    let evidence_path = string_field(
        tranche,
        "completion_evidence_path",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if evidence_path != HANDLER_VALUE_COMPLETION_EVIDENCE {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} evidence path is not frozen"
        ));
    }
    validate_relative_path(&evidence_path, "tranche evidence")?;

    let verification_check = string_field(
        tranche,
        "completion_check",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if verification_check != "wp100-handler-value-primitives-check" {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} completion check is not frozen"
        ));
    }
    let check_status = check_statuses
        .get(&verification_check)
        .ok_or_else(|| {
            format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} references unknown check \
                 {verification_check:?}"
            )
        })?
        .clone();

    let evidence_exists = root.join(&evidence_path).is_file();
    if status == "complete" {
        if check_status != "executable" {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} is complete while check \
                 {verification_check:?} is {check_status:?}"
            ));
        }
        if !evidence_exists {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} is complete but {evidence_path} is missing"
            ));
        }
        if admission_status != "approved" {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} is complete without approved admission"
            ));
        }
        if !registered_artifacts.contains(&evidence_path) {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} completion evidence is not registered: \
                 {evidence_path:?}"
            ));
        }
        check_handler_value_completion_evidence(
            root,
            &evidence_path,
            &verification_check,
            admission_ref.as_deref().ok_or_else(|| {
                format!(
                    "{HANDLER_VALUE_PRIMITIVES_TRANCHE} complete state has no admission_ref"
                )
            })?,
        )?;
    } else if evidence_exists && tranche_evidence_is_passed(root, &evidence_path)? {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} is not complete while {evidence_path} claims \
             passed"
        ));
    }

    Ok(HandlerValueTrancheState {
        status,
        verification_check,
        check_status,
        requirements,
        api_items,
    })
}

fn handler_value_status_pair_is_valid(status: &str, admission_status: &str) -> bool {
    matches!(
        (status, admission_status),
        ("pending", "review-pending")
            | ("pending", "approved")
            | ("in-progress", "approved")
            | ("complete", "approved")
    )
}

fn check_handler_candidate_paths(
    root: &Path,
    candidate_paths: &BTreeSet<String>,
    implementation_paths: &BTreeSet<String>,
    registered_artifacts: &BTreeSet<String>,
) -> Result<(), String> {
    if candidate_paths.is_empty() {
        return Err(format!(
            "{HANDLER_VALUE_PRIMITIVES_TRANCHE} candidate path set must not be empty"
        ));
    }
    let design_checker_source_is_registered = registered_artifacts
        .contains("tools/design-check/src/main.rs")
        || registered_artifacts.contains("tools/design-check/Cargo.toml");
    for path in candidate_paths {
        validate_relative_path(path, "handler value candidate path")?;
        if implementation_paths.contains(path) {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} candidate path enters implementation scope: \
                 {path:?}"
            ));
        }
        let explicitly_governed = path.starts_with("docs/")
            || path.starts_with("workspace/")
            || matches!(
                path.as_str(),
                "AGENTS.md" | "PLAN.md" | "Cargo.toml" | "Cargo.lock"
            )
            || (path == "tools/design-check/src/main.rs" && design_checker_source_is_registered);
        if !registered_artifacts.contains(path) && !explicitly_governed {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} candidate path is outside registered \
                 contract/governance scope: {path:?}"
            ));
        }
        if !root.join(path).is_file() {
            return Err(format!(
                "{HANDLER_VALUE_PRIMITIVES_TRANCHE} candidate path is missing: {path:?}"
            ));
        }
    }
    Ok(())
}

fn check_handler_value_audit_state(
    root: &Path,
    relative_path: &str,
    admission_status: &str,
) -> Result<(), String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    validate_handler_value_audit_source(&source, admission_status)
        .map_err(|error| format!("{relative_path}: {error}"))
}

fn validate_handler_value_audit_source(source: &str, admission_status: &str) -> Result<(), String> {
    let (expected_status, expected_verdict, forbidden_status, forbidden_verdict) =
        match admission_status {
            "review-pending" => (
                "Status: Pending",
                "Verdict: Independent re-review pending",
                "Status: Passed",
                "Verdict: Implementation-ready",
            ),
            "approved" => (
                "Status: Passed",
                "Verdict: Implementation-ready",
                "Status: Pending",
                "Verdict: Independent re-review pending",
            ),
            other => {
                return Err(format!(
                    "unsupported handler value admission state {other:?}"
                ));
            }
        };
    for marker in [expected_status, expected_verdict] {
        if source.lines().filter(|line| *line == marker).count() != 1 {
            return Err(format!("audit must contain exactly one {marker:?}"));
        }
    }
    for marker in [forbidden_status, forbidden_verdict] {
        if source.lines().any(|line| line == marker) {
            return Err(format!("audit contains contradictory marker {marker:?}"));
        }
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
struct HandlerReviewAttestationProjection {
    reviewed_ref: String,
}

fn parse_handler_review_attestation(
    source: &str,
) -> Result<HandlerReviewAttestationProjection, String> {
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid handler review attestation: {error}"))?;
    require_exact_table_fields(
        document.as_table(),
        "handler review attestation",
        &[
            "schema_version",
            "design_revision",
            "tranche",
            "status",
            "reviewer_attestation_kind",
            "reviewer_id",
            "reviewed_ref",
            "precheck",
        ],
    )?;
    require_integer(
        document.get("schema_version"),
        "handler review attestation schema_version",
        1,
    )?;
    require_string(
        document.get("design_revision"),
        "handler review attestation design_revision",
        ACTIVE_DESIGN_REVISION,
    )?;
    require_string(
        document.get("tranche"),
        "handler review attestation tranche",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    require_string(
        document.get("status"),
        "handler review attestation status",
        "passed",
    )?;
    require_string(
        document.get("reviewer_attestation_kind"),
        "handler review attestation kind",
        "separate-agent-task",
    )?;
    let reviewer_id = document
        .get("reviewer_id")
        .and_then(Item::as_str)
        .ok_or_else(|| "handler review attestation has no reviewer_id".to_owned())?;
    if !reviewer_id.starts_with("codex-agent:/root/")
        || reviewer_id == "codex-agent:/root/"
        || reviewer_id == "codex-agent:/root"
    {
        return Err(format!(
            "handler review attestation reviewer is not a child task: {reviewer_id:?}"
        ));
    }
    let reviewed_ref = document
        .get("reviewed_ref")
        .and_then(Item::as_str)
        .ok_or_else(|| "handler review attestation has no reviewed_ref".to_owned())?
        .to_owned();
    require_full_commit_id(&reviewed_ref, "reviewed_ref")?;

    let checks = document
        .get("precheck")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "handler review attestation has no [[precheck]] records".to_owned())?;
    if checks.len() != HANDLER_VALUE_PRECHECKS.len() {
        return Err(format!(
            "handler review attestation must record exactly {} prechecks; found {}",
            HANDLER_VALUE_PRECHECKS.len(),
            checks.len()
        ));
    }
    let mut ids = BTreeSet::new();
    for check in checks {
        require_exact_table_fields(check, "handler review precheck", &["id", "result"])?;
        let id = string_field(check, "id", "handler review precheck")?;
        require_table_string(check, "result", "passed", &id)?;
        if !ids.insert(id.clone()) {
            return Err(format!(
                "handler review attestation duplicates precheck {id:?}"
            ));
        }
    }
    let expected_ids = owned_set(HANDLER_VALUE_PRECHECKS);
    if ids != expected_ids {
        return Err(format!(
            "handler review attestation precheck set mismatch; expected {expected_ids:?}, \
             found {ids:?}"
        ));
    }
    Ok(HandlerReviewAttestationProjection { reviewed_ref })
}

fn check_handler_review_attestation(
    root: &Path,
    relative_path: &str,
    attestation_ref: &str,
    candidate_base_ref: &str,
    candidate_paths: &BTreeSet<String>,
) -> Result<(), String> {
    require_full_commit_id(attestation_ref, "review_attestation_ref")?;
    check_git_commit_is_ancestor(root, attestation_ref, "review_attestation_ref")?;
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let projection = parse_handler_review_attestation(&source)?;
    check_git_commit_is_ancestor(root, &projection.reviewed_ref, "reviewed_ref")?;

    require_git_single_parent(
        root,
        &projection.reviewed_ref,
        candidate_base_ref,
        "reviewed candidate commit",
    )?;
    let reviewed_paths = git_changed_paths_between(
        root,
        candidate_base_ref,
        &projection.reviewed_ref,
        "reviewed candidate diff",
    )?;
    if &reviewed_paths != candidate_paths {
        return Err(format!(
            "reviewed candidate diff mismatch; expected {candidate_paths:?}, found \
             {reviewed_paths:?}"
        ));
    }

    require_git_single_parent(
        root,
        attestation_ref,
        &projection.reviewed_ref,
        "review attestation commit",
    )?;
    let review_paths = git_changed_paths_between(
        root,
        &projection.reviewed_ref,
        attestation_ref,
        "review attestation diff",
    )?;
    let expected_review_paths = owned_set(&["docs/artifacts.csv", relative_path]);
    if review_paths != expected_review_paths {
        return Err(format!(
            "review attestation commit changed files outside its boundary; expected \
             {expected_review_paths:?}, found {review_paths:?}"
        ));
    }

    let object = format!("{attestation_ref}:{relative_path}");
    let committed_source = git_output_bytes(
        root,
        &["show", &object],
        "read committed review attestation",
    )?;
    let worktree_source =
        fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if committed_source != worktree_source {
        return Err(format!(
            "worktree review attestation {relative_path:?} differs from \
             review_attestation_ref"
        ));
    }
    Ok(())
}

fn check_handler_value_completion_evidence(
    root: &Path,
    relative_path: &str,
    verification_check: &str,
    admission_ref: &str,
) -> Result<(), String> {
    check_tranche_evidence(
        root,
        relative_path,
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
        "handler-value-primitives",
        verification_check,
    )?;
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    require_string(
        document.get("verification_command"),
        "handler value evidence verification_command",
        "tools/check-wp100-handler-value-primitives.sh",
    )?;
    let implementation_ref = document
        .get("implementation_ref")
        .and_then(Item::as_str)
        .ok_or_else(|| format!("{relative_path} has no implementation_ref"))?;
    require_full_commit_id(implementation_ref, "handler value implementation_ref")?;
    check_git_commit_is_ancestor(root, implementation_ref, "handler value implementation_ref")?;
    let progress_ref = git_single_parent(
        root,
        implementation_ref,
        "handler value implementation commit",
    )?;
    require_git_single_parent(
        root,
        &progress_ref,
        admission_ref,
        "handler value progress checkpoint",
    )?;
    let progress_paths = git_changed_paths_between(
        root,
        admission_ref,
        &progress_ref,
        "handler value progress checkpoint diff",
    )?;
    let expected_progress_paths =
        owned_set(&["PLAN.md", "docs/work-packages/index.toml"]);
    if progress_paths != expected_progress_paths {
        return Err(format!(
            "handler value progress checkpoint path mismatch; expected \
             {expected_progress_paths:?}, found {progress_paths:?}"
        ));
    }
    let implementation_paths = git_commit_changed_paths(
        root,
        implementation_ref,
        "handler value implementation commit",
    )?;
    let expected_paths = owned_set(&["core/src/handler.rs", "core/src/lib.rs"]);
    if implementation_paths != expected_paths {
        return Err(format!(
            "handler value implementation commit path mismatch; expected {expected_paths:?}, \
             found {implementation_paths:?}"
        ));
    }

    let checker = root.join("tools/check-wp100-handler-value-primitives.sh");
    let status = Command::new(&checker)
        .current_dir(root)
        .status()
        .map_err(|error| {
            format!(
                "cannot execute handler value completion checker {}: {error}",
                checker.display()
            )
        })?;
    if !status.success() {
        return Err(format!(
            "handler value completion checker exited with {status}"
        ));
    }
    Ok(())
}

fn check_handler_admission_commit(
    root: &Path,
    admission_ref: &str,
    attestation_ref: &str,
) -> Result<(), String> {
    require_full_commit_id(admission_ref, "admission_ref")?;
    check_git_commit_is_ancestor(root, admission_ref, "admission_ref")?;
    require_git_single_parent(
        root,
        admission_ref,
        attestation_ref,
        "handler value admission commit",
    )?;
    let admission_paths = git_changed_paths_between(
        root,
        attestation_ref,
        admission_ref,
        "handler value admission diff",
    )?;
    let expected_paths = owned_set(&[
        "PLAN.md",
        HANDLER_VALUE_ADMISSION_REVIEW,
        "docs/work-packages/index.toml",
    ]);
    if admission_paths != expected_paths {
        return Err(format!(
            "handler value admission commit path mismatch; expected {expected_paths:?}, \
             found {admission_paths:?}"
        ));
    }
    Ok(())
}

fn check_handler_progress_checkpoint_state(
    root: &Path,
    admission_ref: &str,
    implementation_paths: &BTreeSet<String>,
) -> Result<(), String> {
    let head = git_text(root, &["rev-parse", "HEAD"], "resolve progress checkpoint HEAD")?;
    let head = head.trim();
    let expected_progress_paths =
        owned_set(&["PLAN.md", "docs/work-packages/index.toml"]);
    if head == admission_ref {
        let changed = git_worktree_paths(root, "pre-progress-checkpoint worktree")?;
        if changed != expected_progress_paths {
            return Err(format!(
                "pre-progress-checkpoint worktree path mismatch; expected \
                 {expected_progress_paths:?}, found {changed:?}"
            ));
        }
        return Ok(());
    }

    require_git_single_parent(
        root,
        head,
        admission_ref,
        "handler value progress checkpoint",
    )?;
    let progress_paths = git_changed_paths_between(
        root,
        admission_ref,
        head,
        "handler value progress checkpoint diff",
    )?;
    if progress_paths != expected_progress_paths {
        return Err(format!(
            "handler value progress checkpoint path mismatch; expected \
             {expected_progress_paths:?}, found {progress_paths:?}"
        ));
    }
    let worktree_paths = git_worktree_paths(root, "handler implementation worktree")?;
    if !worktree_paths.is_subset(implementation_paths) {
        let out_of_scope: Vec<&String> = worktree_paths.difference(implementation_paths).collect();
        return Err(format!(
            "handler implementation worktree has out-of-scope paths {out_of_scope:?}"
        ));
    }
    Ok(())
}

fn check_handler_value_primitives_entry_state(root: &Path, mode: &str) -> Result<(), String> {
    if !matches!(mode, "candidate" | "admission-ready") {
        return Err(format!(
            "invalid handler value entry-state mode {mode:?}; expected candidate or \
             admission-ready"
        ));
    }
    let path = root.join("docs/work-packages/index.toml");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    let tranches = document
        .get("tranche")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "work-package index has no [[tranche]] records".to_owned())?;
    let matching: Vec<&Table> = tranches
        .iter()
        .filter(|table| {
            table.get("id").and_then(Item::as_str) == Some(HANDLER_VALUE_PRIMITIVES_TRANCHE)
        })
        .collect();
    if matching.len() != 1 {
        return Err(format!(
            "work-package index must contain exactly one {HANDLER_VALUE_PRIMITIVES_TRANCHE}; \
             found {}",
            matching.len()
        ));
    }
    let tranche = matching[0];
    let status = string_field(tranche, "status", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    let admission_status = string_field(
        tranche,
        "admission_status",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    let expected_pair = match mode {
        "candidate" => ("pending", "review-pending"),
        "admission-ready" => ("pending", "approved"),
        _ => unreachable!("mode checked above"),
    };
    if (status.as_str(), admission_status.as_str()) != expected_pair {
        return Err(format!(
            "handler value {mode} state must be {:?}/{:?}; found {status:?}/{admission_status:?}",
            expected_pair.0, expected_pair.1
        ));
    }
    if tranche.contains_key("admission_ref") {
        return Err(format!(
            "handler value {mode} pending state must not define admission_ref"
        ));
    }

    let audit_path = string_field(
        tranche,
        "admission_review",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if audit_path != HANDLER_VALUE_ADMISSION_REVIEW {
        return Err("handler value admission review path drifted".to_owned());
    }
    let registered_artifacts = load_artifact_registry(root)?;
    if !registered_artifacts.contains(&audit_path) || !root.join(&audit_path).is_file() {
        return Err("handler value admission review is not registered and present".to_owned());
    }
    check_handler_value_audit_state(root, &audit_path, &admission_status)?;

    let implementation_paths = package_string_set(
        tranche,
        "implementation_paths",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if implementation_paths != owned_set(&["core/src/handler.rs", "core/src/lib.rs"]) {
        return Err("handler value implementation path projection drifted".to_owned());
    }
    let candidate_base_ref = string_field(
        tranche,
        "candidate_base_ref",
        HANDLER_VALUE_PRIMITIVES_TRANCHE,
    )?;
    if candidate_base_ref != "8c89e9346f424923ef3247dd1c402d5ab141c203" {
        return Err("handler value candidate base ref drifted".to_owned());
    }
    check_git_commit_is_ancestor(root, &candidate_base_ref, "candidate base ref")?;
    let candidate_paths =
        package_string_set(tranche, "candidate_paths", HANDLER_VALUE_PRIMITIVES_TRANCHE)?;
    check_handler_candidate_paths(
        root,
        &candidate_paths,
        &implementation_paths,
        &registered_artifacts,
    )?;

    if mode == "candidate" {
        for forbidden_field in ["review_attestation", "review_attestation_ref"] {
            if tranche.contains_key(forbidden_field) {
                return Err(format!(
                    "candidate state must not define {forbidden_field:?}"
                ));
            }
        }
        if root.join(HANDLER_VALUE_REVIEW_ATTESTATION).exists() {
            return Err("candidate state has a premature review attestation".to_owned());
        }
        check_handler_candidate_repository_state(root, &candidate_base_ref, &candidate_paths)?;
    } else {
        let attestation_path = string_field(
            tranche,
            "review_attestation",
            HANDLER_VALUE_PRIMITIVES_TRANCHE,
        )?;
        let attestation_ref = string_field(
            tranche,
            "review_attestation_ref",
            HANDLER_VALUE_PRIMITIVES_TRANCHE,
        )?;
        if attestation_path != HANDLER_VALUE_REVIEW_ATTESTATION
            || !registered_artifacts.contains(&attestation_path)
        {
            return Err("admission-ready review attestation path is not frozen".to_owned());
        }
        check_handler_review_attestation(
            root,
            &attestation_path,
            &attestation_ref,
            &candidate_base_ref,
            &candidate_paths,
        )?;
        let head = git_text(root, &["rev-parse", "HEAD"], "resolve HEAD")?;
        if head.trim() != attestation_ref {
            return Err(format!(
                "admission-ready state requires HEAD at review_attestation_ref \
                 {attestation_ref:?}; found {:?}",
                head.trim()
            ));
        }
    }
    Ok(())
}

fn check_handler_candidate_repository_state(
    root: &Path,
    candidate_base_ref: &str,
    candidate_paths: &BTreeSet<String>,
) -> Result<(), String> {
    let head = git_text(root, &["rev-parse", "HEAD"], "resolve candidate HEAD")?;
    let head = head.trim();
    if head == candidate_base_ref {
        let mut changed = git_path_set(
            &git_text(
                root,
                &["diff", "--name-only", "HEAD"],
                "read candidate tracked worktree diff",
            )?,
            "candidate tracked worktree diff",
        )?;
        changed.extend(git_path_set(
            &git_text(
                root,
                &["ls-files", "--others", "--exclude-standard"],
                "read candidate untracked paths",
            )?,
            "candidate untracked paths",
        )?);
        if &changed != candidate_paths {
            return Err(format!(
                "candidate worktree path mismatch; expected {candidate_paths:?}, found \
                 {changed:?}"
            ));
        }
        return Ok(());
    }

    require_git_single_parent(root, head, candidate_base_ref, "candidate commit")?;
    let changed =
        git_changed_paths_between(root, candidate_base_ref, head, "candidate commit diff")?;
    if &changed != candidate_paths {
        return Err(format!(
            "candidate commit path mismatch; expected {candidate_paths:?}, found {changed:?}"
        ));
    }
    let worktree = git_text(
        root,
        &["status", "--porcelain=v1", "--untracked-files=all"],
        "read candidate worktree status",
    )?;
    if !worktree.is_empty() {
        return Err("candidate commit state requires a clean worktree".to_owned());
    }
    Ok(())
}

fn require_full_commit_id(reference: &str, context: &str) -> Result<(), String> {
    if reference.len() != 40
        || !reference
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{context} must be a full lowercase 40-hex commit id; found {reference:?}"
        ));
    }
    Ok(())
}

fn check_git_commit_is_ancestor(root: &Path, reference: &str, context: &str) -> Result<(), String> {
    require_full_commit_id(reference, context)?;
    let object = format!("{reference}^{{commit}}");
    git_output_bytes(root, &["cat-file", "-e", &object], context)?;
    git_output_bytes(
        root,
        &["merge-base", "--is-ancestor", reference, "HEAD"],
        context,
    )?;
    Ok(())
}

fn require_git_single_parent(
    root: &Path,
    reference: &str,
    expected_parent: &str,
    context: &str,
) -> Result<(), String> {
    require_full_commit_id(expected_parent, &format!("{context} parent"))?;
    let parent = git_single_parent(root, reference, context)?;
    if parent != expected_parent {
        return Err(format!(
            "{context} must have parent {expected_parent:?}; found {parent:?}"
        ));
    }
    Ok(())
}

fn git_single_parent(root: &Path, reference: &str, context: &str) -> Result<String, String> {
    require_full_commit_id(reference, context)?;
    let record = git_text(
        root,
        &["rev-list", "--parents", "-n", "1", reference],
        context,
    )?;
    let fields: Vec<&str> = record.split_whitespace().collect();
    if fields.len() != 2 || fields[0] != reference {
        return Err(format!(
            "{context} must have exactly one parent; found {fields:?}"
        ));
    }
    require_full_commit_id(fields[1], &format!("{context} parent"))?;
    Ok(fields[1].to_owned())
}

fn git_changed_paths_between(
    root: &Path,
    from: &str,
    to: &str,
    context: &str,
) -> Result<BTreeSet<String>, String> {
    let output = git_text(root, &["diff", "--name-only", from, to], context)?;
    git_path_set(&output, context)
}

fn git_commit_changed_paths(
    root: &Path,
    reference: &str,
    context: &str,
) -> Result<BTreeSet<String>, String> {
    let output = git_text(
        root,
        &[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-only",
            "-r",
            reference,
        ],
        context,
    )?;
    git_path_set(&output, context)
}

fn git_path_set(output: &str, context: &str) -> Result<BTreeSet<String>, String> {
    let mut paths = BTreeSet::new();
    for path in output.lines() {
        validate_relative_path(path, context)?;
        if !paths.insert(path.to_owned()) {
            return Err(format!("{context} duplicates path {path:?}"));
        }
    }
    Ok(paths)
}

fn git_worktree_paths(root: &Path, context: &str) -> Result<BTreeSet<String>, String> {
    let mut paths = git_path_set(
        &git_text(root, &["diff", "--name-only", "HEAD"], context)?,
        context,
    )?;
    paths.extend(git_path_set(
        &git_text(
            root,
            &["ls-files", "--others", "--exclude-standard"],
            context,
        )?,
        context,
    )?);
    Ok(paths)
}

fn git_text(root: &Path, args: &[&str], context: &str) -> Result<String, String> {
    let output = git_output_bytes(root, args, context)?;
    String::from_utf8(output)
        .map_err(|error| format!("{context} produced non-UTF-8 output: {error}"))
}

fn git_output_bytes(root: &Path, args: &[&str], context: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| format!("cannot run git for {context}: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "git failed for {context} with {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    Ok(output.stdout)
}

fn check_tranche_admission_review(
    root: &Path,
    relative_path: &str,
    artifacts: &BTreeSet<String>,
    checks: &BTreeSet<String>,
) -> Result<(), String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    for required in [
        "Status: Passed",
        "Design revision: v4.9",
        "Admission scope: `WP-100-FOUNDATION-REFRESH`",
        "Verdict: Implementation-ready",
        "ResourceKind` indices `0..=138`",
        "indices `139..=194`",
        "HandlerSteps = 9",
        "`ResourceLimits` remains explicitly `Clone` but is no longer `Copy`",
        "`&'static ResourceLimits`; a bare profile id is not value authority",
        "`WorkBudget` implements neither `Clone` nor `Copy`",
        "HandlerCall = 1 << 11",
        "ProducerSubscriptionSetup = 1 << 12",
        "ProducerSubscriptionTeardown = 1 << 13",
        "No stable dynamic-library ABI is promised",
    ] {
        if !source.contains(required) {
            return Err(format!(
                "admission review {relative_path:?} misses required evidence {required:?}"
            ));
        }
    }
    for artifact in artifacts {
        let reference = format!("`{artifact}`");
        if !source.contains(&reference) {
            return Err(format!(
                "admission review {relative_path:?} does not cover artifact {artifact:?}"
            ));
        }
    }
    for check in checks {
        let reference = format!("`{check}`");
        if !source.contains(&reference) {
            return Err(format!(
                "admission review {relative_path:?} does not cover check {check:?}"
            ));
        }
    }
    Ok(())
}

fn check_handler_value_primitives_admission_review(
    root: &Path,
    relative_path: &str,
    artifacts: &BTreeSet<String>,
    checks: &BTreeSet<String>,
) -> Result<(), String> {
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    for required in [
        "Status: Passed",
        "Design revision: v4.9",
        "Admission scope: `WP-100-HANDLER-VALUE-PRIMITIVES`",
        "Verdict: Implementation-ready",
        "`CancellationView` is `#[repr(u8)]`",
        "`SubscriptionAcceptance` owns one `InteractionOutput`",
        "`HandlerFootprint` is the exact three-`u64`",
        "`HandlerStep<R>` has exactly `Pending` and `Ready(CoreResult<R>)`",
        "`StaticHandlerRegistration<'h, H>` borrows `H`",
    ] {
        if !source.contains(required) {
            return Err(format!(
                "admission review {relative_path:?} misses required value contract {required:?}"
            ));
        }
    }
    for artifact in artifacts {
        let reference = format!("`{artifact}`");
        if !source.contains(&reference) {
            return Err(format!(
                "admission review {relative_path:?} does not cover artifact {artifact:?}"
            ));
        }
    }
    for check in checks {
        let reference = format!("`{check}`");
        if !source.contains(&reference) {
            return Err(format!(
                "admission review {relative_path:?} does not cover check {check:?}"
            ));
        }
    }
    Ok(())
}

fn check_tranche_evidence(
    root: &Path,
    relative_path: &str,
    tranche: &str,
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
    require_string(document.get("tranche"), "tranche evidence tranche", tranche)?;
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

fn load_requirement_source_paths(root: &Path) -> Result<BTreeSet<String>, String> {
    let relative_path = "docs/requirements.csv";
    let path = root.join(relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut paths = BTreeSet::new();
    for (offset, line) in source.lines().skip(1).enumerate() {
        if line.trim().is_empty() {
            return Err(format!(
                "{relative_path} has a blank data row on line {}",
                offset + 2
            ));
        }
        let fields: Vec<&str> = line.split(',').collect();
        let requirement_source = fields.get(8).ok_or_else(|| {
            format!(
                "{relative_path} line {} has no requirement source",
                offset + 2
            )
        })?;
        validate_relative_path(requirement_source, "requirement source")?;
        if !root.join(requirement_source).is_file() {
            return Err(format!(
                "requirement source {requirement_source:?} does not exist"
            ));
        }
        paths.insert((*requirement_source).to_owned());
    }
    if paths.is_empty() {
        return Err(format!("{relative_path} has no requirement sources"));
    }
    Ok(paths)
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
        "Design revision: v4.9",
        "Depends on:",
        "Global convergence gates:",
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
    known_requirements: &BTreeSet<String>,
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
        if !known_requirements.contains(&requirement) {
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
    match id.as_str() {
        "active-route-driver" => check_active_route_driver_contract(machine)?,
        "binding-call" => check_binding_call_contract(machine)?,
        "binding-emission-slot" => check_binding_emission_slot_contract(machine)?,
        "binding-route" => check_binding_route_contract(machine)?,
        "cleanup-task" => check_cleanup_task_contract(machine)?,
        "compiled-plan-set" => check_compiled_plan_set_contract(machine)?,
        "lazy-binding-artifact" => check_lazy_binding_artifact_contract(machine)?,
        "producer-subscription" => check_producer_subscription_contract(machine)?,
        "route-lifecycle-call" => check_route_lifecycle_call_contract(machine)?,
        "route-readiness" => check_route_readiness_contract(machine)?,
        "serving-activation-authority" => check_serving_activation_authority_contract(machine)?,
        "subscription" => check_binding_subscription_contract(machine)?,
        "subscription-driver-slot" => check_subscription_driver_slot_contract(machine)?,
        _ => {}
    }
    Ok((id, transitions))
}

fn check_active_route_driver_contract(machine: &Table) -> Result<(), String> {
    let id = "active-route-driver";
    require_table_string(
        machine,
        "owner_record",
        "Servient ActiveRouteDriverRecord",
        id,
    )?;
    for field in [
        "scope",
        "wake_contract",
        "permit_contract",
        "shutdown_contract",
        "terminal_contract",
    ] {
        string_field(machine, field, id)?;
    }
    check_exact_machine_set(
        machine,
        id,
        "states",
        &[
            "AcceptClaimed",
            "Active",
            "Draining",
            "DrainingClaimed",
            "Released",
            "Reserved",
            "TerminalRetained",
        ],
    )?;
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Active",
                "claim_accept",
                "RouteAcceptOwner",
                "accept-call-and-route-lease-owned",
            ),
            (
                "AcceptClaimed",
                "pending_registered",
                "RouteAcceptOwner",
                "pending-register-then-recheck",
            ),
            (
                "AcceptClaimed",
                "operational_error",
                "RouteAcceptOwner",
                "operational-error-route-active",
            ),
            (
                "AcceptClaimed",
                "terminal",
                "ActiveRouteDriverOwner",
                "route-terminal-retained",
            ),
            (
                "Active",
                "drain",
                "ActiveRouteDriverOwner",
                "draining-no-new-accept",
            ),
            (
                "AcceptClaimed",
                "drain",
                "ActiveRouteDriverOwner",
                "draining-await-claimed-poll",
            ),
            (
                "DrainingClaimed",
                "request_late",
                "RouteAcceptOwner",
                "pre-drain-request-owned-by-servient",
            ),
            (
                "Draining",
                "release_to_shutdown",
                "BindingRouteCleanupSource",
                "committed-guard-transferred-to-shutdown",
            ),
            (
                "TerminalRetained",
                "acknowledge",
                "BindingRouteCleanupSource",
                "route-terminal-acknowledged",
            ),
        ],
    )
}

fn check_binding_call_contract(machine: &Table) -> Result<(), String> {
    let id = "binding-call";
    for field in [
        "scope",
        "construction_contract",
        "first_cause_contract",
        "poll_race_contract",
        "settlement_contract",
        "budget_contract",
        "wake_contract",
        "transfer_contract",
        "drop_contract",
    ] {
        string_field(machine, field, id)?;
    }
    check_exact_machine_set(
        machine,
        id,
        "states",
        &[
            "Accepted",
            "CancelRequested",
            "CancelStarting",
            "Cancelled",
            "Cancelling",
            "CleanupTransferred",
            "Constructing",
            "ConstructingCancelled",
            "PollClaimed",
            "RejectedInput",
            "Released",
            "Reserved",
            "Residual",
            "Returned",
            "TransferRequired",
        ],
    )?;
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Constructing",
                "input_rejected",
                "BindingCallConstructionOwner",
                "BindingInputRejection-complete-input-returned",
            ),
            (
                "CancelStarting",
                "transfer_required",
                "BindingCallCancellationOwner",
                "TransferRequired-source-owned",
            ),
            (
                "CancelStarting",
                "start_cancelled_complete",
                "BindingCallCancellationOwner",
                "Complete-successor-retained",
            ),
            (
                "CancelStarting",
                "start_residual",
                "BindingCallCancellationOwner",
                "ResidualExternalState-successor-retained",
            ),
            (
                "TransferRequired",
                "transfer_committed",
                "BindingCallCleanupOwner",
                "TransferCommitted-PendingCleanup",
            ),
            (
                "TransferRequired",
                "executor_rejected",
                "BindingCallManualCleanupOwner",
                "ManualFallback-complete-object-retained",
            ),
            (
                "CleanupTransferred",
                "complete",
                "BindingCallCleanupOwner",
                "Complete-successor-retained",
            ),
            (
                "CleanupTransferred",
                "residual",
                "BindingCallCleanupOwner",
                "ResidualExternalState-successor-retained",
            ),
            (
                "CleanupTransferred",
                "executor_drop",
                "BindingCallCleanupOwner",
                "ResidualExternalState-successor-retained",
            ),
        ],
    )
}

fn check_binding_emission_slot_contract(machine: &Table) -> Result<(), String> {
    let id = "binding-emission-slot";
    for field in [
        "scope",
        "input_contract",
        "generation_contract",
        "lane_contract",
        "result_contract",
        "critical_section_contract",
    ] {
        string_field(machine, field, id)?;
    }
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Reserved",
                "stale_generation",
                "BindingEmissionOwner",
                "BindingInputRejection-StaleHandle-complete-input-returned",
            ),
            (
                "Cancelling",
                "transfer_required",
                "BindingEmissionOwner",
                "TransferRequired-source-owned",
            ),
            (
                "TransferRequired",
                "transfer_committed",
                "BindingEmissionCleanupOwner",
                "TransferCommitted-PendingCleanup",
            ),
            (
                "TransferRequired",
                "executor_rejected",
                "BindingEmissionManualCleanupOwner",
                "ManualFallback-complete-input-retained",
            ),
            (
                "CleanupTransferred",
                "residual",
                "BindingEmissionCleanupOwner",
                "ResidualExternalState",
            ),
        ],
    )
}

fn check_binding_route_contract(machine: &Table) -> Result<(), String> {
    let id = "binding-route";
    require_table_string(machine, "owner_record", "Servient BindingRouteRecord", id)?;
    for field in [
        "scope",
        "guard_contract",
        "publication_contract",
        "shutdown_contract",
        "cleanup_contract",
    ] {
        string_field(machine, field, id)?;
    }
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Committing",
                "committed",
                "BindingRouteOwner",
                "CommittedClosed-guard-retained-nonserving",
            ),
            (
                "CommittedClosed",
                "publish",
                "Servient",
                "Serving-permit-eligible",
            ),
            (
                "Serving",
                "drain",
                "Servient",
                "Draining-existing-request-leases-retained",
            ),
            (
                "Draining",
                "begin_shutdown",
                "BindingRouteCleanupSource",
                "shutdown-call-owned",
            ),
            (
                "Activating",
                "activation_failed_prepared_retained",
                "BindingRouteOwner",
                "activation-failed-prepared-guard-retained",
            ),
            (
                "Committing",
                "commit_failed_active_retained",
                "BindingRouteOwner",
                "commit-failed-active-guard-retained",
            ),
            (
                "ActivatingCancelled",
                "active_late",
                "BindingRouteOwner",
                "ActiveLate-shutdown-required",
            ),
            (
                "Cleaning",
                "cleanup_complete",
                "BindingRouteCleanupSource",
                "Complete",
            ),
            (
                "Cleaning",
                "transfer_required",
                "BindingRouteCleanupSource",
                "TransferRequired-source-owned",
            ),
            (
                "TransferRequired",
                "transfer_committed",
                "BindingRouteCleanupOwner",
                "TransferCommitted-PendingCleanup",
            ),
            (
                "TransferRequired",
                "executor_rejected",
                "BindingRouteManualCleanupOwner",
                "ManualFallback-complete-guard-retained",
            ),
        ],
    )?;
    let transitions = machine
        .get("transition")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("machine {id:?} has no transitions"))?;
    if transitions.iter().any(|transition| {
        transition.get("from").and_then(Item::as_str) == Some("Serving")
            && transition.get("event").and_then(Item::as_str) == Some("begin_shutdown")
    }) {
        return Err(
            "machine \"binding-route\" permits Serving:begin_shutdown before route drain and \
             active-route-driver release"
                .to_owned(),
        );
    }
    Ok(())
}

fn check_serving_activation_authority_contract(machine: &Table) -> Result<(), String> {
    let id = "serving-activation-authority";
    require_table_string(machine, "visibility", "cross-crate-servient-contract", id)?;
    require_table_string(
        machine,
        "owner_record",
        "Servient ServingActivationRecord",
        id,
    )?;
    require_table_string(
        machine,
        "authority_contract",
        "ServingActivationAuthority contains only immutable public Thing, produced-generation, \
         and plan-set-generation identity. claim_route exclusively borrows a matching \
         RouteAcceptLease and returns a RouteAcceptClaim whose consuming into_permit operation is \
         the only permit constructor; the private ServingActivationRecord owns all mutable \
         publication and drain state and owns no binding state, route list, queue, or application \
         dispatch capability.",
        id,
    )?;
    for field in [
        "scope",
        "publication_contract",
        "permit_contract",
        "drain_contract",
        "resource_contract",
    ] {
        string_field(machine, field, id)?;
    }
    check_exact_machine_set(
        machine,
        id,
        "states",
        &["Cancelled", "Draining", "Reclaimed", "Reserved", "Serving"],
    )?;
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Reserved",
                "publish",
                "ServientServingActivationOwner",
                "Serving-permit-claims-enabled",
            ),
            (
                "Reserved",
                "cancel",
                "ServientServingActivationOwner",
                "Cancelled-never-serving",
            ),
            (
                "Serving",
                "claim_route",
                "RouteAcceptOwner",
                "Borrowed-route-permit-after-claim-consume",
            ),
            (
                "Serving",
                "reject_route_claim",
                "ServientServingActivationOwner",
                "Route-claim-rejected",
            ),
            (
                "Serving",
                "begin_drain",
                "ServientServingActivationOwner",
                "Draining-existing-claims-retained",
            ),
            (
                "Draining",
                "reclaim_complete",
                "PlanSetReclaimOwner",
                "Reclaimed-zero-authority-bytes",
            ),
        ],
    )
}

fn check_cleanup_task_contract(machine: &Table) -> Result<(), String> {
    let id = "cleanup-task";
    require_table_string(machine, "owner_record", "Servient CleanupTaskRecord", id)?;
    for field in [
        "scope",
        "record_contract",
        "progress_contract",
        "transfer_contract",
    ] {
        string_field(machine, field, id)?;
    }
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Reserved",
                "offer",
                "CleanupSourceOwner",
                "TransferRequired-source-owned",
            ),
            (
                "Offered",
                "accepted",
                "ExecutorCleanupOwner",
                "TransferCommitted-PendingCleanup",
            ),
            (
                "Offered",
                "executor_rejected",
                "ManualCleanupOwner",
                "ManualFallback-complete-object-retained",
            ),
            (
                "ExecutorOwned",
                "zero_budget",
                "ExecutorCleanupOwner",
                "TransferCommitted-PendingCleanup",
            ),
            (
                "ExecutorOwned",
                "executor_drop",
                "CleanupTaskOwner",
                "ResidualExternalState",
            ),
            (
                "ExecutorPollClaimed",
                "executor_drop",
                "CleanupTaskOwner",
                "ResidualExternalState",
            ),
            (
                "ManualOwned",
                "drain_expired",
                "ManualCleanupOwner",
                "ResidualExternalState",
            ),
            (
                "ResidualRetained",
                "acknowledge",
                "CleanupTaskOwner",
                "residual-acknowledged",
            ),
        ],
    )
}

fn check_route_lifecycle_call_contract(machine: &Table) -> Result<(), String> {
    let id = "route-lifecycle-call";
    require_table_string(
        machine,
        "owner_record",
        "Servient RouteLifecycleCallRecord",
        id,
    )?;
    for field in ["scope", "guard_contract", "transfer_contract"] {
        string_field(machine, field, id)?;
    }
    check_exact_machine_set(
        machine,
        id,
        "states",
        &[
            "CancelRequested",
            "CleanupComplete",
            "CleanupTransferred",
            "Completed",
            "InProgress",
            "LateCompleted",
            "RejectedInput",
            "Released",
            "Reserved",
            "Residual",
            "ResidualSuccessorRetained",
            "TransferRequired",
        ],
    )?;
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Reserved",
                "typed_rejection",
                "RouteLifecycleCallOwner",
                "BindingInputRejection-complete-input-returned",
            ),
            (
                "InProgress",
                "complete_success",
                "BindingRouteOwner",
                "Complete-successor-guard-retained",
            ),
            (
                "InProgress",
                "complete_failure_predecessor",
                "BindingRouteOwner",
                "Complete-failure-predecessor-guard-retained",
            ),
            (
                "CancelRequested",
                "late_success",
                "BindingRouteOwner",
                "LateValue-successor-guard-retained",
            ),
            (
                "CancelRequested",
                "cancel_complete_successor",
                "BindingRouteOwner",
                "Complete-cleanup-successor-retained",
            ),
            (
                "CancelRequested",
                "cancel_residual_successor",
                "BindingRouteOwner",
                "ResidualExternalState-cleanup-successor-retained",
            ),
            (
                "TransferRequired",
                "transfer_committed",
                "RouteLifecycleCleanupOwner",
                "TransferCommitted-PendingCleanup",
            ),
            (
                "CleanupTransferred",
                "executor_drop_residual_successor",
                "BindingRouteOwner",
                "ResidualExternalState-cleanup-successor-retained",
            ),
        ],
    )
}

fn check_route_readiness_contract(machine: &Table) -> Result<(), String> {
    let id = "route-readiness";
    require_table_string(machine, "owner_record", "Servient RouteReadinessRecord", id)?;
    for field in ["scope", "wake_contract", "guard_contract"] {
        string_field(machine, field, id)?;
    }
    check_exact_machine_set(
        machine,
        id,
        "states",
        &[
            "Cancelling",
            "CleanupSuccessorRetained",
            "CleanupTransferred",
            "FailedGuardRetained",
            "Polling",
            "Ready",
            "Released",
            "Reserved",
            "ResidualSuccessorRetained",
            "TransferRequired",
        ],
    )?;
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Polling",
                "pending_registered",
                "RouteReadinessOwner",
                "pending-register-then-recheck",
            ),
            (
                "Polling",
                "failed_guard_retained",
                "BindingRouteOwner",
                "Complete-failure-prepared-guard-retained",
            ),
            (
                "Cancelling",
                "drain_expired",
                "RouteReadinessOwner",
                "TransferRequired-drain-expired-source-owned",
            ),
            (
                "Cancelling",
                "cancel_complete_successor",
                "BindingRouteOwner",
                "Complete-cleanup-successor-retained",
            ),
            (
                "Cancelling",
                "cancel_residual_successor",
                "BindingRouteOwner",
                "ResidualExternalState-cleanup-successor-retained",
            ),
            (
                "TransferRequired",
                "executor_rejected",
                "RouteReadinessManualCleanupOwner",
                "ManualFallback-complete-object-retained",
            ),
            (
                "CleanupTransferred",
                "executor_drop_residual_successor",
                "BindingRouteOwner",
                "ResidualExternalState-cleanup-successor-retained",
            ),
        ],
    )
}

fn check_binding_subscription_contract(machine: &Table) -> Result<(), String> {
    let id = "subscription";
    check_exact_machine_set(
        machine,
        id,
        "process_terminal_axis",
        &[
            "Cancelled",
            "Completed",
            "Domain",
            "Failed",
            "Overflowed",
            "TimedOut",
        ],
    )?;
    for field in ["terminal_contract", "terminal_retention"] {
        string_field(machine, field, id)?;
    }
    require_table_string(
        machine,
        "stop_contract",
        "Stop, drop, remote terminal, and Servient drain join one generation-checked stop owner. \
         StopStarting owns the unchanged driver plus one complete SubscriptionStopInput \
         containing the exact SubscriptionStopRequest and independent CleanupPhaseContext until \
         start_stop accepts the full input; pre-acceptance rejection returns both unchanged. \
         After acceptance the full input commits to the retained stop cursor before poll_stop \
         progress. Both callbacks receive Context, execute outside engine locks or constrained \
         critical sections, and are never concurrent with poll_item on the same driver.",
        id,
    )?;
    check_exact_machine_set(
        machine,
        id,
        "states",
        &[
            "Active",
            "CancellingStart",
            "CleanupPending",
            "Closed",
            "Failed",
            "Installing",
            "Released",
            "Reserved",
            "Starting",
            "StopStarting",
            "Stopping",
            "TransferRequired",
        ],
    )?;
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Active",
                "driver_terminal_complete",
                "SubscriptionDriverRegistry",
                "Complete",
            ),
            (
                "Active",
                "driver_terminal_transfer_required",
                "SubscriptionReceiveOwner",
                "TransferRequired-source-owned",
            ),
            (
                "Active",
                "driver_terminal_residual",
                "SubscriptionDriverRegistry",
                "ResidualExternalState",
            ),
            (
                "TransferRequired",
                "transfer_accepted",
                "SubscriptionCleanupOwner",
                "TransferCommitted-PendingCleanup",
            ),
            (
                "TransferRequired",
                "transfer_rejected_manual_accepted",
                "SubscriptionManualCleanupOwner",
                "ManualFallback-complete-object-retained",
            ),
        ],
    )?;
    require_transition_field_fragments(
        machine,
        id,
        "StopStarting",
        "explicit_start_rejected",
        "linearization",
        &[
            "unchanged complete SubscriptionStopInput",
            "driver",
            "pre-acceptance rejection",
        ],
    )?;
    require_transition_field_fragments(
        machine,
        id,
        "StopStarting",
        "implicit_start_invalid",
        "linearization",
        &[
            "complete implicit SubscriptionStopInput",
            "CleanupPhaseContext",
            "durably recording",
        ],
    )?;
    require_transition_field_fragments(
        machine,
        id,
        "StopStarting",
        "start_pending",
        "linearization",
        &[
            "full SubscriptionStopInput",
            "exact request",
            "CleanupPhaseContext",
        ],
    )?;
    Ok(())
}

fn check_subscription_driver_slot_contract(machine: &Table) -> Result<(), String> {
    let id = "subscription-driver-slot";
    for field in [
        "scope",
        "claim_contract",
        "critical_section_contract",
        "terminal_contract",
    ] {
        string_field(machine, field, id)?;
    }
    require_table_string(
        machine,
        "stop_input_contract",
        "StopStarting owns the unchanged driver or typed slot plus one complete \
         SubscriptionStopInput until start_stop accepts its exact request and \
         CleanupPhaseContext. Explicit pre-acceptance rejection returns both unchanged; implicit \
         rejection retains both through durable residual recording; Pending commits the full \
         input to StopClaimed.",
        id,
    )?;
    check_exact_machine_set(
        machine,
        id,
        "states",
        &[
            "Available",
            "CleanupPending",
            "Closed",
            "Installing",
            "ReceiveClaimed",
            "Released",
            "Residual",
            "StopClaimed",
            "StopRequested",
            "StopStarting",
            "TransferRequired",
            "Vacant",
        ],
    )?;
    check_required_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "ReceiveClaimed",
                "terminal_transfer_required",
                "SubscriptionReceiveOwner",
                "TransferRequired-source-owned",
            ),
            (
                "StopStarting",
                "transfer_required",
                "SubscriptionStopOwner",
                "TransferRequired-source-owned",
            ),
            (
                "StopClaimed",
                "transfer_required",
                "SubscriptionStopOwner",
                "TransferRequired-source-owned",
            ),
            (
                "TransferRequired",
                "transfer_accepted",
                "SubscriptionCleanupOwner",
                "TransferCommitted-PendingCleanup",
            ),
            (
                "TransferRequired",
                "transfer_rejected_manual_accepted",
                "SubscriptionManualCleanupOwner",
                "ManualFallback-complete-object-retained",
            ),
        ],
    )?;
    require_transition_field_fragments(
        machine,
        id,
        "StopStarting",
        "explicit_start_rejected",
        "linearization",
        &[
            "unchanged complete SubscriptionStopInput",
            "driver or typed slot",
            "pre-acceptance rejection",
        ],
    )?;
    require_transition_field_fragments(
        machine,
        id,
        "StopStarting",
        "implicit_start_invalid",
        "linearization",
        &[
            "complete implicit SubscriptionStopInput",
            "driver or typed slot",
            "CleanupPhaseContext",
            "durably recording",
        ],
    )?;
    require_transition_field_fragments(
        machine,
        id,
        "StopStarting",
        "start_pending",
        "linearization",
        &[
            "full SubscriptionStopInput",
            "exact request",
            "CleanupPhaseContext",
        ],
    )?;
    Ok(())
}

fn check_compiled_plan_set_contract(machine: &Table) -> Result<(), String> {
    let id = "compiled-plan-set";
    require_table_string(
        machine,
        "owner_record",
        "Servient CompiledPlanSetRecord",
        id,
    )?;
    for field in [
        "scope",
        "publication_contract",
        "pin_contract",
        "failed_disposition_contract",
        "reclaimed_disposition_contract",
    ] {
        string_field(machine, field, id)?;
    }
    check_exact_machine_set(
        machine,
        id,
        "states",
        &[
            "Building",
            "Draining",
            "Failed",
            "Frozen",
            "Published",
            "Reclaimed",
        ],
    )?;
    check_exact_machine_set(machine, id, "terminal", &["Failed", "Reclaimed"])?;
    check_exact_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Building",
                "build_complete",
                "PlanningBuildOwner",
                "Frozen-unpublished",
            ),
            (
                "Building",
                "build_failed",
                "PlanSetReclaimOwner",
                "Failed-unpublished-cleanup-owned",
            ),
            (
                "Building",
                "cancel",
                "PlanSetReclaimOwner",
                "Failed-unpublished-cancelled-cleanup-owned",
            ),
            (
                "Frozen",
                "publish_consumer",
                "Servient",
                "Published-consumer",
            ),
            (
                "Frozen",
                "publish_producer",
                "Servient",
                "Published-producer-serving",
            ),
            (
                "Frozen",
                "route_failure",
                "PlanSetReclaimOwner",
                "Failed-unpublished-route-rollback-owned",
            ),
            (
                "Frozen",
                "cancel",
                "PlanSetReclaimOwner",
                "Failed-unpublished-cancelled-cleanup-owned",
            ),
            (
                "Published",
                "begin_drain",
                "ServientPlanSetOwner",
                "Draining-existing-leases-retained",
            ),
            (
                "Draining",
                "reclaim_complete",
                "PlanSetReclaimOwner",
                "Reclaimed-zero-retained-plan-and-artifact-bytes",
            ),
        ],
    )
}

fn check_lazy_binding_artifact_contract(machine: &Table) -> Result<(), String> {
    let id = "lazy-binding-artifact";
    require_table_string(
        machine,
        "owner_record",
        "Servient LazyBindingArtifactSlot",
        id,
    )?;
    for field in [
        "scope",
        "single_flight_contract",
        "abort_disposition_contract",
        "negative_disposition_contract",
        "reclaim_disposition_contract",
        "quiescent_terminal_semantics",
    ] {
        string_field(machine, field, id)?;
    }
    check_exact_machine_set(
        machine,
        id,
        "states",
        &["Compiling", "Empty", "Negative", "Ready", "Reclaiming"],
    )?;
    check_exact_machine_set(machine, id, "terminal", &["Empty"])?;
    check_exact_machine_set(
        machine,
        id,
        "reusable_terminal_events",
        &["Empty:begin_compile"],
    )?;
    check_exact_transition_owners_and_outcomes(
        machine,
        id,
        &[
            (
                "Empty",
                "begin_compile",
                "LazyArtifactCompilerOwner",
                "Compiling-single-flight",
            ),
            (
                "Compiling",
                "publish_ready",
                "LazyArtifactSlotOwner",
                "Ready-shared-immutable-artifact",
            ),
            (
                "Compiling",
                "publish_negative",
                "LazyArtifactSlotOwner",
                "Negative-shared-deterministic-failure",
            ),
            (
                "Compiling",
                "abort",
                "LazyArtifactCompilerOwner",
                "Empty-noncacheable-abort",
            ),
            (
                "Compiling",
                "retryable_failure",
                "LazyArtifactCompilerOwner",
                "Empty-noncacheable-failure",
            ),
            (
                "Ready",
                "begin_reclaim",
                "PlanSetReclaimOwner",
                "Reclaiming-ready-artifact",
            ),
            (
                "Negative",
                "begin_reclaim",
                "PlanSetReclaimOwner",
                "Reclaiming-negative-diagnostic",
            ),
            (
                "Reclaiming",
                "reclaim_complete",
                "PlanSetReclaimOwner",
                "Empty-reclaimed",
            ),
        ],
    )
}

fn check_exact_machine_set(
    machine: &Table,
    id: &str,
    field: &str,
    expected: &[&str],
) -> Result<(), String> {
    let expected = owned_set(expected);
    let actual = string_set(array_field(machine, field, id)?, id, field)?;
    if actual != expected {
        return Err(format!(
            "machine {id:?} {field:?} mismatch; expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn check_exact_transition_owners_and_outcomes(
    machine: &Table,
    id: &str,
    expected: &[(&str, &str, &str, &str)],
) -> Result<(), String> {
    let transitions = machine
        .get("transition")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("machine {id:?} has no transitions"))?;
    if transitions.len() != expected.len() {
        return Err(format!(
            "machine {id:?} transition ownership contract expected {} entries, found {}",
            expected.len(),
            transitions.len()
        ));
    }
    for (from, event, owner, outcome) in expected {
        let transition = transitions
            .iter()
            .find(|transition| {
                transition.get("from").and_then(Item::as_str) == Some(*from)
                    && transition.get("event").and_then(Item::as_str) == Some(*event)
            })
            .ok_or_else(|| format!("machine {id:?} has no transition {from:?}:{event:?}"))?;
        require_table_string(transition, "owner", owner, id)?;
        require_table_string(transition, "outcome", outcome, id)?;
    }
    Ok(())
}

fn check_required_transition_owners_and_outcomes(
    machine: &Table,
    id: &str,
    expected: &[(&str, &str, &str, &str)],
) -> Result<(), String> {
    let transitions = machine
        .get("transition")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("machine {id:?} has no transitions"))?;
    for (from, event, owner, outcome) in expected {
        let transition = transitions
            .iter()
            .find(|transition| {
                transition.get("from").and_then(Item::as_str) == Some(*from)
                    && transition.get("event").and_then(Item::as_str) == Some(*event)
            })
            .ok_or_else(|| format!("machine {id:?} has no transition {from:?}:{event:?}"))?;
        require_table_string(transition, "owner", owner, id)?;
        require_table_string(transition, "outcome", outcome, id)?;
    }
    Ok(())
}

fn require_transition_field_fragments(
    machine: &Table,
    id: &str,
    from: &str,
    event: &str,
    field: &str,
    expected_fragments: &[&str],
) -> Result<(), String> {
    let transitions = machine
        .get("transition")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("machine {id:?} has no transitions"))?;
    let transition = transitions
        .iter()
        .find(|transition| {
            transition.get("from").and_then(Item::as_str) == Some(from)
                && transition.get("event").and_then(Item::as_str) == Some(event)
        })
        .ok_or_else(|| format!("machine {id:?} has no transition {from:?}:{event:?}"))?;
    let value = string_field(transition, field, id)?;
    let missing: Vec<&&str> = expected_fragments
        .iter()
        .filter(|fragment| !value.contains(**fragment))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "machine {id:?} transition {from:?}:{event:?} field {field:?} is missing required \
             fragments {missing:?}"
        ));
    }
    Ok(())
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
    known_requirements: &BTreeSet<String>,
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
            if !known_requirements.contains(&requirement) {
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
        "active-route-acceptance" => Ok(&[
            "accept_policy",
            "publish_transitions",
            "accept_transitions",
            "status_transitions",
            "terminal_transitions",
            "drain_transitions",
            "claim_join_transitions",
            "shutdown_preconditions",
            "shutdown_transitions",
            "ownership",
        ]),
        "binding-call-cleanup-transfer" => Ok(&[
            "transfer_policy",
            "request_transitions",
            "commit_transitions",
            "fallback_transitions",
            "terminal_transitions",
            "ownership",
        ]),
        "binding-route-lifecycle" => Ok(&[
            "guard_policy",
            "start_transitions",
            "success_transitions",
            "failure_transitions",
            "late_transitions",
            "cleanup_transitions",
            "ownership",
        ]),
        "binding-route-readiness" => Ok(&[
            "wake_policy",
            "start_transitions",
            "progress_transitions",
            "ready_transitions",
            "failure_transitions",
            "cancel_transitions",
            "ownership",
        ]),
        "consumer-plan-publication" => {
            Ok(&["publication_policy", "atomic_transitions", "ownership"])
        }
        "emission-delivery-ownership" => Ok(&[
            "input_policy",
            "accept_transitions",
            "reject_transitions",
            "transfer_transitions",
            "terminal_transitions",
            "ownership",
        ]),
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
        "producer-plan-drain" => Ok(&[
            "drain_policy",
            "drain_transitions",
            "route_claim_join_transitions",
            "route_shutdown_preconditions",
            "route_shutdown_transitions",
            "reclaim_preconditions",
            "ownership",
        ]),
        "producer-plan-serving-publication" => {
            Ok(&["publication_policy", "atomic_transitions", "ownership"])
        }
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
        "response-delivery-ownership" => Ok(&[
            "input_policy",
            "accept_transitions",
            "reject_transitions",
            "transfer_transitions",
            "terminal_transitions",
            "ownership",
        ]),
        "subscription-process-cleanup" => Ok(&[
            "terminal_policy",
            "complete_transitions",
            "request_transitions",
            "commit_transitions",
            "fallback_transitions",
            "residual_transitions",
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

fn check_exact_composition_set(
    composition: &Table,
    id: &str,
    field: &str,
    expected: &[&str],
) -> Result<(), String> {
    let expected = owned_set(expected);
    let actual = string_set(array_field(composition, field, id)?, id, field)?;
    if actual != expected {
        return Err(format!(
            "composition {id:?} {field:?} mismatch; expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn check_exact_composition_sequence(
    composition: &Table,
    id: &str,
    field: &str,
    expected: &[&str],
) -> Result<(), String> {
    let actual = string_vec(array_field(composition, field, id)?, id, field)?;
    if actual != expected {
        return Err(format!(
            "composition {id:?} {field:?} sequence mismatch; expected {expected:?}, found \
             {actual:?}"
        ));
    }
    Ok(())
}

fn check_composition_policy(composition: &Table, id: &str) -> Result<(), String> {
    let (field, expected) = match id {
        "active-route-acceptance" => (
            "accept_policy",
            "one-generation-checked-route-claim-chain-and-one-poll-and-waker-lease-per-route",
        ),
        "binding-call-cleanup-transfer" => ("transfer_policy", "source-owned-until-acknowledged"),
        "binding-route-lifecycle" => ("guard_policy", "predecessor-or-successor-always-retained"),
        "binding-route-readiness" => ("wake_policy", "register-then-recheck-one-lease-per-route"),
        "consumer-plan-publication" => (
            "publication_policy",
            "atomic-plan-set-and-consumer-registry",
        ),
        "handler-direct-response" => ("response_claim_policy", "atomic-with-handler-release"),
        "emission-delivery-ownership" => {
            ("input_policy", "return-complete-input-before-acceptance")
        }
        "producer-start-result-transfer" => (
            "error_response_policy",
            "claim-before-failed-or-same-generation-terminal",
        ),
        "producer-late-start-result-transfer" => {
            ("late_acceptance_policy", "create-teardown-before-discard")
        }
        "producer-plan-drain" => ("drain_policy", "close-ingress-before-reclamation"),
        "producer-plan-serving-publication" => {
            ("publication_policy", "atomic-with-serving-registry")
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
        "response-delivery-ownership" => {
            ("input_policy", "return-complete-input-before-acceptance")
        }
        "subscription-process-cleanup" => (
            "terminal_policy",
            "process-terminal-retained-separately-from-cleanup",
        ),
        _ => return Err(format!("unknown state composition {id:?}")),
    };
    require_string(
        composition.get(field),
        &format!("composition {id} {field}"),
        expected,
    )?;

    if id == "active-route-acceptance" {
        check_exact_composition_sequence(
            composition,
            id,
            "accept_transitions",
            &[
                "active-route-driver:Active:claim_accept->AcceptClaimed",
                "serving-activation-authority:Serving:claim_route->Serving",
                "active-route-driver:AcceptClaimed:request->Active",
                "in-flight:Vacant:admit->Admitted",
            ],
        )?;
        check_exact_composition_sequence(
            composition,
            id,
            "drain_transitions",
            &[
                "serving-activation-authority:Serving:begin_drain->Draining",
                "binding-route:Serving:drain->Draining",
                "active-route-driver:Active:drain->Draining",
                "active-route-driver:AcceptClaimed:drain->DrainingClaimed",
            ],
        )?;
        check_exact_composition_sequence(
            composition,
            id,
            "claim_join_transitions",
            &[
                "active-route-driver:DrainingClaimed:pending_registered|request_late|operational_error->Draining",
                "active-route-driver:DrainingClaimed:terminal->TerminalRetained",
                "active-route-driver:Draining:release_to_shutdown->Released",
                "active-route-driver:TerminalRetained:acknowledge->Released",
            ],
        )?;
        check_exact_composition_set(
            composition,
            id,
            "shutdown_preconditions",
            &[
                "serving-activation-authority-is-Draining",
                "active-route-driver-is-Released",
            ],
        )?;
        check_exact_composition_sequence(
            composition,
            id,
            "shutdown_transitions",
            &["binding-route:Draining:begin_shutdown->Cleaning"],
        )?;
    } else if id == "producer-plan-serving-publication" {
        check_exact_composition_set(
            composition,
            id,
            "requirements",
            &[
                "PLAN-SET-001",
                "LIFE-EXPOSE-002",
                "LIFE-EXPOSE-003",
                "STATE-BIND-001",
            ],
        )?;
    } else if id == "producer-plan-drain" {
        check_exact_composition_sequence(
            composition,
            id,
            "drain_transitions",
            &[
                "serving-activation-authority:Serving:begin_drain->Draining",
                "compiled-plan-set:Published:begin_drain->Draining",
                "expose:Serving:drop|destroy->Draining",
                "binding-route:Serving:drain->Draining",
                "active-route-driver:Active:drain->Draining",
                "active-route-driver:AcceptClaimed:drain->DrainingClaimed",
            ],
        )?;
        check_exact_composition_sequence(
            composition,
            id,
            "route_claim_join_transitions",
            &[
                "active-route-driver:DrainingClaimed:pending_registered|request_late|operational_error->Draining",
                "active-route-driver:DrainingClaimed:terminal->TerminalRetained",
                "active-route-driver:Draining:release_to_shutdown->Released",
                "active-route-driver:TerminalRetained:acknowledge->Released",
            ],
        )?;
        check_exact_composition_set(
            composition,
            id,
            "route_shutdown_preconditions",
            &[
                "serving-activation-authority-is-Draining",
                "active-route-driver-is-Released",
            ],
        )?;
        check_exact_composition_sequence(
            composition,
            id,
            "route_shutdown_transitions",
            &["binding-route:Draining:begin_shutdown->Cleaning"],
        )?;
    } else if id == "producer-teardown-handoff" {
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

fn require_exact_table_fields(
    table: &Table,
    context: &str,
    expected_fields: &[&str],
) -> Result<(), String> {
    let actual: BTreeSet<String> = table.iter().map(|(key, _)| key.to_owned()).collect();
    let expected = owned_set(expected_fields);
    if actual != expected {
        return Err(format!(
            "{context} field set mismatch; expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
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
        HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT, check_handler_scope_partition,
        check_producer_subscription_contract, expand_expressions,
        expected_work_package_dependencies, handler_value_status_pair_is_valid,
        parse_handler_review_attestation, parse_transitions, validate_handler_value_audit_source,
        validate_handler_value_primitives_source,
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

    #[test]
    fn handler_value_status_pairs_are_exact() {
        let allowed = BTreeSet::from([
            ("pending", "review-pending"),
            ("pending", "approved"),
            ("in-progress", "approved"),
            ("complete", "approved"),
        ]);
        for status in ["pending", "in-progress", "complete", "revoked"] {
            for admission in ["review-pending", "approved", "revoked"] {
                assert_eq!(
                    handler_value_status_pair_is_valid(status, admission),
                    allowed.contains(&(status, admission)),
                    "unexpected decision for {status}/{admission}"
                );
            }
        }
    }

    #[test]
    fn handler_value_and_time_scope_projection_allows_only_the_declared_meta_overlap() {
        let values = BTreeSet::from([
            "API-SURFACE-001".to_owned(),
            "HANDLER-VALUE-001".to_owned(),
        ]);
        let value_items = BTreeSet::from(["HandlerStep".to_owned()]);
        let blocking = BTreeSet::from([
            "API-SURFACE-001".to_owned(),
            "TIME-001".to_owned(),
        ]);
        let shared = BTreeSet::from(["API-SURFACE-001".to_owned()]);
        let blocking_items = BTreeSet::from(["Deadline".to_owned()]);
        assert!(
            check_handler_scope_partition(
                &values,
                &value_items,
                &blocking,
                &shared,
                &blocking_items,
            )
            .is_ok()
        );

        let overlapping_items = BTreeSet::from(["HandlerStep".to_owned()]);
        assert!(
            check_handler_scope_partition(
                &values,
                &value_items,
                &blocking,
                &shared,
                &overlapping_items,
            )
            .is_err()
        );

        let undeclared_overlap = BTreeSet::from([
            "API-SURFACE-001".to_owned(),
            "HANDLER-VALUE-001".to_owned(),
            "TIME-001".to_owned(),
        ]);
        assert!(
            check_handler_scope_partition(
                &values,
                &value_items,
                &undeclared_overlap,
                &shared,
                &blocking_items,
            )
            .is_err()
        );
    }

    #[test]
    fn handler_review_attestation_projection_requires_six_exact_passes() {
        let source = r#"
schema_version = 1
design_revision = "4.9"
tranche = "WP-100-HANDLER-VALUE-PRIMITIVES"
status = "passed"
reviewer_attestation_kind = "separate-agent-task"
reviewer_id = "codex-agent:/root/independent_review"
reviewed_ref = "0123456789abcdef0123456789abcdef01234567"

[[precheck]]
id = "api-ownership-check"
result = "passed"

[[precheck]]
id = "architecture-adr-check"
result = "passed"

[[precheck]]
id = "resource-profile-check"
result = "passed"

[[precheck]]
id = "work-package-dag-check"
result = "passed"

[[precheck]]
id = "wp100-amendment-check"
result = "passed"

[[precheck]]
id = "wp100-handler-amendment-check"
result = "passed"
"#;
        let projection = parse_handler_review_attestation(source)
            .expect("the exact review projection must validate");
        assert_eq!(
            projection.reviewed_ref,
            "0123456789abcdef0123456789abcdef01234567"
        );

        let failed = source.replacen("result = \"passed\"", "result = \"failed\"", 1);
        assert!(parse_handler_review_attestation(&failed).is_err());
        let root_reviewer = source.replace(
            "codex-agent:/root/independent_review",
            "codex-agent:/root",
        );
        assert!(parse_handler_review_attestation(&root_reviewer).is_err());
    }

    #[test]
    fn handler_value_audit_state_rejects_crossed_markers() {
        let pending = "Status: Pending\nVerdict: Independent re-review pending\n";
        assert!(validate_handler_value_audit_source(pending, "review-pending").is_ok());
        assert!(validate_handler_value_audit_source(pending, "approved").is_err());

        let passed = "Status: Passed\nVerdict: Implementation-ready\n";
        assert!(validate_handler_value_audit_source(passed, "approved").is_ok());
        assert!(validate_handler_value_audit_source(passed, "review-pending").is_err());
    }

    #[test]
    fn exact_handler_value_source_projection_is_accepted() {
        validate_handler_value_primitives_source(HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT)
            .expect("the frozen five-value source must validate");
    }

    #[test]
    fn handler_value_source_rejects_forbidden_dependencies() {
        let source = format!("use alloc::vec::Vec;\n{HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT}");
        let error = validate_handler_value_primitives_source(&source)
            .expect_err("allocation paths must be rejected");
        assert!(error.contains("forbidden"));
    }

    #[test]
    fn handler_value_source_rejects_bounds_traits_and_public_items() {
        let bounded = HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT.replacen(
            "impl<H> Copy for StaticHandlerRegistration",
            "impl<H: Copy> Copy for StaticHandlerRegistration",
            1,
        );
        assert!(validate_handler_value_primitives_source(&bounded).is_err());

        let defaulted = format!(
            "{HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT}\nimpl Default for HandlerFootprint {{\n    fn default() -> Self {{ Self::new(0, 0, 0) }}\n}}"
        );
        assert!(validate_handler_value_primitives_source(&defaulted).is_err());

        for prohibited_impl in [
            "impl Copy for SubscriptionAcceptance {}",
            "impl Clone for SubscriptionAcceptance { fn clone(&self) -> Self { panic!() } }",
            "impl Default for SubscriptionAcceptance { fn default() -> Self { panic!() } }",
            "impl<R> Copy for HandlerStep<R> {}",
            "impl<R> Clone for HandlerStep<R> { fn clone(&self) -> Self { panic!() } }",
            "impl<R> Default for HandlerStep<R> { fn default() -> Self { Self::Pending } }",
        ] {
            let source = format!("{HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT}\n{prohibited_impl}");
            assert!(
                validate_handler_value_primitives_source(&source).is_err(),
                "prohibited trait impl was accepted: {prohibited_impl}"
            );
        }

        let extra_method = HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT.replacen(
            "impl HandlerFootprint {",
            "impl HandlerFootprint {\n    pub const fn unexpected(self) -> u64 { 0 }",
            1,
        );
        assert!(validate_handler_value_primitives_source(&extra_method).is_err());
    }

    #[test]
    fn handler_value_source_rejects_extra_derives() {
        let cloneable_step = HANDLER_VALUE_PRIMITIVES_SOURCE_CONTRACT.replacen(
            "#[derive(Debug, Eq, PartialEq)]\n#[must_use]\npub enum HandlerStep",
            "#[derive(Clone, Debug, Eq, PartialEq)]\n#[must_use]\npub enum HandlerStep",
            1,
        );
        assert!(validate_handler_value_primitives_source(&cloneable_step).is_err());
    }
}
