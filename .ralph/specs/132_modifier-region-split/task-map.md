# Task Map: 132_modifier-region-split

Single-task-ID packet (`TASK-257`); the map is retained because the preflight gate (S0)
requires all five contract files. Backlog row: `TASK-257` in `docs/07_implementation_status.md`.
FORWARD-DEP: packets 130 and 131 must be `status: implemented` before activation (see
`packet.spec.md` §Prerequisites and Blockers).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-257` | `Step 1` | `docs/adr/0030-modifier-splits-fill-not-perimeters.md`, `docs/specs/modifier-region-infill.md` §M1 | `.ralph/specs/132_modifier-region-split/design.md` (memo append only) | none | S | Discovery memo resolves the three `[FWD]` questions before any code lands. |
| `TASK-257` | `Step 2` | ADR-0030 (AC semantics) | `crates/slicer-runtime/tests/executor/modifier_region_split_tdd.rs` (new) + `main.rs` mod line | none | M | RED suite pins the split/wall-source/no-walls/z-scoping/degenerate semantics TASK-257 names. |
| `TASK-257` | `Step 3` | ADR-0030 Decision points 1-2 | `crates/slicer-runtime/src/region_partition.rs`, modifier-slicing site (per memo), 130 wall-source predicate site | none | M | The geometric split itself — wall-less sub-regions sharing base walls. |
| `TASK-257` | `Step 4` | ADR-0030 Decision point 3 | `crates/slicer-core/src/algos/region_mapping.rs` (`ModifierScope` variant + targeted stamp), new contract test | none | M | `ModifierScope` beyond `AllFeatures` — the config-binding half of the backlog row. |
| `TASK-257` | `Step 5` | `docs/02_ir_schemas.md` | `docs/02_ir_schemas.md` (modifier sub-region subsection) | none | S | Byte-identity guard + Doc Impact grep + gates close the row. |

Aggregate context cost: `M` (S + M + M + M + S). No step rated `L`.
