# Implementation Plan: pbt-ir-config

Wait for implemented #1/#2, re-read their exact exports and accepted profiles on disk, then use their direct submodule paths and runner. Do not implement a second harness, change shared export shapes casually or rename/duplicate `H-IR-ANCHORS`. Commands below use the spec's capture wrapper/aliases and archive logs before overwrite. Ordered steps keep default and explicitly enabled targets compiling; unregistered/zero-test output is never a gate.

## Step 1: Baseline the actual owner targets and establish bounded charters

- Objective: derive honest expectations and numeric/size safety before writing properties.
- Surface:
  - `crates/slicer-ir/src/{slice_ir,entity_id,resolved_config,region_split_registry,validation}.rs`, `crates/slicer-sdk/src/coords.rs` and `crates/slicer-scheduler/src/config_resolution.rs`: inspect live semantics/call paths; make no production changes for test access.
  - Prerequisite `docs/testing/properties.md`: complete the six charters plus existing H-IR anchor extension, with all source §5 fields, bounds inequalities, independent oracle dependencies, shrink witness modes and targeted failure destinations. Read relevant canonical docs from the spec first.
  - New `docs/testing/proptest-ir-config-evidence.md`: baseline command/source/toolchain/features/dirty-content state and actual pass/fail logs. Run the existing IR standalone/lib and scheduler filtered targets listed in Verification before edits, confirming Cargo metadata/gates and named test population. Preserve frozen RegionMap fixture bytes.
- Verify: the spec's exact existing baseline commands, individually captured; inspect `target/test-output.log` instead of rerunning for truncated output. Guest-related failures require `cargo xtask build-guests --check` by exit code before attribution. Validate the coordinate product/cast/error and modifier-parent u128 derivations independently.
- Done when: every property has a precise valid/invalid/raw-representation domain and oracle; observed baseline failures are recorded rather than assumed from stale test comments. Domain/feasibility measurement is implementation work, not a new scope interview.

## Step 2: Add deterministic oracle/domain controls and feature-gated target wiring

- Objective: prove small references can falsify plausible faults before they certify random cases.
- Surface:
  - `crates/slicer-ir/Cargo.toml`: consume foundation feature/dev-dependencies and declare new `property_ir_contracts` requiring `property_testing`; extend existing planned `property_ir` rather than create a duplicate.
  - `crates/slicer-scheduler/Cargo.toml`: add default-off empty `property_testing` selection feature and dev-only SDK/Proptest dependencies; explicitly register new `property_config` and `property_config_contracts`, requiring `property_testing`. Use workspace-managed Proptest resolution, no normal/test-support cycle or engine feature.
  - New `crates/slicer-ir/tests/property_ir_contracts.rs`, `crates/slicer-scheduler/tests/property_config_contracts.rs` and target-private common modules as needed: checked domain constructors, rational/outward-rounded numeric envelope reference, independent config representation key/first-occurrence model, ordered-set anchor oracle use, all-declarations bounds predicate and iterative mixed-radix chain model.
  - Deterministic controls named in ACs: factor/truncation/false-inverse coordinate defects, unsupported parent/assertion/sentinel witnesses, side-table anchor false acceptance, JSON omission/null/type failures, exceptional Postcard bits, omitted-field/native-float/default-pool mistakes, bounds union/off-by-one/index loss, chain empty/order/product defects and impossible/overflowing domains.
- Fallout: register common Rust modules explicitly in their actual standalone target roots, do not put unregistered tests into scheduler's aggregated directories. New files may have private helper types, not new public production APIs. Use `print_entity_base`/`extrusion_path3d_base` and FRU for watched `PrintEntity`, `LayerCollectionIR`, `TravelMove`, `RegionKey`/`RegionPlan` fixtures; raw exceptional floats use `from_bits`. Mark legitimate encode/decode and determinism waivers precisely. Existing production fields/versions are unchanged.
- Verify: `I`, `S`; explicit all-targets checks for each owner with `--features property_testing`; discover target tests and confirm substantive nonzero named execution. Controlled faults remain test-local, never installed into production code or registered as passing correctness properties.
- Done when: independent models reject all stated counterexamples, unsafe bounds cannot become silent empty domains, and both enabled target sets compile while default builds remain unchanged.

## Step 3: Implement coordinate and ID properties against real seams

- Objective: explore bounded unit/namespace contracts without false exactness or uniqueness claims.
- Surface:
  - `crates/slicer-ir/tests/property_ir.rs`: new exact `proptest_coord_conversion_envelopes` and `proptest_entity_and_modifier_ids`, through `PropertyRun` and shared scalar/ID strategies. Call real IR/SDK/Point2 conversions; compare mathematical envelopes and exact half witnesses. Construct exact integer geometry without float identity loss.
  - Generate bounded allocation counts/fresh generators and full supported parent range with explicit boundary modes. Compare mathematical sequence/namespace quotient/remainder/decode, deterministic identity and actual minting assertion for unsupported parents. Do not add a start-at allocator or hidden collision validator.
  - `test-properties.toml`: add the exact H-COORD/H-IDS records from the spec, with temporary calibration held outside accepted completion profiles until measured. Charter/profile values become accepted only after pilots; do not leave placeholder entries or fabricated budgets in the final registry.
- Verify: `I`; exact registry-backed selection of `H-COORD` and `H-IDS` using `cargo xtask test --summary --properties --property H-COORD --property H-IDS --tier routine` after calibrated profiles exist; record resolved config/evidence. Re-run existing `ir_tests` and `entity_id_invariants_tdd` captured separately.
- Done when: both properties reach real production calls for generated and shrink invocations, deterministic boundaries execute regardless of random luck, and accepted limits satisfy the written derivations. No universal inverse/hash-injectivity guarantee appears.

## Step 4: Complete anchor and selected IR serde coverage

- Objective: preserve foundation identity while covering missing default/serialization/error paths.
- Surface:
  - Existing planned `proptest_ir_travel_anchors_membership` in `property_ir.rs`: consume #2 `anchor_cases`/`first_missing_anchor`, add duplicate/reordering/extreme/repeated/side-table witnesses without changing `H-IR-ANCHORS` registry identity, bounds names or actual validator call.
  - New `proptest_ir_serialization_and_defaults`: shared `entity_layers(EntityDomain)` with explicit compatible roles and finite dyadic paths, bounded matching side tables/travels, observation-before-encode and independent assertions after actual JSON/Postcard decode. Preserve required association fields, exact integer bits and optional lock/tool/default semantics.
  - `property_ir_contracts.rs`: precise missing-map-field and required-field/null decode failures; raw exceptional Postcard tests are explicitly representation-only. No universal IR validator, schema bump or truncated-Postcard-default claim.
  - Registry: exact H-IR-SERDE record; extend complete charter/evidence and preserve frozen RegionMap artifact ownership.
- Fallout: fill all dependent path/entity/side-table fixture fields together with FRU bases; shrinking retains role/path/anchor/side-table consistency. Add no struct fields or old-value assertions requiring schema changes. Do not regenerate the independent RegionMap fixture if a test goes red.
- Verify: `I`; registry selection `H-IR-ANCHORS` and `H-IR-SERDE` through xtask; captured existing `ir_validation_tdd`, `entity_id_invariants_tdd` and `region_map_versioned_decode_tdd`.
- Done when: first missing anchor/error text, exact existing/defaulted fields and finite versus exceptional serialization policies are proved with actual execution and negative controls.

## Step 5: Implement config/interner and numeric-validation properties

- Objective: exercise actual Eq/Hash/interner and public validating boundaries with independent models.
- Surface:
  - `crates/slicer-ir/tests/property_ir.rs`: `proptest_config_equality_hash_and_interning` builds independently constructed equal/unequal configs, compatible finite fixtures and clearly separated raw-bit mode, canonical Postcard representation keys and first-occurrence ranks. Verify actual `intern_config`, `config_for_raw` and keyed `config_for` with a nondefault plan. Hash equality is an implication only.
  - `crates/slicer-scheduler/tests/property_config.rs`: `proptest_config_scalar_bounds` uses direct scalar/config strategies and bounded declarations/lists, drives real `ConfigBoundsIndex::check`, compares to per-declaration predicates and observes exact `OutOfRange` fields/first index. Include missing keys, no-bound-but-present declarations, type separation and percent-min-only semantics; do not widen `check_scalar` visibility.
  - Deterministic targets: actual `apply_cli_key` wrong-type and u32-boundary tests; independently proved representation/oracle controls and unsafe bound/invalid-mode shrink probes. Audit live fixed/option/sequence/extension Eq/Hash coverage rather than claim every field from one sampled list.
  - Registry/charters: H-CONFIG-INTERN and H-CONFIG-BOUNDS exact records and reviewed field-family coverage/disposition.
- Fallout: no new production config fields/keys/variants or numeric normalization rules. If a confirmed defect requires a small implementation correction, update the actual owning Eq/Hash/check/extractor and affected literals/assertions together, retain frozen layout and add a named concrete regression before claiming closure. Larger/schema-changing fixes are explicit blocking work, not silent scope expansion.
- Verify: `I`, `S`; registry-backed selection of the two config IDs; captured existing `resolved_config_defaults_tdd`, `region_map_versioned_decode_tdd`, `--lib resolved_config`, and scheduler `scheduler_integration` filters `config_bounds_enforcement_tdd` / `config_resolution_tdd`. Preserve failure evidence before each subsequent run overwrites the log.
- Done when: properties can detect ignored fields/default-pool mistakes and bound/nonfinite/index defects, valid config claims remain distinct from raw representation tests, and affected existing tests pass without weakening semantics.

## Step 6: Implement canonical-chain product coverage and resolve concrete findings

- Objective: prove the helper's full ordered Cartesian contract, including empty/duplicate/order edge cases, with bounded allocation.
- Surface:
  - `crates/slicer-ir/tests/property_ir.rs`: new `proptest_canonical_chain_product`, with construction-bounded unique axes/value lists plus explicit duplicate-value mode; compare complete output to ordinal mixed-radix decoding. Preserve caller Vec order and duplicate multiplicities, ignore unrelated map keys, and check widened products before allocation.
  - `property_ir_contracts.rs`: empty/all-missing/multiple-axis and product-limit witnesses, deliberate oracle faults, shrink witness preservation. Registry adds exact H-CHAINS record.
  - Actual owning production code and narrow concrete regressions only when a reproduced finding proves a defect. Use #2 formatter only for supported scalar/layer cases; manually compose escaped config/chain fixtures where needed, no imaginary formatter variants. Record root cause, previous concrete input and generator/oracle/SUT classification. Do not commit seeds as regression history.
- Verify: `I`; targeted H-CHAINS registry invocation; captured existing `region_split_registry_tdd`; rerun each affected family and existing narrow regression after a fix. Do not relax ordering/tolerance/domain merely to make a false property green; correct and document its actual contract instead.
- Done when: no empty-only/vacuous coverage, cardinality/content/order agree independently, and each confirmed SUT defect has a standalone non-RNG regression. Larger unresolved findings block affected G-FAMILY acceptance.

## Step 7: Measure profiles, prove routine/extended Windows execution and close docs

- Objective: deliver real executed coverage rather than registry-only/test-feature false greens.
- Surface:
  - `test-properties.toml`: finish measured routine/extended profiles and all exact bounds from the spec, respecting checked safety inequalities and positive substantive witness populations. Preserve #1 schema/context/events and fresh default sampling; do not defeat overrides with unconditional Proptest config.
  - `docs/testing/proptest-ir-config-evidence.md`: actual pilot attempts/rejections/shrink behavior, source/dirty-content settings, selected limits, reference numeric proofs, named execution/controller evidence, compile versus run measurements and concrete regression ledger. Record unknown/unavailable metrics honestly.
  - `.github/workflows/ci.yml`: verify prerequisite `properties-windows` dynamically executes the delivered records; change only necessary lane support, not a new hand-maintained per-property roster. Final manual-dispatch audit remains #17.
  - Canonical docs/charters listed under Docs impact; no Packet Queue/AGENTS edits. Preserve ownership and contracts, documenting only proved boundaries/links.
- Verify: complete `P` routine, extended and recorded-seed replay including all six additions and H-IR-ANCHORS; read/archive all logs/events. Then incremental combined routine registry selection, prerequisite deterministic regression targets, workspace build/all-targets check/clippy and optional-feature variants, literals/test-quality report and guest freshness as specified. No workspace test suite is required.
- Done when: actual Windows tests/CI executed the expected exact properties, no missing/zero/exhausted result is called passed, measured profiles and complete charters are durable, all discovered defect obligations are resolved or explicitly blocking, and independent review can reproduce evidence. Authoring does not itself change draft to implemented.
