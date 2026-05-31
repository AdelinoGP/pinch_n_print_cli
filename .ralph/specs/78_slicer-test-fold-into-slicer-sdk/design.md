# Design — Packet 78

## Controlling Code Paths

The fold is structural rather than behavioral — the four source files in `crates/slicer-test/src/` carry the same code after the move; only their location and import paths change. The two design decisions that matter are (a) the gating posture of the new `test_prelude` module and (b) the `pnp_cli module new` scaffold's `[dev-dependencies]` line shape.

- **`crates/slicer-sdk/src/lib.rs`** — gains one line: `#[cfg(any(test, feature = "test"))] pub mod test_prelude;`. The existing `#[cfg(any(test, feature = "test"))] pub mod test_support;` from packet 77 stays; this packet extends `test_support/mod.rs` to declare its four newly-moved submodules.
- **`crates/slicer-sdk/src/test_support/mod.rs`** — gains four `pub mod <name>;` lines (mock_host, capture, fixtures, assert_paths) inside the existing feature-gated module. The four hook functions from packet 77 (`reset_global_state`, `install_panic_handler`, `mock_host_setup`, `mock_host_teardown`) stay byte-identical.
- **`crates/slicer-sdk/src/test_prelude.rs`** (new) — whole-module-gated at line 1 via `#![cfg(any(test, feature = "test"))]`. Body is a flat list of `pub use crate::test_support::{...}` lines covering every documented test helper. Approximate shape:

```rust
#![cfg(any(test, feature = "test"))]

pub use crate::test_support::mock_host::MockHost;
pub use crate::test_support::capture::{InfillOutputCapture, PerimeterOutputCapture, SupportOutputCapture};
pub use crate::test_support::fixtures::{ConfigViewBuilder, SliceRegionViewBuilder, PerimeterRegionViewBuilder, square_polygon, rect_path};
pub use crate::test_support::assert_paths::{assert_paths_planar, assert_max_segment_length, assert_extrusion_width_range, assert_paths_inside_polygon, assert_no_path_intersections};
```

- **`crates/pnp-cli/src/module_new.rs:188-207`** — `generate_cargo_toml` returns a `String`. The template substring containing `slicer-test = { path = "../../crates/slicer-test" }` (line 204) is replaced with `slicer-sdk = { path = "../../crates/slicer-sdk", features = ["test"] }`. The earlier `[dependencies] slicer-sdk = { path = ... }` line (line 201) stays unchanged (no `features`). Cargo's feature unification means production `cargo build` (which walks only `[dependencies]`) sees no `test` feature; `cargo test` (which walks both) sees the union and activates `test`.
- **`Cargo.toml:10`** (workspace root) — the line `"crates/slicer-test",` is deleted from the `members` array. Cargo regenerates `Cargo.lock` on the next build; commit that diff in the same commit as the member-list edit.
- **`modules/core-modules/{arachne-perimeters,rectilinear-infill}/Cargo.toml`** — each gains `slicer-sdk = { path = "../../../crates/slicer-sdk", features = ["test"] }` in `[dev-dependencies]`. The existing runtime `[dependencies] slicer-sdk` line stays unchanged.
- **`modules/core-modules/{arachne-perimeters,rectilinear-infill}/tests/*.rs`** — each file gains `use slicer_sdk::test_prelude::*;` near the top and replaces the bodies of `make_*` helpers with builder chains.
- **`docs/05_module_sdk.md:445-624`** — section heading rename + import line replacements. Body of code-examples gets the new `use` lines and the actual MockHost API from packet 77.
- **`docs/00_project_overview.md:122-156`** — one row removed from the crate table, one line removed from the directory tree.
- **`CLAUDE.md`** — search-and-update any `slicer-test` references (the file mentions the crate in at least two places per current recon).

## Architecture Constraints

- **Whole-module feature gate, not per-item.** `test_prelude.rs` MUST start with `#![cfg(any(test, feature = "test"))]` as line 1 (or line 2 after a doc comment). This makes the prelude either fully present or fully absent — never partial. Per-item gates inside an existing prelude (e.g., adding `#[cfg(...)] pub use ...` lines to `prelude.rs`) is explicitly rejected per the grilling decision because it makes `slicer_sdk::prelude`'s contents build-config-dependent, which breaks IDE jump-to-definition and `cargo doc`'s output.
- **Production guest wasm must NOT pull in `test_support`.** AC-4 enforces this via `cargo check --target wasm32-unknown-unknown` for both exemplar modules, plus a `cargo tree --target wasm32-unknown-unknown` assertion that the `test` feature is not activated. The mechanism: production `[dependencies] slicer-sdk = { path }` has no `features`; only `[dev-dependencies] slicer-sdk = { path, features = ["test"] }` does. Cargo activates `test` only when both dep edges are present in the build graph (i.e., during `cargo test`).
- **AC-N1's `nm`-based check is defense-in-depth.** It catches the case where someone forgets the `#[cfg]` gate on a sub-item of `test_support`. The four production-code `pub fn` hooks added in packet 77 are already inside the gated module, so they shouldn't leak — but the gate must be on the module, not just on individual functions.
- **Migration discipline: builders must preserve every original invariant.** `make_config(density: f64, angle: f64, speed: f64, line_width: f64)` produces a `ConfigView` with exactly those four keys at those values. The replacement `ConfigViewBuilder::new().float("infill_density", density).float("infill_angle", angle).float("infill_speed", speed).float("line_width", line_width).build()` produces an equivalent `ConfigView`. The implementer MUST verify field key names by reading the call sites (e.g., `gyroid-infill/src/lib.rs:14-17` uses `infill_density`, not `density` — that's an exact-match requirement for tests to pass).
- **Test file renames must update bucket aggregator `mod` statements.** `crates/slicer-sdk/tests/` may or may not have a `main.rs` aggregator (recon needed at step time). If it does, every moved test file's `mod` declaration line in `main.rs` must be added.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

Primary edits (≤ 3 conceptual surfaces; the count of files is high because each conceptual change touches multiple files):

1. **The fold** — moving 4 source files + 9 test files from `crates/slicer-test/` to `crates/slicer-sdk/`, adding `test_prelude.rs`, registering the new submodule and prelude in `lib.rs`. Net effect: `slicer-sdk` gains ~1300 LoC under feature gate; `slicer-test` loses everything; workspace member count -1.
2. **The scaffold rewrite** — `crates/pnp-cli/src/module_new.rs:188-207` template change + assertion updates at `crates/pnp-cli/src/module_new.rs:545` and `crates/pnp-cli/tests/module_new_tdd.rs:36`.
3. **The exemplar migrations** — `modules/core-modules/{arachne-perimeters,rectilinear-infill}/Cargo.toml` + test files. Per-file changes are tiny (one `use` line, one builder chain per `make_*` body); the discipline is preserving exact field names and parameter ordering.

Secondary edits (mechanical follow-on):

4. `docs/05_module_sdk.md:445-624` — section heading rename + import line replacements.
5. `docs/00_project_overview.md:122-156` — crate-inventory line removals.
6. Project root `CLAUDE.md` — search-and-update.

## Files in Scope (read+edit)

Edit-allowed:
- `crates/slicer-sdk/src/lib.rs`
- `crates/slicer-sdk/src/test_support/mod.rs`
- `crates/slicer-sdk/src/test_support/mock_host.rs` (moved; possibly edited to fix `use` paths)
- `crates/slicer-sdk/src/test_support/capture.rs` (moved)
- `crates/slicer-sdk/src/test_support/fixtures.rs` (moved; possibly edited if it imported from sibling `slicer-test` modules with `super::` paths)
- `crates/slicer-sdk/src/test_support/assert_paths.rs` (moved)
- `crates/slicer-sdk/src/test_prelude.rs` (new)
- `crates/slicer-sdk/tests/test_support_*_tdd.rs` (9 files: 5 from slicer-test + 4 from packet 77)
- `crates/slicer-sdk/tests/smoke.rs` (absorbed addition)
- `crates/slicer-sdk/tests/main.rs` (if exists — aggregator updates)
- `Cargo.toml` (workspace root)
- `crates/pnp-cli/src/module_new.rs` (lines 188-207 + 545)
- `crates/pnp-cli/tests/module_new_tdd.rs` (line 36)
- `modules/core-modules/arachne-perimeters/Cargo.toml`
- `modules/core-modules/arachne-perimeters/tests/*.rs`
- `modules/core-modules/rectilinear-infill/Cargo.toml`
- `modules/core-modules/rectilinear-infill/tests/*.rs`
- `docs/05_module_sdk.md` (only lines 445-624)
- `docs/00_project_overview.md` (only lines 122-156)
- `CLAUDE.md` (project root) — search-and-update

To-be-deleted:
- `crates/slicer-test/` (entire directory)

## Read-Only Context

- `crates/slicer-sdk/src/lib.rs` — confirm post-packet-77 state (with `pub mod test_support;`).
- `crates/slicer-sdk/src/prelude.rs` — to ensure no test items are added (AC-3's negative grep depends on this).
- `crates/slicer-sdk/src/host.rs` — unchanged; do not read unless tracing an unexpected build error.
- `crates/slicer-test/src/lib.rs` — confirm current `pub mod` declarations before deleting.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` — quote the `## Decision` paragraph in the rewritten `docs/05` section.
- `modules/core-modules/gyroid-infill/Cargo.toml` — read as a baseline for the dev-dep line shape (recon showed it currently has `[dev-dependencies]` empty; the addition is a one-line edit).
- `modules/core-modules/arachne-perimeters/tests/*.rs` — pre-migration baseline. Read each file's `make_*` helper bodies AND ALL call sites of each helper before rewriting — a builder chain that omits a setter the original constructor populated is a silent assertion-weakening bug.
- `modules/core-modules/rectilinear-infill/tests/*.rs` — same.
- `crates/slicer-test/tests/smoke.rs` — confirm what is being absorbed into `crates/slicer-sdk/tests/smoke.rs`.

## Out-of-Bounds Files

- All other core-modules' source and tests (packet 79).
- `crates/slicer-runtime/**` (packet 80 touches a small subset; this packet touches none).
- `crates/slicer-ir/**`, `crates/slicer-core/**`, `crates/slicer-schema/**`, `crates/slicer-helpers/**`, `xtask/**`.
- `crates/pnp-cli/src/**` except `module_new.rs` lines 188-207 and 545.
- `OrcaSlicerDocumented/**` — never load.
- `target/`, lockfiles (Cargo.lock will regenerate — accept the diff, don't pre-edit), all `*.wasm` artifacts.
- `crates/slicer-runtime/test-guests/**`.

## Expected Sub-Agent Dispatches

1. **Pre-fold confirm: list every file in `crates/slicer-test/`** — `Question: enumerate all files under crates/slicer-test/ (src/, tests/, Cargo.toml). Scope: that directory. Return: LOCATIONS (file + LoC count, ≤ 20 entries).`
2. **`crates/slicer-sdk/tests/` aggregator check** — `Question: does crates/slicer-sdk/tests/ contain a main.rs aggregator file, and if so what test modules does it declare? Scope: that directory. Return: FACT: yes/no, with file list and any \`mod <name>;\` declarations if present.`
3. **Pre-migration field-name extraction (arachne-perimeters)** — `Question: for each \`make_*\` helper in modules/core-modules/arachne-perimeters/tests/*.rs, list (a) the helper's parameter names and types, (b) the config keys / region fields its body populates. Scope: that directory, only the helper definitions. Return: SUMMARY (≤ 200 words).`
4. **Pre-migration field-name extraction (rectilinear-infill)** — same as 3 for rectilinear-infill.
5. **Workspace member count recheck** — `Question: how many entries does Cargo.toml's [workspace] members array currently contain? Scope: workspace Cargo.toml. Return: FACT: <number>.` Run before AC-1 verification.
6. **Wasm target check** — `Question: does cargo check --target wasm32-unknown-unknown -p arachne-perimeters pass after the migration? Scope: workspace. Return: FACT: clean / first error.`
7. **Cargo tree feature check** — `Question: does cargo tree --target wasm32-unknown-unknown -p arachne-perimeters reference slicer-sdk WITHOUT the test feature? Scope: workspace. Return: FACT: clean / unexpected feature activation.`
8. **AC-N1 nm scan** — `Question: after cargo build --workspace --release, do any .rlib files in target/release/ contain symbols matching slicer_sdk::test_support? Scope: target/release/. Return: FACT: clean / list of offending symbols.`
9. **Doc grep gates** — `Question: do docs/05_module_sdk.md and docs/00_project_overview.md still reference 'slicer-test'? Scope: those two files. Return: FACT: clean / line numbers of remaining references.`
10. **Closure clippy** — `Question: does cargo clippy --workspace --all-targets -- -D warnings pass? Scope: workspace. Return: FACT: clean / first violation with file:line.`
11. **Closure test sweep** — `Question: does cargo test -p slicer-sdk -p arachne-perimeters -p rectilinear-infill -p pnp-cli --test module_new_tdd pass? Scope: workspace. Return: FACT: counts pass/fail per package.`

## Data and Contract Notes

- **`slicer-sdk` dev-dep behavior under Cargo feature unification**: when `modules/core-modules/X/Cargo.toml` has both `[dependencies] slicer-sdk = { path }` (no features) and `[dev-dependencies] slicer-sdk = { path, features = ["test"] }`, Cargo's build graph treats these as one node. For `cargo build` (production target), only `[dependencies]` is active → `test` feature off. For `cargo test`, both edges are active → features unified → `test` feature on. This is the documented behavior; AC-4's `cargo tree --target wasm32-unknown-unknown` invocation walks only the production dep graph (no `[dev-dependencies]`), so the absence of `feature="test"` in its output is the load-bearing check.
- **`#[cfg(any(test, feature = "test"))]` vs `#[cfg(feature = "test")]`**: the former allows `cargo test -p slicer-sdk` (which adds the `test` cfg flag) to compile test_support even without the explicit feature; the latter would require `cargo test -p slicer-sdk --features test`. The `test` predicate keeps slicer-sdk's own tests ergonomic.
- **Test file renames force module_new_tdd assertion updates**: the existing assertion at `crates/pnp-cli/tests/module_new_tdd.rs:36` asserts `cargo.contains("slicer-test")`. After the rewrite it must assert `cargo.contains("slicer-sdk")` with `features = [\"test\"]`. Similarly at `module_new.rs:545`.
- **`Cargo.lock` regeneration is expected**: the workspace-member removal causes a fresh resolver pass. Lockfile diff is mechanical; commit it in the same commit as the `Cargo.toml` edit to avoid confusing subsequent step diffs.

## Locked Assumptions and Invariants

- **Invariant A**: `crates/slicer-sdk/src/test_support/mod.rs` retains the four hook functions from packet 77 (`reset_global_state`, `install_panic_handler`, `mock_host_setup`, `mock_host_teardown`) byte-identical. This packet's edit to that file is purely additive — `pub mod mock_host;` etc. The macro at `crates/slicer-macros/src/lib.rs:2705-2756` is NOT touched in this packet; its expansion remains `::slicer_sdk::test_support::*` (set in packet 77).
- **Invariant B**: `slicer_sdk::prelude` (the production prelude) does NOT re-export any test helpers. AC-3's verification includes the negative grep `! grep ... crates/slicer-sdk/src/prelude.rs` confirming this.
- **Invariant C**: The exemplar migrations preserve every test's original assertion semantics. A test that previously asserted `assert!((module.density() - 0.2).abs() < 0.001)` continues to make that exact assertion; only the fixture-construction path changes.
- **Invariant D**: The `pnp_cli module new` scaffold's `[dependencies]` line for `slicer-sdk` has NO `features` field. Only the `[dev-dependencies]` line does.

## Risks and Tradeoffs

- **Risk: `Cargo.lock` regeneration produces noisy diffs on unrelated dependencies.** Mitigation: commit `Cargo.lock` in the same commit as the workspace-member edit; subsequent commits will only show domain-relevant diffs. If the lockfile change includes upstream package versions, audit the diff briefly — but accept it as mechanical.
- **Risk: `cargo check --target wasm32-unknown-unknown` may not be a runnable verification command in CI if the wasm32 target isn't installed.** Mitigation: AC-4's command starts with `rustup target list --installed | grep -q wasm32-unknown-unknown` to skip cleanly when the target is missing. CI must install the target before running the gate (the project already builds guests via `cargo xtask build-guests`, which requires the target — so it should always be present).
- **Risk: deleting `crates/slicer-test` between AC-1 and other ACs that grep for `slicer-test` references creates a brief window where the workspace doesn't compile.** Mitigation: per the `Step Completion Expectations` ordering, ALL moves and dev-dep updates land before the directory deletion. Step 4's exit condition includes `cargo check --workspace --all-targets`; if that's not green, deletion is held.
- **Risk: an exemplar migration introduces a subtle bug because the builder defaults differ from the original constructor.** Mitigation: dispatch 3 + 4 extract the exact config keys / region field values the original `make_*` helpers populate, before rewriting; the implementer maps each field to an explicit setter call rather than relying on `ConfigViewBuilder::new().build()` defaults.
- **Tradeoff: keeping `make_*` helper shells (just with builder bodies) vs inlining at call sites.** Accepted: keep the shells when the original used a 4+ parameter constructor — readability wins. Inline when the helper was zero-parameter and only used once.

## Context Cost Estimate

- **Aggregate**: M-L. The file moves are mechanical but numerous; the exemplar migrations require careful invariant preservation.
- **Largest single step**: Step 9 (exemplar migrations) — M. The migration logic is per-helper; each helper is small but the count adds up (6 helpers × 2 modules = 12 builder chains to author).
- **Highest-risk dispatch**: dispatch 3 + 4 (pre-migration field-name extraction). A missed field is a silent assertion failure later.

## Open Questions

None. Every design decision was resolved during the grilling session before generation, including:
- Whole-module gate on `test_prelude` (not per-item gates).
- Exemplar choice: `arachne-perimeters` + `rectilinear-infill` (diverse) over `gyroid-infill` + `lightning-infill` (small/safe).
- Production prelude stays test-free.
- Cargo dev-dep feature-flag pattern confirmed correct (documented cargo behavior; AC-4's wasm target check is the safety net if cargo's behavior were ever wrong).
