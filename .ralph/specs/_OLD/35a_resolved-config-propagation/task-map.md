# Task Map: 35a_resolved-config-propagation

This packet covers exactly one backlog task ID (`TASK-166`) but explicitly closes a deviation (`DEV-040`) surfaced during packet `35_multi-layer-top-bottom-thickness`. The mapping below lets a reviewer trace every step back to that single task and confirms which authoritative docs govern each step.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-166` | Step 1 | `docs/DEVIATION_LOG.md` (DEV-040 row) | `crates/slicer-host/src/dispatch.rs:1640-1660` (read-only confirmation) | none | `S` | Pre-flight FACT-only check; halts the packet if the producer-side default has already been changed elsewhere. |
| `TASK-166` | Step 2 | `docs/02_ir_schemas.md` §`ResolvedConfig` (lines `~575-660`) | `crates/slicer-host/src/config_resolution.rs` (NEW), `crates/slicer-host/src/lib.rs`, `crates/slicer-host/tests/config_resolution_tdd.rs` (NEW) | none | `M` | Implements `resolve_global_config`, `resolve_per_object_configs`, `ConfigResolutionError`. Sufficient evidence for AC-1, AC-2, AC-3, NC-1. |
| `TASK-166` | Step 3 | `docs/04_host_scheduler.md` §`RegionMapIR Compilation` (delegate SUMMARY) | `crates/slicer-host/src/region_mapping.rs` | none | `S` | Stamp authority moves from `LayerPlanIR.resolved_config` (module-emitted) to host built-in. |
| `TASK-166` | Step 4 | `docs/04_host_scheduler.md` §`PrePass lifecycle` | `crates/slicer-host/src/prepass.rs:300-360`, `crates/slicer-host/tests/region_mapping_resolved_config_tdd.rs` (NEW) | none | `S` | Mechanical wiring + integration test for AC-4. |
| `TASK-166` | Step 5 | none new | `crates/slicer-host/src/pipeline.rs:30-150`, `crates/slicer-host/src/main.rs:100-260`, `crates/slicer-host/src/prepass.rs` | none | `M` | Threads the resolved-configs map from the CLI entry point through to the prepass call sites. CLI exits non-zero on `ConfigResolutionError`. |
| `TASK-166` | Step 6 | none new | `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` | none | `M` | Adds the binary E2E acceptance test (AC-5) and the CLI rejection negative case (NC-2). |
| `TASK-166` | Step 7 | none | workspace gates only | none | `S` | `cargo build`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`. Reconciles any pre-existing failures (e.g. tree-support IR-access regression noted in packet 35 closure) as either fixed here or confirmed unrelated. |
| `TASK-166` | Step 8 | `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` | doc-only edits | none | `S` | Flip DEV-040 → `Closed`; mark TASK-166 → `[x]`; flip `packet.spec.md` → `status: implemented`. |

Aggregate context cost (sum across rows): **M** (3 × M + 5 × S). No row is `L`. Packet is approved for activation as a single Ralph slice.

## Cross-Packet Reconciliation

- Packet 35 (`35_multi-layer-top-bottom-thickness`) remains `implemented`. This packet does NOT modify any file under `.ralph/specs/35_*/`. The CONSUMER-side fix in packet 35 is correct as-is; this packet adds the missing PRODUCER-side plumbing. The DEV-040 row is the canonical bridge between the two packets.
- Packets `36_bridge-detector-orca-parity` and `37_fill-role-claims` (currently `draft`) are unblocked once this packet flips to `implemented` and DEV-040 closes. Their `Prerequisites and Blockers` sections may already cite DEV-040; confirm via FACT dispatch at packet completion (Step 8) that the unblock chain is reflected there.

## Why This Single-Task Packet Needs a Task Map

- The packet covers one task ID, but it explicitly closes a deviation surfaced by an earlier packet. A reviewer needs the per-step trace to confirm that producer-side plumbing (this packet) and consumer-side classification (packet 35) together satisfy DEV-040.
- The packet introduces a new flat CLI key family (`object_config:<id>:<key>`) that does not exist in `docs/07` or `docs/03` schemas today; the task map records that the key family is host-resolver-only and confirms the precedent (`object_height:<id>`) in Step 5.
- Three workspace files (`docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md`, the packet's own `packet.spec.md`) move state in Step 8; the task map identifies that Step 8 is doc-only and never opens unrelated rows.
