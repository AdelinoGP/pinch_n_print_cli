# Requirements: 109_perimeter-m1-verification

## Packet Metadata

- Grouped task IDs:
  - `T-100` — Build reference-fixture parity harness at `crates/slicer-runtime/tests/integration/perimeter_parity.rs`
  - `T-101` — Record 6 OrcaSlicer reference outputs (solid square, holed square, multi-tool triangle, overhang ramp, bridge, spiral-vase cone)
  - `T-102` — TDD sweep for 7 edge cases (3-tool polygon, inner-wall material boundary, 0/2-vertex polygon, hole-with-thin-wall, gap-fill-in-overhang, top-flagged region, first-layer override)
  - `T-103` — Walk every M1 deviation entry from P102+; close each with implementing packet ID or justify residual
  - `T-104` — Update `docs/07_implementation_status.md` to mark Classic parity complete; flip roadmap M1 marker
  - `T-105` — Run `cargo test --workspace` at M1 close (CLAUDE.md exception for closure ceremony)
  - `T-P96-A` — Reshape AC-22b assertion in `cube_4color_gcode_output_tdd.rs`; rename test to `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes`
  - `T-P96-C3` — Golden-file parity check for cube_4color G-code output vs OrcaSlicer reference
  - `T-P96-D` — Delete unused `SlicedRegion.external_contour` IR field (cascade through 8 sites — see design.md); schema bump from live 4.3.0 by one minor, computed at activation (NOT hardcoded; P105 may bump first)
  - `T-P96-F` — Capture `P<packet>_CUBE_4COLOR_PARITY_SHA`; add `D-<packet>-AC22-PARITY-RESHAPE` superseding `D-96-AC22-EXTERNAL-CONTOUR`
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

M1 of the perimeter parity roadmap is PLANNED but not yet shipped: P104–P108 are all `status: draft` as of 2026-06-19 (verified by grep). This packet is the M1 closure gate — it cannot activate until P102–P108 are implemented. Once the chain ships, this packet builds the end-to-end verification layer: without recorded OrcaSlicer reference outputs and a parity harness, regressions during M2 work (Voronoi + SkeletalTrapezoidation + BeadingStrategy stack) will land undetected. The audit also enumerated 7 edge cases that lack regression coverage (3-tool polygon, inner-wall material boundary, 0/2-vertex polygon, hole-with-thin-wall, gap-fill-in-overhang, top-flagged region, first-layer override). Finally, the P96 inherited reshape obligation (T-P96-A) leaves the 4-color cube TDD in a divergent state; `external_contour` remains live in `SlicedRegion` (populated by `populate_external_contours` in `bisector_ownership.rs`, with tests at lines ~178-247 and accessed via `views.rs:391/399`) and must be removed as part of T-P96-D once P105's `bisector_edge_skip_mask: Vec<bool>` (flat per-edge, ADR-0013 LOCKED) ships.

This packet closes all four concerns. The parity harness + 6 recorded fixtures give M2 a regression bed; the 7 edge-case TDDs lock down propagation correctness; the cube_4color test gets its final renamed-and-rebased state with a new SHA captured under the packet's deviation entry; `external_contour` is removed end-to-end (IR + WIT + host populator + view accessor — ~5 files); and `docs/07_implementation_status.md` records M1 as shipped.

## In Scope

- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (NEW) — parity harness with per-field tolerance comparator (wall count exact; XYZ within 0.005 mm; width within 0.01 mm; loop_type/role exact); supports loading JSON-serialized reference `PerimeterIR` fixtures.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/{solid_square,holed_square,multi_tool_triangle,overhang_ramp,bridge,spiral_vase_cone}/` (NEW) — each contains `mesh.stl` (or analogous), `config.toml`, `expected_perimeter_ir.json`.
- `crates/slicer-runtime/tests/integration/perimeter_edge_cases.rs` (NEW) — 7 edge-case TDDs.
- `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` — rename + reshape per T-P96-A; assertion logic per ADR-0013.
- `crates/slicer-ir/src/slice_ir.rs` — delete `external_contour` field (live value: `Option<Vec<ExPolygon>>`); bump `CURRENT_SLICE_IR_SCHEMA_VERSION` by one minor above the live base (currently `4.3.0`; exact value computed at activation — do NOT hardcode); serde compat via `#[serde(default)]` on a phantom skip-deserialize helper, or clean removal if no committed fixture relies on the old shape. **FORWARD-DEP on P105**: `bisector_edge_skip_mask: Vec<bool>` (flat per-edge per ADR-0013) must exist in P105 before this deletion is justified.
- `crates/slicer-schema/wit/deps/ir-types.wit` — remove `external-contour` accessor from `slice-region-view` + `external-contour` field from `sliced-region` record.
- `crates/slicer-wasm-host/src/host.rs` — remove `external_contour` populator.
- `crates/slicer-sdk/src/views.rs` — remove `external_contour()` accessor (line ~399) and `set_external_contour()` setter (line ~391). Both are live as of 2026-06-19.
- `crates/slicer-core/src/algos/paint_segmentation/bisector_ownership.rs` — remove `populate_external_contours` function (line 64), its 3 tests (lines ~178-247), and the `external_contour` field assignments (lines 69, 101). Also remove the call site in `paint_segmentation/mod.rs:840`.
- `crates/slicer-core/src/algos/prepass_slice.rs` — remove `external_contour: None` field initializer (line ~356).
- `bisector_edge_skip_mask` consumption: this packet consumes the flat mask via `edge_offset_for_polygon(region: &SlicedRegion, poly_idx: usize) -> usize` produced by P105 in `perimeter_utils`. Both symbols are **FORWARD-DEP on P105** (not present in tree as of 2026-06-19). Any reference to nested `Vec<Vec<bool>>` is non-conformant with ADR-0013 — use flat `Vec<bool>` only.
- `docs/07_implementation_status.md` — Classic parity complete entry.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — M1 milestone marker flipped to DONE.
- `docs/DEVIATION_LOG.md` — closure pass + supersession + new entry.
- `docs/02_ir_schemas.md` — 4.4.0 bump rationale.

## Out of Scope

- M2 work (Voronoi + SkeletalTrapezoidation + BeadingStrategy stack + real Arachne port) — separate roadmap and separate packets.
- Sibling roadmap work (overhang-pipeline-restructuring, spiral-vase-and-non-planar-pipeline) — out of M1 scope.
- New perimeter-module feature work — all done by P102–P108.
- Recording M2 reference fixtures — that's M2's verification packet.
- Adding new edge cases beyond the 7 audit-called-out cases — future audits can extend the suite.

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~700 lines | Range-read Phase 9 + "Inherited from P96" rows. |
| `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` | ~80 lines | Read full — guides AC-22b reshape. |
| `docs/02_ir_schemas.md` | ~900 lines | Range-read schema-versioning section + SlicedRegion. |
| `docs/07_implementation_status.md` | varies | Range-read current M1 status section. |
| `docs/DEVIATION_LOG.md` | varies | Range-read recent M1 entries. |
| `CLAUDE.md` | ~600 lines | Read §"Test Discipline" — confirms workspace-test ceremony exception for T-105. |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- For each of the 6 reference fixtures in T-101, delegate one SUMMARY (≤ 100 words) describing the expected `PerimeterIR` shape (wall count, role distribution, loop_type distribution) for the given mesh + config. No code reads. The recorded reference fixtures are JSON files generated from these expectations.

## Acceptance Summary

- Positive cases: `AC-1` (harness self-test), `AC-2` (6 reference fixtures pass), `AC-3` (7 edge-case TDDs pass), `AC-4` (cube_4color reshape green), `AC-5` (external_contour deletion clean + cascade complete), `AC-6` (M1 deviations closed per corrected file targets + cube SHA captured), `AC-7` (workspace test ceremony green).
- Negative cases: `AC-N1` (deliberate-mismatch detected by harness).
- Refinements not captured in Given/When/Then:
  - Schema bump from live `4.3.0` base is **additive removal** — exact minor version computed at activation (P105 may bump first). Old fixtures must still parse via `#[serde(default)]`. If the implementer chooses `#[serde(skip_deserializing)]` instead, document in closure log. Do NOT hardcode a SemVer literal in any AC.
  - `perimeter_parity.rs` and all 6 fixture subdirectories are net-new work in this packet (not pre-existing).
  - `D-10` and `D-12` live in `docs/specs/perimeter-modules-orca-parity-roadmap.md`, not in `docs/DEVIATION_LOG.md`. T-103 closure pass must update the roadmap for these IDs.
  - `D-104-OVERHANG-QUARTILE-NONE` (correct ID per P104 task-map) is the ID to close in `docs/DEVIATION_LOG.md` (registered there by P104). `D-OVERHANG-QUARTILE-NONE` is not the correct format.
  - The "expected_perimeter_ir.json" reference files are committed to the repo so test runs are deterministic; do NOT regenerate them from OrcaSlicer at test time.
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md`'s M1 marker may be in a §"Milestone Summary" block; if absent, the implementer adds it.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Cross-crate compile after IR removal | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration perimeter_parity` | AC-2 (6 fixtures) | FACT pass/fail per fixture |
| `cargo test -p slicer-runtime --test integration perimeter_parity_harness_self_test` | AC-1 + AC-N1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration perimeter_edge_cases` | AC-3 (7 edge cases) | FACT pass/fail per case |
| `cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes` | AC-4 | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence after IR removal | FACT clean / STALE list |
| `cargo test --workspace 2>&1 \| tee target/test-output.log \| tail -5` | T-105 / AC-7 closure ceremony | FACT (summary line + count) |

## Step Completion Expectations

- Cross-step invariant: every prior M1 packet's regression tests must stay green throughout. If a prior test fails after the `external_contour` removal, it's a signal that the prior packet's revert (T-P96-B in P105) missed a call site; trace and fix.
- Step ordering rationale: harness first (Step 1) so AC-1 can be falsified before recording fixtures (Step 2). Edge-case TDDs (Step 3) and cube_4color reshape (Step 4) are independent and can run in either order. `external_contour` deletion (Step 5) MUST come after all consumer reverts are confirmed clean (P105 + this packet's edge-case + reshape work). Doc closure (Step 6) only after every implementation step is green. T-105 workspace ceremony (Step 7) is the final gate.
- Shared scratch state: the recorded `expected_perimeter_ir.json` fixtures are written once in Step 2 and read many times; subsequent steps must not edit them. If a regression in Step 3-5 makes a fixture stale, the implementer halts and traces the regression (do NOT just re-record the fixture — that masks the regression).

## Context Discipline Notes

- This packet has 7 steps. The largest is Step 2 (6 reference fixtures, each requires an OrcaSlicer SUMMARY dispatch + a JSON fixture write).
- `crates/slicer-ir/src/slice_ir.rs` is ~1700 LOC — range-read by `rg -n 'external_contour|SlicedRegion|CURRENT_SLICE_IR_SCHEMA_VERSION'`.
- `docs/DEVIATION_LOG.md` may have grown significantly during M1 — `wc -l` first; range-read by `rg -n 'D-1[0-9]|D-OVERHANG|D-96-AC22' docs/DEVIATION_LOG.md`. Do NOT load full.
- Likely temptation: re-read OrcaSlicer source for the 6 reference fixtures. **Delegate SUMMARYs** instead; the recorded JSON files are derived from documented OrcaSlicer behavior, not from reading source.
- Sub-agent return-format for the heaviest dispatch: per-fixture SUMMARY (≤ 100 words each); 6 dispatches total. If any returns > 150 words or includes code, re-dispatch tighter.
