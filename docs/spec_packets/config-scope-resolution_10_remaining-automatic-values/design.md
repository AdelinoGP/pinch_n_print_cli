# Design: remaining-automatic-values

## Selected Approach

Use a registry-declared host runtime extension key with a typed accessor, not a fixed serialized field. The existing `ResolvedConfig.extensions` map transports the key after normal packet-05 validation, defaults and scope resolution. Add no second resolution pass and no `typed_field_keys` entry. RegionMapIR stays at its pre-change 3.0.0 struct layout; new map content is distinct from a layout addition. This is not a blanket rule for future host keys.

## Controlling Code Paths

- `HOST_RUNTIME_KEYS`, `HostRuntimeKey`, `host_key_denied_scopes` and `ResolvedConfig` (`crates/slicer-ir/src/resolved_config.rs`): add the Float declaration with `Some("0.0")`, `SCOPE_FILAMENT`, `meta.min = Some(0.0)`, and `TOOL_CAPABLE_SCOPES` denial. The new accessor `filament_max_volumetric_speed(&self) -> Result<f64, String>` accepts Float, Int, or a finite numeric String, either as a scalar or as the first element of a non-empty List; absent is zero, while wrong types, empty Lists, non-finite values, and negative values are rejected. This string parsing is accessor-specific, not a general config coercion rule. Resolution retains this scalar envelope; it is not a tool-index array, so the accessor must not index it by tool number.
- `RegionMapIR.configs`, `RegionPlan.paint_overrides`, and `CURRENT_REGION_MAP_IR_SCHEMA_VERSION` (`crates/slicer-ir/src/slice_ir.rs`): restore only packet-added fixed-field/migration changes; retain pre-existing 3.0.0 shape and F-19 chain. `postcard` remains dev-only in `crates/slicer-ir/Cargo.toml`.
- `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`): live move inputs select a private automatic base when role speed is exactly zero, active-tool config wins, absent tool config falls back globally. Shared `feedrate_from_base_mm_per_s` applies normal clamp/conversion; no absolute point-speed input or public emission seam is added.
- `PrepassContext` and `prepare_prepass_context` (`crates/slicer-runtime/src/run.rs`): carry already-resolved tool configs to the model consumer instead of discarding them. Normal runtime and visual emission share resolved sources, not a duplicate scope engine.
- `load_visual_debug_config` and model GCodeEmit construction (`crates/pnp-cli/src/visual_debug.rs`): feed request-derived `FeedrateConfig::from_raw_config` and resolved global/tool configs to the emitter. Inline tests assert emitted F with a default-config control and distinct tool maxima.
- `resolved_config_keys_match_default` (`crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`): retained `[resolved_config]` mirror reads its default through the typed extension accessor. The previously red numeric-field lookup must not be presented as a passed lock.

## Architecture Constraints

- Config-only percentages/mirrors remain packet-04-owned; packet 05 remains the only precedence/resolution source. Emission does not repeat merges or registry assembly.
- Role speed zero triggers automatic speed; filament limit zero means unavailable. Automatic geometry/limit/base and final rounded/narrowed F must be finite positive. Reject non-finite factors before clamping; retain normal finite-factor clamp semantics. Overflow or round-to-zero fails through `GCodeEmitError::Emit`.
- Explicit positive configured speeds remain intentionally uncapped. This packet does not claim an all-speed volumetric ceiling.
- Canonical `GCode.cpp::GCode::_extrude` independently confirms the zero-speed quotient and active `m_writer.filament()` selection, but also caps explicit positive speeds separately. This packet is scope-limited formula/tool parity, not full canonical emitter parity; the user explicitly chose to leave positive speeds uncapped. The read-only source check is not a test PASS.
- Existing extension comparison/hash already participates in interning; test distinct content, config/paint preservation and identity instead of adding handwritten fixed-field equality/hash arms.
- Q8 "Amend ADR" authorizes the genuine narrow architectural mechanism amendment in ADR-0072 (`docs/adr/0072-context-aware-feedrate-resolution-preserves-factor-contract.md`), registered as `D-CSR10-ADR-0052-AMENDED` in `docs/DEVIATION_LOG.md`. ADR-0052 §Decision 1's unchanged-body, exact single-function placement and direct-production-call requirements are superseded, not merely clarified. Its factor-valued carriers, public role/factor signature, replacement/fallback, profile length and mutation/application constraints remain; original sections and 2026-08-05 amendment stay textually unchanged.
- Public context-free `resolve_feedrate` delegates role-only base selection and shared conversion; configured zero remains its zero placeholder. Production extrusion uses private `resolve_extrusion_feedrate` with live tool/geometry context for both point and entity factors. One private `feedrate_from_base_mm_per_s` applies `clamp(0.05, 5.0)` and conversion for automatic and explicit bases; its actual f64 multiply rounds to three decimals before f32 narrowing. Producers never send absolute mm/s. These are inspected mechanism facts, not test PASS or blanket byte-identity claims.
- WIT, stage packages, CLI JSON, visual schemas/manifest and persisted struct layout are unchanged.
- Coordinate unit is 100 nm; use established mm/unit adapters in fixtures.

## Artifact and Dependency Policy

Keep the independent pre-change fixture under `crates/slicer-ir/tests/fixtures/region_map_v3_0_0/` byte-for-byte, including provenance and expected values. Repurpose `region_map_versioned_decode_tdd.rs` for direct dev-Postcard deserialize and current extension round-trips across configs/paint; its historical filename does not promise a version dispatcher. Abandon the unshipped experimental 3.1.0 layout and decoder without deleting any existing experimental artifacts or promising compatibility. This is a non-deletion constraint, not a requirement to create an experimental fixture or an assertion that a 3.1.0 artifact exists. `review-remediation.md` identifies the extant files, baseline/end hashes and bounded search result; unidentified files are never described as proven present, absent or deleted. Do not create or regenerate fixtures, read large JSON fixtures directly, or rely on experimental artifacts as an oracle.

Retire the packet-added production Postcard dependency. Root/guest locks are regenerated only through normal Cargo operations; inspect resulting diffs rather than assuming previous dependency-promotion diffs remain needed. Do not hand-edit locks or discard unrelated churn. Guest freshness is mandatory: check exact exit 0/1/3, rebuild stale guests, and use gated sequential guest-touching tests. Ordinary `cargo build/test` does not rebuild guests.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs` — declaration/accessor, remove only experimental fixed field/fallout, inline accessor/scope controls.
- `crates/slicer-ir/src/slice_ir.rs` — retire only packet-added decoder/legacy shapes and restore 3.0.0 layout/version chain.
- `crates/slicer-ir/Cargo.toml` — retire packet-added production Postcard dependency, retain dev dependency.
- `crates/slicer-ir/tests/region_map_versioned_decode_tdd.rs` — preserved fixture plus current extension/paint identity and accessor cases.
- `crates/slicer-gcode/src/emit.rs`, `crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs` — extension accessor consumption, real emission and final-F failure controls.
- `crates/slicer-config/tests/automatic_value_expansion_tdd.rs`, `crates/slicer-config/tests/scope_eligibility_tdd.rs`, `crates/slicer-config/tests/registry_census_tdd.rs` — census, legitimate registry-roster follow-through, real declared-extension default/precedence/validation, and exact-denial coverage; preserve source-derived discovery and assertions, and do not rewrite packet-05 resolution behavior.
- `crates/slicer-runtime/src/run.rs` — `PrepassContext` and resolved-tool return/handoff; approved runtime expansion, not read-only.
- `crates/pnp-cli/src/visual_debug.rs` — model config/tool handoff and inline actual-F tests only.
- `crates/pnp-cli/tests/fixtures/config_scope_resolution_10/{visual-debug.json,visual-debug-config.json}` — request/config pair with a distinct tool-0 override to falsify a dropped runtime tool map; inline multiple-tool emission asserts separate literal feedrates.
- `crates/pnp-cli/tests/visual_debug_volumetric_auto_tdd.rs` — introduce a direct Cargo test target that compares actual model CLI captures with and without the tool-0 override; the shared `visual_debug_volumetric_auto` filter must run it, not just helper tests.
- `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` — accessor-based retained mirror lock.
- `docs/config/host-keys.toml`, generated `docs/15_config_keys_reference.md` — mirror retained, regenerate normally.
- `CONTEXT.md`, `docs/02_ir_schemas.md`, `docs/adr/0052-per-point-speed-factor-contract.md` (packet append only), TASK-571 evidence in `docs/07_implementation_status.md`, and this packet's five core files — revised domain/contract/evidence only.
- `docs/spec_packets/config-scope-resolution_10_remaining-automatic-values/review-remediation.md` — coordinator-owned finding disposition, identified artifact inventory and fresh verification receipts.
- `docs/adr/0072-context-aware-feedrate-resolution-preserves-factor-contract.md` — separate accepted narrow mechanism decision; `docs/DEVIATION_LOG.md` — only the `D-CSR10-ADR-0052-AMENDED` row. Q8 explicitly expands documentation scope to these paths; no other ADR or registry-row edits.
- Root `Cargo.lock` and guest Cargo-generated locks only when normal resolution changes them; audit each diff and validate convergence. These are generated outputs, never hand edits or a blanket entitlement to unrelated changes.

## Read-Only Context

- `docs/specs/config-scope-resolution-plan.md`, packet 04/05 exports/status and all other packet directories.
- `docs/19_visual_debug.md`, `docs/22_test_quality.md`, `docs/21_data_defaults_and_fixtures.md`, `docs/11_operational_governance_and_acceptance_gate.md` — relevant bounded sections.
- `crates/slicer-runtime/src/pipeline.rs::dump_prepass_ir_if_requested`, `crates/slicer-ir/src/feedrate.rs::FeedrateConfig`, `crates/slicer-gcode/src/error.rs::GCodeEmitError` — relevant symbols only.
- Independent pre-change fixture bytes/provenance/expected values and experimental on-disk artifacts — immutable inputs/history.

## Out-of-Bounds Files

Approved plan and packet dirs other than this one; WIT packages, UI/renderers, visual schema/manifest vocabulary; packet-04 expansion and packet-05 resolution implementations; unrelated source/docs/fixtures; prior ADR-0052 sections and all other ADRs except the exact new ADR-0072 path above. Deviation registry edits beyond the one named row are out of bounds. Never load canonical Orca directly, large fixture bodies, target/generated code or vendored dependencies. Experimental artifact deletion, fixture regeneration, branch/stash/reset/revert and commits are prohibited.

## Rejected Alternatives

- Phase-B zero-speed expansion or geometric `FeedrateConfig` adapter: missing live move context.
- Fixed serialized field plus 3.1.0 migration: existing extension transport suffices; unshipped experiment need not become a compatibility commitment.
- Public absolute-mm/s point-speed seam: contradicts factor contract; private automatic base does not need it.
- Conforming to ADR-0052's unchanged body/direct production call by simulating move context in configuration or cloning emitters: hides geometry-owned context; abandoning automatic geometry speed drops the chosen feature. Q8 instead accepts the bounded private delegation mechanism in ADR-0072.
- Re-resolving visual tools or taking a default-only map: duplicates precedence or loses tool-specific behavior.
- Manifest-only visual proof or a complete hand-authored census roster: false-green coverage.

## Expected Sub-Agent Dispatches

1. Reconcile dependencies and derived census; exact exports/status and source-derived owners; FACT or bounded LOCATIONS with symbol/path.
2. Verify canonical `GCode.cpp::GCode::_extrude` formula/tool/guards; SUMMARY at most 200 words.
3. Bounded workers by implementation step, each at most three edited files; fresh test/gate FACT with non-empty markers and failure snippets only. Return results outside thinking blocks.
4. Fresh preflight S0–S8 before activation and full review after all gates; no previous approval reused.

## Risks and Open Facts

Direct 3.0.0 fixture decode must prove actual shape restoration; extension round-trip alone is insufficient. Valid division can still produce unsafe final F, so test non-finite factors, overflow and round-to-zero independently. Visual tools must be the already-resolved map, not raw config reconstruction. The explicit-speed regression must use a positive volumetric ceiling below its configured speed. Current named receipts must cover fixture decode, roundtrip and interning; an older two-test receipt is not evidence for the current test file. Aggregate/largest context cost M; no L step.

Original preflight was `PREFLIGHT BLOCKED` on S8. Q8 remains resolved by ADR-0072/`D-CSR10-ADR-0052-AMENDED`; the later cold review reopened the packet with `CHANGES REQUESTED`. Remediation closure is supported by the fresh validation and independent full review recorded in `review-remediation.md`, not inferred from architectural acceptance.


## Implementation Deviations

### Review remediation

This records the repairs requested after the latest full review returned
`CHANGES REQUESTED`. It is not itself a closure verdict. Historical receipts in
`target/packet10-validation/` do not establish current-tree test execution.

#### Finding disposition

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

#### Artifact inventory

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

#### Verification receipts

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

#### Closure

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
