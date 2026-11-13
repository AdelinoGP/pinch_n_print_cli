---
packet: proptest-openrscad_05_scad-ingest
status: implemented
---

# Opt-in SCAD ingestion for slice and mesh import

## Goal

Route self-contained SCAD through the shared strict adapter into one source-identified model for `slice` and `mesh import`, with explicit expression parameters/extrusion and proved Windows feature-off/on behavior.

## Why

The approved input is `docs/specs/proptest-openrscad-adoption.md` §§4, 8–9 and SCAD-INGEST. `load_model` (`crates/slicer-model-io/src/loader.rs`) currently handles STL/OBJ/3MF; `Cmd::Slice` uses `--model`, whereas `MeshCmd::Import` uses `--input` and delegates to the STEP importer. That importer automatically repairs by default and has multi-solid output semantics. Neither behavior is SCAD policy.

The adapter prerequisite owns evaluation, strict diagnostics and conversion. This packet owns only file ingestion, host assembly, frontend routing and placement. A successful adapter test alone cannot prove the actual CLI uses it, preserves the original identity or refrains from publishing output after evaluation failure.

## Scope

- In: targeted ADR-0017 amendment; default-off model-I/O/CLI `scad` features; explicit SCAD load entry; typed diagnostic/error preservation; repeatable expression parameters and explicit 2D extrusion; one-object assembly; only `Cmd::Slice` and `MeshCmd::Import` evaluation routes; bare-model slice placement; focused library/CLI success and failure tests; incremental Windows lane, dependency evidence and product documentation.
- Out: another evaluator/converter, new adapter warning classifications, runtime path-based loading, SCAD support in `support-preview`, `visual-debug`, DAG model probes or other mesh commands; external resources; repair/approximation, materials/colors, persistent caching, a printer/bed option for `mesh import`, full language parity, randomized full-pipeline campaigns, Proptest dependencies or property-runner changes. No IR/WIT/schema version change or new OrcaSlicer port.

## Depends on

- `docs/spec_packets/proptest-openrscad_03_scad-adapter/` (**draft at authoring**): consumes exactly its planned `slicer_scad::{ScadSource, ScadParameter, ScadEvalOptions, ScadGeometry, ScadDiagnostic, ScadProvenance, ScadEvalOutput, ScadError, ScadErrorKind, evaluate}` contract and bounded defaults. Implementation waits for that packet's implementation and adapter-prerequisite Windows acceptance, and rechecks its exports on disk. It does not wait for scripted-generator/full G-SCAD closure in #4 and does not claim that closure here.

## Design notes

### Grounded ownership and feature boundary

- `assemble_object(id, mesh, config, modifier_volumes, paint_data)` and private `path_object_id(path, index)` (`crates/slicer-model-io/src/loader.rs`) are the existing assembly/identity authorities. Use their **actual argument order**; the illustrative call in `docs/02_ir_schemas.md` § `ObjectMesh` Assembly Contract is stale and must be corrected when documenting this additional producer. Reuse private `compute_bounding_box_union` for `MeshIR.build_volume`. Identity transform and recomputed `world_z_extent` come from `assemble_object`, not a second wrapper.
- Add **new** model-I/O manifest features `default = []` and `scad = ["dep:slicer-scad", "slicer-scad/openrscad"]`, with an optional normal dependency on the prerequisite adapter. Add **new** `pnp-cli/scad = ["slicer-model-io/scad"]`; retain its existing `default = ["report"]` unchanged. Neither production feature implies SDK `property_testing` or Proptest. Do not add engine dependencies to runtime, SDK `test`, guests or default feature lists.
- ADR-0017 (`docs/adr/0017-file-format-io-boundary-separation.md`) currently makes `slicer-model-io` a leaf whose only first-party dependency is `slicer-ir`, so the optional adapter edge requires a **targeted amendment**, not an assumption of compliance. Amend its Decision and affected dependency/graph statements to permit only the optional, geometry-only `slicer-scad` normal dependency under `scad`; without that feature, `slicer-ir` remains the sole first-party normal dependency. Rationale: reuse the dedicated strict evaluation/conversion boundary rather than duplicate it or move file I/O into the runtime. Preserve the prohibition on runtime/SDK/Proptest dependencies in the model-I/O/adapter production graph, keep `assemble_object`/`MeshIR` assembly in `slicer-model-io`, and retain runtime consumption of preloaded `Arc<MeshIR>` with no model-I/O/SCAD dependency. This is the only leaf-rule exception; allocate no new ADR.
- Extend existing `ModelFormat` with **new** `Scad`, recognized case-insensitively by `detect_format` even in feature-off builds. Detection is not evaluation. Existing `load_model(&Path)` stays non-evaluating: for `Scad`, return `ScadSupportDisabled` without the feature and `ScadOptionsRequired` with it, before opening/parsing as a mesh. Thus other existing `load_model` callers do not acquire implicit script execution.
- No new ADR/deviation identifier is reserved. Host mesh ownership (`docs/01_system_architecture.md` § Data Ownership Rules), runtime's preloaded `SliceRunOptions.mesh` and existing model assembly ownership remain unchanged.

### New model-I/O handoff contract

All additions below are **new planned public API**, crate `slicer-model-io`. Define the loader entry/result in `crates/slicer-model-io/src/loader.rs` and re-export from `crates/slicer-model-io/src/lib.rs`. The entry/result and adapter-type re-exports are gated by `feature = "scad"`; the format variant and the first two errors below exist in both builds.

```rust
// Existing ModelFormat gains this unit variant:
// ModelFormat::Scad

// New variants of existing ModelLoadError, available in both feature states:
// ModelLoadError::ScadSupportDisabled
// ModelLoadError::ScadOptionsRequired

#[cfg(feature = "scad")]
pub use slicer_scad::{ScadEvalOptions, ScadParameter, ScadDiagnostic, ScadProvenance};

#[cfg(feature = "scad")]
pub struct ScadModelLoad {
    pub mesh: slicer_ir::MeshIR,
    pub diagnostics: Vec<slicer_scad::ScadDiagnostic>,
    pub provenance: slicer_scad::ScadProvenance,
}

#[cfg(feature = "scad")]
pub fn load_scad_model(path: &std::path::Path, options: &ScadEvalOptions)
    -> Result<ScadModelLoad, ModelLoadError>;

// Additional new ModelLoadError variants, gated by feature = "scad":
// ScadEvaluation(slicer_scad::ScadError)
// ScadNeedsExtrusion { source_label: String }
// ScadEmpty { source_label: String }
```

`ScadModelLoad` implements `Debug + Clone`, not an invented printable-model `Default`. Preserve adapter diagnostics/provenance unmodified. Extend `ModelLoadError::Display` with stable fragments `SCAD_SUPPORT_DISABLED` plus `pnp-cli/scad` / `slicer-model-io/scad` activation guidance, `SCAD_OPTIONS_REQUIRED` plus `load_scad_model`, `SCAD_NEEDS_EXTRUSION` plus `--scad-extrude-height`, `SCAD_EMPTY_MODEL`, and `SCAD_EVALUATION` plus the underlying `ScadErrorKind` name, source and parameter context. Extend its currently empty `std::error::Error` implementation to expose the nested `ScadError` through `source()`; retain existing variants and I/O conversions.

`load_scad_model` accepts only detected SCAD paths, reads UTF-8 text with a checked `max_source_bytes + 1` cap before materializing an unbounded file, labels the source with the original path and calls the real `slicer_scad::evaluate`. A zero/overflowing read capacity is `ScadEvaluation` with `ScadError.kind = InvalidOptions` before allocation; an otherwise over-limit input is a structured `ScadEvaluation` carrying `ComplexityLimit`. Invalid UTF-8/read errors retain `ModelLoadError::Io`. Adapter validation still checks aggregate source/override capacity and remaining invalid options; do not duplicate its evaluator/conversion implementation. Source labels are diagnostics, never resource search roots.

- `ScadGeometry::Mesh` is assembled as exactly one object with `path_object_id(path, 0)`, empty `ObjectConfig.data`, no modifier volumes and `paint_data = None`. Disconnected bodies stay in its single indexed mesh. Colors/groups produce no tools or materials. No intermediate STL/temp file or repair call.
- `Contours` becomes `ScadNeedsExtrusion`, never implicit thickness; `Empty` becomes `ScadEmpty`, also when an otherwise valid height was supplied. Adapter errors remain structured, including `MixedDimensions`, `ExtrusionHeightOn3d`, resource/partial/degraded/approximate/unknown and conversion errors. Do not reimplement classification/triangulation here.
- Return source-coordinate geometry. Loading and `mesh import` do **not** place it on a bed. Do not derive IDs from output paths, temporary names or parameter digests: the established identity is original basename plus object index. Different directories with the same basename give the same ID; this is not a promise of hash injectivity. Existing single-input command semantics remain; multi-input jobs would still need `check_basename_collisions`.

### CLI routing and parsing

In `crates/pnp-cli/src/main.rs`, add **new**, feature-gated `scad_param: Vec<String>` / `scad_extrude_height: Option<f64>` arguments only to `Cmd::Slice` and `MeshCmd::Import`, exposed as `--scad-param NAME=EXPR` and `--scad-extrude-height MM`. These are frontend input options, not `ResolvedConfig`, `ConfigKey`, `apply_cli_key`, JSON slicer config or module schema entries. Existing `slice --config PATH`, `slice --model PATH` and `mesh import --input PATH --output-format ...` conventions do not change.

- Split each parameter at its **first** `=`; reject missing separator or empty name/expression as CLI usage error `2` with `SCAD_INVALID_PARAMETER_ARGUMENT`. Preserve identifier case, `$` names, expression bytes and duplicate order in `Vec<ScadParameter>`. Do not parse values as JSON, shell code or slicer config. Adapter grammar/expression validation returns `InvalidParameter`, retaining the offending parameter. Later duplicates win, but an invalid overwritten expression still fails according to #3.
- Parse extrusion as f64 and reject nonfinite, zero or negative height at argument validation with exit `2`, fragment `SCAD_INVALID_EXTRUSION_HEIGHT`; allow negative tokens to reach that validator rather than a misleading unknown-flag failure. The library independently validates options. A valid height on 3D is adapter `ExtrusionHeightOn3d`, not ignored.
- In enabled builds reject either SCAD flag on non-SCAD input before import/slice, exit `2`, `SCAD_OPTIONS_REQUIRE_SCAD_INPUT`. Feature-off builds do not advertise/accept these flags; an existing `.scad` input **without** flags must still reach `SCAD_SUPPORT_DISABLED`, not STL/OBJ/STEP parse errors. `.SCAD` follows the same branch.
- Route SCAD in `Cmd::Slice` through `load_scad_model`, then its actual `place_model_on_bed`, then the existing `Arc<MeshIR>`/`run_slice` path. Ordinary formats keep `load_model`. Preserve model-authored layer-range handling for 3MF only. Do not change `object_ids_from_model` or other consumers to call the evaluator.
- Route SCAD in `MeshCmd::Import` to a separate SCAD branch in `crates/pnp-cli/src/helpers_cmd.rs`, before `run_import`'s STEP path; reuse private `write_mesh` and existing `OutputFormat::{Stl,Obj,ThreeMf}`. STEP routing remains `run_import`/`import_step_with_options`. Reject explicit `--merge-components` or `--no-repair` on SCAD with exit `2`, `SCAD_STEP_OPTION_CONFLICT`, naming the flag. SCAD already has one fused object and never repairs. Do not rename these existing STEP flags or add `--config`/bed options to import.

| SCAD outcome | `slice` exit | `mesh import` exit |
|---|---|---|
| Complete validated output, including recognized information | `0` | `0` |
| CLI option/format conflict | `2` | `2` |
| Missing/unreadable source or output write failure | `1` | existing `UNREADABLE = 2` |
| Empty evaluated geometry | `1` | existing `EMPTY_OR_TRIVIAL = 3` |
| Disabled support, 2D without height, or any adapter rejection | `1` | existing `PARSE_ERROR = 4` |

Do not reuse STEP's `WARNINGS_OR_PARTIAL = 1` for accepted SCAD information or partial SCAD geometry. Partial output is always a failure. Preserve the ordinary `error:` stderr convention and typed source error internally. Display every accepted informational diagnostic even without `--stats`; never contaminate stdout G-code or mesh bytes. With `mesh import --stats`, reuse `start`/`done` and add JSONL `event = "diagnostic"`, `operation = "import"`, `kind = "scad_information"`, with `code`, `message`, `source_label` and `stage`; emit it once per accepted diagnostic instead of duplicating a plain message. The SCAD `done` row has `source_unit = "mm"`, `mesh_count = 1`, `total_triangles` and `warnings = 0`. Failures emit no `done` or successful `slice_complete`; do not create/truncate the output until evaluation and assembly succeed. Test both absent and pre-existing destination files. This is not a new transactional guarantee for later disk-write failures.

### Slice placement and units

Extend `place_model_on_bed`'s bare-format match to include `ModelFormat::Scad` **only after explicit SCAD loading**. It continues to use `read_bed_shape`, `bed_center_mm`, `place_bare_mesh_on_bed` and `warn_if_off_bed`: one whole-model translation centers the XY AABB on the bed and drops world Z minimum to zero, preserving disconnected-body relative positions. Leave 3MF authored placement untouched, and preserve existing unusable-bed warning and overflow-warning behavior. This reuses the established placement policy, not a new canonical port.

<!-- snippet: coord-system -->
- Units: 1 unit = 100 nm (10⁻⁴ mm), not OrcaSlicer's 1 nm; divide OrcaSlicer constants by 100 and convert with `Point2::from_mm` / `mm_to_units()` (`docs/08_coordinate_system.md`).

Engine f64/contour quantization/mesh f32 validation remains #3's responsibility. Frontend placement uses existing mesh millimeters. Derive translation/serialization tolerances for the bounded fixtures from their actual f32 operations and writer precision; do not compare incidental tessellation/vertex order or use an unexplained epsilon.

### Executable acceptance surface and Windows evidence

All following targets are **new standalone targets**, explicitly declared in their owning manifests. `scad_ingest_feature_contract` (model-I/O) and `scad_cli_feature_contract` (CLI) are **ungated**, with substantive tests in either feature state. `scad_model_ingest` (model-I/O) and `scad_cli_ingest` (CLI) require `scad`. No feature-off run of an empty `cfg` binary counts as proof. CLI tests use Cargo's built `pnp_cli` binary through existing `assert_cmd` conventions, not a hardcoded possibly feature-mismatched `target/debug` executable. Private placement tests are registered in a new inline `#[cfg(all(test, feature = "scad"))]` module in `main.rs` and run via `--bin pnp_cli`.

Use small self-contained script text fixtures created here, including an analytically sized Boolean solid, a rectangle-with-hole profile and two separated translated boxes; do not rely on an uncommitted user model. Tests requiring partial/degraded/approximate outcomes consume #3's pin-verified reproducer recipes, not copied evaluator code. Inject unreachable converter/classifier outcomes only in #3's tests; product tests exercise actual public rejection inputs and assert preservation of the error. Dangerous fixtures use the prerequisite's supervised-child pattern, with source/options saved before execution and kill/join on deadline. No in-process render timeout is promised.

Concrete **new test fixture inputs**, embedded in the new targets or checked-in small `.scad` text fixtures:

- Boolean through-hole solid: `difference() { cube([12,10,6]); translate([4,3,-1]) cube([4,4,8]); }`; expected bounds `[0,12] × [0,10] × [0,6]` mm and volume `(12*10 - 4*4)*6` mm³.
- Holed 2D source: `difference() { square([12,10]); translate([4,3]) square([4,4]); }`; explicit height `2` gives Z `[0,2]` mm and volume `(12*10 - 4*4)*2` mm³, with the hole retained.
- Source-coordinate placement witness: `translate([-30,5,-4]) cube([4,6,8]); translate([-20,7,-2]) cube([2,2,4]);`; expect one object with two separated bodies, unplaced bounds `[-30,-18] × [5,11] × [-4,4]` mm. Compute the one shared placement translation from that AABB and the selected bed center, not from the observed placed output.
- Parameter witness: `Size=[6,8,2]; cube(Size);`, with ordered overrides `Size=[5*2,12,3]` followed by `Size=[8,6,4]`; malformed/overwritten expressions still follow #3's policy. Config separation uses `layer_height=3; cube([6,6,layer_height]);` plus a `layer_height` SCAD override and a distinct positive JSON slicer layer height.
- Informational source: `echo("scad information"); cube([2,2,2]);`, using #3's recognized `scad_echo` classification.

Add **new** `jobs.scad-ingest-windows` to `.github/workflows/ci.yml`, `windows-latest`, preserving existing jobs. Reuse #3's supported Rust/CMake/MSVC setup and wasm prerequisites for xtask freshness. Run feature-off library/CLI and feature-on library/import/slice gates, not just compile checks. Product acceptance does not require #1's property job. Save scripts/options/stderr and logs under **new** `target/scad-ingest/`, uploading on failure. Archive the canonical test log after each run.

Create **new** `docs/scad/ingestion-evidence.md`: actual source revision/dirty-content provenance, Windows toolchain/native prerequisites, selected engine/backend, `CARGO_TARGET_DIR`, features/profiles, commands/statuses, discovered/executed named tests, semantic output observations and measured build versus execution costs. Retain normal/build Cargo graphs for default and SCAD-enabled `pnp-cli` and model-I/O, proving enabled production has no Proptest and default graphs have no OpenRSCAD/native backend. Do not confuse workspace dev-feature unification with production edges. Authorship is not Windows feasibility or product acceptance.

## Acceptance criteria

Names below are **new planned tests**. `MF`, `ME`, `CF`, `CE`, `B` and log mechanics are defined in Verification.

- **AC-1** `scad_model_feature_routing` under `MF` detects `.scad`/`.SCAD` as `Scad` and observes feature-off `ScadSupportDisabled` or feature-on `ScadOptionsRequired` through `load_model`, with no parsing/evaluation. `scad_cli_feature_routing` under `CF` executes both commands: disabled inputs report `SCAD_SUPPORT_DISABLED` with exits `1`/`4`, and enabled help exposes only the two intended commands' new flags. ADR-0017 contains the narrowly scoped optional `slicer-scad` exception and retained ownership/prohibitions specified above; normal/build graph evidence proves that exact boundary and the declared isolation.
- **AC-2** `scad_model_one_object_source_identity` under `ME` loads the Boolean model and disconnected boxes: exactly one `MeshIR.objects` entry, UUID-v5 basename-plus-`#0` identity independently calculated with `uuid::Uuid::new_v5`, identity transform, empty config/modifiers, no paint, valid flat indices and dimension-derived AABB/volume. Repeated loads and same basename in different directories retain ID/geometry; changing export paths has no effect. `world_z_extent` and `build_volume` match independently worked geometry. No output-file reload is mistaken for preserving an STL-carried identity.
- **AC-3** `scad_cli_parameters_and_config_separation` under `CE` drives repeatable arithmetic/nested-vector/duplicate overrides, first-`=` splitting, case-sensitive names and `$preview` source/override behavior through real import and slice. Bounds/volume or printed dimensions reflect the last expression; exact Boolean semantics persist. A parameter named `layer_height` changes only script geometry; an independent `--config` layer height still controls slicing and no SCAD flags/names enter emitted slicer config.
- **AC-4** `scad_model_explicit_holed_extrusion` under `ME` and `scad_cli_holed_extrusion` under `CE` import a rectangle-minus-inner-rectangle with positive height: dimension-derived Z extent and `(outer_area - hole_area) * height` volume, preserved through STL/OBJ/3MF exports, with no filled-in hole or automatic repair. Test reopened output geometry semantically, not triangle order.
- **AC-5** `scad_placement_centers_and_drops_to_bed`, `scad_placement_preserves_disconnected_offsets`, `scad_placement_uses_configured_bed_centre`, `scad_placement_leaves_3mf_untouched` and `scad_placement_skips_when_bed_shape_is_unusable` under `B` and `scad_cli_slice_places_and_emits` under `CE` drive the production placement helper and real `slice`: translated-below-zero separated bodies receive the same translation, preserve relative offsets, center on configured/default bed and have Z minimum zero. Real G-code has nonempty extrusion moves at the expected placed XY/layer envelope and `slice_complete.status = "ok"`; 3MF remains unchanged. `scad_cli_import_preserves_source_coordinates` under `CE` exports unplaced coordinates, with no bed-config argument.
- **AC-6** `scad_cli_information_and_stats` under `CE` preserves recognized information on stderr, returns `0`, leaves stdout G-code clean, and emits the stated import diagnostic/done fields exactly once with `--stats`. Existing STEP import exits/stats and ordinary CLI tests still pass. Windows evidence records actual named executions in both feature states and the required `scad-ingest-windows` job.
- **AC-N1** `scad_cli_rejects_option_misuse` under `CE` checks missing `=`, empty name/expression, zero/negative/NaN/infinite height, SCAD flags on STL/STEP, and either STEP-only flag on SCAD: exit `2` and the specified fragment/flag. Feature-off help/unknown-option behavior is checked by `scad_cli_feature_routing`, not a silently skipped enabled test.
- **AC-N2** `scad_model_rejects_dimensions_and_empty` under `ME` and `scad_cli_failures_do_not_publish` under `CE` cover missing extrusion (`ScadNeedsExtrusion`), valid height on 3D (`ScadEvaluation` carrying `ScadError.kind = ExtrusionHeightOn3d`), mixed dimensions (`MixedDimensions`), empty geometry (`ScadEmpty`), parse errors and invalid expressions (`InvalidParameter` with name). Exit mappings follow the table. Fresh output is absent, pre-existing output bytes remain unchanged, stdout has no G-code/mesh and no success event is emitted after ingestion failure.
- **AC-N3** `scad_model_preserves_strict_adapter_errors` under `ME` and `scad_cli_strict_errors_and_resources` under `CE` execute external-resource requests with a valid sibling primitive, the pin's public-path partial/approximate/degraded reproducers, the **default** source-byte bound, out-of-range coordinates and post-quantization conversion damage (the last three are reachable through public input: a >1 MiB script, `translate([10001,0,0]) cube(1);`, and the `union()` gap that the 1e-4 mm grid closes). They preserve the prerequisite's actual typed kind/source/parameter/diagnostics through `ScadEvaluation` and stderr, with no sibling/partial output accepted. For the single kind #3 proves only through classifier/converter injection because the pin has no public reproducer (`non_finite_coordinate`; probed forms evaluate to empty geometry instead), test the real `ModelLoadError` wrapping/`source()`/formatting with a constructed `ScadError`, label it error-envelope evidence rather than end-to-end rejection, and cite #3's executed rejection test. Do not create a second frontend injection/classifier seam or claim an unexecuted engine outcome. Unknown-diagnostic classification remains proved in #3; no new product allowlist bypass exists.

## Verification

Authoring verified docs/source and locked/offline Cargo metadata only; the following are implementation acceptance obligations. Every plain `cargo test` below uses Bash `set -o pipefail`, ensures `target/` exists, and appends `2>&1 | tee target/test-output.log`; read/archive the file before its next overwrite. PowerShell must preserve `$LASTEXITCODE` across `Tee-Object`. `cargo xtask test` already captures its combined output. Explicitly verify named discovery/execution, not a fixed binary count or successful zero-test process.

- `MF`: `cargo test -p slicer-model-io --no-default-features --test scad_ingest_feature_contract -- --nocapture`; repeat with `--features scad`.
- `ME`: `cargo test -p slicer-model-io --features scad --test scad_model_ingest -- --nocapture`.
- `CF`: `cargo test -p pnp-cli --no-default-features --test scad_cli_feature_contract -- --nocapture`; repeat with `--features scad`. Also check ordinary default/report builds, not only no-default configurations.
- `CE`: `cargo xtask test --summary -p pnp-cli --features pnp-cli/scad --test scad_cli_ingest -- --nocapture`. This guest-using slice gate must use the existing gated runner; do not replace it with bare Cargo. Use explicit `--no-default-module-paths --no-integrated-modules --module-dir` pointing at built core guests in the test fixture, as the existing progress-event tests demonstrate. Fail loudly for missing modules/artifacts.
- `B`: `cargo test -p pnp-cli --features scad --bin pnp_cli -- scad_placement_ --nocapture` with the capture wrapper; the test name starts with this filter and reaches `place_model_on_bed` (`crates/pnp-cli/src/main.rs`).
- Existing baseline/regressions: `cargo test -p slicer-model-io --test model_loader_tdd`; separately `cargo test -p slicer-model-io --test bed_placement_tdd`; `cargo test -p pnp-cli --test helpers_cli`; separately `cargo test -p pnp-cli --test cli_tdd`, each captured. Existing guest-using progress acceptance: `cargo xtask test --summary -p pnp-cli --test slice_progress_events_default_tdd`. Record baseline findings before edits and rerun affected tests after them.
- `cargo build --workspace`; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo build -p pnp-cli --features scad`; `cargo check -p pnp-cli --all-targets --features scad`; `cargo clippy -p pnp-cli --all-targets --features scad -- -D warnings`; `cargo check -p slicer-model-io --all-targets --features scad`; `cargo clippy -p slicer-model-io --all-targets --features scad -- -D warnings`.
- Retain `cargo tree -p pnp-cli -e normal,build,features` and the same with `--features scad`, plus model-I/O equivalents. `cargo xtask check-literals`; `cargo xtask check-test-quality --report`. Before interpreting guest/module-dispatch failures run `cargo xtask build-guests --check`, judge its exit code, rebuild on `1`, stop on `3`; if staged in-tree copies fail mtime freshness despite `0`, follow the documented force-rebuild remedy. No workspace suite is required here.

## Docs impact

- `docs/adr/0017-file-format-io-boundary-separation.md` § Decision and affected dependency/graph statements: targeted amendment permitting only optional geometry-only `slicer-scad` under `scad`, retaining the default leaf rule, runtime/SDK/Proptest prohibition and host assembly ownership; no new ADR. Check: `rg -n 'slicer-scad|scad|slicer-sdk|Proptest|assemble_object|Arc<MeshIR>' docs/adr/0017-file-format-io-boundary-separation.md`, followed by review of the complete Decision for contradictory unconditional leaf/graph claims.
- **New** `docs/scad/ingestion.md`: features/build prerequisites, exact flags/exit table, expressions/special variables, strict errors, self-contained restriction, one-object semantics, slice placement versus unplaced import and no render-deadline promise. Check: `rg -n 'scad-param|scad-extrude-height|SCAD_SUPPORT_DISABLED|SCAD_STEP_OPTION_CONFLICT' docs/scad/ingestion.md`.
- **New** `docs/scad/ingestion-evidence.md`: actual Windows executions, provenance, output semantics, separate build/execution measurements and dependency isolation. Check: `rg -n 'Windows|source_revision|scad_cli_ingest|normal.*build' docs/scad/ingestion-evidence.md`.
- `docs/08_coordinate_system.md` § Transform Application and § Bed Placement: extend bare-model applicability to explicitly loaded SCAD; retain 3MF exclusion and slice-only placement. Check: `rg -n 'SCAD|scad|3MF' docs/08_coordinate_system.md`.
- `docs/02_ir_schemas.md` § `ObjectMesh` Assembly Contract: add explicit SCAD producer and correct the illustrative argument order to match `assemble_object`; describe no paint/material assignment. Check: `rg -n 'load_scad_model|assemble_object|SCAD' docs/02_ir_schemas.md`.
- `docs/13_slicer_helpers_crate.md` § CLI Subcommand: `pnp_cli mesh import`: link optional SCAD frontend branch/exit behavior, explain STEP-only flags; do not imply `slicer-helpers` owns the engine or change STEP's API/exit table. Check: `rg -n 'SCAD|scad/ingestion|STEP-only' docs/13_slicer_helpers_crate.md`.
- `README.md` run/example section: link opt-in SCAD input/build instructions, preserving `slice --model`. Check: `rg -n 'scad/ingestion|--model' README.md`. Extend the prerequisite's planned `docs/scad/adapter.md` with the explicit ingestion consumer link; do not rewrite its contract.

## Open questions

- [FWD] Measure bounded Windows import/slice fixtures and build/execution costs after #3 supplies its real pin/defaults; derive placement/writer tolerances and supervised-child limits before accepting the lane. Performance impact is unmeasured at authoring. Any missing strict adapter outcome is blocking prerequisite fix work, not permission to accept degraded geometry.

## Authoring preflight

| # | Check | Result | Evidence / applicability |
|---|---|---|---|
| 1 | Symbols | PASS | Actual loader/assembly/identity/bounds and main/helpers routing/writer symbols read; stale doc argument order explicitly corrected; all new types/flags/errors/targets marked planned. |
| 2 | WIT/IR identifiers | PASS | Actual `MeshIR.objects/build_volume`, `ObjectMesh` assembly and flat mesh fields used; adapter enum shapes match #3; no WIT change. |
| 3 | Schema versions | PASS | No schema/WIT/version bump or pinned future literal. |
| 4 | ADRs | PASS | ADR-0017 read in full; its sole-first-party-dependency restriction requires the explicit targeted amendment now included in scope, design, AC-1, Docs impact and implementation Step 1. Only optional geometry-only SCAD is excepted; prohibitions/host assembly retained; no new ADR slot. |
| 5 | Deviation IDs | PASS | No ID allocated or consumed. |
| 6 | Prerequisites | PASS | #3 explicitly draft; its exact public shape and adapter-prerequisite/full-gate ownership checked; implementation must wait. |
| 7 | Test targets | PASS | Existing targets/features verified with Cargo metadata; new standalone targets and feature requirements explicit, plus registered binary unit module and gated guest CLI run. |
| 8 | Criteria | PASS | Exact flags, variants, stderr/event/exit/output semantics and negative no-publish paths; no workspace criterion. |
| 9 | Fallout | PASS | Enum exhaustive matches/Display/source/CLI destructuring and new literal sites listed in plan; no production struct/schema field addition. |
| 10 | Docs impact | PASS | Actual canonical headings and new/prerequisite docs identified with checks, including ADR-0017's Decision/dependency-graph amendment and contradiction review. |
| 11 | Completeness | PASS | No template placeholders or scope-changing blockers; measured limits/tolerances have bounded implementation ownership. |

**Verdict: PREFLIGHT PASS (rechecked after ADR-0017 finding; targeted amendment is an explicit implementation obligation; authoring only; Windows/product execution unperformed; implementation waits for #3).**

## Deviations

- **`place_model_on_bed` (`crates/pnp-cli/src/main.rs`) refreshes the cached `world_z_extent` after translating.** The packet asked only to extend the bare-format match. Translation happens after `assemble_object` cached the extent, so without a refresh the model's own cached world-space Z range keeps describing the pre-placement geometry (a model loaded at Z = -4 still reports `(-4, 4)` while its vertices rest on the plate). That makes AC-5's "Z minimum zero" unassertable on the model's stated extent and leaves the model internally inconsistent. The refresh is behaviour-preserving for the only production consumer of the value (`run_slice`'s object-height query uses `z_max - z_min`, a translation invariant), and `bed_placement_tdd` (8) plus `world_z_canonical_surface_tdd` (6) stay green. 3MF returns before this code, so authored placement is untouched.
- **Feature-off `.scad` import prefers `SCAD_SUPPORT_DISABLED` over `SCAD_STEP_OPTION_CONFLICT`.** When a feature-off build is given `.scad` *and* `--merge-components`/`--no-repair`, both errors are applicable. The disabled-support diagnostic is the actionable one (it tells the user how to enable the feature), and the STEP-only flags cannot change the outcome. Enabled builds still report `SCAD_STEP_OPTION_CONFLICT` before any evaluation.
- **`SCAD_STEP_OPTION_CONFLICT` is returned through a new `exit_codes::USAGE` constant** (value `2`, matching clap's own usage status). The numeric exit code is unchanged from the packet's table; the constant exists so the code does not read as "unreadable input" when the invocation is what was wrong. It is `#[cfg(feature = "scad")]`-gated because only the SCAD branch uses it.
- **An override of a name the source never declares leaves that name `undef`, which evaluates to empty geometry** (pin behaviour, verified through the built binary: `cube([2,2,e]);` plus `--scad-param e=4` is `SCAD_EMPTY_MODEL`, while `e=1; cube([2,2,e]);` plus the same override renders). AC-3's first-`=` fixture therefore declares `e=1` in the source, so the assertion tests the split, not undefined-variable rendering.
- **`scad_placement_tests` is an inline `#[cfg(all(test, feature = "scad"))]` module in `crates/pnp-cli/src/main.rs`**, run through `--bin pnp_cli`, as the packet specified. It calls the production `place_model_on_bed` (`crates/pnp-cli/src/main.rs`), so it is not a re-implementation of placement. AC-5 originally named one combined test (`scad_placement_rigid_and_3mf_unchanged`); it is implemented as five tests because rigid-body offsets, the configured bed, the 3MF pass-through and the unusable-bed path are independently falsifiable, and a single combined test would hide which invariant broke. The AC now names all five.
- **`scad_cli_rejects_option_misuse` lives in the `CE` target, not `CF`.** The two flags only exist in enabled builds, so the enabled half of the option-misuse matrix cannot live in the ungated feature-contract target; the `CF` target keeps the feature-off half (`scad_cli_feature_routing` asserts the flags are absent from help and rejected as unknown arguments), which is what AC-N1 assigns to `CF`.
- **Dangerous fixtures are contained and recorded.** Renderer-stress model-I/O fixtures (partial, degraded, approximate, post-quantization) run in a supervised, killable child that writes the source and the exact options under `target/scad-ingest/reproducers/` before evaluating and reports its typed outcome on stdout. This is required because `evaluate` joins a worker thread and a native render cannot be interrupted in-process (`crates/slicer-scad/tests/scad_process_limits.rs`, `docs/scad/adapter.md`); the earlier claim that the adapter's worker contained them was wrong. Evaluator-only inputs (parse, invalid options, empty, contour/3D, range, byte bound) stay in-process under the adapter's step budget; the strict-error test's `resource`, `oversize`, `oversize_default` and `coordinate_range` cases also retain source and per-case option records. Every `pnp_cli` child in `crates/pnp-cli/tests/scad_cli_ingest.rs` runs through `run_supervised`, which drains both pipes concurrently, polls `try_wait`, and on a 180 s deadline kills and reaps the child, persisting its streams before failing. Per-invocation argument records and captured stdout/stderr/exit are saved under `target/scad-ingest/scad_cli_ingest/` for the strict-error cases; the option-misuse matrix retains its two fixture sources and recorded arguments but not per-invocation streams, because each invocation is a literal flag tuple in the committed test; the remaining fixtures are likewise literal strings in the committed test file and are reproducible from it.
- **The `scad-ingest-windows` CI job has not executed in this run.** The same commands were executed on Windows on the developer machine and are recorded in `docs/scad/ingestion-evidence.md`; the GitHub job's own execution is the one obligation that requires the first push. Everything else in the packet was executed locally.
