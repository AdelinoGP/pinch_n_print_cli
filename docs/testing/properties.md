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
|units_to_mm(n) - n/10_000| ≤ 2^-24·|n/10_000| + 2^-150  (millimetres)
```

The reference arithmetic is f64, not real arithmetic; the `2^-150` terms absorb the underflow
floor and no additional reference error is silently ignored. `check_mm_case` additionally
asserts exact agreement between `slicer_ir::mm_to_units`, `slicer_sdk::coords::mm_to_units`
and `Point2::from_mm` — explicitly **consistency evidence only**, not correctness evidence,
because all three share the same implementation shape. Correctness rests on the analytical
envelopes and the half-case derivation. `expected_half_away` re-derives the tie rule from
`10_000·m` in f64 and half-away-from-zero rounding, so a truncating implementation is
distinguishable from a rounding one.

### Tolerance

The tolerance is derived, never tuned: `1/2 + 2^-24·|t| + 2^-150` units and
`2^-24·|n|/10_000 + 2^-150` mm. The mutation controls in
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

`declaration_sets(max_declarations, max_abs_bound)` and
`bound_values(max_abs_bound, max_list_len)` from
`crates/slicer-scheduler/tests/common/property_config_oracles.rs`, unioned with
`nonfinite_values()`. Mandatory deterministic witnesses run before sampling: absent key → `Ok`;
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

Shrinks reduce declaration count, simplify bounds toward zero, and shorten lists. Two things are
pinned and tested:

- the **value family** — scalar, list, or non-finite — is pinned by
  `property_config_oracles::mode_locked`, so a list value cannot shrink into a scalar (dropping
  the first-invalid-index shape) and a non-finite value cannot shrink into a finite one;
- every shrunk value stays inside `max_abs_bound` and every list inside `max_list_len`.

The accept/reject **verdict is deliberately not pinned**: declarations and values shrink
independently, so a shrink can move a case across an inclusive edge. The per-case oracle
re-derives the expected verdict from the current declarations and value, and the property
asserts that agreement, so no correctness claim depends on the verdict being preserved. The
first-invalid-index relationship is likewise re-derived per case, never assumed stable under
shrink. Shrink limits are profile-bounded.

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
