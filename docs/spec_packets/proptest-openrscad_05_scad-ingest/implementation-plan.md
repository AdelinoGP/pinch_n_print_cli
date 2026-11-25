# Implementation Plan: scad-ingest

Implement only after #3 is implemented and its exact adapter exports and Windows adapter-prerequisite evidence are checked. Do not reopen engine feasibility by building another adapter. Steps leave the tree compiling in default and enabled configurations. Commands use the logging/capture mechanics and aliases in `packet.spec.md` § Verification; preserve/read/archive `target/test-output.log` for every Cargo test invocation.

## Step 1: Capture the baseline and add explicit feature-aware model ingestion

- Objective: establish production dependency evidence and an explicit options-bearing SCAD entry without silently changing every `load_model` caller.
- Surface:
  - `docs/adr/0017-file-format-io-boundary-separation.md`, Decision and affected dependency/graph statements: amend the existing ADR before landing the optional dependency in this step. Permit only geometry-only `slicer-scad` under default-off `scad`, to reuse the strict adapter rather than duplicate evaluation/conversion or reintroduce runtime I/O. Preserve `slicer-ir` as the sole first-party normal dependency without `scad`, the runtime/SDK/Proptest prohibition in the model-I/O/adapter production graph, model-I/O `assemble_object`/`MeshIR` ownership and runtime's preloaded `Arc<MeshIR>` boundary. No other leaf-rule exception or new ADR is authorized.
  - `crates/slicer-model-io/Cargo.toml`: new default-off `scad`, optional `slicer-scad`, explicit new `scad_ingest_feature_contract` and `scad_model_ingest` targets, latter requiring `scad`.
  - `crates/slicer-model-io/src/loader.rs`: extend `ModelFormat`/`detect_format`; make `load_model` return feature-appropriate SCAD diagnostic before ordinary file opening; implement `load_scad_model`, `ScadModelLoad` and typed error additions specified in the spec. Bound text reads, use original path for source/identity and call `slicer_scad::evaluate` once. Assemble only returned `Mesh` through `assemble_object`, with existing bounding-box union; classify `Contours`/`Empty` without another conversion.
  - `crates/slicer-model-io/src/lib.rs`: gated re-exports of the entry/result and exact adapter types.
  - New `crates/slicer-model-io/tests/scad_ingest_feature_contract.rs` and `scad_model_ingest.rs`: substantive off/on routing, analytic Boolean/disconnected/holed mesh observations, source identity and structured error propagation.
- New contract: exactly `load_scad_model(&Path, &ScadEvalOptions) -> Result<ScadModelLoad, ModelLoadError>`, result fields, re-exports and enum variants in the spec. No production IR fields/defaults/version changes.
- Fallout: update the exhaustive `load_model` format match and `ModelLoadError::fmt`; preserve I/O behavior and expose nested error `source()`. Search all `ModelFormat::` / `ModelLoadError::` users across `crates/` and account for their actual matches/tests, including `main.rs::place_model_on_bed`, 3MF layer-range detection and `model_loader_tdd`. New `ScadEvalOptions`/adapter literal sites use documented defaults/FRU or a justified exhaustive waiver, never silently unlimited limits. Do not copy `ObjectMesh` literal assembly.
- Verify: run existing `model_loader_tdd` / `bed_placement_tdd` baselines first; `MF` both states and `ME`; `cargo check -p slicer-model-io --all-targets --features scad`. Retain default/enabled normal/build graph output and source provenance; run the spec's ADR-0017 doc check and review its complete Decision for contradictory unconditional leaf/graph claims.
- Done when: ADR-0017 authorizes exactly the optional geometry-only SCAD edge with retained prohibitions/ownership, disabled detection has an actionable error, enabled legacy loading still refuses implicit evaluation, explicit loading yields one unplaced source-identified object, and all expected failures preserve adapter data.

## Step 2: Add command-local SCAD arguments and the import branch

- Objective: enable product conversion without inheriting STEP repair, multi-solid or warning-as-partial behavior.
- Surface:
  - `crates/pnp-cli/Cargo.toml`: new `scad = ["slicer-model-io/scad"]` retaining report defaults; declare ungated `scad_cli_feature_contract` and gated `scad_cli_ingest` targets.
  - `crates/pnp-cli/src/main.rs`: new feature-gated fields on `Cmd::Slice` and `MeshCmd::Import`, shared private parsing/validation for first-`=` parameters and positive finite f64 height, and early input-format/STEP-flag conflicts. Retain flags/variants on all other commands unchanged. Feature-off `.scad` import rejects before `run_import` can call STEP.
  - `crates/pnp-cli/src/helpers_cmd.rs`: new SCAD import branch consuming the explicit loader and existing private `write_mesh`; original `run_import` remains STEP-only. Preserve output-format defaults; write exactly the requested file, no component splitting or placement. Emit accepted information/stats and table-defined exits; do not publish `done` on failure.
  - New `crates/pnp-cli/tests/scad_cli_feature_contract.rs` and `scad_cli_ingest.rs`: real command parsing/import export/reload and source-coordinate observations. Build small scripts in test tempdirs; use `assert_cmd`'s Cargo-selected binary.
- Fallout: update every `Cmd::Slice`/`MeshCmd::Import` destructure when fields are added; check default, no-default and `scad` configurations so `cfg` does not leave noncompiling arms. New options structs use FRU. No config field/key additions or writer schema changes. Preserve existing STEP `helpers_cli` behavior, including `--output-format` rather than an invented import `--format` alias.
- Verify: `CF` off/on; captured `cargo test -p pnp-cli --test helpers_cli` and `cargo test -p pnp-cli --test cli_tdd`; `cargo check -p pnp-cli --all-targets --features scad`. Use the eventual `CE` target's import-focused tests through the gated runner while slice tests are being added; do not claim full `CE` acceptance before Step 3.
- Done when: both command help surfaces are correct; import invokes the real explicit loader; flags cannot enter slicer config; bad options/adapter errors leave destination absent or unchanged; every output format preserves analytic geometry without repair.

## Step 3: Wire real slicing and normal bare-model placement

- Objective: prove the production driver uses SCAD geometry and the established whole-model translation before normal slicing.
- Surface:
  - `crates/pnp-cli/src/main.rs`, `Cmd::Slice` arm: branch on detected `Scad`, retain diagnostics and `ScadModelLoad.mesh`, otherwise keep existing `load_model`; then call `place_model_on_bed` and the current `Arc<MeshIR>`/slice path. Keep 3MF project/layer-range reads format-specific and diagnostics on stderr before G-code output begins.
  - `place_model_on_bed`: add `ModelFormat::Scad` to the bare-model match; retain bed parsing/invalid-bed/overflow warnings and 3MF pass-through. Add registered inline feature-gated `scad_placement_tests` with test names beginning `scad_placement_`, directly calling this production function.
  - `crates/pnp-cli/tests/scad_cli_ingest.rs`: actual small primitive/Boolean and holed extrusion slices using current guest module search conventions, explicit parameter/config separation, successful progress events and meaningful nonempty extrusion-coordinate/layer observations. Add unplaced `mesh import` comparison and negative ingestion no-success/no-output assertions, including pre-existing destination bytes.
- Fallout: update only ordinary-bare-format comments/help; do not add SCAD evaluation to `object_ids_from_model`, preview, visual-debug or convert. Verify any `ModelFormat` matches stay exhaustive/default-compatible. Preserve object identity across placement and relative component offsets. No runtime/IR contract changes.
- Verify: `B`; `CE` through `cargo xtask test`; existing `slice_progress_events_default_tdd` through xtask; reread/archive each log. Before interpreting guest failures, enforce ordinary freshness by exit status, rebuilding stale/staged copies as documented.
- Done when: the real CLI produces placed, nonempty print moves and a successful slice event from SCAD; an unrelated command still cannot evaluate scripts implicitly; 3MF placement remains unchanged.

## Step 4: Complete strict negative coverage and dependency isolation

- Objective: ensure adapters cannot be bypassed by the frontend and optional production features stay independent of test helpers.
- Surface:
  - Model/CLI targets: real pin-compatible resource-with-sibling, parse/parameter, empty/mixed/2D/3D-height, partial/approximate/degraded, source-limit and conversion-damage cases. Reuse #3's documented reproducers and supervised-child pattern; do not add a test-only production escape hatch or fake unknown warning classification in the CLI.
  - `ModelLoadError::source` and frontend formatting/mapping: observe typed error, original source and parameter context, accepted diagnostic delivery and exact output exit table. Verify both file and stdout paths do not claim success after ingestion failure.
  - Cargo graph evidence: default workspace/model/CLI versus SCAD-enabled product normal/build graphs, no Proptest edge, no engine in defaults, no accidental guest feature activation.
- Verify: complete `MF`/`ME`/`CF`/`CE`/`B` selections with actual named execution evidence, all AC-N tests and relevant existing targets; default/enabled all-targets check/clippy gates in the spec.
- Done when: every error path is observed at its owning public boundary, impossible private-only converter/classifier cases remain explicitly owned/proved in #3, and enabled product graphs remain property-free. A missing prerequisite diagnostic is blocking fix work, not a waived case.

## Step 5: Windows lane, measured evidence and docs

- Objective: make the optional product feature continuously executable on the approved platform, with honest measured evidence.
- Surface:
  - `.github/workflows/ci.yml`: add `scad-ingest-windows` using #3's native setup plus existing gated-runner WASM prerequisites. Run feature-off/on meaningful targets and full focused import/slice acceptance; preserve each log and `target/scad-ingest/` source/options/output artifacts on failure. Existing Ubuntu and adapter jobs stay intact.
  - New `docs/scad/ingestion.md` / `ingestion-evidence.md` and the canonical doc sections listed in the spec. Correct the assembly argument-order example; extend placement applicability but not STEP semantics or adapter diagnostics. No Packet Queue edit.
  - Evidence: record actual revision plus dirty contents/settings, engine/native identity, selected features/targets and discovered named executions; measure build separately from import/render/slice execution. Derive finite fixture error envelopes and supervised-child settings from actual operations/measurements. No guessed performance budget.
- Verify: actual Windows `MF`/`ME`/`CF`/`CE`/`B` and relevant existing tests; required build/check/clippy/literal/test-quality commands from the spec, doc check commands and final scoped diff review. No workspace test run is required.
- Done when: ACs have saved concrete output evidence, the required Windows job executes both feature states, docs match commands/policies, and incomplete prerequisites/measurements are not reported complete. Leave status draft until `/swarm` closure and independent review.
