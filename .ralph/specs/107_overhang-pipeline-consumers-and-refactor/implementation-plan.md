# Implementation Plan: 107_overhang-pipeline-consumers-and-refactor

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first (write the failing test before the production change), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.

## Steps

### Step 1: O-T030/O-T031/O-T032 — View accessors + WIT + populator

- Task IDs:
  - `O-T030` — Confirm `overhang_areas()` (P104 stub) now returns non-empty post-P106
  - `O-T031` — Add `SliceRegionView::overhang_quartile_polygons() -> &[QuartileBand]`
  - `O-T032` — Decide on `PaintRegionLayerView` / `SurfaceClassificationView` mirror (default: skip unless consumer named)
- Objective: add the new view accessor, mirror in WIT, fill in host populator; write contract TDD that asserts P104's `overhang_areas()` stub now returns non-empty.
- Precondition: P106 is `status: implemented`; workspace builds clean.
- Postcondition: AC-1 + AC-2 verification commands pass; `cargo xtask build-guests --check` no STALE.
- Files allowed to read:
  - `crates/slicer-sdk/src/views.rs` — range-read by `rg -n 'fn (bridge_areas|overhang_areas|surface_group)'`.
  - `crates/slicer-wasm-host/src/host.rs` — range-read by `rg -n 'sliced_region_to_data|bridge_areas'`.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — full.
- Files allowed to edit (≤ 3 per sub-step):
  - 1a (SDK + WIT): `crates/slicer-sdk/src/views.rs`, `crates/slicer-schema/wit/deps/ir-types.wit`.
  - 1b (host populator): `crates/slicer-wasm-host/src/host.rs`.
  - 1c (contract test): `crates/slicer-runtime/tests/contract/slice_region_view_overhang_areas_non_empty_tdd.rs` (NEW).
- Files explicitly out-of-bounds:
  - Module source (Step 2).
  - IR (P106 owns; no IR change here).
- Expected sub-agent dispatches:
  - "Find the `bridge_areas` populator pattern in `crates/slicer-wasm-host/src/host.rs`; return SNIPPETS ≤ 30 lines."
  - "FACT: confirm `QuartileBand` shape from P106 — return field list."
  - "Run `cargo build --tests --workspace`; FACT pass/fail."
  - "Run `cargo xtask build-guests --check`; FACT (clean / STALE list)."
- Context cost: `M`
- Authoritative docs: `docs/05_module_sdk.md` (delegate SUMMARY), `docs/03_wit_and_manifest.md` §"WIT/Type Changes Checklist".
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'pub fn overhang_quartile_polygons\(&self\) -> &\[QuartileBand\]' crates/slicer-sdk/src/views.rs` — exit 0.
  - `cargo test -p slicer-runtime --test contract slice_region_view_overhang_areas_non_empty_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-1 + AC-2 green; no STALE guests.

### Step 2: O-T040/O-T041/O-T042 — Refactor overhang-classifier-default

- Task IDs:
  - `O-T040` — Refactor to read from `Point3WithWidth.overhang_quartile`; apply speed factors only
  - `O-T041` — Delete `classify.rs` + `lines_distancer.rs`
  - `O-T042` — Update manifest: drop broad reads; declare narrow `overhang_quartile` read
- Objective: rewrite the module as a pure consumer; delete the auxiliary files; narrow the manifest.
- Precondition: Step 1 exit condition met.
- Postcondition: AC-3 + AC-4 verification commands pass.
- Files allowed to read:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` — full (≤ 200 LOC).
  - `modules/core-modules/overhang-classifier-default/src/classify.rs` — read once to confirm deletion scope.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` (rewrite)
  - `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml` (narrow manifest)
  - (delete two files via `git rm` — counts as one mechanical operation)
- Files explicitly out-of-bounds:
  - All other modules / crates.
- Expected sub-agent dispatches:
  - "FACT: confirm no other crate imports from `overhang_classifier_default::classify` or `::lines_distancer`; return LOCATIONS ≤ 5 entries (expected zero)."
  - "FACT: signature of `LayerCollectionView::ordered_entities`; return single-line."
  - "Run `cargo check -p overhang-classifier-default`; FACT pass/fail."
- Context cost: `M`
- Authoritative docs: `docs/adr/0008-overhang-as-finalization-module.md` (speed-factor application stays here).
- OrcaSlicer refs: none.
- Verification:
  - `! ls modules/core-modules/overhang-classifier-default/src/classify.rs 2>/dev/null` — exit 0 (file absent).
  - `! ls modules/core-modules/overhang-classifier-default/src/lines_distancer.rs 2>/dev/null` — exit 0.
  - `[ $(wc -l < modules/core-modules/overhang-classifier-default/src/lib.rs) -le 80 ]` — exit 0 (LOC bound).
  - `! rg -q 'path_geometry\|LayerCollectionIR' modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml` — exit 0.
- Exit condition: AC-3 + AC-4 green; module shrunk; auxiliary files deleted.

### Step 3: O-T050 — End-to-end overhang propagation TDD

- Task IDs:
  - `O-T050` — Reference fixture for end-to-end overhang quartile propagation
- Objective: implement an integration test that slices an overhang-ramp mesh through the full stack and asserts wall vertices carry `overhang_quartile = Some(N)` in overhang region + finalization applies the speed factor. Includes AC-N1 no-overhang case.
- Precondition: Step 2 exit condition met.
- Postcondition: AC-5 + AC-N1 pass.
- Files allowed to read:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` (post-refactor).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs` (NEW; both AC-5 positive + AC-N1 negative)
  - `crates/slicer-runtime/tests/fixtures/overhang_ramp.stl` (NEW; or analogous synthetic fixture; see open question)
- Files explicitly out-of-bounds: any source.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test integration overhang_pipeline_e2e_tdd`; FACT pass/fail per case."
- Context cost: `M`
- Authoritative docs: `docs/specs/overhang-pipeline-restructuring.md` Phase 5 row.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test integration overhang_pipeline_e2e_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-5 + AC-N1 green. If AC-5 hits the "P104 still ships None" branch, the implementer registers the follow-up task and the test asserts the partial-state behaviour (overhang region detected; quartile = None; classifier applies no speed factor; manual follow-up needed).

### Step 4: O-T051 — Pre-vs-post-refactor regression check

- Task IDs:
  - `O-T051` — Regression coverage: pre-refactor vs post-refactor G-code on benchy / standard fixtures
- Objective: capture reference G-code SHA (or per-entity speed-factor values) for one standard fixture using the pre-refactor module (recorded BEFORE Step 2 if not already recorded; recorded in Step 4 if pre-refactor is no longer available in HEAD); compare against post-refactor output within calibrated tolerance.
- Precondition: Step 3 exit condition met.
- Postcondition: AC-6 passes.
- Files allowed to read:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` (post-refactor) — confirm behaviour.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/fixtures/overhang_classifier_baseline_speeds.json` (NEW; recorded baseline)
- Files explicitly out-of-bounds: any source.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test integration overhang_classifier_refactor_regression_tdd`; FACT pass/fail with tolerance-deviation summary on fail."
- Context cost: `M`
- Authoritative docs: `docs/specs/overhang-pipeline-restructuring.md` Phase 5.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test integration overhang_classifier_refactor_regression_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-6 green; speed factors within calibrated tolerance vs recorded baseline.

### Step 5: O-T053 — Deviation closure + roadmap unblock markers

- Task IDs:
  - `O-T053` — Close D-10, D-12, D-OVERHANG-QUARTILE-NONE; mark T-024 / T-077 unblocked
- Objective: walk the three deviation entries and add closure notes referencing this packet + P106; update the perimeter roadmap to mark T-024 and T-077 as unblocked (preconditions met).
- Precondition: Step 4 exit condition met.
- Postcondition: AC-7 passes; all Doc Impact Statement greps pass.
- Files allowed to read:
  - `docs/DEVIATION_LOG.md` — range-read the three target entries.
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — range-read T-024 + T-077 rows.
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Files explicitly out-of-bounds: source files.
- Expected sub-agent dispatches:
  - "For each Doc Impact grep, run `rg -q`; FACT pass/fail per grep."
- Context cost: `S`
- Authoritative docs: the two files being edited.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'D-10.*closed\|D-10.*resolved' docs/DEVIATION_LOG.md` — exit 0.
  - `rg -q 'OVERHANG-QUARTILE-NONE.*closed\|OVERHANG-QUARTILE-NONE.*resolved' docs/DEVIATION_LOG.md` — exit 0.
  - `rg -q 'T-024.*unblocked\|T-077.*unblocked' docs/specs/perimeter-modules-orca-parity-roadmap.md` — exit 0.
- Exit condition: AC-7 green; deviation log + roadmap markers updated.

### Step 6: O-T052 — Architecture doc updates

- Task IDs:
  - `O-T052` — Update `docs/01_system_architecture.md` Tier 3 block + `docs/02_ir_schemas.md` (consumer notes); update `docs/05_module_sdk.md` accessor convention.
- Objective: land the three architecture doc updates per Doc Impact Statement.
- Precondition: Step 5 exit condition met.
- Postcondition: remaining Doc Impact Statement greps pass.
- Files allowed to read:
  - `docs/01_system_architecture.md` — range-read §"Tier 3 PostPass".
  - `docs/02_ir_schemas.md` — range-read SurfaceClassificationIR section.
  - `docs/05_module_sdk.md` — range-read §"SliceRegionView accessors".
- Files allowed to edit (≤ 3):
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
  - `docs/05_module_sdk.md`
- Files explicitly out-of-bounds: source files.
- Expected sub-agent dispatches:
  - "For each remaining Doc Impact grep, run `rg -q`; FACT pass/fail per grep."
- Context cost: `S`
- Authoritative docs: the three files being edited.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'overhang-classifier-default.*reads.*overhang_quartile' docs/01_system_architecture.md` — exit 0.
  - `rg -q 'overhang_quartile_polygons.*QuartileBand' docs/05_module_sdk.md` — exit 0.
- Exit condition: all Doc Impact Statement greps pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | SDK + WIT + populator + new contract TDD. |
| Step 2 | M | Module refactor + 2 deletions + manifest narrowing. |
| Step 3 | M | End-to-end TDD + fixture. |
| Step 4 | M | Regression TDD + recorded baseline. |
| Step 5 | S | Three deviation closure entries + roadmap markers. |
| Step 6 | S | Three architecture doc edits. |

Aggregate context cost: `M`. No step `L`. Per-step file edit count ≤ 3.

## Packet Completion Gate

- All six steps complete; each exit condition met.
- AC-1 through AC-7 + AC-N1 all PASS via worker dispatch.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` updated for each O-T030..O-T053 entry — via worker dispatch.
- `packet.spec.md` ready to move `draft` → `implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm gate commands green.
- If AC-5 hit the "P104 still ships None" branch, confirm the follow-up task is registered in the perimeter roadmap.
- Record the AC-6 tolerance-deviation summary in the closure log (speed factors shifted by X% on average; Y entities outside tolerance).
- Confirm implementer's peak context usage < 70%.
