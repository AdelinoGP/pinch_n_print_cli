# Design: 289-speed-acceleration-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — the per-entity loop already exposing `entity.path.role` and `global_layer_index`. The new stage selects one accel per print entity via the canonical precedence chain and emits through the existing `GcodeFlavor::set_acceleration` arm; travels emit through `set_travel_acceleration` where the flavor supports a separate travel command. No E computation is touched.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_emit_tdd.rs` owns the `emit_gcode` E2E fixture pattern (`point3_with_width`, `print_entity_fixture_with_id`, `layer_with_entity`) — the new `speed_p56_accel_emission_tdd.rs` guard clones that shape; `crates/slicer-gcode/tests/gcode_flavor_dialect_tdd.rs` owns the only M204/SET_VELOCITY_LIMIT expectations today (form-level, unwired to config); `docs/config/host-keys.toml` `[resolved_config]` rows + `cargo xtask gen-config-docs` output.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The stage lives in the existing owner (`crates/slicer-gcode`) at the existing emission seam — not as a host-side special case outside the emitter, not as module constants, and not in `machine-gcode-emit` (its generic `[key]` sweep publishes placeholders; motion commands are the host emitter's job — ticket-27 hazard checked, owner stands).
- Rule-4 trigger test does not fire: the eleven keys parameterise one emitter over roles/flavors it already sees (`entity.path.role`, `GcodeFlavor`); they do not select across alternative algorithm implementations, so no `claim:*` holders and no `*_fill_holder` selection.
- Stream position: 281's envelope (draft) opens the stream ahead of the start block; this packet's per-path lines follow it in canonical position (after the envelope, interleaved with moves the way canonical interleaves `set_acceleration` with extrusion). No reorder of 281's `M201 → M203 → M204 P/R/T → M205` order.
- Determinism: selection is a pure function of (`entity.path.role`, `global_layer_index`, travel-vs-print, resolved keys + flavor) — no ordering dependence, no cross-entity state; the master gate short-circuits the whole stage.
- Schema/version constants and event-specific locking: none bumped (no `PROGRESS_EVENT_SCHEMA_VERSION`, wire-version, or IR-version touch; host-only `ResolvedConfig` scalar fields are additive and the e2e/config-block impact is one intended M204 addition, pinned by AC-2).

## Code Change Surface

- Selected approach: eleven scalar-global `ResolvedConfig` fields (canonical defaults; three as float-or-percent over canonical bases) + one pure selection helper + accel-emit call sites on the print/travel paths + bounds arms in the existing validator + host-keys rows + new TDD guard. Host-only omitted from `to_config_map` module visibility and from the CONFIG_BLOCK (ticket-42 precedent — emission-control scalars, not module inputs; zero padding twins).
- Exact functions, traits, manifests, tests, and fixtures:
  - `ResolvedConfig` declaration (`crates/slicer-ir/src/resolved_config.rs`, `declare_resolved_config!` invocation): 1× `bool = true` (`accel_to_decel_enable`, `extract_bool`), 7× `f32` bounded scalars (`extract_float @ { min/max }`), 3× `ResolvedFloatOrPercent` (`extract_float_or_percent` — ticket-107 `support_threshold_overlap` precedent) for the percent trio, with CLI ingestion (first-wins scalar; bool word-form rides ticket 132) and bounds arms seeded into the ticket-113 index.
  - `resolve_accel_for(role, layer_index, is_travel, cfg) -> Option<u32>` (new, `crates/slicer-gcode/src/emit.rs` or sibling): `None` when `default_acceleration <= 0` (master gate, AC-N2); travels → `travel_acceleration` (caller renders via the separate-travel arm or folds into the print form per `supports_separate_travel_acceleration`); prints → canonical chain (first-layer `> 0` at `global_layer_index == 0`, bridge/sparse/internal-solid via percent-resolved bases, outer for `OuterWall`/`ThinWall`, inner, top-surface, else default); `floor(v + 0.5)` rounding at the render boundary (canonical `floor(acceleration + 0.5)`).
  - Call sites: print path renders `GcodeFlavor::set_acceleration(accel)` with the Klipper `ACCEL_TO_DECEL` suffix (`accel * factor / 100`, suffix iff enable — reuse the arm's Klipper form, do not fork it); travel path renders `set_travel_acceleration(accel)` where `Some`, else nothing extra. Dedup: emit only on accel change (canonical emits per path through stateful `set_acceleration`; the port tracks last-emitted print/travel values — DEV-181(c) records the stateless-restore simplification).
  - Bounds: `min 0` on nine accels (reject-the-slice, DEV-181(a)); `min 1, max 100` on the factor; bool carries no range.
  - Schema rows: `docs/config/host-keys.toml` `[resolved_config]` gains eleven rows; generated `docs/15_config_keys_reference.md` regened in Step 1b.
  - Tests: new `crates/slicer-gcode/tests/speed_p56_accel_emission_tdd.rs` (schema guard AC-1, default-emit AC-2, role chain AC-3, percent bases AC-4, flavor gate AC-5, decel suffix AC-6, bounds AC-N1, master gate AC-N2).
- Rejected alternatives and reasons:
  - Declaring the accels on wall/infill modules and threading them through `flow_factor`-style production: rejected — canonical's decision point is emission-time and the port emits motion commands in exactly one place; production modules own geometry, not M204.
  - Publishing the keys via the `machine-gcode-emit` placeholder sweep (M204 in start G-code): rejected — wrong seam (placeholders render once at print start; canonical re-selects per path; ticket-27 hazard).
  - Wiring the short-travel/first-layer-travel/wipe-tower-travel overrides now: rejected — no port-side short-travel role or travel-index seam at the travel call site; wiring a branch on unobservable state would be declaration-only; `[FWD]` re-check when the travel sites expose it.
  - Adopting per-nozzle vectors now: rejected — canonical's `NOZZLE_CONFIG` vectors need the ticket-125 model; scalar-global matches packets 276/277/279–288 (DEV-181(b)).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-ir/src/resolved_config.rs` - role: declare the eleven fields; expected change: macro-invocation rows + CLI arms + bounds entries.
- `crates/slicer-gcode/src/emit.rs` - role: selection helper + print/travel call sites + bounds hookup; expected change: ~60 lines staged accel selection/emission with role/flavor/layer/master-gate branches.
- `crates/slicer-gcode/tests/speed_p56_accel_emission_tdd.rs` (new) - role: AC-1–AC-N2 pins; expected change: net-new guard binary, no registration needed (auto-discovered).
- `docs/config/host-keys.toml` - role: `[resolved_config]` rows; expected change: eleven rows (extra justified: schema source of truth, one-line rows).
- `docs/DEVIATION_LOG.md` - role: DEV-181 row; expected change: one row with (a)+(b)+(c) clauses (extra justified: preflight-visible obligation).
- `docs/15_config_keys_reference.md` - role: generated output; expected change: regen only, via `cargo xtask gen-config-docs` (extra justified: generated, not hand-edited).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-gcode/src/emit.rs` - lines covering `DefaultGCodeEmitter::emit_gcode` per-entity loop + travel emission only - purpose: selection/emission anchor + change-dedup position (delegate a LOCATIONS fix before reading).
- `crates/slicer-gcode/src/flavor.rs` - lines 74–120 only - purpose: `set_acceleration` / `set_travel_acceleration` / `supports_separate_travel_acceleration` signatures to reuse (do not fork).
- `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` float/bool/float-or-percent row syntax + one scalar precedent only - purpose: field-declaration syntax model.
- `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` only (`ExtrusionRole` variants) - purpose: role-variant spelling for the mapping arms.
- `crates/slicer-runtime/src/run.rs` - lines covering the `gcode_flavor` → `with_flavor` wiring only - purpose: flavor-selection confirmation (no change; the packet adds no new flavor path).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - read-only reference at most (zero twins for the eleven; table untouched — rule 2)
- `modules/core-modules/machine-gcode-emit/` - out of bounds (wrong seam for motion commands; generic sweep untouched)
- `modules/core-modules/path-optimization-default/src/lib.rs` - cite as travel-shape context only; do not edit
- `docs/spec_packets/281-machine-motion-limits-emitter/` - gate producer (stream position) — reference only, never edit
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: per-entity loop anchor + travel-emission site + last-emitted-state spelling in `emit.rs`; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each); purpose: Step 2 call-site positioning.
- Question: `declare_resolved_config!` float/bool/float-or-percent row syntax + CLI arm pattern for one scalar precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each); purpose: Step 1 field declaration.
- Question: struct-literal blast radius of adding eleven `ResolvedConfig` fields (every test/non-test literal compiling against the struct + every test hard-asserting related defaults); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 edit-list completeness.
- Question: `cargo xtask gen-config-docs` invocation + `--check` gate spelling; scope: `xtask/src/gen_config_docs.rs`; return: `FACT` (≤5 lines); purpose: Step 1b regen.

## Data and Contract Notes

- IR/manifest contracts: no IR field, no WIT accessor, no manifest `[config.schema]` table for modules (host-only scalars); `docs/config/host-keys.toml` is the schema source of truth for the eleven.
- WIT boundary: untouched (no guest-visible key; no rebuild; `cargo xtask build-guests --check` not required at authoring — host-only prose, ticket-60 precedent).
- Determinism/scheduler constraints: selection is per-entity pure; no claim, priority, or ordering interaction; `default_priority` untouched; change-dedup is per-stream monotonic (last-emitted print/travel values) so parallel emission order cannot disturb it — the emitter is single-stream.

## Locked Assumptions and Invariants

- At defaults the stage emits `M204 P500` / `M204 T10000` (flavor-adjusted) — the intended default output change; the stage is never inert at defaults unless `default_acceleration = 0` (AC-N2 pins the gate).
- Canonical scalarity is deliberately NOT held: canonical declares nine keys per-nozzle vector; scalar-global is the recorded port simplification (DEV-181(b)), consistent with packets 276/277/279–288 — no ticket-125 vector arm in this packet.
- Bridge percent base is `outer_wall_acceleration` (not default); sparse/internal-solid base is `default_acceleration` — canonical `ratio_over`, pinned by AC-4; a base at `0` resolves the percent to `0`, which falls through to default per the `> 0` rule.
- `BottomSolidInfill`, `GapFill`, support roles, `Skirt`/`Brim` have no canonical arm and fall through to default — recorded in AC-3, not a silent extension.

## Risks and Tradeoffs

- Default-output churn: every existing golden/self-captured baseline with an emitter stream gains M204 lines (AC-2 pins the exact lines; Step 3 re-baselines with measured justification — the change is the stage appearing, not geometry moving).
- Change-dedup divergence: canonical's stateful `set_acceleration` suppresses repeats via writer state; the port's last-emitted dedup must key print vs travel separately or travel lines collapse into print state (AC-5 pins both arms per flavor; Step 2 exit requires the two-keyed form).
- Bounds-as-divergence: enforcing `min 0` / factor `1–100` rejects values canonical loads silently (GUI-hint finding) — accepted per ticket-113's reject-the-slice ruling, recorded DEV-181(a).
- 281-sequencing churn: 281 is draft, so its envelope position is read from its `packet.spec.md` at implementation time, not frozen here; if 281 lands with a factored envelope submodule, the Step-2 anchor dispatch re-points there.

## Context Cost Estimate

- Aggregate: `M` (never L)
- Largest step: `M` (Step 2 selection stage + AC-3/AC-4 arms)
- Highest-risk dispatch and required return format: per-entity loop anchor + travel site + last-emitted spelling (`LOCATIONS`, ≤10 entries) — mispositioning the emit silently duplicates M204 on every path or drops travels.

## Open Questions

- `[FWD]` Short-travel overrides: do the port's travel call sites expose a short-travel/overhang test (length + neighbouring role) the canonical `travel_to` overrides could key on, or does that need a carrier from the path-optimization stage first? Implementer delegates a LOCATIONS check; a missing signal is a follow-up packet, not a blocker.
- `[FWD]` Calib-PA-line arm: `GCode::_do_export` uses `outer_wall_acceleration` for calibration lines — the port has no calibration-line emission; confirm absent at implementation (one grep) and leave it out.
- `[FWD]` First-layer index source: confirm `global_layer_index == 0` (not a local index) is the emitter-visible layer-0 test at the call site; if the emitter sees only a local index, carry the global through the existing layer context rather than inventing a field.
- None blocking: no `[BLOCK]` — scope, seams, and verification are decidable from the tree as cited.
