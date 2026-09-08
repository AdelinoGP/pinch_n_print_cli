---
status: implemented
packet: internal-bridge-over-infill
task_ids:
  - ISSUE-82
---

# 233-internal-bridge-over-infill

## Goal

Introduce the internal bridge-over-infill decision at the post-surface/infill seam (`Layer::InfillPostProcess`), constructing anchored bridge polygons with a windowed-mean angle per canonical `PrintObject::bridge_over_infill`, and thread a proper `ExtrusionRole::InternalBridgeInfill` variant through IR/WIT/host/marshal/gcode — bundling the sparse ±90° alternation fix (D11/F7) and canonical `bridging_flow` spacing + decoupled bridge feedrate (F5/F6). This is a NEW decision, not a move: at HEAD no internal-bridge decision exists in the prepass (see AC-N2).

## Problem Statement

Canonical OrcaSlicer runs `PrintObject::bridge_over_infill` inside `prepare_infill()` *after* `process_external_surfaces`/`clip_fill_surfaces`: it generates sparse-infill anchor polylines itself (`Layer::generate_sparse_infill_polylines_for_anchoring`), clusters anchored lines above voids, picks the angle with `determine_bridging_angle` (length-weighted mean over a ±18° sliding window of nearest-anchor orientations — why real prints get non-grid angles like 23.3°), builds polygons with `construct_anchored_polygon` (scan lines every `bridging_flow.scaled_spacing()`, clipped to anchors/walls), emits `stInternalBridge` surfaces and subtracts them from `stInternal`. PnP has **no internal bridge-over-infill decision at HEAD**: `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) contains zero internal-bridge logic (reviewer-verified tree search). The only bridge-labelled material at HEAD comes from `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) stamping mesh-derived candidates wherever the layer cross-section intersects the facet footprint (F1 false sites), claimed from infill roles by `region_partition.rs` precedence `bridge > bottom > top > sparse`. There is no `InternalBridgeInfill` role — the only `InternalBridge` occurrence in Rust code at HEAD is the dead/reserved feedrate mapping `"InternalBridge" => internal_bridge_speed` in `crates/slicer-gcode/src/emit.rs` (nothing emits a `Custom("InternalBridge")` role at HEAD; only the stash does). PnP couples every role's feedrate to `infill_speed / BASE_SPEED(50)` (F6), ignores configured bridge width and lacks `BRIDGE_EXTRA_SPACING` in core `bridging_flow` (F5), and rotates sparse rectilinear +90° on odd layers where canonical keeps `infill_angle` constant (F7, bundled per D11). These are one coherent slice: all are the internal-bridge decision (introduced here), its role identity, its flow/spacing, and its speed/direction behavior in the module.

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
