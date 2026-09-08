# Design: 291-slow-down-layers-initial-layer-speed

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — the per-entity `F` resolution at the per-point `Move` emission site (`resolve_feedrate(role, entity-or-profile factor)` over the `feedrate_config` table wired from the raw config by `run.rs`, plus the typed `resolved_config` for the new key), extended with a layer-gated blend factor. The new helper `slow_down_blend_factor(layer_index, slow_down_layers) -> Option<f32>` (or inline arm) computes the canonical `lerp` weight; the call site multiplies the resolved base speed by the blend before the existing `* 60.0` + clamp + round rendering. No E computation is touched.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` owns the `emit_gcode` multi-layer fixture pattern (`LayerCollectionIR` with distinct `global_layer_index` values, `new_with_config` + `with_resolved_config` composition) — the new `speed_p58_slow_down_layers_emission_tdd.rs` guard clones that shape; `crates/slicer-gcode/tests/pressure_advance_emission_tdd.rs` owns the `with_resolved_config` + per-command-scan pattern this packet's F-comparison assertions reuse; `docs/config/host-keys.toml` `[resolved_config]` rows + `cargo xtask gen-config-docs` output.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The arm lives in the existing owner (`crates/slicer-gcode`) at the existing emission seam — not as a host-side special case outside the emitter, not as module constants, and not in `machine-gcode-emit` (its generic `[key]` sweep publishes placeholders; per-move feedrate math is the host emitter's job — ticket-27 hazard checked, owner stands).
- Rule-4 trigger test does not fire: the key parameterises one emitter over moves it already sees (role + layer index + already-resolved speeds); it does not select across alternative algorithm implementations, so no `claim:*` holders and no `*_fill_holder` selection.
- Stream position: in-loop, per-entity, at `F` resolution time — mirroring canonical's position inside `GCode::_extrude` after role-speed selection (`speed_for_path` result in hand) and before the filament-cap. The port resolves base speeds in `resolve_feedrate`'s role match and applies per-point/per-entity factors at the call site; the blend arm composes there (as a factor on the resolved mm/s value, clamped by the existing `0.05..=5.0` factor clamp only insofar as the factor path is reused — see the clamp note below). The estimator then times the *blended* stream (canonical's estimator runs downstream of emission too), and M73 positions follow automatically. No draft packet owns this position, so no FORWARD-DEP.
- Determinism: the blend is a pure function of (`entity.path.role`, `layer.global_layer_index`, resolved `slow_down_layers`, resolved `FeedrateConfig` first-layer speeds, entity/profile factor) — no ordering dependence beyond stream order, no cross-entity state; the master gate short-circuits the whole arm.
- ADR-0052 conformance: per-point factor REPLACES the whole-entity scalar (fallback, never composition — the existing `profile…unwrap_or(entity.path.speed_factor)` call). The blend composes with the *resolved* speed after that fallback (it scales the mm/s value the fallback selected), never by rewriting the factor itself — so the single-place `resolve_feedrate` contract (base-speed selection + `clamp(0.05, 5.0)` host-side, in one function) is untouched and no contract change is implied.
- ADR-0062 conformance: locks protect sequence and geometry (points, widths) while "speed/flow side mutations remain legal" — F-scaling is speed-side and point direction is preserved; blending a locked path is conformant, not an amendment.
- The `#if 0` over-raft first-layer arm is dead upstream and is not borrowed: `GCode::_extrude`'s accel-side `object_layer_over_raft` branch is compiled out, and the speed side has no over-raft arm at all — only the two live slow-down arms (plain + raft-offset). The raft-offset arm's *offset* (subtract `raft_layers` from the layer counter) is documented as a comment because the port emits no raft prefix layers today; it is not a runtime branch (DEV-183(c)).
- The `BottomSolidInfill` exemption and the flat skirt/brim hold are deliberate port divergences, not borrowed canonical (DEV-183(b) — see the call-site rationale): canonical's `erBottomSurface` flatness falls out of the never-slow guard, and canonical overrides `erSkirt` post-blend from `skirt_speed`; this port holds all three flat explicitly because its role-speed table differs at exactly those rows.
- Schema/version constants and event-specific locking: none bumped (no `PROGRESS_EVENT_SCHEMA_VERSION`, wire-version, or IR-version touch; one host-only `ResolvedConfig` int field is additive and the default-path impact is byte-identical, pinned by AC-2).

## Code Change Surface

- Selected approach: one scalar-global `ResolvedConfig` int field (canonical default `0`, `min 0`) + one pure blend helper over (layer index, key, first-layer speed, role speed) + call-site composition at the per-point `F` emission + host-keys row + new TDD guard. Host-only omitted from `to_config_map` module visibility (packet-42/P35 precedent — the `to_config_map` body is hand-written per key, so omission is one absent `m.insert`, and the emitter reads the typed field directly) and from the CONFIG_BLOCK (the block renders the raw config map, which carries no such key — zero padding twins).
- Exact functions, traits, manifests, tests, and fixtures:
  - `ResolvedConfig` declaration (`crates/slicer-ir/src/resolved_config.rs`, `declare_resolved_config!` invocation): 1× `u32 = 0` (`slow_down_layers`, `extract_int_as_u32` — `wall_loops` precedent) with CLI ingestion and no `@`-block bounds (none exists to declare — post-extraction values are `u32`, so no runtime bound is representable).
  - `slow_down_blend_factor(layer_index: u32, slow_down_layers: u32) -> Option<f32>` (new, `crates/slicer-gcode/src/emit.rs` or sibling): `None` when `slow_down_layers <= 1` (master gate — canonical `> 1`, AC-2); `None` when `layer_index >= slow_down_layers` (ramp complete — full role speed); else `Some(layer_index as f32 / slow_down_layers as f32)` (the canonical lerp weight; layer 0 yields 0.0 = pure first-layer speed).
  - Call site (one arm in the per-point `F` emission, `crates/slicer-gcode/src/emit.rs`): when the factor is `Some(t)` — resolve the base role speed `S` (the value `resolve_feedrate`'s role match would return, before `* 60.0`) and the first-layer speed `F0` (`feedrate_config.initial_layer_speed` when `is_perimeter(role)`, else `feedrate_config.initial_layer_infill_speed` — canonical selection over the already-live `FeedrateConfig` fields, re-derive the predicate over this tree's `ExtrusionRole` variants at implementation: walls, not thin/gap guesses); skip the arm entirely for `BottomSolidInfill` and for `Skirt`/`Brim` regardless of `skirt_speed` (both deliberate port divergences, DEV-183(b): canonical's `erBottomSurface` role speed IS `initial_layer_infill_speed`, so the `first_layer_speed < speed` guard alone keeps it flat — this port's PnP-only `bottom_surface_speed` needs the explicit skip or the arm invents a ramp canonical never has; and the port resolves both `Skirt` and `Brim` to `skirt_speed` with no `erBrim` arm at the borrowed site, so both are held flat — implement as an early skip with the rationale in a code comment, not as blend-then-override); if `F0 >= S`, leave `F` untouched (canonical never-slow guard); else emit `F = lerp(F0, S, t) * 60` through the existing clamp+round rendering. Clamp note: the existing `speed_factor.clamp(0.05, 5.0)` clamps *factors*, not speeds — the blend must scale the mm/s value directly (or as an equivalent pre-clamp factor `lerp(F0,S,t)/S`), never as a post-clamp factor, or a deep blend (e.g. F0/S < 0.05) would clamp to the wrong speed. Travel/retract/z-hop `Move`s (roles `Custom("Travel")`, `Retract` commands) never enter the arm — canonical blends print paths only.
  - Bounds: none beyond the type — `u32` post-extraction admits no representable violation, so there is no emitter-side bound to build (contrast packet-282's `f32` speeds, which need one). Non-integer spellings reaching the extractor reject as `TypeMismatch` (the `extract_int_as_u32` contract, `wall_loops` precedent); the negative-`Int` wrap is the shared pre-existing extractor contract, recorded in DEV-183(a) as map context, not introduced here.
  - Schema rows: `docs/config/host-keys.toml` `[resolved_config]` gains one row; generated `docs/15_config_keys_reference.md` regened in Step 1b.
  - Tests: new `crates/slicer-gcode/tests/speed_p58_slow_down_layers_emission_tdd.rs` (schema guard AC-1, identity AC-2, blend AC-3, exemptions AC-4, bounds AC-N1).
- Rejected alternatives and reasons:
  - Implementing the blend as a per-point `speed_factor` profile (`EntitySpeedProfile` rows written by a pre-pass): rejected — profiles are producer-side per-point multipliers indexed by original point; the blend is a uniform per-layer scalar over already-resolved speeds, and no producer stamps layer indices onto profiles. An emitter-side arm reads `global_layer_index` directly with no new IR.
  - Putting the blend inside `resolve_feedrate`'s role match (a layer-aware base-speed table): rejected — `resolve_feedrate(role, factor)` takes no layer index and is called from travel/z-hop sites where the blend must not fire; threading the layer through every call site widens the blast radius for zero gain. The arm lives at the print-path call site only.
  - Publishing the key via the `machine-gcode-emit` placeholder sweep (a `[slow_down_layers]` template variable): rejected — wrong seam (placeholders render once at print start; canonical re-selects per path per layer; ticket-27 hazard).
  - Adopting per-nozzle vectors now: rejected — canonical's key is scalar `coInt` with no `NOZZLE_CONFIG` arm; there is no vector to adopt (unlike packets 276/277/279–289, DEV-183 carries no vector clause at all).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-ir/src/resolved_config.rs` - role: declare the one field; expected change: macro-invocation row + CLI arm (no `@`-block bounds — none representable).
- `crates/slicer-gcode/src/emit.rs` - role: blend helper + one print-path call-site arm; expected change: ~40 lines staged blend/selection/guard/exemption branches.
- `crates/slicer-gcode/tests/speed_p58_slow_down_layers_emission_tdd.rs` (new) - role: AC-1–AC-N1 pins; expected change: net-new guard binary, no registration needed (auto-discovered).
- `docs/config/host-keys.toml` - role: `[resolved_config]` row; expected change: one row (extra justified: schema source of truth, one-line row).
- `docs/DEVIATION_LOG.md` - role: DEV-183 row; expected change: one row with (a)+(b)+(c) clauses (extra justified: preflight-visible obligation).
- `docs/15_config_keys_reference.md` - role: generated output; expected change: regen only (one `resolved_config.rs::ResolvedConfig` int row under the host-speeds block), via `cargo xtask gen-config-docs` (extra justified: generated, not hand-edited).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-gcode/src/emit.rs` - lines covering the per-point `F` emission site (`resolve_feedrate` call with profile/entity-factor fallback) + the per-entity loop's layer binding only - purpose: blend-arm anchor + clamp-order confirmation (delegate a LOCATIONS fix before reading).
- `crates/slicer-ir/src/feedrate.rs` - lines covering `SPEED_KEYS` `initial_layer_speed` / `initial_layer_infill_speed` rows + defaults only - purpose: first-layer base confirmation (already live, no change).
- `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` int row syntax (`wall_loops` precedent) only - purpose: field-declaration syntax model.
- `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` only (`ExtrusionRole` variants) - purpose: `is_perimeter` predicate derivation + exemption-variant spelling.
- `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` - lines 1–115 only (fixture + `new_with_config` + scan pattern) - purpose: guard shape to clone (no change).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - read-only reference at most (zero twins for the key; table untouched — rule 2)
- `crates/slicer-gcode/src/estimator.rs` - read-only context at most (it times whatever the arm leaves; no change — the blended stream flows in automatically)
- `modules/core-modules/machine-gcode-emit/` - out of bounds (wrong seam for per-move feedrates; generic sweep untouched)
- `modules/core-modules/path-optimization-default/src/lib.rs` - cite as travel-shape context only; do not edit
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: per-point `F` emission anchor (the `resolve_feedrate` call with profile/entity-factor fallback) + the layer binding in scope there (`layer.global_layer_index` vs loop variable) + clamp position relative to the call; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each); purpose: Step 2 call-site positioning.
- Question: `declare_resolved_config!` int row + CLI arm syntax for the `wall_loops` precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each); purpose: Step 1 field declaration.
- Question: struct-literal blast radius of adding one `ResolvedConfig` field (every test/non-test literal compiling against the struct + every test hard-asserting related defaults); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 edit-list completeness.
- Question: `cargo xtask gen-config-docs` invocation + `--check` gate spelling; scope: `xtask/src/gen_config_docs.rs`; return: `FACT` (≤5 lines); purpose: Step 1b regen.

## Data and Contract Notes

- IR/manifest contracts: no IR field, no WIT accessor, no manifest `[config.schema]` table for modules (host-only scalar, absent from the hand-written `to_config_map` body by construction); `docs/config/host-keys.toml` is the schema source of truth for the key.
- WIT boundary: untouched (no guest-visible key; no rebuild; `cargo xtask build-guests --check` not required at authoring — host-only prose, ticket-60 precedent).
- Determinism/scheduler constraints: the blend is per-entity pure over the layer loop; no claim, priority, or ordering interaction; `default_priority` untouched; the layer counter is the emitted `global_layer_index` sequence (already Z-sorted by LayerFinalization), so parallel emission order cannot disturb it — the emitter is single-stream.

## Locked Assumptions and Invariants

- At defaults (`slow_down_layers = 0`, and `= 1` by canonical's `> 1` gate) the arm does not exist (helper returns `None` before touching any speed) — the packet's core identity invariant; AC-2 pins both spellings of it.
- The arm scales the mm/s value before `* 60.0` rendering and conserves total `E` trivially (it never touches `e`) — AC-3/AC-4 pin speeds only; an `e`-touching diff fails review regardless of test colour.
- The `is_perimeter` selection is canonical's predicate, re-derived over this tree's `ExtrusionRole` at implementation (walls — the implementer greps the tree for the predicate or derives walls-vs-rest from the role list; `ThinWall`/`GapFill` membership is decided then, not frozen here). `GapFill`, support roles, and whichever wall-adjacent roles land outside the predicate ride the generic non-perimeter infill base (`initial_layer_infill_speed`) through the guard — they have no canonical arm of their own; the explicit skip list is exactly `BottomSolidInfill` + `Skirt` + `Brim` (DEV-183(b)) — recorded here, not a silent extension.
- `BottomSolidInfill` + `Skirt` + `Brim` are explicitly skipped (DEV-183(b) port divergences — see above); travels/retracts never enter the arm.
- Canonical scalarity IS held here (unlike packets 276/277/279–289): the key is scalar `coInt` in canonical, so no DEV records a vector simplification — DEV-183 carries (a) extractor-contract note, (b) bottom/skirt/brim port divergences, (c) dead-arm + raft-comment record.

## Risks and Tradeoffs

- Clamp-order coupling: the existing `0.05..=5.0` factor clamp must not reshape the blend — blending the mm/s value directly avoids it entirely; if the implementer routes through the factor path instead, the pre-clamp factor `lerp(F0,S,t)/S` must be computed against the *unclamped* ratio. AC-3's deep-blend case (10 → 50 at t=1/3 ≈ 0.47, safely inside the clamp) passes either way; a guard with F0/S < 0.05 would distinguish — noted, not built (canonical has no clamp here at all).
- Estimator coupling: the blended stream feeds the estimator, so enabling the key changes estimated time as well as motion (canonical-identical coupling — the estimator is downstream there too); AC-3 pins only the F-values, not the estimate delta, to keep the contract narrow.
- First-layer-speed staleness: the blend reads `FeedrateConfig.initial_layer_speed` / `initial_layer_infill_speed`, which today nothing in the emitter consumes on the print path (zero-occurrence outside tests and the `run.rs` wiring — the emitter never special-cases layer 0 at all). The bases are live schema with canonical defaults (30/60); the packet does not change them, only reads them for the first time on this path.
- Bounds-as-non-event: canonical `min 0` is a GUI hint the port inherits structurally — post-extraction values are `u32`, so no runtime bound is representable and none is built (contrast packet-282's `f32` speeds, which need one); the negative-`Int` wrap is the shared pre-existing `extract_int_as_u32` contract, recorded in DEV-183(a) as map context, not introduced here.

## Context Cost Estimate

- Aggregate: `M` (never L)
- Largest step: `M` (Step 2 blend arm + selection/guard/exemption branches)
- Highest-risk dispatch and required return format: per-point F-emission anchor + layer binding + clamp position (`LOCATIONS`, ≤10 entries) — mispositioning the arm (inside the base-speed match, or after the `* 60.0` render, or on the travel path) silently blends nothing or blends travels.

## Open Questions

- `[FWD]` `is_perimeter` exact membership over this tree's roles: canonical `is_perimeter` covers walls (perimeter + external perimeter); this tree splits walls into `OuterWall`/`InnerWall`/`ThinWall` plus `GapFill`. The implementer derives the predicate at implementation (walls, likely `OuterWall|InnerWall` with `ThinWall` decided then); AC-3 pins the outer/sparse halves, AC-4 pins the exempt roles — the `ThinWall`/`GapFill` membership is a test-visible choice, not a blocker.
- `[FWD]` Raft-prefix layers if they ever emit: the blend counter must exclude them (canonical subtracts `raft_layers`); today there are none to exclude, so the offset is a code comment. If raft prefix emission lands, this arm gains the subtraction — filed as a comment obligation, not a branch.
- `[FWD]` `initial_layer_travel_speed` non-participation: canonical's blend covers print paths only (travels keep `travel_speed`); the port's travel path already resolves `travel_speed` with no layer logic — no change, recorded so a later travel-layer feature does not "complete" this packet by mistake.
- None blocking: no `[BLOCK]` — scope, seams, and verification are decidable from the tree as cited.
