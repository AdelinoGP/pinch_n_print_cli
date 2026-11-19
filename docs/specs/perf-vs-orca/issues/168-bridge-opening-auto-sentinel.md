# Bridge opening radius resolves to zero through the auto sentinel

Type: task / measured candidate (canonical-parity fix + standing gate)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the [t166 take](../evidence/t166-shell-opening-parity/FINDINGS.md),
which found the auto sentinel while fixing the *shell* opening and confirmed the
bridge classifier reads the same value.

## Question

**`classify_unsupported_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`)
opens its unsupported-bottom diff at `0.1 × external_perimeter_width_mm`, but on
the matched job that width resolves to `0`, so the opening removes nothing.
What is the paired wall/CPU and output effect of resolving the auto sentinel the
way `Flow` does, and is the result canonical-correct?**

The matched job authors `outer_wall_line_width = 0`, which is canonical's
**auto** sentinel (Orca's own resolved appendix prints `outer_wall_line_width =
0.42`, i.e. a resolved width). `resolve_external_perimeter_width_mm` returns the
literal `0.0` for it, so the radius is `0.1 × 0 = 0` and the canonical statement

```text
bottom = opening_ex(diff_ex(slices, lower_slices, ApplySafetyOffset::Yes),
                    external_perimeter_width / 10)
```

is only half implemented: the diff runs, the opening cannot. Every unsupported
sliver therefore survives as a bridge.

## What is already known (t166)

- The shell stage hit the same sentinel. Measured there with the literal `0`:
  **5,201,451 points in, 5,201,451 out, 6.8 ms for 974 calls** — a no-op. Resolved
  through `line_width` (`0.4 mm`) the opening becomes real and the same cell gets
  **faster** (20.9 s → 19.2 s, one run each, same binary).
- `resolve_shell_external_width_mm`
  (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) is the resolution
  t166 landed for the shell stage. This ticket should reuse or lift it, not add a
  third copy — the t166 ticket deliberately left the bridge path untouched so
  its delta audit stayed single-cause.
- `bridge_areas` is **read** in production, so this is not dead work:
  `crates/slicer-runtime/src/region_partition.rs` derives `sparse_infill_area`
  from it, `crates/slicer-core/src/perimeter_spatial.rs` answers per-vertex
  `is_bridge` from it, and `detect_bridging_direction_deg`
  (`crates/slicer-core/src/algos/prepass_slice.rs`) derives the bridge direction
  from it. Tightening it moves the sparse/bridge partition and the bridge
  anchors, so output **will** move.

## Scope

1. **Resolve the sentinel once.** Use the `outer_wall_line_width` → `line_width`
   → 0.4 mm chain the shell stage now uses (positive absolute wins; a percentage
   falls back). No second resolution policy.
2. **Keep the safety offset as it is.** `classify_unsupported_bridge_areas`
   already inflates the clip by one workspace unit
   (`SCALED_EPSILON_MM`), which is canonical's `ApplySafetyOffset` at this
   resolution. t166 measured the same choice on the shell side and it was
   *slower* and moved output; do not re-litigate it here without evidence.
3. **Delta audit.** Section-level and per-layer: `Bridge`, `Internal Bridge`,
   `Sparse infill`, `Gap infill`, plus the bridge-anchor direction on base
   classic-off. The expected direction is **fewer** bridge lines and **more**
   sparse infill, never the reverse; if a cell moves the other way, report it
   rather than widening the claim.
4. **Paired campaign.** The standing 8-cell gate: 5 interleaved pairs per cell,
   ordinary arm, accelerated arm as corroboration ([t163](163-accelerated-timing-board.md)).
   Output is expected to move in every cell.
5. **Unit tests.** A falsifier that the radius resolves non-zero for the auto
   sentinel, plus the existing bridge-classification cases updated to
   canonical-correct output. Never weaken an assertion.

## Pre-declared gate

- 0.80 % floor and outside the measured null band on at least the four `base`
  cells; no median regression on the four `benchy` cells.
- The parity fix lands on correctness grounds even if the cell effect is
  sub-floor; the human rules whether it is banked as speed.
- The new reference set is recorded per cell, superseding t166's.

## Acceptance

- Production change plus updated tests. `clippy --workspace --all-targets -D
  warnings`, `check-literals` and `check-test-quality --report` all green.
- `evidence/<ticket>-bridge-opening/` with the campaign rows, the delta audit, a
  `reference.json` and a `verify.py`.
- Map: destination-status block, Decisions-so-far line, active reference set.

## Answer

_(pending)_
