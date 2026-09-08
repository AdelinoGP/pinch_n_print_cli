# Task Map: 205a-integrated-edition-coverage

No `docs/07_implementation_status.md` TASK row exists for the multi-edition
distribution program (see `docs/specs/multi-edition-distribution-plan.md`
§"Backlog anchoring [FWD]"). This packet anchors to ADR IDs instead. Do not
invent a TASK number, and do not edit `docs/07` while the parallel 194-199
session is active.

| Anchor | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `ADR-0056` | `Step 1` | `docs/adr/0056-...md`, `docs/spec_packets/204-hybrid-pilot-parity/design.md` | none (read-only reconciliation) | none | `S` | Proves the sixteen modules' stages are natively committed and exactly two are transport-blocked |
| `ADR-0056` | `Step 2` | `docs/adr/0056-...md` | `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}` | none | `M` | AC-1, AC-2; registry features + native entries for the sixteen |
| `ADR-0056` | `Step 3` | `docs/adr/0042-...md` §Decision | `crates/slicer-runtime/tests/common/parity_invariants.rs`, `tests/contract/parity_invariants_selftest_tdd.rs` | none | `M` | New family comparators (finalization, seam-plan, layer-planning) + self-tests |
| `ADR-0056` | `Steps 4a1-4e` | `docs/adr/0042-...md` §Decision | sixteen per-module parity gate test files in `crates/slicer-runtime/tests/contract/` + `tests/contract/main.rs`, and the external-override test in `tests/integration/` | none | `S` each (4c `M`) | AC-3, AC-N2; per-module parity gates, each step at most 3 edits (2 new test files + `contract/main.rs`) — 4a1/4a2/4a3 infill ×5, 4b1/4b2/4b3 support/perimeters-postprocess ×5, 4c prepass ×2 (M), 4d1/4d2 finalization ×4, 4e external-override integration test |
| `ADR-0057` | `Step 5` | `docs/spec_packets/205-.../packet.spec.md` AC-7 | `crates/pnp-cli/Cargo.toml` | none | `S` | AC-5, AC-6; passthrough features + coverage-gate proof |
| `ADR-0057` | `Step 6` | `docs/adr/0057-...md` | `docs/01_system_architecture.md`, `docs/specs/multi-edition-distribution-plan.md` | none | `S` | Doc impact; records the two transport-blocked modules |
| `ADR-0056` | `Step 7` | `docs/adr/0042-...md` §Decision | `docs/DEVIATION_LOG.md` (conditional) | none | `S` | Conditional; only if Steps 4a1-4e widened a parity tolerance |

Costs are copied from `implementation-plan.md`. Aggregate `L`; no row is `L`.
This packet is the bulk of the plan's "205a+" follow-on; it does **not** close
the plan — packet 205b (transport completion for `Layer::PathOptimization` and
gcode-command application, then integration of `path-optimization-default` +
`machine-gcode-emit`) is the required follow-on that makes `--edition
integrated` build.
