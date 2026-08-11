# Task Map: 205b-native-transport-completion

No `docs/07_implementation_status.md` TASK row exists for the multi-edition
distribution program. This packet anchors to ADR IDs and does not edit `docs/07`.

| Anchor | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `ADR-0056` | `Step 1` | `docs/adr/0056-...md`, 205a design | none (read-only reconciliation) | none | `S` | Confirms the two blocked transports and module shapes |
| `ADR-0056` | `Step 2` | `docs/adr/0056-...md` | `crates/slicer-wasm-host/src/marshal/native.rs` | none | `M` | Completes path-optimization and postpass gcode commits |
| `ADR-0056` | `Step 3` | `docs/adr/0056-...md` | `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}` | none | `M` | Features, registrations, native entries, in-file tests |
| `ADR-0056` | `Steps 4a-4c` | `docs/adr/0042-...md` §Decision | `crates/slicer-runtime/tests/{common/parity_invariants.rs,contract/,integration/}` | none | `M`/`S` | 4a comparators + AC-N1, 4b parity gates + AC-3/AC-4, 4c external override + AC-N2 |
| `ADR-0057` | `Step 5` | `docs/adr/0057-...md` | `crates/pnp-cli/Cargo.toml` | none | `S` | Passthrough features and Integrated-edition closure |
| `ADR-0057` | `Step 6` | `docs/adr/0057-...md` | `docs/01_system_architecture.md`, `docs/specs/multi-edition-distribution-plan.md` | none | `S` | Doc impact; records that the Integrated edition now builds |

Costs are copied from `implementation-plan.md`. Aggregate `M`; no row is `L`.
