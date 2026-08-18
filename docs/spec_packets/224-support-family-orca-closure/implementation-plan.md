# Implementation Plan: support-family-orca-closure

## Execution Rules
- Work one atomic step at a time; map every step to `TASK-335`.
- Use TDD, then implementation, then narrow falsifying validation.
- Never claim parity from uninspected or self-captured goldens.
- **No test may read `tmp/SupportTest_Tree_Orca.gcode` or `tmp/SupportTest_Normal_Orca.gcode`**, and no Orca-derived constant may be hardcoded into a test. Parity gating is structural invariants plus the written inspection checklist.
- **Extruding-move counts are not a parity metric** (Orca tree segments are ~15x shorter). Do not gate on them or quote them as evidence.
- Run `cargo xtask build-guests --check` after editing any `modules/core-modules/*/src/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`, or `crates/slicer-core/**` path, and rebuild before trusting any measurement. Stale-guest artifacts already caused one recorded false diagnosis (see `design.md` §Root Causes RC-11).
- `eprintln!` from guest code does not reach the test harness; use `push_diagnostic`.
- Every canonical feature gap discovered mid-flight is **registered and routed** (`docs/specs/support-parity-gap-register.md`, packets 224a/225/226/227), never implemented here.

## Steps

### Step 0: Protect and re-baseline
- Task IDs: `TASK-335`
- Objective: get the existing uncommitted work onto a commit, prove guest freshness, derive a matched Orca config fixture, re-measure the baseline against the regenerated Orca references, and void the stale handoff numbers.
- Precondition: the working tree carries the uncommitted packet-224 work described in `HANDOFF-224.md`; branch `parity/support-planners`.
- Postcondition: work is committed; `cargo xtask build-guests --check` is clean; a matched PnP config fixture reproducing the recorded Orca `normal` profile exists; the dated baseline table in `design.md` §Measured Baseline is confirmed or corrected in place; `HANDOFF-224.md` carries a header stating its numbers are void.
- Files allowed to read: `HANDOFF-224.md`; `git status` / `git diff --stat` output; `docs/spec_packets/224-support-family-orca-closure/design.md`.
- Files allowed to edit: `HANDOFF-224.md` (void annotation only); `docs/spec_packets/224-support-family-orca-closure/design.md` (§Measured Baseline only); one matched-config fixture file under `crates/slicer-runtime/tests/fixtures/support-family/`.
- Files explicitly out of bounds: all `modules/core-modules/**/src/**`; all `crates/**/src/**`; `packet.spec.md`.
- Expected sub-agent dispatches: Question: re-measure the five baseline metrics for both families from freshly emitted PnP G-code and the two Orca references; scope: `target/**`, `tmp/*.gcode`; return: `FACT` table only.
- Context cost: `S`
- Verification: `cargo xtask build-guests --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo test -p slicer-runtime --test integration support_family_closure`.
- Exit condition: a commit exists containing the prior verified work, the guest check is clean, and the baseline table is dated and reproducible.

### Step 1: Tree top-Z gap (RC-11)
- Task IDs: `TASK-335`
- Objective: make `tree-support-planner` read `support_top_z_distance_mm` and honour it by shifting the contact layer along actual layer Z.
- Precondition: Step 0 committed and guest check clean. `from_config` currently reads 17 keys and not this one; the key is absent from `crates/slicer-schema/wit/`.
- Postcondition: a red-first test pins the gap between the overhang underside and the topmost tree support layer for at least two distinct config values; the contact shift is derived by **walking actual layer Z** (the technique `traditional-support-planner::plan_for_object` uses); `effective_layer_height` is never used as a divisor in the new code.
- Files allowed to read: `modules/core-modules/tree-support-planner/src/**`; `modules/core-modules/traditional-support-planner/src/lib.rs` (`plan_for_object` only); `modules/core-modules/tree-support-planner/tree-support-planner.toml`.
- Files allowed to edit (3): `modules/core-modules/tree-support-planner/src/lib.rs`; `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`; the tree-support-planner manifest if the declared key needs correcting.
- Files explicitly out of bounds: `modules/core-modules/traditional-support-planner/src/**` (read-only); `crates/slicer-schema/wit/**`; both renderers.
- Expected sub-agent dispatches: Question: describe canonical tree contact-gap placement; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY`.
- Context cost: `M`
- Verification: `cargo test -p tree-support-planner`; then `cargo xtask build-guests --check` (rebuild if stale) and `cargo test -p slicer-runtime --test integration support_family_closure`.
- Exit condition: the red-first test is green, the gap changes with the config value, and every measurement was taken after a clean guest check.

### Step 2: Interface layer counts in both families
- Task IDs: `TASK-335`
- Objective: diagnose and fix why `support_interface_top_layers` / `support_interface_bottom_layers` are not honoured, so both families emit the configured number of interface layers.
- Precondition: Step 1 landed. Measured blocker: PnP normal emits 1 `;TYPE:Support interface` block against Orca's 3 at `top_layers=2` / `bottom_layers=2` (see `design.md` §Measured Baseline).
- Postcondition: the counts are honoured by both families; a permanent test asserts the interface layer count as a function of the two config keys (not against a hardcoded Orca number); interface remains carved out of the body and is never duplicated with it.
- Files allowed to read: both planners' `src/**`; both renderers' `src/**`; `crates/slicer-wasm-host/src/marshal/out.rs`.
- Files allowed to edit (3): `modules/core-modules/tree-support/src/lib.rs`; `modules/core-modules/traditional-support/src/lib.rs`; `crates/slicer-runtime/tests/integration/support_family_closure.rs`. If the defect proves planner-side, substitute the owning planner `src/lib.rs` for one renderer file and record the substitution.
- Files explicitly out of bounds: `crates/slicer-schema/wit/**`; interface **pattern** generation (packet 226); `support_bottom_z_distance` (packet 226).
- Expected sub-agent dispatches: Question: canonical roof/floor layer-count semantics, including the negative-`bottom_interface_layers` fallback to `support_interface_top_layers`; scope: `TreeSupport.cpp`, `SupportMaterial.cpp`, `SupportCommon.cpp`; return: `SUMMARY`.
- Context cost: `M`
- Verification: `cargo test -p slicer-runtime --test integration support_family_closure`; `cargo xtask build-guests --check`.
- Exit condition: configured interface layer counts are produced by both families and pinned by a test that fails when the keys are ignored.

### Step 3: Tree density diagnosis (read-only)
- Task IDs: `TASK-335`
- Objective: produce a written, evidence-backed root cause for tree support filament at **31.6%** of Orca's (486.33 mm vs 1538.36 mm) over an identical Z range and layer count.
- Precondition: Steps 1-2 landed and re-measured. This step is **read-only** and changes no production code.
- Postcondition: a root-cause write-up in `design.md` that explicitly eliminates or convicts each of: fill pitch versus `support_base_pattern_spacing`; wall count; branch radius; branch count. Only after the cause is known is the finding classified **bug** (fixed in a follow-up step of this packet) or **gap** (registered in Step 7). Classifying before diagnosis is prohibited.
- Files allowed to read: `modules/core-modules/tree-support/src/**`; `modules/core-modules/tree-support-planner/src/**`; emitted PnP G-code and the two Orca references under `tmp/` (inspection only, via sub-agent).
- Files allowed to edit (1): `docs/spec_packets/224-support-family-orca-closure/design.md`.
- Files explicitly out of bounds: every `src/**` path (this step edits no code); `packet.spec.md`.
- Expected sub-agent dispatches: Question: per-layer support extrusion length and line spacing for PnP tree versus Orca tree at three matched heights; scope: emitted G-code plus `tmp/*.gcode`; return: `FACT`. Question: canonical tree base-area density derivation; scope: `TreeSupport.cpp`; return: `SUMMARY`.
- Context cost: `M`
- Verification: no cargo change required; re-measure only with `cargo xtask build-guests --check` clean before quoting any number.
- Exit condition: each of the four candidate causes is eliminated or convicted with a measurement, and a bug-versus-gap classification is recorded with its basis.

### Step 4: Config-key reconciliation (four support modules)
- Task IDs: `TASK-335`
- Objective: reconcile declared-versus-read config keys across `tree-support-planner`, `traditional-support-planner`, `tree-support`, and `traditional-support` only.
- Precondition: Steps 1-3 landed. Known dead keys: raft keys and `support_base_pattern` — these **stay as-is**, recorded rather than wired or removed.
- Postcondition: for each of the four modules, every manifest-declared key is either read by that module or listed as a dead key with its routing packet; the list lands in the gap register (create the stub here if Step 7 has not run).
- Files allowed to read: the four modules' `src/**` and `*.toml`; `docs/15_config_keys_reference.md`.
- Files allowed to edit (3): the four modules' `*.toml` manifests (only where a key must be corrected) and `docs/specs/support-parity-gap-register.md`.
- Files explicitly out of bounds: `xtask/**` — **no new xtask gate is introduced by this packet**; any module outside the four; `crates/slicer-schema/wit/**`.
- Expected sub-agent dispatches: Question: enumerate declared-versus-read config keys for the four support modules; scope: those four module directories; return: `LOCATIONS`.
- Context cost: `S`
- Verification: `cargo xtask build-guests --check`; `cargo test -p tree-support-planner`; `cargo test -p traditional-support-planner`.
- Exit condition: the declared/read/dead classification is complete for all four modules and every dead key names its routing packet.

### Step 5: Honest tests
- Task IDs: `TASK-335`
- Objective: delete the tests that assert nothing and replace them with real invariants on tracked models.
- Precondition: Steps 1-4 landed.
- Postcondition: deleted — the empty `if` blocks in `differential_evidence` and `task_163b_disposition`, the `missing_fixture_is_blocking` test, and the three `#[allow(dead_code)]` manifest helpers (`read_manifest`, `manifest_images`, `layer_indices`). Added — four invariants, each exercised across `resources/cube_with_concave_hole_enlarged_standing.obj`, `resources/two_hollow_squares.obj`, `resources/V_standing.obj`, and `resources/A_upsidedown.obj`:
  1. **Exact-Z non-intersection** — no support body or nozzle sweep intersects model occupancy at exact Z.
  2. **Termination** — every accepted demand terminates on the build plate or on the model.
  3. **Interface placement** — interface is the topmost support and is carved out of the body, never duplicated with it.
  4. **No-overhang null result** — a mesh with no overhang produces zero support entries.
- Files allowed to read: `crates/slicer-runtime/tests/integration/**`; the `resources/` model listing.
- Files allowed to edit (3): `crates/slicer-runtime/tests/integration/support_family_closure.rs`; `crates/slicer-runtime/tests/integration/main.rs`; one family integration test file if an invariant belongs there.
- Files explicitly out of bounds: all `src/**`; `tmp/*.gcode` (no test may read them).
- Expected sub-agent dispatches: Question: confirm the four `resources/` models are tracked and load through the real slice path; scope: `resources/**`; return: `FACT`.
- Context cost: `M`
- Verification: `cargo test -p slicer-runtime --test integration support_family_closure`; `cargo xtask check-literals`.
- Exit condition: no closure test contains an assertion-free branch or a dead helper, and each of the four invariants fails when its guard is inverted.

### Step 6: Inspection gate
- Task IDs: `TASK-335`
- Objective: replace the broken AC-2 command with two genuine per-family visual-debug requests, render Orca G-code side by side at matched heights, and write the inspection checklist.
- Precondition: Steps 1-5 landed; both families emit support G-code. The current AC-2 command renders **one family twice**.
- Postcondition: two distinct request fixtures (`tmp/visual-debug-support-family-tree.json`, `tmp/visual-debug-support-family-normal.json`) replace the single request rendered twice; each family and its Orca counterpart are rendered at the same physical heights; `design.md` §Orca Inspection Checklist is written with per-axis inspected verdicts (termination, coverage, collision freedom, interface placement/count, independent heights), each naming its layer and tap.
- Files allowed to read: `crates/pnp-cli/src/visual_debug.rs` (bounded ranges); existing visual-debug tests; generated manifests via delegated inspection only.
- Files allowed to edit (3): `tmp/visual-debug-support-family-tree.json`; `tmp/visual-debug-support-family-normal.json`; `docs/spec_packets/224-support-family-orca-closure/design.md`.
- Files explicitly out of bounds: `packet.spec.md` (AC text is amended in Step 7); generated PNGs as source edits; any test reading the Orca G-code.
- Expected sub-agent dispatches: Question: inspect matched-height PNGs for both families against the Orca renders and report per-axis verdicts; scope: `target/vd-support-family-*`; return: `FACT` plus manifest paths.
- Context cost: `M`
- Verification: `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-tree.json --output target/vd-support-family-tree --overwrite`; the same for the `-normal` request into `target/vd-support-family-normal`; `cargo test -p slicer-runtime --test integration support_family_closure`.
- Exit condition: two per-family requests exist and render distinct output, side-by-side Orca renders were inspected, and the checklist records a verdict per axis.

### Step 7: Paperwork — ACs, gap register, packet stubs, docs/07
- Task IDs: `TASK-335`
- Objective: amend the acceptance criteria to the delivered gate, create the gap register, stub the follow-on packets, and update implementation status.
- Precondition: Steps 0-6 landed with their evidence recorded.
- Postcondition: `packet.spec.md` AC-2/AC-3/AC-6 amended with the amendment recorded verbatim in place (the style used for AC-N2), pointing at the two per-family commands and the inspection-checklist gate, AC-2's fixture path corrected from `tmp/SupportTest.stl` to the authoritative `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`, and the stale claim about `missing_fixture_is_blocking` corrected to match Step 5's actual deletion; `docs/specs/support-parity-gap-register.md` exists listing every routed gap with its owning packet; stubs exist for **224a** (AGG rasterizer / `support_area_algorithm`), **225** (independent support-layer Z), **226** (base/interface patterns, `support_expansion`, `support_bottom_z_distance`), **227** (raft); `docs/07_implementation_status.md` carries the `TASK-335` row and the follow-on rows.
- Files allowed to read: `packet.spec.md`; `docs/07_implementation_status.md`; the gap notes produced in Steps 3-4.
- Files allowed to edit: `docs/spec_packets/224-support-family-orca-closure/packet.spec.md`; `docs/specs/support-parity-gap-register.md`; the four packet stubs; `docs/07_implementation_status.md` (via delegated status worker).
- Files explicitly out of bounds: all `src/**` and all test files.
- Expected sub-agent dispatches: Question: update `docs/07_implementation_status.md` rows for `TASK-335` and the follow-on packets; scope: that file; return: `SUMMARY`.
- Context cost: `S`
- Verification: `rg -q 'TASK-335' docs/07_implementation_status.md`; `rg -q '224a' docs/specs/support-parity-gap-register.md`; `cargo xtask check-deviations` if any deviation row was filed.
- Exit condition: no gap named in this packet is unrouted, and every AC reads as the gate actually delivered.

### Step 8: Close
- Task IDs: `TASK-335`
- Objective: regenerate the benchy golden last and run the full acceptance gates.
- Precondition: Steps 0-7 complete and **no further production change is pending**. The golden is regenerated only now, after all fixes.
- Postcondition: the benchy golden is regenerated via the sanctioned path (`SUPPORT_PLANNER_REGEN_GOLDEN=1`) with human approval, **renamed off `orca_parity`** to a regression-tripwire name, and carries a provenance header stating it is a PnP self-capture and **not** parity evidence; the workspace gates pass.
- Files allowed to read: the golden fixture and its owning test.
- Files allowed to edit (3): the benchy golden fixture; its owning test file (rename plus provenance); `docs/spec_packets/224-support-family-orca-closure/design.md` (closure note).
- Files explicitly out of bounds: everything else; no production `src/**` change may land in this step.
- Expected sub-agent dispatches: Question: run `cargo xtask test --summary --workspace`; scope: workspace; return: `FACT pass/fail` plus failing-test names only. Never absorb the full output.
- Context cost: `M`
- Verification: `cargo xtask check-literals`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask test --summary --workspace` (sub-agent, FACT return).
- Exit condition: all three gates pass and the renamed golden's provenance header disclaims parity.

## Per-Step Budget Roll-Up
| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | protect, guest check, re-baseline |
| Step 1 | M | tree `support_top_z_distance_mm` (RC-11) |
| Step 2 | M | interface layer counts, both families |
| Step 3 | M | tree density diagnosis (read-only) |
| Step 4 | S | config-key reconciliation, four modules |
| Step 5 | M | honest tests on tracked `resources/` models |
| Step 6 | M | inspection gate plus checklist |
| Step 7 | S | ACs, gap register, packet stubs, docs/07 |
| Step 8 | M | golden regeneration plus workspace gates |

No step is rated `L`.

## Packet Completion Gate
- All nine steps complete with their exit conditions met.
- **Correctness closure, not canonical completeness.** The packet closes when tree honours `support_top_z_distance_mm`, both families honour the interface layer-count keys, the tree-density root cause is written down and classified, the four support modules' config keys are reconciled, and the closure suite contains no assertion-free test or dead helper.
- **Parity gate:** structural invariants plus the written `/visual-debug` inspection checklist with side-by-side Orca renders, recorded in `design.md` §Orca Inspection Checklist. No test reads the Orca G-code. Extruding-move counts are not evidence.
- Every routed gap (base/interface patterns, `support_expansion`, `support_bottom_z_distance`, raft, independent support-layer Z, the AGG rasterizer, and the dead raft/`support_base_pattern` keys) appears in `docs/specs/support-parity-gap-register.md` against packet 224a/225/226/227.
- `packet.spec.md` AC-2/AC-3/AC-6 amended in place with the amendment recorded verbatim; every remaining AC command returns PASS as written.
- Benchy golden regenerated **last**, renamed off `orca_parity`, provenance header disclaiming parity.
- `cargo xtask check-literals`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo xtask test --summary --workspace` (sub-agent, FACT return) all pass.
- Assert packet 213 / `TASK-329` supersession with closure evidence, and either close `TASK-163b-orca-ref` using the existing references or record its precise external blocker.
- `docs/07_implementation_status.md` updated through a worker dispatch.

## Acceptance Ceremony
- Re-dispatch every AC and packet-level gate command.
- Inspect matched-height PNGs and manifest provenance side by side with the Orca renders — not just file existence.
- Confirm every number quoted in `design.md` was measured after a clean `cargo xtask build-guests --check`.
- Record remaining external and inherited blockers, and confirm each routed gap has a named owning packet.
