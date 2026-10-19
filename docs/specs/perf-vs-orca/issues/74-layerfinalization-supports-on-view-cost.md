# LayerFinalization supports-on whole-print view cost

Type: task
Label: wayfinder:task
Status: resolved (2026-10-19; human-approved KEEP — candidate committed
`091e6bc9`; standing paired gate PASS in all four cells with byte-identical
outputs; see ## Resolution)
Assignee: 2026-10-18 OpenCode wayfinder session — resumed on the human's
"continue" (no ticket named), so it took the first frontier ticket in order per
the map's standing selection rule. Graduated by the 2026-10-18 base refresh;
ticket creation is not execution approval.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 73

## Question

Why do the four whole-print LayerFinalization module brackets jump from
0.30–1.39 s on supports-off to **45.0–48.5 s** on supports-on, when their
combined guest fuel is **5.45 B of 12,964 B (0.042 %)** of the slice?

Fresh base captures ([findings](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md),
instrumented observer): classic-on modules 10.11 / 11.61 / 11.07 / 12.25 s
(classifier / part-cooling / skirt-brim / wipe-tower), stage 55.65 s; arachne-on
11.42 / 12.20 / 12.48 / 12.40 s, stage 60.22 s; supports-off controls
0.30–1.39 s total. The `overhang-classifier-default` early-return audit
([Consume-only-when-read annotations](20-consume-only-when-read-annotations.md))
says the matched configs return before reading a stamp, yet its bracket is
10–12 s on supports-on. Ticket 58 already removed the old per-module
whole-print copies (`execute_layer_finalization_with_instrumentation`
(`crates/slicer-runtime/src/layer_finalization.rs`)); the remaining bracket is
therefore expected to be host-side input-view construction
(`push_finalization_layer_view` (`crates/slicer-wasm-host/src/host.rs`),
`dispatch_finalization_call` (`crates/slicer-wasm-host/src/dispatch.rs`)),
guest dispatch, and output merge (`apply_finalization_pushes`
(`crates/slicer-wasm-host/src/dispatch.rs`)) — this is a hypothesis, not a
measured split.

Deliverables:

1. Decompose one supports-on cell's four brackets into host input-view
   construction, guest execution (including the early-return path) and output
   merge; state which term owns the 30–150x support-state jump and why the
   support entity volume is not proportional to the output-byte ratio.
2. Identify and test the cheapest representation-preserving reduction:
   materialize only the entity classes a module declares it reads, skip a
   module whose own liveness predicate is false, share an immutable input view
   where the contract permits, or a bounded delta protocol. Do not change
   output semantics.
3. If a candidate survives, qualify it on the 16-cell output-identity matrix
   against the preserved references and run the standing paired ordinary +
   accelerated A/B on the four pre-declared supports-on cells. Keep/drop is the
   human's.

Constraints: attribution first; no production edit before a measured owner;
output byte-identity is the hard gate; no default switch or auto-commit.
Profile/worker elapsed is not process CPU or uninstrumented wall.

## Acceptance

- A measured bracket split and a named owner for the supports-on jump.
- Either a candidate passing the standing gate, or a measured no-safe-reduction
  verdict with the refuted shapes recorded.
- Durable reduced evidence and a checkout-only verifier, per the map's
  evidence policy.

## Take log (2026-10-18 → 2026-10-19)

Attribution complete, candidate implemented, proof gate **16/16 PASS**, standing
paired gate **PASS**. Human-approved **KEEP**; candidate committed `091e6bc9`.
Full record:
[`evidence/t74-finalization-view/FINDINGS.md`](../evidence/t74-finalization-view/FINDINGS.md).

- Measured bracket split on ordinary base classic: **96.1 % of the 61,555 ms
  supports-on module sum is whole-print copy/free churn** (12,112 ms input
  snapshot clone, 16,653 ms store drop, 13,446 ms merge rollback clone,
  16,940 ms old-vector drop); guest call is 1,816 ms (2.9 %). Supports-off
  control is 1,290 ms.
- Owner: the full `LayerCollectionIR` clone/free, dominated by the IR-only
  `support_entity_identities` side table (23,751 rows on vs 0 off; each row
  carries two `String`s and a `demand_ids: Vec<String>`), not the module's
  declared reads and not the printed output-byte ratio (1.96x bytes vs 47.7x
  bracket).
- Candidate (committed `091e6bc9`; `candidate.patch` is the as-measured diff):
  share the input snapshot's entity
  list with the view via `Arc`, and skip the merge rollback clone for provably
  no-op / infallible streams (`order_lock_remap_is_noop` + validated annotation
  targets + append-only entity pushes). Fallible streams keep the rollback
  clone; `malformed_finalization_merge_rolls_back_prior_entity_push_on_both_transports`
  stays green.
- Candidate probe: module sum **61,555 → 2,775 ms (−95.5 %)**; supports-off
  **1,290 → 1,252 ms (no regression)**; outputs byte-identical to the t58
  references (`f60e5e9b…` / `20e7204c…`); 24 executor + 10 contract
  finalization tests pass.
- `split.json` + `reduce.py` + `verify.py` (exit 0, three negative controls
  rejected); `probe.patch` (reverted) and `candidate.patch` are in the evidence
  directory.
- The probe take made no wall or CPU claim: machine state drifted materially
  (baseline supports-on wall 571.9 s vs t73's 408.7 s). The standing gate below
  is the separate quiet-machine wall/CPU measurement.

### Proof and standing gate (2026-10-19)

- **Proof gate: 16/16 PASS.** Baseline and candidate snapshots built from the
  same commit (`67ef742b`) and working tree (only the two candidate source files
  differ; `provenance-{baseline,candidate}.json`), 24 external modules each.
  Every cell's baseline and candidate outputs are byte-identical, clean
  (`status=ok`, `degraded=false`, 0 fatal / 0 non-fatal) and non-vacuous;
  ordinary shas match the preserved references (`34886ccb…`, `76eb4e1d…`,
  `20e7204c…`, `f60e5e9b…`), plus the arachne shas (`b76d846a…`, `ae9d9061…`,
  `3ba16419…`, `a1bc8110…`). See
  [`qualification.json`](../evidence/t74-finalization-view/qualification.json).
- **Standing paired gate: PASS.** Four pre-declared cells, 6 alternating
  baseline/candidate pairs each, frozen snapshots, quiet machine (09:08–11:50);
  24/24 pairs retained (no starved pair), both arms clean, every pair
  byte-identical. `ab-reduction.json`; log `target/t74/measure-morning.log`.
- **Paired deltas (candidate vs baseline):** ordinary benchy wall −3.13 % / CPU
  −0.91 %; ordinary base −12.71 % / −2.30 %; accelerated benchy −3.89 % / −0.84 %;
  accelerated base −14.23 % / −2.97 %.
- **Base supports-on absolute wall:** ordinary 361.95 → 315.95 s (−46.00 s);
  accelerated 326.76 → 279.24 s (−47.52 s), inside t73's 45.0–48.5 s bracket
  attribution. CPU medians fall by ~45 s, matching a serial host copy/free.
- Outputs remain `76eb4e1d…` (benchy) and `f60e5e9b…` (base) in every pair. The
  previous evening's interrupted set remains preserved as
  `measure-interrupted-20261019` and is not used.
- **Human-approved KEEP (2026-10-19).** Candidate committed `091e6bc9`; the
  evidence set and this resolution are the follow-up docs commit. Pre-commit
  gates: workspace clippy `-D warnings` clean, `check-literals` 0 violations,
  `check-test-quality --report` 0 findings, 24 executor + 10 contract
  finalization tests pass, both guest freshness gates exit 0.

## Resolution

**KEEP (2026-10-19).** The whole-print copy/free churn owned by the IR-only
`support_entity_identities` side table was removed by (1) sharing the input
snapshot's ordered entity list with the view via `Arc` and (2) skipping the merge
rollback clone only when the order-lock remap is a no-op and the stream has no
fallible op. Fallible streams retain the rollback clone; the malformed-merge
rollback contract test stays green.

Measured: supports-on module brackets **61,555 → 2,775 ms (−95.5 %)** in the
probe split, supports-off unchanged (1,290 → 1,252 ms, inside noise), proof gate
**16/16 PASS**, and the standing paired gate **PASS** with 6/6 pairs in every
cell: wall −3.13 % ordinary-benchy / −12.71 % ordinary-base / −3.89 %
accelerated-benchy / −14.23 % accelerated-base, CPU −0.91 % / −2.30 % / −0.84 %
/ −2.97 %. Base supports-on wall falls 46.00 s ordinary and 47.52 s accelerated,
inside t73's 45.0–48.5 s bracket attribution. All measured outputs are
byte-identical to the preserved t58 references.

Production commit: `091e6bc9`. Evidence:
[`evidence/t74-finalization-view/`](../evidence/t74-finalization-view/FINDINGS.md)
(`qualification.json`, `ab-reduction.json`, `candidate.patch`, `verify.py`).
The remaining ~2.4 s of candidate brackets is guest call time (~85 %), now the
largest term.

Closeout: the standing ordinary and accelerated developer distributions were
rebuilt at the kept HEAD after the source and docs commits. Both `pnp_cli.exe`
hashes and all 24 modules per mode are byte-identical to the take-era candidate
snapshots (`ac01456d…` ordinary / `b8263a03…` accelerated); both guest freshness
gates exit 0.
