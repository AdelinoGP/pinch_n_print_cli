# Implementation Plan: workspace-aware-guest-builder

## Execution Rules

- One atomic step at a time.
- Each step maps back to `TASK-214` (sole task ID).
- TDD-light: end-to-end ACs (1, 3, 4) drive verification. The xtask is a build tool, not a library; behaviour is verified by running it.
- Each step honors the context-discipline preamble. The fields below are the budget contract.

## Steps

### Step 1: Scaffold `xtask/` crate

- Task IDs:
  - `TASK-214`
- Objective: Create `xtask/` workspace member with `Cargo.toml`, `src/main.rs`, `src/build_guests.rs`. Add `xtask` to workspace `Cargo.toml` `members`. Add (or extend) `.cargo/config.toml` at workspace root with `[alias] xtask = "run -p xtask --"`. Verify `cargo xtask --help` returns clap usage (or hand-rolled equivalent).
- Precondition: Packet 1 `pnp-cli-unification` is `status: implemented`; workspace builds clean.
- Postcondition: `cargo build -p xtask` returns success; `cargo xtask --help` exits 0 (no subcommands implemented yet, just CLI skeleton).
- Files allowed to read:
  - `Cargo.toml` (workspace root) — current `members` list
- Files allowed to edit (≤ 3):
  - `xtask/Cargo.toml` (new)
  - `xtask/src/main.rs` (new — CLI skeleton)
  - `Cargo.toml` (workspace root) — add `xtask` to `members`
  - `.cargo/config.toml` (new or extended) — add alias
- Files explicitly out-of-bounds for this step:
  - all bash scripts (deletion is step 4)
  - all docs (rewrite is step 6)
- Expected sub-agent dispatches:
  - "Run `cargo build -p xtask`; return FACT pass/fail." — purpose: crate compiles.
  - "Run `cargo tree -p xtask --no-default-features`; return SNIPPETS ≤ 30 lines confirming no `slicer-runtime`, `wasmtime`, `pyo3`, `truck-stepio`, `meshopt` in the tree." — purpose: agentic-hook compile-cost invariant.
- Context cost: `S`
- Authoritative docs:
  - none — pure scaffolding
- Verification:
  - `cargo build -p xtask` — FACT pass
  - `cargo xtask --help` — FACT exit-0
  - `cargo tree -p xtask --no-default-features | grep -E 'slicer-runtime|wasmtime|pyo3'` — FACT empty
- Exit condition: `cargo xtask --help` works; crate has no heavy deps.

### Step 2: Implement guest discovery via `cargo_metadata`

- Task IDs:
  - `TASK-214`
- Objective: In `xtask/src/build_guests.rs`, implement a `discover_guests() -> Vec<GuestSpec>` function where `GuestSpec { crate_name: String, manifest_path: PathBuf, source_root: PathBuf, artifact_path: PathBuf, tree: GuestTree }`. Use `cargo_metadata::MetadataCommand::new().exec()`. Filter: workspace members whose manifest path starts with `<workspace>/modules/core-modules/` AND ends in `/wit-guest/Cargo.toml` get `tree: Core` and `artifact_path: modules/core-modules/<dir>/<dir>.wasm` (where `<dir>` is the parent of `wit-guest`). Workspace members whose manifest path starts with `<workspace>/test-guests/` AND ends in `Cargo.toml` (one level deep) get `tree: TestGuest` and `artifact_path: test-guests/<crate-name>.component.wasm`. Crates that have neither prefix are excluded silently. Add a `--list` subcommand to `xtask` that prints `discover_guests()` results one per line. Wire it via the existing CLI dispatcher.
- Precondition: Step 1 green.
- Postcondition: `cargo xtask build-guests --list` returns one line per discovered guest. Discovery count matches filesystem count.
- Files allowed to read:
  - `Cargo.toml` (workspace root) — `members` list
  - `modules/core-modules/layer-planner-default/wit-guest/Cargo.toml` — verify the conventional shape (one sample is sufficient; the bash script's MODULES array is the canonical list, but we are explicitly NOT consulting it — discovery is via filesystem + metadata only)
  - `test-guests/layer-infill-guest/Cargo.toml` — same, for the test-guest tree
- Files allowed to edit (≤ 3):
  - `xtask/src/build_guests.rs` — add `discover_guests` + `GuestSpec` + `GuestTree`
  - `xtask/src/main.rs` — wire `build-guests --list` subcommand
- Files explicitly out-of-bounds for this step:
  - All other guest crate `Cargo.toml`s — discovery is generic; only sample is needed for the convention check
  - The bash scripts (load them in step 3 for freshness logic; not for discovery)
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --list`; return SNIPPETS of stdout truncated to 40 lines (output should be ≤ 35 lines)." — purpose: discovery output sanity.
  - "Run `find modules/core-modules -mindepth 2 -maxdepth 2 -type d -name wit-guest -printf '%p\n' | wc -l` AND `find test-guests -mindepth 1 -maxdepth 1 -type d | wc -l`; return FACT with the two counts." — purpose: cross-check against filesystem (AC-2).
- Context cost: `S`–`M`
- Authoritative docs:
  - `cargo_metadata` crate docs — delegate SUMMARY if uncertain about the field surface.
- Verification:
  - `cargo xtask build-guests --list | wc -l` matches `find …` count — FACT diff returns empty
  - AC-2's pipe-suffixed command — FACT pass
- Exit condition: discovery output is exhaustive and matches filesystem.

### Step 3: Implement build + `wasm-tools component new` per guest

- Task IDs:
  - `TASK-214`
- Objective: In `xtask/src/build_guests.rs`, implement `build_one(spec: &GuestSpec) -> Result<(), Error>` which runs `cargo build --target wasm32-unknown-unknown --release -p <crate_name>` (forwarding stdout/stderr; abort on non-zero exit with the first 20 lines of stderr), then runs `wasm-tools component new <core_wasm_path> -o <spec.artifact_path>` (same error handling). Wire `cargo xtask build-guests` (no flag) to call `discover_guests()` then `build_one` for each spec serially. At xtask startup (in `main.rs`), verify `wasm-tools --version` returns success; if not, abort with `error: wasm-tools not found on PATH; install with 'cargo install wasm-tools'`.
- Precondition: Step 2 green; `wasm-tools` available on PATH.
- Postcondition: `cargo xtask build-guests` from a clean target produces all expected `.wasm` artifacts.
- Files allowed to read:
  - `modules/core-modules/build-core-modules.sh` (~220 lines) — load in full; reference for `wasm-tools` invocation and core-wasm path resolution (`target/wasm32-unknown-unknown/release/<lib_name_underscored>.wasm`)
  - `test-guests/build-test-guests.sh` (~200 lines) — load in full; cross-reference
- Files allowed to edit (≤ 3):
  - `xtask/src/build_guests.rs` — add `build_one` + wire `build-guests` dispatch
  - `xtask/src/main.rs` — add the wasm-tools startup check
- Files explicitly out-of-bounds for this step:
  - any guest's `src/`
  - any docs
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests` against a clean `target/`; return FACT pass/fail; on failure, SNIPPETS ≤ 30 lines of the first failing guest's cargo or wasm-tools error." — purpose: AC-1 full build.
  - "After the previous, verify each expected artifact exists: `for p in $(cargo xtask build-guests --list | awk '{print $NF}'); do test -f $p || echo MISSING:$p; done`; return FACT empty (success) or LOCATIONS of missing." — purpose: artifact-path correctness.
- Context cost: `M`
- Authoritative docs:
  - `modules/core-modules/build-core-modules.sh` and `test-guests/build-test-guests.sh` — these ARE the spec for what the Rust code must do.
- Verification:
  - `cargo xtask build-guests` — FACT pass
  - Every guest's expected artifact path exists — FACT pass
- Exit condition: full build path works for both trees; no missing artifacts.

### Step 4: Implement `--check` freshness mode

- Task IDs:
  - `TASK-214`
- Objective: In `xtask/src/build_guests.rs`, implement `is_stale(spec: &GuestSpec, workspace_root: &Path) -> bool` mirroring the bash freshness rule: stale iff any of (guest `src/**` mtime, guest `Cargo.toml` mtime, `wit/**/*.wit` mtime, `crates/slicer-{macros,sdk,ir,schema}/{src/**,Cargo.toml}` mtime) > `spec.artifact_path` mtime. Wire `cargo xtask build-guests --check` to call `discover_guests()` then `is_stale` for each; collect stale specs; if any: print one `STALE: <crate-name>` line per stale spec to stdout, exit 1. Otherwise exit 0 silently.
- Precondition: Step 3 green (artifacts exist).
- Postcondition: `--check` returns exit 0 immediately after `build-guests`; `--check` returns exit 1 with STALE lines after touching `wit/world-layer.wit`.
- Files allowed to read:
  - `modules/core-modules/build-core-modules.sh` (~lines 31-70) — the freshness logic to mirror
- Files allowed to edit (≤ 3):
  - `xtask/src/build_guests.rs` — add `is_stale` + wire `--check`
- Files explicitly out-of-bounds for this step:
  - any guest crate
  - any docs
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check` immediately after a successful `cargo xtask build-guests`; return FACT (exit 0 expected)." — purpose: AC-1 / AC-4 freshness post-build.
  - "Run `touch wit/world-layer.wit && cargo xtask build-guests --check`; return FACT (exit 1 expected) + SNIPPETS of the STALE lines truncated to 40 entries." — purpose: AC-3 freshness detection.
  - "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT (exit 0 expected from the second)." — purpose: AC-4 recovery.
- Context cost: `S`–`M`
- Authoritative docs:
  - `modules/core-modules/build-core-modules.sh` — freshness rule reference.
- Verification:
  - AC-3 verification command — FACT exit-1 + STALE lines
  - AC-4 verification command — FACT exit-0 second time
- Exit condition: freshness detection mirrors bash behaviour exactly.

### Step 5: Update CI yml (conditional)

- Task IDs:
  - `TASK-214`
- Objective: Inspect `.github/workflows/ci.yml`. If the file currently references `build-core-modules.sh` or `build-test-guests.sh`, replace the invocations with `cargo xtask build-guests --check`. If the file does NOT currently invoke the bash scripts (today's reality post-Packet-1), add a new step `cargo xtask build-guests --check` before the existing `cargo test -p pnp-cli` step (if a test-guest staleness gate is desired) OR leave the file unchanged. The implementer decides based on whether they want CI to catch test-guest staleness. Either way, AC-6's grep for `cargo xtask build-guests` against `.github/workflows/ci.yml` must pass — adding even one reference satisfies it.
- Precondition: Step 4 green.
- Postcondition: `.github/workflows/ci.yml` references `cargo xtask build-guests` somewhere AND does not reference either bash script.
- Files allowed to read:
  - `.github/workflows/ci.yml` (~60 lines) — load directly
- Files allowed to edit (≤ 3):
  - `.github/workflows/ci.yml`
- Files explicitly out-of-bounds for this step:
  - all docs (rewrite is step 6)
  - all bash scripts (deletion is step 7, but the verification confirms they're not invoked from CI before deletion)
- Expected sub-agent dispatches:
  - "Run `grep -n 'build-core-modules\|build-test-guests' .github/workflows/ci.yml`; return SNIPPETS of any matches." — purpose: determine pre-edit state.
- Context cost: `S`
- Authoritative docs:
  - none
- Verification:
  - `grep -q 'cargo xtask build-guests' .github/workflows/ci.yml` — FACT pass
  - `! grep -E 'build-core-modules\.sh|build-test-guests\.sh' .github/workflows/ci.yml` — FACT empty
- Exit condition: CI yml references xtask command; no bash-script references remain.

### Step 6: Rewrite `docs/05_module_sdk.md` Developer CLI section + update CLAUDE.md staleness block

- Task IDs:
  - `TASK-214`
- Objective: Rewrite the "Developer CLI" / build-flow section in `docs/05_module_sdk.md` to document:
  1. The module-author canonical build path is `cargo build --target wasm32-unknown-unknown --release` followed by `wasm-tools component new target/wasm32-unknown-unknown/release/<name_underscored>.wasm -o target/slicer/<name_kebab>.wasm`.
  2. `pnp_cli` deliberately has no `build` verb (cargo is the canonical build tool).
  3. Sidebar/note: workspace contributors rebuilding the core-module guest set should use `cargo xtask build-guests`; freshness can be verified with `cargo xtask build-guests --check`.
  Update `CLAUDE.md` §"Guest WASM Staleness (MUST follow)": replace both `--check` script invocations with one `cargo xtask build-guests --check`. Keep the prohibition-against-deflection language verbatim; only the command names change.
- Precondition: Step 5 green (the xtask is real and CI uses it, so docs can cite it).
- Postcondition: Doc-impact greps in `packet.spec.md` pass (AC-6, AC-7, AC-N2).
- Files allowed to read:
  - `docs/05_module_sdk.md` — delegate SUMMARY of "Developer CLI" section first (file is ~700 lines).
  - `CLAUDE.md` (~150 lines) — load directly; §"Guest WASM Staleness" is the target.
- Files allowed to edit (≤ 3):
  - `docs/05_module_sdk.md`
  - `CLAUDE.md`
- Files explicitly out-of-bounds for this step:
  - other docs (`docs/00`, `docs/13`, etc. — Packet 1 handled the binary-name sweep in those)
- Expected sub-agent dispatches:
  - "Summarize `docs/05_module_sdk.md`'s 'Developer CLI' section: list every CLI invocation example, the section's heading, and its line range; return SUMMARY ≤ 200 words." — purpose: scope the rewrite.
- Context cost: `S`
- Authoritative docs:
  - `docs/05_module_sdk.md` (the file being edited)
  - `CLAUDE.md` (the file being edited)
- Verification:
  - AC-6 grep — FACT pass
  - AC-7 grep — FACT pass
  - AC-N2 grep — FACT empty (no `pnp_cli build` recommendation)
  - Doc Impact Statement greps in `packet.spec.md` — FACT pass each
- Exit condition: docs reflect the post-Packet-1, post-xtask reality.

### Step 7: Delete both bash scripts

- Task IDs:
  - `TASK-214`
- Objective: Delete `modules/core-modules/build-core-modules.sh` and `test-guests/build-test-guests.sh`. Verify no remaining references in workspace.
- Precondition: Step 6 green (all docs / CI now point at the xtask).
- Postcondition: AC-5 holds; no `.sh` script references remain in living documentation.
- Files allowed to read:
  - none (deletion only)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/build-core-modules.sh` — deleted
  - `test-guests/build-test-guests.sh` — deleted
- Files explicitly out-of-bounds for this step:
  - everything else
- Expected sub-agent dispatches:
  - "Run `grep -rln 'build-core-modules\.sh\|build-test-guests\.sh' CLAUDE.md docs/ .github/ .claude/ .agents/ 2>/dev/null`; return LOCATIONS of any remaining references (expected: empty)." — purpose: AC-6 second clause.
- Context cost: `S`
- Authoritative docs:
  - none
- Verification:
  - `test ! -f modules/core-modules/build-core-modules.sh && test ! -f test-guests/build-test-guests.sh` — FACT pass
  - `! grep -rln 'build-core-modules\.sh\|build-test-guests\.sh' CLAUDE.md docs/ .github/ .claude/ .agents/ 2>/dev/null` — FACT empty
- Exit condition: scripts deleted; no stale references.

### Step 8: Packet Completion Gate

- Task IDs:
  - `TASK-214`
- Objective: Run the full acceptance ceremony; confirm every AC verification returns FACT pass.
- Precondition: Steps 1–7 green.
- Postcondition: All AC and AC-N criteria green; packet ready to flip to `status: implemented`. `TASK-214` appended to `docs/07_implementation_status.md`.
- Files allowed to read:
  - `packet.spec.md` — re-read AC list
- Files allowed to edit (≤ 3):
  - `packet.spec.md` — flip `status: draft` → `status: implemented`
- Files explicitly out-of-bounds for this step:
  - everything else
- Expected sub-agent dispatches:
  - "Run each pipe-suffixed acceptance verification command from `packet.spec.md`; return FACT line per AC; on first failure SNIPPETS ≤ 20 lines." — purpose: ceremony.
  - "Append a TASK-214 closure entry to `docs/07_implementation_status.md`: 'TASK-214 Replace build-core-modules.sh and build-test-guests.sh with cargo xtask build-guests driven by cargo metadata; rewrite docs/05_module_sdk.md build-flow section to document the cargo + wasm-tools two-step; collapse CLAUDE.md guest-staleness check to a single command. **Closed YYYY-MM-DD via packet 70_workspace-aware-guest-builder.**'; return FACT done." — purpose: backlog book-keeping.
- Context cost: `S`
- Authoritative docs:
  - none
- Verification:
  - `cargo build --workspace --release` — FACT pass
  - `cargo clippy --workspace -- -D warnings` — FACT pass
  - `cargo xtask build-guests && cargo xtask build-guests --check` — FACT pass
  - Every AC- and AC-N command — FACT pass each
- Exit condition: all green; status flipped; TASK-214 in docs/07.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Crate scaffolding |
| Step 2 | S–M | Discovery via cargo_metadata |
| Step 3 | M | Build + wasm-tools per guest |
| Step 4 | S–M | Freshness check mirroring bash rule |
| Step 5 | S | CI yml update |
| Step 6 | S | Doc rewrites |
| Step 7 | S | Script deletions |
| Step 8 | S | Ceremony |

Aggregate: `M`. No `L` step.

## Packet Completion Gate

- All steps complete.
- Every step exit condition met.
- Every AC- and AC-N command in `packet.spec.md` dispatched and returned FACT pass.
- `docs/07_implementation_status.md` updated with `TASK-214` closure (via worker dispatch).
- `packet.spec.md` ready to flip `status: draft` → `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-7 and AC-N1, AC-N2).
- Confirm the 3 gate commands listed in `packet.spec.md` §Verification are green.
- Confirm `cargo build --workspace --release && cargo clippy --workspace -- -D warnings` returns FACT pass.
- Record peak implementer context usage; if it exceeded 70%, log as a packet-authoring lesson.
- Flip `status: draft` → `status: implemented` in `packet.spec.md` frontmatter.
