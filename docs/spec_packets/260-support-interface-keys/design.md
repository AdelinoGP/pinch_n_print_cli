# Design: support-interface-keys

## Controlling Code Paths

- Primary code path: `modules/core-modules/{traditional-support,tree-support}/src/lib.rs` —
  `TraditionalSupport::from_config` / `TreeSupport::from_config` (the two
  `config.get("support_interface_spacing")` / `config.get("support_bottom_interface_spacing")`
  reads with fallback const `DEFAULT_INTERFACE_SPACING_MM`), and `pitches_mm` in both modules
  (the interface-pitch derivation: `top_gap = top_interface_spacing_mm.max(0.0)`,
  `bottom_gap = if bottom < 0.0 { top_gap } else { bottom }`, density via
  `slicer_core::support_regularize::{interface_density,bottom_interface_density}`, pitch =
  `interface_flow_spacing / density`). No reads are added for the pattern keys.
- Neighboring tests/fixtures:
  `modules/core-modules/{traditional-support,tree-support}/tests/{traditional_support_tdd,
  tree_support_tdd}.rs` (the `interface_paths(flow)` helpers + `interface_pitch_derives_from_
  interface_flow_over_line_width` / `zero_base_and_interface_spacing_clamp_to_solid_pitch`
  invariants this packet's AC-2/3/N1 arms extend); guard-pattern source
  `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` (TOML-direct parse;
  part-cooling's Cargo.toml carries the `toml = "0.8"` dev-dependency both support modules
  will need, verified absent at authoring). Integration arms:
  `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` and
  `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  (both proven drivers at packet 259 authoring time), plus
  `crates/slicer-runtime/tests/integration/support_family_closure.rs` (consumer of the
  `orca-matched-config.json` fixture).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- snake_case config key strings only (repo convention): all four keys and the existing
  `config.get` strings are already snake_case by construction here.
- The two declared-with-gap pattern keys are declared in the manifests but **never read** in
  either `src/lib.rs` — declaring them must not perturb behavior (AC-N1). The two spacing
  keys stay read exactly as today (fallback-to-default match arms); only their fallback
  const and toml defaults change (Step 2).
- Both support-family modules must end up **byte-identical in behavior for the two pattern
  keys and the loop key** — the tree and traditional manifests declare the same tables
  (AC-1 doubles), so no family asymmetry is introduced.

## Code Change Surface

- Selected approach: declare the four tables in both manifests (the two existing spacing
  tables updated to the aligned 0.5 default; the two pattern tables net-new); change both
  modules' `DEFAULT_INTERFACE_SPACING_MM` const and its "matches Orca" comment to 0.5;
  leave both modules' reading/wiring logic untouched (it is already canonically exact —
  `slicer_core::support_regularize` density formula verified identical to
  `SupportParameters`); add guard tests, invariant arms (AC-2/3/N1), bounds + CONFIG_BLOCK
  arms, and the fixture update. The bottom key's `< 0.0 → top gap` mirror branch stays
  (user ruling) and gains a pinned witness test + manifest comment.
- Exact functions, traits, manifests, tests, and fixtures:
  `traditional-support.toml` + `tree-support.toml` `[config.schema]` (4 tables each,
  AC-1); `DEFAULT_INTERFACE_SPACING_MM` const ×2 (`src/lib.rs` each, 0.4 → 0.5 + comment);
  `support_config_schema_tdd.rs` ×2 (net-new guard, AC-1/N2); `traditional_support_tdd.rs`
  + `tree_support_tdd.rs` (AC-2 default-reach arm, AC-3 mirror witness, AC-N1 gap-inert
  arms; plus fallout re-measurement of any default-0.4-pinned assertion);
  `crates/slicer-runtime/tests/fixtures/support-family/orca-matched-config.json`
  (0.4 → 0.5 value) + `support_family_closure.rs` (re-measured interface block counts);
  `config_bounds_enforcement_tdd.rs` (AC-4 arms); `gcode_header_thumbnail_config_blocks_tdd.rs`
  (AC-5 arms); both modules' `Cargo.toml` (`toml = "0.8"` dev-dep, add-if-absent);
  `docs/15_config_keys_reference.md` (generated, Step 4).
- Rejected alternatives and reasons:
  - *Aligning away the `-1` mirror* (min 0, drop `< 0.0` branch) — rejected by user ruling
    2026-08-31; the mirror is retained and recorded as an intended divergence instead. The
    alignment branch remains a documented future option (canonical-fidelity ticket), not
    this packet's work.
  - *Wiring `support_interface_pattern` dispatch* — rejected: canonical consumes it through
    the `contact_fill_pattern` branch order and `Fill::new_from_type`; the port has one
    scan-line generator with no pattern dispatch, no `FillConcentric`/`FillGrid`, and no
    `support_interface_angle` specialization (snug −45°, interlaced ±45°, grid
    = `base_angle`). Wiring only `rectilinear` would fake a dispatch; declaring with-gap and
    pinning non-perturbation (AC-N1) is the packet 259 pattern. Explicit `rectilinear` is
    behaviorally faithful by construction and the gap note says so.
  - *Wiring `support_interface_loop_pattern`* — rejected: canonical's only consumer is
    `LoopInterfaceProcessor` (`n_contact_loops`); no contact-loop generator exists in-tree.
    coBool default false is inert; AC-N1 pins explicit-true inertness too.
  - *Widening the spacing keys' bounds to canonical (no max, bottom min 0)* — rejected
    alongside the mirror ruling; declared-bounds divergences are recorded, not changed.
  - *Declaring in the planner modules* (`tree-support-planner`/`traditional-support-planner`)
    — rejected: they hold the `support-planner` claim (the tier table's owner) but never
    read interface config; the decision points are in the two support modules, which is
    where the four tables land. The owner correction is recorded in `requirements.md`.
  - *Adding `ORCA_CONFIG_PADDING`/`SUPPORT_CONFIG_DEFAULTS` twins* for the four keys —
    rejected: packet 254/255/257/258/259 precedent says module-manifest defaults do not
    thread into raw config; the block carries zero `support_interface_*` lines at defaults
    (AC-5 pins the honest absence; verified no pre-existing entries exist).

## Files in Scope (read + edit)

- `modules/core-modules/traditional-support/traditional-support.toml` — role: owner manifest (traditional family); expected change: 2 tables updated (0.5 default + divergence comment on bottom), 2 tables added (AC-1).
- `modules/core-modules/tree-support/tree-support.toml` — role: owner manifest (tree family); expected change: same four-table contract (AC-1).
- `modules/core-modules/traditional-support/src/lib.rs` — role: module source; expected change: `DEFAULT_INTERFACE_SPACING_MM` 0.4 → 0.5 + comment (AC-2); nothing else.
- `modules/core-modules/tree-support/src/lib.rs` — role: module source; expected change: same const/comment change (AC-2).
- `modules/core-modules/{traditional-support,tree-support}/Cargo.toml` — role: dev-deps; expected change: +`toml = "0.8"` (add-if-absent) ×2.
- `modules/core-modules/{traditional-support,tree-support}/tests/support_config_schema_tdd.rs` — role: net-new guard tests (AC-1/N2); expected change: created ×2.
- `modules/core-modules/{traditional-support,tree-support}/tests/{traditional_support_tdd,tree_support_tdd}.rs` — role: module suites; expected change: AC-2/3/N1 arms + any 0.4-default fallout (AC-2/3/N1).
- `crates/slicer-runtime/tests/fixtures/support-family/orca-matched-config.json` — role: fixture; expected change: `"support_interface_spacing": 0.4` → `0.5` (AC-2 fallout).
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` — role: fixture consumer; expected change: re-measured interface-count expectations if any pinned 0.4.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — role: scheduler arm; expected change: +AC-4 rejection + legality tests.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — role: CONFIG_BLOCK arm; expected change: +AC-5 tests.
- `docs/15_config_keys_reference.md` — role: generated; expected change: regenerated via `cargo xtask gen-config-docs` (AC-6).

## Read-Only Context

- `modules/core-modules/traditional-support/src/lib.rs` — lines `40-175` (from_config reads) and `330-430` (`pitches_mm`, density/pitch formula) — purpose: the wiring being verified/left untouched; the const edit is at lines `46-56`.
- `modules/core-modules/tree-support/src/lib.rs` — lines `50-100` (const + struct docs) and `260-360` (from_config reads + pitches_mm call site) — purpose: same wiring; plus the `pitches_mm` block near line `742` for the divergence branch.
- `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` — full — purpose: guard-test pattern source.
- `modules/core-modules/tree-support-planner/tree-support-planner.toml` lines `216-226` — purpose: canonical enum-table form (`type = "enum"` + `values = [...]`) and packet-comment precedent.
- `crates/slicer-gcode/src/serialize.rs` lines `330-470` (`serialize_config_block` + raw_config dump) and `560-566` (`SUPPORT_CONFIG_DEFAULTS` — none of the four keys; read-only) — purpose: AC-5's no-twins contract.
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` — lines `150-200` (slice runner + `interface_block_count`) and the interface-count assertions — purpose: fallout measurement.
- `crates/slicer-runtime/tests/fixtures/support-family/orca-matched-config.json` — full (small) — purpose: the one-value edit target.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (sibling path `..\pinch_n_print_cli\OrcaSlicerDocumented`).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/core-modules/tree-support-planner/` and `modules/core-modules/traditional-support-planner/` — the `support-planner` claim is context, not surface; never read beyond the toml enum-form reference above.
- `crates/slicer-gcode/src/serialize.rs` — read-only (the `ORCA_CONFIG_PADDING` / `SUPPORT_CONFIG_DEFAULTS` tables; AC-5 pins no edits).
- `docs/spec_packets/253* … 259*` — other packets' directories are read-only context; only the named reference files above may be consulted.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: does `config_bounds_enforcement_tdd.rs` drive the real `traditional-support.toml` manifest through the bounds index for float/enum/bool keys, and which existing test arms to mirror for the five AC-4 cases?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` + `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 3.
- Question: does the runtime CONFIG_BLOCK driver thread explicit module-declared keys (e.g. an explicit `support_interface_spacing = 0.8`) into `raw_config` for `serialize_config_block`, and do any existing tests already assert a `; support_interface_spacing` line?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` + `crates/slicer-runtime/src` (raw_config construction); return: `FACT`; purpose: Step 3.
- Question: which assertions in `support_family_closure.rs` count `;TYPE:Support interface` blocks or otherwise depend on the top-interface default pitch, and what are their current expected values?; scope: `crates/slicer-runtime/tests/integration/support_family_closure.rs`; return: `LOCATIONS` (≤20) + `FACT`; purpose: Step 2 fallout.
- Question: did `cargo xtask gen-config-docs --check` pass after regeneration, do the two pattern keys appear in the module-key table under both owner columns, do the spacing rows show 0.5, and does the deviations block count 25?; scope: `docs/15_config_keys_reference.md` + xtask; return: `FACT`; purpose: Step 4.

## Data and Contract Notes

- IR/manifest contracts: the enum tables must use the in-tree form `type = "enum"` +
  `values = [...]` + `default` (grounded: `tree-support-planner.toml`
  `[config.schema.support_style]`); bounds enforcement is host-side generic via
  `ConfigBoundsIndex::from_modules` — enum membership lands in `TypeMismatch`, numeric
  min/max in `ConfigResolutionError::OutOfRange` (`resolve_global_config`,
  `crates/slicer-scheduler/src/config_resolution.rs`), verified for packet 259's keys.
- WIT boundary: none touched — no WIT/world changes; the four keys ride the existing
  `ConfigView` string/int/float/bool plumbing.
- Determinism/scheduler constraints: the pitch derivation (`pitches_mm`) and the retained
  mirror branch are pure arithmetic over the two float fields — no new RNG state, no float
  ambiguity; the aligned default changes the numeric inputs, not the derivation, so
  determinism is preserved exactly. The pattern keys are unread, so they cannot reach any
  computation.

## Locked Assumptions and Invariants

- Default-path identity, post-alignment: with the four keys absent, both modules emit
  everything exactly as before **except** the top-interface pitch, which shifts from the
  0.4-gap to the 0.5-gap density (AC-2 pins absent == explicit 0.5). The bottom-key
  behavior at defaults is unchanged (0.5 both before and after, and no sentinel involvement
  when the key is absent).
- The `-1` mirror branch (`bottom_interface_spacing_mm < 0.0 → top gap`) in both modules
  is retained byte-for-byte and pinned by AC-3 — recorded divergence per user ruling.
- Neither pattern key is read in either module's `src/lib.rs` (AC-N1).
- `serialize_config_block` and both padding/support-default tables are untouched — no
  CONFIG_BLOCK twins (AC-5).
- The density formula `flow/(gap+flow)` in `slicer_core::support_regularize` is untouched
  and canonically exact (verified at authoring against
  `SupportParameters::SupportParameters`).
- No WIT/IR schema changes; no deviation-table additions — the block loses exactly the two
  `support_interface_spacing` rows (27 → 25, AC-6).

## Risks and Tradeoffs

- The default alignment changes observable default output for the support family: interface
  scan-lines become sparser (pitch 0.4+flow → 0.5+flow per layer). This is a
  canonical-alignment change with user ruling; the runtime suite
  (`support_family_closure.rs` interface block counts) is the likely fallout site and is in
  Step 2's edit list with measured justification.
- `support_family_closure.rs` uses `run_slice`-adjacent drivers; if the interface block
  count changes, the re-measured values must be justified from the pitch math (0.5/0.4
  ratio), not eyeballed.
- The retained mirror divergence is a permanent behavioral delta vs canonical for explicit
  `-1` inputs (canonical rejects them at bounds validation). The AC-3 witness + manifest
  comments keep it visible; a future canonical-fidelity ticket can align it under a new
  ruling.
- The two pattern keys are honest-but-inert today; a future packet owning pattern dispatch
  or contact loops consumes them (queue rows, not this packet).
- The guard tests require the `toml = "0.8"` dev-dependency in two module Cargo.tomls —
  add-if-absent per packet 257/258 precedent (verify, don't assume).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — const/comment ×2, fixture, consumer fallout, three new test
  arm groups across two suites)
- Highest-risk dispatch and required return format: the `support_family_closure.rs` fallout
  inventory — `LOCATIONS` + `FACT` (a wrong count expectation would silently pass a lenient
  AC or fail a strict one; the dispatch must pin current expected values).

## Open Questions

- `[FWD]` Do any OTHER test/golden sites beyond `support_family_closure.rs` pin the 0.4
  top-interface default (e.g. e2e G-code goldens under `crates/slicer-runtime/tests/e2e/`)?
  The Step-2 blast-radius dispatch inventories this before the alignment lands; either
  answer changes no contract here.
- `[FWD]` If the e2e goldens pin the 0.4 pitch, may they be re-recorded with measured
  justification in Step 2 (packet 257/258 re-baseline precedent applies)? Resolved at
  implementation time by the inventory results.
- No `[BLOCK]` questions.
