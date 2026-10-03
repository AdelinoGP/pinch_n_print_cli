# Modal Z/F token emission — DROP / inconclusive

Route ticket: [Modal G-code Z/F token redundancy and speed gate](../../issues/36-modal-gcode-token-redundancy.md).

**DROP / inconclusive for this keep/drop experiment.** The candidate is
semantically equivalent to the baseline on every measured batch and removes
27.0–27.1% of G-code bytes, but it fails the pre-declared keep gate in 3 of the
4 protocol batches: median wall improves slightly (0.3–1.7%) while process CPU
does **not** corroborate — the candidate's median CPU is higher (slower) in
three of the four batches, and in the one batch whose paired median CPU delta is
positive (classic-off accelerated, +0.109 s) it contradicts the wall direction
outright. This is a formatting/shape result, not a demonstrated speed win. No
commit, no production change was made by this take.

**Human decision (2026-10-03): DROP.** The gate was pre-declared and failed in
three of four batches, and a ~1% wall effect cannot matter against the map
destination's 6.28–26.24x gap. See the ticket's audit section for the
independent re-derivation of these figures.

## What the candidate does

`DefaultGCodeSerializer::serialize_gcode`
(`crates/slicer-gcode/src/serialize.rs`) now carries a `ModalZfState` cache: a
`Z` or `F` word is written on a `G0`/`G1` line only when its **formatted** value
differs from the last emitted one (formatted comparison matches what the printer
actually receives). Retract/unretract lines elide `F` under the same rule. A
move whose operands all match modal state still occupies its line, rendered as a
bare `G0`/`G1` no-op, so one IR move stays one output line.

Invalidation is deliberately conservative: any non-comment `Raw` command and any
`ToolChange` clear the cache (a firmware macro or raw motion can change Z, feed
or coordinate mode invisibly), so the next move re-states both. Comment-only
raws (`;TYPE:` / `;LAYER_CHANGE` / `;Z:` / `;HEIGHT:`) and `ExtrusionMode`
(M82/M83, E only) do **not** invalidate.

The implementation and its tests are **not committed and not left in the
working tree**: the candidate is preserved as
[`source-worktree.diff`](source-worktree.diff), a patch applied against HEAD
`ce4a8906` (the tree at the campaign's start). It exists for inspection and for
a future byte/bandwidth-bound reason to revisit; re-apply it with
`git apply source-worktree.diff`.

## Semantic equivalence (independent oracle)

[`verify-t36.py`](verify-t36.py) is a stdlib-only, independent G-code interpreter:
it tracks XYZ/E absolute/relative modes (G90/G91, M82/M83), G92 resets, sticky Z/F,
retract/unretract and raw commands, and compares the **physical move stream**, not
bytes. `--falsify` mutates the candidate and must be detected (proven).

Every measured batch is semantically identical, with token counts collapsing and
bytes falling ~27%:

| batch | normalized stream SHA-256 | move lines | Z tokens | F tokens | bytes | reduction |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off (both modes) | `f49b5ec57c7a0c03…` both arms | 105,549 = 105,549 | 105,070 → 520 | 105,549 → 1,039 | 4,332,881 → 3,158,453 | **27.10%** |
| benchy arachne-off (both modes) | `c237fb0187eeb07c…` both arms | 100,195 = 100,195 | 99,716 → 521 | 100,195 → 955 | 4,126,888 → 3,013,383 | **26.98%** |

E token counts and line counts are also identical per pair. The oracle's raw
counts, per-row disclosure and both mode-specific proof batches are in
[`evidence.json`](evidence.json).

### Baseline provenance

The ordinary baseline output SHA-256 for classic-off is
`e1088b3df78b51b4d0add0c64b3cf143d44ca95cfb5c15db14080772b4e77fbf`, matching
ticket 47's post-repair classic-off reference exactly (integrated/external gate,
2026-10-03), and the benchy model hash is `6a07f34c…` as in t44. This take's
baseline is therefore the current post-repair tree, not a stale pre-repair
capture.

## Standing paired A/B

Protocol declared before measurement in [`protocol.json`](protocol.json):
six measured pairs per batch, one excluded warmup per arm, 12 threads, AB/BA
alternation by pair, one warmup per arm. Starvation policy: below 0.75 × best
CPU/wall **within the same batch and arm**, exclude the whole pair; retain all
excluded rows; require at least five retained pairs; no automatic resampling.
No pair was excluded in any batch (all six retained everywhere).

| batch | baseline median wall | candidate median wall | wall Δ | baseline median CPU | candidate median CPU | CPU Δ |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| classic-off ordinary | 16.655 s | 16.602 s | −0.3% | 118.891 s | 119.375 s | **+0.4%** |
| classic-off accelerated | 16.403 s | 16.136 s | −1.6% | 99.750 s | 99.781 s | **+0.03%** |
| arachne-off ordinary | 14.413 s | 14.168 s | −1.7% | 68.258 s | 67.750 s | −0.7% |
| arachne-off accelerated | 14.279 s | 14.207 s | −0.5% | 66.250 s | 66.273 s | **+0.04%** |

Paired median candidate-minus-baseline deltas (wall / CPU, seconds):

| batch | wall | CPU | pairs favoring candidate (wall / CPU) |
| --- | ---: | ---: | --- |
| classic-off ordinary | −0.032 | −0.063 | 3/6 / 3/6 |
| classic-off accelerated | −0.101 | +0.109 | 3/6 / 3/6 |
| arachne-off ordinary | −0.197 | −0.508 | 4/6 / 5/6 |
| arachne-off accelerated | −0.100 | −0.242 | 5/6 / 4/6 |

Only **arachne-off ordinary** passes both pre-declared gate clauses (candidate
median wall and CPU both lower, paired median deltas both negative). Every other
batch fails at least one clause — most decisively classic-off accelerated, where
the CPU paired delta is positive while wall is negative. No batch has strict
range separation in either metric. The candidate's measured CPU/wall ranges are
not uniformly higher than the baseline's; the signal is small relative to run
noise at this effect size.

Measured per-sample CPU/wall ranges per batch and arm, every warmup and
measured row, input hashes (model + both configs), and the full 102-file
snapshot inventory are in [`evidence.json`](evidence.json) (reduce with
`python verify-t36.py --reduce target/t36-ab`; verify tracked evidence
standalone with `python verify-t36.py`). The reducer independently asserts
arm isolation: within each mode, `pnp_cli.exe` is the only file that differs
between baseline and candidate.

## Output disclosure

- `status=ok`, `degraded=false`, `fatal=0`, `non_fatal=0` for all 56 runs in the
  campaign (both arms, both modes, both cells).
- Existing open-loop warnings are unchanged baseline vs candidate: 0 in every
  classic-off output, 12 in every arachne-off output. `degraded=false` is not
  claimed to prove their absence.
- Proved generator markers: classic and arachne claim holders validated by
  `Test-MeasurementEvidence` on every run.
- No probe or instrumentation events; runs are uninstrumented.

## Why this result

The candidate removes 99,195–104,550 Z tokens and 99,240–104,510 F tokens per
benchy slice, yet process CPU — the work signal — does **not** fall in 3 of
4 batches and the wall change is within this job's own measured run-to-run
spread (measured here: per-arm wall min–max spans of 0.38–1.41 s across the
measured samples of a single batch/arm; the largest candidate wall win is
0.20 s). The serialization work
(a `format!` per token avoided and ~1.17 MB less output on classic-off) is small
against a ~16 s benchy slice dominated by module execution; the file write and
scan tail shrink, but not enough to clear the agreed CPU-corroboration bar. This
is consistent with ticket 28's note that the token budget is a formatting
result, not a speed result.

## Recommendation

**DROP / inconclusive — human approved DROP 2026-10-03.** Keep the finding
(modal emission is provably semantics-preserving and removes ~27% of bytes) but
do not keep the change on this evidence: it does not pass the map's standing
keep gate (median wall and corroborating process CPU in every cell). The
candidate diff stays uncommitted; if a future byte- or bandwidth-bound consumer
matters (e.g. SD-write time, serial transmission, viewer load), this
semantics-preserving implementation and oracle are ready and evidence-backed —
that would be a separate decision needing its own justification, and would
carry the `docs/03_wit_and_manifest.md` packet-52 note and the golden emit-test
updates with it.

## Reproduce

```powershell
# Candidate build (source state in source-worktree.diff)
cargo build --release --bin pnp_cli
cargo xtask dist --accelerated --edition developer

# Campaign (4 batches; ordinary classic-off already archived under the batch id)
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t36-modal-zf/run-campaign.ps1 -BatchId <id>

# Oracles and reduction
python docs/specs/perf-vs-orca/evidence/t36-modal-zf/verify-t36.py --oracle <baseline.gcode> <candidate.gcode> --falsify
python docs/specs/perf-vs-orca/evidence/t36-modal-zf/verify-t36.py --reduce target/t36-ab
python docs/specs/perf-vs-orca/evidence/t36-modal-zf/verify-t36.py
```

Snapshots the arms were measured from:
`target/t36-ab/snapshots/{ordinary,accelerated}-{baseline,candidate}/`
(executables `192f3529…`, `e09d4d72…`, `b436fa61…`, `86efd7e0…`). Raw captures
live under `target/t36-ab/` (gitignored); the tracked evidence is self-contained.
