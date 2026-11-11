# Overhang quartile bands: is the 4-band polygon partition read by anything?

Type: task / Phase-1 attribution + measured candidate
Label: wayfinder:task
Status: resolved (2026-11-11; Phase-1 attribution + price — **the quartile
  VALUE has no production reader; only its presence bit is read, and that bit
  is the sign of the signed distance the consumer already computes. The band
  partition is 8.96–10.58 s of the base classic-off wall (22–27 %, 34–41 % of its
  required cut), 9.5 s on base arachne-off, 11.2 s on base classic-on and 0.95 s
  on benchy classic-off. GO for a measured deletion candidate, graduated as
  [t142](142-overhang-band-partition-deletion.md).** See ## Answer)
Assignee: 2026-11-11 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map, pick an afk ticket"; the AFK frontier was empty (137 is the
  human's grilling; 136 and 135 Stage 2 are gated by it), so this ticket was
  graduated from the map's "sweep-volume work-set family" fog and claimed.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Prepass work-set and per-op parity against canonical v2.5.0](129-prepass-work-set-per-op-parity.md)'s
  2026-11-08 S1 row (overhang quartile bands → per-vertex distance, **9,689 ms**
  base classic-off = 34.8 % of its required cut, 28.5x the 0.80 % floor;
  10,309 ms / 11.2 % base classic-on), left as an un-ticketed "clean PnP-local
  work-set deletion" behind the t131/t132/t133/t134 sequence.

## Question

`PrePass::OverhangAnnotation` builds a **4-band polygon partition** — 3 round
offsets plus ~9.8 clips per band layer (**1,455 offsets / 4,859 clips** over 485
band layers on base classic-off, t84) — and ships both the bands
(`overhang_quartile_polygons`) and the previous layer's boundary to the guest.
Canonical has **no band partition at all**: `precompute_overhang_layer`
(`GCode/ExtrusionProcessor.hpp`) builds one `LinesDistancer<Linef3>` from the
lower layer and compares **one signed distance per vertex** against the numeric
thresholds `{0.1w, 0.25w, 0.5w, 0.75w, 0.87w, 1.0w}`.

PnP's `expolygon_to_path3d_indexed`
(`crates/slicer-core/src/perimeter_spatial.rs`) already computes
`overhang_distance_mm` per vertex against `prev_layer_boundary`
— the same previous-layer contour the prepass hands it — **and** separately
queries `overhang_quartile(x, y)` through the band polygons. So the distance is
already paid for at the consumer.

The question is whether the band partition has **any production reader** at all:

- Does anything consume `Point3WithWidth.overhang_quartile` for geometry, `E`,
  speed selection, or ordering — or is the emission-time speed profile (which
  reads `overhang_distance_mm` with `speed_sections {90, 75, 50, 25, 13, 0}`)
  the only consumer, making the quartile a debug-only surface?
- Does anything besides `visual_debug_render` read
  `SurfaceClassificationIR.overhang_quartile_polygons` (and the per-point
  `PreparedRegions` bands)?
- Is the previous-layer boundary map (`annotate_overhangs`' second return value)
  independently required, so the partition can be dropped while the boundary
  stays?

If the answer is "no production reader", the 9,689 ms is **dead work** and the
deletion is exact by construction (no delta audit needed beyond the
visual-debug/IR disposition). If a production reader exists, the ticket must
instead price deriving the quartile from the already-computed distance at the
consumer (PnP-local semantics, byte-comparison against the polygon query).

Gate (unchanged from the effort's standing contract):

- Output: byte-identical G-code in the 16 arm/cell cells, or a bounded delta
  confined to a named surface, audited **before** timing.
- Timing: median uninstrumented wall at the **0.80 %** paired floor, process CPU
  corroborating, in **all 8** cells the shape touches.
- No production edit before the Phase-1 reader census and the price land.

## Comments

_None yet._

## Answer

**Resolved 2026-11-11 as a Phase-1 attribution and price. The band partition
is deletable work, not a classification the pipeline reads.** The ticket
graduates a measured candidate, [t142](142-overhang-band-partition-deletion.md);
no production edit landed here.

1. **The quartile value is unread.** `Point3WithWidth.overhang_quartile` is
   stamped at three sites — `expolygon_to_path3d_indexed`
   (`crates/slicer-core/src/perimeter_spatial.rs`), `expolygon_to_path3d`
   (`crates/slicer-core/src/perimeter_utils.rs`) and
   `arachne-perimeters`' per-vertex loop — and every other production site
   copies it or tests it for presence. `overhang-classifier-default`'s speed
   loop gates on `point.overhang_quartile.is_some()` and computes the speed
   from `overhang_distance_mm` via `calculate_speed`. No production line
   compares the value. `overhang_areas` (the flattened bands) has no reader in
   `modules/` at all.
2. **The presence bit is the distance sign.** The four bands' union is exactly
   `current \ previous`, so a path vertex (inside `current`) is in a band iff it
   is outside `previous`. The same per-vertex loop already computes
   `signed_distance_to_boundary` against `prev_layer_boundary`, the same
   object-level previous-layer set the bands were built from. The two tests can
   disagree only on the previous boundary itself; a one-unit (0.0001 mm)
   epsilon on the raw distance matches Clipper's quantization. This is an
   argument, not a measurement — t142's gate is byte equality in the 16
   arm/cell cells.
3. **Price on the post-t139 tree** (probe `[t141]`, `probe.patch`):

   | cell | stage wall | share of wall | share of required cut |
   | --- | ---: | ---: | ---: |
   | base classic-off (3 probes) | 8.964 / 9.558 / 10.583 s | 22.1–26.5 % | 34.3–40.5 % |
   | base arachne-off | 9.450 s | 24.0 % | 37.7 % |
   | base classic-on | 11.195 s | 12.1 % | 15.9 % |
   | benchy classic-off | 0.954 s | 21.8 % | 37.8 % |
   | benchy classic-on | 0.817 s | 10.4 % | 15.0 % |

   The counts are **3 offsets + 6 whole-layer booleans per band layer** in the
   cumulative stage (the probe counts those; the three band-subtraction
   differences are extra), over 486 band layers of 496 on base and 239 of 240
   on the benchy. The band thread-time converts at 11.6–11.7x on base,
   reproducing t84's 11.86x. t84's **85.4 %** offset share is the floor for the
   deletion if the per-layer overhang key test stays; the full stage wall is
   the ceiling.
4. **Two forks for the build.** (a) `prev_layer_boundaries` is populated only
   for layers with overhang, so `overhang_distance_mm` is `None` elsewhere;
   keeping that pattern keeps one whole-layer `difference_ex` per layer and
   gives up at most the 14.6 % of native work not attributed to the offsets.
   Populating every layer looks inert (no overhang ⇒ no boundary crossing to
   insert) but must be proven. (b) `overhang_quartile_polygons` is also the
   visual-debug band tap and a preparation-input key; the clean shape leaves it
   empty in production and reconstructs bands in the renderer from
   `prev_layer_boundary` + the resolved line width.
5. **Not claimed.** No A/B, no scoreboard, no reference move; the box was not
   verified idle and the three base-off probes span 18 % (8.96–10.58 s). The
   equivalence is geometric, not measured on emitted G-code.

Evidence: [`evidence/t141-overhang-bands/`](../evidence/t141-overhang-bands/FINDINGS.md)
(`verify.py` 7 rows / 3 source files; `--self-test` rejects 4/4 negative
controls).
