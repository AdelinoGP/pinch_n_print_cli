# Design: support-support-keys

## Controlling Code Paths

- Primary code path A (wiring): `resolve_contact_params` in `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` — the `SupportContactParams` literal hardcodes `enforce_support_layers: 0` behind the comment "New bridge / sharp-tail / enforce / layer-index knobs have no production config source yet; neutral values keep the candidate stream unchanged (all stages OFF)". The replacement read is the typed CLI-bound field `config.enforce_support_layers` (`crates/slicer-ir/src/resolved_config.rs`, `u32 = 0`, `extract_int_as_u32`), the same typed-field pattern the function already uses for `support_threshold_angle` (the doc comment at the top of the function warns that `extensions` lookups silently ignore CLI-bound values). Downstream: `SupportContactParams` (`crates/slicer-core/src/algos/overhang_annotation.rs`), whose `force_support = params.layer_id < params.enforce_support_layers` branch forces `lower_layer_offset = 0.0`, and the existing arms `enforce_support_layers_forces_full_contacts_in_leading_layers` / `enforce_support_layers_beyond_model_changes_nothing` in `crates/slicer-core/tests/support_overhang_detection_tdd.rs` already pin the branch geometry.
- Primary code path B (declarations): `modules/core-modules/tree-support-planner/tree-support-planner.toml` and `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` `[config.schema]` — nine/eight net-new tables (AC-1/AC-2); `modules/core-modules/traditional-support/traditional-support.toml` `[config.schema.support_style]` type correction string → enum (AC-2). Manifest tables parse through `crates/slicer-scheduler/src/manifest.rs` (the `percent` field type with `default = "50%"` string form is packet-150 machinery; `min`/`max` floats) and flow into `ConfigBoundsIndex::from_modules` + `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`; `TypeMismatch` for non-enum-member strings, `OutOfRange` for numeric bounds — percent values checked numerically per `check_scalar`, verified by the existing `rejects_unknown_support_style_value` arm of `config_bounds_enforcement_tdd.rs`).
- Neighboring tests/fixtures: `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` (the `make_planner_config(entries: &[(&str, ConfigValue)]) -> ConfigView` builder at line ~1481 and the `SupportPlanner` + `run_support_geometry_with_analysis` driving pattern this packet's AC-N1 harness replicates in a new file — the parity file itself is out of scope; packet 261's planned AC-2 arms claimed it); the producer's own `#[cfg(test)] mod tests` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs:737` — `support_enabled_config()` / `square()` / `global_layers()` fixtures; the `resolve_contact_params_uses_typed_threshold_overlap_percent_and_literal` arm is the AC-3 template); `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (real-manifest load via `load_module_from_paths`; `rejects_unknown_support_style_value` / `out_of_range_support_threshold_angle_is_rejected` arms are the AC-4 template); `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (proven CONFIG_BLOCK driver at packets 257–264 authoring); `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` (host-key parity pins — unchanged by this packet).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- snake_case config key strings only (repo convention): all twelve keys and the existing `config.get` strings are already snake_case by construction here.
- The nine manifest-declared keys are **never read** in either planner's `src/lib.rs` (with-gap keys) or are read host-side only via typed fields (the wired four) — declaring them must not perturb module behavior (AC-N1). `support_type` IS read by the planners' `canonical_support_family`, so AC-N1 uses the family-consistent value (`"tree(auto)"` for the tree planner run; explicitly setting `"tree(auto)"` equals absence for the tree family).
- The `percent` field type with a `"<n>%"` string default is the in-tree form for `support_threshold_overlap` (`support_interface_flow` precedent in `traditional-support.toml`; `default` parsed by `manifest.rs`'s percent branch, bounds enforced numerically by `config_resolution.rs::check_scalar`); no `float_or_percent` form is needed because the manifest declares the canonical percent default.
- No schema/version constants, WIT worlds, or IR variants are touched — this change is manifest tables + one host function field read; no struct-literal blast radius exists (no Rust struct gains a field).

## Code Change Surface

- Selected approach: (1) wire `enforce_support_layers` as the only behavioral change — one field read in `resolve_contact_params` plus corrected comment and two unit arms; (2) declare the nine/eight tables in the two planner manifests and upgrade `support_style` in `traditional-support.toml` to the enum; (3) pin everything with three net-new manifest guards, a net-new planner non-perturbation harness, integration arms in two existing binaries, and doc regeneration. Everything else is verification.
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` `[config.schema]` — 9 tables (AC-1), each with `display` + `group = "Support"` and a `description` comment per disposition (with-gap keys name the canonical consumer; wired keys name the read site).
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` `[config.schema]` — 8 tables (AC-2), no `raft_first_layer_expansion` (AC-N2).
  - `modules/core-modules/traditional-support/traditional-support.toml` `[config.schema.support_style]` — type string → enum + 7 values (AC-2).
  - `modules/core-modules/tree-support-planner/tests/support_main_keys_schema_tdd.rs` — net-new guard (AC-1/N1/N2; `toml = "0.8"` dev-dep add-if-absent in `modules/core-modules/tree-support-planner/Cargo.toml`).
  - `modules/core-modules/traditional-support-planner/tests/support_main_keys_schema_tdd.rs` — net-new guard (AC-2/N2; dev-dep add-if-absent).
  - `modules/core-modules/traditional-support/tests/support_style_enum_schema_tdd.rs` — net-new guard (AC-2; dev-dep add-if-absent).
  - `modules/core-modules/tree-support-planner/tests/support_main_keys_nonperturbation_tdd.rs` — net-new planner-run harness (AC-N1) replicating `orca_parity_tdd.rs`'s `make_planner_config` + planner-call pattern locally.
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` — `resolve_contact_params`: `enforce_support_layers: 0` → `enforce_support_layers: config.enforce_support_layers`, comment correction, plus two unit arms in the existing tests mod (AC-3).
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — +4 arms (AC-4).
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — +AC-5 arms.
  - `docs/15_config_keys_reference.md` — generated; regenerated in Step 5 (AC-6).
- Rejected alternatives and reasons:
  - *Declaring the with-gap keys in the support geometry modules (`tree-support.toml` / `traditional-support.toml`) instead of the planners* — rejected: packet 260's owner correction put interface keys in the geometry modules because those modules' `from_config` reads the decision points; the P13 with-gap keys have no decision point anywhere, and the four wired host keys are read by the host, not by the geometry modules. The planner manifests hold the tier table's `support-planner` claim and already carry the family-agnostic `support_object_xy_distance`; the packet declares there and records the owner correction.
  - *Adding `support_threshold_angle` to `tree-support-planner.toml`* — rejected (recorded instead): the key is already wired and enforced via `traditional-support-planner.toml` + the typed-field read; the tree-side declaration asymmetry is pre-existing state and duplicating the table doubles guard surface with no behavior delta. The packet records the asymmetry in `requirements.md` instead of acting on it.
  - *Wiring `support_object_first_layer_gap` as a first-layer override of the planners' XY clearance* — rejected: both planners apply `support_object_xy_distance` uniformly; the canonical first-layer override (`gap_xy_first_layer` in `SupportParameters::SupportParameters`, the `obj_layer_nr == 0` branch of `TreeSupport::draw_circles`) is new geometry (Tier B+), correctly a declared-with-gap record, not a silent half-wiring.
  - *Adding `ORCA_CONFIG_PADDING` / `SUPPORT_CONFIG_DEFAULTS` twins* for the nine declared keys — rejected: packets 254–261 precedent — module-manifest defaults do not thread into raw config; the pre-existing lists already carry `support_type` / `support_style` / `support_expansion` / `support_bottom_z_distance` with canonical values; AC-5 pins the honest absence of the other six at defaults.
  - *Touching `docs/config/host-keys.toml` or `host_keys_doc_lock_tdd.rs`* — rejected: all eight host defaults already equal canonical (verified at authoring); no default moves.
  - *Extending packet 261's planned `raft_config_schema_tdd.rs` or 260's planned `support_config_schema_tdd.rs`* — rejected: those files are claimed by their packets' implementation plans; net-new guard filenames avoid all planned-file collisions (263/264 precedent).

## Files in Scope (read + edit)

- `modules/core-modules/tree-support-planner/tree-support-planner.toml` — role: owner manifest (planner claim, raft cluster); expected change: 9 tables (AC-1).
- `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` — role: owner manifest; expected change: 8 tables (AC-2/N2).
- `modules/core-modules/traditional-support/traditional-support.toml` — role: style consumer manifest; expected change: `support_style` string → enum (AC-2).
- `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` — role: host wiring + unit arms; expected change: one field read + comment + 2 tests (AC-3).
- `modules/core-modules/tree-support-planner/tests/support_main_keys_schema_tdd.rs` — role: net-new guard (AC-1/N1); expected change: created.
- `modules/core-modules/traditional-support-planner/tests/support_main_keys_schema_tdd.rs` — role: net-new guard (AC-2/N2); expected change: created.
- `modules/core-modules/traditional-support/tests/support_style_enum_schema_tdd.rs` — role: net-new guard (AC-2); expected change: created.
- `modules/core-modules/tree-support-planner/tests/support_main_keys_nonperturbation_tdd.rs` — role: net-new planner-run harness (AC-N1); expected change: created.
- `modules/core-modules/{tree-support-planner,traditional-support-planner,traditional-support}/Cargo.toml` — role: dev-deps; expected change: +`toml = "0.8"` each (add-if-absent; verified absent at authoring).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — role: scheduler arm; expected change: +4 AC-4 tests.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — role: CONFIG_BLOCK arm; expected change: +AC-5 tests.
- `docs/15_config_keys_reference.md` — role: generated; expected change: regenerated via `cargo xtask gen-config-docs` (AC-6).

## Read-Only Context

- `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` — lines `580-670` (`resolve_contact_params` + helpers) and `737-1270` (tests mod, ranged) — purpose: the wiring surface and the AC-3 arm template.
- `crates/slicer-core/src/algos/overhang_annotation.rs` — lines `168-200` (`SupportContactParams`) and `325-340` (the `force_support` branch) — purpose: the decision point; not edited.
- `modules/core-modules/tree-support-planner/tree-support-planner.toml` lines `97-127` (raft cluster — the `raft_first_layer_expansion` home) and `160-230` (the `support_object_xy_distance` / `support_style` tables — the declaration-form templates).
- `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` — `make_planner_config` (~line 1481) and one planner-call fixture test — purpose: harness pattern for `support_main_keys_nonperturbation_tdd.rs`; the parity file itself is read-only.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — real-manifest load + `rejects_unknown_support_style_value` / `out_of_range_support_threshold_angle_is_rejected` arms — purpose: AC-4 arm templates.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — setup + one existing CONFIG_BLOCK assertion — purpose: AC-5 arm form.
- `crates/slicer-scheduler/src/manifest.rs` / `config_resolution.rs` — the `percent` parse and `check_scalar` bounds paths — purpose: the percent-table declaration's legal form; ranged.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (sibling path `..\pinch_n_print_cli\OrcaSlicerDocumented`).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-gcode/src/serialize.rs` — read-only (the `ORCA_CONFIG_PADDING` / `SUPPORT_CONFIG_DEFAULTS` tables; AC-5 pins no edits).
- `docs/spec_packets/240-support-raft/` and packets 253–264 directories — read-only context (the future raft consumer's plan and the guard-filename collision checks); only the named references above may be consulted.
- `modules/core-modules/tree-support-planner/src/lib.rs` — read-only beyond the `from_config` config-get regions and the `canonical_support_family` / `TreeSupportStyle::from_config` reads already pinned in `requirements.md` (~4000 lines; never browse).
- `docs/config/host-keys.toml` and `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` — read-only context (host defaults already canonical; no edits).
- `crates/slicer-model-io/src/loader.rs` — read-only context for the per-object `support_type` divergence note, not a change surface.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: what is the exact planner-call fixture pattern (fixture geometry source, `ConfigView` construction, and `run_support_geometry_with_analysis` + `raft_plan()` assertion style) to replicate in `support_main_keys_nonperturbation_tdd.rs`, and which planner functions does the new file need publicly exported vs. accessible through `slicer_sdk` traits?; scope: `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` + `modules/core-modules/tree-support-planner/src/lib.rs` (entry/export surface only) + `modules/core-modules/tree-support-planner/tests/` fixtures; return: `SNIPPETS` (≤3, ≤30 lines each); purpose: Step 4.
- Question: does `config_bounds_enforcement_tdd.rs` load the real `tree-support-planner.toml` via `load_module_from_paths` such that newly declared enum/value keys (`support_type`, `raft_first_layer_expansion`, `enforce_support_layers`) will be picked up by `ConfigBoundsIndex::from_modules`, and do existing arms already prove `TypeMismatch` for enum strings and `OutOfRange` for floats (quote the two arms to mirror)?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` + `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 3.
- Question: does the runtime CONFIG_BLOCK driver thread explicit module-declared keys into `raw_config` for `serialize_config_block`, and do existing tests already assert `; support_type` / `; support_expansion` lines (quote one); does an explicit `raft_first_layer_expansion` value reach the block exactly once via the sorted dump?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` + `crates/slicer-gcode/src/serialize.rs` (`serialize_config_block` / `emit_config_kv` only); return: `FACT`; purpose: Step 3.
- Question: does `cargo xtask gen-config-docs --check` pass after regeneration, do the nine keys appear in the module-key table under the planner owner columns, does the `traditional-support` `support_style` row render as enum with the 7-value list, and does the deviations block still count 26 data rows?; scope: `docs/15_config_keys_reference.md` + xtask; return: `FACT`; purpose: Step 5.

## Data and Contract Notes

- IR/manifest contracts: the nine tables use the in-tree forms — `enum` with `values` (grounded: `tree-support-planner.toml` `[config.schema.support_style]`), `int` with `min`/`max` (grounded: `[config.schema.support_max_branches_per_layer]`), `float` with optional `max` (grounded: `[config.schema.max_bridge_length]` no-max form), `bool` (grounded: `[config.schema.enable_support]`), and `percent` with `default = "<n>%"` (grounded: `traditional-support.toml` `[config.schema.support_interface_flow]`); bounds enforcement is host-side generic via `ConfigBoundsIndex::from_modules` — numeric `OutOfRange`, enum/type `TypeMismatch` (`resolve_global_config`, `crates/slicer-scheduler/src/config_resolution.rs`), verified for packets 259–261's keys.
- WIT boundary: none touched — no WIT/world changes; the declared keys ride the existing `ConfigView` string/int/float/bool/percent plumbing (packet-150 machinery).
- Determinism/scheduler constraints: `support_type` already flows to family selection via the raw config (scheduler module-load read) and to both planners' `canonical_support_family` through `ConfigView`; the new manifest declarations add enum validation on the global path (AC-4) — a behavior change only for currently-invalid values: previously-silent garbage (e.g. `"banana"`) becomes a resolution error, which is canonical-faithful strictness (Orca's config parser rejects unknown enum values); family-consistent values behave identically (AC-N1 pins `"tree(auto)"` == absent for the tree planner).

## Locked Assumptions and Invariants

- Default-path identity: with the nine declared keys absent or explicit, the tree planner emits byte-identical `SupportPlanIR` + `RaftPlan` (AC-N1); the same holds for the traditional planner's eight keys by the same unread-ness (pinned by its guard + the shared rationale — the traditional non-perturbation run is not duplicated, the unread-ness is structural: zero `config.get` / `ConfigView` reads for those keys in `traditional-support-planner/src/lib.rs`, verified at authoring).
- `enforce_support_layers` default 0 keeps `force_support` false on every layer — the wiring is identity at defaults (AC-3's default arm).
- `traditional-support-planner.toml` does not declare `raft_first_layer_expansion` (AC-N2).
- `serialize_config_block` and both padding/support-default tables are untouched — no CONFIG_BLOCK twins (AC-5).
- `docs/config/host-keys.toml` and the 8 host defaults are untouched — no deviation rows; the block stays at 26 data rows (AC-6).
- No WIT/IR/schema-version changes; no struct gains a field — no struct-literal blast radius.
- The three edited manifests are guest-fingerprint inputs — `cargo xtask build-guests` must be run and `--check` must return exit 0 before closure.

## Risks and Tradeoffs

- **The with-gap declarations are honest-but-inert today**: users setting `raft_first_layer_expansion`, `support_bottom_z_distance`, `support_critical_regions_only`, `support_object_first_layer_gap` or `support_remove_small_overhang` see no behavior change until future geometry packets implement the decision points. This is the queue's declared-with-gap contract (packets 259/260/261 precedent), pinned by AC-N1 so the inertness is tested, not assumed.
- **`support_type` strictness on the global path**: currently-invalid global values that silently degraded to the traditional family become hard resolution errors. This is canonical-faithful (Orca rejects unknown enum values) and deliberately scoped: the per-object metadata path keeps its tolerant fallback, recorded as a divergence note — a user with a legacy sidecar spelling will see a clear error, not a silent family switch. Per-object values (3MF object metadata) bypass the global enum gate; the packet records this, it does not extend enforcement to the per-object path (that would change loader behavior beyond Tier A plumbing).
- **First implementation of the guard pattern**: the TOML-direct-parse guards exist only in packet plans (253/260/261) — Step 1 is the first to build one, and the `toml = "0.8"` dev-dep must be added per module; if the direct-parse form proves awkward, the real-manifest load pattern of `config_bounds_enforcement_tdd.rs` is the fallback (recorded, not assumed).
- **Same-manifest churn with packet 261**: `raft_first_layer_expansion` lands in the same `tree-support-planner.toml` raft cluster packet 261's plan edits — recorded as queue-order merge churn (packets 263/264 precedent), trivial when both land sequentially.
- **`support_scale`-class host reads**: `resolve_contact_params` reads `config.enforce_support_layers` typed, mirroring `support_threshold_angle`; the doc-comment rule about not consulting `extensions` for CLI-bound keys must be respected in the edit (a careless `extension_float` fallback would silently ignore configured values — the exact failure the function's own comment documents).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 — two integration-arm files, each requiring a driver read)
- Highest-risk dispatch and required return format: the CONFIG_BLOCK driver question (Step 3, `FACT` — a wrong assumption about how explicit keys reach `raw_config` would make AC-5 unbuildable) and the planner-harness replication question (Step 4, `SNIPPETS` — a wrong API assumption would make AC-N1 unbuildable).

## Open Questions

- `[FWD]` Does `config_bounds_enforcement_tdd.rs`'s real-manifest load cover a newly-declared `support_type` enum for the global-path `TypeMismatch` arm, or does the arm need `load_module_from_paths` to include both planner manifests? Either answer changes no contract here; the Step-3 dispatch settles it.
- `[FWD]` Does the AC-N1 harness replicate `make_planner_config` exactly (fixtures inline or in a shared `tests/` module), and is `run_support_geometry_with_analysis` publicly reachable? The Step-4 dispatch settles it.
- No `[BLOCK]` questions.
