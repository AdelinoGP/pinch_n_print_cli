# Design: 297-conical-overhang-slice-prepass

## Controlling Code Paths

- Primary code path: `PrePass::Slice` commit → NEW `commit_conical_overhang_builtin` (`crates/slicer-runtime/src/builtins/conical_overhang_producer.rs`, new) → `replace_slice_ir` (`crates/slicer-runtime/src/blackboard.rs`) → `PrePass::OverhangAnnotation` (`commit_overhang_annotation_builtin` in `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`) → shell classification → support analysis. Pure geometry in `slicer_core::algos::conical_overhang::apply_conical_overhang` (new module beside `crates/slicer-core/src/algos/overhang_annotation.rs`).
- Neighboring tests/fixtures: `crates/slicer-core/tests/overhang_annotation_ramp_tdd.rs` (ramp-fixture pattern to reuse for AC-3); `crates/slicer-core/tests/overhang_annotation_no_overhang_case.rs` (identity-assertion pattern for AC-2); `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs` (stage-order + guard pattern for AC-6/AC-N1); `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs` (defaults-assertion pattern for AC-1).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Host-only change surface: no WIT edit, no guest manifest row, no `SliceIR` schema change (polygons mutate in place, no new field), so no schema version bumps.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- No range validation on the angle: canonical min/max are GUI hints (map Notes, ticket 113). The kernel computes `tan(angle)` for any finite `f32` and early-returns only on `== 90.0`, exactly like `PrintObject::apply_conical_overhang`. Non-finite inputs are rejected at the producer boundary (mapped to the default) rather than fed to `tan`.
- Bool CONFIG_BLOCK spelling (word-form vs `1`/`0`) is the known ticket-112 condition and rides ticket 132; this packet performs no `serialize.rs` or padding-table edit.
- Determinism: the sweep is per-layer independent reads of `(current, upper)` pairs plus a deterministic union; layer order is `global_layer_index` ascending, object iteration follows the sibling's mesh order. No RNG, no wall-clock, no rayon nondeterminism beyond what `polygon_ops` already guarantees for the sibling stage.

## Code Change Surface

- Selected approach: mirror the sibling `OverhangAnnotation` staging shape exactly — pure kernel in `slicer-core`, thin `Blackboard` bridge in `slicer-runtime/builtins`, one `run_builtin_stage` registration in `prepass.rs`. Config arrives via the sibling's raw-source convention (`HashMap<ConfigKey, ConfigValue>` lookups with canonical-default fallback), backed by three real `ResolvedConfig` fields so CLI, overlay, and `to_config_map` all carry the keys (no severed plumbing: AC-6 proves an explicit raw-source value changes output).
- Exact functions, traits, manifests, tests, and fixtures:
  - NEW `crates/slicer-core/src/algos/conical_overhang.rs` — `pub fn apply_conical_overhang(layers: &[(u32, Vec<ExPolygon>)], params: &ConicalOverhangParams) -> Vec<(u32, Vec<ExPolygon>)>` plus `pub struct ConicalOverhangParams { enabled: bool, angle_deg: f32, hole_size_mm2: f32, layer_h_mm: f32 }`; declared in `crates/slicer-core/src/algos/mod.rs`.
  - NEW `crates/slicer-runtime/src/builtins/conical_overhang_producer.rs` — `pub fn commit_conical_overhang_builtin(blackboard, raw_config_source) -> Result<(), ConicalOverhangBuiltinError>` with `MissingSliceIr` / `MissingLayerPlan` / `Blackboard` variants; declared in `crates/slicer-runtime/src/builtins/mod.rs`.
  - EDIT `crates/slicer-runtime/src/prepass.rs` — one `run_builtin_stage` call between the `Slice` and `OverhangAnnotation` registrations (guard `slice_ir().is_some()`).
  - EDIT `crates/slicer-ir/src/resolved_config.rs` — three `cli` fields + three `to_config_map` emissions (macro arms only).
  - NEW `crates/slicer-ir/tests/resolved_config_conical_overhang_tdd.rs` (auto-discovered binary).
  - NEW `crates/slicer-core/tests/conical_overhang_tdd.rs` (opens with `#![cfg(feature = "host-algos")]`, auto-discovered binary per the 296 precedent — no `Cargo.toml` entry; AC commands pass `--features host-algos`).
  - NEW `crates/slicer-runtime/tests/executor/prepass_conical_overhang_stage_order_tdd.rs` + one `mod` line in `crates/slicer-runtime/tests/executor/main.rs`.
  - EDIT `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (P71 packet linkage) and `05-asset-packet-list.md` (P71 → 297).
- Rejected alternatives and reasons:
  - WASM module with claim holders (rule-4 shape): rejected — the alternatives here are scalar parameters of one geometric pass, not competing algorithms in separate modules; the claim seam would add dispatch with no selection to resolve.
  - Mutating mesh triangles instead of `SliceIR`: rejected — canonical operates post-slicing on layer polygons, and re-slicing a grown mesh would recurse; the `SliceIR` level is the faithful seam and matches the `PaintSegmentation` mutation precedent.
  - Reading `ResolvedConfig` directly in the producer instead of the raw source: rejected — every sibling builtin takes the raw source; a second config path invites drift between the CLI value and the stage value.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/conical_overhang.rs` (NEW) - role: pure kernel; expected change: full pass per §Code Change Surface.
- `crates/slicer-runtime/src/builtins/conical_overhang_producer.rs` (NEW) - role: Blackboard bridge + key resolution; expected change: sibling-mirroring commit fn.
- `crates/slicer-ir/src/resolved_config.rs` - role: typed config source; expected change: three `cli` fields + emissions (macro arms + drift-guard extension).
- `crates/slicer-runtime/src/prepass.rs` - role: stage ordering; expected change: one `run_builtin_stage` registration.
- `crates/slicer-core/src/algos/mod.rs`, `crates/slicer-runtime/src/builtins/mod.rs`, `crates/slicer-runtime/tests/executor/main.rs`, tier-table + packet-list docs - role: wiring/ledger; expected change: one line each (justified extras: single-line registrations, split would cost more than it saves).

## Read-Only Context

- `crates/slicer-core/src/algos/overhang_annotation.rs` - lines 1-120 only - purpose: kernel conventions (footprint input shape, `polygon_ops` reuse, arc tolerance).
- `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` - lines 86-197 only - purpose: producer conventions (raw-source read, per-object loop, FRU replace).
- `crates/slicer-runtime/src/prepass.rs` - lines 820-960 only - purpose: `Slice`/`OverhangAnnotation` registration text to bracket the new call.
- `crates/slicer-runtime/src/blackboard.rs` - lines 170-330 only - purpose: `replace_slice_ir` contract + error variants.
- `crates/slicer-core/src/polygon_ops.rs` - lines 370-560 only - purpose: `offset` / `union_ex` / `difference_ex` / `intersection_ex` signatures.
- `crates/slicer-ir/src/resolved_config.rs` - lines 76-110 (emission arms) + 1830-1850 (field pattern) + 2035-2060 (bool field pattern) + 2890-2960 (drift guard) only - purpose: exact macro spelling to extend.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` + `ORCA_CONFIG_PADDING` - ticket-132 owned; no spot fix
- `modules/core-modules/*` - no module changes; do not browse for owners (ticket-27 hazard already cleared: owner is the host prepass)
- `docs/15_config_keys_reference.md` - generated; never hand-edit
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: confirm the canonical hole-cut operand order in `PrintObject::apply_conical_overhang` (cut small covered holes from upper, then offset, then union into current — or other order) + the per-region skip rule when the upper region disables the key; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`; return: `SUMMARY`; purpose: Step 1 binding decision.
- Question: which `LayerPlanIR`/slice-adjacent field carries the per-layer height the port should use for `tan(angle) * layer_h` (else global `layer_height` fallback); scope: `crates/slicer-ir/src` + `crates/slicer-runtime/src/prepass.rs` Slice commit; return: `LOCATIONS`; purpose: Step 1 binding decision.
- Question: does `OrcaSlicerDocumented/tests/fff_print/` cover conical overhang with portable assertions; scope: that dir listing + grep for `make_overhang`; return: `FACT`; purpose: Step 2 (port with attribution header iff present).

## Data and Contract Notes

- IR/manifest contracts: none changed. `SliceIR` polygons mutate in place; no new field, no schema bump, no manifest row (host keys live in `ResolvedConfig`, and `ConfigView::from_declared` whitelisting constrains modules only — the host producer reads the raw source directly like its sibling).
- WIT boundary: untouched. No guest sees these keys; no `.wit` edit, no version change.
- Determinism/scheduler constraints: stage runs inside the existing prepass instrument guard via `run_builtin_stage`; idempotent in the sibling sense (re-run recomputes the same deterministic result from committed slices); must precede Tier 2 layer-slot writes (`replace_slice_ir` debug-asserts it).

## Locked Assumptions and Invariants

- Defaults locked to canonical: `false` / `55.0` / `0.0` (AC-1 pins them; any future change is a recorded divergence).
- Disabled (or angle `== 90.0`) output is byte-identical to input (AC-2/AC-4 pin it; downstream baselines must not move under this packet).
- Per-object isolation: an object's mutated layers derive only from that object's footprints (AC-6 pins it).
- Hole-area comparison happens in mm² at the unit boundary (`shoelace / (UNITS_PER_MM * UNITS_PER_MM)`); there is no public `area()` helper to reuse (verified at authoring: `signed_area`, `polygon_area_units2`, `contour_area_abs` are all module-private).
- Step 1 binds the layer-height source and the hole-cut operand order; Steps 2–4 consume them without re-derivation.

## Risks and Tradeoffs

- `offset` on pathological upper footprints (self-intersections from triangulation) can grow spikes; mitigated by reusing the sibling's battle-tested `polygon_ops` path plus the ramp/holed-plate fixtures, and by the default-off gate (no existing user moves).
- Large angles near 90° produce large `tan` offsets; canonical accepts this (no validation) and so does this packet — the `== 90.0` early-return is the only guard. Documented, not diverged.
- Unioning grown upper geometry into the current layer can merge previously disjoint islands; downstream consumers (support, infill) already handle multi-island footprints, and AC-3 asserts containment rather than island count to avoid over-pinning.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 kernel + tests)
- Highest-risk dispatch and required return format: canonical hole-cut operand order — `SUMMARY` ≤ 200 words (wrong order fails AC-5 and mis-ports canonical behaviour).

## Open Questions

- None. Step 1's two binding decisions are implementer-resolvable by delegation (`[FWD]`-class, not activation blockers) with safe fallbacks stated in the plan.
