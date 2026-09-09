---
status: implemented
packet: 238a-support-pattern-config-keys
task_ids:
  - TASK-472
  - TASK-473
  - TASK-474
  - TASK-475
  - TASK-476
  - TASK-477
---

# 238a-support-pattern-config-keys

## Goal

Declare and wire the support pattern/expansion/bottom-z/line-width config surface with
canonical semantics — typed host keys, manifest declarations that defeat T8 silent defaults,
bounds enforcement, one canonical layer-height transport rule — so 237/238b/238c consume
keys that provably exist.

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
- Sequencing hazard: packet 237's implementation touches `marshal/in_.rs` / `native.rs`
  (Step 6's surface) while in flight — land 237 before starting Step 6, or rebase the
  marshal edits onto its landed shape.
- `gen-config-docs --check` will fail CI if any step forgets the regen — that is the T8
  gate working, not a defect.
