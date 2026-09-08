# Design: 238c-support-renderer-flow-interfaces

## Measured Renderer Baseline (2026-08-25)

Authoritative snapshot of what the renderers/planner actually DO on disk today, verified
this session. Where any older statement in this packet or the plan conflicts, THIS
section wins.

**Planner output shape (`build_roles`, `modules/core-modules/tree-support-planner/src/lib.rs`):**
role regions are DISCRETE per-connected-component bodies per layer. Each planned node
contributes its own footprint — a circle, plus a capsule ONLY between consecutive layers
of the SAME node (`structural_body_regions` calls `swept_region(point, point)`);
`union_connected_components` then merges only geometrically OVERLAPPING polygons into
connected bodies. There is NO cross-branch capsule sweep along MST edges and NO global
per-role union of separate branches. The renderer therefore receives MANY SMALL REGIONS
per role per layer (trunk segments, tip discs), not one merged body outline. Any AC,
test, or design text assuming a single merged region per role is stale.

**Renderer mechanics (`render_polygon` + `scan_fill_region`,
`modules/core-modules/tree-support/src/lib.rs`; mirrored in traditional-support):**

- `render_polygon` = `wall_count` inset wall passes + `scan_fill_region` of the inner
  remainder. This wall/fill SPLIT already exists (landed with 238b's renderer work).
- Scan fill is HORIZONTAL-ONLY: scan lines advance in +y; there is no infill-angle or
  pattern parameter anywhere in the fill path. Orca's 45° crosshatch has no counterpart.
- Body pitch = `line_width / density.min(1.0)` from the percent-as-fraction
  `support_density` key (the G-11 defect this packet replaces); interface pitch =
  `support_interface_spacing + flow_spacing` (~0.757 mm at defaults: gap 0.4 +
  flow spacing ~0.357 at width 0.4 / layer height 0.2).
- Min-fill ALREADY LANDED: when the pitch loop would place no line inside a sub-pitch
  region, `scan_fill_region` emits one center fill line — hollow tip outlines are fixed;
  remaining tip delta is solid-puck vs ring semantics (see deltas below).
- The renderer reads ONLY `entry.roles` / `family_id` / `global_layer_index` /
  `decline_reason`. The `SupportPlanSkeleton` (points + `wall_counts`) carried by each
  entry is UNUSED for path generation today — extra-wall printing and centerline
  rendering would both need to start consuming it.
- Branch centerline rendering (Orca's circle+chord look around skeleton points) is
  UNIMPLEMENTED. There is also NO min-area filter on rendered regions.
- Renderer files measured: tree-support `lib.rs` 665 lines; traditional-support `lib.rs`
  622 lines.

**Remaining visual deltas vs Orca** (evidence: comparison bundles `tmp/vdcmp/{ours,ref}/`
with manifests; reusable gcode-source requests `tmp/vdcmp/{ours,ref}-request.json`;
numeric per-role extrusion profile method documented in `tmp/p238b-human-validation.md`):

1. Trunk infill pattern: horizontal rungs vs Orca's 45° crosshatch → AC-13.
2. Tip solidity: rings vs solid pucks → AC-14 (min-fill center line landed; pattern and
   ring-vs-puck coverage remain).
3. Top-layer tip count/size: ~30 (PnP) vs ~50 (Orca) at the reference l120 — roof/
   interface band semantics, G-18-adjacent → AC-15.
4. Branch centerline rendering unimplemented → AC-16 disposition required.

Reference artifacts: `tmp/SupportTest_Tree_Orca.gcode` (124 `;TYPE:Support` blocks),
`tmp/p238b-tree-fixture.gcode` (124 blocks, delta 0).

**AC-16 disposition (recorded 2026-08-26): branch centerline rendering stays UNIMPLEMENTED by deliberate choice — option (b).** The renderer draws the planner's DISCRETE role regions (circle footprints + same-node capsules, §baseline above) rather than overlaying circle+chord outlines around skeleton centerlines. Visual consequence, named: branch cross-sections follow planned region outlines (slightly polygonal where capsule/circle regions abut), not Orca's circular chord look; wall/fill structure and densities are unaffected because they key off the same regions. Revisit trigger: if the 242 closure human gate judges the outline character a visible defect, a follow-up implements circle+chord over `SupportPlanSkeleton.points`.

## Controlling Code Paths

- Primary code path: `modules/core-modules/tree-support/src/lib.rs` (`render_polygon`,
  config parse, interface pitch plumbing) and
  `modules/core-modules/traditional-support/src/lib.rs` (mirror surface) — the two family
  renderers; `modules/core-modules/tree-support-planner/src/lib.rs` for G-12/G-13/F-37
  planner-side attribution (and any planner deltas surfaced by this packet per the scope
  directive in packet.spec.md §Scope Boundaries).
- Neighboring tests/fixtures:
  `modules/core-modules/tree-support/tests/tree_support_tdd.rs`,
  `modules/core-modules/traditional-support/tests/{traditional_support_tdd,support_fill_geometry_tdd}.rs`,
  `modules/core-modules/tree-support-planner/tests/{tree_family_tdd,diagnostics_tdd}.rs`,
  `crates/slicer-runtime/tests/integration/support_family_closure.rs`
  (`interface_layer_count_follows_config`, commit `ee27ac94` pins),
  `crates/slicer-gcode/src/emit.rs` unit tests,
  new `crates/slicer-core/tests/{support_flow_semantics_tdd,support_interface_regularize_tdd}.rs`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Plan Corrections

Recorded per the pre-verified canonical probe (trust over plan where differing):

1. **DEV-146 (plan §12 said "this packet adds an interface-width key").** Canonical has NO
   `support_interface_line_width`; interface pitch derives from the interface flow width
   produced by `support_material_interface_flow` (`Flow.cpp`) — the interface flow ratio
   applied over `support_line_width` (fallback `line_width`). **Decision (see §Interface
   Width Mechanism):** add a percent `support_interface_flow` key mirroring the flow-ratio
   derivation over 238a's `float_or_percent` `support_line_width`, NOT a width key. The
   DEVIATION_LOG row records this as a deliberate PnP knob choice with canonical-equivalent
   default behavior.
2. **G-11 framing.** The plan's "percent consumed as fraction" and "no density model" are
   one defect chain: canonical derives all three densities from spacings; PnP reads one
   mis-scaled key. Fix = replace the mechanism (AC-2), not re-scale the key — hence the
   `support_density` key is REMOVED from both manifests rather than re-ranged. New DEV row
   documents the removal (user-facing key retirement).
3. **DEV-129 verification-first.** Plan §10 already suspects implemented-truth; live tree
    confirms `InterfaceRole::Floor` classification exists in the planner node loop and
    `diagnostics_ttd`-suite assertions expect NO code-1003. Default disposition: CLOSE as
    implemented after removing the stale manifest comment; finishing work only if Step 3's
    verification finds a real gap.
4. **G-18 mechanism corrected by measurement (2026-08-26).** Layer-mapped block extraction of both sides (`tmp/p238c-baseline-normal.gcode`, `tmp/SupportTest_Normal_Orca.gcode`; embedded config headers verified identical at top=2/bottom=2) shows Orca's three `;TYPE:Support interface` runs are CONSECUTIVE ROOF layers (l122–124, Z24.4–24.8) with no plate-level or mid-air floor band anywhere — the older floor-collapse story is stale. Canonical `generate_interface_layers` (`SupportCommon.cpp`) sizes the top band as contact + N−1 intermediates and never lets the bottom count touch the top branch, leaving the reference third layer's exact provenance unresolved after targeted probing. Adopted rule (satisfies every locked constraint): the traditional top band widens by ONE layer iff raw configured `support_interface_bottom_layers >= 1`; bottom ≤ 0 (incl. the −1 mirror-top legacy sentinel) keeps exactly N. Locked pins hold; AC-5's 3@top=2/bottom=2 is met. If 242 closure identifies the true canonical source of the third layer, replace the condition — the test asserts counts, not the condition.

## Approach Per Surface

### G-10 + G-11 — renderer flow/density model

The wall/fill SPLIT in `render_polygon` already exists (see §Measured Renderer
Baseline); this step replaces its DENSITY and PATTERN semantics:

- Keep the `wall_count` concentric inset loops (inset `line_width * (i + 0.5)`) — the
  tree-planner's LANDED `wall_counts` transport (238b export, schema 2.1.0) supplies
  per-node extra-wall counts once the renderer starts reading the skeleton; the manifest
  key remains the fallback. Consuming `wall_counts` for regions that contain a
  skeleton point with count ≥ 1 is IN SCOPE (extra-wall printing).
- Interior fill only when the region needs infill (canonical `area_group.need_infill`),
  pitched at body density spacing `line_width / body_density` where
  `body_density = min(1, support_flow_spacing / (support_base_pattern_spacing +
  support_flow_spacing))`, `support_flow_spacing =
  line_width_to_spacing(resolved support_line_width, effective_layer_height)` — the live
  helper is BINARY: `slicer_core::flow::line_width_to_spacing(width: f32, layer_height:
  f32) -> Result<f32, NegativeSpacingError>`; the layer-height source is the renderer's
  per-layer effective layer height already carried by the plan entry's layer context
  (never a hardcoded constant; the G-09 hardcode `support_layer_height_mm: 0.0` in
  `crates/slicer-core/src/algos/support_geometry.rs` was retired by 238a and must not
  reappear). Every call site unwraps via the existing renderer error path, mapping
  `NegativeSpacingError` to a structured decline, never to a silent default.
  NOTE: `support_base_pattern_spacing` is declared today ONLY on
  `traditional-support-planner` (float, default 2.5); the tree family needs it declared
  on `tree-support-planner`/`tree-support` for this derivation to read a real value.
- Fill DIRECTION alternates across layers (canonical crosshatch equivalent): extend the
  fill path with an angle/axis parameter (45°/135° alternating, or horizontal/vertical)
  keyed off the entry's global layer index — replacing the current horizontal-only scan
  (AC-13).
- Interface roles keep dedicated pitches: top
  `min(1, interface_flow_spacing / (support_interface_spacing + interface_flow_spacing))`;
  bottom analogous over `support_bottom_interface_spacing`. `0`-spacing ⇒ solid interface
  (pitch == extrusion width).
- Densities/pitches computed in ONE place per renderer via a shared helper moved to
  `slicer-core` (see consolidation below) so traditional and tree cannot drift.
- Config parsing: drop `support_density` handling; resolve `support_line_width` through
  the 238a typed-key semantics (float_or_percent, 0 = auto → nozzle-derived default
  recorded as deviation).
- Per-component rendering is PRESERVED: because planner role regions are discrete
  connected components (§Measured Renderer Baseline), fill each region independently;
  do NOT reintroduce a global per-role union before filling — Orca fills each branch's
  own footprint too.

### G-12 — radius cap

`MAX_BRANCH_RADIUS_MM: f32 = 6.0` → `10.0` in
`modules/core-modules/tree-support-planner/src/lib.rs` (constant site, clamp in the
radius function `raw.clamp(MIN_BRANCH_RADIUS, MAX_BRANCH_RADIUS_MM)`, doc-comment
mentions, and the clamp test). Locate all sites by symbol grep — line pins from
authoring time have rotted (the file is ~6.5k lines). Canonical pair:
`MIN_BRANCH_RADIUS = 0.4` / `MAX_BRANCH_RADIUS = 10.0` (`TreeSupport.hpp`). Pure constant
flip plus test update; golden reblessing NOT expected (cap raise only widens radii that
were previously clamped; classify any golden drift per E3 before regenerating).

### G-13 — raise-to-base under interfaces

In the same planner radius pipeline: when `support_interface_top_layers > 0`, compute the
interface band base radius (canonical `calc_branch_radius` mm-to-top variant raises to
`base_radius`) and emit `max(ordinary_radius, base_radius)` for layers whose distance-to-
roof falls inside the band. Reference profile runs top=2, so this is active in every
current parity slice.

### G-18 — roof/floor band structure

Two coordinated changes pinned by AC-5:

- Traditional band planning adopts canonical
  `number_of_support_interface_bottom_layers` semantics exactly: bottom < 0 ⇒ mirror top;
  else explicit bottom count. The existing contact-inclusive anchoring from `ee27ac94`
  stays; what changes is the FLOOR side emitting its full canonical band (today the
  floor band collapses into the roof block count at top=2/bottom=2).
- Tree-side `draw_circles` floor-block gating replicated where absent:
  `!support_on_build_plate_only && (bottom_gap_height > EPSILON || bottom_interface_layers
  > 0)`, downward-scanning base components and splitting overlapping bands into floor
  areas (node-classification already carries `InterfaceRole::Floor`; verify the gate
  conditions match canonical, fix where they don't).

### F-37 piece 2 — base-interface role carrier

New role end to end, derived-at-activation (schema bump rides the packet that introduces
the variant). The SupportPlanIR schema is 2.1.0 today (238b's wall-counts minor bump);
the base-interface role rides the next derived-at-activation bump:

- WIT: `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`
  gains `base-interface` in `support-plan-role` (currently the four variants
  `support-body`, `top-interface`, `bottom-interface`, `raft-related`);
  `crates/slicer-schema/wit/deps/ir-types.wit` gains a base-interface push method on
  `support-output-builder` (mirroring `push-interface-path`'s shape). Both host `bindgen!`
  and guest `include_str!` read these canonical files — no inline copies exist. The
  skeleton's landed `wall-counts: list<u32>` field (238b) is not touched.
- IR: `SupportPlanRole::BaseInterface` in `crates/slicer-ir/src/slice_ir.rs`;
  `ExtrusionRole::SupportBaseInterface` with `default_priority()` between
  `SupportMaterial` (5000) and `SupportInterface` (5500) — e.g. 5250 — so base-interface
  passes sort after body, before roof, matching canonical material ordering intent.
- Host: builder impl in `crates/slicer-wasm-host/src/host.rs`; the live four-arm
  `SupportPlanRole` dispatch match lives in
  `crates/slicer-wasm-host/src/dispatch.rs` and gains the `BaseInterface` arm; BOTH
  marshal
  legs (`marshal/in_.rs` wasm view, `marshal/native.rs` native view) round-trip the new
  role — T9 leg-skew hazard called out per-step. Both legs already carry the landed
  length-assertion for skeleton `wall_counts`; extend, don't fork.
- Planner: node circles landing within `num_top_base_interface_layers` of a roof get
  attributed `BaseInterface` (disjoint from Roof/Body by construction of the existing
  `InterfaceRole::target_for_node` precedence — extend it, don't fork it). The key must
  be DECLARED first-time in `tree-support-planner.toml` (Step 12 owns the declaration;
  it exists in no manifest today — an undeclared key silently defaults and the planner
  could not read it).
- Renderers: consume the plan role and push through the new carrier method.
- G-code: `orca_type_label(SupportBaseInterface) = ";TYPE:Support interface"` — DECISION:
  reuse the interface marker because canonical prints base-interface as interface-material
  geometry; a distinct marker would break Orca reference diffing (block counts). Feedrate
  mapping uses the interface feedrate branch. Review the closed-loop-role set for the new
  variant (default false — fill passes are open paths).
- Marker-doc home DECISION: no canonical doc enumerates the support `;TYPE:` marker set
  as an owned reference; no new enumeration doc is created. The authoritative label
  contract stays `orca_type_label` (`crates/slicer-gcode/src/emit.rs`) + its AC-8 unit
  test; observed block counts are recorded in the human-gate checklist. The two
  `docs/02_ir_schemas.md` sections that reference the role surface ("Support plan entry"
  role enumeration under IR 9b; "Extrusion-role default priority" table) are the only
  doc edits.
- Schema docs: `docs/02_ir_schemas.md` documents the variant.

### interface_regularize consolidation

The two files are byte-identical (re-verified on disk 2026-08-25 with `cmp`). Shape:

- Move `regularize_entry_roles` + private helpers into ONE shared module
  `crates/slicer-core/src/support_regularize.rs` (pub API), tests move to
  `crates/slicer-core/tests/support_interface_regularize_tdd.rs` unchanged in substance.
- Rationale for `slicer-core`: it already hosts `polygon_ops`, `smooth_outward`, and the
  flow helpers the module calls; both guest modules depend on it; no new crate, no host
  round-trip needed (pure geometry over `slicer_ir` types).
- Delete both module-local copies; renderers call
  `slicer_core::support_regularize::regularize_entry_roles`.
- Scope-limited: the `rectilinear-infill` third copy (DEV-127) stays out.

### DEV-129 verify-close-or-finish procedure

1. Verify: run the `diagnostics_tdd`-suite narrow test
   (`interface_bottom_layers_is_supported_and_warns_nothing`) and confirm a real slice
   emits `InterfaceRole::Floor` bands (planner test exists). Guest freshness first (T4).
2. If truth == implemented: remove the stale "Not yet implemented" comment above
   `[config.schema.support_interface_bottom_layers]` in
   `tree-support-planner.toml`, update the DEVIATION_LOG row to closed-implemented with
   evidence pointers, keep tests green. (Expected path.)
3. If verification finds missing emission: FINISH the gap in this packet (same step
   budget), then close identically. No third state; no silent rewrite of the diagnostic.

### DEV-145 correction mechanics

Both manifests flip `default = -1.0` → `0.5` (and `min = -1.0` stays so the legacy
mirror-top sentinel remains expressible); renderer parsers treat negative as mirror-top
legacy input, documented as non-canonical. DEVIATION_LOG row corrected to state the real
divergence (PnP default −1.0 vs canonical 0.5) and its resolution. Regenerate
`docs/15_config_keys_reference.md`.

### Extra-wall printing over the landed transport

238b delivered `SupportPlanSkeleton.wall_counts: Vec<u32>` end to end (WIT
`wall-counts`, IR field, both marshal legs with length assertions, plus a planner pin
test proving nonzero counts for extra-wall nodes). The renderer-side consumption is THIS
packet's: read the entry's skeleton, and for each role region containing a skeleton
point with `wall_count >= 1`, emit `1 + count` wall loops there (falling back to the
manifest `tree_support_wall_count` where no skeleton point applies). This closes the
DEV-144 consequence ("branches at merge points print thinner than canonical") on the
consumer side.

## Architecture Constraints

- Invariant 16 (plan §6): every acceptance command names explicit `--exact` test names or
  asserts matched-count non-zero in the same run — all AC commands tee to
  `target/test-output.log` and guard `grep -c '^test .* ok$' > 0`.
- E1 (no vacuous assertions): hollow-wall proof asserts path STRUCTURE (wall-loop count +
  interior pitch), not artefact existence; density proof asserts computed ratios against
  closed-form values.
- E6 (feature-gated blindness): slicer-core test commands carry `--features host-algos`.
- E8/E9: snake_case keys everywhere; mm↔unit conversions only at declared boundaries.
- T8 (silent config defaults): every newly-read key gets a manifest `[config.schema]`
  entry + regenerated `docs/15_config_keys_reference.md` in the same commit.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

Additional mandatory constraint (schema/version bump): the F-37 schema-version bump must
be authored in the SAME step that adds the variants, together with every test hard-asserting
the old version literal; locked wire formats stay pinned at their constructor literals.

## Code Change Surface

- Selected approach: renderer-mechanism replacement (density derivations + fill-pattern/
  per-component semantics on the EXISTING wall/fill split), constant+rule parity flips in
  the planner, one shared regularize module in `slicer-core`, and a single WIT/IR/gcode
  carrier for the base-interface role.
- Exact functions, traits, manifests, tests, and fixtures:
  - `render_polygon`, `scan_fill_region` (+ callers, config parse) — `modules/core-modules/tree-support/src/lib.rs`
  - equivalent body/interface pitch paths — `modules/core-modules/traditional-support/src/lib.rs`
  - `MAX_BRANCH_RADIUS_MM`, radius clamp fn, `InterfaceRole::target_for_node`,
    node-classification loop — `modules/core-modules/tree-support-planner/src/lib.rs`
  - `regularize_entry_roles` — moves to `crates/slicer-core/src/support_regularize.rs`
  - `SupportPlanRole`, `ExtrusionRole`, `default_priority` — `crates/slicer-ir/src/slice_ir.rs`
  - `support-output-builder`, `support-plan-role` — `crates/slicer-schema/wit/deps/{prepass-support-geometry/prepass-support-geometry.wit, ir-types.wit}`
  - builder/dispatch/marshal — `crates/slicer-wasm-host/src/{host.rs, dispatch.rs, marshal/in_.rs, marshal/native.rs}`
  - `orca_type_label`, feedrate mapping — `crates/slicer-gcode/src/emit.rs`
  - manifests — `modules/core-modules/{tree-support/tree-support.toml, traditional-support/traditional-support.toml, tree-support-planner/tree-support-planner.toml}`
  - docs — `docs/02_ir_schemas.md`, `docs/15_config_keys_reference.md`, `docs/DEVIATION_LOG.md`
- Rejected alternatives and reasons:
  - Keep-and-rescale `support_density` percent key: canonical has no such key; rescaling
    preserves a phantom knob and leaves the density model divergent (Q3 evidence).
  - Add `support_interface_line_width` key: canonical derives pitch from the flow ratio
    over line width (Q8); a width key forks the derivation surface for no user value.
  - Consolidate regularize into a new shared crate: unnecessary — `slicer-core` already
    holds the polygon/flow dependencies both guests link.
  - Distinct `;TYPE:` marker for base-interface: breaks Orca block-count comparability
    (human-gate requirement) and contradicts canonical material usage.

## Files in Scope (read + edit)

Target at most 3 primary files per step; the packet total spans the enumerated surface.

- `modules/core-modules/tree-support/src/lib.rs` - role: tree renderer; expected change:
  hollow-wall split + density model + carrier consumption
- `modules/core-modules/traditional-support/src/lib.rs` - role: traditional renderer;
  expected change: density model + floor-band counts + carrier consumption
- `modules/core-modules/tree-support-planner/src/lib.rs` - role: planner; expected change:
  cap 10.0, raise-to-base, BaseInterface attribution
- `crates/slicer-core/src/support_regularize.rs` (new) - shared regularize + density
  helpers
- `crates/slicer-ir/src/slice_ir.rs` - role enums + priorities
- `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`,
  `crates/slicer-schema/wit/deps/ir-types.wit` - WIT carrier
- `crates/slicer-wasm-host/src/host.rs`, `src/dispatch.rs`, `src/marshal/in_.rs`,
  `src/marshal/native.rs` - dispatch + both legs
- `crates/slicer-gcode/src/emit.rs` - marker + feedrate
- manifests ×3, docs ×3 as listed under Code Change Surface

## Read-Only Context

- `modules/core-modules/tree-support-planner/src/lib.rs` - symbol-grepped ranges only
  (grep `MAX_BRANCH_RADIUS_MM`, the radius clamp in the clamp fn,
  `InterfaceRole`, `build_roles`, `structural_body_regions`, node classification); file is
  ~6.5k lines and prior line pins have rotted — re-locate by symbol, never by stale line.
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` - range around
  `final_gcode_roles` / `interface_layer_count_follows_config` only (the latter is a pub
  helper chained by the former, not its own registered test).
- `docs/specs/support-families-anchored-entities-plan.md` - §12 brief + §13 traps ranges.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `modules/core-modules/*/tests/golden/**` fixture bodies - never load; classify drift
  from summaries
- Packets 236–238b directories, `rectilinear-infill`, AGG surfaces, raft modules - other
  packets' scope (238b's own packet dir is read-only reference)
- `docs/specs/support-families-anchored-entities-plan.md` queue table - orchestrator-owned

## Expected Sub-Agent Dispatches

- Question: exact current text + neighbors of the `support_bottom_interface_spacing`
  blocks in both manifests; scope: `modules/core-modules/*/*.toml`; return: `SNIPPETS`;
  purpose: Step 1 red-baseline.
- Question: confirm `render_polygon`/`scan_fill_region` wall/fill behavior and
  `fill_pitch_honours_support_density`
  assertions; scope: `modules/core-modules/tree-support/src/lib.rs` + tests; return:
  `SNIPPETS`; purpose: Step 2 red tests.
- Question: LOCATIONS of every `match` on `SupportPlanRole` / `ExtrusionRole` across
  workspace; scope: `crates/ modules/`; return: `LOCATIONS` (≤20); purpose: F-37 blast
  radius (struct-literal discipline analog for enum variants).
- Question: SUMMARY of `draw_circles` floor-block gating vs PnP node classification;
  scope: `TreeSupport.cpp` + planner loop; return: `SUMMARY`; purpose: G-18 step.
- Question: FACT on next free DEV id + TASK id collisions; scope: `docs/DEVIATION_LOG.md`,
  `docs/07_implementation_status.md`; return: `FACT`; purpose: closure registration.

## Data and Contract Notes

- IR/manifest contracts: `SupportPlanEntry.roles` carries
  `SupportPlanRoleRegion { role, regions }`; new role rides this record — no new entry
  type. `SupportPlanEntry.skeleton` carries the LANDED
  `points` + `wall_counts` parallel lists (equal lengths; count 0 = plain node, ≥1 =
  extra walls) — the renderer currently ignores it and this packet wires consumption.
  Manifest keys snake_case; undeclared keys silently default (E9/T8) so every read
  key must be declared.
- WIT boundary: canonical sources only (`crates/slicer-schema/wit/`); both `bindgen!` and
  guest macro read them; after edits `cargo build --tests` then rebuild guests.
- Determinism/scheduler constraints: no new claims; role attribution happens inside the
  existing planner claim window; serial/parallel determinism preserved (pure functions of
  plan entries + config).
- CLI evidence contract: any slice run for evidence MUST include
  `--module-dir modules/core-modules` (support modules otherwise silently fail to
  register — zero support blocks, no error) and use `--model <path>` for the input.

## Locked Assumptions and Invariants

- `;TYPE:` label decision LOCKED: `ExtrusionRole::SupportBaseInterface` →
  `;TYPE:Support interface`.
- Density formulas LOCKED to canonical forms (AC-2); no alternate scaling selectable
  (Ruling 8 applies to knobs replacing legitimate behavior, not defect fixes).
- `ee27ac94` top-count pins LOCKED passing (contact-inclusive anchoring not regressed).
- Regularize behavior byte-conserved: moved code + tests assert identical outcomes.

## Risks and Tradeoffs

- Removing `support_density` changes user-visible config surface; mitigated by deviation
  row + regenerated config docs + human-gate inspection.
- Radius-cap raise can move golden endpoints; E3 classification required before any
  rebless (expected: none, since clamped-at-6 branches are rare in the benchy fixture).
- WIT addition ripples generated bindings across crates; contained by same-step
  blast-radius ownership and `cargo build --tests`.
- Floor-band expansion may shift interface block counts in OTHER configurations (top=1,
  bottom=-1); AC-5 pins top=2/bottom=2 plus regression-pins the `ee27ac94` rows.
- Shared `slicer-core` helper grows guest dependency surface for both modules (already
  dependents; no new linkage risk).
- Per-component fill means many small regions per layer: a naive per-region fill pass is
  fine, but any cross-region optimization (e.g. chaining scan lines across components)
  must preserve deterministic ordering — keep fills per region in plan-entry order.
- The ~30-vs-~50 tip-count delta (AC-15) may partly reflect Orca's denser tip seeding
  rather than band semantics alone; AC-15 requires reconciling only what band semantics
  own and recording the residual explicitly.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (F-37 carrier step; mitigated by LOCATIONS dispatch pre-enumerating
  match sites)
- Highest-risk dispatch and required return format: enum-match sweep across workspace —
  `LOCATIONS` ≤20 entries.

## Open Questions

- [FWD] Does any golden beyond `benchy_tree_support_regression_*` hard-assert radii
  affected by the cap raise? Resolve by running the planner suite at Step 4; if drift
  appears, classify per E3 before proceeding.
- [FWD] Exact priority integer for `SupportBaseInterface` (proposed 5250) — implementer
  may adjust if scheduler ordering tests reveal a conflict, keeping the 5000–5500 band.
- [FWD] AC-16 centerline rendering: implement (circle+chord over skeleton points) or
  record discrete-region rendering as the disposition? Decide at Step 7 with the human
  gate's visual verdict in hand; either branch satisfies AC-16, silence does not.
- [BLOCK] None remaining. (At authoring time activation was blocked on 238b reaching
  `implemented`; that dependency is SATISFIED as of 2026-08-25.)
