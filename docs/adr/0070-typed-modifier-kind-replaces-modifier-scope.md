# ADR-0070 — Modifier kind is typed; `ModifierScope` is removed

Status: **Implemented** (TASK-569, config-scope-resolution packet 08 —
documentation closure). Approved in the config-scope design interview.
Supersedes the "extend `ModifierScope` beyond `AllFeatures`" future work named
in ADR-0030.

`ModifierVolume` carries its **modifier kind** as a typed field across the IR seam.
The loader maps each typed `PartSubtype` to `ModifierKind` once when constructing
`MeshIR`; it no longer round-trips the kind through `config_delta.fields["subtype"]`.
Ten production sites across `slicer-core` and `slicer-runtime` match `ModifierKind`
exhaustively, so adding a kind is a compile error at each route rather than a silent
skip.

A modifier's settings route through the **config schema registry** like any other
scope. `stamp_modifier_sub_region_configs` resolves modifier deltas through the
registry before stamping them into `ResolvedConfig`, so modifier values receive
type and bounds validation and are available to host code through typed fields.
Modifier deltas are also checked for **scope eligibility** (ADR-0069).

`ModifierScope` was deleted rather than wired. It was written at five construction
sites, always as `AllFeatures`, and was read by no production code. Its `LayerHeight`
variant is superseded by **layer range** becoming a real config scope. Its
`Infill` / `Perimeters` / `Support` variants restrict which feature classes a
modifier's settings reach — a genuine capability, but one already expressible by
which keys the modifier states, now that those keys are typed and eligibility is
declared per key. Keeping an inert field that promises restriction and delivers
none is the worse of the two.
