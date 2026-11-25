# Ticket 125 — per-origin occupancy batching is output-exact but fails the paired gate

Route: [Strong planner occupancy-offset batching: representation-safe four-cell campaign](../../issues/125-strong-planner-occupancy-offset-batching.md).

## Verdict

**DROP.** The candidate batched only the **per-origin role-carve** occupancy
offsets inside the existing bounded emit flush; the per-layer emit-check offsets
remained singular. Output was byte-identical in every arm and cell, but only two
of eight cells passed the human-amended gate. The candidate source and WASM were
retired after measurement; the patch is preserved as
[`candidate.patch`](candidate.patch).

No bridge change, per-layer batching change or other rescue was folded in after
the gate failed. No Orca scoreboard was refreshed because nothing landed.

## Output and provenance proof

The output campaign contains **32 runs**: eight cells × four arms
(baseline/candidate × ordinary/accelerated). The paired campaign contains **48
runs**: three interleaved pairs × eight cells × two ordinary arms. All **80**
campaign runs exited 0 with `completion_status="ok"`, `degraded=false` and
`non_fatal_error_count=0`; every generator/claim-holder validation passed. There
were **zero starvation exclusions and zero discarded campaign pairs**.

- In every cell, baseline ordinary ≡ baseline accelerated ≡ candidate ordinary ≡
  candidate accelerated, and all four hashes equal the post-t123 reference.
- The ordinary host binaries are both
  `7737a74fd9f7ea012681ba83e9afdcf511d067fc50a0f919cef8b877b35759b0`;
  the accelerated host binaries are both
  `a6ae2adc998572217664a124ac0ac62c58977cfd46804379dcf5153fbf53e41b`.
  Only the planner WASM differs between arms.
- Baseline/candidate ordinary planner WASM:
  `a53e2df38778b7f29e1e27332a020861709b6c978d3ea18367fad9c95b68bcad` →
  `9ce369d3b20e8abd3e7d58670f4f38e46d2c5e1eb7044c99133853a3e5942e9a`.
- Baseline/candidate accelerated planner WASM:
  `817ed9a4583d9e5502c8127d341fbced77100416025a33315f16de3c55cdcad3` →
  `33554f970cd167d77728bafb745e34cf8fc8c02b9a7ea2b63222b2ea28d2e38d`.
- Both freshness checks exited 0 before the campaigns. After the source was
  restored, both guests rebuilt and the checks again exited 0; the in-tree
  ordinary planner WASM is back at the baseline hash above.

The raw rows, G-code and stderr remain under gitignored `target/t125/`. The
committed reduced observations are [`reduced/summary.json`](reduced/summary.json);
[`verify.py`](verify.py) re-derives the output identity, provenance, medians and
gate verdict checkout-only.

## Paired gate

Wall and CPU are medians of three alternating, interleaved ordinary-arm pairs.
Wall improvement is baseline minus candidate, so positive values favour the
candidate. The floor is 0.80 % of each cell's own same-run baseline median wall.
Supports-on cells require the floor and a CPU decrease; supports-off cells
require no median wall or CPU regression.

| cell | baseline wall | candidate wall | wall delta | floor multiple | CPU delta | gate |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| benchy classic-off | 5.9155 s | 5.9267 s | **−0.19 %** | −0.24x | +1.2031 s | **FAIL — wall regressed** |
| benchy classic-on | 11.3130 s | 13.3377 s | **−17.90 %** | −22.37x | +0.1406 s | **FAIL — below floor** |
| benchy arachne-off | 6.9882 s | 7.1803 s | **−2.75 %** | −3.44x | −0.1718 s | **FAIL — wall and CPU regressed** |
| benchy arachne-on | 12.6517 s | 12.4121 s | **+1.89 %** | +2.37x | +0.5782 s | PASS |
| base classic-off | 65.0858 s | 63.6771 s | +2.16 % | +2.71x | −0.2813 s | **FAIL — CPU regressed** |
| base classic-on | 162.3938 s | 152.4322 s | +6.13 % | +7.67x | −1.7656 s | **FAIL — CPU contradicted wall** |
| base arachne-off | 50.1856 s | 51.7064 s | **−3.03 %** | −3.79x | +0.9532 s | **FAIL — wall regressed** |
| base arachne-on | 140.6966 s | 125.3925 s | **+10.88 %** | +13.60x | +2.3125 s | PASS |

The base classic-on wall spread is visible in the committed per-pair samples;
the gate was neither widened nor retried selectively to accommodate it. The
verdict uses exactly the pre-declared median rule. Machine load was not sampled,
so no idle-box or causal occupancy-fan-out claim is made. The paired wall/CPU
effect is measured; a separate batch-occupancy timing probe was not taken after
the gate failed, and ticket 78's 2.6x is not quoted as this request set's price.

## Tests and retirement

While the candidate was active:

- Two focused tests proved empty/single/multiple input alignment, exact equality
  with the singular offset path, and the non-positive-distance no-op.
- `cargo test --manifest-path modules/core-modules/tree-support-planner/Cargo.toml`
  passed **148 tests**: 64 unit and 84 integration tests.

The candidate was then reverted manually (no branch or checkout operation), the
baseline guests were rebuilt, and the production source diff became empty. The
campaign driver [`run_t125.ps1`](run_t125.ps1), reducer
[`reduce_t125.py`](reduce_t125.py), patch, reduced summary and verifier preserve
the experiment. Reproduction and artifact layout are in
[`REPRODUCE.md`](REPRODUCE.md).
