---
status: draft
packet: 224-support-family-orca-closure
task_ids:
  - TASK-335
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: All four originally named dependencies (220-support-analysis-family-contracts, 221-tree-support-family, 222-traditional-support-family, 223-mixed-support-family-routing) are implemented as of 2026-09-03; 213-support-planner-defect-fix is superseded. Statuses are ledger facts - re-derive with `grep '^status:' docs/spec_packets/<dep>/packet.spec.md`, never quote them from here. This packet additionally waits on the support sequence listed in the Activation Gate below.
---

# Packet Contract: support-family-orca-closure

## Goal
Close the support-family sequence with fixture-driven invariants and inspected visual/differential evidence showing tree and traditional support reach valid termination surfaces without model collision and retain family roles through final G-code.

## Scope Boundaries
This packet owns closure tests against the decisive fixtures, visual-debug manifests/PNGs, final G-code role checks, and packet 213/TASK-329 plus TASK-163b-orca-ref disposition evidence. It consumes all prior anchored, analysis, tree, traditional, and routing contracts and does not alter their algorithms.

## Prerequisites and Blockers
- Depends on: `220-support-analysis-family-contracts` (TASK-331), `221-tree-support-family` (TASK-332), `222-traditional-support-family` (TASK-333), `223-mixed-support-family-routing` (TASK-334). **All four were `implemented` when this was written on 2026-09-03**; `213-support-planner-defect-fix` is `superseded`. The earlier text calling 221/222/223 "draft" was stale.
- Unblocks: closure of the support-family remediation sequence.
- Activation blockers: [RESOLVED] TASK-331 exact-Z seam ownership and breaking-versus-additive WIT migration (packet 220 implemented; see `design.md` §Open Questions). The decisive SupportTest model and both Orca reference G-code fixtures exist; closure runs against them. Status remains `draft`.

### Activation Gate (added 2026-09-03) — do not execute this packet before it passes

This packet is the **closure** of the support sequence. Its evidence is a measurement of support behaviour, so executing it while any support packet is still open produces a checklist that is stale before it is committed (exactly the rot recorded in `design.md` §Orca Inspection Checklist). **Every packet below must be `implemented` before any step of this packet runs:**

`237-support-analysis-parity`, `238a-support-pattern-config-keys`, `238b-tree-planner-canonical-fidelity`, `238c-support-renderer-flow-interfaces`, `239a-anchored-host-seams`, `239b-anchored-wit-contract`, `239c-support-layer-height-producer`, `239d-support-coarse-floating-planes`, `240a-support-raft-substrate`, `240b-support-raft-module`, `241-support-agg-rasterizer`.

**Statuses are ledger facts and MUST be re-derived at activation, never read from this list.** Run:

```bash
for p in 237-support-analysis-parity 238a-support-pattern-config-keys 238b-tree-planner-canonical-fidelity 238c-support-renderer-flow-interfaces 239a-anchored-host-seams 239b-anchored-wit-contract 239c-support-layer-height-producer 239d-support-coarse-floating-planes 240a-support-raft-substrate 240b-support-raft-module 241-support-agg-rasterizer; do printf '%s: ' "$p"; grep -m1 '^status:' "docs/spec_packets/$p/packet.spec.md"; done
```

**At authoring time (2026-09-03)** the 237/238a/238b/238c/239a-d packets were `implemented` and **`240a-support-raft-substrate`, `240b-support-raft-module`, and `241-support-agg-rasterizer` were `draft` — those three are the live blockers.** Do not treat that as current; re-run the command.

### Relationship to `242-support-family-orca-closure` (neutral note, 2026-09-03)

A separate packet, `242-support-family-orca-closure`, exists and was authored to supersede this one; it covers overlapping ground. **The disposition between 224 and 242 — which one closes the sequence, and whether the other is retired — is an open decision for the human. This packet does not resolve it and makes no claim either way.** Nothing here edits 242.

## Acceptance Criteria
- **AC-1. Given** `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` and support enabled for each built-in family, **when** the real slice fixture test runs, **then** every accepted demand has an attributed body connected to plate/model termination, every body/nozzle sweep is disjoint from exact-Z occupancy within modeled tolerance, and no support-disabled output exists. | `cargo test -p slicer-runtime --test integration -- fixture_invariants --exact`
- **AC-2. Given** tree and traditional outputs for matched physical heights from `tmp/SupportTest.stl`, **when** `pnp_cli visual-debug` renders the dual-family request `tmp/visual-debug-support-family.json`, **then** the inspected PNGs and manifest cover host `PrePass::SupportAnalysis` candidates, occupancy, envelope, and routing cells; aggregated family `SupportPlanIR` body/interface polygons and skeletons; each anchored `Layer::Support` event with structured `SupportIR`; final PNP G-code; and standalone `tmp/SupportTest_Tree_Orca.gcode` and `tmp/SupportTest_Normal_Orca.gcode`, with tree and traditional views inspected at the same physical heights. Exact taps/layers are `PrePass::SupportAnalysis` at analysis layer heights, `PrePass::SupportGeometry` at support-plan layer heights, and `Layer::Support` at each anchored support event layer. | `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-tree.json --output target/vd-support-family-tree --overwrite && cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-normal.json --output target/vd-support-family-normal --overwrite && cargo test -p slicer-runtime --test integration -- matched_height_evidence --exact`

  *Amended 2026-08-18 (packet 224 remediation).* The original AC-2 required a **test** (`matched_height_evidence`) to establish that the inspected PNGs and manifest cover every listed tap, family, and standalone reference. That is not a machine-checkable property: what AC-2 actually demands is that a human or LLM **looked at** matched-height renders and reached a verdict. The test can only assert that the artefacts exist and are indexed — which is exactly the "PNG existence / manifest grep" evidence this packet's own `requirements.md` §Out of Scope forbids as proof. As implemented it was worse: its manifest helpers (`read_manifest`, `manifest_images`, `layer_indices`) were left `#[allow(dead_code)]`, so it read no manifest at all, and its verification command rendered the same single-family request into two output directories. **AC-2 is hereby inspection-only.** It is satisfied by the written checklist in `design.md` §Orca Inspection Checklist, which must name per family the two per-family requests, the matched physical heights inspected, the Orca render placed beside each PnP render, and the verdict for each of termination, coverage, collision freedom, interface placement/count, and independent heights — each with the layer and tap it was read from. The `matched_height_evidence` test is retained **only** as a precondition for that inspection; it is not evidence of AC-2 and may not be cited as such. The two-request command above replaces the single-request-twice form. Exact path identity is never claimed.

  *Amended again 2026-09-03 (activation-gate review) — two corrections.*

  **(i) The checklist is a point-in-time measurement and MUST be regenerated at execution time.** Every packet in the Activation Gate above changes support behaviour, so a checklist written before them is stale by construction — and the existing one demonstrably is: its traditional interface placement/count verdict (`DIVERGENT`, PnP 2 vs Orca 3) was **closed by `238c-support-renderer-flow-interfaces`**, which brought PnP to 3 blocks at `top_layers=2`/`bottom_layers=2`, matching Orca. **The 2026-08-20 table in `design.md` §Orca Inspection Checklist is a historical record and is NOT reusable as closure evidence for AC-2, AC-3 half (b), or AC-6 half (b).** Those three obligations are satisfied only by a **new, separately dated checklist regenerated against HEAD** at execution time, carrying the same five required elements: the two per-family requests; the matched physical heights; the Orca render placed beside each PnP render; a verdict for each of termination / coverage / collision freedom / interface placement+count / independent heights; and the layer **and** tap each verdict was read from. Do not edit the old table in place and do not refresh it selectively.

  **The regenerated checklist must stand alone.** The four render bundles (`target/vd-support-family-*`, `target/vd-orca-*-compare`), the four `tmp/*.json` request files, and the two `tmp/SupportTest_*_Orca.gcode` references are **untracked/gitignored** — a later reader cannot re-open any of them. The checklist must therefore inline the request contents (or their decisive fields), the exact layer indices with their physical Z, the Orca profile used, and a description of what was seen precise enough to audit each verdict without the bundles.

  **(ii) `matched_height_evidence` is not an artefact-presence precondition.** It reads no manifest and touches no render artefact (there is not a single `manifest` reference in `crates/slicer-runtime/tests/integration/support_family_closure.rs`); it asserts a **physical-Z overlap between the two families**. That is a genuine and useful assertion, so the honest fix is to describe it as what it is rather than to bolt manifest reads onto it: the obligation is renamed the **matched-height overlap precondition** — it establishes that the two families actually carry support at common physical heights, which is the property that makes a matched-height comparison meaningful in the first place. Artefact presence is not re-asserted, because the inspection itself fails immediately if a bundle is missing, and because "the PNG exists" is evidence this packet's own `requirements.md` §Out of Scope forbids.
- **AC-3. Given** PNP and standalone Orca tree/normal references at matched heights, **when** differential review runs, **then** the inspected evidence records source, layer, tap, and disposition for both families, with behavioral parity claims limited to termination, coverage, collision freedom, interfaces, and independent heights rather than exact path identity. | `cargo test -p slicer-runtime --test integration -- differential_evidence --exact`

  *Amended 2026-08-18 (packet 224 remediation).* The original AC-3 required `differential_evidence` to establish that "the inspected evidence records source, layer, tap, and disposition for both families". A test cannot do this: no test may read the Orca G-code (locked gate shape, `design.md` §Orca Differential Evidence), so a PnP-only test has nothing to differentiate against — and the implemented version duly computed a boolean and ran an empty `if` block. **AC-3 is hereby invariant-plus-recorded-inspection.** It is satisfied only when both halves hold: (a) `differential_evidence` asserts **PnP-side structural invariants** — per-entry attribution (family, body, demand, and **non-empty `object_id` and `region_id`, asserted, not merely interpolated into an error message**), a decline reason on **every** unmet demand, and role presence for both families — failing loudly on any missing field; and (b) the differential itself is **recorded by inspection** in a checklist **regenerated at execution time** (see the AC-2 amendment and `design.md` §Orca Inspection Checklist), naming source, layer, tap, and disposition per family. Neither half alone satisfies AC-3. Parity claims stay limited to termination, coverage, collision freedom, interfaces, and independent heights; exact path identity is never claimed.

  *Amended again 2026-09-03 (activation-gate review), half (a).* **The decline-reason assertion is currently unreachable.** In `crates/slicer-runtime/tests/integration/support_family_closure.rs`, `pnp_support_evidence` hard-errors on the **first** declined entry, so control never reaches `assert_attribution_and_decline_reasons` with a decline in hand — "a decline reason on every unmet demand" can never execute. Separately, `object_id` and `region_id` appear only inside error-message formatting and are never asserted non-empty. Half (a) is met only when the declined-entry path is **reachable** (collect declines and evaluate them, rather than hard-erroring on the first) and object/region identity is **asserted**. Half (b) may not cite the 2026-08-20 checklist; that record is superseded.
- **AC-4. Given** final PNP G-code for both family selections, **when** role inspection runs, **then** support and interface output contains the exact markers `;TYPE:Support` and `;TYPE:Support interface`, and family attribution remains present in the closure evidence manifest. | `cargo test -p slicer-runtime --test integration -- final_gcode_roles --exact`
- **AC-5. Given** packet 213's degenerate-disk result and reopened `TASK-329`, **when** closure evidence is reviewed, **then** the packet records them as superseded by this fixture-backed closure and does not count the degenerate-disk result as evidence. | `cargo test -p slicer-runtime --test integration -- supersedes_packet_213_and_task_329 --exact`
- **AC-6. Given** `TASK-163b-orca-ref`, **when** the authoritative Orca fixtures are reviewed, **then** closure evidence either closes that task with the existing authoritative tree/normal references or records a precise external blocker; it never claims exact path parity. | `cargo test -p slicer-runtime --test integration -- task_163b_disposition --exact`

  *Amended 2026-08-18 (packet 224 remediation).* The original AC-6 required `task_163b_disposition` to establish that the authoritative Orca fixtures either close `TASK-163b-orca-ref` or record a precise external blocker. Closing a task on fixture **authority and provenance** is a judgement about where a file came from; it is not a property of the file that a test can read — and the implemented version, like AC-3's, computed a boolean and ran an empty `if` block. **AC-6 is hereby invariant-plus-recorded-inspection.** It is satisfied only when both halves hold: (a) `task_163b_disposition` asserts the **PnP-side invariants** it can genuinely check — that the tracked decisive fixture resolves via `support_test_path` and that no Orca-derived constant and no read of `tmp/SupportTest_*_Orca.gcode` appears in any test; and (b) the disposition of `TASK-163b-orca-ref` is **recorded in writing** in `design.md`, stating the references' provenance and regeneration settings and either closing the task or naming the precise external blocker, **beside a checklist regenerated at execution time** (see the AC-2 amendment). Exact path parity is never claimed by either half.

  *Amended again 2026-09-03 (activation-gate review), half (a).* **The scanner implements about half of what half (a) demands.** `collect_test_files` (`crates/slicer-runtime/tests/integration/support_family_closure.rs`) is real, but (i) it **excludes its own file**, so a forbidden reference added to `support_family_closure.rs` — the single most likely place for one — is invisible; and (ii) it scans only for the three Orca G-code **path literals**, with **no scan at all for Orca-derived numeric constants**, which this AC explicitly names. Half (a) is met only when the scanner **includes its own file** and when every hardcoded canonical-provenance constant in the closure tests either **cites its canonical source (file + function, never a line number)** at the constant, or is replaced by a value derived from config. **Known instance to dispose of:** `interface_band_counts_match_canonical_structure` hardcodes `count != 3`, justified only by a doc-comment asserting canonical provenance — precisely the class this AC exists to catch. Either cite `Slic3r::generate_interface_layers` (`SupportCommon.cpp`) at the constant or derive the expected band count from `support_interface_top_layers` / `support_interface_bottom_layers`. Half (b) may not cite the 2026-08-20 checklist; that record is superseded.
- **AC-7 (added 2026-09-03). Given** the four support modules (`tree-support-planner`, `traditional-support-planner`, `tree-support`, `traditional-support`), **when** config-key reconciliation runs, **then** every key declared in a module's manifest `[config.schema]` is either read by that module's `src/` or recorded in `docs/specs/support-parity-gap-register.md` as a dead key with an owning destination packet; the inverted G-16 and G-22 rows are corrected; and the `tree-support` `line_width` / `support_line_width` mismatch is either fixed or explicitly recorded as a deviation. | `bash -c 'miss=0; for m in tree-support-planner traditional-support-planner tree-support traditional-support; do for k in $(sed -n "s/^\[config\.schema\.\([a-z0-9_]*\)\]/\1/p" "modules/core-modules/$m/$m.toml" | sort -u); do grep -rqF "\"$k\"" "modules/core-modules/$m/src/" && continue; grep -q "declared-but-unread.*$k" docs/specs/support-parity-gap-register.md && continue; echo "UNRECONCILED: $m $k"; miss=1; done; done; test "$miss" -eq 0'`

  **Gate convention.** The command above is the AC's single runnable check: for each of the four modules it lists every `[config.schema.<key>]` declared in the manifest, passes the key if the module's `src/` reads it, and otherwise requires a row in `docs/specs/support-parity-gap-register.md` containing the literal marker **`declared-but-unread`** alongside the key name. **224 must write that marker into each dead-key row** (including the corrected G-16 and G-22 rows); it is what makes the reconciliation machine-checkable instead of a promise. The command **exits 1 today**, listing all five keys — that is the unmet deliverable, not a broken command.

  *Authoring-time finding (2026-09-03) — **re-derive, do not quote**.* Five declared-but-unread keys were observed; only one of them is recorded in the register. This table is a **ledger fact** and rots — re-run the re-derivation command in `requirements.md` before acting on it.

  | module | declared-but-unread key | in the gap register? |
  | --- | --- | --- |
  | `tree-support-planner` | `support_branch_merge_distance_mm` | **NO** — and **G-16 asserts the opposite**, claiming the key is "not declared in its manifest". G-16 is **inverted**; it is declared in `modules/core-modules/tree-support-planner/tree-support-planner.toml`. |
  | `traditional-support-planner` | `support_threshold_angle` | **NO** — and **G-22 asserts** "No module manifest declares `support_threshold_angle` or its legacy alias", contradicted by `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`. G-22 is **inverted**. |
  | `traditional-support-planner` | `support_overhang_angle` (the module's own source comment says it "is no longer read here") | **NO** |
  | `traditional-support-planner` | `support_base_pattern_spacing` | YES — G-03 |
  | `tree-support` | `line_width` | **NO** |

  **Live defect, not bookkeeping.** `modules/core-modules/tree-support/` declares `line_width` in its manifest, but `modules/core-modules/tree-support/src/lib.rs` reads **`support_line_width`**, which that manifest does **not** declare. A module's config view is filtered to its declared schema, so `support_line_width` always resolves to its `1.125 * nozzle_diameter` in-code default and the declared `line_width` is dead. Fix it (declare the key the module actually reads, or read the key the manifest declares) or record it explicitly with a named owner — silence is not a disposition.

  **Register ownership note.** The G-16/G-22 corrections and the dead-key rows are **224's work**, but `docs/specs/support-parity-gap-register.md` is a **shared file also written by `242-support-family-orca-closure`**. Coordinate before editing; do not assume exclusive ownership, and re-read the file immediately before writing.

## Negative Test Cases
- **AC-N1. Given** a fixture body that aggregation must itself **judge** invalid — entering exact-Z model occupancy, lacking a valid termination, or carrying a cross-family overlap — supplied with a **non-empty** `roles` list and **no** pre-set `decline_reason`, **when** closure validation runs, **then** the returned plan is **bound and asserted on**: the invalid body is **absent** from the retained plan, its demand is present and carries an unmet marker (a structured decline diagnostic naming the body and a reason), and the test fails rather than accepting a golden or fallback path. | `cargo test -p slicer-runtime --test integration -- invalid_geometry_fails --exact`

  *Amended 2026-09-03 (activation-gate review).* The original AC-N1 required the body to be **dropped**, but `invalid_geometry_fails` (`crates/slicer-runtime/tests/integration/support_family_closure.rs`) discards the retained plan — `let (_, diagnostics) = try_aggregate_support_plan_irs_with_diagnostics(...)` — and asserts only on the diagnostic. **The drop is therefore never asserted.** Its inputs are also too weak to exercise the judgement the AC names: one entry arrives with `decline_reason: Some(Blocked)` already set by the test, and `invalid_body` carries `roles: Vec::new()`, so aggregation can reject on emptiness or simply echo the caller's label without evaluating geometry. The amended Then-clause above is the gate: bind the plan, assert the body's absence and the demand's unmet marker, and feed inputs that force a judgement.
- **AC-N2. Given** the decisive fixture is absent from its tracked path, **when** any closure test runs, **then** the shared `support_test_path` resolver panics naming the exact tracked path, so every closure test fails loudly rather than silently degrading; the tracked fixture remains the primary closure path and parity is never claimed from PNG existence alone. | covered by the `support_test_path` resolver contract exercised by every closure test in `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition --exact`

  *Amended 2026-08-17 (packet 224 remediation).* The original AC-N2 mandated a dedicated `missing_fixture_is_blocking` test against "a deliberately missing copied fixture path". As implemented, that test asserted only that `std::fs::read` returns `NotFound` for a path the test itself constructed to not exist — it exercised `std::fs`, not the closure gate, and was a placeholder. The fixture is now tracked in-repo, and `support_test_path` already panics with the tracked path when it is absent, which fails all seven remaining closure tests loudly. The dedicated test **was deleted in this session, in commit `4c67ccd9`†** — it was not already absent when this amendment was first written, and any reading of the earlier wording as "already deleted" was wrong. The resolver's panic contract is the gate.

## Verification
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition --exact`

**Test-name filters, corrected 2026-08-18.** Every command above previously carried `support_family_closure` as a filter token. **That form runs ZERO tests** — it reports `0 passed; ... 306 filtered out` and exits `0`, which reads as a pass. The closure tests are registered in `crates/slicer-runtime/tests/integration/main.rs` as bare `#[test] fn` wrappers, so no test name carries a module prefix and nothing matches `support_family_closure`. Filter on explicit test names with `--exact`, and confirm the `N passed` count is non-zero before believing any closure run.

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

*Line-pinned citations removed 2026-09-03 per `CLAUDE.md` §OrcaSlicer Citation Style; cite by file + function only. Each function below was resolved against the local `OrcaSlicerDocumented/` checkout.*

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::generate_contact_points` (tree contact-point derivation, incl. the top-gap virtual node); `TreeSupport::get_collision` (collision-avoidance volume lookup); `TreeSupport::drop_nodes` (branch body propagation between layers); `TreeSupport::draw_circles` (builds the base, roof, and first-layer-roof **areas** — it is not a dedicated interface routine, so do not describe it as one); `TreeSupport::calc_branch_radius` (branch-radius taper).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — the class is **`PrintObjectSupportMaterial`**, not `SupportMaterial`: `PrintObjectSupportMaterial::generate` (traditional orchestration); `PrintObjectSupportMaterial::top_contact_layers` (contact-layer derivation); `PrintObjectSupportMaterial::generate_base_layers` (downward propagation); `PrintObjectSupportMaterial::trim_support_layers_by_object` (collision trimming).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `Slic3r::generate_interface_layers` (free function in namespace `Slic3r`; roof/floor interface generation and layer-count semantics).

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

## Status (2026-08-18) — remains `draft`

> **HISTORICAL RECORD — SUPERSEDED, NOT CLOSURE EVIDENCE (annotated 2026-09-03).**
> Every per-AC pass/fail claim below was measured on 2026-08-18/20, **before** `237`, `238a`, `238b`, `238c`, and `239a`-`239d` landed, and before `240a`/`240b`/`241` were authored. Support behaviour has changed since — at least one claim here is known wrong (the interface-count divergence; see `design.md` §Checklist correction). **Do not cite any row below as evidence, and do not re-quote its verdicts.** The ACs must be re-run against `HEAD` after the §Activation Gate passes; the amendments dated 2026-09-03 under each AC above are authoritative where they conflict with this block.

**This block supersedes the 2026-08-17 status, which is corrected in two places: it reported RC-4 as a live blocker and `final_gcode_roles` as intentionally red. Both are wrong now.** RC-4 is **FIXED** (region-config backfill in `promote_global_layers`, `crates/slicer-runtime/src/layer_executor.rs`; see `design.md` §RC-4) and `final_gcode_roles` **PASSES** for both families through the real `run_slice`. RC-11 is also fixed: `tree-support-planner`'s `from_config` now reads `support_top_z_distance_mm` and honours it (see `design.md` §RC-11 for how tree and traditional differ — the tree planner computes a layer count, it does not walk Z). Do not requote the 2026-08-17 state.

Root causes RC-0..RC-17 are recorded in `design.md` §Root Causes, with measured evidence. RC-14..RC-17 and the top-Z-gap deviation were added on 2026-08-18.

- **AC-1 `fixture_invariants`** — passes, with genuine angle-thresholded contact detection and no fallback. Both families terminate on the plate; exact-Z collision holds.
- **AC-2** — **amended above to inspection-only.** `matched_height_evidence` is now an artefact-presence precondition, not evidence. Satisfied when `design.md` §Orca Inspection Checklist is written; that checklist is still outstanding.
- **AC-3 `differential_evidence`** / **AC-6 `task_163b_disposition`** — **amended above to invariant-plus-recorded-inspection.** The invariant halves pass; the recorded-inspection halves are outstanding, and no test reads the Orca G-code.
- **AC-4 `final_gcode_roles`** — **passes.** Hardened to the AC as written (both markers, both families, through the real `run_slice`). It was red on RC-4, which is fixed. It must stay hardened; a regression here is a real failure, not a gate to relax.
- **AC-5 `supersedes_packet_213_and_task_329`** — passes.
- **AC-N1 `invalid_geometry_fails`** — passes.
- **AC-N2** — amended above; the dedicated test was deleted in `4c67ccd9`†.

**Open 224 work.**
- **RC-15 — tree contact-point derivation.** ~~Classified a GAP but agreed to be implemented in 224, not routed.~~ **IMPLEMENTED 2026-08-20 (`ad9019ee`†, Step 3b).** The three canonical sampling streams landed with the collision-gate narrowing reverted; closure 12/12, planner crate only RC-C red, tree deficit re-measured 1.58x deposited / 1.75x XY path.
- **Interface layer counts** — ~~the normal family's 1-versus-3 `;TYPE:Support interface` block count.~~ **DONE (Step 2)**: counts follow the configured top band (1/2/3 exact, pinned by `interface_layer_count_follows_config`). ~~The remaining 2-vs-3 difference against Orca at `top=2`/`bottom=2` is canonical roof/floor band structure, registered as gap G-18.~~ **Corrected 2026-09-03: that difference no longer exists.** `238c-support-renderer-flow-interfaces` closed it (`ea71ebc0`, reachable from HEAD); PnP and Orca now agree at **3** blocks at `top_layers=2`/`bottom_layers=2`, asserted by `interface_band_counts_match_canonical_structure` (`crates/slicer-runtime/tests/integration/support_family_closure.rs`). **G-18 is closed-by-238c — its register destination column already routes it there — and is not an open 224 gap.**
- **RC-17 — tree-family regressions from `9f4540bd`†.** ~~Ten failures across `tree-support-planner` and `tree-support` at HEAD, eight of them introduced by the renderer rewrite.~~ **DONE (Step 3a, session-3 audit).** RC-A/RC-B fixed in production; RC-C left red for Step 8 regeneration; RC-D/RC-E carried the audit's verdicts.
- **The Orca inspection checklist** (`design.md` §Orca Inspection Checklist), required by amended AC-2/AC-3/AC-6. ~~**WRITTEN 2026-08-20 (Step 6).**~~ **Re-opened 2026-09-03.** The 2026-08-20 checklist is a superseded historical record (one verdict is now known wrong; see the AC-2 amendment) and may not be cited as closure evidence. AC-2, AC-3(b) and AC-6(b) require a **checklist regenerated at execution time against HEAD**, after the Activation Gate passes. This is outstanding work, not done work.

`TASK-335` stays unchecked in `docs/07_implementation_status.md`.

---

† Commit SHAs marked † were **orphaned by the packet-224 squash** and no longer resolve (`git show` fails on a fresh clone). The work is present at HEAD in the reachable squashes `bab79c5c` and `55211648`. Cite a change by what it did plus its crate-qualified symbol or file, never by an orphaned SHA; see `design.md` §Orphaned commit references.
