# Config appendix must reflect resolved settings

Type: task
Status: resolved
Assignee: wayfinder session (ses_f1fc866b9ffelTjvcGPaOc9kbc), 2026-09-26

## Question

The PNP G-code config appendix (`crates/slicer-gcode/src/serialize.rs`'s static
defaults + same-name overlay) prints **static defaults** for keys whose
resolved-config name differs from the appendix key name: `wall_loops` always
prints `2` regardless of `wall_count` (measured 2026-09-22 during the
[Matched-pair rig and first scoreboard](11-matched-pair-rig-and-scoreboard.md)
work: `wall_count` 1 and 3 both printed `wall_loops = 2` while the wall output
changed accordingly — same-name keys like `wall_generator`,
`sparse_infill_pattern`, `support_type` overlay correctly).

The ticket-11 matched job used 2 walls, so its disclosure is coincidentally
correct. Any other wall count silently mis-discloses the settings evidence the
fairness contract ("validate actual generator dispatch per run", labels prove
nothing) relies on.

Work:

- Derive appendix values from the resolved config (map `wall_count` to
  `wall_loops`; audit every static table key for the same name-mismatch lie).
- Add a check that a changed setting shows up in the appendix, so the
  disclosure surface cannot silently rot again.

Disclosure-only: no slicing behavior changes. Parallel-takeable (not a
timing/acceptance ticket).

## Answer

**Resolved for one-to-one setting aliases.** `serialize_config_block`
(`crates/slicer-gcode/src/serialize.rs`) now emits Orca-named keys from their
PNP source keys *before* raw same-name passthrough or cosmetic padding, so
`wall_count = 1` / `3` yields one `wall_loops = 1` / `3` line rather than the
old fixed `2`, even if raw input also supplies a conflicting `wall_loops`.
The audited alias table also covers `infill_angle` → `infill_direction`,
`gcode_resolution` → `resolution`, `enable_support` → `support_material`,
`seam_mode` → `seam_position`, `spiral_vase` → `spiral_mode`, and the two
fuzzy-skin thickness/distance spellings. `resolved_config_to_map` adds the
wall-path `gcode_resolution` to the *appendix-only* map; it does not change
`ResolvedConfig::to_config_map`, which also feeds module views. `resolution`
uses `gcode_resolution`, **not** the separate `infill_resolution` (the mapping
is adjudicated in `docs/specs/orca-feature-gap/issues/03-asset-scoped-gap.md`).

The table audit deliberately did **not** equate the fractional
`infill_density` with the perimeter modules' percent-valued
`sparse_infill_density`, or collapse split `raft_layers` or narrowed
`ironing_type` into one-to-one aliases. The sparse density padding default is
now `20%`, matching both the perimeter-module defaults in
`docs/15_config_keys_reference.md` and Orca's snapshot in
`docs/ORCA_CONFIG_REFERENCE.md`, rather than the prior `15%`.

Regression evidence: `config_block_discloses_resolved_wall_count_under_orca_name`
tests both formerly failing wall counts and a conflicting raw alias;
`config_block_discloses_distinct_resolved_precision_and_infill_direction`
tests changed angle and precision with distinct infill precision;
`config_block_keeps_sparse_infill_density_distinct_from_fractional_infill_density`
pins the unconfigured padding; and
`config_block_uses_resolved_wall_count_and_precision_over_raw_orca_aliases`
(`crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`)
drives the production postpass wrapper. `cargo test -p slicer-gcode --lib`
passed (19 tests), the named runtime integration test passed, and
`cargo check -p slicer-gcode --all-targets`,
`cargo check -p slicer-runtime --all-targets`, and `cargo xtask check-literals`
passed. No guest-dependent or Orca-viewer run was made; no slice timing or
geometry claim follows from this disclosure-only change.

**Boundary:** a global appendix cannot represent per-object/per-region
effective settings, and padding-only keys still need a separate decision on
whether their synthesized values can be read as PNP settings while preserving
Orca's minimum-key gate. See [Config padding disclosure boundary and viewer-key
gate](34-config-padding-disclosure-boundary.md); this ticket does not assert
that every cosmetic padding row describes actual execution.
