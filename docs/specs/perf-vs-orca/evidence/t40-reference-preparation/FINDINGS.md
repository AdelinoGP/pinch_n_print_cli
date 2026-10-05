# Current-job reference preparation evidence

**Prepared and frozen:** every ordinary-only reference passed the agreed
preparation checks. This is not accelerated adoption acceptance, a speed claim,
or an independent geometry oracle. The human-confirmed policy lives in
[Current-job adoption reference preparation policy](../../issues/40-acceptance-reference-refresh-policy.md).

## Observed job and output checks

The historical adoption models/configs are byte-identical in the new corpus.
The output configuration blocks disclose 0.2 mm first/normal layers, 0.5 mm
nozzle, 0.4 mm configured line width and three walls. The effective sparse fill
is **20% rectilinear** (`infill_density = 0.20000000298023224`, the promoted
`f32` default, and `sparse_fill_holder = rectilinear-infill`). The explicit
perimeter-module `sparse_infill_density = 25` is distinct; it is not 25% fill.
Padding-only `sparse_infill_pattern` disclosure is not a fill-selection oracle.
All explicit cell config settings were checked against their new disclosure.

| Workload | Generator | Completion | Fatal / non-fatal | Degraded | Printed sparse segments | Worst sparse overshoot (mm) |
| --- | --- | --- | --- | --- | ---: | ---: |
| supports-off-benchy | classic | ok | 0 / 0 | false | 2,886 | 0.0 |
| supports-off-benchy | arachne | ok | 0 / 0 | false | 3,069 | 0.0 |
| tree-support-benchy | classic | ok | 0 / 0 | false | 2,886 | 0.0 |
| tree-support-benchy | arachne | ok | 0 / 0 | false | 3,069 | 0.0 |
| tree-support-base | classic | ok | 0 / 0 | false | 25,719 | 0.0 |
| tree-support-base | arachne | ok | 0 / 0 | false | 25,899 | 0.0 |

Source: [freeze verification](attempt-2/freeze-verification.json).
Every reference has matching JSON config, G-code generator marker and scheduler
claim-holder evidence, verified using `Test-MeasurementEvidence`
(`resources/perimeter-acceptance/validate_measurement.ps1`) plus the stricter
preparation-only clean-status wrapper `validate-reference.ps1` in this directory.
Support-enabled cells also contain support TYPE evidence.

`check_output` (`prepare.py` in this directory) checks **both** endpoints of
printed sparse segments against their layer's deposited-wall bounding box.
Travel moves do not enlarge that box; missing layer/wall/sparse evidence fails
loudly. Synthetic controls accept an inside segment and reject an outside
start and an outside end. This only detects envelope overshoot: holes, concavity
and other within-box defects remain outside its claims. Repair-contract tests
provide the separate producer/validation checks; no full polygon containment or
Orca parity result is claimed.

## Prerequisites and retained evidence

- Exact allowed compiler identity verified; ordinary guest freshness exited 0
  before tests and after each snapshot build. See [prerequisite results](preflight-results.json)
  and [snapshot build/freshness provenance](attempt-2/frozen-manifest.json).
- Support-repair narrow run: **8 passed**, including
  `identity_aggregate_spread_across_the_plate_is_measured_per_body` and
  `support_body_wider_than_the_deleted_routing_cell_is_retained`
  (`crates/slicer-wasm-host/tests/contract/support_plan_validation.rs`).
  [Full log](support-repair.full.log).
- Empty-infill producer-boundary run: **6 passed**, covering both legs, merge
  contrasts and raft-only output
  (`crates/slicer-wasm-host/tests/contract/infill_postprocess_empty_commit_tdd.rs`).
  [Full log](empty-infill-producers.full.log).
- Gated runtime contract run: **9 passed**, including
  `infill_postprocess_empty_replacement_supersedes_prior_ir` and the in-boundary
  positive control `infill_postprocess_inside_path_survives_so_the_clip_is_real`
  (`crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs`).
  [Full log](empty-infill-runtime.full.log).
- The runtime `cargo xtask test --summary` wrapper passed literal preflight,
  reported existing test-quality findings in report mode, checked guests and
  rebuilt its stale debug CLI. Those reported files are untouched. Its injected
  Arachne-parity skip does not intersect this filter; all named required infill
  regressions executed. This is not a claim that packet-254/full-workspace gates
  passed. [Wrapper log](empty-infill-runtime.log).
- Rust test output was teed to `target/test-output.log`, read from disk, and
  copied here before the next test could replace it. Cargo features/target gates
  were inspected; required named successes are enforced by `run` (`preflight.py`).

## Preparation failure and authorized new attempt

The first attempt built/staged a fresh ordinary snapshot, then failed **before
any slice**: `corpus_files` used Windows backslash keys while input verification
looked up `supports-off-benchy/model.stl`. Its blocked state, source and corpus
hashes, and exact original script remain preserved here:
[blocked state](preparation-state.json), [original script](attempt-1-prepare.py).

The human explicitly authorized a new attempt after this stop. `relative_key`
(`prepare.py`) now produces/looks up portable POSIX-relative manifest keys.
`test-preparation.py` proves the original Windows input fails against the old
script and passes against the repaired script; it also checks the real corpus
inventory and the overshoot controls. [Red/green results](path-regression-results.json).
The successful new attempt has separate evidence and output directories; this
was not an automatic retry or an acceptance campaign retry.

## Freeze and handoff

The frozen manifest records commands, compiler/policy identity, source revision
and dirty-tree evidence, build-input hashes, ordinary executable/module hashes,
input/config/reference/completion-sidecar hashes and per-cell output checks.
Generation used 12 threads, no timing harness, and explicit isolated snapshot
modules with `--no-default-module-paths --no-integrated-modules`. No environment
module path or acceleration/build override was present. Existing dist output
was copied aside before rebuilding; historical corpus hashes are unchanged.
`target/dist/developer/` was refreshed as an intended build side effect; both
attempt snapshots and preserved prior dist copies stay in the ignored target
tree. Licensed models and raw G-code were not added to tracked evidence.

The [frozen manifest](attempt-2/frozen-manifest.json) and its
[digest](attempt-2/frozen-manifest.sha256) are authoritative for the actual local
corpus/snapshot paths. In this preparation they are:

```text
target/perimeter-reference-preparation/t40-20260930T065207Z/corpus/
target/perimeter-reference-preparation/t40-20260930T065207Z/ordinary/
```

Before using these references, and again after a campaign, run the read-only
identity gate; do not silently regenerate missing or changed inputs:

```bash
python docs/specs/perf-vs-orca/evidence/t40-reference-preparation/verify-freeze.py docs/specs/perf-vs-orca/evidence/t40-reference-preparation/attempt-2
```

The gate passed against actual frozen files and source inputs. Negative controls
reject a changed manifest and disabled Python assertions without modifying any
frozen file: [control results](freeze-control-results.json).
The [final identity verification](attempt-2/freeze-verification-final.json) also
passed after tracker updates. Python syntax, local evidence/tracker links and
`git diff --check` passed; no all-target build/clippy or full adoption test result
is implied by these final checks.
Pass the manifest's **new corpus root explicitly** as `-CorpusRoot` when the
separate acceptance ticket is taken; the runner's default root still contains
the deliberately preserved historical references. Campaign preparation must
also establish the actual loaded module set on its ordinary/accelerated legs,
not silently add modules from environment/platform paths.

No accelerated validation, current ordinary-versus-accelerated exactness,
campaign timing, full packet/all-target gate, automatic commit or default change
occurred. Ordinary remains the actual default. Remaining work belongs to
[Accelerated production adoption acceptance campaign](../../issues/38-accelerated-adoption-acceptance-campaign.md).
