---
status: draft
packet: 164_per-stage-wit-packages-bulk
task_ids:
  - TASK-146c
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 164_per-stage-wit-packages-bulk

## Goal

Migrate every remaining WASM-dispatched stage — 8 layer stages and 4 prepass stages (12 packages; `PrePass::PaintSegmentation` is host-built-in since packet 97 and gets none) — onto the per-stage versioned WIT-package machinery packet 163 built, and retire the now-unfalsifiable tier-world surface: the manifest `wit-world` key, `SUPPORTED_WIT_WORLDS`, and `validate_wit_world`.

## Scope Boundaries

This packet replaces `slicer:world-layer@2.0.0` and `slicer:world-prepass@1.0.0` with 12 per-stage packages at `@1.0.0` (plus one new **unversioned** shared dep package `slicer:prepass-types` for the view records two prepass stages share), rewires schema, host `bindgen!`, dispatch, macro glue, test guests and docs onto the stage→package→typed-instantiate path 163 proved, and deletes the `wit-world` manifest key end to end. It applies 163's decisions — the naming rule, `wit_export == "run"`, the imported-`-types`/exported-`run` WIT shape, the five-key `with:` block, fatal-on-miss — without re-litigating any of them. It also closes the two-mechanism-intermediate deviation row 163 files. The `pnp_cli` binary-locator extraction stays with packet `165_cli-binary-locator-extraction`.

## Prerequisites and Blockers

- Depends on: `163_per-stage-wit-packages-pilot` (TASK-146b) — **must be IMPLEMENTED, not merely generated**, which itself requires 162 implemented. Every symbol this packet consumes is cited in its post-163 form, taken from `.ralph/specs/163_per-stage-wit-packages-pilot/design.md` §"Exports handed to packet #3" (the contract) — `StageSpec.{wit_dir,wit_package,wit_interface,wit_world}`, `qualified_export_for_stage_id`, `StageGlueKind`/`resolve_stage_glue`, the `build_postpass_gcode_glue`/`build_postpass_text_glue` glue template, `GuestSpec.stage_id` + `stage_wit_mtime` in `xtask`, and the per-stage `bindgen!` mod pattern in `crates/slicer-wasm-host/src/host.rs`. Starting against a tree where 163 has not landed produces spurious conflicts in every file this packet touches. Step 0 of `implementation-plan.md` verifies the tree before any edit.
- Unblocks: nothing structurally (165 depends only on 162), but until this packet lands two contract mechanisms are live in-tree — the accepted intermediate 163 records as a deviation row owned by this packet.
- Activation blockers: none. `[FWD]` entries in `design.md` are implementer-resolvable.

## Acceptance Criteria

- **AC-1. Given** the canonical WIT tree post-163 holds `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` (8 stage exports) and `deps/world-prepass/world-prepass.wit` (4 stage exports; `PrePass::PaintSegmentation` has none — host-built-in since packet 97), **when** they are replaced by 12 per-stage packages named by 163's rule (`slicer:<tier>-<stage-local-part-kebab>@1.0.0`, tier read from `StageSpec.world_id`), **then** each of the 12 dirs `deps/<d>/<d>.wit` for `d` in `layer-slice-postprocess, layer-perimeters, layer-perimeters-postprocess, layer-infill, layer-infill-postprocess, layer-support, layer-support-postprocess, layer-path-optimization, prepass-mesh-analysis, prepass-layer-planning, prepass-seam-planning, prepass-support-geometry` exists; each declares exactly one `package slicer:<d>@1.0.0;`, exactly one `run: func`, and exactly one `world`; zero `resource` declarations appear in any exported interface (a resource there is guest-owned — ADR-0045 §"The naive shape inverts resource ownership"); the four prepass packages each declare an **imported** `<iface>-types` interface holding their `resource` (`mesh-analysis-output`, `layer-plan-output`, `seam-planning-output`, `support-geometry-output`); the shared view records (`mesh-object-view`, `paint-value-view`, `paint-stroke-view`, `paint-layer-view`) live once in the new flat `deps/prepass-types.wit` (`package slicer:prepass-types;`, **unversioned** like `slicer:common`); and neither `deps/world-layer/` nor `deps/world-prepass/` survives. | `cd F:/slicerProject/pinch_n_print_cli && python3 -c "
import re,os
B='crates/slicer-schema/wit/deps'
L=['layer-slice-postprocess','layer-perimeters','layer-perimeters-postprocess','layer-infill','layer-infill-postprocess','layer-support','layer-support-postprocess','layer-path-optimization']
P={'prepass-mesh-analysis':'mesh-analysis','prepass-layer-planning':'layer-planning','prepass-seam-planning':'seam-planning','prepass-support-geometry':'support-geometry'}
IF={**{d:d.split('-',1)[1] for d in L},**P}
bad=[]
def block(s,name):
    m=re.search(r'(?m)^interface '+re.escape(name)+r'\s*\{',s)
    if not m: return None
    i=m.end()-1;d=0
    for j in range(i,len(s)):
        if s[j]=='{':d+=1
        elif s[j]=='}':
            d-=1
            if d==0: return s[i:j]
for d in list(IF):
    p=f'{B}/{d}/{d}.wit'
    if not os.path.exists(p): bad.append(f'missing {p}'); continue
    s=open(p,encoding='utf-8').read()
    if f'package slicer:{d}@1.0.0;' not in s or len(re.findall(r'(?m)^package ',s))!=1: bad.append(f'{d}: package header')
    if len(re.findall(r'(?m)^\s*run: func',s))!=1: bad.append(f'{d}: run func count')
    if len(re.findall(r'(?m)^world ',s))!=1: bad.append(f'{d}: world count')
    exp=block(s,IF[d])
    if exp is None: bad.append(f'{d}: no exported interface {IF[d]}')
    elif 'resource ' in exp: bad.append(f'{d}: resource in EXPORTED interface')
for d,iface in P.items():
    s=open(f'{B}/{d}/{d}.wit',encoding='utf-8').read()
    if f'interface {iface}-types' not in s: bad.append(f'{d}: no {iface}-types interface')
    if not re.search(r'(?m)^\s*import '+re.escape(iface)+r'-types;',s): bad.append(f'{d}: world does not import {iface}-types')
pt=f'{B}/prepass-types.wit'
if not os.path.exists(pt): bad.append('missing flat deps/prepass-types.wit')
else:
    s=open(pt,encoding='utf-8').read()
    if 'package slicer:prepass-types;' not in s: bad.append('prepass-types must be the unversioned package slicer:prepass-types;')
    for t in ['mesh-object-view','paint-value-view','paint-stroke-view','paint-layer-view']:
        if t not in s: bad.append(f'prepass-types missing {t}')
for old in ['world-layer','world-prepass']:
    if os.path.exists(f'{B}/{old}'): bad.append(f'{old}/ survives')
print('PASS' if not bad else 'FAIL '+'; '.join(bad))"`

- **AC-2. Given** 163 left 13 `STAGES` rows with `wit_package == ""` and `wit_dir` in `{"world-layer","world-prepass"}`, **when** this packet lands, **then** in `crates/slicer-schema/src/lib.rs` every row except `PrePass::PaintSegmentation` carries non-empty `wit_dir`/`wit_package`/`wit_interface`/`wit_world` and `wit_export == "run"`; the `PrePass::PaintSegmentation` row carries `wit_dir == ""`, `wit_package == ""`, `wit_interface == ""`, `wit_world == ""`, `wit_export == ""` with a source comment naming it host-built-in (packet 97; the executing code is `crates/slicer-runtime/src/prepass.rs`); `qualified_export_for_stage_id("Layer::Perimeters") == Some("slicer:layer-perimeters/perimeters@1.0.0#run".to_string())`; `qualified_export_for_stage_id("PrePass::PaintSegmentation") == None`; no `"world-layer"`/`"world-prepass"` string literal survives in `STAGES`; and the unfiltered `cargo test -p slicer-schema` passes (163's totality guard `stage_and_world_lookups_are_consistent` is relaxed here from "wit_dir non-empty for all 16 rows" to "non-empty for all rows except the documented host-built-in row"). | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && cargo test -p slicer-schema 2>&1 | tee target/test-output.log | rg '^test result' && python3 -c "import re; s=open('crates/slicer-schema/src/lib.rs',encoding='utf-8').read(); n=len(re.findall(r'\"world-(layer|prepass)\"',s)); print('PASS' if n==0 else f'FAIL residual_tier_dirs={n}')"`

- **AC-3. Given** post-163 `crates/slicer-wasm-host/src/host.rs` declares five `bindgen!` mods (`layer`, `prepass`, `postpass_gcode`, `postpass_text`, `finalization_layer`), **when** `layer` and `prepass` are replaced by 12 per-stage mods, **then** `host.rs` declares exactly fifteen `bindgen!` invocations; their `world:` keys are exactly the 15 per-stage worlds (the 12 from AC-1 plus 163's three) with no key containing `world-layer` or `world-prepass`; every `path:` remains `"../slicer-schema/wit"`; each of the 12 new mods repeats the five `with:` alias keys (`slicer:types/geometry`, `slicer:config/config-types`, `slicer:common/host-services`, `slicer:common/module-errors`, `slicer:ir-handles/ir-handles`); and `slicer:prepass-types/prepass-types` is `with:`-aliased in every prepass mod except the one canonical definer, so one Rust type set exists for the shared views (ADR-0002). | `cd F:/slicerProject/pinch_n_print_cli && python3 -c "
import re
s=open('crates/slicer-wasm-host/src/host.rs',encoding='utf-8').read()
n=len(re.findall(r'bindgen!\(\{',s))
worlds=set(re.findall(r'world:\s*\"([^\"]+)\"',s))
twelve=['layer-slice-postprocess/slice-postprocess-module','layer-perimeters/perimeters-module','layer-perimeters-postprocess/perimeters-postprocess-module','layer-infill/infill-module','layer-infill-postprocess/infill-postprocess-module','layer-support/support-module','layer-support-postprocess/support-postprocess-module','layer-path-optimization/path-optimization-module','prepass-mesh-analysis/mesh-analysis-module','prepass-layer-planning/layer-planning-module','prepass-seam-planning/seam-planning-module','prepass-support-geometry/support-geometry-module']
want={'slicer:'+w for w in twelve}|{'slicer:postpass-gcode-postprocess/gcode-postprocess-module','slicer:postpass-text-postprocess/text-postprocess-module','slicer:finalization-layer-finalization/layer-finalization-module'}
bad=[]
if n!=15: bad.append(f'bindgen={n} (expect 15)')
if worlds!=want: bad.append(f'world diff extra={sorted(worlds-want)} missing={sorted(want-worlds)}')
if set(re.findall(r'path:\s*\"([^\"]+)\"',s))!={'../slicer-schema/wit'}: bad.append('non-canonical path:')
if len(re.findall(r'\"slicer:prepass-types/prepass-types\"',s))<1: bad.append('prepass-types never with:-aliased')
ALIAS=['slicer:types/geometry','slicer:config/config-types','slicer:common/host-services','slicer:common/module-errors','slicer:ir-handles/ir-handles']
mods=['layer_slice_postprocess','layer_perimeters','layer_perimeters_postprocess','layer_infill','layer_infill_postprocess','layer_support','layer_support_postprocess','layer_path_optimization','prepass_mesh_analysis','prepass_layer_planning','prepass_seam_planning','prepass_support_geometry']
for m in mods:
    b=re.search(r'(?m)^pub mod '+m+r' \{(.*?)^\}',s,re.S)
    if not b: bad.append(f'no pub mod {m}'); continue
    missing=[k for k in ALIAS if f'\"{k}\"' not in b.group(1)]
    if missing: bad.append(f'{m}: missing with: aliases {missing}')
print('PASS' if not bad else 'FAIL '+'; '.join(bad))"`

- **AC-4. Given** `dispatch.rs::dispatch_layer_call` instantiates `host::LayerModule` once and routes via `call_layer_export`'s stage match, and `dispatch_prepass_call` does the same with `host::PrepassModule`, **when** both are rewired to per-stage typed instantiation (linker setup, `instantiate`, and the single `run` call selected by `stage_id` — 163's dispatch pattern), **then** `crates/slicer-wasm-host/src/dispatch.rs` contains zero occurrences of `host::LayerModule` and `host::PrepassModule`, every `DispatchPhase::TypedInstantiation` arm's `reason` includes `slicer_schema::qualified_export_for_stage_id(stage_id)`, and the full (unfiltered) executor suite plus the dispatch contract tests pass. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && python3 -c "import re; s=open('crates/slicer-wasm-host/src/dispatch.rs',encoding='utf-8').read(); n=len(re.findall(r'host::(LayerModule|PrepassModule)',s)); print('PASS' if n==0 else f'FAIL tier_bindings={n}')" && cargo xtask build-guests --check && (cargo test -p slicer-runtime --test executor 2>&1 | tee target/test-output.log | rg '^test result') && ((cargo test -p slicer-runtime --test contract -- dispatch 2>&1 | tee -a target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 dispatch contract tests ran')`

- **AC-5. Given** `crates/slicer-macros/src/lib.rs` post-163 carries `build_layer_world_glue(self_ty, detected_stage)` and `build_prepass_world_glue(self_ty, detected_stage)` (each one `impl Guest` with benign-`Ok` padding arms for sibling stages) and `StageGlueKind` variants `Prepass`/`Layer` with surviving trait-name fallbacks, **when** the glue is split per stage by copying 163's `build_postpass_gcode_glue` template 12×, **then** `lib.rs` contains zero occurrences of `build_layer_world_glue` and `build_prepass_world_glue`; `resolve_stage_glue` has **no** trait-name fallback arms left (a stageless impl gets no glue — grep for `Some("LayerModule")`/`Some("PrepassModule")` arms returning `Some`); and the emitted glue covers all 12 new `impl exports::slicer::<pkg_snake>::<iface_snake>::Guest` blocks, each with exactly one `fn run`. | `cd F:/slicerProject/pinch_n_print_cli && python3 -c "
import re
s=open('crates/slicer-macros/src/lib.rs',encoding='utf-8').read()
dead=[t for t in ['build_layer_world_glue','build_prepass_world_glue'] if t in s]
ifaces=set(re.findall(r'impl exports::slicer::([a-z0-9_]+)::([a-z0-9_]+)::Guest',s))
want={('layer_slice_postprocess','slice_postprocess'),('layer_perimeters','perimeters'),('layer_perimeters_postprocess','perimeters_postprocess'),('layer_infill','infill'),('layer_infill_postprocess','infill_postprocess'),('layer_support','support'),('layer_support_postprocess','support_postprocess'),('layer_path_optimization','path_optimization'),('prepass_mesh_analysis','mesh_analysis'),('prepass_layer_planning','layer_planning'),('prepass_seam_planning','seam_planning'),('prepass_support_geometry','support_geometry')}
fb=re.search(r'Some\(\"(Layer|Prepass)Module\"\)\s*=>\s*Some',s)
print('PASS' if not dead and want<=ifaces and not fb else f'FAIL dead={dead} missing={sorted(want-ifaces)} trait_fallback_survives={bool(fb)}')"`

- **AC-6. Given** all 15 remaining core-module guests decode (post-163) to bare freestanding stage funcs on the tier worlds, **when** `cargo xtask build-guests` regenerates every artifact, **then** `wasm-tools component wit` decodes `arachne-perimeters.wasm` exporting `slicer:layer-perimeters/perimeters@1.0.0` with no bare `run-perimeters`; `gyroid-infill.wasm` exports `slicer:layer-infill/infill@1.0.0`; `support-planner.wasm` exports `slicer:prepass-support-geometry/support-geometry@1.0.0`; and `seam-planner-default.wasm` exports `slicer:prepass-seam-planning/seam-planning@1.0.0`. | `cd F:/slicerProject/pinch_n_print_cli && cargo xtask build-guests >/dev/null 2>&1; python3 -c "
import subprocess
def wit(m): return subprocess.run(['wasm-tools','component','wit',f'modules/core-modules/{m}/{m}.wasm'],capture_output=True,text=True).stdout
bad=[]
for m,pkg,old in [('arachne-perimeters','slicer:layer-perimeters/perimeters@1.0.0','run-perimeters'),('gyroid-infill','slicer:layer-infill/infill@1.0.0','run-infill'),('support-planner','slicer:prepass-support-geometry/support-geometry@1.0.0','run-support-geometry'),('seam-planner-default','slicer:prepass-seam-planning/seam-planning@1.0.0','run-seam-planning')]:
    g=wit(m)
    if pkg not in g: bad.append(f'{m}: no {pkg}')
    if f'{old}:' in g: bad.append(f'{m}: bare {old} survives')
print('PASS' if not bad else 'FAIL '+'; '.join(bad))"`

- **AC-7. Given** post-163 the manifest key `wit-world` is required by `crates/slicer-scheduler/src/manifest.rs` (`required_string(&root, manifest_path, "module.wit-world")` then `validate_wit_world`) and `slicer_schema::SUPPORTED_WIT_WORLDS` backs the allowlist, **when** the key retires, **then** zero `modules/core-modules/*/*.toml` files contain a `wit-world` line; `validate_wit_world` and the manifest struct's `wit_world` field and `wit_world()` accessor are deleted from `crates/slicer-scheduler/src/manifest.rs`; `SUPPORTED_WIT_WORLDS` is deleted from `crates/slicer-schema/src/lib.rs`; `crates/pnp-cli/src/module_new.rs` no longer scaffolds a `wit-world` line (its scaffold comment names the stage's qualified export via `slicer_schema::qualified_export_for_stage_id` instead, and `wit_world_for_stage` plus its `wit_world_mapping` test are deleted); and the scheduler manifest-ingestion suite passes with its `wit_world_*` tests removed or rewritten per AC-N3. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && python3 -c "
import glob,re
n=sum(1 for f in glob.glob('modules/core-modules/*/*.toml') if re.search(r'(?m)^\s*wit-world\s*=',open(f,encoding='utf-8').read()))
m=open('crates/slicer-scheduler/src/manifest.rs',encoding='utf-8').read()
s=open('crates/slicer-schema/src/lib.rs',encoding='utf-8').read()
p=open('crates/pnp-cli/src/module_new.rs',encoding='utf-8').read()
bad=[]
if n!=0: bad.append(f'manifests still declaring wit-world: {n}')
if 'validate_wit_world' in m: bad.append('validate_wit_world survives')
if 'SUPPORTED_WIT_WORLDS' in s: bad.append('SUPPORTED_WIT_WORLDS survives')
if 'wit-world' in p: bad.append('module_new still scaffolds wit-world')
print('PASS' if not bad else 'FAIL '+'; '.join(bad))" && ((cargo test -p slicer-scheduler --test integration -- manifest_ingestion 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 manifest_ingestion tests ran')`

- **AC-8. Given** the deviation row packet 163 files for the two-mechanism intermediate names `164_per-stage-wit-packages-bulk` as owner (its ID must be **re-derived at implementation time** — run `rg -n '164_per-stage-wit-packages-bulk' docs/DEVIATION_LOG.md`; do not assume `DEV-086`: two IDs landed out from under 163 while it was being authored and the same can happen again), **when** this packet lands, **then** that row's status column reads `Resolved` with a date and a note that layer+prepass migrated and `wit-world`/`SUPPORTED_WIT_WORLDS`/`validate_wit_world` retired. | `cd F:/slicerProject/pinch_n_print_cli && python3 -c "
rows=[l for l in open('docs/DEVIATION_LOG.md',encoding='utf-8') if '164_per-stage-wit-packages-bulk' in l and l.startswith('| DEV-')]
ok=bool(rows) and all('Resolved' in r for r in rows)
print('PASS' if ok else ('FAIL: no owner row found — was it filed by 163?' if not rows else 'FAIL: owner row not Resolved'))"`

- **AC-9. Given** `wit_drift_detection_tdd.rs::every_stage_package_major_is_at_least_one` (from 163) already walks every `.wit` under `crates/slicer-schema/wit/deps/*/` (so it covers the 12 new packages with no new assertions) and `wit_single_source_tdd.rs::canonical_wit_resolves` pins the delivered world list, **when** both suites are updated to the 15-package surface, **then** both pass with non-zero test counts and neither file mentions `world-layer` or `world-prepass`. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && ((cargo test -p slicer-runtime --test contract -- wit_single_source 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 wit_single_source tests ran') && ((cargo test -p slicer-runtime --test contract -- wit_drift_detection 2>&1 | tee -a target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 wit_drift_detection tests ran') && python3 -c "
F=['crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs','crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs']
bad=[p for p in F if 'world-layer' in open(p,encoding='utf-8').read() or 'world-prepass' in open(p,encoding='utf-8').read()]
print('PASS' if not bad else f'FAIL residual_tier_world_pins={bad}')"`

## Negative Test Cases

- **AC-N1. Given** 163's `stage_miss_is_fatal_at_instantiation` (`crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs`) proves fatal-on-miss for a postpass stage, **when** a layer case is added to the same test (not a second test — 163's exports ledger: "add cases, not a second test") dispatching `Layer::Perimeters` at a guest exporting only `slicer:layer-infill` (e.g. `layer-infill-guest`), **then** the call returns `Err(DispatchError)` with `phase == DispatchPhase::TypedInstantiation` and a `reason` containing the engine's expected-only wording — the literal `` no exported instance named `slicer:layer-perimeters/perimeters@1.0.0` `` — never `Ok`, and never a "found @x.y.z" fragment wasmtime does not produce. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && ((cargo test -p slicer-runtime --test contract -- stage_miss_is_fatal_at_instantiation 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: test absent, unregistered, or 0 tests ran')`

- **AC-N2. Given** ADR-0045's headline isolation claim, now demonstrable on the tier that motivated it, **when** everything is freshly rebuilt and `crates/slicer-schema/wit/deps/layer-perimeters/layer-perimeters.wit` alone is touched, **then** `cargo xtask build-guests --check` prints `STALE:` for the two perimeters core guests (`arachne-perimeters`, `classic-perimeters`) and does **not** print `STALE:` for `gyroid-infill` or `machine-gcode-emit` (test guests **may** go stale — 163's `stage_wit_mtime` charges stage-less guests all package dirs, conservative by contract; never invert that to make this pass); and after the ensuing rebuild, `gyroid-infill.wasm` and `machine-gcode-emit.wasm` are sha256-byte-identical to their pre-touch selves. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && cargo xtask build-guests >/dev/null 2>&1 && python3 -c "
import hashlib,json
M=['gyroid-infill','machine-gcode-emit']
print(json.dumps({m:hashlib.sha256(open(f'modules/core-modules/{m}/{m}.wasm','rb').read()).hexdigest() for m in M}))" > target/pre.json && sleep 2 && touch crates/slicer-schema/wit/deps/layer-perimeters/layer-perimeters.wit && cargo xtask build-guests --check > target/stale.txt 2>&1; cargo xtask build-guests >/dev/null 2>&1; python3 -c "
import hashlib,json
pre=json.load(open('target/pre.json'))
s=open('target/stale.txt',encoding='utf-8').read()
stale={l.split('STALE:')[1].strip() for l in s.splitlines() if l.startswith('STALE:')}
must={'arachne-perimeters','classic-perimeters'}
missing=sorted(m for m in must if not any(m in x for x in stale))
leaked=sorted(m for m in pre if any(m in x for x in stale))
changed=sorted(m for m,h in pre.items() if hashlib.sha256(open(f'modules/core-modules/{m}/{m}.wasm','rb').read()).hexdigest()!=h)
print('PASS' if not missing and not leaked and not changed else f'FAIL not_stale={missing} leaked={leaked} rebuilt={changed}')"`

- **AC-N3. Given** post-163 a manifest **without** `wit-world` is rejected by `required_string`, **when** the key retires, **then** a manifest omitting `wit-world` loads successfully, and a legacy manifest still carrying `wit-world = "slicer:world-layer"` also loads successfully with the key ignored (the parser's existing policy for informational keys like `display-name` — see `docs/03_wit_and_manifest.md` §Module Manifest Schema) — asserted by a manifest-ingestion test named `wit_world_key_is_ignored` replacing the deleted `wit_world_mismatch_rejects_invalid_package_name` and `versioned_wit_world_is_rejected_with_actionable_diagnostic` tests (both in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`; do not trust docs/07's TASK-146 row, which carries stale pre-rename names for them). | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && ((cargo test -p slicer-scheduler --test integration -- wit_world_key_is_ignored 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: test absent, unregistered, or 0 tests ran')`

- **AC-N4. Given** the behavior-neutrality baseline (`perimeter_parity` → `12 passed; 0 failed; 11 ignored`; `legacy_zero_matches_golden` → `1 passed; 0 failed`; green at 163's close per its AC-N3), **when** this packet's migration is complete, **then** both still report `0 failed` — a regression here is caused by this packet and must not be "fixed" by editing goldens. `0 passed` means the filter matched nothing and is a FAIL. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && cargo xtask build-guests --check && ((cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 perimeter_parity tests ran') && ((cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 | tee -a target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 legacy_zero tests ran')`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `(cargo test -p slicer-runtime --test contract -- stage_miss_is_fatal_at_instantiation 2>&1 | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 tests ran'` — the `rg -v '0 passed'` guard is **mandatory on every name-filtered `cargo test` gate in this packet**: an absent or unregistered test filters to nothing, prints `ok. 0 passed`, and exits 0. Unfiltered whole-binary runs do not need it.

## Authoritative Docs

- `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` (accepted; long — ranged reads only): §"Decision", §"Why this works", §"Verified empirically, not just read", §"The naive shape inverts resource ownership", §"Consequences". Note: its "17 packages: 10 layer + 4 prepass" count is **wrong against the tree** — layer has 8 stage exports and prepass 4 (`PrePass::PaintSegmentation` is host-built-in), so the delivered end state is **15** packages. See `design.md` §"Grounded counts (falsifies the plan's arithmetic)". The mechanism sections remain governing.
- `.ralph/specs/163_per-stage-wit-packages-pilot/design.md` §"Exports handed to packet #3" — the machinery contract this packet consumes verbatim; do not re-derive its decisions. §"Naming — the `[DECIDE + FLAG]` resolved" is the naming rule.
- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` (long; ranged reads only): §"Grounding corrections", §"Packet Queue", both §"Exports ledger" subsections.
- `docs/adr/0006-export-for-stage-id-sole-lookup.md` — direct read; extend `STAGES`, never add a parallel table.
- `docs/adr/0002-wit-marshalling-type-unification.md` — direct read; governs the `with:` aliasing all 12 new `bindgen!` mods repeat and the `prepass-types` single-type-set decision.
- `docs/adr/0044-wit-world-version-is-not-an-identity-token.md` — delegated SUMMARY; the argument for retiring `wit-world` / `SUPPORTED_WIT_WORLDS` / `validate_wit_world`.
- `docs/03_wit_and_manifest.md` (long; ranged reads only) — the `world-layer.wit` / `world-prepass.wit` listing sections, §"Why `wit-world` carries no version", and §Module Manifest Schema.
- `CLAUDE.md` §"Guest WASM Staleness", §"Test Discipline", §"WIT/Type Changes Checklist", §"Ledger Facts Must Be Re-derived, Not Quoted" — direct read.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/03_wit_and_manifest.md` §"WIT File Organization" tree and the `world-layer.wit` / `world-prepass.wit` listing sections — replace with the 12 per-stage package listings plus `deps/prepass-types.wit`, and delete the stale `run-paint-segmentation` listing (the world file never declared it; state that `PrePass::PaintSegmentation` is host-built-in, packet 97). Verification grep: `rg -q 'package slicer:layer-perimeters@1.0.0' docs/03_wit_and_manifest.md && (rg -q 'package slicer:world-layer' docs/03_wit_and_manifest.md && exit 1 || exit 0)`
- `docs/03_wit_and_manifest.md` §"Why `wit-world` carries no version" and §Module Manifest Schema — rewrite for the retired key: the version now lives in the package name inside the binary, which is what makes it checkable (the ADR-0044 → ADR-0045 arc); drop `wit-world` from the manifest schema listing and note the key is tolerated-but-ignored for legacy manifests. Verification grep: `(rg -q '^wit-world' docs/03_wit_and_manifest.md && exit 1 || exit 0)`
- `CONTEXT.md` **Module tier** — "Each tier has exactly one WIT *world*" is false after this packet; rewrite: a tier is vocabulary (package-name prefix, SDK trait grouping), each stage has its own versioned package. **Stage contract** — the unit of contract becomes the stage's package; a module satisfies exactly one. **Stage interface** — drop only the closing "not yet implemented" clause (now delivered); the rest of that entry was already corrected before this packet and must not be reworded. Describe the delivered code, never intentions (domain-modeling discipline). Verification grep: `(rg -q 'Each tier has exactly one WIT' CONTEXT.md && exit 1 || exit 0) && (rg -qi 'not yet.implemented' CONTEXT.md && exit 1 || exit 0)`
- `crates/slicer-schema/wit/README.md` §"Layout" — replace the `world-layer`/`world-prepass` rows with the 12 per-stage rows plus `prepass-types.wit`. **Verify what 163 left it saying first** (163 already rewrote the postpass/finalization rows, the versioning sentence, and the host-consumer example — extend, don't re-write). Verification grep: `rg -q 'layer-perimeters/layer-perimeters.wit' crates/slicer-schema/wit/README.md && (rg -q 'world-layer/world-layer.wit' crates/slicer-schema/wit/README.md && exit 1 || exit 0)`
- `docs/07_implementation_status.md` — record TASK-146c under the TASK-146 slice. Verification grep: `rg -q 'TASK-146c' docs/07_implementation_status.md`
- `docs/DEVIATION_LOG.md` — close the two-mechanism-intermediate row per AC-8 (re-derive its ID with AC-8's command). This packet files **no** new deviation row. Verification: AC-8's command.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
