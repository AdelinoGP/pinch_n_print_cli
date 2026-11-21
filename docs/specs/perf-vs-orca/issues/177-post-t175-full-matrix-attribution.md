# Post-t175 full-matrix attribution and 0.50x headroom map

Type: task / attribution + headroom pricing (no production code edit)
Label: wayfinder:task
Status: resolved (2026-11-21; full 8-cell post-t175 phase/stage/host-op/guest-fuel attribution + `< 1.0x` and `<= 0.50x` headroom map verified 32/32 byte-identical; unblocks 179 & 180 and graduates 181)
Assignee: 2026-11-21 Antigravity wayfinder session — resolved (zero production edits).
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

**Resolved 2026-11-21 (attribution + headroom map only; zero production code edits; unblocks [Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen](179-per-layer-tree-support-and-infill-linker-screen.md) and [`ShellClassification` post-t172 residual attribution and speedup screen](180-shell-classification-post-t172-residual-screen.md); graduates [`SupportGeometry` post-planner host tail attribution and reduction screen](181-support-geometry-post-planner-host-tail.md)).**

Evidence: [`evidence/t177-post-t175-attribution/FINDINGS.md`](../evidence/t177-post-t175-attribution/FINDINGS.md) (`verify.py` PASS 5/5 checks, `--self-test` **12/12** negative controls rejected; `cargo xtask build-guests --check` exit `0`; `target/release/pnp_cli.exe` sha256 `f3075232…`, 24 external core modules tree sha256 `b46eb4cf…`; all **24/24** instrumented passes + **8/8** `--profile` passes exit `0`, `degraded = false`, `0` code-1200, byte-identical to `t175`'s active `reference.json`; phase shares sum to `1.000000` on 24/24 passes, max prepass stage reconciliation `4.09 %`, max postpass `2.04 %`).

### 1. Post-t175 phase split (`--instrument-stderr`, median of 3 unsandboxed passes/cell)

Prepass fell **−2.4 to −7.5 pp** across all eight cells vs `t171`, while per-layer and postpass grew (notably `postpass` reached **11.8 %** on both `benchy` supports-on cells):

- `benchy-classic-off`: **prepass 55.1 %** (−3.1 pp), **per-layer 32.4 %** (−0.0 pp), **postpass 8.2 %**, **outside 4.1 %** (`1,700 ms`, spread `1.74 pp`)
- `benchy-classic-on`: **prepass 66.8 %** (−4.8 pp), **per-layer 19.2 %** (+0.7 pp), **postpass 11.8 %**, **outside 2.0 %** (`3,452 ms`, spread `0.66 pp`)
- `benchy-arachne-off`: **prepass 53.5 %** (−2.4 pp), **per-layer 34.3 %** (−0.4 pp), **postpass 8.1 %**, **outside 3.9 %** (`1,783 ms`, spread `0.76 pp`)
- `benchy-arachne-on`: **prepass 63.9 %** (−6.1 pp), **per-layer 22.1 %** (+1.7 pp), **postpass 11.8 %**, **outside 2.0 %** (`3,553 ms`, spread `5.04 pp`)
- `base-classic-off`: **prepass 70.9 %** (−5.1 pp), **per-layer 21.3 %** (+2.7 pp), **postpass 7.0 %**, **outside 0.8 %** (`9,913 ms`, spread `1.84 pp`)
- `base-classic-on`: **prepass 77.7 %** (−6.0 pp), **per-layer 14.3 %** (+2.2 pp), **postpass 7.7 %**, **outside 0.3 %** (`29,533 ms`, spread `4.32 pp`)
- `base-arachne-off`: **prepass 66.9 %** (−7.5 pp), **per-layer 25.2 %** (+5.0 pp), **postpass 6.8 %**, **outside 0.8 %** (`10,591 ms`, spread `1.83 pp`)
- `base-arachne-on`: **prepass 76.7 %** (−6.5 pp), **per-layer 16.1 %** (+3.5 pp), **postpass 7.3 %**, **outside 0.3 %** (`28,879 ms`, spread `0.65 pp`)

*(Disclosed: an initial sandboxed capture inflated `per-layer` by ~2.8× on `benchy` via Windows sandbox syscall/stderr overhead; those raw streams were discarded and all 32 runs were captured cleanly unsandboxed, with `slice_complete_ms` within `−5.7 % to +4.6 %` of `t175`'s uninstrumented candidate medians.)*

### 2. Host Clipper per-op census & deterministic guest fuel (`--profile`, 1 pass/cell)

- **All 8 cells are now `clip_polygons`-dominated** (reversing `t171`'s supports-off offset dominance): host Clipper thread-time dropped **−36.8…−40.3 %** on supports-off (`88.85 → 53.40 s` `base-classic-off`; `7.56 → 4.51 s` `benchy-classic-off`) and **−58.4…−59.7 %** on supports-on (`343.38 → 140.34 s` `base-classic-on`; `20.57 → 8.56 s` `benchy-classic-on`). `clip_polygons` now holds **46.5–51.6 %** (supports-off) and **54.0–58.6 %** (supports-on) vs `offset` **30.8–37.8 %**, `offset2_ex` **4.9–14.1 %**, `closing_ex` **1.1–3.6 %**.
- **Deterministic guest fuel** (`−3.3 % to −16.3 %` vs `t171`): supports-on is dominated by `com.core.tree-support` (**61.0 / 60.8 %** = `41.94 B` on `benchy`; **68.9 / 70.3 %** = `388.79 B` on `base`) + `com.core.tree-support-planner` (**8.5–8.8 %**, `5.85 / 48.56 B`); supports-off is split between `com.core.infill-linker` (**50.7 / 54.2 %** `benchy`, **30.4 / 35.9 %** `base`) and the active perimeter generator (`classic-perimeters` **38.6 %** `benchy` / **61.5 %** `base`; `arachne-perimeters` **34.8 %** `benchy` / **55.3 %** `base`).

### 3. `< 1.0x` closure and `<= 0.50x` headroom map (`ESTIMATE ONLY`, cross-window `t174 × t175`)

- **5 of the 6 open cells now have a single-stage `< 1.0x` ceiling; ONLY binding `benchy-classic-on` (`~1.570x` ord / `~1.602x` acc, 36.3 % cut) cannot close from one stage:**
  - `base-classic-on` (`~1.221x`, 18.1 % cut) & `base-arachne-on` (`~1.201x`, 16.7 % cut): `PrePass::SupportGeometry` is **44.4 % / 43.4 %** (**0.678x / 0.679x** at zero; needs only ~39–41 % of stage).
  - `benchy-classic-off` (`1.144x`, 12.6 % cut) & `benchy-arachne-off` (`1.155x`, 13.4 % cut): `PrePass::ShellClassification` is **27.9 % / 27.0 %** (**0.825x / 0.843x** at zero; **2.22× / 2.02×** cut), and `per-layer` is **32.4 % / 34.3 %** (**0.774x / 0.758x**).
  - `benchy-arachne-on` (`~1.389x`, 28.0 % cut): `SupportGeometry` (**30.4 %** → **0.966x**) crosses only at 92 % removal; realistic closure is multi-track (`SupportGeometry + ShellClassification + tree-support` = **53.3 %** → **0.648x**).
  - `benchy-classic-on` (`~1.570x`, **36.3 % cut**, binding): `SupportGeometry` (**31.7 %** → **1.071x**), `per-layer` (**19.2 %** → **1.268x**), `ShellClassification` (**13.8 %** → **1.353x**), `postpass` (**11.8 %** → **1.385x**), `SupportAnalysis` (**7.9 %** → **1.446x**). `ShellClassification + per-layer` (**33.1 %** → **1.051x**) still fails without `SupportGeometry`; closing `benchy-classic-on` requires stacking wins across **3–4 tracks** (`SupportGeometry + ShellClassification + tree-support + SupportAnalysis` = **63.4 %** addressable).
- **`<= 0.50x` (2× faster) headroom theorem:** **NO `benchy` cell can reach `<= 0.50x` via prepass deletion alone** (zeroing all of `prepass` leaves `benchy` at **0.513x / 0.521x / 0.537x / 0.501x**). All four `benchy` cells require BOTH prepass AND per-layer cuts (`prepass + per-layer` zero → `0.141x–0.219x`), and `benchy-classic-on` (`68.2 %` cut; `postpass + outside` = `13.8 %`) requires cutting across `SupportGeometry`, `ShellClassification`, `SupportAnalysis`, `per-layer`, and likely `postpass`. On the two provisionally won `base` supports-off cells (`0.769x / 0.792x`, needing **35.0 % / 36.9 %** cuts to hit `0.50x`), `ShellClassification` (**37.0 % / 34.2 %** → **0.485x / 0.521x**) plus `per-layer` (**21.3 % / 25.2 %** → combined **0.321x / 0.321x**) provides wide headroom.

### 4. Sharpened inputs for `t178`–`t180` and new graduation `t181`

- **Graduated [`SupportGeometry` post-planner host tail attribution and reduction screen](181-support-geometry-post-planner-host-tail.md):** `2,293 / 2,017 ms` (**7.8 / 7.0 %** cell, **17.5 / 16.1 %** of `SupportGeometry`) on `base` supports-on and `215 / 216 ms` (**6.2 / 6.1 %** cell, **19.6 / 20.0 %** of stage) on `benchy` supports-on elapse *after* `com.core.tree-support-planner::module_complete` and *before* `PrePass::SupportGeometry::stage_complete` (`0 ms` on supports-off). This is a separate host lever from [t178](178-planner-phase3-union-attribution.md)'s Phase-3 union (`59.29 / 59.91 s` `host:native` clip thread on `base`, `3.04 / 3.05 s` on `benchy`).
- **Sharpened inputs for unblocked [t179](179-per-layer-tree-support-and-infill-linker-screen.md):**
  - `com.core.infill-linker` on **single layer `L2`** (`191 / 226 ms` vs `p50 = 3 ms`) creates a **134 ms (7.9 % cell, 24.4 % of `per-layer`)** and **104 ms (5.8 % cell, 17.0 % of `per-layer`)** idle-core straggler tail on `benchy-classic-off` / `benchy-arachne-off` (11/12 cores idle; `L2` also takes `292–306 ms` on `benchy` supports-on) — fixing or front-loading `L2` saves 1:1 serial wall time.
  - Fuel rank ≠ per-layer elapsed rank on supports-off: `classic-perimeters` (**44.8 %** thread → **14.5 %** proportional cell) and `arachne-perimeters` (**59.3 %** thread → **20.3 %** cell) outrank `infill-linker` (**36.9 % → 11.9 %** classic, **28.5 % → 9.8 %** arachne) because host Clipper/Voronoi calls aren't fuel-metered.
  - `com.core.tree-support`: **61.0 / 60.8 %** fuel (`41.94 B`) and **9.9 / 9.5 %** proportional cell (`benchy-on`); **68.9 / 70.3 %** fuel (`388.79 B`) and **8.3 / 8.2 %** proportional cell (`base-on`), `96.8–97.1 %` `<module self>`.
  - `com.core.raft-default`'s `252–284 ms` / `2,107–2,213 ms` per-layer thread is **99 % `first_in_stage` host region-preparation cost** (`t155`), not raft compute.
- **Sharpened inputs for unblocked [t180](180-shell-classification-post-t172-residual-screen.md):**
  - `PrePass::ShellClassification`: **27.9 / 27.0 %** (`474 / 482 ms`, `0.825x / 0.843x` zero) on `benchy-off`, **13.8 / 13.4 %** (`478 / 477 ms`) on `benchy-on`, **37.0 / 34.2 %** (`3,664 / 3,624 ms`, `0.485x / 0.521x` zero) on `base-off`, **12.4 / 12.9 %** (`3,676 / 3,720 ms`) on `base-on`. `<module self>` is `0.0 %` (`102–243 µs`), yet `--profile` thread / instrumented-wall is **8.08–8.61×** on `base` vs only **4.50–4.57×** on `benchy` — flagging low `benchy` parallelism alongside op volume (`clip_polygons` 39 % / `offset` 33 % / `offset2_ex` 21 % on `benchy`).
  - Sibling prepass stages: `PrePass::Slice` **9.1–9.4 %** (`benchy-off`), **13.8–14.0 %** (`base-off`), **4.6–5.0 %** (`on`); `PrePass::SupportAnalysis` **7.8–7.9 %** (`benchy-on`, `3.2–3.4×`), **9.5–10.0 %** (`base-on`, `6.9–7.0×`, `84 % offset`); `PrePass::SeamPlanning` **6.2–8.0 %** (`off`) / **2.7–3.3 %** (`on`); `PrePass::MeshAnalysis` **4.8–6.1 %** (`off`) / **2.0–2.5 %** (`on`); `PrePass::OverhangAnnotation` **0.5–1.6 %**.
