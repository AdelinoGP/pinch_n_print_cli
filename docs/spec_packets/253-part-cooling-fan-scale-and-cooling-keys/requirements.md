# Requirements: 253-part-cooling-fan-scale-and-cooling-keys

## Packet Metadata

- Grouped task IDs: none — the feature-gap queue's established pattern is `task_ids: []` (packet 234a precedent); `docs/07_implementation_status.md` holds no TASK row for this queue.
- Backlog source: `docs/specs/orca-feature-gap/issues/08-author-packet-p01-cooling-notes-part-cooling.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P01).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer's cooling configuration is 19 % of every FFF feature gap packet queue's smallest owner-scoped slice (packet P01), and today Pinch 'n Print implements almost none of it faithfully:

- The `part-cooling` module declares 8 keys and reads 4; `fan_min_speed` is declared but **never read** (04-asset note), so `reduce_fan_stop_start_freq`'s never-off idle behavior has no input to idle at.
- `fan_max_speed`/`fan_min_speed` are raw S-values (0–255, defaults 255/51) while Orca declares percents (0–100, defaults 100/20) — the ticket-99 finding. Any preset or 3MF carrying Orca's percent values is misinterpreted as a raw S-value off by a factor of 2.55.
- The module's fan decision is `layer_index < close_fan ? 0 : fan_max_speed` plus a bridge-role bump — none of Orca's layer-time interpolation, full-fan ramp, stop/start suppression, role-fan speeds, overhang threshold, or fan speedup/kickstart exist.
- The air-filtration / chamber-temperature / exhaust-fan keys (`activate_air_filtration`, `activate_chamber_temp_control`, `during_print_exhaust_fan_speed`, `complete_print_exhaust_fan_speed`) do not exist anywhere in code; canonically they are start/end-G-code emission-time features (commands belong in the machine's custom G-code templates), so what must land here is the **config surface + emission-surface reachability**, not slicer-internal logic.
- `dont_slow_down_outer_wall` canonically gates CoolingBuffer's slowdown of external perimeters; this tree has no layer-slowdown decision point at all (the `slow_down_*` keys the module declares are schema-inert), so the honest disposition is the emission surface + recorded gap.

The slice is coherent because all 19 keys share one owner (`part-cooling`), one decision stream (the per-layer fan curve), and one emission surface; the four header/footer keys additionally share the `machine-gcode-emit` co-declaration pattern.

## In Scope

All 19 P01 keys (membership from `05-asset-packet-list.md` as amended by ticket 99):

1. **Percent normalization (Tier B):** `fan_max_speed`, `fan_min_speed` — re-declare percent 0–100 (defaults 100/20 = Orca byte-identical), migrate every read site, convert percent→S-value at emission via canonical `GCodeWriter::set_fan`'s `floor(255.5 × p / 100)`.
2. **Fan curve port (Tier B + Tier A):** port `CoolingBuffer.cpp::apply_layer_cooldown`'s `change_extruder_set_fan` decision into `PartCooling` — `fan_cooling_layer_time`, `full_fan_speed_layer`, `reduce_fan_stop_start_freq` (now actually wired to `fan_min_speed`), first-layers gate; the already-declared `slow_down_layer_time`/`slow_down_min_speed` become read as interpolation inputs.
3. **Role-fan keys (Tier A):** `internal_bridge_fan_speed`, `ironing_fan_speed`, `support_material_interface_fan_speed` — declared, read, applied with `-1`-disabled fallback semantics per canonical precedence.
4. **Overhang threshold (Tier A):** `overhang_fan_threshold` — enum `0%|10%|25%|50%|75%|95%` (canonical default `95%`), classifying entities by `overhang_quartile` bands and bridge roles.
5. **Time-domain fan control (Tier A):** `fan_kickstart`, `fan_speedup_time`, `fan_speedup_overhangs` — re-time rising fan commands per canonical `FanMover` semantics using a per-entity time model derived from IR geometry.
6. **Auxiliary channel (Tier A):** `auxiliary_fan`, `additional_cooling_fan_speed` — `M106 P2 S{n}` per-layer raw annotation, percent-converted, gated on `auxiliary_fan`.
7. **Emission-surface keys (Tier A):** `activate_air_filtration`, `activate_chamber_temp_control`, `during_print_exhaust_fan_speed`, `complete_print_exhaust_fan_speed` — declared in `part-cooling` and **co-declared** in `machine-gcode-emit` (existing co-declaration pattern) so custom G-code templates can substitute them as placeholders; values stay raw percents in templates.
8. **Emission-surface with recorded gap (Tier A):** `dont_slow_down_outer_wall` — declared + emitted in the config block; the slowdown decision point it gates does not exist in this tree, recorded as a known gap in `design.md` (never silently dropped).
9. **Declarations + docs:** all 19 keys carry Orca-parity defaults, bounds, enums, display/group metadata; `docs/15_config_keys_reference.md` regenerated; guest WASM rebuilt.

## Out of Scope

- A layer-time-slowdown stage (`calculate_layer_slowdown` analogue) — does not exist in this tree; building it is future work. `dont_slow_down_outer_wall` therefore lands with its gap recorded, not consumed.
- Porting Orca's `M191`/`M141` header/footer emission or any air-filtration automation — custom G-code templates carry those commands here; this packet only guarantees placeholder reachability.
- `object_config` / per-object overrides of the new keys (host plumbing exists for typed fields; extensions-bucket keys ride global config only).
- The `slow_down_for_layer_cooling` bool (declared today, inert) — not one of the 19 keys; renaming/removing it is not this packet's scope beyond keeping the schema compile-clean.
- Any other feature-gap packet (P02+ in `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`).
- `OrcaSlicerDocumented/` code changes — read-only checkout, never modified.

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/08-author-packet-p01-cooling-notes-part-cooling.md` — the ticket; direct read (30 lines).
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` — the evidence standard binding every key below; direct read (~80 lines).
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` — the 19 keyed rows defining tier/owner framing (ranged read of the cooling section only).
- `docs/15_config_keys_reference.md` — regenerated artifact; never hand-edited (grep-verified post-regen).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`):

- `src/libslic3r/GCode/CoolingBuffer.cpp` — `apply_layer_cooldown` (lambda `change_extruder_set_fan`): the full fan-speed decision — `reduce_fan_stop_start_freq` base, `slow_down_layer_time` / `fan_cooling_layer_time` interpolation, `full_fan_speed_layer` ramp, role-fan marker precedence (`OVERHANG > INTERNAL_BRIDGE > SUPP_INTERFACE > IRONING`), and `-1` fallbacks; `parse_layer_gcode` / `process_layer`: `dont_slow_down_outer_wall` external-perimeter gating (decision point absent from this port).
- `src/libslic3r/GCode.cpp` — `check_overhang_fan`: `overhang_fan_threshold` overlap comparisons; `process_layers`: `FanMover` construction from `fan_speedup_time` / `fan_speedup_overhangs` / `fan_kickstart`; `_do_export`: air-filtration / chamber-temp / exhaust-fan emission structure (never ported — header/footer semantics belong to custom G-code templates here).
- `src/libslic3r/GCode/FanMover.cpp` — `_process_gcode_line`: re-timing of rising fan commands and the kickstart burst.
- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef`: declarations, types, defaults (incl. `overhang_fan_threshold` defaulting to `Overhang_threshold_bridge` = `"95%"`) and `s_keys_map_OverhangFanThreshold`.
- `src/libslic3r/GCodeWriter.cpp` — `set_fan`: the `255.5 × speed / 100` percent→S conversion (also `set_additional_fan` for the `P2` channel).

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Per-Key Canonical Evidence (verified this session at authoring time)

| Key | Canonical declaration | Canonical consumer (file + function) | Ported behaviour |
| --- | --- | --- | --- |
| `fan_max_speed` | `PrintConfig.cpp` `PrintConfigDef` (coFloats, default 100, [%]) | `CoolingBuffer.cpp::apply_layer_cooldown` lambda `change_extruder_set_fan`; `GCodeWriter.cpp::set_fan` for %→S | curve maximum; `255.5 × p / 100` floor at emission |
| `fan_min_speed` | same (coFloats, default 20) | same lambda; also the `reduce_fan_stop_start_freq` idle base | curve interpolation floor; idle base when suppressing stop/start |
| `fan_cooling_layer_time` | same (coFloats, default 60, [0, 1000]) | same lambda — top of interpolation window | interpolation upper bound |
| `fan_kickstart` | same (coFloat, default 0, min 0) | `GCode.cpp::process_layers` → `FanMover` (`GCode/FanMover.cpp::_process_gcode_line`) | max-speed burst N seconds before demand |
| `fan_speedup_time` | same (coFloat, default 0) | same `FanMover` construction + re-timing | rising fan emitted N seconds early |
| `fan_speedup_overhangs` | same (coBool, default true) | same gate `(!only_overhangs \|\| role == erOverhangPerimeter)` | restrict early emission to overhang layers |
| `full_fan_speed_layer` | same (coInts, default 0) | same lambda's ramp factor | linear ramp from `close_fan` to `full_fan_speed_layer` |
| `reduce_fan_stop_start_freq` | same (coBools, default false) | same lambda's base `fan_speed_new = R ? fan_min_speed : 0` | fan idles at min instead of off |
| `dont_slow_down_outer_wall` | same (coBools, default false) | `CoolingBuffer.cpp::parse_layer_gcode`/`process_layer` — external-perimeter non-adjustable; **gate absent here** | declared + emitted; gap recorded |
| `auxiliary_fan` | same (coBool machine-level, default false) | `GCode.cpp::_do_export` (P2 switch) | P2-channel enable + placeholder |
| `additional_cooling_fan_speed` | same (coInts, default 0, [0,100]) | same lambda's `change_extruder_set_fan` P2 line | per-layer P2 speed |
| `activate_air_filtration` | same (coBools, default false) | `GCode.cpp::_do_export` header/footer gates | placeholder for custom templates |
| `activate_chamber_temp_control` | same (coBools, default false) | `GCode.cpp::_do_export` (M191 before start / M141 at end), header/footer only | placeholder for custom templates |
| `during_print_exhaust_fan_speed` | same (coInts, default 60, [0,100]) | `GCode.cpp::_do_export` + custom-G-code placeholder `during_print_exhaust_fan_speed_num` | placeholder (raw percent) |
| `complete_print_exhaust_fan_speed` | same (coInts, default 80, [0,100]) | `GCode.cpp::_do_export` end section | placeholder (raw percent) |
| `internal_bridge_fan_speed` | same (coInts, default −1, [%]) | same lambda — `_INTERNAL_BRIDGE_FAN_START` marker path; −1 falls back to `overhang_fan_speed` | role fan with fallback |
| `ironing_fan_speed` | same (coInts, default −1) | same lambda — `erIroning` marker path, −1 = disabled | role fan, −1 = disabled |
| `support_material_interface_fan_speed` | same (coInts, default −1) | same lambda — `_SUPP_INTERFACE` marker path | role fan, −1 = disabled |
| `overhang_fan_threshold` | same (coEnums, default `Overhang_threshold_bridge` = `"95%"`; map `0%|10%|25%|50%|75%|95%`) | `GCode.cpp` lambda `check_overhang_fan` (overlap comparisons) | quartile-band classifier (mapping table in design) |

Note: the repo's snapshot reference `docs/ORCA_CONFIG_REFERENCE.md` records the `overhang_fan_threshold` default as 50%; the fresh canonical read this session (`PrintConfig.cpp` `PrintConfigDef`, `set_default_value(new ConfigOptionEnumsGeneric{ (int)Overhang_threshold_bridge })`) says the default is `95%`. This packet follows the fresh read and records the discrepancy here — do **not** re-size off the snapshot column (map Notes rule).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9` (declarations+bounds; percent conversion formula; full fan-curve branch matrix; role-fan precedence and −1 fallbacks; threshold mapping; kickstart/speedup re-timing; auxiliary P2 channel; placeholder reachability; docs regeneration).
- Negative: `AC-N1` (bounds enforcement of new keys), `AC-N2` (no key leakage into non-declaring modules), `AC-N3` (exhaustive conversion-formula pin).
- Cross-packet impact: none — no other packet touches `part-cooling`; packet P15 (`support_ironing`) may later consume `ironing_fan_speed`'s decision point but owns no change here.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p part-cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` | all module behaviour + schema tests | FACT pass/fail + counts |
| `cargo test -p part-cooling --test cooling_curve_parity_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | the new canonical-curve invariant suite | FACT pass/fail |
| `cargo test -p machine-gcode-emit 2>&1 | tee target/test-output.log | grep -E "^test result"` | placeholder reachability | FACT pass/fail |
| `cargo test -p slicer-scheduler --test config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | bounds/enum enforcement of the 19 keys | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- gcode_part_cooling_emission_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | emission-surface regression pin (module inside the aggregated `integration` binary, registered at `tests/integration/main.rs`) | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- integrated_parity_part_cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` | module→host parity contract stays green (module registered in the aggregated `contract` binary) | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 | tail -5` | docs/15 regenerated tables match manifests | FACT (tail lines) |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness after manifest/src edits | exit code |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 | tail -3` | struct-literal churn gate (new test fixtures) | FACT (tail lines) |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Steps build in order: percent normalization precedes curve port (curve tests assert percent-domain expectations); co-declaration precedes the reachability test; docs regen is last.
- The existing tests `first_layers_disabled_then_fan_on`, `overhang_region_bumps_fan`, and slicer-runtime's `gcode_part_cooling_emission_tdd` fixtures assert raw-scale bytes (`S255`, `S100`) — under percent normalization the *defaults* still produce the exact module-emitted bytes (100% → S255 via `floor(255.5)`; the overhang bump at `overhang_fan_speed=100` × `fan_max_speed=100` percent → 100% → S255). Any fixture whose input data was raw-typed must restate its intent in percent and **re-derive** expected bytes through `floor(255.5 × p / 100)` (e.g. a 50% expectation is `floor(127.75)` = **S127**, not S128) — never silently edit a byte to pass.
- `OrcaSlicerDocumented/` lives at the sibling path (see orca-delegation in `packet.spec.md`): all canonical reads delegated.

## Context Discipline Notes

- The fan-curve port (Step 4) is the packet's deepest step: delegate canonical re-reads (`CoolingBuffer.cpp` lambda quotes) rather than re-deriving from memory; the load-bearing interpolation formula is already quoted in this file's per-key table's source (ticket grounding above) but must be re-verified against the sibling checkout at implementation time.
- `crates/slicer-ir/src/slice_ir.rs` is ~2900 lines: ranged reads only (the `ExtrusionRole` enum at ~2216, `PrintEntity`/`ExtrusionPath3D` at ~2340–2830, `LayerCollectionIR` at ~2828+).
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` is long: read the cooling section rows only.
- Never load `target/`, `OrcaSlicerDocumented/` directly, or generated lockfiles.