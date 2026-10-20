---
status: draft
packet: layer-module-preparation_09_lightning-kernel-portability
task_ids:
  - TASK-576
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: lightning kernel portability (Gate B)

## Goal

Execute the isolated module-owned lightning kernel on the canonical planning inputs published by
packet 07, through the real native and WASM adapters, for the five Classic case families. Compare
the planned graph/grounding and the clipped segment multisets to the independent canonical
records, and record a truthful `pass`/`fail`/`inconclusive` Gate B outcome. A mismatch is a
completed investigation, not an incomplete packet.

## Scope Boundaries

This packet owns **Gate B only**. It implements the owned kernel's tree/planner/clipping, a
test-only guest adapter using the existing `slicer:layer-infill` world, host dev wiring, the
native/WASM driver, five tests, and the B outcome. It consumes packet 07's checked-in fixtures and
does not edit them, the recorder, or the recorder patch. No production wiring, no migration, no
new WIT/SDK surface, no preparation integration.

Final-fill contributor attribution is partially unavailable in the current recordings (surfaces
created by `combine_infill`). Clipped comparison therefore classifies each dispatch explicitly:
segment multisets are always compared; a dispatch whose canonical contributor evidence is
`unavailable` is recorded `inconclusive` with a named reason. No dispatch is silently skipped.

## Prerequisites and Blockers

- Packet 07 must publish `crates/slicer-wasm-host/tests/fixtures/lightning_gate/` with the five
  Classic cases and separate A/B/reference branches. That packet is `draft`; this packet's fixture
  steps are blocked until its publisher step lands. This is a draft dependency, not an implemented
  prerequisite.
- The kernel `model.rs`/`lib.rs` exist; `tree.rs`/`planner.rs` are the implementation surface.
- `cargo xtask build-guests --check` must exit 0 before any guest-execution failure is attributed
  to this packet. Exit 1 is stale/lock divergence; exit 3 is infrastructure and stops the run.
- Completion requires execution and a truthful record, not `pass`. `fail`/`inconclusive` blocks
  migration and returns the port for design review.

## Acceptance Criteria

Commands are non-interactive Bash. `set -euo pipefail` rejects Cargo, tee, and log-check failures;
every command creates `target` before writing. Each exact filter executes exactly one test; zero
tests fails. Read/archive the combined log before overwrite; never rerun for truncated output.

- **AC-1. Given** the packet-07 canonical planning inputs, **when** both real adapters execute `plan_lightning` on all five cases, **then** every case returns a complete planned record set (tags 1, 2, and 3), every path has an explicit non-`None` origin, path/origin vectors have equal lengths before zipping, all metadata integers are exactly representable in f32, the guest artifact is fresh, and the native and WASM executions each report nonzero node/edge/root populations. | `set -euo pipefail; mkdir -p target; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::planned_graph_native_and_wasm -- --exact --nocapture >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-2. Given** the executed planned records, **when** the independent comparator runs in the same test on a fresh planned-driver execution, **then** the recorded per-case status equals the recomputed comparison: under a node bijection preserving coordinates (within the recorded transport tolerance), origin identity, parent/root relations, and grounding witness, the complete directed edge-occurrence multiset, isolated roots, and complete boundary/source/contributor/operation table match for `pass`; `fail` names the first missing/extra/reversed entry; `inconclusive` names the unavailable population. Both adapters are compared independently to the canonical reference, never to each other. | `set -euo pipefail; mkdir -p target; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::gate_b_decision_is_truthful -- --exact --nocapture >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-3. Given** every recorded final dispatch, **when** `clip_lightning` executes and emits two-point segments, **then** each dispatch's oriented segment multiset is compared to the canonical clipped record and bound to that dispatch's caller identity and contributor set; a dispatch with `contributor_evidence = complete` is `pass`/`fail`, a dispatch with `unavailable` is `inconclusive` with the named reason, and the executed dispatch count equals the recorded count. | `set -euo pipefail; mkdir -p target; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::clipped_segments_and_contributors -- --exact --nocapture >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-4. Given** deliberately canned, echoed, region-isolated, or host-fallback outputs, **when** the driver and decoder run, **then** each is rejected with a named mismatch or a named missing-producer error, including identical corruption applied to both adapters; a valid nonempty case still passes the same decoder. No callable host lightning producer or fallback is installed. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_portability_tdd::rejects_canned_echo_and_fallback -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-5. Given** the executed B cases, **when** the outcome writer validates `target/lightning-portability/outcome.json` in the same test on a fresh driver execution, the source-plan outcome section, and `TASK-576`, **then** the record carries `gate: "B"`, executed/freshness identities, kernel/guest hashes, native/WASM executed counts, per-case planned/clipped statuses, commands/exits, and named blockers; `TASK-576 Gate B outcome:` is one of `pass|fail|inconclusive`; and `migration_eligible` is false unless A passes, B passes, and framework acceptance is independently recorded. | `set -euo pipefail; mkdir -p target; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::outcome_records_gate_b -- --exact --nocapture >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a missing/extra/duplicate node or edge, a reversed edge, a disconnected parent, a corrupt root witness, or a tag3 byte error, **when** the decoder runs, **then** each is named and fails, including when both adapters return the same corrupt payload.
- **AC-N2. Given** an empty, echoed, cached, region-isolated positional pair, or host-fallback output, **when** the comparator runs, **then** it is rejected; a digest-only or same-object self-comparison never counts as a pass.
- **AC-N3. Given** a failed/inconclusive A or B record, unexecuted artifacts, or absent framework acceptance, **when** migration eligibility is evaluated, **then** `migration_eligible` is false with the named blocker; no approximation or defaulted parameter changes the status.

## Completion Semantics

This is a feasibility investigation. The packet is complete when every exact command above has
executed and the outcome record truthfully reflects the measured comparison. The five tests assert
execution, strict decoding, and that the recorded status equals the independently recomputed
comparison; they do not assert that the comparison must be `pass`. A measured mismatch is recorded
as a case-level or dispatch-level `fail` (or a named `inconclusive`) and blocks migration; it is
never converted into a pass, repaired from expectations, or skipped. `TASK-576` is not `[x]`
unless both Gate A and Gate B pass.

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`
- `set -euo pipefail; mkdir -p target; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd:: -- --nocapture >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 5 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- `cargo xtask check-literals`
- `cargo xtask check-test-quality --report`

## Doc Impact Statement (Required)

- Future implementation appends the B subsection to `## Lightning geometry/portability gate outcomes` in `docs/specs/layer-module-preparation-plan.md`, preserving the A record. If the heading is absent, create it and add both records only after A is recorded. Check: `rg -q '^## Lightning geometry/portability gate outcomes$' docs/specs/layer-module-preparation-plan.md`.
- Future implementation updates only the existing `TASK-576` item under `## Perimeter Modules — OrcaSlicer Parity Roadmap` / `### M2 — Real Arachne` in `docs/07_implementation_status.md`, adding `  - TASK-576 Gate B outcome: (pass|fail|inconclusive)`. `[x]` requires both verified passes and packet acceptance; fail/inconclusive keeps `[~]`/`[ ]`. Placement checks: `rg -q '^- \[[ x~]\] \*\*TASK-576\*\*' docs/07_implementation_status.md`; `rg -q '^  - TASK-576 Gate B outcome: (pass|fail|inconclusive)\b' docs/07_implementation_status.md`.
- No backlog or source-plan edit occurs during authoring.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md`: row 07 Gate B sub-gate, Q13/Q14, row-08 entry.
- `docs/adr/0066-private-layer-preparation-capability.md`: migration waits for canonical strategy and portability proof.
- `docs/03_wit_and_manifest.md`: existing `slicer:layer-infill` world; no new WIT.
- `docs/05_module_sdk.md`: generated native entry, per-call reconstruction, test feature.
- `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md`, `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `Generator::generateTrees`, `generateInitialInternalOverhangs`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp` — `Layer::getBestGroundingLocation`, `Layer::attach`, `reconnectRoots`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp` — `Node::convertToPolylines` (root-point-hash seed, sampled lines).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` — `Filler::_fill_surface_single` (final clipping).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` (caller identity/policy).

Citations use file plus function, never canonical line numbers. Source inspection is not execution evidence.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
