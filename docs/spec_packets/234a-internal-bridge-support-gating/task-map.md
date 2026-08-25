# Task Map: 234a-internal-bridge-support-gating

## ISSUE-82 split (explicit, closure edition)

Backlog issue `docs/specs/orca-feature-gap/issues/82-author-packet-p75-quality-bridging-bridge-over-infill.md` owns the P75 key set (`dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle`). Packet 233 delivered the construction seam and angle port; the ORIGINAL 234a edition delivered support-math gating and the ShellClassification relocation with one recorded deviation (0 qualifying sites on calicat). THIS closure edition revises the packet in place to terminate ISSUE-82: RC-A arithmetic correction, WIT-visible dense-interior taxonomy (`internal_solid_fill`), qualify-prepass/build-InfillPostProcess venue split, FULL F4 coverage/anchoring parity (expansion zones, depth harvesting, clustering, `enable_extra_bridge_layer` emission semantics), and bundle-primary one-site arbitration. Decisions frozen in `docs/specs/orca-feature-gap/issues/82-parity-closure-decision-brief.md`. After this packet, ISSUE-82 closes with no residual bridge-over-infill parity work.

## Crosswalk

This packet has no `docs/07_implementation_status.md` task ID; the docs/07 crosswalk is therefore N-A (backlog = ISSUE-82 closure edition). Rows summarize; authoritative step contracts live in `implementation-plan.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A (ISSUE-82 closure) | Step 1 | decision brief §0.1/§4 | `crates/slicer-core/src/algos/bridge_over_infill.rs` (`unsupported_span_areas`, `fill_envelope` removal) + `bridge_support_gating_tdd.rs` | `PrintObject.cpp::bridge_over_infill` gather init | `S` | RC-A fix; fixtures re-blessed canonical-correct |
| N-A (ISSUE-82 closure) | Steps 2a–2b | `docs/21_data_defaults_and_fixtures.md`; `docs/19_visual_debug.md` | `slice_ir.rs`, `prepass_slice.rs` literal, WIT region type, `views.rs`, `visual_debug_render.rs` | — | `M` | `internal_solid_fill` WIT-mirrored; `internal_bridge_areas` host-only; serde defaults |
| N-A (ISSUE-82 closure) | Step 3 | `docs/04_host_scheduler.md` (delegated) | `slice_postprocess_prepass.rs`, integration test | density==100 branch | `M` | per-lower-region config resolution |
| N-A (ISSUE-82 closure) | Steps 4a–4c | decision brief Item 3 | `layer_executor.rs` arm, field retirement, contract rewrite | `generate_sparse_infill_polylines_for_anchoring` | `M` | probe-first; AC-4 reversal recorded |
| N-A (ISSUE-82 closure) | Steps 5a–5b, 5c-host | `docs/ORCASLICER_ATTRIBUTION.md`; F4 row; P75 `enable_extra_bridge_layer` (host-side) | core helpers + executor threading; carrier-free duplicate authoring (prepass) + existing InfillPostProcess construction | expansion zones, `gather_areas_w_depth`, clustering | `M` | attribution headers mandatory |
| N-A (ISSUE-82 closure) | Steps 6a–6c | decision brief Items 5–7; `bridge-parity-plan.md` F3/F4 | net-new e2e pair, gcode bars, golden ceremony, doc rows | — | `M` | bundle-primary arbiter; conditional re-bless policy |

Copy costs from `implementation-plan.md`. Aggregate `M`; no row is `L`.

## Closure (2026-08-25)

Steps 1–5b, 5c-host, and 6a–6c delivered. AC-1..AC-7 and N1..N3 green after restatements
(AC-5/AC-6 carry matched-oracle-profile baselines per the owner out-of-scope ruling).
Deviations DEV-149..DEV-153 carry the residual fronts: low-z mid-stack qualification,
cavity coverage breadth, duplicate angle anchor-derived, flow-ratio omission, and
Internal-Bridge role labeling. ISSUE-82 terminus is PARTIAL by owner ruling — machinery
terminus achieved; behavioral parity residuals are owned by the shell-classification,
infill, and support tracks.
