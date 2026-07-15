# Task Map: 133_infill-linker-module

Single-task-ID packet (`TASK-258`); the map is retained because the preflight gate (S0)
requires all five contract files. Backlog row: `TASK-258` in `docs/07_implementation_status.md`.
Depends on packets 129–132 (`clip_polylines`, the InfillPostProcess contract, per-region
config, and modifier sub-regions) all being `status: implemented` before activation (see
`packet.spec.md` §Prerequisites and Blockers). All OrcaSlicer reads are delegated per the
§OrcaSlicer Reference Obligations block — line ranges below are for the delegated sub-agent, not
for the implementer to open directly.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-258` | `Step 1` | `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md`, `docs/adr/0026-infill-linking-algorithms-in-linker-module.md`, `docs/03_wit_and_manifest.md` §claim table | `modules/core-modules/infill-linker/**` (new tree + `infill-linker.toml`), root `Cargo.toml`, `docs/03` claim row, scheduler dedup test, `manifest_ingestion` count 20 → 21 | none | M | Pass-through module + `claim:infill-link` (non-fill, no `FILL_CLAIM_IDS`); AC-8/AC-9/AC-N3. |
| `TASK-258` | `Step 2` | `docs/08_coordinate_system.md` (÷100) | `modules/core-modules/infill-linker/src/offset.rs` (new) + `src/lib.rs` wiring + `tests/infill_linker_tdd.rs` | `FillRectilinear.cpp:388-490` (`ExPolygonWithOffset`, sign-verify MANDATORY), `FillGyroid.cpp:356-359` (0.8×spacing filter) | M | Offset port + verified overlap sign + re-clip (`clip_polylines`) + short filter; AC-2/AC-3/AC-4. |
| `TASK-258` | `Step 3` | none new | `modules/core-modules/infill-linker/src/graph.rs` (new) + `tests/infill_linker_tdd.rs` | `FillBase.cpp:1432-1544` (`create_boundary_infill_graph`; `struct BoundaryInfillGraph` at 1265) | M | Arc-length boundary parametrization; graph unit tests (projection/arc distance/wrap-around). |
| `TASK-258` | `Step 4` | none new | `modules/core-modules/infill-linker/src/connect.rs` (new) + `src/lib.rs` wiring + `tests/infill_linker_tdd.rs` | `FillBase.cpp:1580-1818` (`connect_infill`, sectioned; constants ÷100) | M | Greedy endpoint connection core; AC-1/AC-5 + determinism. Step-4 split-packet tripwire armed. |
| `TASK-258` | `Step 5` | ADR-0025 §Amendment (the two branches) | `modules/core-modules/infill-linker/src/orchestrate.rs` (new) + `src/connect.rs` + `tests/infill_linker_tdd.rs` | `FillBase.cpp:1820-2246` (`chain_or_connect_infill`) | M | Wall-sharing groups: branch (a) union-then-link, branch (b) un-offset shared arcs; AC-6/AC-7/AC-N1/AC-N2. |
| `TASK-258` | `Step 6` | `docs/01_system_architecture.md` | `crates/slicer-runtime/tests/executor/infill_linker_pipeline_smoke_tdd.rs` (new) + harness mod line, `docs/01_system_architecture.md`, `.ralph/specs/131_per-region-config-delivery/carve-list.md` (append-only) | none | M | Pipeline smoke (AC-10) + Doc Impact + gates + carve-delta record. |

Aggregate context cost: `M` (six `M` steps). No step rated `L`; the Step-4 tripwire is the
anti-`L` guard (if the `connect_infill` port exceeds `M` mid-flight, split the packet rather
than rate the step `L`).
