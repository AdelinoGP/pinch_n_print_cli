# 105 — Rename host and infill-angle keys to Orca names

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa67397f7ffeFA39uExSribFbV) — claimed 2026-08-31
Blocked by: —
Map: ../map.md

## Question

Standardise host and infill config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 7 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) | owner |
|---|---|---|
| `infill_angle` | `infill_direction` | gyroid-infill + rectilinear-infill |

**Scope note (re-adjudicated 2026-08-31):** the original second row
`gcode_resolution` → `resolution` was **re-judged a gap, not a rename** — the
adjudication source row (03-asset-scoped-gap.md) is updated accordingly.
Canonical `resolution` is a generation-time **global** simplification key
(`PerimeterGenerator.cpp` `ex.simplify_p`, `Brim.cpp`, `Fill.cpp`, `Layer.cpp`,
`PrintObjectSlice.cpp`, `Print.cpp`, `TreeSupport`), the host's `gcode_resolution`
is emit-time and per-role (`tolerance_for_role`, `crates/slicer-gcode/src/serialize.rs`):
different decision points. `resolution` now rides queue packet **P51**
(04/05 assets updated); `gcode_resolution` stays PnP-specific, unrenamed.
Adjudication source row: 03-asset-scoped-gap.md (`infill_direction` — exact, verified).

Obligations:

- Rename in the owner manifests `[config.schema]` **and every read site** (typed
  fields, `ConfigView::get_*`, decision points) and tests. Grep the whole tree
  (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for the old spelling
  before and after — zero residual live occurrences. The community-module
  dragon-curve example mirrors rectilinear's spellings by design
  (`mirroring rectilinear-infill's exact snake_case spellings`) — it follows
  the rename.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild; guests embed config key names), workspace tests, and the relevant lock tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the row in `03-asset-scoped-gap.md` to record old → new name.
- **Blocks ticket 107** (the infill-duplicate collapse also edits gyroid-infill + rectilinear-infill).
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the rename is merged, the tree is green on the gates above, and the 03 row is updated.

## Answer

Resolved 2026-08-31 on a split scope — **one rename merged, one adjudication corrected**.

### Re-adjudication: `resolution` / `gcode_resolution` is a gap, not a rename (user ruling)

The scope-note investigation (initiated by the human challenging the "exact" row) verified against
`OrcaSlicerDocumented/` that the two are **different decision points**: canonical `resolution`
(default 0.01) is a **generation-time global** simplification key — `PerimeterGenerator.cpp`
`ex.simplify_p(m_scaled_resolution, …)`, `Brim.cpp`, `Fill/Fill.cpp`, `Layer.cpp`/`LayerRegion.cpp`,
`PrintObjectSlice.cpp`, `Print.cpp`, `TreeSupport3D.cpp`/`TreeSupportCommon.hpp`, plus emit-side arc
density in `GCodeWriter.cpp` — while the host's `gcode_resolution` (default 0.0125) is an **emit-time
per-role** D-P tolerance for the wall-family/brim/gap-fill/raft-infill roles only
(`tolerance_for_role`, `crates/slicer-gcode/src/serialize.rs`). The host has no generation-time
simplification keyed to it and no single global key: renaming would have claimed parity the host does
not implement ("present-in-name, unimplemented-in-behaviour" — the ironing class of finding).

**Recorded by user ruling**: the 03 row is reclassified out of the rename pool into the gap set
(03's 414 → 415, queue target 406 → 407); `resolution` is tiered **B** in 04 and added to packet
**P51 — Quality / Precision — emitter** (2 keys) in 05; `gcode_resolution` stays PnP-specific,
**unrenamed** (default 0.0125 untouched — the earlier in-session alignment ruling to 0.01 was
withdrawn with the rename, since a PnP-specific key has no canonical default to align to; deviation
table stays **27 rows**). `docs/15` regenerated with no `resolution` deviation row.

### Rename: `infill_angle` → `infill_direction` (verified exact, merged)

Canonical `infill_direction` (coFloat 45) is read at the same decision point the host's
`infill_angle` feeds (`Fill/Fill.cpp` `calculate_infill_rotation_angle` with
`region_config.infill_direction.value` → the infill modules' base rotation). Pure rename,
**defaults byte-identical (45.0), zero deviation rows, zero sign-off consumed**:

- `ResolvedConfig` field/key (cli macro `cli "infill_direction"`, `to_config_map`, PartialEq, Hash) —
  `crates/slicer-ir/src/resolved_config.rs`
- Host consumers: `region_mapping::overlay_resolved` (`crates/slicer-core/src/algos/region_mapping.rs`),
  lightning generator angle feed (`crates/slicer-core/src/algos/lightning/mod.rs`)
- Owners: gyroid-infill and rectilinear-infill manifests `[config.schema.infill_direction]`, module
  `config.get("infill_direction")` reads, all module tests (gyroid 23, rectilinear 35 — all green)
- dragon-curve community example (MoonBit, unbuilt here) follows per its stated mirroring of
  rectilinear's spellings — manifest, `main.mbt.in`, `dragon.mbt`, `dragon_test.mbt`, README
- Residual sweep: old spelling survives only in historical records (01 inventory, 03 row
  old→new, spec-packets 227/233, `_OLD/` docs, this ticket) — zero live occurrences

### Gates

gen-config-docs --check OK (260 module / 54 host keys, **27** deviations); check-literals 0;
`check --workspace --all-targets` clean; clippy `-D warnings` clean; slicer-ir 20 binaries green;
slicer-core `--features host-algos` 599 tests green; slicer-gcode 16 binaries green;
slicer-runtime e2e **136/136** (first run's 54 failures were the `pnp_cli is stale` mtime guard —
`crates/pnp-cli-locator/src/lib.rs`, binary rebuilt, rerun clean). build-guests: all 44 rebuilt
(slicer-ir is in every guest's dependency closure — the field rename stales all guests; fingerprint
check exit 0 after). `slicer-sdk --doc` red at HEAD unchanged (pre-existing, flagged to the map).

**Blocks ticket 107** (infill-duplicate collapse edits the same two modules).
