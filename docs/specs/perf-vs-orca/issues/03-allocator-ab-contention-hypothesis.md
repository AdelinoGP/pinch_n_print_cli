# Allocator A/B closes the contention hypothesis

Type: task
Status: resolved

## Question

Does the suspected heap contention behind the parallel qualification phase's
inflated CPU (51.4 s aggregate worker CPU vs 33.2 s block wall vs 23.6 s
serial) justify switching to a scalable allocator?

## Answer

No — closed 2026-09-05 (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §10, `docs/specs/perf-vs-orca/evidence/alloc-bench/EXPERIMENT.md`).

Clean-machine A/B of System vs mimalloc vs snmalloc (benchy + base at 1 and 12
Rayon workers, 1 warmup + 5 measured interleaved, external wall/CPU/peak-WS with
cpu/wall per sample):

- **No wall win** at 12 threads (mimalloc −2%/−3%, inside spread) and none at
  1 thread on a quiet machine (system 131.1 s vs snmalloc 132.7 s).
- **Replacements lose on memory**: +5–23% peak working set across blocks.
- Calicat smoke G-code byte-identical across all three variants; `--report`
  intact under the wrapper; wiring reverted from the tree afterwards.

The motivating "CPU more than doubled under parallelism" reading is attributed
to a **new trap**: summed worker elapsed includes waits/descheduling and the
machine was loaded (benchy@1t measured 153–346 s contended vs 128–139 s clean —
a 2.5× band wider than any allocator effect). Any future timing claim must
quote cpu/wall per sample and exclude starved ones.

Do not repeat this A/B without new contention evidence (a real contention
signature reproduced on a quiet machine with a profiler).
