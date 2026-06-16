# Design: workspace-aware-guest-builder

## Controlling Code Paths

- Primary code path: `xtask/src/main.rs` (new) hosts a hand-rolled CLI dispatcher (`build-guests`, `build-guests --check`, `build-guests --list`). `xtask/src/build_guests.rs` (new) hosts the discovery + build + freshness logic. Discovery is a validated filesystem walk over two tree-roots: `modules/core-modules/*/wit-guest/Cargo.toml` and `test-guests/*/Cargo.toml`. Each candidate manifest is parsed (via the `toml` crate) and validated against a per-tree predicate (see Locked Assumptions). Per-guest build runs `cargo build --target wasm32-unknown-unknown --release --quiet` either by `cd`-ing into the guest directory or by passing `--manifest-path` — each guest is its own isolated workspace, so its compiled `.wasm` lands at `<guest-dir>/target/wasm32-unknown-unknown/release/<lib_name>.wasm`. Component post-processing invokes `Command::new("wasm-tools").args(["component", "new", &core_path, "-o", &out_path])`. Freshness (`--check`) computes max-mtime over `(guest_src, guest_cargo_toml, wit/**, slicer-{macros,sdk,ir,schema}/{src,Cargo.toml})` and compares to the component artifact mtime.
- Neighboring tests or fixtures: a small unit test in `xtask/src/build_guests.rs` (or a `tests/` directory) exercises the discovery filter against a synthesised `MetadataCommand` output OR against the live workspace metadata (the latter is simpler and survives any module list change). End-to-end verification is via the AC commands in `packet.spec.md` — the packet does not require a heavy test suite; the bash scripts didn't have one either.
- OrcaSlicer comparison surface: none — this packet has no OrcaSlicer parity work.

## Architecture Constraints

- The `xtask` crate must NOT depend on `slicer-runtime`. Driving reason: agentic hooks (Claude Code pre-tool-use hooks running `cargo xtask build-guests --check`) need cheap compile cost. Verify by `cargo tree -p xtask` not containing `slicer-runtime`, `wasmtime`, `pyo3`, `truck-stepio`, or `meshopt`. Discovery is filesystem-only (validated walk over `modules/core-modules/*/wit-guest/Cargo.toml` and `test-guests/*/Cargo.toml`); `cargo_metadata` is intentionally absent because guest Cargo.tomls declare `[workspace]` sentinels and are invisible to the parent workspace's metadata.
- The freshness rule mirrors the bash scripts exactly: tracked source surfaces are `wit/**/*.wit` + the 4 shared crates (`slicer-macros`, `slicer-sdk`, `slicer-ir`, `slicer-schema`) + per-guest `src/` + per-guest `Cargo.toml`. `slicer-core` and `slicer-helpers` are intentionally NOT tracked (per bash-script comments — `slicer-core` is depended on by only ~6 of ~20 guests, global tracking causes spurious rebuilds; `slicer-helpers` is host-only).
- The artifact-output convention is per-tree:
  - `modules/core-modules/<dir>/wit-guest/Cargo.toml` → component lands at `modules/core-modules/<dir>/<dir>.wasm` (the manifest loader's resolution path; matches existing bash output).
  - `test-guests/<crate-name>/Cargo.toml` → component lands at `test-guests/<crate-name>.component.wasm` (matches existing bash output).
- The xtask MUST run `wasm-tools component new` with the same flag shape as the existing scripts: `wasm-tools component new <core_wasm> -o <component_wasm>`. No additional flags. (This is the verified shape from `cli/slicer-cli/src/cmd_build.rs:131-142`.)
- The xtask must not silently drop guests on parse or build errors. A discovery filter mismatch (e.g., a guest crate that doesn't declare `[lib] crate-type = ["cdylib"]`) is surfaced as a `SKIP: <name> (reason)` line on stderr but does not fail the build. A build or `wasm-tools` failure for any guest fails the whole xtask invocation with the guest name and the first 20 lines of the underlying tool's error.
- The packet's change surface includes `crates/slicer-schema` (no — verify: this packet does NOT edit `slicer-schema`; that was Packet 1). Therefore the wasm-staleness snippet does NOT apply to this packet's change surface — the xtask is host-side workspace tooling, and the workspace bash-script retirement happens to the build mechanism itself, not to inputs that feed guest WASM contents. The xtask IS the rebuild tool, so by definition it does not require its own re-run after editing. (Validation: the implementer who edits `xtask/` does not need to re-run guest builds; the implementer who edits `wit/` or the 4 shared crates DOES — and that rule is preserved verbatim, just behind the new entry point.)

## Code Change Surface

- Selected approach: new `xtask/` crate following the cargo-xtask convention. Discovery is a validated filesystem walk over two tree-roots (no hardcoded list, no `cargo_metadata` — guest `[workspace]` sentinels make metadata-driven discovery infeasible). Per-guest build calls `cargo build` directly with the guest's manifest path (preserving cargo's incremental cache inside the guest's own `target/`). Component post-processing is a separate `wasm-tools` invocation per guest. `--check` mode reuses the discovery path and adds an mtime comparison.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **New**: `xtask/Cargo.toml`, `xtask/src/main.rs`, `xtask/src/build_guests.rs`, `.cargo/config.toml` workspace root (adds `[alias] xtask = "run -p xtask --"` so `cargo xtask …` resolves).
  - **Changed**: `.github/workflows/ci.yml` (conditionally — see Step 5 in implementation plan), `CLAUDE.md` §"Guest WASM Staleness (MUST follow)", `docs/05_module_sdk.md` Developer CLI / build section.
  - **Deleted**: `modules/core-modules/build-core-modules.sh`, `test-guests/build-test-guests.sh`.
- Rejected alternatives:
  - `pnp_cli build-guests` verb (plan Q5 alternative): rejected because compiling `pnp_cli` (and its wasmtime/pyo3/truck-stepio/meshopt deps) every time an agentic hook runs `--check` is too expensive.
  - Hardcoded module list inside the Rust xtask: rejected because that preserves the original drift hazard with no architectural improvement.
  - One xtask subcommand per tree (`build-core-modules`, `build-test-guests`): rejected because the bash scripts already share 80%+ of their code; the unified subcommand parameterised by `(tree_root, artifact_convention)` is the right shape.
  - Watch-mode (continuous rebuild on file changes): out of scope; `cargo build` already supports `--watch` indirectly via `cargo-watch`.

## Files in Scope (read + edit)

- `xtask/Cargo.toml` (new) — declare crate, deps on `walkdir` and `toml`. NO `cargo_metadata`, `serde`, `serde_json`, `clap`.
- `xtask/src/main.rs` (new) — CLI dispatcher.
- `xtask/src/build_guests.rs` (new) — discovery + build + freshness logic.
- `Cargo.toml` (workspace root) — add `xtask` to `members`.
- `.cargo/config.toml` (new or extended) — add `[alias] xtask = "run -p xtask --"`.
- `.github/workflows/ci.yml` — see Step 5 (may or may not require edits depending on whether CI currently invokes the scripts).
- `CLAUDE.md` — §"Guest WASM Staleness (MUST follow)" rewrite.
- `docs/05_module_sdk.md` — Developer CLI / build section rewrite.
- `modules/core-modules/build-core-modules.sh` — deleted.
- `test-guests/build-test-guests.sh` — deleted.

## Read-Only Context

- `modules/core-modules/build-core-modules.sh` (~220 lines) — load in full; reference for freshness rules, `wasm-tools` invocation, and component-output paths. The Rust xtask preserves the rules verbatim.
- `test-guests/build-test-guests.sh` (~200 lines) — load in full; verify the artifact-output convention for test-guests (per `test-guests/<crate-name>.component.wasm`).
- `Cargo.toml` (workspace root, ~110 lines post-Packet-1) — read `members` list to understand the workspace shape (for background context only; the xtask does not invoke `cargo_metadata`).
- `docs/05_module_sdk.md` — read §"Developer CLI" only (delegate SUMMARY) before editing.
- `CLAUDE.md` — read §"Guest WASM Staleness" only (≤ 50 lines).
- `cli/slicer-cli/src/cmd_build.rs:131-142` — this file is DELETED by Packet 1. Implementer reads it via `git show` against the pre-Packet-1 ref if uncertain about the `wasm-tools` invocation shape. (Better: read the still-extant `modules/core-modules/build-core-modules.sh:175` which has the same shape.)

## Out-of-Bounds Files

- `crates/slicer-runtime/**`, `crates/pnp-cli/**`, `crates/slicer-schema/**`, `crates/slicer-ir/**`, `crates/slicer-sdk/**`, `crates/slicer-macros/**` — not edited by this packet. Treat as fixed dependencies of guest crates.
- `modules/core-modules/<dir>/src/**`, `modules/core-modules/<dir>/wit-guest/src/**`, `test-guests/<dir>/src/**` — guest sources. The xtask invokes `cargo build` on these but does not read or modify them.
- `OrcaSlicerDocumented/**` — foreign tree; no parity.
- `target/`, `Cargo.lock` — never load.
- `.ralph/specs/_OLD/**` and any `status: implemented` packet — cross-packet mutation rule applies.

## Expected Sub-Agent Dispatches

- "Run `cargo build -p xtask`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: xtask crate compiles in isolation.
- "Run `cargo tree -p xtask --no-default-features`; return SNIPPETS ≤ 30 lines (truncate if longer); confirm `slicer-runtime`, `wasmtime`, `pyo3`, `truck-stepio`, `meshopt` are NOT in the output." — purpose: agentic-hook compile-cost invariant.
- "Run `cargo xtask build-guests` against a clean target; return FACT pass/fail; on failure, SNIPPETS ≤ 30 lines of the first failing guest's cargo or wasm-tools error." — purpose: AC-1 build path.
- "Run `cargo xtask build-guests --check` immediately after the previous; return FACT (exit 0 expected)." — purpose: AC-1 freshness post-build.
- "Run `touch wit/world-layer.wit && cargo xtask build-guests --check`; return FACT (exit 1 expected) + SNIPPETS of the STALE: lines." — purpose: AC-3 freshness detection.
- "Summarize `docs/05_module_sdk.md`'s 'Developer CLI' section; return SUMMARY ≤ 200 words and a count of distinct CLI invocation examples therein." — purpose: scope the rewrite before editing.
- "Append a TASK-214 closure entry to `docs/07_implementation_status.md`; see this packet's `requirements.md` for the entry text. Return FACT done." — purpose: backlog book-keeping.

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

## Context Cost Estimate

- Aggregate: `M` — 8 steps, all S or M.
- Largest single step: `M` (steps 1–3 are the Rust implementation work).
- Highest-risk dispatch: `cargo xtask build-guests` against a clean target (step 7 acceptance), which builds 20+12 guests. Required return format: FACT pass/fail; on failure SNIPPETS ≤ 30 lines of the first failing guest. The full build log can be ~thousands of lines and must NOT enter context.

## Open Questions

- `[FWD]` Step 5: whether `.github/workflows/ci.yml` should gain a `cargo xtask build-guests --check` step before `cargo test -p pnp-cli` (so test-guest staleness is caught at CI rather than at developer-test time). The CI today does NOT run any guest builds — the implementer decides whether this packet introduces that. Either choice satisfies AC-6 as phrased.
- `[FWD]` Step 1: whether to use `clap` for the xtask CLI or hand-roll. The surface is tiny (`build-guests`, `--check`, `--list`); hand-rolled is fewer deps but slightly more code. Implementer picks.
- `[FWD]` Step 6: whether the `docs/05_module_sdk.md` rewrite explicitly recommends a `cargo-component` mention as an alternative to the two-step. Today the project does NOT use `cargo-component`; the rewrite documents the manual two-step as canonical. Adding a "see also" for `cargo-component` is optional and at the implementer's discretion.
