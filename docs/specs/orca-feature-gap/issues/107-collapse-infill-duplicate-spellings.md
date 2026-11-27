# 107 — Collapse infill duplicate spellings to Orca names

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa5a9480dffeaSQe7D0RI63Ucq) — claimed 2026-08-31, resolved 2026-09-01
Blocked by: 102, 105
Map: ../map.md

## Question

Collapse the three duplicate spellings to the Orca names (ticket 07 ruling — workstream ticket 9 of 9). Both spellings are live today (03's "not renames — duplicates" row; verified read sites across modules and crates), so this is a delete-the-Pinch-duplicate rename, not a documentation fix:

| Pinch duplicate (drop) | Orca key (keep) |
|---|---|
| `infill_density` | `sparse_infill_density` |
| `infill_speed` | `sparse_infill_speed` |
| `infill_overlap` | `infill_wall_overlap` |

Blast radius (verified by read-site grep at charting time — re-derive): `arachne-perimeters`, `classic-perimeters`, `gyroid-infill`, `rectilinear-infill`, `infill-linker` (crate), `crates/slicer-gcode` (serialize), `crates/slicer-model-io` (+ its TDD fixtures), and tests in several modules. Every module that declared or read the duplicate spelling must converge on the Orca name — where a module declared *both* spellings, drop the duplicate and keep the Orca-named row; where code reads the duplicate, re-point the read.

Obligations:

- For each of the three pairs: the Orca-named key is authoritative (its manifest row, default, and decision point stay); the Pinch-duplicate spelling is deleted everywhere. **No behaviour change**: decision points keep reading the same underlying setting through the surviving spelling.
- Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`, tests) for each dropped spelling before and after — zero residual live occurrences.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing (doc 15 re-keyed; verify no duplicate rows remain), `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** the surviving Orca keys now match the reference by name — any "Deviations from OrcaSlicer" row where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the duplicates row in `03-asset-scoped-gap.md` (record the collapse).
- **Blocked by 102 and 105** — this ticket edits classic-perimeters (also touched by 102) and gyroid/rectilinear-infill (also touched by 105); run after both are merged.
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the three collapses are merged, the tree is green on the gates above, and the 03 row is updated.

## Answer

**Resolved 2026-09-01 — two collapses merged, one pair re-adjudicated (user rulings).**

**Pair 1 — `infill_density` → `sparse_infill_density` (canonical percent everywhere, user ruling).** The Pinch spelling is deleted from every manifest, read site, typed field, and test; the surviving key is percent (20.0 [0,100]) in all five declaring manifests (gyroid/rectilinear/lightning + the pre-existing arachne/classic gate rows). Modules divide by 100 at the read site (`ConfigView::get_abs_value("sparse_infill_density", 100.0)` — extended to resolve percent-form strings, the 3MF-preserved form); per-region reads go through the new `slicer_sdk::config_resolution::resolve_percent_float`. `ResolvedConfig` field/key renamed (default 20.0) with a new `extract_percent_float` input adapter (ticket-100 precedent) so Orca 3MF percent strings resolve instead of aborting the slice — pinned by three new slicer-ir tests. The loader's part-metadata arm now preserves percent strings raw (was: fraction floats); the M3 fixture already carried `"15%"/"40%"` and its density overrides now finally reach the modules (previously inert — the extension key never matched the old `infill_density` reads). The linker's density read is re-pointed + percent-converted and stays undeclared (DEV-114 deferral intact).

**Pair 2 — `infill_speed` → `sparse_infill_speed` (manifests aligned to canonical 100, user ruling).** Renamed across gyroid/rectilinear/lightning (reads, struct fields, manifests — rectilinear's duplicate row deleted), `ResolvedConfig` (field/key/CLI, default 50.0 unchanged — the speed-factor base that yields factor 1.0 = canonical 100 mm/s live), region-mapping overlay, dragon-curve example (manifest + MoonBit glue + README + wasm rebuilt with the pinned toolchain). Host `FeedrateConfig.sparse_infill_speed` (100.0) is the canonical feedrate decision and was untouched. Deviation table 27 → **26 rows** (measured; exactly the rectilinear 60-vs-100 row removed, no new rows).

**Pair 3 — `infill_overlap` re-adjudicated (user ruling): NOT a duplicate.** Canonical `infill_wall_overlap` (coPercent 15, consumed only in `PerimeterGenerator.cpp` as `inset -= infill_peri_overlap`) is already ported in classic-perimeters; the linker's `infill_overlap` (0.45, fraction-of-spacing, infill-side post-pass) is a PnP-invented second mechanism with no canonical counterpart. Kept live; 03's row and the map Notes updated (duplicate-collapse count 3 → 2; rename pool 25 → 24 keys).

**Also in-ticket:** the persistent `slicer-sdk --doc` red (13 `ExtrusionPath3D.order_lock` doctest examples, flagged through tickets 99–106) repaired — the map's fog item is cleared. One latent prepass bug fixed: the `BridgeDepthLayer` density thresholds (0.999) were not converted with the pooling gate (99.9) — caught by `extra_bridge_layer_emission_semantics`.

**Gates (all green):** `cargo check --workspace --all-targets`; clippy `-D warnings`; `cargo xtask check-literals`; `cargo xtask gen-config-docs --check` (26 deviations); `cargo xtask build-guests --check` (44 guests rebuilt twice — slicer-ir sits in every closure); module suites (gyroid/rectilinear/lightning/infill-linker); slicer-ir/sdk/model-io/core(host-algos)/wasm-host/gcode; runtime contract/executor/integration/unit + full e2e (136/136). Residual greps: zero live occurrences of the retired spellings in code/config/resources (two historical packet-doc mentions kept as records). CONFIG_BLOCK composition changed deliberately (`infill_density`/`infill_speed` → `sparse_infill_density`/`sparse_infill_speed`; the `sparse_infill_density = 15%` padding twin is now deduped by the typed 20.0 emission) — the wedge canary's key-count assertion (95) is unchanged and still passes.
