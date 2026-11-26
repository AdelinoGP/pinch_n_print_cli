# Evidence: I/O, G-code and module-kernel properties

Packet: `proptest-openrscad_16_pbt-io-gcode`. This file records the measured provenance, the
prerequisite recheck, the owner-seam corrections, the executed commands, the confirmed defect ledger,
the measured Windows routine/extended profiles, the manual scanner review and the workspace gates.
Every number below comes from a command executed on this tree; anything not measured is stated as
unmeasured. All sections are filled from the Step 10 run except where a lane is explicitly marked as
not run.

## 1. Provenance

| Field | Value |
| --- | --- |
| Implementation base | `4badb776e5814a40cdc645633b390bd73f67f159` (branch `feature/proptest-openrscad`, clean tree at start) |
| Toolchain | `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)` |
| Platform | Windows, native host build |
| `CARGO_TARGET_DIR` | unset; workspace default `target/` for every recorded command |
| Workspace Proptest | `proptest = "1.11"`, `default-features = false`, features `std`, `fork`, `timeout` |
| Packet status at start | `draft`; prerequisites #1, #2 and #6 all `implemented` |
| Dirty paths at base | none (`git status --porcelain` empty) |

Per-command dirty digests and `source_state_digest` are carried by the `PropertyRun` context
(`context.json`) written under `target/property-testing/<run_token>/<id>/`; a replay cannot present a
stale provenance line as current.

## 2. Prerequisite handoffs actually in the tree

Rechecked on disk before implementation (the packet requires all three, not just #1/#2):

- `proptest-openrscad_01_pbt-foundation`: `status: implemented`. Live exports
  `slicer_sdk::test_support::property_testing::{PropertyRun, PropertyRunError, CaseGuard}`,
  `PropertyRun::{new,bound,check}`, `domains`, `oracles`, `regression`; `test-properties.toml`
  schema 1 with 82 records before this packet; `cargo xtask test --properties` with
  `--tier/--property/--seed`.
- `proptest-openrscad_02_pbt-domains`: `status: implemented`. Live exports
  `domains::{DomainError, finite_f32, finite_f64, rounding_boundary_mm, ScalarConfigDomain,
  scalar_config_values, EntityDomain, entity_layers, BoundaryRule, analytical_meshes,
  rigid_transforms, SolidKind}`, `oracles::{point_in_region, segment_in_region, segment_relation,
  PointLocation}`, `regression::{ConcreteInput, format_regression_fixture}`. No `MeshDomain` or
  `GCodeDomain` exists; none was used.
- `proptest-openrscad_06_pbt-ir-config`: `status: implemented`. `H-CHAINS`/`H-CONFIG`/`H-IDS` are
  registered and passing; this packet consumes the ordered-chain identity as fixture input only and
  does not add #6 or #11 as a dependency.

## 3. Owner selection features and dependency isolation (Step 1)

Added `property_testing = []` (empty, default off) to `slicer-model-io`, `slicer-gcode`,
`machine-gcode-emit`, `part-cooling`, `overhang-classifier-default`, `skirt-brim` and `wipe-tower`;
workspace Proptest plus an SDK dev-dependency with `features = ["test", "property_testing"]`
(`slicer-model-io` had no SDK dev-edge and gained one; the others extended theirs). New gated
targets: `slicer-model-io/property_io`, `slicer-gcode/property_gcode`,
`overhang-classifier-default/property_overhang_speed`, each with
`required-features = ["property_testing"]`.

Compile evidence: `cargo check -p slicer-model-io -p slicer-gcode -p machine-gcode-emit
-p part-cooling -p overhang-classifier-default -p skirt-brim -p wipe-tower --all-targets
--features <each>/property_testing` finished with 0 errors (4m08s cold). `cargo metadata --no-deps`
showed the new features and targets discovered for all seven packages. No production, build or guest
graph acquired Proptest or the SDK property module: `proptest` appears only as a dev-dependency in the
seven manifests, and the features are empty.

## 4. Registry and CI scaffolding

Eleven new records appended to `test-properties.toml` with `generator_version = 1`,
`requires_host_algos = false`, `requires_scad = false`, both profiles, and the bound keys named by the
packet: `H-IO-ID`, `H-IO-EXTENT`, `H-IO-PLACEMENT`, `H-GCODE-FEEDRATE`, `H-GCODE-EMIT`,
`H-GCODE-SERIALIZE`, `H-GCODE-TEMPLATE`, `H-COOLING`, `H-OVERHANG-SPEED`, `H-SKIRT-BRIM`,
`H-WIPE-TOWER`. The properties-windows assertion in `.github/workflows/ci.yml` grew from 43 to 54
recorded IDs; the lane still executes every other selected record.

## 5. Owner seams and packet corrections

- H-IO-ID/EXTENT/PLACEMENT: `path_object_id` (private), `check_basename_collisions`,
  `object_world_z_extent` and `place_bare_mesh_on_bed` in
  `crates/slicer-model-io/src/loader.rs`; the private id test is an inline `#[cfg(test)]` module in
  the owner crate; the public extent/placement tests live in the `property_io` target.
- H-GCODE-*: `DefaultGCodeEmitter::{resolve_feedrate,emit_gcode,emit_gcode_owned}`,
  `DefaultGCodeSerializer::serialize_gcode`, `format_coord`/`format_xyz` and
  `GcodeFlavor::set_temperature`; `calculate_speed`'s last-boundary branch is the confirmed defect
  fixed in Step 8; `role_equals`'s missing arms and the absolute-E serializer accumulator are the
  confirmed defects fixed in Steps 5 and 6.
- Module kernels stay private in their owners; the two lib-test homes use stacked
  `#[cfg(test)]` + `#[cfg(feature = "property_testing")]` gates. `modules/core-modules/*/src` is
  outside both scanner scopes, and the manual review is recorded in §8.
- **Skirt/brim seam correction** (packet `## Deviations` item 1): the authored
  `generate_skirt_entities`/`generate_brim_entities` do not exist. The property drives the real private
  `SkirtBrim::generate(&[LayerSnapshot])`, models the quantised annulus
  `floor(brim_width/spacing/2)` with `spacing = flow_spacing()`, and checks whole-bbox enclosure rather
  than rectangle identity. The extended tier then found that the exact count is only valid above the
  offset's arc-decimation bound: at `spacing <= 0.083579 mm` the last band splits into extra
  non-degenerate clipper rings (measured 693 vs 690 on a 1/8 mm line width), so the property keeps the
  exact prefix and validates/records the tail fragments. The review round also replaced the uniform
  half-anchor envelope with derived family-specific corner-join windows (`contour_env = 0.086579` mm,
  `max_clearance_dev`/`max_bbox_excess` = 0) and reclassified the sliver emission as a candidate
  observation rather than a proven equivalence (canonical `tryExPolygonOffset` uses a different
  erode-dilate step). The 1.19/0.17 boundary control is retained and measures 4 bands / 8 rings with no
  zero-offset ring.
- **Wipe-tower bed correction** (packet `## Deviations` item 2): `generate_purge_paths` does not read
  `bed_shape`; the production convex/concave bed check lives in `WipeTower::run_finalization`. The
  property asserts tower-rectangle containment and uses the convex bed only as an independent scene
  witness; the concave-bed control is a test-oracle demonstration.
- **Placement `build_volume` correction** (packet `## Deviations` item 3): `MeshIR::build_volume` is
  the post-placement **world-space** model AABB by contract (`docs/08` § Bed Placement;
  `bed_placement_tdd::build_volume_tracks_the_placed_model`). `visual_debug` names the field as
  world-space but never sees a placed mesh, so no consumer was observed reading the stale box; the
  defect is contract-level. Because placement folds the shift into `transform.matrix`, the former
  raw-vertex `compute_bounding_box_union` preserved the pre-placement box: a confirmed SUT defect,
  fixed with `compute_world_bounding_box_union` and pinned by
  `identity_matrix_placement_updates_build_volume_to_the_world_union`. The property now asserts the
  world union unconditionally and counts live-transform scenes.

## 6. Executed verification

All commands ran on the Windows host against this tree; plain Cargo runs are logged under
`target/test-output-p16-*.log`, registry runs additionally under `target/property-testing/`.

| Command family | Result |
| --- | --- |
| `cargo check` / `cargo clippy -p <owner> --all-targets --features property_testing` for all seven owners (also as one combined command) | PASS, rc 0, no warnings |
| The 12 exact deterministic controls (`E` list in `packet.spec.md`) | PASS, `1 passed; 0 failed` each |
| Each of the 11 `P` selectors at `--tier routine` | PASS, 64/64 accepted, 0 rejects, 0 rejected calls |
| Each of the 11 `P` selectors at `--tier extended` | PASS, 256/256 accepted, 0 rejects |
| Combined 11-record routine selection (`M` line 1) | PASS, 35 s warm wall |
| Combined 11-record extended selection (`M` line 2) | PASS, 42 s warm wall |
| Retained-seed replay `H-SKIRT-BRIM --tier routine --seed 4285591042002023685` | PASS, 64/64 |
| Existing baselines: 8 `slicer-model-io` (plus the new `build_volume` regression), 10 `slicer-gcode` (incl. the two named regressions), 2 `overhang-classifier-default`, 5 module-kernel, 2 `skirt-brim` (plus `skirt_brim_corner_join_witness`), 2 `wipe-tower` | PASS, each `1 passed; 0 failed` |
| `cargo xtask check-literals` | PASS, 0 violations |
| `cargo xtask check-test-quality --report` | PASS, 0 findings (in-scope files only; module `src/**` is out of scope, see §8) |
| `cargo xtask check-fmt` | PASS, 62 packages clean |
| `cargo xtask build-guests --check` | exit 0 (EXIT_FRESH) |
| `cargo build --workspace` | PASS (233 s cold) |
| `cargo check --workspace --all-targets` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |

## 7. Measured Windows profiles, counters and replay

Measured on the Windows development machine, rustc 1.96.0, `dev` profile, workspace `target/`,
Proptest 1.11, warm binaries. Per-record libtest walls from the combined selections:

| ID | routine cases / wall | extended cases / wall |
| --- | --- | --- |
| `H-IO-ID` | 64 / 0.06 s | 256 / 0.14 s |
| `H-IO-EXTENT` | 64 / 0.05 s | 256 / 0.12 s |
| `H-IO-PLACEMENT` | 64 / 0.04 s | 256 / 0.15 s |
| `H-GCODE-FEEDRATE` | 64 / 0.04 s | 256 / 0.10 s |
| `H-GCODE-EMIT` | 64 / 0.06 s | 256 / 0.20 s |
| `H-GCODE-SERIALIZE` | 64 / 0.06 s | 256 / 0.19 s |
| `H-GCODE-TEMPLATE` | 64 / 0.06 s | 256 / 0.19 s |
| `H-COOLING` | 64 / 0.04 s | 256 / 0.10 s |
| `H-OVERHANG-SPEED` | 64 / 0.04 s | 256 / 0.10 s |
| `H-SKIRT-BRIM` | 64 / 0.63 s | 256 / 2.98 s |
| `H-WIPE-TOWER` | 64 / 0.04 s | 256 / 0.11 s |

Combined selection: 11 records in 35 s (routine) and 42 s (extended) warm wall, including the
per-record compile/link checks the executor performs. Every record reported `outcome = passed`,
`accepted_cases` equal to its `cases`, `rejected_test_calls = 0`, `local/global rejects 0` and
`call_counts_complete = true`; `shrink_completed` is null (Proptest 1.11 cannot report completion).
The profiles committed in `test-properties.toml` are 64/32/64 (routine) and 256/128/256 (extended)
with zero rejection/regeneration budgets, `rng_algorithm = "chacha"`, `isolation = "in_process"`,
`timeout_ms = 0`, `max_shrink_time_ms = 0`, and the bound keys listed per record in
`packet.spec.md` § Registry.

Measured populations (final runs): `H-IO-ID` all five path/collision modes; `H-IO-EXTENT`
identity/affine/flat/NaN/infinity; `H-IO-PLACEMENT` empty/single/multi/already-placed/below-bed/live transform/all-nonfinite-X
(7 modes; extended 35 live-transform and 46 non-finite-X cases in the implementing run); `H-GCODE-FEEDRATE` every role, both clamp ends and the overflow probe;
`H-GCODE-EMIT` empty/skipped/side-entry/annotation-only layers, tool changes, singleton and
zero-length entities and all three negative modes; `H-GCODE-SERIALIZE` both modes and flavors, Gcode
and firmware retracts, G92 resets and the formatting probe; `H-GCODE-TEMPLATE` all malformed shapes
and all nine command variants; `H-COOLING` boundary/below/`fan_max = 0`/empty-role/enabled-bump/
disabled-with-overhang; `H-OVERHANG-SPEED` `sections=17`, `zero_width=9`, `interpolation_samples=161`,
`empty=44`, `singleton=21`, `monotone=11`, `boundary=6`, `below_sentinel=4`, `exact_half=4` (routine);
`H-SKIRT-BRIM` `thin_tail`/`tail_fragments` are **single-run samples, not bounds** (seed-dependent;
routine `1`/`0`, extended `20-27`/`0-6` across the observed runs), `decimation_classified=0`,
`max_clearance_dev=0.000000` and `max_bbox_excess=0.000000` (all residuals inside the derived
corner-join windows), `max_bbox_dev` 0.049-0.054 mm (raw corner-chord sagitta), `enclosed_loops`
57 routine / 238 extended; `H-WIPE-TOWER` routine
`positive_cross=39, empty_cross=25, rows_zero=6, rows_one=5, rows_many=21, exact_rows=29,
classified_rows=10, rectangle_bed=33, triangle_bed=31`, extended `exact=117, classified=52,
boundary=16, ceiling_volume=43, rect=121, tri=135`.

Retained-seed replay: `H-SKIRT-BRIM --tier routine --seed 4285591042002023685` reproduced the
selected case stream and passed 64/64 in 0.4 s. Contexts (including `source_revision`,
`dirty_paths`, `source_state_digest`, seed and profile) are written under
`target/property-testing/<run_token>/<ID>/context.json`. The directory was pruned after the
disk-space failure above; a fresh `H-WIPE-TOWER` routine run regenerated a live context
(`property_id`, `tier`, `seed`, `source_revision = 4badb776...`, `completion_eligible = true`,
36 dirty paths) as a post-prune witness, and the per-record evidence logs are archived as
`target/test-output-p16-*.log`.


## 8. Manual literal and test-quality review (scanner blind spot)

`cargo xtask check-literals` covers `crates/*/tests/**` and inline `crates/*/src/**` test subtrees but
not `modules/core-modules/*/src`, and does not follow an out-of-line `mod property_tests;`. The
following hand reviews were recorded in the file headers:

- `machine-gcode-emit/src/property_tests.rs`: no watched-type struct literals (only `GCodeCommand` enum
  struct-variants, excluded by construction, and `ConfigView::from_map`); no waiver needed.
- `part-cooling/src/property_tests.rs`: `PrintEntity` uses `..print_entity_base(role)`,
  `LayerCollectionIR` uses `..Default::default()`, `extrusion_path3d_base` is called rather than
  spelled; `RegionKey` has four fields (unwatched) and comes from the base. No waiver needed.
- `skirt-brim/src/property_tests.rs`: every watched-type literal uses FRU --
  `Point3WithWidth { ..Default::default() }`, `ExtrusionPath3D { ..extrusion_path3d_base(..) }`,
  `PrintEntity { ..print_entity_base(..) }`, `LayerCollectionIR { ..Default::default() }`; `RegionKey` has
  four fields (unwatched) and is exhaustive by design. The pinned 693-ring count is a characterisation of
  measured clipper fragmentation, not a hand-maintained roster.
- `wipe-tower/src/property_tests.rs`: no watched-type struct literals; `ToolChange` has three fields and
  is still built with an FRU rest, `ConfigView` goes through `ConfigView::from_map`, and
  `Point3WithWidth`/`ExtrusionPath3D`/`RegionKey` are only read from output.
- `crates/slicer-model-io/src/loader.rs` inline module is inside the scanner's inline scope, but the
  scan is not correctness evidence; its watched literals and test-quality patterns were reviewed by
  hand.

Test-quality review of the four module files: no success-only assertions, no vacuous loops, no
source-grep assertions, no placeholder tests; the shrink probes assert validity after every protocol
step and do not claim global minimality; `no_shrink` selectors are determinism pins. Two helper
predicates deliberately mirror documented production contracts rather than compute alternatives:
`slicer-model-io`'s `same_file_model` mirrors `check_basename_collisions`'s literal-equality-then-
canonicalize rule (the packet requires modelling that exact predicate), and the placement AABB
helper applies the documented transform and finite-X gate. They are contract mirrors, not
independent algorithms: the identity/extent/transform *results* are still recomputed independently,
and the collision oracle's grouping/order logic is the property's own. `cargo xtask
check-test-quality --report` reported 0 findings over its in-scope files; that result is not claimed
for module `src/**`.

## 9. Workspace gates

| Gate | Result |
| --- | --- |
| `cargo build --workspace` | PASS, 233 s cold |
| `cargo check --workspace --all-targets` | PASS, 85 s warm |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, 79 s warm |
| Feature-enabled `cargo check`/`cargo clippy` for all seven owners, `--all-targets`, `-D warnings` | PASS |
| `cargo xtask check-literals` | PASS, 0 violations in 0 files |
| `cargo xtask check-test-quality --report` | PASS, 0 findings (in-scope files; module `src/**` out of scope) |
| `cargo xtask check-fmt` | PASS, 62 packages clean |
| `cargo xtask build-guests --check` | exit 0, EXIT_FRESH |

The first attempt at the full 93-record routine lane (`cargo xtask test --summary --properties
--tier routine`) aborted with `error: failed to write ... full.rmeta: There is not enough space on
the disk. (os error 112)` and link errors 1318/1140 after 78 records had already passed; it is an
environmental disk-space failure, not a property failure (all eleven packet records passed their
own selections and the 78 completed records reported `outcome = passed`). `target/property-testing`
and `target/debug/incremental` were pruned (free space 73.7 GB -> 111.1 GB) and the lane was re-run:

- `cargo xtask test --summary --properties --tier routine` -> `VERDICT: PASS`, rc 0, 1056 s
  (17.6 min), 89 end events recorded, every one `outcome = passed` (the four `requires_scad`
  records are skipped without `--with-scad`). The CI-style positive assertion was reproduced
  locally: all eleven packet IDs appear in `target/test-output.log`.
- `cargo xtask test --summary --properties --tier extended` -> `VERDICT: PASS`, rc 0, 1255 s
  (20.9 min), 89 end events, every one `outcome = passed`, all eleven packet IDs present.
  The all-record binary count is 89 in both lanes, consistent with the four skipped SCAD records.




## 10. Defect ledger

| # | Family | Confirmed defect (reproduced before fix) | Fix + named regression |
| --- | --- | --- | --- |
| 1 | H-GCODE-EMIT | `role_equals` omitted the `GapFill`, `SupportBaseInterface`, `RaftInfill` same-variant arms, so two consecutive same-role entities re-emitted an identical `;TYPE:` label | three arms added; `same_role_entities_emit_one_type_label_including_gap_fill` (`tests/gcode_emit_tdd.rs`) |
| 2 | H-GCODE-SERIALIZE | absolute-mode Gcode retract/unretract emitted `E-<length>`/`E<length>` deltas and never updated the accumulator; absolute Moves/G92 did not maintain it | Move sets it, G92 resets it in both modes, retract/unretract update and emit the updated absolute E, parallel seeder runs in both modes; `absolute_mode_gcode_retract_emits_updated_absolute_e_position` (`tests/gcode_relative_extrusion_tdd.rs`) |
| 3 | H-OVERHANG-SPEED | `calculate_speed`'s `>= last` branch returned the raw section speed, skipping round and cap (1.75 where 2.0 was expected) | `sections[len-1].1.round().min(original_speed)`; `calculate_speed_last_boundary_rounds_and_caps_last_section_speed` (`tests/basic_tdd.rs`) |
| 4 | H-GCODE-SERIALIZE oracle | RepRapFirmware renderer truncated `256 C` to `255 C` via a `u8` cast (test oracle, not production) | oracle renders the integer Celsius value; recorded here, no production regression |
| 5 | H-SKIRT-BRIM model | The oracle asserted `2 * floor(brim_width/spacing/2)` rings unconditionally, and its nearest-vertex window hid a one-anchor-wide geometric term; at `line_width = 1/8` the last band is one spacing wide and clipper splits it into 2 contour lobes plus 3 sliver holes (measured input: `line_width 0.125`, `brim_width 56.6875`, `1/16 x 3/16` mm rectangle at `(-64,-64)` -> 693 rings, 690 clean) | oracle keeps the exact `2*(bands-1)` prefix, records `tail_fragments`, derives family-specific corner-join windows (`contour_env = 0.086579`), reports `max_clearance_dev`/`max_bbox_excess` outside those windows (0.0), and pins the corner vertex plus a half-anchor rejection in `skirt_brim_corner_join_witness`. Production unchanged; the sliver emission is recorded as a **candidate observation** (canonical `tryExPolygonOffset` differs in its `-1.3s/+0.3s` erode-dilate step, so equivalence is unmeasured) |
| 6 | H-IO-PLACEMENT | `place_bare_mesh_on_bed` recomputed `build_volume` from raw vertices after folding the placement into `transform.matrix`, leaving the pre-placement box (confirmed SUT defect) | `compute_world_bounding_box_union`; `identity_matrix_placement_updates_build_volume_to_the_world_union`; the property asserts the world union unconditionally |

Each named regression was verified to fail with the fix stashed and pass with it (counterfactual
check recorded by the implementing step). The three output-changing fixes are not exercised
against `slicer-runtime` end-to-end scenes in this packet: the packet does not require it, and the
blast radius (G-code text for absolute E, duplicate `;TYPE:` labels, last-boundary overhang speed)
is therefore **unmeasured** beyond the unit/property suites. Two low review observations are
accepted as recorded: the per-family mode rosters are test-local arrays (a new production
`ExtrusionRole` variant still breaks the exhaustive `from_role` match at compile time, but a new
test-local mode variant must be added to its array by hand), and the skirt containment oracle uses
a sampled f64 ray-cast rather than #2's integer `segment_in_region` (recorded in the packet
deviations).

## 11. Non-claims

No universal uuid5 injectivity; no concave-bed containment; no fan-speed monotonicity; no route
optimality; no canonical rectangle/tessellation claim for the PnP skirt/brim/wipe-tower simplifications;
no global shrink minimality; no production multi-input caller for `check_basename_collisions`; no
OrcaSlicer parity claim for the ADR-0052 factor clamp or the PnP template warn-and-pass policy.
