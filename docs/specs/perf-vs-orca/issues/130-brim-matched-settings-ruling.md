# Matched settings vs the 8 mm brim: close the asymmetry or keep it disclosed?

Type: grilling
Label: wayfinder:grilling
Status: open (unclaimed; human's decision)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Destination arithmetic after t123: re-aim, rewrite, or park?](128-destination-arithmetic-ruling.md)'s
  secondary decision (2026-11-08).

## Question

The destination says **"matched settings"**, but the two arms are not printing
the same adhesion geometry:

- PnP prints its **8 mm brim** in every cell. It is the skirt-brim module's own
  default (`modules/core-modules/skirt-brim/skirt-brim.toml` declares
  `brim_width = 8.0`), the matched config authors no brim key, and the emitted
  PnP appendix resolves `brim_width = 8` in all 16 PnP groups.
- The pinned Orca rig prints **none** (`brim_width = 0`, asserted in all 32
  runs).

Its slice-time price is small — `com.core.skirt-brim` measured at **~235 ms** for
the whole print ([t103](../evidence/t103-orca-rebaseline/SCOREBOARD.md)) — but it
is a job difference carried by every ratio, and it is a separate key from the
filament leg t98 repaired.

Which is it:

- **Match at 0.** Author `brim_width = 0` on both arms. PnP stops printing its
  product default; ~235 ms leaves PnP's side; every PnP G-code reference moves
  (a brim is real extrusion, not a no-op).
- **Match at 8.** Author `brim_width = 8` on both arms. Orca prints the same
  adhesion geometry and gets slower by its own brim; PnP's references stay. A
  same-value variant (e.g. 5 mm, Orca's 2.5 default) is equally matched and
  needs the rig pin updated to assert it.
- **Keep the disclosure.** Reword the destination to *"matched except the
  disclosed adhesion key"*, and keep the disclosure beside every scoreboard.
  No re-take and no reference move.

## Acceptance

- A recorded answer in the map's **Destination** wording or **Notes**, naming
  the value (or the disclosure) and which ratios it applies to.
- If the key is matched, a ticket for the re-take that moves the reference set
  and re-derives the scoreboard; the re-take keeps the standing 16-cell and
  all-8-cell gates.
- No production edit in this take.

## Answer

_To be recorded on resolution._
