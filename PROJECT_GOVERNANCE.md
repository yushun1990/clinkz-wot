# Project Governance

This document defines how the ClinkZ-WoT project is planned, reviewed,
and progressed.

It does not define technical architecture decisions. Technical
convergence rules are maintained in `ARCHITECTURE_GOVERNANCE.md`.

## Governance Principles

ClinkZ-WoT separates:

  Concern                            Artifact
  ---------------------------------- ------------------------------
  AI operating behavior              `AGENTS.md`
  Project execution governance       `PROJECT_GOVERNANCE.md`
  Technical convergence governance   `ARCHITECTURE_GOVERNANCE.md`
  Project roadmap                    `PLAN.md`
  Current execution context          `PROJECT_STATE.md`

## Roles and Responsibilities

### AI Agent

Responsible for: - maintaining `PROJECT_STATE.md`; - keeping milestone
progress current; - identifying blockers; - proposing technical
analysis; - recording execution context.

AI agents must not silently change project direction.

### Project Owner

Responsible for: - approving significant direction changes; - validating
milestone completion; - resolving strategic conflicts; - confirming
release readiness.

## Milestone Lifecycle

Milestones are defined in `PLAN.md`.

    OPEN
     |
    IN_PROGRESS
     |
    REVIEW
     |
    CLOSED

Additional states:

    IN_PROGRESS -> BLOCKED
    REVIEW -> REOPEN

## Milestone Update Rules

Milestone status must reflect repository evidence.

Evidence may include: - implementation; - documentation; - tests; -
validation results.

Do not use percentage completion as the primary progress indicator.

## PLAN.md Maintenance Rules

PLAN.md contains: - objectives; - release targets; - milestones; -
dependencies; - milestone status; - acceptance objectives.

PLAN.md does not contain: - session logs; - temporary debugging
information; - detailed design discussions; - architecture decisions; -
governance policies.

## Review Requirements

A milestone entering REVIEW should provide evidence.

Review verifies: - intended goal achieved; - acceptance criteria
satisfied; - implementation matches specifications; - no known
architectural conflict remains.

## Change Management

Changes affecting: - project direction; - milestone objectives; -
ownership boundaries; - release goals;

require explicit review.

Changes affecting technical architecture must follow
`ARCHITECTURE_GOVERNANCE.md`.

## Workspace Transition

Unresolved topics belong in `workspace/`.

Lifecycle:

    OPEN -> DISCUSSING -> DECIDED -> MIGRATED

Stable conclusions must be migrated to authoritative documents.

## AI Session Continuity

Before ending substantial work: - update `PROJECT_STATE.md`; - record
blockers; - record next actions; - ensure milestone status is accurate.

The repository must remain understandable without previous conversation
history.
