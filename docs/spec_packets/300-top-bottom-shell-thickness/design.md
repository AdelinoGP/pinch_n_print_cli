# Design: 300-top-bottom-shell-thickness

## Controlling Code Paths

- Primary code path: `compute_region_updates` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`, lines 351–520) gains the thickness arms — the top seed walk `(1..k_top.min(...))` and the bottom seed walk `(1..k_bot.min(...))` each extend past the count cap while the accumulated print-z distance stays below the configured thickness; `resolve_shell_counts` (lines 1081–1106) gains the two thickness reads through the same first-timeline-entry `region_map.config_for` pattern and threads them into the walks.
- Config path: `resolve_shell_counts`' `config_for(&key)` read pattern is the per-region config entry for the thickness pair — the same seam packet 299's P73 stages use, so no new plumbing is invented.
- Neighboring tests/fixtures: `#[cfg(test)] mod tests` in `slice_postprocess_prepass.rs` (the `rect`/`slice_with` fixture helpers, lines 1137–1160); the `bottom_shadow_does_not_propagate_past_k_bot_via_a_later_seed` test pins the count-cap walk shape the arms extend.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Thickness-vs-height comparison is mm-vs-mm: `Snapshot[z]` slice heights are `f32` mm and the thickness fields are `f32` mm — no unit conversion enters the arm itself (the `EPSILON` margin is an mm-domain constant, not a coordinate-unit constant).
- Prepass ordering: the thickness arms ride the existing Pass-2 shadow walks inside `compute_region_updates` — no new stage, no new call site in `commit_shell_classification_builtin`, no ordering change. The `0`-thickness fast path skips the distance check entirely (count floor only, pre-packet shape).
- ADR-0062/0063 locked-path conformance: thickness growth unions its shadow into untagged fill only; locked paths are neither clipped nor merged (AC-N1). The five-way partition's pairwise-disjoint invariant (`SlicedRegion.sparse_infill_area` doc, `crates/slicer-ir/src/slice_ir.rs`) must survive — the arms extend the existing union-into-fill, never a second subtraction.
- Rule 4 does not fire: the two arms branch inside one prepass seam's existing walks — no cross-module algorithm selection, no claim holders (`seam_position` precedent, map Notes Q8).
- `ConfigView::from_declared` whitelist is not engaged: both keys are host-prepass inputs read through `region_map.config_for` (the `resolve_shell_counts` pattern), so no module manifest declares them; no manifest changes anywhere.
- Bounds: no numeric range rejection anywhere (ticket-113 rule — canonical min/max are GUI hints); a negative thickness saturates to disabled (`0` behaviour), never rejects. This packet has no rejection criterion at all — AC-N1 is a lock-bypass invariant, not a validation gate.
- Draft-299 merge: packet 299 changes the same file's stage list and the same `resolve_shell_counts` call signature. This packet's resolver edit merges with — never duplicates — 299's signature at activation time; whichever lands second rebases onto the first. No shared helper, no cross-packet dep.

## Code Change Surface

- Selected approach: two `declare_resolved_config!` rows (canonical defaults) → `resolve_shell_counts` returns the count pair plus the thickness pair → the two Pass-2 seed walks extend past the count cap while the print-z distance stays below thickness → three unit tests (beats-count, defaults-identity, lock-bypass) → DEV-191 + docs regen. No IR field, no WIT line, no emitter arm — the thickness rides the existing `top_solid_fill` / `bottom_solid_fill` buckets the walks already write.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs` — 2 macro rows (`top_shell_thickness` f32 `0.6` via `extract_float`, `bottom_shell_thickness` f32 `0.0` via `extract_float`).
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — `resolve_shell_counts` thickness reads + top/bottom walk extensions + tests (`shell_thickness_extends_projection_past_count`, `shell_thickness_defaults_are_identity`, `shell_thickness_bypasses_locked_paths`).
  - `docs/DEVIATION_LOG.md` — DEV-191; `docs/15_config_keys_reference.md` — regen via `cargo xtask gen-config-docs`.
- Rejected alternatives and reasons: (a) declare the keys in infill-module manifests and read them module-side — rejected because the decision points are cross-layer (a layer's shell state depends on neighbours), which the per-layer module dispatch cannot see; the prepass is the architecture's cross-layer seam (rule 4: new decision points go where the architecture puts them). (b) Fold into draft packet 299 — rejected: 299's preflight is closed and its four stages own different decisions (vertical shells, extra-solid insertion, sparse combination); the thickness floor is the P74 packet boundary the queue already fixed, and 299's scope statement excludes it. (c) Build the `PrintObject::infill` scatter as a second thickness site — rejected: the prepass projection already carries the thickness into the solid-fill buckets every downstream consumer reads; a second scatter would double-cover the same layers for no new behaviour.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs` - role: single source of truth for the two defaults; expected change: two macro rows (+ doc comments; the macro emits `Default`, `apply_cli_key`, `to_config_map`, and overlay arms per ticket 126).
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: prepass host; the resolver reads + walk extensions + tests land here; expected change: thickness pair in `resolve_shell_counts` + two walk arms in `compute_region_updates` + three unit tests.
- `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md` - role: DEV-191 + regen.
- Justification for ≤3 primary files: the two-way split (config → prepass) is the architecture's own seam order; no IR/WIT/emitter change exists to split further.

## Read-Only Context

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `351-520` only - purpose: `compute_region_updates` Pass-1 + Pass-2 walk shapes the arms extend.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1081-1106` only - purpose: `resolve_shell_counts`' `config_for` pattern the thickness reads mirror.
- `crates/slicer-ir/src/resolved_config.rs` - lines `1892-1897` only - purpose: neighbouring `top_shell_layers` / `bottom_shell_layers` declaration rows the thickness rows sit beside.
- `crates/slicer-ir/src/resolved_config.rs` - lines `1833-1835` only - purpose: `extract_float` row shape for plain mm floats.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Unrelated crates - delegate symbol lookups; do not browse
- `crates/slicer-wasm-host/test-guests/**` - no test-guest edit needed; no WIT change, no guest surface touched
- Draft packet `299-*/` - read Goal line only via dispatch if needed; never open its design file
- `modules/**` - no module manifest or emitter changes; the thickness never reaches module dispatch

## Expected Sub-Agent Dispatches

- Question: exact `discover_horizontal_shells` top/bottom loop arms (count floor + `||` thickness + `EPSILON` + `combine_holes` follow-ups); scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY`; purpose: Step 2 arm borrow.
- Question: does `cargo check --workspace --all-targets` pass; scope: workspace; return: `FACT`; purpose: Step gates.
- Question: neighbouring `extract_float` macro-row shape for plain mm floats; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS`; purpose: Step 1 row authoring (already verified at authoring — re-dispatch only if the macro moved).

## Data and Contract Notes

- IR/manifest contracts: none — no new IR field, no manifest row, no WIT line. The thickness rides the existing `top_solid_fill` / `bottom_solid_fill` buckets and the existing `top_shell_index` / `bottom_shell_index` stamps.
- WIT boundary: untouched (no new accessor — the views already expose the buckets and stamps the arms extend).
- Determinism/scheduler constraints: the arms extend the existing sequential Pass-2 walks inside `compute_region_updates` (the timeline loop already runs sequentially per the `commit_shell_classification_builtin` comment); no parallel determinism change.

## Locked Assumptions and Invariants

- Canonical defaults are locked: top `0.6` / bottom `0.0`. Defaults are identity on ordinary prints (AC-3): top `0.6` extends the walk only where 3 count-layers cover less than 0.6 mm, and bottom `0.0` disables the extension — so the default slice matches the pre-packet baseline.
- DEV-191 clauses (minted at authoring, re-derive `max(DEV-*)` before writing): (a) the `PrintObject::infill` scatter is not borrowed as a second site — the prepass projection already carries the thickness downstream, so re-scattering would double-cover; (b) the spiral-mode bottom-layer gate (`LayerRegion::make_perimeters`, `PrintObjectSlice.cpp`) is not borrowed — `spiral_mode` is unimplemented queue scope; (c) canonical's `Print.cpp` reslice-invalidation entries ride ticket 124.
- The port's uniform planner means the print-z distance accumulates over uniform layer heights only; canonical's variable-profile distance arithmetic is not borrowed (ticket-75/144 envelope, named non-borrow).

## Risks and Tradeoffs

- The top `0.6` default is live (not disabled): on thin-layer prints where 3 layers cover less than 0.6 mm, the default slice newly grows solid layers versus the pre-packet baseline. AC-3 pins the flat-geometry fixture identical; any sloping-fixture churn lands as test-fixture updates with measured justification (map test discipline).
- The walk extension reads neighbour `z` from the immutable snapshot — the same snapshot the count-cap walks already read — so no new borrow shape is introduced; the likeliest silent defect is a print-z vs bottom-z arm swap (top walks compare `print_z`, bottom walks compare `bottom_z`), pinned by AC-2's separate top/bottom assertions.
- Draft-299 merge hazard: both packets edit `resolve_shell_counts` and the same prepass file. The implementer rebases onto whichever lands first; the AC commands name only this packet's tests, so a 299-first landing cannot silently satisfy them.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — two walk arms + three tests in one file)
- Highest-risk dispatch and required return format: horizontal-shell loop-arm borrow — `SUMMARY` (≤200 words) from `PrintObject.cpp`; reject any reply pasting the whole function.

## Open Questions

None. `[FWD]` none pending; `[BLOCK]` none.
