# Requirements: workspace-aware-guest-builder

## Packet Metadata

- Grouped task IDs:
  - `TASK-214`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The workspace ships two near-identical bash scripts — `modules/core-modules/build-core-modules.sh` (~220 lines) and `test-guests/build-test-guests.sh` (~200 lines, verified structurally similar) — that build guest WASMs from source. Each script hardcodes a list of crates as a bash array (`MODULES=( "layer-planner-default:layer_planner_default_guest" … )` and a `SHARED_GUEST_CRATES=( … )` array of 4 deps), walks `wit/**/*.wit` mtimes by hand, runs `cargo build --target wasm32-unknown-unknown --release` per guest, post-processes each output with `wasm-tools component new`, and supports a `--check` mode that exits non-zero if any guest is stale. Adding a new guest crate requires editing both the workspace `Cargo.toml` `members` AND the relevant bash array — drift is silent (a missing array entry causes the guest to be skipped without warning). `CLAUDE.md` spends ~40 lines warning agents not to attribute test failures to other causes before running `--check` on both scripts — a bandage over a shallow interface. The redundancy and the dual maintenance burden go away if a `cargo metadata`-driven discovery routine enumerates guests from workspace structure: a workspace member is a guest if its directory matches a configured tree-root (`modules/core-modules/<dir>/wit-guest/` or `test-guests/<dir>/`) AND its `Cargo.toml` declares `[lib] crate-type = ["cdylib"]` AND `wit-bindgen` as a dep. A new `xtask/` crate exposing `cargo xtask build-guests [--check]` replaces both scripts with one binary that has zero dependency on `slicer-runtime` — agentic hooks (e.g., Claude Code pre-tool-use hooks) can invoke `--check` cheaply without compiling wasmtime/pyo3/truck-stepio.

## In Scope

- Add a new `xtask/` workspace member (workspace-relative, conventional `xtask` pattern). `xtask/Cargo.toml` declares `[package] name = "xtask"`, dev-mode workflow only. Dependencies: `walkdir` (recursive mtime + file enumeration) and `toml` (parse candidate Cargo.tomls for the validation predicate). NO `cargo_metadata`, `serde`, `serde_json`, `clap` — the CLI surface is `build-guests` + `--check` + `--list`, hand-rolled on `std::env::args()`. Add a root-level `.cargo/config.toml` with `[alias] xtask = "run --quiet -p xtask --"` so `cargo xtask` resolves.
- Implement `cargo xtask build-guests`: discover guests via a **validated filesystem walk** over two tree-roots — `modules/core-modules/*/wit-guest/Cargo.toml` and `test-guests/*/Cargo.toml`. For each candidate manifest, parse the TOML and apply a per-tree validation predicate before treating it as a guest:
  - **Core-module wit-guest:** `[lib].crate-type` contains `"cdylib"` AND a `[workspace]` sentinel is present AND `[dependencies]` declares a parent path dep of shape `<name> = { path = ".." }`.
  - **Test-guest:** `[lib].crate-type` contains `"cdylib"` AND a `[workspace]` sentinel is present AND `[dependencies]` declares `wit-bindgen` directly.
  Candidates that fail validation are surfaced as `SKIP: <path> (reason)` on stderr and excluded from the build set; they do not fail the run. (Rationale: `cargo_metadata` is infeasible because each guest declares a `[workspace]` sentinel and is invisible to the parent workspace's metadata.) For each validated guest, invoke `cargo build --target wasm32-unknown-unknown --release --quiet` with the guest's manifest path (or equivalently `cd` into the guest directory), then run `wasm-tools component new <core_wasm_path> -o <component_output_path>` where the output path follows the per-tree convention (`modules/core-modules/<dir>/<dir>.wasm` for core-modules, `test-guests/<crate-name>.component.wasm` for test-guests). **Important: each guest is its own isolated workspace, so its build artifacts land in the guest's own `target/` directory** (e.g., `modules/core-modules/<dir>/wit-guest/target/wasm32-unknown-unknown/release/<lib_name>.wasm`), NOT the parent workspace's `target/`. Mirror the bash scripts' path resolution exactly.
- Implement `cargo xtask build-guests --check`: enumerate the same guest set; for each, compute the newest mtime across (a) the guest's own `src/` + `Cargo.toml`, (b) `wit/**/*.wit`, (c) the 4 shared crates `crates/slicer-macros`, `crates/slicer-sdk`, `crates/slicer-ir`, `crates/slicer-schema` (their `src/` + `Cargo.toml`). Compare against the existing component artifact's mtime. If newer source ⇒ exit 1 with `STALE: <crate-name>` line on stdout. If all up-to-date ⇒ exit 0 silently.
- Implement `cargo xtask build-guests --list`: emit one line per discovered guest (crate name, manifest path, expected artifact path). No build action. Used by AC-2 verification.
- Update `.github/workflows/ci.yml` lines that invoke the bash scripts → `cargo xtask build-guests --check`. If the scripts are not currently invoked from CI, no change is needed (the existing `cargo test -p slicer-runtime` line in CI does NOT trigger guest builds — guest rebuilds remain a developer concern per `CLAUDE.md`'s pre-existing rules).
- Delete `modules/core-modules/build-core-modules.sh`.
- Delete `test-guests/build-test-guests.sh`.
- Rewrite `CLAUDE.md` §"Guest WASM Staleness (MUST follow)": replace both `--check` invocations with one `cargo xtask build-guests --check`. Keep the prohibition-against-deflection language; only the command name changes.
- Rewrite `docs/05_module_sdk.md`'s "Developer CLI" / build-flow section: document the module-author two-step (`cargo build --target wasm32-unknown-unknown --release` followed by `wasm-tools component new target/wasm32-unknown-unknown/release/<name_underscored>.wasm -o target/slicer/<name_kebab>.wasm`); explain that `pnp_cli` deliberately has no `build` verb (cargo is the canonical build tool); add a sidebar pointing workspace contributors at `cargo xtask build-guests` for the core-module rebuild flow.
- Append a TASK-214 closure entry to `docs/07_implementation_status.md` (via worker dispatch).

## Out of Scope

- Any change to WASM module ABI, manifest TOML schema, IR schemas, the `wasm-tools component new` invocation shape, or `manifest::is_placeholder_wasm`.
- Any change to host-runtime code (Packet 1 territory).
- A `cargo-generate` template for external module authors (intentionally deferred per plan Q8 of the source planning document).
- A `pnp_cli` verb for building guests (intentionally deferred per plan Q5 — agentic-hook cost considerations).
- Per-guest parallelism / caching beyond what `cargo build -p <name>` already provides.
- Cross-workspace publishing of the xtask binary.

## Authoritative Docs

- `CLAUDE.md` (~150 lines) — load directly.
- `docs/05_module_sdk.md` (~700 lines) — delegate a SUMMARY of the "Developer CLI" section before editing; the rest of the file is out of scope.
- `modules/core-modules/build-core-modules.sh` (~220 lines) — load directly; reference for the mechanics.
- `test-guests/build-test-guests.sh` (~200 lines) — load directly; reference for the second guest tree.
- `.github/workflows/ci.yml` (~60 lines) — load directly.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-7` from `packet.spec.md`. Refinements:
  - The xtask's freshness rule mirrors the bash scripts' rule exactly: a guest is stale if any of (its own source, its Cargo.toml, any `wit/**/*.wit`, any `src/` or `Cargo.toml` under the 4 shared crates `slicer-macros`, `slicer-sdk`, `slicer-ir`, `slicer-schema`) is newer than the component artifact at the expected output path. `slicer-core` is intentionally NOT tracked (only ~6 of ~20 guests depend on it; global tracking forces spurious rebuilds for the rest). `slicer-helpers` is host-only and not a guest dep — also not tracked.
  - The xtask binary does NOT depend on `slicer-runtime`. Verify by `cargo build -p xtask` time (should be < 10s on a warm cache) and by `cargo tree -p xtask --no-default-features` not containing `slicer-runtime`.
- Negative cases: `AC-N1`, `AC-N2` from `packet.spec.md`.
- Cross-packet impact: depends on Packet 1 being implemented (binary name `pnp_cli` is used in the rewritten docs).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo build --workspace --release` | Workspace builds after xtask addition | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo clippy --workspace -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask build-guests` (clean target) | Full build path for both guest trees (AC-1) | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo xtask build-guests --check` (after AC-1) | Freshness check exits 0 when fresh (AC-1, AC-4) | FACT pass/fail |
| `cargo xtask build-guests --list` | Discovery enumeration (AC-2) | SNIPPETS — count compared to filesystem |
| `touch wit/world-layer.wit && cargo xtask build-guests --check` | Freshness detects WIT change (AC-3) | FACT exit-1 + STALE lines |
| `test ! -f modules/core-modules/build-core-modules.sh && test ! -f test-guests/build-test-guests.sh` | Bash retirement (AC-5) | FACT pass/fail |
| `grep -q 'cargo xtask build-guests' CLAUDE.md && grep -q 'cargo xtask build-guests' docs/05_module_sdk.md` | Doc rewrites (AC-6) | FACT pass/fail |
| `! grep -rln 'build-core-modules\.sh\|build-test-guests\.sh' CLAUDE.md docs/ .github/ .claude/ .agents/` | No stale script refs (AC-6) | FACT empty = pass |
| `grep -q 'cargo build --target wasm32-unknown-unknown' docs/05_module_sdk.md && grep -q 'wasm-tools component new' docs/05_module_sdk.md` | Two-step documented (AC-7) | FACT pass/fail |
| `! grep -rn 'MODULES\s*[:=]\s*\[\|GUESTS\s*[:=]\s*\[' xtask/` | No hardcoded module list (AC-N1) | FACT empty = pass |
| `! grep -E 'pnp_cli\s+build\b' docs/05_module_sdk.md` | No spurious build-verb recommendation (AC-N2) | FACT empty = pass |

The slice smoke command from Packet 1 (`pnp_cli slice …`) is NOT a verification command for this packet — `pnp_cli` is not modified here; only the build orchestration around its module dependencies changes.

## Step Completion Expectations

- Step 1 (xtask scaffolding) must produce a buildable binary before any subsequent step. Verify with `cargo build -p xtask` before progressing.
- Step 4 (delete bash scripts) must run AFTER the xtask is verified to build all guests successfully (step 3) — otherwise an unbuildable guest leaves the workspace without a recovery path.
- Step 5 (CI yml update) is a no-op if CI does not currently invoke the bash scripts (verify by grep before editing). The current `.github/workflows/ci.yml:47,49` invokes `cargo test -p slicer-host` and `cargo test -p slicer-cli && cargo test -p slicer-helpers` — neither of these involves the build scripts. After Packet 1, those lines reference `slicer-runtime` and `pnp-cli`. This packet's step 5 verifies the situation and adds a CI invocation of `cargo xtask build-guests --check` only if it makes sense (e.g., as a pre-build step before `cargo test -p pnp-cli` if any of pnp-cli's tests require fresh guests).
- The doc rewrite (step 6) MUST land in `docs/05_module_sdk.md` as one cohesive section — partial rewrites that leave a half-deleted `slicer build` reference are caught by AC-N2.

## Context Discipline Notes

- The two bash scripts (~420 lines total) are the largest read of this packet. The implementer loads each in full (necessary to understand the freshness logic exactly) but extracts the rules into Rust code without copying bash idioms — the Rust implementation uses `std::fs::metadata` + `walkdir` for source discovery and TOML parsing (`toml` crate) for the guest validation predicate; `cargo_metadata` is intentionally absent (guest `[workspace]` sentinels make it return zero guests). After step 4 deletes the scripts, they should not be re-read.
- Tempting curiosity reads to skip: any individual guest's `src/lib.rs` (the xtask doesn't need to read guest sources; it only invokes `cargo build -p <name>`); `crates/slicer-runtime/**` (out of scope for this packet); `modules/core-modules/<dir>/<dir>.toml` manifest files (the xtask doesn't read manifests; it builds artifacts).
- Heaviest dispatch: the post-AC-1 `cargo xtask build-guests` invocation against a clean target — builds 20 core-module guests + 12 test-guests. Required return format: FACT pass/fail; on failure, SNIPPETS ≤ 30 lines of the first failing guest's cargo error. Do not return the full build log.

## Changelog

- 2026-05-29: Corrected discovery mechanism from `cargo_metadata` to a validated filesystem walk over the two tree-roots. Guest Cargo.tomls declare `[workspace]` sentinels and are invisible to the parent workspace's metadata, so `cargo_metadata` returns zero guests. Per-tree validation: core-module wit-guests require `crate-type=["cdylib"]` + a `[workspace]` sentinel + a parent path dep (`{ path = ".." }`); test-guests require `crate-type=["cdylib"]` + a `[workspace]` sentinel + a direct `wit-bindgen` dependency. The "no hardcoded MODULES list" invariant (AC-N1) is satisfied identically. Also fixed AC-2's verification command, which had a `find -o` precedence bug that prevented it from counting the `test-guests/` tree.
