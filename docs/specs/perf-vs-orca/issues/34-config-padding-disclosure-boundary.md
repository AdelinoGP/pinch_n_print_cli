# Config padding disclosure boundary and viewer-key gate

Type: task
Status: open
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
