# CLAUDE.md

`docs/` is the canonical source for architecture, IR schemas, WIT contracts, scheduler behavior, and the coordinate system. **Read the relevant doc before answering architecture questions or modifying contracts** — do not rely on summaries here. For the full doc index with one-line descriptions of each file, read @.claude/doc-index.md.

## Build & Test Commands

```bash
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings  # required before committing
cargo test -p slicer-runtime --test contract core_module_ir_access_contract_tdd   # narrow run; integration tests bucket into 5 binaries: unit|contract|executor|integration|e2e
cargo xtask build-guests                             # build all guest WASMs (core-modules + test-guests; needs wasm32 target)
cargo xtask dist                                     # build guests + pnp_cli (release), stage into target/dist/ (add --debug for debug binary)
cargo run --bin pnp_cli --release -- slice --input model.stl --output model.gcode
```

Benchmark commands and the HTML slicer report (`--report`) are rarely needed — read @.claude/aux-commands.md when you need them.

## Post-Merge Naming (Packet 69)

- `slicer-host` library → `slicer-runtime` (crate name)
- `slicer-cli` crate → deleted (verbs absorbed into `pnp_cli`)
- `slicer` / `slicer-host` binaries → `pnp_cli`

## Test Discipline

**Do not run `cargo test --workspace` by default.** The full suite is >1000 tests, ≥11 minutes. Default to the narrowest test that proves the change:
- A single test:        `cargo test -p <crate> --test <file> -- <test_name> --nocapture`
- One test file:        `cargo test -p <crate> --test <file>`
- One crate:            `cargo test -p <crate>`
- Type-check only:      `cargo check --workspace --all-targets`

**Always use `--all-targets` for check/clippy gates.** Plain `cargo check/clippy --workspace` does **not** compile test/bench targets — a change can leave test targets non-compiling while the gate stays green. The acceptance gate must build all targets.

`cargo test --workspace` is permitted **only** when:
1. The user explicitly asks for it, OR
2. A packet's acceptance ceremony / completion gate (`packet.spec.md` / `implementation-plan.md`) requires it for closure, AND every narrower verification command on that packet has already passed.

When a packet does require it, dispatch it to a sub-agent with a `FACT pass/fail` return — never absorb the full output. See `.claude/skills/swarm/SKILL.md` and `.claude/skills/spec-review/SKILL.md`.

### `cargo xtask test` — the gated entry point

`cargo xtask test [ARGS...]` runs `cargo xtask build-guests --check` first (rebuilding if any guest is stale), then execs `cargo test ARGS...` with output tee'd to `target/test-output.log`. It is the **enforced entry point** for any test run that touches guest WASM artifacts — i.e. the two `cargo test --workspace` cases above and any whole-suite / multi-crate run.

`cargo xtask test --summary [ARGS...]` — same pipeline but prints a compact LLM-friendly digest (one `test result:` line per test binary, failing-test detail, final `PASS`/`FAIL`, full-output path). **Prefer `--summary` for agent-driven runs.**

`cargo xtask test --summary-from <FILE>` (or `-` for `target/test-output.log`) — re-summarizes an existing log without re-running tests.

**Rule:** before running `cargo test --workspace` (or any broad test run after stashing working-tree changes to diagnose a regression), you MUST use `cargo xtask test --workspace`. This guarantees the guest-WASM freshness gate fires. A stale guest is your bug until `--check` proves otherwise (see "Guest WASM Staleness" below). Narrow runs stay on plain `cargo test`.

**Regression-diagnosis workflow (enforced):** when you stash working-tree changes to bisect / diagnose a regression (per `.claude/skills/diagnose/SKILL.md`), the test run after stashing MUST go through `cargo xtask test` so the freshness gate runs against the stashed (baseline) tree, not whatever guest artifacts happen to be on disk. Skipping the gate during a bisect silently attributes stale-guest failures to your stashed code.

### Test output must always tee to `target/test-output.log`

The Bash tool truncates long console output. **Every `cargo test` / `cargo nextest` invocation MUST redirect combined output to `target/test-output.log`:**

```bash
mkdir -p target && cargo test -p <crate> --test <file> 2>&1 | tee target/test-output.log
```

(PowerShell: `cargo test ... 2>&1 | Tee-Object -FilePath target/test-output.log`.)

When inspecting results, you MUST read the file — never re-run the tests to see more output:
- Summary lines: `Grep` for `^test result` in `target/test-output.log`.
- Failures: `Grep` for `FAILED|panicked at|---- .* stdout ----` with `-C 5`.
- Specific test: `Grep` for the test name, then `Read` the surrounding lines.

**Prohibited:** re-invoking `cargo test` because the previous run's stdout was truncated, claimed to "only show doc-tests", or "needs the full picture". The full picture is already on disk — read it. Capture findings before launching the next run (the log is overwritten each run).

## Coordinate System Hazard

**1 unit = 100 nm (10⁻⁴ mm)**, NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` / `mm_to_units()`. Full porting checklist in `docs/08_coordinate_system.md`.

## Config Key Naming Convention

All config key strings in Rust code (host-side and module-side) must use **snake_case** (underscores), never kebab-case (hyphens).

- Correct: `config.get("apply_to_all")`, `ConfigKey::from("fuzzy_skin.apply_to_all")`
- Wrong:   `config.get("apply-to-all")`, `ConfigKey::from("fuzzy_skin.apply-to-all")`

Module manifest TOML section headers already use snake_case. Runtime key strings must match.

## Guest WASM Staleness (MUST follow)

Guest `.wasm` artifacts under `modules/core-modules/*/` and `crates/slicer-wasm-host/test-guests/*.component.wasm` are **not** rebuilt by `cargo build` or `cargo test`. Stale guests fail typed instantiation at runtime and surface as test failures that look unrelated to your edits but are not. Test-guests all build into one shared target dir at `crates/slicer-wasm-host/test-guests/target/` (one `CARGO_TARGET_DIR`, not one `target/` per guest); per-guest `[workspace]` sentinels are retained.

**You MUST run the freshness check before attributing any guest, component, host-integration, or module-dispatch test failure to your changes, to "flaky tests", to "a separate workstream", or to "unrelated infrastructure":**

```bash
cargo xtask build-guests --check
```

If it reports `STALE:`, rebuild (drop `--check`) and re-run the failing test before drawing any conclusion.

**You MUST run `--check` (and rebuild if stale) after editing any of these paths** (build scripts treat them as guest-WASM inputs):

- `crates/slicer-schema/wit/**/*.wit` — invalidates every guest's bindgen (canonical single source; old top-level `wit/` deleted in packet 72)
- `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**` — universal guest deps baked into every guest `.wasm`
- `modules/core-modules/*/src/**` and `modules/core-modules/*/Cargo.toml` — `#[slicer_module]` impl bodies
- `modules/core-modules/*/wit-guest/**` — per-module guest shim
- `crates/slicer-wasm-host/test-guests/*/src/**` and `crates/slicer-wasm-host/test-guests/*/Cargo.toml` — test guest sources

**Prohibited claims unless `--check` was just run and returned clean:** "the wasm rebuild is a separate workstream", "this is unrelated to my changes", "the build scripts are out of scope for this packet", or any equivalent deflection. Treat a stale guest as your bug until `--check` proves otherwise.

## WIT/Type Changes Checklist

When modifying WIT types or interface definitions:
1. Search all `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for the affected type.
2. Verify type identity matches across component boundaries (e.g., `list<object-id>` in one file and `list<MeshObjectView>` in another causes linking failures).
3. Run `cargo build --tests` after WIT changes.
4. Edit the canonical source at `crates/slicer-schema/wit/` — both host (`bindgen! path:`) and guest macro (`include_str!`) read these files directly. There is no inline copy to keep in sync.

## Spec Packet Workflow

Implementation work is organized into spec packets under `.ralph/specs/<NN>_<slug>/`, each containing `packet.spec.md`, `requirements.md`, `design.md`, and `implementation-plan.md`. The active packet is the one whose `packet.spec.md` has `status: active` (grep for it). Packets are authored with `/spec-packet-generator`, gated with `/spec-review <packet> --preflight`, and executed with `/swarm <packet>`. Backpressure gates require `cargo build`, the packet's narrow verification commands, and `cargo clippy` to pass before closing; the full `cargo test --workspace` runs only at the packet-close acceptance ceremony (see Test Discipline above).

## OrcaSlicer Attribution Rules

Any time an agent ports or translates C++ code from OrcaSlicer into this codebase, it MUST prepend the standard porting header defined in `docs/ORCASLICER_ATTRIBUTION.md` to the top of the new file. This ensures AGPLv3 compliance and proper attribution.