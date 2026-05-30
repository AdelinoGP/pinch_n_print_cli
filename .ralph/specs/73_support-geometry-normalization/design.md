# Design: 73_support-geometry-normalization

## Controlling Code Paths

- WIT export: `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — `run-support-geometry` (record-returning today) + the `support-geometry-output` / `support-plan-entry` records.
- Guest glue: `crates/slicer-macros/src/lib.rs` `build_prepass_world_glue` support arm (≈1825–1907) and the generated `fn run_support_geometry(...) -> SupportGeometryOutput` impl signature (≈1962–1968). The empty-`ConfigView` injection (≈1831–1833) and `let _ = out;` swallow (≈1900–1903) are the defects.
- Host dispatch: `crates/slicer-runtime/src/dispatch.rs` `PrePass::SupportGeometry` arm (≈975–1018) + `harvest_support_plan_ir` (≈1848). Today it calls `call_run_support_geometry(... no config ...)` and stashes the returned record via `store.data_mut().push_support_geometry_result(...)`.
- Host record-stash: `crates/slicer-runtime/src/wit_host.rs` `push_support_geometry_result` (≈1944).
- Already-correct (do NOT edit): SDK trait `crates/slicer-sdk/src/traits.rs` `run_support_geometry` (≈475–485); module `modules/core-modules/support-planner/src/lib.rs` `run_support_geometry` (≈184–254, already takes `output` + `config`, returns `Result`, calls `output.push_support_plan_entry`).
- Neighboring tests/fixtures: `prepass_support_geometry_tdd.rs`, `blackboard_support_geometry_slot_tdd.rs`, `benchy_end_to_end_tdd.rs`; module unit tests in `support-planner/src/lib.rs` (≈1116–1269) already pass a real `ConfigView`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Depends on packet 72: this edits `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` (created by 72) and reuses the shared `slicer:common` `module-error`. If 72 is not `implemented`, this packet cannot proceed.
- The new `support-geometry-output` resource must follow the sibling pattern (cf. `layer-plan-output { push-layer }`, `seam-planning-output { push-seam-plan }` in the same world) so the macro's existing resource-drain helpers apply with minimal new code.
- Behavior change is intentional and ABI-affecting (the export signature changes): both compiled sides must move together; the all-worlds roundtrip + this packet's integration tests are the proof.

## Code Change Surface

- Selected approach: full sibling parity for `run-support-geometry` — `config-view` in, output resource out, `result<_, module-error>`. Delete the macro down-converter so the support arm becomes the same shape as the other prepass arms; route the host through a builder + config handle like the other prepass stages; harvest from the drained resource.
- Exact surfaces expected to change:
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`: `run-support-geometry` signature + `support-geometry-output` record→resource.
  - `crates/slicer-macros/src/lib.rs`: support arm (≈1825–1907) + glue impl signature (≈1962–1968).
  - `crates/slicer-runtime/src/dispatch.rs`: support-geometry arm (≈975–1018) + `harvest_support_plan_ir` (≈1848).
  - `crates/slicer-runtime/src/wit_host.rs`: remove `push_support_geometry_result` (≈1944); add the `support-geometry-output` builder resource impl mirroring the other prepass output builders.
  - **New** `crates/slicer-runtime/tests/support_geometry_config_normalization_tdd.rs`.
- Rejected alternatives: "reshape the WIT but keep ignoring config" — rejected (self-defeating; would wire a `config-view` param then discard it). "Fix only the error channel, leave config" — rejected; config plumbing is the larger bug and both share the same root (no `config-view` on the export).

## Files in Scope (read + edit)

Primary edit surface is the WIT export + two glue files + the new test; the host builder-impl is a small mirror of existing ones.

- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — role: the contract; expected change: normalize `run-support-geometry`.
- `crates/slicer-macros/src/lib.rs` — role: guest glue; expected change: rewrite support arm, delete injection/swallow.
- `crates/slicer-runtime/src/dispatch.rs` — role: host dispatch + harvest; expected change: pass config + builder, read drained resource.
- `crates/slicer-runtime/src/wit_host.rs` — role: host builder resource; expected change: add output-builder impl, remove record-stash.
- `crates/slicer-runtime/tests/support_geometry_config_normalization_tdd.rs` — role: behavior regression; expected change: new file.

(Five files — one over the ≤3 target. Justified: the WIT export touches its two codegen consumers plus the host builder/harvest, which are inseparable for one ABI change; the test is new and independent. Splitting further would leave the build red between packets.)

## Read-Only Context

- `crates/slicer-sdk/src/traits.rs` — read `run_support_geometry` (≈475–485) only — purpose: confirm signature is unchanged (AC-5).
- `modules/core-modules/support-planner/src/lib.rs` — read `run_support_geometry` (≈184–254) + the existing unit tests (≈1116–1269) only — purpose: confirm module unchanged; reuse the test's config/layer-plan setup as the integration test's fixture shape.
- `crates/slicer-runtime/tests/prepass_support_geometry_tdd.rs` — read in full only if ≤300 lines — purpose: copy the live-guest dispatch harness pattern for the new test.
- `docs/02_ir_schemas.md` — `SupportPlanIR`/`support-plan-entry` field names — delegate SUMMARY.

## Out-of-Bounds Files

- `crates/slicer-runtime/src/{wit_host.rs,dispatch.rs}` and `crates/slicer-macros/src/lib.rs` **in full** — range-read only.
- `modules/core-modules/support-planner/src/lib.rs` algorithm body (≈257–1068) — read-only, never edit.
- `target/`, `Cargo.lock`, generated `.wasm` — never load; rebuild via `cargo xtask build-guests`.
- `OrcaSlicerDocumented/**` — not relevant (no parity surface); never load.

## Expected Sub-Agent Dispatches

- "Summarize `docs/02_ir_schemas.md` `SupportPlanIR` + `support-plan-entry` field names; return FACT (field list)" — purpose: Step 1 resource definition.
- "Run `cargo xtask build-guests` then `--check`; return FACT clean or `STALE:` list" — purpose: after Step 3.
- "Run `cargo test -p slicer-runtime --test support_geometry_config_normalization_tdd`; FACT pass/fail + assertion ≤20 lines" — purpose: AC-2/N1/N2.
- "Run `cargo test -p slicer-runtime --test prepass_support_geometry_tdd --test blackboard_support_geometry_slot_tdd --test benchy_end_to_end_tdd`; FACT pass/fail each" — purpose: regression after the behavior change.
- "Show how the `PrePass::SeamPlanning` arm in `dispatch.rs` pushes its output builder + config handle; return SNIPPETS ≤30 lines" — purpose: mirror the pattern for the support arm.

## Data and Contract Notes

- IR contracts touched: `SupportPlanIR` is now populated from a drained output resource rather than a returned record — same data, same `SupportPlanEntry` fields (`global_layer_index`, `object_id`, `region_id`, `branch_segments`); no IR field change.
- WIT boundary: `run-support-geometry` gains a `config-view` import use + an output resource + a `result<_, module-error>` return — an ABI change requiring both sides to move together.
- Determinism: unchanged — entry ordering is still the planner's top-to-bottom emission.

## Locked Assumptions and Invariants

- Default-config slices produce the **same** `SupportPlanIR` as before (the old empty-`ConfigView` already yielded the SDK defaults; default config equals those defaults) — locked by AC-6 + benchy regression. Any default-config output change must be investigated, not accepted.
- `support_enabled = false` now yields zero entries — newly locked by AC-N1 (this is the intended behavior change).
- Planner fatals propagate as `DispatchError` — newly locked by AC-N2.
- The SDK trait and module body are untouched — locked by AC-5.

## Risks and Tradeoffs

- **Fixture churn:** if benchy's profile sets non-default support params, its gcode changes. Mitigation: inspect the diff; re-baseline only after confirming it reflects honored config, and note it in the packet record.
- **AC-N2 harness cost:** forcing an empty layer-plan through the live dispatch may need a minimal blackboard fixture. Mitigation: reuse the `prepass_support_geometry_tdd` harness; if the empty-layer-plan path is hard to stage end-to-end, assert at the dispatch seam that a guest `Err` becomes `DispatchError` using the smallest fixture that reaches the planner.
- **Config-key spelling:** the planner reads snake_case keys (`support_enabled`, `support_raft_layers`); the test config must use those exact keys (per `CLAUDE.md` Config Key Naming Convention).

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 4 — the live-guest integration test, which needs a real support-planner guest and a config-bearing dispatch harness).
- Highest-risk dispatch: the regression-test batch — must return FACT pass/fail per test, never full logs.

## Open Questions

- `[FWD]` Whether benchy fixtures need re-baselining depends on whether benchy's profile sets non-default support params — resolved by inspecting the Step 4 regression diff; not activation-blocking.
- `[BLOCK]` None of this packet's own content. The only blocker is the external prerequisite: packet 72 must be `implemented` first (stated in Prerequisites).
