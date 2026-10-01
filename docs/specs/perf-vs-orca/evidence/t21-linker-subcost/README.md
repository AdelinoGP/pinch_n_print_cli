# Ticket 21 — evidence pointers

The authoritative write-up is [FINDINGS.md](FINDINGS.md). This file exists only
to index the artifacts in this directory.

| artifact | what it is |
|---|---|
| `FINDINGS.md` | the attributed split, candidate, validation, gaps |
| `probe.patch` | the `T21-PROBE` instrumentation (applies cleanly to the tree at d6be6203; stashed out, not committed) |
| `t21_split.py` | reduces one probe capture to the per-layer/split tables |
| `verify-t21.py` | read-only re-derivation of every FINDINGS headline from the raw captures (exit 0 = all pass) |

Raw captures (durable, gitignored):
`.local-artifacts/perimeter-reference-preparation/t21-linker-subcost-run1/`

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
