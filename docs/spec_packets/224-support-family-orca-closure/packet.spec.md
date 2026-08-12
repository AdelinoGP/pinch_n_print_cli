---
status: draft
packet: 224-support-family-orca-closure
task_ids:
  - TASK-335
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Depends on draft tree-support-family, traditional-support-family, and mixed-support-family-routing; inherited TASK-331 blockers remain open.
---

# Packet Contract: support-family-orca-closure

## Goal
Close the support-family sequence with fixture-driven invariants and inspected visual/differential evidence showing tree and traditional support reach valid termination surfaces without model collision and retain family roles through final G-code.

## Scope Boundaries
This packet owns closure tests against the decisive fixtures, visual-debug manifests/PNGs, final G-code role checks, and packet 213/TASK-329 plus TASK-163b-orca-ref disposition evidence. It consumes all prior anchored, analysis, tree, traditional, and routing contracts and does not alter their algorithms.

## Prerequisites and Blockers
- Depends on: draft `tree-support-family` (TASK-332), draft `traditional-support-family` (TASK-333), and draft `mixed-support-family-routing` (TASK-334).
- Unblocks: closure of the support-family remediation sequence.
- Activation blockers: [BLOCK] TASK-331 exact-Z seam ownership; [BLOCK] TASK-331 breaking-versus-additive WIT migration. The decisive SupportTest model and both Orca reference G-code fixtures exist; closure runs against them. Status remains draft.

## Acceptance Criteria
- **AC-1. Given** `tmp/SupportTest.stl` and support enabled for each built-in family, **when** the real slice fixture test runs, **then** every accepted demand has an attributed body connected to plate/model termination, every body/nozzle sweep is disjoint from exact-Z occupancy within modeled tolerance, and no support-disabled output exists. | `cargo test -p slicer-runtime --test integration support_family_closure -- fixture_invariants -- --exact`
- **AC-2. Given** tree and traditional outputs for matched physical heights from `tmp/SupportTest.stl`, **when** `pnp_cli visual-debug` renders the dual-family request `tmp/visual-debug-support-family.json`, **then** the inspected PNGs and manifest cover host `PrePass::SupportAnalysis` candidates, occupancy, envelope, and routing cells; aggregated family `SupportPlanIR` body/interface polygons and skeletons; each anchored `Layer::Support` event with structured `SupportIR`; final PNP G-code; and standalone `tmp/SupportTest_Tree_Orca.gcode` and `tmp/SupportTest_Normal_Orca.gcode`, with tree and traditional views inspected at the same physical heights. Exact taps/layers are `PrePass::SupportAnalysis` at analysis layer heights, `PrePass::SupportGeometry` at support-plan layer heights, and `Layer::Support` at each anchored support event layer. | `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family.json --output target/vd-support-family-tree --overwrite && cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family.json --output target/vd-support-family-normal --overwrite && cargo test -p slicer-runtime --test integration support_family_closure -- matched_height_evidence -- --exact`
- **AC-3. Given** PNP and standalone Orca tree/normal references at matched heights, **when** differential review runs, **then** the inspected evidence records source, layer, tap, and disposition for both families, with behavioral parity claims limited to termination, coverage, collision freedom, interfaces, and independent heights rather than exact path identity. | `cargo test -p slicer-runtime --test integration support_family_closure -- differential_evidence -- --exact`
- **AC-4. Given** final PNP G-code for both family selections, **when** role inspection runs, **then** support and interface output contains the exact markers `;TYPE:Support` and `;TYPE:Support interface`, and family attribution remains present in the closure evidence manifest. | `cargo test -p slicer-runtime --test integration support_family_closure -- final_gcode_roles -- --exact`
- **AC-5. Given** packet 213's degenerate-disk result and reopened `TASK-329`, **when** closure evidence is reviewed, **then** the packet records them as superseded by this fixture-backed closure and does not count the degenerate-disk result as evidence. | `cargo test -p slicer-runtime --test integration support_family_closure -- supersedes_packet_213_and_task_329 -- --exact`
- **AC-6. Given** `TASK-163b-orca-ref`, **when** the authoritative Orca fixtures are reviewed, **then** closure evidence either closes that task with the existing authoritative tree/normal references or records a precise external blocker; it never claims exact path parity. | `cargo test -p slicer-runtime --test integration support_family_closure -- task_163b_disposition -- --exact`

## Negative Test Cases
- **AC-N1. Given** a fixture body entering exact-Z model occupancy, lacking a valid termination, or having cross-family overlap, **when** closure validation runs, **then** the body is dropped, its demand is marked unmet with a structured diagnostic, and the test fails rather than accepting a golden or fallback path. | `cargo test -p slicer-runtime --test integration support_family_closure -- invalid_geometry_fails -- --exact`
- **AC-N2. Given** a deliberately missing copied fixture path, **when** the closure gate runs, **then** it reports the precise missing fixture and exits non-zero; the existing decisive fixtures remain the primary closure path and parity is never claimed from PNG existence alone. | `cargo test -p slicer-runtime --test integration support_family_closure -- missing_fixture_is_blocking -- --exact`

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
