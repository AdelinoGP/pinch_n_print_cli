# Implementation Plan: layer-range-scope

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-570`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Reconcile packets 03, 05, and 07 before editing; do not guess around a forward-interface mismatch.
- Every cargo/xtask invocation is delegated and tee'd to `target/test-output.log` where required by repository policy.

## Steps

### Step 1: Reconcile forward exports and freeze blast-radius inventories

- Task IDs: `TASK-570`
- Objective: confirm the landed packet-03/05/07 symbols and inventory every exhaustive `ConfigScope`/`ResolutionError` match, `ResolutionTarget` literal, `SliceRunOptions` literal, `prepare_prepass_context` call, and model-source adapter.
- Precondition: packets 03, 05, and 07 are implemented in the working tree.
- Postcondition: bounded inventories name every required edit owner and either confirm the design's shapes or record a packet-local name adaptation that preserves semantics.
- Files allowed to read, with ranges when over 300 lines:
  - predecessor packet `packet.spec.md`/`design.md` files — exports only
  - `crates/slicer-config/**`, `crates/slicer-runtime/src/run.rs`, and exact callers returned by symbol searches — symbol windows only
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds: source plan edits, predecessor edits, Orca source direct reads, implementation edits, binary fixture contents.
- Blast-radius discipline: dispatch separate `LOCATIONS` inventories for enum/error exhaustive matches, target/option struct literals, and runtime call sites; no new field/variant is authored until each inventory is complete.
- Expected sub-agent dispatches:
  - Question: report actual predecessor export shapes/mismatches; scope: predecessor contracts and landed `slicer-config`; return: `FACT: <5 lines or fewer>`.
  - Question: enumerate the three blast-radius families above; scope: `crates/**/*.rs`; return: `LOCATIONS: <at most 20 file:line entries, one context line each>` per family.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — queue row 9 and dependency exports only.
- OrcaSlicer refs: none.
- Verification: inventory review returns no unresolved caller or exhaustive match.
- Exit condition: stop before code if any prerequisite is absent semantically, any inventory is incomplete, or reconciliation would broaden TASK-570.

### Step 2a: Author the canonical single-range fixture

- Task IDs: `TASK-570`
- Objective: create a deterministic one-object 3MF whose range member has exactly AC-1's canonical XML.
- Precondition: delegated canonical XML/object-linkage evidence confirms the exact shape in `requirements.md`.
- Postcondition: the committed archive has the required ordinary model members and exactly one range member/object/range/option.
- Files allowed to read, with ranges when over 300 lines:
  - delegated canonical SNIPPETS — named importer/exporter functions only
  - existing tiny synthetic-3MF test writers — ZIP member construction helpers only
- Files allowed to edit (at most 3):
  - `resources/layer_range_one_range.3mf`
- Files explicitly out of bounds: existing binary fixtures, production Rust, source plan, predecessor packets.
- Blast-radius discipline: not applicable; no Rust struct or schema field is added.
- Expected sub-agent dispatches:
  - Question: build the deterministic fixture from reviewed member text, then inspect only member names and bounded XML; scope: new fixture only; return: `FACT: <5 lines or fewer>`.
- Context cost: `S`
- Authoritative docs:
  - `docs/22_test_quality.md` — fixture must be present and independently asserted.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — delegated exporter XML shape.
- Verification: bounded fixture FACT reports `Metadata/layer_config_ranges.xml`, one object ordinal, `[0.4,0.8)`, and `layer_height` text `0.1`.
- Exit condition: archive member shape differs, fixture construction is nondeterministic, or an existing fixture was modified.

### Step 2b: Implement the model-IO parser

- Task IDs: `TASK-570`
- Objective: create the one-range fixture and parse the exact optional XML part into raw ordinal/range/string records with atomic structural errors.
- Precondition: Step 2a fixture exists and delegated canonical XML/object-linkage evidence confirms the exact shape in `requirements.md`.
- Postcondition: AC-1 and AC-N3's model-IO portions pass; missing part is empty success and invalid/malformed XML never yields partial records.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-model-io/src/sidecar.rs` — ZIP/XML helper/error-style symbols only
  - delegated canonical SNIPPETS — named importer/exporter functions only
- Files allowed to edit (at most 3):
  - `crates/slicer-model-io/src/layer_config_ranges.rs`
  - `crates/slicer-model-io/src/lib.rs`
  - `crates/slicer-model-io/tests/layer_config_ranges_tdd.rs`
- Files explicitly out of bounds: loader geometry parsing, slicer-config/runtime, existing binary fixtures, source plan.
- Blast-radius discipline: net-new structs/errors have no prior literals; tests that assert every field use `// exhaustive: wire-shape contract` if FRU would weaken the claim.
- Expected sub-agent dispatches:
  - Question: inspect canonical XML shape and ordinal mapping; scope: named `bbs_3mf.cpp` functions; return: `SNIPPETS` ≤3×30 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — authored Z millimetres.
  - `docs/22_test_quality.md` — fixture must not silently skip.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — delegate named importer/exporter functions.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test layer_config_ranges_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT pass/fail.
- Exit condition: exact AC-1 record/ordinal mapping passes, each invalid class returns a named error with no partial output, and missing member returns an empty vector.

### Step 3: Add and validate the typed layer-range scope

- Task IDs: `TASK-570`
- Objective: extend packet-03 ingestion with `ConfigScope::LayerRange`, interval carriers, registry typing/admission, deterministic indices, and non-height conflict detection.
- Precondition: Step 1 inventories are complete and Step 2 produces raw records.
- Postcondition: AC-2 and AC-N1/AC-N2/AC-N4 pass; all ranges are validated before immutable `ScopedConfig` exposure.
- Files allowed to read, with ranges when over 300 lines:
  - landed packet-03 ingestion symbols and packet-07 admission/error symbols only
  - Step-1 exhaustive-match inventory
- Files allowed to edit (at most 3):
  - `crates/slicer-config/src/ingestion.rs`
  - `crates/slicer-config/src/lib.rs`
  - `crates/slicer-config/tests/layer_range_scope_tdd.rs`
- Files explicitly out of bounds: resolver/profile code, runtime, model-IO parser, manifests, IR/WIT.
- Blast-radius discipline: the enum-variant inventory is owned here; split any external exhaustive-match fallout into adjacent ≤3-file substeps before continuing. If `ScopedConfig` gains a field, every inventoried literal is included here or converted with justified FRU.
- Expected sub-agent dispatches:
  - Question: verify no exhaustive match/literal was missed after the variant/carrier change; scope: Step-1 locations plus compiler command; return: `FACT: <5 lines or fewer>` or first error `SNIPPETS` ≤20 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` — typed scope boundary.
  - `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — atomic denial.
- OrcaSlicer refs: none; policy validation is owner-defined.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd world_z_half_open_range_is_typed_and_indexed_per_object -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test world_z_half_open_range_is_typed_and_indexed_per_object .* ok" target/test-output.log'` — FACT pass/fail.
  - Run AC-N1, AC-N2, and AC-N4 commands — FACT pass/fail.
- Exit condition: values are registry-typed, bounds/ordinal mapping are valid, denial/conflict failures are atomic, and no hard-coded denial roster exists.

### Step 4: Compose canonical Z profiles and fill both resolver seams

- Task IDs: `TASK-570`
- Objective: implement earlier-starting `layer_height` trim/gap composition in `query_z_grid` and top-Z range precedence/catch-up inheritance in `resolve_scope_stack`.
- Precondition: Step 3 yields normalized validated typed ranges and delegated canonical overlap evidence reconfirms the approved correction.
- Postcondition: AC-3 and AC-4 pass with independently authored expected segments/values; both resolver queries consume the same range collection.
- Files allowed to read, with ranges when over 300 lines:
  - landed packet-05 resolution module — `query_z_grid`, `resolve_scope_stack`, `ResolutionTarget` only
  - delegated `layer_height_profile_from_ranges` summary
- Files allowed to edit (at most 3):
  - `crates/slicer-config/src/resolution.rs`
  - `crates/slicer-config/tests/layer_range_scope_tdd.rs`
  - one landed resolver helper file only if Step 1 proves the module is split
- Files explicitly out of bounds: runtime callers, XML parser, WIT/IR, predecessor docs.
- Blast-radius discipline: `ResolutionTarget` field/literal fallout from Step 1 is owned here; if more than two additional files are needed, split literal updates into deterministic ≤3-file substeps before running the focused suite.
- Expected sub-agent dispatches:
  - Question: reconfirm canonical earlier-wins trim/fixed-first/gap behavior; scope: `Slicing.cpp::layer_height_profile_from_ranges`; return: `SUMMARY` ≤200 words.
  - Question: run focused profile/resolver tests; scope: command below; return: `FACT: <5 lines or fewer>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — precedence/catch-up/gap requirements, subject to the recorded overlap correction.
  - `docs/22_test_quality.md` — literal independent expectations.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — delegate `layer_height_profile_from_ranges`; cite function, never lines.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT pass/fail.
- Exit condition: the overlap negative control fails under later-wins replacement, gaps use base height, upper endpoints are excluded, catch-up uses top Z, and precedence is exactly object < range < modifier.

### Step 5: Carry one typed range set through both production paths

- Task IDs: `TASK-570`
- Objective: parse/map ranges in the model-source adapters and carry them explicitly through ordinary slicing and visual-debug prepass without duplicated semantics.
- Precondition: Step 4 resolver APIs are green and Step 1 caller/literal inventories are complete.
- Postcondition: AC-5 passes and both runtime paths receive byte/value-equivalent typed ranges from their adapters.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/main.rs` — slice model/config setup only
  - `crates/pnp-cli/src/visual_debug.rs` — `run_visual_debug` model/config/prepass setup only
  - `crates/slicer-runtime/src/run.rs` — `SliceRunOptions`, `run_slice_with_collector`, `prepare_prepass_context` only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/main.rs`
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/slicer-runtime/src/run.rs`
- Files explicitly out of bounds: resolver semantics, mesh IR fields, CLI arguments, support-preview behavior, unrelated call sites.
- Blast-radius discipline: every Step-1 `SliceRunOptions` literal and `prepare_prepass_context` call must compile. Apply updates in subsequent ≤3-file substeps named from the inventory; do not add a default that silently drops model-authored ranges on a production path.
- Expected sub-agent dispatches:
  - Question: verify all carrier callers/literals were updated and no XML parser call appears in runtime; scope: Step-1 location inventory; return: `FACT: <5 lines or fewer>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — layer-planning/region-mapping ownership.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p pnp_cli -p slicer-runtime --all-targets` — FACT pass/fail.
- Exit condition: both adapters parse once, runtime only carries typed data, all callers compile, and neither production path can silently substitute an empty range set for the fixture.

### Step 6: Add runtime convergence and visual-debug regressions

- Task IDs: `TASK-570`
- Objective: prove the full fixture reaches both setup paths and produces an observable nonuniform schedule in a real visual-debug bundle.
- Precondition: Step 5 compiles and guest freshness check exits 0 before diagnosing any integration failure.
- Postcondition: AC-5 and AC-6 pass with registered, non-vacuous tests and `manifest.json` assertions.
- Files allowed to read, with ranges when over 300 lines:
  - runtime integration `main.rs` — module registrations only
  - existing visual-debug request/silhouette helpers — fixture and manifest helper symbols only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/layer_range_scope_tdd.rs`
  - `crates/slicer-runtime/tests/integration/main.rs`
  - `crates/pnp-cli/tests/layer_range_scope_visual_debug_tdd.rs`
- Files explicitly out of bounds: production code, existing fixtures/tests, snapshot baselines, guest artifacts.
- Blast-radius discipline: test structs with ≥5 public fields use FRU or a contract-naming exhaustive waiver; no assertion derives expected Zs from production profile output.
- Expected sub-agent dispatches:
  - Question: run guest freshness then each focused test; scope: exact commands; return: `FACT: <5 lines or fewer>` or failure `SNIPPETS` ≤20 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/19_visual_debug.md` — model source, silhouette request, and manifest contract.
  - `docs/22_test_quality.md` — non-vacuous output and independent oracle.
- OrcaSlicer refs: none.
- Verification:
  - Run AC-5 and AC-6 commands — FACT pass/fail.
- Exit condition: runtime test proves equivalent resolved data, visual-debug writes a valid non-empty bundle, and a uniform-grid implementation fails at least one literal schedule assertion.

### Step 7: Update contracts and run closure gates

- Task IDs: `TASK-570`
- Objective: document the corrected canonical semantics and shared resolver ownership, then run every packet/quality gate.
- Precondition: Steps 1–6 pass.
- Postcondition: AC-7 and all packet verification commands pass; no unrelated files or version constants changed.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` — Config Key Namespaces/precedence only
  - `docs/04_host_scheduler.md` — config resolution/layer planning/region mapping only
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- Files explicitly out of bounds: source plan, predecessor packets, ADRs, deviation log, unrelated docs/code.
- Blast-radius discipline: not applicable; no struct/schema field is added in this step.
- Expected sub-agent dispatches:
  - Question: run AC-7, all-target check/clippy, literals, test-quality report, focused suites, and freshness; scope: each exact command; return: `FACT: <5 lines or fewer>` each.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — final scope, with requirements' approved overlap correction.
  - `docs/08_coordinate_system.md` — Z units.
- OrcaSlicer refs:
  - Reuse the bounded delegated evidence from Steps 2 and 4; do not re-open canonical source.
- Verification:
  - AC-7 command; every command in `requirements.md` §Verification Commands.
- Exit condition: docs say earlier-starting trim (not later-starting replacement), every gate passes, and diff scope is limited to design-owned paths.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | forward reconciliation and blast-radius inventories |
| Step 2a | S | deterministic canonical fixture |
| Step 2b | M | XML parser and parser failures |
| Step 3 | M | enum/carrier and atomic validation |
| Step 4 | M | canonical profile and both resolver seams |
| Step 5 | M | runtime carrier/caller blast radius |
| Step 6 | M | two production-path tests and visual gate |
| Step 7 | S | docs and delegated closure gates |

## Packet Completion Gate

- All steps and exits complete; every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` exits 0 before interpreting integration/visual-debug failures.
- `cargo check --workspace --all-targets`, clippy, literals, and touched-file test-quality review pass.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Confirm the plan-amendment note remains in requirements, while the source plan and packets 01–08 remain untouched.
- `packet.spec.md` is ready for `status: implemented` only after forward dependencies reconcile and all gates pass.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Re-inspect only the new fixture's member list/XML through a bounded worker return.
- Record remaining packet-local risk and confirm no WIT/IR/manifest/schema version changed.
- Confirm context stayed at or below 150k standard, or record the required swarm escalation/lesson.
