# Port canonical's top/bottom opening into ShellClassification Pass 1

Type: task / measured candidate (canonical-parity fix + standing gate)
Label: wayfinder:task
Status: resolved (2026-11-19; measured candidate - parity fix lands, speed claim DEGRADED 5/8 with CPU lower 8/8; graduate t168)
Assignee: 2026-11-19 OpenCode wayfinder session — the human said "Continue the
perf vs orca map" (no ticket named); 166 is first on the frontier.
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

**The canonical opening lands; the speed claim is DEGRADED (5/8 by the strict
wall gate, all eight cells CPU-lower), and the human rules whether it is banked
as speed.** Evidence:
[`evidence/t166-shell-opening-parity/`](../evidence/t166-shell-opening-parity/FINDINGS.md).

Pass 1's opening is now canonical `detect_surfaces_type`'s:
`opening_ex(diff, external_perimeter_width / 10)` with `jtMiter` and miter limit
3, replacing `line_width / 2` (0.2 mm) with `Round` joins. Pass 2's
shrinking-shadow width is unchanged — `ShellClassificationParams` carries the two
widths separately so the parity fix cannot move the cascade.

**Two defects surfaced and are disclosed.** (1) The region config carries
`outer_wall_line_width = 0`, canonical's **auto** sentinel; resolving the literal
zero disables the opening entirely (measured no-op: 5,201,451 points in and out,
6.8 ms for 974 calls). `resolve_shell_external_width_mm` resolves it the way
`Flow` does: `0.4 mm → radius 0.04 mm`. (2) The same sentinel reaches
`classify_unsupported_bridge_areas`, so canonical's *bridge* opening
(`0.1 × fw`) has been a no-op on the matched job — a separate, output-affecting
divergence with its own readers (`region_partition.rs`, `perimeter_spatial.rs`,
`detect_bridging_direction_deg`), graduated as
[168](168-bridge-opening-auto-sentinel.md) and deliberately not bundled here.

**Rejected on measurement:** also porting canonical's `ApplySafetyOffset`
(the clip inflate `ClipperUtils::diff_ex` applies). At this workspace's
quantisation it lands on one unit (`SCALED_EPSILON_MM`), and on base classic-off
it made the cell **slower** (20.2 s vs 19.2 s) and moved output. The radius-only
arm reproduces the original t166 probe's hash `0a1ee3df…` exactly.

**Campaign:** 80 paired runs, one window, ordinary arm, 5 interleaved pairs per
cell, every exit 0. Wall saved: benchy **+20.435 / +7.915 / +21.395 / +9.277 %**
(25.5x / 9.9x / 26.7x / 11.6x the floor, 5/5 pairs faster each), base
**+3.448 / +2.468 / +0.310 / +0.734 %** (4.31x / 3.09x / 0.39x / 0.92x).
**Process CPU is lower in 8/8** (+2.340…+10.938 %). Null control (3 pairs, 3
cells): wall bands 6.09–11.54 % any-pair, 0.50–5.35 % median; CPU bands
0.74–1.74 % any-pair. So the benchy cluster is decisive, `base classic-on` passes,
`base classic-off` clears the floor but sits inside this window's 3.17 % null
median band (DEGRADED), and the two base Arachne cells are sub-floor on wall
while CPU-lower — **unresolved by this instrument**, not proven neutral.

**Parity:** top/bottom solid path grows in all eight cells (+5.8…+783.6 mm), but
the ticket's "never less" expectation is **falsified per layer** (12–29 top
layers and 1–15 bottom layers lose path in every cell) and disclosed: Pass 1
classifies each layer against its neighbours, so the change is a relabelling
across layers, not monotone growth (depth-0 seeds 361/311 → 481/451 on base
classic-off).

**Output:** all eight baselines reproduce t151's active set exactly; all eight
candidates move and are the new active reference set
([`reference.json`](../evidence/t166-shell-opening-parity/reference.json)):
benchy `276782a7…` / `44564d29…` / `46111bb0…` / `12c5e45c…`, base
`0a1ee3df…` / `bc5cb5e1…` / `cadebffa…` / `77f2b655…`.

**Route (estimate, cross-window):** composing the paired medians with t164's
board moves the required cut on the two benchy supports-off cells 49.45 % →
36.47 % and 47.21 % → 32.84 %, and on the supports-on pair 65.33 % → 62.35 % and
64.08 % → 60.40 %; the `base` cells move 0.3–3.4 %, which is too small to change their place on the route. **0 of
8 cells are won** and the binding constraint is untouched.

**Gap:** no accelerated arm — the corroboration needs two full
`dist --accelerated` builds plus its own campaign and was not taken. t163 bounds
the arm's own effect at −13.2…+4.4 % wall, which cannot overturn the benchy
numbers but could matter for the two sub-floor base Arachne cells.

