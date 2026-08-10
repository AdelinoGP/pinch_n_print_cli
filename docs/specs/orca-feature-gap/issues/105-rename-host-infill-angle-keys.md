# 105 — Rename host and infill-angle keys to Orca names

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Standardise host and infill config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 7 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) | owner |
|---|---|---|
| `gcode_resolution` | `resolution` | host (docs/config/host-keys.toml + slicer-ir host config) |
| `infill_angle` | `infill_direction` | gyroid-infill + rectilinear-infill |

Adjudication source rows: 03-asset-scoped-gap.md (`resolution`/`infill_direction` — exact).

Obligations:

- Rename in the owner manifests `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points) and tests. `gcode_resolution` lives in `docs/config/host-keys.toml` — rename there too and update the consumer struct it mirrors (the `host_keys_doc_lock_tdd.rs` lock test fails until the struct and the TOML agree). Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests, and the host-key lock test.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the two rows in `03-asset-scoped-gap.md` to record old → new name.
- **Blocks ticket 107** (the infill-duplicate collapse also edits gyroid-infill + rectilinear-infill).
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the two renames are merged, the tree is green on the gates above, and the 03 rows are updated.
