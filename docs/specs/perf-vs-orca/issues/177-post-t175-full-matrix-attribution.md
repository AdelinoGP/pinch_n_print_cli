# Post-t175 full-matrix attribution and 0.50x headroom map

Type: task / attribution + headroom pricing (no production code edit)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Destination ruling: CPU-corroboration semantics and sample count for cell closure](176-destination-cpu-corroboration-ruling.md)
(with [t174](174-post-t172-scoreboard-refresh.md) and
[t175](175-tiled-collision-carve.md)), because the last full-matrix phase,
stage, host-op, and guest-fuel attribution
([t171](171-post-t169-prepass-reprice.md)) predates **both** `t172`
(`OffsetConcaveCrossing`, −14.9…−23.0 % wall across 8/8) and `t175` (phase-1
collision tiling, −16.1…−31.3 % wall on supports-on), leaving all four `benchy`
cells and the non-Phase-3 `base` residuals unpriced at HEAD.

## Question

**At post-t175 HEAD, what is the exact phase split, prepass stage split,
host-clipper per-op census, and per-module guest-fuel breakdown across all eight
cells — what combination closes the binding `benchy-classic-on` (~1.57x est.)
and near-win `benchy` supports-off (1.14–1.15x) cells, and what is each cell's
verified ceiling toward both `< 1.0x` and `<= 0.50x` (the planned follow-on 2×
faster effort)?**

Concretely:

1. **Re-derive the 8-cell phase & stage split** (median of 3 instrumented passes
   per cell via `--instrument-stderr`, same bracket definitions as
   [t164](164-phase-re-attribution.md) and [t171](171-post-t169-prepass-reprice.md):
   `prepass`, `per-layer`, `postpass`, `outside`, plus every sequential prepass
   stage bracket). Every pass must reproduce `t175`'s active currency
   ([`evidence/t175-tiled-collision-carve/reference.json`](../evidence/t175-tiled-collision-carve/reference.json))
   byte-for-byte with `degraded = false` and `0` code-1200 diagnostics.
2. **Re-derive the 8-cell `--profile` per-op and guest-fuel census** (1 pass per
   cell; thread-time attribution only, never denominated as wall):
   - Host clipper breakdown (`offset`, `clip_polygons`, `offset2_ex`,
     `closing_ex`) by owning host built-in (`host:shell_classification`,
     `host:support_analysis`, `host:slice`, `host:mesh_analysis`,
     `host:overhang_annotation`, `host:native`).
   - Deterministic guest WASM fuel per module (`tree-support`,
     `tree-support-planner`, `infill-linker`, `classic-perimeters`,
     `arachne-perimeters`, etc.) and per-layer wall share per module.
3. **Price the 6 open cells against `< 1.0x` AND all 8 cells against `<= 0.50x`:**
   - Using `t174`'s quiet-board Orca medians (and `t175`'s paired supports-on
     deltas disclosed explicitly as cross-window estimates, never ratios of
     record), rank the single-stage and multi-stage ceilings for:
     - `benchy-classic-on` (binding, ~1.57x est.) and `benchy-arachne-on` (~1.39x est.),
     - `benchy-classic-off` (1.144x) and `benchy-arachne-off` (1.155x),
     - `base-classic-on` (~1.22x est.) and `base-arachne-on` (~1.20x est.),
     - `base-classic-off` (0.769x, provisionally won) and `base-arachne-off` (0.792x, provisionally won) toward `<= 0.50x`.
4. **Sharpen the inputs for blocked tickets
   [179](179-per-layer-tree-support-and-infill-linker-screen.md) and
   [180](180-shell-classification-post-t172-residual-screen.md)** by stating the
   exact post-t175 wall/fuel share and ceiling of `com.core.tree-support`,
   `com.core.infill-linker`, `PrePass::ShellClassification`, `PrePass::Slice`,
   `PrePass::SupportAnalysis`, and `PrePass::SupportGeometry` remainder.

## Scope

- Ordinary arm at HEAD (`cargo xtask build-guests --check` exit 0 before
  running); `RAYON_NUM_THREADS=12`, standing matched-pair configs.
- Zero production code changes, zero reference moves, no candidate A/B, and no
  OrcaSlicer scoreboard re-run (cross-window route arithmetic is explicitly
  labelled `ESTIMATE ONLY`).
- Check `PrePass::SupportGeometry` vs `per-layer` vs `postpass` on the four
  `benchy` cells specifically so the route knows whether `benchy` supports-off
  closes via `ShellClassification` alone, per-layer (`infill-linker` /
  perimeters), or both, and which combination closes `benchy-classic-on`.

## Pre-declared gate

**Attribution-only (no performance gate).** Correctness & method bar:

- `cargo xtask build-guests --check` exits 0; exe and module-tree SHA-256
  stamped.
- All instrumented/profile runs exit 0, `degraded = false`, `0` code-1200, and
  reproduce `t175`'s active `reference.json` **8/8** byte-for-byte.
- Phase brackets reconcile with `slice_complete` and stage brackets reconcile
  with the prepass bracket in all 8 cells.
- `--profile` numbers are denominated strictly as accumulated thread-time /
  deterministic guest fuel, never wall claims.

## Acceptance

- Reduced evidence under `docs/specs/perf-vs-orca/evidence/t177-post-t175-attribution/`
  with `FINDINGS.md`, `REPRODUCE.md`, machine-readable reduced JSONs, and a
  checkout-only `verify.py` (`verify.py` + `verify.py --self-test` exit 0).
- Updated `## Answer` in this ticket, `map.md` Destination status +
  Decisions-so-far + unblocking note for
  [179](179-per-layer-tree-support-and-infill-linker-screen.md) and
  [180](180-shell-classification-post-t172-residual-screen.md).

## Answer

_(open)_
