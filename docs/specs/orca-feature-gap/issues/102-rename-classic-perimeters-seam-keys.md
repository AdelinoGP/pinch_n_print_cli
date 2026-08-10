# 102 — Rename classic-perimeters and seam keys to Orca names

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Standardise perimeter and seam config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 4 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) | owner |
|---|---|---|
| `wall_count` | `wall_loops` | classic-perimeters |
| `smaller_perimeter_threshold_mm` | `small_perimeter_threshold` | classic-perimeters |
| `seam_mode` | `seam_position` | seam-placer + seam-planner-default |

Adjudication source rows: 03-asset-scoped-gap.md (`wall_loops`/`small_perimeter_threshold`/`seam_position` — exact; the `_mm` suffix is a unit-suffix rename).

Obligations:

- Rename in the owner manifests `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points, `ResolvedConfig` fields — `wall_count` may be surfaced as a typed struct field per ticket 01's finding about typed-field keys) and tests. Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02. (Note: `wall_loops` default 3 is expected to match; verify, don't assume.)
- Update the three rows in `03-asset-scoped-gap.md` to record old → new name.
- **Blocks ticket 107** (the infill-duplicate collapse also edits classic-perimeters).
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the three renames are merged, the tree is green on the gates above, and the 03 rows are updated.
