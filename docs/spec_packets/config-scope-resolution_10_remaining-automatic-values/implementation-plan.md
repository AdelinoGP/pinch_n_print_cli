# Implementation Plan: remaining-automatic-values

## Execution Rules

Every step maps to TASK-571. Use bounded read/edit slices, preserve unrelated working-tree content, and write falsifying tests before the implementation repair. No checkout/stash/reset/revert, fixture regeneration, artifact deletion, full-workspace test or commit. The earlier 2026-09-30 closure annotations below are historical and superseded by the subsequent cold review's `CHANGES REQUESTED`. The packet was reopened while remediation, fresh evidence and full review ran; the final approved closure is recorded in `packet.spec.md` and `review-remediation.md`. No commit is authorized.

All test runs are sequential: plain narrow commands require pipefail and combined-output tee to `target/test-output.log`; guest-touching commands use `cargo xtask test --summary` which writes the log itself. Require exact passed markers, inspect and archive each log under `target/packet10-remediation/` before overwrite, and report FACT outside thinking blocks. Check/clippy use `--all-targets`; a runtime test selecting `--test unit` excludes harness-free Criterion benches.

## Step 1: Reconcile dependencies and census (TASK-571)

- Objective: verify landed packets 04/05, source-derived negative candidates, preserved fixture provenance and canonical Phase-C formula.
- Precondition: revised conversation choices recorded; previous evidence not accepted as current PASS.
- Postcondition: exact exports/status and global/tool source identified; each scalar/array/lower-bound negative candidate has an owner or a precise blocker.
- Read: packet 04/05 exports/status; `crates/slicer-runtime/src/run.rs` config resolution/handoff; `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` registry/census; pre-change fixture provenance, bounded only.
- Edit: none.
- Out of bounds: approved plan/other packet edits; direct Orca reads; fixture bodies/regeneration.
- Dispatch: dependency/census FACT, and delegated `GCode.cpp::GCode::_extrude` SUMMARY at most 200 words. Do not freeze ledger facts such as a SHA/count into later instructions.
- Authoritative docs: plan Phase C and docs 02 config/interner, docs 22 test quality.
- Context cost: S.
- Verification: source reconciliation and independently derived owner report; no claimed test pass.
- Exit: dependency/canonical facts decisive and no unnamed owner; otherwise stop.

## Step 1B: Record Q8's narrow mechanism amendment (TASK-571)

- Objective: repair confirmed `PREFLIGHT BLOCKED` S8 without silently rewriting ADR-0052 or claiming conformance to its unchanged-body/direct-call clauses.
- Precondition: Q8 "Amend ADR" authorized in the current user conversation 2026-09-30; inspect actual normative clauses and private resolver delegation/calls. Re-derive the next free ADR slot and deviation-ID availability from the live directory/registry at creation, never assume a reservation.
- Postcondition: accepted separate ADR-0072 (`docs/adr/0072-context-aware-feedrate-resolution-preserves-factor-contract.md`) quotes both contested clauses and the direct-call expression; `D-CSR10-ADR-0052-AMENDED` is registered; ADR-0052 packet append points to the narrow superseder. Public signature and factor carriers/replacement/fallback/profile-length/mutation constraints remain; original sections and 2026-08-05 amendment are untouched.
- Read: `.agents/skills/spec-review/references/preflight-gate.md` S8, live ADR directory/registry, ADR-0052 Decision/Consequences/append, `crates/slicer-gcode/src/emit.rs` resolver helpers/production call, domain-modeling ADR format.
- Edit (three): the exact new ADR-0072 path, `docs/DEVIATION_LOG.md` one named row, `docs/adr/0052-per-point-speed-factor-contract.md` packet append only.
- Out of bounds: code, locks/generated docs, other ADRs/registry rows, original ADR-0052 normative text, WIT/IR layout/public factor interface; no cargo/build/test/rustfmt for this documentation repair (central validation has the sole cargo owner).
- Dispatch: bounded docs FACT outside thinking; authorization is architectural acceptance, not a test/preflight PASS.
- Authoritative docs: ADR-0052 and new ADR-0072; S8; live deviation registry.
- Context cost: S.
- Verification: packet Doc Impact greps check quotes/references only; inspect bounded diff for retained original text. Fresh independent preflight must repeat S2/S4/S8 and remaining checks.
- Exit: amendment decision and actual deviation recorded; Q8 resolved. (Closed 2026-09-30: repeated preflight returned `PREFLIGHT PASS`; fresh validation and the full closure review returned `APPROVED WITH NOTES`.)

## Step 2A: Restore unchanged persisted shape (TASK-571)

- Objective: retire only the packet's experimental fixed-field migration surface, preserving unrelated schema history.
- Precondition: Step 1 verified the independent pre-change 3.0.0 fixture and identified packet-added changes.
- Postcondition: RegionMapIR uses pre-change 3.0.0 shape/version chain, packet-added legacy decoder removed, Postcard dev-only.
- Read: `crates/slicer-ir/src/slice_ir.rs` RegionMapIR/version/packet decoder ranges; `crates/slicer-ir/Cargo.toml`; `crates/slicer-ir/src/resolved_config.rs` field/equality/hash ranges and git diff.
- Edit (three files): those same three paths; remove only packet-added fixed field and its fallout in resolved_config, never revert entire files.
- Out of bounds: fixtures, experimental artifacts, prior schema behavior, unrelated dependencies, manual lockfile edits.
- Dispatch: worker FACT reporting exact retired symbols and preserved layout; dependency-diff audit FACT for generated locks when Cargo regenerates them.
- Authoritative docs: docs 02 RegionMapIR/interner/versioning; docs 21 fixture policy.
- Context cost: M.
- Verification: `cargo check -p slicer-ir --all-targets`; unchanged-layout regression is Step 2B, not assumed green here.
- Exit: no packet-only fixed field, migration API, or production Postcard promotion; 3.0.0 layout ready for independent decode proof. Artifacts/fixtures unchanged.

## Step 2B: Declare extension/accessor and prove layout/identity (TASK-571)

- Objective: use existing validated extension transport with typed access and independent preserved-fixture evidence.
- Precondition: Step 2A removed experimental layout; Step 1 established normal resolution/default source.
- Postcondition: `HOST_RUNTIME_KEYS` has Float key/default `Some("0.0")`, `SCOPE_FILAMENT`, `meta.min = Some(0.0)`, `TOOL_CAPABLE_SCOPES` denial; accessor returns f64 Result, absent numeric zero, Float/Int/finite numeric String and existing first-element numeric List envelope accepted, wrong type/empty List/non-finite/negative rejected. This envelope is not a tool-index array.
- Read: `crates/slicer-ir/src/resolved_config.rs` runtime declarations/map/accessors/scope tests; `crates/slicer-ir/tests/region_map_versioned_decode_tdd.rs`; `crates/slicer-config/tests/scope_eligibility_tdd.rs` denial roster.
- Edit (three files): those same paths. Preserve fixture files; no new macro field or `typed_field_keys` entry.
- Out of bounds: emitter/runtime resolution, WIT, fixture bytes/provenance/expected values, production decoder.
- Dispatch: worker FACT for direct dev-Postcard legacy deserialize, current config/paint extension round-trip, identity and accessor cases; reconcile actual new test names only after authoring.
- Authoritative docs: docs 02 extension/interner; docs 22 independent oracle/non-vacuity.
- Context cost: M.
- Verification: full `region_map_versioned_decode_tdd` file and `scope_eligibility_tdd` file (requirements commands); require non-empty results and inspect assertions, including configs plus paint and distinct interning. No unsupported-version API promise.
- Exit: independent fixture stays schema 3.0.0 with full expected contents; current extension content/identity preserved; accessor and denial behavior falsifiable.

## Step 2C: Census and mirror lock (TASK-571)

- Objective: reconcile declaration metadata/census and repair the prior red mirror-default lock through the accessor.
- Precondition: Step 2B declaration/accessor available; retained host-keys mirror identified.
- Postcondition: census derives scalar/array/default/lower-bound candidates, detects a negative-array control and unknown owner, reconciles minimum; mirror default reads via typed accessor, not fixed field.
- Read: `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` census; `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` numeric lookup/`resolved_config_keys_match_default`; `docs/config/host-keys.toml` key/default mirror.
- Edit (three files): those same paths, host-key mirror retained.
- Out of bounds: packet-05 resolution, unrelated doc-lock arms, generated doc 15 hand edits.
- Dispatch: census and lock FACT with exact markers; never quote the old lock PASS.
- Authoritative docs: docs 02 registry/default semantics; docs 22 roster rules.
- Context cost: M.
- Verification: exact census command, AC-8 real `resolve_scope_stack` regression rows, and gated `cargo xtask test --summary -p slicer-runtime --test unit host_keys_doc_lock_tdd`; require `resolved_config_keys_match_default` and `filament_volumetric_doc_lock_reads_effective_extension_value` markers (absent zero versus stored 8). Do not add `--all-targets`: that selects the harness-free `gate_evidence` Criterion bench, which rejects xtask's libtest-only `--skip` arguments. All-target check/clippy gates still compile that bench.
- Exit: metadata/census/mirror default coherent; no unowned declared Phase-C candidate. Guest freshness enforced by gated runtime test.

## Step 2C2: Reconcile the declaration roster (TASK-571)

- Objective: account for the related registry test omitted from the original edit list.
- Precondition: the declared extension row is present and Step 2C has identified the existing roster pin.
- Postcondition: the roster includes the new key; its comment distinguishes the original synthesized rows from this row's `Some("0.0")` default.
- Read/edit (one): `crates/slicer-config/tests/registry_census_tdd.rs`; read the `HOST_RUNTIME_KEYS` row in `crates/slicer-ir/src/resolved_config.rs` without editing it.
- Out of bounds: other registry/resolution behavior, fixtures and unrelated roster changes.
- Dispatch: bounded test/comment worker; sole validation owner runs the full census test file and archives its result.
- Authoritative docs: docs 22 test quality and the declaration/default contract in docs 02.
- Context cost: S.
- Verification: full `registry_census_tdd` command in requirements; retain the fresh log before overwrite.
- Exit: scoped roster follow-through is documented, its default comment is accurate, and the full file passes.

## Step 3: Automatic base and final-F safety (TASK-571)

- Objective: consume typed extension limit at the real move site and fail closed after final conversion as well as before division.
- Precondition: resolved key/accessor available, canonical formula verified, ADR-0052 factor contract and Step 1B's accepted ADR-0072 mechanism amendment read.
- Postcondition: literal tool/global/explicit controls hold; automatic non-finite factors, overflow and round-to-zero error without unsafe returned F; explicit positive speeds intentionally uncapped.
- Read: `crates/slicer-gcode/src/emit.rs` move/context/factor conversion; `crates/slicer-gcode/src/error.rs` error shape; `crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs` real output controls.
- Edit (two files): emit source and volumetric test file.
- Out of bounds: public absolute point-speed interfaces, packet-04 expansion, packet-05 merges, unrelated serialization.
- Dispatch: worker and test FACT; full test-file coverage for new final-F rows, no speculative exact names.
- Authoritative docs: ADR-0052 Decision as narrowly amended by ADR-0072; docs 02 Phase C; docs 22 literals/negative controls.
- Context cost: M.
- Verification: AC-1/2/7/N1 and full-file AC-N2, plus `cargo check -p slicer-gcode --all-targets`.
- Exit: removing zero branch/tool/geometry/final-F checks breaks a real-output assertion; finite-factor normal clamp retained and no public factor contract change.

## Step 4A: Carry resolved tools into model visual-debug (TASK-571)

- Objective: make model GCodeEmit use request speed settings and the normal runtime's resolved tool map, not defaults or duplicate resolution.
- Precondition: Step 3 emitter passes; request/config pair available; runtime map ownership reconciled.
- Postcondition: `PrepassContext` retains tool configs; model emitter receives them with parsed FeedrateConfig; inline actual-F tests distinguish two tools and default control.
- Read: `crates/slicer-runtime/src/run.rs` `PrepassContext`/`prepare_prepass_context`; `crates/pnp-cli/src/visual_debug.rs` model config/emitter and inline tests; request fixture paths only.
- Edit (two files): `crates/slicer-runtime/src/run.rs`, `crates/pnp-cli/src/visual_debug.rs`. This runtime expansion is user-authorized.
- Out of bounds: scope engine rewrite, visual renderer/UI/schema/manifest, unrelated runtime construction.
- Dispatch: tool-handoff worker FACT and gated target-neutral `visual_debug_volumetric_auto` prefix test FACT, with both actual-output controls inspected.
- Authoritative docs: docs 19 model/tap/bundle, docs 22 real-path oracle, docs 02 resolution.
- Context cost: M.
- Verification: AC-4's gated prefix tests plus bundle command; marker must name the shared prefix, no assumed lib/bin target.
- Exit: replacing resolved tools with defaults, discarding map or ignoring request speed must break emitted-output assertions; bundle still valid.

## Step 4B: Retain bounded visual fixtures (TASK-571)

- Objective: keep the request/config pair usable without expanding visual schema.
- Precondition: Step 4A model path wired; inspect existing fixtures before any edit.
- Postcondition: request references a global-8/tool-0-12 companion; inline multi-tool and actual CLI controls independently distinguish dropped tool configs.
- Read: request/config pair and docs 19 relevant request section.
- Edit (at most three): `crates/pnp-cli/tests/fixtures/config_scope_resolution_10/visual-debug.json`, its `visual-debug-config.json`, and new direct Cargo target `crates/pnp-cli/tests/visual_debug_volumetric_auto_tdd.rs`; preserve unrelated fixtures.
- Out of bounds: all other fixtures, experimental artifacts, renderer and CLI schema.
- Dispatch: bundle FACT with image non-empty and warning/executed-stage assertions, no timing metrics.
- Authoritative docs: docs 19 deterministic bundle.
- Context cost: S.
- Verification: AC-4 bundle command and named runtime CLI regression; the same model's outer-wall emitted feedrates must change by the independent 12/8 ratio when tool 0's override is present. Require non-empty matched extrusion vectors.
- Exit: model bundle valid with actual emitter evidence already proved, not manifest-only automatic-speed claim.

## Step 5A: Canonical docs and glossary (TASK-571)

- Objective: distinguish declared extension content from persisted shape and private base from public absolute point speeds.
- Precondition: revised user choices fixed; inspected code semantics inform docs, but no unrun check marked PASS.
- Postcondition: doc 02 restores 3.0.0/F-19 chain and explains extensions/role-zero/final F with the necessary ADR-0072 cross-reference; glossary has tight resolved terms. Step 1B separately owns the mechanism decision/registry/ADR-0052 append.
- Read: CONTEXT glossary neighborhoods; docs 02 extension/Phase C/RegionMapIR; ADR-0052 Decision/append and ADR-0072.
- Edit (two): `CONTEXT.md`, `docs/02_ir_schemas.md`. The S8 documentation repair may only add the necessary doc-02 cross-reference, not edit CONTEXT or broaden normative text.
- Out of bounds: prior ADR content, broad versioning normative rewrites, future host-key blanket rules.
- Dispatch: docs FACT and fresh S8 check against the explicitly registered narrow amendment; no conformance-only claim or unregistered normative departure.
- Authoritative docs: domain-modeling CONTEXT format; docs 02/ADR-0052/ADR-0072/docs 22.
- Context cost: S.
- Verification: bounded diff review and AC-5 documentation checks; no behavioral claim from grep alone.
- Exit: chosen map/layout/artifact/factor story consistent; glossary has definitions only.

## Step 5A2: Align packet authority and scope (TASK-571)

- Objective: remove the superseded conformance-only claim and trace Q8's actual amendment without altering AC commands.
- Precondition: Step 1B decision/registry/append exist; read current packet files and preserve unrelated changes.
- Postcondition: re-scope/deviations/doc-impact and design/requirements scopes name ADR-0072 and `D-CSR10-ADR-0052-AMENDED`, retain all other ADR constraints. (Closed 2026-09-30; historical draft/pending wording retained below for the audit trail.)
- Read/edit (three): this packet's `packet.spec.md`, `design.md`, `requirements.md` only.
- Out of bounds: approved plan/other packets; code/gates/locks/generated docs; existing test-marker ACs, including AC-4's three-key companion and CLI 12/8 regression.
- Dispatch: docs FACT outside thinking; no cargo commands by this documentation worker.
- Authoritative docs: ADR-0052, ADR-0072 and S8.
- Context cost: S.
- Verification: Doc Impact text/reference greps and bounded diff; these do not prove behavior or preflight success.
- Exit: Q8 resolved; repeat preflight returned `PREFLIGHT PASS`; fresh validation and the full closure review subsequently returned `APPROVED WITH NOTES` (2026-09-30).

## Step 5A3: Align amendment execution and task trace (TASK-571)

- Objective: map the amendment and five-file packet repair into bounded slices.
- Precondition: Steps 1B/5A2 recorded; retain existing validation commands and pending evidence.
- Postcondition: plan/task map trace the actual new ADR and deviation; all edit slices stay at most three files. (Closed 2026-09-30: packet `implemented`; TASK-571 checked.)
- Read/edit (two): this packet's `implementation-plan.md`, `task-map.md` only.
- Out of bounds: code/gates/locks/generated docs, approved plan, other packets/ADRs; no cargo/build/test/rustfmt.
- Dispatch: bounded SUMMARY outside thinking, separate from central validation owner.
- Authoritative docs: ADR-0072, deviation registry and S8.
- Context cost: S.
- Verification: bounded diff and documentation greps only. (Closed 2026-09-30: repeated preflight returned `PREFLIGHT PASS`; validation and full review completed.)
- Exit: amendment trace coherent without any new gate/PASS/closure claim.

## Step 5B: Generated docs and conditional closure (TASK-571)

- Objective: regenerate normally, validate fresh tree, record only actual gates, prepare formal closure without commit.
- Precondition: fresh preflight S0–S8 passes before activation; narrower verification commands all run on revised implementation.
- Postcondition: generated docs current, guest freshness/convergence decisive, all-target compilation/lint gates, focused tests and fresh full review recorded; status/evidence changes only after gates.
- Read: host-key mirror, TASK-571 row/evidence, five packet files; gate logs bounded after each sequential run.
- Edit (bounded slices): generated `docs/15_config_keys_reference.md` plus TASK-571 evidence in `docs/07_implementation_status.md`; a separate at-most-three-file packet slice updates actual results/status, then remaining packet files if needed. Do not change queue generation status or approved plan.
- Out of bounds: TASK-302 except a separately verified stale claim, unrelated backlog, manual generated edits, commits/full-workspace tests.
- Dispatch: one bounded FACT per generator/freshness/check/clippy/literals/test-quality gate, then fresh full reviewer. Run guest-touching tests via sequential xtask test --summary; record exact exit code and test markers.
- Authoritative docs: docs 11 compatibility; docs 22 test quality; freshness rules in AGENTS.md.
- Context cost: M.
- Verification: every AC, full touched files, `cargo xtask gen-config-docs --check`, `cargo xtask build-guests --check`, all-target workspace check/clippy, literals and test-quality report with findings resolved/waived where justified.
- Exit: fresh preflight and tests/gates/full review pass before formal closure; otherwise remain draft/open with exact blocker. No commit. (Closed 2026-09-30: all gates passed; full review `APPROVED WITH NOTES`; packet `implemented`, TASK-571 checked.)

## Cold-review remediation (TASK-571)

The latest user instruction, "Fix all issues", authorizes the bounded repairs below, not a schema/API change or a new fixture. Existing artifacts must not be deleted or regenerated. That prohibition does not assert that a 3.1.0 artifact was ever produced: identify extant files, preserve them, and withdraw unsupported existence claims rather than manufacture a fixture.

### Step 6A: Discriminating regression controls

- Objective: make an accidental explicit-speed cap observable and name the actual narrowing failure accurately.
- Precondition: the cold review identified the below-ceiling explicit control and mislabeled finite-product case.
- Postcondition: explicit 30 mm/s with positive limit 1 mm³/s still asserts independent F1800; the derived-speed underflow case asserts its specific diagnostic. The census default comment is accurate.
- Read/edit (two): `crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs`, `crates/slicer-config/tests/registry_census_tdd.rs`.
- Out of bounds: production behavior, fixture generation and unrelated assertions.
- Dispatch: bounded test worker; sole command owner runs both full test files.
- Authoritative docs: docs 21 fixtures and docs 22 falsifiability.
- Context cost: S.
- Verification: full volumetric and census suites with fresh archived receipts.
- Exit: an erroneous positive-limit cap would change F1800 to F750 and fail the control; narrowing underflow is not mislabeled as f64 product overflow.

### Step 6B: Align accessor and scoped surface documentation

- Objective: document existing numeric-string coercion and the related census/evidence files.
- Precondition: inspect the actual accessor and its existing scalar-envelope semantics.
- Postcondition: accessor shapes and rejection policy match code; census and auxiliary evidence paths are in scope; focused runtime tests exclude Criterion benches while check/clippy retain all-target compilation.
- Read/edit (three): this packet's `design.md`, `requirements.md`, and `docs/02_ir_schemas.md`.
- Out of bounds: resolver behavior, ADR decisions and generated doc 15.
- Dispatch: bounded documentation worker; fresh independent symbol/doc checks.
- Authoritative docs: docs 02, ADR-0052/0072 and docs 22.
- Context cost: S.
- Verification: doc-impact greps and bounded diff; text does not prove test execution.
- Exit: no scalar-shape ambiguity or unlisted census edit remains.

### Step 6C: Correct commands, task trace and historical status

- Objective: reopen unsupported closure and require durable current-tree evidence.
- Precondition: latest cold review returned `CHANGES REQUESTED`.
- Postcondition: AC-2 includes the above-ceiling control; AC-6 requires all named fixture/roundtrip/interning markers and archives the log; Step 2C's runtime selector is corrected; task-map Step 4B includes the actual CLI regression. Historical closures are explicitly superseded, not asserted as current evidence.
- Read/edit (three): this packet's `packet.spec.md`, `implementation-plan.md`, and `task-map.md`.
- Out of bounds: approved plan, other packets, fixtures and commits.
- Dispatch: coordinator documentation slice; sole validation owner runs every AC and gate.
- Authoritative docs: review skill, docs 22 and runtime test-target wiring.
- Context cost: S.
- Verification: every current AC command, full touched suites, workspace build/check/clippy, literals, quality report, generated-doc and guest-freshness gates.
- Exit: all commands have fresh receipts with named evidence; no historical receipt substitutes for a current run.

### Step 6D: Inventory, receipts and conditional reclosure

- Objective: record only measured preservation and validation facts, then obtain a fresh full review.
- Precondition: Steps 6A–6C completed; independently identify extant fixture/archive files before claiming their retention.
- Postcondition: `review-remediation.md` records artifact roles, baseline/end hashes, bounded search results, exact commands and fresh result lines. TASK-571 reflects the latest review, not the superseded closure.
- Read/edit (two): this packet's `review-remediation.md` and TASK-571 in `docs/07_implementation_status.md`; read the fixture provenance and existing artifact inventory without modifying them.
- Out of bounds: artifact/fixture deletion or regeneration, unsupported existence/absence claims and unrelated backlog rows.
- Dispatch: read-only artifact search, sole command owner, then one sequential holistic full reviewer after workers and validation finish.
- Authoritative docs: docs 21/22, review burden-of-proof rules and packet non-deletion constraint.
- Context cost: M.
- Verification: re-hash every identified artifact against its baseline; review current archived test markers and gate exit codes.
- Exit: full review passes before packet status/TASK-571 completion changes. Otherwise retain draft/open with the exact blocker; no commit.

### Step 6E: Remove duplicated mutable closure state

- Objective: clear the fresh preflight's stale ADR/deviation closure references without changing the accepted mechanism decision.
- Precondition: fresh preflight passes S0–S8/static AC checks but identifies closure-state copies inconsistent with the reopened packet.
- Postcondition: ADRs direct readers to the packet/backlog/evidence instead of duplicating mutable packet status; the single named deviation row follows review-ordered closure, never closing before fresh full approval. AC-4 archives both test and CLI output under the remediation receipt root through Step 6C.
- Read/edit (three): `docs/adr/0052-per-point-speed-factor-contract.md` (packet append only), `docs/adr/0072-context-aware-feedrate-resolution-preserves-factor-contract.md` (status/verification bookkeeping only), and the `D-CSR10-ADR-0052-AMENDED` row in `docs/DEVIATION_LOG.md`.
- Out of bounds: original ADR-0052 text, ADR-0072's normative Decision, other ADRs/registry rows and source code.
- Dispatch: coordinator bounded documentation slice; preflight worker rechecks only the reported closure/receipt blockers.
- Authoritative docs: ADR-0052/0072 and review preflight gate.
- Context cost: S.
- Verification: bounded diff, preserved normative clauses, corrected AC-4 wrapper and fresh preflight tail receipt.
- Exit: no accepted architectural decision is misrepresented as current packet closure; preflight blockers clear without relaxing assertions.

### Step 6F: Resolve full-review comment notes

- Objective: correct both optional clarity notes from the full `APPROVED WITH NOTES` review without changing executable code or assertions.
- Precondition: the full review reports no blockers and identifies only the unavailable-limit and flow-magnitude comments.
- Postcondition: the accessor documents an error for automatic derivation without a positive limit; flow 2 is described as doubling volume per distance and halving speed.
- Read/edit (two): `ResolvedConfig::filament_max_volumetric_speed`'s doc comment in `crates/slicer-ir/src/resolved_config.rs` and the flow-2 comment in `zero_role_speed_auto_base_tracks_width_flow_factor_clamp_and_layer_height` in `crates/slicer-gcode/tests/volumetric_auto_speed_tdd.rs`.
- Out of bounds: Rust statements, signatures, assertions, fixtures and unrelated comments.
- Dispatch: coordinator comment-only patch; independent reviewer verifies old-comment restoration matches the validated source hashes; sole command owner validates freshness and final touched gates.
- Authoritative docs: docs 02 Phase C and the full review's optional notes.
- Context cost: S.
- Verification: comment-only hash witness; decisive guest freshness with normal rebuild if needed; all-target workspace clippy, focused runtime doc-lock and full volumetric suite.
- Exit: both notes resolved; full approval retained; final current-tree checks pass before closure bookkeeping.

## Budget Roll-Up

| Step | Cost | Slice |
| --- | --- | --- |
| 1 | S | Dependency/census grounding |
| 1B | S | Q8 separate ADR, deviation row and packet-only supersession pointer |
| 2A | M | Experimental layout retirement |
| 2B | M | Declaration/accessor/layout regression |
| 2C | M | Census and accessor doc lock |
| 2C2 | S | Related declaration roster follow-through |
| 3 | M | Real emitter/final F |
| 4A | M | Runtime/model tool handoff |
| 4B | S | Bounded fixtures |
| 5A | S | Canonical docs and narrow mechanism cross-reference |
| 5A2 | S | Three packet authority/scope files |
| 5A3 | S | Two packet execution/trace files |
| 5B | M | Sequential gates and conditional closure |
| 6A–6C | S each | Regression controls and bounded documentation repairs |
| 6D | M | Measured inventory, durable receipts and full review |
| 6E | S | Mutable closure bookkeeping; original ADR decisions preserved |
| 6F | S | Comment-only full-review polish with current-tree revalidation |

Aggregate M; no L slice. Repeated fallout/doc slices edit at most three files and require explicit inherited pre/post/exit contracts.

## Implementation Evidence

Historical implementation repaired the mirror accessor lock, a clippy-rejected dead `mut`, the host declaration roster pin, and the AC-4 entity-indexed `tool_changes`. An earlier review deferred on budget, then reported closure after follow-up work. Those receipts under `target/packet10-validation/` and the earlier closure are historical: the subsequent cold review returned `CHANGES REQUESTED` and reopened the packet. Do not freeze a hand-count of doc-impact greps or reuse an older two-test IR receipt for the current interning test. Step 6 and `review-remediation.md` own the current finding disposition, preservation facts and validation receipts under `target/packet10-remediation/`. Fresh full review supports remediation closure; both optional comment notes and final current-tree checks are complete. Status is authoritative in the packet contract and backlog, not these historical annotations.

Q8's architectural amendment remains accepted in ADR-0072 and registered as `D-CSR10-ADR-0052-AMENDED`; the cold-review repair does not change that decision. Central validation owns cargo exclusively; documentation workers run no cargo/build/test/rustfmt commands. Current closure evidence is recorded only after those commands and the full review return.
