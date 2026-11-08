# Ticket 78 — Tree-support planner emit-pass batching

Take 2026-10-22 on the pre-take HEAD `6fdb02ac`. The candidate is implemented,
the 16-cell output-identity matrix passes, and the standing paired ordinary +
accelerated gate **passes in all four pre-declared classic supports-on cells**.
Keep/drop is the human's.

## Question

Can the planner's emit pass buffer each layer's role inputs and issue its
per-region carves and per-role unions through `clip_polygons_batch` — keeping
every per-call result identical — and does the resulting wall win pass the
standing paired ordinary + accelerated gate?

t77 ([contract](../t77-planner-contract/FINDINGS.md)) measured **99.03 %** of
the planner's 79.659 s clipper core inside `build_roles`: 472 role unions
(52.4 %, median 79.2 ms, mean 312 input polygons) plus 69,161 per-region carves
(44.9 %, median 0.474 ms). Both are independent per item; the guest was calling
the singular import inline, one call at a time.

## Candidate

`modules/core-modules/tree-support-planner/src/lib.rs` (patch:
[`candidate.patch`](candidate.patch)):

- `build_roles` is kept unchanged as the **reference implementation**.
- `build_roles_batched(&[&RoleBuildInput])` is production. It runs the same
  phases in the same order — per-region carve (ticket-32 bbox gate evaluated in
  the guest), the lone-gated re-clip, one union per multi-region role, in-guest
  simplification, the set-wide difference, the body-minus-interface differences
  (one round per cut role), and the territory intersect — but each phase is one
  cross-item `clip_polygons_batch` call instead of N singular calls.
- The emit pass buffers each region's `RoleBuildInput` plus its entry metadata
  and flushes every `EMIT_BATCH_LAYERS = 4` layers, then once at the end, so
  entry order is the serial order.
- `run_clip_batches` slices a phase into ≤ `CLIP_BATCH_MAX_REQUESTS = 256`
  requests. The batch API is per-item, so each request carries its own clip set;
  the slice keeps the marshalled payload bounded independently of chunk size.
- `largest_part` is now shared by both carve paths, so the survivor rule
  (largest part, ties on the contour point list) cannot drift.

**Exactness is by construction.** Each deferred request is the same single
clipper operation with the same arguments as the serial call, and the host
returns the i-th result for the i-th request (`crate::batch::map_batch`), so
worker count cannot change the result. This is the t65 precedent, not a
tolerance argument: t76's sub-resolution partition tolerance does **not** apply
because the union output *is* printed support geometry.

## Snapshots

Two frozen snapshots per mode under
`.local-artifacts/perimeter-reference-preparation/t78-planner-emit-batching/snapshots/`,
built with `cargo xtask dist --edition developer` (plus `--accelerated`):

| mode | arms | differing modules vs the other arm |
| --- | --- | --- |
| ordinary | baseline (pre-take guest) / candidate | exactly 1: `tree-support-planner` |
| accelerated | baseline / candidate | exactly 1: `tree-support-planner` |

The `pnp_cli.exe` binaries are byte-identical between arms in both modes, and
every other module file is byte-identical too, so the snapshots isolate exactly
the planner guest.

## Qualification: 16-cell output-identity matrix

`run-all-proof.ps1` → `build-qualification.py`: **16/16 cells pass**. Per cell
the baseline and candidate arms are byte-identical **and** equal to the
preserved t58 reference; every run is `status=ok`, `degraded=false`,
`fatal=0`, `non_fatal=0`, with non-vacuous role counts.

| cell | sha256 | bytes |
| --- | --- | ---: |
| `ordinary-benchy-classic-off` | `34886ccb…` | 4,332,737 |
| `ordinary-benchy-classic-on` | `76eb4e1d…` | 8,658,663 |
| `ordinary-benchy-arachne-off` | `b76d846a…` | 4,126,744 |
| `ordinary-benchy-arachne-on` | `ae9d9061…` | 8,452,736 |
| `ordinary-base-classic-off` | `20e7204c…` | 22,428,863 |
| `ordinary-base-classic-on` | `f60e5e9b…` | 43,912,792 |
| `ordinary-base-arachne-off` | `3ba16419…` | 21,180,367 |
| `ordinary-base-arachne-on` | `a1bc8110…` | 42,664,440 |
| `accelerated-benchy-classic-off` | `34886ccb…` | 4,332,737 |
| `accelerated-benchy-classic-on` | `76eb4e1d…` | 8,658,663 |
| `accelerated-benchy-arachne-off` | `b76d846a…` | 4,126,744 |
| `accelerated-benchy-arachne-on` | `ae9d9061…` | 8,452,736 |
| `accelerated-base-classic-off` | `20e7204c…` | 22,428,863 |
| `accelerated-base-classic-on` | `f60e5e9b…` | 43,912,792 |
| `accelerated-base-arachne-off` | `3ba16419…` | 21,180,367 |
| `accelerated-base-arachne-on` | `a1bc8110…` | 42,664,440 |

## Standing paired gate

`run-all-measure.ps1` → `reduce_ab.py`: the four pre-declared classic
supports-on cells, 6 alternating pairs each, 2 warmups per arm,
`RAYON_NUM_THREADS=12`, external `GetProcessTimes`, frozen snapshots. **24/24
pairs retained, zero starvation exclusions, every pair's outputs byte-identical.**

| cell | wall (paired median) | CPU (paired median) | occupancy | pairs |
| --- | ---: | ---: | ---: | ---: |
| `ordinary-benchy-classic-on` | **−13.41 %** | +0.51 % | 3.46 → 4.12 | 6/6 |
| `ordinary-base-classic-on` | **−12.30 %** | +1.51 % | 4.06 → 4.76 | 6/6 |
| `accelerated-benchy-classic-on` | **−7.22 %** | +0.74 % | 3.79 → 4.50 | 6/6 |
| `accelerated-base-classic-on` | **−16.66 %** | +2.80 % | 5.14 → 6.34 | 6/6 |

Arm medians and ranges:

| cell | baseline median (range) | candidate median (range) |
| --- | ---: | ---: |
| ordinary-benchy-classic-on | 24.7 s (19.3–32.7) | 20.6 s (17.3–27.6) |
| ordinary-base-classic-on | 331.4 s (307.0–385.5) | 282.9 s (269.4–418.7) |
| accelerated-benchy-classic-on | 21.6 s (18.9–27.6) | 18.2 s (16.9–26.8) |
| accelerated-base-classic-on | 239.2 s (234.8–293.2) | 199.3 s (197.6–229.7) |

Per-pair wall / CPU, for the two base cells (the ones that decide the gate):

- `ordinary-base-classic-on`: 327→277 s (+1.3 % CPU), 322→282 (+1.7 %),
  351→419 (+2.3 %), 385→401 (+1.8 %), 307→269 (+0.7 %), 336→284 (+0.0 %).
  Pairs 3 and 4 are the window in which **both** arms slowed (the baseline
  itself ran 351/385 s against 307–336 s elsewhere); the paired median is
  −12.30 %.
- `accelerated-base-classic-on`: 293→230 s (+0.8 %), 274→228 (+1.4 %),
  235→198 (+3.0 %), 236→199 (+3.1 %), 240→199 (+2.8 %), 238→199 (+2.8 %).
  The **+2.80 % CPU median sits 0.2 pp under the pre-declared +3.0 % ceiling**;
  it is a real, consistent per-item marshalling cost in this cell, not noise.

Verdict per the pre-declared clause: **PASS** — wall falls and CPU stays inside
the work-preserving band in all four cells.

## Mechanism attribution

Same-instrument pair (`--instrument-stderr --profile --profile-verbose`, base
classic supports-on, ordinary), attribution only:

| quantity | baseline | candidate | delta |
| --- | ---: | ---: | ---: |
| `PrePass::SupportGeometry` | 127,723 ms | 81,362 ms | **−46,361 ms (−36.3 %)** |
| `com.core.tree-support-planner` | 109,018 ms | 60,038 ms | **−48,980 ms (−44.9 %)** |

Against t77's 79.659 s clip core, the ~29 s of non-clip module time is
unchanged, so the clip core fell from 79.7 s to ~30.7 s — a **2.6x** effective
speedup rather than 12x. That is the cost of eight dependent batch phases per
4-layer chunk, per-item marshalling (the accelerated-base CPU +2.80 %), and
fan-out that only pays when a phase's batch is large enough. The ticket's
idealized ceiling (78.885 s × (1 − 1/12) ≈ 72.3 s) was explicitly arithmetic;
the measured stage drop is 46.4 s.

## Limits

- `EMIT_BATCH_LAYERS = 4` and `CLIP_BATCH_MAX_REQUESTS = 256` are chosen
  constants, not swept. Chunk size trades barrier count against payload size;
  the 2.6x clip-core speedup suggests a sweep is the obvious next lever.
- Instrumented walls are attribution only; the standing harness rejects
  `module_complete`/`stage_complete` events in a timing cell for that reason.
- Base-cell wall variance is large (ordinary-base baseline range 307–386 s
  within the campaign). Alternation and per-pair ratios handle it, and
  starvation exclusion (cpu/wall < 0.5× the arm's own median) rejected none, but
  the two slow ordinary-base pairs are disclosed above rather than averaged
  away.
- The gate is the four pre-declared cells; the other twelve cells are proof-only
  (identity), not timing evidence.

## Tests and gates

- `cargo test` in `modules/core-modules/tree-support-planner` (its own
  workspace): **146 passed, 0 failed**. New test
  `build_roles_batched_matches_the_serial_path_per_item` drives four items
  (lone-gated simplify repair, carve + union, no-collision early return, and
  territory intersect) and asserts item-for-item equality with the serial
  reference.
- `cargo clippy --all-targets -- -D warnings` clean in the guest's workspace;
  `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask check-literals` 0 violations (159-type watchlist);
  `cargo xtask check-test-quality --report` 0 findings.
- Both guest freshness gates exit 0 after the rebuild
  (`cargo xtask build-guests --check`, `cargo xtask build-guests --accelerated
  --check`).
- [`verify.py`](verify.py) exits 0 checkout-only with **19/19** negative
  controls rejected across the take summary, the qualification matrix and the
  campaign reduction.

## State

The human approved **KEEP** on 2026-10-22; the candidate is committed as
`73a6f1e8`. Guests were rebuilt from the committed sources, both freshness gates
exit 0, and the standing `target/dist` / `target/dist-accelerated` artifacts are
built from the kept HEAD. Nothing else was adopted and no default changed.
