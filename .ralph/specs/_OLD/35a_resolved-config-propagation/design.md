# Design: 35a_resolved-config-propagation

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-host/src/region_mapping.rs::commit_region_mapping_builtin` — host built-in that today reads `region.resolved_config` from `LayerPlanIR` and clones it into `RegionPlan.config` (line `190-194`). This packet replaces the source of that value with a packet-supplied per-object map.
  - `crates/slicer-host/src/main.rs:135-150,218-229` — CLI entry point that already parses `--config` JSON and constructs `PipelineConfig`. This packet adds a resolver call between those two points and adds a new field to `PipelineConfig`.
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_multi_layer_top_bottom_evidence` (lines `~1599-1840`) — packet 35's two-part test. Part 1 (API) must remain green; Part 2 (binary E2E) is replaced/superseded by this packet's stricter binary E2E inequality test (Part 2 currently passes coincidentally because Benchy has many layers; the new test asserts strict inequality, which is the correct gate).
  - `crates/slicer-host/tests/external_surface_classification_tdd.rs` and `multi_layer_thickness_tdd.rs` — packet 35's API-level tests. Must remain green; this packet does not change the consumer surface.
- OrcaSlicer comparison surface: none. This packet is plumbing-only; the codebase already intentionally deviates on default values.

## Architecture Constraints

- The `ResolvedConfig` shape is fixed by `crates/slicer-ir/src/slice_ir.rs:575-660`. No schema bump (no field additions/removals).
- `harvest_layer_plan_ir` (`dispatch.rs:1612-1690`) is OUT OF SCOPE. `ActiveRegion.resolved_config` remains `ResolvedConfig::default()` from the module path; the host stamps the authoritative value at RegionMapping time. This narrows authority: modules cannot tamper with config that downstream stages will read.
- `bind_module_config_view` (`execution_plan.rs:63-91`) is UNCHANGED. Per-module `ConfigView` filtering by declared schema continues to work exactly as today.
- `parse_cli_config_source` (`execution_plan.rs:193-211`) is UNCHANGED. JSON shape limits (no nested objects, no `null`) persist.
- Per-object overlay key convention is `object_config:<object_id>:<config_key>` — flat, colon-delimited, mirrors the existing `object_height:<object_id>` host-injected pattern. `<object_id>` MUST match `ObjectMesh.id` (UUID-ish strings already produced by `model_loader`).
- Determinism: `BTreeMap<String, ResolvedConfig>` (not `HashMap`) is required for the per-object resolved map so iteration order is stable across runs (matches the `RegionMapIR` deterministic-iteration contract in `docs/04`).

## Code Change Surface

- Selected approach:
  1. Build a base `ResolvedConfig` from non-prefixed keys via `resolve_global_config(source)`.
  2. For each object id supplied by the caller, build a per-object `ResolvedConfig` by starting from the base and applying any `object_config:<id>:<key>` overrides via `resolve_per_object_configs(source, &object_ids)`.
  3. Thread the resulting `Arc<BTreeMap<String, ResolvedConfig>>` through `PipelineConfig` to the prepass execution path.
  4. `commit_region_mapping_builtin` accepts the map by reference and stamps each `RegionPlan.config` from `entries[*].object_id`, falling back to a `default` resolved config (provided by the caller alongside the per-object map) when an object has no entry.
  5. Strict typing: each declared `ResolvedConfig` field is read with the exact `ConfigValue` variant it expects (`Int` for `u32` fields, `Float` for `f32`, `Bool` for `bool`, etc.). A wrong-typed value yields `ConfigResolutionError::TypeMismatch`.
  6. Permissive on unknown keys: any non-declared, non-`object_config:`-prefixed, non-`object_height:`-prefixed key lands in `ResolvedConfig.extensions`.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/config_resolution.rs` (NEW): `resolve_global_config`, `resolve_per_object_configs`, `ConfigResolutionError`.
  - `crates/slicer-host/src/region_mapping.rs::commit_region_mapping_builtin`: signature gains `&BTreeMap<String, ResolvedConfig>` plus `&ResolvedConfig` (default fallback). `execute_region_mapping_with_cap` is unchanged for its existing signature; the per-object stamp happens in the commit layer.
  - `crates/slicer-host/src/pipeline.rs::PipelineConfig`: add `resolved_configs: Arc<BTreeMap<String, ResolvedConfig>>` and `default_resolved_config: Arc<ResolvedConfig>`.
  - `crates/slicer-host/src/prepass.rs:313,340`: forward the new fields from `PipelineConfig` to `commit_region_mapping_builtin`.
  - `crates/slicer-host/src/main.rs:135-229`: call `resolve_per_object_configs` on the parsed `config_source`, build the default, populate the new `PipelineConfig` fields. CLI exits non-zero on `ConfigResolutionError`.
  - `crates/slicer-host/src/lib.rs`: re-export the new module's public surface.
  - `crates/slicer-host/tests/config_resolution_tdd.rs` (NEW).
  - `crates/slicer-host/tests/region_mapping_resolved_config_tdd.rs` (NEW).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`: append two new tests (`benchy_user_top_shell_layers_propagates_through_binary`, `cli_rejects_top_shell_layers_string`).
- Rejected alternatives that were considered and why they were not chosen:
  - **Fix `harvest_layer_plan_ir` to populate `resolved_config` from a passed-in source**: rejected — bloats `HostExecutionContext`, leaks host-side resolution into the prepass dispatch path, and gives modules a place to tamper. Stamping at RegionMapping centralizes authority.
  - **Add a `ResolvedConfig::from_source` constructor in `slicer-ir`**: rejected (per user direction) — pulls `HashMap<ConfigKey, ConfigValue>` resolution semantics into the IR crate, which today knows nothing about the CLI input shape. Resolver lives in `slicer-host`.
  - **Fail-fast even on unknown keys**: rejected — breaks forward-compat with module-contributed config keys that aren't part of the central `ResolvedConfig` schema. `extensions` is the documented overflow bucket (`docs/02 §304-344`); use it.
  - **Use `HashMap<String, ResolvedConfig>` instead of `BTreeMap`**: rejected — non-deterministic iteration order would surface as flake on `RegionMapIR.entries` ordering downstream.
  - **Nested-object JSON shape (`{"object_overrides": {"obj-A": {...}}}`)**: rejected — `parse_cli_config_source` deliberately rejects nested objects (`docs/03 §host-boundary enforcement`). Flat colon-delimited keys reuse existing precedent.

## Files in Scope (read + edit)

Primary edited files (logic-bearing; ≤ 3):

- `crates/slicer-host/src/config_resolution.rs` (NEW) — role: the resolver itself; expected change: full module body (~150 lines) implementing `resolve_global_config`, `resolve_per_object_configs`, `ConfigResolutionError`. Top-of-file rustdoc MUST disambiguate this module from the adjacent `config_schema.rs`: state that `config_schema.rs` describes per-module manifest field shapes (author-time, declarative, drives `bind_module_config_view` and the `config-schema` CLI subcommand) while `config_resolution.rs` resolves user-supplied CLI input into per-object `slicer_ir::ResolvedConfig` (invocation-time, imperative, drives `RegionPlan.config`). Use `slicer_ir::ConfigValue` (from `parse_cli_config_source`); never `config_schema::ConfigValue`.
- `crates/slicer-host/src/region_mapping.rs` — role: stamp authority for `RegionPlan.config`; expected change: extend `commit_region_mapping_builtin` to take and apply the resolved-configs map; small inline helper to look up per-object value with default fallback.
- `crates/slicer-host/src/main.rs` — role: CLI entry point that resolves once and threads through `PipelineConfig`; expected change: ~10 added lines after `parse_cli_config_source`, plus structured-error exit on `ConfigResolutionError`.

Mechanical (signature/wiring) edits (justified, not primary):

- `crates/slicer-host/src/pipeline.rs` — add 2 fields to `PipelineConfig`.
- `crates/slicer-host/src/prepass.rs` — forward 2 args at call sites `:313` and `:340`.
- `crates/slicer-host/src/lib.rs` — `pub mod config_resolution; pub use config_resolution::*;`.

Test files:

- `crates/slicer-host/tests/config_resolution_tdd.rs` (NEW) — resolver unit coverage.
- `crates/slicer-host/tests/region_mapping_resolved_config_tdd.rs` (NEW) — RegionMapping stamp coverage.
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append 2 binary E2E tests.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` — read lines `570-660` only — purpose: confirm `ResolvedConfig` field set, types, defaults, and `extensions` overflow bucket; no other IR types relevant.
- `crates/slicer-host/src/execution_plan.rs` — read lines `45-211` only — purpose: confirm `parse_cli_config_source` and `bind_module_config_view` semantics this packet does NOT modify.
- `crates/slicer-host/src/dispatch.rs` — read lines `1612-1690` only — purpose: confirm `harvest_layer_plan_ir` is the producer-side default this packet leaves alone. NEVER load the rest of `dispatch.rs`.
- `crates/slicer-host/src/region_mapping.rs` — full file already short (~260 lines); safe to read directly when editing.
- `crates/slicer-host/src/pipeline.rs` — read lines `30-150` only (PipelineConfig struct + run_pipeline/run_pipeline_with_events entry points).
- `crates/slicer-host/src/prepass.rs` — read lines `300-360` only — purpose: locate the two `commit_region_mapping_builtin` call sites and confirm what's already in scope at each.
- `docs/02_ir_schemas.md` — read lines `~300-410` and `~575-660` (RegionMapIR + ResolvedConfig).
- `docs/04_host_scheduler.md` — delegate SUMMARY of "RegionMapIR Compilation" subsection only.
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — read lines `1280-1370` (firmware-retraction E2E pattern; mirrors what we want for `--config` E2E) and lines `1599-1840` (existing multi-layer evidence test) — purpose: copy structural pattern for `run_slicer_host` invocation; do NOT load the rest of the file (it's > 1800 lines).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — never load. No parity surface in this packet.
- `target/`, `Cargo.lock`, generated WASM/component artifacts under `modules/core-modules/*/target/` — never load.
- `crates/slicer-host/src/wit_host.rs` — never load. WIT boundary unchanged.
- `modules/core-modules/layer-planner-default/` — never load. Layer-planner module surface unchanged.
- `crates/slicer-macros/` — never load. No macro changes.
- `wit/` — never load. No WIT changes.
- `crates/slicer-host/src/dispatch.rs` outside lines `1612-1690` — out of bounds. The harvest function is reference-only; the rest is irrelevant.

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-host --test config_resolution_tdd`; return FACT pass/fail. On fail, return SNIPPETS of failing assertion + ≤ 20 lines." — purpose: validate Step 2 (resolver TDD).
- "Run `cargo test -p slicer-host --test region_mapping_resolved_config_tdd`; return FACT pass/fail. On fail, return SNIPPETS of failing assertion + ≤ 20 lines." — purpose: validate Step 4 (RegionMapping stamping).
- "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_user_top_shell_layers_propagates_through_binary cli_rejects_top_shell_layers_string`; return FACT pass/fail. On fail, return SNIPPETS." — purpose: validate Step 6 (binary E2E).
- "Run `cargo build --workspace` and `cargo clippy --workspace -- -D warnings`; return FACT pass/fail." — purpose: validate Step 7 (gate).
- "Find every caller of `commit_region_mapping_builtin`; return LOCATIONS." — purpose: confirm the two known call sites in `prepass.rs` are the only ones; flag any test-only callers.
- "Summarize `docs/04_host_scheduler.md` §`RegionMapIR Compilation`; ≤ 200-word SUMMARY, no code." — purpose: confirm the built-in's contract and that adding a per-object stamp is consistent.
- "Read the DEV-040 row in `docs/DEVIATION_LOG.md` and confirm its Status field; return FACT (one of Open/Closed/Partial + the Target Close cell)." — purpose: pre-flight at packet completion before flipping to Closed.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `RegionPlan.config: ResolvedConfig` — same shape; this packet changes how the value is sourced, not the shape.
  - `ResolvedConfig.extensions: HashMap<String, ConfigValue>` — used by the permissive fallback for unknown keys (already documented as the overflow bucket in `docs/02 §304-344`).
  - No `SemVer` bumps anywhere.
- WIT boundary considerations: none. WIT files unchanged; per-module `ConfigView` filtering unchanged.
- Determinism or scheduler constraints:
  - `BTreeMap<String, ResolvedConfig>` (sorted by `object_id`) keeps RegionMapping iteration deterministic.
  - The default-fallback `ResolvedConfig` is computed once at startup; `Arc`-shared across rayon workers; immutable.

## Locked Assumptions and Invariants

- `ObjectMesh.id` is the canonical `<object_id>` string used in all overlay keys; it is already populated by `model_loader::load_model` and never mutated.
- `parse_cli_config_source` continues to flatten any input that is not a top-level JSON object into `ConfigSourceParseError::NotAnObject`. The resolver receives only `HashMap<String, ConfigValue>` after that gate.
- `ResolvedConfig::default()` is the correct fallback when no user input is supplied (matches existing behavior).
- `RegionMapIR.entries` is keyed by `(global_layer_index, object_id, region_id)`; the stamp uses `object_id` from the key, which is guaranteed non-empty by `harvest_layer_plan_ir`'s canonical-id parsing (`dispatch.rs:1598-1610`).
- A user passing `top_shell_layers = 0` is honored (not coerced) — this is the documented "disable" path in `layer_slice.rs:303,319` and `packet 35` AC.

## Risks and Tradeoffs

- **`ActiveRegion.resolved_config` becomes effectively dead** for the live path. Documented as advisory in the IR doc comment so future contributors don't add producers expecting it to be authoritative. No removal in this packet — keeping the field avoids a schema bump and existing test fixtures that build `ActiveRegion` continue to compile.
- **Module-emitted config from a future custom layer planner is silently overwritten**. Acceptable because the host built-in is defined as the single authoritative resolver of per-region config (`docs/04 §RegionMapIR Compilation`). Documented in the new module's rustdoc.
- **CLI shape grows new key family `object_config:<id>:<key>`** without docs/03 schema declaration. Mitigated by the existing `object_height:*` host-injected precedent; the resolver is the only consumer, so module manifests don't need to declare these keys.
- **Strict typing on declared fields can break a user passing a quoted integer** (`"top_shell_layers": "4"`). Acceptable — caught by the new negative-case AC; CLI exits with a clear error.
- **Test surface area grows by 3 files**, but this packet adds ≤ 80 LOC of test per file and no test reads the workspace beyond the new resolver and RegionMapping path.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`.
- Largest single step: `M` (Step 2 — resolver TDD: ~5 unit tests + the resolver implementation, all in one file pair).
- Highest-risk dispatch (the one whose return could blow budget if mis-shaped): the binary E2E run in Step 6. Required return format: FACT pass/fail; on fail, SNIPPETS of the failing assertion plus ≤ 20 lines of stderr context. Sub-agent MUST NOT echo the produced G-code.

## Open Questions

- None blocking activation. The four scope decisions raised before file generation are locked: (1) slug `35a_resolved-config-propagation`; (2) hybrid strict/permissive resolver; (3) per-object overlay included; (4) resolver lives in `crates/slicer-host/src/config_resolution.rs`.
