# t102 findings — global allocator substitution (mimalloc)

Ticket: [Global allocator substitution on the host
binary](../issues/102-global-allocator-substitution.md).
Take: 2026-10-30 → 2026-10-31, HEAD `cc2c54e6` + the two-file candidate.

**Measured result: the standing four-cell gate PASSES.** Wall falls
**−6.19 / −5.64 / −5.54 / −4.80 %** (ordinary classic-on / arachne-on,
accelerated classic-on / arachne-on) with process CPU **−1.74 / −2.18 / −1.54 /
−1.88 %**, **24/24 pairs byte-identical**, 6/6 pairs per cell, zero starvation
exclusions. The recommended ruling is **KEEP**; the candidate is not committed
until the human rules.

## Candidate

Two host files plus the contract doc, no guest change:

- `crates/pnp-cli/Cargo.toml`: `mimalloc = { version = "0.1.52", default-features = false }`.
- `crates/pnp-cli/src/main.rs`: `#[global_allocator]` becomes
  `AccountingAllocator<mimalloc::MiMalloc>` — the wrapping shape
  `docs/16_slicer_report.md` §"Global allocator contract" already prescribes for
  a packager, so `--report` accounting still works and no second allocator is
  declared.
- `docs/16_slicer_report.md`: the contract's example now names mimalloc.

Both arms ship byte-identical guest modules (host-only change): 24 modules each,
distinct host binaries.

| mode | baseline exe | candidate exe |
| --- | --- | --- |
| ordinary | `db153219b485e066…` | `1de06f27db99dc49…` |
| accelerated | `7f5ad486922fc668…` | `9986754e7cb1fa31…` |

## Proof matrix — 16/16

One run per arm in every cell (8 cells x ordinary/accelerated), uninstrumented,
12 threads. All 16 cells: baseline and candidate **byte-identical**, both arms
clean (`degraded=false`, `fatal=0`, `non_fatal=0`), non-vacuous role counts. The
**8/8 ordinary cells equal the post-t81 references**
(`evidence/t91-exact-z-memo/references.json`), and **8/8** ordinary ≡ accelerated
byte-equal. No reference drift; the allocator changes no emitted byte.

## Pre-gate — ordinary base classic-on, 3 alternating pairs

| pair | baseline | candidate | wall Δ | CPU Δ |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 259.70 s | 244.70 s | −5.79 % | −2.21 % |
| 2 | 246.80 s | 227.70 s | −7.74 % | −2.25 % |
| 3 | 265.88 s | 237.57 s | −10.65 % | −2.28 % |
| **paired median** | | | **−7.74 %** | **−2.25 %** |

3/3 pairs retained, zero exclusions, every output byte-identical. Clause:
proceed. (These walls are inflated by a heavy load event that ended before the
campaign; see Disclosures.)

## Paired campaign — gate PASS in all four cells

Four pre-declared base supports-on cells, 2 warmups + 6 alternating pairs each,
12 threads, external `GetProcessTimes` wall and process CPU, zero starvation
exclusions:

| cell | baseline wall | candidate wall | paired wall | wall Δ s | paired CPU | floor |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ordinary base classic-on | 155.66 s | 146.16 s | **−6.19 %** | −10.19 s | **−1.74 %** | 7.73x |
| ordinary base arachne-on | 154.17 s | 144.93 s | **−5.64 %** | −9.42 s | **−2.18 %** | 7.05x |
| accelerated base classic-on | 152.15 s | 143.68 s | **−5.54 %** | −9.04 s | **−1.54 %** | 6.92x |
| accelerated base arachne-on | 151.24 s | 143.29 s | **−4.80 %** | −7.41 s | **−1.88 %** | 6.00x |

**23 of 24 pairs are wall-negative**; the one positive pair is accelerated
arachne-on pair 6 (**+0.18 %**), and that cell's paired median is −4.80 % with
all six pairs retained. CPU falls in every cell, so the win is not thread
contention: the same work is done in less process CPU, which is what a cheaper
allocator should show.

The floor column is the measured wall delta divided by **0.80 %** of t91's cell
wall (1,318 / 1,335 / 1,306 / 1,234 ms), the map's pre-declared interpretation
bar. Every cell is **6–7.7x** it.

## Interpretation

The probe in [ticket 100](../evidence/t100-allocation-arena/FINDINGS.md) priced
only the whole-print free (3.527 → 1.302 s measured, 1.35–1.66 % of the cell).
The campaign measures **~5–6 %**, because the allocator acts on every allocation
in the process — prepass, marshalling, layer execution, finalization — not just
the 34.8M-block free. The free remains the single largest identified term, and
the block count is unchanged: this makes each allocation and free cheaper, it
does not remove any of them. That is still the job of
[ticket 101](../issues/101-support-identity-table-materialisation.md).

## Disclosures

- **Machine load.** The pre-gate ran while a stray integration-test binary
  (`integration-c6a6eb823ad8c0c8`) pinned ~8 cores (measured 24.3 s CPU per 3 s
  wall; system load 43–91 %), which is why its walls are 227–266 s. The human
  directed the campaign to run under known load; by the time it started the
  stray process had exited, and the recorded pre-campaign snapshot
  (`target/t102-load-precampaign.txt`) is **load median 1 %** with no
  `integration-*` process. Opera/Discord were idle. The campaign is paired and
  alternating, so any residual ambient load applies to both arms.
- **Baseline walls differ from the recorded cells** (152–156 s here vs t91's
  164.803 / 166.931 s) because of machine drift, so the campaign is read as
  **today-vs-today ratios**; the floor uses t91's larger walls, which makes the
  multiple slightly *smaller* than a same-day floor would.
- **The allocator is process-wide.** No stage-level attribution is claimed: the
  campaign cannot say which phases gained, only that the whole job does.
- **`--report` accounting is unchanged in shape** (the wrapper still sits
  outside the allocator) but was not re-measured for overhead in this take.
- **Memory is not gated** (out of scope per the map); mimalloc's peak working
  set was not recorded.
- **Dependency growth**: one crate (`mimalloc`) plus its small C library
  (`libmimalloc-sys`), built by the existing MSVC toolchain. The accelerated
  build hit one transient `cl.exe` failure compiling `mimalloc-static.cc` and
  passed on retry; the failed attempt produced no artifact and the retry is
  disclosed.
- **Candidate not committed.** The map's rule is that the human rules KEEP or
  DROP on the measured result; the candidate stays in the working tree until
  then.
- `verify.py --self-test` exits 0 against the committed artifacts alone:
  **12/12 claims, 12/12 negative controls rejected**.

## Reproduce

See [`REPRODUCE.md`](REPRODUCE.md).
