---
status: draft
packet: 284-quality-precision-emitter
task_ids: []
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 58 (P51).
---

# Packet Contract: 284-quality-precision-emitter

## Goal

Make `enable_arc_fitting` and `resolution` drive host-side emission at parity with canonical `GCodeWriter` arc handling and the generation-time `resolution` simplify — ported as an emitter-side PnP simplification inside `crates/slicer-gcode`, with no new module, IR field, or WIT change.

## Scope Boundaries

P51 is two Tier B keys owned by the host emitter (`crates/slicer-gcode`). Both are zero-occurrence as behaviour on HEAD: `enable_arc_fitting` is undeclared (`classify_declared_key` returns `Undeclared`, proven by `crates/slicer-model-io/src/loader.rs` test pin) and bare `resolution` occurs only as the `ORCA_CONFIG_PADDING` row `("resolution", "0.012")` (`crates/slicer-gcode/src/serialize.rs`); no manifest, `ResolvedConfig` field, `config.get`, or `docs/config` row exists. The packet builds both missing decision points where the port already simplifies and serializes — the per-role D-P tolerance seam (`tolerance_for_role` in `crates/slicer-gcode/src/serialize.rs`, consumed by `DefaultGCodeEmitter::emit_gcode` in `crates/slicer-gcode/src/emit.rs`) and the `Move` → `G0`/`G1` rendering loop (`DefaultGCodeSerializer` in `crates/slicer-gcode/src/serialize.rs`) — as scalar-global `ResolvedConfig` fields at canonical defaults (`false`, `0.01`). Canonical declares both scalar (`coBool` default `0`, `coFloat` default `0.01` min `0` — grounded at authoring against the map oracle), so no per-tool vector model rides ticket 125. One intended default CONFIG_BLOCK value change: the padding `0.012` is shadowed by the live `0.01` via `to_config_map` dedup (count unchanged); emitted-move geometry is byte-identical at defaults.

## Prerequisites and Blockers

- Depends on: none. No adjacent draft packet shares this code surface. Ticket 132 (CONFIG_BLOCK reader contract, open) touches the same file's parse-side contract but a different region; this packet neither blocks on it nor implements it (re-derive adjacency at implementation).
- Unblocks: wayfinder ticket 58 (P51 closes alone).
- Activation blockers: none. All symbols below are live on HEAD (verified 2026-09-07); DEV-176 is the next collision-free ID (LOG max DEV-171; drafts propose DEV-172/173/174/175 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** `enable_arc_fitting` is declared as scalar-global `bool = false` and `resolution` as scalar-global `f32 = 0.01` (`min 0`, no `max`), with matching `docs/config/host-keys.toml` `[resolved_config]` rows. | `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd schema_declares_both_keys 2>&1 | tail -5`
- **AC-2. Given** default config (`enable_arc_fitting = false`, `resolution = 0.01`), **when** a curve-rich fixture is emitted, **then** the output contains zero `G2`/`G3` lines, the extrusion-move count equals the HEAD arc-disabled path, and the CONFIG_BLOCK differs from HEAD by exactly one value (`; resolution = 0.012` → `; resolution = 0.01`) with no line-count change (padding shadowed via `emit_config_kv` dedup, not edited). | `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd default_path_is_identity 2>&1 | tail -5`
- **AC-3. Given** `resolution = 0.5`, **when** the same curve-rich fixture is emitted, **then** the extrusion-move count is strictly smaller than the AC-2 default count (global floor dominates per-role tolerances at large values). | `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd large_resolution_simplifies_more 2>&1 | tail -5`
- **AC-4. Given** `enable_arc_fitting = true` at default `resolution`, **when** a circle-rich fixture is emitted, **then** the output contains at least one `G2`/`G3` extrusion line, total extruded length is conserved against the AC-2 G1-only run within `1e-3` mm, and every travel move still renders as `G0` (never `G2`/`G3`). | `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd arc_enabled_emits_g2_g3 2>&1 | tail -5`
- **AC-5. Given** `enable_arc_fitting = true` at default `resolution`, **when** a small-arc fixture is emitted, **then** the kept-vertex count strictly exceeds the AC-2 arc-disabled count at the same `resolution` (canonical `0.2 * resolution` tightening ported as `min(per_role, 0.2 * resolution)` when arcs are on). | `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd arc_enabled_tightens_tolerance 2>&1 | tail -5`
- **AC-6. Given** the generated config reference, **when** inspected after `cargo xtask gen-config-docs`, **then** both keys appear at canonical effective defaults (`false`, `0.01`) with no new default-deviation row (DEV-176 is behaviour-only). | `cargo test -p slicer-runtime --test unit host_keys_doc_lock 2>&1 | tail -3`

## Negative Test Cases

- **AC-N1. Given** `resolution = -1.0`, **when** the emitter resolves, **then** the slice is rejected with a stable validation error (min-0 enforcement; canonical declares min `0` as a GUI hint and never enforces — ticket-113 class, DEV-176(c)). | `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd negative_resolution_rejected 2>&1 | tail -5`
- **AC-N2. Given** `enable_arc_fitting = true` with travel moves and `order_lock` entities present, **when** emission runs, **then** no travel move renders as `G2`/`G3` and every locked entity keeps its full vertex count even at `resolution = 0.5` (locked paths are self-clipping per ADR-0063: the linker neither clips nor links them). | `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd travel_and_locked_never_arc 2>&1 | tail -5`
- **AC-N3. Given** the packet lands, **when** `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs` is inspected, **then** no `enable_arc_fitting` row was added and the `("resolution", "0.012")` row is byte-untouched (rule 2: padding is never evidence; the live `0.01` shadows it at runtime via dedup — the table itself is not a deliverable). | `rg -q 'enable_arc_fitting' crates/slicer-gcode/src/serialize.rs && exit 1 || rg -q '\("resolution", "0.012"\)' crates/slicer-gcode/src/serialize.rs`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd 2>&1 | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraints on emitter changes)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - direct range read of P51 rows only (tier B × 2, owner `crates/slicer-gcode`)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - direct range read of P51 section only (2-key membership)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'enable_arc_fitting' docs/15_config_keys_reference.md`
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'enable_arc_fitting' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-176" - `rg -q 'DEV-176' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`: both declarations (`enable_arc_fitting` `coBool` default `0`; `resolution` `coFloat` default `0.01` min `0`; already grounded at authoring)
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::apply_print_config` + `_spiral_travel_to_z`: `m_resolution` storage and the resolution-derived segment density when fitting is off
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — extrusion-path emission + `GCode::apply_print_config`: `G1`-vs-arc selection and the scaled `resolution` store
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_classic` / `process_arachne`: simplification tolerance reduced to `0.2 * resolution` when arc fitting is enabled (the AC-5 interaction)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
