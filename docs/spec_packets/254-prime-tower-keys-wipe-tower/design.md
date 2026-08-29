# Design: 254-prime-tower-keys-wipe-tower

## Controlling Code Paths

- Primary code path: `modules/core-modules/wipe-tower/wipe-tower.toml` (declarations) → `modules/core-modules/wipe-tower/src/lib.rs` (`WipeTower::from_config` + `generate_purge_paths` — the one live wiring) — both feed the guest build.
- Neighboring tests/fixtures: `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` (schema + geometry invariants; `line_w` fixture parameter at `fields.insert("line_width"...)` and the `wt.line_width()` assert); `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` (the percent-threading fixtures this packet mirrors, not edits).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **Percent basis divergence is deliberate.** Canonical pitches tower infill off `m_perimeter_width = nozzle_diameter × Width_To_Nozzle_Ratio`; this port has no nozzle→perimeter-width pipeline at the wipe-tower stage, so the wiring uses the module's existing `line_width` as the pitch basis: `advance = (percent/100) × line_width`. Do not introduce a nozzle-diameter read to "fix" this — that is Tier-B geometry work owned by a future packet. Record the divergence in the module code comment where the pitch is computed.
- **Depth-refitting is out of scope.** Canonically `m_extra_spacing` is re-fitted at runtime to make the tower fit its depth; the port takes the schema/config value as the fixed pitch factor. Note this next to the same comment.
- **Percent transport is one-way at the schema.** Only `percent`-typed manifest defaults thread into `ResolvedConfig.extensions` (packet-185 machinery); the bool/float/int defaults of the other 12 keys stay manifest-side and live at runtime in the module's `from_config` match fallbacks. Do not "help" by extending the scheduler's schema-default collection to other types — that would alter CONFIG_BLOCK bytes for unrelated modules.
- **The six canonically-vector keys are declared scalar.** Per queue ruling (ticket 04; Tier-D fog on the map), the `coFloats`/`coInts` keys become scalar globals here. Keep their `display` metadata free of per-filament promises; a note in `description` points at the Tier-D deferral.
- **Manifest additions are guest-visible.** The wipe-tower guest artifact embeds its manifest schema; edits here are stale-guest bait.
- <!-- snippet: wasm-staleness -->Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- <!-- snippet: coord-system -->Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Schema/version constants: none touched. No `PROGRESS_EVENT_SCHEMA_VERSION` or public constant bump exists in this change surface.

## Code Change Surface

- Selected approach: declare 13 keys in the owning manifest; wire the single key with a live decision point by replacing the hardcoded scan-line advance with the canonical percent formula; leave the 12 gap keys declared-but-unread with honest per-key gaps recorded (packet 253's `dont_slow_down_outer_wall` disposition). Scheduler side gets only tests (bounds + threading), zero production changes — the existing machinery already does everything AC-3 asserts.
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` — +13 `[config.schema.<key>]` tables.
  - `modules/core-modules/wipe-tower/src/lib.rs` — `WipeTower` struct: new `infill_gap_percent: f32` field (default 150.0); `from_config`: read `prime_tower_infill_gap` accepting `ConfigValue::Percent(p) => p` and `ConfigValue::FloatOrPercent { value, is_percent: true } => value` (float-typed percent-plain values rejected by bounds; do not add a bare-float arm); `generate_purge_paths`: `let pitch = self.infill_gap_percent / 100.0 * self.line_width;` replacing `y += self.line_width` (single advance site; the `y_min + line_width/2` start offset is untouched — canonical aligns scan lines onto the spacing grid, which is part of the absent planner).
  - `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` (new) — AC-1: parse `wipe-tower.toml` with the `toml` crate (mirroring part-cooling's schema test), assert the 21-key set with per-key type/default/bounds; guest-crate dev-dep on `toml = "0.8"` mirrors part-cooling's dev-dependencies (add to `modules/core-modules/wipe-tower/Cargo.toml` `[dev-dependencies]`).
  - `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` (edit) — AC-2 fallout: any assertion pinning scan-line count/advance must move to the canonical formula (e.g. `line_width = 0.4` now advances 0.6 by default; construct configs with `"prime_tower_infill_gap": "100%"` where the old hardcoded `line_width` pitch is wanted).
  - `crates/slicer-scheduler/tests/wipe_tower_config_bounds_tdd.rs` (new) — AC-3 + AC-N1: in-memory `LoadedModuleBuilder` fixtures (mirroring `config_resolution_tdd.rs::percent_schema_bounds` and `config_bounds_enforcement_tdd.rs`'s percent arms): bounds accept/reject for `prime_tower_infill_gap` (reject 99%, accept 110%) and `prime_tower_brim_width` (reject −2.0, accept −1.0 and 3.0); schema-default threading asserts `extensions["prime_tower_infill_gap"] == Percent(150.0)` under empty source, and asserts `prime_tower_brim_width` is *absent* from `extensions` (bool/float defaults do not thread).
  - `crates/slicer-runtime/tests/integration/` (edit one aggregated file) — AC-N2: add `undeclared_prime_tower_keys_stay_hidden_from_other_modules` to the integration bucket (registered by `tests/integration/main.rs`). No cross-module config-hiding arm exists in the tree at authoring time (grep `from_declared` finds comments only; packet 253's sibling arm lives in its own `draft` packet, not the tree) — this test creates it, mirroring packet 253's AC-N2 shape.
- Rejected alternatives and reasons:
  - *Wiring more keys* (e.g. `prime_tower_brim_width` → emit a first-layer brim): brim geometry is new emission logic (Tier B/C), not plumbing into an existing decision point; authoring it here would turn a Tier A packet into geometry work the queue deliberately sequenced later.
  - *Extending `schema_defaults` threading to all numeric types*: changes CONFIG_BLOCK bytes and scheduler behavior for every module; the packet-185 authors scoped percent-only deliberately.
  - *Removing the `ORCA_CONFIG_PADDING` entry for `prime_tower_brim_width`*: the padding table is host-side static filler; once user values ride the extensions bucket the dedup path handles collisions, and removing the entry could drop some G-code below Orca's minimum-key gate.
  - *Declaring float-list types with parsed vector defaults to model the per-filament vectors*: the manifest parser has no percent/list default → `extensions` path for lists; a scalar-global with a recorded Tier-D deferral is honest and small.

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `modules/core-modules/wipe-tower/wipe-tower.toml` - role: owner manifest (declarations); expected change: +13 schema entries.
- `modules/core-modules/wipe-tower/src/lib.rs` - role: the module (one live wiring + one field + comment); expected change: field, read arm, pitch line, divergence comment.
- `modules/core-modules/wipe-tower/Cargo.toml` - role: test dependency only; expected change: `toml = "0.8"` in `[dev-dependencies]`.

Justified extras (tests only, no production surface): `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` (new), `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` (pitch-fallout edits), `crates/slicer-scheduler/tests/wipe_tower_config_bounds_tdd.rs` (new), `crates/slicer-runtime/tests/integration/<the aggregated file that gains the AC-N2 arm>` (one new test; no such arm exists in-tree today — see Expected Sub-Agent Dispatches).

## Read-Only Context

Include ranges for files over 300 lines.

- `modules/core-modules/wipe-tower/src/lib.rs` - lines 30-60 (struct), 143-207 (`from_config`), 283-425 (`generate_purge_paths`) only - purpose: the wiring target; the file is 772 lines, read ranged.
- `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` - the three percent tests only (`percent_schema_bounds`, `percent_round_trip`, `percent_profile_value_overrides_schema_default`) - purpose: fixture shape to mirror.
- `crates/slicer-gcode/src/serialize.rs` - lines 332-460 only - purpose: confirm the config-block emission path (read-once during review; not edited).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` (sibling path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-scheduler/src/config_resolution.rs`, `crates/slicer-scheduler/src/manifest.rs` - production scheduler/IR files stay untouched; facts needed are quoted in `requirements.md` §Verified Grounding; delegate any further lookup
- `crates/slicer-gcode/src/serialize.rs` - read-only context above; never edited (`ORCA_CONFIG_PADDING` stays)
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: do the scheduler percent fixtures construct `ConfigFieldEntry` with `parsed_default` through `LoadedModuleBuilder::config_schema`? scope: `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`; return: `SNIPPETS` (≤ 30 lines); purpose: Step 2 fixture shape — **already answered at authoring time** (§Verified Grounding); re-dispatch only if the file moved.
- Question: which `slicer-runtime --test integration` file should carry the AC-N2 arm? scope: `crates/slicer-runtime/tests/integration/`; return: `LOCATIONS` (≤ 5 entries); purpose: Step 4 home for AC-N2 — **no cross-module config-hiding arm exists in-tree at authoring time** (grep `from_declared` finds comments only; packet 253's sibling arm is in its own `draft` packet); re-derive at implementation time and append to the thematically-nearest aggregated file (config/manifest reconciliation), or create a new file + `main.rs` registration if none fits.
- Question: (during implementation, only if a baseline test fails after the pitch change) which assertions hard-pin the old `line_width` pitch? scope: `modules/core-modules/wipe-tower/tests/`; return: `LOCATIONS` (≤ 10 entries); purpose: Step 3 fallout list.

## Data and Contract Notes

- IR/manifest contracts: no IR shape change; manifest `[config.schema]` entries follow the existing `ConfigFieldEntry` wire shape (type/default/min/max/display/group/description). `percent` type requires a string default ending in `%` (parser-enforced).
- WIT boundary: none touched — the module's WIT world is unchanged; only its config schema grows (transported through the existing `ConfigView` path).
- Determinism/scheduler constraints: the scan-line advance stays deterministic and layer-parallel-unsafe exactly as today (`layer-parallel-safe = false` in `[hints]` is untouched); no new config read ordering (`from_config` gains one match arm following the existing pattern).

## Locked Assumptions and Invariants

- The pitch at schema defaults is `(150/100) × line_width` — output-visible at defaults; baseline suites inside the module crate are updated, never weakened; no cross-crate golden pins the wipe-tower pitch (verified at authoring time: no `CONFIG_BLOCK` line-count/hash asserts exist in `crates/slicer-gcode/tests/` or `crates/pnp-cli/tests/`; the toolchange-wrapping test pins purge *volume*, not pitch).
- Non-percent declared defaults do not enter `ResolvedConfig.extensions` (packet-185 scoping stands).
- The six vector keys are scalar globals; no per-filament model is introduced.
- `prime_volume` (45.0) and `line_width` (0.4) semantics are unchanged.

## Risks and Tradeoffs

- **Output change at defaults** (pitch 0.4 → 0.6 mm): intended; makes the tower match Orca's line density at Orca's default. Cross-crate byte-golden suites (if any exist outside the surveyed set) would surface it — the verification matrix's workspace gates will catch stragglers; any found are updated in Step 3, named in the step's blast-radius list.
- **Declared-but-unread keys** may look like dead config to future agents: mitigated by the per-key disposition table (ticket 02 pattern) + `description` strings in the manifest pointing at the gap.
- **Percent key user values**: bounds `[100, …]` with no max mirrors canonical (`min = 100`, no max); a user could set `500%` — accepted, as canonical does.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3: wiring + test fallout; bounded by ranged reads)
- Highest-risk dispatch and required return format: the Step 3 fallout `LOCATIONS` (≤ 10 entries) — if it returns more than 10 pin sites, the pitch change is bigger than surveyed and the step must stop and re-scope to the coordinator.

## Open Questions

- [FWD] The AC-N2 arm has no existing in-tree arm to mirror (packet 253's is in its own `draft` packet). If the chosen integration file has no reusable config-with-module-extensions fixture, AC-N2 may need a small in-file helper — implementer-resolvable within the step's ≤ 3-edit cap (the new test + its fixture helper in the same file).
- No `[BLOCK]` questions.