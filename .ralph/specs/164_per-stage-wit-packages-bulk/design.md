# Design: 164_per-stage-wit-packages-bulk

## Controlling Code Paths

- Primary code path: the stage→package→typed-instantiate chain 163 built, now applied to layer + prepass. `crates/slicer-schema/wit/deps/<pkg>/<pkg>.wit` → `slicer_schema::STAGES` (sole lookup, ADR-0006) → per-stage `bindgen!` mods in `crates/slicer-wasm-host/src/host.rs` → `dispatch_layer_call` / `dispatch_prepass_call` in `crates/slicer-wasm-host/src/dispatch.rs` → per-stage glue builders in `crates/slicer-macros/src/lib.rs` → `xtask/src/build_guests.rs` per-stage staleness (163's `stage_wit_mtime`, unchanged — the 12 new dirs are resolved through `wit_dir_for_stage_id` automatically).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/contract/{wit_single_source_tdd.rs, wit_drift_detection_tdd.rs, dispatch_protocol_tdd.rs}`; `crates/slicer-runtime/tests/executor/*`; `crates/slicer-scheduler/tests/integration/{manifest_ingestion_tdd.rs, manifest_unknown_stage_tdd.rs}`; `crates/slicer-macros/tests/*`; the non-pilot `modules/core-modules/*/tests/slicer_module_binding_tdd.rs` (derive the set with `ls modules/core-modules/*/tests/slicer_module_binding_tdd.rs` — 14 of the 15 layer/prepass modules have one; `arachne-perimeters` does not and gains one here); hand-written guests `crates/slicer-wasm-host/test-guests/{prepass-guest, layer-infill-guest, infill-postprocess-echo-guest, path-optimization-multi-read}`; manifest-writing test helpers in `crates/slicer-scheduler/tests/` and `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs`.
- OrcaSlicer comparison: **none, deliberately.** Host/contract refactor of the WASM component system; OrcaSlicer has no module system, WIT, or plugin ABI. The `orca-delegation` snippet is omitted from `packet.spec.md` and `requirements.md` rather than filled with a fictional path (same reasoning 163 recorded).

## Grounded counts (falsifies the plan's arithmetic)

Verified against the tree at authoring time (`spec(163)` HEAD) — re-derive after 162/163 land:

- `slicer_schema::STAGES` (`crates/slicer-schema/src/lib.rs`, `pub const STAGES`) has **16 rows**: 8 `Layer::*`, 5 `PrePass::*`, 2 `PostPass::{GCode,Text}PostProcess`, 1 `PostPass::LayerFinalization`.
- `world-layer.wit` declares **8** stage exports (the ADR's "10 layer" counted the two lifecycle exports 162 deletes, then mislabeled the sum as stage count). `world-prepass.wit` declares **4** stage exports — there is **no** `run-paint-segmentation` export.
- `PrePass::PaintSegmentation` is host-built-in since packet 97 (`crates/slicer-runtime/src/prepass.rs` — "host built-in (sub-step 15 / AC-14)"; `PrepassExecutionError::PaintSegmentation`). No core module declares it (verified across all 20 manifests). Its `STAGES` row's `wit_export: "run-paint-segmentation"` names an export that exists nowhere; `docs/03_wit_and_manifest.md`'s `export run-paint-segmentation` listing is fiction against the `.wit`.
- Therefore: end state = **15 per-stage packages** (163's 3 + this packet's 12), not the plan's 17 and not "one per STAGES row" (16). 163's ledger phrase "the template #3 copies 13×" is also off by one: 12 glue builders (its own later sentence — "splits `Prepass` into 4 and `Layer` into 8 variants" — has the right arithmetic).
- Core-module stage census (drives AC-6/AC-N2 module choices): `Layer::Perimeters` {arachne-perimeters, classic-perimeters}; `Layer::PerimetersPostProcess` {fuzzy-skin, seam-placer}; `Layer::Infill` {gyroid-infill, lightning-infill, rectilinear-infill, top-surface-ironing}; `Layer::Support` {traditional-support, tree-support}; `Layer::SupportPostProcess` {support-surface-ironing}; `Layer::PathOptimization` {path-optimization-default}; `PrePass::LayerPlanning` {layer-planner-default}; `PrePass::SeamPlanning` {seam-planner-default}; `PrePass::SupportGeometry` {support-planner}. **No core module** exists for `Layer::SlicePostProcess`, `Layer::InfillPostProcess`, `PrePass::MeshAnalysis`, or `PostPass::TextPostProcess` — their packages still ship (test guests and third parties target them; `layer-infill-guest`, `infill-postprocess-echo-guest`, `prepass-guest`, `sdk-prepass-guest` exercise several).

## Architecture Constraints

- **Consume 163's decisions; do not re-derive them.** The naming rule (`slicer:<tier>-<stage-local-kebab>@1.0.0`, tier from `StageSpec.world_id`, never from splitting `stage_id`, never from `wit_export`), `wit_export == "run"`, the imported-`-types`/exported-`run` shape, fatal-on-miss with the engine's expected-only diagnostic, `@1.0.0` as mechanically load-bearing (`alternate_lookup_key` major-track requires major ≥ 1), and conservative `stage_wit_mtime` all come from `.ralph/specs/163_per-stage-wit-packages-pilot/design.md` §"Exports handed to packet #3".
- **A resource in an exported interface is guest-owned** (ADR-0045 §"The naive shape inverts resource ownership"). The four prepass resources are host-implemented today (`crates/slicer-wasm-host/src/host.rs` prepass resource impls), so each moves to that stage's **imported** `<iface>-types` interface. The 8 layer packages need **no** `-types` companion — `world-layer.wit` takes every type from the imported `slicer:ir-handles/ir-handles` / `slicer:config/config-types` / `slicer:common/module-errors`, exactly the shape 163 predicted ("most layer packages will need no `-types` companion at all" — confirmed: all 8).
- **One Rust type set across worlds (ADR-0002).** Every new `bindgen!` mod repeats the five-key `with:` block verbatim from 163's pilot mods. The new shared `slicer:prepass-types/prepass-types` interface follows the same discipline: one mod defines its bindings; every other mod importing it aliases via `with:`. Records need this as much as resources do — without the alias, `MeshObjectView` becomes two distinct Rust types and every host converter forks.
- **ADR-0006:** all new columns/lookups read `STAGES`; no parallel table anywhere (including `xtask`, which already goes through `wit_dir_for_stage_id`).
- **ADR-0015:** fatal-on-miss replaces the padding arms; no `Ok(())` stub survives for any stage. The `MissingComponent` laundering documented in DEV-087 is **out of scope and must not be widened or narrowed here** — reference it if touched, change nothing about it.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

### Selected approach

Copy 163's machinery 12 times; invent nothing. Every WIT type body moves **verbatim** from the tier world file into its new home, so host converters and impls change only in module path. The one genuinely new design element is `slicer:prepass-types` (below).

### WIT packages — exact contents

All type/resource bodies move verbatim from `world-layer.wit` / `world-prepass.wit`; func signatures keep their exact param lists with the func renamed to `run`.

**8 layer packages** — single exported interface each, no `-types` half. Template (from `world-layer.wit`'s `run-perimeters` row):

```wit
// crates/slicer-schema/wit/deps/layer-perimeters/layer-perimeters.wit
package slicer:layer-perimeters@1.0.0;

interface perimeters {
    use slicer:ir-handles/ir-handles.{layer-idx, slice-region-view,
        paint-region-layer-view, perimeter-output-builder};
    use slicer:config/config-types.{config-view};
    use slicer:common/module-errors.{module-error};

    run: func(layer-index: layer-idx, regions: list<slice-region-view>,
        paint: paint-region-layer-view, output: perimeter-output-builder,
        config: config-view) -> result<_, module-error>;
}

world perimeters-module {
    import slicer:common/host-services;
    import slicer:config/config-types;
    import slicer:ir-handles/ir-handles;
    export perimeters;
}
```

The other seven repeat this with their own `use` lists and signatures, copied from the corresponding `export run-*` line of `world-layer.wit`. Name table (package / interface / world — 163's rule, no exceptions):

| stage_id | package | interface | world |
|---|---|---|---|
| `Layer::SlicePostProcess` | `slicer:layer-slice-postprocess` | `slice-postprocess` | `slice-postprocess-module` |
| `Layer::Perimeters` | `slicer:layer-perimeters` | `perimeters` | `perimeters-module` |
| `Layer::PerimetersPostProcess` | `slicer:layer-perimeters-postprocess` | `perimeters-postprocess` | `perimeters-postprocess-module` |
| `Layer::Infill` | `slicer:layer-infill` | `infill` | `infill-module` |
| `Layer::InfillPostProcess` | `slicer:layer-infill-postprocess` | `infill-postprocess` | `infill-postprocess-module` |
| `Layer::Support` | `slicer:layer-support` | `support` | `support-module` |
| `Layer::SupportPostProcess` | `slicer:layer-support-postprocess` | `support-postprocess` | `support-postprocess-module` |
| `Layer::PathOptimization` | `slicer:layer-path-optimization` | `path-optimization` | `path-optimization-module` |
| `PrePass::MeshAnalysis` | `slicer:prepass-mesh-analysis` | `mesh-analysis` | `mesh-analysis-module` |
| `PrePass::LayerPlanning` | `slicer:prepass-layer-planning` | `layer-planning` | `layer-planning-module` |
| `PrePass::SeamPlanning` | `slicer:prepass-seam-planning` | `seam-planning` | `seam-planning-module` |
| `PrePass::SupportGeometry` | `slicer:prepass-support-geometry` | `support-geometry` | `support-geometry-module` |

`Layer::PerimetersPostProcess` deliberately does **not** package as `layer-wall-postprocess`: the rule keys on `stage_id`, and 163 repurposes `wit_export` (the only place "wall" appears) to `"run"`. The legacy divergence dies with the export name.

**4 prepass packages** — two-interface shape (imported `<iface>-types` + exported `run`-only), per 163's postpass-gcode template. Ownership of `world-prepass.wit`'s inline types:

- `prepass-mesh-analysis` / `mesh-analysis-types`: `facet-class`, `facet-annotation`, `surface-group-proposal`, `resource mesh-analysis-output`, plus a local `type object-id = string`.
- `prepass-layer-planning` / `layer-planning-types`: `region-layer-proposal`, `layer-proposal`, `resource layer-plan-output`, local `object-id`/`region-id` aliases.
- `prepass-seam-planning` / `seam-planning-types`: `seam-reason`, `scored-seam-candidate`, `seam-plan-entry`, `resource seam-planning-output`; `use slicer:prepass-types/prepass-types.{mesh-object-view}` in the exported interface's `run` signature.
- `prepass-support-geometry` / `support-geometry-types`: `support-plan-entry`, `layer-plan-view-entry`, `layer-plan-view`, `region-segmentation-view-entry`, `region-segmentation-view`, `support-geometry-view-entry`, `support-geometry-view`, `resource support-geometry-output`; uses `mesh-object-view` from `slicer:prepass-types`.
- The trivial `type` aliases (`object-id`, `region-id`, `layer-idx`) are duplicated locally where needed — they alias `string`/`s32` and carry no cross-package identity; duplicating them avoids a dependency for nothing. `layer-idx` is declared in `world-prepass.wit` today but appears in **no prepass signature or record** — the implementer verifies and drops it where unused rather than porting it ritually.

**`deps/prepass-types.wit`** (new, flat, **unversioned** — the same cross-package-resolution rule that keeps `slicer:common` unversioned, documented in `crates/slicer-schema/wit/README.md`):

```wit
package slicer:prepass-types;

interface prepass-types {
    use slicer:types/geometry.{point3};

    variant paint-value-view { flag(bool), scalar(f32), tool-index(u32) }
    record paint-stroke-view { triangles: list<point3>, semantic: string, value: paint-value-view }
    record paint-layer-view { semantic: string, facet-values: list<option<paint-value-view>>, strokes: list<paint-stroke-view> }
    record mesh-object-view { object-id: string, vertices: list<point3>,
        triangles: list<tuple<u32, u32, u32>>, paint-layers: list<paint-layer-view> }
}
```

Being flat under `deps/`, it is charged to **every** guest by 163's narrowed `compute_shared_mtime` — correct: it is genuinely shared, exactly like `ir-types.wit`. A breaking change to `mesh-object-view` affects both consumers by construction; that is honest, not a defect (they genuinely share the record — the alternative, duplication, hides the same coupling behind drift).

### slicer-schema (`crates/slicer-schema/src/lib.rs`)

- 12 rows: `wit_dir`/`wit_package`/`wit_interface`/`wit_world` per the name table, `wit_export: "run"`.
- `PrePass::PaintSegmentation` row: all five WIT columns `""`, comment `// Host-built-in since packet 97 (crates/slicer-runtime/src/prepass.rs); no WIT contract.` `export_for_stage_id` consequently returns `Some("")` for it — acceptable because no dispatcher reaches it (the host-builtin path in `prepass.rs` never consults the export table); `qualified_export_for_stage_id` and the three package lookups already return `None` on empty per 163's implementation. Guard: extend `stage_and_world_lookups_are_consistent` to assert exactly one row has empty WIT columns and it is `PrePass::PaintSegmentation`.
- Delete `SUPPORTED_WIT_WORLDS` (its rustdoc link to `WORLD_LIFECYCLE_EXPORTS` was already re-pointed by 162; deleting the const kills the last `wit-world` anchor).
- `WORLD_*` consts and `world_id` column: **kept**, doc comments corrected to "tier id (vocabulary); not a loadable WIT package since packet 164". See Open Questions for the recorded divergence from 163's expectation.
- Metadata qualification (163 `[FWD]` consumed): `SlicerModuleSchema.stage_export` doc + macro emission change to the qualified spelling `slicer:<pkg>/<iface>@1.0.0#run`; `ExportBinding.name` follows. One move, all 15 WASM stages, no half-qualified surface.

### slicer-wasm-host

- `host.rs`: `pub mod layer` (the `with:` alias **target** — its generated dep types are the canonical set per ADR-0002/packet 75) is replaced by 12 mods. **Pick `layer_perimeters` as the new canonical definer** of the five dep interfaces' Rust types; the other 11 (and 163's three pilot mods, whose `with:` blocks currently alias `layer`'s paths) re-point their `with:` values at `layer_perimeters`'s generated paths. This is the packet's riskiest mechanical edit: every `with:` value string in all 15 mods must name the same defining mod. `prepass_seam_planning` defines `slicer:prepass-types/prepass-types` bindings; `prepass_support_geometry` aliases them (sixth key). Prepass resource impls (`mesh-analysis-output` etc.) and layer resource plumbing re-point module paths only — type bodies moved verbatim, so `impl Host*` bodies are unchanged.
- `dispatch.rs`: `dispatch_layer_call` currently does linker-setup → `host::LayerModule::instantiate` → `call_layer_export`'s `match stage_id` (the seam ADR-0045 names). Rewire: the `match stage_id` moves **up** to select the per-stage world — each arm does `add_to_linker` + `instantiate` + the interface accessor's `call_run(...)` with that stage's existing marshalling body (the code inside today's `call_layer_export` arms moves, unmodified, into the corresponding new arm). `CallConfig.bindings: &host::LayerModule` disappears; `LayerParams` and the IR marshalling stay as-is. Same restructuring for `dispatch_prepass_call`. Every `TypedInstantiation` error arm gains 163's reason pattern: `format!("{e}; module does not export the interface required by stage {stage_id}: {}", slicer_schema::qualified_export_for_stage_id(stage_id).unwrap_or_default())`.

### slicer-macros (`crates/slicer-macros/src/lib.rs`)

- `StageGlueKind`: `Layer` → `LayerSlicePostprocess, LayerPerimeters, LayerPerimetersPostprocess, LayerInfill, LayerInfillPostprocess, LayerSupport, LayerSupportPostprocess, LayerPathOptimization`; `Prepass` → `PrepassMeshAnalysis, PrepassLayerPlanning, PrepassSeamPlanning, PrepassSupportGeometry` (15 variants total with 163's three).
- `resolve_stage_glue`: the layer/prepass stage_id arms map 1:1; the surviving `Some("LayerModule")`/`Some("PrepassModule")` trait-name fallbacks are **deleted** (a stageless impl gets no glue — completing what 163 did for postpass/finalization).
- `build_layer_world_glue` / `build_prepass_world_glue` (and their `include_str!` of the tier world files) are replaced by 12 builders copying `build_postpass_gcode_glue`'s shape: one `impl exports::slicer::<pkg_snake>::<iface_snake>::Guest for __Slicer<Stage>Component` with one `fn run`, no padding arms, `emit_world_preamble("<iface>-module", …, include_str!("../../slicer-schema/wit/deps/<pkg>/<pkg>.wit"))` (signature unchanged per 163's ledger). The prepass builders' preambles must also nest `prepass-types.wit` — follow how `emit_world_preamble` already nests the flat dep packages unconditionally; verify it picks up new flat deps automatically before adding anything.
- Metadata emission: `stage_export`/`__slicer_wit_exports()` switch to the qualified spelling (see schema bullet).

### wit-world retirement

- `crates/slicer-scheduler/src/manifest.rs`: delete the `wit_world` field (struct + builder + accessor), the `required_string(&root, manifest_path, "module.wit-world")` parse line, and `validate_wit_world`. A leftover `wit-world` key in a legacy TOML is simply never read (the parser reads named keys; verify no deny-unknown pass exists). Tests `wit_world_mismatch_rejects_invalid_package_name` and `versioned_wit_world_is_rejected_with_actionable_diagnostic` in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` are replaced by `wit_world_key_is_ignored` (AC-N3) — those are the real on-disk names, verified; docs/07's TASK-146 row cites stale pre-rename names (`wit_world_mismatch`, `wit_world_major_version_mismatch`), do not copy them; assertions like `module.wit_world()` at `manifest_ingestion_tdd.rs` and `execution_plan_tdd.rs` are deleted; test manifest-writer helpers across `crates/slicer-scheduler/tests/` and `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs` drop their `wit_world` parameters/lines (mechanical; enumerate via `rg -l 'wit_world' crates/ --type rust` at implementation time).
- 20 `modules/core-modules/*/*.toml`: delete the one `wit-world    = "…"` line each.
- `crates/pnp-cli/src/module_new.rs`: drop the scaffolded `wit-world` line and the "Expected WIT exports … in world" comment (replace with the qualified export via `slicer_schema::qualified_export_for_stage_id`); delete `wit_world_for_stage` and its `wit_world_mapping` test.

### Test guests

- Hand-written retargets (same recipe as 163's `postpass-guest`): `prepass-guest` (today targets `slicer:world-prepass/prepass-module`; keep only the stage(s) its consumers exercise — delegate a consumer survey first, and if it pads multiple prepass stages, split or narrow exactly as 163 narrowed `postpass-guest` to gcode-only), `layer-infill-guest` → `slicer:layer-infill/infill-module`, `infill-postprocess-echo-guest` → `slicer:layer-infill-postprocess/infill-postprocess-module`, `path-optimization-multi-read` → `slicer:layer-path-optimization/path-optimization-module`. Each: `generate!` `world:` retarget + `impl exports::slicer::<pkg>::<iface>::Guest` + `fn run`.
- Macro-authored guests (`sdk-layer-infill-guest`, `sdk-layer-pathopt-guest`, `sdk-prepass-guest`, plus all 15 core modules): regenerated by the macro, no source edits.
- Core-module `slicer_module_binding_tdd.rs` sweep — derive the file set at point of use (`ls modules/core-modules/*/tests/slicer_module_binding_tdd.rs`; 14 layer/prepass files at authoring time): expectation updates for `__slicer_stage_export_name() == "run"` and the qualified `stage_export` (exact new strings per the name table). **Plus one new file**: `modules/core-modules/arachne-perimeters/tests/slicer_module_binding_tdd.rs` — arachne-perimeters is the module ADR-0045 and AC-N2 use as the headline isolation example, yet it is the only layer/prepass module with no binding-surface guard; copy `classic-perimeters`' test shape (same stage, same expected strings). In scope because it is one small file, closes the sweep's only hole, and directly guards the surface this packet changes.

## Files in Scope (read + edit)

More than three primaries, unavoidably — the same six-layer simultaneous move as 163, at 4× the stage count. Split by layer, one layer per step (see `implementation-plan.md`).

- `crates/slicer-schema/wit/deps/{12 new dirs}/*.wit` + `deps/prepass-types.wit` — created; `deps/{world-layer,world-prepass}/` — deleted.
- `crates/slicer-schema/src/lib.rs` — 12 rows' columns, PaintSegmentation exception, `SUPPORTED_WIT_WORLDS` deletion, guard relaxation.
- `crates/slicer-wasm-host/src/host.rs` — 2 tier mods → 12 stage mods; canonical `with:` definer moves to `layer_perimeters`; prepass-types definer/alias.
- `crates/slicer-wasm-host/src/dispatch.rs` — layer + prepass runners restructured to per-stage instantiate; qualified-export reasons.
- `crates/slicer-macros/src/lib.rs` — 12 glue builders; fallback deletion; qualified metadata.
- `crates/slicer-scheduler/src/manifest.rs` — `wit_world` surface deleted.
- `crates/pnp-cli/src/module_new.rs` — scaffold cleanup.
- `modules/core-modules/*/*.toml` (20) — one-line mechanical edits; `modules/core-modules/*/tests/slicer_module_binding_tdd.rs` (derive set via `ls`; 14 exist) — expectation updates; plus one new `modules/core-modules/arachne-perimeters/tests/slicer_module_binding_tdd.rs`.
- `crates/slicer-wasm-host/test-guests/{prepass-guest,layer-infill-guest,infill-postprocess-echo-guest,path-optimization-multi-read}/src/lib.rs` — retargets.
- `crates/slicer-runtime/tests/contract/{wit_single_source_tdd.rs,wit_drift_detection_tdd.rs,dispatch_protocol_tdd.rs}`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` (+ the manifest-fixture helpers `rg` finds), `crates/slicer-macros/tests/*` — guard updates.
- `crates/slicer-schema/wit/README.md`, `docs/03_wit_and_manifest.md`, `CONTEXT.md`, `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md` — per Doc Impact.

## Read-Only Context

Line hints below were verified pre-162/163 and **will shift**; re-verify each against the post-163 tree at the moment of use (they are navigation hints, the symbol names are the citation).

- `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` (30 lines) and `world-prepass/world-prepass.wit` (149 lines) — read whole; they are the verbatim source of every moved signature/type.
- `crates/slicer-schema/wit/deps/prepass-types.wit` dependencies: `deps/types.wit` (`point3`) — the `use` targets.
- `.ralph/specs/163_per-stage-wit-packages-pilot/design.md` — §"Exports handed to packet #3", §"Naming", §"Data and Contract Notes" only.
- `crates/slicer-wasm-host/src/host.rs` — post-163: the `layer` and `prepass` `bindgen!` mods and their `with:` blocks; one pilot mod (`postpass_gcode`) as the copy template; the prepass resource `impl Host*` blocks. Ranged reads via `rg -n 'pub mod |bindgen!|with:' `.
- `crates/slicer-wasm-host/src/dispatch.rs` — `dispatch_layer_call`, `call_layer_export`, `dispatch_prepass_call` and its export router, plus one pilot runner (`dispatch_postpass_gcode_call`) as the pattern. Locate by symbol, read ±60.
- `crates/slicer-macros/src/lib.rs` — `resolve_stage_glue`, `StageGlueKind`, one pilot glue builder, `build_layer_world_glue` / `build_prepass_world_glue` heads and their per-stage match arms, `emit_world_preamble`. **Never read the file whole (~2900 lines).**
- `crates/slicer-scheduler/src/manifest.rs` — the `wit_world` field/accessor/builder sites and `validate_wit_world` (`rg -n 'wit_world' ` first, then ±20 windows).
- `crates/pnp-cli/src/module_new.rs` — the scaffold template and `wit_world_for_stage`.
- `docs/adr/0002-wit-marshalling-type-unification.md`, `docs/adr/0006-export-for-stage-id-sole-lookup.md` — whole (short).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — not applicable; a step tempted here is wrong.
- `target/`, `crates/slicer-wasm-host/test-guests/target/`, `Cargo.lock`, any `.wasm` — never load; inspect artifacts only via `wasm-tools component wit <path> | grep`.
- 163's deliverables as *edit* targets: `deps/{postpass-gcode-postprocess,postpass-text-postprocess,finalization-layer-finalization}/`, the three pilot glue builders, the three pilot dispatch runners, `stage_wit_mtime`/`compute_shared_mtime` in `xtask` — consumed, not modified (**exception:** the pilot mods' `with:` value strings in `host.rs` re-point when the canonical definer moves off `layer` — that one edit is in scope; nothing else in the pilot surface is).
- `crates/slicer-runtime/src/prepass.rs` (the PaintSegmentation host-builtin) — read-only evidence; no edits.
- `crates/slicer-runtime/src/run.rs` DAG-advisory surface (DEV-026), `dispatch.rs`'s `MissingComponent` conversion arms (DEV-087) — do not touch.
- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` — ranged reads of three sections only.
- Unrelated crates (`slicer-core`, `slicer-helpers`, `slicer-model-io`) — delegate symbol lookups.

## Expected Sub-Agent Dispatches

- Question: post-163, list every `docs/03_wit_and_manifest.md` line range mentioning `world-layer`, `world-prepass`, `run-paint-segmentation`, or `wit-world`; scope: `docs/03_wit_and_manifest.md`; return: `LOCATIONS` (≤20); purpose: the docs step.
- Question: which test files construct module manifests carrying `wit-world` or call `.wit_world()`, and where; scope: `crates/slicer-scheduler/tests/**`, `crates/slicer-runtime/tests/**`, `crates/pnp-cli/**`; return: `LOCATIONS` (≤20); purpose: the retirement step.
- Question: which consumers call into `prepass-guest`, and which prepass stage(s) do they exercise; scope: `crates/slicer-runtime/tests/**`, `crates/slicer-wasm-host/**`; return: `FACT` (≤5 lines); purpose: the test-guest retarget step.
- Question: does `emit_world_preamble` nest **all** flat `deps/*.wit` packages unconditionally (so `prepass-types.wit` is picked up with no macro change)?; scope: `crates/slicer-macros/src/lib.rs` (`emit_world_preamble` ±60); return: `FACT`; purpose: prepass glue step.
- Question: full `cargo check --workspace --all-targets` — pass, or first 3 distinct errors with `file:line`; scope: workspace; return: `FACT` + ≤20-line `SNIPPETS` on failure; purpose: the compile-gate step.
- Question: `cargo xtask test --summary --workspace` verdict; scope: workspace; return: `FACT` pass/fail + failing names only; purpose: acceptance ceremony. Never absorb the full output.
- Question: the deviation row owned by `164_per-stage-wit-packages-bulk` — its DEV id and current status line; scope: `docs/DEVIATION_LOG.md`; return: `FACT`; purpose: AC-8 closure (re-derive, never assume `DEV-086`).

## Data and Contract Notes

- **IR/manifest contracts:** `[stage] id` unchanged and still singular; `wit-world` deleted (legacy key tolerated-ignored — AC-N3); no IR schema or claim moves; config keys stay snake_case.
- **WIT boundary:** the central risk 163 retired mostly survives at larger n — resource identity across **15** `bindgen!` calls instead of 5. 163 proved the mechanism with real imports and `with:`-mapped resources; this packet adds two new wrinkles: (a) the canonical `with:` definer moves from the dying `layer` mod to `layer_perimeters`, so all 15 mods' alias value paths change in one sweep — a mismatch is a compile/link error of the `CLAUDE.md` §"WIT/Type Changes Checklist" kind, loud not subtle; (b) `slicer:prepass-types` is a *new* shared interface crossing two mods — the executor prepass round-trips (seam-planning and support-geometry both marshal `mesh-object-view`) are its falsifier. If either breaks in a way that cannot be fixed by path correction, stop and report — it would falsify 163's generalization, not just this packet.
- **Prepass resources stay `with:`-unmapped**, exactly like 163's pilot resources: bindgen markers + `ResourceTable` push. Do not add `with:` entries for them.
- **Determinism/scheduler:** untouched. `STAGE_ORDER`, DAG planning, `[stage] id` ingestion all unchanged; no WASM instantiated during planning (ADR-0006's rejected alternative stays rejected).
- **PaintSegmentation is the one deliberate non-package row.** Any future guard asserting "every STAGES row has a package" is wrong by design; the guard shape is "every row except the documented host-built-in set".

## Locked Assumptions and Invariants

- Package names/versions per the name table are a public contract at `@1.0.0` the moment this lands; breaking them later costs what ADR-0045 §Consequences says it costs. `every_stage_package_major_is_at_least_one` (163) already enforces `major >= 1` over the 12 new packages with zero new code.
- `slicer:prepass-types` is **unversioned** and shared — the same status as `slicer:common`/`slicer:ir-handles`; a breaking change to it is a cross-stage event by design.
- After this packet, `qualified_export_for_stage_id` is total over WASM stages and `None` exactly for unknown ids and `PrePass::PaintSegmentation`.
- Two-mechanism intermediate ends here; the deviation row closes (AC-8). No successor packet inherits contract-migration work.
- `stage_wit_mtime` stays conservative for stage-less guests. Never invert to make AC-N2 pass.
- Reversibility: breaking contract change, no flag; revert = revert the packet.

## Risks and Tradeoffs

- **The `with:` canonical-definer move is the highest-risk edit** (15 mods × 5-6 alias strings, all must agree). Mitigation: do it in the same step that creates the mods, gate with `cargo check -p slicer-wasm-host`, and treat any `imported interface ... has the wrong type` linker error as a path mismatch before suspecting the design.
- **Tree is red from the WIT step until dispatch lands** (six layers move together, as in 163). The step order pins this; no compile exit before the gate step.
- **`dispatch_layer_call` restructuring moves ~8 marshalling arms.** The arms' bodies are copy-moves, but the surrounding pool-lease/store/config-handle scaffolding is per-call and must be replicated per arm or hoisted; the step contract requires the executor suite (unfiltered) plus AC-N4 as the falsifier, and DEV-087's laundering arms must not be disturbed in the move.
- **`prepass-guest` may pad multiple prepass stages today.** Its consumer survey decides narrow-vs-split; wrong narrowing surfaces as a `0 passed` guard trip in the affected suite, which is why every name-filtered gate carries `rg -v '0 passed'`.
- **Manifest-fixture fallout is wide but shallow**: every test helper that writes `wit-world` or asserts `.wit_world()` breaks at compile time — enumerable via `rg`, mechanical to fix, but easy to under-count; the step allocates a dispatch for the inventory.
- **20 + 15 mechanical file edits** strain the ≤3-edits-per-step rule; the plan groups them into explicit sweep steps with per-file one-line contracts rather than pretending they fit.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (dispatch rewiring; `dispatch.rs` ~2500 lines, ranged reads only)
- Highest-risk dispatch and required return format: `cargo xtask test --summary --workspace` at the ceremony — `FACT` pass/fail + failing names only.

## Open Questions

- `[FWD]` **Divergence from 163's hand-off recorded:** 163 listed `WORLD_*` consts and `StageSpec.world_id` among things "still live for #3 to retire". This packet keeps them as tier vocabulary (doc comments corrected) because 30+ files consume them as tier identity, the naming rule reads the tier from `world_id`, and the plan's queue row for this packet never asked for their retirement. If a reviewer disagrees, that is a scope discussion for a follow-up packet, not silent widening here.
- `[FWD]` **`prepass-guest` narrowing** is decided by the consumer survey (dispatch listed above): keep only exercised stages, mirroring 163's `postpass-guest` precedent. If consumers exercise ≥2 prepass stages, prefer splitting into per-stage guests over re-adding padding.
- `[FWD]` **Test-guest per-stage staleness** (`[package.metadata.slicer] stage_id`, 12 `Cargo.toml`s) is deferred a second time: conservative over-rebuild is safe, this packet does not otherwise touch those 12 manifests, and the edit buys build time only. If an implementer finds the rebuild cost material during this packet, it is 12 mechanical edits plus a small `xtask` metadata read — do it in a trailing step, not mid-migration.
- `[FWD]` **`export_for_stage_id` returning `Some("")` for PaintSegmentation** is slightly ugly but honest to the table shape; if the implementer prefers `.filter(|e| !e.is_empty())` inside the lookup so it returns `None`, that is compatible with every consumer (dispatchers never dispatch a host-builtin stage) — verify the ADR-0006 guard test's expectation either way.
