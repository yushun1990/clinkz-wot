# ClinkZ-WoT Development Plan

## Current Goal

Deliver ClinkZ-WoT v1:
A protocol-neutral W3C WoT runtime with stable Servient architecture
and Zenoh binding support.

---

# Milestone Overview


| ID | Milestone | Status |
|----|-----------|--------|
| M1 | Architecture Closure | IN_PROGRESS |
| M2 | Core Contract Stabilization | OPEN |
| M3 | Planning and Compilation Pipeline | OPEN |
| M4 | Protocol Binding SPI | OPEN |
| M5 | Servient Runtime Lifecycle | OPEN |
| M6 | Zenoh Binding MVP | OPEN |
| M7 | v1 Release Hardening | OPEN |


---

# M1 Architecture Closure

Status:

IN_PROGRESS


Objective:

Freeze the fundamental architecture boundaries before
large-scale implementation.


Scope:

- Servient ownership model
- Protocol Binding responsibility boundary
- Request/event data flow
- Compiled plan lifecycle
- Runtime ownership
- Module responsibilities


Current understanding:

Completed:

- PB no longer owns handler dispatch
- Servient orchestrates request routing
- Binding handles protocol conversion only
- Startup-only binding composition selected for v1


Remaining:

- finalize architecture documents
- reconcile ADRs
- remove conflicting legacy concepts


Exit Criteria:

- architecture overview approved
- ownership boundaries frozen
- no conflicting runtime flow exists



---

# M2 Core Contract Stabilization

Status:

OPEN


Objective:

Freeze protocol-neutral core contracts.


Scope:

- Handler contract
- Interaction model
- Resource ownership
- Generation model
- Error handling
- Lifecycle primitives


Dependency:

M1


Exit Criteria:

Core APIs are stable and do not contain
protocol-specific assumptions.



---

# M3 Planning and Compilation Pipeline

Status:

OPEN


Objective:

Establish deterministic execution planning.


Scope:

- TD parsing
- capability discovery
- logical plan
- binding plan
- compiled plan lifecycle


Dependency:

M2


Exit Criteria:

A TD can produce an immutable execution plan.



---

# M4 Protocol Binding SPI

Status:

OPEN


Objective:

Define stable binding integration.


Scope:

Client Binding:

- outbound requests
- subscription handling
- response conversion


Server Binding:

- inbound requests
- transport acceptance
- event/property emission


Dependency:

M3


Exit Criteria:

A binding can be implemented without knowing
handler internals.



---

# M5 Servient Runtime Lifecycle

Status:

OPEN


Objective:

Complete runtime orchestration.


Scope:

- startup composition
- activation authority
- cleanup
- scheduling
- application facade


Dependency:

M4


Exit Criteria:

One Servient instance can manage complete lifecycle.



---

# M6 Zenoh Binding MVP

Status:

OPEN


Objective:

Provide first usable binding implementation.


Scope:

- zenoh transport
- request routing
- property interaction
- event emission


Dependency:

M5


Exit Criteria:

Real Thing interaction works through Zenoh.



---

# M7 v1 Release Hardening

Status:

OPEN


Scope:

- test coverage
- documentation
- examples
- API cleanup
- obsolete removal


Exit Criteria:

v1 release candidate.
