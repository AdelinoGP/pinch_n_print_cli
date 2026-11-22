# Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen

Type: task / attribution + candidate screen (no production commit in this take;
probe reverted if used)
Label: wayfinder:task
Status: resolved 2026-11-21 — attribution + exact ablation + unsandboxed screen complete (zero production edits; `probe.patch` cleanly reverted; both guest freshness gates exit 0; 130/130 runs byte-identical to `t175`); graduated [183](183-per-layer-tree-support-and-infill-linker-exact-acceleration.md)
Assignee: agent (2026-11-21)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 177 (resolved — unblocked 2026-11-21; see `evidence/t177-post-t175-attribution/FINDINGS.md` §6 for post-t175 module shares, fuel-vs-wall inversion, and the `L2` `infill-linker` 134/104 ms idle-core straggler tail)
Graduated from: [Destination ruling: CPU-corroboration semantics and sample count for cell closure](176-destination-cpu-corroboration-ruling.md),
using the pre-t172 fuel census in [t171](171-post-t169-prepass-reprice.md)
(where `com.core.tree-support` held **57.7–68.0 %** of all supports-on guest
fuel and `com.core.infill-linker` held **52.5–57.6 %** of benchy supports-off
guest fuel) and the post-t175 realization that `benchy-classic-on` (binding,
~1.57x est.) is **not** boolean-bound (only `2.93 s` `host:native` clip thread
remains) and both `< 1.0x` on `benchy` and the follow-on `<= 0.50x` goal need
real per-layer/guest cuts.

## Question

**Using [t177](177-post-t175-full-matrix-attribution.md)'s verified post-t175
per-layer wall and guest-fuel shares, what specific kernels inside
`modules/core-modules/tree-support/src/lib.rs` (`com.core.tree-support`) and
`modules/core-modules/infill-linker/src/` (`com.core.infill-linker`) dominate
remaining per-layer wall on the four `benchy` cells and `base` supports-on —
and what exact, output-preserving (or scoped-exception) candidates clear the
standing 0.80 % gate with meaningful headroom toward `< 1.0x` and `<= 0.50x`?**

Concretely:

1. **Reconcile against [t177](177-post-t175-full-matrix-attribution.md) first:**
   confirm the post-t175 per-layer phase wall, per-module wall/fuel share, and
   remaining headroom for both `com.core.tree-support` (supports-on) and
   `com.core.infill-linker` (all 8 cells, especially `benchy` supports-off).
   Do not rely on pre-t172 fuel figures once t177 is resolved.
2. **Attribute `com.core.tree-support` (`modules/core-modules/tree-support/src/lib.rs`)
   post-t66/t160/t175:**
   - Remember prior screens so they are not re-run blind:
     [t64](64-tree-support-renderer-attribution.md) &
     [t66](66-paint-policy-dead-walk-reduction.md) already landed the paint-walk
     fix and left `skeleton_wall_count` / `point_in_expolygon` bbox scans in fog
     (~15.9 % of renderer fuel then);
     [t160](160-tree-support-prepass-batch-probe.md) exonerated per-layer
     placement (batch-folding was slower; the renderer's cost is its own work,
     null-renderer ceiling was 9.2–12.6 % of cell wall then).
   - Break down current `tree-support` fuel and wall on `benchy-classic-on` and
     `base-classic-on` into named inner kernels (e.g., skeleton/wall generation,
     `point_in_expolygon` / containment scans, infill/interface hatching,
     in-guest `polygon_ops`, marshal/decode) and price the top 2–3 exact
     candidates.
3. **Attribute `com.core.infill-linker` (`modules/core-modules/infill-linker/src/`)
   post-t44/t62/t65/t172:**
   - Remember prior screens: [t44](44-clip-universe-hoist.md) hoisted the clip
     universe, [t62](62-infill-linker-post-hoist-residual.md) measured the
     post-hoist residual, and [t65](65-infill-linker-batch-execute-experiment.md)
     tested batching.
   - Break down current `infill-linker` fuel and wall on `benchy-classic-off`,
     `benchy-arachne-off`, and `benchy-classic-on` (`connect.rs`, `graph.rs`,
     `offset.rs`, `orchestrate.rs`, `lib.rs`) and price the top exact
     candidates (e.g., clip-universe / spatial culling, graph/nearest-neighbor
     lookup, or redundant per-path work).
4. **Price route impact & decide graduations:** state each candidate's ceiling
   in ms, % of cell wall (ordinary & accelerated), whether it clears the
   applicable gate (8-cell `0.80 %` for `infill-linker`; 4-cell `t137`-class or
   8-cell for `tree-support`), and graduate only candidates with a verified,
   non-trivial wall ceiling.

## Scope

- Blocked by [177](177-post-t175-full-matrix-attribution.md) so the screen
  starts from verified post-t175 per-layer and module shares.
- Attribution and candidate pricing only; no production commit in this take,
  any diagnostic probe cleanly reverted and saved as `probe.patch`.

## Pre-declared gate

**Attribution + screen only (no landing gate).** Correctness bar:

- `cargo xtask build-guests --check` exits 0 before and after probe revert.
- Baseline attribution passes reproduce `t175`'s active `reference.json`
  byte-for-byte (`exit 0`, `degraded = false`, `0` code-1200).
- Every candidate's ceiling is priced in both deterministic guest fuel and
  implied wall share against `t177`'s phase/module split (never from `--profile`
  thread-time as wall).

## Acceptance

- Evidence directory `docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/`
  with `FINDINGS.md`, `REPRODUCE.md`, reduced JSONs, `probe.patch` (if used),
  and checkout-only `verify.py` (`verify.py` + `verify.py --self-test` exit 0).
- Updated `## Answer` in this ticket, `map.md` update, and graduated
  implementation ticket(s) for any candidate clearing the route bar.

## Answer

**Resolved 2026-11-21 (attribution + deterministic fuel ablation + unsandboxed screen; zero production edits landed; `probe.patch` cleanly reverted; `cargo xtask build-guests --check` and `--accelerated --check` both exit `0`; `130/130` runs byte-identical to `t175`'s active reference set; `verify.py --self-test` PASS `6/6` checks + `14/14` targeted negative controls).** Full evidence: [`evidence/t179-per-layer-guest-screen/FINDINGS.md`](../evidence/t179-per-layer-guest-screen/FINDINGS.md).

1. **`t177` reconciliation:** Post-`t175`, `com.core.tree-support` + `com.core.infill-linker` own **75.59–77.58 %** of all guest fuel (**9.63–13.48 %** implied cell wall) on the four supports-on cells (`tree-support` alone `60.82–70.28 %` fuel / `8.21–9.89 %` implied wall; `infill-linker` `6.64–16.11 %` fuel / `1.31–3.64 %` implied wall) and **30.68–54.67 %** of all guest fuel (**4.75–12.19 %** implied cell wall) on the four supports-off cells (where `tree-support` is `enable_support: false`-inert at `0.20–0.48 %` and `infill-linker` owns `30.39–54.24 %` fuel / `4.31–11.91 %` implied wall, plus the `134 / 104 ms` = `7.88 / 5.83 %` `L2` straggler tail on `benchy` supports-off).
2. **`com.core.tree-support` premise correction & attribution (`98.1–98.5 %` exact fuel reducible):**
   - **Premise correction:** `[t66](66-paint-policy-dead-walk-reduction.md)` was **DROPPED on 2026-10-14**, never landed. `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::paint_policy_for` still walks **723.5 M** (`benchy`) / **6.76 B** (`base`) ring edges across `0` painted annotations (`paint_has_ann = 0`, `paint_cover_hits = 0`), consuming **80.09 %** (`benchy`) / **80.27 %** (`base`) of `tree-support` fuel (`71.6–76.7 %` of `--profile` scope wall, attribution only).
   - `modules/core-modules/tree-support/src/lib.rs::skeleton_wall_count` + `point_in_expolygon` consumes **15.56 %** (`benchy`) / **16.30 %** (`base`) of fuel (`358.8 M` / `3.43 B` contour bbox rescans; **`99.21 %`** / **`99.06 %`** of skeleton points have `count <= best`).
   - Redundant in-guest `slicer_core::polygon_ops::union_ex` in `TreeSupport::run_support` consumes **2.85 %** (`benchy`, `1.19 B`) / **1.51 %** (`base`, `5.87 B`). The remaining host wall (`14.36 %` `benchy` / `9.10 %` `base` of `--profile` scope wall) is real `host:native` `offset_polygons` in `render_polygon_with_wall_count` (`2,001` / `6,345` calls). Classic ≡ Arachne counters on both models.
   - **Exact fuel ablations (byte-identical to `t175`):** **`TS-1`** (empty support-paint guard in `paint_policy_for` + precomputed bboxes & `count <= best` early exit in `skeleton_wall_count`) cuts fuel **−95.61 % (`22.8×`)** on `benchy` and **−96.55 % (`29.0×`)** on `base`; **`TS-2`** (`TS-1` + safe bypass of redundant `union_ex` when `regularized.is_some()` or pairwise bbox-disjoint) cuts fuel **−98.45 % (`64.7×`, `41.94 B → 0.65 B`)** on `benchy-classic-on` and **−98.06 % (`51.4×`, `388.79 B → 7.56 B`)** on `base-classic-on`.
3. **`com.core.infill-linker` attribution & `benchy` `L2` breakdown (`73.0–78.6 %` exact fuel reducible):**
   - `crates/slicer-core/src/polygon_ops.rs::PreparedPolylineClip::clip_batched` geometric re-attribution (`attributed_runs`, `16.15–31.97 %`) plus per-subject ambiguity repairs (`17.62–36.78 %`; `1,878–1,895` `benchy`, `3,820–4,002` `base`) consume **49.59–52.93 %** of all `infill-linker` fuel. On `benchy` Layer `2` alone — **34.08 %** (`classic`) / **32.51 %** (`arachne`) of the entire slice's `infill-linker` fuel — **35** ambiguity repairs take **70.83–71.25 %** (`2.49–2.58 B`) of `L2`.
   - `modules/core-modules/infill-linker/src/orchestrate.rs::link_paths_without_offset` + `point_with_metadata` takes **8.53–21.72 %** fuel (`union_groups == 0` in 8/8 so `source_segments` is 100 % dead; `3.22–30.26 M` linear metadata scans).
   - `modules/core-modules/infill-linker/src/graph.rs` + `connect.rs` (`project_on_ring`, `nearest_pair_candidates`, merge) take **10.80–16.24 %** fuel (`4.51–11.51 M` edge projections; `1.94–1.97 M` on `L2` alone = `13.32–13.54 %` of `L2`).
   - **Exact fuel ablations (byte-identical 8/8 to `t175`):** **`IL-1`** (native `source_id: u32` open-subject tagging in `vendor/clipper2-rust` `Clipper64::add_open_subject_tagged` + `execute_open_tagged` — exact because open paths never self-intersect or join in Clipper2) cuts fuel **−49.73…−53.16 % (`2.0×–2.1×`)**; **`IL-ALL`** (`IL-1` + `IL-2` dead `source_segments` / `O(1)` uniform metadata + `IL-3` precomputed bbox-pruned `project_on_ring` & `O(E)` sorted-ring `nearest_pair_candidates`) cuts fuel **−72.96 / −73.47 % (`3.70× / 3.77×`)** on `benchy` and **−78.60 / −78.28 % (`4.67× / 4.60×`)** on `base`.
   - **No standalone `SCHED-1` (`with_max_len(1)`) graduation:** `IL-ALL` collapses `L2` `infill-linker` (`186 → 42 ms` `benchy-classic-off`, `237 → 84 ms` `benchy-arachne-off`), eliminating the `L2` tail on `benchy-arachne-off` (`111 → 5 ms`), `benchy-classic-on` (`50 → 6 ms`), and `benchy-arachne-on` (`69 → 5 ms`), leaving only `31 ms` (`1.9 %`) on `benchy-classic-off` (`<= 0.3 %` on 7/8 cells; the remaining `~159 ms` inside `L2` is 3× host `region_partition`, already tracked in `map.md` `Not yet specified`).
4. **Unsandboxed screen (attribution only, not a landing gate) & combined graduation (`183`):**
   - **Standalone `TS-2` trap:** unmasks `L2`'s `infill-linker` tail on `benchy-classic-on` (`50 → 110 ms`, `c/w` `4.57× → 3.97×`, screen `−1.39 %` wall despite `+11.94 %` CPU) and is supports-off inert.
   - **Standalone `IL-ALL` trap:** structurally bounded at `146 ms / 27,920 ms = 0.52 % < 0.80 %` on `base-classic-on` (screen `−0.07 %` wall despite `+1.48 %` CPU) and masked behind `tree-support` on `benchy-classic-on` (screen `−0.40 %`).
   - **Combined `TS-2 + IL-ALL` (`arm_both` on supports-on, `arm_il` on supports-off) clears the literal standing ALL-8-CELL `0.80 %` gate with ZERO waivers and ZERO geometry exceptions:** cuts `per_layer` wall **`−44.56 / −34.57 / −44.34 / −41.14 %`** on supports-on and **`−29.09 / −23.05 / −6.98 / −12.25 %`** on supports-off; 3-round uninstrumented screen clears `0.80 %` in **8/8 cells at `2.64×–12.82×`** (`benchy`: **`+10.26 / +4.62 / +5.32 / +6.71 %`**, `base`: **`+2.50 / +6.20 / +2.11 / +6.58 %`**), **`24/24` wall wins** (`7/8` strict sep), **`24/24` CPU wins (`8/8` strict sep, `+3.33…+13.75 %`)**, **104/104** screen runs byte-identical to `t175`.
   - **`ESTIMATE ONLY` route (`t174 × t175 × t179`):** `benchy-classic-off` `1.144x → ~1.026x` ord / `~1.003x` acc (**81.6 %** of `< 1.0x` gap closed), `benchy-arachne-off` `1.155x → ~1.093x` / `~1.007x`, `base-classic-on` `1.221x → ~1.145x` (`~0.970x` stacked with `t182`, `~0.875x` with `t182 + t181`), `base-arachne-on` `1.201x → ~1.122x` (`~0.953x` with `t182`, `~0.869x` with `t182 + t181`), `benchy-classic-on` `1.570x → ~1.498x` (`1.404x` `t177`-implied; `~1.351x` with `t182 + t181`), `benchy-arachne-on` `1.389x → ~1.295x` (`~1.169x` with `t182 + t181`).
   - **Graduated:** [Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](183-per-layer-tree-support-and-infill-linker-exact-acceleration.md) (AFK, unblocked).
