# Consume-only-when-read annotations

Type: task
Status: resolved (2026-10-22; human-approved KEEP - candidate committed
`053904aa`; proof 16/16 byte-identical to the t58 references; standing paired
gate PASS in all four pre-declared classic supports-off cells, 6/6 pairs each:
wall -25.93/-34.47/-14.38/-18.10 per cent and CPU -37.67/-39.61/-24.47/-21.58
per cent, ordinary/accelerated x benchy/base)
Assignee: 2026-10-21 OpenCode wayfinder session — the human lifted the
below-fold park for this take and selected this ticket explicitly ("Lift the
park; take ticket 20"). The park lift is candidate-scoped to this ticket and
does not redraw the map's destination or ticket 12's ~1.5x condition.
Blocked by: 12, 18, 19

## Question

Design and measure consume-only-when-read per-vertex annotations in
`expolygon_to_path3d_indexed` (`crates/slicer-core/src/perimeter_spatial.rs`) —
compute each per-vertex quantity (`overhang_quartile`,
`signed_distance_to_boundary`, `is_bridge`) only when a downstream consumer
actually reads it. This is lead 3 of the
[emit_walls premise falsified](08-emit-walls-premise-falsified.md) ranking,
which the mode-comparative addendum **raised in weight**: with acceleration
necessary-but-not-sufficient, avoiding work the consumer never reads compounds
with whatever the query path eventually costs.

Scoping comes first: [Accelerated residual query
attribution](18-accelerated-residual-query-attribution.md) determines what is
still worth avoiding once accelerated queries and fallback suppression are
accounted for — the candidate is that remainder, not the ordinary-mode 69.8%.

Constraints: annotations must be value-identical wherever they are read
(output preserved exactly); the change is guest-side in the classic perimeter
path (and `PerimeterSpatialContext::is_bridge`'s scan belongs in the same
look-before-you-read design if the split says so).

Acceptance: paired ordinary + accelerated A/B per the standing metric, then
keep/drop to the human. Timing chain position 5.

## Comments

### Routing claim and premise audit — 2026-10-08 (read-only; no take started)

The session took this ticket as the first open, unblocked child on resumption
(t19 closed 2026-10-08 with a KEEP). Per the map's convention the claim is a
routing claim only: the below-fold park in
[Gap budget per cell](12-gap-budget-per-cell.md) is **not** lifted (t19's lift
was explicitly candidate-scoped), so no implementation, experiment or timing
campaign was started. The claim resolved into the following premise audit at
HEAD `b56f429a` (t19's KEEP commit); every fact below is code/config-derived —
no slice was run.

**The premise survives and sharpens: on the matched job the only semantic
consumer of the two stamps returns before reading them, so the
`overhang_quartile` / `overhang_distance_mm` stamping is unread work there.**

1. **Consumer audit.** The only production site that makes a *decision* on
   `Point3WithWidth::overhang_quartile` / `overhang_distance_mm` is
   `OverhangClassifierDefault::run_finalization`
   (`modules/core-modules/overhang-classifier-default/src/lib.rs`): it reads
   `point.overhang_quartile.is_some()` and `point.overhang_distance_mm` to build
   per-point speed factors. Every other referencing site transports or
   null-fills: `seam-placer`'s `interpolate_point` copies whichever side of `t`
   it is on; `seam-planner-default` writes `None`; `fuzzy-skin` nulls;
   `wave-overhangs`, `part-cooling`, `path-optimization-default`,
   `machine-gcode-emit` and `crates/slicer-gcode` never read them; the
   `slicer-wasm-host` marshal sites are transports; tests read.
   `WallFeatureFlags::is_bridge` sits differently: classic's `build_ring_wall`
   drives `bridging_flow` through it into `flow_factor`, which
   `crates/slicer-gcode` consumes for `E`, so it cannot be dropped as unread
   while any wall vertex is a bridge vertex; whether the matched job carries one
   is a take-time measurement. Classic also lacks arachne's
   `!bridge_areas.is_empty()` empty-input guard around the scan (arachne guards,
   classic does not — a parity-shaped observation, not yet a candidate).

2. **The liveness gate.** `run_finalization` early-returns twice: on
   `enable_overhang_speed == false` (manifest default **true**) and then on all
   four `overhang_1_4_speed` … `overhang_4_4_speed` bands at 0 (manifest
   defaults **0.0**). None of the four matched configs
   (`evidence/matched-pair/configs/pnp-{classic,arachne}-supports-{off,on}.json`)
   authors any of those keys, so on every matched cell the module returns
   before reading a single stamp, while `classic-perimeters` /
   `arachne-perimeters` still run every query. (`slowdown_for_curled_perimeters`
   shares the same table and the same skip; `bridge_flow` / `thick_bridges` are
   untouched.)

3. **What that means for the candidate shape.** The surviving design is not a
   faster query but a **conditional stamp**: gate the two stamps on the
   consumer's own liveness predicate, value-identical when live. The queries it
   removes are the measured dominant cost: ticket 08 measured them at 69.8% of
   classic guest fuel (ordinary); ticket 18's accelerated stage-2 capture puts
   the two exact-scan families at ≈61.7% benchy / 57.9% base of the classic
   module (`inside_scan` 147.13 / 2,814.51 B + `quartile_scan` 141.48 /
   1,004.66 B of 467.50 / 6,598.34 B); ticket 25 puts Arachne's two queries at
   86.4% of that module. The wall-transfer caveat from
   [Gap budget](12-gap-budget-per-cell.md) still governs: this cannot close a
   6.28–26.24x gap, and the standing paired A/B decides keep/drop.

4. **Take-readiness notes.** Exactness requires gating on exactly the
   classifier's predicate (`enable_overhang_speed` && any band ≠ 0) so a live
   config keeps byte-identical stamps; the liveness keys would have to be
   visible to the perimeter modules' config views. Both generators stamp (the
   ticket's "classic path" wording is narrower than the sites — arachne holds
   the same two stamps in `run_perimeters`). The existing
   `PathAnnotationMode::NonPlanarNoQuartile` split already proves the per-mode
   stamp distinction. This audit is static — a take must re-verify dynamically
   (classifier skip observed, no other runtime reader).

**Route state.** The frontier is this ticket and [serial host
floor](27-serial-host-prepass-floor.md); both sit under standing park decisions
and the route awaits the human's selection. No candidate, experiment or timing
was started.

### Benchy refresh context — 2026-10-13 (not a take or park override)

The [Benchy current bottleneck refresh](61-benchy-current-bottleneck-refresh.md)
re-derived the matched ordinary Classic profile in both support states after
the finalization KEEP. Classic perimeter module self is 639.971 billion fuel
in both cells; total perimeter fuel is 89.62%/84.51% of whole-job fuel
(off/on). This keeps the existing lead relevant, but the aggregate's unmarked
self bucket does not dynamically prove the liveness audit or isolate the two
stamps. No duplicate ticket, annotation candidate or park lift follows.
Evidence: [fresh findings](../evidence/perf-refresh-2026-10-13/FINDINGS.md).

### Base refresh context — 2026-10-18 (not a take or park override)

The [Base current bottleneck refresh](73-base-current-bottleneck-refresh.md)
re-derived the four matched ordinary base cells at post-t70 HEAD with fresh
fuel profiles and instrumentation-only corroborations. On base Classic the
per-layer phase is **94.3 s (supports-off) / 98.2 s (supports-on)**
instrumented — 44 % / 24 % of process wall — and
`com.core.classic-perimeters` is 98.8 % / 95.0 % of whole-job guest fuel with
**10,573 B self fuel** and 1,186.7 / 1,195.1 s accumulated wall. Ticket 18's
accelerated stage-2 capture put the two exact scan families at **57.9 % of the
base classic module fuel**; this refresh does not re-split those scans.

The below-fold park is **not lifted**: the t72 best cell is still 3.30x and
ticket 12's condition is unchanged. This comment records the base size of the
parked term for the human's route decision — the 57.9 % is an attribution
share, not a wall prediction. No duplicate ticket, annotation candidate or
park lift follows. Evidence:
[findings](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md).

### Take claimed — 2026-10-21 (pre-declaration, before any measured build)

The human lifted the below-fold park for this ticket and selected it as the
session's route. The take implements the audit's conditional-stamp shape:

- A shared predicate, `overhang_annotations_consumed` in
  `crates/slicer-sdk/src/config_predicates.rs`, mirrors the classifier's own
  liveness gate exactly (`enable_overhang_speed` && any of the four
  `overhang_{1..4}_4_speed` bands != 0). The classifier now calls the same
  helper, so producer and consumer cannot disagree by construction.
- `classic-perimeters` and `arachne-perimeters` declare the five predicate
  keys in their manifests (same types/defaults/denied_scopes as
  `overhang-classifier-default.toml`) and read the predicate once per
  `run_perimeters`. When inactive, the planar wall path uses the new
  `PathAnnotationMode::Unconsumed` (both `overhang_quartile` and
  `overhang_distance_mm` stay `None`), and arachne skips both stamp loops.
  When live, the exact pre-take modes/queries run unchanged.
- `is_bridge` is deliberately not touched: the t18 split localizes the hot
  cost in the two exact scan families, and `is_bridge` has no consumer
  liveness predicate (its flow-factor consumer is unconditional). Classic's
  missing `bridge_areas.is_empty()` parity guard remains a separate
  observation, not this candidate.

**Dynamic premise check (done before the campaign):** one ordinary base
classic-supports-off slice at `RAYON_NUM_THREADS=12` with a temporary
`T20-PROBE` in the classifier logged `consumed=false layers=495`; the probe
was then removed (grep-guarded before the snapshot builds). Output hash
`20e7204c...` equals the preserved t58 base-classic-off reference.

**Pre-declared acceptance (before any measured build):**

- Proof: all 16 cells ({benchy, base} x {classic, arachne} x supports
  off/on x ordinary/accelerated) baseline and candidate byte-identical,
  clean, non-vacuous, and equal to the preserved t58 references.
- Standing paired gate: the four classic supports-off cells
  (ordinary/accelerated x benchy/base), 6 alternating pairs per cell plus 2
  warmups per arm, `RAYON_NUM_THREADS=12`, external `GetProcessTimes`.
  Wall must fall (paired-median delta < 0) and process CPU must corroborate
  (paired-median delta <= +1.0 %) in all four cells; fewer than 6 retained
  pairs makes a cell inconclusive.
- No candidate source is committed before the human rules.

Evidence root (gitignored raw captures):
`.local-artifacts/perimeter-reference-preparation/t20-consume-only/`; the
committed reduced set lands under `docs/specs/perf-vs-orca/evidence/t20-consume-only/`.

## Answer

**Resolved 2026-10-22 — human-approved KEEP; candidate committed `053904aa`.**

### Premise re-verified

The 2026-10-08 consumer audit was re-checked at HEAD: no production site
makes a decision on `overhang_quartile` / `overhang_distance_mm` except
`OverhangClassifierDefault::run_finalization`, and that module returns before
reading a stamp when its liveness predicate is false. The predicate is
`enable_overhang_speed` && any of the four `overhang_{1..4}_4_speed` bands != 0.
None of the matched configs authors those keys; the manifest defaults are
`true` / `0.0`, so the predicate is false on every matched cell.

**Dynamic check (before any measured build):** a temporary `T20-PROBE` in the
classifier printed `T20-PROBE classifier consumed=false layers=495` on one
ordinary base classic-supports-off slice at `RAYON_NUM_THREADS=12`. The probe
was removed and `build-snapshots.ps1` grep-guards it; that cell's output hash
`20e7204c...` equals the preserved t58 reference.

### Candidate

- `crates/slicer-sdk/src/config_predicates.rs` adds
  `overhang_annotations_consumed` (required-read predicate) and a
  producer-side variant that reports inactive on a missing key. The classifier
  calls the required-read variant, so producer and consumer share one
  definition.
- `classic-perimeters` and `arachne-perimeters` declare the five predicate keys
  in their manifests (same types/defaults/denied scopes as
  `overhang-classifier-default.toml`) and read the predicate once per
  `run_perimeters`.
- `PathAnnotationMode::Unconsumed` (`crates/slicer-core/src/perimeter_spatial.rs`)
  emits neither annotation and runs neither query. Classic's planar wall path
  uses `Planar` when live and `Unconsumed` when dead; its non-planar shell path
  uses `NonPlanarNoQuartile` when live and `Unconsumed` when dead. Arachne
  skips both stamp loops when dead and keeps the pre-take stamps when live.
- `is_bridge` is deliberately untouched: the t18 split localizes the hot cost
  in the two exact scan families, and `is_bridge` has no consumer liveness
  predicate (its flow-factor consumer is unconditional). Classic's missing
  `bridge_areas.is_empty()` parity guard remains a separate observation.

When the predicate is live, the exact pre-take queries and modes run, so the
change is value-identical wherever the stamps are read.

### Acceptance

- **Proof 16/16 PASS.** All 16 cells ({benchy, base} x {classic, arachne} x
  supports off/on x ordinary/accelerated) baseline and candidate byte-identical,
  clean, non-vacuous, and equal to the preserved t58 references
  (`qualification.json`).
- **Standing paired gate PASS in all four pre-declared classic supports-off
  cells**, 6 alternating pairs each plus 2 warmups per arm, zero starvation
  exclusions, every pair byte-identical:

| cell | baseline median wall | candidate median wall | paired wall delta | paired CPU delta |
| --- | ---: | ---: | ---: | ---: |
| ordinary benchy classic-off | 12.367 s | 9.282 s | -25.93 % | -37.67 % |
| ordinary base classic-off | 157.775 s | 103.396 s | -34.47 % | -39.61 % |
| accelerated benchy classic-off | 10.192 s | 8.733 s | -14.38 % | -24.47 % |
| accelerated base classic-off | 120.004 s | 98.399 s | -18.10 % | -21.58 % |

The larger CPU reduction corroborates work removal rather than a scheduling
shift. The mechanism matches the premise: the two exact scan families were
57.9 % of the accelerated base classic module fuel (t18) and 69.8 % of classic
guest fuel (t08).

### Evidence

- [`evidence/t20-consume-only/`](../evidence/t20-consume-only/FINDINGS.md):
  `FINDINGS.md`, `qualification.json`, `ab-reduction.json`, `candidate.patch`,
  the four drivers/checkers, and `verify.py`, which exits 0 from the committed
  set alone with four negative controls rejected.
- Raw snapshot/capture root (gitignored):
  `.local-artifacts/perimeter-reference-preparation/t20-consume-only/`.
- Pre-commit gates: workspace check and clippy `-D warnings` clean,
  `check-literals` green, `check-test-quality --report` 0 findings,
  `gen-config-docs --check` current, ordinary and accelerated guest freshness
  exit 0.

### Caveats

- The gate cells are classic supports-off; arachne and supports-on output
  identity is proven in all 16 cells, but their wall effect was not timed in
  this take.
- The proof warmup walls are single runs, not the paired gate.
- The candidate does not touch `is_bridge`; classic's missing
  `bridge_areas.is_empty()` parity guard remains a separate observation.

