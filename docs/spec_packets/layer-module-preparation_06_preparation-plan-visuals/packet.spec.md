---
status: draft
packet: layer-module-preparation_06_preparation-plan-visuals
task_ids:
  - TASK-575
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: preparation-plan-visuals

## Goal

Publish optional owner-private typed XY diagnostic snapshots atomically with prepared plans, and render them through the existing visual-debug bundle path with truthful execution evidence.

## Scope Boundaries

Two internal gates remain distinct: A is generic projection transport, validation and atomic ownership; B is visual-debug selection, production-prefix capture, rendering and bundle compatibility. This is framework work, not lightning visualization or an opaque payload decoder. Authoring produces a draft only; no implementation or acceptance commands have run.

## Prerequisites and Blockers

- Required queue dependency: `docs/spec_packets/layer-module-preparation_05_preparation-runtime-lifecycle/`; its #03/#04 dependencies are transitive. Direct interface authority also includes `docs/spec_packets/layer-module-preparation_03_preparation-contract-transport/` and `docs/spec_packets/layer-module-preparation_04_whole-print-input-views/`.
- These are FORWARD-DEPs, not existing APIs or passed gates. Activation requires their implementation and executable pilot evidence, plus independent packet preflight. #06 does not waive any prerequisite.
- Unblocks generic framework visual acceptance and #08 migration; #08 still requires its independent #07 geometry/portability gate.

## Acceptance Criteria — gate A

- **AC-1. Given** the same-source native/component visual fixture with view `probe_xy`, **when** its real preparation adapter receives that request, **then** one committed scene `final` contains classes `anchors`, `domains`, `paths`, a point `(10000,20000)`, open line `[(0,0),(20000,0)]`, filled contour `[(0,0),(40000,0),(40000,40000),(0,40000)]` with clockwise hole `[(10000,10000),(10000,30000),(30000,30000),(30000,10000)]`, and a separate outline polygon; labels and primitive ids survive exactly, the private `probe` piece remains four bytes, and metadata never contains those bytes. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --features preparation-test-fixtures --test contract preparation_projection_tdd::typed_projection_native_and_wasm -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-2. Given** two owners in separate prints with identical view/scene ids and four-byte `probe` pieces, **when** projections commit and owners dispose, **then** each handle sees only its own immutable scene, foreign/disposed access returns `AccessDenied`, storage is released after active calls drain, and retained diagnostic byte totals become zero without exporting payload bytes. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --features preparation-test-fixtures --test contract preparation_projection_tdd::projection_owner_and_lifetime -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-3. Given** one real selected prepared fixture, **when** full slices run with and without `probe_xy` capture, **then** independently pinned ordinary `probe` comments and emitted G-code are identical, private piece names/lengths and module-owned plan results agree, the unrequested path has zero projection publications and zero staged/retained diagnostic bytes, while the requested path has a nonempty committed typed snapshot and actual diagnostic accounting. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e preparation_visual_capture_tdd::capture_is_semantically_neutral -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`

## Acceptance Criteria — gate B

- **AC-4. Given** schema `1.4.0` model-source taps `{"preparation":{"module_id":"test.preparation-visual","view":"probe_xy"}}` and selected layer 0, **when** preparation-only visual capture executes, **then** its preparation owner has `Ready`, lifecycle phase `Preparation` and `global_layer_index: None`, arena Created/Committed/Dropped and consumer-call counts are zero, `executed_layer_indices` is empty, `rendered_layer_indices` is `[0]`, snapshot Z is the real schedule Z, and metadata capture still reports `transferred_bytes: 0` and post-disposal `retained_bytes: 0`. Validation uses the shared full startup prefix, not `prepare_prepass_context`. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e preparation_visual_capture_tdd::preparation_only_uses_production_prefix -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-5. Given** a preparation tap plus `Layer::PathOptimization`, **when** schema `1.4.0` renders `preparation_geometry` and ordinary `filament_lines`, **then** the same production preparation runs once, real selected-layer closure consumes its Ready plan, both entries have identical `world_bounds_mm`, point/line/polygon/hole pixels match independent projector-coordinate expectations, and the preparation image has `capture_phase: "preparation"`, exact owner/version/artifact, `consumer_stage: "Layer::PathOptimization"`, `view: "probe_xy"`, `projection_schema: "1.0.0"`, real `layer_index`/`layer_z`, `empty: false`, `class_legend`, and `typed_capture` integer coordinates. `typed_capture` has exactly `kind`/`value`, with kind `"PreparationProjection"`; value has exactly `scene_id`, `view`, `classes`, `primitives`, matching independent literal diagnostic ids/labels/geometry for the rendered layer. It contains no private plan/store/piece/codec/handle field. No diagnostic primitive is labeled InfillIR or assigned extrusion width/tool/volume. | `set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --test preparation_visual_bundle_tdd mixed_plan_and_stage_bundle -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-6. Given** a whole-print scene with geometry/classes on an unrendered layer, **when** all-layer and subset bundles render, **then** full-plan framing and `class_legend` are identical, every selected-layer StageCapture shares the same scene Arc, snapshot retention is counted once rather than once per image, and `executed_preparation` records every actually executed owner separately from `rendered_layer_indices`. An explicit completed empty scene yields `empty: true`, zero mirrored primitives and a successful PNG, unlike missing completion. | `set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --test preparation_visual_bundle_tdd subset_snapshot_and_explicit_empty -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-7. Given** existing stage-only model/G-code requests under `1.0.0`, `1.1.0`, `1.2.0`, `1.3.0`, **when** the updated path renders, **then** legacy PNG/manifest deterministic pins and all original validation results remain unchanged, preparation-only JSON fields are absent, and old version constants/legend behavior are not repinned to `1.4.0`. | `set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --test preparation_visual_bundle_tdd legacy_schema_compatibility -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-8. Given** implemented public projection/visual contracts, **when** normative documentation and real API examples are reviewed, **then** the named sections describe the request schema, typed sink, atomicity, metadata-only opaque-plan boundary, execution-versus-render distinction and accounting exclusions. | `set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --test preparation_visual_bundle_tdd documented_request_shape -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log; rg -q '^## Prepared plan XY views \(schema 1.4.0\)$' docs/19_visual_debug.md; rg -q '^### Preparation diagnostic projection transport \(Normative\)$' docs/03_wit_and_manifest.md; rg -q '^### Optional preparation projection sink \(Normative\)$' docs/05_module_sdk.md; rg -q '^### Preparation visual execution evidence \(Normative\)$' docs/17_agent_debugging.md`

## Negative Test Cases

- **AC-N1. Given** valid staged `probe` bytes plus duplicate/missing scene, undeclared view/class, unknown layer, malformed line/ring/hole, overflowing coordinate predicate or foreign sink access, **when** native/component code catches a projection write error and returns success, **then** errors are respectively `DuplicateScene`, `MissingScene`, `UndeclaredView`, `InvalidReference` (class/layer), `InvalidShape` (line/ring/hole), `InvalidCoordinate`, `AccessDenied`; duplicate/empty ids are `InvalidReference`, diagnostic byte overflow is `ArithmeticOverflow`, repeated writes after poison are `PoisonedOutput`, and unrequested sink publication is `NotRequested`. Finish cannot publish either plan or projection; `current()` is `NotReady` for the failing caller, foreign-owner state is unmodified. Module-returned original error/trap and cancellation also roll both back, preserving producer failure precedence. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --features preparation-test-fixtures --test contract preparation_projection_tdd::projection_rejections_are_atomic -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-N2. Given** malformed preparation selector, schema below `1.4.0`, G-code source, front/side dimension, unknown owner/view, unsupported/inactive owner, invalid layer, missing requested scene or undeclared projection, **when** visual-debug validates/captures, **then** it returns respectively `InvalidPreparationSelector`, `PreparationRequiresSchema14`, `PreparationUnsupportedSource`, `PreparationUnsupportedDimension`, `UnknownPreparationOwner`, `UnknownPreparationView`, `PreparationUnsupportedOwner`/`InactivePreparationOwner`, `NoApplicableLayer`, or `PreparationProjectionRejected` with the typed projection reason; no successful partial bundle is written and an existing sentinel bundle remains byte-identical even with overwrite. Pre-execution rejections have no preparation invocation marker. | `set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --test preparation_visual_bundle_tdd fail_closed_requests_and_bundle -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --test preparation_visual_bundle_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`

## Authoritative Docs and Doc Impact Statement

Authorities: approved `docs/specs/layer-module-preparation-plan.md` (Q17–Q21, projection contract, visual witnesses and queue; bounded slices), `docs/adr/0066-private-layer-preparation-capability.md`, `docs/19_visual_debug.md` (request revisions/framing/closure), `docs/17_agent_debugging.md` (truthful instrumentation), `docs/08_coordinate_system.md`, `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`. No Orca algorithm is ported or parity claimed.

Same-packet doc edits are exactly the four headings/greps in AC-8. Docs and wire examples must use the actual typed API, never opaque codec inspection. Full scope and verification matrix belong to requirements; downstream export shapes belong to design.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
