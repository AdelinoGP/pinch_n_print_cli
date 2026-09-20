# Requirements: resolved-config-view

## Packet Metadata

- Grouped task IDs: `TASK-567`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`ConfigView` has two sources today, and neither is complete.

- The load-time binding comes from `bind_module_config_view(module, source)` (`crates/slicer-scheduler/src/execution_plan.rs`) over `expanded_global_source` (`crates/slicer-runtime/src/run.rs`). That map mixes the raw authored source, including unrecognized keys, with a `to_config_map()` overlay.
- Per-region views come from `ConfigView::from_declared` over the region `ResolvedConfig`'s `to_config_map()` (`crates/slicer-wasm-host/src/dispatch.rs`).
- Neither source seeds registry defaults. `resolve_scope_stack` starts from `ResolvedConfig::default()` with empty `extensions`, so an unauthored module-declared key is absent from every view. That is why 87 literal fallbacks across 13 guests carry behavior.

The remaining problems:

1. `ResolvedConfig::to_config_map` hand-lists its keys (RC-4).
2. `apply_delta` (`crates/slicer-config/src/resolution.rs`) writes `extensions` with no registry bounds check.
3. `CONFIG_BLOCK` overlays the raw source (`run_postpass_with_thumbnail`, `crates/slicer-runtime/src/pipeline.rs`) and emits strings unescaped.
4. Several keys are read by the host but declared nowhere: `gcode_flavor`, `printer_model`, `filament_colour`, `extruder_colour`, `filament_cost`, `printable_area`, `support_type`, `support_family`, `thumbnails`, `machine_max_acceleration_retracting`, `extruder`. They survive only because ingestion is warn-and-keep and the raw overlay re-injects them.
5. Unknown keys are still retained after warning, although the approved staging permits dropping them once a no-drop oracle proves no declared value is lost.

## In Scope

- **Reconciliation.** Reconcile packet 05's landed `resolve_scope_stack`, `query_z_grid`, `ResolvedObjectLayerConfig`, `ResolutionError`, and WIT 2.0.0 exports before edits.
- **Default seeding.** Seed the typed registry default of every seed-set key into the lowest-precedence layer of `resolve_scope_stack`, before Phase-B expansion. A seed-set key is exact, has a default, is not `selector`, and is not a `declare_resolved_config!` field (see `design.md`).
- **Generated host map.** Generate `ResolvedConfig::to_config_map` and `ResolvedConfig::typed_field_keys()` from the `declare_resolved_config!` rows, with no hand list and no omissions. Omission is expressed only by `config_block`.
- **Extension validation.** Validate every `extensions` write (seeded or authored) against registry type and `min`/`max`. Fail atomically through `ResolutionError::Application(ConfigResolutionError::{TypeMismatch, OutOfRange})`.
- **Host-key registration.** Make `HostRuntimeKey.default` an `Option<&'static str>`. Register every host-consumed key that has no registry entry as a `HOST_RUNTIME_KEYS` row. This includes the keys listed in the docs/02 viewer-key contract, `thumbnails`, `machine_max_acceleration_retracting`, and `extruder`. `extruder` gets no default, so seeding never injects it. Keys the serializer synthesizes when absent get no default.
- **Resolved binding.** Change `bind_module_config_view` and `build_live_execution_plan` to take the effective `ResolvedConfig` instead of a `HashMap<ConfigKey, ConfigValue>` source. Preserve module declaration encapsulation and the `support_type` / `support_family` injection for `support-family:` claimants.
- **`config_block` metadata.** Add polarity-safe metadata as `omit_from_config_block` on `HostKeyMeta`, `ConfigFieldEntry`, and `RegistryEntry`, parsed from manifest `config_block = false`. Any false declaration excludes the key. Mark exactly `mmu_segmented_region_max_width`, `mmu_segmented_region_interlocking_depth`, `mmu_segmented_region_interlocking_beam`, and `thumbnail_path`.
- **CONFIG_BLOCK projection.**
  - Add `ConfigSchemaRegistry::config_block_map`, which covers registry entries not omitted plus typed fields with no registry entry, and drops retained unknown keys.
  - `run_slice_with_collector` computes the map and passes it on the crate-private path as an added `config_block` input; `run_postpass_with_thumbnail` then uses it verbatim, with no raw overlay.
  - `raw_config_source` keeps its thumbnail and prepass roles.
  - The public `run_pipeline_with_*` entry points keep their signatures and today's composition.
- **Serializer.** Escape `CONFIG_BLOCK` string values per canonical `escape_string_cstyle`, and render `filament_diameter` per filament from the effective value.
- **Required accessors.** Add `ConfigView::require_{bool,int,float,string,abs_value}` returning `Result<_, ConfigReadError>`, with `From<ConfigReadError> for ModuleError` (fatal).
- **Guest declarations.** Declare, in the reading guests' manifests, the eleven verified undeclared reads plus any Step 1 adds:
  - overhang-classifier-default: `outer_wall_line_width`, `line_width`;
  - tree-support and traditional-support: `nozzle_diameter`, `layer_height`, `support_line_width`;
  - infill-linker: `infill_density`;
  - rectilinear-infill: `infill_shift_step`;
  - wave-overhangs: `thick_bridges`.
- **Fallback removal.** Delete the 87 classified config-literal fallbacks, and replace them with `require_*` reads matching each key's registry type.
- **No-drop oracle.** Add `SliceOutcome.ingestion_warnings`. Add an independently generated, registry-derived one-value-per-key config and combine it with `resources/cube_4color.3mf` in a no-skip `run_slice` e2e with a withheld-declaration negative control.
- **Warn-to-drop.** Flip packet 03's warn-and-keep to warn-and-drop only after that e2e is green and every host-consumed key is registered.
- **Step-6 pre-existing blockers.** Fix, in-packet: the authored-only `nozzle_diameter` lookup in `seed_expansion_context` (`crates/slicer-runtime/src/run.rs`) via a registry-default fallback; the `seam-planner-default` aligned-mode coordinate gap (guest emits planner/inset-boundary coordinates; host escalation untouched); and the two pre-existing stock reds, the Packet-68 stamping red and the NegativeSpacing red, whose exact files the Step 6a-bis diagnosis names.
- **Docs.** Update the resolved-view, viewer-key, manifest-metadata, and scheduler view-sourcing docs.

## Out of Scope

- New config aliases or key renames. A non-canonical key a guest already reads, such as infill-linker's `infill_density`, is declared as-is.
- `denied_scopes` authoring/enforcement and inert manifest-table deletion (packet 07).
- Typed modifier kind, layer-range ingestion/application, and Phase-C speed expansion (packets 08–10).
- Changing `ResolvedConfig.extensions`' storage type, interner identity, or hash semantics.
- Registering `infill_type`. It is a typed `plain` field and keeps emitting through `typed_field_keys()`.
- Migrating host reads of the authored `config_source` map in `run_slice_with_collector` (e.g. flavor selection). Those reads are unaffected by the delta drop, and Step 1 records them.
- Guest literal defaults that are not `.unwrap_or(<literal>)` chains, such as match-arm defaults, helper default arguments like `cfg_float(config, key, 0.4)`, and `slicer_sdk::config_resolution::resolve_float` fallbacks. The acceptance ceremony records them as residual.
- Changing the `ORCA_CONFIG_PADDING` table or its 96-key cap.
- Any WIT or public IR schema-version bump.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — RC-4, RC-5, the "Drop-unknown ships in two steps" paragraph, "Guests and delivery", the no-drop gate, and queue row 6. RC-5 keeps the original 54-site count with a note. Queue row 6 and "Guests and delivery" carry the 87-site chain-aware census this packet uses.
- `docs/adr/0067-unified-config-schema-registry.md` and `docs/adr/0068-config-scope-is-a-wire-encoding.md` — registry reconciliation/provenance and the typed-ingestion boundary.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` — the resolved config, CONFIG_BLOCK viewer-key, manifest, and view-sourcing contracts.
- `docs/22_test_quality.md` §2.1, §2.4, §2.6, §4 — independent oracle, no hand roster, no silent fixture skip.

## Acceptance Summary

| Criteria | Covers |
| --- | --- |
| `AC-1` | Binding |
| `AC-2`, `AC-8` | Typed, seeded extensions |
| `AC-3` | Calibrated census plus the declared-read check |
| `AC-4` | `config_block_map` population |
| `AC-9` | Escaping |
| `AC-11` | Production CONFIG_BLOCK wiring through `run_slice` |
| `AC-5` | No-drop oracle with a negative control |
| `AC-6`, `AC-10` | Warn-to-drop through ingestion, binding, and emission |
| `AC-12` | Registered host-consumed keys survive the drop |
| `AC-13` | Generated `to_config_map` renders every legacy key exactly as the old hand map did |
| `AC-7` | Docs |
| `AC-N1` (negative) | Atomic bounds rejection |
| `AC-N2` (negative) | Module encapsulation and the `require_*` error |

Cross-packet impact: packets 08–10 may rely on guest views containing every declared key that has a default or an authored value, with no local literals.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --test resolved_config_view_tdd 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test result: ok\. [1-9][0-9]* passed" target/test-output.log'` | Seeding, typed extensions, bounds, `config_block_map`, unknown-drop, host-key survival | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test contract resolved_config_view_tdd:: 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test result: ok\. [1-9][0-9]* passed" target/test-output.log'` | Binding, census, encapsulation, drop projection | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test integration gcode_ 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test result: ok\. [1-9][0-9]* passed" target/test-output.log'` | Escaping plus pre-existing CONFIG_BLOCK and flavor tests | FACT pass/fail |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e resolved_config_view_no_drop_tdd:: 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test result: ok\. [1-9][0-9]* passed" target/test-output.log'` | Production wiring, real no-drop slice, negative control | FACT pass/fail |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e slice_end_to_end_tdd:: 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test result: ok\. [1-9][0-9]* passed" target/test-output.log'` | Re-derived CONFIG_BLOCK structural canary | FACT pass/fail |
| `cargo test -p slicer-config --test registry_census_tdd` | Real-registry assembly and runtime-channel pin | FACT pass/fail |
| `cargo xtask build-guests --check` | Artifact-verified guest freshness | FACT exit code and stale names only |
| `cargo check --workspace --all-targets` | All-target type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Touched-test quality | FACT touched findings only |

The `cargo test` commands deliberately omit `--all-targets`, because cargo lets it override `--test <bin>`. This was verified on 2026-09-18 with a probe on the `xtask` package: `--all-targets --test zz_no_such_target_probe` compiled instead of erroring. With the override, every package target would be built and run, and a `test result: ok` grep would match zero-test binaries. `--all-targets` stays mandatory for the `check` and `clippy` gates.

## Step Completion Expectations

- Land metadata, host-key registration, seeding, and validation before any consumer changes.
- Then land emission, then binding, then guest cleanup. Each guest batch must compile against a complete view.
- The no-drop oracle must pass in retained/warn mode before the same step flips unknown entries to drop.
- The census test keeps the baseline 87 count, the per-guest table, and the calibration snippets, so a weakened detector cannot self-certify zero.
- When the canonical-correct `CONFIG_BLOCK` change alters a fixture or test expectation, rebaseline it by re-deriving the value from the registry, never by pasting a captured literal.

## Context Discipline Notes

Do not load `resources/cube_4color.3mf`; delegate archive inspection. Guest edits are grouped into bounded batches, and cargo work must return only pass/fail with bounded failure snippets.
