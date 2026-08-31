# Design: raft-keys

## Controlling Code Paths

- Primary code path: `modules/core-modules/tree-support-planner/tree-support-planner.toml`
  `[config.schema]` — two net-new float tables (`raft_contact_distance`, `raft_expansion`)
  declared with canonical defaults/bounds, unread in `SupportPlanner::from_config`
  (`modules/core-modules/tree-support-planner/src/lib.rs` — the raft cluster reads
  `support_raft_layers` / `raft_first_layer_density` / `base_raft_layers` /
  `interface_raft_layers` only, and emits the configuration-only `RaftPlan` when
  `support_raft_layers > 0`). No reads are added for the two keys.
- Neighboring tests/fixtures:
  `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` (the
  `make_planner_config` + `run_support_geometry_with_analysis` + `output.raft_plan()`
  harness this packet's AC-2 arms extend — `SupportPlanEntry` and `RaftPlan` both derive
  `PartialEq`); guard-pattern source
  `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` (TOML-direct
  parse; part-cooling's Cargo.toml carries the `toml = "0.8"` dev-dependency
  tree-support-planner will need, verified absent at authoring). Integration arms:
  `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (already
  loads the real `tree-support-planner.toml` via `load_module_from_paths` — the
  `rejects_unknown_support_style_value` / `rejects_max_bridge_length_below_min` arms are
  the pattern) and
  `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  (proven CONFIG_BLOCK driver at packet 258/259/260 authoring time).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- snake_case config key strings only (repo convention): both keys and the existing
  `config.get` strings are already snake_case by construction here.
- The two declared-with-gap keys are declared in the manifest but **never read** in
  `src/lib.rs` — declaring them must not perturb behavior (AC-2).
- The no-max float form is legal and in-tree-grounded: `[config.schema.max_bridge_length]`
  in the same manifest declares `min = 0.0` with no `max`; the schema parser reads
  `min`/`max` via `get_float_opt` (`crates/slicer-scheduler/src/manifest.rs`) and the
  bounds index treats `None` as unbounded (`crates/slicer-scheduler/src/config_resolution.rs`
  `NumericBounds`).

## Code Change Surface

- Selected approach: declare the two float tables in `tree-support-planner.toml`
  `[config.schema]` with canonical defaults (0.1 / 1.5), canonical bounds (min 0.0, no
  max), `display` + `group = "Support"`, and a `description` comment per table recording
  the decision-point gap and canonical consumer; leave the planner's reading/wiring logic
  untouched (the keys are unread); add the manifest guard (AC-1/N1/N2), non-perturbation
  arms (AC-2), bounds + CONFIG_BLOCK arms (AC-3/4), and regenerate the docs (AC-5).
- Exact functions, traits, manifests, tests, and fixtures:
  `tree-support-planner.toml` `[config.schema]` (2 tables, AC-1);
  `raft_config_schema_tdd.rs` (net-new guard, AC-1/N1/N2); `orca_parity_tdd.rs` (AC-2
  arms); `config_bounds_enforcement_tdd.rs` (AC-3 arms);
  `gcode_header_thumbnail_config_blocks_tdd.rs` (AC-4 arms); the module's `Cargo.toml`
  (`toml = "0.8"` dev-dep, add-if-absent); `docs/15_config_keys_reference.md`
  (generated, Step 4).
- Rejected alternatives and reasons:
  - *Wiring the keys into `RaftPlan`* (add `raft_contact_distance` / `raft_expansion`
    fields to the WIT `raft-plan` record and emit them from the planner) — rejected:
    the raft geometry generator does not exist, so the fields would have no consumer;
    the change forces a WIT change + guest rebuilds; and draft packet 240 plans for
    `com.core.raft-default` to read config directly (its AC-5), making the RaftPlan
    fields redundant. Declaring with-gap and pinning non-perturbation (AC-2) is the
    packet 259/260 pattern.
  - *Declaring in `traditional-support-planner.toml` too* — rejected: the traditional
    family has no raft surface (no raft keys declared, no `RaftPlan` emitted, no raft
    handling in `traditional-support`); declaring there would invent a claim. The
    omission is pinned by AC-N2 instead.
  - *Deferring to draft packet 240-support-raft* — rejected: 240 is a draft geometry
    packet from the support-families plan, not preflighted, and the wayfinder queue's
    destination requires an authored + preflighted packet per in-scope feature. This
    packet is the config-reachability half (Tier A plumbing); 240's AC-5 wire-or-record
    decision consumes it. The relationship is recorded, not deferred.
  - *Adopting a port `max` on the floats* (packet 260's spacing-key precedent) —
    rejected: those keys kept a pre-existing port bound; these are net-new declarations,
    so canonical bounds (min 0, no max) are adopted outright — no declared-bounds
    divergence is created.
  - *Adding `ORCA_CONFIG_PADDING` / `SUPPORT_CONFIG_DEFAULTS` twins* for the two keys —
    rejected: packet 254/255/257/258/259/260 precedent says module-manifest defaults do
    not thread into raw config; the block carries zero `raft_contact_distance` /
    `raft_expansion` lines at defaults (AC-4 pins the honest absence; verified no
    pre-existing entries exist — the padding list's `("raft_layers", "0")` is the
    canonical layer-count key, not these two).

## Files in Scope (read + edit)

- `modules/core-modules/tree-support-planner/tree-support-planner.toml` — role: owner manifest (raft config cluster); expected change: 2 tables added (AC-1).
- `modules/core-modules/tree-support-planner/tests/raft_config_schema_tdd.rs` — role: net-new guard test (AC-1/N1/N2); expected change: created.
- `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` — role: module suite; expected change: AC-2 non-perturbation arms.
- `modules/core-modules/tree-support-planner/Cargo.toml` — role: dev-deps; expected change: +`toml = "0.8"` (add-if-absent).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — role: scheduler arm; expected change: +AC-3 rejection tests.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — role: CONFIG_BLOCK arm; expected change: +AC-4 tests.
- `docs/15_config_keys_reference.md` — role: generated; expected change: regenerated via `cargo xtask gen-config-docs` (AC-5).

## Read-Only Context

- `modules/core-modules/tree-support-planner/src/lib.rs` — lines `60-64` (raft-plan doc comment), `153-160` (raft struct fields), `1571-1590` (raft `from_config` reads), `1723-1731` (`push_raft_plan`) — purpose: the wiring being left untouched; the keys are unread.
- `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` — full — purpose: guard-test pattern source.
- `modules/core-modules/tree-support-planner/tree-support-planner.toml` lines `97-127` (existing raft tables) and `209-214` (`max_bridge_length` — the no-max float form) — purpose: table-form grounding.
- `crates/slicer-gcode/src/serialize.rs` lines `490-529` (`ORCA_CONFIG_PADDING` — `("raft_layers", "0")` present, neither raft key) and `562-566` (`SUPPORT_CONFIG_DEFAULTS` — neither raft key; read-only) — purpose: AC-4's no-twins contract.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — full (~460 lines) — purpose: AC-3 arm pattern (real-manifest load + `OutOfRange`/`TypeMismatch` assertions).
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror — purpose: AC-4 arm form.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (sibling path `..\pinch_n_print_cli\OrcaSlicerDocumented`).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/core-modules/traditional-support-planner/` and `modules/core-modules/traditional-support/` — the traditional family's raft absence is context, not surface; never read beyond the omission pin's needs.
- `crates/slicer-gcode/src/serialize.rs` — read-only (the `ORCA_CONFIG_PADDING` / `SUPPORT_CONFIG_DEFAULTS` tables; AC-4 pins no edits).
- `docs/spec_packets/240-support-raft/` — read-only context (the future consumer's plan); only the AC-5 wire-or-record relationship is cited.
- `docs/spec_packets/253* … 260*` — other packets' directories are read-only context; only the named reference files above may be consulted.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: does `config_bounds_enforcement_tdd.rs` drive the real `tree-support-planner.toml` manifest through the bounds index for float keys, and which existing test arms to mirror for the three AC-3 cases (two `OutOfRange`, one `TypeMismatch`)?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` + `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 3.
- Question: does the runtime CONFIG_BLOCK driver thread explicit module-declared keys (e.g. an explicit `raft_contact_distance = 0.5`) into `raw_config` for `serialize_config_block`, and do any existing tests already assert a `; raft_*` line?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` + `crates/slicer-runtime/src` (raw_config construction); return: `FACT`; purpose: Step 3.
- Question: does `cargo xtask gen-config-docs --check` pass after regeneration, do the two raft keys appear in the module-key table under the `tree-support-planner` owner column, and does the deviations block still count 27?; scope: `docs/15_config_keys_reference.md` + xtask; return: `FACT`; purpose: Step 4.

## Data and Contract Notes

- IR/manifest contracts: the float tables use the in-tree form `type = "float"` +
  `default` + `min` (no `max` — grounded: `tree-support-planner.toml`
  `[config.schema.max_bridge_length]`); bounds enforcement is host-side generic via
  `ConfigBoundsIndex::from_modules` — numeric min in `ConfigResolutionError::OutOfRange`,
  non-numeric value in `TypeMismatch` (`resolve_global_config`,
  `crates/slicer-scheduler/src/config_resolution.rs`), verified for packet 259/260's keys.
- WIT boundary: none touched — no WIT/world changes; the two keys ride the existing
  `ConfigView` string/int/float/bool plumbing.
- Determinism/scheduler constraints: the keys are unread, so they cannot reach any
  computation; the AC-2 byte-identity comparison relies on the planner's existing
  determinism (same inputs → same `SupportPlanEntry`/`RaftPlan` values — the suite's
  existing raft-plan test already depends on this).

## Locked Assumptions and Invariants

- Default-path identity: with the two keys absent or explicit, the planner emits
  byte-identical `SupportPlanIR` + `RaftPlan` (AC-2).
- Neither key is read in `tree-support-planner/src/lib.rs` (AC-2).
- `traditional-support-planner.toml` does not declare either key (AC-N2).
- `serialize_config_block` and both padding/support-default tables are untouched — no
  CONFIG_BLOCK twins (AC-4).
- No WIT/IR schema changes; no deviation-table additions — the block stays at 27 data
  rows (AC-5).

## Risks and Tradeoffs

- The declarations are honest-but-inert today: a user setting `raft_contact_distance` or
  `raft_expansion` sees no behavior change until the raft geometry generator lands
  (draft packet 240). This is the queue's declared-with-gap contract (packet 259/260
  precedent), pinned by AC-2 so the inertness is tested, not assumed.
- When packet 240 is implemented, the keys will be declared in two manifests
  (`tree-support-planner.toml` here, `com.core.raft-default` there) — the same-key-in-
  two-modules pattern packet 260's spacing keys already exercise; 240's AC-5 wire-or-
  record requirement is this packet's recorded input.
- The traditional family's raft absence is a port state, not a canonical one; AC-N2 pins
  the omission so a future traditional-raft packet must consciously update it.
- The guard tests require the `toml = "0.8"` dev-dependency in the module's Cargo.toml —
  add-if-absent per packet 257/258/259/260 precedent (verify, don't assume).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 — two integration-arm files, each requiring a driver read)
- Highest-risk dispatch and required return format: the CONFIG_BLOCK driver question —
  `FACT` (a wrong assumption about how explicit keys reach `raw_config` would make AC-4
  unbuildable; the dispatch must pin the mechanism).

## Open Questions

- `[FWD]` Does the CONFIG_BLOCK driver in `gcode_header_thumbnail_config_blocks_tdd.rs`
  thread explicit module-declared keys into `raw_config` via the same per-test config
  injection packet 258/259/260 used, or does it need a new injection path for the two
  raft keys? Either answer changes no contract here; the Step-3 dispatch settles it.
- No `[BLOCK]` questions.
