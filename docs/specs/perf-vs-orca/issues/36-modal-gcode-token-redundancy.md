# Modal G-code Z/F token redundancy and speed gate

Type: task
Status: resolved (2026-10-03; measured DROP/inconclusive recommending drop —
**human approved DROP 2026-10-03** — see ## Answer)
Blocked by: 28

## Claim record

Claimed 2026-10-03 by a wayfinder session after the human chose this as the
next map take from the open frontier. Scope: prove a semantics-preserving modal
Z/F state machine through `GCodeSerializer`'s move rendering with an
independent G-code semantics oracle on tricky fixtures, then run the standing
matched ordinary + accelerated pairs and return a measured keep/drop
recommendation. No auto-commit; production/default-mode changes need their own
authorization.

## Question

Does emitting modal Z/feed state only when necessary preserve the printing
job, and does it improve **median uninstrumented wall with corroborating
process CPU**? [Classic output-volume surplus](28-classic-output-volume-surplus.md)
found that PNP repeats Z and F on every XY+E move in both generators,
including when unchanged; historical Orca captures generally omit them.
The measured Z/F token budget is **not** a safe-removal count or a time saving.

First prove a semantics-preserving state machine through `GCodeSerializer`'s
move rendering (`crates/slicer-gcode/src/serialize.rs`), including travel,
retraction, raw G-code, G90/G91, G92, layer changes and tool commands. Build an
independent G-code interpreter/round-trip oracle on tricky fixtures; the
output itself will be byte-different, so a byte-equality check is insufficient.
Then run matched ordinary+accelerated pairs with the map's freshness, starvation
and output-disclosure gates and return a measured keep/drop recommendation to
the human. Do not infer a speed win from file-size arithmetic alone. Evidence:
`evidence/t28-output-volume/FINDINGS.md`.

## Answer — DROP / inconclusive: provably semantics-preserving, −27.0–27.1% bytes, but fails the CPU-corroboration gate in 3 of 4 batches

The candidate (`ModalZfState` in `DefaultGCodeSerializer::serialize_gcode`,
`crates/slicer-gcode/src/serialize.rs`) states a `Z`/`F` word only when its
formatted value changes; retract/unretract lines elide unchanged `F`; a move
that changes nothing renders as a bare `G0`/`G1` no-op so one IR move stays one
line. Non-comment `Raw` commands and `ToolChange` invalidate both caches
(firmware macros can change state invisibly); comment-only raws and
`ExtrusionMode` do not. The candidate is **not committed and not left in the
working tree**: it is preserved as `source-worktree.diff` (a patch against HEAD
`ce4a8906`), and the evidence base is `evidence/t36-modal-zf/`.

An independent stdlib-only interpreter (`verify-t36.py`; XYZ/E modes, G92,
sticky Z/F, retract, raw; `--falsify` proves it detects sabotage) declares every
measured pair physically identical by normalized move-stream SHA — classic-off
`f49b5ec5…` both arms, arachne-off `c237fb01…` both arms — with E tokens and line
counts unchanged, Z tokens 105,070→520 / 99,716→521 and F 105,549→1,039 /
100,195→955. Bytes drop **27.10%** (4,332,881→3,158,453) and **26.98%**
(4,126,888→3,013,383). Baseline classic-off SHA `e1088b3d…` matches ticket 47's
post-repair reference; model hash `6a07f34c…`.

Standing paired A/B, protocol declared before measurement (`protocol.json`):
1 warmup + 6 measured pairs per arm per batch, 12 threads, AB/BA alternation,
0.75× best-ratio paired starvation rule (no pair excluded), both modes, both
support-off benchy cells. Paired median candidate−baseline deltas (wall / CPU):
classic-off ordinary −0.032 / −0.063 s, but candidate median CPU is higher
(119.375 vs 118.891 s, +0.4%) → fails the median clause; classic-off accelerated
−0.101 / **+0.109** s → fails both clauses; arachne-off ordinary −0.197 /
−0.508 s → **passes both clauses** (4/6 wall and 5/6 CPU pairs favor the
candidate); arachne-off accelerated −0.100 / −0.242 s, but candidate median CPU
is higher (66.273 vs 66.250 s, +0.04%) → fails the median clause. Wall effects
(0.3–1.7%) sit inside the job's own measured spread (0.38–1.41 s per-arm spans),
and CPU does not corroborate 3 of 4 batches. **No fix, commit or production
change was made; recommendation: DROP/inconclusive.** If a byte/bandwidth-bound
consumer matters later, the implementation and oracle are ready and
evidence-backed. Evidence: `evidence/t36-modal-zf/` (`FINDINGS.md`,
`evidence.json`, `verify-t36.py`, `protocol.json`, `run-ab.ps1`,
`run-campaign.ps1`, `source-worktree.diff`; raw captures under gitignored
`target/t36-ab/`).

## Audit and human decision (2026-10-03, second session)

An independent review session re-derived every headline number from the raw
captures with its own token census and per-move effective-Z/F walk (not
`verify-t36.py`), and replayed both arms from the preserved sources:

- Arm outputs reproduce byte-for-byte from `source-worktree.diff` plus
  `git apply` and a fresh `--release` build: baseline classic-off
  `e1088b3d…`, candidate classic-off `132f9501…`, candidate arachne-off
  `8099ce5c…`; the frozen snapshots reproduce `c27461333c54eac6…` (arachne
  baseline). `verify-t36.py --reduce target/t36-ab` regenerates the committed
  `evidence.json` byte-for-byte; `--self-test` and `--oracle … --falsify` pass.
- Byte, token, line and stream-SHA figures re-derive exactly, as do all four
  batches' medians, paired deltas, gate outcomes and zero starvation
  exclusions. `gcode_modal_zf_tdd` 7/7, full `slicer-gcode` suite green, no
  `check-literals` or `check-test-quality` findings in touched files.
- One prose defect found in `FINDINGS.md` (the opening gist said the paired
  median CPU delta was positive in all three failing batches; only
  classic-off accelerated is positive) and corrected there; the tables and
  this answer were already right.
- Environment repaired: the take had left candidate-built binaries in the
  standing measurement dirs (`target/dist-accelerated/developer/` matched the
  campaign's accelerated-candidate snapshot `86efd7e0…` and emitted
  `132f9501…`; `target/debug/pnp_cli.exe` likewise). Both were rebuilt from
  HEAD and now emit the `e1088b3d…` baseline; both guest-freshness checks
  still exit 0.

**Human decision (2026-10-03): DROP.** The gate was pre-declared and failed in
three of four batches, and a ~1% wall effect cannot matter against the map
destination's 6.28–26.24x gap ([Gap budget per cell](12-gap-budget-per-cell.md)).
The candidate remains uncommitted; the semantics-preserving implementation and
oracle stay preserved as `source-worktree.diff` for a future byte- or
bandwidth-bound consumer, which would be a separate decision with its own
justification (and would need `docs/03_wit_and_manifest.md`'s packet-52 note
and the golden emit tests updated with it).

**Related red test (not caused by this ticket):**
`machine_start_end_gcode_emission_tdd::empty_end_gcode_emits_no_block`
(`crates/slicer-runtime/tests/integration/`) fails at HEAD — reproduced by
this audit on a clean tree with the candidate reverted. The offending line is
a trailing `M107` in the span after the last `G1`, i.e. ticket 47's intentional
external fan-command relay, so the assertion now needs re-scoping to exclude
fan commands. Tracked here only as a disclosure; no fix was made.
