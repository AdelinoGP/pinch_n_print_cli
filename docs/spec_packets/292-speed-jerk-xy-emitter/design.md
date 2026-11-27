# Design: 292-speed-jerk-xy-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — the per-entity loop already exposing `entity.path.role` and `global_layer_index`. The new stage selects one jerk per print entity via the canonical precedence chain and emits through the existing `GcodeFlavor::set_jerk_xy` arm; travels emit `travel_jerk` through the same arm on the travel path; the first-layer junction-deviation line renders through the existing `GcodeFlavor::set_junction_deviation` arm. No E computation is touched.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` owns the `emit_gcode` multi-layer fixture pattern (`LayerCollectionIR` with distinct `global_layer_index` values, `new_with_config` + `with_resolved_config` composition) — the new `speed_p59_jerk_emission_tdd.rs` guard clones that shape; `crates/slicer-gcode/tests/pressure_advance_emission_tdd.rs` owns the `with_resolved_config` + per-command-scan pattern this packet's M205-comparison assertions reuse; `docs/config/host-keys.toml` `[resolved_config]` rows + `cargo xtask gen-config-docs` output.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The stage lives in the existing owner (`crates/slicer-gcode`) at the existing emission seam — not as a host-side special case outside the emitter, not as module constants, and not in `machine-gcode-emit` (its generic `[key]` sweep publishes placeholders; motion commands are the host emitter's job — ticket-27 hazard checked, owner stands).
- Rule-4 trigger test does not fire: the eight keys parameterise one emitter over roles/flavors it already sees (`entity.path.role`, `GcodeFlavor`, `global_layer_index`); they do not select across alternative algorithm implementations, so no `claim:*` holders and no `*_fill_holder` selection.
- Stream position: 281's envelope (draft) opens the stream ahead of the start block; this packet's per-path lines follow it in canonical position (after the envelope, interleaved with moves the way canonical interleaves `set_jerk_xy` with extrusion). No reorder of 281's envelope order.
- Determinism: selection is a pure function of (`entity.path.role`, `global_layer_index`, travel-vs-print, resolved keys + flavor) — no ordering dependence, no cross-entity state; the master gate short-circuits the whole stage.
- Change-dedup is per-stream monotonic with separate print/travel keys (last-emitted print jerk, last-emitted travel jerk, last-emitted JD) so parallel emission order cannot disturb it — the emitter is single-stream. A same-value entity emits no repeat line (packet-289 precedent, borrowed shape — not a shared helper).
- The `default_junction_deviation` line renders once per first-layer print entity at most (deduped like jerk), and only when the flavor arm returns `Some` — the `None` on Marlin/Klipper/RepRapFirmware/Repetier is the port's existing Marlin2-only contract, not a packet decision (DEV-184(c)).
- Schema/version constants and event-specific locking: none bumped (no `PROGRESS_EVENT_SCHEMA_VERSION`, wire-version, or IR-version touch; host-only `ResolvedConfig` float fields are additive and the default-path impact is byte-identical, pinned by AC-2).

## Code Change Surface

- Selected approach: eight scalar-global `ResolvedConfig` fields (canonical defaults; `max 0.3` on junction deviation) + one pure selection helper + jerk-emit call sites on the print/travel paths + first-layer JD arm + bounds arms in the existing validator + host-keys rows + new TDD guard. Host-only omitted from `to_config_map` module visibility and from the CONFIG_BLOCK (ticket-42 precedent — emission-control scalars, not module inputs; zero padding twins).
- Exact functions, traits, manifests, tests, and fixtures:
  - `ResolvedConfig` declaration (`crates/slicer-ir/src/resolved_config.rs`, `declare_resolved_config!` invocation): 8× `f32` scalars (`extract_float_or_first @ { min/max }` — the ticket-140 first-wins precedent for Orca vector spellings), with CLI ingestion (first-wins scalar) and bounds arms seeded into the ticket-113 index. Defaults: `default_jerk = 0.0`, `default_junction_deviation = 0.0`, the five role jerks `= 9.0`, `travel_jerk = 12.0`.
  - `select_jerk(role, layer_index, cfg) -> f32` (new, `crates/slicer-gcode/src/emit.rs` or sibling): canonical precedence — `global_layer_index == 0 && initial_layer_jerk > 0` first; else `OuterWall`/`ThinWall` → outer; `InnerWall` → inner; `TopSolidInfill` → top-surface; `SparseInfill` → infill; every other role (`BottomSolidInfill`, `GapFill`, `BridgeInfill`/`InternalBridgeInfill`, support roles, `Skirt`, `Brim`) → `default_jerk`; any selected arm at `0` falls through to `default_jerk`. Re-derive the exact `ExtrusionRole` variant spellings at implementation from `crates/slicer-ir/src/slice_ir.rs` (`ExtrusionRole` range) — do not freeze variant names here beyond the 289/291 precedents (`OuterWall`, `InnerWall`, `ThinWall`, `TopSolidInfill`, `SparseInfill`, `BottomSolidInfill`, `GapFill`, `Skirt`, `Brim`; the bridge/support spellings are re-derived then, and any missing variant is a Step-2 finding, not a Step-3 surprise).
  - Call sites (print path + travel path in the per-entity loop, `crates/slicer-gcode/src/emit.rs`): when `default_jerk > 0` (master gate — canonical `> 0`, AC-N2) — on print entities render `set_jerk_xy(selected)` through the flavor with print-key dedup; on travel entities render `set_jerk_xy(travel_jerk)` with travel-key dedup (flat on every layer — no `initial_layer_travel_jerk` arm exists); on first-layer (`global_layer_index == 0`) print entities with `default_junction_deviation > 0`, render `set_junction_deviation(jd)` when the flavor arm returns `Some` (Marlin2-only; other flavors emit jerk only — no fallback form is invented). No `M566` emission on any path (no `M566` builder exists in `GcodeFlavor` — verified at authoring; machine-max jerks are 281's envelope, DEV-184(d)).
  - Bounds: `min 0` on all eight jerks (reject-the-slice, DEV-184(a)); `min 0, max 0.3` on junction deviation (canonical `max = 0.3f`, the only canonical max in scope).
  - Schema rows: `docs/config/host-keys.toml` `[resolved_config]` gains eight rows; generated `docs/15_config_keys_reference.md` regened in Step 1b.
  - Tests: new `crates/slicer-gcode/tests/speed_p59_jerk_emission_tdd.rs` (schema guard AC-1, identity AC-2, role chain AC-3, travel AC-4, flavor+dedup AC-5, JD AC-6, bounds AC-N1, master gate AC-N2).
- Rejected alternatives and reasons:
  - Declaring the jerks on wall/infill modules and threading them through production: rejected — canonical's decision point is emission-time and the port emits motion commands in exactly one place; production modules own geometry, not M205.
  - Publishing the keys via the `machine-gcode-emit` placeholder sweep (M205 in start G-code): rejected — wrong seam (placeholders render once at print start; canonical re-selects per path; ticket-27 hazard).
  - Wiring the short-travel `outer_wall_jerk` override, the `initial_layer_travel_jerk` percent override, or the Calib-PA-line `outer_wall_jerk` use now: rejected — no port-side travel-length/index seam at the travel call site and no Calib-PA header in this tree; wiring a branch on unobservable state would be declaration-only; named non-borrows with `[FWD]` re-checks (DEV-184(d)).
  - Adopting per-nozzle vectors now: rejected — canonical's `NOZZLE_CONFIG` vectors need the ticket-125 model; scalar-global matches packets 276/277/279–291 (DEV-184(b)).
  - Restoring jerk statefully on layer transitions (canonical's second-layer `default_jerk` restore): rejected — the port re-selects per entity, so no restore state exists to maintain (DEV-184(c)).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-ir/src/resolved_config.rs` - role: declare the eight fields; expected change: macro-invocation rows + CLI arms + bounds entries.
- `crates/slicer-gcode/src/emit.rs` - role: selection helper + print/travel/JD call sites + bounds hookup; expected change: ~60 lines staged jerk selection/emission with role/flavor/layer/master-gate branches.
- `crates/slicer-gcode/tests/speed_p59_jerk_emission_tdd.rs` (new) - role: AC-1–AC-N2 pins; expected change: net-new guard binary, no registration needed (auto-discovered).
- `docs/config/host-keys.toml` - role: `[resolved_config]` rows; expected change: eight rows (extra justified: schema source of truth, one-line rows).
- `docs/DEVIATION_LOG.md` - role: DEV-184 row; expected change: one row with (a)+(b)+(c)+(d) clauses (extra justified: preflight-visible obligation).
- `docs/15_config_keys_reference.md` - role: generated output; expected change: regen only, via `cargo xtask gen-config-docs` (extra justified: generated, not hand-edited).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-gcode/src/emit.rs` - lines covering `DefaultGCodeEmitter::emit_gcode` per-entity loop + travel emission only - purpose: selection/emission anchor + change-dedup position (delegate a LOCATIONS fix before reading).
- `crates/slicer-gcode/src/flavor.rs` - lines 99–118 only - purpose: `set_jerk_xy` / `set_junction_deviation` signatures to reuse (do not fork).
- `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` float row syntax + one `extract_float_or_first` scalar precedent only - purpose: field-declaration syntax model.
- `crates/slicer-ir/src/slice_ir.rs` - lines covering the `ExtrusionRole` variants only - purpose: role-variant spelling for the mapping arms.
- `crates/slicer-runtime/src/run.rs` - lines covering the `gcode_flavor` → `with_flavor` wiring only - purpose: flavor-selection confirmation (no change; the packet adds no new flavor path).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - read-only reference at most (zero twins for the keys; table untouched — rule 2)
- `crates/slicer-gcode/src/estimator.rs` - read-only context at most (it times whatever the arm leaves; no change — the jerked stream flows in automatically)
- `modules/core-modules/machine-gcode-emit/` - out of bounds (wrong seam for per-move motion commands; generic sweep untouched)
- `modules/core-modules/path-optimization-default/src/lib.rs` - cite as travel-shape context only; do not edit
- `docs/spec_packets/281-machine-motion-limits-emitter/` - gate producer (stream position) — reference only, never edit
- `docs/spec_packets/289-speed-acceleration-emitter/` - position-adjacent precedent (dedup shape) — reference only, never edit
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: per-entity loop anchor + travel-emission site + last-emitted-state spelling in `emit.rs`; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each); purpose: Step 2 call-site positioning.
- Question: `declare_resolved_config!` float row syntax + CLI arm pattern for one `extract_float_or_first` scalar precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each); purpose: Step 1 field declaration.
- Question: struct-literal blast radius of adding eight `ResolvedConfig` fields (every test/non-test literal compiling against the struct + every test hard-asserting related defaults); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 edit-list completeness.
- Question: `cargo xtask gen-config-docs` invocation + `--check` gate spelling; scope: `xtask/src/gen_config_docs.rs`; return: `FACT` (≤5 lines); purpose: Step 1b regen.

## Data and Contract Notes

- IR/manifest contracts: no IR field, no WIT accessor, no manifest `[config.schema]` table for modules (host-only scalars); `docs/config/host-keys.toml` is the schema source of truth for the eight.
- WIT boundary: untouched (no guest-visible key; no rebuild; `cargo xtask build-guests --check` not required at authoring — host-only prose, ticket-60 precedent).
- Determinism/scheduler constraints: selection is per-entity pure; no claim, priority, or ordering interaction; `default_priority` untouched; change-dedup is per-stream monotonic (separate print/travel/JD keys) so parallel emission order cannot disturb it — the emitter is single-stream.

## Locked Assumptions and Invariants

- At defaults the stage emits nothing (no M205/M207/SET_VELOCITY_LIMIT/J line) — the whole-stream identity invariant; AC-2 pins it (the inverse of 289's emitting default).
- Canonical scalarity is deliberately NOT held: canonical declares all eight keys per-nozzle vectors; scalar-global is the recorded port simplification (DEV-184(b)), consistent with packets 276/277/279–291 — no ticket-125 vector arm in this packet.
- The five role mappings (`OuterWall`/`ThinWall` → outer, `InnerWall` → inner, `TopSolidInfill` → top-surface, `SparseInfill` → infill, first-layer → initial) plus the six fallthrough roles (`BottomSolidInfill`, `GapFill`, bridge pair, support roles, `Skirt`/`Brim`) are recorded in AC-3, not silent extensions; a role at `0` falls through to default.
- Travels use `travel_jerk` on every layer (no first-layer percent override, no short-travel override — both named non-borrows, DEV-184(d)); no `M566` line is emitted on any path (no builder exists; machine-max jerks are 281's envelope).
- The JD line appears on first-layer print paths only, and only where the flavor arm returns `Some` (Marlin2) — other flavors emit jerk only, with no invented fallback form (DEV-184(c)).

## Risks and Tradeoffs

- Dedup-order coupling: print and travel dedup keys must stay separate — a travel at jerk 12 must not suppress the next print at jerk 12 or vice versa; AC-5 pins the separation. The JD key is a third independent key (a JD-only change must still emit its line).
- Estimator coupling: the jerked stream feeds the estimator, but jerk lines carry no time model there — estimated time is unchanged by enabling the key (unlike packet 291's speed blend, which changes F values); ACs pin only the command lines, not estimate deltas, to keep the contract narrow.
- Flavor-form drift: the packet reuses `set_jerk_xy` / `set_junction_deviation` verbatim — if a later packet changes those arms' forms, this packet's AC-5/AC-6 expectations move with them (queue-order merge churn, not a second form).
- Bounds-as-divergence: canonical warns-and-caps (`Print::validate`); the port rejects per ticket 113 — the same deliberate divergence packets 281/282/289 record (DEV-184(a)); a user value canonical would clamp now fails the slice loudly.
- First-wins vector ingest: an Orca 3MF spelling a jerk key as a per-nozzle list resolves to element 0 (the `extract_float_or_first` contract); multi-nozzle prints lose the other elements until ticket 125 lands (DEV-184(b)).

## Context Cost Estimate

- Aggregate: `M` (never L)
- Largest step: `M` (Step 2 selection stage + flavor rendering + bounds)
- Highest-risk dispatch and required return format: per-entity loop anchor + travel-emission site + last-emitted-state spelling (`LOCATIONS`, ≤10 entries) — mispositioning the stage (inside the base-speed match, or after serialization, or on the wrong path) silently jerks nothing or jerks travels as prints.

## Open Questions

- `[FWD]` Short-travel `outer_wall_jerk` override if the travel call site ever exposes travel length + nearby role: the override needs `travel.length() < retraction_minimum_travel` plus the external/overhang role test — neither observable at the call site today. Re-check when a travel-index seam lands.
- `[FWD]` `initial_layer_travel_jerk` if ticket 123's source audit queues it: first-layer travels would resolve the percent-over-`travel_jerk` base via the port's percent machinery (ticket-107 precedent) instead of today's flat value. Not in P59 — do not "complete" this packet by declaring it.
- `[FWD]` Calib-PA-line `outer_wall_jerk` if a pressure-advance calibration header ever emits in this tree: the header would select the outer-wall jerk the way canonical's `Calib_PA_Line` does. No such header exists today.
- None blocking: no `[BLOCK]` — scope, seams, and verification are decidable from the tree as cited.
