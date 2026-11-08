# Config padding disclosure boundary and viewer-key gate

Type: task
Status: resolved (2026-10-07; padding is inert in production; two wire-format
defects found and fixed; table audited to Orca defaults; path-coverage tests
and reproducible block artifact added 2026-10-08 — see ## Answer)
Assignee: agpen (current OpenCode wayfinding session), 2026-10-07
Blocked by: 26

## Question

Which rows in `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`)
can be presented as effective PNP settings, and which are only Orca-viewer
compatibility padding? The alias repair in [Config appendix must reflect
resolved settings](26-config-appendix-resolved-settings.md) handles known
one-to-one name differences, but the table still includes padding-only keys
whose values need not describe the slice: for example `detect_thin_wall = 1`
versus `0` in `docs/ORCA_CONFIG_REFERENCE.md`, and enum display names that may
not be Orca's wire spellings. Some legacy table keys have no row in that
reference. The reference itself is an upstream snapshot, not PNP's resolved
configuration.

Audit the remaining padding keys against actual PNP consumers, module-schema
defaults, Orca's config option wire values (where available), and the
`CONFIG_BLOCK` minimum-key viewer gate in `docs/02_ir_schemas.md`. Decide how
to preserve viewer loading without representing a synthesized value as a
setting that PNP actually applied; test the chosen behavior at the generated
block, not by grepping the table. Distinguish same-name raw overrides from
default-only padding and per-object/per-region settings that a global block
cannot encode. Do not infer that a differently printed enum label denotes a
different wire value without checking its parser.

AFK, disclosure/correctness only; no timing runs or performance change.

## Answer

**The premise needed one correction and the audit found two real wire-format
defects.** Measured 2026-10-07 at HEAD, against the pinned
`OrcaSlicerDocumented` checkout (`3e1daccd7c…`, 2026-09-15) —
`src/libslic3r/PrintConfig.cpp` (+`Config.hpp`, `GCode/GCodeProcessor.cpp`) —
and one real OrcaSlicer 2.4.1 output
(`tmp/3dbenchy_ABS_35m51s.gcode`, 638 pairs, 620 loadable).

### 1. `ORCA_CONFIG_PADDING` is inert in every production path

`serialize_config_block` runs the registry projection first and only then the
padding loop, which breaks once the block holds 96 pairs. Production always
passes `Some(&config_block)` (`crates/slicer-runtime/src/run.rs`'s three arms;
the `None` arm exists only for the public test entry points), and that
projection measured **233 pairs** on the wedge job — so the loop never executes
there and the table's values are never disclosed. The 233 decompose exactly as
5 synthetic + 3 `SUPPORT_CONFIG_DEFAULTS` + 8 alias rows + **217
registry-projected**, with no double-counting.

The table *is* still reachable on the test/legacy path, where it fills 56 → 96
and fires 40 rows. So it is not dead code — but its blast radius is test
surfaces, not G-code shipped to a viewer.

### 2. Two real wire-format defects, both fixed at the serializer

**(a) Booleans were rendered `true`/`false`.** OrcaSlicer's
`ConfigOptionBool::deserialize` accepts only `"1"`/`"0"`; every other literal
falls to `Config.cpp`'s substitution branch, where `enum_looks_like_true_value`
is true only for `enabled`/`on` — so a rendered `true` loaded as **`false`**.
Measured on the production block: **23** lines were affected. The damaging half
is every `true`: `detect_overhang_wall = true` and
`support_remove_small_overhang = true` both loaded as `false`, silently
disabling overhang-wall detection and small-overhang support removal in the
viewer's configuration panel. (The `false` rows coincided with the substituted
value by luck, so they were wrong for the right reason rather than harmless.)
`serialize_config_block` now renders the `1`/`0` wire literal (matching
canonical `ConfigOptionBool::serialize`), and both `true`/`false` occurrences
are gone from the generated block.

**(b) Four enum-typed values used PnP-internal spellings.** OrcaSlicer's
`ConfigOptionEnum::deserialize` is a strict `enum_keys_map` lookup, and a miss
*replaces the option with its own default* (`opt->set(optdef->default_value)`),
so a wrong spelling silently discloses a setting the slice did not use:

| key | emitted before | OrcaSlicer wire value |
| --- | --- | --- |
| `wall_sequence` | `InnerOuter` | `inner wall/outer wall` |
| `wall_generator` | `Classic` | `classic` |
| `wall_direction` | `counter_clockwise` | `ccw` |
| `seam_position` (alias of `seam_mode`) | `rear` | `back` |

Two more share the shape but needed the boolean path handled, because PnP
models them as `bool` while Orca declares a `coEnum`:
`dont_filter_internal_bridges` (`false`→`disabled`, `true`→`nofilter`) and
`enable_extra_bridge_layer` (`false`→`disabled`, `true`→`apply_to_all`). All
seven mappings live in one documented `translate_appendix_enum` table; it is a
pure rename, applied **only** at the appendix, so module `ConfigView`s keep
reading the PnP spelling. Verified on the freshly generated production block:
**0 strict wire violations** (bool literals and enum domains), against 23 + 4
before.

### 3. The padding table itself, audited row by row

- **10 rows carried a value that is not OrcaSlicer's upstream default** — 7
  numeric/bool values (`detect_thin_wall` `1`→`0`, `resolution` `0.012`→`0.01`,
  `wipe_on_loops` `1`→`0`, `reduce_infill_retraction` `1`→`0`,
  `slow_down_layer_time` `8`→`5`, `fan_cooling_layer_time` `100`→`60`,
  `reduce_fan_stop_start_freq` `1`→`0`) and 3 enum domains
  (`top_surface_pattern` `monotonic`→`monotonicline`, `sparse_infill_pattern`
  `grid`→`crosshatch`, the `ipCrossHatch` default the reference displays as
  "Cross Hatch", and `fuzzy_skin` `none`→`disabled_fuzzy`, upstream's
  `FuzzySkinType::Disabled_fuzzy`). The edited table compares **0 mismatches**
  against the pinned defaults, and every one of its 16 enum-typed rows is inside
  OrcaSlicer's `enum_keys_map` domain.
- **Four rows fabricated settings PNP does not have, in the one class the
  contract forbids.** `docs/02_ir_schemas.md`'s minimum-key rule allows only
  cosmetic keys that "the viewer's `GCodeProcessor` does not feed into
  motion/time computation". Measured against `GCodeProcessor.cpp`, five table
  rows named keys it *does* consume: `printable_height` (the height check),
  `z_offset` (applied to every emitted position), `ooze_prevention`
  (`backtrace_enabled`), `single_extruder_multi_material` (the extruder-offset
  branch), and `spiral_mode` (`m_detect_layer_based_on_tag`). Only `spiral_mode`
  is legitimate — `ORCA_CONFIG_ALIAS_KEYS` renders it from PnP's real
  `spiral_vase` setting, so its value is PnP's, not a fabrication. The other four
  were replaced with `precise_z_height`, `hole_to_polyhole`,
  `staggered_inner_seams`, and `wall_distribution_count` (all real Orca keys at
  their upstream defaults, none viewer-consumed). A new test,
  `padding_never_fabricates_viewer_consumed_keys`, pins the rule.
- **Three rows OrcaSlicer cannot load at all**: `support_material` and
  `extra_perimeters` are absent from the pinned source *and* from OrcaSlicer
  2.4.1 output; `wipe_tower_width` survives only as a `handle_legacy` rename to
  `prime_tower_width`. The first two were removed (`wipe_tower_width` is
  retained and documented). Because `set_deserialize` *ignores* unknown keys
  and does not count them toward the `>= 80` gate, such rows padded the line
  count without buying the loading behavior the padding exists for.
- **Four rows named keys that do not exist in OrcaSlicer**
  (`outer_wall_direction`, `infill_first`, `solid_infill_filament`,
  `top_fill_pattern`) and were replaced with real cosmetic keys at their
  upstream defaults (`solid_infill_direction`, `is_infill_first`,
  `top_solid_infill_flow_ratio`, `top_surface_density`).

The table is now 68 rows; **every row names a key OrcaSlicer's loader can
reach**, pinned by a new test that reads the in-repo reference snapshot.

### What was *not* changed, and why

- **Registry-projected values stay registry-driven.** 33 table rows name a key
  the registry also projects; where they disagree (`brim_width` `0` vs the
  resolved `8`, `skirt_loops` `1` vs `6`, `wall_generator` `arachne` vs the
  resolved `classic`, …) the table value is unreachable in production by
  construction — the projection always wins and the loop never runs. Rewriting
  them to mirror one machine's resolved defaults would encode a *lie of a
  different kind*: a `ResolvedConfig::default()` projection is not the job's
  configuration either. The honest fix is the one made: the table is a
  fallback for callers with no resolved config, so it holds upstream defaults
  and nothing claims to be PNP's applied value.
- **Per-object/per-region settings remain inexpressible.** A global block
  cannot encode them — [Config appendix must reflect resolved settings](26-config-appendix-resolved-settings.md)
  already recorded that boundary; nothing here narrows or widens it. The five
  keys whose *module* declarations disagree on a default
  (`inner_wall_speed`, `ironing_speed`, `line_width`, `outer_wall_speed`,
  `retract_length`) are all already registry-projected, so the global block
  carries the deterministically-selected default rather than a fabricated one.
- **`detect_thin_wall` is a genuine PNP-vs-Orca default divergence, not a
  padding lie**: `classic-perimeters` declares `true` where upstream is
  `false`. That is the module's applied default and is disclosed through the
  registry projection; the padding row now matches upstream, and the
  divergence itself belongs to the deviations table, not this ticket. The
  production block measured `detect_thin_wall = 0` because
  `com.core.arachne-perimeters` sorts before `com.core.classic-perimeters` and
  supplies the winning default — a registry-selection question, not a
  disclosure one.

### Residual / still open

- **The padding block cannot satisfy OrcaSlicer's `>= 80` gate on its own.**
  Measured post-fix: the legacy fallback block is 96 pairs of which **72** are
  loadable, and the bare (empty-map) block is 76 pairs, **76** loadable. Both
  fail the gate. The remaining unloadable lines are PNP's own vocabulary
  (`wall_count`, `infill_angle`, `thickness`, `gcode_resolution`, the
  `*_holder` keys, …) plus `min_segment_length`; they are registry-projected or
  emitted from `resolved_config_to_map`, not from the table, so this is **not**
  a table defect and no table edit can fix it. The production path passes the
  gate (161 of 233 loadable). Closing this belongs to a decision about whether
  a resolved-config-less caller should emit an Orca-shaped block at all — ruled
  **out of scope** on the map (it is past this destination: no measured cell's
  disclosure depends on it), not silently fixed here.
- **`infill_anchor = 400%` and `support_interface_bottom_layers = 2`** are
  flagged by a naive enum check but are **not** violations: both are
  `coFloatOrPercent`/`coInt` whose `enum_values` are a GUI suggestion list, not
  a parser domain (`infill_anchor`'s `400%` *is* upstream's default).

No behavior, geometry, timing, or adoption change: the edit is confined to
`serialize_config_block`, its alias renderer, and one new translation helper.
No guest source was touched; `cargo xtask build-guests --check` exits 0. The
`prepass-layer-planning-guest` staleness warning clippy prints pre-dates this
session (its source carries an October mtime; no guest source in the diff): it
is emitted by `crates/slicer-wasm-host/build.rs::main`, which compares the
staged in-tree copy's mtime against its sources — the documented in-tree vs
shared-target fingerprint divergence — and is silenced by refreshing the
staged guests (`cargo xtask build-guests --force`), not by editing sources.

**Verification.** Full batch re-run 2026-10-08 (post-amendment). `cargo test -p
slicer-gcode --lib` **26/26** (the session added two path-coverage tests:
explicit bool literals `1`/`0` on the rendered block, and end-to-end enum
canonicalization including the `seam_mode` alias; both were falsified by
mutating the serializer back to the pre-fix rendering and confirmed red before
restoration). Every suite below was also re-run and reproduced its number; the
ticket's original run was 24/24 (the two added tests account for the delta):
`cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd`
25/25; `--test integration gcode_flavor_config_block_tdd` 2/2;
`--test e2e resolved_config_view_no_drop_tdd` 4/4 (this test's
registry-projection assertion had encoded the old `false` literal and was
updated to normalize the registry text to the wire literal);
`--test e2e slice_end_to_end_tdd` 42/42;
`--test e2e slicing_precision_integration_tdd` 4/4;
`--test e2e modifier_infill_tdd` 2/2;
`--test e2e slicing_promotion_e2e_dispatch_regression_tdd` 3/3;
`cargo test -p slicer-config` all green (99 passed);
`cargo clippy --workspace --all-targets -- -D warnings` clean;
`cargo xtask check-literals` 0 violations;
`cargo xtask check-test-quality --report` 0 findings.

**Boundary.** This ticket resolves the disclosure question for the padding
table and the two wire-format defects it exposed. It does **not** author,
time, or approve any performance candidate; no cell moved.

**Reproducibility.** `evidence/t34-config-block/` carries the regeneration
path: `generate-and-verify.py` (run against a fresh `pnp_cli slice` output and
the pinned checkout at `3e1daccd7c…`) plus the generated
`production-block.txt` and `verification.json` from a 2026-10-08 re-run. It
reproduces **233 pairs, 0 strict wire violations, gate satisfied**, and parses
enum tokens live from the pinned `s_keys_map_*` tables (never the reference's
GUI-label column). One measured delta it discloses rather than adopts: this
method measures **157 of 233 loadable**, not the 161 the ticket body records;
the same method reproduces the ticket's capture figure exactly (620 loadable in
`tmp/3dbenchy_ABS_35m51s.gcode`), so the four-key difference is a measurement
gap between the two takes, not a gate failure (both clear `>= 80`).

