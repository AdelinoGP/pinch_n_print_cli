# Design: infill-pattern-specific-keys

## Controlling Code Paths

- Primary code path: the manifest-declaration pipeline only — `[config.schema]` tables in
  `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` parsed by
  `crates/slicer-scheduler/src/manifest.rs` (`ConfigFieldEntry`, bounds index via
  `ConfigBoundsIndex::from_modules` in `crates/slicer-scheduler/src/config_resolution.rs`),
  rendered into `docs/15_config_keys_reference.md` by `cargo xtask gen-config-docs`
  (`render_deviations` numeric-only comparison in `xtask/src/gen_config_docs.rs`), and
  surfaced in the G-code CONFIG_BLOCK only via explicit raw-config values
  (`crates/slicer-gcode/src/serialize.rs` `serialize_config_block` + `emit_config_kv`
  dedup; `ORCA_CONFIG_PADDING` carries none of the 10 keys — zero occurrences in `crates/`
  at authoring, so the default-state arm asserts honest absence).
- Consumed by NO module code: all four infill modules' sources are read-free pins for the
  10 keys (AC-2's no-reads grep over
  `rectilinear-infill/src`, `gyroid-infill/src`, `lightning-infill/src`,
  `infill-linker/src`). The only consumer-relevant code paths are canonical's, which this
  packet does not port.
- Neighboring tests/fixtures:
  `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` (the
  `make_config` + `RectilinearInfill::from_config` + `run_infill` + `InfillOutputBuilder`
  harness the AC-2 arm extends); guard-pattern source
  `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` (TOML-direct
  parse); bool-table form source `modules/core-modules/wipe-tower/wipe-tower.toml`
  (`enable_prime_tower`); integration arms:
  `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (real
  `rectilinear-infill.toml` via `load_module_from_paths` + `ConfigBoundsIndex::from_modules`
  + `resolve_global_config` — the `rejects_value_below_min` arm is the pattern) and
  `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  (proven CONFIG_BLOCK driver at packet 258/259/260/261/262 authoring time:
  `run_pipeline_with_raw_config` + `region_between`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- snake_case config key strings only (repo convention): all 10 keys are snake_case by
  construction (canonical spellings, no aliases).
- The declared-with-gap keys are declared in the manifest but **never read** in any module
  source — declaring them must not perturb behavior (AC-2 pins byte-identity for explicit
  values vs absent and the no-reads grep pins the sources).
- No unit conversion is performed by this packet: the manifest defaults are declared in
  canonical units (mm for the depth/lock keys, degrees for the angle keys, percent numbers
  for the density keys per the ticket-107 convention, unitless 0.0-fallback for the width
  keys per the in-tree width convention) and nothing downstream consumes them yet.
- ADR-0027 `gyroid-multi-role-fill-holder` conformance: this packet does not change the
  default `*_fill_holder` config (Decision #2 — defaults stay `"rectilinear-infill"`), does
  not point solid roles at gyroid (Future-Reviewer note), and does not remove
  top/bottom/bridge emission from gyroid-infill (Future-Reviewer note). The 10 keys are
  declared in `rectilinear-infill.toml` only; nothing is wired to the holder resolution.
  No amendment deviation is required. ADR-0030 (modifier splits) and ADR-0061
  (bridge-orientation tie-break) are not touched.

## Code Change Surface

- Selected approach: declare the 10 tables in `rectilinear-infill.toml` (AC-1); add the
  net-new guard test + `toml` dev-dep; add the inertness, bounds, and CONFIG_BLOCK arms;
  regenerate the docs and rebuild the guests. Zero module-source edits; zero padding twins.
- Exact functions, traits, manifests, tests, and fixtures:
  `rectilinear-infill.toml` `[config.schema]` (10 tables, AC-1);
  `infill_pattern_specific_config_schema_tdd.rs` (net-new guard, AC-1/N1/N2);
  `rectilinear-infill/Cargo.toml` (`toml = "0.8"` dev-dep, add-if-absent);
  `rectilinear_raw_emit_tdd.rs` (AC-2 arm); `config_bounds_enforcement_tdd.rs` (AC-3 arms);
  `gcode_header_thumbnail_config_blocks_tdd.rs` (AC-4 arms); `docs/15_config_keys_reference.md`
  (generated, Step 5).
- Rejected alternatives and reasons:
  - *Wiring `symmetric_infill_y_axis` into the rectilinear scan-line generator* (mirror the
    region polygon about a computed axis before scanning, mirror the emitted lines back) —
    rejected: canonical activates the flag only when the region's sparse pattern is
    `ipZigZag`/`ipCrossZag`/`ipLockedZag` (`Fill.cpp` `Layer::make_fills`, verified
    verbatim); the port ships none of those, and plain `ipRectilinear` never activates it.
    Wiring would implement behavior canonical never activates for this port's patterns, and
    the only in-module axis source (region bbox) diverges from canonical's
    `extended_object_bounding_box()` center. A zigzag-family packet re-opens the key.
  - *Declaring the 10 keys in `gyroid-infill.toml` / `lightning-infill.toml` too* — rejected:
    no canonical consumer exists outside `FillRectilinear` subclasses; declaring them there
    would fabricate decision points. The omission is pinned (AC-N2) so a future pattern
    packet must consciously update it.
  - *Wiring the density keys to the existing sparse-density read*
    (`sparse_infill_density` in `rectilinear-infill/src/lib.rs`) — rejected: canonical
    consumes them only inside `FillLockedZag::fill_surface_locked_zag` (skin/skeleton region
    split), which has no in-port analogue; subsuming them into the sparse density would
    change behavior canonical does not specify outside the locked-zag pattern.
  - *Adding `ORCA_CONFIG_PADDING` twins for the 10 keys* — rejected: packet
    254/255/257/258/259/260/261/262 precedent says module-manifest defaults do not thread
    into raw config; the block must carry nothing at defaults (AC-4 pins the honest absence).
  - *Porting the locked-zag/lateral-lattice/lateral-honeycomb pattern classes' geometry* —
    rejected: new geometry (Tier B+), the queue's cheapest-first ordering keeps this packet
    declaration-only; the consuming pattern packet re-opens the keys.

## Files in Scope (read + edit)

- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` — role: owner manifest (default fill holder); expected change: 10 tables added (AC-1).
- `modules/core-modules/rectilinear-infill/tests/infill_pattern_specific_config_schema_tdd.rs` — role: net-new guard test (AC-1/N1/N2); expected change: created.
- `modules/core-modules/rectilinear-infill/Cargo.toml` — role: dev-deps; expected change: +`toml = "0.8"` (add-if-absent; packet 262 may have added it — verify, don't assume).
- `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` — role: module suite; expected change: AC-2 arm.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — role: scheduler arm; expected change: +AC-3 tests.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — role: CONFIG_BLOCK arm; expected change: +AC-4 tests (net-new — no padding edits anywhere).
- `docs/15_config_keys_reference.md` — role: generated; expected change: regenerated via `cargo xtask gen-config-docs` (AC-5).

## Read-Only Context

- `modules/core-modules/wipe-tower/wipe-tower.toml` lines `66-73` — purpose: the in-tree `bool` manifest form (`[config.schema.enable_prime_tower]`).
- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` lines `93-104` — purpose: the in-tree width-table form (`internal_solid_infill_line_width`) and the float-percent form (`sparse_infill_density` at the file head).
- `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` — full — purpose: guard-test pattern source.
- `crates/slicer-gcode/src/serialize.rs` lines `490-560` (`ORCA_CONFIG_PADDING` + `emit_config_kv` dedup) — purpose: AC-4's honest-absence context (no edits).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — full (~460 lines) — purpose: AC-3 arm pattern (real-manifest load + `OutOfRange`/`TypeMismatch` assertions).
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror — purpose: AC-4 arm form.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (sibling path `..\pinch_n_print_cli\OrcaSlicerDocumented`).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/core-modules/rectilinear-infill/src/lib.rs`, `modules/core-modules/gyroid-infill/src/lib.rs`, `modules/core-modules/lightning-infill/src/lib.rs`, `modules/core-modules/infill-linker/src/*` — all module sources are read-free pins for the 10 keys; never open them for reads (AC-2's grep is the evidence).
- `modules/core-modules/gyroid-infill/gyroid-infill.toml` and `modules/core-modules/lightning-infill/lightning-infill.toml` — omission pins (AC-N2); never edit.
- `crates/slicer-gcode/src/serialize.rs` — read-only; zero padding edits (AC-4 pins honest absence).
- `docs/spec_packets/253* … 262*` — other packets' directories are read-only context; only the named reference files above may be consulted.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: does `config_bounds_enforcement_tdd.rs` drive the real `rectilinear-infill.toml` manifest through the bounds index for float/bool keys, and which existing test arms to mirror for the AC-3 cases (five float `OutOfRange`, one bool `TypeMismatch`, one valid bool)?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` + `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 3.
- Question: does the runtime CONFIG_BLOCK driver emit an explicit non-padding module key (e.g. `skin_infill_density = 30.0`) exactly once via the raw-config sorted dump (packet-257 AC-5 precedent), and does the defaults run carry zero lines for all 10 keys (no padding twins)?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` + `crates/slicer-gcode/src/serialize.rs`; return: `FACT`; purpose: Step 4.
- Question: does `cargo xtask gen-config-docs --check` pass after regeneration, do the 10 keys appear in the module-key table under the `rectilinear-infill` owner column, and does the deviations block still count 26 data rows?; scope: `docs/15_config_keys_reference.md` + xtask; return: `FACT`; purpose: Step 5.

## Data and Contract Notes

- IR/manifest contracts: the float/bool tables use the in-tree forms (`wipe-tower.toml`
  `enable_prime_tower` for bool; `rectilinear-infill.toml` `sparse_infill_density` for
  float percent; the `description` field is parsed by `crates/slicer-scheduler/src/manifest.rs`);
  bounds enforcement is host-side generic via `ConfigBoundsIndex::from_modules` — numeric
  min/max in `ConfigResolutionError::OutOfRange`, non-numeric value in `TypeMismatch`
  (`resolve_global_config`, `crates/slicer-scheduler/src/config_resolution.rs`), verified
  for packet 259/260/261/262's keys.
- WIT boundary: none touched — no WIT/world changes; the 10 keys ride the existing
  `ConfigView` string/int/float/bool plumbing, and none is read by any module.
- Determinism/scheduler constraints: the keys are declared-but-unread, so no module
  computation changes; AC-2's byte-identity comparison relies on the module suite's
  existing determinism (same inputs → same paths).
- Deviation gate: `render_deviations` in `xtask/src/gen_config_docs.rs` parses the
  canonical Default column with `parse::<f64>` — `25%`/`100%` fail and never enter the
  map (ticket 106's finding); the five parseable floats compare equal; the bool `false`
  matches canonical `0` under the ticket-100 bool comparison. Block stays at 26 rows.

## Locked Assumptions and Invariants

- Default-path identity: with the 10 keys absent or explicit-canonical-default, the
  rectilinear module emits byte-identical `InfillIR` (AC-2).
- None of the 10 keys is read in any module source (AC-2's no-reads grep over all four
  infill module src dirs).
- `gyroid-infill.toml` and `lightning-infill.toml` do not declare any of the 10 keys (AC-N2).
- `crates/slicer-gcode/src/serialize.rs` and `ORCA_CONFIG_PADDING` are untouched — no twins
  added (AC-4), so at defaults the CONFIG_BLOCK carries nothing for these keys.
- No WIT/IR schema changes; no deviation-table additions — the block stays at 26 data
  rows (AC-5, re-measured at implementation time per the ledger-fact rule; 26 measured at
  263 authoring, 2026-09-01).
- No struct fields or schema constants are added anywhere; no module binaries change
  (only the rectilinear guest rebuilds, driven by its manifest fingerprint).

## Risks and Tradeoffs

- The keys are honest-but-inert today: a user setting any of the 10 sees no behavior change
  until a locked-zag/lateral-*/zigzag-family pattern packet lands. This is the queue's
  declared-with-gap contract (packet 259/260/261/262 precedent), pinned by AC-2 (byte
  identity) and AC-4 (CONFIG_BLOCK) so the inertness is tested, not assumed.
- `symmetric_infill_y_axis` is the one key with a live in-port decision point (the
  rectilinear scan-line generator) that this packet deliberately does not wire, because
  canonical pattern-gates it off for every port-shipped pattern. A future zigzag-family
  packet must re-open it — the disposition description records the canonical gate and axis
  provenance so that packet has the evidence.
- Merge churn with packet 262 on `rectilinear-infill.toml` (both append `[config.schema]`
  tables) and on `rectilinear-infill/Cargo.toml` (the `toml` dev-dep): 262 implements
  first per queue order; both edits are appends, and the dev-dep is add-if-absent. The
  guard binaries are distinct (`infill_config_schema_tdd` vs
  `infill_pattern_specific_config_schema_tdd`) — no file collision.
- The guest rebuild after Step 1 is mandatory before Step 4's integration arm dispatches
  the real rectilinear guest — a stale guest surfaces as unrelated failures
  (wasm-staleness snippet).

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S` (Step 1 — manifest + guard + dev-dep, the file with the most edit surface)
- Highest-risk dispatch and required return format: the CONFIG_BLOCK driver question —
  `FACT` (a wrong assumption about how explicit non-padding keys reach the block would make
  AC-4 unbuildable; packet-257 AC-5 already proved the mechanism, the dispatch re-confirms
  it against the current binary).

## Open Questions

- No `[BLOCK]` questions. All canonical evidence was verified by delegated read
  (declarations, consumer functions, the `symmetric_infill_y_axis` activation gate) and all
  in-tree symbols by authoring-time survey (2026-09-01).
- `[FWD]` Whether the CONFIG_BLOCK driver's raw-config injection for explicit non-padding
  keys needs a new injection path or rides packet 258/259/260/261's per-test config
  injection — no contract changes either way; the Step-4 dispatch settles it.
