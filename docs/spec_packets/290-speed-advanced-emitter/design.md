# Design: 290-speed-advanced-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — the tail after the per-entity loop, before the estimator/M73 block. The new stage walks the built `commands: Vec<GCodeCommand>` (print `Move`s carry geometry + sticky `F` + role; `E` is absolute), computes per-move volumetric rates from move geometry, and rewrites `F` in place / inserts split moves. No E computation is touched.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_emit_tdd.rs` owns the `emit_gcode` E2E fixture pattern (`point3_with_width`, `print_entity_fixture_with_id`, `layer_with_entity`) — the new `speed_p57_ers_emission_tdd.rs` guard clones that shape; `crates/slicer-gcode/tests/pressure_advance_emission_tdd.rs` owns the `with_resolved_config` + per-command-scan pattern (`pa_raw_texts`) this packet's retime assertions reuse; `docs/config/host-keys.toml` `[resolved_config]` rows + `cargo xtask gen-config-docs` output.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The stage lives in the existing owner (`crates/slicer-gcode`) at the existing emission seam — not as a host-side special case outside the emitter, not as module constants, and not in `machine-gcode-emit` (its generic `[key]` sweep publishes placeholders; move retiming is the host emitter's job — ticket-27 hazard checked, owner stands).
- Rule-4 trigger test does not fire: the three keys parameterise one emitter over moves it already sees (`GCodeCommand::Move` geometry + role); they do not select across alternative algorithm implementations, so no `claim:*` holders and no `*_fill_holder` selection.
- Stream position: after the entity loop, before the estimator/M73 tail — mirroring canonical's position after generation (`generator & pressure_equalizer & cooling …` per `GCode::process_layers`; the port has no spiral stage and no tbb — the packet borrows the after-generation position, not the filter shape). The estimator then times the *smoothed* stream (canonical's estimator runs downstream of the equalizer too), and M73/inject positions are unaffected (command-count changes land before their computation). No draft packet owns this position, so no FORWARD-DEP.
- Determinism: smoothing is a pure function of (move geometry, sticky feedrate, role, point `overhang_quartile`, resolved keys) — no ordering dependence beyond stream order, no cross-print state; the master gate short-circuits the whole stage.
- ADR-0062 conformance: order-locked paths bypass geometry mutation (D-P, min-segment) but "speed/flow side mutations remain legal" — F-retiming and sub-move splitting with conserved E are speed-side, and point direction is preserved (splits interpolate in emission order, never reverse); smoothing a locked path is conformant, not an amendment.
- Schema/version constants and event-specific locking: none bumped (no `PROGRESS_EVENT_SCHEMA_VERSION`, wire-version, or IR-version touch; host-only `ResolvedConfig` scalar fields are additive and the default-path impact is byte-identical, pinned by AC-2).

## Code Change Surface

- Selected approach: three scalar-global `ResolvedConfig` fields (canonical defaults; one bounded pair on the segment length) + one pure smoothing helper over the built command vector + bounds arms in the existing validator + host-keys rows + new TDD guard. Host-only omitted from `to_config_map` module visibility and from the CONFIG_BLOCK (ticket-42 precedent — emission-control scalars, not module inputs; zero padding twins).
- Exact functions, traits, manifests, tests, and fixtures:
  - `ResolvedConfig` declaration (`crates/slicer-ir/src/resolved_config.rs`, `declare_resolved_config!` invocation): 1× `bool = false` (`extrusion_rate_smoothing_external_perimeter_only`, `extract_bool`), 2× `f32` scalars (`extract_float @ { min/max }`) — slope `0.0` min `0`, segment length `3.0` min `0.5` max `5` — with CLI ingestion; the `@`-block mins/maxes flow into the ticket-113 bounds index via the same `ConfigBoundsIndex::from_modules` seam that seeds `slicer_ir::feedrate::speed_bounds()`.
  - `smooth_extrusion_rates(commands: &mut Vec<GCodeCommand>, slope: f32, segment_len: f32, external_only: bool)` (new, `crates/slicer-gcode/src/emit.rs` or sibling): `return` immediately when `slope <= 0` (master gate, AC-N2); per print `Move`, derive the volumetric rate from segment length (XY distance), per-move E delta (absolute-`E` differences), and sticky feedrate — the port's native equivalent of canonical's parsed `volumetric_extrusion_rate` (no text parsing, no `filament_crossections` table: E deltas already encode the cross-section); skip travels (`Custom("Travel")`), retracts/unretracts, and non-move commands outright; skip `BridgeInfill`/`InternalBridgeInfill`/`Ironing` always and, under the bool, every role except `OuterWall` and `overhang_quartile`-marked points; run the forward/backward limiter over the surviving rate sequence (canonical `adjust_volumetric_rate` shape — single slope value for both directions, per canonical's ctor assigning the same value to positive and negative); rewrite `F` only (`f = f * limited_rate / original_rate` per move, the port form of canonical's `volumetric_correction_avg`); split surviving moves longer than `segment_len` into `ceil(len / segment_len)` sub-moves with interpolated positions and conserved total E (canonical `output_gcode_line` split rule); leave sub-trivial deltas as one unmodified move (canonical `NON_TRIVIAL_RATE_DELTA` floor).
  - Call site: one call on the built `commands` after the entity loop closes, before `EstimatorLimits::from_config` (`crates/slicer-gcode/src/emit.rs`) — so the estimator times smoothed feedrates and M73 positions follow automatically.
  - Bounds: `min 0` on the slope, `min 0.5 max 5` on the segment length (reject-the-slice, DEV-182(a)); bool carries no range.
  - Schema rows: `docs/config/host-keys.toml` `[resolved_config]` gains three rows; generated `docs/15_config_keys_reference.md` regened in Step 1b.
  - Tests: new `crates/slicer-gcode/tests/speed_p57_ers_emission_tdd.rs` (schema guard AC-1, identity AC-2, retime AC-3, gate AC-4, split AC-5, bounds AC-N1, master gate AC-N2).
- Rejected alternatives and reasons:
  - Re-parsing serialized G-code text like canonical (`process_line` over `G1 X.. Y.. E.. F..` strings): rejected — the port holds the moves as typed `GCodeCommand::Move` with absolute `E`; stringifying then re-parsing would throw away type information to re-derive it lossily (fast_float vs f32, comment stripping, tag scanning). The IR-native rate derivation is the PnP-better seam (recorded divergence DEV-182(c)).
  - A per-entity in-loop limiter (adjusting `F` as each move is pushed): rejected — canonical's limiter needs lookahead/lookbehind across the whole extrusion run (the 128-line window, the one-layer delay); an in-loop form sees only the past and can only decelerate, never pre-shape acceleration. The post-stage sees the whole stream.
  - Splitting in the serializer (segment-length moves as text lines): rejected — wrong seam (the serializer renders commands; it must not invent geometry — E conservation would need re-derivation from text). Splits are `Move` insertions in the stage.
  - Wiring the `adjustable_flow` marker dimmer as a config key: rejected — the marker state is canonical-internal (`;_EXTRUDE_SET_SPEED` blocks per `GCode::_extrude`, off only on the sequential-object path this port lacks per ticket 32); the port has no marker blocks at all, so every print move is smoothable (DEV-182(b)).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-ir/src/resolved_config.rs` - role: declare the three fields; expected change: macro-invocation rows + CLI arms + bounds entries.
- `crates/slicer-gcode/src/emit.rs` - role: smoothing helper + one post-loop call site + bounds hookup; expected change: ~80 lines staged rate-limit/split/emission with role/gate/master-gate branches.
- `crates/slicer-gcode/tests/speed_p57_ers_emission_tdd.rs` (new) - role: AC-1–AC-N2 pins; expected change: net-new guard binary, no registration needed (auto-discovered).
- `docs/config/host-keys.toml` - role: `[resolved_config]` rows; expected change: three rows (extra justified: schema source of truth, one-line rows).
- `docs/DEVIATION_LOG.md` - role: DEV-182 row; expected change: one row with (a)+(b)+(c) clauses (extra justified: preflight-visible obligation).
- `docs/15_config_keys_reference.md` - role: generated output; expected change: regen only, via `cargo xtask gen-config-docs` (extra justified: generated, not hand-edited).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-gcode/src/emit.rs` - lines covering the post-entity-loop tail (estimator/M73 block) + the per-point `F` emission site only - purpose: stage call-site anchor (delegate a LOCATIONS fix before reading).
- `crates/slicer-gcode/src/serialize.rs` - lines 1–65 only (`tolerance_for_role` role-exhaustiveness precedent) - purpose: role-match exhaustiveness pattern for the skip list (do not touch).
- `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` float/bool row syntax + one scalar precedent only - purpose: field-declaration syntax model.
- `crates/slicer-ir/src/slice_ir.rs` - lines `2340-2420` (`Point3WithWidth.overhang_quartile` + `ExtrusionRole` variants) only - purpose: gate field spelling + role-variant spelling.
- `crates/slicer-gcode/tests/pressure_advance_emission_tdd.rs` - lines 1–115 only (fixture + `with_resolved_config` + scan pattern) - purpose: guard shape to clone (no change).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - read-only reference at most (zero twins for the three; table untouched — rule 2)
- `crates/slicer-gcode/src/estimator.rs` - read-only context at most (it times whatever the stage leaves; no change — the smoothed stream flows in automatically)
- `modules/core-modules/machine-gcode-emit/` - out of bounds (wrong seam for move retiming; generic sweep untouched)
- `modules/core-modules/path-optimization-default/src/lib.rs` - cite as travel-shape context only; do not edit
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: post-entity-loop tail anchor (where `commands` is complete, where the estimator/M73 block begins) + sticky-feedrate spelling at the call site; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each); purpose: Step 2 call-site positioning.
- Question: `declare_resolved_config!` float/bool row syntax + CLI arm pattern for one scalar precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each); purpose: Step 1 field declaration.
- Question: struct-literal blast radius of adding three `ResolvedConfig` fields (every test/non-test literal compiling against the struct + every test hard-asserting related defaults); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 edit-list completeness.
- Question: `cargo xtask gen-config-docs` invocation + `--check` gate spelling; scope: `xtask/src/gen_config_docs.rs`; return: `FACT` (≤5 lines); purpose: Step 1b regen.

## Data and Contract Notes

- IR/manifest contracts: no IR field, no WIT accessor, no manifest `[config.schema]` table for modules (host-only scalars); `docs/config/host-keys.toml` is the schema source of truth for the three.
- WIT boundary: untouched (no guest-visible key; no rebuild; `cargo xtask build-guests --check` not required at authoring — host-only prose, ticket-60 precedent).
- Determinism/scheduler constraints: smoothing is post-loop pure over the built stream; no claim, priority, or ordering interaction; `default_priority` untouched; splits insert deterministically in stream order so parallel emission order cannot disturb them — the emitter is single-stream.

## Locked Assumptions and Invariants

- At defaults the stage does not exist (`slope <= 0` returns before touching `commands`) — the packet's core identity invariant; AC-2/AC-N2 pin both spellings of it (default config, and explicit zero with non-default companions).
- The stage rewrites `F` only and conserves total `E` across every split and every retime — AC-3/AC-5 pin it; an `e`-touching diff fails review regardless of test colour.
- The skip list is exactly `BridgeInfill` + `InternalBridgeInfill` + `Ironing` + (under the bool) everything except `OuterWall` and `overhang_quartile`-marked points — `erOverhangPerimeter` folds into the quartile-marked arm (the port's overhang representation); no other role is skipped, none of the four is limited.
- Canonical scalarity IS held here (unlike packets 276/277/279–289): all three keys are scalar in canonical (`GCodeConfig`), so no DEV records a vector simplification — DEV-182 carries (a) bounds, (b) always-on markers, (c) IR-native port.
- `BottomSolidInfill`, `GapFill`, support roles, `Skirt`/`Brim` have no canonical skip and are limited normally — recorded in requirements Out of Scope, not a silent new behaviour.

## Risks and Tradeoffs

- Estimator coupling: the smoothed stream feeds the estimator, so enabling smoothing changes estimated time as well as motion (canonical-identical coupling — the estimator is downstream there too); AC-3 pins only the F-rewrite, not the estimate delta, to keep the contract narrow.
- Split-count churn: enabling smoothing on long moves inserts `Move`s, shifting `after_entity_index`-style positional assertions in neighbouring guards that count commands on a smoothed fixture — Step 3 re-baselines only its own guard (default fixtures are unsmoothed, so no neighbour churns at defaults).
- Bounds-as-divergence: enforcing `min 0` / `0.5–5` rejects values canonical loads silently (GUI-hint finding) — accepted per ticket-113's reject-the-slice ruling, recorded DEV-182(a).
- Single-slope simplification: canonical carries separate positive/negative slope fields but the ctor assigns both from the one config value today, so the port's single-slope limiter is faithful, not simplified — if upstream ever splits them, the helper gains a second parameter (noted, not built).

## Context Cost Estimate

- Aggregate: `M` (never L)
- Largest step: `M` (Step 2 smoothing stage + skip/split arms)
- Highest-risk dispatch and required return format: post-loop tail anchor + sticky-feedrate spelling (`LOCATIONS`, ≤10 entries) — mispositioning the call (before the loop closes, or after the estimator snapshot) silently smooths nothing or desyncs M73 timing.

## Open Questions

- `[FWD]` `overhang_quartile` coverage on wall points: do the wall producers stamp `overhang_quartile` on `OuterWall`/`InnerWall` points today (the AC-4 gate needs marked overhang points to exist), or does the gate need a producer-side stamp first? Implementer delegates a LOCATIONS check for `overhang_quartile: Some` producers; a missing stamp is a follow-up packet, not a blocker (the role arm still holds).
- `[FWD]` Trivial-floor magnitude: canonical's `10` is in mm³/min rate units over parsed text; the port's native floor needs re-derivation in its own rate units (mm³/s over IR moves) at implementation — AC-5 pins the behaviour (sub-trivial emits one unmodified move), not the constant.
- `[FWD]` Accel-then-decel vs single-slope: canonical picks per move by the peak-rate feasibility test; the port mirrors the branch structure but the peak test's rate units need the same re-derivation as the floor — AC-3/AC-5 pin the observable halves (retime happens; splits conserve E), not the branch internals.
- None blocking: no `[BLOCK]` — scope, seams, and verification are decidable from the tree as cited.
