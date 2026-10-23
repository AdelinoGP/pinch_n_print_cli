# Tree-support planner emit-batch chunk and request-cap sweep

Type: task
Label: wayfinder:task
Status: resolved (2026-10-22; measurement-only — no constant adopted: the
screening chose `(4, 4096)`, the frozen-snapshot confirmation measured it
**+6.30 % slower on wall in both pairs**, so the standing campaign was not
started; `4 / 256` stays — see ## Answer)
Assignee: 2026-10-22 OpenCode wayfinder session — human selected "Sweep the
t78 batching constants" from the route options after the frontier
re-derivation found no open child
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Which `(EMIT_BATCH_LAYERS, CLIP_BATCH_MAX_REQUESTS)` pair minimises the batched
planner's wall, and does the chosen pair beat the committed `4 / 256` baseline
under the standing paired ordinary + accelerated gate?

[Ticket 78](78-tree-support-planner-emit-batching.md) made the emit pass batch
each clip phase across four layers, and measured the planner's clip core
falling from 79.7 s to ~30.7 s — a **2.6x** effective speedup against an
idealized 12x ceiling. Its own Limits section names the unswept constants as
the obvious next lever: chunk size trades dependent-phase barrier count
against payload size, and the role-union phase carries only about one request
per layer, so four buffered layers hand the host only ~4 union tasks at a time
on a 12-thread pool.

The mechanism to test is the two constants, not the batching shape:

- `EMIT_BATCH_LAYERS` (committed value 4) sets how many layers' `RoleBuildInput`
  records are buffered before each dependent phase runs. Larger chunks give
  the union phase enough independent requests to fill the pool and cut the
  number of barrier round trips; smaller chunks bound buffered collision-set
  memory.
- `CLIP_BATCH_MAX_REQUESTS` (committed value 256) bounds the requests per host
  batch call. Larger caps cut sequential host calls per phase; smaller caps
  bound the marshalled payload.

Both are exactness-neutral by construction: every request is the same single
clipper operation with the same arguments as the serial path, and the host
returns the i-th result for the i-th request. Output identity is therefore a
gate, not a tolerance.

## Work

1. **Screen** the two constants on the base classic supports-on cell, ordinary
   mode, in-tree modules. `EMIT_BATCH_LAYERS` first at
   `CLIP_BATCH_MAX_REQUESTS = 256`; then the request cap at the best chunk
   size. Instrumented runs are screening-only (module/stage brackets + peak
   working set), never wall claims.
2. **Confirm** the chosen pair with an uninstrumented run against a same-session
   `4 / 256` uninstrumented control.
3. **Prove** the full 16-cell output-identity matrix against the preserved t58
   references, byte-identical in both modes.
4. **Gate**: the standing paired ordinary + accelerated A/B on the four
   pre-declared classic supports-on cells, 6 alternating pairs each, every pair
   byte-identical. Return KEEP/DROP to the human. No auto-commit, no default
   switch.

## Constraints

Exactness first: every arm must reproduce the preserved t58 reference hashes.
Instrumented and uninstrumented numbers are never mixed in a wall claim.
Profile/worker elapsed is never process CPU or uninstrumented wall. Guest
freshness gates must exit 0 before any module-dispatch conclusion. Buffered
memory is disclosed (peak working set per arm), not assumed away. Chunk sizes
that buffer the whole 431-layer job are out of scope.

## Comments

**Take opened 2026-10-22.** Frontier re-derived on resume: no open child (78
children, all resolved); the human selected the t78 constant sweep from the
route options. Claimed before any work. Preflight: both guest freshness gates
exit 0; `tmp/base.stl` and `tmp/3dbenchy.stl` present; preserved t58 references
`f60e5e9b…` (base classic-on) and `76eb4e1d…` (benchy classic-on) unchanged.

**Pre-declaration (recorded before any candidate measurement).**

- **Screening cell:** `ordinary-base-classic-on` — the cell where t78's
  planner cost is largest (`com.core.tree-support-planner` 109,018 ms baseline
  → 60,038 ms candidate in the t78 attribution pair), run with the matched
  config against in-tree modules and `RAYON_NUM_THREADS=12`.
- **Chunk sweep:** `EMIT_BATCH_LAYERS ∈ {1, 2, 4, 8, 16, 32}` at
  `CLIP_BATCH_MAX_REQUESTS = 256`; `4` is the control.
- **Cap sweep:** `CLIP_BATCH_MAX_REQUESTS ∈ {64, 256, 1024, 4096}` at the best
  chunk size from the first pass.
- **Screening statistic:** the `com.core.tree-support-planner` module bracket
  from one `--instrument-stderr --profile --profile-verbose` run per variant,
  with `PrePass::SupportGeometry` stage wall and peak working set recorded.
  One repeat is taken for any variant within 5 % of the leader; the final
  choice is the lowest module bracket, ties broken toward the smaller buffered
  footprint.
- **Confirmation:** one uninstrumented run of the chosen pair against one
  uninstrumented `4 / 256` control, same session, wall and external
  `GetProcessTimes` CPU. This confirms direction only; the standing gate is
  the decision.
- **Gate:** candidate pair vs committed `4 / 256`, frozen snapshots, 16-cell
  proof, then the four classic supports-on cells (benchy/base × ordinary/
  accelerated), 6 alternating pairs each, 2 warmups per arm. **Wall must fall
  in all four cells** (paired median); **CPU paired median at or below
  +3.0 %** (the t78 work-preserving ceiling, since per-item marshalling may
  legitimately add CPU); **6/6 pairs retained per cell**, zero starvation
  exclusions, every pair byte-identical. KEEP/DROP is the human's.

**Screening pass 1 — chunk sweep (recorded 2026-10-22, instrumented, screening only).**
Cell `ordinary-base-classic-on`; every run byte-identical to the t58 reference
`f60e5e9b…`; clean (`status=ok`, `degraded=false`). Statistic: the summed
`com.core.tree-support-planner` module bracket, one run per variant at
`CLIP_BATCH_MAX_REQUESTS = 256`:

| `EMIT_BATCH_LAYERS` | planner bracket |
| ---: | ---: |
| 1 | 92,875 ms |
| 2 | 75,503 ms |
| 4 (control) | 61,593 ms |
| 8 | 65,492 ms |
| 16 | 62,574 ms |
| 32 | 76,813 ms |

The committed `4` is at the knee: going smaller loses cross-layer union
parallelism, going larger (32) costs 25 % more. `16` is inside 1.6 % of the
control. Pass-1 process peak-RSS reads returned 0 MB (a harness bug, fixed
before pass 2), so no memory claim is made for these rows. The tables in these
pass notes are the contemporaneous readings; the final reduced medians across
all samples are the ones in `sweep-summary.json` and the FINDINGS (`4` =
61,519 ms at n=7; `64` = 71,158 ms at n=3; `4096` = 58,113 ms at n=6).

**Screening pass 2 — request-cap sweep at `EMIT_BATCH_LAYERS = 4`.** All clean
and reference-identical; peak working set ≈ 5.3 GB in every pass-2 run:

| `CLIP_BATCH_MAX_REQUESTS` | planner bracket | wall |
| ---: | ---: | ---: |
| 64 | 56,879 ms | 230.2 s |
| 256 | 61,593 ms | 261.3 s |
| 1024 | 61,044 ms | 245.6 s |
| 4096 | 58,544 ms | 255.4 s |

`64` looked like a 7.7 % leader and was repeated, but it did not replicate
(82,583 / 71,158 ms in pass 3). A host-load spike in pass 3 inflated every
variant's bracket and wall (slice elapsed 303–388 s against 228–259 s in pass
2), so cross-pass cap comparisons are confounded.

**Screening pass 4 — paired `256` vs `4096` (recorded before the gate).** One
contiguous window, interleaved (`256, 4096, 4096, 256, 256, 4096`), clean and
reference-identical. Planner bracket: `256` 59,551 / 61,519 / 56,146 ms
(median 59,551) against `4096` 58,184 / 57,540 / 57,390 ms (median 57,540) —
**3.4 % lower paired median**; wall medians 239.9 s vs 235.6 s (**1.8 %**).
Across every collected sample (`256` n=7, `4096` n=6) the medians are 61,519
vs 58,113 ms (**5.5 %**). The cap sweep therefore selects
**`(EMIT_BATCH_LAYERS = 4, CLIP_BATCH_MAX_REQUESTS = 4096)`**. The absolute
effect is small — the bracket is ~60 s of a ~240–280 s slice — so the standing
gate, not this screen, decides.

**Candidate declared: `(4, 4096)`.** Frozen snapshots were built for both arms
and their provenance passed: each mode differs in exactly one module
(`tree-support-planner`) and the `pnp_cli.exe` binaries are byte-identical.

**Confirmation — the candidate is slower (recorded 2026-10-22).** Two
alternating uninstrumented pairs on `ordinary-base-classic-on` with the frozen
snapshots, external `GetProcessTimes`, byte-identical output in every pair:

| pair | baseline (4/256) | candidate (4/4096) | wall delta | CPU delta |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 220.9 s | 231.8 s | **+4.97 %** | +0.95 % |
| 2 | 226.3 s | 243.5 s | **+7.63 %** | +1.73 % |

Paired median **+6.30 % wall / +1.34 % CPU**. The screening bracket did not
transfer to uninstrumented wall. The standing 16-cell proof and 6-pair campaign
were **not started** (stop discipline: no plausible path from a same-direction
+6.3 % to a PASS). Measured recommendation: **DROP — no constant changed.**

## Answer

**Resolved 2026-10-22 as a measurement-only take: no constant adopted; the
committed `EMIT_BATCH_LAYERS = 4` / `CLIP_BATCH_MAX_REQUESTS = 256` is the
measured choice.**

- **Chunk sweep (instrumented, screening only):** the committed `4` is the
  knee — bracket 92,875 ms at 1, 75,503 at 2, **61,519 at 4**, 65,492 at 8,
  62,574 at 16, 76,813 at 32. Smaller chunks lose cross-layer union
  parallelism; `32` costs 25 % more.
- **Cap sweep (instrumented, screening only):** `64` looked like a 7.7 %
  leader but did not replicate; a pass-3 host-load spike confounded the
  single-sample cells. An interleaved paired window gave `4096` a 3.4 % bracket
  edge (58,113 vs 61,519 ms across all samples, −5.5 %), so `(4, 4096)` was the
  pre-declared pick.
- **Confirmation (uninstrumented, frozen snapshots):** `(4, 4096)` was
  **+4.97 % and +7.63 % slower** on wall in the two pairs (paired median
  **+6.30 %**, CPU +1.34 %), outputs byte-identical. The standing gate was not
  run; the candidate is dropped and survives only as
  [`candidate.patch`](../evidence/t79-emit-batch-sweep/candidate.patch).
- **State:** planner source restored byte-identical to HEAD (blob
  `a59a11e5…`); both guest freshness gates exit 0; `target/dist` and
  `target/dist-accelerated` rebuilt from the restored HEAD. No production edit,
  adoption or commit.

Evidence: [`evidence/t79-emit-batch-sweep/`](../evidence/t79-emit-batch-sweep/FINDINGS.md)
(`verify.py` exits 0 checkout-only with 13/13 negative controls rejected;
`FINDINGS.md`, `sweep-summary.json`, `confirm.json`, `snapshots.json`,
`summary.json`, `candidate.patch`, and the reproducer scripts).
