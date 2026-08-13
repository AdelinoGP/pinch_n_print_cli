---
status: draft
packet: 225-dragon-curve-feasibility-gate
task_ids:
  - TASK-336
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 225-dragon-curve-feasibility-gate

## Goal

Bump the workspace wasmtime and wit-bindgen toolchain to 47.0.3 / 0.60.0, absorb the API fallout, re-run the Go feasibility probe against the updated host, and record the gate verdict plus the Dragon Curve module's chosen authoring language.

## Scope Boundaries

This packet owns the spec §6 Step 0 feasibility gate only. It updates the workspace toolchain, proves the bump is safe, re-runs the Go probe (MoonBit is recorded "not re-run — toolchain absent"), and writes the verdicts into `docs/14_submodule_programming_languages.md` §Community-module context and `docs/feasibility-probes/go-wasm.md`. It does not author the Dragon Curve module (packet 227) and does not land the authored-coloring mechanism (packet 226).

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: `226-authored-coloring-carrier` (its guest rebuild must target the post-bump toolchain to avoid a double rebuild); `227-dragon-curve-community-module` (consumes this packet's gate verdict).
- Activation blockers: none. The gate verdict is a measurement, not a precondition for this packet's own activation; packet 227 must not activate until this packet's verdict is recorded.

## Acceptance Criteria

- **AC-1. Given** the workspace root manifest, **when** this packet's Step 1 lands, **then** `Cargo.toml` pins `wasmtime = "47.0.3"` (with `call-hook`) and `wit-bindgen = "0.60.0"`. | `rg -n 'wasmtime = \{ version = "47\.0\.3", features = \["call-hook"\] \}|wit-bindgen = "0\.60\.0"' Cargo.toml`
- **AC-2. Given** every `Cargo.toml` under `crates/` and `modules/`, **when** the sweep step runs, **then** no manifest still pins `wasmtime = "43.*"` or `wit-bindgen = "0.57.1"`. | `rg -n 'wasmtime = "43\.0\.0"|wit-bindgen = "0\.57\.1"' crates modules --glob 'Cargo.toml' && echo 'FAIL: stale pins remain' || echo 'PASS: no stale pins'`
- **AC-3. Given** the bumped workspace, **when** `cargo check --workspace --all-targets` runs, **then** it exits 0 (all targets compile, including the test guests and every bindgen consumer). | `cargo check --workspace --all-targets 2>&1 | tail -30`
- **AC-4. Given** the bumped workspace, **when** guests are rebuilt and re-checked, **then** `build-guests` exits 0 and `build-guests --check` exits 0 (no stale guest artifacts). | `cargo xtask build-guests 2>&1 | tail -20 && cargo xtask build-guests --check 2>&1 | tail -20`
- **AC-5. Given** the bumped workspace, **when** the clippy gate runs, **then** it exits 0 with `-D warnings` across all targets. | `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -30`
- **AC-6. Given** the Go re-check script in the scratchpad, **when** it is executed with Go 1.26.5, wasm-tools 1.250.0, wit-bindgen-cli 0.60.0 (`go` feature), and a wasmtime 47 slicer-only linker, **then** it produces a `VERDICT:` line whose recorded result (loadable-and-correct OR instantiation-failed) is transcribed verbatim into `docs/feasibility-probes/go-wasm.md`'s appended re-check section. | `python3 -c "import re,sys; t=open('docs/feasibility-probes/go-wasm.md',encoding='utf-8').read(); m=re.search(r'## Go probe — re-check \(2026-08-\d+\).*?### Verdict table(.*?)(\n## |\Z)', t, re.S); assert m, 'missing re-check section'; body=m.group(1); assert 'RESULT:' in body and ('INSTANTIATION FAILED' in body or 'INSTANTIATED OK' in body), body; print('PASS')"`
- **AC-7. Given** `docs/14_submodule_programming_languages.md`, **when** this packet lands, **then** the MoonBit verdict paragraph is explicitly marked not re-run on this toolchain. | `rg -n 'not re-run \(toolchain absent\)' docs/14_submodule_programming_languages.md`
- **AC-8. Given** `docs/14_submodule_programming_languages.md`, **when** this packet lands, **then** the section-intro paragraph no longer asserts "the two probes are complete and neither is loadable-and-correct". | `rg -n 'complete and neither is loadable-and-correct' docs/14_submodule_programming_languages.md && echo 'FAIL: stale intro' || echo 'PASS: intro updated'`
- **AC-9. Given** `docs/14_submodule_programming_languages.md`, **when** this packet lands, **then** §Community-module context carries exactly one gate-verdict line matching one of the two permitted forms (PASS→Go, or FALLBACK CONFIRMED→Rust). | `python3 -c "import re; t=open('docs/14_submodule_programming_languages.md',encoding='utf-8').read(); m=re.search(r'\*\*Gate verdict \(re-check\): ([^*]+)\*\*', t); assert m, 'missing gate verdict'; v=m.group(1).strip(); assert v in ('PASS — Go component loadable-and-correct; authoring language = Go.','FALLBACK CONFIRMED — no non-Rust component loadable-and-correct; authoring language = Rust (Go tiling source retained as labeled reference only).'), v; print('PASS')"`

## Negative Test Cases

- **AC-N1. Given** a stale toolchain pin anywhere under `crates/` or `modules/`, **when** the sweep verification runs, **then** the gate fails with the stale file listed (no silent partial bump). | `rg -n 'wasmtime = "43\.0\.0"|wit-bindgen = "0\.57\.1"' crates modules --glob 'Cargo.toml' && echo 'FAIL: stale pins remain' || echo 'PASS'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests && cargo xtask build-guests --check`

## Authoritative Docs

- `docs/specs/community-modules-dragon-curve-plan.md` — direct read (102 lines) — binding symbol contract and grounding facts.
- `docs/specs/community-modules-dragon-curve-infill.md` §6 — direct read — the gate's normative procedure.
- `docs/feasibility-probes/go-wasm.md` — direct read (197 lines) — the probe brief and original evidence.
- `docs/feasibility-probes/moonbit-wasm.md` — delegated SUMMARY of §2/§8 only (222 lines) — MoonBit original verdict and commands; no re-run here.
- `docs/14_submodule_programming_languages.md` — direct read of §Community-module context (lines 96–171) — the living verdict table being edited.

## Doc Impact Statement (Required)

- `docs/14_submodule_programming_languages.md` — §Community-module context: update the section-intro paragraph (lines 98–105), the Go verdict paragraph (lines 107–138), and the MoonBit verdict paragraph (lines 140–171) to record the re-check; add the gate-verdict line. `rg -q 'Gate verdict \(re-check\):' docs/14_submodule_programming_languages.md`; `rg -q 'not re-run \(toolchain absent\)' docs/14_submodule_programming_languages.md`; `! rg -q 'complete and neither is loadable-and-correct' docs/14_submodule_programming_languages.md`
- `docs/feasibility-probes/go-wasm.md` — append a dated `## Go probe — re-check (2026-08-DD)` section with a `### Verdict table` and a `RESULT:` line. `rg -q 'Go probe — re-check' docs/feasibility-probes/go-wasm.md`; `rg -q '### Verdict table' docs/feasibility-probes/go-wasm.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
