# Task Map: 234a-internal-bridge-support-gating

## ISSUE-82 split (explicit)

Backlog issue `docs/specs/orca-feature-gap/issues/82-author-packet-p75-quality-bridging-bridge-over-infill.md` owns the P75 internal-bridge-over-infill key set (`dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle`). Packet 233 delivered the construction seam and angle port; THIS packet covers the **filtering-parity half** — canonical lower-layer support qualification, candidate-source correction, and the ShellClassification relocation. `enable_extra_bridge_layer` remains parse-only (unchanged from 233); `internal_bridge_angle` pass-through semantics are unchanged. ISSUE-82 stays open after this packet for any residual coverage/anchoring parity work.

## Crosswalk

This packet has no `docs/07_implementation_status.md` task ID; the docs/07 crosswalk is therefore N-A (backlog = ISSUE-82 filtering half). Rows record each step's backlog source, docs, code surface, and canonical refs.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A (backlog: ISSUE-82 filter half) | Step 1 | `docs/specs/bridge-parity-plan.md` §3/F3; `docs/15_config_keys_reference.md` entry | `crates/slicer-core/src/algos/bridge_over_infill.rs` (`unsupported_span_areas`, `qualify_internal_bridge_surface`), net-new `crates/slicer-core/tests/bridge_support_gating_tdd.rs` + Cargo.toml [[test]] | `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (`bridge_over_infill` gather lambda), `PrintConfig.hpp/.cpp` (`InternalBridgeFilter`) | `M` | Pure math port; Q1/Q2 discovery recorded before any runtime edit. |
| N-A (backlog: ISSUE-82 filter half) | Step 2 | `docs/04_host_scheduler.md` ShellClassification section | `crates/slicer-runtime/src/layer_executor.rs` (arm removal), `crates/slicer-runtime/src/slice_postprocess_prepass.rs` (relocated pass) | — (placement mirrors 234's gate precedent) | `S` | Ordering locked: support gating strictly after 234's gate. |
| N-A (backlog: ISSUE-82 filter half) | Step 3 | `docs/specs/bridge-parity-plan.md` §6 invariants | `resources/calicat.stl` import, net-new e2e test through e2e aggregator | — (bar frozen from authoring-session measurements) | `M` | AC-5 bar frozen 2026-08-24: ≤6 layers, ≤5000 mm, external Z≈3.2 ∈ [85°,95°]. |
| N-A (backlog: ISSUE-82 filter half) | Step 4 | `docs/15_config_keys_reference.md`; `docs/specs/bridge-parity-plan.md` §3/F3 addendum | doc-only edits (config-key semantics row; F3 addendum pointer) | — | `S` | Doc Impact execution; both verification greps must hit before closure. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
