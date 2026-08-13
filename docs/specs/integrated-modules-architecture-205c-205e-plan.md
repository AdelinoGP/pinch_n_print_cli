# Integrated Modules Architecture Remediation — packets 205c–205e

Approved scope from the architecture review: reduce the carried architectural debt from the integrated-modules effort without changing module behaviour, edition membership, or the parity contract.

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 205c-native-dispatch-seam | Make native and WASM dispatch share one marshalling authority, remove duplicate held-claim resolution, close lossy native commits, and validate dispatch mode at load time. | TASK-329 | 205b implemented | blocked | docs/spec_packets/205c-native-dispatch-seam/ |
| 2 | 205d-integrated-registry | Derive integrated registrations, native entries, and coverage checks from one registry authority without changing feature or edition semantics. | TASK-330 | 205c implemented | pending | docs/spec_packets/205d-integrated-registry/ |
| 3 | 205e-integrated-parity-harness | Consolidate integrated parity setup and comparator scaffolding while preserving all 21 parity gates and negative self-tests. | TASK-331 | 205c and 205d implemented | pending | docs/spec_packets/205e-integrated-parity-harness/ |

The plan and packet directories should be committed together. These packets generate artifacts only; implementation is a downstream `/swarm` activity.
