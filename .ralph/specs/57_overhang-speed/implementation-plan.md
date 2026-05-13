# Implementation Plan: 57_overhang-speed

## Execution Rules

- One atomic step at a time.
- Each step maps back to `TASK-182` (this packet has a single task ID).
- TDD first, then implementation, then narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Files-in-scope, dispatches, and context cost are budget contracts, not metadata.
- **Step 0 is a gate.** No subsequent step may begin until Step 0's exit condition is met. The WIT field addition + binding fan-out must compile cleanly across the workspace before any classifier or dispatch work starts.

## Steps

### Step 0: WIT record extension + binding fan-out (GATE)

- Task IDs:
  - `TASK-182`
- Objective: Add `overhang-quartile: option<u8>` to the `point3-with-width` WIT record at `wit/deps/types.wit`. Mirror it on Rust `Point3WithWidth` in `crates/slicer-ir/src/slice_ir.rs:1218` with `#[serde(default)] pub overhang_quartile: Option<u8>`. Bump the IR schema minor-version constant. Propagate the new field through every WIT↔Rust conversion site in the workspace.
- Precondition: Branch on a clean tree; `cargo check --workspace` is green.
- Postcondition: `cargo build --tests --workspace` is green; every conversion site preserves `overhang_quartile` from WIT to Rust and back; the schema-version constant has been bumped one minor.
- Files allowed to read (with line-range hints):
  - `CLAUDE.md` — *WIT/Type Changes Checklist* section only.
  - `wit/deps/types.wit` — full (small).
  - `wit/deps/ir-types.wit` — full (small).
  - `crates/slicer-ir/src/slice_ir.rs` — lines `[1210-1260]` only.
  - `crates/slicer-ir/src/lib.rs` — lines around the schema-version constant (found via dispatch).
  - Every conversion site enumerated by the LOCATIONS dispatch below — read ± 40 lines around each.
- Files allowed to edit (≤ 3 primary; the binding fan-out is mechanical):
  - `wit/deps/types.wit`
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/src/lib.rs` (schema-version constant)
  - Plus the enumerated binding fan-out files (mechanical; one-line patches each).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/gcode_emit.rs`, `crates/slicer-host/src/pipeline.rs` — touched in Step 4.
  - `OrcaSlicerDocumented/**`.
  - `target/`, `Cargo.lock`, any generated bindings.
- Expected sub-agent dispatches:
  - "List every Rust call site that converts between WIT `point3-with-width` and Rust `Point3WithWidth` under `crates/` and `modules/`. Search for `Point3WithWidth { x:`, `Into<...Point3WithWidth>`, `From<...Point3WithWidth>`, and any `bindings::*::Point3WithWidth` references. Return LOCATIONS, ≤ 20 entries." — purpose: bound the fan-out.
  - "Find the IR schema minor-version constant under `crates/slicer-ir/src/`. Return FACT with file:line and the current value." — purpose: pin the bump target.
  - "Run `cargo build --tests --workspace`; return FACT pass/fail with failing assertion + ≤ 20-line SNIPPET on failure." — purpose: gate exit verification.
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `Point3WithWidth` section; delegate SUMMARY if > 300 lines.
  - `docs/03_wit_and_manifest.md` — `point3-with-width` and host-boundary sections; delegate the rest.
  - `CLAUDE.md` — *WIT/Type Changes Checklist*.
- OrcaSlicer refs: none in this step.
- Verification:
  - `cargo build --tests --workspace` — FACT pass/fail.
  - `cargo check --workspace` — FACT pass/fail.
- Exit condition: workspace compiles tests-on; no binding site silently drops `overhang_quartile` (verified by setting it to `Some(2)` in a host fixture and asserting roundtrip — preview the AC-6 test if needed). If exit fails, fix and re-run; no other step may proceed.

### Step 1: Author RED TDD scaffold

- Task IDs:
  - `TASK-182`
- Objective: Create `crates/slicer-host/tests/overhang_speed_tdd.rs` with the six AC tests + AC-N1 stubs (all currently failing). Create or extend the IR roundtrip test for AC-6.
- Precondition: Step 0 exit met.
- Postcondition: All seven tests compile but fail. `cargo test -p slicer-host --test overhang_speed_tdd` reports compiled failures (not compile errors); `cargo test -p slicer-ir --test point3_overhang_quartile_roundtrip` likewise.
- Files allowed to read:
  - `crates/slicer-host/tests/gcode_feedrate_emission_tdd.rs` — full; pattern reference for fixture construction.
  - `crates/slicer-host/tests/gcode_emit_tdd.rs` — full (range-read if > 600 lines).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/overhang_speed_tdd.rs` (new)
  - `crates/slicer-ir/tests/point3_overhang_quartile_roundtrip.rs` (new — or the existing roundtrip file identified by the dispatch below)
- Files explicitly out-of-bounds for this step:
  - All non-test source under `crates/slicer-host/src/` and `crates/slicer-core/src/` — Steps 2, 3, 4 own those.
- Expected sub-agent dispatches:
  - "Find existing `Point3WithWidth` serde/JSON roundtrip tests under `crates/slicer-ir/tests/` and `crates/slicer-ir/src/`. Return LOCATIONS." — purpose: choose new-file vs extend.
  - "Run `cargo test -p slicer-host --test overhang_speed_tdd --no-run`; return FACT pass/fail with ≤ 10-line SNIPPET on failure." — purpose: confirm scaffold compiles.
- Context cost: `S`.
- Authoritative docs:
  - `docs/08_coordinate_system.md` — confirm mm convention in fixture construction.
- OrcaSlicer refs: none in this step.
- Verification:
  - `cargo test -p slicer-host --test overhang_speed_tdd --no-run` — FACT compile-ok.
  - `cargo test -p slicer-host --test overhang_speed_tdd` — FACT all-fail (RED).
  - `cargo test -p slicer-ir --test point3_overhang_quartile_roundtrip --no-run` — FACT compile-ok.
- Exit condition: every AC-1…AC-5, AC-6, AC-N1 test exists in the suite, compiles, and currently fails for an expected reason (e.g., assertion on unimplemented classifier output).

### Step 2: Implement `LinesDistancer2D` in slicer-core

- Task IDs:
  - `TASK-182`
- Objective: Create `crates/slicer-core/src/aabb_lines_2d.rs` with `LinesDistancer2D::new(segments: &[(Point2, Point2)])`, `signed_distance(p: Point2) -> f32`, and `nearest_distance(p: Point2) -> f32`. Linear scan with bbox prefilter. Hook `pub mod aabb_lines_2d;` into `crates/slicer-core/src/lib.rs` and re-export per crate convention.
- Precondition: Step 0 + Step 1 exit met.
- Postcondition: Module compiles; unit tests in the module body cover at least: zero-segment input, single-segment nearest-distance, two-loop signed-distance with one loop CW and one CCW, point on segment endpoint, point inside a closed polygon.
- Files allowed to read:
  - `crates/slicer-core/src/aabb_tree.rs` — full IF ≤ 600 lines, else SUMMARY dispatch; pattern reference only.
  - `crates/slicer-helpers/src/` — locate `Point2` and `Point2::from_mm` via LOCATIONS dispatch.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/aabb_lines_2d.rs` (new)
  - `crates/slicer-core/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/**`.
  - `OrcaSlicerDocumented/**`.
- Expected sub-agent dispatches:
  - "Locate `Point2` and `Point2::from_mm` in the workspace; return LOCATIONS." — purpose: confirm import path.
  - "Run `cargo test -p slicer-core --lib aabb_lines_2d`; return FACT pass/fail with ≤ 20-line SNIPPET on failure." — purpose: in-module unit tests.
- Context cost: `S`.
- Authoritative docs:
  - `docs/13_slicer_helpers_crate.md` — full; small file.
- OrcaSlicer refs: none in this step (the distancer is a pure utility; the bucketization convention lives in Step 3).
- Verification:
  - `cargo build -p slicer-core` — FACT pass.
  - `cargo test -p slicer-core --lib aabb_lines_2d` — FACT pass.
  - `cargo clippy -p slicer-core -- -D warnings` — FACT clean.
- Exit condition: module unit tests green; the signed-distance sign is verified for both CW and CCW closed loops.

### Step 3: Implement `overhang_classifier` in slicer-host

- Task IDs:
  - `TASK-182`
- Objective: Create `crates/slicer-host/src/overhang_classifier.rs` with `pub fn classify_layers(layers: &mut [LayerCollectionIR], feedrate_config: &FeedrateConfig)`. Implement: short-circuit on `[0,0,0,0]`; skip layer 0; build the prev-layer wall set (`OuterWall|InnerWall|ThinWall` segments, respecting loop winding) into a `LinesDistancer2D` plus a polygon-set for the inside-test; for each `Point3WithWidth` on a wall-family path of the current layer, compute signed distance, bucket to quartile using thresholds `[0, 0.25w, 0.5w, 0.75w]`, assign `Some(q)`. `debug_assert!(q >= 1 && q <= 4)`. Mirror OrcaSlicer's `< / <=` endpoint convention exactly.
- Precondition: Steps 0–2 exit met.
- Postcondition: Module compiles; in-module unit tests cover: zero-config short-circuit (no mutation), first-layer skip, role-scope guard (non-wall points untouched), each quartile boundary endpoint, and the `debug_assert` on `q == 0` path.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs` — lines `[240-280]` only (prev-layer iteration shape).
  - `crates/slicer-ir/src/slice_ir.rs` — lines `[1210-1310]` only (Point3WithWidth, ExtrusionRole, ExtrusionPath3D).
  - `crates/slicer-core/src/aabb_lines_2d.rs` (Step 2 output).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/overhang_classifier.rs` (new)
  - `crates/slicer-host/src/lib.rs` (add `pub mod overhang_classifier;`)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/gcode_emit.rs` body — only the prev-layer iteration shape is read for pattern.
  - `crates/slicer-host/src/pipeline.rs` — Step 4.
  - `OrcaSlicerDocumented/**` — delegate.
- Expected sub-agent dispatches:
  - "Return the exact `<` vs `<=` convention at the four quartile boundaries in `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` around `:397` and `:535`. SNIPPETS, ≤ 30 lines each." — purpose: nail off-by-one for AC-5.
  - "Return the overhang overlap levels and the `overhang_N_4_speed` lookup in `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:6599-6618`. SNIPPETS, ≤ 30 lines." — purpose: confirm 4-band schedule matches our 4 keys.
  - "Run `cargo test -p slicer-host --lib overhang_classifier`; return FACT pass/fail with ≤ 20-line SNIPPET on failure." — purpose: in-module unit tests.
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `LayerCollectionIR` + `ExtrusionPath3D` sections.
  - `docs/08_coordinate_system.md` — mm convention.
- OrcaSlicer refs (delegate; never load):
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp:71,147,397,514,535`.
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:6599-6618,6639`.
- Verification:
  - `cargo build -p slicer-host` — FACT pass.
  - `cargo test -p slicer-host --lib overhang_classifier` — FACT pass.
  - `cargo clippy -p slicer-host -- -D warnings` — FACT clean.
- Exit condition: in-module unit tests green; `debug_assert!(q >= 1 && q <= 4)` is present on every quartile assignment; threshold convention SNIPPETS dispatch returned and was honored.

### Step 4: Extend `resolve_feedrate` and wire `classify_layers` into the pipeline

- Task IDs:
  - `TASK-182`
- Objective:
  1. In `crates/slicer-host/src/gcode_emit.rs`, change `resolve_feedrate` signature to `pub fn resolve_feedrate(&self, role: &ExtrusionRole, speed_factor: f32, overhang_quartile: Option<u8>) -> Option<f32>`. Before the existing role match, return wall-family overhang dispatch when `role ∈ {OuterWall, InnerWall, ThinWall}`, `overhang_quartile == Some(q)`, and `self.feedrate_config.overhang_{q}_4_speed > 0.0`; the return value is `speed × 60 × clamped(speed_factor)`. Otherwise fall through.
  2. Update the per-point emission site at `:388` to `self.resolve_feedrate(role, entity.path.speed_factor, point.overhang_quartile)`.
  3. Update the z-hop site at `:443` (and any other `resolve_feedrate` caller enumerated by dispatch) to pass `None`.
  4. In `crates/slicer-host/src/pipeline.rs`, insert `overhang_classifier::classify_layers(&mut layer_irs, &feedrate_config)` between the layer-finalization output and the `DefaultGCodeEmitter::emit_gcode` call in BOTH pipeline arms.
- Precondition: Steps 0–3 exit met.
- Postcondition: All seven Step-1 tests pass GREEN (AC-1 … AC-5 + AC-N1 + AC-6 was already on a separate test).
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs` — lines `[140-200]`, `[355-400]`, `[435-460]` only.
  - `crates/slicer-host/src/pipeline.rs` — full IF ≤ 600 lines, else range-read around the two `emit_gcode` call sites (LOCATIONS dispatch).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs`
  - `crates/slicer-host/src/pipeline.rs`
  - (Optional spillover only if another `resolve_feedrate` caller surfaced from dispatch.)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/overhang_classifier.rs` — finalized in Step 3.
  - `crates/slicer-core/src/**` — finalized in Step 2.
  - `crates/slicer-ir/src/**` — finalized in Step 0.
  - `OrcaSlicerDocumented/**`.
- Expected sub-agent dispatches:
  - "List every call site of `resolve_feedrate` in the workspace. Return LOCATIONS." — purpose: ensure every caller is updated.
  - "List every `DefaultGCodeEmitter::emit_gcode` call in `crates/slicer-host/src/pipeline.rs`. Return LOCATIONS." — purpose: confirm both pipeline arms.
  - "Run `cargo test -p slicer-host --test overhang_speed_tdd`; return FACT pass/fail with failing test name + assertion + ≤ 20-line SNIPPET on failure." — purpose: AC dispatch.
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `LayerCollectionIR` ordering invariants.
- OrcaSlicer refs (delegate): same set as Step 3 if a tie-break is needed; no new reads expected.
- Verification:
  - `cargo build -p slicer-host` — FACT pass.
  - `cargo test -p slicer-host --test overhang_speed_tdd` — FACT all-pass (GREEN, all seven tests).
  - `cargo test -p slicer-ir --test point3_overhang_quartile_roundtrip` — FACT pass (AC-6 GREEN).
  - `cargo clippy -p slicer-host -- -D warnings` — FACT clean.
- Exit condition: every AC defined in `packet.spec.md` returns PASS via a per-AC dispatch; the per-point loop at `gcode_emit.rs:362-393` is structurally unchanged except for the one threading change at `:388`.

### Step 5: Regression sweep + clippy gate

- Task IDs:
  - `TASK-182`
- Objective: Confirm no neighboring test suite regressed and the clippy gate is clean across the three primary crates plus the workspace check.
- Precondition: Step 4 exit met.
- Postcondition: All listed regression tests pass; `cargo clippy` reports zero diagnostics; `cargo check --workspace` is clean.
- Files allowed to read: none (pure dispatch step).
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds for this step:
  - All source files — no edits allowed; if a regression surfaces, raise it as a packet-blocking finding and return to the prior step.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_feedrate_emission_tdd`; FACT pass/fail." — packet 52 regression.
  - "Run `cargo test -p slicer-host --test gcode_emit_tdd`; FACT pass/fail." — emit-shape regression.
  - "Run `cargo test -p slicer-host --test orca_comment_contract_tdd`; FACT pass/fail." — `;TYPE:` label regression.
  - "Run `cargo clippy -p slicer-ir -p slicer-core -p slicer-host -- -D warnings`; FACT clean/dirty with the first three warnings if dirty." — clippy gate.
  - "Run `cargo check --workspace`; FACT pass/fail." — WIT-binding drift gate.
- Context cost: `S`.
- Authoritative docs: none in this step.
- OrcaSlicer refs: none in this step.
- Verification:
  - The five dispatches above. The step is GREEN only when all five return PASS.
- Exit condition: every dispatch returns PASS. If any returns FAIL, classify (regression vs orthogonal) and either return to the relevant prior step or raise to the user.

### Step 6: Documentation updates

- Task IDs:
  - `TASK-182`
- Objective: Append a remediation note to `docs/DEVIATION_LOG.md` for DEV-009 (cite packet 57 closure for the per-quartile wall slowdown). Do NOT edit the TASK-182 row yet — that happens in Step 7 as the close gate.
- Precondition: Step 5 exit met.
- Postcondition: DEV-009 row in `docs/DEVIATION_LOG.md` has a new sentence/clause in the *Remediation progress* field referencing packet 57.
- Files allowed to read:
  - `docs/DEVIATION_LOG.md` — DEV-009 row only (LOCATIONS dispatch first).
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md`
- Files explicitly out-of-bounds for this step:
  - `docs/07_implementation_status.md` — closure happens in Step 7.
- Expected sub-agent dispatches:
  - "Return the DEV-009 row in `docs/DEVIATION_LOG.md` as SNIPPETS (≤ 15 lines)." — purpose: targeted Edit.
- Context cost: `S`.
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` (row read above).
- OrcaSlicer refs: none.
- Verification:
  - Re-dispatch the SNIPPETS read of the DEV-009 row; FACT-confirm the new clause is present.
- Exit condition: DEV-009 row carries an explicit packet-57 remediation clause.

### Step 7: Packet completion gate / acceptance ceremony

- Task IDs:
  - `TASK-182`
- Objective: Close the packet. Re-dispatch every pipe-suffixed AC command from `packet.spec.md`. Close the TASK-182 row in `docs/07`. Run the `cargo test --workspace` close-time ceremony (per CLAUDE.md, only at packet close).
- Precondition: Steps 0–6 exit met.
- Postcondition: `packet.spec.md` updated to `status: implemented`; TASK-182 marked `[x]` in `docs/07`; workspace test suite reported pass.
- Files allowed to read:
  - `.ralph/specs/57_overhang-speed/packet.spec.md` — full.
- Files allowed to edit (≤ 3):
  - `.ralph/specs/57_overhang-speed/packet.spec.md` (status flip)
  - `docs/07_implementation_status.md` (TASK-182 closure row)
- Files explicitly out-of-bounds for this step:
  - All source crates and `OrcaSlicerDocumented/**`.
- Expected sub-agent dispatches:
  - Re-dispatch each AC's pipe-suffixed `cargo test` command; FACT pass per AC.
  - "In `docs/07_implementation_status.md`, return LOCATIONS for the TASK-182 line." — purpose: targeted Edit.
  - "Run `cargo test --workspace`; return FACT pass/fail with failing test name + ≤ 20-line SNIPPET on failure." — purpose: close-time ceremony only.
- Context cost: `M`.
- Authoritative docs:
  - `CLAUDE.md` — *Test Discipline* section (confirms `cargo test --workspace` is appropriate at packet close).
- OrcaSlicer refs: none.
- Verification:
  - Per-AC `cargo test` dispatches.
  - `cargo test --workspace` (close-time only).
  - LOCATIONS-then-Edit on `docs/07_implementation_status.md` to flip `[ ] TASK-182` to `[x] TASK-182 ... **Closed YYYY-MM-DD via packet 57 — <one-line evidence>.**`.
- Exit condition: every AC PASS; workspace ceremony PASS; `status: implemented` in this packet's `packet.spec.md`; TASK-182 row closed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | M | WIT fan-out enumeration is the heaviest dispatch; cap at LOCATIONS ≤ 20. |
| Step 1 | S | Pure test-writing; only one secondary file read. |
| Step 2 | S | Self-contained utility; in-module unit tests only. |
| Step 3 | M | OrcaSlicer SNIPPETS reads and classifier body; in-module unit tests. |
| Step 4 | M | Two-arm pipeline edit + per-point thread; AC dispatches. |
| Step 5 | S | Pure-dispatch regression sweep. |
| Step 6 | S | Single doc row edit. |
| Step 7 | M | Workspace ceremony + AC re-dispatch + closure edit. |

Aggregate: `M`. No single step is `L`.

## Packet Completion Gate

- All steps complete.
- Every step exit condition met.
- Every AC pipe-suffixed command dispatched and returned PASS.
- `docs/07_implementation_status.md` updated for TASK-182 via worker dispatch (LOCATIONS-then-Edit; never loaded in full).
- DEV-009 remediation note appended in `docs/DEVIATION_LOG.md`.
- `packet.spec.md` flipped to `status: implemented`.
- No prior packet status transition required (this packet does not reopen or supersede).

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-6 and AC-N1). Each returns FACT PASS.
- Run `cargo test --workspace` as the close-time ceremony; FACT PASS.
- Confirm `cargo check --workspace` and `cargo clippy -p slicer-ir -p slicer-core -p slicer-host -- -D warnings` are green.
- Record any remaining packet-local risk explicitly (e.g., `LinesDistancer2D` BVH deferral, four-band vs Orca's six-overlap-level schedule, smoothed-mode deferral) before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future `spec-packet-generator` runs (the WIT fan-out enumeration in Step 0 is the most likely budget pressure point — keep the LOCATIONS dispatch strict).
