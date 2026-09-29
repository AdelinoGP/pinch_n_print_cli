# Classic output-volume attribution (2026-09-29)

Ticket: [Classic output-volume surplus](../../issues/28-classic-output-volume-surplus.md).
This is an **output census**, not a geometry fix or speed A/B.

## Capture and reproducibility

`cargo xtask build-guests --check` exited 0, then
`cargo build --release --bin pnp_cli` succeeded. The current release host ran
`pnp_cli slice --model tmp/{3dbenchy,base}.stl --config
docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-{classic,arachne}-supports-off.json
--module-dir modules/core-modules --output <file>` with
`RAYON_NUM_THREADS=12`: one uninstrumented capture per fixture/generator.
Raw G-code and stdout/stderr logs live under
`target/matched-pair/t28-output-volume/` (gitignored). Every run reported
`slice_complete status=ok, degraded=false, fatal_error_count=0,
non_fatal_error_count=0`; each log also has `ERR_MALFORMED_LAYER_MARKER`
warnings from `machine-gcode-emit` (not investigated here). For the Orca side,
the **retained 2026-09-22** matched supports-off G-code under
`target/matched-pair/s{1-benchy,2-base}/gcode/*-orca-m3.gcode` is a historical
reference, not a same-day Orca rerun. It is already disclosed in the matched
scoreboard. Do not read single-run walls as an A/B or updated scoreboard.

Run `measure_sections.py FILE...` on the eight files listed in `roles.json` to
reproduce the role census; raw G-code is not committed (model licences and
generated output size). Each byte, including type markers, belongs to the
current `;TYPE:` / `; FEATURE:` role, except headers, layer transitions and
trailing PNP config, which are `[untyped]`. The role-byte sum equals the exact
file size for every capture. `sparse_layers.py classic.gcode arachne.gcode`
reproduces the per-layer XY path census in the four CSVs; only positive-E G1
XY moves with a changed XY position count as printed segments. The two tools'
feature labels do not promise equivalent surface classification; role counts
and per-role bytes are directly comparable **within a tool**, not as canonical
geometry equivalence across tools.

## Where the PNP classic-versus-Arachne bytes are

| Measured item (bytes unless labelled) | Benchy classic | Benchy Arachne | Base classic | Base Arachne |
| --- | ---: | ---: | ---: | ---: |
| Whole G-code | 7,444,447 | 4,705,584 | 21,935,986 | 20,994,790 |
| Sparse infill | 4,486,477 | 2,005,148 | 9,431,668 | 9,895,251 |
| Outer wall | 1,398,177 | 1,221,076 | 6,288,351 | 5,364,434 |
| Inner wall | 1,313,447 | 1,243,473 | 6,028,351 | 5,581,346 |
| Gap infill | 30,140 | 0 | 48,639 | 0 |
| Sparse printed XY segments | 106,442 | 47,659 | 225,004 | 236,062 |
| Sparse printed XY length (mm) | 79,939.9 | 36,219.4 | 164,904.6 | 174,242.2 |

**Benchy:** Classic exceeds Arachne by 2,738,863 bytes. Sparse infill alone
accounts for **2,481,329 bytes (90.6% of that delta)** and 58,783 more
positive-E XY segments; the gap-fill section contributes only 30,140 bytes.
This is printed path volume, **not just section markers or a formatting-only
surplus**. At layer index 104 (Z 21 mm), classic records 2,562 sparse segments,
1,741.164 mm XY path and 103,756 section bytes; Arachne records one segment,
1.701 mm and 176 bytes. The historical Orca classic and Arachne captures each
record **zero** sparse path at that layer (Orca classic instead has labelled
gap fill and internal solid infill). Many neighboring layers exhibit the same
shape; see `benchy-sparse-layers.csv` and `orca-benchy-sparse-layers.csv`.
**This suggests a fill-domain/classification discrepancy**, not that the
classic sparse paths are redundant or safe to delete. A layer IR comparison
through the perimeter infill-area and fill-claim boundaries is needed before
changing geometry. Neither generator is presumed correct merely because its
G-code is shorter.

**Base:** Classic exceeds Arachne by 941,196 bytes. Outer+inner wall sections
are 1,370,922 bytes larger, partly offset by classic's **463,583 fewer** sparse
bytes. Classic has *fewer* sparse printed XY segments/length than Arachne on
base. Its gap-fill section is 48,639 bytes. This explains why the Benchy
sparse-dominated surplus does not extrapolate to base: the byte delta is mostly
wall output there. Classic's extra walls and gap-fill are not by themselves
emission-shape waste; the generators draw different wall geometries.

## Cross-tool size has a separate serialization component

The historical matched Orca classic files are 3,890,416 bytes (Benchy) and
20,653,987 bytes (base). Counting XY+E motion lines in the PNP and Orca files
shows that PNP writes **Z and F on every XY+E move** (e.g. base classic:
526,258/526,258), while the retained Orca classic files include Z on only
eight and F on 125 such moves for either model. The PNP base-classic XY+E
lines carry 2,901,753 Z-token bytes and 3,157,548 F-token bytes. These are
measured **token budgets, not savings**: a modal serializer must preserve
coordinate/feed state across travel, tool changes, raw commands and layer
boundaries, and reducing bytes has no demonstrated process-CPU or wall effect.
The base PNP classic file has fewer XY+E motions than the historical Orca
classic file (526,258 vs 621,226) despite being 1,281,999 bytes larger; a
cross-tool size multiple is not a path-count multiple. This formatting issue
also affects Arachne and cannot explain the *within-PNP* Benchy classic-vs-
Arachne sparse segment explosion. No serializer change was made here.

## Classification and follow-ups

- **Semantically required unless disproven:** classic's thin-gap fill and
  different wall paths. Neither may be removed to match another generator's
  bytes.
- **Geometry question, not established waste:** the Benchy classic sparse
  burst; compare `SliceIR` claims, `PerimeterIR.infill_areas`, and actual
  generated sparse paths against Arachne and canonical roles on the same Z
  planes before proposing any fill-domain fix.
- **Demonstrable emission-shape redundancy, impact unmeasured:** repeated Z/F
  tokens in PNP G-code. A stateful modal-output proposal needs a parser/machine
  semantics oracle (especially raw commands) and a paired wall/CPU A/B before
  any keep/drop. The total token budget is **not** the safe removable byte count.

No acceptance timing or geometry-preservation claim follows from this census.
