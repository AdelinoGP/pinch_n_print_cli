# t91 findings — per-object memo for the z-independent exact-Z terms

Ticket: [Memoize z-independent exact-Z query terms per object](../issues/91-exact-z-object-geometry-memo.md).
Take: 2026-10-27, HEAD `a1185c0b`. Verdict: **GATE NOT MET** on the ticket's
pre-declared CPU clause by **+0.08 pp in one of four cells**; the wall clause
passed in all four. Measured recommendation returns to the human; nothing was
auto-committed.

## The candidate

`ExactZQueryService` (`crates/slicer-wasm-host/src/exact_z_query.rs`) now holds
a per-object memo of the terms that do not read the requested Z:

- the world-space transformed mesh,
- its bounds-derived build-plate rectangle,
- the cross-section at the object's **constant** maximum Z (`bbox.5`).

The public per-`(object, region, z)` query cache and its contract are unchanged:
every distinct key still resolves its own entry, and the region id still
participates in the key for a future region-clipping contract. Only the
z-independent work moved behind the memo. Concurrent first queries can build
the same terms twice; the first inserted entry wins and the terms are equal by
construction, so the retained values never depend on which thread built them.

A diagnostic accessor, `geometry_build_count()`, makes the memo's identity
contract falsifiable. The new contract test
`exact_z_memoizes_z_independent_terms_per_object`
(`crates/slicer-wasm-host/tests/contract/exact_z_support_query.rs`) asserts:
two Z values on one object build the terms once and see identical termination
surfaces and plate; a second object builds its own terms and never shares the
first object's plate; a third Z on the first object builds nothing; and the
requested-Z occupancy still tracks the requested Z.

**Exact by construction.** No memoized term reads the requested Z, so every
value is identical to what the pre-ticket code recomputed per query.

## Exactness

- **16/16 proof cells byte-identical between arms**, both modes: the full
  `{benchy, base} x {classic, arachne} x {supports off, on}` matrix.
- **8/8 ordinary cells** equal the derived post-t81 reference set
  (`references.json`, re-derived from the t81 campaign rows by
  `derive_references.py`).
- **8/8 cross-mode comparisons** equal (`ordinary == accelerated` bytes).
- **24/24 measured pairs byte-identical**, one G-code SHA per arm per cell.
- The two probe captures also reproduce the reference SHA
  `22200671…` — the probe changes counters, never output.
- Every run clean: `completion=ok`, `degraded=false`, `fatal=0`,
  `non_fatal=0`, non-vacuous role counts.

Both snapshots ship **24 external modules with byte-identical module trees**;
only `pnp_cli.exe` differs per arm (`provenance.json`). Both guest freshness
gates exited 0 before the snapshots were built.

## The paired campaign (the decision)

Four pre-declared base supports-on cells, 2 warmups + 6 alternating pairs each,
12 threads, external `GetProcessTimes` wall and process CPU, zero starvation
exclusions:

| cell | baseline wall | candidate wall | paired wall | paired CPU | pairs |
| --- | ---: | ---: | ---: | ---: | ---: |
| ordinary base classic-on | 175.128 s | 164.803 s | **−5.52 %** (−10.33 s) | **−1.22 %** | 6/6 |
| ordinary base arachne-on | 172.881 s | 166.931 s | **−3.84 %** (−5.95 s) | **−0.66 %** | 6/6 |
| accelerated base classic-on | 169.529 s | 163.258 s | **−2.68 %** (−6.27 s) | **+0.08 %** | 6/6 |
| accelerated base arachne-on | 161.794 s | 154.282 s | **−4.95 %** (−7.51 s) | **−0.32 %** | 6/6 |

**The CPU clause is the ticket's "flat or lower" (`≤ 0.0 %`), not the map's
+3.0 % work-preserving ceiling.** Three cells are flat or lower;
`accelerated-base-classic-on` measured **+0.08 %** — about **0.8 s of ~1,053 s**
process CPU — and that single cell is why the pre-declared gate is **NOT MET**.

Per-pair spread, disclosed rather than averaged away:

| cell | per-pair wall | per-pair CPU | pairs moving the wrong way |
| --- | ---: | ---: | ---: |
| ordinary classic-on | −8.03 … +0.94 % | −1.55 … −0.01 % | 1/6 |
| ordinary arachne-on | −6.57 … −2.69 % | −1.34 … +0.12 % | 0/6 |
| accelerated classic-on | −8.28 … +5.11 % | −0.99 … +0.59 % | 1/6 |
| accelerated arachne-on | −8.98 … −0.90 % | −1.43 … +0.49 % | 0/6 |

The two wrong-way wall pairs are single outliers under transient machine load
(the accelerated-classic pair ran 188.7 s candidate against 179.5 s baseline
while **both** arms sat above their medians; its CPU ratio is −0.31 %, so it is
not a starved pair and was retained under the protocol). The paired medians are
the declared verdict metric; the outliers are not hidden by them.

The CPU miss is sub-noise on this job: in that cell the six baseline repeats
alone span **1,040.8–1,061.0 s** of process CPU and the candidate's span
**1,040.9–1,062.0 s**, while the paired median moves by 0.8 s. The ordinary
counterpart of the same job shape measured **−1.22 %** CPU.

## Attribution

### The bracket (instrumented stage walls, never a wall claim)

`post_planner_gap_ms` = the `PrePass::SupportGeometry` stage wall minus the
support planners' own module walls. That bracket contains
`aggregate_support_plan_irs_degrading_with_attributed_diagnostics`, i.e. the
serial exact-Z validation loop:

| cell | baseline gap | candidate gap | drop |
| --- | ---: | ---: | ---: |
| ordinary base classic-on | 14.7 s | 9.9 s | **−4.83 s** |
| ordinary base arachne-on | 18.4 s | 7.4 s | **−11.02 s** |
| accelerated base classic-on | 21.8 s | 9.9 s | **−11.88 s** |
| accelerated base arachne-on | 13.8 s | 12.1 s | **−1.75 s** |

The gap falls in all four cells but its baseline size varies 13.8–21.8 s
because it also carries other host work; it localizes the change, it does not
price it.

### The probe (direct, ordinary base classic-on)

One probe snapshot per arm (`probe.py` + `build-probe.ps1` + `run-probe.ps1`,
`PNP_T91_PROBE=1`), both outputs equal to the reference SHA:

| term | baseline | candidate |
| --- | ---: | ---: |
| `query` calls | 431 | 431 |
| cache misses | 431 | 431 |
| **geometry-term builds** | **431** | **1** |
| time inside `ExactZQueryService::query` | **22,636.3 ms** | **8,922.3 ms** |
| `validate_entry` loop | **24,335.0 ms** | **10,004.3 ms** |

The memo removes **13,714 ms (−60.6 %)** from the instrumented query path and
**14,331 ms** from the validation loop. The remaining 8.9 s is the
requested-Z cross-section work, which is the only per-Z term left. This is the
direct evidence that the candidate removes work rather than spreading it — the
t89 parallel candidate did the opposite and paid +6.78 % CPU.

Instrumented timings are attribution only and are never mixed into the wall
claim above.

## Ceiling and floor

t89's arithmetic ceiling was **7.5 s / 6.8 s (4.1 % / 3.6 %)** of the cell,
derived by transferring a 12-way concurrent split to the serial arm. Measured
wall wins: **10.33 / 5.95 s ordinary**, **6.27 / 7.51 s accelerated** —
−5.52 / −3.84 / −2.68 / −4.95 %. Two cells beat the ceiling, one lands close,
and `accelerated-base-classic-on` lands below it (6.27 s against 7.5 s). The
0.80 % screening floor is cleared in every cell.

## Memory

The memo retains one world-space `IndexedTriangleSet` per object for the life
of the service. Measured on the matched fixture: `tmp/base.stl` loads to one
object with **1,230,602 vertices and 7,383,702 indices** → **44,302,032 bytes
(42.25 MiB)** at 12 B/vertex and 4 B/index. That is the retained cost, on top
of the source mesh the service already holds. It is recorded here, not treated
as a gate (the map's out-of-scope bullet excludes memory instrumentation, not
memory use).

## What was not done

- No production edit beyond the single memo file and its contract test.
- No commit, adoption, or KEEP decision: the gate miss returns the choice to
  the human.
- The probe is not part of the shipped candidate; it lives only in the
  reproducible scripts above and the `.local-artifacts` capture tree.

## Evidence

- `proof-rows.json`, `measure-rows.json`, `attribution.json`,
  `provenance.json` — committed reduced observations.
- `qualification.json`, `ab-reduction.json`, `summary.json` — reductions
  `verify.py` re-derives from those rows.
- `references.json` — post-t81 ordinary references, re-derived by
  `derive_references.py`.
- `verify.py --self-test` exits 0 against the in-repo set alone and rejects
  **13/13** negative controls.
- Raw captures live under
  `.local-artifacts/perimeter-reference-preparation/t91-exact-z-memo/`
  (gitignored, optional); every script that produces them is in this directory.
