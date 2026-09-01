# Design: top-bottom-surface-keys

## Controlling Code Paths

- Primary code path: the manifest-declaration pipeline (`[config.schema]` tables in
  `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` parsed by
  `crates/slicer-scheduler/src/manifest.rs` (`ConfigFieldEntry`, bounds index via
  `ConfigBoundsIndex::from_modules` in `crates/slicer-scheduler/src/config_resolution.rs`),
  rendered into `docs/15_config_keys_reference.md` by `cargo xtask gen-config-docs`
  (`render_deviations` numeric-only comparison in `xtask/src/gen_config_docs.rs`)) plus
  the module wire: `RectilinearInfill::from_config` reads the two density keys via
  `config.get_abs_value` (the `sparse_infill_density` read pattern, percent/100
  fraction), and `run_infill`'s top/bottom solid blocks consume the fractions at
  `solid_spacing = line_width / density` (exposed surface, index 0) with the `density >
  0` gate, keeping `SOLID_DENSITY` 1.0 for internal solid (index ≥ 1).
- Consumed by NO module code: the two pattern keys — all four infill modules' sources are
  read-free pins for them (the no-reads grep over `rectilinear-infill/src`,
  `gyroid-infill/src`, `lightning-infill/src`, `infill-linker/src`). The only
  consumer-relevant code paths are canonical's, which this packet does not port.
- Neighboring tests/fixtures:
  `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs` (the
  `make_test_region` + `RectilinearInfill::from_config` + `run_infill` +
  `InfillOutputBuilder` harness the AC-2/AC-3/AC-N3 arms extend; the
  `ConfigViewBuilder` config form from `rectilinear_infill_tdd.rs`'s `make_config`);
  guard-pattern source `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs`
  (TOML-direct parse); enum-table form source `modules/core-modules/seam-planner-default/
  seam-planner-default.toml` (`[config.schema.seam_position]`); integration arms:
  `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (real
  `rectilinear-infill.toml` via `load_module_from_paths` + `ConfigBoundsIndex::from_modules`
  + `resolve_global_config` — the `rejects_value_below_min` arm is the pattern) and
  `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  (proven CONFIG_BLOCK driver at packet 258/259/260/261/262/263 authoring time:
  `run_pipeline_with_raw_config` + `region_between`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- snake_case config key strings only (repo convention): all 4 keys are snake_case by
  construction (canonical spellings, no aliases).
- The declared-with-gap pattern keys are declared in the manifest but **never read** in
  any module source — declaring them must not perturb behavior (AC-2 pins byte-identity
  for explicit values vs absent and the no-reads grep pins the sources).
- The wired density keys are default-path identity: at canonical defaults (100 → fraction
  1.0) the emitted paths are byte-identical to the pre-packet `SOLID_DENSITY` constant
  (AC-2); only non-default values change the solid spacing (AC-3).
- No unit conversion is performed by this packet: the manifest defaults are declared in
  canonical units (percent numbers for the density keys per the ticket-107 convention,
  enum strings for the pattern keys) and the wire divides by 100 at the read site, the
  `sparse_infill_density` pattern.
- ADR-0027 `gyroid-multi-role-fill-holder` conformance: this packet does not change the
  default `*_fill_holder` config (Decision #2 — defaults stay `"rectilinear-infill"`),
  does not remove top/bottom/bridge emission from gyroid-infill (Future-Reviewer note),
  and does not wire the P10 keys into gyroid's opt-in solid path (its solid emission
  rides the sparse density — a pre-existing divergence this packet records, not fixes).
  No amendment deviation is required. ADR-0030 (modifier splits) and ADR-0061
  (bridge-orientation tie-break) are not touched.

## Code Change Surface

- Selected approach: declare the 4 tables in `rectilinear-infill.toml` (AC-1); wire the
  two density keys into the top/bottom solid blocks (2 struct fields + `from_config`
  reads + the spacing/gate change); add the net-new guard test + `toml` dev-dep; add the
  identity/reachability/skip, bounds, and CONFIG_BLOCK arms; correct the
  `top_surface_pattern` padding twin; regenerate the docs and rebuild the guests. Zero
  module-source reads of the pattern keys; zero padding twins added.
- Exact functions, traits, manifests, tests, and fixtures:
  `rectilinear-infill.toml` `[config.schema]` (4 tables, AC-1);
  `rectilinear-infill/src/lib.rs` (`RectilinearInfill` struct +2 fields,
  `LayerModule::from_config` +2 reads, `run_infill` top/bottom solid blocks — the
  `solid_spacing` computation and the `density > 0` gate);
  `top_bottom_surface_config_schema_tdd.rs` (net-new guard, AC-1/N1/N2);
  `rectilinear-infill/Cargo.toml` (`toml = "0.8"` dev-dep, add-if-absent);
  `top_bottom_fill_tdd.rs` (AC-2/AC-3/AC-N3 arms);
  `config_bounds_enforcement_tdd.rs` (AC-4 arms);
  `crates/slicer-gcode/src/serialize.rs` (the single `ORCA_CONFIG_PADDING` value
  correction `("top_surface_pattern", "monotonic")` → `("top_surface_pattern",
  "monotonicline")`);
  `gcode_header_thumbnail_config_blocks_tdd.rs` (AC-5 arms);
  `docs/15_config_keys_reference.md` (generated, Step 5).
- Rejected alternatives and reasons:
  - *Wiring the density keys into gyroid's opt-in solid path* — rejected: gyroid's solid
    emission (`emit_polys` over `top_solid_fill()`/`bottom_solid_fill()`) rides the
    module's single `self.density` read from `sparse_infill_density` (ADR-0027 opt-in);
    wiring the P10 keys there would change that opt-in behavior at defaults (sparse 0.2 →
    solid 1.0), a behavior change this packet does not make. Recorded divergence; a
    future gyroid-solid-density packet re-opens it (AC-N2 pins the omission).
  - *Applying the density to internal solid too (whole top/bottom block)* — rejected:
    canonical `group_fills` gives `stInternalSolid` a fixed `100.f` (verified verbatim),
    so the wire keeps `SOLID_DENSITY` 1.0 for top_shell_index/bottom_shell_index ≥ 1 and
    reads the key only at index 0 — mirroring the existing `resolve_role_width` split.
  - *Wiring the pattern keys to a pattern→module mapping* — rejected: canonical's
    `top_surface_pattern`/`bottom_surface_pattern` select the filler class
    (`FillBase.cpp` `Fill::new_from_type`); the port's pattern is module identity
    selected by the host `*_fill_holder` resolution (packet 262's finding). A
    pattern→module mapping is host-side config-resolution work, not an infill-module
    decision point.
  - *Removing the `top_surface_pattern` padding twin instead of correcting it* —
    rejected: ticket 14's `fuzzy_skin` and packet 262's `sparse_infill_pattern`
    precedent correct the value to the canonical default; the twin's purpose (Orca-
    typical CONFIG_BLOCK lines) is preserved.
  - *Adding `ORCA_CONFIG_PADDING` twins for the density keys* — rejected: packet
    254/255/257/258/259/260/261/262/263 precedent says module-manifest defaults do not
    thread into raw config; the block must carry nothing for them at defaults (AC-5
    pins the honest absence).

## Files in Scope (read + edit)

- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` — role: owner manifest (default fill holder); expected change: 4 tables added (AC-1).
- `modules/core-modules/rectilinear-infill/src/lib.rs` — role: the density wire; expected change: +2 struct fields, +2 `from_config` reads, top/bottom solid blocks' spacing + gate (AC-2/AC-3/AC-N3).
- `modules/core-modules/rectilinear-infill/tests/top_bottom_surface_config_schema_tdd.rs` — role: net-new guard test (AC-1/N1/N2); expected change: created.
- `modules/core-modules/rectilinear-infill/Cargo.toml` — role: dev-deps; expected change: +`toml = "0.8"` (add-if-absent; packets 262/263 may have added it — verify, don't assume).
- `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs` — role: module suite; expected change: AC-2/AC-3/AC-N3 arms.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — role: scheduler arm; expected change: +AC-4 tests.
- `crates/slicer-gcode/src/serialize.rs` — role: padding correction; expected change: one `ORCA_CONFIG_PADDING` value (`top_surface_pattern` → `"monotonicline"`).
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — role: CONFIG_BLOCK arm; expected change: +AC-5 tests (net-new — no padding edits beyond the one correction).
- `docs/15_config_keys_reference.md` — role: generated; expected change: regenerated via `cargo xtask gen-config-docs` (AC-6).

## Read-Only Context

- `modules/core-modules/seam-planner-default/seam-planner-default.toml` — purpose: the in-tree `enum` manifest form (`[config.schema.seam_position]` with `values`).
- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` lines `38-44` — purpose: the in-tree float-percent form (`sparse_infill_density`) the two density tables mirror.
- `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` — full — purpose: guard-test pattern source.
- `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs` — purpose: the `make_config` helper's `ConfigViewBuilder` config form the AC-2/AC-3/AC-N3 arms use.
- `crates/slicer-gcode/src/serialize.rs` lines `455-470` (`serialize_config_block`'s padding loop + `emit_config_kv` dedup) — purpose: AC-5's twin/dedup context (the only edit is the padding value at line 505).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — full (~460 lines) — purpose: AC-4 arm pattern (real-manifest load + `OutOfRange`/`TypeMismatch` assertions).
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror — purpose: AC-5 arm form.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (sibling path `..\pinch_n_print_cli\OrcaSlicerDocumented`).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/core-modules/gyroid-infill/src/lib.rs`, `modules/core-modules/lightning-infill/src/lib.rs`, `modules/core-modules/infill-linker/src/*` — read-free pins for the 4 keys; never open them for reads (the no-reads grep is the evidence).
- `modules/core-modules/gyroid-infill/gyroid-infill.toml` and `modules/core-modules/lightning-infill/lightning-infill.toml` — omission pins (AC-N2); never edit.
- `crates/slicer-gcode/src/serialize.rs` — read-only except the single padding value correction (Step 4); no other edits.
- `docs/spec_packets/253* … 263*` — other packets' directories are read-only context; only the named reference files above may be consulted.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: does `config_bounds_enforcement_tdd.rs` drive the real `rectilinear-infill.toml` manifest through the bounds index for float and enum keys, and which existing test arms to mirror for the AC-4 cases (four float `OutOfRange`, two enum `TypeMismatch`, two valid)?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` + `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 3.
- Question: does the runtime CONFIG_BLOCK driver emit a corrected padding twin (`top_surface_pattern = monotonicline`) at defaults, zero lines for the two density keys, and an explicit non-padding float (`top_surface_density = 50.0`) exactly once via the raw-config sorted dump (packet-257 AC-5 precedent), with an explicit enum value suppressing its padding twin?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` + `crates/slicer-gcode/src/serialize.rs`; return: `FACT`; purpose: Step 4.
- Question: does `cargo xtask gen-config-docs --check` pass after regeneration, do the 4 keys appear in the module-key table under the `rectilinear-infill` owner column, and does the deviations block still count 26 data rows?; scope: `docs/15_config_keys_reference.md` + xtask; return: `FACT`; purpose: Step 5.

## Data and Contract Notes

- IR/manifest contracts: the float/enum tables use the in-tree forms
  (`rectilinear-infill.toml` `sparse_infill_density` for float percent;
  `seam-planner-default.toml` `seam_position` for enum; the `description` field is parsed
  by `crates/slicer-scheduler/src/manifest.rs`); bounds enforcement is host-side generic
  via `ConfigBoundsIndex::from_modules` — numeric min/max in
  `ConfigResolutionError::OutOfRange`, non-numeric value in `TypeMismatch`, unknown enum
  value in `TypeMismatch` with "unsupported enum value" (`resolve_global_config`,
  `crates/slicer-scheduler/src/config_resolution.rs`), verified for packet 259/260/261/
  262/263's keys.
- WIT boundary: none touched — no WIT/world changes; the 4 keys ride the existing
  `ConfigView` string/int/float/bool plumbing, and the two pattern keys are read by no
  module.
- Determinism/scheduler constraints: the density wire changes module computation only at
  non-default values; AC-2's byte-identity comparison relies on the module suite's
  existing determinism (same inputs → same paths).
- Deviation gate: `render_deviations` in `xtask/src/gen_config_docs.rs` parses the
  canonical Default column with `parse::<f64>` — `100%` fails and never enters the map
  (ticket 106's finding); the enum defaults are strings and never enter it either. Block
  stays at 26 rows.

## Locked Assumptions and Invariants

- Default-path identity: with the 4 keys absent or explicit-canonical-default, the
  rectilinear module emits byte-identical `InfillIR` (AC-2).
- The density wire is exposed-surface-only: top_shell_index/bottom_shell_index 0 reads
  the key fraction, ≥ 1 keeps `SOLID_DENSITY` 1.0 (canonical `group_fills`' fixed
  `100.f` for `stInternalSolid`).
- The `density > 0` gate on the top block is live (canonical `density <= 0` skip); the
  bottom gate is provably inert under the canonical min 10 (AC-4 pins the bound).
- Neither pattern key is read in any module source (the no-reads grep over all four
  infill module src dirs).
- `gyroid-infill.toml` and `lightning-infill.toml` do not declare any of the 4 keys
  (AC-N2).
- `crates/slicer-gcode/src/serialize.rs` gains exactly one edit: the
  `top_surface_pattern` padding value correction (AC-5) — no twins added or removed.
- No WIT/IR schema changes; no deviation-table additions — the block stays at 26 data
  rows (AC-6, re-measured at implementation time per the ledger-fact rule; 26 measured at
  264 authoring, 2026-09-01).
- No schema/version constants are bumped; the `RectilinearInfill` struct gains 2 fields
  with zero struct-literal sites (all 39 construction sites in the tree use
  `RectilinearInfill::from_config`, verified by grep at authoring).

## Risks and Tradeoffs

- The pattern keys are honest-but-inert today: a user setting either sees no behavior
  change until a pattern→module mapping packet lands. This is the queue's
  declared-with-gap contract (packet 259/260/261/262/263 precedent), pinned by AC-2
  (byte identity) and the no-reads grep.
- The density wire changes behavior at non-default values (spacing scales with
  `1/density`), which is the point of the Tier A plumbing — but a user setting
  `top_surface_density = 0` now suppresses top fill entirely (canonical behavior,
  AC-N3), which is a visible change from today's always-solid top. The canonical skip is
  verified verbatim; the AC-N3 arm pins it so the behavior is tested, not assumed.
- The gyroid opt-in solid path (ADR-0027) does not consume the P10 keys — a user with
  `top_fill_holder = "gyroid-infill"` setting `top_surface_density` sees no change in
  gyroid solid output (it rides the sparse density). Recorded divergence; AC-N2 pins the
  omission so a future gyroid-solid-density packet must consciously update.
- Merge churn with packets 262/263 on `rectilinear-infill.toml` (all append
  `[config.schema]` tables) and on `rectilinear-infill/Cargo.toml` (the `toml` dev-dep):
  262/263 implement first per queue order; all edits are appends, and the dev-dep is
  add-if-absent. The guard binaries are distinct (`infill_config_schema_tdd`,
  `infill_pattern_specific_config_schema_tdd`, `top_bottom_surface_config_schema_tdd`) —
  no file collision.
- The guest rebuild after Steps 1-2 is mandatory before Step 4's integration arm
  dispatches the real rectilinear guest — a stale guest surfaces as unrelated failures
  (wasm-staleness snippet).

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S` (Step 2 — the module wire + three test arms, the file with the most edit surface)
- Highest-risk dispatch and required return format: the CONFIG_BLOCK driver question —
  `FACT` (a wrong assumption about how corrected padding twins and explicit non-padding
  keys reach the block would make AC-5 unbuildable; packet-257 AC-5 and packet-262 AC-7
  already proved the mechanism, the dispatch re-confirms it against the current binary).

## Open Questions

- No `[BLOCK]` questions. All canonical evidence was verified by delegated read
  (declarations, `group_fills` per-surface-type assignments, the `stInternalSolid` fixed
  `100.f`, the `FillLine::_fill_surface_single` spacing formula, the `density <= 0` skip,
  the enum value list, the min/max bounds) and all in-tree symbols by authoring-time
  survey (2026-09-01).
- `[FWD]` Whether the AC-2/AC-3/AC-N3 arms' region fixture needs a new
  `top_shell_index(Some(1))` variant in `top_bottom_fill_tdd.rs` (the existing
  `make_test_region` only sets `Some(0)`/`None`) — no contract changes either way; the
  Step-2 read of the harness settles it.
