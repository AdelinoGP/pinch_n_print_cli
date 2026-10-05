# emit_walls attribution — premise falsified; classic-perimeter re-attribution

Captured 2026-09-22 at HEAD `bde9b1ba` (working tree clean before and after;
all probe code removed). Fixture: `tmp/3dbenchy.stl`, supports-off,
`tmp/perf-flags/quiet-validation/benchy-supports-off-classic.json` (explicit
`wall_generator: classic`), `--module-dir modules/core-modules`, 12 Rayon
workers, release `pnp_cli`. Guest freshness exit 0 before every capture run;
`module diagnose` reports 24/24 modules provenance `external` (the handoff's
"23" has drifted — a module has been added since; the invariant that all core
modules load externally holds).

This session executed the grilling decision Q13: the emit_walls clone/store
premise was measured first and **falsified**, so the session became a full
re-attribution of `com.core.classic-perimeters` and a re-ranked lead list.
No production code was changed and nothing was committed.

## Method

Three captures, all benchy supports-off classic:

1. Baseline fuel: `slice --profile --profile-verbose 2> fuel-profile.jsonl`.
2. Probe fuel: same, with temporary user scopes (ADR-0055
   `slicer_sdk::profile::register_scope` + `slicer_core::profile::scope`) in
   `emit_walls` and `build_ring_wall`
   (`modules/core-modules/classic-perimeters/src/lib.rs`), every block tagged
   `// [PERF-EMIT-PROBE]`.
3. Stage confirmation: `slice --instrument-stderr 2> stage-events.jsonl`.

Fuel is exact: the profile states "fuel = executed wasm instructions
(deterministic; mark overhead costs no fuel)", so probe shares are
undistorted. Scope entries are **exclusive** (children deducted from parents;
entries sum to the module total). Raw captures and the three gcode outputs
are kept next to this file.

Probe discipline (handoff §11.6): one grep-able marker, mechanical removal,
`git status --porcelain` empty and grep-confirmation of zero
`PERF-EMIT-PROBE` / guard-name remnants after removal, guests rebuilt.

Harness: `tmp/perf-flags/quiet-validation/run-warmups.ps1` was the last
unrepaired script from the §13 label-audit failure (it used the generator-less
`benchy-supports-off.json` and passed no `-ExpectedGenerator`). Fixed to
per-generator configs with validation. `tmp/alloc-bench/regression-check-generator-validation.ps1`
passes (11 cases, including the exact audit failure: Arachne label with
Classic config rejected).

## Verdict on the candidate (Q13 premise)

`emit_walls`' `all_wall_polygons.push((i, inset_result.clone()))` — the inset
clone/store the candidate targeted — measures **14,341,212 fuel = 0.0014% of
the module** across its 694 calls. The premise "inset clones/stores are a
meaningful cost" is **falsified**. The handoff §13 lead 1 (retain only the
first inset for seam generation) has no time prize; it remains only a
memory-shape idea, and peak RSS is unmeasurable today (DEV-026).

## The split (guest fuel; percentages of total 1,133.9 B)

`com.core.classic-perimeters` = **89.8%** of all guest fuel
(1,017,700,604,576; 240 dispatches). Next module, `com.core.infill-linker`,
is 9.8%; no other module exceeds 0.1%.

| scope (all in classic-perimeters) | fuel | share | calls |
| --- | ---: | ---: | ---: |
| `cp::wall_build::path3d` (`expolygon_to_path3d_indexed`) | 791,560,629,195 | **69.8%** | 2,107 |
| `polygon_ops::offset2_ex` | 172,122,933,344 | 15.2% | 960 |
| `cp::wall_build::bridge_scan` (`PerimeterSpatialContext::is_bridge` loop) | 47,848,410,445 | 4.2% | 2,107 |
| `cp::emit_walls::seam` | 208,252,722 | 0.02% | 240 |
| `cp::emit_walls::gap_collect` | 186,578,171 | 0.02% | 934 |
| `cp::emit_walls::thinwall` | 142,964,640 | 0.01% | 240 |
| `cp::wall_build::flags` (`build_wall_flags`) | 84,368,041 | 0.01% | 2,107 |
| `cp::emit_walls::inset_host` (host `offset_polygons` marshalling) | 45,937,735 | 0.004% | 720 |
| `cp::emit_walls::store` (the clone) | 14,341,212 | 0.001% | 694 |
| `cp::emit_walls::gapfill_emit` | 11,549,667 | 0.001% | 240 |
| `cp::emit_walls::wall_build` residual (WallLoop construction/push) | 7,457,064 | 0.001% | 694 |
| `cp::emit_walls::emit_reorder` | 1,769,959 | 0.0002% | 240 |
| `<module self>` (region prep, dispatch) | 5,465,412,381 | 0.5% | — |

The baseline run (no probes) reproduces the totals to 7 significant figures
(`offset2_ex` 172,122,878,513; `<module self>` split confirmed
`polygon_ops::offset2_ex` 15.2% + self 74.6% before drilling). `build_wall_flags`
being negligible confirms the accepted empty-annotation fast path
(`has_effective_annotation` early return, `crates/slicer-core/src/perimeter_utils.rs`)
works on this workload.

`polygon_ops::offset2_ex`'s 960 calls split across two call sites in
`ClassicPerimeters` (`modules/core-modules/classic-perimeters/src/lib.rs`):
720 from the `only_one_wall_top` split path — the `min_width_top` shrink/expand
(`offset2_ex(&kept, -min_width_top, min_width_top + 0.85 * inner_wall_line_width, ..)`)
after `split_top_surfaces` — and 240 from the gap-fill width band's
`opened_max` in the gap-fill emission block.

## The hotspot mechanics — why path3d is 69.8%

`expolygon_to_path3d_indexed` (`crates/slicer-core/src/perimeter_spatial.rs`)
runs **per wall-ring vertex**: `PerimeterSpatialContext::overhang_quartile`
and `PerimeterSpatialContext::signed_distance_to_boundary`. In ordinary
builds `default_dispatch()` returns `indexed: false` — the reserved
`pnp_perimeter_spatial_accelerated` cfg (declared by `crates/slicer-core/build.rs`,
injected only by the controlled rustc driver, `xtask/src/rustc_driver.rs`) is
absent — so **every one of those queries is the legacy linear scan** over
boundary edges / overhang bands. Per ring that is O(vertices × edges); 2,107
rings at ~376 M instructions each. `bridge_scan` is the same story through
`PerimeterSpatialContext::is_bridge` → `point_in_any_polygon`
(`crates/slicer-core/src/perimeter_spatial.rs`).

This is packet 254's (`docs/spec_packets/254-exact-perimeter-spatial-queries`)
ordinary/accelerated duality: ordinary builds are deliberately the "reference
route for exactness" (`docs/23_controlled_perimeter_builds.md`), and the
indexed path exists behind the controlled build's acceptance campaign
(`resources/perimeter-acceptance/run-acceptance.ps1`). The measured 74% of
guest fuel is exactly what that campaign's accelerated mode targets.

Caveat for planning: guest fuel is not wall. The path3d work is pure guest
computation (fuel is a good proxy), but any lead's prize must still be
confirmed under the Q3/Q5 acceptance gate.

## Stage confirmation (instrumented run; attribution only)

Phase walls: prepass 7,893 ms, per_layer 15,914 ms, postpass 474 ms
(supports-off benchy — prepass is small without tree support).
`com.core.classic-perimeters` accumulated worker elapsed **114,026 ms over
240 calls** (§3.3: accumulated, not wall/CPU) vs `com.core.infill-linker`
12,150 ms — consistent with the fuel picture.

Anomaly, recorded not explained: the profile's native built-in wall table
shows `host:slice` at 25.9 s with `polygon_ops::closing_ex` 22.3 s over 240
calls, but `module_complete` for `host:slice` reports 3,152 ms. The two
cannot both be wall. Most likely the span fold sums per-thread spans
(accumulated worker wall, the §3.3 trap) or buckets marks differently; the
spans were stable across three runs (22.7 / 23.5 / 22.3 s). **Verify
`fold_marks` thread handling before treating this as a hotspot.** Repro
captures: `fuel-profile.jsonl`, `probe-fuel-profile.jsonl`,
`drill-fuel-profile.jsonl` (scratch; not retained).
**Resolved 2026-09-23:** the first hypothesis is correct — accumulated
per-thread spans vs wall, `fold_marks` is thread-correct — so this retires as
a hotspot. See `../../issues/24-host-slice-closing-span-contradiction.md` and
`../t24-span-contradiction/SAME-RUN.md`.

## Re-ranked leads (user decision required; nothing implemented)

1. **Accelerated perimeter-spatial adoption (74% of guest fuel addressed).**
   Not an ad-hoc optimization: run/extend the packet-254 acceptance campaign
   (`resources/perimeter-acceptance/run-acceptance.ps1`,
   `docs/23_controlled_perimeter_builds.md`) and quantify ordinary-vs-
   accelerated wall/CPU on our fixtures with the Q3/Q5 gate. The indexed path
   (`IndexState::Indexed` r-tree queries) targets the measured hotspot
   exactly. Note `build_index` keeps small record sets on the linear path by
   design (`records.len() <= RSTAR_MAX_SIZE`), which is fine — small sets are
   cheap.
2. **`offset2_ex` in the `only_one_wall_top` `min_width_top` shrink/expand**
   (15.2% of guest fuel; 720 of 960 calls). Look at call-count reduction
   (skip when `top_portion`/`kept` is empty or below the bbox gate before
   expanding) in `modules/core-modules/classic-perimeters/src/lib.rs`.
   Fuel≠wall: confirm under the gate before keeping.
3. **Per-vertex annotation work in `expolygon_to_path3d_indexed`**
   (subsumed by lead 1 in ordinary mode, complementary in accelerated mode):
   `overhang_quartile` + `signed_distance_to_boundary` are computed for every
   wall vertex regardless of whether any consumer reads
   `overhang_quartile` / `overhang_distance_mm` on `Point3WithWidth`. A
   consume-only-when-read (or per-ring) path could cut work in both modes.
   Verify all consumers first (classic, arachne, downstream modules).
4. **`bridge_scan` (4.2%)** — same index story as lead 1 via `bridge_index`;
   folded into lead 1's campaign unless it survives accelerated mode.
5. **`host:slice` `closing_ex` measurement question** (claimed 22.3 s
   accumulated vs 3.2 s module wall): resolve the contradiction before any
   decision. If the work is real and serial, it is large; if it is
   accumulated worker wall, it shrinks accordingly.
6. **CLOSED — emit_walls inset clone/store retention** (handoff §13 lead 1):
   measured 0.0014% of module fuel. Do not pursue for time; memory-only
   question, unmeasurable today (DEV-026).

Superseded context: §12's hotspot shares predate the wall-flags fast path;
the tables above are measured at HEAD `bde9b1ba` with the fast path present.

## Addendum 2026-09-22 — mode-comparative capture (accelerated build)

Fairness challenge raised after the first write-up: all captures above ran
with `pnp_perimeter_spatial_accelerated` disabled (ordinary mode). Verdict by
use: the falsification is mode-independent; "where does today's build spend
its time" is fair (ordinary is the production default and every prior
handoff baseline is ordinary-mode); but the **shares below lead 1 were
ordinary-only**. This addendum closes that gap with the accelerated pair.

Recipe (hard-won; record it): the bare accelerated artifacts dir
(`target/guests-accelerated/artifacts/modules/core-modules`) stages wasm
**without manifests** — `module diagnose` there silently loads the 18
*integrated* modules (the §3.4 trap). Dual `--module-dir modules/core-modules
--module-dir target/guests-accelerated/...` does **not** shadow: first dir
wins (proven by identical calicat fuel totals). The correct route is
`cargo xtask dist --accelerated` → `target/dist-accelerated/developer/`
(complete pnp_cli + 24-module layout, host and guests both through the
controlled driver).

**Fuel, benchy supports-off classic, ordinary vs accelerated:**

| | ordinary | accelerated | delta |
| --- | ---: | ---: | ---: |
| total guest fuel | 1,133,915,941,322 | 727,636,734,962 | **−35.8%** |
| `com.core.classic-perimeters` | 1,017,699,458,919 | 611,420,304,599 | −39.9% |
| `polygon_ops::offset2_ex` | 172,122,878,513 | 172,123,017,590 | identical |
| classic `<module self>` | 845,576,580,406 | 439,297,287,009 | −48.0% |
| `com.core.infill-linker` | 111,323,907,737 | 111,323,907,707 | identical |

Everything outside classic-perimeters is bit-stable across modes, which
confirms the accelerated cfg changes exactly the perimeter-spatial query
paths. Critically: the hot work shrank by only **~1.9×** (path3d+bridge_scan
implied: ~839 B → ~433 B by self-delta), not the O(n²)→O(log n) one might
assume. Suspects for the residual, to verify before promising a prize: tree
queries still expensive per vertex in wasm (u128 envelope math), and/or
fallbacks firing in accelerated mode (`bridge_arithmetic_safe`'s
unsafe-extents fallback, `IndexState::Linear` for small record sets,
`index_failed` catch_unwind path in `crates/slicer-core/src/perimeter_spatial.rs`).

Coarse output parity (one pair, §3.6 limits apply): 4,294,185 vs 4,294,012
bytes (173 B on 4.29 MB — inside the same-binary noise band); TYPE section
set identical, `Outer wall` 240/240, `Inner wall` 247/246 (within the known
244–251 instability). No wholesale geometry change detectable; the
packet-254 exactness gate remains the real proof.

Wall, one run each under `--profile` (indicative only): slice_complete
27,921 ms ordinary vs 18,971 ms accelerated.

**Effect on the ranked list (accelerated-mode shares):** classic
`<module self>` 60.4%, `polygon_ops::offset2_ex` **23.7%**, `com.core.infill-linker`
**15.3%**. Revised reading: lead 1 (accelerated adoption) is confirmed
necessary and delivers a large but partial win; it is likely **not
sufficient** — the per-vertex annotation work persists at ~433 B, so lead 3
(consume-only-when-read annotations / fewer queries per vertex) gains weight
and should be scoped against accelerated mode, not ordinary. Lead 2
(`min_width_top` `offset2_ex`) rises in relative terms and is mode-independent
(calicat: `offset2_ex` is 53.7% of classic fuel). `infill-linker`'s
111.1 B self — untouched by acceleration and now the #2 module — is a new
entry for the list.
