# Requirements: lightning kernel portability (Gate B)

## Packet Metadata

- Packet: `layer-module-preparation_09_lightning-kernel-portability`
- Task: `TASK-576` (Gate B)
- Status: draft
- Governs: native/WASM execution of the owned kernel, planned graph/grounding comparison, clipped
  segment comparison, and the B outcome
- Does not govern: Gate A construction/fixtures (packet 07), production wiring, migration

## Problem Statement

The owned feasibility kernel exists as typed interfaces with unimplemented tree/planner/clipping.
Gate B must show whether the same kernel source, compiled natively and to WASM, reproduces the
independently recorded canonical planning graph, grounding witnesses, and clipped segments for
the five Classic families. The previous bundled packet could not close because clipped contributor
provenance was treated as a hard prerequisite of every comparison. This packet treats unavailable
contributor evidence as a per-dispatch `inconclusive` result, so the portability experiment can
execute and record a truthful verdict.

## In Scope

- Implement `tree.rs` and `planner.rs`: directed node/edge/root construction with object-layer
  pooling, grounding witnesses, root-point-hash sampling, and `clip_lightning` against the
  canonical final domains.
- Add the test-only guest `crates/slicer-wasm-host/test-guests/lightning-portability-guest/` using
  the existing `slicer:layer-infill` world and the current SDK macro/native entry.
- Wire host dev dependencies for native guest/kernel linking and SHA-256 fixture validation.
- Author the native/WASM driver and five tests: planned execution, truthful B decision, clipped
  segments/contributors, canned/echo/fallback rejection, and outcome recording.
- Record the B outcome in `target/lightning-portability/outcome.json`, the source-plan outcome
  section, and the `TASK-576 Gate B outcome:` backlog line.

## Out of Scope

- Gate A reconstruction, fixture publication, and the recorder. This packet consumes packet 07's
  fixtures and never edits them.
- New WIT, SDK/macro/runtime changes, production module/guest changes, preparation integration.
- `combine_infill`/grouped-union provenance repair. Affected dispatches are recorded
  `inconclusive`; the gap remains a migration requirement.
- Arachne, translated, and density-30 companions; the gate matrix is five Classic families.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md` — row 07 Gate B sub-gate, Q13/Q14, row-08 entry.
- `docs/adr/0066-private-layer-preparation-capability.md`.
- `docs/03_wit_and_manifest.md` — existing `slicer:layer-infill` world, no new WIT.
- `docs/05_module_sdk.md` — native entry, per-call reconstruction, `test` feature.
- `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md`,
  `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `generateTrees`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp` — grounding/attachment/
  `reconnectRoots`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp` — `convertToPolylines`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` — `_fill_surface_single` clipping.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` caller identity.

Citations use file plus function, never canonical line numbers.

## Acceptance Summary

| ID | Obligation | Evidence |
| --- | --- | --- |
| AC-1 | Both adapters execute and emit complete planned records | `planned_graph_native_and_wasm` |
| AC-2 | Recorded B decision equals the independent comparison | `gate_b_decision_is_truthful` |
| AC-3 | Clipped segment multisets and per-dispatch contributor evidence | `clipped_segments_and_contributors` |
| AC-4 | Canned/echo/fallback outputs are rejected | `rejects_canned_echo_and_fallback` |
| AC-5 | B outcome + plan/backlog records are truthful | `outcome_records_gate_b` |
| AC-N1 | Corrupt graph/provenance records fail, including both-adapter corruption | folded into AC-2/AC-4 |
| AC-N2 | Empty/echo/cached/region-isolated/substitute data cannot pass | folded into AC-4 |
| AC-N3 | Fail/inconclusive blocks migration | folded into AC-5 |

## Verification Commands

| Command | Purpose | Return |
| --- | --- | --- |
| `cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::planned_graph_native_and_wasm -- --exact` | Native + WASM planned execution | FACT pass/fail + counts |
| `cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::gate_b_decision_is_truthful -- --exact` | Decision truthfulness (fresh driver run) | FACT pass/fail + statuses |
| `cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::clipped_segments_and_contributors -- --exact` | Clipped multisets | FACT pass/fail + dispatch counts |
| `cargo test -p slicer-wasm-host --test integration lightning_portability_tdd::rejects_canned_echo_and_fallback -- --exact` | Negative controls | FACT pass/fail |
| `cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::outcome_records_gate_b -- --exact` | Outcome/backlog truthfulness (fresh driver run) | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest freshness prerequisite | exit 0/1/3 |
| `cargo check --workspace --all-targets` | Compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Warning gate | FACT pass/fail |
| `cargo xtask check-literals` | Literal/FRU gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Test-quality findings | SUMMARY touched findings |

The exact fail-closed command strings are in `packet.spec.md`; this table abbreviates the shared
prefix.

## Step Completion Expectations

- Step 1 ends when the kernel planned graph compiles and in-file tests exercise real topology.
- Step 2 ends when `clip_lightning` compiles and clips a real plan to real final domains.
- Step 3 ends when `cargo xtask build-guests --check` exits 0 for the new guest.
- Step 4 ends when host all-target compilation sees the native guest/kernel.
- Step 5 ends when the five tests execute; scientific mismatches are recorded, not hidden.
- Step 6 ends when the B outcome, plan section, and backlog line exist and `migration_eligible` is
  correctly false/true under the stated conjunction.

## Context Discipline Notes

Do not load the guest `component.wasm`, full Cargo output, lockfile contents, the whole fixture
tree, or large graph JSON. Use the driver's persisted per-run artifacts and bounded `SUMMARY`
dispatches. A guest failure is attributed only after `build-guests --check` returns 0.
