# Implementation Plan: 96_paint-segmentation-phase5-width-limit

## Execution Rules

- One step at a time.
- All `cargo test` runs prefixed with `mkdir -p target &&` so the tee target exists.
- Test output teed to `target/test-output.log`.

## Steps

### Step 0: Capture pre-packet baselines (wedge + cube default-config SHAs)

- Task IDs: `TASK-246`
- Objective: AC-8 prerequisite — both must match post-packet SHAs via machine equality check.
- Precondition: P95 closed.
- Postcondition: 2 SHAs persisted to disk at `target/p96-baseline-wedge.sha` and `target/p96-baseline-cube.sha` (single-line hex, no filename).
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-baseline-wedge.gcode && sha256sum /tmp/p96-baseline-wedge.gcode | cut -d' ' -f1 > target/p96-baseline-wedge.sha && cat target/p96-baseline-wedge.sha`; return FACT (hex line)".
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-baseline-cube.gcode && sha256sum /tmp/p96-baseline-cube.gcode | cut -d' ' -f1 > target/p96-baseline-cube.sha && cat target/p96-baseline-cube.sha`; return FACT (hex line)".
- Context cost: `S`.
- Exit condition: both `.sha` files exist with 64-char hex content; the AC-8 machine equality command can now resolve.

### Step 1: Confirm pre-packet anchors (schema syntax, integration point, polygon-ops signatures, RegionKey)

- Task IDs: `TASK-246`
- Objective: confirm the anchors declared in `design.md` are still accurate and gather syntactical details needed by Step 2 (kernel) and Step 4a (integration). The schema landing site is ALREADY DECIDED (`modules/core-modules/mesh-segmentation/mesh-segmentation.toml`); this step verifies the syntax to mirror, not the location.
- Precondition: Step 0 complete.
- Postcondition: implementer notes record (a) the exact `[config.schema.*]` syntax from one existing block in mesh-segmentation.toml; (b) the exact lines flanking the variant-composition end at `mod.rs:802` and the final return at `mod.rs:999`; (c) the exact `OffsetJoinType` variant + arc-tolerance value that existing P95 paint-segmentation callers pass to `polygon_ops::offset` (the inward-offset primitive — invoked with NEGATIVE `delta_mm`) and that `difference_ex` takes slice refs at `polygon_ops.rs:266`; (d) the canonical paint `RegionKey` used to call `config_for`.
- Expected dispatches:
  - "Open `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` and return one existing `[config.schema.*]` block verbatim; SNIPPETS ≤ 15 lines".
  - "Open `crates/slicer-core/src/algos/paint_segmentation/mod.rs` and return SNIPPETS for lines 798–815 (end of variant-composition block) AND lines 992–1002 (final return); ≤ 30 lines total".
  - "Return the EXACT pub fn signature of `offset` at `crates/slicer-core/src/polygon_ops.rs:195` (the only inward-offset primitive; invoked with NEGATIVE `delta_mm`) and `difference_ex` at `:266`; FACT. Plus: which `OffsetJoinType` variant and arc-tolerance value do existing P95 paint-segmentation callers in `paint_segmentation/` use? Return as one FACT line.".
  - "In `paint_segmentation/mod.rs::execute_paint_segmentation`, what `RegionKey` value is passed to `region_map.config_for(...)` for paint-related config reads (if any). If none yet, return the canonical constant or pattern used by other paint-segmentation reads in the codebase; FACT".
- Context cost: `S`.
- Verification: all four dispatches return.
- Exit condition: implementer notes contain the four anchors above and Step 2 can begin.

### Step 2: Implement `cut_segmented_layers` kernel + 6 unit tests (3 positive + 2 negative + 1 short-circuit)

- Task IDs: `TASK-246`
- Objective: AC-1, AC-N1, AC-N2 (AC-N3 belongs to driver-level Step 4a).
- Precondition: Step 1 complete (polygon-ops signatures known).
- Postcondition: kernel exists; 6 unit tests pass; the kernel signature MATCHES `design.md` §"Code Change Surface" (no `beam` parameter, `BTreeMap<ChainKey, Vec<ExPolygon>>` per layer).
- Files allowed to read:
  - `crates/slicer-core/src/polygon_ops.rs` — exact `offset_*` + `difference_ex` signatures.
  - `crates/slicer-core/src/algos/paint_segmentation/compose_variants.rs` lines 40–70 (for `ChainKey` re-export).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` (NEW).
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (add `pub mod width_limit;` declaration only — do NOT add the integration call yet; that's Step 4a).
- Files out-of-bounds: any other sub-module; the integration site in `mod.rs`'s driver body.
- Expected dispatches:
  - "Run `mkdir -p target && cargo test -p slicer-core --features host-algos paint_segmentation::width_limit 2>&1 | tee target/test-output.log`; FACT (`test result: ok. [6-9] passed; 0 failed`)". (`--features host-algos` required: slicer-core has `default = []` and the `algos` module is feature-gated.)
- Context cost: `M`.
- Authoritative docs: roadmap §"P4", spec §3 Phase 5. OrcaSlicer parity FACT for depth-selection logic at `MultiMaterialSegmentation.cpp:1294` already encoded in `design.md` kernel sketch — no further OrcaSlicer dispatch needed for this step.
- Verification: 6 tests pass; the kernel does NOT take a `beam` parameter.
- Exit condition: AC-1, AC-N1, AC-N2 satisfied.

### Step 3: Add config-schema entries for the three keys in `mesh-segmentation.toml`

- Task IDs: `TASK-246`
- Objective: AC-3.
- Precondition: Step 2 green; Step 1 has captured the schema syntax to mirror.
- Postcondition: three `[config.schema.mmu_segmented_region_*]` sections exist in `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` with full TOML field structure (`type`, `default`, `units` for f32 keys, `minimum`, `description`).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/mesh-segmentation/mesh-segmentation.toml`.
- Files out-of-bounds: any other.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; FACT".
  - "Run the AC-3 schema TOML structural assertion from `packet.spec.md` AC-3 (the multi-block grep with `default = 0.0`, `units = \"mm\"`, `minimum = 0.0`, `type = \"bool\"`, `default = false`); FACT".
- Context cost: `S`.
- Verification: workspace check clean; AC-3 structural assertion PASS.
- Exit condition: AC-3 satisfied.

### Step 4a: Integrate `cut_segmented_layers` into `execute_paint_segmentation`

- Task IDs: `TASK-246`
- Objective: AC-2, AC-4, AC-N3.
- Precondition: Step 3 green; schema entries reachable via `config_for`.
- Postcondition: driver reads the three MMU config keys via `RegionMapIR::config_for`, guards on `!interlocking_beam`, calls `cut_segmented_layers(...)` between `mod.rs:802` (end of variant block) and `mod.rs:999` (return). One driver-level unit test (`interlocking_beam_true_skips_phase5_driver`) asserts the kernel is NOT invoked when beam = true.
- Files allowed to read:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` lines 393–999 (driver body; range-read in halves to stay within context budget).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (integration block + one new `#[cfg(test)]` test).
- Files out-of-bounds: `width_limit.rs` (no changes; already complete).
- Expected dispatches:
  - "Run `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; FACT".
  - "Run the AC-2 verification grep from `packet.spec.md`; FACT".
  - "Run the AC-4 verification grep from `packet.spec.md` (all three keys at integration site); FACT".
  - "Run `mkdir -p target && cargo test -p slicer-core --features host-algos interlocking_beam_true_skips_phase5_driver 2>&1 | tee target/test-output.log`; FACT — proves AC-N3 driver-level skip (filter intentionally has no `paint_segmentation::` prefix — substring match works whether the test sits at the file root or inside `mod tests`)".
  - "Helper-vs-driver wire-in (P95 W6/W8 trap guard): in `crates/slicer-core/src/algos/paint_segmentation/mod.rs` does `pub fn execute_paint_segmentation` invoke `cut_segmented_layers` on the production path (i.e. NOT only inside `#[cfg(test)]`)? LOCATIONS (file:line, function context)".
- Context cost: `S`.
- Verification: all dispatches PASS; the helper-vs-driver wire-in returns a production-path location.
- Exit condition: AC-2, AC-4, AC-N3 satisfied.

### Step 4b: Add `bisector_edge_skip_mask` field to `SlicedRegion` + populate it in the driver

- Task IDs: `TASK-246-BISECTOR`
- Objective: prepare AC-22b (IR + tagging only; consumer change comes in Step 4c).
- Precondition: Step 4a green.
- Postcondition:
  - `SlicedRegion` in `crates/slicer-ir/src/slice_ir.rs:1273` has an additive `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` field with `#[serde(default)]`.
  - The driver `execute_paint_segmentation` calls a new helper `populate_bisector_edge_skip_masks(&mut working)` between the variant-composition block end (`mod.rs:802`) and the Phase 5 call.
  - The helper lives in a new submodule `crates/slicer-core/src/algos/paint_segmentation/bisector_ownership.rs`.
  - Guest WASMs are rebuilt (`cargo xtask build-guests`); `--check` returns clean.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` lines 1265–1310 (`SlicedRegion` shape + derives).
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs` (additive field + Default impl preservation).
  - `crates/slicer-core/src/algos/paint_segmentation/bisector_ownership.rs` (NEW).
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (tagging-helper invocation + `pub mod bisector_ownership;`).
- Files out-of-bounds: `classic-perimeters/`.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; FACT".
  - "Run `cargo xtask build-guests --check`; if STALE: rebuild without --check, re-run --check; FACT".
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log`; FACT (must still show 11 passed; 0 failed — the additive `None`-default field MUST NOT regress; baseline file count verified during P96 review)".
- Context cost: `M`.
- Verification: workspace check clean; guests rebuilt; AC-10 11/11 STILL GREEN.
- Exit condition: IR additive change shipped; tagging stage live; no AC-10 regression.

### Step 4c: Consume `bisector_edge_skip_mask` in `classic-perimeters` outer-wall emission

- Task IDs: `TASK-246-BISECTOR`
- Objective: AC-22b.
- Precondition: Step 4b green; mask is populated by the driver.
- Postcondition:
  - `modules/core-modules/classic-perimeters/src/lib.rs::run_perimeters` (line 85) reads `region.bisector_edge_skip_mask`; outer-wall emission (loop index `i == 0` at lines 111–118; edge iteration at line 153) skips edges flagged `true`.
  - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs:337` has its `#[ignore]` attribute removed.
  - The test `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` drives GREEN.
- Files allowed to read:
  - `modules/core-modules/classic-perimeters/src/lib.rs` lines 85–200 (range-read).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/classic-perimeters/src/lib.rs`.
  - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` (delete `#[ignore]` only).
- Files out-of-bounds: `slice_ir.rs`, paint-segmentation submodules.
- Expected dispatches:
  - "Run `cargo xtask build-guests` (rebuild after classic-perimeters edit); then `--check`; FACT".
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one 2>&1 | tee target/test-output.log`; FACT (1 passed; 0 failed; 0 ignored)".
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log`; FACT (11 passed; 0 failed — no regression from consumer change)".
- Context cost: `S`.
- Verification: AC-22b test GREEN; AC-10 still 11/11.
- Exit condition: AC-22b satisfied.

### Step 5: Add 3 integration tests (band width, interlocking alternation, beam-flag skip)

- Task IDs: `TASK-246`
- Objective: AC-5, AC-6, AC-7.
- Precondition: Steps 4a + 4b + 4c all green.
- Postcondition: 3 integration tests pass.
- Files allowed to read:
  - `crates/slicer-runtime/tests/common/` (fixture helpers).
- Files allowed to edit (≤ 3 per commit; multi-commit):
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_width_limit_bands_tdd.rs` (NEW; AC-5).
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_interlocking_alternates_tdd.rs` (NEW; AC-6).
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_interlocking_beam_skips_phase5_tdd.rs` (NEW; AC-7 — asserts byte-identicality vs `width=0,depth=0` baseline; NOT "constant bands").
  - Optionally `resources/cube_4color_tall.3mf` if needed (≤ 100 KB).
- Files out-of-bounds: kernel; driver; classic-perimeters.
- Expected dispatches:
  - "Determine whether `resources/cube_4color.3mf` produces ≥ 30 mm tall geometry at default layer height; return FACT (height in mm or layer count)" — purpose: decide if `cube_4color_tall.3mf` authoring is required.
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 | tee target/test-output.log`; FACT pass/fail".
- Context cost: `M`.
- Verification: 3 integration tests pass.
- Exit condition: AC-5, AC-6, AC-7 satisfied.

### Step 6: Regression checks — AC-8 (wedge + cube default-config byte-identical via SHA equality) + AC-10 (21 cube tests still GREEN: 11 cube_4color + 10 cube_fuzzy_painted)

- Task IDs: `TASK-246`, `TASK-246-BISECTOR`
- Objective: AC-8, AC-10.
- Precondition: Step 5 green; `target/p96-baseline-{wedge,cube}.sha` from Step 0 still on disk.
- Postcondition: AC-8 machine equality (`[ "$post_sha" = "$baseline_sha" ]`) returns 0 for both wedge and cube; 21/21 cube tests still GREEN (11 cube_4color_paint_tdd + 10 cube_fuzzy_painted_tdd; counts verified during P96 review).
- Expected dispatches:
  - "Run the AC-8 wedge equality command from `packet.spec.md` AC-8 verification; FACT pass/fail".
  - "Run the AC-8 cube equality command from `packet.spec.md` AC-8 verification; FACT pass/fail".
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log`; FACT (must show 11 passed; 0 failed)".
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log`; FACT (must show 10 passed; 0 failed)".
- Context cost: `S`.
- Verification: both equality FACTs PASS + 21/21 GREEN (11+10).
- Exit condition: AC-8, AC-10 satisfied.

### Step 7: Visual report capture (AC-9)

- Task IDs: `TASK-246`
- Objective: AC-9.
- Precondition: Step 6 green.
- Postcondition: HTML report file exists; closure log notes layer ID + visual confirmation.
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube-banded.gcode --report /tmp/p96-cube-banded-report.html && test -f /tmp/p96-cube-banded-report.html`; FACT pass/fail".
- Context cost: `S`.
- Verification: file exists.
- Exit condition: AC-9 satisfied (closure-log visual confirmation is a human step).

### Step 8: Guest WASM `--check`

- Task IDs: `TASK-246`
- Objective: AC-11.
- Expected dispatches:
  - "Run `cargo xtask build-guests --check`; FACT pass/fail".
- Context cost: `S`.
- Exit condition: AC-11 satisfied.

### Step 9: Final acceptance ceremony (full per-AC re-dispatch)

- Task IDs: `TASK-246`, `TASK-246-BISECTOR`
- Objective: final gate; every AC re-verified end-to-end.
- Expected dispatches (each is FACT pass/fail; reject any reply pasting full build logs):
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; FACT".
  - "Run `mkdir -p target && cargo test -p slicer-core --features host-algos paint_segmentation 2>&1 | tee target/test-output.log`; FACT" — covers AC-1, AC-N1, AC-N2, AC-N3 kernel + driver suite.
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 | tee target/test-output.log`; FACT" — covers AC-5, AC-6, AC-7.
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log`; FACT (11/11 GREEN)" — covers AC-10 cube_4color.
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log`; FACT (10/10 GREEN)" — covers AC-10 cube_fuzzy_painted.
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one 2>&1 | tee target/test-output.log`; FACT (1 passed; 0 failed; 0 ignored)" — covers AC-22b.
  - "Run the AC-2 verification grep from `packet.spec.md`; FACT" — re-verify wire-in.
  - "Run the AC-3 schema TOML structural assertion from `packet.spec.md`; FACT".
  - "Run the AC-4 verification grep from `packet.spec.md`; FACT".
  - "Run the AC-8 wedge + cube equality commands from `packet.spec.md`; FACT both pass".
  - "Run the AC-9 visual report command from `packet.spec.md`; FACT (file existence)".
  - "Run `cargo xtask build-guests --check`; FACT (no STALE:)" — covers AC-11.
  - "Run the Doc Impact Statement greps from `packet.spec.md` Doc Impact Statement section; FACT each".
  - "Run the workspace acceptance ceremony test gate `cargo test --workspace 2>&1 | tee target/test-output.log`; FACT — this is the SINGLE end-of-packet workspace-test pass per `CLAUDE.md` Test Discipline".
- Context cost: `S` (dispatches; absorbed FACTs only).
- Verification: all PASS.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baselines (now persist SHAs to `target/p96-baseline-{wedge,cube}.sha`). |
| Step 1 | S | Confirm 4 pre-packet anchors. |
| Step 2 | M | Kernel + 6 tests. |
| Step 3 | S | Config-schema entries (mesh-segmentation.toml). |
| Step 4a | S | Phase 5 driver integration + AC-N3 driver test + wire-in dispatch. |
| Step 4b | M | `SlicedRegion` field + tagging stage + new `bisector_ownership.rs` + guest rebuild. |
| Step 4c | S | classic-perimeters consumer + unignore deferred test. |
| Step 5 | M | 3 integration tests. |
| Step 6 | S | Regression (SHA-equality + 21/21 cube tests: 11 + 10). |
| Step 7 | S | Visual report. |
| Step 8 | S | Guest check. |
| Step 9 | S | Acceptance ceremony (dispatches; absorbed FACTs only). |

Aggregate: M.

## Packet Completion Gate

- All 12 steps complete (0, 1, 2, 3, 4a, 4b, 4c, 5, 6, 7, 8, 9).
- AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-7, AC-8, AC-9, AC-10, AC-11, AC-22b verified.
- AC-N1, AC-N2, AC-N3 verified.
- Closure log records:
  - pre/post wedge SHAs MATCH (Step 0 baselines vs. Step 6 post-packet hashes; machine-checked).
  - pre/post cube SHAs MATCH (Step 0 baselines vs. Step 6 post-packet hashes; machine-checked).
  - 11/11 + 10/10 cube test counts (cube_4color_paint_tdd + cube_fuzzy_painted_tdd; counts verified during P96 review).
  - AC-22b test count line: `test result: ok. 1 passed; 0 failed; 0 ignored`.
  - visual-banding confirmation (Step 7 HTML report path + screenshot reference if available).
  - schema landing site URL: `modules/core-modules/mesh-segmentation/mesh-segmentation.toml`.
  - chosen chain-key shape FACT (Step 1): `ChainKey = Vec<(String, PaintValue)>` re-exported from `compose_variants.rs:45`.
- `docs/07_implementation_status.md` updated for `TASK-246` AND `TASK-246-BISECTOR` (delegate).
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4" flipped to `implemented`; `D-95-AC22-BISECTOR-DEDUP` marked `resolved` (delegate).
- `docs/02_ir_schemas.md` §"SlicedRegion" documents the new `bisector_edge_skip_mask` field (delegate).
- `docs/DEVIATION_LOG.md` registers `D-96-DEFAULT-ZERO` and `D-96-BEAM-FLAG-SKIPS` (delegate).
- All Doc Impact Statement greps PASS.
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC (positive + negative + AC-22b); PASS.
- Confirm 21 cube tests still GREEN (11 + 10).
- Confirm `cargo xtask build-guests --check` clean (no STALE:).
- Confirm AC-8 wedge + cube SHA-equality FACTs PASS.
- Run `cargo test --workspace` exactly ONCE at acceptance ceremony (per `CLAUDE.md` Test Discipline); FACT pass/fail.
- Peak context usage under 70%.
