# 103 — Rename fuzzy-skin keys to Orca names

Type: task
Status: resolved
Assignee: wayfinder session (ses_faa7d4541ffeVQyqB0Ub3XdfOD) — claimed 2026-08-30
Blocked by: —
Map: ../map.md

## Question

Standardise fuzzy-skin's config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 5 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) |
|---|---|
| `thickness` | `fuzzy_skin_thickness` |
| `point_distance` | `fuzzy_skin_point_distance` |

Owner: `modules/core-modules/fuzzy-skin`. Adjudication source rows: 03-asset-scoped-gap.md (`fuzzy_skin_thickness`/`fuzzy_skin_point_distance` — exact).

Note: `thickness` and `point_distance` are the bare, non-namespaced names ticket 07 flagged as internal inconsistency — there is no namespacing convention protecting them in the shared config space. The rename to `fuzzy_skin_*` closes that hole. `apply_to_all` stays untouched (Pinch-specific, 03's 34).

Obligations:

- Rename in the module manifest `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points) and tests. Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences. Pay special attention to `thickness` and `point_distance` as generics: any other module or crate that reads them under those names must be re-pointed or is out of this ticket's scope (verify, don't guess).
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the two rows in `03-asset-scoped-gap.md` to record old → new name.
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the two renames are merged, the tree is green on the gates above, and the 03 rows are updated.

## Resolution (2026-08-30)

Committed as `1be6a5af`.

- Renamed `thickness` to `fuzzy_skin_thickness` and `point_distance` to
  `fuzzy_skin_point_distance` across the fuzzy-skin manifest, module fields and
  config reads, native/wasm parity fixtures, module tests, and the SDK worked
  example. No aliases remain. Whole-tree audit found no residual live config-key
  occurrences; the remaining bare `thickness` JSON fields belong to the
  unrelated Arachne beading model.
- Aligned defaults to canonical OrcaSlicer by user ruling:
  `fuzzy_skin_thickness` 0.3 → 0.2 and `fuzzy_skin_point_distance` 0.5 → 0.3.
  This also reconciles the latter's pre-existing `FuzzySkinModule::from_config`
  fallback (`modules/core-modules/fuzzy-skin/src/lib.rs`) of 0.8 with both the
  manifest and canonical default. The
  `empty_config_uses_canonical_fuzzy_skin_defaults` regression test
  (`modules/core-modules/fuzzy-skin/tests/fuzzy_skin_tdd.rs`) pins empty-config
  output to explicit 0.2/0.3 output.
- Regenerated `docs/15_config_keys_reference.md`; the deviation table returned
  to its prior baseline, so no intended-deviation sign-off was needed. Updated the two
  adjudication rows in `03-asset-scoped-gap.md` and the fuzzy-skin worked example
  in `docs/05_module_sdk.md`.
- Rebuilt all guest artifacts; `cargo xtask build-guests --check` returned exit
  0. Validation passed: fuzzy-skin tests, both filtered runtime fuzzy-skin parity
  contract tests, `cargo xtask gen-config-docs --check`, `cargo xtask
  check-literals`, `cargo check --workspace --all-targets`, and `cargo clippy
  --workspace --all-targets -- -D warnings`.
- Per the user's explicit closure ruling, the prohibited-by-default full
  workspace test suite was not run; focused tests plus workspace all-target
  check/clippy are the acceptance evidence for this ticket.
