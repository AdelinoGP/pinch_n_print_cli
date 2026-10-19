# Perimeter spatial acceleration — grilling decision record

This is a scratch design record, not an implementation packet or authorization
to edit production code. Q1–Q17 below refer to this conversation's decisions.
The final shared-understanding confirmation is still required.

## Status and evidence

- The requested temporary probe removal is complete. Backups are in
  `tmp/perf-next/probe-removal-backup-20260907-235620/`; the earlier probe patches
  remain under `tmp/perf-next/`.
- Cleanup validation passed: `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo xtask check-literals`, `cargo xtask build-guests` (including Classic),
  and `cargo xtask build-guests --check` (exit 0).
- Design-phase builds captured release host and Classic/Arachne guest compiler
  arguments. Release, ordinary-debug, and optimized-core-debug integration-test
  targets were compiled with `--no-run`; no tests were executed in those captures.
- No candidate implementation or acceptance timing has run. No commit was made.
- Current hotspot evidence is `tmp/perf-next/PROBE-FINDINGS.md`; earlier handoff
  shares are superseded. Measurement methodology is fixed by
  `tmp/perf-next/EXPERIMENT.md`, `MEASUREMENTS.md`, and the repaired harness.

## Settled decision tree

| Branch | Decision |
| --- | --- |
| Candidate scope (Q1) | Include boundary distance/sign, overhang quartiles, and bridge lookup in both generators. |
| Index family (Q2) | Existing `rstar` R-tree dependency. |
| Reuse (Q3–Q4) | After investigating broader ownership, one context per region invocation, outside every wall pass. |
| Numerical gate (Q5, Q7) | Require controlled compiler invocations; a build-script-only gate is insufficient. |
| Oracle (Q6) | Retain independent legacy scans and exact full-output comparisons. |
| Timing schedule (Q8) | Four measured samples per variant/generator/workload: ABBA then BAAB. |
| Inconclusive evidence (Q9) | Stop and report; no automatic extra timing batches. |
| Acceptance threshold (Q10) | Disjoint candidate/baseline ranges in both process CPU and whole-slice wall, for each generator/workload. |
| Driver scope (Q11) | Include isolated caches, mode-aware guest freshness, artifact metadata, and the documented shared-guest-directory amendment. |
| Corpus (Q12) | Committed synthetic/existing fixtures plus local versioned postcard captures with explicit float bits. |
| Debug support (Q13–Q14) | Controlled debug optimizes `slicer-core` at level 3, retaining debug information/assertions. Guests retain release builds. |
| Driver mechanism (Q15) | Owned `RUSTC` shim, actual-argv validation, reserved cfg injection for canonical core library/unit tests, root rejection latch. |
| Query layout (Q16) | Separate trees, conservative computed-point envelopes, source-ordinal tie reduction, structural small-set and exceptional-input fallbacks. |
| Test plumbing (Q17) | Non-default feature-gated capture/scoped native query controls; WASM compared to its own preserved baseline. |

## Canonical audit and architectural boundary

The intact source is `OrcaSlicerDocumented/`. Canonical
`ExtrusionQualityEstimator::prepare_for_new_layer` and
`estimate_extrusion_quality` (`OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp`)
reuse an `AABBTreeLines::LinesDistancer` for previous-layer boundary queries.
Normal Classic/Arachne supported/unsupported perimeter splitting in
`PerimeterGenerator::process_classic`, `process_arachne`, `traverse_loops`, and
`traverse_extrusions` (`OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`)
uses polygon clipping.

This is a canonical design precedent for build-once/query-many indexing, not a
drop-in evaluator port. Preserve PnP's current arithmetic and classification
policies. The earlier audit of the GUI fork could not establish absence because
that fork removed native slicing.

The helper belongs in the proposed
`crates/slicer-core/src/perimeter_spatial.rs`, with one immutable context for a
region's borrowed boundary/band/bridge data and owned compact index records.
Construct it before repeated `ClassicPerimeters::emit_walls` and
`ArachnePerimeters::build_walls` calls in their respective
`modules/core-modules/*-perimeters/src/lib.rs` files.

Previous-layer boundaries may repeat by value across regions, but broader reuse
would require new identity/host-access plumbing. Quartiles are region-clipped;
bridge geometry is region-specific. No cross-region cache or WIT/IR/scheduler
contract change is selected. ADR-0012 is superseded historical precedent, not a
claim that an existing Arc companion still exists.

## Exact query design

### Independent legacy behavior

Retain `signed_distance_to_boundary`, `expolygon_to_path3d`, and
`point_in_any_polygon` (`crates/slicer-core/src/perimeter_utils.rs`) independently
from the indexed evaluation path. Retain `point_in_polygon_winding`
(`crates/slicer-ir/src/polygon_predicate.rs`) as the authoritative winding
predicate. Do not share new pruning logic with the reference oracle.

- Distance endpoints use the current `i64 -> f32 mm -> f64` conversion.
- Winding uses its existing direct `i64 -> f64 / 10_000.0` conversion.
- Bridge lookup retains integer `Point2` inputs and strict-boundary semantics.
- Distance includes contour and hole edges; sign remains membership in any
  outer contour, with the current predicate's hole behavior.
- Quartiles remain the maximum matching band value.
- Preserve original edge order for ties, signed zero, `None`/`Some`, closing
  repeats, widths, flags, and the final `distance + 0.5 * width` operation.

### Separate indexes in one region context

1. **Boundary edges:** two-dimensional f64 R-tree in the distance conversion
   domain. Flatten ExPolygons, contour before holes, and edges including closing
   edges in original order; store the flattened source ordinal.
2. **Boundary sign:** separate two-dimensional f64 R-tree with neutral X = 0,
   indexing conservative direct-f64 Y intervals for outer contours.
3. **Quartiles:** another neutral-X Y-interval tree, retaining band/polygon
   indices and quartile values.
4. **Bridges:** separate two-dimensional boxes in integer-coordinate space,
   represented with outward-rounded f64 bounds and queried conservatively.

`rstar` requires more than one dimension, hence neutral-X trees rather than a
one-dimensional tree. Do not use a mixed tagged tree across these coordinate
domains. Index choice is independent per query type: sets fitting a single
`rstar::DefaultParams::MAX_SIZE` leaf use the legacy linear path. This structural
cutoff is a selected design rule, not a measured tuning claim.

### Nearest-edge algorithm

For each coordinate, with the current converted endpoint values:

```text
d = b - a
computed_endpoint = a + d
envelope = [min(a, computed_endpoint), max(a, computed_endpoint)]
```

This bounds the actual computed closest point `a + clamp(t,0,1) * d` under the
audited arithmetic. Raw `[min(a,b), max(a,b)]` alone is not the selected bound.

For a finite f32 query:

1. Use nearest-envelope traversal to select any edge as an upper-bound seed.
   The envelope distance is a heuristic only, never the final segment metric.
2. Evaluate that edge with the unchanged legacy arithmetic to obtain `D`.
3. Compute `radius = D.next_up().sqrt().next_up().next_up()`.
4. Form inclusive query bounds `(q-radius).next_down()` and
   `(q+radius).next_up()` on both axes.
5. Retrieve with `locate_in_envelope_intersecting`; evaluate every candidate
   using the exact legacy projection/clamp/square sequence.
6. Initialize the final winner to `None`, then reduce lexicographically by
   `(distance_sq.total_cmp, source_ordinal)`. The seed does not preempt earlier
   equal-distance edges. No per-query candidate vector or sort is needed.
7. Preserve `sqrt() as f32`, followed by the original sign operation.

Proof boundary: ordinary rounded multiplication makes each component square no
greater than the rounded nonnegative sum; `next_up(D)` bounds the exact squares
of candidates capable of beating/tieing the seed. The inflated root and outward
query bounds cover subtraction rounding. Monotonic finite multiply/add over
clamped `t` bound every computed closest point by the constructed edge box.
Source-ordinal reduction is equivalent to the original strict-minimum scan.
This relies on the explicitly audited compilation contract below.

Nonfinite query coordinates use the complete legacy scan. Empty supplied
boundaries retain caller gating/debug-assert behavior; nonempty boundary lists
with no edges preserve zero. Width normalization remains after query evaluation.

### Winding envelopes

Use the winding predicate's direct-f64 coordinates. For each contour edge, the
Y interval includes both raw endpoint Ys and computed `a_y + (b_y-a_y)`.
Expand extrema by `f64::from_bits(1).sqrt().next_up()` and outward-round the final
bounds. Query at `[0.0, query_y]`, then evaluate the unchanged predicate.

Y-straddling is necessary for any winding contribution. The expanded computed
range also covers the predicate's zero-distance boundary check. Holes, short
contours, inclusive boundaries, and nonfinite fallback retain legacy behavior.
Sign uses `any`; quartile uses `max`, so candidate traversal order is immaterial.

### Bridge guard before pruning

Compute global bridge-contour coordinate extrema and spans once. For each point,
derive maximum query-to-extrema distances with widened arithmetic. Before any
bbox rejection, require all spans/extents <= `i64::MAX` and a checked u128 bound:

```text
span_x * query_y_extent + span_y * query_x_extent <= i64::MAX
```

This conservatively proves the legacy i64 subtractions, products, and cross
subtraction safe; subsequent widened ray products fit i128. If the guard fails,
run the full original predicate in source order before pruning anything,
preserving debug-panic/release-wrapping behavior. Otherwise, outward box
candidates use the original strict integer predicate. Empty sets return false.

### Generator-specific behavior

- Classic's nonplanar shell conversion retains distances with no quartile bands.
- Arachne follows its actual current wall path; do not impose Classic's
  nonplanar behavior on it.
- Both generators reuse their region context across every relevant pass,
  including Arachne's only-one-wall-top second pass.

## Controlled build driver

### Trust and arithmetic premise

Exactness means equality to the legacy oracle compiled in the same supported
configuration. It is not a universal cross-compiler bit-pattern guarantee.

The initial audited allowlist is an exact compiler identity, deliberately pinned
as a support contract rather than inferred from moving `stable`:

```text
release: 1.96.0
commit-hash: ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96
host: x86_64-pc-windows-msvc
LLVM version: 22.1.2
targets: x86_64-pc-windows-msvc; wasm32-unknown-unknown
```

Read/validate the real compiler identity on each driver session; never accept a
version range. Future allowlist updates require an explicit audit.

Rust/LLVM source audit: Rust's `call_simple_intrinsic` maps powi to `llvm.powi`;
LLVM's `ExpandPowI` lowers exponent two to ordinary multiplication in the audited
optimized configuration. Normal inlining/ThinLTO do not authorize `reassoc` or
`contract`. Scratch lowering confirms this for release host/WASM and the chosen
optimized-core debug configuration. Unoptimized Windows debug instead calls C
`pow` and is not covered by that multiplication premise.

### Mechanism selected in Q15

- Bootstrap `xtask` ordinarily; use that executable in shim mode through an
  absolute `RUSTC` path. Resolve/save the actual rustc executable first.
- Clear `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` in controlled children.
- Validate every actual shim argv, including inherited build-script probes and
  appended compiler arguments, before delegating to the saved compiler.
- Route `rustc_version_verbose` (`xtask/src/build_guests.rs`) through the
  selected compiler path too; its current `Command::new("rustc")` is a known
  integration point.
- Inject the reserved acceleration cfg only for the canonical core library
  source/identity and its unit-test compilation, after all validation. Include
  the corresponding check-cfg declaration. A build script does not grant it.
- Record and latch policy rejection at session scope. Reject publication even
  if a dependency swallows a rejected child's exit. Ordinary failing compiler
  feature probes that pass policy are not themselves policy violations.
- Unknown/unsupported explicit accelerated builds reject. Ordinary builds use
  the legacy path. This controls the audited trusted source closure, not
  adversarial process execution or deliberately modified compilers.

### Profiles and argument policy

Controlled host release keeps the root profile. Controlled host debug/test uses
Cargo's `profile.dev.package.slicer-core.opt-level=3` override with debug
information/assertions; unrelated debug crates remain unoptimized. Guest builds
retain their isolated-workspace release profile. Accelerated doctests are not
selected; normal doctests remain on the ordinary build route.

Policy inputs are actual records under `tmp/perf-next/driver-audit/`, parsed by
`inventory.py` into `inventory.json`. The earlier `summarize-driver-argv.py`
output is not authoritative: it ignored additional directory arguments and
misclassified option values as source operands.

The implementation policy must parse all tokens structurally and classify
version/print probes separately from compilation. Accept audited host targets
both implicit and explicit, plus the exact WASM target. `--test` is a bare
boolean. Repeated `--print`, `--cfg`, `--check-cfg`, link/search/extern/lint
arguments are legitimate; reject duplicate control/codegen **keys**, not every
repeated `-C` token. Validate path-valued arguments against their actual roles.

Observed codegen families to encode with profile-specific values:

- `opt-level=3` for accelerated core and optimized release crates;
- omitted opt-level for legitimate debug/build-helper compiles;
- `embed-bitcode=no`, `lto=thin`, bare `linker-plugin-lto`, bare `prefer-dynamic`;
- `strip=symbols` (host release) / `strip=debuginfo` (guest release);
- `debuginfo=2`, debug assertions `on`/`off` or the profile default;
- Cargo metadata/extra-filename hashes and build-root-contained incremental paths.

Core acceleration specifically requires the audited optimized configuration;
accepting an unoptimized helper does not authorize unoptimized core. Structural
native `-l` arguments are separately validated. Response files, unknown options,
raw LLVM arguments, sysroot/backend overrides, custom target CPU/features, and
unaudited numerical transformations reject. Do not broadly permit linker or
LLVM options merely because a dependency requests them.

### Cache, freshness, and publication

Use isolated accelerated host and shared-guest namespaces keyed by policy,
compiler, and mode; Cargo retains source/dependency/profile invalidation.
All accelerated guests still share one target directory in their mode namespace.
Amend the documented fixed `target/guests` rule intentionally for that namespace.
Do not allow ordinary artifacts to satisfy an accelerated cache lookup.

Guest freshness must include the requested mode/policy/toolchain identity while
retaining WIT artifact inspection, lock convergence, and the distinct exit codes.
Mode switches must not silently accept the opposite artifact. Metadata for
published snapshots records actual enabled/fallback mode and exact provenance.
Rejected builds do not publish mixed/stale host/guest snapshots.

Proposed explicit entry points are accelerated modes of `build-guests`, `dist`,
and `xtask test`; the test route retains literals and guest freshness preflights.
Acceptance artifacts omit the test-support feature.

## Parity fixtures and capture

- Keep programmatic adversarial fixtures and suitable existing repository
  perimeter fixtures committed. Follow `docs/21_data_defaults_and_fixtures.md`.
- Local Benchy/base captures stay under `tmp/rtree_query_corpus/`; do not commit
  supplied models or derived local corpora. Use versioned postcard with explicit
  integer coordinates and floating bits plus a readable provenance manifest.
- Dedicated opt-in recorder tests capture before changing the query algorithm.
  Store region geometry once and query/result records referring to it; preserve
  layer/object/region/generator identity and actual capture/pass provenance.
- Feature-gated adapter capture observes the actual prepared region data rather
  than guessing it from another adapter. In the WASM route,
  `push_slice_regions` (`crates/slicer-wasm-host/src/dispatch.rs`) applies region
  filtering before storing `SliceRegionData`; capture that actual projection.
- Extend/use `PerimeterCapturingLayerStageRunner`
  (`crates/slicer-runtime/tests/common/perimeter_harness.rs`) for output capture.
- Scoped native test controls can select indexed or legacy queries on identical
  module inputs. Feature-gated dependency support must not require dependency
  `cfg(test)`, which integration tests do not enable.
- WASM output comparisons use the same WASM inputs and their own baseline
  artifacts. Native-vs-WASM output is not the oracle; known adapter differences
  remain outside this optimization.
- Private core diagnostics prove both accelerated/fallback selection and fewer
  exact evaluations on separated synthetic sets. No global atomic counters or
  production report metrics are introduced.

Required cases include equal-distance edges across contours/holes/closing edges;
computed-endpoint reconstruction drift; signed zero; empty and degenerate rings;
nonfinite queries; negative and rounding-sensitive coordinates; hole-edge
distance with unchanged outer-contour sign; overlapping/touching quartiles;
strict bridge boundaries; bridge-overflow fallback before rejection; widths and
distance normalization; full closure/flags; multi-region isolation; Classic
nonplanar shells; and Arachne second-pass reuse. Fixtures must exceed structural
cutoffs where acceleration is expected and assert nonempty relevant output.

Compile and run meaningful narrow core/module/runtime tests in both controlled
and ordinary configurations. Explicitly enable core `host-algos` where required,
verify nonzero tests, and tee every Cargo test invocation to
`target/test-output.log`. Retain independent scalar comparisons in addition to
complete module-output regression checks.

## Fixed performance acceptance

For each workload, each generator, each isolated variant:

1. One excluded warmup per variant/generator/workload.
2. ABBA followed by BAAB; A is baseline host+guests, B is candidate host+guests.
3. Alternate generator order between the two blocks.
4. Four measured samples per variant/generator/workload; use 12 threads.

Workload progression: supports-off Benchy, tree-support Benchy, original
tree-support base. Re-run each generator's own isolated baseline interleaved
with its candidate; historical baseline medians do not replace the A samples.

Use `tmp/alloc-bench/run_bench.ps1` with `-ExpectedGenerator`; explicit config,
stderr claim holder, and G-code generator marker must agree. Keep final
GetProcessTimes CPU, creation-to-exit process wall, accumulated worker elapsed,
and fuel separate. Attribution/test-support builds never provide acceptance
timing. Validate source/host/guest/mode provenance before each campaign.

At each workload, for **each generator and each metric (CPU and wall)**:

```text
max(candidate measured samples) < min(baseline measured samples)
```

Only passing exactness and both metrics for both generators permits progression.
Overlap/conflict is inconclusive: stop and report, with no automatic added runs.
Exactness failure or clear regression is DROP. Preserve all samples and quote
their CPU/wall ratios; do not selectively discard slow samples for load.
Generator-validation failures invalidate the campaign rather than becoming
favorable replacement samples. Keep completion/fatal/nonfatal/degraded status
visible, including the known base tree-support condition.

After all stages, return measured KEEP/DROP/inconclusive recommendations for the
user's decision. No automatic commit or quiet acceptance relaxation.

## Next authorization

The branch decisions above are settled. Final confirmation of shared
understanding and permission to author/preflight the implementation-ready packet
is the remaining user decision. No implementation begins from this scratch
record alone. The packet must translate this design into concrete tasks and
verification commands, not defer the chosen architecture or numerical proof to
an implementation agent.
