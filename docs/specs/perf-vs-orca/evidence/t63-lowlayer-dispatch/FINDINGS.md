# t63 — low-layer dispatch granularity: worker identity, mechanism, and a stale premise

Route: [Independent low-layer dispatch and Arachne finishing-tail reduction](../../issues/63-low-layer-dispatch-granularity.md).

## Verdict in one paragraph

The dispatch-granularity **mechanism is real and now established**: at 12
threads, rayon's indexed `par_iter().map(...).collect()` dispatch structure
puts all of layers 0, 1 and 2 on the initiating worker's left spine — they run
depth-first, strictly one after another, on one worker in every measured run —
and the WASM instance pools cannot be the wait (every per-layer-tier module is
`layer-parallel-safe = true`, so its pool has one slot per worker; measured
zero waits at 6 and 12 threads). But the 2,340 ms "low-layers-only interval"
that motivated the ticket is a **pre-t65 measurement that does not survive at
current HEAD**: on the same cells at the post-t65 tree the per-layer phase's
scheduling deficit (observed span minus the ÷12 work floor) collapsed from
**1,079–3,022 ms (12.7–49.9%)** across the six t61 captures to **161–221 ms
(2.4–3.6%)** in the arachne cells and 969 ms (7.0%) classic-on, with the
lone-open tail gone (0–61 ms). Attribution: the t65 batch-execute commit cut
the low-layer linker work that the spine was serializing (layer-1 linker
dispatch 3,191 → 1,228–1,662 ms; layers-0–2 linker accumulated 4.50–6.23 s →
1.39–1.80 s, ≈3–4×). The recommendation is **do not open the
output-identical dispatch-granularity experiment now**; the measured ceiling at
HEAD is ~0.12–2.3% of whole-slice wall on the Benchy cells, below the map's
"~1% effect cannot matter against the 6.28–26.24× gap" bar. The mechanism,
the counterfactual, and the candidate shape are recorded here so a future
fresh scheduling baseline that re-develops the deficit (e.g. after a change
re-inflating low-layer module work) can reopen the experiment without
re-deriving everything.

## Method and evidence

Two independent evidence sources, both attribution-only (never mixed into
wall claims):

1. **The t61 refresh captures reuse** (pre-t65 tree, quiet machine): the six
   reduced captures under `../../perf-refresh-2026-10-13/captures/` carry
   every layer's phase-relative start/end and every per-layer module-call
   bracket. `analyze.py` re-derives dispatch structure, occupancy, tails and
   bracket coverage from them (`summary.json`).
2. **Fresh worker-identity probe runs** (post-t65 HEAD `980fde14`): seven
   instrumented Benchy slices with an env-gated, observation-only probe
   (uncommitted; patch preserved as `probe.patch`, reverted after the batch,
   clean rebuild verified — see "Restoration" below). The probe prints
   `T63EVENT`/`T63POOL` lines carrying the rayon worker index, OS thread id
   and pool-acquire waits to the instrumented stderr stream. Raw captures
   (optional fuller source per the map's evidence policy) live under
   `target/t63-lowlayer-dispatch/`; `reduce.py` reduces them to the in-repo
   `probe_summary.json`; `verify.py` re-derives every headline claim from the
   in-repo set alone. Runs: arachne-on ×2, arachne-off, classic-on,
   classic-off at 12 threads, arachne-on at 6 and 24 threads.

`cargo xtask build-guests --check` exited 0 before the probe batch; module
provenance is all-external; every probe run's output is **byte-identical to
its t58 finalization qualification reference** with clean completion status
(`runs.csv`).

**Machine-load disclosure:** the probe batch ran under visible background
churn (arachne-on r2 wall 34.9 s vs r1 26.5 s; classic-on 35.1 s; layer-0
elapsed swinging 382→1,834 ms between reps). The t61 captures were captured on
a quieter machine (arachne-on instrumented wall 25.67 s). Structural claims
below hold in every run regardless of load; the quantitative deficit
comparison is anchored on arachne-on r1, whose wall/quietness matches the t61
captures most closely (26.5 vs 25.7 s).

## What the captures already proved (before the probe)

From `analyze.py` on the six t61 captures (12 threads, all cells):

- **Layers 0, 1, 2 start strictly sequentially in all six captures**, layer 2
  finishing last in every one; tail (last end − second-last end) 1,038–1,604
  ms arachne, 1,146–1,319 ms classic, with **exactly one layer open**
  throughout the tail.
- **The first wave of starts is a halving-cascade boundary set**: e.g.
  arachne-on instrumented starts layers `{0, 3, 7, 15, 30, 60, 75, 82, 90,
  120, 150, 180}` at t≈0 — twelve workers, each beginning its own sub-range.
  Layer 3 starts at 0 ms (not behind layer 2), so the three low layers are a
  special-case chunk of the split structure, not a general front queue.
- **The field is saturated at the front** (12 layers open for ~2.6–7.9 s per
  cell) — queue starvation is excluded as the cause of the low layers' late
  starts.
- **Module brackets cover the layer elapsed**: layer 1's module-call bracket
  sum is 3,369 of 3,669 ms (arachne-on instrumented); the ~300 ms uncovered
  is host-side inter-module work (region preparation, input staging), an
  order smaller than the layer's dispatcher cost. **No pool-wait gaps.**

## What the probe established (new evidence, post-t65 tree)

`reduce.py` → `probe_summary.json`:

1. **Worker identity: layers 0/1/2 are the initiator's left-spine chunk.**
   In all five 12-thread runs — and, more deeply, layers 0–3 in the 6-thread
   run — layers 0, 1 and 2 carry **worker index 0** (the thread participating in
   the global pool under the caller; OS tid `ThreadId(3)`), starting
   depth-first: each low layer starts within 0–112 ms of its predecessor's
   end. First-batch workers are exactly `{0..11}` (each worker starting its
   own sub-range's spine). At 6 threads, worker 0 runs 0, 1, 2 *and* 3
   sequentially before its deque empties — **the spine serialization deepens
   as workers get scarcer**.
2. **Pool waits: zero at 6 and 12 threads** (structurally expected: every
   per-layer-tier manifest is `layer-parallel-safe = true`, so
   `build_wasm_instance_pool` sizes each pool at `host_parallelism`
   (`available_parallelism`) — one slot per worker; a wait would need more
   than `available_parallelism` layers dispatching the same module).
   **Measured waits exist only at 24 threads**: 11 `T63POOL` events, 13–42 ms
   each — pool size 12 < 24 workers. This both confirms the pool mechanics
   and removes "module-pool serialization" as any part of the low-layer
   pattern at production thread counts.
3. **The 24-thread counterfactual breaks the spine and kills the tail —
   mechanism demonstrated.** At 24 workers the initiator's stack is emptied
   fast enough that layer 1 is stolen at ≈0 ms onto worker 17 and overlaps
   layer 0; layer 2 lands on worker 12, starts at 1,182 ms and ends at
   1,887 — **mid-field**, not last. The run's last-finishing layer is an
   ordinary one (194/237, ends 5,278/5,282 ms, 4 ms apart) and the lone-open
   tail is 0. Nothing Benchy-specific: same job, same binary, only steal
   width.

## The premise is stale at current HEAD (pre-t65 vs post-t65)

Observed per-layer-phase span vs the work-conserving floor (Σ layer elapsed ÷
threads; span − floor = the scheduling deficit above pure work; because a
layer's elapsed carries the concurrent field's own slowdown, the floor is
*inflated* under saturation and the deficit is a **lower bound** on the true
idle-worker time — the headline comparison is the directly-observed lone-open
tail, which has no such bias). ÷12 rows use
12 threads; the 24-thread row's Σ inflates from oversubscription and is not
floor-comparable (÷24 shown as context).

| capture | tree | span, ms | Σ elapsed, ms | floor, ms | deficit | lone-open tail |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| t61 arachne-on instrumented | pre-t65 | 5,393 | 37,260 | 3,105 | **2,288 (42.4%)** | 2,340 |
| t61 arachne-off instrumented | pre-t65 | 5,840 | 57,137 | 4,761 | 1,079 (18.5%) | 1,038 |
| t61 arachne-on profile | pre-t65 | 6,058 | 39,107 | 3,259 | 2,799 (46.2%) | 2,973 |
| t61 arachne-off profile | pre-t65 | 6,054 | 36,390 | 3,032 | 3,022 (49.9%) | 3,194 |
| t61 classic-on profile | pre-t65 | 9,516 | 99,732 | 8,311 | 1,205 (12.7%) | 1,146 |
| t61 classic-off profile | pre-t65 | 9,286 | 93,493 | 7,791 | 1,495 (16.1%) | 1,319 |
| probe arachne-on r1 (26.5 s wall) | post-t65 | 4,815 | 55,689 | 4,641 | **174 (3.6%)** | 0 |
| probe arachne-on r2 (34.9 s, loaded) | post-t65 | 9,316 | 109,140 | 9,095 | 221 (2.4%) | 0 |
| probe arachne-off (14.4 s) | post-t65 | 4,421 | 51,121 | 4,260 | 161 (3.6%) | 0 |
| probe classic-on (35.1 s, loaded) | post-t65 | 13,820 | 154,208 | 12,851 | 969 (7.0%) | 0 |
| probe classic-off (19.2 s) | post-t65 | 8,213 | 92,783 | 7,732 | 481 (5.9%) | 61 |
| probe arachne-on 6 threads | post-t65 | 5,956 | 34,399 | 2,867 | **3,089 (51.9%)** | 26 |
| probe arachne-on 24 threads | post-t65 | 5,282 | 121,872 | 10,156 (÷12) | n/a (÷24 = 5,078) | 0 |

The low-layer chunk the spine serializes is mostly linker work:
`com.core.infill-linker` dispatches on layers 0–2 sum to 4.50–6.23 s
accumulated across the pre-t65 captures (layer 1 alone: 3,191 ms) versus
**1.39–1.80 s (layer 1: 1,228–1,662 ms)** in the post-t65 probe runs — the
≈3–4× reduction measured (and committed) by the t65 batch-execute change. The
deficit collapsed with it; the arachne-on r1 comparison (both runs ~26 s
machine, quietness comparable) isolates the code change as the cause rather
than background load.

The map's route note attached to this ticket — *"the t65 commit is a
linker-cost change: any later scheduling diagnosis (63's family) needs a
fresh scheduling baseline"* — was the correct constraint: the ticket's
motivating interval was measured on the pre-t65 tree.

## Design comparison if the question ever re-opens (scope item 2)

- **Candidate shape (generic, output-safe in principle):** bound the leaves
  so no worker executes a multi-layer left-spine chunk — `with_max_len(1)`
  (single-item leaves) on the indexed iterator, per the measured mechanism:
  single-item leaves make every layer dispatchable the moment a worker frees
  (the 24-thread run demonstrates the end state from the other direction,
  via steal width). Rayon's indexed `collect()` preserves result order, so
  blackboard-commit-adjacent collection stays in authoritative global-layer
  order; per-layer stage ordering and the immutable-Blackboard boundary are
  untouched (each layer still runs its stages sequentially on one worker);
  cancellation and failure propagation semantics are unchanged. The pool
  sizes already cover any dispatch count a 12-worker machine can create.
- **Bounded split the other way (chunks ≈ n/threads) is the wrong direction**:
  it would deepen the initiator's spine (measured at 6 threads: worker 0 ran
  0, 1, 2, 3 sequentially; a chunk-of-20 design would hold layers 0–19 on one
  worker) and grow, not shrink, the tail.
- `par_bridge()`/independent-task queues are unnecessary at this granularity:
  single-item leaves already give per-layer dispatchable units with indexed
  semantics.
- **Why not open the experiment now:** the measured ceiling at
  post-t65 HEAD is ~0.12–2.3% of whole-slice wall on Benchy (deficit
  161–969 ms of ~15–35 s slices, and that surplus includes load inflation —
  the quiet-run point is arachne-on r1's 174 ms). The map's standing bar ("a
  ~1% effect cannot matter against the 6.28–26.24× gap") rejects a
  keep/drop campaign for a candidate whose measured upside is that small.
  Re-open the experiment only if a **fresh scheduling baseline** shows a
  large deficit again (e.g. a future change re-inflating low-layer module
  work — the pre-t65 42–50% deficits are the shape to watch for).

Secondary observation (not this ticket's decision): the ~300 ms of
uncovered bracket time inside layer 1 (host-side inter-module work — region
preparation, input staging) is the kind of residual the
[host:native attribution](../../issues/60-host-native-attribution.md) ticket owns.

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t63-lowlayer-dispatch/analyze.py   # t61 captures → summary.json
python docs/specs/perf-vs-orca/evidence/t63-lowlayer-dispatch/reduce.py    # raw probe tree → probe_summary.json (needs the optional raw tree)
python docs/specs/perf-vs-orca/evidence/t63-lowlayer-dispatch/verify.py    # checkout-only: re-derives every headline claim from in-repo evidence
```

`verify.py` passes from the in-repo set alone (`probe_summary.json` +
`summary.json` + the t61 captures); its negative controls reject altered
deficit/lone-open numbers, a broken spine invariant, and a 24-thread-run
claim of zero pool waits.