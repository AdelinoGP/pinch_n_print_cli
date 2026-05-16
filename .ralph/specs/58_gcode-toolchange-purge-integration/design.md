# Design: 58_gcode-toolchange-purge-integration

## Controlling Code Paths

- **Primary code path**: `wipe-tower` module's per-layer fn under `run_finalization` inserts purge `PrintEntity` rows into each `LayerCollectionIR` → `GCodeEmitter` (in `crates/slicer-host/src/gcode_emit.rs`) serializes those entities in order with `;TYPE:Wipe tower` markers and a defensive guard around the existing bare `T<n>` emission at lines 1155-1156.
- **Neighboring tests/fixtures**:
  - `crates/slicer-host/tests/tool_ordering_tdd.rs` — closest neighbor; demonstrates `LayerCollectionIR` assembly idioms and tool-index propagation via `region_key.region_id`.
  - `modules/core-modules/wipe-tower/src/lib.rs` and `modules/core-modules/wipe-tower/wipe-tower.toml` — module under change.
  - `crates/slicer-host/tests/fixtures/` — target dir for the new STL + reference G-code (Step 2 adds the files).
- **OrcaSlicer comparison surface**: `WipeTower2.cpp:1557-1640` for Unload/Change/Load/Wipe ordering; `WipeTower2.cpp:2069-2205` for `finish_layer()` perimeter+infill polygon emission shape.

## Architecture Constraints

- `wipe-tower` runs in `PostPass::LayerFinalization` and mutates `&mut Vec<LayerCollectionIR>`. It MUST NOT run after `PostPass::GCodeEmit`. The sequential `for module in &stage.modules` loop at `crates/slicer-host/src/layer_finalization.rs:83-108` consumes a module ordering that is populated upstream during `ExecutionPlan` build; the Step 4 dispatch confirms `wipe-tower` is positioned last among entity-injecting `LayerFinalization` modules (`skirt-brim`, `part-cooling`, `top-surface-ironing`).
- New entities added by `wipe-tower` use existing IR fields. Adding fields to `ToolChange` or `PrintEntity` is out of scope. `ExtrusionRole::WipeTower` already exists at `crates/slicer-ir/src/slice_ir.rs:1233-1262` (confirmed during spec-review); no enum change required. Step 1 reverifies the variant has not been removed.
- The `wipe_tower_enabled` config flag is the canonical gate. When `false`, the wipe-tower module skips emission entirely (existing behavior from packet 17 — verify, do not modify).
- Per `docs/02_ir_schemas.md` determinism contract, purge entity positions must be deterministic given the same input. Use `wipe_tower_x`, `wipe_tower_y`, `wipe_tower_width`, `line_width` from config — no RNG, no allocation-order dependence.
- Coordinate units: X/Y in scaled integers (1 unit = 100 nm, per `docs/08_coordinate_system.md`). Z in mm `f32`. Tower polygon vertices via `Point2::from_mm`.
- Per packet 11's emission contract, role labels are `;TYPE:<RoleName>`. The wipe-tower role label is `;TYPE:Wipe tower` (with that exact spelling and capitalization, matching OrcaSlicer ecosystem `;TYPE:` parity). The user's ACs accept either `Wipe tower` or `Prime tower`; this packet emits `Wipe tower`. If Step 1's dispatch on `docs/02_ir_schemas.md`'s role table mandates `Prime tower`, update AC4/AC5/NC2 spelling in lockstep before Step 3 begins.

## Code Change Surface

- **Selected approach** — "Entity-level injection". The `wipe-tower` module inserts dedicated `PrintEntity` rows (tagged with `ExtrusionRole::WipeTower`) into each layer's entity list immediately before and after each `ToolChange.after_entity_index`. The `GCodeEmitter` already serializes entities in order, and `orca_type_label` at `gcode_emit.rs:218-235` already maps `ExtrusionRole::WipeTower → ";TYPE:Wipe tower"` (and `PrimeTower → ";TYPE:Prime tower"`). The only new emitter code is a defensive guard that rejects an unbracketed `ToolChange` under `wipe_tower_enabled=true`, plus an additive `PostpassError::MissingToolchangePurge` variant in `postpass.rs`.

- **Exact functions, traits, manifests, tests, or fixtures expected to change**:
  - `modules/core-modules/wipe-tower/src/lib.rs::run_finalization` (or the per-layer inner fn it dispatches to) — for each `LayerCollectionIR.tool_changes` entry, emit, in order: a retract entity (negative E delta), a travel entity to tower X/Y, the tower wall + infill polygon entities with `ExtrusionRole::WipeTower`, the wipe rows, and a prime entity whose cumulative E delta equals `wipe_tower_purge_volume` mm (converted to extrusion length via the existing `volume_to_length` analog or by `length = volume / cross_section_area(line_width, layer_height)`).
  - `crates/slicer-host/src/gcode_emit.rs::orca_type_label` at lines 218-235 — already returns `";TYPE:Wipe tower"` for `ExtrusionRole::WipeTower` and `";TYPE:Prime tower"` for `ExtrusionRole::PrimeTower`. **No edit required**; Step 1 verifies the arm remains intact.
  - `crates/slicer-host/src/gcode_emit.rs` tool-change emission block at lines 1155-1156 — add a defensive check: when `wipe_tower_enabled=true`, the ±N surrounding entities of a `ToolChange` must include at least one retract (negative E) before and at least one `ExtrusionRole::WipeTower` entity after; otherwise return `PostpassError::MissingToolchangePurge { layer_index, tool_change_index }`.
  - `crates/slicer-host/src/postpass.rs::PostpassError` at lines 39-59 — add the additive `MissingToolchangePurge { layer_index: usize, tool_change_index: usize }` variant. Existing variants (`FatalModule`, `GCodeEmit { message }`, `GCodeSerialization { message }`) are untouched.
  - `crates/slicer-ir/src/slice_ir.rs::ExtrusionRole` at lines 1233-1262 — `WipeTower` variant already present. **No edit required**.
  - **New**: `crates/slicer-host/tests/gcode_toolchange_wrapping.rs` — TDD-first integration tests for AC1, AC3, NC1.
  - **New**: `modules/core-modules/wipe-tower/src/lib.rs#tests` — unit tests for AC4 (role marker) and AC6 (geometry bounds).
  - **New**: `crates/slicer-host/tests/fixtures/multi_color_cube.stl` and `multi_color_cube.orca.gcode` — checked-in fixtures used by AC2, AC3, AC5, NC2, NC3.

- **Rejected alternatives** (must choose one):
  1. **"Emitter-level wrapping"** — synthesize retract/prime moves at G-code emit time from config alone. Rejected because purge geometry depends on layer-level state (object footprints, prior wipe-tower remnants) that the `wipe-tower` module already owns. Synthesizing twice violates the source-of-truth rule and duplicates determinism risk.
  2. **"New `PurgeIR` IR struct attached to `ToolChange`"** — add a `purge: Option<PurgeSequence>` field. Rejected because it introduces a new versioned IR shape for a fix fully expressible as additional `PrintEntity` rows tagged with one new (additive) role variant. Bigger blast radius, more migration surface, no behavioral gain.
  3. **"Flip `wipe_tower_enabled` default to true"** — out of scope per the user's bugfix-only directive. Default behavior unchanged.
  4. **"Borrow OrcaSlicer's `;Wipe_Tower_Start` / `;Wipe_Tower_End` marker pair"** — rejected because packet 11's emission contract uses `;TYPE:<RoleName>` exclusively. A parallel marker style fragments the contract and breaks G-code consumers (slicers, viewers) that rely on the `;TYPE:` convention.

## Files in Scope (read + edit)

Four primary edit targets (one is a single additive variant on `PostpassError`):

- `modules/core-modules/wipe-tower/src/lib.rs` — primary; emit retract/prime/wipe entities around each `ToolChange`; tag with `ExtrusionRole::WipeTower`; add two unit tests in `#[cfg(test)] mod tests`.
- `crates/slicer-host/src/gcode_emit.rs` — primary; add `MissingToolchangePurge` guard around the existing T<n> writeln at lines 1155-1156. `orca_type_label` (218-235) already maps the `WipeTower` role — no edit there.
- `crates/slicer-host/src/postpass.rs` — **additive only**: one new `PostpassError::MissingToolchangePurge { layer_index, tool_change_index }` variant in the enum at lines 39-59.
- `crates/slicer-host/tests/gcode_toolchange_wrapping.rs` — primary; new TDD file driving the wrapping invariant + parity check + rejection test.

`crates/slicer-ir/src/slice_ir.rs` is **read-only** — `ExtrusionRole::WipeTower` already exists at lines 1233-1262.

Test fixture files (`multi_color_cube.stl`, `multi_color_cube.orca.gcode`) are data, not code; they do not count against the file limit.

## Read-Only Context

- `docs/02_ir_schemas.md` — delegate via SUMMARY for `ExtrusionRole` variants and `ToolChange`/`PrintEntity`/`LayerCollectionIR` shapes. Do not load directly.
- `docs/03_wit_and_manifest.md` — range-read the wipe-tower manifest schema and the `FinalizationOutputBuilder` exports only.
- `docs/04_host_scheduler.md` — range-read the LayerFinalization → GCodeEmit boundary only.
- `docs/08_coordinate_system.md` — direct read (short file).
- `docs/09_progress_events.md` — direct read; confirm no progress event is being violated.
- `docs/11_operational_governance_and_acceptance_gate.md` — range-read §1 (deviation log entry format) only.
- `crates/slicer-ir/src/slice_ir.rs:1435-1469` — `ToolChange` (1435-1442) and `TravelRetract` (1455-1469) definitions (range-read).
- `crates/slicer-ir/src/slice_ir.rs:740-760` — `ActiveRegion.tool_index` at line 750 (range-read; field is single-line amid comments).
- `crates/slicer-ir/src/slice_ir.rs:1524-1543` — `LayerCollectionIR.tool_changes` at line 1534 (range-read).
- `crates/slicer-ir/src/slice_ir.rs:1233-1262` — `ExtrusionRole` enum (range-read; `WipeTower` and `PrimeTower` variants already present).
- `crates/slicer-host/src/gcode_emit.rs:290-410` — current toolchange emission entry (range-read).
- `crates/slicer-host/src/gcode_emit.rs:1140-1170` — bare `T<n>` writeln at 1155-1156 (range-read).
- `crates/slicer-host/src/gcode_emit.rs:218-235` — `orca_type_label` role-to-`;TYPE:` mapping (read-only verification; already includes `WipeTower` and `PrimeTower` arms).
- `modules/core-modules/wipe-tower/wipe-tower.toml` — full read (small).
- `crates/slicer-host/src/layer_finalization.rs:80-110` — `execute_layer_finalization` orchestration (range-read).
- `crates/slicer-host/tests/tool_ordering_tdd.rs` — full read for idioms (small, focused).

## Out-of-Bounds Files

- All of `OrcaSlicerDocumented/` — delegate every parity check.
- `target/`, `Cargo.lock`, any `.wasm` artifact.
- Any crate not listed above (`slicer-helpers`, `slicer-cli`, other `modules/core-modules/*/src/lib.rs`).
- Other module manifests in `modules/core-modules/` outside `wipe-tower/` — not relevant.
- `docs/14_deviation_audit_history.md` — read-only audit trail; only `docs/DEVIATION_LOG.md` itself is appended.
- `docs/07_implementation_status.md` in full — use a sub-agent to locate the three TASK-### line ranges for Step 6, do not load.

## Expected Sub-Agent Dispatches

- **Step 1**: "Confirm `ExtrusionRole::WipeTower` is still present at `crates/slicer-ir/src/slice_ir.rs:1233-1262` and the exact `ToolChange` field shape at `1435-1442` is unchanged; FACT ≤ 5 lines." — purpose: reverify pre-resolved facts (no enum edit planned).
- **Step 1**: "Confirm `orca_type_label` at `crates/slicer-host/src/gcode_emit.rs:218-235` still maps `ExtrusionRole::WipeTower → \";TYPE:Wipe tower\"`; FACT pass/fail." — purpose: reverify pre-resolved fact (no mapping edit planned).
- **Step 1**: "Confirm `PostpassError` at `crates/slicer-host/src/postpass.rs:39-59` still has the shape `FatalModule { stage_id, module_id, message } | GCodeEmit { message } | GCodeSerialization { message }` (no `MissingToolchangePurge` yet); FACT ≤ 5 lines." — purpose: confirm Step 3's additive variant insertion site.
- **Step 1**: "Summarize OrcaSlicer `WipeTower2.cpp:1557-1640` Unload/Change/Load/Wipe call order; FACT, ≤ 5 lines." — purpose: confirm Unload/Change/Load/Wipe ordering without loading the file.
- **Step 2**: "Run `cargo test -p slicer-host --test gcode_toolchange_wrapping`; FACT pass/fail and (on fail) SNIPPETS ≤ 20 lines of the first failing assertion." — Step verification (expect 3 failures at this stage).
- **Step 3**: "Run `cargo check --workspace`; FACT pass/fail." — type-check after the emitter edit.
- **Step 3**: "Run `cargo clippy --workspace -- -D warnings`; FACT pass/fail." — lint gate.
- **Step 3**: "Run `cargo test -p slicer-host --test gcode_toolchange_wrapping bare_toolchange_rejected -- --nocapture`; FACT pass/fail." — verify the rejection guard.
- **Step 4**: "Confirm no other `PostPass::LayerFinalization` module reads `LayerCollectionIR.entities.len()` or asserts entity-count invariants; LOCATIONS ≤ 10 entries from `modules/core-modules/{skirt-brim,part-cooling,top-surface-ironing}/src/lib.rs`." — invariant safety.
- **Step 4**: "Run `./modules/core-modules/build-core-modules.sh`; FACT exit code + last 5 lines." — WASM rebuild.
- **Step 4**: "Run `cargo test -p wipe-tower --lib`; FACT pass/fail (expect the 2 new tests green)." — module verification.
- **Step 4**: "Run `cargo test -p slicer-host --test gcode_toolchange_wrapping`; FACT pass/fail (expect all 3 green)." — wrapping verification.
- **Step 5**: "Run `cargo run --bin slicer-cli --release --slice --input ... --output ...`; FACT exit code + last 5 lines." — end-to-end slice.
- **Step 5**: "Run each AC and NC pipe-suffixed command from `packet.spec.md` against the produced G-code; FACT pass/fail per command, ≤ 1 line per AC/NC." — final verification.
- **Step 6**: "Locate the line ranges for TASK-143, TASK-152b, TASK-120d2 in `docs/07_implementation_status.md`; LOCATIONS ≤ 6 entries." — narrow line edits without loading the full file.
- **Step 6**: "Show the most recent 3 entries of `docs/DEVIATION_LOG.md`; SNIPPETS ≤ 30 lines each." — format reference.

## Data and Contract Notes

- **IR contracts**: `ExtrusionRole::WipeTower` is additive; no migration burden. `ToolChange` shape is unchanged. `LayerCollectionIR.tool_changes` is read-only for `gcode_emit.rs` (existing behavior).
- **WIT boundary**: `wipe-tower` continues to call `push-print-entity` (or its existing equivalent) via the layer-collection-builder WIT. No WIT change.
- **Determinism**: tower X/Y from config keys; line spacing from `line_width`; purge volume from `wipe_tower_purge_volume`. No RNG. The wipe-tower module already enforces deterministic emission for skirt-brim and top-surface-ironing — follow the same pattern.
- **Scheduler**: `wipe-tower` is one of several `PostPass::LayerFinalization` modules. Per `crates/slicer-host/src/layer_finalization.rs:83-108`, modules run sequentially on the same `&mut Vec<LayerCollectionIR>`. The Step 4 dispatch confirms that no neighboring finalization module asserts entity-count invariants that adding wipe-tower entities would break.

## Locked Assumptions and Invariants

- `wipe_tower_enabled=false` (the default for non-multi-material slices) keeps current behavior. No regression to single-color paths.
- The wipe-tower module is the **only** emitter of `ExtrusionRole::WipeTower` entities and `;TYPE:Wipe tower` markers. No other module emits this role.
- `ToolChange.after_entity_index` semantics are stable across `path-optimization-default` (which pushes the tool changes) and `wipe-tower` (which reads them). Both ordering and indexing were closed by packet 19; this packet does not perturb either.
- Purge geometry vertices are scaled integers (100 nm units) and computed in the same `Point2` arithmetic the rest of the codebase uses.
- The new fixture is < 64 KB STL and < 256 KB OrcaSlicer reference G-code; both are checked into the repo (not git-lfs).
- The codebase has no standalone `volume_to_length` helper. The forward per-segment extrusion math at `crates/slicer-host/src/gcode_emit.rs:363-371` is `E = distance * width * flow_factor`. The wipe-tower module needs the inverse direction (given a target purge volume, compute the prime length); implement inline as `length_mm = volume_mm3 / (line_width_mm * layer_height_mm)` using `wipe_tower_purge_volume`, `line_width`, and the active layer height. Do NOT add a new shared helper as part of this packet — keep the conversion local to `wipe-tower/src/lib.rs`.

## Risks and Tradeoffs

- **Risk**: another `PostPass::LayerFinalization` module re-orders entities after `wipe-tower` runs. → **Mitigation**: Step 4 dispatch verifies finalization order keeps `wipe-tower` last among entity-injecting modules, or the new integration test asserts the post-finalization ordering directly.
- **Design decision (was a risk)**: AC6 runs against module-internal stub `PrintConfig.bed_polygon` and stub object footprint. The wipe-tower module does not currently have host-service access to live bed-bounds; exposing it would expand scope beyond this bugfix. → **Resolution**: AC6 is gated on the stub; real cross-module bed-bounds enforcement is deferred to a follow-up packet recorded in Step 6's `docs/DEVIATION_LOG.md` entry.
- **Risk**: ±20% purge-volume parity (AC3) is sensitive to extrusion-width quirks between Slicer A and B. → **Mitigation**: tolerance is loose; the test reports a SNIPPETS diff on first failure so the gap is visible without a re-run.
- **Tradeoff**: adding `MissingToolchangePurge` to `PostpassError` enlarges the error enum by one variant (from 3 to 4). Acceptable — silent regression to a bare `T<n>` is the bug this packet fixes; the explicit error is the regression guard.
- **Tradeoff**: emitting `;TYPE:Wipe tower` rather than OrcaSlicer's `;Wipe_Tower_Start/End` makes side-by-side diff against Orca files slightly more verbose. Acceptable — packet 11 owns the marker style. (The `orca_type_label` arm for this is already in place at `gcode_emit.rs:218-235`.)

## Context Cost Estimate

- Aggregate (sum across 6 steps): **M**.
- Largest single step: **M** (Step 4 — module emission + 2 unit tests + WASM rebuild; the new entities and their geometry are the bulk of the work).
- Highest-risk dispatch: the `OrcaSlicer WipeTower2.cpp:1557-1640` ordering summary — must return FACT in ≤ 5 lines. If the response exceeds the contract, re-dispatch with tighter scope; do not paste oversize replies.

## Open Questions

None blocking activation. The packet is ready to move from `draft` to `active` after user review.

**Resolved facts** (pre-confirmed during spec-review; Step 1 reverifies they have not regressed):

- `ExtrusionRole::WipeTower` already exists at `crates/slicer-ir/src/slice_ir.rs:1233-1262` (variant at line 1251; `PrimeTower` at 1253). No additive enum variant needed.
- `orca_type_label` at `crates/slicer-host/src/gcode_emit.rs:218-235` already maps `ExtrusionRole::WipeTower → ";TYPE:Wipe tower"` (line 230) and `PrimeTower → ";TYPE:Prime tower"` (line 231). No mapping edit needed.
- The codebase error type for the emit path is `PostpassError` at `crates/slicer-host/src/postpass.rs:39-59`. Current variants: `FatalModule { stage_id, module_id, message }`, `GCodeEmit { message }`, `GCodeSerialization { message }`. Packet 58's `MissingToolchangePurge { layer_index, tool_change_index }` is added additively.
- AC6 runs against module-internal stubs — real host-service bed-bounds access is out of scope; follow-up is tracked in Step 6's `docs/DEVIATION_LOG.md` entry. AC6's wording explicitly says "stub bed_polygon" / "stub object footprints".
- `docs/02_ir_schemas.md` does NOT publish a separate `;TYPE:` role table; the authoritative `;TYPE:` mapping lives at `orca_type_label` in `gcode_emit.rs` (already includes the `Wipe tower` / `Prime tower` arms). AC4/AC5/NC2 spelling is correct as written.
- No standalone `volume_to_length` helper exists. The forward per-segment math `E = distance * width * flow_factor` lives at `gcode_emit.rs:363-371`. Step 4 computes the inverse inline within `wipe-tower/src/lib.rs` per the formula in the locked-assumptions section above.
- `slice_ir.rs:750`'s `tool_index` field is on struct `ActiveRegion` (not `ObjectLayerRegion`).
