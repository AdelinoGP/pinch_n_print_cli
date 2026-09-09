# Design: 293-speed-other-layers-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::resolve_feedrate` (`crates/slicer-gcode/src/emit.rs`) — the base-speed match already exposing every `ExtrusionRole` arm; the internal-solid arm is a one-line reseat there. The small-perimeter arm sits at the per-entity `F` resolution site in the same `emit_gcode` loop (`resolve_feedrate`'s caller chain, positioned per packet 291's precedent — after role-speed selection and after 291's blend, before the filament cap), where `entity.path.role` + `entity.path.points` + `entity.path.is_closed()` are all in scope. No E computation is touched.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` owns the `emit_gcode` multi-role fixture pattern (`LayerCollectionIR` default + `ExtrusionPath3D` two-point paths, `new_with_config` + `FeedrateConfig::from_raw_config` composition, `extrusion_path3d_base`/`print_entity_base` helpers) and the `raw_config_source_wires_feedrate_table_into_emitted_f_values` production-path shape — the new `speed_p60_other_layers_emission_tdd.rs` guard clones that shape, adding closed wall loops (explicit closing repeat per the `is_closed` convention) with measured planar lengths; `crates/slicer-gcode/tests/pressure_advance_emission_tdd.rs` owns the `with_resolved_config` + per-command-scan pattern AC-2's config-line count reuses; `docs/config/host-keys.toml` `[speeds]` + `[resolved_config]` rows + `cargo xtask gen-config-docs` output.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The arms live in the existing owner (`crates/slicer-gcode`) at the existing emission seam — not as host-side special cases outside the emitter, not as module constants, and not in `machine-gcode-emit` (its generic `[key]` sweep publishes placeholders; motion speeds are the host emitter's job — ticket-27 hazard checked, owner stands).
- Rule-4 trigger test does not fire: the two keys parameterise one emitter over roles and loop geometry it already sees (`entity.path.role`, entity points, `GcodeFlavor`-free pure `F` values); they do not select across alternative algorithm implementations, so no `claim:*` holders and no `*_fill_holder` selection.
- Stream position: no command lines are emitted on any path — both arms scale `F` values only. There is no envelope to follow and no dedup state to keep (unlike packets 289/292's per-path M-lines); 291's blend is the only position neighbour, and the small-perimeter arm wraps its output (blend first, gate second — re-read 291's `packet.spec.md` position at implementation, draft, do not freeze).
- Determinism: internal-solid selection is a pure function of (`entity.path.role`, feedrate table) — no ordering dependence, no cross-entity state. Small-perimeter selection is a pure function of (role, closed-ness, planar length, threshold, speed value, live outer speed) — same.
- Length is measured in mm straight from entity points (`Point3WithWidth.x/y`), summing planar segment lengths including the closing edge (the closing repeat contributes zero) — no mm↔unit conversion, no IR unit-system touch.
- Schema/version constants and event-specific locking: none bumped (no `PROGRESS_EVENT_SCHEMA_VERSION`, wire-version, or IR-version touch; host-only `SPEED_KEYS` + `ResolvedConfig` scalar additions are additive and the default F stream is byte-identical, pinned by AC-2).

## Code Change Surface

- Selected approach: one `SPEED_KEYS` row + `ResolvedConfig` twin for internal-solid (sparse-twin precedent); one percent-capable `ResolvedConfig` pair for the small-perimeter speed + threshold (ticket-107 machinery precedent); a one-line reseat in the base-speed match; a small gate helper at the per-entity `F` site; host-keys rows; new TDD guard. Internal-solid ships its live value through `to_config_map`; the small-perimeter pair stays host-only omitted (ticket-42 precedent); zero padding-table edits.
- Exact functions, traits, manifests, tests, and fixtures:
  - `FeedrateConfig` table (`crates/slicer-ir/src/feedrate.rs`, `SPEED_KEYS` + `SPEED_META` + `SPEED_BOUNDS` + `SPEED_KEY_COUNT` + `Default`): one row `("internal_solid_infill_speed", |fc| &mut fc.internal_solid_infill_speed)` appended in table position with `None` meta (Orca identity — a label here would never be seen on the fork) and `(1.0, None)` bounds (canonical `min 1`, no canonical max — ticket-113 precedent); one field `pub internal_solid_infill_speed: f32` with `Default 100.0` (canonical default; equals the sparse default it previously shadowed, so the reseat is stream-identical at defaults). Positional-alignment const asserts (`SPEED_KEYS.len() == SPEED_META.len() == SPEED_BOUNDS.len()`) must be extended in the same edit — the Step-1 dispatch owns them.
  - `ResolvedConfig` declaration (`crates/slicer-ir/src/resolved_config.rs`, `declare_resolved_config!` invocation + hand-written `to_config_map`): `cli "internal_solid_infill_speed" internal_solid_infill_speed: f32 = 100.0 => extract_float_or_first` (first-wins scalar ingest of Orca vector spellings, ticket-140 precedent) + its `to_config_map` arm (sparse-twin precedent — the module filter passes declared keys through, the CONFIG_BLOCK renders the raw map); `cli "small_perimeter_speed" small_perimeter_speed: ResolvedFloatOrPercent = ResolvedFloatOrPercent { value: 50.0, is_percent: true } => extract_float_or_percent` (ticket-107 `support_threshold_overlap` precedent — percent-capable with canonical `50%`-percent default) + `cli "small_perimeter_threshold" small_perimeter_threshold: f32 = 0.0 => extract_float` (scalar twin; the module-side `classic-perimeters` twin is a different input — DEV-185(c) — and is not touched, not aliased, not redeclared). Re-derive the `ResolvedFloatOrPercent` literal spelling and the `extract_float_or_percent` row syntax from the `support_threshold_overlap` precedent at implementation — do not freeze macro syntax here.
  - `resolve_feedrate` reseat (`crates/slicer-gcode/src/emit.rs`): `ExtrusionRole::InternalSolidInfill => self.feedrate_config.internal_solid_infill_speed` (replacing the sparse mapping + its two-line comment). No other arm moves.
  - Small-perimeter gate (new helper + call, `crates/slicer-gcode/src/emit.rs` at the per-entity `F` site): `small_perimeter_f(role, points, is_closed, threshold, speed_value, outer_speed) -> Option<f32>` returning `Some` only when `threshold > 0.0` AND `role.is_loop()` AND `is_closed()` AND `planar_length(points) <= threshold as f64 * 2.0 * PI`; the `Some` value is `0`-as-auto → `outer * 0.5`, percent → `get_abs_value`-resolved against live `outer_wall_speed`, absolute → as-is. The caller applies it as an override of the blended role base (never composed — ADR-0052 fallback-not-composition, the 289/291 precedent). `planar_length` sums `hypot(dx, dy)` over consecutive points in mm. canonical `speed == -1` entry condition has no port analogue (every entity carries a base `F`) — recorded, not stubbed.
  - Bounds: `min 1` on internal-solid, `min 0` on the small-perimeter speed + threshold (reject-the-slice, DEV-185(a)) — seeded into the ticket-113 index via the `speed_bounds()` seam for the `SPEED_KEYS` row and the existing `ResolvedConfig` bounds path for the pair.
  - Schema rows: `docs/config/host-keys.toml` `[speeds]` gains `internal_solid_infill_speed`; `[resolved_config]` gains `small_perimeter_speed` + `small_perimeter_threshold` (the internal-solid `ResolvedConfig` twin rides the `[speeds]` row — sparse-twin precedent, no `[resolved_config]` row); generated `docs/15_config_keys_reference.md` regened in Step 1b.
  - Tests: new `crates/slicer-gcode/tests/speed_p60_other_layers_emission_tdd.rs` (schema guard AC-1, near-identity AC-2, independence AC-3, gate+arms AC-4, bounds AC-N1, silence AC-N2).
- Rejected alternatives and reasons:
  - Wiring the rectilinear module's parsed `internal_solid_infill_speed` into its `speed_factor`: rejected — the module hardcodes 1.0 under the ticket-114 host-owns-speed contract; the host reseat is the only implementation and the parsed tuple stays dead. A dual implementation would reintroduce the double-count ticket 114 retired.
  - Declaring the small-perimeter pair on `classic-perimeters` and threading a per-path factor through production: rejected — canonical's gate is emission-time over the clipped loop, the port emits speeds in exactly one place, and production modules own geometry, not `F` values; the existing module threshold is a width-classification input, not a speed gate (DEV-185(c)).
  - Merging the gate length test into `classify_narrow_island` (opening + bbox rule): rejected — different predicate (planar loop length vs morphological opening), different subject (one emitted loop vs the whole region polygon set), different consumer (emitter `F` vs wall-width override). Sharing it would couple two features that share only a canonical name family.
  - Publishing either key via the `machine-gcode-emit` placeholder sweep (speeds in start G-code): rejected — wrong seam (placeholders render once at print start; canonical re-selects per path; ticket-27 hazard).
  - Adopting per-nozzle vectors now: rejected — canonical's `NOZZLE_CONFIG` vectors need the ticket-125 model; scalar-global matches packets 276/277/279–292 (DEV-185(b)).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-ir/src/feedrate.rs` - role: `SPEED_KEYS` row + field + default + meta + bounds; expected change: one table row + one field + one default + aligned meta/bounds entries.
- `crates/slicer-ir/src/resolved_config.rs` - role: declare the three fields + `to_config_map` arm for internal-solid; expected change: macro-invocation rows + one map arm + bounds entries.
- `crates/slicer-gcode/src/emit.rs` - role: internal-solid reseat + small-perimeter gate helper + per-entity call + bounds hookup; expected change: ~30 lines staged speed selection with role/closed/length/threshold branches.
- `crates/slicer-gcode/tests/speed_p60_other_layers_emission_tdd.rs` (new) - role: AC-1–AC-N2 pins; expected change: net-new guard binary, no registration needed (auto-discovered).
- `docs/config/host-keys.toml` - role: `[speeds]` + `[resolved_config]` rows; expected change: three rows (extra justified: schema source of truth, one-line rows).
- `docs/DEVIATION_LOG.md` - role: DEV-185 row; expected change: one row with (a)+(b)+(c) clauses (extra justified: preflight-visible obligation).
- `docs/15_config_keys_reference.md` - role: generated output; expected change: regen only, via `cargo xtask gen-config-docs` (extra justified: generated, not hand-edited).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-gcode/src/emit.rs` - lines covering `DefaultGCodeEmitter::resolve_feedrate` base-speed match + the per-entity `F` resolution site only - purpose: reseat anchor + gate call position (delegate a LOCATIONS fix before reading).
- `crates/slicer-ir/src/resolved_config.rs` - lines covering one `extract_float_or_first` scalar row + the `support_threshold_overlap` `ResolvedFloatOrPercent` row + the sparse `to_config_map` arm only - purpose: field-declaration syntax models.
- `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` only (`ExtrusionRole` variants) + `is_closed` + `is_loop` - purpose: gate role qualification spelling.
- `crates/slicer-runtime/src/run.rs` - lines covering the `FeedrateConfig::from_raw_config` + `with_resolved_config` composition only - purpose: table-vs-twin wiring confirmation (no change; the packet adds no new source).
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` - `[config.schema.small_perimeter_threshold]` block only - purpose: twin-non-interference evidence (read, never edit).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - read-only reference at most (zero twins for all three spellings; table untouched — rule 2)
- `crates/slicer-gcode/src/estimator.rs` - read-only context at most (it times whatever the arm leaves; no change — the reseated/gated stream flows in automatically)
- `modules/core-modules/rectilinear-infill/` - out of bounds (parsed tuple stays dead by design; the host arm is the only implementation — do not touch)
- `modules/core-modules/classic-perimeters/` - out of bounds (width-classification twin untouched — read the toml block as evidence only)
- `modules/core-modules/machine-gcode-emit/` - out of bounds (wrong seam for per-move speeds; generic sweep untouched)
- `docs/spec_packets/281-machine-motion-limits-emitter/` - no edge in this packet — reference only if stream position ever matters, never edit
- `docs/spec_packets/289-speed-acceleration-emitter/` - position-adjacent precedent (per-entity stage shape) — reference only, never edit
- `docs/spec_packets/291-slow-down-layers-initial-layer-speed/` - blend-position neighbour (gate wraps the blend) — reference only, never edit
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: base-speed match anchor + per-entity `F` call chain (role selection → 291-blend → filament cap order) + last-`F`-site spelling in `emit.rs`; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each); purpose: Step 2 reseat + gate positioning.
- Question: `declare_resolved_config!` row syntax for one `extract_float_or_first` scalar + the `ResolvedFloatOrPercent` row + the sparse `to_config_map` arm; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤3 snippets, ≤30 lines each); purpose: Step 1 field declaration.
- Question: struct-literal blast radius of adding one `FeedrateConfig` field + two `ResolvedConfig` fields (every test/non-test literal compiling against each struct + every test hard-asserting related defaults); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 edit-list completeness.
- Question: `cargo xtask gen-config-docs` invocation + `--check` gate spelling; scope: `xtask/src/gen_config_docs.rs`; return: `FACT` (≤5 lines); purpose: Step 1b regen.

## Data and Contract Notes

- IR/manifest contracts: no IR field, no WIT accessor, no manifest `[config.schema]` table for modules (host-only speeds + one module-visible twin via the existing `to_config_map` path); `docs/config/host-keys.toml` is the schema source of truth for all three spellings.
- WIT boundary: untouched (no guest-visible key; no rebuild; `cargo xtask build-guests --check` not required at authoring — host-only prose, ticket-60 precedent).
- Determinism/scheduler constraints: both arms are per-entity pure; no claim, priority, or ordering interaction; `default_priority` untouched; no dedup state (F values, not command lines).

## Locked Assumptions and Invariants

- At defaults the F stream is byte-identical (internal-solid 100 == sparse 100 it previously shadowed; threshold 0 silences the small-perimeter arm) — the near-identity invariant; AC-2 pins it plus the exactly-+1 CONFIG_BLOCK twin line.
- Canonical scalarity is deliberately NOT held: canonical declares both keys per-nozzle vectors; scalar-global is the recorded port simplification (DEV-185(b)), consistent with packets 276/277/279–292 — no ticket-125 vector arm in this packet.
- The small-perimeter gate is wall-loops-only by construction: `OuterWall`/`InnerWall`/`ThinWall` closed loops at or under the circumference gate resolve through the small-perimeter value; every other role keeps its role `F` at any threshold (DEV-185(c)); a value of `0` means auto (`outer * 0.5`), never "stop".
- The module-side `small_perimeter_threshold` (classic-perimeters width classification) and the host-side twin (this packet's gate circumference input) are deliberately separate inputs sharing a canonical name family — neither reads the other, and unifying them is explicitly out of scope (DEV-185(c)).
- The `speed == -1` entry condition has no port analogue and is not stubbed: every entity carries a base `F`, so the gate qualifies on (threshold, closed-ness, length, role) alone.

## Risks and Tradeoffs

- F-only packet in an M-line neighbourhood: packets 289/292 emit command lines with dedup state, this packet scales `F` values with none — a later merge that mistakes the gate helper for a command site would emit phantom lines. The helper returns an `F` override (`Option<f32>`), never a command, by construction.
- Blend-order coupling: the gate must wrap 291's blended base, not the pre-blend role speed — a first-layer small loop at `slow_down_layers = 3` resolves the small-perimeter value, not a blend of it. AC-4 pins small-loop `F` against the live outer speed; 291's position is re-read at implementation so a landed 291 cannot silently reorder.
- Percent-base coupling: the percent form resolves against the *live* `outer_wall_speed` (the same table the walls emit), not a frozen default — an `outer_wall_speed = 30` profile halves every percent-derived small-loop `F` with it. AC-4 pins the coupling at defaults; non-default-base behaviour is recorded, not separately pinned.
- Bounds-as-divergence: canonical warns-and-caps (`Print::validate`); the port rejects per ticket 113 — the same deliberate divergence packets 281/282/289/292 record (DEV-185(a)); a user value canonical would clamp now fails the slice loudly.
- First-wins vector ingest: an Orca 3MF spelling `internal_solid_infill_speed` as a per-nozzle list resolves to element 0 (the `extract_float_or_first` contract); multi-nozzle prints lose the other elements until ticket 125 lands (DEV-185(b)).

## Context Cost Estimate

- Aggregate: `M` (never L)
- Largest step: `M` (Step 2 reseat + gate + percent resolution + bounds)
- Highest-risk dispatch and required return format: base-speed match anchor + per-entity `F` call chain order (`LOCATIONS`, ≤10 entries) — mispositioning the gate (inside the base-speed match, or before the blend, or after serialization) silently gates nothing or double-scales `F`.

## Open Questions

- `[FWD]` Small-support sibling pair if the queue ever carries it: support entities would gain their own threshold + percent-over-support-base closure mirroring this gate's shape over `SupportMaterial`/`SupportInterface` roles. Not in P60 — do not "complete" this packet by declaring it.
- `[FWD]` Minimum-cross-section guard if the estimator ever needs it: canonical collects `min_mm3_per_mm` over zero-speed roles to floor the volumetric rate. The port's floor derives from rendered geometry; re-check only if a zero-`F` defect is ever observed, not pre-emptively.
- None blocking: no `[BLOCK]` — scope, seams, and verification are decidable from the tree as cited.
