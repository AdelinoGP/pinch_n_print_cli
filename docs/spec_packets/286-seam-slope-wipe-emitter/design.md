# Design: 286-seam-slope-wipe-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) per-entity loop, extended at packet 285's scarf stage: slope-gate evaluation per closed loop, ramp segment emission generalising 285's overlap-length logic, and a loop-end wipe site emitting one inward `Move` per qualifying boundary.
- Neighboring tests/fixtures: new `crates/slicer-gcode/tests/seam_slope_wipe_emission_tdd.rs` (closed-loop emit fixture from Step 1, reused by all ACs); packet 285's `seam_scarf_joint_emission_tdd` guard (re-run in Step 4 when landed).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- WASM-staleness scope note: the `machine-gcode-emit.toml` schema edit is the trigger — guest WASMs embed config key names (ticket-101 finding), so the schema edit stales `machine-gcode-emit` until rebuilt. Host-only `.rs` edits do not.
- Emitter-side, not module-side: slope and loop-wipe are emission geometry over already-placed loops (`GCode::extrude_loop` is emission-time; owner `crates/slicer-gcode` stands — the ticket-27 hazard was checked and `machine-gcode-emit`'s sweep is the wrong seam, same as 285).
- Additive on 285, never a second stage: the slope ramp generalises 285's scarf overlap logic in place (one gate chain, one ramp emitter). A parallel slope path that duplicates the scarf path is rejected — it would double-clip seam regions.
- Scalar-global is parity: canonical declares all eight keys scalar, so no ticket-125 vector model; per-tool overrides do not apply (same as 285).
- `order_lock` geometry contract (ADR-0062/0063): locked paths are self-clipping producer footprints — the emitter neither ramps, clips, nor adds wipe moves to them (AC-N2).

## Code Change Surface

- Selected approach: eight `ResolvedConfig` fields + host-keys/machine-manifest schema rows; slope-gate chain (`type → loop-match → inner → min-length`) feeding a steps-modulated ramp with entire-loop and start-height variants, built by generalising 285's scarf overlap block; loop-end wipe site emitting one inward `Move` per qualifying boundary with retract-coincidence precedence; bounds rejection at the emitter gate (ticket-113 class); DEV-178 row.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs` (`declare_resolved_config!` field list): 8 new fields with canonical defaults.
  - `crates/slicer-gcode/src/emit.rs` (`DefaultGCodeEmitter::emit_gcode` + bounds gate): slope-gate chain, ramp emission, loop-wipe site, coincidence precedence.
  - `crates/slicer-gcode/src/serialize.rs` (`resolved_config_to_map` / `to_config_map` path): arms for `seam_slope_type` + `wipe_on_loops` only (twin shadowing; 284 precedent).
  - `docs/config/host-keys.toml` (`[resolved_config]`): 8 rows.
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` (`[config.schema.*]`): 8 tables.
  - `crates/slicer-gcode/tests/seam_slope_wipe_emission_tdd.rs`: new (all ACs).
  - `docs/DEVIATION_LOG.md`: DEV-178 row.
- Rejected alternatives and reasons:
  - Second parallel slope stage beside 285's scarf block: rejected — double-clips seam regions; the gate chain must be one.
  - Loop wipe as a policy gate on draft 277's retract `Move`: rejected — different trigger (loop end vs retract); 277 stays untouched, only the coincidence precedence is shared.
  - `Layer::is_perimeter_compatible` grouping port: rejected unless the delegated Orca read proves the loop-type gate insufficient — grouping machinery for an emitter gate is speculative Tier-C-shaped work inside a Tier-B packet.
  - Editing `ORCA_CONFIG_PADDING` twins: rejected (rule 2) — shadow via `to_config_map`, table untouched.

## Files in Scope (read + edit)

- `crates/slicer-gcode/src/emit.rs` - role: slope-gate chain, ramp emission, loop-wipe site, bounds gate; expected change: generalise 285's scarf block + add wipe site (~120 lines).
- `crates/slicer-ir/src/resolved_config.rs` - role: eight field declarations with canonical defaults; expected change: extend the `declare_resolved_config!` invocation (macro auto-covers `overlay_onto` per ticket 126).
- `crates/slicer-gcode/src/serialize.rs` - role: twin-shadowing arms for two keys; expected change: two `to_config_map` arms beside 284's.
- Beyond-3 files, justified (one row/table each, no logic; splitting them out would strand the schema across packets): `crates/slicer-gcode/tests/seam_slope_wipe_emission_tdd.rs` (new guard binary, ~250 lines), `docs/config/host-keys.toml`, `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, `docs/DEVIATION_LOG.md` (DEV-178), `docs/15_config_keys_reference.md` (regen).

## Read-Only Context

- `docs/08_coordinate_system.md` - lines `[1-60]` only - purpose: mm↔unit helper names for the min-length/start-height math.
- `docs/spec_packets/285-seam-scarf-joint-emitter/design.md` - SUMMARY dispatch only - purpose: scarf block anchor names to generalise (never copy its steps).
- `docs/spec_packets/277-retraction-wipe-travel-firmware-emitter/packet.spec.md` - SUMMARY dispatch only - purpose: retract-wipe `Move` trigger shape for the coincidence precedence.
- `crates/slicer-gcode/src/serialize.rs` - lines `[515-560]` only - purpose: padding-twin rows and 284-precedent shadowing arms.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Unrelated crates - delegate symbol lookups; do not browse
- `docs/spec_packets/285-seam-scarf-joint-emitter/` beyond the SUMMARY dispatch - never open `design.md`/`implementation-plan.md` (packet safety: no cross-packet edits)

## Expected Sub-Agent Dispatches

- Question: slope gating order + ramp construction inputs in `GCode::extrude_loop`/`ExtrusionLoopSloped`; scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` (≤200 words); purpose: Step 2 gate order.
- Question: loop-wipe trigger sites + neighbour-qualification shape; scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` (≤200 words); purpose: Step 3 wipe site.
- Question: every struct-literal site compiling against `ResolvedConfig` plus tests asserting field counts; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20); purpose: Step 1 blast radius.
- Question: 285's scarf block anchor names; scope: `docs/spec_packets/285-seam-scarf-joint-emitter/`; return: `SUMMARY` (≤150 words); purpose: Step 2 additive anchor.

## Data and Contract Notes

- IR/manifest contracts: eight scalar-global fields (enum ×1, float-or-percent ×1, float ×1, int ×1, bool ×4); canonical bounds adopted as reject-the-slice validation (DEV-178(a)); no IR/WIT/schema-version change.
- WIT boundary: untouched — no new accessor; the emitter reads `ResolvedConfig` directly.
- Determinism/scheduler constraints: ramp segmentation and wipe insertion are pure functions of loop geometry + resolved config; loop iteration order unchanged; no new claim, no DAG edge.

## Locked Assumptions and Invariants

- Defaults inert: `seam_slope_type = none` + wipe keys `false` emit geometry byte-identical to pre-packet (AC-2 pins it; only the `wipe_on_loops` CONFIG_BLOCK value shadows the stale twin).
- One wipe per boundary: a loop end coinciding with a retract site emits at most one wipe move (precedence: loop-wipe; 277's site skips when the loop site fired).
- Locked paths bypass: `order_lock` paths never ramp, clip, or gain wipe moves (ADR-0062/0063 self-clipping).
- DEV-178 sub-clauses: (a) bounds rejection is a deliberate divergence (canonical mins are GUI hints, ticket-113 class); (b) emitter-side ramp resolution vs canonical `ExtrusionLoopSloped` tessellation is an approximation recorded with rationale, not a gap; (c) CONFIG_BLOCK value spellings ride ticket 132.

## Risks and Tradeoffs

- 285 not yet landed at authoring: Step 2's additive anchor is a FORWARD shape — if 285's block names drift at its swarm, Step 2 reconciles names before editing (dispatch 4 re-runs first).
- `seam_slope_start_height` percent-base ambiguity (layer height vs line width): the delegated Orca read settles the base; if unresolvable, absolute-mm with percent-of-layer-height recorded as divergence — surfaced via `[FWD]`, never blocks.
- Inner-loop identification in the emitter's entity stream: if loop role metadata cannot distinguish inner walls, AC-5's gate degrades to region-index matching recorded as divergence rather than new IR metadata (no IR change in a Tier-B packet).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 slope stage)
- Highest-risk dispatch and required return format: Orca slope-gate-order SUMMARY (≤200 words) — grounds Step 2's gate chain; a vague return re-dispatches narrower on `ExtrusionLoopSloped` construction only.

## Open Questions

- `[FWD]` Percent base of `seam_slope_start_height` (layer height vs line width) — settled by the delegated Orca read in Step 2; absolute-mm fallback recorded as divergence.
- None blocking: no `[BLOCK]` — all eight keys are live in canonical (rule 3 checked at authoring against the map oracle) and every decision point is emitter-local.
