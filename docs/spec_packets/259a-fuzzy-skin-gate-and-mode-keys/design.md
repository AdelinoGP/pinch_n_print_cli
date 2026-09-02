# Design: fuzzy-skin-gate-and-mode-keys

## Tier Derivation

**Tier B.** Map Authoring rule 1 requires a packet that builds a decision point to be re-tiered B or C. This packet builds three decision points, all inside one existing module, plus one IR enum variant and its two producers. No new module crate ships, so it stays below Tier C. The prior revision was Tier A with five of seven keys "declared-with-gap", a disposition rule 1 prohibits.

## Approach

Three keys, one gate function, one mode switch, one enabling IR change.

**The gate.** Canonical concentrates every fuzzy-skin decision in `should_fuzzify(cfg, layer_id, loop_idx, is_contour)`. The port has the same three inputs available at the same point: `run_wall_postprocess` receives `layer_index`, each `WallLoop` carries `perimeter_index` (canonical's `loop_idx`), and — once BLOCK-1 is resolved — `loop_type` carries contour-vs-hole. So the port ports the function verbatim as a private `should_fuzzify` in `modules/core-modules/fuzzy-skin/src/lib.rs` and calls it in place of today's `self.apply_to_all || wall.feature_flags.iter().any(|f| f.fuzzy_skin)` heuristic. `apply_to_all` disappears: `fuzzy_skin = "allwalls"` is its canonical spelling, and keeping both would give two ways to say one thing.

Note the current code also hard-restricts perturbation to `LoopType::Outer`. That restriction is subsumed by the gate — under `allwalls`, inner loops must fuzzify — so it is deleted, not kept alongside.

**The mode switch.** Canonical implements `fuzzy_skin_mode` only in `fuzzy_extrusion_line`, its Arachne path; `fuzzy_polyline`, the classic path, ignores the setting entirely. That asymmetry is an artifact of canonical's two-code-path structure, not a design intent — the Orca tooltip has to warn "Works only with Arachne!". This port has **one** path: `apply_fuzzy_skin` operates on `ExtrusionPath3D` + `WidthProfile` regardless of which generator produced the loop, and it already returns `out_widths`. So the mode switch goes into that one function and works under both `classic-perimeters` and `arachne-perimeters`. That is a recorded improvement (DIV-1), not a divergence to apologise for.

The three modes, from canonical `fuzzy_extrusion_line`:

| Mode | Point | Width |
| --- | --- | --- |
| `displacement` (default) | offset perpendicular by `r` | unchanged |
| `extrusion` | unchanged | `max(w + r + 0.01, 0.01)` |
| `combined` | offset perpendicular by `(rad - w) / 2` | `rad = max(w + r + 0.01, 0.01)` |

where `r` is the noise sample times thickness — today `rng.next_f32() * fuzzy_skin_thickness`, which packet 259b later replaces with a selectable generator.

`extrusion` mode implies one structural change: displacement mode inserts resampled subdivision points along each segment so the perturbation has somewhere to live. Extrusion mode moves nothing, so it must not insert them — it perturbs the widths of the existing vertices. AC-N4 pins that.

**The IR change.** This is the only part that leaves the module. See BLOCK-1.

## Controlling Code Paths

- `FuzzySkinModule::run_wall_postprocess` (`modules/core-modules/fuzzy-skin/src/lib.rs`) — the loop over `region.wall_loops()`; where the gate replaces the heuristic and the `LoopType::Outer` restriction is deleted.
- `apply_fuzzy_skin` (same file) — the perturbation itself; where the mode switch lands and where the single `rng.next_f32() * fuzzy_skin_thickness` noise sample lives.
- `FuzzySkinModule`'s `LayerModule::from_config` impl (same file) — gains the three new fields, loses `apply_to_all`.
- `Rng::next_f32` (same file) — returns `[-1.0, 1.0]`, matching canonical `UniformNoise::GetValue`. Unchanged by this packet.
- `LoopType` and `WallLoop` (`crates/slicer-ir/src/slice_ir.rs`) — the enum gaining the variant and the struct carrying it.
- `classic-perimeters` and `arachne-perimeters` (`modules/core-modules/*/src/lib.rs`) — the two producers that must populate the new variant.

## What Carries the New Data

- The three config values travel as ordinary module config keys declared on `fuzzy-skin.toml`, read in `LayerModule::from_config` — no new carrier.
- The mode's width output travels in `WidthProfile.widths`, which `apply_fuzzy_skin` already builds and returns — no new carrier.
- The contour/hole fact travels in `WallLoop::loop_type`, which exists but lacks the variant. **This is the one new carrier, and it is both an IR schema change and a WIT interface change (`enum wall-loop-type` in `crates/slicer-schema/wit/deps/ir-types.wit` mirrors `LoopType`) — BLOCK-1.**

## Recorded Divergences (port improves on or intentionally differs from canonical)

- **DIV-1 — `fuzzy_skin_mode` applies to both wall generators.** Canonical switches on the mode only in `fuzzy_extrusion_line` (Arachne); the classic path silently ignores it. The port applies it in the single `apply_fuzzy_skin` used by both, so the setting means the same thing regardless of `wall_generator`. Rationale: the port has one perturbation path, and honouring a user's setting on only half the generators is a defect to inherit, not a behaviour to reproduce. Needs a `docs/DEVIATION_LOG.md` row — re-derive both the ID **and the convention** from `docs/DEVIATION_LOG.md` at the moment of writing: the log carries two schemes — a dominant `DEV-###` series and a minority `D-<packet>-<SLUG>` series — so follow whichever the recent rows use rather than assuming.
- **DIV-2 — degenerate-value handling stays at the point of use.** Canonical forces `fuzzy_skin = None` in `region_config_from_model_volume` when `fuzzy_skin_point_distance < 0.01` or `fuzzy_skin_thickness < 0.001`. The port's `apply_fuzzy_skin` already returns its input unchanged when either value is non-finite or non-positive, which is behaviourally equivalent where it matters and keeps config resolution free of geometry knowledge. Recorded, not re-implemented.
- **DIV-3 — `apply_to_all` is retired rather than kept.** The PnP-invented boolean is exactly `fuzzy_skin = "allwalls"`. Keeping both would leave two spellings of one decision and a silent precedence question. AC-N3 requires the removal to be loud, not silent.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **`slicer-ir` is a fingerprint input for every guest.** The `LoopType` change makes **all** core-module guests stale, not just `fuzzy-skin`'s. Budget for a full `cargo xtask build-guests` rebuild, not a targeted one.
- **Match-exhaustiveness blast radius.** Adding a `LoopType` variant breaks every non-wildcard `match` on it across the workspace. The step that adds the variant owns finding and fixing all of them; `cargo check --workspace --all-targets` is the discovery tool, and it must be run inside that step rather than left to a later one.
- **Struct-literal churn gate.** `WallLoop` is a watched type; every test-code literal needs a `..` rest or an `// exhaustive: <reason>` waiver (`docs/21_data_defaults_and_fixtures.md`). `cargo xtask check-literals` enforces it.

## Code Change Surface

- `modules/core-modules/fuzzy-skin/fuzzy-skin.toml` — three `[config.schema]` tables added, `apply_to_all` removed.
- `modules/core-modules/fuzzy-skin/src/lib.rs` — three fields on `FuzzySkinModule`, `apply_to_all` removed; new private `should_fuzzify`; `run_wall_postprocess` gate replacement and deletion of the `LoopType::Outer` restriction; the mode switch inside `apply_fuzzy_skin`.
- `modules/core-modules/fuzzy-skin/tests/fuzzy_skin_tdd.rs` — AC-2, AC-3, AC-4, AC-N4 arms.
- `modules/core-modules/fuzzy-skin/tests/fuzzy_config_schema_tdd.rs` — net-new (AC-1, AC-N3); needs `toml = "0.8"` as a dev-dependency in `modules/core-modules/fuzzy-skin/Cargo.toml` (add-if-absent).
- `crates/slicer-ir/src/slice_ir.rs` — the `LoopType` variant and, if BLOCK-1 resolves that way, the schema-version constant.
- `crates/slicer-ir/tests/loop_type_hole_tdd.rs` — net-new (AC-5).
- `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs` — populate the new variant.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — AC-6 arms.
- `crates/slicer-runtime/tests/integration/` — AC-7 arms in the CONFIG_BLOCK suite.
- `docs/DEVIATION_LOG.md` — one row for DIV-1.
- `docs/03_wit_and_manifest.md` — the `LoopType` mention and schema-range guidance, if the version moves.
- `docs/15_config_keys_reference.md` — regenerated, never hand-edited.
- Guest `.wasm` artifacts — all of them.

## Files in Scope (read + edit)

The change surface above is the authoritative list. No file outside it may be edited.

## Read-Only Context

- `crates/slicer-sdk/src/views.rs` — `PerimeterRegionView`, `WallLoop` accessors used by `run_wall_postprocess`.
- `docs/04_host_scheduler.md` — bounds enforcement behaviour for enum keys.
- `docs/spec_packets/259b-fuzzy-skin-noise-modules/design.md` — via a SUMMARY dispatch only, to confirm the noise seam this packet must leave intact.

## Out-of-Bounds Files

- `crates/slicer-gcode/src/serialize.rs` — `ORCA_CONFIG_PADDING`. Zero diff lines (AC-N2, map Authoring rule 2).
- `crates/slicer-schema/wit/deps/ir-types.wit` — **in bounds only once BLOCK-1 is accepted.** Verified at authoring: `enum wall-loop-type` mirrors `LoopType` there and is consumed by `record wall-loop-view`. The WIT edit is part of BLOCK-1, not separate from it; no other WIT file is touched.
- Every other packet directory under `docs/spec_packets/`.
- `docs/specs/orca-feature-gap/map.md` and `issues/**` — read-only; required updates are reported, not applied.
- `docs/15_config_keys_reference.md` — generated; regenerate, never hand-edit.

## Expected Sub-Agent Dispatches

- **`should_fuzzify` and `fuzzy_extrusion_line`** — `SUMMARY` ≤ 200 words + ≤ 3 snippets ≤ 30 lines. The single most important canonical read in the packet; the gate must be ported clause for clause.
- **`LoopType` match sites** — `LOCATIONS` ≤ 20: every non-wildcard `match` on `LoopType` across the workspace, for the blast radius.
- **Schema-version constant** — `FACT` ≤ 5 lines: which constant governs `PerimeterIR`/`SliceIR` and its live value, re-derived at the moment of use. Never freeze a version in this document.
- **Deviation ID** — `FACT` ≤ 5 lines: the next free `D-` ID, re-derived from `docs/DEVIATION_LOG.md` at the moment of writing.
- **Cargo runs** — all delegated with a `FACT pass/fail` return.

## Data and Contract Notes

- Config key strings are snake_case (`CLAUDE.md` § Config Key Naming Convention).
- Manifest enum tables use `type = "enum"` with a `values` list; rejection of an out-of-list value is the scheduler's `TypeMismatch`, exercised by AC-6.
- `perimeter_index` is the port's `loop_idx`. Canonical's `loop_idx == 0` test means "outermost loop of its own contour", so a hole's outermost loop is also index 0 — the gate must not conflate `perimeter_index == 0` with "the region's outer wall".

## Locked Assumptions and Invariants

1. `apply_fuzzy_skin` returns a per-vertex width vector today, so `extrusion` and `combined` need no new carrier — verified at authoring.
2. `Rng::next_f32` is already `[-1, 1]`, so the noise sample's sign symmetry matches canonical — verified at authoring. Do not "fix" it.
3. Default configuration is `fuzzy_skin = "disabled_fuzzy"`, which perturbs nothing, so AC-N1 must stay byte-identical throughout.
4. The gate's `loop_idx` is `WallLoop::perimeter_index`, and `is_contour` is the new `LoopType` fact — not `WallBoundaryType`, which does not carry it.
5. Deleting the `LoopType::Outer` restriction is required, not optional: `allwalls` must reach inner loops.

## Risks and Tradeoffs

- **BLOCK-1 dominates the packet.** Two of the six `fuzzy_skin` values (`hole`, `all`) and the correctness of the other four depend on the IR change. If it is refused, the fallback is to ship `none` / `external` / `allwalls` / `disabled_fuzzy` and reject `hole` / `all` by name — a smaller but still rule-1-compliant packet. That fallback is written out under BLOCK-1 so the decision can be taken without re-authoring.
- **Blast radius of a `LoopType` variant** is wide and cheap to underestimate. Mitigated by owning it in one step with `cargo check --workspace --all-targets` as the discovery tool.
- **Retiring `apply_to_all`** is a user-visible config break. AC-N3 forces the break to be loud.

## Context Cost Estimate

**M aggregate.** The largest single step is the `LoopType` variant plus its blast radius (M).

## Open Questions

### `[BLOCK]` BLOCK-1 — hole-loop identification requires **both** an IR schema change and a WIT interface change

Canonical `should_fuzzify` takes `is_contour` as a parameter. This tree cannot supply it: `LoopType` is `Outer | Inner | ThinWall | NonPlanarShell | GapFill` and `WallBoundaryType` is `ExteriorSurface | MaterialBoundary { segments } | Interior` — verified against `crates/slicer-ir/src/slice_ir.rs` at authoring. Neither distinguishes a contour from a hole.

The user's ruling for this packet is to **add a `LoopType::Hole` variant**. Two of this queue's three blocker triggers fire, both verified against the tree at authoring:

1. **IR schema change.** It changes a public IR enum, is likely to require a schema-version bump, breaks match exhaustiveness workspace-wide, and makes every guest `.wasm` stale (`slicer-ir` is a fingerprint input for all of them).
2. **WIT interface change — verified, not hypothetical.** `LoopType` is mirrored across the component boundary as `enum wall-loop-type { outer, inner, thin-wall, nonplanar-shell, gap-fill }` in `crates/slicer-schema/wit/deps/ir-types.wit`, consumed by `record wall-loop-view`'s `loop-type` field. Adding a Rust variant without adding the matching WIT case produces exactly the class of failure `CLAUDE.md` § WIT/Type Changes Checklist warns about: type identity mismatch across the boundary, surfacing as guest instantiation or linking failures rather than a compile error at the edit site. Both host (`bindgen! path:`) and the guest macro (`include_str!`) read `crates/slicer-schema/wit/` directly, so there is one file to edit and no inline copy — but it is a WIT edit, and the packet may not make one on its own authority.

**The packet is authored around the change but must not be activated until an architecture owner accepts both.**

Unresolved sub-questions, all of which must be answered before activation:

1. **Does the variant require a schema-version bump?** Adding an enum variant is a backward-compatible read for older producers but not for older consumers. Re-derive which constant governs the IR carrying `WallLoop` and decide the bump then; do **not** freeze a version number here.
2. **Is `Hole` a `LoopType` variant or an orthogonal flag?** `Outer` / `Inner` describe *depth*; contour / hole describes *which boundary the loop follows*. They are independent axes: a hole has an outer and inner loop too. Making `Hole` a fifth peer of `Outer` / `Inner` conflates them and makes "the inner loop of a hole" unrepresentable — which canonical *can* represent (`loop_idx` and `is_contour` are separate arguments to `should_fuzzify`). A `bool is_hole` field on `WallLoop`, or a `LoopBoundary { Contour, Hole }` field, models it correctly. **The reviewer should treat the user's `LoopType::Hole` ruling as a decision to add the distinction to the IR, and settle the exact shape at activation** — this design does not silently pick a different shape, but it records that the peer-variant form loses information canonical keeps. Note that the orthogonal-field form is also the *cheaper* WIT change: adding a field to `wall-loop-view` avoids touching an enum every existing guest matches on.
3. **Which guests match on `wall-loop-type` today?** A `LOCATIONS` sweep of guest-side matches is a Step 1 precondition; each is a site that must gain an arm.

**Fallback if the block is refused:** ship `none`, `external`, `allwalls`, `disabled_fuzzy`; reject `hole` and `all` by name with an error naming the missing feature ("hole-loop identification"); return those two *values* (not the key) to the queue. `fuzzy_skin` stays a class-(b) key with a live gate, and the packet drops to a pure single-module diff with no IR surface.

- `[FWD]` Painted-region fuzzy promotion (canonical `PrintApply.cpp` `generate_print_object_regions` forcing `All` on brush-painted regions, via `fuzzy_skin_segmentation_by_painting` and `apply_fuzzy_skin_segmentation`) needs a fuzzy-skin paint semantic this tree does not have. Returned to the queue in `requirements.md`.
- `[FWD]` The `ripple` noise type and its three keys are packet 259b's scope and are outside ticket 14's key list; the map/ticket update is reported in this session's handoff.
