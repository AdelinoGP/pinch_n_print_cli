# Implementation Plan: 73_support-geometry-normalization

## Execution Rules

- One atomic step at a time; each maps back to `TASK-166` (WIT-export layer).
- TDD: Step 4 authors the three regression tests; AC-N1/AC-N2 must fail against the *old* glue and pass after Steps 1–3.
- Both compiled sides move together — never trust a `support_geometry`/`prepass` test without a fresh guest rebuild after Step 3.
- Prerequisite: packet 72 is `implemented` (canonical `world-prepass.wit` exists; shared `module-error` available).

## Steps

### Step 1: Normalize the WIT export

- Task IDs: `TASK-166`
- Objective: in `crates/slicer-schema/wit/world-prepass.wit`, give `run-support-geometry` `config: config-view`, turn `support-geometry-output` into a `resource { push-support-plan-entry: func(entry: support-plan-entry) -> result<_, string> }`, and return `result<_, module-error>` (shared `slicer:common`).
- Precondition: packet 72 `implemented`.
- Postcondition: AC-1 passes; workspace still type-checks once Steps 2–3 land.
- Files allowed to read: the sibling `seam-planning-output` / `layer-plan-output` resource defs in the same file.
- Files allowed to edit (≤3): `crates/slicer-schema/wit/world-prepass.wit`.
- Files explicitly out-of-bounds: `slicer-macros`/`wit_host.rs` in full.
- Expected sub-agent dispatches:
  - `Summarize docs/02_ir_schemas.md SupportPlanIR + support-plan-entry field names; return FACT field list.`
- Context cost: `S`
- Authoritative docs: `docs/03_wit_and_manifest.md` (delegate SUMMARY of the prepass export); `docs/02_ir_schemas.md` (field names).
- OrcaSlicer refs: none.
- Verification: AC-1 grep — dispatch as FACT `EXIT=0`.
- Exit condition: AC-1 returns `EXIT=0`.

### Step 2: Rewrite the macro support arm

- Task IDs: `TASK-166`
- Objective: pass the real `config-view`, drain the SDK `SupportGeometryOutput` builder into the new WIT resource, propagate the `Result` via `__slicer_error_out`; delete the empty-`ConfigView` injection (≈1831–1833), the `let _ = out;` swallow (≈1900–1903), and the `-> SupportGeometryOutput` glue signature (≈1962–1968).
- Precondition: Step 1 complete.
- Postcondition: AC-3 passes; guest compiles against the new WIT.
- Files allowed to read: `crates/slicer-macros/src/lib.rs` (support arm ≈1825–1916 + glue sig ≈1955–1970 only); an existing prepass drain helper for the pattern.
- Files allowed to edit (≤3): `crates/slicer-macros/src/lib.rs`.
- Files explicitly out-of-bounds: macro file in full; `wit_host.rs`.
- Expected sub-agent dispatches:
  - `Run cargo build -p <a support-planner-consuming guest crate>; return FACT pass/fail + first error ≤20 lines.`
- Context cost: `M`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC-3 grep (`EXIT=0`); guest crate builds.
- Exit condition: AC-3 `EXIT=0` and the guest compiles.

### Step 3: Rework host dispatch + harvest + builder

- Task IDs: `TASK-166`
- Objective: in `dispatch.rs` support arm (≈975–1018) push a `support-geometry-output` resource + `config_handle` and consume `result<_, ModuleError>`; reshape `harvest_support_plan_ir` (≈1848) to read the drained resource; add the builder-resource impl in `wit_host.rs` and remove `push_support_geometry_result` (≈1944).
- Precondition: Steps 1–2 complete.
- Postcondition: AC-4 passes; workspace builds; guests rebuild clean.
- Files allowed to read: `dispatch.rs` (≈975–1018, ≈1848 only); `wit_host.rs` (≈1944 + a sibling prepass output-builder impl for the pattern); the `SeamPlanning` arm for the push pattern.
- Files allowed to edit (≤3): `crates/slicer-runtime/src/dispatch.rs`, `crates/slicer-runtime/src/wit_host.rs`.
- Files explicitly out-of-bounds: both files in full — fix by compiler error.
- Expected sub-agent dispatches:
  - `Show how the PrePass::SeamPlanning arm pushes its output builder + config handle; return SNIPPETS ≤30 lines.`
  - `Run cargo build -p slicer-runtime; FACT pass/fail + first error ≤20 lines.`
  - `Run cargo xtask build-guests then --check; FACT clean or STALE: list.`
- Context cost: `M`
- Authoritative docs: `docs/04_host_scheduler.md` (delegate; error→DispatchError mapping).
- OrcaSlicer refs: none.
- Verification: AC-4 grep (`EXIT=0`); `cargo build -p slicer-runtime`; guests rebuilt.
- Exit condition: AC-4 `EXIT=0`, host builds, `build-guests --check` clean.

### Step 4: Regression test + fixture re-baseline

- Task IDs: `TASK-166`
- Objective: add `support_geometry_config_normalization_tdd.rs` with `raft_layers_config_is_honored`, `support_disabled_emits_no_plan`, `planner_fatal_surfaces_as_dispatch_error`; re-run existing support/benchy tests and re-baseline fixtures only if default-config output changed (after inspection).
- Precondition: Steps 1–3 complete; guests rebuilt.
- Postcondition: AC-2, AC-6, AC-7, AC-N1, AC-N2 pass; AC-5 still holds.
- Files allowed to read: `prepass_support_geometry_tdd.rs` (harness pattern, ≤300 lines); `support-planner/src/lib.rs` tests (≈1116–1269) for config/layer-plan fixture shapes.
- Files allowed to edit (≤3): `crates/slicer-runtime/tests/support_geometry_config_normalization_tdd.rs` (new); affected fixture files **only if** a diff is confirmed.
- Files explicitly out-of-bounds: support-planner algorithm body; golden fixtures (regenerate only after inspecting the diff, never blind).
- Expected sub-agent dispatches:
  - `Run cargo test -p slicer-runtime --test support_geometry_config_normalization_tdd; FACT pass/fail + assertion ≤20 lines.`
  - `Run cargo test -p slicer-runtime --test prepass_support_geometry_tdd --test blackboard_support_geometry_slot_tdd --test benchy_end_to_end_tdd; FACT pass/fail each.`
  - `Run cargo test -p support-planner; FACT pass/fail (confirm module unit tests unchanged).`
- Context cost: `M`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC-2/AC-N1/AC-N2 (`support_geometry_config_normalization_tdd`); AC-6 (`prepass_support_geometry_tdd`); AC-5 grep; AC-7 (`build-guests --check`).
- Exit condition: the new test's three cases pass; AC-6 + AC-5 + AC-7 green; any fixture change documented.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | one WIT export edit |
| Step 2 | M | macro support arm rewrite |
| Step 3 | M | host dispatch/harvest/builder; name churn |
| Step 4 | M | live-guest integration test + fixture inspection |

Aggregate: `M`. No step is `L`.

## Packet Completion Gate

- All steps complete; every exit condition met.
- AC-1…AC-7 + AC-N1 + AC-N2 dispatched and PASS; both Doc Impact greps hit.
- `cargo check --workspace` + `cargo clippy --workspace -- -D warnings` green.
- Guests rebuilt; `cargo xtask build-guests --check` clean.
- Any re-baselined fixture has a recorded one-line rationale (what config now flows).
- `docs/07_implementation_status.md` updated (worker dispatch) noting the WIT-boundary config/error fix.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm the three gate commands are green.
- Confirm default-config slicer output is unchanged (AC-6 + benchy) and the only behavior deltas are the intended `support_enabled`/config-honoring and error-surfacing ones.
- Confirm implementer peak context stayed < 70%; if not, log it as a packet-authoring lesson.
