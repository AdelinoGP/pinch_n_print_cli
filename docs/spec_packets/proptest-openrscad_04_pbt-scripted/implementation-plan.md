# Implementation Plan: pbt-scripted

Wait for the direct-domain and adapter packets to be implemented, then recheck their actual exports and #3's adapter-prerequisite portion of G-SCAD evidence, not a full gate pass. This packet supplies recipe generation/shrink cost measurements and final eligible property profiles, combines them with #3's evidence to close full G-SCAD, and owns the combined G-GEOMETRY decision. Preserve their public shapes and the foundation's `PropertyRun::check` signature. Implement helper properties, not a new live engine-versus-slicer correctness oracle.

## Step 1: Add the native scripted feature and compile the typed recipe contract

- Objective: establish an explicit SDK opt-in without leaking native engine/Proptest into defaults or guests.
- Surface:
  - `crates/slicer-sdk/Cargo.toml`: planned `property_testing_scad`, native optional `slicer-scad` edge and explicit standalone `property_scripted` / `property_scripted_contracts` targets with `required-features = ["property_testing_scad"]`.
  - Planned prerequisite `crates/slicer-sdk/src/test_support/property_testing/mod.rs`: whole-module registration of new `scripted` behind native scripted feature only; leave production prelude/test/default/self-dev-dependency untouched.
  - New `crates/slicer-sdk/src/test_support/property_testing/scripted/mod.rs`: exact `Recipe2d`, `Recipe3d`, `ScadRecipe`, `SolidWitness`, bounds/domain/case/error, strategy/tree declarations and function signatures from the spec, implemented incrementally with real behavior, not placeholder passing tests.
- New contract: the spec's full planned SDK block is authoritative; no new `ConcreteInput` variant or modified direct geometry record. `ScriptedStrategy::Value` is certified `ScriptedCase`, not raw source text.
- Fallout: new `RecipeBounds` and `ScriptedCase` are watched structs. Use bounded default/FRU for the former and justified exhaustive replay-contract waivers for the latter; do not fabricate a valid default geometry/engine output. No pre-existing literal or IR/WIT version changes. If private foundation structures gain telemetry fields later, update their real construction sites together and keep public shapes unchanged.
- Verify: native feature-enabled all-target check; actual Cargo metadata/normal/build graph assertions in `scripted_feature_isolation_and_union`; feature-off/property-only checks and discovered guest graph inspection. Log tests according to the spec.
- Done when: default SDK `test` and property-only configurations stay engine-free; typed contracts compile under the new opt-in and are absent from production guest exports.

## Step 2: Implement deterministic SCAD emission and bounded reference scenes

- Objective: generate readable bounded self-contained source from typed primitives/contours/CSG/extrusions rather than arbitrary strings.
- Surface:
  - New `crates/slicer-sdk/src/test_support/property_testing/scripted/emit.rs`: `emit_scad`, checked traversal/size accounting and exact scale-aware source formatting.
  - New `crates/slicer-sdk/src/test_support/property_testing/scripted/reference.rs`: independently derived bounded region/orthogonal-cell/analytical-solid witness checks, relying on prerequisite independent predicates but not engine/production geometry routines.
  - New `crates/slicer-sdk/tests/property_scripted_contracts.rs`: `scripted_emission_and_independent_recipe_references` and `scripted_topology_and_solid_witnesses` with deterministic nonempty witnesses and negative controls.
- Approach: validate dimensions/turns/facets/bounds before emission; explicit polygon leaves consume #2 domain/winding conventions. Construct cases for every required 2D class and every `SolidWitness`, including multiple holes/bodies, genuine non-star-shaped visibility-kernel evidence and thin/quantization-scale clearances. Map recipe source through adapter `evaluate` in `evaluate_recipe`; no private upstream types or duplicate conversion. Derive expected faceted polygon geometry, box/extrusion volumes and reference interior slices/void/span observations independently; document error envelopes first.
- Verify: `C` filtered to those two test names, plus `scripted_rejects_bounds_invalid_output_and_fatal_adapter_diagnostics`; source is parsed/evaluated by the real pinned adapter. Read and archive logs between invocations. Private helper tests are actually registered in `crates/slicer-sdk/src/test_support/property_testing/scripted/mod.rs`.
- Done when: output-based independent observations distinguish required witnesses from intentionally removed holes/bodies/voids/spans, and source is deterministic/self-contained without resource/font/random dependencies.

## Step 3: Implement validated Strategy/ValueTree and truthful failure telemetry

- Objective: expose only certified domain cases during the entire real shrink protocol and retain errors that occur outside callbacks.
- Surface:
  - New `crates/slicer-sdk/src/test_support/property_testing/scripted/strategy.rs`: `scripted_cases`, `ScriptedStrategy`, `ScriptedValueTree`, `recipe_complexity`, bounded candidate enumeration/cached restoration and per-tree evaluation accounting.
  - Prerequisite `domains` implementation under `crates/slicer-sdk/src/test_support/property_testing/domains/`: reuse `geometry_cases`/`GeometryValueTree` for explicit contour leaves without changing their contract.
  - Prerequisite `PropertyRun::check` private implementation under `crates/slicer-sdk/src/test_support/property_testing/`: per-run/process generator telemetry audit before controller end events; keep callback/value/fresh-guard public signature unchanged.
  - `crates/slicer-sdk/tests/property_scripted_contracts.rs`: protocol, useful-shrink, domain-loss, generation/candidate/evaluation-exhaustion and injected fatal-error probes.
- Approach: constructive bounded templates first; evaluate candidate with the adapter, independently validate converted geometry and witness, then expose/cache. `current` performs no evaluation/I/O. Accepted value-changing shrinks decrease `RecipeComplexity`; record #2 `geometry_complexity` for contours. Repeated complicate returns cached certified predecessors; test false-return transitions explicitly. Domain-invalid candidates may count as rejection; fatal engine diagnostics/artifact errors must escape through truthful telemetry, not be swallowed by a `false` shrink result. Candidate-limit-on-known-failure means incomplete shrinking, not globally minimal input. Preserve `CaseGuard` per SUT callback; do not assume it covers `new_tree`.
- Fallout: any private harness telemetry fields update all real constructors/tests in this step. Event envelope/version and existing accepted-case semantics remain unchanged; no invented private Proptest counter or per-case panic hook.
- Verify: `C` for `scripted_value_tree_protocol_and_useful_shrinking`, `scripted_generation_and_shrink_exhaustion_are_not_success`, `scripted_rejects_bounds_invalid_output_and_fatal_adapter_diagnostics`; prerequisite `property_domain_contracts` and `property_harness` regressions. All plain runs tee to the canonical log and are read/archived.
- Done when: nonminimal rings/holes/sets/scripts/3D scenes structurally shrink with witness retention, every exposed state is valid, and fatal generation/shrink errors cannot produce a passed harness completion.

## Step 4: Persist before risky execution and implement concrete replay

- Objective: retain useful source and concrete geometry even if native execution aborts or minimization cannot finish.
- Surface:
  - New `crates/slicer-sdk/src/test_support/property_testing/scripted/artifacts.rs`: pre-attempt persistence, `write_scripted_artifacts`, `replay_scripted_case`, fixed filenames/local schema, safe atomic paths and error handling.
  - Strategy/evaluation private call sites: save recipe/source/options/bounds/engine/state before the real adapter call; update evaluated/rejected/failed state afterward and link the actual attempt from the foundation failure record.
  - `crates/slicer-sdk/tests/property_scripted_contracts.rs`: `scripted_artifacts_replay_and_concrete_fixture`, provenance/damaged-file negative tests and controlled failing-property replay probes.
- Approach: use source-state/generator/engine metadata from the real context plus adapter `engine_info`, never a guessed pin. Track fork-worker attempt ownership without duplicate controller completion. Compare retained source with re-emission, source/input digests (including options/bounds bytes), and expected generator/engine/backend/patch before replay; never fall back to defaults. Use fixed filenames, checked file bounds and explicit symlink/reparse-point escape rejection. Compile/evaluate #2 `format_regression_fixture` output via unchanged Geometry/Mesh variants; assert concrete bits/topology/indices with no engine/Proptest harness dependency. A controlled failure is proof of machinery, not a claimed SUT bug; confirmed real defects get named concrete owner assertions after diagnosis.
- Verify: the two artifact/replay tests under `C`; a real explicit-seed sequence/reproduction command through `P` after records exist; controlled interruption proof in step 5. Missing or unwritable files fail loudly, not silent optional fixture skips.
- Done when: pre-execution inputs survive native failure; same-pin retained-case replay uses no RNG and matches the declared deterministic geometry observations; changed provenance/damaged files are rejected, and a durable concrete fixture is evaluator-independent.

## Step 5: Supervise whole SCAD property processes and register measured pilots

- Objective: cover native generation/shrink work outside callbacks and exercise the actual runner feature union.
- Surface:
  - Planned prerequisite `xtask/src/properties.rs`: for `requires_scad` records require positive checked `bounds.test_process_timeout_ms`; supervise exact execution children including Proptest worker descendants with actual Windows process-tree containment, kill/join and status/artifact retention. Reuse existing combined log/gates, no recursive xtask invocations.
  - Registered xtask internal tests: `property_scad_supervision_covers_generation_shrink_and_descendants`, real feature union/discovery checks and failure/missing-completion regression probes.
  - `crates/slicer-sdk/tests/property_scripted_contracts.rs`: `scripted_abort_and_timeout_keep_pre_execution_case` using selected child modes before callback and during candidate evaluation; real bounded adapter execution under the same path.
  - `test-properties.toml`, new `crates/slicer-sdk/tests/property_scripted.rs`: exact two registry IDs/test names/charters/bounds keys from the spec, real `PropertyRun::new/bound/check` calls and fresh SUT guards.
  - New `docs/testing/proptest-scripted-evidence.md`: actual Windows pilots, process/native exit evidence, compile versus generate/render/shrink measurements, finite resolved profiles.
- Approach: callback Proptest fork/timeout remains an additional per-profile mechanism, not coverage of every render phase. Prove descendant cleanup and abnormal native statuses rather than treating controller termination as complete cleanup. Measure pilot bounds/case/rejection/shrink/timeouts before committing accepted values; both records require actual routine and extended execution. Derive emitter AST/geometry capacity bounds and record them instead of silently accepting unlimited engine settings.
- Verify: `X`, `C` interruption probes, then `P` routine/extended and actual seed replay. Inspect named discovery/execution/controller evidence and canonical combined logs; zero/ignored/missing callback runs remain failures. Retain aborted prepared attempts. Run prerequisite xtask feature/log/status tests.
- Done when: real SCAD records execute with explicit SDK features and preserved owner host-algos composition, every dangerous lifecycle phase is killable on Windows, and final eligible property profiles are based on measured recipe generation/render/shrink costs, not author estimates. Combine these results with #3's passed adapter-prerequisite evidence to record full G-SCAD; do not close that gate if any required evidence is missing.

## Step 6: Resolve the hybrid geometry gate and any measured bounded gap

- Objective: close combined G-GEOMETRY from evidence without overstating generality.
- Surface:
  - `docs/testing/proptest-scripted-evidence.md`: row-by-row direct/scripted geometry obligation matrix, deterministic witness/rejection/shrink evidence and exactly one defined disposition.
  - `crates/slicer-sdk/src/test_support/property_testing/domains/` private implementations and `crates/slicer-sdk/tests/property_domain_contracts.rs`, plus `D-GEOMETRY` charter/profile, only if a measured bounded direct family is missing: extend behavior under unchanged exported `GeometryDomain`/strategy/oracle contracts; do not modify the earlier draft packet artifacts.
  - A newly authored blocking generator/fix packet only if broad gaps are demonstrated; allocate identifiers when authoring, never from a frozen ledger. Do not implement a broad unreviewed redesign in this packet.
- Approach: reconcile #2 direct proof with scripts for convex/concave/non-star/holes/disconnected/thin/near-degenerate/contact-invalid and layered/3D cases. Use actual post-conversion validity and known-failing structural shrink probes, including witness loss negative controls. If `bounded_direct_gap_filled`, implement and rerun independent direct-domain proofs. If `general_generator_blocked`, name affected charters and leave this gate/closure incomplete pending follow-up or an explicit user-approved scope disposition; allow unaffected direct work to proceed.
- Verify: `C`, affected `D-GEOMETRY`/`D-MESH` routine and extended selections, then full incremental routine/extended `--with-scad` gates. Check all table rows have reachable fixtures/reference evidence, not only a family name or large case count.
- Done when: `hybrid_sufficient` or a proven `bounded_direct_gap_filled` satisfies all combined obligations. A blocking disposition is accurately documented but does not satisfy AC-8 or permit implemented status.

## Step 7: Wire the Windows lane, docs and optional-feature gates

- Objective: make delivered profiles required and reproducible rather than leaving registry-only coverage.
- Surface:
  - `.github/workflows/ci.yml`: new `jobs.properties-scad-windows`, checked-in Rust/CMake/MSVC and freshness prerequisites, real routine `--with-scad` registry execution and failure uploads of full logs/attempts; keep ordinary and adapter gates intact.
  - `docs/05_module_sdk.md`, `docs/21_data_defaults_and_fixtures.md`, prerequisite `docs/testing/properties.md`, `docs/scad/windows-feasibility.md` and new scripted evidence document: exact feature/API/charters/limits/replay and final combined disposition.
- Verify: all narrow and registry/optional-feature build/check/clippy/literal/test-quality commands in the spec on Windows, then actual guest freshness exit handling for SDK source/manifest edits. No workspace suite is required; use `cargo xtask test` for any broad/guest-dependent test run. Run docs checks and independent workflow closure review.
- Done when: two SCAD properties have true routine/extended execution evidence and required CI, default/product/guest dependency isolation holds, complete artifact/replay/witness docs are present, all required geometry obligations are met, and no discovered failure was hidden by looser assertions, skip filters or invented measurements.
