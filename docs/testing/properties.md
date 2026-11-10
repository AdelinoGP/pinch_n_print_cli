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
