# Benchy fixture replacement: re-record the benchy reference set and retire cross-swap benchy ratios

Type: task / measured re-baseline
Label: wayfinder:task
Status: open (unclaimed)
Blocked by: none
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Graduated from: [Union-phase request grouping](143-union-phase-request-grouping.md)'s
  2026-11-12 take. The user-supplied `tmp/3dbenchy.stl` was lost and replaced with
  a different export of the same model (240 layers / 48.0 mm, ~225.7k
  triangles). `tmp/base.stl` is byte-identical to the original — t139's four
  base hashes still reproduce **exactly** — so only the benchy half of the
  reference currency moved.

## Question

`tmp/3dbenchy.stl` is a different export from the one every prior benchy number
was measured on. The replacement prints the same model (240 layers, 48.0 mm,
first layer equal) but emits different G-code: benchy classic-on hashes
`dceb7a48…` where t139 recorded `b3f1eccc…`. What is the benchy reference
currency now, which pre-swap benchy claims survive, and what does the map quote
until a post-swap scoreboard exists?

Decide and record, in order:

1. **The new benchy reference set.** [t143](143-union-phase-request-grouping.md)'s
   output campaign already recorded all four benchy hashes from the baseline
   arm on the replacement fixture: `a89e449b…` (classic-off), `dceb7a48…`
   (classic-on), `3286449f…` (arachne-off), `be786718…` (arachne-on). Promote
   them from a local disclosure to the active set, in whatever artifact the map
   treats as the currency ([t139](139-layer-plane-parity.md)'s set is the
   precedent).

   **Update (2026-11-12, [t144](144-brim-anchor-reconstruction.md)):** the
   active set is now t144's candidate set
   ([`evidence/t144-brim-anchor/reference.json`](../evidence/t144-brim-anchor/reference.json)),
   not t143's baseline hashes — the anchor reconstruction moved four of the
   four benchy cells (`1ae0cfa8…` / `d9fc80ba…` / `8faf02b1…` / `cd7d4a2d…`) and
   is byte-identical on both build arms. Treat t143's four hashes as the
   pre-t144 baseline, not the currency. The remaining items (2–5) are unchanged.
2. **What the swap actually changes.** A delta audit between the old and new
   export cannot be taken directly (the old file is gone); reconstruct it from
   the retained t139 G-code if a copy exists, otherwise from the measured
   invariants (layer count, first-layer height, per-layer areas on the retained
   evidence) and say plainly which claims are re-derivable and which are not.
3. **Retire the pre-swap benchy claims.** Every benchy G-code hash, layer-0
   cross-section area, brim/skirt oracle band, A/B pair and scoreboard median
   measured against the old export is historical. Base claims are unaffected.
   The map must mark them rather than leave them quotable.
4. **The post-swap benchy scoreboard.** Orca's four benchy cells must be sliced
   from the replacement file with the matched keys pinned and asserted per run,
   then paired against PnP's four benchy cells. Until that exists, no benchy
   ratio of record may be quoted; the base-only ratios stand with the
   machine-state disclosure.
5. **The disclosure rule.** A benchy number is either pre-swap (historical) or
   post-swap (current); never mixed, never quoted across the boundary.

## Pre-declared gate

This ticket is a re-baseline, not a candidate: no performance gate. Its
correctness bar is the fairness contract already used by
[t130](130-brim-matched-settings-ruling.md) / [t139](139-layer-plane-parity.md):

- the four benchy hashes are recorded and reproduce on both build arms of the
  take that records them;
- the Orca rig pins the matched keys on every benchy run
  (`enable_arc_fitting = disabled`, `elefant_foot_compensation = 0`,
  `brim_width = 8`, `skirt_loops = 6`, `skirt_distance = 3`, `skirt_height = 1`,
  `initial_layer_line_width = 0.4`, `tree_support_brim_width = 0`) and asserts
  them fail-closed;
- every retired benchy claim is named in the map, not silently dropped.

## Acceptance

- A reference-set artifact carrying the four benchy hashes, linked from the map.
- The swap classified: tessellation-only vs geometry change, with the evidence
  that supports the classification.
- A post-swap 8-cell × 3-tool scoreboard (or an explicit ruling that only the
  four benchy cells are re-taken), with the disclosure attached.
- The map's Decisions-so-far entries updated so no pre-swap benchy ratio reads
  as current.
