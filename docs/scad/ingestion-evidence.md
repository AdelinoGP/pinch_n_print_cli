# SCAD ingestion: measured Windows evidence

This file records what was actually executed for opt-in SCAD ingestion
(packet `proptest-openrscad_05_scad-ingest`), on which machine, at which source
revision. It is an evidence record, not a feasibility claim: the adapter
prerequisite's own evidence lives in
[`docs/scad/windows-feasibility.md`](windows-feasibility.md), and the product
contract is [`docs/scad/ingestion.md`](ingestion.md).

## Provenance

| Field | Value |
|---|---|
| Source revision | `73dd67bf…` for the cold build, `4a5a1780` for the gate/slice/import measurements — see §Measurement provenance |
| Working-tree changes during the runs | none for either measurement set (`git status --porcelain` empty) |
| Host | Microsoft Windows [Version 10.0.26300.9974], 12 logical CPUs |
| Toolchain | `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)` |
| CMake | 4.4.3 |
| MSVC | present (the native kernel links; see the adapter evidence) |
| Profile | `dev` (`unoptimized + debuginfo`), `CARGO_INCREMENTAL=0` only for the cold-build measurement |
| `CARGO_TARGET_DIR` | unset → the repository's `target/`; the cold-build measurement used a dedicated `target/scad-cold-probe` |
| Engine revision | `1a45bd33f2886c32bda32fe6e8358a67ab03f5ef` |
| Engine patch digest | `c029707967be77f41f5bab502f0a1b1bbad24990c02fa91096a203548baa4024` |
| Engine backend | `manifold_cpp` |

The engine identity above is the value exposed as `ScadEngineInfo` — the
adapter records it before a dangerous fixture runs, so a retained failure stays
attributable. Provenance files from an actual run of the new CI lane land in
`target/scad-ingest/` (`rustc.txt`, `cargo.txt`, `cmake.txt`, `cl.txt`,
`source_revision.txt`, `dirty-content.txt`, `target-env.txt`, `rustflags.txt`).

## Dependency isolation (normal/build graphs)

`cargo tree -e normal,build,features` captured for the workspace and both
product crates. The numbers are matching lines, so `0` means the crate never
appears in any normal or build edge.

| Graph | `slicer-scad` | `openrscad`/`manifold` | `proptest` |
|---|---|---|---|
| Workspace, default | 1 — the member root only, no dependency edge | 0 | 0 |
| `pnp-cli`, default | 0 | 0 | 0 |
| `pnp-cli`, `--features scad` | 2 | present | 0 |
| `slicer-model-io`, default | 0 | 0 | 0 |
| `slicer-model-io`, `--features scad` | present | 20 | 0 |

`cargo tree --workspace` lists every workspace member as a root, so the
`slicer-scad v0.1.0` line in the workspace capture is that root, not an edge; no
default package's *dependency* graph reaches the adapter or the engine. (An
earlier revision of this table said `0` there, which contradicted the retained
artifact. The artifact is right, the table was wrong.)

The enabled `slicer-model-io` graph additionally contains no `slicer-sdk`,
`slicer-runtime` or `slicer-wasm-host` edge, and no guest crate is built into
it. That is the ADR-0017 boundary the amendment permits: exactly one optional,
geometry-only first-party edge, and no property-test or runtime edge in the
product production graph. Captured files:
`target/scad-ingest/tree-workspace-default.txt`, `tree-pnp-cli-default.txt`,
`tree-pnp-cli-scad.txt`, `tree-model-io-default.txt`, `tree-model-io-scad.txt`.

## Measurement provenance

Two measurement sets are recorded, because they were taken at different times:

| Set | Revision | Tree | What it covers |
|---|---|---|---|
| Cold build | `73dd67bfda76eea4474ebdabb9e3957680d097aa` | clean | `CARGO_TARGET_DIR` from scratch, enabled product |
| Gates, slice, import | `4a5a1780` | clean (`git status --porcelain` empty) | warm `target/`, the acceptance gates above |

The delivered commit contains no change to production source relative to either
revision; the cold-build revision differs only in later test-file edits, and the
gate-set revision differs from the delivered commit only in this evidence file.
The raw measurements are in `target/scad-ingest/measure/cold-scad-build.time`
and `target/scad-ingest/measure/exec-times-final.txt` (whose header records the
revision, the dirty-file count and the UTC timestamp). Per-gate logs are the
`final-*.log` files beside it.

## Build cost (measured, separate from execution)

A clean `CARGO_TARGET_DIR` was used, so this is a genuine from-scratch build of
the enabled product and everything it needs, including the vendored engine and
its native C++ kernel:

```text
CARGO_TARGET_DIR=target/scad-cold-probe CARGO_INCREMENTAL=0 \
  cargo build -p pnp-cli --features scad
```

| Measurement | Result |
|---|---|
| Exit status | `0` |
| Wall time | **950 s** (15 min 50 s) |
| Log | `target/scad-ingest/measure/cold-scad-build.log` |

Scope note: a matching **default-feature** cold build was not re-measured in
this lane. The default graph result above (no engine, no Proptest) is
dependency evidence, not a timing claim, and no performance budget is asserted
anywhere in this packet.

## Execution cost (measured, warm build)

Per-gate wall time at the gate-set revision above. `elapsed` includes Cargo's
target selection and the test harness; the `test result` figure is the harness's
own duration.

| Gate | Command | Exit | elapsed | tests |
|---|---|---|---|---|
| `MF` off | `cargo test -p slicer-model-io --no-default-features --test scad_ingest_feature_contract` | 0 | 1 s | 2 passed (0.00 s) |
| `MF` on | `cargo test -p slicer-model-io --features scad --test scad_ingest_feature_contract` | 0 | 0 s | 2 passed (0.00 s) |
| `ME` | `cargo test -p slicer-model-io --features scad --test scad_model_ingest` | 0 | 1 s | 5 passed (0.19 s) |
| `CF` off | `cargo test -p pnp-cli --no-default-features --test scad_cli_feature_contract` | 0 | 9 s | 1 passed (0.83 s) |
| `CF` on | `cargo test -p pnp-cli --features scad --test scad_cli_feature_contract` | 0 | 10 s | 1 passed (1.51 s) |
| `B` | `cargo test -p pnp-cli --features scad --bin pnp_cli -- scad_placement_` | 0 | 1 s | 5 passed (0.01 s) |
| `CE` | `cargo xtask test --summary -p pnp-cli --features pnp-cli/scad --test scad_cli_ingest` | 0 | 41 s | 10 passed (4.95 s) |
| one slice | `pnp_cli slice --model placed_two_body.scad --output … --module-dir modules/core-modules --no-default-module-paths --no-integrated-modules` | 0 | **1019 ms** | 40 layers |
| one import | `pnp_cli mesh import --input placed_two_body.scad --output … --stats` | 0 | **46 ms** | 1 mesh, 24 triangles |

The `CE` elapsed includes the gated runner's `check-literals` and
`check-test-quality` preflights, the guest-freshness check and a `pnp_cli`
rebuild when the binary is stale; the ingestion tests themselves are the harness
figure.

## Discovered and executed tests

Named executions, not binary counts. A green zero-test binary is not acceptance,
so every gate above reports the names it ran.

- `MF` (both states): `scad_model_feature_routing`,
  `scad_detection_is_case_insensitive_and_not_evaluation`.
- `ME`: `scad_model_one_object_source_identity`,
  `scad_model_explicit_holed_extrusion`,
  `scad_model_rejects_dimensions_and_empty`,
  `scad_model_preserves_strict_adapter_errors`, plus
  `scad_model_ingest_child_entry` (the supervised-child dispatcher, which passes
  as an ordinary contract test and is the entry point the parent spawns).
- `CF` (both states): `scad_cli_feature_routing` (the enabled option-misuse
  matrix lives in `CE` as `scad_cli_rejects_option_misuse`, because the flags
  only exist in enabled builds; `CF` keeps the feature-off unknown-argument and
  help-surface assertions).
- `CE`: `scad_cli_import_preserves_source_coordinates`,
  `scad_cli_holed_extrusion`, `scad_cli_holed_extrusion_slices`,
  `scad_cli_parameters_and_config_separation`,
  `scad_cli_information_and_stats`, `scad_cli_slice_places_and_emits`,
  `scad_cli_rejects_option_misuse`, `scad_cli_failures_do_not_publish`,
  `scad_cli_strict_errors_and_resources`, and
  `scad_cli_supervisor_drains_large_stdout` (the harness's own pipe-drain
  regression probe).
- `B`: `scad_placement_centers_and_drops_to_bed`,
  `scad_placement_preserves_disconnected_offsets`,
  `scad_placement_uses_configured_bed_centre`,
  `scad_placement_leaves_3mf_untouched`,
  `scad_placement_skips_when_bed_shape_is_unusable`.
- Existing regressions kept green in the same runs: `model_loader_tdd` (44
  passed, 1 pre-existing ignored), `bed_placement_tdd` (8),
  `world_z_canonical_surface_tdd` (6), `helpers_cli` (9), `cli_tdd` (3),
  `slice_progress_events_default_tdd` (3) through `cargo xtask test`.
- `cargo xtask check-literals`: 0 violations. `cargo xtask
  check-test-quality --report`: 0 findings. `cargo xtask build-guests --check`:
  exit `0` (fresh).

## Semantic output observations

Fixtures are analytic, so each expectation is computed from the source geometry
rather than from the observed output.

- Boolean through-hole `difference() { cube([12,10,6]); translate([4,3,-1]) cube([4,4,8]); }`:
  1 object, AABB `[0,12] × [0,10] × [0,6]` mm, volume `624 mm³`
  (`(12·10 − 4·4)·6`), identity transform, empty config, no modifiers, no paint.
- Holed 2D profile with `--scad-extrude-height 2`: Z `[0,2]`, volume `208 mm³`
  (`(12·10 − 4·4)·2`) — the hole is retained, not filled, and persists through
  STL/OBJ/3MF export.
- Placed two-body slice (`translate([-30,5,-4]) cube([4,6,8]); translate([-20,7,-2]) cube([2,2,4]);`):
  unplaced AABB `x[-30,-18] y[5,11] z[-4,4]`; analytic placement with the default
  250 × 250 bed gives `dx=149, dy=117, dz=4`, i.e. `x[119,131] y[122,128] z[0,8]`.
  The produced G-code has **40 layers**, `max_z_height: 8.00`, `Z ∈ [0.200, 8.000]`,
  and 728 extrusion moves. Excluding skirt/brim, the part's own extrusion
  envelope is `X ∈ [119.225, 130.775]` (span 11.55 mm, midpoint 125.000) and
  `Y ∈ [122.225, 127.775]` (span 5.55 mm, midpoint 125.000), i.e. the source
  geometry centred on the bed and resting on the plate.
- `mesh import` of the same script writes **source coordinates**:
  AABB `x[-30,-18] y[5,11] z[-4,4]`, volume `208 mm³`, no bed placement, exit 0,
  and one `done` row with `source_unit: "mm"`, `mesh_count: 1`, `warnings: 0`.
- `layer_height=3; cube([6,6,layer_height]);` with `--scad-param layer_height=5`
  and a JSON config `{"layer_height":0.4}`: geometry `Z ∈ [0,5]`, `max_z_height`
  `5.00`, 13 layers — the script parameter controls geometry, the slicer config
  controls layers, and neither leaks into the other.

## Strict rejections driven through real public input

Every kind below is rejected end to end (exit `4` for `mesh import`, `1` for
`slice`), with no destination published and no success event:

| Case | Input | Typed kind on stderr |
|---|---|---|
| External resource with a readable sibling | `include <…/tempting_sibling.scad>; cube(1);` | `external_resource_denied` |
| Recursion abort | `function cum(list, k) = … cube(cum([5], -1)); …` | `partial_output` |
| Boolean fallback / holed roof | `union(){ cube(10); polyhedron(…); }`, `roof(){ difference(){ square(10); … } }` | `degraded_output`, `approximate_output` |
| Source over the **default** bound | a generated script larger than 1 MiB (the loader reads 1 048 577 bytes before refusing) | `complexity_limit` |
| Coordinate out of range | `translate([10001,0,0]) cube(1);` | `coordinate_out_of_range` |
| Post-quantization conversion damage | `union(){ square(1); translate([1.00002, 0]) square(1); }` | `invalid_contours` (`quantized grid`) |
| Mixed 2D/3D | `union(){ cube(1); square(1); }` | `mixed_dimensions` |
| Invalid override, overwritten by a valid one | `--scad-param nope=( --scad-param nope=2` | `invalid_parameter` (`'nope'`) |

`non_finite_coordinate` is the only kind left to constructed-envelope evidence:
the probed public forms (`cube([0/0,1,1])`, `cube([1/0,1,1])`,
`cube([1e400,1,1])`) all evaluate to *empty* geometry instead, and the adapter's
private conversion tests own the executed rejection.

## Containment and retained artifacts

- **Renderer-stress model-I/O fixtures run in a supervised, killable child.**
  `evaluate` joins a worker thread and a native render cannot be interrupted from
  inside the library, so `crates/slicer-scad/tests/scad_process_limits.rs` and
  `docs/scad/adapter.md` require a killable child for any wall-clock guarantee.
  `crates/slicer-model-io/tests/scad_model_ingest.rs` therefore spawns its own
  test binary with `--exact scad_model_ingest_child_entry` for the partial,
  degraded, approximate and post-quantization cases; the child writes the source
  and the exact options under `target/scad-ingest/reproducers/` **before** it
  evaluates, reports `KIND`/`CODES`/`STAGES`/`DISPLAY` on stdout, and the parent
  asserts on that report plus the child's exit status and reaping at a 180 s
  deadline. The remaining model cases (parse errors, invalid options, empty
  geometry, contour-vs-3D, range, byte bound) are evaluator-only inputs that the
  adapter's step budget bounds; they run in-process, and the ones in the
  strict-error test (`resource`, `oversize` with the custom cap,
  `oversize_default`, `coordinate_range`) also retain their source and options
  records. `scad_model_rejects_dimensions_and_empty` uses temporary fixtures whose
  sources are literal strings in the committed test file.
- **Every `pnp_cli` child is supervised and both pipes are drained concurrently.**
  A child writing more than a pipe buffer would otherwise block before exiting and
  be killed as if it had hung. On the deadline the child is killed, reaped, and
  its stdout/stderr are written to `target/scad-ingest/scad_cli_ingest/timeout-*.{stdout,stderr}.txt`
  **before** the test fails, so the failure path has diagnostics too.
  `scad_cli_supervisor_drains_large_stdout` is the regression probe: it slices an
  ordinary STL to stdout (about 800 KB, well above the pipe buffer) and requires a
  clean exit with the complete stream.
- Fixture sources, the exact extra arguments of each invocation, and the captured
  stdout / stderr / exit status are written under
  `target/scad-ingest/scad_cli_ingest/` for every case in
  `scad_cli_strict_errors_and_resources`. `scad_cli_rejects_option_misuse`
  retains its two fixture sources (`misuse.scad`, `empty.step`) with their
  recorded arguments, but not per-invocation streams: every invocation in that
  matrix is fully determined by literal flag tokens in the committed test file.
  The remaining CLI fixtures (parameter matrix, holed extrusion, information,
  successful slices, failure cases A–F) are likewise generated in temporary
  directories from literal strings in the committed test. The retained copies
  exist exactly where a killed run would otherwise lose the input.
- No deterministic *public* SCAD input is used as a synthetic hang: the adapter's
  own `hang` probe owns that proof, and a frontend "hang" fixture would either be
  rejected by the step budget or become timing-dependent. The frontend supervisor
  shares the same `try_wait` → kill → reap structure.

## The CI lane (executed)

`.github/workflows/ci.yml` gains `jobs.scad-ingest-windows` (`windows-latest`,
90-minute timeout). It reuses the adapter lane's native prerequisites and the
existing guest build, then runs, with exit-status checks and named-test
assertions on every step:

1. provenance and native-prerequisite capture into `target/scad-ingest/`;
2. default-versus-enabled dependency-graph isolation for `pnp-cli` and
   `slicer-model-io`;
3. `MF` in both feature states;
4. `CF` in both feature states;
5. `ME`;
6. `B`;
7. `CE` through the gated `cargo xtask test` runner;
8. ordinary STEP/CLI and model-IO regressions;
9. narrow clippy for both crates in both feature states.

Logs and `target/scad-ingest/` are uploaded on failure.

### Hosted execution record

| Field | Value |
|---|---|
| Workflow run | [43369601815](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43369601815) |
| Job | [SCAD ingestion (Windows) 131702024722](https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43369601815/job/131702024722) |
| Head commit | `7f4416607940d66495d2fefbb040a6cc152e92e1` |
| Job result | `success` |
| Job window | 2026-11-13T05:04:41Z → 05:27:38Z (22 min 57 s, cold engine + guest build included) |
| Whole run | `success`, all nine jobs, 2026-11-13T05:04:38Z → 05:56:33Z |

Named executions from that job's own log:

| Step | Result |
|---|---|
| `MF` off | `test result: ok. 2 passed; 0 failed`; `scad_model_feature_routing` and `scad_detection_is_case_insensitive_and_not_evaluation` both named `... ok` |
| `MF` on | `test result: ok. 2 passed; 0 failed` |
| `ME` | `test result: ok. 5 passed; 0 failed` |
| `CF` off | `test result: ok. 1 passed; 0 failed` |
| `CF` on | `test result: ok. 1 passed; 0 failed` (25.18 s) |
| `B` | `test result: ok. 5 passed; 0 failed; 34 filtered out` |
| `CE` | `test result: ok. 10 passed; 0 failed` (8.07 s), `VERDICT: PASS`, with `check-literals: 0 violation(s)` and `check-test-quality: 0 finding(s)` in the same gated run |
| Regressions | `helpers_cli` 9 passed, `cli_tdd` 3 passed, `model_loader_tdd` 44 passed / 1 ignored, `bed_placement_tdd` 8 passed |

The default-versus-enabled graph-isolation step and the narrow clippy step also
passed; the job's per-step exit checks would have failed the job otherwise.
This closes the packet's "the required Windows job executes both feature
states" condition on the hosted runner, not only on the developer machine.
