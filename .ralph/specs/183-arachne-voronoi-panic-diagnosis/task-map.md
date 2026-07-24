# Task Map: 183-arachne-voronoi-panic-diagnosis

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-296` | `Step 1` | none required | `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md` (`## Baseline`) | none | `S` | Captures the pre-change `perimeter_parity` status and raw-panic count — obtainable only before the guard lands. |
| `TASK-296` | `Step 2` | `docs/adr/0023-arachne-port-strategy.md` | `crates/slicer-core/src/voronoi.rs` (`voronoi_from_segments`, `VoronoiError`) | none | `S` | Adds the missing `catch_unwind` guard, closing the asymmetry with the two already-guarded boostvoronoi call sites; this is also the diagnostic capture point. |
| `TASK-296` | `Step 3` | none required | `crates/slicer-core/tests/voronoi_stress.rs` | none | `S` | Degenerate-input regression test, modeled on the existing `medial_axis_degenerate_input_tdd.rs` precedent. |
| `TASK-296` | `Step 4` | none required | `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md` (counts, characterization) | none | `M` | Runs the workload under the guard and measures the wall-loop delta that answers D-167's open question. |
| `TASK-296` | `Step 5` | `docs/DEVIATION_LOG.md` (D-167 row) | `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md` (`## Verdict`), `docs/DEVIATION_LOG.md` | none | `S` | Records the verdict and reconciles the D-167 row — closed as inert, or narrowed to a named successor owning `preprocess_input_outline` hardening. |

Backlog anchor: deviation `D-167-BOOSTVORONOI-ROBUST-FPT-PANICS` in `docs/DEVIATION_LOG.md` (also listed in the generated open-deviations block of `docs/07_implementation_status.md`).

**Task-ID allocation note.** `TASK-296` was verified free against both `docs/07_implementation_status.md` and `.ralph/specs/**`. The highest id in `docs/07` is `TASK-294`, owned by packet `178-seam-region-aware-planning`; sibling packets 182 and 181 take `TASK-295` and `TASK-297`. **Re-derive the next free id from BOTH the ledger and the spec tree before trusting this note** — packet 181's first allocation collided precisely because only `docs/07` was checked.

No OrcaSlicer parity refs: the guard defends against a Rust dependency's assertion and has no canonical analogue.

Scope note: this packet is the T1 **diagnosis spike** half of the original P-ARACHNE-GEOM grouping. `D-154-DISCRETIZE-POINT-POINT-CASE` was split into its own T3 packet (queue row 6 of `docs/specs/deviation-backlog-remediation-plan.md`) because it requires adding an `is_secondary` field to `HalfEdge`, whose blast radius would make a bundled packet context-cost `L`.
