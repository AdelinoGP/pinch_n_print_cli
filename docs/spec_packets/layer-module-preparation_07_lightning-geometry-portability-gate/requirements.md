# Requirements: lightning canonical input oracle (Gate A)

## Packet Metadata

- Packet: `layer-module-preparation_07_lightning-geometry-portability-gate`
- Task: `TASK-576` (Gate A)
- Status: implemented
- Governs: the canonical input oracle and the source-only reconstruction decision
- Does not govern: Gate B execution (packet 09), production wiring, migration, recorder extension

## Problem Statement

The recorder now produces independent canonical planning domains, scalars, resolved policy,
planning-boundary provenance, and replay records for the five Classic case families. What is
missing is a portable oracle and an independent reconstruction: the gate must show whether a
restricted earlier-source description is sufficient to rebuild the same planning domains, or
whether the strategy fails/inconclusive for a named reason. The previous bundled packet could not
close because it made Gate B and exhaustive final-fill provenance prerequisites of Gate A. This
packet isolates Gate A and accepts a truthful negative result.

## In Scope

- Publish the three portable fixture files for `anchor_bridge`, `pooled_regions`, `hole_void`,
  `island_split_merge`, `paint_modifier_config`, Classic only, from the existing recorder
  captures. `inputs.json` separates `a_source` from optional `b_input`; `canonical.json` separates
  the A reference from the B reference; `manifest.json` carries revisions, patch identity, and
  per-subtree hashes.
- Add a strict `GateASourceInput` projector/extractor that selects only the `a_source` subtree and
  never requires `b_input`/`reference` to deserialize.
- Implement the pure `build_planning_domain(&GateASourceInput) -> GateAConstruction` for the five
  cases: locally computed internal/void/temporary-anchor domains, planning-boundary provenance,
  derived parameters, scalar source, and `FillPolicyInput`.
- Compare A construction to the independent reference: planning layers, planning-boundary
  provenance, parameters, scalar source, and per-region/layer policy. Record per-case
  `pass`/`fail`/`inconclusive` with named mismatches or unavailable fields.
- Execute the existing `anchor_bridge` wrong-domain discriminator (faithful vs wrong final domain)
  as the negative control that the case actually discriminates.
- Author five tests (`fixture_provenance`, `gate_a_decision`, `wrong_domain_is_rejected`,
  `source_only_projection_is_isolated`, `outcome_records_gate_a`) and the A outcome record; append
  the A outcome to the source plan and the `TASK-576 Gate A outcome:` line to the backlog.

## Out of Scope

- Gate B: native/WASM kernel execution, planned graph transport, clipped final-fill comparison,
  guest adapter, guest freshness. Owned by packet 09.
- Final-fill contributor multisets, `combine_infill` lineage, grouped union/`diff_ex` operation
  records. These remain named migration requirements.
- Arachne, translated companions, and density-30 companions. Captures are retained as supporting
  evidence but are not gate coverage.
- Any production lightning path, public IR/WIT/schema/version change, framework packet, or
  migration edit.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md` — row 07 Gate A sub-gate, Q13/Q14, row-08 entry.
- `docs/adr/0066-private-layer-preparation-capability.md` — canonical strategy and portability
  proof precede migration.
- `docs/01_system_architecture.md` — PrePass stage order.
- `docs/02_ir_schemas.md` — SliceIR/SlicedRegion and post-perimeter fill invariants.
- `docs/05_module_sdk.md` — isolated crate and test feature boundaries.
- `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md`,
  `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::bridge_over_infill`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `Generator::Generator`,
  `generateInitialInternalOverhangs`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills`, `group_fills`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.hpp` — `Fill::fixed_angle` (read-only).

Citations use file plus function, never canonical line numbers. The sibling checkout is read-only.

## Acceptance Summary

| ID | Obligation | Evidence |
| --- | --- | --- |
| AC-1 | Strict portable fixture provenance for five Classic cases | `fixture_provenance` |
| AC-2 | A constructs all five cases and records a truthful decision | `gate_a_decision` |
| AC-3 | `anchor_bridge` faithful/wrong discriminator is real | `wrong_domain_is_rejected` |
| AC-4 | A construction is isolated from B/reference branches | `source_only_projection_is_isolated` |
| AC-5 | A outcome + plan/backlog records are truthful | `outcome_records_gate_a` |
| AC-N1 | Missing/tampered/incomplete evidence never passes | folded into AC-1/AC-2/AC-5 |
| AC-N2 | Reference poison changes only the verdict | folded into AC-4 |
| AC-N3 | Fail/inconclusive blocks migration | folded into AC-5 |

## Verification Commands

| Command | Purpose | Return |
| --- | --- | --- |
| `cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::fixture_provenance -- --exact` | Fixture strictness and hashes | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::gate_a_decision -- --exact` | Executed A decision | FACT pass/fail + statuses |
| `cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::wrong_domain_is_rejected -- --exact` | Discriminator | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::source_only_projection_is_isolated -- --exact` | Source isolation | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test integration lightning_oracle_tdd::outcome_records_gate_a -- --exact` | Outcome/backlog truthfulness | FACT pass/fail |
| `cargo check --workspace --all-targets` | Compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Warning gate | FACT pass/fail |
| `cargo xtask check-literals` | Literal/FRU gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Test-quality findings | SUMMARY touched findings |

Every test command runs with the fail-closed prefix from `packet.spec.md`; the table abbreviates
the shared prefix.

## Step Completion Expectations

- Step 1 ends when `--publish-fixtures` converts a Classic capture and emits all five cases with
  strict separation; no fixture is hand-edited.
- Step 2 ends when the three fixture files are checked in and their hashes are stable across two
  runs of the publisher.
- Step 3 ends when the kernel crate compiles and `build_planning_domain` returns typed
  `GateAConstruction` from `GateASourceInput` only.
- Step 3a ends when the recorded bottom-bridge surface, `post_anchor` checkpoint, absent segment
  identity, object-level anchor-filter key, and the four recorded scalars are all transportable, and
  the kernel's unit tests stay green.
- Step 4 ends when host dev dependencies compile on all targets.
- Step 5 ends when the five oracle tests compile and execute; a scientific mismatch is recorded,
  not hidden.
- Step 6 ends when the A outcome, plan section, and backlog line exist and the placement checks
  pass.

## Context Discipline Notes

Read the fixture publisher and the kernel model narrowly. Do not load the full 36-variant capture
tree, raw G-code, large JSON, lockfile contents, or canonical C++ sources. Delegate Cargo runs and
any OrcaSlicer fact to sub-agents. Keep the seven-step aggregate (Steps 1–6 plus 3a) at M; a step
that needs the full capture conversion may be split, never widened.
