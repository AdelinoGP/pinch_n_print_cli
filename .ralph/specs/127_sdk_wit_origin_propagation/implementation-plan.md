# Implementation Plan: 127_sdk_wit_origin_propagation

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Fold in the uncommitted marshal precondition

- Task IDs:
  - `TASK-252`
- Objective: Land the 11-file marshal precondition (per-call `infill_areas` accumulation + `OriginBucket` per-origin drain + SDK per-call `set_infill_areas` + macro per-call drain) as the foundation for the explicit-origin mechanism. These changes are already in the working tree (uncommitted from the diagnose session); this step verifies they're intact and tests pass before building on them.
- Precondition: working tree has the 11 modified/new files from the diagnose session (check `git diff --stat HEAD` shows the expected files).
- Postcondition: `cargo test -p slicer-wasm-host --test contract` and `cargo test -p slicer-runtime --test contract` pass; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Files allowed to read (with line-range hints when > 300 lines):
  - `git diff --stat HEAD` — verify the 11 files are present (run via bash, not read).
  - `crates/slicer-wasm-host/src/marshal/out.rs` — lines `277-460` — confirm `convert_perimeter_output` has the per-origin drain.
  - `crates/slicer-wasm-host/src/marshal/accumulators.rs` — lines `42-78` — confirm `infill_areas: Vec<Vec<ExPolygon>>` + parallel `infill_areas_origins`.
  - `crates/slicer-sdk/src/builders.rs` — lines `117-292` — confirm `infill_areas: Vec<Vec<ExPolygon>>` + `set_infill_areas` appends.
- Files allowed to edit (≤ 3): none — this step verifies existing uncommitted changes. If any file is missing, restore from the diagnose session (the handoff documents the expected changes).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-wasm-host/src/host.rs` — not edited in this step (the precondition doesn't touch `effective_perimeter_origin`).
  - Any WIT file — not edited in this step.
  - Any guest module — not edited in this step.
- Expected sub-agent dispatches:
  - "Run `git diff --stat HEAD 2>&1`; return FACT: list of modified files (≤ 20 entries)" — purpose: verify the 11 precondition files are present.
  - "Run `cargo test -p slicer-wasm-host --test contract 2>&1 | tail -3`; return FACT pass/fail" — purpose: marshal contract tests pass.
  - "Run `cargo test -p slicer-runtime --test contract 2>&1 | tail -3`; return FACT pass/fail" — purpose: runtime contract tests pass.
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3`; return FACT pass/fail" — purpose: clippy clean.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` — read directly (< 200 lines) — the `OriginBucket` all-or-none attribution rule.
- OrcaSlicer refs: none for this step.
- Verification:
  - `cargo test -p slicer-wasm-host --test contract 2>&1 | tail -3` — dispatch as FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3` — dispatch as FACT pass/fail.
- Exit condition: `git diff --stat HEAD` shows the 11 expected files; all 4 dispatches return FACT pass.

### Step 2: Add `set-current-origin` to the WIT + SDK `begin_region` + host `explicit_perimeter_origin`

- Task IDs:
  - `TASK-252`
- Objective: Add the `set-current-origin` WIT method, the SDK `begin_region` method + `current_origin` field, and the host `explicit_perimeter_origin` field + `set_current_origin` impl + additive `.or_else()` in `effective_perimeter_origin`. This is the core mechanism — after this step, the plumbing exists but no guest uses it yet.
- Precondition: Step 1 exit condition met (marshal precondition intact, tests pass).
- Postcondition: `cargo check --workspace --all-targets` passes (WIT change regenerates bindgen; everything compiles). `cargo test -p slicer-wasm-host --test contract` passes (existing tests unchanged — the additive `.or_else()` preserves the fallback path).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-schema/wit/deps/ir-types.wit` — lines `85-94` — the `perimeter-output-builder` resource.
  - `crates/slicer-sdk/src/builders.rs` — lines `117-292` — the `PerimeterOutputBuilder` struct + push methods.
  - `crates/slicer-wasm-host/src/host.rs` — lines `641-646, 811-812, 901-941, 2341-2420` — origin chain + `HostPerimeterOutputBuilder` impl.
  - `crates/slicer-wasm-host/src/marshal/accumulators.rs` — lines `42-78` — the `*_origins` Vecs the SDK appends to.
- Files allowed to edit (≤ 3):
  - `crates/slicer-schema/wit/deps/ir-types.wit` — add `set-current-origin: func(object-id: string, region-id: string) -> result<_, string>;` to the `perimeter-output-builder` resource (after line 92, before the closing `}`).
  - `crates/slicer-sdk/src/builders.rs` — add `current_origin: Option<OriginId>` field to `PerimeterOutputBuilder` (after line 135); add `begin_region(&mut self, object_id: &str, region_id: u64)` method; in each push method (`push_wall_loop`, `set_infill_areas`, `push_seam_candidate`, `push_reordered_wall_loop`), append `self.current_origin.clone()` to the corresponding origins Vec. Note: the SDK builder does NOT currently store per-item origins in the same way the host collector does — the SDK builder stores `wall_loops: Vec<WallLoop>` without parallel origins. You need to add `wall_loop_origins: Vec<Option<OriginId>>`, `seam_candidate_origins: Vec<Option<OriginId>>`, `rotated_wall_loop_origins: Vec<Option<OriginId>>` to the SDK builder, parallel to the existing `infill_areas` per-call accumulation. The `OriginId` type must be importable from `slicer-ir` or defined in the SDK.
  - `crates/slicer-wasm-host/src/host.rs` — add `explicit_perimeter_origin: Option<OriginId>` field (near line 646); initialize to `None` in builder (near line 812); implement `set_current_origin` WIT host trait method (sets the field); modify `effective_perimeter_origin()` (line 937-941) to prepend `self.explicit_perimeter_origin.clone()` as the highest-precedence fallback.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-macros/src/lib.rs` — not edited yet (drain forwarding is Step 3).
  - Any guest module — not edited yet (Step 4).
  - `crates/slicer-wasm-host/src/marshal/out.rs` — not edited (marshal unchanged).
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets 2>&1 | tail -3`; return FACT pass/fail" — purpose: WIT change compiles.
  - "Run `cargo test -p slicer-wasm-host --test contract 2>&1 | tail -3`; return FACT pass/fail" — purpose: existing tests pass (additive `.or_else()` preserves fallback).
  - "Find all callers of `effective_perimeter_origin` in `crates/slicer-wasm-host/src/host.rs`; return LOCATIONS (file:line, ≤ 20 entries)" — purpose: confirm the `.or_else()` prepend does not miss any origin-reading site.
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — delegate a SUMMARY of the `perimeter-output-builder` resource section.
  - `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` — read directly (< 200 lines).
- OrcaSlicer refs: none for this step.
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tail -3` — dispatch as FACT pass/fail.
  - `cargo test -p slicer-wasm-host --test contract 2>&1 | tail -3` — dispatch as FACT pass/fail.
- Exit condition: `cargo check` passes; existing contract tests pass; `effective_perimeter_origin` has 3-level fallback chain (explicit → perimeter → slice).

### Step 3: Modify the macro drain to forward per-item origins via `set-current-origin`

- Task IDs:
  - `TASK-252`
- Objective: Modify `__slicer_drain_perimeter` in `crates/slicer-macros/src/lib.rs` to call `wit.set_current_origin(object_id, region_id)` before each WIT push (`push_wall_loop`, `set_infill_areas`, `push_seam_candidate`, `push_reordered_wall_loop`), forwarding the SDK item's per-item origin. For items with `None` origin, skip the `set_current_origin` call (the host's fallback chain handles it).
- Precondition: Step 2 exit condition met (WIT method exists, SDK builder has per-item origins, host has `explicit_perimeter_origin`).
- Postcondition: `cargo check --workspace --all-targets` passes. The macro drain now forwards origins. No guest uses `begin_region` yet, so all SDK items have `None` origin — the drain skips `set_current_origin` and the host fallback chain is unchanged.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-macros/src/lib.rs` — lines `2384-2428` — `__slicer_drain_perimeter`.
  - `crates/slicer-sdk/src/builders.rs` — lines `258-292` — the SDK accessor methods (`wall_loops`, `infill_areas`, `seam_candidates`, `rotated_wall_loops`) and their new origin accessors.
- Files allowed to edit (≤ 3):
  - `crates/slicer-macros/src/lib.rs` — modify `__slicer_drain_perimeter` (lines 2384-2428) to iterate SDK items with their origins and call `wit.set_current_origin(...)` before each WIT push when origin is `Some`. The SDK builder needs new accessor methods: `wall_loop_origins()`, `seam_candidate_origins()`, `rotated_wall_loop_origins()` (parallel to the existing `infill_areas()` per-call accessor). Add these accessors to `crates/slicer-sdk/src/builders.rs` if not already present.
  - `crates/slicer-sdk/src/builders.rs` — add origin accessor methods if needed (the `infill_areas()` accessor already returns `&[Vec<ExPolygon>]`; add `infill_areas_origins() -> &[Option<OriginId>]`, `wall_loop_origins() -> &[Option<OriginId>]`, etc.).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-wasm-host/src/host.rs` — not edited in this step (host impl done in Step 2).
  - Any guest module — not edited yet (Step 4).
  - `crates/slicer-wasm-host/src/marshal/out.rs` — not edited.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets 2>&1 | tail -3`; return FACT pass/fail" — purpose: macro change compiles.
  - "Run `cargo test -p slicer-wasm-host --test contract 2>&1 | tail -3`; return FACT pass/fail" — purpose: existing tests still pass (no guest uses `begin_region` yet, so origins are all `None`, drain skips `set_current_origin`, fallback unchanged).
- Context cost: `M`
- Authoritative docs: none for this step.
- OrcaSlicer refs: none for this step.
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tail -3` — dispatch as FACT pass/fail.
  - `cargo test -p slicer-wasm-host --test contract 2>&1 | tail -3` — dispatch as FACT pass/fail.
- Exit condition: `cargo check` passes; existing contract tests pass; the drain function calls `wit.set_current_origin` for `Some` origins and skips for `None`.

### Step 4: Add `begin_region` calls to 4 guest modules

- Task IDs:
  - `TASK-252`
- Objective: Add `output.begin_region(region.object_id(), *region.region_id());` at the top of the `for region in regions` loop in 4 guest modules. This is the one-line-per-module change that makes the guest set the explicit origin before pushing for each region.
- Precondition: Step 3 exit condition met (drain forwards origins; plumbing complete).
- Postcondition: `cargo check --workspace --all-targets` passes. The 4 guest modules now call `begin_region` at the loop top. The code compiles but guests are not rebuilt yet (Step 5 does that).
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/classic-perimeters/src/lib.rs` — lines `193-210` — the `for region in regions` loop top.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — lines `199-216` — the `for region in regions` loop top.
  - `modules/core-modules/seam-placer/src/lib.rs` — lines `219-224` — the `for region in regions` loop top.
  - `modules/core-modules/fuzzy-skin/src/lib.rs` — lines `80-82` — the `for region in regions` loop top.
- Files allowed to edit (≤ 3): (4 files, but each is a one-line addition — split into two sub-steps if the ≤ 3 limit is strict)
  - `modules/core-modules/classic-perimeters/src/lib.rs` — add `output.begin_region(region.object_id(), *region.region_id());` at line 193 (before the `if region.polygons().is_empty()` skip).
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — add `output.begin_region(region.object_id(), *region.region_id());` at line 199 (before the `if region.polygons().is_empty()` skip).
  - `modules/core-modules/seam-placer/src/lib.rs` — add `output.begin_region(region.object_id(), *region.region_id());` at line 219 (before the `if wall_loops.is_empty()` skip).
  - `modules/core-modules/fuzzy-skin/src/lib.rs` — add `output.begin_region(region.object_id(), *region.region_id());` at line 80 (before the inner `for (wall_index, wall) in region.wall_loops()...` loop).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-macros/src/lib.rs` — not edited (drain done in Step 3).
  - `crates/slicer-wasm-host/src/host.rs` — not edited.
  - Any test file — not edited yet (Step 6).
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets 2>&1 | tail -3`; return FACT pass/fail" — purpose: all 4 module edits compile.
- Context cost: `S`
- Authoritative docs: none for this step.
- OrcaSlicer refs: none for this step.
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tail -3` — dispatch as FACT pass/fail.
- Exit condition: `cargo check` passes; all 4 modules have `begin_region` at the loop top.

### Step 5: Rebuild guest WASMs and run the gcode feedback loop

- Task IDs:
  - `TASK-252`
- Objective: Rebuild all guest WASMs (the WIT change in Step 2 invalidated every guest's bindgen output). Then copy the fresh classic-perimeters wasm into the repro dir. Then slice `cube_4color.3mf` and measure the per-tool sparse-infill metric (AC-1).
- Precondition: Step 4 exit condition met (all 4 modules have `begin_region`; code compiles).
- Postcondition: `cargo xtask build-guests --check` reports `EXIT=0` (clean). The repro dir has the fresh classic-perimeters wasm. The gcode metric shows T1 >= 1000 and T3 <= 1500.
- Files allowed to read (with line-range hints when > 300 lines):
  - None — this step is build + run, not read.
- Files allowed to edit (≤ 3): none — this step builds and runs.
  - If `cargo xtask build-guests --check` reports `STALE:`, run `cargo xtask build-guests` (rebuild), then `cp modules/core-modules/classic-perimeters/classic-perimeters.wasm tmp/repro/modules-no-arachne/classic-perimeters/`.
- Files explicitly out-of-bounds for this step:
  - Any source file — not edited in this step.
  - `tmp/repro/modules-no-arachne/` — only the classic-perimeters wasm is copied; do not recreate the directory (it's a symlink-filtered copy from the diagnose session).
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check 2>&1; echo EXIT=$?`; return FACT: EXIT=0 or STALE: lines" — purpose: guest freshness gate.
  - "Run `cargo xtask build-guests 2>&1 | tail -5`; return FACT pass/fail" — purpose: rebuild if stale.
  - "Run `cp modules/core-modules/classic-perimeters/classic-perimeters.wasm tmp/repro/modules-no-arachne/classic-perimeters/ 2>&1`; return FACT pass/fail" — purpose: refresh repro wasm.
  - "Run `cargo build --release --bin pnp_cli 2>&1 | tail -3`; return FACT pass/fail" — purpose: rebuild release binary.
  - "Run `./target/release/pnp_cli slice --model resources/cube_4color.3mf --no-default-module-paths --module-dir tmp/repro/modules-no-arachne --output tmp/repro/pnp_out.gcode 2>tmp/repro/run.log; echo EXIT=$?`; return FACT: EXIT=0 or error" — purpose: slice the cube.
  - "Run `awk 'BEGIN{layer=0;t=\"\";tool=\"\"} /^;LAYER_CHANGE/{layer++;t=\"\";tool=\"\"} /^;TYPE:/{t=$0} /^T[0-9]+$/{tool=$0} /^G1 /&&/E/{if(t==\";TYPE:Sparse infill\"){key=tool; if(key==\"\")key=\"(no tool)\"; counts[key]++}} END{for(k in counts) print k, counts[k]}' tmp/repro/pnp_out.gcode | sort`; return FACT: T0/T1/T2/T3/(no tool) counts" — purpose: AC-1 metric.
- Context cost: `M`
- Authoritative docs: none for this step.
- OrcaSlicer refs: none for this step.
- Verification:
  - `cargo xtask build-guests --check 2>&1; echo EXIT=$?` — dispatch as FACT: EXIT=0 or STALE.
  - The awk metric command above — dispatch as FACT: per-tool counts.
- Exit condition: `--check` returns EXIT=0 (or rebuild + recheck returns EXIT=0); gcode metric shows T1 >= 1000 and T3 <= 1500.

### Step 6: Add the new host-level contract test (AC-4)

- Task IDs:
  - `TASK-252`
- Objective: Add `set_current_origin_routes_to_correct_bucket_tdd.rs` to `crates/slicer-wasm-host/tests/contract/`. The test builds a `HostExecutionContext` with only `explicit_perimeter_origin` set (via the new `set_current_origin` WIT method), drives `set_infill_areas` and `push_wall_loop` through the trait impl, converts via `convert_perimeter_output`, and asserts the `PerimeterRegion` has the explicit origin's `object_id` and `region_id` (not empty-string, not the fallback).
- Precondition: Step 2 exit condition met (host has `explicit_perimeter_origin` + `set_current_origin` impl).
- Postcondition: `cargo test -p slicer-wasm-host --test contract -- set_current_origin_routes_to_correct_bucket` passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-wasm-host/tests/contract/effective_perimeter_origin_integration_tdd.rs` — full file (155 lines) — purpose: copy the test structure (builder setup, trait impl invocation, `convert_perimeter_output` assertion pattern).
  - `crates/slicer-wasm-host/tests/contract/perimeter_infill_per_origin_route_tdd.rs` — lines `1-60` — purpose: copy the square/expolygon fixture helpers.
  - `crates/slicer-wasm-host/tests/contract/main.rs` — full file (small) — purpose: find the module registration pattern.
- Files allowed to edit (≤ 3):
  - `crates/slicer-wasm-host/tests/contract/set_current_origin_routes_to_correct_bucket_tdd.rs` — NEW test file.
  - `crates/slicer-wasm-host/tests/contract/main.rs` — add `mod set_current_origin_routes_to_correct_bucket_tdd;`.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-wasm-host/src/host.rs` — not edited (host impl done in Step 2).
  - Any guest module — not edited.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-wasm-host --test contract -- set_current_origin_routes_to_correct_bucket 2>&1 | tail -3`; return FACT pass/fail; on failure SNIPPETS with assertion + ≤ 20 lines" — purpose: AC-4.
- Context cost: `S`
- Authoritative docs: none for this step.
- OrcaSlicer refs: none for this step.
- Verification:
  - `cargo test -p slicer-wasm-host --test contract -- set_current_origin_routes_to_correct_bucket 2>&1 | tail -3` — dispatch as FACT pass/fail.
- Exit condition: test passes; `PerimeterRegion.object_id` matches the explicit origin's UUID; `PerimeterRegion.region_id` matches the explicit origin's region ID.

### Step 7: Add the new gcode-level parity test (AC-3)

- Task IDs:
  - `TASK-252`
- Objective: Add `cube_4color_sparse_infill_per_painted_region_tdd.rs` to `crates/slicer-runtime/tests/executor/`. The test slices `cube_4color.3mf` through the executor test path and asserts: (a) all four tool indices T0-T3 each have at least one `;TYPE:Sparse infill` segment with a `G1 ... E` extrusion move, (b) T1 count >= 1000, (c) T3 count <= 1500. Use inequality thresholds (not exact counts) because the gcode is NOT byte-stable (Voronoi RNG drift — see `cube_4color_gcode_output_tdd.rs:588-600`).
- Precondition: Step 5 exit condition met (gcode metric confirmed T1 >= 1000, T3 <= 1500 via the feedback loop).
- Postcondition: `cargo test -p slicer-runtime --test executor -- cube_4color_sparse_infill_per_painted_region` passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` — lines `430-605` — purpose: copy the `slice_fixture_file` + `parse_tool_index_lines` + gcode parsing patterns. Read the section that parses `;TYPE:` and `T<n>` lines.
  - `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` — lines `38-48` — purpose: copy the `cube_4color_path()` + `load_cube_4color()` fixture helpers.
  - `crates/slicer-runtime/tests/executor/main.rs` — full file (small) — purpose: find the module registration pattern.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/cube_4color_sparse_infill_per_painted_region_tdd.rs` — NEW test file.
  - `crates/slicer-runtime/tests/executor/main.rs` — add `mod cube_4color_sparse_infill_per_painted_region_tdd;`.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-runtime/src/layer_executor.rs` — not edited (spatial fallback stays).
  - `crates/slicer-runtime/src/region_partition.rs` — not edited.
  - Any guest module — not edited.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor -- cube_4color_sparse_infill_per_painted_region 2>&1 | tail -3`; return FACT pass/fail; on failure SNIPPETS with assertion + ≤ 20 lines" — purpose: AC-3.
  - "Run `cargo test -p slicer-runtime --test executor -- cube_4color_first_layer_perimeter_colour_matches_bottom_face 2>&1 | tail -3`; return FACT pass/fail" — purpose: AC-2 (no regression).
- Context cost: `S`
- Authoritative docs: none for this step.
- OrcaSlicer refs: none for this step.
- Verification:
  - `cargo test -p slicer-runtime --test executor -- cube_4color_sparse_infill_per_painted_region 2>&1 | tail -3` — dispatch as FACT pass/fail.
  - `cargo test -p slicer-runtime --test executor -- cube_4color_first_layer_perimeter_colour_matches_bottom_face 2>&1 | tail -3` — dispatch as FACT pass/fail.
- Exit condition: new test passes (all 4 tools in Sparse infill, T1 >= 1000, T3 <= 1500); existing wall-colour test passes.

### Step 8: Update docs (docs/07, CONTEXT.md, ADR-0022)

- Task IDs:
  - `TASK-252`
- Objective: Add TASK-252 entry to `docs/07_implementation_status.md`, add the "Per-region output origin" term to `CONTEXT.md`, and create `docs/adr/0022-explicit-per-region-origin-for-perimeter-output-builders.md` documenting the Shape 2 vs Shape 1 trade-off.
- Precondition: Steps 1-7 complete; all ACs pass.
- Postcondition: The three doc-impact verification greps (from `packet.spec.md` §Doc Impact Statement) return hits.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/07_implementation_status.md` — delegate a SUMMARY for the section where TASK-250 (packet 126) is listed, to find the insertion point for TASK-252. Do not load the full backlog.
  - `CONTEXT.md` — full file (150 lines) — purpose: find the insertion point for the new term (after "Marshalling boundary").
  - `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` — read directly (< 200 lines) — purpose: ADR format reference + cross-reference for ADR-0022.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` — add TASK-252 entry (cross-reference packet 126, packet 95).
  - `CONTEXT.md` — add "Per-region output origin" term after "Marshalling boundary".
  - `docs/adr/0022-explicit-per-region-origin-for-perimeter-output-builders.md` — NEW ADR.
- Files explicitly out-of-bounds for this step:
  - Any source file — not edited in this step.
  - Any test file — not edited.
- Expected sub-agent dispatches:
  - "Run `rg -q 'TASK-252' docs/07_implementation_status.md`; return FACT: hit or no-hit" — purpose: verify doc grep.
  - "Run `rg -q 'Per-region output origin' CONTEXT.md`; return FACT: hit or no-hit" — purpose: verify doc grep.
  - "Run `rg -q 'ADR-0022' docs/adr/0022-explicit-per-region-origin-for-perimeter-output-builders.md`; return FACT: hit or no-hit" — purpose: verify ADR exists.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` — read directly for ADR format.
- OrcaSlicer refs: none for this step.
- Verification:
  - `rg -q 'TASK-252' docs/07_implementation_status.md` — dispatch as FACT: hit.
  - `rg -q 'Per-region output origin' CONTEXT.md` — dispatch as FACT: hit.
  - `rg -q 'ADR-0022' docs/adr/0022-explicit-per-region-origin-for-perimeter-output-builders.md` — dispatch as FACT: hit.
- Exit condition: all 3 greps return hit.

### Step 9: Final clippy gate + acceptance ceremony

- Task IDs:
  - `TASK-252`
- Objective: Run the final clippy gate (AC-6) and re-dispatch every pipe-suffixed AC verification command to confirm all pass.
- Precondition: Steps 1-8 complete.
- Postcondition: All ACs green; packet ready for `status: implemented`.
- Files allowed to read: none — this step is verification only.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds for this step: all source files.
- Expected sub-agent dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3`; return FACT pass/fail" — purpose: AC-6.
  - "Run `cargo test -p slicer-wasm-host --test contract 2>&1 | tail -3`; return FACT pass/fail" — purpose: AC-4, AC-5, AC-N1.
  - "Run `cargo test -p slicer-runtime --test executor -- cube_4color 2>&1 | tail -3`; return FACT pass/fail" — purpose: AC-2, AC-3.
  - "Run `cargo test -p slicer-runtime --test contract 2>&1 | tail -3`; return FACT pass/fail" — purpose: marshal precondition intact.
  - "Run `cargo test -p slicer-runtime --test integration 2>&1 | tail -3`; return FACT pass/fail" — purpose: integration tests (gap_fill shape).
  - "Run `cargo xtask build-guests --check 2>&1; echo EXIT=$?`; return FACT: EXIT=0" — purpose: guest freshness.
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all dispatches above.
- Exit condition: all dispatches return FACT pass / EXIT=0.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Verify precondition (dispatches only) |
| Step 2 | M | WIT + SDK + host (3 primary files, range-reads) |
| Step 3 | M | Macro drain (range-read lib.rs:2384-2428) |
| Step 4 | S | 4 one-line module edits |
| Step 5 | M | Guest rebuild + gcode feedback loop (dispatches) |
| Step 6 | S | New host test (copy fixture pattern) |
| Step 7 | S | New gcode test (copy parsing pattern) |
| Step 8 | S | Doc updates (3 files, small) |
| Step 9 | S | Final gate (dispatches only) |

Aggregate: M (sum of 4×S + 3×M = 7S+3M, well under the L threshold). No single step is L.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-252 (via worker dispatch — never edited by loading the full backlog into the implementer's context).
- `CONTEXT.md` updated with "Per-region output origin" term.
- `docs/adr/0022-explicit-per-region-origin-for-perimeter-output-builders.md` created.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green (`cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p slicer-runtime --test executor -- cube_4color`).
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.