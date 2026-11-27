# Design: 302-bridge-angle-counterbore-classic-perimeters

## Controlling Code Paths

- Primary code path: `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`, lines 200–260) gains two arms after its existing bridge gates. First, the override: after `update_external_bridge_orientation(region, lower_layer_slices)` derives the detected direction, a `resolve_bridge_angle` sibling of `resolve_shell_counts` (lines 1169–1194) reads the float through the same first-timeline-entry `region_map.config_for` pattern, and a `> 0` value overwrites `region.bridge_orientation_deg` verbatim (0 keeps the detected value — the pre-packet shape). Second, the stage: a `author_counterbore_bridge_spans` pass after `gate_internal_bridge_sites` resolves the enum per timeline through the same pattern and authors hole-bearing unsupported spans into `region.bridge_areas` — whole uncovered spans in `filled` mode, rim spans only in `partial` mode.
- Config path: both `config_for(&key)` reads are per-region config entries for the flags — the same seam packet 301's `resolve_interface_shells` uses, so no new plumbing is invented. The macro emits `Default`, `apply_cli_key`, `to_config_map`, `host_config_keys`, and the ticket-126 overlay arms for both rows; the enum is held as a `String` (`flat_bridge_closing_join` precedent — no new typed enum, canonical spellings round-trip through `to_config_map`).
- Neighbouring tests/fixtures: `#[cfg(test)] mod tests` in `slice_postprocess_prepass.rs` (the `rect`/`slice_with` fixture helpers, lines 1225–1252); the `prepass_slice_and_shell_tdd` executor binary (the `cuboid_mesh` / `make_plan` / `make_region_map` fixture helpers) for the hole-bearing executor tests — the stepped-hole fixture needs two layers with distinct footprints, which the direct-call unit shape cannot build with its single-`(object_id, region_id)` helpers, so AC-3/AC-4 live there. The AC commands name only this packet's tests, so no other packet's landing can silently satisfy them.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The override is an *overwrite*, not a second detection: no new stage, no new orientation math, no ordering change. It runs after the existing `update_external_bridge_orientation` call and reuses its output as the `0` arm — `detect_bridging_direction_deg` is consumed, never re-derived.
- The counterbore stage authors *survivor* spans: unsupported is computed against the committed lower layer (the same `object_layers` + `lower_layer_polygons` maps the existing bridge loop builds), holes come from the region's own polys, and already-bridged spans are subtracted so the stage never duplicates `gate_bridge_areas_by_unsupported_span`'s output. Authored spans get a detected orientation from `detect_bridging_direction_deg` against the raw lower contours (the packet-235 seam) — they are gate survivors by construction and need no second gate pass.
- `ConfigView::from_declared` whitelist is not engaged: both flags are host-prepass inputs read through `region_map.config_for` (the `resolve_shell_counts` pattern), so no module manifest declares them; no manifest changes anywhere.
- String-enum spelling: `none` / `partiallybridge` / `sacrificiallayer` — the exact canonical `enum_values` (`PrintConfig.cpp`), not the labels (`None` / `Partially bridged` / `Sacrificial layer`). Unknown spellings fall back to `none` (the `flat_bridge_closing_join` precedent — unknown values fall back); the typed `extract_string` `TypeMismatch` (non-string config values) is never a saturate. This packet has no numeric-range rejection (ticket-113 rule); the float's `> 0` gate is behaviour selection, not validation.
- CONFIG_BLOCK: neither key has a padding twin (honest absence, AC-6 pins it); the `to_config_map` arms emit the live values as `Float` / `String` so the CONFIG_BLOCK carries them (canonical value spellings ride 132 — the enum's lowercase spellings need no conversion, the float emits canonically).
- Rule 4 does not fire: the override branches inside one prepass seam's existing write and the stage authors buckets the existing bridge-fill holders already consume — no cross-module algorithm selection, no claim holders (`seam_position` precedent, map Notes Q8).
- ADR-0062/0063 locked-path conformance: the override reads orientation only (no geometry); the counterbore stage neither clips nor links locked paths — the self-clipping obligation stays with the lock emitter (AC-N2's hole-free twin plus the partition's pairwise-disjoint invariant, `SlicedRegion.sparse_infill_area` doc, `crates/slicer-ir/src/slice_ir.rs`).
- Draft-299 merge: packet 299 changes the same file's stage list and the same `resolve_shell_counts` call neighbourhood. Both resolvers merge with — never duplicate — 299's signature at activation time; whichever lands second rebases onto the first. No shared helper, no cross-packet dep.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Doc-15 honest mechanics: the generated host tables source from `docs/config/host-keys.toml`, which the `host_keys_doc_lock_tdd` test ties to live defaults — the float row needs the lock's `resolved_num` arm and the enum row needs the `resolved_str` arm extended alongside the TOML rows (Step 1; packet 301's regen-only precedent is corrected here, or AC-6's `--check` passes while the keys stay absent from the table).
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: two `declare_resolved_config!` rows (float `0.0` via `extract_float`, string `"none"` via `extract_string`) + two `to_config_map` arms → `resolve_bridge_angle` + post-detection overwrite → `author_counterbore_bridge_spans` stage after `gate_internal_bridge_sites` → six lib tests (override, three counterbore, two negatives) + two executor tests (hole-interior whole-span, hole-interior rim-only) → lock-test arms + host-keys rows → DEV-193 + docs regen. No IR field, no WIT line, no emitter arm — the override changes the existing `bridge_orientation_deg` bucket and the stage changes the existing `bridge_areas` bucket both perimeter roles already consume.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs` — 2 macro rows (`bridge_angle` float `0.0` via `extract_float`; `counterbore_hole_bridging` string `"none"` via `extract_string`, `wire_type: Some("enum")` + `values: &["none", "partiallybridge", "sacrificiallayer"]`) + 2 `to_config_map` arms (`ConfigValue::Float` / `ConfigValue::String`).
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — `resolve_bridge_angle` + post-`update_external_bridge_orientation` overwrite + `author_counterbore_bridge_spans` + lib tests (`bridge_angle_overrides_detected_orientation`, `bridge_angle_zero_keeps_detected_orientation`, `counterbore_defaults_leave_regions_untouched`, `counterbore_unknown_spelling_falls_back_to_none`, `counterbore_hole_free_spans_unaffected`).
  - `crates/slicer-runtime/tests/executor/prepass_slice_and_shell_tdd.rs` — stepped-hole executor tests (`counterbore_filled_authors_hole_interior_as_bridge`, `counterbore_partial_leaves_hole_interior_unbridged`).
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` — `resolved_num` arm for `bridge_angle`, `resolved_str` arm for `counterbore_hole_bridging`.
  - `docs/config/host-keys.toml` — 2 `[resolved_config]` rows (`bridge_angle = { default = 0.0, range = "[0, 180]" }`, `counterbore_hole_bridging = { default = "none" }`).
  - `docs/DEVIATION_LOG.md` — DEV-193; `docs/15_config_keys_reference.md` — regen via `cargo xtask gen-config-docs`.
- Rejected alternatives and reasons: (a) declare the keys in perimeter-module manifests and read them module-side — rejected because the decisions are cross-layer (a layer's bridge state depends on the committed layer below), which the per-layer module dispatch cannot see; the prepass is the architecture's cross-layer seam (rule 4: new decision points go where the architecture puts them). (b) Port canonical's `BridgeDetector` sweep as the counterbore bridgeability test — rejected: the port's committed-layer span gate plus the raw-contour span test already decide bridgeability at the same seam; a second detector would double-cover the same spans for no new behaviour. (c) Fold into the internal-bridge construction arm (`layer_executor.rs`'s `angle_override` + `determine_bridging_angle`) — rejected: that arm serves internal bridges over infill (ticket-82 scope); external orientation and stepped holes are different buckets at a different seam.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs` - role: single source of truth for both defaults + CLI bindings + map arms; expected change: two macro rows + two `to_config_map` arms (the macro emits `Default`, `apply_cli_key`, `host_config_keys`, and the ticket-126 overlay arms).
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: prepass host; both resolvers + the overwrite + the stage + lib tests land here; expected change: `resolve_bridge_angle` + post-detection overwrite + `author_counterbore_bridge_spans` + five lib tests.
- `crates/slicer-runtime/tests/executor/prepass_slice_and_shell_tdd.rs` - role: hole-bearing executor home (stepped-hole fixtures need two layers with distinct footprints); expected change: two stepped-hole tests reusing the `cuboid_mesh` / `make_plan` / `make_region_map` helpers.
- `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`, `docs/config/host-keys.toml`, `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md` - role: lock arms + TOML rows + DEV-193 + regen.
- Justification for the fourth group: the lock pair is one mechanical arm each (the doc-15 regen reads the TOML rows); the deviation row + regen are the packet's ledger close. No IR/WIT/emitter change exists to split further.

## Read-Only Context

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `200-260` only - purpose: the bridge-gate neighbourhood (`object_layers`/`lower_layer_polygons` maps, `gate_bridge_areas_by_unsupported_span` + `update_external_bridge_orientation` call order) the overwrite and stage extend.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1169-1194` only - purpose: `resolve_shell_counts`' `config_for` pattern both sibling resolvers mirror.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1225-1252` only - purpose: `rect` / `slice_with` fixture helpers the lib tests reuse.
- `crates/slicer-core/src/algos/prepass_slice.rs` - lines `604-613` only - purpose: `update_external_bridge_orientation`'s gated-geometry + raw-contour contract the overwrite rides after.
- `crates/slicer-ir/src/resolved_config.rs` - lines `2035-2036` only - purpose: neighbouring `bridge_no_support` bool row the new rows sit beside.
- `crates/slicer-ir/src/resolved_config.rs` - lines `554-564` only - purpose: `extract_float` accepted spellings (`Float` + `Int`).
- `crates/slicer-ir/src/resolved_config.rs` - lines `682-693` only - purpose: `extract_string` accepted spellings (`String` only) — the unknown-spelling fallback lives in the stage, not the extractor.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Unrelated crates - delegate symbol lookups; do not browse
- `crates/slicer-wasm-host/test-guests/**` - no test-guest edit needed; no WIT change, no guest surface touched
- Draft packet `299-*/` - read Goal line only via dispatch if needed; never open its design file
- `modules/**` - no module manifest or emitter changes; the gate changes prepass buckets, never module dispatch
- `crates/slicer-gcode/src/serialize.rs` - no padding twin for either key; never edited
- `crates/slicer-core/src/algos/prepass_slice.rs` - consumed read-only (orientation + span-gate helpers); the overwrite and stage land in the runtime prepass, not here
- `crates/slicer-runtime/src/layer_executor.rs` - the internal-bridge construction arm is read-only context (semantics precedent); never edited

## Expected Sub-Agent Dispatches

- Question: exact `process_external_surfaces` top/bottom custom-angle arms (gate shape, absolute vs relative application, align offset position); scope: `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp`; return: `SUMMARY`; purpose: Step 2 overwrite borrow.
- Question: exact `process_no_bridge` island separation and filled-vs-partial handling (mode distinction, coverage role, anchor-band role); scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SUMMARY`; purpose: Step 3 stage borrow.
- Question: did `cargo test -p slicer-ir` pass after the rows land; scope: `crates/slicer-ir`; return: `FACT`; purpose: Step 1 gate.
- Question: did `cargo xtask gen-config-docs --check` pass; scope: repo root; return: `FACT`; purpose: Step 4 gate.

## Data and Contract Notes

- IR/manifest contracts: none — no new IR field, no manifest row, no WIT line. The overwrite changes the existing `bridge_orientation_deg` bucket and the stage changes the existing `bridge_areas` bucket (plus the `is_bridge` flag where spans are newly authored).
- WIT boundary: untouched (no new accessor — the views already expose the buckets the arms change).
- Determinism/scheduler constraints: the overwrite is a per-region pure function of the resolved float (no ordering dependence — AC-2 pins repeated-run identity); the stage iterates timelines in the existing deterministic order and unions per-region spans (the timeline loop is sequential on ordinary prints). Both arms run inside the sequential `commit_shell_classification_builtin` reading the committed `SliceIR`; no new cross-layer dependency beyond the maps the existing bridge loop already builds.

## Locked Assumptions and Invariants

- Canonical defaults are locked: `bridge_angle = 0.0` (automatic), `counterbore_hole_bridging = "none"` (off). Defaults are identity (AC-5): zero means automatic = pre-packet shape, `none` means off = pre-packet shape — the inverse of 299's emitting default.
- DEV-193 clauses (minted at authoring, re-derive `max(DEV-*)` before writing): (a) `relative_bridge_angle` is not borrowed — absent from source, queue, and tree; the override applies absolutely; (b) the `align_infill_direction_to_model` rotation offset is not borrowed — draft packet 262a's scope; (c) `PrintObject.cpp`'s chbFilled slice-union (sacrificial fill counted as solid support above) is not borrowed — above-layer support-map restructure is out of scope; (d) canonical's `Print.cpp` reslice-invalidation entries ride ticket 124.
- The five-way partition invariant holds by construction: authored `bridge_areas` subtract already-bridged spans and the partition subtracts `bridge_areas` from the sparse zone at Perimeters commit — authored spans land in the bridge bucket, never double-counted.

## Risks and Tradeoffs

- The `sacrificiallayer` default-off shape is live on stepped holes: hole-bearing prints newly author bridge spans where the pre-packet tree authored nothing. AC-3 pins the intended change; AC-N2 pins the no-hole boundary (hole-free spans identical); any sloping stepped-hole fixture churn lands as test-fixture updates with measured justification (map test discipline).
- The likeliest silent defect is a mode swap (`partial` authoring whole spans or `filled` authoring rims only) — pinned by AC-3/AC-4 asserting the hole interior separately in the two modes on the same fixture.
- The second likeliest silent defect is an overwrite/gate-order swap (the `> 0` check running before detection, or detection overwriting the override) — pinned by AC-2 asserting the override wins on a non-degenerate span.
- Draft-299 merge hazard: both packets edit `resolve_shell_counts`' neighbourhood and the same prepass file. The implementer rebases onto whichever lands first; the AC commands name only this packet's tests, so a 299-first landing cannot silently satisfy them.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 — stage + orientations + hole tests)
- Highest-risk dispatch and required return format: bridge-angle arm borrow — `SUMMARY` (≤200 words) from `LayerRegion.cpp`; reject any reply pasting the whole function.

## Open Questions

None. `[FWD]` none pending; `[BLOCK]` none.
