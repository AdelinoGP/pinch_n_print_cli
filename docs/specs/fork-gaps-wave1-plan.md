# Plan: First-wave PNP packets for the OrcaSlicer-frontend fork

## Context

The OrcaSlicer-frontend fork replaces in-process slicing with `pnp_cli slice` shell-outs. Fork tickets 011–013 locked the framing: the fork ships **no UI restrictions and no user-visible gap warnings**, so every PNP capability gap fails silently for the user. The handoff (grilled 2026-07-17) lists 16 items; this session packets the **first wave only** — the hard blocker, the mandatory-correctness item, the highest-visibility quality gap, and the stats workstream. A second plan+grill session will generate the remaining packets (raft, flavor, MM proof, per-object keys, cancel, thumbnails, M73, support preview).

All factual claims were verified against the codebase. Two corrections to the handoff:
- **Item 8 overstated**: seam-placer already *prefers* `seam_candidates` and falls back to `resolved_seam` (`modules/core-modules/seam-placer/src/lib.rs:242-248`). The remaining TASK-120c risk is the rotated-wall sibling-erasure path.
- **Item 6 easier than framed**: the loader already bakes full transforms into vertices (`apply_transform_to_mesh`, `crates/slicer-model-io/src/loader.rs:457`, plus `apply_transform_to_paint_data`); non-uniform scale is a deliberate policy rejection in `validate_non_uniform_scale` (`loader.rs:2551`).
- **New hazard found**: `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs:402-475`) already emits *fake* `machine_max_*` speed/accel/jerk values as cosmetic padding. Orca's `GCodeProcessor` trusts CONFIG_BLOCK values, so today's padding actively feeds the viewer wrong machine limits when the fork doesn't override them.

## Deliverable of this session

Author **5 spec packets** under `.ralph/specs/` via `/spec-packet-generator` (Batch Protocol), each then gated with `/spec-review <packet> --preflight` and executed with `/swarm`. Packets C, G, 16 touch disjoint crates and can swarm **in parallel**; A depends on G for accuracy validation; 8 is independent of 16 but same module — run after 16 to avoid churn.

## Packet C — Non-uniform scale support (item 6, hard blocker)

- Delete/neutralize `validate_non_uniform_scale` (`crates/slicer-model-io/src/loader.rs:2551-2567`) and the `NonUniformScaleUnsupported` error variant (`loader.rs:49`); let the existing transform-baking path handle per-axis scale.
- **Include a downstream audit**: verify nothing assumes uniform scale (paint-data radii, normals, any place that extracts a single scale factor). Grep for `NonUniformScale`, uses of transform scale extraction.
- Tests: load a 3MF with a non-uniform-scale transform, assert vertices baked per-axis; regression test that uniform behavior is unchanged.

## Packet G — CONFIG_BLOCK viewer keys (item 10, mandatory correctness)

Contract: **the fork supplies real values via raw_config** (verbatim passthrough already works, `serialize_config_block`, `serialize.rs:283-382`). PNP side:
- Remove the misleading `machine_max_*` / speed / accel / jerk entries from `ORCA_CONFIG_PADDING`; keep only truly cosmetic keys. Padding still targets Orca's ~80-key minimum gate — verify the gate still passes with the reduced table (add neutral cosmetic keys if count drops too low).
- Synthesize `printer_model` with a safe non-"Bambu Lab" default when absent (guards `s_IsBBLPrinter`, which defaults to `true` on drag-in).
- Document the required key list (`printer_model`, `filament_density`, `filament_cost`, `printable_area`, `nozzle_diameter`, `machine_max_*` family) in the G-code emit docs as the fork-facing contract.

## Packet 16 — `aligned` / `aligned_back` seam modes (highest per-slice visibility)

Orca's default `seam_position` is `spAligned`; PNP falls back to `nearest` on every untouched-settings slice — a visible quality regression with no user-facing signal.
- **Full Orca-parity port** of canonical `SeamPlacer`'s aligned path (seam-string chaining across layers with visibility/angle penalties and least-squares smoothing) — decided over the simple per-object-anchor accumulator. `aligned_back` = same machinery with rear-biased seeding.
- Extend `SeamMode` enum (`modules/core-modules/seam-placer/src/lib.rs:32-40`) with `Aligned`, `AlignedBack`; selection consumes `region.seam_candidates()` (the mechanism already preferred by `run_wall_postprocess`).
- **OrcaSlicer attribution header required** (`docs/ORCASLICER_ATTRIBUTION.md`); cite canonical code by file + function name, never line numbers. Coordinate hazard: 1 unit = 100 nm — divide Orca constants by 100.
- Guest WASM: seam-placer is a core module — packet must run `cargo xtask build-guests` and the freshness check.

## Packet A — Time estimator + slice_stats + layer_count (items 1, 2, 12)

- **Estimator**: acceleration-aware simplified trapezoidal model (Marlin-style) using `machine_max_acceleration_*` / `machine_max_speed_*` / jerk from config. Lives as a **post-emit analysis pass in `crates/slicer-gcode`** (where `resolve_feedrate`, `emit.rs:144-185`, and `PrintMetadata` already live) — no new WASM module. One walk over emitted moves computes: total time, per-extruder extruded volume map, filament length, toolchange count. Fills `estimated_print_time_s` (currently hardcoded 0 at `emit.rs:739`).
- **`slice_stats` event**: emit before `slice_complete`, bumping the progress-event schema to 1.2.0 (`docs/09_progress_events.md:153`). Fields: the five reserved ones (`gcode_prediction_seconds`, `gcode_weight_grams`, `gcode_filament_length_mm`, `layer_count`, `first_layer_height_mm`) **plus** per-extruder volume breakdown (mm³ map) and toolchange count. **No cost field** (fork computes cost from its own preset).
- **`layer_count` in `phase_start(per_layer)`**: additive optional field (minor-bump-safe per the schema rules) so the fork's progress bar is exact during the slice, not only at the end.
- Depends on Packet G for accuracy validation (real machine limits in test fixtures).

## Packet 8 — Seam live-path audit (TASK-120c residue)

- Scope narrowed by the code check: candidates-vs-resolved_seam preference is already correct. Remaining work: verify/fix the **rotated-wall replacement erasing sibling walls** unless the full region wall set is re-emitted; close or re-scope TASK-120c in `docs/07_implementation_status.md`.
- Correctness-audit packet: reproduce with a multi-wall region fixture, assert all sibling walls survive seam rotation.

## Deferred to wave 2 (explicit)

Items 3 (raft — ADR-0009 already specced), 4 (TASK-210/211/212 MM proof), 5 (flavor: klipper→reprapfirmware order), 9 (per-object key parity), 11 (graceful cancel), 13 (support preview), 14 (thumbnails PNG-first), 15 (M73 — strictly downstream of Packet A's estimator). Item 7 closed won't-fix.

## Domain model / glossary updates (CONTEXT.md)

Add terms as they land (packet acceptance ceremony, not now): **Seam Mode** (incl. Aligned/AlignedBack semantics), **Slice Stats event**, **Print-Time Estimator** (post-emit analysis pass, not a module), **Viewer-Config Passthrough contract** (fork-supplied CONFIG_BLOCK keys). No ADR needed for C/G/A (reversible, unsurprising); the aligned-seam port fidelity choice (full parity over accumulator) may warrant a short ADR if the port forces IR changes.

## Verification

- Per packet: narrow tests per Test Discipline (`cargo test -p <crate> --test <file>`), tee to `target/test-output.log`; `cargo clippy --workspace --all-targets -- -D warnings` before commit; `cargo xtask build-guests --check` after any seam-module/WIT-adjacent edit.
- End-to-end for G+A: slice a fixture (`resources/benchy.stl`) with fork-realistic raw_config, assert CONFIG_BLOCK contains real (not padded) machine limits, `slice_stats` JSONL event present with all fields, `estimated_print_time_s > 0`, non-zero per-extruder volumes.
- End-to-end for C: slice a non-uniformly-scaled 3MF to G-code without error; golden-compare geometry against a pre-baked equivalent mesh.
- Packet-close: `cargo xtask test --workspace` only at each acceptance ceremony, dispatched to a sub-agent with FACT pass/fail.

## Next step after approval

Run `/spec-packet-generator` in Batch Protocol mode over this plan to author the 5 packets (C, G, 16, A, 8), then `/spec-review --preflight` each.

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 166-nonuniform-scale-bake | Delete the dead validate_non_uniform_scale rejection (zero production call sites — grounding correction) and prove per-axis scale baking with tests + downstream audit. | TASK-272 (new) | - | generated | packet 166 (archived) |
| 2 | 167-config-block-viewer-keys | Purge 34 speed/accel/jerk-valued padding keys (no machine_max_* existed — grounding correction), synthesize non-BBL printer_model, document the fork-facing viewer-key contract. | TASK-273 (new) | - | generated | .ralph/specs/167-config-block-viewer-keys |
| 3 | 168-seam-aligned-modes | Port OrcaSlicer SeamPlacer's aligned/aligned_back path into the seam-planner-default prepass (grounding redesign, user-approved: per-layer seam-placer holds no cross-layer state; WIT run-seam-planning gains layer-plan-view, major world-prepass bump, ADR-0046, D-168-SEAM-PREPASS-SOURCE); seam-placer gains Aligned/AlignedBack snap variants. | TASK-274 (new) | - | generated | .ralph/specs/168-seam-aligned-modes |
| 4 | 169-time-estimator-slice-stats | Add an acceleration-aware trapezoidal time estimator as a post-emit analysis pass in slicer-gcode and emit the slice_stats 1.2.0 event (amended fields) plus layer_count in phase_start(per_layer); also creates the missing production slice_complete emission (grounding addition — it had zero call sites). | TASK-275 (new) | #2 | generated | .ralph/specs/169-time-estimator-slice-stats |
| 5 | 170-seam-livepath-audit | Verify/fix rotated-wall replacement erasing sibling walls and reconcile the existing reopened TASK-120c row. | TASK-120c | #3 | generated | .ralph/specs/170-seam-livepath-audit |
