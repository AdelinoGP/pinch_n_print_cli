# 101 — Rename path-optimization keys to Orca names

# 101 — Rename path-optimization keys to Orca names

Type: task
Status: resolved
Assignee: wayfinder session (ses_fb4c17ce7ffejbt1HkbP1wJcf2) — claimed 2026-08-28
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

## Answer

All three renames merged in one commit: manifest `[config.schema]` sections
(`modules/core-modules/path-optimization-default/path-optimization-default.toml`),
module fields + `config.get` strings + doc comments
(`modules/core-modules/path-optimization-default/src/lib.rs`), both module test
files, and `docs/02` §per-tool prose. `docs/15`'s module table regenerated via
`cargo xtask gen-config-docs`. Zero residual old spellings under the module
(grep-verified); remaining `retract_length` hits in the tree belong to the
**separate wipe-tower-owned key** (`modules/core-modules/wipe-tower/wipe-tower.toml`
+ the host typed arm in `crates/slicer-ir/src/resolved_config.rs`,
default 2.0, consumed by `retract_length_for_tool` in `crates/slicer-gcode` for
toolchange/purge negative-E) — canonical's toolchange retract is
`retract_length_toolchange` (Tier B, in P36/P43), NOT `retraction_length`, so
that key is out of this ticket's 03 rows by owner.

### Deviation triage (user ruling: align both)

The rename makes `gen-config-docs`' deviation gate newly compare these keys;
canonical `PrintConfig.cpp` says `retraction_length` 0.8 (matched), but
`retraction_speed` 30 (Pinch had 25.0) and `z_hop` 0.4 (Pinch `travel_z_hop`
had 0.0). **User ruling: align both** (ticket 100 precedent over the byte-
identical reading — the alternative P36/P43 queue-carried alignment path was
offered and declined):

- `retraction_speed` 25.0 → **30.0**
- `z_hop` 0.0 → **0.4**, and canonical's declared range `min 0 / max 5`
  adopted on the manifest entry (precedent: wipe-tower's `retract_length`
  carries Orca's [0, 20] the same way)

`retraction_length` stays 0.8 (already matching). Net effect: **no new
deviation-table rows** — the generated block stays at 27 Orca deviation(s).
Behaviour at defaults: `run_path_optimization` now emits ZHop (hop_height 0.4)
between regions and retract/unretract at 30 mm/s (module-side `ConfigView`
fallback constants mirror the manifest defaults).

### False green caught before close (repo-rule §feature-gated/narrow-run hazard)

First `cargo test -p slicer-scheduler --test scheduler_integration` run
reported 12 failures. 10 were harness staleness (`pnp_cli` binary), but 2
were real and pre-existing on HEAD, **not** introduced by the rename:

- `core_modules_directory_is_discoverable_and_all_load` expected **22** core
  modules; HEAD's tree ships 23 (packet 246 added `wave-overhangs`, the test
  count was never bumped). Verified red on stashed HEAD. Fixed here: 23 +
  lineage comment + `com.core.wave-overhangs` in `NON_PLACEHOLDER`
  (`crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`).
- `config_schema_json_matches_documented_shape`
  (`crates/slicer-runtime/tests/integration/runtime_wiring_tdd.rs`) pinned
  `schema_version` "1.0.0"; commit `a50bfc28` (SchemaBridgeMap ticket 02)
  bumped `CONFIG_SCHEMA_WIRE_VERSION` to 1.1.0 and never updated this test —
  red on plain HEAD. Fixed here: pin the constant, assert the new top-level
  `host` array.
- Also pre-existing: `check-literals` 1 violation — exhaustive
  `ConfigFieldEntry` literal in `crates/slicer-scheduler/src/manifest.rs`
  unit test `module_schema_fields_carry_a_preset_scope`; converted to
  `..Default::default()` FRU (the type derives `Default`). Gate now reports
  0 violations.

### Gates

- `cargo xtask gen-config-docs --check` — OK (260 module keys, 54 host keys,
  27 Orca deviations, unchanged count).
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask build-guests --check` — exit 0, 0 stale after full rebuild
  (44 guests). **Correction to ticket 99's recorded claim:** guest artifacts
  *do* embed config key names (byte-search of the pre-rebuild
  `path-optimization-default.wasm` found each old spelling); a rename must
  rebuild guests or typed instantiation runs the old key strings. On the
  tree's mtime-staleness condition (ticket 100): every wide `--check` run
  this ticket ran returned 33 STALE lines until rebuilt because each
  `git stash` cycle touched source mtimes; narrow runs before the rebuild
  were the blind ones.
- `cargo check --workspace --all-targets` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test -p path-optimization-default` — 25/25 across 6 binaries.
- `cargo test -p slicer-scheduler` (unit/contract/integration buckets) —
  24 + 42 + 81, 0 failed (after the two pre-existing fixture repairs above).
- `cargo test -p slicer-ir`, `-p slicer-gcode`, `-p slicer-sdk --tests`,
  `-p slicer-wasm-host`, `-p slicer-runtime` (unit/contract/executor/e2e
  buckets) — all `test result: ok`, 0 failed. `slicer-sdk --doc` remains red
  at HEAD (13 doc examples missing `ExtrusionPath3D.order_lock`, packet
  25398ebf's field; out of this ticket's scope — flagged to the map).
- Host keys: none — `docs/config/host-keys.toml` carries no
  `retract_length`/`retract_speed`/`travel_z_hop` rows; the host typed arm is
  the wipe-tower-owned key and stays.

03-asset-scoped-gap.md rows updated with rename + default-alignment notes.
