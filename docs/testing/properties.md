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
| `requires_scad` | bool | record needs the native SCAD engine; its profiles must also carry a positive `test_process_timeout_ms` bound |
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

A `requires_scad = true` record must additionally carry a positive
`bounds.test_process_timeout_ms`. The executor applies it to the whole libtest
execution child — recipe generation, candidate rendering, shrink rendering and
the property callbacks — and terminates the Windows process tree on deadline.
The child is created suspended, assigned to a kill-on-close Job Object, and only
then resumed, so a descendant cannot be created before containment is
established. Pipe capture is bounded and reports an incomplete tail instead of
silently truncating it. That is not a substitute for Proptest's per-case
`timeout_ms`, and neither one substitutes for the other.

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

### Direct-domain pilots (packet `proptest-openrscad_02_pbt-domains`)

Measured on the Windows development machine on 2026-11-10, rustc 1.96.0, `dev` profile,
workspace `target/`, Proptest 1.11.0, `generator_version = 1`, features
`["slicer-sdk/property_testing"]`. Source state at measurement:
`source_revision = 56dae6bc4c51194d437eb23873e1d42f07452699`, dirty paths
`["test-properties.toml"]`, `source_state_digest = c421c41a5799…`. Every run reported
`call_counts_complete = true`, `accepted_cases == cases`, `rejected_test_calls == 0` and
`shrink_completed = null` (Proptest 1.11 cannot report whether shrinking finished).

Routine (`--tier routine`, 64 cases each; libtest wall per property):

| Property | libtest wall | accepted | rejected |
|---|---:|---:|---:|
| `D-SCALARS` | 0.03s | 64 | 0 |
| `D-GRAPHS` | 0.04s | 64 | 0 |
| `D-IR` | 0.26s | 64 | 0 |
| `D-GEOMETRY` | 0.19s | 64 | 0 |
| `D-MESH` | 0.03s | 64 | 0 |

Extended (`--tier extended`, 1024 cases each):

| Property | libtest wall | accepted | rejected |
|---|---:|---:|---:|
| `D-SCALARS` | 0.46s | 1024 | 0 |
| `D-GRAPHS` | 0.94s | 1024 | 0 |
| `D-IR` | 41.76s | 1024 | 0 |
| `D-GEOMETRY` | 8.39s | 1024 | 0 |
| `D-MESH` | 0.46s | 1024 | 0 |

`cargo xtask test` wall including the literal preflight, guest freshness check and
`pnp_cli` relink: routine 34.99s (12.83s of which was the relink); extended 67.10s.
`H-IR-ANCHORS` routine after adopting the shared anchor helpers: libtest 0.10s, xtask wall
31.77s. Contract target `property_domain_contracts`: 11 tests in 5.20s, including the native
fixture probe (first cold probe build 51.24s, warm 1.24s).

Explicit-seed replays using the retained seeds: `D-GRAPHS` routine
`--seed 5412186833007693115` passed (xtask wall 11.36s) and `D-GEOMETRY` extended
`--seed 1173614043627998710` passed (xtask wall 18.45s).

Chosen live profiles (committed in `test-properties.toml`; profile keys other than bounds are
identical to the foundation: 64/1024 cases, zero reject budgets, size range 32/256, shrink
iterations 64/256, `in_process`, `timeout_ms = 0`):

| Key | routine | extended |
|---|---:|---:|
| `D-SCALARS` `max_scalar_units` / `max_id_bytes` | 10 000 / 64 | 100 000 / 256 |
| `D-GRAPHS` `max_nodes` | 16 | 32 |
| `D-IR` `max_entities` / `max_travels` / `max_points` / `max_layers` | 16 / 32 / 8 / 8 | 64 / 128 / 16 / 16 |
| `D-GEOMETRY` `max_abs_units` / `max_vertices` / `max_holes` / `max_components` / `max_shrink_candidates` | 1 000 / 32 / 4 / 4 / 64 | 10 000 / 64 / 8 / 8 / 256 |
| `D-MESH` `max_extent_mm` / `max_translation_mm` | 128 / 128 | 1 000 / 1 000 |

Reasons for the selected values: routine bounds keep every family sub-second while still
covering the interesting IEEE, graph, membership and topology shapes; extended bounds grow the
domains (32-node graphs, 8-component geometry, 10 mm coordinates, 1 m meshes) without exceeding
the exact-arithmetic cap of 1 000 000 units. `max_entities * max_points * max_layers` is
16 384 for the extended `D-IR` profile, below the constructor's 100 000 allocation guard, and
`max_nodes = 32` is the constructor's hard cap. Zero rejection budgets are honest because every
generator is total; a rejection would signal a generator change and abort as `exhausted`.

The existing `properties-windows` CI job runs `cargo xtask test --summary --properties
--tier routine` with no hand-maintained record list, so it picks up all five new records
automatically. The routine lane was executed here on Windows; the CI execution itself is not
claimed from this workstation run.

The six `H-*` records added by packet `proptest-openrscad_06_pbt-ir-config` were calibrated the
same way and their pilot, routine, extended and retained-seed-replay measurements — per-record
walls, accepted-case counts, per-record bounds and the reference numeric proofs — are recorded
in [proptest-ir-config-evidence.md](proptest-ir-config-evidence.md) rather than duplicated
here. Their committed profiles use the same shape (`cases` 64/1024, zero rejection budgets,
`max_default_size_range` 32/256, `max_shrink_iters` 64/256, chacha, `in_process`,
`timeout_ms = 0`) with family-specific bounds; the coordinate pair additionally satisfies
`0 < 10_000·max_abs_mm ≤ 2^24`, `0 < max_abs_units ≤ 2^24` and
`min(max_abs_units, 10_000·max_abs_mm) ≤ 1_000_000` so the shared rounding-boundary helper
receives a legal argument.

The three scheduler records added by packet `proptest-openrscad_07_pbt-scheduler` (`H-DAG`,
`H-CLAIMS`, `H-REGISTRY`) were calibrated the same way; their pilot, routine, extended and
retained-seed-replay measurements, per-record walls, accepted-case counts, bounds and
oracle/control provenance are recorded in
[proptest-scheduler-evidence.md](proptest-scheduler-evidence.md) rather than duplicated here. Their
committed profiles use the same shape (`cases` 64/1024, zero local/global/flat-map rejection
budgets, `max_default_size_range` 32/256, `max_shrink_iters` 64/256, chacha, `in_process`,
`timeout_ms = 0`) with scheduler-specific bounds. The existing `properties-windows` job runs
`cargo xtask test --summary --properties --tier routine` with no hand-maintained record list, so it
selects the three new records automatically; the combined routine selection was executed here on
Windows and reported `VERDICT: PASS`.

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

Local generator replaced by the shared `anchor_cases`
(`slicer_sdk::test_support::property_testing::domains`) unioned over
`AnchorDomain::Valid` and `AnchorDomain::Dangling`, without changing property identity. Mandatory structural
witnesses run deterministically inside `proptest_ir_travel_anchors_membership` on every
execution: empty collection, valid singleton, dangling anchor, multiple missing anchors
(first-in-travel-order assertion), repeated anchors (valid and dangling), reordered
`ordered_entities`, and extreme IDs `0` / `u64::MAX` as member and as dangling. The
IR/config packet adds one required witness: an ID that appears only in an unrelated side table
(`speed_profiles`) is still dangling. A test-local "side-table union" reference (unioning
`ordered_entities` with the `speed_profiles` and `support_entity_identities` keys) must answer
`None` on that witness while both the independent oracle and `validate_travel_anchors` report
the missing id, so the mutation is provably caught rather than merely absent. Size dimensions
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

## H-COORD

Registry record: `id = "H-COORD"`, `package = "slicer-ir"`, `target_kind = "test"`,
`target_name = "property_ir"`, `test_name = "proptest_coord_conversion_envelopes"`,
`production_symbol = "slicer_ir::mm_to_units"`,
`production_path = "crates/slicer-ir/src/slice_ir.rs"`,
`features = ["slicer-ir/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-coord"`.

### Stable property ID

`H-COORD` — bounded mm↔unit conversion envelopes for the canonical 100 nm unit.

### Owner

Package `slicer-ir`; test target `property_ir` (integration target with
`required-features = ["property_testing"]`); test `proptest_coord_conversion_envelopes` in
`crates/slicer-ir/tests/property_ir.rs`. Production surface: `slicer_ir::mm_to_units`,
`slicer_ir::units_to_mm`, `Point2::from_mm` / `Point2::to_mm` and `UNITS_PER_MM`
(`crates/slicer-ir/src/slice_ir.rs`), plus `slicer_sdk::coords::{mm_to_units, units_to_mm,
SCALING_FACTOR}` (`crates/slicer-sdk/src/coords.rs`) as the module-author wrappers. Shared
reference models live in `crates/slicer-ir/tests/common/property_oracles.rs`.

### Contract

Per `docs/08_coordinate_system.md` § Conversion & Determinism: one unit is 100 nm
(`1 unit = 10⁻⁴ mm`), so the factor is `10_000` units per millimetre. The raw helpers perform
a plain f32 multiply-and-round (mm→units) and a divide (units→mm); they promise **no**
validation, so non-finite or unbounded inputs are outside this charter. The property certifies
two things only:

1. a derived absolute-error envelope per direction over a checked finite domain, and
2. exactness of the documented half-away tie rule at `m = 1/32 mm`.

It does **not** assert a universal f32 inverse. `mm_to_units(units_to_mm(n)) == n` and
`units_to_mm(mm_to_units(m)) == m` are both false in general (the `1/32 mm → 313 → 0.0313 mm`
witness shows it), and the test says so in a comment rather than asserting the stronger claim.

### Domain

Profile integers `M = max_abs_mm` and `U = max_abs_units`, constrained by
`CoordProfile::new` (`crates/slicer-ir/tests/common/property_oracles.rs`) to

- `M` finite and positive, `U > 0`;
- `0 < 10_000·M ≤ 2^24`, and `U ≤ 2^24`;
- `min(U, 10_000·M) ≤ 1_000_000`, the cap of the shared
  `slicer_sdk::test_support::property_testing::domains::rounding_boundary_mm` helper.

These inequalities keep the scaled products finite, keep integer outputs far from cast
saturation, and keep integer-to-f32 inputs exact. They are safety conditions; they prove
nothing about inverses. Generated mm inputs are finite f32 in `[-M, M]` from the shared
`finite_f32` strategy (which reaches both zero signs and subnormal witnesses), plus
`rounding_boundary_mm(min(U, 10_000·M))` so the half-unit boundaries around
`(k + 0.5)/10_000 mm` are populated. Unit inputs are integers in `[-U, U]`. Ordered pairs for
monotonicity are drawn from the same mm domain.

### Strategy

One tagged value type (`CoordCase`) with three structurally distinct variants, so shrinking
cannot turn a millimetre case into a unit case or into an unordered pair:

- `Millimetres(f32)` — `finite_f32(-M, M)` (weight 4) and the boundary strategy (weight 2);
- `Units(i64)` — `-U..=U`;
- `OrderedMillimetres(f32, f32)` — a pair normalised to `left <= right`.

Mandatory deterministic witnesses run before sampling on every execution: `1/32 mm` and its
negative mirror (`±313`), the three floats around each side of that half case, both zero signs,
two subnormal witnesses, both interval ends, the unit envelope at
`{-U, -1, 0, 1, U}`, monotonicity at the interval ends, `UNITS_PER_MM == 10_000` and
`SCALING_FACTOR == 10_000`, and exact `Point2` integer identity at `i64::MIN` / `i64::MAX`
through `postcard` (never through a float).

### Oracle

`check_mm_case`, `check_units_case`, `check_monotonic_pair` and `check_mm_case_with` in
`crates/slicer-ir/tests/common/property_oracles.rs`. For exact real input value `m`
represented by f32, `t = 10_000·m` is computed exactly in f64 on this domain (a 24-bit
significand times the integer factor fits f64 precision). With f32 round-to-nearest
multiplication, the proved error is `e_mul ≤ 2^-24·|t| + 2^-150` **units**, so

```text
|mm_to_units(m) - t| ≤ 1/2 + e_mul                     (units)
```

For integer `n` with `|n| ≤ U` the f32 input cast is exact and

```text
|units_to_mm(n) - x| ≤ (2^-24 + 2^-53)·|x| + 2^-150  (millimetres),  x = n/10_000
```

The reference arithmetic is f64, not real arithmetic. The `2^-24·|x|` term is the f32 division's
half ulp; the `2^-53·|x|` term is the **f64 reference division's own** rounding error, which is
not below the `2^-150` floor (for `n = 1` it is about `4.8e-21`, while `2^-150` is about
`7.0e-46`), so it is added explicitly. `2^-150` remains only as the subnormal floor, where a
relative bound underflows to zero. Both allowed-error values are outward-rounded (`next_up`) so
the comparison can never be tighter than the derivation. `check_mm_case` additionally
asserts exact agreement between `slicer_ir::mm_to_units`, `slicer_sdk::coords::mm_to_units`
and `Point2::from_mm` — explicitly **consistency evidence only**, not correctness evidence,
because all three share the same implementation shape. Correctness rests on the analytical
envelopes and the half-case derivation. `expected_half_away` re-derives the tie rule from
`10_000·m` in f64 and half-away-from-zero rounding, so a truncating implementation is
distinguishable from a rounding one.

### Tolerance

The tolerance is derived, never tuned: `1/2 + 2^-24·|t| + 2^-150` units and
`(2^-24 + 2^-53)·|n|/10_000 + 2^-150` mm, both outward-rounded. The mutation controls in
`ir_coord_oracle_and_bound_controls` (`crates/slicer-ir/tests/property_ir_contracts.rs`)
prove the bounds falsify a factor-of-100 scaling mutant and a truncating mutant at the exact
half case, and that the envelope **alone** accepts truncation there (truncation is exactly
`0.5` units off, which the packet's bound admits) — which is why the tie rule is checked
separately by `check_mm_case_with` instead of by tightening the envelope. The
"universal exact inverse" claim is falsified by a concrete pair rather than accepted.

### Shrinking

Shrinks move within `finite_f32(-M, M)`, `rounding_boundary_mm` and `-U..=U`, so every shrink
stays inside the declared bounds. The case tag is pinned by
`property_oracles::mode_locked` — a `no_shrink` mode selector fed through `prop_flat_map` — so
the payload tree shrinks inside the originally generated variant and a unit case cannot become
a millimetre case mid-shrink (the mechanism is probed by
`mode_lock_preserves_tags_under_simplification`). Boundary and zero-sign witnesses
are reachable through the deterministic block regardless of what the random sampler produces;
the shared `finite_f32` and `rounding_boundary_mm` strategies preserve the finite bound in
their own value trees. Shrink limits are profile-bounded (`max_shrink_iters`,
`max_shrink_time_ms = 0` = no elapsed-time cutoff). Shrink completion is reported
`null`/unknown: proptest 1.11 cannot identify it, so a retained case is never claimed minimal.

### Execution

Both tiers. `isolation = "in_process"`, `timeout_ms = 0`, `rng_algorithm = "chacha"`,
`cases = 64` routine / `1024` extended, `max_default_size_range = 32` / `256`,
`max_shrink_iters = 64` / `256`, rejection budgets `0` (the domain is total).
Bounds `max_abs_mm = 25 → 100`, `max_abs_units = 250_000 → 1_000_000`. Every run was executed
on Windows through `cargo xtask test --summary --properties --property H-COORD --tier
<routine|extended>`; see `docs/testing/proptest-ir-config-evidence.md` for measured case
counts, walls and the retained-seed replay.

### Failure record

`failure.json` retains the classification (`sut_or_contract`, `generator`, `oracle`, or
`contract`), the full context, the concrete/debug input, the tier-bearing reproduction command
(`cargo xtask test --summary --properties --property H-COORD --tier <tier> --seed <seed>`),
an executable direct reproduction, captured SDK logs and shrink status. Root-cause regression
destination for a confirmed defect: a named concrete input in
`crates/slicer-ir/tests/common/property_oracles.rs`'s controls or a dedicated named test — never
a seed.

### Acceptance

The target is actually executed with `--exact --nocapture`; the derived envelope and the exact
half-away pair are checked every execution; the mutation controls reject the scaling,
truncation and false-inverse claims; `ir_property_domain_rejections` rejects unsafe profiles
before sampling. Defect outcome: **no production defect in the conversion helpers was
observed**; no regression was manufactured.

## H-IDS

Registry record: `id = "H-IDS"`, `package = "slicer-ir"`, `target_kind = "test"`,
`target_name = "property_ir"`, `test_name = "proptest_entity_and_modifier_ids"`,
`production_symbol = "slicer_ir::LayerEntityIdGen::next"`,
`production_path = "crates/slicer-ir/src/entity_id.rs"`,
`features = ["slicer-ir/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-ids"`.

### Stable property ID

`H-IDS` — bounded allocation sequence plus representable modifier-id parents.

### Owner

Package `slicer-ir`; test target `property_ir`; test `proptest_entity_and_modifier_ids` in
`crates/slicer-ir/tests/property_ir.rs`. Production surface: `LayerEntityIdGen::{new, next}`
(`crates/slicer-ir/src/entity_id.rs`) and `modifier_sub_region_id`,
`modifier_sub_region_id_fits`, `is_modifier_namespace_id`, `modifier_base_region_id`,
`MODIFIER_FOOTPRINT_REGION_ID`, `MODIFIER_VARIANT_REGION_ID_STRIDE`
(`crates/slicer-ir/src/slice_ir.rs`). The namespace flag itself is private; the charter states
its documented value `2^63` rather than importing an invented public constant.

### Contract

`LayerEntityIdGen::new` starts at `1`; `next` returns a strictly monotonic allocation and never
returns the reserved `0` sentinel. The type is intentionally `!Send + !Sync` (`Cell<u64>` plus
`PhantomData<*const ()>`); that static assertion stays in `entity_id_invariants_tdd` and this
property does not change concurrency or exhaustion semantics. Allocation exhaustion is **not**
exercised by looping to `u64::MAX`; the property is explicitly bounded below overflow.

For modifier sub-region ids, with `S = MODIFIER_VARIANT_REGION_ID_STRIDE` (1_000_003),
`B = 2^63` and `P = floor((B - S - 1)/S)`, the documented worst-case payload
`parent·S + (S - 1)` fits in the namespace iff `parent ≤ P`. `P` is derived at runtime with
checked `u128` arithmetic in `representable_parent_limit`, and `payload_fits_model` re-derives
the same predicate independently of `modifier_sub_region_id_fits`. For a supported parent the
minted id carries the high-bit namespace marker over `parent·S + hash` with
`1 ≤ hash ≤ S - 1`; `modifier_base_region_id` recovers `parent`.

The property claims **no universal injectivity**: distinct parents cannot share an id for
identical object/footprint inputs (the quotient recovers the parent), but distinct
object/footprint inputs can collide inside the bounded stride of 1_000_003 values. Unequal
hashes are never required. Repeated identical inputs are determinism evidence, not an
independent hash oracle. No geometry-validation claim is made: hashing does not certify the
footprint.

### Domain

- Allocations: `0..=max_allocations` (256 routine / 4096 extended, both far below `u64::MAX`).
- Parents: `0..=P` sampled for the supported mode; `{P + 1, u64::MAX}` for the unsupported
  mode.
- Object text: the shared `slicer_sdk::test_support::property_testing::domains::id_text`
  domain, bounded by `max_id_bytes` (64 / 256); the UUID family (36 bytes) is used when the
  bound admits it, otherwise the non-UUID complement family.
- Footprints: integer-grid `ExPolygon` values with at most `max_footprint_points` (8 / 32)
  total points, including an empty footprint and a holed footprint.

### Strategy

One tagged `IdCase` with three structural variants — `Allocations(u64)`,
`SupportedModifier { parent, object_id, footprint }`, `UnsupportedParent { parent }` — so
shrinking cannot move a supported parent into the unsupported arm nor an allocation length into
a modifier case. Deterministic mandatory witnesses run before sampling: zero and singleton
allocations, the full `1..=64` sequence against `1..=n`, two fresh generators agreeing,
`fits(P)` true, `fits(P + 1)` and `fits(u64::MAX)` false, model/predicate agreement over
`{0, 1, 2, S-1, S, S+1, P-1, P}` and at `P + 1`, the namespace/payload/decode rules for
`{0, 1, 2, S, P}`, determinism on identical inputs, distinct ids for distinct parents under
shared object/footprint inputs, ordinary low-bit ids and the footprint sentinel decoding to
`None`, and a never-minted bit-marked id still decoding to `Some(7)` (namespace membership is
not provenance validation).

### Oracle

`property_oracles::{representable_parent_limit, payload_fits_model, expected_payload_bounds,
decode_model, allocation_model, footprint}`. `decode_model` is a direct quotient/remainder
decode of the documented payload shape, not a call into the production helpers.
`allocation_model(n)` is the mathematical sequence `1..=n`. The production
`modifier_sub_region_id_fits` is compared against `payload_fits_model` rather than trusted.

### Tolerance

Exact integer comparison throughout; no numeric tolerance exists on this surface.

### Shrinking

Shrinks remove allocations or simplify text/footprint values while preserving the variant tag:
the tag is pinned by `property_oracles::mode_locked`, so a supported parent stays supported
(`≤ P`) and an unsupported one stays at `P + 1`/`u64::MAX` instead of shrinking into another
arm. The mechanism is probed by `mode_lock_preserves_tags_under_simplification`. The documented
minting assertion is **not** re-triggered per generated case: `modifier_sub_region_id` asserts
rather than returning a `Result`, and minting every unsupported sample would emit one panic per
case. The assertion text
(`modifier sub-region id payload exceeds its namespace or reserved sentinel`) is observed
exactly once per run, deterministically, by `ir_id_parent_and_sentinel_rejections` in
`crates/slicer-ir/tests/property_ir_contracts.rs` via `catch_unwind`, including a `P + 1` and a
`u64::MAX` probe. This keeps the retained property log free of panic output while the static
assertion still has a named, falsifiable observer.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, `cases = 64 / 1024`,
`max_default_size_range = 32 / 256`, `max_shrink_iters = 64 / 256`, rejection budgets `0`.
Bounds `max_allocations = 256 → 4096`, `max_id_bytes = 64 → 256`,
`max_footprint_points = 8 → 32`. Measured on Windows through the registry-backed command; see
`docs/testing/proptest-ir-config-evidence.md`.

### Failure record

As for `H-COORD`; the direct reproduction carries the same consumed `PNP_PROPERTY_CONTEXT` and
the tier-bearing registry command. Root-cause regression destination: a named concrete test
(allocator or modifier-id), never a seed.

### Acceptance

The allocation sequence equals `1..=n` for every generated length, the derived `P` boundary is
observed from both sides, and namespace/payload/decode/sentinel rules hold for every supported
parent. Runtime collision detection is explicitly **not** claimed here: it is the obligation of
`proptest-openrscad_08_pbt-runtime` (`split_modifier_footprints` transactionality) and
`proptest-openrscad_11_pbt-prepass` (region-map caller behaviour). Defect outcome: **no
production defect was observed**; no regression was manufactured.

## H-IR-SERDE

Registry record: `id = "H-IR-SERDE"`, `package = "slicer-ir"`, `target_kind = "test"`,
`target_name = "property_ir"`, `test_name = "proptest_ir_serialization_and_defaults"`,
`production_symbol = "slicer_ir::LayerCollectionIR"`,
`production_path = "crates/slicer-ir/src/slice_ir.rs"`,
`features = ["slicer-ir/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-ir-serde"`.

### Stable property ID

`H-IR-SERDE` — finite-domain serialization of a deliberately selected IR slice.

### Owner

Package `slicer-ir`; test target `property_ir`; test `proptest_ir_serialization_and_defaults`
in `crates/slicer-ir/tests/property_ir.rs`. Complementary deterministic controls live in
`ir_serde_defaults_and_nonfinite_policy`
(`crates/slicer-ir/tests/property_ir_contracts.rs`).

### Contract

For a bounded, finite, dyadic IR slice, a Postcard round trip and a finite-domain JSON round
trip preserve: entity IDs, entity ordering, `ExtrusionRole` values, `RegionKey` identity,
`PrintEntity.tool_index`, `ExtrusionPath3D.{role, speed_factor bits, tool_index, order_lock}`,
point counts and the selected point float bits, travel anchors and their optional coordinate
bits, `speed_profiles` rows with per-point factor bits, and `support_entity_identities` rows
with their family/body/demand strings. Integer `Point2` geometry (`ExPolygon`) survives both
codecs exactly, including `i64::MIN`/`i64::MAX`.

This is a **selected slice**, not certification of every IR type and not a printable-validity
claim. The observations are captured from the source value before encoding by
`observe_layers`, which reads fields directly and never calls either codec; the encoder/decoder
pair is the surface under test, which is why the test carries a `// test-quality:` waiver
naming the pair.

Nonfinite policy is boundary-specific: JSON correctness properties here admit finite dyadic
records only, and primitive/`ConfigValue` nonfinite serialization may produce `null` and is
never credited as a lossless successful JSON round trip. A JSON `null` in a required `f32`/`f64`
field is rejected on decode. Postcard is separately tested on raw scalar/`ConfigValue` bit
patterns, including signed zero, infinities and NaN payloads, comparing bits rather than
native NaN equality — and those cases are explicitly representation-only, never
production-valid config/model inputs. No blanket IR finite validator is introduced.

The legacy-JSON defaults covered deterministically are exactly the actually-defaulted fields:
`PrintEntity.tool_index -> 0`, `ExtrusionPath3D.tool_index -> None`,
`ExtrusionPath3D.order_lock -> None`, `LayerCollectionIR.speed_profiles -> []`,
`LayerCollectionIR.support_entity_identities -> []`. Omission of required `entity_id` and a
wrong ID type are rejected. Omission tests are JSON map compatibility, **not** a claim that
serde defaults recover arbitrary truncated Postcard sequences.

### Domain

`max_entities = 8 / 16`, `max_travels = 16 / 32`, `max_points = 4 / 8`, `max_layers = 4 / 8`
(each dimension also inside the shared `entity_layers` limits: ≤ 128 per dimension and
≤ 100 000 points per product). Layers come from
`slicer_sdk::test_support::property_testing::domains::entity_layers` with the roles
`{OuterWall, InnerWall, SparseInfill, SupportMaterial}`; augmentation choices (path tool
selectors, ordering locks, travel counts) and the integer geometry are generated as part of the
strategy, not sampled inside `prop_map`. Points are dyadic (`i16/8` mm), so the JSON domain is
finite and exactly representable in both directions.

### Strategy

`(entity_layers, per-layer choices, integer geometry)` mapped into an augmented layer vector:
odd selectors set `ExtrusionPath3D.tool_index`, even locks set `order_lock`,
`PrintEntity.tool_index` cycles over `0..3`, travels anchor to real entity ids with dyadic or
absent coordinates, `speed_profiles` rows are derived from each even-index entity's own points
(so a shrink cannot strand a row), and `support_entity_identities` rows cover odd-index
entities. Shrinking removes layers/entities/travels and simplifies values while keeping the
side-table relationships, because they are computed from the entity set rather than sampled
independently.

### Oracle

`observe_layers` (field/bit observation) plus `property_oracles::footprint` for the exact
integer geometry. The property compares `observe_layers(decoded)` against the pre-encode
observation, and additionally compares the decoded value to the source value; the observation
is the independent check, the value comparison is the natural round-trip assertion.

### Tolerance

Exact: integer identity, enum identity, string identity and float **bit** identity. No
tolerance is applied and `null` is never accepted as a lossless value.

### Shrinking

Preserves the finite-dyadic domain and the role/path/side-table relationships described above.
Shrink limits are profile-bounded; completion is reported unknown.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, `cases = 64 / 1024`,
`max_default_size_range = 32 / 256`, `max_shrink_iters = 64 / 256`, rejection budgets `0`.
Measured on Windows; see `docs/testing/proptest-ir-config-evidence.md`.

### Failure record

As for `H-COORD`. The frozen `region_map_versioned_decode_tdd` fixture is **not** regenerated
by this charter, and the current schema constants are unchanged.

### Acceptance

Both codecs preserve the observed slice for every generated finite case, integer geometry stays
exact through both, and the deterministic target observes all five named JSON defaults plus the
absent/mistyped-required-ID and required-null rejections and the representation-only Postcard
exceptional bits. Defect outcome: **no production defect was observed** in the selected slice.

## H-CONFIG-INTERN

Registry record: `id = "H-CONFIG-INTERN"`, `package = "slicer-ir"`, `target_kind = "test"`,
`target_name = "property_ir"`, `test_name = "proptest_config_equality_hash_and_interning"`,
`production_symbol = "slicer_ir::RegionMapIR::intern_config"`,
`production_path = "crates/slicer-ir/src/slice_ir.rs"`,
`features = ["slicer-ir/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-config-intern"`.

### Stable property ID

`H-CONFIG-INTERN` — bitwise config equality/hash and the `RegionMapIR` interner contract.

### Owner

Package `slicer-ir`; test target `property_ir`; test
`proptest_config_equality_hash_and_interning` in `crates/slicer-ir/tests/property_ir.rs`.
Production surface: `ConfigValue::{PartialEq, Eq, Hash}`, `ResolvedConfig::{PartialEq, Eq,
Hash}`, `slicer_ir::resolved_config::ResolvedFloatOrPercent` and
`RegionMapIR::{intern_config, config_for_raw, config_for}`
(`crates/slicer-ir/src/{slice_ir,resolved_config}.rs`). Deterministic controls live in
`ir_config_oracle_counterexamples` and `ir_config_type_and_integer_range_rejections`.

### Contract

`ResolvedConfig` and `ConfigValue` compare and hash **float bits** (`to_bits()`), including
options, sequences and `BTreeMap` extensions, so `Eq` and `Hash` agree by construction —
`signed zero`, distinct NaN payloads and infinities stay distinct. Equality therefore means
"same representation", not "numerically close" and not "semantically equivalent after
resolution". `RegionMapIR::default()` seeds `configs[0]` with `ResolvedConfig::default()`, so
`ConfigId(0)` is always a valid index; `intern_config` deduplicates by equality (linear scan,
first occurrence wins) and `config_for` resolves a `RegionKey` through the plan's
`ConfigId`. `config_for` panics on broken construction invariants; it is not a validating API
and indexing invalid ids is not tested as a "missing validator".

The property asserts exactly two things per case:

```text
actual_eq(a, b) == (representation_key(a) == representation_key(b))
actual_eq(a, b)  =>  hash_with_same_hasher(a) == hash_with_same_hasher(b)
```

Unequal hashes are **never** required for unequal values, and no digest is persisted.

### Domain

Two clearly labelled input modes per generated item:

1. **Finite compatible owner fixtures** built from `ResolvedConfig::default()` with explicit
   positive `layer_height`, `first_layer_height` and `line_width`, plus a varied
   `outer_wall_speed`, `initial_layer_line_width` (percent or absolute),
   `arachne_min_feature_size` (present/absent), `bed_shape` length, `fill_authored_coloring`
   length and nested `extensions` shape. These are compatible structural fixtures, not a
   replacement for the full resolver.
2. **Raw representation-contract fixtures** with signed zeros, infinities and distinct/same NaN
   payloads in `layer_height`/`outer_wall_speed`/`arachne_min_feature_size`. These exercise
   Eq/Hash/serde mechanics only and are never claimed to be valid configs.

`0 < max_abs_config_value ≤ 2^24` (4096 routine / 16384 extended) is checked before narrowing
into the f32-backed fields; positive/cross-field compatible subsets are chosen explicitly
rather than inferred from that magnitude bound. `max_list_len` (4 / 8) bounds the **total**
generated list elements across one config's extension trees (the extension shape is clamped to
`breadth^depth` nodes plus the two sequence lengths), and `max_list_depth` (2 / 3) bounds
nesting, so a per-node branching bound cannot hide exponential allocation.

### Strategy

`ConfigSeed::{Finite, Raw, Repeat, Scalar, Holder}` with structural tags. `Repeat` rebuilds the
previous configuration from its canonical Postcard bytes, so equal pairs come from an
independent construction path rather than `clone == clone`; unequal pairs arise from real
field/value/variant/length changes across the finite and raw arms; `Scalar`/`Holder` push
bounded `ScalarConfigDomain` values through the real `ResolvedConfig::apply_cli_key` seam.
Mandatory deterministic witnesses run before sampling: `-0.0` vs `0.0`, two distinct NaN
payloads, percent vs absolute `FloatOrPercent`, an independently reconstructed equal pair
(equality and hash agree), `ConfigId(0)` for the default seed, rank-1 insertion + idempotence
for a nondefault config, and a real keyed plan reading the nondefault config.

### Oracle

`property_oracles::{representation_key, struct_eq, hash_of, pool_model, with_*}`.
`representation_key` is the canonical Postcard encoding of the whole `ResolvedConfig`, used as
an equality oracle **independent of the handwritten Eq/Hash**. Its premise — that Postcard
preserves scalar float bits and ordered `BTreeMap` content on these structures — is validated
first, on deliberate signed-zero / NaN-payload / option / list / extension counterexamples, by
`ir_config_oracle_counterexamples`. `pool_model` computes first-occurrence ranks over those
keys, seeded with the serialized default. `hash_of` uses `DefaultHasher`, a within-run
observation, never a persisted digest.

### Field-family coverage (reviewed against the live declaration)

The generated axes are a deliberate subset, not an exhaustive roster. Dispositions:

| Field family | Examples | This property | Deterministic coverage |
|---|---|---|---|
| fixed `f64` | `layer_height`, `first_layer_height`, `line_width` | varied (finite and raw-bit) | `resolved_config_defaults_tdd` |
| fixed `f32` | `outer_wall_speed` | varied (finite and raw-bit) | `resolved_config_defaults_tdd` |
| `ResolvedFloatOrPercent` | `initial_layer_line_width` | varied (percent and absolute) | `resolved_config_defaults_tdd` |
| optional `f32` | `arachne_min_feature_size` | varied (present/absent/raw) | `config_map_parity_tests` in `--lib` |
| `Vec<f64>` / `Vec<String>` | `bed_shape`, `fill_authored_coloring` | varied length and content | `config_map_parity_tests` |
| `BTreeMap` extensions | `extensions` | varied depth/breadth/content | `config_map_parity_tests::resolution_extension_reaches_the_config_map_and_participates_in_eq`, `filament_max_volumetric_speed_tests::extension_value_participates_in_equality_and_hash` |
| `u32` / `String` | `wall_count`, `top_fill_holder` | varied through `apply_cli_key` | `resolved_config_defaults_tdd` |
| `bool` and enumerated plain fields | `support_enabled`, `wall_generator`, `infill_type`, `support_type` | **not varied here** | read by the handwritten `PartialEq`; covered structurally, not by this property |
| serialization-only `carrier` field | `flat_bridge_closing_join` | **not varied here** | it exists only to keep the Postcard field order; `flat_bridge_closing_join_is_a_serialization_only_carrier` pins that it is absent from `to_config_map`/`typed_field_keys` and still round-trips |
| remaining `f32`/`u32`/`Option` support, machine-limit and MMU keys | `support_*`, `machine_max_*`, `mmu_*` | **not varied here** | `machine_limit_config_tests`, `config_map_parity_tests` |

A defect confined to a field family listed as "not varied here" would not be caught by this
property; the property's claim is scoped to the varied axes plus the representation contract.

### Tolerance

Exact: bitwise equality, byte equality of the canonical representation, and exact `ConfigId`
equality. No tolerance exists on this surface.

### Shrinking

Shrinks shorten lists, reduce extension depth/breadth, and simplify scalars while staying inside
`max_abs_config_value`, `max_list_len` and `max_list_depth`. Each configuration's input mode is
pinned by `property_oracles::mode_locked`, so a raw-bit fixture cannot shrink into a finite
structural fixture. `Repeat` always rebuilds from its shrunk predecessor, so an equal pair stays
an equal pair. The extension budget counts **total** list elements
(`b + b² + … + b^depth`, `property_oracles::extension_node_count`), not only the innermost
leaves; a depth-2, breadth-2 request against a budget of 4 is clamped and the concrete case is
asserted in `ir_config_oracle_counterexamples`.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, `cases = 64 / 1024`,
`max_default_size_range = 32 / 256`, `max_shrink_iters = 64 / 256`, rejection budgets `0`.
Bounds `max_configs = 4 → 8`, `max_list_len = 4 → 8`, `max_list_depth = 2 → 3`,
`max_text_bytes = 16 → 32`, `max_abs_config_value = 4096 → 16384`. Measured on Windows; see
`docs/testing/proptest-ir-config-evidence.md`.

### Failure record

As for `H-COORD`. Root-cause regression destination for a confirmed defect: a named concrete
config pair in `crates/slicer-ir/tests/region_map_versioned_decode_tdd.rs` or a dedicated named
test — never a seed, and never a regenerated fixture.

### Acceptance

Equality agrees with the canonical representation on every generated pair, equal values hash
equally, interning matches the first-occurrence model, `config_for_raw` returns the interned
input, and a real keyed plan reads its nondefault config. Defect outcome: **no production
defect was observed** in `Eq`/`Hash`/`intern_config`.

## H-CONFIG-BOUNDS

Registry record: `id = "H-CONFIG-BOUNDS"`, `package = "slicer-scheduler"`,
`target_kind = "test"`, `target_name = "property_config"`,
`test_name = "proptest_config_scalar_bounds"`,
`production_symbol = "slicer_scheduler::config_resolution::ConfigBoundsIndex::check"`,
`production_path = "crates/slicer-scheduler/src/config_resolution.rs"`,
`features = ["slicer-scheduler/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-config-bounds"`.

### Stable property ID

`H-CONFIG-BOUNDS` — the public bounds-only config API against an independent
all-declarations predicate.

### Owner

Package `slicer-scheduler`; standalone test target `property_config` (explicit `[[test]]` entry
with `required-features = ["property_testing"]`, selected by the new empty
`slicer-scheduler/property_testing` feature); test `proptest_config_scalar_bounds` in
`crates/slicer-scheduler/tests/property_config.rs`. Production surface:
`ConfigBoundsIndex::{from_declarations, check}` and `BoundsDeclaration`
(`crates/slicer-scheduler/src/config_resolution.rs`), which internally reach the private
`check_value` / `check_scalar`. Deterministic controls live in
`crates/slicer-scheduler/tests/property_config_contracts.rs`; the reference model lives in
`crates/slicer-scheduler/tests/common/property_config_oracles.rs`.

### Contract

`check` merges every declaration for a key by intersection (min = larger, max = smaller) and
then:

- returns `Ok(())` for a key with no bounds, and for non-numeric (`Bool`/`String`) values —
  variant mismatch is `apply_cli_key`'s job, not this API's;
- returns `OutOfRange { key, value, min, max, index }` when a numeric scalar or list element is
  outside `[min, max]` or is NaN/non-finite against a finite bound, reporting the **first**
  offending list element index;
- for `Percent` and `FloatOrPercent { is_percent: true }` enforces finiteness and the **min**
  side only: a declared max is expressed in the key's absolute unit, and a raw percentage's
  absolute equivalent is only known after Phase-B expansion. Absolute `FloatOrPercent` uses the
  full bounds.

This is explicitly a **bounds-only** API: a completely missing key or a non-numeric variant
passes it. A present declaration with neither min nor max still reaches the finite check, so a
NaN/infinity is rejected there. No whole-config or cross-field validity is inferred. The
private `check_scalar`/`check_value` visibility is unchanged.

### Domain

`max_declarations = 4 / 8` (strategy maximum 64), `max_list_len = 4 / 8` (maximum 64),
`max_abs_bound = 1000 / 100 000` with a construction ceiling of
`MAX_ABS_BOUND = 2^52` — strictly inside the exactly-representable f64 integer domain
(`2^53 - 1`), so `Int` magnitudes and integer-grid `Float` values narrow exactly. Declarations
are drawn over a fixed key alphabet with each shape reachable: one-sided min, one-sided max,
two-sided, `None`/`None` (present but unbounded), intersected duplicates, and contradictory
(`min > max`). Values are `Int`/`Float` on the integer grid, arbitrary finite `f64` including
both zero signs and subnormal witnesses, the `next_down`/`next_up` neighbours of every drawn
centre, `Percent`, both `FloatOrPercent` modes, `Bool`, `String`, and flat `List`s of length
`0..=max_list_len`. Nonfinite values are a separate labelled family.

### Strategy

A tuple of independently generated pieces, joined by `prop_map` (never `prop_flat_map`, so a
declaration shrink cannot re-draw the value):

- `declaration_sets(max_declarations, max_abs_bound)`, then
  `property_config_oracles::ensure_bounded_witness` appends a `[-4, 4]` declaration on
  `WITNESS_KEY` — a key outside `KEY_ALPHABET`, so no generated declaration can narrow it;
- a `ProbeMode` chosen by `proptest::sample::select(PROBE_MODES).no_shrink()`;
- `bound_scalar_values` and `bound_list_values` for the two general families;
- a key pick that selects a generated key or `ABSENT_KEY`.

`derive_probe` then computes the value from the current declaration set and the pinned mode:
edges, the midpoint, one-step violations, flat lists whose only invalid element sits at index
0/1/2, and non-finite scalars/lists. Mandatory deterministic witnesses run before sampling:
absent key → `Ok`;
present-but-unbounded declaration → finite passes, NaN/inf rejected; one-sided min and max;
inclusive edges accepted while `next_down`/`next_up` neighbours are rejected; contradictory
intersections reject every finite value; empty list → `Ok`; lists failing at index 0 and at
index 2; `Bool`/`String` → `Ok`; percent above max but above min → `Ok` while below min →
`OutOfRange`; absolute `FloatOrPercent` fully bounded; non-finite absolute scalars rejected with
the declaration's min/max echoed. A bounded pre-sample additionally proves the generated
population contains accepted cases, rejected cases, at least one list-index case and at least
one non-finite case — a success-only population fails the run.

### Oracle

`expected_accepts`, `expected_first_invalid_index` and `expected_effective` in the oracle module:
an independent all-declarations predicate (every present min and every present max of **every**
matching declaration), with the first-invalid-index reported separately. It never calls
`NumericBounds::intersect` and never uses `check` to certify the generator.

### Tolerance

Exact. `OutOfRange.value` is compared bitwise (`to_bits()`), so `-0.0`/`0.0` and NaN payloads
are distinguished; `min`/`max` are echoed from the independently computed effective bounds.

### Shrinking

Shrinking preserves the probe: **shape, verdict class and first-invalid index** all survive.
The strategy selects a probe mode from a `no_shrink` list and then *derives* the value from the
current declaration set inside `prop_map` — there is no `prop_flat_map` re-drawing it, so when
the declarations shrink the derivation re-runs and produces a value of the same family. The
witness families are derived from a bounded witness declaration on a key outside the generated
alphabet (`property_config_oracles::WITNESS_KEY`), so no other declaration can perturb the
interval:

- `EdgeMin`/`EdgeMax` land exactly on the inclusive bounds (accepted);
- `InsideScalar`/`InsideList` use the interval midpoint (accepted);
- `ViolationScalarLow`/`ViolationScalarHigh` sit one step outside (rejected, `index: None`);
- `ViolationListAt0/1/2` place the only out-of-range element at that position, so `index` is
  `Some(0)`/`Some(1)`/`Some(2)`;
- `NonFiniteScalar`/`NonFiniteList` stay non-finite (rejected).

The two `General*` families are independent draws, so only their **shape** is pinned; their
verdict is whatever the oracle derives for the current declarations. `Absent*` keeps the key
unnamed, which the bounds-only API accepts by contract.

This is checked two ways: `derived_probe_modes_have_their_documented_verdicts` asserts every
mode's verdict and first-invalid index on a concrete declaration set, and the property runs
`assert_mode_preserved` over the **composed** strategy on every execution, tagging each case with
its shape plus — for witness keys — its verdict class and first-invalid index, so a shrink that
changed any of them fails the run.

Shrink limits are profile-bounded (`max_shrink_iters`, `max_shrink_time_ms = 0`).

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, `cases = 64 / 1024`,
`max_default_size_range = 32 / 256`, `max_shrink_iters = 64 / 256`, rejection budgets `0`.
The package gained a default-off empty `property_testing` selection feature and dev-only
`proptest` / `slicer-sdk` dependencies; no normal dependency, engine edge or default feature
changed. Measured on Windows; see `docs/testing/proptest-ir-config-evidence.md`.

### Failure record

As for `H-COORD`; the reproduction command is
`cargo xtask test --summary --properties --property H-CONFIG-BOUNDS --tier <tier> --seed <seed>`.
Root-cause regression destination: a named concrete case in
`crates/slicer-scheduler/tests/integration/` (`config_bounds_enforcement_tdd`) — never a seed.

### Acceptance

The real public API agrees with the independent predicate on every generated declaration set
and value, including inclusive edges, empty intersections, optional bounds, first invalid list
position, finite/nonfinite and percent/absolute distinctions. The deterministic controls refute
the accept-all, union-instead-of-intersection, off-by-one-bound and lost-index mutants, and
`ir_config_type_and_integer_range_rejections` observes the specified `apply_cli_key`
type/range variants and fields. Existing `config_bounds_enforcement_tdd` and
`config_resolution_tdd` pass. Defect outcome: **no production defect was observed**.

## H-CHAINS

Registry record: `id = "H-CHAINS"`, `package = "slicer-ir"`, `target_kind = "test"`,
`target_name = "property_ir"`, `test_name = "proptest_canonical_chain_product"`,
`production_symbol = "slicer_ir::region_split_registry::enumerate_canonical_chains"`,
`production_path = "crates/slicer-ir/src/region_split_registry.rs"`,
`features = ["slicer-ir/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-chains"`.

### Stable property ID

`H-CHAINS` — the complete ordered Cartesian contract of the variant-chain enumerator.

### Owner

Package `slicer-ir`; test target `property_ir`; test `proptest_canonical_chain_product` in
`crates/slicer-ir/tests/property_ir.rs`. Production surface:
`enumerate_canonical_chains(&HashMap<String, Vec<PaintValue>>, &[String]) -> Vec<Vec<(String,
PaintValue)>>` (`crates/slicer-ir/src/region_split_registry.rs`). Deterministic controls live in
`ir_chain_oracle_order_and_empty_controls`.

### Contract

Per `docs/03_wit_and_manifest.md` § Variant-chain enumeration order, the helper emits every
subset of `(semantic, value)` pairs: for each axis in `canonical_order`, either omit it or
include exactly one of its supplied values, in the caller's `Vec` order. Cardinality is
`∏ (1 + K_i)`, an absent or empty axis contributes a factor of 1, the empty chain is the first
element, and each chain's pairs follow canonical axis order. The helper performs **no** sorting,
no deduplication and no duplicate-axis validation; the caller owns the priority/name and
per-value sorting contract. Scalar region-split declarations and duplicate canonical axes are
outside the valid caller domain.

### Domain

`max_semantics = 3 / 4`, `max_values_per_semantic = 3 / 4`, `max_chains = 64 / 625`,
`max_text_bytes = 16 / 32`. With those maxima the largest reachable product is exactly
`(1 + 3)^3 = 64` routine and `(1 + 4)^4 = 625` extended, so a well-formed case can always be
compared in full. Axis names are unique and bounded; each axis is drawn as absent, empty, or a
bounded value list; the value alphabet is `PaintValue::{Flag, ToolIndex, Custom}`; at least one
map key outside `canonical_order` is always present so "ignore extra keys" is exercised on
every case.

### Strategy

A `(HashMap<String, Vec<PaintValue>>, Vec<String>)` pair built from tagged axis specs, so
shrinking removes axes or values without changing the canonical order's meaning. Value order is
deliberately unsorted (the alphabet interleaves `Flag`/`ToolIndex`/`Custom`). Mandatory
deterministic witnesses run before sampling: empty map with empty order → exactly `[[]]`; one
axis with three unsorted values → empty chain first, then caller order verbatim; absent and
empty axes plus an unrelated key → `[[]]`; duplicate values → multiplicity preserved (three
chains, two of length one); a 64-axis request → `chain_product_size` refuses before allocating.

### Oracle

`property_oracles::{chain_product_size, chain_product_model}`. The model decodes each ordinal
`0..product` into mixed-radix digits with the **last** axis changing fastest (least-significant
digit), digit `0` omitting the axis and digit `d > 0` selecting `values[d - 1]`, then emits the
selected pairs in `canonical_order`. It is an ordinal decode loop, not a copy of the production
recursive omit-first traversal.

### Tolerance

Exact `Vec` equality of the whole ordered sequence, plus exact cardinality. Unordered sets are
never used, because they would erase multiplicity and ordering — the two properties the helper
is expected to preserve.

### Shrinking

Shrinks remove trailing axes and values and simplify text, staying within
`max_semantics`/`max_values_per_semantic` and keeping the product inside `max_chains`. The
nonempty/two-axis witnesses needed for the mode-specific assertions are supplied by the
deterministic block, so a shrink never needs to preserve them probabilistically.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, `cases = 64 / 1024`,
`max_default_size_range = 32 / 256`, `max_shrink_iters = 64 / 256`, rejection budgets `0`.
Measured on Windows; see `docs/testing/proptest-ir-config-evidence.md`.

### Failure record

As for `H-COORD`. Root-cause regression destination: a named concrete case in
`crates/slicer-ir/tests/region_split_registry_tdd.rs` — never a seed.

### Acceptance

The full ordered output, cardinality and per-chain canonical axis order agree with the
mixed-radix model on every case; the empty chain is first; caller value order and duplicate
multiplicities survive; extra map keys and map insertion order do not affect the result.
`ir_chain_oracle_order_and_empty_controls` refutes the omitted-empty-chain, sorted-values,
swapped-axis, dropped-product and duplicated-product mutants, and the existing
`region_split_registry_tdd` still passes. Defect outcome: **no production defect was observed**.

## H-DAG

Registry record: `id = "H-DAG"`, `package = "slicer-scheduler"`, `target_kind = "test"`,
`target_name = "property_scheduler"`, `test_name = "proptest_scheduler_dag_models"`,
`production_symbol = "slicer_scheduler::topological_sort"`,
`production_path = "crates/slicer-scheduler/src/topology.rs"`,
`features = ["slicer-scheduler/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-dag"`.

### Stable property ID

`H-DAG` — exhaustive lexical-minimum linear extensions for acyclic scheduler graphs and the
complete blocked remainder for cyclic ones.

### Owner

Package `slicer-scheduler`; standalone test target `property_scheduler` (explicit `[[test]]` entry
with `required-features = ["property_testing"]`, selected by the empty
`slicer-scheduler/property_testing` feature); test `proptest_scheduler_dag_models` in
`crates/slicer-scheduler/tests/property_scheduler.rs`, which mounts the target-private model module
`crates/slicer-scheduler/tests/common/property_scheduler_dag.rs`. Production surface:
`slicer_scheduler::topological_sort` (`crates/slicer-scheduler/src/topology.rs`),
`slicer_scheduler::build_intra_stage_dag` (`crates/slicer-scheduler/src/dag.rs`) and
`slicer_scheduler::validation::validate_startup_dag` (`crates/slicer-scheduler/src/validation.rs`)
for the cyclic validator surface. Deterministic controls live in
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/dag_controls.rs` and
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/plan_controls.rs`.

### Contract

For a valid acyclic graph, `topological_sort` returns exactly the declared nodes (a bijection) and
the lexicographically minimum module-id sequence satisfying every edge — the finite
linear-extension definition, not a ready-queue traversal. Its `Err` value is the entire blocked
remainder: every node on a cycle plus every node reachable from a cycle by a nonempty path,
including an acyclic tail, while independent sortable nodes stay out; the returned `Vec` is
lexically sorted. `validate_startup_dag`'s private sorter promises no order, so its
`CycleDetection`/`CyclicDependency.cycle` is compared as a set against the same path model.

**Malformed-model boundary.** A malformed raw graph — a `SmallGraph` with duplicate node entries or
an edge naming an unknown endpoint — is rejected by the target-private checked adapter `checked_nodes`
before any production sorter call, and `sort_checked` tags that rejection. There is no promised scheduler
raw-graph validation API, so this is generator/adapter negative coverage and not SUT-validator
credit; actual scheduling negatives are cycles, missing dependencies and invalid plan bindings at
their real validating APIs.

### Domain

`max_nodes` must lie inside the packet's `3..=7` safety ceiling (routine `5`, extended `7`).
Within it the shared `graphs(GraphKind::Dag, max_nodes)` domain draws `0..=max_nodes` nodes, so
empty and singleton DAGs are legitimate cases — empty input has the one empty linear extension —
while `GraphKind::CycleWithTail` requires at least three nodes and constructs a real directed cycle
plus a downstream acyclic tail. Node values map to fixed-width lexical module ids `com.pbt.NNNNN`
through `module_id_for`, independently of the input `Vec` order, so lexical and numeric order agree.
Edges are unique index pairs; the checked adapter gives each one a genuine
`IrWriteRead { writer_path }` declaration, and a separate robustness mode duplicates a destination
with a second reason. The exhaustive search is capped at
`MAX_LINEAR_EXTENSION_NODES = 7` (`7! = 5040` leaves); that ceiling must not be raised to absorb
extended-case cost.

### Strategy

`sample_strategy` builds a `Vec<SampleMode>` by repeating `DAG_MODE_WEIGHT` (9) `Dag` tags and
`CYCLE_MODE_WEIGHT` (4) `Cycle` tags, then draws one mode from
`proptest::sample::select(modes).no_shrink()` and dispatches it through `prop_flat_map`; the 9:4
weighting comes from list membership, not from a union. A DAG branch pairs the graph with a
`no_shrink` duplicate-destination flag, then draws a node-index permutation through
`permutations(...).prop_map`; `verify_dag_case` applies that permutation to the node `Vec` and to
each `edges_to` list and requires the public order to be unchanged. Cycle branches stay in
`GraphKind::CycleWithTail`. Mandatory deterministic witnesses run before sampling: a self-cycle
with a downstream tail (`100 -> 100`, `100 -> 101`, `101 -> 102`) and two disconnected cycles
(`200 <-> 201`, `202 <-> 203`, `201 -> 204`) with independent sortable node 205; both must match
the path model on the public sorter and on `CycleDetection`. The 128-case pre-sample population
guard requires empty, singleton, disconnected multi-node, multi-edge, duplicate-destination and
cycle-with-tail samples and a nonempty cycle remainder; there is no `prop_assume` and no rejection
budget.

### Oracle

`minimum_linear_extension` enumerates every node-index permutation, filters by edge precedence and
keeps the lexicographically smallest id sequence; `check_success_oracle` checks bijection first,
then exact order, then precedence. `check_cycle_remainder` compares the public blocked `Vec` to
`slicer_sdk::test_support::property_testing::oracles::graph_cycle_remainder` and additionally
requires lexical order because only the public `Vec` promises it. `check_validator_cycles` compares
`CycleDetection`'s cycle set to the same model, with four fixed global fill holders making a
fill-coverage `MissingDependency` impossible in the cycle-only request. `check_intra_stage_relation`
derives the expected node, destination, read, write and reason table from the raw graph and asserts
that the real `build_intra_stage_dag` reproduces it, including the exact
`IrWriteRead { writer_path }` reason per edge. The oracle never calls `topological_sort`, the
private validator sorter, or a Kahn loop to certify itself.

### Tolerance

Exact string, `Vec` and set comparison throughout. No numeric tolerance exists on this surface.

### Shrinking

The case mode is drawn from a **non-shrinkable** weighted list
(`proptest::sample::select(modes).no_shrink()`), not from `prop_oneof!`: a union shrinker would try
an earlier branch and could turn a `Cycle` case into a `Dag` case. Cycle-plus-tail cases are
rebuilt from `(cycle_len, tail_len)` inside the domain's `prop_map`, so a shrink keeps a real
directed cycle and at least one downstream tail node; the duplicate-destination flag is also drawn
from a `no_shrink` list. DAG cases keep the forward-edge construction and the linear-extension
oracle is recomputed for the shrunk graph; the node permutation is re-drawn by `prop_flat_map` and
is re-validated as a complete permutation. `assert_shrink_preserves_modes` audits this on every
execution: fixed-seed trees are simplified repeatedly and must keep their mode tag, a simplified
cycle must still have a nonempty blocked remainder, a simplified DAG must stay acyclic and still
match the exhaustive oracle, and the duplicate-destination *flag* must survive. That last assertion
is flag preservation, not an exercised duplication check: the audit's shrunk-DAG body never builds
a duplicate edge, and `verify_dag_case` calls `check_duplicate_destination` only when the flag is
set and the shrunk graph still has at least one edge, so a shrink that empties the edge list keeps
the tag while no duplication assertion runs. Shrink limits are
profile-bounded (`max_shrink_iters` 64/256, `max_shrink_time_ms = 0`); completion is reported
`null`/unknown because Proptest 1.11 cannot identify it.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, `cases = 64 / 1024`,
`max_default_size_range = 32 / 256`, `max_shrink_iters = 64 / 256`, rejection budgets `0`,
`bounds.max_nodes = 5 / 7`. Measured on Windows: routine libtest `0.20s` and extended `1.69s`, each
at its profile case count (64 / 1024), with `rejected_test_calls = 0`,
`call_counts_complete = true` and outcome `passed`; the retained routine seed
`8015297453583596151` replayed to `VERDICT: PASS`. Command, logs and provenance are in
[proptest-scheduler-evidence.md](proptest-scheduler-evidence.md).

### Failure record

`failure.json` is produced by the foundation harness (packet `proptest-openrscad_01_pbt-foundation`)
and retains the classification, the full context, the concrete/debug input, the tier-bearing
reproduction command
(`cargo xtask test --summary --properties --property H-DAG --tier <routine|extended> --seed <seed>`),
an executable direct reproduction carrying the exact consumed `PNP_PROPERTY_CONTEXT`, captured SDK
logs and shrink status. Committed regression history is a named concrete input — never a seed. The
control homes are
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/dag_controls.rs`,
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/plan_controls.rs` and
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/bound_controls.rs`.

### Acceptance

The target was actually executed through the registry with `--exact --nocapture`, never
compile-only and never as an unrelated passing test; the exhaustive or path-model oracle is checked
on every case. Generator and shrink validity are demonstrated by `scheduler_dag_oracle_controls`
(rejecting FIFO-instead-of-minimum, missing-node, duplicated-node and cycle-only-tail-omission
mutations), by `scheduler_dag_raw_model_rejections_before_sorter`, and by the pre-sample population
guard. `scheduler_graph_construction_and_plan_observations` observes real plan assembly, compiled
module sequence, claims, access masks and host paint-stage insertion with independently derived
expectations. Existing scheduler baselines `topological_sort_tdd`, `dag_construction_tdd` and
`execution_plan_tdd` pass. Defect outcome: **no production defect in `topological_sort`,
`build_intra_stage_dag` or the cycle pass was observed**; no regression was manufactured.

## H-CLAIMS

Registry record: `id = "H-CLAIMS"`, `package = "slicer-scheduler"`, `target_kind = "test"`,
`target_name = "property_scheduler"`, `test_name = "proptest_scheduler_claim_models"`,
`production_symbol = "slicer_scheduler::validation::validate_startup_dag"`,
`production_path = "crates/slicer-scheduler/src/validation.rs"`,
`features = ["slicer-scheduler/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-claims"`.

### Stable property ID

`H-CLAIMS` — typed claim-scope, write-ordering and effective-universe dependency contract of the
startup validator.

### Owner

Package `slicer-scheduler`; standalone test target `property_scheduler`; test
`proptest_scheduler_claim_models` in `crates/slicer-scheduler/tests/property_scheduler.rs`, which
mounts `crates/slicer-scheduler/tests/common/property_scheduler_claims.rs`. Production surface:
`slicer_scheduler::validation::validate_startup_dag`
(`crates/slicer-scheduler/src/validation.rs`). Deterministic controls live in
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs` and
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/selection_controls.rs`; the
object-wide cross-region regression lives in
`crates/slicer-scheduler/tests/contract/claim_transition_matrix_tdd.rs`.

### Contract

`validate_startup_dag` is driven as the public validator over an effective universe. Claim scopes
are typed values `(object_id, region_id, Option<layer>)`; the model never concatenates them, so two
distinct tuples cannot alias. For `ConflictScope::Global`, ordinary holders conflict pairwise and
duplicate observations of one module collapse; `claim:raft-fill` is ordinary here, not a configured
fill exception. Exactly the four `FILL_CLAIM_IDS` — `claim:top-fill`, `claim:bottom-fill`,
`claim:bridge-fill`, `claim:sparse-fill` — allow global multiplicity, and a global absence produces
`MissingDependency { module: "fill-role-claim:<claim>", requires: "no module holds <claim>" }`; a
per-region duplicate holder still conflicts. Exactly the four family-support globals
`support-generator`, `support-planner`, `support-family:traditional` and `support-family:tree` are
exempt from the global conflict check; an arbitrary `support-family:<suffix>` is not.

The four stable claims `perimeter-generator`, `seam-placer`, `layer-planner` and `mesh-analyzer`
must keep one holder per object across every region and layer; the reported `ClaimConflict` carries
the lexically first holder's original region with `global_layer_index = None`.
`claim:sparse-fill` and `support-generator` are the transition-capable claims in this domain, so
their cross-layer transitions stay allowed. Per-region observations of other claims compare the
full typed tuple, layer included.

Write conflicts are decided by exact shared fields: a pair is orderable only when the downstream
writer declares the shared field in its `ir_reads` and a real `StageDag` path runs from the
upstream writer to it. Family-support writers are orderable only when both hold one of the four
family-support globals and the shared path is exactly `SupportPlanIR`/`SupportIR` or a
dot-descendant; `SupportIRx` and unrelated fields are not exempt. `WriteConflict` comparisons
normalize the unordered module pair lexically.

Module and capability dependencies are modelled by exact effective membership, and direct and
transitive stage legality by `STAGE_ORDER` rank and real finite paths. Each
`TransitiveStageDependency.path` must start at the requesting module, be simple, follow declared
`requires_modules` edges and end at a later stage; expected existence comes from the model, not a
particular BFS route.

**Effective-universe semantics.** The production request has no disable flag, so the case builder
removes locally tagged disabled modules together with their holder observations and stage-DAG nodes
before constructing the request. A retained consumer still requiring the removed module or
capability must reject; removing a competing holder must eliminate only its conflict. This tests
effective-input behaviour, not config parsing or a nonexistent scheduler disable mechanism.

### Domain

`checked_claim_limits` enforces `max_modules 3..=7`, `max_claims 1..=8`,
`max_observations 2..=16` (variable observations in addition to the fixed four-role fill setup),
`max_layers 2..=4` and `max_text_bytes 36..=64`. The text ceiling is not widened: `check_text_bound`
rejects any value outside `36..=64` with a named error, because object ids are UUID syntax and need
36 bytes, so a lower ceiling cannot be honoured without silently changing the declared profile (the
packet's 8-byte variable-text floor applies to raw text components, not the UUID object ids).
Committed values are `max_modules / max_claims / max_observations / max_layers / max_text_bytes`:
routine `4 / 4 / 8 / 3 / 36`, extended `7 / 8 / 16 / 4 / 64`; `uuid_text` never shortens a UUID.
Object ids use the shared UUID-syntax domain plus a separate delimiter-rich raw text domain; module
ids use a bounded prefix with `com.pbt.claim-N-` overhead. Modules carry
compatible versions, canonical stages (`Layer::Infill`, `PrePass::MeshAnalysis`,
`PrePass::LayerPlanning`) and placeholder WASM paths. Every unrelated case carries the fixed
global four-claim fill setup, so an unrelated fill-coverage error can never make a rejection look
correct; only `FillGlobalAbsent` deliberately drops one.

### Strategy

`claim_case_strategy` draws one mode from `available_modes`, which is `ClaimMode::ALL` filtered
only by `ClaimMode::is_available`; `StableThreeHolders` is withheld when the accepted
`max_observations` bound cannot carry its three holder observations on top of the fixed fill setup,
and every other mode fits every accepted profile. `ClaimMode::ALL` is the source of truth for the
family inventory: ordinary and raft globals; fill and finite family-support exceptions (global
multiplicity, global absence, per-region duplicate, non-exempt `support-family:<suffix>`); typed
region and delimiter scopes; write conflicts; missing universe; disabled universe; direct and
shared-stage transitive cross-stage legality; stable same-region, cross-region, three-holder,
same-module-across-layers and distinct-objects shapes; the transition-capable per-layer shape and
its same-scope counterpart; and the `SelectionPairing` selector ladder. Each case is exactly one
witness shape. The mode source is a `proptest::sample::select(..).no_shrink()` list, so no union
shrinker can switch modes. The 512-case pre-sample audit requires every available tagged mode and
both rejecting and accepting verdicts, and audits up to 32 `simplify()` calls per sample to prove
the mode and the verdict signature survive shrinking. No `prop_assume` and no rejection budget is
used.

### Oracle

`expected_diagnostics` recomputes a normalized `BTreeSet<DiagKey>` from the case's enabled modules,
holders and stage DAGs. It models only the five named passes (`GlobalClaimConflicts`,
`PerRegionClaimConflicts`, `MissingDependencies`, `WriteConflicts`,
`CrossStageDependencyLegality`) and compares the validator's exact error fields, not
`report.is_valid()`. `ScopeModel` is a typed enum, never a serialized key. `assert_fixed_fill_setup`
proves the four fill obligations are present. `assert_delimiter_witness` proves two distinct typed
tuples collide under the previous concatenated string key, so the test falsifies the
delimiter-aliasing shape rather than merely omitting it. `assert_transitive` validates the real
path shape. The model never calls `compute_reachability` or any private validator pass.

### Tolerance

Exact normalized-set equality of diagnostics, exact string identity for claims/modules/scopes and
exact enum/stage fields; no numeric tolerance exists on this surface.

### Shrinking

Every case carries its `ClaimMode` tag, selected from a **non-shrinkable** list
(`proptest::sample::select(available_modes).no_shrink()`) so a union shrinker cannot switch modes.
`assert_shrink_preserves_witnesses` simplifies fixed-seed trees on every execution and asserts the
tag and that mode's distinguishing witness survive: two distinct typed identities for the
claim/region modes, three holders in `r0`/`r1`/`r2` for the object-wide mode, the legacy
delimiter-colliding pair for the delimiter mode, the conflicting pair for the write mode, the
same-scope `support-generator` holder pair for the transition same-scope mode, the retained
consumer's missing module for the disabled mode, and the selector witness (typed
`wall_generator` value plus `spiral_vase` flag) for the selection/pairing mode. Only the module
declaration order rotates as a shrinkable dimension there, so the expected kept set cannot change
identity. The fixed fill setup is re-asserted on every case, so a shrink cannot make an unrelated
fill error pass for the wrong reason. Shrink limits are profile-bounded; completion is reported
`null`/unknown.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, `cases = 64 / 1024`,
`max_default_size_range = 32 / 256`, `max_shrink_iters = 64 / 256`, rejection budgets `0`.
Measured on Windows: routine libtest `2.27s` and extended `1.92s`, each at its profile case count,
with `rejected_test_calls = 0`, `call_counts_complete = true` and outcome `passed`; the retained
routine seed `5053888522409382591` replayed to `VERDICT: PASS`. See
[proptest-scheduler-evidence.md](proptest-scheduler-evidence.md).

### Failure record

`failure.json` is produced by the foundation harness with the tier-bearing reproduction command
(`cargo xtask test --summary --properties --property H-CLAIMS --tier <routine|extended> --seed <seed>`),
the full context, concrete/debug input, direct reproduction, captured logs and shrink status.
Committed regressions are named concrete inputs, never seeds. Homes:
`crates/slicer-scheduler/tests/contract/claim_transition_matrix_tdd.rs` for the object-wide
cross-region stable-claim regression and
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs` for the
`delimiter_rich_object_and_region_ids_never_share_a_claim_scope` witness, with
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/selection_controls.rs` and
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/bound_controls.rs` for the
remaining selection and bound controls.

### Acceptance

The target was actually executed through the registry with `--exact --nocapture`; the typed
diagnostic model is checked on every case. Generator and shrink validity are demonstrated by
`scheduler_claim_scope_and_exception_controls` (including the same-scope `support-generator`
rejection that `ClaimMode::TransitionSameScope` also generates),
`delimiter_rich_object_and_region_ids_never_share_a_claim_scope`,
`scheduler_write_dependency_and_disabled_controls`, `scheduler_claim_model_ceiling_walk_controls`,
`scheduler_claim_selection_and_pairing_controls`, and the 512-case mode/verdict population audit.
Selection/pairing is generative as well as deterministic: `ClaimMode::SelectionPairing` drives the
real `dedup_same_claim_modules_with_typed_wall_generator`, `resolve_held_claims` (with
`FillHolders`) and `validate_support_family_pairing` seams against an independent selector-table
model, while `scheduler_claim_selection_and_pairing_controls` remains the hand-computed companion.
Existing baselines `claim_transition_matrix_tdd`, `holder_matching_tdd`,
`typed_selector_claim_selection_tdd`, `support_family_selection` and
`support_family_pairing_rejected` pass. Defect outcome: **two production defects were confirmed
and fixed** in the validator's claim pass — (1) the scope-keying defect where delimiter-bearing
object/region ids were serialized into `scope_key_without_layer`, so distinct typed scopes lost
their identity, and (2) missing object-wide cross-region stable-claim enforcement, where per-region
grouping never compared two regions of one object. Both are pinned by named concrete regressions:
`delimiter_rich_object_and_region_ids_never_share_a_claim_scope` and
`non_transitionable_claim_rejects_holder_change_across_regions_of_one_object`; no seed is
committed.
No other defect in the covered passes was observed, and live per-region support aggregation is not
claimed from synthetic observations.

## H-REGISTRY

Registry record: `id = "H-REGISTRY"`, `package = "slicer-scheduler"`, `target_kind = "test"`,
`target_name = "property_scheduler"`, `test_name = "proptest_scheduler_region_split_registry"`,
`production_symbol = "slicer_scheduler::region_split::aggregate_region_splits"`,
`production_path = "crates/slicer-scheduler/src/region_split.rs"`,
`features = ["slicer-scheduler/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-registry"`.

### Stable property ID

`H-REGISTRY` — declaration-only region-split aggregation, the separate canonical ordering helper,
and the explicit first-seen/tied-priority warning modes.

### Owner

Package `slicer-scheduler`; standalone test target `property_scheduler`; test
`proptest_scheduler_region_split_registry` in `crates/slicer-scheduler/tests/property_scheduler.rs`,
which mounts `crates/slicer-scheduler/tests/common/property_scheduler_registry.rs`. Production
surface: `slicer_scheduler::region_split::aggregate_region_splits` and
`slicer_scheduler::region_split::canonical_variant_chain_order`
(`crates/slicer-scheduler/src/region_split.rs`), plus the real plan-assembly caller
`slicer_scheduler::execution_plan::build_execution_plan`
(`crates/slicer-scheduler/src/execution_plan.rs`). Deterministic controls live in
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/registry_controls.rs` and
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/manifest_controls.rs`.

### Contract

`aggregate_region_splits` is declaration-only. For a semantic repeated compatibly — every
declaration agrees on `(priority, value_type)` — it produces one entry
`(priority, value_type, sorted distinct declaring_modules)`, and no `material`/`fuzzy_skin` seed
exists unless a module declares it. The `BTreeMap` keys iterate in semantic-name order; the
separate `canonical_variant_chain_order` returns `(priority, name)` order, so raw map keys are not
that sequence in general. Repeating the same loaded module at the aggregation seam cannot duplicate
`declaring_modules`.

For incompatible repeats the contract is deliberately order-sensitive: a linear positional scan
keeps the first-seen declaration and appends exactly one `Warning` for every later incompatible
declaration, carrying that module's WASM path, `field = Some("region_split")`, the semantic, the
later `priority=<n>/value_type=<T>` and the earlier contract; the later contract never joins
`declaring_modules`. No permutation-invariance claim applies to those inputs. Two distinct
semantics tied at one priority produce exactly one nonfatal warning per tied priority group, with
the empty `PathBuf` sentinel, `field = Some("region_split")` and a message naming the shared
priority, each tied semantic with its sorted declaring module ids, and the lexical tiebreaker
order. Aggregation appends to the caller's diagnostics and never clears or reorders them.

`build_execution_plan` actually calls the aggregator and stores its output in
`ExecutionPlan.aggregated_region_split`; plan assembly is checked separately from the direct helper
call. `canonical_variant_chain_order` has no production caller in this tree, so a passing helper
assertion is **not** evidence that `RegionMapping` uses the helper. Driver-level integration
remains with the H-REGIONMAP audit and the wording of ADR-0018's amendment.

### Domain

`checked_registry_limits` enforces `max_modules 2..=7`, `max_declarations 2..=16` and
`max_text_bytes 8..=64`, with `declaration_capacity = max_modules * MAX_SEMANTIC_POOL`. Committed
values are `max_modules / max_declarations / max_text_bytes`: routine `4 / 8 / 36`, extended
`7 / 16 / 64`. The finite semantic pool is four generated community
names plus `material` and `fuzzy_skin` only when a case requests them; core priorities are 100 and
200, and community priorities sit at or above `COMMUNITY_PRIORITY_FLOOR` (1000). Value types are
`RegionSplitValueType::{Flag, ToolIndex, CustomString}`. Modules are builder-created with unique
ids and WASM paths, deliberately bypassing TOML ingestion: this is the positive aggregation
domain, not manifest-validation coverage (`scheduler_manifest_region_split_rejections` owns raw
ingestion).

### Strategy

`sample_strategy` derives a guaranteed shape — `Compatible`, `Conflicting`, `TiedSameModule` or
`TiedCrossModule` — plus a module permutation, per-module declaration permutations, a module index
to repeat at the seam, and an optional plan-assembly mode. `build_case` re-establishes the requested
shape from the shrunk parameters and never assumes a shape they no longer support. The 128-case
pre-sample guard requires all four shapes, at least one same-module tie, one cross-module tie, a
nonempty aggregate and a tied-priority warning; there is no `prop_assume` and no rejection budget.

### Oracle

`expected_model` derives the relational group model and the first-seen conflict list in linear scan
order; `raw_keys` is semantic-name order and `helper_order` is the separate `(priority, name)`
sort. `tie_warning_messages` derives the exact tied-priority messages.
`check_registry_observation` compares the whole aggregate map, raw key order, the direct
`canonical_variant_chain_order` output, the preserved sentinel, every conflict warning payload and
the full tied-priority `LoadDiagnostic` including the empty `PathBuf`. `assert_normative_catalog`
cross-checks the priority values against `CORE_REGION_SPLIT_PRIORITIES` and
`COMMUNITY_PRIORITY_FLOOR`. The plan mode builds a real plan from unique bindings through
`bind_module_config_view` and fails when `aggregated_region_split` is empty, so plan assembly
stopping its aggregation is observable; it compares that output to the independent model. The
model is relational and first-seen scanning, not the aggregator's own loop.

### Tolerance

Exact map, sequence and message equality, including the empty `PathBuf` sentinel and the
`Some("region_split")` field. No tolerance exists on this surface.

### Shrinking

The requested shape and every flag that decides whether a fault exists (tie membership, conflicting
contract choice, core-semantic inclusion, pool slot) come from a **non-shrinkable** plan selected by
`proptest::sample::select(...).no_shrink()`. A shrink therefore simplifies only module slots,
declaration slots, semantic names and permutation orders, and the check compares against the model
for the current case. `assert_shrink_preserves_faults` simplifies fixed-seed trees on every
execution and asserts the shape never changes, a conflicting case keeps the same first-seen
`(priority, value_type)` with exactly one incompatible warning, a tied case keeps exactly one tied
priority group and warning, and a compatible case acquires no conflicting warning. Compatible cases
keep equality under permutation because the model is relational; deliberately incompatible cases
are compared to their own recomputed first-seen model after each permutation rather than to the
original order. The seam-repeat index is checked against the shrunk module count. Shrink limits are
profile-bounded; completion is reported `null`/unknown.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, `cases = 64 / 1024`,
`max_default_size_range = 32 / 256`, `max_shrink_iters = 64 / 256`, rejection budgets `0`.
Measured on Windows: routine libtest `0.13s` and extended `0.83s`, each at its profile case count,
with `rejected_test_calls = 0`, `call_counts_complete = true` and outcome `passed`; the retained
routine seed `2960010196684534321` replayed to `VERDICT: PASS`. See
[proptest-scheduler-evidence.md](proptest-scheduler-evidence.md).

### Failure record

`failure.json` is produced by the foundation harness with the tier-bearing reproduction command
(`cargo xtask test --summary --properties --property H-REGISTRY --tier <routine|extended> --seed <seed>`),
the full context, concrete/debug input, direct reproduction, captured logs and shrink status.
Committed regressions are named concrete inputs, never seeds. Control homes:
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/registry_controls.rs` for the
aggregate/ordering/warning witnesses and
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/manifest_controls.rs` for raw
TOML ingestion, plus
`crates/slicer-scheduler/tests/common/property_scheduler_contracts/bound_controls.rs` for bound and
shrink rejection.

### Acceptance

The target was actually executed through the registry with `--exact --nocapture`; the relational
model and the direct helper result are checked on every case. Generator and shrink validity are
demonstrated by `scheduler_registry_oracle_and_warning_controls` (rejecting raw name order
substituted for the helper's `(priority, name)` sequence, reversed ties, implicit seeds, an omitted
declaration, the wrong first-seen contract, a lost warning and a cleared sentinel), by
`scheduler_manifest_region_split_rejections`, and by the pre-sample population guard. The
plan-mode check fails if `build_execution_plan` stops
aggregating or adds seeds. Existing baselines `region_split_manifest_tdd`,
`region_split_aggregation_tdd` and `region_split_plan_tdd` pass. Defect outcome: **no production
defect in `aggregate_region_splits`, `canonical_variant_chain_order` or the plan-aggregation call
was observed**; no regression was manufactured. Neither the helper assertion nor the plan check is
evidence of RegionMapping integration.

## H-RUNTIME-DISPATCH

Registry record: `id = "H-RUNTIME-DISPATCH"`, `package = "slicer-runtime"`,
`target_kind = "test"`, `target_name = "property_runtime"`,
`test_name = "proptest_runtime_semantic_dispatch"`,
`production_symbol = "slicer_wasm_host::selection::paint_allows_layer"`,
`production_path = "crates/slicer-wasm-host/src/selection.rs"`,
`features = ["slicer-runtime/property_testing"]`, `requires_host_algos = true`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-runtime-dispatch"`.

### Stable property ID

`H-RUNTIME-DISPATCH` — transparent when the declared semantic set is empty or the slice is
absent, and match-gated when a declared semantic appears anywhere in a layer's ordered paint
chains, at `(module, layer)` granularity.

### Owner

Package `slicer-runtime`; integration test target `property_runtime`
(`crates/slicer-runtime/tests/property_runtime.rs`) with test
`proptest_runtime_semantic_dispatch`. Production surface:
`slicer_wasm_host::selection::paint_allows_layer` (`crates/slicer-wasm-host/src/selection.rs`).
Real runtime consumption is `slicer_runtime::layer_executor::build_and_commit_layer_selection_index`
(`crates/slicer-runtime/src/layer_executor.rs`), `LayerSelectionIndex::selection_for`
(`crates/slicer-wasm-host/src/selection.rs`) and the layer executors reading
`selection_for(...).invoke`. Deterministic control:
`runtime_semantic_dispatch_oracle_controls` in
`crates/slicer-runtime/tests/property_runtime_contracts.rs` (target
`property_runtime_contracts`, run as
`cargo test -p slicer-runtime --features property_testing --test property_runtime_contracts`).
The shared domain/oracle control in that target is
`runtime_property_domain_and_oracle_controls`.

### Contract

`paint_allows_layer(&HashSet<String>, Option<&SliceIR>) -> bool` allows the pair
iff the declared set is empty, the slice is absent, or at least one region's ordered
`variant_chain` names a declared semantic. Names are case-sensitive strings; `PaintValue`
contents and polygon emptiness are irrelevant. Gating is per `(module, layer)`, never per region,
so a present matching scene selects every candidate region of the neutral module-backed stage, in
committed slice order, not only the matching regions. An absent-slice scene, or a transparent
(empty declared set) scene with no regions, keeps `invoke = true` with zero model targets; a
populated transparent scene keeps `invoke = true` and selects every candidate target; a
present-empty slice with a nonempty declared set yields `invoke = false`.

### Domain

`slicer-runtime`'s `property_testing` profiles carry `max_layers`, `max_regions`, `max_semantics`,
`max_chain_len` and `max_text_bytes`, inside the packet's safety ceilings: `2..=4` layers,
`2..=4` regions, `2..=4` modules, `2..=8` semantic choices, `2..=4` chain length and `8..=64`
bytes per variable semantic name. Committed profile values are routine
`3 / 3 / 4 / 3 / 32` and extended `4 / 4 / 8 / 4 / 64` in that key order, with `cases = 64 / 512`
and `max_default_size_range = 32 / 128`. Every bound is checked from `u64` before narrowing; the
local domain permits at most 64 input vertices per region-field carrier. A cell-budget exceedance
is an explicit failed/exhausted diagnostic, never a correctness pass.

### Strategy

The case strategy draws a scene mode from a plan so that absent slice, transparent empty declared
set, present matching, present nonmatching, present-empty and case-differing scenes are all
reachable, with generated ordered chains whose region order is permuted by the property. Semantic names, chain length and per-region
chain order are shrinkable dimensions; the scene mode tag is not. Every generator is total, so the
zero rejection budgets are honest: any rejection is reported as an exhausted run rather than
silently consuming budget, and the deterministic control in the companion target covers each
documented mode explicitly rather than relying on a sampled population.

### Oracle

The oracle evaluates the independent predicate `declared.is_empty() || slice.is_none() ||
any(region, any(chain, declared.contains(semantic)))` over the generated declared set and ordered
chains; it never calls `paint_allows_layer`. For each committed scene it derives the expected
`(invoke, target table)` from the declared set and slice presence: a matching present scene selects
every candidate region in committed slice order; an absent-slice or present-empty transparent scene
selects none; a populated transparent scene selects every candidate; and a present-empty slice with
a nonempty declared set yields `invoke = false`. `PaintValue` and polygon
fields are deliberately not consulted, so no geometric claim is made here; the shared finite
cell-label comparator described under `H-RUNTIME-PARTITION` governs the sibling geometric charters,
not this one.

### Tolerance

Exact boolean and `invoke` equality, exact ordered identifier sequences and case-sensitive string
identity. No numeric tolerance and no epsilon exist on this surface.

### Shrinking

The scene mode is drawn from a non-shrinkable plan, so a shrink cannot turn an absent-slice case
into a present-matching one or a transparent case into a gated one. Shrinking may simplify semantic
names, chain length and per-region chain order, and the check recomputes the independent predicate
for the shrunk case. The shrink audit asserts that the mode and the verdict signature survive
simplification. Shrink limits are profile-bounded (`max_shrink_iters` 64/128,
`max_shrink_time_ms = 0`); completion is reported `null`/unknown because Proptest 1.11 cannot
identify it.

### Execution

Both tiers run `in_process` with `timeout_ms = 0` and chacha. The registry-backed command is
`cargo xtask test --summary --properties --property H-RUNTIME-DISPATCH --tier routine|extended`,
and `--seed <u64>` replays a retained stream. All four H-RUNTIME profiles carry zero rejection
budgets and total generators. The recorded Windows runs are 0.54 s routine, 0.74 s extended and
0.55 s retained-seed replay, each with 64/512/64 accepted cases and zero rejected test calls; the
whole four-record lane took 57.1 s routine, 133.7 s extended and 55.5 s routine replay. Full
provenance and per-run artifacts are in
[proptest-runtime-evidence.md](proptest-runtime-evidence.md), which is the only place measured
values are quoted.

### Failure record

The foundation harness writes the predecessor context/case/failure contract (`PropertyRun`
artifacts under `target/`): classification, full context, concrete/debug input, the tier-bearing
reproduction command
(`cargo xtask test --summary --properties --property H-RUNTIME-DISPATCH --tier <routine|extended> --seed <seed>`),
direct reproduction and shrink status. Committed regressions are named concrete inputs, never
seeds. `ConcreteInput` supports geometry and mesh payloads only, so any `SliceIR`, authority or
transaction fixture is an explicit finite builder plus a real call, not a serialized input enum.
The deterministic control home is
`crates/slicer-runtime/tests/property_runtime_contracts.rs`.

### Acceptance

This charter maps to AC-1: the property compares the public predicate and real committed selections
against the independent semantic-set predicate for absent, empty, matching, nonmatching,
case-differing and order-differing modes. `runtime_semantic_dispatch_oracle_controls` falsifies
require-all gating, value-sensitive (`PaintValue`-dependent) gating, per-region filtering and
absent-slice skip. The existing pure helper baseline `selection::tests` in `slicer-wasm-host` and the
executor baseline `shared_selection_runtime_tdd` are recorded in the evidence file; the four
measured Windows runs (routine, extended, seed replay, combined routine) all passed with 64/512/64
accepted cases and zero rejects.

## H-RUNTIME-SELECTION

Registry record: `id = "H-RUNTIME-SELECTION"`, `package = "slicer-runtime"`,
`target_kind = "test"`, `target_name = "property_runtime"`,
`test_name = "proptest_runtime_selection_authority"`,
`production_symbol = "slicer_runtime::layer_executor::build_and_commit_layer_selection_index"`,
`production_path = "crates/slicer-runtime/src/layer_executor.rs"`,
`features = ["slicer-runtime/property_testing"]`, `requires_host_algos = true`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-runtime-selection"`.

### Stable property ID

`H-RUNTIME-SELECTION` — exact per-target config authority in the committed layer selection index,
and atomic all-or-nothing publication of that index.

### Owner

Package `slicer-runtime`; integration test target `property_runtime` with test
`proptest_runtime_selection_authority`. Production surfaces:
`slicer_wasm_host::selection::build_layer_selection_index` and
`LayerSelectionIndex::selection_for` (`crates/slicer-wasm-host/src/selection.rs`), the runtime seam
`slicer_runtime::layer_executor::build_and_commit_layer_selection_index`
(`crates/slicer-runtime/src/layer_executor.rs`), and the write-once
`Blackboard::commit_layer_selection_index` / `Blackboard::layer_selection_index`
(`crates/slicer-runtime/src/blackboard.rs`). Deterministic control:
`runtime_selection_exact_key_and_publication_rejections` in
`crates/slicer-runtime/tests/property_runtime_contracts.rs`, alongside the shared
`runtime_property_domain_and_oracle_controls`.

### Contract

The pure builder consumes `LayerSelectionIndexInput` and returns
`Result<LayerSelectionIndex, LayerSelectionBuildError>`, whose `error` is a
`SelectionError::{UnknownLayerStage { stage_id }, MissingModelConfig { key }}`.
Config authority is exact: the map address is the layer, object, source region id and full ordered
chain. Ordinary empty-chain ids are used verbatim; painted nonmodifier ids with a nonempty chain are
divided by `PAINT_VARIANT_REGION_ID_STRIDE` only at the map address (the source identity keeps the
painted id); modifier-namespace ids are used verbatim; the layer, object id and ordered chain are
exact, never sorted or normalized. A missing exact key fails before any commit and publishes
nothing. Source target order follows committed slice order and survives map insertion permutations;
`selection_for` returns `None` for an unbound pair, and consumers must reject that rather than
reconstruct a selection.

The runtime seam builds the same index from `ExecutionPlan` and `Blackboard` state (committed
slices, region map and support-plan presence; it supplies no whole-print perimeter product) and
publishes through the write-once `Blackboard::commit_layer_selection_index`. A duplicate runtime
publication is rejected by that seam and preserves the original `Arc` identity; `layer_selection_index`
still returns the first committed index. Raw unknown stage ids fail at the pure builder only: the
runtime seam filters to layer stages before calling it, and the pure builder rejects an unknown or
non-WIT stage.

### Domain

Profiles carry `max_layers`, `max_regions`, `max_modules` and `max_chain_len` inside the packet's
`2..=4` ceilings for each key, with `cases = 64 / 512` and `max_default_size_range = 32 / 128`.
Committed values are routine `3 / 3 / 2 / 3` and extended `4 / 4 / 4 / 4`, with zero rejection
budgets. Every bound is checked from `u64` before narrowing. Identity pools are finite, and any
domain exhaustion is an explicit failed/exhausted diagnostic rather than a pass.

### Strategy

The generator builds committed layers, modules, region-map entries and ordered chains (including a
same-layer cross-object pair with the same numeric source id and chain), then selects a case mode
from a non-shrinkable plan: exact match, map insertion permutation, missing exact key, reversed or
sibling-only authority, and the object-distinction pair. Duplicate runtime publication and raw
unknown stage ids are deterministic inputs in the companion control, not generated modes: the
runtime seam filters its roster and is not an unknown-stage validator. The generator is total, so
any rejection is reported as exhaustion; the fixed-seed shrink audit requires both the positive and
negative modes.

### Oracle

The oracle builds the exact-key authority table from the generated `RegionMapIR` entries and
source regions, recomputing the map-space address from the written rule instead of calling
`region_map_key_for_sliced_region`. It derives expected `(invoke, config identity, ordered target
keys)` for every module and layer, including the source order after an insertion permutation; it
never calls `build_layer_selection_index` or `build_and_commit_layer_selection_index` to certify
itself. It derives the expected `SelectionError` variant and exact key for the generated
missing/reversed/sibling-only/cross-object modes and asserts that a missing-key failure leaves
`layer_selection_index()` unchanged. Duplicate publication (`Arc::ptr_eq` on the original index) and
raw unknown stage rejection at the pure builder are deterministic control claims, because the
runtime seam filters its roster before calling the builder.

### Tolerance

Exact enum, key, config identity, ordered-key sequence and pointer-identity equality. No tolerance
and no epsilon exist on this surface.

### Shrinking

The authority mode and every flag that decides whether a fault exists are drawn from a
non-shrinkable plan, so a shrink cannot turn an exact match into a missing-key case or a
duplicate-publication case into a first publication. Shrinks may simplify layer, module and chain
slots and map insertion order; expectations are recomputed for the shrunk case. The shrink audit
asserts the mode and its distinguishing witness survive. Shrink limits are profile-bounded;
completion is reported `null`/unknown.

### Execution

Both tiers run `in_process` with `timeout_ms = 0` and chacha, zero rejection budgets and total
generators. The registry-backed command is
`cargo xtask test --summary --properties --property H-RUNTIME-SELECTION --tier routine|extended`;
`--seed <u64>` replays a retained stream. The recorded Windows runs are 2.19 s routine, 2.79 s
extended and 2.15 s retained-seed replay, each with 64/512/64 accepted cases and zero rejected test
calls; provenance is in [proptest-runtime-evidence.md](proptest-runtime-evidence.md).

### Failure record

`PropertyRun` writes the predecessor context/case/failure artifacts under `target/`, including the
tier-bearing reproduction command
(`cargo xtask test --summary --properties --property H-RUNTIME-SELECTION --tier <routine|extended> --seed <seed>`),
full context, concrete/debug input, direct reproduction and shrink status. Committed regressions are
named concrete inputs, never seeds. `ConcreteInput` supports geometry and mesh only, so authority
and publication fixtures are explicit finite builders that construct real `LayerSelectionIndexInput`
or `ExecutionPlan`/`Blackboard` state and then call the real seam. The control home is
`crates/slicer-runtime/tests/property_runtime_contracts.rs`.

### Acceptance

This charter maps to AC-2. The property reads exact per-target config markers and source identities
from committed index authority rather than default, sibling or first-match lookups, and source
target order survives map insertion permutations. `runtime_selection_exact_key_and_publication_rejections`
falsifies missing, reversed, sibling-only and object-blind authority, partial publication after
earlier valid work, raw unknown stages accepted by the pure builder instead of being rejected
there, and a duplicate publication that replaces the original `Arc`. The property adds the same-layer cross-object twin with distinct
markers. Existing baselines `shared_selection_runtime_tdd`, `source_identity_repair_tdd` and
`apply_commit_regression_tdd` are recorded in the evidence file; the measured Windows runs all
passed with 64/512/64 accepted cases and zero rejects.

## H-RUNTIME-MODIFIERS

Registry record: `id = "H-RUNTIME-MODIFIERS"`, `package = "slicer-runtime"`,
`target_kind = "lib"`, `target_name = "slicer_runtime"`,
`test_name = "region_partition::property_tests::proptest_runtime_modifier_transactions"`,
`production_symbol = "slicer_runtime::region_partition::split_regions_by_modifier_footprints"`,
`production_path = "crates/slicer-runtime/src/region_partition.rs"`,
`features = ["slicer-runtime/property_testing"]`, `requires_host_algos = true`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-runtime-modifiers"`.

### Stable property ID

`H-RUNTIME-MODIFIERS` — first-owner modifier cell ownership, full-identity collision rejection and
all-or-nothing `SliceIR`/arena transactions across the private splitter, raw staging wrapper and
public prepass.

### Owner

Package `slicer-runtime`; library target `slicer_runtime`, submodule
`region_partition::property_tests`
(`crates/slicer-runtime/src/region_partition/property_tests.rs`), test
`proptest_runtime_modifier_transactions`. The submodule mounts
`crates/slicer-runtime/tests/common/property_runtime_cells.rs` by `#[path]`; the integration
control target `crates/slicer-runtime/tests/property_runtime_contracts.rs` mounts the same file, so
both identities share one cell model. Production surfaces are the private
`split_regions_by_modifier_footprints(&mut SliceIR, impl IntoIterator<Item = (String, Vec<ExPolygon>)>) -> Result<(), String>`,
the raw staging wrapper `split_modifier_footprints`, the public
`split_modifier_sub_regions_for_prepass(&mut SliceIR, &MeshIR)` and the arena hook
`sync_perimeter_infill_areas_into_slice(&mut LayerArena, u32)`, all in
`crates/slicer-runtime/src/region_partition.rs`.

Deterministic library controls run as
`cargo test -p slicer-runtime --features property_testing --lib -- region_partition::property_tests::runtime_`:
`runtime_modifier_ownership_and_angle_controls`,
`runtime_modifier_identity_and_transaction_rejections` and
`runtime_modifier_collision_restores_taken_slice`. The shared cross-cutting control in
`crates/slicer-runtime/tests/property_runtime_contracts.rs` is
`runtime_property_domain_and_oracle_controls`. The registry-backed lib identity runs with
`--lib --exact region_partition::property_tests::proptest_runtime_modifier_transactions`.

### Contract

Footprints are applied in supplied order; the first footprint that intersects a cell owns it, and
later footprints see only the remaining parent area. The private splitter reduces every parent
field after each split by exact difference. The public prepass derives footprint order as priority
descending, then original document index, and excludes support enforcer and support blocker kinds.
Both splitters work on a clone and assign only on success, so a rejected call leaves the full input
`SliceIR` equal to its pre-call snapshot.

Collision is full identity `(object_id, minted modifier region id, ordered variant_chain)`: an equal
numeric id on a different object or a different chain is a different identity, not a collision.
Unsupported parent ids fail with `parent_region_id` context. Children copy parent metadata and the
ordered chain, clear `segment_annotations`, mint modifier-namespace ids that decode to the parent,
add no `PerimeterIR` walls, and keep per-polygon internal-bridge angles index-aligned; an empty
angle vector falls back to `bridge_orientation_deg`. An empty parent `infill_areas` is restored
from the original polygons before splitting.

**Arena boundary.** `sync_perimeter_infill_areas_into_slice` takes the staged slice, and a rejected
modifier split must not leave `arena.slice() = None` or a half-repartitioned slice. The required
contract is hook-local rollback to the entry-state slice: the transaction starts at entry to this
hook, and this charter makes no claim about earlier perimeter module commits. The named concrete
regression for this obligation is `runtime_modifier_collision_restores_taken_slice`. Source
inspection showed the hook taking the slice through `take_slice` and then propagating
`split_modifier_footprints(...)?` before `set_slice`. Implementation reproduced the loss with a
concrete rejecting input (pre-fix failure: `the hook took the staged SliceIR and returned the split
error without re-staging it, leaving the arena without a layer`), fixed it by running role sync and
the split on a working copy and re-staging the entry-state slice on any rejection, and re-ran the
named regression green. The focused fix and the red/green evidence are recorded in the evidence
file.

### Domain

Profiles carry `max_regions`, `max_footprints`, `max_abs_units` and `max_cells`: routine
`3 / 3 / 512 / 512`, extended `4 / 4 / 4096 / 2048`, with `cases = 64 / 512` and
`max_default_size_range = 32 / 128`. The safety ceilings are `2..=4` regions, `2..=4` footprints,
`16 <= max_abs_units <= 2^20` and `16 <= max_cells <= 4096`, with at most 64 input vertices per
region-field or footprint carrier. Bounds are checked from `u64` before narrowing. Even-coordinate
grids and bounded orthogonal records keep the cell product small; cell-budget exceedance is an
explicit failed/exhausted diagnostic, never a pass.

### Strategy

The generator draws region-field carriers (polygons, the four fill roles, internal solid fill, raft
fill and index-aligned internal-bridge polygons/angles), footprint rectangles or orthogonal
polygons, object ids, ordered chains and an object-sharing plan so cross-object and cross-chain
identities are reachable. A non-shrinkable case plan tags ownership, collision, unsupported parent,
empty `infill_areas` restoration and angle-fallback modes, and constructs the `Valid`, `Collision`,
`UnsupportedParent` and `LaterFailure` modes plus holed parents, an inert empty footprint, a
later-footprint overlap and a role-precedence overlap. The fixed-seed shrink audit requires all four
mode tags and the structural holed/empty/overlap witnesses; the generator is total, so any rejection
is an exhausted run.

### Oracle

The oracle is a finite cell-label model, not a Clipper call. Boundaries from the inputs and from the
observed output partition the checked box into elementary open rectangles; a doubled-coordinate
midpoint in every cell is classified with the independent
`slicer_sdk::test_support::property_testing::oracles::point_in_region`
(`crates/slicer-sdk/src/test_support/property_testing/oracles/mod.rs`), and the first-owner truth
table assigns each cell to the earliest intersecting footprint for the owning parent region, or to
that region's parent remainder. Ownership, conservation, metadata, annotation clearing, namespace
decoding and angle association are evaluated from those labels. Output edges must be orthogonal and on the collected
grid, and output area must equal the independently labelled cell-sum area using absolute
contour/hole double areas from `signed_area2`. No random interior probes, no Clipper Boolean output
as oracle and no epsilon are used.

### Tolerance

Geometry is compared on the exact integer grid with no epsilon. Byte equality is required only for
the modifier-child contained-bridge preservation branch; elsewhere ring start and ring order may
differ and geometry is compared semantically.

### Shrinking

The case mode and every flag that decides first ownership, collision identity, unsupported parent
and empty-`infill_areas` restoration come from a non-shrinkable plan. A shrink may simplify
geometry, footprint count and chain slots, and the oracle is recomputed for the shrunk case. The
shrink audit requires the four mode tags and the structural holed/empty/later-footprint/role-
precedence witnesses to survive at every simplification step, and re-runs the same checker on the
generated and the final simplified tree. Shrink limits are profile-bounded; completion is reported
`null`/unknown, and an audit endpoint at the 64-step cap is not claimed minimal.

### Execution

Both tiers run `in_process` with `timeout_ms = 0` and chacha, zero rejection budgets and total
generators. The registry-backed command is
`cargo xtask test --summary --properties --property H-RUNTIME-MODIFIERS --tier routine|extended`;
`--seed <u64>` replays a retained stream, and the direct library
identity is selected with `--lib --exact`. The recorded Windows runs are 22.31 s routine, 64.46 s
extended and 20.97 s retained-seed replay, each with 64/512/64 accepted cases and zero rejected test
calls; provenance is in [proptest-runtime-evidence.md](proptest-runtime-evidence.md).

### Failure record

`PropertyRun` writes the predecessor context/case/failure artifacts under `target/` with the
tier-bearing reproduction command
(`cargo xtask test --summary --properties --property H-RUNTIME-MODIFIERS --tier <routine|extended> --seed <seed>`),
full context, concrete/debug input, direct reproduction and shrink status. Committed regressions are
named concrete inputs, never seeds. `ConcreteInput` supports geometry and mesh payloads only, so
`SliceIR` and transaction fixtures are explicit finite builders that construct the real arena and
slice state and then call the private splitter, raw wrapper, public prepass or hook. The
deterministic control home is the `region_partition::property_tests` submodule in
`crates/slicer-runtime/src/region_partition/property_tests.rs`; the shared model lives in
`crates/slicer-runtime/tests/common/property_runtime_cells.rs`.

### Acceptance

This charter maps to AC-3 and AC-N1. `runtime_modifier_ownership_and_angle_controls` falsifies
lower-priority overlap theft, cross-object/chain leakage, omission of the raft and
internal-bridge carriers, and lost angle association. `runtime_modifier_identity_and_transaction_rejections`
falsifies collision and unsupported-parent failures that do not occur after earlier possible work
through the private splitter, raw wrapper, public prepass and hook, private and public `SliceIR`
inputs that differ from their originals after an error, hook failures that do not return the
specified `FatalModule` or do not retain the original `arena.slice()`, `PerimeterIR`, raw sentinel
and roles, and a same numeric id with a different full identity treated as a collision.
`runtime_modifier_collision_restores_taken_slice` is the named regression for the taken-slice loss:
it fails pre-fix with the loss message and passes post-fix. The generator and oracle are also pinned
by `runtime_property_domain_and_oracle_controls`; the existing baseline
`modifier_region_split_tdd` is recorded in the evidence file and stayed 15/15 after the fix. The
measured Windows runs all passed with 64/512/64 accepted cases and zero rejects.

## H-RUNTIME-PARTITION

Registry record: `id = "H-RUNTIME-PARTITION"`, `package = "slicer-runtime"`,
`target_kind = "lib"`, `target_name = "slicer_runtime"`,
`test_name = "region_partition::property_tests::proptest_runtime_fill_partition"`,
`production_symbol = "slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice"`,
`production_path = "crates/slicer-runtime/src/region_partition.rs"`,
`features = ["slicer-runtime/property_testing"]`, `requires_host_algos = true`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-runtime-partition"`.

### Stable property ID

`H-RUNTIME-PARTITION` — conditional `bridge > bottom > top > sparse` fill-role precedence over the
wall inset, exact full-source donor joins, and nonfatal missing-donor preservation.

### Owner

Package `slicer-runtime`; library target `slicer_runtime`, submodule
`region_partition::property_tests`
(`crates/slicer-runtime/src/region_partition/property_tests.rs`), test
`proptest_runtime_fill_partition`. Production surface:
`slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice`
(`crates/slicer-runtime/src/region_partition.rs`), driven through a real `LayerArena` with staged
`SliceIR` and `PerimeterIR`. Library controls run as
`cargo test -p slicer-runtime --features property_testing --lib -- region_partition::property_tests::runtime_`:
`runtime_partition_precedence_and_identity_controls` and
`runtime_partition_stage_and_domain_rejections`, with the shared
`runtime_property_domain_and_oracle_controls` in
`crates/slicer-runtime/tests/property_runtime_contracts.rs`. The property body mounts
`crates/slicer-runtime/tests/common/property_runtime_cells.rs`.

### Contract

The partition runs over `W = donor.infill_areas ∩ slice_region.polygons` with the conditional
precedence `bridge > bottom > top > sparse`. When `W` is nonempty, the four output roles are
pairwise disjoint and their union equals `W`. When `W` is empty and the region has walls, all four
roles are empty. When `W` is empty and the region has no walls, the original bridge is preserved,
top becomes the original top minus bridge, bottom and sparse are empty, and this branch makes no
subset claim against the empty `W`. The conditional empty-W/no-wall fallback versus the
nonempty-inset clipping rule is the actual policy; neither is generalized into a universal
identity.

A missing donor is not fatal: the region's roles are unchanged and a warning names the object and
region, with the modifier variant also naming the parent. Missing `SliceIR` or missing
`PerimeterIR` is `LayerStageError::FatalModule` with stage `Layer::Perimeters`, module
`host:region_partition` and message fragments `no staged SliceIR` / `no staged PerimeterIR`; the
missing-perimeter path restores the taken slice before returning. Donors join exactly on
`(object_id, source region id, ordered variant_chain)`; modifier children decode only the parent
id while preserving object and chain. Immutable `Blackboard` data and `PerimeterIR` are unchanged by
the arena-only partition; prepared-region caches may invalidate or recompute but must never serve
stale geometry.

### Domain

Profiles carry `max_regions`, `max_abs_units` and `max_cells`: routine `3 / 512 / 512`, extended
`4 / 4096 / 2048`, with `cases = 64 / 512` and `max_default_size_range = 32 / 128`. Safety
ceilings are `2..=4` regions, `16 <= max_abs_units <= 2^20` and `16 <= max_cells <= 4096`, with at
most 64 input vertices per region-field carrier. Bounds are checked from `u64` before narrowing;
cell-budget exceedance is an explicit failed/exhausted diagnostic, not a pass.

### Strategy

The generator draws `SliceIR` regions with polygons and per-role carriers, a `PerimeterIR` whose
donors include exact-key matches, a missing donor, a sibling-only pair collision and a
exact reverse two-entry chain pair (with distinct donors), plus wall/no-wall and empty/nonempty
inset modes and a nonempty effective inset with role-precedence overlap. A non-shrinkable case plan
tags each conditional branch, the missing-donor outcome and the chain-exactness witnesses. Fatal
stage modes (`no staged SliceIR` / `no staged PerimeterIR`) and the modifier later-failure
transaction are deterministic control inputs, not generated modes, because they describe staging
errors rather than partition geometry. The generator is total, so any rejection is an exhausted
run.

### Oracle

The oracle is the exact finite cell-label model. Boundaries from the generated inputs and the
observed output partition the checked box into elementary open rectangles; a doubled-coordinate
midpoint per cell is classified with
`slicer_sdk::test_support::property_testing::oracles::point_in_region`
(`crates/slicer-sdk/src/test_support/property_testing/oracles/mod.rs`), independently testing
membership in the donor inset, the slice-region polygons and each original role carrier. The
conditional truth table then assigns the expected role: nonempty `W` uses `bridge > bottom > top >
sparse` over `W`; empty `W` with walls assigns no role; empty `W` without walls assigns original
bridge, original top minus bridge, no bottom and no sparse. Role/ownership truth tables are
evaluated per cell, including donor identity from the exact `(object, source region id, ordered
chain)` key. Output edges must be orthogonal and on the collected grid, and output area must equal
the independently labelled cell-sum area using absolute contour/hole double areas from
`signed_area2`. No random interior probes, no Clipper Boolean output as oracle and no epsilon are
used; a missing-donor case is checked for unchanged roles and the named warning, not as a fatal
error.

### Tolerance

Geometry is compared on the exact integer grid with no epsilon. Ring start and ring order may
differ and geometry is compared semantically; byte equality is not claimed on this surface except
where the modifier-child contained-bridge preservation branch is shared with `H-RUNTIME-MODIFIERS`.

### Shrinking

The case mode and the flags that decide the empty/nonempty inset branch, wall presence, donor
exactness, chain order and missing-donor outcome come from a non-shrinkable plan. A shrink may
simplify region geometry, chain slots and role carriers, and the cell oracle is recomputed for the
shrunk case. The shrink audit requires a nonempty effective inset and a role-precedence overlap tag
to survive at every step; fatal-stage witnesses are deterministic and therefore have no shrink
interaction. Shrink limits are profile-bounded; completion is reported `null`/unknown.

### Execution

Both tiers run `in_process` with `timeout_ms = 0` and chacha, zero rejection budgets and total
generators. The registry-backed command is
`cargo xtask test --summary --properties --property H-RUNTIME-PARTITION --tier routine|extended`;
`--seed <u64>` replays a retained stream, and the direct library
identity is selected with `--lib --exact`. The recorded Windows runs are 13.79 s routine, 48.07 s
extended and 13.31 s retained-seed replay, each with 64/512/64 accepted cases and zero rejected test
calls; provenance is in [proptest-runtime-evidence.md](proptest-runtime-evidence.md).

### Failure record

`PropertyRun` writes the predecessor context/case/failure artifacts under `target/` with the
tier-bearing reproduction command
(`cargo xtask test --summary --properties --property H-RUNTIME-PARTITION --tier <routine|extended> --seed <seed>`),
full context, concrete/debug input, direct reproduction and shrink status. Committed regressions are
named concrete inputs, never seeds. `ConcreteInput` supports geometry and mesh payloads only, so
`SliceIR`, `PerimeterIR` and prepared-cache fixtures are explicit finite builders that stage a real
`LayerArena` and call the real hook. The deterministic control home is the
`region_partition::property_tests` submodule; the shared model is
`crates/slicer-runtime/tests/common/property_runtime_cells.rs`.

### Acceptance

This charter maps to AC-4 and AC-N2. `runtime_partition_precedence_and_identity_controls` falsifies
swapped priority, an unclipped nonempty bridge, pair-only donor joins, missing-donor fatal
conversion, and an unconditional empty-W subset assertion; it also checks that `Blackboard` and
`PerimeterIR` stay unchanged and that prepared views re-associate current geometry.
`runtime_partition_stage_and_domain_rejections` observes the missing-slice and missing-perimeter
`FatalModule` identity and message fragments and the slice restoration on the missing-perimeter
path, and rejects unsafe domains. Existing baselines `modifier_region_split_tdd` and
`blackboard_layer_arena_tdd` are recorded in the evidence file. Cross-cutting execution for all
four records maps to AC-5, and defect disposition maps to AC-N3, both carried by
[proptest-runtime-evidence.md](proptest-runtime-evidence.md); the measured Windows runs all passed
with 64/512/64 accepted cases and zero rejects, and the one confirmed SUT defect is fixed with a
red/green regression.

## Direct-domain charters (packet `proptest-openrscad_02_pbt-domains`)

These five records certify the shared direct-input helpers in
`slicer_sdk::test_support::property_testing::{domains,oracles,regression}`. They are
**helper-validation** properties: they prove the domains are valid, independent and
witness-preserving. They are not credits toward host or module SUT coverage, and the
scripted/engine families belong to later packets.

Common execution facts for all five records:

- Package `slicer-sdk`, target `property_domains`, features
  `["slicer-sdk/property_testing"]`, `requires_host_algos = false`, `requires_scad = false`,
  `generator_version = 1`.
- `production_path` is `crates/slicer-sdk/src/test_support/property_testing/domains/mod.rs`,
  where every public entry point is defined; private helpers live in subfiles below it.
- Rejection budgets are zero in both tiers. Every domain is construction-first and total
  (no `prop_assume`, no rejection-only filter), so any rejection means the generator changed
  and must fail as `exhausted` rather than consume budget silently.
- `slicer-sdk` is native-only for this feature (`#[cfg(all(feature = "property_testing",
  not(target_arch = "wasm32")))]`); guest builds and the production `prelude` never see it.

### Oracle independence (applies to all five)

The oracle modules import only `slicer-ir` data types, `num-bigint`/`num-rational` (both
optional and gated by `property_testing`) and the standard library. They do not import
`slicer-core`, Clipper, `geo`, `slicer-scheduler`, or any production validator, geometry
helper or slicing routine. `SDK` already depends on `slicer-core`, so dependency-package
separation alone is not an independence argument; the guarantee is that no production
computation is called by an oracle and every predicate is an independent reference
computation over the same generated input.

### Exact arithmetic and tolerances (applies to all five)

- Coordinate integers are canonical units: `1 unit = 100 nm = 10^-4 mm`. No float round trip
  constructs exact test geometry; `Point2::from_mm` is used only where a real conversion is
  under test (quantization collapse).
- `orientation` and `signed_area2` widen to `i128` before subtracting and use checked
  multiplication/addition; overflow returns `DomainError::ArithmeticOverflow`, never a wrapped
  result. For a validated domain (`|x|,|y| <= 1_000_000`) an orientation determinant is at most
  `8*10^12` and a shoelace term at most `2*10^12`, so every legal domain value fits.
- Intersection parameters, half-plane clipping, point-in-polygon and edge-clearance use exact
  `BigRational` values. Products of denominators are unbounded in principle, so no `i128`
  budget is assumed. Domains are capped at `1_000_000` absolute units to keep the exact
  computation small, not to make it approximate.
- Transform observations use exact equality: quarter-turn matrix entries are `{0, ±1}` and
  translations are dyadic, all exactly representable in `f64`. Arbitrary-rotation invariance is
  not claimed.
- Mesh volume compares the oriented tetrahedron sum against the dimension-derived volume with
  the derived budget `128 * f64::EPSILON * M^3` where `M` is the largest absolute vertex
  coordinate in mm: at most 72 signed monomials of magnitude `<= M^3`, each through at most 17
  rounded operations including the final division by six, is below
  `108 * f64::EPSILON * M^3`. The dimensional volume itself is exact.

## D-SCALARS

Registry record: `test_name = "proptest_direct_scalar_domains"`,
`production_symbol = "slicer_sdk::test_support::property_testing::domains::finite_f32"`,
`bounds = { max_scalar_units, max_id_bytes }`.

### Contract

Finite helpers expose only finite values inside the requested inclusive range, during
generation and shrinking. Nonfinite helpers expose infinities and at least two distinguishable
NaN payload/sign encodings. `rounding_boundary_mm` exposes representable `f32` neighbours of
half-unit `mm_to_units` boundaries (`(k + 0.5) / 10_000` mm). `scalar_config_values` produces
exactly the requested `ConfigValue` variant and range. `id_text(UuidSyntax)` produces the
lowercase `8-4-4-4-12` hex grammar; `id_text(OtherText)` produces its complement within
`max_bytes`. None of these certify whole-`ResolvedConfig` validity: cross-field role/config
compatibility stays with the owner packets.

### Strategy

`finite_f32`/`finite_f64` union a total order-preserving bit-range sampler with explicit range
ends, both zero signs, a reachable subnormal and the representable neighbours of interior
values. `nonfinite_*` are deterministic selects (both infinities, signalled/quiet payloads).
`rounding_boundary_mm` derives boundaries from `mm_to_units = (mm * 10_000).round()` and emits
the value and its `next_down`/`next_up` neighbours. `id_text` builds the UUID grammar from 32
hex digits and rejects `max_bytes < 36` instead of returning an empty strategy; the complement
is forced outside the grammar by construction.

### Tolerance

Exact bit equality (`to_bits`) for floats and exact string grammar for IDs. `ConfigValue`
float equality compares `to_bits()`, so signed zero is observed, not collapsed.

### Acceptance

`domain_scalar_edges_and_config_types` pins both zero signs, subnormals, boundary neighbours,
range ends, infinities, distinct NaN payloads, every requested variant and the UUID/complement
witnesses, and rejects nonfinite/reversed/excessive ranges and a too-small UUID bound.

## D-GRAPHS

Registry record: `test_name = "proptest_direct_graph_domains"`,
`production_symbol = "slicer_sdk::test_support::property_testing::domains::graphs"`,
`bounds = { max_nodes }`.

### Contract

Valid graphs have unique node IDs and only known endpoints. DAGs are built from ordered nodes
and forward edges. `CycleWithTail` contains a real directed cycle of at least two nodes plus at
least one acyclic node reachable from it. Malformed families preserve their named duplicate-node
or unknown-endpoint witness and are used only at APIs that promise validation, never as input to
the already-validated scheduler contract.

### Oracle

`graph_reachability` enumerates every nonempty path by breadth-first search and reports
`(v, v)` only for a cycle, so it is a path model, not reflexivity and not a copy of Kahn's
algorithm. `graph_cycle_remainder` is every cycle node plus every node reachable from a cycle
node. Malformed raw graphs are rejected with `InvalidInput`. The scheduler's
`topological_sort` is never imported or called.

### Shrinking

Cycles and tails are constructed from `(cycle_len, tail_len)` parameters and re-established in
the map, so shrinking keeps a real cycle and a downstream tail. Malformed witnesses append the
fault structurally, so it survives shrinking.

### Acceptance

`domain_graph_models_and_cycle_tail` hand-computes path sets, proves a concrete cycle-plus-tail
includes the acyclic tail in `graph_cycle_remainder`, and checks that duplicate/unknown-endpoint
witnesses are rejected and retained through shrink/complicate cycles.

## D-IR

Registry record: `test_name = "proptest_direct_ir_domains"`,
`production_symbol = "slicer_sdk::test_support::property_testing::domains::anchor_cases"`,
`bounds = { max_entities, max_travels, max_points, max_layers }`.

### Contract

Anchor cases are membership fixtures: `Valid` anchors always belong to `ordered_entities`,
`Dangling` always has a missing reference, and `0`/`u64::MAX`, empty valid layers and repeated
anchors are explicit witnesses. `entity_layers` builds bounded paths with finite dyadic mm
coordinates, positive widths/flow/speed factors, fresh strictly increasing `LayerEntityIdGen`
IDs, matching entity/path roles, consistent layer indices and strictly increasing finite Z.
Permutations are complete index bijections; locked orders are complete permutations in which
each authored group appears contiguously and in its authored order. None of this certifies
printable geometry.

### Oracle and shared adoption

`first_missing_anchor` scans travel order against a sorted ID set and never calls
`validate_travel_anchors`. `is_permutation` and `locked_groups_are_contiguous` check completeness
and authored order separately. The owning `H-IR-ANCHORS` property in
`crates/slicer-ir/tests/property_ir.rs` now consumes `anchor_cases` and `first_missing_anchor`
while still invoking the real `validate_travel_anchors` through `check_travel_anchors_charter`;
helper tests alone do not replace that validator call.

### Acceptance

`domain_ir_anchors_permutations_and_layers` proves anchor mode preservation, full index
bijections, forward-contiguous disjoint lock groups, matching role/path metadata, fresh
per-layer IDs and finite ordered Z. `H-IR-ANCHORS` routine execution still passes against the
library validator.

## D-GEOMETRY

Registry record: `test_name = "proptest_direct_geometry_domains"`,
`production_symbol = "slicer_sdk::test_support::property_testing::domains::geometry_cases"`,
`bounds = { max_abs_units, max_vertices, max_holes, max_components, max_shrink_candidates }`.

### Bounded topology catalog

The catalog is deliberately bounded, not a general simple-polygon generator:

- `Convex`: axis-aligned rectangle, optional one or two chamfered corners, optional collinear
  vertices, and a triangle when the vertex bound is tight.
- `Concave`: L-shape with a parameterized notch plus optional collinear vertices.
- `NonStarShaped`: a deep U-notch whose right wall (directed downward) imposes `x >= xR` and
  whose left wall (directed upward) imposes `x <= xL` with `xR > xL`, so the closed visibility
  kernel is empty at every notch depth `>= 1`. This contradiction is checked from coordinates in
  the contract test, not by trusting the tag or the kernel helper.
- `Holed`: rectangle with CW rectangular holes that are strictly internal and mutually
  disjoint with at least one unit of clearance.
- `Disconnected`: separated CCW rectangles with one-unit gaps between closures.
- `Thin`: narrow notch/corridor at most `max(1, bbox_min_dim/16)` units wide with
  `bbox_min_dim >= 16`.
- `NearDegenerate`: the same family with an exact one-unit clearance.
- `Empty`: no components, zero minima.

`validate_geometry` checks winding, strict hole nesting/disjointness, component separation,
count bounds, vertex bounds and the class witness. It is a small-input test predicate, not
production repair logic; `Thin`/`NearDegenerate` are structural edge-clearance witnesses, not
general thinness proofs.

### Shrinking

`GeometryValueTree` implements the real `current`/`simplify`/`complicate` protocol. `simplify`
generates a deterministic candidate list (optional vertex, hole, component, dimension and
translation reduction), revalidates every candidate with `validate_geometry`, and accepts only a
strictly smaller `geometry_complexity` tuple `(components, holes, vertices, sum_abs_xy_units)`.
Candidate attempts are bounded by `max_shrink_candidates`; exhaustion records the diagnostic
`candidate_limit_reached`, retains the best known case and reports incomplete shrinking. A
`false` return is never described as global minimality. `complicate` restores the previous
accepted case and repeated calls walk back to the initial value.

### Arithmetic

`geometry_complexity` accumulates with checked `i128::unsigned_abs` and `checked_add`, returning
`ArithmeticOverflow` rather than wrapping. Geometry coordinates are capped at `1_000_000`
absolute units and count bounds must fit `u32`; excessive bounds are rejected, not narrowed.
Quantization collapse is an explicit negative fixture: a float-separated ring that collapses
after `mm_to_units` is rejected because the post-conversion coordinates repeat.

### Acceptance

`domain_geometry_topology_witnesses`, `domain_shrink_protocol_preserves_witnesses`,
`domain_known_failures_shrink_structurally` and `domain_exhaustion_is_not_success` prove that
every class has coordinate-level evidence, every exposed shrink state validates and preserves
its class/minimum witnesses, and real vertex/hole/component reductions occur from nonminimal
witnesses. The known-failure probes also run failing predicates through a real Proptest
`TestRunner`: a three-hole failure shrinks to two holes and a four-component failure to three,
while always-failing predicates keep the required minima (three holes when `min_holes = 3`, four
components when `min_components = 4`). Ring-vertex reduction is asserted by the explicit
nonminimal five-vertex convex witness rather than by the runner, because the convex generator
cannot guarantee a nonminimal vertex count on its first failing case. A geometry strategy whose
`max_shrink_candidates` is zero runs
through `PropertyRun::check`, emits `candidate_limit_reached`, reports
`failed`/`sut_failure` with `shrink_completed = null` and retains the failing `GeometryCase`;
rejection exhaustion separately reports `exhausted` with its retained input. Neither path is
reported as a pass.

## D-MESH

Registry record: `test_name = "proptest_direct_mesh_domains"`,
`production_symbol = "slicer_sdk::test_support::property_testing::domains::analytical_meshes"`,
`bounds = { max_extent_mm, max_translation_mm }`.

### Contract

Boxes are 8 vertices/12 triangles, right triangular prisms 6/8, both closed and consistently
outward oriented, with positive dyadic dimensions no larger than `max_extent_mm`. `expected_bounds`
and `expected_volume_mm3` are derived from the dimensions, never from the triangle sum or a
production mesh routine. `validate_indexed_mesh` checks finiteness, triangle-stream
divisibility, index range, distinct triangle vertices, nonzero triangle area, and — in closed
mode — that every undirected edge occurs exactly twice in opposite directions. It does not
claim absence of 3D self-intersections. `mesh_signed_volume_mm3` computes the oriented
tetrahedron sum with finite-intermediate checks.

### Oracle and tolerance

The volume comparison uses the derived `128 * f64::EPSILON * M^3` budget from the arithmetic
section above. Rigid transforms are explicit column-major Z quarter-turns with bounded dyadic
translation; the independent observation is a sign/swap plus translation applied to the
`quarter_turns_z`/`translation_mm` parameters, never the matrix. Transformed meshes keep their
closed-edge incidence and volume.

### Acceptance

`domain_analytical_mesh_oracles` pins a hand-worked `2 x 3 x 4 = 24 mm^3` box and a
`(3 * 4 / 2) * 5 = 30 mm^3` prism, checks closed incidence, dimension-derived bounds/volume,
sign/swap transform observations and rejection of bad indices, truncated triangle streams,
nonfinite vertices and degenerate triangles. Each malformed witness is rechecked during actual
shrink states.

## D-SCAD-2D

Registry record: `test_name = "proptest_scripted_contour_domains"`,
`target_name = "property_scripted"`, `features = ["slicer-sdk/property_testing_scad"]`,
`requires_scad = true`, `production_symbol = "slicer_sdk::test_support::property_testing::scripted::scripted_cases"`.

### Contract

The property builds one `ScriptedStrategy` per planar topology class
(`Convex`, `Concave`, `NonStarShaped`, `Holed`, `Disconnected`, `Thin`,
`NearDegenerate`, `Empty`) and selects among them with `prop_oneof!`, so a run
reaches every family rather than sampling one. Every exposed value is a
`ScriptedCase` whose recipe was emitted, evaluated through the public
`slicer_scad::evaluate`, and independently validated against its
`GeometryDomain` before it was exposed.

### Bounds

Both records share exactly these `bounds` keys: `max_abs_units`,
`max_vertices`, `max_holes`, `max_components`, `max_shrink_candidates`,
`max_recipe_nodes`, `max_recipe_depth`, `max_facets`, `max_source_bytes`,
`max_evaluations`, `max_scad_vertices`, `max_scad_triangles`,
`evaluation_steps`, `worker_stack_bytes`, `test_process_timeout_ms`. The
geometry keys feed `GeometryBounds`; recipe keys map to their named
`RecipeBounds` fields; adapter options derive `max_ast_nodes`,
`max_ast_depth` and `max_geometry_nodes` conservatively from the recipe bound
(structural estimate, floored by `max_source_bytes`, because a polygon leaf
contributes one AST node per vertex) and convert `max_abs_units` to
`max_abs_coordinate_mm`.

### Oracle

Converted contours are checked with `oracles::validate_geometry` plus exact
point observations of the intended region and exact signed-area agreement for
explicit polygon leaves. The property itself additionally requires:
byte-identical re-emission of the retained source, simple rings, positive area,
and that only an explicitly empty recipe may evaluate to `ScadGeometry::Empty`.
The engine is never the oracle: a successful render is stimulus.

Both halves of every class are exercised: the direct catalog leaf and the
scripted family. Family selection is part of the generated value (it is not a
retry side effect), so `Difference`/`Union`/`Intersection` templates are sampled
during a passing run. `NearDegenerate` reaches an explicit leaf that carries two
independent features — a one-unit non-adjacent clearance and an
almost-collinear vertex (within two canonical units of the line through its
neighbours, but not on it). `scripted_generator_reaches_every_advertised_family`
asserts this reachability, and `scripted_rejects_bounds_invalid_output_and_fatal_adapter_diagnostics`
covers touching converted rings, a collapsed difference, a damaged converted
mesh, and constructor rejection of too-small adapter capacities.

### Shrinking

`domains::GeometryValueTree` drives explicit polygon leaves (reused, not
re-implemented); CSG recipes shrink by dropping optional cuts and union members,
halving dimensions/radii/heights, reducing facet counts and translations, and
simplifying one-child Booleans. A Boolean **cut** is never translated or replaced
by its child: moving a cut can change whether it stays inside the base, which
would turn a shrink proposal into a generator-fatal conversion rejection rather
than an expected domain rejection. Every accepted candidate strictly decreases
`RecipeComplexity`, is re-evaluated and re-certified, and keeps required holes
and components down to their required minimum (the search ends at a fixed
point). Both the per-tree proposal budget (`max_shrink_candidates`) and the
evaluation budget (`max_evaluations`) report `shrink_completed = false` when
reached, and conversion-collapsed candidates are rejected, never exposed.

### Execution

`cargo xtask test --summary --properties --property D-SCAD-2D --tier routine --with-scad`
(routine) and `--tier extended`. Profiles use `isolation = "fork"` with a
positive per-case `timeout_ms` plus the whole-process
`test_process_timeout_ms`. These properties validate helper contracts only and
are not credits toward host or module SUT coverage.

### Failure record

Retained attempt directories keep `recipe.json`, `recipe-bounds.json`,
`source.scad`, `options.json`, `engine.json` and `state.json` from before the
render, then the evaluated geometry/diagnostics/output. Failure metadata names
the property, tier, seed, profile, features, generator version, source state and
engine revision/backend/patch digest.

### Acceptance

`scripted_topology_and_solid_witnesses` proves every class witness (including a
proven empty visibility kernel for `NonStarShaped`, a measured one-unit
clearance for `NearDegenerate`, and an explicit `Empty`), and rejects
deliberately lost holes and bodies. `scripted_value_tree_protocol_and_useful_shrinking`
proves the real `new_tree`/`current`/`simplify`/`complicate` protocol, strict
complexity decrease, optional hole/body removal and required-witness retention.

## D-SCAD-3D

Registry record: `test_name = "proptest_scripted_solid_domains"`, same target,
features, bounds keys and profile shape as `D-SCAD-2D`.

### Contract

A solid `ScriptedDomain` describes the **constituent** planar profile domain.
The bounded solid families build convex rectangular profiles, so any other
constituent topology (or a hole/component minimum above the convex baseline) is
rejected as `DomainError::UnsupportedDomain` before sampling rather than being
silently ignored; both records request `TopologyClass::Convex`.

The property selects among the five `SolidWitness` families
(`ClosedSolid`, `Cavity`, `Bridge`, `Overhang`, `SeparatedBodies`) with
`prop_oneof!`. Each case is a typed `Recipe3d` (box, faceted cylinder,
`linear_extrude` of a planar recipe, translate, quarter turn, union, difference,
intersection) that evaluates to a checked `ScadGeometry::Mesh`.

### Oracle

Beyond `oracles::validate_indexed_mesh` (closed incidence, finite vertices,
index range, nonzero triangle area) and `mesh_signed_volume_mm3`, the reference
model derives an axis-aligned occupancy from the recipe's own dimension and
boundary intervals — independently of the engine and of the kernel — and checks
mesh bounds and volume against it inside a documented `1e-5 * M^3` f32 envelope.
The model then chooses designated probes: an enclosed void centre plus its six
blocking samples, an unsupported-span point plus its support columns, or two
separated body centres plus the gap between them. The **converted mesh** must
agree at exactly those points (inside where the model has material, outside where
it has none), checked by ray casting only against converted vertices. Volume and
bounds agreement alone cannot transfer a cavity, span or body separation to the
output, so it is never accepted as that evidence. These bounded witnesses do not
claim arbitrary self-intersection freedom.

Every `SolidWitness` has more than one construction variant (box, faceted
cylinder, extrusion, quarter turn, intersection, union/difference), and
`scripted_generator_reaches_every_advertised_family` proves they are generated,
not only emitted by fixed tests.

Two observations are deliberately topology-level rather than aggregate:

- a faceted cylinder has no axis-aligned occupancy model, so it is certified
  against its regular-polygon area, radius bounds and per-vertex radius instead;
- separated bodies require the **converted surface** to have more than one
  edge-connected component (vertices welded by position first). Two occupied
  probe points and one empty gap point are not enough: a committed counterexample
  joins two real converted bodies with a thin beam and is rejected even though
  both centres are inside and the gap is empty.

### Shrinking

Solid recipes shrink by deleting optional cuts and union members, halving box
dimensions, cylinder radius/height and extrusion height, reducing facets and
translations, and simplifying one-child Booleans. Certification re-runs after
every candidate, so a shrink that loses the cavity, span or a separated body is
rejected rather than exposed.

### Acceptance

`scripted_topology_and_solid_witnesses` pins controlled `8 mm` and `6 mm` box
scenes with independent mesh point probes: a cavity centre is outside the solid
while the cavity-lost control is inside, a bridge span is solid with nothing
below and pillars on both sides, an overhang has no support under its plate,
and separated bodies are separated by real empty space. Each lost-witness control
is rejected. `scripted_value_tree_protocol_and_useful_shrinking` proves the
witness survives structural shrinking with a non-increasing volume.

## Direct-to-scripted disposition (G-GEOMETRY)

This packet proves the **direct** half of G-GEOMETRY: bounded scalar/config/ID domains, small
graph and IR membership models, exact integer predicates with independent hand-worked
counterexamples, a bounded topology catalog with protocol-correct witness-preserving shrinking,
analytical mesh/transform families and compiled concrete regression fixtures.

Remaining with `pbt-scripted`: OpenRSCAD/recipe/SCAD feature generation, scripted
operations/cavities/bridges and engine-conversion fidelity, the **combined** direct/scripted
generality decision, and any broad arbitrary-polygon generator. If that combined feasibility
result demonstrates a bounded direct-scope gap, the gap is filled there; a general-shrinker
prerequisite is not declared here and SCAD coverage is not claimed.

## H-BOOL-SET

Registry record: `id = "H-BOOL-SET"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_geometry"`, `test_name = "proptest_boolean_set_algebra"`,
`production_symbol = "slicer_core::polygon_ops::union"`,
`production_path = "crates/slicer-core/src/polygon_ops.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-bool-set"`.

### Stable property ID

`H-BOOL-SET` — set algebra for `union`, `intersection`, `difference` and `xor` over bounded
well-wound empty, convex, concave, holed and disconnected sets, plus whole-millimetre rectangle
pairs judged by an exact open-cell truth table.

### Owner

Package `slicer-core`; standalone target `property_geometry`, body
`property_geometry_common::boolean::run_property` in
`crates/slicer-core/tests/property_geometry_common/boolean.rs`. Production surface: all four
wrappers in `crates/slicer-core/src/polygon_ops.rs`, called for every ordered pair. Controls run as
`geometry_controls_boolean_and_clip_mutations` in
`crates/slicer-core/tests/property_geometry_common/controls_boolean_clip.rs`.

### Contract

For every ordered operand pair the four results are consistent as sets: `|A∪B| + |A∩B| = |A| + |B|`
when both operands are rectilinear, `|A\B| = |A| - |A∩B|`, `|A⊕B| = |A∪B| - |A∩B|`, `A\A` and
`A⊕A` are empty, and `A∪A` and `A∩A` reproduce `A` including its holes. Output rings must be valid
and well wound (contour CCW, holes CW). Ring order and ring start are not part of the contract.

### Domain

Generated operands come from #2 `geometry_cases` for `TopologyClass::{Empty, Convex, Concave,
Holed, Disconnected}`; `Thin` and `NearDegenerate` are excluded because this row makes no claim
about lattice-thin geometry. Profiles carry `max_abs_units` 60000/120000, `max_vertices` 32/64,
`max_holes` 4/8, `max_components` 3/4, `max_shrink_candidates` 32/64 and `max_queries` 64/128
(routine/extended). Bounds are checked before any strategy is built: `max_abs_units` in
`20000..=1_000_000`, counts in `1..=64`, `max_queries` at least `2*(max_components+max_holes)+1`.
Invalid or overflowing values are rejected, never narrowed.

### Strategy

Two families. The rectangle family draws two whole-millimetre rectangles whose clamped edges always
stay positive, so a shrink cannot collapse an operand. The general family draws two #2 cases, places
each on a checked offset, and re-validates every realised operand against its tagged class and the
same absolute bound. A translated copy of the first operand supplies an overlapping pair; the
same-set identities supply the disjoint extreme.

### Oracle

The rectangle family is the **complete restricted model**: partition at all input X/Y edges, classify
each open cell's integer midpoint against the Boolean truth table, require every output vertex to lie
on the edge grid, every output edge to be axis-aligned, and the checked `i128` double-area sum to
equal the expected cells. The general family is explicitly bounded: a deterministic integer lattice
at quarter-gaps between distinct vertex coordinates, with points dropped when within 2 units
(Euclidean, exact `i128`) of any input edge, classified with #2 `point_in_region` on the inputs and on
all four outputs. A *witness* point (one per component and per hole) may fall back to a 1-unit margin
when no 2-unit point exists at all — a one-unit catalogue triangle has inradius ≈ 1.72, so the 2-unit
requirement is unsatisfiable there; a point at distance ≥ 1 from the exact input boundary keeps more
than 0.29 units of clearance to the ≤ √½-rounded output boundary, so its classification cannot flip.
No witness at 1 unit remains a hard error, never a skip. Clipper output, self-union, input tags, raw
ring order and single-midpoint containment are never used as the reference.

### Tolerance

Rectangle comparisons are exact integers; Clipper rounds a crossing vertex by at most half a unit per
axis (`√½ ≈ 0.71` Euclidean), which the 2-unit sample margin exceeds, so expected membership at a
sampled cell is exact. General sampling is exact at the sampled points only; no claim is made at
unobserved points.

General outputs may touch themselves. Clipper's `NonZero` result can legitimately **pinch** at a
non-consecutive repeated vertex, so a ring passing through the same point twice is a correct
representation of a self-touching region, not a malformed one. The general family therefore uses a
**weakly-simple** predicate built on the exact integer oracles (`orientation`, `segment_relation`):
adjacent edges must meet at their shared endpoint; non-adjacent edges may be disjoint or touch at a
single point (the pinch); and proper crossings, collinear overlaps of positive length, and
backtracking spikes (`orientation == 0` with a reversing dot product) are rejected. A vertex visited
more than once is additionally checked for angular **interleaving**: if another visit's incident edge
lies strictly between a visit's two edges around that vertex, the strands cross there and the ring is
rejected, which is the figure-8 a per-edge-pair test cannot see. Consecutive duplicates, duplicates
across the closing seam, zero area, fewer than three points and the wrong winding are rejected too.
Strict #2 `validate_ring` is applied only where the expected result is a single simple rectangle.

One limitation is stated rather than implied: a vertex lying on the **interior** of another edge is a
T-touch, which `segment_relation` reports as an endpoint touch, while the interleave rule only
inspects repeated vertices. A crossing that occurs exactly at a T-touch is therefore not rejected
structurally, and the region claim for such a ring rests on the membership oracle alone. Closing it
needs the edges split at every T-touch before the interleave test.

Whether a general region is semantically right is still decided by the membership oracle, not by ring
shape — but ring shape is no longer permissive: tightening this predicate is what exposed the real
`xor` spike defect recorded above, and the regression for that defect now runs in
`crates/slicer-core/tests/polygon_ops_tdd.rs`.

### Shrinking

The #2 value tree keeps each class's witness through `simplify`, and each exposed value is
re-validated against its tagged class. Scale and placement are a deterministic map applied to every
shrunk value, so a shrunk operand never leaves the documented absolute bound.

### Execution

Both tiers run `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets and total generators.
Recorded Windows runs: 32 cases in 8.92 s, and 64 cases in 1.91 s for the clip sibling, at pilot bounds; accepted cases always equal `cases` and rejected test calls are zero. Provenance is in
[proptest-geometry-evidence.md](proptest-geometry-evidence.md).

### Failure record

`PropertyRun` writes the context/case/failure artifacts under `target/property-testing/<run-token>/`
with the tier-bearing reproduction command. Committed regressions are named concrete inputs, never
seeds. The previously failing `xor` pinch input (repeat vertex at `(120, 8)`, where one operand's
second hole meets the notch cut by the other's second component) is pinned by a control arm that
asserts #2 `validate_ring` rejects that ring while the weakly-simple predicate accepts it and the whole
pair check passes; a bowtie and a collinear backtracking spike are pinned as rejections. The production
defect that same tightening exposed — `polygon_ops::xor` emitting a backtracking spike — now has its
own RNG-free regression, `xor_redundant_collinear_vertex_emits_no_backtracking_spike` in
`crates/slicer-core/tests/polygon_ops_tdd.rs`, asserting the exact analytic double area and a
nine-point membership truth table. Deterministic control home:
`geometry_controls_boolean_and_clip_mutations`, which rejects a union returning only operand A and a
difference that fills a hole, with both the rectangle and the general checker.

### Acceptance

Maps to AC-1 and AC-N1. The property must call all four wrappers, both reference families must
produce nonempty observation populations, and the control must reject both named wrong results while
the real SUT passes the same checker.

## H-BOOL-CLIP

Registry record: `id = "H-BOOL-CLIP"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_geometry"`, `test_name = "proptest_clip_polylines_full_segments"`,
`production_symbol = "slicer_core::polygon_ops::clip_polylines"`,
`production_path = "crates/slicer-core/src/polygon_ops.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-bool-clip"`.

### Stable property ID

`H-BOOL-CLIP` — full-path retention for `clip_polylines` over inside, outside, crossing,
boundary-riding, concave and holed clips, with every retained and omitted interval accounted for.

### Owner

Package `slicer-core`; standalone target `property_geometry`, body
`property_geometry_common::clip::run_property` in
`crates/slicer-core/tests/property_geometry_common/clip.rs`. Production surface:
`slicer_core::polygon_ops::clip_polylines`, called once on the whole batch. Controls live in
`geometry_controls_boolean_and_clip_mutations`.

### Contract

Seven documented guarantees of `clip_polylines`: a fully inside polyline is returned whole; a single
crossing is clipped to its inside portion with the new endpoint on the boundary; multiple crossings
yield one sub-polyline per inside span; spans through a hole are removed and split around it; segments
collinear with a clip edge count as inside; multiple inputs are clipped independently and fully
outside inputs are dropped; inputs with fewer than two points are skipped. Output order across paths
is unspecified and never asserted.

### Domain

Scene shapes are a rectangle, an L (concave notch) and a rectangle with a rectangular hole, on a grid
of `max_abs_units/32`. Edge indices keep every feature wider than `2*(INFLATE+BAND)`. Generated
segments are bounded nonzero-length draws: a local axis-aligned family plus #2 `line_segments` wrapped
by a map that turns a zero-length draw into a one-unit segment, so a zero-length segment can never be
exposed before or after shrinking. Profiles carry the six shared keys plus `max_polylines` 12/16
(routine/extended), which must be at least 10 because nine deterministic witness paths are always
present. `max_abs_units` is validated in `512..=1_000_000` and `max_abs_units*max_queries*4` must fit
`i64`.

### Strategy

Each value is a `(scene, paths)` tuple of ranges, so every generated value is admissible. Nine
witness paths are rebuilt from the scene on every case and on every shrunk value: fully inside, fully
outside, crossing once, riding the original bottom edge, through the hole, through the notch, a
three-vertex polyline, a diagonal crossing, and the short/empty pair.

### Oracle

The clip universe is the documented one: `inflate_paths_64(raw, 1.0, Miter, Polygon, 2.0, 0.0)`, which
yields an expanded outer rectangle and a contracted hole (`rect::offset_rectilinear`). Axis-aligned
inputs are partitioned into exact 1D intervals at every input/emitted endpoint and at each
perpendicular crossing `c` and `c ± BAND`; the emitted multiplicity must equal the input coverage
inside, zero outside, and be bounded inside the band. Every emitted vertex must lie within
`offset(R, 3)`. Non-axis-aligned pieces are matched to an input whose line lies within `BAND` of both
endpoints, must satisfy #2 `segment_in_region` against `offset(R, 3)` inclusively, and their input
must be sampled at `t = i/max_queries` so a span that certainly stays `offset(R, -1)` inside is
covered by some matched piece. Result order is never used.

### Tolerance

`BAND = 2` canonical units, matching the documented boundary-endpoint policy, applied to the 1D
breaks, the input-span extension, the vertex allowance and the perpendicular input match. The
perpendicular match was pinned by a two-sided control after an initial one-unit test wrongly rejected
a real 1.41-unit output; see `## Deviations` item 6 of the packet.

### Shrinking

The scene is a tuple of ranges and every element stays valid; the segment vector shrinks by removing
elements, each element stays nonzero-length through its map, and the nine witness paths cannot be
shrunk away because they are rebuilt from the scene.

### Execution

Both tiers run `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets. Recorded Windows runs:
64 cases in 1.91 s at pilot bounds, and a 512-case wide pilot at `max_abs_units = 1_000_000` passed
after the checker fix. Accepted cases always equal `cases`; rejected test calls are zero.

### Failure record

Artifacts follow the shared `PropertyRun` layout. Named replayable witness: the L clip
`(-218750,-218750), (187500,-218750), (187500,0), (0,0), (0,187500), (-218750,187500)` with the path
`[(-55783,146645), (279653,-230017)]`, whose real output is
`[[(-55783,146645),(0,84004)], [(74811,1),(187501,-126539)]]`.

### Acceptance

Maps to AC-2 and AC-N1. The control must reject a chord shortcut through the notch, a span left
inside the notch, an omitted interior span, an endpoint outside the allowance, and the
endpoints-and-midpoint-inside counterexample, while the real SUT passes the same checker.

## H-OFFSET-MITER

Registry record: `id = "H-OFFSET-MITER"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_geometry"`, `test_name = "proptest_offset_rectangular_clearance"`,
`production_symbol = "slicer_core::polygon_ops::offset2_ex"`,
`production_path = "crates/slicer-core/src/polygon_ops.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-offset-miter"`.

### Stable property ID

`H-OFFSET-MITER` — one- and two-pass miter offsets of axis-aligned whole-millimetre rectangles and
rectangular annuli, compared against algebraically expanded bounds and checked double areas.

### Owner

Package `slicer-core`; standalone target `property_geometry`, body
`property_geometry_common::offset::run_property` in
`crates/slicer-core/tests/property_geometry_common/offset.rs`. Primary production surface
`slicer_core::polygon_ops::offset2_ex`; secondary `slicer_core::polygon_ops::offset`. Controls:
`geometry_controls_offset_and_smooth_mutations`.

### Contract

Within the exact rectangular family: a pass with delta `d` moves the contour bounds outward by `d`
and the hole bounds the opposite way; a two-pass sequence applies the same rule twice, so a surviving
pair depends only on `d1 + d2`. A first pass that collapses the shape returns empty. Nothing else is
claimed: no universal two-pass identity, no monotonicity on arbitrary topology, no byte equivalence
with canonical.

### Domain

Rectangles with whole-millimetre sides in `1..=max_side_mm` and rectangular annuli with a band and
hole that keep positive width after **each** pass. Deltas are whole millimetres in
`[-max_offset_mm, max_offset_mm]`, so the f32 and f64 conversions are exact. Profiles carry
`max_abs_units` 120000/200000, `max_side_mm` 8/16 and `max_offset_mm` 3/5 (routine/extended), which
satisfy the placement relation `max_abs_units/10_000 * 2 >= 4*max_offset_mm + max_side_mm`.

### Strategy

A two-branch union. The rectangle branch draws sides and a position, then clamps both cumulative
deltas into the range that keeps every side positive. The annulus branch draws the cumulative deltas
first and then sizes the band and hole from them, so a drawn annulus is guaranteed to admit a
non-degenerate offset. All whole-millimetre values reach canonical units only through checked
conversion.

### Oracle

`require_clearance` proves the per-pass survivor condition before the SUT runs. `stage` then moves
the contour bounds to `[x0-c, x1+c]` and the hole bounds to `[hx0+c, hx1-c]` stage by stage, and a
collapsed first stage yields empty. `check_offset` validates output rings and winding with #2
`validate_ring` and `signed_area2`, requires exactly one component and the expected hole count,
canonicalises the rectangle by dropping repeated and exactly collinear vertices, and compares the
checked `i128` double area. No Clipper call or ring order is part of the reference.

### Tolerance

None — every comparison is an exact integer. Deltas are integers with `|d| <= 2000`, exact in f32 and
f64, so `d * 10_000` is an exact integer well under `2^53`. Axis-aligned unit normals are exactly
`±1/0`, the 90-degree miter scale is exactly `delta`, the 90-degree miter ratio `√2` is below the
limit 2.0, and every cleanup crossing of one horizontal and one vertical integer segment is an
integer point.

### Shrinking

Every invariant is restored by a `prop_map` on each generated and shrunk value, so a shrunk annulus
still admits its clearance. The fixed witnesses sit outside the generator and cannot be shrunk away.
The pass-order witness is deliberately not part of the random family.

### Execution

Both tiers run `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.

### Failure record

Named witnesses include: the one-millimetre square with `-1` then `+2` mm (empty) versus `+2` then
`-1` mm (the `[-1,2]²` survivor); annuli at `(-2,+1)`, `(+1,-1)` and `(+1,0)`; empty input for both
seams; and zero-delta single and two-pass cases. Deterministic control home:
`geometry_controls_offset_and_smooth_mutations`.

### Acceptance

Maps to AC-3 and AC-N1. The control must reject a reversed sign, the swapped pass order, a lost hole
and a hole left unmoved, while the real SUT passes the same checker.

## H-OFFSET-SMOOTH

Registry record: `id = "H-OFFSET-SMOOTH"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_geometry"`, `test_name = "proptest_smooth_outward_safe_families"`,
`production_symbol = "slicer_core::smooth_outward::smooth_outward_polygon"`,
`production_path = "crates/slicer-core/src/smooth_outward.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-offset-smooth"`.

### Stable property ID

`H-OFFSET-SMOOTH` — preservation for convex rings larger than the chord, and the exact 90-degree
chamfer geometry of a controlled L, plus a separate tiny-CW deletion witness.

### Owner

Package `slicer-core`; standalone target `property_geometry`, body
`property_geometry_common::smooth::run_property` in
`crates/slicer-core/tests/property_geometry_common/smooth.rs`. Production surface
`slicer_core::smooth_outward::smooth_outward_polygon`; one witness also uses `smooth_outward_ex`.
Controls: `geometry_controls_offset_and_smooth_mutations`.

### Contract

A convex CCW ring whose every edge is far above the duplicate-prepass epsilon and whose chord is
smaller than every edge comes back geometrically unchanged. A CCW L with one 90-degree reflex corner,
long arms and clearance greater than the chord has that reflex vertex replaced by exactly two chamfer
points, and the other source corners remain. `clip_dist_scaled` is the **chord**, not the per-arm
setback. No universal containment or superset property is claimed.

### Domain

Profiles carry `max_abs_units` 120000/200000, `max_side_units` 40000/80000 and `max_clip_units`
5000/8000 (routine/extended). Every feature is at least `chord + 1000` units, comfortably above the
100-unit duplicate-prepass epsilon, and `max_side_units` is validated against both
`2*(max_clip_units + 1000)` and the 40000-unit witness side.

### Strategy

A two-branch union sharing the chord and the feature-clearance relation. The convex branch draws CCW
rectangles with all four ring starts. The L branch draws leg thicknesses and arm lengths, all four
quarter-turn orientations and all six ring starts.

### Oracle

The convex branch requires validity, CCW winding, the same vertex count, a cyclic rotation of the
input and an identical double area — the ring start is not part of the contract. The L branch
requires the reflex vertex gone, the other five corners present, exactly one added vertex, one new
point strictly inside each arm, a valid CCW ring, a cyclic rotation of the independently built ring,
and a double area equal to the input's plus `k²`. `k` comes from an exact integer search, never from
the SUT.

### Tolerance

For the true setback `s* = chord/√2`, production's f64 path has at most five rounded operations after
an exact `d²/2`, so `|ŝ − s*| <= γ(5)·s*` with `γ(5) ≈ 5.55e-16`. Truncation puts the integer `k` in
`(s* − 1 − δ, s* + δ]`, and since `chord² = 2k²` has no positive integer solution,
`|s* − k| >= 1/(2√2·chord)` — at the 10⁶ ceiling that is `>= 3.5e-7` units against `δ <= 3.9e-10`,
so `k = floor(chord/√2)` exactly. The perpendicular coordinate is exactly zero.

### Shrinking

Every feature stays at least `chord + 1000` through the `prop_map`; turns, ring starts and
translations stay in range. Fixed witnesses sit outside the generator.

### Execution

Both tiers run `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.

### Failure record

Named witnesses: `floor_setback(5000) = 3535`; a 30000×20000 convex rectangle; the L with
20000-unit legs and arms at chord 5000 in all four orientations and six starts; a tiny CW square of
side 2000 at chord 5000, which is deleted; and `smooth_outward_ex` on a 40000 square with that tiny
hole centred, where the hole is dropped and the contour unchanged. `DEV-141`'s far/blocked branch is
never used as a byte oracle and production smoothing is unmodified.

### Acceptance

Maps to AC-3 and AC-N1. The control must reject a setback equal to the chord, a rounded-up setback
`3536`, a missing chamfer and an unchanged tiny CW ring, while the real SUT passes the same checker.

## H-MESH-SLICE

Registry record: `id = "H-MESH-SLICE"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_geometry"`, `test_name = "proptest_slice_analytical_solids"`,
`production_symbol = "slicer_core::triangle_mesh_slicer::slice_mesh_ex"`,
`production_path = "crates/slicer-core/src/triangle_mesh_slicer.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-mesh-slice"`.

### Stable property ID

`H-MESH-SLICE` — axis-aligned boxes and vertical right-triangular prisms sliced at strictly
increasing finite planes, compared against cross-sections derived from the solid's own dimensions.

### Owner

Package `slicer-core`; standalone target `property_geometry`, body
`property_geometry_common::mesh_slice::run_property` in
`crates/slicer-core/tests/property_geometry_common/mesh_slice.rs`. Production surface
`slicer_core::slice_mesh_ex`. Controls: `geometry_controls_slice_and_query_mutations` and
`geometry_controls_vertex_plane_and_hole_witnesses`.

### Contract

A box interior cross-section is the rectangle `[x0,x0+w] × [y0,y0+d]`; a prism interior section is
the XY triangle `(x0,y0), (x0+w,y0), (x0,y0+d)` extruded over `z0..=z0+h`. A plane exactly at the
bottom is empty (horizontal facets are ignored) and a plane exactly at the top yields the full
section (upper-edge ownership). Empty `zs` returns an empty layer vector; an empty mesh yields one
empty layer per plane. Unsorted or duplicate planes, invalid indices and open meshes are outside any
rejection contract and are never fed.

### Domain

Whole-millimetre solids with extents in `2..=max_extent_mm` and origins within
`±max_translation_mm`; profiles carry `max_extent_mm` 8/16, `max_translation_mm` 8/16 and
`max_planes` 6/10 (routine/extended). Planes are whole millimetres generated as a strictly increasing
arithmetic sequence, and the narrowed `f32` vector is re-checked for strict increase before the SUT
runs.

### Strategy

`(box-or-prism, w, d, h, x0, y0, z0, count, base, gap)` mapped to a solid plus its plane vector. A
scene whose planes all fall outside the solid is legitimate and has no nonempty layer; when at least
one layer is expected nonempty, the sampled population must be real.

### Oracle

Because every side face of both solids is a **vertical** plane, an intersection point's XY is exactly
a corner coordinate, so the section is exact on the integer grid. `check_slice` requires the exact
component count, valid rings with contour CCW and holes CW, a checked double area within the derived
envelope, and membership agreement on a 9×9 lattice restricted to points proved clear of the analytic
boundary by exact integer arithmetic (an `L∞` margin for the rectangle and a squared signed-distance
test for the prism hypotenuse). The sample set always also contains one analytically guaranteed
interior point (rectangle centre, prism centroid) and four points eight units outside the bounding box,
because a lattice divided across the longest axis can leave a thin prism with no interior candidate at
all; those points pass through the same exact clearance gates, so they only add checks. `point_in_region`
classifies the analytic section and the emitted output independently. No second `slice_mesh_ex` or
Clipper call is used.

### Tolerance

The box section is exact: axis-aligned output edges are input coordinates, so the double area must
match exactly. The prism has one diagonal edge whose chained vertices pass through an `f32`
millimetre intersection plus integer rounding; moving every vertex of a convex ring by at most one
unit changes the double area by at most `2 × perimeter`, and the perimeter is bounded by `2(w+d)` in
units, so the envelope is `4(w+d)·10_000` squared units (at most `5_120_000`, far below one square
millimetre of double area). The measured deviation on the failing pilot was 10_000 units².

### Shrinking

Extents, origins and plane bases shrink within their ranges and every value stays admissible; the
strictly-increasing property is preserved by construction. The boundary-convention and through-vertex
witnesses are fixed and cannot be shrunk away.

### Execution

Both tiers run `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets. The pilot recorded 16
accepted cases with zero rejected calls.

### Failure record

Named witnesses: the cube bottom (empty) and top (square), each re-asserted as a real SUT result; an
exterior plane; an interior prism plane; and the through-vertex tetrahedron at `z = 0.5` whose
analytic triangle has double area `100_000_000`. Deterministic control home:
`geometry_controls_slice_and_query_mutations`.

### Acceptance

Maps to AC-4 and AC-N1. The control must detect a filled bottom, an empty top, an omitted interior
section and a filled exterior section, while the real SUT passes the same checker.

## H-MESH-QUERY

Registry record: `id = "H-MESH-QUERY"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_geometry"`, `test_name = "proptest_mesh_transform_and_aabb"`,
`production_symbol = "slicer_core::transform_point3"`,
`production_path = "crates/slicer-core/src/lib.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-mesh-query"`.

### Stable property ID

`H-MESH-QUERY` — quarter-turn-plus-translation transforms against an independent sign/swap oracle,
and `AabbTree` bounds and ray hits against the box's own dimensions.

### Owner

Package `slicer-core`; standalone target `property_geometry`, body
`property_geometry_common::mesh_query::run_property` in
`crates/slicer-core/tests/property_geometry_common/mesh_query.rs`. Production surfaces
`slicer_core::transform_point3` and `slicer_core::aabb_tree::{AabbTree, RayHit}`. Controls:
`geometry_controls_slice_and_query_mutations`.

### Contract

An explicit quarter-turn about Z composed with translation is a coordinate sign/swap plus the
translation. `transform_point3` treats an all-zero matrix as identity and performs a perspective
divide by `tw` unless it is zero. `AabbTree::bounds` is the component-wise min/max over surviving
triangles and `None` for an empty tree. `AabbTree::raycast_all_hits` returns hits sorted by distance
then point, with consecutive hits within `1e-5` deduplicated, and `raycast_first_hit` returns the
first. No arbitrary rotation and no direction-normalization promise is made.

### Domain

Profiles carry `max_extent_mm` 8/16, `max_translation_mm` 8/16 and `max_queries` 8/16
(routine/extended). `max_extent_mm` is floored at 4 so a face-interior probe clear of both diagonals
exists. Transform points lie on an eighth-millimetre grid and are exactly representable in `f32`.

### Strategy

A three-branch union: #2 `analytical_meshes(SolidKind::Box, ...)` for a validated bounds-and-volume
case, a locally constructed ray-friendly whole-millimetre box, and a #2 `rigid_transforms` case.

### Oracle

`expected_transform` computes the sign/swap and translation directly in `f64`, never through a matrix
product. Ray probes sit at `(x0+1, y0+2)`, which is provably off both face diagonals:
`x − x0 = y − y0` needs `1 = 2`, and the other diagonal needs `(w−1)(d−2) = 2`, which has no integer
solution for `w, d >= 4`. A `+Z` ray must hit the bottom then the top at parametric distances `1` and
`1+h`; a `−Z` ray mirrors that; a laterally displaced ray must miss; the empty tree must report no
bounds and no hits. The analytical arm additionally compares
`mesh_signed_volume_mm3` against the dimension-derived volume.

### Tolerance

Transforms: one rounded operation (the final narrowing to `f32`), since matrix entries are `0/±1`,
inputs are exact `f32` widened to `f64`, and dyadic translations are exact in `f64`; the envelope is
`γ(1)·magnitude` from `envelopes::f32_abs_error_envelope`. Rays: sixteen rounded operations in the
Möller–Trumbore evaluation, giving `γ(16)` times the probe magnitude. Volume: the documented
`128·f64::EPSILON·M³` mm³ bound. `RayHit::distance` is the parametric `t`, so every ray is unit
length, which is what makes a millimetre claim meaningful.

### Shrinking

Box dimensions shrink within `4..=max_extent_mm`, translations within their range, and transform cases
stay on the dyadic domain. The identity, zero-matrix, empty-tree and quarter-turn witnesses are fixed.

### Execution

Both tiers run `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets. The pilot recorded 16
accepted cases with zero rejected calls.

### Failure record

Deterministic control home: `geometry_controls_slice_and_query_mutations`, which rejects a transposed
transform (a `−90`-degree matrix against the `+90` sign/swap oracle), a missing exit hit (a box with
its top face removed) and a fabricated hit (an interior sheet), while the real fixtures pass the same
checker.

### Acceptance

Maps to AC-5 and AC-N1. The control must reject all three named mutants, explicit zero-matrix and
identity witnesses must be separate assertions, and the inspected tree contains no non-test
`AabbTree::new` caller, so this charter claims a library contract rather than pipeline wiring.

## H-HELPERS-SPLIT

Registry record: `id = "H-HELPERS-SPLIT"`, `package = "slicer-helpers"`, `target_kind = "test"`,
`target_name = "property_helpers"`, `test_name = "proptest_helpers_split_components"`,
`production_symbol = "slicer_helpers::split_connected_components"`,
`production_path = "crates/slicer-helpers/src/split.rs"`,
`features = ["slicer-helpers/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-helpers-split"`.

### Stable property ID

`H-HELPERS-SPLIT` — the connected-component partition of an indexed triangle set, compared against an
independent pairwise face-reachability model over **oriented** face multisets.

### Owner

Package `slicer-helpers`; standalone target `property_helpers`, body
`property_helper_common::split::run_property` in
`crates/slicer-helpers/tests/property_helper_common/split.rs`. Production surface
`slicer_helpers::split_connected_components`. Controls:
`helper_controls_edge_adjacency_and_singleton_mutations` and
`helper_controls_owned_shrink_witnesses`.

### Contract

Adjacency is **index topology only**, and only through a shared **reversed** directed edge: face A
carrying `(a→b)` joins face B carrying `(b→a)`. Vertex-only contact does not join, and a shared edge
traversed in the same direction does not join. Components are seeded in ascending first-face order,
each component's faces appear in ascending original face index, and every component is emitted,
including a single-triangle fragment. Every input face appears in exactly one output component.

### Domain

Profiles carry `max_components` 4/8, `max_faces` 12/24 and `max_abs_mm` 8/16 (routine/extended). A
scene is a non-shrinkable plan (nine `SceneKind` variants naming the contact mode and its component
count) plus shrinkable eighth-millimetre offsets and heights. Parts are laid out on a 2 mm stride with
1 mm extent, so parts can never overlap and positions can never collide for any admitted parameter.

### Strategy

`SceneKind` covers empty input, a single triangle, one closed tetrahedron, two and three disjoint
solids, an opposite-direction shared-edge pair (joins), a same-direction shared-edge pair (stays
separate), a vertex-only shared-index pair (stays separate) and a solid with a lone fragment.

### Oracle

`adjacency_matrix` tests **every unordered face pair with two plain nested loops** over the three
directed edges of each face, then groups faces with a union-find. It never builds production's
`(from, to) → faces` map and never runs production's stack DFS. Faces are compared as oriented
triangles of vertex **positions** (three `f32` bit patterns reduced to the lexicographically smallest
**cyclic rotation**; reversal is never applied), and each output face is resolved through its own
component's local vertex table. `check_split` rejects an incomplete or out-of-range output index, a
wrong component count, a missing/duplicated/foreign face, a non-ascending component first-face order,
and a per-component face multiset that differs from the model.

### Tolerance

None — exact equality throughout. Comparison uses `u32` bit patterns for positions and exact index
equality for topology and order; the eighth-millimetre grid keeps every constructed coordinate exactly
representable in `f32`, and position distinctness is exact bit inequality.

### Shrinking

`SplitSceneValueTree` re-validates every candidate before it becomes `current` and accepts a candidate
only when the documented simplification weight strictly decreases, so the contact mode cannot move and
a lost contact witness becomes a rejection rather than a silent pass. `simplify` drops the offset then
halves the height toward the minimum; `complicate` restores each replaced parameter set from history.

### Execution

Both tiers run `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets. The pilot recorded 64
accepted cases with zero rejected test calls and all nine scene kinds observed in generation.

### Failure record

Named witnesses: `split_keeps_tiny_fragment` and `split_component_counts` in
`crates/slicer-helpers/tests/split_tdd.rs` remain the concrete regressions. Deterministic control
home: `helper_controls_edge_adjacency_and_singleton_mutations`, which rejects a vertex-adjacency
mutant, a same-direction-edge mutant, a dropped singleton, a wrong component order and an
extra/missing face.

### Acceptance

Maps to AC-6 and AC-N1. The oracle asserts the **oriented face partition** only: per-component vertex
compaction and unreferenced-vertex dropping are production implementation choices, not claimed
contract, so no vertex-array or `concat(split(m)) == m` identity is asserted. No repair or decimation
geometry-preservation claim is made, and no malformed-index error contract exists.

## H-BOOL-SCAD

Registry record: `id = "H-BOOL-SCAD"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_geometry_scad"`, `test_name = "proptest_scripted_boolean_membership"`,
`production_symbol = "slicer_core::polygon_ops::union"`,
`production_path = "crates/slicer-core/src/polygon_ops.rs"`,
`features = ["slicer-core/property_testing_scad", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = true`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-bool-scad"`.

### Stable property ID

`H-BOOL-SCAD` — the four Boolean wrappers over **actual engine-converted** SCAD contours paired with a
controlled integer rectangle operand, judged by independent membership.

### Owner

Package `slicer-core`; standalone target `property_geometry_scad`, body
`property_geometry_common::scad_boolean::run_property` in
`crates/slicer-core/tests/property_geometry_common/scad_boolean.rs`. The target is gated on
`slicer-core/property_testing_scad`, and its two modules are mounted behind
`#[cfg(feature = "property_testing_scad")]` so the direct targets never compile the engine. The
adapter and engine belong to `proptest-openrscad_03_scad-adapter` and
`proptest-openrscad_04_pbt-scripted`; this charter uses them as stimulus only.

### Contract

For each certified scripted case, the four operations on `(converted contours, controlled rectangle)`
agree with the Boolean truth table at every accepted sample, and the output rings are structurally
valid with the correct winding. Adapter output is never an oracle: the engine says what the operand
*is*, never what the result *should be*.

### Domain

Three `Union` branches over `TopologyClass::{Convex, Holed, Disconnected}` with
`solid_witness = None`, `BoundaryRule::Strict`, `min_holes = 0` and
`min_components = usize::from(class != Empty)`. Profiles carry the 17 scripted keys; routine is
`max_abs_units` 20000, `max_vertices` 64, `max_holes` 4, `max_components` 4,
`max_shrink_candidates` 32, `max_recipe_nodes` 24, `max_recipe_depth` 6, `max_facets` 16,
`max_source_bytes` 16384, `max_evaluations` 48, `max_scad_vertices` 200000,
`max_scad_triangles` 400000, `evaluation_steps` 5000000, `worker_stack_bytes` 268435456,
`test_process_timeout_ms` 120000, `max_queries` 32, `max_planes` 4. Extended doubles the
coordinates and budgets. Every key is read checked; `max_vertices` is capped at 64 because the
scripted validator rejects more.

### Strategy

`scripted_cases` over the three classes, each value a certified `ScriptedCase` (recipe, exact source
and adapter output) tagged with the class whose branch produced it. The adapter capacity is
re-derived locally because `required_adapter_capacity` is `pub(crate)`: `nodes =
max(8*max_recipe_nodes + 16, max_source_bytes)`, `depth = 2*max_recipe_depth + 8`, geometry nodes
`= nodes`, and `max_abs_coordinate_mm = max_abs_units/10_000`.

### Oracle

#2 `point_in_region` (exact `BigRational`) over a deterministic pool of quarter points between
distinct coordinates, a 12-division lattice and padded outer points, compared against the `BoolOp`
truth table; output rings use the tolerant weakly-simple independent structural check. The controlled rectangle
spans the right quarter of the operand's bounding box and overhangs by at least one millimetre, so a
point beyond `max_x` is provably farther than that overhang from every operand edge and a
rectangle-only population always exists.

### Tolerance

`MARGIN = 2` canonical units. A Clipper intersection vertex moves by at most `√½ ≈ 0.7071` units, so
every accepted sample is more than `2 − 0.7071 ≈ 1.29` units from any output edge; a `Boundary`
classification is a failure, not an ambiguity. No tolerance is widened at runtime.

### Shrinking

The value tree is #4's `ScriptedValueTree`: `simplify`/`complicate` delegate to it, every candidate is
re-emitted, re-evaluated and re-certified, and a candidate is accepted only when `RecipeComplexity`
strictly decreases within the `max_shrink_candidates`/`max_evaluations` budget. The class tag is
fixed per branch, so a shrunk case cannot lose its topology witness, and `check_case` re-emits
`case.source` and compares it byte for byte. Shrink completion is reported `null`/unknown when no
failure occurs.

### Execution

Routine and extended both run `isolation = "fork"` with nonzero `timeout_ms`, and
`bounds.test_process_timeout_ms` bounds the whole libtest child, so recipe generation and shrink
rendering are supervised and not only callbacks. The green pilot recorded 8 accepted cases, 0
rejected calls, `shrink_completed = null`, 2.14 s, with controller and worker pids journaled.

### Failure record

Artifacts are written before evaluation: `recipe.json`, `recipe-bounds.json`, `source.scad`,
`options.json`, `engine.json`, `state.json`, then `geometry.json`, `diagnostics.json`,
`output.json`. Engine provenance recorded by the pilot: backend `manifold_cpp`, engine revision
`1a45bd33f2886c32bda32fe6e8358a67ab03f5ef`, patch digest `c0297079…`.

### Acceptance

Maps to AC-8. A removed hole, a dropped body or an all-outside answer must be detected. The sample
vector is built as **every component witness, then every hole witness, in operand order**, followed by
four mandatory class populations and only then the strided fill; an over-cap selection is an explicit
error naming forced versus population counts, never truncation. This ordering is load-bearing and was
measured: with the old tail-first ordering restored, a needle-shaped component placed in the gap
between two bodies was **accepted** (`accepted: 32, material_witnesses: 3`); with the fix the same
mutant is rejected at sample `(6252, 2)`.

Deterministic reject-arms run inside the target, each paired with the correct result passing the same
checker: `filled-hole/<Op>` (holes cleared on every operation that has one — the `difference` output
of this witness has none, because the rectangle's left edge coincides with the hole's right edge, so
the arm mutates whichever operation actually carries a hole and names it), `all-outside` (all four
outputs empty), `dropped-body` and `dropped-unseen-component`. A rejection only counts when it is a
membership-classification message, so an unrelated fault cannot make an arm pass.

**Classification, not silent skipping:** a converted component whose material contains no integer
point at the 2-unit margin is enumerated exhaustively over its integer bounding box (bounded at
65 536 points) and counted as `proved_unobservable`. Per-case populations are asserted nonempty for
`outside_both` and `inside_rectangle_only` always, and for `inside_operand` only when the case has at
least one observable component — the charter previously overstated this as unconditional. A
**run-level** counter proves that at least one checked case observed a real operand component; because
fork-isolated callbacks cannot update controller memory, the counter is mirrored to a one-line
`scad-boolean-observability.txt` in the run's artifact directory and the module panics after
`run.check` if it is zero. Both directions were verified: the positive path reports
`7 of 8 checked case(s) observed at least one operand component`, and a forced-negative experiment
panicked with `run-level non-vacuity: none of the 8 checked case(s) observed a single operand
component`.

Measured in the pilot: 4 of 8 cases had at least one unobservable component and 1 case had all
components unobservable while 21 rectangle-only and 11 outside-both samples were still checked against
all four operations. This is #4's catalog scale, not a production defect.

## H-MESH-SCAD

Registry record: `id = "H-MESH-SCAD"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_geometry_scad"`, `test_name = "proptest_scripted_mesh_cross_sections"`,
`production_symbol = "slicer_core::triangle_mesh_slicer::slice_mesh_ex"`,
`production_path = "crates/slicer-core/src/triangle_mesh_slicer.rs"`,
`features = ["slicer-core/property_testing_scad", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = true`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-mesh-scad"`.

### Stable property ID

`H-MESH-SCAD` — `slice_mesh_ex` over **actual converted scripted solids**, compared against a local
ray-parity model and an independent section-topology model.

### Owner

Package `slicer-core`; standalone target `property_geometry_scad`, body
`property_geometry_common::scad_mesh::run_property` in
`crates/slicer-core/tests/property_geometry_common/scad_mesh.rs`.

### Contract

For every certified solid case and every probed plane, the emitted layer's component count, hole
count, winding and total double area match the independent model, and every accepted sample's
membership matches the ray-parity reference.

### Domain

A five-branch `Union` over `SolidWitness::{ClosedSolid, Cavity, Bridge, Overhang, SeparatedBodies}`.
`scripted_cases` requires a solid domain to be `TopologyClass::Convex` with `min_holes = 0` and
`min_components <= 1`, so the solid structure is carried by `solid_witness` and not by the planar
class. Preconditions per case: `ScadGeometry::Mesh`, #2 `validate_indexed_mesh(..., true)` and a
positive `mesh_signed_volume_mm3`. Profiles are the same 17 keys; `max_planes` must be at least 2 and
`max_queries` at least 4.

### Strategy

`scripted_cases` per witness with a fixed tag. Planes are the midpoints of the largest distinct
vertex-Z gaps, re-verified finite, distinct, strictly increasing and clear of every vertex Z, so no
plane is coplanar or through a vertex.

### Oracle

A local ray-parity model over the **input triangles only**: a vertical `+Z` ray counts triangles that
strictly contain the query in projected XY and whose plane is crossed strictly above `z`; odd means
inside. Section topology comes from union-find over mesh-edge identities, which yields the exact
number of closed section curves, and each segment is oriented from the outward normal so per-curve
signed double areas classify outer curves versus holes. `AabbTree`, `slice_mesh_ex`, Clipper, the
SDK's solid reference and the slicer's chaining algorithm are all excluded from the reference.

### Tolerance

`EDGE_CLEARANCE_UNITS = 8` from every projected triangle edge; `SECTION_CLEARANCE_UNITS = 8` from every
model section segment, which exceeds the ≤1-unit per-vertex grid rounding; `COPLANAR_CLEARANCE_MM =
1e-6` for near-coplanar triangles, which are excluded and counted, never resolved; and
`PLANE_VERTEX_CLEARANCE_MM = 1e-5` from every vertex Z. Ambiguity is never resolved by widening:
excluded samples are reported in `PlaneStats::excluded`. The area envelope is derived as
`2 × perimeter_units`, since moving each vertex of a closed curve by at most one unit moves its double
area by at most that.

### Shrinking

Identical to `H-BOOL-SCAD`: delegated `ScriptedValueTree`, per-candidate re-emission and
re-certification, strictly decreasing `RecipeComplexity`, bounded candidate and evaluation budgets,
and a fixed witness tag per branch.

### Execution

Fork isolation with nonzero `timeout_ms` and whole-process `test_process_timeout_ms` supervision. The
green pilot recorded 8 accepted cases, 0 rejected calls, `shrink_completed = null`, 0.30 s, with both
pids journaled.

### Failure record

Measured deterministic witnesses, each through the real adapter: `ClosedSolid` (1 curve, 16 inside +
16 outside samples), `Cavity` (1 curve, then 2 curves forming 1 component with 1 hole, then 1 curve),
`Bridge` (2 curves / 2 components, then 1), `Overhang` (1 curve per plane) and `SeparatedBodies`
(2 curves / 2 components). Witness-implied identities are asserted on the independent model and the
SUT layer must match it, so a vanished cavity, a dropped body or a filled void each breaks a named
identity plus the probe samples.

### Acceptance

Maps to AC-8. A filled cavity/hole, a missing body and a fake all-outside answer are all detectable:
the component and hole counts are model-derived per plane and the probe pairs require both an
interior and a confirmed exterior sample for every nonempty witness — a plane with only one sample
side is an **error**, not a classification, so the mesh family has no unobservable case to count.

Deterministic reject-arms run inside the target, each paired with the correct layer passing the same
checker: `dropped-body z=…` removes one component from the `SeparatedBodies` layer, and
`filled-cavity z=…` clears the holes on the `Cavity` plane that shows exactly one. Each arm's rejection
must name `component` or `hole` respectively, so an unrelated fault cannot make the arm pass.

Documented limitations: `f64` parity over `f32` triangles with the stated clearances; ambiguous hits
are excluded and counted, not resolved; through-vertex and exact top/bottom regimes belong to direct
`H-MESH-SLICE`; parity proves membership, not boundary location; a consistently outward-oriented
closed surface is required. The clearances are **proved**, not assumed: with `M = 100` mm (the
`max_abs_units` cap), `u = 2⁻⁵³` and 15 rounded operations, the barycentric crossing height carries
`γ₁₅·3M ≈ 5e-13` mm and the edge distance `γ₁₅·2√2M ≈ 4.7e-13` mm (`≈ 4.7e-9` units), so the 8-unit
edge clearance exceeds the rounding by about `1.7e9`, and the section clearance exceeds the
1-unit grid rounding plus `1.3e-9` units by about `8×`.



