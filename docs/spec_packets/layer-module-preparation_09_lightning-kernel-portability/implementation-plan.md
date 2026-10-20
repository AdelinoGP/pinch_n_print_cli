# Implementation Plan: lightning kernel portability (Gate B)

## Execution Rules

Steps are future work for `TASK-576` Gate B, not actions taken during authoring. Commands are
non-interactive Bash. Execute one bounded step at a time. Packet 07's fixtures, the kernel
`model.rs`/`lib.rs`, the recorder, and the recorder patch are read-only. A scientific mismatch is
recorded, not repaired. Guest failures are attributed only after `build-guests --check` exits 0.

### Step 1 — Owned tree/planner

- Task: TASK-576. Objective: implement canonical top-down planning, grounding witnesses, and
  root-point-hash sampling.
- Precondition: packet 07 fixtures exist; `model.rs` defines `KernelInput`/`KernelPlan`.
- Postcondition: `plan_lightning` returns complete nodes/edges/roots/grounding/provenance and
  `sampled_segments` for object-layer pooled domains; no host producer, `rayon`, `boostvoronoi`, or
  `host-algos` dependency.
- Allowed reads: kernel model/API, packet 07 fixture samples, bounded current host lightning tree
  code for signatures only, canonical `generateTrees`/`convertToPolylines` via delegation.
- Allowed edits (3): `tree.rs`; `planner.rs`; Cargo-generated `Cargo.lock`.
- Forbidden: `model.rs`/`lib.rs`; production lightning; host driver; guest/WIT.
- Blast radius: isolated crate; standard attribution header on translated files; in-file
  `#[cfg(test)]` tests allowed. No public schema change.
- Dispatch: canonical tree/grounding question, `SUMMARY ≤200 words`; compile, `FACT`.
- Cost: M. Authorities: design, docs08/attribution/22.
- Canonical refs: `Generator::generateTrees`; `Layer::getBestGroundingLocation`/`attach`;
  `Node::convertToPolylines` (delegated).
- Verification: `cargo check --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml --all-targets`;
  `cargo test --manifest-path ...` for the in-file tests.
- Falsifying exit: host generator calls, empty placeholder graphs, index-paired islands, or a host
  dependency in the closure.

### Step 2 — Final clipping

- Task: TASK-576. Objective: implement `clip_lightning` against recorded final fills.
- Precondition: Step 1 emits sampled segments.
- Postcondition: each recorded dispatch clips to its final expolygon and returns a `ClippedFill`
  with dispatch token, caller identity, contributors, and two-point segments; missing/ambiguous
  dispatch identity is a named error.
- Allowed reads: kernel planner/model, packet 07 fixture samples.
- Allowed edits (2): `planner.rs`; Cargo-generated `Cargo.lock`.
- Forbidden: `model.rs` changes, production clipping, expected-data lookup.
- Blast radius: same crate; clipping is a pure function of plan + input.
- Dispatch: clipping semantics check, `SUMMARY ≤200 words`.
- Cost: S. Authorities: design; `Filler::_fill_surface_single` via delegation.
- Canonical refs: `FillLightning.cpp::Filler::_fill_surface_single` (`intersection_pl`).
- Verification: kernel `cargo test` for clipping cases (real polygon, hole, empty/partial).
- Falsifying exit: clipping to planning domains, geometry welding, or a dispatch-token fallback.

### Step 3 — Test guest adapter

- Task: TASK-576. Objective: compile the owned kernel through the existing native/WASM adapters.
- Precondition: the kernel compiles; the existing `slicer:layer-infill` world is authoritative.
- Postcondition: the guest implements `run_infill`, decodes `probe_input_json`/`probe_output`/
  `probe_final_dispatch`, emits tags 1/2/3 or clipped two-point segments, rejects Custom/None
  origins, and has a generated native entry. No new WIT.
- Allowed reads: `sdk-layer-infill-guest` manifest/source, owned kernel exports, native entry and
  infill world symbols, guest discovery/freshness sections.
- Allowed edits (3): guest `Cargo.toml`; guest `src/lib.rs`; guest `Cargo.lock`.
- Forbidden: canonical WIT, SDK/macros, production guest, custom export.
- Blast radius: test guest only; exhaustive production-style `Point3WithWidth`/`ExtrusionPath3D`
  literals; metadata representability/Z/positive-width checks.
- Dispatch: guest build/freshness, `FACT` artifact identity and exit.
- Cost: S. Authorities: docs03/05/22, AGENTS freshness.
- Canonical refs: recorded parameter/graph references only.
- Verification: `cargo xtask build-guests`; `cargo xtask build-guests --check` (exit 0 only is
  fresh; exit 1 stale/lock; exit 3 infrastructure stop).
- Falsifying exit: missed discovery, incompatible WIT, hidden host planner, or an expectation file
  entering guest linking.

### Step 4 — Host dev wiring

- Task: TASK-576. Objective: make the native guest/kernel and SHA-256 validation available to the
  host test binary.
- Precondition: Step 3 artifacts build.
- Postcondition: host dev-dependencies include the native guest/kernel path and `sha2`; SDK `test`
  support is explicitly enabled; all targets compile.
- Allowed reads: host dev-dependency block, guest manifest.
- Allowed edits (2): `crates/slicer-wasm-host/Cargo.toml` (dev-dependencies only); root
  `Cargo.lock` (generated).
- Forbidden: production dependencies/functions; guest/kernel source; SDK/WIT.
- Blast radius: dev graph only; native SDK may legitimately contain `host-algos`; the owned kernel
  and wasm32 guest closures must not.
- Dispatch: dependency audit, `FACT` same package/native entry and exclusions.
- Cost: S. Authorities: docs05, AGENTS all-target gate.
- Canonical refs: none.
- Verification: `cargo check -p slicer-wasm-host --all-targets`.
- Falsifying exit: a production dependency change or a host-algos edge into the kernel/guest.

### Step 5 — Driver and five tests

- Task: TASK-576. Objective: author the native/WASM driver, decoder, comparator, and the five exact
  tests.
- Precondition: fixtures, guest, and dev wiring exist.
- Postcondition: `main.rs` registers `mod lightning_portability_tdd;`; the test file includes its
  sibling via `#[path = "lightning_portability_support.rs"] mod support;`. Tests are
  `planned_graph_native_and_wasm`, `gate_b_decision_is_truthful`,
  `clipped_segments_and_contributors`, `rejects_canned_echo_and_fallback`,
  `outcome_records_gate_b`. Both adapters are compared independently to canonical.json; the
  recorded status equals the recomputed comparison; clipped dispatches with unavailable
  contributor evidence are `inconclusive`, never skipped.
- Allowed reads: fixtures, owned API, native request/entry/output builder, host origin vectors,
  common guest cache, current infill world.
- Allowed edits (3): `main.rs`; `lightning_portability_tdd.rs`;
  `lightning_portability_support.rs`.
- Forbidden: fixture rebaselines, production host/paint/module changes, WIT/SDK changes, a sixth
  test.
- Blast radius: new tests only; exhaustive native/WIT transport literals with the documented
  waiver where no base exists; no digest-only or adapter-equality-only assertion.
- Dispatch: each Cargo command, `FACT` exit/test count; canonical difference audit,
  `SUMMARY ≤200 words`.
- Cost: M. Authorities: docs03/05/08/21/22, AGENTS test/freshness.
- Canonical refs: independent recorded graph/grounding/clip functions via delegation.
- Verification: the five exact AC commands; then the five-test aggregate with the fail-closed
  prefix.
- Falsifying exit: zero tests, skipped population, native/WASM equality-only assertion, or a
  canned/empty fallback counted as a pass.

### Step 6 — B outcome and documentation

- Task: TASK-576. Objective: write the validated B outcome and record it truthfully.
- Precondition: the five tests execute and produce per-case/per-dispatch statuses.
- Postcondition: `target/lightning-portability/outcome.json` carries gate B, executed/freshness
  identities, kernel/guest hashes, native/WASM counts, per-case planned/clipped statuses,
  commands/exits, and blockers; the plan gains the B record; `TASK-576` gains
  `  - TASK-576 Gate B outcome: (pass|fail|inconclusive)`; `migration_eligible` is the conjunction
  of A pass, B pass, and independently recorded framework acceptance, with the unverified
  framework term named as a blocker when absent.
- Allowed reads: outcome JSON, plan section window, `TASK-576` block, recorded A outcome.
- Allowed edits (3): `lightning_portability_support.rs` (writer); plan (append-only);
  `docs/07_implementation_status.md` (`TASK-576` only).
- Forbidden: rewriting approved plan prose, other backlog entries, packet 07, production code.
- Blast radius: evidence-local; re-derive revision/hash facts at execution.
- Dispatch: evidence review, `FACT` statuses with named blockers.
- Cost: S. Authorities: plan row-07 rule, ADR0066, docs22.
- Canonical refs: recorded revision/patch identity.
- Verification: AC-5; placement greps; `cargo xtask check-literals`;
  `cargo xtask check-test-quality --report`.
- Falsifying exit: a recorded `pass` from a failed comparison, `[x]` without both gates, or a
  missing named blocker.

## Per-Step Budget Roll-Up

| Step | Cost | Reason |
| --- | --- | --- |
| 1 | M | Tree/grounding/sampling port |
| 2 | S | Final clipping |
| 3 | S | Existing-stage guest adapter |
| 4 | S | Dev-dependency wiring |
| 5 | M | Driver, decoder, five tests |
| 6 | S | Truthful outcome and docs |

Aggregate M; no L step. Stop and re-delegate before a step exceeds its budget.

## Packet Completion Gate

Re-dispatch every exact AC command, `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask build-guests --check`,
`cargo xtask check-literals`, and `cargo xtask check-test-quality --report`. Validate the B record
independently, archive log hashes, and record the truthful B status. Completion does not require B
`pass`; a red scientific comparison is reported as failed/inconclusive feasibility with migration
blocked.
