# Task Map: 147-arachne-cross-cutting-closure

This packet has `task_ids: none` (the cross-cutting closure packet for the
N1–N13 chain). The task map documents the crosswalk for the chain closure.

## `docs/07` Crosswalk

| `docs/07` row | Packet | Status | This packet's relationship |
| --- | --- | --- | --- |
| M2 — P110..P113c (Real Arachne foundations + graph construction) | 110..113c | implemented | F closes the chain that extends 113c's faithful graph construction with canonical emission + transitions + post-process (A1–E). |
| M2 — P141/P142/P143/P144/P145/P146 (no TASK-###) | A1/A2/B/C/D/E | draft → implemented | F depends on ALL of A1–E strictly; F cannot close until they are `status: implemented`. |
| M2 — Real Arachne N1–N13 parity | (this chain) | — | F records the chain closure in `docs/07_implementation_status.md` (M2 Real Arachne N1–N13 parity complete). |

## `docs/DEVIATION_LOG.md` Crosswalk

| Entry | Status | This packet's action |
| --- | --- | --- |
| `D-141-JUNCTION-BANDS` | Closed (A1) | F adds a one-line addendum noting the chain is closed. No in-place edits. |
| `D-142-CONNECTJUNCTIONS-EMISSION` | Closed (A2) | F adds a one-line addendum noting the chain is closed. |
| `D-143-TRANSITION-ENDS` | Closed (B) | F adds a one-line addendum noting the chain is closed. |
| `D-144-ANGLE-FUDGE-NONCENTRAL` | Closed (C) | F adds a one-line addendum noting the chain is closed. |
| `D-145-LOCAL-MAXIMA-EPILOGUE` | Closed (D) | F adds a one-line addendum noting the chain is closed. |
| `D-146-POSTPROCESS-ORDER` | Closed (E) | F adds a one-line addendum noting the chain is closed. |
| `D-147-CHAIN-CLOSURE` (NEW) | — | F creates this entry documenting the chain closure (all N1–N13 fixes in place, e2e closure gate green, `cargo xtask test --workspace --summary` PASS). |
| `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` | Closed (113c) | Untouched by F (113c's chain is its own; F's ADR 0035 references it as the predecessor). |

## OrcaSlicer Refs by Step

F owns NO new OrcaSlicer parity refs (A1–E own the chain's refs). F's ADR 0035
references the chain's parity surface but does not introduce new refs. Any
diagnostic reads during F's Step 1 e2e-residual diagnosis MUST be delegated
per the `orca-delegation` contract.

## ADR Crosswalk

| ADR | Status | This packet's action |
| --- | --- | --- |
| `docs/adr/0034-arachne-faithful-graph-construction.md` | Existing (113c) | F reads it (short); ADR 0035 follows it as the next free number. |
| `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (NEW) | — | F authors this ADR recording the chain's architectural decision: canonical `generateJunctions`/`connectJunctions` emission (A1/A2), transition ends + `generateExtraRibs` (B), `filterNoncentralRegions` + configured angle (C), local maxima + construction epilogue (D), canonical post-process order (E), superseding the PNP "ADAPTATION" divergence documented in 113c's ADR 0034. |