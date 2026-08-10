# 107 — Collapse infill duplicate spellings to Orca names

Type: task
Status: open
Assignee: —
Blocked by: 102, 105
Map: ../map.md

## Question

Collapse the three duplicate spellings to the Orca names (ticket 07 ruling — workstream ticket 9 of 9). Both spellings are live today (03's "not renames — duplicates" row; verified read sites across modules and crates), so this is a delete-the-Pinch-duplicate rename, not a documentation fix:

| Pinch duplicate (drop) | Orca key (keep) |
|---|---|
| `infill_density` | `sparse_infill_density` |
| `infill_speed` | `sparse_infill_speed` |
| `infill_overlap` | `infill_wall_overlap` |

Blast radius (verified by read-site grep at charting time — re-derive): `arachne-perimeters`, `classic-perimeters`, `gyroid-infill`, `rectilinear-infill`, `infill-linker` (crate), `crates/slicer-gcode` (serialize), `crates/slicer-model-io` (+ its TDD fixtures), and tests in several modules. Every module that declared or read the duplicate spelling must converge on the Orca name — where a module declared *both* spellings, drop the duplicate and keep the Orca-named row; where code reads the duplicate, re-point the read.

Obligations:

- For each of the three pairs: the Orca-named key is authoritative (its manifest row, default, and decision point stay); the Pinch-duplicate spelling is deleted everywhere. **No behaviour change**: decision points keep reading the same underlying setting through the surviving spelling.
- Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`, tests) for each dropped spelling before and after — zero residual live occurrences.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing (doc 15 re-keyed; verify no duplicate rows remain), `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** the surviving Orca keys now match the reference by name — any "Deviations from OrcaSlicer" row where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the duplicates row in `03-asset-scoped-gap.md` (record the collapse).
- **Blocked by 102 and 105** — this ticket edits classic-perimeters (also touched by 102) and gyroid/rectilinear-infill (also touched by 105); run after both are merged.
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the three collapses are merged, the tree is green on the gates above, and the 03 row is updated.
