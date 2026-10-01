# infill-linker attribution

Type: task
Status: open
Blocked by: 12, 20

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
