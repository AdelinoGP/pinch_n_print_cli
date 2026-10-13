# Scanbeam TopX projection-cache experiment scope

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-10-07; human confirmed bounded host + guest experiment scope)
Assignee: agpen (current OpenCode wayfinder scope session), 2026-10-07
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 55

## Question

Should one scanbeam-local unchanged-edge `TopX` projection cache be implemented
as an isolated experiment, through what pinned-dependency integration, with
which invalidation/fallback exactness tests and standing paired-mode speed gate?

## Context and boundaries

[Large body-union exact corpus and native-internals investigation](55-large-body-union-corpus-and-internals.md)
establishes repeated identical projection calculations in a dominant native
sweep path and supplies a selected largest/hole-bearing full-role replay corpus.
Read its [findings](../evidence/t55-body-union/FINDINGS.md) before prescribing a
candidate. Counter/timer builds have observer overhead; cache benefit, state
cost and end-to-end impact are unmeasured.

Claim and settle this with the human before implementation. The cache must be
distinct from mutable intersection `curr_x`, preserve exact existing Rust
`TopX` arithmetic and tie/order handling, and invalidate or fall back on changed
edge geometry/slope/vertex, scanbeam y or recycled identity. Native host and any
other consumers affected by dependency integration must be identified; a private
copied-library diagnostic is not already an authorized production dependency.
Do not silently import canonical C++ joined-edge behavior or make a parity fix.

Keep the original projection/full-role chain as an independent oracle; selected
real replay alone does not test every invalidation path. Preserve role/holes/
integer order and require current clean matched outputs plus both freshness
gates before any later probe-free ordinary/accelerated speed campaign. Both-mode
acceptance, no automatic retry, human KEEP/DROP and no auto-commit remain standing
rules. This ticket authorizes no implementation, dependency change, new captures,
timing campaign, adoption retry or default switch by its creation.

## Comments

### Continuation claim — 2026-10-07

Claimed for the live experiment-scope decision. The earlier below-fold and
serial-floor leads remain parked under their recorded human route decisions;
this follows the newly graduated large-body-union lead. No implementation,
dependency change, capture or timing campaign is authorized by this claim.

## Answer — resolution comment, 2026-10-07

The human selected **Scope experiment**, **Host + guest trial**, **Explicit
path**, **Full matrix**, **Measured jobs**, and **Full-key guard**, then confirmed
the complete handoff. Authorize one isolated implementation/evaluation take,
not execution in this scope session. This resolution owns the contract below.

### Candidate and numerical boundary

- Cache only the existing Rust `top_x` projection calculated during
  `ClipperBase::adjust_curr_x_and_copy_to_sel` for reuse at
  `ClipperBase::do_top_of_scanbeam` (`clipper2-rust/src/engine.rs` in the
  experiment's pinned private source). Keep the unchanged `top_x`
  (`clipper2-rust/src/engine_fns.rs`) as the fallback and independent oracle.
  Preserve its branches, integer result, f64 slope arithmetic and existing
  half-to-even rounding; do not substitute PNP's coordinate-conversion rounding.
- One conservative record per active edge, separate from mutable `curr_x`.
  Reuse only when edge lifetime/identity, scanbeam y, bot/top coordinates, dx
  bit pattern and `vertex_top` all match. Missing or mismatched state recomputes
  with the original function. Reset/recycling cannot inherit another edge's
  record. Preserve terminal/horizontal handling, intersection processing,
  winding, equal-x ordering and output provenance. No mutation-site-only fast
  flag, approximate arithmetic, different sorter, geometry simplification,
  role coalescing, union skipping or canonical C++ joined-edge repair.
- The native evidence is a lead, not a savings estimate. Include cache
  allocation/initialization, key checks, storage, accesses and teardown in the
  candidate's total cost. Do not optimize guards into a second candidate during
  this take. Memory overhead may be disclosed for accounting, not judged under
  a new peak-RSS acceptance bound.

### Pinned integration and freshness

- Start from a separate pristine, provenance-checked `clipper2-rust` **1.1.0**
  copy, not the instrumented large-body-union harness or an edited Cargo
  registry. Preserve its licence/attribution. Keep patches, setup instructions,
  original/source inventories and lock provenance reconstructible in-repo.
- In isolated candidate source snapshots, replace the inherited dependency in
  `crates/slicer-core/Cargo.toml` with an explicit repository-relative private
  path and exact `=1.1.0` version. This trial intentionally affects the shared
  host **and** guest engine; it is not a tree-planner-only wrapper optimization.
  Identify consumers, resolve/audit the root and every affected independent
  guest graph, update candidate locks only, and prove they use the same patched
  source with no unrelated version drift. Permanent vendoring or an immutable
  fork, upstream publication and production adoption are separate decisions.
- `path_dep_manifests` and `guest_closure_input_paths`
  (`xtask/src/build_guests.rs`) follow explicit path dependencies and collect
  their sources, but do not resolve a path inherited via `workspace = true`.
  The explicit candidate seam avoids relying on that untracked inheritance.
  Verify actual closure membership and source-edit staleness sensitivity before
  accepting freshness; a root-only patch or forced rebuild without coverage
  proof is insufficient. Stop if the integration cannot meet this contract;
  do not silently change the checker or proceed with partial host-only wiring.
- Independently rebuild/check ordinary and accelerated candidate guests,
  including affected test-guests, and prepare complete external Developer host
  plus module snapshots. Both mode-specific freshness checks must exit 0 for
  the corresponding source state; preserve baseline and candidate inventories.
  Prove no mixed-arm, stale or opposite-mode artifact can enter the campaign.

### Exactness before speed

- Execute nonvacuous native and actual WASM engine tests against an unmodified
  pinned engine. Assert cache hits and fallback execution, not just successful
  instantiation. Cover every key mismatch, edge advance/geometry/slope/vertex
  changes, y changes, missing records, reset/recycling and an intersection that
  overwrites `curr_x` without changing the valid saved projection. Exercise
  terminal/horizontal/vertical edges, endpoint and half-to-even projection
  cases, signed slopes and equal-x tie/order behavior. Use analytical cases
  and negative controls to catch stale reuse and arithmetic/order changes.
- Compare ordered integer contours/holes and complete role results against the
  independent original-engine/full-role oracle, including the selected largest
  and hole-bearing corpus from
  [Large body-union findings](../evidence/t55-body-union/FINDINGS.md).
  Exercise affected boolean, offset and mesh-slicing consumers with relevant
  existing/adversarial tests on both targets; do not treat area equality,
  comparator self-tests or final G-code equality as engine correctness.
  Retain the corpus's documented coverage limits; do not claim all-layer replay.
- Fresh clean baseline/candidate G-code must be byte-identical for every
  {Benchy, base} × {classic, Arachne} × {supports off, on} job in both modes,
  using the current matched-pair configs and current post-repair output refs.
  Require correct generator, no degradation, zero fatal/nonfatal errors and
  nonvacuous role output where the job requires it. Disclose bytes/TYPE counts
  and warnings. Pass the **entire** qualification set before any timing batch.
  Missing inputs, unexplained reference drift or failed exactness stops the
  take; do not weaken assertions or silently fix unrelated geometry behavior.

### Bounded campaign and handback

- Time only Benchy/base **classic supports-on**, in ordinary and accelerated
  modes, using
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json`.
  Freeze a candidate-specific protocol before runs. Use
  [the standing paired protocol](../evidence/t45-quartile-bbox/protocol.json)
  for sampling/gates, adapting its job/identity and arm-difference fields to
  this shared engine change rather than retaining its Arachne-only metadata.
- Per fixture/mode cell: 12 threads, one excluded warmup per arm, six measured
  alternating AB/BA pairs. Derive starvation cutoffs separately per cell/mode/
  arm at 0.75 of that group's best CPU/wall ratio; discard an entire pair if
  either arm fails its cutoff. Require at least five retained pairs and lower
  candidate median wall **and** process CPU with negative paired median deltas
  in both metrics. Every timed cell must pass; do not pool cells/modes or
  resample automatically. These are protocol choices, not measured outcomes.
- Timing artifacts must be probe-free external-only snapshots; instrumentation
  and test-support code stay out. Charge the whole process, including cache
  preparation and teardown. Check every output. Fuel or instrumented buckets
  cannot establish a wall win. Stop on failed/inconclusive gates and return the
  result to the human; no automatic retry or one-mode exception.
- Preserve checkout-usable headline evidence, patches/tests and falsifiable
  reducers/verifiers, without committing the heavy model fixtures. Follow
  narrow-test feature checks and tee/read logs; guest-touching broad tests must
  use `cargo xtask test`. Run relevant build/all-targets Clippy/literal and
  touched-test-quality gates. This scope does not require the full workspace
  test suite.
- Restore pre-experiment production sources, manifests/locks and active
  distributions from audited baseline state, preserving unrelated work. Verify
  restoration and both freshness gates. Hand back a bounded KEEP/DROP
  recommendation with exactness, measured overhead, speed evidence and gaps.
  No automatic commit, permanent dependency adoption, acceleration adoption
  retry, default switch or Orca scoreboard claim.

Execution is [Scanbeam TopX projection cache: exactness + paired host/guest A/B](57-scanbeam-topx-cache-experiment.md),
open and unclaimed at this scope handoff. No candidate code, builds, captures
or timing ran in this session.
