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
| `bridge_vertices_get_bridge_flow_ratio_when_thin` | `modules/core-modules/arachne-perimeters/tests/bridge_flow_factor_tdd.rs:53` | FIX |
| `bridge_vertices_get_round_section_factor_when_thick_bridges_on` | same:123 | FIX |
| `ac1_local_maximum_emits_hexagonal_micro_loop` | `crates/slicer-core/tests/arachne_local_maxima_single_beads.rs:44` | FIX (also: stale RED-style header from before P145 closed N9 — clean up) |
| `bead_count_tapered_wedge` | `crates/slicer-core/tests/bead_count.rs:205` | FIX (fixture predates P155 clamp; re-verify, don't blind-rebless) |
| `generate_toolpaths_tapered_wedge` | `crates/slicer-core/tests/generate_toolpaths.rs:278` | FIX (shares wedge fixture with bead_count) |
| `wedge_multi_layer_top_bottom_evidence` (packet-109 bottom-surface) | `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs:1596` | FIX (root cause not pinned statically — confirm) |
| `legacy_zero_matches_golden` | `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs:225` | FIX (byte-exact golden — inspect diff before rebless) |
| `arachne_perimeters_simple_square_produces_walls` | `crates/slicer-runtime/tests/executor/arachne_perimeters_simple_square.rs:44` | FIX (real WASM-guest; likely broken by `57191889`) |
| `cube_4color_ironing_per_painted_top_color` | `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs:170` | FIX (previously-green gate; paint/region-order churn) |
| `cube_4color_*` gcode (exact fn TBD) | `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` | FIX (confirm which fn) |
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
> edges are segment-segment not point-point. Follow-up: 2 tapered-wedge SELF-CAPTURED baselines now
> drift "emit more" (correct direction) → re-baseline / convert to structural invariants (Track B).

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
- **C — closure-chain fix:** localize cube_4color closure (arachne `pipeline.rs`/
  `generate_toolpaths.rs`/`stitch.rs`); drive to 0 failures; un-ignore only at green.

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
