---
status: implemented
packet: layer-module-preparation_08_lightning-migration
task_ids:
  - TASK-577
---

# layer-module-preparation_08_lightning-migration

## Goal

Move lightning planning out of the host PrePass and into the selected `com.core.lightning-infill` artifact. The planner and its two plan types relocate into the module (`modules/core-modules/lightning-infill/src/kernel/`), a NET-NEW module-side adapter builds the planner's inputs from typed preparation data, preparation plans the trees from those committed inputs, fresh ordinary `Layer::Infill` calls consume module-private pieces, a typed `planning_geometry` projection publishes the final plan, and the legacy host producer, public `LightningTreeIR`/`LightningTreeEntry`/`PrePass::LightningTreeGen`, the `lightning-tree-segments` accessor and the `slicer:lightning-queries` WIT package are deleted. Correctness is **recorded behavioural equivalence to the live producer, captured before removal**: this packet moves ownership and changes no output.

## Problem Statement

Lightning planning currently runs in the host as a PrePass builtin: `LIGHTNING_TREE_PRODUCER` commits a public `LightningTreeIR` into the Blackboard, and the consuming module reads it back through the `lightning-tree-segments` WIT method and the `PaintRegionLayerView::lightning_tree_segments_for` accessor. That splits one algorithm's ownership across a host producer, a public IR, a scheduler stage and a guest accessor, and it makes the plan's lifetime and identity unrelated to the artifact that consumes it.

ADR-0066 already decides that preparation-owning artifacts plan and publish privately, with the host owning transport/access/lifetime only, and [ADR-0074](../../adr/0074-lightning-migration-on-recorded-equivalence.md) records that the ownership move is authorized by recorded equivalence while a strategy change still needs the gates. Rows 01–06 built that capability. What remains is the mechanical part: relocate the existing planner into the consuming artifact behind preparation, switch the consumer to private pieces, publish a typed projection, and delete the legacy contract.

Two earlier attempts to author this packet failed for the same reason: they made the migration's correctness depend on a *new scientific result* (Gate A/B). That dependency is unsatisfiable in this tree — row 07 recorded `Gate A outcome: fail`, and row 09's `feasibility-kernel` contains empty `planner.rs`/`tree.rs` stubs, so Gate B never had a planner to execute. The rework replaces that dependency with behavioural equivalence, which is falsifiable now and is the property a refactor actually needs.

## Architecture Constraints

- Same selected artifact and frozen identity; one `prepare_print` per owner before any consumer. Host owns transport/access/lifetime only.
- No `LayerArena` survives preparation. `retained_bytes == 0` after success, failure and cancellation. Disposal reports `Busy` for active guards; callers join, never spin or revoke.
- **Planning numeric logic is frozen, and the permitted deltas are enumerated.** The relocated code may differ from `crates/slicer-core/src/algos/lightning/` only in: (a) file location; (b) the plan-type import — `kernel::plan::{KernelPlan, PlanEntry}` instead of `slicer_ir::{LightningTreeIR, LightningTreeEntry}`; (c) `crate::` → `slicer_core::` on the ungated helper paths listed above; (d) the error type's own name and path; (e) the NET-NEW `planning_input.rs` adapter, which supplies inputs and changes no arithmetic; (f) doc comments that named the moved types now name `KernelPlan`/`PlanEntry`, because AC-5 greps the module's own `src/` as a public-name leak check — comment text only, no code token changes. Everything else in `generator.rs`, `layer.rs`, `tree_node.rs`, `distance_field.rs`, `error.rs` and the computation in `mod.rs` — layer iteration order, domain and parameter derivation, grounding, clipping, and the `sparse_fill_holder` branch — is unchanged. Any seventh difference is a stop, not a review note. AC-2/AC-3 are the proof, not the intent.
- **Deletion order is compile-forced.** The public plan types leave `crates/slicer-ir/src/slice_ir.rs` only after their last consumer is gone: the host producer (Step 10b) and then the host algorithm copy (Step 10c), which is why the IR step (12a) follows them. No step deletes the IR while `crates/slicer-core/src/algos/lightning/mod.rs` still returns it; that would be a compile-fatal contradiction, not a migration.
- After retirement, nothing under `crates/slicer-runtime/**` or `crates/slicer-wasm-host/**` references the lightning kernel; the only production caller is the module.
- No speculative public version bump. Deleting the WIT package is an affected-contract change: regenerate affected bindings and rebuild guests.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- New translated files carry the header from `docs/ORCASLICER_ATTRIBUTION.md`. This packet relocates in-tree code and translates nothing new.
