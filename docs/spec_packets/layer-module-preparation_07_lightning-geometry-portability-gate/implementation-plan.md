# Implementation Plan: lightning canonical input oracle (Gate A)

## Execution Rules

Steps are future work for `TASK-576`, not actions taken during authoring. Commands are
non-interactive Bash. Execute one bounded step at a time; keep the recorder patch frozen and never
edit packet 09's files. Every edit list is exhaustive and has at most three files (Cargo-generated
lockfiles budgeted separately). A scientific mismatch is recorded, not repaired.

### Step 1 — Fixture publisher

- Task: TASK-576. Objective: add a deterministic `--publish-fixtures` mode to the recorder that
  converts one Classic capture into the gate envelopes.
- Precondition: `target/lightning-gate-current` contains the five Classic variants; the
  checked-in recorder patch produced them.
- Postcondition: the publisher reads `source.json`, `planning.json`, `domains.json`,
  `provenance.json`, `replay.json`, `callers.jsonl`, and `clips.jsonl`; emits `a_source`,
  `b_input`, A/B `reference`, and the manifest hashes; never invents a value. A missing capture or
  case exits 3 with `{"status":"inconclusive"}`.
- Allowed reads: the recorder file, the capture manifest, one variant's artifact keys through
  bounded SUMMARY dispatches. Do not load large JSON or G-code.
- Allowed edits (2): `tools/lightning_gate/record.py`; `tools/lightning_gate/README.md`.
- Forbidden: fixture files in this step; the recorder patch; kernel/host code; production paths.
- Blast radius: recorder-only; no canonical build, no PnP code, no public fields. The publisher
  must be deterministic (`sort_keys`, stable case order, no timestamps) and must label
  `contributor_evidence` per final fill.
- Dispatch: artifact-to-field mapping question, `LOCATIONS ≤20`; conversion run, `FACT` exits and
  case counts.
- Cost: M. Authorities: packet `design.md` fixture model; docs08/21/22.
- Canonical refs: none new; use recorded artifacts only.
- Verification: `python -B -m py_compile tools/lightning_gate/record.py`; run the publisher twice
  into `target/lightning-fixture-check-1` and `-2`; compare every output hash; assert five cases
  and no missing artifact.
- Falsifying exit: a hand-authored expected geometry, a case silently dropped, a hash derived from
  the whole envelope instead of its subtree, or nondeterministic output.

### Step 2 — Emit the three portable fixture files

- Task: TASK-576. Objective: publish `inputs.json`, `canonical.json`, and `manifest.json` for the
  five Classic families.
- Precondition: Step 1 publisher is deterministic.
- Postcondition: the three files are checked in; each case has `wall_strategy = Classic`, a
  strict `a_source`, optional `b_input`, A/B reference branches, and manifest hashes. A's
  `a_source` contains no planning/final/resolved/graph/reference field.
- Allowed reads: publisher output and manifest.
- Allowed edits (3): the three fixture files.
- Forbidden: editing JSON by hand to fit a test; adding a fourth fixture file; recorder changes.
- Blast radius: data-only; the files must stay small enough for bounded reads (target each at
  most a few hundred KB). If a file would exceed that, split the conversion by case into the same
  three-file envelope and record the partition in the manifest, never a new implicit file.
- Dispatch: fixture size and key inventory, `SUMMARY ≤200 words`.
- Cost: S. Authorities: design fixture model, docs21.
- Canonical refs: recorded artifacts only.
- Verification: run the publisher into the fixture directory; `python -B -c` asserts five cases,
  present hashes, `Classic`-only, and no `planning_layers`/`final_fills`/`resolved_` key inside
  any `a_source` subtree.
- Falsifying exit: a missing case, a reference key inside `a_source`, or a fixture that requires
  the sibling checkout to load.

### Step 3 — Kernel source-only reconstruction

- Task: TASK-576. Objective: implement `build_planning_domain` and the portable polygon/distance
  primitives it needs.
- Precondition: Step 4 model exists (already delivered); fixture semantics are fixed.
- Postcondition: `domain.rs` derives planning domains, boundary provenance, operations,
  parameters, scalar source, and policies from `GateASourceInput` only. `polygon.rs` provides
  normalization/holes/area/offset helpers; `distance_field.rs` provides the nearest-boundary
  query. Missing required evidence returns the named `KernelError`.
- Allowed reads: kernel `model.rs`/`lib.rs`, bounded fixture samples, existing host geometry
  helpers for signatures only.
- Allowed edits (3): `domain.rs`; `polygon.rs`; `distance_field.rs` (+ Cargo-generated
  `Cargo.lock`).
- Forbidden: `model.rs`/`lib.rs` changes; reference/B access; host-algos; production paths.
- Blast radius: isolated crate only; no public IR/WIT/schema change. Each translated file gets the
  standard attribution header. In-file `#[cfg(test)]` tests are allowed and must assert real
  polygon behavior.
- Dispatch: canonical anchor/domain formula check, `SUMMARY ≤200 words`; compile, `FACT`.
- Cost: M. Authorities: design, docs02/08/attribution/22.
- Canonical refs: `PrintObject::bridge_over_infill`, `Generator::generateInternalOverhangs`
  (delegated).
- Verification: `cargo check --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml --all-targets`;
  `cargo test --manifest-path ...` for the in-file unit tests.
- Falsifying exit: a reference/B lookup, a defaulted angle/width, a stub returning empty domains,
  or a host-algos dependency.

### Step 3a — Kernel model capability extension

- Task: TASK-576. Objective: extend the delivered kernel DTO so the recorded A evidence is
  transportable, then repopulate it from `GateASourceInput`.
- Precondition: Step 1 measured four recorded-evidence gaps in the delivered `model.rs`; Step 3
  construction compiles against the current model.
- Postcondition: `EarlierSurfaceKind` carries the recorded bottom-bridge surface kind;
  `SourceCheckpoint` carries the recorder's `post_anchor` phase; contribution/operation segment
  identity can express the recorder's `null` (not-applicable) segment, distinct from ring/segment
  `0`; `RawRegionConfig` carries `dont_filter_internal_bridges` and `sparse_infill_pattern`;
  `LightningParameters` carries `supporting_radius`, `wall_supporting_radius`, `prune_length`, and
  `straightening_max_distance`. `domain.rs` populates every new field from `a_source` and never
  defaults a value it cannot see. The kernel still compiles and its unit tests stay green.
- Allowed reads: kernel `model.rs`/`lib.rs`/`domain.rs`, the recorder's artifact key names through
  bounded extraction, `docs/02_ir_schemas.md` for naming.
- Allowed edits (2): `model.rs`; `domain.rs` (+ Cargo-generated `Cargo.lock`).
- Forbidden: `lib.rs`; `polygon.rs`/`distance_field.rs` (Step 3 deliverables); reference/B access;
  host-algos; production paths; recorder changes.
- Blast radius: DTO-only, additive where possible; no public IR/WIT/schema change. The crate is
  reachable only as a **dev-dependency** of this packet's host test binary and adds no production
  dependency edge. Measured workspace facts (both true at once, which is why they are stated
  separately): the root `[workspace] exclude` keeps the directory out of guest discovery and out of
  a second-workspace-root conflict, while the host path dev-dependency additionally makes Cargo
  treat the crate as a root-workspace member, so it resolves against the root lock. `clipper2-rust`
  is pinned `=1.1.0` to the root lock's resolved version so standalone and in-workspace runs use one
  geometry version. Every new field documented with the recorded evidence it exists to carry.
- Dispatch: compile, `FACT`.
- Cost: S. Authorities: design fixture model, docs02/08.
- Canonical refs: none; this is a transport-capability extension, not a behaviour change.
- Verification: `cargo check --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml --all-targets`;
  `cargo test --manifest-path ...` stays green.
- Falsifying exit: a new field that no recorded artifact supplies, a behaviour change in
  `build_planning_domain`, `lib.rs`/`polygon.rs`/`distance_field.rs` edits, or a defaulted value.

### Step 4 — Host dependency wiring

- Task: TASK-576. Objective: make the kernel, `serde_json`, and `sha2` available to the host test
  binary.
- Precondition: the kernel crate compiles.
- Postcondition: `slicer-wasm-host` dev-dependencies include the kernel path dependency,
  `serde_json`, and `sha2`; all targets compile.
- Allowed reads: host dev-dependency block, kernel manifest.
- Allowed edits (4): `crates/slicer-wasm-host/Cargo.toml` (dev-dependencies only); root `Cargo.toml`
  (`[workspace] exclude` for the kernel path, required because the kernel sits inside a member's
  directory); `modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml` (remove the
  workspace sentinel the `exclude` replaces; pin `clipper2-rust` to the root lock's resolved
  version so standalone and in-workspace builds use one geometry version); root `Cargo.lock`
  (Cargo-generated).
- Forbidden: production dependencies/features; host producer; guest/SDK/WIT.
- Blast radius: dev graph only; native SDK/host-algos may remain for the host binary. The kernel's
  own closure must not gain `host-algos`.
- Dispatch: dependency audit, `FACT` same package path and no kernel host-algos.
- Cost: S. Authorities: docs05, AGENTS all-target gate.
- Canonical refs: none.
- Verification: `cargo check -p slicer-wasm-host --all-targets`.
- Falsifying exit: a production dependency change or a host-algos edge into the kernel.

### Step 5 — Oracle support and tests

- Task: TASK-576. Objective: author the projector, extractor, comparator, fixture loader, outcome
  writer, and the five exact tests.
- Precondition: fixtures and dev dependencies exist.
- Postcondition: `main.rs` registers `mod lightning_oracle_tdd;`; the test file includes its
  sibling via `#[path = "lightning_oracle_support.rs"] mod support;`; the tests are
  `fixture_provenance`, `gate_a_decision`, `wrong_domain_is_rejected`,
  `source_only_projection_is_isolated`, `outcome_records_gate_a`. The comparator recomputes the
  decision from raw construction + reference; the record matches it. Missing evidence is
  `inconclusive`, measured difference is `fail`, equality is `pass`.
- Allowed reads: fixtures, kernel API, host test registration, common cache.
- Allowed edits (3): `main.rs`; `lightning_oracle_tdd.rs`; `lightning_oracle_support.rs`.
- Forbidden: fixture rebaselines from candidate, production code, packet 09 files, a sixth test.
- Blast radius: new tests only; existing literal/FRU discipline applies. No test may skip a case
  or assert only a digest.
- Dispatch: compile/test, `FACT` exit and test count; comparator fact check, `SUMMARY`.
- Cost: M. Authorities: docs21/22, AGENTS test discipline.
- Canonical refs: independent recorded replay/domains only.
- Verification: `cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd:: -- --nocapture`
  (five tests); AC-1..AC-4 exact commands.
- Falsifying exit: a self-referential oracle, a skipped population, a green test that does not
  recompute the comparison, or a candidate-derived expectation.

### Step 6 — A outcome and documentation

- Task: TASK-576. Objective: write the validated A outcome and record it truthfully.
- Precondition: the five tests execute and produce per-case statuses.
- Postcondition: `target/lightning-oracle/outcome.json` carries gate A, executed flag, case
  statuses, hashes, commands/exits, and blockers; the source plan gains the outcome section with
  the A record; `TASK-576` gains `  - TASK-576 Gate A outcome: (pass|fail|inconclusive)` and stays
  `[ ]`/`[~]` (never `[x]` without both gates passing); `migration_eligible` is false with the B
  blocker.
- Allowed reads: outcome JSON, the plan's outcome section window, the `TASK-576` block.
- Allowed edits (3): `lightning_oracle_support.rs` (writer); `docs/specs/layer-module-preparation-plan.md`
  (append-only section); `docs/07_implementation_status.md` (`TASK-576` only).
- Forbidden: rewriting approved plan prose, other backlog entries, packet 09, code.
- Blast radius: evidence-local; re-derive revision/hash facts at execution.
- Dispatch: evidence review, `FACT` statuses with named blockers.
- Cost: S. Authorities: plan row-07 recording rule, ADR0066, docs22.
- Canonical refs: recorded revision/patch identity.
- Verification: AC-5 exact command; placement greps from `packet.spec.md`; `cargo xtask check-literals`;
  `cargo xtask check-test-quality --report`.
- Falsifying exit: a recorded `pass` from a failed comparison, `[x]` without both gates, or a
  missing named blocker.

## Per-Step Budget Roll-Up

| Step | Cost | Reason |
| --- | --- | --- |
| 1 | M | Deterministic conversion of five families |
| 2 | S | Emit and audit three data files |
| 3 | M | Source-only domain reconstruction |
| 3a | S | Minimal DTO capability extension |
| 4 | S | Dev-dependency wiring |
| 5 | M | Comparator, isolation, five tests |
| 6 | S | Truthful outcome and docs |

Aggregate M; no L step. Stop and re-delegate before a step exceeds its budget.

## Packet Completion Gate

Re-dispatch every exact AC command, `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and
`cargo xtask check-test-quality --report`. Validate the A record independently, then archive the
log hashes. Completion does not require A `pass`. A red scientific comparison is retained and
reported as failed/inconclusive feasibility with migration blocked.
