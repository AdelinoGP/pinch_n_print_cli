# `PrePass::SupportAnalysis` attribution and speedup screen

Type: task / attribution + mechanism screen (no production code edit)
Label: wayfinder:task
Status: resolved (2026-11-23; observation-only probe 16/16 + host-only exact screen 52/52 byte-identical to t182/reference.json; zero production edits; verify.py --self-test PASS 13/13; graduates 191)
Assignee: 2026-11-23 Antigravity wayfinder session — resolved (zero production edits).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t185 full-matrix attribution and 8/8 closure headroom map](186-post-t185-full-matrix-attribution.md)

## Question

**Why does `PrePass::SupportAnalysis` (`292 / 290 ms` = `10.4 / 9.6 %` of `benchy` supports-on, `2,853 / 2,892 ms` = `14.5 / 14.4 %` of `base` supports-on, `12.0–18.1×` floor) run at only `2.95–3.00×` concurrency on `benchy` (vs `6.47–7.44×` on `base`), how much is serial `collect_print_plane_occupancy` (`~85 ms` `benchy` / `~745 ms` `base` single-threaded `slice_mesh_ex` spotted in `t181`) vs `host:support_analysis` `offset` (`84 %` of built-in thread-time: `0.61 s` `benchy`, `15.54–18.11 s` `base`), and what exact (zero-drift vs `t182/reference.json`) bundle clears the standing `t137` 4-cell planner gate?**

## Scope

- Substage attribution inside `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` (`model_occupancy` assembly, `collect_print_plane_occupancy`, overhang/contact detection `offset` + `clip_polygons`, termination surfaces, modifier-region handling).
- Screen exact mechanisms: parallelizing `collect_print_plane_occupancy` (or sharing/reusing mesh slicing), improving layer/region parallelism in `detect_overhangs`, and eliminating redundant/unnecessary `offset` calls (`401` `benchy` / `1,231` `base`) with `100 %` bit-identical IR vs `t182/reference.json`.
- Zero production code edits landed in this ticket; graduate an implementation & standing-gate ticket for the winning exact bundle.

## Pre-declared gate

- Attribution & screen only (no production code edits); any temporary probe patch reverted before resolution, `cargo xtask build-guests --check` exit 0, and `100 %` byte-identical to `t182/reference.json`.
- Supports-off is structurally inert (`0 ms` when `enable_support: false`), so any graduated implementation candidate qualifies for `t137`'s standing 4-cell planner gate (`>= 0.80 %` paired wall saving with CPU corroborating on all 4 supports-on cells).

## Acceptance

- Reduced evidence under `docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/` with `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`--self-test` exit 0).
- Updated `## Answer`, `map.md`, and graduated implementation ticket (if a priced exact mechanism clears `t137`).

## Answer

**Resolved 2026-11-23 (attribution + exact mechanism screen; zero production edits; `probe.patch` cleanly reverted; `cargo xtask build-guests --check` and `--accelerated --check` both exit `0`; `16/16` probe + `52/52` screen runs byte-identical to `t182/reference.json`; `verify.py --self-test` PASS `13/13`).**

1. **Root cause of low concurrency (`2.95–3.00×` `benchy`, `6.5–7.4×` `base`) and `offset` dominance inside `PrePass::SupportAnalysis` (`348.53 ms` `benchy` / `3,429.38 ms` `base` probed; classic ≡ arachne 100 % op/vertex-identical):**
   - **Half of stage wall is 100 % single-threaded (`50.4 %` `benchy`, `43.6 %` `base`):**
     - `collect_print_plane_occupancy`: **`122.68 ms` (`35.2 %` `benchy`) / `1,088.96 ms` (`31.8 %` `base`)** inside single-threaded `slicer_core::slice_mesh_ex` (`slice_make_lines` `23.78 / 427.60 ms` + per-layer `chain_lines_to_expolygons` `93.30 / 658.82 ms`, max layer only `3.30 / 3.56 ms`).
     - `object_layer_union`: **`48.14 ms` (`13.8 %` `benchy`) / `389.20 ms` (`11.4 %` `base`)** running `union_ex` serially over `240 / 496` layers.
   - **Inside parallel `contacts_par` (`182.65 ms` `benchy`, `1,905.70 ms` `base`):**
     - On `benchy`, ascending `(L1, L2, …)` order puts the two slowest layers (`L1 = 70.59 ms`, `L2 = 94.43 ms`) into the **same first Rayon chunk** (`165.0 ms` serial on one thread vs `p50 = 2.07 ms`).
     - `cantilever_offset` (`offset(lower, +3.0 mm, Square)` inside `detect_support_contacts_with_annotations`) burns **`9,948.55 ms` (`45.0 %` of contact thread-time on `base`)** and **`262.76 ms` (`25.7 %`, incl. `53.67 ms` on `L2` `benchy`)** solely for a boolean empty check even though only `2 / 81` (`benchy`) and `11 / 368` (`base`) layers are cantilevers.
     - Unconditional `intersection_ex(&geometry, enforcers)` / `intersection_ex(polygons, blockers)` on empty slices (`&[]`) wastes **`25.92 ms` (`162` calls `benchy`) / `482.88 ms` (`736` calls `base`)**.
     - `grown_offset` (`offset(lower, +δ ≈ 0.33 mm, Square)`) accounts for the remaining `525.86 ms` (`51.4 %`) / `9,206.71 ms` (`41.7 %`) on `99.5 %` outer-contour vertices (`base`); hole-drop and overhang-window mechanisms (`G_holes`, `G_win`, `G_island_win`, `G_cert`) were tested and **falsified/rejected** (no win + Clipper2 ordering drift on `2` `benchy` / `41–121` `base` layers).
2. **Exact (`0`-drift) winning bundle `SA-1 + SA-2 + SA-3 + SA-4 + SA-5 + SA-6` clears `t137` at `8.39×–14.15×` floor (`12/12` wall wins, `4/4` CPU corroborating, `52/52` byte-identical to `t182/reference.json`, supports-off structurally inert `2/2`, strict monotonic ablation):**
   - `benchy-classic-on`: **`+6.72 %`** wall (`3.3239 → 3.1007 s`, `8.39×`), CPU **`+4.88 %`**, stage `328 → 97 ms` (`−70.4 %`, `3.38×`), `host:support_analysis` `1,366 → 1,046` calls (`−22.8 %` thread-time).
   - `benchy-arachne-on`: **`+7.94 %`** wall (`3.4424 → 3.1690 s`, `9.93×`), CPU **`+3.72 %`**, stage `293 → 113 ms` (`−61.4 %`, `2.59×`), `host:support_analysis` `1,366 → 1,046` calls (`−22.9 %`).
   - `base-classic-on`: **`+11.32 %`** wall (`25.9686 → 23.0296 s`, `14.15×`), CPU **`+7.07 %`**, stage `3,069 → 1,266 ms` (`−58.8 %`, `2.42×`), `host:support_analysis` `4,430 → 2,990` calls (`−39.7 %`, `22.89 → 13.80 s`).
   - `base-arachne-on`: **`+10.57 %`** wall (`26.5024 → 23.7002 s`, `13.22×`), CPU **`+4.75 %`**, stage `3,382 → 1,149 ms` (`−66.0 %`, `2.94×`), `host:support_analysis` `4,430 → 2,990` calls (`−43.4 %`, `22.35 → 12.65 s`).
   - Cross-window `ESTIMATE ONLY` route: stacked with `t190` (`t187`), `benchy-classic-on` drops to **`~1.0258x` ord** (`~1.0172x` stage-delta, only `2.51 %` left for `t189` `PostPass` at `14.4 %`), `benchy-arachne-on` **`~0.9074x`**, `base-on` **`~0.7183x / ~0.6635x`**.
3. **Graduated:** [Exact `PrePass::SupportAnalysis` acceleration: parallel print-plane slicing, exact cantilever distance certificate, parallel layer union, and LPT overlapped scheduling](191-support-analysis-exact-acceleration.md). Evidence: [`evidence/t188-support-analysis-screen/FINDINGS.md`](../evidence/t188-support-analysis-screen/FINDINGS.md).
