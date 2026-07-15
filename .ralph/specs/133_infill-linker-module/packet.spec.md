---
status: draft
packet: 133_infill-linker-module
task_ids:
  - TASK-258
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 133_infill-linker-module

## Goal

Ship `modules/core-modules/infill-linker/` — the single `Layer::InfillPostProcess` module
(holding new non-fill claim `claim:infill-link`) that reads `prior-infill`, applies the
OrcaSlicer overlap semantics, re-clips via `clip_polylines`, filters short segments, and
connects raw infill into linked polylines per (region, role) and across wall-sharing groups,
emitting the complete replacement `InfillIR`.

## Scope Boundaries

One new guest module containing the ported linking algorithms (`ExPolygonWithOffset`,
`BoundaryInfillGraph`, `connect_infill`, `chain_or_connect_infill`, `remove_short_polylines` —
in-module per ADR-0026), its manifest with `claim:infill-link` + `infill_overlap` config
schema, the claim-catalog entry, and module-level + one pipeline-smoke test. It does NOT
rewrite any infill module (134/135), does not restore carved goldens (136), and adds no CLI
flag (136). The module links whatever the current modules emit — including today's
stub output — from the day it lands.

## Prerequisites and Blockers

- Depends on: `129_clip-polylines` (re-clip primitive), `130_infill-postprocess-contract`
  (prior-infill + six view fields), `131_per-region-config-delivery` (per-region spacing),
  `132_modifier-region-split` (wall-less-sibling fixtures for branch (b) tests).
- Unblocks: `134`/`135` (their raw output is linked immediately), `136` (integration).
- Activation blockers: none — architecture (ADR-0025/0026 + amendments), claim (D4), both
  linking branches, and the overlap-ownership rule are locked.

## Acceptance Criteria

- **AC-1. Given** a region-role bucket of raw 2-point scan-line segments over a 10 mm square,
  **when** the linker runs, **then** the output for that bucket is fewer, longer multi-point
  polylines: every output polyline has ≥ 2 points, and the output polyline count is < the
  input segment count (segments within link range were joined). | `cargo test -p infill-linker -- raw_segments_in_linked_polylines_out 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** the overlap-inset boundary produced by the ported `ExPolygonWithOffset` for
  a region, **when** the linker re-clips and connects, **then** no output point lies outside
  that boundary (containment against the ported boundary, ±2 units). | `cargo test -p infill-linker -- re_clip_to_offset_boundary 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** a 10 mm square wall-inset polygon and spacing s, **when**
  `ExPolygonWithOffset` is built with `infill_overlap = 0.45`, **then** its inner/outer
  boundaries match the hand-computed expectation for the OrcaSlicer semantics verified from
  `FillRectilinear.cpp:388-490` (the verified offset sign + magnitude are recorded as a
  commented constant in the test). | `cargo test -p infill-linker -- expolygon_with_offset_matches_orca_square_case 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** clipped fragments shorter than `0.8 × spacing`, **when** the linker
  filters, **then** they are absent from the output; fragments ≥ 0.8 × spacing survive. | `cargo test -p infill-linker -- short_segment_filter 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** raw segments tagged `SparseInfill` with `speed_factor = 0.8`, **when**
  linked, **then** every output polyline carries `role == SparseInfill` and
  `speed_factor == 0.8` (roles and speeds are never merged across buckets). | `cargo test -p infill-linker -- role_and_speed_preserved 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-6. Given** two same-config wall-less sibling regions (sub-region with
  `wall_source_region_id = Some(base)`), same tool, same role, path-compatible, **when** the
  linker runs, **then** at least one output polyline contains points from both regions'
  input segments (union-then-link connected across), and the merged polyline lands in the
  bucket of the region containing the majority of its length. | `cargo test -p infill-linker -- wall_sharing_same_config_union_link 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-7. Given** two different-config wall-less siblings (different spacing), **when** the
  linker runs, **then** each region's segments link only within their own region along the
  region's own boundary INCLUDING the shared arc, and no overlap inset is applied along the
  wall-less shared arc (segment endpoints adjacent to the shared arc reach it within
  0.5 × spacing — no unfilled band). | `cargo test -p infill-linker -- wall_sharing_diff_config_no_inset_on_shared_arc 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-8. Given** prior `ironing` paths, **when** the linker emits, **then** the output
  `ironing` bucket is path-for-path identical to the input (full re-emit pass-through). | `cargo test -p infill-linker -- ironing_passthrough_identical 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-9. Given** the manifest and claim catalog, **when** grepped, **then**
  `infill-linker.toml` holds exactly `["claim:infill-link"]` and the catalog table in
  `docs/03_wit_and_manifest.md` documents `claim:infill-link`. | `rg -q 'claim:infill-link' modules/core-modules/infill-linker/infill-linker.toml && rg -q 'claim:infill-link' docs/03_wit_and_manifest.md && echo CLAIM-OK`
- **AC-10. Given** a pipeline run with the linker in the module set and raw 2-point sparse
  segments emitted at `Layer::Infill`, **when** `Layer::InfillPostProcess` commits, **then**
  the committed `InfillIR` contains at least one sparse polyline with > 2 points. | `cargo test -p slicer-runtime --test executor -- infill_linker_pipeline_smoke 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** two adjacent regions that each own their walls
  (`wall_source_region_id == None` for both), **when** the linker runs, **then** NO output
  polyline contains points from both regions' segments — wall-backed boundaries are never
  crossed. | `cargo test -p infill-linker -- walls_separated_regions_never_connected 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** two wall-less siblings with different `tool_index`, **when** the linker
  runs, **then** no cross-region connection occurs. | `cargo test -p infill-linker -- different_tool_never_connected 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N3. Given** two loaded modules both holding `claim:infill-link`, **when** the
  scheduler validates, **then** first-winner dedup leaves exactly one active (the existing
  non-fill-claim dedup path covers the new claim). | `cargo test -p slicer-scheduler -- infill_link_claim_dedup 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p infill-linker 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo clippy -p infill-linker --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` + §Amendment 2026-07-01 — binding
  (both linking branches; the overlap-is-linker-concern rule); full read.
- `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` — binding (algorithms
  in-module); full read.
- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 4 (rewritten step 6) —
  load lines for Phase 4 only.
- `docs/08_coordinate_system.md` — delegate SUMMARY (constants ÷100).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp:1580-1818` — `connect_infill`: boundary graph construction, arc-length parametrization, greedy endpoint connection via perimeter walks (the core port).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp:1820-2246` — `chain_or_connect_infill`: nearest-neighbor ordering wrapper.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp:388-490` — `ExPolygonWithOffset`: two-level offset structure; VERIFY the overlap offset sign/direction here before implementing (the spec's "wall-inset minus overlap" phrasing must be validated, not assumed).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp:356-359` — `remove_short_polylines` threshold semantics (0.8 × spacing).

## Doc Impact Statement (Required)

- `docs/03_wit_and_manifest.md` §claim catalog — `claim:infill-link` row (non-fill,
  first-winner dedup) — `rg -q 'claim:infill-link' docs/03_wit_and_manifest.md`
- `docs/01_system_architecture.md` §module inventory / pipeline — infill-linker in the
  Layer::InfillPostProcess position — `rg -q 'infill-linker' docs/01_system_architecture.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
