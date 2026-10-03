---
status: draft
packet: layer-module-preparation_02_preparation-capability-pilot
task_ids:
  - TASK-573
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: preparation-capability-pilot

## Goal

Build an executable same-artifact, native/WASM preparation pilot using real imported host resources, immutable named pieces, fresh ordinary Layer calls, and measured transport accounting, without activating preparation in the slicing pipeline.

## Scope Boundaries

This is approved queue row 02, independently runnable using controlled input, not the routing projection. It owns the executable proof and additive authoring/binding seams; production declaration validation and normalized storage belong to row 03, whole-print projections to row 04, activation to row 05, and visual projections to row 06. No lightning algorithm or parity claim is made.

## Prerequisites and Blockers

- Depends on: none; TASK-572 is not a pilot prerequisite.
- Unblocks: row 03 / TASK-574 only after executable acceptance and independent review.
- Activation prerequisites: independent preflight must pass before activation. If the agreed composition cannot execute, stop dependent implementation for a design decision; do not substitute companion artifacts, exported guest-owned resources, retained instances, import-free stubs, or algorithm-specific native objects.
- Authoring state: draft only; no implementation or verification execution has occurred.

## Forward Export Summary — planned NET-NEW, subject to the pilot gate

These are concrete producer contracts for row 03, not shipped evidence. Later packets must say FORWARD-DEP until this pilot actually passes.

- `slicer:layer-preparation@1.0.0`, canonical new `crates/slicer-schema/wit/deps/layer-preparation/layer-preparation.wit`: imported `preparation-types` owns `print-preparation-input`, `prepared-plan-output`, `prepared-plan-view`, `prepared-piece` and `preparation-error`; exported `prepare.prepare-print(input: print-preparation-input, output: prepared-plan-output) -> result<_, slicer:common/module-errors.module-error>`. The resources are host-owned because their defining interface is imported, never exported.
- `print-preparation-input.read-fixture(offset: u64, max-bytes: u64) -> result<list<u8>, preparation-error>` is explicitly pilot-only. `prepared-plan-output.put(name: string, bytes: list<u8>) -> result<_, preparation-error>` publishes a complete unique piece into staging. `plan-access.current() -> result<prepared-plan-view, preparation-error>`, `prepared-plan-view.open(name: string) -> result<option<prepared-piece>, preparation-error>`, `prepared-piece.len() -> u64`, and `prepared-piece.read(offset: u64, max-bytes: u64) -> result<list<u8>, preparation-error>` provide implicit-owner, bounded reads.
- New composed worlds `prepared-<ordinary-world>` include each schema-derived ordinary Layer world unchanged, import `preparation-types` and `plan-access`, and export `prepare`. Existing stage signatures, shared imported interfaces, and plain worlds do not change. Preparation is not a scheduled stage.
- `slicer_sdk::preparation::LayerPreparation`: required associated `fn prepare_print(input: &PrintPreparationInput, output: &mut PreparedPlanOutput) -> Result<(), ModuleError>`; no default body and no print-wide constructor config. New wrappers `PrintPreparationInput`, `PreparedPlanOutput`, `PreparedPlanView`, `PreparedPiece` expose the operations above. `PreparedPlanView::current()` obtains a fresh call-local read facade; ordinary `LayerModule` signatures stay unchanged.
- New `#[slicer_module(preparation)]` on the single ordinary `impl LayerModule for T`, paired with `impl LayerPreparation for T`, emits both real guest exports, requires the trait method at compile time, and preserves ordinary `from_config` reconstruction. Plain `#[slicer_module]` remains plain. Generated `T::__slicer_prepared_native_entry() -> NativePreparedStageEntry` and separate `T::__SLICER_PREPARATION_SCHEMA: PreparationExportSchema` do not widen pre-existing structs.
- `slicer_sdk::native::NativePreparedStageEntry { stage: NativeStageEntry, prepare: fn(&PrintPreparationInput, &mut PreparedPlanOutput) -> Result<(), ModuleError> }`; SDK `with_prepared_plan` installs/removes a native call-local serialized read facade around its ordinary stage pointer. `PreparationExportSchema` is a new `slicer-schema` sidecar containing the qualified preparation export, not another scheduled stage.
- Host binding resource backings `PreparationInputData`, `PreparationOutputData`, `PreparedPlanViewData`, `PreparedPieceData` live in the new pilot contract harness; row 03 must promote/normalize them, not assume production storage or declaration parsing already exists.

## Acceptance Criteria

All test symbols below are NET-NEW tests in the registered `preparation_capability_pilot_tdd` module of the existing `slicer-wasm-host --test contract` target. Its actual typed driver and fixtures are authored here; no `run_slice` claim is made. Each command rejects zero executed tests and preserves combined output.

- **AC-1. Given** an Infill guest authored with `#[slicer_module(preparation)]` and real ordinary config/IR imports, **when** typed preparation executes on controlled bytes `[3, 5, 11, 17]`, its store is dropped, and fresh Layer stores read `numbers` at `(0,2)` and `(2,2)`, **then** the actual infill builders contain independent expected two-point paths with endpoint X values `(3,5)` and `(11,17)` mm, respectively, and no preparation instance or table index survives. | `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract preparation_pilot_fresh_wasm -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))" && rg -q '^## Preparation capability pilot composition' docs/03_wit_and_manifest.md`
- **AC-2. Given** the same fixture sources compiled natively, **when** generated native preparation is destroyed and each ordinary native stage is reconstructed through `from_config`, **then** the same range requests yield the independently specified path coordinates, with only serialized piece bytes retained, and native/WASM outputs satisfy those expectations separately. | `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract preparation_pilot_native -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))" && rg -q '^### Preparation capability pilot authoring' docs/05_module_sdk.md`
- **AC-3. Given** module-backed Layer rows derived from `slicer_schema::STAGES` by `tier_id == TIER_LAYER` and nonempty `wit_package`, **when** the compile driver builds plain/prepared macro shapes natively and as real guest components for every derived row, **then** the observed set equals the derived set and opt-in without `LayerPreparation::prepare_print` fails compilation; additionally a prepared PathOptimization guest with input `b'alpha|beta'` reads `(0,5)` and `(6,4)` in fresh calls and emits exactly `alpha` and `beta` comments through the actual G-code builder. Compile coverage alone is not execution coverage. | `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract preparation_pilot_stage_surface -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- **AC-4. Given** the real pre-existing `classic-perimeters.wasm` archived with hash before any pilot SDK/macro/WIT edits, **when** the pilot's host registers preparation imports alongside the ordinary linker and instantiates that unchanged archived plain artifact, **then** typed ordinary load succeeds without a preparation export or parameter; its hash equals the pre-edit hash, and capability addition alone produces no stage-package drift. Rebuilding an ordinary guest against the new SDK is not this compatibility witness. | `set -o pipefail; mkdir -p target; cargo xtask build-guests --check && cargo test -p slicer-wasm-host --test contract preparation_pilot_plain_guest -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- **AC-5. Given** successful unique puts of `numbers` and UTF-8 name `π` plus ready-empty preparation, **when** actual native/guest operations and instrumented representative preparation/read calls run, **then** recorded `payload_bytes` and `name_utf8_bytes` equal independently summed fixture byte lengths, each transfer records direction/bytes/count, and `prepare_duration_ns`/`read_duration_ns` contain observed durations; metadata is restricted to preparation lifecycle/phase and ready state, owner, piece names/lengths, retained payload/name accounting and transfer sizes/counts, plus measured timings when instrumented, and excludes all raw opaque payloads. The evidence labels allocator/process overhead and process peak memory unmeasured, not zero, and states no latency acceptance threshold. | `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract preparation_pilot_accounting -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))" && rg -q '^## Executable pilot evidence' docs/adr/0066-private-layer-preparation-capability.md`

## Negative Test Cases

- **AC-N1. Given** owners A/B publishing different `numbers` values and a separate print, **when** guest/native consumers request current plan and optional/required pieces, **then** B never observes A, cross-print access fails `not-ready`, explicit ready-empty opens missing pieces as `None`, absent ready state returns `not-ready`, absent optional piece is accepted by the module, and absent required `numbers` preserves module failure `required piece numbers missing` with no success output. | `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract preparation_pilot_isolation -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- **AC-N2. Given** duplicate `numbers`, empty names, caught duplicate errors, guest traps, and a module failure `pilot preparation failed`, **when** preparation returns or unwinds, **then** errors `duplicate-name`/`invalid-name` poison staging, caught errors followed by success still publish nothing with `poisoned-output`, trap/failure preserve owner and preparation phase plus original details, and no Layer consumer executes. | `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract preparation_pilot_publication_failures -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- **AC-N3. Given** a four-byte piece and controlled input read permission, **when** native and real guest imports receive `(4,8)`, `(5,1)`, `(3,8)`, `(0,0)`, `(u64::MAX,1)`, checked accounting overflow, or denied fixture reads, **then** results are empty, `range-error`, the one remaining byte, empty, `range-error`, `arithmetic-overflow`, and `access-denied`, respectively, with no unchecked conversion/allocation and no payload in diagnostic metadata; malformed required codec bytes fail the module rather than becoming empty output. | `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract preparation_pilot_ranges -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- **AC-N4. Given** a plain artifact requested as prepared or a capability with incompatible preparation signature, **when** typed combined-world instantiation runs and artifact verification compares it, **then** it fails naming `slicer:layer-preparation/prepare@1.0.0#prepare-print`; ordinary stage resolution remains singular, and missing/mutated capability exports are rejected rather than ignored as arbitrary shared packages. | `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract preparation_pilot_export_mismatch -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `set -o pipefail; mkdir -p target; cargo xtask build-guests --check && cargo test -p slicer-wasm-host --test contract preparation_pilot_ -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`

## Authoritative Docs

Direct bounded grounding: `docs/00_project_overview.md` normative map; `docs/specs/layer-module-preparation-plan.md` entire source in separate ranges; ADR-0066; ADR-0045 decision; ADR-0056 decision; `docs/01_system_architecture.md` Tier 2 and ownership; `docs/03_wit_and_manifest.md` host boundary, isolation and package compatibility; `docs/05_module_sdk.md` single-stage constraint and module state lifecycle; `docs/08_coordinate_system.md`; `docs/21_data_defaults_and_fixtures.md`; `docs/22_test_quality.md`.

## Doc Impact Statement

Implementation must add these sections, explicitly labeled pilot evidence, not production availability; no document edits are performed during authoring:

- `docs/03_wit_and_manifest.md`, `Preparation capability pilot composition`: `rg -q '^## Preparation capability pilot composition' docs/03_wit_and_manifest.md` (AC-1).
- `docs/05_module_sdk.md`, `Preparation capability pilot authoring`: `rg -q '^### Preparation capability pilot authoring' docs/05_module_sdk.md` (AC-2).
- ADR-0066, `Executable pilot evidence`: `rg -q '^## Executable pilot evidence' docs/adr/0066-private-layer-preparation-capability.md` (AC-5). Preserve its accepted decision; evidence records commands/results and measured limits, not an unexecuted promise.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
