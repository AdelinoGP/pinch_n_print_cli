---
status: implemented
packet: layer-module-preparation_07_lightning-geometry-portability-gate
task_ids:
  - TASK-576
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: lightning canonical input oracle (Gate A)

## Goal

Produce the independently recorded canonical input oracle for the five Classic lightning case
families and decide Gate A: whether a restricted earlier-source reconstruction reproduces canonical
planning domains, parameters, and resolved policy. Record a truthful `pass`/`fail`/`inconclusive`
outcome. A failed or inconclusive reconstruction is a completed investigation, not an incomplete
packet.

## Scope Boundaries

This packet owns **Gate A only**. It publishes the portable fixture set from the existing
recorder captures, implements the source-only reconstruction in the isolated kernel, and records
the A outcome. Gate B (the same owned kernel executed through native and WASM adapters) is packet
`layer-module-preparation_09_lightning-kernel-portability` and consumes these fixtures. No
production wiring, no migration, no recorder extension, no Arachne, translated, or density
companions.

Final-fill contributor provenance is explicitly out of scope. Gate A compares planning domains,
planning-boundary provenance, parameters, and policy; it does not require grouped final-fill
contributors. The fixture publisher emits B's structural branches so packet 09 finds the keys
present, but the capture does **not** record the underlying evidence for three of them —
`b_input.input`, `reference.graph`, and `reference.final_fills` are published as `null` with a
named `absent` reason in the manifest — so packet 09 starts with those as named blockers, not with
consumable data. A never parses or uses any B branch during construction.

## Prerequisites and Blockers

- The recorder at `tools/lightning_gate/` already produces the required captures. A Classic-only
  capture exists at `target/lightning-gate-current` (disposable). If it is absent, Step 1 re-runs
  `record.py --capture --strategy classic`; a missing canonical build exits 3 with
  `inconclusive`, never a synthesised oracle.
- Gate A cannot claim `pass` for a case whose canonical planning record is absent or whose
  restricted earlier-source evidence is unavailable. It records `inconclusive` with the named
  field.
- Completion requires execution and a truthful record, not `pass`. `fail`/`inconclusive` blocks
  migration and returns the strategy for design review.

## Acceptance Criteria

Commands are non-interactive Bash. `set -euo pipefail` rejects Cargo, tee, and log-check failures;
every command creates `target` before writing. Each exact filter executes exactly one test; zero
tests fails. Read/archive `target/test-output.log` before overwrite; never rerun for truncated
output.

- **AC-1. Given** the checked-in `crates/slicer-wasm-host/tests/fixtures/lightning_gate/{inputs.json,canonical.json,manifest.json}` and the `anchor_bridge`, `pooled_regions`, `hole_void`, `island_split_merge`, `paint_modifier_config` cases, **when** the oracle loader validates them without the sibling checkout, **then** every case resolves a strict `GateInputCase`/`GateOracleCase` envelope, `wall_strategy` is `Classic`, `a_source_sha256`, `a_raw_config_sha256`, `a_earlier_provenance_sha256`, `b_input_sha256`, and `reference_sha256` are present as a hash or explicit `"absent"` and match their subtrees when present, the manifest carries `canonical_revision` and recorder-patch identity, and absent files are errors. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::fixture_provenance -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-2. Given** each case's `a_source` subtree, **when** `project_gate_a` → `extract_gate_a_source` → `build_planning_domain` runs, **then** all five cases are attempted and each yields either a `GateAConstruction` or a named `inconclusive`/`fail` error, using the restricted input only, and the recorded per-case status equals an independently recomputed comparison of planning layers, planning-boundary provenance, parameters, scalar source, and policies: `pass` has no mismatch, `fail` has named mismatches, `inconclusive` names the unavailable population. No `b_input` or `reference` value may reach construction. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::gate_a_decision -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-3. Given** the `anchor_bridge` faithful and wrong final-domain replay records, **when** the direct replay comparison runs, **then** the faithful graph matches the independently recorded production graph, the wrong-domain graph differs, and the aligned per-layer symmetric-difference area is positive (`142.55912927276 mm²` on the current capture). | `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::wrong_domain_is_rejected -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-4. Given** a supported case with permitted source bytes held fixed, **when** `b_input` and `reference` are independently removed, replaced, and poisoned, **then** the projected `a_source` bytes and the three source/config/provenance hashes are identical before and after every run, and the complete `GateAConstruction` is unchanged; a reference-only change alters only the comparator verdict. Unknown reference-like keys inside `a_source` fail the whitelist. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::source_only_projection_is_isolated -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-5. Given** the executed A cases, **when** the outcome writer validates `target/lightning-oracle/outcome.json`, the source-plan outcome section, and the existing `TASK-576` item, **then** the record carries `gate: "A"`, the executed flag, per-case statuses, fixture/provenance hashes, commands and exits, and named blockers; the plan section exists; `TASK-576 Gate A outcome:` is one of `pass|fail|inconclusive`; and `migration_eligible` is false with the B/framework blocker. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::outcome_records_gate_a -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a missing fixture case, a tampered `a_source` hash, an unknown `a_source` key, or an absent planning record, **when** the loader and builder run, **then** the result is `inconclusive` or `fail` with a named field/population, never `pass`; a missing case cannot be skipped silently.
- **AC-N2. Given** the same permitted source bytes, **when** only the reference planning domains or policies are poisoned, **then** the comparator verdict may change but the A construction and source hashes do not. A discriminating reference change is actually executed.
- **AC-N3. Given** a `fail` or `inconclusive` A record, **when** migration eligibility is evaluated, **then** `migration_eligible` is false with the A blocker named; no approximation or defaulted parameter repairs the result.

## Completion Semantics

This is a feasibility investigation. The packet is complete when every exact command above has
executed and the outcome record truthfully reflects the measured comparison. The five tests assert
execution, strict decoding, and that the recorded status equals the independently recomputed
comparison; they do not assert that the comparison must be `pass`. A measured mismatch is recorded
as a case-level `fail` (or a named `inconclusive`) and blocks migration; it is never converted into
a pass or repaired from reference data. `TASK-576` is not `[x]` unless Gate A passes and Gate B
later passes.

## Verification

- `cargo check --workspace --all-targets`
- `cargo test --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd:: -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 5 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- `cargo xtask check-literals`
- `cargo xtask check-test-quality --report`
- `cargo xtask build-guests --check` (exit 0 required; the root workspace manifest is a guest
  fingerprint input, so a manifest edit makes every guest stale until rebuilt)

## Doc Impact Statement (Required)

- Future implementation appends `## Lightning geometry/portability gate outcomes` to `docs/specs/layer-module-preparation-plan.md` and records the A result only. If the heading already exists, append the A subsection; never rewrite approved prose. Check: `rg -q '^## Lightning geometry/portability gate outcomes$' docs/specs/layer-module-preparation-plan.md`.
- Future implementation updates only the existing `TASK-576` item under `## Perimeter Modules — OrcaSlicer Parity Roadmap` / `### M2 — Real Arachne` in `docs/07_implementation_status.md`, adding `  - TASK-576 Gate A outcome: (pass|fail|inconclusive)`. The checkbox stays `[ ]` or `[~]` until both Gate A and Gate B pass and this packet is accepted; A alone never earns `[x]`. Placement checks: `rg -q '^- \[[ x~]\] \*\*TASK-576\*\*' docs/07_implementation_status.md`; `rg -q '^  - TASK-576 Gate A outcome: (pass|fail|inconclusive)\b' docs/07_implementation_status.md`.
- No backlog or source-plan edit occurs during authoring.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md`: settled requirements, row 07 Gate A sub-gate, Q13/Q14, and the row-08 entry rule.
- `docs/adr/0066-private-layer-preparation-capability.md`: no migration before canonical strategy and portability proof.
- `docs/01_system_architecture.md`: PrePass stage order; validation order is not an execution trace.
- `docs/02_ir_schemas.md`: SliceIR/SlicedRegion and post-perimeter fill invariants; final wall-inset geometry is not a late-PrePass product.
- `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md`, `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`: conversion, attribution, fixture/literal discipline, and independent oracle/negative controls.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::bridge_over_infill`: pre-anchor surfaces, temporary anchor expansion, restoration seam.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `Generator::Generator`, `generateInitialInternalOverhangs`: first-region scalar-source policy, pooling, overhangs.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` / `group_fills`: resolved caller policy, angles, `fixed_angle`, grouped geometry.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.hpp` — `Fill::fixed_angle`: bool type authority, read-only.

The verified local source location is `D:/slicerProject/pinch_n_print_cli_2/OrcaSlicerDocumented/`; it is read-only source context, never an ordinary fixture dependency. Citations use file plus function, not canonical line numbers. Source inspection is not canonical recording/execution evidence.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
