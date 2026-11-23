# Clipper engine vendoring and hot-path optimization

Type: task
Label: wayfinder:task
Status: resolved (2026-10-16; headroom take — measured headroom, not parity: the Rust port runs 1.42–2.86x slower than the canonical C++ per opclass on matched-workload inputs, and the conditional mapped scale exceeds the map's prior below-floor context; no candidate implemented in this take, one graduated; probe reverted; see ## Resolution)
Assignee: claimed and resolved by the 2026-10-16 OpenCode wayfinder
session — the human's route decision named this ticket for this take
("rely on subagents"); execution carried per the map's standing override.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Does bringing the clipper2 engine in-tree (`clipper2-rust` 1.1.0, the
production polygon engine) and optimizing its hot paths yield a byte-exact,
gate-passing reduction on the matched jobs — and if so, where does the
headroom actually live?

Context: the engine is the largest measured cost object in the base cells.
The t60 host:native capture's `polygon_ops` scope totals sum to ≈1,055 s
accumulated on base-off alone (`offset` 482.9 s / `closing_ex` 374.5 s /
`clip_polygons` 197.9 s, all owners — re-derived from
`evidence/t60-host-native/captures/base-classic-off-profile.json`), and the
t68 take measured that 97.55 % / 99.02 % of the classic-perimeters per-layer
round-trip bucket is these calls themselves
([findings](../evidence/t68-host-roundtrips/FINDINGS.md)). Every previous
ticket optimized how the engine is *used* ([clip-universe
hoist](44-clip-universe-hoist.md), [batch-execute](65-infill-linker-batch-execute-experiment.md),
[carve gate](32-emit-carve-gate-representation-and-ab.md)); the engine
*itself* has never been touched.

## Notes

- **Dependency posture (re-derive at take time).** The workspace `Cargo.toml`
  declares `clipper2-rust = "1.1.0"` (crates.io; the lock resolves 1.1.0,
  checksum `0fd663fe…`). Consumers: `crates/slicer-core` (`polygon_ops.rs` —
  host and guest; `triangle_mesh_slicer.rs`) and the guest modules
  transitively — the crate compiles into the wasm guests too, so any engine
  change is a guest rebuild + both freshness gates + byte-identity
  qualification on both legs. Version posture is settled: stay on the 1.1.0
  code base ([clipper2 1.2.0 delta](31-clipper2-1-2-0-geometry-delta.md); the
  bump is parked; mind [t17](17-clipper2-cost-output-verdict.md)'s harness
  trap — pin the dep and assert the resolved version from the harness's own
  lock).
- **Vendoring mechanics have precedent.** The t57 take vendored the crate
  locally
  (`.local-artifacts/perimeter-reference-preparation/t57-topx-cache/source/.local-libs/clipper2-rust`,
  with `clipper2-original` as the oracle twin) and drove both through an
  independent exactness harness. The in-tree version would be a workspace
  member or a `[patch.crates-io]` redirect; the evidence must carry the
  upstream commit + the local diff list.
- **Headroom first.** The bundled canonical Clipper2 C++ lives at
  `OrcaSlicerDocumented/deps_src/clipper2` (the parity reference; re-derive
  its version at take time). First measurement: profile the Rust port
  against the canonical on representative matched-workload inputs (t68's
  per-site input shapes; the t55 body-union corpus) to size the optimization
  headroom. If the port is already at parity, resolve as a measured
  no-candidate ([t59](59-prep-slice-flat-bridge-closing-reduction.md) /
  [t68](68-classic-perimeters-host-round-trips.md) precedent) rather than
  forcing an experiment. **Not a replacement candidate:** exposing the C++
  to the host leg while the wasm guests keep the Rust port risks host/guest
  output divergence (the t33/t47 oracle contract) — the C++ is the baseline,
  not the target.
- **Exactness contract.** Output geometry must stay byte-identical: any
  coordinate-level change is a parity event and out of this map's scope (see
  the map's Out of scope). Candidate optimizations must be value-exact
  against the pristine crate on identical inputs (t57's dual-engine oracle
  pattern), then pass the fresh 16-cell output matrix before any timing.
- **Candidate space (not pre-approved, to be measured):** hot-path
  allocation/aliasing, integer-op bottlenecks, data layout, redundant
  intersection sweeps, the `closing_ex` / `offset` inner loops (the two
  largest scopes), and wasm-side codegen effects — both legs must be
  qualified, since an optimization can help one and hurt the other.
- **Acceptance:** the headroom measurement first (or a measured no-candidate
  verdict); for any candidate — independent exactness (dual-engine
  value-identity + full 16-cell output identity), then the standing paired
  ordinary + accelerated A/B on pre-declared cells (6 retained pairs per
  mode, zero starvation exclusions), keep/drop to the human, no auto-commit.
  Probe reverted; freshness green; evidence under `evidence/t69-clipper-engine/`
  (reducer + verifier in-repo; raw tree gitignored); candidate-built binaries
  purged from the standing dirs after the take (t36 audit precedent).
- **Recipe constraints:** instrumented runs never mixed into wall claims;
  `--module-dir modules/core-modules` mandatory; `cargo xtask build-guests
  --check` exit 0 before any capture; outputs disclosed per the fairness
  contract.

## Resolution

**Verdict (headroom take, 2026-10-16): measured headroom — not parity, and
not a no-candidate.** On identical matched-workload inputs the pristine
production engine (`clipper2-rust` 1.1.0, pin- and lock-sealed) runs
**1.42–2.86x slower than the bundled canonical Clipper2 C++ 1.5.2** per
opclass, and the conditional mapped scale exceeds the below-floor context
this map last applied (t68's ≤ 0.80 % ceiling). The ticket's "headroom
first" clause is satisfied by measurement alone: **no candidate was
implemented, adopted, or timed against the slicer** in this take; one
candidate ticket is graduated
([70](70-clipper-in-tree-vendoring-first-candidate.md)) and awaits the
human's route decision. No production edit, no timing campaign, no
auto-commit; the capture probe is reverted and both freshness gates are
green.

- **Method — two legs, both checkout-only re-derivable.** (1) A dual-engine
  harness (`evidence/t69-clipper-engine/`: `setup-engines.py`, `driver.rs`,
  `driver.cpp`) executes the *same* corpus through the sealed Rust engine
  and a g++ 16.1.0 `-O2` build of the canonical C++ (seal asserts the
  harness lock resolves exactly 1.1.0 with no registry source — the t17
  trap; provenance in `results/provenance.json`). (2) The workload corpus:
  the t55 body-union specimens (16 cases, converted with the t55 harness's
  exact union semantics) plus a **new t68-site capture** — a host-only dump
  probe (`probe-t69-dump.patch`, applied and reverted) recording the exact
  engine-call inputs and parameters of the 11 classic-perimeters sites plus
  a best-effort region-prep leg, on both matched jobs, stratified-sampled
  (reduced in-repo ≈ 17 MB; raw gitignored). **Both capture runs were
  byte-identical to the preserved t58 references** (`34886ccb…` /
  `20e7204c…`), clean status, zero non-fatals; freshness exit 0 before and
  after; probe-built binaries purged from the standing dirs.
- **The measurement.** 295 cases × two full passes (file order, then
  globally reversed; fresh driver process per case; warmup + 5 timed
  repeats; engine create/add/execute/destroy inside the timer). Median
  Rust/C++ ratios per opclass (forward/reverse): **offset 2.0889 / 2.0336
  (177 cases), union 2.6280 / 2.6084 (16), intersection 1.9998 / 1.9409
  (34), difference 1.7608 / 1.5983 (68)**; per-site ratios span 1.18
  (site J, reverse) to 2.86 (accelerated-benchy union) — every site is
  materially above 1 in both passes. These are cross-compiler/allocator
  microbenchmarks sizing what a mature implementation of the same
  algorithm achieves: **headroom evidence, not a slicer-speedup claim.**
- **Value divergence (disclosed, not investigated).** 249/295 ordered-hash
  matches between the engines; 46 divergent (difference 38/68,
  intersection 8/34; offset and union 0/177, 0/16) with output
  path/ring/vertex counts preserved — consistent with the known 1.1.0-vs-
  1.5.2 `clean_collinear` convergence delta (t31). The exactness contract
  for any candidate is **candidate-vs-pristine-Rust**, never
  candidate-vs-C++; the C++ remains the baseline, not the target.
- **Headroom-to-scope mapping (ceilings, never wall claims).** With the
  t60 scope totals re-derived at take time from
  `evidence/t60-host-native/captures/base-classic-off-profile.json`
  (offset 482.93 s / closing_ex 374.47 s / clip_polygons 197.87 s
  accumulated, base-off), the idealized removable accumulated thread-time
  if Rust matched C++ per opclass is **offset 256.5–258.3 s, closing_ex
  198.9–200.3 s, clip_polygons 79.5–81.6 s** on base-off and one order
  smaller on benchy-off. **Coverage is partial and stated:** the corpus
  directly samples the classic-perimeters host-service shapes (whose t60
  owner is 17.79 % base / 22.24 % benchy of the offset scope); the RP leg
  captured only intersection calls, so the region-prep closing/offset
  portion of those ceilings is **conditional on within-opclass ratio
  transfer**. The directly-sampled CP-only accumulated ceilings are
  ≈ 46.7–46.8 s (base) / ≈ 2.5–2.7 s (benchy); as an occupancy-style
  diagnostic scale (÷ 12 threads against t68's *instrumented* walls of
  263.5 s base / 17.6 s benchy) that is ≈ 1.48 % (base) / ≈ 1.21–1.28 %
  (benchy) — above the ≤ 0.80 % below-floor context from t68, hence
  candidate-justified. Uninstrumented walls were not available from the
  cited captures and are not claimed.
- **Where the headroom lives.** Ranked by mapped ceiling: **offset**
  (the `offset`/`closing_ex` scope families, ratio ≈ 2.0–2.1), then
  **union** (≈ 2.6, the t55 body-union workload), then intersection/
  difference (≈ 1.6–2.0). **No engine-internal cost center was measured**
  (no engine profiling ran); the candidate direction is the measured slow
  opclasses, nothing finer — allocation/aliasing, layout, or sweep-level
  attribution belongs to the candidate take.
- **Stability + verification.** Two independent passes agree within 0.02
  (union) to 0.47 (site J) ratio points; order and engine precedence
  change together, so spread is descriptive, not a confidence interval.
  `verify.py` exits 0 against the in-repo set alone (no raw tree, no
  engine binaries — proven on a raw-free copy) and rejects four negative
  controls (changed timing, deleted case, shifted site, changed scope
  total). One runner interruption (a zero-duration empty-case repeat)
  was fixed by retaining clock-resolution zeros and resuming; no
  timing-driven exclusions.
- **Evidence:** `evidence/t69-clipper-engine/` (`FINDINGS.md`,
  `HARNESS.md`, `CAPTURE.md`, `SMOKE.md`, `probe-t69-dump.patch`,
  `campaign.py`, `analyze.py`, `verify.py`, `results/`, `corpus/`;
  `setup-engines.py`, `driver.rs`, `driver.cpp`, `reduce.py`,
  `verify-capture.py`, `t55-to-cases.py`). Raw runs gitignored under
  `.local-artifacts/perf-vs-orca/t69-clipper-engine/`.
- **Frontier.** [70](70-clipper-in-tree-vendoring-first-candidate.md) is
  graduated and open, unclaimed; [20](20-consume-only-when-read-annotations.md)
  remains parked below-fold. Route decision back to the human.
