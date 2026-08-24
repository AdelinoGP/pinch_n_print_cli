# Requirements: 237-support-analysis-parity

## Motivation

Host support analysis is not canonical-faithful in three ways, each measured and registered:

1. **G-17 — `needs_support` carries no signal.** `classify_object`
   (`crates/slicer-core/src/algos/mesh_analysis.rs`) hardcodes
   `needs_support: true` on every synthesized `OverhangRegion`, and
   `SliceRegionView::Default`/`from_ir` (`crates/slicer-sdk/src/views.rs`) hardcode the flag
   `true` as well. No producer ever sets it false, so planner- and renderer-side consumers of
   the documented eligibility precedence (`crates/slicer-sdk/src/traits.rs` `run_support`
   doc-comment) consume a constant. Packet 224 decision 2 kept the renderer-side inversion and
   deleted the vacuous test `enforcer_overrides_needs_support_false`; this packet restores the
   signal by real classification (E1), never by resurrecting that test.
2. **Divergence 5.2 — enforcers ignored under auto.**
   `commit_support_analysis_builtin`
   (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) calls `enforcer_contacts`
   only when `!support_type.is_auto()`. Canonical `detect_contacts`
   (`SupportMaterial.cpp`) runs its enforcer branch whenever `has_enforcer`
   (`annotations.enforcers_layers[layer_id]` non-empty) with **no** support-type gate; the
   auto gate applies only to `detect_overhangs`' angle-thresholded branch.
3. **Divergence 5.3 — five missing stages.** `detect_support_contacts`
   (`crates/slicer-core/src/algos/overhang_annotation.rs`) implements diff → expand-back →
   blockers → tiny-spot filter → XY expansion → union and self-documents a "Not modelled"
   list: sharp-tail detection, buildplate-only subtraction, bridge removal, the cantilever
   pass, and `enforce_support_layers` forcing.

## Scope (Authoritative)

In scope, fully owned by this packet:

- **G-17 producer:** `classify_object` derives `needs_support` from whether the object's
  overhang facets' XY projection (`OverhangRegion.xy_footprint`) overlaps the region's
  polygons, instead of the hardcoded `true`. Regions whose cross-section is disjoint from all
  overhang footprints classify ineligible.
- **G-17 view derivation:** new `SliceRegionView::derive_needs_support(surface_classification)`
  consults `SurfaceClassificationIR.per_object[object_id].overhang_regions[].xy_footprint`
  against the region's polygons; wired into `SliceRegionView::from_ir` callers so both
  transport legs carry the derived value:
  - native leg: `build_native_layer_request`
    (`crates/slicer-wasm-host/src/marshal/native.rs`) — note: today it does NOT read
    `input.surface_classification`; this packet threads the field into it (mirroring
    `sliced_region_to_data`, which already does)
  - wasm leg: `sliced_region_to_data` (`crates/slicer-wasm-host/src/marshal/in_.rs`)
  - guest-side shim: the `slicer-macros` region-view adapter
    (`crates/slicer-macros/src/lib.rs`) already forwards `r.needs_support()` — it must keep
    doing so after the derivation lands.
  `Default::default()` intentionally keeps `needs_support: true` (legacy fixture
  compatibility; the comment at views.rs documents why).
- **G-17 consumer:** candidate gating inside `commit_support_analysis_builtin`: an
  auto-detected (thresholded) contact is suppressed when the source region's derived
  eligibility is false and no enforcer covers it; enforcer-derived contacts are exempt
  (canonical precedence order). The structured per-region `family_assignments` minting
  (236-owned, Ruling 1) still records an assignment for every RegionMap region regardless of
  suppression.
- **Divergence 5.2 fix:** under any `support_type`, run the enforcer branch whenever enforcer
  polygons exist for the region; under auto additionally run the thresholded branch and union
  both geometries. Correct the contradicting `SupportType::NormalAuto` doc comment
  (`crates/slicer-ir/src/slice_ir.rs`) which claims manual-only enforcers apply to
  `detect_contacts`.
- **Divergence 5.3, four of five stages implemented here:**
  - sharp-tail detection at first layer, opt-in key behavior. Transitional choice: until
    238a declares the key, absence resolves to an in-code default of OFF (AC-N1); canonical
    `g_config_support_sharp_tails` (`libslic3r.h`) is a developer constant set `true`, so
    once 238a declares `support_sharp_tails` (bool true) the stage is on by default;
  - bridge removal via a port of `SupportMaterialInternal::remove_bridges_from_contacts`,
    gated by bridge-no-support behavior;
  - post-union cantilever pass recording wide spans (`dist_max > scale_(3)` = 3 mm via
    `mm_to_units`) into additive `SupportAnalysisIR.cantilever_surfaces`;
  - `enforce_support_layers` forcing full contacts (zero lower-layer offset) for leading
    layers.
  The fifth stage, buildplate-only subtraction, is **not silently cut**: see Out of Scope.
- **Schema bump:** `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` receives a minor-version bump
  for the additive map (live constant today: 1.1.0; the bump is derived at activation, and no
  packet artifact asserts a frozen literal — tests and docs reference the constant and derive
  the expectation at activation time),
  with the blast radius pre-baked into the owning step.

## Cross-Packet Dependencies

- **`bridge_no_support` AND `support_sharp_tails` KEY DECLARATIONS belong to
  238a-support-pattern-config-keys** (`bridge_no_support` via the plan §12 issue-20
  intersecting-key list; `support_sharp_tails` added to that list by cross-packet
  reconciliation — canonical spelling from `PrintConfig.cpp`, default `true` matching the
  `g_config_support_sharp_tails` constant in `libslic3r.h`); the **CONSUMING BEHAVIOR lands
  here**
  as AC-3/AC-N1/AC-N2. Until 238a declares the manifest keys, this packet drives the stages
  from the
  typed/in-code parameters only — no manifest edit, no `docs/15_config_keys_reference.md`
  regeneration in this packet (T8/E9 discipline stays with 238a).
- Depends on `236-support-stabilization` — a FORWARD dependency on a `status: draft` packet:
  composes with its per-region
  `family_assignments` minting in `commit_support_analysis_builtin`; this packet must NOT
  revert minting to per-candidate and must NOT touch
  `crates/slicer-scheduler/src/validation.rs` write-conflict logic (236 owns it; read-only  here).
- Unblocks 238a (declared keys get consuming behavior already present), 238b/238c (consume the
  corrected candidate stream; 238c consumes `cantilever_surfaces`).

## Out of Scope

- **Buildplate-only subtraction (divergence 5.3 stage b): [FWD]** — the host analysis contract
  does not transport buildplate-covered annotations today (no `buildplate_covered` equivalent
  in `SurfaceClassificationIR`, `SliceIR`, or the WIT boundary). Implementing it would require
  a new host→stage data channel, which is a contract decision beyond this packet's slice.
  Recorded as `[FWD]` in `design.md` with recommendation to resolve alongside 242 (or as a
  reasoned deviation logged then); NOT a silent cut.
- Planner geometric fidelity (tree smoothing/top-Z gap/styles) → 238b; renderer flow/interface
  fidelity and DEV-129/DEV-145 corrections → 238c.
- Support-area rasterizer choice → 241.
- `crates/slicer-scheduler/src/validation.rs` write-conflict logic → 236-owned, read-only
  here.
- Manifest `[config.schema]` declarations and `docs/15_config_keys_reference.md`
  regeneration → 238a.
- The deleted vacuous test `enforcer_overrides_needs_support_false` stays deleted
  (AC-N4 guards this).

## References

- Plan: `docs/specs/support-families-anchored-entities-plan.md` §12 brief "237" ; rulings §3;
  invariants §6 (15, 16); evidence standards §7 (E1–E9); traps §13 (T1, T4–T6, T8, T9).
- Gap register: `docs/specs/support-parity-gap-register.md` row G-17 (destination now this
  packet).
- Divergences: `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md`
  5.2 and 5.3.
- Absorbed stub: `docs/spec_packets/stubs/stub-support-eligibility-classification.md`
  (deleted at authoring; its goal and G-17 ownership are restated in this file's Motivation).

## AC-ID Summary

| AC | Surface | Requirement bullet |
| --- | --- | --- |
| AC-1 | div 5.2 routing | Enforcer contacts union into candidates under auto |
| AC-2 / AC-N1 | div 5.3 sharp tails | First-layer tail contacts when enabled / none when disabled |
| AC-3 / AC-N2 | div 5.3 bridge removal | Bridge area removed under gate / kept when disabled |
| AC-4 / AC-N3 | div 5.3 enforce layers | Forced-full leading-layer contacts / no forcing beyond model |
| AC-5 | div 5.3 cantilever + schema | Wide-span annotations recorded; additive `cantilever_surfaces`; minor bump derived from live constant (no frozen literal) |
| AC-6 | G-17 view derivation | `derive_needs_support` returns false for disjoint footprint |
| AC-7 | G-17 both legs | Native + wasm marshalling deliver derived flag |
| AC-8 / AC-N5 | G-17 consumer + regression | Ineligible region emits no auto candidates, assignments intact; manual routing unchanged |
| AC-N4 | E1 guard | Vacuous test stays deleted; renderer inversion intact |

## Verification Matrix

Every command satisfies invariant 16 (non-zero matched tests asserted in-run) and tees to
`target/test-output.log`.

| When | Command | Notes |
| --- | --- | --- |
| Any slicer-core run | `cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd <NAME> -- --exact` | E6/T5: bare `-p slicer-core` compiles gated tests to zero — never trust it |
| Producer unit tests | `cargo test -p slicer-runtime --lib <NAME> -- --exact` | source-module `#[cfg(test)]` tests are reachable ONLY via `--lib`; the `tests/unit/` aggregator does not mount them |
| Both-legs contract | `cargo test -p slicer-wasm-host --test contract region_eligibility` | T9 leg-skew guard |
| End-to-end decline | `cargo test -p slicer-runtime --test integration needs_support_false_region_yields_no_auto_candidates -- --exact` | aggregator binary `integration` |

Every cargo-test row appends the in-run zero-guard
`test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (or an explicit `passed`
count grep) after tee, so a zero-match run can never read green.
| Guest-facing failure attribution | `cargo xtask build-guests --check` FIRST (exit 0 fresh / 1 stale / 3 infra) | E4/T4; rebuild before re-running if stale |
| Type gate | `cargo check --workspace --all-targets` | catches struct-literal blast radius of the schema bump |
| Lint gate | `cargo clippy --workspace --all-targets -- -D warnings` | required before commit |
| Whole-suite (closure only) | `cargo xtask test --summary --workspace -- --no-fail-fast` | E5/T3; only at acceptance ceremony per AGENTS Test Discipline |

Cross-step expectations:

- Steps are ordered core → IR → runtime so each step's verification runs green without later
  steps; no step leaves the workspace uncompiling.
- The schema-bump step (cantilever) and the eligibility-consumer step both touch
  `support_analysis_producer.rs`; the later step must rebase onto the earlier one's shape,
  not duplicate edits.
- All new slicer-core tests open under the existing gated harness conventions
  (`required-features = ["host-algos"]` already declared for
  `support_overhang_detection_tdd` in `crates/slicer-core/Cargo.toml`); new test names are
  authored red-first by their cited steps (invariant 16).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `detect_overhangs`: the
  divergent stages (sharp-tail detection gated `g_config_support_sharp_tails`;
  buildplate-only subtraction of `annotations.buildplate_covered[layer_id]`;
  `remove_bridges_from_contacts` under `bridge_no_support`; the post-union cantilever pass
  recording `layer.cantilevers` when `dist_max > scale_(3)`; `lower_layer_offset = 0` forcing
  under `enforce_support_layers`), plus the tiny-spot filter and the
  `support_threshold_overlap` overlap-offset alternative already mirrored
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `detect_contacts`: the
  enforcer branch keyed purely on `has_enforcer` with no support-type/auto gate (the
  canonical half of divergence 5.2)
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` —
  `SupportMaterialInternal::remove_bridges_from_contacts`: the bridge-area subtraction whose
  semantics AC-3 ports
