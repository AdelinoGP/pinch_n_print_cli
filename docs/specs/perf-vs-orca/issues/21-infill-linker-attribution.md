# infill-linker attribution

Type: task
Status: resolved
Assignee: current OpenCode session (wayfinder), 2026-10-01
Blocked by: 12, 20 — **superseded 2026-10-01**: the human selected this ticket
as the next map step after [Arachne critical-tail module
attribution](43-arachne-critical-tail-module-attribution.md) promoted its
low-layer subcost lead; ticket 12's chain places 21's acceptance before the
below-fold 19/20 polish, so the `20` edge does not gate this take. Scope
authorization is recorded below in `## Authorized scope`.

## Question

Where does `com.core.infill-linker`'s cost go, and which one optimization
candidate does the split justify?

It entered the ranked list at **15.3%** of classic fuel in accelerated mode and
is **bit-identical across modes** — acceleration never touches it
([Accelerated-mode pair](09-accelerated-mode-pair.md)). Per the glossary
(Infill linker, ADR-0025) its work is: connect raw infill segments into
continuous polylines per (region, role), apply the infill overlap offset
(Clipper2 offset on the wall-inset polygon), and re-clipping against the
partitioned fill polygons — the split must separate these.

Work:

- Re-attribute against the committed
  [Wall-flags annotation-free fast path](07-wall-flags-annotation-fast-path.md)
  first (hotspot shares older than that commit need re-derivation).
- Attribute linking vs overlap offset vs re-clipping (fuel and scopes, not
  wall claims), then propose **at most one** candidate from the split.
- Preserve the linking semantics exactly: per (region, role) linking,
  cross-region joins only inside a wall-sharing group, order-lock blocks
  untouched (ADR-0063).

Acceptance: paired-mode A/B, keep/drop to the human. Timing chain position 6.

## Current-job lead — low-layer Arachne tail

The separately authorized [critical-tail diagnostic pair](43-arachne-critical-tail-module-attribution.md)
now provides a concrete witness: infill-linker dispatch occupies 90.01–96.69%
of global layer 1/2 elapsed in the frozen supports-off Benchy Arachne job.
This promotes **low-layer linker dispatch subcost attribution**, not an
optimization based on whole-module timing. Split host preparation/marshalling
from guest orchestration, overlap offset, re-clipping, connection and ownership
assignment before selecting at most one candidate. Scope/authorization for
that next study is still required; the diagnostic pair authorized no fuel probe,
temporary source instrumentation, rebuild or implementation.

Currency correction: this ticket's earlier "bit-identical across modes" claim
belongs to its historical artifacts. The current frozen ordinary/accelerated
linker WASM hashes differ, verified against their manifests. Do not use that
historical claim as a present-day control.

Evidence: [T43 findings and measured dispatch-boundary caveat](../evidence/t43-critical-tail/FINDINGS.md).

## Authorized scope (2026-10-01)

The human selected this ticket as the next map step after ticket 43 and
authorized the take. Scope: one low-layer linker subcost attribution study —
host preparation/marshalling vs guest orchestration, overlap offset,
re-clipping, connection and ownership assignment — returning **at most one**
candidate per the original question. No optimization, no commit, no acceptance
retry, no default switch. Any fix needs its own scope plus the standing paired
ordinary + accelerated A/B.

## Answer — the re-clip owns the linker; the clip-universe pre-inflate owns the re-clip

Probe-scoped, deterministic fuel attribution on the frozen supports-off Benchy
Arachne job (4 captures: baseline, two ordinary probe iterations, one
accelerated probe; every output byte-identical to the frozen reference).
Full evidence: [FINDINGS.md](../evidence/t21-linker-subcost/FINDINGS.md),
raw captures under
`.local-artifacts/perimeter-reference-preparation/t21-linker-subcost-run1/`,
probe preserved in `probe.patch`, re-derivation in `verify-t21.py` (exit 0).

- **Boundary.** The layers 1–2 tail is **~99.95% guest execution**: the
  in-export wall (`now_us`) is 4,153.3 / 2,807.8 ms against `module_complete`
  4,155 / 2,808 ms, so host region-view preparation and dispatch/marshalling
  are ~1.7 ms / 0.2 ms. Ticket 43's unmeasured boundary closes for these
  layers, and it is not host-side.
- **Sub-operations.** Of linker fuel,
  `t21::path_reclip` (the per-path `clip_polylines` loop in
  `link_paths_without_offset`) is **97.65%**; `connect_infill` 2.01%; overlap
  offset 0.03%; graph build 0.02%; everything else ≤0.01%.
- **Inside the re-clip.** The clip-universe flatten + 1-unit `inflate_paths_64`
  is **64.84% of all linker fuel**; the clipper add/execute is 32.65%;
  0.15% glue. 4,520 calls over 212 invocations (+27 raw-boundary fallbacks),
  each clipping a 2-point polyline against a universe of up to ~5,700 points
  that is re-prepared every call.
- **Tail concentration.** Layers 0–2 carry 87.0% of linker fuel on 3 of 240
  dispatches (baseline capture); inside the split, 85.8% of the inflate and
  91.6% of the execute.
- **Modes.** The linker total and both sub-terms are numerically identical
  between ordinary and accelerated (±1.1e-7 relative) despite the differing
  WASM hashes; acceleration does **not** touch this work. In accelerated mode
  the linker is the **#1** slice fuel consumer (59.1%).
- **Whole-slice weight.** Linker fuel is 45.8% of the slice in ordinary mode
  (#2, behind arachne's 53.3%) and 59.1% accelerated (**#1**). It is the
  largest fuel term whose internal split now names a single dominant,
  value-preserving candidate.

**Candidate (one): hoist the clip-universe preparation out of the per-path
loop.** Every per-path clip in one invocation shares one `boundary`; preparing
its inflated universe once per invocation (≤239 preparations vs 4,520 calls)
removes 94.7% of the inflate term = **61.4% of linker fuel ≈ 28.1% of slice
fuel** (fuel ceiling; wall unmeasured). Value-identical by construction.
Needs its own scope/authorization plus the standing paired ordinary +
accelerated A/B before any keep; no implementation was attempted here.

`<module self>` note: the baseline capture's 99.83% unmarked share is now
accounted — `path_reclip` 97.65% + `connect_infill` 2.01% + residual 0.17%.
