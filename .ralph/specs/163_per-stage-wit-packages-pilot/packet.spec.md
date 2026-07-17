---
status: draft
packet: 163_per-stage-wit-packages-pilot
task_ids:
  - TASK-146b
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 163_per-stage-wit-packages-pilot

## Goal

Prove the per-stage versioned-WIT-package mechanism survives contact with **real host imports and `with:`-mapped resources** — the one thing ADR-0045's spike did not test — by building it end to end (canonical `.wit` packages at `@1.0.0`, `slicer_schema::STAGES` package/interface/world columns, per-stage host `bindgen!`, per-stage `#[slicer_module]` glue, fatal-on-miss typed instantiation, per-stage guest staleness in `xtask`) on the three cheapest stages: `PostPass::GCodePostProcess`, `PostPass::TextPostProcess`, `PostPass::LayerFinalization`.

## Scope Boundaries

This packet replaces `slicer:world-postpass@1.0.0` and `slicer:world-finalization@1.0.0` with three per-stage packages — `slicer:postpass-gcode-postprocess@1.0.0`, `slicer:postpass-text-postprocess@1.0.0`, `slicer:finalization-layer-finalization@1.0.0` — each pairing an **imported** types-and-resources interface with an **exported** interface holding one `run` func, and rewires schema, host bindgen, dispatch, macro glue and `xtask` staleness onto the resulting stage→package→typed-instantiate path. It deliberately leaves the layer (8 stages) and prepass (4 exports) tiers on their monolithic tier worlds; migrating them, and retiring `wit-world` / `SUPPORTED_WIT_WORLDS` / `validate_wit_world`, belongs to `164_per-stage-wit-packages-bulk`. Between this packet and #3 two contract mechanisms are live in-tree; that is the accepted intermediate recorded in `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Dependency note", not an end state.

## Prerequisites and Blockers

- Depends on: `162_wit-lifecycle-export-removal` (TASK-146a) — **must be IMPLEMENTED, not merely generated**. Every symbol this packet cites is cited in its POST-162 form: `world-layer` is 8 exports; postpass/finalization worlds never declared lifecycle exports and are unchanged by 162; the SDK constructor is `from_config(config: &ConfigView) -> Result<Self, ModuleError>` (required, no default); `WORLD_LIFECYCLE_EXPORTS`, `lifecycle_exports_for_world`, `ExportKind::Lifecycle` and `__SLICER_LIFECYCLE_EXPORT_COUNT` no longer exist; `ExportKind` has one `Stage` variant; `SlicerModuleSchema.exports` is ≤1 entry. Starting this packet against a tree where 162 has not landed will produce spurious conflicts in `crates/slicer-macros/src/lib.rs` and `crates/slicer-schema/src/lib.rs`.
- Unblocks: `164_per-stage-wit-packages-bulk` (TASK-146c), which consumes the `StageSpec` columns, the `slicer_schema` lookups, the macro's `StageGlueKind` seam, and the `xtask` per-stage staleness rule listed in `design.md` §"Exports handed to packet #3".
- Activation blockers: none. Two `[FWD]` entries remain in `design.md` (test-guest staleness granularity; `__slicer_wit_exports()` qualification). Both are follow-ups owned by #3, not blockers.

## Acceptance Criteria

- **AC-1. Given** the canonical WIT tree holds `crates/slicer-schema/wit/deps/world-postpass/world-postpass.wit` (`package slicer:world-postpass@1.0.0`, 2 world-level exports) and `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit` (`package slicer:world-finalization@1.0.0`, 1 world-level export), **when** they are replaced by three per-stage packages, **then** `deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit`, `deps/postpass-text-postprocess/postpass-text-postprocess.wit` and `deps/finalization-layer-finalization/finalization-layer-finalization.wit` each exist; each declares exactly one `package slicer:<dir>@1.0.0;` header, exactly one `run: func`, and exactly one `world <dir>-module` that declares exactly one `export <iface>;`; the two packages carrying host-owned resources declare a second, **imported** `<iface>-types` interface holding every `resource` (zero `resource` declarations appear in any exported interface — a resource in an exported interface would invert ownership onto the guest); and neither replaced world file nor its directory survives. | `cd F:/slicerProject/pinch_n_print_cli && python3 -c "
import re,os
B='crates/slicer-schema/wit/deps'
S=[('postpass-gcode-postprocess','gcode-postprocess',True),('postpass-text-postprocess','text-postprocess',False),('finalization-layer-finalization','layer-finalization',True)]
bad=[]
def block(s,name):
    m=re.search(r'(?m)^interface '+re.escape(name)+r'\s*\{',s)
    if not m: return None
    i=m.end()-1; d=0
    for j in range(i,len(s)):
        if s[j]=='{': d+=1
        elif s[j]=='}':
            d-=1
            if d==0: return s[i:j]
    return None
for d,iface,has_res in S:
    p=f'{B}/{d}/{d}.wit'
    if not os.path.exists(p): bad.append(f'missing {p}'); continue
    s=open(p,encoding='utf-8').read()
    if len(re.findall(r'(?m)^package ',s))!=1 or f'package slicer:{d}@1.0.0;' not in s: bad.append(f'{d}: package header')
    if len(re.findall(r'(?m)^\s*run: func',s))!=1: bad.append(f'{d}: run func')
    if len(re.findall(r'(?m)^world ',s))!=1 or f'world {d}-module ' not in s: bad.append(f'{d}: world')
    if len(re.findall(r'(?m)^\s*export '+re.escape(iface)+r';',s))!=1: bad.append(f'{d}: world export')
    exp=block(s,iface)
    if exp is None: bad.append(f'{d}: no exported interface {iface}'); continue
    if 'resource ' in exp: bad.append(f'{d}: resource in EXPORTED interface {iface} (ownership inverted onto guest)')
    if has_res:
        t=block(s,iface+'-types')
        if t is None or 'resource ' not in t: bad.append(f'{d}: missing imported {iface}-types interface holding resources')
        if len(re.findall(r'(?m)^\s*import '+re.escape(iface)+r'-types;',s))!=1: bad.append(f'{d}: world does not import {iface}-types')
for old in ['world-postpass','world-finalization']:
    if os.path.exists(f'{B}/{old}'): bad.append(f'{old}/ survives')
print('PASS' if not bad else 'FAIL '+'; '.join(bad))"`

- **AC-1b. Given** ADR-0045 §"Verified empirically, not just read" measures that a stage package shipped at `0.x` gets **no** compatibility track at all (`guest @0.1.0 / host wants @0.2.0` → FAIL; `alternate_lookup_key` yields a major-track key only for major ≥ 1), so `@1.0.0` is mechanically load-bearing rather than housekeeping, **when** the new guard `every_stage_package_major_is_at_least_one` is added to `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` (mirroring the existing `canonical_world_files_exist_on_disk` on-disk walk), **then** it parses the `package slicer:<name>@<major>.<minor>.<patch>;` header of every `.wit` under `crates/slicer-schema/wit/deps/*/`, asserts `major >= 1` for each, and fails with the offending `file:package` list — so a future `0.x` "tidy-up" is caught rather than silently disabling every compatibility claim. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && ((cargo test -p slicer-runtime --test contract -- every_stage_package_major_is_at_least_one 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: test absent, unregistered, or 0 tests ran')`

- **AC-2. Given** `slicer_schema::StageSpec` carries `method`, `stage_id`, `wit_export`, `world_id`, `trait_name`, **when** the columns `wit_dir`, `wit_package`, `wit_interface`, `wit_world` are added, **then** `wit_dir` is non-empty for all 16 rows in `STAGES`; the three pilot rows carry exactly `("postpass-gcode-postprocess", "slicer:postpass-gcode-postprocess@1.0.0", "gcode-postprocess", "gcode-postprocess-module")`, `("postpass-text-postprocess", "slicer:postpass-text-postprocess@1.0.0", "text-postprocess", "text-postprocess-module")`, `("finalization-layer-finalization", "slicer:finalization-layer-finalization@1.0.0", "layer-finalization", "layer-finalization-module")` with `wit_export == "run"`; every other row carries `wit_package == ""`, `wit_interface == ""`, `wit_world == ""` and `wit_dir` in `{"world-layer","world-prepass"}`; and `cargo test -p slicer-schema` passes. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && cargo test -p slicer-schema 2>&1 | tee target/test-output.log | rg '^test result'`

- **AC-3. Given** `slicer_schema::export_for_stage_id` is the sole stage→export lookup (ADR-0006), **when** the package columns land, **then** the new lookups `package_for_stage_id`, `interface_for_stage_id`, `wit_world_for_stage_id`, `wit_dir_for_stage_id` and `qualified_export_for_stage_id` all read `STAGES` (no second table exists), `qualified_export_for_stage_id("PostPass::GCodePostProcess") == Some("slicer:postpass-gcode-postprocess/gcode-postprocess@1.0.0#run".to_string())`, `qualified_export_for_stage_id("Layer::Perimeters") == None` (unmigrated), `qualified_export_for_stage_id("NotAStage") == None`, and `crates/slicer-schema/src/lib.rs` contains exactly one `const STAGES`. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && ((cargo test -p slicer-schema --test export_for_stage_id_tdd 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 tests ran') && python3 -c "import re; s=open('crates/slicer-schema/src/lib.rs',encoding='utf-8').read(); n=len(re.findall(r'(?m)^pub const STAGES',s)); print('PASS' if n==1 else f'FAIL STAGES_tables={n}')"`

- **AC-4. Given** `crates/slicer-wasm-host/src/host.rs` declares four `bindgen!` mods `layer`, `prepass`, `finalization`, `postpass` (the latter three aliasing the layer world's generated types via `with:` per ADR-0002 / packet 75), **when** the two tier mods `finalization` and `postpass` are replaced by three per-stage mods, **then** `host.rs` declares exactly five `bindgen!` invocations, whose `world:` keys are exactly `slicer:world-layer/layer-module`, `slicer:world-prepass/prepass-module`, `slicer:postpass-gcode-postprocess/gcode-postprocess-module`, `slicer:postpass-text-postprocess/text-postprocess-module`, `slicer:finalization-layer-finalization/layer-finalization-module`; every `path:` remains `"../slicer-schema/wit"`; no `world:` key contains `world-postpass` or `world-finalization`; and each of the three new mods repeats the same five `with:` alias keys the retired `postpass` mod carried (`slicer:types/geometry`, `slicer:config/config-types`, `slicer:common/host-services`, `slicer:common/module-errors`, `slicer:ir-handles/ir-handles`). | `cd F:/slicerProject/pinch_n_print_cli && python3 -c "
import re
s=open('crates/slicer-wasm-host/src/host.rs',encoding='utf-8').read()
n=len(re.findall(r'bindgen!\(\{',s))
worlds=set(re.findall(r'world:\s*\"([^\"]+)\"',s))
want={'slicer:world-layer/layer-module','slicer:world-prepass/prepass-module','slicer:postpass-gcode-postprocess/gcode-postprocess-module','slicer:postpass-text-postprocess/text-postprocess-module','slicer:finalization-layer-finalization/layer-finalization-module'}
paths=set(re.findall(r'path:\s*\"([^\"]+)\"',s))
bad=[]
if n!=5: bad.append(f'bindgen={n} (expect 5)')
if worlds!=want: bad.append(f'worlds={sorted(worlds)}')
if paths!={'../slicer-schema/wit'}: bad.append(f'paths={sorted(paths)}')
# each of the 3 new stage mods must repeat the 5 dep alias keys the retired postpass mod carried
ALIAS=['slicer:types/geometry','slicer:config/config-types','slicer:common/host-services','slicer:common/module-errors','slicer:ir-handles/ir-handles']
for m in ['postpass_gcode','postpass_text','finalization_layer']:
    b=re.search(r'(?m)^pub mod '+m+r' \{(.*?)^\}',s,re.S)
    if not b: bad.append(f'no pub mod {m}'); continue
    missing=[k for k in ALIAS if f'\"{k}\"' not in b.group(1)]
    if missing: bad.append(f'{m}: missing with: aliases {missing}')
print('PASS' if not bad else 'FAIL '+'; '.join(bad))"`

- **AC-5. Given** `dispatch.rs::dispatch_postpass_gcode_call`, `dispatch_postpass_text_call` and `dispatch_finalization_call` each call `host::PostpassModule::instantiate` / `host::FinalizationModule::instantiate` on a monolithic tier world, **when** each is repointed at its own stage world's generated bindings, **then** `crates/slicer-wasm-host/src/dispatch.rs` contains zero occurrences of `host::PostpassModule` and `host::FinalizationModule`, and the postpass + finalization executor suites pass unchanged. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && python3 -c "import re; s=open('crates/slicer-wasm-host/src/dispatch.rs',encoding='utf-8').read(); n=len(re.findall(r'host::(PostpassModule|FinalizationModule)',s)); print('PASS' if n==0 else f'FAIL old_world_bindings={n}')" && ((cargo test -p slicer-runtime --test executor -- postpass 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 postpass executor tests ran') && ((cargo test -p slicer-runtime --test executor -- finalization 2>&1 | tee -a target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 finalization executor tests ran')`

- **AC-6. Given** `crates/slicer-macros/src/lib.rs::build_postpass_world_glue(self_ty, detected_stage)` emits one `impl Guest for __SlicerPostpassComponent` carrying **both** `run_gcode_postprocess` and `run_text_postprocess`, one of which is padding (the source comment reads "The other arm returns a benign `Ok` so the component remains WIT-conformant"), **when** the glue is split per stage, **then** `lib.rs` contains zero occurrences of `build_postpass_world_glue`, `__SlicerPostpassComponent`, `resolve_world_glue` and `WorldGlueKind`; it contains `build_postpass_gcode_glue`, `build_postpass_text_glue`, `resolve_stage_glue` and `enum StageGlueKind` with variants `PostpassGcode`, `PostpassText`, `Finalization`, `Prepass`, `Layer`; and each of the three pilot glue builders emits exactly one `impl exports::slicer::<pkg>::<iface>::Guest` block with exactly one `fn run`. | `cd F:/slicerProject/pinch_n_print_cli && python3 -c "
import re
s=open('crates/slicer-macros/src/lib.rs',encoding='utf-8').read()
dead=[t for t in ['build_postpass_world_glue','__SlicerPostpassComponent','resolve_world_glue','WorldGlueKind'] if t in s]
need=[t for t in ['build_postpass_gcode_glue','build_postpass_text_glue','resolve_stage_glue','enum StageGlueKind'] if t not in s]
ifaces=re.findall(r'impl exports::slicer::([a-z0-9_]+)::([a-z0-9_]+)::Guest',s)
want={('postpass_gcode_postprocess','gcode_postprocess'),('postpass_text_postprocess','text_postprocess'),('finalization_layer_finalization','layer_finalization')}
print('PASS' if not dead and not need and want<=set(ifaces) else f'FAIL dead={dead} missing={need} ifaces={sorted(set(ifaces))}')"`

- **AC-7. Given** `#[slicer_module]` on `impl PostpassModule for MachineGcodeEmit` today reports `__slicer_stage_export_name() == "run-gcode-postprocess"`, **when** the pilot lands, **then** `MachineGcodeEmit::__slicer_stage_export_name() == "run"`, `MachineGcodeEmit::__slicer_stage_name() == "PostPass::GCodePostProcess"`, `MachineGcodeEmit::__slicer_world_id() == slicer_schema::WORLD_POSTPASS` (the tier world id survives as manifest vocabulary until packet #3), and `__slicer_module_schema().exports == [ExportBinding { name: "run", kind: ExportKind::Stage }]`. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && ((cargo test -p machine-gcode-emit --test slicer_module_binding_tdd 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 tests ran')`

- **AC-8. Given** all 20 core-module guests currently decode to a `world root` exporting bare freestanding funcs (no `@` and therefore, per ADR-0045 §"Why this works", no wasmtime `alternate_lookup_key`), **when** `cargo xtask build-guests` regenerates every artifact, **then** `wasm-tools component wit modules/core-modules/machine-gcode-emit/machine-gcode-emit.wasm` decodes an export of `slicer:postpass-gcode-postprocess/gcode-postprocess@1.0.0` and no `run-gcode-postprocess` freestanding export; each of `skirt-brim`, `part-cooling`, `wipe-tower`, `overhang-classifier-default` decodes an export of `slicer:finalization-layer-finalization/layer-finalization@1.0.0`; and `arachne-perimeters.wasm` decodes neither. | `cd F:/slicerProject/pinch_n_print_cli && cargo xtask build-guests >/dev/null 2>&1; python3 -c "
import subprocess
def wit(m): return subprocess.run(['wasm-tools','component','wit',f'modules/core-modules/{m}/{m}.wasm'],capture_output=True,text=True).stdout
bad=[]
g=wit('machine-gcode-emit')
if 'slicer:postpass-gcode-postprocess/gcode-postprocess@1.0.0' not in g: bad.append('machine-gcode-emit: no gcode pkg export')
if 'run-gcode-postprocess' in g: bad.append('machine-gcode-emit: bare func survives')
for m in ['skirt-brim','part-cooling','wipe-tower','overhang-classifier-default']:
    if 'slicer:finalization-layer-finalization/layer-finalization@1.0.0' not in wit(m): bad.append(f'{m}: no finalization pkg export')
a=wit('arachne-perimeters')
if 'postpass-gcode-postprocess' in a or 'finalization-layer-finalization' in a: bad.append('arachne-perimeters: leaked a pilot package')
print('PASS' if not bad else 'FAIL '+'; '.join(bad))"`

- **AC-9. Given** `xtask/src/build_guests.rs::compute_shared_mtime` folds **every** `.wit` under `crates/slicer-schema/wit/` into one mtime applied to every guest — so today a change to one stage's WIT marks all 32 guests `STALE` regardless of packaging — **when** the WIT input is split into shared files (`wit/root.wit` plus the flat `wit/deps/*.wit`) and one per-guest package directory resolved through `slicer_schema::wit_dir_for_stage_id(spec.stage_id)`, **then** `xtask/Cargo.toml` declares a `slicer-schema` dependency, `GuestSpec` carries `stage_id: Option<String>` populated for `GuestTree::Core` from the sibling module manifest's `[stage] id`, `compute_shared_mtime`'s walk no longer descends into `wit/deps/*/` subdirectories, and `cargo test -p xtask` passes with a non-zero test count for the filter `stage_wit`. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && ((cargo test -p xtask -- stage_wit 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 stage_wit tests ran') && python3 -c "import re; s=open('xtask/Cargo.toml',encoding='utf-8').read(); b=open('xtask/src/build_guests.rs',encoding='utf-8').read(); ok='slicer-schema' in s and 'wit_dir_for_stage_id' in b and 'stage_id' in b; print('PASS' if ok else 'FAIL: xtask not wired to slicer_schema stage dirs')"`

- **AC-10. Given** the pilot moves postpass + finalization off their tier worlds while `SUPPORTED_WIT_WORLDS`, the manifest `wit-world` key and `validate_wit_world` stay live for packet #3, **when** the packet lands, **then** all 20 `modules/core-modules/*/*.toml` still declare `wit-world`, `machine-gcode-emit.toml` still declares `wit-world = "slicer:world-postpass"`, `crates/slicer-schema/src/lib.rs` still exports `SUPPORTED_WIT_WORLDS`, and the scheduler manifest-ingestion suite passes unchanged — proving the pilot did not silently widen into #3's scope. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && python3 -c "
import glob,re
n=sum(1 for f in glob.glob('modules/core-modules/*/*.toml') if re.search(r'(?m)^\s*wit-world\s*=',open(f,encoding='utf-8').read()))
g=open('modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml',encoding='utf-8').read()
s=open('crates/slicer-schema/src/lib.rs',encoding='utf-8').read()
bad=[]
if n!=20: bad.append(f'wit_world_manifests={n} (expect 20)')
if not re.search(r'(?m)^\s*wit-world\s*=\s*\"slicer:world-postpass\"',g): bad.append('machine-gcode-emit lost its wit-world')
if 'SUPPORTED_WIT_WORLDS' not in s: bad.append('SUPPORTED_WIT_WORLDS retired early (packet #3 owns it)')
print('PASS' if not bad else 'FAIL '+'; '.join(bad))" && ((cargo test -p slicer-scheduler --test integration -- manifest_ingestion 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 manifest_ingestion tests ran')`

- **AC-11. Given** `crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs::canonical_wit_resolves` selects the four worlds `slicer:world-{layer,prepass,postpass,finalization}/*-module` and `worlds_are_not_self_contained` iterates the four `world-*` dep directories, and `wit_drift_detection_tdd.rs::{macro_other_world_package_names_are_canonical, host_inline_wit_uses_canonical_world_package_names, canonical_world_files_exist_on_disk}` pin the same four names, **when** both suites are updated to the delivered five-world surface, **then** `cargo test -p slicer-runtime --test contract -- wit_single_source` and `cargo test -p slicer-runtime --test contract -- wit_drift_detection` both pass with a non-zero test count, and neither file mentions `world-postpass` or `world-finalization`. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && ((cargo test -p slicer-runtime --test contract -- wit_single_source 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 wit_single_source tests ran') && ((cargo test -p slicer-runtime --test contract -- wit_drift_detection 2>&1 | tee -a target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 wit_drift_detection tests ran') && python3 -c "
F=['crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs','crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs']
bad=[p for p in F if 'world-postpass' in open(p,encoding='utf-8').read() or 'world-finalization' in open(p,encoding='utf-8').read()]
print('PASS' if not bad else f'FAIL residual_tier_world_pins={bad}')"`

- **AC-12. Given** — *this is the pilot's reason to exist* — ADR-0045's spike proved the packaging mechanism with **no host imports at all**, while the real stage worlds import `slicer:common/host-services`, `slicer:config/config-types` and `slicer:ir-handles/ir-handles` and depend on `with:`-mapped resource **identity** holding across separate `bindgen!` calls (ADR-0002 remaps every non-layer world onto the layer world's Rust types; going from 4 `bindgen!` calls to 5 is the untested risk, and a break surfaces as a linking failure of the kind `CLAUDE.md` §"WIT/Type Changes Checklist" describes), **when** the three stage worlds are bound and dispatched, **then** all **four** suites that drive a **real typed instantiation plus a real host↔guest resource round-trip** through a pilot stage pass: `postpass_gcode_command_preservation` (guest receives an `own<gcode-output-builder>` handle and expands a 1-command input into 8 via `push_move`/`push_retract`/`push_tool_change` back into `HostExecutionContext` — output the host cannot have fabricated), `macro_postpass_text_roundtrip` (the `"[stamped] "` prefix exists only guest-side), `finalization_world_deep_copy` and `finalization_mutation_roundtrip` (guest reads `layer-collection-view` and writes `finalization-output-builder`; host IR observably mutates 1.0 → 0.5). Each asserts on a value only a live guest can produce, so a broken resource identity fails them. `postpass_gcode_boundary` is **deliberately excluded** — see `design.md` §"Why `postpass_gcode_boundary` is not in AC-12"; it never instantiates a component and would pass with the guest deleted. Do not re-add it. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && cargo xtask build-guests --check && for f in postpass_gcode_command_preservation macro_postpass_text_roundtrip; do ((cargo test -p slicer-runtime --test contract -- $f 2>&1 | tee -a target/test-output.log | rg '^test result') | rg -v '0 passed' || echo "FAIL: 0 tests ran for $f"); done; for f in finalization_world_deep_copy finalization_mutation_roundtrip; do ((cargo test -p slicer-runtime --test executor -- $f 2>&1 | tee -a target/test-output.log | rg '^test result') | rg -v '0 passed' || echo "FAIL: 0 tests ran for $f"); done`

## Negative Test Cases

- **AC-N1. Given** `crates/slicer-wasm-host/test-guests/sdk-postpass-text-guest` implements only `PostpassModule::run_text_postprocess`, and today its `.wasm` nonetheless satisfies `run-gcode-postprocess` through the macro's benign-`Ok` padding arm — so dispatching `PostPass::GCodePostProcess` at it returns `Ok(())` and produces no G-code, silently — **when** the new contract test `stage_miss_is_fatal_at_instantiation` (added to `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs`, already registered via `crates/slicer-runtime/tests/contract/main.rs`) dispatches `PostPass::GCodePostProcess` at `sdk-postpass-text-guest`, **then** the call returns `Err(DispatchError)` with `phase == DispatchPhase::TypedInstantiation`, `stage_id == "PostPass::GCodePostProcess"`, and a `reason` containing the engine's own measured wording — the literal `` no exported instance named `slicer:postpass-gcode-postprocess/gcode-postprocess@1.0.0` `` (ADR-0045 §"Verified empirically, not just read") — **not** `Ok`. The diagnostic is **expected-only**: wasmtime names what the host wanted and never what the guest shipped, so the test must NOT assert any "found @x.y.z" fragment. `0 passed` is a FAIL: a misnamed or unregistered test filters to nothing and exits 0. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && ((cargo test -p slicer-runtime --test contract -- stage_miss_is_fatal_at_instantiation 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: test absent, unregistered, or 0 tests ran')`

- **AC-N2. Given** ADR-0045's headline claim is that a stage's contract change stops invalidating unrelated modules ("infill change breaks perimeters modules: no — untouched, doesn't even rebuild"), which its spike measured as the unrelated guest's `sha256` being **byte-identical** across the bump; and given `xtask/src/build_guests.rs::compute_shared_mtime` currently maxes over `wit/**/*.wit` and applies the result to every guest, so today the claim is false in-tree no matter how the packages are cut, **when** `cargo xtask build-guests` has rebuilt everything and `crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit` alone is touched, **then** `cargo xtask build-guests --check` prints `STALE:` for the four finalization core guests (`skirt-brim`, `part-cooling`, `wipe-tower`, `overhang-classifier-default`) and does **not** print `STALE:` for `machine-gcode-emit` or `arachne-perimeters`; and after the ensuing `cargo xtask build-guests`, `machine-gcode-emit.wasm` and `arachne-perimeters.wasm` are **byte-identical** (`sha256`) to their pre-touch selves while the four finalization artifacts are free to differ. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && cargo xtask build-guests >/dev/null 2>&1 && python3 -c "
import hashlib,json
M=['machine-gcode-emit','arachne-perimeters']
print(json.dumps({m:hashlib.sha256(open(f'modules/core-modules/{m}/{m}.wasm','rb').read()).hexdigest() for m in M}))" > target/pre.json && sleep 2 && touch crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit && cargo xtask build-guests --check > target/stale.txt 2>&1; cargo xtask build-guests >/dev/null 2>&1; python3 -c "
import hashlib,json
pre=json.load(open('target/pre.json'))
s=open('target/stale.txt',encoding='utf-8').read()
stale={l.split('STALE:')[1].strip() for l in s.splitlines() if l.startswith('STALE:')}
must={'skirt-brim','part-cooling','wipe-tower','overhang-classifier-default'}
missing=sorted(m for m in must if not any(m in x for x in stale))
leaked=sorted(m for m in pre if any(m in x for x in stale))
changed=sorted(m for m,h in pre.items() if hashlib.sha256(open(f'modules/core-modules/{m}/{m}.wasm','rb').read()).hexdigest()!=h)
print('PASS' if not missing and not leaked and not changed else f'FAIL not_stale_but_should_be={missing} stale_but_should_not_be={leaked} rebuilt_but_should_not_be={changed}')"`

- **AC-N3. Given** the behavior-neutrality baseline is **green and committed at `ff21378e`** and therefore reproduces from HEAD — `cargo test -p slicer-runtime --test integration -- perimeter_parity` → `12 passed; 0 failed; 11 ignored` and `cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden` → `1 passed; 0 failed` — **when** this packet's WIT split, macro split, bindgen split and dispatch rewiring are complete, **then** both still report `0 failed`, proving the refactor changed no emitted G-code. A regression here is **caused by this packet**; it is a gate failure, not flakiness, and must not be "fixed" by editing goldens. Both are name filters (12 and 1 matches today), so `0 passed` means the filter matched nothing — a deleted test is a FAIL. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && cargo xtask build-guests --check && ((cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 perimeter_parity tests ran') && ((cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 | tee -a target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 legacy_zero tests ran')`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `(cargo test -p slicer-runtime --test contract -- stage_miss_is_fatal_at_instantiation 2>&1 | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 tests ran'` — the `rg -v '0 passed'` guard is **mandatory on every name-filtered `cargo test` gate in this packet**: an absent or unregistered test filters to nothing, prints `ok. 0 passed`, and exits 0. Unfiltered whole-binary runs (which print `0 filtered out`) do not need it.

## Authoritative Docs

- `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` (accepted; long — ranged reads only) - direct read: §"Decision", §"The unit is the package, not the interface", §"Versions reset to 1.0.0", §"Why this works", §"Verified empirically, not just read", §"The naive shape inverts resource ownership", §"Alternatives rejected". This is the governing decision; do not redesign it. The last two sections are **not optional background**: §"Verified empirically" supplies the measured semver matrix behind AC-1b's `major >= 1` rule and the measured miss-diagnostic wording AC-N1 pins, and §"The naive shape inverts resource ownership" supplies AC-1's entire imported-`-types` / exported-`run` rule. Skipping them means under-reading the governing design for two of this packet's ACs.
- `docs/adr/0044-wit-world-version-is-not-an-identity-token.md` (accepted) - delegated SUMMARY; supplies why `wit-world` / `SUPPORTED_WIT_WORLDS` are unfalsifiable ceremony that this packet deliberately leaves standing for #3.
- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` (>380 lines; ranged reads only) - direct read: §"Grounding corrections", §"Packet Queue", §"Exports ledger".
- `docs/adr/0006-export-for-stage-id-sole-lookup.md` - direct read; `export_for_stage_id` must be extended, never paralleled.
- `docs/adr/0002-wit-marshalling-type-unification.md` - direct read; governs the `with:` aliasing every non-layer `bindgen!` mod repeats.
- `docs/03_wit_and_manifest.md` (long; ranged reads only) - the `world-postpass` / `world-finalization` listings only.
- `CLAUDE.md` §"Guest WASM Staleness", §"Test Discipline", §"WIT/Type Changes Checklist", §"Config Key Naming Convention" - direct read.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `crates/slicer-schema/wit/README.md` section "Layout" — replace the `world-postpass` / `world-finalization` rows with the three per-stage package rows; correct the stale sentence "World packages carry `@1.0.0`" to state that each **stage** package carries `@1.0.0` and that dep packages stay unversioned. Verification grep: `rg -q 'postpass-gcode-postprocess/postpass-gcode-postprocess.wit' crates/slicer-schema/wit/README.md`
- `crates/slicer-schema/wit/README.md` section "How each consumer reads these files" — the host `bindgen!` example currently cites the non-existent path `crates/slicer-runtime/src/wit_host.rs`; correct it to `crates/slicer-wasm-host/src/host.rs` and to a per-stage `world:` key. Verification grep: `rg -q 'crates/slicer-wasm-host/src/host.rs' crates/slicer-schema/wit/README.md && (rg -q 'crates/slicer-runtime/src/wit_host.rs' crates/slicer-schema/wit/README.md && exit 1 || exit 0)`
- `docs/03_wit_and_manifest.md` sections "world-postpass.wit" and "world-finalization.wit" — replace with the three per-stage package listings (package header, interface, `run` func, world). Packet #3 owns the layer/prepass listings; these two are disjoint. Verification grep: `rg -q 'package slicer:finalization-layer-finalization@1.0.0' docs/03_wit_and_manifest.md && (rg -q 'package slicer:world-postpass' docs/03_wit_and_manifest.md && exit 1 || exit 0)`
- `docs/07_implementation_status.md` — record TASK-146b. Verification grep: `rg -q 'TASK-146b' docs/07_implementation_status.md`
- `docs/DEVIATION_LOG.md` — **one new row**, `DEV-086`, in the existing column format (`| DEV-### | date | files | Severity | description | owner | … | status |`). Severity **Medium**. The accepted intermediate: postpass + finalization are on per-stage packages while layer + prepass remain on tier worlds, so two contract mechanisms are live in-tree; `wit-world`, `SUPPORTED_WIT_WORLDS` and `validate_wit_world` remain live and now name two packages that no longer exist. Owner: `164_per-stage-wit-packages-bulk` (TASK-146c). **Confirm the id is still free at implementation time** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -3`) — `DEV-085` and `DEV-087` both landed while this packet was being authored, and `DEV-086` may not stay free either. Verification grep: `rg -q '^\| DEV-086 ' docs/DEVIATION_LOG.md && rg -q '164_per-stage-wit-packages-bulk' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` — **`DEV-087` is referenced, never created.** The `postpass_gcode_boundary_tdd.rs` no-instantiation defect that AC-12 excludes is **already filed and committed** as `DEV-087` (`57ceae39`, "docs(deviations): DEV-087 — MissingComponent laundered into Ok(success)"), carrying the same evidence chain this packet verified. Do not file a second row for it. Verification grep (reference, do not create): `rg -q '^\| DEV-087 ' docs/DEVIATION_LOG.md && rg -q 'postpass_gcode_boundary_tdd' docs/DEVIATION_LOG.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
