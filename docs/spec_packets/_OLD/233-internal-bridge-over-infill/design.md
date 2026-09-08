# Design: internal-bridge-over-infill

## Controlling Code Paths

- Primary code path: `"Layer::InfillPostProcess"` stage / `LayerStageCommit::InfillPostProcess(ir)` dispatch arm (`crates/slicer-runtime/src/layer_executor.rs`) — the post-surface/infill seam where canonical runs `PrintObject::bridge_over_infill` inside `prepare_infill()`. This packet INTRODUCES the internal-bridge decision here: at HEAD no such decision exists — `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) contains zero internal-bridge logic, and the only bridge-labelled material at HEAD comes from `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) + `region_partition.rs` precedence. New pure-geometry home: `crates/slicer-core/src/algos/bridge_over_infill.rs`. Module behavior: `run_infill` in `modules/core-modules/rectilinear-infill/src/lib.rs`. Role plumbing: `ExtrusionRole` (`crates/slicer-ir/src/slice_ir.rs`), WIT `extrusion-role` (`crates/slicer-schema/wit/`), feedrate mapping (`crates/slicer-gcode/src/emit.rs`, `crates/slicer-ir/src/feedrate.rs`), spacing (`crates/slicer-core/src/flow.rs`).
- Neighboring tests/fixtures: `slicer-core` flat test files (the new `bridge_over_infill_tdd.rs` is ungated — the ported geometry uses only non-optional deps `clipper2-rust`/`rstar`, so no `--features host-algos`; the `host-algos`-gated `arachne_*`/`algo_*` files are a different surface), `slicer-runtime --test integration|e2e`, `slicer-gcode --test gcode_feedrate_emission_tdd`, module tests in `rectilinear-infill` (`--test rectilinear_infill_tdd`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Enum blast radius: adding `ExtrusionRole::InternalBridgeInfill` breaks every exhaustive `match` on `ExtrusionRole` workspace-wide (IR, WIT bindings, host marshal, macros, SDK, gcode, region partition, report/visual-debug role styling). Step 2 dispatches a `LOCATIONS` worker to enumerate ALL match sites before editing and fixes them in the same step. If `ExtrusionRole` is serialized into committed SliceIR, the same step owns the IR schema-version bump plus every test hard-asserting the old constant.
- Struct-literal churn gate: any touched watched-type (pub struct with ≥5 named fields under `crates/*/src`) test literal needs a `..` rest or `// exhaustive: <reason>` waiver; `cargo xtask check-literals` is a hard gate (docs/21_data_defaults_and_fixtures.md).
- Config keys are snake_case everywhere in Rust and TOML: `dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle` — never kebab-case.
- Angle determinism: equal-cost orientation candidates resolve to the smallest quantized angle per ADR-0061 (reference `docs/adr/0061-deterministic-bridge-orientation-tie-break.md`; never recreate it). Anchor clustering must iterate in a deterministic (spatially sorted) order.
- Region precedence `bridge > bottom > top > sparse` in `crates/slicer-runtime/src/region_partition.rs` is verified canonical-equivalent — do not reorder.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Stash-pop freshness: popping `stash@{0}` in Step 1 flips guest WASM artifacts stale again (guests on disk match HEAD, not the stash's WIT world); `cargo xtask build-guests --check` exit codes arbitrate (0 fresh / 1 stale / 3 missing wasm-tools infra error) before ANY guest-touching test result is attributed to the code.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- D6 angle boundary: angles crossing IR/WIT/module boundaries stay degrees mod 180°; canonical radians (stored as `PI + atan2(dir.y, dir.x)`, CCW-from-X) are converted ONCE at the port boundary. `internal_bridge_angle` is degrees in config, degrees mod 180° internally.

## Code Change Surface

- Selected approach:
  1. **New pure-geometry module** `crates/slicer-core/src/algos/bridge_over_infill.rs`: ports of `determine_bridging_angle` (length-weighted mean over a ±18° sliding window of nearest-anchor orientations; `internal_bridge_angle > 0` overrides) and `construct_anchored_polygon` (scan lines every `bridging_flow.scaled_spacing()`, clipped to anchors/walls), plus anchor clustering above voids. Ported files carry the standard OrcaSlicer porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
  2. **Host introduction at the seam**: the `LayerStageCommit::InfillPostProcess(ir)` arm (grouped `"Layer::Infill" | "Layer::InfillPostProcess"`) gains the internal-bridge pass: take committed sparse-infill polylines as anchors (canonical `Layer::generate_sparse_infill_polylines_for_anchoring` equivalent — reuse the wall-source sharing noted in `region_partition.rs`'s "Shared with the Layer::InfillPostProcess dispatch arm's wall-source" comment), cluster, angle, construct polygons, emit `InternalBridgeInfill` regions, subtract from sparse infill. This is a NEW decision, not a move: at HEAD `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) contains zero internal-bridge logic, so nothing is removed from it — AC-N2 is a post-implementation guard that no internal-bridge decision or contour-band constant appears there. The stash's prepass alternatives (prepass promotion + contour-band expansion + `Custom("InternalBridge")` tag) are things this packet declines to land (discarded in Step 1's triage), not things it deletes from HEAD.
  3. **D7 variant threading** (Step 2): `ExtrusionRole::InternalBridgeInfill` in `slice_ir.rs`; WIT `extrusion-role`; host marshal; gcode `emit.rs` maps the variant → `internal_bridge_speed` (default 37.5, `feedrate.rs`) and label `Internal Bridge`; retire `Custom("InternalBridge")`, the `"InternalBridge"` string arm, and the stash's `is_internal_bridge` flag (AC-N1).
  4. **F5 flow canonicalization**: `bridging_flow` (`crates/slicer-core/src/flow.rs`) gains `thread_diameter = bridge_line_width if set else nozzle_diameter` selection and `bridge_extrusion_spacing(dmr) = dmr + BRIDGE_EXTRA_SPACING (0.05 mm)`; the module's +0.05 mm shim is deleted (AC-N3). `resolve_role_width` already consumes `RoleWidthContext.bridge_line_width` separately — untouched.
  5. **F6 + Q1 (module)**: delete the shared `speed_factor = self.infill_speed / BASE_SPEED` (BASE_SPEED=50.0) coupling; each emitted role's feedrate comes from its own resolved speed (bridge roles → `internal_bridge_speed`/`bridge_speed`; solid roles per the Q1 decision below).
  6. **D11/F7 (module)**: delete the `layer_index.is_multiple_of(2)` +90° odd-layer rotation in `run_infill`; rectilinear direction stays `infill_angle` constant (canonical `FillRectilinear::_layer_angle` ≡ 0; `Fill::_infill_direction` applies `_layer_angle` only when not fixed-angle and not `dont_alternate_fill_direction`).
- Rejected alternatives and reasons (all are stash alternatives this packet declines to land, not things it deletes from HEAD):
  - The stash's prepass promotion: no infill paths exist at `PrePass::ShellClassification` — the F3 root cause; anchors are unavailable there by construction.
  - The stash's contour-band expansion (`INTERNAL_BRIDGE_EXPANSION_MULTIPLIER = 3.0`): shrinks instead of anchoring; measured ~30–35% of canonical extruded length at matched sites (plan §3 F4).
  - Porting the legacy `BridgeDetector::detect_angle` 5°-sweep class: dead code behind the legacy `#else` upstream — never port.
  - The stash's `Custom("InternalBridge")` tag: stringly-typed role can't carry canonical's per-role flow/speed semantics and bypasses exhaustive matching.

## Authoring Decisions (plan §7 open questions)

- **Q1 — solid-role speed coupling: DECIDED — bundle the decoupling into this packet.** Evidence (verified at authoring, `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp`, the `params.role_speed` assignment block in `Layer::make_fill`/`Fill` surface loop): canonical assigns each role its own speed independently — `erBridgeInfill → bridge_speed`, `erInternalBridgeInfill → internal_bridge_speed`, `erInternalInfill → sparse_infill_speed`, `erTopSolidInfill → top_surface_speed`, `erSolidInfill → internal_solid_infill_speed`. There is no shared coupling factor anywhere upstream. Since F6 must already delete the shared `speed_factor` line in `run_infill`, leaving solid roles coupled would preserve a known divergence at the exact line being edited. Scope limit: only roles the rectilinear-infill module emits; each role's speed comes from its own resolved config value; if the module manifest lacks `top_solid_speed`/`internal_solid_speed`-equivalent keys, Step 5 adds them snake_case in the same manifest edit.
- **Q2 — fan handling: DECIDED — defer to the fan-key packet family.** Rationale: ISSUE-82 owns exactly three keys, none fan-related; canonical internal-bridge fan is a separate mechanism (`_INTERNAL_BRIDGE` role fan markers in `GCode.cpp`, `enable_overhang_bridge_fan` machinery) requiring new config keys plus `crates/slicer-gcode/src/serialize.rs` changes; no invariant in I4–I7 covers fan behavior, so bundling would add unverifiable surface. The variant threaded here keeps fan behavior byte-identical to today's `BridgeInfill` handling.

## Files in Scope (read + edit)

More than 3 primary files is inherent to a cross-cutting introduction (host seam + core geometry + role plumbing + module); steps split the surface so no single step edits more than 3 primary files plus dispatch-enumerated fallout sites.

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: prepass guard target (AC-N2); expected change: verified to contain no internal-bridge decision or contour-band constant. At HEAD it already contains zero internal-bridge logic; the only edit is Step 1's triage discarding the stash's `INTERNAL_BRIDGE_EXPANSION_MULTIPLIER` + orientation heuristic hunks if the pop leaves them here.
- `crates/slicer-runtime/src/layer_executor.rs` - role: introduction target (the seam); expected change: `LayerStageCommit::InfillPostProcess` arm runs the internal-bridge pass.
- `crates/slicer-core/src/algos/bridge_over_infill.rs` - role: NEW pure-geometry port home; expected change: created with porting header.
- `crates/slicer-core/src/flow.rs` - role: F5; expected change: `bridging_flow` thread-diameter selection + `BRIDGE_EXTRA_SPACING`.
- `crates/slicer-ir/src/slice_ir.rs` - role: D7; expected change: `InternalBridgeInfill` variant.
- `crates/slicer-schema/wit/` (`extrusion-role` definition file) - role: D7; expected change: matching WIT variant.
- `crates/slicer-gcode/src/emit.rs` - role: D7; expected change: variant → `internal_bridge_speed` mapping + `Internal Bridge` label; string arm retired.
- `modules/core-modules/rectilinear-infill/src/lib.rs` (+ manifest TOML) - role: F6/Q1/D11 + owned keys; expected change: per-role speeds, no odd-layer rotation, anchor/bridge emission wiring, three snake_case keys.

## Read-Only Context

- `docs/specs/bridge-parity-plan.md` - full - controlling plan.
- `docs/adr/0061-deterministic-bridge-orientation-tie-break.md` - full (short) - tie-break rule.
- `crates/slicer-runtime/src/region_partition.rs` - ranged read around the "Shared with the Layer::InfillPostProcess dispatch arm's wall-source" comment and the precedence chain only.
- `crates/slicer-core/src/algos/prepass_slice.rs` (`assemble_bridge_areas`) - ranged read of the stamper only; W-A context, never edited.
- `crates/slicer-ir/src/feedrate.rs` - ranged read around `internal_bridge_speed` (field, default 37.5, config key).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `stash@{0}` full diff - triage file-by-file per the salvage map; never load wholesale
- `crates/slicer-core/src/algos/mesh_analysis.rs` (`compute_bridge_direction_deg`) - packet 235's surface
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: enumerate every exhaustive `match` on `ExtrusionRole` and every construction/marshal site; scope: `crates/`, `modules/core-modules/`; return: `LOCATIONS`; purpose: Step 2 blast radius.
- Question: does committed SliceIR serialize `ExtrusionRole`, and is there an IR schema-version constant; scope: `crates/slicer-ir/`, `crates/slicer-runtime/`; return: `FACT + LOCATIONS`; purpose: Step 2 version bump decision.
- Question: which `./resources/` model yields an internal-bridge-over-sparse site (and its Z); scope: `resources/`, reslice via `pnp_cli slice --module-dir modules/core-modules`; return: `FACT model + Z`; purpose: AC-6 nomination at activation.
- Question: struct-literal sites of watched types touched by Steps 2/3/5; scope: `crates/*/tests`, `modules/core-modules/*/tests`; return: `LOCATIONS`; purpose: pre-baked churn-gate fallout.
- Question: canonical `determine_bridging_angle` / `construct_anchored_polygon` lambda pseudocode; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY` (≤200 words, snippet ≤30 lines); purpose: Step 4 port fidelity — only if the grounded facts in requirements.md prove insufficient.

## Data and Contract Notes

- IR/manifest contracts: `ExtrusionRole` gains `InternalBridgeInfill` (no `InternalBridgeInfill` exists today; variants include `BridgeInfill`, `Custom(String)`); module manifest gains `dont_filter_internal_bridges` (enum: disabled/no-filter/… per canonical `ibfDisabled`/`ibfNofilter`), `enable_extra_bridge_layer` (enum per canonical `eblApplyToAll`/`eblExternalBridgeOnly`/`eblInternalBridgeOnly`), `internal_bridge_angle` (float, > 0 = override).
- WIT boundary: `extrusion-role` under canonical `crates/slicer-schema/wit/` — host `bindgen!` and guest macro both read these files; variant addition stales all guests (rebuild gate).
- Determinism/scheduler constraints: no new stage edges — `Layer::InfillPostProcess` already runs after `Layer::Infill` (grouped dispatch); the pass consumes only same-layer committed IR, no N±1 layer dependency (that question belongs to packet 234). Anchor iteration order must be deterministic (sorted), tie-breaks per ADR-0061.

## Locked Assumptions and Invariants

- **I4 — self-consistent internal angle**: emitted internal-bridge angle equals what the ported windowed mean computes on the same anchor set; never a frozen constant; `internal_bridge_angle > 0` overrides exactly (AC-3).
- **I5 — density**: internal-bridge line count ≈ span ÷ `bridging_flow` spacing, ±1 line (AC-4).
- **I6 — role disjointness**: role-partition polygons stay pairwise disjoint; internal-bridge area is subtracted from sparse infill (AC-5).
- **I7 — feedrate**: bridge moves' feedrate equals the resolved bridge speed regardless of infill speed (AC-2, AC-6).
- Q1/Q2 decisions above are locked for this packet. I1/I2/I3 belong to packets 234/235 and are NOT asserted here.

## Risks and Tradeoffs

- Anchor availability at the seam depends on committed sparse infill in the `InfillPostProcess` IR; if the wall-source sharing noted in `region_partition.rs` does not expose polylines, Step 4 derives anchors from the same sparse-infill geometry the module committed — flagged as the step's first verification.
- Enum blast radius is the largest compile-risk; mitigated by the pre-edit LOCATIONS dispatch and same-step fixes.
- Stash pop may conflict with HEAD drift; salvage-triage in Step 1 absorbs this.
- AC-6's nominated model may show a weak internal-bridge site; the substitution clause in requirements.md keeps the AC runnable.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: Step 4 (seam introduction + geometry ports) — `M`
- Highest-risk dispatch and required return format: the Step 2 `ExtrusionRole` match-site enumeration — `LOCATIONS` (file:line + 1-line context, ≤ 20 entries per crate, explicit statement if more exist).

## Open Questions

- [FWD] to packet 234: whether the unsupported-span test needs a new prepass data dependency on N±1 layers (plan §7 Q3) — re-derive from scheduler docs at 234's authoring; this packet deliberately stays single-layer.
- [FWD] to packet 235: `compute_bridge_direction_deg` remains divergent (hardcodes 0.0 on degenerate input); 235 ports `detect_bridging_direction` and should reuse the D6 degrees-mod-180 conversion helper introduced here.
- [FWD] to the fan-key packet family: internal-bridge fan markers (`_INTERNAL_BRIDGE`, `enable_overhang_bridge_fan`) — deferred per Q2.
- None blocking — packet is implementation-ready once activated.
