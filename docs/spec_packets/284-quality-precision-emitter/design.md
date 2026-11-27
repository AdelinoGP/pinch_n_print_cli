# Design: 284-quality-precision-emitter

## Controlling Code Paths

- Primary code path: `tolerance_for_role` (`crates/slicer-gcode/src/serialize.rs`) — the per-role D-P tolerance seam consumed by `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) at the simplify call (`simplify_polyline_mm` over XY pairs, `drop_short_segments_mm` under `min_segment_length`, `order_lock` bypass) — plus the `Move` rendering arm (`DefaultGCodeSerializer` in `crates/slicer-gcode/src/serialize.rs`, today `G0`-for-travel / `G1`-otherwise with `format_xyz` / E-accumulator / `F` suffix).
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_emit_per_role_tolerance_tdd.rs` (per-role tolerance precedent to extend, not duplicate), `crates/slicer-gcode/tests/gcode_emit_tdd.rs` + `golden_emit_tdd.rs` (emission no-regression), `docs/config/host-keys.toml` `[resolved_config]` shape, `host_keys_doc_lock_tdd` (runtime lock test asserting the TOML mirrors live Rust defaults, with per-key match arms to extend).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Host-emitter ownership stands (ticket-27 hazard checked): canonical's reads for both keys are emission/estimation-time (`GCodeWriter` arc + `m_resolution` store, `GCode` extrusion selection and stats store) plus generation-time consumers the port deliberately does not reproduce per stage (see DEV-176(a)); `machine-gcode-emit`'s generic `[key]` sweep is the wrong seam (no module reads these keys — `ConfigView::from_declared` whitelisting would make manifest rows dead, ticket-34 shape). No manifest declaration.
- `enable_arc_fitting` is host-only and omitted from `to_config_map` (P35 `enable_pressure_advance` / P18 `disable_m73` precedent): the emitter reads the typed `bool` directly; emitting it would add a CONFIG_BLOCK line for every print while buying no module visibility. The canonical `0`/`1` value spelling is owed via ticket 132, not spot-fixed here (DEV-176(d)).
- `resolution` is emitted into `to_config_map` (plain `f32`, always present): the live `0.01` shadows the stale padding `0.012` at runtime through `emit_config_kv` dedup — one intended default value change, count unchanged. The padding table itself is untouched (Authoring rule 2; AC-N3).
- Ticket-113 conformance: canonical `resolution` min `0` is a GUI hint (`PrintConfig.cpp` spinner only; `Config.cpp` never enforces). Enforcing it here is a deliberate divergence recorded in DEV-176(c), not parity. No `max` is adopted (no canonical speed-style maximum exists on this key; invented maxima were retired by ticket 113).
- Scalar-global is parity, not divergence: canonical declares both scalar (`coBool`, `coFloat`), so no vector DEV and no ticket-125 engagement. Per-tool composition rides the macro-generated overlay arms (ticket-126 precedent — no hand-written composition edit).
- Emitter tolerances stay `mm` `f32` end-to-end (`gcode_resolution` / `infill_resolution` / `support_resolution` / new `resolution` / `0.2 *` product); no internal-unit boundary is crossed, so no `Point2::from_mm` / `mm_to_units` conversion applies.

## Code Change Surface

- Selected approach: two scalar-global `ResolvedConfig` fields (`enable_arc_fitting: bool = false` with `extract_bool_or_first` ingest for real-3MF spellings per the ticket-140 lesson; `resolution: f32 = 0.01` with `extract_float`, `min 0`, no `max`) + one `to_config_map` arm for `resolution` only (arc intentionally omitted host-only) + `[resolved_config]` mirror rows + lock-test arms + effective-tolerance selection in `tolerance_for_role` + emitter-side arc coalescing in `emit_gcode` + emitter-side negative rejection + DEV-176 row + `gen-config-docs` regen + one new test file.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs` — `declare_resolved_config!` invocation (2 new field lines in the precision/resolution window) + `to_config_map` (1 new `resolution` insert adjacent to the existing host-key inserts; arc explicitly absent with the P35-comment precedent).
  - `crates/slicer-gcode/src/serialize.rs` — `tolerance_for_role` (effective-tolerance selection: arc off → `max(per_role, resolution)`; arc on → `min(per_role, 0.2 * resolution)`; travel arm unchanged at `0.0`) + `DefaultGCodeSerializer` `Move` rendering arm (unchanged for arc off; arc-on runs are supplied as `Raw` `G2`/`G3` by the emitter, so the renderer needs no new match arm).
  - `crates/slicer-gcode/src/emit.rs` — `DefaultGCodeEmitter::emit_gcode` (post-simplify arc coalescing over kept XY points before `Move` construction: XY-plane, same-Z, extrusion-only, travel-excluded, `order_lock`-excluded, E-conserving with `F` carried; arc off short-circuits to today's path) + validation before estimating (negative `resolution` → stable rejection).
  - `docs/config/host-keys.toml` — 2 `[resolved_config]` rows (`enable_arc_fitting` bool default `false`; `resolution` default `0.01`, `>= 0` range, canonical provenance notes).
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` — 2 new match arms (bool arm + `unwrap_or`/float arm mirroring the emission shape).
  - `docs/DEVIATION_LOG.md` — DEV-176 row (4 clauses below).
  - `docs/15_config_keys_reference.md` — regenerated (no hand edit).
  - `crates/slicer-gcode/tests/quality_precision_arc_resolution_tdd.rs` — NEW (auto-discovered; schema + 4 behaviour + 2 negative tests + padding-absence doc check).
- Rejected alternatives and reasons:
  - Per-role-only (no new global): rejected — leaves the canonical global with no decision point; the tier-table gap is exactly the missing global.
  - Generation-time per-module plumbing (declare `resolution` in perimeter/infill/support manifests): rejected — reproduces canonical's coupling across N owners for one tolerance; the emitter-side single seam is the PnP better answer (rule 4) and keeps the packet Tier B in one owner.
  - New `GCodeCommand::Arc` IR variant: rejected — enum blast radius (serializer match, all constructors, tests) for two keys; `Raw` `G2`/`G3` lines carry the same bytes with no contract change.
  - Serializer-side arc coalescing (lookahead over rendered `Move`s): rejected — `Move` carries no `order_lock` flag, so the serializer cannot honour ADR-0063; fitting must sit where the lock is visible (`emit_gcode`).
  - Emitter-side `Raw` for the tolerance too (bypass `tolerance_for_role`): rejected — splits the tolerance decision across two seams; the existing exhaustive function is the one seam.
  - Canonical conditional arc table (first-change / same-extruder gates): rejected — no extruder model exists in tree to condition on; uniform on/off gate recorded in DEV-176(b).
  - Per-tool vectors now: rejected — canonical is scalar; ticket-125 owns no model here.
  - Padding correction as deliverable: rejected — rule 2; the table is load-bearing (≥80 floor) and stays byte-untouched; shadowing via the resolved map is the only mechanism.

DEV-176 (single row, four clauses): (a) emission-side global — canonical applies `resolution` at generation time across perimeter/brim/fill/layer/slice/support; the port applies it once at emission as the floor/ceiling selection above (cleaner seam, one decision point; small-global-values floor at per-role tolerances rather than loosening them). (b) arc-fitter scope — XY extrusion-only coalescing with travel/lock exclusion and E conservation; canonical's spiral-travel density, wipe-tower propagation, and tree-support fan-out are not borrowed. (c) negative/min enforcement — canonical declares min `0` and never enforces (`Config.cpp` ignores min/max); the port rejects negatives (ticket-113 class). (d) CONFIG_BLOCK shape — `resolution` shadows padding with the single intended `0.012` → `0.01` value change; `enable_arc_fitting` is host-only omitted (no line at any value); the scalar-bool `0`/`1` spelling for any future emission rides ticket 132, not this packet.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs` - role: 2 field declarations + 1 map arm; expected change: 2 `cli` lines + 1 insert + P35-style omission comment for arc.
- `crates/slicer-gcode/src/serialize.rs` - role: effective-tolerance selection; expected change: `tolerance_for_role` body only (padding table untouched).
- `crates/slicer-gcode/src/emit.rs` + `crates/slicer-gcode/tests/quality_precision_arc_resolution_tdd.rs` - role: arc coalescing + validation + pins; expected change: fitter (~40 lines) + validation (~10 lines) + 1 new test file (grouped: estimate-adjacent, justified — single concern, lands with the behaviour step).
- `docs/config/host-keys.toml` + `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` + `docs/DEVIATION_LOG.md` - role: mirror + lock arms + behaviour record; expected change: 2 TOML rows, 2 match arms, 1 DEV row (grouped: docs-adjacent, justified — lands in one step with regen).

## Read-Only Context

- `crates/slicer-ir/src/resolved_config.rs` - precision/resolution window only - purpose: `cli` field declaration syntax (`bool` with `extract_bool_or_first`, `f32` with `extract_float` + `min`) and the `to_config_map` host-key insert region (arc-omission comment shape).
- `crates/slicer-gcode/src/serialize.rs` - `tolerance_for_role` + `Move` rendering arm + `ORCA_CONFIG_PADDING` resolution row only - purpose: exhaustive role match to extend, renderer to leave unchanged, padding row to leave untouched.
- `crates/slicer-gcode/src/emit.rs` - simplify + `Move`-construction window only - purpose: kept-points pipeline (`simplify_polyline_mm`, `drop_short_segments_mm`, `order_lock` bypass, coordinate-identity remap) where the fitter inserts.
- `crates/slicer-gcode/tests/gcode_emit_per_role_tolerance_tdd.rs` - construction precedent lines only - purpose: entity/IR fixture shape for the new tests.
- `docs/spec_packets/283-printer-timing-emitter/requirements.md` - mirror-precedent lines only - purpose: quote the `[resolved_config]` + lock-test + host-only-omission landing shape.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `modules/...` - no module reads these keys; do not browse for this packet
- `crates/slicer-gcode/src/serialize.rs` padding table (AC-N3 forbids touching it; cite the row, do not edit it)
- `docs/spec_packets/276-*/`, `docs/spec_packets/277-*/`, `docs/spec_packets/278-*`, `docs/spec_packets/281-*`, `docs/spec_packets/282-*`, `docs/spec_packets/283-*` - adjacent, not consumed; cite by name only
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: do both names remain absent as behaviour before Step 1 (no silent landing: `enable_arc_fitting` undeclared, bare `resolution` only in padding)?; scope: `crates/slicer-ir/src/resolved_config.rs`, `docs/config/host-keys.toml`, `crates/slicer-gcode/src/serialize.rs`; return: `FACT`; purpose: Step 1 pre-condition.
- Question: what stable error code does `emit_gcode` return for config validation, and which existing test asserts one?; scope: `crates/slicer-gcode/src/emit.rs`, `crates/slicer-gcode/tests/`; return: `SNIPPETS` (≤1 snippet, ≤30 lines); purpose: Step 3 validation seam.
- Question: did each gate command pass?; scope: none (run only); return: `FACT` (pass quoting result line) or `SNIPPETS` (fail, ≤20 lines); purpose: every Verification row.

## Data and Contract Notes

- IR/manifest contracts: none — host-only keys plus one `to_config_map` insert add no Slice IR field, no manifest schema, no `CONFIG_SCHEMA_WIRE_VERSION` bump, no guest rebuild.
- WIT boundary: none — tolerance selection plus emitter-internal coalescing into `Raw` text; no WIT accessor, no `SliceRegionView` metadata.
- Determinism/scheduler constraints: pure function of (paths, two config values); no RNG, no ordering effects; defaults → geometry-identical (AC-2).

## Locked Assumptions and Invariants

- Effective defaults locked to canonical (`false`, `0.01` via the emitted map for `resolution` and the typed field for arc).
- Every per-role default exceeds `0.01`, so `max()` is identity at defaults for all roles; `Custom` (travel) is always `0.0`.
- Arc-on tightening is exactly `min(per_role, 0.2 * resolution)`; arc lines appear iff `enable_arc_fitting` is true and a run passes the plane/Z/travel/lock/E-continuity guards.
- Change is reversible via defaults: arc off + `resolution = 0.01` restores HEAD move bytes exactly; CONFIG_BLOCK restoration needs only the stale `0.012` padding value.

## Risks and Tradeoffs

- `f32` tolerance vs canonical `float`: same width; comparisons in tests use round inputs (`0.5`, `0.01`, `0.002`) with exact-count assertions (fewer/more moves, ≥1 arc line) rather than float-equality on kept coordinates, to prevent precision drift.
- E conservation across coalescing must thread the relative/absolute accumulator exactly like the `Move` arm; AC-4 asserts `1e-3` conservation against the G1-only run rather than a hardcoded E constant.
- DEV-172–175 collision lesson: drafts already propose DEV-172/173/174/175 while only DEV-171 has landed in LOG; this packet takes DEV-176 as first collision-free — implementer must re-derive `max(DEV-*)` over LOG + `docs/spec_packets/*/` before writing the row.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 arc fitter + tests)
- Highest-risk dispatch and required return format: emit-time validation-error precedent (`SNIPPETS`, ≤30 lines) — the negative test must assert the packet's stable code through the real `emit_gcode` error type.

## Open Questions

None.
