# Arachne Parity Recovery — Campaign Tracker

**Status:** Active (opened 2026-07-15).
**Supersedes status claims in:** `perimeter-modules-orca-parity-roadmap.md` and
`arachne-parity-N1-N13-plan.md` where those drifted from code reality (see the
DEVIATION_LOG reconciliation below). Those roadmaps describe the *intended build*;
this doc tracks the *recovery* of real OrcaSlicer Arachne parity.

## Why this campaign exists

An audit of the Arachne/Classic perimeter pipelines found the Arachne module
"severely lacking." Verified against source, the problem is as much a **trust
failure in the certifying instruments** as a feature gap:

- **The test suite cannot detect the brokenness.** Every Arachne parity fixture is
  a *self-captured baseline* (D-109, D-112 — "no OrcaSlicer oracle in-repo"). A
  self-captured snapshot regression-locks PnP's *own current output*, so green means
  "unchanged from the broken snapshot," never "correct." A visibly-broken pipeline
  therefore sits behind a green board. Even at 100% green, the pipeline needs
  significant work.
- **The status ledger drifted** (corrected 2026-07-15 — see below).
- **The board is not even green:** a full-suite `--no-fail-fast` run (309 binaries,
  2,761 passed) found **12 pre-existing failing tests** on `parity/arachne`.

## The two-instrument model

Correctness is judged by two independent, deliberately-separated instruments:

| Instrument | Purpose | Basis | Committed? |
|---|---|---|---|
| **LLM-visual steering** | *Find* defects; localize where geometry first breaks | Claude renders PnP output **and** canonical OrcaSlicer gcode to PNGs via `pnp_cli visual-debug` (final gcode-mode *and* any `Layer::*`/prepass model-mode tap) and compares them with multimodal vision. Comparison is **semantic, not pixel-exact** | No — `tmp/benchy.stl`, `tmp/orcaSlicer_arachne_benchy.gcode` uncommitted; tests never read them |
| **Structural-invariant regression** | *Prevent* regressions; gate CI | Unit-independent assertions (closure within tol, loop count/nesting, bead-count sequence, transitions-present, no self-intersection) on **synthetic fixtures reproducing benchy error classes** | Yes — committed, host-algos-gated |

Structural invariants are **invariant to absolute-unit divergence** (PnP's
1-unit = 100 nm coordinate system + float/algorithm drift makes any absolute-coordinate
equality test flaky even when output is correct), so green finally means "structurally
right." The OrcaSlicer gcode stays a steering reference, never an automated numeric fixture.

## Locked decisions

1. North star: full parity — `cube_4color_arachne_outer_walls_close_end_to_end` to 0
   failures — but tests-green is necessary, not sufficient; visual parity vs OrcaSlicer benchy is the real bar.
   **✅ Met 2026-07-16 (0/699, un-ignored) — see "Track C closure" below. The second clause stands:
   the gate being green does not end the campaign.**
2. Automated basis: structural invariants (unit-independent). Self-captured snapshots and absolute-numeric OrcaSlicer fixtures both rejected.
3. OrcaSlicer oracle: `tmp/orcaSlicer_arachne_benchy.gcode`, uncommitted, LLM-visual comparison only.
4. visual-debug is LLM-driven via multimodal image reading, free to walk any supported stage tap.
5. First deliverable: benchy defect inventory **and** closure-chain fix, in parallel.
6. CI: host-algos parity suite joins the default gate.
7. Docs corrected to code truth (not code bent to a stale plan).
8. Synthetic fixtures manufactured to reproduce benchy error classes (simple fixtures don't trigger them).

## Ledger reconciliation done 2026-07-15 (Workstream 0)

- **ADR-0035** — replaced 7 fabricated filenames with real loci
  (`skeletal_trapezoidation/propagation.rs`, `arachne/generate_toolpaths.rs`,
  `skeletal_trapezoidation/centrality.rs`, `arachne/remove_small.rs`,
  `arachne/simplify.rs`, `arachne/pipeline.rs`); removed the fictitious
  `MAX_FAILURES=500` guard (the only mechanism is `#[ignore]`).
- **D-105-FLOW-NOT-WIRED** — reopened (classic half): `classic-perimeters` still uses
  width-average `(outer+inner)/2` at `lib.rs:600,605,907` with zero `line_width_to_spacing`
  calls; only `arachne-perimeters` was wired (P150). T-052 genuinely open.
- **D-147-CHAIN-CLOSURE** — reopened: cannot stand `Closed` while its AC-1 gate is
  `#[ignore]`d and ~50% failing, per "F blocks on green."

## RED baseline — triage of the 12 (2026-07-15)

**Structural root cause of most FIX items:** packet 155's closure-verification command
(`cargo test -p slicer-runtime --test arachne_parity && cargo test -p slicer-core`) ran
neither `--features host-algos` nor the `arachne-perimeters` crate, silently skipping the
tests P155's beading-threshold change (D-155) would break. Packet 156's `wip(arachne)`
region-order commit `57191889` is the likely cause of the runtime arachne/paint items.
This is exactly why host-algos must join the default gate (Workstream 0).

| Test | File | Verdict |
|---|---|---|
| `bridge_vertices_get_bridge_flow_ratio_when_thin` | `modules/core-modules/arachne-perimeters/tests/bridge_flow_factor_tdd.rs:53` | ~~FIX~~ **RESOLVED 2026-07-17 (D-166 closed)** — fixture defect: origin-centered 4mm bridge square never overlaps walls under the bead-count clamp (only did under the retired 9-inset/wide-centre-bead regime). Bridge area re-centered on the region corner; thick test's `flow_to_width` re-conversion of `pt.width` (a pre-Bug-B spacing-domain assumption) also removed; vacuous companion `is_bridge` test hardened with an inside-vertex guard. |
| `bridge_vertices_get_round_section_factor_when_thick_bridges_on` | same:123 | FIX |
| `ac1_local_maximum_emits_hexagonal_micro_loop` | `crates/slicer-core/tests/arachne_local_maxima_single_beads.rs:44` | FIX (also: stale RED-style header from before P145 closed N9 — clean up) |
| `bead_count_tapered_wedge` | `crates/slicer-core/tests/bead_count.rs:205` | FIX (fixture predates P155 clamp; re-verify, don't blind-rebless) |
| `generate_toolpaths_tapered_wedge` | `crates/slicer-core/tests/generate_toolpaths.rs:278` | FIX (shares wedge fixture with bead_count) |
| `wedge_multi_layer_top_bottom_evidence` (packet-109 bottom-surface) | `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs:1596` | ~~FIX (root cause not pinned statically)~~ **RESOLVED 2026-07-17 — the test asserted a defect (8th).** Bisected to `a076038c` (f64 layer-Z fix, correct & human-verified, kept). Under the old f32-tainted Zs, the slice plane through the wedge's slope-start vertex plane (z=2.0) produced an EMPTY layer (0 moves), making z=2.2's whole footprint "unsupported" → a spurious whole-area `Bottom surface` — the third block the `>= 3` count required. Exact-f64 Zs slice z=2.0 correctly, the spurious bottom vanishes, and the y-extension underside moves to its first CONTAINING layer (29.0→29.2, physically correct: the slab spans z=29..31). Test rewritten: `>= 2` genuine bottoms (0.2, 29.2), non-empty-layer regression guard at z=2.0, no-bottom guard at z=2.2, extension-at-29.2 assertion; slot-ceiling bridge assertions unchanged (still pass). |
| `legacy_zero_matches_golden` | `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs:225` | ~~FIX (inspect diff before rebless)~~ **RESOLVED 2026-07-17 — reblessed after full hunk attribution.** Sorted-content diff was 138 lines, all traced to two intended changes: (1) `a076038c` f64 layer-Z — 99→100 layers (top layer at z=20.00 restored; old golden was MISSING the box's final layer), config-block `layer_height 0.20000000298…→0.2`, E last-digit corrections, config-hash object UUID; (2) `57191889` region-order contract (ADR-0011) — path-optimization preserves the committed wall sequence instead of role-priority reordering (pure within-layer reorder, move content identical). No unexplained hunks. |
| `arachne_perimeters_simple_square_produces_walls` | `crates/slicer-runtime/tests/executor/arachne_perimeters_simple_square.rs:44` | ~~FIX (likely broken by `57191889`)~~ **RESOLVED 2026-07-17 — the test was the defect (7th test-asserting-the-defect).** Empirical: 3 walls, every width exactly 0.4mm — correct uniform output for a uniform 10mm square. Assertion (d) demanded NON-identical width vectors, citing the retired pre-P155 9-inset regime ("26 lines across 9 insets", 1.11mm centre beads from the odd-center over-cap branch). Rewritten to pin wall count == 3 and all widths == configured 0.4mm ±0.01. NOT a `57191889` regression. |
| `cube_4color_ironing_per_painted_top_color` | `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs:170` | FIX (previously-green gate; paint/region-order churn) |
| `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes` | `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs:945` | FIX — **fn confirmed 2026-07-16** (was "exact fn TBD"). Fails AC-4(a) at layer 78: 3 outer-wall header fragments vs 4 distinct tool indices (`headers={0,1,2} tools={0,1,2,3}`). **Classic**-perimeters path (arachne is dropped by claim-dedup in this test), so NOT Track C and unaffected by the arachne fixes |
| `n3_apply_transitions_creates_lower_and_upper_end_splits` | `crates/slicer-core/tests/arachne_parity_red_transition_ends.rs:116` | **QUARANTINE** (deliberate RED anchor, N3) |
| `arachne_parity_pipeline_concentric_infill_uses_arachne` | `crates/slicer-runtime/tests/arachne_parity.rs:915` | **QUARANTINE/IGNORE** — user decision 2026-07-15: concentric-infill is not on the roadmap and may never be implemented with Arachne; out of campaign scope |

**Unconfirmed (static reasoning only — narrow runs pending):** exact failing assertion
for bead_count/generate_toolpaths; exact failing fn in cube_4color_gcode; root cause for
packet-109 bottom-surface and legacy golden.

## Track A — first benchy findings (2026-07-15)

Setup: benchy sliced with Arachne → `tmp/benchy.gcode` (240 layers, 48mm; Outer wall
653 / Inner wall 644 / Sparse 548 / Bridge 404 …). Rendered PnP gcode via `pnp_cli
visual-debug` (gcode-mode) → `tmp/vdbg_pnp/` (8 PNGs, zero warnings).

**Oracle feasibility blocker (RESOLVED):** OrcaSlicer's benchy gcode used arc moves
`G2`/`G3` (595+483) the visual-debug gcode parser cannot render, so arc geometry was
silently absent from Orca renders (a naïve PnP-vs-Orca diff would misread unrendered arcs
as "missing walls"). Resolved 2026-07-15: OrcaSlicer benchy gcode regenerated with
arc-fitting OFF (pure `G1`) at `tmp/orcaSlicer_arachne_benchy.gcode`. Re-render
`tmp/vdbg_orca/` from it, then do the side-by-side.

**Closure is NOT the benchy defect (visual false-positive corrected).** Initial multimodal
read of PnP final-gcode filament-lines flagged "severe wall fragmentation" at layer 96.
A structural stage-walk (model-mode `Layer::Slice` + `Layer::Perimeters` typed IR)
**disproved it**: benchy's mesh cross-section at layer 96 (z=19.4mm) genuinely consists of
**5 disjoint closed polygons already in the raw pre-Arachne `SliceIR`**; Arachne correctly
emits 5×3=15 wall loops, all `closure_gap=0.0000`, preserved unchanged to final gcode.
Layers 3 / 96 / 230 all have fully closed walls at every stage — 0 open fragments anywhere.
What looked like "disconnected fragments" in a line render was 5 legitimate separate closed
islands.

**Methodology lesson (folds into Track A):** a filament-lines render cannot distinguish an
open/broken wall from several separate closed islands. The visual inventory MUST cross-check
any suspected closure defect against `Layer::Perimeters` `closure_gap` before recording it.
Consequence: single-color benchy Arachne has **no gross open-wall defects**; real defects
will surface as **PnP-vs-OrcaSlicer differences** (wall counts, thin-feature beads, bead
widths, seams), which the arc-free side-by-side now targets. (The cube_4color closure gate
is a separate MMU population, per the user.)

**Tooling nit found:** `docs/specs/visual-pipeline-debug.md`'s worked JSON example uses stale
field names (`model_path`, `module_dir`, `views`); the real `VisualDebugRequest` uses
`source.model` / `source.config` / `source.module_dirs` / `visualizations`. Doc-cleanup backlog.

**Tooling gap found 2026-07-16 — the Open Deviation Map silently under-reports (found while
filing `D-160`).** `docs/07_implementation_status.md`'s generated "Open Deviation Map" presents
itself as "a generated snapshot of the open set", and `DEVIATION_LOG.md`'s header says the views
are "generated from this table". Both overstate it: `xtask`'s parser filters with
`if !line.starts_with("| DEV-")`, so it sees **only** `DEV-###` rows and silently drops every
`D-###-SLUG` row. At least four open deviations are therefore invisible in the map — `D-105`
(reopened), `D-109`, `D-110`, and the new **`D-160` (High)** — while the map reports a
confident "6 open". Consequence: an agent or human trusting the map for "what's open" gets a
number that is roughly half the truth, and misses the highest-severity open Arachne defect.
`cargo xtask check-deviations` returns clean throughout, because it only ever compares the map
against the subset it can see. **The deviation log itself is correct and remains authoritative
(as its own header states); it is the generated view that lies.** Fix is a parser predicate
(accept `| D-` as well as `| DEV-`) plus a regenerate; not done here — it would add ~4 entries
to a generated doc section and is unrelated to Arachne parity. Same family as this campaign's
other instrument failures: the check passes, so the gap reads as absence.

### Benchy PnP-vs-OrcaSlicer findings (structurally verified 2026-07-15)

Method: matched-Z structural comparison of the two gcodes (vertex counts, bboxes, `;TYPE:`),
which is scale/arc-independent. Object-only footprints match within 1.4% (PnP 59.6×30.6mm,
Orca 60.5×31.0mm) — same geometry scale.

- **D1 — perimeter over-simplification/faceting: REFUTED.** PnP outer-wall point density
  matches OrcaSlicer (layer 1: 70 vs 77 vertices, ~same bbox/length; Z≈19: raw counts differ
  but points-per-mm comparable). The "faceted" visual impression was a viewport/zoom artifact
  (PnP's skirt+brim inflate the toolpath bbox → viewer autofit differs). **Not a defect.**

- **D2 — bottom layers sparse-filled instead of solid: CONFIRMED.** PnP emits `Sparse infill`
  inside the bottom-shell region on layers 1–3 where OrcaSlicer emits `Internal solid infill`/
  `Top surface`/`Bottom surface`. Reproducible. Suspected locus: bottom-shell-layer count or
  solid-infill threshold (shell classification / infill, **not** wall generation).

- **D3 — mid-hull loop fragmentation: ~~OPEN~~ REFUTED 2026-07-16 (NOT a defect).** Original claim:
  at Z≈19mm PnP splits the cross-section into disjoint island walls (38/37/31/20-pt loops) where
  OrcaSlicer produces one continuous 138-pt loop of the same combined span, allegedly a `SliceIR`
  (upstream-of-Arachne) fragmentation bug.

  > **REFUTED — it was a layer-alignment artifact.** The benchy's deck rim *genuinely* opens up: the
  > raw mesh cross-section (probed directly via `slicer_core::algos::mesh_cross_section::cross_section_at_z`
  > on `tmp/benchy.stl`, i.e. plane-triangle intersection upstream of everything) is **one 1438-pt
  > contour at z ≤ 18.85** and **three contours at z ≥ 18.90** — a real, sharp topology transition at
  > z ≈ 18.875 (port rail / starboard rail / bow cap). **OrcaSlicer makes the same 3→5 outer-wall-loop
  > transition**, just one layer later in its own Z labelling:
  >
  > | | last 3-loop layer | first 5-loop layer |
  > |---|---|---|
  > | PnP | `;Z:18.80` | `;Z:19.00` |
  > | OrcaSlicer | `;Z:19.05` | `;Z:19.25` |
  >
  > At the geometrically **aligned** layers the two agree completely — PnP `;Z:18.80` emits ONE
  > continuous **147-pt** loop (bbox 21.148×30.552) matching Orca `;Z:19.05`'s **137-pt** loop
  > (21.363×30.846); the other two loops match 1:1 as well (59↔47 pts, 30↔22 pts; all bboxes within
  > ~1%). **PnP does not fragment anything.** The original finding compared PnP's `;Z:19.0` layer
  > (ABOVE the transition → 5 loops) against Orca's `;Z:19.05` layer (BELOW it → 3 loops), assuming a
  > 0.05mm offset; the true geometric offset is ~0.25mm — a full layer.
  >
  > **Method lesson (the campaign's third false positive on this exact symptom, after layer-96 and D1
  > "faceting"):** "PnP splits what Orca keeps whole" is a *comparison* claim, and a comparison is only
  > valid at an aligned sampling plane. Before attributing a topology difference to a defect, verify the
  > two references sample the SAME mesh plane — Z LABELS ARE NOT AN ALIGNMENT. Cheapest discriminator:
  > sweep the metric across a Z band in BOTH references and look for the same transition at a shifted
  > offset (a shifted-but-identical curve = alignment artifact; a curve present in one and absent in the
  > other = real defect).

  > **REAL finding surfaced instead — slice-plane Z convention (NEW, open, NOT Arachne).** The two
  > slicers assign Z labels to the same physical mesh plane with a **~0.25mm offset**: PnP's `;Z:18.80`
  > layer and Orca's `;Z:19.05` layer cut the same geometry. Since the mesh transition is at z≈18.875,
  > PnP's `;Z:19.00` layer must sample at ≥18.88 (≈ its own `print_z`, i.e. the layer **top**), while
  > Orca's `;Z:19.05` layer samples at ≤18.87 (≈ `print_z − layer_height`, i.e. ~the layer **bottom**;
  > notably NOT the PrusaSlicer-family `slice_z = print_z − height/2` mid-plane, which would be 18.95 and
  > would have shown 5 loops). Consequence: **every layer's geometry is sampled up to a layer height
  > off vs OrcaSlicer**, which systematically shifts overhang classification, dimensional accuracy, and
  > any Z-sensitive parity fixture. Not investigated further (out of this session's Arachne scope);
  > needs its own packet — confirm PnP's `slice_z` derivation against OrcaSlicer's `Layer::slice_z`
  > (`OrcaSlicerDocumented/src/libslic3r/`) before changing anything, and beware: benchy's first-layer
  > height differs between the two configs (PnP 0.2 vs Orca 0.25), which accounts for only 0.05mm of the
  > 0.25mm offset.

- **D4 — inner-wall bead degeneracy / self-overlapping paths: CONFIRMED (the real in-scope
  Arachne defect; this is the user's "degenerate perimeters").** Structural bead-width analysis:
  PnP's OUTER wall is fine (width stdev 0.007, comparable to Orca), but INNER walls are degenerate
  — 11–18% of inner-wall moves exceed 0.7mm, up to 2.9–5.3mm (7–12× the 0.42mm nominal); Orca never
  exceeds 0.83mm. Root cause (line evidence): an **out-and-back path retrace** in an inner-wall ring
  at `tmp/benchy.gcode:45784–45823` — the nozzle retraces the same coordinates in reverse,
  **extruding on both passes (double-extrusion)**, which yields the impossible wide "beads". Adjacent
  outer→inner wall spacing collapses to 0.03–0.05mm (physical overlap) at points vs Orca's ≥0.5mm
  floor. Suspected locus: Arachne junction/toolpath walk (`arachne/generate_toolpaths.rs`) or
  `arachne/stitch.rs` doubling a domain chain back on itself. **In scope — Track C target.**
  Yields three unit-independent Track-B invariants: (i) no wall loop retraces its own coordinates
  (no non-adjacent point revisited within ε, excluding the closing point); (ii) bead width ≤ ~2×
  configured line width; (iii) adjacent-wall centerline spacing ≥ ~½ line width (no overlap).

  > **Root cause localized 2026-07-16 (post-D5 re-measurement + OrcaSlicer source at `OrcaSlicerDocumented/`).**
  > The dominant symptom is **over-extrusion, not retrace**: on the fresh (post-D5) benchy, 3,313 inner-wall
  > moves compute width >3mm, worst **19.6mm (43× the 0.45mm nozzle)**, all at **Y≈0 (the hull medial spine)**
  > around z≈4.6–6.4 (0 such moves in Classic). The width is real (M83 relative E; a 16.87mm move carries
  > 27.5mm filament). Mechanism (instrumented `generate_junctions`, production/host path): these peaks resolve
  > a **stored** beading (`graph.get_beding`) with an **odd bead_count of 3 or 5** at thickness `2·to_r ≈ 19.7mm`,
  > and `DistributedBeadingStrategy::compute(19.7mm, 3)` returns `[0.4, 18.9, 0.4]` — the Gaussian weights
  > (`distribution_count=1` → `[0,1,0]`) dump the **entire surplus into a single ~18mm centre bead**, which is
  > then extruded. **`optimal_bead_count(19.7mm)=7` is correct** (verified) and `compute(19.7,7)` yields
  > `[0.4×3, 0-sentinels, 0.4×3]` (walls + infill, no giant bead), so `assign_bead_counts` is NOT the fault —
  > the bug is in **beading PROPAGATION**. **RESOLVED 2026-07-16 — see below.**
  >
  > **ROOT CAUSE + FIX (2026-07-16).** PnP had canonical's beading pass **ORDER inverted**. Canonical
  > (`SkeletalTrapezoidation.cpp:1488-1514`) computes each node's beading from ITS OWN `bead_count` +
  > thickness, THEN `propagateBeadingsUpward`/`Downward` copy the resulting **Beading objects** to nodes
  > that have none. PnP ran `populate_beading_propagation` **last**, and `propagate_beadings_upward`
  > propagated the **scalar** bead count (`to_vertex.bead_count = Some(from_bead_count)`) — so a thin
  > node's `bead_count = 3` landed on a 16x-thicker spine node and was then recomputed there as
  > `compute(19.7mm, 3)` = `[0.4, 18.9, 0.4]`. (The inverted order also left both propagation passes'
  > side-table reads dead — the table was still empty — even though `propagate_beadings_downward`'s own
  > comments assumed populate had already run.) Fixed by: (a) moving `populate_beading_propagation` before
  > the propagation passes; (b) a faithful `propagateBeadingsUpward` port (`:1561-1588`) — copy the source's
  > Beading into the destination's side-table slot, skip when the destination has its own `bead_count`
  > ("Don't override local beading") or already has a beading, and **never** write `bead_count`/
  > `transition_ratio` on a purely-propagated joint (canonical leaves it `-1`; `propagate_beadings_downward`
  > already documented this exact rule via `D-144`); (c) switching the downward pass's three "has beading"
  > gates from `bead_count` to the side table, matching canonical's `hasBeading()` predicate. Canonical's own
  > assert states the intent: `upper_beading.beading.total_thickness <= to->distance_to_boundary * 2`
  > (`:1587`) — **a propagated beading is EXPECTED to be thinner than its destination; the surplus is infill,
  > not extrudate.**
  >
  > **Verified (production benchy):** inner-wall moves >3mm **3,313 → 0**, >2mm **4,178 → 0**, max computed
  > width **19.637mm → 0.734mm** (median 0.361 / p90 0.611; Classic is median 0.400 / max 0.509).
  > **D5 coverage unaffected** — per-layer maxX still tracks Classic at ratio ≈1.001 across Z=0.4–6.4.
  > `slicer-core --features host-algos`: **441 passed / 4 failed** (the same 4 known pre-existing) — no regressions.
  >
  > **Regression pin (Track-B-style structural invariant):**
  > `propagation_upward_copies_beading_without_rescaling_to_thicker_node` (`tests/propagation.rs`) on a new
  > thickness-gradient fixture — propagated beading must be the source's verbatim; no bead > 2x `optimal_width`;
  > `total_thickness` ≤ the destination's own; `bead_count` stays `None`. **Method lesson:** the pre-existing
  > `propagation_fills_gap_from_central_neighbor` actually **asserted the defect** (`bead_count == Some(4)` on a
  > propagated joint) and was corrected to the canonical contract — it could never have caught this because its
  > fixture gives every vertex `distance_to_boundary = 5.0`, and **under uniform thickness copy-vs-recompute are
  > indistinguishable**. The bug only exists where thickness *differs* across the propagation edge. A fixture
  > that cannot vary the quantity under test is not a test of it.
  >
  > **Landed 2026-07-16 (separate, faithful sub-fix — does NOT resolve the above):** `max_bead_count` must be
  > EVEN (OrcaSlicer `WallToolPaths.cpp:525` `= 2·inset_count`; its `LimitedBeadingStrategy` ctor warns on odd,
  > and the odd-`max_bead_count` `compute` branch — a faithful port of `LimitedBeadingStrategy.cpp:73` — parks
  > surplus in one wide centre bead). PnP used **odd 9** in the module manifest default (which ALSO shadowed
  > `wall_count` entirely, making it non-functional), core `ArachneParams::default()`, and test `factory_params()`.
  > Fixed the module: manifest `max_bead_count` `default 9→0` (sentinel), `min 1→0`; `arachne_params_from_config`
  > now derives `2·wall_count` when absent/≤0 (even, and tracks `wall_count`). Unit-proven on the captured Z≈0.4
  > cross-section: even caps (6/8/10) → max width ≤0.65mm; odd 9 → 14.75mm. But production benchy is byte-identical
  > before/after this change because its giant beads come from stored bead_count 3/5 (< cap), so the cap never
  > engages — confirming the two are distinct bugs. Core `ArachneParams::default()`/test `factory_params()` still
  > carry odd 9 (faithfulness follow-up; tangled with the tapered-wedge self-captured baselines).

**Scope map of benchy defects (decided 2026-07-15 — "skip only D2"):**
- **D4** inner-wall self-overlap/double-extrusion — **RESOLVED 2026-07-16** (beading-propagation pass
  order + faithful `propagateBeadingsUpward` port; benchy inner-wall max width 19.6mm → 0.73mm).
- **D3** mid-hull loop fragmentation — **REFUTED 2026-07-16, not a defect** (layer-alignment artifact;
  both slicers show the same deck-rim 3→5 transition, PnP emits one continuous 147-pt loop at the
  aligned layer). Surfaced a real, separate **slice-plane Z-convention** deviation (~0.25mm) — out of
  Arachne scope, needs its own packet.
- **cube_4color** closure gate — IN SCOPE (separate MMU population).
- **D2** bottom sparse-not-solid infill — PARKED (shell/solid-fill classification, out of scope for this
  campaign; logged in the parked backlog).

### D5 (DOMINANT) — Arachne grossly drops/distorts wall geometry (Classic comparison, 2026-07-15)

> **RESOLVED 2026-07-16** (commit `5d0e1bcf`, cherry-picked from the D5 diagnosis run
> `d0e78daa`). Root cause: a taper's medial-axis spine is the segment-segment bisector of
> the two converging sides, whose dR/dD legitimately exceeds the centrality cap
> `sin(wall_transition_angle/2)` ≈ 0.087, so the spine is (correctly, matching canonical)
> NON-central and receives no primary bead count. `generate_junctions` then `continue`d on
> any peak whose `bead_count` was `None`/`0`, dropping the whole region. Fix ported canonical
> `generateJunctions` (`SkeletalTrapezoidation.cpp:1740-1744`): skip ONLY when both endpoints
> share an equal non-negative bead count (or the edge is not upward), and synthesize a beading
> from `distance_to_boundary` (`getOrCreateBeading`, `:1808-1839`) for unassigned peaks —
> `crates/slicer-core/src/arachne/generate_toolpaths.rs:210` `generate_junctions`. Regression
> test `arachne_d5_taper_coverage` (captured benchy Z≈0.4 cross-section: X-coverage 66.8% →
> 99.0%). **Verified on the full benchy slice (2026-07-16):** per-layer extruding-move maxX now
> tracks Classic within ~0.1% across the entire Z=0.4–6.4 bow (e.g. Z=1.2: was arachne 2.37mm
> vs classic 15.1mm → now 15.11mm vs 15.10mm, ratio ≈1.00 throughout). Refuted en route: thin-wall
> widening and point-point discretization (D-154) — both zero effect; the bow is thick, its spine
> edges are segment-segment not point-point. Follow-up: **3** tapered-wedge SELF-CAPTURED baselines now
> drift "emit more" (correct direction) → re-baseline / convert to structural invariants (Track B).
> `1d7eb1de` re-captured the 2 slicer-core ones; the third —
> `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge`
> (`perimeter_parity::arachne_perimeter_parity`) — was missed and is still RED. See the Track C
> closure section for the proof that it is D5 drift and not a Track C regression.

Experiment: sliced benchy with `wall_generator=classic` (`tmp/benchy_classic.gcode`,
`tmp/vdbg_classic/`) as a second reference alongside OrcaSlicer.

- **Classic renders correct, complete benchy wall geometry** (e.g. `tmp/vdbg_classic/images/
  final_gcode_filament_lines_l3.png`: clean full smooth hull, proper contour, nested walls). Proves PnP
  CAN produce correct wall geometry.
- **Arachne drops entire regions of wall geometry.** Per-layer XY-extent analysis: from Z≈0.4 through
  ≈6.4 (~40 layers) Arachne's whole toolpath caps at maxX≈2–3mm where Classic AND OrcaSlicer both grow
  smoothly to ~15mm (e.g. Z=1.2: Classic maxX=15.1mm vs Arachne 2.37mm). **Arachne emits only a thin
  sliver and drops ~16mm of the bow cross-section entirely** (no walls/infill/surface of any kind for
  ~8mm of print height), and distorts/facets the contour where it does generate.
- **Arachne-specific** (Classic renders the same region correctly) → the defect is in Arachne wall
  generation, not shared pipeline. Suspected root: skeletal-trapezoidation collapsing on the thin/tapering
  bow — same family as the D-105D thin-strip medial-axis collapse.

**Correction to earlier notes:** the truncated/"faceted" Arachne renders I earlier attributed to a
"viewport artifact" were REAL geometry loss (D5). The D1 "faceting refuted" result held only for layer
index 0 (Z=0.2), where Arachne happens to be intact; at index ≥1 Arachne is dropping geometry. **D5 is
the primary Track-C/Arachne target; D4 (bead widths/self-overlap) is secondary.**

**Method lesson (reinforced):** aggregate metrics (vertex count, bbox, density, bead-width histograms) at
a single "representative" layer completely masked a catastrophe where whole regions of wall geometry are
absent. Geometry fidelity must be checked as PATH SHAPE / per-layer coverage vs a known-correct reference
(Classic AND OrcaSlicer), across the layer stack — not sampled metrics at one layer.

**Methodology finding (important):** the LLM-visual instrument produced THREE false positives
this session (layer-96 "fragmentation" → legit-closed by stage-walk; D1 "faceting" → refuted by
vertex density). Rule going forward: **LLM-visual FLAGS "these differ"; the mechanism/root cause
must ALWAYS be settled structurally (gcode/IR), never concluded from the image.** This is the
two-instrument model working as designed — the visual steers, the structure adjudicates.

## Workstreams

- **0 — enablers:** ledger reconciliation (done); triage (done, above); host-algos
  gating (implemented — see below).

### Host-algos gating (implemented 2026-07-15, decision: gate-then-fix)

Mechanism (`xtask/src/test.rs`, `test_command`): every `cargo xtask test` run now
injects `--features slicer-core/host-algos` (unless the caller set `--features`), split
correctly around any user `--`. Rationale: `host-algos` is not a Cargo default, so a
narrow `cargo test -p slicer-core` alone compiles the ~34 gated arachne test files to
empty no-ops (how P155 escaped). We do **not** flip slicer-core's Cargo default — that
would pull `rayon`/`boostvoronoi` into the five module crates' wasm32 guest builds, which
don't compile.

Quarantine: deliberate RED anchors / out-of-scope tests are skipped via a libtest
`--skip` allowlist owned by `xtask`, **not** `#[ignore]` — two sibling RED files
(`arachne_parity_gaps.rs`, `arachne_parity_round2.rs`) carry a checked-in policy
forbidding `#[ignore]` on this family. Current roster:
`n3_apply_transitions_creates_lower_and_upper_end_splits` (N3 RED anchor),
`arachne_parity_pipeline_concentric_infill_uses_arachne` (D-104f, out of scope).

Sequencing: **gate-then-fix** (user decision 2026-07-15) — land the gate + quarantine
first so the default board goes red and surfaces all 10 FIX regressions honestly, then
drive to green with the gate active throughout (no new regression can re-hide via the
narrow-command blind spot). Expect the gate RED until the 10 FIX items are resolved.

Cleanup noted for Track B: `arachne_parity.rs`'s "every test fails on purpose" header is
stale (14/15 are now green regression locks; only the D-104f test is open).
- **A — benchy visual defect inventory (LLM-visual):** slice benchy w/ arachne, render
  PnP vs OrcaSlicer PNGs, walk stage taps to localize breaks, catalog defects → backlog + fixture specs.
- **B — fixture & methodology redesign:** minimal synthetic fixtures reproducing each
  benchy error class; extend `arachne_invariants.rs`; demote self-captured baselines;
  rehome `arachne_parity_red_*.rs` into stage-grouped homes.
- **C — closure-chain fix: ✅ DONE 2026-07-16 — the north star is met.**
  `cube_4color_arachne_outer_walls_close_end_to_end` is at **0/699 (0.00%), mean gap
  0.0000mm, across all 125 layers** and is **un-ignored**; so is its sibling
  `cube_4color_arachne_per_color_footprint_within_bbox` (the D-113C "264 sub-loops,
  seam-at-origin" residual). All 3 cube_4color arachne tests green by default, 0 ignored.
  See the "Track C closure" section below for the full account — including the
  faithfulness bug the ADR-0035 re-audit caught on the way in.

## Track C closure — the north star is met (2026-07-16)

`cube_4color_arachne_outer_walls_close_end_to_end`: **0/699 outer-wall sub-loops fail to
close (0.00%), mean gap 0.0000mm, across all 125 layers.** Un-ignored, along with the
sibling `cube_4color_arachne_per_color_footprint_within_bbox`. `D-147-CHAIN-CLOSURE`
closed. Progression: 283/283 (100%) pre-113c → 455/898 (50.67%) at packet 147 → **0/699**.

**The recorded 49.33% was stale, and re-measuring first was the whole ballgame.** It
predated D5 (`5d0e1bcf`), D4 (`1dfac847`) and the `max_bead_count` correction
(`f71b82b5`). The gate was *already passing* on current code before this session's Track C
work began. **No production code was changed to make closure pass**; the residual packet
147 attributed to "a real wall/infill bug, out of scope" was upstream in the beading
pipeline all along, and D5+D4 dissolved it as a side effect. Packet 147's parting
hypothesis — that `fc362cc4`'s canonical `dissolve_noncentral_gap`/merge-rule changes
needed a downstream stitch/topology handler fix — is **refuted**: `stitch.rs`'s chain walk
needed no closure fix. The handoff's planned diagnose/bisect around `fc362cc4` was moot.
This is the campaign's fourth "the instrument lied, not the code."

**Anti-vacuity checks (a passing gate is a claim; each was verified, not assumed):**
- The gate's body is **byte-identical** to `182892ad`, the commit that recorded 455/898
  (`git diff 182892ad..HEAD` on `cube_4color_arachne.rs` is empty) → no assertion was
  retargeted or weakened; the delta is production-only.
- The test guards its own non-emptiness (`total_checked > 0`) and scanned 699 sub-loops
  over all 125 layers → not a vacuous green.
- The sub-loop population fell **898 → 699**. Fewer loops passing a closure check is
  exactly the shape a false pass takes, so it was **measured**: arachne-vs-classic on the
  real cube_4color gcode gives total outer-wall length **31705.5mm vs 31822.8mm (ratio
  0.9963)**, outer-wall content on all 125 layers in both, per-layer max|X| **137.321 vs
  137.300mm on every layer**. No region dropped — the drop is D4's giant spurious centre
  beads no longer fragmenting real loops into extra travel-delimited sub-loops.

**ADR-0035's second condition paid for itself.** The ADR requires un-ignoring at 0 failures
**and** a re-audit against OrcaSlicer C++ ("the percentage alone does not measure
algorithmic faithfulness"). The percentage was already 0; the re-audit still found a real,
live bug — `D-147-STITCH-TINY-POLY-UNITS`: a spurious `/ UNITS_PER_MM` in
`stitch.rs::finalize_chain` defeated canonical's `3 * max_stitch_distance` tiny-polygon
rule in production (threshold 1.2mm → 0.00012mm), prematurely closing small fragments
which `remove_small_lines` then exempted from cleanup (it skips `is_closed` lines, matching
canonical). Fixed. **Crucially, that bug would have *inflated* closure**, so the gate was
re-measured after fixing it: **still 0/699, identical** — cube_4color's outer loops all
exceed the 1.2mm threshold, so the rule never fires on them and the gate never rested on
the defect. Re-audited **FAITHFUL**: `generate_toolpaths.rs`'s `connectJunctions` domain
walk, and `pipeline.rs`'s post-process order (matches `WallToolPaths::generate`).

**Workspace state at Track C close (`cargo xtask test --summary --workspace --no-fail-fast`):
310 binaries, 2767 passed, 8 failed, 16 ignored, 2 quarantine-skipped.** All 8 failures are
pre-existing; **none is caused by Track C's changes**. Seven are RED-baseline items above.
The eighth was NOT on that list and is worth recording precisely, because the first instinct
was wrong:

- **`perimeter_parity::arachne_perimeter_parity`** (`tests/integration/perimeter_parity.rs`)
  — `tapered_wedge` self-captured baseline: `regions[0].walls[0].path.points[1].x`
  **actual 9.82146 vs expected 3.7797625**. **This is D5's own documented follow-up, not a new
  break.** `5d0e1bcf` made tapered regions emit geometry they previously dropped, so every
  tapered_wedge self-captured baseline drifts "emit more" (actual > expected — the correct
  direction). `1d7eb1de` re-captured the two **slicer-core** fixtures
  (`fixtures/arachne/bead_count_tapered_wedge.json`, `toolpaths_tapered_wedge.json`) but
  **missed this third, slicer-runtime one** — same drift, different crate.
  **Proven pre-existing, not inferred:** stashing *only* the `stitch.rs` production change
  (the session's sole production edit; everything else is tests/docs) and rebuilding guests
  reproduces the **identical** numbers, so the fix has zero effect on this fixture.
  Left RED deliberately: per D-109 + ADR-0042 this is a self-captured baseline, and the
  remedy is Track B's "convert to a structural invariant", **not** a blind rebless.
  → **Track B, carried forward.**

  > **Method note — an inference is not a measurement, even a well-reasoned one.** A reviewing
  > agent, told the stitch fix changes which short fragments close, confidently classified this
  > as "NEW — precisely what this baseline test caught", reasoning from mechanism alone. It was
  > wrong: the stash discriminator took two minutes and refuted it outright. **When a failure
  > appears alongside your change, measure the counterfactual before attributing it** — the
  > plausibility of a causal story is not evidence for it. (Same family as D3 and the layer-96
  > and D1 false positives; this campaign's recurring failure mode is *confident attribution
  > without the control experiment*.)

**Second divergence, found by challenging the fix (2026-07-16) — `D-147-STITCH-GAP-USES-OUTER-BEAD-WIDTH`.**
Asked "goal is OrcaSlicer parity, was your fix correct in that sense?", the re-audit of the
re-audit found that `D-147-STITCH-TINY-POLY-UNITS` had corrected the **units** of the
tiny-poly comparison but never checked the **operand**. Canonical stitches with
`bead_width_x` — the INNER wall width (`stitchToolPaths(toolpaths, this->bead_width_x)`);
PnP passed `preferred_bead_width_outer` (canonical's `bead_width_0`, OUTER). Proven via the
binding `makeStrategy(bead_width_0, bead_width_x, ...)` against the signature
`makeStrategy(preferred_bead_width_outer, preferred_bead_width_inner, ...)`, plus
`makeStrategy`'s own `optimal_width = max_bead_count <= 2 ? outer : inner` local — which is
exactly what PnP's `optimal_width` manifest entry already documented. Fixed via
`stitch_max_gap` (`optimal_width - 1e-6`). **Same root cause as the units bug:** the call
site said *"Matches this packet's brief verbatim"* — a local spec trusted over canonical.
Latent at default config (both widths 0.4mm ⇒ identical), so the AC-1 gate is byte-identical
(0/699) before and after and **no existing fixture could see it**; pinned by
`stitch_gap_follows_inner_bead_width_not_outer`, which drives the widths apart and was
**verified to fail against the old operand** rather than merely pass against the new one.
Also resolved a live doc conflict: `arachne-perimeters/src/lib.rs` claimed `optimal_width`
maps to `ext_perimeter_spacing` (OUTER), contradicting `ArachneParams`' "preferred_bead_width_inner";
canonical settles it as INNER.

> **Method lesson — "is it faithful?" is a different question from "does the gate pass?", and
> both of this session's real bugs were found only by asking the first.** The closure gate was
> 0/699 before either fix and 0/699 after both; neither defect was observable through it. The
> units bug was caught by ADR-0035's mandated re-audit; the operand bug was caught only because
> the fix itself was then challenged. **Generalisation: after fixing a faithfulness defect, audit
> the fix with the same suspicion you brought to the original** — a corrected line is not a
> verified line, and "I checked the units" is not "I checked the expression".

**Method lessons (both new, both earned):**
1. **A false premise in a comment is a bug with a cover story.** `finalize_chain`'s
   division was justified by "the call sites pass `max_gap` in slicer units" — true of the
   *test* call sites, false of production. The unit conventions were split across callers
   (production and `tests/stitch.rs` in mm; three other test files in slicer units) and
   nobody reconciled them. When a comment justifies an adjustment by describing "the call
   sites", **go read the production call site.**
2. **A test whose harness is mis-scaled can assert the exact defect it was written to
   catch.** `arachne_annulus_split` handed `stitch_extrusions` **4000mm of slack against a
   2mm annulus** (while its comment claimed to "run the SAME stitch pass the production
   pipeline uses"), which merged the outer contour with the hole — and its `inset 0 must be
   exactly one closed loop` assertion then *demanded that merge*, i.e. the "missing/merged
   outer walls" bug its own file header exists to catch. Corrected against canonical: every
   boundary polygon (contour and hole alike) seeds bead index 0, so inset 0 holds **two**
   separate closed loops; `PerimeterGenerator.cpp::traverse_extrusions` confirms it
   downstream (`is_external = inset_idx == 0` alone selects `erExternalPerimeter`; the
   contour-vs-hole flag plays no part). Re-pinned with a unit-independent structural nesting
   invariant per ADR-0042. Related to, but distinct from, the `propagation_fills_gap_from_
   central_neighbor` and `ac1`-pentagon instances: here the *fixture's scale*, not the
   fixture's uniformity, is what made the defect invisible. **Generalisation: a test that
   feeds a production function a parameter production would never pass is not testing
   production.**

## D6 (RESOLVED 2026-07-16 — was two defects, not one) — Arachne ignores the user's wall line width

> **Resolution.** The parked item was not just the biggest fish — it was TWO
> fish, and the original analysis below conflated them:
>
> - **Bug B (emission, unlogged until the fix session):** arachne emitted the
>   beading SPACING as the extrusion width. Canonical converts back at emission
>   (`VariableWidth.cpp::thick_polyline_to_multi_path`:
>   `flow.with_width(unscale(w) + height·(1 − π/4))`); PnP never did, so EVERY
>   arachne wall was ~10.7% narrow at default config. The 0.3571 measured below
>   and read as "the hardcoded default width" was actually
>   `line_width_to_spacing(0.4)` escaping the spacing domain — the
>   default-config row of the table was itself a defect, not a baseline.
> - **Bug A (wiring, the logged half):** `arachne_params_from_config` read the
>   internal knobs and never the wall-width keys, so output was invariant to
>   the user's setting.
>
> Fixed as two commits (B then A) so each moved number had one cause. The fix
> shape written below would have DOUBLE-converted (the module already
> spacing-converts what it reads); the real fix re-sourced the raw widths. The
> blast-radius claim was inverted: A is a no-op at default (no fixture sets the
> keys — proven, fixtures byte-identical through the A commit), and it was B
> that moved every arachne fixture (+0.0429 = layer_height·(1 − π/4) on every
> nonzero width, geometry frozen). The "three conflicting default sources"
> prerequisite was already discharged: manifest defaults are never injected
> into the runtime ConfigView, so the resolved default was always the code
> fallback 0.4/0.4; the other two surfaces were lies (classic manifest 0.5 —
> now fixed and guarded by the exhaustive reconcile test; `serialize.rs`
> 0.42/0.45 — logged).
>
> Post-fix measurement (same method as below): classic 0.4000 / arachne
> **0.4000** at default; classic 0.8000 / arachne **0.8000** at
> outer=inner=0.8. North-star gate re-measured:
> `cube_4color_arachne_outer_walls_close_end_to_end` = **0/699 (0.00%)**, mean
> gap 0.0000mm — unchanged. The internal keys are retired (ADR-0043); the
> wall-width/bead-width/flow-spacing distinction is now a CONTEXT.md glossary
> entry. Original analysis preserved below.

### Original entry (2026-07-16, superseded by the resolution above)

`D-160-ARACHNE-IGNORES-WALL-LINE-WIDTH`. Found by completing the width-wiring follow-up
that the `D-147-STITCH-GAP-USES-OUTER-BEAD-WIDTH` fix parked as "suspected, unverified".
**Now proven, by measurement and by code.** This is very likely the largest remaining
Arachne parity defect, and it is *upstream of everything the campaign has fixed so far*:
D5 and D4 corrected where beads are placed and how thick propagated beadings are — this
one says the **target width they are all placed against is the wrong number**.

**Measured (real `pnp_cli` slices of `regression_wedge.stl`; outer-wall width computed
from E-volume / distance / layer height):**

| config | `classic` median outer wall | `arachne` median outer wall |
|---|---|---|
| default | 0.4000mm (n=1160) | **0.3571mm** (n=1320) |
| `outer_wall_line_width = inner_wall_line_width = 0.8` | **0.8000mm** | **0.3571mm** |

Arachne's output is **invariant** to the keys. 0.3571 is exactly `line_width_to_spacing(0.4)`
= `0.4 − 0.2·(1 − π/4)` — the manifest's hardcoded `optimal_width` default of 4000 units. Ask
for 0.8mm walls, get 0.357mm: a **2.24×** error on the most basic wall parameter.

**Code:** `arachne-perimeters/src/lib.rs` has **zero** references to
`outer_wall_line_width`/`inner_wall_line_width`; the manifest declares **neither**.
`arachne_params_from_config` reads two Arachne-INTERNAL knobs instead (`optimal_width`,
`preferred_bead_width_outer`, both defaulting to 4000 units). `classic-perimeters` reads both
wall-width keys directly — so **switching `wall_generator` to arachne silently drops the
user's wall width settings on the floor.**

**Canonical:** `PerimeterGenerator` builds `Arachne::WallToolPaths(last_p, bead_width_0,
perimeter_spacing, ...)` with `bead_width_0 = ext_perimeter_spacing =
ext_perimeter_flow.scaled_spacing()` (OUTER flow) and `bead_width_x = perimeter_spacing =
perimeter_flow.scaled_spacing()` (INNER flow). Upstream **derives** Arachne's bead widths FROM
the user's wall flows. **PnP inverted the relationship** — it exposes Arachne's internals as
user config and never connects them. The `optimal_width` manifest entry documents the trap
against itself: *"Not a user-facing OrcaSlicer PrintConfig.cpp option — upstream sets it
internally."*

**This is why `D-147-STITCH-GAP-USES-OUTER-BEAD-WIDTH` was invisible.** Both keys pin to the
same 4000-unit default, so PnP's outer and inner bead widths are *always equal* — which is
exactly what made using the outer width where canonical uses the inner numerically
undetectable. Fixing this wiring makes that operand distinction live, so the two must be
reasoned about together (the operand fix is already landed, so this ordering is safe).

**Fix shape (NOT applied — needs its own packet):** declare the two wall-width keys in the
arachne manifest; derive `preferred_bead_width_outer = line_width_to_spacing(outer_wall_line_width)`
(canonical `bead_width_0`) and `optimal_width = line_width_to_spacing(inner_wall_line_width)`
(canonical `bead_width_x`); decide explicitly whether the internal keys survive as overrides
(upstream has no such user keys) or are retired. **Blast radius unmeasured:** it changes
emitted wall width for every non-0.4mm config, with real exposure on the self-captured
`perimeter_parity` fixtures and possibly the AC-1 closure gate. **Also unresolved:** PnP has
three conflicting default sources for these keys (`classic-perimeters.toml` outer 0.5 / inner
0.4; `slicer-gcode/src/serialize.rs` outer 0.42 / inner 0.45; a measured default `classic`
slice emits 0.4000) — the resolved default must be established *before* fixing, or the fix
will be calibrated against a guess.

> **Method note — the parked item was the biggest fish.** This was logged as a hedged
> "suspected, unverified" aside at the end of a fix, and it turned out to be a higher-severity
> defect than the one being fixed. Two prior campaign findings (D5, D4) surfaced the same way.
> **Corollary: when an investigation parks something because it is out of scope, the park is a
> lead, not a dismissal** — and "I did not verify this" should be read as work remaining, not as
> a caveat discharged by writing it down.

## Faithfulness follow-ups found by direct canonical read (2026-07-16)

OrcaSlicer source is now vendored at `OrcaSlicerDocumented/` (user-supplied) — every claim below is
read from it directly, not recalled. None are known to affect benchy output today; all are real
divergences from canonical and should be closed or explicitly ADR'd (ADR-0035's bar).

1. **`getOrCreateBeading` synthesis input (D5 fix).** Canonical
   (`SkeletalTrapezoidation.cpp:1808-1839`) derives an unassigned node's bead count from a **min over
   incident edges**: `dist = min(edge->to->distance_to_boundary + |edge|)`, then
   `bead_count = getOptimalBeadCount(dist * 2)` — and only afterwards computes the beading as
   `compute(node->distance_to_boundary * 2, node->bead_count)`. PnP's D5 fix
   (`arachne/generate_toolpaths.rs::generate_junctions`) instead synthesizes from the node's OWN
   radius: `optimal_bead_count(2.0 * to_r)`. Canonical's form is strictly more conservative (it can
   only be ≤ the own-radius form). Canonical also logs `"Unknown beading for non-central node!"` when no
   incident edge is central — a diagnostic PnP has no equivalent of. Not fixed: the D5 fix is verified
   correct on benchy, and this path fires rarely now that beadings propagate correctly; changing the
   synthesis input could shift D5 behaviour and needs its own verification.
2. **Odd `max_bead_count` still in library/test defaults.** Canonical always uses
   `max_bead_count = 2 * inset_count` (`WallToolPaths.cpp:525`), always EVEN; `LimitedBeadingStrategy`'s
   ctor warns on odd (`LimitedBeadingStrategy.cpp:36-40`) and its odd-centre `compute` branch (`:73`)
   parks the surplus in one wide bead. The `arachne-perimeters` module was fixed to derive `2 * wall_count`,
   but core `ArachneParams::default()` (`arachne/pipeline.rs`) and the test `factory_params()` helpers
   (`tests/generate_toolpaths.rs`, `tests/bead_count.rs`, `tests/propagation.rs`) still hardcode **9**.
   Harmless where the cap is never reached, but it is a live trap and it entangles the tapered-wedge
   self-captured baselines (Track B).
3. **`propagate_beadings_upward` bookkeeping.** Canonical carries `dist_to_bottom_source` and
   `is_upward_propagated_only` on the propagated `BeadingPropagation` (`:1583-1585`), and
   `propagateBeadingsDownward` asserts `!top_beading.is_upward_propagated_only` (`:1624`). PnP has no
   `is_upward_propagated_only` flag and recomputes distances structurally
   (`compute_dist_to_bottom_source`), so that canonical assert has no PnP counterpart — meaning PnP
   cannot currently detect the "propagating down from an upward-only beading" condition canonical
   treats as a bug.

## Docs to author

- **ADR-0042:** Arachne parity certified by structural invariants + LLM-visual OrcaSlicer
  comparison, not absolute-unit fixtures.
- **CONTEXT.md glossary:** Self-captured baseline, Structural invariant, LLM-visual oracle,
  Benchy error class, Synthetic reproduction fixture.

## Parked backlog (verified gaps, deferred behind Arachne-output correctness)

T-052 classic flow-wiring · T-054c InnerOuterInner island grouping · T-042 ThickPolyline
reverse converter · T-062b GapFill role arm in `emit.rs::role_equals` · T-065
`filter_out_gap_fill` doc dup · T-005/T-018 arachne manifest speed keys + one-directional
`incompatible-with` · N6 dumbbell test re-strengthening · **D2** benchy bottom layers emit `Sparse
infill` inside the bottom-shell region where OrcaSlicer emits solid — shell-count / solid-infill-threshold
defect, parked 2026-07-15 (out of Arachne scope; user decision "skip only D2").

**Added 2026-07-16 by the Track C ADR-0035 re-audit** (real divergences from canonical,
found by direct source read; all confined to the zero-width inner-contour bookkeeping path,
NOT outer-wall closure, so they did not block Track C):
- **`separate_out_inner_contour` vs canonical `separateOutInnerContour`** (`WallToolPaths.cpp`):
  PnP pushes **every** zero-first-junction-width line into `inner_contour`; canonical skips odd
  lines (`odd lines don't contribute to the contour`) and keeps only even **closed** ones. PnP
  also has **no equivalent of canonical's terminating global even-odd
  `union_(inner_contour, pftEvenOdd)`** Clipper cleanup across all insets — it just concatenates
  raw `ExtrusionLine`s. Affects infill-boundary accuracy.
- **`remove_empty_toolpaths` granularity**: PnP filters individual `ExtrusionLine`s with empty
  junctions; canonical `removeEmptyToolPaths` filters whole empty **inset groups**
  (`VariableWidthLines`) from the top-level vector. An artifact of PnP's flattened
  `Vec<ExtrusionLine>` shape vs canonical's `Vec<VariableWidthLines>`.
- **Doc/code drift (cosmetic, no behaviour change)**: `generate_toolpaths.rs`'s module doc says
  `chain_junctions_for_bead` merges shared-vertex junctions by "keeping the wider surviving
  junction", but the code implements a presence-priority rule (`this_to` if present, else
  `next_from`). The code is arguably closer to canonical's index-based dedup than its own
  comment is; fix the comment, not the code.
