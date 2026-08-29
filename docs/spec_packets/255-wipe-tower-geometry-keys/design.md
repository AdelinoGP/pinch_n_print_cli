# Design: 255-wipe-tower-geometry-keys

## Controlling Code Paths

- Primary code path: `modules/core-modules/wipe-tower/wipe-tower.toml` (declarations) → `modules/core-modules/wipe-tower/src/lib.rs` (`WipeTower` + `from_config` + `generate_purge_paths` — the one live wiring, consumed by both `process()` and `run_finalization()`) — both feed the guest build.
- Neighboring tests/fixtures: `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` (geometry invariants), `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` + `config_bounds_enforcement_tdd.rs` (the percent/enum fixture shapes this packet mirrors, not edits).
- OrcaSlicer comparison: `packet.spec.md` §OrcaSlicer Reference Obligations owns the file list; do not repeat delegation rules here.

## Architecture Constraints

- **Flow-factor identity at defaults is the packet's safety property.** `wipe_tower_extra_flow` default 100% → factor 1.0 → scan-line paths identical to today's. Any test fallout is therefore *negative-case only* (new tests asserting the identity and the multiplier behaviour), not baseline churn. Do not "fix" the hardcoded `1.0` at the travel/prime sites — those flows are canonical too (zero-E travel, prime line at nominal flow).
- **Percent transport is one-way at the schema** (packet 185 machinery; packet 254 constraint stands): only `percent`-typed manifest defaults thread into `ResolvedConfig.extensions`. Here that is exactly `wipe_tower_extra_flow` and `wipe_tower_extra_spacing` (+2 CONFIG_BLOCK lines at defaults, spelled `100%` like Orca). The bool/float/enum defaults stay manifest-side with the module's read fallback as their runtime home. Do not extend scheduler threading to other types.
- **The enum key needs no bounds.** `wipe_tower_wall_type` is enforced by `ConfigBoundsIndex::check`'s enum-membership arm (String values), not by numeric min/max — declaring min/max on it would be inert machinery. Domain `["rectangle", "cone", "rib"]`, default `"rib"` (canonical `WipeTowerWallType::wtwRib`).
- **Do not re-derive percent semantics.** Canonical `coPercent` stores 100 = 100% (plain number, sidetext "%"); `wipe_tower_extra_flow` bounds are [100, 300] *in percent units*, and the module divides by 100 once at the compute site. Do not introduce a fraction representation.
- **`wipe_tower_max_purge_speed` is out of bounds for this packet.** It is an alias finding on host key `wipe_tower_speed` (`crates/slicer-ir/src/feedrate.rs::FeedrateConfig`, default 90.0 = canonical's default; consumed at `ExtrusionRole::WipeTower` in `crates/slicer-gcode/src/emit.rs::resolve_feedrate`). Do not declare it module-side and do not touch `crates/slicer-ir/src/feedrate.rs`; the rename question is wayfinder ticket 108's.
- <!-- snippet: wasm-staleness -->Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. (Authoring-time note: `tree-support-planner-guest` was stale on a clean tree — pre-existing, outside this packet's surface.)
- <!-- snippet: coord-system -->Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Schema/version constants: none touched. No `PROGRESS_EVENT_SCHEMA_VERSION` or public constant bump exists in this change surface. The module's WIT world is unchanged; config travels the existing `ConfigView` path.

## Code Change Surface

- Selected approach: declare 12 keys in the owning manifest; wire the single key with a live decision point by replacing the hardcoded scan-line `flow_factor: 1.0` with the canonical percent factor; leave the 10 gap keys declared-but-unread with honest per-key gaps recorded (packet 253/254 disposition); exclude the alias key with recorded evidence. Scheduler side gets only tests (bounds + threading + enum), zero production changes — the existing machinery already does everything AC-3 asserts (verified at authoring time: `percent` parsing in `manifest.rs::read_config_schema`, `schema_defaults` threading + enum collection in `config_resolution.rs::from_modules`, bounds/enum enforcement in `ConfigBoundsIndex::check`).
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` — +12 `[config.schema.<key>]` tables.
  - `modules/core-modules/wipe-tower/src/lib.rs` — `WipeTower` struct: new `extra_flow_factor: f32` field (default 1.0); `from_config`: read `wipe_tower_extra_flow` accepting `ConfigValue::Percent(p) => p / 100.0` and `ConfigValue::FloatOrPercent { value, is_percent: true } => value / 100.0` (a bare-float arm is deliberately absent — mirroring packet 254's read-arm pattern; percent-typed bounds reject plain floats before the module sees them); `generate_purge_paths`: scan-line points carry `flow_factor: self.extra_flow_factor` (two sites: forward and return, same path struct), travel stays 0.0 and prime keeps 0.0/1.0. A one-line comment at the compute site notes the [100, 300] percent semantics and the identity-at-defaults property.
  - `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` (new, or extended from packet 254's) — AC-1: parse `wipe-tower.toml` with the `toml` crate (part-cooling pattern), assert the key union (8 pre-existing + 13 P02 from 254 if landed, else 8 pre-existing + the 12 declared here — the precondition check) with per-key type/default/bounds, plus the `!contains_key("wipe_tower_max_purge_speed")` exclusion assert.
  - `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` or a new `wipe_tower_extra_flow_tdd.rs` — AC-2: config `"wipe_tower_extra_flow": "150%"` → every scan-line point's `flow_factor == 1.5` on both `process()` and `run_finalization()` outputs; `"200%"` → 2.0; empty config → 1.0 everywhere; travel/prime flows unchanged in all three cases.
  - `crates/slicer-scheduler/tests/wipe_tower_p03_config_bounds_tdd.rs` (new; flat file, auto-discovered binary) — AC-3 + AC-N1: in-memory `LoadedModuleBuilder` fixtures (mirroring `config_resolution_tdd.rs::percent_schema_bounds` and `config_bounds_enforcement_tdd.rs`'s percent arms): threading asserts `extensions["wipe_tower_extra_flow"] == Percent(100.0)` and `extensions["wipe_tower_extra_spacing"] == Percent(100.0)` under empty source; absent-from-`extensions` asserts for `wipe_tower_cone_angle` / `wipe_tower_wall_type`; bounds accepts `100%`/`300%` and rejects `99%`/`301%` naming the key; enum accepts `"rib"` and rejects `"hexagon"`.
  - `crates/slicer-runtime/tests/integration/` (edit one aggregated file) — AC-N2: add `undeclared_p03_wipe_tower_keys_stay_hidden_from_other_modules`, mirroring packet 254's AC-N2 shape (a non-owner module's `ConfigView::from_declared` hides `wipe_tower_extra_flow`).
- Rejected alternatives and reasons:
  - *Wiring `wipe_tower_extra_spacing` into the scan-line pitch*: canonical feeds it to wipe/ramming spacing arithmetic (`toolchange_Unload`/`set_toolchange`), not the infill pitch — the port's pitch is `prime_tower_infill_gap`'s decision point (packet 254). A second percent multiplying the same site would double-count a semantics the canonical tower does not have.
  - *Declaring `wipe_tower_max_purge_speed` alongside `wipe_tower_speed`*: creates the duplicate-spelling class ticket 107 collapses; the feedrate decision point already exists host-side with the canonical default.
  - *Renaming `wipe_tower_speed` → `wipe_tower_max_purge_speed` inside this packet*: renames are workstream tickets (99–107 pattern) with their own gates (parity harness, guest rebuild, 3MF round-trip), out of a Tier A declaration packet's scope; filed as ticket 108 instead.
  - *Wiring `purge_in_prime_tower`/`single_extruder_multi_material` to a zeroed purge*: there is no flush-matrix input to zero — "purge when false" would mean deleting the module's only behaviour, inventing a semantics canonical expresses through `extract_wipe_volumes`' matrix. Gap-recorded instead.
  - *Bounds on the enum key*: numerics-only bounds machinery would ignore min/max on an enum field; membership enforcement already covers it.

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `modules/core-modules/wipe-tower/wipe-tower.toml` - role: owner manifest (declarations); expected change: +12 schema entries.
- `modules/core-modules/wipe-tower/src/lib.rs` - role: the module (one read arm + one field + flow-factor sites + comment); expected change: field, read arm, two scan-line flow factor sites.
- `crates/slicer-scheduler/tests/wipe_tower_p03_config_bounds_tdd.rs` - role: new scheduler test binary (AC-3/AC-N1); expected change: new file.

Justified extras (tests only, no production surface): `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` (new-or-extended), `modules/core-modules/wipe-tower/tests/wipe_tower_extra_flow_tdd.rs` (new) or `wipe_tower_tdd.rs` (extended), `modules/core-modules/wipe-tower/Cargo.toml` (only if 254's `toml` dev-dep has not landed), `crates/slicer-runtime/tests/integration/<the aggregated file that gains the AC-N2 arm>` (one new test).

## Read-Only Context

Include ranges for files over 300 lines.

- `modules/core-modules/wipe-tower/src/lib.rs` - lines 30-60 (struct), 143-207 (`from_config`), 283-425 (`generate_purge_paths`), 540-568 (`run_finalization` insertion loop) only - purpose: the wiring target; the file is 772 lines, read ranged.
- `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` - the three percent tests only - purpose: fixture shape to mirror.
- `crates/slicer-gcode/src/serialize.rs` - lines 332-460 + 470-560 only - purpose: CONFIG_BLOCK emission + padding interplay (read-once during review; not edited).
- `crates/slicer-ir/src/feedrate.rs` - the `FeedrateConfig` struct + default + the `wipe_tower_speed` registration arm - purpose: alias-finding evidence; not edited.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` (sibling path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-ir/src/feedrate.rs`, `crates/slicer-scheduler/src/config_resolution.rs`, `crates/slicer-scheduler/src/manifest.rs` - production scheduler/IR files stay untouched; facts needed are quoted in `requirements.md` §Verified Grounding; delegate any further lookup
- `crates/slicer-gcode/src/serialize.rs` - read-only context above; never edited (`ORCA_CONFIG_PADDING` stays; the 3 P03 padding literals stay)
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: does any test pin scan-line `flow_factor == 1.0` (or copy the literal into expectations)? scope: `modules/core-modules/wipe-tower/` + `crates/slicer-gcode/tests/`; return: `LOCATIONS` (≤ 10 entries); purpose: Step 3 fallout list — **authoring-time survey found none** (the module fixture copies the literal; the gcode emitter has no wipe-tower-flow pin), re-derive before editing.
- Question: locate the registered integration file asserting cross-module config hiding (packet 254's AC-N2 shape)? scope: `crates/slicer-runtime/tests/integration/`; return: `LOCATIONS` (≤ 5 entries); purpose: Step 4 home for AC-N2 — re-derive at implementation time; packet 254's arm lives in its own `draft` packet, so it may or may not be in-tree by the time this packet runs.
- Question: which scheduler integration test constructs the `LoadedModuleBuilder` percent schema fixture? scope: `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`; return: `SNIPPETS` (≤ 30 lines); purpose: Step 4 fixture shape — verified at authoring time (§Verified Grounding); re-dispatch only if the file moved.

## Data and Contract Notes

- IR/manifest contracts: no IR shape change; manifest `[config.schema]` entries follow the existing `ConfigFieldEntry` wire shape (type/default/min/max/display/group/description; `percent` requires a `"<n>%"` string default, parser-enforced; `enum` requires `values = [...]`).
- WIT boundary: none touched — the module's WIT world is unchanged; only its config schema grows (transported through the existing `ConfigView` path).
- Determinism/scheduler constraints: the purge entity set, ordering, and insertion positions are unchanged; `layer-parallel-safe = false` in `[hints]` is untouched; no new config read ordering beyond one `from_config` match arm following the existing pattern.

## Locked Assumptions and Invariants

- Scan-line paths carry `flow_factor == (extra_flow_percent / 100.0)`; travel and prime entities are flow-invariant across all configs; identity at the manifest default.
- Non-percent declared defaults do not enter `ResolvedConfig.extensions` (packet-185 scoping stands); the two percent defaults do, adding exactly 2 CONFIG_BLOCK lines at defaults.
- The 12 declarations carry no per-filament promises (all P03 keys are canonically scalar; Tier-D fog not engaged).
- `wipe_tower_max_purge_speed` does not appear in any manifest; the host key `wipe_tower_speed` keeps its name pending wayfinder ticket 108.
- No cross-crate golden pins CONFIG_BLOCK line counts (verified at authoring time: the nearest asserts are `≥ 80` lines and block-occurrence counts, all satisfied by +2).

## Risks and Tradeoffs

- **CONFIG_BLOCK grows by 2 lines at defaults** (`wipe_tower_extra_flow = 100%`, `wipe_tower_extra_spacing = 100%`): intended — mirrors what Orca's own viewer writes. If a self-captured baseline suite outside the surveyed set pins the block's line count, it surfaces at the workspace gates and is updated in the owning step (named in its blast-radius result).
- **Declared-but-unread keys may look like dead config** to future agents: mitigated by the per-key disposition table + `description` strings in the manifest pointing at the gap.
- **The alias finding could be wrong**: if implementation discovers `wipe_tower_speed` is *not* semantically the max-purge-speed cap (e.g. canonical also caps against per-role speeds in a way the host arm cannot express), STOP and report to the coordinator before declaring anything; ticket 108's premise would need the human.
- **Percent key user values**: bounds [100, 300] with no max beyond 300 mirrors canonical exactly; a user wanting stronger purge is out of canonical's domain too.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3: wiring + fallout dispatch; bounded by ranged reads)
- Highest-risk dispatch and required return format: the Step 3 `flow_factor` pin `LOCATIONS` (≤ 10 entries) — if it returns more than 10 pin sites, the change is bigger than surveyed and the step must stop and re-scope to the coordinator.

## Open Questions

- **[FWD]** Packet 254 (same crate, `draft`) may land before or after this packet; AC-1's union assertion and Step 2's precondition re-derive the base manifest state from disk at implementation time rather than freezing a count.
- **[FWD]** The AC-N2 arm mirrors packet 254's shape but no such arm existed in-tree at authoring time (grep `from_declared` found comments only); if the chosen integration file lacks a reusable config-with-module-extensions fixture, a small in-file helper is implementer-resolvable within the ≤ 3-edit cap.
- No `[BLOCK]` questions.