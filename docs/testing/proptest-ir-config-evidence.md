# Property-testing evidence: IR, coordinates, IDs, config and chains

Packet: `proptest-openrscad_06_pbt-ir-config`. This document records the executed evidence for
the six registry families added by that packet (`H-COORD`, `H-IDS`, `H-IR-SERDE`,
`H-CONFIG-INTERN`, `H-CONFIG-BOUNDS`, `H-CHAINS`) plus the preserved `H-IR-ANCHORS` family, and
the G-FAMILY disposition those families support.

Everything below was executed on Windows (the development machine for this repository). Seeds
and seed files are diagnostic artifacts only: committed regression history consists of named
concrete inputs, never seeds.

## G-FAMILY disposition

| Charter | Registry ID | Disposition |
|---|---|---|
| `H-COORD` | `H-COORD` | **Complete.** Derived per-direction error envelopes, exact half-away tie rule, monotonicity, SDK/`Point2` consistency, no-float integer identity, and mutation controls for scaling/truncation/false-inverse. |
| `H-IDS` | `H-IDS` | **Complete for the pure helper.** Allocation sequence, derived representable-parent boundary, namespace/payload/decode/sentinel rules, documented minting assertion observed once deterministically. Runtime and region-map collision obligations are handed to `proptest-openrscad_08_pbt-runtime` and `proptest-openrscad_11_pbt-prepass`. |
| `H-IR` (anchors) | `H-IR-ANCHORS` | **Extended, identity preserved.** Added the side-table-only missing-anchor witness with a controlled side-table-union mutation that the oracle and validator both reject. |
| `H-IR` (serde) | `H-IR-SERDE` | **Complete for the selected slice.** Bounded finite Postcard + JSON round trips of entity/travel/side-table/point-bit observations, exact integer `Point2` identity, named legacy-JSON defaults, required-field/null rejections, representation-only Postcard exceptional bits. Not a whole-IR validator. |
| `H-CONFIG` (interner) | `H-CONFIG-INTERN` | **Complete for the varied axes.** Representation-key equality oracle, hash implication, first-occurrence interner model, idempotence, `config_for_raw` contents, keyed nondefault read. Field families not varied by the generator are listed explicitly in the charter's coverage table. |
| `H-CONFIG` (bounds) | `H-CONFIG-BOUNDS` | **Complete.** Real `ConfigBoundsIndex::check` against an independent all-declarations predicate, inclusive edges, empty intersections, one-sided bounds, first-invalid list index, nonfinite handling, percent-versus-absolute distinction, and four refuted mutants. |
| `H-CHAINS` | `H-CHAINS` | **Complete.** Complete ordered mixed-radix model, cardinality/product guard before allocation, empty/absent axes, caller value order, duplicate multiplicities, ignored extra keys, five refuted mutants. |

No family is claimed complete beyond the scope written in its charter, and no family is closed
on registry-only evidence: every row above was executed through the registry-backed command.

## Source, toolchain and dirty-content provenance

- Source revision: the executor records it per run in
  `target/property-testing/<run-token>/<ID>/context.json` (`source_revision`,
  `dirty_paths`, `source_state_digest`). Re-derive the current commit with
  `git log --format=%H -1 --grep proptest-openrscad_06` rather than trusting a frozen copy,
  because an amend or a squash changes it.
- **Provenance is per run, not per document, and it is content-addressed.** Every run's
  `context.json` records `source_revision`, `dirty_paths` and a `source_state_digest` over the
  measured content. The tables below quote runs from more than one revision, because the
  strategies were changed between measurement rounds and re-measuring everything after every
  edit would only produce another mixed batch:
  - the combined routine table row comes from the run at `ac9dc0ef` (`dirty_paths: []`);
  - the per-record extended walls and the combined extended / seed-37 rows come from runs at
    `ba807a18` (also `dirty_paths: []`);
  - earlier measurement rounds at `727ba9e9` and `89c1e069` carried edited source or
    documentation in their `dirty_paths`; those rows were superseded by the clean runs above
    and are not quoted in the tables.
  Each `source_state_digest` pinpoints the measured content, and this document does not describe
  a run as clean unless its own context says so.
  `target/evidence/final-provenance.txt` is a summary generated at one moment and is **not**
  authoritative for every table row; the per-run `context.json` files are. This document does not
  restate a hash that an amend would invalidate.
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`, Windows `x86_64-pc-windows-msvc`.
- Proptest: `1.11.0` (`workspace.dependencies` in the root `Cargo.toml`, `chacha` RNG).
- Features: `slicer-ir/property_testing` for the six `slicer-ir` records (`H-COORD`, `H-IDS`,
  `H-IR-SERDE`, `H-CONFIG-INTERN`, `H-CHAINS` and the preserved `H-IR-ANCHORS`),
  `slicer-scheduler/property_testing` for `H-CONFIG-BOUNDS`. Both are default-off selection
  features; neither implies an engine, SCAD or host-algorithm feature.
- `CARGO_TARGET_DIR`: unset — the executor records `cargo_target_dir: null` and the default
  workspace `target/` is used.
- `requires_host_algos = false` and `requires_scad = false` for all seven records. The
  `slicer-sdk` dev-dependency still enables its required non-wasm host algorithms in the dev
  graph; the metadata/feature-isolation checks in `cargo xtask` verified the actual union rather
  than relying on a no-explicit-feature fallback.

## Baseline: existing owner tests, captured before and after the additions

Every command below was captured to `target/test-output.log` and copied to
`target/evidence/baseline-*.log`. All results are from Windows.

| Command | Result |
|---|---|
| `cargo test -p slicer-ir --test ir_tests` | PASS |
| `cargo test -p slicer-ir --test entity_id_invariants_tdd` | PASS |
| `cargo test -p slicer-ir --test ir_validation_tdd` | PASS |
| `cargo test -p slicer-ir --test region_split_registry_tdd` | PASS |
| `cargo test -p slicer-ir --test resolved_config_defaults_tdd` | PASS |
| `cargo test -p slicer-ir --test region_map_versioned_decode_tdd` | **FAIL (pre-existing)** |
| `cargo test -p slicer-ir --lib -- resolved_config` | PASS (14 tests) |
| `cargo test -p slicer-scheduler --test scheduler_integration -- config_bounds_enforcement_tdd` | PASS |
| `cargo test -p slicer-scheduler --test scheduler_integration -- config_resolution_tdd` | PASS |
| `cargo test -p slicer-sdk --features property_testing --test property_domain_contracts` | PASS |
| `cargo test -p xtask --bin xtask -- property_` | PASS |
| `cargo test -p slicer-ir --features property_testing --test property_ir_contracts` | PASS (8 tests) |
| `cargo test -p slicer-scheduler --features property_testing --test property_config_contracts` | PASS (3 tests) |

### Pre-existing failure found in the baseline, then fixed at its root cause

The first baseline capture found
`region_map_versioned_decode_tdd::frozen_region_map_3_0_0_fixture_decodes_unchanged_with_absent_extension_default`
failing with `DeserializeBadBool` while decoding the frozen 3.0.0 Postcard fixture
(`crates/slicer-ir/tests/region_map_versioned_decode_tdd.rs`, frozen bytes under
`crates/slicer-ir/tests/fixtures/`). It was **not** caused by this packet (no file under
`crates/*/src/**` changed, and the test plus fixture were byte-identical), but it was a required
baseline, so it was fixed rather than waived.

Root cause: commit `6db49f0f` removed `ResolvedConfig::flat_bridge_closing_join` from the middle
of the declared struct. Postcard is positional, so every later field shifted and the decoder read
a `String` length byte as the `support_enabled` bool. The field is restored at its exact original
serialized position with a new serialization-only `carrier` row kind in
`declare_resolved_config!`: it is part of the struct, the default, `Eq`/`Hash` and the wire
layout, but **not** part of `to_config_map`, `typed_field_keys` or CLI dispatch — so the retired
output key does not come back. The independent fixture was **not** regenerated and the assertion
was **not** weakened; `flat_bridge_closing_join_is_a_serialization_only_carrier`
(`crates/slicer-ir/tests/resolved_config_defaults_tdd.rs`) pins the boundary in both directions.

## Calibration: substantive populations, not empty-only coverage

Each family runs mandatory deterministic witnesses on every execution, independent of sampling:

- `H-COORD`: exact `1/32 mm` half case and mirror, three neighbour floats per side, both zero
  signs, two subnormals, both interval ends, unit envelope at `{-U, -1, 0, 1, U}`, monotonicity
  at the ends. Boundary and sign modes are therefore always exercised.
- `H-IDS`: `0`/`1`/`1..=64` allocation sequences, the derived `P` boundary from both sides,
  `{0, 1, 2, S-1, S, S+1, P-1, P}`, a holed and an empty footprint, sentinel and namespace
  membership, and a never-minted bit-marked id.
- `H-IR-SERDE`: a hand-built **non-empty, two-entity, travelling, side-tabled** layer is
  round-tripped through both codecs on every run, so an all-empty sampled population can never
  stand in for coverage of this charter.
- `H-CONFIG-INTERN`: `-0.0` vs `0.0`, two distinct NaN payloads, percent vs absolute
  `FloatOrPercent`, an independently reconstructed equal pair, a nested extension tree of depth
  1 vs 2 (equality, representation and hash), default-seed interning, rank-1 insertion and a
  keyed nondefault read.
- `H-CONFIG-BOUNDS`: absent key, present-but-unbounded declaration, one-sided bounds, inclusive
  edges with `next_down`/`next_up` neighbours, a contradictory intersection, empty list, lists
  failing at index 0 and index 2, `Bool`/`String` pass-through, percent max-not-applied versus
  min-applied, absolute `FloatOrPercent` fully bounded, and non-finite absolutes. A bounded
  pre-sample additionally asserts that the generated population contains accepted cases,
  rejected cases, at least one list-index case and at least one non-finite case; a
  success-only population fails the run.
- `H-CHAINS`: `[[]]` for no axes, a three-unsorted-value axis, absent and empty axes with an
  unrelated key, duplicate multiplicities, and a 64-axis product refusal before allocation.

Rejected-test-call counts were `0` for every family in every tier: the domains are total (no
filters, no `prop_assume` gates, zero rejection budgets). That is honest for these properties
because every generated value is valid for the API being called; invalid-input classes that are
*not* in a helper's contract (for example non-finite coordinate inputs) are deliberately not
generated.

## Measured profiles

Committed values in `test-properties.toml` (identical shape for all six new records plus the
preserved anchor record): `cases = 64` routine / `1024` extended,
`max_local_rejects = max_global_rejects = max_flat_map_regens = 0`,
`max_default_size_range = 32 / 256`, `max_shrink_iters = 64 / 256`,
`max_shrink_time_ms = 0` (no elapsed-time cutoff), `rng_algorithm = "chacha"`,
`isolation = "in_process"`, `timeout_ms = 0`.

Per-record bounds:

| ID | routine bounds | extended bounds |
|---|---|---|
| `H-COORD` | `max_abs_mm = 25`, `max_abs_units = 250_000` | `max_abs_mm = 100`, `max_abs_units = 1_000_000` |
| `H-IDS` | `max_allocations = 256`, `max_id_bytes = 64`, `max_footprint_points = 8` | `max_allocations = 4096`, `max_id_bytes = 256`, `max_footprint_points = 32` |
| `H-IR-SERDE` | `max_entities = 8`, `max_travels = 16`, `max_points = 4`, `max_layers = 4` | `16 / 32 / 8 / 8` |
| `H-CONFIG-INTERN` | `max_configs = 4`, `max_list_len = 4`, `max_list_depth = 2`, `max_text_bytes = 16`, `max_abs_config_value = 4096` | `8 / 8 / 3 / 32 / 16384` |
| `H-CONFIG-BOUNDS` | `max_declarations = 4`, `max_list_len = 4`, `max_abs_bound = 1000` | `8 / 8 / 100000` |
| `H-CHAINS` | `max_semantics = 3`, `max_values_per_semantic = 3`, `max_chains = 64`, `max_text_bytes = 16` | `4 / 4 / 625 / 32` |

The coordinate pair is not arbitrary: `CoordProfile::new` requires `0 < 10_000·max_abs_mm ≤ 2^24`
and `0 < max_abs_units ≤ 2^24`, and additionally
`min(max_abs_units, 10_000·max_abs_mm) ≤ 1_000_000` so the shared `rounding_boundary_mm` helper
is fed a legal argument. The chain maxima are exactly `(1+3)^3 = 64` and `(1+4)^4 = 625`, so a
maximal generated case is still compared in full rather than truncated.

## Execution evidence

Command shape (identical for all rows; one invocation per record for the per-record walls, then
the combined selections):

```text
cargo xtask test --summary --properties --property <ID> --tier <routine|extended>
cargo xtask test --summary --properties --property H-COORD --property H-IDS \
  --property H-IR-ANCHORS --property H-IR-SERDE --property H-CONFIG-INTERN \
  --property H-CONFIG-BOUNDS --property H-CHAINS --tier <routine|extended>
cargo xtask test --summary --properties <same seven> --tier routine --seed 37
```

Discovery is per-record (`cargo test ... --test <target> -- --list` and `--list --ignored`), and
the executor fails the run on a registered test that is not discovered
(`missing_property_execution`). No run reported a missing, zero-test or filtered-out
registration for these records.

### Per-record extended-tier execution (Windows, warm build)

| ID | exact discovered test | libtest wall | accepted cases | rejected calls | shrink status | log |
|---|---|---:|---:|---:|---|---|
| `H-IR-ANCHORS` | `proptest_ir_travel_anchors_membership` | see table below | 1024 | 0 | `null` (unknown) | `target/evidence/final-extended-H-IR-ANCHORS.log` |
| `H-COORD` | `proptest_coord_conversion_envelopes` | see table below | 1024 | 0 | `null` (unknown) | `target/evidence/final-extended-H-COORD.log` |
| `H-IDS` | `proptest_entity_and_modifier_ids` | see table below | 1024 | 0 | `null` (unknown) | `target/evidence/final-extended-H-IDS.log` |
| `H-CONFIG-BOUNDS` | `proptest_config_scalar_bounds` | see table below | 1024 | 0 | `null` (unknown) | `target/evidence/final-extended-H-CONFIG-BOUNDS.log` |
| `H-IR-SERDE` | `proptest_ir_serialization_and_defaults` | see table below | 1024 | 0 | `null` (unknown) | `target/evidence/final-extended-H-IR-SERDE.log` |
| `H-CONFIG-INTERN` | `proptest_config_equality_hash_and_interning` | see table below | 1024 | 0 | `null` (unknown) | `target/evidence/final-extended-H-CONFIG-INTERN.log` |
| `H-CHAINS` | `proptest_canonical_chain_product` | see table below | 1024 | 0 | `null` (unknown) | `target/evidence/final-extended-H-CHAINS.log` |

| ID | exact discovered test | libtest wall | verdict |
|---|---|---:|---|
| `H-IR-ANCHORS` | `proptest_ir_travel_anchors_membership` | 9.90s | PASS |
| `H-COORD` | `proptest_coord_conversion_envelopes` | 0.27s | PASS |
| `H-IDS` | `proptest_entity_and_modifier_ids` | 0.32s | PASS |
| `H-CONFIG-BOUNDS` | `proptest_config_scalar_bounds` | 1.42s | PASS |
| `H-IR-SERDE` | `proptest_ir_serialization_and_defaults` | 6.79s | PASS |
| `H-CONFIG-INTERN` | `proptest_config_equality_hash_and_interning` | 1.32s | PASS |
| `H-CHAINS` | `proptest_canonical_chain_product` | 0.36s | PASS |

Every run printed a matching `begin`/`end` controller pair with
`"outcome":"passed"`, `"completion_eligible":true`, `"call_counts_complete":true` and
`"rejected_test_calls":0`; the routine tier reported 64 accepted cases per record and the
extended tier 1024. Shrink completion is reported `null` throughout: proptest 1.11 cannot
report whether minimization finished, so no retained case is claimed minimal when a shrink
limit could have hit.

The combined selections — the packet's `P` command — reported the same per-record counts in
every tier:

| Selection | Tier | Wall | Records at their profile case count | Verdict |
|---|---|---:|---|---|---|
| all seven `--property` flags | routine | 19 s | 7 / 7 at 64 | PASS |
| all seven `--property` flags | extended | 36 s | 7 / 7 at 1024 | PASS |
| all seven `--property` flags `--seed 37` | routine | 19 s | 7 / 7 at 64 | PASS |

Logs: `target/evidence/final-all-routine.log`, `final-all-extended.log`,
`final-replay-37.log`; the corresponding inner executor logs are the `-inner.log` siblings.

Artifacts are retained under `target/property-testing/<run-token>/<ID>/` (context, events,
case logs and any `failure.json`). No `failure.json` was produced: no property failed, so there
was no SUT, generator, oracle or contract failure to classify, and no concrete regression was
manufactured.

### Windows CI lane

### Hosted Windows execution (required evidence, obtained)

| Field | Value |
|---|---|
| Workflow run | https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43523810446 |
| Commit | `e157169` (includes the `carrier` fix) |
| Job | `Properties (Windows)` — **success** |
| Step | `Run routine properties` — **success** |
| Property test binaries | 12 `test result: ok` lines, 0 `FAILED` |
| Runner verdict | `VERDICT: PASS` |

Two earlier runs on the same branch also passed this lane (`ba807a1` and `89c1e069`); the table
cites the head that carries the final production change, so the evidence matches the code that
would merge.

The `properties-windows` job selects records dynamically from `test-properties.toml` with no
hand-maintained list, so its `cargo xtask test --summary --properties --tier routine` invocation
runs every non-SCAD record — the seven families above plus the five `D-*` domain records. Twelve
passing binaries is exactly that population, and the run reported `PASS`. This satisfies the
packet's requirement that an actual hosted Windows execution include these records; packet
`proptest-openrscad_17_pbt-ci-closure` still owns the rollout/manual-dispatch audit, which is a
separate obligation.

The local runs recorded earlier in this document remain as the per-record timing and
accepted-case evidence; they are not a substitute for the hosted run above, which is why both are
recorded.

## Compile versus execution cost (measured separately)

- Warm incremental recompile of the two `slicer-ir` property targets after touching their
  sources: `cargo test -p slicer-ir --features property_testing --test property_ir --test
  property_ir_contracts --no-run` — 2 s.
- Warm incremental recompile of the two `slicer-scheduler` property targets: 2 s.
- Cold dependency compilation was not separately isolated from the workspace build described
  below; no figure is claimed for it.
- Execution wall per record is in the timings table; the per-record `cargo xtask` wall includes
  the `check-literals` preflight, the `pnp_cli` staleness rebuild check and Cargo preparation,
  which dominate the ~12 s floor of a single-record invocation.

## Retained-seed replay

`--seed 37` replay of all seven records at the routine tier passed with the same accepted-case
counts. Log: `target/evidence/final-replay-37.log`; the resolved context artifacts record
`"seed":37`. Seeds are diagnostic artifacts: nothing in the repository encodes a seed as
regression history.

## Defect ledger

| Family | Classification | Detail |
|---|---|---|
| `H-COORD` | no production defect | Envelopes, tie rule, monotonicity and wrapper consistency all held; the scaling, truncation and false-inverse mutants were refuted by the controls. |
| `H-IDS` | no production defect | Allocation sequence, derived `P` boundary, namespace/payload/decode and sentinel exclusions held; the minting assertion text was observed for `P+1` and `u64::MAX`. |
| `H-IR-ANCHORS` | no production defect | The side-table-union mutation was refuted by the oracle and the validator; the existing validator still reads `ordered_entities` alone. |
| `H-IR-SERDE` | no production defect | All five named JSON defaults, required-field/type/null rejections and finite Postcard/JSON bit preservation held. |
| `H-CONFIG-INTERN` | no production defect | Bitwise equality agreed with the canonical representation on every pair, hashes agreed for equal values, and interning matched the first-occurrence model. |
| `H-CONFIG-BOUNDS` | no production defect | The public API agreed with the all-declarations predicate, including the percent-min-only rule and first-invalid-index reporting. |
| `H-CHAINS` | no production defect | The helper matched the mixed-radix model exactly, including empty-chain-first, caller value order, duplicate multiplicity and ignored extra keys. |

The one red baseline (`region_map_versioned_decode_tdd`, above) was a pre-existing wire-layout
defect; it is fixed at its root cause with the fixture and assertion unchanged, so no family
result absorbs it and no fixture was regenerated.

## Post-review remediation

An independent full packet review returned CHANGES REQUESTED. Every finding it raised was
addressed inside this packet's scope; the three that needed structural work are summarised at the
end of this section. No assertion was weakened and no tolerance was tuned to obtain a pass.

| Finding | Resolution | Falsifiable evidence |
|---|---|---|
| **F1** Shrink claims were false: `prop_oneof!` unions shrink into earlier alternatives, so a "structural variant tag" did not pin the mode | Added `mode_locked` (a `no_shrink` mode selector fed through `prop_flat_map`) to both shared oracle modules and used it for the IR owner strategies (`coord_cases`, `id_cases`, `config_seed_cases`) and for the scheduler value domains (`bound_values`, `nonfinite_values`), plus `no_shrink` on the probe selection. The `H-CONFIG-BOUNDS` strategy was then **redesigned** so its value is *derived* from the current declaration set inside `prop_map` (no declaration-level `prop_flat_map`), which preserves value shape, verdict class and first-invalid index under shrinking. Each property calls `assert_mode_preserved` before sampling. | `mode_lock_preserves_tags_under_simplification` in both contracts targets calls `simplify()` (proptest's shrink direction) and asserts at least one successful simplification and at least two observed modes; `complicate()` was additionally probed by review, not by a committed test. `assert_mode_preserved` runs inside `proptest_coord_conversion_envelopes`, `proptest_entity_and_modifier_ids`, `proptest_config_equality_hash_and_interning` (over the **config-seed** strategy, not the `Vec<ResolvedConfig>` collection) and `proptest_config_scalar_bounds` (over the composed declarations-plus-probe strategy). `derived_probe_modes_have_their_documented_verdicts` pins the derived modes deterministically. |
| **F2** The nested-list budget counted leaves (`b^depth`) instead of total elements (`b + b² + … + b^depth`) | `property_oracles::extension_node_count` counts the true total; `clamped_extension_shape` clamps **both** breadth and depth against it (depth is clamped to the budget because `extension_node_count(d, 1) == d`); `config_from_seed` budgets with it, and the `Scalar`, `Holder` and first `Repeat` arms no longer inherit the default eight-element `bed_shape`. The generator additionally asserts `config_list_elements(config) <= max_list_len` on every produced configuration, so all five seed arms are covered at runtime rather than by a sampled claim. | Concrete regressions in `ir_config_oracle_counterexamples`: `extension_node_count(2, 2) == 6`; depth-2/breadth-2 against budget 4 clamps and the fixture stays at `config_list_elements <= 4`; depth 5 against budget 4 clamps the depth to 4. |
| **F3** The `units_to_mm` reference-error proof omitted the f64 division's own rounding, claiming it was below `2^-150` | `units_bound` is `(2^-24 + 2^-53)·|x| + 2^-150`: f32 half-ulp for the conversion **plus** the f64 reference division's `2^-53` relative error, with `2^-150` kept only as the subnormal floor. Both allowed-error values are outward-rounded (`next_up`) so the comparison can never be tighter than the derivation. The `H-COORD` charter's Oracle and Tolerance headings were corrected to match (the earlier text still quoted the old budget). | The corrected bound is used by `check_units_case` and quoted in `docs/testing/properties.md` §H-COORD; no sample-tuned constant exists. |
| **F4** Two bounds-oracle mutants were refuted only against production, not against the reference predicate, so substituting the mutant for `expected_accepts` stayed green | `config_bounds_oracle_counterexamples` now asserts the reference predicate's own verdict on the union witnesses (3 and 50 rejected, 7 accepted) and on the inclusive edges (1 and 3 accepted), in addition to the mutant disagreement. | `config_bounds_oracle_counterexamples`; the four mutants now fail the suite if substituted for the reference predicate. |
| **F8** `Point2::to_mm` was named on the owner surface but never called | `check_mm_case` now builds `Point2::from_mm(m, -m)` and compares `to_mm()` **bitwise** against `units_to_mm` for both coordinates, covering the negative and zero-sign directions. | `ir_coord_oracle_and_bound_controls` and every sampled `Millimetres` case (plus the deterministic witness loop) run through `check_mm_case`; the `Units` and `OrderedMillimetres` variants intentionally use the unit and monotonicity checkers instead. |
| **F9** A constant-true `defective_exact_roundtrip_claim` was asserted decoratively | Removed the helper and its assertion; the false universal-inverse claim is falsified only by the concrete `1/32 mm → 313 → 0.0313 mm` and subnormal pairs, which remain. | `ir_coord_oracle_and_bound_controls`. |
| **F7** Provenance prose contradicted the retained contexts | The provenance section is now explicitly per run and names the revision behind every quoted table row, and it states the recorded dirty state of each run rather than asserting a clean tree. | The per-run `context.json` files under `target/property-testing/<run>/<ID>/`; `target/evidence/final-provenance.txt` is only a snapshot and is no longer cited as authoritative. |

### Delta review

A second, delta-scoped review re-ran both contracts targets (9 and 4 tests), both F4 mutant
substitutions, an old-budget substitution, composed-strategy probes with `simplify()` **and**
`complicate()`, and the frozen-fixture baseline. It confirmed F4, F8 and the per-run provenance
as fixed, and found three remaining composition/budget problems and two documentation
contradictions. All of them are now addressed: the F1/F2/F3 rows above record the fixes, the
superseded "blocks nothing"/"obligation was met" prose is gone, and the composed scheduler
strategy was redesigned so it preserves the value shape — plus the verdict class and the
first-invalid index for witness-derived probes — under shrinking. A third, focused review then
verified the two code changes and the hosted CI run.

### Findings that were blocking, and how each was closed

- **F5 — closed at the root cause.** See "Pre-existing failure found in the baseline" above: the
  3.0.0 wire layout is restored with a serialization-only `carrier` row kind, `RegionMapIR`
  3.0.0 decodes again, and the retired output key stays retired. A focused review then probed
  the field and found the first attempt (`plain`) leaked it into `to_config_map` and
  `CONFIG_BLOCK`; the `carrier` kind removes it from the effective map, the typed-key roster and
  CLI dispatch, and the regression asserts absence from both maps.
- **F6 — closed with a hosted run.** `Properties (Windows)` on
  https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43523810446 at `e157169`
  succeeded and reported `VERDICT: PASS` over 12 property binaries, which is exactly the seven
  packet records plus the five `D-*` records.
- **F1 residual — closed for the scheduler family.** The `H-CONFIG-BOUNDS` strategy was
  redesigned so its probe value is derived from the declarations, and the property now audits
  the composed strategy on every execution. The IR properties audit their composed strategies
  the same way; the remaining scope of those audits is stated in the charters (shape for the
  general families, shape plus verdict and index for the derived witness families).

## Regressions

No regression was manufactured for a **newly property-discovered** production defect: no property
in these seven families falsified the contract it targets. One production defect *was* found —
by the baseline, not by a property — and is repaired with regressions: the `ResolvedConfig` 3.0.0
wire-layout break described above, covered by
`flat_bridge_closing_join_is_a_serialization_only_carrier`
(`crates/slicer-ir/tests/resolved_config_defaults_tdd.rs`) and
`config_block_omits_the_serialization_only_carrier`
(`crates/slicer-config/tests/resolved_config_view_tdd.rs`). The deterministic controls that
*would* catch a future property regression are named in each charter under "Acceptance" and live
in `crates/slicer-ir/tests/property_ir_contracts.rs` and
`crates/slicer-scheduler/tests/property_config_contracts.rs`.

One **test-infrastructure** change was required and is not a production regression:
`property_registry_validates_and_discovers_exact_tests`
(`xtask/src/properties/executor.rs`, packet 01's suite) hardcoded a one-test discovery roster for
the `slicer-ir`/`property_ir` target. That target now carries six registered tests, so the
assertion derives the expected roster from the live registry and compares it to the sorted live
discovery, plus an explicit assertion that the preserved anchor test is still discovered. The
check is strictly stronger than the fixed roster and still fails on a
registered-but-undiscovered test.

## Gaps and limits, stated explicitly

- `H-IR-SERDE` certifies a deliberately selected IR slice; it is not a whole-IR validator and
  makes no printable-validity claim.
- `H-CONFIG-INTERN`'s field-family coverage table names the families the generator does **not**
  vary; a defect confined to one of those would not be caught by this property alone.
- `H-IDS` certifies the pure helper only. Runtime modifier collision rejection and region-map
  caller behaviour remain obligations of packets `08` and `11`.
- `H-CONFIG-BOUNDS` is a bounds-only API: absent keys and non-numeric variants pass it by
  design, and no cross-field whole-config validity is inferred.
- No runtime budget or pass-count is predeclared; the numbers above are measurements, and the
  only wall-time claims are the measured ones.

## Closure

Independent review closed all review findings and approved implementation
(`item 2 provenance reconciliation` was the last open item; the final signoff returned
APPROVED). The packet is marked `status: implemented` at `e2f4`-series head; the checkpoint
commits on the branch are deliberately **not** squashed, because the evidence above cites the
per-run revisions (`ac9dc0ef` for the routine table, `ba807a18` for the extended/replay tables)
and squashing would delete those objects and orphan the provenance.
