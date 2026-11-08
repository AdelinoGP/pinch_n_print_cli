# Global allocator substitution on the host binary

Type: task
Label: wayfinder:task
Status: resolved (2026-10-31; human-approved **KEEP** — the standing four-cell
gate PASSES: wall **−6.19 / −5.64 / −5.54 / −4.80 %**, CPU **−1.74 / −2.18 /
−1.54 / −1.88 %**, 24/24 pairs byte-identical, 6/6 pairs per cell. The candidate
is committed; see ## Answer)
Assignee: 2026-10-30 OpenCode wayfinder session — the human selected the
allocation-volume route, [ticket 100](100-allocation-volume-layer-collection.md)
measured the allocator arm at **1.35 % measured / 1.66 % corrected**, and the
human then ruled that the lever must be tested as a candidate rather than
parked as a packaging question
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Allocation volume of the whole-print layer
collection](100-allocation-volume-layer-collection.md) ("Family verdicts" — the
allocator arm)

## Question

Does replacing `pnp_cli`'s global allocator (the Windows system heap) with
**mimalloc**, wrapped by the existing `AccountingAllocator` exactly as
`docs/16_slicer_report.md` §"Global allocator contract" prescribes for a
packager, pass the map's standing four-cell gate — **wall strictly better, CPU
within +3.0 %, 24/24 pairs byte-identical** — and is the win large enough to be
a product-visible change rather than noise?

## Measured premise

From [ticket 100](../evidence/t100-allocation-arena/FINDINGS.md), ordinary base
classic supports-on, whole-print free inside `host:gcode_emit`:

| fact | value |
| --- | ---: |
| blocks freed | **34,796,481** (identical in both allocator arms) |
| bytes freed | **1,358,397,544** |
| system-heap free | **3,526,544 µs** (101.3 ns per block) |
| mimalloc free, measured | **1,301,783 µs** (upper bound: sampling build) |
| mimalloc free, corrected | **795,526 µs** (after the measured 0.506 s probe overhead) |
| measured saving | **2.225 s = 1.35 %** of the 164.803 s cell = **1.69x** the floor |
| corrected saving | **2.731 s = 1.66 %** = **2.07x** the floor |
| blocks removed | **0** — the same 34.8M blocks are still freed |

The probe priced the free only. The allocator acts on **every** allocation in
the process (prepass, marshalling, layer execution, finalization), so the
process-wide wall effect is expected to be larger than the free alone; it is
unmeasured.

## Candidate

Two host files, no guest change:

- `crates/pnp-cli/Cargo.toml`: `mimalloc` dependency (`default-features = false`).
- `crates/pnp-cli/src/main.rs`: the `#[global_allocator]` becomes
  `AccountingAllocator<mimalloc::MiMalloc>` — the wrapping shape the contract
  already documents, so `--report` accounting keeps working and no second
  allocator is declared anywhere.

The change is process-wide by construction: it can alter wall and CPU in every
stage, and it can alter no emitted byte.

## Pre-declared gate

**Proof (Phase 2, before any timing):**

- **16/16** cells (8 cells x ordinary/accelerated arms) byte-identical between
  the baseline and candidate arms, both arms clean (`degraded=false`,
  `fatal=0`, `non_fatal=0`), every cell non-vacuous on role counts.
- **8/8** ordinary cells equal to `evidence/t91-exact-z-memo/references.json`
  (no production change since t91 has invalidated that set).
- **8/8** ordinary ≡ accelerated bytes.
- Module trees byte-identical between arms (host-only change), host binaries
  distinct; provenance recorded per arm.

**Timing (standing four-cell gate):**

- Four pre-declared base supports-on cells: {ordinary, accelerated} x
  {classic, arachne}; **2 warmups + 6 alternating pairs** each, 12 threads,
  external `GetProcessTimes` wall and process CPU, zero starvation exclusions,
  **6/6 pairs retained per cell**, every pair's output byte-identical.
- **Wall clause (strict):** paired median wall **< baseline** in **all four**
  cells.
- **CPU clause:** paired median CPU **≤ baseline + 3.0 %** in every cell.
- **Pre-gate stop rule:** 3 alternating pairs on ordinary base classic-on; if
  paired wall ≥ 0 or paired CPU > +3.0 %, stop and report the miss.

## Pre-declared floor

**0.80 % of each cell wall**, from t91's kept arms: **1,318 / 1,335 ms**
ordinary classic-on / arachne-on, **1,306 / 1,234 ms** accelerated classic-on /
arachne-on. The wall clause is per-cell strict, so the floor is the
interpretation bar for "is this a product-visible win", not a separate clause.

## Deliverables

1. Proof matrix and provenance.
2. The paired campaign table (wall, CPU, pairs, bytes, exclusions).
3. A measured recommendation to the human: KEEP (commit the two-file candidate)
   or DROP (revert). No auto-commit.

## Disclosures to carry

- The allocator is process-wide; any stage's wall can move without a stage-level
  attribution, and a cell's CPU can move in either direction.
- The contract slot this fills is the packager slot; the candidate makes the
  repo's own shipped binary take that slot, which is a product decision the
  human has now ruled in scope.
- `mimalloc` builds a small C library (`libmimalloc-sys`); the binary's
  dependency set grows by one crate.

## Answer

**Measured KEEP — the standing four-cell gate PASSES, and the human approved the
candidate (2026-10-31, HEAD `cc2c54e6` + the two-file candidate).**

### Proof matrix — 16/16

One run per arm in every cell (8 cells x ordinary/accelerated), uninstrumented,
12 threads. All 16 cells: baseline and candidate **byte-identical**, both arms
clean, non-vacuous role counts; **8/8** ordinary cells equal to
`evidence/t91-exact-z-memo/references.json`; **8/8** ordinary ≡ accelerated. The
arms ship byte-identical 24-module trees and distinct host binaries (ordinary
`db153219…` vs `1de06f27…`; accelerated `7f5ad486…` vs `9986754e…`).

### Pre-gate — ordinary base classic-on, 3 alternating pairs

Paired median wall **−7.74 %**, CPU **−2.25 %**, 3/3 pairs retained, every output
byte-identical. Clause: proceed.

### Campaign — gate PASS in all four cells

| cell | baseline wall | candidate wall | paired wall | wall Δ s | paired CPU | floor |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ordinary base classic-on | 155.66 s | 146.16 s | **−6.19 %** | −10.19 s | **−1.74 %** | 7.73x |
| ordinary base arachne-on | 154.17 s | 144.93 s | **−5.64 %** | −9.42 s | **−2.18 %** | 7.05x |
| accelerated base classic-on | 152.15 s | 143.68 s | **−5.54 %** | −9.04 s | **−1.54 %** | 6.92x |
| accelerated base arachne-on | 151.24 s | 143.29 s | **−4.80 %** | −7.41 s | **−1.88 %** | 6.00x |

6/6 pairs retained per cell, zero starvation exclusions, **24/24 pairs
byte-identical**. **23 of 24 pairs are wall-negative**; the single positive pair
is accelerated arachne-on #6 (**+0.18 %**), and that cell's median is **−4.80 %**.
CPU falls in every cell, so the win is not contention — the same work is done in
less process CPU. Floor multiples are the paired wall delta over **0.80 %** of
t91's cell wall (1,318 / 1,335 / 1,306 / 1,234 ms).

### What this does and does not buy

The allocator acts on **every** allocation in the process, so the measured
**~5–6 %** is much larger than the free-only price from
[ticket 100](../evidence/t100-allocation-arena/FINDINGS.md) (3.527 → 1.302 s =
**1.35–1.66 %**). It changes **no emitted byte**, and it removes **no
allocations**: the census still shows the same 34,796,481 blocks, each freed
more cheaply. The cause-level route — the per-path support-attribution side
table — remains [ticket 101](101-support-identity-table-materialisation.md).

### Disclosures

- **Machine load.** The pre-gate ran while a stray integration-test binary
  pinned ~8 cores (24.3 s CPU per 3 s wall; system load 43–91 %), which is why
  its walls are 227–266 s. The human directed the campaign to run under known
  load; the stray process had exited by then, and the recorded pre-campaign
  snapshot (`target/t102-load-precampaign.txt`) is **load median 1 %**. The
  campaign is paired and alternating, so residual ambient load applies to both
  arms.
- **Baseline walls are today's** (152–156 s) against t91's recorded
  164.803 / 166.931 s, so the campaign is read as today-vs-today ratios; the
  floor uses t91's larger walls and is therefore conservative.
- **The allocator is process-wide**: no stage-level attribution is claimed.
- **`--report` accounting keeps its shape** (wrapper outside the allocator) and
  was not re-measured for overhead.
- **Memory is not gated** (out of scope on this map); mimalloc's peak working set
  was not recorded.
- **One dependency added** (`mimalloc` + `libmimalloc-sys`, built by the existing
  MSVC toolchain). The accelerated build hit one **transient** `cl.exe` failure
  compiling `mimalloc-static.cc` and passed on retry; the failed attempt
  produced no artifact.
- Candidate committed after the human's KEEP ruling; both guest freshness gates
  exit 0 (the change is host-only and touches no guest).

Evidence: [`evidence/t102-allocator/`](../evidence/t102-allocator/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 12/12 negative
controls rejected).
