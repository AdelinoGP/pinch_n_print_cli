# Post-t172 eight-cell two-tool scoreboard refresh (verify the two estimated base supports-off wins)

Type: task / measured board (two-tool OrcaSlicer 2.5.1 vs PnP ordinary + accelerated, no production change)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t172 OffsetConcaveCrossing port](172-port-offset-concave-crossing.md)
([`evidence/t172-offset-concave-crossing/FINDINGS.md`](../evidence/t172-offset-concave-crossing/FINDINGS.md)),
which landed +14.89–23.01 % wall across all eight cells on top of t169's +5.74–25.08 %,
putting `base classic-off` at an estimated **~0.82x** and `base arachne-off` at an
estimated **~0.89x** against OrcaSlicer (cross-window composition from t163's board).

## Question

**On a fresh, same-window eight-cell matched-pair board against the installed
OrcaSlicer 2.5.1, do `base classic-off` and `base arachne-off` officially move
the map's destination tally from 0/8 to 2/8 cells won (lower median
uninstrumented wall with process CPU corroborating), and what are the exact
ratios of record and remaining required cuts for all eight cells after t169 and
t172?**

## Why this ticket exists now

- Every ratio quoted since t163 (`1.04x`, `1.12x`, now `~0.82x`, `~0.89x`, and
  the six remaining `1.26x–1.99x` cuts) is a **two-step cross-window estimate**
  (`t163` board × `t169` paired saving × `t172` paired saving). Standing rule
  (t161): a cross-window composition is never a ratio of record and cannot close
  a destination cell.
- Measuring a single-window 8-cell × {OrcaSlicer 2.5.1, PnP ordinary, PnP
  accelerated} board (1 warmup + 3 measured runs/cell, matched keys asserted per
  Orca run, Orca binary sha256 stamped, PnP output verified 16/16 against t172's
  `reference.json`) establishes the official win count (`2/8` if the estimate
  holds) and replaces the two-step estimated cuts that t173 and the benchy route
  depend on.

## Scope

1. Verify `cargo xtask build-guests --check` and `--accelerated --check` both
   exit 0, and stamp the installed OrcaSlicer executable sha256/version + both
   PnP executable/module-tree hashes.
2. Run one interleaved 8-cell × {OrcaSlicer 2.5.1, PnP ordinary, PnP
   accelerated} board (1 warmup + 3 measured runs/cell, `RAYON_NUM_THREADS=12`),
   asserting all matched keys on every Orca run and verifying all 16 PnP outputs
   reproduce t172's active `reference.json` byte-for-byte with `degraded: false`
   and zero code-1200 diagnostics.
3. Reduce wall/CPU medians, ratios of record, required cuts, and the official
   destination tally (`N/8` won). Update the map's destination status and
   remaining cuts.

## Acceptance

- No production code changes; all PnP runs reproduce t172's `reference.json`
  byte-for-byte (`16/16`).
- `evidence/t174-post-t172-scoreboard/` with reduced board JSONs, `FINDINGS.md`,
  and checkout-only `verify.py --self-test`.
- Map: destination-status block with official ratios of record and win count,
  Decisions-so-far line, and updated remaining cuts for t173 / benchy fog.

## Answer

_(pending)_
