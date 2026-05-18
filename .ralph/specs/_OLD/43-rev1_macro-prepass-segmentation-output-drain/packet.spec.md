---
status: implemented
packet: 43-rev1_macro-prepass-segmentation-output-drain
supersedes: 43_macro-prepass-segmentation-output-drain
task_ids:
  - TASK-130
  - TASK-130a
  - TASK-130b
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 43-rev1_macro-prepass-segmentation-output-drain

## Goal

Land an end-to-end macro-arm proof for `PrePass::PaintSegmentation` and `PrePass::MeshSegmentation` (DEV-025 mismatch 3 closure) by adding **two** sibling test guests authored via `#[slicer_module]` and reverting `sdk-prepass-guest` to its pre-deviation single-stage form so previously-macro-faithful tests stop running through hand-rolled `wit_bindgen::generate!` glue.

This packet absorbs and corrects Packet 43, whose `design.md` rejected the two-crate alternative on scaffolding-economy grounds without first verifying that one `#[slicer_module]` impl block can host two stage methods. It cannot — `crates/slicer-macros/src/lib.rs:43-52` enforces single-stage per impl and lines 689/989/2024/2306 emit hardcoded module names that collide if applied twice in one crate. The Step 3 deviation (commit `0c4e8b2`) replaced `sdk-prepass-guest` with raw `wit_bindgen::generate!` to work around the constraint, which silently demoted **two existing tests** (`dispatch_tdd.rs` macro-path MeshAnalysis section and `macro_all_worlds_roundtrip_tdd.rs` prepass-world section) from macro-arm coverage to raw-bindgen coverage.

### Packet revision (in-flight, 2026-05-08): bounded macro fix + host layer-idx alignment

The original 43-rev1 design (and its predecessor 43) carried the locked assumption that `crates/slicer-macros/src/lib.rs` was unchanged after commit `46aed61`. Step 3 of this packet exposed a latent compilation bug in the paint_seg_arm landed by `46aed61`: the inline WIT for `world prepass-module` at `lib.rs:1317` declares only `use geometry.{ex-polygon}`, but the generated quote-block at `lib.rs:1814-1829` constructs WIT geometry using bare `Polygon { ... }` and `Point2 { ... }` names. Without `polygon` and `point2` at world scope, those names do not resolve and any `#[slicer_module]`-authored guest that exercises `run_paint_segmentation` fails to compile. The bug was latent in master because no macro guest had ever invoked the paint_seg_arm — packet 43 ducked it via raw `wit_bindgen::generate!` in `sdk-prepass-guest`, which is exactly what 43-rev1 reverts. This packet's scope is therefore expanded by a bounded two-hunk edit in `build_prepass_world_glue`: (1) line 1317 inline-WIT extended to `use geometry.{ex-polygon, polygon, point2};` and (2) two explicit Rust `use self::slicer::world_prepass::geometry::{Polygon, Point2};` statements in the `segmentation_helpers` quote block, mirroring the finalization-world pattern at lib.rs:998. The line-1317 fix alone was tested during Step 2.5 and is necessary but not sufficient — wit-bindgen 0.24 skips flat re-exports for world-level `use` items whose TypeInfo modes_of() returns empty, requiring the explicit Rust `use` statements as well. The paint_seg_arm quote-block at lines 1814-1829 stays byte-identical. Packet 42 (TASK-130c) closed DEV-025 mismatches 4 and 5 on 2026-05-08 by keeping `paint-value-input.custom` as `string`; AC-6/AC-7 are amended in this revision to match that contract instead of the pre-Packet-42 `{semantic, payload}` shape they were originally written against.

## Scope Boundaries

- In scope:
  - Revert `test-guests/sdk-prepass-guest/src/lib.rs` to its pre-`0c4e8b2` `#[slicer_module] impl PrepassModule` (MeshAnalysis-only) form and rebuild its `.component.wasm` so previously macro-faithful tests stop being silently demoted.
  - Bounded two-hunk edit to `crates/slicer-macros/src/lib.rs` in `build_prepass_world_glue` only: (1) line 1317 inline-WIT extended from `use geometry.{ex-polygon};` to `use geometry.{ex-polygon, polygon, point2};`; (2) explicit `use self::slicer::world_prepass::geometry::{Polygon, Point2};` (two lines plus comment) added to the `segmentation_helpers` quote block, mirroring the finalization-world pattern at lib.rs:998. The paint_seg_arm quote-block (lines 1814-1829) and every other macro arm stay byte-identical to commit `46aed61`. Total churn < 20 lines.
  - Bounded host alignment in `crates/slicer-host/src/wit_host.rs` and `crates/slicer-host/src/dispatch.rs` (added in 2026-05-08 packet revision; Step 2.6): change wit_host.rs:543 from `type layer-idx = u32;` to `type layer-idx = s32;` (matches canonical `wit/deps/ir-types.wit:8`); leave the four other view records (seam-plan-entry, layer-plan-view-entry, region-segmentation-view-entry, support-geometry-view-entry) on explicit `u32` (the macros crate WIT only uses `layer-idx` for paint-region-entry); add `entry.layer_index < 0` rejection to the host validator at wit_host.rs:4089-4127; cast `entry.layer_index as u32` at the IR boundary in `dispatch.rs:harvest_paint_segmentation_ir` so PaintRegionIR keeps its `HashMap<u32, _>` shape. No IR contract change.
  - Author `test-guests/sdk-prepass-paintseg-guest/` (Cargo.toml + `src/lib.rs`) with one `#[slicer_module] impl PrepassModule` overriding `on_print_start` + `run_paint_segmentation`. Fixture switch on config key `fixture_case` ∈ {`hole_bearing`, `custom_payload`, `force_push_failure`}; default = no-op.
  - Author `test-guests/sdk-prepass-meshseg-guest/` (Cargo.toml + `src/lib.rs`) with one `#[slicer_module] impl PrepassModule` overriding `on_print_start` + `run_mesh_segmentation`. Fixture switch on config key `fixture_case == "marks_basic"`; default = no-op.
  - Register both new siblings in `test-guests/build-test-guests.sh` (GUESTS array), `crates/slicer-host/tests/guest_fixture_freshness_tdd.rs` (GUESTS table at line ~11-31), and `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` (so macro-arm proof extends to the new guests automatically).
  - Retarget `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (10 tests) to load `sdk-prepass-paintseg-guest.component.wasm`.
  - Retarget `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (1 test) to load `sdk-prepass-meshseg-guest.component.wasm`.
  - Add a "Single-Stage-Per-Impl" subsection to `docs/05_module_sdk.md` recording the macro constraint (line citations into `crates/slicer-macros/src/lib.rs`) and the sibling-crate workaround pattern.
  - Close out `docs/07_implementation_status.md` for TASK-130 / TASK-130a / TASK-130b and close DEV-025 (including mismatch 3) in `docs/DEVIATION_LOG.md` and `docs/14_deviation_audit_history.md`.
  - Mark the original packet 43 `status: superseded` with `superseded_by: 43-rev1_macro-prepass-segmentation-output-drain`.

- Out of scope:
  - Any change to `crates/slicer-macros/src/lib.rs` BEYOND the bounded two-hunk edit in `build_prepass_world_glue` described above (line 1317 inline-WIT + `segmentation_helpers` Rust `use` block). The paint_seg_arm quote-block (lines 1787-1849) and every other macro arm stay byte-identical to commit `46aed61`. Multi-stage `#[slicer_module]` support is a separate future packet.
  - Any change to host code beyond the bounded `wit_host.rs` (layer-idx alias to s32; explicit u32 retention for the four non-paint records; negative-layer rejection in push_paint_region validator) and `dispatch.rs:harvest_paint_segmentation_ir` (i32→u32 cast at IR boundary) edits described above. No IR type changes.
  - Modifying any WIT files, `crates/slicer-sdk/`, host-side validators, or harvest code.
  - Modifying `test-guests/prepass-guest/` (the raw-bindgen reference guest).
  - Adding new fixture cases beyond the four named above.
  - Touching unrelated packets in `.ralph/specs/` (the only allowed packet-dir edit is the supersede marker on Packet 43).

## Prerequisites and Blockers

- Depends on:
  - Commit `46aed61` (PaintSegmentation arm drain in `crates/slicer-macros/src/lib.rs:1787-1822`). Already in master.
  - Commit `0c4e8b2` test files (`macro_paint_segmentation_output_roundtrip_tdd.rs` + `macro_mesh_segmentation_output_roundtrip_tdd.rs`). Already in master; will be retargeted.
- Unblocks:
  - DEV-025 mismatch 3 closure.
  - Cleanup of TASK-130 cluster in `docs/07_implementation_status.md`.
- Activation blockers:
  - None — answered by user during packet authoring (Path A approved, no other packet currently `status: active`).

## Acceptance Criteria

- **Given** master has the deviation in `0c4e8b2`, **when** Step 2 runs, **then** `test-guests/sdk-prepass-guest/src/lib.rs` matches the pre-`0c4e8b2` single-stage `#[slicer_module] impl PrepassModule for SdkPrepassModule` form (header doc-line `//! TASK-109 round-trip witness for the world-prepass world (MeshAnalysis stage). Authored purely via #[slicer_module].`, only `on_print_start` + `run_mesh_analysis`, no `wit_bindgen::generate!` macro invocation). | `git diff --quiet 0c4e8b2^ -- test-guests/sdk-prepass-guest/src/lib.rs && echo PASS || echo FAIL`
- **Given** the two sibling crates are authored, **when** Step 3 runs, **then** `test-guests/sdk-prepass-paintseg-guest/src/lib.rs` contains exactly one `#[slicer_module]` attribute and exactly one `impl PrepassModule for` block whose method set is `{on_print_start, run_paint_segmentation}` (no other stage methods); its `Cargo.toml` lists `slicer-sdk`, `slicer-ir`, `slicer-schema`, and `wit-bindgen = "0.24"`; the source contains no `wit_bindgen::generate!` call. | `cargo build -p sdk-prepass-paintseg-guest --target wasm32-unknown-unknown && rg -c '^#\[slicer_module\]' test-guests/sdk-prepass-paintseg-guest/src/lib.rs | grep -q '^1$' && rg -c 'wit_bindgen::generate!' test-guests/sdk-prepass-paintseg-guest/src/lib.rs | grep -q '^0$' && echo PASS`
- **Given** the two sibling crates are authored, **when** Step 4 runs, **then** `test-guests/sdk-prepass-meshseg-guest/src/lib.rs` contains exactly one `#[slicer_module]` attribute and exactly one `impl PrepassModule for` block whose method set is `{on_print_start, run_mesh_segmentation}`; its `Cargo.toml` mirrors AC-2 dep set; the source contains no `wit_bindgen::generate!` call. | `cargo build -p sdk-prepass-meshseg-guest --target wasm32-unknown-unknown && rg -c '^#\[slicer_module\]' test-guests/sdk-prepass-meshseg-guest/src/lib.rs | grep -q '^1$' && rg -c 'wit_bindgen::generate!' test-guests/sdk-prepass-meshseg-guest/src/lib.rs | grep -q '^0$' && echo PASS`
- **Given** the two sibling crates exist, **when** Step 5 runs `test-guests/build-test-guests.sh`, **then** the GUESTS array contains entries `sdk-prepass-paintseg-guest:sdk_prepass_paintseg_guest` and `sdk-prepass-meshseg-guest:sdk_prepass_meshseg_guest`, the script exits 0, and `test-guests/sdk-prepass-paintseg-guest.component.wasm` and `test-guests/sdk-prepass-meshseg-guest.component.wasm` both exist. | `bash test-guests/build-test-guests.sh && test -f test-guests/sdk-prepass-paintseg-guest.component.wasm && test -f test-guests/sdk-prepass-meshseg-guest.component.wasm && rg -q 'sdk-prepass-paintseg-guest:sdk_prepass_paintseg_guest' test-guests/build-test-guests.sh && rg -q 'sdk-prepass-meshseg-guest:sdk_prepass_meshseg_guest' test-guests/build-test-guests.sh && echo PASS`
- **Given** the paintseg sibling is built and the round-trip test is retargeted, **when** Step 6 runs the `hole_bearing` fixture, **then** the harvested `PaintSegmentationIR` contains exactly one region with `polygons.len() == 1`, that polygon has at least one hole (`holes.len() >= 1`), all `Point2.x` and `Point2.y` values are integer multiples of `100` (10 000-nm conversion factor; SDK f64 mm × 10_000 → WIT i64 100-nm), `paint_value` round-trips byte-equivalent to the SDK input. | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd hole_bearing_typed_value_round_trips -- --exact --nocapture`
- **Given** the paintseg sibling is built, **when** Step 6 runs the `custom_payload` fixture, **then** the harvested region's `paint_value` is `PaintValue::Custom(s)` with `s == "test-semantic|DEADBEEF"` byte-for-byte (proves no silent fallback to a built-in variant). Note: `Custom` is a single-string tuple variant per `crates/slicer-ir/src/slice_ir.rs:189-199`; the marker `"test-semantic|DEADBEEF"` is the byte-identifiable payload. The pre-Packet-42 `{semantic, payload}` framing was retired by Packet 42 (TASK-130c) on 2026-05-08. | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd custom_semantic_and_custom_value_round_trip -- --exact --nocapture`
- **Given** the paintseg sibling is built and `fixture_case` is unset, **when** Step 6 runs the no-fixture default branch, **then** the guest pushes zero regions, the harvest produces an empty `PaintRegionIR` (`per_layer.is_empty()`), and no `ModuleError` is returned. (The original AC-7 `empty_polygons` fixture was unrealizable: `crates/slicer-host/src/wit_host.rs:4089-4127` rejects empty `polygons` lists; this AC was reframed in the 2026-05-08 packet revision to test the actual silent path.) | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd no_fixture_yields_empty_harvest -- --exact --nocapture`
- **Given** the meshseg sibling is built and the round-trip test is retargeted, **when** Step 7 runs the `marks_basic` fixture, **then** the harvested `MeshSegmentationIR` contains exactly one object with `marked_triangles == [12]` (single triangle index, value 12) for `object_id == "obj-a"`. | `cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd mesh_segmentation_marks_round_trip -- --exact --nocapture`
- **Given** the reverted `sdk-prepass-guest` is rebuilt, **when** Step 2 reruns existing dispatch tests, **then** the `dispatch_tdd.rs` MeshAnalysis macro-path tests at lines 6113-6260 (load_macro_path_prepass_guest et al.) emit and harvest mesh-analysis facet annotations / surface groups via the macro-emitted `__slicer_prepass_world_export` boundary (no raw `wit_bindgen::generate!` in the loaded guest). | `cargo test -p slicer-host --test dispatch_tdd macro_path -- --nocapture 2>&1 | rg -q 'test result: ok' && echo PASS`
- **Given** the reverted `sdk-prepass-guest` is rebuilt and registries updated, **when** Step 8 reruns `macro_all_worlds_roundtrip_tdd.rs`, **then** the test's `prepass_world_macro_guest_*` cases pass against the reverted guest **and** the test's guest registry/loader has been extended to also exercise `sdk-prepass-paintseg-guest` and `sdk-prepass-meshseg-guest`. | `cargo test -p slicer-host --test macro_all_worlds_roundtrip_tdd prepass -- --nocapture 2>&1 | rg -q 'test result: ok'`
- **Given** the two sibling crates are built, **when** Step 8 updates `guest_fixture_freshness_tdd.rs:11-31` GUESTS table, **then** the table contains entries `("sdk-prepass-paintseg-guest", "sdk-prepass-paintseg-guest.component.wasm")` and `("sdk-prepass-meshseg-guest", "sdk-prepass-meshseg-guest.component.wasm")` and the freshness test passes. | `cargo test -p slicer-host --test guest_fixture_freshness_tdd -- --nocapture`
- **Given** the implementation is complete, **when** Step 10 edits `docs/05_module_sdk.md`, **then** that file contains a section titled "Single-Stage-Per-Impl Constraint" (or equivalent H2/H3) that cites `crates/slicer-macros/src/lib.rs:43-52` (compile_error guard) and `crates/slicer-macros/src/lib.rs:2024` (hardcoded `__slicer_prepass_world_export`) and documents the sibling-crate workaround pattern using `sdk-prepass-paintseg-guest` / `sdk-prepass-meshseg-guest` as exemplars. | `rg -q 'Single-Stage-Per-Impl' docs/05_module_sdk.md && rg -q 'slicer-macros/src/lib.rs:43' docs/05_module_sdk.md && rg -q '__slicer_prepass_world_export' docs/05_module_sdk.md && echo PASS`
- **Given** the implementation is complete, **when** Step 10 edits `docs/07_implementation_status.md` and `docs/DEVIATION_LOG.md`, **then** TASK-130, TASK-130a, and TASK-130b show `[x]` (closed) status, and the DEV-025 row shows mismatch 3 closed (no remaining open mismatch in DEV-025). | `rg -q '\[x\] TASK-130\b' docs/07_implementation_status.md && rg -q '\[x\] TASK-130a' docs/07_implementation_status.md && rg -q '\[x\] TASK-130b' docs/07_implementation_status.md && ! rg -q 'DEV-025.*open' docs/DEVIATION_LOG.md && echo PASS`
- **Given** the host inline WIT at `crates/slicer-host/src/wit_host.rs:543` carried a stale `type layer-idx = u32;` that drifted from the canonical `wit/deps/ir-types.wit:8` `s32`, **when** Step 2.6 (added in the 2026-05-08 packet revision) aligns the host to the canonical (alias to s32; the four non-paint view records keep explicit u32 to match the macros crate WIT; validator rejects negative layer_index; harvest casts i32→u32 at the IR boundary), **then** the wasmtime 43 component linker accepts `[method]paint-segmentation-output.push-paint-region` for both core modules and macro-authored test guests, `cargo build --workspace` PASSes, and `cargo test -p slicer-host --test dispatch_tdd macro_path` and `cargo test -p slicer-host --test macro_all_worlds_roundtrip_tdd prepass` stay green (regression-defense). | `cargo build --workspace && rg -q '^\s*type layer-idx = s32;\s*$' crates/slicer-host/src/wit_host.rs && rg -q 'entry\.layer_index < 0' crates/slicer-host/src/wit_host.rs && rg -q 'entry\.layer_index as u32' crates/slicer-host/src/dispatch.rs && cargo test -p slicer-host --test dispatch_tdd macro_path -- --nocapture 2>&1 | rg -q 'test result: ok' && echo PASS`
- **Given** the macro paint_seg_arm bug blocks any macro guest from invoking `run_paint_segmentation`, **when** Step 2.5 applies the bounded fix to `crates/slicer-macros/src/lib.rs`, **then** the change is bounded to two hunks in `build_prepass_world_glue` only (1) line 1317 inline-WIT extended from `use geometry.{ex-polygon};` to `use geometry.{ex-polygon, polygon, point2};`, and (2) two explicit Rust `use` statements added to the `segmentation_helpers` quote block — `use self::slicer::world_prepass::geometry::Polygon;` and `use self::slicer::world_prepass::geometry::Point2;` — mirroring the finalization-world pattern at lib.rs:998; `cargo build --workspace` succeeds; the paint_seg_arm quote-block at lines 1814-1829 stays byte-identical to commit `46aed61`; and total churn is bounded at < 20 lines. (The line-1317 fix alone was tested and is necessary but not sufficient — wit-bindgen 0.24 skips flat re-exports for world-level `use` items whose TypeInfo modes_of() returns empty, requiring the explicit Rust use statements as well.) | `cargo build --workspace && rg -q '^\s*use geometry\.\{ex-polygon, polygon, point2\};\s*$' crates/slicer-macros/src/lib.rs && rg -q '^\s*use self::slicer::world_prepass::geometry::Polygon;\s*$' crates/slicer-macros/src/lib.rs && rg -q '^\s*use self::slicer::world_prepass::geometry::Point2;\s*$' crates/slicer-macros/src/lib.rs && [ "$(git diff --numstat crates/slicer-macros/src/lib.rs | awk '{print $1+$2}')" -lt 20 ] && echo PASS`
- **Given** this packet supersedes Packet 43, **when** Step 11 runs, **then** `.ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md` frontmatter contains `status: superseded` and `superseded_by: 43-rev1_macro-prepass-segmentation-output-drain`. | `rg -q '^status: superseded$' .ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md && rg -q '^superseded_by: 43-rev1_macro-prepass-segmentation-output-drain' .ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md && echo PASS`

## Negative Test Cases

- **Given** the paintseg sibling's `force_push_failure` fixture pushes a `PaintRegionEntry` with `polygons: vec![]`, **when** the macro arm forwards it through WIT `push-paint-region`, **then** the host validator at `crates/slicer-host/src/wit_host.rs:4089-4127` returns `Err("paint-segmentation-output: polygons list must not be empty")`, the macro arm at `crates/slicer-macros/src/lib.rs:1837-1843` maps that to `ModuleError { code: 10, fatal: true, message }`, and the host harvest reports a fatal stage error rather than silently emitting an empty region. (Empty-polygons rejection is the canonical force-failure vector since the host validator covers `<3` contour points, empty `object_id`, empty `semantic`, and empty `polygons` — all reachable from the guest with a one-line change.) | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd push_failure_surfaces_as_fatal_module_error -- --exact --nocapture`
- **Given** the macro-faithful contract is load-bearing, **when** the source-grep AC's run, **then** **neither** `sdk-prepass-paintseg-guest/src/lib.rs` **nor** `sdk-prepass-meshseg-guest/src/lib.rs` contains the literal string `wit_bindgen::generate!` (regression-defense — catches any future deviation that bypasses `#[slicer_module]`). | `! rg -q 'wit_bindgen::generate!' test-guests/sdk-prepass-paintseg-guest/src/lib.rs test-guests/sdk-prepass-meshseg-guest/src/lib.rs && echo PASS`

## Verification

- `cargo build --workspace` — must pass after every edit step (S — fast).
- `cargo clippy --workspace -- -D warnings` — must pass at the packet completion gate.
- `bash test-guests/build-test-guests.sh` — must rebuild all guests including the two new siblings.
- `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd` — full file (≤ 11 tests).
- `cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd` — full file (1 test).
- `cargo test -p slicer-host --test dispatch_tdd macro_path` — regression-defense for reverted `sdk-prepass-guest`.
- `cargo test -p slicer-host --test macro_all_worlds_roundtrip_tdd prepass` — regression-defense + new-guest registration coverage.
- `cargo test -p slicer-host --test guest_fixture_freshness_tdd` — registry freshness.
- **No `cargo test --workspace` is required for this packet** (no new contract, validator, or scheduler changes — the guarded targeted tests cover the surface).

## Authoritative Docs

- `docs/05_module_sdk.md` — load directly (≤ 300 lines expected; this packet adds a section to it). Read the existing macro-related section before editing.
- `docs/03_wit_and_manifest.md` — delegate a SUMMARY for the `paint-segmentation-output` interface and `push-paint-region` signature.
- `docs/02_ir_schemas.md` — delegate a SUMMARY for `PaintSegmentationIR.regions[*].polygons[*]` and `MeshSegmentationIR.objects[*].marked_triangles` shapes.
- `docs/07_implementation_status.md` — delegate ALL reads (file is large); only edits at Step 10 via worker dispatch.
- `docs/DEVIATION_LOG.md` — delegate a SNIPPET fetch for DEV-025 row before editing.
- `docs/14_deviation_audit_history.md` — delegate a SNIPPET fetch.
- `docs/08_coordinate_system.md` — delegate a FACT confirming the SDK-mm × 10_000 → WIT-100-nm conversion (relevant to AC for `hole_bearing` fixture's coordinate assertions).

## OrcaSlicer Reference Obligations

- None. This packet adds no algorithmic behavior — only test-harness scaffolding, doc updates, and a revert. Macro-arm correctness is verified end-to-end by the round-trip ACs against the existing macro arm (already drained in commit `46aed61`).

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost is **M** — one M step (Step 6/7 round-trip retargeting can swing M depending on test-file inspection depth) plus several S steps. No step is L. If any step's actual cost approaches L, split before proceeding.
