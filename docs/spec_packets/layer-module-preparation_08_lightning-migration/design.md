# Design: lightning-migration

## Decision State

**Implementable draft.** The planner and the two plan types move from the host into the consuming artifact; the move is proven by recorded behavioural equivalence and then the host contract is deleted. The ADR conflict is resolved by in-place amendments to ADR-0066 and ADR-0029, both dated 2026-10-20 and both quoting the clause they supersede. No `[BLOCK]` remains.

What is still open is only the *outcome* of running the equivalence comparison, which cannot be authored in advance.

## Decisions That Replace the Former Blockers

### D1 — Correctness is recorded equivalence to the live producer (was B1)

The migration's promise is "same output, different owner". The oracle is the current producer's own output, recorded before removal:

- **Capture**: a committed baseline fixture produced by a recorder test driving the real pipeline with the live host producer. Per case: plan entries keyed by `(global_layer_index, object_id, region_id, ordered variant_chain)` → directed edge multiset + grounding/root witnesses; the final caller-attributed segment multiset; `sha256` of the plan payload and of the inputs it was derived from.
- **Compare**: after the move, native and fresh-WASM preparations each reproduce that baseline. Key identity is the full key; edges compare as a **multiset** with multiplicity; segments compare per case and per caller. Adapters are compared to the baseline, never to each other.
- **Diagnose**: the separate input digest is what makes an "algorithm differs" claim honest — a run whose inputs hash differently has a plumbing defect, and its plan comparison says nothing about the algorithm.

**Corruption controls** (AC-N1) exist because a comparator is easy to make vacuous: a dropped entry, reversed edge, dropped `variant_chain` element, duplicated entry, swapped region identity, or a length-only stub must each be rejected, and an empty population must fail its own control.

**Equivalence is not parity.** It preserves whatever the live producer does, including its existing deviations. Row 07's `Gate A outcome: fail` and row 09's unexecuted Gate B remain recorded and are neither consumed nor improved here.

### D2 — Inputs come from the committed typed preparation surface (was B2)

The host producer reads `Blackboard::slice_ir()` (per-region `object_id`, `region_id`, `variant_chain`, `polygons`) plus `ResolvedConfig.{sparse_fill_holder, infill_density, layer_height, infill_angle, line_width}` resolved through `flow::resolve_role_width(ExtrusionRole::SparseInfill, …)`.

The guest's typed equivalent. All three accessors return `Result<Vec<…>, PreparationInputError>` — the error is part of the contract and a missing declared read surfaces as `PreparationInputError::MissingRequired { read }`:

| Host input | Guest source |
| --- | --- |
| `SliceIR.regions[].polygons` | `PrintPreparationInput::regions(layer_index, offset, max_items) -> Result<Vec<PreparationRegion>, PreparationInputError>` → `polygons` (holes travel inside each `ExPolygon`), plus `sparse_infill_area`, `infill_areas`, `internal_solid_fill`, `internal_bridge_areas` |
| `object_id` / `region_id` / `variant_chain` | `PreparationRegion::{object_id, region_id, variant_chain}`, `PreparationVariantChain = Vec<(String, PreparationPaintValue)>` |
| `layer_index` / `z` | `PreparationRegion::{global_layer_index, z}` |
| `ResolvedConfig` scalars | `PrintPreparationInput::region_configs(...) -> Result<Vec<PreparationRegionConfig>, PreparationInputError>` → `PreparationRegionConfig.config: ConfigView` (`get_float`/`require_float`), plus `PreparationRegion.effective_layer_height` |
| selected targets / eligibility | `PrintPreparationInput::selected_targets(...) -> Result<Vec<PreparationSelectedTarget>, PreparationInputError>` (`invoke`, `source`, `config`, `held_claims`, `resolved_role_tool`). The target list itself is host-side (`slicer_wasm_host::selection::LayerSelection::targets`) and feeds the shared predicate; the module never re-reads it |

**Who builds the kernel input.** The host handed the producer `&[SliceIR]` and `&ResolvedConfig` directly. The guest cannot: it receives typed preparation records. `modules/core-modules/lightning-infill/src/kernel/input.rs` therefore defines `KernelInput { layers: Vec<KernelLayer> }` and `KernelConfig`, and `src/preparation.rs` populates them from the typed records. This adaptation is the one place where the move is not purely mechanical, and it is exactly what the input digest in the baseline is for.

Manifest declaration: `[preparation] interface = "slicer:layer-preparation/prepare@1.0.0"`, `input_reads = ["slices", "region_map", "selected_targets"]`, `views = ["planning_geometry"]`. Step 7b confirms each declared read against `validate_reads` in `crates/slicer-wasm-host/src/preparation_input.rs` and the accessor it feeds. `fixture` grants no typed read and is not declared.

**Eligibility moves from a config string to selection.** Today the producer runs when `default_resolved_config.sparse_fill_holder == "lightning-infill"`. After migration, preparation runs when the shared predicate in `PrintPreparationScope::capture_committed_selection` holds: `index.selection_for(layer.index, &frozen.module_id).is_some_and(|s| s.invoke && !s.targets.is_empty())`. For the same print this selects the same work; the string branch disappears with the producer. AC-N3 asserts the gating behaviour, AC-3 asserts the output.

### D3 — Private pieces are module-owned, versioned and bounded (was B3)

- **Granularity**: one piece per layer, named `lightning-layer-{global_layer_index}`. Cross-layer tree construction happens inside preparation; a consumer only needs its own layer, so every read stays bounded and no offset table or shared codec is needed. If the row-03 name validator rejects the spelling, the step records the accepted form and uses it — the format is settled here, the exact accepted spelling is a step exit condition.
- **Encoding**: module-local, little-endian, fixed width. Header `b"LTPL"`, `u16` format version `1`, `u32` entry count; per entry `u32` object-id length + UTF-8, `u64` region id, `u32` layer index, `u32` variant-chain count + per element (`u32` key length + UTF-8, `u8` paint tag + payload), `u32` edge count + per edge four `i64` coordinates in **units** (1 unit = 100 nm). No float crosses the boundary.
- **Decode errors**: missing piece keeps the inherited `PreparedPlanView::require` contract — `ModuleError` code `57401`, `fatal == true`, `message == format!("required preparation piece missing: {}", name)`. Malformed body → `57410`, `malformed lightning plan piece: {name}`. Unsupported version → `57411`, `unsupported lightning plan piece version: {version}`. `57410`/`57411` are allocated here rather than reusing `57402`/`57403`, which `docs/05_module_sdk.md` already gives the fixture transport decoder a different meaning; row-05 codes stay untouched. AC-N2 asserts all three on the lightning module's own errors.
- **Opaque to the host**: no runtime or CLI code decodes these bytes, discovers views by piece name, or receives them serialized. Bounds are enforced on every `read(offset, max_bytes)` in module code.

### D4 — Retirement is partitioned, with the contract package deleted whole (was B4)

The inventory below is re-derived from the tree. The previous revision's WIT citation was stale twice over: the accessor lives in `lightning-queries.wit::lightning-query-view`, and the package's *only* member is that resource, so retiring the accessor retires the interface and the package. Deleting a hollow interface instead would leave a package nothing imports.

Each deletion step owns every constructor, literal, exhaustive match, old-value assertion, registration and doc line its symbol removal breaks. If a step cannot fit three files, it is split before work starts.

### D5 — The evidence harness is the fixture, not a status field (was B5)

The former non-zero `python -c … sys.exit('BLOCKED …')` hold is deleted. Its replacement is the baseline fixture plus the recorder test, which fail when the baseline is missing, stale (input digest mismatch), non-deterministic (two runs differ) or incomplete (a case absent). Nothing in this packet reads a gate status to decide eligibility, so no synthetic status can substitute for evidence.

## Behavioural Equivalence Contract

Equivalence is asserted at three points, and the distinction matters:

1. **Input identity** — the baseline records `sha256` of the exact inputs the producer consumed; a differing input hash is a plumbing defect, not an algorithm verdict.
2. **Plan identity** — key set equality with full identity including ordered `variant_chain`, edge multiset equality with multiplicity, grounding/root witness set equality.
3. **Output identity** — per caller, the emitted segment multiset, role `ExtrusionRole::SparseInfill`, widths, and the `variant_chain` that reached the emitted region.

## Controlling Code Paths (current, verified)

- **Producer**: `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs::{LIGHTNING_TREE_PRODUCER, commit_lightning_tree_ir_builtin}` declares `host:lightning_tree` on stage `PrePass::LightningTreeGen`, `ir_writes: &["LightningTreeIR"]`, and calls `slicer_core::algos::lightning::generate_lightning_trees`. Wired directly in `crates/slicer-runtime/src/prepass.rs` via `run_builtin_stage`, gated by the `sparse_fill_holder` string; `runtime_builtins()` deliberately does not register it.
- **Algorithm**: `crates/slicer-core/src/algos/lightning/` — `mod.rs` (`generate_lightning_trees(&[SliceIR], &ResolvedConfig) -> Result<Arc<LightningTreeIR>, LightningTreeError>`), `generator.rs` (`Generator::generate_trees`, `committed_segments_for_layer`), `layer.rs`, `tree_node.rs` (`Node`/`NodeRef`), `distance_field.rs` (`DistanceField`), `error.rs`; 1729 lines total, behind `#[cfg(feature = "host-algos")]` in `crates/slicer-core/src/algos/mod.rs`. Std-only, no threads, no host facilities; its `slicer-core` internal dependencies (`polygon_ops::clip_polylines`, `geometry::closest_point_on_segment`, `difference`/`offset`) are ungated.
- **Plan types**: `crates/slicer-ir/src/slice_ir.rs::{LightningTreeEntry, LightningTreeIR, CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION}` — the algorithm's return type is this public IR, which is why the move and the type's relocation are one decision, not two.
- **Consumer**: `modules/core-modules/lightning-infill/src/lib.rs` (174 lines) is an ordinary `LayerModule`; it reads `PaintRegionLayerView::lightning_tree_segments_for(object_id, region_id, variant_chain)` and pushes one sparse path per segment, discarding the `push_sparse_path` result. No `LayerPreparation`, no preparation reads, no `[preparation]` manifest section.
- **Contract transport**: `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::{lightning_tree_segments_for, with_lightning_tree_ir, lightning_tree_ir}`; `crates/slicer-wasm-host/src/{binding,dispatch,host}.rs`; `crates/slicer-wasm-host/src/marshal/native.rs`; macro glue `crates/slicer-macros/src/lib.rs::{__slicer_adapt_paint_layer, __slicer_lightning_tree_from_view}` plus `LIGHTNING_QUERIES_WIT`; `crates/slicer-macros/build.rs` watch entry.
- **Preparation surface consumed** (rows 01–06, implemented): `slicer_sdk::preparation::{LayerPreparation, PrintPreparationInput, PreparedPlanOutput, PreparedPlanView, PreparedPiece, PreparationError}`, `crates/slicer-sdk/src/preparation_visual.rs::{PreparationProjectionScene, PreparationProjectionPrimitive, PreparationProjectionGeometry, PreparationProjectionOutput}`, `crates/slicer-runtime/src/preparation_lifecycle.rs::{PrintPreparationScope, PreparationCapture, SliceRuntimeExtensions}`, `crates/slicer-runtime/src/preparation_visual_capture.rs::{PreparationViewRequest, PreparationVisualCapture, PreparationVisualCaptureError}` and `crates/slicer-runtime/src/run.rs::run_slice_preparation_visual_capture`.

## Code Change Surface

### New kernel inside the consumed artifact

| Path | Change |
| --- | --- |
| NET-NEW `modules/core-modules/lightning-infill/src/kernel/mod.rs` | Kernel entry `plan_layer_trees(&KernelInput) -> Result<KernelPlan, KernelError>` and module wiring |
| NET-NEW `modules/core-modules/lightning-infill/src/kernel/plan.rs` | `KernelPlan` / `PlanEntry`, moved from `crates/slicer-ir/src/slice_ir.rs`, `pub(crate)`, no schema-version constant |
| NET-NEW `modules/core-modules/lightning-infill/src/kernel/input.rs` | `KernelInput`/`KernelLayer`/`KernelRegion`/`KernelConfig` — the typed adaptation of D2 |
| NET-NEW `modules/core-modules/lightning-infill/src/kernel/{generator,layer,tree_node,distance_field,error}.rs` | Moved from `crates/slicer-core/src/algos/lightning/`; module paths and container types updated, planning decisions untouched |
| NET-NEW `modules/core-modules/lightning-infill/src/preparation.rs` | `impl LayerPreparation for LightningInfill`: read typed inputs, build `KernelInput`, plan, encode pieces, publish projection |
| NET-NEW `modules/core-modules/lightning-infill/src/private_plan.rs` | D3 encoder/decoder, bounds checks, `57410`/`57411` |
| NET-NEW `modules/core-modules/lightning-infill/src/projection.rs` | Typed `planning_geometry` scene from the same in-memory plan |

The kernel lives inside the module tree, so it is a plain module (no new crate, no workspace change, no feature flag); the guest compiles it directly and `slicer-core` stops owning lightning at all.

### Edited existing files

| Path | Change |
| --- | --- |
| `modules/core-modules/lightning-infill/src/lib.rs` | `#[slicer_module(preparation)]`; `run_infill` consumes `PreparedPlanView::require("lightning-layer-{index}")` and the module decoder |
| `modules/core-modules/lightning-infill/lightning-infill.toml` | Add `[preparation]` |
| NET-NEW `modules/core-modules/lightning-infill/tests/kernel_tdd.rs` | Moved from `crates/slicer-core/tests/algo_lightning_tdd.rs`, adapted to the kernel API; assertions preserved |
| `crates/slicer-core/src/algos/lightning/` (directory), `crates/slicer-core/src/algos/mod.rs`, `crates/slicer-core/Cargo.toml` | Delete the host copy and its `[[test]]` entry |
| NET-NEW `crates/slicer-runtime/tests/e2e/lightning_baseline.rs`, `lightning_migration_tdd.rs`; `crates/slicer-runtime/tests/e2e/main.rs` | Recorder + nine acceptance tests; registration |
| `crates/slicer-runtime/tests/fixtures/lightning_baseline/{baseline_classic.json, manifest.json}` | Committed baseline and digests |

### Retirement inventory (grounded, current)

| Path / symbol | Obligation |
| --- | --- |
| `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs` | Delete file; remove the `pub use` in `crates/slicer-runtime/src/lib.rs` and the module export in `builtins/mod.rs` |
| `crates/slicer-runtime/src/prepass.rs` | Remove the `run_builtin_stage` block, `PrepassExecutionError::LightningTree`, the `"PrePass::LightningTreeGen" => &[…]` arm of `required_slots`, and the `BlackboardPrepassSlot::LightningTreeIR => blackboard.lightning_tree_ir().is_some()` arm of `ensure_stage_prerequisites` (`stage_requires_region_map` is a derived helper with no per-stage arm) |
| `crates/slicer-runtime/src/blackboard.rs` | Remove `lightning_tree_ir` field, `commit_lightning_tree_ir`, getter, `estimated_lightning_tree_ir_bytes` and its accounting |
| `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-ir/src/lib.rs`, `crates/slicer-ir/src/stage_io.rs` | Delete `LightningTreeIR`/`LightningTreeEntry`/version const and re-exports; remove `BlackboardPrepassSlot::LightningTreeIR`, its `Display` arm and `BlackboardError::LightningTreeGeneration` |
| `crates/slicer-scheduler/src/execution_plan.rs` + `tests/stage_order_tdd.rs` | Remove `"PrePass::LightningTreeGen"` from `STAGE_ORDER` and the stale expectation |
| `crates/slicer-sdk/src/traits.rs` | Delete `lightning_tree_segments_for`, `with_lightning_tree_ir`, `lightning_tree_ir`, the field and its builders |
| `crates/slicer-wasm-host/src/{binding,dispatch,host}.rs` | Remove `LayerStageInput::lightning_tree_ir` and constructors; remove the legacy dispatch/test helper; remove `LightningQueryViewData`, the `lightning_tree_segments` handler and the bindgen `with:` key |
| `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-wasm-host/src/marshal/native.rs` | Remove the field at every construction and the native attachment |
| `crates/slicer-macros/src/lib.rs`, `crates/slicer-macros/build.rs` | Remove the paint-layer/tree glue, `LIGHTNING_QUERIES_WIT` + its `nest_dep` branch, and the watch entry |
| `crates/slicer-wasm-host/src/preparation_bindings.rs` | Remove the linker `with:` key for the deleted package |
| `crates/slicer-schema/wit/deps/lightning-queries/lightning-queries.wit` | Delete the package (its only member is the retired resource); remove its `import`/`use` from `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` |
| `modules/community-modules/dragon-curve/wit/` | Delete the vendored package copy and its `layer-infill.wit` import; update `Makefile`/glue and regenerate bindings via `make bindings` (generated output is regenerated, never hand-edited) |
| `xtask/src/wit_verify.rs` | Remove the package from `SHARED_PACKAGES`, from the expected include-str set, the watch-list expectations, the difference array and the inline package snippets |
| Affected tests | Listed per binary in `implementation-plan.md` Steps 14a–14e; retarget where the subject survives |
| `docs/feasibility-probes/foreign-language-text-postprocess/` | **No edit.** Its `ir-types.wit` is already stale and its Go bindings are generated historical output, outside the live contract. Record that in the WIT doc section instead of regenerating it |

## Diagnostic Representation

Declare only `planning_geometry`; scene id `lightning-plan`. `planning_domain` uses `PreparationProjectionGeometry::Polygon { polygon: ExPolygon, filled: bool }` with holes preserved; `tree_branch` uses `Polyline` with open paths; `tree_root` and `grounding` use `Point(Point2)` with labels carrying source/checkpoint identity. Primitive ids are deterministic and scene-local, derived from the plan key plus an edge or node index — never an assumed array position. A requested empty scene declares its classes and publishes `primitives: []`; a requested-but-missing scene is a named row-06 error. Layer Z comes from the host schedule, never from module payload. Diagnostic classes are not extrusion roles and carry no width.

Pieces and scene are built from the same in-memory `KernelPlan` inside one `prepare_print` call and share the transaction: a projection failure fails the preparation.

## Architecture Constraints

- Same selected artifact and frozen identity; one `prepare_print` per owner before any consumer. Host owns transport/access/lifetime only.
- No `LayerArena` survives preparation. `retained_bytes == 0` after success, failure and cancellation. Disposal reports `Busy` for active guards; callers join, never spin or revoke.
- **Planning logic is frozen.** The kernel may change file location, container types, error type and input adaptation. It may not change a planning decision: no reordering of layer iteration, no domain/parameter change, no change to grounding or clipping. Equivalence is the detector; if it fails, the packet stops rather than adjusting the baseline.
- After retirement, nothing under `crates/slicer-runtime/**` or `crates/slicer-wasm-host/**` references the lightning kernel; the only production caller is the module.
- No speculative public version bump. Deleting the WIT package is an affected-contract change: regenerate affected bindings and rebuild guests.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- New translated files carry the header from `docs/ORCASLICER_ATTRIBUTION.md`. This packet relocates in-tree code and translates nothing new.

## Read-Only Context and Out-of-Bounds

Read symbol-centred bounded windows. Delegate: any file over 300 lines, all cross-crate closure surveys, every cargo run, the baseline JSON (inspect via recorder/decoder, never by loading it), and all OrcaSlicer inspection.

**Out of bounds for every step**: `modules/core-modules/lightning-infill/feasibility-kernel/` (row 07/09 artifacts, untouched), `crates/slicer-wasm-host/tests/fixtures/lightning_gate/` (input material, never modified), other packet directories, `target/`, lockfiles, vendored trees, and the historical probe under `docs/feasibility-probes/`.

Editing the kernel is permitted **only** as relocation plus the D2 input adaptation; any change to a planning decision is out of bounds by construction.

## Expected Sub-Agent Dispatches

- **Recorder/equivalence execution**: run the real driver, return FACT with the first divergence (case, key, expected vs actual multiplicity class) and archived log paths. Never paste the fixture.
- **Symbol closure**: per retirement step, LOCATIONS (≤ 20) of every remaining reference to the symbol being deleted, re-derived at edit time.
- **Guest transport**: confirm the deleted package's guests regenerate and `cargo xtask build-guests --check` exits 0, or 1 with named stale guests.
- **Doc placement**: confirm each `### ` heading exists exactly once in its target doc.
- **Test retarget review**: for each retargeted test, name the contract retained and the population asserted. A retarget that leaves a success-only assertion is not a retarget.

## Locked Invariants and Risks

- No reference contamination: the baseline is read only by the comparator; planning reads typed inputs and config, nothing else.
- No plan injection: tests instantiate the real module and let it prepare.
- Multiplicity survives unions, splits and subtractions; a set comparison is a defect even when it passes.
- Empty successful preparation is `Ready`/no-work; a missing piece is an error, never an empty result.
- **Risk — the input adaptation changes a value.** `SliceIR` regions and typed `PreparationRegion` values may differ (notably `sparse_infill_area` after `sync_perimeter_infill_areas_into_slice`). The recorded input digest exposes it; the fix belongs in the input contract, and if it cannot be expressed there the packet stops for re-approval.
- **Risk — relocation is not behaviour-free.** Container changes (`Arc` → `Rc`, map types) are permitted only if equivalence holds. If equivalence fails for a reason that cannot be fixed without changing a planning decision, the packet stops.
- **Risk — retirement is wide.** Roughly 45/26/24/17 files mention the four legacy symbols, most of them tests and docs, plus a whole WIT package whose expectations are hardcoded in `xtask`. The partition bounds each step to three files; the risk this controls is compiler-driven cleanup that drops an unrelated assertion.

## Open Questions

- [FWD] Does the relocated kernel reproduce the recorded baseline exactly on all five Classic cases, and where is the first divergence if not? Recorded at execution; a divergence blocks deletion.
- [FWD] Does the row-03 piece-name validator accept `lightning-layer-{index}` unchanged? Answered by Step 7a's exit condition.
- [FWD] Which declared `input_reads` does each typed accessor actually require? Answered by Step 7b's `MissingRequired { read }` signal.

## Author Self-Review

Approach, oracle, representation, partition and harness are settled, each with a falsifiable exit. The earlier revision's compile-fatal hole — a planner returning a type the same plan deleted — is closed by moving algorithm and plan types together. Acceptance criteria exist only in `packet.spec.md`; scope and matrix only in `requirements.md`; code surface here; step contracts in `implementation-plan.md`; backlog crosswalk in `task-map.md`. ADR-0066 and ADR-0029 carry dated in-place amendments quoting the superseded text. No claim of canonical parity is made anywhere.
