# Implementation Plan: 60_configurable-slicing-precision

## Execution Rules

- One atomic step at a time.
- Each step maps back to `TASK-201` (DEV-009 umbrella).
- TDD first where natural (helpers, format, manifest, slice round-trip). For broader emit wiring, write the per-role dispatch test first and let it drive the integration.
- Honor the context-discipline preamble: delegate every cargo run, never load `OrcaSlicerDocumented/`, range-read large files.

## Steps

### Step 1: Declare 7 config keys

- Task IDs:
  - `TASK-201`
- Objective: Add 7 entries to `declare_resolved_config!` so `ResolvedConfig::default()` exposes the new keys with OrcaSlicer-aligned defaults.
- Precondition: HEAD clean; `cargo check --workspace` green.
- Postcondition: `cargo test -p slicer-ir --test resolved_config_defaults_tdd -- new_precision_keys_have_orca_defaults` PASSES.
- Files allowed to read (with line-range hints):
  - `crates/slicer-ir/src/resolved_config.rs` — lines `[1-67]` and `[300-410]` only
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs` *(new or existing)*
- Files explicitly out-of-bounds for this step:
  - The middle of `resolved_config.rs` (`[68-299]`)
  - All other crates
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace`; return FACT pass/fail." — heartbeat after edit.
  - "Run `cargo test -p slicer-ir --test resolved_config_defaults_tdd -- new_precision_keys_have_orca_defaults`; return FACT pass/fail." — AC-1 verification.
- Context cost: `S`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — confirm additive-field rule (delegate SUMMARY if > 300 lines).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/libslic3r.h` — defaults confirmation. Delegate FACT: "Confirm RESOLUTION = 0.0125, SPARSE_INFILL_RESOLUTION = 0.04, SUPPORT_RESOLUTION = 0.0375."
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_defaults_tdd -- new_precision_keys_have_orca_defaults --nocapture` — dispatch as FACT pass/fail.
- Exit condition: AC-1 green AND `cargo check --workspace` green.

### Step 2: Douglas-Peucker + min-segment helpers in slicer-helpers

- Task IDs:
  - `TASK-201`
- Objective: Add `simplify_polyline_mm` (iterative D-P, squared-distance, preserves endpoints) and `drop_short_segments_mm` (preserves first AND last point) to `slicer-helpers/src/decimate.rs`. Add 4 unit tests.
- Precondition: Step 1 complete.
- Postcondition: `cargo test -p slicer-helpers --lib -- decimate::tests` PASSES (4 tests: dp_collapses_collinear_to_endpoints, dp_zero_tolerance_is_identity, dp_non_positive_tolerance_is_identity, min_segment_drops_micro_and_preserves_endpoints).
- Files allowed to read:
  - `crates/slicer-helpers/src/decimate.rs` — full file (likely small)
  - `crates/slicer-helpers/src/lib.rs` — full file
- Files allowed to edit (≤ 3):
  - `crates/slicer-helpers/src/decimate.rs`
  - `crates/slicer-helpers/src/lib.rs` *(only if re-export needed)*
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — port via delegated SUMMARY only
  - All other crates
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp:179` — `_douglas_peucker` control flow, distance metric, endpoint handling. SUMMARY ≤ 200 words. No code." — algorithm port.
  - "Run `cargo test -p slicer-helpers --lib -- decimate::tests`; return FACT pass/fail." — AC-2, AC-3, AC-4, NEG-1.
- Context cost: `M`.
- Authoritative docs:
  - `docs/13_slicer_helpers_crate.md` — load fully (small).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp:179` — delegate SUMMARY.
- Verification:
  - `cargo test -p slicer-helpers --lib -- decimate::tests --nocapture` — dispatch FACT pass/fail.
- Exit condition: 4 unit tests green AND `cargo check --workspace` green.

### Step 3: Apply slice_closing_radius at mesh slice

- Task IDs:
  - `TASK-201`
- Objective: Insert a `+r / -r` Clipper2 offset round-trip per layer in `triangle_mesh_slicer.rs` after `simplify_polygon_points`. Gated on `slice_closing_radius > 0.0`.
- Precondition: Step 1 complete. (Independent of Step 2.)
- Postcondition: `cargo test -p slicer-core --test triangle_mesh_slicer_tdd -- slice_closing_radius_fuses_gap_within_two_r slice_closing_radius_zero_is_noop` PASSES.
- Files allowed to read:
  - `crates/slicer-core/src/triangle_mesh_slicer.rs` — lines `[330-370]`
  - `crates/slicer-core/src/polygon_ops.rs` — lines `[180-220]` (for `offset` import)
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/triangle_mesh_slicer.rs`
  - `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs` *(new or existing)*
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**`
  - Any other slicer-core file
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp:192,1393` — how `slice_closing_radius` is applied (inflate→deflate order, join type). SUMMARY ≤ 150 words." — port semantics.
  - "Run `cargo test -p slicer-core --test triangle_mesh_slicer_tdd -- slice_closing_radius_fuses_gap_within_two_r slice_closing_radius_zero_is_noop`; return FACT pass/fail." — AC-7, NEG-3.
- Context cost: `S`.
- Authoritative docs:
  - `docs/08_coordinate_system.md` — confirm `mm * 10_000.0` for offset (delta is already in mm in `polygon_ops::offset`, so the closing-radius value is passed in mm).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp:192,1393`
- Verification:
  - `cargo test -p slicer-core --test triangle_mesh_slicer_tdd -- slice_closing_radius_fuses_gap_within_two_r slice_closing_radius_zero_is_noop --nocapture`
- Exit condition: AC-7 and NEG-3 green.

### Step 4: arc_tolerance parameter on slicer_core::polygon_ops::offset

- Task IDs:
  - `TASK-201`
- Objective: Change signature of `slicer_core::polygon_ops::offset` to add `arc_tolerance_mm: f32` as 4th positional param; multiply by `10_000.0` and cast to `f64` for the Clipper2 call. Update every direct caller to pass `0.0` (zero behavioral change at this step).
- Precondition: Step 3 complete (so the closing-radius round-trip already calls `offset(_, _, _, 0.0)`).
- Postcondition: `cargo check --workspace` green; `cargo test -p slicer-core --test polygon_ops_tdd -- offset_arc_tolerance_reduces_vertex_count` PASSES; all existing `polygon_ops` tests still PASS.
- Files allowed to read:
  - `crates/slicer-core/src/polygon_ops.rs` — lines `[180-220]`
  - Each direct caller — only the import line + the call site (range-read ±5 lines)
- Files allowed to edit (≤ 3 in the main flow; the caller updates are mechanical pass-throughs and treated as one logical edit-set):
  - `crates/slicer-core/src/polygon_ops.rs`
  - `crates/slicer-core/tests/polygon_ops_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - The body of `wit_host.rs` outside `:2412` and `:3193`
  - All module sources (those get the real value at Step 5, not `0.0`)
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace`; return FACT pass/fail. On fail, SNIPPETS of first 5 errors so we can see every direct caller." — this drives the caller-update pass.
  - "Run `cargo test -p slicer-core --test polygon_ops_tdd -- offset_arc_tolerance_reduces_vertex_count`; return FACT pass/fail." — AC-6.
  - "Audit any indirect `slicer_core::polygon_ops::offset` callers in `crates/slicer-host/src/wit_host.rs` outside lines `:2412` and `:3193`. Return LOCATIONS only." — catch any missed sites.
- Context cost: `M`.
- Authoritative docs:
  - `docs/08_coordinate_system.md` — mm→units conversion.
- OrcaSlicer refs:
  - None for this step (signature is internal).
- Verification:
  - `cargo check --workspace` — FACT pass/fail.
  - `cargo test -p slicer-core --test polygon_ops_tdd -- offset_arc_tolerance_reduces_vertex_count --nocapture` — AC-6.
- Exit condition: Workspace compiles; AC-6 green; pass-through call sites at `slicer-host/src/wit_host.rs:2412, :3193`, `slicer-host/src/layer_slice.rs`, `slicer-sdk/src/host.rs:253`, `slicer-core/benches/polygon_ops.rs`, both perimeter modules' `src/lib.rs` all pass `0.0` (perimeter modules upgrade to real value at Step 5).

### Step 5: Per-module manifest entry + read-and-pass-through for perimeter modules

- Task IDs:
  - `TASK-201`
- Objective: Register `[config.schema.perimeter_arc_tolerance]` in both perimeter module manifests; replace the temporary `0.0` arc-tolerance args (set in Step 4) with `cfg.perimeter_arc_tolerance` reads at all 4 perimeter `offset(...)` call sites.
- Precondition: Step 4 complete (signature exists; modules currently passing `0.0`).
- Postcondition: `cargo test -p slicer-host --test module_manifest_tdd -- perimeter_modules_declare_arc_tolerance` PASSES. WASM guests rebuild without error.
- Files allowed to read:
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` — full
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — full
  - `modules/core-modules/classic-perimeters/src/lib.rs` — range-read at `[1-30]`, `[100-130]`, `[170-200]`
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — range-read at `[1-30]`, `[150-170]`, `[240-260]`
- Files allowed to edit (≤ 3 — split into two edit waves):
  - Wave A: `classic-perimeters.toml`, `classic-perimeters/src/lib.rs`
  - Wave B: `arachne-perimeters.toml`, `arachne-perimeters/src/lib.rs`, `slicer-host/tests/module_manifest_tdd.rs` *(new or existing)*
- Files explicitly out-of-bounds for this step:
  - Other module manifests / sources
  - `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - "Run `./modules/core-modules/build-core-modules.sh --check`; return FACT FRESH/STALE per module." — confirm baseline.
  - "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail." — rebuild after edits.
  - "Run `cargo test -p slicer-host --test module_manifest_tdd -- perimeter_modules_declare_arc_tolerance`; return FACT pass/fail." — AC-9.
- Context cost: `S`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — manifest schema (delegate SUMMARY if > 300 lines).
  - `docs/05_module_sdk.md` — config accessor pattern (delegate SUMMARY).
- OrcaSlicer refs:
  - None for this step.
- Verification:
  - `cargo test -p slicer-host --test module_manifest_tdd -- perimeter_modules_declare_arc_tolerance --nocapture` — AC-9.
  - `./modules/core-modules/build-core-modules.sh --check` — must return FRESH.
- Exit condition: Manifests carry `perimeter_arc_tolerance` block; both module `src/lib.rs` files read from `cfg` and pass to `offset(...)`; WASM guests FRESH; AC-9 green.

### Step 6: Parameterize XYZ decimal output

- Task IDs:
  - `TASK-201`
- Objective: Add `fn format_xyz(value: f32, decimals: u32) -> String` to `gcode_emit.rs` (sibling of `format_coord`, with the existing trailing-zero stripping behavior). Update only the 5 XYZ call sites (`:314`, `:317`, `:1093`, `:1096`, `:1099`) to call `format_xyz(v, cfg.gcode_xy_decimals)`. Leave `format_coord` byte-identical.
- Precondition: Step 1 complete.
- Postcondition: `cargo test -p slicer-host --test gcode_emit_format_tdd -- format_coord_decimals` PASSES (covers both functions).
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs` — lines `[300-330]`, `[1080-1170]`, `[1180-1210]`, `[1290-1320]`
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs`
  - `crates/slicer-host/tests/gcode_emit_format_tdd.rs` *(new or existing)*
- Files explicitly out-of-bounds for this step:
  - The middle of `gcode_emit.rs` (`[330-1080]` and `[1170-1290]`) unless a dispatch reveals an XYZ site there
  - All other crates
- Expected sub-agent dispatches:
  - "In `crates/slicer-host/src/gcode_emit.rs`, grep for every call to `format_coord` and return LOCATIONS. Cap 30." — confirms the audit. Implementer compares the returned list against the 5 expected XYZ sites + the F/E/temperature sites + the comment-Z sites; any unexpected hit is a new edit target or out-of-scope.
  - "Run `cargo test -p slicer-host --test gcode_emit_format_tdd -- format_coord_decimals`; return FACT pass/fail." — AC-5.
- Context cost: `S`.
- Authoritative docs:
  - `docs/08_coordinate_system.md` — Z convention.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeWriter.hpp:234` — confirm `XYZF_EXPORT_DIGITS = 3`. Delegate FACT.
- Verification:
  - `cargo test -p slicer-host --test gcode_emit_format_tdd -- format_coord_decimals --nocapture` — AC-5.
- Exit condition: AC-5 green; `cargo check --workspace` green; F / E / temperature emit unchanged (covered by NEG-2 golden in Step 9).

### Step 7: Per-ExtrusionRole tolerance dispatch + min-segment sweep at emit

- Task IDs:
  - `TASK-201`
- Objective: Insert `tolerance_for_role(role: ExtrusionRole, cfg: &ResolvedConfig) -> f32` helper and apply `simplify_polyline_mm` + `drop_short_segments_mm` at every polyline-emit loop in `gcode_emit.rs`. Travel polylines are NOT simplified.
- Precondition: Step 2 complete (helpers exist) and Step 6 complete (`format_xyz` path is in).
- Postcondition: `cargo test -p slicer-host --test gcode_emit_per_role_tolerance_tdd -- per_role_tolerance_dispatch` PASSES.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs` — narrow ranges via the locate-loops dispatch (see below)
  - `crates/slicer-ir/src/slice_ir.rs` — lines `[1310-1370]` for `ExtrusionRole` variants
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs`
  - `crates/slicer-host/tests/gcode_emit_per_role_tolerance_tdd.rs` *(new)*
- Files explicitly out-of-bounds for this step:
  - Other `slicer-host` source files
  - All modules
  - `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - "Locate every polyline-emit loop in `crates/slicer-host/src/gcode_emit.rs` that produces `G1 X Y` lines. Return LOCATIONS (file:line + 1-line context per loop). Cap 5 entries." — pins edit sites without loading the whole file.
  - "Run `cargo test -p slicer-host --test gcode_emit_per_role_tolerance_tdd -- per_role_tolerance_dispatch`; return FACT pass/fail." — AC-8.
- Context cost: `M`. Largest single step.
- Authoritative docs:
  - `docs/01_system_architecture.md` — confirm emit-stage ownership of polyline mutation (delegate SUMMARY).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp:179` — already summarized in Step 2.
- Verification:
  - `cargo test -p slicer-host --test gcode_emit_per_role_tolerance_tdd -- per_role_tolerance_dispatch --nocapture` — AC-8.
- Exit condition: AC-8 green; `cargo check --workspace` green; `tolerance_for_role` is exhaustive `match` (no wildcard arm) and includes a comment explaining why new variants intentionally fail compile.

### Step 8: WASM guest rebuild

- Task IDs:
  - `TASK-201`
- Objective: Bring guest WASM in sync with edits to perimeter module manifests/sources and to `slicer-ir`/`slicer-helpers`/`slicer-core` (which are universal guest deps per `CLAUDE.md`).
- Precondition: Steps 1, 2, 5, and any prior step that touches a guest-relevant path are complete.
- Postcondition: `./modules/core-modules/build-core-modules.sh --check` returns FRESH for every module; `./test-guests/build-test-guests.sh --check` returns FRESH.
- Files allowed to read:
  - Build script output only
- Files allowed to edit (≤ 3):
  - None directly; the build script regenerates `.wasm` artifacts
- Files explicitly out-of-bounds for this step:
  - All `.wasm` artifacts (don't open them)
- Expected sub-agent dispatches:
  - "Run `./modules/core-modules/build-core-modules.sh --check`; return FACT FRESH/STALE per module." — gate.
  - "If STALE, run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail." — rebuild.
  - "Run `./test-guests/build-test-guests.sh --check`; return FACT FRESH/STALE." — gate.
  - "If STALE, run `./test-guests/build-test-guests.sh`; return FACT pass/fail." — rebuild.
- Context cost: `S`.
- Authoritative docs:
  - `CLAUDE.md` "Guest WASM Staleness" section — already loaded in conversation context.
- OrcaSlicer refs:
  - None.
- Verification:
  - `./modules/core-modules/build-core-modules.sh --check` — FRESH.
  - `./test-guests/build-test-guests.sh --check` — FRESH.
- Exit condition: Both `--check` commands return FRESH.

### Step 9: Integration test — legacy vs default precision

- Task IDs:
  - `TASK-201`
- Objective: Slice a small fixture STL twice (default precision, then all-legacy precision via `--config` overrides) and assert: (a) default G1 X Y line count < legacy G1 X Y line count by ≥ 5%; (b) legacy output is byte-identical to a pre-recorded golden under `tests/fixtures/golden/`.
- Precondition: Steps 1, 2, 3, 4, 5, 6, 7 complete; WASM guests FRESH (Step 8).
- Postcondition: `cargo test -p slicer-host --test slicing_precision_integration_tdd -- default_emits_fewer_lines_than_legacy legacy_zero_matches_golden` PASSES.
- Files allowed to read:
  - `crates/slicer-host/tests/` — directory listing only (to pick fixture infrastructure)
  - One small fixture STL chosen from `crates/slicer-host/tests/fixtures/`
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/slicing_precision_integration_tdd.rs` *(new)*
  - `crates/slicer-host/tests/fixtures/golden/precision_legacy_<fixture>.gcode` *(new — generated by an initial test run with `BLESS_GOLDEN=1` or equivalent — implementer documents the regeneration command in the test file's top comment)*
  - Optionally a tiny synthetic STL fixture if no existing fixture is suitable
- Files explicitly out-of-bounds for this step:
  - The full golden file once recorded — treat as opaque
  - Other tests
- Expected sub-agent dispatches:
  - "List fixtures under `crates/slicer-host/tests/fixtures/`; return LOCATIONS (path + size). Pick the smallest that exercises perimeter + infill + support." — fixture choice.
  - "Run `cargo test -p slicer-host --test slicing_precision_integration_tdd -- default_emits_fewer_lines_than_legacy legacy_zero_matches_golden`; return FACT pass/fail. On fail, SNIPPETS of the assertion + ≤ 20 lines of context." — AC-10 + NEG-2.
- Context cost: `M`.
- Authoritative docs:
  - None new.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p slicer-host --test slicing_precision_integration_tdd -- default_emits_fewer_lines_than_legacy legacy_zero_matches_golden --nocapture` — AC-10 + NEG-2.
- Exit condition: Both assertions green. Golden file committed.

### Step 10: Packet completion gate

- Task IDs:
  - `TASK-201`
- Objective: Re-dispatch every pipe-suffixed AC, run packet-level verification commands, append `TASK-201` to `docs/07_implementation_status.md`, transition `packet.spec.md` to `status: implemented`.
- Precondition: Steps 1-9 complete; all individual ACs PASS in isolation.
- Postcondition: Packet ready to ship.
- Files allowed to read:
  - This packet's `packet.spec.md` and `task-map.md`
- Files allowed to edit (≤ 3):
  - `.ralph/specs/60_configurable-slicing-precision/packet.spec.md` (status change only)
  - `docs/07_implementation_status.md` (single-line append via worker dispatch — do NOT load full file)
- Files explicitly out-of-bounds for this step:
  - The body of `docs/07_implementation_status.md` (delegate the append)
- Expected sub-agent dispatches:
  - Re-dispatch every AC's `cargo test ...` command from `packet.spec.md`; each returns FACT pass/fail.
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail. On fail, SNIPPETS of first 3 warnings." — `CLAUDE.md` gate.
  - "Append `TASK-201` line to `docs/07_implementation_status.md` under DEV-009 umbrella at `:184`. Format: `- [x] TASK-201 — Configurable slicing precision (D-P at emit, arc tolerance, closing radius, gcode XY decimals; closed YYYY-MM-DD / packet 60).` Return FACT inserted-at-line-N." — backlog amendment.
  - Final: "Run `cargo check --workspace`; return FACT pass/fail." — heartbeat.
- Context cost: `S`.
- Authoritative docs:
  - None new.
- OrcaSlicer refs:
  - None.
- Verification:
  - Every AC's pipe-suffixed command — FACT pass/fail each.
  - `cargo clippy --workspace -- -D warnings` — FACT pass/fail.
  - `./modules/core-modules/build-core-modules.sh --check` — FRESH.
- Exit condition: All ACs green; clippy clean; WASM FRESH; `docs/07` updated; `packet.spec.md` status: implemented.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Single-file macro extension + 1 unit test. |
| Step 2 | M | Algorithm port + 4 unit tests; delegated OrcaSlicer SUMMARY. |
| Step 3 | S | Small geometric edit + 2 unit tests. |
| Step 4 | M | Signature change + mechanical caller updates; `cargo check` drives the audit. |
| Step 5 | S | Two manifest edits + two src updates; manifest schema test. |
| Step 6 | S | One new fn + 5 call-site updates + 1 unit test. |
| Step 7 | M | Per-role dispatch in a large emit file; LOCATIONS-driven. |
| Step 8 | S | Build-script invocation; no source edits. |
| Step 9 | M | Integration test + golden generation. |
| Step 10 | S | Re-dispatch ACs + clippy + backlog amendment. |

Aggregate: M. No step is L.

## Packet Completion Gate

- All 10 steps complete and exit conditions met.
- Every pipe-suffixed acceptance criterion command re-dispatched green.
- Packet-level `cargo clippy --workspace -- -D warnings` clean.
- `./modules/core-modules/build-core-modules.sh --check` and `./test-guests/build-test-guests.sh --check` both FRESH.
- `docs/07_implementation_status.md` updated with `TASK-201` (worker dispatch only — never loaded in full).
- No prior packet superseded by this one; this is a fresh slice.
- `packet.spec.md` transitions to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md` (10 commands; 11 if NEG-2 is dispatched separately from AC-10). Each returns FACT pass/fail; ≤ 20-line SNIPPETS on failure.
- Confirm packet-level verification:
  - `cargo check --workspace` — FACT pass.
  - `cargo clippy --workspace -- -D warnings` — FACT pass.
  - `./modules/core-modules/build-core-modules.sh --check` — FRESH.
  - `./test-guests/build-test-guests.sh --check` — FRESH.
- Record any remaining packet-local risk:
  - Risk note: legacy-mode golden may bit-rot in future packets that touch G-code emit; document the regeneration command.
  - Risk note: D-P in f32 mm-space could lose precision for accumulated polylines > 100 m; acceptable at printer scale, document in `decimate.rs`.
- Confirm peak context stayed under 70%. If not, log a packet-authoring lesson: "Step N forced direct-read of file X due to insufficient initial dispatch; future packets should pre-scope X with a tighter dispatch."
- Move `packet.spec.md` to `status: implemented`.

Workspace-test reminder: this packet does NOT require `cargo test --workspace` at any point. Every AC and every gate uses a targeted `cargo test -p <crate> --test <file>` invocation per `CLAUDE.md` "Test Discipline".
