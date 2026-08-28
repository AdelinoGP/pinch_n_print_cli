# 100 — Rename wipe-tower keys to Orca names

Type: task
Status: resolved
Assignee: wayfinder session (2026-08-28)
Blocked by: —
Map: ../map.md

## Question

Standardise wipe-tower's config vocabulary to OrcaSlicer's names (ticket 07 ruling — workstream ticket 2 of 9). Rename, don't alias:

| Pinch key (today) | Orca key (adopt) |
|---|---|
| `bed_shape` | `printable_area` |
| `wipe_tower_purge_volume` | `prime_volume` |
| `wipe_tower_enabled` | `enable_prime_tower` |
| `wipe_tower_width` | `prime_tower_width` |

Owner: `modules/core-modules/wipe-tower`. Adjudication source rows: 03-asset-scoped-gap.md (`printable_area`/`prime_volume`/`enable_prime_tower`/`prime_tower_width` — exact; `bed_shape` is Slic3r's legacy name that Orca itself renamed).

Obligations:

- Rename in the module manifest `[config.schema]` **and every read site** (typed fields, `ConfigView::get_*`, decision points) and tests. Grep the whole tree (`modules/`, `crates/`, `xtask/`, `docs/`, `resources/`) for each old spelling before and after — zero residual live occurrences. Watch for the `wipe_tower_*` family: sibling keys with the prefix (e.g. `wipe_tower_speed`, `wipe_tower_brim_*` if any) that are NOT in this ticket's list must stay untouched — only the four rows above rename.
- Keep defaults, ranges, and behaviour byte-identical; this is a pure rename.
- Green tree before close: `cargo xtask gen-config-docs` regenerated and `--check` passing, `cargo xtask build-guests --check` (guest/wasm rebuild), workspace tests.
- **Triage the deviation table:** renamed keys now match the reference by name — any newly-appearing "Deviations from OrcaSlicer" row for these keys where Pinch's default ≠ Orca's is a real finding: report it to the map or record as an intended deviation with the human's sign-off per ticket 02.
- Update the four rows in `03-asset-scoped-gap.md` to record old → new name.
- Ledger facts re-derived from disk at edit time, never frozen.

Resolved when: the four renames are merged, the tree is green on the gates above, and the 03 rows are updated.

## Answer

All four renames merged. **The rename was not mechanical** — `printable_area`
turned out to be a value-format change, and the deviation gate meant to catch
such things was blind to booleans. Both were resolved with user rulings.

### The four renames

`bed_shape` → `printable_area`, `wipe_tower_purge_volume` → `prime_volume`,
`wipe_tower_enabled` → `enable_prime_tower`, `wipe_tower_width` →
`prime_tower_width` — manifest, module source, module tests, `slicer-ir`
(`ResolvedConfig` field + CLI key string), `slicer-gcode`, `slicer-model-io`,
`slicer-runtime` tests + the `benchy_4color.config.json` fixture, `pnp-cli`
visual-debug, and docs 15/19. Zero residual live occurrences of the old
spellings; `docs/07`, `docs/spec_packets/_OLD/`, and `01-asset-gap-inventory.md`
keep theirs as historical records.

Side effect: `enable_prime_tower` is now a **declared** key, so Orca 3MF project
settings route it to the typed extractor instead of `extensions`. The "keys this
port does not declare" assertion in `slicer-model-io`'s `loader.rs` dropped it
accordingly — it is already covered by the declared-bool list above it.

### `printable_area` is a value-format divergence, not a rename (user ruling: add the adapter)

Orca serialises `printable_area` as a list of **point strings** —
`["0x0", "250x0", "250x210", "0x210"]` — while this port models the bed as an
interleaved `[x0, y0, x1, y1, ...]` float list. Adopting the name alone made PnP
claim Orca's key while rejecting Orca's values: `support-preview` on
`resources/bridge_support_enforcers.3mf` failed with
`config key 'printable_area[0]': expected Float value, got String`, taking 4
`support_preview_tdd` tests with it. Isolated by temporarily renaming only this
key back — the other three coerce cleanly from the same sidecar.

Fixed by widening the input rather than changing the representation:
`slicer_ir::parse_orca_point_string` plus expansion in `extract_float_list`, so a
point string contributes *two* entries. Plain numeric strings still resolve
one-per-entry, leaving Orca's `coFloats` lists (`filament_density`) unaffected.
wipe-tower had **two** read sites for this key — `from_config` and
`run_finalization` — and the second would have silently kept falling back to a
250×250 default bed; both now share one `float_list_from_config` helper.

Regression tests: 4 in `slicer-ir` (`orca_point_string_tests`) and 2 in
wipe-tower's `bed_bounds_tdd`. The negative one is load-bearing — a 60 mm tower
at x=10 on a 50 mm point-string bed must be *rejected*; had the point strings
been dropped, the 250×250 fallback would accept it.

### Deviation triage

The rename made these keys comparable by name for the first time.

| key | Pinch (was) | Orca | outcome |
|---|---|---|---|
| `prime_volume` | 10.0 | 45 | **adopted Orca's 45.0** (user ruling) — manifest, `from_config` fallback, and `from_config_defaults`' assertion |
| `enable_prime_tower` | true | 0 | **adopted Orca's `false`** (user ruling) |
| `prime_tower_width` | 60.0 | 60 | matches; no deviation |
| `printable_area` | — | polygon | not numerically comparable; handled as the format divergence above |

### The deviation gate was blind to every boolean in the tree

`gen_config_docs.rs`'s `num_of()` returned `None` for `toml::Value::Boolean`, so
**no boolean default anywhere had ever been compared** against the reference —
the gate silently under-reported. Ticket 99's "`enable_overhang_bridge_fan`
(true = 1) matches Orca exactly — no deviation" was right by luck, not by check:
that comparison never ran.

Fixed with `default_num_of()`, which widens `num_of` with booleans as 1/0 to
match the reference's `coBool` spelling. Ranges keep using `num_of` — `min`/`max`
are never boolean. Regression test `deviations_flag_boolean_mismatch` pins both
the mismatch and the agreeing-boolean case.

That surfaced **8 boolean deviations**, triaged in this session on the user's
ruling. Six were aligned to Orca:

| key | owner(s) | was → now |
|---|---|---|
| `enable_prime_tower` | wipe-tower | true → **false** |
| `enable_support` | traditional-support, traditional-support-planner, tree-support, tree-support-planner | true → **false** (the two planners' code fallbacks said `true` while the other two said `false`; all four now agree) |
| `detect_thin_wall` | classic-perimeters | true → **false** (arachne already declared `false`) |
| `slowdown_for_curled_perimeters` | overhang-classifier-default | false → **true** |

The seventh and eighth — `precise_outer_wall` in both perimeter modules — were
**not** flipped. Orca defaults it to 1, but turning it on made
`classic-perimeters` emit the inner loop first and shift every perimeter index by
one: `concave_region_emits_outer_wall_without_panic` failed with "first wall must
be the outer loop" (left `Inner`, right `Outer`) and `inner_walls_correct_type`
with "Wall 1 should have perimeter_index 1" (left `2`, right `1`). Reverting only
this key returned all 24 `classic-perimeters` tests to green, isolating it as the
sole cause. PnP's precise-outer-wall path changes wall *ordering* where
canonical's changes only spacing, so aligning the default means fixing that
defect first — packet work, not a default change. Recorded as **DEV-158**
(number re-derived from disk at write time); its two rows stay in doc 15's table.

Tests were updated to track the new defaults, not weakened:
`manifest_default_reconcile_tdd`'s hand-maintained table (it asserts manifest
default == code fallback), and `integrated_parity_support_planner_tdd`, whose
empty `ConfigView` made the native-vs-wasm comparison vacuous once supports
defaulted off — it now sets `enable_support` explicitly, the same trap its own
`support_type` comment already describes.

### Map wiring corrected (user ruling: gate by owner)

The map's Notes claimed the rename workstream "gates the queue", but only ticket
08 carried the blocks — tickets 09–98 listed `Blocked by: 06`, which is resolved,
putting all 90 packet tickets on the frontier. A session taking "the first
frontier ticket in order" would have authored P02 (owner wipe-tower) *before*
this ticket renamed wipe-tower's keys.

67 tickets re-wired to gate on the rename tickets touching their owner:
wipe-tower packets → 100; emitter → 101, 107; classic-perimeters → 102, 107;
fuzzy-skin → 103; support-planner / layer-planner / tree-support → 104;
infill-modules → 105, 107; ironing → 106; arachne → 107; config-resolution →
104, 105; seam-placer → 102. Ticket 08 narrowed from all nine to just 99
(part-cooling), so **P01 is now the unblocked queue head**. 20 packets carry no
rename gate at all (skirt-brim, tool-ordering, slice-prepass,
object-level-planning, bridge-over-infill, print-orchestration, host-export, and
the six new-module packets).

### Gates

- `cargo xtask gen-config-docs --check` — OK (260 module keys, 54 host keys, 27
  Orca deviations).
- `cargo xtask check-deviations --check` — OK (doc 07 Open Deviation Map matches
  `DEVIATION_LOG.md`, 52 open).
- `cargo xtask build-guests --check` — OK (44 guests).
- `cargo test -p slicer-runtime --test contract` — 295 passed, 0 failed.
- `cargo test --workspace --no-fail-fast` — green except **16 failures confirmed
  pre-existing** by stashing this work and re-running on a clean tree: 13
  `slicer-sdk` doc-tests (`missing field 'order_lock' in initializer of
  ExtrusionPath3D`), `runtime_wiring_tdd::config_schema_json_matches_documented_shape`
  (`schema_version` is `1.1.0`, the test pins `1.0.0`), and 2
  `manifest_ingestion_tdd` cases ("expected 22 core modules, got 23").

### Note on ticket 99's "all guests stale" report

Reproduced and explained. The parity harness
(`tests/common/integrated_parity_harness.rs`) calls an artifact **stale** when
the newest source mtime exceeds the artifact mtime, while `build-guests --check`
uses a different criterion and passed at the same moment. Any operation that
rewrites source files without changing their content — a `git stash push` /
`pop`, a branch switch — trips the harness but not the gate. Rebuilding the
guests clears it. Not a defect in the renames, and not a property of the guests
themselves.
