---
status: implemented
packet: workspace-aware-guest-builder
task_ids:
  - TASK-214
---

# 70_workspace-aware-guest-builder

## Goal

Replace `modules/core-modules/build-core-modules.sh` AND `test-guests/build-test-guests.sh` with a single `cargo xtask build-guests [--check]` command using a validated filesystem walk for discovery. The xtask discovers guest crates via a walk over two tree-roots (no hardcoded module lists), tracks freshness via file mtimes, runs `wasm-tools component new` post-processing per a configurable artifact-path convention, and lands as a new `xtask/` crate with no dependency on `slicer-runtime` so agentic hooks can invoke it without compiling the full runtime closure. The `docs/05_module_sdk.md` module-author build-flow section is rewritten to document the `cargo build --target wasm32-unknown-unknown --release` + `wasm-tools component new` two-step plus a sidebar pointing workspace contributors at `cargo xtask build-guests`.

## Problem Statement

The workspace ships two near-identical bash scripts — `modules/core-modules/build-core-modules.sh` (~220 lines) and `test-guests/build-test-guests.sh` (~200 lines, verified structurally similar) — that build guest WASMs from source. Each script hardcodes a list of crates as a bash array (`MODULES=( "layer-planner-default:layer_planner_default_guest" … )` and a `SHARED_GUEST_CRATES=( … )` array of 4 deps), walks `wit/**/*.wit` mtimes by hand, runs `cargo build --target wasm32-unknown-unknown --release` per guest, post-processes each output with `wasm-tools component new`, and supports a `--check` mode that exits non-zero if any guest is stale. Adding a new guest crate requires editing both the workspace `Cargo.toml` `members` AND the relevant bash array — drift is silent (a missing array entry causes the guest to be skipped without warning). `CLAUDE.md` spends ~40 lines warning agents not to attribute test failures to other causes before running `--check` on both scripts — a bandage over a shallow interface. The redundancy and the dual maintenance burden go away if a `cargo metadata`-driven discovery routine enumerates guests from workspace structure: a workspace member is a guest if its directory matches a configured tree-root (`modules/core-modules/<dir>/wit-guest/` or `test-guests/<dir>/`) AND its `Cargo.toml` declares `[lib] crate-type = ["cdylib"]` AND `wit-bindgen` as a dep. A new `xtask/` crate exposing `cargo xtask build-guests [--check]` replaces both scripts with one binary that has zero dependency on `slicer-runtime` — agentic hooks (e.g., Claude Code pre-tool-use hooks) can invoke `--check` cheaply without compiling wasmtime/pyo3/truck-stepio.

## Architecture Constraints

- The `xtask` crate must NOT depend on `slicer-runtime`. Driving reason: agentic hooks (Claude Code pre-tool-use hooks running `cargo xtask build-guests --check`) need cheap compile cost. Verify by `cargo tree -p xtask` not containing `slicer-runtime`, `wasmtime`, `pyo3`, `truck-stepio`, or `meshopt`. Discovery is filesystem-only (validated walk over `modules/core-modules/*/wit-guest/Cargo.toml` and `test-guests/*/Cargo.toml`); `cargo_metadata` is intentionally absent because guest Cargo.tomls declare `[workspace]` sentinels and are invisible to the parent workspace's metadata.
- The freshness rule mirrors the bash scripts exactly: tracked source surfaces are `wit/**/*.wit` + the 4 shared crates (`slicer-macros`, `slicer-sdk`, `slicer-ir`, `slicer-schema`) + per-guest `src/` + per-guest `Cargo.toml`. `slicer-core` and `slicer-helpers` are intentionally NOT tracked (per bash-script comments — `slicer-core` is depended on by only ~6 of ~20 guests, global tracking causes spurious rebuilds; `slicer-helpers` is host-only).
- The artifact-output convention is per-tree:
  - `modules/core-modules/<dir>/wit-guest/Cargo.toml` → component lands at `modules/core-modules/<dir>/<dir>.wasm` (the manifest loader's resolution path; matches existing bash output).
  - `test-guests/<crate-name>/Cargo.toml` → component lands at `test-guests/<crate-name>.component.wasm` (matches existing bash output).
- The xtask MUST run `wasm-tools component new` with the same flag shape as the existing scripts: `wasm-tools component new <core_wasm> -o <component_wasm>`. No additional flags. (This is the verified shape from `cli/slicer-cli/src/cmd_build.rs:131-142`.)
- The xtask must not silently drop guests on parse or build errors. A discovery filter mismatch (e.g., a guest crate that doesn't declare `[lib] crate-type = ["cdylib"]`) is surfaced as a `SKIP: <name> (reason)` line on stderr but does not fail the build. A build or `wasm-tools` failure for any guest fails the whole xtask invocation with the guest name and the first 20 lines of the underlying tool's error.
- The packet's change surface includes `crates/slicer-schema` (no — verify: this packet does NOT edit `slicer-schema`; that was Packet 1). Therefore the wasm-staleness snippet does NOT apply to this packet's change surface — the xtask is host-side workspace tooling, and the workspace bash-script retirement happens to the build mechanism itself, not to inputs that feed guest WASM contents. The xtask IS the rebuild tool, so by definition it does not require its own re-run after editing. (Validation: the implementer who edits `xtask/` does not need to re-run guest builds; the implementer who edits `wit/` or the 4 shared crates DOES — and that rule is preserved verbatim, just behind the new entry point.)

## Data and Contract Notes

- **WASM binary layout**: unchanged. The xtask emits the same `wasm-tools component new` shape with the same flags.
- **Component output paths**: unchanged per tree (`modules/core-modules/<dir>/<dir>.wasm` and `test-guests/<crate-name>.component.wasm`). The manifest loader (`crates/slicer-runtime/src/manifest.rs`) resolves modules at the first path; any breakage there indicates the artifact-convention callback is wrong.
- **Freshness contract**: the xtask reports STALE iff source mtime > artifact mtime, where source is the union defined in `crates/slicer-runtime/src/manifest.rs` (no — the rule is documented in the bash script comments; the actual files are `wit/`, the 4 shared crates, and per-guest sources). This is a build-tool contract, not a runtime one.
- **CI**: depends on whether CI currently invokes the bash scripts (verify in Step 5). Today's `.github/workflows/ci.yml:47,49` does NOT — it only runs cargo tests.

## Locked Assumptions and Invariants

- The two bash scripts share their freshness logic verbatim except for the per-tree guest list and artifact path. The Rust port preserves this — one freshness function, one build function, two call sites with different `(tree_root, artifact_convention)` tuples.
- Discovery is a validated filesystem walk over two tree-roots. The validation predicate is:
  - **Core-module wit-guest** at `modules/core-modules/<dir>/wit-guest/Cargo.toml`: `[lib].crate-type` contains `"cdylib"` AND a `[workspace]` sentinel is present AND `[dependencies]` declares a `{ path = ".." }` dep on the sibling parent module crate.
  - **Test-guest** at `test-guests/<dir>/Cargo.toml`: `[lib].crate-type` contains `"cdylib"` AND a `[workspace]` sentinel is present AND `[dependencies]` declares `wit-bindgen` directly.
  Candidates failing validation are skipped (logged to stderr) but do not fail the xtask run. This pair of predicates is the symmetric guest-vs-non-guest signal — guest crates are uniquely the ones isolated by a `[workspace]` sentinel and shaped for cdylib output.
- The component-artifact output paths above are stable across the lifetime of this packet. Any change in artifact convention requires a coordinated change in `crates/slicer-runtime/src/manifest.rs` and is OUT OF SCOPE for this packet.

## Risks and Tradeoffs

- **Discovery filter false negatives**: if a future guest crate declares `crate-type = ["cdylib", "rlib"]` instead of just `["cdylib"]`, the filter might miss it. Mitigation: filter checks for `cdylib` IN the crate-type list, not equality. Verified by reading bash-script logic; ported with same lenience.
- **Wasm-tools availability**: the xtask shells out to `wasm-tools`. If `wasm-tools` is not on PATH, the build fails with a non-obvious error. Mitigation: at xtask startup, check `Command::new("wasm-tools").arg("--version").status()` and emit a clear `error: wasm-tools not found on PATH; install with 'cargo install wasm-tools'` before doing anything else.
- **Parallelism**: cargo serialises `cargo build -p <name>` invocations within one xtask run. Total wall-clock is sum of per-guest build times, not max. The bash scripts had the same serial behaviour, so this is not a regression. Mitigation: not addressed in this packet; future optimisation if it matters.
- **CI invocation change**: Step 5 is conditional — the implementer must verify CI's current state before editing. If `.github/workflows/ci.yml` does NOT invoke the bash scripts (today's reality), step 5 either (a) adds a new `cargo xtask build-guests --check` step before the test step, or (b) is a no-op. The decision goes to the implementer; AC-6 requires the file references `cargo xtask build-guests` ONLY if CI gains the invocation — phrasing of AC-6 is conservative ("references"... so adding any reference satisfies it).
