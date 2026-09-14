# Implementation Plan: resolved-config-view

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-567`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every cargo command is delegated and tee'd to `target/test-output.log`.

## Steps

### Step 1: Reconcile prerequisites and freeze blast radii

- Task IDs: `TASK-567`
- Objective: confirm packet 03/05 exports and record the exact 89 fallback and declaration-struct literal inventories.
- Precondition: packets 03 and 05 are landed.
- Postcondition: an implementation note records resolved signatures, all struct literal sites, and per-guest fallback locations matching AC-3 counts.
- Files allowed to read, with ranges when over 300 lines: predecessor authority files; named registry/binding symbols; 13 guest production trees through bounded searches.
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds: predecessor packet files, code edits, fixture contents, `OrcaSlicerDocumented/**`, generated code.
- Blast-radius discipline: dispatch `LOCATIONS` inventories for every `ConfigFieldEntry`, `HostConfigKey`, and `RegistryEntry` literal before Step 2.
- Expected sub-agent dispatches: prerequisite exports `FACT` ≤5 lines; struct literals and fallback sites `LOCATIONS` ≤20 per query.
- Context cost: `S`
- Authoritative docs: approved plan RC-4/RC-5 and Guests/delivery sections; packet 03/05 export contracts.
- OrcaSlicer refs: none.
- Verification: compare fallback totals to `29+27+10+4+3+3+3+2+2+2+2+1+1 = 89` and reject category leakage.
- Exit condition: any count mismatch, unresolved signature, or missing struct-literal owner prevents Step 2.

### Step 2: Add registry projection and config-block metadata

- Task IDs: `TASK-567`
- Objective: add default-true `config_block`, deterministic reconciliation, resolved projections, and focused red/green tests.
- Precondition: Step 1 inventories are complete.
- Postcondition: registry APIs materialize all effective declared values and a config-block-filtered map; extension values are typed/bounded.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-config/src/lib.rs`; named declarations in `crates/slicer-ir`; manifest parser symbols.
- Files allowed to edit (at most 3): `crates/slicer-config/src/lib.rs`; `crates/slicer-ir/src/config_schema.rs`; `crates/slicer-config/tests/resolved_config_view_tdd.rs`.
- Files explicitly out of bounds: guest sources, scheduler/runtime binding, G-code serializer, WIT.
- Blast-radius discipline: include every Step-1-discovered `RegistryEntry`/`ConfigFieldEntry` literal in this step or split into immediately adjacent ≤3-file substeps before activation; no compile-discovered cleanup.
- Expected sub-agent dispatches: cargo tests return `FACT` pass/fail or ≤20-line failure snippets.
- Context cost: `M`
- Authoritative docs: ADR-0067 and plan Extensions/Guests sections.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-config --all-targets --test resolved_config_view_tdd`.
- Exit condition: projections omit a declared default, accept an invalid extension, or depend on hash iteration.

### Step 3: Wire resolved views into production binding

- Task IDs: `TASK-567`
- Objective: make the binding API accept only effective resolved config plus registry metadata.
- Precondition: Step 2 projection tests pass and packet 05 resolver signatures are reconciled.
- Postcondition: production plans expose complete module-filtered resolved views; raw `config_source` is absent from binding; `resolved_config_view_tdd` is registered in the contract aggregator.
- Files allowed to read, with ranges when over 300 lines: named binding/live-plan functions and contract tests only.
- Files allowed to edit (max 3 per substep; 4 total split 3a/3b): 3a production `crates/slicer-scheduler/src/execution_plan.rs`, `crates/slicer-wasm-host/src/execution_plan_live.rs`; 3b `crates/slicer-runtime/tests/contract/resolved_config_view_tdd.rs`, `crates/slicer-runtime/tests/contract/main.rs` (register `mod resolved_config_view_tdd;`).
- Files explicitly out of bounds: guests, G-code serializer, unknown-drop mode, WIT.
- Blast-radius discipline: not applicable; no public struct field added; the aggregator registration counts as an edit against Step 3b's cap.
- Expected sub-agent dispatches: binding call-site inventory `LOCATIONS` ≤20; test run `FACT`.
- Context cost: `M` (substep budgets are slices, not additive)
- Authoritative docs: ADR-0068 and `docs/04_host_scheduler.md` binding sections.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --all-targets --test contract resolved_config_view_tdd` after 3b; AC-1 and AC-N2 exact commands during 3b.
- Exit condition: any production path binds raw source, omits a declared default, or exposes another module's key.

#### Step 3a: Production binding edits

- Task IDs: `TASK-567`
- Objective: change `bind_module_config_view` (`crates/slicer-scheduler/src/execution_plan.rs`) and `build_live_execution_plan` (`crates/slicer-wasm-host/src/execution_plan_live.rs`) to accept the loaded module, effective `ResolvedConfig`, and `ConfigSchemaRegistry`; no raw source input.
- Precondition: Step 2 projection tests pass and packet 05 resolver signatures are reconciled.
- Postcondition: production binding no longer reads raw `config_source` and compiles.
- Files allowed to edit (at most 3): `crates/slicer-scheduler/src/execution_plan.rs`; `crates/slicer-wasm-host/src/execution_plan_live.rs`.
- Files explicitly out of bounds: contract tests, guests, G-code serializer, unknown-drop mode, WIT.
- Blast-radius discipline: no public struct field added; touch only named binding/live-plan functions and their call sites.
- Expected sub-agent dispatches: binding call-site inventory `LOCATIONS` ≤20; `cargo check` `FACT` ≤5 lines.
- Context cost: `M` (slice of Step 3's `M`)
- Authoritative docs: ADR-0068 and `docs/04_host_scheduler.md` binding sections.
- OrcaSlicer refs: none.
- Verification: `cargo check -p slicer-scheduler -p slicer-wasm-host --all-targets`.
- Exit condition: a production path still requires raw `config_source` to compile.

#### Step 3b: Contract tests and aggregator registration

- Task IDs: `TASK-567`
- Objective: author `production_binding_is_registry_complete_and_resolved` (AC-1), `resolved_binding_preserves_module_encapsulation` (AC-N2), and the `guest_config_literal_fallback_census_is_zero` census (AC-3, expected red until Step 4) in `crates/slicer-runtime/tests/contract/resolved_config_view_tdd.rs`, then register the module.
- Precondition: Step 3a binding compiles.
- Postcondition: `crates/slicer-runtime/tests/contract/main.rs` contains `mod resolved_config_view_tdd;`; AC-1 and AC-N2 pass; the census test exists and is the only red test until Step 4.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/contract/resolved_config_view_tdd.rs`; `crates/slicer-runtime/tests/contract/main.rs` (one `mod` registration line only).
- Files explicitly out of bounds: production sources, other test modules, guest sources.
- Blast-radius discipline: registration is a single `mod` line; an unregistered or name-mismatched module compiles to zero tests and reports green.
- Expected sub-agent dispatches: AC-1/AC-N2 runs `FACT`; census failure `FACT` with the residual count only.
- Context cost: `S` (slice of Step 3's `M`)
- Authoritative docs: ADR-0067/0068 and `docs/22_test_quality.md` independent-oracle rules.
- OrcaSlicer refs: none.
- Verification: AC-1 exact command; AC-N2 exact command; `cargo test -p slicer-runtime --all-targets --test contract resolved_config_view_tdd` shows the census as the only failure until Step 4.
- Exit condition: registration missing (module compiles to zero tests), AC-1/AC-N2 fail, or the census test is absent rather than executed.

### Step 4: Remove guest fallback literals in bounded batches

- Task IDs: `TASK-567`
- Objective: replace all 89 classified fallbacks with required resolved reads while preserving unrelated `unwrap_or` uses.
- Precondition: Step 3 guarantees complete resolved views.
- Postcondition: AC-3's classified census is zero and excluded categories remain untouched.
- Files allowed to read, with ranges when over 300 lines: only Step-1-recorded locations and adjacent function bodies.
- Files allowed to edit (at most 3): batch A `arachne-perimeters`, `classic-perimeters`, `rectilinear-infill`; subsequent atomic substeps each edit at most three of the remaining ten named guest source files.
- Files explicitly out of bounds: guest tests, non-classified `unwrap_or` sites, host APIs, manifests, WIT.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches: one bounded location/result report per batch; cargo guest freshness only after all batches.
- Context cost: `M`
- Authoritative docs: plan Guests and delivery section; locked classification in AC-3.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --all-targets --test contract resolved_config_view_tdd guest_config_literal_fallback_census_is_zero -- --exact`; `cargo xtask build-guests --check`.
- Exit condition: the census is nonzero, more than 89 baseline sites were changed, or an excluded site was edited.

### Step 5: Make CONFIG_BLOCK registry-driven

- Task IDs: `TASK-567`
- Objective: consume the registry projection and encode the three intentional exclusions as metadata.
- Precondition: Step 2 projection exists.
- Postcondition: emission key/value membership is registry-derived and only the three named MMU keys are metadata-excluded.
- Files allowed to read, with ranges when over 300 lines: `ResolvedConfig::to_config_map`, `serialize_config_block`, existing CONFIG_BLOCK tests.
- Files allowed to edit (at most 3): `crates/slicer-ir/src/resolved_config.rs`; `crates/slicer-gcode/src/serialize.rs`; `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`.
- Files explicitly out of bounds: unrelated G-code formatting and whole-file golden baselines.
- Blast-radius discipline: include all host declaration-row syntax fallout identified in Step 1; split the metadata declaration edit if needed rather than exceeding three files.
- Expected sub-agent dispatches: focused test command `FACT`.
- Context cost: `M`
- Authoritative docs: plan RC-4 and Guests/emission sections.
- OrcaSlicer refs: none.
- Verification: AC-4's exact integration test.
- Exit condition: a hand-maintained emission roster remains or any key other than the three named keys is implicitly omitted.

### Step 6: Prove no-drop, then flip unknown retention

- Task IDs: `TASK-567`
- Objective: add the independent full-registry e2e and only after green change unknown handling to warn-and-drop.
- Precondition: Steps 2–5 pass and guest freshness exits 0.
- Postcondition: no-drop e2e is green; undeclared keys warn once and appear in no delta/view/emission.
- Files allowed to read, with ranges when over 300 lines: packet-03 ingestion module, registry census helper, existing `run_slice` e2e fixtures.
- Files allowed to edit (at most 3): `crates/slicer-config/src/ingestion.rs`; `crates/slicer-runtime/tests/e2e/resolved_config_view_no_drop_tdd.rs`; `crates/slicer-runtime/tests/e2e/main.rs`.
- Files explicitly out of bounds: `resources/cube_4color.3mf` contents, aliases, eligibility/layer ranges.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches: fixture member check `FACT` ≤5 lines; e2e run `FACT`.
- Context cost: `M`
- Authoritative docs: plan Ingestion staging and no-drop gate; test-quality oracle rules.
- OrcaSlicer refs: none.
- Verification: AC-5 e2e in retained mode first; then AC-5 and AC-6 after warn-to-drop.
- Exit condition: the fixture is skipped, the synthesized population is hand-listed instead of registry-derived, or any declared census key warns as unknown.

### Step 7: Update contracts and run closure gates

- Task IDs: `TASK-567`
- Objective: document resolved delivery and manifest metadata, then run all packet gates.
- Precondition: Steps 1–6 pass.
- Postcondition: AC-7 and all closure gates pass.
- Files allowed to read, with ranges when over 300 lines: only relevant sections of `docs/02_ir_schemas.md` and `docs/03_wit_and_manifest.md`.
- Files allowed to edit (at most 3): `docs/02_ir_schemas.md`; `docs/03_wit_and_manifest.md`.
- Files explicitly out of bounds: other docs, plan, packets, backlog except delegated completion update.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches: each cargo/doc gate returns `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs: approved plan and ADR-0067/0068.
- OrcaSlicer refs: none.
- Verification: AC-7; `cargo check --workspace --all-targets`; clippy; literals; test-quality report; guest freshness.
- Exit condition: any AC/gate fails or docs claim a WIT/IR version change.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| 1 | S | inventories only |
| 2 | M | public metadata/projection blast radius |
| 3 | M | production binding (3a) + contract tests/aggregator (3b) |
| 4 | M | 13 guests in ≤3-file substeps |
| 5 | M | emission byte change |
| 6 | M | real no-drop e2e |
| 7 | S | docs/gates |

## Packet Completion Gate

- All steps and exits complete; every AC command passes.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `cargo xtask build-guests --check` exits 0; all-target check/clippy and required quality gates pass.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record the intentional CONFIG_BLOCK behavior change and remaining packet-local risk.
- Confirm context stayed within the standard band or record the required swarm escalation/lesson.
