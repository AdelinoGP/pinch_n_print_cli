# Bridge Parity Plan — external/internal bridging vs canonical OrcaSlicer

Status: plan (pre-spec). A later session authors the actual spec packets from this
document via `/spec-packet-generator`, gated by `/spec-review <packet> --preflight`.

Provenance: findings gathered from a read-only exploration pass against commit
`9048cd37` (`fix(infill): emit solid shells at full role width and density`) with the
bridge-parity WIP safely parked in `stash@{0}`. Canonical source read from the local,
git-ignored `OrcaSlicerDocumented/` checkout; load-bearing canonical assertions were
re-verified by direct reads of `BridgeDetector.hpp`, `LayerRegion.cpp`,
`PrintObject.cpp`, and `Flow.hpp/.cpp`. Baseline G-code re-measured this session; see
§2 for method.

## 0. Decision record (grilling session, human-owned)

| # | Decision |
|---|---|
| D1 | Acceptance evidence = structural invariants evaluated on PnP output over models in `./resources/` (e.g. `bridge.obj`, `overhang.obj`, `ipadstand.obj`) |
| D2 | No golden fixtures extracted from OrcaSlicer output; strong hardened invariants only. Any uncommitted Orca G-code is steering evidence under the existing **LLM-visual oracle** rule (CONTEXT.md), never adjudication |
| D3 | Sequencing is **internal-first**: internal bridge-over-infill relocation (P75 scope) → false-site classification gating (new packet) → external orientation (P77 scope) |
| D4 | `stash@{0}` is popped when implementation begins and work continues from it; anything outright wrong in it may be discarded during packet work |
| D5 | Equal-cost orientation candidates resolve deterministically (smallest quantized angle) — an intentional divergence recorded in **ADR-0061**, not a DEVIATION_LOG row (the log is reserved for wrongful deviations) |
| D6 | Angle representation crossing IR/WIT/module boundaries stays degrees mod 180°; canonical radians are converted once at the port boundary |
| D7 | Internal bridge gets a proper `InternalBridgeInfill` enum variant threaded through WIT/host/marshal/gcode in the internal-bridge packet; the stash's `Custom("InternalBridge")` string tag retires then |
| D8 | End-state classification mechanism is the canonical unsupported-span test (bridge area minus anchor areas derived from lower-layer slices/expansion zones); the stash's mesh-validity filter is at most a pre-filter |
| D9 | The invariant set is I1–I7 as listed in §6 |
| D10 | The stash pops at the start of the first implementation session, not now |
| D11 | The sparse ±90° per-layer alternation divergence (F7) bundles into the internal-bridge packet |
| D12 | Planning artifacts (this doc, ADR-0061, glossary additions) commit together |

---

## 1. Executive summary

The previous ad-hoc session (see `stash@{0}` and the handoff at
`C:\Users\agpen\AppData\Local\Temp\opencode\handoff_bridge_parity.md`) correctly
identified three parity gaps but under-scoped the problem. Fresh measurement shows a
**fourth, larger gap**: at HEAD, PnP labels bridge material on essentially *every*
layer of the calicat torture model (~160 of 174 layers), while canonical emits exactly
two bridge sites. The existing packet map already reserves slots for most of this work
(P27/P50/P75/P77); what is missing is (a) the false-site/classification gap itself,
(b) a faithful port target for the orientation algorithm (the active inline
`detect_bridging_direction`, **not** the legacy sweep class), and (c) the
post-surface architectural relocation for internal bridges.

## 2. Measured baseline (this session, commit 9048cd37)

Method: `tmp/compare_bridges.py` parses each G-code with M83-relative-E semantics
(extrusion = positive E delta on an XY move), attributes segments to `;TYPE:` carried
across layer changes, keys layers by Z (never layer index), and reports segment count,
extruded length, and dominant direction mod 180° (circular mean over doubled angles,
so short connector segments influence the mean). Reslice command:

```
./target/release/pnp_cli.exe slice --model tmp/calicat.stl \
  --output <out>.gcode --module-dir modules/core-modules   # module-dir is MANDATORY
```

| Metric | PnP HEAD (baseline) | OrcaSlicer reference |
|---|---|---|
| Layers carrying `Bridge`-type extrusion | ~160 of 174 | 1 (+1 internal) |
| External site Z≈3.2/3.25 | 24 segs / 116.9 mm / 1.6° | 89 segs / 424.3 mm / 88.6° |
| Internal site Z≈29.4/29.45 | buried in false sites (≈57 mm @ 0°) | 90 segs / 526.3 mm / ≈30.2° mean (line angle ≈23.3°) |
| Total bridge-labelled extrusion | **7924.9 mm** | **950.6 mm** |

Interpretation caveats:
- Segment counts include the short extruded connectors of Orca's zigzag pattern; the
  prior handoff counted lines only (46 segs / 402–497 mm). Both sides are measured
  identically here, so ratios are comparable across this document.
- The prior handoff's "exactly Orca's two bridge sites" table described the *stashed
  WIP*, not HEAD. At HEAD the false sites dominate the total.

## 3. Finding inventory

Severity labels are estimates (evidence cited inline), per repo rules. "Stash"
= whether the parked WIP already addresses it.

### F1 — HIGH — False bridge classification floods every layer (NEW, uncatalogued)
`assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) stamps
mesh-derived bridge candidates onto any layer whose cross-section intersects the facet
footprint, without canonical's unsupported-span test; `region_partition.rs`
(`crates/slicer-runtime/src/region_partition.rs`) precedence `bridge > bottom > top >
sparse` then claims those areas from infill roles. Result: §2's ~7925 mm vs 951 mm.
Canonical gates bridge surfaces on unsupported-area analysis during surface-type
assignment (`LayerRegion::process_external_surfaces`, expansion-zone anchors or
`lower_layer->lslices`). Stash: **mostly fixed** (validity filter +
layer-gated attachment) — the fix must be preserved/re-derived by the packets, not
re-invented.

### F2 — HIGH — External orientation algorithm divergent (known gap #1, respecified)
At HEAD the orientation comes from `compute_bridge_direction_deg`
(`crates/slicer-core/src/algos/mesh_analysis.rs`): perpendicular of the longest 3D
anchor-edge run; no lower-layer input; hardcodes 0.0 on degenerate input.
Canonical ACTIVE path is the inline `detect_bridging_direction(Lines, Polygons)` /
`(Polygons, Polygons)` pair declared in `BridgeDetector.hpp` and called from
`LayerRegion::process_external_surfaces`:
- floating edges = polyline difference of the bridge expolygon boundary against
  `expand(anchors, SCALED_EPSILON)` (EPSILON = 1e-4 mm);
- candidate directions = unique normals of floating edges (`Line::normal()` =
  `(dy, −dx)`), quantized by `ceil(atan2·1000)`; cost = Σ |edge · normal| over all
  floating edges; pick minimal cost; return the **perpendicular** of the winner;
- **no floating edges → principal components of the overhang area; return the minor
  axis** (degenerate → `{1,0}`). This fallback is what yields calicat's 88.6°/90°;
- stored as `PI + atan2(dir.y, dir.x)` radians, CCW-from-X convention.
The classic `BridgeDetector::detect_angle` class (5° sweep, coverage cost,
spacing tie-break) still exists in `BridgeDetector.cpp` but is dead code behind the
legacy `#else` branch — do NOT port it. Stash: **open** (its floating-edge heuristic
in the stash uses edge-direction candidates, no overhang pre-difference, no PC
fallback, first-wins ties — measured 0° on calicat).

### F3 — HIGH — Internal bridge-over-infill stage placement (known gap #2, respecified)
Canonical runs `PrintObject::bridge_over_infill` inside `prepare_infill()` after
`process_external_surfaces`/`clip_fill_surfaces`; it *generates* sparse-infill anchor
polylines itself (`Layer::generate_sparse_infill_polylines_for_anchoring`),
clusters anchored lines above voids, picks the angle with `determine_bridging_angle`
(length-weighted mean over a ±18° sliding window of nearest-anchor orientations —
which is why real prints get non-grid angles like 23.3°), builds polygons with
`construct_anchored_polygon` (scan lines every `bridging_flow.scaled_spacing()`,
clipped to anchors/walls), emits `stInternalBridge` surfaces and subtracts them from
`stInternal`. Our pipeline decides in `PrePass::ShellClassification`
(`commit_shell_classification_builtin`, `crates/slicer-runtime/src/slice_postprocess_prepass.rs`)
where no infill paths exist; `Layer::InfillPostProcess` exists as the natural seam.
Stash: **open** (prepass placement kept; grid-parity fallback lands 0°).

**Addendum (packet 234a, 2026-08-24):** post-series calicat re-slice measured the
filtering half of this gap: after packets 233–235 the tree emitted
`;TYPE:Internal Bridge` on 148 of 174 layers totalling 86675.76 mm (canonical:
exactly one layer near Z≈29.45 / ≈526 mm) because the InfillPostProcess seam
treated every sparse-infill area as candidate voids with zero lower-layer support
testing. Packet `234a-internal-bridge-support-gating` ported canonical's
unsupported-span arithmetic (`unsupported_span_areas` /
`qualify_internal_bridge_surface`), relocated qualification + anchored-line
construction into the ShellClassification prepass writing a host-only
`SlicedRegion.internal_bridge_lines` carrier field, and reduced InfillPostProcess
to a pure emitter. Post-fix calicat: byte-identical double slice,
Internal-Bridge layers 0 (bar ≤6), external Bridge row at Z≈3.2 unchanged
(90.0° / 74 segs / 324.6 mm). Known residual divergence vs canonical's single
site: our IR has no dense-interior (`stInternalSolid`) surface taxonomy —
`top_solid_fill` is the candidate proxy and canonical gates currently reject all
calicat candidates; coverage/anchoring parity follow-up stays under ISSUE-82.
The Internal-Bridge layers 0 statement is historical (superseded 2026-08-25: see closure addendum below).

**Closure addendum (packet 234a, 2026-08-25):** this packet closed RC-A fills-as-initial
arithmetic; the `internal_solid_fill` taxonomy is WIT-mirrored; the qualification-prepass /
`InfillPostProcess` venue split is landed; expansion, harvesting, and clustering ports are
landed; and the bundle-primary arbiter plus G-code bars and carrier-free
`enable_extra_bridge_layer` emission are covered. The oracle provenance is corrected:
`tmp/calicat_orcaSlicer.gcode` exists untracked, and the brief's numbers were verified
bit-exact against it. The matched-profile arbiter baseline is {(4.45, 23.2 mm²),
(18.45, 8.4 mm²), (29.45, 143.2 mm²)}. Residual low-z mid-stack qualification is
DEV-149 and cavity-site coverage deficit is DEV-150; both are out of scope here and owned
by the shell-classification / infill / support tracks. Carrier-free duplicate angle delivery
is DEV-151, and the unrepresentable `top_solid_infill_flow_ratio` is DEV-152.

### F4 — HIGH — Coverage/anchoring far below canonical (known gap #3)
Even at matched sites the WIP reaches ~30–35% of Orca's extruded length: canonical
anchors and expands candidates (`construct_anchored_polygon`, expansion zones grown
by `expansion_step = scaled(0.1)` up to 5 steps, `expansion_bottom_bridge =
shell_width·sqrt(2)`, closing radius from `frSolidInfill` spacing); the stash's
contour-band approximation (`INTERNAL_BRIDGE_EXPANSION_MULTIPLIER = 3.0` in the
stashed `slice_postprocess_prepass.rs`) shrinks instead. Stash: **partial at best**.

**Closure-partial addendum (2026-08-25).** Expansion zones, `gather_areas_w_depth`
harvesting, thread clustering, and filled-lower-layer removal are all ported and green.
Measured qualified cavity area is approximately 143 mm² versus approximately 262 mm²-equivalent
in the oracle (about 55%). Residual coverage breadth is owned by the infill/construction track
via DEV-150; this packet records the ported machinery and does not tune production toward the
oracle bar.

### F5 — MEDIUM-HIGH — `bridging_flow` ignores configured bridge width; spacing constant absent (NEW)
`bridging_flow` (`crates/slicer-core/src/flow.rs`) derives `dmr` from
`nozzle_diameter·sqrt(ratio)` only. Canonical `LayerRegion::bridging_flow` selects
`thread_diameter = bridge_line_width if set else nozzle_diameter` and canonical
`Flow::bridge_extrusion_spacing(dmr) = dmr + BRIDGE_EXTRA_SPACING (0.05 mm)`; the
`BRIDGE_EXTRA_SPACING` constant exists nowhere in our tree at HEAD. Stash: **module-
level workaround** (passes `thread_base_width` into the flow helper and adds 0.05 mm
spacing in `modules/core-modules/rectilinear-infill/src/lib.rs`); core function left
divergent. Spec should canonicalize the core signature rather than keep the shim.

### F6 — MEDIUM — Bridge feedrate coupled to sparse speed (NEW, fixed in stash)
At HEAD every role shares `speed_factor = infill_speed / BASE_SPEED(50)`
(`modules/core-modules/rectilinear-infill/src/lib.rs`), so emitted bridge feedrate =
`bridge_speed × infill_speed/50`. Canonical assigns `role_speed = bridge_speed`
directly (`Fill.cpp`). Stash: **fixed** (per-class factor ≡ 1.0). Related open
question: solid roles share the same coupling vs canonical `top_solid_speed`/
`internal_solid_speed` — unexamined; flag to the authoring session.

### F7 — MEDIUM — Sparse rectilinear alternates ±90° per layer; canonical does not (NEW)
`run_for_infill` adds 90° on odd layers for all roles. Canonical
`Fill::_infill_direction` applies `_layer_angle(layer_id/thickness_layers)` only when
not fixed-angle and not `dont_alternate_fill_direction`, and `FillRectilinear::
_layer_angle` returns 0 — plain rectilinear keeps `infill_angle` constant. Also
uncatalogued: `infill_rotate_template`/`solid_infill_rotate_template`
(`calculate_infill_rotation_angle`) unread by our module. Stash: untouched.

### F8 — MEDIUM — No `InternalBridgeInfill` role variant (sub-gap of F3)
`ExtrusionRole` (`crates/slicer-ir/src/slice_ir.rs`) has only `BridgeInfill`;
canonical distinguishes `erInternalBridgeInfill` with own flow/speed/fan. Note
`resolve_feedrate` already maps the string tag `"InternalBridge"` →
`internal_bridge_speed` (default 37.5) at HEAD (`crates/slicer-gcode/src/emit.rs`,
`crates/slicer-ir/src/feedrate.rs`), and the stash emits that tag from the module plus
a `;TYPE:Internal Bridge` label mapping. A proper enum variant touches IR + WIT
(`extrusion-role`) — schedule it inside the packet that owns F3, not ad hoc.

### F9 — LOW — Bridge fan handling absent; overhang fields hardcoded; label naming
No `enable_overhang_bridge_fan`/internal-bridge fan markers (single `M106` in
`crates/slicer-gcode/src/serialize.rs`); `scan_expolygon` writes
`overhang_quartile: None`, `dist_to_top_mm: 0.0` unconditionally; bottom-role label
reads `Bottom surface` where canonical viewer text is `BottomSurface`. Cosmetic-to-low;
bundle where convenient.

### Verified-equivalent (no action)
Region partition precedence order matches canonical surface precedence;
`adjust_solid_spacing` matches the three documented D-209 divergences exactly (comment
current); partition precedence verified OK by sweep.

## 4. Proposed packet decomposition

Ledger facts below (packet numbers, issue states) were read this session and MUST be
re-derived at authoring time (`docs/specs/orca-feature-gap/issues/map.md`,
`05-asset-packet-list.md`, next-free-number rule of ticket 06).

| Work item | Content | Existing slot |
|---|---|---|
| W-C. Internal bridge-over-infill relocation (FIRST) | Move the decision post-surface/infill; anchor-line generation or reuse; `determine_bridging_angle` windowed-mean port; `construct_anchored_polygon`; `InternalBridgeInfill` enum variant through IR/WIT/host/marshal/gcode (retires the stash's Custom tag); sparse ±90° alternation fix bundled (D11) | **P75** (`dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle`; open issue `82-author-packet-p75-quality-bridging-bridge-over-infill.md`, blocked by ticket 06) |
| W-A. Bridge classification & false-site gating (SECOND) | End-state mechanism is canonical's unsupported-span test (D8): bridge area minus anchor areas from lower-layer slices/expansion zones; stash's mesh-validity filter survives only as a cheap pre-filter if measurement justifies it | new packet (no owner today) |
| W-B. External orientation port (THIRD) | Active inline `detect_bridging_direction` semantics incl. PC fallback, SCALED_EPSILON expand, deterministic tie-break per ADR-0061, degrees-mod-180 boundary conversion per D6 | **P77** owns `bridge_angle` |

Coverage/anchoring parity (F4) has no separate packet: its primitives
(`construct_anchored_polygon`, expansion-zone growth constants) are delivered inside
W-C for internal bridges and inside W-B for external bridges. Flow/speed role
correctness (F5/F6) rides with whichever packet first touches the relevant code, and
must not regress invariant I7.

## 5. Stash disposition

Per D4/D10: `stash@{0}` pops at the start of the first implementation session and
work continues from it; anything measured or read as outright wrong may be discarded
in packet work. The stash contains ~1338 added lines across 19 files: flag threading
(`is_internal_bridge` through IR/WIT/host/marshal/macros/SDK/partition), module
routing + TOML schema, gcode label mapping, prepass promotion, orientation heuristic,
and the F1 false-site gating. Salvage map: keep threading + routing + label mapping +
false-site gating direction (upgrade to D8's span test); discard or rewrite the
orientation heuristic (replaced by W-B's port) and the contour-band expansion
approximation (replaced by anchored-polygon construction). Guests on disk match HEAD,
not the stash's WIT world — after popping, `cargo xtask build-guests --check` exit
codes decide freshness (0 fresh / 1 stale / 3 missing wasm-tools infra error).

## 6. Verification protocol for the packets

Per D1/D2/D9, acceptance is **invariant-based**, evaluated by slicing models from
`./resources/` (`bridge.obj`, `overhang.obj`, `ipadstand.obj`, and any further model
a packet nominates). The seven invariants:

- **I1 — no bridge over support**: zero bridge-role extrusion where the lower layer
  is solid beneath the segment's span.
- **I2 — site existence**: `bridge.obj` / `overhang.obj` produce bridge sites at
  their known unsupported spans (and only there).
- **I3 — external orientation**: external bridge lines run within ±5° of
  perpendicular to floating edges, or of the minor principal axis when fully anchored.
- **I4 — self-consistent internal angle**: internal-bridge angle equals what our
  ported windowed-mean computes on the same anchor set (never a frozen constant).
- **I5 — density**: bridge line count ≈ span ÷ bridging spacing (±1 line).
- **I6 — role disjointness**: role-partition polygons stay pairwise disjoint.
- **I7 — feedrate**: bridge moves' feedrate equals the resolved bridge speed
  regardless of infill speed.

The calicat table in §2 is steering evidence only (LLM-visual-oracle rule): it
directs attention, never adjudicates. The uncommitted Orca G-code stays in `tmp/`,
unreferenced by any test.

Narrow-first test discipline per AGENTS.md; guest-consuming runs go through
`cargo xtask test --summary`. Gotchas to carry into every packet's verification
section: `--module-dir modules/core-modules` mandatory on reslice; compare by Z never
layer index; `;TYPE:` only appears on role change (carry it); both outputs are M83
relative-E (a positive E delta on an XY move is extrusion); leading-dot floats are
legal (`E.25723`); `cargo fmt --all` broken on this machine (format touched files
individually); boostvoronoi asserts make some perimeter visual-debug captures
unreliable.

## 7. Open questions for the authoring session

1. Solid-role speed coupling (F6 corollary): does canonical route top/internal solid
   through separate speeds, and do we? Unmeasured; decide during W-C authoring.
2. Fan handling (F9): bundle into W-C or defer to the fan-key packet family.
3. Whether W-A's unsupported-span test needs a new prepass data dependency on N±1
   layers (the scheduler constraint noted in packet 36-rev1) or resolves inside the
   existing per-layer stage inputs — re-derive from the scheduler docs at authoring.

## Packet Queue

Generated via `/spec-packet-generator` (orchestrated). Backlog slots are
`docs/specs/orca-feature-gap/issues/` entries, not docs/07 TASK IDs. Numbers follow
ticket 06 Rule 1 (next free = highest numeric `docs/spec_packets/[0-9]*/` prefix + 1,
re-derived from disk at authoring time; 233 was correct when this queue was written).

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 233-internal-bridge-over-infill | Relocate internal bridge-over-infill to the post-surface/infill seam with anchored-polygon construction and windowed-mean angle, thread an `InternalBridgeInfill` role through IR/WIT/host/marshal/gcode, bundling the sparse ±90° alternation fix (D11) and canonical `bridging_flow` spacing (F5/F6) | ISSUE-82 (P75) | - | generated | docs/spec_packets/233-internal-bridge-over-infill/ |
| 2 | 234-bridge-false-site-gating | Gate bridge classification on the canonical unsupported-span test (bridge area minus anchor areas from lower-layer slices/expansion zones), demoting the stash's mesh-validity filter to at most a cheap pre-filter | new (§4 W-A, no prior owner) | #1 | generated | docs/spec_packets/234-bridge-false-site-gating/ |
| 3 | 235-external-bridge-orientation | Port the active inline `detect_bridging_direction` semantics (floating-edge candidates, PC fallback, SCALED_EPSILON anchor expand) with the ADR-0061 deterministic tie-break and D6 degrees-mod-180 boundary conversion | ISSUE-84 (P77, `bridge_angle` half) | #2 | generated | docs/spec_packets/235-external-bridge-orientation/ |

Mechanical reminders encoded in every packet below: (a) popping `stash@{0}` flips
guest WASM artifacts stale again — `cargo xtask build-guests --check` exit codes
(0 fresh / 1 stale / 3 missing wasm-tools) arbitrate freshness; (b) any reslice in a
verification command MUST pass `--module-dir modules/core-modules`.

Errata from generation-time grounding (packets supersede the prose here):
- §3/F2's "dead code behind the legacy `#else` branch" characterization of canonical
  `BridgeDetector::detect_angle` is wrong for the checked-out tree — it is an active
  implementation; the ACTIVE path selects the inline `detect_bridging_direction`
  overloads at `LayerRegion::process_external_surfaces`. Packet 235 states it that way.
- §3/F3's "our pipeline decides in `PrePass::ShellClassification`" describes the STASH
  WIP, not HEAD: at HEAD that function holds no internal-bridge logic and nothing emits
  a `Custom("InternalBridge")` role. Packet 233 frames W-C as introducing the decision
  at the seam.
