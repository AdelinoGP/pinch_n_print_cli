# Requirements: prime-tower-interface-and-ramming

## Packet Metadata

- Packet directory: `docs/spec_packets/254b-prime-tower-interface-and-ramming/`
- Backlog source: `docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md` (wayfinder map packet P02)
- Owner modules: `wipe-tower` (existing) and `prime-tower-interface` (**new**, `PostPass::GCodePostProcess`)
- Sibling half: `docs/spec_packets/254a-prime-tower-geometry-keys/` — **must land first**
- Tier: **C** — the packet builds decision points *and* introduces a new core module participating in a stage `wipe-tower` cannot reach, with a cross-crate registration surface. Re-derived at authoring per map Authoring rule 1.
- Status: `draft`

## Problem Statement

Packet 254 declared nine interface / ramming keys in the `wipe-tower` manifest and recorded every one of them as a "decision-point gap". Map Authoring rule 1 prohibits that: a key is covered only when the behaviour OrcaSlicer attaches to it exists in this tree and the key drives it.

Two obstacles made those nine look unbuildable, and both dissolve on inspection:

1. **"The port has no interface tower."** Canonical's interface feature is a purge block with a different volume, a pre-extrusion lead-in, and an optional ironing pass. The port's tower is already a purge block with a volume, a leading travel entity, and a scan-line body. Seven of the nine keys are parameters of that block, and `254a`'s per-layer depth model gives them somewhere to attach.
2. **"The port cannot change nozzle temperature."** It can. `GCodeCommand::Temperature { tool, celsius, wait }` exists in `crates/slicer-ir/src/slice_ir.rs`, `GcodeFlavor::set_temperature` emits `M104`/`M109` (`G10 P`/`M116` on RepRapFirmware) in `crates/slicer-gcode/src/flavor.rs`, `GcodeOutputBuilder::push_temperature` exists in `crates/slicer-sdk/src/postpass_builders.rs`, and the marshalling is plumbed end-to-end. **No production module has ever constructed one** — that is the gap, not a missing capability. The constraint is only that the command channel lives on `GcodeOutputBuilder` (`run_gcode_postprocess`), never on `FinalizationOutputBuilder`, so the two temperature keys need a module at `PostPass::GCodePostProcess`. `GCodeCommand::Move` carries `role: ExtrusionRole`, and `ExtrusionRole::WipeTower` exists, so that module can find the tower in the stream without any new carrier.

Consequently **no key in this packet is blocked, and no WIT interface, IR schema bump, or `ResolvedConfig` field is required.**

## In Scope

1. **`enable_tower_interface_features` (interface gate).** When true, a purge block becomes an *interface* block: its depth is computed from `filament_tower_interface_purge_volume` instead of `prime_volume`, and items 2–4 apply. Default `false` keeps `254a`'s behaviour exactly.
2. **`filament_tower_interface_pre_extrusion_dist` (lead-in travel).** The block's leading travel entity (`flow_factor = 0.0`) spans this distance instead of today's degenerate zero-length two-point travel.
3. **`filament_tower_interface_pre_extrusion_length` (lead-in extrusion).** An extruding entity of this path length precedes the block's first scan line. `0.0` (default) emits none.
4. **`filament_tower_ironing_area` + `prime_tower_flat_ironing` (ironing pass).** When both the interface gate and the flat-ironing flag are on, an `ExtrusionRole::Ironing` boustrophedon pass follows the block, covering `filament_tower_ironing_area` mm² (`ironing_span = area / tower_width`, clamped to the block depth).
5. **`enable_filament_ramming` (ramming zigzag).** Default `true`: a ramming entity precedes each block's scan lines, a zigzag over the block's leading `y_step = (prime_tower_infill_gap / 100) × line_width` band — canonical `WipeTower::toolchange_Unload` uses `m_extra_spacing` as exactly that `y_step`. `false` omits it.
6. **`filament_tower_interface_print_temp` (tower nozzle temp)** and **`enable_tower_interface_cooldown_during_tower` (when the temp change lands)**, in the new `prime-tower-interface` module at `PostPass::GCodePostProcess`.
7. **The new module's registration surface**, bounds enforcement for all nine keys, and the generated docs.

## Out of Scope

- `254a`'s three geometry keys and its per-layer depth model — consumed here, not re-implemented.
- `prime_tower_skip_points` — returned to the queue by `254a` (needs a travel-avoid-perimeter facility). Canonical ANDs it into `m_use_gap_wall`, which gates the interface tower and flat ironing; the port ANDs against `enable_tower_interface_features` instead (D-254b-3).
- **Canonical's per-filament parameter arrays.** `filament_tower_interface_pre_extrusion_dist/_length/_purge_volume` and `filament_tower_ironing_area` are `coFloats`, and `filament_tower_interface_print_temp` is `coInts`, indexed by extruder in `m_filpar`. All five are declared **scalar-global** here per ticket 04's ruling (D-254b-1).
- **Canonical's MMU-gated staged unload/load state machine.** `WipeTower2::toolchange_Unload` / `toolchange_Load` run a multi-stage retraction with cooling moves and MMU gating. The port emits the ramming zigzag only (D-254b-4).
- **Canonical's "max nozzle temp" resolution of the `-1` sentinel.** The port has no nozzle-temperature model (D-254b-2).
- Cone / rib / fillet wall shapes — packet `255-wipe-tower-geometry-keys`.
- Any edit to `ORCA_CONFIG_PADDING` or a CONFIG_BLOCK padding twin (map Authoring rule 2).
- New WIT interface, IR schema bump, or new `ResolvedConfig` field — **none is required**; every type and builder method this packet uses already exists.

## Returned to Queue — unimplemented, needs a feature this packet does not build

None. All nine keys assigned to this half are implemented by it. (`prime_tower_skip_points`, the tenth key of the former packet 254 not covered by `254a`, is returned by `254a`, which owns that disposition.)

## Ruled Dead-in-canonical

None. Every one of the nine has at least one read site inside `src/libslic3r/` in the slicing pipeline, confirmed by delegated canonical read at authoring. Two are worth naming because their read set is narrow enough that a careless sweep would call them dead:

- `enable_tower_interface_cooldown_during_tower` — **only** the `WipeTower2::WipeTower2` constructor, consumed in `WipeTower2::tool_change`. No `WipeTower.cpp` and no `GCode.cpp` read.
- `prime_tower_flat_ironing` — **only** the `WipeTower::WipeTower` constructor (`m_flat_ironing`, then ANDed with `m_use_gap_wall`), consumed in `WipeTower::toolchange_wipe_new`. No `WipeTower2` and no `GCode.cpp` read.

Occurrences in `Print::invalidate_state_by_config_options`' option list are invalidation bookkeeping and were **not** counted as slicing reads for any key.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` §Host-Boundary Access Enforcement (Normative) + the stage-declaration sections — govern the new module's manifest, its stage id, and its `[ir-access]`.
- `docs/01_system_architecture.md` §Claim System and `docs/04_host_scheduler.md` §Claim Resolution — consulted to confirm the new module is stage-scheduled, not claim-held.
- `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract — forbids padding edits.
- `docs/08_coordinate_system.md` — plain mm floats; never port `scale_()`.
- `docs/00_project_overview.md` — modular pipeline / community extensibility; the new module is a normal core module a community fork could replace.
- `docs/15_config_keys_reference.md` — generated; never hand-edited.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the nine declarations.
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` — `WipeTower::WipeTower` ctor, `WipeTower::set_extruder`, `WipeTower::toolchange_wipe_new`, `WipeTower::finish_block_solid`, `WipeTower::get_next_pos`, `WipeTower::get_wall_skip_points`, `WipeTower::toolchange_Unload`.
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` — `WipeTower2::WipeTower2` ctor, `WipeTower2::set_extruder`, `WipeTower2::tool_change`, `WipeTower2::toolchange_Unload`, `WipeTower2::toolchange_Load`, `WipeTower2::finish_layer`, `WipeTower2::tool_ramming_enabled`.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `WipeTowerIntegration::append_tcr`, `WipeTowerIntegration::append_tcr2`, `GCode::set_extruder`.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

## Parity Evidence Standard

Every canonical claim below was produced by a delegated read of the sibling checkout at authoring time and is cited by **file + function only**, never by line number. In-tree citations are by crate-qualified path + symbol name. A worker who disputes a row re-dispatches the read. Behaviour is pinned with invariant tests (entity presence/absence, ordering, computed spans, command position), not golden G-code — the canonical checkout is readable, not runnable.

## Per-Key Canonical Evidence

| Key | Canonical type | Default | Bounds | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- |
| `enable_tower_interface_features` | coBool | `false` | — | `WipeTower::WipeTower` ctor (the `m_use_gap_wall` cluster) and `WipeTower::set_extruder`; `WipeTower2::WipeTower2` ctor; `WipeTowerIntegration::append_tcr` / `append_tcr2` (`GCode.cpp`). Downstream: `WipeTower::get_next_pos`, `WipeTower::get_wall_skip_points`, `WipeTower::finish_block_solid`, `WipeTower::toolchange_wipe_new`, `WipeTower2::tool_change`, `WipeTower2::finish_layer` | **Built (AC-3)** — gates the interface block |
| `filament_tower_interface_purge_volume` | coFloats | `20` | min 0 | `WipeTower2::set_extruder`; `WipeTowerIntegration::append_tcr`; `GCode::set_extruder`. **No `WipeTower.cpp` read site** | **Built (AC-3)** — drives interface block depth; declared scalar-global (D-254b-1) |
| `filament_tower_interface_pre_extrusion_dist` | coFloats | `10` | min 0 | `WipeTower::set_extruder`, `WipeTower2::set_extruder` (per-filament `m_filpar`). No direct `GCode.cpp` read | **Built (AC-4)** — lead-in travel span; scalar-global |
| `filament_tower_interface_pre_extrusion_length` | coFloats | `0` | min 0 | `WipeTower::set_extruder`, `WipeTower2::set_extruder`, `WipeTowerIntegration::append_tcr` | **Built (AC-5)** — lead-in extrusion length; scalar-global |
| `filament_tower_ironing_area` | coFloats | `4` | min 0 | `WipeTower::set_extruder` (`m_filpar[idx].flat_iron_area`), `WipeTower2::set_extruder` (`tower_ironing_area`). No `GCode.cpp` read | **Built (AC-6)** — ironing pass area; scalar-global |
| `prime_tower_flat_ironing` | coBool | `false` | — | **Only** `WipeTower::WipeTower` ctor — `m_flat_ironing`, then `m_flat_ironing = m_flat_ironing && m_use_gap_wall` — consumed in `WipeTower::toolchange_wipe_new`. No `WipeTower2`, no `GCode.cpp` read | **Built (AC-6)** — gates the ironing pass, ANDed with the interface gate (D-254b-3) |
| `enable_filament_ramming` | coBool | `true` | — | `WipeTower2::WipeTower2` ctor (`m_enable_filament_ramming`); used in `WipeTower2::toolchange_Unload`, `WipeTower2::toolchange_Load`, `WipeTower2::tool_ramming_enabled`. **No `WipeTower.cpp` read site.** The ramming `y_step` in `WipeTower::toolchange_Unload` comes from `m_extra_spacing` (`prime_tower_infill_gap`) | **Built (AC-7)** — ramming zigzag; the staged MMU unload/load is D-254b-4 |
| `filament_tower_interface_print_temp` | coInts | `-1` | min −1 (`-1` = use max nozzle temp) | `WipeTower::set_extruder`, `WipeTower2::set_extruder`, `WipeTowerIntegration::append_tcr`, `WipeTowerIntegration::append_tcr2`, `GCode::set_extruder` | **Built (AC-8, AC-9)** — pushes `GCodeCommand::Temperature`; `-1` means "no override" here (D-254b-2); scalar-global |
| `enable_tower_interface_cooldown_during_tower` | coBool | `false` | — | **Only** `WipeTower2::WipeTower2` ctor, consumed in `WipeTower2::tool_change` — chooses temp boost *during tower printing* vs *at toolchange* | **Built (AC-8)** — selects where the `Temperature` command lands in the stream |

## Recorded Divergences

**ID convention.** The `D-254b-*` labels are **packet-local divergence identifiers** for cross-referencing inside this packet's five files. They are *not* `docs/DEVIATION_LOG.md` row IDs — that log uses `DEV-###` (verified against the live log at authoring; no `D-254b*` token appears in it). The closure step registers each as a `DEV-###` row with the ID **re-derived from the log at write time** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next), never frozen here (CLAUDE.md ledger-fact rule).

- **D-254b-1 — scalar-global, not per-filament.** Canonical's five `filament_tower_*` keys are `coFloats`/`coInts` indexed by extruder in `m_filpar`. This port declares them as scalar module keys, per ticket 04's ruling. Exact for single-material and for multi-material prints using one filament profile; a per-filament model stays in the map's Tier-D fog.
- **D-254b-2 — `-1` means "no temperature override", not "max nozzle temp".** Canonical resolves the sentinel to the maximum nozzle temperature across filaments. This tree has no nozzle-temperature model at all: `ResolvedConfig` carries no temperature field, and the only temperature keys anywhere are `nozzle_temperature_initial_layer` / `bed_temperature_initial_layer_single` on `machine-gcode-emit`. Inventing a max over a set that does not exist would be fiction, so `-1` (the default) emits nothing and the default path is unchanged. Asserted by AC-9.
- **D-254b-3 — flat ironing is ANDed with the interface gate, not with `m_use_gap_wall`.** Canonical's `m_use_gap_wall` derives from `prime_tower_skip_points`, which `254a` returned to the queue as needing a travel-avoid facility. The port preserves the *shape* of canonical's conjunction (`flat_ironing && <interface mode>`) while sourcing the second operand from `enable_tower_interface_features`. Asserted by AC-6's two negative directions.
- **D-254b-4 — ramming is a zigzag, not a staged unload/load.** Canonical runs a multi-stage retraction with cooling moves under MMU gating (`WipeTower2::toolchange_Unload` / `toolchange_Load`). The port emits the ramming extrusion zigzag using canonical's own `y_step` basis (`m_extra_spacing`) and leaves the retraction staging to the existing retract machinery. Rationale: the staged sequence is expressed in terms of a `WipeTowerWriter` the port does not have, and half-porting it would produce retractions the port's own retract handling would then duplicate.
- **D-254b-5 — the temperature seam is a separate module, not a `wipe-tower` capability.** Canonical emits the interface temp from `GCode::set_extruder` / `WipeTowerIntegration::append_tcr`, i.e. inside the G-code writer that also builds the tower. In this port those are two different stages: `wipe-tower` runs at `PostPass::LayerFinalization`, whose `FinalizationOutputBuilder` has no command channel, while the command channel lives on `GcodeOutputBuilder` at `PostPass::GCodePostProcess`. Splitting the concern into `prime-tower-interface` respects the port's stage boundaries rather than widening a builder to match canonical's coupling — an improvement recorded per map Authoring rule 4, not a gap.

## Acceptance Summary

| AC | Key(s) | Non-default value asserted | Home test |
| --- | --- | --- | --- |
| AC-1 / AC-N3 | the seven `wipe-tower` keys | — (manifest guard) | `wipe-tower::wipe_tower_config_schema_tdd` |
| AC-2 | the two `prime-tower-interface` keys | — (manifest + module-count guard) | `slicer-scheduler::integration::manifest_ingestion_tdd` |
| AC-3 | `enable_tower_interface_features`, `filament_tower_interface_purge_volume` | `true`, `40.0` | `wipe-tower::wipe_tower_tdd` |
| AC-4 | `filament_tower_interface_pre_extrusion_dist` | `25.0` | `wipe-tower::wipe_tower_tdd` |
| AC-5 | `filament_tower_interface_pre_extrusion_length` | `5.0` | `wipe-tower::wipe_tower_tdd` |
| AC-6 | `prime_tower_flat_ironing`, `filament_tower_ironing_area` | `true`, `9.0` | `wipe-tower::wipe_tower_tdd` |
| AC-7 | `enable_filament_ramming` | `false` (canonical default is `true`) | `wipe-tower::wipe_tower_tdd` |
| AC-8 | `filament_tower_interface_print_temp`, `enable_tower_interface_cooldown_during_tower` | `250`, `true` | `prime-tower-interface::interface_temp_tdd` |
| AC-9 | `filament_tower_interface_print_temp` | `-1` (default sentinel — the *absence* assertion for D-254b-2) | `prime-tower-interface::interface_temp_tdd` |
| AC-10 | all nine | rejection path | `slicer-scheduler::integration::config_bounds_enforcement_tdd` |
| AC-11 | all nine | — (generated docs) | `cargo xtask gen-config-docs --check` |
| AC-N1 | all nine | default-path identity to `254a` (**additional**, never the only evidence for any key) | `wipe-tower::wipe_tower_tdd` |
| AC-N2 | (module inert without a tower) | no `WipeTower` role in the stream | `prime-tower-interface::interface_temp_tdd` |

Map gate (b) check: every one of the nine keys has at least one AC asserting a behaviour change at a non-default value — `enable_tower_interface_features` AC-3 (`true`), `filament_tower_interface_purge_volume` AC-3 (`40.0`), `filament_tower_interface_pre_extrusion_dist` AC-4 (`25.0`), `filament_tower_interface_pre_extrusion_length` AC-5 (`5.0`), `prime_tower_flat_ironing` AC-6 (`true`), `filament_tower_ironing_area` AC-6 (`9.0`), `enable_filament_ramming` AC-7 (`false` — canonical's default is `true`, so `false` is the non-default value), `filament_tower_interface_print_temp` AC-8 (`250`), `enable_tower_interface_cooldown_during_tower` AC-8 (`true`).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1 / AC-N3 manifest guard | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-3 / AC-4 / AC-5 / AC-6 / AC-7 / AC-N1 | FACT pass/fail |
| `cargo test -p prime-tower-interface --test interface_temp_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-8 / AC-9 / AC-N2 | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration manifest_ingestion_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2 module count 23 → 24 | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-10 bounds rejection | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | AC-11 generated docs | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness **and new-guest discovery** | FACT exit=0 |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate | FACT exit code |
| `cargo xtask check-deviations --check` | deviation-log / doc-07 freshness | FACT exit code |

## Step Completion Expectations

- **`254a` must be implemented and merged before this packet starts.** Its `plan_layer_depths` and the `depth_offset` / `block_depth` parameters on `generate_purge_paths` are the attachment points for every interface AC here. Starting before it would fork the same function twice.
- The new module's scaffold (crate + guest + manifest) and its registration (`slicer-integrated-modules`, `slicer-runtime`, `pnp-cli`, the 23 → 24 count) must land in one commit: a discovered-but-unregistered module fails the integrated-edition build, and a registered-but-undiscovered one fails `build-guests --check`.
- The manifest steps must precede their wiring steps — `bind_module_config_view` filters to declared keys, so an undeclared key reads as absent and every non-default AC would silently pass on the default branch.
- After every step touching either module's `.toml` or `src/lib.rs`, `cargo xtask build-guests --check` must return exit 0 before any host-integration result is believed (CLAUDE.md Guest WASM Staleness). This packet is the more dangerous case because it adds a **new** guest: a `wasm-tools`-missing exit `3` prints no `STALE:` line and must not be read as clean.

## Context Discipline Notes

- `modules/core-modules/wipe-tower/src/lib.rs` is 772 lines at authoring — **over the 600-line ceiling**. Read located windows around `from_config`, `generate_purge_paths` and `run_finalization` only.
- `crates/slicer-ir/src/slice_ir.rs` and `crates/slicer-sdk/src/traits.rs` are far over the ceiling. Read located windows around `GCodeCommand`, `ExtrusionRole`, and `run_gcode_postprocess` only; never in full.
- Use an existing `PostPass::GCodePostProcess` module (`machine-gcode-emit`) as the structural template for the new crate rather than deriving the scaffold from the docs.
- Every cargo invocation is delegated with a FACT return; output tees to `target/test-output.log` and is read from disk, never re-run for more output.
