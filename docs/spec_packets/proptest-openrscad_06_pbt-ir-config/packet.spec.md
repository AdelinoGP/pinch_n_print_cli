---
packet: proptest-openrscad_06_pbt-ir-config
status: draft
---

# Independently oracled IR, coordinates, IDs, config and chain properties

## Goal

Complete H-COORD, H-IDS, H-IR, H-CONFIG and H-CHAINS with bounded, falsifiable owner-local properties, concrete defect regressions and measured Windows routine/extended execution.

## Why

The approved source is `docs/specs/proptest-openrscad-adoption.md` §§3–7, §10 and G-FAMILY. Existing examples exercise coordinates, IDs, travel anchors, serialization, configs and variant enumeration, but do not explore their declared domains. The foundation's `H-IR-ANCHORS` is a real vertical slice, not completion of all these families.

The dangerous false properties are already identifiable from source: f32 conversions are not universal exact inverses; IDs based on a bounded hash are not injective; bitwise float config equality is not native float equality; `ConfigBoundsIndex::check` does not validate every unbounded key; and chain enumeration preserves caller value order rather than sorting it. This packet specifies the actual contracts instead of weakening an assertion after exploration finds a counterexample.

## Scope

- In: complete charters and derived scalar/size domains for the five assigned families; shared direct strategy consumption; independent mathematical/representation/set/mixed-radix oracles; registered property tests and deterministic negative controls; focused fixes with named concrete regressions when confirmed defects surface; relevant existing-test baselines; measured profiles, Windows execution evidence and canonical documentation updates.
- Out: new IR fields/schema/WIT versions, public visibility widened for tests, SCAD/engine dependencies, full scheduler claims/DAG/registry coverage, runtime collision/transaction geometry coverage, arbitrary unrestricted float/hash guarantees, all-fields IR validation, a live canonical differential harness or randomized guest/E2E campaign. A larger defect blocks affected family closure; it is not deferred as passing coverage.

## Depends on

- `docs/spec_packets/proptest-openrscad_01_pbt-foundation/` (**draft at authoring**): exact planned `slicer_sdk::test_support::property_testing::{PropertyRun, CaseGuard}`, `PropertyRun::{new,bound,check}`, IR `property_testing` feature/`property_ir` target, `H-IR-ANCHORS` identity, registry schema/context/events/artifacts and `cargo xtask test --properties` selection.
- `docs/spec_packets/proptest-openrscad_02_pbt-domains/` (**draft at authoring**): exact planned `domains::{DomainError, finite_f32, finite_f64, nonfinite_f32, nonfinite_f64, rounding_boundary_mm, ScalarConfigDomain, scalar_config_values, IdTextKind, id_text, AnchorDomain, AnchorCase, anchor_cases, EntityDomain, entity_layers}`, `oracles::first_missing_anchor`, and `regression::{ConcreteInput, format_regression_fixture}`. Imports are direct `domains`/`oracles`/`regression` submodules, not invented re-exports. Implementation waits for both prerequisites to be implemented and their exports/profiles reverified. No dependency is claimed satisfied.

## Design notes

### Grounded production seams and additions

- `mm_to_units(f32) -> i64`, `units_to_mm(i64) -> f32`, `Point2::{from_mm,to_mm}`, `UNITS_PER_MM` (`crates/slicer-ir/src/slice_ir.rs`) and wrappers/`SCALING_FACTOR` (`crates/slicer-sdk/src/coords.rs`) implement the current f32 boundary. Integer identity lives in `Point2 { x: i64, y: i64 }`.
- `LayerEntityIdGen::{new,next}` (`crates/slicer-ir/src/entity_id.rs`) starts at 1, uses `Cell<u64>` and is intentionally `!Send + !Sync`. Do not fabricate a reset/start-at/checked-next API. Allocation exhaustion is not exercised by looping to `u64::MAX`; the allocation property is explicitly bounded below overflow.
- `modifier_sub_region_id`, `modifier_sub_region_id_fits`, `is_modifier_namespace_id`, `modifier_base_region_id`, `MODIFIER_FOOTPRINT_REGION_ID` and `MODIFIER_VARIANT_REGION_ID_STRIDE` are in `crates/slicer-ir/src/slice_ir.rs`. The namespace bit is private; tests can state its documented mathematical value, not import an invented public constant. The minting helper asserts on unsupported parents; it does not reject hash collisions itself. `split_regions_by_modifier_footprints`, reached by `split_modifier_footprints` (`crates/slicer-runtime/src/region_partition.rs`), separately rejects duplicate minted identities transactionally; that caller behavior remains H-RUNTIME's obligation, not falsely credited here.
- `validate_travel_anchors` (`crates/slicer-ir/src/validation.rs`) tests only `ordered_entities` membership and identifies the first missing travel ID. Keep the foundation's existing planned property and ID; do not register an alias for the same test or assert a runtime call site that does not exist.
- `ConfigValue::{PartialEq,Eq,Hash}` (`crates/slicer-ir/src/slice_ir.rs`), `ResolvedConfig::{PartialEq,Eq,Hash}` / `slicer_ir::resolved_config::ResolvedFloatOrPercent` (`crates/slicer-ir/src/resolved_config.rs`) compare/hash float bits, including options, sequences and extensions. The latter type is not an assumed crate-root re-export. `RegionMapIR::{intern_config,config_for_raw,config_for}` (`crates/slicer-ir/src/slice_ir.rs`) seeds default config at index 0 and interns via equality. `config_for` panics on broken construction invariants; it is not a validating API.
- `ConfigBoundsIndex::{from_declarations,check}` and `BoundsDeclaration` (`crates/slicer-scheduler/src/config_resolution.rs`) provide an existing public seam that calls private `check_value`/`check_scalar`. Do not make those private functions public. `check` returns `Ok` for a missing bounds key and nonnumeric scalar variants; a present no-min/no-max declaration still reaches the finite check. Percent-authored values check finite/min only here; absolute maxima belong after expansion. `ResolvedConfig::apply_cli_key` supplies type/range semantics separately; it is not a universal finite validator by itself.
- `enumerate_canonical_chains(&HashMap<String, Vec<PaintValue>>, &[String]) -> Vec<Vec<(String, PaintValue)>>` (`crates/slicer-ir/src/region_split_registry.rs`) recurses omit-first, then input value order. `docs/03_wit_and_manifest.md` § Variant-chain enumeration order governs the caller's priority/name and value sorting; this helper does not perform that sorting or validate duplicate axes.
- Extend prerequisite `crates/slicer-ir/tests/property_ir.rs` and add **new** standalone `property_ir_contracts` (required feature `property_testing`). Add **new** empty selection feature `slicer-scheduler/property_testing = []`, dev-only workspace Proptest and SDK `property_testing` dependencies, and **new** standalone `property_config` / `property_config_contracts`, both requiring it. Do not add a normal dependency cycle/engine edge or change default features. All new tests below are planned, not existing execution claims. No new public SDK/production API is required.

<!-- snippet: coord-system -->
- Units: 1 unit = 100 nm (10⁻⁴ mm), not OrcaSlicer's 1 nm; divide OrcaSlicer constants by 100 and convert with `Point2::from_mm` / `mm_to_units()` (`docs/08_coordinate_system.md`).

<!-- snippet: wasm-staleness -->
- Feeds guest WASM: run `cargo xtask build-guests --check` after edits and judge by exit code (0 fresh, 1 stale, 3 infrastructure error) before interpreting any guest test failure (AGENTS.md §Guest WASM Staleness).

No new OrcaSlicer behavior/config key is ported. These are existing PnP contracts and original mathematical/test models. Preserve canonical behavior if a focused production fix is necessary; a fix requiring new borrowed upstream behavior must first verify its file/function provenance and apply repository attribution rules.

### Charter H-COORD: bounded conversions, not fictional exact inverses

Owner: `slicer-ir/property_ir`, exact test `proptest_coord_conversion_envelopes`, calling actual IR functions, `Point2` methods and SDK wrappers. Contract: `docs/08_coordinate_system.md` § Conversion & Determinism. Generate bounded finite f32 pairs with signed zeros, subnormals, interval ends and `rounding_boundary_mm` neighbors; independently ordered pairs test monotonicity. Use profile integers `M = max_abs_mm`, `U = max_abs_units`, requiring `0 < 10_000*M <= 2^24` and `0 < U <= 2^24`. Feed `min(U, 10_000*M)` to the shared rounding-boundary strategy so its mm outputs also satisfy this charter's bound. Checked bound conversion must reject excessive profiles, not truncate them. These safety inequalities keep scaled products finite, integer outputs far from cast saturation, and integer-to-f32 inputs exact; they do **not** prove an inverse round trip.

Derive the oracle before accepting profiles. For exact real input value `m` represented by f32, `t = 10_000*m` can be calculated exactly in f64 on this domain (a 24-bit significand times the integer factor fits f64 precision). For f32 round-to-nearest multiplication, use a proved error `e_mul <= 2^-24*abs(t) + 2^-150` in **units**. Then `abs(mm_to_units(m) - t) <= 1/2 + e_mul`. For integer `n` with `abs(n) <= U`, input cast is exact and `abs(units_to_mm(n) - n/10_000) <= 2^-24*abs(n/10_000) + 2^-150` in **mm**, using an exact-rational expectation or a separately bounded f64 reference error. Document any additional reference arithmetic error rather than treating f64 as real arithmetic. Compose these budgets for a round trip; never assert it is exactly `n` or universally within half a unit. Use outward-rounded/rational comparison limits, not a tolerance tuned from failed samples.

Test exact half-away cases independently: `m = 1/32 mm` has exact scaled value `312.5 units`, expected `313`, with the symmetric negative case. Neighbor floats probe both sides; both zeros map to integer zero. Wrapper/Point2 agreement is exact (and explicitly only consistency evidence); analytical envelopes and half cases provide correctness. Exact integer `Point2` equality/hash/serde identity, including full-i64 extrema, never takes a float round trip. Nonfinite/unbounded conversion is outside this valid numeric charter: these raw helpers promise no `Result` validation, so do not invent nonfinite rejection. Shrinking remains finite/in-bounds, retaining a boundary/sign mode when required. Deterministic oracle controls reject a synthetic factor-of-100 error, truncation at the exact half case and unjustified exact-roundtrip assertion.

### Charter H-IDS: bounded allocation and representable modifier parents

Owner: `slicer-ir/property_ir`, `proptest_entity_and_modifier_ids`. For `0..=N` allocations from a fresh generator, with measured `max_allocations = N < u64::MAX`, compare the actual sequence to mathematical `1..=n`, including zero/singleton/repeated fresh generators; sentinel zero is never allocated. Retain the existing static `!Send + !Sync` assertion in `entity_id_invariants_tdd`; this property does not change concurrency/exhaustion semantics.

For modifier IDs let `S = MODIFIER_VARIANT_REGION_ID_STRIDE`, `B = 2^63` and `P = floor((B - S - 1)/S)`. Derive with checked u128 arithmetic that the documented worst-case payload fits iff `parent <= P`, since `parent*S + (S-1) < B-1`. Generate parents throughout `0..=P`, deterministic `P`, `P+1`, base/namespace sentinel boundaries and `u64::MAX`. For supported parents assert the actual output is high-bit marked, not `MODIFIER_FOOTPRINT_REGION_ID`, its low payload quotient is the parent and remainder lies in `1..S`, and public decode returns that parent. Different parents cannot share IDs; different object/footprint inputs **can**, so never assert universal injectivity or unequal hashes. Repeated identical inputs are determinism evidence, not an independent hash oracle.

Use bounded case-sensitive ID text and integer-grid footprint records, including empty/holed witnesses; no new claim that hashing validates geometry. Existing concrete hole-participation test in `ir_tests` stays relevant, but do not require every arbitrary geometry mutation to change a finite hash. Unsupported parents return `false` from `modifier_sub_region_id_fits`; deterministic `catch_unwind` tests observe the documented assertion/message if minting is called anyway. Ordinary low-bit IDs and the footprint sentinel are not modifier IDs and decode to `None`. Bit-marked arbitrary IDs may decode even if not actually minted; do not equate namespace membership with provenance validation. Shrinks retain supported/unsupported mode and bounded allocation lengths. Runtime collision detection is explicitly handed to #8; no fictitious collision error is added to this helper.

### Charter H-IR: membership, finite serialization and explicit nonfinite policy

Preserve `H-IR-ANCHORS` / `proptest_ir_travel_anchors_membership` exactly, using prerequisite `anchor_cases` / `first_missing_anchor`. Complete its independent set assertions with duplicate/reordered `ordered_entities`, repeated travel references, valid/missing anchors `0` and `u64::MAX`, and a missing ID present only in an unrelated side table. The expected missing ID is first in travel order; the error includes that ID and `layer.ordered_entities`. This is not an allocator-sentinel validator. Shrinking retains valid/dangling witnesses.

Add `H-IR-SERDE`, owner `slicer-ir/property_ir`, `proptest_ir_serialization_and_defaults`. Generate bounded finite dyadic `entity_layers(EntityDomain { ... })` with explicit role/path compatibility, then bounded travels, path tool/lock values, entity tool selectors and aligned speed-profile/support-identity side tables. Capture a field/bit observation before encode; verify Postcard and finite-domain JSON decode preserve entity IDs, exact Point2 integer coordinates, selected float bits, ordering, roles, keys, tools, optional locks and side-table associations. This is a deliberately selected IR slice, not certification of every IR type or printable validity. Mark encoder/decoder pairs with a test-quality waiver naming the actual pair; no golden generated by the SUT is an independent oracle.

For legacy JSON remove **only** actually defaulted fields from serialized fixtures: `PrintEntity.tool_index -> 0`, `ExtrusionPath3D.tool_index -> None`, `ExtrusionPath3D.order_lock -> None`, `LayerCollectionIR.speed_profiles -> []`, `LayerCollectionIR.support_entity_identities -> []`. Independently assert those defaults, while omission of required `entity_id` and wrong ID type reject. Omission tests are JSON map compatibility, **not** a claim that serde defaults recover arbitrary truncated Postcard sequences. Keep the existing frozen RegionMap Postcard fixture unchanged.

Nonfinite policy is boundary-specific: JSON correctness properties admit finite dyadic records only; primitive/`ConfigValue` nonfinite serialization may produce null and must never be credited as a lossless successful JSON round trip. A JSON null in a required f32/f64 field is rejected on decode. Postcard is separately tested on raw scalar/`ConfigValue` bit patterns, including signed zero/infinities/NaN payloads; compare bits rather than native NaN equality, and do not call these production-valid config/model inputs. No blanket IR finite validator is introduced. Shrinking preserves each property mode and role/side-table relationships.

### Charter H-CONFIG: independent equality/interner model and validating bounds

`H-CONFIG-INTERN`, owner `slicer-ir/property_ir`, `proptest_config_equality_hash_and_interning`, exercises `ConfigValue`, `ResolvedFloatOrPercent`, `ResolvedConfig` and real `RegionMapIR` reads. Two clearly labeled input modes:

1. finite compatible owner fixtures based on `ResolvedConfig::default()` with explicit positive layer/first-layer/line width and valid role/flow choices, using `ScalarConfigDomain` for requested scalar variants; these are compatible structural fixtures, not a replacement for the full resolver;
2. raw representation-contract fixtures with signed zeros, infinities, distinct/same NaN payloads, option/sequence fields and bounded nested extension lists. These test Eq/Hash/serde mechanics only, never pass through a claimed valid-config domain.

Generate equal pairs through independent construction/insertion orders and unequal pairs through real field/value/variant/length changes, not just `clone == clone`. Choose axes covering fixed f64 and f32 fields, `ResolvedFloatOrPercent`, options, sequence lengths and extensions; keep a reviewed field-family coverage table derived from the live `declare_resolved_config!` and handwritten implementations, crediting existing deterministic field-specific tests where appropriate rather than pretending a fixed sampled roster is exhaustive.

For finite structural values require `0 < max_abs_config_value <= 2^24` before narrowing into f32-backed fields; positive/cross-field compatible subsets are chosen explicitly, not inferred from that magnitude bound. `max_list_len` bounds the **total** generated list elements across one config's extension trees, and `max_list_depth` bounds their nesting depth, preventing a per-node branching bound from hiding exponential allocation. Check config/text/element allocation arithmetic before construction. Raw exceptional-bit mode remains separately labeled and never certified finite by this bound.

Use canonical Postcard representation bytes as an equality oracle independent of the handwritten Eq/Hash: on these declared serializable structures it preserves scalar float bits and ordered BTreeMap content. Validate that premise with deliberate signed-zero/NaN/option/list/extension counterexamples first. The property is `actual_eq == representation_eq` and `actual_eq => hash_with_same_hasher(a) == hash_with_same_hasher(b)`; never demand distinct hashes for unequal values or persist an architecture-specific digest. For the pool model, use distinct representation keys and first-occurrence ranks, seeded with the serialized default. Compare each actual returned `ConfigId`, pool growth/idempotence and `config_for_raw` contents to that model. Insert a real `RegionKey`/`RegionPlan` using the returned ID and verify `config_for` returns the intended nondefault config rather than index zero. Do not index invalid IDs and call the resulting panic a missing validator.

`H-CONFIG-BOUNDS`, owner `slicer-scheduler/property_config`, `proptest_config_scalar_bounds`, drives actual `ConfigBoundsIndex::from_declarations` / `check` (hence private `check_scalar`) with small finite bound sets and flat numeric lists. Use an independent all-declarations predicate: an absolute scalar is finite and satisfies **each** present minimum/maximum. Compute expected effective min/max separately for error observation, including empty intersections; do not call production `NumericBounds::intersect` or use `check` to certify the generator. Restrict generated integer magnitudes/bounds to the exactly representable f64 integer domain, checked against `2^53 - 1`; finite f64 probes include neighbors immediately below/above inclusive bounds. Both zero signs and subnormals are meaningful; no unsupported cross-field whole-config validity is inferred.

Assert `OutOfRange { key, value, min, max, index }`, with `index = None` for scalars and the first invalid flat-list index otherwise, using bit/NaN-aware value observations. Cover one-sided/unbounded declarations, intersected/contradictory bounds and deterministic nonfinite cases. A completely **missing** key or a nonnumeric Bool/String passes this bounds-only API; actual `apply_cli_key` wrong-type tests separately assert `TypeMismatch` with key/expected/actual. Percent/percent-authored FloatOrPercent uses finite/min-only checks here; absolute FloatOrPercent uses full bounds. Do not impose a raw percentage's absolute maximum before Phase B.

Deterministic `ir_config_type_and_integer_range_rejections` calls real `ResolvedConfig::apply_cli_key` for Bool on `layer_height` (`TypeMismatch`) and negative / above-u32 `wall_count` (`OutOfRange`). No new canonical config key behavior or parser coercion is introduced. A generated invalid value is only passed to an API whose contract promises that validation. Shrinking retains the violating bound/nonfinite/first-invalid-index mode; scalar domains alone do not certify a cross-field config.

### Charter H-CHAINS: independent bounded Cartesian model

Owner: `slicer-ir/property_ir`, `proptest_canonical_chain_product`. Generate unique semantic axes, absent/empty axes, extra map keys not in canonical order and bounded `PaintValue::{Flag, ToolIndex, Custom}` lists. Valid unique-value mode uses distinct values; include unsorted order to prove the helper preserves the supplied Vec. A separate duplicate-value witness compares multiplicities rather than falsely deduplicating the helper's output. Scalar region-split declarations/duplicate canonical axes are outside the valid caller domain; this helper promises no validator for them.

Let each axis radix be `r_i = 1 + len(values_i)` or 1 when absent/empty. Check products/suffix products in widened arithmetic and require total `<= max_chains` before allocating a strategy/output expectation. The independent oracle decodes each ordinal `0..product` into mixed-radix digits with the last axis changing fastest: zero omits an axis, positive digit selects that value index. It does **not** copy the production recursive omit/include traversal. Compare the entire ordered sequence, cardinality, contents and per-chain canonical axis order. Empty/all-missing choices yield exactly `[[]]`; unrelated map insertion order/extra keys do not change output. Use exact integer/list/value equality, not unordered sets that erase multiplicity or ordering. Shrinks remove axes/values within bounds, preserving a required nonempty/two-axis witness for mode-specific assertions. Negative controls detect omission of the empty chain, sorting caller values, swapped axis order and duplicated/missing products.

### Exact registry handoff, measurement and defects

Append the following **new** records to prerequisite `test-properties.toml`; do not change its schema or duplicate `H-IR-ANCHORS`. All use kind `test`, `requires_host_algos = false`, `requires_scad = false`, `generator_version = 1`, and both measured `routine` / `extended` profiles. SDK's native dev graph still enables its required host algorithms; let metadata verify the actual union rather than relying on xtask's no-explicit-feature fallback. Bounds are u64 and checked before narrowing.

| ID | Package / target / exact test | Features | Required bounds keys | production_symbol / production_path | Charter |
|---|---|---|---|---|---|
| `H-COORD` | `slicer-ir` / `property_ir` / `proptest_coord_conversion_envelopes` | `["slicer-ir/property_testing"]` | `max_abs_mm`, `max_abs_units` | `slicer_ir::mm_to_units` / `crates/slicer-ir/src/slice_ir.rs` | `docs/testing/properties.md#h-coord` |
| `H-IDS` | `slicer-ir` / `property_ir` / `proptest_entity_and_modifier_ids` | `["slicer-ir/property_testing"]` | `max_allocations`, `max_id_bytes`, `max_footprint_points` | `slicer_ir::LayerEntityIdGen::next` / `crates/slicer-ir/src/entity_id.rs` | `docs/testing/properties.md#h-ids` |
| `H-IR-SERDE` | `slicer-ir` / `property_ir` / `proptest_ir_serialization_and_defaults` | `["slicer-ir/property_testing"]` | `max_entities`, `max_travels`, `max_points`, `max_layers` | `slicer_ir::LayerCollectionIR` / `crates/slicer-ir/src/slice_ir.rs` | `docs/testing/properties.md#h-ir-serde` |
| `H-CONFIG-INTERN` | `slicer-ir` / `property_ir` / `proptest_config_equality_hash_and_interning` | `["slicer-ir/property_testing"]` | `max_configs`, `max_list_len`, `max_list_depth`, `max_text_bytes`, `max_abs_config_value` | `slicer_ir::RegionMapIR::intern_config` / `crates/slicer-ir/src/slice_ir.rs` | `docs/testing/properties.md#h-config-intern` |
| `H-CONFIG-BOUNDS` | `slicer-scheduler` / `property_config` / `proptest_config_scalar_bounds` | `["slicer-scheduler/property_testing"]` | `max_declarations`, `max_list_len`, `max_abs_bound` | `slicer_scheduler::config_resolution::ConfigBoundsIndex::check` / `crates/slicer-scheduler/src/config_resolution.rs` | `docs/testing/properties.md#h-config-bounds` |
| `H-CHAINS` | `slicer-ir` / `property_ir` / `proptest_canonical_chain_product` | `["slicer-ir/property_testing"]` | `max_semantics`, `max_values_per_semantic`, `max_chains`, `max_text_bytes` | `slicer_ir::region_split_registry::enumerate_canonical_chains` / `crates/slicer-ir/src/region_split_registry.rs` | `docs/testing/properties.md#h-chains` |

Use `PropertyRun::new(ID, exact_test_name)` / `bound` / `check` and per-case guard for every property; do not bypass the runner with unconditional `Config::with_cases`. Retain the existing `H-IR-ANCHORS` bounds/metadata. Each charter in `docs/testing/properties.md` has all source §5 fields: owner, contract, domain, strategy/witnesses, actual oracle dependencies, tolerance derivation, shrink behavior, execution, failure record and acceptance. No SCAD selector is necessary for this family.

Create **new** `docs/testing/proptest-ir-config-evidence.md` with G-FAMILY disposition by charter and actual Windows baseline/pilot/routine/extended/replay commands, source revision and dirty-content provenance, features/toolchain/`CARGO_TARGET_DIR`, discovered/executed exact names, begin/end events and resolved profiles, accepted/rejected calls, exhaustion/shrink status, retained log/artifact locations, and measured compile versus execution costs. Calibration must permit substantive boundary, invalid and multi-element populations, not only empty cases. Required rare witnesses run deterministically alongside fresh exploration.

The foundation's required `properties-windows` routine job automatically selects these records; verify an actual Windows execution includes them. #17 owns final rollout/manual-dispatch auditing, not permission to defer this family's CI execution. Run the six new properties **plus** existing `H-IR-ANCHORS` in both tiers and an explicit retained-seed replay. No invented runtime budget or predeclared pass count.

For every failure reproduce concrete input, classify SUT versus generator/oracle/contract, and keep evidence before editing. Fix a generator/oracle defect as such. A confirmed SUT defect gets a focused canonical-correct production fix and named, minimized concrete regression stating root cause/previous input, running without RNG/context. The prerequisite formatter supports only its declared `ConcreteInput` variants: use it for scalar/layer cases, and write explicit bit-preserving config/chain fixtures when needed rather than inventing `ConcreteInput::Config`. No tolerance loosening, ignored assertions, source-adjacent seed history or frozen-fixture regeneration to obtain a pass.

## Acceptance criteria

`I`, `S`, `P` and capture mechanics are defined in Verification. Test names added here are **new planned tests**, except preserved prerequisite `H-IR-ANCHORS` and explicitly identified existing regressions.

- **AC-1** `H-COORD` under `P` satisfies the derived unit/mm envelopes and monotonicity on its checked finite domain, exact half-away/sign witnesses, SDK/Point2 consistency and no-float integer identity. `ir_coord_oracle_and_bound_controls` under `I` detects the stated scaling/truncation/false-roundtrip mutations and rejects unsafe profile bounds; no universal f32 inverse or invalid-input validation is claimed.
- **AC-2** `H-IDS` under `P` compares real allocation output to `1..=n`, tests the independently derived representable-parent boundary, namespace/payload/decode rules and sentinel exclusions. `ir_id_parent_and_sentinel_rejections` under `I` observes `P+1`/`u64::MAX` rejection by `fits` and minting assertion/message, without an exhaustion loop or global hash-injectivity assertion. Existing `entity_id_invariants_tdd` static and serde checks still execute.
- **AC-3** Preserved `H-IR-ANCHORS` under `P` calls the production validator with independent ordered-set results, including side-table-only/missing/first-error/reordered/repeated/extreme-ID witnesses. `ir_anchor_oracle_rejects_side_table_membership` under `I` catches a controlled accept-from-unrelated-table mutation. The test's exact registry identity is not duplicated or renamed.
- **AC-4** `H-IR-SERDE` under `P` preserves the selected finite JSON/Postcard field/bit observations and exact integer identity. `ir_serde_defaults_and_nonfinite_policy` under `I` observes all named absent defaults, rejects absent/mistyped required IDs and required numeric null, and separately checks raw Postcard exceptional bits without treating them as valid JSON/config inputs. The frozen `region_map_versioned_decode_tdd` fixture and current schema constants remain unchanged.
- **AC-5** `H-CONFIG-INTERN` under `P` agrees with the independently verified representation equality/first-occurrence model across independently constructed equal and unequal configs, zero signs, NaN payloads, options, sequences and extensions; equal values hash equally, interning is idempotent, default ID remains valid and a real keyed plan reads its actual nondefault config. `ir_config_oracle_counterexamples` under `I` rejects controlled omitted-field/native-float-equality/default-pool-read defects. Unequal hashes are never required.
- **AC-6** `H-CONFIG-BOUNDS` under `P` compares the real public bounds API to the all-declarations model, including inclusive edges, empty intersections, optional bounds, first invalid list position, finite/nonfinite and percent/absolute distinctions. `config_bounds_oracle_counterexamples` under `S` detects accept-all, union-instead-of-intersection, off-by-one-bound and lost-index defects. `ir_config_type_and_integer_range_rejections` under `I` observes the specified `apply_cli_key` variants/key/type/range fields. Existing bounds/resolution examples pass.
- **AC-7** `H-CHAINS` under `P` equals the complete ordered mixed-radix Cartesian model, including cardinality/product limits, empty/absent choices, caller value order, duplicate multiplicities and ignored extra map keys. `ir_chain_oracle_order_and_empty_controls` under `I` rejects the named ordering/empty/duplicate/missing-product mutations; existing `region_split_registry_tdd` still passes.
- **AC-8** Registry discovery and Windows routine/extended evidence show all six new IDs and `H-IR-ANCHORS` actually executed with matching passing controller events and accepted cases at least their measured profile cases; retained-seed replay works. Charter ledger contains domain/error proofs and oracle dependency reviews, selected counts/bounds/shrink limits from real pilots, baseline results and required routine CI execution. Compilation and execution costs are recorded separately, without an invented budget.
- **AC-N1** `ir_property_domain_rejections` under `I` and `config_property_domain_rejections` under `S` reject overflowing/narrowing profile bounds, unsafe coordinate domains, empty required witness modes and impossible Cartesian-size requests before sampling/allocation; valid shrinks retain in-bound/role/reference/violation witnesses. No empty strategy or exhausted run is accepted as passing correctness. Foundation missing/zero-execution and explicit-feature-composition probes remain green.
- **AC-N2** Every confirmed SUT defect found has a focused fix plus a named concrete regression with root cause/previous input and narrow passing invocation, and its property and affected existing tests pass unchanged in strength. Otherwise evidence states no confirmed SUT defect observed. Unresolved larger defects leave the affected charter/G-FAMILY incomplete, not waived by registry removal, ignored tests, looser tolerance or seed-only history.

## Verification

Authoring performed source/docs and locked/offline Cargo metadata inspection only. Execution below is an implementation obligation. Every plain `cargo test` uses Bash `set -o pipefail`, an existing `target/`, and suffix `2>&1 | tee target/test-output.log`; read/archive it before overwrite. Preserve native exit status with PowerShell `Tee-Object`. Registry/multi-crate runs use `cargo xtask test` and its combined log/freshness behavior. Confirm actual exact test discovery/execution, not green zero-test output.

- `I`: `cargo test -p slicer-ir --features property_testing --test property_ir_contracts -- --nocapture` with the capture wrapper.
- `S`: `cargo test -p slicer-scheduler --features property_testing --test property_config_contracts -- --nocapture` with that wrapper.
- `P`: `cargo xtask test --summary --properties --property H-COORD --property H-IDS --property H-IR-ANCHORS --property H-IR-SERDE --property H-CONFIG-INTERN --property H-CONFIG-BOUNDS --property H-CHAINS --tier routine`; repeat with `--tier extended`, then explicit replay using a recorded `--seed` value. Archive each run first. Do not execute `property_ir` directly without prerequisite context.
- Existing IR baselines, separately captured: `cargo test -p slicer-ir --test ir_tests`; `cargo test -p slicer-ir --test entity_id_invariants_tdd`; `cargo test -p slicer-ir --test ir_validation_tdd`; `cargo test -p slicer-ir --test region_split_registry_tdd`; `cargo test -p slicer-ir --test resolved_config_defaults_tdd`; `cargo test -p slicer-ir --test region_map_versioned_decode_tdd`. These standalone targets have no existing feature/required-feature gate. Also capture `cargo test -p slicer-ir --lib -- resolved_config --nocapture` for existing handwritten Eq/Hash/config-map tests.
- Existing scheduler tests are registered by `crates/slicer-scheduler/tests/integration/main.rs`: separately captured `cargo test -p slicer-scheduler --test scheduler_integration -- config_bounds_enforcement_tdd --nocapture` and `cargo test -p slicer-scheduler --test scheduler_integration -- config_resolution_tdd --nocapture`; no existing feature gate. Before interpreting any manifest/guest-related failure, run actual guest freshness and rebuild/stop by exit status.
- Prerequisite deterministic regressions: captured `cargo test -p slicer-sdk --features property_testing --test property_domain_contracts -- --nocapture`; captured `cargo test -p xtask --bin xtask -- property_ --nocapture`. Run the incremental combined routine registry gate `cargo xtask test --summary --properties --tier routine` after the targeted family passes.
- `cargo build --workspace`; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo check -p slicer-ir --all-targets --features property_testing`; `cargo clippy -p slicer-ir --all-targets --features property_testing -- -D warnings`; scheduler equivalents with `-p slicer-scheduler --features property_testing`. `cargo xtask check-literals`; `cargo xtask check-test-quality --report`; `cargo xtask build-guests --check` (rebuild on `1`, stop on `3`, apply staged-copy remedy if applicable). No workspace suite is required by this packet.

## Docs impact

- `docs/testing/properties.md` (**prerequisite-planned**, not present at authoring): add the six exact registry charters and expand existing H-IR anchor witnesses, numeric/error proofs, representation-only versus valid-input policy and field-family coverage/disposition. Check: `rg -n '^## H-(COORD|IDS|IR-SERDE|CONFIG-INTERN|CONFIG-BOUNDS|CHAINS)|H-IR-ANCHORS' docs/testing/properties.md`.
- **New** `docs/testing/proptest-ir-config-evidence.md`: G-FAMILY charter outcomes, measured profiles/source/Windows execution and defect classification/regressions. Check: `rg -n 'G-FAMILY|Windows|source_revision|H-CONFIG-BOUNDS|regression' docs/testing/proptest-ir-config-evidence.md`.
- `docs/08_coordinate_system.md` § Conversion & Determinism: link the derived bounded-domain property/error proofs and distinguish correctness from wrapper agreement; preserve canonical conversion policy. Check: `rg -n 'H-COORD|properties.md|round trip' docs/08_coordinate_system.md`.
- `docs/02_ir_schemas.md` § Canonical ID Types, § ResolvedConfig Hash invariant, § Config Interner Contract, § Stable entity IDs / LayerCollection default: document tested boundaries/nonfinite policy and link charters; do not claim new blanket validation, universal hash uniqueness, production validator wiring or changed serialized layout. Check: `rg -n 'H-IDS|H-IR-SERDE|H-CONFIG|properties.md' docs/02_ir_schemas.md`.
- `docs/04_host_scheduler.md` § Unified config-resolution entry points: link the bounds-only API charter and scope of `check_scalar`, absent keys and percent-before-expansion behavior without changing scope precedence. Check: `rg -n 'H-CONFIG-BOUNDS|bounds-only' docs/04_host_scheduler.md`.
- `docs/05_module_sdk.md` § Test Support: show the coordinate consistency property consuming shared direct strategies and link complete family charters. Check: `rg -n 'H-COORD|properties.md' docs/05_module_sdk.md`. No AGENTS.md edit or new ADR/deviation is needed.

## Open questions

- [FWD] Measure Windows pilot counts, size/rejection/shrink limits and routine/extended profiles within the proven numeric domains; select exact reference-arithmetic implementation and prove its error/overflow bounds before acceptance. No runtime budget is supplied by authoring.
- [FWD] Audit the live config declaration/Eq/Hash field families and existing deterministic coverage at implementation, then record which axes this property's generator varies and which remain covered structurally/example-wise. Do not describe a fixed sampled field list as exhaustive.
- [FWD] Downstream #8 must exercise actual runtime modifier collision rejection/transactionality; #11 must exercise region-map caller collision/unsupported-parent behavior. H-IDS here certifies the pure helper's bounded namespace/round-trip/assertion contract only, not those callers.

## Deviations

Recorded while implementing; none changes the packet's scope.

1. **The truncating mutant is inside the derived envelope at an exact tie.** §Charter H-COORD and
   AC-1 say `ir_coord_oracle_and_bound_controls` detects the stated "truncation" mutation. At
   `m = 1/32 mm` the exact scaled value is `312.5` units, so truncation yields `312` — exactly
   `0.5` units away and therefore *inside* the packet's own bound `1/2 + e_mul`. The control
   therefore rejects truncation with the envelope **plus** the derived half-away tie rule
   (`check_mm_case_with`), and separately asserts that the envelope alone accepts it. The bound
   was not tightened and no tolerance was tuned; the tie rule is checked independently because
   the envelope provably cannot see it.
2. **The minting assertion is observed deterministically, not per generated case.** AC-2 places
   the `P + 1` / `u64::MAX` minting-assertion and message observation in
   `ir_id_parent_and_sentinel_rejections` under `I`, and §Charter H-IDS says deterministic
   `catch_unwind` tests observe it. The `H-IDS` property's unsupported-parent arm therefore
   asserts only `modifier_sub_region_id_fits(parent) == false`: minting every unsupported sample
   would emit one panic per case, and `cargo xtask test --summary` reports any `panicked at`
   line as failure detail, so a passing property would carry a failing verdict line. The
   contract target owns the assertion text, as the AC already states.
3. **Registry discovery is all-or-nothing per target.** The executor fails a selected run when
   *any* registered test for that target is undiscovered, so the three later properties'
   records were held outside the live registry while their tests did not yet exist and appended
   as each landed. The committed registry contains all six records plus the preserved
   `H-IR-ANCHORS` entry, and the combined run reports all seven.
4. **The prerequisite discovery test's roster is now registry-derived.** Packet 01's
   `property_registry_validates_and_discovers_exact_tests` (`xtask/src/properties/executor.rs`)
   asserted that the live `slicer-ir`/`property_ir` discovery listed exactly one test. That
   target now carries six, so the assertion derives the expected roster from the live registry
   (every `slicer-ir` record whose target is `property_ir` and whose features include
   `slicer-ir/property_testing`) and compares it to a sorted live discovery, plus an explicit
   assertion that the preserved anchor test is still discovered. The check is strictly stronger
   than the fixed one-element roster and still fails on a registered-but-undiscovered test,
   which is the condition the executor reports as `missing_property_execution`.
5. **The `H-CONFIG-BOUNDS` shrink contract is now met as written, after a redesign.** An
   intermediate implementation grew the probe value through a `prop_flat_map` over the generated
   declarations, so a shrink of the declarations re-drew the value and could turn a list case
   into a scalar case (the independent delta review demonstrated it). That was recorded as
   blocking rather than waved through. It is now fixed properly: the strategy selects a probe
   mode from a `no_shrink` list and **derives** the value from the current declaration set inside
   `prop_map`, against a bounded witness key outside the generated alphabet
   (`property_config_oracles::WITNESS_KEY`). Shrinking the declarations re-runs the derivation,
   so the value shape always survives and, for the witness-derived modes, the verdict class and
   the first-invalid index survive too. The two `General*` families are independent draws, so only
   their shape is pinned and their verdict is whatever the oracle derives for the current
   declarations. Two checks pin this: `derived_probe_modes_have_their_documented_verdicts` asserts
   every mode's verdict and index on a concrete set, and the property runs a composed shrink audit
   on every execution.
6. **Both external blockers are resolved.** (a) The frozen `RegionMapIR` 3.0.0 Postcard fixture
   failed with `DeserializeBadBool` because commit `6db49f0f` removed
   `ResolvedConfig::flat_bridge_closing_join` from the middle of the struct; Postcard is
   positional, so every later field shifted. The field is re-added at its original position as a
   serialization-only `carrier` row kind (struct, default, wire position and `Eq`/`Hash` only —
   no effective-map entry, no typed key, no CLI dispatch), which restores the 3.0.0 wire layout
   without resurrecting the retired key; `region_map_versioned_decode_tdd` passes again with the
   frozen fixture and assertion strength unchanged, and no fixture was regenerated. (b) The
   required hosted Windows execution was obtained through the routine `properties-windows` lane
   on draft PR #6: workflow run
   https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43523810446 at commit `e157169`,
   job `Properties (Windows)` = success, step `Run routine properties` = success, 12 property
   binaries passed with `VERDICT: PASS` and no failures. Two earlier heads of the same branch
   also passed the lane.

## Authoring preflight

| # | Check | Result | Evidence / applicability |
|---|---|---|---|
| 1 | Symbols | PASS | Actual coordinate/ID/serde/Eq/Hash/interner/bounds/enumerator definitions read; helper boundaries, private visibility and downstream collision ownership checked; new tests/features marked planned. |
| 2 | WIT/IR identifiers | PASS | Actual path/tool/lock/default/side-table fields and ConfigValue/PaintValue variants checked; no invented alias/validator or WIT addition. |
| 3 | Schema versions | PASS | No version bump; frozen layout fixture retained and defaults tested only on supported map formats. |
| 4 | ADRs | PASS | ADR-0004/0016 and ADR-0054 amendment preserve SDK fixture home/gating; ADR-0030 wall-sharing/identity boundary unchanged; no ADR slot allocated. |
| 5 | Deviation IDs | PASS | No deviation identifier allocated, consumed or closed. |
| 6 | Prerequisites | PASS | Both prerequisites explicitly draft; exact module/function/feature/registry shapes read and matched; implementation waits. |
| 7 | Test targets | PASS | Existing targets/features grounded with Cargo metadata and scheduler module registration; new standalone required-feature targets explicit and property naming matches registry. |
| 8 | Criteria | PASS | Exact test IDs/fields/errors, independent models, controlled negative cases, measured execution and defect obligations; no workspace criterion. |
| 9 | Fallout | PASS | No struct/schema additions; feature/dependency/target and potential focused-fix fallout accounted for; fixture/FRU discipline retained. |
| 10 | Docs impact | PASS | Actual canonical headings and prerequisite/new documents identified with checks. |
| 11 | Completeness | PASS | No placeholders or unresolved scope-changing blockers; reference/profile/field-audit decisions have explicit bounded forward ownership. |

**Verdict: PREFLIGHT PASS (authoring only; family execution/measurement unperformed; implementation waits for #1/#2).**
