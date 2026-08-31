# Design: skirt-type-and-draft-shield-keys

## Controlling Code Paths

- Primary code path: `modules/core-modules/skirt-brim/src/lib.rs` —
  `SkirtBrim::from_config` (the six existing `config.get` reads), the live
  `run_finalization` (span loop, brim gate, entity pushes via
  `FinalizationOutputBuilder::push_entity_to_layer`), the test-only `process()`
  arm, and the generators `generate_skirt_entities` / `generate_brim_entities` /
  `make_rect_loop`.
- Neighboring tests/fixtures:
  `modules/core-modules/skirt-brim/tests/{slicer_module_binding_tdd,skirt_brim_tdd,finalization_live_tdd}.rs`
  (`finalization_live_tdd.rs` is the live-path driver with the
  `LayerCollectionView` + `FinalizationOutputBuilder` setup this packet's AC-2
  arms reuse); guard-pattern source
  `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs`
  (parses the TOML directly; part-cooling's Cargo.toml carries the `toml = "0.8"`
  dev-dependency skirt-brim will need).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- snake_case config key strings only (repo convention): the five new keys and all `config.get` / `ConfigKey` strings are already snake_case by construction here.
- `run_finalization` and the test-only `process()` stay behaviorally aligned: both must implement the three gates identically (packet 257 precedent — packet 246/247-era dual-path divergence is a recorded lesson).

## Code Change Surface

- Selected approach: declare the five keys in the `skirt-brim` manifest with
  canonical defaults/bounds; widen `SkirtBrim`'s config reads by three fields
  (`draft_shield: bool`, `single_loop_draft_shield: bool`, `start_angle: f32`);
  implement three gates on the existing generators: (1) span — when
  `draft_shield` is enabled choose the layer span
  `layers.len()` instead of `min(skirt_height, layers.len())`; (2) loop count —
  when `single_loop_draft_shield` and `global_layer_index > 0`, generate exactly
  the innermost loop instead of all `skirt_loops`; (3) start corner — when
  `global_layer_index == 0`, rotate the first loop's point list so it begins at
  the corner angularly nearest the canonical desired start point
  (`find_start_point` analogue); leave `skirt_type`/`min_skirt_length` unread
  (declared-with-gap).
- Exact functions, traits, manifests, tests, and fixtures:
  `skirt-brim.toml` `[config.schema]` + five tables; `SkirtBrim` struct + 3 new
  fields; `from_config` + 3 new reads with fallback-to-default match arms
  (tree-support-planner's enum-read pattern for the two enums — invalid values
  fall back to the canonical default, host enum enforcement happens earlier at
  resolve time, AC-5); `run_finalization` + span/count/corner gates;
  `generate_skirt_entities` signature grows the layer-index condition and the
  corner-rotation for the first loop; `make_rect_loop` + a point-list rotation
  (rotate the 5-point closed ring, keep `first == last` closure invariant);
  `process()` mirrors all three gates.
- Rejected alternatives and reasons:
  - *Rotation via path rotation metadata* — `ExtrusionPath3D` has no start-point
    or rotation field; the loop is the point list, so the rotation must be the
    literal ring rotation. Rejected alternative was emitting a rotated copy of
    the ring (same result, more code).
  - *Teaching the emitter mid-edge placement* (canonical seats the start point
    on the perimeter, possibly mid-edge): the port's rect loops have only corner
    vertices, so corner-nearest selection is the faithful port; mid-edge seating
    would invent geometry the rect loop cannot carry. Recorded as a divergence.
  - *Adding ORCA_CONFIG_PADDING twins* for the five keys — rejected: packet
    254/255/257 precedent says module-manifest bool/int/float/enum defaults do
    not thread into raw config; padding twins would emit five lines the port's
    defaults cannot produce at runtime. AC-6 pins the honest absence.
  - *Wiring `skirt_type`/`min_skirt_length` into the rect-loop generator* —
    rejected: decision points don't exist here (per-object grouping, extruded
    length); declaring them live would fake parity.

## Files in Scope (read + edit)

- `modules/core-modules/skirt-brim/skirt-brim.toml` — role: owner manifest; expected change: +5 `[config.schema]` tables (AC-1).
- `modules/core-modules/skirt-brim/src/lib.rs` — role: owner module; expected change: +3 config fields/reads, three gates in `run_finalization` + `process()` + generators, start-corner rotation helper (AC-2/3/4, AC-N1).
- `modules/core-modules/skirt-brim/Cargo.toml` — role: dev-dep; expected change: +`toml = "0.8"` dev-dependency (guard test), add-if-absent.
- `modules/core-modules/skirt-brim/tests/skirt_config_schema_tdd.rs` — role: net-new guard test (AC-1/N2); expected change: created.
- `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` — role: module tests; expected change: +1 loop-count test, +1 start-corner test, +1 gap-keys-inert test, +1 default-identity test (AC-3/4/N1).
- `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` — role: live-path driver; expected change: +2 tests (span enabled/disabled identity) (AC-2).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — role: scheduler arm; expected change: +2 rejection tests against the real manifest (AC-5).
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — role: CONFIG_BLOCK arm; expected change: +2 tests (explicit-value single emission; default absence) (AC-6).
- `docs/15_config_keys_reference.md` — role: generated; expected change: regenerated via `cargo xtask gen-config-docs` (AC-7).

## Read-Only Context

- `crates/slicer-gcode/src/serialize.rs` - lines `490-560` only - purpose: `ORCA_CONFIG_PADDING` inventory (verify the five keys are absent; never edit).
- `crates/slicer-ir` `ExtrusionPath3D` definition - purpose: `order_lock` field semantics (None preserves start invariance but does not govern start selection).
- `modules/core-modules/seam-planner-default/seam-planner-default.toml` lines `27-33` - purpose: canonical enum-table form (`type = "enum"` + `values = [...]`).
- `modules/core-modules/tree-support-planner/src/lib.rs` lines `226-234` - purpose: enum config-read pattern (invalid falls back to default).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (sibling path `..\pinch_n_print_cli\OrcaSlicerDocumented`).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-gcode/src/serialize.rs` `ORCA_CONFIG_PADDING` - read-only; must not gain or lose entries (AC-6).
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: does the real-manifest bounds index reject an undeclared enum value / out-of-range float via `resolve_global_config`?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`; return: `FACT`; purpose: Step 4.
- Question: does the emitted CONFIG_BLOCK contain exactly one `; skirt_type = perobject` line for an explicit value, and zero lines for the five keys at defaults?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`; return: `FACT`; purpose: Step 4.
- Question: did `cargo xtask gen-config-docs --check` pass after regeneration and do the five keys appear?; scope: `docs/15_config_keys_reference.md` + xtask; return: `FACT`; purpose: Step 5.

## Data and Contract Notes

- IR/manifest contracts: the enum tables must use the in-tree form
  `type = "enum"` + `values = [...]` + `default` (grounded:
  `seam-planner-default.toml` `[config.schema.seam_position]`); bounds
  enforcement is host-side generic via `ConfigBoundsIndex::from_modules` —
  enum membership lands in `TypeMismatch` ("one of the manifest-declared enum
  values"), numeric min/max in `ConfigResolutionError::OutOfRange`
  (`resolve_global_config`, `crates/slicer-scheduler/src/config_resolution.rs`).
- WIT boundary: none touched — no WIT/world changes; the five keys ride the
  existing `ConfigView` string/int/float/bool plumbing.
- Determinism/scheduler constraints: the start-corner rotation must be
  deterministic pure geometry (bbox + angle → corner index), no float ambiguity
  beyond the angle math itself; ties (angle exactly between two corners) fall
  to the lower-index corner — pin this in the AC-4 test so the mapping is
  total. The loop closure invariant `points.first == points.last` holds under
  rotation (rotate the ring, re-append the first point as the closing point).
  `skirt_height` is unchanged in the disabled case, so the pre-packet identity
  holds exactly.

## Locked Assumptions and Invariants

- Default-path identity: with the five keys absent (or at canonical defaults),
  module output and the emitted G-code are byte-identical to pre-packet
  behavior (AC-2 disabled arm, AC-4 identity clause, AC-6 default arm, AC-N1).
- `ORCA_CONFIG_PADDING` is untouched (AC-6).
- The loop ring closure invariant (`first == last` point) survives rotation.
- No deviation rows: all five manifest defaults are canonical-identical, and
  the two declared-with-gap keys' defaults (0.0 / "combined") match canonical.
- No WIT/IR schema changes.

## Risks and Tradeoffs

- Packet 257 merges first into the same manifest and `from_config`; a
  same-module edit conflict is expected to be trivial (disjoint key sets and
  disjoint field additions) — resolved by implementing 257 first (queue
  ordering), with Step 1's `toml` dev-dep add-if-absent defending either order.
- The start-corner wiring makes the first loop's start observable in final
  G-code (verified at authoring: the emitter never rotates closed loops; path
  optimization permutes whole entities only). Risk: a future seam-style
  re-selection for skirt would silently undo the wiring's observability — the
  AC-4 module-level pin remains the contract.
- Declared-with-gap keys are honest-but-inert today; a future packet owning
  per-object skirt grouping or the per-filament model consumes them (queue
  rows, not this packet).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2-3 wiring, bounded module file ~420 lines)
- Highest-risk dispatch and required return format: the AC-6 CONFIG_BLOCK arm — `FACT` (a wrong-home test binary would silently pass; the dispatch must confirm the binary's setup actually drives the block: `gcode_header_thumbnail_config_blocks_tdd.rs` has `run_slice`-adjacent setup at authoring time — 1040 lines, real pipeline driver).

## Open Questions

- `[FWD]` If packet 257's `brim_type` gate lands first and adds shared helper
  structure to `from_config`, may this packet's Step 2 reuse it? Answer at
  implementation time by reading the tree then-current state; either answer
  changes no contract here.
- No `[BLOCK]` questions.