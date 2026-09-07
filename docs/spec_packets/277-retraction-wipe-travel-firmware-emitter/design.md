# Design: 277-retraction-wipe-travel-firmware-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — entity retract/unretract loops, layer-change path, ZHop execution arms, preamble Z, toolchange synthesis (`retract_length_for_tool` neighbour).
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (AC-1–AC-7, AC-N1), `crates/slicer-gcode/tests/gcode_toolchange_wrapping.rs` (regression neighbour, untouched), `modules/core-modules/machine-gcode-emit` tests `machine_gcode_emit_tdd` (AC-8/AC-9 placeholder render).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Rule-4 trigger test (map Notes Q8): this packet is in-module emitter branching, not cross-module algorithm selection — no `claim:*` holders, no new module, no holder key. `z_hop_types` stays a plain enum on the emitter for the same reason `retract_lift_enforce` did in packet 276.
- Scalar-global divergence: nine keys are canonical per-filament vectors (`coFloats`/`coBools`/`coPercents`/`coEnums`); this packet declares scalar-globals and records DEV-172 (276's DEV-171 precedent). The vector model stays with ticket 125. No `tool_config:` override for these eleven keys.
- `retract_before_wipe` is percent-typed (canonical `coPercents`): the emitter clamps to `[0, 100]` and divides by `100` (ticket-107 `extract_percent_float` precedent); the manifest declares a percent-compatible float.
- `G11` carries no speed in `crates/slicer-gcode/src/serialize.rs` (276-measured): `use_firmware_retraction` selects the mode only; no deretraction-speed interaction is built here.
- One `design.md` Architecture Constraints bullet when the change surface feeds guest WASM: this packet edits `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, so guest artifacts embed the new schema — the implementer MUST run `cargo xtask build-guests --check` (exit `0` fresh / `1` stale / `3` infra) and rebuild before attributing any guest/module-dispatch failure to its own changes.
- Strict-parse for `z_hop_types`: unknown values are fatal with the key name + four legal values (276 AC-N1 precedent); canonical display strings (`"Auto Lift"` etc.) are normalized to snake-case (`auto`/`normal`/`slope`/`spiral`) at the parse boundary.
- Schema/version constants and event-specific locking: none bumped by this packet. If implementation finds a bump is required, the bumping step owns the struct-literal blast radius AND the test-assertion fallout in the same step (no deferred `cargo check` discovery).

## Code Change Surface

- Selected approach: eleven scalar `ResolvedConfig` fields + `to_config_map` inserts (host-owned emission control, ticket-42 P35 precedent — host-only keys are omitted from nothing; they are emitted via `to_config_map` as the live side effect); nine wire into the existing `emit_gcode` decision arms, two publish via the existing `machine-gcode-emit` substitution seam. The wipe site is new emission (no existing wipe path to extend): split-retract → wipe `Move` → remainder-retract, inserted at the entity-retract site.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs` — eleven field arms beside `retract_length` + eleven `to_config_map` inserts.
  - `crates/slicer-gcode/src/emit.rs` — `needs_retraction`-style travel gate, layer-change gate, wipe emission + split helper, `RetractMode` selection, ZHop style arms, Z-offset addition (bounded sites only).
  - `crates/slicer-gcode/src/serialize.rs` — read-only (G10/G11 render shape already exists; no edit expected).
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — two float schema rows (`retraction_distances_when_cut`, `retraction_distances_when_ec`).
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — two placeholder lookup arms (float spelling, no `1`/`0` coercion).
  - Tests: new cases in `gcode_emit_tdd`, `machine_gcode_emit_tdd`, new guard binary `retraction_keys_schema_tdd`; `docs/DEVIATION_LOG.md` (DEV-172), `docs/config/host-keys.toml`, generated `docs/15_config_keys_reference.md`.
- Rejected alternatives and reasons:
  - Per-filament vectors now: rejected — ticket 125 owns the model; scalar-global with a recorded divergence is the map precedent (276/DEV-171).
  - Wipe as a module claim holder: rejected — wipe is an emitter move between retract halves, not an alternative algorithm in a separate module (Q8 trigger test fails).
  - Declaring `_cut`/`_ec` distances host-only without the manifest: rejected — the module substitution reads only declared keys (`ConfigView::from_declared` whitelist, ticket-34 hazard); without the declaration the placeholder never resolves.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs` - role: eleven field declarations + `to_config_map` inserts; expected change: macro arms beside `retract_length` + inserts beside its entry.
- `crates/slicer-gcode/src/emit.rs` - role: all nine emitter decisions; expected change: gates + wipe emission + mode/style/offset edits at bounded sites only, never the whole file.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - role: two distance declarations; expected change: two float schema rows with display/group.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - role: placeholder publication; expected change: two lookup arms with float rendering.
- Tests, `docs/DEVIATION_LOG.md`, `docs/config/host-keys.toml`, generated reference - justified extras: rule-1 evidence, DEV record, and the 267-precedent doc lock.

## Read-Only Context

- `crates/slicer-gcode/src/serialize.rs` - lines 760-811 only - purpose: `Retract`/`Unretract` Gcode-vs-Firmware render shape (`G10`/`G11`, G11-carries-no-speed limitation).
- `crates/slicer-ir/src/slice_ir.rs` - lines 3040-3070 only - purpose: `RetractMode::Gcode`/`Firmware` variants + `TravelRetract` mm/s contract (ticket-43 finding).
- `crates/slicer-gcode/src/emit.rs` - lines 60-140 only - purpose: `retract_length_for_tool` override pattern (NOT generalized here) + feedrate neighbours (`wipe_speed`, `travel_speed`).
- `docs/DEVIATION_LOG.md` - last 20 lines only - purpose: DEV-172 row format + next-free-number re-derivation.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `docs/spec_packets/276-*/**` - predecessor; SUMMARY dispatch only if a seam question arises, never edit
- Unrelated crates - delegate symbol lookups; do not browse
- `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs` padding table) - never edit (Authoring rule 2)

## Expected Sub-Agent Dispatches

- Question: pin the eleven P37 defaults/types/enum strings/placeholder names/slope math/G10-G11 spelling/Z-offset sites; scope: `OrcaSlicerDocumented/src/libslic3r`; return: `LOCATIONS` (≤20 entries) or `SUMMARY` (≤200 words); purpose: Step 1 binding inputs.
- Question: list every struct-literal site compiling against `ResolvedConfig` today; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries); purpose: Step 2 blast radius.
- Question: confirm `gcode_emit_tdd` / `machine_gcode_emit_tdd` driver setup for the new fixtures; scope: `crates/slicer-gcode/tests, modules/core-modules/machine-gcode-emit/tests`; return: `FACT`; purpose: Steps 3-5 AC homes.

## Data and Contract Notes

- IR/manifest contracts: host keys flow `ResolvedConfig` → `emit_gcode` directly; placeholder keys additionally flow `ResolvedConfig.to_config_map` → module `ConfigView` (manifest-declared) → substitution. A module cannot read an undeclared key (ticket-34 whitelist hazard) — hence the two manifest rows.
- WIT boundary: untouched. No new IR field, no WIT accessor, no schema bump expected.
- Determinism/scheduler constraints: emitter decisions are per-entity/per-layer pure functions of config + geometry; no scheduler ordering change; wipe direction derives deterministically from the last extrusion vector (no RNG).

## Locked Assumptions and Invariants

- Default-path identity holds for every key except `retraction_minimum_travel` (`2.0` newly gates short-travel retracts) and `z_hop_types` (`slope` newly shapes the default lift): those two are intended canonical-alignment output changes, each pinned by its own AC arm.
- `retract_before_wipe = 100.0` means "all before, none after"; `0.0` means "all after". Values outside `[0, 100]` are clamped, never fatal.
- Canonical's EC-null state has no scalar representation: unset `retraction_distances_when_ec` means the `10.0` default, recorded in DEV-172.
- Wipe direction invariant: the wipe `Move` retraces the last extrusion vector reversed, length exactly `wipe_distance`; when no prior extrusion exists in the layer, no wipe emits even when enabled.

## Risks and Tradeoffs

- Wipe emission is new geometry on a previously wipe-free path: the split changes retract counts (one `Retract` becomes two around a `Move`) — golden-sensitive fixtures that count retracts may need re-baselining with measured justification, never silent updates.
- Slope-hop diagonal changes the default lift shape (see identity exception above): fixtures asserting vertical-only lifts will move; the AC pins the new math so the move is intentional.
- Scalar-global collapses nine per-filament vectors: multi-tool prints cannot vary these per tool until ticket 125 lands — accepted and recorded (DEV-172), not engineered around.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4 wipe build + split)
- Highest-risk dispatch and required return format: Step 1 canonical pin-down, `LOCATIONS` ≤20 or `SUMMARY` ≤200 words; reject oversized returns and redispatch narrowly.

## Open Questions

None.
