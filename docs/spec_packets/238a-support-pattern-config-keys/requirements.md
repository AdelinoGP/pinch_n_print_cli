# Requirements: 238a-support-pattern-config-keys

## Packet Metadata

- Grouped task IDs: `TASK-363` … `TASK-368`
- Backlog source: `docs/specs/support-parity-gap-register.md`
- Packet status: `implemented`
- Aggregate context cost: `M`

## Motivation

The support config surface is a lattice of silent defaults and dead transports, each
measured and registered:

1. **G-03 — pattern keys declared-and-dead or absent.** The traditional planner declares
   `support_base_pattern` as an unconstrained string (default `"rectilinear"`,
   `traditional-support-planner.toml`) that only feeds a provenance label; the reference
   profile value `rectilinear` with spacing 2 cannot be expressed because
   `support_base_pattern_spacing` is not declared anywhere in the tree.
2. **G-04 — `support_expansion` has a consumer but no host declaration.** Its consumption
   already exists (`detect_support_contacts` step 6,
   `crates/slicer-core/src/algos/overhang_annotation.rs`; producer plumbing in
   `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
   `resolve_contact_params`), but the host config surface does not declare the canonical
   key, so profiles cannot set it portably.
3. **G-05 — bottom-z is a G-code lie.** PnP honors only the top-Z distance; canonical
   `support_bottom_z_distance` (default 0.2) exists solely as a hardcoded literal in
   `crates/slicer-gcode/src/serialize.rs`'s config-block table, while
   `execute_support_geometry` (`crates/slicer-core/src/algos/support_geometry.rs`)
   hardcodes `DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM`. Neither reaches geometry.
4. **G-08 — `support_line_width` is three unrelated things.** The tree planner declares a
   plain-mm float (default 0.35, min 0, max 2) consumed as the `get_max_move_dist` cap; the
   G-code header emits a hardcoded 0.35 from `DefaultGCodeSerializer.support_line_width`;
   canonical makes it a `coFloatOrPercent` over nozzle diameter, default 0 = auto via
   `Flow::auto_extrusion_width(frSupportMaterial, nozzle_diameter)`. Divergence 5.4: PnP has
   no flow model, so this packet decides the key-based mapping and records the deviation.
5. **G-09 — one run, two layer heights.** `project_layer_plan_view`
   (`crates/slicer-wasm-host/src/marshal/in_.rs`) derives `effective_layer_height` as MAX
   across participating objects; `build_native_prepass_request`
   (`crates/slicer-wasm-host/src/marshal/native.rs`) takes FIRST-MATCH. The same run can
   hand guests different heights per transport. Additionally
   `execute_support_geometry` stamps `support_layer_height_mm: 0.0` into every
   `SupportGeometryIR` regardless of resolved config.
6. **G-16 + divergence 3.1 — read-but-undeclared keys.** The tree planner reads
   `support_branch_merge_distance_mm` and `support_max_branches_per_layer` from config;
   neither is declared in its manifest, so T8's filtered-config-view mechanism silently
   discards any user-supplied value. `max_bridge_length` is consumed through the undeclared
   fallback constant `DEFAULT_MAX_BRIDGE_LENGTH_MM` (= 10.0). `support_style` is read for
   the slim branch but undeclared.
7. **Issue-20/37 intersecting keys.** `bridge_no_support`, `enforce_support_layers`,
   `support_critical_regions_only`, `support_remove_small_overhang`,
   `support_threshold_overlap`, `support_object_first_layer_gap`,
   `support_sharp_tails` have behaviors landing in
   237/238b but no declarations — this packet is their declaration home (plan §3 Ruling 5).

## In Scope

Fully owned by this packet:

- **Tree-planner manifest declarations** (`tree-support-planner.toml`):
  `support_branch_merge_distance_mm` (float 0.8, min 0), `support_max_branches_per_layer`
  (int 1024, min 1, max 10000), `max_bridge_length` (float 10.0, min 0),
  `support_style` (enum with values `default|grid|snug|organic|tree_slim|tree_strong|tree_hybrid`
  and default `"default"` — declare-only; behavior is 238b).
- **Traditional-planner manifest declarations**
  (`traditional-support-planner.toml`): `support_base_pattern_spacing` (float, default
  2.5, min 0.1, max 10); `support_base_pattern` documented against the canonical enum set
  `default|rectilinear|rectilinear-grid|honeycomb|lightning|hollow`.
- **Host typed keys** (`declare_resolved_config!`, `resolved_config.rs`):
  `support_expansion` (f32 0.0), `support_top_z_distance` (f32 0.2),
  `support_bottom_z_distance` (f32 0.2), `support_threshold_overlap` (float_or_percent
  50%), `support_line_width` (float_or_percent 0 = auto), plus declaration-only
  `bridge_no_support` (bool false), `enforce_support_layers` (int 0),
  `support_critical_regions_only` (bool false), `support_remove_small_overhang`
  (bool true), `support_object_first_layer_gap` (f32 0.2),
  `support_sharp_tails` (bool true — canonical `g_config_support_sharp_tails`
  (`libslic3r.h`) is a developer constant set `true`; consuming behavior is 237) —
  all eleven routed through
  `docs/config/host-keys.toml`.
- **Geometry wiring of the two z-distances:** `execute_support_geometry` reads resolved
  `support_top_z_distance` / `support_bottom_z_distance` instead of hardcoding;
  `SupportGeometryIR.support_layer_height_mm` carries the resolved region target instead of
  literal 0.0. Bottom-z *planner semantics* stay with 238b/238c; this packet makes the value
  real and transported.
- **G-09 canonical transport rule:** ONE shared helper derives `effective_layer_height`
  (MAX rule, preserved from the wasm leg) used by BOTH marshal legs; the native first-match
  path is deleted. RC-11's prohibition on dividing by the field stands untouched.
- **Divergence-5.4 decision recorded:** `support_line_width` re-typed float →
  float_or_percent on both declaring manifests; default 0 resolves to `nozzle_diameter`
  (the `frSupportMaterial` mapping), percent resolves against `nozzle_diameter`; the
  tree-planner mm reader migrates to `ConfigView::get_abs_value`-style resolution; the
  G-code serializer's header field sources its value from resolved config instead of the
  0.35 literal. Deviation row logged for the no-flow-model divergence itself.
- **Bounds enforcement:** every new numeric declaration carries min/max so
  `ConfigBoundsIndex` rejects out-of-range input at resolution time (negative ACs below).
- **Doc regen:** `cargo xtask gen-config-docs && cargo xtask gen-config-docs --check` in
  the same commits as manifest changes (T8).

## Out of Scope

- **Pattern-generator algorithms** (non-rectilinear base fills, honeycomb/lightning/hollow
  generators, canonical `SupportMaterial.cpp` pattern stage): the renderer half of the
  absorbed stub belongs to `238c-support-renderer-flow-interfaces`; this packet declares the
  keys and documents the enum, it does not implement fills. (The stub file
  `docs/spec_packets/stubs/stub-support-patterns-expansion-bottom-z.md` was already
  deleted from the tree before this packet authored — commit `48d09a36`; nothing here to
  preserve; 238c consumes its renderer half later.)
- **Raft keys** (`raft_contact_distance`, `raft_expansion`, `raft_first_layer_expansion`)
  → 240 per plan §3 Ruling 5.
- **Ironing keys (issue 22)** and **filament keys (issue 38)** → feature-gap track.
- **`support_style` behavior** (style normalization, tree-style rejection, strong/hybrid
  branches) → 238b; here it is declare-only.
- **Renderer density consumption** of `support_base_pattern_spacing` and flow/density
  corrections (divergences behind G-10/G-11) → 238c.
- **`bridge_no_support` and `enforce_support_layers` consuming behavior** → 237
  (AC-3/AC-4 there); dependency runs BOTH ways — this packet owns the declaration, 237 owns
  the stage behavior, and neither may fork the key name.
- **Bottom-z gap planner fidelity** (actual floor-interface geometry under
  `support_bottom_z_distance`) → 238b (tree)/238c (interfaces); DEV-129/DEV-145 corrections
  are 238c's.
- **Support-area rasterizer choice** → 241.
- **`docs/07_implementation_status.md` content edits beyond the packet-owned closure step's
  registration rows**, DEVIATION_LOG rows other than this packet's own new row.

## Cross-Packet Dependencies

- Depends on `236-support-stabilization` — FORWARD DEPENDENCY (draft at this packet's
  authoring; re-derive 236's `packet.spec.md` frontmatter at activation rather than
  trusting this label): composition is limited to coexisting in the scheduler/runtime
  graph; this packet must NOT
  touch `crates/slicer-scheduler/src/validation.rs` (236-owned) nor revert 236's
  orderability guards. Steps touching shared scheduler surfaces (Step 7 bounds ACs) gate
  their green-light composition checks on 236 reaching `implemented` if 236's landed shape
  diverges.
- Unblocks `238b-tree-planner-canonical-fidelity` (declared `support_style`,
  `max_bridge_length`, bottom-z/top-z values now transportable), `238c` (pattern-spacing +
  enum documentation, line-width semantics), and closes 237's `[FWD]→238a`
  `bridge_no_support` spelling question: the declared spelling is exactly
  `bridge_no_support`.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - ~755 lines; §12 brief "238a",
  §3 rulings, §7 E-standards, §13 traps (direct ranged reads at authoring time)
- `docs/specs/support-parity-gap-register.md` - six rows (small; direct read)
- `docs/15_config_keys_reference.md` - generated; never hand-edited, regenerate via xtask
- `docs/ORCA_CONFIG_REFERENCE.md` - pre-derived canonical config table (ranged lookups)
- `docs/spec_packets/224-support-family-orca-closure/design.md` - §RC-11 only (ranged read)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::build` /
  `init_fff_params`: canonical type/default/range for every key in §In Scope (coEnum /
  coFloat / coFloatOrPercent / coBool / coInt values restated in `packet.spec.md`)
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `s_keys_map_SupportMaterialPattern`
  and `s_keys_map_SupportMaterialStyle`: exact enum-string sets for the two enum-ish keys
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `auto_extrusion_width(frSupportMaterial,
  nozzle_diameter)` and `opt_key_to_flow_role`: canonical auto-width derivation backing the
  divergence-5.4 key-based mapping decision
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — ctor style
  normalization/rejection: what the declared `support_style` value set must admit
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — ctor `is_strong` /
  hybrid contact minting: confirms the style surface 238b will consume

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7`.
- Negative: `AC-N1` through `AC-N5`.
- Cross-packet impact: declarations consumed by 237 (`bridge_no_support`,
  `enforce_support_layers`), 238b (`support_style`, `max_bridge_length`,
  merge-distance/branch-cap keys), 238c (spacing/pattern/line-width); G-09 fix removes a
  cross-transport skew any guest could observe.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only gate commands. Every
cargo-test row is the pipe-suffixed guarded form (tee to `target/test-output.log`,
successful-result grep, in-run non-zero matched-count guard) and matches the corresponding
`packet.spec.md` AC command exactly.

| When | Command | Notes |
| --- | --- | --- |
| Manifest shape (AC-1) | `rg -q '^\[config\.schema\.support_branch_merge_distance_mm\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '^\[config\.schema\.support_max_branches_per_layer\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '^\[config\.schema\.max_bridge_length\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '^\[config\.schema\.support_style\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'tree_strong' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'tree_hybrid' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'support_branch_merge_distance_mm' docs/15_config_keys_reference.md && rg -q 'max_bridge_length' docs/15_config_keys_reference.md && echo PASS \|\| echo FAIL` | four tables asserted individually |
| Tree planner guest (AC-2) | `mkdir -p target && cargo test -p slicer-runtime --test executor support_config_surface_tdd::max_bridge_length_config_reaches_tree_planner -- --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | guest dispatch through compiled module |
| Pattern spacing (AC-6) | `rg -q '^\[config\.schema\.support_base_pattern_spacing\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q 'rectilinear-grid' docs/15_config_keys_reference.md && echo PASS \|\| echo FAIL` + the CLI schema-view probe (implementation-plan.md Step 3) | declaration surfaces through `module config-schema` |
| Host keys (AC-3) | `rg -q 'support_bottom_z_distance' crates/slicer-ir/src/resolved_config.rs && rg -q 'support_threshold_overlap' docs/config/host-keys.toml && rg -q 'support_object_first_layer_gap' docs/15_config_keys_reference.md && rg -q 'support_remove_small_overhang' docs/15_config_keys_reference.md && echo PASS \|\| echo FAIL` | typed fields + doc routing |
| Geometry distances (AC-4) | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_geometry_config_surface_tdd support_geometry_ir_carries_resolved_distances -- --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | E6/T5: `--features host-algos` mandatory; new gated target |
| Serializer width (AC-7) | `mkdir -p target && cargo test -p slicer-gcode --lib serialize::tests::support_line_width_header_sources_resolved_value -- --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | in-file `#[cfg(test)]` module via `--lib` |
| Both-legs contract (AC-5) | `mkdir -p target && cargo test -p slicer-wasm-host --test contract layer_height_transport_tdd::native_and_wasm_layer_views_share_canonical_layer_height -- --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | T9 leg-skew guard; new file registered in `contract/main.rs` |
| Bounds negative 1 (AC-N1) | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd::rejects_max_bridge_length_below_min -- --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | `ConfigResolutionError::OutOfRange`, `config_bounds_enforcement_tdd.rs` precedent |
| Bounds negative 2 (AC-N2) | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd::rejects_support_max_branches_per_layer_zero -- --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | min-1 floor matches planner clamp |
| Bounds negative 3 (AC-N3) | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd::rejects_negative_support_branch_merge_distance -- --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | declared min 0 |
| Regression net (AC-N4) | `cargo test -p slicer-runtime --test executor --no-fail-fast 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | historical injectors survive |
| Doc freshness (AC-N5) | `cargo xtask gen-config-docs --check && echo PASS \|\| echo FAIL` | T8 same-commit rule |
| Type gate | `cargo check --workspace --all-targets` | struct-literal blast radius |
| Lint gate | `cargo clippy --workspace --all-targets -- -D warnings` | required before commit |
| Guest freshness attribution | `cargo xtask build-guests --check` FIRST (exit 0 fresh / 1 stale / 3 infra) | E4/T4 |

## Step Completion Expectations

- Steps are ordered so each verification runs green without later steps; no step leaves the
  workspace uncompiling.
- Every manifest `[config.schema]` addition and its `docs/15_config_keys_reference.md`
  regeneration land in the SAME step/commit (T8).
- The G-09 helper extraction precedes the native-leg rewiring; the wasm leg switches onto
  the helper first so behavior is provably unchanged before the native leg flips.

## Context Discipline Notes

- `crates/slicer-gcode/src/serialize.rs` is large; read ranged slices around the
  `DefaultGCodeSerializer` fields and the support-header writer only.
- `modules/core-modules/tree-support-planner/src/lib.rs` is very large (>5000 lines);
  ranged reads at the cited symbols only, never full loads.
- Never load `OrcaSlicerDocumented/**` directly (E7/T1); delegate per the snippet above.
