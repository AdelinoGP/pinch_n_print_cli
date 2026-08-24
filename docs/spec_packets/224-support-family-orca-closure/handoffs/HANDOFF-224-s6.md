# HANDOFF-224-s6 — Packet 224 remediation session 6 (2026-08-21)

Condensed from the session-6 handoff during the history squash. Continues `HANDOFF-224-s5.md`; 24 commits landed, `e85e546a..c5fe29c4`, on `parity/support-planners`.

## Structural findings

1. **OrcaSlicer source IS on this machine**: `OrcaSlicerDocumented/src/libslic3r/` (gitignored, so
   `git ls-files` misses it). A shallow search concluded no checkout existed and told five workers so. Three defects trace to that false premise: `smooth_nodes` (F-33) wrong kernel/iterations vs
   canonical; `smooth_outward` (F-37) arc-fillet resampler vs canonical corner clipping; dual contact seeding (`a40f971a`) withheld canonical's `layer_nr - 1` shift (broke AC-4). **Rule:
   read `OrcaSlicerDocumented/` directly; never accept "no checkout exists" without listing it.**
2. **G-23 invalidates the tree suite's green**: `benchy_tree_support_regression_tripwire` runs with
   an empty `SupportGeometryView` and `..Default::default()` occupancy, so collision AND avoidance
   AND model_occupancy are empty at every layer. A 76/0 crate suite coexisted with a planner that
   produced no usable support on real meshes; it hid three defects. Fixing G-23 is the highest-value support follow-up.
3. **Native/wasm request seam**: wasm builds its layer view via `dispatch_layer_call` + guest shim;
   native via `build_native_layer_request` (`crates/slicer-wasm-host/src/marshal/native.rs`). An
   input added to one leg only makes native silently render nothing. Hit 3x: `85f1f889` (plan
   threaded into native OUTPUT only; fixed `c237b046`), `ddf9dffe` (begin_region origin recovery),
   `with_slice_ir` (support stage only). Follow-up: share one construction path or add a view-equivalence test per stage.

## Defects fixed (24 commits)

Tree re-port completed (steps 6/7 of 7: `43b1e8bb`, `936786db`, `3319ad35`). Real-geometry defects
(none in the audit; found by meeting real meshes): `c3c1ed5a` emit gates rejected any node
overlapping occupancy (68/72 at layer 99) + dual contact seeding; `c4f67120` carve vs RAW occupancy (940/19856 vertices within 0.05 mm of wall); `ac43eab9` paint segmentation built BASE
regions from whole-layer all-object contours (bit-for-bit inert on single-object layers); `de6d53b6` cross-family rejection unscoped by object/layer; `c237b046` native plan attach.

## Test-discipline lessons (measured)

- `cargo test --workspace` is FAIL-FAST and fooled this session TWICE: "258 binaries / 2112 passed"
  skipped the whole e2e binary (128 tests); "259 binaries / 2242 tests" was also truncated. True
  complete figure (`--no-fail-fast`): 386 result lines (345 test binaries + 41 doc-test suites), 3806 passed. Never report a workspace total not produced with `--no-fail-fast`.
- Golden reblessing must be classified first: legacy golden drift was exactly one config-block line (`support_type = Traditional` → `normal(auto)`, from `0d0c8d4a`), zero toolpath bytes.

## State at handoff

- Full suite (`--no-fail-fast`): **3809 passed / 1 failed / 7 ignored** across 386 result lines. **NOT GREEN** — one failure remains, needing a SPEC decision (section 8).
- Gates green: clippy, check-literals, check-deviations (was RED), gen-config-docs, build-guests; deviations recorded: **DEV-135..DEV-140** (six; a seventh deliberately excluded).
- Review artifacts in `target/review-224/` (regenerated after `92077096`; earlier set superseded):
  `SupportTest_tree.gcode` — 122 `;TYPE:Support` blocks, 2 interface, 10533 extrusion moves,
  Z 0.2–24.4, 150 layers. Orca reference: Tree 122 support / 2 interface — **matches exactly**
  (was 120 blocks / 6526 moves / columns stopping at z=8.0 before the AC-1 fix). Normal 121/3 (G-18). Layer counts 150 vs 452 (reference cut at finer layer height).
- Packet status: still `draft`, deliberately — DoD includes human review of generated G-code for both Tree and Traditional; not done. `TASK-335` stays unchecked.

## Open work, explicitly not done

- **F-37 piece 2** — base-interface (`num_top_base_interface_layers`) role. ~10 files, 2 WIT edits, 1 schema bump, new `ExtrusionRole` + `;TYPE:` decision. Canonical derivation in `050d5c3a`.
- G-23; the native/wasm request seam; `interface_regularize.rs` consolidation; STUDIO-4252 retry
  passes `radius_sample_resolution + EPSILON` where canonical passes `max_move_between_samples`;
  `execute_paint_segmentation`'s `matching_base.is_empty()` fallback. Routed out by earlier human decision: F-8/9/10/17/30/39/40/42-part/43, F-29.

## One failure outstanding — SPEC decision, not a code fix

`prepass_support_geometry_layer_plan_tdd::planner_emits_one_entry_per_region_in_region_map`
(AC-8) is RED and left red deliberately: expected 2 entries for (layer=2, object=obj-multi), got 1;
region 42 never appears. Root cause: `support_analysis_producer.rs` mints `family_assignments` per
CANDIDATE; candidates come from `SliceIR` regions (this plate slices to one); `candidate_family`
refuses to self-default, so a RegionMap region with no candidate gets no family and is declined.
**Human decision required:** does AC-8 mean one entry per RegionMap region or per host-ASSIGNED region? Count assertion left INTACT; diagnosis written into the test. The coordinator's `c3c1ed5a`
mesh-path-gate hypothesis was DISPROVED — do not re-attempt.
