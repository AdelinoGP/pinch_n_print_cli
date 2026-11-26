# Design: 296-slicing-mode-prepass

## Controlling Code Paths

- Primary code path: `slice_mesh_ex` (`crates/slicer-core/src/triangle_mesh_slicer.rs`) → `polygons_to_expolygons` (same file, EvenOdd union) ← `execute_prepass_slice_single_layer_impl` (`crates/slicer-core/src/algos/prepass_slice.rs`, region-map `config_for` read + `slice_mesh_ex` call) ← `RegionMapIR::config_for` (`crates/slicer-ir/src/slice_ir.rs`) ← `ResolvedConfig` (`crates/slicer-ir/src/resolved_config.rs`, `declare_resolved_config!` seam).
- Neighboring tests/fixtures: `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs` (slice kernel pins, incl. `slice_closing_radius` round-trip); `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` (prepass slice wiring); `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs` (config default pins); annulus fixtures in the kernel tests (hole-carrying meshes for CloseHoles).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The mode parameterises the existing prepass slice stage; no new module, claim, IR field, WIT accessor, or manifest row (rule 4 trigger test does not fire — in-stage fill-rule parameter, not cross-module algorithm selection).
- `slice_mesh_ex` keeps its signature and behaviour for all existing callers (mesh cross-section, overhang annotation, region mapping, paint paths); the mode-aware entry is additive (`slice_mesh_ex_with_mode` or equivalent), defaulting to the current EvenOdd path.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Strict unknown-value rejection follows the ticket-113 class (canonical enums silently reset under the reader's forward-compatibility rule; this port rejects at resolution with a `TypeMismatch`-family error naming the key — DEV-188(b)).

## Code Change Surface

- Selected approach: new `SlicingMode` enum in `slicer-ir` (`Regular`/`EvenOdd`/`CloseHoles`, `Default = Regular`, `as_canonical_str` + strict `from_str` following the `SupportType` precedent in `crates/slicer-ir/src/slice_ir.rs`); one `plain slicing_mode: SlicingMode` field in `declare_resolved_config!` (`crates/slicer-ir/src/resolved_config.rs`, canonical default, per-object via the existing overlay — no new axis); additive mode-aware slice entry in `triangle_mesh_slicer.rs` (`Regular`/`EvenOdd` → existing `polygons_to_expolygons` EvenOdd path; `CloseHoles` → orient all rings CCW then `boolean_op_tree_64` with `FillRule::Positive`, both variants verified live in `clipper2-rust`); prepass read of `cfg.slicing_mode` alongside `cfg.slice_closing_radius` in `execute_prepass_slice_single_layer_impl`, applied before the closing round-trip.
- Exact functions, traits, manifests, tests, and fixtures: `SlicingMode` (new, `crates/slicer-ir/src/slice_ir.rs`); `ResolvedConfig::slicing_mode` (new field, `crates/slicer-ir/src/resolved_config.rs`); `slice_mesh_ex_with_mode` (new, `crates/slicer-core/src/triangle_mesh_slicer.rs`) + `polygons_to_expolygons` CloseHoles arm (same file); `execute_prepass_slice_single_layer_impl` mode read (same prepass file); tests `crates/slicer-ir/tests/resolved_config_slicing_mode_tdd.rs` (new) + `crates/slicer-core/tests/slicing_mode_fill_rule_tdd.rs` (new) + `slicing_mode_*` cases in `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` (existing); docs `docs/DEVIATION_LOG.md` (DEV-188) + 04/05 annotations.
- Rejected alternatives and reasons: changing `slice_mesh_ex` signature in place (breaks five callers for one consumer — additive entry keeps the blast radius to the prepass site); declaring the key on `layer-planner-default.toml` (ticket-34 shape — the module never sees the slice union, so the declaration would be dead on the production path); post-hoc hole-dropping instead of Positive union (observably equal only when no island nests inside a hole — Positive matches canonical nesting exactly, so borrow it).

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` - role: `SlicingMode` enum home (`SupportType` precedent); expected change: add enum + round-trip impls.
- `crates/slicer-ir/src/resolved_config.rs` - role: config surface (`declare_resolved_config!` + overlay); expected change: add one `plain` field + Default/PartialEq/Hash arms via macro.
- `crates/slicer-core/src/triangle_mesh_slicer.rs` - role: fill-rule kernel; expected change: additive mode entry + CloseHoles Positive arm.
- `crates/slicer-core/src/algos/prepass_slice.rs` - role: prepass wiring; expected change: read `cfg.slicing_mode`, call mode entry.
- `crates/slicer-ir/tests/resolved_config_slicing_mode_tdd.rs` + `crates/slicer-core/tests/slicing_mode_fill_rule_tdd.rs` (new) - role: AC-1/AC-N1 + AC-2/AC-3 proof; expected change: create with the AC-named tests. (Five files in scope justified: two are new test binaries owned by the ACs; implementation edits are four.)

## Read-Only Context

- `crates/slicer-core/src/triangle_mesh_slicer.rs` - lines `778-834` only - purpose: `polygons_to_expolygons` EvenOdd union shape to extend.
- `crates/slicer-core/src/algos/prepass_slice.rs` - lines `995-1045` only - purpose: region-map `config_for` read + `slice_mesh_ex` call site to parameterise.
- `crates/slicer-ir/src/slice_ir.rs` - lines `2067-2160` only - purpose: `SupportType` enum + `as_canonical_str`/`from_str` precedent to mirror.
- `crates/slicer-gcode/src/serialize.rs` - lines `525-535` only - purpose: `ORCA_CONFIG_PADDING` shape for the AC-N2 honest-absence pin.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `modules/core-modules/layer-planner-default/*` - no slicing behaviour lives here; out of scope by owner correction
- `docs/spec_packets/295-print-sequence-tool-ordering/*` - neighbouring packet; read only via SUMMARY if needed, never edit
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: does `clipper2-rust` expose `FillRule::Positive` for the union call; scope: `clipper2-rust core.rs`; return: `FACT`; purpose: Step 2 kernel authorship (already verified at packet authoring — re-confirm at implementation).
- Question: which existing `algo_prepass_slice_tdd.rs` harness builds a `SliceIR` from an annulus mesh with a caller-supplied region-map config; scope: `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`; return: `LOCATIONS`; purpose: Step 3 wiring test placement.
- Question: `declare_resolved_config!` plain-enum field shape (`SupportType` row) and its PartialEq/Hash arm pattern; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS`; purpose: Step 1 config field authorship.

## Data and Contract Notes

- IR/manifest contracts: no IR field, no manifest row; the mode travels inside the existing `ResolvedConfig` → `RegionMapIR::config_for` channel (same as `slice_closing_radius`).
- WIT boundary: untouched (slicer-core internal; no guest surface).
- Determinism/scheduler constraints: slice output is deterministic in the mode; `regular` default keeps every existing print byte-identical (AC-2 pins it); no scheduler ordering edge.

## Locked Assumptions and Invariants

- `regular` and `even_odd` are observably identical on valid manifold meshes (the `polygons_to_expolygons` EvenOdd doc comment); the packet does not build a separate NonZero path and records this as DEV-188(a).
- CloseHoles fills holes to the outer contour (islands inside holes stay solid — canonical Positive nesting); the 1% area tolerance in AC-3 covers tessellation, not semantics.
- The closing-radius round-trip runs after the mode union, unchanged (mode does not move the `slice_closing_radius` gate).

## Risks and Tradeoffs

- Positive-union nesting on pathological meshes (self-overlapping loops from invalid meshes): EvenOdd and Positive diverge there by design — out of scope per the kernel doc comment; AC-2 pins valid meshes only.
- Per-object overlay coverage: the field rides the macro-generated overlay (ticket-126 shape), so no allowlist can drop it — Step 1 asserts an explicit per-object override reaches the composed config.
- `algo_prepass_slice_tdd.rs` setup weight: if no annulus-capable harness exists, Step 3 authors a minimal one in the same file rather than inventing a new binary.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 kernel + tests)
- Highest-risk dispatch and required return format: prepass harness LOCATIONS (≤20 entries, one context line each)

## Open Questions

None.
