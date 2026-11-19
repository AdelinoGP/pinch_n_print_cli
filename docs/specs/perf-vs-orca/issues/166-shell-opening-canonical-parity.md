# Port canonical's top/bottom opening into ShellClassification Pass 1

Type: task / measured candidate (canonical-parity fix + standing gate)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the fog item "Pass 1 `apply_opening` round-join
arc-tolerance-0 anomaly (t27 candidate (a))", priced on 2026-11-18 by the
[t166 probe](../evidence/t166-shell-opening-parity/FINDINGS.md):
`apply_opening` is **53–59 s** of thread-time on base classic-off (about half of
the stage), canonical parameters cut it to **12.6–14.3 s**, and the Pass 1+2 wall
drops **2.4–3.0 s** (instrumented, n = 2).

## Question

**If `apply_opening` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`)
is replaced with canonical `PrintObject::detect_surfaces_type`'s opening
(`opening_ex(diff, external_perimeter_width / 10)` with `jtMiter`, miter limit
3), what is the paired wall/CPU effect on all eight cells? And is the new output
canonical-correct, so that it becomes the next active reference set?**

This is a parity fix first and a speed shape second. PnP's radius
(`line_width / 2`) is about 5x canonical's and its join differs, so PnP
classifies narrower top/bottom features as non-solid where canonical keeps them.
It falls under the t111/t128 prepass exception: the supports-off cluster may be
replaced by canonical's work set at canonical's cost.

## Scope

1. **Exact canonical radius.** Read the matched job's external-perimeter width
   from the Orca resolved-config appendix and from PnP's `RegionMapIR`. The probe
   used `line_width / 10` as an approximation. Resolve it the way
   `layerm->flow(frExternalPerimeter)` does, or record the gap as a deviation.
2. **Keep Pass 2 unchanged.** `shell_width_mm = opening_r × 2` feeds the
   shrinking-shadow cascade and is a separate parity question. Split the radius
   so that only the opening changes.
3. **Parity evidence.** Write a per-layer top/bottom area delta against the
   current output on all four base cells and two benchy cells. Spot-check against
   canonical surface classification where a visual-debug bundle can show it. The
   expected direction is *more* top/bottom solid area, never less.
4. **Paired campaign.** Use the standard 8-cell gate: 5 interleaved pairs per
   cell, ordinary arm, accelerated arm as corroboration (t163). Output is
   expected to move in every cell with top/bottom geometry, so the gate is
   **wall/CPU against a fresh reference set**, not byte identity. Disclose the
   downstream cost the extra solid area adds (internal-bridge gate, per-layer
   fill).
5. **Unit tests.** Update fixtures to canonical-correct output (CLAUDE.md Test
   Discipline). Never weaken an assertion.

## Pre-declared gate

- 0.80 % floor and outside the null band on at least the four base cells.
  Benchy cells must show no median regression.
- If the cell-level effect is sub-floor because downstream work absorbs the
  saving, the parity fix still lands on correctness grounds. The human rules
  whether it is banked as speed.
- The new reference set is recorded with the same per-cell hash discipline as
  t151.

## Acceptance

- Production change plus updated tests. `clippy --workspace --all-targets -D
  warnings`, `check-literals` and `check-test-quality --report` all green.
- `evidence/t166-shell-opening-parity/` extended with campaign rows, the parity
  delta and a `verify.py`.
- Map: destination-status block, Decisions-so-far line, and the active reference
  set updated.

## Answer

_(pending)_
