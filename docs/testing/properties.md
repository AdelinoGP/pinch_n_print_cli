# Property testing: registry, profiles, and workflow

Property tests run through the registry-backed `cargo xtask test --properties` lane on top of
the default-off SDK harness (`slicer-sdk/property_testing`). Selection, profiles, seeds, and
evidence are driven by the root `test-properties.toml` registry, not by directory conventions.
Seeds and seed files are diagnostic artifacts: committed regression history consists of named
concrete inputs only.

## Registry schema

`test-properties.toml` (workspace root) is the central executable registry, parsed into typed
records by `parse_registry` (`xtask/src/properties.rs`) and validated before any Cargo child is
spawned. Activation is never inferred from a directory existing.

Root keys:

| Key | Kind | Rule |
|---|---|---|
| `schema_version` | integer | must be `1` |
| `property` | array of tables | one record per property |

Per-property keys:

| Key | Kind | Rule |
|---|---|---|
| `id` | string | matches `[A-Z][A-Z0-9-]*`; unique |
| `package` | string | must exist in Cargo metadata |
| `target_kind` | string | `"test"` or `"lib"` |
| `target_name` | string | must equal a Cargo metadata target name of `package` |
| `test_name` | string | full libtest-discovered name; final `::`-component starts `proptest_`; unique per `package`+`target_name` |
| `production_symbol` | string | the production entry point the property exercises |
| `production_path` | string | workspace-relative source path of that entry point |
| `features` | array of string | owner-scoped Cargo feature specifications (e.g. `"slicer-ir/property_testing"`) |
| `requires_host_algos` | bool | owner-scoped merge retains `slicer-core/host-algos` when true |
| `requires_scad` | bool | SCAD-eligible records (none exist yet) |
| `generator_version` | positive integer | bumped when the generator semantics change |
| `charter` | string | workspace-relative Markdown path plus heading anchor (e.g. `docs/testing/properties.md#h-ir-anchors`) |
| `routine`, `extended` | profile tables | at least one required |

Per-profile keys:

| Key | Kind | Rule |
|---|---|---|
| `cases` | positive u32 | Proptest case count |
| `max_local_rejects` | u32 | local rejection budget |
| `max_global_rejects` | u32 | global rejection budget |
| `max_flat_map_regens` | u32 | `flat_map` regeneration budget |
| `max_default_size_range` | positive usize | Proptest default size range |
| `max_shrink_iters` | positive u32 | iteration-bounded shrinking |
| `max_shrink_time_ms` | u32 | `0` means no elapsed-time cutoff |
| `rng_algorithm` | string | only `"chacha"` (maps to `RngAlgorithm::ChaCha`) |
| `isolation` | string | `"in_process"` or `"fork"`; a nonzero `timeout_ms` requires `"fork"` |
| `timeout_ms` | u32 | per-case timeout; `0` disables |
| `bounds` | table of string to u64 | property-specific domain bounds (e.g. `max_entities`, `max_travels`) |

The typed parser rejects duplicate IDs and test identities, unknown keys at any level, missing
profiles, nonexistent package/target/feature claims (checked against Cargo metadata via
`validate_record` (`xtask/src/properties.rs`)), non-positive required values, unsafe charter path
components (`validate_charter` (`xtask/src/properties.rs`): workspace-relative only, no absolute
paths, `..`, or backslashes), and contradictory isolation (nonzero `timeout_ms` with
`isolation = "in_process"`). Feature merging is owner-scoped and explicit
(`merge_features` (`xtask/src/properties.rs`)): `requires_host_algos` records retain
`slicer-core/host-algos` even alongside explicit SDK/SCAD features, and that qualified feature is
never appended to a package that cannot see `slicer-core`.

## Measured profiles

Profiles are accepted only after pilots measured on the Windows development machine (warm
builds, seed 37). Each pilot launched exactly:

```text
cargo test -p slicer-ir --features property_testing --test property_ir proptest_ir_travel_anchors_membership -- --exact --nocapture
```

via the retained launcher `target/evidence/step5-pilot.py` with a subprocess-local
`PNP_PROPERTY_CONTEXT`; every configuration, context, and child output is retained under
`target/evidence/step5-*`.

| Pilot | cases | bounds (entities/travels) | reject limits (local/global/flat-map) | shrink iters | wall | cargo prep | libtest | valid/dangling observed |
|---|---:|---|---|---:|---:|---:|---:|---|
| `smoke` | 16 | 8 / 16 | 1 / 1 / 1 | 16 | 0.4691594s | 0.25s | 0.01s | 7 / 9 |
| `routine` | 64 | 32 / 64 | 0 / 0 / 0 | 64 | 0.9220788s | 0.45s | 0.21s | 26 / 38 |
| `extended` | 1024 | 256 / 512 | 0 / 0 / 0 | 256 | 8.5795951s | 0.46s | 7.87s | 503 / 521 |

All production pilots reported zero local/global rejects, zero rejected callbacks, and zero
shrink callbacks. Observed valid/dangling counts are diagnostics, not coverage assertions;
mandatory structural witnesses cover the rare shapes deterministically (see the charter).
Mutation shrink pilots (iteration limits 16/64/256) each made four callbacks and retained `[1]`,
with callback-run times of 311.7µs / 77.2µs / 75.4µs. Shrink completion is reported `null`
throughout: proptest 1.11 cannot report whether minimization finished, so the harness never
claims a retained case is minimal when a limit could have hit.

Chosen live profiles (committed in `test-properties.toml`):

| Key | routine | extended |
|---|---:|---:|
| `cases` | 64 | 1024 |
| `max_local_rejects` | 0 | 0 |
| `max_global_rejects` | 0 | 0 |
| `max_flat_map_regens` | 0 | 0 |
| `max_default_size_range` | 32 | 256 |
| `max_shrink_iters` | 64 | 256 |
| `max_shrink_time_ms` | 0 | 0 |
| `rng_algorithm` | chacha | chacha |
| `isolation` | in_process | in_process |
| `timeout_ms` | 0 | 0 |
| `bounds.max_entities` | 32 | 256 |
| `bounds.max_travels` | 64 | 512 |

Zero rejection budgets are honest for this property because its generator is total (no
`prop_assume`); any rejection at all signals a generator change and aborts as `exhausted` rather
than silently consuming budget. Both bound pairs permit empty, singleton, repeated-anchor,
missing-anchor, and reordered cases.

## Context and event schema

`PNP_PROPERTY_CONTEXT` is one JSON object set on each selected execution child (serialized, never
shell-interpolated). Keys: `schema_version: 1`, `run_token`, `property_id`, `tier`
(`"routine" | "extended"`), `seed` (u64), `package`, `target_kind`, `target_name`, `test_name`,
`features` (array of string), `profile` (the profile keys above, with `bounds` nested),
`artifact_dir`, `proptest_version`, `generator_version`, `source_revision`, `dirty_paths`,
`source_state_digest`, `cargo_target_dir` (string or null), `completion_eligible`. The producer
is `build_property_context` (`xtask/src/properties.rs`); the consumer is `PropertyRun::new`
(`crates/slicer-sdk/src/test_support/property_testing/run.rs`), which rejects missing context
with an actionable instruction, rejects identity mismatches, and never silently chooses an
unmeasured profile. The source-state digest covers the revision, tracked diff bytes, and
relevant untracked source/manifest/config bytes (`source_state` (`xtask/src/properties.rs`)):
changing dirty contents changes the digest.

SDK events are newline-delimited `PNP_PROPERTY_EVENT ` followed by one JSON object. Required
envelope keys: `schema_version: 1`, `run_token`, `property_id`, `phase: "begin" | "end"`,
`process_role: "controller" | "worker"`. End events add `outcome: "passed" | "failed" |
"exhausted"`, `accepted_cases: u64 | null`, `accepted_test_calls`, `rejected_test_calls`,
`runner_statistics`, `shrink_completed: bool | null`, `completion_eligible`. Semantics:

- Matching controller `begin`/`end` pairs are the only completion evidence. Fork workers tag
  `process_role: "worker"` and never impersonate or duplicate controller completion.
- Process-tagged per-case observations (pid, role, attempt, result) are retained in
  `observations.jsonl`, because in-memory parent counters cannot observe forked child execution.
- Call counters include shrink invocations. `accepted_cases` is `null` on unsuccessful runs
  unless it can be established truthfully; unavailable counts are labeled, never fabricated
  from private Proptest state. `runner_statistics` is the runner's public `Display` string.
- `shrink_completed` is `null`/unknown unless the harness can establish completion from its own
  iteration-bounded accounting; it is never claimed when a limit could have hit.

## Execution, replay and failure workflow

Commands (registry-backed execution through `cargo xtask test`):

```text
cargo xtask test --summary --properties --tier routine
cargo xtask test --summary --properties --tier extended
cargo xtask test --summary --properties --property H-IR-ANCHORS --seed 42
```

`--tier` defaults to `routine`, and repeatable `--property ID` flags restrict the selected
records. The runner performs the ordinary test preflights, guest freshness/rebuild and CLI
freshness gates once, then discovers each selected feature-enabled target with libtest `--list`
and `--list --ignored`. It compares the discovered `proptest_` names with the registry in both
directions for that target and rejects a missing or ignored selected test. It then executes the
exact registered test with `--exact <test_name> --nocapture`, supplying its serialized
`PNP_PROPERTY_CONTEXT` JSON to the execution child.

Discovery and execution child stdout and stderr are retained together in
`target/test-output.log`, with a delimiter, command and status for each child; earlier child
output remains available even if a later child fails. A log-write failure fails the run. A passing
child alone is not enough: the executor requires exact libtest success and matching controller
`begin` and `end` events for the property and run token, with `outcome = "passed"`,
`accepted_cases >= cases` for the selected profile, and `completion_eligible = true` on the end
event. Worker events cannot substitute for controller completion. An unrelated passing test,
zero cases, a compile-only invocation or missing completion evidence is not success.

Property-only flags require `--properties`. Unknown IDs or tiers, malformed seeds, empty
selections, `--summary-from` combined with property selectors, ordinary package/test/filter/libtest
selectors in property mode, and `--properties --accelerated` are usage errors. Conflicting
inherited sampling/size/rejection/shrink/seed/RNG/fork/timeout controls (`PROPTEST_CASES`,
`PROPTEST_RNG_ALGORITHM`, `PROPTEST_MAX_DEFAULT_SIZE_RANGE` and the rest of the measured set) are
rejected before any gate or child runs with exit `1` and `profile_override_conflict: <NAME>`; the
logging-only `PROPTEST_VERBOSE` passes through and is recorded as `proptest_verbose_override`.
Direct SDK harness callers receive the same `profile_override_conflict` diagnostic.

Sampling and replay: a fresh OS-entropy-backed seed is resolved before spawning cases
(`resolve_seed` (`xtask/src/properties.rs`)) and recorded per run; `--seed U64` replays one
recorded concrete seed across all selected properties. Two random draws are not guaranteed to
produce different seeds. Seeds are not committed regression history: a durable regression is a
named concrete input, for example in `crates/slicer-ir/tests/ir_validation_tdd.rs`.

Artifacts under `target/property-testing/<run_token>/<property_id>/`:

| File | Written | Content |
|---|---|---|
| `context.json` | before execution | the exact context object |
| `case.txt` | before each SUT callback | per-attempt concrete input (index, phase `initial`/`shrink`, debug value) |
| `failure.json` | best-effort on failure | classification, context, concrete/debug input, tier-bearing reproduction command, executable direct reproduction (`reproduction_direct`: cargo argv plus the exact `PNP_PROPERTY_CONTEXT`), captured SDK logs, shrink status |
| `observations.jsonl` | per case observation | process-tagged case records (pid, role, attempt, result) |

Failure classifications: `sut_or_contract`, `generator_or_oracle`, `exhausted`, `panic`,
`process_failure`, `harness`. An unexplained assertion is classified `sut_or_contract` pending
investigation and is never labeled a confirmed production defect before root-causing. Exhaustion
(rejection budget spent) is distinguished from assertion failure.

Exit-status policy: a child failure wins over missing evidence; ordinary child exit codes
`1..=255` are preserved (a controlled exit `17` remains `17`); an otherwise unrepresentable
nonzero Windows status or abnormal termination maps to exit `1`, never wrapped to success;
missing or zero property execution with an otherwise successful child returns `1` with
`missing_property_execution`; usage errors return `2` before any gate or child runs. `--with-scad`
is parsed and reserved: with no eligible SCAD record registered it returns usage exit `2` with
`no_eligible_scad_properties`. Source-adjacent seed persistence is disabled
(`failure_persistence: None`); no `proptest-regressions*` file is ever written beside sources.

## H-IR-ANCHORS

Registry record: `id = "H-IR-ANCHORS"`, `package = "slicer-ir"`, `target_kind = "test"`,
`target_name = "property_ir"`, `test_name = "proptest_ir_travel_anchors_membership"`,
`features = ["slicer-ir/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`H-IR-ANCHORS` — semantic (travel-anchor membership on the IR), independent of packet numbers.

### Owner

Package `slicer-ir`; test target `property_ir` (integration target with
`required-features = ["property_testing"]`); test `proptest_ir_travel_anchors_membership` in
`crates/slicer-ir/tests/property_ir.rs`; production symbol `slicer_ir::validate_travel_anchors`
at `crates/slicer-ir/src/validation.rs`. Shared oracle helpers live in
`slicer_sdk::test_support::property_testing`.

### Contract

Per the doc comment on `validate_travel_anchors` (`crates/slicer-ir/src/validation.rs`): every
`TravelMove.entity_id` must be a member of `LayerCollectionIR.ordered_entities` (via
`PrintEntity.entity_id`), and a dangling anchor yields an `Err(String)` identifying the missing
id. This is a library-validator property; it does not claim the runtime currently calls the
validator, and it certifies nothing about printable geometry.

### Domain

Bounded generated lists of entity IDs and travel anchor IDs (`max_entities` / `max_travels`
bounds via `PropertyRun::bound` (`crates/slicer-sdk/src/test_support/property_testing/run.rs`)).
IDs are opaque u64 and membership is exact integer comparison. `0` and `u64::MAX` are legitimate
members as well as legitimate dangling ids: this validator is not an allocator or sentinel
validator. Meaningful degenerate cases: empty collection, singleton, repeated anchors, reordered
`ordered_entities`, and multiple missing anchors (first in travel order is the reported one).
Dangling cases are in scope and are never filtered out of the property.

### Strategy

Local generator in `crates/slicer-ir/tests/property_ir.rs` (bounded vec pairs of entity IDs and
anchors, generating already-dangling cases as well as valid ones). Mandatory structural
witnesses run deterministically inside `proptest_ir_travel_anchors_membership` on every
execution: empty collection, valid singleton, dangling anchor, multiple missing anchors
(first-in-travel-order assertion), repeated anchors (valid and dangling), reordered
`ordered_entities`, and extreme IDs `0` / `u64::MAX` as member and as dangling. Size dimensions
are the two list lengths against the registry bounds. Entities are built with FRU over
`print_entity_base` (`crates/slicer-sdk/src/test_support/fixtures.rs`); layers and travels use
`Default` plus the fields under test. Empty paths and other layer fields are immaterial to this
validator and are not certified as printable geometry. A later domains packet replaces the local
strategy with the shared `anchor_cases` without changing property identity.

### Oracle

`travel_anchor_oracle` (`crates/slicer-sdk/src/test_support/property_testing/oracle.rs`):
independent sorted/deduplicated-vector membership over the generated `ordered_entities` IDs
only. It never calls `validate_travel_anchors`, never reuses its `HashSet` helper, and never
looks up entities outside `ordered_entities`. Shared dependency: `check_travel_anchors_charter`
(same module) performs the per-case comparison between oracle expectation and validator output.
Independence rationale (source plan §5.1): a production function's own validator is not
automatically an independent oracle; this oracle is a separate reference computation over the
same generated input. The oracle is mutation-probed by `property_anchor_oracle_catches_accept_all`
(`xtask/src/properties.rs` test module) against a synthetic accept-all validator substitute on a
dangling witness; the substitute is never registered as a production property.

### Tolerance

Exact integer/set comparison. The error shape is checked as tokens: the message must carry the
`entity_id` token, the first missing ID in travel order as a standalone decimal token (digit-wise
comparison, so `1` cannot match inside `11`), and `layer.ordered_entities`. No numeric tolerance
applies.

### Shrinking

May simplify: list lengths and individual ID values. Must remain valid: the membership outcome is
derived per case from the oracle, so shrinking needs no mode filter and invalid membership is
never filtered out (mode preservation would only be required for mode-specific assertions, which
this property does not use). Shrink limits are profile-bounded (`max_shrink_iters`,
`max_shrink_time_ms = 0` = no elapsed-time cutoff), which makes replay comparisons independent of
wall-clock behaviour. Shrink completion is reported `null`/unknown (proptest 1.11 cannot identify
it); a retained case is not claimed minimal when a limit could have hit.

### Execution

Both tiers, values as in Measured profiles (routine 64 cases, extended 1024 cases;
`isolation = "in_process"` — forking is a per-profile choice and is not required for this cheap
property; `timeout_ms = 0` in both). Features `["slicer-ir/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. All pilot measurements were taken on
Windows.

### Failure record

`failure.json` retains the classification (initially `sut_or_contract` pending investigation),
the full context, the concrete/debug input, the tier-bearing reproduction command
(`cargo xtask test --summary --properties --property H-IR-ANCHORS --tier <routine|extended> --seed <seed>`),
an executable direct reproduction (`reproduction_direct`: cargo argv plus the exact consumed
`PNP_PROPERTY_CONTEXT`), captured SDK logs (retained case logs; teardown never discards the only
failure log), and shrink status.
Root-cause regression destination for a confirmed defect: `crates/slicer-ir/tests/ir_validation_tdd.rs`
as a named concrete input regression — never a seed.

### Acceptance

The target is actually executed with `--exact --nocapture` (never compile-only, never an
unrelated passing test); the oracle is checked every case; generator and shrink validity are
demonstrated by the deterministic structural witnesses and the accept-all mutation probe;
existing tests pass (`ir_validation_tdd`, `entity_id_invariants_tdd`, SDK `property_harness`).

Defect outcome for this slice (AC-N4): **no production defect in `validate_travel_anchors` was
observed**; the synthetic accept-all substitute was rejected by
`property_anchor_oracle_catches_accept_all`; no regression was manufactured.
