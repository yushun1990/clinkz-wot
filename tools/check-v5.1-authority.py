#!/usr/bin/env python3
"""Validate the active v5.1 Consumer one-shot authority projection."""

from __future__ import annotations

import csv
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "docs/spec/v5-authority-reset.toml"
API_OWNERSHIP = ROOT / "docs/api-ownership.csv"
REVIEWED_CANDIDATE = "3b133ebfe3c870102931982d6c056595f9d44255"


def fail(message: str) -> None:
    raise SystemExit(f"v5.1 authority check: {message}")


def expand_requirement(expression: str) -> list[str]:
    if ".." not in expression:
        if not expression:
            fail("empty requirement expression")
        return [expression]
    first, last = expression.split("..", 1)
    match = re.fullmatch(r"(.+?)(\d{3})", first)
    if match is None or not re.fullmatch(r"\d{3}", last):
        fail(f"invalid requirement range {expression!r}")
    prefix, start_text = match.groups()
    start = int(start_text)
    end = int(last)
    if start > end:
        fail(f"descending requirement range {expression!r}")
    return [f"{prefix}{number:03d}" for number in range(start, end + 1)]


def requirement_rows() -> tuple[set[str], dict[str, dict[str, str]]]:
    result: set[str] = set()
    rows: dict[str, dict[str, str]] = {}
    with (ROOT / "docs/requirements.csv").open(newline="", encoding="utf-8") as source:
        for row in csv.DictReader(source):
            for expression in row["requirement"].split("|"):
                for requirement in expand_requirement(expression):
                    if requirement in result:
                        fail(f"duplicate indexed requirement {requirement}")
                    result.add(requirement)
                    rows[requirement] = row
    return result, rows


def require_metadata(
    rows: dict[str, dict[str, str]],
    requirement: str,
    *,
    source: str | None = None,
    role: str | None = None,
    evidence: str | None = None,
) -> None:
    row = rows.get(requirement)
    if row is None:
        fail(f"missing metadata row for {requirement}")
    if source is not None and row.get("source_path") != source:
        fail(f"{requirement} source_path is not {source}")
    if evidence is not None and row.get("evidence_key") != evidence:
        fail(f"{requirement} evidence_key is not {evidence}")
    if role is not None and role not in row.get("capability_roles", "").split("|"):
        fail(f"{requirement} metadata does not include capability role {role}")


def require_consumer_response_validator_ownership() -> None:
    with API_OWNERSHIP.open(newline="", encoding="utf-8") as source:
        matches = [
            row
            for row in csv.DictReader(source)
            if row.get("item") == "validate_untrusted_binding_output"
        ]
    if len(matches) != 1:
        fail("api ownership must contain exactly one validate_untrusted_binding_output row")
    row = matches[0]
    expected = {
        "kind": "function",
        "defining_package": "clinkz-wot-core",
        "defining_module": "response",
        "visibility": "public",
        "public_path": "clinkz_wot_core::validate_untrusted_binding_output",
        "compilation_cells": "no-default|async-no-std|std",
        "execution_models": "all",
        "resource_profiles": "all",
        "capability_roles": "consumer",
        "requirements": "API-PAYLOAD-001|BIND-OUT-001",
        "current_path": "absent",
        "migration_action": "add",
        "status": "frozen",
    }
    for field, value in expected.items():
        if row.get(field) != value:
            fail(
                "validate_untrusted_binding_output has wrong "
                f"{field}: {row.get(field)!r}; expected {value!r}"
            )
    if row.get("defining_package") == "clinkz-wot-planning" or row.get(
        "public_path", ""
    ).startswith("clinkz_wot_planning::"):
        fail("Consumer response validator must not be Planning-owned")


def main() -> None:
    with MANIFEST.open("rb") as source:
        manifest = tomllib.load(source)

    if manifest.get("schema_version") != 1:
        fail("unexpected manifest schema")
    if manifest.get("current_design_revision") != "5.1":
        fail("current_design_revision must be 5.1")
    if "target_design_revision" in manifest:
        fail("active v5.1 manifest must not retain target_design_revision")
    if manifest.get("status") != "active":
        fail("v5.1 manifest status must be active")
    if manifest.get("decision") != "docs/ADRs/0019-consumer-one-shot-authority-entry.org":
        fail("active decision must be ADR-0019")
    if manifest.get("workspace_basis") != "workspace/0061-consumer-one-shot-domain-entry.md":
        fail("active workspace basis must be workspace/0061")
    if manifest.get("reviewed_candidate_commit") != REVIEWED_CANDIDATE:
        fail("reviewed_candidate_commit does not match independently accepted candidate")
    if "candidate_decision" in manifest or "candidate_workspace_basis" in manifest:
        fail("active manifest must not retain candidate selector fields")
    if manifest.get("classified_requirement_count") != 121:
        fail("classified requirement total must remain 121")
    if manifest.get("active_requirement_count") != 65:
        fail("v5.1 must contain exactly 65 active requirements")

    classification = manifest.get("classification")
    if not isinstance(classification, dict):
        fail("manifest has no classification table")

    classified: set[str] = set()
    active: set[str] = set()
    for name, table in classification.items():
        if not isinstance(table, dict):
            fail(f"classification.{name} is not a table")
        requirements = table.get("requirements")
        expected = table.get("expected_count")
        status = table.get("authority_status")
        if not isinstance(requirements, list) or not all(isinstance(item, str) for item in requirements):
            fail(f"classification.{name}.requirements is invalid")
        if expected != len(requirements):
            fail(f"classification.{name} expected_count does not match")
        for requirement in requirements:
            if requirement in classified:
                fail(f"requirement {requirement} is classified more than once")
            classified.add(requirement)
            if status == "active":
                active.add(requirement)

    if len(classified) != 121 or len(active) != 65:
        fail(f"classification counts are classified={len(classified)} active={len(active)}")
    if classification.get("consumer_one_shot", {}).get("requirements") != [
        "PLAN-REQUEST-001",
        "BIND-OUT-001",
        "API-OPTIONS-001",
    ]:
        fail("consumer_one_shot must contain exactly the three reviewed identities")
    deferred = classification.get("v1_deferred", {})
    if deferred.get("authority_status") != "inactive-domain-entry-review-required":
        fail("v1_deferred has the wrong authority status")
    if deferred.get("expected_count") != 31:
        fail("v1_deferred must contain 31 identities")

    indexed, rows = requirement_rows()
    if indexed != classified:
        fail(
            "authority classification and requirements index differ; "
            f"missing={sorted(classified - indexed)!r} extra={sorted(indexed - classified)!r}"
        )
    require_metadata(
        rows,
        "PLAN-REQUEST-001",
        source="docs/spec/planning.md",
        role="consumer",
        evidence="consumer-request-static-data",
    )
    require_metadata(
        rows,
        "BIND-OUT-001",
        source="docs/spec/binding-spi.md",
        role="consumer",
    )
    require_metadata(
        rows,
        "API-OPTIONS-001",
        source="docs/spec/interaction-core.md",
        role="consumer",
        evidence="consumer-selection-options",
    )
    require_metadata(rows, "BIND-DELIVERY-001", role="consumer")
    require_consumer_response_validator_ownership()

    sources = manifest.get("active_source")
    if not isinstance(sources, list):
        fail("manifest has no active_source records")
    sourced: set[str] = set()
    expected_source_counts = {
        "docs/spec/interaction-core.md": 11,
        "docs/spec/planning.md": 9,
        "docs/spec/binding-spi.md": 12,
    }
    for source in sources:
        if not isinstance(source, dict):
            fail("active_source record is invalid")
        path = source.get("path")
        requirements = source.get("requirements")
        expected = source.get("expected_count")
        if not isinstance(path, str) or path.startswith("/") or ".." in Path(path).parts:
            fail(f"invalid active_source path {path!r}")
        if not isinstance(requirements, list) or not all(isinstance(item, str) for item in requirements):
            fail(f"active_source {path!r} requirements are invalid")
        if expected != len(requirements):
            fail(f"active_source {path} expected_count does not match")
        if path in expected_source_counts and expected != expected_source_counts[path]:
            fail(f"active_source {path} has wrong v5.1 active count")
        source_path = ROOT / path
        if not source_path.is_file():
            fail(f"missing active authority source {path}")
        text = source_path.read_text(encoding="utf-8")
        for requirement in requirements:
            if requirement not in active:
                fail(f"active_source {path} owns inactive requirement {requirement}")
            if requirement in sourced:
                fail(f"active requirement {requirement} has multiple sources")
            sourced.add(requirement)
            if f"`{requirement}`:" not in text:
                fail(f"active source {path} does not define `{requirement}`:")

    if sourced != active:
        fail(
            "active classification and active sources differ; "
            f"missing={sorted(active - sourced)!r} extra={sorted(sourced - active)!r}"
        )

    package_projection = ROOT / "docs/work-packages/CONSUMER-PROPERTY-READ-V5.1-CANDIDATE.md"
    if package_projection.exists():
        fail("candidate package projection must be migrated and removed after activation")
    package_markers = {
        "docs/work-packages/WP-100-core.md": "## v5.1 Consumer Property Read entry slice",
        "docs/work-packages/WP-200-planning.md": "## v5.1 Consumer Property Read entry slice",
        "docs/work-packages/WP-300-bindings.md": "## v5.1 Consumer Property Read entry slice",
        "docs/work-packages/WP-400-servient.md": "## v5.1 Consumer Property Read entry slice",
    }
    for relative, marker in package_markers.items():
        text = (ROOT / relative).read_text(encoding="utf-8")
        if text.count(marker) != 1:
            fail(f"{relative} must contain exactly one activated Consumer entry slice")

    if (ROOT / "tools/check-v5.1-authority-candidate.py").exists():
        fail("candidate authority checker must be removed after activation")
    if (ROOT / "tools/check-architecture-adrs-candidate.sh").exists():
        fail("candidate ADR checker must be removed after activation")

    print("v5.1 authority check: active 65/121 authority, ownership, metadata, and package migration valid")


if __name__ == "__main__":
    main()
