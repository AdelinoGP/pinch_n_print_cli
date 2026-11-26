# Design: 285-seam-scarf-joint-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` per-entity loop (`crates/slicer-gcode/src/emit.rs`) → new scarf/slope stage → existing `Move`/`Extrusion` rendering; speed selection via the existing `DefaultGCodeEmitter::resolve_feedrate` role seam (same file), which already maps custom `"Wipe"` to `FeedrateConfig::wipe_speed` (`crates/slicer-ir/src/feedrate.rs`).
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (existing emit driver — regression home), `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` (role-feedrate precedent), new `crates/slicer-gcode/tests/seam_scarf_joint_emission_tdd.rs` (all eight ACs).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- No guest/WASM surface: all eight decision points are host-emitter-internal; no manifest-driven module reads, so no guest rebuild rides this packet (the `machine-gcode-emit.toml` rows are schema surface only — that module never reads these keys through `ConfigView::from_declared`).
- No IR/WIT/schema change: scarf is an emitter-internal point-vector transform; no new field, variant, or version constant (S3 vacuous — no `*_SCHEMA_VERSION` is named anywhere in this packet).
- No new ADR: the stage conforms to ADR-0062/0063 (below); S4 vacuous.
- ADR-0062/0063 conformance (no amendment): the shipped lock implementation (`remap_infill_order_locks_from` / `next_global_infill_tag` / `validate_infill_order_locks` in `crates/slicer-runtime/src/layer_executor.rs`) makes locked paths self-clipping — the producer guarantees the swept footprint and downstream stages neither clip nor reshape it. This packet therefore bypasses EVERY path carrying `order_lock` around both the `seam_gap` clip and the scarf transform (AC-N2), regardless of role. No `ADR-AMENDED` deviation is filed (ticket-33 precedent: closing as conformance).
- Rule-4 shape (PnP way): scarf stays an emitter-internal path transform over the seam the placer's `LayerModule::run_wall_postprocess` already chose (`modules/core-modules/seam-placer/src/lib.rs`) — not a module, not a claim. Rejected: a `claim:seam-finish` holder family (Q8 trigger test fails — scarf is one emitter move-shape, not cross-module algorithm selection).

## Code Change Surface

- Selected approach: eight scalar-global `ResolvedConfig` fields (canonical defaults/bounds) declared via the existing `declare_resolved_config!` syntax (`crates/slicer-ir/src/resolved_config.rs`); seven are deliberately OMITTED from `to_config_map` (host-only emission control — the emitter reads the typed fields directly and modules never see these keys; ticket-42 P35 precedent), while `seam_gap` IS carried by a `to_config_map` arm in the same file so its live value shadows the `("seam_gap", "10%")` padding twin — `resolved_config_to_map` (`crates/slicer-gcode/src/serialize.rs`) delegates straight to `to_config_map`, and the serializer's emitted-set dedups padding against live keys (284's `resolution` precedent exactly). Plus `machine-gcode-emit.toml` schema tables (ticket-04 contract surface; the manifest default is dead for this class — the macro default rules, stated in every table comment) and `docs/config/host-keys.toml` rows. Behaviour (all in `emit.rs`): gate-ordered stage — (1) `order_lock` bypass, (2) `seam_gap` clip + slope termination, (3) `has_scarf_joint_seam` master gate, (4) `seam_slope_conditional` smoothness/overhang gates, (5) scarf overlap emission with flow/speed modulation, (6) `role_based_wipe_speed` feedrate selection on draft 277's wipe `Move` (FORWARD-DEP).
- Key table (canonical types/defaults/bounds grounded 2026-09-07 against the map oracle):

  | key | type | default | bounds | percent/absolute base |
  | --- | --- | --- | --- | --- |
  | `has_scarf_joint_seam` | bool | `false` | — | — (port-side master enable gate — PnP role; canonical reads it only in the viewer path, generation selection lives with P53's slope keys: deliberate divergence DEV-177(a), see `requirements.md`) |
  | `seam_slope_conditional` | bool | `false` | — | — (restriction switch) |
  | `role_based_wipe_speed` | bool | `true` | — | — (wipe feedrate source; FORWARD-DEP on 277) |
  | `scarf_angle_threshold` | int | `155` | `[0, 180]` (°; converted to radians once at emission) | — |
  | `scarf_joint_flow_ratio` | float | `1.0` | `[0, 2]` | applied via the scarf segments' `flow_factor` at creation (ticket-57 lesson: no emitter role multiplier — E already scales through `point.flow_factor`) |
  | `scarf_joint_speed` | float-or-percent | `100%` | `min 1` | percent of the emitting role speed; bare number = absolute mm/s (×60 feedrate). NOT added to `SPEED_KEYS` (ticket-109 precedent: selective/relative semantics stay out; the `min 1` bound is enforced at this packet's own gate instead) |
  | `scarf_overhang_threshold` | percent | `40%` | `min 0` | percent of wall line width (unsupported-overhang estimate below it keeps scarf) |
  | `seam_gap` | float-or-percent | `10%` | `min 0` | percent of nozzle diameter (canonical `get_abs_value(nozzle_diameter)`); the port reads the `extensions` scalar `nozzle_diameter` (ticket-118 measurement — it is NOT a `ResolvedConfig` field), absolute = mm. Carried in `to_config_map` (same file as the fields) so the live value shadows the padding twin via the serializer's emitted-set dedup |
- Scarf overlap length = the resolved `seam_gap` distance (self-contained: the clip opens the gap, the tapered overlap bridges it; slope termination consumes the same distance). Rejected alternative: waiting on P53's ramp-length params (`seam_slope_steps`, `seam_slope_min_length`, …) — that would strand all eight keys behind an unauthored packet for a length this stage can close itself; P53 generalises to multi-step ramps additively when authored (ticket 60 notes the extension; no edge added).
- `role_based_wipe_speed` reconciliation with draft 277 (FORWARD-DEP, names verified 2026-09-07 against `docs/spec_packets/277-retraction-wipe-travel-firmware-emitter/design.md`): 277 creates the wipe `Move` ("retraces the last extrusion vector reversed, length exactly `wipe_distance`") inside `emit.rs` next to the `wipe_speed`/`travel_speed` feedrate neighbours; this packet contributes exactly the feedrate-source selection — `true` → the emitting role's speed, `false` → `FeedrateConfig::wipe_speed` (default `96.0`) through the existing `resolve_feedrate` `"Wipe"` mapping. Same names, same file, same mapping — S1-acceptable. Default `true` changes no default output today (no wipe `Move` exists until 277 lands; the joint default is defined by both packets together).
- `seam_gap` Fill-side arm (canonical concentric-fill clipping via `Layer::make_fills`): unimplementable — the port has no concentric filler — recorded in DEV-177(b), never declared for any infill module. `has_scarf_joint_seam` viewer-recognition role (canonical `GCodeProcessor::apply_config`): no port counterpart (no viewer pipeline) — recorded in DEV-177(a) together with the flag's deliberate repurposing as the stage enable gate (generation selection properly lives with P53's slope keys, ticket 60); the wired role is the enable gate.
- Exact functions, traits, manifests, tests, and fixtures: `DefaultGCodeEmitter::emit_gcode` + `DefaultGCodeEmitter::resolve_feedrate` (`crates/slicer-gcode/src/emit.rs`) — extend at bounded sites (gates + overlap + validation); `declare_resolved_config!` field syntax + one `seam_gap` `to_config_map` arm (`crates/slicer-ir/src/resolved_config.rs` — fields and arm live in the same file; the other seven keys stay out of the map); `FeedrateConfig::wipe_speed` + `SPEED_KEYS` (`crates/slicer-ir/src/feedrate.rs`) — read the former, leave the latter untouched; `ORCA_CONFIG_PADDING` + `emit_config_kv` (`crates/slicer-gcode/src/serialize.rs`) — leave the table untouched (rule 2), live `seam_gap` shadows its twin via dedup; `machine-gcode-emit.toml` — eight `[config.schema.*]` tables; `docs/config/host-keys.toml` — eight `[resolved_config]` rows; `docs/DEVIATION_LOG.md` — DEV-177 row; new `crates/slicer-gcode/tests/seam_scarf_joint_emission_tdd.rs` (own `--test` binary; no aggregator exists under `crates/slicer-gcode/tests/` — verified no `main.rs` — so S7 needs no `mod` registration); re-baselined golden fixtures in Step 6 only.
- Rejected alternatives and reasons: per-module `seam_gap` declaration (dead under `ConfigView::from_declared` — ticket-34 shape — the readers are host-side); `SPEED_KEYS` membership for `scarf_joint_speed` (109 precedent); serializer-side clipping (ADR-0063 blindness — the serializer cannot see `order_lock`); claim-holder seam mode (Q8 fails); folding `role_based_wipe_speed` into P53 (it modifies 277's site, not P53's slope params — wrong home).

## Files in Scope (read + edit)

Four production files plus tests/docs — justified: the eight-key declaration surface is inherently three-file (typed field + schema table + config-doc row; the P35/P18 precedent shape), behaviour is one file (`emit.rs`); splitting would strand declarations from their only consumer.

- `crates/slicer-ir/src/resolved_config.rs` - role: eight field declarations via macro syntax; expected change: +8 fields with canonical defaults/bounds docs.
- `crates/slicer-gcode/src/emit.rs` - role: all six wired decisions (clip, gates, overlap, flow/speed, wipe-speed arm); expected change: bounded gate + helpers, never whole-file rewrites.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - role: eight schema tables (contract surface); expected change: append tables with dead-default comments.
- `docs/config/host-keys.toml` - role: eight `[resolved_config]` rows; expected change: append rows.
- Also edited (docs/tests): `docs/DEVIATION_LOG.md` (DEV-177 row), `docs/15_config_keys_reference.md` (generator output via `cargo xtask gen-config-docs`, Step 1b), `crates/slicer-gcode/tests/seam_scarf_joint_emission_tdd.rs` (new), golden fixtures re-baselined in Step 6 with measured justification.

## Read-Only Context

- `crates/slicer-gcode/src/serialize.rs` - relevant padding/dedup neighbourhood only - purpose: confirm `("seam_gap", "10%")` twin spelling + `emit_config_kv` dedup shape before asserting AC-2's shadow claim.
- `docs/spec_packets/277-retraction-wipe-travel-firmware-emitter/design.md` - wipe-site section only - purpose: re-confirm wipe-`Move` names/shape at implementation time (FORWARD-DEP reconciliation; ledger — re-derive, never trust this bullet if 277 has moved).
- `modules/core-modules/seam-placer/src/lib.rs` - `LayerModule::run_wall_postprocess` (trait-method impl) signature only - purpose: confirm the stage consumes (never moves) the placed seam.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `modules/core-modules/seam-placer/*`, `modules/core-modules/seam-planner-default/*` (upstream seam selection — consumed, not moved), any `modules/*/src/lib.rs` geometry (no module reads these keys), `docs/spec_packets/277-*/**` (read-only reconciliation above; never edit another packet's files)

## Expected Sub-Agent Dispatches

- Question: confirm the eight canonical defaults/bounds + `extrude_loop` gate order at implementation time; scope: `OrcaSlicerDocumented/src/libslic3r/`; return: `SUMMARY ≤200 words`; purpose: Steps 1–2 grounding refresh (the authoring-time grounding is in `requirements.md`; re-derive if the oracle moved).
- Question: which golden fixtures shift under live `seam_gap` clipping and by what measured delta; scope: `crates/slicer-gcode/tests/`; return: `LOCATIONS ≤20`; purpose: Step 6 re-baseline list.
- All cargo runs delegated with `tail`-filtered FACT returns (never full output in context).

## Data and Contract Notes

- IR/manifest contracts: none changed; the eight manifest tables are host-s schema surface (modules never read them — stated in each table comment so no future reader mistakes them for module inputs).
- WIT boundary: untouched.
- Determinism/scheduler constraints: the stage is a per-loop pure function of config + geometry (angle math, no RNG); no scheduler ordering change; wipe direction/shape stays 277's.

## Locked Assumptions and Invariants

- Defaults lock: `has_scarf_joint_seam = false` + `seam_slope_conditional = false` keep every scarf path inert at defaults; the ONLY default-path delta this packet introduces is the `seam_gap` clip (AC-2) — everything else is byte-identical at defaults by construction (gates, not blends).
- Scarf overlap length == resolved `seam_gap` distance (P53 generalises additively; this identity is the P52/P53 seam — P53 must preserve it for non-ramp loops).
- Locked paths are never clipped or scarved (ADR-0062/0063 conformance, AC-N2).
- `SPEED_KEYS` stays untouched (explicit non-goal; `scarf_joint_speed` bounds live at this packet's gate).

## Risks and Tradeoffs

- `seam_gap`-at-defaults moves goldens: Step 6 re-baselines with per-fixture measured deltas; any fixture whose delta is NOT exactly the gap clip is a defect, not fallout.
- Overlap vertex math must be grounded against canonical `extrude_loop` at implementation ([FWD] below) — wrong taper direction would read as a gap, not a joint.
- AC-6 cannot activate before 277 lands: the packet is still committable (five of six positive ACs live independently); Step 5 is entered only when 277's wipe `Move` exists.
- Percent-base mistakes (nozzle diameter vs line width vs role speed per key) are the likeliest silent defect — the table above assigns exactly one base per key; tests assert absolute-vs-percent spelling pairs for all three percent-typed keys.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 scarf build)
- Highest-risk dispatch and required return format: golden-delta LOCATIONS survey (Step 6) — `LOCATIONS ≤20`, fixture + delta each.

## Open Questions

- `[FWD]` Exact scarf overlap vertex placement (taper start/end offsets along the loop): resolve at implementation via the `GCode::extrude_loop` delegation in `requirements.md`; must reduce to the `seam_gap` distance at defaults and stay deterministic.
- No `[BLOCK]` questions — nothing here needs a human ruling before authoring completes (DEV-177 clauses (a)–(d) are all previously-ruled classes: unimplementable viewer role, missing filler family, 113-class bounds, 132-riding spellings).
