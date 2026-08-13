---
status: draft
packet: 225-dragon-curve-feasibility-gate
task_ids:
  - TASK-336
backlog_source: docs/specs/community-modules-dragon-curve-plan.md
context_cost_estimate: M
---

# Packet Contract: 225-dragon-curve-feasibility-gate

## Goal

Upgrade the workspace to wasmtime 47.0.3 and wit-bindgen 0.60.0, then run MoonBit, AssemblyScript, C++, and Go components through one deterministic `slicer:postpass-text-postprocess/text-postprocess-module` host oracle and select the first loadable-and-correct authoring language in the locked priority order MoonBit, AssemblyScript, C++, Go, Rust.

## Scope Boundaries

This packet owns the Dragon Curve plan's language-feasibility gate. It preserves the slicer-only linker, adds reproducible foreign-language probe fixtures and one ignored integration-test driver, records one evidence document per candidate, and writes the final selection to `docs/14_submodule_programming_languages.md`. AssemblyScript bindings come from the latest committed clean-tree HEAD of the read-only local fork `D:\wit-bindgen` on `feat/assemblyscript-backend`, resolved immediately before probe execution after the user confirms the concurrent async-support work is committed. This packet does not implement Dragon/Hilbert geometry or compare geometry with OrcaSlicer.

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: `226-authored-coloring-carrier` sequencing and `227-dragon-curve-community-module` language selection.
- Tooling gate: before each probe, check the exact commands in AC-N2. If a required command is absent, stop that implementation step, ask the user to install it, provide the installation and version-verification commands from that candidate's probe README, and wait. Absence is `BLOCKED: TOOLCHAIN`, never a candidate failure and never grounds fallback selection.
- Fork-readiness gate: if `D:\wit-bindgen` is not on `feat/assemblyscript-backend`, has any tracked or untracked changes, or the user has not confirmed the concurrent async-support work is committed, stop and ask the user. Never stash, clean, switch branches, pull, or probe the in-progress tree.
- Activation remains draft until preflight passes. Probe execution may block later on user-installed tools without invalidating this packet's authoring.

## Acceptance Criteria

- **AC-1. Given** the workspace root manifest, **when** the toolchain bump lands, **then** `workspace.dependencies.wasmtime` has version `47.0.3` and feature list `['call-hook']`, and `workspace.dependencies.wit-bindgen` is `0.60.0`. | `python3 -c "import tomllib; d=tomllib.load(open('Cargo.toml','rb'))['workspace']['dependencies']; assert d['wasmtime']=={'version':'47.0.3','features':['call-hook']},d['wasmtime']; assert d['wit-bindgen']=='0.60.0',d['wit-bindgen']; print('PASS')"`
- **AC-2. Given** all manifests under `crates/` and `modules/`, **when** the pin sweep completes, **then** none pins `wasmtime = "43.0.0"` or `wit-bindgen = "0.57.1"`. | `python3 -c "import pathlib; hits=[str(p) for root in ('crates','modules') for p in pathlib.Path(root).rglob('Cargo.toml') if any(x in p.read_text() for x in ('wasmtime = \"43.0.0\"','wit-bindgen = \"0.57.1\"'))]; assert not hits,hits; print('PASS')"`
- **AC-3. Given** the bumped workspace, **when** the compile, lint, and guest-freshness gates run, **then** all exit zero; the guest gate checks first, rebuilds only when its failed output contains `STALE:`, and finishes with a green freshness check. | `set -o pipefail; cargo check --workspace --all-targets >/dev/null && cargo clippy --workspace --all-targets -- -D warnings >/dev/null && { log=$(mktemp); if cargo xtask build-guests --check >"$log" 2>&1; then rm -f "$log"; else grep -q 'STALE:' "$log" || { tail -20 "$log"; rm -f "$log"; exit 1; }; rm -f "$log"; cargo xtask build-guests >/dev/null && cargo xtask build-guests --check 2>&1 | tail -20; fi; }`
- **AC-4. Given** an existing candidate component path in `PNP_FOREIGN_COMPONENT`, **when** `foreign_language_feasibility_tdd::foreign_language_text_postprocess_component` runs in the `integration` test binary, **then** the production `TextPostprocessModule` linker invokes `run-text-postprocess` with input `; probe input\n` and the test asserts the exact result `;; foreign-language-probe\n; probe input\n`. | `set -o pipefail; test -f "$PNP_FOREIGN_COMPONENT" && cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact 2>&1 | tail -20`
- **AC-5. Given** the four candidate probe records, **when** packet verification reads their evidence fields, **then** each has exactly one terminal result from the two permitted values, exactly one 64-hex component SHA-256, non-empty tool/host command and host-output fields, and no blocker marker. | `python3 -c "import pathlib,re; names=('moonbit','assemblyscript','cpp','go'); docs=[pathlib.Path(f'docs/feasibility-probes/{n}-text-postprocess.md').read_text() for n in names]; ok=lambda d: len(re.findall(r'^RESULT: (LOADABLE_AND_CORRECT|NOT_LOADABLE_OR_CORRECT)$',d,re.M))==1 and len(re.findall(r'^COMPONENT_SHA256: [0-9a-f]{64}$',d,re.M))==1 and all(re.search(rf'^{f}: .+',d,re.M) for f in ('TOOL_VERSIONS','HOST_COMMAND','HOST_OUTPUT')) and 'BLOCKED:' not in d; assert all(map(ok,docs)); print('PASS')"`
- **AC-6. Given** the user has confirmed the concurrent fork work is committed and `D:\wit-bindgen` is clean on `feat/assemblyscript-backend`, **when** the AssemblyScript evidence is finalized, **then** it records the 40-hex HEAD resolved immediately before generation, `WIT_BINDGEN_STATUS: clean`, UTF-16 component embedding, and world `slicer:postpass-text-postprocess/text-postprocess-module`. | `python3 -c "import pathlib,re; t=pathlib.Path('docs/feasibility-probes/assemblyscript-text-postprocess.md').read_text(); assert re.search(r'^WIT_BINDGEN_HEAD: [0-9a-f]{40}$',t,re.M); req=('WIT_BINDGEN_BRANCH: feat/assemblyscript-backend','WIT_BINDGEN_STATUS: clean','UTF-16','slicer:postpass-text-postprocess/text-postprocess-module'); assert all(x in t for x in req),[x for x in req if x not in t]; print('PASS')"`
- **AC-7. Given** all four completed probe results, **when** the Dragon Curve language is selected, **then** `docs/14_submodule_programming_languages.md` contains exactly one selection line naming the first `LOADABLE_AND_CORRECT` candidate in `MoonBit, AssemblyScript, C++, Go`, or `Rust` only if all four results are `NOT_LOADABLE_OR_CORRECT`. | `python3 -c "import pathlib,re; order=(('MoonBit','moonbit'),('AssemblyScript','assemblyscript'),('C++','cpp'),('Go','go')); vals=[]; [(lambda m,label: (m and vals.append((label,m.group(1)))))(re.fullmatch(r'RESULT: (LOADABLE_AND_CORRECT|NOT_LOADABLE_OR_CORRECT)',next(x for x in pathlib.Path(f'docs/feasibility-probes/{slug}-text-postprocess.md').read_text().splitlines() if x.startswith('RESULT: '))),label) for label,slug in order]; assert len(vals)==4; expected=next((label for label,v in vals if v=='LOADABLE_AND_CORRECT'),'Rust'); found=re.findall(r'\*\*Dragon Curve authoring language: ([^*]+)\*\*',pathlib.Path('docs/14_submodule_programming_languages.md').read_text()); assert found==[expected],(expected,found); print('PASS')"`
- **AC-8. Given** the committed probe fixture, **when** its contract is inspected, **then** it uses the existing package `slicer:postpass-text-postprocess@1.0.0`, world `text-postprocess-module`, export `run`, input `; probe input\n`, and expected output `;; foreign-language-probe\n; probe input\n` for every language. | `python3 -c "import pathlib; t=pathlib.Path('docs/feasibility-probes/foreign-language-text-postprocess/README.md').read_text(); req=('slicer:postpass-text-postprocess@1.0.0','text-postprocess-module','run','; probe input\\n',';; foreign-language-probe\\n; probe input\\n','MoonBit','AssemblyScript','C++','Go'); assert all(x in t for x in req),[x for x in req if x not in t]; print('PASS')"`
- **AC-9. Given** the workspace pins are wasmtime 47.0.3 and wit-bindgen 0.60.0, **when** dependency-version documentation is inspected, **then** the overview table says `47.0.3 workspace requirement` and `0.60.0 workspace requirement`, the SDK's current-macro note says `wit-bindgen 0.60.0`, and the language guide's workspace assignment says `wit-bindgen = "0.60.0"`; the MoonBit historical paragraph retains its measured `wit-bindgen rust 0.57.1` attribution. | `python3 -c "import pathlib; o=pathlib.Path('docs/00_project_overview.md').read_text(); s=pathlib.Path('docs/05_module_sdk.md').read_text(); l=pathlib.Path('docs/14_submodule_programming_languages.md').read_text(); assert '47.0.3 workspace requirement' in o and '0.60.0 workspace requirement' in o; assert 'current macro\n     (wit-bindgen 0.60.0)' in s; assert 'wit-bindgen = \"0.60.0\"' in l; assert 'wit-bindgen` rust 0.57.1 (the host)' in l; assert '43.0.0 workspace requirement' not in o and '0.57.1 workspace requirement' not in o and 'current macro\n     (wit-bindgen 0.57.1)' not in s and 'wit-bindgen = \"0.57.1\"' not in l; print('PASS')"`

## Negative Test Cases

- **AC-N1. Given** the existing `sdk-postpass-text-guest.component.wasm`, whose output differs from the foreign-language oracle, **when** the shared ignored integration driver runs against it, **then** the command observes a nonzero test exit and the diagnostic contains `foreign component returned wrong output`. | `set -o pipefail; p='crates/slicer-wasm-host/test-guests/sdk-postpass-text-guest.component.wasm'; test -f "$p" || exit 1; log=$(mktemp); if PNP_FOREIGN_COMPONENT="$p" cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact >"$log" 2>&1; then tail -20 "$log"; rm -f "$log"; exit 1; else grep -F 'foreign component returned wrong output' "$log" | tail -1; rc=$?; rm -f "$log"; exit $rc; fi`
- **AC-N2. Given** a simulated absent `asc` command, **when** the shared prerequisite gate runs, **then** it exits 42, prints `BLOCKED: TOOLCHAIN assemblyscript asc`, and prints the AssemblyScript installation plus version-verification instructions without creating a result record. | `set -o pipefail; log=$(mktemp); docs/feasibility-probes/foreign-language-text-postprocess/check-prerequisites.sh --simulate-missing asc >"$log" 2>&1; rc=$?; test "$rc" -eq 42 && grep -F 'BLOCKED: TOOLCHAIN assemblyscript asc' "$log" && grep -F 'INSTALL:' "$log" && grep -F 'VERIFY:' "$log" && test ! -e docs/feasibility-probes/assemblyscript-text-postprocess.md; out=$?; rm -f "$log"; exit $out`
- **AC-N3. Given** a simulated dirty `D:\wit-bindgen` checkout, **when** the fork-readiness gate runs, **then** it exits 43, prints `BLOCKED: FORK_NOT_READY dirty`, and emits no `GENERATION_COMMAND:` line. | `set -o pipefail; log=$(mktemp); docs/feasibility-probes/foreign-language-text-postprocess/check-fork-readiness.sh --simulate-dirty >"$log" 2>&1; rc=$?; test "$rc" -eq 43 && grep -F 'BLOCKED: FORK_NOT_READY dirty' "$log" && ! grep -q '^GENERATION_COMMAND:' "$log"; out=$?; rm -f "$log"; exit $out`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests && cargo xtask build-guests --check`

## Authoritative Docs

- `docs/specs/community-modules-dragon-curve-plan.md` - queue ownership and `TASK-336` mapping.
- `docs/specs/community-modules-dragon-curve-infill.md` section 6 - feasibility gate and Rust fallback.
- `docs/14_submodule_programming_languages.md` - living language verdict authority.
- `docs/feasibility-probes/go-wasm.md` and `docs/feasibility-probes/moonbit-wasm.md` - prior candidate evidence, retained as historical baselines.
- `D:\wit-bindgen\README.md:267-439`, `D:\wit-bindgen\crates\assemblyscript\src\lib.rs`, and `D:\wit-bindgen\crates\test\src\{assemblyscript,cpp}.rs` - read-only local generator/build authority; its latest committed clean HEAD is resolved at probe time.

## Doc Impact Statement (Required)

- Add `docs/feasibility-probes/{moonbit,assemblyscript,cpp,go}-text-postprocess.md`, each with commands, versions, component digest, invocation output, and one `RESULT:`. | `for n in moonbit assemblyscript cpp go; do rg -q '^RESULT: (LOADABLE_AND_CORRECT|NOT_LOADABLE_OR_CORRECT)$' "docs/feasibility-probes/$n-text-postprocess.md" || exit 1; done`
- Add `docs/feasibility-probes/foreign-language-text-postprocess/README.md` plus candidate source/build fixtures; this owns the shared contract and missing-tool stop instructions. | `rg -q 'slicer:postpass-text-postprocess@1\.0\.0' docs/feasibility-probes/foreign-language-text-postprocess/README.md && rg -q 'BLOCKED: TOOLCHAIN' docs/feasibility-probes/foreign-language-text-postprocess/README.md && rg -q 'BLOCKED: FORK_NOT_READY' docs/feasibility-probes/foreign-language-text-postprocess/README.md`
- Update `docs/14_submodule_programming_languages.md` with the four measured results and one `**Dragon Curve authoring language: ...**` line. | `test "$(rg -c '^\*\*Dragon Curve authoring language: (MoonBit|AssemblyScript|C\+\+|Go|Rust)\*\*$' docs/14_submodule_programming_languages.md)" -eq 1`
- Update dependency-version statements in `docs/00_project_overview.md`, `docs/05_module_sdk.md`, and `docs/14_submodule_programming_languages.md` from the prior workspace versions to wasmtime 47.0.3 and wit-bindgen 0.60.0 while retaining the MoonBit paragraph's historical 0.57.1 attribution. | `python3 -c "import pathlib; o=pathlib.Path('docs/00_project_overview.md').read_text(); s=pathlib.Path('docs/05_module_sdk.md').read_text(); l=pathlib.Path('docs/14_submodule_programming_languages.md').read_text(); assert '47.0.3 workspace requirement' in o and '0.60.0 workspace requirement' in o and 'current macro\n     (wit-bindgen 0.60.0)' in s and 'wit-bindgen = \"0.60.0\"' in l and 'wit-bindgen` rust 0.57.1 (the host)' in l; assert '43.0.0 workspace requirement' not in o and '0.57.1 workspace requirement' not in o and 'current macro\n     (wit-bindgen 0.57.1)' not in s and 'wit-bindgen = \"0.57.1\"' not in l; print('PASS')"`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
