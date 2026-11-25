# Direct property-domain evidence (`proptest-openrscad_02_pbt-domains`)

Measured evidence for the shared direct domains, oracles, shrinking protocol and concrete
regression formatter. Chartes live in
[`docs/testing/properties.md`](properties.md); this file records what was actually executed.

## Source and environment

| Key | Value |
|---|---|
| Date | 2026-11-10 |
| Platform | Windows (win32, x86_64-pc-windows-msvc) |
| Toolchain | rustc 1.96.0 |
| Profile | `dev` (tests: `test` profile), workspace `target/` |
| Proptest version | 1.11.0 |
| `generator_version` | 1 |
| Features | `["slicer-sdk/property_testing"]` |
| `cargo_target_dir` | `null` (workspace default) |
| `source_revision` | `56dae6bc4c51194d437eb23873e1d42f07452699` (packet checkpoint, squashed into the single packet commit) |
| `dirty_paths` at measurement | `["test-properties.toml"]` |
| `source_state_digest` | `c421c41a5799…` |

The pilots were measured against the checkpoint above before the review-driven fixes to the
regression formatter and the two harness probes; the domain generators and oracles are unchanged
by those fixes. The registry file was the only tracked modification at measurement time, and the
documentation edits were authored afterwards, so they are not part of the measured dirty set.
Each registry child received its own fresh OS seed; the retained seeds are listed under replays
below.

## Registry pilots

Executed through the registry-backed lane, which runs the literal preflight, the guest freshness
check and a `pnp_cli` staleness rebuild before spawning the selected children.

| Tier | Command | xtask wall | Result |
|---|---|---:|---|
| routine | `cargo xtask test --summary --properties --property D-SCALARS --property D-GRAPHS --property D-IR --property D-GEOMETRY --property D-MESH --tier routine` | 34.99s | PASS |
| extended | same five properties, `--tier extended` | 67.10s | PASS |
| `H-IR-ANCHORS` | `--property H-IR-ANCHORS --tier routine` (after shared-anchor adoption) | 31.77s | PASS |
| replay | `D-GRAPHS` routine, `--seed 5412186833007693115` | 11.36s | PASS |
| replay | `D-GEOMETRY` extended, `--seed 1173614043627998710` | 18.45s | PASS |

Per-property libtest wall, accepted callbacks and rejections (`call_counts_complete = true` on
every run):

| Property | routine wall | extended wall | accepted routine/extended | rejected |
|---|---:|---:|---:|---:|
| `D-SCALARS` | 0.03s | 0.46s | 64 / 1024 | 0 / 0 |
| `D-GRAPHS` | 0.04s | 0.94s | 64 / 1024 | 0 / 0 |
| `D-IR` | 0.26s | 41.76s | 64 / 1024 | 0 / 0 |
| `D-GEOMETRY` | 0.19s | 8.39s | 64 / 1024 | 0 / 0 |
| `D-MESH` | 0.03s | 0.46s | 64 / 1024 | 0 / 0 |

`shrink_completed` is reported `null` throughout because Proptest 1.11 does not expose whether
minimisation finished. No retained case is described as minimal.

Archived combined pilot logs: `target/evidence/domain-routine.log`,
`target/evidence/domain-extended.log`, `target/evidence/h-ir-anchors-routine.log`,
`target/evidence/domain-graphs-replay-routine.log`,
`target/evidence/domain-geometry-replay-extended.log`. Final-tree logs:
`target/evidence/final/`.

## Compile and execution measurements (separate)

Warm measurements (live observations, not archived timings):

| Measurement | Value |
|---|---:|
| `property_domain_contracts` target (11 tests, warm) | 5.20s |
| Native fixture probe, warm rebuild inside that target | 1.24s |
| `property_harness` (foundation regression target) | 0.46s |

The first cold build of the temporary fixture crate's dependency graph was observed live at
51.24s. It is paid once per target directory, then cached; it is a compile measurement, not a
property execution measurement. Only the per-property walls and accepted/rejected counts in the
pilot tables are backed by archived logs.

## Sampled topology witnesses

`domain_geometry_topology_witnesses` samples 128 cases per `TopologyClass` (1 024 sampled cases)
plus ten tight-bound domains at 32 cases each (320 cases). Every sampled case passes
`validate_geometry`, and every class has an independent check that does not call the generator's
tag:

- `Convex`: all outer turns `>= 0`, at least four vertices on roomy domains.
- `Concave`: at least one negative turn.
- `NonStarShaped`: a downward vertical edge and an upward vertical edge strictly to its left, so
  the closed half-planes `x >= xR` and `x <= xL` contradict. The deterministic witness used in
  the registry property is `[(0,0),(20,0),(20,20),(8,20),(8,3),(6,3),(6,20),(0,20)]`: the right
  wall at `x = 8` directed downward imposes `x >= 8`, the left wall at `x = 6` directed upward
  imposes `x <= 6`, so the kernel is empty regardless of notch depth.
- `Holed`: at least one hole; sampled populations include two and three holes, each strictly
  inside with one-unit clearances.
- `Disconnected`: at least two components; sampled populations include three and four
  separated rectangles.
- `Thin`: a one-unit notch in a component whose bounding-box minimum dimension is at least 16.
- `NearDegenerate`: exact one-unit clearance between nonadjacent edges.
- `Empty`: zero components.

## Generation, rejection and shrinking

- Generation attempts equal accepted callbacks plus rejections because no generator filters:
  64/1024 accepted and 0 rejected per property in both tiers.
- Contract-level shrink probes start from explicit nonminimal witnesses and observed
  `observed_structural_reductions vertices=2 holes=2 components=2`. Required minima were
  retained: the three-hole case shrank to exactly one hole (`min_holes = 1`) and the
  four-component case to exactly two (`min_components = 2`).
- The same families are also driven through a real Proptest `TestRunner` with a failing
  predicate, not only through a hand loop. The recorded traces are (components, holes, vertices,
  sum_abs_xy_units):
  `known_failure_trace holes (1, 3, 16, 146) -> (1, 2, 12, 28)`,
  `components (4, 1, 21, 183) -> (3, 0, 12, 26)`, and, for always-failing predicates,
  `holes_retained (1, 3, 17, 167) -> (1, 3, 16, 44)` and
  `components_retained (4, 1, 21, 183) -> (4, 0, 16, 42)`. The threshold probes therefore start
  structurally nonminimal and reduce hole/component counts, while the always-failing probes keep
  the required minima. Ring-vertex reduction (a five-vertex witness shrinking by two vertices) is
  asserted by the explicit nonminimal convex probe rather than by the runner, because the convex
  generator cannot guarantee a nonminimal first failure. Every retained case passes
  `validate_geometry`.
- `domain_shrink_protocol_preserves_witnesses` drives `simplify` to `false` and then `complicate`
  back to the initial case for every non-`Empty` class; every intermediate passes
  `validate_geometry`, and every successful `simplify` strictly decreases the
  `(components, holes, vertices, sum_abs_xy_units)` tuple. A three-vertex triangle with
  complexity `(1, 0, 3, 2)` is an explicitly minimal case whose `simplify()` returns `false`
  immediately.
- Candidate retries are bounded by `max_shrink_candidates` per `simplify` call. Retry counts
  are not emitted per call; the limit path itself is exercised with `max_shrink_candidates = 0`,
  which emits exactly one `candidate_limit_reached (components, holes, vertices, sum_abs)`
  diagnostic, retains the unchanged valid case and returns `false`.
- Harness exhaustion is exercised through two controlled subprocesses. A `Just(1234)` property
  whose callback always rejects produced a controller end event with `"outcome":"exhausted"`,
  `"error_kind":"exhausted"`, `accepted_test_calls = 0`, `rejected_test_calls > 0`, and a
  `failure.json` classified `exhausted` retaining concrete input `1234`. A `GeometryStrategy`
  with `max_shrink_candidates = 0` and an always-failing predicate ran through the same
  `PropertyRun::check`; the shrink emitted `candidate_limit_reached`, the controller end event
  reported `"outcome":"failed"`/`"error_kind":"sut_failure"` with
  `"shrink_completed":null`, and `failure.json` classified `sut_or_contract` with the retained
  `GeometryCase`. Neither exhaustion path is reported as a pass or as a proven minimum.

## Concrete regression formatting

`domain_regression_fixture_compiles_and_preserves_bits` emits seven representative blocks (f32 NaN
payload, f64 negative zero, graph tuple, layer with non-default region identity, a
default-region FRU layer, signed-zero/NaN/
infinite floats and extremes, holed geometry, indexed mesh) and compiles/evaluates them in a
native temporary crate that depends only on `slicer-ir` and `slicer-sdk` with the `test` feature.
`cargo tree -e normal` in that crate is asserted to contain no `proptest`, `num-bigint` or
`num-rational`; the probe also runs with `PNP_PROPERTY_CONTEXT` removed. A lossless canonical
encoding (floats by `to_bits`, plus `RegionKey` object/region IDs and paint variants) is compared
against the originals. Dropping the layer `z` bit pattern or the `region_key` from the formatter
makes the comparison fail, so the probe is load-bearing. The readable fixture is retained under
`target/property-testing/<token>/regressions/fixture.rs`.

## Selected limits and reasons

Committed in `test-properties.toml`. Routine values keep every family sub-second; extended
values grow the domains within the exact-arithmetic cap of 1 000 000 canonical units.

| Property | Routine bounds | Extended bounds |
|---|---|---|
| `D-SCALARS` | `max_scalar_units=10000`, `max_id_bytes=64` | `100000`, `256` |
| `D-GRAPHS` | `max_nodes=16` | `32` (constructor cap) |
| `D-IR` | `16 / 32 / 8 / 8` | `64 / 128 / 16 / 16` |
| `D-GEOMETRY` | `1000 / 32 / 4 / 4 / 64` | `10000 / 64 / 8 / 8 / 256` |
| `D-MESH` | `128 / 128` | `1000 / 1000` |

The extended `D-IR` profile's `max_entities * max_points * max_layers = 16 384` stays below the
constructor's 100 000 allocation guard. Zero rejection budgets are honest because all generators
are total.

## Windows CI lane

The `properties-windows` job runs `cargo xtask test --summary --properties --tier routine` with
no hand-maintained record list, so the five `D-` records and `H-IR-ANCHORS` are discovered from
the registry. The routine lane was executed on Windows here. The workflow triggers on `push` to
`main`/`master`, so the merge-to-master push will produce the CI execution; that run is deferred
by project decision and its log is the post-merge check. Any failure there is a regression
against this packet.

## Direct G-GEOMETRY disposition

This packet proves the direct half of G-GEOMETRY: finite/exceptional scalars, scalar-config and
ID text, small graph and IR membership models, exact integer predicates, a bounded topology
catalog with protocol-correct shrinking, analytical mesh/transform families and compiled
concrete fixtures.

Remaining with `pbt-scripted`: OpenRSCAD/recipe/SCAD feature generation, scripted
operations/cavities/bridges and engine-conversion fidelity, and the **combined**
direct/scripted generality decision. The bounded catalog is not advertised as a general
simple-polygon generator; any broad arbitrary-polygon generator remains conditional on that
combined feasibility result. No SCAD coverage and no general-shrinker prerequisite is claimed
here.
