# 104 — Rename support/layer-planner keys to Orca names

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa9db3295ffeuWCZyGeZmZ7yHK) — claimed 2026-08-31
Blocked by: —
Map: ../map.md

## Question

Standardise support and layer-planning config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 6 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) | owner |
|---|---|---|
| `support_top_z_distance_mm` | `support_top_z_distance` | support-planner + tree-support |
| `first_layer_height` | `initial_layer_print_height` | layer-planner-default (+ `crates/slicer-ir/src/resolved_config.rs` — ticket 01 found this key implemented under the Pinch spelling with zero Orca-spelling occurrences) |

Adjudication source rows: 03-asset-scoped-gap.md (`support_top_z_distance`/`initial_layer_print_height` — exact; the `_mm` suffix is a unit-suffix rename).

Obligations:

- Rename in the owner manifests `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points, the `ResolvedConfig` field for `first_layer_height`) and tests. Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the two rows in `03-asset-scoped-gap.md` to record old → new name.
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the two renames are merged, the tree is green on the gates above, and the 03 rows are updated.

## Resolution (2026-08-31)

Committed as `f03d8408`.

- Renamed `support_top_z_distance_mm` → `support_top_z_distance` across the
  traditional + tree support-planner manifests, guest struct fields and
  `config.get` strings, the host `SupportGeometryIR` field
  (`crates/slicer-ir/src/slice_ir.rs`), the `slicer-core` prepass local
  binding, module tests, runtime executor/contract tests, and the
  `orca-matched-config.json` fixture. The Orca spelling already existed
  host-side (`ResolvedConfig` cli field, `host-keys.toml`, prepass consumer,
  3MF loader, gcode serializer); the module-view filter
  (`bind_module_config_view`, exact declared-key match) had kept the two
  disconnected, so the rename also makes one explicit user value reach both
  the prepass and the planner modules. Defaults identical (0.2 = canonical),
  no deviation rows.
- Renamed `first_layer_height` → `initial_layer_print_height` across the
  layer-planner-default manifest + guest, the host `ResolvedConfig` cli field
  and `to_config_map` key, `region_mapping.rs`'s paint-overlay comparisons,
  `slicer-gcode`'s emitter, `run.rs`'s slice-stats derivation, 11
  perimeter-parity fixture JSONs, `orca-matched-config.json`,
  `resources/test_config/gate_evidence_50l.json`, and the affected tests.
  The `slice_stats` event field `first_layer_height_mm` is event-schema
  vocabulary, not a config key, and stays.
- **Deviation-table triage (user ruling in-ticket):** the manifest declared
  `first_layer_height` default 0.3 while canonical, host `ResolvedConfig`,
  and live slice behaviour are all 0.2 (module None-fallback = `layer_height`).
  Aligned the manifest default 0.3 → **0.2** (ticket 102 `wall_loops`
  precedent); doc-only change, no slice behaviour delta. Deviation count
  stays 27.
- Regenerated `docs/15_config_keys_reference.md`; updated the two
  adjudication rows in `03-asset-scoped-gap.md` (old → new + the default
  alignment note) and the live prose in docs 01/02/12 and the
  visual-debug-silhouette plan.
- Rebuilt all 44 guests; `cargo xtask build-guests --check` exit 0. Gates
  green: `gen-config-docs --check` (260 module / 54 host keys, 27
  deviations), `check-literals` 0 violations, `cargo check --workspace
  --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and full test runs of slicer-ir, slicer-core (host-algos),
  layer-planner-default, traditional-support-planner, tree-support-planner,
  slicer-gcode, slicer-model-io, slicer-scheduler, slicer-wasm-host,
  pnp-cli, and slicer-runtime (unit 89, contract 295, executor 323,
  integration 209, e2e 136 — all green). `slicer-sdk --doc` red at HEAD
  unchanged (13 pre-existing `order_lock` doctest failures, flagged to the
  map).
- Unblocks the 13 packet tickets gated on 104 (P11–P13, P29–P31, P68–P70,
  P72, P81–P83).
