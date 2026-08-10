# 100 — Rename wipe-tower keys to Orca names

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Standardise wipe-tower's config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 2 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) |
|---|---|
| `bed_shape` | `printable_area` |
| `wipe_tower_purge_volume` | `prime_volume` |
| `wipe_tower_enabled` | `enable_prime_tower` |
| `wipe_tower_width` | `prime_tower_width` |

Owner: `modules/core-modules/wipe-tower`. Adjudication source rows: 03-asset-scoped-gap.md (`printable_area`/`prime_volume`/`enable_prime_tower`/`prime_tower_width` — exact; `bed_shape` is Slic3r's legacy name that Orca itself renamed).

Obligations:

- Rename in the module manifest `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points) and tests. Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences. Watch for the `wipe_tower_*` family: sibling keys with the prefix (e.g. `wipe_tower_speed`, `wipe_tower_brim_*` if any) that are NOT in this ticket's list must stay untouched — only the four rows above rename.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the four rows in `03-asset-scoped-gap.md` to record old → new name.
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the four renames are merged, the tree is green on the gates above, and the 03 rows are updated.
