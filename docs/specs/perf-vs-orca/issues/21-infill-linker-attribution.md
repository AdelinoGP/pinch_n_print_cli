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
