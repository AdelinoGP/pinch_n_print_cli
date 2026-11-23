# Ticket 19 — `offset2_ex` call-graph re-derivation at HEAD `ca6841fc`

Route ticket: [offset2_ex call reduction](../../issues/19-offset2-ex-call-reduction.md).

**Verdict of this measurement: the ticket's stated call graph is stale and does
not describe HEAD. The 720-call `min_width_top` shrink/expand — the ticket's
headline site — does not execute at all on the map's matched classic job.** The
site is gated on `min_width_top_surface > 0` inside `only_one_wall_top`, and the
route configs leave both at their defaults (gate off, `only_one_wall_top`
false). Every `offset2_ex` call at HEAD comes from three other sites, which the
ticket either mis-attributes or does not name.

This note is the re-derivation the ticket asked for, captured with a temporary
probe that was then removed mechanically (see §Probe discipline). It is the
attribution half of the take; the candidate implementation and its standing
paired A/B are in [RESULTS.md](RESULTS.md) and `protocol.json`, `verify-t19.py`,
`manage.py` alongside this note.

## Why re-derive at all

The ticket's numbers are ledger facts captured 2026-09-22 at `bde9b1ba`
(`evidence/perf-emit-walls/FINDINGS.md`). Between that capture and HEAD the
module gained the `only_one_wall_top` config gate (`min_width_top_surface`,
packet 184), the narrow-island classification path, per-invocation config
resolution, and the config-scope-resolution program. The capture's own config
(`tmp/perf-flags/quiet-validation/benchy-supports-off-classic.json`) is no
longer on disk — `tmp/perf-flags/` is gone — so the split could not be
reproduced by re-running the original recipe. It had to be measured.

## Method

Temporary per-call-site fuel scopes (`[T19-PROBE]`, marker-grepped and removed)
were inserted at every `offset2_ex`/`opening_ex` call site in
`modules/core-modules/classic-perimeters/src/lib.rs` and inside
`classify_narrow_island`. Each scope was braced around its single expression so
the RAII guard could not bleed into a sibling. The guest was rebuilt, staged
into `target/dist/developer/modules/`, and the map's matched classic
supports-off job was sliced with `--profile`. Fuel shares are exact and
mode-independent (ADR-0055).

Job: `docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json`
(benchy, 0.4 mm, 0.2 mm layers, 2 walls, classic, 20% gyroid, supports off),
12 threads, external-only, probe-free except for the scopes themselves.

## Measured call graph (route job, `only_one_wall_top = 0`)

`com.core.classic-perimeters` total fuel **750,034,599,482**; the
`polygon_ops::offset2_ex` scope is **720 calls, 110,045,535,936 fuel = 14.67%**
of the module.

| call site (crate-qualified symbol) | calls | fuel | % of `offset2_ex` | % of module |
| --- | ---: | ---: | ---: | ---: |
| `offset2_ex` in the `min_width_top` shrink/expand (`classic-perimeters` `run_perimeters`, `only_one_wall_top` branch) | **0** | 0 | 0% | 0% |
| `opening_ex` in `emit_walls` thin-wall `thick_core` (site gated on `detect_thin_wall`) | **0** | 0 | 0% | 0% |
| `opening_ex` gap-fill `opened_min` (`emit_walls` gap-fill block) | 240 | 10,147,895,405 | 9.22% | 1.35% |
| `offset2_ex` gap-fill `opened_max` (same block) | 240 | 54,957,437,007 | 49.94% | 7.33% |
| `opening_ex` narrow-island classification (`classify_narrow_island`) | 240 | 44,940,282,244 | 40.84% | 5.99% |

The three live sites sum to **110,045,614,656** fuel (14.672% of the module) —
the whole `offset2_ex` scope, within 0.00007% of the instrumented total. The
arithmetic closes, so no fourth site exists on this job.

Two of the five `offset2_ex`/`opening_ex` call sites in the file are dead at the
route config:

- the `min_width_top` shrink/expand (the ticket's 720-call headline) needs
  `only_one_wall_top && top_shell > 0 && min_width_top_surface > 0`;
- the thin-wall `thick_core` needs `detect_thin_wall`, which the route configs
  leave at the module default `false`
  (`modules/core-modules/classic-perimeters/classic-perimeters.toml`;
  Orca's own `detect_thin_wall` default is `false` too —
  `PrintConfigDef::init_fff_params` in `PrintConfig.cpp`), so the block never
  runs. The emitted appendix line `; detect_thin_wall = 0` is the
  `ORCA_CONFIG_ALIAS_KEYS` disclosure in `crates/slicer-gcode/src/serialize.rs`,
  consistent with the observed zero calls.

## The five live sites under a gate-on config

To bound the ticket's headline site rather than just report it as dead, the
same probe was run with `only_one_wall_top = true` and
`min_width_top_surface = 300%` (Orca's upstream default, `PrintConfig.cpp`).
Classic supports-off benchy then measures **743** `offset2_ex` calls and module
fuel **584,485,103,574**:

| site | calls | fuel | % of module |
| --- | ---: | ---: | ---: |
| `min_width_top` shrink/expand | **13** | 216,987,474 | 0.037% |
| thin-wall `thick_core` | 0 | 0 | 0% |
| gap-fill `opened_min` | 245 | 7,019,377,561 | 1.201% |
| gap-fill `opened_max` | 245 | 38,065,636,702 | 6.513% |
| narrow-island `opening_ex` | 240 | 44,940,075,921 | 7.689% |

The ticket's 720-call figure for this site is therefore wrong twice over: the
site is off on the matched job, and even switched on it fires on **13 of 240
layers** (0.037% of module fuel), not 720 times. The 720 in the ticket and in
the handoff is the *total* `offset2_ex` call count of the old job, not this
site's share; the 960/720/240 split in the ticket does not reproduce.

## What this means for the ticket's candidate shapes

The ticket asked for three shapes; the measurement reframes all of them:

1. **"Eliminating redundant shrink/expand pairs around `split_top_surfaces`"**
   — the site is off on the route job and 0.037% when on. There is no redundancy
   to eliminate that could move a cell; the site's own cost is already at the
   instrumented noise floor. **Not a candidate.**
2. **"Reusing results across the top-surface flow"** — the same dead site.
   **Not a candidate on this job.**
3. **"Coalescing the gap-fill width-band calls"** — live, but the two band ops
   (49.94% + 9.22% of `offset2_ex`) are not redundant with each other: they
   compute `open(gaps, min/2)` and `open(gaps, max/2)` at *different* distances,
   and Orca's own canonical gap band does the same (`opening_ex(gaps, min/2)`
   vs `offset2_ex(gaps, -max/2, +max/2 + ClipperSafetyOffset)` —
   `PerimeterGenerator::process_classic`'s gap-fill block). Coalescing them
   would change the width band's meaning. **Not output-preserving.**

A fourth shape the ticket did **not** name is the largest single site:
`classify_narrow_island`'s `opening_ex(polygons, threshold/2)` — 40.84% of
`offset2_ex`, 5.99% of module fuel — whose **result geometry is discarded**. The
function reads only `opened.is_empty()`.

The emptiness identity holds for a positive distance. Opening is
`dilate(erode(X, d), d)` with the *same* positive `d` on both passes. A
positive polygon offset maps `∅ → ∅` (Clipper never manufactures area from
nothing) and is extensive on non-empty inputs (the offset region contains its
input polygon), so `dilate(Y) = ∅ ⟺ Y = ∅`. Therefore for `d > 0`

    opening(X, d) = ∅  ⟺  erode(X, d) = ∅.

Both directions hold; this is not an empirical coincidence. A single-pass
erode that shares `offset2_ex`'s first pass (same miter limit, same arc
tolerance, same mm→unit conversion) is an exact output-preserving short-circuit
for the emptiness read, not a predicate rewrite.

For `d <= 0` the identity does **not** hold, and the candidate delegates to the
unchanged two-pass opening there. Two measured boundaries:

- `d == 0`: `inflate_paths_64` short-circuits to a clone, so the first pass
  skips the union cleanup the second pass still runs; a degenerate zero-area
  ring survives the clone and is dropped by the opening.
  `smaller_perimeter_threshold_mm` admits 0.0, so this is reachable.
- `d < 0`: opposite-sign passes; the second can empty a non-empty first pass on
  the same degenerate ring.

(An earlier draft of this note argued the converse fails because a shape "can
erode to a non-empty skeleton whose dilation is also non-empty" — that sentence
argues the equivalence, not against it, and the note's claim that the
short-circuit was "falsified by counterexample in principle" was wrong. The
error is corrected here. It also missed the `d <= 0` boundary, which the
candidate's delegation covers and its tests pin.)

Measured on the route job, replacing the two-pass `opening_ex` with a
single-pass erosive offset produced an output **byte-identical** to the
two-pass baseline
(`34886ccb56b214d57c28aa58bdcc8d371a41f440200f5b49276b96b53e18cc40`, 4,332,737
bytes, equal to the unprobed dist baseline) and cut the site from 44.94 B to
27.34 B fuel (module fuel 750.03 B → 732.43 B, ≈ −2.35%). This is the surviving
candidate shape; it reduces `offset2_ex` calls by 240 (720 → 480). The
measurement above used `offset_with_miter_limit` (tree-extraction) as the erode;
the candidate implementation instead shares `offset2_ex`'s first pass exactly,
which is empty-equivalent by construction. The candidate and its standing paired
A/B are documented in [RESULTS.md](RESULTS.md).

## Probe discipline

The probe exists only as
[`probe-callsites.patch`](probe-callsites.patch) (130 lines, 8 `[T19-PROBE]` /
`t19_scope` markers). Production source was reverted with
`git checkout -- modules/core-modules/classic-perimeters/src/lib.rs`; the file
greps **0** for the marker, and `git status` shows only pre-existing unrelated
work. The guest was rebuilt from the clean tree; the dist copy was refreshed
from it. Raw captures live under `target/t19-ab/` (gitignored) with the reduced
attribution in [`reduced/probe-attribution.json`](reduced/probe-attribution.json).

## Limits

- Fuel is not wall. This take measured a **call graph**, not a speed win; the
  ticket's acceptance gate (paired ordinary + accelerated, interleaved repeats,
  starvation exclusion, probe-free runs, isolated snapshots) has not been run
  and no candidate was gated.
- One fixture (benchy), one generator (classic), the route job's settings. The
  gate-on differential is a bound, not a second job's measured graph.
- The 2026-09-22 capture's raw profile and its config are gone; the historical
  figures in the ticket and the handoff are treated as unverifiable and are not
  silently adopted.
