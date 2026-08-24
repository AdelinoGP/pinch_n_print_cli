# Design: 238c-support-renderer-flow-interfaces

## Controlling Code Paths

- Primary code path: `modules/core-modules/tree-support/src/lib.rs` (`render_polygon`,
  config parse, interface pitch plumbing) and
  `modules/core-modules/traditional-support/src/lib.rs` (mirror surface) — the two family
  renderers; `modules/core-modules/tree-support-planner/src/lib.rs` for G-12/G-13/F-37
  planner-side attribution.
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

## Approach Per Surface

### G-10 + G-11 — renderer flow/density model

Replace `render_polygon`'s filled-body scan-fill with the canonical wall/fill split:

- Emit `tree_support_wall_count` concentric inset loops (inset `line_width * (i + 0.5)`)
  from each contour — the tree-planner's `wall_counts` transport (238b export) supplies
  per-region counts when present; the manifest key remains the fallback.
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
- Interface roles keep dedicated pitches: top
  `min(1, interface_flow_spacing / (support_interface_spacing + interface_flow_spacing))`;
  bottom analogous over `support_bottom_interface_spacing`. `0`-spacing ⇒ solid interface
  (pitch == extrusion width).
- Densities/pitches computed in ONE place per renderer via a shared helper moved to
  `slicer-core` (see consolidation below) so traditional and tree cannot drift.
- Config parsing: drop `support_density` handling; resolve `support_line_width` through
  the 238a typed-key semantics (float_or_percent, 0 = auto → nozzle-derived default
  recorded as deviation).

### G-12 — radius cap

`MAX_BRANCH_RADIUS_MM: f32 = 6.0` → `10.0` in
`modules/core-modules/tree-support-planner/src/lib.rs` (constant site ~line 87, clamp in
the radius function, doc-comment mentions, and the clamp test at ~5641). Canonical pair:
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
the variant):

- WIT: `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`
  gains `base-interface` in `support-plan-role`;
  `crates/slicer-schema/wit/deps/ir-types.wit` gains `push-base-interface-path` on
  `support-output-builder` (mirroring `push-interface-path`'s shape). Both host `bindgen!`
  and guest `include_str!` read these canonical files — no inline copies exist.
- IR: `SupportPlanRole::BaseInterface` in `crates/slicer-ir/src/slice_ir.rs`;
  `ExtrusionRole::SupportBaseInterface` with `default_priority()` between
  `SupportMaterial` (5000) and `SupportInterface` (5500) — e.g. 5250 — so base-interface
  passes sort after body, before roof, matching canonical material ordering intent.
- Host: builder impl in `crates/slicer-wasm-host/src/host.rs`; the live
  `SupportPlanRole` dispatch match (all four role arms) lives in
  `crates/slicer-wasm-host/src/dispatch.rs` and gains the `BaseInterface` arm; BOTH
  marshal
  legs (`marshal/in_.rs` wasm view, `marshal/native.rs` native view) round-trip the new
  role — T9 leg-skew hazard called out per-step.
- Planner: node circles landing within `num_top_base_interface_layers` of a roof get
  attributed `BaseInterface` (disjoint from Roof/Body by construction of the existing
  `InterfaceRole::target_for_node` precedence — extend it, don't fork it).
- Renderers: consume the plan role and push through the new carrier method.
- G-code: `orca_type_label(SupportBaseInterface) = ";TYPE:Support interface"` — DECISION:
  reuse the interface marker because canonical prints base-interface as interface-material
  geometry; a distinct marker would break Orca reference diffing (block counts). Feedrate
  mapping uses the interface feedrate branch. Review the closed-loop-role set for the new
  variant (default false — fill passes are open paths).
- Marker-doc home DECISION: no doc enumerates the support `;TYPE:` marker set today
  (`rg ';TYPE:Support' docs/*.md` returns zero hits at authoring time); no new
  enumeration doc is created. The authoritative label contract stays `orca_type_label`
  (`crates/slicer-gcode/src/emit.rs`) + its AC-8 unit test; observed block counts are
  recorded in the human-gate checklist. The two `docs/02_ir_schemas.md` sections that
  reference the role surface (plan-entry role enumeration ~line 985; extrusion-role
  priority table ~line 1273) are the only doc edits.
- Schema docs: `docs/02_ir_schemas.md` documents the variant.

### interface_regularize consolidation

The two files are byte-identical today (verified by diff at authoring). Shape:

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

1. Verify: run `diagnostics_ttd`-suite narrow test
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

- Selected approach: renderer-mechanism replacement (density derivations + wall/fill
  split), constant+rule parity flips in the planner, one shared regularize module in
  `slicer-core`, and a single WIT/IR/gcode carrier for the base-interface role.
- Exact functions, traits, manifests, tests, and fixtures:
  - `render_polygon` (+ callers, config parse) — `modules/core-modules/tree-support/src/lib.rs`
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

- `modules/core-modules/tree-support-planner/src/lib.rs` - lines around
  `MAX_BRANCH_RADIUS_MM` (~87), radius clamp (~4409–4437), `InterfaceRole` (~913–955),
  node classification (~2897–2944) only; file is ~5.9k lines.
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` - range around
  `interface_layer_count_follows_config` (~839–990) only.
- `docs/specs/support-families-anchored-entities-plan.md` - §12 brief + §13 traps ranges.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `modules/core-modules/*/tests/golden/**` fixture bodies - never load; classify drift
  from summaries
- Packets 236–238b directories, `rectilinear-infill`, AGG surfaces, raft modules - other
  packets' scope
- `docs/specs/support-families-anchored-entities-plan.md` queue table - orchestrator-owned

## Expected Sub-Agent Dispatches

- Question: exact current text + neighbors of the `support_bottom_interface_spacing`
  blocks in both manifests; scope: `modules/core-modules/*/*.toml`; return: `SNIPPETS`;
  purpose: Step 1 red-baseline.
- Question: confirm `render_polygon` wall/fill behavior and `fill_pitch_honours_support_density`
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
  type. Manifest keys snake_case; undeclared keys silently default (E9/T8) so every read
  key must be declared.
- WIT boundary: canonical sources only (`crates/slicer-schema/wit/`); both `bindgen!` and
  guest macro read them; after edits `cargo build --tests` then rebuild guests.
- Determinism/scheduler constraints: no new claims; role attribution happens inside the
  existing planner claim window; serial/parallel determinism preserved (pure functions of
  plan entries + config).

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
- [BLOCK] None at authoring time; activation blocked only by 238b reaching `implemented`.
