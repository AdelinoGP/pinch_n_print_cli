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

The six Arachne records added by packet `proptest-openrscad_10_pbt-arachne` use the same standard
profile shape: `cases` 64/1024, zero rejection budgets, `max_default_size_range` 32/256,
`max_shrink_iters` 64/256, chacha, `in_process`, `timeout_ms = 0`. Their family-specific bounds
are:

- `max_beads`/`max_width_units`/`max_excess_units` 8/20000/8000 → 16/40000/16000 for
  `H-ARACHNE-BEADING`;
- the five geometry keys 400000/24/2/3/64 → 1000000/32/3/4/256 for `H-ARACHNE-GRAPH`;
- the same five keys plus `max_beads` 10 → 16 for `H-ARACHNE-PIPELINE`;
- `max_inset_idx` 8 → 32;
- `max_junctions`/`max_abs_mm_units`/`max_spacing_units`/`max_height_units` 32/20000/2000/500 →
  64/40000/8000/2000;
- `max_walls` 6 → 12.

Measured on the Windows host (rustc 1.96.0, `dev` profile), per-record libtest walls in registry
order (BEADING, GRAPH, PIPELINE, CLASSIFY, BUILD, SEQUENCE):

| Lane | Total | Per-record walls |
|---|---:|---|
| Six-record routine selection | 19.3 s | 0.02 / 0.22 / 0.23 / 0.03 / 0.33 / 0.03 s |
| Six-record extended selection | 35.7 s | 0.28 / 5.32 / 5.39 / 0.24 / 5.90 / 0.75 s |
| Combined incremental routine (32 records) | 2 m 35.0 s | 0.02 / 0.25 / 0.24 / 0.02 / 0.33 / 0.04 s |
| Combined incremental extended (32 records) | 10 m 10.9 s | 0.31 / 5.61 / 5.64 / 0.31 / 5.18 / 0.84 s |

Every lane passed: the routine selection and both combined lanes reported `VERDICT: PASS`, and in
the extended selection every record's end event reported `outcome = passed`. The combined-lane
totals are warm-cache executions with all 32 record binaries already linked; the per-record walls
are unaffected by that. In all lanes accepted
cases equalled `cases`, rejects were 0 and `call_counts_complete = true`. The combined lanes cover 32 records because the four
`requires_scad` records are skipped without `--with-scad`. The retained-seed replay
`--property H-ARACHNE-BEADING --tier routine --seed 14268583537219271429` passed 64/64 in a 15.5 s
total.

Logs are archived under `target/proptest-arachne-evidence/` and per-record contexts under
`target/property-testing/<run-token>/<ID>/context.json`. Provenance, populations, controls, the
canonical audit and defects are in [proptest-arachne-evidence.md](proptest-arachne-evidence.md).
The hosted `properties-windows` job ran head `6b340919` in run `44434672409` and passed, including
the assertion that all six Arachne IDs executed (see
[proptest-arachne-evidence.md](proptest-arachne-evidence.md#measurements)).

### I/O, G-code and module-kernel pilots (packet `proptest-openrscad_16`)

Measured on the Windows development machine (rustc 1.96.0, `dev` profile, workspace `target/`,
Proptest 1.11, warm binaries). Each record ran through the registry executor; per-record walls are the
libtest times from the combined selections.

| ID | routine cases / wall | extended cases / wall |
| --- | --- | --- |
| `H-IO-ID` | 64 / 0.04 s | 256 / 0.11 s |
| `H-IO-EXTENT` | 64 / 0.04 s | 256 / 0.10 s |
| `H-IO-PLACEMENT` | 64 / 0.03 s | 256 / 0.11 s |
| `H-GCODE-FEEDRATE` | 64 / 0.02 s | 256 / 0.08 s |
| `H-GCODE-EMIT` | 64 / 0.04 s | 256 / 0.17 s |
| `H-GCODE-SERIALIZE` | 64 / 0.09 s | 256 / 0.18 s |
| `H-GCODE-TEMPLATE` | 64 / 0.07 s | 256 / 0.17 s |
| `H-COOLING` | 64 / 0.04 s | 256 / 0.08 s |
| `H-OVERHANG-SPEED` | 64 / 0.05 s | 256 / 0.34 s |
| `H-SKIRT-BRIM` | 64 / 0.51 s | 256 / 1.60 s |
| `H-WIPE-TOWER` | 64 / 0.05 s | 256 / 0.09 s |

The 11-record combined selection took 36 s (routine) and 32 s (extended) warm wall. Every record
reported `accepted_cases` equal to its `cases`, `rejected_test_calls = 0`, zero local/global rejects,
`call_counts_complete = true` and `outcome = passed`; `shrink_completed` is null because Proptest 1.11
cannot report completion. Profiles: 64/32/64 and 256/128/256, zero rejection/regeneration budgets,
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`.
Retained-seed replay `--property H-SKIRT-BRIM --tier routine --seed 4285591042002023685` passed 64/64.
The 1/8 mm thin-tail case in `H-SKIRT-BRIM` was found by the extended tier and is recorded in
[proptest-io-gcode-evidence.md](proptest-io-gcode-evidence.md) §7 and §10.

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

A vertex lying on the **interior** of another edge is a T-touch, which `segment_relation` reports as
an endpoint touch, so the pairwise test cannot tell a touching T from a crossing one. Before the
interleave test the ring is therefore split at every T-touch (each ring vertex strictly inside an
edge, by exact `orientation` and `i128` betweenness, is inserted into that edge in order), which turns
the meeting point into a repeated vertex judged by the same interleave rule. Every meeting point of
two non-adjacent edges is then a proper crossing, a collinear overlap or a repeated vertex of the split
ring, so the predicate rejects every transverse self-intersection structurally. Control arms in
`controls_boolean_clip.rs` pin both directions: a T-touch figure-8 (lobes of double area `+10` and
`-6`, total `+4`) is rejected as a vertex crossing, and a square notched down to a single point on its
bottom edge (a T-touch pinch) is accepted.

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

Seeded-fault minimisation: `geometry_controls_seeded_fault_is_minimised` runs this exact body
(`check_input_with`) and strategy through `PropertyRun` at both registry tiers. It reads the
profiles from `test-properties.toml` and runs them against a `union` that drops holes. From the
retained `failure.json` and `observations.jsonl` it asserts a `sut_or_contract` failure naming
`Union`, at least one accepted shrink simplification, and a reported counterexample that a callback
actually failed on and that differs from the initial failure. At the extended tier it also asserts
that shrinking stopped below `max_shrink_iters`, which makes the result a local minimum. The routine
budget is measured to truncate this shrink; see `docs/testing/proptest-geometry-evidence.md`,
"Post-closure follow-ups".

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
`filled-cavity z=…` clears the holes on the `Cavity` plane that shows exactly one, and
`all-outside <witness> z=…` replaces the layer of every probed plane of all five witnesses with the
empty answer. Each arm's rejection must name `component`, `hole` or `all-outside` respectively, so an
unrelated fault cannot make the arm pass.

Documented limitations: `f64` parity over `f32` triangles with the stated clearances; ambiguous hits
are excluded and counted, not resolved; through-vertex and exact top/bottom regimes belong to direct
`H-MESH-SLICE`; parity proves membership, not boundary location; a consistently outward-oriented
closed surface is required. The clearances are **proved**, not assumed: with `M = 100` mm (the
`max_abs_units` cap), `u = 2⁻⁵³` and 15 rounded operations, the barycentric crossing height carries
`γ₁₅·3M ≈ 5e-13` mm and the edge distance `γ₁₅·2√2M ≈ 4.7e-13` mm (`≈ 4.7e-9` units), so the 8-unit
edge clearance exceeds the rounding by about `1.7e9`, and the section clearance exceeds the
1-unit grid rounding plus `1.3e-9` units by about `8×`.

## H-ARACHNE-BEADING

Registry record: `id = "H-ARACHNE-BEADING"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_arachne"`, `test_name = "proptest_arachne_beading_relations"`,
`production_symbol = "slicer_core::beading::distributed::DistributedBeadingStrategy::compute"`,
`production_path = "crates/slicer-core/src/beading/distributed.rs"`,
`features = ["slicer-core/host-algos", "slicer-core/property_testing"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-arachne-beading"`.

### Stable property ID

`H-ARACHNE-BEADING` checks analytic bead and sentinel relations for the three real beading
strategies (distributed, limited, widening) over bounded dyadic inputs.

### Owner

- **Package and target.** Package `slicer-core`; standalone target `property_arachne`, gated by
  `#![cfg(feature = "host-algos")]` and `required-features`.
- **Body.** `property_arachne_common::beading::run_property` in
  `crates/slicer-core/tests/property_arachne_common/beading.rs`.
- **Production surface.**
  - `DistributedBeadingStrategy::{new,compute}` (`crates/slicer-core/src/beading/distributed.rs`)
  - `LimitedBeadingStrategy::{new,compute}` (`crates/slicer-core/src/beading/limited.rs`)
  - `WideningBeadingStrategy::{new,compute}` (`crates/slicer-core/src/beading/widening.rs`)

  All three are called through `BeadingStrategy`.
- **Controls and regressions.** These live in `property_arachne_contracts`
  (`property_arachne_common::controls_beading`).

### Contract

**Distributed**, with bead width `w` and thickness `t`:

| Bead count | Expected result |
|---|---|
| 0 | both arrays empty, `left_over = t` |
| 1 | one bead of width `t` at `t/2` |
| 2 | two beads of `t/2` at `t/4` and `3t/4` |
| `n >= 3`, `t = n*w` | every width `w`, centres `(i + 1/2)w`, `left_over = 0` |

**Limited**, with even cap `2k` over a distributed parent:

- **At the cap** (`t = 2k*w`): `2k` real beads of width `w` at `(i + 1/2)w`, one zero-width
  sentinel at `k*w`, `left_over = 0`.
- **Over the cap** (requested `2k + 1`, `t = 2k*w + excess`):
  - the real beads keep width `w`;
  - the left-half centres stay `(i + 1/2)w` and the right-half centres mirror them about `t/2`;
  - two sentinels sit at `k*w` and `t - k*w`;
  - `left_over = excess`.

**Widening**, using canonical `WideningBeadingStrategy::compute`'s gate
`bead_count <= 1 && thickness < getTransitionThickness(1)`:

- below `min_input_width`: no beads, `left_over = t`;
- inside the gate window: one bead of `max(t, min_output_width)` centred at `t/2`;
- `bead_count >= 2`: the parent's own result, even when `t < optimal_width`.

In every case `bead_widths` and `toolpath_locations` are parallel. `bead_widths.len()` is not
required to equal the requested count.

### Domain

Bounds and their validation:

- Routine `max_beads`/`max_width_units`/`max_excess_units` = 8/20000/8000; extended
  16/40000/16000.
- `validate_limits` requires `max_beads >= 3`, an admissible even cap from `witness_cap`, and
  `(max_beads + 2)*max_width_units + max_excess_units <= 2^53`. That last condition makes every
  `n*w`, `2k*w` and `2k*w + excess` exact in binary64.
- Lengths are dyadic: `units / 2^shift`, with `shift <= 3`.
- The distribution count is in `1..=4`. Zero and out-of-range bounds are rejected before sampling.

### Strategy

One `BeadingCase` carries the selectors for **every** branch: a dyadic width, a distributed
`n >= 3` offset, a free thickness for counts 0–2, a distribution count, `cap_half`, an over-cap
excess and two widening selectors. `check_case` runs every branch on every case, so no branch's
coverage depends on which values the RNG happens to sample.

### Oracle

The oracle is independent closed-form arithmetic derived from `w`, `t`, `k` and `excess`. It never
reads back SUT output. Each observed population is compared against its derived size: one bead, two
beads, `n` beads, or `2k` real beads plus 1 or 2 markers. A branch therefore cannot be "covered" by
an empty result.

Widening conservation (`sum(widths) = t - left_over`) is **deliberately not asserted**: the clamped
single bead can be wider than the region. `check_conservation` is applied only where conservation
is valid, and a control proves it must fail on a real widening output.

### Tolerance

The bound is Higham's `gamma(m)` with round-to-nearest binary64: `|computed - exact| <= gamma(m) *
magnitude + gamma(m) * |expected|`, implemented as `Envelope::from_operations`. The operation
budgets are counted from `compute`'s statements and rounded up in `Envelopes::derive`: `max_beads +
8` for widths and `6*max_beads + 8` for locations. At the extended bounds this gives about 2.0e-9
units for widths and 8.0e-9 units for locations, both far below one canonical unit. The envelope is
a derived bound, not a tuned epsilon.

### Shrinking

Every field shrinks inside its own validated range. Each range's lower bound still admits its
branch, and the branch selectors ride in every case. A shrunk case therefore keeps all branches and
their boundary witnesses.

### Execution

Both tiers run `in_process` with chacha, `timeout_ms = 0` and zero rejection budgets. Measured on
Windows:

| Lane | Accepted | libtest wall |
|---|---:|---:|
| Routine selection | 64/64 | 0.02 s |
| Extended selection | 1024/1024 | 0.28 s |
| Retained-seed replay `--seed 14268583537219271429` | 64/64 | 0.03 s |

Details are in [proptest-arachne-evidence.md](proptest-arachne-evidence.md).

### Failure record

`PropertyRun` artifacts are written under `target/property-testing/<run-token>/H-ARACHNE-BEADING/`.
Committed regressions are named concrete inputs, never seeds:

- `widening_two_requested_below_optimal_delegates_to_parent`: `w = 4000`, `t = 3000`, count 2.
  Pre-fix it returned `[3400.0]`; it now expects `[1500.0, 1500.0]`.
- `widening_transition_propagation_preserves_two_bead_endpoint`: the real factory and population
  caller. It expects widths `[2450, 1500]` and locations `[1125, 2250]`.
- `limited_transition_thickness_at_cap_matches_canonical_floor`: cap 4. Pre-fix it returned
  `18000.0`; canonical gives `19900.0`.

### Acceptance

Maps to AC-1, AC-N1 and AC-N2. `arachne_beading_oracle_controls` rejects a lost sentinel pair, a
non-parallel width/location pair, and conservation applied to widening, while the real strategies
pass the same comparator. `arachne_property_domain_rejections` rejects unusable bounds before
sampling.

## H-ARACHNE-GRAPH

Registry record: `id = "H-ARACHNE-GRAPH"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_arachne"`, `test_name = "proptest_arachne_graph_topology"`,
`production_symbol = "slicer_core::skeletal_trapezoidation::SkeletalTrapezoidationGraph::from_polygons"`,
`production_path = "crates/slicer-core/src/skeletal_trapezoidation/graph.rs"`,
`features = ["slicer-core/host-algos", "slicer-core/property_testing"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-arachne-graph"`.

### Stable property ID

`H-ARACHNE-GRAPH` checks the index and reciprocity relations of graphs built by the real
`SkeletalTrapezoidationGraph::from_polygons`, over independently validated outlines.

### Owner

Package `slicer-core`; target `property_arachne`. The body is
`property_arachne_common::graph::run_property` in
`crates/slicer-core/tests/property_arachne_common/graph.rs`. The controls are
`arachne_graph_oracle_controls_and_input_errors` (`controls_graph_pipeline.rs`).

### Contract

For every constructed graph:

- every `start_vertex` is `NO_INDEX` or `< vertices.len()`;
- every non-sentinel `twin`/`next`/`prev` is `< edges.len()`;
- `twin.twin`, `next.prev` and `prev.next` return the edge whenever the link exists;
- a twin's `start_vertex` resolves the edge's endpoint;
- `beading_propagation` and `beading_dist_to_bottom_source` are vertex-parallel at construction
  stage;
- finite, nonnegative endpoint radii satisfy `0 <= r_min <= r_max`.

NaN `distance_to_boundary` values are allowed sentinels. Universal twin presence is not claimed.

### Domain

- **Inputs.** #2 `geometry_cases`, scaled by the exact integer `SCALE = 2500` (1 catalog unit =
  0.25 mm) and re-validated with `validate_geometry` against the realized domain.
- **Bounds.** Routine `max_abs_units`/`max_vertices`/`max_holes`/`max_components`/
  `max_shrink_candidates` = 400000/24/2/3/64; extended 1000000/32/3/4/256. Minimums: 100000, 8,
  1, 2 and 1.
- **Classes.** `CAMPAIGN_CLASSES` samples `Empty`, `Convex`, `Concave`, `Holed`, `Thin` and
  `Disconnected`.
  - `NonStarShaped` is not sampled.
  - `NearDegenerate` is excluded by derivation: its one-catalog-unit clearance witness does not
    survive the exact ×2500 map.

### Strategy

The strategy draws one tagged, scaled catalog case per value. An extra branch generates 4–8 mm
origin-anchored rectangles directly, so the shared guaranteed-output class is non-vacuous by
construction. The deterministic rectangle, holed and concave witnesses run first.

### Oracle

`check_relations` indexes `vertices` and `edges` itself and never calls the production compactor
or a mirrored pipeline graph. Each class has a declared outcome that is checked, not assumed:

- `Empty` must return `Err(SktError::EmptyInput)`.
- Every other sampled class must return `Ok` with nonempty vertex and edge populations.

Measured over 400 cases:

| Class | Cases | Outcome |
|---|---:|---|
| `Empty` | 74 | `Err(EmptyInput)` |
| `Convex` | 68 | `Ok`, nonempty |
| `Concave` | 70 | `Ok`, nonempty |
| `Holed` | 49 | `Ok`, nonempty |
| `Thin` | 70 | `Ok`, nonempty |
| `Disconnected` | 69 | `Ok`, nonempty |

### Tolerance

Index and reciprocity checks are exact. The radius ordering is checked only where both radii are
finite and nonnegative.

### Shrinking

The #2 value tree keeps each class's witness through `simplify`. Scaling is an exact integer map
applied to every shrunk value, so a shrunk case never leaves its tagged class or the absolute bound.

### Execution

Measured libtest walls on Windows: 0.22 s for the routine selection (64/64) and 5.32 s for the
extended selection (1024/1024), with zero rejects.

### Failure record

Artifacts are written under `target/property-testing/<run-token>/H-ARACHNE-GRAPH/`. The
deterministic validation-boundary examples are:

- empty input → `SktError::EmptyInput`;
- a zero-point ring and a two-point ring → `SktError::DegeneratePolygon`.

These are boundary examples, not a promise that arbitrary invalid rings are diagnosed.

### Acceptance

Maps to AC-2. On test-owned copies of a real rectangle graph, `check_relations` rejects broken twin
reciprocity, an out-of-range `next`, an out-of-range `start_vertex`, and a truncated
`beading_propagation` or `beading_dist_to_bottom_source`. The unmutated graph still passes.

## H-ARACHNE-PIPELINE

Registry record: `id = "H-ARACHNE-PIPELINE"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_arachne"`, `test_name = "proptest_arachne_pipeline_structure"`,
`production_symbol = "slicer_core::arachne::pipeline::run_arachne_pipeline"`,
`production_path = "crates/slicer-core/src/arachne/pipeline.rs"`,
`features = ["slicer-core/host-algos", "slicer-core/property_testing"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-arachne-pipeline"`.

### Stable property ID

`H-ARACHNE-PIPELINE` checks the structure of real `run_arachne_pipeline` output. It separates
printable lines from inner-contour markers and declares an outcome mode for each case.

### Owner

Package `slicer-core`; target `property_arachne`. The body is
`property_arachne_common::pipeline::run_property` in
`crates/slicer-core/tests/property_arachne_common/pipeline.rs`. The controls are
`arachne_pipeline_oracle_controls` and `arachne_local_maximum_pre_stitch_is_open`
(`controls_graph_pipeline.rs`).

### Contract

The pipeline returns `(printable_lines, inner_contour)`. Each printable line has:

- nonempty junctions;
- finite coordinates and widths, bounded by the profile extent plus the constructor's clamp margin;
- a first-junction width `> 0`, with later widths finite and `>= 0`;
- its `inset_idx` carried by at least one endpoint junction (`IndexRule::PipelineEndpoints`).

Every inner-contour line has a first width of exactly `0.0`.

Closure equality (`is_closed` ⇒ the endpoint XY repeats) is asserted only in the explicit-repeat
domain (`ClosedDomain::ExplicitRepeat`). Legitimate open paths are preserved and closure is never
manufactured. Containment, a fixed wall count, a universal width cap and rotation invariance are
not asserted.

### Domain

- **Geometry.** The same five geometry bounds and catalog as `H-ARACHNE-GRAPH`, plus `max_beads`
  (routine 10, extended 16, minimum 10, which is the `ArachneParams::default` cap).
- **Parameters.** Built by FRU from `ArachneParams::default`, varying:
  - an even `max_bead_count` in `2..=max_beads`;
  - `distribution_count` in `1..=3`;
  - `wall_transition_length` of 0.2, 0.4 or 0.8 mm;
  - `print_thin_walls` and `is_initial_layer`, both values each.

### Strategy

Outcome modes:

- A case is `OutcomeMode::Guaranteed` when all of these hold:
  - its class is `Convex`;
  - every component's bounding box is at least 3 mm on both axes (`ROBUST_EXTENT_UNITS`);
  - every ring edge is at least 0.5 mm (`ROBUST_EDGE_UNITS`).

  A guaranteed case must return `Ok` with nonempty printable lines.
- Every other case may return `Ok` (possibly empty) or `Err(EmptyAfterPreprocess)`. Any other
  error is a failure.

Four deterministic witnesses run first, each under a fresh `CaseGuard`:

| Witness | Measured outcome |
|---|---|
| 10 mm square | `Ok`; 5 closed printable lines plus 1 inner-contour marker; all index-uniform; closed lines repeat their endpoint |
| tapered wedge | `Ok`; 3 closed printable lines with differing widths |
| 8 × 0.3 mm strip with `print_thin_walls` | `Ok`; one open, odd, two-junction 0.34 mm line |
| 5 µm square | `Err(EmptyAfterPreprocess)` |

### Oracle

`check_output` compares each result against the declared mode and does not use the pipeline's own
closure or index logic. In a 200-case probe:

- 608 printable lines: 608/608 index-uniform, 608/608 satisfy the endpoint rule;
- 470 closed lines, all with explicit repeats;
- 29 `EmptyAfterPreprocess` outcomes;
- 29/29 guaranteed-class cases nonempty.

The 10 mm square, the 5 µm square, the outcome modes and the probe are not canonical output
goldens.

### Tolerance

Index, closure and the marker's first width of zero are exact checks. Coordinate and width bounds
are derived from the live profile extent.

### Shrinking

The geometry shrinks through the shared #2 value tree, as in `H-ARACHNE-GRAPH`. The parameter
choices shrink within their finite sets, which keeps the cap even and inside the profile. The
outcome mode is re-derived from the shrunk geometry.

### Execution

Measured libtest walls on Windows: 0.23 s for the routine selection (64/64) and 5.39 s for the
extended selection (1024/1024), with zero rejects.

### Failure record

Artifacts are written under `target/property-testing/<run-token>/H-ARACHNE-PIPELINE/`.
`arachne_local_maximum_pre_stitch_is_open` pins the canonical pre-stitch local maximum: six
junctions, odd, `is_closed = false`. Before the fix this was a seven-junction closed line. The
post-stitch pentagon fixture now measures one open, odd, six-junction inset-1 line.

### Acceptance

Maps to AC-3. `arachne_pipeline_oracle_controls` rejects:

- a falsely closed line with unequal endpoints in the explicit-repeat domain;
- a corrupted `perimeter_index`;
- an inset carried by neither endpoint;
- a nonzero-first-width inner-contour marker;
- an empty guaranteed output.

Empty and tiny outcomes are never credited as printable coverage.

## M-ARACHNE-CLASSIFY

Registry record: `id = "M-ARACHNE-CLASSIFY"`, `package = "arachne-perimeters"`,
`target_kind = "lib"`, `target_name = "arachne_perimeters"`,
`test_name = "property_tests::proptest_module_arachne_classify"`,
`production_symbol = "arachne_perimeters::classify_line"`,
`production_path = "modules/core-modules/arachne-perimeters/src/lib.rs"`,
`features = ["arachne-perimeters/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-arachne-classify"`.

### Stable property ID

`M-ARACHNE-CLASSIFY` checks the role and loop-type table of the private `classify_line`.

### Owner

- **Package and module.** Package `arachne-perimeters`, private unit-test module `property_tests`
  (`modules/core-modules/arachne-perimeters/src/property_tests.rs`), declared with stacked
  `#[cfg(test)]` and `#[cfg(feature = "property_testing")]`. See
  [module-owned property files](../05_module_sdk.md#module-owned-private-kernel-property-files-packet-proptest-openrscad_10).
- **Controls.** `property_tests::arachne_module_oracle_controls` and
  `property_tests::arachne_module_domain_rejections`.

### Contract

| Input | Result |
|---|---|
| `is_odd`, `inset_idx == 0`, `print_thin_walls` | `(ExtrusionRole::ThinWall, LoopType::ThinWall)` |
| any other `inset_idx == 0` | `(OuterWall, Outer)` |
| every positive inset, odd ones included | `(InnerWall, Inner)` |

`is_closed` alone changes no classification. This PnP ThinWall labelling is not claimed as an
upstream role identity.

### Domain

`max_inset_idx` is 8 for routine and 32 for extended. `checked_classify_limits` rejects zero and
out-of-range values. Junction count is `0..=3`.

### Strategy

The strategy draws every Boolean mode, plus `inset_idx` from `prop_oneof![Just(0), 1..=max]`.

### Oracle

The oracle is `expected_classification`, an independent truth table. Each case also re-runs
`classify_line` with `is_closed` toggled and requires the same result.

### Tolerance

Exact enum equality.

### Shrinking

A positive inset shrinks toward 1 and never to 0, so the `InnerWall` row survives every shrink
path. Zero stays zero.

### Execution

Measured libtest walls on Windows: 0.03 s for the routine selection (64/64) and 0.24 s for the
extended selection (1024/1024).

### Failure record

Artifacts are written under `target/property-testing/<run-token>/M-ARACHNE-CLASSIFY/`. Run
plainly, the body fails with `missing_property_context` by design.

### Acceptance

Maps to AC-4. `arachne_module_oracle_controls` tabulates all Boolean modes for both inset classes
and rejects a mutation that maps a deeper odd wall to GapFill.

## M-ARACHNE-BUILD

Registry record: `id = "M-ARACHNE-BUILD"`, `package = "arachne-perimeters"`,
`target_kind = "lib"`, `target_name = "arachne_perimeters"`,
`test_name = "property_tests::proptest_module_arachne_build_walls"`,
`production_symbol = "arachne_perimeters::ArachnePerimeters::build_walls"`,
`production_path = "modules/core-modules/arachne-perimeters/src/lib.rs"`,
`features = ["arachne-perimeters/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-arachne-build"`.

### Stable property ID

`M-ARACHNE-BUILD` checks that `ArachnePerimeters::build_walls` keeps each point, its width, its
flag and the line topology together.

### Owner

Package `arachne-perimeters`, module `property_tests`. The test calls real `build_walls` with a real
`SliceRegionViewBuilder` region and `PerimeterSpatialContext::new` built from the same declared
data.

### Contract

For each nonempty input line, exactly one wall keeps `inset_idx` as `perimeter_index` and has the
tabulated role and loop type. An empty line produces no wall. At each output index `i`:

- the point maps to its source junction;
- its width matches the independent real-number value `spacing + height*(1 - π/4)`;
- `width_profile.widths[i] == path.points[i].width` exactly;
- the bridge flag belongs to that same point.

With `LoopType::ThinWall`, every flag has `is_thin_wall` set and none has `is_bridge`. `line.is_closed`
agrees with `path.is_closed()`, and orientation normalization never adds a closing edge to an open
path.

Winding follows canonical `traverse_extrusions` (`PerimeterGenerator.cpp`):

- An open line keeps its input order.
- A closed line is reversed only when its input winding disagrees with
  `want_ccw = (wall_direction == CCW) == (is_contour || region_line_count == 2)`, flipped by
  `g7_reverse`.
- `is_contour` is `ExtrusionLine::is_contour` (`ExtrusionLine.cpp`): the line is closed and its input
  junctions wind clockwise.
- `region_line_count` is the number of nonempty lines in the batch.
- Neither `inset_idx` nor `boundary_type` affects winding.

### Domain

- **Bounds.** `max_junctions`/`max_abs_mm_units`/`max_spacing_units`/`max_height_units` are
  32/20000/2000/500 for routine and 64/40000/8000/2000 for extended. `checked_build_limits`
  rejects invalid values before sampling. The two bounds that shape the region are live:
  `max_junctions` is the upper end of the sampled total junction budget (the five-line roster
  needs 23), and `max_abs_mm_units` is the upper end of the sampled region half-extent; every
  roster shape is built as a fraction of that half-extent.
- **Roster.** Each case carries:
  - an explicit-repeat closed outer and inner rectangle;
  - two distinct-endpoint open chains in opposite orders;
  - an odd inset-0 explicit-repeat rectangle;
  - an empty line.

  The two-line roster keeps only the outer and inner rectangles, plus the empty line. The
  generated total junction count is sampled in `23..=max_junctions` and extra collinear points are
  placed on the outer rectangle's bottom and top edges; the region half-extent is sampled in
  `10000..=max_abs_mm_units`.
- `build_walls_bounds_drive_geometry_and_junctions` builds two different bound pairs and asserts
  both the input and the emitted walls change extent and junction total, so neither bound can
  silently stop reaching generation. `max_spacing_units` must also be at least
  `max_junctions`.

### Strategy

A `no_shrink` mode word selects six things:

- contour winding (`wall_direction`);
- `g7_reverse`;
- `print_thin_walls`;
- whether an asymmetric mixed bridge rectangle is present;
- whether the roster is the two-line region;
- whether the closed inner rectangle is wound clockwise (a contour) or counter-clockwise (a hole).

Together these cover open and closed lines, both input windings, both directions, both `g7_reverse`
values and the two-line term. The bridge rectangle gives positive and negative flags.
`arachne_module_oracle_controls` also sweeps all 64 mode words through the real kernel, so no
branch depends on what the sampler draws. The layer height, the base of the unequal per-point bead
spacings, the junction budget and the region half-extent are shrinkable; the mode word and the
bridge membership are not.

The deterministic prelude runs three canonical-winding witnesses and the bounds-reach-generation
control before the generated campaign, each under a fresh `CaseGuard`:

- `build_walls_winding_witness_open_path_keeps_order`;
- `build_walls_winding_witness_clockwise_inner_is_contour`;
- `build_walls_winding_witness_two_line_region_inner_is_contour_like`;
- `build_walls_bounds_drive_geometry_and_junctions`.

Each is also a plain test. The three winding witnesses all failed under the former inset-based
rule. See
`docs/testing/proptest-arachne-evidence.md#wall-winding-traverse_extrusions-and-extrusionlineis_contour`.

### Oracle

- **Bridge membership.** Predicted by `point_in_region` and analytic rectangle inequalities, never
  by `PerimeterSpatialContext::is_bridge` or `build_wall_flags`.
- **Expected winding.** Derived independently by `expected_want_ccw`, from `is_closed`, the input
  signed area, the nonempty line count, `wall_direction` and `g7_reverse`, using the canonical
  formula above. The output-order model is identity for an open line, and for a closed line a
  reversal exactly when the input winding disagrees with the target.
- **Widths.** Compared with the real-number formula above.

### Tolerance

`width_envelope` is derived from the f32 evaluation order `1 - PI_f32/4`, then `h * q`, then
`s + p`: `E = (s_max + (1 + 2*q_ub)*h_max) * 2^-24`. Index, flag and width-profile equality are
exact.

### Shrinking

The mode word does not shrink, so the witness survives every shrink step.
`shrink_retains_geometry_and_mode_witness` drives the real value tree through `simplify`. It
asserts that the witness is unchanged, the contract still holds, and a nonempty line remains.

### Execution

Measured libtest walls on Windows: 0.33 s for the routine selection (64/64) and 5.90 s for the
extended selection (1024/1024).

### Failure record

Artifacts are written under `target/property-testing/<run-token>/M-ARACHNE-BUILD/`. The concrete
regression `property_tests::build_walls_reversed_variable_width_keeps_profiles_and_bridge_flags_attached`
reproduced the defect before the fix: `build_walls` reversed `path.points` but not
`width_profile.widths` or `feature_flags`. Its assertion reported `width_profile.widths[1] =
0.6429204` against `path.points[1].width = 0.84292036`. The test now covers reversal with unequal
widths and asymmetric bridge flags. After the winding correction, its fixture was moved to a case
canonical still reverses: a closed CCW inset-1 rectangle in a three-line region under
`wall_direction = counter_clockwise`. A closed input wound CCW is a hole under
`ExtrusionLine::is_contour`. The former rule also reversed this fixture, so the test is not a
falsifier of the winding fix. The three winding witnesses above are.

### Acceptance

Maps to AC-5. `arachne_module_oracle_controls` rejects:

- a points-only reversal;
- a missed spacing → width conversion;
- fabricated closure on an open path.

The real unmutated result passes the same comparator.

## M-ARACHNE-SEQUENCE

Registry record: `id = "M-ARACHNE-SEQUENCE"`, `package = "arachne-perimeters"`,
`target_kind = "lib"`, `target_name = "arachne_perimeters"`,
`test_name = "property_tests::proptest_module_arachne_sequence"`,
`production_symbol = "arachne_perimeters::commit_wall_sequence"`,
`production_path = "modules/core-modules/arachne-perimeters/src/lib.rs"`,
`features = ["arachne-perimeters/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-arachne-sequence"`.

### Stable property ID

`M-ARACHNE-SEQUENCE` checks that the private `commit_wall_sequence` commits walls region-locally
and loses nothing.

### Owner

Package `arachne-perimeters`, module `property_tests`. The existing `wall_sequence_commit_tdd` (3
tests, green) separately covers the per-region driver, the layer-dependent sandwich and
cross-region precedence. No second host-algorithm credit is recorded.

### Contract

In the controlled single-outer batch (exactly one `Outer` wall, at either end), first normalize:
leave an outer-first input unchanged and reverse an outer-last one. Then:

| Mode | Committed order |
|---|---|
| `InnerOuter` | the normalized order |
| `OuterInner` | the normalized order reversed |
| `InnerOuterInner` | first inner, outer, remaining inners |

Empty and singleton input are unchanged. A batch without a unique endpoint outer (outer at both
ends, or no outer) is checked only for being a lossless permutation. Identity covers the whole
`WallLoop`, including point direction, per-point widths and flags.

### Domain

`max_walls` is 6 for routine and 12 for extended; `checked_sequence_limits` rejects invalid
values.

### Strategy

The strategy draws a wall count in `0..=max_walls`, a `no_shrink` layout word and a mode. The
layout word picks outer-first, outer-last, outer at both ends, or no outer. Each wall from
`sequence_wall` carries a unique `perimeter_index`, coordinates, unequal widths and
`tool_index`/bridge flags.

### Oracle

`expected_sequence_order` derives the expected identity list independently, and `verify_sequence`
compares complete `WallLoop` values against it.

### Tolerance

Exact `WallLoop` equality.

### Shrinking

The wall count shrinks, and the expected order is re-derived from the shrunk batch. The layout
word does not shrink.

### Execution

Measured libtest walls on Windows: 0.03 s for the routine selection (64/64) and 0.75 s for the
extended selection (1024/1024).

### Failure record

Artifacts are written under `target/property-testing/<run-token>/M-ARACHNE-SEQUENCE/`.

### Acceptance

Maps to AC-6. `arachne_module_oracle_controls` rejects a naive full reversal of an outer-first
`InnerOuter` batch and a lost feature-flag vector. It also confirms that `OuterInner` on the same
batch is the full reverse.

## H-REGIONMAP

Registry record: `id = "H-REGIONMAP"`, `package = "slicer-core"`, `target_kind = "test"`,
`target_name = "property_prepass"`, `test_name = "proptest_region_mapping_expansion"`,
`production_symbol = "slicer_core::algos::region_mapping::execute_region_mapping_with_cap"`,
`production_path = "crates/slicer-core/src/algos/region_mapping.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-regionmap"`.

### Stable property ID

`H-REGIONMAP` — the complete ordered `(layer, object, region) × variant_chain` expansion of
`PrePass::RegionMapping`, including the per-key config authority and the cap boundary.

### Owner

Package `slicer-core`; test target `property_prepass`; test
`proptest_region_mapping_expansion` in `crates/slicer-core/tests/property_prepass.rs`. Production
surface: `execute_region_mapping_with_cap` (`crates/slicer-core/src/algos/region_mapping.rs`).
Deterministic controls live in `crates/slicer-core/tests/property_prepass_common/controls.rs`
(`prepass_model_controls`, `prepass_regionmap_order_and_cap_controls`,
`prepass_regionmap_rejection_controls`, `prepass_owned_shrink_and_bound_controls`). The
cross-stage join (committed map → paint segmentation → `build_layer_selection_index`) is
`prepass_paint_map_identity_integration` in
`crates/slicer-runtime/tests/property_prepass_controls.rs`; the wrapper-only chain assertion is
`prepass_regionmap_runtime_wrapper` in the same target.

### Contract

Per ADR-0018 § Shared priority/name chain identity and `docs/03_wit_and_manifest.md`
§ Variant-chain enumeration order: for every active `(layer, object, region)` parent the map
holds exactly the chains produced by choosing, per canonical axis, either no value or exactly one
of the object's distinct values. The canonical axis sequence is ascending
`(declared priority, semantic name)`; each chain is a subsequence of that sequence, and its
pairs appear in it. Cardinality per parent is `∏ (1 + K_i)` and the empty chain occurs exactly
once per parent. `RegionKey` equality includes the ordered vector.

Each entry's config is the compatibility authority: the parent's base config, then the declared
paint-config overlays in **lexical semantic namespace order** (wholesale replacement, lexically
last present wins), then the tool overlay last. Chain order does not drive overlay order. The
interner holds exactly the distinct expected configs, once each.

### Domain

`max_layers`, `max_objects`, `max_regions`, `max_semantics`, `max_values_per_semantic` and
`max_chains` are required profile bounds. The strategy is total (no `prop_assume`, zero rejection
budget). Axes are drawn from the five legal fixture declarations — `material` (100),
`fuzzy_skin` (200), `com.example.zeta` (1000), `com.example.alpha` (1100) and
`com.example.beta` (1100) — so both `priority`-beats-name and lexical-tie cases are reachable.
Per object, each axis is absent or carries a nonempty subset of its declared value family
(`Flag`, `ToolIndex` or `Custom`). Objects and layers are enumerated explicitly, so absent paint,
one object, two objects, one layer and multiple layers are all reachable.

### Strategy

Axis declarations, layer/object/region counts and, per `(object, axis)`, a 4-bit value mask.
The mask keeps one uniform element strategy across the three value families; bits beyond an
axis's alphabet are ignored. Values are sorted by an independently implemented canonical
`PaintValue` comparator before use. Before sampling, the property runs the four named
material/fuzzy chains through the same checker, so a broken checker cannot be masked by a lucky
draw.

### Oracle

`crates/slicer-core/tests/property_prepass_common/model.rs`. The expected chain set is an
**ordinal decode** over the mixed radix `1 + K_i` (digit 0 omits an axis, digit `d > 0` selects
`values[d - 1]`), not the production recursive enumerator. The expected config is computed from
the declared base config and the declared overlay table by sorting overlay names lexically. The
model never calls `execute_region_mapping_with_cap`'s sorter, enumerator, interner, `config_for`
or `paint_variant_region_id` to build an expectation.

### Tolerance

Exact equality throughout: full `RegionKey` sets (including ordered chains), exact
`config_for`/`config_for_raw` values, exact distinct-config pool membership and cardinality, and
exact `CapExceeded { entry_count, cap }` at `cap = total - 1`. No unordered comparison is used for
a chain, and no float tolerance is needed because every declared width and density is dyadic.

### Shrinking

Proptest shrinks masks toward zero (fewer painted axes and values), object/layer/region counts
toward one, and axes toward an empty declaration. The rare modes the contract names — the empty
chain, the combined chain, a tied priority, a nonempty painted object — are supplied by the
deterministic witnesses in the controls and by the pre-sampling witness, so a shrink never needs
to preserve them probabilistically.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, zero rejection budgets. Measured on Windows
on 2026-11-19: routine `cases = 64` in 0.10 s, extended `cases = 256` in 0.99 s (libtest walls,
after the build was warm). Bounds: routine
`max_layers = 2`, `max_objects = 2`, `max_regions = 2`, `max_semantics = 3`,
`max_values_per_semantic = 3`, `max_chains = 64`; extended
`max_layers = 2`, `max_objects = 3`, `max_regions = 2`, `max_semantics = 4`,
`max_values_per_semantic = 3`, `max_chains = 256`. Both are the exact widest reachable products
for their declared maxima, so a well-formed case is always compared in full.

### Failure record

Artifacts are written under `target/property-testing/<run-token>/H-REGIONMAP/`; a failure keeps
`context.json`, `observations.jsonl`, `case.txt` and `failure.json` with the reproduction
command. Root-cause regression destination: `prepass_paint_map_identity_integration`
(`crates/slicer-runtime/tests/property_prepass_controls.rs`) — a named, RNG-independent scene,
never a seed.

### Acceptance

Every generated case's full key set, per-key config, config pool and cap boundary agree with the
independent model. `prepass_model_controls` rejects a dropped empty chain, a dropped combined
chain, a name-order substitution, a substituted value, a duplicated key, a wrong config, a
non-idempotent pool, a truncated pool and a cap success. `prepass_regionmap_order_and_cap_controls`
pins the four named chains, the community `1000 → 1100/1100` order with its lexical tie, the
lexical overlay width `0.75` and the tool-last width `1.0`, the exact/one-below cap boundary, the
zero-count/zero-cap mode and a unique top contributor. `prepass_regionmap_rejection_controls`
covers the declared-scalar error fields, a duplicated full active identity,
identical-footprint modifier grouping, an unsupported base parent, a real
distinct-footprint `modifier_sub_region_id` collision, and both halves of the
painted-id contract;
`prepass_owned_shrink_and_bound_controls` rejects a narrowing bound.
`prepass_paint_map_identity_integration` proves the real map → paint → selection join, both
negative mutations returning `SelectionError::MissingModelConfig` with the exact map-space key.
Defect outcome for this family: **no production defect was confirmed**; the ordering change is an
approved identity correction authorized by the source plan, recorded in ADR-0018's 2026-11-19
amendment and DEV-199.

## H-PAINT-COMPOSE

Registry record: `id = "H-PAINT-COMPOSE"`, `package = "slicer-core"`,
`target_kind = "test"`, `target_name = "property_prepass"`,
`test_name = "proptest_paint_composition_cells"`,
`production_symbol = "slicer_core::algos::paint_segmentation::compose_variants::compose_variants"`,
`production_path = "crates/slicer-core/src/algos/paint_segmentation/compose_variants.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-paint-compose"`.

### Stable property ID

`H-PAINT-COMPOSE` — the geometric partition, coverage and ordered chain identity
of the paint-compilation composer.

### Owner

Package `slicer-core`; target `property_prepass`; test
`proptest_paint_composition_cells` in `crates/slicer-core/tests/property_prepass.rs`.
Production surface: `compose_variants` in
`crates/slicer-core/src/algos/paint_segmentation/compose_variants.rs`. The model is
`crates/slicer-core/tests/property_prepass_common/paint.rs`; deterministic controls are
`prepass_paint_partition_and_driver_controls` in
`crates/slicer-core/tests/property_prepass_common/controls_paint.rs`.

### Contract

Per ADR-0018 § Shared priority/name chain identity and `docs/02_ir_schemas.md` IR 6/IR 9a, the
composer emits one key per subset of the supplied semantic outputs — the caller supplies one
output per `(declared axis, declared value)` in canonical `(priority, semantic name)` order — and
assigns every point of the layer total to exactly one chain: the subset of semantics covering it,
in that same axis order, with the empty chain holding the unpainted residual. Chains are pairwise
disjoint, their union is the total, no chain extends outside it, and longer chains claim their
intersection before shorter ones. The key vector order is the caller's axis order; it is never
re-sorted.

### Domain

`max_layers = 2 / 2`, `max_semantics = 2 / 3`, `max_cells = 16 / 16`,
`max_abs_units = 64 / 128`. The grid is built on an **even** coordinate lattice (outer edges and
every interior cut are even), so each cell midpoint is an integer point that can never lie on a
boundary. Axes are drawn from `material` (100, two tool values), `fuzzy_skin` (200, one flag) and
`com.example.alpha` (1100, one custom value), so priority order, multi-value axes, disjoint areas
of one semantic, partial and full overlap, and BASE-only cells are all reachable.

### Strategy

Layer count, a `columns x rows` grid with even interior cuts, a declared axis subset, and one label
choice per `(layer, cell, axis)`: unpainted, or one of that axis's declared values. A layer whose
draw is entirely unpainted is promoted to one painted cell so the painted half of the checker is
never vacuous; the fully-BASE mode keeps its own deterministic witness.

### Oracle

`property_prepass_common::paint` enumerates the expected key set as the exact cross-product
`∏ (1 + K_i)` over the supplied outputs, derives each cell's chain from the declared labels and
the independently sorted `(priority, name)` axis order, then classifies each returned chain with
`oracles::point_in_region` at the cell midpoint. It never calls Clipper, `compose_variants` or any
production sorter to build an expectation.

### Tolerance

Exact. Cell midpoints are integer points; membership is `Inside`/`Outside` with `Boundary`
rejected outright. Areas, counts and `Vec` key equality are exact; no float geometry is involved.

### Shrinking

Removes layers, axes, values, cells and labels. The model is re-derived for every shrink, and the
rare BASE-only and single-axis shapes are covered by deterministic witnesses.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on Windows
on 2026-11-19: routine `cases = 64` in 0.05-0.64 s across runs, extended
`cases = 256` in 0.20-0.21 s.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-PAINT-COMPOSE/`. Root-cause regression
destination: `prepass_paint_partition_and_driver_controls`
(`crates/slicer-core/tests/property_prepass_common/controls_paint.rs`), never a seed.

### Acceptance

Every cell of every layer is owned by exactly its expected chain; the key set is the full supplied
cross-product; no chain leaves the total extent. The controls refute a lost BASE chain, a filled
(unpainted) cell, a name-ordered key, a false all-empty result and swapped ownership, and confirm a
live combined material/fuzzy cell plus a BASE residual.

## H-PAINT-OVERHANG

Registry record: `id = "H-PAINT-OVERHANG"`, `package = "slicer-core"`,
`target_kind = "test"`, `target_name = "property_prepass"`,
`test_name = "proptest_overhang_adjacent_layer_areas"`,
`production_symbol = "slicer_core::algos::overhang_annotation::annotate_overhangs"`,
`production_path = "crates/slicer-core/src/algos/overhang_annotation.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-paint-overhang"`.

### Stable property ID

`H-PAINT-OVERHANG` — the unsupported area and previous-boundary identity of one
object's adjacent layer footprints.

Since ticket 142 the producer no longer partitions that area into four distance
bands: the consumer derives the quartile from the signed distance to the
previous boundary and `SurfaceClassificationIR.overhang_band_width_mm`. This
record therefore checks the annotation's area and predecessor identity; the
per-point quartile is checked by `M-ARACHNE-BUILD` and `M-CLASSIC-*`.

### Owner

Package `slicer-core`; target `property_prepass`; test
`proptest_overhang_adjacent_layer_areas`. Production surface:
`annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`).
Model: `crates/slicer-core/tests/property_prepass_common/overhang.rs`. Controls:
`prepass_overhang_area_controls` in
`crates/slicer-core/tests/property_prepass_common/controls_paint.rs`.

### Contract

For one object's consecutive footprints, `overhang_areas` holds the unsupported
area `current \ previous` — never a band partition — and
`prev_layer_boundaries` holds the actual previous footprint for the same layer.
Layers with no overhang have **no key** in either returned map. A single layer,
identical consecutive footprints and an empty current slice are the no-overhang
modes; the first layer has no predecessor and never overhangs. A nonempty
current after an explicitly empty predecessor is the separate positive mode:
the whole current footprint is unsupported and the present predecessor entry is
an empty vector.

### Domain

`max_layers = 2 / 2`, `max_abs_units = 200000 / 200000`,
`max_band_width_units = 12000 / 24000`, `max_cells = 32 / 64`. The scene is the
packet's X-strip fixture: two rectangles with the same Y extent, the current one
wider by a positive `shift`; the unsupported area is the exact X strip
`[WIDTH, WIDTH + shift] x [Y0, Y1]`. The registry's `max_band_width_units` now
caps that strip width, since the annotation no longer emits band polygons.

### Strategy

A positive `shift` capped by `max_band_width_units`; the classification grid is
derived from the strip edges with a midpoint and quarter-point cuts, so every
cell midpoint is strictly inside the strip.

### Oracle

`property_prepass_common::overhang` asserts that every cell midpoint of the
analytically derived strip is inside the reported unsupported area and that a
point well inside the previous footprint is outside it, using
`oracles::point_in_region`; `Boundary` is rejected. Every reported vertex must
stay inside the current footprint's extent, and both maps must carry exactly
the overhanging layer.

### Tolerance

Exact cell ownership. Every midpoint sits at least a quarter strip-width from
the strip edges, while the reported difference may round the fixture's corners,
so no ownership decision depends on a rounded vertex.

### Shrinking

Shrinks the shift; the grid is re-derived from the shrunk strip, and the
positive witness is deterministic, so shrinking never needs to preserve it
probabilistically.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.
Measured libtest wall for the routine selection on Windows on 2026-11-25:
16.8 s. The extended selection was not re-measured this session.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-PAINT-OVERHANG/`. Root
cause regression destination: `prepass_overhang_area_controls`, never a seed.

### Acceptance

Every cell midpoint of the analytic strip is inside the reported unsupported
area; a previous-footprint interior point is outside it; both maps hold exactly
the overhanging layer and the actual previous footprint; and the three
no-overhang modes leave both maps without the layer key. The controls refute a
substituted previous footprint, a missing area, an un-differenced current
footprint and the previous footprint standing in for the strip. The runtime
object-delivery half of AC-2 is covered by
`prepass_overhang_object_delivery`, which asserts each object's own published
`overhang_areas`, its own previous boundary, and the empty post-ticket-142
`overhang_quartile_polygons` map.

## H-PAINT-DRIVER

Registry record: `id = "H-PAINT-DRIVER"`, `package = "slicer-core"`,
`target_kind = "test"`, `target_name = "property_prepass"`,
`test_name = "proptest_paint_driver_source_isolation"`,
`production_symbol = "slicer_core::algos::paint_segmentation::execute_paint_segmentation"`,
`production_path = "crates/slicer-core/src/algos/paint_segmentation/mod.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-paint-driver"`.

### Stable property ID

`H-PAINT-DRIVER` — the paint driver's source isolation, axis ownership and
documented painted-region identity.

### Owner

Package `slicer-core`; target `property_prepass`; test
`proptest_paint_driver_source_isolation`. Production surface:
`execute_paint_segmentation` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`).
Model: `crates/slicer-core/tests/property_prepass_common/driver.rs`. The
cross-stage selection join is `prepass_paint_map_identity_integration` in
`crates/slicer-runtime/tests/property_prepass_controls.rs`.

### Contract

For each object the driver emits one region per chain that owns geometry, plus
a BASE region. A region may only contain geometry from its own object; a
neighbour's axes never appear in its chains; BASE and the painted chain must
never claim the same point; and the region ids follow the documented scheme —
the empty chain keeps the parent's base id, and the material `ToolIndex(0)` fast
path mints payload 1. Empty mesh, absent paint and an empty map are
unchanged-input paths that return the caller's `Arc` unchanged. A same-object
synthesized-id collision is a hard error.

### Domain

`max_layers = 1 / 1`, `max_objects = 2 / 3`, `max_regions = 1 / 1`,
`max_abs_units = 200000 / 200000`, `max_facets = 16 / 16`. Objects are
axis-aligned 2 mm square tubes on a 1 mm pitch, so their footprints are disjoint
by construction. Each object paints a drawn subset of `material`
(`ToolIndex(0)`, 100) and `fuzzy_skin` (`Flag(true)`, 200) on every wall facet;
the paint-layer order is a rotation of the axes in use. An object with no axes
is the unpainted-object mode.

### Strategy

Object count, one axis subset per object, and a paint-order rotation. The
deterministic block runs the three unchanged-input paths and the colliding-chain
error before sampling.

### Oracle

`property_prepass_common::driver` derives each object's widest chain from its
own axes in independently sorted `(priority, name)` order, checks every output
point against that object's square, samples a 4x4 interior grid to prove BASE
and the painted chain are disjoint, and classifies the widest chain's coverage
with `oracles::point_in_region`. It never calls Clipper or the driver's own
partition to build an expectation.

### Tolerance

Exact for ids, chain vectors and point containment; the BASE/painted
disjointness check is a point grid rather than an exact area so a Clipper sliver
cannot make it flaky, while "BASE is the whole contour" still fails.

### Shrinking

Removes objects and axes, and rotates the paint order toward the identity. The
unchanged-input and collision-error modes are deterministic.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.
Measured on Windows on 2026-11-19: routine `cases = 64` in 0.22 s, extended
`cases = 256` in 1.05 s.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-PAINT-DRIVER/`. Root
cause regression destination: the unchanged-path and collision assertions in
`property_prepass_common::driver`, never a seed.

### Acceptance

No region leaves its own object's square; each object's chain set is exactly its
own widest chain plus BASE; the widest chain covers the square's centre; BASE
and the painted chain are disjoint; ids follow the documented scheme; repeated
runs agree; and the three unchanged-input paths return the input `Arc`.
`colliding_chain_error` proves the real `PaintVariantRegionIdCollision` path with
`material`/`ToolIndex(0)` and `ToolIndex(999_999)`.

## H-SUPPORT-SCHEDULE

Registry record: `id = "H-SUPPORT-SCHEDULE"`, `package = "slicer-core"`,
`target_kind = "test"`, `target_name = "property_prepass"`,
`test_name = "proptest_support_schedule_and_sources"`,
`production_symbol = "slicer_core::algos::support_geometry::execute_support_geometry"`,
`production_path = "crates/slicer-core/src/algos/support_geometry.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-support-schedule"`.

### Stable property ID

`H-SUPPORT-SCHEDULE` — the per-object support emission schedule and the exact
source-geometry entries the stage publishes.

### Owner

Package `slicer-core`; target `property_prepass`; test
`proptest_support_schedule_and_sources`. Production surfaces:
`build_emit_schedule` and `execute_support_geometry`
(`crates/slicer-core/src/algos/support_geometry.rs`). Model:
`crates/slicer-core/tests/property_prepass_common/support.rs`. Controls:
`prepass_support_schedule_controls` in
`crates/slicer-core/tests/property_prepass_common/controls_paint.rs`.

### Contract

Each object accumulates its effective layer height and emits on the layer where
the declared `support_layer_height_mm` target is reached, resetting the
accumulator; a target of `0` emits on every active layer and an object with no
active region never accumulates. Each emitted `(layer, object, region, chain)`
carries the committed source polygons for that exact region and chain; a
scheduled region with **no** committed source is a present key with an empty
polygon vector and the empty chain, which is not the same as an absent key. The
intermediate-model pass adds the same geometry under the `u32::MAX` sentinel for
every layer within `support_top_z_distance` of that object's column top. The
IR-wide `support_layer_height_mm` / `support_top_z_distance_mm` come from the
first active region of the plan.

### Domain

`max_layers = 4 / 4`, `max_objects = 2 / 3`, `max_regions = 1 / 1`,
`max_abs_units = 200000 / 200000`. Layer indices are unique and never the
sentinel; `z = 0.2 * (index + 1)`. Heights are `0.1`, `0.2` or `0.25` mm and
targets are `0`, one step, two steps or three steps, so the threshold comparison
is exact in `f32`. Chains alternate between empty and
`material`/`ToolIndex(0)`, and some scheduled layers deliberately carry no
source.

### Strategy

Layer count, object count, target choice per object, height choice, and a source
presence/chain selector. The deterministic block covers the empty plan (no
schedule, no entries) and the controls cover an empty-source scheduled key.

### Oracle

`property_prepass_common::support` re-derives the schedule with its own
accumulator, re-derives each object's column top, and rebuilds the expected key
map from the declared fixture. It never calls `build_emit_schedule` or the
collector to build an expectation.

### Tolerance

Exact. Keys, chain vectors, polygon counts and the IR-wide settings are compared
with `==`; all fixture coordinates are integers and all heights/targets are
dyadic.

### Shrinking

Removes layers and objects and simplifies the target choice. The empty-plan and
empty-source modes are deterministic.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.
Measured on Windows on 2026-11-19: routine `cases = 64` in 0.03 s, extended
`cases = 256` in 0.31 s.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-SUPPORT-SCHEDULE/`. Root
cause regression destination: `prepass_support_schedule_controls`, never a seed.

### Acceptance

The schedule, the full key set, the empty-source present keys, the sentinel
entries and the IR-wide settings all agree with the model. The controls refute a
dropped emission, a wrong object, a wrong chain and a false empty map, and they
confirm the live source-less scheduled key. Not covered here: the runtime
wrapper's `NoLayerPlan` / `MissingSliceIR` errors
(`prepass_support_wrapper_absence_controls`) and any planner output.

## H-SUPPORT-ROLES

Registry record: `id = "H-SUPPORT-ROLES"`, `package = "slicer-core"`,
`target_kind = "test"`, `target_name = "property_prepass"`,
`test_name = "proptest_support_role_partition"`,
`production_symbol = "slicer_core::support_regularize::regularize_entry_roles"`,
`production_path = "crates/slicer-core/src/support_regularize.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-support-roles"`.

### Stable property ID

`H-SUPPORT-ROLES` — the disjoint top/bottom/body/raft role partition of one
support entry in grid style.

### Owner

Package `slicer-core`; target `property_prepass`; test
`proptest_support_role_partition`. Production surface: `regularize_entry_roles`
(`crates/slicer-core/src/support_regularize.rs`) with `smooth_supports = false`.
Model: `crates/slicer-core/tests/property_prepass_common/roles.rs`. Controls:
`prepass_support_contact_role_controls`.

### Contract

In grid style the regularizer is set algebra over the declared role polygons:
`layer_area = top ∪ bottom ∪ body`, `iface_top = top ∩ layer_area`,
`iface_bottom = (bottom ∩ layer_area) − iface_top`, and the body remainder is
`layer_area − (iface_top ∪ iface_bottom)`. Raft geometry is passed through
untouched and is **not** part of `layer_area`. Output roles follow the input's
first-appearance order, with `SupportBody` appended last when it was absent from
the input and owns geometry. `None` means "unchanged": a nonpositive flow
spacing, a body-only entry, or no interface input. `TopInterface` and
`BaseInterface` alias onto the same output role; the aliasing is pinned
separately and deliberately kept out of the disjoint partition.

### Domain

`max_cells = 8 / 16`, `max_abs_units = 200000`. One row of cells on an even
lattice (the grid is derived from the label count), each labelled absent, top,
bottom, body or raft; the input roster is a rotation of the labels actually
present, so the fixture and the model always agree about which interfaces exist.

### Strategy

Cell count and a role rotation; each cell's label is drawn independently. The
deterministic block drives a top-over-body witness and the three `None` modes.

### Oracle

`property_prepass_common::roles` re-derives the partition with its own set
algebra and classifies each cell midpoint with `oracles::point_in_region`. It
never calls the regularizer or Clipper to build an expectation.

### Tolerance

Exact cell ownership and exact role order. `Boundary` classification is rejected
outright; all coordinates are integers and all cells are at least two units wide.

### Shrinking

Removes cells and simplifies labels; the `None` and aliasing modes are
deterministic.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.
Measured on Windows on 2026-11-19: routine `cases = 64` in 0.14 s, extended
`cases = 256` in 0.27 s.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-SUPPORT-ROLES/`. Root
cause regression destination: `prepass_support_contact_role_controls`, never a
seed.

### Acceptance

Every cell is owned by exactly the role its label names (or by none), the role
order matches the input order plus the appended body, every emitted role is
nonempty, and the three `None` modes return `None`. Top/base aliasing emits both
roles. No area monotonicity across layers and no universal nonempty support is
asserted.

## H-SUPPORT-CONTACT

Registry record: `id = "H-SUPPORT-CONTACT"`, `package = "slicer-core"`,
`target_kind = "test"`, `target_name = "property_prepass"`,
`test_name = "proptest_support_contact_rectangles"`,
`production_symbol = "slicer_core::algos::overhang_annotation::detect_support_contacts"`,
`production_path = "crates/slicer-core/src/algos/overhang_annotation.rs"`,
`features = ["slicer-core/property_testing", "slicer-core/host-algos"]`,
`requires_host_algos = true`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-support-contact"`.

### Stable property ID

`H-SUPPORT-CONTACT` — the enforced-layer plain-difference support contact set.

### Owner

Package `slicer-core`; target `property_prepass`; test
`proptest_support_contact_rectangles`. Production surface:
`detect_support_contacts` (`crates/slicer-core/src/algos/overhang_annotation.rs`).
Model: `crates/slicer-core/tests/property_prepass_common/contact.rs`. Controls:
`prepass_support_contact_role_controls`.

### Contract

On the enforced-layer plain-difference branch (`layer_id` inside
`[raft_layers, raft_layers + enforce_support_layers)`) the contact set is
`upper − lower − blockers`, with no offset, no expand-back, no sharp-tail
bypass, no bridge margin and zero XY expansion, then filtered by the tiny-spot
erosion of `−0.1 × external_perimeter_width` and unioned. A fully supported
column, a blocker-covered contact and a sub-threshold sliver produce no contact;
an empty upper region is the no-work mode.

### Domain

`max_abs_units = 200000`, `max_cells = 4`. A 2 × 1 cell lattice at
millimetre scale (each cell 4 mm × 8 mm) with independent upper/lower/blocker
cell flags, so supported cells, blocker-covered cells and genuine contacts are
all reachable.

### Strategy

Two independent boolean triples (upper/lower/blocker) on the fixed lattice. The
deterministic block runs the sliver, supported-column, blocker and empty-upper
modes and a positive witness.

### Oracle

`property_prepass_common::contact` derives the expected contact cells as
`upper ∧ ¬lower ∧ ¬blocker` and classifies the returned polygons at each cell
midpoint with `oracles::point_in_region`. It never calls `difference_ex` to
build an expectation. Cell geometry is millimetre-scale so the tiny-spot filter
cannot silently empty a legitimately printable contact.

### Tolerance

Exact cell ownership; `Boundary` is rejected. Coordinates are integers and
cells are 4 mm wide, far above the `0.04 mm` erosion.

### Shrinking

Flips cell flags toward the no-contact configuration; the sliver and supported
modes are deterministic.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.
Measured on Windows on 2026-11-19: routine `cases = 64` in 0.03 s, extended
`cases = 256` in 0.09 s.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-SUPPORT-CONTACT/`. Root
cause regression destination: `prepass_support_contact_role_controls`, never a
seed.

### Acceptance

Every cell's contact membership equals `upper ∧ ¬lower ∧ ¬blocker`; contacts
stay inside the upper extent; the sliver, supported-column, blocker-covered and
empty-upper modes produce no contact. The controls refute a substituted lower
layer, a lost blocker and a false empty result. This is not `annotate_overhangs`
(whose contract differs) and it makes no claim about planner routing.

## H-SUPPORT-TERRITORY

Registry record: `id = "H-SUPPORT-TERRITORY"`, `package = "slicer-wasm-host"`,
`target_kind = "test"`, `target_name = "property_prepass"`,
`test_name = "proptest_support_territory_clearance"`,
`production_symbol = "slicer_wasm_host::support_territory::TerritoryClipper::clip"`,
`production_path = "crates/slicer-wasm-host/src/support_territory.rs"`,
`features = ["slicer-wasm-host/property_testing"]`, `requires_host_algos = true`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-support-territory"`.

### Stable property ID

`H-SUPPORT-TERRITORY` — the host territory own/foreign clip and its clearance.

### Owner

Package `slicer-wasm-host`; target `property_prepass`; test
`proptest_support_territory_clearance`. Production surface:
`TerritoryClipper::{from_ir, clip}` (`crates/slicer-wasm-host/src/support_territory.rs`).
Model: `crates/slicer-wasm-host/tests/property_prepass_common/territory.rs`.
Controls: `prepass_support_analysis_territory_controls` (runtime target).

### Contract

A query naming an exact minted sub-region origin `(object, layer, region_id,
ordered chain)` keeps `polygons ∩ own territory`. A base region keeps
`polygons − inflate(foreign, clearance)`, and keeps `polygons` unchanged when no
foreign territory exists. `inflate` is the miter offset by the
`support_territory_clearance_mm` shared setting. `from_ir` returns `None` when
the analysis carries no territory, and `clip` returns `None` when the layer
carries no territory — both are no-clipping outcomes, not empty successes. The
`region_id` is the IR integer rendered as a decimal `&str`, so a distinct id is
a distinct origin.

### Domain

`max_objects = 1`, `max_regions = 3`, `max_abs_units = 200000`,
`max_clearance_units = 8000`, `max_cells = 16 / 32`. One to three territory
rectangles on a whole-millimetre lattice with clearances `0`, `0.2` or `0.4` mm;
the query body is a separate rectangle and the query is either a minted
sub-region or a base region.

### Strategy

Rectangle count, one rectangle per entry, a clearance choice and a sub-region
flag. Region ids are the entry index + 1 and families alternate, so own and
foreign territory always coexist.

### Oracle

`property_prepass_common::territory` re-derives membership analytically
(`body ∩ own` for a minted origin; `body` minus every inflated foreign rectangle
otherwise) and classifies sample points with `oracles::point_in_region`. Sample
coordinates are offset by 2500 units from every edge, so a `Boundary` outcome is
impossible by construction rather than tolerated.

### Tolerance

Exact point membership on a lattice where every edge is a multiple of 1000 units
and every clearance is 2000 or 4000 units. No float geometry is compared.

### Shrinking

Removes entries and simplifies the clearance; the absence and foreign-only modes
are deterministic.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.
Measured on Windows on 2026-11-19: routine `cases = 64` in 0.48 s, extended
`cases = 256` in 2.13 s.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-SUPPORT-TERRITORY/`. Root
cause regression destination: `prepass_support_analysis_territory_controls`,
never a seed.

### Acceptance

Every sample point's membership equals the analytic model; `from_ir` on an
empty analysis is `None`; a layer with no territory clips to `None`; a distinct
region id does not match a minted entry. This checks the host clipping seam, not
planner routing, aggregation ownership or whole-print support, and it is a PnP
territory rule (DEV-165), not an OrcaSlicer parity claim.

## H-SUPPORT-ANALYSIS

Registry record: `id = "H-SUPPORT-ANALYSIS"`, `package = "slicer-runtime"`,
`target_kind = "test"`, `target_name = "property_prepass"`,
`test_name = "proptest_support_analysis_layered_scenes"`,
`production_symbol = "slicer_runtime::builtins::support_analysis_producer::commit_support_analysis_builtin"`,
`production_path = "crates/slicer-runtime/src/builtins/support_analysis_producer.rs"`,
`features = ["slicer-runtime/property_testing"]`, `requires_host_algos = true`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-support-analysis"`.

### Stable property ID

`H-SUPPORT-ANALYSIS` — the host-owned layered support analysis: occupancy, the
feasible envelope, termination surfaces and the territory clearance.

### Owner

Package `slicer-runtime`; target `property_prepass`; test
`proptest_support_analysis_layered_scenes`. Production surface:
`commit_support_analysis_builtin`
(`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`). Model:
`crates/slicer-runtime/tests/property_prepass_common/analysis.rs`.

### Contract

With support enabled, `model_occupancy` holds exactly one entry per nonempty
committed region, keyed by the full `(layer, object, region_id, ordered chain)`
identity and carrying that region's polygons verbatim. Each object with a
nonempty bound contributes exactly one `baseline_feasible_envelope` rectangle —
its full XY bounds. Every occupancy key at that object's highest active layer
carries a `termination_surfaces` entry holding the top footprint plus the plate;
non-top keys carry none. `support_territory_clearance_mm` is the resolved
positive support line width; in the literal mode the property declares it is
that literal, and the percent mode is the documented percentage of the fallback
line width. A disabled analysis publishes the settings and no occupancy,
envelope or candidates.

### Domain

`max_layers = 3`, `max_objects = 2`, `max_regions = 2`, `max_abs_units = 200000`,
`max_cells = 16 / 32`. One to three layers of whole-millimetre rectangles, one
or two objects, two chain spellings, and an enabled/disabled flag. Every layer
carries at least one region so each object's top layer is well defined.

### Strategy

Layer count, object count, six region specs (rectangle, chain, presence) and the
enabled flag.

### Oracle

`property_prepass_common::analysis` rebuilds occupancy, bounds, the envelope
rectangles and the termination sets from the declared fixture, and re-derives
the literal clearance. It never calls the producer's own collectors.

### Tolerance

Exact key sets, exact polygon equality for occupancy, exact envelope rectangles
(compared as bounds), exact termination counts and exact `f32` clearance.

### Shrinking

Removes layers, objects, regions and the enabled flag; both witnesses are
deterministic.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.
Measured on Windows on 2026-11-19: routine `cases = 64` in 0.04 s, extended
`cases = 256` in 0.12 s.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-SUPPORT-ANALYSIS/`. Root
cause regression destination: `prepass_support_analysis_territory_controls`,
never a seed.

### Acceptance

Occupancy, envelope, termination and clearance agree with the model in both
enabled modes and the disabled mode publishes nothing. Candidate selection and
contact-layer eligibility are **not** asserted here; they depend on surface
classification this property does not fabricate. A candidate is not a printed
support plan, and no planner feasibility is claimed.

## H-LIGHTNING-SEEDS

Registry record: `id = "H-LIGHTNING-SEEDS"`, `package = "lightning-infill"`,
`target_kind = "lib"`, `target_name = "lightning_infill"`,
`test_name = "kernel::property_tests::proptest_lightning_seed_regions"`,
`production_symbol = "lightning_infill::kernel::generator::generate_initial_internal_overhangs"`,
`production_path = "modules/core-modules/lightning-infill/src/kernel/generator.rs"`,
`features = ["lightning-infill/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-lightning-seeds"`.

### Stable property ID

`H-LIGHTNING-SEEDS` — the per-layer internal-overhang seed regions of the
module-private lightning kernel.

### Owner

Package `lightning-infill` (a **lib** test, not a standalone binary); module
`kernel::property_tests`. Production surface:
`generate_initial_internal_overhangs`
(`modules/core-modules/lightning-infill/src/kernel/generator.rs`).

### Contract

Walking from the top layer down, each layer's seed regions are
`erode(current) − above`, where `erode` is the miter offset by the wall
supporting radius and `above` is the next-higher layer's **un-eroded** outline.
An empty layer above resets `above` to nothing. Radius zero degenerates to a
plain difference, and a layer fully covered by the layer above has no seed.

### Domain

`max_layers = 3`, `max_abs_units = 200000`, `max_radius_units = 20000`,
`max_cells = 32 / 64`. Layer outlines are axis-aligned rectangles of 1-5 mm on a
whole-millimetre lattice, one to three layers, some layers deliberately absent.

### Strategy

Layer count, square size and a presence vector; the radius is half a millimetre
bounded by the profile.

### Oracle

The model computes the expected region analytically (rectangle inset by the
radius minus the un-eroded layer above) and classifies the kernel's **flat ring
list** with an even-odd ray-crossing test written in the test module. It never
calls the kernel's offset or difference helpers.

### Tolerance

Exact point membership on samples offset 2500 units from every edge and from the
erosion ring, so a boundary sample is impossible by construction.

### Shrinking

Removes layers and presences; the empty-above, empty-layer-reset, zero-radius and
fully-covered witnesses are deterministic.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.
Measured on Windows on 2026-11-19: routine `cases = 64` in 0.03 s, extended
`cases = 256` in 0.08 s.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-LIGHTNING-SEEDS/`.
Controls: `kernel::property_tests::lightning_model_controls` and
`lightning_seed_tree_controls`.

### Acceptance

Every sample's membership equals the analytic model, and the four deterministic
witnesses hold. Hole preservation is covered only through the eroded-outline
case (the fixture's `above` rectangle is the only inner boundary); a general
polygon-with-holes fixture is not claimed.

## H-LIGHTNING-TREES

Registry record: `id = "H-LIGHTNING-TREES"`, `package = "lightning-infill"`,
`target_kind = "lib"`, `target_name = "lightning_infill"`,
`test_name = "kernel::property_tests::proptest_lightning_tree_structure"`,
`production_symbol = "lightning_infill::kernel::generator::Generator::generate_trees"`,
`production_path = "modules/core-modules/lightning-infill/src/kernel/generator.rs"`,
`features = ["lightning-infill/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-lightning-trees"`.

### Stable Property ID

`H-LIGHTNING-TREES` — the structural integrity and determinism of the generated
lightning forest.

### Owner

Package `lightning-infill` (lib test); module `kernel::property_tests`.
Production surface: `Generator::{new, generate_trees, get_trees_for_layer}`
(`modules/core-modules/lightning-infill/src/kernel/generator.rs`).

### Contract

Every node reachable from `tree_roots` reports `is_root()` exactly when it is a
root, is reached exactly once (no shared child, no cycle), and the traversal
terminates. The walk reports an exceeded `max_tree_nodes` budget as an
incomplete case rather than certifying a prefix. Two identical runs produce the
same forest. Points stay inside the fixture's declared bounded domain.

### Domain

`max_layers = 3`, `max_objects = 1`, `max_regions = 1`, `max_abs_units = 200000`,
`max_tree_nodes = 20000`. Narrowing stacks of 1-3 layers, 1-4 mm wide, shrinking
by 1-3 mm per layer.

### Strategy

Layer count, base width and per-layer shrink; the node budget is a profile bound.

### Oracle

A pointer-identity visited set over `Rc::as_ptr`, plus a declared domain box (the
union of the layer rectangles grown by 20 mm). Trees legitimately branch outside
their outline, so containment is a bounded-domain check, not an outline check.

### Tolerance

Exact. Structure and point equality only; no numeric tolerance is involved.

### Shrinking

Fewer layers, a smaller base and a smaller shrink; determinism is checked inside
every case, so a shrink that changes the forest fails rather than silently
passing.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets.
Measured on Windows on 2026-11-19: routine `cases = 64` in 0.05 s, extended
`cases = 256` in 0.17 s.

### Failure record

Artifacts under `target/property-testing/<run-token>/H-LIGHTNING-TREES/`.
Controls: `lightning_model_controls`, `lightning_seed_tree_controls`,
`lightning_owned_shrink_controls`.

### Acceptance

Roots, ownership, termination and determinism hold on every case; the node-budget
and duplicate-root mutants fail. Repeated-run equality is a determinism pin, not
a correctness or parity claim; no snapshot is treated as canonical evidence.

### Classic-perimeter and fuzzy-skin pilots (packet `proptest-openrscad_12_pbt-walls`)

Measured on Windows 11, rustc 1.96.0, Proptest 1.11.0, `dev` profile, workspace `target/`,
`generator_version = 1`, source revision `7fc9c2b4` plus the Step 7 registry/doc worktree.
Selected runs used seed `17102094542611295699` (routine) and `5869198346882150039` (extended);
the retained replay reused seed `17102094542611295699`. All records had
`accepted_cases == cases`, zero rejected callbacks, zero local/global rejects and
`shrink_completed = null`.

| Property | routine libtest | extended libtest | routine accepted | extended accepted |
|---|---:|---:|---:|---:|
| `M-CLASSIC-WALL-STRUCTURE` | 0.11 s | 0.39 s | 64 | 256 |
| `M-CLASSIC-BAND-SELECTION` | 0.10 s | 0.38 s | 64 | 256 |
| `M-FUZZY-REPEATABILITY` | 0.04 s | 0.12 s | 64 | 256 |
| `M-FUZZY-SPAN-PRESERVATION` | 0.04 s | 0.13 s | 64 | 256 |
| `M-FUZZY-DISPLACEMENT` | 0.04 s | 0.12 s | 64 | 256 |

Bounds are `max_abs_units=2048`, `max_vertices=16`, `max_holes=2`, `max_components=3`,
`max_shrink_candidates=32`, `max_wall_count=4`, `max_width_units=8`,
`max_layer_height_units=4` for both classic records; and `max_abs_mm_units=20000`,
`max_thickness_units=3000`, `max_point_distance_units=8000`, `max_points=8` for all fuzzy
records. Both selected and combined lanes reported `VERDICT: PASS`. The final measured
artifacts and exact logs are retained under `target/proptest-walls-evidence/review-fix/`.

## M-CLASSIC-WALL-STRUCTURE

Registry record: `id = "M-CLASSIC-WALL-STRUCTURE"`, `package = "classic-perimeters"`,
`target_kind = "lib"`, `target_name = "classic_perimeters"`,
`test_name = "property_tests::proptest_classic_wall_structure"`,
`production_symbol = "classic_perimeters::ClassicPerimeters::emit_walls"`,
`production_path = "modules/core-modules/classic-perimeters/src/lib.rs"`,
`features = ["classic-perimeters/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-classic-wall-structure"`.

### Stable property ID

`M-CLASSIC-WALL-STRUCTURE` checks ordinary wall role, closure and metadata alignment for the
selected no-medial-axis path.

### Owner

- **Package and module.** `classic-perimeters`, private `property_tests`
  (`modules/core-modules/classic-perimeters/src/property_tests.rs`), with stacked `#[cfg(test)]`
  and `#[cfg(feature = "property_testing")]` gates. See
  [module-owned property files](../05_module_sdk.md#module-owned-private-kernel-property-files-packet-proptest-openrscad_10).
- **Controls.** `classic_required_witnesses_are_nonempty`,
  `classic_structure_oracle_rejects_mutations`,
  `classic_shrink_preserves_emit_mode_and_topology`.

### Contract

Every emitted ordinary `Outer`/`Inner` wall has a `perimeter_index` below the requested wall
count; band 0 is `LoopType::Outer` / `ExtrusionRole::OuterWall` with the outer speed/width, and
band > 0 is `LoopType::Inner` / `ExtrusionRole::InnerWall` with the inner speed/width. Points,
width-profile entries and flags are parallel; the path is closed with an explicit repeat; at
least four points exist; consecutive XY are distinct; copied z/width/flow/default annotations,
closing width/flag and absent tool/order-lock values match the fixture. The emitted count is
never compared with `wall_count`.

### Domain

The shared `geometry_cases` constructions for `Convex`, `Concave`, `Holed` and `Disconnected`
are prebuilt for `max_abs_units = 2048`, `max_vertices = 16`, `max_holes = 2`,
`max_components = 3`, `max_shrink_candidates = 32`; every case is certified by
`validate_geometry` before the SUT runs. `wall_count` is `1..=4`, widths are
`width_floor(4)..=8` canonical units, layer height is `1..=4` units, and z spans
0.05..=0.35 mm. Both emit gates and both `variant_fuzzy` values are selected. Empty geometry
output is valid; an aggregate nonempty counter and the deterministic 20 mm witness make the
population non-vacuous.

### Strategy

The no-shrink topology `ClassTag` selector feeds `prop_ind_flat_map` into the matching prebuilt
domain. Wall count, both widths, layer height and z stay shrinkable. Emit mode and
`variant_fuzzy` use `prop::sample::select(...).no_shrink()`, never a union of `Just` arms. The
fixture calls the real private `emit_walls` with empty annotations, an empty spatial context,
`PathAnnotationMode::Unconsumed`, `detect_thin_wall = false` and `medial_axis_enabled = false`,
and a fresh `PerimeterOutputBuilder` per invocation.

### Oracle

`classic_structure_oracle` is local and independent. It does not call a production validator or
reconstruct the offset. It checks finite coordinates/width/flow first, then exact bits for
copied z, widths, flow and closing point, exact default annotations/flags, band/role/loop/speed
mapping, closure, distinct consecutive XY and parallel lengths.

### Tolerance

Exact f32 equality for copied values and structural metadata; no numeric epsilon.

### Shrinking

The class and both mode selectors cannot flip. Geometry shrinking preserves its declared
topology and every current case remains `validate_geometry`-valid; wall count and ordinary
scalars may shrink. The probe exercises both simplify and complicate for every class and fails
on a false-only walk.

### Execution

Both tiers, `in_process`, chacha, `timeout_ms = 0`, zero rejection budgets. Registry routine
(64 cases) measured 0.11 s with 43 nonempty generated cases (Convex/Concave/Holed/Disconnected
= 7/11/12/13); extended (256 cases) measured 0.39 s with 160 nonempty (42/27/44/47). Both had
zero rejects and every class had a nonempty population. The 128-case controlled pilot measured
0.20 s and 78 nonempty cases.

### Failure record

Artifacts under `target/property-testing/<run-token>/M-CLASSIC-WALL-STRUCTURE/`; retain geometry,
wall/band index and the first failing field. A confirmed SUT defect requires a named concrete
RNG/context-independent regression.

### Acceptance

Every ordinary wall satisfies the structure contract; the deterministic 20 mm/3-band witness is
nonempty with bands `{0,1,2}` in both fuzzy modes; the generated nonempty counter is positive;
all named oracle mutations are rejected.

## M-CLASSIC-BAND-SELECTION

Registry record: `id = "M-CLASSIC-BAND-SELECTION"`, `package = "classic-perimeters"`,
`target_kind = "lib"`, `target_name = "classic_perimeters"`,
`test_name = "property_tests::proptest_classic_band_selection"`,
`production_symbol = "classic_perimeters::ClassicPerimeters::emit_walls"`,
`production_path = "modules/core-modules/classic-perimeters/src/lib.rs"`,
`features = ["classic-perimeters/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-classic-band-selection"`.

### Stable property ID

`M-CLASSIC-BAND-SELECTION` checks the emit gates, calculated allowed-prefix band set, capacity,
degenerate inputs and fatal spacing.

### Owner

Same owner module and controls file as the structure record. Controls are
`classic_band_oracle_and_degenerate_cases`, `classic_negative_spacing_is_fatal`,
`classic_property_domains_reject_unsafe_bounds` and the structure controls rerun after shared
helper changes.

### Contract

`emit_outer`/`emit_inner` select exactly their ordinary roles. For nonempty output the emitted
band set is the gate-filtered prefix through the maximum computed band; a missing allowed index
is a failure. No outer output means no seam candidates; no inner output means no infill areas.
The requested wall count is only an upper bound. Degenerate contours emit nothing, and a
nonpositive spacing is fatal `ERR_NEGATIVE_SPACING`, never a clamp.

### Domain

The structure geometry and scalar domain, all three emit modes, and `wall_count == 1`. The
deterministic controls add a 1 mm square requesting ten bands, 2-point and collinear contours,
an empty polygon list, and a width below the positive-spacing threshold. Bounds are supplied
explicitly; missing/invalid/overflow values are rejected before sampling.

### Strategy

Reuses the construction-first class strategy and the single no-shrink emit selector, and keeps
ordinary scalars shrinkable.

### Oracle

`classic_band_oracle` runs `classic_structure_oracle` first, then checks gates, the allowed
prefix and side-output implications over the real builder's walls, infill and seams. It never
compares emitted wall/band count with `wall_count`.

### Tolerance

Exact integer band indices and booleans; no geometry tolerance.

### Shrinking

Modes cannot flip; smaller requests may legitimately emit fewer bands, but may not introduce a
band outside the request or break the allowed prefix.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Registry routine
(64 cases) measured 0.10 s with 38 nonempty cases, every class nonempty (Convex/Concave/Holed/
Disconnected = 11/7/12/8), modes 28/19/17 and 10 single-band cases; extended (256 cases) measured
0.38 s with 145 nonempty (30/44/45/26), modes 77/94/85 and 52 single-band cases. Both had zero
rejects. The capacity witness emitted exactly band `{0}`.

### Failure record

Retain the requested count, emit mode, actual band set and first violating index. Capacity
differences are documented, not failures unless the prefix/gate contract breaks.

### Acceptance

Generated mode/wall-count coverage, capacity, degenerates, fatal spacing, exact domain errors
and all gate/gap mutations pass; no universal wall count is claimed.

## M-FUZZY-REPEATABILITY

Registry record: `id = "M-FUZZY-REPEATABILITY"`, `package = "fuzzy-skin"`,
`target_kind = "lib"`, `target_name = "fuzzy_skin"`,
`test_name = "property_tests::proptest_fuzzy_repeatability"`,
`production_symbol = "fuzzy_skin::apply_fuzzy_skin"`,
`production_path = "modules/core-modules/fuzzy-skin/src/lib.rs"`,
`features = ["fuzzy-skin/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-fuzzy-repeatability"`.

### Stable property ID

`M-FUZZY-REPEATABILITY` is a determinism pin for identical `apply_fuzzy_skin` arguments, not a
geometric correctness claim.

### Owner

`fuzzy-skin`, private `property_tests`
(`modules/core-modules/fuzzy-skin/src/property_tests.rs`), with the same stacked default-off
gates as the classic records. Controls: `fuzzy_repeatability_control` and
`fuzzy_shrink_preserves_modes_and_fixed_prefix`.

### Contract

Two calls with identical path, flags, profile, modes, parameters and layer/wall indices return
bit-identical points, flags and widths. The SUT seed derives from layer/wall indices; the
Proptest seed samples cases.

### Domain

Canonical-grid points with a fixed selected leading nonzero segment, a fixed unselected
following segment when `apply_to_all=false`, a shrinkable tail (including duplicate points),
parallel flags/widths, positive thickness/point distance and arbitrary u32 indices. Bounds:
`max_abs_mm_units = 20000`, `max_thickness_units = 3000`,
`max_point_distance_units = 8000`, `max_points = 8`.

### Strategy

Construction-first tail vectors; `apply_to_all` and open/closed topology are separate no-shrink
`prop::sample::select` strategies. A closed path carries its explicit final repeat.

### Oracle

Every point field, flag and width is compared by float bits, including signed zero. A changed
bit fails.

### Tolerance

None; bitwise equality.

### Shrinking

Modes cannot flip; generated tail flags/points/widths and ordinary parameters shrink as valid
inputs. The deterministic mixed witness carries the specific selected/unselected contrast.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`. Registry routine (64 cases) measured 0.04 s
and extended (256 cases) 0.12 s, with zero rejects. The 128-case pilot passed in 0.07 s; every
case called the SUT twice.

### Failure record

Retain both argument tuples and the first differing point/flag/width. Do not relax to
approximate equality.

### Acceptance

Repeatability property, one-bit mutation rejection, changed-layer sensitivity and seed-repeat
control pass. The `// test-quality:` determinism waiver is present.

## M-FUZZY-SPAN-PRESERVATION

Registry record: `id = "M-FUZZY-SPAN-PRESERVATION"`, `package = "fuzzy-skin"`,
`target_kind = "lib"`, `target_name = "fuzzy_skin"`,
`test_name = "property_tests::proptest_fuzzy_span_preservation"`,
`production_symbol = "fuzzy_skin::apply_fuzzy_skin"`,
`production_path = "modules/core-modules/fuzzy-skin/src/lib.rs"`,
`features = ["fuzzy-skin/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-fuzzy-span-preservation"`.

### Stable property ID

`M-FUZZY-SPAN-PRESERVATION` checks ordered span consumption and exact preservation of unselected
spans and original endpoint metadata.

### Owner

Same fuzzy owner file. Controls: `fuzzy_required_witnesses_are_nonempty`,
`fuzzy_span_oracle_rejects_mutations`, `fuzzy_disabled_parameters_are_identity`,
`fuzzy_property_domains_reject_unsafe_bounds` and the shrink probe.

### Contract

Every input entry is consumed once in order. An unselected segment and a selected zero-length
segment add no sample and copy endpoint point/flag/profile width exactly. A selected nonzero
segment adds at least one sample. Output arrays are parallel. Synthetic inserted overhang/flow
metadata is not claimed preserved.

### Domain

The repeatability domain with mixed, all-selected and all-unselected deterministic witnesses.
The five-point square uses flags `[true,false,true,true,true]`, widths `[0.4,0.5,0.4,0.45,0.4]`,
thickness 0.3 mm and point distance 0.8 mm; segment 1 is unselected and segment 3 closes.

### Strategy

Fixed `Just` selected/unselected prefixes survive shrinking; generated tail values remain
shrinkable. Modes are no-shrink selectors.

### Oracle

`match_fuzzy_output` is a memoized full-tail parser. It requires the first point exactly, then
for every segment either a direct endpoint (unselected/zero) or at least one inserted sample
followed by the endpoint (selected nonzero). Inserted flags/profile widths must equal a source
endpoint; endpoints and unselected metadata are bit-exact. Full-tail parsing disambiguates a
sample that rounds onto an endpoint.

### Tolerance

Bitwise equality for original endpoints/flags/widths and inserted flag/profile selection; no
width epsilon.

### Shrinking

Modes and fixed prefixes cannot flip in the shrink probe; ordinary tail length/points/widths
shrink and every state remains parseable with its closure invariant.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`. Registry routine (64 cases) measured 0.04 s
and extended (256 cases) 0.13 s; every case inserted samples on the fixed leading segment. A
selected zero-length segment was exercised in 33 routine and 130 extended cases. Both tiers had
zero rejects. The 128-case pilot passed in 0.07 s.

### Failure record

Retain the input path, flags, widths and first mismatched output index; classify SUT versus
matcher.

### Acceptance

Mixed/all-selected/all-unselected witnesses, unselected-point insertion/removal mutations,
disabled parameters and bound rejection pass.

## M-FUZZY-DISPLACEMENT

Registry record: `id = "M-FUZZY-DISPLACEMENT"`, `package = "fuzzy-skin"`,
`target_kind = "lib"`, `target_name = "fuzzy_skin"`,
`test_name = "property_tests::proptest_fuzzy_displacement"`,
`production_symbol = "fuzzy_skin::apply_fuzzy_skin"`,
`production_path = "modules/core-modules/fuzzy-skin/src/lib.rs"`,
`features = ["fuzzy-skin/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-fuzzy-displacement"`.

### Stable property ID

`M-FUZZY-DISPLACEMENT` independently measures each inserted sample against its source segment.

### Owner

Same fuzzy owner file. Control: `fuzzy_displacement_oracle_rejects_mutations`.

### Contract

Each inserted sample lies on the perpendicular offset of its source segment with magnitude at
most `thickness` within the derived envelope, and its longitudinal projection lies in the
source span within the derived envelope. Source identity comes from output order.

### Domain

The span-preservation domain with finite positive thickness/distance, all coordinates bounded
by `max_abs_mm_units`, the fixed selected leading nonzero segment and both no-shrink mode
selectors. Selected zero-length segments are excluded from displacement assertions.

### Strategy

Construction-first path/parameter families; the oracle widens f32 inputs to f64 so the SUT's
f32 arithmetic is the only source of the envelope.

### Oracle

`match_fuzzy_output` returns `InsertedSample { segment_index, output_index, point }`.
`fuzzy_measure_displacement` computes perpendicular distance and projection in f64.
`check_inserted_displacement` requires finite nonnegative distance, bounded magnitude and
projection in `[-tol_t, 1+tol_t]` with `tol_t = tolerance/segment_length + 8*f64::EPSILON`.

### Tolerance

`fuzzy_displacement_tolerance = K * f32::EPSILON * (max_abs_mm + thickness_mm + 1.0)` with
`K = 32`, derived from the actual f32 subtraction/squared-length/sqrt/normalize/base/displace/add
sequence and the f64 measurement. The derivation is recorded in the packet evidence; no
observed-output epsilon is used.

### Shrinking

The fixed leading selected segment and both mode selectors survive simplify/complicate. The
Step 5 shrink probe covers the same case family and retains closure and parseability.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`. Registry routine (64 cases) measured 0.04 s
and 1,221 inserted samples; extended (256 cases) measured 0.12 s and 8,460 samples; both had
zero rejects. The 128-case pilot passed in 0.07 s with 6,871 samples.

### Failure record

Retain input points, segment/output indices, measured distance/projection, tolerance and
derivation; classify SUT, generator/oracle or contract.

### Acceptance

Generated displacement and longitudinal checks pass; the deterministic closing edge has
positive displacement; over-thickness, beyond-span and wrong-segment mutations are rejected;
the tolerance derivation is recorded.

## M-RECTILINEAR-SCAN

Registry record: `id = "M-RECTILINEAR-SCAN"`, `package = "rectilinear-infill"`, `target_kind = "lib"`,
`target_name = "rectilinear_infill"`, `test_name = "property_tests::proptest_rectilinear_scan_intervals"`,
`features = ["rectilinear-infill/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`M-RECTILINEAR-SCAN` — semantic (complete scan-interval coverage of the rectilinear emitter),
independent of packet numbers.

### Owner

Package `rectilinear-infill`; lib target `rectilinear_infill`; test module `property_tests` in
`modules/core-modules/rectilinear-infill/src/property_tests.rs`; production symbol
`scan_expolygon` at `modules/core-modules/rectilinear-infill/src/lib.rs`. The kernel is private,
so the properties are a crate-internal unit-test module gated by `#[cfg(test)]` +
`#[cfg(feature = "property_testing")]`; production visibility is not widened. Module-source tests
are outside the `check-literals` and `check-test-quality` scanners
(`docs/21_data_defaults_and_fixtures.md` §7.1), so the file carries a manual FRU/waiver ledger.

### Contract

`scan_expolygon` scans one `ExPolygon` (contour + holes) with parallel lines at a positive
integer spacing, half-open in Y (include at `min_y`, exclude at `max_y`), and emits one two-point
`ExtrusionPath3D` per interior X interval per row. The property restricts itself to the exact
axis-aligned model: unit rotation (`cos_a = 1`, `sin_a = 0`), an integer `x_shift`,
`adjust_for_solid` both false and true, positive spacing and finite metadata. Rotation,
tangential vertex contact and the `align_to_grid` anchor are outside this exact model and are
observed separately by the degenerate/tangential controls.

### Domain

Construction-first integer fixtures on the canonical grid (1 unit = 100 nm): axis-aligned
rectangles, L-notches, U-notches and rectangles with one rectangular hole, plus a sub-spacing
rectangle whose positive height is below the passed spacing. Classes are selected with
`prop::sample::select(..).no_shrink()`; dimensions, origin, spacing, Z, speed factor, flow
factor, line width and solid/sparse/shift modes are generated. Every class is independently
validated with `oracles::validate_geometry` against a matching `GeometryDomain`, with coordinates
derived from the fixture edges rather than the scanner. Negative XY is a required sub-case.
Bounds are `max_abs_units`, `max_vertices`, `max_holes`, `max_components`,
`max_shrink_candidates`, `max_spacing_units`, `max_rows`; each is read through
`PropertyRun::bound`, so a missing or zero bound is an error, never a default.

### Strategy

One parameter draw produces a suite of cases (rectangle, sub-spacing rectangle, L-notch, U-notch,
holed rectangle, each with its solid/sparse and shift modes), so every class and mode is
exercised on every accepted call; per-class segment counters are asserted positive after `check`.
The half-open row grid is enumerated arithmetically from the bbox and cross-checked against an
independent `ceil(height / spacing)` count. Modes never switch during shrinking because they come
from non-shrinking selectors.

### Oracle

Independent of the SUT: rows are enumerated from the fixture bbox arithmetic, and each row's
interior X intervals are derived by probing the open cells of a **doubled** copy of the region
with `oracles::point_in_region`, so no probe can land on an edge (a boundary probe would be a
fixture error, not a silent Inside). The expected interval multiset, including omitted spans and
the translated region under `x_shift`, is compared element-wise against the emitted intervals;
every emitted segment is additionally checked with the boundary-inclusive
`oracles::segment_in_region`. The expected solid spacing comes from the
`M-RECTILINEAR-SPACING` reference, never from a call to the SUT. Role, both endpoints, Z, width,
flow, speed and `None` tool/lock are compared bit-exactly.

### Tolerance

Exact. All fixtures are on the canonical integer grid and the endpoint check requires
`mm_to_units(units_to_mm(x)) == x` bit-exactly, so no numeric tolerance applies. A non-grid
endpoint fails the property rather than being rounded into the model.

### Shrinking

`rect_shrink_bounds` probes the real local strategy with `TestRunner::deterministic` /
`Strategy::new_tree`, driving `current`, `simplify` and `complicate`, including a false simplify
return, repeated complicate calls until exhaustion, and revalidation of every exposed input and
retained fixture at every state. A false-only probe, a missing reduction from a nonminimal start,
or a lost class fails.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on
Windows in the registry lanes: 64 cases = 14.05 s of libtest (routine) and
256 cases = 53.85 s (extended). Every accepted call checks a suite covering all five shape classes and the shifted, solid and negative-XY modes.

Per-run case-mix counters and the retained-seed replay are recorded in
[`docs/testing/proptest-infill-evidence.md`](proptest-infill-evidence.md) § Step 8.

### Failure record

`failure.json` retains the classification, the full context, the concrete/debug case, the
tier-bearing reproduction command, the direct cargo reproduction with the consumed
`PNP_PROPERTY_CONTEXT`, captured SDK logs and the shrink status. A confirmed SUT defect belongs
in `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` as a named
concrete no-RNG regression, never a seed.

### Acceptance

The exact test is discovered and executed with `--exact --nocapture`; every expected interval and
every whole emitted segment is checked; per-class counters are positive; the exact
hole/notch/sub-spacing/empty/tangential controls pass; the mutation controls (removed interval,
filled hole, and a concavity or hole chord whose endpoints and midpoint are in material) are
rejected by the same checker. Defect outcome: **none confirmed** for the scanner by this
property.

## M-RECTILINEAR-SPACING

Registry record: `id = "M-RECTILINEAR-SPACING"`, `package = "rectilinear-infill"`,
`target_kind = "lib"`, `target_name = "rectilinear_infill"`,
`test_name = "property_tests::proptest_rectilinear_solid_spacing"`,
`features = ["rectilinear-infill/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`M-RECTILINEAR-SPACING` — semantic (canonical solid-spacing interval and cap arithmetic).

### Owner

Same package, target and module as `M-RECTILINEAR-SCAN`; production symbol
`adjust_solid_spacing` at `modules/core-modules/rectilinear-infill/src/lib.rs`.

### Contract

Canonical `Fill::_adjust_solid_spacing` (`src/libslic3r/Fill/FillBase.cpp`) computes
`number_of_intervals = coord_t((width - EPSILON) / distance)`, then
`distance_new = coord_t((width - EPSILON) / number_of_intervals)` (or `distance` when that count
is zero), and caps an over-limit interval when `distance_new / distance > 1.2` by returning
`coord_t(floor(distance * 1.2 + 0.5))`, with `EPSILON = 1e-4`. The kernel is non-validating:
`distance > 0` and `width >= 0` are caller contracts, so no division-by-zero branch is added.

### Domain

Positive `distance`, nonnegative bounded `width`, checked i64 arithmetic. Zero and too-large
bounds are rejected by the test-domain builder, never by the kernel. The generator draws four
named pair families per accepted call — width below distance, an exact interval boundary
(`width` an exact multiple of `distance`), a general truncation interval, and an over-cap pair —
with non-shrinking labels.

### Strategy

The four pair families come from checked bounds (`max_width_units`, `max_spacing_units`); the
over-cap family requires `distance` in
`MIN_CAP_DISTANCE_UNITS..=min(max_spacing_units, max_width_units / 2)` so the band is
admissible. Because all four families are present in one parameter draw, the property never
depends on a randomly selected mode.

### Oracle

An independent canonical reference on the integer grid: `adjusted = width - 1` (the exact integer
form of `width - EPSILON` for any `0 < epsilon < 1` unit), truncated interval count, truncated
interval distance, and the over-cap branch expressed exactly as
`distance_new * 5 > distance * 6` and `(distance * 12 + 5) / 10`. The equivalence of the integer
epsilon form to the source's `1e-4` is proved in the file header and was swept exhaustively over
`distance in 1..400`, `width in 0..200000` with zero divergences. The oracle never calls the SUT.

### Tolerance

Exact integer comparison; no tolerance.

### Shrinking

Probed with a real value tree over the four pair families, with simplify/complicate alternation
and revalidation of every pair at every state, plus explicit rejection of a pair outside the
checked profile and of `distance = 0`.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on
Windows in the registry lanes: 64 cases = 0.04 s of libtest (routine) and
256 cases = 0.13 s (extended). Every accepted call checks the below-distance, exact-multiple, truncation-interval and over-cap pair families.

Per-run case-mix counters and the retained-seed replay are recorded in
[`docs/testing/proptest-infill-evidence.md`](proptest-infill-evidence.md) § Step 8.

### Failure record

As `M-RECTILINEAR-SCAN`. Exact canonical cases asserted in `rect_controls`:
`(15000,10000) -> 12000`, `(12000,10000) -> 11999`, `(12001,10000) -> 12000`,
`(12002,10000) -> 12000`, `(10000,10000) -> 10000`, `(5000,10000) -> 10000`,
`(100000,4200) -> 4347`, `(100000,5000) -> 5263`, plus the additional boundary pairs listed in
the packet evidence.

### Acceptance

The exact test runs; all four families are nonempty at both tiers; the canonical over-cap witness
is reproduced and pinned. **Confirmed defect and focused fix:** the pre-fix kernel used bare
`width`, `.round()` and returned the requested `distance` on the over-cap branch. It now matches
the canonical arithmetic, and the no-RNG regression
`solid_spacing_over_cap_uses_canonical_1_2_factor`
(`modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs`) pins the
1.5 mm / 1.0 mm grid change. `D-209-ADJUST-SOLID-SPACING-DIVERGENCE` is closed.

## M-GYROID-PHASE

Registry record: `id = "M-GYROID-PHASE"`, `package = "gyroid-infill"`, `target_kind = "lib"`,
`target_name = "gyroid_infill"`, `test_name = "property_tests::proptest_gyroid_phase_and_grid"`,
`features = ["gyroid-infill/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`M-GYROID-PHASE` — semantic (gyroid branch evaluation and grid floor semantics).

### Owner

Package `gyroid-infill`; lib target `gyroid_infill`; module `property_tests` in
`modules/core-modules/gyroid-infill/src/property_tests.rs`; production symbols `gyroid_f`
(private) and `align_to_grid` (public) at `modules/core-modules/gyroid-infill/src/lib.rs`.

### Contract

`gyroid_f(x, z_sin, z_cos, vertical, flip)` inverts the gyroid implicit surface in normalized
units. Both `asin` terms are clamped into `[-pi/2, pi/2]`, so the vertical branch returns a value
in `[0, 2pi]` and the horizontal branch in `[-pi/2, 3pi/2]`; the `r < 1e-15` degenerate case
returns `PI` and `FRAC_PI_2` respectively. `align_to_grid(val, grid) = floor(val / grid) * grid`.

### Domain

Bounded finite `x` and phase `z` within one gyroid period (`max_phase_milliradians = 6284`);
`z_sin`/`z_cos` are derived from that same phase and the branch is the fill caller's own rule
`abs(sin) <= abs(cos)`, not a free choice. Positive finite grids and safe ratios
(`max_grid_units = 512`). Both orientation branches, both flip branches, positive and negative
phases, and periodic observations are required sub-cases. Reachability is recorded: with a
consistent phase and the caller's rule the unused component is at least `1/sqrt(2)`, so the
singular return is unreachable from generated cases and is covered by explicit controls instead.

### Strategy

Non-shrinking phase-band and flip selectors plus generated phase/x values and exact integer grid
cases. Periodicity is asserted only when `r >= 0.25` and the two compared arguments satisfy
`|u| <= 0.75`, which keeps the check off the clamp plateaus and off the singular early return.

### Oracle

Mathematical, not SUT-derived: the branch range from the clamped-`asin` argument, and for the
grid the exact integer model `val.div_euclid(grid) * grid` compared **bit-exactly** against
`align_to_grid` (bit-exactness argument: `grid <= 16384`, `|val/grid| <= 65`, and `1/grid` is far
above the f64 division error, so the quotient cannot cross an integer). The
truncation-toward-zero mutant is rejected by the same checker.

### Tolerance

`1e-12` for the phase range, over 500x the derived term-wise error (two clamped-`asin` terms at at
most 2 ulps each plus one addition), and `4e-12` for periodic observations against a derived
`~4e-13` bound. No observed-output epsilon. No implicit-surface residual is asserted: the file
records that no meaningful residual subdomain is proved, because the emitter's
swapped/offset/clamped coordinates, f32 stamp and grid expansion make the emitted-to-source
mapping non-injective.

### Shrinking

`gyroid_shrink_bounds` probes eight phase cases (both bands, both flips, grid cases) and five clip
cases with real `simplify`/`complicate` alternation, repeated complicate calls and revalidation;
the non-shrinking selectors are asserted to reject both directions.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on
Windows in the registry lanes: 64 cases = 0.04 s of libtest (routine) and
256 cases = 0.11 s (extended). Both orientation branches and both flip branches are required sub-cases; periodicity is asserted only away from the clamp plateaus.

Per-run case-mix counters and the retained-seed replay are recorded in
[`docs/testing/proptest-infill-evidence.md`](proptest-infill-evidence.md) § Step 8.

### Failure record

As the rectilinear records; the reproduction command and the direct cargo argv are retained in
`failure.json`. A confirmed defect's regression home is
`modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs`.

### Acceptance

The exact test runs; both branches and both flips are nonempty; the exact dyadic floor witnesses
(`-3 -> -4`, `-2 -> -2`, `+3 -> 2` at grid 2; `-9 -> -12`, `-8 -> -8`, `7 -> 4` at grid 4) are
bit-exact; the truncation mutant and the out-of-range/NaN phase mutants are rejected.

Recorded limitation: the generated phase cases check the output range and the periodic identity
against the SUT's own values, so a `gyroid_f` that always returned `PI` would pass every generated
case. The deterministic controls are what pin the actual branch values (`PI` and `FRAC_PI_2` at
the singular inputs, and the near-singular clamp), which is why they are mandatory rather than
decorative. Defect
outcome: **none confirmed**; no production change.

## M-GYROID-CLIP

Registry record: `id = "M-GYROID-CLIP"`, `package = "gyroid-infill"`, `target_kind = "lib"`,
`target_name = "gyroid_infill"`,
`test_name = "property_tests::proptest_gyroid_raw_wave_clipping"`,
`features = ["gyroid-infill/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`M-GYROID-CLIP` — semantic (raw wave emission plus the linker's prepared batch clip seam).

### Owner

Same package and module as `M-GYROID-PHASE`; primary production symbol
`GyroidInfill::fill_expolygon` (private) at `modules/core-modules/gyroid-infill/src/lib.rs`,
composed with `PreparedPolylineClip::{new,clip_batched}` at `crates/slicer-core/src/polygon_ops.rs`.

### Contract

`fill_expolygon` deliberately emits **raw, unclipped** waves (ADR-0025): clipping and connecting
belong to the linker. Raw output may extend beyond the outline, so this property never asserts
hole exclusion on the raw paths. It does assert finite coordinates, copied Z/speed, the
`SparseInfill` role, `None` tool/lock, and the source-stamped point width, which is the module's
`line_width`, **not** the per-call spacing width. The clipping half composes the real emitter
output with the real batch clip primitive the linker calls; it is not an end-to-end dispatch
claim (`M-LINKER-*` own that).

### Domain

`0 < density <= 1` (`min_density_ppm = 100000`), positive finite width
(`max_width_units = 6000`), bounded finite Z and controlled angles, controlled axis-aligned
rectangles with rectangular holes, plus rotated and dense modes (`max_abs_units`,
`max_vertices`, `max_holes`, `max_components`, `max_shrink_candidates`,
`max_phase_milliradians`, `max_wave_points`). The profile also carries a derived worst-case wave
envelope check, so an impossible `max_wave_points` fails at bound construction instead of during
generation.

### Strategy

Non-shrinking topology and rotation selectors with generated density, width, Z, speed factor and
angle. Legitimate empty modes (empty contour, nonpositive density or width) are separate
deterministic controls, as is a real wave set clipped against a far-away boundary, which
legitimately yields an empty result.

### Oracle

For the raw half: fixture arithmetic and source-stamped metadata only. For the clip half: the clip
universe is each contour inflated by exactly 1 unit and each hole contracted by 1 unit; each
subject segment's critical parameters (where `x` or `y` hits any inflated contour or contracted
hole edge) partition it, every open subinterval is classified at its exact `i128` rational
midpoint, and kept intervals are merged. Returned per-subject spans are mapped back to subject
parameter space and compared in **both** directions per segment, so a retained span the model does
not expect and a missing span the model expects are both failures. Every surviving segment is
additionally checked directly: no point outside contour + 4 units, and no segment entering a hole
deeper than `inflation + band`. No SUT clipper, offset or union routine contributes to the
expected result, and no flat output order is assumed.

### Tolerance

Linear allowance `CLIP_BAND_UNITS = 1.0` unit. The model already computes its expected intervals
on the inflated edge, and for these axis-aligned integer fixtures the pre-inflation lands exactly one
unit out, so the only deviation of a reported coordinate from the modelled edge is the clipper's
±1-unit integer intersection rounding. There is no separate f32 quantization term: the property
converts the raw waves to an integer `Point2` subject and the model consumes that same integer
subject. The returned-point projection onto the subject has a separate derived 2-unit tolerance used
only for attribution, never as a boundary allowance. The parameter
allowance is **per expected-interval endpoint and derived from the local crossing geometry**, not a
single uniform `band / length`:

- a subject endpoint (no clip-edge crossing) uses `band / length`;
- a crossing of a vertical or horizontal clip edge uses `band / |Δnormal|` for that edge
  (`|Δx|` on a vertical edge, `|Δy|` on a horizontal one).

The derivation: any displacement within the band of the modelled edge has a component
perpendicular to that edge and one along it. The perpendicular component `d_perp` moves the subject
parameter by at most `d_perp / |Δnormal|`, and the along-edge component `d_par` by at most
`d_par·cos θ / |Δnormal|`; the perpendicular bound dominates, so the term is `band / |Δnormal|`.
A perpendicular crossing (`|Δnormal| = length`) reduces exactly to `band / length`, and a shallow
crossing grows as `1/sin θ`. An earlier `(band/length)·max(1, cot θ)` form bounded only the
along-edge slide and missed the perpendicular component by `cos θ`; a shallow-crossing control
caught the ~0.008-unit shortfall. The term saturates at one full segment: beyond that the band
alone accounts for the whole segment, so the interval carries no discriminating power and is
excluded from the coverage direction; the file counts those saturated endpoints and the docstring
records why. Returned cut points are attributed to the **nearest** subject segment within a bounded
window of the first in-band hit, not to the first segment within the band, because near a vertex
that first hit is the incoming segment clamped at its endpoint and fabricates a vertex start.
Coverage is required outside each expected interval eroded by its own endpoint terms and
containment allows a returned span only inside each interval dilated by its own terms. A negative
control in `gyroid_controls` proves the term is finite: a shallow crossing accepts a cut exactly at
the derived term and still rejects one 5 units short through the coverage direction.

### Shrinking

`gyroid_shrink_bounds` covers the clip domain across density/width classes and clip topologies
with real simplify/complicate alternation (measured 19-22 simplifications and 13-16 repeated
complicate calls) and revalidation, plus the bound-rejection matrix.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on
Windows in the registry lanes: 64 cases = 0.46 s of libtest (routine) and
256 cases = 0.98 s (extended). The clip half requires a nonempty raw-wave witness, retained interior observations and at least one hole-crossing split; saturated endpoints are counted and printed.

Per-run case-mix counters and the retained-seed replay are recorded in
[`docs/testing/proptest-infill-evidence.md`](proptest-infill-evidence.md) § Step 8.

### Failure record

As the other records. A confirmed clip-stage defect's regression home is `crates/slicer-core/tests/`
for the primitive and `modules/core-modules/infill-linker/tests/` for the driver; the emitter's own
regressions stay in `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs`.

### Acceptance

The exact test runs; a nonempty raw-wave witness with out-of-outline points is observed; every raw
segment's metadata is checked; the full retained/missing subinterval comparison runs on real
clipped output with nonempty interior observations and at least one hole-crossing split; an
all-empty clip, an omitted interior span and a fabricated hole-crossing span are each rejected
through the intended mechanism. Defect outcome: **none confirmed**; no production change.

## M-LINKER-GRAPH

Registry record: `id = "M-LINKER-GRAPH"`, `package = "infill-linker"`, `target_kind = "lib"`,
`target_name = "infill_linker"`, `test_name = "property_tests::proptest_linker_ring_walks"`,
`features = ["infill-linker/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`M-LINKER-GRAPH` — semantic (boundary-graph ring geometry and connector/stub walks).

### Owner

Package `infill-linker`; lib target `infill_linker`; module `property_tests` in
`modules/core-modules/infill-linker/src/property_tests.rs`; production symbols
`BoundaryInfillGraph`, `BoundaryRing::directed_distance`, `contour_connector` and
`contour_stub` at `modules/core-modules/infill-linker/src/graph.rs`. The linker's submodules
are already `pub mod`, so the properties call the same seams the driver calls without widening
any API.

### Contract

`BoundaryInfillGraph::new` builds one closed ring per contour and per hole, contour first, with
global start positions tiling `[0, total_len)`. `BoundaryRing::directed_distance(from, to)`
returns the **shorter** forward or backward arc and its direction, ties resolving to `Forward`.
`contour_connector` returns only the intermediate points of a ring walk between two arc
positions, with Z and width lerped by arc fraction; `contour_stub` returns a walk of at most
`budget.min(ring.length)` from a position in a direction, with every point inheriting the
anchor's attributes.

### Domain

Rectangles, a concave notch, a rectangular hole and two separated components, with endpoints
constructed **on** known ring edges at least 3 units from any vertex (`VERTEX_MARGIN_UNITS`,
because production deliberately merges arc positions within `ARC_EPSILON_UNITS = 1`). An empty
graph is a distinct mode. Bounds: `max_abs_units`, `max_vertices`, `max_holes`,
`max_components`, `max_shrink_candidates`.

### Strategy

Non-shrinking shape-class selectors (rectangle, notch, hole, two components, empty) over an
axis-aligned integer `RingModel`, with generated cell sizes, origins and arc positions on ring
edges.

### Oracle

A small `RingModel` computes every expected value in exact `i64` arithmetic: perimeter, ring
start positions, the shortest forward/backward arc (tie to `Forward`), the corners a walk passes
and a stub's corners and end point. Hand-derived literal witnesses pin the model first — a
320,000-unit rectangle with one strictly shorter arc each way and one tie, a notch walk through
both reflex corners, a clockwise hole ring starting at 400,000, a second component at 120,000,
and an empty graph — and the real `BoundaryInfillGraph` structure, `directed_distance`,
`contour_connector` and `contour_stub` are then compared against it. Every connector and stub
segment must lie on a single expected ring edge, and endpoints are additionally checked with the
shared SDK `oracles::point_in_region`. `project`, `vertices_between` and the code's own walks are
never used to build an expected value. `BoundaryInfillGraph::project` is compared to the model as
well, but it is a public boundary-graph API with no production caller inside the linker, so that
comparison is API-surface evidence rather than driver-path evidence.

### Tolerance

Grid and interpolation envelopes derived from the operation sequence, not observed:
grid `2·(max_abs+1)·2⁻²⁴` and stub interpolation `16·(max_abs+1)·2⁻²⁴`.

### Shrinking

`linker_shrink_bounds` probes every ring class, connect class and chain/lock class with real
`simplify`/`complicate` alternation, repeated complicate calls, exhaustion, and revalidation of
the constructed ring and the real output at every state.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on
Windows in the registry lanes: 64 cases = 0.08 s of libtest (routine) and
256 cases = 0.24 s (extended). Every accepted call performs at least one query or empty-graph case and at least one stub check.

Per-run case-mix counters and the retained-seed replay are recorded in
[`docs/testing/proptest-infill-evidence.md`](proptest-infill-evidence.md) § Step 8.

### Failure record

`failure.json` keeps the classification, context, concrete case, reproduction command, direct
cargo argv and SDK logs. A confirmed defect's regression home is
`modules/core-modules/infill-linker/tests/graph_tdd.rs`.

### Acceptance

The exact test runs; shorter forward and backward arcs, a tie, reflex corner traversal, hole and
second-component walks, stubs and the empty graph are all exercised (deterministic witnesses
assert them nonzero on every run); a notch walk replaced by a chord, a same-length shortcut that
skips the reflex corners, a hole connector judged against the outer ring, outer-ring corners
offered as a hole connector, a stub missing its corner, and a nonempty model against the empty
graph are each rejected. Defect outcome: **none confirmed**; no production change.

## M-LINKER-CONNECT

Registry record: `id = "M-LINKER-CONNECT"`, `package = "infill-linker"`, `target_kind = "lib"`,
`target_name = "infill_linker"`,
`test_name = "property_tests::proptest_linker_connection_compatibility"`,
`features = ["infill-linker/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`M-LINKER-CONNECT` — semantic (source-path preservation and independent metadata compatibility at
the connector).

### Owner

Same package and module as `M-LINKER-GRAPH`; production symbol `connect_infill` at
`modules/core-modules/infill-linker/src/connect.rs`.

### Contract

`connect_infill(paths, graph, anchor)` keeps every eligible source path as a contiguous
ordered-or-reversed point/metadata subsequence of exactly one output, and joins two paths only
when they are on the same ring, their four compatibility clauses hold, and the shorter arc is
strictly shorter than `anchor_length_max`. The predicate is equal `role`, bit-equal
`speed_factor`, equal `tool_index`, and both corresponding first/last endpoint width differences
at most `0.000001` mm. Paths with fewer than two points are dropped only in the connecting branch;
`AnchorParams::dont_connect()` early-returns the input untouched. Ordinary interior endpoints are
not unanchorable: the live boundary-position search selects the nearest ring, so exact ring-walk
claims are restricted to constructed on-ring endpoints. `DEV-110`'s stub-residual algorithm is not
reproduced in the oracle.

### Domain

Distinguishable, nonzero, at-least-two-point paths with endpoints on known rings and widths on
both sides of the `1e-6` boundary; outer/hole and disconnected pairs; strict-anchor and
`dont_connect` modes; one-, two- and three-path classes. Bounds: `max_abs_units`, `max_paths`,
`max_points`, `max_anchor_units`.

### Strategy

Non-shrinking class selectors (merge, separated, cross-ring, mismatch family, strict anchor,
identity, short path, multi) over constructed rings and paths.

### Oracle

Each output path is scanned for every eligible source, matched point-for-point **bitwise**,
forward or reversed; every source must appear exactly once and no output may consist only of
added points. Points between two matched sources are a connector and must satisfy four
independent checks: both ends on the same ring in the `RingModel`; the two sources pass the
independently computed compatibility predicate (with widths compared exactly in `f64`, which is
exact in this width domain); the joining arc is strictly shorter than the anchor; and the points
equal the model's corner walk. Leading or trailing added points are stubs, checked on-ring and no
longer than the anchor (their exact length is deliberately not predicted because of `DEV-110`).
For two-path cases the model independently decides "must merge across distance `d`" or "must stay
separate". The SUT's own `project` result is compared to the model, never used to build it.

### Tolerance

Widths are compared exactly; the `1e-6` boundary is exercised at exactly `1e-6` (merge), 33 ulps
(merge) and `next_up(2c)` (reject). Grid and walk tolerances are the same derived envelopes as
`M-LINKER-GRAPH`.

### Shrinking

Covered by `linker_shrink_bounds` across the connect classes with revalidation of every
source/output at every state.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on
Windows in the registry lanes: 64 cases = 0.09 s of libtest (routine) and
256 cases = 0.31 s (extended). Every accepted call carries at least one eligible source path; per-mode populations are asserted by the deterministic witness.

Per-run case-mix counters and the retained-seed replay are recorded in
[`docs/testing/proptest-infill-evidence.md`](proptest-infill-evidence.md) § Step 8.

### Failure record

As `M-LINKER-GRAPH`; the regression home for a confirmed connector defect is
`modules/core-modules/infill-linker/tests/connect_tdd.rs` (or `anchor_length_tdd.rs` for anchor
arithmetic).

### Acceptance

The exact test runs; merges, separated pairs, cross-ring pairs, mismatch families, strict-anchor,
identity and short-path modes are all nonempty; a dropped source, an unmerged compatible pair, an
interior connector chord, fabricated role/speed/tool/first-width/last-width merges, a fabricated
outer-to-hole merge and a `dont_connect` output with its 1-point path removed are each rejected;
the real code passes the positive counterparts (exactly `1e-6` and 33 ulps merge, the 1-point path
survives `dont_connect` and is dropped when connecting, interior endpoints are still anchored).
Defect outcome: **none confirmed**; no production change.

## M-LINKER-CHAIN

Registry record: `id = "M-LINKER-CHAIN"`, `package = "infill-linker"`, `target_kind = "lib"`,
`target_name = "infill_linker"`, `test_name = "property_tests::proptest_linker_chain_and_origins"`,
`features = ["infill-linker/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`M-LINKER-CHAIN` — semantic (chain-only behaviour and ordered-region provenance at the real
orchestration seam).

### Owner

Same package and module; production symbols `chain_or_connect_infill` at
`modules/core-modules/infill-linker/src/connect.rs` and `orchestrate_infill` at
`modules/core-modules/infill-linker/src/orchestrate.rs`.

### Contract

`chain_or_connect_infill` reorders and reverses paths to minimize travel but must never add
connector geometry in chain-only mode (`dont_connect`). `orchestrate_infill` matches a prior
`InfillRegion` to a view on the exact ordered triple `(object_id, region_id, variant_chain)`; an
unmatched region passes its bucket through unchanged, an empty chain is a real value and not a
wildcard, wall groups are keyed by wall source plus the borrower's ordered chain, `order_lock`
paths bypass clip/link and are emitted first while their swept footprint is subtracted from the
boundary, and a union link additionally needs same object, same tool, same chain, same wall group
and pairwise path compatibility. The region tool and the authored path tool are separate checks.

### Domain

Nonempty at-least-two-point paths, both sides of `dont_connect`, full `(object_id, region_id,
ordered variant_chain)` identities, same-object/same-wall-group and separate-region/tool/role/width
fixtures, a legal priority/name-ordered `material` then `fuzzy_skin` chain and a distinct chain, a
deliberately mismatched and a deliberately reversed lookup, locked and unlocked paths, and
unequal plus tied source lengths. Bounds: `max_abs_units`, `max_paths`, `max_points`,
`max_regions`, `max_anchor_units`.

### Strategy

Non-shrinking mode selectors (chain-only, chain-connect, orchestrate union/separated, locked
bypass) with generated ring geometry, path roles/tools/widths and region identities. Every test
row has its own height and owning region, so provenance and majority ownership are measurable
without inferring from the first point.

### Oracle

Chain-only output must be a permutation of the inputs, each possibly reversed, with nothing added
or lost; chain-connect output goes through the `M-LINKER-CONNECT` checker. For `orchestrate_infill`
the test builds its own compatibility/ownership table (exact view match plus same object, region
tool, ordered chain, wall source, role, speed, authored tool and width) and checks four things:
each output's full identity exists and outputs come out in prior-region order; unmatched regions
pass through bit-identical; locked paths appear exactly once and are emitted first; each linked
path contains only rows from its own group, with only axis-aligned connectors inside the group's
area, and its owner is the group member with the most independently measured row length (ties to
the lower region id). Every row must be covered exactly once. Adjacency in the returned list is
never treated as extrusion between paths, and no global route optimality is asserted.

### Tolerance

Derived grid/interpolation envelopes as above; lengths and ownership compared from measured `i64`
row lengths, never from the first point. Orchestrate connectors are checked as axis-aligned and
inside the group area rather than as exact walks, because the offset boundary ring is not modelled
independently.

### Shrinking

Covered by `linker_shrink_bounds` across chain-only, chain-connect, orchestrate union/separated and
locked classes, revalidating every input and output at every state.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on
Windows in the registry lanes: 64 cases = 0.16 s of libtest (routine) and
256 cases = 0.37 s (extended). Every accepted case takes exactly one chain mode; per-mode populations are asserted by the deterministic witness.

Per-run case-mix counters and the retained-seed replay are recorded in
[`docs/testing/proptest-infill-evidence.md`](proptest-infill-evidence.md) § Step 8.

### Failure record

As `M-LINKER-GRAPH`; the regression home for a confirmed orchestration defect is
`modules/core-modules/infill-linker/tests/orchestrate_tdd.rs`.

### Acceptance

The exact test runs; chain-only reordering adds no connector geometry; connect-mode output passes
the full compatibility checker; every orchestrate row is covered exactly once with its owning
region's identity; locked paths bypass linking and are emitted first; pass-through regions are
bit-identical. Mutations rejected: a connector point added in chain-only mode, two outputs merged
in chain-only mode, an origin with the chain reversed or emptied, a merged path re-attributed to
the minority owner, and an altered reversed-chain pass-through region. Defect outcome: **none
confirmed**; no production change.

## M-WAVE-TRIM

Registry record: `id = "M-WAVE-TRIM"`, `package = "wave-overhangs"`, `target_kind = "lib"`,
`target_name = "wave_overhangs"`,
`test_name = "property_tests::proptest_wave_trim_and_coverage"`,
`features = ["wave-overhangs/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`M-WAVE-TRIM` — semantic (deposited-footprint trim containment and sampled coverage of the wave
generator).

### Owner

Package `wave-overhangs`; lib target `wave_overhangs`; module `property_tests` in
`modules/core-modules/wave-overhangs/src/property_tests.rs`; production symbol `generate` in the
private `crate::generator` module at `modules/core-modules/wave-overhangs/src/generator.rs`.
`WaveParams`, `WaveOutput`, `FallbackReason` and `EPSILON_MM` are `pub(crate)`, so the properties
are a crate-internal unit-test module and no API is widened.

### Contract

For the declared positive-flow domain, let `r = (flow_width * flow_ratio) / 2` be the deposited
radius (required `>= EPSILON_MM`), `P` the independently derived original rectangular wave piece
before the first trim, and `T = P minus disk(r)` the centerline trim. Every output centerline
endpoint must lie in `T`, and every whole deposited capsule (both endcaps included, radius `r`)
must lie in `P`. The capsule is **never** required to fit in `T`; doing so applies the inset
twice and false-rejects a legitimate boundary-tangent output. Because `T` is rectangular and
therefore convex, endpoint containment proves the whole straight centerline lies in `T`. `filled`
is a regularized union and is never used as a footprint. Fallback reasons are a repeatable vector
and coexist with nonempty `paths`; fallback is not equivalent to empty output.

### Domain

A controlled axis-aligned bridge rectangle with analytically specified supporting bands,
`perimeter_overlap = 0`, `minimum_width = 0` (no narrow split), `min_length = 0` and an unbounded
iteration class, with generated line spacing, flow width, flow ratio, pattern and iteration bound.
Every parameter that would change `P` (`perimeter_overlap`, `minimum_width`, `min_length`,
`base_spacing`) is held fixed inside the property. Bounds are `max_abs_units`,
`max_spacing_units`, `max_width_units`, `max_iterations`, `max_cells`.

### Strategy

The fixture is the existing seam geometry adapted to a direct `generate` call: `P = [0,6] x [0,6]`
mm with support bands extended 0.5 mm into the bridge (the plan's edge-flush translation produces
no wave because `generate` clips `lower` to the fill bbox before deriving anchors; measured in the
packet evidence). `r = (0.4 * 1.875) / 2 = 0.375` mm and `T = [0.375, 5.625]^2` mm. The nonempty
no-fallback witness was established by execution before any neighborhood was generalized. Pattern,
iteration class and full/capped modes are non-shrinking selectors.

### Oracle

`P` and `T` are derived analytically from the fixture and the input parameters, never from the
SUT. Containment checks the centerline against `T` and the radius-`r` capsule against `P` with
exact segment-to-boundary distance arithmetic. Coverage is an independently computed
point-to-segment/capsule-distance lattice over `P` (96x96 cell centres, 0.0625 mm pitch, 9,216
cells) with a boundary-connected empty-cell model: a cell farther than `r + 0.1` mm from every
centerline is empty, and only empty cells not 4-connected to the lattice border count as interior
gaps. `0.1` mm is half the existing seam fixture's 0.2 mm defect width. This is labelled sampled
coverage, never a proof of arbitrary continuous area; guaranteed detection is for axis-aligned
voids at least 0.2625 mm wide and one pitch long. The determinism half calls `generate` twice on
identical inputs and compares every point, filled contour/hole and fallback in order as an
explicit `docs/22_test_quality.md` determinism pin, not a correctness oracle.

### Tolerance

`CLIP_BAND_UNITS = 2` (documented `PreparedPolylineClip` ±2-unit band) + `TRIM_QUANTIZATION_UNITS
= 2` (integer snap of the integer-unit trim offset) = 4 units = 0.0004 mm, plus a `1e-9` mm
rational comparison epsilon and the same 4-unit outward band on the capsule. The derived total is
625x smaller than the 0.25 mm gap the nominal-radius negative control relies on, so it cannot mask
that control, and the exact tangent control proves the checker is not double-inset.

### Shrinking

`wave_shrink_bounds` probes six mode x pattern combinations with real `simplify`/`complicate`
alternation (measured 26-32 simplifications with matching restores, `exhausted = true`),
revalidating every exposed input and the generated output at every state, plus the
bound-rejection matrix.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on
Windows in the registry lanes: 32 cases = 21.12 s of libtest (routine) and
128 cases = 95.28 s (extended). This is the expensive record in the family (lattice coverage over every emitted segment), which is why its case counts are lower than the other eight.

Per-run case-mix counters and the retained-seed replay are recorded in
[`docs/testing/proptest-infill-evidence.md`](proptest-infill-evidence.md) § Step 8.

### Failure record

`failure.json` retains the classification, context, concrete case, reproduction command, direct
cargo argv and captured SDK logs. Confirmed defects belong in
`modules/core-modules/wave-overhangs/tests/wave_overhangs_tdd.rs` as named no-RNG regressions.

### Acceptance

The exact test runs; every positive-run centerline is checked against `T` and every whole capsule
against `P`; the exact `P = [0,10]^2`, `r = 1`, `T = [1,9]^2`, `(1,2) -> (1,8)` tangent case is
accepted while a `capsule subset T` checker rejects it; the nominal-width `(0.75,2) -> (0.75,8)`
case is rejected at the deposited radius and accepted at the mistaken nominal radius; a changed
repeat, a removed central wave (692 and 39 interior gap cells in the two controls) and a false
fallback are each detected; the empty-area, min-length, missing-anchor and one-iteration fallback
fixtures are observed with `paths`/`filled`/`fallbacks` coexisting where specified. Defect
outcome: **none confirmed**; no production change.

## M-IRONING-ZIGZAG

Registry record: `id = "M-IRONING-ZIGZAG"`, `package = "top-surface-ironing"`,
`target_kind = "lib"`, `target_name = "top_surface_ironing"`,
`test_name = "property_tests::proptest_ironing_full_paths"`,
`features = ["top-surface-ironing/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`.

### Stable property ID

`M-IRONING-ZIGZAG` — semantic (complete scan-interval coverage plus whole-path containment of the
ironing emitter).

### Owner

Package `top-surface-ironing`; lib target `top_surface_ironing`; module `property_tests` in
`modules/core-modules/top-surface-ironing/src/property_tests.rs`; production symbol
`generate_zigzag_strokes_for_polygon` (private) at
`modules/core-modules/top-surface-ironing/src/lib.rs`. `bbox_of_expoly`, `BBox2D` and the
`IRONING_LINE_WIDTH` constant are crate-visible, so the property checks the same values the
module stamps.

### Contract

For each scan row at a positive spacing, the kernel derives the row's interior X intervals from
the contour and every hole with a half-open Y rule (`[y_lo, y_hi)`), drops intervals of width
`<= 1e-4`, and emits `Ironing` paths at planar Z with `flow_factor = ironing_flow / 100`,
`speed_factor = ironing_speed / 50`, the module's fixed `IRONING_LINE_WIDTH`, and `None`
tool/lock. Consecutive scan rows alternate direction. A path continues across rows, and between a
row's intervals, only while the joining move stays inside the region (exact segment-in-region
test); a move that would leave the material — across a hole, a notch, or a concave/slanted
boundary — starts a new path instead. **Every consecutive point pair of every returned path is an
extruded segment**, including the implicit joins between intervals in one row and between rows;
evaluating only `chunks(2)` misses exactly those joins.

### Domain

Independently validated axis-aligned rectangles, U-notches and rectangles with one rectangular
hole on an exact 0.25 mm fixture grid, so every generated coordinate and spacing converts to
`f32` millimetres without rounding and `y_min + i * spacing` stays exact. Spacings are 0.5-1.5 mm;
the cut's bottom edge is placed exactly on an interior scan row, so every generated hole or notch
case really splits a row. Empty contour, zero-height, zero-width, nonpositive spacing and a
positive sub-spacing span are separate deterministic modes. Bounds are `max_abs_units`,
`max_vertices`, `max_holes`, `max_components`, `max_shrink_candidates`, `max_spacing_units`,
`max_rows`; `max_rows` is constrained to `4..=64`, because the derived `f32` envelope assumes at
most 64 rows and at least four rows are needed to place a gap that a scan row crosses.

### Strategy

Non-shrinking shape selector with generated dimensions, origin, cut placement, spacing, Z, flow
and speed. Each accepted call checks the complete point sequence against the independent interval
model, so a missing row, an extra segment or a reordering is a failure. Per-shape counters
(rect/notch/hole), whole-path segment counters and a split-row counter are asserted positive after
`check`.

### Oracle

The expected row grid is `floor(span / spacing) + 1` half-open rows from the bbox, and the expected
X intervals are derived analytically from the fixture edges and the cut (rect: one interval; notch
from `y >= notch_y`: two; hole for `hole_y0 <= y < hole_y1`: two), reversed on odd rows. The full
expected point sequence is compared element-wise with a derived envelope. Every consecutive
segment of every returned path is additionally checked with the boundary-inclusive
`oracles::segment_in_region` against the real `ExPolygon`, which is what catches an implicit join
crossing a gap. Metadata is compared bit-exactly (Z, width, flow, speed, role, `None` tool/lock).

### Tolerance

`64 * f32::EPSILON * (max_abs_mm + span_mm + 1.0)` for the point comparison, derived from the
documented operation sequence (one unit-to-mm rounding plus at most 64 exact-grid row additions)
and twice the largest allowed row count; the fixtures are exact, so the observed deviation is
zero. Containment is exact integer arithmetic through `Point2::from_mm`.

### Shrinking

`ironing_shrink_bounds` probes all three shapes with real `simplify`/`complicate` alternation
(measured 154 simplifications per shape), revalidating geometry, the expected model and the real
output at every state, then exercises a false simplify return and repeated complicate calls to a
stable false state, and asserts a real reduction from a nonminimal start. The shape selector is
asserted to reject both directions.

### Execution

Both tiers, in-process, chacha, `timeout_ms = 0`, zero rejection budgets. Measured on
Windows in the registry lanes: 64 cases = 0.53 s of libtest (routine) and
256 cases = 1.98 s (extended). Every accepted call checks the complete point sequence and every whole-path segment.

Per-run case-mix counters and the retained-seed replay are recorded in
[`docs/testing/proptest-infill-evidence.md`](proptest-infill-evidence.md) § Step 8.

### Failure record

`failure.json` retains the classification, context, concrete case, reproduction command, direct
cargo argv and captured SDK logs. A confirmed defect's regression home is
`modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` as a named
concrete no-RNG test.

### Acceptance

The exact test runs; the deterministic 8 mm square with hole `[3,5] x [2,6]` mm at 1 mm spacing
produces nonempty rows on both sides of the hole with no segment through its interior; a U-notch
retains both arms; every segment of every path is in-region; a gap-connector merge, a dropped row,
a false-empty output and a mutated flow factor are rejected by the same checker. Two slanted
controls pin the join rule: a right triangle keeps a single snake (the joins lie on the
hypotenuse), and a concave notch whose kink sits inside one row spacing splits into more than one
path, both with every segment in-region. **Confirmed
defect and focused fix:** the pre-fix kernel emitted one `ExtrusionPath3D` per polygon by
concatenating all rows' intervals, so the join `(5,3) -> (3,3)` mm (both endpoints on material)
crossed the hole. The kernel now starts a new path whenever the next interval does not start at
the x the previous interval ended on, so every implicit join is a vertical move at a shared x on
the outer boundary. The no-RNG regression
`hole_crossing_regression_no_ironing_segment_enters_the_hole` fails on the old kernel. A
determinism-pin exception is not used by this property (its checks are independent), and the
current one-path-per-polygon shape is no longer pinned by any test.

## M-SUPPORT-IRONING

Registry record: `id = "M-SUPPORT-IRONING"`, `package = "support-surface-ironing"`,
`target_kind = "lib"`, `target_name = "support_surface_ironing"`,
`test_name = "property_tests::proptest_support_ironing_fill"`,
`production_symbol = "support_surface_ironing::SupportSurfaceIroning::fill_expolygon"`,
`production_path = "modules/core-modules/support-surface-ironing/src/lib.rs"`,
`features = ["support-surface-ironing/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-support-ironing"`.

### Stable property ID

`M-SUPPORT-IRONING` — semantic (complete half-open row/material-interval coverage plus metadata
propagation for the support-surface ironing fill).

### Owner

Package `support-surface-ironing`; lib target `support_surface_ironing`; module `property_tests` in
`modules/core-modules/support-surface-ironing/src/property_tests.rs`; production symbol
`SupportSurfaceIroning::fill_expolygon` (private) at
`modules/core-modules/support-surface-ironing/src/lib.rs`, whose only production caller is
`SupportSurfaceIroning::run_support_postprocess`. The module is constructed through `from_config`
so the property sees the same field values the module would.

### Contract

Rows are `min_y + n * line_spacing` while below `max_y`, with no half-spacing offset. At each row
the kernel gathers crossings from the contour and every hole ring with a half-open lower-inclusive
rule, rounds each X to the nearest canonical unit, sorts, pairs even-odd, and skips a pair whose
endpoints coincide. Each surviving pair becomes one `Ironing` path with the supplied Z, the
configured `line_width`, the configured `ironing_flow_rate` as `flow_factor`, `dist_to_top_mm = 0.0`
and `None` overhang and lock fields. The kernel does not compute `speed_factor`; the caller passes
`ironing_speed / 50` and it is propagated verbatim. Nonpositive spacing and degenerate geometry may
return empty. This is a metadata-propagation contract: it does not claim the manifest default's
numeric `ironing_flow_rate` is an independently verified physical percentage.

### Domain

Independently validated rectangles, U-notches and rectangles with one rectangular hole on an exact
0.25 mm grid, with 1 mm spacing and generated origins. A holed `[0,10]^2` mm fixture with hole
`[4,6]^2` and 1 mm spacing has nonempty material intervals on both sides at an interior hole row.
Separate deterministic modes cover a nonpositive spacing and a degenerate ring. Bounds are
`max_abs_units`, `max_vertices`, `max_holes`, `max_components`, `max_shrink_candidates`,
`max_spacing_units`, `max_rows`.

### Strategy

Nonshrinking shape-mode selector with generated dimensions, origin, cut placement, spacing, Z, flow
and speed. Deterministic positive fixtures for the rectangle, notch and hole modes run inside every
callback. Every `Cell` counter is asserted positive after `check`: `calls`, the five witness
counters `w_charter` (holed `[4,6]^2` split on both sides), `w_asym` (asymmetric hole split),
`w_notch`, `w_square` and `w_empty` (degenerate inputs), plus `strokes`, `segments` and
`split_rows`. A run in which the callback never executed, or in which any witness counter stayed
zero, fails.

### Oracle

The expected rows and material intervals are derived from the same rectangle arithmetic the
contract states — the bbox minimum, the spacing steps, and the input edges — written once and
reused by both the property and the controls. Every consecutive point pair of every path is checked
with the boundary-inclusive `oracles::segment_in_region` against the self-built `ExPolygon`, and all
metadata is compared exactly against the supplied configuration values.

### Tolerance

Exact. Every generated coordinate and spacing is a multiple of 0.25 mm, so `mm_to_units` and
`units_to_mm` round-trip and each crossing is an exact integer unit.

### Shrinking

`support_ironing_shrink_bounds` drives the real strategy through
`current`/`simplify`/`complicate`, revalidates the ring structure and the shape mode at every
state, requires a permitted reduction, and exercises a false `simplify` return and repeated
`complicate`.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, zero rejection budgets. Registry command
`cargo xtask test --summary --properties --property M-SUPPORT-IRONING --tier routine|extended`.
Measured timings are in the evidence file.

### Failure record

Standard `PropertyRun` artifacts; no confirmed defect, so no committed regression.

### Acceptance

Maps to AC-5, AC-8 and AC-N1. `support_ironing_controls` accepts the valid positive observation and
then rejects, with the same checker: a single unsplit `(1,5) -> (9,5)` mm segment across the
asymmetric hole of `[0,10]^2` minus `[2,3] x [4,6]` (both endpoints and the midpoint are material);
the U-notch chord `(2,5) -> (8,5)` on `[0,10]^2` minus `[4,6] x [4,10]`; an otherwise correct
interval set with the interior `y = 1` row deleted from a 10 mm square at 1 mm pitch; and a
replacement of all output with empty. The metadata control also rejects a mutated `flow_factor`,
width, Z or speed.

## M-TRADITIONAL-SCAN

Registry record: `id = "M-TRADITIONAL-SCAN"`, `package = "traditional-support"`,
`target_kind = "lib"`, `target_name = "traditional_support"`,
`test_name = "property_tests::proptest_traditional_support_fill"`,
`production_symbol = "traditional_support::TraditionalSupport::fill_expolygon"`,
`production_path = "modules/core-modules/traditional-support/src/lib.rs"`,
`features = ["traditional-support/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-traditional-scan"`.

### Stable property ID

`M-TRADITIONAL-SCAN` — semantic (complete half-open row and material-interval coverage plus
whole-segment containment of the traditional support fill kernel, with separated fallback and
no-work modes).

### Owner

Package `traditional-support`; lib target `traditional_support`; module `property_tests` in
`modules/core-modules/traditional-support/src/property_tests.rs`; production symbol
`TraditionalSupport::fill_expolygon` (private) at
`modules/core-modules/traditional-support/src/lib.rs`, whose only production caller is
`TraditionalSupport::run_support`. The module is constructed through `from_config` with the
manifest's seeded defaults.

### Contract

The kernel rotates the polygon about its unrotated bbox centre, scans rows at
`y0 + n * spacing` while below `y1`, rounds each crossing X to the nearest canonical unit, sorts,
pairs even-odd, and drops a pair whose endpoints coincide. Each surviving pair becomes one
`ExtrusionPath3D` with `role = SupportMaterial`, the supplied Z and `speed_factor`,
`flow_factor = 1.0`, the supplied path width and unset `tool_index`/`order_lock`/`overhang`
fields. The **caller**, not this kernel, changes interface roles. When the scan loop emits nothing
the centroid fallback may emit one band per centroid row, translated back by the same
bbox-centre offset the scan branch adds; when that also emits nothing and the caller flagged the
region as an interface, the contour is emitted as a closed path so the layer stays printable. A
nonpositive spacing or a degenerate ring returns empty.

### Domain

Independently validated axis-aligned rectangles, an open-to-top U-notch and a rectangle with one
rectangular hole on an exact 0.25 mm grid. Generated cases carry `1..=max_rows` rows, a spacing that is
a grid multiple and at most `span / 4`, and a cut placed on a row boundary so at least one row is
really split; a cut-carrying shape always has at least two rows (a notch) or three (a hole, which
also covers the rotated mode), so its cut always has real height above it at every shrink state.
Rotated cases carry the cut only on the 180-degree turn, because a quarter turn maps the cut's x
extent onto the scan axis and a y-aligned cut then only lands on scan-row boundaries at half turns;
the odd turns exercise the axis swap alone. Rotated cases use exact quarter turns only, so the derived rounding envelope is
zero. Separate deterministic modes cover a translated narrow tip (the centroid fallback), the
sub-pitch diamond at 2 mm pitch (the interface-contour fallback) and six degenerate classes
(zero height, zero width, a two-point ring, an empty contour, zero spacing, negative spacing).
Bounds are `max_abs_units`, `max_vertices`, `max_holes`, `max_components`,
`max_shrink_candidates`, `max_spacing_units`, `max_rows`.

### Strategy

Nonshrinking shape selector (`Rect`, `Notch`, `Hole`, `Rotated`, `Degenerate`) over a total
generator, so the run has zero local and global rejects. Every generated case is revalidated
against the live bounds and `validate_geometry` before the kernel is called. Each callback also
re-checks five deterministic witnesses: the charter's holed `[0,10]^2` mm fixture with hole
`[4,6]^2` at 1 mm spacing, the asymmetric `[2,3] x [4,6]` hole, the U-notch, the 10 mm square, and
the six degenerate classes. Counters `rect/notch/hole/rotated/degenerate`, `strokes`, `segments`,
`rows` and `split_rows` are asserted positive after `check`.

### Oracle

Angle-zero expectations are rectangle arithmetic over the fixture's own fields: the half-open rows
are `y0 + n * spacing < y1`, and each row's intervals are the outer rectangle minus the cut, with
the cut active on `[cy0, cy1)` for a hole and on `[cy0, y1)` for a notch. The complete sorted
`(row, x_start, x_end)` set is compared, so a missing row, an extra row, a reordered interval or a
merged arm is a failure. Rotated modes are mapped back into the kernel's own bbox-centre-relative scan frame and compared
against the **same complete row-and-interval model** there, which quarter turns keep axis-aligned.
Because the turn is exact, this is an exact integer comparison rather than a row-count proxy, so a
split row that loses one arm fails. Every consecutive point pair of every path — including a closed ring's closing edge
— is checked with the boundary-inclusive `oracles::segment_in_region`.

### Tolerance

Exact. Every fixture coordinate and spacing is a multiple of 0.25 mm, each output coordinate is
required to recover to a whole canonical unit bit-exactly, and the quarter-turn rotation is an
integer permutation. A coordinate that does not round-trip is a failure, not a widened allowance.

### Shrinking

`traditional_renderer_shrink_bounds` probes all five modes with real `simplify`/`complicate`
alternation, revalidating the bounds, the row count, the cut containment and the shape mode at
every state, then exercises a false `simplify` return and repeated `complicate`, and asserts a
real reduction from a nonminimal start. The mode selector is asserted to refuse both directions.

### Execution

Both tiers run `in_process` with `timeout_ms = 0`, chacha, zero rejection budgets and total
generators. The registry-backed command is
`cargo xtask test --summary --properties --property M-TRADITIONAL-SCAN --tier routine|extended`.
Measured Windows timings and the retained-seed replay are in
[`docs/testing/proptest-support-modules-evidence.md`](proptest-support-modules-evidence.md).

### Failure record

`failure.json` retains the classification, context, concrete case, reproduction command, direct
cargo argv, captured SDK logs and shrink status. One confirmed defect and one hardening change live here, each with a
named concrete no-RNG regression in the crate's `#[cfg(test)] mod tests`:

- **Confirmed defect** — `narrow_translated_tip_fallback_is_placed_at_the_bbox_centre_regression`:
  the centroid fallback rotated into bbox-centre-relative coordinates but never added the
  translation back, so on the triangle `(5,4), (6,4.5), (4,4.5)` mm it emitted
  `(-0.5, 0) -> (0.5, 0)` mm instead of `(4.5, 4.25) -> (5.5, 4.25)` mm. The packet required
  reconciling whether the fallback is required before touching it; two independent live branches
  (the centroid band and the interface-contour fallback) exist to keep a non-empty layer printable,
  so the fallback is intended behaviour and the fix restores its contract. The
  `D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE` wording that says the fallback was removed is stale
  relative to the code; correcting that row is a separate authorised action.
- **Degenerate-input hardening** — `two_point_ring_emits_no_zero_length_fallback_regression`: the
  same fallback pushed a coincident crossing pair unconditionally, so a two-point ring emitted one
  zero-length stroke at `(0.5, 0.5)` mm. A two-point ring is not valid geometry, so no reachable
  production loss was demonstrated; the guard mirrors the invariant the scan branch already
  enforces and has no effect on non-degenerate input.

### Acceptance

Maps to AC-5, AC-8, AC-N1 and AC-N2. A holed `[0,10]^2` mm fixture with hole `[4,6]^2` and 1 mm
spacing emits `(0,4)` and `(6,10)` mm on the interior hole row `y = 5` mm.
`traditional_renderer_controls` accepts the valid positive observation and then rejects, with the
same checker: a single unsplit `(1,5) -> (9,5)` mm segment across outer `[0,10]^2` minus
`[2,3] x [4,6]` (both endpoints and the midpoint are material), the U-notch chord
`(2,5) -> (8,5)`, an otherwise correct interval set with the interior `y = 1` row deleted, a row
on the excluded top edge `y = 10`, a replacement of all output with empty, one arm of a rotated
split row dropped, the untranslated centroid band, a sub-pitch diamond with no interface contour,
and output injected into any of the six degenerate classes.

## M-TREE-SMOOTH

Registry record: `id = "M-TREE-SMOOTH"`, `package = "tree-support-planner"`, `target_kind = "lib"`,
`target_name = "tree_support_planner"`,
`test_name = "property_tests::proptest_tree_smoothing_graph"`,
`production_symbol = "tree_support_planner::smooth_nodes"`,
`production_path = "modules/core-modules/tree-support-planner/src/lib.rs"`,
`features = ["tree-support-planner/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-tree-smooth"`.

### Stable property ID

`M-TREE-SMOOTH` — semantic (pinned-end branch relaxation, short-chain preservation and the
source-backed merge extra-wall trigger).

### Owner

Package `tree-support-planner`; lib target `tree_support_planner`; module `property_tests` in
`modules/core-modules/tree-support-planner/src/property_tests.rs`; production symbol
`smooth_nodes` (private) at `modules/core-modules/tree-support-planner/src/lib.rs`, called by
`SupportPlanner::plan_for_object` after pruning and before drawing and emitting. The fixtures build
a private `NodeArena`, `LayerRecord`s and real parent/child/active links; no production visibility
is widened.

### Contract

`SMOOTH_NODES_ITERATIONS` passes of a three-point average over each chain's interior nodes, with
the head and tail held fixed, then a single `round()` on commit. On the simple single-chain
subdomain: fixed endpoints keep position, radius and movement; chains shorter than three nodes are
untouched; committed interior positions stay inside the coordinate-wise hull of the original
chain. A committed interior node with more than one parent sets `need_extra_wall`, and the
canonical end-of-chain interpolation then promotes a node bracketed by two extra-wall nodes. The
width-derived half-line value is a **signed component threshold for the flag**, not a cap on
`movement` and not a guarantee about `max_move_dist`; neither is asserted.

`DEV-143` is preserved: the port relaxes in `f64` scaled units and rounds once where canonical
truncates each pass, so no bit-exact upstream equality is claimed and no 100-pass reimplementation
is used as the expected answer. `DEV-178` records that no renderer consumes the flag, so this
charter asserts the flag on the node and makes no claim about rendered walls.

### Domain

Bounded valid graphs: a straight equally spaced chain (checked as an exact fixed point, position
equal to the original and movement equal to the step), a bent chain with at least three nodes, a
merge node with two parents, a polygon-typed child, and a chain shorter than three nodes, each with
valid `active`, `parent`, `child` and `parents` links. Every chain stays under the tall-branch
threshold, so that arm is out of the domain and is not asserted either way. Bounds are
`max_abs_units`, `max_nodes`, `max_layers`, `max_width_units`; `max_layers` is constrained to
`5..=256`, `max_nodes` to at least six, and `max_width_units` to an even value in `4000..=20000`.

### Strategy

Nonshrinking selector over the straight, bent, merge, polygon-child and short-chain modes with a
total generator, so the run has zero rejects. The deterministic positive witness for each mode is
built and checked inside the callback. Counters for accepted calls, interior nodes, straight-chain
interior nodes, short-chain nodes, flagged nodes and the five witness classes are asserted positive
after `check`.

### Oracle

Graph- and flag-based, derived from the contract: pinned endpoints compared field-by-field against
their pre-call values, short chains compared field-by-field, interior positions checked for
coordinate-wise containment in the original chain's per-axis integer hull with **zero** extra
rounding allowance (the single `round()` cannot leave an integer hull while the `f64` error stays
under half a unit), every reference re-inspected after the call, and the `need_extra_wall` trigger
checked from the source-backed condition — `parents.len() > 1`, a signed component of `movement`
greater than `mm_to_units(line_width / 2)`, or the tall-branch rule — never from the kernel's own
output.

### Tolerance

Exact integers. The interior hull is an integer interval and the only rounding is the single
commit `round()`, whose error is bounded below one unit for the fixture grid; the header records
that derivation.

### Shrinking

`tree_planner_shrink_bounds` drives the real strategy, revalidates every exposed arena and layer
record at every state, requires a permitted reduction, and exercises a false `simplify` return and
repeated `complicate`.

### Execution

Both tiers run `in_process` with `timeout_ms = 0`, chacha, zero rejection budgets and total
generators. Registry command
`cargo xtask test --summary --properties --property M-TREE-SMOOTH --tier routine|extended`.
Measured on Windows: 64 cases in 0.07 s and 1024 cases in 1.32 s of libtest. Provenance and the
full counter mix are in
[`docs/testing/proptest-support-modules-evidence.md`](proptest-support-modules-evidence.md).

### Failure record

Standard `PropertyRun` artifacts; no confirmed defect, so no committed regression.

### Acceptance

Maps to AC-3, AC-8 and AC-N1. `tree_planner_controls` accepts a valid straight chain and then
rejects, with the same checker and its exact failure class: a pinned endpoint mutated to a
different grid point, a `parent` or `parents[0]` index set to the arena length (a checker-only
mutation — the kernel indexes its arena directly and promises no validation error), a rewired link,
an interior node placed outside the hull, a broken straight-chain fixed point, the cleared merge
flag, a spurious flag on a negative movement, and a changed short chain. The callback requires
nonempty node and chain observations, and a zero-node run fails.

## M-TREE-ROLES

Registry record: `id = "M-TREE-ROLES"`, `package = "tree-support-planner"`, `target_kind = "lib"`,
`target_name = "tree_support_planner"`,
`test_name = "property_tests::proptest_tree_carve_and_roles"`,
`production_symbol = "tree_support_planner::build_roles_batched"`,
`production_path = "modules/core-modules/tree-support-planner/src/lib.rs"`,
`features = ["tree-support-planner/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-tree-roles"`.

### Stable property ID

`M-TREE-ROLES` — semantic (per-region collision carve, serial and production batched role assembly,
and the batched-only own-territory intersection).

### Owner

Package `tree-support-planner`; lib target `tree_support_planner`; module `property_tests`;
production symbols `carve_emitted_regions` and `build_roles` (public hidden test seams) and
`build_roles_batched` (private, the real emission kernel) at
`modules/core-modules/tree-support-planner/src/lib.rs`. Production reaches the batched kernel
through `flush_pending_emits` over private `RoleBuildInput`. The serial `build_roles` has **no**
production caller; both are checked against the same independent model, and neither is the other's
oracle. `carve_emitted_regions_with_bbox_gate` calls `carve_emitted_regions` only for its
lone-gated/simplification repair, so nothing here claims every emitted role invokes the
unconditional helper.

### Contract

For each input region, subtract the collision set and retain the uniquely larger surviving part
where the cut splits it. Then form the declared body, roof, base and floor sets, and subtract each
interface set from the body. The emitted role vector is ordered `SupportBody`, `TopInterface`,
`BaseInterface`, `BottomInterface` with empty roles omitted, and the body is disjoint from each
interface. Interface roles need not be pairwise disjoint outside the intentionally disjoint fixture.
In the batched own-territory mode each role is additionally intersected with its own territory.
Distinct-point skeleton segments are not assumed to create role polygons — every fixture passes
empty segment lists, so the kernel's `structural_body_regions` contributes nothing and the claim
stays inside the area algebra. The larger production routing around this kernel is out of scope.

### Domain

Bounded, non-simplifying, axis-aligned rectangle scenes: a body rectangle, a collision bar that
splits it into one strictly larger and one strictly smaller part (both orientations, via a scene
transpose), a roof rectangle inside the body, a floor rectangle, a fully swallowed region, a
no-collision scene, a bbox-disjoint gated region, and a batched own-territory scene. Two
distinguishable inputs are always submitted together so the batch's per-item association is
verifiable. `role_simplify_tolerance` returns `None` for the whole generated domain —
`avg_node_per_layer` in `0..=200` at line widths `1/64`, `0.4` and `2.0` mm for both
`is_base_area` values — which is what makes the rectangular model exact; the controls assert the
first `Some` value (2000 units at `avg = 201`, width 0.4 mm, base area). Bounds are
`max_abs_units`, `max_regions`, `max_cells`, `max_batch_items`; `max_abs_units` is at least
300 000 units because the second fixture reaches x = 30 mm, `max_regions` is `3..=32`,
`max_cells` is at least `(2 * (max_regions + 3) - 1)^2`, and `max_batch_items` is `2..=16`.

### Strategy

Nonshrinking selector over the split, swallowed, no-collision, disjoint-gated and own-territory
modes with a total generator. Deterministic positive fixtures for every mode run inside the
callback. Counters for accepted calls, split/swallow/no-collision/territory cases, batch items,
body cells, interface cells, territory-removed cells, swallowed cells and split regions are
asserted positive after `check`.

### Oracle

The scene is partitioned at every input and output X and Y edge, and each open cell is labelled
from the source rectangles at doubled-coordinate centres, so the comparison is an exact parity test
insensitive to vertex order and ring start. The label algebra produces the expected body
(`material - collision - roof - base - floor`) and the expected interface sets exactly as the
contract states. Each batch item is compared against its own independently computed expectation,
which is what catches a swapped or dropped result.

### Tolerance

None. All fixture coordinates are multiples of a whole millimetre and the operations are Clipper
booleans on axis-aligned rectangles, so cells are compared exactly.

### Shrinking

`tree_planner_shrink_bounds` drives the real strategy, revalidates the rectangle ordering, the
collision disjointness preconditions, the `role_simplify_tolerance` invariant and the batch arity
at every state, requires a permitted reduction, and exercises a false `simplify` return and
repeated `complicate`.

### Execution

Both tiers run `in_process` with `timeout_ms = 0`, chacha, zero rejection budgets and total
generators. Registry command
`cargo xtask test --summary --properties --property M-TREE-ROLES --tier routine|extended`.
Measured on Windows: 64 cases in 0.23 s and 1024 cases in 5.95 s of libtest, with 127 and 2 535
batch items checked. Provenance and the full counter mix are in
[`docs/testing/proptest-support-modules-evidence.md`](proptest-support-modules-evidence.md).

### Failure record

Standard `PropertyRun` artifacts; no confirmed defect, so no committed regression. A batched result
that disagrees with its own independently computed item is a failure, never a reason to fall back
to the serial path.

### Acceptance

Maps to AC-4, AC-8 and AC-N1. In an independently modelled body `[0,10]^2` with collision
`[7,8] x [0,10]` and roof `[3,5]^2`, the unique larger carved body side contains `(1,1)`, excludes
`(7.5,1)` and `(8.5,1)`, and excludes roof point `(4,4)` from the body while the roof retains it.
With batched own territory `[0,6] x [0,10]` the body additionally excludes `(6.5,1)`.
`tree_planner_controls` accepts the valid positive observation and then rejects, with the same
checker and its exact failure class: a body speck at each forbidden point (classed as collision
material, wrong retained fragment and body/interface overlap), a wrong role order, an empty role,
an omitted body, all-empty output, a carve fragment dropped, the batched `(6.5,1)` territory
speck, the batch result swapped with the second fixture translated by 20 mm in X, a dropped batch
item, and serial output passed off as the batched result.

## M-SUPPORT-RASTER

Registry record: `id = "M-SUPPORT-RASTER"`, `package = "traditional-support-planner"`,
`target_kind = "lib"`, `target_name = "traditional_support_planner"`,
`test_name = "property_tests::proptest_support_raster_cells"`,
`production_symbol = "traditional_support_planner::agg_raster::rasterize_polygons"`,
`production_path = "modules/core-modules/traditional-support-planner/src/agg_raster.rs"`,
`features = ["traditional-support-planner/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-support-raster"`.

### Stable property ID

`M-SUPPORT-RASTER` — semantic (whole-buffer cell coverage of the AGG support rasterizer under a
restricted open-interior model).

### Owner

Package `traditional-support-planner`; lib target `traditional_support_planner`; module
`property_tests` in `modules/core-modules/traditional-support-planner/src/property_tests.rs`;
production symbol `rasterize_polygons` (public) at
`modules/core-modules/traditional-support-planner/src/agg_raster.rs`. Grids are built with
`GridParams::for_test`, so constructor arithmetic is never inferred from the rasterizer. The
kernel is consumed only on the opt-in `RasterizerMode::Agg` descent arm; this charter makes no
claim about the default `LegacySemantic` mode.

### Contract

A cell is marked when the polygon set covers any part of it, from either a centre-inside
nonzero-winding scan or any edge passing through the cell; there is no coverage threshold. The
model is restricted to axis-aligned, well-wound rectangles, notches and rectangular holes whose
vertices sit strictly away from pixel-grid lines and corners, which makes the open-interior set of
each material rectangle unambiguous. Within that restriction the **entire** row-major `0/1` buffer
and its length are compared. Coincident-boundary conventions, nonzero-winding parity at shared
edges and antialiasing amplitudes are explicitly out of scope, and the kernel is angle-0 only.

### Domain

Bounded orthogonal scenes on a checked integer grid with a translated (including negative) origin,
a pixel size that can partially cover a cell, at least one cell whose centre is outside while its
coverage is positive, and a rectangular hole whose interior cell stays unset. Empty and short rings
and a zero-sized grid are separate deterministic no-work modes. Bounds are `max_abs_units`,
`max_pixel_units`, `max_grid_width`, `max_grid_height`, `max_cells`; `max_cells` is checked before
any allocation and every dimension, stride and origin offset is computed in checked arithmetic.

### Strategy

A nonshrinking shape-mode selector (rectangle, notch, holed rectangle, multi-component, no-work)
over a total generator, plus deterministic witnesses built and checked inside every callback. Each
generated value is validated against the domain before the kernel runs. After `check` the
accepted-call count, the four witness counters (centre-outside, hole-interior, negative-origin
notch, no-work) and the generated set-cell total are asserted positive. The per-run mode mix, the
checked-cell tally, the centre-outside tally and the generated hole-interior tally are reported but
not required positive, because a run may legitimately generate no holed shape — the positive
hole-interior topology is supplied by the deterministic witness, not by the sampler.

### Oracle

The expected buffer is enumerated cell by cell with exact interval arithmetic: each cell's
half-open integer square is intersected with each analytically described material rectangle, the
analytically described holes are subtracted, and the cell is set iff the remaining area is
positive. The model never consults production edge rasterization, `contours_simplified` or the
host clipper.

### Tolerance

None. Coordinates and pixel sizes are integers in canonical units, so the comparison is exact
buffer equality. The only derived quantity is the fixture-side exclusion band that keeps vertices
off grid lines and corners.

### Shrinking

`traditional_planner_shrink_bounds` drives the real strategy through
`current`/`simplify`/`complicate`, revalidates every exposed case against the domain at every
state, requires a real reduction from a nonminimal start, and exercises a false `simplify` return
and repeated `complicate`. The mode selector is asserted to refuse both directions.

### Execution

Both tiers run `in_process` with `timeout_ms = 0`, chacha, zero rejection budgets and total
generators. Registry command
`cargo xtask test --summary --properties --property M-SUPPORT-RASTER --tier routine|extended`.
Measured Windows timings are in
[`docs/testing/proptest-support-modules-evidence.md`](proptest-support-modules-evidence.md).

### Failure record

Standard `PropertyRun` artifacts; no confirmed defect, so no committed regression.

### Acceptance

Maps to AC-1, AC-8 and AC-N1. `traditional_planner_controls` first accepts the valid positive
observation, then rejects with the same checker and its exact failure class: a centre-only bit flip
on the witness whose cell `(1,1)` has positive coverage but a centre at `(15,15)` grid units
outside the material, and a flipped hole-interior bit. A size mismatch, a shifted origin, an
unset-but-covered cell or a set hole-interior cell fails.

## M-SUPPORT-HOLES

Registry record: `id = "M-SUPPORT-HOLES"`, `package = "traditional-support-planner"`,
`target_kind = "lib"`, `target_name = "traditional_support_planner"`,
`test_name = "property_tests::proptest_support_hole_fill"`,
`production_symbol = "traditional_support_planner::agg_raster::fill_grid_holes"`,
`production_path = "modules/core-modules/traditional-support-planner/src/agg_raster.rs"`,
`features = ["traditional-support-planner/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-support-holes"`.

### Stable property ID

`M-SUPPORT-HOLES` — semantic (one-pass simultaneous single-cell hole fill on the original grid).

### Owner

Package `traditional-support-planner`; lib target `traditional_support_planner`; module
`property_tests`; production symbol `fill_grid_holes` (public) at
`modules/core-modules/traditional-support-planner/src/agg_raster.rs`, reached in production through
`contours_simplified`'s `fill_holes` branch when a caller requests extraction with hole filling.
The property checks the kernel directly and inherits no caller guarantee.

### Contract

Output is a copy of the input with exactly one non-iterative pass over the interior cells. An
interior cell becomes set when it is set, or when both horizontal neighbours are set in the
**original** grid, or when both vertical neighbours are set in the original grid. The outer ring is
never written. Reading the original rather than the partially written copy is load-bearing: a
two-cell gap stays unbridged and a left-to-right in-place fill would wrongly bridge it. Inputs with
either dimension below three, or a buffer shorter than `width * height`, are returned unchanged.
This is not a general enclosed-hole fill and idempotence is not claimed.

### Domain

Bounded small grids with an independently validated boundary ring and interior populations that
include a hole bridged horizontally only, a hole bridged vertically only, a single-cell hole, a
two-cell horizontal gap and a two-cell vertical gap. Separately: a dimension below three, a short
buffer, an all-zero grid and an all-set grid. Bounds are `max_grid_width`, `max_grid_height`,
`max_cells`, each checked before allocation.

### Strategy

Nonshrinking shape-mode selector over a total generator, with the deterministic 5x5 cascading
witness executed inside every callback. Counters for accepted calls, grid/tiny/short modes,
checked cells, filled cells and horizontal/vertical fills are asserted positive after `check`.

### Oracle

The expected buffer is computed from the **original** grid with a pure two-neighbour relation
written into a separate output buffer, so the model cannot cascade by construction. It is written
from the rule above, never by re-invoking the kernel or by iterating a flood fill.

### Tolerance

None; the buffers are exact.

### Shrinking

`traditional_planner_shrink_bounds` drives the real strategy, rejects any exposed case with a set
outer-ring cell or an out-of-bounds dimension, requires a real reduction, and demonstrates the
stable-false `complicate` state.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, zero rejection budgets. Registry command
`cargo xtask test --summary --properties --property M-SUPPORT-HOLES --tier routine|extended`.
Measured timings are in the evidence file.

### Failure record

Standard `PropertyRun` artifacts; no confirmed defect, so no committed regression.

### Acceptance

Maps to AC-1, AC-8 and AC-N1. The property compares the whole buffer for every accepted case.
`traditional_planner_controls` asserts the exact 5x5 requirement — set cells
`(column,row) = (1,1), (3,2), (1,3)` yield `(1,2) = 1` and `(2,2) = 0` — then rejects an output
that changes `(2,2)` after the valid fill, and rejects a fill of one cell inside a two-cell
horizontal gap.

## M-SUPPORT-GRID

Registry record: `id = "M-SUPPORT-GRID"`, `package = "traditional-support-planner"`,
`target_kind = "lib"`, `target_name = "traditional_support_planner"`,
`test_name = "property_tests::proptest_support_grid_extraction"`,
`production_symbol = "traditional_support_planner::agg_raster::SupportGrid::new"`,
`production_path = "modules/core-modules/traditional-support-planner/src/agg_raster.rs"`,
`features = ["traditional-support-planner/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-support-grid"`.

### Stable property ID

`M-SUPPORT-GRID` — semantic (macro-block isolation, 3x3 erosion, sample selection and trim
subtraction over explicitly constructed rasters).

### Owner

Package `traditional-support-planner`; lib target `traditional_support_planner`; module
`property_tests`; production symbols `SupportGrid::{new, for_test, grid, extract_islands,
filter_islands_by_samples, extract_support}` and `seed_fill_block`, `dilate_trimming_region`,
`contours_simplified` at
`modules/core-modules/traditional-support-planner/src/agg_raster.rs`.

`SupportPlanner::plan_candidate` really calls `SupportGrid::new`, `extract_islands` and
`filter_islands_by_samples` on the `RasterizerMode::Agg` arm and does **not** call
`extract_support`, so `extract_support` is exercised as a public composition seam only and is never
presented as a production caller.

### Contract

Two separate contracts are checked and they are not interchangeable:

1. `seed_fill_block` does exactly two block-local sweeps. A step copies a set bit to the adjacent
   cell only when the eroded trimming mask is clear at **both** endpoints, so propagation never
   crosses a macro block and never passes a masked cell. Original masked seeds remain present but
   cannot propagate. There is no fixed point: an arbitrary masked maze is **not** modelled.
2. `extract_islands` runs `contours_simplified` at a legal in-cell offset and differences the
   trimming polygons away; `filter_islands_by_samples` keeps only islands containing at least one
   sample. AGG block snapping may enlarge the extracted carry beyond the input polygon and may
   avoid a *local* blocker that yields `NoRoute` in legacy mode (`DEV-166`), so no
   extracted-subset-of-input or cross-layer monotonicity claim is made.

### Domain

Bounded multi-block scenes with a pre-validated empty outer ring, an analytically specified 3x3
erosion mask, and per block either one axis-aligned rectangle of unmasked traversable cells with a
nonempty seed inside it, or no usable seed. That is the only subdomain on which the source's two
sweeps provably equal a flood-to-fixpoint, so the queue flood is used **only** there and is proved
from the rectangular shape rather than asserted. A block outside that subdomain is never compared
to the flood model. It is instead held to sound necessary conditions only — a block with no
unmasked seed gains no cell, and a gained cell must lie inside the block's fixpoint reach — which
cannot pass a wrong result but does not pin the exact set. Those blocks are counted separately and
reported, so each run states how much of its population was exactly modelled rather than
substituting a weaker oracle silently. Separately: a mask/grid length mismatch
(the source asserts, so it is tested as the specified panic), zero size or stride, a blocked
source, a blocked destination, an empty mask, neighbouring-block isolation, a tiny or short
extraction buffer with a valid offset, and an invalid offset with its own assertion guard.
Extraction uses analytic single-cell and rectangular islands separated by at least one pixel.
Bounds are `max_abs_units`, `max_pixel_units`, `max_grid_width`, `max_grid_height`, `max_blocks`,
`max_cells`.

### Strategy

Nonshrinking selector over the scene, seed, island and trim modes with a total generator. Every
scene passes the per-block rectangularity predicate before the flood model is used, and the
deterministic witnesses for block isolation, masked source and destination, the macro-block halo,
sample selection, trim, hole exclusion, no samples, empty extraction and the legal offset boundary
run inside every callback. Counters for proven/seeded/masked blocks, filled cells, multi-block
scenes, islands, selected and trimmed islands are asserted positive after `check`.

### Oracle

The erosion mask is modelled as an all-nine-neighbour conjunction over the trimming raster. The
seed-fill result on the proven subdomain is modelled with a four-neighbour queue flood from each
block's seeds, and the equivalence with the source's two sweeps is justified from the rectangle
shape and verified block by block. Extraction is compared as a geometric set at test-owned interior
points and at explicitly derived rectangle limits, never by contour start vertex, vertex order or
ring count. Trim is modelled as set subtraction, sample filtering as containment of the sample in
the island's own geometry, and the halo witness is compared against the block-local extent rather
than against the original polygon subset.

### Tolerance

Exact integer cell comparisons for the mask and the fill. The extraction comparison is exact in
canonical units: the expected rectangle limits are derived from `offset_in_grid * pixel_size` and
the corpus offset is an integer, so the corner displacement is an integer addition.

### Shrinking

`traditional_planner_shrink_bounds` drives the real strategy, revalidates the empty outer ring, the
rectangularity predicate and the block layout at every simplified state, requires a permitted
reduction, and exercises the false-`simplify` and repeated-`complicate` states.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, zero rejection budgets. Registry command
`cargo xtask test --summary --properties --property M-SUPPORT-GRID --tier routine|extended`.
Measured timings are in the evidence file.

### Failure record

Standard `PropertyRun` artifacts; no confirmed defect, so no committed regression.

### Acceptance

Maps to AC-1, AC-8 and AC-N1. `traditional_planner_controls` rejects, using the same checker: a
second-block cell set by the first block's fill (`grid_size = (6,4)`, `blocks = (2,1)`,
`oversampling = 2`, seed `(2,1)`, clear mask — `(3,1)` must stay `0`); propagation out of a masked
seed `(1,1)` or into a masked destination `(2,1)`; selecting the second of two separated
`SupportGrid::for_test` islands at pixel size 10 with sample `(15,15)` grid units; and retained
trim material at `(17,15)` when the trim covers `[15,20] x [10,20]` grid units and the sample is the
untrimmed point `(12,15)`.

## M-SUPPORT-TERRITORY

Registry record: `id = "M-SUPPORT-TERRITORY"`, `package = "traditional-support-planner"`,
`target_kind = "lib"`, `target_name = "traditional_support_planner"`,
`test_name = "property_tests::proptest_support_planner_territory"`,
`production_symbol = "traditional_support_planner::SupportPlanner::clip_to_territory"`,
`production_path = "modules/core-modules/traditional-support-planner/src/lib.rs"`,
`features = ["traditional-support-planner/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-support-territory"`.

### Stable property ID

`M-SUPPORT-TERRITORY` — semantic (exact-source own intersection and base-minus-expanded-foreign
clearance at the planner's local seam).

### Owner

Package `traditional-support-planner`; lib target `traditional_support_planner`; module
`property_tests`; production symbol `SupportPlanner::clip_to_territory` (private) at
`modules/core-modules/traditional-support-planner/src/lib.rs`, called in `plan_candidate` after
extraction and again on the propagation arm. Fixtures are fresh test-local `SupportAnalysisView`
values built over `SupportAnalysisIR`.

### Contract

With a minted modifier sub-region's own territory present, the result is `polys` intersected with
that territory, looked up by the exact `(global_layer_index, object_id,
modifier_sub_region_id, ordered variant_chain)` identity. With no own entry, the result is `polys`
minus the positive-line-width expanded family-foreign rectangle from
`territory_partition(object_id, layer, "traditional")`; when the partition or its foreign set is
empty the input is returned unchanged, and an empty input is returned unchanged. A base region
therefore loses the band the foreign family grows by the line width, while a minted sub-region
keeps its own territory. This is `DEV-165`'s PnP-only rule at the planner's local seam. It is
**not** the host clipper proof owned by packet `proptest-openrscad_11_pbt-prepass`, and it makes no
canonical-routing claim.

### Domain

Controlled axis-aligned rectangles in canonical units: a subject rectangle, rectangular own
territory, rectangular foreign territory, a positive line width, at least two distinguishable
ordered variant chains and their swap, a sibling region with the other chain, a second object id
and a second layer, plus no-partition, own-family-only, empty-subject and fully-removed modes. The
fully-removed subject is placed inside the foreign rectangle grown by the line width, so the whole
subject is removed rather than just its middle band. Bounds are `max_abs_units`,
`max_clearance_units`, `max_cells`; the classification grid is derived from the checked subject
extent and `max_cells` is checked before allocation.

### Strategy

Nonshrinking selector over the own, base, no-partition, own-family-only, empty and fully-removed
modes with a total generator. The deterministic positive fixture for each mode is built and checked
inside the callback. Counters for kept cells, removed cells, clearance cells and the seven witness
classes are asserted positive after `check`.

### Oracle

Cells are classified by rectangle arithmetic at test-owned points: a cell is retained when it lies
in the subject and, for the own mode, inside the own territory rectangle; for the base mode it is
retained when it lies in the subject and outside the foreign rectangle expanded by the supplied
line width. Nothing in the oracle calls the host clipper, so a clipping or clearance defect cannot
be reproduced by the expectation. The chain vector is compared exactly and never sorted.

### Tolerance

Exact at cell points placed away from the expanded rectangle's boundary by at least one canonical
unit. The `(4.5, 5)` mm witness is exactly on the expanded boundary for a 1 mm clearance from
`x = 5` and is asserted **excluded**, because the difference removes the closed expanded set. No
allowance is expanded to make a failure pass.

### Shrinking

`traditional_planner_shrink_bounds` drives the real strategy through
`current`/`simplify`/`complicate`, revalidates the exact chain vector and the rectangle ordering at
every state, requires a permitted reduction, and demonstrates the stable `complicate` state.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, zero rejection budgets. Registry command
`cargo xtask test --summary --properties --property M-SUPPORT-TERRITORY --tier routine|extended`.
Measured timings are in the evidence file.

### Failure record

Standard `PropertyRun` artifacts; no confirmed defect, so no committed regression.

### Acceptance

Maps to AC-2, AC-8 and AC-N1. For subject `[0,10]^2`, own territory `[0,4] x [0,10]` must contain
`(2,5)` and exclude `(6,5)`; for foreign `[5,7] x [0,10]` with width `1` the base result must
exclude `(4.5,5)` and `(6,5)` and retain `(2,5)` and `(9,5)`. A fully covered subject must come
back empty, a second object or layer must not clip, and `traditional_planner_controls` rejects
reinserting `(4.5,5)` (omitted clearance), substituting the other ordered-chain territory for the
exact own-source lookup, and a false all-empty own result.

## M-TREE-SCAN

Registry record: `id = "M-TREE-SCAN"`, `package = "tree-support"`, `target_kind = "lib"`,
`target_name = "tree_support"`, `test_name = "property_tests::proptest_tree_support_scan"`,
`production_symbol = "tree_support::TreeSupport::scan_fill_region"`,
`production_path = "modules/core-modules/tree-support/src/lib.rs"`,
`features = ["tree-support/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-tree-scan"`.

### Stable property ID

`M-TREE-SCAN` — semantic (complete horizontal and vertical scan-interval coverage of the tree fill
kernel, with a separated mid-scan fallback).

### Owner

Package `tree-support`; lib target `tree_support`; module `property_tests` in
`modules/core-modules/tree-support/src/property_tests.rs`; production symbol
`TreeSupport::scan_fill_region` (private) at `modules/core-modules/tree-support/src/lib.rs`. Its
production caller is `TreeSupport::render_polygon_with_wall_count`, reached from
`TreeSupport::run_support`; the `vertical` flag is the real production parameter
`entry.anchor_layer_index % 2 != 0`.

### Contract

Crossings are gathered from the contour and every hole ring at
`scan_min + spacing / 2 + n * spacing` while below `scan_max`, where the scan axis is Y for the
horizontal mode and X for the vertical mode. Crossings are sorted and paired; a pair is emitted
only when it is non-degenerate and lies inside the region's extent **on the segment's own axis**.
Each surviving pair becomes one `ExtrusionPath3D` with `role = SupportMaterial`, the supplied Z and
`speed_factor`, `flow_factor = 1.0` and the module's `line_width`. When no ordinary line is emitted,
a separate mid-scan fallback is attempted at the axis midpoint; it is a distinct mode, not a
universal tiny-tip guarantee.

### Domain

Axis-aligned rectangles, notches and holed rectangles in horizontal and vertical modes, with
positive dyadic dimensions and spacing so the row grid is exact, plus a separated fallback mode and
a separate nonpositive-spacing and degenerate-ring empty mode. Bounds are `max_abs_units`,
`max_vertices`, `max_holes`, `max_components`, `max_shrink_candidates`, `max_spacing_units`,
`max_rows`; `max_abs_units` is at least 200 000 units because the required vertical witness reaches
y = 20 mm.

### Strategy

Nonshrinking selector over the horizontal-rectangle, vertical-rectangle, notch, holed, fallback and
degenerate modes with a total generator. Deterministic positive fixtures for every mode run inside
the callback. Counters for accepted calls, per-mode classes, vertical cases, strokes, segments and
split rows, plus the six witness classes, are asserted positive after `check`.

### Oracle

Expected intervals are derived independently at the row positions above from the fixture's own
rectangle edges, and expected segments are compared whole, with holes excluded by the same
rectangle arithmetic. Every consecutive point pair of every path is additionally checked with the
boundary-inclusive `oracles::segment_in_region` against the self-built `ExPolygon`. Metadata is
compared exactly.

### Tolerance

Exact: every scan fixture coordinate is a multiple of 1/8 mm and every spacing a multiple of
1/4 mm, so `units_to_mm`, the `f64` row accumulation, the mid-scan fallback and `mm_to_units` are
all exact dyadic arithmetic. The kernel's crossing test `(a > scan) != (b > scan)` makes a ring's
low edge inclusive and its high edge exclusive, which the model reproduces, so rows that land
exactly on a hole edge are compared exactly too.

### Shrinking

`tree_renderer_shrink_bounds` drives the real strategy, revalidates the ring structure and the mode
selection at every state, requires a permitted reduction, and exercises the false-`simplify` and
repeated-`complicate` states.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, zero rejection budgets. Registry command
`cargo xtask test --summary --properties --property M-TREE-SCAN --tier routine|extended`.
Measured timings are in the evidence file.

### Failure record

**Confirmed defect and focused fix (commit `1515f8ce`).** The pair guard compared the segment-axis
pair against the region's perpendicular extent. In `vertical` mode the pair holds Y coordinates
while the guard tested `pair[1] <= max_x`, so the required `[0,10] x [0,20]` mm witness at 2.5 mm
spacing emitted **zero** strokes — all four pairs are `(0.0, 20.0)` and fail `20.0 <= 10.0` — and
the mid-scan fallback was rejected by the same comparison. Because `vertical` is
`entry.anchor_layer_index % 2 != 0`, every odd-anchor-layer tree region taller than it is wide lost
its fill. The kernel now selects the pair extent from the segment's own axis. The no-RNG regression
`vertical_scan_fill_spans_the_full_y_extent_regression` lives in the crate's `#[cfg(test)] mod
tests`, fails on the old kernel with `got []` against an expected 4, and needs neither RNGs nor
`PNP_PROPERTY_CONTEXT`.

### Acceptance

Maps to AC-5, AC-8, AC-N1 and AC-N2. The vertical witness produces exactly four strokes at
`x = 1.25, 3.75, 6.25, 8.75` mm each spanning `y [0,20]`. `tree_renderer_controls` accepts the
valid positive observation and then rejects, with the same checker: a single unsplit
`(1,5) -> (9,5)` mm segment across outer `[0,10]^2` minus `[2,3] x [4,6]`, the U-notch chord
`(2,5) -> (8,5)`, a dropped interior `y = 1` row, and a false all-empty output.

## M-TREE-RENDER

Registry record: `id = "M-TREE-RENDER"`, `package = "tree-support"`, `target_kind = "lib"`,
`target_name = "tree_support"`,
`test_name = "property_tests::proptest_tree_support_walls_and_fill"`,
`production_symbol = "tree_support::TreeSupport::render_polygon_with_wall_count"`,
`production_path = "modules/core-modules/tree-support/src/lib.rs"`,
`features = ["tree-support/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-tree-render"`.

### Stable property ID

`M-TREE-RENDER` — semantic (distinct inset wall bands plus clearance-respecting fill for the tree
renderer's explicit-wall-count path).

### Owner

Package `tree-support`; lib target `tree_support`; module `property_tests`; production symbol
`TreeSupport::render_polygon_with_wall_count` (private) at
`modules/core-modules/tree-support/src/lib.rs`, reached from `TreeSupport::run_support`.
`TreeSupport::render_polygon` is a test-used wrapper only and is not treated as the production
explicit-wall-count path.

### Contract

Wall `i` is the region offset by `-line_width * (i + 0.5)`, emitted as a closed ring per contour and
per hole. Fill is the region offset by `-line_width * wall_count`, scanned at
`max(fill_spacing_mm, line_width)` through `scan_fill_region`. For a 10 mm square with line width
`0.5` and two walls this gives the distinct closed wall-centre bands `[0.25, 9.75]` and
`[0.75, 9.25]`, and fill centre lines inside `[1, 9]^2`. For a hole the outward hole expansion is
accounted for and every wall and fill segment is checked against the relevant analytically inset
region. Offset-collapse and small-tip cases are explicit possibly-empty outcomes; nonempty output is
required only in the independently feasible roomy scenes. `from_config` clamps the configured wall
count, so the private zero-wall mode is checked as a kernel mode and is never claimed to pass
through the entrypoint.

### Domain

Sufficiently roomy axis-aligned rectangles and a holed rectangle with line width `0.5` mm, wall
counts zero, one and two, both scan orientations, and a positive fill spacing, plus explicit
collapse and sub-pitch fallback modes. Bounds are `max_abs_units`, `max_vertices`, `max_holes`,
`max_components`, `max_shrink_candidates`, `max_width_units`, `max_walls`, `max_rows`.

### Strategy

Nonshrinking selector over the one-wall, two-wall, holed, zero-wall, holed-zero-wall, collapse and
fallback modes with a total generator. Deterministic positive fixtures for every mode run inside the
callback. After `check` the accepted-call count and all nine witness counters (two-wall bands,
two-wall fill, vertical fill, hole bands, hole split rows, zero-wall exact fill, partial collapse,
full collapse, sub-pitch fallback) are asserted positive. The per-run mode mix, wall-loop,
fill-stroke and split-row tallies are reported but not required positive, because a run may
legitimately generate no fully-collapsing shape — that topology comes from the deterministic
witness.

### Oracle

Expected extents are derived analytically from the inset arithmetic above — never from
`offset_polygons` output, which would make the kernel its own oracle. Distinctness of the two wall
bands is asserted from their derived extents. Fill centre lines are checked against the
independently inset fill rectangle, and every consecutive point pair of every path is checked with
the boundary-inclusive `oracles::segment_in_region` against the self-built inset region.

### Tolerance

`CLIPPER_ALLOW = 2` canonical units (200 nm) on every Clipper-derived coordinate, derived as
follows: the offset delta is a dyadic multiple of 1/4 mm so `delta_mm as f64 * 10_000` is exact;
Clipper2 rounds each offset vertex to the nearest integer (<= 0.5 unit); the scan kernel then
re-derives rows through one `f32` round trip and one `mm_to_units` rounding (<= 0.5 unit plus an
`f32` relative error below 0.06 unit at 10^6 units). The total is <= 1.06 unit, so 2 is a
factor-of-two margin rather than an observed epsilon. The generator keeps every fill row at least
625 units (1/16 mm) from every analytic fill edge, far outside the allowance, and the checker
re-verifies that separation and reports a violation as a fixture failure. The zero-wall mode never
calls Clipper and is compared with allowance 0. `M-TREE-SCAN` needs no allowance at all: its
fixtures are on a 1/8 mm grid with 1/4 mm spacings, so the whole comparison is exact dyadic
arithmetic.

### Shrinking

`tree_renderer_shrink_bounds` drives the real strategy, revalidates the wall count against
`max_walls`, the roominess precondition and the hole nesting at every state, requires a permitted
reduction, and exercises the false-`simplify` and repeated-`complicate` states.

### Execution

Both tiers, `in_process`, `timeout_ms = 0`, chacha, zero rejection budgets. Registry command
`cargo xtask test --summary --properties --property M-TREE-RENDER --tier routine|extended`.
Measured timings are in the evidence file.

### Failure record

Standard `PropertyRun` artifacts; no confirmed defect, so no committed regression.

### Acceptance

Maps to AC-6, AC-8 and AC-N1. `tree_renderer_controls` accepts the valid positive observation and
then rejects, with the same checker and its exact failure class: the inner `[0.75,9.25]` loop
replaced by a duplicate of the outer `[0.25,9.75]` loop (a coincident wall), the hole wall removed
from the holed case, and fill inserted at `(0.5,5)` outside the required `[1,9]^2` fill region. The
sub-pitch scan fallback and the offset-collapse/no-work modes are separately observed and are not
treated as always-success or always-empty.


<!-- PBT-LAYERS-SEAMS charters (packet proptest-openrscad_15) -->

## M-LAYERS-SEQ

Registry record: `id = "M-LAYERS-SEQ"`, `package = "layer-planner-default"`, `target_kind = "lib"`,
`target_name = "layer_planner_default"`, `test_name = "property_tests::proptest_layer_sequence"`, `production_symbol = "layer_planner_default::generate_object_layers"`,
`production_path = "modules/core-modules/layer-planner-default/src/lib.rs"`, `features = ["layer-planner-default/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-layers-seq"`.

### Stable property ID

`M-LAYERS-SEQ` -- semantic, independent of packet numbers.

### Owner

Package `layer-planner-default`; lib target `layer_planner_default`; module `property_tests` in
`modules/core-modules/layer-planner-default/src/property_tests.rs`; production symbol `layer_planner_default::generate_object_layers` in
`modules/core-modules/layer-planner-default/src/lib.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_layer_steps, max_raft_layers, max_z_quanta`.

### Contract

Independent event/interval model of the object-local native Z-plane sequence. The uniform fallback terminates on the canonical band midpoint (`generate_object_layers_f64`: emit while `z_f64 - 0.5*step < height`, break at `>= height`), so a final print top may overshoot the object by up to half a layer; explicit nonempty `layer_zs` is forwarded verbatim (host `layer_top_zs` already applied the midpoint rule). This is canonical `generate_object_layers` (`Slicing.cpp`), resolved upstream by perf-vs-orca t139 / closed `DEV-201`; `docs/02_ir_schemas.md` §"Print top vs sample plane" is normative. The fast(f64)/general(f32-round-trip) boundary and the general-merge catch-up guard `base_z_f64 > plan.height + 1e-6` are characterizations, not re-pinned end rules.

### Domain

Unique object IDs; finite positive bounded heights, first heights and steps on a 1/1024 mm dyadic grid with representable sums and gaps > 1e-6; bounded raft counts; strictly increasing finite explicit tops. Bounds `max_layer_steps=16, max_raft_layers=4, max_z_quanta=8192` validated before allocation. Empty/no-native and non-finite inputs are excluded from the raw loop; empty cases asserted separately.

### Strategy

Nonshrinking `prop::sample::select(modes).no_shrink()` for explicit-vs-uniform and fast-vs-general semantic modes; bounded dyadic dimensions/raft counts shrink. Deterministic companions (explicit A `[0.25,0.5,0.75,1.0]` h=1.0, B `[0.375,0.75]` h=0.75, raft/raft-free pair, fractional-height 0.55/0.2 -> `[0.2,0.4,0.6]`, endpoint/no-native/boundary-neighbor) plus positive `Cell` counters asserted `>0` after `PropertyRun::check`.

### Oracle

Independently derived native tops, raft prefix + common raft offset, per-object first credits and catch-up envelopes; NEVER calls the SUT to produce expected values. Midpoint termination recomputed from the model, not copied from the kernel.

### Tolerance

Exact dyadic arithmetic for the model; a derived f32 conversion envelope for print tops; `1e-6` coincidence tolerance for dedup/native-index only (enumerated in code). Explicit rounding-boundary probe kept separate from the ordinary dyadic generator.

### Shrinking

`packet15_witnesses_controls_and_shrinking` drives the real strategy's ValueTree through `simplify`/`complicate`, requires a permitted reduction, re-checks mode/domain/witness at every state, and asserts false `simplify` / repeated `complicate` settle. Shrink completion is `null`/unknown (pure Proptest 1.11).

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["layer-planner-default/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-LAYERS-SEQ --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-LAYERS-SEQ/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-LAYERS-SEQ --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-1, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. Both `in_process` tiers (routine 64 / extended 256 cases, chacha, timeout 0, zero reject budgets); `C` and the two `B` layer baselines pass; `M-LAYERS-SEQ` and `M-LAYERS-MERGE` discover with exact filters and emit matching controller begin/end events. Mutation controls: drop a native top, omit raft offset, replace catch-up bottom/height, swap association, insert empty-region plane, replace first explicit credit after catch-up -- each rejected by the same checker.

## M-LAYERS-MERGE

Registry record: `id = "M-LAYERS-MERGE"`, `package = "layer-planner-default"`, `target_kind = "lib"`,
`target_name = "layer_planner_default"`, `test_name = "property_tests::proptest_layer_merge"`, `production_symbol = "layer_planner_default::merge_layer_sequences"`,
`production_path = "modules/core-modules/layer-planner-default/src/lib.rs"`, `features = ["layer-planner-default/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-layers-merge"`.

### Stable property ID

`M-LAYERS-MERGE` -- semantic, independent of packet numbers.

### Owner

Package `layer-planner-default`; lib target `layer_planner_default`; module `property_tests` in
`modules/core-modules/layer-planner-default/src/property_tests.rs`; production symbol `layer_planner_default::merge_layer_sequences` in
`modules/core-modules/layer-planner-default/src/lib.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_objects, max_layer_steps, max_raft_layers, max_z_quanta`.

### Contract

Independent union-of-events model over multiple objects. `merge_same_height` (fast path: all-same-height, no explicit schedule) terminates on the f64 band midpoint against `max_height` and filters each object by its own midpoint; `merge_different_heights` (general path, forced by any explicit schedule) unions native tops, keeps native planes unfiltered (they inherit the schedule's midpoint rule) and still gates catch-up planes top-based by `base_z_f64 > plan.height + 1e-6`. Raft events precede model events; the greatest final raft top is added to ALL objects. Native and catch-up regions are distinct (`RegionLayerProposal.is_catchup`).

### Domain

Up to 4 objects; bounded dyadic-mm heights/steps/raft counts; explicit and uniform objects mixed; strictly increasing authored tops. Bounds `max_objects=4, max_layer_steps=16, max_raft_layers=4, max_z_quanta=8192` validated before any sequence construction. An emitted plane must have a nonempty region population.

### Strategy

Nonshrinking mode selector for fast/general + raft/no-raft + explicit/uniform participation; generated object sets and dyadic heights shrink. Positive `Cell` counters for native events, catch-ups, raft entries and participating objects, asserted `>0`; empty cases asserted separately.

### Oracle

Independently constructs the expected global event table (union of native tops, per-object catch-up from last participating Z, raft prefix) and each `RegionLayerProposal` (`object_id`, `region_id=="0"`, `effective_layer_height`, `is_catchup`, `catchup_z_bottom`) without calling the merge kernels. `MergedLayer` carries only `z` and `regions` -- no invented `is_raft`.

### Tolerance

f64 fast-path vs f32-general (`z as f64 - raft_top`) distinction characterized with a deliberately constructed rounding-boundary probe; exact dyadic model values; `1e-6` coincidence/sentinel uses enumerated; derived f32 envelope.

### Shrinking

Combined `packet15_witnesses_controls_and_shrinking` with real ValueTree simplify/complicate, mode + witness retention, a bounded step guard, and false-simplify / repeated-complicate assertions; shrink completion is `null`/unknown (pure Proptest 1.11).

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["layer-planner-default/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-LAYERS-MERGE --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-LAYERS-MERGE/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-LAYERS-MERGE --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-1, AC-N1, AC-5, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. Both tiers with exact discovery; the A/B and raft witnesses, one-object, equal-height fast-path, mixed explicit/uniform, exact-end witnesses execute. Mutation controls (drop native top, omit raft offset, replace catch-up, swap association, empty-region plane, reorder raft/model, replace first credit) each rejected by the same checker. `B` layer baselines pass.

## M-SEAM-PLANNER-PICK

Registry record: `id = "M-SEAM-PLANNER-PICK"`, `package = "seam-planner-default"`, `target_kind = "lib"`,
`target_name = "seam_planner_default"`, `test_name = "property_tests::proptest_planner_pick"`, `production_symbol = "seam_planner_default::comparator::pick_seam_point"`,
`production_path = "modules/core-modules/seam-planner-default/src/comparator.rs"`, `features = ["seam-planner-default/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 2`,
`charter = "docs/testing/properties.md#m-seam-planner-pick"`.

### Stable property ID

`M-SEAM-PLANNER-PICK` -- semantic, independent of packet numbers.

### Owner

Package `seam-planner-default`; lib target `seam_planner_default`; module `property_tests` in
`modules/core-modules/seam-planner-default/src/property_tests.rs`; production symbol `seam_planner_default::comparator::pick_seam_point` in
`modules/core-modules/seam-planner-default/src/comparator.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_candidates, max_coord_quanta, max_score_quanta`.

### Contract

Dormant-kernel coverage of `comparator::pick_seam_point` (and `pick_random_seam_point`): the comparator scans a nonempty, in-bounds `perimeter_range` and returns an index in that range; strict ties retain the first index; valid paint priority is `Enforced > Neutral > Blocked` (aligned central-enforcer preference checked separately). An empty range returns its unchecked start -- reported as dormant/defensive behaviour, never dereferenced or credited as a valid candidate. Both kernels have zero production callers on this tree.

### Domain

Finite bounded candidate scores on a 1/1024 grid; bounded coordinate quanta; nonempty in-bounds ranges with a nonzero-start singleton and mixed-paint ranges. Bounds `max_candidates=16, max_coord_quanta=8192, max_score_quanta=4096`. Empty range asserted separately; no invalid `central_enforcer`/paint combination assumed.

### Strategy

Nonshrinking selector for paint-class/mode; bounded scores and coordinates shrink. Positive counters for nonempty-range membership, tie-first and priority wins, asserted `>0` after `check`. Generator v2: the `RearY` class reserves one quantum of Y headroom below `max_coord_quanta` for in-range candidates, so the out-of-range "strongest" decoration is strictly better than every in-range candidate and the class always keeps a Y spread (fixes a v1 collision found by the extended tier; see the evidence doc).

### Oracle

Independently computes the best index over the sliced range by its own priority/score comparison; NEVER calls `pick_seam_point` to derive the expected answer. `pick_random_seam_point` checked for reproducible membership and an interpolated point on the selected edge, not distribution or seam quality.

### Tolerance

Exact integer index/priority comparison; bounded f32 score arithmetic; no tolerance for membership. Interpolated point uses a derived bounded f32 error.

### Shrinking

Combined controls/shrink test with real ValueTree simplify/complicate; mode/domain/witness revalidated at every state; nonshrinking mode trees asserted not to simplify.

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["seam-planner-default/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-SEAM-PLANNER-PICK --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-SEAM-PLANNER-PICK/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-SEAM-PLANNER-PICK --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-2, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. Dormant vs live distinction reported in evidence. Mutation controls: out-of-range or lower-priority candidate returned, blocked vertex retained, only enforcer demoted -- each rejected by the same checker. `B` spline baseline passes.

## M-SEAM-PLANNER-PAINT

Registry record: `id = "M-SEAM-PLANNER-PAINT"`, `package = "seam-planner-default"`, `target_kind = "lib"`,
`target_name = "seam_planner_default"`, `test_name = "property_tests::proptest_planner_paint"`, `production_symbol = "seam_planner_default::region_candidates"`,
`production_path = "modules/core-modules/seam-planner-default/src/lib.rs"`, `features = ["seam-planner-default/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-seam-planner-paint"`.

### Stable property ID

`M-SEAM-PLANNER-PAINT` -- semantic, independent of packet numbers.

### Owner

Package `seam-planner-default`; lib target `seam_planner_default`; module `property_tests` in
`modules/core-modules/seam-planner-default/src/property_tests.rs`; production symbol `seam_planner_default::region_candidates` in
`modules/core-modules/seam-planner-default/src/lib.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_candidates, max_coord_quanta`.

### Contract

LIVE planner selection. Exact `Custom("seam_blocker")`/`Custom("seam_enforcer")` `Flag(true)` annotations classify contour vertices: blockers removed, central enforcers score 2, other enforcers 1, neutral 0; `choose_region_candidate` picks only among MAX-score candidates and returns None on empty selection. `Random` uses `layer_index` as a deterministic modulo choice among eligible candidates (no external RNG seed). This is direct selection coverage, NOT end-to-end paint delivery. Hole vertices are neutral under the current contour annotation convention.

### Domain

Bounded integer XY `Point2` coordinates (native units) and bounded candidate counts. Bounds `max_candidates=16, max_coord_quanta=8192`. Enforcer, enforcer-run, blocked, no-paint, singleton and empty fixtures; empty selection asserted separately.

### Strategy

Nonshrinking selector for mode (Nearest/Rear/Random/Aligned/AlignedBack) and paint pattern; coordinates/counts shrink. Positive counters for nonempty eligible selections, max-score-only choice, blocked-exclusion and deterministic modulo cycling, asserted `>0`.

### Oracle

Independently builds the annotation -> score -> eligible-set -> mode-winner table; NEVER calls `region_candidates`/`choose_region_candidate` to derive the expected winner. Empty selection expected `None`.

### Tolerance

Exact score/class comparison and exact integer modulo; no numeric tolerance. Repeatability at fixed layer indices is a determinism pin (`// test-quality:`).

### Shrinking

Combined controls/shrink test; mode selector is nonshrinking; shrink states re-check paint classification and required witness; the mode tree itself is asserted not to simplify.

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["seam-planner-default/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-SEAM-PLANNER-PAINT --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-SEAM-PLANNER-PAINT/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-SEAM-PLANNER-PAINT --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-2, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. Mutation controls: return a nonmember / lower-priority candidate, retain a blocked vertex, demote the only enforcer -- each rejected by the same checker. Determinism at fixed layer indices and registry-seed replay recorded. `B` spline baseline passes.

## M-SEAM-PLANNER-SPLINE

Registry record: `id = "M-SEAM-PLANNER-SPLINE"`, `package = "seam-planner-default"`, `target_kind = "lib"`,
`target_name = "seam_planner_default"`, `test_name = "property_tests::proptest_planner_spline"`, `production_symbol = "seam_planner_default::align::fit_cubic_bspline"`,
`production_path = "modules/core-modules/seam-planner-default/src/align.rs"`, `features = ["seam-planner-default/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-seam-planner-spline"`.

### Stable property ID

`M-SEAM-PLANNER-SPLINE` -- semantic, independent of packet numbers.

### Owner

Package `seam-planner-default`; lib target `seam_planner_default`; module `property_tests` in
`modules/core-modules/seam-planner-default/src/property_tests.rs`; production symbol `seam_planner_default::align::fit_cubic_bspline` in
`modules/core-modules/seam-planner-default/src/align.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_observations, max_segments, max_coord_quanta, max_weight_quanta`.

### Contract

Dormant-kernel coverage of `align::fit_cubic_bspline` (zero production callers; `align_seam_points` is transitively dormant). Supported inputs only: nonempty equal-length arrays, finite sorted observation parameters, positive finite weights, `1 <= segments <= observations.len()`, bounded finite intermediates. The source-backed positive domain comes from `fit_curve` (`OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp`).

### Domain

Bounded finite observation parameters and 2D observations on a 1/1024 grid; positive finite weights; segment count bounded by observation count. Bounds `max_observations=16, max_segments=8, max_coord_quanta=8192, max_weight_quanta=4096`; `max_segments <= max_observations` enforced. Empty arrays / zero segments are INVALID fit-domain inputs and are never fed.

### Strategy

Nonshrinking selector for the fit family; bounded parameters/weights/segments shrink while preserving nonempty equal-length arrays and `segments <= observations.len()`. Positive counters for nondegenerate constant-data fits, weighted-mean degeneracies and single-observation fits, asserted `>0`.

### Oracle

Independently derives: a positive-span, well-conditioned CONSTANT-data fit at observed/interior parameters; the same-parameter weighted-mean degeneracy (equal parameters, observations `[[0,0],[10,0],[0,10]]`, weights `[1,2,1]` -> `[5,2.5]`); single observation. NEVER calls the fit to manufacture its expected answer.

### Tolerance

A dimensionally valid error envelope derived from the restricted basis/conditioning and f32 ops BEFORE choosing bounds; finite output required. No arbitrary interpolation, affine-exactness-for-all-samples, extrapolation or universal error claim.

### Shrinking

Combined controls/shrink test; shrink states re-check equal-length arrays, sorted parameters, positive weights and the required witness; nonshrinking family selector.

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["seam-planner-default/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-SEAM-PLANNER-SPLINE --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-SEAM-PLANNER-SPLINE/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-SEAM-PLANNER-SPLINE --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-2, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. Mutation controls: replace `[5,2.5]` weighted mean by the unweighted mean or a non-finite fit -- each rejected by the same checker. `B` `seam_canonical_spline_tdd` baseline passes. Dormant status named in evidence.

## M-SEAM-PLACER-SELECT

Registry record: `id = "M-SEAM-PLACER-SELECT"`, `package = "seam-placer"`, `target_kind = "lib"`,
`target_name = "seam_placer"`, `test_name = "property_tests::proptest_placer_select"`, `production_symbol = "seam_placer::select_seam_candidate"`,
`production_path = "modules/core-modules/seam-placer/src/lib.rs"`, `features = ["seam-placer/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-seam-placer-select"`.

### Stable property ID

`M-SEAM-PLACER-SELECT` -- semantic, independent of packet numbers.

### Owner

Package `seam-placer`; lib target `seam_placer`; module `property_tests` in
`modules/core-modules/seam-placer/src/property_tests.rs`; production symbol `seam_placer::select_seam_candidate` in
`modules/core-modules/seam-placer/src/lib.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_candidates, max_coord_quanta, max_score_quanta`.

### Contract

For finite nonempty candidates, independently check actual membership and local choice: `Nearest`/defensive aligned arms minimize `score + reason_bonus` (ties by lowest Y then X; bonuses UserForced -1.0, Concave -0.5, Sharp -0.2, Aligned 0.0); `Rear` maximizes Y then lower effective score then higher X; `Random` chooses `layer_index % candidates.len()`. Empty selection must produce no candidate and preserve the wall loop INCLUDING `Random`. AC-N2 requires REPRODUCTION AND CLASSIFICATION before a defect is confirmed: source inspection shows the `Random` arm divides by `candidates.len()` before `.get()`. If the empty case is reproduced and the documented no-candidate contract confirmed, a focused fix guards `len()==0 -> None` with a named concrete regression. The empty-candidate guarantee is documented mode-agnostically (`docs/05_module_sdk.md` T-082 + P109 graceful degradation + wall-preservation invariant), not by Random name.

### Domain

Finite bounded candidate positions/scores on a 1/1024 grid; all `SeamMode` variants; singleton, exact ties, and the empty list. Bounds `max_candidates=16, max_coord_quanta=8192, max_score_quanta=4096`.

### Strategy

Nonshrinking selector for `SeamMode`; positions/scores/layer_index shrink. Positive counters for nonempty member selection, each mode, exact-tie winner and deterministic Random cycling, asserted `>0`; empty case asserted separately (no panic, no candidate, walls pristine).

### Oracle

Independently computes the winning candidate by recomputing effective score and tie-break keys; NEVER calls `select_seam_candidate` to derive the expected member. Random expected index is `layer_index % len` computed independently.

### Tolerance

Exact integer index and `total_cmp` score comparison; exact reason bonuses; bounded f32 score arithmetic. Deterministic layer-index cycling is a determinism pin, not statistical randomness.

### Shrinking

Combined controls/shrink test; nonshrinking mode selector; shrink states re-check membership, mode and required witness; the mode tree itself is asserted not to simplify.

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["seam-placer/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-SEAM-PLACER-SELECT --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-SEAM-PLACER-SELECT/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-SEAM-PLACER-SELECT --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-3, AC-N2, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. AC-N2 obliges the empty-Random witness to be executed and classified (reproduce the panic first; fix only if confirmed, with a named concrete RNG/context-independent regression giving root cause + failing input). Mutation controls: return a nonmember, wrong tie winner -- each rejected by the same checker. `B` placer baselines pass; replay `--seed 42` reproduces the concrete pre-shrink input stream.

## M-SEAM-PLACER-PROJECT

Registry record: `id = "M-SEAM-PLACER-PROJECT"`, `package = "seam-placer"`, `target_kind = "lib"`,
`target_name = "seam_placer"`, `test_name = "property_tests::proptest_placer_project"`, `production_symbol = "seam_placer::project_onto_wall_segment"`,
`production_path = "modules/core-modules/seam-placer/src/lib.rs"`, `features = ["seam-placer/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-seam-placer-project"`.

### Stable property ID

`M-SEAM-PLACER-PROJECT` -- semantic, independent of packet numbers.

### Owner

Package `seam-placer`; lib target `seam_placer`; module `property_tests` in
`modules/core-modules/seam-placer/src/property_tests.rs`; production symbol `seam_placer::project_onto_wall_segment` in
`modules/core-modules/seam-placer/src/lib.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_wall_points, max_walls, max_coord_quanta`.

### Contract

Continuous aligned-seam projection onto the nearest ACTUAL wall segment, with an independent clamped dot-product/segment-distance oracle. For a CLOSED loop (explicit closing repeat) interior insertion interpolates continuous fields (XY/Z, width, flow_factor, dist_to_top_mm) and `width_profile.widths` linearly, while discrete metadata (`overhang_quartile`, `overhang_distance_mm`, `WallFeatureFlags`) selects start at `t<=0.5` else end; the closing repeat is re-appended. For an OPEN polyline, last-to-first is NOT a real segment and the path MUST stay open: `docs/02_ir_schemas.md` IR 7 says "Nothing may fabricate closure for an open line" and per-point arrays are parallel. AC-N2 requires REPRODUCTION AND CLASSIFICATION before a defect is confirmed: source inspection shows `project_onto_wall_segment` wraps last->first for open paths and `insert_projected_point` unconditionally appends a closing point/width (breaking `flags==points==widths`). If reproduced and the IR-7 open-geometry contract confirmed, a focused fix preserves open geometry with a named concrete regression. The XY-only `ExtrusionPath3D::is_closed` vs full-record equality mismatch is classified as a separate representation witness.

### Domain

Small finite closed loops (explicit closing duplicate) AND open polylines; distinct per-point widths and flags; bounded 1/1024-mm coordinates. Bounds `max_wall_points=16, max_walls=4, max_coord_quanta=8192`. Interior/endpoint/zero-length/equal-distance/empty-wall cases; `1e-5` t-snap and `t=0` zero-length accounted with separate boundary witnesses; equal-distance ties choose lower wall index then segment start.

### Strategy

Nonshrinking selector for closed-vs-open topology and segment position; coordinates shrink while preserving interior insertion and parallel array lengths. Positive counters for interior projections and closed-loop metadata interpolation, asserted `>0`; empty-wall/endpoint cases asserted separately.

### Oracle

Independently computes the nearest actual segment and t by clamped dot product / bounded f32 distance; NEVER calls `project_onto_wall_segment`/`insert_projected_point` to derive the expected projection. Open paths are verified to remain open with parallel arrays.

### Tolerance

Exact retained metadata (flags/widths/discrete fields) comparison; a derived bounded f32 error for computed coordinates/widths; `1e-5` parameter snap handled by keeping ordinary points away from the snap boundary (separate boundary witnesses).

### Shrinking

Combined controls/shrink test; nonshrinking topology selector; shrink states re-check parallel array lengths, interior t in (0,1), domain bounds and the required witness.

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["seam-placer/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-SEAM-PLACER-PROJECT --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-SEAM-PLACER-PROJECT/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-SEAM-PLACER-PROJECT --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-3, AC-N2, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. AC-N2 obliges the open-polyline implicit-closing-edge witness to be executed and classified; fix only after reproduction with a named concrete regression. If a confirmed fix changes an existing open-wall example, it must be updated to the corrected open/closed contract WITHOUT deleting or weakening coverage (and still pass). Mutation controls: put projection off the actual segment, alter interpolated width/discrete flag, close an open path -- each rejected by the same checker. `B` placer baselines pass.

## M-SEAM-PLACER-ROTATE

Registry record: `id = "M-SEAM-PLACER-ROTATE"`, `package = "seam-placer"`, `target_kind = "lib"`,
`target_name = "seam_placer"`, `test_name = "property_tests::proptest_placer_rotate"`, `production_symbol = "seam_placer::rotate_wall_loop"`,
`production_path = "modules/core-modules/seam-placer/src/lib.rs"`, `features = ["seam-placer/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-seam-placer-rotate"`.

### Stable property ID

`M-SEAM-PLACER-ROTATE` -- semantic, independent of packet numbers.

### Owner

Package `seam-placer`; lib target `seam_placer`; module `property_tests` in
`modules/core-modules/seam-placer/src/property_tests.rs`; production symbol `seam_placer::rotate_wall_loop` in
`modules/core-modules/seam-placer/src/lib.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_wall_points, max_coord_quanta`.

### Contract

Closed-loop rotation compares the effective cyclic point sequence and paired flags/widths by an independently chosen start index, then one closing repeat. Open paths remain unchanged in EVERY field by `rotate_wall_loop` (it early-returns a clone for `!path.is_closed()`), so rotating a closed sequence changes only the starting index -- not geometry or orientation. The repeated endpoint is distinguished from effective vertices; correspondence is by vertex identity, not lengths alone. Same-XY/different-metadata closing endpoints are a separately classified representation witness (XY-only `is_closed` vs full-record equality).

### Domain

Small finite closed loops with explicit closing duplicates and open polylines; distinct per-point widths/flags; bounded 1/1024-mm coordinates. Bounds `max_wall_points=16, max_coord_quanta=8192`.

### Strategy

Nonshrinking selector for closed-vs-open topology and start index; coordinates shrink. Positive counters for closed cyclic rotations, asserted `>0`; open-path preservation asserted separately in every field.

### Oracle

Independently computes the expected rotated sequence by cyclic zipped `(point, flags, width)` shift + one closing repeat; NEVER calls `rotate_wall_loop` to derive the expected order. Open paths expected byte-for-byte unchanged.

### Tolerance

Exact retained metadata comparison; exact vertex-identity correspondence; derived bounded f32 error only where coordinates are calculated elsewhere. No orientation reversal asserted.

### Shrinking

Combined controls/shrink test; nonshrinking topology/start selectors; shrink states re-check cyclic correspondence and the required witness; selectors asserted not to simplify.

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["seam-placer/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-SEAM-PLACER-ROTATE --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-SEAM-PLACER-ROTATE/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-SEAM-PLACER-ROTATE --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-3, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. Mutation controls: rotate points without matching width/flag, lose the closing repeat, reverse orientation, close an open path -- each rejected by the same checker. `B` placer baselines pass.

## M-PATH-NEAREST

Registry record: `id = "M-PATH-NEAREST"`, `package = "path-optimization-default"`, `target_kind = "lib"`,
`target_name = "path_optimization_default"`, `test_name = "property_tests::proptest_path_nearest"`, `production_symbol = "path_optimization_default::nearest_neighbor_permutation"`,
`production_path = "modules/core-modules/path-optimization-default/src/lib.rs"`, `features = ["path-optimization-default/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-path-nearest"`.

### Stable property ID

`M-PATH-NEAREST` -- semantic, independent of packet numbers.

### Owner

Package `path-optimization-default`; lib target `path_optimization_default`; module `property_tests` in
`modules/core-modules/path-optimization-default/src/property_tests.rs`; production symbol `path_optimization_default::nearest_neighbor_permutation` in
`modules/core-modules/path-optimization-default/src/lib.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_candidates, max_coord_quanta`.

### Contract

Local greedy nearest-neighbour behaviour (NOT a globally shortest tour). The cursor begins at the origin and advances to the chosen candidate's end; the source documents a `0.001 mm` tie window with earlier candidate index winning. Candidate-index and original-index bijections are checked independently with `oracles::is_permutation` / a local seen-set, including empty and singleton. Exact two-candidate ties and separated-distance witnesses may assert the selected first index from an independent distance calculation.

### Domain

Finite bounded start/end positions on a 1/1024 grid. Bounds `max_candidates=16, max_coord_quanta=8192`. Empty and singleton included; no globally optimal route claim.

### Strategy

Nonshrinking selector for tie/geometry class; positions shrink. Positive counters for nonempty permutations and exact-tie wins, asserted `>0`; empty/singleton asserted separately.

### Oracle

Independently computes the greedy first choice by its own bounded distance calculation; NEVER calls `nearest_neighbor_permutation` to derive the expected first index. Bijection via independent `is_permutation`/seen-set.

### Tolerance

Bounded f32 squared-distance arithmetic; the `0.001 mm` tie window treated exactly; no global-optimality tolerance.

### Shrinking

Combined controls/shrink test; nonshrinking class selector; shrink states re-check bijection, bounds and the required witness.

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["path-optimization-default/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-PATH-NEAREST --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-PATH-NEAREST/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-PATH-NEAREST --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-4, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. Mutation controls: duplicate/drop an index, violate an exact tie winner -- each rejected by the same checker. `B` path baselines pass.

## M-PATH-LOCKS

Registry record: `id = "M-PATH-LOCKS"`, `package = "path-optimization-default"`, `target_kind = "lib"`,
`target_name = "path_optimization_default"`, `test_name = "property_tests::proptest_path_locks"`, `production_symbol = "path_optimization_default::coalesce_locked_candidates"`,
`production_path = "modules/core-modules/path-optimization-default/src/lib.rs"`, `features = ["path-optimization-default/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-path-locks"`.

### Stable property ID

`M-PATH-LOCKS` -- semantic, independent of packet numbers.

### Owner

Package `path-optimization-default`; lib target `path_optimization_default`; module `property_tests` in
`modules/core-modules/path-optimization-default/src/property_tests.rs`; production symbol `path_optimization_default::coalesce_locked_candidates` in
`modules/core-modules/path-optimization-default/src/lib.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_entities, max_coord_quanta`.

### Contract

ADR-0062 lock preservation. An untagged item is a singleton; each maximal adjacent equal `Some(order_lock)` run becomes one candidate with original indices in authored order, first start and last end. For a valid locked block in one `(layer, object, region)`, output positions are compared to input via `oracles::locked_groups_are_contiguous` plus explicit `false` reversal flags: the block may move as a unit, never split or reverse. Locks take precedence over role/tool grouping that would split one. ADR-0062 does not limit a valid lock to one role or tool; contiguous same-region multi-role and multi-tool blocks are exercised where the producer/view can represent them.

### Domain

Finite bounded `OrderedEntityView` positions on a 1/1024 grid; bounded entity counts; adjacent distinct tags and same-tag-separated-by-`None` characterization. Bounds `max_entities=16, max_coord_quanta=8192`. `OrderedEntityView` (8 fields, no Default) built via FRU on `ordered_entity_view_base(role)` or an `// exhaustive:` waiver.

### Strategy

Nonshrinking selector for lock pattern; positions/counts shrink while preserving contiguous authored-order runs. Positive counters for locked block moves-as-unit and first-start/last-end coalescing, asserted `>0`; empty/singleton asserted separately.

### Oracle

Independently derives adjacent runs from the input and checks contiguity/direction with `locked_groups_are_contiguous`; NEVER calls `coalesce_locked_candidates` to derive the expected grouping. A nearer unrelated singleton and a first-vs-last-endpoint witness included.

### Tolerance

Exact index/order/tag comparison; bounded f32 coordinate arithmetic; exact `false` reversal flags. Malformed separated tags are characterized, not claimed ADR-0062-valid.

### Shrinking

Combined controls/shrink test; nonshrinking pattern selector; shrink states re-check contiguity, direction and the required witness.

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["path-optimization-default/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-PATH-LOCKS --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-PATH-LOCKS/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-PATH-LOCKS --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-4, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. Mutation controls: split/reverse a locked block or set a reversal flag, substitute the first block end for the last, duplicate/drop an index -- each rejected by the same checker. `B` path baselines pass.

## M-PATH-GROUPS

Registry record: `id = "M-PATH-GROUPS"`, `package = "path-optimization-default"`, `target_kind = "lib"`,
`target_name = "path_optimization_default"`, `test_name = "property_tests::proptest_path_groups"`, `production_symbol = "path_optimization_default::PathOptimizationDefault::group_then_nearest_neighbor"`,
`production_path = "modules/core-modules/path-optimization-default/src/lib.rs"`, `features = ["path-optimization-default/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#m-path-groups"`.

### Stable property ID

`M-PATH-GROUPS` -- semantic, independent of packet numbers.

### Owner

Package `path-optimization-default`; lib target `path_optimization_default`; module `property_tests` in
`modules/core-modules/path-optimization-default/src/property_tests.rs`; production symbol `path_optimization_default::PathOptimizationDefault::group_then_nearest_neighbor` in
`modules/core-modules/path-optimization-default/src/lib.rs` (private kernel, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`.
Bound keys: `max_entities, max_coord_quanta`.

### Contract

Applicable role/tool grouping with staged wall order. For ordinary unlocked/compatible inputs: ascending tool clusters, skirt before the staged wall block, walls preserved in staged order, then documented non-wall role groups. Never asserts that the optimizer re-sorts `InnerWall` before `OuterWall`; expected roles are derived independently, NOT by calling `role_group`. The archived `docs/spec_packets/_OLD/61_path-optimization-role-sequencing.md` suggests new infill roles such as `InternalBridgeInfill` belong with infill, which is archived design-only evidence, NOT by itself an applicable live contract: the current `role_group` `_ => 9` wildcard (taking `InternalBridgeInfill`, `InternalSolidInfill`, `RaftInfill`, `SupportBaseInterface`, `Brim`) is INVESTIGATION material: reproduce/classify the witness; fix only if the applicable live contract is confirmed, otherwise characterize.

### Domain

Finite bounded `OrderedEntityView` positions on a 1/1024 grid; bounded entity counts; authored contiguous cross-group locks and an `InternalBridgeInfill`/`SupportMaterial` witness. Bounds `max_entities=16, max_coord_quanta=8192`.

### Strategy

Nonshrinking selector for role/tool mix; positions/counts shrink. Positive counters for role/tool groups, staged wall order and a cross-group locked block, asserted `>0`; degenerate cases asserted separately.

### Oracle

Independently computes the expected group order from an authored role table (skirt, walls, infill, ironing, support, towers) and verifies lock precedence; NEVER calls `role_group` to derive expected roles. Wildcard-role behaviour recorded as characterization evidence.

### Tolerance

Exact group/order/index comparison; bounded f32 coordinate arithmetic; no optimality claim.

### Shrinking

Combined controls/shrink test; nonshrinking mix selector; shrink states re-check group order, staged walls and the required witness.

### Execution

Both tiers (measured Windows profiles, values unchanged from the pilot; evidence in `docs/testing/proptest-layers-seams-evidence.md` §9.4): routine 64 cases / size 32 / shrink 64, extended 256 / 128 / 256;
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`,
`max_shrink_time_ms = 0`, zero rejection/regeneration budgets. Features `["path-optimization-default/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`. Selected by
`cargo xtask test --summary --properties --property M-PATH-GROUPS --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/M-PATH-GROUPS/`.
Plain `cargo test` of a generated property is NOT acceptance (missing context). Windows
routine lane includes this record. Retained-seed reproduction command: `cargo xtask test --summary --properties --property M-PATH-GROUPS --tier <tier> --seed <seed>` (the replay reproduces the concrete input stream once measured; `--seed 42` is the Step-6 replay for the selected property).

### Failure record

`failure.json` retains the classification (`sut_or_contract` / `panic` / `exhausted` /
`process_failure` / `generator_or_oracle` / `harness`), the full context, the concrete/debug input,
the tier-bearing reproduction command, an executable direct reproduction, captured SDK logs and
`shrink_completed` (null for pure Proptest). The foundation registry owns discovery/execution/context/
profile identity; no hand-maintained roster. Waivers `// exhaustive: <reason>` and
`// test-quality: <reason>` name their protected surface; `modules/core-modules/*/src/**` is
outside both scanner scopes, so a green `check-literals` / `check-test-quality` is NOT evidence
for this file (manual review recorded in the evidence doc).

### Acceptance

Maps to AC-4, AC-N1, AC-6. The combined `packet15_witnesses_controls_and_shrinking` accepts every
unmodified positive witness and rejects each named mutation with the SAME checker; positive
counters cannot be satisfied by empty cases. Mutation controls: reorder staged walls, put support before the documented infill witness, split a cross-group lock -- each rejected by the same checker. `B` path baselines pass. `InternalBridgeInfill` wildcard is investigated and classified; fix only if the applicable contract is confirmed (archived evidence alone justifies characterization, not a change).

## H-OVERHANG-SPEED

Registry record: `id = "H-OVERHANG-SPEED"`, `package = "overhang-classifier-default"`, `target_kind = "test"`,
`target_name = "property_overhang_speed"`, `test_name = "proptest_overhang_speed_sections"`,
`production_symbol = "overhang_classifier_default::calculate_speed"`,
`production_path = "modules/core-modules/overhang-classifier-default/src/lib.rs"`,
`features = ["overhang-classifier-default/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-overhang-speed"`.

### Stable property ID

`H-OVERHANG-SPEED` -- semantic, independent of packet numbers.

### Owner

Package `overhang-classifier-default`; test target `property_overhang_speed` in
`modules/core-modules/overhang-classifier-default/tests/property_overhang_speed.rs`; production symbols
`overhang_classifier_default::build_speed_sections` and
`overhang_classifier_default::calculate_speed` in `modules/core-modules/overhang-classifier-default/src/lib.rs`
(both public). Bound keys: `max_path_width_mm`, `max_speed_mm_s`, `max_interpolation_samples`.

### Contract

`build_speed_sections` always returns exactly six `(distance, speed)` entries derived from
`OVERHANG_OVERLAP_LEVELS = [90, 75, 50, 25, 13, 0]`: distances `path_width * (1 - level/100)`, speeds
`[ref_speed, rule(overhang_1_4), rule(overhang_2_4), rule(overhang_3_4), rule(overhang_4_4),
slowdown ? rule(overhang_4_4) : bridge_speed]` where `rule(v) = v < 0.5 ? ref_speed : v`. The output is
sorted ascending by f32 distance with ties by descending speed, and every run of equal f32 distance is
flattened to the run's first (highest) speed **without dropping entries**. `calculate_speed(d, sections,
original)` is `original` for empty sections and for `d <= first`; `min(round(last_section_speed), original)`
for `d >= last`; and `min(round(clamped linear interpolation), original)` inside an interval. The
`>= last` branch is the round-then-cap fix for the confirmed defect (see below). This is a PnP contract, not
an exact OrcaSlicer parity claim: canonical drops duplicate entries and dereferences the empty domain.

### Domain

Positive finite `ref_speed` and `path_width` bounded by `max_path_width_mm`/`max_speed_mm_s`; configured
overhang speeds `>= 0` including the `< 0.5` reference sentinel; finite `bridge_speed >= 0`; both
`slowdown_for_curled_perimeters` values; a complete `ConfigView`. Interpolation sections are dyadic and
strictly increasing (empty, singleton and `>= 2` samples), samples are at boundaries, exact dyadic
in-interval fractions, below the first and above the last, with integer-valued `original_speed`.
Duplicate/unsorted/negative/NaN sections are separate labelled boundary observations, never the only
generated strategy; `path_width = 0` is the duplicate-distance boundary witness.

### Strategy

Construction-first: a mode selector (`prop::sample::select(...).no_shrink()`) picks the section-build or
interpolation family; sizes shrink. Mandatory witnesses on every run: the `path_width = 0` six-zero-distance
case (all six retained, all flattened to the highest speed), the `< 0.5` sentinel, both sixth-slot branches,
the last-boundary cap, and the `[(0,10),(1,20)]` / `original = 30` `<= first` monotonicity counterexample.

### Oracle

An independent insertion sort plus run flattening recomputes the six entries from the authored table; the
SUT is never called for an expected value. Interpolation is recomputed with `f64` products from the f32
inputs, rounded half-away-from-zero and capped, then compared by f32 bits. Shared dependencies: the
documented six overlap levels and the `f32::round`/`min` semantics; the production sort/flatten loop is not
copied. No canonical-parity claim beyond the verified round-then-cap behaviour.

### Tolerance

Exact f32 bit equality on the declared dyadic domain (sections, samples and `original_speed` are chosen so
products/sums/round are exact). No approximate speed comparison.

### Shrinking

Distances/widths/speeds simplify; the selected mode, the six-entry multiplicity and any required duplicate
distance survive. `no_shrink` pins the mode selector. A real shrink/complicate probe simplifies a nonminimal
value tree to exhaustion, re-validates, complicates and re-validates.

### Execution

Both tiers; bounds `max_path_width_mm=4, max_speed_mm_s=200, max_interpolation_samples=8`.
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero
rejection/regeneration budgets. Features `["overhang-classifier-default/property_testing"]`.
Selected by `cargo xtask test --summary --properties --property H-OVERHANG-SPEED --tier <routine|extended>`;
context via `PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-OVERHANG-SPEED/`.
Plain `cargo test` of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification (`sut_or_contract`/`panic`/`exhausted`/`process_failure`/
`generator_or_oracle`/`harness`), full context, concrete/debug input, tier-bearing reproduction command, an
executable direct reproduction, captured SDK logs and `shrink_completed` (null for pure Proptest).

### Acceptance

Maps to AC-9 and AC-N4. Confirmed defect, reproduced before the fix: `calculate_speed`'s `distance >= last`
branch returned the raw section speed, so `sections = [(0.0, 1.0), (1.0, 1.75)]`, `original_speed = 2.0`
returned `1.75` at `d = 1.0` (a spurious decrease from the in-interval `2.0`). Fixed to
`sections[len-1].1.round().min(original_speed)`; named concrete regression
`calculate_speed_last_boundary_rounds_and_caps_last_section_speed` in `tests/basic_tdd.rs` states the former
input/output. Controls reject the missing sentinel, wrong sixth slot, dropped duplicate entry, ascending
ties, unequal tie speeds, wrong interval selection, missing round and the missing last-boundary round-and-cap.
Restricted monotonicity is asserted only for `first_distance < a < b <= last_distance` with nondecreasing
section speeds and integer `original_speed`; no global monotonicity and no fan/speed monotonicity claim.
## H-GCODE-TEMPLATE

Registry record: `id = "H-GCODE-TEMPLATE"`, `package = "machine-gcode-emit"`, `target_kind = "lib"`,
`target_name = "machine_gcode_emit"`, `test_name = "property_tests::proptest_placeholder_substitution"`,
`production_symbol = "machine_gcode_emit::substitute_placeholders"`,
`production_path = "modules/core-modules/machine-gcode-emit/src/lib.rs"`,
`features = ["machine-gcode-emit/property_testing"]`, `requires_host_algos = false`, `requires_scad = false`,
`generator_version = 1`, `charter = "docs/testing/properties.md#h-gcode-template"`.

### Stable property ID

`H-GCODE-TEMPLATE` -- semantic, independent of packet numbers.

### Owner

Package `machine-gcode-emit`; lib target `machine_gcode_emit`; module `property_tests` in
`modules/core-modules/machine-gcode-emit/src/property_tests.rs`; production symbols
`machine_gcode_emit::substitute_placeholders` and `format_placeholder_value` (private kernels, metadata
only -- no new public export). Registered from `src/lib.rs` under stacked `#[cfg(test)]` /
`#[cfg(feature = "property_testing")]`. Bound keys: `max_template_bytes`, `max_placeholders`.

### Contract

Single-pass, byte-oriented placeholder substitution: at `[` search for `]` before the next newline; if found,
substitute the lookup value verbatim and advance past `]`; if absent, copy the rest of the line literally.
Substituted values are never rescanned. Unresolved keys stay verbatim and are returned sorted and
deduplicated. `format_placeholder_value` renders scalars, recurses on a `List`'s first element until a
renderable scalar or an empty list (empty string) and returns `None` for unsupported variants
(`Percent`/`FloatOrPercent`), leaving the placeholder verbatim and warned about. Canonical
`PlaceholderParser::process`/`process_macro` throws on an unknown key; PnP's warn-and-pass is deliberate and
is not claimed as canonical parity. `run_gcode_postprocess` with empty templates preserves every
`GCodeCommand` in order and re-emits `ExtrusionMode` as `Raw("M82")`/`Raw("M83")`.

### Domain

Templates of bounded bytes built from a construction-first token sequence: literal runs (including Unicode,
empty runs and no `[`), `[key]` placeholders with keys from a bounded lookup, adjacent placeholders, empty
`[]`, unclosed `[`, a `]` before any `[`, nested `[[key]]` (including a non-ASCII key) and newlines. One
malformed shape per case is forced by a `no_shrink` mode selector; an excess-token budget drops (never
rejects) tokens. Lookup values include strings containing `[other]` to prove substituted values are not
rescanned.

### Strategy

Construction-first token sequence, not arbitrary malformed bytes. Mandatory witnesses every run: the
no-rescan value, sorted/deduplicated unresolved keys, an empty list, a list whose first element is itself a
list, an unsupported variant, and the command-order passthrough over all nine `GCodeCommand` variants
(compiler-enforced exhaustive roster in the test).

### Oracle

An independent line-split left-to-right scanner (structurally different from the production flat byte
index) plus an independent `ValueKind` table for `format_placeholder_value`. Shared dependencies: the
lookup map's `BTreeMap` ordering (sorted output) and the existing `run_gcode_postprocess` entrypoint;
the production scanner is never called to derive an expected value.

### Tolerance

Exact string equality for the rendered text and exact vector equality for the unresolved keys; no
normalization.

### Shrinking

Placeholders/literal bytes simplify; the selected malformed mode survives through a `no_shrink` selector and
a present/absent/unclosed witness must remain. A real shrink/complicate probe simplifies a nonminimal tree
to exhaustion for each malformed shape, asserts `changed = true`, re-validates, complicates and re-validates.

### Execution

Both tiers; bounds `max_template_bytes=64, max_placeholders=6`. `rng_algorithm = "chacha"`,
`isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero rejection/regeneration
budgets. Features `["machine-gcode-emit/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-GCODE-TEMPLATE --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-GCODE-TEMPLATE/`. Plain
`cargo test` of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed` (null for pure
Proptest). `modules/core-modules/*/src/**` is outside both scanner scopes, so a green `check-literals` /
`check-test-quality` is NOT evidence for this file; the manual FRU/test-quality review is recorded in the
file header and the evidence doc.

### Acceptance

Maps to AC-7 and AC-N3. Exact rendered text and sorted/deduplicated unresolved vectors pass over 64 routine
cases with zero rejects; the non-recursive, first-element-recursion, empty-list and unsupported-variant
behaviours are pinned deterministically. `template_controls` rejects recursion into substituted values,
unsorted and duplicated unresolved keys, an empty-string substitute for an empty list and a command-order
swap -- each by the same checker after first accepting the honest claim. The three machine-gcode-emit
baselines pass unchanged.
## H-COOLING

Registry record: `id = "H-COOLING"`, `package = "part-cooling"`, `target_kind = "lib"`,
`target_name = "part_cooling"`, `test_name = "property_tests::proptest_cooling_decision_table"`,
`production_symbol = "part_cooling::PartCooling::cooling_decision_for_event"`,
`production_path = "modules/core-modules/part-cooling/src/lib.rs"`,
`features = ["part-cooling/property_testing"]`, `requires_host_algos = false`, `requires_scad = false`,
`generator_version = 1`, `charter = "docs/testing/properties.md#h-cooling"`.

### Stable property ID

`H-COOLING` -- semantic, independent of packet numbers.

### Owner

Package `part-cooling`; lib target `part_cooling`; module `property_tests` in
`modules/core-modules/part-cooling/src/property_tests.rs`; production symbols
`part_cooling::PartCooling::layer_fan_speed` (private) and
`part_cooling::PartCooling::cooling_decision_for_event` (public). Registered from `src/lib.rs` under stacked
`#[cfg(test)]` / `#[cfg(feature = "property_testing")]`. Bound keys: `max_layer_index`, `max_close_layers`.

### Contract

A pure layer/event decision table. `layer_fan_speed(layer) = 0` when `layer <
close_fan_the_first_x_layers`, else `fan_max_speed`. `cooling_decision_for_event(view) =
(overhang_fan_speed as u16 * fan_max_speed as u16) / 100` when `enable_overhang_bridge_fan` and any ordered
entity has role `BridgeInfill` (the only overhang role), else the layer base. The `u16` multiplication
cannot wrap `u8` under the declared manifest ranges (`overhang_fan_speed <= 100`, `fan_max_speed <= 255`).
No fan-speed monotonicity is claimed: the branch is a table, and a higher percentage can leave the integer
result unchanged. Canonical `CoolingBuffer::apply_layer_cooldown` is a richer reference; this is a
simplified PnP decomposition, not a full parity claim.

### Domain

`fan_max_speed` in `[0,255]`, `close_fan_the_first_x_layers` in `[0,max_close_layers]`,
`overhang_fan_speed` in `[0,100]`, both `enable_overhang_bridge_fan` values (a `no_shrink` mode), layer
indices before/at/after/random around the close boundary, and entity role sequences over
Empty/SingleOverhang/SinglePlain/OverhangAndPlain/MultiRole. `LayerCollectionIR`/`LayerCollectionView` are
built through the existing `slicer_sdk::test_support::fixtures` bases. Raw values above the declared ranges
are outside the valid config domain and are not used to assert monotonic behaviour.

### Strategy

Construction-first; `no_shrink` pins the enable/role modes and the boundary class, while sizes shrink.
Non-vacuity counters assert boundary, below-boundary, `fan_max = 0`, empty-role, enabled-bump and
disabled-with-overhang populations are all exercised.

### Oracle

The exact integer table above, recomputed in `u16` with an exact `u8` conversion, compared for both
`layer_fan_speed` (private) and `cooling_decision_for_event`. No call to either SUT function produces an
expected value. Shared dependency: the manifest-declared key ranges, which make the `u16 -> u8` conversion
lossless.

### Tolerance

Exact integer equality; no percentage rounding tolerance and no monotonicity predicate.

### Shrinking

Layers/entities and the boundary distance simplify; the overhang/no-overhang and fan-disabled/enabled modes
survive. A real shrink/complicate probe covers all five role modes x both enable values and re-validates
after each transition.

### Execution

Both tiers; bounds `max_layer_index=8, max_close_layers=4`. `rng_algorithm = "chacha"`,
`isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero rejection/regeneration
budgets. Features `["part-cooling/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-COOLING --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-COOLING/`. Plain `cargo test`
of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed` (null for pure
Proptest). `modules/core-modules/*/src/**` is outside both scanner scopes; the manual FRU/test-quality review
is recorded in the file header and the evidence doc.

### Acceptance

Maps to AC-8 and AC-N3. The exact table passes over 64 routine cases with zero rejects, including
`fan_max = 0`, `close_layers = 0`, the boundary layer, overhang present/absent and enable false.
`cooling_controls` rejects a strict `<=` boundary, a percentage computed from the wrong base, an overhang
bump with the feature disabled and a role classifier that treats `InternalBridgeInfill` as overhang -- each
by the same checker. The two part-cooling baselines pass unchanged.
## H-GCODE-FEEDRATE

Registry record: `id = "H-GCODE-FEEDRATE"`, `package = "slicer-gcode"`, `target_kind = "test"`,
`target_name = "property_gcode"`, `test_name = "proptest_feedrate_resolution"`,
`production_symbol = "slicer_gcode::emit::DefaultGCodeEmitter::resolve_feedrate"`,
`production_path = "crates/slicer-gcode/src/emit.rs"`, `features = ["slicer-gcode/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-gcode-feedrate"`.

### Stable property ID

`H-GCODE-FEEDRATE` -- semantic, independent of packet numbers.

### Owner

Package `slicer-gcode`; test target `property_gcode` in
`crates/slicer-gcode/tests/property_gcode.rs`; production symbol
`slicer_gcode::emit::DefaultGCodeEmitter::resolve_feedrate` in `crates/slicer-gcode/src/emit.rs` (public).
Bound keys: `max_base_speed_mm_s`, `max_factor_milli` (factor = milli/1000).

### Contract

`resolve_feedrate(role, factor)` always returns `Some`: the role selects a configured base speed (an
independent 23-slot table covering every `ExtrusionRole`, the `InternalBridgeInfill` percent-versus-absolute
rule, the `filament_ironing_speed > 0` override and the `Custom("Travel"/"Wipe"/"GapInfill")` names), the
factor is clamped to `[0.05, 5.0]`, and the wire value is
`round(f64(base) * 60.0 * f64(clamped) * 1000.0) / 1000.0` narrowed to f32. A zero base yields
`Some(0.0)`, never an error and never a fabricated automatic speed (`resolve_extrusion_feedrate` is the
automatic path and is covered separately by `H-GCODE-EMIT`). **The clamp is PnP ADR-0052, not OrcaSlicer
parity**: a delegated read of the canonical checkout found no such clamp in `GCode::_extrude_piece`,
`GCodeWriter::set_speed` or the `GCode/` extrusion path.

### Domain

Bounded finite bases in `[0, max_base_speed_mm_s]` for every role, and finite factors (as milli-units)
spanning `[0, max_factor]` including `0`, negative values, values below `0.05`, above `5.0` and the exact
`0.05`/`5.0` ends. The overflow probe derives `max_finite_base = f32::MAX / (60.0 * 5.0)` and shows a larger
finite base narrows to `f32::INFINITY` (an overflow observation, not a valid rate). Non-finite factors are a
separately labelled boundary observation: `+/-inf` saturate through `f32::clamp` and NaN propagates; none is
asserted as correct.

### Strategy

Construction-first per-role/factor pairs with `no_shrink` on the over/under-clamp and role mode; magnitudes
shrink. Mandatory witnesses every run: zero base, exact clamp ends, ironing both branches, internal-bridge
percent and absolute, and the four `Custom` names.

### Oracle

The independent role table plus the f64 multiply/round/narrow formula, compared by f32 bits. Shared
dependency: Rust's documented `f64::round` half-away-from-zero semantics. `role_base_speed_mm_per_s` is
never called to derive an expected value; the production clamp/conversion seam is not reused.

### Tolerance

Exact f32 bit equality. No approximate speed comparison and no "roundtrip" claim beyond the declared dyadic
witnesses.

### Shrinking

Base/factor magnitudes simplify; the selected role and the over/under-clamp mode survive via `no_shrink`.
A real shrink/complicate probe re-validates after each transition.

### Execution

Both tiers; bounds `max_base_speed_mm_s=1000, max_factor_milli=10000`. `rng_algorithm = "chacha"`,
`isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero rejection/regeneration
budgets. Features `["slicer-gcode/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-GCODE-FEEDRATE --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-GCODE-FEEDRATE/`. Plain
`cargo test` of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed` (null for pure
Proptest).

### Acceptance

Maps to AC-4, AC-N2 and AC-N4. Passes over 64 routine cases with zero rejects; the role table, clamp,
60x conversion and round-to-millidegree match by bits. `gcode_feedrate_controls` rejects a missing 60x
conversion, a `[0,1]` clamp, truncation instead of round-to-millidegree, a role-table swap and an
unjustified exact-roundtrip claim, each by the same checker. The three
`gcode_feedrate_emission_tdd` baselines and the `gcode_emit_format_tdd` formatter baseline pass unchanged.

## H-GCODE-EMIT

Registry record: `id = "H-GCODE-EMIT"`, `package = "slicer-gcode"`, `target_kind = "test"`,
`target_name = "property_gcode"`, `test_name = "proptest_emission_state_sequence"`,
`production_symbol = "slicer_gcode::emit::DefaultGCodeEmitter::emit_gcode"`,
`production_path = "crates/slicer-gcode/src/emit.rs"`, `features = ["slicer-gcode/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-gcode-emit"`.

### Stable property ID

`H-GCODE-EMIT` -- semantic, independent of packet numbers.

### Owner

Package `slicer-gcode`; test target `property_gcode` in
`crates/slicer-gcode/tests/property_gcode.rs`; production symbols
`slicer_gcode::emit::DefaultGCodeEmitter::{emit_gcode,emit_gcode_owned}` in `crates/slicer-gcode/src/emit.rs`
(public). Bound keys: `max_layers`, `max_entities_per_layer`, `max_points_per_entity`, `max_abs_mm`.

### Contract

A small command/state model over the real emitter. The stream starts with
`ExtrusionMode { absolute: !relative }`; M73 is disabled and never expected. A layer with output emits
`Raw(";LAYER_CHANGE")`, `Raw(";Z:<...>")`, `Raw(";HEIGHT:<...>")` before its first move; a layer with no
output emits nothing and does not increment `metadata.layer_count`; a layer with only annotations, or with
only a side-table entry and no ordered entities, is a no-output layer. Entity order is preserved; each
entity's first `Move` has `e = None`, subsequent positive-length Moves carry the cumulative absolute E,
zero-length segments carry `e = None`, x/y/z equal the source points, the role equals the entity role and
`f` equals the feedrate oracle. After each entity the sequence is retract(s) -> Z-hop up -> travel(s) ->
tool change (with the wipe-tower guard) -> annotation(s) -> Z-hop down -> unretract(s). A tool change emits
`ToolChange { after_entity_index, from, to }` and resets the next entity's `;TYPE:` label; a layer-boundary
tool reset precedes the first entity when `tool_index` differs. `metadata.filament_used_mm` has length
`max_tool + 1` over the keys actually present in `filament_per_tool`, with a zero entry for a tool that owns
any processed Move point (including a singleton/non-extruding entity) and no entry for a tool that appears
only in `tool_changes`. Two consecutive entities of the same role emit exactly one `;TYPE:` label,
including `GapFill`, `SupportBaseInterface` and `RaftInfill` (the confirmed defect below).

### Domain

0..=`max_layers` layers with 0..=`max_entities_per_layer` entities of 0..=`max_points_per_entity` dyadic
points; distinct per-layer `entity_id`s; `order_lock = Some(0)` (simplification bypassed); `disable_m73`;
wipe-tower synthesis off in the positive domain; role base speeds nonzero so `f` follows the explicit
feedrate path. Includes empty layers, a skipped empty layer between nonempty layers, an entity-less
side-table layer, annotations, singleton/zero-length entities, a `#11`-ordered nonempty
`RegionKey.variant_chain` as fixture input only, `emit_gcode`/`emit_gcode_owned` parity and the negative
modes: wipe tower enabled with a tool change and no `WipeTower` entity (`MissingToolchangePurge`), a tool
index `>= 1024` on an extruding entity (`ToolIndexOutOfRange`), and the automatic-volumetric branch with a
zero base speed and non-finite geometry/factor (`GCodeEmitError::Emit` messages, never `F0`/`FNaN`).

### Strategy

Construction-first scene builder with `no_shrink` on the empty/skip/tool-change/negative modes; sizes
shrink. A mandatory chain-insensitivity witness compares a nonempty `#11` chain scene against the same scene
with an empty chain (`GCodeIR` carries no `RegionKey`, so this is fixture-only evidence).

### Oracle

An independent state machine that walks `GCodeIR.commands`; it never calls the emitter's geometry math or
reads back a value it is supposed to predict. Shared dependency: the declared `ResolvedConfig` keys and the
tool-change anchor declarations.

### Tolerance

Exact command-kind/order/field equality for the state model; per-tool filament sums use a derived f32
accumulation envelope (documented in the test file). No global minimality or ordering-optimality claim.

### Shrinking

Layers/entities/points and side entries simplify; the empty/nonempty, skip, tool-change/retract/travel/anchor
and chain-witness modes survive via `no_shrink`. A real shrink/complicate probe re-validates.

### Execution

Both tiers; bounds `max_layers=3, max_entities_per_layer=3, max_points_per_entity=4, max_abs_mm=64`.
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero
rejection/regeneration budgets. Features `["slicer-gcode/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-GCODE-EMIT --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-GCODE-EMIT/`. Plain
`cargo test` of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed`.

### Acceptance

Maps to AC-5, AC-N2 and AC-N4. Passes over 64 routine cases with zero rejects. Confirmed defect, reproduced
before the fix: `role_equals` omitted the `GapFill`, `SupportBaseInterface` and `RaftInfill` same-variant
arms, so two consecutive entities of those roles re-emitted an identical `;TYPE:` label (observed
`;TYPE:Gap infill` twice). Fixed by adding the three arms; named concrete regression
`same_role_entities_emit_one_type_label_including_gap_fill` in `tests/gcode_emit_tdd.rs` fails without the
fix and passes with it. `gcode_emission_controls` rejects a missing header, a dropped empty-layer skip, an
unretracted/reordered tool change, a per-entity E accumulator, a dropped singleton zero entry and a
fabricated tool-change-only entry. The `gcode_emit_tdd` baselines pass unchanged.

## H-GCODE-SERIALIZE

Registry record: `id = "H-GCODE-SERIALIZE"`, `package = "slicer-gcode"`, `target_kind = "test"`,
`target_name = "property_gcode"`, `test_name = "proptest_serializer_extrusion_state"`,
`production_symbol = "slicer_gcode::serialize::DefaultGCodeSerializer::serialize_gcode"`,
`production_path = "crates/slicer-gcode/src/serialize.rs"`, `features = ["slicer-gcode/property_testing"]`,
`requires_host_algos = false`, `requires_scad = false`, `generator_version = 1`,
`charter = "docs/testing/properties.md#h-gcode-serialize"`.

### Stable property ID

`H-GCODE-SERIALIZE` -- semantic, independent of packet numbers.

### Owner

Package `slicer-gcode`; test target `property_gcode` in
`crates/slicer-gcode/tests/property_gcode.rs`; production symbols
`slicer_gcode::serialize::DefaultGCodeSerializer::serialize_gcode`, the free `format_coord`/`format_xyz`
and `GcodeFlavor::set_temperature` in `crates/slicer-gcode/src/flavor.rs` (public). Bound keys:
`max_commands`, `max_abs_mm`, `max_abs_e_mm`.

### Contract

An independent text and extrusion-state model in relative and absolute modes with Marlin and
RepRapFirmware temperature forms. `G0` for `Custom("Travel")` else `G1`; XYZ at 3 trimmed decimals; E as the
absolute value in absolute mode and as `current - previous_accumulator` in relative mode; F and retract
speeds at 4 trimmed decimals. Relative Gcode retract `G1 E-<length> F<speed>` subtracts; unretract adds.
**Absolute mode is absolute E**: Moves set the accumulator, `G92 E<value>` resets it in both modes, and a
Gcode retract/unretract updates the accumulator and emits the updated absolute position (never a bare
delta). Firmware `G10`/`G11` leave it unchanged. `FanSpeed` is `M106 S<v>`; `Temperature` uses the selected
flavor (`M104`/`M109` vs `G10 P<n> S<c>` + `M116`); `T<n>`; `; <text>`; Raw verbatim; `M82`/`M83`. Each
stream opens with exactly one matching `ExtrusionMode`.

### Domain

0..=`max_commands` commands built directly as `GCodeIR` (`Move`, `Retract`, `Unretract`, `FanSpeed`,
`Temperature`, `ToolChange`, `Comment`, `Raw`, `ExtrusionMode`) with dyadic XYZ (<= 3 decimals), dyadic
absolute E and retract/unretract lengths (<= 5 decimals), integer Celsius values, bounded ASCII text
(including `G92 E0`, `G92 E1.5`, `G92 X0`, unparseable `G92 E`, a non-`G92` line), and no non-finite numeric
field in the valid domain. Duplicate/negative/unparseable E and mode-switch fixtures are separate labelled
text-ordering observations.

### Strategy

Construction-first command sequence with `no_shrink` on the mode/flavor and the G92/mode-switch modes.
Mandatory witnesses: a relative retract/unretract pair, an absolute retract/unretract pair, a firmware pair
in both modes, a G92 reset, a `-0`/rounding boundary and the exact header/width block.

### Oracle

An independent renderer and accumulator recomputed from the documented contract; a deterministic
`gcode_serializer_formatting_probe` pins the formatter's actual tie/`-0` behaviour before it is used.
Shared dependency: the documented `format_coord` wire shape (4 decimals, trimmed) and the flavor's
temperature command forms. The SUT is never called to produce expected text.

### Tolerance

Exact string equality on the generated domain; a tolerance is not a substitute for a formatting policy.

### Shrinking

Commands and magnitudes simplify; the mode/flavor and the required G92/mode-switch witnesses survive via
`no_shrink`. A real shrink/complicate probe re-validates.

### Execution

Both tiers; bounds `max_commands=12, max_abs_mm=64, max_abs_e_mm=100`. `rng_algorithm = "chacha"`,
`isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero rejection/regeneration
budgets. Features `["slicer-gcode/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-GCODE-SERIALIZE --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-GCODE-SERIALIZE/`. Plain
`cargo test` of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed`. One oracle defect was
found and fixed during implementation (the RepRapFirmware renderer truncated `256 C` to `255 C` via a
`u8` cast); it was an oracle bug, not a SUT bug, and is recorded here rather than as a production regression.

### Acceptance

Maps to AC-6, AC-N2 and AC-N4. Passes over 64 routine cases with zero rejects. Confirmed defect, reproduced
before the fix: absolute-mode Gcode retract/unretract emitted `E-<length>`/`E<length>` deltas and never
updated the accumulator, and absolute Moves/G92 did not maintain it. Fixed in `serialize.rs` (Move sets the
accumulator, G92 resets it in both modes, retract/unretract update and emit the updated absolute E, and the
parallel chunk seeder runs in both modes); named concrete regression
`absolute_mode_gcode_retract_emits_updated_absolute_e_position` in `tests/gcode_relative_extrusion_tdd.rs`
fails with the fix stashed and passes with it. `gcode_serializer_controls` rejects a mode swap, a missed G92
reset, a relative retract/unretract accumulator error, a formatting-boundary error, a reordered header and a
mutated E delta. The two `gcode_relative_extrusion_tdd` baselines pass unchanged.
## H-IO-ID

Registry record: `id = "H-IO-ID"`, `package = "slicer-model-io"`, `target_kind = "lib"`,
`target_name = "slicer_model_io"`, `test_name = "loader::property_tests::proptest_object_id_and_basename_collisions"`,
`production_symbol = "slicer_model_io::loader::path_object_id"`,
`production_path = "crates/slicer-model-io/src/loader.rs"`,
`features = ["slicer-model-io/property_testing"]`, `requires_host_algos = false`,
`requires_scad = false`, `generator_version = 1`, `charter = "docs/testing/properties.md#h-io-id"`.

### Stable property ID

`H-IO-ID` -- semantic, independent of packet numbers.

### Owner

Package `slicer-model-io`; lib target `slicer_model_io`; inline `loader::property_tests` module in
`crates/slicer-model-io/src/loader.rs`; production symbols `slicer_model_io::loader::path_object_id`
(private) and `slicer_model_io::check_basename_collisions` (public). Bound keys: `max_basename_bytes`,
`max_paths`, `max_index`.

### Contract

`path_object_id(path, index) = uuid5(X500, "<file_name>#<index>")` where the 16-byte namespace is RFC 4122's
X.500 namespace (`6ba7b814-...`), **not** `Uuid::NAMESPACE_OID` (`6ba7b812-...`); a path whose
`file_name()` is `None` falls back to the whole `path.to_string_lossy()` text. The historical local name
`NS_OID` is a misnomer that does not change the bytes or the minted ids. `check_basename_collisions`
groups by `file_name()` in input order, allows literal repeats and canonicalized same files, skips paths
with no file name, and returns `ModelLoadError::DuplicateInputBasename { basename, first, second }` naming
the first path in input order and the first later distinct path. It has no production multi-input caller
today; the property does not claim one.

### Domain

Bounded ASCII/Unicode basenames without separators (`max_basename_bytes`), bounded distinct directories,
indices `0..=max_index` plus deterministic `0` and a large representable index; empty path lists, repeats,
same basename in different directories, a collision witness, Unicode basenames, `file_name() == None`
paths (empty/root/`.`/`..`), and a trailing-separator path (`/usr/bin/` -> `bin`, **not** skipped). Rust's
actual `Path::file_name()` component semantics are modelled, not a hand-rolled split.

### Strategy

Construction-first basename/directory/mode strategies, not a filter-heavy arbitrary-path generator;
`prop::sample::select(...).no_shrink()` pins the collision/no-collision mode so shrinking cannot silently
change the verdict. Sizes shrink.

### Oracle

An independent `format!("{basename}#{index}")` name constructor plus `uuid::Uuid::new_v5` with the actual
X.500 bytes; the `uuid` crate's RFC 4122 implementation is a recorded shared dependency, but the
basename/index/fallback and collision logic are not shared with the SUT. Assertions are canonical lowercase
hyphenated form, determinism, cross-directory stability, trailing-separator basename, and **bounded-domain**
distinct-id observations for different indices/basenames. A deterministic control substitutes
`Uuid::NAMESPACE_OID` and requires rejection. No universal injectivity is asserted.

### Tolerance

Exact string equality for ids and exact `PathBuf` equality for the error fields; no normalization of path
text.

### Shrinking

Directory/basename text and list length simplify; the collision/no-collision mode is a `no_shrink`
selector. A shrink/complicate probe obtains a nonminimal value tree, simplifies to exhaustion, asserts the
required mode survives, complicates and re-validates.

### Execution

Both tiers; bounds `max_basename_bytes=32, max_paths=8, max_index=64`. `rng_algorithm = "chacha"`,
`isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero rejection/regeneration
budgets. Features `["slicer-model-io/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-IO-ID --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-IO-ID/`. Plain `cargo test`
of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed`. The inline
`crates/*/src` module is inside the literal scanner's scope, but a clean scan is not correctness evidence;
its watched literals and test-quality patterns were also reviewed by hand.

### Acceptance

Maps to AC-1, AC-N1 and AC-N4. Passes over 64 routine cases with zero rejects. `io_identity_controls`
rejects a full-path hash, an index-ignoring id, the `NAMESPACE_OID` substitution, a missing collision, a
same-file false collision and an id that ignores the trailing-separator basename; it also exercises the
temp-file canonicalization case and asserts the exact `DuplicateInputBasename` fields. The four
`model_loader_tdd` baselines pass unchanged.

## H-IO-EXTENT

Registry record: `id = "H-IO-EXTENT"`, `package = "slicer-model-io"`, `target_kind = "test"`,
`target_name = "property_io"`, `test_name = "proptest_object_world_z_extent"`,
`production_symbol = "slicer_model_io::loader::object_world_z_extent"`,
`production_path = "crates/slicer-model-io/src/loader.rs"`,
`features = ["slicer-model-io/property_testing"]`, `requires_host_algos = false`, `requires_scad = false`,
`generator_version = 1`, `charter = "docs/testing/properties.md#h-io-extent"`.

### Stable property ID

`H-IO-EXTENT` -- semantic, independent of packet numbers.

### Owner

Package `slicer-model-io`; test target `property_io` in
`crates/slicer-model-io/tests/property_io.rs`; production symbol
`slicer_model_io::loader::object_world_z_extent` (public, not crate-root re-exported). Bound keys:
`max_vertices`, `max_abs_mm`.

### Contract

`object_world_z_extent(object)` returns `Some((z_min, z_max))` exactly when the reduced extrema are finite
and `z_max > z_min`; the all-zero matrix is the documented identity convention (`z = v.z`); otherwise
`z = m[2]x + m[6]y + m[10]z + m[14]` is computed in `f64` from the f32 inputs and narrowed once to f32.
NaN vertices are ignored by the `<`/`>` reduction; an infinity that reaches an extremum makes the extent
non-finite and returns `None`. Empty, flat and non-finite results are `None`, never a panic.

### Domain

0..=`max_vertices` vertices on the `2^-4` mm grid inside `max_abs_mm < 2^12`, and an affine matrix whose Z
row is on the `2^-2` mm grid. Modes: identity (all-zero matrix), live affine, flat/degenerate, NaN and
infinity, each pinned by a `no_shrink` selector; identity/affine/flat are the valid printable domain, the
non-finite modes are separately labelled observations and never called a valid object.

### Strategy

Construction-first dyadic generator; magnitudes shrink, vertices are removed, and the mode witness survives.
Flat mode requires a constant Z, NaN mode requires at least one NaN and one finite vertex, infinity mode
requires an infinite vertex.

### Oracle

Integer `2^-6` mm arithmetic: every product and the four-term sum are exact integers on the declared domain,
so the expected Z is derived exactly and narrowed once to f32, matching the production's f64 sum plus one
cast. The reduction is modelled independently (identity, empty, flat, NaN-ignore, infinity-rejects). No SUT
call produces an expected value.

### Tolerance

Exact f32 bit equality for `Some((min,max))`, and exact `None` for the rejected modes. No approximate
extent comparison.

### Shrinking

Vertices and magnitudes simplify; the identity/affine/flat/NaN/infinity mode survives via `no_shrink`. A
real shrink/complicate probe re-validates each mode after every transition.

### Execution

Both tiers; bounds `max_vertices=16, max_abs_mm=64`. `rng_algorithm = "chacha"`,
`isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero rejection/regeneration
budgets. Features `["slicer-model-io/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-IO-EXTENT --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-IO-EXTENT/`. Plain
`cargo test` of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed`.

### Acceptance

Maps to AC-2, AC-N1 and AC-N4. Passes over 64 routine cases with zero rejects. `io_extent_controls` rejects
a missing `m[2]`/`m[6]`/`m[10]`/`m[14]` term, a raw-vertex identity extent, a swapped min/max and a false
`Some` for a degenerate extent. The two `world_z_canonical_surface_tdd` baselines pass unchanged.

## H-IO-PLACEMENT

Registry record: `id = "H-IO-PLACEMENT"`, `package = "slicer-model-io"`, `target_kind = "test"`,
`target_name = "property_io"`, `test_name = "proptest_place_bare_mesh_on_bed"`,
`production_symbol = "slicer_model_io::loader::place_bare_mesh_on_bed"`,
`production_path = "crates/slicer-model-io/src/loader.rs"`,
`features = ["slicer-model-io/property_testing"]`, `requires_host_algos = false`, `requires_scad = false`,
`generator_version = 1`, `charter = "docs/testing/properties.md#h-io-placement"`.

### Stable property ID

`H-IO-PLACEMENT` -- semantic, independent of packet numbers.

### Owner

Package `slicer-model-io`; test target `property_io` in
`crates/slicer-model-io/tests/property_io.rs`; production symbols
`slicer_model_io::loader::place_bare_mesh_on_bed` and `BedPlacement` (both public). Bound keys:
`max_objects`, `max_vertices_per_object`, `max_abs_mm`, `max_translation_mm`.

### Contract

One rigid whole-scene translation: `dx = bed_center.x - (min.x+max.x)/2`, `dy` likewise, `dz = -min.z`,
derived from the union of per-object world AABBs (zero matrix = identity). After placement the world
`z_min` is 0, the whole XY AABB centre is `bed_center`, every pair of objects keeps its AABB min/max
differences, and no object is individually re-centred or re-dropped. `None` is returned for an empty or
unplaceable world (no object with a finite-X AABB). The post-placement `build_volume` is asserted as the
**world-space** AABB union for every scene (Deviation 3: the former raw-vertex recompute was a confirmed
SUT defect, now fixed by `compute_world_bounding_box_union`); identity-mode scenes additionally equal the
shifted raw-vertex union. The recompute is unconditional, including an exactly-zero shift, and the
zero-shift witness starts from a deliberately stale box.

### Domain

0..=`max_objects` objects with 0..=`max_vertices_per_object` dyadic vertices, zero-matrix identity or a
bounded live affine transform (`no_shrink` mode), and a finite dyadic `bed_center` bounded by
`max_translation_mm`. A construction-first box family (`analytical_meshes(SolidKind::Box, ...)`) supplies
nonempty worlds; included modes are empty, single, multi, already placed, below-bed/off-centre, and one
empty object beside a nonempty object.

### Strategy

Construction-first box family with a `no_shrink` identity/live-transform and placeable/unplaceable selector;
object counts and vertex counts shrink.

### Oracle

Independent per-object world AABB (zero matrix = identity), union, and the three translation formulas
recomputed on the f32 values; post-placement assertions use derived envelopes: z-min zero, centre within
`2 * ulp(max(|coordinate|, |bed_center|))`, pairwise AABB differences within `4 * EPSILON * max_abs`
(covers `fl(a+t) - fl(b+t) - (a-b)`). The SUT is never called to derive an expected placement.

### Tolerance

Derived float envelopes above, not exact AABB equality after translation; the pre-placement placement
formula is compared on the f32 values. No claim that `build_volume` equals the world union for live
transforms.

### Shrinking

Objects/vertices simplify and transforms are simplified; the empty/nonempty, identity/live-transform and
placeable/unplaceable modes survive via `no_shrink`. A real shrink/complicate probe re-validates.

### Execution

Both tiers; bounds `max_objects=4, max_vertices_per_object=8, max_abs_mm=64, max_translation_mm=200`.
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero
rejection/regeneration budgets. Features `["slicer-model-io/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-IO-PLACEMENT --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-IO-PLACEMENT/`. Plain
`cargo test` of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed`.

### Acceptance

Maps to AC-3, AC-N1 and AC-N4. Passes over 64 routine cases with zero rejects. `io_placement_controls`
rejects per-object centring, a Z-only drop, a non-rigid per-object scale, a stale pre-placement
`build_volume` and a fabricated empty-world placement; the zero-shift witness requires a deliberately
stale box to be refreshed. The named regressions
`identity_matrix_placement_updates_build_volume_to_the_world_union` and
`zero_shift_placement_still_refreshes_build_volume` pass, as does the CLI consumer-path test
`pnp_cli::scad_placement_refreshes_build_volume_to_world_union`, and the model-io baselines pass.
## H-WIPE-TOWER

Registry record: `id = "H-WIPE-TOWER"`, `package = "wipe-tower"`, `target_kind = "lib"`,
`target_name = "wipe_tower"`, `test_name = "property_tests::proptest_generate_purge_paths"`,
`production_symbol = "wipe_tower::WipeTower::generate_purge_paths"`,
`production_path = "modules/core-modules/wipe-tower/src/lib.rs"`,
`features = ["wipe-tower/property_testing"]`, `requires_host_algos = false`, `requires_scad = false`,
`generator_version = 1`, `charter = "docs/testing/properties.md#h-wipe-tower"`.

### Stable property ID

`H-WIPE-TOWER` -- semantic, independent of packet numbers.

### Owner

Package `wipe-tower`; lib target `wipe_tower`; module `property_tests` in
`modules/core-modules/wipe-tower/src/property_tests.rs`; production symbol
`wipe_tower::WipeTower::generate_purge_paths` (private, metadata only -- no new public export). Registered
from `src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`. Bound keys:
`max_abs_mm`, `max_scan_lines`, `max_line_width_mm`, `max_purge_volume_mm3`.

### Contract

For one tool change, `generate_purge_paths(z, layer_height, layer_index, tc)` returns, in order: one
zero-flow travel pair at `(tower_x, tower_y)`; a sequence of rectilinear scan pairs at
`y = tower_y + line_width/2 + k*line_width` while `y < y_max`, alternating `x_min -> x_max` and
`x_max -> x_min`, each with two points, flow `1.0` and width `line_width`; and one prime pair from
`(tower_x, tower_y)` (flow `0.0`) to `(tower_x + min(purge_volume/(line_width*layer_height), tower_width),
tower_y)` (flow `1.0`). Every pair carries role `WipeTower`, region `__wipe_tower__`, `region_id = 0`, an
empty chain, the argument layer index, `speed_factor = 1.0`, `tool_index = None` and `order_lock = None`.
`cross_section = line_width * layer_height * tower_width <= 0` yields an empty vector; a zero purge volume
with a positive cross-section yields the zero-length travel and prime pair and no scan rows. **Bed
containment is not part of this seam** (packet `## Deviations` item 2): the production `bed_shape` check
lives in `WipeTower::run_finalization`; the property uses a convex bed only as an independent scene witness.
Canonical `WipeTower::tool_change`/`toolchange_Wipe`/`finish_layer` has no direct equivalent; this is a PnP
rectilinear generator, not a canonical-parity claim.

### Domain

`line_width` a power of two in `[1/8, 2]` mm, `tower_width` a 1/16 mm multiple, `tower_x`/`tower_y`/`z` on
the 1/16 mm grid inside `max_abs_mm`, `layer_height` a 1/16 mm multiple (with 0.2 mm, the module default and
off-grid, as a labelled classified control), `purge_volume` in `[0, max_purge_volume_mm3]`, a bounded
`global_layer_index`, and a `ToolChange` fixture. Zero `line_width`/`tower_width`/`purge_volume`/
`layer_height` are separate labelled zero modes. `no_shrink` pins the zero/positive, row-count class and
bed-shape modes; sizes shrink.

### Strategy

Construction-first cases with a row-count ladder (zero, one, many, near-boundary), a `n_upper =
ceil(purge_depth/line_width) + 1` exhaustion guard checked before the model iterates, and strict
`y += line_width` progress. Non-vacuity counters cover positive cross-sections, empty cross-sections, each
zero mode, row counts, exact/classified row counts, both bed shapes and the volume ceiling.

### Oracle

An independent rational/geometric model: the travel/scan/prime sequence and alternating x-order from the
documented formulas; the scan-row count from an f64 rational quotient (exact `ceil` only when the f32
additions are exact, otherwise a classified within-one observation, never claimed exact); tower-rectangle
containment within a derived envelope; positive scan-segment lengths; capped prime length; exact region keys;
and independent half-plane containment against a convex rectangle or right triangle. The generator is never
called to derive an expected value.

### Tolerance

Exact counts/keys/roles/flows on the exact sub-domain; a derived f32 envelope for rectangle containment; a
classified `+-1` count on non-exact/boundary witnesses (never presented as exact); no optimality or
tessellation claim.

### Shrinking

Scan rows/volume/dimensions simplify; the zero/positive mode, the row-count class and the convex-bed mode
survive via `no_shrink`. A real shrink/complicate probe re-validates.

### Execution

Both tiers; bounds `max_abs_mm=64, max_scan_lines=64, max_line_width_mm=2, max_purge_volume_mm3=64`.
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero
rejection/regeneration budgets. Features `["wipe-tower/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-WIPE-TOWER --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-WIPE-TOWER/`. Plain
`cargo test` of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed`.
`modules/core-modules/*/src/**` is outside both scanner scopes, so a green `check-literals` /
`check-test-quality` is NOT evidence for this file; the manual FRU/test-quality review is recorded in the
file header and the evidence doc.

### Acceptance

Maps to AC-11, AC-N3 and AC-N4. Passes over 64 routine and 256 extended cases with zero rejects and **no
confirmed SUT defect**. `wipe_tower_controls` rejects a scan row outside the rectangle, an off-by-one row
count, an over-wide prime segment, a false positive-length claim and a concave-bed containment assertion --
each by the same checker. The concave-bed control is explicitly a test-oracle demonstration, not a
production-rejection claim. The `wipe_tower_tdd` and `bed_bounds_tdd` baselines pass unchanged.
## H-SKIRT-BRIM

Registry record: `id = "H-SKIRT-BRIM"`, `package = "skirt-brim"`, `target_kind = "lib"`,
`target_name = "skirt_brim"`, `test_name = "property_tests::proptest_skirt_brim_enclosure"`,
`production_symbol = "skirt_brim::SkirtBrim::generate"`,
`production_path = "modules/core-modules/skirt-brim/src/lib.rs"`,
`features = ["skirt-brim/property_testing"]`, `requires_host_algos = false`, `requires_scad = false`,
`generator_version = 1`, `charter = "docs/testing/properties.md#h-skirt-brim"`.

### Stable property ID

`H-SKIRT-BRIM` -- semantic, independent of packet numbers.

### Owner

Package `skirt-brim`; lib target `skirt_brim`; module `property_tests` in
`modules/core-modules/skirt-brim/src/property_tests.rs`; production symbol
`skirt_brim::SkirtBrim::generate` (private, metadata only -- no new public export). Registered from
`src/lib.rs` under stacked `#[cfg(test)]` / `#[cfg(feature = "property_testing")]`. Bound keys:
`max_abs_mm`, `max_skirt_distance_mm`, `max_loops`, `max_line_width_mm`.

### Contract (packet `## Deviations` item 1)

The authored `generate_skirt_entities`/`generate_brim_entities` do not exist; the property drives the real
private `SkirtBrim::generate(&[LayerSnapshot])`. `spacing = flow_spacing() = line_width -
first_layer_height*(1 - PI/4)` (canonical `Flow::rounded_rectangle_extrusion_spacing`, shared formula), not
`line_width`. The brim anchors on the filled `OuterWall` contour grown by half the modal wall width, builds
the annulus `[contour+gap, contour+gap+quantised]` with
`quantised = floor(brim_width/spacing/2)*spacing*2`, and emits one outer contour and one hole ring per
inward band from `spacing/2`, outermost band first (`2 * floor(brim_width/spacing/2)` rings above the
thin-tail bound). The skirt is a convex-hull offset of the occupied outline (layer-0 points plus brim-area
contours), `skirt_loops` offsets at `skirt_distance + spacing/2 + i*spacing`, emitted outermost first, once
per layer up to `min(skirt_height, layers)`. Roles/regions are `Skirt`/`__skirt__` and `Brim`/`brim`, with
`region_id = 0`, empty chain, matching layer index, `speed_factor = 1.0`, `tool_index = None`,
`order_lock = None`, every point width `spacing` (skirt) or `line_width` (brim), flow `1.0`, and a closed
path. No rectangle identity, four-corner or vertex-count claim is made.

### Domain

One or two layers carrying a closed rectangle `OuterWall` loop on the 1/16 mm grid (625 integer units,
converted with `Point2::from_mm`), `line_width` a power of two in `[1/8, 2]` mm, `first_layer_height = 0.2`,
`skirt_distance` a 1/16 mm multiple in `[0, max_skirt_distance_mm]`, `skirt_loops` in `[0, max_loops]`,
`skirt_height = 1`, `brim_width` a 1/16 mm multiple in `[0, max_abs_mm]`, `brim_object_gap = 0`,
`min_skirt_length = 0`, enabled, rectangle coordinates inside `max_abs_mm`, with a degenerate
`x_min == x_max` mode for closure/count only.

### Strategy

Construction-first cases with `no_shrink` on the rectangle class, skirt/brim presence and layer count;
dimensions shrink. Mandatory witnesses every run: positive skirt loops, zero skirt loops, positive and zero
brim, a degenerate rectangle, a two-layer scene and the wide-brim thin-tail shape.

### Oracle

An independent model computes `spacing`, `quantised`, the expected band count and the skirt offsets from the
documented formulas; the rectangle footprint is contained with a test-local f64 point-in-polygon and edge
sampling (clipper output is not on the 1/16 grid, so the integer `point_in_region` conversion is inexact --
recorded in the file header). Whole-bbox edge enclosure replaces four-corner membership; the brim rings are
checked for closure, key/width/flow invariants, nesting order and clearance windows. The generator is never
called to produce an expected value.

**Thin-tail classification (derived, measured).** The exact `2 * floor(brim_width/spacing/2)` count holds
while each band is wider than the offset's arc decimation: for a 90-degree corner the outer boundary's
single chord sits up to `wall_width/2 * (1 - cos 45 deg) = 0.058579 mm` inside a parallel offset, plus
`BRIM_ARC_TOLERANCE_MM` per boundary, so the two boundaries can meet when
`spacing <= 0.083579 mm` (only `line_width = 1/8` on the domain). In that tail the first
`2 * (bands - 1)` rings keep the exact structure and the last band may split into several non-degenerate
rings; every fragment is validated (closed, >= 3 distinct vertices, non-zero area, inside the last band's
clearance window), the fragment count is recorded as `tail_fragments`, and no bound is invented. The
measured failing input (`line_width 0.125`, `brim_width 56.6875`, 1/16 x 3/16 mm rectangle at (-64,-64))
yields 693 rings: 690 clean rings plus 2 contour lobes and 3 sliver holes of 0.0004-0.0016 mm2 area, all at
the band-344 hole boundary. The sliver emission is a **recorded candidate observation**. Canonical's
**append step** is verified against `Brim.cpp::tryExPolygonOffset` (upstream `main`, read 2026-11-26):
after a first `-0.5*spacing` offset it appends every contour **and hole** of each Clipper2 result with no
area/vertex filter, and its brim width uses the same `floor(width/flow/2)*flow*2` quantisation. That is
not the full emission pipeline: canonical Douglas-Peucker-simplifies each result at
`0.0125 / SCALING_FACTOR` immediately before appending, and `Brim.cpp::makeBrimInfillImpl` unions the
loops with `union_pt_chained_outside_in`; PnP does neither, and a 3-vertex sliver under 0.0125 mm high is
exactly what Douglas-Peucker collapses. Whether canonical emits these slivers is therefore **unmeasured
and not claimed**. The exact tail ring multiset is likewise not claimed: canonical applies
`-1.3*spacing` / `+0.3*spacing` open-close with `douglas_peucker` between passes while
`SkirtBrim::generate` applies one `-depth` offset per band.

**Corner-join windows (derived, measured).** The brim's outer boundary offset decimates the reconstruction
contour's 90-degree corner arc against `OFFSET_SHORTEST_EDGE_FACTOR * quantised`, which can place a ring
vertex `quantised - depth` from the footprint **corner** instead of `anchor + quantised - depth`. The
nearest-vertex windows are therefore two-sided and family-specific: outer ring
`[quantised - depth - contour_env, anchor + quantised - depth + contour_env]`, hole ring
`anchor + depth +/- contour_env`, skirt ring `[annex + d - anchor - SKIRT_ENV, annex + d + SKIRT_ENV]`, with
`contour_env = 0.086579 mm` (two round-join passes + `wall_width/2 * (1 - cos 45 deg) = 0.058579` corner
sagitta + rounding + slack). The named `skirt_brim_corner_join_witness` pins the exact vertex
`(-11.658100, -95.147697)` at `quantised - depth = 61.648008`, the exact extent (2e-4), and rejects a
uniform half-anchor error (measured deviation 0.201207 against the 0.086579 window).

### Tolerance

Exact ring counts outside the thin tail; `max_clearance_dev` and `max_bbox_excess` report the excess
**outside the derived windows** (0.000000 in the final runs, i.e. every residual is explained), while
`max_bbox_dev` is the raw measurement (corner-chord sagitta, ~0.05 mm and seed-dependent). The tail asserts
structure and validity, not a fragment count. No area/simplicity claim for degenerate rectangles.

### Shrinking

Loops/vertices and dimensions simplify; the positive/zero-loop, safe/boundary and skirt/brim modes survive
via `no_shrink`. A real shrink/complicate probe re-validates the required witness after each transition.

### Execution

Both tiers; bounds `max_abs_mm=64, max_skirt_distance_mm=8, max_loops=3, max_line_width_mm=2`.
`rng_algorithm = "chacha"`, `isolation = "in_process"`, `timeout_ms = 0`, `max_shrink_time_ms = 0`, zero
rejection/regeneration budgets. Features `["skirt-brim/property_testing"]`. Selected by
`cargo xtask test --summary --properties --property H-SKIRT-BRIM --tier <routine|extended>`; context via
`PNP_PROPERTY_CONTEXT`; artifacts under `target/property-testing/<run_token>/H-SKIRT-BRIM/`. Plain
`cargo test` of the generated property is NOT acceptance. Included in the Windows routine job.

### Failure record

`failure.json` retains classification, full context, concrete/debug input, the tier-bearing reproduction
command, an executable direct reproduction, captured SDK logs and `shrink_completed`.
`modules/core-modules/*/src/**` is outside both scanner scopes; the manual FRU/test-quality review is
recorded in the file header and the evidence doc.

### Acceptance

Maps to AC-10, AC-N3 and AC-N4. Routine 64 and extended 256 cases pass with zero rejects; the extended lane
exercised 25 thin-tail cases and recorded 4 tail fragments, and the 1.19/0.17 boundary control asserts the
measured 4 bands / 8 rings, offsets 1.1531/0.2635 .. 0.7718/0.6448, innermost hole 0.6448 and **no**
zero-offset ring (the authored note is classified as a mis-prediction). `skirt_brim_controls` calls the same
boundary helper, rejects an off-by-one count, inward skirt order, an open loop, lost whole-bbox enclosure, a
dropped brim loop and a fabricated rectangle-shape equality, and `skirt_brim_corner_join_witness` pins the
corner-join geometry and the half-anchor rejection. Final runs: routine `decimation_classified=0`; `thin_tail`/`tail_fragments` are single-run samples, not
bounds (across the observed runs, routine `thin_tail` 1-4 / `tail_fragments` 0, extended 20-27 / 0-7),
`max_clearance_dev=0`, `max_bbox_excess=0`, `enclosed_loops` 57 routine / 238 extended. The two
`skirt_brim_tdd` baselines pass unchanged. No production code changed for this family.
