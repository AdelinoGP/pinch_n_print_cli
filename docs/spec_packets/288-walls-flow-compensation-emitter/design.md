# Design: 288-walls-flow-compensation-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — the per-point loop computing `distance * point.width * height_delta * point.flow_factor / filament_area` into E. The new stage multiplies that E by the role-selected ratio (global print first, then the role arm) and, for solid roles with compensation enabled, by the line-length model's interpolation — both after the existing computation, before serialization.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/` emitter TDD binaries (287's `flow_ratio_emission_tdd` pattern); new `walls_p55_flow_emission_tdd.rs` guard owns this packet's schema + behaviour pins; `docs/config/host-keys.toml` `[resolved_config]` rows + `cargo xtask gen-config-docs` output.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The stage lives in the existing owner (`crates/slicer-gcode`) at the existing emission seam — not as a host-side special case outside the emitter, not as module constants, and not in `machine-gcode-emit` (its generic `[key]` sweep publishes placeholders; E scaling is the host emitter's job — ticket-27 hazard checked, owner stands).
- Rule-4 trigger test does not fire: the five ratios scale E inside one module over roles it already sees (`entity.path.role`); the compensator scales per-segment E inside the same module over solid roles it already sees. Neither selects across alternative algorithm implementations, so no `claim:*` holders and no `*_fill_holder` selection.
- Determinism: multiplier selection is a pure function of (`entity.path.role`, resolved ratios + gate); compensator selection is a pure function of (segment length, role, parsed model + bool) — no ordering dependence, no cross-entity state; locked paths bypass both without disturbing neighbours.
- Pattern-gate omission is a recorded divergence, not a gap: canonical `_needSAFC` additionally requires a rectilinear-family pattern on the matching surface role, but patterns are holder-selected module identity here (rule 4 holder-only) and the emitter never sees them. Default holders are rectilinear-family, so default behaviour is faithful. DEV-180(c).
- Schema/version constants and event-specific locking: none bumped (no `PROGRESS_EVENT_SCHEMA_VERSION`, wire-version, or IR-version touch; host-only `ResolvedConfig` float/bool/string fields are additive and default-identity).

## Code Change Surface

- Selected approach: seven scalar-global `ResolvedConfig` fields (canonical defaults) + one pure multiplier helper + one model parser/interpolator + two call-site products in the E path + bounds arms in the existing validator + manifest/schema rows + new TDD guard. Host-only omitted from `to_config_map` module visibility and from the CONFIG_BLOCK (ticket-42 precedent — emission-control scalars, not module inputs; no padding twin exists for any of the seven).
- Exact functions, traits, manifests, tests, and fixtures:
  - `ResolvedConfig` declaration (`crates/slicer-ir/src/resolved_config.rs`, `declare_resolved_config!` invocation): 5× `f32` (four `1.0` min-0 + `print_flow_ratio` `1.0` min-0.01) + 1× `bool = false` + 1× model string (canonical ten-pair default verbatim), with CLI ingestion (first-wins scalar; Orca vector spellings take element 0 per the `extract_*_or_first` precedent; bool word-form rides ticket 132).
  - `resolve_p55_flow_ratio_for(role, cfg, gate) -> f32` (new, `crates/slicer-gcode/src/emit.rs` or sibling): global `print_flow_ratio` always applied; `TopSolidInfill` unconditional; `SparseInfill` / `SupportMaterial` / `SupportInterface` gated on the draft-287 `set_other_flow_ratios` value read from the same `ResolvedConfig` (reference, never a second declaration); all other roles `1.0`.
  - `parse_small_area_model(text) -> Result<Vec<(f64, f64)>, StableError>` + `small_area_factor(length, model) -> f64` (new, same file): strict comma-pair parse of the `length,factor;` table, ascending-length interpolation between entries, `1.0` past the max length; malformed lines reject (AC-N1) rather than fall back.
  - Call sites: multiply the existing `flow_factor`-derived E by the helper's return, then — for `InternalSolidInfill` / `TopSolidInfill` / `BottomSolidInfill` (`crates/slicer-ir/src/slice_ir.rs` `ExtrusionRole`) with compensation enabled — by `small_area_factor(distance, model)` using the loop's already-computed per-segment `distance`; at defaults both products are `1.0` — identity.
  - Bounds: `min 0 / max 2` on four floats, `min 0.01 / max 2` on `print_flow_ratio` (reject-the-slice, DEV-180(a)); bool carries no range; model string validated by the strict parser.
  - Manifests/schema: `machine-gcode-emit.toml` needs no row (host-only); `docs/config/host-keys.toml` `[resolved_config]` gains seven rows; generated `docs/15_config_keys_reference.md` regened in Step 1b.
  - Tests: new `crates/slicer-gcode/tests/walls_p55_flow_emission_tdd.rs` (schema guard AC-1, identity AC-2, role+gate AC-3, small-area AC-4, global AC-5, bounds AC-N1, lock bypass AC-N2).
- Rejected alternatives and reasons:
  - Declaring the ratios on wall/infill/support modules and scaling `flow_factor` at production: rejected — production `flow_factor` is geometry-coupled (width/bridge logic) and split across producers; the canonical decision point is emission-time and the port emits E in exactly one place (287's rejected alternative, unchanged here).
  - Wiring `reduce_crossing_wall` as an emitter travel clamp: rejected — no planner to clamp (emitter consumes precomputed travels; path-optimization emits direct moves); a clamp with no detour computation is declaration-only; returned to the queue for the avoidance-planner feature (ticket-61 detour precedent).
  - Gating the compensator on surface patterns via new IR metadata: rejected — patterns are module identity (rule 4 holder-only, never declared input keys); carrying pattern names to the emitter would reintroduce the enum this map removed. Role-only gating with the DEV-180(c) divergence is the PnP way.
  - Adopting per-tool vectors now: rejected — canonical declares all seven scalar; the vector model stays with ticket 125.

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-ir/src/resolved_config.rs` - role: declare the seven fields; expected change: macro-invocation rows + CLI arms + bounds entries.
- `crates/slicer-gcode/src/emit.rs` - role: multiplier helper + model parser/interpolator + two call-site products + bounds hookup; expected change: ~60 lines staged E scaling with role/gate/length/model/lock branches.
- `crates/slicer-gcode/tests/walls_p55_flow_emission_tdd.rs` (new) - role: AC-1–AC-N2 pins; expected change: net-new guard binary, no registration needed (auto-discovered; S7 file-per-binary like 287's guard).
- `docs/config/host-keys.toml` - role: `[resolved_config]` rows; expected change: seven rows (extra justified: schema source of truth, one-line rows).
- `docs/DEVIATION_LOG.md` - role: DEV-180 row; expected change: one row with (a)+(b)+(c) clauses (extra justified: preflight-visible obligation).
- `docs/15_config_keys_reference.md` - role: generated output; expected change: regen only, via `cargo xtask gen-config-docs` (extra justified: generated, not hand-edited).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-gcode/src/emit.rs` - lines covering `DefaultGCodeEmitter::emit_gcode` E computation + `order_lock` handling only - purpose: composition position for the two products (delegate a LOCATIONS fix before reading).
- `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` float/bool row syntax + one scalar precedent only - purpose: field-declaration syntax model.
- `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` only (`ExtrusionRole` variants) - purpose: role-variant spelling for the mapping arms.
- `crates/slicer-gcode/src/emit.rs` - lines `594-660` only (per-point `distance` + `flow_factor` E computation) - purpose: segment-length source for the compensator (reuse the loop's `distance`, do not recompute).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - read-only reference at most (no twin exists for the seven; `reduce_crossing_wall`'s twin stays untouched with the returned key — rule 2)
- `modules/core-modules/machine-gcode-emit/` - out of bounds (wrong seam for E scaling; generic sweep untouched)
- `modules/core-modules/path-optimization-default/src/lib.rs` - cite as the returned travel-key context (direct-travel evidence); do not edit
- `docs/spec_packets/287-walls-flow-ratios-emitter/` - cite as the gate producer (FORWARD-DEP); never edit another packet (annotations here only reference it)
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: E-computation anchor + `order_lock` bypass spelling in `emit.rs`; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each); purpose: Step 2 call-site positioning.
- Question: `declare_resolved_config!` float/bool/string row syntax + CLI arm pattern for one scalar precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each); purpose: Step 1 field declaration.
- Question: struct-literal blast radius of adding seven `ResolvedConfig` fields (every test/non-test literal compiling against the struct + every test hard-asserting related defaults); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 edit-list completeness.
- Question: `cargo xtask gen-config-docs` invocation + `--check` gate spelling; scope: `xtask/src/gen_config_docs.rs`; return: `FACT` (≤5 lines); purpose: Step 1b regen.

## Data and Contract Notes

- IR/manifest contracts: no IR field, no WIT accessor, no manifest `[config.schema]` table for modules (host-only scalars); `docs/config/host-keys.toml` is the schema source of truth for the seven.
- WIT boundary: untouched (no guest-visible key; no rebuild; `cargo xtask build-guests --check` not required at authoring — host-only prose, ticket-60 precedent).
- Determinism/scheduler constraints: multiplier + compensator are per-segment pure; no claim, priority, or ordering interaction; `default_priority` untouched; locked-path bypass preserves ADR-0062 self-clipping (producer footprint untouched — scaling skipped, not compensated).

## Locked Assumptions and Invariants

- At defaults (ratios `1.0`, compensation `false`) every E is identity — the stage is provably inert without the implementer needing goldens (AC-2 pins it).
- Canonical scalarity holds for all seven (verified at authoring against the map oracle): no ticket-125 vector arm, no `tool_config:<idx>:` override in this packet; per-tool stays out of scope.
- The draft-287 gate is referenced, never redeclared: Step 1's schema diff must not contain `set_other_flow_ratios`, and AC-3's gate-`false` arm is meaningful only after 287 lands (activation sequences after it — a FORWARD-DEP, not a satisfied dependency).
- `reduce_crossing_wall` leaves no stub behind — returning it writes tier-table annotations only, never declarations.
- The model default is adopted verbatim from `PrintConfig.cpp` (ten `length,factor` pairs); the parser treats that text as the golden valid input (AC-1 pins the round-trip).

## Risks and Tradeoffs

- Gate-activation ordering risk: AC-3's gated arms cannot pass until draft 287's gate field exists (same `ResolvedConfig`, different packet). Mitigation: the packet sequences activation after 287 and AC-3's gate-`false` arm passes standalone; the residual ordering is a FORWARD-DEP, not a blocker at authoring.
- Model-parse strictness risk: rejecting malformed lines fails slices canonical would load silently (GUI-hint finding extended to strings). Mitigation: strict-parse is the ticket-113 reject-the-slice rule applied uniformly; AC-N1 pins one malformed line, not an exhaustive grammar.
- Pattern-gate omission risk: solid segments under non-rectilinear holders compensate where canonical would not (fail-open, small E deltas on short segments only). Mitigation: DEV-180(c) records it with the default-holder fidelity rationale; AC-4 pins role gating, not pattern gating.
- Bounds-as-divergence: enforcing `max 2` (and `print_flow_ratio`'s `0.01` floor) rejects values canonical loads silently — accepted per ticket-113's ruling, recorded DEV-180(a).

## Context Cost Estimate

- Aggregate: `M` (never L)
- Largest step: `M` (Step 2 emitter stage + model parser)
- Highest-risk dispatch and required return format: E-computation anchor + lock-bypass spelling (`LOCATIONS`, ≤10 entries) — mispositioning either product silently double-scales or misses locked paths.

## Open Questions

- `[FWD]` Model-table edge semantics: does canonical clamp below the first entry, extrapolate, or hold the first factor for zero-length segments? Implementer delegates a SUMMARY check on `SmallAreaInfillFlowCompensator::flow_comp_model`; a mismatch is a follow-up, not a packet blocker (AC-4 pins mid-table interpolation + past-max identity only).
- `[FWD]` Gate-field read path at the call site: confirm the draft-287 `set_other_flow_ratios` field is readable from the same `ResolvedConfig` the emitter already holds (no new threading); if the emitter holds a snapshot without it, carry it through the existing config reference rather than inventing a channel.
- None blocking: no `[BLOCK]` — scope, seams, and verification are decidable from the tree as cited.
