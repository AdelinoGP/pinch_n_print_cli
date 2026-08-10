# 104 — Rename support/layer-planner keys to Orca names

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Standardise support and layer-planning config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 6 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) | owner |
|---|---|---|
| `support_top_z_distance_mm` | `support_top_z_distance` | support-planner + tree-support |
| `first_layer_height` | `initial_layer_print_height` | layer-planner-default (+ `crates/slicer-ir/src/resolved_config.rs` — ticket 01 found this key implemented under the Pinch spelling with zero Orca-spelling occurrences) |

Adjudication source rows: 03-asset-scoped-gap.md (`support_top_z_distance`/`initial_layer_print_height` — exact; the `_mm` suffix is a unit-suffix rename).

Obligations:

- Rename in the owner manifests `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points, the `ResolvedConfig` field for `first_layer_height`) and tests. Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the two rows in `03-asset-scoped-gap.md` to record old → new name.
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the two renames are merged, the tree is green on the gates above, and the 03 rows are updated.
