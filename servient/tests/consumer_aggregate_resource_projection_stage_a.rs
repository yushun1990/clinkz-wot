#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Disposition {
    Active,
    ZeroContribution,
    Deferred,
    NotApplicable,
}

fn consumer_role(roles: &str) -> bool {
    roles
        .split('|')
        .any(|role| role == "consumer" || role == "all")
}

fn classify(field: &str, kind: &str, roles: &str) -> Option<Disposition> {
    use Disposition::{Active, Deferred, NotApplicable, ZeroContribution};

    if !consumer_role(roles) {
        return Some(NotApplicable);
    }

    let disposition = match kind {
        "document" => match field {
            "document_bytes_max" | "string_bytes_max" | "extension_bytes_max" => Active,
            "json_nesting_depth_max"
            | "json_members_per_object_max"
            | "json_array_items_max"
            | "json_value_nodes_per_document_max" => NotApplicable,
            "generated_effective_document_bytes_max" => ZeroContribution,
            "retained_source_bytes_per_owner_max" | "retained_source_bytes_global_max" => Active,
            "affordances_per_thing_max"
            | "forms_per_context_max"
            | "forms_per_thing_max"
            | "additional_responses_per_form_max" => Active,
            _ => return None,
        },
        "admission" => Active,
        "runtime" => match field {
            "engine_live_bytes_global_max"
            | "largest_contiguous_allocation_bytes_max"
            | "compiled_runtime_bytes_per_thing_max"
            | "compiled_runtime_bytes_global_max"
            | "things_global_max" => Active,
            _ => return None,
        },
        "schema" => Active,
        "uri" => match field {
            "expanded_uri_bytes_max" => Deferred,
            "uri_variables_per_form_max"
            | "uri_template_source_bytes_max"
            | "uri_template_variables_max" => Active,
            _ => return None,
        },
        "security" => match field {
            "provider_probes_per_interaction_max" => ZeroContribution,
            "security_expression_depth_max" | "security_branches_per_plan_max" => Active,
            _ => return None,
        },
        "plan" => {
            if field.starts_with("lazy_") {
                NotApplicable
            } else {
                match field {
                    "binding_and_contributor_probes_per_admission_max"
                    | "wildcard_binding_and_contributor_probes_per_admission_max" => {
                        ZeroContribution
                    }
                    "plan_pins_per_plan_set_max" | "plan_pins_global_max" => Deferred,
                    "compiled_plan_bytes_max"
                    | "form_binding_candidates_per_operation_max"
                    | "plan_sets_per_thing_max"
                    | "plan_sets_global_max"
                    | "logical_plan_bytes_per_thing_max"
                    | "binding_artifacts_per_thing_max"
                    | "binding_artifacts_global_max"
                    | "binding_artifact_bytes_per_item_max"
                    | "binding_artifact_bytes_per_thing_max"
                    | "binding_artifact_bytes_global_max"
                    | "binding_compiler_cursor_bytes_per_item_max"
                    | "binding_compiler_cursor_bytes_global_max"
                    | "plan_compile_work_units_per_step_max"
                    | "plan_reclaim_bytes_per_step_max" => Active,
                    _ => return None,
                }
            }
        }
        "cache" => NotApplicable,
        "payload" | "codec" => Deferred,
        "binding" => match field {
            "bindings_global_max" => Active,
            _ => Deferred,
        },
        "subscription" => NotApplicable,
        "queue" => {
            if field.starts_with("binding_runtime_event_")
                || field.starts_with("binding_reactor_queue_")
            {
                Deferred
            } else {
                NotApplicable
            }
        }
        "cleanup" => match field {
            "cleanup_items_max"
            | "cleanup_bytes_max"
            | "cleanup_item_bytes_max"
            | "cleanup_work_items_per_step_max"
            | "cleanup_retry_records_max"
            | "cleanup_retry_attempts_max" => ZeroContribution,
            "cleanup_transfer_slots_global_max"
            | "cleanup_transfer_bytes_global_max"
            | "binding_cancel_buffer_bytes_per_call_max"
            | "binding_cancel_buffer_bytes_global_max"
            | "host_binding_cancel_drain_timeout_millis_max" => Deferred,
            _ => return None,
        },
        "status" => Deferred,
        "accounting" => match field {
            "accounting_batch_items_max" | "accounting_reconcile_owners_per_step_max" => Active,
            "accounting_idle_items_max" => ZeroContribution,
            "accounting_reconcile_interval_millis_max" => NotApplicable,
            "accounting_reconcile_steps_max" => Active,
            _ => return None,
        },
        "resolver" => ZeroContribution,
        "directory" | "discovery" | "query" | "response" | "emission" | "fanout" | "handler" => {
            NotApplicable
        }
        _ => return None,
    };

    Some(disposition)
}

#[test]
fn every_registered_resource_row_has_a_first_proof_disposition() {
    let csv = include_str!("../../docs/resource-limits.csv");
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some(
            "field,resource_kind,unit,scope,capability_roles,zero_semantics,gateway_default_v1,directory_client_default_v1,benchmark_static_reference_v1,requirements"
        )
    );

    let mut rows = 0usize;
    let mut consumer_rows = 0usize;
    for line in lines.filter(|line| !line.is_empty()) {
        rows += 1;
        let columns: Vec<&str> = line.split(',').collect();
        assert_eq!(columns.len(), 10, "malformed resource row: {line}");
        let field = columns[0];
        let kind = columns[1];
        let roles = columns[4];
        if consumer_role(roles) {
            consumer_rows += 1;
        }
        assert!(
            classify(field, kind, roles).is_some(),
            "unclassified Consumer aggregate resource row: {field} ({kind}, {roles})"
        );
    }

    assert_eq!(
        rows, 195,
        "schema growth must force this Stage-A map to be reviewed"
    );
    assert!(consumer_rows > 0);
}

#[test]
fn review_sensitive_rows_have_the_selected_dispositions() {
    use Disposition::{Active, Deferred, NotApplicable, ZeroContribution};

    assert_eq!(
        classify("string_bytes_max", "document", "all"),
        Some(Active)
    );
    assert_eq!(
        classify("extension_bytes_max", "document", "all"),
        Some(Active)
    );
    assert_eq!(
        classify("json_value_nodes_per_document_max", "document", "all"),
        Some(NotApplicable)
    );
    assert_eq!(
        classify("retained_source_bytes_per_owner_max", "document", "all"),
        Some(Active)
    );
    assert_eq!(
        classify(
            "binding_and_contributor_probes_per_admission_max",
            "plan",
            "producer|consumer"
        ),
        Some(ZeroContribution)
    );
    assert_eq!(
        classify("security_branches_per_plan_max", "security", "all"),
        Some(Active)
    );
    assert_eq!(
        classify("provider_probes_per_interaction_max", "security", "all"),
        Some(ZeroContribution)
    );
    assert_eq!(
        classify("things_global_max", "runtime", "producer|consumer"),
        Some(Active)
    );
    assert_eq!(
        classify("accounting_batch_items_max", "accounting", "all"),
        Some(Active)
    );
    assert_eq!(
        classify("accounting_idle_items_max", "accounting", "all"),
        Some(ZeroContribution)
    );
    assert_eq!(
        classify("cleanup_retry_records_max", "cleanup", "all"),
        Some(ZeroContribution)
    );
    assert_eq!(
        classify("cleanup_items_max", "cleanup", "all"),
        Some(ZeroContribution)
    );
    assert_eq!(
        classify("plan_pins_per_plan_set_max", "plan", "producer|consumer"),
        Some(Deferred)
    );
    assert_eq!(
        classify(
            "cleanup_transfer_slots_global_max",
            "cleanup",
            "producer|consumer"
        ),
        Some(Deferred)
    );
    assert_eq!(
        classify(
            "durable_status_entries_per_binding_max",
            "status",
            "producer|consumer|directory-client"
        ),
        Some(Deferred)
    );
}
