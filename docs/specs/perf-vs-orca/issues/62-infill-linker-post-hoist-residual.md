# Infill-linker post-hoist residual and exact clip-work reduction

Type: task
Label: wayfinder:task
Status: resolved (2026-10-13; attribution + representation analysis complete —
the per-path execute owns the linker; candidate graduated to
[65](65-infill-linker-batch-execute-experiment.md) — see ## Answer)
Assignee: claimed and resolved by the 2026-10-13 OpenCode wayfinder session
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 61

## Question

What owns the current infill linker's post-hoist residual, and which
representation-identical reduction, if any, warrants a separately qualified
paired-mode experiment?

The [Benchy current bottleneck refresh](61-benchy-current-bottleneck-refresh.md)
measured the linker at 44.51% of all guest fuel on matched ordinary Benchy
Arachne supports-off (34.18% supports-on), with 99.73% module self and 76.52%
of linker fuel on layers 0–2. Profiling-off instrumentation retains the
low-layer tail: layer 1 linker dispatch 3.182/3.191 s and layer 2 1.389/1.179 s
(off/on). This is attribution, not an uninstrumented wall or speedup claim.

The accepted [Clip-universe preparation hoist](44-clip-universe-hoist.md) already
removed per-path flatten/inflate. `link_paths_without_offset`
(`modules/core-modules/infill-linker/src/orchestrate.rs`) still clips each path
independently; `PreparedPolylineClip::clip_open_subjects`
(`crates/slicer-core/src/polygon_ops.rs`) rebuilds `Clipper64`, adds the whole
prepared clip set and executes each call. **Do not reuse the old pre-hoist
percentage split as current attribution or repeat the hoist.**

## Scope and decision evidence

1. Qualify fresh clean outputs in both modes before temporary probes. Split
   preparation, per-call input/edge construction, clipping execution, metadata
   conversion/short filtering and connectivity with non-overlapping fuel scopes
   and guest/host boundary brackets.
2. Capture actual inputs and operation counts for layers 0–2 plus an ordinary
   middle-layer control, across both generators. Include raw-boundary fallback
   invocations; do not infer their reach from historical captures.
3. Replay the full real-input chain against an independent pre-change oracle.
   Preserve ordered integer paths, contour/hole edge inclusion, roles, owners,
   metadata, short-path policy, fallback and order-lock semantics. Batching
   subjects or narrowing the clip universe is not safe merely because the set
   union looks equal. Keep the ran-and-empty replacement contract intact.
4. Decide between a measured candidate with exactness evidence and a documented
   no-safe-reduction result. Native rerouting, if proposed, must measure native
   work too: disappearing guest fuel is not proof of a speedup.

## Acceptance and boundaries

This take is attribution/representation analysis only; no production candidate,
WIT change or timing campaign is authorized by creating the ticket. A surviving
candidate graduates separately to the standing ordinary + accelerated
uninstrumented wall/CPU gate, with fresh output qualification and a human
KEEP/DROP ruling. The dispatch-granularity investigation is independent;
do not double-count its potential benefit or run timing campaigns concurrently.

Evidence: [refresh findings](../evidence/perf-refresh-2026-10-13/FINDINGS.md),
its checkout-only `verify.py` and `summary.json`.

## Answer

**Resolved (2026-10-13) — attribution complete: the linker's post-hoist cost
is the per-path engine round trip itself, and one measured candidate
graduates.** Evidence:
[evidence/t62-linker-post-hoist/FINDINGS.md](../evidence/t62-linker-post-hoist/FINDINGS.md)
(`probe.patch`, `reduce.py`, in-repo `captures/`, `verify.py` exit 0).

Fresh qualification first (scope item 1): benchy classic/arachne × supports
off/on in **both modes, 8/8 cells byte-identical** to the
[t58 qualification references](../evidence/t58-finalization-copies/qualification.json)
before any probe; the six probe captures (4 profiled ordinary + 1
instrumented-only + 1 accelerated profile from a scratch snapshot copy)
reproduce the same references byte-for-byte — the probe never changed output.
Captured at commit `921306dc` with the refresh's own exe hash and both guest
freshness checks green.

**The split** (linker module self fuel, ordinary arachne supports-off; classic
cells agree in shape):

| term | fuel | share |
| --- | ---: | ---: |
| `t62::execute` — `Clipper64::execute` per path | 62,524,472,575 | **85.96%** |
| `t62::engine_add` — engine construction + add subjects + add clip per call | 3,215,819,329 | **4.42%** |
| `t62::connect_infill` (connectivity) | 3,442,526,642 | 4.73% |
| `t62::clip_prepare` — universe flatten+inflate, **once per invocation** | 1,890,706,516 | 2.60% |
| `t62::meta_convert` — per-output-point metadata scan | 907,887,504 | 1.25% |
| footprint `difference_ex` in boundary prep + all other named terms | 574,409,776 | 0.79% |
| unmarked entry wrapper | 184,859,441 | 0.25% |

The t44 hoist is confirmed working as designed: prepares == invocations ==
393 (one per invocation), and the once-dominant 64.84% pre-inflate term is
now 2.60%. What remains is the per-path rebuild of everything downstream of
the prepare: 12,022 clip calls across 393 invocations each construct a fresh
`Clipper64` and re-add the whole prepared universe — the universe is
**re-fed to fresh engines 57.3× per preparation** on arachne (49.5×
classic). Every subject is a 2–6-point open scan span; the per-call execute
cost is universe-dominated (fit ≈ −2.8M + 7,990·U, R² 0.869; universe term
89.9% of layer-1 per-call cost), which is exactly the batchable term.
Raw-boundary fallback reach is measured (scope item 2): 90 arachne / 102
classic fallback invocations over 87 arachne layers, including the t35
sparse-burst middle layers — each is another ordinary invocation and takes
the same treatment. Cross-mode agreement ≤5.4e-4 relative on all headline
scopes (accelerated run from a scratch copy; the standing snapshot untouched).

**Decision (scope item 4): graduate the batch-execute candidate.** Under the
same-cost assumption the uniform-by-invocation ceiling takes execute to
393/12,022 of current and removes engine_add per call: the linker drops to
≈**9.15 B ≈ 12.58% of its current fuel** (arachne off; layer-aware bound
7.36 B ≈ 10.1%), slice share 44.28% → ≈5.57%; classic off posts ≈13.31% of
current, 9.77% → 1.30%. These are assumption-based ceilings, not
measurements (the batched execute's universe pass is assumed to cost one
current per-call execute). Representation conditions for the execution take:
(1) per-subject output attribution must be re-derived — vendored
`clipper2-rust` 1.1.0 exposes no owner tags on open-path solutions; (2) the
batch shape is exactly `clip_polylines`'s existing one-shot engine usage
(AC-6 per-polyline independence), with ordering, short filtering, metadata,
`source_segments`/`majority_owner`, the t37 ran-and-empty contract and the
raw-fallback arm preserved; (3) subject–subject adjacency is the one
behavioural risk and the replay oracle exists for it; (4) native rerouting is
not proposed (displaced fuel proves nothing about wall).

Graduated: [Infill-linker batch-execute experiment](65-infill-linker-batch-execute-experiment.md),
with the standing paired ordinary + accelerated uninstrumented wall/CPU gate
and fresh output qualification; human KEEP/DROP. Production restored before
resolution (probe dropped, both freshness gates exit 0, staged linker hashes
back to `544f6183…`/`2c0a2b4d…`, post-restore spot qualification
byte-identical). No commit was made for this take; the probe and evidence are
preserved in the in-repo evidence set.
