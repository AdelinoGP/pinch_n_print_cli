# Design: remaining-automatic-values

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) already combines per-move width/flow with the layer's `height_delta` when computing `e_delta`, then calls `DefaultGCodeEmitter::resolve_feedrate`; the new automatic branch stays at this context-rich seam.
- Config transport: the `declare_resolved_config!` invocation (`crates/slicer-ir/src/resolved_config.rs`) supplies global and per-tool `ResolvedConfig`; `run_slice_with_collector` (`crates/slicer-runtime/src/run.rs`) passes both through `with_resolved_config` and `with_tool_configs`.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`, net-new `crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs`, FORWARD-DEP net-new `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` from draft packet 04 (reconcile its landed filename before use), and net-new `crates/pnp-cli/tests/fixtures/config_scope_resolution_10/{visual-debug.json,visual-debug-config.json}`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Phase placement is strict: packet 04 resolves only config-known placeholders; width, effective layer height, flow factor, and current tool remain live until `DefaultGCodeEmitter::emit_gcode` handles Phase C.
- `slicer-config` remains independent of `slicer-gcode`; do not move emitter geometry or G-code errors into the resolution crate.
- `filament_max_volumetric_speed` is a snake_case host key, per-filament/per-tool like `filament_diameter`, and must be finite and positive when a zero speed needs it.
- `ResolvedConfig` equality/hash/to-map behavior remains macro-driven; adding its field must not introduce a parallel hand-maintained serializer.
- Packet 04 retains exclusive ownership of overhang percentages and both existing config-only negative mirror rules.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add the typed filament maximum to `ResolvedConfig`; keep `resolve_feedrate` as the sole factor-to-`F` seam and add a private move-context helper used by `emit_gcode` when the selected role base is zero. The helper selects tool-over-global config, validates finite positive inputs, and calculates the automatic mm/s base from `mm3_per_mm`; that base then flows through `resolve_feedrate`'s existing speed-factor clamp, mm/min conversion, and rounding rather than bypassing them.
- Exact functions, traits, manifests, tests, and fixtures:
  - `declare_resolved_config!` gains `cli @filament "filament_max_volumetric_speed" filament_max_volumetric_speed: f32 = 0.0 => extract_float_or_first` with metadata documenting `0` as unavailable unless no automatic speed is requested.
  - `DefaultGCodeEmitter` gains private tool/global maximum and move-context base-speed selection; `emit_gcode` supplies `current_tool`, `point.width`, the layer's `height_delta`, and `point.flow_factor`, while `resolve_feedrate` remains responsible for factor clamping and `F` conversion.
  - `GCodeEmitError::Emit` carries invalid maximum/width/height/flow diagnostics; no new public error variant is required.
  - `volumetric_auto_speed_tdd.rs` emits real `GCodeIR` and inspects literal `Move.f` values; it does not test a duplicate formula helper in isolation.
  - `registry_negative_sentinel_census_has_no_unowned_phase_c_candidate` derives entries with a negative numeric default or lower bound from the assembled registry and fails on an unclassified negative-capable key rather than maintaining a complete key roster.
  - The visual-debug request drives a model through `PostPass::GCodeEmit`; its `source.config` references `visual-debug-config.json`, parsed by `parse_cli_config_source`, containing `outer_wall_speed = 0` and `filament_max_volumetric_speed = 8.0`.
- Rejected alternatives and reasons:
  - Expanding zero speed in `expand_automatic_values`: rejected because move width, effective layer height, flow factor, and active tool are unavailable in Phase B.
  - Teaching `FeedrateConfig::from_raw_config` geometry: rejected because it is a config adapter and would freeze context too early.
  - Re-owning overhang percentages: rejected because packet 04 resolves all four against typed `outer_wall_speed`.
  - Inventing branches for undeclared Orca negative sentinels: rejected; the derived census must first establish a live registry key and owner.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs` — role: typed host/per-tool config carrier; expected change: add `filament_max_volumetric_speed` through the declaration macro.
- `crates/slicer-gcode/src/emit.rs` and `crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs` — role: Phase-C owner and falsifying tests; expected change: context-aware zero-speed resolution and literal output checks.
- FORWARD-DEP net-new `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` from draft packet 04 (name-reconciled against its landed test target), `crates/pnp-cli/tests/fixtures/config_scope_resolution_10/{visual-debug.json,visual-debug-config.json}`, `docs/config/host-keys.toml` (new `[resolved_config]` `filament_max_volumetric_speed` entry feeding `cargo xtask gen-config-docs`; `host_keys_doc_lock_tdd` holds it equal to the `ResolvedConfig` default), `docs/02_ir_schemas.md`, and generated `docs/15_config_keys_reference.md` — justified extras: derived ownership guard, mandated visual evidence with its parsed config source, the machine-readable host-key declaration behind the generated config row, and required docs; expected change: census, request/config fixture, host-key entry, and contract documentation only.

## Read-Only Context

- `crates/slicer-runtime/src/run.rs` — `DefaultGCodeEmitter` construction and resolved global/tool handoff only.
- `crates/slicer-ir/src/feedrate.rs` — `FeedrateConfig`, `SPEED_KEYS`, `read_speed`, and `from_raw_config` only.
- `crates/slicer-gcode/src/error.rs` — `GCodeEmitError` variants and runtime mapping comments.
- `docs/specs/config-scope-resolution-plan.md` — RC-8, Expansion, queue row 10, and cross-cutting requirements only.
- `docs/spec_packets/config-scope-resolution_04_automatic-value-expansion/{design.md,task-map.md}` — FORWARD-DEP exports and exclusions only.
- `docs/spec_packets/config-scope-resolution_05_scope-resolution-module/{design.md,packet.spec.md}` — resolution exports and precedence only.
- `docs/19_visual_debug.md` — request, manifest, and G-code emit tap sections only.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md` and packet directories 01–09 — read-only; never edit.
- Overhang-classifier manifests/source and `FeedrateConfig` overhang-percent expansion — packet 04 ownership.
- `OrcaSlicerDocumented/...` — delegate; never load.
- WIT, IR schema-version constants, CLI JSON schema versions, manifest-schema vocabulary — unchanged.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- Unrelated crates — delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: do packets 04/05's implemented exports match the names/shapes in this packet, and does resolution deliver per-tool `filament_max_volumetric_speed` to emitter tool configs?; scope: exact exports plus emitter construction; return: `FACT: <5 lines or fewer>`; purpose: Step 1 prerequisite gate.
- Question: derive every assembled registry entry with a negative numeric default and classify whether its automatic base is config-only or stage-context-dependent; scope: registry assembly inputs and automatic rules; return: `LOCATIONS: <at most 20 file:line entries, one context line each>`; purpose: Step 1 census.
- Question: verify canonical zero-speed volumetric formula, tool selection, and invalid-flow behavior; scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`, function `GCode::_extrude`; return: `SUMMARY: <at most 200 words, no code unless requested>`; purpose: Step 1 parity lock.
- Question: run each narrow cargo/visual/freshness command and report only verdict plus bounded failure evidence; scope: commands in `requirements.md`; return: `FACT: <5 lines or fewer>`; purpose: Steps 2–4 validation.

## Data and Contract Notes

- IR/manifest contracts: adding a host field to `ResolvedConfig` changes in-memory config content but introduces no new serialized IR container or manifest field vocabulary; existing schema/version constants remain unchanged unless implementation proves this assumption false, which blocks rather than silently bumps.
- WIT boundary: unchanged; no guest receives move-context automatic-speed inputs.
- Determinism/scheduler constraints: packet-05 precedence selects the resolved global/tool values; emitter selection is deterministic by `PrintEntity.tool_index`. No scope merge occurs in the emitter.
- Units: maximum volumetric speed is mm³/s, `mm3_per_mm` is mm³/mm, their quotient is mm/s, and emitted `F` remains mm/min.
- ADR contract: ADR-0052 (`docs/adr/0052-per-point-speed-factor-contract.md`) makes `DefaultGCodeEmitter::resolve_feedrate` the sole factor-to-`F` seam and requires its `speed_factor.clamp(0.05, 5.0)`. The Phase-C quotient supplies the base speed to that seam; it does not bypass factor application or clamping. AC-1/AC-2 deliberately use factor `1.0`, so their literal `F` values isolate automatic-base and tool-selection behavior rather than re-testing the clamp.

## Locked Assumptions and Invariants

- A positive configured role speed keeps the existing path and is not capped by this packet.
- Exactly zero selects Phase-C automatic speed; negative configured speeds remain invalid at registry validation and are not an auto signal.
- Tool-specific resolved config wins; absent tool config falls back to resolved global config.
- Packet 04's four overhang percentage values arrive as absolute mm/s or numeric zero and are never resolved again here.
- Current grounding found no declared geometry-dependent negative sentinel; the conservative derived census converts any future discovery into a loud failure before implementation can claim completion.

## Risks and Tradeoffs

- The current role-only `resolve_feedrate` input cannot calculate automatic speed. The implementation must extend its private input path to receive the context-selected base rather than emit from a parallel helper; otherwise it either silently preserves zero or violates ADR-0052's sole factor-to-`F` seam. The emitter test must exercise `emit_gcode` itself.
- A zero/negative layer delta or width can produce division by zero or a non-finite F token. Validate all factors before division and test each class.
- Adding a `ResolvedConfig` field has a broad struct-literal/equality/hash surface. The macro should own generated behavior; all test literals must use FRU or an exhaustive waiver under `docs/21_data_defaults_and_fixtures.md`.
- A registry census can become a hand-maintained roster if it starts from key names. Derive candidates from live declarations and keep only explicit owner classification for discovered negative defaults.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: Orca `GCode::_extrude` formula/tool/guard verification — `SUMMARY` ≤200 words.

## Open Questions

- [FWD] At implementation start, reconcile packet 04/05's landed exports and adapt imports/call signatures without changing Phase-C ownership.
- [FWD] If the derived negative-default census finds a live geometry-dependent `-1` key beyond packet 04's two mirrors, pause after naming its key, owner stage, base inputs, and test target; incorporate it only if it fits this packet's Phase-C boundary, otherwise return a scope blocker.
