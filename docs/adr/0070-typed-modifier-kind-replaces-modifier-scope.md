# ADR-0070 — Modifier kind is typed; `ModifierScope` is removed

Status: **Accepted.** Approved in the config-scope design interview; not yet
implemented. Supersedes the "extend `ModifierScope` beyond `AllFeatures`" future
work named in ADR-0030.

`ModifierVolume` carries its **modifier kind** as a typed field across the IR seam.
Today the loader parses a typed `PartSubtype` in `crates/slicer-model-io/src/sidecar.rs`,
stringifies it into `config_delta.fields["subtype"]` in `resolve_object`, and ten
production sites across `slicer-core` and `slicer-runtime` re-derive the
classification by string comparison. Typing it means a new kind is a compile error
at each site rather than a silent skip.

A modifier's settings route through the **config schema registry** like any other
scope. `stamp_modifier_sub_region_configs` currently copies every delta key straight
into `ResolvedConfig.extensions` without calling `apply_cli_key`, so a region
modifier is the one scope whose values are never type-checked or bounds-checked, and
are invisible to any host code reading the typed field. Routing them through the
registry also brings them under **scope eligibility** (ADR-0069).

`ModifierScope` is deleted rather than wired. It is written at five construction
sites, always as `AllFeatures`, and read by no production code. Its `LayerHeight`
variant is superseded by **layer range** becoming a real config scope. Its
`Infill` / `Perimeters` / `Support` variants restrict which feature classes a
modifier's settings reach — a genuine capability, but one already expressible by
which keys the modifier states, now that those keys are typed and eligibility is
declared per key. Keeping an inert field that promises restriction and delivers
none is the worse of the two.
