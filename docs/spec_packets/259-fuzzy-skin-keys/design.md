# Design: fuzzy-skin-keys

## Controlling Code Paths

- Primary code path: `modules/core-modules/fuzzy-skin/src/lib.rs` —
  `FuzzySkinModule::from_config` (the three existing `config.get` reads), the
  `run_wall_postprocess` entry (layer gate + per-wall loop-selection gate +
  the existing `apply_to_all || flags.any(fuzzy_skin)` decision), and the
  unchanged `apply_fuzzy_skin` generator.
- Neighboring tests/fixtures:
  `modules/core-modules/fuzzy-skin/tests/{fuzzy_skin_tdd,closed_loop_tdd,slicer_module_binding_tdd}.rs`
  (`fuzzy_skin_tdd.rs` is the primary driver with the `outer_wall`/`inner_wall`/
  `region_with_walls`/`ConfigViewBuilder` fixtures this packet's AC-2/3/N1 arms
  reuse); guard-pattern source
  `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs`
  (parses the TOML directly; part-cooling's Cargo.toml carries the `toml = "0.8"`
  dev-dependency fuzzy-skin will need).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- snake_case config key strings only (repo convention): the seven new keys and all `config.get` / `ConfigKey` strings are already snake_case by construction here.
- The five declared-with-gap keys are declared in the manifest but **never read** in `src/lib.rs` — declaring them must not perturb behavior (AC-N1). The two wired keys are read in `from_config` with fallback-to-default match arms (tree-support-planner's enum-read pattern for `fuzzy_skin` — invalid values fall back to the canonical default; host enum enforcement happens earlier at resolve time, AC-4).

## Code Change Surface

- Selected approach: declare the seven keys in the `fuzzy-skin` manifest with
  canonical defaults/bounds; widen `FuzzySkinModule` by two fields
  (`fuzzy_skin_type: FuzzySkinType` — a module-local enum mirroring canonical
  `FuzzySkinType` — and `fuzzy_skin_first_layer: bool`); implement two gates on
  the existing `run_wall_postprocess`: (1) layer gate — when
  `!fuzzy_skin_first_layer && layer_index == 0`, pass every wall through
  unchanged; (2) loop-selection gate — per wall, map `fuzzy_skin_type` to
  candidate classes (`disabled_fuzzy`/`hole` → none; `external`/`all` →
  `LoopType::Outer`; `allwalls` → every loop; `none` → `LoopType::Outer` with
  the per-vertex flag gate), then keep the existing
  `apply_to_all || flags.any(fuzzy_skin)` decision inside candidates; leave the
  five gap keys unread. One value correction in `crates/slicer-gcode/src/serialize.rs`
  `ORCA_CONFIG_PADDING`: `("fuzzy_skin", "none")` → `("fuzzy_skin", "disabled_fuzzy")`
  (the canonical default; no entries gained or lost).
- Exact functions, traits, manifests, tests, and fixtures:
  `fuzzy-skin.toml` `[config.schema]` + seven tables; `FuzzySkinModule` struct +
  2 new fields; `from_config` + 2 new reads with fallback-to-default match arms
  (tree-support-planner's enum-read pattern for `fuzzy_skin` — invalid values
  fall back to the canonical default, host enum enforcement happens earlier at
  resolve time, AC-4); `run_wall_postprocess` + layer gate + loop-selection
  gate; `apply_fuzzy_skin` untouched; `fuzzy_skin_tdd.rs` + enum-selection
  tests (AC-2), first-layer tests (AC-3), gap-keys-inert test (AC-N1), and
  updates to the existing layer-0/apply-to-all tests; `closed_loop_tdd.rs` +
  layer-0 fixture updates; `fuzzy_config_schema_tdd.rs` (net-new guard, AC-1/N2).
- Rejected alternatives and reasons:
  - *Wiring `fuzzy_skin_mode`* — rejected: canonical consumes it only in
    `fuzzy_extrusion_line` (the Arachne `ExtrusionLine` path); the port's module
    is a `fuzzy_polyline` (Polygon-path) port over `WallLoop` IR with no Arachne
    junction path. Declaring it live would fake parity.
  - *Wiring `fuzzy_skin_noise_type`/`octaves`/`persistence`/`scale`* — rejected:
    canonical consumes them in `get_noise_module`'s libnoise coherent modules;
    the port's xorshift RNG is the `UniformNoise` (classic) analogue, and no
    coherent-noise implementation exists in-tree. Wiring only `classic` would
    silently ignore the other five enum values.
  - *Adding `LoopType::Hole` to the IR* — rejected: an IR/schema change,
    queue-sized geometry work, out of scope for a Tier A plumbing packet; the
    `hole`/`all` gap is recorded instead.
  - *Removing or renaming `apply_to_all`* — rejected: ticket 07 keeps the 34
    Pinch-specific keys untouched; the packet only defines the interaction
    (enum selects loops; `apply_to_all` overrides flags within them).
  - *Adding `ORCA_CONFIG_PADDING` twins* for the seven keys — rejected: packet
    254/255/257/258 precedent says module-manifest bool/int/float/enum defaults
    do not thread into raw config; padding twins would emit seven lines the
    port's defaults cannot produce at runtime. AC-5 pins the honest absence.
    The pre-existing `fuzzy_skin` padding entry's *value* is corrected
    (`"none"` → `"disabled_fuzzy"`) because it contradicts the canonical default
    this packet declares; the entry count is untouched.

## Files in Scope (read + edit)

- `modules/core-modules/fuzzy-skin/fuzzy-skin.toml` — role: owner manifest; expected change: +7 `[config.schema]` tables (AC-1).
- `modules/core-modules/fuzzy-skin/src/lib.rs` — role: owner module; expected change: +2 config fields/reads, layer gate + loop-selection gate in `run_wall_postprocess` (AC-2/3, AC-N1).
- `modules/core-modules/fuzzy-skin/Cargo.toml` — role: dev-dep; expected change: +`toml = "0.8"` dev-dependency (guard test), add-if-absent.
- `modules/core-modules/fuzzy-skin/tests/fuzzy_config_schema_tdd.rs` — role: net-new guard test (AC-1/N2); expected change: created.
- `modules/core-modules/fuzzy-skin/tests/fuzzy_skin_tdd.rs` — role: module tests; expected change: +enum-selection tests, +first-layer tests, +gap-keys-inert test, +default-identity test; existing layer-0/apply-to-all tests updated (AC-2/3/N1).
- `modules/core-modules/fuzzy-skin/tests/closed_loop_tdd.rs` — role: closed-loop regression; expected change: layer-0 fixtures updated to layer 1 (AC-3 fallout).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — role: scheduler arm; expected change: +3 rejection tests against the real manifest (AC-4).
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — role: CONFIG_BLOCK arm; expected change: +2 tests (explicit-value single emission; default padding state) (AC-5).
- `crates/slicer-gcode/src/serialize.rs` — role: padding table; expected change: one value correction `("fuzzy_skin", "none")` → `("fuzzy_skin", "disabled_fuzzy")` in `ORCA_CONFIG_PADDING` (AC-5); no entries gained or lost.
- `docs/15_config_keys_reference.md` — role: generated; expected change: regenerated via `cargo xtask gen-config-docs` (AC-6).

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` `LoopType` definition (lines `2138-2152`) - purpose: no `Hole` variant — the structural basis for the `hole`/`all` gap.
- `modules/core-modules/seam-planner-default/seam-planner-default.toml` lines `27-33` - purpose: canonical enum-table form (`type = "enum"` + `values = [...]`).
- `modules/core-modules/tree-support-planner/src/lib.rs` lines `226-234` - purpose: enum config-read pattern (invalid falls back to default).
- `crates/slicer-model-io/src/loader.rs` lines `944-950` - purpose: 3MF sidecar String allowlist for `fuzzy_skin` (per-object metadata; separate surface; never edit).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (sibling path `..\pinch_n_print_cli\OrcaSlicerDocumented`).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-gcode/src/serialize.rs` `ORCA_CONFIG_PADDING` - read-only **except** the single `fuzzy_skin` value correction (Step 3); must not gain or lose entries (AC-5).
- `crates/slicer-model-io/src/loader.rs` - read-only context; the sidecar allowlist is a separate surface and must not be edited.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: does the real-manifest bounds index reject an undeclared enum value / out-of-range int/float via `resolve_global_config` for the `fuzzy-skin` manifest specifically?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`; return: `FACT`; purpose: Step 3.
- Question: does the emitted CONFIG_BLOCK contain exactly one `; fuzzy_skin = external` line for an explicit value, and zero lines for the seven keys at defaults?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`; return: `FACT`; purpose: Step 3.
- Question: did `cargo xtask gen-config-docs --check` pass after regeneration and do the seven keys appear?; scope: `docs/15_config_keys_reference.md` + xtask; return: `FACT`; purpose: Step 4.

## Data and Contract Notes

- IR/manifest contracts: the enum tables must use the in-tree form
  `type = "enum"` + `values = [...]` + `default` (grounded:
  `seam-planner-default.toml` `[config.schema.seam_position]`); bounds
  enforcement is host-side generic via `ConfigBoundsIndex::from_modules` —
  enum membership lands in `TypeMismatch` ("one of the manifest-declared enum
  values"), numeric min/max in `ConfigResolutionError::OutOfRange`
  (`resolve_global_config`, `crates/slicer-scheduler/src/config_resolution.rs`).
- WIT boundary: none touched — no WIT/world changes; the seven keys ride the
  existing `ConfigView` string/int/float/bool plumbing.
- Determinism/scheduler constraints: the loop-selection and layer gates are
  pure predicates over existing loop metadata (`loop_type`, `perimeter_index`,
  `layer_index`) — no new RNG state, no float ambiguity; the perturbation
  algorithm and its seeded RNG are untouched, so determinism is preserved
  exactly. The default path (`disabled_fuzzy`, `fuzzy_skin_first_layer = false`)
  is byte-identical to the pre-packet no-config behavior (inert module).

## Locked Assumptions and Invariants

- Default-path identity: with the seven keys absent (or at canonical defaults),
  module output is byte-identical to pre-packet behavior (AC-2 `disabled_fuzzy`
  arm, AC-3 layer-0 arm, AC-6 default arm, AC-N1). The pre-packet live-path
  default was already inert (per-vertex flags are never written by production
  paint segmentation — DEV-126 — and `apply_to_all` defaults false).
- `ORCA_CONFIG_PADDING` gains and loses no entries; the single `fuzzy_skin`
  value correction (`"none"` → `"disabled_fuzzy"`) is the only edit to
  `crates/slicer-gcode/src/serialize.rs` (AC-5).
- The five declared-with-gap keys are unread in `src/lib.rs` (AC-N1).
- `apply_fuzzy_skin` and its seeded RNG are untouched — determinism and the
  closed-loop invariants (`closed_loop_tdd.rs`) hold unchanged.
- No deviation rows: all seven manifest defaults are canonical-identical, and
  the gap keys' defaults (displacement / classic / 4 / 0.5 / 1.0) match
  canonical.
- No WIT/IR schema changes.

## Risks and Tradeoffs

- The two gates change observable behavior for two existing configurations:
  `apply_to_all = true` alone no longer fuzzes (the enum's default
  `disabled_fuzzy` is the master gate), and layer 0 is no longer fuzzed at
  default `fuzzy_skin_first_layer = false`. Both are canonical-alignment
  changes; the existing tests that pinned the old behavior are updated in the
  same step with measured justification, and the changes are recorded in the
  wiring notes.
- The `hole`/`all` values are honest-but-partial today: `hole` is inert and
  `all` degrades to `external` (contour only) because the IR cannot identify
  hole boundaries. A future IR change (a `LoopType::Hole` variant or hole
  metadata on `WallLoop`) can consume them; recorded as a divergence, not
  fixed here.
- The five declared-with-gap keys are honest-but-inert today; a future packet
  owning coherent-noise generation or the Arachne extrusion-line path consumes
  them (queue rows, not this packet).
- The padding value correction changes one CONFIG_BLOCK line for existing slices
  at defaults (`; fuzzy_skin = none` → `; fuzzy_skin = disabled_fuzzy`). This is
  a canonical-alignment correction (the pre-existing value contradicted the
  canonical default); the AC-5 default arm pins the post-packet state, and the
  runtime binary's existing CONFIG_BLOCK tests are checked for fallout in Step 3.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 wiring + test fallout, bounded module file ~336 lines + two test files)
- Highest-risk dispatch and required return format: the AC-5 CONFIG_BLOCK arm — `FACT` (a wrong-home test binary would silently pass; the dispatch must confirm the binary's setup actually drives the block: `gcode_header_thumbnail_config_blocks_tdd.rs` has `run_slice`-adjacent setup at authoring time — 1040 lines, real pipeline driver, verified for packet 258's keys).

## Open Questions

- `[FWD]` If packet 258's `emit_config_kv` dedup machinery lands first and
  changes the CONFIG_BLOCK emission path, may this packet's Step 3 reuse it?
  Answer at implementation time by reading the tree then-current state; either
  answer changes no contract here.
- No `[BLOCK]` questions.
