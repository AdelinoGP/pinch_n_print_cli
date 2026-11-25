# Post-t185 full-matrix attribution and 8/8 closure headroom map

Type: task / attribution + headroom pricing (no production code edit)
Label: wayfinder:task
Status: resolved (2026-11-23; full 8-cell post-t185 phase/stage/host-op/guest-fuel attribution + 8/8 and <=0.50x headroom map verified 32/32 byte-identical; graduates 187, 188, and 189)
Assignee: 2026-11-23 Antigravity wayfinder session — resolved (zero production edits).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t185](185-support-geometry-post-planner-host-tail-acceleration.md)
(with [t182](182-host-spatial-hierarchical-phase3-union.md),
[t183](183-per-layer-tree-support-and-infill-linker-exact-acceleration.md), and
[t184](184-shell-classification-exact-acceleration.md)), because the last
full-matrix phase, stage, host-op, and guest-fuel attribution
([t177](177-post-t175-full-matrix-attribution.md)) predates all four landed
implementation tracks — which together cut `benchy` supports-on by ~30 %+ and
moved 4 more cells across `< 1.0x` on the cross-window estimate (`6/8` estimated
`< 1.0x`), leaving only `benchy-classic-on` (`~1.204x` est., ~17 % cut) and
`benchy-arachne-on` (`~1.085x` est., ~8 % cut) above parity.

## Question

**At post-t185 HEAD (`748efbf5` / `92b62d9a`), what is the exact phase split,
prepass/postpass stage split, host-clipper per-op census, and per-module
guest-fuel breakdown across all eight cells — what exact levers close the last
two open cells (`benchy-classic-on` at `~1.204x` est. and `benchy-arachne-on`
at `~1.085x` est.) while clearing the standing gates on `base`, and what is the
updated headroom map toward both `8/8 < 1.0x` and the follow-on `<= 0.50x`
(2× faster) map?**

Concretely:

1. **Re-derive the 8-cell phase & stage split** (median of 3 unsandboxed
   instrumented passes per cell via `--instrument-stderr`, identical bracket
   definitions to [t177](177-post-t175-full-matrix-attribution.md): `prepass`,
   `per_layer`, `postpass`, `outside`, plus every sequential prepass and
   postpass stage bracket, per-layer module thread sums, straggler tail, and
   `PrePass::SupportGeometry` post-planner host tail). Every pass must
   reproduce `t182`'s active currency
   ([`evidence/t182-spatial-hierarchical-phase3-union/reference.json`](../evidence/t182-spatial-hierarchical-phase3-union/reference.json))
   byte-for-byte with `degraded = false` and `0` code-1200 diagnostics.
2. **Re-derive the 8-cell `--profile` per-op and guest-fuel census** (1
   unsandboxed pass per cell; thread-time / fuel attribution only, never
   denominated as wall):
   - Host clipper breakdown (`clip_polygons`, `offset`, `offset2_ex`,
     `closing_ex`, `opening`) by owning host built-in (`host:native`,
     `host:shell_classification`, `host:support_analysis`, `host:slice`,
     `host:mesh_analysis`, `host:overhang_annotation`).
   - Deterministic guest WASM fuel per module and per-layer implied wall share.
3. **Price the 2 remaining open cells (`benchy-classic-on`, `benchy-arachne-on`)
   against `< 1.0x` AND all 8 cells against `<= 0.50x`:**
   - Using `t185`'s `reduced/route.json` (`ESTIMATE ONLY` chain `t174 × t175 ×
     t182 × t183 × t184 × t185`, never a ratio of record), rank the single-stage
     and multi-stage ceilings and check whether each reserve lever (`SupportAnalysis`,
     remaining `SupportGeometry` / `tree-support-planner`, `PostPass`, `Slice`,
     remaining `per-layer` / `ShellClassification`) also clears the standing
     `0.80 %` gate floor on `base`.
4. **Graduate concrete, falsifiable next-step screen/implementation tickets** to
   close `8/8 < 1.0x` before running the single-HEAD `n >= 4` closing Orca
   scoreboard.

## Scope

- Ordinary arm at HEAD (`cargo xtask build-guests --check` exit 0 before
  running); `RAYON_NUM_THREADS=12`, standing matched-pair configs, unsandboxed
  on Windows (per `t177`'s ~2.8× sandbox distortion finding).
- Zero production code changes, zero reference moves, no candidate A/B, and no
  OrcaSlicer scoreboard re-run (cross-window route arithmetic is explicitly
  labelled `ESTIMATE ONLY`).

## Pre-declared gate

**Attribution-only (no performance gate).** Correctness & method bar:

- `cargo xtask build-guests --check` exits 0; exe and module-tree SHA-256
  stamped.
- All 32 attribution runs (`24` instrumented + `8` `--profile`) exit 0,
  `degraded = false`, `0` code-1200, and reproduce `t182`'s active
  `reference.json` **8/8** byte-for-byte.
- Phase brackets reconcile with `slice_complete` and stage brackets reconcile
  with the prepass/postpass brackets in all 8 cells.
- `--profile` numbers are denominated strictly as accumulated thread-time /
  deterministic guest fuel, never wall claims.

## Acceptance

- Reduced evidence under
  `docs/specs/perf-vs-orca/evidence/t186-post-t185-attribution/` with
  `FINDINGS.md`, `REPRODUCE.md`, machine-readable reduced JSONs, and a
  checkout-only `verify.py` (`verify.py` + `verify.py --self-test` exit 0).
- Updated `## Answer` in this ticket, `map.md` Destination status +
  Decisions-so-far + graduated frontier tickets for the final push to 8/8.

## Answer

**Resolved 2026-11-23 (attribution + headroom map only; zero production code edits; graduates [t187](187-tree-support-planner-post-t182-residual-screen.md), [t188](188-support-analysis-attribution-and-speedup-screen.md), and [t189](189-postpass-attribution-and-speedup-screen.md)).**

Evidence: [`evidence/t186-post-t185-attribution/FINDINGS.md`](../evidence/t186-post-t185-attribution/FINDINGS.md) (`verify.py --self-test` PASS 5/5 checks + **12/12** negative controls rejected; `cargo xtask build-guests --check` exit `0`; `target/release/pnp_cli.exe` sha256 `e661baa5…`, 24 external core modules tree sha256 `8d33516c…`; **24/24** instrumented + **8/8** `--profile` = **32/32** clean runs byte-identical to `t182`'s active `reference.json`).

### Key results

1. **Phase split (median of 3 unsandboxed instrumented passes/cell):**
   - `benchy-classic-on` (`2,801 ms`, −18.9 % vs `t177`, est. `1.204x`, needs `16.9 %` cut): **prepass 64.2 %**, **per-layer 18.7 %**, **postpass 14.4 %**, **outside 2.6 %**.
   - `benchy-arachne-on` (`3,022 ms`, −14.9 % vs `t177`, est. `1.085x`, needs `7.8 %` cut): **prepass 60.9 %**, **per-layer 21.9 %**, **postpass 15.1 %**, **outside 2.6 %**.
   - `base-classic-on` / `base-arachne-on` (`19,737 / 20,073 ms`, −33.2 / −30.5 % vs `t177`, est. `0.843x / 0.833x`): **prepass 76.8 / 73.4 %**, **per-layer 12.6 / 15.4 %**, **postpass 10.4 / 10.8 %**, **outside 0.4 %**.
   - Supports-off (`1,317 / 1,502 ms` `benchy`, `9,436 / 9,322 ms` `base`, all 4 est. `< 1.0x`): prepass `48.5–68.5 %`, per-layer `22.6–35.2 %`, postpass `7.6–10.5 %`.
2. **Host Clipper & deterministic guest fuel (`--profile`, 1 pass/cell):**
   - Guest fuel collapsed **−69.9…−74.0 %** on supports-on (`68.74 → 20.72 B` `benchy-classic-on`, `563.92 → 156.91 B` `base-classic-on`) and **−24.1…−40.4 %** on supports-off. `com.core.tree-support-planner` is now #2 in supports-on fuel (`28.2–28.9 %` `benchy`, `31.0–33.7 %` `base`).
   - Host Clipper thread-time dropped **−38.4 %** on `base-classic-on` (`140.34 → 86.46 s`), flipping `base` supports-on back to **`offset`-dominant** (`51.1–51.9 %` offset vs `38.4–39.6 %` clip), while `benchy-on` is nearly split (`47.9–48.2 %` clip vs `41.4–41.9 %` offset).
3. **Graduated frontier to close 8/8 `< 1.0x`:**
   - **[t187](187-tree-support-planner-post-t182-residual-screen.md):** `PrePass::SupportGeometry` remainder (`com.core.tree-support-planner` = `785/791 ms` = `28.0/26.2 %` `benchy-on`, `6,296/5,884 ms` = `31.9/29.3 %` `base-on`; sole single stage able to close `benchy-classic-on` alone at `→ 0.849x`; `t137` 4-cell gate).
   - **[t188](188-support-analysis-attribution-and-speedup-screen.md):** `PrePass::SupportAnalysis` (`292/290 ms` = `10.4/9.6 %` `benchy-on` at only `2.95–3.00×` concurrency; `2,853/2,892 ms` = `14.5/14.4 %` `base-on`; serial `collect_print_plane_occupancy` + `84 %` `offset`; `t137` 4-cell gate).
   - **[t189](189-postpass-attribution-and-speedup-screen.md):** `PostPass` (`LayerFinalization` + `GCodeSerialize` + `GCodePostProcess` = `14.4/15.1 %` `benchy-on`, `10.4/10.8 %` `base-on`, `7.6–10.5 %` supports-off; all-8-cell gate, widening `benchy-off`'s `< 1.0x` safety margin simultaneously).

