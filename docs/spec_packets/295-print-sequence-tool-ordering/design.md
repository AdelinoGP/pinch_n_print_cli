# Design: 295-print-sequence-tool-ordering

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) walks the Z-sorted `&[LayerCollectionIR]` and emits per-entity tool changes from `layer.tool_changes`; `apply_cross_layer_tool_rotation` (`crates/slicer-gcode/src/emit.rs`) is the existing per-layer tool-order stage with its `tool_changes` recompute tail. The packet adds one sequence-driven stable re-sort applied per layer after the rotation.
- Neighboring tests/fixtures: `apply_cross_layer_tool_rotation_*` unit tests inside `crates/slicer-gcode/src/emit.rs` (rotation + recompute precedent); `crates/slicer-gcode/tests/emit_tool_guard_tdd.rs` (emitter guard shape); `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs` (host-key default-guard shape).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Host-only change: no `[config.schema]` manifest row anywhere (host emitter reads `ResolvedConfig` directly; declaring a manifest row for a host-consumed key would be dead surface — ticket-34 converse), no WIT change, no IR field, no guest rebuild. `cargo xtask build-guests --check` must still exit fresh-0 as a pre-existing gate, not as packet work.
- No `int-list` type is introduced: `slicer_schema::VALID_CONFIG_TYPES` (`crates/slicer-schema/src/lib.rs`) stays closed; the two sequence keys reuse the `float-list` wire shape (`Vec<f64>` + round-to-index, dragon-curve precedent), recorded as DEV-187(a).
- Pure reorder: the sort changes only tool visitation order within a layer (stable partition of unlocked entities by tool, original intra-tool entity order preserved); entities with `path.order_lock.is_some()` (`ExtrusionPath3D.order_lock: Option<u64>`, reached via `entity.path.order_lock` on `PrintEntity`) stay pinned at authored indices, so ADR-0062 locked blocks ("they stay adjacent, in authored order and point direction") never split or interleave; no polygon, width, height, or position changes, so ADR-0063 linker obligations are unaffected (order-only).
- Schema/version constants and event-specific locking: none bumped. No new `PROGRESS_EVENT_SCHEMA_VERSION` or wire-version surface.

## Code Change Surface

- Selected approach: three scalar-global `ResolvedConfig` fields + one pure ordering kernel + one call site in `emit_gcode`, all host-side in two crates.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs`: three `declare_resolved_config!` rows mirroring the `filament_density` (`Vec<f64>` / `extract_float_list`) and `filament_flush_temp` (`u32` / `extract_u32_or_first`) arms — `first_layer_print_sequence: Vec<f64> = vec![0.0]`, `other_layers_print_sequence: Vec<f64> = vec![0.0]`, `other_layers_print_sequence_nums: u32 = 0` — with non-negative validation on list entries (reject negatives with the key named) and host-only omission from the CONFIG_BLOCK (no `to_config_map` arm, packet-287/288 precedent).
  - `crates/slicer-gcode/src/emit.rs`: pure kernel `sort_layer_tools_by_sequence(entities: &[PrintEntity], seq: &[u32]) -> Vec<usize>` (stable order permutation over `PrintEntity.tool_index: u32`; entities with `path.order_lock.is_some()` pinned at authored indices, unlocked entities stable-partitioned by tool with absent-from-seq tools trailing in original relative order) + `other_layer_covered(layer_idx_0based: usize, seq_len: usize, nums: u32) -> bool` (layer 0 never; layers ≥ 1 covered iff `nums > 0 && (layer_idx - 1) / seq_len < nums as usize`); call site in `DefaultGCodeEmitter::emit_gcode` after `apply_cross_layer_tool_rotation`: layer 0 re-sorted iff `first_layer_print_sequence.len() >= distinct_tools(layer_0)` (ported ignore-condition), layers ≥ 1 re-sorted iff covered and `other_layers_print_sequence` non-empty; `tool_changes` recomputed through the existing recompute tail (no forked recompute).
  - New `crates/slicer-ir/tests/resolved_config_print_sequence_tdd.rs`: AC-1 + AC-N2.
  - New `crates/slicer-gcode/tests/print_sequence_ordering_tdd.rs` with synthetic-layer fixture: AC-2/3/4/5 (defaults identity, layer-0-only, count-zero inert, cycle coverage + beyond-coverage untouched).
- Rejected alternatives and reasons:
  - Module-manifest declaration (e.g. on a tool-ordering or path-optimization module): rejected — no module owns per-layer tool visitation and `int-list` is not a valid manifest type; a `float-list` module row read by a module that never sees the layer stack would be declaration-only (rule 1).
  - New `int-list` schema/WIT type: rejected — blast radius across `slicer-schema`, WIT `config.wit`, and every guest binding for what rounding already expresses (dragon-curve precedent); recorded as DEV-187(a), not built.
  - Per-tool vector model (ticket 125): rejected — canonical declares plain global lists (`coInts`/`coInt`, verified); 125's axis is a different dimension.
  - Gating on ticket 122/124/136: rejected — ordering composes with any tower output, is not sequential printing, and needs no grouping engine (the override *is* the order here, DEV-187(b)).

## Files in Scope (read + edit)

Four files, not three: two are new guard binaries authored by the packet itself (zero pre-existing surface), so the pre-existing edit surface is two files.

- `crates/slicer-ir/src/resolved_config.rs` - role: config surface (3 DSL rows + validation); expected change: +3 field rows mirroring the `filament_density` / `filament_flush_temp` arms.
- `crates/slicer-gcode/src/emit.rs` - role: ordering kernel + `emit_gcode` call site; expected change: +2 pure functions, +1 call-site block reusing the recompute tail.
- `crates/slicer-ir/tests/resolved_config_print_sequence_tdd.rs` - role: new guard binary (AC-1, AC-N2); expected change: new file.
- `crates/slicer-gcode/tests/print_sequence_ordering_tdd.rs` - role: new guard binary with synthetic-layer fixture (AC-2..5); expected change: new file.

## Read-Only Context

- `crates/slicer-gcode/src/emit.rs` - lines `64-210` only - purpose: `DefaultGCodeEmitter` struct + `resolved_config` field + `with_resolved_config` test seam.
- `crates/slicer-gcode/src/emit.rs` - lines `1031-1130` only - purpose: `apply_cross_layer_tool_rotation` + recompute tail to reuse.
- `crates/slicer-ir/src/resolved_config.rs` - lines `2100-2230` only - purpose: `declare_resolved_config!` rows for `printable_area` (`Vec<f64>`), `filament_flush_temp` (`u32`), `filament_density` (`Vec<f64>`) to mirror.
- `crates/slicer-gcode/tests/emit_tool_guard_tdd.rs` - whole file only if under 600 lines, else first 80 lines - purpose: emitter guard fixture shape to mirror.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `modules/**` (all guest modules + manifests) - no surface here; do not declare, do not rebuild guests beyond the freshness `--check`
- `crates/slicer-runtime/src/layer_executor.rs` - ordering lands in the emitter, not the executor; do not split the stage
- `crates/slicer-gcode/src/serialize.rs` - padding table untouched by design (AC-N1); do not add twins
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: re-verify DEV-187 collision-free (`max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/`) at Step 4 time; scope: `docs/DEVIATION_LOG.md`, `docs/spec_packets/`; return: `FACT`; purpose: Step 4 deviation row.
- Question: confirm `LayerCollectionIR.tool_changes` element shape + entity tool-index accessor for the kernel's distinct-tool computation; scope: `crates/slicer-ir/src/slice_ir.rs`; return: `LOCATIONS` ≤10 entries; purpose: Step 2 kernel typing.
- Question: confirm the synthetic-layer fixture constructors used by `emit_tool_guard_tdd.rs`; scope: `crates/slicer-gcode/tests/emit_tool_guard_tdd.rs`; return: `SNIPPETS` ≤2 snippets ≤30 lines; purpose: Step 2 fixture.

## Data and Contract Notes

- IR/manifest contracts: none changed. Host-only omission means the keys never enter a module `ConfigView` (no `from_declared` whitelist interaction) and never enter the CONFIG_BLOCK (no `to_config_map` arm).
- WIT boundary: untouched (no `float-list` addition — the type already exists; no guest surface).
- Determinism/scheduler constraints: stable sort only; absent-from-sequence tools trail deterministically in original order; no RNG, no hash-iteration-order dependence (sequence positions indexed, not hashed).

## Locked Assumptions and Invariants

- Defaults inert: `[0.0]` first-layer list is shorter than any multi-tool layer's tool set (ignore-condition) and identity for single-tool; `[0.0]` + `0` other-layer pair is inert by the count-zero gate. AC-2 pins byte-identity at defaults.
- Pure reorder invariant: entity count, geometry, and intra-tool relative order preserved per layer; locked-tagged entities pinned at authored indices (ADR-0062 conformance, unconditional — holds even if a future producer spans one tag across tools); `tool_changes` recomputed, never hand-edited.
- Explicit order authoritative: sequence sort runs after `apply_cross_layer_tool_rotation` (port-internal rotation yields to user config, not vice versa).
- No struct-literal blast radius beyond the macro: `declare_resolved_config!` rows add fields; the macro emits overlay arms itself (ticket-126 precedent), and `PartialEq`-over-all-fields drift guards exist — Step 1 re-runs them.

## Risks and Tradeoffs

- `float-list` spelling for tool indices (DEV-187(a)): a `1.6` entry rounds — documented, validated non-negative, tested; alternative was a schema-wide `int-list` type (rejected on blast radius).
- Cycle-coverage model (DEV-187(b)): first-`nums`-cycles is a PnP simplification of grouping-record ranges; a profile relying on canonical's record-aligned ranges may cover different layers — accepted with rationale (no grouping engine to align to).
- Single-stack first layer (DEV-187(c)): layer index 0 of the emitted stack, not per-object minima-area ordering; multi-object first-layer nuances deferred with the area base.
- Bounds enforcement (DEV-187(d)): rejecting negatives is a deliberate divergence (canonical GUI-hint note) — fail-fast beats silently indexing tool `u32::MAX`.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 kernel + fixture + behaviour tests)
- Highest-risk dispatch and required return format: `LayerCollectionIR.tool_changes` shape — `LOCATIONS` ≤10 entries; wrong shape guesses cost a rewrite of the kernel signature.

## Open Questions

None. No `[FWD]`, no `[BLOCK]` — activation-ready after preflight PASS.
