# Ticket 79 — Tree-support planner emit-batch constant sweep

Take 2026-10-22 on HEAD `d06ccf2a`. **Measurement-only take: no constant is
adopted.** Screening selected `(EMIT_BATCH_LAYERS = 4,
CLIP_BATCH_MAX_REQUESTS = 4096)`, the frozen-snapshot confirmation then measured
it **slower on wall in both pairs**, and the standing campaign was not started.
The committed `4 / 256` stays; the planner source is restored byte-identical to
HEAD.

## Question

Which `(EMIT_BATCH_LAYERS, CLIP_BATCH_MAX_REQUESTS)` pair minimises the batched
planner's wall, and does the chosen pair beat the committed `4 / 256` baseline
under the standing paired ordinary + accelerated gate?

Ticket 78 measured the batched planner's clip core at ~30.7 s from 79.7 s
(2.6x effective) against a 12x idealized ceiling and named the unswept
constants as the obvious next lever. This take sweeps them.

## Screening

Instrumented ordinary `base classic supports-on`, in-tree modules, 12 rayon
threads. **Screening only** — the module bracket chooses constants; it is never
a wall claim. Every run was clean and byte-identical to the preserved t58
reference `f60e5e9b…`.

Chunk sweep at `CLIP_BATCH_MAX_REQUESTS = 256`, `com.core.tree-support-planner`
module bracket (sum of the per-layer `module_complete` brackets):

| `EMIT_BATCH_LAYERS` | planner bracket | samples |
| ---: | ---: | ---: |
| 1 | 92,875 ms | 1 |
| 2 | 75,503 ms | 1 |
| **4** | **61,519 ms** | 7 |
| 8 | 65,492 ms | 1 |
| 16 | 62,574 ms | 1 |
| 32 | 76,813 ms | 1 |

The committed `4` is the knee. Below it the union phase has too few independent
requests to fill the pool; at `32` the bracket is 25 % worse, consistent with
the comment's buffered-collision-set warning.

Cap sweep at `EMIT_BATCH_LAYERS = 4`:

| `CLIP_BATCH_MAX_REQUESTS` | planner bracket | samples |
| ---: | ---: | ---: |
| 64 | 71,158 ms | 3 |
| 256 | 61,519 ms | 7 |
| 1024 | 61,044 ms | 1 |
| **4096** | **58,113 ms** | 6 |

`64` looked like a 7.7 % leader in pass 2 but did not replicate (82,583 /
71,158 ms in pass 3). A host-load spike in pass 3 inflated every variant
(slice elapsed 303–388 s against 228–259 s in pass 2), so the remaining
`256` vs `4096` question was resolved by an interleaved paired window
(`256, 4096, 4096, 256, 256, 4096`): `256` 59,551 / 61,519 / 56,146 ms
(median 59,551) against `4096` 58,184 / 57,540 / 57,390 ms (median 57,540),
**3.4 % lower paired median**. Across every sample the medians are 61,519 vs
58,113 ms (**5.5 %**). Peak working set was ~5.3 GB in every measured run.

## Chosen pair and confirmation

Chosen by the pre-declared rule (lowest module bracket): `(4, 4096)`.

The pre-gate confirmation ran two alternating uninstrumented pairs on
`ordinary-base-classic-on` with frozen snapshots (baseline 4/256 vs candidate
4/4096):

| pair | baseline wall | candidate wall | wall delta | CPU delta |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 220.9 s | 231.8 s | **+4.97 %** | +0.95 % |
| 2 | 226.3 s | 243.5 s | **+7.63 %** | +1.73 % |

Paired median **+6.30 % wall / +1.34 % CPU**; every pair's output is
byte-identical. The candidate is slower, not faster, in both pairs — the
screening bracket did not transfer to uninstrumented wall. The standing 6-pair
campaign was therefore **not started** (the map's stop discipline: a cell that
fails is not rerun, and there is no plausible path from a same-direction
+6.3 % to a PASS).

**Measured recommendation: DROP — keep `EMIT_BATCH_LAYERS = 4` and
`CLIP_BATCH_MAX_REQUESTS = 256`.**

## Limits

- Screening is instrumented-only and the bracket is a proxy; this take is a
  direct demonstration that a bracket win need not survive into uninstrumented
  wall. No wall claim is made from any instrumented run.
- The chunk and cap tables are mostly single-sample. The `4` control has 7
  samples and `4096` has 6; the other cells are one each. Cross-pass host load
  is disclosed above rather than normalized.
- The confirmation is 2 pairs, not the 6-pair campaign. The campaign was not
  run because the direction check failed; this is a deliberate stop, not a
  gate result.
- Memory is disclosed only (peak ~5.3 GB); it is not a gate (out of scope on
  the map).

## Verification

- `python docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/verify.py`
  exits 0 checkout-only with **13/13** negative controls rejected.
- Snapshot provenance: each mode differs in exactly one module
  (`tree-support-planner`) and the `pnp_cli.exe` binaries are byte-identical
  between arms.
- Source restored: `git hash-object` of the planner `lib.rs` equals the HEAD
  blob `a59a11e5529db446caa3dcc39b444b8ca0821fb1`; `git diff` is empty for the
  guest tree.
- Both guest freshness gates exit 0 after the rebuild; `target/dist` and
  `target/dist-accelerated` were rebuilt from the restored HEAD.

## State

No production edit, no adopted constant, no production-source commit. The
candidate survives only as [`candidate.patch`](candidate.patch), a one-line
`CLIP_BATCH_MAX_REQUESTS` 256 → 4096 diff, re-appliable but not adopted.
