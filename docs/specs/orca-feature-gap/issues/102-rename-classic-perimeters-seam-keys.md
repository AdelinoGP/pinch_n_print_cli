# 102 — Rename classic-perimeters and seam keys to Orca names

Type: task
Status: resolved
Assignee: wayfinder session (ses_faf34295dffe6k8VXja3tV2zfB) — claimed 2026-08-29, re-claimed 2026-08-30 (continuation)
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

## Resolution (2026-08-30 — continuation session re-claiming the 2026-08-29 claim)

Merged as `3904c361` (renames + default alignment) and `4cdf5692` (contract fix + test baselines). All obligations above are met.

- **The three renames** are applied across the owner manifests `[config.schema]`, module struct fields and `config.get` strings, doc comments, host typed fields (`wall_loops` in `ResolvedConfig`), the region-overlay, scheduler manifest parsing doc-comments, all tests, resources/fixtures, docs 01/05/15 (including the generated module tables and the `seam_position` values section), and the seam-placer accessor `seam_mode()` → `seam_position()`. Zero residual live occurrences of the old spellings (whole-tree grep; remaining hits are the distinct `tree_support_wall_count` / `base_wall_count` / `SupportPlanSkeleton.wall_counts` identifiers and historical tracker rows).
- **Defaults aligned to Orca per user ruling in-ticket (100/101 precedent):**
  - `wall_loops` 3 → **2** — matches Orca's `wall_loops` (coInt 2). At HEAD the manifests said 3 while the host `ResolvedConfig` field already defaulted to **2**, so the tree was internally inconsistent before this work; the reconcile-test transcription row had drifted the same way. Behaviour at defaults: one outer + one inner wall per contour.
  - `small_perimeter_threshold` 0.8 → **0.0** — Orca coFloat 0 = "no threshold effect"; the narrow-island narrower-inner-width override is now **off** at defaults on live slices (schema-default injection, packet 185 machinery). Range unchanged.
  - `seam_position` default `aligned` already matched Orca (`spAligned`, ADR-0046 amendment). Deviation table stays at **27 rows** — both realigned keys now match upstream, no new deviations, no sign-off rows.
- **Root-cause fix the rename forced (pre-existing latent defect):** the wasm typed dispatch escalated *every* module error into `LayerStageError::FatalModule`, including `fatal=false`, contradicting the WIT contract (`common.wit` `module-error.fatal` doc; `slicer-sdk` `error.rs`: "If false, host logs and continues") and the seam-placer's own designed code-6 degraded fallback (`seam_degraded_fallback_tdd.rs`). The rename *surfaced* this: the painted-3MF fixture's `seam_position=aligned` never reached a module whose manifest said `seam_mode`, so aligned mode never armed at HEAD; after the rename it does, the placer hits the missing-plan degraded path on painted-variant regions (whose seam-plan entries the prepass plan never covers — plan keys are `(gli≥1, region 0)` only, layer 0 absent), and every painted slice aborted at `Layer::PerimetersPostProcess`. The host now honours `fatal=false`: `log::warn!` + continue, degraded output preserved. Bisect evidence: baseline HEAD ran this fixture clean (0 module errors) only because the key never reached the module; `seam_position=nearest` on the renamed tree was green; `wall_loops` 3 vs 2 was irrelevant to the failure.
- **Contract-test alignment:** the three test-guests' `intentional_error_code` witness channel used `ModuleError::non_fatal` while the macro round-trip tests assert typed `FatalModule` surfacing; flipped to `fatal` so the assertions keep their meaning under the corrected host contract.
- **Baseline updates mandated by the ratified default change:** e2e wedge canary `INNER_LOOPS_PER_OUTER` 2 → 1 (measured 240/240), duplicate `"wall_loops"` entry removed from the required-keys list; executor arachne simple-square 3 → 2 walls; `perimeter_parity` annulus 6 → 4 walls (2 outer + 2 hole); INV-3.4 E-monotonicity given two printed quanta (1e-5) of slack — the measured delta is exactly 2 quanta (`0.73152 → 0.73151`) from f32 length jitter through the now-live aligned-seam projection vertex insertion.
- **Guests:** all 44 rebuilt fresh; `build-guests --check` exit 0 on the committed tree. Guests embed the new spellings (byte-search verified).
- **Gates:** gen-config-docs --check OK (260 module / 54 host keys, 27 deviations, unchanged); check-literals 0 violations; check --workspace --all-targets clean; clippy --workspace --all-targets -D warnings clean; slicer-runtime 18/18 binaries green (e2e 136/136); seam-placer / seam-planner-default / classic-perimeters / arachne-perimeters / wave-overhangs / tree-support / tree-support-planner / slicer-scheduler / slicer-ir / slicer-gcode / slicer-sdk --lib green.
- **Carried to the map:** (1) `slicer-sdk --doc` remains red at HEAD — 13 pre-existing doctest failures missing `ExtrusionPath3D.order_lock` (packet 25398ebf), verified identical on the baseline stash and untouched by this work; (2) the wasm dispatch's degraded-fallback `log::warn!` does not yet increment `SliceEventCollector.non_fatal_error_count` / flip `degraded` — a fully degraded slice still reports `degraded: false`; (3) the seam planner does not cover painted-variant regions (every painted layer logs one code-6 degraded fallback). None block this ticket; all three are observations with no sign-off consumed.
