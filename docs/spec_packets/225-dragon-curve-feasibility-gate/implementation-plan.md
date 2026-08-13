# Implementation Plan: 225-dragon-curve-feasibility-gate

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Discovery — enumerate the wasmtime/wit-bindgen API surface and pin sites

- Task IDs: `TASK-336`
- Objective: Produce the authoritative inventory that bounds Steps 2–4: (a) the exact stale-pin list under `crates/` and `modules/`, (b) the exact wasmtime-API occurrence list, (c) the exact wit-bindgen generated-shape consumer list.
- Precondition: workspace present at `F:\slicerProject\pinch_n_print_cli_2`.
- Postcondition: a scratchpad inventory file records three lists — stale pins (root + in-tree), wasmtime API sites (file:line), and generated-shape consumers (file:line) — and each list is cited inline in the later steps.
- Files allowed to read, with ranges when over 300 lines:
  - `Cargo.toml` - lines 1-116
  - `crates/**/Cargo.toml` and `modules/**/Cargo.toml` - grep-only, no full reads
- Files allowed to edit (at most 3):
  - `$COMMANDCODE_SCRATCHPAD/225_inventory.md`
- Files explicitly out of bounds:
  - `target/`, `Cargo.lock`, `**/wit-guest/**` generated code, `OrcaSlicerDocumented/`
- Blast-radius discipline: n/a (no struct/schema change in this packet).
- Expected sub-agent dispatches:
  - Question: list every `wasmtime`/`wit-bindgen` pin with file:line under `crates/` and `modules/` (`--glob Cargo.toml`); scope: `crates/ modules/`; return: `LOCATIONS`
  - Question: list every `wasmtime::component::bindgen!`, `add_to_linker`, `ResourceLimiter`, `call_hook`, `get_fuel`, `set_fuel`, `Store::new`, `Engine` occurrence with file:line in the four host/runtime files; scope: `crates/slicer-wasm-host/src/{host.rs,dispatch.rs,instance.rs}` + `crates/slicer-runtime/src/run.rs`; return: `LOCATIONS`
  - Question: list every `wit_bindgen::generate!` / `bindgen!` generated-shape consumer (sdk, macros, guests) with file:line; scope: `crates/slicer-sdk/src`, `crates/slicer-macros/src`, `modules/*/wit-guest`, `crates/slicer-wasm-host/test-guests/*/wit-guest`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-plan.md` - direct read (Grounding facts)
- OrcaSlicer refs:
  - none (no parity obligation)
- Verification:
  - `test -s "$COMMANDCODE_SCRATCHPAD/225_inventory.md" && echo PASS || echo FAIL` - FACT pass/fail
- Exit condition: inventory file non-empty and lists the three categories with concrete file:line entries.

### Step 2: Bump workspace root pins

- Task IDs: `TASK-336`
- Objective: Land `wasmtime = { version = "47.0.3", features = ["call-hook"] }` and `wit-bindgen = "0.60.0"` at `Cargo.toml:61-62`.
- Precondition: Step 1 inventory confirms the two root lines are the only workspace-dependencies pins.
- Postcondition: root manifest pins the two target versions; the `call-hook` feature comment is preserved.
- Files allowed to read, with ranges when over 300 lines:
  - `Cargo.toml` - lines 56-63
- Files allowed to edit (at most 3):
  - `Cargo.toml`
- Files explicitly out of bounds:
  - every crate/module manifest (swept in Step 3), `Cargo.lock`
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches:
  - none (two-line edit against the already-read root manifest)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-plan.md` - Grounding facts (already read)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -n 'wasmtime = \{ version = "47\.0\.3", features = \["call-hook"\] \}|wit-bindgen = "0\.60\.0"' Cargo.toml` - FACT pass (2 matches)
- Exit condition: both target pins present at root; AC-1 satisfied.

### Step 3: Sweep stale in-tree pins

- Task IDs: `TASK-336`
- Objective: Ensure no `crates/**/Cargo.toml` or `modules/**/Cargo.toml` still pins `wasmtime = "43.0.0"` or `wit-bindgen = "0.57.1"`.
- Precondition: Step 2 landed the root bump; Step 1 inventory lists the known in-tree pin sites (grounded: 24 inline `wit-bindgen = "0.57.1"` manifests — 21 `test-guests/*/Cargo.toml` + 3 `wit-guest/Cargo.toml` — plus the workspace-pinned crates).
- Postcondition: zero stale pins under `crates/` and `modules/`; every inline `0.57.1` pin becomes `0.60.0` (test-guests and wit-guest crates use the bare `0.57.1` form today); workspace-pinned crates use `wit-bindgen.workspace = true` and are already correct.
- Files allowed to read, with ranges when over 300 lines:
  - each stale-pin manifest named by Step 1 - grep-confirmed line only
- Files allowed to edit (at most 3 — justified exception):
  - the 24 inline-pinned manifests. This exceeds the 3-file cap because a version sweep is one mechanical transformation (`0.57.1` → `0.60.0` at the identical line position) applied 24 times, not 24 independent edits; all sites are already enumerated by the Step 1 `LOCATIONS` dispatch, so no additional reading is incurred. If the reviewer requires strict 3-edit batching, split into eight sub-batches of 3 manifests each (no cost change; the transformation is identical).
- Files explicitly out of bounds:
  - `Cargo.lock`, `target/`, generated code
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches:
  - Question: for each manifest in the Step 1 stale-pin list, confirm the exact line and whether it is a bare `0.57.1` pin or a `workspace = true` reference; scope: `crates/ modules/`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - none beyond Step 1 inventory
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -n 'wasmtime = "43\.0\.0"|wit-bindgen = "0\.57\.1"' crates modules --glob 'Cargo.toml' && echo 'FAIL: stale pins remain' || echo 'PASS: no stale pins'` - FACT pass/fail
- Exit condition: AC-2 and AC-N1 satisfied (zero stale pins).

### Step 4: Absorb toolchain API fallout to a green compile gate

- Task IDs: `TASK-336`
- Objective: Fix every wasmtime 47 / wit-bindgen 0.60 fallout surfaced by `cargo check --workspace --all-targets` so the workspace compiles; then rebuild and re-check all guests.
- Precondition: Steps 2–3 complete (no stale pins).
- Postcondition: `cargo check --workspace --all-targets` exits 0; `cargo xtask build-guests` exits 0; `cargo xtask build-guests --check` exits 0.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/instance.rs` - lines 140-220 (engine/store/call-hook/fuel)
  - `crates/slicer-wasm-host/src/host.rs` - grep-delegated ranges only: bindgen blocks near 331-600 and 914-1060; `ResourceLimiter` near 1087-1124
  - `crates/slicer-wasm-host/src/dispatch.rs` - grep-delegated `add_to_linker`/`Linker::new` ranges only
  - `crates/slicer-runtime/src/run.rs` - grep-delegated `WasmInstancePool`/engine range only
  - `crates/slicer-sdk/src/host.rs` - lines 32-70 (sdk-host-services `generate!` block)
  - `crates/slicer-macros/src/lib.rs` - grep-delegated ranges near 1290-1330, 2599-2603, 2739-2750
- Files allowed to edit (at most 3):
  - the compile-error files enumerated by `cargo check` (this step may legitimately exceed 3 files — a toolchain bump is cross-cutting; each edit is a mechanical signature update, and the step is justified by the bounded LOCATIONS dispatch)
- Files explicitly out of bounds:
  - `target/`, `Cargo.lock`, `**/wit-guest/**` generated code (regenerated, never hand-edited), `OrcaSlicerDocumented/`
- Blast-radius discipline: n/a (no struct/schema change; the "blast radius" here is compile fallout, enumerated by the gate itself).
- Expected sub-agent dispatches:
  - Question: run `cargo check --workspace --all-targets 2>&1 | grep -E '^error' | head -50` and return the file:line:error triples; scope: workspace; return: `SNIPPETS` (≤50 lines)
  - Question: for each error site, locate the exact wasmtime 47 / wit-bindgen 0.60 signature to mirror; scope: `~/.cargo/registry/src/**/wasmtime-47.0.3/**` and `**/wit-bindgen-0.60.0/**`; return: `SNIPPETS`
- Context cost: `M`
- Authoritative docs:
  - `docs/14_submodule_programming_languages.md` - not needed here
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tail -30` - FACT exit 0
  - `cargo xtask build-guests 2>&1 | tail -20` - FACT exit 0
  - `cargo xtask build-guests --check 2>&1 | tail -20` - FACT exit 0
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -30` - FACT exit 0
  - `cargo test -p slicer-wasm-host --test contract host_services_tdd 2>&1 | tail -20` - FACT exit 0
  - `cargo test -p slicer-runtime --test contract wit_drift_detection_tdd 2>&1 | tail -20` - FACT exit 0
- Exit condition: all six verification commands exit 0; AC-3, AC-4, AC-5 satisfied.

### Step 5: Re-run the Go feasibility probe against wasmtime 47

- Task IDs: `TASK-336`
- Objective: Reproduce the spec §6 step 3 Go probe with the updated toolchain and record an honest `VERDICT:` line (loadable-and-correct OR instantiation-failed), plus the `RESULT:` line for the docs.
- Precondition: Step 4 proved the wasmtime 47 host compiles, so the probe's slicer-only linker matches production. Toolchain confirmed present: Go 1.26.5, wasm-tools 1.250.0; wit-bindgen-cli 0.60.0 (`--features go`) is installed into the scratch environment.
- Postcondition: `$COMMANDCODE_SCRATCHPAD/225_go_recheck.md` records the tool versions, the exact commands (from `docs/feasibility-probes/go-wasm.md` §8), and the `RESULT:` line. MoonBit is explicitly recorded `not re-run (toolchain absent)` in the same file.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/feasibility-probes/go-wasm.md` - lines 1-197 (the full probe brief and §8 commands)
- Files allowed to edit (at most 3):
  - `$COMMANDCODE_SCRATCHPAD/225_go_recheck.md` (scratchpad-only; no tree files edited in this step)
- Files explicitly out of bounds:
  - the repo tree (scratch-only probe artifacts), `target/`, `Cargo.lock`
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches:
  - Question: confirm `go version`, `wasm-tools --version`, and that `cargo install wit-bindgen-cli --version 0.60.0 --features go` succeeded; also confirm `moon --version` fails (absent); scope: shell; return: `FACT`
  - Question: reproduce the §8 command sequence in the scratchpad and capture the final instantiation output; scope: scratchpad dir; return: `SNIPPETS` (the `RESULT:` line and the failing import name if any)
- Context cost: `M`
- Authoritative docs:
  - `docs/feasibility-probes/go-wasm.md` - direct read (already read; §4b and §8 are the binding recipe)
- OrcaSlicer refs:
  - none
- Verification:
  - `test -s "$COMMANDCODE_SCRATCHPAD/225_go_recheck.md" && grep -q 'RESULT:' "$COMMANDCODE_SCRATCHPAD/225_go_recheck.md" && echo PASS || echo FAIL` - FACT pass/fail
- Exit condition: scratch re-check record contains a `RESULT:` line and a MoonBit `not re-run (toolchain absent)` note; AC-6's evidence is produced.

### Step 6: Record verdicts in docs/14 and append the go-wasm re-check section

- Task IDs: `TASK-336`
- Objective: Transcribe the Step 5 verdict into the living docs and append the dated re-check section, closing the gate with the single recorded verdict packet 227 consumes.
- Precondition: Step 5 scratch record exists with a `RESULT:` line.
- Postcondition: `docs/14_submodule_programming_languages.md` §Community-module context has (a) the section intro updated to no longer claim "complete and neither is loadable-and-correct", (b) the Go verdict paragraph updated with the re-check result, (c) the MoonBit verdict paragraph marked `not re-run (toolchain absent)`, and (d) a single `**Gate verdict (re-check): …**` line in exactly one of the two permitted forms. `docs/feasibility-probes/go-wasm.md` has an appended `## Go probe — re-check (2026-08-DD)` section with a `### Verdict table` and the `RESULT:` line.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/14_submodule_programming_languages.md` - lines 96-171 (the three paragraphs being edited)
  - `docs/feasibility-probes/go-wasm.md` - lines 175-197 (the append point)
- Files allowed to edit (at most 3):
  - `docs/14_submodule_programming_languages.md`
  - `docs/feasibility-probes/go-wasm.md`
- Files explicitly out of bounds:
  - `docs/specs/community-modules-dragon-curve-infill.md` (not edited), `docs/DEVIATION_LOG.md`, `docs/adr/`
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches:
  - Question: confirm the exact §Community-module context line ranges and current wording of the three paragraphs (intro, Go, MoonBit); scope: `docs/14_submodule_programming_languages.md`; return: `SNIPPETS`
- Context cost: `S`
- Authoritative docs:
  - `docs/14_submodule_programming_languages.md` - direct range read
- OrcaSlicer refs:
  - none
- Verification:
  - AC-7 command (MoonBit marker grep)
  - AC-8 command (stale-intro absence grep)
  - AC-9 command (single gate-verdict form check)
  - AC-6 command (go-wasm re-check section + RESULT line)
- Exit condition: all four AC commands (AC-6 through AC-9) return PASS; the gate verdict is recorded and unambiguous for packet 227.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | grep-inventory discovery, all reads delegated |
| Step 2 | S | two-line root edit |
| Step 3 | S | stale-pin sweep (split if >3 edits) |
| Step 4 | M | compile-fallout absorption, bounded by LOCATIONS dispatch |
| Step 5 | M | scratch Go probe re-run |
| Step 6 | S | doc transcription |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate = M (Steps 4 and 5 are the M steps; neither is L because both are grep/scratch-bounded).

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
