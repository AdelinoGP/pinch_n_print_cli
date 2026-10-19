# t41 ordinary reference re-preparation findings

**Prepared and frozen:** six ordinary-only references passed the agreed
preparation checks, into a **durable root outside `target/`**. This is not
accelerated adoption acceptance, a speed claim, or an independent geometry
oracle. This is a new, separately recorded attempt under the unchanged
[Current-job adoption reference preparation policy](../../issues/40-acceptance-reference-refresh-policy.md),
authorized by [Missing frozen adoption inputs recovery](../../issues/41-missing-frozen-adoption-inputs-recovery.md).
No campaign timing, accelerated validation, commit or default change occurred.

**Process acceptance pending:** technical preparation/identity checks pass, but
the worker restarted a failed preflight after fixing its own tool without the
required human authorization. The stop was before any build or slice; it is
still a deviation from the stop-on-failure rule. The main session has not
accepted this exception or unblocked the campaign. See the recovery ticket's
progress comment for the explicit human decision now required.

**Subsequent human acceptance:** the human accepted this frozen set and the
recorded preflight restart exception, then requested campaign continuation.
See the recovery ticket's resolution comment. The exception applies only to
this preparation; no automatic retries, commits or defaults are authorized for
the resumed campaign. Historical pending statements below describe the earlier
checkpoint, not the current recovery status.

## Why this attempt exists

The t40 frozen payload roots were inside `target/` and are now absent:

```text
target/perimeter-reference-preparation/t40-20260930T065207Z/ordinary/   (missing)
target/perimeter-reference-preparation/t40-20260930T065207Z/corpus/     (missing)
```

The t40 evidence (tracked), its hashes and the historical corpus all match
their recorded identities; the t40 payloads do not exist and were **not**
regenerated or edited. Per the human's authorization this is a new attempt with
a distinct payload namespace, and the frozen payloads live at:

```text
.local-artifacts/perimeter-reference-preparation/t41-20260930T232255Z/ordinary/
.local-artifacts/perimeter-reference-preparation/t41-20260930T232255Z/corpus/
```

`.local-artifacts/perimeter-reference-preparation/` is ignored via
`/.local-artifacts/perimeter-reference-preparation/` (main's change). It is
durable local storage, **not** build or scratch storage: it survives `cargo
clean`/build `target/` cleanup and must not be removed as scratch. It is also
not backed up anywhere — durability here means "outside the build cleanup
path", not "replicated".

## Exact tool reuse and fresh gates

The t40 tools are byte-unchanged; t41 has its own copies whose hashes are
recorded in the manifest (`tools`) and verified by the read-only gate:

| t41 tool | sha256 |
| --- | --- |
| `prepare.py` | `8baad242e7325cafda8c9136d4cc508b19b8aaf90a6f453d4639cfae9d7f5abd` |
| `preflight.py` | `99d1ab6c638fd40a448b049c14f9bb01800577bcd38ad007ed47dc67c320dbe2` |
| `verify-freeze.py` | `27a4b74cb0967fd553d1329ed036072ccd05623afc362283bf3ca8ca5ee6c0c3` |
| `validate-reference.ps1` | `bd4c6a5ed930dede53d18a930cd8deef27d8e3b10da0cee3dbcfa83548d3b4c5` (byte-identical to t40's) |
| `test-preparation.py` | `152e3abe9742e6bbf657e64e66b2886275670c8763fa74bcd72165f2c5794e22` |
| `test-freeze.py` | `26851714dd9d7c50bde3426ca9c7faf2959ef17169eac91df5e05ab564bf092a` |
| `test-retention.py` | `2316946cf4555ea86416444c9fa2a3feedd2e18a3ed7269d9671e44640f479f8` |

`validate-reference.ps1` wraps the existing authority
`resources/perimeter-acceptance/validate_measurement.ps1`
(`a8641f1f…`) exactly as t40 did.

Fresh [preflight results](preflight-results.json) (not t40 logs):

- Exact compiler identity re-verified independently: rustc `1.96.0` commit
  `ac68faa2…`, host `x86_64-pc-windows-msvc`, LLVM `22.1.2`; the controlled-
  build allowlist is parsed from `xtask/src/rustc_driver.rs` and compared
  including `WASM_TARGET`.
- Ordinary guest freshness: initial `--check` exited **1** (47 stale guests;
  the target-nested guest artifacts had also been lost), plain
  `cargo xtask build-guests` rebuilt them, recheck exited **0**. Exit 3
  (cannot form an opinion) would have stopped preparation; no `--sync-locks`,
  no `--force` and no production change were used.
- Narrow support-repair suite: **8 passed, 0 failed**, including
  `identity_aggregate_spread_across_the_plate_is_measured_per_body` and
  `support_body_wider_than_the_deleted_routing_cell_is_retained`
  (`crates/slicer-wasm-host/tests/contract/support_plan_validation.rs`).
- Empty-infill producers: **6 passed, 0 failed**, both legs, merge contrasts
  and raft-only output
  (`crates/slicer-wasm-host/tests/contract/infill_postprocess_empty_commit_tdd.rs`).
- Gated runtime contract via `cargo xtask test --summary`: **9 passed,
  0 failed**, VERDICT PASS, including
  `infill_postprocess_empty_replacement_supersedes_prior_ir` and the
  in-boundary positive control
  `infill_postprocess_inside_path_survives_so_the_clip_is_real`
  (`crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs`).
  The wrapper ran its literal preflight, reported existing test-quality
  findings in report mode, checked guests and rebuilt its stale debug CLI.
  This is not a claim that packet-254 or full-workspace gates passed.
- Features/target gates inspected before running: neither `contract` target has
  `required-features`, none of the three files carries a file-level `#![cfg]`,
  and every named test is present and ungated. Each Rust run's combined output
  was teed to `target/test-output.log`, read from disk, and copied into this
  directory before the next run.

## Tool defect found and fixed during preflight

The first t41 preflight run stopped at its first gate on a defect in the new
tool itself: `preflight.py` parsed all five `ALLOWED_*` constants from
`xtask/src/rustc_driver.rs` but compared them against a four-entry expected set,
omitting `WASM_TARGET`. Nothing was built, no test or slice ran, and no durable
root was created; that run's evidence is preserved under
[`preflight-stop-1/`](preflight-stop-1/). The check was fixed to compare the
full policy grammar and a red/green regression was added to
`test-preparation.py` (the buggy four-key expectation rejects the correct tree;
the fixed five-key comparison accepts it and rejects an extra constant). The
re-run is recorded separately as [preflight results](preflight-results.json).
The preparation attempt itself ran exactly once and succeeded; no reference
generation retry occurred. However, the preflight restart was not authorized
by the human. It must not be described as full compliance with the no-retry
policy merely because reference generation had not started.

## Frozen output checks

All six cells: exit 0, `CompletionStatus ok`, degraded `false`, fatal/non-fatal
`0/0`, config/G-code/scheduler generator evidence agreeing, and every explicit
config setting plus the effective job disclosed in the output appendix.

| Workload | Generator | Completion | Fatal / non-fatal | Degraded | Support TYPE | Sparse segments | Worst sparse overshoot (mm) |
| --- | --- | --- | --- | --- | --- | ---: | ---: |
| supports-off-benchy | classic | ok | 0 / 0 | false | n/a (disabled) | 2,886 | 0.0 |
| supports-off-benchy | arachne | ok | 0 / 0 | false | n/a (disabled) | 3,069 | 0.0 |
| tree-support-benchy | classic | ok | 0 / 0 | false | yes | 2,886 | 0.0 |
| tree-support-benchy | arachne | ok | 0 / 0 | false | yes | 3,069 | 0.0 |
| tree-support-base | classic | ok | 0 / 0 | false | yes | 25,719 | 0.0 |
| tree-support-base | arachne | ok | 0 / 0 | false | yes | 25,899 | 0.0 |

The six models and configs are byte-identical to the historical corpus, whose
21-file identity is unchanged. Effective settings preserve the human-confirmed
job: `infill_density = 0.20000000298023224` (the promoted `f32` default) with
`sparse_fill_holder = rectilinear-infill`, and the explicit separate
perimeter-module `sparse_infill_density = 25` — 20% rectilinear fill, not 25%.
`wall_loops = 3`, `enable_support`/`support_type` match each config.

`check_output` (`prepare.py`) checks **both** endpoints of every printed sparse
segment against its layer's deposited-wall bounding box; travel moves never
enlarge that box and missing evidence fails loudly. Positive/negative controls
accept an inside segment and reject outside-start, outside-end and
sparse-without-wall-envelope cases. This is an envelope-overshoot check only —
holes, concavity and full polygon containment are outside its claims, and no
Orca parity is implied.

## Frozen manifest and verifier

[Durable manifest](attempt-1/frozen-manifest.json) sha256 (with its
[sidecar](attempt-1/frozen-manifest.sha256)):

```text
7b533240b7e602731eb6d11320c134c830fa00ec12b3d301b970dac935c334a3
```

Re-run the read-only identity gate before any accelerated comparison and again
after the campaign; **do not** regenerate missing or changed inputs:

```bash
python docs/specs/perf-vs-orca/evidence/t41-reference-repreparation/verify-freeze.py docs/specs/perf-vs-orca/evidence/t41-reference-repreparation/attempt-1
```

It checks the manifest digest, frozen/inventory identity of both roots (missing
roots are reported as `MISSING FROZEN ROOT: <path>`), a root placed under
`target/` is rejected outright, historical corpus and 1158-file build-input
identity, the recorded tool hashes, policy source/doc hashes, preflight gates,
per-cell input/output/sidecar hashes, clean completion, and it recomputes each
cell's sparse-bbox check. Both runs passed (exit 0; output in
[verification](attempt-1/freeze-verification.json) and
[final verification](attempt-1/freeze-verification-final.json)). Negative
controls ([results](freeze-control-results.json)) reject a changed manifest,
disabled Python assertions, a missing durable root and a tampered frozen-file
copy without modifying any real frozen file.

## Retention regression (root cause)

`test-retention.py` builds a disposable tree under the system temp directory
(never the real workspace) and proves, with [results](retention-results.json):

- **RED**: a payload frozen under `target/` is removed by a disposable
  `target/` cleanup, and the reconstructed t40 inventory gate fails against the
  recorded hashes — the issue-41 failure mode.
- **RED**: the t41 durability policy rejects a root under `target/`.
- **GREEN**: a payload under
  `.local-artifacts/perimeter-reference-preparation/` survives the same
  cleanup, still matches its inventory, and is accepted by the policy.
- Missing-root and file-tamper controls reject; the real workspace `target/`
  and real frozen files were never touched.

## Ordinary-vs-ordinary notes and limitations

Arachne `reference-*.gcode` files are byte-identical to t40's recorded hashes
and byte-reproducible. Classic `reference-*.gcode` files are **not**
byte-identical to t40's recorded hashes, and a bounded determinism probe on the
frozen t41 binary and inputs (kept in the durable payload root and summarized in
[determinism characterization](determinism-characterization.json)) shows classic
output differs run-to-run while every measured delta stays inside the documented
structural-exactness tolerances (`run-acceptance.ps1` header: lines ±0.1%,
TYPE markers ±1%, E totals ±0.5%; user decision 2026-09-11). Deposited sparse
segments were stable (2,886/2,886/2,886 with worst overshoot 0.0). `.jsonl`
sidecars embed absolute module paths and timestamps and are not byte-stable by
construction. The manifest records the actual hashes of the actual frozen
files; nothing was re-frozen to bless a replacement and no gate was weakened.

## Handoff

Pass the manifest's **new corpus root explicitly** as `-CorpusRoot` when the
separate acceptance ticket is taken; the runner's default root still holds the
deliberately preserved historical references. Re-establish the actual campaign
module set and mode-specific snapshots/freshness when taking
[Accelerated production adoption acceptance campaign](../../issues/38-accelerated-adoption-acceptance-campaign.md).

This preparation proves ordinary-output identity and targeted defect checks, not
full geometric correctness, accelerated exactness or adoption acceptance.
Remaining: accelerated comparison and campaign timing. Human acceptance of
the recorded preflight restart is also pending; do not
use this set for the campaign until that exception is decided. The
[main-session read-only identity verification](attempt-1/freeze-verification-main.json)
passed after worker completion, but does not grant process acceptance.
No production code, config, threshold or default changed; no commit was made;
ordinary remains the
actual default. `target/dist/developer/` was refreshed as an intended build side
effect (there was no previous dist to preserve); t40's evidence and the
historical corpus are untouched, and the durable payload directories are
untracked and must not be cleaned as scratch.
