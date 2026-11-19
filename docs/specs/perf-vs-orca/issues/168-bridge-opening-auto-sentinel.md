# Bridge opening radius resolves to zero through the auto sentinel

Type: task / measured candidate (canonical-parity fix + standing gate)
Label: wayfinder:task
Status: resolved
Resolved: 2026-11-19 — the auto sentinel is resolved through one shared policy (`outer_wall_line_width` → `line_width` → 0.4 mm), the bridge opening is live, and the parity fix lands on correctness grounds: bridge path falls 8/8, process CPU is lower 8/8 on both build arms, but no cell's wall effect clears the measured null band, so the standing gate fails on wall (0/4 `base`) and no speed is banked.
Assignee: 2026-11-19 deepseek wayfinder session — the human said "Continue the perf vs orca map" (no ticket named); 168 is first on the frontier.
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

**The sentinel was the defect; the fix is canonical and lands, but it buys no
measurable wall.** `resolve_external_perimeter_width_mm`
(`crates/slicer-core/src/algos/prepass_slice.rs`) now requires a **positive**
absolute `outer_wall_line_width` (both `Float` and absolute `FloatOrPercent`)
and falls through to `line_width`, then 0.4 mm; the shell stage's copy is now a
one-line delegate to it, so the two openings share one policy and cannot drift
again. On the matched job the radius moves `0.1 x 0` → `0.1 x 0.4 = 0.04 mm`, so
the bridge opening is live for the first time.

**Output:** the baseline reproduces t166's active set in **8/8**; the candidate
moves **8/8** (`d6cb4acd…` / `21c6969f…` / `5c0f846e…` / `6fe4b09b…` /
`9e360450…` / `f77cf265…` / `cd896bab…` / `3044c4d6…`, bytes −2,373 / −2,327 /
−2,112 / −2,218 / +24,435 / +24,313 / −5,288 / −5,328), and the accelerated arm
is byte-identical to the ordinary arm in **16/16**. All 286 runs are clean
(`degraded: false`, zero code-1200).

**Delta audit:** bridge path falls in **8/8** (−10.5 / −10.5 / −5.0 / −5.0 /
−11.4 / −11.4 / −8.1 / −8.1 mm), internal bridge rises in 8/8 (+3.6 / +2.0 mm),
`Gap infill` is inert, and sparse infill **rises only on `base classic`**
(+490.2 mm, +0.28 %) while falling by 0.01–0.02 % on the other six cells — a
partial reversal of the expected direction, reported. The per-layer census
(19–26 layers gain bridge path, 27–33 lose it on base) says the opening
relabels the bottom set rather than shrinking it monotonically. The bridge
anchor direction proxy (dominant bridge segment angle, not the IR field) moves
by ≥ 45° on 12/48 / 13/61 / 2/7 bridge layers — a real re-derivation, as
canonical's own post-opening area implies.

**Paired wall (10 interleaved pairs/cell, ordinary):** benchy **+1.99 / +1.29 /
+1.17 / +1.71 %**, base **+4.09 / −0.14 / +4.15 / +0.73 %**; null bands are
4.89–9.86 % any-pair on the wall. **Gate FAILED on wall, 0/4 `base` cells**:
both supports-off base medians sit inside their null band and both supports-on
medians are at zero. The benchy clause passes (no median regression).
**Process CPU is lower in 8/8** cells (10/10 pairs in five of them) and on the
accelerated arm (8/8, 3 pairs/cell), which is the one consistent channel and is
reported as such, not as a wall win.

**Ruling left open:** the ticket pre-authorized landing a parity fix whose cell
effect is sub-floor, and this take changes no ratio of record, so the change is
landed and the map's route arithmetic is unchanged. Whether to bank it as speed
is the human's call and is not needed for anything downstream. Evidence:
[`evidence/t168-bridge-opening/`](../evidence/t168-bridge-opening/FINDINGS.md)
(`verify.py` PASS, `--self-test` 8/8 negative controls; `audit_t168.py
--self-test` 5/5; `reduce_t168.py` re-derives every paired number from the raw
rows).
