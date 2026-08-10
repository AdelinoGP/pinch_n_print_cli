# 101 — Rename path-optimization keys to Orca names

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Standardise path-optimization-default's config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 3 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) |
|---|---|
| `retract_length` | `retraction_length` |
| `retract_speed` | `retraction_speed` |
| `travel_z_hop` | `z_hop` |

Owner: `modules/core-modules/path-optimization-default` plus any consumers of the old spellings elsewhere (emission-time consumers — grep the whole tree). Adjudication source rows: 03-asset-scoped-gap.md (`retraction_length`/`retraction_speed`/`z_hop` — exact).

Obligations:

- Rename in the module manifest `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points, emitter consumers, host keys if the defaults are mirrored) and tests. Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the three rows in `03-asset-scoped-gap.md` to record old → new name.
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the three renames are merged, the tree is green on the gates above, and the 03 rows are updated.
