# Ticket 25 — evidence pointers

The authoritative write-up is [FINDINGS.md](FINDINGS.md). This file exists only
to index the artifacts in this directory.

| artifact | what it is |
|---|---|
| `FINDINGS.md` | the re-attribution, the one candidate, validation, gaps |
| `t25_split.py` | reduces one host-probe stderr capture to the stage tables |
| `verify-t25.py` | read-only re-derivation of every FINDINGS headline from the raw captures (exit 0 = all pass) |
| `host-split.txt` | the reduced host-side stage table for `attrib8-final` |
| `probe.patch` | the temporary `[T25-PROBE]` host/guest instrumentation (tracked-file diff; applies to `5e0e2fdb` + the claim commit; removed before any commit) |
| `perf_t25_probe.rs.txt` | the probe module source (untracked file; not in `probe.patch`) |
| `t25_probe_quartile_shape_tdd.rs.txt` | probe test: quartile-query cost scales with band-polygon size |
| `t25_probe_winding_pass_share_tdd.rs.txt` | probe test: the `eps=0` boundary pre-pass share of the predicate |
| `arachne-perimeters-lib.probe.rs.txt` | the guest module source with the `t25::*` user scopes |

Raw captures (durable, gitignored):
`.local-artifacts/perimeter-reference-preparation/t25-arachne-attribution-run1/`

| path | capture |
|---|---|
| `ordinary/attrib-output.gcode`, `attrib2`, `attrib3` | early host-probe builds, output byte-identical |
| `ordinary/attrib4-inst.stderr.txt` | instrumented, host probe only (same-run module split) |
| `ordinary/attrib5-prof.stderr.txt` | `--profile`, host probe only |
| `ordinary/attrib6..8.stderr.txt` | guest scopes added; `attrib8-final` is the reduced reference |
| `ordinary/attrib9-verbose.stderr.txt` | `--profile-verbose` (per-layer guest scopes) |
| `ordinary/attrib10-verify.stderr.txt` | final probe build, instrumented + `--profile` (verify-t25 reference) |
| `accelerated/attrib.stderr.txt` | accelerated module set + accelerated host snapshot, instrumented + `--profile` |
| `host/pnp_cli-probe{2,3,4}.exe` | the probe host binaries (build #4 is the final one) |

Frozen inputs used by every capture:
`t38-campaign-20261001T024827Z/{ordinary,accelerated}/` and
`t41-20260930T232255Z/corpus/supports-off-benchy/`.

The accelerated probe guest was built with `cargo xtask build-guests
--accelerated` and the accelerated host snapshot with `cargo xtask dist
--accelerated`; both were rebuilt to production after the take.
