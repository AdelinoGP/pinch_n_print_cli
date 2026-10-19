# Ticket 21 — evidence pointers

The authoritative write-up is [FINDINGS.md](FINDINGS.md). This file exists only
to index the artifacts in this directory.

| artifact | what it is |
|---|---|
| `FINDINGS.md` | the attributed split, candidate, validation, gaps |
| `captures/` | in-repo reduced captures + `output-hashes.json`; `verify-t21.py` runs against these by default |
| `reduce_captures.py` | regenerates `captures/` from the durable raw tree |
| `probe.patch` | the `T21-PROBE` instrumentation (applies cleanly to the reviewed tree; stashed out, not committed) |
| `t21_split.py` | reduces one probe capture to the per-layer/split tables |
| `verify-t21.py` | read-only re-derivation of every FINDINGS headline (exit 0 = all pass) |

Raw captures (full set, durable, gitignored):
`.local-artifacts/perimeter-reference-preparation/t21-linker-subcost-run1/`.
The reduced in-repo set is derived from these by `reduce_captures.py`; the
reduction keeps `T21-PROBE` lines, `profile_summary` events, and
`module_complete` events for `com.core.infill-linker`.

| path | capture |
|---|---|
| `ordinary/profile.jsonl` | capture 1 — frozen everything, baseline split |
| `ordinary/probe-profile.jsonl` | capture 2 — probe build, boundary + sub-ops |
| `ordinary/probe2-profile.jsonl` | capture 3 — in-`clip_polylines` sub-split |
| `accelerated/probe2-profile.jsonl` | capture 4 — accelerated mode |
| `*/probe2-output.gcode` etc. | every output byte-identical to the frozen reference |

Probe module dirs (`probe-modules`, `accel-probe-modules`) hold 23 frozen
modules plus the one probe linker wasm.

Frozen inputs used by every capture:
`t38-campaign-20261001T024827Z/{ordinary,accelerated}/` and
`t41-20260930T232255Z/corpus/supports-off-benchy/`.
