# Cold-review remediation: remaining-automatic-values

This records the repairs requested after the latest full review returned
`CHANGES REQUESTED`. It is not itself a closure verdict. Historical receipts in
`target/packet10-validation/` do not establish current-tree test execution.

## Finding disposition

| Finding | Root cause and repair | Verification |
| --- | --- | --- |
| Uncapped explicit-speed control | `per_tool_volumetric_auto_and_explicit_speed_are_distinct` (`crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs`) previously used a ceiling above the explicit speed. Positive limit 1 mm³/s now gives a 12.5 mm/s ceiling against explicit 30 mm/s; the independent F1800 assertion rejects an erroneous F750 cap. | Fresh full volumetric suite passes; named marker archived. |
| Runtime verification failure | `--all-targets` selected the harness-free `gate_evidence` Criterion bench, while `xtask::test_command` (`xtask/src/test.rs`) supplied libtest-only `--skip`. Step 2C now selects `--test unit` alone; check/clippy still use `--all-targets`, so benchmark compilation/lint coverage is retained. | Corrected focused command passes with all four doc-lock markers. |
| Current interning receipt | A historical two-test result was quoted for the current three-test IR target. AC-6 now checks and archives each named decode, roundtrip and interning marker before log overwrite. | Fresh AC-6 receipt has all three named markers and `3 passed; 0 failed`. |
| Unsupported artifact existence claim | Preservation was described as proven existence of unnamed 3.1.0 files. The non-deletion/regeneration prohibition remains; the identified files and search limits are recorded below, without manufacturing a fixture or claiming global absence. | Identified paths re-hashed: all exist and match baseline. |
| Numeric-string documentation | The accessor follows the existing scalar envelope, including finite numeric String or List-first equivalents. Packet/docs now state these accepted shapes explicitly; no coercion behavior changed. | Current library accessor/hash tests pass; documentation preflight passes. |
| Unlisted census edit and inaccurate comment | The related `registry_census_tdd.rs` roster follow-through is now in scope and Step 2C2/task map; its comment distinguishes the original synthesized rows from the new declared zero default. | Full census suite passes. |
| Narrowing control mislabeled as product overflow | The finite f64 product produces a quotient that narrows to f32 zero. `invalid_volumetric_auto_inputs_fail_closed` (`crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs`) now names underflow and asserts the specific conversion diagnostic separately from genuine base-speed overflow. | Named invalid-input marker passes in the fresh full volumetric suite. |
| Stale closure/task trace | TASK-571 and the packet were reopened; historical closure annotations are labeled superseded. Step 4B names `crates/pnp-cli/tests/visual_debug_volumetric_auto_tdd.rs`; no hand-maintained doc-grep count is treated as evidence. | Fresh full review `APPROVED WITH NOTES`; final checks pass and closure is recorded. |
| Fresh preflight closure/receipt blockers | ADR-0052's packet append and ADR-0072's status/verification no longer duplicate mutable packet closure; the named deviation remained open until full approval. AC-4 now archives both outputs under `target/packet10-remediation/` itself, not only through the validation worker. Original/normative ADR decisions are unchanged. | Focused preflight tail recheck and current AC-4 wrapper both pass. |
| Full-review optional comments | The IR accessor now states that unavailable zero cannot supply automatic derivation; the flow-2 test comment correctly doubles volume per distance and halves speed. No Rust statements or assertions are changed. | Independent comment-only hash witness passes; full approval retained; final touched gates pass. |

## Artifact inventory

Retention is a non-deletion rule for any existing experimental artifacts, not
a requirement to create a 3.1.0 fixture. No deletion, regeneration, checkout,
stash or reset is authorized or performed by this remediation. The independent
3.0.0 oracle remains the only identified authoritative regression fixture.

Inventory snapshot, 2026-09-30: each path below existed and was hashed from disk.
The binary fixture matches both its recorded provenance and the independently
archived generator output. The companion files are immutable inputs, not newly
generated expected values. Their current Git status is untracked; that does not
make them proof of alteration or prove they were committed.

Workspace fixture prefix:
`crates/slicer-ir/tests/fixtures/region_map_v3_0_0/`.

| File under that prefix | Baseline SHA-256 |
| --- | --- |
| `region_map_v3_0_0.postcard` | `5af57d3e3d2fe2205a12dda7d7927e71b49925cfc7850a6893c06ddc4e4b33e3` |
| `region_map_v3_0_0.provenance.txt` | `1e9e4ae0b9409c9819d9b914cf56a133b76251dae30407c8ba159d7d4a3722e7` |
| `region_map_v3_0_0.expected.json` | `67d922ddc97c622cf93711f51f4fac28382389ad9b79cc7d9a6e4dea6f32cd5d` |

Identified archive prefix, from the immutable provenance:
`C:/Users/agpen/AppData/Local/Temp/opencode/w2a-legacy-6ba38c34/`.
These files are generator history for the pre-change **3.0.0** layout, not
identified 3.1.0 experimental artifacts. The executables were not executed.

| File under that archive prefix | Baseline SHA-256 |
| --- | --- |
| `fixtures_out/region_map_v3_0_0.postcard` | `5af57d3e3d2fe2205a12dda7d7927e71b49925cfc7850a6893c06ddc4e4b33e3` |
| `target/debug/examples/region_map_v3_0_0_fixture_gen.exe` | `bd5ba85101f4eb351336304bf1a8ba64b6e204a6cdeb3f9d0b383db511d09afd` |
| `target/debug/examples/region_map_v3_0_0_fixture_gen.pdb` | `40efc9794607762e77731715f65e1b8b1ed903a1c77f419774b2a25929ed042f` |
| `target/debug/examples/region_map_v3_0_0_fixture_gen.d` | `5b1cf0a91df22a5f742125826f52ec9e6a0cf0030c6fea1274ac5882a2ca301d` |

Filename searches covered the workspace fixture directory, relevant workspace
`target/` names, the approved Temp/opencode tree, and the requested sibling
source-fixture path (which did not exist). They identified no specifically
attributable 3.1.0 file. This is a bounded non-finding, not proof that no such
file ever existed elsewhere or under an unrelated filename. Unknown artifacts
are not deleted, recreated, used as an oracle, or advertised as compatible.

Machine-readable baseline: `target/packet10-remediation/artifact-baseline.json`.
Re-hash receipt: `target/packet10-remediation/artifact-check.json`, result `PASS`:
all identified paths exist and their observed end hashes equal the baseline
hashes above. No assertion of historical retention is inferred from the absence
of a filename match.

## Verification receipts

Fresh remediation logs must be inspected and copied before the shared
`target/test-output.log` is overwritten. Each row below will quote the actual
current result, not an older receipt. Test-output counts are observations of a
specific run, not a future fixed roster requirement.

| Verification | Fresh result | Receipt |
| --- | --- | --- |
| `cargo test -p slicer-gcode --all-targets --test volumetric_auto_speed_tdd -- --nocapture` | `test result: ok. 10 passed; 0 failed`; `per_tool_volumetric_auto_and_explicit_speed_are_distinct ... ok` and `invalid_volumetric_auto_inputs_fail_closed ... ok` | `target/packet10-remediation/01-volumetric-auto-speed.log` |
| AC-6's current full IR wrapper | `test result: ok. 3 passed; 0 failed`; each marker below appears in this fresh receipt | `target/packet10-remediation/AC-6-tests.log` |
| `cargo test -p slicer-ir --lib -- --nocapture` | `test result: ok. 30 passed; 0 failed`; accessor/invalid-value/equality-hash markers passed | `target/packet10-remediation/03-slicer-ir-lib.log` |
| `cargo test -p slicer-config --all-targets --test registry_census_tdd -- --nocapture` | `test result: ok. 5 passed; 0 failed` | `target/packet10-remediation/04-registry-census.log` |
| `cargo xtask test --summary -p slicer-runtime --test unit host_keys_doc_lock_tdd` and AC-5 doc checks | `test result: ok. 4 passed; 0 failed`; both effective-value/mirror markers pass; generated docs and doc greps pass | `target/packet10-remediation/AC-5-tests.log` |
| AC-1, AC-2, AC-3, AC-7, AC-N1 current wrappers | Each required named test marker is `... ok`; each command exits successfully | `target/packet10-remediation/AC-1-tests.log`, `AC-2-tests.log`, `AC-3-tests.log`, `AC-7-tests.log`, `AC-N1-tests.log` |
| AC-4 current test/CLI/bundle wrapper | Both inline emitter tests and `visual_debug_volumetric_auto_runtime_tool_override_changes_emitted_feedrates ... ok`; CLI bundle Python assertions pass | `target/packet10-remediation/AC-4-tests.log`, `AC-4-cli.log` |
| AC-8 current precedence/rejection wrappers | `declared_volumetric_extension_resolves_defaults_and_tool_precedence ... ok`; `declared_volumetric_extension_rejects_invalid_values_and_object_scope ... ok` | `target/packet10-remediation/AC-8-precedence.log`, `AC-8-rejections.log` |
| AC-N2 current full-file wrapper | All three required factor/final-F markers `... ok`; `test result: ok. 10 passed; 0 failed` | `target/packet10-remediation/AC-N2-tests.log` |
| Full automatic-value file | `test result: ok. 10 passed; 0 failed`; sentinel negative-control panic is caught and its named test passes | `target/packet10-remediation/full-automatic-values.log` |
| Full scope-eligibility file | `test result: ok. 8 passed; 0 failed`; `host_denial_roster_is_exact_across_host_declaration_channels ... ok` | `target/packet10-remediation/full-scope-eligibility.log` |
| `cargo build --workspace` | `COMMAND_EXIT_CODE=0` | `target/packet10-remediation/build.log` |
| `cargo check --workspace --all-targets` | `COMMAND_EXIT_CODE=0` | `target/packet10-remediation/check.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | `COMMAND_EXIT_CODE=0` | `target/packet10-remediation/clippy.log` |
| `cargo xtask check-literals` | `check-literals: 0 violation(s) in 0 file(s)`; exit 0 | `target/packet10-remediation/check-literals.log` |
| `cargo xtask check-test-quality --report` | Exit 0; no findings in packet-touched tests. Findings elsewhere remain outside this repair; this is not a blanket repository quality pass. | `target/packet10-remediation/test-quality.log` |
| `cargo xtask gen-config-docs --check` | `OK: doc 15 generated sections current`; exit 0 | `target/packet10-remediation/gen-config-docs.log` |
| `cargo check -p slicer-ir --all-targets` | `COMMAND_EXIT_CODE=0` | `target/packet10-remediation/check-ir.log` |
| `cargo check -p slicer-gcode --all-targets` | `COMMAND_EXIT_CODE=0` | `target/packet10-remediation/check-gcode.log` |
| Full pnp-cli `visual_debug` filter through gated xtask | Visual-debug module `23 passed`; actual CLI override marker `... ok`; `VERDICT: PASS`, exit 0 | `target/packet10-remediation/full-visual-debug.log` |
| Fresh preflight, including reported-blocker tail recheck | `PREFLIGHT PASS`; initial blocked receipt preserved separately; architectural acceptance remains distinct from closure | `target/packet10-remediation/preflight.txt`, `preflight-initial.txt` |
| Shell syntax of each current AC wrapper | Each `AC-1`–`AC-8`, `AC-N1` and `AC-N2` wrapper passes Git Bash `bash -n`; this syntax check is not test-execution evidence | `target/packet10-remediation/ac-shell-syntax.log` |
| Corrected AC-4 archival command | All three named markers pass; wrapper/Python assertions exit 0; both output logs are retained at the wrapper's actual archive paths | `target/packet10-remediation/AC-4-tests.log`, `AC-4-cli.log` |
| `cargo xtask build-guests --check` | `COMMAND_EXIT_CODE=0` (fresh, not stale or infrastructure-unknown) | `target/packet10-remediation/guest-freshness.log` |
| `git diff --check` | `COMMAND_EXIT_CODE=0` | `target/packet10-remediation/diff-check.log` |
| Artifact hash comparison | `artifact preservation: PASS; all identified paths exist and match baseline` | `target/packet10-remediation/artifact-check.json` |
| Independent holistic full review | `APPROVED WITH NOTES`; all ten ACs pass, no blockers or load-bearing unverified rows; only two optional comment clarifications | `target/packet10-remediation/full-review.md` |
| Post-review comment-only witness | `COMPLETE HASH WITNESS PASS`; restoring each old comment in memory matches the validated source baseline, all other source hashes match; prior full approval retained | `target/packet10-remediation/comment-notes-review.txt` |
| Post-comment guest freshness | Initial check exit 1; normal stale-only build exit 0; decisive recheck exit 0 | `target/packet10-remediation/post-comments-guest-before.log`, `post-comments-build-guests.log`, `post-comments-guest-freshness.log` |
| Post-comment `cargo clippy --workspace --all-targets -- -D warnings` | `COMMAND_EXIT_CODE=0` | `target/packet10-remediation/post-comments-clippy.log` |
| Post-comment gated runtime doc-lock | All four named tests pass; `test result: ok. 4 passed; 0 failed` | `target/packet10-remediation/post-comments-runtime-doc-lock.log` |
| Post-comment full volumetric file | All ten named tests pass; `test result: ok. 10 passed; 0 failed` | `target/packet10-remediation/post-comments-volumetric.log` |
| Final closure bookkeeping/integrity | `PASS`: packet implemented, TASK-571 checked, named deviation closed, identified artifact hashes unchanged, latest guest freshness clean; `git diff --check` exit 0 | `target/packet10-remediation/closure-checks.json`, `post-closure-diff-check.log` |

The fresh AC-6 receipt names the current tests in
`crates/slicer-ir/tests/region_map_versioned_decode_tdd.rs`:

```text
test distinct_extension_values_are_interned_separately_and_equal_ones_dedupe ... ok
test frozen_region_map_3_0_0_fixture_decodes_unchanged_with_absent_extension_default ... ok
test region_map_roundtrip_preserves_extension_magnitudes_at_both_sites ... ok
test result: ok. 3 passed; 0 failed
```

The first runtime capture incorrectly added an outer tee to xtask's own shared
log and produced NUL bytes. Its raw capture is retained as
`target/packet10-remediation/05-runtime-host-key-doc-lock.log`; the readable
captured stdout is retained separately as
`target/packet10-remediation/05-runtime-host-key-doc-lock-readable.log`. No test was rerun to
recover this output. Subsequent xtask runs must never add a second writer to
`target/test-output.log`; archive xtask's own completed log instead.

AC-4's CLI run reports `ERR_MALFORMED_LAYER_MARKER` warnings. The bundle passes
the contract's warning-array, executed-stage and non-empty-image checks; this
packet does not assert a warning-free bundle or hide those diagnostics. The
feedrate handoff claim is independently established by the actual CLI ratio
test, not inferred from the manifest or warning count.

## Closure

Remediation closed on 2026-09-30 after fresh preflight, all ACs, validation and
independent holistic full review. The full review is `APPROVED WITH NOTES`, with
no blockers or load-bearing unverified rows. Its two optional comment
clarifications are corrected; independent comment-only verification passes and
full approval is retained. Final touched gates pass on the corrected tree.
The shared-source comment changed guest fingerprints, so the initially stale
guests were rebuilt normally; the decisive freshness recheck returns exit 0.
Packet status is `implemented`, TASK-571 is checked and the named deviation is
closed. The full review itself remains immutable evidence of the tree it
reviewed, supplemented by the comment-only witness and current-tree receipts,
not retroactively rewritten as a review of later changes.
No commit or full-workspace test is authorized. A completed implementation or a
successful command does not, by itself, authorize packet closure.
