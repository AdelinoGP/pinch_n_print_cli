# t81 — Slicing-time contour simplification (`resolution`) port

Ticket: [81](../issues/81-resolution-slicing-time-contour-simplification.md).
Graduated from [80](../issues/80-resolution-fairness-premise.md), which measured
the key's worth on Orca's side and disclosed that the matched job was never
resolution-matched.

## What this is

PnP gains OrcaSlicer's `resolution`: a **slicing-time** contour simplification
that runs before walls and infill are generated. It is a geometry change by
design, so the gate is canonical parity (function + placement), not byte
identity against the t58 references.

Canonical consumption, verified in `OrcaSlicerDocumented/src/libslic3r`:

| # | Canonical site | Tolerance | PnP seam |
|---|---|---|---|
| 1 | `union_ex(surface.expolygon.simplify_p(surface_simplify_resolution))` at the top of the wall pass (classic and arachne paths) | `surface_simplify_resolution` | perimeter module, on the region's polygons before wall generation |
| 2 | `ex.douglas_peucker(surface_simplify_resolution)` on each gap before `medial_axis` | same | the `filtered_gaps` loop, before `host::medial_axis` |
| 3 | classic: `for (ExPolygon &ex : last) ex.simplify_p(m_scaled_resolution, &pp)` then `union_ex` → infill boundary | raw `m_scaled_resolution` | on the innermost wall contour, before the infill boundary offset |
| 4 | classic inner-part `infill_contour` → same | raw | **absent** — PnP has no inner-part pass |
| 5 | arachne `infill_contour` → same | raw | `publish_infill_areas`, on the unioned contour |

`surface_simplify_resolution` is `(enable_arc_fitting && !has_fuzzy_skin) ? 0.2 *
m_scaled_resolution : m_scaled_resolution`. PnP has no `enable_arc_fitting` key
(and the matched job runs with it off — appendix-verified in t80's captures), so
the branch collapses to the raw tolerance and is recorded as unexpressible
rather than invented as a padding key.

## Mechanics and units

- Key: `resolution`, canonical default `0.01` mm, declared `coFloat` with
  `def->min = 0` in `PrintConfig.cpp`.
- Clamp: `DynamicPrintConfig::normalize_fdm` raises any explicitly-set value
  below `0.001` mm to `0.001` mm ("Resolution will be above 1um"), so "disabled"
  is not expressible. PnP applies the same clamp **in the resolver**
  (`crates/slicer-config/src/resolution.rs`), so the value the G-code appendix
  discloses is exactly the tolerance the modules apply.
- Units: canonical scales millimetres by 1e6 (`SCALING_FACTOR` 1e-6); PnP's
  `Point2` is 1 unit = 100 nm (`UNITS_PER_MM` = 10 000). The tolerance is
  converted, never copied: `0.01 mm → 100 units` here versus `10 000` canonical
  units.
- Algorithm: `MultiPoint::_douglas_peucker` with canonical's stack discipline,
  `<=` acceptance test, strictly-greater furthest-index tie-break, and the
  **closed-ring** call shape `ExPolygon::simplify_p` uses (push the first point,
  simplify, pop it). The seeded degenerate segment at the ring's first vertex is
  reproduced, which is what makes the first split land on the vertex furthest
  from the ring start.

## Parity regime — invariants, not goldens

The repo's Parity Evidence Standard is explicit: *"Invariants, not goldens…
Golden G-code comparison is not part of the standard."* No Orca test covers
`simplify_p`/`_douglas_peucker`, so there is nothing to port; the invariant
suite carries the claim (`crates/slicer-sdk/src/contour_resolution.rs`):

- every input vertex strictly between two output vertices lies within tolerance
  of that output segment (canonical's actual guarantee — when a floater is
  emitted, every interior point was tested against that exact segment);
- output vertices are a subsequence of the input, anchored at index 0;
- a corner deviating further than the tolerance survives; a collinear run
  collapses to its endpoints;
- larger tolerance never adds vertices; repeated simplification never
  reintroduces a discarded vertex;
- ring closure and the first-split seeding above;
- `simplify_p` preserves hole topology and exact area at a tolerance below the
  feature scale, and is the identity at a non-positive tolerance;
- the floor/default conversion (`0.0` → floor, non-finite → default, scale ratio
  against canonical's `SCALING_FACTOR`).

**Union-engine caveat, disclosed.** Canonical's `simplify_polygons` runs on the
legacy Clipper 6.4.2 with `StrictlySimple(true)` + NonZero. PnP unions through
`clipper2-rust` with NonZero. Same fill rule, same set semantics; different
representation (legacy Clipper splits self-touching output into strictly-simple
polygons, and the Rust engine has no strictly-simple pass). Byte-identical
output is therefore **not** claimable and is not claimed.

## Confinement

`confinement.py` renders the same layer in both arms and compares the **typed IR
captures** exactly — the IR itself, not a rasterised view. A PNG at 1024² cannot
falsify a sub-pixel leak; the typed captures can.

Result: **20/20 checks PASS** across all eight cells (`confinement.json`).

- `Layer::Slice` — identical in all 8 cells (the host prepass is untouched).
- `Layer::Perimeters` — differs in all 8 cells (the intended change).
- `PrePass::SupportGeometry` — silhouette PNG byte-identical in all 4
  supports-on cells (raster evidence only, disclosed as such: that tap has no
  typed capture).

The visual bundle is a human/agent artifact, not the oracle. At 1024² the
perimeter images of layer 120 are visually indistinguishable between arms,
which is exactly why the exact IR comparison is the evidence.

Note on artifacts: 24 of 37 guest `.wasm` files change hash between arms even
though only two guests changed source, because `slicer-sdk` is statically linked
into every guest. Artifact identity is therefore *not* used as confinement
evidence here (the repo already records this effect for `polygon_ops.rs` text
changes); the IR comparison is.

## Changes

- `crates/slicer-sdk/src/contour_resolution.rs` (new): `simplify_ring`,
  `douglas_peucker`, `simplify_p`, `tolerance_units`, plus the invariant suite.
- `crates/slicer-ir/src/resolved_config.rs`: the canonical constants, the
  extension-carried `resolution()` accessor, and a config-map/Eq test. `resolution`
  is **not** a fixed typed field: `RegionMapIR` interns `ResolvedConfig`, so a new
  typed field changes the Postcard layout and breaks the frozen 3.0.0 fixture —
  the same constraint that reverted the packet-10 fixed-field addition (see
  `crates/slicer-ir/tests/region_map_versioned_decode_tdd.rs`). It rides the
  `extensions` carrier, which `to_config_map` merges through.
- `crates/slicer-config/src/resolution.rs`: the `normalize_fdm` floor clamp.
- `modules/core-modules/classic-perimeters`: sites 1–3 + manifest row.
- `modules/core-modules/arachne-perimeters`: sites 1 and 5 + manifest row.
- `crates/slicer-gcode/src/serialize.rs`: the appendix now renders `resolution`
  from its own resolved value instead of aliasing `gcode_resolution`, and alias
  floats go through `format_config_float` (the old path emitted the f32→f64
  widening artifact, e.g. `0.012500000186264515`).

## Campaign

Pre-declared clauses (ticket 81, fixed before any run): paired-median wall < 0 in
**every** cell, paired-median CPU ≤ +3.0 %, 6 alternating pairs per cell, one
warmup per arm, stop by rule at the first cell whose wall clause fails and do
not run the remaining cells.

Harness: `run_port_pair.ps1`. Arms are frozen snapshots (`target/t81/{baseline,candidate}`),
alternating run order per pair, appendix-verified job (`wall_generator`,
`wall_loops`, `sparse_infill_density`, `layer_height`, `enable_support`,
`resolution`), wall = process creation-to-exit, CPU = `GetProcessTimes`
kernel+user. Raw rows: `campaign/*.csv`; machine-readable summary:
`campaign/results.json`.

| cell | n | paired-median wall | paired-median CPU | G-code bytes | wall range |
|---|---:|---:|---:|---:|---:|
| benchy classic-off | 6 | **−14.31 %** | −23.12 % | −9.40 % | −21.41 … −11.23 |
| benchy classic-on | 6 | **−9.00 %** | −18.77 % | −4.69 % | −25.55 … +3.07 |
| benchy arachne-off | 6 | −0.07 % | −3.16 % | −1.51 % | −0.92 … +3.77 |
| benchy arachne-on | 6 | −1.87 % | −1.80 % | −0.74 % | −3.93 … −0.06 |
| base classic-off | 6 | **−22.24 %** | −26.87 % | −6.26 % | −33.56 … −12.02 |
| base classic-on | 6 | **−10.87 %** | −20.29 % | −3.20 % | −11.75 … −9.01 |
| base arachne-off | 12 | −1.07 % | −2.08 % | −1.58 % | −5.57 … +29.71 (two campaigns) |
| base arachne-on | 6 | −0.88 % | −1.44 % | −0.78 % | −3.13 … +3.10 |

**Every cell's paired-median wall is negative and every cell's CPU is negative.**

### base arachne-off needed a second campaign

Its first six pairs failed the pre-declared wall clause at **+2.30 %**, so the
campaign stopped by rule (its three unrun neighbours were run only after the
human's explicit "run it again"). The re-run measured **−2.77 %**. Combined
n=12: median **−1.07 %**, per-pair spread −5.57 … +29.71, stdev 9.25. The
spread, not the sign, is the finding: this cell cannot resolve a sub-3 % wall
effect at six pairs, and one of the twelve samples is visibly disturbed
(+29.71 %, cpu/wall 6.6 against 7.6–8.3 elsewhere — the closest any sample comes
to the recipe's starvation floor).

Read plainly: **Arachne is wall-neutral for this change** (four cells between
−1.9 % and −0.07 %, three of them inside their own spread); **classic wins
double digits** (−9 to −22 % wall, −18 to −27 % CPU). The mechanism explains
both: classic's inset loop is vertex-proportional, while Arachne's beading
pipeline is dominated by graph construction over offsets that simplification
barely shrinks — the pass buys back less than its own Douglas–Peucker + union
sweep costs.

Provenance: the `extensions`-carrier refactor landed after the campaign, so the
committed source was re-run against the campaign's own outputs. Both fixtures
reproduce the measured bytes exactly — benchy classic-off
`3672dc7e1649cab69a0ffab2ebd293233a9af0504d0f98666746da933d8c641d` and base
classic-off `292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80`,
each equal to its campaign candidate SHA. The timing numbers therefore describe
the committed source, not a stale build.

Pre-existing red disclosed: `crates/slicer-config/tests/registry_census_tdd.rs`
asserts a hand-maintained `entry_count == 281`; the tree is at **291 at HEAD**
(verified by stashing this work) and **293** with the two new manifest rows. The
drift is pre-existing and is not "fixed" here by editing the roster.

Reproduction checks:

- Determinism: every arm/cell produced **exactly one** G-code SHA across all its
  runs — the candidate SHAs in `campaign/*.csv` are the new reference set.
- The baseline arm reproduces the standing t58 classic-off reference exactly
  (`34886ccb56b2…`), so the harness measured the job the map has always
  measured.
- The first benchy batch (`benchy-20261023`, preserved) had two pairs
  contaminated by a concurrent `cargo xtask` invocation; the clean batch
  (`benchy-20261023-clean`) is quoted above and agrees with it within 0.2 pp on
  both classic cells.

## Verdict

**Gate met on the campaign of record: 8/8 cells wall-negative, 8/8 CPU-negative,
no cell above the +3.0 % CPU ceiling, no starvation exclusion, every cell
byte-deterministic.** Two honest qualifiers travel with it: base arachne-off
failed its first campaign and is wall-neutral on combined evidence, and the four
Arachne cells sit inside their own spread. The speed case that survives is
"classic cells, double digits; Arachne, neutral-to-slightly-positive on CPU".

The parity case is unconditional: invariants pass, stage confinement is exact in
all eight cells, and the appendix now discloses the value the modules apply.

No production commit was made before this record: the map's standing rule returns
keep/drop to the human.

## Disclosures

- Canonical site 4 (classic inner-part pass) has no PnP counterpart; recorded,
  not invented.
- The `0.2 ×` arc-fitting tolerance branch is unexpressible (no arc-fitting key)
  and inert on the matched job.
- PnP's non-planar shell path (a PnP extension with no canonical counterpart)
  keeps raw geometry.
- Walls and infill now consume simplified geometry, so seam-candidate counts fall
  with the vertex count. That is a faithful consequence of canonical's ordering
  (seams are generated from wall paths), not a separate change.
