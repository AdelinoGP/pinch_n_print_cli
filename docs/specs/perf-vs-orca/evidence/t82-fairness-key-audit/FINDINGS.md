# t82 — Fairness-key parity audit of the matched job

Ticket: [82-fairness-key-parity-audit](../../issues/82-fairness-key-parity-audit.md)
Map: [Slice performance vs OrcaSlicer](../../map.md)
Take: 2026-10-23, measurement-only (no candidate, no timing campaign, no production edit).

## Verdict

**There is no third `resolution`-class lever at scale.** The only unmatched
pair of pre-emission geometry keys where PnP runs materially more work than the
matched Orca job is the Arachne wall-simplification tolerance pair
`wall_maximum_resolution` / `wall_maximum_deviation` — PnP is **10x / 5x
tighter** than the values the matched job actually resolves. It is already a
logged deviation (`D-168-ARACHNE-SIMPLIFY-FALLBACKS-TIGHTER-THAN-CANONICAL`,
re-derive its row from `docs/DEVIATION_LOG.md`), but it has never been priced
against wall, and the repo's generated deviations table **cannot see it**
because the upstream reference snapshot predates the key.

Everything else that differs is either emission-only, a wire-format or unit
artefact, a rename, a feature that resolves inert on this job, or a difference
that makes **Orca** do more work than PnP.

## What was measured

Eight Orca runs, one per cell of the matched matrix, each sliced from the
vendor profile triple plus the same string-typed override block the
matched-pair scoreboard rig uses. Each emitted G-code carries a **618-key**
resolved-config appendix; that appendix, not the profile file, is the authority
for what the job actually ran.

| | |
|---|---|
| cells | 8 ({benchy, base} x {classic, arachne} x {supports off, on}) |
| resolved keys per cell | 618 (identical key set in all eight) |
| keys that vary by cell | 3 — `wall_generator`, `enable_support`, `print_settings_id` |
| keys constant across all eight | 615 |
| keys in pre-emission geometry scope | 227 |
| PnP has the counterpart, values differ | 14 (`geometry-differs`) |
| PnP has no counterpart, value enables geometry work | 18 (`absent-material`) |
| PnP has no counterpart, key is not geometry at all | 332 |
| PnP has no counterpart, feature resolves inert | 101 |
| emission-only (no pre-emission geometry work) | 40 |
| counterpart exists, but its consumer never runs on this job | 1 |
| equal by value, unit/format, rename, or canonical clamp | 112 |

The two settled controls resolve correctly in **every** cell:
`resolution = 0.01` and `slice_closing_radius = 0.049`. Ticket 81's port is
visible in the appendix, and the closing radius is the canonical default
applied before the resolution simplify — same key, same value, same order.

## Finding 1 — the fairness machinery has a snapshot blind spot

`docs/15_config_keys_reference.md` carries a generated
**Deviations from OrcaSlicer** table, and it is the repo's mechanical answer to
"which PnP defaults differ from canonical". It has **27 rows**.

It cannot answer this audit's question, for a structural reason:

> Generated keys whose numeric default differs from the matching key in
> `docs/ORCA_CONFIG_REFERENCE.md` (the upstream snapshot).

The reference snapshot is dated **2026-05-19**. **82 of the 618 keys the matched
job resolves are absent from it entirely** — Orca keys added after that date.
Most are machine/filament/UI (`input_shaping_*`, `zaa_*`, `machine_max_*`,
`thumbnails`). Two are pre-emission geometry:

- `wall_maximum_resolution = 0.5` mm
- `wall_maximum_deviation = 0.025` mm

Both are missing from the 27-row table precisely because the snapshot does not
contain them. That is the same failure shape as ticket 80's `resolution`
discovery: a work-affecting key that the map's fairness apparatus was
structurally unable to see. `resolution` was caught only because it happened to
predate the snapshot.

**Action for the human:** the deviations generator compares against a stale
upstream snapshot; the matched job's own appendix is the only authority for
what the comparison job resolves. This audit does not change the generator.

## Finding 2 — the Arachne tolerance pair is the only `resolution`-class lever

| key | matched Orca | PnP | effect |
|---|---|---|---|
| `wall_maximum_resolution` | 0.5 mm | 0.05 mm | PnP keeps **10x** more Arachne wall detail |
| `wall_maximum_deviation` | 0.025 mm | 0.005 mm | PnP keeps **5x** more Arachne wall detail |

Both are wired directly (no `min()`/merge) into
`ArachneParams.smallest_line_segment_squared` / `allowed_error_distance_squared`
as mm², i.e. they change how aggressively Arachne's wall toolpaths are
simplified before beading, rendering and emission. **Arachne cells only**
(benchy arachne-off/on, base arachne-off/on).

**Magnitude: unmeasured.** This take produced no timing. What is established is
that the two engines are not doing the same work on those four cells, and the
direction is PnP doing more. Whether it is worth a percentage point or ten is
the next question, and it is a measurement question, not a design one.

Note the interaction with ticket 81: the Arachne cells were the ones that came
back **wall-neutral** in the `resolution` campaign. A tighter wall simplifier is
a candidate partial explanation, but this audit does not claim it.

## Finding 3 — the remaining differences do not flatter PnP

Of the 14 `geometry-differs` keys and 18 `absent-material` keys:

**PnP does more work**

| key | Orca | PnP | note |
|---|---|---|---|
| `wall_maximum_resolution` | 0.5 | 0.05 | Finding 2 |
| `wall_maximum_deviation` | 0.025 | 0.005 | Finding 2 |
| `tree_support_branch_distance` | 5 mm | 1 mm | PnP places branches 5x denser (supports-on) |
| `skirt_loops` | 1 | 6 | layer 0 only |
| `support_interface_spacing` | 0.5 | 0.4 | marginal |

**Orca does more work**

| key | Orca | PnP | note |
|---|---|---|---|
| `top_shell_layers` | 4 | 3 | one more solid top layer |
| `support_interface_top_layers` | 3 | 2 | one more dense interface layer |
| `precise_outer_wall` | 1 | 0 | already adjudicated parity work |
| `ensure_vertical_shell_thickness` | `ensure_all` | absent | vertical shell reinforcement |
| `detect_narrow_internal_solid_infill` | 1 | absent | widens solid infill into narrow regions |
| `tree_support_auto_brim` / `tree_support_brim_width` | 1 / 3 mm | absent | support base brim |

**Shape only (same amount of work), or inert on this job:** `infill_shift_step`,
`tree_support_branch_angle`, `skirt_distance`, `brim_type`, the infill/surface
patterns and directions, the flow-ratio table (all 1), and every feature family
whose own flag resolves off (`fuzzy_skin`, `seam_slope_type`, `ironing_type`,
`spiral_mode`, `raft_layers`, `support_ironing`, `hole_to_polyhole`,
`make_overhang_printable`, the `*_organic` tree style, prime tower,
`interlocking_beam`, `zaa_enabled`).

The Orca-does-more column is the one that matters for the destination: where
the two differ in that direction, the measured ratio **understates** PnP's
per-unit-work deficit rather than flattering it. Nothing in this audit makes
the existing scoreboard look better than it is.

## Observations worth recording

1. **`detect_thin_wall` declared-vs-runtime conflict.** `classic-perimeters.toml`
   declares `default = true`; `arachne-perimeters.toml` declares `false`; the
   emitted PnP config block records **0**. Orca resolves 0, so the job is
   matched — but the classic manifest's declared default is dead at runtime.
   Two modules declaring one key with different defaults is a ledger hazard the
   reconcile guard does not surface here.
2. **`support_interface_bottom_layers` declared-vs-emitted drift.** PnP's current
   manifests declare `-1`; the emitted block records `2`; the matched Orca
   resolves `0`. Direction favours PnP, but the declared value does not describe
   the job.
3. **`brim_width` is unresolved at the output level.** Orca resolves 0 (with
   `brim_type = auto_brim`), PnP resolves 8 mm, yet both captured cells emit a
   `Brim` role. The output-level difference is not established, so this is
   recorded as an open difference rather than a work claim.
4. **Overhang speed bands are live on Orca's side.** Orca resolves
   `overhang_2_4_speed = 50 / 3_4 = 30 / 4_4 = 10`, while PnP's matched config
   leaves all four bands 0. Ticket 20's consume-only-when-read candidate is
   therefore **valid for PnP's job but the two engines are not doing the same
   work**: Orca runs its overhang classifier, PnP skips it. That is a work
   difference in PnP's favour, and it should be disclosed rather than silently
   enjoyed.

## What this does not say

- No wall, CPU or ratio claim. Nothing here was timed.
- No candidate. A material finding graduates its own ticket; it is not
  implemented in this take.
- No claim that finding 2 explains the Arachne cells' wall-neutrality.
- No claim about keys that neither side resolves (they cannot affect the job).

## Recommendation

One candidate for the human: **price the Arachne tolerance match** — slice the
four Arachne cells with `wall_maximum_resolution = 0.5` /
`wall_maximum_deviation = 0.025` (canonical values) and measure Arachne module
fuel and wall against the current defaults. That is a measurement ticket with
an output-changing candidate, so it needs its own pre-declared gate and a
human KEEP/DROP, exactly like ticket 81.
