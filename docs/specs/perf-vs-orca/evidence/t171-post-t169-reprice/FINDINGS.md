# t171 — Post-t169 re-price: the split, the per-op census, and the re-aim

Route: [Re-price the prepass work-set, the phase split and S5/S6 after t169](../../issues/171-post-t169-prepass-reprice.md).
Take: 2026-11-20, ordinary arm at HEAD `d72b07f3`, `target/release/pnp_cli.exe`
sha256 `b2e8261e…` (byte-identical to t169's candidate build — the tree has not
moved since it landed), 24 external core modules, `RAYON_NUM_THREADS=12`.
**No production edit, no candidate, no reference move.**

Attribution only. Every share below comes from `--instrument-stderr`; every
per-op total from `--profile`. Neither instrument measures wall, and no number
here is a wall claim or a ratio of record.

## Verdict

**The prepass is still the route, but the lever inside it changed: the
supports-off cluster is offset-bound and the supports-on cluster is
boolean-bound.** Four findings:

1. **The phase split moved as t169 predicted, on all eight cells.** Prepass
   **55.9–83.7 %** (was 67.0–87.2 % at t164), per-layer **12.0–34.7 %** (was
   9.3–26.3 %), postpass **4.0–8.3 %**, outside **0.2–3.3 %**. The prepass share
   fell **3.5–12.3 pp** and the per-layer share rose **2.7–9.9 pp** everywhere —
   the decimation cut the phase that owned the offset work.
2. **Each base cell has a single sufficient stage; no benchy cell does.** Zeroing
   `ShellClassification` alone leaves base classic-off at **0.58x** and base
   arachne-off at **0.64x** (required cuts 3.9 % / 10.3 % — only **8.9 % / 24.2 %**
   of that stage); zeroing `SupportGeometry` alone leaves base classic-on at
   **0.95x** and base arachne-on at **0.92x**, but that is **96.0 % / 92.4 %** of
   the stage. On the four benchy cells the largest stage is **not** sufficient:
   its own share is **114.5 % / 143.1 % / 140.7 % / 150.3 %** of what the cell
   needs (`ShellClassification` 34.1 % / 32.2 % → 1.08x / 1.24x;
   `SupportGeometry` 40.5 % / 38.9 % → 1.41x / 1.47x). Benchy needs combinations,
   and the two supports-on benchy cells need their whole top three.
3. **The surviving clipper mass is a different op on each cluster.** Host
   clipper thread-time is **offset-dominated on the supports-off cells**
   (`offset` 42.9–51.2 %, `clip_polygons` 29.5–34.4 %) and **boolean-dominated
   on the supports-on cells** (`clip_polygons` 53.1–60.9 %, `offset`
   34.1–39.4 %, `offset2_ex` 4.2–6.0 %). This is t170's bound seen from the
   other side: offsets cannot close six cells because on four of them the
   majority of the remaining clipper work is not an offset.
4. **S5 and S6 were already landed, and neither has a residual worth a
   ticket.** Both are t132/t133 keeps. Rescaled to the post-t169 primitive, S5's
   *landed* win is **9.01 %** of base classic-off's wall and its *residual*
   mechanism is **0.83 %** — at the 0.80 % floor; S6's landed win is **5.86 %**
   of base classic-on and its residual **5.89 %**, and S6 is inert on the four
   supports-off cells by construction. Neither residual is a route term.

**Re-aim:** the supports-off cluster's next step is t172 (`OffsetConcaveCrossing`,
priced to win both base supports-off cells). The supports-on cluster's next step
is a **boolean-volume program in the support path** — `SupportGeometry` must shed
**92.4–96.0 %** of itself, and 60.8–60.9 % of the host clipper thread-time on
those cells is `clip_polygons`. Charted as ticket 173.

## 0. Two premise corrections

Both are stale premises in the ticket, not in the map:

- **`PrePass::InternalBridges` does not exist.** There is no stage bracket of
  that name on any cell, and there was none at t164 either. The internal-bridge
  work runs inside `PrePass::ShellClassification`:
  `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`)
  precomputes `compute_internal_bridge_layer` per qualified layer and commits it
  in the same stage. So "is `PrePass::InternalBridges` still worth anything" has
  no stage to price; the mechanism is priced in §5 from t131's census instead,
  and its whole contribution is now below the floor.
- **S5 and S6 are landed, not candidates.** `inflate_model_occupancy_memoized`
  (`modules/core-modules/tree-support-planner/src/lib.rs`) and
  `internal_bridge_offset_window` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`)
  are in the production path. What can be re-priced is what each is worth
  *today* (rescales), and what each still costs *after* landing (residuals). It
  cannot be re-measured by ablation without reverting a landed keep.

## 1. Build identity

| field | value |
| --- | --- |
| HEAD | `d72b07f3` (tracker commit; last code commit `9c3125d5`, t169) |
| exe sha256 | `b2e8261efb0b80f2478428948d3a9c8f761fef9abe0ea97fbabe9b4e3da3a852` |
| modules | 24 wasm under `modules/core-modules/`, tree sha256 `a65e2976…` |
| guests | `cargo xtask build-guests --check` exit 0 (before the campaign) |

The exe hash equals t169's *candidate ordinary* arm, so the tree is the one
t169's numbers were taken on and no rebuild drift is in play. The module tree
hash is this take's own construction — `sha256` over the sorted
`"<relpath> <sha256>\n"` lines of every `.wasm` — and is not comparable with
t169's recorded `9c1da3bf…`, which used a method this ticket did not recover.

## 2. Phase split (instrumented, three passes per cell)

Median of three passes; `repeat_max_share_delta_pp` is 0.43–2.96 pp across the
eight cells, so the medians are stable within the instrument.

| cell | prepass | per-layer | postpass | outside | Δprepass vs t164 |
| --- | --- | --- | --- | --- | --- |
| benchy classic off | **58.2 %** | 32.4 % | 6.0 % | 3.3 % | −12.3 pp |
| benchy classic on | **71.7 %** | 18.5 % | 8.3 % | 1.4 % | −5.4 pp |
| benchy arachne off | **55.9 %** | 34.7 % | 6.2 % | 3.1 % | −11.1 pp |
| benchy arachne on | **70.0 %** | 20.4 % | 8.1 % | 1.3 % | −5.5 pp |
| base classic off | **76.0 %** | 18.6 % | 5.0 % | 0.5 % | −6.7 pp |
| base classic on | **83.7 %** | 12.0 % | 4.2 % | 0.2 % | −3.5 pp |
| base arachne off | **74.4 %** | 20.3 % | 4.8 % | 0.6 % | −6.2 pp |
| base arachne on | **83.1 %** | 12.6 % | 4.0 % | 0.2 % | −3.8 pp |

Deltas are against [t164](../t164-phase-re-attribution/reduced/instrumented.json)'s
split on the same definitions (t154's), same thread count, same configs. The
movement is the expected direction and its size tracks each cell's t169 saving
(base −21.9…−25.1 %, benchy −5.7…−15.2 %): the phase that owned the offset work
shrank, so every other phase's share rose.

Instrumented walls for these passes are 2.339 / 5.348 / 2.437 / 5.534 / 14.089 /
51.032 / 13.767 / 50.295 s against t169's paired walls of record 2.245 / 5.024 /
2.309 / 5.137 / 13.730 / 49.336 / 14.396 / 49.832 s — **−4.4 % to +7.7 %**. The
two are different windows and are never substituted for one another; the shares
are the quantity this take carries.

## 3. Prepass stage split

Share of cell wall (instrumented), and what the cell's ratio would become if that
stage cost nothing (`estimated ratio × (1 − share)`, the same zero-cost
substitution t170 used for the offset family).

**Supports-off — `ShellClassification` is the cluster:**

| cell | `ShellClassification` | `Slice` | `SeamPlanning` | `MeshAnalysis` |
| --- | --- | --- | --- | --- |
| benchy classic off | **34.1 % → 1.08x** | 9.2 % → 1.49x | 6.3 % → 1.54x | 3.9 % → 1.58x |
| benchy arachne off | **32.2 % → 1.24x** | 9.0 % → 1.66x | 5.7 % → 1.72x | 3.6 % → 1.76x |
| base classic off | **44.0 % → 0.58x** | 14.5 % → 0.89x | 8.6 % → 0.95x | 5.2 % → 0.99x |
| base arachne off | **42.7 % → 0.64x** | 14.4 % → 0.95x | 8.1 % → 1.02x | 4.7 % → 1.06x |

**Supports-on — `SupportGeometry` is the cluster:**

| cell | `SupportGeometry` | `ShellClassification` | `SupportAnalysis` | `Slice` |
| --- | --- | --- | --- | --- |
| benchy classic on | **40.5 % → 1.41x** | 15.0 % → 2.02x | 6.6 % → 2.22x | 4.1 % → 2.28x |
| benchy arachne on | **38.9 % → 1.47x** | 14.8 % → 2.06x | 6.4 % → 2.26x | 4.1 % → 2.31x |
| base classic on | **54.9 % → 0.95x** | 11.9 % → 1.86x | 8.4 % → 1.94x | 3.9 % → 2.03x |
| base arachne on | **54.9 % → 0.92x** | 11.6 % → 1.80x | 8.4 % → 1.86x | 4.0 % → 1.95x |

Three things follow, and the third is the one that re-aims the route:

- The supports-on **base** cells are winnable by one stage, but only by nearly
  all of it: the required cut is **96.0 % / 92.4 %** of `SupportGeometry`'s own
  share. That is a rewrite or a deletion, not a tuning opportunity.
- The supports-off **base** cells are winnable by a *fraction* of one stage:
  **8.9 % / 24.2 %** of `ShellClassification`. This is exactly the cluster t172
  is priced for.
- The four **benchy** cells are winnable by no single stage: the largest needs
  **114.5 % / 143.1 % / 140.7 % / 150.3 %** of itself. Their top two stages
  leave **0.93x / 1.06x / 1.08x / 1.12x** (two of them still short) and their top
  three leave **0.83x / 0.90x / 0.97x / 0.96x** — which is why they sit in the
  map's fog rather than on a ticket: any benchy route is a combination, and the
  combination is not yet named.

The other phases are not candidates for the cells that need a large cut: zeroing
the per-layer phase leaves **1.11x–1.94x** on the four benchy cells, and zeroing
the postpass leaves **1.54x–2.22x** on them and on the two supports-on base
cells. On the two base supports-off cells several of these terms are
individually sufficient (per-layer 0.85x / 0.89x, postpass 0.99x / 1.06x) — that
cluster is over-determined, which is why t172's patch is the cheapest win on the
board rather than the only one.

Stage brackets are sequential inside the prepass and their sum reconciles with
the phase bracket in every cell — worst case +1.6 % (`verify.py`'s
`brackets_reconcile`).

## 4. Per-op census (`--profile`, one pass per cell)

`--profile` inflates wall and sums concurrent worker spans, so the columns below
are **accumulated thread-time**, ranked work shares — never wall, and never
compared with a wall. Host scopes are self-cost, summed over the built-in that
owns them.

| cell | host clipper total | `offset` | `clip_polygons` | `offset2_ex` | `closing_ex` |
| --- | --- | --- | --- | --- | --- |
| benchy classic off | 7.6 s | **44.8 %** | 33.3 % | 17.3 % | 4.7 % |
| benchy arachne off | 7.2 s | **42.9 %** | 34.4 % | 18.3 % | 4.5 % |
| base classic off | 88.8 s | **51.2 %** | 29.5 % | 16.1 % | 3.2 % |
| base arachne off | 82.5 s | **49.2 %** | 30.2 % | 17.4 % | 3.2 % |
| benchy classic on | 20.6 s | 38.8 % | **53.6 %** | 6.0 % | 1.6 % |
| benchy arachne on | 21.3 s | 39.4 % | **53.1 %** | 5.9 % | 1.6 % |
| base classic on | 343.4 s | 34.2 % | **60.8 %** | 4.2 % | 0.8 % |
| base arachne on | 343.4 s | 34.1 % | **60.9 %** | 4.2 % | 0.8 % |

**Owner split, supports-on (base classic-on; base arachne-on is within 1 %):**

| built-in | thread-time | calls | scopes |
| --- | --- | --- | --- |
| `host:native` (unattributed) | 236.3 s | 169,586 | clip 186.6 s, offset 49.7 s |
| `host:shell_classification` | 53.9 s | 8,914 | offset 27.5, clip 12.0, offset2_ex 11.6, closing 2.7 |
| `host:support_analysis` | 35.7 s | 4,430 | offset 32.6, clip 3.1 |
| `host:slice` | 12.7 s | 2,954 | offset 7.5, offset2_ex 2.9, clip 2.2 |
| `host:mesh_analysis` | 3.2 s | 49 | clip 3.2 |
| `host:overhang_annotation` | 1.6 s | 495 | clip 1.6 |

`host:shell_classification` is **100 %** marked clipper work (its `<module self>`
is 0.000 s on every cell): the stage's own residue is not a cost. On the
supports-off cells the same built-in owns 53.1 s of base classic-off's 88.8 s.

**`host:native` is an inference, disclosed.** The tool documents it as "native
`polygon_ops` activity that no host built-in bracket claimed — e.g. a marked
primitive reached from the per-layer tier" (`crates/slicer-runtime/src/profiling_report.rs`,
`UNATTRIBUTED_NATIVE_MODULE`). On the supports-on cells it holds 68.8 % of the
host clipper thread-time. It cannot be the per-layer phase: 236.3 s of thread at
12 threads is ≥ 19.7 s of wall, and the per-layer phase is 5.94 s. Its
thread-to-wall ratio (8.6x) instead fits `SupportGeometry` (27.6 s), so the
consistent reading is **prepass-tier native work that the stage bracket does not
attribute to a built-in**. This take did not build a probe to label it, and no
number here depends on the label — the clip-vs-offset *ratio* is measured
regardless of which bracket owns it.

**Guest fuel is deterministic and splits the same way:**

| cell | total fuel | top modules | `polygon_ops` scope fuel |
| --- | --- | --- | --- |
| base classic off | 138.8 B | `classic-perimeters` 65.0 %, `infill-linker` 27.8 % | `offset2_ex` 18.4 % |
| benchy classic off | 24.2 B | `infill-linker` 52.5 %, `classic-perimeters` 38.2 % | `offset2_ex` 10.5 % |
| base arachne off | 118.1 B | `arachne-perimeters` 56.0 %, `infill-linker` 35.5 % | `offset` 2.1 % |
| benchy arachne off | 23.9 B | `infill-linker` 57.6 %, `arachne-perimeters` 32.7 % | `offset` 1.4 % |
| base classic on | 592.8 B | `tree-support` 65.7 %, `classic-perimeters` 15.2 %, `tree-support-planner` 10.4 % | `offset2_ex` 4.3 %, clip 2.0 % |
| base arachne on | 572.1 B | `tree-support` 68.0 %, `arachne-perimeters` 11.6 %, `tree-support-planner` 10.7 % | clip 2.1 %, `offset` 0.6 % |
| benchy classic on | 75.6 B | `tree-support` 57.7 %, `infill-linker` 16.8 % | `offset2_ex` 3.4 %, clip 1.8 % |
| benchy arachne on | 75.3 B | `tree-support` 57.9 %, `infill-linker` 18.3 % | clip 1.8 %, `offset` 0.5 % |

The supports-on cluster is a **support-engine** cluster in guest work
(`tree-support` 57.7–68.0 % of fuel) as well as in host clipper work; the
supports-off cluster is the perimeters/infill-linker cluster. The two clusters
are not one program.

## 5. S5 and S6, re-priced

Both were landed by t132/t133, so there are two different quantities: what each
*is worth today* (the landed win, rescaled) and what each *still costs after
landing* (its residual mechanism). Rescaling multiplies a measured pre-t169
census by t169's measured `polygon_ops::offset` self-wall ratio for the same
cell, and divides by the stage parallelism t131 measured — **an ESTIMATE, not a
re-measurement**, because no instrument in this take separates either mechanism
from its enclosing stage.

| shape | cell | pre-t169 (thread) | rescaled | landed win | residual | residual ÷ cell wall |
| --- | --- | --- | --- | --- | --- | --- |
| **S5** internal-bridge whole-layer offsets | base classic off | 22,963.7 → 1,937.4 ms | ×0.4905 → 950.3 ms | **1,237 ms wall (9.01 %)** | 950.3 ms thread | **0.83 %** |
| **S5** | benchy classic off | 1,198.5 → 130.9 ms | ×0.5901 → 77.2 ms | 630 ms thread (3.36 %) | 77.2 ms thread | **0.41 %** |
| **S5** | base classic on | 23,086.4 → 1,991.6 ms | ×0.4451 → 886.5 ms | 9,389 ms thread (2.28 %) | 886.5 ms thread | **0.22 %** |
| **S6** occupancy `+0.35 mm` | base classic on | 864 calls / 433 distinct | ×0.4451 → 6,362.1 ms | **2,892 ms wall (5.86 %)** | 6,362.1 ms thread | **5.89 %** |
| **S6** | benchy classic on | 365 / 186 | ×0.5486 → 347.5 ms | 334 ms thread (3.04 %) | 347.5 ms thread | **3.16 %** |

Readings:

- **S5's residual is at the 0.80 % floor on base classic-off (0.83 %) and below
  it elsewhere.** The window already harvests 91.6 % of the whole-layer cost;
  what is left is the windowed erosion itself, and no further localization of it
  can pay. The landed win is real and is part of why base classic-off sits at a
  3.9 % cut.
- **S6's residual is 5.89 % of base classic-on**, i.e. **11 % of that cell's
  52.8 % cut** — and it is the *memoized* remainder (433 distinct payloads), so
  further work on the same family tops out there. **S6 is also inert on the four
  supports-off cells by construction** (`enable_support: false` returns before
  either memo site, t132's own finding), so it can never satisfy an all-eight-cell
  gate whatever its price.
- **Neither shape is a route lever, and neither number may be quoted again as a
  candidate price.** The only live offset candidate is t172.

## 6. What survives (the short list)

| cluster | survivor | measured carrier | against its cut |
| --- | --- | --- | --- |
| base classic-off + base arachne-off | **t172** `OffsetConcaveCrossing` port (already charted) | `offset` 51.2 % / 49.2 % of host clipper thread-time; `ShellClassification` 44.0 % / 42.7 % of cell | 0.58x / 0.64x at zero stage; t170 prices the patch to win both |
| base classic-on + base arachne-on | **boolean volume in the support path** (new ticket 173) | `clip_polygons` 60.8 % / 60.9 % of host clipper thread-time; `host:native` 68.8 % of that mass; `SupportGeometry` 54.9 % of cell | needs 96.0 % / 92.4 % of the stage |
| four benchy cells | **nothing sharp yet** — combinations only | `ShellClassification` 34.1/32.2 % or `SupportGeometry` 40.5/38.9 % of cell; whole prepass needed | 1.08x–1.47x at zero for the largest stage |
| all cells | S5, S6 | offsets, already landed | residual 0.22–5.89 %, see §5 |

Nothing else in this take carries a cut on the six cells that need one: `Slice`
(3.9–14.5 % of cell), `SeamPlanning` (2.3–8.6 %), `MeshAnalysis` (1.3–5.2 %) and
`SupportAnalysis` (6.4–8.4 % on supports-on, 0.0 % on supports-off) all leave
their cluster above 1.0x when zeroed. On the two base supports-off cells they are
individually sufficient, which is a property of that cluster's already-small cut,
not a second route.

## 7. Disclosures

- **No fresh two-tool board.** The required cuts are t169's §9 composition
  (t163's board × t169's measured saving), re-derived from the committed
  artifacts — an estimate, not a ratio of record. t161 forbids carrying any
  required cut across windows as a measurement, so every `ratio_if_zero` here
  inherits that status. A fresh scoreboard is a separate question and was out of
  scope.
- **One window, one box.** Shares and profiles come from the same session;
  instrumented walls sit −4.4 % to +7.7 % of t169's walls of record purely as a
  cross-window artifact. No wall claim is made.
- **`--profile` totals are thread-time**, inflated by mark overhead and summed
  across concurrent workers. They rank work and split ops; they are never
  compared with a wall, and the clip-vs-offset ratio is the only cross-cell
  quantity quoted from them.
- **`host:native` is a labelled inference**, not an instrument reading (see §4).
  Nothing in the verdict depends on it.
- **S5/S6 are rescales**, not re-measurements, and their parallelism factors
  (8.34x / 2.19x) are t131's, not this take's. The landed wins are historical
  facts; the residuals are estimates with a measured input.
- **No production code changed**, so no test was added and no reference moved.
  All 24 instrumented runs reproduce t169's reference set byte-for-byte, which
  is the take's control that the tree is the one t169 measured.

## 8. Verification

- `python verify.py` — **PASS** (4 checks: payload recomputes, 24 runs match
  t169's reference set, every pass `degraded: false` with zero errors, phase and
  stage brackets reconcile).
- `python verify.py --self-test` — **PASS**, **8/8** negative controls rejected
  (instrumented share, t169 paired saving, t169 offset profile, t131 S5 census,
  t131 S6 family, run hash, validation flag, stage bracket).
- `cargo xtask build-guests --check` — exit 0 before the campaign.
- `cargo build --release -p pnp-cli` — clean; exe sha256 `b2e8261e…` equals
  t169's candidate build.

## 9. Reproduce

See [`REPRODUCE.md`](REPRODUCE.md). Short form:

```bash
cargo xtask build-guests --check
cargo build --release -p pnp-cli
python run-instrumented.py               # 8 cells x 3 passes, hash-checked
python run_profile_t171.py --tag clean   # 8 cells x 1 --profile pass
python reduce_instrumented.py
python reduce_profile_t171.py --tag clean
python arithmetic.py
python verify.py --self-test
```
