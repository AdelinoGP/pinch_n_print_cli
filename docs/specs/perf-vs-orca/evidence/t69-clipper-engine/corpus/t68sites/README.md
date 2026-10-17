# T68-site engine-input corpus (format v1)

Coordinates are raw signed i64 engine values: 1 unit = 100 nm. No scaling,
sorting, orientation normalization, simplification, or polygon filtering is
performed by the recorder or reducer. Each already-flat engine path is framed
as one path with one ring; flatten all rings, in order, before engine execution.
Case call_index is the zero-based host-service index for CP; for RP it is the
zero-based engine invocation within that layer's preparation derivation.

## Production boundary and parameters

All classic sites are in `ClassicPerimeters::emit_walls`
(`modules/core-modules/classic-perimeters/src/lib.rs`), via `hs::Host::{offset_polygons,clip_polygons}`
(`crates/slicer-wasm-host/src/host.rs`). The recorder runs immediately before
`ClipperOffset::add_paths/execute_tree` in `inflate_once` and before
`boolean_op_tree_64` in `clip_polygons` (`crates/slicer-core/src/polygon_ops.rs`).
Offsets returning early for empty input or zero delta are counted as services
but do NOT invent engine cases; compare raw service and engine counts below.
Offsets use `ClipperOffset::new(miter_limit, arc_tolerance, false, false)` and
`execute_tree`; booleans use the PolyTree helper, not a flat-path approximation.

Opcode: 0 offset, 1 difference, 2 union, 3 intersection, 4 xor.
Arg 1: delta f64 bits in integer units (not mm); 2: JoinType (Square=0,
Bevel=1, Round=2, Miter=3); 3: EndType (Polygon=0, Joined=1, Butt=2,
Square=3, Round=4); 4: miter-limit f64 bits; 5: arc-tolerance f64 bits in
integer units. Boolean arg 10: FillRule (EvenOdd=0, NonZero=1, Positive=2,
Negative=3). These are clipper2-rust 1.1.0 enum discriminants. Both ClipperOffset
boolean constructor flags are false, the production defaults, with no additional
per-case parameters. No separate subject/clip fill rule exists in this API.

| Site | `emit_walls` expression |
| --- | --- |
| A0 | outer wall-loop inset |
| A1 | inner wall-loop inset |
| B | gap shrunk_prev |
| C | gap grown_cur |
| D | gap difference |
| E | infill_area |
| F | shrunk_inner |
| G | grown_infill |
| H | final-transition difference |
| J | gap-width-band filtered_gaps |
| K | infill-boundary inset |

RP1 is the actual derivation entered from `LayerArena::ensure_prepared_regions`;
RP2 is the actual derivation entered from `LayerArena::ensure_prepared_perimeter_source_regions`
(`crates/slicer-runtime/src/blackboard.rs`). All nested `inflate_once` and
`clip_polygons` engine calls are included, not just the outer closing wrapper.
The marshal adapters are `prepare_slice_regions`, `prepare_perimeter_source_regions`,
and `prepare_regions` (`crates/slicer-wasm-host/src/marshal/prepared.rs`);
`SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`) is
one native intersection consumer. The opcode census in `summary.json` records
what this checkout actually invokes; old t60 attribution is not presumed current.
On identity projections RP2 may execute `prepare_slice_regions` and populate
the shared ordinary cache; its RP2 tag describes the initiating arena method,
not an independent second projection. RP1/RP2 files therefore do not imply both
derivations execute on every layer (ticket 67 sharing is preserved).
The optional RP recorder keeps the last snapshot per owner/layer: repeated
derivations overwrite that owner's earlier snapshot. RP raw counts therefore
mean engine calls in retained snapshots, not an exhaustive preparation census.
Do not use RP counts as timing denominators or frequency weights.

## Sampling and census

The classic-site census is checked against the complete matched-job service
totals; the RP census has the retained-snapshot limitation above. Geometry was
captured only for layers divisible by 20, plus layers 239, 270, 494 when present.
The endpoints retain terminal shapes; 270 retains the known collapsed-inset
stratum. The reduced set retains whole layers within each of the classic and RP
families. It tests power-of-two multiples of 20 through 1280, preferring the
smallest classic stride first, then the smallest RP stride that fits the
19,500,000-byte geometry budget (reserving space under the total ceiling for
documentation). Thus the optional RP leg does not crowd out classic coverage.
This is a shape corpus,
not a frequency-weighted benchmark; use the exhaustive census for weights.
Service sequence is mapped only after validating the whole dispatch signature;
the 8-service collapsed-inner-inset signature maps A0,A1,E,F,G,H,J,K, rather
than incorrectly relabeling shifted calls B,C,D,... . Unsupported signatures fail.

Reduced rule: `classic: layer % 40 == 0; RP: layer % 320 == 0; both additionally retain layer in {239, 494, 270} (existing layers only)`.

### benchy

Classic sampled layers: [0, 40, 80, 120, 160, 200, 239]. RP sampled layers: [0, 239]. Raw dump: 926 files, 9038848 bytes.

| Site | Raw services / RP engine calls | Raw engine calls | Raw geometry cases | Reduced engine cases |
| --- | ---: | ---: | ---: | ---: |
| A0 | 240 | 240 | 13 | 7 |
| A1 | 240 | 240 | 13 | 7 |
| B | 240 | 240 | 13 | 7 |
| C | 240 | 240 | 13 | 7 |
| D | 240 | 240 | 13 | 7 |
| E | 240 | 240 | 13 | 7 |
| F | 240 | 240 | 13 | 7 |
| G | 240 | 214 | 10 | 5 |
| H | 240 | 240 | 13 | 7 |
| J | 240 | 240 | 13 | 7 |
| K | 240 | 240 | 13 | 7 |
| RP1 | 661 | 661 | 33 | 3 |
| RP2 | 661 | 661 | 33 | 3 |
### base

Classic sampled layers: [0, 40, 80, 120, 160, 200, 239, 240, 270, 280, 320, 360, 400, 440, 480, 494]. RP sampled layers: [0, 239, 270, 320, 494]. Raw dump: 1993 files, 61981830 bytes.

| Site | Raw services / RP engine calls | Raw engine calls | Raw geometry cases | Reduced engine cases |
| --- | ---: | ---: | ---: | ---: |
| A0 | 495 | 495 | 28 | 16 |
| A1 | 495 | 495 | 28 | 16 |
| B | 494 | 494 | 27 | 15 |
| C | 494 | 494 | 27 | 15 |
| D | 494 | 494 | 27 | 15 |
| E | 495 | 495 | 28 | 16 |
| F | 495 | 495 | 28 | 16 |
| G | 495 | 492 | 25 | 13 |
| H | 495 | 495 | 28 | 16 |
| J | 495 | 495 | 28 | 16 |
| K | 495 | 495 | 28 | 16 |
| RP1 | 1926 | 1926 | 103 | 14 |
| RP2 | 1926 | 1926 | 103 | 14 |

## Inventory

| File | Bytes | Cases | SHA-256 |
| --- | ---: | ---: | --- |
| benchy-t68sites.corpus | 2124517 | 75 | `81e234a8b22e1d581f2982d509a69f74268ee1bb0cc7f66ff0e3dd50fd0e9800` |
| benchy-region-prep.corpus | 675472 | 6 | `7fc94e638c4d80f19918711d5cc5e4e9a2cb73984f0477b4749c8fbfa2788ac3` |
| base-t68sites.corpus | 8241039 | 170 | `9153ed4c0b00c36b83fb68f63104ea56175c3ef9e8f085e1c60502128ebca4ed` |
| base-region-prep.corpus | 5725950 | 28 | `0251a77241f8bef6445b8b64b7224d08d77a2ba2f8bd4d6c70b02a614c9849ed` |

Regenerate: `python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/reduce.py`. Raw root defaults to `.local-artifacts/perf-vs-orca/t69-clipper-engine/raw/`; override with `--raw`. Output identity gates are re-read from t58 qualification and applied to actual raw G-code files before reduction.
