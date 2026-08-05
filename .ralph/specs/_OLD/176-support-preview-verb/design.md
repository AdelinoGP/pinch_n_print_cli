# Design: 176-support-preview-verb

## Controlling Code Paths

- Primary code path: `crates/pnp-cli/src/main.rs` (`Cmd` enum at line 52; dispatch match) → new `crates/pnp-cli/src/support_preview.rs` → `slicer_model_io::load_model` → `slicer_runtime::parse_cli_config_source` (pattern: `visual_debug.rs:994`) → `slicer_runtime::prepare_prepass_context` (`run.rs:744`) → `PrepassContext { plan, blackboard, .. }` (`run.rs:701`) → `Blackboard::support_geometry() -> Option<&Arc<SupportGeometryIR>>` (`blackboard.rs:271`).
- Neighboring tests/fixtures: `crates/pnp-cli/tests/` per-file binaries (no aggregator); candidate fixtures `resources/bridge_support_enforcers.3mf`, `resources/bridge.obj`.
- OrcaSlicer comparison: none — this verb is a fork-specific contract with no Orca counterpart; no parity obligations.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Use `slicer_ir::units_to_mm(i64) -> f32` (slice_ir.rs:71) / `Point2::to_mm()` (slice_ir.rs:98) for the conversion — never a hand-rolled factor. Emit as f64 for JSON.
- `SupportGeometryIR.entries: HashMap<SupportGeometryKey, Vec<ExPolygon>>` (slice_ir.rs:1175-1185) — HashMap iteration order is nondeterministic; the handler MUST sort (by `layer_index`, then `object_id`, then `region_id`) before emission so output is byte-deterministic across runs.
- `SupportGeometryKey.global_support_layer_index == u32::MAX` is the intermediate-model-resolution sentinel (slice_ir.rs:1160-1168) — must be skipped and counted, never used to index `plan.global_layers` (it would panic).
- Latency contract: the verb runs Tier 1 (prepass) only — module loading, config resolution, plan build, slicing, mesh analysis, paint segmentation, support geometry. It never constructs a per-layer closure, never touches `execute_captured_stages` / `execute_postpass*`, so walls, infill, support path generation, path optimization, and G-code emit are all skipped by construction. `prepare_prepass_context` additionally skips the 14-pass startup DAG validation and thumbnail/CONFIG_BLOCK wiring (documented on the fn, `run.rs:729-737`).

## Code Change Surface

- Selected approach: direct blackboard-slot read after `prepare_prepass_context` (the same committed slot the `PrePass::SupportGeometry` blackboard tap reads), rather than going through `execute_blackboard_taps` + `CaptureRequest`.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/pnp-cli/src/main.rs`: add `Cmd::SupportPreview { input: PathBuf, output: PathBuf, config: Option<PathBuf>, module_dir: Vec<PathBuf>, no_default_module_paths: bool }` (mirror `Slice`'s flag names/attrs) + one dispatch arm calling `support_preview::run_support_preview(...)` and mapping `Err` to a nonzero exit (mirror the `Slice` arm's error handling); `mod support_preview;`.
  - New `crates/pnp-cli/src/support_preview.rs`:
    - `pub fn run_support_preview(input: &Path, output: &Path, config: Option<&Path>, module_dirs: &[PathBuf], no_default_module_paths: bool) -> Result<(), String>`
    - serde structs: `SupportPreviewDoc { schema_version: String, units: String, layer_count: u32, skipped_intermediate_entries: u32, layers: Vec<SupportPreviewLayer> }`, `SupportPreviewLayer { layer_index: u32, z_mm: f64, support: Vec<SupportPreviewExPolygon> }`, `SupportPreviewExPolygon { contour: Vec<[f64; 2]>, holes: Vec<Vec<[f64; 2]>> }`.
    - Flow: read/validate input exists → `load_model` → config map (empty when `--config` absent) merged the way `visual_debug.rs` builds `config_source` → `prepare_prepass_context(Arc::new(mesh), config_source, module_dirs, no_default_module_paths)` → `blackboard.support_geometry()`; `None` ⇒ empty `layers` (AC-N1); `Some` ⇒ sort keys, skip/count sentinels, group by layer, `z_mm = plan.global_layers[i].z as f64`, convert points via `Point2::to_mm()` → `serde_json::to_string_pretty` → write file. Write output only after successful serialization (AC-N2: no partial file on error).
  - Tests: new `crates/pnp-cli/tests/support_preview_tdd.rs` — prefer calling `run_support_preview` in-process (mirrors how other pnp-cli tests call library fns) with `tempfile` outputs; AC-4's sentinel case builds the doc from a synthetic `SupportGeometryIR` via a small pure helper (`fn build_preview_doc(geometry: &SupportGeometryIR, global_layers: &[GlobalLayer]) -> SupportPreviewDoc`, exported for tests) so it needs no fixture that happens to produce intermediate layers.
  - Docs: `docs/20_support_preview.md` (contract), `.claude/doc-index.md` row.
- Rejected alternatives and reasons:
  - `execute_blackboard_taps` + `CaptureRequest`: pulls in visual-debug request/manifest types and layer-universe validation the verb doesn't need; the tap ultimately reads the same slot; rejected for surface area.
  - Extending `run_slice` with a `--stop-after-stage` flag: contaminates the production entry point's option surface and its skipped-validation semantics differ (`prepare_prepass_context` already exists precisely for partial runs); rejected.
  - Emitting `SupportPlanIR.branch_segments` too: paths not polygons, doubles the contract surface before the fork asks for it; rejected (future minor bump).
  - JSONL (one line per layer): the fork loads the whole preview atomically; a single JSON doc with `schema_version` is simpler to version; rejected.

## Files in Scope (read + edit)

- `crates/pnp-cli/src/support_preview.rs` — role: new handler + serde contract + `build_preview_doc` helper; expected change: created (~200 lines).
- `crates/pnp-cli/src/main.rs` — role: CLI surface; expected change: one enum variant + one dispatch arm + `mod` line.
- `crates/pnp-cli/tests/support_preview_tdd.rs` — role: all ACs; expected change: created.
- (Extras, justified): `docs/20_support_preview.md` + `.claude/doc-index.md` (doc steps only).

## Read-Only Context

- `crates/pnp-cli/src/visual_debug.rs` — lines `1325-1375` only — purpose: the load_model → config → `prepare_prepass_context` wiring pattern.
- `crates/slicer-runtime/src/run.rs` — lines `694-800` only — purpose: `PrepassContext` fields and `prepare_prepass_context` signature/seeding behavior.
- `crates/slicer-runtime/src/blackboard.rs` — lines `255-275` only — purpose: `support_geometry()` accessor.
- `crates/slicer-ir/src/slice_ir.rs` — lines `60-100`, `990-1010`, `1155-1200`, `1333-1346` only — purpose: `units_to_mm`/`Point2::to_mm`, `GlobalLayer`, `SupportGeometryKey`/`SupportGeometryIR`, `Polygon`/`ExPolygon`.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — no parity for this packet; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load
- `crates/slicer-runtime/src/layer_executor.rs`, `postpass.rs`, `prepass.rs`, `pipeline.rs` — precedent only; the verb must not edit or load them
- `crates/pnp-cli/src/visual_debug.rs` outside the stated range; `modules/**`; `crates/slicer-schema/wit/**`

## Expected Sub-Agent Dispatches

- Question: does slicing `resources/bridge_support_enforcers.3mf` (or `bridge.obj` + `enable_support=true`) through prepass commit a non-empty `SupportGeometryIR`? Which config keys are required (enable_support, support type)?; scope: `cargo run --bin pnp_cli -- slice ...` probe or the support-geometry module's manifest under `modules/core-modules/`; return: `FACT` (fixture name + required keys); purpose: Step 1 fixture lock.
- Question: SUMMARY of `docs/19_visual_debug.md`'s description of `prepare_prepass_context` reuse (precedent citation for docs/20); scope: `docs/19_visual_debug.md`; return: `SUMMARY`; purpose: Step 4 doc authoring.
- Question: run `cargo xtask build-guests --check` before the first e2e test run; scope: workspace; return: `FACT`; purpose: Step 3 precondition.

## Data and Contract Notes

- IR/manifest contracts: read-only consumers of `SupportGeometryIR` (schema 1.x) and `ExecutionPlan.global_layers`; no IR change, no schema bump.
- WIT boundary: untouched.
- Determinism/scheduler constraints: output must be byte-deterministic for identical input+config (sorted iteration, `to_string_pretty` stable field order via struct definition) — the fork may cache previews by content hash.
- Fork-facing JSON contract is versioned independently (`schema_version: "1.0.0"` — a document version, not any IR's version; additive fields bump minor).

## Locked Assumptions and Invariants

- `SupportGeometryIR` coarse outlines are the preview product; the fork accepts geometry-stage fidelity (no per-layer support paths, no interface split) — locked by the approved plan's "run the pipeline through the support stage ONLY".
- `GlobalLayer.z` is millimeters (f32) — grounded against constructors in `crates/slicer-ir/tests/ir_tests.rs` (`z: 0.2`) and emit's mm-based layer z usage.
- Absent/empty support geometry is success (`layers: []`), never an error.

## Risks and Tradeoffs

- Fixture risk: `bridge_support_enforcers.3mf` may not drive the support-geometry module without extra config keys; Step 1's dispatch locks the fixture + keys before tests are written (fallback: `bridge.obj` with explicit `--config`). The ACs assert schema/conversion, not specific polygon shapes, so fixture substitution does not weaken them.
- Latency: prepass still slices the whole model; acceptable for paint-time use (no Tier 2/3), but the contract doc must state the cost is model-size-dependent so the fork debounces calls.
- Coarse outlines may differ from final support paths (post-plan trimming happens in Tier 2); docs/20 must state the preview is approximate by design.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3, handler e2e tests)
- Highest-risk dispatch and required return format: fixture/config probe — `FACT` (wrong fixture invalidates the AC-1 test premise).

## Open Questions

- `[FWD]` Whether the verb should also honor 3MF-embedded sidecar config automatically (as `run_slice` does via loader metadata) or rely solely on `--config`: mirror whatever `load_model` already surfaces for the fixture; if the enforcer paint drives support without extra keys, `--config` stays optional. Resolvable by Step 1's FACT.
- `[FWD]` `layer_count` source: use `ctx.plan.global_layers.len() as u32` (model layers). If the fixture's support layer height diverges from model layer height, document that `layer_index` is a model-layer index — resolvable while authoring docs/20.
- None blocking.
