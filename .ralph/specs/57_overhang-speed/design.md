# Design: 57_overhang-speed

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-host/src/gcode_emit.rs` — `resolve_feedrate` (line 154), per-point emission loop (lines 362–393), z-hop site (line 443). The per-point loop already iterates `Point3WithWidth`; only the new arg needs threading.
  - `crates/slicer-host/src/pipeline.rs` — both pipeline arms (slicer-cli binary and WASM execution). Wire `overhang_classifier::classify_layers(...)` between layer finalization and `DefaultGCodeEmitter::emit_gcode`.
  - `crates/slicer-ir/src/slice_ir.rs` — `Point3WithWidth` (line 1218): add the new optional `overhang_quartile: Option<u8>` field with `#[serde(default)]`.
  - `wit/deps/types.wit` — `point3-with-width` record (lines 7–11): add `overhang-quartile: option<u8>`.
  - **NEW** `crates/slicer-core/src/aabb_lines_2d.rs` — `LinesDistancer2D` struct (naïve linear scan + AABB prefilter).
  - **NEW** `crates/slicer-host/src/overhang_classifier.rs` — `classify_layers` entry point.

- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/gcode_feedrate_emission_tdd.rs` — packet 52's regression suite; structurally mirror the fixture-construction style.
  - `crates/slicer-host/tests/gcode_emit_tdd.rs` — emit-shape regression.
  - `crates/slicer-host/tests/orca_comment_contract_tdd.rs` — `;TYPE:` label invariants.
  - `crates/slicer-ir/tests/` — existing IR roundtrip tests give the pattern for AC-6.

- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` (lines 71, 147, 397, 514, 535).
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (lines 4804, 5324, 6599, 6604–6618, 6620, 6639).

## Architecture Constraints

- WIT boundary integrity: any field added to the `point3-with-width` record propagates to every host-binding and guest-binding site. Per `CLAUDE.md` *WIT/Type Changes Checklist*: search `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules; verify type identity matches across component boundaries; run `cargo build --tests` after the change.
- IR schema evolution: bumping the schema minor version constant is mandatory (per `docs/02_ir_schemas.md` versioning rules). New field on a host-serialized struct without a version bump is a contract regression.
- Coordinate convention: `Point3WithWidth.x/y/z` are `f32` mm at the emitter layer (per usage at `gcode_emit.rs:380` and `docs/08_coordinate_system.md`). The classifier MUST work in mm and use `Point2::from_mm` (per `docs/13_slicer_helpers_crate.md`).
- Determinism: classification iterates `layer_irs.windows(2)`; for each layer, paths and points are visited in their existing IR order. No hashing, no parallel iteration that could reorder writes.
- Backpressure gate (per `CLAUDE.md`): `cargo build`, narrow tests, and `cargo clippy` must pass before any packet-close motion. `cargo test --workspace` runs only at the close ceremony.

## Code Change Surface

- Selected approach: **per-point** carrying of `overhang_quartile: Option<u8>` on `Point3WithWidth`, with a host-side prepass (`overhang_classifier`) populating it after layer finalization and before G-code emission. `resolve_feedrate` consumes the per-point value and dispatches wall-family roles to `overhang_{q}_4_speed × 60 × clamped(speed_factor)`. The WIT record `point3-with-width` mirrors the field so it survives any module roundtrip.

- Exact functions, traits, manifests, tests, or fixtures expected to change (kept ≤ 3 *primary* surfaces; binding fan-out is mechanical):

  **Primary code surfaces:**
  1. `crates/slicer-ir/src/slice_ir.rs` — `Point3WithWidth` struct; schema-version constant.
  2. `crates/slicer-host/src/gcode_emit.rs` — `resolve_feedrate` signature + dispatch; per-point emission site; z-hop site; any other `resolve_feedrate` caller.
  3. `crates/slicer-host/src/overhang_classifier.rs` (new) + `crates/slicer-core/src/aabb_lines_2d.rs` (new).

  **Mechanical secondary surfaces (binding fan-out):**
  - `wit/deps/types.wit` — record field addition.
  - `crates/slicer-host/src/wit_host*.rs` / `dispatch*.rs` — every conversion site between Rust `Point3WithWidth` and the WIT record (LOCATIONS dispatch in Step 0 enumerates them; expected ≤ 8 sites).
  - Guest binding crates that re-emit the WIT type into Rust on the WASM side (typically `wit_guest` modules under `crates/slicer-sdk` or `modules/core-modules/*`).
  - `crates/slicer-host/src/pipeline.rs` — two `classify_layers` call sites.
  - `crates/slicer-host/src/lib.rs` — `pub mod overhang_classifier;`.
  - `crates/slicer-core/src/lib.rs` — `pub mod aabb_lines_2d;` (+ re-export if conventional in this crate).

- Rejected alternatives:
  - **Per-segment carrying on `ExtrusionPath3D`.** Rejected: the per-point loop at `gcode_emit.rs:362-393` already emits F per `Point3WithWidth`. A per-segment representation would require either coalescing or re-keying back to point indices at emission time, which costs more than the optional byte per point.
  - **Adding `ExtrusionRole::Overhang` and reclassifying paths upstream.** Rejected: would force a new role-classification stage *before* layer finalization (since `LayerCollectionIR` is partially immutable post-finalization), and would conflict with packet 52's role/speed dispatch that's already shipped. The role stays `OuterWall|InnerWall|ThinWall` and the per-point quartile carries the modulation.
  - **Bridge-double-modulation (run classifier over `BridgeInfill` too).** Rejected per the user's clarification: `bridge_speed` already captures the "in air" semantics; double-modulating would conflict.
  - **Smoothed/interpolated speed mode.** Rejected for this packet: quantized first; smoothed is a refinement packet.
  - **Line-distance only (no inside-test).** Rejected: would falsely classify points above an internal hole as supported. Polygon-based inside-test (winding-rule) chosen up front for closer Orca parity. Marginal extra code.

## Files in Scope (read + edit)

Target ≤ 3 primary files (kept) plus the new files (which by definition are owned by this packet). The binding fan-out is enumerated by a LOCATIONS dispatch in Step 0 so it does not bloat this list.

- `crates/slicer-ir/src/slice_ir.rs` — role: `Point3WithWidth` definition; expected change: add `overhang_quartile: Option<u8>` field with `#[serde(default)]`, bump schema version constant.
- `crates/slicer-host/src/gcode_emit.rs` — role: `resolve_feedrate` + per-point emission; expected change: extend signature, add dispatch arm for wall roles, update per-point and z-hop call sites.
- `crates/slicer-host/src/pipeline.rs` — role: pipeline arms; expected change: insert `classify_layers` call after layer finalization in both arms.
- `crates/slicer-host/src/overhang_classifier.rs` — NEW; role: classifier; expected change: full file.
- `crates/slicer-core/src/aabb_lines_2d.rs` — NEW; role: 2D distancer utility; expected change: full file.
- `crates/slicer-host/src/lib.rs` — role: module declaration; expected change: `pub mod overhang_classifier;`.
- `crates/slicer-core/src/lib.rs` — role: module declaration; expected change: `pub mod aabb_lines_2d;` and re-export.
- `wit/deps/types.wit` — role: WIT record; expected change: add `overhang-quartile: option<u8>` to `point3-with-width`.
- Binding fan-out (≤ 8 conversion sites; enumerated by Step 0 dispatch): touched only at the named conversion points, not browsed in full.
- `crates/slicer-host/tests/overhang_speed_tdd.rs` — NEW; role: AC-1…AC-5 + AC-N1 tests.
- `crates/slicer-ir/tests/point3_overhang_quartile_roundtrip.rs` — NEW (or extend existing roundtrip file if one exists per Step 1 LOCATIONS dispatch); role: AC-6 test.
- `docs/DEVIATION_LOG.md` — role: remediation log; expected change: append DEV-009 progress note (Step 6).
- `docs/07_implementation_status.md` — role: TASK-182 closure row; expected change: flip the checkbox in Step 7.

## Read-Only Context

The implementer may read these but not edit them; line-range hints respect the 600-line read cap.

- `crates/slicer-host/src/gcode_emit.rs` — range-read lines `[140-200]` (`resolve_feedrate`), `[240-280]` (`emit_gcode` prev-layer iteration shape), `[355-400]` (per-point emission), `[435-460]` (z-hop site).
- `crates/slicer-ir/src/slice_ir.rs` — range-read lines `[1210-1260]` (Point3WithWidth, ExtrusionRole), `[1280-1310]` (ExtrusionPath3D), and the schema-version constant block (LOCATIONS dispatch to find exact lines).
- `crates/slicer-core/src/aabb_tree.rs` — read in full IF ≤ 600 lines; otherwise delegate SUMMARY. Pattern reference only; do not modify.
- `wit/deps/types.wit` — load in full (small file).
- `wit/deps/ir-types.wit` — load in full (small file). Confirms downstream consumers of `point3-with-width`.
- `docs/02_ir_schemas.md` — `Point3WithWidth` section only. Delegate SUMMARY if > 300 lines.
- `docs/03_wit_and_manifest.md` — `point3-with-width` and host-boundary sections only. Delegate the rest as SUMMARY.
- `docs/08_coordinate_system.md` — read in full (small).
- `docs/13_slicer_helpers_crate.md` — read in full (small).
- `CLAUDE.md` — re-read *WIT/Type Changes Checklist* section before Step 0.

## Out-of-Bounds Files

The implementer MUST NOT load these directly; delegate any fact-check.

- `OrcaSlicerDocumented/**` — every read delegated; SNIPPETS return only.
- `target/`, `Cargo.lock`, any generated `wit_bindgen` output — never load.
- Vendored deps — never load.
- `docs/07_implementation_status.md` in full — dispatch LOCATIONS and Edit; do not browse.
- Any crate outside the change surface (e.g., `slicer-cli` source beyond pipeline call sites, paint modules, finalization modules) — delegate impl/caller lookups via Grep tool with a tight scope; do not browse.
- `crates/slicer-host/src/gcode_emit.rs` in full — range-read per `Read-Only Context` only.
- `crates/slicer-ir/src/slice_ir.rs` in full — range-read per `Read-Only Context` only.

## Expected Sub-Agent Dispatches

Not exhaustive; covers the predictable ones.

- **Step 0 — WIT fan-out enumeration:** "List every Rust call site that converts between WIT `point3-with-width` and Rust `Point3WithWidth` under `crates/`. Search for `Point3WithWidth { x:`, `Point3WithWidth { .. ..`, `Into<wit::Point3WithWidth>`, `From<wit::Point3WithWidth>`, and any `bindings::*::Point3WithWidth` references. Return LOCATIONS, ≤ 20 entries." Purpose: bound the binding fan-out before Step 0 begins editing.
- **Step 0 — schema-version constant lookup:** "Find the schema minor-version constant in `crates/slicer-ir/src/`. Return FACT with file:line and the current value." Purpose: pin the bump target.
- **Step 0 — IR roundtrip test location:** "Find existing `Point3WithWidth` serde/JSON roundtrip tests under `crates/slicer-ir/tests/` and `crates/slicer-ir/src/`. Return LOCATIONS." Purpose: decide whether to create a new test file or extend.
- **Step 3 — OrcaSlicer threshold endpoint parity:** "Return the exact `<` vs `<=` convention at the four quartile boundaries in `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` around line 397 and line 535. SNIPPETS, ≤ 30 lines each." Purpose: nail the off-by-one for AC-5.
- **Step 4 — pipeline call-site enumeration:** "List every `DefaultGCodeEmitter::emit_gcode` call in `crates/slicer-host/src/pipeline.rs`. Return LOCATIONS." Purpose: confirm both pipeline arms are touched.
- **Every cargo step:** "Run `<cargo command>`; return FACT pass/fail with ≤ 20 lines of SNIPPET on failure." Purpose: avoid absorbing cargo output.
- **Step 7 — docs/07 closure edit:** "In `docs/07_implementation_status.md`, return LOCATIONS for the TASK-182 line." Purpose: targeted Edit, no browsing.
- **Step 7 — workspace ceremony:** "Run `cargo test --workspace`; return FACT pass/fail with failing test name + assertion + ≤ 20-line SNIPPET on failure." Purpose: close-time gate.

## Data and Contract Notes

- IR contracts touched: `Point3WithWidth` gains an optional field; `#[serde(default)]` preserves backward-compat for older JSON producers (AC-6). Schema minor version bump signals the change to any downstream consumer.
- WIT boundary considerations: `point3-with-width` record adds `overhang-quartile: option<u8>`. WIT `option<u8>` maps to Rust `Option<u8>` directly via `wit-bindgen`; no manual wrapping required. The conversion sites enumerated by Step 0's LOCATIONS dispatch must each propagate the new field — no silent drops, no `Default::default()` shortcuts that erase classifier output.
- Determinism / scheduler constraints: classifier mutates `LayerCollectionIR` in place. It MUST run after layer finalization (the existing `LayerCollectionIR` is fully built) and before `emit_gcode`. No new scheduling phase; piggybacks on the host pipeline before emit.
- The classifier short-circuits when all four `overhang_N_4_speed` keys are exactly `0.0` (the packet 52 default). This preserves AC-2's byte-identical zero-config no-op.

## Locked Assumptions and Invariants

- `Point3WithWidth.x/y/z` are `f32` mm at the time the classifier runs (per `gcode_emit.rs:380` usage). The classifier works in mm; no unit conversion.
- `Point3WithWidth.width` is `f32` mm (consistent with the `width` arg of the OrcaSlicer estimator). The thresholds `[0, 0.25w, 0.5w, 0.75w]` use this `width` field per point — not a global default.
- `overhang_quartile` value space: `None | Some(1) | Some(2) | Some(3) | Some(4)`. `Some(0)` is reserved and treated as an invariant violation (AC-N1).
- `classify_layers` only mutates entries whose role is in `{OuterWall, InnerWall, ThinWall}`. Other roles' points are left with `overhang_quartile = None`.
- First layer (no previous layer) leaves every quartile as `None` regardless of config (AC-4).
- The classifier consumes the previous layer's `OuterWall | InnerWall | ThinWall` polylines, joined into closed-loop polygons using the existing IR's loop convention. Interior holes (inner contours) flip the inside-test sign — the classifier respects loop winding.

## Risks and Tradeoffs

1. **WIT-binding fan-out blast radius.** `Point3WithWidth` is a foundational record. The conversion sites are mechanical but enumeration must be exhaustive — a missed site silently drops `overhang_quartile` on the floor. Mitigation: Step 0 begins with a LOCATIONS dispatch; the implementer audits the list against the resulting build errors from a deliberate `unimplemented!()` placeholder before filling in correct conversions.
2. **Quartile threshold endpoint off-by-one.** OrcaSlicer's `< / <=` convention at quartile boundaries is critical for AC-5. Mitigation: Step 3 dispatches a SNIPPETS read of `ExtrusionProcessor.hpp:397` and `:535`; the implementer mirrors the convention verbatim.
3. **Polygon-inside-test for interior holes.** A naïve line-distance approach would mis-classify points above an interior hole as "supported." Mitigation: Up-front polygon-winding inside-test (already chosen as the selected approach). Trade-off: more code than line-distance, but only marginally — and avoids a guaranteed deviation against Orca.
4. **`LinesDistancer2D` performance.** Linear scan with bbox prefilter is O(N·M) for N points × M segments per layer-transition. Mitigation: profile-only-if-needed; defer BVH. If profiling later shows dominance, a follow-up packet adds the BVH.
5. **Byte-identical zero-config baseline (AC-2).** A subtle bug — e.g., the classifier writing `Some(_)` despite short-circuit — would silently break AC-2 even though `resolve_feedrate` ignores it. Mitigation: the AC-2 test asserts BOTH the G-code bytes AND that every wall point's `overhang_quartile == None` after the pipeline runs.
6. **Schema-version bump coordination.** Any consumer of the IR with a pinned older schema fails roundtrip. Mitigation: `#[serde(default)]` + the AC-6 missing-field branch. The bump is "minor" — additive compatible field.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: Step 0 (WIT field + binding fan-out) and Step 3 (classifier) are both `M`. No step is `L`.
- Highest-risk dispatch: Step 0's binding fan-out LOCATIONS enumeration. Required return format: `LOCATIONS: ≤ 20 entries, 1-line context each`. Reject any reply with code snippets; re-dispatch tighter.

## Open Questions

None blocking activation. Soft items resolved inside implementation:

- The exact identifier of the schema minor-version constant in `crates/slicer-ir/`. Resolved by Step 0's dispatch.
- Whether an existing IR roundtrip test file under `crates/slicer-ir/tests/` should be extended vs. creating a new `point3_overhang_quartile_roundtrip.rs`. Resolved by Step 1's LOCATIONS dispatch.
- The exact `< / <=` endpoint convention from OrcaSlicer. Resolved by Step 3's SNIPPETS dispatch.

If any of these resolutions changes scope, interfaces, or verification strategy, the packet returns to `draft` and the open question is escalated.
