---
status: implemented
packet: 43-rev1_macro-prepass-segmentation-output-drain
supersedes: 43_macro-prepass-segmentation-output-drain
task_ids:
  - TASK-130
  - TASK-130a
  - TASK-130b
---

# 43-rev1_macro-prepass-segmentation-output-drain

## Goal

Land an end-to-end macro-arm proof for `PrePass::PaintSegmentation` and `PrePass::MeshSegmentation` (DEV-025 mismatch 3 closure) by adding **two** sibling test guests authored via `#[slicer_module]` and reverting `sdk-prepass-guest` to its pre-deviation single-stage form so previously-macro-faithful tests stop running through hand-rolled `wit_bindgen::generate!` glue.

This packet absorbs and corrects Packet 43, whose `design.md` rejected the two-crate alternative on scaffolding-economy grounds without first verifying that one `#[slicer_module]` impl block can host two stage methods. It cannot — `crates/slicer-macros/src/lib.rs:43-52` enforces single-stage per impl and lines 689/989/2024/2306 emit hardcoded module names that collide if applied twice in one crate. The Step 3 deviation (commit `0c4e8b2`) replaced `sdk-prepass-guest` with raw `wit_bindgen::generate!` to work around the constraint, which silently demoted **two existing tests** (`dispatch_tdd.rs` macro-path MeshAnalysis section and `macro_all_worlds_roundtrip_tdd.rs` prepass-world section) from macro-arm coverage to raw-bindgen coverage.

### Packet revision (in-flight, 2026-05-08): bounded macro fix + host layer-idx alignment

The original 43-rev1 design (and its predecessor 43) carried the locked assumption that `crates/slicer-macros/src/lib.rs` was unchanged after commit `46aed61`. Step 3 of this packet exposed a latent compilation bug in the paint_seg_arm landed by `46aed61`: the inline WIT for `world prepass-module` at `lib.rs:1317` declares only `use geometry.{ex-polygon}`, but the generated quote-block at `lib.rs:1814-1829` constructs WIT geometry using bare `Polygon { ... }` and `Point2 { ... }` names. Without `polygon` and `point2` at world scope, those names do not resolve and any `#[slicer_module]`-authored guest that exercises `run_paint_segmentation` fails to compile. The bug was latent in master because no macro guest had ever invoked the paint_seg_arm — packet 43 ducked it via raw `wit_bindgen::generate!` in `sdk-prepass-guest`, which is exactly what 43-rev1 reverts. This packet's scope is therefore expanded by a bounded two-hunk edit in `build_prepass_world_glue`: (1) line 1317 inline-WIT extended to `use geometry.{ex-polygon, polygon, point2};` and (2) two explicit Rust `use self::slicer::world_prepass::geometry::{Polygon, Point2};` statements in the `segmentation_helpers` quote block, mirroring the finalization-world pattern at lib.rs:998. The line-1317 fix alone was tested during Step 2.5 and is necessary but not sufficient — wit-bindgen 0.24 skips flat re-exports for world-level `use` items whose TypeInfo modes_of() returns empty, requiring the explicit Rust `use` statements as well. The paint_seg_arm quote-block at lines 1814-1829 stays byte-identical. Packet 42 (TASK-130c) closed DEV-025 mismatches 4 and 5 on 2026-05-08 by keeping `paint-value-input.custom` as `string`; AC-6/AC-7 are amended in this revision to match that contract instead of the pre-Packet-42 `{semantic, payload}` shape they were originally written against.

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

## Architecture Constraints (Locked Assumptions)

These are **invariants** the implementation must preserve. Verifying them is part of the activation gate.

1. **Macro is single-stage per impl block.** `crates/slicer-macros/src/lib.rs:43-52` enforces this with `compile_error!`. No `#[slicer_module(stage = "...")]` attribute argument exists; `detect_stage_methods()` at `lib.rs:106-119` is the only stage-selector and it iterates the impl methods, looking up names in `STAGES`. **Do not author one `#[slicer_module]` impl with multiple stage methods.**
2. **Macro hardcodes the export module name per world.** `__slicer_prepass_world_export` etc. Two `#[slicer_module]` impls in one crate that target the same world will fail to link with duplicate-symbol errors. **Therefore, paint-seg and mesh-seg sibling crates must be separate crates, not separate impls in one crate.**
3. **`PrepassModule` trait permits multi-stage** — `traits.rs:367-495` provides default `Ok(())` bodies for `run_mesh_analysis`, `run_paint_segmentation`, `run_mesh_segmentation`. A sibling crate can implement only `run_paint_segmentation` (or `run_mesh_segmentation`) and rely on defaults for the rest. **Use this — do not stub the unrelated stage methods explicitly.**
4. **The `#[slicer_module]` macro produces the `__slicer_prepass_world_export` boundary.** Tests asserting "this guest is macro-authored" must NOT contain `wit_bindgen::generate!` literal in source. The negative AC encodes this.
5. **SDK f64 mm → WIT i64 100-nm conversion is `× 10_000`.** Per `docs/08_coordinate_system.md`. Coordinate assertions in round-trip tests must round-trip with this scaling. (Already encoded in the test files from `0c4e8b2`.)
6. **WIT `layer-idx` is `s32` (not `u32`).** Cast `u32 → i32` is required at the macro arm boundary (already done in commit `46aed61`).
7. **Macro `build_prepass_world_glue` requires both an inline-WIT geometry import AND explicit Rust `use` statements for the segmentation helpers.** After the 2026-05-08 in-flight revision: (a) `lib.rs:1317` reads `use geometry.{ex-polygon, polygon, point2};` (declarative WIT-level intent), and (b) the `segmentation_helpers` quote block contains `use self::slicer::world_prepass::geometry::Polygon;` and `use self::slicer::world_prepass::geometry::Point2;` mirroring the finalization-world pattern at `lib.rs:998`. The line-1317 fix alone was tested during Step 2.5 and proved necessary but not sufficient — wit-bindgen 0.24 skips flat re-exports for world-level `use` items whose TypeInfo modes_of() returns empty, requiring the explicit Rust `use` statements as well. Without both, the existing paint_seg_arm quote-block at `lib.rs:1814-1829` (which constructs WIT geometry using bare `Polygon { ... }` and `Point2 { ... }` names) does not resolve and any macro-authored guest invoking `run_paint_segmentation` fails to compile. The bug was latent in master because no macro guest had ever invoked the paint_seg_arm — packet 43 ducked it with raw `wit_bindgen::generate!` in `sdk-prepass-guest`. Fixing it inside this packet keeps the audit trail honest: the same packet whose acceptance test catches the latent bug closes it. Total macro churn is < 20 lines; the paint_seg_arm quote-block stays byte-identical; no other macro arm is touched.
8. **`PaintValueInput::Custom` is a single-string tuple variant** — `crates/slicer-sdk/src/prepass_builders.rs:294-303` and `crates/slicer-ir/src/slice_ir.rs:189-199`, mirrored at the WIT layer in `wit/deps/ir-types.wit:46-51`. The pre-Packet-42 `{semantic, payload}` framing was retired by Packet 42 (TASK-130c) on 2026-05-08; AC-6 of this packet is amended to assert against `Custom("test-semantic|DEADBEEF")` (a byte-identifiable marker string), preserving the original AC's intent (no silent fallback to a built-in variant) while matching the actual contract.
9. **Empty `polygons` list is rejected by the host validator** — `crates/slicer-host/src/wit_host.rs:4089-4127` rejects `polygons.is_empty()`, empty `object_id`, empty `semantic`, and contour with `<3` points. The original AC-7 `empty_polygons` fixture (which expected silent success with an empty harvested region) was unrealizable. AC-7 is reframed in this packet to assert the silent path (no `fixture_case` configured → guest pushes zero regions → harvest is empty `PaintRegionIR`); AC-14 (force_push_failure) uses empty `polygons` as the canonical force-failure vector since it surfaces through both the host validator and the macro arm's `Err → fatal ModuleError` mapping.
10. **Host inline WIT must align with canonical `wit/deps/ir-types.wit` for `paint-region-entry.layer-index`.** After the 2026-05-08 packet revision (Step 2.6): `crates/slicer-host/src/wit_host.rs:298` and `:543` both declare `type layer-idx = s32;` (one per inline-WIT world block; were `u32`, drifted from canonical `wit/deps/ir-types.wit:8` `s32`). The four non-paint view records (seam-plan-entry, layer-plan-view-entry, region-segmentation-view-entry, support-geometry-view-entry) keep explicit `u32` because the macros crate WIT only uses the `layer-idx` alias for paint-region-entry — those four records remain `u32` in the macros crate WIT. The host validator at `wit_host.rs:4089-4127` now rejects negative `entry.layer_index` (preserving PaintRegionIR's `HashMap<u32, _>` invariant via boundary validation), and `dispatch.rs:harvest_paint_segmentation_ir` casts `entry.layer_index as u32` at the IR boundary. This drift was latent in master because no end-to-end test exercised `push-paint-region`; the new `sdk-prepass-paintseg-guest` is the first to do so and surfaced the wasmtime 43 component-linker s32/u32 mismatch. The `SupportPlanEntry.global_layer_index: i32` precedent (commit `1c19bc4`) confirms s32 is the project's direction for future raft-prefix layer indexing — keeping the WIT contract architecturally honest while the IR continues to validate non-negative at the boundary.

## Data and Contract Notes

- The `paint-segmentation-output::push-paint-region` WIT signature (verify via SUMMARY dispatch on `docs/03_wit_and_manifest.md` or `wit/` files):
  - region: `record { layer-idx: s32, polygons: list<ex-polygon>, paint-value: paint-value-input }`
  - returns: `result<_, error-record>`
- Coordinate conversion: SDK builder accepts f64 mm; WIT receives i64 100-nm. Multiply by `10_000`. Match the existing macro arm's conversion (lib.rs:1787-1822).
- `PaintValue::Custom { semantic, payload }` round-trips byte-for-byte under the wider transport landed in Packet 42. The custom_payload fixture validates this.

## Risks and Tradeoffs

- **Two more guest crates means two more `.component.wasm` builds.** Adds 10–15 s of incremental cargo build time per `bash test-guests/build-test-guests.sh` invocation. Acceptable.
- **Registry extension in `macro_all_worlds_roundtrip_tdd.rs` may need code shape work** — the file's loader pattern is not yet inspected (Step 1 dispatch will determine if extending it is one-line or requires a small refactor). If the latter, Step 8 may need to split.
- **Sibling crate workspace membership.** Existing `sdk-*-guest` crates declare an empty `[workspace]` block in their `Cargo.toml` (treating themselves as standalone). New siblings should mirror this. Verify via the template inspection in Step 3, do not silently break workspace resolution.
- **Revert window.** `git show 0c4e8b2^:...` produces the exact bytes; use `git checkout 0c4e8b2^ -- test-guests/sdk-prepass-guest/src/lib.rs` to apply, or copy the content into a Write call. The latter is preferable when only this one file needs to revert (avoids accidental broader checkout).

## Locked Assumptions and Invariants

The implementation must preserve these invariants. If any one is violated, the change is rejected.

1. `crates/slicer-macros/src/lib.rs` is unchanged after this packet **except for** the bounded two-hunk fix in `build_prepass_world_glue` introduced by Step 2.5: (a) line 1317 inline-WIT extended to `use geometry.{ex-polygon, polygon, point2};`, and (b) `use self::slicer::world_prepass::geometry::{Polygon, Point2};` (with a brief explanatory comment) added to the `segmentation_helpers` quote block, mirroring the finalization-world pattern at lib.rs:998. The paint_seg_arm quote-block at lines 1814-1829 and every other arm stay byte-identical to commit `46aed61`. `git diff --numstat crates/slicer-macros/src/lib.rs` after Step 2.5 must show total churn < 20 lines.
2. Each new sibling crate contains exactly one `#[slicer_module]` attribute and exactly one `impl PrepassModule for ...` block.
3. Neither new sibling crate contains `wit_bindgen::generate!`.
4. `test-guests/sdk-prepass-guest/src/lib.rs` matches its pre-`0c4e8b2` form byte-for-byte after revert.
5. `dispatch_tdd.rs` and `macro_all_worlds_roundtrip_tdd.rs` prepass cases stay green throughout the packet (no test deletions or `#[ignore]`).
6. No existing passing test is weakened (no assertion removed, no `assert!` → `eprintln!` regression).
7. Test discipline: targeted `cargo test -p slicer-host --test <file>` only; never `cargo test --workspace`.
