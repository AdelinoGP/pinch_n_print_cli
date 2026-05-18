# Requirements: 43-rev1_macro-prepass-segmentation-output-drain

## Packet Metadata

- Slug: `43-rev1_macro-prepass-segmentation-output-drain`
- Status: `implemented`
- Supersedes: `43_macro-prepass-segmentation-output-drain`
- Task IDs: `TASK-130`, `TASK-130a`, `TASK-130b`
- Backlog source: `docs/07_implementation_status.md`

## Problem Statement

DEV-025 mismatch 3 ("PaintSegmentation output drain — non-functional") was meant to close in Packet 43. Two of the original packet's three implementation steps landed in master:

- Commit `46aed61` drained `PaintSegmentationOutput::regions()` through `paint-segmentation-output::push-paint-region` in `crates/slicer-macros/src/lib.rs:1787-1822`. **This is correct and stays.**
- Commit `0c4e8b2` added the round-trip TDD test files (`macro_paint_segmentation_output_roundtrip_tdd.rs` and `macro_mesh_segmentation_output_roundtrip_tdd.rs`). **These are correct and stay** (they will be retargeted at the load path).

The third step — extending the macro test guest — was blocked by an architectural fact the original `design.md` failed to verify: `#[slicer_module]` is single-stage per impl block. The macro at `crates/slicer-macros/src/lib.rs:43-52` raises a `compile_error!` when more than one stage method is detected, and the worlds module names (`__slicer_prepass_world_export` at line 2024 et al.) are hardcoded — two `#[slicer_module]` impls in one crate would emit duplicate symbols and fail to link.

The Step 3 worker rewrote `test-guests/sdk-prepass-guest/src/lib.rs` to use raw `wit_bindgen::generate!` to host all three prepass stages in one crate. This rewrite was committed in `0c4e8b2`, and it has two consequences:

1. **The new round-trip tests no longer exercise the macro arm.** They call into the host validator and harvest — that's still useful — but the bytes coming over the WIT boundary are emitted by hand-rolled `wit_bindgen::generate!` glue, not by `#[slicer_module]`-emitted code. AC-2/3/4/5 of the original packet would test only the host side, not the macro arm. DEV-025 mismatch 3 closure becomes a syntactic claim, not an end-to-end proof.
2. **Two pre-existing tests were silently demoted from macro coverage to raw-bindgen coverage.** `crates/slicer-host/tests/dispatch_tdd.rs:6076-6260` is doc-commented as proving "the macro-authored `PrePass::MeshAnalysis` arm" and `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs:232-296` is doc-commented as proving "Each guest under `test-guests/sdk-*-guest/` is authored purely via `#[slicer_module]`". Both still pass (the WIT contract is honored either way), but neither now tests what its name claims.

This revision packet:

- Reverts `test-guests/sdk-prepass-guest/src/lib.rs` to its pre-`0c4e8b2` `#[slicer_module] impl PrepassModule` (MeshAnalysis-only) form, restoring macro coverage for the two demoted tests.
- Adds two sibling crates — `sdk-prepass-paintseg-guest` and `sdk-prepass-meshseg-guest` — each authored via `#[slicer_module]`, each implementing exactly one of the missing prepass stages, retargets the new round-trip tests at them, and registers them in the existing macro-arm-proof loaders so the proof extends automatically.
- Documents the macro single-stage-per-impl constraint in `docs/05_module_sdk.md` so future packets do not repeat the original packet 43's planning mistake.

## Task Mapping

- **TASK-130** ("Finish the `#[slicer_module]` prepass segmentation bridge for macro-authored modules. Covers DEV-025.")
  → Closes when AC-1 through AC-15 (this packet's spec) are all green.
- **TASK-130a** ("Drain `PaintSegmentationOutput` back through WIT `push-paint-region`...")
  → Already drained in `46aed61`; closes when the round-trip ACs (AC-5, AC-6, AC-7, AC-14) prove the drain works end-to-end through `#[slicer_module]`-emitted code.
- **TASK-130b** ("Add end-to-end macro-path regression tests proving `MeshSegmentation` and `PaintSegmentation` round-trip real data through WIT.")
  → Closes when the round-trip tests at `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` and `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` load `#[slicer_module]`-authored sibling guests and pass.

## In Scope

- `crates/slicer-macros/src/lib.rs` (bounded two-hunk fix in `build_prepass_world_glue` added in 2026-05-08 packet revision; total churn < 20 lines: hunk 1 — line 1317 inline-WIT extended from `use geometry.{ex-polygon};` to `use geometry.{ex-polygon, polygon, point2};`; hunk 2 — explicit `use self::slicer::world_prepass::geometry::{Polygon, Point2};` added to the `segmentation_helpers` quote block, mirroring the finalization-world pattern at lib.rs:998. Discovered during the original Step 3 attempt; without it, the existing paint_seg_arm quote-block at lines 1814-1829 fails to resolve bare `Polygon` and `Point2` and any macro-authored guest invoking `run_paint_segmentation` cannot compile. The line-1317 fix alone was tested and proved necessary but not sufficient — wit-bindgen 0.24 skips flat re-exports for world-level `use` items whose TypeInfo modes_of() returns empty.)
- `crates/slicer-host/src/wit_host.rs` (Step 2.6, added in 2026-05-08 packet revision): align the host inline-WIT alias to canonical (`type layer-idx = s32;` at line 543, was `u32`), keep the four non-paint view records on explicit `u32` (the macros crate WIT does not use `layer-idx` for them), and add `entry.layer_index < 0` rejection in the host push_paint_region validator at lines 4089-4127.
- `crates/slicer-host/src/dispatch.rs` (Step 2.6): cast `entry.layer_index as u32` at the IR boundary in `harvest_paint_segmentation_ir` (line ~1984). Preserves PaintRegionIR's `HashMap<u32, _>` shape — no IR contract change.
- `test-guests/sdk-prepass-guest/src/lib.rs` (revert)
- `test-guests/sdk-prepass-guest.component.wasm` (rebuild)
- `test-guests/sdk-prepass-paintseg-guest/Cargo.toml` (new)
- `test-guests/sdk-prepass-paintseg-guest/src/lib.rs` (new)
- `test-guests/sdk-prepass-meshseg-guest/Cargo.toml` (new)
- `test-guests/sdk-prepass-meshseg-guest/src/lib.rs` (new)
- `test-guests/build-test-guests.sh` (GUESTS array)
- `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (retarget load path)
- `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (retarget load path)
- `crates/slicer-host/tests/guest_fixture_freshness_tdd.rs` (extend GUESTS table)
- `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` (extend registry to cover the two new siblings)
- `Cargo.toml` workspace `members` if needed (verify before editing — sibling crates may use the existing `[workspace]` declaration in their own `Cargo.toml` like `sdk-layer-pathopt-guest` does).
- `docs/05_module_sdk.md` (add Single-Stage-Per-Impl section)
- `docs/07_implementation_status.md` (close TASK-130/130a/130b — via worker dispatch)
- `docs/DEVIATION_LOG.md` (close DEV-025 mismatch 3)
- `docs/14_deviation_audit_history.md` (cross-reference closure)
- `.ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md` (mark superseded — only allowed cross-packet edit)

## Out of Scope

- Any change to `crates/slicer-macros/src/lib.rs` BEYOND the bounded two-hunk edit in `build_prepass_world_glue` (line 1317 inline-WIT extension + `use self::slicer::world_prepass::geometry::{Polygon, Point2};` block in `segmentation_helpers`). The paint_seg_arm quote-block (lines 1787-1849) and every other macro arm stay byte-identical to commit `46aed61`.
- Multi-stage `#[slicer_module]` support (unique export module names per stage). That work belongs in a separate future packet.
- Any WIT file changes.
- `crates/slicer-sdk/` — trait definitions stay as-is.
- Host-side validators or the harvest path.
- `test-guests/prepass-guest/` (raw-bindgen reference; deliberately stays raw).
- Other `.ralph/specs/` packet directories.
- `docs/00_project_overview.md`, `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` — not affected by this scaffolding-only change.

## Authoritative Docs

- `docs/05_module_sdk.md` — read directly (one section will be added).
- `docs/03_wit_and_manifest.md` — delegate SUMMARY for `paint-segmentation-output::push-paint-region` signature only.
- `docs/02_ir_schemas.md` — delegate SUMMARY for `PaintSegmentationIR` and `MeshSegmentationIR` field shapes.
- `docs/07_implementation_status.md` — delegate ALL reads/edits.
- `docs/DEVIATION_LOG.md` — delegate SNIPPET fetch for DEV-025.
- `docs/14_deviation_audit_history.md` — delegate SNIPPET fetch.
- `docs/08_coordinate_system.md` — delegate FACT for the SDK-mm × 10_000 → WIT-100-nm conversion.

## OrcaSlicer Reference Obligations

- None. This packet contains no algorithmic behavior change. Parity for `PaintSegmentation` and `MeshSegmentation` semantics is owned by other packets and modules; this packet only proves the macro-authored guest can emit a region/mark through the existing WIT contract.

## Acceptance Summary

The packet is complete when:

1. The reverted `sdk-prepass-guest` matches its pre-`0c4e8b2` source byte-for-byte and rebuilds without errors.
2. The bounded two-hunk macro fix in `build_prepass_world_glue` is applied (line 1317 inline-WIT + `use self::slicer::world_prepass::geometry::{Polygon, Point2};` in segmentation_helpers, mirroring lib.rs:998); `cargo build --workspace` passes; `git diff --numstat crates/slicer-macros/src/lib.rs` reports total churn < 20 lines; the paint_seg_arm quote-block at 1814-1829 stays byte-identical. (Added in the 2026-05-08 packet revision.)
3. Two sibling crates exist, each containing exactly one `#[slicer_module] impl PrepassModule` block with exactly the named methods. Neither contains `wit_bindgen::generate!`.
4. The build script emits the two new `.component.wasm` files.
5. Both round-trip TDD files load the matching sibling and all 11 tests pass (10 paint + 1 mesh). The paintseg fixtures are `hole_bearing`, `custom_payload`, `force_push_failure`, plus a default no-op (the original `empty_polygons` fixture was retired in the 2026-05-08 revision since the host validator rejects empty polygon lists).
6. `PaintValue::Custom("test-semantic|DEADBEEF")` round-trips byte-for-byte through the macro arm and host harvest (AC-6 amended in the 2026-05-08 revision; `Custom` is a single-string tuple variant per Packet 42's TASK-130c).
7. Push failure surfaces as a fatal `ModuleError` code 10 when the guest pushes a `polygons: vec![]` entry (negative test; host validator rejection vector).
8. `dispatch_tdd.rs` macro-path MeshAnalysis tests still pass against the reverted `sdk-prepass-guest` (regression-defense — proves the silent demotion is undone).
9. `macro_all_worlds_roundtrip_tdd.rs` prepass tests pass and its registry extends to cover the two new siblings (so the macro-arm proof loop catches any future deviation).
10. `guest_fixture_freshness_tdd.rs` registry includes the two new siblings.
11. `docs/05_module_sdk.md` records the single-stage-per-impl constraint.
12. `docs/07_implementation_status.md` shows TASK-130/130a/130b closed.
13. `docs/DEVIATION_LOG.md` shows DEV-025 mismatch 3 closed.
14. Original packet 43 is marked `status: superseded`.

## Cross-Packet Dependencies

- **Reopens**: Packet 43. Original `design.md` rejected the two-crate alternative without verifying the macro single-stage constraint; this revision selects exactly that alternative and adds a regression-defense layer.
- **Constraint discovered during 43**: The macro is single-stage per impl block. The constraint is documented in this packet (`design.md` Locked Assumptions and the docs/05 update) so the next packet that touches macro guest scaffolding starts with that as a given.

## Verification Commands

Targeted verification (use these for per-step adjudication):

- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- `bash test-guests/build-test-guests.sh`
- `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd`
- `cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd`
- `cargo test -p slicer-host --test dispatch_tdd macro_path`
- `cargo test -p slicer-host --test macro_all_worlds_roundtrip_tdd prepass`
- `cargo test -p slicer-host --test guest_fixture_freshness_tdd`

`cargo test --workspace` is **not** required at packet close — no contract or scheduler change. The targeted suite above is sufficient.

## Step Completion Expectations

Each implementation step in `implementation-plan.md` declares files-allowed-to-read, files-allowed-to-edit (≤ 3), expected sub-agent dispatches, context cost (S/M; never L), and a falsifying check or exit condition. Step boundaries are non-negotiable; no step may load OrcaSlicer source, generated WIT bindings, or `target/` artifacts.

## Context Discipline Notes

- Read budget: 60% (≈ 120 k). Stop reading at 60%, hand off at 85%.
- The pre-deviation `sdk-prepass-guest/src/lib.rs` source is recovered via `git show 0c4e8b2^:test-guests/sdk-prepass-guest/src/lib.rs` — do not load the working-tree (raw-bindgen) version when authoring the revert.
- Use `sdk-finalization-guest` and `sdk-layer-pathopt-guest` as templates when authoring the two new siblings (their `Cargo.toml` and `src/lib.rs` shapes are minimal and proven).
- The macro source (`crates/slicer-macros/src/lib.rs`, > 2 300 lines) is **out of bounds** for direct reading in this packet. If a sub-step needs to verify macro behavior, dispatch a SNIPPET sub-agent with explicit line-range scope.
