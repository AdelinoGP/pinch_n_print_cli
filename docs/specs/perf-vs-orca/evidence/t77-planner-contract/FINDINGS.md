# T77 — Tree-support planner clip-core attribution and the batch contract

Ticket: [Tree-support planner native union/carve batch-or-parallel host contract](../../issues/77-tree-support-planner-batch-parallel-contract.md).

The fog's entry condition for this lineage was "no new ticket without a new
batch/parallel host contract". This take supplies one, and refutes the shape
the fog's own wording pointed at.

## What ran

One instrumented slice of the standing matched job, ordinary external mode,
12 rayon threads, at the take-time HEAD:

- cell: **base classic supports-on** (`tmp/base.stl`, `pnp-classic-supports-on.json`)
- command shape: `pnp_cli slice --module-dir modules/core-modules
  --no-default-module-paths --no-integrated-modules --instrument-stderr
  --profile --profile-verbose`, `RAYON_NUM_THREADS=12`
- probe: `probe.patch` (temporary; reverted after the run). The guest tags its
  `clip_polygons` call sites on **site change only** (`PERF-T77-SITE`), the host
  brackets the clipper core of each call and appends one TSV line per call.
- output: `f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319`,
  43,912,792 bytes — **byte-identical to the preserved t58 reference** for this
  cell, so the probe is output-neutral.

Walls in the same run: `PrePass::SupportGeometry` **147,040 ms**, of which
`com.core.tree-support-planner` **126,674 ms** (86.1 %; the stage's other
producers are `host:support_geometry` 15 ms and
`com.core.traditional-support-planner` 200 ms).

## The measured split

Planner `clip_polygons` calls: **92,804**, clipper core **79.659 s = 62.9 % of
the 126.674 s module wall**. (A further 1,506 calls / 2.634 s are recorded by
the same process-global probe *after* the planner — later modules whose host
context starts untagged — and are excluded from every share below.)

| site | calls | core (s) | share of planner clip core | median | p95 | max | mean input polys |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `role_union` | 472 | 41.739 | **52.4 %** | 79.214 ms | 301.595 ms | 436.047 ms | 312.5 |
| `carve_region` | 69,161 | 35.779 | **44.9 %** | 0.474 ms | 0.905 ms | 35.884 ms | 1.96 |
| `set_diff` | 1,724 | 1.290 | 1.6 % | 0.477 ms | 2.245 ms | 14.001 ms | 1.66 |
| `ladder_union` | 20,275 | 0.662 | 0.8 % | 0.031 ms | 0.058 ms | 0.176 ms | 1.05 |
| `below` | 494 | 0.102 | 0.13 % | 0.228 ms | 0.334 ms | 0.402 ms | 1.69 |
| `cross_role_diff` | 73 | 0.078 | 0.10 % | 0.898 ms | 1.866 ms | 5.513 ms | 1.19 |
| `simplify_union` | 495 | 0.009 | 0.01 % | 0.016 ms | 0.032 ms | 0.092 ms | 0.69 |
| `move_union` | 110 | 0.001 | 0.001 % | 0.011 ms | 0.015 ms | 0.025 ms | 1.00 |

Tail: 3,556 calls ≥ 1 ms carry 47.771 s; 382 calls ≥ 10 ms carry 41.705 s; 187
calls ≥ 100 ms carry 31.558 s; the top 100 calls alone are **26.3 %** of the
recorded clip core.

## What this refutes

**The `layer_outlines_below` running-union chain is not an owner.** The static
read of `TreeVolumes::new` looks like a 495-step prefix union with a growing
accumulator — the obvious "prefix scan to parallelize". Measured: **494 calls,
0.102 s, 0.13 % of planner clip core**, at 0.23 ms median, because each layer's
outline contributes ~1 polygon and the accumulator stays around a thousand
vertices. Its consumers are predicates (`assign_node_group`'s
`is_inside_ex`/nearest-part test), so a tolerance argument would have been
available — and it is irrelevant, because there is no time there.

The same measurement deflates the other two shapes the fog carried:

- **t76's intra-call partition** (Morton-chunk a large union, merge once) has no
  target of that kind here. There is no 15-second union: `role_union`'s median
  is 79 ms and its max is 436 ms. Partitioning a 79 ms union into 48 chunks and
  merging is overhead-dominated; and its output *is* printed support geometry, so
  the sub-resolution-sliver tolerance that carried t76 does not transfer.
- **The 20,275 `ladder_union` calls are noise** (0.662 s, 0.031 ms median).

## What it establishes

**99.03 % of the planner's clip core (78.885 s) is inside `build_roles`**, in two
complementary shapes:

1. **472 big unions** — one per (layer, role) that has more than one region,
   each unioning a mean of 312 polygons (max 1,023) in one clipper call.
2. **69,161 tiny differences** — one per emitted region per role, each a single
   `clip_polygons([region], collision_polys, Difference)` followed by a
   largest-part `max_by` (median 0.474 ms).

Both are **independent per item**: the carve is a pure per-region map, and each
role union is a pure function of that (layer, role)'s region set. They are
serial today only because the guest is a single-threaded wasm module calling the
singular `clip_polygons` import inline, in a per-layer loop.

### The contract

`clip_polygons_batch` already exists in `slicer:common/host-services` and is
built for exactly this: a whole list of independent requests in one call, fanned
out over rayon by estimated work (`crate::batch::map_batch`,
`FANOUT_WORK_THRESHOLD = 4_096` primitives), with results returned in input
order (ADR-0049's "batching is cheap where guest-internal threading is not").

The missing piece is guest-side structure, not host capability: the emit pass
must buffer each layer's role inputs and issue batched carves/unions instead of
calling inline. Chunked buffering (N layers at a time) bounds memory while
keeping batches large enough to fan out.

**Exactness is by construction, unlike t76.** Each batch item is the *same
single clipper operation with the same arguments* as today; `map_batch` returns
the i-th result for the i-th request whether it ran serially or in parallel, and
the serial and parallel paths are byte-compared by the batch service's own
tests. This is the t65 precedent (batched `clip_polygons` over 12,022–19,290
subjects per cell, outputs byte-identical), not a tolerance argument.

### Idealized ceiling (arithmetic, **not measured**)

If the 78.885 s `build_roles` core scaled perfectly across 12 threads:
`78.885 × (1 − 1/12) ≈ 72.3 s` removed, taking the 147.0 s stage to ≈ 74.7 s.
This is an idealized bound and deliberately excludes: batch payload marshalling
(69k request records), buffering memory, the per-call WIT boundary saving that
is *not* in the measured core, imperfect scaling, and the 47.0 s of the module
that is not clip work at all (offsets, MST, node moves). The real win must be
measured by the standing paired gate.

## Limits

- One cell (base classic supports-on), ordinary external mode only, one
  instrumented run — an attribution observation, not a wall claim.
- The probe times the **clipper core only**. Guest marshalling and host WIT
  conversion sit outside it, so the 62.9 % is a lower bound on the call-path
  share and the ceiling above is conservative on that axis and optimistic on
  every other.
- The stage's remaining 13.9 % in this run is `host:support_geometry` (15 ms)
  plus `com.core.traditional-support-planner` (200 ms) plus inter-producer
  gaps; t73's 119.7–147.8 s stage range brackets this run's 147.0 s.

## Restoration

`probe.patch` applied and reverted with `git apply -R`; no probe string remains
in `crates/` or `modules/`. Guests rebuilt, `cargo xtask build-guests --check`
and `cargo xtask build-guests --accelerated --check` both exit 0, and
`target/release/pnp_cli.exe` rebuilt from the reverted tree. The raw capture is
gitignored under `target/t77-probe2/` and is reproducible from `probe.patch`
plus `reduce.py`.

Verification: `python verify.py` exits 0 against the committed summary and
rejects 8/8 negative controls.
