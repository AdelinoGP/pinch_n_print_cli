# Lightning geometry/portability gate recorder

Opt-in, reproducible canonical recording for packet
`layer-module-preparation_07_lightning-geometry-portability-gate` (TASK-576).

The recorder exists so that Gate A and Gate B are compared against **independently executed**
canonical OrcaSlicer data, never against PnP output. Nothing here runs during a normal build or
test. The sibling `OrcaSlicerDocumented/` checkout is read-only; instrumentation is applied only to
a disposable clone.

## Files

| File | Purpose |
| --- | --- |
| `record.py` | orchestrates clone/patch/build/slice/replay and writes the portable fixtures |
| `canonical-recorder.patch` | opt-in C++ instrumentation applied to the disposable clone |
| `README.md` | this file |

## Build the instrumented slicer

```bash
# 1. disposable clone (never the sibling checkout)
git clone "D:/slicerProject/pinch_n_print_cli_2/OrcaSlicerDocumented" \
          "D:/slicerProject/_lightning_gate/OrcaSlicer"
cd "D:/slicerProject/_lightning_gate/OrcaSlicer"
git checkout 3e1daccd7c567a0a3d2b5721841d30845f21307e

# 2. apply the recorder patch
git apply "D:/slicerProject/pinch_n_print_cli/tools/lightning_gate/canonical-recorder.patch"

# 3. host toolchain workarounds (measured on this machine, 2026-10-19)
#    a. OpenSSL's configure rejects Git-for-Windows' cygwin Perl. Use a portable
#       Strawberry Perl (which also ships NASM) and put it first on PATH.
export PATH="/d/slicerProject/_lightning_gate/strawberry/perl/bin:/d/slicerProject/_lightning_gate/strawberry/c/bin:$PATH"

#    b. CPython's PCbuild bootstrap resolves the newest `pythonx86` NuGet package
#       (3.15), whose `SourceFileLoader` no longer has `load_module`. Pin the
#       bootstrap interpreter to 3.10.4 before building:
#       cd deps/build/dep_python3-prefix/src/dep_python3
#       rm -rf externals/pythonx86
#       ./externals/nuget.exe install pythonx86 -Version 3.10.4 -ExcludeVersion \
#           -OutputDirectory externals

# 4. build dependencies once, then the slicer
cmd //c "build_win.bat -d -j 8"
cmd //c "build_win.bat -s -j 8"
```

The instrumented executable lands at
`D:/slicerProject/_lightning_gate/OrcaSlicer/build/src/Release/orca-slicer.exe`.

## Run

```bash
python tools/lightning_gate/record.py --capture \
  --output crates/slicer-wasm-host/tests/fixtures/lightning_gate

python tools/lightning_gate/record.py --replay \
  --manifest crates/slicer-wasm-host/tests/fixtures/lightning_gate/manifest.json \
  --output target/lightning-canonical-replay

python tools/lightning_gate/record.py --publish-fixtures \
  --capture target/lightning-gate-current \
  --outputdir target/lightning-fixture-check
```

`--capture` writes per-variant 3MF scenes (`raw/<variant>/model.3mf`), an auditable
`raw/<variant>/scene.json`, and the canonical observations under `<output>/raw/<variant>/`, plus
`manifest.json` with reference-source and build-checkout revisions/dirty state, patch hash and
applied state, tool versions, input/scene/settings hashes and per-variant exit codes. Each case
entry also records `hole_rings`, the number of hole rings across the variant's typed
planning/final domains. `hole_void` fails the capture (exit 3, `inconclusive`) when that count is
zero, so a scene change cannot silently regress the family back to a slit contour. `--replay`
refuses a missing manifest or missing observations.

## Provenance

`--source` is the read-only reference checkout used for shipped presets, and its revision is the
`canonical_revision`. The instrumented slicer is built from a separate disposable checkout, derived
from `--slicer` or supplied explicitly with `--build-source`. `manifest.json` records
`canonical_source`/`canonical_revision` for the reference tree and
`canonical_build_source`/`canonical_build_revision`/`canonical_build_dirty` plus
`recorder_patch_applied` for the patched build tree. The build tree is expected to be dirty because
the recorder patch changes tracked files; `canonical_build_dirty: true` is normal and is not a
clean-state claim. If the build checkout cannot be resolved as a git work tree, `--capture` exits 3
with `{"status": "inconclusive", ...}` rather than reporting a clean build.

## Scene writer

`record.py` authors real 3MF projects rather than STL files:

- Each `SceneObject` becomes one build item. A single-part object is one mesh object; a multi-part
  object is a wrapper `<object>` whose `<components>` reference one leaf object per part.
- `Metadata/model_settings.config` carries object-wide metadata and per-`<part>` metadata with the
  exact subtype strings `normal_part`, `modifier_part`, `negative_part`. Every `part id` equals the
  leaf component `objectid`.
- `translate_mm` is emitted only as the build-item transform (12 row-major values), never baked
  into mesh coordinates.
- Packaging is deterministic: fixed entry order, 1980-01-01 timestamps, `ZIP_DEFLATED`.
- `scene.json` mirrors the package (object/part ids, kinds, boxes, per-object/per-part configs,
  build transforms) and records the intended fixture crosswalk (paint chains) where applicable.

Scenes use 0.2 mm layers. Every object starts with `sparse_infill_pattern="lightning"`,
`sparse_infill_density="15%"` and `wall_loops="2"`; common process overrides are
`top_shell_layers="2"`, `bottom_shell_layers="1"` and
`dont_filter_internal_bridges="nofilter"`.

| Case | Intent |
| --- | --- |
| `smoke_box` | one normal 20 × 20 × 0.8 mm box |
| `anchor_bridge` | 24 × 18 × 1.2 mm plateau + two 6 × 18 × 1.6 mm towers; top shell over sparse tower interiors |
| `pooled_regions` | lightning base + grid 30% / 45° modifier; two regions in one planning pass |
| `hole_void` | 24 × 18 × 1.4 mm rectangular ring mesh with an 8 × 6 mm through-hole |
| `island_split_merge` | two disjoint 1.0 mm islands merged by a 1.4 mm cap |
| `paint_modifier_config` | lightning base + 30%/45° and 10%/90° modifier parts; paint-chain crosswalk |

Each case is captured as `base`, `_density30` (base object density 30%) and `_translated` (a second
`object-b` deep copy at `translate_mm=(40, 0, 0)`), for each requested wall strategy.

## Behaviour when prerequisites are absent

`record.py` exits `3` with `{"status": "inconclusive", ...}` when the instrumented slicer or a
recorded observation is missing. It never synthesises geometry. This is the Gate A/B
`inconclusive` outcome defined by the packet, not a silent pass.

## Instrumentation surface

The patch is opt-in: all hooks return immediately unless `LIGHTNING_GATE_OUTPUT` names a directory.
`LIGHTNING_GATE_CASE` selects the case identifier written into the JSON file names.
`LIGHTNING_GATE_REPLAY=1` additionally asks the recorder to build the immutable planner snapshot
and write the independent `replay.json` artifact; `record.py --capture` sets it for every variant.

Recorded observations per object:

- `<case>.object_<id>.source.json` — earlier-source surfaces and raw configuration, captured in
  `PrintObject::bridge_over_infill` before any temporary bridge-anchor expansion. This is the only
  material Gate A may consume. Every surface entry carries a stable `source_id`
  (`o<object_id>_l<global_layer_index>_r<region_id>_s<surface_index>`, index within that region's
  `fill_surfaces.surfaces`), plus `region_id`, `local_layer_index`, `global_layer_index`, `z_mm`,
  `height_mm` and `checkpoint = "before_anchor_edit"`. `surface_type` and `expolygon` are
  unchanged. The recorder catalogs each surface address against its id so Phase 2 can resolve the
  exact candidate and operand surfaces without a geometric guess.
- `<case>.object_<id>.planning.json` — pooled canonical planning domains and overhangs, the
  generator scalars, per-region surfaces, and the complete per-layer node/edge/root/grounding graph
  from `FillLightning::Generator::generateTrees`. Each graph node additionally carries `ptr`, the
  runtime `Node*` address as a `uint64`, so the grounding witnesses in `grounding.jsonl` can be
  joined to the final graph; `ptr` is omitted from the replay graphs because production and replay
  nodes necessarily have different addresses.
- `<case>.object_<id>.domains.json` — the exact post-anchor planning checkpoint, written once per
  object between the bridge-over-infill Phase 2 write pass and the config-only
  `prepare_lightning_infill_data()`. Each layer records `local_layer_index`,
  `global_layer_index`, `z_mm` and `height_mm`; each region records `internal_domains`
  (post-anchor `stInternal`), `void_domains` (post-anchor `stInternalVoid`), `temporary_anchors`
  (the exact Phase-2 `expansion` copies keyed by layer/region, not inferred later from
  `stInternalSolid`), `final_expolygons` (the **untyped** pre-anchor surfaces copied before
  Phase 2, i.e. the geometry that is restored after the generator is built),
  `final_internal_domains` (that same pre-anchor copy filtered to `stInternal`) and
  `final_void_domains` (that same copy filtered to `stInternalVoid`). The typed fields split the
  untyped union without reordering surfaces; all three preserve expolygon holes exactly.
- `<case>.object_<id>.operations.json` — the exact Phase-2 anchor-expansion transformation events,
  written once after the parallel pass and before the pre-anchor surfaces are restored. Each event
  records `operation_id` (zero-based, stable in layer → region → source-id order), `kind`
  (`AnchorExpansion`, `Union` or `Difference`), `checkpoint = "post_anchor"`, `inputs` (pre-anchor
  `source_id` values), `parent_operations`, ordered `operands` (`subject` / `clip`; each supplies
  either source ids or a prior `operation_id`), `offset_units`, `join`, `result_geometry` in
  canonical units and `outputs: []` (the provenance builder fills final boundary addresses).
  `AnchorExpansion` records `offset_units = 3000000` (the canonical `scaled<float>(3.0)` radius)
  and `join = "miter"`; `Union` and `Difference` record `offset_units: null`, `join: null`.
  `inputs` is the checked source-level projection while operation-to-operation dependencies live in
  `parent_operations` and operand `operation_id`; the two are consistent projections of the same
  event, never competing representations. A candidate or pre-anchor surface that cannot be
  resolved to its catalog entry is emitted as `null` and counted in the top-level `lookup_miss`;
  the recorder never invents an id. `seed_policy` is `root_point_hash`, matching the graph records.
- `<case>.object_<id>.replay.json` — the independent snapshot replay artifact, written at the same
  post-anchor seam when `LIGHTNING_GATE_REPLAY` is set. It records the canonical snapshot
  fingerprints before and after both fixture runs (`FNV-1a` over the key-sorted snapshot JSON),
  whether the snapshot was left unchanged, whether the faithful-planning replay graph matches the
  production `m_lightning_generator` graph, the faithful/wrong domain polygon counts, the
  wrong-domain nonempty flag, the mm² symmetric-difference area of the two candidate outline sets,
  whether the wrong-domain graph differs from the faithful one, and the complete
  production/faithful/wrong graph JSON with node/edge/root counts. `topology_differs` compares the
  complete serialized graph (node coordinates, edges, roots and grounding). A faithful mismatch
  adds a bounded summary of the first differing layer/node/edge instead of a full diff. Replay is
  reference-only: it never writes `planning.json`, `callers.jsonl` or `clips.jsonl`.
- `<case>.object_<id>.grounding.jsonl` — production-only grounding witnesses, written while
  `FillLightning::Generator::generateTreesFromDomains` runs the real `PrintObject` layer loop. Each
  line is one `kind`:
  - `grounding` records a `Layer::getBestGroundingLocation` decision: `object_name`, `object_id`,
    `layer_id`, `z_mm`, `site` (`generateNewTrees` or `reconnectRoots`), `unsupported_mm`, the
    winning outline edge (`boundary_point_mm`, `edge_from_mm`, `edge_to_mm`; null when no outline
    edge exists), `boundary_winner`, `weighted_distance` in canonical units, and `tree_node_ptr`
    (0 for a boundary winner).
  - `attachment` records an actual `addChild` site after it happened: `unsupported_mm`,
    `parent_ptr`, `child_ptr`, `new_root`, `parent_point_mm` and `child_point_mm`, with `site`
    `attach` or `reconnectRoots`. For the `reconnectRoots` reroot site the recorded edge is the
    final orientation after `reroot()` (new root as parent).
  Pointers are runtime `Node*` addresses and join to the same capture's `planning.json` graph
  `ptr` field. Snapshot replay passes no `PrintObject`, so no context is set and these writers
  no-op; `grounding.jsonl` is not a replay artifact and never influences selection.

All recorded JSON files (including the derived `provenance.json`) record both `object_id` (the
numeric `PrintObject::get_id()`, unstable across process ordering) and `object_name` (the
`ModelObject::name` stable scene name, e.g. `object-a` / `object-b`). `domains.json` additionally
records the five production-resolved generator `scalars`
(`infill_extrusion_width`, `supporting_radius`, `wall_supporting_radius`, `prune_length`,
`straightening_max_distance`) and the `resolved_inputs` they derive from (`max_nozzle_diameter_mm`,
`layer_height_mm`, `sparse_infill_line_width_mm`, `object_line_width_mm`, `fill_multiline`,
`sparse_infill_density_percent`, `overhang_angle_deg`, `prune_angle_deg`,
`straightening_angle_deg`). Scalar distances are millimetres, matching `planning.json`.
- `<case>.object_<id>.callers.jsonl` — one JSON object per fill dispatch installed by
  `Layer::make_fills` and `Layer::generate_sparse_infill_polylines_for_anchoring`. A dispatch token
  `o<object_id>_l<global_layer>_r<region_id>_d<counter>`, object/layer/region identity,
  `region_id_group`, surface type, pattern string and integer, extrusion role, `angle_deg`,
  `fixed_angle`, `density_percent`, `spacing_mm`, the exact caller expolygon, the same
  region-config keys recorded by `regions_json`, and `source_ids`.
  `source_ids` is the complete pre-anchor contributor set of the grouped fill that produced the
  dispatch, in `group_fills` accumulation order. It is resolved by the occurrence identity
  `(object, global layer, region, surface index)` rather than by the runtime `Surface*` address,
  because `fill_surfaces` can be reallocated between the source snapshot and the dispatch pass. A
  miss is written as JSON `null` and is never replaced by a geometric guess. Measured across the
  36-variant current-patch capture: only `smoke_box` resolves every dispatch. Every discriminating
  family has unresolved groups — `hole_void` classic 14/16, Arachne 13/28; `anchor_bridge` classic
  28/49, Arachne 28/43; `pooled_regions` 16/17; `paint_modifier_config` 18/19. Those surfaces are
  created after the pre-anchor snapshot: `combine_infill` runs between `bridge_over_infill` and
  `make_fills`, and Arachne surface variation adds more post-snapshot surfaces. They have no earlier
  source occurrence until `combine_infill` is instrumented as a transformation event, so a dispatch
  containing a `null` contributor is not yet usable as final-fill provenance.
- `<case>.object_<id>.clips.jsonl` — one JSON object per
  `FillLightning::Filler::_fill_surface_single` invocation. Records the caller dispatch token,
  `callee_call_index`, the exact received `callee_expolygon`, `pre_clip_polylines` (after
  `multiline_fill`), `clipped_polylines` (after `intersection_pl`), `appended_output_range` as
  `[start_index, end_index)` around `chain_or_connect_infill`, `thickness_layers` and
  `spacing_mm`. A missing caller context records `dispatch_token: null` in
  `<case>.object_unknown.clips.jsonl` instead of crashing.

`Fill::fill_surface` offsets the caller expolygon before `_fill_surface_single`; the caller and
callee geometries are recorded separately, and `callee_call_index` distinguishes the (possibly
multiple) callee invocations of one dispatch. Clips are written only for the lightning pattern;
the caller file covers every pattern because the probe is installed on the real dispatch site.

Coordinates are recorded in canonical integer units and millimetres. 1 canonical unit = 1 nm;
PnP uses 100 nm units, so the Rust comparison divides by 100 at the boundary.

## Provenance DAG

`--capture` writes `<case>.object_<id>.provenance.json` for every object after its `source.json`,
`domains.json` and `operations.json` exist. The builder reads only those recorder artifacts; it
never consults PnP and never invents a source id. `--no-provenance` skips the step for debugging.

Top-level fields:

| Field | Meaning |
| --- | --- |
| `schema_version` | provenance schema version, currently `1` |
| `case_id`, `object_name`, `object_id` | crosswalk copied from the recorded artifacts |
| `unit`, `unit_note` | `canonical_units`; OrcaSlicer 1 unit = 1 nm, converted by PnP at 100 nm/unit |
| `ring_normalization` | the exact normalization rule applied to every polygon |
| `source_catalog` | pre-anchor source surfaces with normalized `geometry` and addressable `rings` |
| `planning_domains` | `internal`, `void` and `temporary_anchor` surfaces with boundary contributors |
| `final_domains` | `final_internal` and `final_void` survivors with boundary contributors |
| `operations` | normalized Phase-2 operations and the surviving boundary addresses they produced |
| `counts` | `source_surfaces`, `planning_boundaries`, `source_overlaps`, `generated_boundaries`, `unavailable_boundaries`, `operations` |
| `unavailable` | boundary addresses with no source or operation attribution and a `reason` |
| `provenance_sha256` | SHA-256 of the canonical JSON body with this field omitted |

Boundary addresses are deterministic. A source address is `(source_id, ring_id, segment_id)`. A
domain address is `(domain, surface_id, ring_id, segment_id)` where `surface_id` is
`o<object>_l<local_layer>_r<region>_<slug>_<index>` and `slug` identifies the typed domain
(`internal`, `void`, `anchor`, `final_internal`, `final_void`).

### Ring normalization and units

Every source, planning, final and operation-result polygon goes through one normalizer:

- contours are counter-clockwise (positive shoelace area) and holes are clockwise;
- the duplicated closing point and consecutive duplicate points are dropped;
- each ring is rotated so its lexicographically smallest `(x, y)` point is first;
- `ring_id` 0 is the contour, `1..n` are holes, and `segment_id` is `0..n-1` in normalized order.

The same normalized point sequence is emitted in `geometry` and in `rings[].segments`, so a
boundary endpoint can be compared byte-for-byte with a source endpoint. Inputs stay in canonical
integer units (1 nm); no PnP conversion happens here. The Rust consumer applies the 100 nm
conversion.

### Overlap rule

For each planning/final boundary segment, the builder searches source segments in the **same
object, local layer and region**:

1. exact integer collinearity, then
2. a retry with a 1 canonical-unit tolerance.

An overlap is a positive-length projection of the boundary segment onto the source segment. The
interval is computed with exact `Fraction` arithmetic, clamped to `[0, 1]`, and emitted as reduced
`numerator`/`denominator` pairs with a positive denominator. A tolerance match adds
`match_tolerance_units: 1` to the relation. All overlapping source segments are preserved as
separate contributors, so a boundary can carry more than one source.

### Generated rule

When no source overlap exists, the boundary is matched against normalized operation
`result_geometry` segments (exact, then 1 unit tolerance) on the operation's layer/region derived
from its input source ids. Precedence is:

- `temporary_anchor` domains: `AnchorExpansion`;
- other domains: `Difference`, then `Union`;
- if no direct result matches, the same match is retried against `parent_operations`;
- a final fallback accepts any operation kind.

A temporary anchor is a derived post-anchor surface whose edges are often collinear with the
source edge it was expanded from, so it is resolved through `AnchorExpansion` first and only falls
back to source overlap when no operation result contains the segment. This keeps a derived anchor
from being misreported as a pre-anchor source boundary.

For the chosen operation, one contributor is emitted per input `source_id` (or per operand
source when `inputs` is empty) with `relation = {kind: "generated", operation_id, ...}`. Generated
contributors carry `ring_id: null` and `segment_id: null`: there is no source segment of their own,
and the operation result address lives in the matched operation entry. If the chosen operation has
no inputs, or no operation matches, the boundary is recorded in `unavailable` with
`operation_has_no_inputs` or `no_matching_operation`; the recorder never synthesizes a source id.

`counts.source_overlaps` and `counts.generated_boundaries` count **boundary segments** resolved by
that relation (not contributor records). `counts.planning_boundaries` counts all internal, void and
temporary-anchor contour/hole segments in `planning_domains`; final domains are reported separately.

### Operation outputs

Each operation's normalized result segments are matched against every surviving planning and final
domain boundary, restricted to the operation's layer/region when its inputs identify one. Matching
addresses are written to `outputs` in `(domain, surface_id, ring_id, segment_id)` order. An
operation whose result does not survive a domain (for example a `Difference` with empty
`result_geometry`) keeps an empty `outputs` list.

### Determinism and missing families

The file is canonical JSON (`sort_keys=True`, compact separators) written to a temporary file and
atomically replaced. Source catalog order is `(local layer, region, source_id)`, domains
`(domain, layer, surface_id)`, boundaries `(ring_id, segment_id)`, contributors
`(source_id, relation kind, operation_id)` and operations `operation_id`. No timestamps or runtime
addresses are included. A family with zero planning domains still gets a provenance file with the
source/final catalogs, zero counts and `reason: "no_planning_domains"`; this is not a capture
failure. A per-object provenance error is logged to `capture.log` and does not abort the capture.

`grounding.jsonl` remains a separate production-only witness stream. It is not folded into the
provenance DAG and does not influence contributor selection.

## Measured canonical constraints (revision 3e1daccd7c)

These were discovered by execution, not assumed:

- `lightning_straightening_angle` is validated in `[5, 85]`. The packet's seed value of `0` is
  rejected by the CLI (`Param values in 3mf/config error`); the recorder uses `5`.
- The CLI accepts exactly one machine, one process and one filament preset. A second process file
  is rejected as a duplicate, and a CLI-authored preset that names an `inherits` parent is rejected
  with `Preset was not found in the loaded bundle`. `record.py` flattens the `inherits` chain from
  the shipped profile tree and passes complete presets.
- `Layer::print_z` and `Layer::height` are already millimetres (`coordf_t`). Only polygon/collision
  coordinates are scaled units; unscaling the layer Z yields `2e-7` for a 0.2 mm layer.

## Scene coverage status

All six families are authored as 3MF and record real planning data. Measured coverage:

- `anchor_bridge` combines a 1.2 mm central plateau with two 1.6 mm towers in one normal part. The
  taller towers keep sparse interiors while the wide plateau transitions to solid top-shell layers;
  this co-occurrence is the Phase-2 temporary-anchor precondition. Measured: 2 `temporary_anchors`
  at z=0.6 in the base classic variant.
- `pooled_regions` produces two planning regions per layer (`lightning` 15% and grid 30% / 45°),
  and the graph pools both regions' surfaces.
- `hole_void` is authored as a true rectangular ring mesh: `Frame` emits the outer and inner
  loops plus side/top/bottom quads, so the mesh itself is a closed manifold with a hole. Measured:
  the layer at z=0.4 has an `internal_domains` entry and a `final_internal_domains` entry that are
  each a 4-point contour with one 4-point hole ring. Classic resolves 8 planning boundaries by
  exact source overlap (4 contour + 4 hole segments), Arachne 36, with zero unavailable
  boundaries in both.
- The earlier `hole_void` scene subtracted the through-hole as a `negative_part` and also carved a
  sealed void. Canonical represented that opening as a single 26-segment slit contour with no hole
  ring: the sealed void's cavity fused the hole boundary into the outer contour. That scene was
  measured again with a ring mesh *and* the sealed void and still produced the slit, so the void
  was dropped and the ring mesh kept. `void_domains` were empty on every layer of both scenes;
  void coverage is not claimed for this family.
- `island_split_merge` shows two separate surfaces on the lower layers and a bridge surface joining
  them under the cap.

The instrumentation records both the exact post-anchor planning domains and the pre-anchor final
surfaces. On the flat-plate scenes the Phase-2 `expansion` site produced no `temporary_anchors`:
the only layers with `stInternal` sat directly above the solid bottom shell, so their
`needs_lightning_expansion` flag was false, and the layers above them were fully solid, so the
canonical `if (sparse_infill.empty()) break;` exited before the expansion loop. The plateau/tower
anchor scene fixes that by keeping tower interiors sparse across the plateau's solid top-shell
layers. Measured with the final geometry: `temporary_anchors` is nonempty (2 anchors at z=0.6),
`internal_domains` is nonempty at z=0.4 (2 domains), and `void_domains` is empty on every layer.
The initially requested 1.0 mm plateau produced zero anchors; the 1.2 mm plateau with 1.6 mm
towers produced 2.

### Measured replay discriminator (`anchor_bridge` classic)

- Faithful snapshot replay matches the production graph exactly: **27 nodes / 16 edges / 11 roots**
  for both, with `faithful_matches_production = true` and `snapshot_unchanged = true`.
- The wrong `final_domain_only` replay produces **67 nodes / 43 edges / 24 roots**, with
  `domain_symmetric_difference_area = 142.55912927276 mm²` and `topology_differs = true`.
- The area is the sum of **per-layer aligned** symmetric differences. A cross-layer flat union
  measured 0.0 even though layer 2 has 0 post-anchor internal domains versus 2 final internal
  domains; the aligned metric is the one the recorder writes.
- `hole_void` is a coverage/control family, not the AC-N1 discriminator: its faithful and wrong
  replays both measure **41 nodes / 27 edges / 14 roots**, area 0.0, `topology_differs = false`.

## Publish gate fixtures (`--publish-fixtures`)

`--publish-fixtures` converts one Classic capture into the three portable gate documents. It only
projects recorded values; it never derives an expected geometry, defaults an angle, or synthesises
a missing field.

```bash
python tools/lightning_gate/record.py --publish-fixtures \
  --capture target/lightning-gate-current \
  --outputdir target/lightning-fixture-check
```

Inputs (all under `--capture`): the capture root's `manifest.json` (revision, patch identity, tool
versions) and, for each of the five fixed cases `anchor_bridge`, `pooled_regions`, `hole_void`,
`island_split_merge`, `paint_modifier_config` (Classic only, in that order),
`raw/<case>_classic/<case>_classic.object_<id>.{source,planning,domains,provenance,replay}.json`
plus `.callers.jsonl` and `.clips.jsonl`. The variant's recorded `settings.json` is read for the
one key the earlier-source snapshot does not carry, `dont_filter_internal_bridges`. Translated,
Arachne and density-30 companions are never read.

Outputs (`--outputdir`): `inputs.json` (`GateInputsFile`), `canonical.json` (`GateOracleFile`) and
`manifest.json` (revisions, units, per-case `a_source_sha256`, `a_raw_config_sha256`,
`a_earlier_provenance_sha256`, `b_input_sha256`, `reference_sha256`, contributor-evidence counts,
and a named `absent` map for every branch the capture cannot supply). Hashes are SHA-256 over
canonical JSON (`sort_keys=True`, `separators=(",", ":")`, UTF-8) of the typed subtree only:
`a_raw_config_sha256` covers `{object_config, regions[{identity, config}]}` in `a_source` order and
`a_earlier_provenance_sha256` covers `a_source.provenance`. Missing hashes are the string
`"absent"`, never omitted.

Semantics:

- `a_source` is the kernel's restricted `GateASourceInput`; geometry is transported from canonical
  units (1 nm) to PnP IR units (100 nm) with round-half-away-from-zero and a declared tolerance of
  1 PnP unit. `reference` geometry stays canonical (`units.reference` in the manifest) so the
  oracle keeps full recorded precision. `variant_chain` is empty because no authorized artifact
  records a paint/variant chain.
- A non-`Option` kernel field with no recorded source makes the case `inconclusive`; it is never
  coerced. A reference/B branch that needs an unrecorded value is published as `null` with a named
  reason in `manifest.json.cases.<case>.absent`.
- `contributor_evidence` labels every recorded dispatch (keyed by `dispatch_token`, never vector
  position): `Complete` when every recorded contributor resolves in the earlier-source catalog,
  `Unavailable` with a named reason otherwise.

Recorded-value projections worth knowing:

- `a_source.object_config.dont_filter_internal_bridges` comes from the variant's `settings.json`
  and is mapped to the closed `disabled` / `limited` / `nofilter` spelling (all five current cases
  record `nofilter`). A preset without the key publishes `null`, which leaves the branch
  undecidable for Gate A; an unrecognised spelling is unrepresentable and does not publish.
  `model_alignment_offset_rad` stays `0.0` because every case records
  `align_infill_direction_to_model = 0` — a recorded zero, not a default.
- Every `a_source` region config carries `sparse_infill_pattern` verbatim from
  `source.json.regions[].region_config` (`lightning`, or `grid` for `pooled_regions`'
  modifier region).
- `EarlierSurfaceKind.BottomBridge` is its own kind: `island_split_merge_classic` publishes the
  recorded `bottom_bridge` surface `o0_l5_r0_s2` instead of aborting.
- `SourceSegmentRef` emits `surface`, `ring_id` and `segment_id` always, with `null` ids exactly
  where the capture recorded null (generated contributors and surface-level operation
  inputs/operands). `0` is a real ring/segment and is never used as an absence marker.
- `PlanningLayer.operations` is published from `provenance.json.operations`, placed on the single
  local layer their recorded input source ids name. `checkpoint` is transcribed verbatim
  (`post_anchor` for the three `anchor_bridge_classic` events, not rewritten). `outputs` keep only
  the surviving addresses inside this planning layer's typed domains; a recorded `final_internal`
  address is a final-domain address and is not addressable by `BoundaryAddress.domain`, so it is
  not published here.
- `reference.parameters` transcribes the eight `resolved_inputs` values plus the four
  `planning.json.scalars` constructor scalars (`supporting_radius`, `wall_supporting_radius`,
  `prune_length`, `straightening_max_distance`) — none is recomputed.

Branches still absent in the current capture (published as `null` + a named `absent` reason, never
fabricated):

- `reference.policies`: `callers.jsonl` records the resolved angle, pattern, `fixed_angle` and
  `surface_type` per *dispatch*, but a `FillPolicyInput` is one policy per region occurrence.
  `bridge_angle_rad` is recorded by none of the seven artifacts, `is_bridge` is not recorded for an
  occurrence, and `fixed_angle` disagrees inside a single occurrence (true for top/bottom
  dispatches, false for internal/solid). The manifest names each of those fields.
- `reference.graph`: `planning.json` graph nodes carry no `origin_region_index` and no
  `sampled_segments`.
- `reference.final_fills`: `FinalFillInput.policy` needs the per-region policy above.
- `b_input.input`: `RegionInput.outlines` and per-region shell/classified surfaces are not
  recorded.

Exit codes: `0` all five cases published; `3` a missing capture/case artifact or an
unrepresentable required field. On exit `3` no fixture files are written — only `status.json`
naming each case's status, field and reason — so a partial case set can never be mistaken for a
published one.

Determinism: fixed five-case order, `sort_keys`, compact canonical JSON, no timestamps, no absolute
paths, and no pointer identity (planning graph `ptr` values and the numeric `object_id` are never
published). Two runs over the same capture produce byte-identical documents.


## Determinism

- The scene is generated from `record.py` box descriptions, not from PnP output.
- The canonical revision, patch hash, settings hash, `model.3mf` hash and `scene.json` hash are
  recorded per variant. `manifest.json` also carries top-level `settings_sha256` and `scene_sha256`
  maps keyed by `variant_id`; the Classic/Arachne distinction lives in the settings preset, so
  their `input_sha256` is identical while their `settings_sha256` differs.
- Time-based diagnostic SVG naming in `Generator` is never read or recorded.
- The graph is captured after pruning/reconnection, from `Layer::tree_roots`, with node identity
  derived from pointer identity and parent/root relations from the tree itself.
