# Workspace ceremony chunk manifest (packet `layer-module-preparation_01_shared-selection-prerequisite`)

Closes final-review item D6: a compact map of every `cargo metadata --no-deps` workspace member (47 members, including the auto-enrolled `raft-default` and `witness`) to the gated chunk that selected it and the retained execution receipt. DEV-198 records the serialized-chunk equivalence; this manifest is its coverage proof.

Receipt naming: checkpoint commits recording each chunk's verdict are durable receipts (commit subjects carry the chunk facts verbatim). Where a named retained log exists it is listed additionally. The supplemental receipt `target/cer-chunk-manifest.log` (retained 2026-10-13, exit 0, 142 `test result:` summaries, 0 FAILED) re-executes the member set no earlier chunk provably selected.

| Member | Chunk | Receipt |
| --- | --- | --- |
| slicer-runtime | A — runtime (with workspace-unified `slicer-runtime/perimeter-spatial-test-support` mirror) | c8c47c05 (1398/0); later targeted logs `target/cer-executor.log`, `target/cer-int-final.log` |
| slicer-wasm-host | A — wasm-host | c8c47c05 (283/0) |
| traditional-support-planner, tree-support-planner, traditional-support, tree-support, lightning-infill, infill-linker, seam-placer, seam-planner-default | A — module chunk (8) | c8c47c05 (62 rows, 0 failed) |
| slicer-core | B1 (`--features host-algos`) | c7f0f2f6 (629/0) |
| slicer-ir, slicer-schema, slicer-scheduler | B2 | c7f0f2f6 (375/0) |
| slicer-sdk | B3 (`--features test`) | c7f0f2f6 (346/0) |
| slicer-gcode, slicer-macros, xtask, pnp-cli | B4 (`--features pnp-cli/report` mirror) | c7f0f2f6 (57 rows, 0 failed; xtask bin 179/0) |
| slicer-model-io, slicer-config, slicer-helpers, slicer-integrated-modules, pnp-cli-locator | B5 residual; independently re-executed | supplemental `target/cer-chunk-manifest.log` |
| sdk-finalization-guest, sdk-infill-postprocess-view-guest, sdk-layer-infill-guest, sdk-layer-pathopt-guest, sdk-postpass-text-guest, sdk-prepass-guest | B5 residual; independently re-executed | supplemental `target/cer-chunk-manifest.log` |
| arachne-perimeters, classic-perimeters, fuzzy-skin, gyroid-infill, layer-planner-default, machine-gcode-emit, overhang-classifier-default, path-optimization-default, raft-default, rectilinear-infill, skirt-brim, support-surface-ironing, top-surface-ironing, wave-overhangs, wipe-tower, part-cooling | supplemental (also claimed under B5's core-module sweep) | supplemental `target/cer-chunk-manifest.log` |
| witness (`crates/slicer-wasm-host/test-guests/witness`) | supplemental (fixture guest; no authored unit tests — zero-executed class) | supplemental `target/cer-chunk-manifest.log` |

Zero-executed/ignored classes on record for this manifest (unchanged from the chunk returns): `medial_axis_determinism_tdd` ignored (DEV-093); feature-gated `slicer_integrated_modules --lib` partial-feature exclusion; `--lib` binaries without authored unit tests (including `witness` and `slicer-macros`-style fixture guests); guest fixture libs without unit tests; `slicer-sdk` `host_batch::batch_offset` doctest ignored; core paint-segmentation fixture TODO ignores; `support_enforcer_and_multi_layer_paint_from_real_3mf_fixture_documented_gap` ignore. Every such target is owned by one of these classes; none silently counts as coverage.