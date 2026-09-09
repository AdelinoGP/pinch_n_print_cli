---
status: implemented
packet: 198-literal-sweep-sdk-modules
task_ids:
  - TASK-320
---

# 198-literal-sweep-sdk-modules

## Goal

Convert every `cargo xtask check-literals` violation in `slicer-sdk` and `modules/core-modules/*/tests` test code to FRU-over-a-base (sdk fixtures / `Default`) or a reasoned waiver — gating the sdk's own not-yet-gated fixture-consuming test files via `[[test]] required-features = ["test"]` entries — so `cargo xtask check-literals crates/slicer-sdk modules/core-modules` exits 0 with every suite green, test counts unchanged, and guests rebuilt fresh after the sdk manifest edit.

## Problem Statement

Queue row #5 of `docs/specs/_OLD/struct-literal-churn-gate-plan.md`. The guest-side area: `slicer-sdk`'s own test files and the 21 core modules' native test dirs. Sizing estimates (measured 2026-08-07, re-derive from the Step-1 report): 8 sdk test/bench-scope files with `Point3WithWidth` literals, 2 with `LayerCollectionIR`, 1 with `PrintEntity`, 3 with `WallLoop`, 1 with `OrderedEntityView`; 26 module test files across 10 modules (`seam-placer` 7, `infill-linker` 4, `path-optimization-default` 3, `wipe-tower` 3, `fuzzy-skin` 2, `skirt-brim` 2, `support-planner` 2, `arachne-perimeters` 1, `overhang-classifier-default` 1, `part-cooling` 1). One coherent slice: both sides consume the same `slicer_sdk::test_support` fixture home (ADR-0004/0054 "single IR-fixture home"), and the sdk manifest gating decision affects both.

## Architecture Constraints

- **Sdk manifest gating is the packet's only manifest edit and its only guest-input edit (grounded 2026-08-07).** `crates/slicer-sdk/tests/*.rs` files referencing `test_support` MUST be `[[test]]`-gated with `required-features = ["test"]`, because integration-test binaries do not see `cfg(test)` on the linked sdk lib. Non-gated files whose only violations involve `Default`-able types (`Point3WithWidth`, `GlobalLayer`, `LayerCollectionIR`) convert with plain FRU and stay ungated (grounded candidate: `finalization_module_tdd.rs`). Files needing class-b fixtures get gated (grounded candidates: `layer_module_tdd.rs`, `finalization_builder_tdd.rs`).
- **Module manifests are already correct.** All 21 `modules/core-modules/*/Cargo.toml` carry `[dev-dependencies] slicer-sdk = { path = "../../../crates/slicer-sdk", features = ["test"] }` (verified 2026-08-07, e.g. `arachne-perimeters`); a dev-dep feature applies to every test target, so module manifests need zero `required-features` entries and zero edits (AC-N4). Guests never enable `test`: it appears only under `[dev-dependencies]`, which wasm guest builds never resolve (ADR-0004 amendment).
- **Fixture bases stay exhaustive-with-waiver.** `crates/slicer-sdk/src/test_support/fixtures.rs` literals are deliberate propagation checkpoints — when a watched type gains a field, the fixture is where the compiler forces one conscious decision. If the Step-1 report flags them, add `// exhaustive: fixture base is the single propagation checkpoint for this type` rather than FRU. Fixture signatures and returned values are packet-195 contract; never adjust them here.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- The guest-feeding path in this packet is exactly `crates/slicer-sdk/Cargo.toml` (collected by `shared_input_paths` alongside shared-crate `src/`; verified in `xtask/src/build_guests.rs` 2026-08-07). `STALE:` for all guests immediately after the manifest edit is EXPECTED — the mtime check cannot know a `[[test]]` section is guest-inert. Rebuild, then gate clean (AC-5). Sdk `tests/**` edits do NOT trip the gate (only `src/`, `Cargo.toml`, `build.rs` are collected).

## Data and Contract Notes

- IR/manifest contracts: untouched; conversions value-identical by construction. Fixture base values are packet-195 contract (`print_entity_base`: `entity_id 0`, 1-point path, `speed_factor 1.0`; `wall_loop_base`: `perimeter_index 0`, widths length-matched to points, role mapping Outer→OuterWall / ThinWall→ThinWall / else InnerWall; `ordered_entity_view_base`: `point_count 2`, default endpoints) — conversions must omit exactly the fields equal to these values, nothing else.
- WIT boundary: untouched; guests never see `feature = "test"`.
- Determinism/scheduler constraints: none.

## Locked Assumptions and Invariants

- Test counts, assert counts, and every constructed value invariant; construction syntax (plus manifest gating) only.
- The six no-`Default` locks hold (AC-N1, including `OrderedEntityView`).
- Module manifests unchanged (AC-N4); sdk manifest gains only `[[test]]` entries.
- All sdk suite runs in this packet use `--features test`; the bare-run check (AC-7) is the sole exception, by design.

## Risks and Tradeoffs

- The gating edit widens the sdk's bare-run blind spot (more binaries skipped without `--features test`). Accepted: the convention and its CLAUDE.md reconciliation rule already exist; exported to packet 199's notes.
- Full-guest rebuild after the manifest edit is the packet's slowest operation; it is the guest-staleness rebuild tax the plan's locked decision 3(b) explicitly accepted. Budget exactly one rebuild (manifest edits complete before it runs).
- Module test helper fns sometimes encode geometry meaning in every field; over-eager omission can obscure intent even when values equal the base. The conversion rule's "meaningful fields stay spelled" clause is the guard; reviewers check spelled-field choices, not just greenness.
- Checker defects at module scale (e.g. macro-heavy module tests) are deviations against packet 194, not local patches.
