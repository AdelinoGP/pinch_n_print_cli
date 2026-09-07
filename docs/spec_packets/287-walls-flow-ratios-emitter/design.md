# Design: 287-walls-flow-ratios-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — the per-entity loop computing `distance * point.width * height_delta * point.flow_factor / filament_area` into E. The new stage multiplies that E by the role-selected ratio (plus the first-layer modifier) after the existing computation, before serialization.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/` emitter TDD binaries (285/286's `seam_*_emission_tdd` pattern); new `flow_ratio_emission_tdd.rs` guard owns this packet's schema + behaviour pins; `docs/config/host-keys.toml` `[resolved_config]` rows + `cargo xtask gen-config-docs` output.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The stage lives in the existing owner (`crates/slicer-gcode`) at the existing emission seam — not as a host-side special case outside the emitter, not as module constants, and not in `machine-gcode-emit` (its generic `[key]` sweep publishes placeholders; E scaling is the host emitter's job — ticket-27 hazard checked, owner stands).
- Rule-4 trigger test does not fire: the seven ratios scale E inside one module over roles it already sees (`entity.path.role`); they do not select across alternative algorithm implementations, so no `claim:*` holders and no `*_fill_holder` selection. `support_flow_ratio`-family keys stay in P55.
- Determinism: multiplier selection is a pure function of (`entity.path.role`, per-point overhang marking, `global_layer_index`, resolved ratios + gate) — no ordering dependence, no cross-entity state; locked paths bypass without disturbing neighbours.
- Schema/version constants and event-specific locking: none bumped (no `PROGRESS_EVENT_SCHEMA_VERSION`, wire-version, or IR-version touch; host-only `ResolvedConfig` float/bool fields are additive and default-identity).

## Code Change Surface

- Selected approach: eight scalar-global `ResolvedConfig` fields (canonical defaults) + one pure multiplier helper + one call site in the E path + bounds arms in the existing validator + manifest/schema rows + new TDD guard. Host-only omitted from `to_config_map` module visibility and from the CONFIG_BLOCK (ticket-42 precedent — emission-control scalars, not module inputs; no padding twin exists for any of the eight).
- Exact functions, traits, manifests, tests, and fixtures:
  - `ResolvedConfig` declaration (`crates/slicer-ir/src/resolved_config.rs`, `declare_resolved_config!` invocation): 7× `f32 = 1.0` + 1× `bool = false`, with CLI ingestion (first-wins scalar; bool word-form rides ticket 132).
  - `resolve_flow_ratio_for(role, point, layer_index, cfg) -> f32` (new, `crates/slicer-gcode/src/emit.rs` or sibling): bottom-solid unconditional; outer/inner/overhang/gap/internal-solid gated on `set_other_flow_ratios`; first-layer `× first_layer_flow_ratio` when `global_layer_index == 0` and role is not `Skirt`/`Brim`; overhang arm reads per-point `overhang_quartile.is_some()` (DEV-179(b)); `order_lock` callers skip the helper (AC-N2).
  - Call site: multiply the existing `flow_factor`-derived E by the helper's return (one product; at defaults `1.0` — identity).
  - Bounds: `min 0, max 2` on the seven floats (reject-the-slice, DEV-179(a)); bool carries no range.
  - Manifests/schema: `machine-gcode-emit.toml` needs no row (host-only); `docs/config/host-keys.toml` `[resolved_config]` gains eight rows; generated `docs/15_config_keys_reference.md` regened in Step 1b.
  - Tests: new `crates/slicer-gcode/tests/flow_ratio_emission_tdd.rs` (schema guard AC-1, identity AC-2, role+gate AC-3, first-layer AC-4, overhang AC-5, bounds AC-N1, lock bypass AC-N2).
- Rejected alternatives and reasons:
  - Declaring the ratios on wall/infill modules and scaling `flow_factor` at production: rejected — production `flow_factor` is geometry-coupled (width/bridge logic) and split across five producers; the canonical decision point is emission-time and the port emits E in exactly one place.
  - Wiring `is_infill_first` as an emitter reorder: rejected — wrong seam (emitter preserves `ordered_entities`; reordering there would fight the scheduler's stable-sort merge and ADR-0062 priorities); returned to the queue for orchestration.
  - Wiring `max_travel_detour_distance` as an emitter travel clamp: rejected — no planner to clamp (emitter consumes precomputed travels; path-optimization emits direct moves); a clamp with no detour computation is declaration-only; returned to the queue for the avoidance-planner feature.
  - Adopting per-tool vectors now: rejected — canonical declares all eight scalar; the vector model stays with ticket 125.

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-ir/src/resolved_config.rs` - role: declare the eight fields; expected change: macro-invocation rows + CLI arms + bounds entries.
- `crates/slicer-gcode/src/emit.rs` - role: multiplier helper + call site + bounds hookup; expected change: ~40 lines staged E scaling with role/gate/layer/overhang/lock branches.
- `crates/slicer-gcode/tests/flow_ratio_emission_tdd.rs` (new) - role: AC-1–AC-N2 pins; expected change: net-new guard binary, no registration needed (auto-discovered).
- `docs/config/host-keys.toml` - role: `[resolved_config]` rows; expected change: eight rows (extra justified: schema source of truth, one-line rows).
- `docs/DEVIATION_LOG.md` - role: DEV-179 row; expected change: one row with (a)+(b) clauses (extra justified: preflight-visible obligation).
- `docs/15_config_keys_reference.md` - role: generated output; expected change: regen only, via `cargo xtask gen-config-docs` (extra justified: generated, not hand-edited).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-gcode/src/emit.rs` - lines covering `DefaultGCodeEmitter::emit_gcode` E computation only - purpose: composition position for the multiplier (delegate a LOCATIONS fix before reading).
- `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` float/bool row syntax + one neighbouring ratio-free scalar field only - purpose: field-declaration syntax model.
- `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` only (`ExtrusionRole` variants) - purpose: role-variant spelling for the mapping arms.
- `crates/slicer-ir/src/slice_ir.rs` - lines `2340-2360` only (point `flow_factor` + `overhang_quartile` fields) - purpose: overhang-selection field spelling.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - read-only reference at most (no twin exists for the eight; table untouched — rule 2)
- `modules/core-modules/machine-gcode-emit/` - out of bounds (wrong seam for E scaling; generic sweep untouched)
- `crates/slicer-runtime/src/layer_executor.rs` (`assemble_ordered_entities_with_support_identities`) - cite as the returned `is_infill_first` home; do not edit
- `modules/core-modules/path-optimization-default/src/lib.rs` - cite as the returned detour-key context (direct-travel evidence); do not edit
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: E-computation anchor + `order_lock` bypass spelling in `emit.rs`; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each); purpose: Step 2 call-site positioning.
- Question: `declare_resolved_config!` float/bool row syntax + CLI arm pattern for one scalar precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each); purpose: Step 1 field declaration.
- Question: struct-literal blast radius of adding eight `ResolvedConfig` fields (every test/non-test literal compiling against the struct + every test hard-asserting related defaults); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 edit-list completeness.
- Question: `cargo xtask gen-config-docs` invocation + `--check` gate spelling; scope: `xtask/src/gen_config_docs.rs`; return: `FACT` (≤5 lines); purpose: Step 1b regen.

## Data and Contract Notes

- IR/manifest contracts: no IR field, no WIT accessor, no manifest `[config.schema]` table for modules (host-only scalars); `docs/config/host-keys.toml` is the schema source of truth for the eight.
- WIT boundary: untouched (no guest-visible key; no rebuild; `cargo xtask build-guests --check` not required at authoring — host-only prose, ticket-60 precedent).
- Determinism/scheduler constraints: multiplier is per-entity pure; no claim, priority, or ordering interaction; `default_priority` untouched; locked-path bypass preserves ADR-0062 self-clipping (producer footprint untouched — scaling skipped, not compensated).

## Locked Assumptions and Invariants

- At defaults (ratios `1.0`, gate `false`) every E is identity — the stage is provably inert without the implementer needing goldens (AC-2 pins it).
- Canonical scalarity holds for all eight (verified at authoring against the map oracle): no ticket-125 vector arm, no `tool_config:<idx>:` override in this packet; per-tool stays out of scope.
- Overhang selection via point-level marking (`overhang_quartile.is_some()`) is a recorded port-seam divergence (DEV-179(b)), not a semantic change: canonical keys off the overhang perimeter role, the port has no such role and marks overhang at the point level.
- `is_infill_first` / `max_travel_detour_distance` leave no stub behind — returning them writes tier-table annotations only, never declarations.

## Risks and Tradeoffs

- Overhang-marking coverage risk: if some overhang walls lack point marking on a producer path, the ratio under-applies on that path (fail-quiet, not fail-loud). Mitigation: AC-5 pins both arms on a fixture whose marking the test constructs explicitly; the residual producer-coverage question rides as an `[FWD]` re-check, not a blocker.
- Gate-adoption churn: P55 (ticket 62) must shed `set_other_flow_ratios` (9→8) and depend backward on this packet — recorded in Step 4's 05 annotation so the next claim does not redeclare the key.
- Bounds-as-divergence: enforcing `max 2` rejects values canonical loads silently (GUI-hint finding) — accepted per ticket-113's reject-the-slice ruling, recorded DEV-179(a).

## Context Cost Estimate

- Aggregate: `M` (never L)
- Largest step: `M` (Step 2 emitter stage + AC-3/AC-4 arms)
- Highest-risk dispatch and required return format: E-computation anchor + lock-bypass spelling (`LOCATIONS`, ≤10 entries) — mispositioning the product silently double-scales or misses locked paths.

## Open Questions

- `[FWD]` Overhang-marking producer coverage: do all five wall/infill producers that can emit overhang geometry stamp `overhang_quartile` consistently, or does one path need a stamping fix before AC-5's selection is complete? Implementer delegates a LOCATIONS check; a missing stamp is a follow-up, not a packet blocker.
- `[FWD]` First-layer index source: confirm `global_layer_index == 0` (not `layer_index`) is the emitter-visible layer-0 test at the call site; if the emitter sees only a local index, carry the global through the existing layer context rather than inventing a field.
- None blocking: no `[BLOCK]` — scope, seams, and verification are decidable from the tree as cited.
