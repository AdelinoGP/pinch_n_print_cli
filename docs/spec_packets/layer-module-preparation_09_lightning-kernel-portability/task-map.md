# Task Map: lightning kernel portability (Gate B)

Canonical task crosswalk: `TASK-576` in `docs/07_implementation_status.md`, specifically the
native/WASM portability sub-gate split out of source-plan queue row 07 by explicit approval. Packet
`layer-module-preparation_07_lightning-geometry-portability-gate` owns the canonical input fidelity
sub-gate and publishes the fixtures this packet consumes. Packet
`layer-module-preparation_08_lightning-migration` owns the ownership move into
`modules/core-modules/lightning-infill/src/kernel/` and its recorded-equivalence proof; that proof is
**not** a parity claim and does not resolve Gate B. The split does not waive either sub-gate, and
neither packet authorizes row-08 generation until both verified outcomes and implemented generic
framework acceptance exist. This packet does not reopen `TASK-577`; re-derive its state at
activation rather than quoting a status here.

| Task | Packet steps | Primary authority | Expected code surface | Canonical refs | Cost | Obligation |
| --- | --- | --- | --- | --- | --- | --- |
| TASK-576 | 1 | Approved portability gate; docs08/attribution | `feasibility-kernel/src/kernel/mod.rs`, crate manifest/lock | `Generator.cpp::Generator` scalar derivation | S | Same source included, never copied; recorded derived scalars |
| TASK-576 | 2 | docs22; `Layer::attach`/`getBestGroundingLocation` | `feasibility-kernel/src/tree.rs` | `Layer::attach`; `Node::convertToPolylines` | M | Complete node/edge/root/grounding records, resolvable addresses |
| TASK-576 | 3 | docs08/22; `FillLightning.cpp` | `feasibility-kernel/src/planner.rs` | `Filler::_fill_surface_single` (`intersection_pl`) | M | Canonical input adapter, `plan_lightning`, constructed-fill clip semantics |
| TASK-576 | 4 | docs03/05/22; guest discovery/freshness | Test guest manifest/source/lock | Recorded graph/parameter references only | S | Existing `slicer:layer-infill` world; tags 1–3 and clipped two-point output |
| TASK-576 | 5 | docs03/05/08/21/22; AGENTS all-target/freshness | Host dev dependency, `main.rs`, `lightning_portability_tdd.rs`, `lightning_portability_support.rs` | Independent canonical replay graph | M | Native + WASM execution and independent comparison; named non-comparable populations |
| TASK-576 | 6 | Plan row-07 recording, ADR0066/0074, docs22 | Outcome writer; append-only plan section; `TASK-576` B line | Kernel-source/guest/revision hashes | S | Truthful B status; `[x]` only after both gates pass; migration blocked otherwise |

Acceptance criteria belong exclusively to `packet.spec.md`. Gate A construction and fixture
publication belong to packet 07; the ownership move and recorded equivalence belong to packet 08.
This packet does not assert a B result; it authorizes the experiment and the truthful record. The
canonical clipped comparison and the per-node origin/provenance/sampling populations are named
non-comparable populations because the capture does not record them; they remain migration blockers.
