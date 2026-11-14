# Property-testing evidence: scheduler DAG, claims and the region-split registry

Packet: `proptest-openrscad_07_pbt-scheduler`. This document records the executed evidence for the
three registry families added by that packet (`H-DAG`, `H-CLAIMS`, `H-REGISTRY`) and the G-FAMILY
disposition those families support.

Everything below was executed on Windows (the development machine for this repository). Seeds are
diagnostic artifacts: committed regression history is a named concrete input, never a seed. No
authoring run is represented as test evidence — authoring executed nothing, and every row here was
produced while implementing the packet.

## G-FAMILY disposition

| Charter | Registry ID | Disposition |
|---|---|---|
| `H-DAG` | `H-DAG` | **Complete for the assigned charters.** Success cases are compared to an exhaustive lexical-minimum linear extension over all node permutations; cyclic cases compare both sort paths to a nonempty-path cycle remainder including the acyclic tail; duplicated destinations, input permutations and real `build_intra_stage_dag` edge/reason relations are observed. |
| `H-CLAIMS` | `H-CLAIMS` | **Complete, with two confirmed SUT defects fixed.** Typed scope/claim and write/dependency models, the reviewed fill/family/stable exception table, effective-universe removal, held-claim resolution, typed perimeter selection and support-family pairing. Live per-region support aggregation remains outside this API and is not claimed. |
| `H-REGISTRY` | `H-REGISTRY` | **Complete for aggregation, ordering and warnings.** Compatible permutation invariance, name-ordered map keys versus the separate `(priority, name)` helper, first-seen conflicting mode, tied-priority payload and real plan-assembly aggregation. Helper-versus-RegionMapping integration is explicitly not claimed. |

No family is claimed complete beyond the scope written in its charter, and no family is closed on
registry-only evidence: every row above was executed through the registry-backed command, and every
negative shape is also pinned by a deterministic control that runs under a plain `cargo test`.

## Source, toolchain and dirty-content provenance

- Source revision: each run records it per run in
  `target/property-testing/<run-token>/<ID>/context.json` (`source_revision`, `dirty_paths`,
  `source_state_digest`, `cargo_target_dir`, `completion_eligible`). The runs quoted in the tables
  below were taken on `source_revision = 14fd1776` **with the review-remediation files
  uncommitted**, and their `dirty_paths` listed the three charter models, the seven control
  modules, the two aggregation roots, `crates/slicer-scheduler/src/validation.rs` and the docs.
  That is the final implementation generation. An earlier generation at `source_revision =
  3cd01be0` produced the first pass of measurements; the independent review rejected that pass on
  shrink preservation and generative coverage, so those numbers are superseded and are not quoted
  here.
  Re-derive the current revision with `git log --format=%H -1 --grep proptest-openrscad_07` rather
  than trusting a frozen copy; an amend or squash changes it. The per-run `context.json` files are
  authoritative for what each measurement actually measured, and this document does not restate a
  hash that an amend would invalidate.
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`, Windows `x86_64-pc-windows-msvc`.
- Proptest: `1.11.0` (`Cargo.lock`), `chacha` RNG, Proptest's own timeout/fork features unused
  (`isolation = "in_process"`, `timeout_ms = 0`).
- Features: `slicer-scheduler/property_testing` for all three records. It is a default-off empty
  selection feature: it implies no engine, SCAD or host-algorithm feature. The `slicer-sdk`
  dev-dependency supplies the harness, domains and regression helpers; it is a **dev-only** edge,
  so the scheduler's normal dependency graph stays wasmtime-free (asserted by the existing
  `property_dependency_graph_handles_dev_feature_unification` xtask test).
- `CARGO_TARGET_DIR`: unset — the executor records `cargo_target_dir: null` and the default
  workspace `target/` is used.
- `requires_host_algos = false` and `requires_scad = false` for all three records;
  `generator_version = 1`.

## Registry records added

| ID | Package / target | Test | Production symbol | Charter |
|---|---|---|---|---|
| `H-DAG` | `slicer-scheduler` / `property_scheduler` | `proptest_scheduler_dag_models` | `slicer_scheduler::topological_sort` | [properties.md#h-dag](properties.md#h-dag) |
| `H-CLAIMS` | `slicer-scheduler` / `property_scheduler` | `proptest_scheduler_claim_models` | `slicer_scheduler::validation::validate_startup_dag` | [properties.md#h-claims](properties.md#h-claims) |
| `H-REGISTRY` | `slicer-scheduler` / `property_scheduler` | `proptest_scheduler_region_split_registry` | `slicer_scheduler::region_split::aggregate_region_splits` | [properties.md#h-registry](properties.md#h-registry) |

Both targets are new standalone files registered in `crates/slicer-scheduler/Cargo.toml` with
`required-features = ["property_testing"]`:
`crates/slicer-scheduler/tests/property_scheduler.rs` (the three generative bodies, all
target-private) and `crates/slicer-scheduler/tests/property_scheduler_contracts.rs` (the
deterministic controls). The `property_config` / `property_config_contracts` targets from packet
`proptest-openrscad_06_pbt-ir-config` are untouched; the three records reuse that packet's
`property_testing = []` feature and dev-dependency declarations instead of duplicating them.

Committed profiles (identical shape to the earlier families; bounds are the family-specific part):

| Key | `H-DAG` routine / extended | `H-CLAIMS` routine / extended | `H-REGISTRY` routine / extended |
|---|---|---|---|
| `cases` | 64 / 1024 | 64 / 1024 | 64 / 1024 |
| `max_local_rejects` / `max_global_rejects` / `max_flat_map_regens` | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| `max_default_size_range` | 32 / 256 | 32 / 256 | 32 / 256 |
| `max_shrink_iters` | 64 / 256 | 64 / 256 | 64 / 256 |
| `max_shrink_time_ms` | 0 / 0 | 0 / 0 | 0 / 0 |
| `rng_algorithm` / `isolation` / `timeout_ms` | chacha / in_process / 0 | chacha / in_process / 0 | chacha / in_process / 0 |
| `max_nodes` | 5 / 7 | — | — |
| `max_modules` | — | 4 / 7 | 4 / 7 |
| `max_claims` | — | 4 / 8 | — |
| `max_observations` | — | 8 / 16 | — |
| `max_layers` | — | 3 / 4 | — |
| `max_declarations` | — | — | 8 / 16 |
| `max_text_bytes` | — | 36 / 64 | 36 / 64 |

Zero rejection budgets are honest because every generator is total (no `prop_assume`, no filtering
of valid cases); a rejection would signal a generator change and abort as `exhausted` rather than
silently consuming budget. The bounds stay inside the packet's safety ceilings (`3..=7` nodes;
`3..=7` modules, `1..=8` claims, `2..=16` observations, `2..=4` layers, `36..=64` bytes for claims
(UUID-syntax object ids; the packet's 8-byte variable-text floor does not carry them);
`2..=7` modules, `2..=16` declarations, `8..=64` bytes for the registry), enforced before any
allocation by the target-private `checked_max_nodes` / `checked_claim_limits` /
`checked_registry_limits` helpers. A claims `max_text_bytes` below 36 is rejected by
`check_text_bound` (called from `checked_claim_limits`) with a named error rather than widened:
the UUID-syntax id family `domains::id_text(IdTextKind::UuidSyntax, ..)` needs 36 bytes.

## Baseline: existing owner tests, captured before the additions

Every command was captured to `target/test-output.log` and archived as
`target/evidence/baseline-<name>.log`, with `target/evidence/baseline-summary.txt` holding one
`<name> exit=<code> <test result line>` entry per run.

| Command | Result |
|---|---|
| `cargo test -p slicer-scheduler --test scheduler_unit -- topological_sort_tdd` | PASS (7 tests) |
| `cargo test -p slicer-scheduler --test scheduler_unit -- dag_construction_tdd` | PASS (6 tests) |
| `cargo test -p slicer-scheduler --test scheduler_unit -- execution_plan_tdd` | PASS (17 tests) |
| `cargo test -p slicer-scheduler --test scheduler_contract -- claim_transition_matrix_tdd` | PASS (4 tests) |
| `cargo test -p slicer-scheduler --test scheduler_contract -- holder_matching_tdd` | PASS (2 tests) |
| `cargo test -p slicer-scheduler --test scheduler_contract -- typed_selector_claim_selection_tdd` | PASS (1 test) |
| `cargo test -p slicer-scheduler --test region_split_manifest_tdd` | PASS (9 tests) |
| `cargo test -p slicer-scheduler --test region_split_aggregation_tdd` | PASS (4 tests) |
| `cargo test -p slicer-scheduler --test region_split_plan_tdd` | PASS (9 tests) |
| `cargo test -p slicer-scheduler --test scheduler_integration -- support_family_selection` | PASS (2 tests) |
| `cargo test -p slicer-scheduler --test scheduler_integration -- support_family_pairing_rejected` | PASS (1 test) |
| `cargo xtask test --summary -p slicer-scheduler --test raft_claim_conflict_tdd` | PASS (1 test; gated freshness check ran first) |
| `cargo test -p slicer-sdk --features property_testing --test property_domain_contracts` | PASS (11 tests) |
| `cargo test -p slicer-scheduler --features property_testing --test property_config_contracts` | PASS (5 tests) |
| `cargo test -p xtask --bin xtask -- property_` | PASS (23 tests) |

`cargo xtask build-guests --check` returned exit `0` (`EXIT_FRESH`) before any module/manifest
work, so no guest-staleness explanation is available for any scheduler result here.

The xtask property tests print several `test result: FAILED. 0 passed; 1 failed` blocks and
`expected failure` child exits. Those are the deliberate fixture children
(`proptest_fixture_context_child`, `proptest_fixture_all_rejecting`) asserting that bad contexts and
exhaustion fail; the outer target's own result line is `ok. 23 passed`, and the command exited `0`.

`claim_transition_matrix_tdd` was already green before the fix below: its four tests cover
*same-region* cross-layer variance, which the old `scope_key_without_layer` grouping happened to
catch. The gap this packet confirmed is the object-wide rule across **different regions** of one
object, which had no test — see "Confirmed SUT findings".

## Calibration: mandatory witnesses, population guards and pilots

Each family runs deterministic witnesses on every execution, independent of sampling:

- `H-DAG`: a self-cycle with a downstream tail and two disconnected cycles with one tail each, each
  compared against the path model on both the public sorter and `CycleDetection`; and a 128-case
  pre-sample guard that must reach an empty DAG, a singleton, a disconnected multi-node DAG, a
  multi-edge DAG, a duplicate-destination sample, a cycle-with-tail sample and a nonempty cycle
  remainder.
- `H-CLAIMS`: a fixed global four-fill-claim setup on every unrelated request, the exact
  `MissingDependency` fill strings, delimiter-rich scope identities, same-region and cross-region
  stable-claim witnesses, the family-support root boundary (`SupportIR` accepted, `SupportIRx`
  rejected), and a bounded pre-sample guard covering every tagged mode plus the
  positive/negative verdicts.
- `H-REGISTRY`: normative-catalog cross-check (`CORE_REGION_SPLIT_PRIORITIES` and
  `COMMUNITY_PRIORITY_FLOOR` against the literals the charter states), a 128-case guard requiring
  compatible, conflicting, tied, same-module-tie and cross-module-tie samples plus a nonempty
  aggregate and a tied-priority warning.

Controlled-context pilots were run for `H-DAG` only, before its registry record existed, using
hand-built `PNP_PROPERTY_CONTEXT` files under `target/property-pilots/h-dag/` (`context.json`
`max_nodes = 5`, `context-n3.json` `max_nodes = 3`, `context-n7.json` `max_nodes = 7`; 24 cases
each, routine tier, seeds `424242` / `424245` / `424249`). All three pilot contexts passed with 24
accepted calls, zero rejects and a complete call journal. These pilots are calibration only and are
not the packet's `P` evidence; the registry runs below are.

Profile calibration therefore used a separate pre-registration pilot harness for `H-DAG` only, not
for all three families: `H-CLAIMS` and `H-REGISTRY` were registered inside the packet's safety
ceilings, their first registry-backed run **is** the pilot, and no bound, case count or reject
budget was changed after that measurement (packet `## Deviations` item 4).

## Executed profiles

All rows were produced by the registry-backed command; no row is a compile-only observation.

| Run | Command shape | Result |
|---|---|---|
| routine | `cargo xtask test --summary --properties --property H-DAG --property H-CLAIMS --property H-REGISTRY --tier routine` | `VERDICT: PASS` |
| extended | same with `--tier extended` | `VERDICT: PASS` |
| retained-seed replay | same routine selection with `--seed 8015297453583596151` | `VERDICT: PASS` |
| per-record replay | `--property <ID> --tier routine --seed <that record's routine seed>` | PASS for all three |
| incremental selection | `cargo xtask test --summary --properties --tier routine` (no `--property`) | `VERDICT: PASS`, 15 binaries |

Per-property measurements (libtest wall as printed by the executor's captured child):

| ID | routine wall | routine accepted / rejected | extended wall | extended accepted / rejected | routine seed (retained replay) |
|---|---:|---|---:|---|---|
| `H-DAG` | 0.20s | 64 / 0 | 1.69s | 1024 / 0 | 8015297453583596151 |
| `H-CLAIMS` | 2.27s | 64 / 0 | 1.92s | 1024 / 0 | 5053888522409382591 |
| `H-REGISTRY` | 0.13s | 64 / 0 | 0.83s | 1024 / 0 | 2960010196684534321 |

Every row reported `call_counts_complete = true`, `accepted_cases == cases`, outcome `passed` and
`error_kind: null`; `rejected_test_calls` was `0`. The retained-seed replays printed libtest walls
0.07s / 1.07s / 0.07s for `H-DAG` / `H-CLAIMS` / `H-REGISTRY`, and the combined-retained-seed run
(routine selection with `--seed 8015297453583596151` applied to all three) also passed — the
shared seed replays the `H-DAG` stream exactly and gives the other two a different stream, so the
per-record replays above are the authoritative retained-stream evidence.

Discovery and controller evidence: each selected property emits a `PNP_PROPERTY_EVENT` begin and
end line with `"process_role":"controller"`, the registry `property_id`, a `run_token`, the
profile-derived `accepted_cases`, `rejected_test_calls`, `call_counts_complete` and
`completion_eligible: true`. The three-property routine selection produced exactly six such events
(one pair per record, no worker events under `in_process` isolation), and the combined routine run
produced fifteen passing test binaries, proving the foundation's `properties-windows` job picks the
new records up dynamically: no hand-maintained roster was edited.

### Hosted routine execution (AC-7)

The foundation's `properties-windows` job executed this implementation on a hosted Windows runner:

- Workflow run: <https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43556307245>
- Job: `Properties (Windows)` — <https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43556307245/job/132329819467>
- Head: `f5c1577906f3a7282c59a373e6b1b6b6ee7f20ce` (the reviewed implementation commit), trigger
  `pull_request` on branch `proptest-openrscad-07-pbt-scheduler`, runner `windows-latest`,
  concluded `success` at `2026-11-14T04:28:36Z` with every step green (`Build guest WASMs`,
  `Run routine properties`, upload step skipped because nothing failed).
- Command: `cargo xtask test --summary --properties --tier routine` — the dynamic selector, no
  `--property` list, after the job's own `check-test-quality` and `check-literals` preflights.
- Result: the digest printed fifteen `test result: ok. 1 passed; 0 failed` lines and the final
  `VERDICT: PASS`. Exactly three of those binaries carry the `2 filtered out` signature of the
  `property_scheduler` target (three registered tests), at 0.07s / 0.98s / 0.06s, which is the
  selection order `H-DAG`, `H-CLAIMS`, `H-REGISTRY`.

What this evidence establishes and what it does not: `VERDICT: PASS` is reachable only when every
selected record completed — the foundation harness turns a missing discovery, an exhausted run, an
incomplete call journal or `fewer observed accepted calls than required cases` into a non-zero
exit, so a passing hosted verdict means all three routines met their committed case counts. The
digest does not echo record IDs or `accepted_cases` per line, and the job uploads no artifacts on
success, so record identification here relies on the target's unique `2 filtered out` signature
plus the committed registry, not on a per-ID hosted line. Per-ID accepted counts are recorded from
the local Windows runs above.

## Compile versus execution measurements

Compilation and execution are recorded separately; the walls above are execution only.

- Warm incremental target compile: `cargo check -p slicer-scheduler --all-targets --features
  property_testing` finished in 0.7s–3.2s across the implementation runs (observed values 0.70s,
  0.92s, 1.19s, 1.22s, 1.71s, 3.19s). These are warm incremental numbers, not cold builds.
- Cold-ish first registry run after the tree changed: the xtask pre-step reported
  `pnp_cli is stale or absent; rebuilding...` and finished the `dev` relink in 3m 34s before any
  property executed. Subsequent runs relinked in 1.08s–1.37s, so that 3m 34s is a one-off
  workspace relink and is **not** attributed to the new test targets.
- Deterministic control target: `cargo test -p slicer-scheduler --features property_testing --test
  property_scheduler_contracts` runs the contracts target; its current test count is whatever that
  command reports. It includes the in-process harness-refusal controls in
  `crates/slicer-scheduler/tests/common/property_scheduler_contracts/bound_controls.rs`, which
  drive `slicer_sdk::test_support::property_testing::PropertyRun` directly and write temporary
  artifact directories; that binary spawns no nested `cargo` child. The measured walls of 3.23s
  (4.47s on an earlier, longer-windowed run) belong to the earlier 13-test target; this document
  holds no wall measurement for the expanded target and none is inferred from them.
- Execution proper stays cheap at the extended tier (largest wall 1.92s for 1024 `H-CLAIMS` cases in the quoted run; an earlier same-head run measured 3.89s / 3.91s / 2.00s, so these walls vary with machine load),
  so the committed bounds are measurable rather than aspirational.

## Shrinking and exhaustion status

No accepted run performed a shrink call: with no failure persistence and a passing run,
`non-rejected callback calls == cases`, and the observed `shrink_completed` is `null` with the
reason "Proptest 1.11 does not expose whether shrinking reached a limit". The packet therefore does
not claim any retained case is minimal.

Shrink behaviour is instead pinned structurally:

- `H-DAG` draws its `Dag`/`Cycle` mode from a non-shrinkable weighted list
  (`proptest::sample::select(..).no_shrink()`), not `prop_oneof!`, because a union shrinker would
  try an earlier branch and could switch a cycle witness into an acyclic one. Cycle samples are
  rebuilt from `(cycle_len, tail_len)` inside the domain's `prop_map`, keeping a real cycle plus a
  tail; the node permutation is re-drawn through `prop_flat_map` and re-validated as a complete
  permutation. `assert_shrink_preserves_modes` simplifies fixed-seed trees on every execution and
  asserts the mode tag, a nonempty blocked remainder for cycles, acyclicity plus exhaustive-oracle
  agreement for DAGs, and preservation of the duplicate-destination flag.
- `H-CLAIMS` selects its `ClaimMode` from a non-shrinkable list and
  `assert_shrink_preserves_witnesses` audits, per mode, that the distinguishing witness survives
  simplification (two distinct typed identities, three holders across `r0`/`r1`/`r2`, the
  delimiter-colliding pair, the conflicting write pair, the same-scope `support-generator` holder
  pair, the disabled consumer's missing module, and the typed `wall_generator` plus `spiral_vase`
  selector). Only the module declaration order is a shrinkable dimension in the selection/pairing
  mode, so the expected kept set cannot change identity.
- `H-REGISTRY` selects the requested shape and every fault-deciding flag from a non-shrinkable
  case plan; `assert_shrink_preserves_faults` asserts the shape never changes, a conflicting case
  keeps the same first-seen `(priority, value_type)` and exactly one incompatible warning, a tied
  case keeps exactly one tied priority group and warning, and a compatible case acquires no
  conflicting warning. First-seen expectations are recomputed for the shrunk module order so a
  reorder cannot be mistaken for a failure.
- Exhaustion is impossible under the committed zero budgets for a total generator; the
  harness-level refusal is itself pinned by
  `crates/slicer-scheduler/tests/common/property_scheduler_contracts/bound_controls.rs`, which
  drives an always-rejecting strategy to `kind == "exhausted"` and asserts a passing run is the
  only success path.

## Confirmed SUT findings

Two defects were confirmed in `validate_claim_conflicts`
(`crates/slicer-scheduler/src/validation.rs`) and fixed there; both were reproduced on the pre-fix
tree with the concrete inputs below before any production edit. `H-DAG` and `H-REGISTRY` produced
no production finding, and no regression was manufactured for them.

### 1. Claim scope identity was serialized into a string (aliasing and lost identity)

- Previous input: claim `claim:infill-link` held by `com.control.claim-0` at object `a:b`, region
  `c`, layer `0` and by `com.control.claim-1` at object `a`, region `b:c`, layer `0`, with the
  fixed global fill setup present.
- Pre-fix observation: exactly one per-region `ClaimConflict` with
  `scope = Region { object_id: "a", region_id: "b", global_layer_index: None }` — two distinct
  typed scopes collided through `region:{object}:{region}:{layer}` concatenation, and the reported
  scope was re-parsed from the key, so both original identities were lost.
- Root cause: `scope_key`, `scope_key_without_layer` and `region_scope_from_key` treated opaque
  object/region text as a delimiter-separated grammar that `ConflictScope` never promised.
- Fix: a typed `ClaimScopeKey` (`Global`, `Object { object_id }`,
  `Region { object_id, region_id, global_layer_index }`) groups holders; the three string helpers
  are deleted and the reported scope is the stored original value.
- Regression:
  `crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs::delimiter_rich_object_and_region_ids_never_share_a_claim_scope`
  (doc comment carries the root cause and the previous input).

### 2. Stable claims were not enforced object-wide across regions

- Previous input: `perimeter-generator` held by `com.example.alpha-perimeters` at object `cube`,
  region `top`, layer `0` and by `com.example.beta-perimeters` at object `cube`, region `bottom`,
  layer `10`, with the fixed global fill setup present.
- Pre-fix observation: the report was clean — no `ClaimConflict` at all, although
  `docs/01_system_architecture.md` §"Allowed Claim Transition Matrix" requires one holder across
  all layers for one `(object_id, claim)`.
- Root cause: `scope_key_without_layer` dropped only the layer and kept `region`, so two regions of
  one object were separate groups and never compared.
- Fix: the four non-transitionable claims (`perimeter-generator`, `seam-placer`, `layer-planner`,
  `mesh-analyzer`) group by `Object { object_id }`; a conflict reports the first holder's original
  region with `global_layer_index = None`. Transition-capable claims keep the full typed
  `(object, region, layer)` key, so permitted sparse-fill and support transitions stay legal, and
  no temporal policy is invented for other claim ids.
- Regression:
  `claim_transition_matrix_tdd::non_transitionable_claim_rejects_holder_change_across_regions_of_one_object`
  (registered in `crates/slicer-scheduler/tests/contract/main.rs`, doc comment carries the root
   cause and the previous input). The four pre-existing same-region/cross-layer and
   transitionable-claim tests in that module still pass unchanged.

### Defect-ledger statement

The fix removes a `pub`-internal string helper trio, changes no public type, adds no
`SchedulerError` variant, and keeps the stable-claim set at the four ids the reviewed exception
table names. `support-planner` remains family-scoped at dispatch (per-family planners coexist) and
is deliberately not added to the stable set by this packet.

## Mutation controls (what the models provably reject)

The deterministic controls run under `cargo test -p slicer-scheduler --features property_testing
--test property_scheduler_contracts`; the target's current test count is whatever that command
reports, and each control names the mutation it must catch. Every
control below lives in `crates/slicer-scheduler/tests/common/property_scheduler_contracts/`; the
citations give the full workspace-relative file path rather than a bare module name.

- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/dag_controls.rs::scheduler_dag_oracle_controls`
  — FIFO/insertion-ready instead of lexically-minimum-ready, a missing node, a duplicated node, and
  a cycle remainder that omits the acyclic tail.
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/dag_controls.rs::scheduler_dag_raw_model_rejections_before_sorter`
  and
  `crates/slicer-scheduler/tests/common/property_scheduler_contracts/dag_controls.rs::scheduler_raw_graph_and_plan_rejections`
  — duplicate-node and unknown-endpoint raw models rejected before any sorter call (and by the
  shared path oracles), plus the four real `ExecutionPlanError` binding/config negatives with their
  offending ids/keys and a cyclic production input whose public `Err` and `CycleDetection` set agree
  and never yield a partial schedule.
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/plan_controls.rs::scheduler_graph_construction_and_plan_observations`
  — real edge/reason relation, stage exclusion, compiled sequence/claims/masks/paint semantics,
  host paint-stage insertion and the non-vacuous `aggregated_region_split` propagation.
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs::scheduler_claim_scope_and_exception_controls`
  — delimiter aliasing, dropped `claim:raft-fill` conflicts, a blanket `support-family:*` exemption,
  lost original identity, every stable-claim direction including the permitted transitions, and the
  same-scope `support-generator` rejection (two distinct holders of the transition-capable claim in
  one typed `(object, region, layer)` scope still conflict).
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs::claims_shrink_audit_runs_without_property_context`
  — the claims shrink audit is callable from a bare bounds profile with no `PNP_PROPERTY_CONTEXT`
  and no `PropertyRun`, while an unsafe module ceiling still rejects before allocation.
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs::shared_stage_transitive_only_cross_stage_witness`
  and
  `crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs::stable_claim_scope_is_reported_per_holder_pair`
  — the claims-remediation controls added after the original 13-test measurement: a
  transitive-only cross-stage witness and per-holder-pair reporting of a rejected stable-claim
  scope.
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs::scheduler_write_dependency_and_disabled_controls`
  — unordered writers, ordering-without-consumption, the `SupportIRx` pseudo-exemption,
  absent/removed module and capability, later-stage direct/transitive dependencies, and a removed
  competitor that must not resurrect a disabled candidate.
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs::scheduler_claim_model_ceiling_walk_controls`
  — model/generator agreement at both safety-ceiling extremes, registry-free.
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/selection_controls.rs::scheduler_claim_selection_and_pairing_controls`
  — configured-role resolution, core-short-name matching, typed classic/arachne/vase selection,
  fill/family candidate retention and incomplete-pair warnings through the real seams.
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/registry_controls.rs::scheduler_registry_oracle_and_warning_controls`
  — the explicit priority-versus-name witness, the empty/material-only/duplicate-declarer controls,
  the first-seen conflict in both declaration orders, the tied-priority payload (empty `PathBuf`,
  `field = Some("region_split")`, module ids not manifest paths), a preserved pre-existing
  diagnostic, and mutated substitutes (raw keys for the helper result, reversed tie, implicit
  seeds, omitted declaration, wrong conflict winner, lost warning, cleared sentinel).
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/manifest_controls.rs::scheduler_manifest_region_split_rejections`
  — real TOML ingestion for duplicate/scalar/priority/paint-only violations with exact
  variants/fields, plus accepted floor and core neighbours.
  `crates/slicer-scheduler/tests/common/property_scheduler_contracts/manifest_controls.rs::scheduler_aggregate_does_not_validate_builder_declarations`
  pins that builders construct already-loaded stimuli.
- `crates/slicer-scheduler/tests/common/property_scheduler_contracts/bound_controls.rs::scheduler_property_bound_and_shrink_rejections`
  — unsafe/narrowing bounds rejected before allocation, and
  missing/invalid/identity-mismatched/ineligible contexts, unknown and zero bounds, exhaustion and
  a Proptest override conflict all failing with their named kinds while a two-case passing run
  writes the promised evidence. The same control drives the three target-private shrink audits
  directly through `dag_models::run_shrink_audit`, `claim_models::run_shrink_audit` and
  `registry_models::run_shrink_audit` at both accepted ceiling extremes; that is the control's
  wiring contract, not a wall-clock measurement, and no timing is claimed for it.

## What this evidence does not prove

- It does not prove `canonical_variant_chain_order` is used by `RegionMapping`. Source inspection
  shows `commit_region_mapping_builtin` (`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`)
  passes `plan.aggregated_region_split` to `execute_region_mapping_inner`
  (`crates/slicer-core/src/algos/region_mapping.rs`), which hands the map's name-ordered keys to
  `enumerate_canonical_chains` (`crates/slicer-ir/src/region_split_registry.rs`); the helper has no
  production caller. The packet owns the helper and aggregation contract, not the driver audit
  (`proptest-openrscad_11_pbt-prepass`).
- It does not prove live per-region support aggregation or family ownership: the synthetic
  `ConflictScope::Region` observations exercise the public validator contract, and the host
  aggregation seam is elsewhere.
- It does not claim any config parsing or module-disable machinery: the disabled mode removes
  locally tagged modules and their observations before the production request, which is
  effective-input behaviour.
- It does not run the workspace suite. Narrow verification only, per AGENTS.md.

## Reproduction

```text
cargo test -p slicer-scheduler --features property_testing --test property_scheduler_contracts -- --nocapture
cargo xtask test --summary --properties --property H-DAG --property H-CLAIMS --property H-REGISTRY --tier routine
cargo xtask test --summary --properties --property H-DAG --property H-CLAIMS --property H-REGISTRY --tier extended
cargo xtask test --summary --properties --property H-CLAIMS --tier routine --seed 5053888522409382591
cargo xtask test --summary --properties --tier routine
```

Read `target/test-output.log` before starting another run; the executor appends the per-property
events, and `target/property-testing/<run-token>/<ID>/` retains `context.json`, `case.txt`,
`observations.jsonl` and (only on failure) `failure.json` for each row above.
