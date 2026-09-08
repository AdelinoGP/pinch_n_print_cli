---
status: implemented
packet: 227-dragon-curve-community-module
task_ids:
  - TASK-338
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 227-dragon-curve-community-module

## Goal

Author the first community module — the dragon-curve sparse-infill tiling with deterministic **per-dragon** tool coloring — at `modules/community-modules/dragon-curve/`. The recorded 225a verdict selects **MoonBit** (see §Authoring-Language Branch), so the module ships as a hand-written MoonBit guest plus a committed `.wasm`.

## Scope Boundaries

This packet owns only the new `modules/community-modules/dragon-curve/` directory and its artifacts: the `dragon-curve.toml` manifest, `wit/` (frozen Layer::Infill WIT closure), `src/dragon/` (pure tiling + colour logic and its tests), `src/glue/` (the WIT glue template), the build `Makefile`, the committed `dragon-curve.wasm`, and the banner `README.md`. It consumes the `tool-index` carrier, `claim:authored-coloring` grant, and `tool-count` query that draft 226 produces but does not re-land them. It touches no workspace Cargo members, no `docs/*.md`, and no host/WIT code.

## Prerequisites and Blockers

- Depends on: draft `225-dragon-curve-feasibility-gate` (the recorded Go/MoonBit verdict that selects the authoring branch), draft `226-authored-coloring-carrier` (the `tool-index: option<u32>` WIT field, `slicer_ir::ExtrusionPath3D.tool_index: Option<u32>`, `claim:authored-coloring`, `fill_authored_coloring`, host `tool-count: func() -> u32`, and SDK `slicer_sdk::host::tool_count()`).
- Unblocks: `228-community-module-docs-banner`.
- Activation blockers:
  - **FORWARD-DEP on draft 225-dragon-curve-feasibility-gate** — the authoring branch (Go vs Rust fallback) is unreadable until 225 records its verdict in `docs/14_submodule_programming_languages.md`.
  - **FORWARD-DEP on draft 226-authored-coloring-carrier** — `ExtrusionPath3D.tool_index`, `slicer_sdk::host::tool_count()`, and the `claim:authored-coloring` grant surface do not exist until 226 lands. AC-7 and its emission test are placed behind this blocker; the pure tiling/color-mapping logic is deliberately 226-free and testable now.

## Authoring-Language Branch (resolved 2026-08-14)

`design.md` offered two branches, A (Go) and B (Rust fallback). **Neither is the
recorded verdict.** `docs/14_submodule_programming_languages.md` §"Re-measurement
under the accommodating host — packet 225a (2026-08-13)" supersedes the original
probes and records: Go **NOT_LOADABLE_OR_CORRECT (terminal)**; MoonBit,
AssemblyScript and C++ all **LOADABLE_AND_CORRECT**; and, under the locked
priority order, **"Dragon Curve authoring language: MoonBit"**. The packet was
authored before that re-measurement. The module is therefore MoonBit, on explicit
user direction.

Consequences, recorded because they change this contract:

- AC-3/4/5/6/N1's verification commands originally named `cargo test --test ...`,
  which cannot run against a MoonBit module. They have been rewritten to the
  equivalent `moon test --target wasm -p slicer/layer-infill/src/dragon -f '<name>'`
  invocations, each of which selects exactly one test. The assertions are
  unchanged.
- There is no `Cargo.toml`, no `#[slicer_module]`, no `slicer-sdk`, and no
  `tests/dragon_*_tdd.rs`. The pure logic lives in `src/dragon/dragon.mbt` with
  `src/dragon/dragon_test.mbt` alongside it, and the WIT glue is a template at
  `src/glue/main.mbt.in` copied into the generated interface package by the
  `Makefile`.
- AC-6's prose names `slicer_sdk::config_resolution::resolve_float`. A foreign
  guest has no SDK; the override-precedence rule is reimplemented as
  `pick_tiling_depth` and unit-tested directly. The behaviour asserted is the
  same.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the module manifest at `modules/community-modules/dragon-curve/dragon-curve.toml`, **when** it is inspected, **then** `[module].id` is `com.example.dragon-curve`, `[stage].id` is `Layer::Infill`, and `[claims].holds` contains exactly `claim:sparse-fill` and `claim:authored-coloring`. | `rg -q '"com\.example\.dragon-curve"' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q 'id = "Layer::Infill"' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '"claim:sparse-fill"' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '"claim:authored-coloring"' modules/community-modules/dragon-curve/dragon-curve.toml`
- **AC-2. Given** the module manifest, **when** its `[config.schema]` is inspected, **then** all six keys are declared with the rectilinear-mirrored spellings `infill_density`, `infill_angle`, `infill_speed`, `line_width` plus the dragon-specific `tiling_depth` and `color_map`, each in snake_case. | `rg -q '\[config\.schema\.infill_density\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.infill_angle\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.infill_speed\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.line_width\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.tiling_depth\]' modules/community-modules/dragon-curve/dragon-curve.toml && rg -q '\[config\.schema\.color_map\]' modules/community-modules/dragon-curve/dragon-curve.toml`
- **AC-3. Given** the dragon tiling helper driven by `(ExPolygon, line_spacing, tiling_depth)` with no RNG, clock, or map-iteration-order dependence, **when** it is run twice over the same sparse polygon, **then** the two emitted segment lists are byte-identical (same segment order, ordinals, and coordinates). | `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'tiling_is_deterministic_across_runs' 2>&1 | rg -q 'Total tests: 1, passed: 1, failed: 0'`
- **AC-4. Given** the pure color-mapping helper `map_tiling_index_to_tool(tile_index, color_map, tool_count)`, **when** `color_map` exceeds `tool_count`, **then** the returned tool is wrapped into `[0, tool_count)` and is deterministic for a fixed `(tile_index, color_map, tool_count)`. **Amended 2026-08-14:** the colour driver is the *dragon instance index*, not the segment ordinal — the region is filled by many dragons on a rep-tile lattice and each dragon prints in one tool, so the tiling is visible in the part. The original per-segment-ordinal signature coloured *within* each dragon and made the tiling invisible. | `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'color_map_wraps_into_tool_count' 2>&1 | rg -q 'Total tests: 1, passed: 1, failed: 0'`
- **AC-5. Given** an `ExPolygon` whose contour encloses one or more holes, **when** the tiling helper runs, **then** no emitted segment endpoint lies inside a hole and every segment lies within the contour minus holes. | `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'holes_are_excluded_from_tiling' 2>&1 | rg -q 'Total tests: 1, passed: 1, failed: 0'`
- **AC-6. Given** a region whose per-region `ConfigView` carries a `tiling_depth` override, **when** the dragon resolves the region's tiling depth through the `slicer_sdk::config_resolution::resolve_float` path, **then** the override value wins over the module-global default. | `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'per_region_tiling_depth_override' 2>&1 | rg -q 'Total tests: 1, passed: 1, failed: 0'`
- **AC-7. Given** draft 226 has landed the `ExtrusionPath3D.tool_index: Option<u32>` field and `slicer_sdk::host::tool_count()`, **when** `run_infill` emits a sparse path, **then** the path carries `tool_index = Some(map_tiling_index_to_tool(...))` wrapped into the host's `tool_count()` range (unconditional `Some`; the host strips ungranted per ADR-0058). **226 has landed; AC-7 is PASS — see below.** | `wasm-tools component wit modules/community-modules/dragon-curve/dragon-curve.wasm | rg -q 'tool-index: option<u32>' && rg -q 'tool_index: tool' modules/community-modules/dragon-curve/src/glue/main.mbt.in`

  **AC-7 status (2026-08-14): PASS, verified end-to-end.**

  226's carrier is present on disk (`slicer_ir::ExtrusionPath3D.tool_index: Option<u32>`, `tool-index: option<u32>` in `crates/slicer-schema/wit/deps/types.wit`, `slicer_sdk::host::tool_count()`, and the `tool-count` host service). The module's glue sets `tool_index` on every emitted path unconditionally and never guards on the grant, per ADR-0058.

  Evidence: a 60 mm cube sliced at `infill_density = 0.6`, `tiling_depth = 12`, `color_map = [0,1,2]`, `filament_density` listing three filaments, and `fill_authored_coloring = ["claim:sparse-fill"]`, with sparse fill routed to `com.example.dragon-curve`, produced **589 tool-change commands across 300 layers using all three tools (T0 x294, T1 x148, T2 x147)**, interleaved with `;TYPE:Sparse infill` blocks. Authored per-path tool indices therefore survive the marshal-boundary grant check, the infill linker, and the G-code emitter. The one-tool-change-per-sparse-block ratio is the expected consequence of 226's linker tool-equality guard grouping same-tool paths.

  **Fixture provenance (this was a review finding — the original wording was not reproducible):** the 60 mm cube is **not** an in-tree resource. It was generated into `target/scratch/cube60.stl`, which is gitignored, so the numbers above cannot be re-derived from a clean checkout. Re-measured against the shipped four-fold build on the in-tree `resources/20mm_cube.obj` (density 0.6, `color_map [0,1,2,3]`, four `filament_density` entries): **283 tool changes across four tools (94 x T0, 94 x T1, 48 x T2, 47 x T3)**, identical at `tiling_depth` 8 and 12, against a core-modules-only control emitting **zero** `^T` lines. Depth changes dragon size, not colour count — four rotations exist at every depth. This supersedes an earlier figure of 189 across three tools, which was measured on the translation-only build with a three-entry `color_map`.

  Earlier negative result, now explained: a `resources/regression_wedge.stl` slice produced zero tool changes because the manifest did not yet declare `filament_density`. `derive_tool_count` (`crates/slicer-wasm-host/src/host.rs`) sources the tool count solely from that key and returns the single-tool default of 1 when a module does not declare it, so every segment mapped to tool 0. The manifest now declares it. (A later wedge run also hit a pre-existing `boostvoronoi` panic that reproduces with core modules only and is unrelated to this module.)

  A host-side assertion at the marshal boundary (extending `crates/slicer-wasm-host/tests/contract/authored_coloring_grant_and_strip_tdd.rs`) would still be a stronger regression guard than an end-to-end slice, since nothing in CI covers this module. **That remains an open follow-up, recorded in §Known Gaps.**

  **Reproduce:** `pnp_cli slice --model <60mm cube> --module-dir modules/core-modules --module-dir modules/community-modules/dragon-curve --config <cfg>.json --output out.gcode` then `rg -o '^T[0-9]+' out.gcode | sort | uniq -c`.

## Negative Test Cases

- **AC-N1. Given** `tool_count == 0` or `color_map == 0` as inputs to `map_tiling_index_to_tool`, **when** the helper is called, **then** it returns `None` (no division-by-zero, no out-of-range tool) and `run_infill` maps `None` to `ExtrusionPath3D.tool_index = None`. | `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon -f 'tool_count_zero_returns_none' 2>&1 | rg -q 'Total tests: 1, passed: 1, failed: 0'`

## Verification

- `cargo check --workspace --all-targets` (proves the packet added no accidental workspace breakage; the new module dir is outside the workspace graph).
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cd modules/community-modules/dragon-curve && moon test --target wasm -p slicer/layer-infill/src/dragon` (the module's own suite; re-derive the count from the run rather than quoting it here). There is no `Cargo.toml` in this directory, so `cargo test` is unrunnable and was removed.

## Authoritative Docs

- `docs/specs/community-modules-dragon-curve-infill.md` - direct read, §§1, 4, 5 (the governing design spec; 279 lines).
- `docs/specs/community-modules-dragon-curve-plan.md` - direct read of the Central Symbol Contract and Grounding Facts (binding; 102 lines).
- `docs/adr/0058-authored-coloring-per-path-tool-carrier.md` - direct read (Accepted; the strip-ungranted rule and linker consequences).
- `docs/14_submodule_programming_languages.md` §Community-module context - delegated SUMMARY for the recorded 225 verdict (the implementer reads only this section, not the whole 172-line file).

## Known Gaps (open at closure)

Recorded rather than quietly closed. None blocks the ACs, but each is a real limit a reader should know.

1. **The `ConfigView` read path has no automated coverage.** `cfg_int`/`cfg_color_map` (`src/glue/main.mbt.in`) call `ConfigView::get_int` / `get`, whose host impls are strictly typed: a `tiling_depth` arriving as a Float returns `None` and the module silently falls back to its default, and a `color_map` that is not a `FloatList` yields an empty list, which disables colouring. AC-6's Given names a per-region `ConfigView`, but the unit test exercises `pick_tiling_depth` directly and never crosses the WIT boundary. Closing this needs a host-side contract test (extend `crates/slicer-wasm-host/tests/contract/authored_coloring_grant_and_strip_tdd.rs`); a MoonBit unit test cannot reach it.

2. **Coverage and density are exact; this was reached by correcting two wrong designs, recorded here because the corrections are instructive.**

   The module tiles the region with **four dragons rotated 90 degrees apart about each lattice point**, on the lattice generated by `e(1+i)` and `e(1-i)` where `e` is the curve's end-vector. Measured on a 60 mm square, at depths 8/10/12: path-length ratio **2.000** against `area / line_spacing` (a complete single cover of the grid, hence the segment length is set to twice the target spacing) and **0/400 empty sample cells at every depth**. Whole-print slice: **99.9% of a rectilinear control** at the same density.

   Two earlier designs were wrong and are recorded so the reasoning is not repeated:

   - **Translation-only placement cannot tile.** A lattice cell holds `2 x 2^n` unit grid edges while one dragon supplies `2^n`, so translated copies cover at most half the edges. This left 65/400 empty cells at depth 12 and was misdiagnosed three times — as a lattice-index problem, a margin problem, and a depth problem.
   - **A region-size depth cap was added to hide those voids, and has been removed.** It shrank dragons until three spanned the region. With an exact tiling it buys nothing (voids are 0 at every depth), and it cost two real things: the dragons became too small to see, and it silently overrode `tiling_depth` including per-region overrides — which made **AC-6 misdescribe shipped behaviour**. Removing it restores the documented meaning of the key, so AC-6 is now fully honest rather than partial.

   A length-only guard cannot detect the failure mode here: an early version scored a perfect 1.00 length ratio while leaving 17% of the region empty and double-extruding the rest. Both guards are therefore kept — `tiling_covers_the_region_at_the_requested_density` (bounded above and below) and `tiling_leaves_no_large_voids` (void fraction, now guarded at 1%).

   **Correction of an earlier record in this document.** A previous revision reported 41% / 75% / 67% coverage and claimed "a user asking for 60% gets roughly 40%". Those figures were wrong — a per-layer extractor stopped at the first `;Z:` marker change after it began accumulating, and this model emits repeated `;Z:` markers within a layer (the `ERR_MALFORMED_LAYER_MARKER` warnings), so it truncated mid-layer. The defect was in the measurement. Whole-print totals are the reliable comparison.

   **Caveat on reproducibility:** the 60 mm cube comes from `target/scratch/cube60.stl`, generated and gitignored. Reproducible in-tree evidence: `resources/20mm_cube.obj`.

3. **Four rotations, so four tools show the tiling.** `Seg.tile` is the rotation index (0-3), which is intrinsic to the dragon rather than derived from lattice position or emission order — so a dragon cannot change colour across regions or layers. With a `color_map` shorter than 4, rotations wrap and some dragons share a tool; `[0,1,2,3]` with four filaments renders the full four-colour tiling.

4. **`filament_density` is a seventh config-schema key not named in the original design.** It is not read by the module, but `derive_tool_count` (`crates/slicer-wasm-host/src/host.rs`) sources the printer's tool count from it and only sees keys a module declares. Undeclared, `tool-count()` returns 1, every dragon maps to tool 0, and authored colouring silently does nothing. AC-2 pins six keys; this is additive and does not violate it.

5. **`max_tiles = 4096` truncates silently at very low `tiling_depth` on very large regions.** Threshold: `region_area > 32768 * line_spacing^2 * 2^depth`. At 0.75 mm spacing that is a ~136 mm square at depth 0, ~271 mm at depth 2, and off any real bed by depth 4. The default depth is 10, so the default path has enormous headroom. On hitting the cap the remaining lattice cells are simply not placed, with no diagnostic — a guest has no log channel on this path.

6. **Three pre-existing workspace test failures, explicitly accepted for closure.** `cargo xtask test --summary --workspace` reports `VERDICT: FAIL` with 3 failures in `slicer-runtime --test contract`, all `native/wasm parity: "region[0] identity mismatch"` (`integrated_parity_support_surface_ironing`, `integrated_parity_traditional_support`, `integrated_parity_tree_support`). They are **not** caused by this packet: every file it touches is either a packet doc or inside `modules/community-modules/dragon-curve/`, which is not a workspace member and is absent from the Rust compile graph, and no `slicer-runtime` test references the module. The failures stem from the in-progress packet-226 work on this branch; the user confirmed them pre-existing, stated a fix sits at commit `30a7cef266b9f3a308528b3372c11649046667da`, and explicitly authorised closing this packet with them broken. Recorded rather than left implicit, so closure does not imply a green workspace suite.

7. **`TASK-338` does not exist in `docs/07_implementation_status.md`** (highest is `TASK-335`). Packet 228 owns creating rows TASK-336..339. This packet therefore records **`no docs/07 delta`**; the `task_ids` front-matter entry and `task-map.md` crosswalk point at a row that 228 will create.

8. **`[hints] estimated-ms-per-layer = 12` is unmeasured.** It is a scheduler hint, not a contract, but it was not derived from a profile run.

## Doc Impact Statement (Required)

- **`none`** - this packet changes no `docs/*.md`, IR schema, WIT contract, scheduler, claim, manifest, host-service, or SDK contract. Those surfaces belong to draft 225/226. The module `README.md` is a module artifact under `modules/community-modules/dragon-curve/`, not a `docs/` edit, and the manual slice-test command documented there is a module-level example, not a contract doc change.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
