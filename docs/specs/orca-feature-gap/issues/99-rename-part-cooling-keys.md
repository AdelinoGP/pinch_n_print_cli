# 99 — Rename part-cooling keys to Orca names

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Standardise part-cooling's config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 1 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) |
|---|---|
| `disable_fan_first_layers` | `close_fan_the_first_x_layers` |
| `fan_speed_max` | `fan_max_speed` |
| `fan_speed_min` | `fan_min_speed` |
| `enable_overhang_fan` | `enable_overhang_bridge_fan` |

Owner: `modules/core-modules/part-cooling`. Adjudication source rows: 03-asset-scoped-gap.md lines `close_fan_the_first_x_layers` / `fan_max_speed` / `fan_min_speed` / `enable_overhang_bridge_fan` (fidelity: exact / word order).

Obligations:

- Rename in the module manifest `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points) and tests. Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing (doc 15's generated tables re-key), `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map (may amend queue packets) or record as an intended deviation with the human's sign-off per ticket 02's standard.
- Update the four rows in `03-asset-scoped-gap.md` to record old → new name (the 07 update note already classifies them as mechanical).
- Ledger facts (numbers, counts) re-derived from disk at edit time, never frozen.

Resolved when: the four renames are merged, the tree is green on the gates above, and the 03 rows are updated.
