# Design: 276-retraction-toolchange-restart-lift-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — the entity retract loop (canonical retract-then-travel order site), the `entity_z_hop` execution arms (which build the Z-only `Move` from `layer_z + zh.hop_height`), the layer-boundary and mid-layer toolchange `Retract` synthesis sites (both hardcode `speed: 2400.0`), and the single `GCodeCommand::Unretract` construction site in the entity unretract loop.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (retract-order test constructs `GCodeCommand::Retract` with mm/min speeds; `E-0.8`/`F2400` precedent), `crates/slicer-gcode/tests/gcode_toolchange_wrapping.rs` (toolchange-driving binary), `per_tool_config_overrides_retract_length` (the per-tool override test pattern Step 2 mirrors), `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` (placeholder-render harness for AC-8) and the P35 `pressure_advance_config_schema_tdd` manifest-guard precedent (one guard binary per packet).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Only `machine-gcode-emit.toml` (AC-8) feeds the guest build; guests embed config key names (ticket-101 lesson), so that one manifest edit stales exactly the `machine-gcode-emit` guest. The seven `ResolvedConfig` + `emit.rs` changes are host-only and need no rebuild.
- New `ResolvedConfig` fields ride `apply_cli_key` (macro-derived, covers every `cli`-declared field), so `tool_config:<idx>:` overrides work with no extra plumbing (the `retract_length_for_tool` precedent). They do NOT ride `overlay_resolved` (29-of-83 hand allowlist, ticket 126 open) — irrelevant here because the emitter reads global + per-tool configs, never region overlays; record the interaction, do not fix 126 in-packet.
- Bounds follow the ticket-113 precedent: `min 0.0` on lengths/speeds where canonical declares it, never a `max` (canonical declares no speed maxima; a max here would be a PnP invention). Adopting a canonical range is a deliberate divergence only where this packet does it — it does not.
- Bool CONFIG_BLOCK spelling must be canonical `1`/`0`, not word-form `true` (map CONFIG_BLOCK Note; systemic fix stays with ticket 132 — this packet only spells its own key correctly and tests it).

## Code Change Surface

- Selected approach: single-seam host emission. All seven numeric keys become scalar-global `ResolvedConfig` fields (declare-macro arms beside `retract_length`, `crates/slicer-ir/src/resolved_config.rs`) with explicit `to_config_map` inserts; `emit_gcode` consumes them at the four existing decision sites. `long_retractions_when_cut` is declared in `machine-gcode-emit.toml` and published via that module's existing `substitute_placeholders` lookup (no new seam, no new placeholder machinery).
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs` — 8 field declarations + 8 `to_config_map` inserts + doc comments citing canonical defaults.
  - `crates/slicer-gcode/src/emit.rs` — `retract_length_for_tool`-shaped resolver for the toolchange length; unretract-loop length/speed overrides with the variant selected by a per-layer toolchange-boundary set built from the existing `tool_changes` anchor map (anchors at/after a `ToolChange` in command order take the `_toolchange` variant; exact comparison pinned in Step 3 with AC-3 as falsifier); ZHop-arm gating helper (`should_lift(layer_z, is_top_layer, is_bottom_layer)`); strict `retract_lift_enforce` parse returning a fatal config error on unknown values.
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — `long_retractions_when_cut` bool schema row; `src/lib.rs` placeholder-lookup arm with a key-specific `1`/`0` rendering guarantee added explicitly per `docs/adr/0050-custom-gcode-architecture.md` (the generic `format_placeholder_value` renders word-form `Bool`, which the map's CONFIG_BLOCK Note already rules out as a canonical spelling).
  - Tests: `gcode_emit_tdd.rs` (+5 tests), `gcode_toolchange_wrapping.rs` (+2 tests), new `modules/core-modules/machine-gcode-emit/tests/retraction_keys_schema_tdd.rs` manifest guard (packet-260/262 precedent: one guard binary per packet; the P35 `pressure_advance_config_schema_tdd` binary is not reused), plus one placeholder-render case in the existing `machine_gcode_emit_tdd` render harness.
  - Docs: `docs/DEVIATION_LOG.md` (DEV-171), generated `docs/15_config_keys_reference.md`, `docs/config/host-keys.toml` + the `host_keys_doc_lock_tdd` lock test (packet-267 precedent).
- Rejected alternatives and reasons:
  - Path-opt seam (declare travel-side keys on `path-optimization-default`): rejected — the toolchange synthesis and `TravelRetract` rendering both live in the emitter, and execution-side gating covers every retract source (path-opt, wipe-tower prime entities, host synthesis) where a module-side gate would miss the synthesized ones. `TravelRetract` gains no field.
  - Widening to per-filament vectors now: rejected — the ticket-125 ruling owns the model; DEV-171 records the scalar choice with the `tool_config:` override as the per-tool channel.
  - Geometric longer-retract for `_cut`: rejected — canonical has none (its reads are the export flag plus placeholder publication); the port mirrors what canonical wires.

## Files in Scope (read + edit)

- `crates/slicer-gcode/src/emit.rs` - role: all seven numeric decisions; expected change: resolver + loop/arm edits + parse helper (bounded sites only, never the whole file).
- `crates/slicer-ir/src/resolved_config.rs` - role: seven field declarations + `to_config_map` inserts; expected change: macro arms beside `retract_length` + inserts beside its entry.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - role: `_cut` declaration; expected change: one bool schema row.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - role: placeholder publication; expected change: one lookup arm.
- Tests, `docs/DEVIATION_LOG.md`, `docs/config/host-keys.toml`, generated reference - justified extras: rule-1 evidence, rule-2-adjacent DEV record, and the 267-precedent doc lock.

## Read-Only Context

- `crates/slicer-gcode/src/serialize.rs` - lines 760-811 only - purpose: `Retract`/`Unretract` render shape (`F<speed>` passthrough proves the emitter-seam unit is mm/min) and the `G11`-carries-no-speed limitation.
- `crates/slicer-ir/src/slice_ir.rs` - `TravelRetract` + `GCodeCommand::Retract/Unretract` + `ZHop` struct defs only - purpose: field names and documented units (mm, mm/s, mm).
- `modules/core-modules/path-optimization-default/src/lib.rs` - `from_config` + inter-region emission block only - purpose: proof the travel-side keys are owned elsewhere and untouched.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - placeholder lookup construction + `format_placeholder_value` (generic word-form `Bool` rendering — hence the key-specific guarantee) only - purpose: AC-8's exact insertion point.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- P37 wipe/travel/cut key sites (`wipe_distance`, `use_firmware_retraction`, `retraction_distances_when_*` consumers) - different packet's scope
- `crates/slicer-gcode/src/estimator.rs`, `crates/slicer-scheduler/**`, ticket-126's `region_mapping.rs` - no change in this packet
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: pin `retract_lift_enforce` canonical enum key strings + the five numeric defaults adopted above; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `LOCATIONS` (<=10 entries); purpose: Step 1.
- Question: pin `lazy_lift`/`eager_lift` bound-comparison shape (strict vs inclusive, zero-as-sentinel); scope: `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp`; return: `SUMMARY` (<=150 words); purpose: Step 1.
- Question: pin the `_cut` placeholder publication (names published, per-filament shape); scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` (<=150 words); purpose: Step 1.
- Question: list every `ResolvedConfig` struct-literal site that must gain the seven new fields; scope: `crates/ modules/ xtask/` struct literals of `ResolvedConfig`; return: `LOCATIONS` (<=20 entries); purpose: Step 2 blast radius.

## Data and Contract Notes

- IR/manifest contracts: no IR, WIT, or schema-version change. `TravelRetract`/`GCodeCommand`/`ZHop` shapes are untouched; the packet only changes the values the emitter computes.
- WIT boundary: none crossed (emitter is host-native; the one module change is manifest schema + placeholder text, no new host-service call).
- Determinism/scheduler constraints: gating is a pure function of `(layer_z, layer position, config)`; no ordering or claim interaction. `should_lift` must be total (every `ZHop` maps to lift-or-skip, never to an error except the AC-N1 parse failure which happens once per slice, not per layer).
- `retract_lift_enforce` layer-position semantics are a recorded divergence: canonical gates on surface ROLES (top/bottom surface regions within a layer); the emitter seam has no surface-role metadata, so the packet gates on layer POSITION (topmost/bottommost object layers). Rationale: the only observable layer identity at this seam; region-role plumbing would be a new IR field (packet-worthy on its own, not smuggled here).
- `deretraction_speed` unit contract: declared mm/s (canonical unit), converted `* 60.0` at the render override; the `0.0 = passthrough` fallback mirrors canonical's `<= 0` rule from `Extruder::deretraction_speed`.

## Locked Assumptions and Invariants

- Default-path identity: at canonical defaults (`10.0` toolchange length is NOT today's synthesis length — see risk) every byte of default output is unchanged EXCEPT the toolchange retract length, which canonically changes at defaults by design (today: `retract_length` 2.0; after: `retract_length_toolchange` 10.0). This is the packet's one intended default-output change and its own regression tests pin it; everything else is default-identical.
- `0.0` disables each lift bound independently; `all_surfaces` lifts everywhere; `false` publishes `0`.
- No step declares `retract_before_wipe` or `long_retractions_when_ec`.

## Risks and Tradeoffs

- The toolchange-length default change (2.0 -> 10.0 at defaults) alters default G-code for every multi-tool print: intended (canonical parity), but it is the largest blast radius in the packet — AC-1 pins both arms and Step 5 re-runs the full e2e suite, not just the targeted binaries.
- If Step 1's bound-comparison delegation contradicts strict-inequality, Step 3 conforms (tests use far-from-boundary values precisely so the comparison direction is not load-bearing in the ACs).
- If Step 1 confirms the `TravelRetract` path emits unconverted mm/s speeds, the packet files a follow-up ticket (128-family) and does not fix it: fixing changes every travel retract's feedrate and would swallow this packet's own speed ACs.
- Layer-position vs surface-role enforce divergence may mis-gate prints whose top/bottom surface regions sit mid-object (e.g. stepped tops); accepted with rationale above, revisit when a surface-role field reaches the emitter seam.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 emitter logic + 7 tests)
- Highest-risk dispatch and required return format: struct-literal `LOCATIONS` (Step 2) — an incomplete list breaks `cargo check` on all targets; must be exhaustive (<=20 entries, redispatch by crate on overflow).

## Open Questions

- None. (`[FWD]` for the implementer: re-audit absolute retract feedrates end-to-end if the Step 1 unit-path finding confirms the `TravelRetract` conversion gap — file, do not fix.)
