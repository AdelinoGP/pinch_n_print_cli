---
status: implemented
packet: 73_support-geometry-normalization
task_ids:
  - TASK-163c
---

# 73_support-geometry-normalization

## Goal

Give the `run-support-geometry` WIT export the same shape as its sibling prepass stages — `config: config-view` in, a `support-geometry-output` resource, `result<_, module-error>` out — so the tree-support planner finally receives real config and its fatals propagate instead of being swallowed.

## Problem Statement

`run-support-geometry` is the one prepass export that never received config and whose errors vanish — a latent bug, not a design choice. The macro down-converter (`crates/slicer-macros/src/lib.rs`, support arm ≈1825–1907) injects an **empty** `ConfigView` because the WIT export has no `config-view` parameter (it states this verbatim at ≈1831–1833), so the `support-planner` reads `support_enabled`, `support_raft_layers`, `tree_support_branch_diameter`, and every other key as a default — the tree-support planner has never honored user support config, and `support_enabled = false` cannot disable it. The same export returns a bare `support-geometry-output` record instead of `result<_, module-error>`, so the module's `Err(ModuleError::fatal(…))` (e.g. on an empty layer-plan, `support-planner/src/lib.rs:198`) is explicitly discarded (`let _ = out;`, ≈1900–1903). Every sibling prepass stage gets both `config-view` and a `result` channel; this one is an incomplete normalization. The SDK trait and module body are already in the correct shape, so the fix is confined to the WIT export and its host/macro glue — but it deliberately changes slicer behavior, which is why fixture re-baselining and a regression test are part of the slice.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Depends on packet 72: this edits `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` (created by 72) and reuses the shared `slicer:common` `module-error`. If 72 is not `implemented`, this packet cannot proceed.
- The new `support-geometry-output` resource must follow the sibling pattern (cf. `layer-plan-output { push-layer }`, `seam-planning-output { push-seam-plan }` in the same world) so the macro's existing resource-drain helpers apply with minimal new code.
- Behavior change is intentional and ABI-affecting (the export signature changes): both compiled sides must move together; the all-worlds roundtrip + this packet's integration tests are the proof.

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
