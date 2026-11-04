# Requirements: lightning kernel portability (Gate B)

## Packet Metadata

- Packet: `layer-module-preparation_09_lightning-kernel-portability`
- Task: `TASK-576` (Gate B)
- Status: draft
- Governs: native/WASM execution of the module-owned kernel source on canonical planning inputs,
  planned graph/grounding comparison, constructed-fill clip semantics, and the B outcome
- Does not govern: Gate A construction/fixtures (packet 07), production wiring, migration, canonical
  final-fill capture

## Problem Statement

Packet 08 moved the canonical lightning planner out of the host and into
`modules/core-modules/lightning-infill/src/kernel/`, and proved the moved module reproduces the live
producer's recorded output natively and as a fresh WASM guest. That equivalence is an ownership
claim, not a canonical-parity claim: the producer's own constants and input derivation were frozen
by that packet, and its baseline is the producer's output, not OrcaSlicer's.

Gate B asks the remaining question: does the same kernel source, given the **canonical** planning
domains, reproduce the **canonically recorded** planning graph and grounding witnesses when executed
through the real native entry and a WASM guest? The experimental vehicle is the isolated
`feasibility-kernel` crate, which exists to hold a dependency-light portability proof. The kernel
source is therefore included by `#[path]` from the module, not copied: identical bytes, one source of
truth, no drift.

The canonical capture does not contain every population the packet's schema can carry. It records
the replay graph (nodes, edges, roots, `last_grounding`), the planning layers, parameters, and
clipped output segments — but not per-node origin identity, sampled segments, the per-node provenance
table, or final-fill domains/policy. Gate B must compare exactly the recorded populations, record the
unrecorded ones as named non-comparable populations, and never let the missing populations turn a
non-comparison into a pass.

## In Scope

- Add `feasibility-kernel/src/kernel/mod.rs` that includes the module's
  `generator.rs`, `layer.rs`, `tree_node.rs`, and `distance_field.rs` by `#[path]`, with no edit to
  the included files; declare the module from `lib.rs` and amend its crate comment to name the
  `slicer-core` exception; add `slicer-core` (default features) to the crate.
- Implement `plan_lightning(input: &KernelInput) -> Result<KernelPlan, KernelError>` in
  `planner.rs`: group canonical planning domains by resolved region, flatten each layer's internal
  domains (contours and holes) into the per-layer outline vector, construct the included `Generator`
  from the recorded parameters, and map each generated layer. (Amended 2026-10-25: the original
  polygon-ordinal grouping was not canonical; see `design.md`.)
- Implement the graph extraction in `tree.rs`: traverse `Layer::tree_roots`/`Node`, assign stable
  local ids, resolve parent/root relations, resolve each grounding point into the layer's
  `BoundaryAddress`, and carry the input `GraphProvenance` unchanged.
- Implement `clip_lightning` in `planner.rs` against constructed `FinalFillInput`s, using the
  production `clip_polylines` open-subject clip; prove hole preservation, oriented segments, and
  named missing/ambiguous-dispatch errors.
- Add a test-only guest under `crates/slicer-wasm-host/test-guests/lightning-portability-guest/`
  using the existing `slicer:layer-infill` world, the generated native entry, and an isolated
  workspace sentinel.
- Wire host dev dependencies for the guest and author the native/WASM driver plus four integration
  tests: planned execution, truthful graph decision, canned/echo/fallback rejection, and outcome
  recording.
- Record the B outcome in `target/lightning-portability/outcome.json`, the source-plan outcome
  section, and the `TASK-576 Gate B outcome:` backlog line.

## Out of Scope

- Gate A reconstruction, fixture publication, and the recorder. The packet consumes packet 07's
  fixtures and never edits them, and it does not extend or unfreeze the recorder.
- Per-node `origin_region_index`, per-node provenance-table, `sampled_segments`, and grounding-`kind`
  parity. These are recorded as named non-comparable populations because the canonical capture does
  not contain them.
- The canonical clipped-segment comparison: `reference.final_fills` is published `null` with the
  reason that `FinalFillInput.policy` needs per-region resolved angles the capture does not record.
  Clipping is proven only against constructed fills.
- `combine_infill`/grouped-union provenance repair. Affected dispatches stay recorded as a named
  migration blocker.
- Production adapter changes. The current production adapter calls `Generator::new` with
  `config.infill_angle` in the `lightning_overhang_angle` slot and with `prune_angle = 5.0`,
  `straightening_angle = 0.0`, while the canonical capture records `45.0`, `5.0`, and `5.0`. Gate B's
  adapter passes the recorded values in the named slots, and the production adapter's argument
  choices are recorded as an out-of-scope finding.
- New WIT, SDK/macro/runtime changes, production module/guest changes, preparation integration.
- Arachne, translated, and density-30 companions; the gate matrix is five Classic families.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md` — row 07 Gate B sub-gate, Q13/Q14, row-08 entry.
- `docs/adr/0066-private-layer-preparation-capability.md` and its `## Amendment — 2026-10-20`.
- `docs/adr/0074-lightning-migration-on-recorded-equivalence.md`.
- `docs/03_wit_and_manifest.md` — existing `slicer:layer-infill` world, no new WIT.
- `docs/05_module_sdk.md` — native entry, per-call reconstruction, `test` feature.
- `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md`,
  `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `generateTrees`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp` — grounding, attachment,
  `reconnectRoots`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp` — `convertToPolylines`
  propagation/straightening.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` — `_fill_surface_single` clipping.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` caller identity.

Citations use file plus function, never canonical line numbers.

## Acceptance Summary

| ID | Obligation | Evidence |
| --- | --- | --- |
| AC-1 | Both adapters execute `plan_lightning` and emit complete planned records | `planned_graph_native_and_wasm` |
| AC-2 | Recorded per-case decision equals the independent comparable-population comparison | `gate_b_decision_is_truthful` |
| AC-3 | `clip_lightning` semantics on constructed final fills | `clip_lightning_semantics_tdd` (kernel crate) |
| AC-4 | Canned/echo/fallback and corrupt records are rejected | `rejects_canned_echo_and_fallback` |
| AC-5 | B outcome, plan section, and backlog line are truthful | `outcome_records_gate_b` |
| AC-N1 | Corrupt graph/provenance records fail, including both-adapter corruption | folded into AC-2/AC-4 |
| AC-N2 | Empty/echoed/cached/region-isolated/substitute data cannot pass | folded into AC-4 |
| AC-N3 | Fail/inconclusive blocks migration | folded into AC-5 |

## Verification Commands

| Command | Purpose | Return |
| --- | --- | --- |
| `cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::planned_graph_native_and_wasm -- --exact` | Native + WASM planned execution | FACT pass/fail + counts |
| `cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::gate_b_decision_is_truthful -- --exact` | Decision truthfulness (fresh driver run) | FACT pass/fail + statuses |
| `cargo test --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml clip_lightning_semantics_tdd` | Constructed-fill clip semantics | FACT pass/fail |
| `cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::rejects_canned_echo_and_fallback -- --exact` | Negative controls | FACT pass/fail |
| `cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::outcome_records_gate_b -- --exact` | Outcome/backlog truthfulness (fresh driver run) | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest freshness prerequisite | exit 0/1/3 |
| `cargo check --workspace --all-targets` | Compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Warning gate | FACT pass/fail |
| `cargo test --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml` | Kernel crate suite | FACT pass/fail |
| `cargo xtask check-literals` | Literal/FRU gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Test-quality findings | SUMMARY touched findings |

The exact fail-closed command strings are in `packet.spec.md`; this table abbreviates the shared
prefix.

## Step Completion Expectations

- Step 1 ends when the four module kernel files are included by `#[path]` and the crate compiles, with
  an in-file test proving the recorded parameters produce the four recorded derived scalars.
- Step 2 ends when `tree.rs` turns a real multi-layer `Generator` run into complete
  node/edge/root/grounding records with resolvable boundary addresses.
- Step 3 ends when `plan_lightning` builds canonical inputs from `KernelInput.planning_layers` and
  `clip_lightning` passes constructed-fill tests, including named missing/ambiguous-dispatch errors.
- Step 4 ends when `cargo xtask build-guests --check` exits 0 for the new guest.
- Step 5 ends when the four integration tests execute; scientific mismatches are recorded, not hidden.
- Step 6 ends when the B outcome, plan section, and backlog line exist and `migration_eligible` is
  correctly false/true under the stated conjunction.

## Context Discipline Notes

Do not load the guest `component.wasm`, full Cargo output, lockfile contents, the whole fixture tree,
or large graph JSON. Use the driver's persisted per-run artifacts and bounded `SUMMARY` dispatches.
A guest failure is attributed only after `build-guests --check` returns 0.
