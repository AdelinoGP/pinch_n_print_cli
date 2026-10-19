# Issue 38 campaign — attempt 1 findings

- **FACT outcome:** BLOCKED
- **Runner status / actual campaign exit:** inconclusive / 0
- **Reason:** Runner decision was recorded, but post-run integrity or evidence preservation failed.
- **First stopping cell / cause:** {'workload': 'supports-off-benchy', 'generator': 'arachne', 'decision': 'inconclusive', 'cause': 'CPU or wall measured ranges overlap or touch'}
- **Retained measured rows / paired sample count:** 16 / 8
- **Timing status:** retained measured rows summarized.
- **Post-campaign t41 and frozen-input identity:** PASS
- **Raw archive:** `D:\slicerProject\pinch_n_print_cli_2\.local-artifacts\perimeter-reference-preparation\t38-campaign-20261001T024827Z\campaign-raw-attempt1` (copied)
- Synthetic status-roundtrip and dry-run values are controls only and are excluded from all timing results.
- CPU and wall statistics below are computed only from retained measured rows in the real campaign summary; exactness and warmup rows are excluded.

| Cell | Decision | Exactness | Retained B/C | Status B/C | Marker B/C | Degraded B/C | Nonfatal B/C | Fatal B/C | CPU B/C (s: range; median) | Wall B/C (s: range; median) | CPU / wall ratio | Exactness deltas |
|---|---|---|---:|---|---|---|---|---|---|---|---|---|
| supports-off-benchy / classic | KEEP | pass | 4/4 | ok/ok | classic/classic | 0/0 | 0/0 | 0/0 | 144.7188–146.0938; median 145.0625 / 118.3125–119.8125; median 119.6016 | 20.6938–21.3761; median 20.8286 / 19.7013–20.0700; median 19.9081 | 0.8244828953037484 / 0.9558083596593144 | `{"baseline":{"lines_pct":0.0159,"types_pct":0.3607,"e_pct":0.0034},"candidate":{"lines_pct":0.015,"types_pct":0.3607,"e_pct":0.0024}}` |
| supports-off-benchy / arachne | inconclusive | pass | 4/4 | ok/ok | arachne/arachne | 0/0 | 0/0 | 0/0 | 71.7344–73.6719; median 72.8672 / 67.9375–69.9062; median 69.5938 | 16.5563–17.4910; median 17.1731 / 16.9912–17.1664; median 17.1163 | 0.9550764953229985 / 0.9966925112748679 | `{"baseline":{"lines_pct":0.0,"types_pct":0.0,"e_pct":0.0},"candidate":{"lines_pct":0.0,"types_pct":0.0,"e_pct":0.0}}` |
| tree-support-benchy / classic | not-run | not-run | 0/0 | / | / | 0/0 | 0/0 | 0/0 | not run / not run | not run / not run | None / None | `null` |
| tree-support-benchy / arachne | not-run | not-run | 0/0 | / | / | 0/0 | 0/0 | 0/0 | not run / not run | not run / not run | None / None | `null` |
| tree-support-base / classic | not-run | not-run | 0/0 | / | / | 0/0 | 0/0 | 0/0 | not run / not run | not run / not run | None / None | `null` |
| tree-support-base / arachne | not-run | not-run | 0/0 | / | / | 0/0 | 0/0 | 0/0 | not run / not run | not run / not run | None / None | `null` |

Machine-readable cell measurements and commands: [`results.json`](results.json).
Synthetic dry-run summary (not acceptance evidence): [`dry-run.synthetic.summary.json`](dry-run.synthetic.summary.json).
Campaign runner summary: [`campaign-summary.json`](campaign-summary.json) when present.
Tracked raw logs/CSV/process diagnostics exclude licensed G-code; complete raw output is retained only in the durable archive.
