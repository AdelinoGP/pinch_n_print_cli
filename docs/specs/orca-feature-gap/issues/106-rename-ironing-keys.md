# 106 — Rename ironing keys to Orca names

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Standardise ironing config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 8 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) | owner |
|---|---|---|
| `ironing_flow_rate` | `support_ironing_flow` | support-surface-ironing |
| `ironing_spacing` | `support_ironing_spacing` | support-surface-ironing |
| `ironing_spacing_mm` | `ironing_spacing` | top-surface-ironing |

Adjudication source rows: 03-asset-scoped-gap.md (`support_ironing_flow`/`support_ironing_spacing` — exact) plus the 07 amendment: `ironing_spacing_mm` → `ironing_spacing` (the "four spellings, one concept" row 03 missed — top-surface-ironing's manifest).

**Scope boundary — do NOT rename `ironing_enabled`.** It is being *widened*, not renamed: ticket 07 reclassified `ironing_type` and `support_ironing` as gaps, and that work belongs to packets P14 and P15 (tickets 21, 22). This ticket renames only the three rows above.

Obligations:

- Rename in the owner manifests `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points) and tests. Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences. Note `ironing_spacing` appears on **both** sides of the table (top-surface-ironing's target name collides with support-surface-ironing's old name) — the two renames must not cross wires: support-surface-ironing's `ironing_spacing` → `support_ironing_spacing` first, then top-surface-ironing's `ironing_spacing_mm` → `ironing_spacing`.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the three rows in `03-asset-scoped-gap.md` (two renamed rows + the 07 amendment's `ironing_spacing_mm` row).
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the three renames are merged, the tree is green on the gates above, and the 03 rows are updated.
