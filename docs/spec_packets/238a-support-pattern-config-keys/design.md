# Design: 238a-support-pattern-config-keys

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` — the four new
    `[config.schema]` declarations (G-16/div 3.1); sibling reader
    `SupportPlanner::from_config` (`modules/core-modules/tree-support-planner/src/lib.rs`,
    `config.get("support_line_width")` / `"support_max_branches_per_layer"` /
    `"max_bridge_length"` consumption sites) migrates to percent-aware resolution.
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` —
    `support_base_pattern_spacing` declaration + `support_base_pattern` enum documentation.
  - `crates/slicer-ir/src/resolved_config.rs` — `declare_resolved_config!` entries for the
    eleven host keys; `to_config_map` pass-through keeps them reaching guests via extensions.
  - `crates/slicer-core/src/algos/support_geometry.rs` — `execute_support_geometry`
    replaces `DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM` and the `support_layer_height_mm: 0.0`
    literal with resolved-config values.
  - `crates/slicer-wasm-host/src/marshal/in_.rs` (MAX rule, kept) +
    `crates/slicer-wasm-host/src/marshal/native.rs` (FIRST-MATCH, deleted) — both legs call
    one shared helper after this packet.
  - `crates/slicer-gcode/src/serialize.rs` — `DefaultGCodeSerializer.support_line_width`
    sources from resolved config; the dead `("support_bottom_z_distance", "0.2")` literal
    in the config-block table is replaced by the real resolved value.
- Neighboring tests/fixtures:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — the
    OutOfRange precedent (`resolve_global_config` + `BoundsDeclaration`) the negative ACs
    extend; lives in the named `scheduler_integration` binary.
  - `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs`,
    `crates/slicer-runtime/tests/integration/support_geometry_config_normalization_tdd.rs`
    — historical guest-config injectors of the two previously-undeclared keys
    (`support_branch_merge_distance_mm = 0.8`, `support_max_branches_per_layer = 1024`);
    AC-N4 proves declaration does not break them.
  - `modules/core-modules/tree-support-planner/tests/diagnostics_tdd.rs` — injects
    `support_max_branches_per_layer = 1024` module-side; stays green (value in-bounds).
  - `crates/slicer-core/tests/support_geometry_ir_shape_tdd.rs` — constructs
    `SupportGeometryIR`; struct-literal blast-radius candidate for any IR field change
    (none planned — field exists).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not
  repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. (This packet edits `modules/core-modules/*/src/**`, `modules/core-modules/*/[Ct]*.toml`, and `crates/slicer-ir/**` — all inside the snippet's applicability list.)
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`. (Concretely here: every declared min/max/default in this packet is millimetres or unitless counts — canonical's `coFloat` defaults transfer as-is with no ÷100 scaling because PnP manifests are mm-native; only code crossing into scaled-integer geometry uses `mm_to_units()`.)
- T8 same-commit rule: a manifest `[config.schema]` entry and its
  `docs/15_config_keys_reference.md` regeneration land in ONE commit (a past deletion,
  `4d1848eb`, left the doc stale). The gen-config-docs `--check` gate is part of Verification.
- E9 snake_case: all declared key strings are snake_case in manifests, host-keys.toml,
  runtime lookups, and docs — no kebab-case anywhere.

## Divergence Decisions (recorded)

- **Divergence 5.4 (G-08) — key-based mapping chosen.** Canonical derives support width
  from a flow model (`Flow::auto_extrusion_width(frSupportMaterial, nozzle_diameter)`
  returns `nozzle_diameter` for support roles; `opt_key_to_flow_role` maps
  `support_line_width` → `frSupportMaterial`). PnP has no flow model. DECISION:
  `support_line_width` becomes `float_or_percent` on every declaring surface; default `0`
  resolves to `nozzle_diameter` (the frSupportMaterial auto result); an explicit percent
  resolves against `nozzle_diameter`; an explicit mm value passes through. The divergence
  itself (no flow model → widths are key-derived rather than flow-derived) is logged as a
  DEVIATION_LOG row by this packet. This mirrors the established percent-transport pattern
  (`classic-perimeters` float_or_percent + `ConfigView::get_abs_value`, packet 184/185).
- **G-09 — MAX rule chosen as the single canonical layer-height transport rule.**
  Rationale: MAX is the wasm leg's existing behavior (all participating objects observed at
  the global layer), it is order-independent (FIRST-MATCH depends on HashMap iteration
  order of `object_participation`), and it degrades toward the larger height (safer for
  consumers still dividing heights). One shared helper in the marshal module emits the
  value for both legs; the native first-match closure is deleted. RC-11's prohibition on
  DIVIDING by `effective_layer_height` stands untouched — consumers walk actual layer Z;
  this decision only unifies what the field carries.
- **Bottom-z scope split:** this packet makes `support_bottom_z_distance` a real,
  transported, bounds-enforced value (host key + serializer truth); planner/renderer
  semantics that consume it stay with 238b/238c per the queue plan.

## Code Change Surface

- Selected approach: declaration-first wiring. Each surface gets canonical type + default +
  range at its owning manifest/host-key location, then the minimal consumer migration so no
  declared key remains read-but-undeclared or declared-but-dead.
- Exact functions, traits, manifests, tests, fixtures:
  - `tree-support-planner.toml` `[config.schema]`: add
    `support_branch_merge_distance_mm`, `support_max_branches_per_layer`,
    `max_bridge_length`, `support_style`.
  - `traditional-support-planner.toml` `[config.schema]`: add
    `support_base_pattern_spacing`; document `support_base_pattern`'s value set.
  - `resolved_config.rs` `declare_resolved_config!`: add the eleven host keys (§In Scope);
    retype nothing existing (the tree-planner manifest retype of `support_line_width`
    float→float_or_percent is manifest-side; its reader migrates in the same step).
  - `execute_support_geometry` (`support_geometry.rs`): accept a resolved-config input (or
    region-resolved values already computed upstream) for top/bottom z-distance and
    support layer height; delete both literals.
  - marshal `in_.rs`/`native.rs`: extract `canonical_effective_layer_height(plan, index)`
    helper (MAX rule); native leg consumes it.
  - `DefaultGCodeSerializer` (`serialize.rs`): constructor takes/supports the resolved
    support width; header writer emits it; delete the hardcoded 0.35 and the dead
    bottom-z/top-z/expansion/pattern table literals' hardcoding (values become sourced).
- Rejected alternatives and reasons:
  - *Flow-model port* (derive widths like canonical from extrusion math): rejected — a
    flow model is a cross-cutting feature beyond this packet's declaration slice; recorded
    as deviation instead (Ruling 8: pure defect fixes get no knob, but this is a missing
    capability, not a defect fix).
  - *FIRST-MATCH as the G-09 canonical rule*: rejected — order-dependent on
    `object_participation` iteration; MAX is deterministic and already the wasm-leg
    behavior (smaller diff, no wasm-leg churn).
  - *Declaring raft keys alongside*: rejected — Ruling 5 assigns them to 240; declaring
    here would fork ownership.
  - *Enum-typed manifest schema for `support_base_pattern`*: rejected — the manifest
    schema language supports string today; documenting the canonical value set plus bounds
    validation where supported avoids inventing new schema machinery in a config packet.

## Files in Scope (read + edit)

Target at most 3 primary files per step; the full packet spans:

- `modules/core-modules/tree-support-planner/tree-support-planner.toml` - role: four key
  declarations; expected change: +4 `[config.schema]` tables (support_style as an enum
  carrying the full canonical value set incl. `tree_strong`/`tree_hybrid`).
- `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` -
  role: spacing declaration + pattern enum doc; expected change: +1 table, comment block.
- `crates/slicer-ir/src/resolved_config.rs` - role: eleven typed host keys; expected change:
  `declare_resolved_config!` additions (+`to_config_map` rows if keyed output needed).
- `crates/slicer-core/src/algos/support_geometry.rs` - role: de-hardcode distances;
  expected change: signature/input plumbing + two literal deletions (Step 5a; AC-4's new
  gated target `support_geometry_config_surface_tdd` registered in
  `crates/slicer-core/Cargo.toml`).
- `crates/slicer-runtime/src/builtins/support_geometry_producer.rs` - role: builtin caller
  of `execute_support_geometry`; expected change: pass the resolved top-z distance through
  (Step 5a).
- `crates/slicer-wasm-host/src/marshal/in_.rs`, `.../native.rs` - role: G-09 helper;
  expected change: extract helper, delete first-match branch.
- `crates/slicer-gcode/src/serialize.rs` - role: source support width from config;
  expected change: builder field + header emission.
- Test homes and their registrations:
  - `crates/slicer-runtime/tests/executor/support_config_surface_tdd.rs` (new) +
    `crates/slicer-runtime/tests/executor/main.rs` (`mod
    support_config_surface_tdd;`) — Step 1 red tests.
  - `crates/slicer-core/tests/support_geometry_config_surface_tdd.rs` (new) + its
    `[[test]]` `required-features = ["host-algos"]` registration in
    `crates/slicer-core/Cargo.toml` — Step 5a-2 (AC-4 home).
  - `crates/slicer-wasm-host/tests/contract/layer_height_transport_tdd.rs` (new) +
    `crates/slicer-wasm-host/tests/contract/main.rs` (`mod
    layer_height_transport_tdd;`) — Step 6-2 (AC-5 home).
  - scheduler integration bucket (`config_bounds_enforcement_tdd.rs`, named binary
    `scheduler_integration`) — Step 7 bounds negatives.
Justified extras: each is a thin mechanical surface owned by exactly one step.

## Read-Only Context

- `docs/specs/support-families-anchored-entities-plan.md` - §3 rulings, §12 "238a" brief,
  §13 traps only - authority for decisions.
- `docs/spec_packets/224-support-family-orca-closure/design.md` - §RC-11 only - the
  walk-actual-Z prohibition preserved verbatim.
- `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` -
  divergence 5.4 row only.
- `modules/core-modules/tree-support-planner/src/lib.rs` - cited symbol ranges only
  (`from_config` ~1395–1445, constants ~79–160, `sample_contact_points` ~3533–3560);
  >5000 lines, never full-load.
- `docs/ORCA_CONFIG_REFERENCE.md` - ranged row lookups only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load (E7/T1).
- `target/`, `Cargo.lock`, generated code, vendored dependencies, guest build artifacts -
  never load.
- Other packet directories under `docs/spec_packets/` - never modify (Packet Safety); the
  absorbed stub `stubs/stub-support-patterns-expansion-bottom-z.md` is NOT deleted (238c
  owns its renderer half).
- `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md` content beyond this packet's
  own registration/deviation rows.
- `crates/slicer-scheduler/src/validation.rs` write-conflict logic - 236-owned; read-only.
- Renderer/planner algorithm bodies (`tree-support-planner` smoothing/merge internals) -
  behavior is 238b; this packet touches only config reading at the cited sites.

## Expected Sub-Agent Dispatches

- Question: enumerate every test/fixture asserting the current manifest table set for
  tree-support-planner and traditional-support-planner (shape assertions, count asserts,
  golden TOML dumps); scope: `crates/**/tests/**`, `xtask/**`; return: `LOCATIONS` ≤20;
  purpose: pre-bake the Step 2/Step 3 blast radius instead of cargo-check discovery.
- Question: confirm the exact `declare_resolved_config!` extension mechanics for adding a
  float_or_percent key (extractor name, percent-transport precedent fields); scope:
  `crates/slicer-ir/src/resolved_config.rs` lines 840–1000 + classic-perimeters precedent;
  return: `SNIPPETS` ≤30 lines; purpose: Step 4 authoring.
- Question: locate all readers of `DefaultGCodeSerializer.support_line_width` and the
  construction site(s) feeding it from the pipeline; scope: `crates/slicer-gcode/**`,
  `crates/slicer-runtime/**`, `crates/pnp-cli/**`; return: `LOCATIONS` ≤20; purpose:
  Step 7 wiring without breaking constructors.
- Question: verify no other caller duplicates the FIRST-MATCH layer-height derivation;
  scope: `crates/slicer-wasm-host/src/marshal/**`; return: `FACT`; purpose: Step 5 safety.

## Data and Contract Notes

- IR/manifest contracts: NO WIT change. All eleven host keys ride the existing
  `ResolvedConfig.extensions` pass-through into guest config maps; the four tree-planner
  declarations ride the standard filtered config view once declared (T8 mechanism —
  declaration IS the transport). No IR schema bump: `SupportGeometryIR` fields exist; only
  their values change provenance.
- WIT boundary: untouched; guest artifacts may still flip fingerprint staleness via the
  dependency-closure walk — hence the freshness gate before attribution.
- Determinism/scheduler constraints: the G-09 MAX helper must preserve the wasm leg's
  exact numeric output (same partial_cmp tie-break) so existing goldens do not drift; the
  native leg CHANGES value on multi-object layers by design (that is the fix) — covered by
  the AC-5 contract test, not by goldens.

## Locked Assumptions and Invariants

- Invariant 16/T2: every verification command asserts non-zero matched tests in-run.
- E9: snake_case everywhere; undeclared-key silence is the defect being fixed (T8/G-16),
  so every consumed-by-module key in this packet's surface ends the packet DECLARED.
- E8: manifest values are mm-native; no ÷100 conversions appear in declarations.
- RC-11 stands: nobody divides by `effective_layer_height`; consumers walk actual Z.
- 224 decision context: `support_on_build_plate_only` stays untouched; existing declared
  keys keep their ranges unless this packet retypes them (`support_line_width` only).
- No frozen future schema/version literals anywhere; version expectations derive from live
  constants (none bumped by this packet).

## Risks and Tradeoffs

- Retyping `support_line_width` on the tree-planner manifest can reject previously-accepted
  plain-mm configs if percent parsing is mishandled — mitigated by migrating the reader to
  the established `get_abs_value` percent path and keeping plain-mm accepted.
- Declaration of formerly-silent keys means user profiles supplying those keys START taking
  effect (T8 inverse): intended, but called out for the human gate (non-default profile
  exercises exactly this).
- The G-09 native-leg value change alters multi-object-layer inputs to native-dispatched
  guests; single-object prints are unaffected (first match == max there).
- `gen-config-docs --check` will fail CI if any step forgets the regen — that is the T8
  gate working, not a defect.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4 — eleven host keys + extractor mechanics)
- Highest-risk dispatch and required return format: manifest-shape blast radius —
  `LOCATIONS` ≤20.

## Open Questions

- `[FWD]` Whether `max_bridge_length` should also be declared on
  `traditional-support-planner.toml` (canonical reads it family-wide; PnP's only consumer
  today is the tree planner's `sample_contact_points`). Recommendation: declare tree-side
  now; traditional side when 238c wires bridge removal there. Implementer-resolvable with
  the dispatch record; does not gate activation.
- `[FWD]` `support_threshold_overlap` percent-vs-mm resolution currently happens
  producer-side (`resolve_contact_params`); whether the host key should carry a resolved-mm
  mirror field. Recommendation: no — keep the raw float_or_percent and resolve at the
  consumer, matching 184/185 precedent. Recorded decision suffices.
- Implementer-resolvable: exact `min` for `support_base_pattern_spacing` (proposed 0.1)
  vs canonical's absence of a range — pick the value that keeps the reference profile's 2
  legal and record it in the step notes.
