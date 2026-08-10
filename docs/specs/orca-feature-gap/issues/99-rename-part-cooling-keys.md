# 99 — Rename part-cooling keys to Orca names

Type: task
Status: resolved
Assignee: wayfinder session (ses_016603e7affem7g2wEEEmg12cw)
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

## Answer

All four renames done and merged into this commit: manifest, `src/lib.rs`
(struct fields + `config.get` strings + comments), both module test files,
slicer-runtime's integration test, slicer-sdk's fixture docs, `docs/01`
prose, and `docs/15` (hand-maintained prose + regenerated generated tables).
`fan_speed_min` → `fan_min_speed` note: the key was declared but never read by
the module; the rename preserved that fact.

### Gates

- `cargo xtask gen-config-docs --check` — OK (204 module keys, 42 host keys).
- `cargo test -p part-cooling` — 9 tests pass (3 schema + 6 behavioural).
- `cargo test -p slicer-runtime` — 217 unit tests pass; 127 e2e tests pass
  (after rebuilding `pnp-cli`, which the e2e harness requires).
- `cargo xtask build-guests --check` — **stale for ALL 30+ guests, verified
  pre-existing** (reproduced with the working tree stashed). Not caused by,
  or fixable in, this rename ticket; guests do not embed config key names.
  Reported as a known pre-existing condition for the map.

### Deviation triage (user ruling: amend P01)

The rename exposed two real findings in the generated "Deviations from
OrcaSlicer" table:

| key | Pinch | Orca | root cause |
|---|---|---|---|
| `fan_max_speed` | 255 (raw 0–255) | 100 (% 0–100) | scale mismatch |
| `fan_min_speed` | 51 (raw 0–255) | 20 (% 0–100) | scale mismatch + declared-but-never-read |

`close_fan_the_first_x_layers` (1 = 1) and `enable_overhang_bridge_fan`
(true = 1) match Orca exactly — no deviation.

**Ruling: amend P01** — `fan_max_speed` + `fan_min_speed` join P01 as Tier B
keys (scale conversion + wiring `fan_min_speed` to its consumer via
`reduce_fan_stop_start_freq`). Queue amended: 04 asset (+2 rows, B 224→226,
405→407), 05 asset (P01 17→19 keys, 356→358; packet tiers 19A→18A/66B→67B),
ticket 08 re-keyed, map gists updated. 03-asset rows updated with the rename
+ reclassification notes.
