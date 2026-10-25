# Implementation Plan: lightning kernel portability (Gate B)

## Execution Rules

Steps are future work for `TASK-576` Gate B, not actions taken during authoring. Commands are
non-interactive Bash. Execute one bounded step at a time. Packet 07's fixtures, the four included
kernel files, the module's `mod.rs`, `model.rs`/`lib.rs`, and the recorder are read-only. A
scientific mismatch is recorded, not repaired. Guest failures are attributed only after
`build-guests --check` exits 0. The four included kernel files are byte-identical inputs: if a step
needs a change inside one of them, the step is wrong and the packet must be re-scoped.

### Step 1 — Kernel source inclusion and scalar derivation

- Task: TASK-576. Objective: compile the module-owned kernel source inside the isolated crate and
  prove its recorded-parameter scalar derivation.
- Precondition: packet 07 fixtures exist; the module kernel files exist; the feasibility crate
  compiles.
- Postcondition: `feasibility-kernel/src/kernel/mod.rs` includes the four module files by `#[path]`
  with zero edits to them; `lib.rs` declares `pub mod kernel;` and its crate comment names the
  `slicer-core` exception instead of claiming crate independence from it; the crate depends on
  `slicer-core` with default features and contains no
  `host-algos`/`rayon`/`boostvoronoi`/`slicer-runtime` edge; an in-file test constructs a
  `Generator` from the recorded parameter set and asserts the four derived scalars equal
  `3.0/0.2/0.017497/0.017497` mm in PnP units, asserts a zero-density set yields a zero supporting
  radius with `supporting_radius_mm = None`, and asserts a mismatched option/density pair is a named
  error.
- Allowed reads: the four included files (imports, `Generator::new`, field visibility), the
  feasibility `Cargo.toml`, `docs/08_coordinate_system.md`.
- Allowed edits (4): `feasibility-kernel/Cargo.toml`; `feasibility-kernel/src/lib.rs` (`pub mod
  kernel;` plus the crate-comment `slicer-core` exception); `feasibility-kernel/src/kernel/mod.rs`
  (new); `feasibility-kernel/Cargo.lock` (generated).
- Forbidden: editing or copying the included files; `model.rs` changes; production module edits;
  WIT/SDK changes; `host-algos` in the kernel closure.
- Blast radius: isolated crate; no public schema change; the include module is the only new file in
  the kernel tree.
- Dispatch: dependency/closure audit, `FACT`; include-path check, `FACT`.
- Cost: S. Authorities: design; docs08; AGENTS freshness.
- Canonical refs: `Generator::new` scalar derivation (`Generator.cpp::Generator`).
- Verification: `cargo check --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml --all-targets`;
  `cargo test --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml derived_scalars_match_canonical_tdd`
  (exactly one test passes).
- Falsifying exit: a copied kernel file, an edit inside an included file, a host/`host-algos`
  dependency in the closure, or any derived scalar that differs from the recorded value.

### Step 2 — Graph extraction adapter

- Task: TASK-576. Objective: turn the included `Generator`'s tree graph into complete `GraphLayer`
  records.
- Precondition: Step 1 compiles and `Generator::get_trees_for_layer`/`Layer::tree_roots`/`Node`
  accessors are reachable.
- Postcondition: `tree.rs` walks the tree deterministically and emits nodes (stable ids, point,
  `parent_id`, `root_id`, `origin_region_index`, grounding), edges (one per occurrence), isolated
  roots, and `sampled_segments`; every grounding point resolves to a `BoundaryAddress` in the layer's
  `GraphProvenance`, and an unresolvable point is `KernelError::SourceProvenanceUnavailable`; a real
  multi-layer synthetic outline run produces a nonempty graph with at least one root and one
  non-root; coincident-but-distinct nodes keep distinct ids.
- Allowed reads: included `tree_node.rs`/`layer.rs`/`generator.rs` public methods, `model.rs` graph
  types, `boundary_provenance` shape.
- Allowed edits (2): `feasibility-kernel/src/tree.rs`; `feasibility-kernel/Cargo.lock` (generated).
- Forbidden: `model.rs` changes; reading private `Node` fields; fabricating addresses or ids;
  geometry dedup.
- Blast radius: isolated crate; pure extraction over an in-memory tree.
- Dispatch: graph/grounding traversal audit, `SUMMARY ≤200 words`.
- Cost: M. Authorities: design; `Layer::getBestGroundingLocation`/`attach`; `TreeNode.cpp`.
- Canonical refs: `Layer::attach`, `Node::convertToPolylines`.
- Verification: `cargo test --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml tree_extraction_tdd`
  (exactly one test passes).
- Falsifying exit: index-paired islands, ids derived from coordinates, a grounding witness without a
  resolved boundary, or an edge emitted per unique geometry instead of per occurrence.

### Step 3 — Plan and clip surface

- Task: TASK-576. Objective: implement `plan_lightning` and `clip_lightning` on canonical inputs.
- Precondition: Step 2 emits complete graph layers.
- Postcondition: `plan_lightning` groups `planning_layers` domains by resolved region and polygon
  ordinal, builds the per-global-layer outline vector (empty for unrecorded layers), constructs one
  `Generator` per group from the recorded parameters, and returns a `KernelPlan` whose layers carry
  the input provenance unchanged; an unresolvable domain→region or a parameter/option disagreement
  is a named error. `clip_lightning` clips planned polylines to constructed `FinalFillInput` geometries
  with `clip_polylines`, emits oriented two-point segments with holes preserved, and rejects a
  missing/duplicate/ambiguous dispatch token as `KernelError::InvalidInput`.
- Allowed reads: `model.rs` (`KernelInput`, `PlanningLayer`, `LayerInput`, `RegionInput`,
  `FinalFillInput`, `ClippedFill`), included `generator.rs` constructor/committed-segment methods,
  `slicer_core::polygon_ops::clip_polylines`.
- Allowed edits (2): `feasibility-kernel/src/planner.rs`; `feasibility-kernel/Cargo.lock` (generated).
- Forbidden: `model.rs` changes; clipping to planning domains; welding/reordering segments; a
  dispatch-token fallback; defaulted parameters.
- Blast radius: isolated crate; `plan_lightning` is a pure function of `KernelInput` and does not
  read `KernelInput.layers`.
- Dispatch: clipping semantics audit, `SUMMARY ≤200 words`.
- Cost: M. Authorities: design; `FillLightning.cpp::Filler::_fill_surface_single`.
- Canonical refs: `Filler::_fill_surface_single` (`intersection_pl`).
- Verification: `cargo test --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml`
  (full crate suite, including `clip_lightning_semantics_tdd`).
- Falsifying exit: a planned record that changes when `KernelInput.layers` is emptied, clipping to
  planning domains, geometry welding, or an ambiguous dispatch treated as the first match.

### Step 4 — Test guest adapter

- Task: TASK-576. Objective: compile the included kernel through the existing native and WASM
  adapters.
- Precondition: the kernel crate compiles and the existing `slicer:layer-infill` world is
  authoritative.
- Postcondition: `test-guests/lightning-portability-guest/` implements a `LayerModule` with the
  `run_infill` trait method, decodes `probe_input_json`/`probe_output`/`probe_final_dispatch`, emits
  tags 1/2/3 for `planned` or the selected two-point dispatch for `clipped`, rejects
  `PaintValue::Custom` and `None` origins, has `cdylib` + `rlib`, and carries an isolated `[workspace]`
  sentinel. `cargo xtask build-guests --check` exits 0.
- Allowed reads: `test-guests/sdk-layer-infill-guest` manifest/source, `infill-postprocess-echo-guest`
  sentinel, the WIT world, SDK macro native entry, guest discovery/freshness.
- Allowed edits (3): guest `Cargo.toml`; guest `src/lib.rs`; guest `Cargo.lock`.
- Forbidden: canonical WIT; SDK/macros; production guest; a custom export; expectation data in the
  guest.
- Blast radius: test guest only; exhaustive transport literals with the documented waiver where no
  base exists; metadata representability and positive-width checks.
- Dispatch: guest build/freshness, `FACT` artifact identity and exit.
- Cost: S. Authorities: docs03/05/22, AGENTS freshness.
- Canonical refs: recorded parameter/graph references only.
- Verification: `cargo xtask build-guests`; `cargo xtask build-guests --check` (exit 0 only is
  fresh).
- Falsifying exit: missed discovery, incompatible WIT, a hidden host planner, or an expectation file
  entering guest linking.

### Step 5 — Driver, decoder, comparator, and four tests

- Task: TASK-576. Objective: author the native/WASM driver, the strict decoder, the comparable/
  non-comparable comparator, the negative controls, and the outcome writer.
- Precondition: Steps 3–4 artifacts build; the fixtures and canonical replay record exist.
- Postcondition: `tests/integration/main.rs` registers `mod lightning_portability_tdd;`; the test
  file includes its sibling via `#[path = "lightning_portability_support.rs"] mod support;`. Tests are
  `planned_graph_native_and_wasm`, `gate_b_decision_is_truthful`, `rejects_canned_echo_and_fallback`,
  and `outcome_records_gate_b`. Both adapters decode through the same strict decoder and are compared
  independently to `reference.replay.production.graph`; the recorded status equals the recomputed
  comparison; the clipped population is recorded `inconclusive` with the manifest reason and counts;
  the outcome writer emits `target/lightning-portability/outcome.json`.
- Allowed reads: fixtures, canonical replay record, kernel plan types, own API, native
  request/response and output builder, host WIT bindings, `tests/common/wasm_cache.rs`.
- Allowed edits (5): `crates/slicer-wasm-host/Cargo.toml` (dev-dependencies only); root `Cargo.lock`
  (generated); `tests/integration/main.rs`; `tests/integration/lightning_portability_tdd.rs`;
  `tests/integration/lightning_portability_support.rs`.
- Forbidden: fixture rebaselines, production host/paint/module changes, WIT/SDK changes, a fifth
  integration test, a digest-only or adapter-equality-only assertion, skipping a dispatch.
- Blast radius: dev graph and new tests only; exhaustive transport/request literals where no base
  exists.
- Dispatch: each Cargo command, `FACT` exit/test count; canonical difference audit,
  `SUMMARY ≤200 words`.
- Cost: M. Authorities: docs03/05/08/21/22, AGENTS test/freshness.
- Canonical refs: independent recorded graph/grounding via delegation.
- Verification: the four exact AC commands; the aggregate with the fail-closed prefix.
- Falsifying exit: zero tests, a skipped or unclassified population, a native/WASM equality-only
  assertion, or a canned/empty fallback counted as a pass.

### Step 6 — B outcome and documentation

- Task: TASK-576. Objective: write the validated B outcome and record it truthfully.
- Precondition: the four integration tests and the kernel clip test execute and produce per-case
  statuses.
- Postcondition: `target/lightning-portability/outcome.json` carries gate B, executed/freshness
  identities, the include list with per-file sha256, kernel/guest hashes, native/WASM counts,
  per-case planned statuses, per-case clipped `inconclusive` with the manifest reason and
  `13/2/2/4/6` counts, the named non-comparable populations, commands/exits, and blockers; the plan
  gains the B record preserving A; `TASK-576` gains
  `  - TASK-576 Gate B outcome: (pass|fail|inconclusive)`; `migration_eligible` is the conjunction of
  A pass, B pass, and independently recorded framework acceptance, with each unverified term named
  as a blocker.
- Allowed reads: outcome JSON, plan section window, `TASK-576` block, recorded A outcome.
- Allowed edits (3): `lightning_portability_support.rs` (writer hardening); the plan (append-only);
  `docs/07_implementation_status.md` (`TASK-576` only).
- Forbidden: rewriting approved plan prose, other backlog entries, packet 07, production code.
- Blast radius: evidence-local; re-derive revision/hash facts at execution.
- Dispatch: evidence review, `FACT` statuses with named blockers.
- Cost: S. Authorities: plan row-07 rule, ADR0066, docs22.
- Canonical refs: recorded revision/patch identity.
- Verification: AC-5; the two placement greps; `cargo xtask check-literals`;
  `cargo xtask check-test-quality --report`.
- Falsifying exit: a recorded `pass` from a failed comparison, `[x]` without both gates, or a
  missing named blocker.

## Per-Step Budget Roll-Up

| Step | Cost | Reason |
| --- | --- | --- |
| 1 | S | `#[path]` inclusion and scalar checks |
| 2 | M | Graph/grounding extraction over the included tree |
| 3 | M | Canonical input adapter, planning, clip semantics |
| 4 | S | Existing-stage guest adapter |
| 5 | M | Driver, decoder, comparator, four tests |
| 6 | S | Truthful outcome and docs |

Aggregate M; no L step. Stop and re-delegate before a step exceeds its budget.

## Packet Completion Gate

Re-dispatch every exact AC command, `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask build-guests --check`,
`cargo test --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml`,
`cargo xtask check-literals`, and `cargo xtask check-test-quality --report`. Validate the B record
independently, archive log hashes, and record the truthful B status. Completion does not require B
`pass`; a red scientific comparison is reported as failed/inconclusive feasibility with migration
blocked. The clipped population is complete only as a named `inconclusive`; the packet does not claim
canonical clip parity.
