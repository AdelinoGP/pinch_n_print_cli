# 106 — Rename ironing keys to Orca names

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa5da5f11ffeqEInJXdF70MATx) — claimed 2026-08-31
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

## Answer

Resolved 2026-08-31 — **three renames merged, one default aligned by user ruling**.

### Renames (order respected — the two `ironing_spacing` spellings never crossed wires)

- `ironing_flow_rate` → `support_ironing_flow` (support-surface-ironing): manifest
  `[config.schema]`, module field/getter/`config.get`/`flow_factor` feed, in-module
  test, `ironing_tdd.rs`, contract parity test.
- `ironing_spacing` → `support_ironing_spacing` (support-surface-ironing): manifest,
  module field/getter/`mm_to_units` feed/`config.get`, in-module test,
  `ironing_tdd.rs`, `ironing_scanline_parity_tdd.rs`, contract parity test.
- `ironing_spacing_mm` → `ironing_spacing` (top-surface-ironing): manifest, module
  field/getter/`config.get`/zigzag feed, `top_surface_ironing_emission_tdd.rs`,
  contract parity test, `resources/test_config/benchy_combined_feature_evidence.json`
  + the two embedded copies in `slicing_promotion_e2e_dispatch_regression_tdd.rs`.

### Finding: `support_ironing_flow` scale deviation (user ruling — align to canonical)

The rename surfaced a **value-format** deviation (ticket 100's class, not a pure
rename): canonical `support_ironing_flow` is **coPercent, default 10%**,
`ConfigDef.cpp` (`set_default_value(new ConfigOptionPercent(10))`, min 0, max 100),
while the port's `ironing_flow_rate` default **100.0** is consumed as a raw
`flow_factor` multiplier — `crates/slicer-gcode/src/emit.rs`:
`distance * point.width * height_delta * point.flow_factor / filament_area`, whose
own comment pins the expected scale ("1.0 normally; e.g. ~0.1 for ironing"). At
defaults, enabling support ironing emitted **100× nominal flow**. The deviation
gate is blind to it: `orca_defaults` parses the reference's Default column with
`parse::<f64>()`, so `"10%"` never enters the comparison map.

**User ruling: align the default with the rename** — manifest default 100.0 →
**0.10**, range [1.0, 200.0] → **[0.01, 1.0]** (mirrors top-surface-ironing's
`ironing_flow` convention; canonical 10% as a fraction). Behavioural change only
when support ironing is enabled at defaults (previously 100× over-extrusion).
The contract parity test's explicit config value 100.0 → 0.10 (test-only,
canonical-consistent). `support_ironing_spacing` (0.1) and `ironing_spacing`
(0.1) match canonical exactly — no other changes. Deviation table stays **27 rows**.

### Gates

gen-config-docs regenerated + `--check` OK (260 module / 54 host keys, **27**
deviations); check-literals 0; `check --workspace --all-targets` clean; clippy
`-D warnings` clean; support-surface-ironing 16 tests, top-surface-ironing 13;
slicer-runtime unit 89 / contract 295 / executor 209 / integration 323 / e2e
136; slicer-ir, slicer-core `--features host-algos`, slicer-gcode green.
build-guests: all 44 rebuilt (the two ironing guests were the only stale ones —
key names embedded), `--check` exit 0. Residual sweep: old spellings survive
only in historical records (01/03/07 issue files, `_OLD/` docs, spec packets,
this ticket, map notes) — zero live occurrences.

**Unblocks tickets 21 (P14) and 22 (P15)** — both were gated on 106.
