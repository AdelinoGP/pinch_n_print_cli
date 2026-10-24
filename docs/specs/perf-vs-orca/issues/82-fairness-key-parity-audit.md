# Fairness-key parity audit of the matched job

Type: task
Label: wayfinder:task
Status: resolved (2026-10-23; measurement-only — all 618 resolved keys
classified; no third resolution-class lever at scale, the Arachne tolerance
pair is the one unmatched PnP-does-more geometry key and its magnitude is
unmeasured; the generated deviations table cannot see it because the upstream
snapshot predates the key — see ## Answer)
Assignee: 2026-10-23 OpenCode wayfinder session — human selected "Fairness-key
parity audit" from the route options after the frontier re-derivation found no
open child (81/81 resolved)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Slice performance vs OrcaSlicer](../map.md) — "Not yet specified",
"Work-relevant key parity of the matched job" (surfaced by
[80](80-resolution-fairness-premise.md), `resolution` half closed by
[81](81-resolution-slicing-time-contour-simplification.md))

## Question

Which keys does the matched Orca job **actually resolve** — read from its
G-code appendix, not the profile file — that change *pre-emission geometry
work*, and for each one does PnP have the counterpart at a value that makes it
do the same work?

The output is a classified key table plus a verdict: is there a **third**
unmatched material lever of `resolution`'s class, or is the pre-emission
geometry work now matched?

## Why this is route work

Ticket 80 found that the map had been comparing PnP at full contour detail
against an Orca that was silently doing materially less geometric work:
`resolution` alone was worth **+9.5 % to +65.2 %** of Orca's own wall (median
**+40.2 %**) across the eight cells. Ticket 81 ported it and measured classic
cells **−9.0 % to −22.2 %** wall. That is a large fairness correction found by
accident, so the general question — *what else is Orca doing before it emits
geometry that PnP is not* — has to be asked deliberately rather than waiting
for the next accident.

## What is already known, and must not be re-derived

Two keys are settled and appear in the audit only as controls:

- **`resolution`** — ported end to end by ticket 81 (default 0.01 mm, canonical
  0.001 mm floor) at four call sites, with the canonical-exact Douglas–Peucker.
  Matched.
- **`slice_closing_radius`** — PnP declares its counterpart in
  `ResolvedConfig::slice_closing_radius` at the canonical default `0.049` mm,
  and `PrePass::Slice` applies `apply_slice_closing_radius` to the raw layer
  contours **before** the resolution simplify — the same key, default and
  order as canonical's `PrintObject::slice` closing followed by the
  per-surface simplify. Matched. (This retires the `slice_closing_radius` half
  of the map's fog entry; it was named there as "the obvious next candidate".)

The audit's job is everything else.

## Scope

**In scope**: keys resolved by the matched job that change the amount or shape
of geometry work **before emission** — mesh slicing, closing, surface
simplification, perimeter generation (classic and Arachne), fill generation,
support generation and its geometric post-processing.

**Out of scope**: emission-time keys (G-code formatting, arc fitting, modal
compression, per-role D-P tolerances), UI/viewer keys, machine limits, filament
and temperature keys, and anything that only changes *what is written* rather
than *what is computed*. Those are adjudicated in the map's Out-of-scope
section and in the orca-feature-gap tracker.

Keys already adjudicated as **parity work** (`precise_outer_wall`,
`only_one_wall_top`, the `wall_maximum_*` family) are recorded with that
verdict and are **not** re-opened here — this ticket classifies, it does not
adjudicate parity scope.

## Work

1. **Resolve the job.** Generate the eight matched derived configs (the same
   override block the scoreboard rig uses) and run OrcaSlicer once per cell.
   Extract each run's full `; key = value` appendix. The appendix is the
   authority: it records what the resolved config actually was, including keys
   the vendor profile sets that no override mentions.
2. **Diff the cells.** Which keys vary by cell, and which are constant across
   all eight? A constant key is still in scope if its value differs from PnP's
   counterpart default.
3. **Join against PnP.** For every in-scope key, record PnP's counterpart
   (config key, struct field, or "absent") and its **default on the matched
   job** (after the matched configs' overrides). Source of truth: the generated
   registry in `docs/15_config_keys_reference.md` plus the matched configs in
   `evidence/matched-pair/configs/`, cross-checked against PnP's own emitted
   config block (`evidence/t34-config-block/production-block.txt`) — the
   generated registry and the emitted block disagree on at least two keys, so
   neither alone is authoritative.
4. **Classify** each in-scope key into the audit's buckets: `matched` /
   `equal-by-units` / `equal-by-rename` / `equal-by-clamp` / `emission` /
   `differs-inert` / `geometry-differs` / `absent-inert` / `absent-material` /
   `absent-non-geometry`. Every row gets a bucket; `review` fails the merge.
5. **Verdict**: name any geometry key where PnP does more work than the matched
   job, or state that the pre-emission geometry work is matched.

## Evidence standard

Per the map's evidence policy: the appendix extracts, the derived configs and
the classifier live **in-repo** under
`docs/specs/perf-vs-orca/evidence/t82-fairness-key-audit/`; a `verify.py`
re-derives every headline count from the committed extracts and exits 0
checkout-only, with negative controls rejected under `--self-test`. Raw G-code
is regenerable via the committed runner and is never committed.

## Answer

**Resolved 2026-10-23 (measurement-only).** Verdict: **no third
`resolution`-class lever at scale**; the only unmatched geometry pair where PnP
does materially more work than the matched Orca job is the Arachne
wall-simplification tolerance pair, already logged as
`D-168-ARACHNE-SIMPLIFY-FALLBACKS-TIGHTER-THAN-CANONICAL` but never priced.

Eight Orca runs produced a **618-key** resolved-config appendix per cell
(identical key set in all eight). The mechanical join classifies **every** key:

| bucket | count |
|---|---:|
| PnP has no counterpart, key is not geometry | 332 |
| PnP has no counterpart, feature resolves inert | 101 |
| equal (value, unit/format, rename, or canonical clamp) | 112 |
| emission-only | 40 |
| PnP has no counterpart, value enables geometry work | 18 |
| counterpart exists, values differ, consumer not exercised | 1 |
| counterpart exists, both run it, values differ | 14 |

**Finding 1 — the fairness machinery has a snapshot blind spot.** The generated
*Deviations from OrcaSlicer* table in `docs/15_config_keys_reference.md` (27
rows) compares against `docs/ORCA_CONFIG_REFERENCE.md`, dated **2026-05-19**.
**82 of the 618 resolved keys are absent from that snapshot**, including both
Arachne tolerance keys — which is why the table cannot see them. Same failure
shape as ticket 80's `resolution` discovery.

**Finding 2 — the Arachne pair is the lever.** `wall_maximum_resolution`
0.5 mm vs PnP 0.05 mm and `wall_maximum_deviation` 0.025 mm vs PnP 0.005 mm:
PnP keeps 10x / 5x more Arachne wall detail before beading. Arachne cells only.
**Magnitude unmeasured** — this take ran no timing. It is a plausible partial
explanation for those cells coming back wall-neutral in ticket 81's campaign,
but that is not claimed.

**Finding 3 — the rest does not flatter PnP.** The other differences either make
**Orca** do more work (`top_shell_layers` 4 vs 3,
`support_interface_top_layers` 3 vs 2, `precise_outer_wall` 1 vs 0,
`ensure_vertical_shell_thickness`, `detect_narrow_internal_solid_infill`, tree
support brim), are layer-0-only (`skirt_loops` 6 vs 1, `skirt_distance` 2 vs 3),
are shape-only, or resolve inert. Where Orca does more, the measured ratio
understates PnP's per-unit-work deficit.

**Controls matched in all eight cells:** `resolution = 0.01` (ticket 81's port
is visible in the appendix) and `slice_closing_radius = 0.049` (same key, same
default, same order as canonical). This closes the `slice_closing_radius` half
of the map's fog entry.

**Observations recorded:** `detect_thin_wall` is declared `true` by
`classic-perimeters.toml` but resolves `0` in the emitted block (two modules
declare one key with different defaults — a ledger hazard, matched with Orca by
accident); `support_interface_bottom_layers` is declared `-1` but emits `2`;
`brim_width` (8 mm vs 0) is unresolved at the output level because both cells
emit a `Brim` role; Orca's overhang speed bands are live (50/30/10) while PnP's
are 0, so ticket 20's consume-only gate is valid for PnP's job but the engines
are not doing the same work there.

**Recommendation:** graduate one measurement ticket — price the Arachne
tolerance match on the four Arachne cells (canonical 0.5 / 0.025 vs current),
with its own pre-declared gate and a human KEEP/DROP. Not implemented here.

Evidence: [`evidence/t82-fairness-key-audit/`](../evidence/t82-fairness-key-audit/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only with 9/9 negative controls
rejected; `key-table.csv` carries all 618 rows). No candidate, no timing
campaign, no production edit.

## Non-goals

No candidate implementation, no timing campaign, no production edit. A
material finding graduates its own ticket and goes back to the human for a
route decision — it is not implemented here.
