# Design: 43-rev1_macro-prepass-segmentation-output-drain

## Implementation Shape

This packet is **almost** scaffolding-only. The 2026-05-08 in-flight revision adds one bounded production-code edit. The shape is:

0. **One bounded macro edit** (two hunks in `build_prepass_world_glue` at `crates/slicer-macros/src/lib.rs`: line 1317 inline-WIT extension, plus a `use self::slicer::world_prepass::geometry::{Polygon, Point2};` block in the `segmentation_helpers` quote — total churn < 20 lines). Discovered during Step 3 of this packet; without it, no macro-authored guest can invoke `run_paint_segmentation`. See "Architecture Constraints (Locked Assumptions) #7" below for the full rationale, including why the WIT-level fix alone is insufficient under wit-bindgen 0.24.
0.5. **One bounded host alignment** (Step 2.6 — added in the 2026-05-08 packet revision). The host inline WIT in `crates/slicer-host/src/wit_host.rs:543` had `type layer-idx = u32;`, drifted from canonical `wit/deps/ir-types.wit:8` `s32`. The fix: align the alias to s32 (matches the macros crate WIT and the canonical wit/), explicitly keep the four non-paint view records on `u32` (the macros crate WIT only uses `layer-idx` for paint-region-entry), reject negative `layer_index` in the host push_paint_region validator, and cast `entry.layer_index as u32` at the IR boundary in `dispatch.rs:harvest_paint_segmentation_ir` so `PaintRegionIR.per_layer` keeps its `HashMap<u32, _>` shape. No IR contract change. See "Architecture Constraints (Locked Assumptions) #10" below for the full rationale.
1. One revert (one file).
2. Two new sibling crates (six new files: 2 × `Cargo.toml` + 2 × `src/lib.rs` + 2 × `.component.wasm` artifacts produced by the build script).
3. One build-script edit (one line in the GUESTS array, repeated for two entries).
4. Three test-file edits (load-path retargets in two test files + registry extension in the freshness/all-worlds-roundtrip tests).
5. Three doc edits (one section in `docs/05`; closures in `docs/07`, `docs/DEVIATION_LOG.md`, `docs/14`).
6. One supersede marker on Packet 43's `packet.spec.md`.

## Controlling Code Paths and Surfaces

- **Macro arm under test (read-only — already drained):** `crates/slicer-macros/src/lib.rs:1787-1822`. The PaintSegmentation arm now drains `regions()` from the SDK builder through WIT `push-paint-region`. This packet does not modify it.
- **Macro guard for single-stage impl:** `crates/slicer-macros/src/lib.rs:43-52` (`compile_error!` when `detected_stages.len() > 1`).
- **Macro hardcoded module names:** `crates/slicer-macros/src/lib.rs:689` (`__slicer_postpass_world_export`), `:989` (`__slicer_finalization_world_export`), `:2024` (`__slicer_prepass_world_export`), `:2306` (`__slicer_layer_world_export`).
- **Stage detection table:** `crates/slicer-schema/src/lib.rs:32-156` (`STAGES`).
- **PrepassModule trait** (read-only): `crates/slicer-sdk/src/traits.rs:367-495`. All stage methods have `Ok(())` defaults; siblings override only what they need.

## Neighboring Tests and Fixtures

- **Round-trip targets (already authored, will be retargeted):**
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (10 tests).
  - `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (1 test).
- **Regression-defense targets (must pass after revert):**
  - `crates/slicer-host/tests/dispatch_tdd.rs:6076-6260` (macro-path MeshAnalysis).
  - `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs:232-296` (prepass world macro guest).
- **Registries to extend:**
  - `crates/slicer-host/tests/guest_fixture_freshness_tdd.rs:11-31` (GUESTS table).
  - `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` (loader/registry — exact shape to be discovered in Step 1, dispatch only).
- **Sibling guest templates (read-only):**
  - `test-guests/sdk-finalization-guest/src/lib.rs` — minimal `#[slicer_module] impl FinalizationModule` with one stage method, four-section structure (use, struct, impl, on_print_start, run_*).
  - `test-guests/sdk-layer-pathopt-guest/src/lib.rs` — same pattern, different trait.
  - Both `Cargo.toml` files share `[lib] crate-type = ["cdylib"]`, `[dependencies] wit-bindgen = "0.24" + slicer-sdk + slicer-ir + slicer-schema (path deps)`, and `[workspace]` declared empty (signals "I'm a leaf, not a member of the parent workspace").

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

## Selected Approach

**Path A: Revert + two macro siblings.**

- Revert `test-guests/sdk-prepass-guest/src/lib.rs` to its pre-`0c4e8b2` content (recovered via `git show 0c4e8b2^:...`). Rebuild its `.component.wasm`. This restores macro coverage for the two silently-demoted tests.
- Author `test-guests/sdk-prepass-paintseg-guest/` with `#[slicer_module] impl PrepassModule for SdkPrepassPaintsegGuest` overriding `on_print_start` + `run_paint_segmentation`. The run-method body switches on `config.get_string("fixture_case")`:
  - `"hole_bearing"` → `output.push_paint_region(0, "fuzzy_skin", "obj-a", 0, PaintValueInput::Custom("test-semantic|hole-bearing"), vec![ExPolygonView { contour: integer-mm 4-point square, holes: vec![integer-mm 4-point inner square] }])`. SDK builder buffers; macro arm multiplies × 10_000 to produce integer 100-nm coordinates that are trivially integer multiples of 100.
  - `"custom_payload"` → `output.push_paint_region(0, "fuzzy_skin", "obj-a", 0, PaintValueInput::Custom("test-semantic|DEADBEEF"), vec![ExPolygonView { /* 3-point triangle, no holes */ }])`. AC-6 asserts the harvested `PaintValue::Custom(s)` equals `"test-semantic|DEADBEEF"` byte-for-byte. Note: `Custom` is a single-string tuple variant per Locked Assumption #8; the marker string is the byte-identifiable payload that proves no silent fallback to a built-in variant.
  - `"force_push_failure"` → push a `PaintRegionEntry` with `polygons: vec![]`. The host validator at `crates/slicer-host/src/wit_host.rs:4089-4127` rejects empty `polygons` with `Err("paint-segmentation-output: polygons list must not be empty")`; the macro arm at `crates/slicer-macros/src/lib.rs:1837-1843` maps `Err` to `ModuleError { code: 10, fatal: true, message }`. Negative AC verifies the fatal stage error surfaces. Empty-`polygons` is the canonical force-failure vector — see Locked Assumption #9.
  - default / no `fixture_case` set → no-op `Ok(())`. AC-7 (reframed) asserts this path: zero regions pushed, harvest produces empty `PaintRegionIR`. The original `empty_polygons` fixture was retired in the 2026-05-08 packet revision because the host validator makes a silent empty-polygons region unrealizable.
- Author `test-guests/sdk-prepass-meshseg-guest/` with `#[slicer_module] impl PrepassModule for SdkPrepassMeshsegGuest` overriding `on_print_start` + `run_mesh_segmentation`. The run-method body switches on `config.get_string("fixture_case")`:
  - `"marks_basic"` → mark triangle index 12 on object id `"obj-a"` via the `MeshSegmentationOutput` builder.
  - default → no-op.
- Wire both siblings into `test-guests/build-test-guests.sh` (GUESTS array additions: `sdk-prepass-paintseg-guest:sdk_prepass_paintseg_guest` and `sdk-prepass-meshseg-guest:sdk_prepass_meshseg_guest`).
- Retarget both round-trip TDD files to load the matching `.component.wasm`.
- Extend `guest_fixture_freshness_tdd.rs` and `macro_all_worlds_roundtrip_tdd.rs` registries.
- Document the macro single-stage rule in `docs/05_module_sdk.md`.
- Close out backlog and deviation log.
- Mark Packet 43 superseded.

### Rejected Alternatives

- **Path B (keep raw-bindgen multi-stage `sdk-prepass-guest` + only add two siblings).** Rejected because it leaves `dispatch_tdd.rs` macro-path MeshAnalysis tests and `macro_all_worlds_roundtrip_tdd.rs` prepass tests silently demoted from macro-arm coverage to raw-bindgen coverage. Both files are doc-commented as proving macro-authored emission; that claim must remain true. The cost of the revert is one git operation plus one rebuild — negligible.
- **Path C (extend the macro to support multi-stage in one impl, e.g., suffix module names with stage_id).** Rejected because (a) it expands packet boundary into the macro itself, (b) it conflates a fix-the-test-scaffolding packet with a fix-the-macro packet, (c) the multi-stage-macro work has its own design decisions (export name collisions across worlds, attribute-argument vs method-name selector) that deserve their own packet. We document the single-stage constraint in `docs/05` so the next packet that wants to attempt this has the context.
- **Path D (mark 43 superseded with no follow-up).** Rejected because TASK-130 / DEV-025 mismatch 3 stays open with committed work in master that doesn't actually prove what it claims. Closure is needed.
- **Re-use the original packet 43 directory and amend its design.md in place.** Rejected because the existing packet has the original wrong assumption embedded in its design and AC framing. A fresh `43-rev1` directory follows the project's established convention (`01-rev1`, `02-rev1`, `12-rev1`, `14-rev1`, `23-rev1`, `36-rev1`, `38-rev1`) and keeps the audit trail intact.

## Code Change Surface (authoritative files-in-scope list)

Primary editing surfaces (these are the files an implementer edits):

1. `test-guests/sdk-prepass-guest/src/lib.rs` (revert; one edit step).
2. `test-guests/sdk-prepass-paintseg-guest/Cargo.toml` (new).
3. `test-guests/sdk-prepass-paintseg-guest/src/lib.rs` (new).
4. `test-guests/sdk-prepass-meshseg-guest/Cargo.toml` (new).
5. `test-guests/sdk-prepass-meshseg-guest/src/lib.rs` (new).
6. `test-guests/build-test-guests.sh` (extend GUESTS array).
7. `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (retarget load).
8. `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (retarget load).
9. `crates/slicer-host/tests/guest_fixture_freshness_tdd.rs` (extend GUESTS table).
10. `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` (extend registry).
11. `docs/05_module_sdk.md` (add Single-Stage-Per-Impl section).
12. `docs/07_implementation_status.md` (close TASK cluster — via worker dispatch).
13. `docs/DEVIATION_LOG.md` (close DEV-025 mismatch 3).
14. `docs/14_deviation_audit_history.md` (cross-reference closure).
15. `.ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md` (supersede marker).

No step opens more than 3 of these files at once. The implementation-plan groups them.

## Read-Only Context the Implementer Needs

- `test-guests/sdk-finalization-guest/src/lib.rs` (≤ 80 lines) — sibling template structure.
- `test-guests/sdk-layer-pathopt-guest/Cargo.toml` (≤ 15 lines) — Cargo.toml template.
- `crates/slicer-sdk/src/traits.rs:367-495` — `PrepassModule` trait shape (delegate SUMMARY if needed).
- `crates/slicer-sdk/src/prepass_builders.rs` — `PaintSegmentationOutput`, `MeshSegmentationOutput` builder method signatures (delegate LOCATIONS for `push_paint_region` and the marks builder).
- `git show 0c4e8b2^:test-guests/sdk-prepass-guest/src/lib.rs` — the revert target (full content, ≤ 110 lines).

## Out-of-Bounds Files (forbidden direct reads)

- `crates/slicer-macros/src/lib.rs` — > 2 300 lines. Delegate every read. The bounded two-hunk edit in `build_prepass_world_glue` (Step 2.5 — line 1317 inline-WIT extension + `segmentation_helpers` Rust `use` block) is the ONLY allowed direct write to this file in this packet; all other lines stay byte-identical to commit `46aed61`. Worker dispatches that need to inspect macro behavior must specify a narrow line range.
- `target/` — generated artifacts.
- `OrcaSlicerDocumented/` — not relevant to this scaffolding-only packet.
- `docs/07_implementation_status.md` — large; delegate reads and edits.
- `docs/DEVIATION_LOG.md` — large; delegate SNIPPET fetches.
- `wit/` (any WIT package files) — no WIT change in this packet.
- Other `.ralph/specs/` packet directories beyond Packet 43's `packet.spec.md`.

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

## Open Questions

- **None blocking activation.** All three E-paths (A vs B vs C) and the docs/supersede decisions are explicitly resolved by user input during packet authoring.

## Locked Assumptions and Invariants

The implementation must preserve these invariants. If any one is violated, the change is rejected.

1. `crates/slicer-macros/src/lib.rs` is unchanged after this packet **except for** the bounded two-hunk fix in `build_prepass_world_glue` introduced by Step 2.5: (a) line 1317 inline-WIT extended to `use geometry.{ex-polygon, polygon, point2};`, and (b) `use self::slicer::world_prepass::geometry::{Polygon, Point2};` (with a brief explanatory comment) added to the `segmentation_helpers` quote block, mirroring the finalization-world pattern at lib.rs:998. The paint_seg_arm quote-block at lines 1814-1829 and every other arm stay byte-identical to commit `46aed61`. `git diff --numstat crates/slicer-macros/src/lib.rs` after Step 2.5 must show total churn < 20 lines.
2. Each new sibling crate contains exactly one `#[slicer_module]` attribute and exactly one `impl PrepassModule for ...` block.
3. Neither new sibling crate contains `wit_bindgen::generate!`.
4. `test-guests/sdk-prepass-guest/src/lib.rs` matches its pre-`0c4e8b2` form byte-for-byte after revert.
5. `dispatch_tdd.rs` and `macro_all_worlds_roundtrip_tdd.rs` prepass cases stay green throughout the packet (no test deletions or `#[ignore]`).
6. No existing passing test is weakened (no assertion removed, no `assert!` → `eprintln!` regression).
7. Test discipline: targeted `cargo test -p slicer-host --test <file>` only; never `cargo test --workspace`.
