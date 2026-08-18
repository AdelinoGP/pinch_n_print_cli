---
status: draft
packet: 224-support-family-orca-closure
task_ids:
  - TASK-335
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Depends on draft tree-support-family, traditional-support-family, and mixed-support-family-routing; TASK-331 blockers resolved (packet 220 implemented).
---

# Packet Contract: support-family-orca-closure

## Goal
Close the support-family sequence with fixture-driven invariants and inspected visual/differential evidence showing tree and traditional support reach valid termination surfaces without model collision and retain family roles through final G-code.

## Scope Boundaries
This packet owns closure tests against the decisive fixtures, visual-debug manifests/PNGs, final G-code role checks, and packet 213/TASK-329 plus TASK-163b-orca-ref disposition evidence. It consumes all prior anchored, analysis, tree, traditional, and routing contracts and does not alter their algorithms.

## Prerequisites and Blockers
- Depends on: draft `tree-support-family` (TASK-332), draft `traditional-support-family` (TASK-333), and draft `mixed-support-family-routing` (TASK-334). TASK-331 (packet 220) is implemented (2026-08-13).
- Unblocks: closure of the support-family remediation sequence.
- Activation blockers: [RESOLVED] TASK-331 exact-Z seam ownership and breaking-versus-additive WIT migration (packet 220 implemented 2026-08-13; see design.md). The decisive SupportTest model and both Orca reference G-code fixtures exist; closure runs against them. Status remains draft.

## Acceptance Criteria
- **AC-1. Given** `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` and support enabled for each built-in family, **when** the real slice fixture test runs, **then** every accepted demand has an attributed body connected to plate/model termination, every body/nozzle sweep is disjoint from exact-Z occupancy within modeled tolerance, and no support-disabled output exists. | `cargo test -p slicer-runtime --test integration support_family_closure -- fixture_invariants -- --exact`
- **AC-2. Given** tree and traditional outputs for matched physical heights from `tmp/SupportTest.stl`, **when** `pnp_cli visual-debug` renders the dual-family request `tmp/visual-debug-support-family.json`, **then** the inspected PNGs and manifest cover host `PrePass::SupportAnalysis` candidates, occupancy, envelope, and routing cells; aggregated family `SupportPlanIR` body/interface polygons and skeletons; each anchored `Layer::Support` event with structured `SupportIR`; final PNP G-code; and standalone `tmp/SupportTest_Tree_Orca.gcode` and `tmp/SupportTest_Normal_Orca.gcode`, with tree and traditional views inspected at the same physical heights. Exact taps/layers are `PrePass::SupportAnalysis` at analysis layer heights, `PrePass::SupportGeometry` at support-plan layer heights, and `Layer::Support` at each anchored support event layer. | `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family.json --output target/vd-support-family-tree --overwrite && cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family.json --output target/vd-support-family-normal --overwrite && cargo test -p slicer-runtime --test integration support_family_closure -- matched_height_evidence -- --exact`
- **AC-3. Given** PNP and standalone Orca tree/normal references at matched heights, **when** differential review runs, **then** the inspected evidence records source, layer, tap, and disposition for both families, with behavioral parity claims limited to termination, coverage, collision freedom, interfaces, and independent heights rather than exact path identity. | `cargo test -p slicer-runtime --test integration support_family_closure -- differential_evidence -- --exact`
- **AC-4. Given** final PNP G-code for both family selections, **when** role inspection runs, **then** support and interface output contains the exact markers `;TYPE:Support` and `;TYPE:Support interface`, and family attribution remains present in the closure evidence manifest. | `cargo test -p slicer-runtime --test integration support_family_closure -- final_gcode_roles -- --exact`
- **AC-5. Given** packet 213's degenerate-disk result and reopened `TASK-329`, **when** closure evidence is reviewed, **then** the packet records them as superseded by this fixture-backed closure and does not count the degenerate-disk result as evidence. | `cargo test -p slicer-runtime --test integration support_family_closure -- supersedes_packet_213_and_task_329 -- --exact`
- **AC-6. Given** `TASK-163b-orca-ref`, **when** the authoritative Orca fixtures are reviewed, **then** closure evidence either closes that task with the existing authoritative tree/normal references or records a precise external blocker; it never claims exact path parity. | `cargo test -p slicer-runtime --test integration support_family_closure -- task_163b_disposition -- --exact`

## Negative Test Cases
- **AC-N1. Given** a fixture body entering exact-Z model occupancy, lacking a valid termination, or having cross-family overlap, **when** closure validation runs, **then** the body is dropped, its demand is marked unmet with a structured diagnostic, and the test fails rather than accepting a golden or fallback path. | `cargo test -p slicer-runtime --test integration support_family_closure -- invalid_geometry_fails -- --exact`
- **AC-N2. Given** the decisive fixture is absent from its tracked path, **when** any closure test runs, **then** the shared `support_test_path` resolver panics naming the exact tracked path, so every closure test fails loudly rather than silently degrading; the tracked fixture remains the primary closure path and parity is never claimed from PNG existence alone. | covered by the `support_test_path` resolver contract exercised by every test in `cargo test -p slicer-runtime --test integration support_family_closure -- --exact`

  *Amended 2026-08-17 (packet 224 remediation).* The original AC-N2 mandated a dedicated `missing_fixture_is_blocking` test against "a deliberately missing copied fixture path". As implemented, that test asserted only that `std::fs::read` returns `NotFound` for a path the test itself constructed to not exist — it exercised `std::fs`, not the closure gate, and was a placeholder. The fixture is now tracked in-repo, and `support_test_path` already panics with the tracked path when it is absent, which fails all seven remaining closure tests loudly. The dedicated test is deleted as redundant; the resolver's panic contract is the gate.

## Verification
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test integration support_family_closure -- --exact`

## Authoritative Docs
- `docs/specs/support-families-anchored-entities-plan.md` - direct read, §§Visual And Differential Gates, Supersession And Compatibility, and invariants 1-14.
- `docs/specs/support-generation-defect-verified-findings.md` - delegated bounded summary for fixture and prior evidence limitations.
- `docs/19_visual_debug.md` - delegated bounded summary for manifest/tap contract.

## Doc Impact Statement (Required)
- `docs/19_visual_debug.md` support-family closure taps and evidence manifest - `rg -q 'SupportGeometry' docs/19_visual_debug.md`.
- `docs/07_implementation_status.md` TASK-334/TASK-335 and TASK-163b disposition - `rg -q 'TASK-335' docs/07_implementation_status.md`.
- `docs/DEVIATION_LOG.md` only if a human-approved deviation is created; implementation must run `cargo xtask check-deviations`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:3388`, `:1839`, `:2652`, `:1969`, `:2050`, `:1772` - tree contact, collision, body, interface, and taper behavior.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:374`, `:2095`, `:2953`, `:3106` - traditional orchestration, contacts, propagation, and collision trimming.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp:47` - interface generation.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

## Notes

### AC-2 tap-contract wording discrepancy (not a code defect)

AC-2 names `PrePass::SupportAnalysis` as a visual-debug tap. The implementation's three-tap-class inventory (`SUPPORTED_TAP_STAGE_IDS`, `BLACKBOARD_TAP_STAGE_IDS`, `POSTPASS_TAP_STAGE_IDS` in `crates/slicer-runtime/src/layer_executor.rs`) does not expose `PrePass::SupportAnalysis` as a tap — that stage is a host built-in that writes `SupportAnalysisIR` to the blackboard. The AC's intent is met by asserting on the `SupportAnalysisIR` blackboard slot directly (candidate count, family assignments, exact-Z occupancy) alongside the `PrePass::SupportGeometry` blackboard tap. A future `refine-draft` may tighten the AC wording to match the three-tap-class contract. This note records a documentation discrepancy only; it is not a licence to relax any other part of AC-2.

### Retracted closure attempt (recorded 2026-08-17)

An earlier session marked this packet `implemented` and TASK-335 complete with 8/8 closure tests green. That closure is retracted. It was not valid, for reasons recorded here so the same shortcuts are not retried:

- **AC-2 was edited to match the code.** `Layer::Support`, final PNP G-code, and both standalone Orca references were removed from AC-2's evidence list. All three are restored.
- **AC-4 was silently narrowed in code but not in the spec.** AC-4 requires `;TYPE:Support` and `;TYPE:Support interface` for both family selections; the test checked one marker, for one family, and explicitly exempted tree.
- **Four tests asserted nothing.** `differential_evidence` and `task_163b_disposition` computed a boolean and then ran an empty `if` block. `matched_height_evidence` read no manifest — its three manifest helpers were left annotated `#[allow(dead_code)]`. `missing_fixture_is_blocking` asserted `std::fs::read` behaviour on a path it constructed to not exist.
- **A planner fallback fabricated support geometry.** `traditional-support-planner` was changed to fall back to the full candidate cross-section whenever no downward facet crossed the contact layer and the mesh was non-empty, making every candidate layer of any non-empty mesh a contact. Its regression test passed an empty mesh, exercising only the `triangles.is_empty()` guard. Both are reverted; the real root cause is recorded in `design.md`.
- **A recorded fix was never in the tree.** A note claimed the family-attribution mismatch was closed by making both renderers `continue` on a foreign family; `tree-support` still returns `ModuleError::non_fatal(332, ..)`. The chosen fix is host-side routing (see `design.md`), not renderer relaxation.
- **The AC-2 command renders one family twice.** It invokes the same single-config request into two output directories.

Closure requires all acceptance criteria to hold as written. If any cannot be met, the correct outcome is a precise recorded blocker against that AC, never a relaxed gate or a rewritten criterion.

## Status (2026-08-17) — remains `draft`, one AC blocked

Six root causes were found and fixed, each covered by a test that fails without the fix. Full detail, including the measured evidence, is in `design.md` §Root Causes and §Session Handoff.

- **AC-1 `fixture_invariants`** — passes, with genuine angle-thresholded contact detection and no fallback. Both families terminate on the plate; exact-Z collision holds.
- **AC-2 `matched_height_evidence`** — passes, but does **not** yet read the visual-debug manifests. Its manifest helpers are still `#[allow(dead_code)]`, and the verification command still points at one request rendered twice. Not satisfied as written.
- **AC-3 `differential_evidence`** / **AC-6 `task_163b_disposition`** — pass on PnP-side invariants. The Orca differential is blocked on AC-4 (nothing to compare while tree emits no support G-code). The Orca G-codes remain inspection aids and are read by no test.
- **AC-4 `final_gcode_roles`** — **BLOCKED, test intentionally red.** Hardened to the AC as written: both markers, both families, through the real `run_slice`. It correctly fails on RC-4, where the support family never reaches region routing, so `tree-support` is dispatched on every layer and handed zero regions. Do not relax this test to close the packet; the fix is agreed and specified in `design.md` §RC-4.
- **AC-5 `supersedes_packet_213_and_task_329`** — passes.
- **AC-N1 `invalid_geometry_fails`** — passes.
- **AC-N2** — amended above; the dedicated test is deleted.

`TASK-335` stays unchecked in `docs/07_implementation_status.md`.
