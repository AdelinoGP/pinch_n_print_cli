# Task Map: 25_wit-canonical-surface-lock → docs/07

This packet maps to Workstream 1 remediation tasks TASK-144 and TASK-145 (WIT canonical source consolidation and drift lock).

## Task ID Mapping

| Packet Step | docs/07 Task | Notes |
|---|---|---|
| Step 2 | TASK-144 | Update `wit/world-prepass.wit` segmentation signatures |
| Step 3 | TASK-144 | Add seam members to `wit/deps/ir-types.wit` |
| Step 4 | TASK-145 | Expand `wit_drift_detection_tdd.rs` with specific signature assertions |
| Step 5 | TASK-144 | Update `docs/03_wit_and_manifest.md` perimeter sections |
| Step 6 | TASK-144 | Rebuild WASM artifacts if bindings changed |

## Superseding Relationship

- Packet 25 does NOT supersede any prior packet. It continues the WIT consolidation work from packets 03, 03-rev1, and 03-rev2, focusing specifically on the prepass segmentation and seam surface that were identified in the review findings.

## Parallelism Note

- Packet 25 can run in parallel with Packet 24 after the Step 2 vocabulary decision is settled in Packet 24.
