# Design: 165_cli-binary-locator-extraction

## Controlling Code Paths

- Primary code path: `slicer_test_support::pnp_cli_bin` (new) → profile-inference (`current_exe().parent().parent()` sibling lookup) → `newest_source_mtime` scan → `staleness_reason` decision → return path or panic. Consumers: `crates/slicer-runtime/tests/common/slicer_cache.rs::run_pnp_cli_uncached` (site 1), `crates/slicer-runtime/benches/gate_evidence.rs` (site 2, DEV-026 evidence producer), `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs::run_dag` etc. (site 3).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` (162's regression tests over `staleness_reason`, reached via `common::slicer_cache`'s re-export — and the only file that consumes a re-exported *locator* symbol); the remaining `slicer-runtime` e2e/integration files calling `common::slicer_cache::{cached_run, run_pnp_cli_uncached, expect_outcome, ...}` — untouched by design. The population is single-digit, not the "~30" earlier revisions of this packet claimed; re-derive with `rg -l 'slicer_cache' crates/slicer-runtime/tests/` rather than trusting any number written here.
- OrcaSlicer comparison: none — no parity content; the `orca-delegation` snippet deliberately does not apply. This packet moves host-side test plumbing between crates.

## Architecture Constraints

- **The wasm-staleness snippet does not apply.** No file in the change surface is a guest-WASM input (`CLAUDE.md` §"Guest WASM Staleness" lists them): the new crate is host-side, dev-dep-only, and never linked into any guest or production target. The coord-system snippet likewise does not apply (no geometry).
- ADR-0004 boundary: guest-side test support lives in `slicer-sdk`; the new crate is its host-side counterpart and must never be depended on by a guest crate, `slicer-sdk`, or any `[dependencies]` (non-dev) section. The ADR authored by this packet records this.
- `xtask` stays bin-only. `slicer_test_support::staleness_reason` remains a documented **mirror** of `is_stale` (`xtask/src/build_guests.rs`) — the crate's rustdoc must pin that sibling relationship, carrying forward the pin 162 placed in `slicer_cache.rs`.
- The freshness gate's loudness contract (162): stale ⇒ panic whose message contains `pnp_cli`, `stale`, the resolved path, and a remedy; absent ⇒ panic; no release/debug fallback probing, ever.

## Code Change Surface

### Selected approach — new std-only crate `crates/slicer-test-support` (the ADR's decision)

The locator needs **zero dependencies** (std `fs`/`path`/`time`/`env` only). Weighed against the tree:

- **(a) `pnp-cli` lib behind a `test-support` feature — rejected.** Cargo permits the dev-dep cycle (`slicer-runtime` dev→ `pnp-cli` → `slicer-runtime`), but three costs, all verified against `crates/pnp-cli/Cargo.toml`: (1) the `pnp_cli` lib target's `[dependencies]` are non-optional — `slicer-runtime`, `slicer-scheduler`, `clap`, `png`, `toml`, … — so every `cargo test -p slicer-runtime` / `-p slicer-scheduler` would newly compile the entire CLI lib to obtain ~80 std-only lines; a `test-support` feature cannot avoid that without making the CLI's own deps optional, i.e. restructuring the CLI for a test helper. (2) Feature interaction: `default = ["report"]` → `report = ["slicer-runtime/report"]`. The dev-dep must say `default-features = false` to keep `report` out of narrow test builds; but any invocation that also builds the `pnp_cli` bin (workspace runs) unifies `report` back on, so `slicer-runtime` flips feature sets between narrow and broad invocations — rebuild churn on the exact narrow-vs-broad boundary `CLAUDE.md` §Test Discipline tells agents to walk daily. (3) It inverts the dependency direction: library tests depending on the CLI crate.
- **(b) new crate — chosen.** No deps, no features, no cycle, no unification surface. Bench targets receive dev-dependencies (standard Cargo; `gate_evidence` is `harness = false`, which changes the runner, not dependency resolution), so site 2's "cannot import `tests/common`" constraint — the original reason for its self-contained mirror — dissolves.
- **(c) `slicer-sdk` — rejected** by ADR-0004: guest-side only; compiles into guest WASM; guests must keep `default-features = false`.
- **(d) `xtask` lib target — rejected** (plan grounding correction 6): bin-only today; a lib would drag `walkdir`+`toml` into test builds and still not carry the pnp_cli-specific locator.
- **(e) revive `slicer-test` — rejected**: deleted by packet 78 (commit `c68f8973`) to end a two-surface test-support split; reviving the name re-opens exactly that confusion. The new crate has a disjoint charter (host-side process plumbing), which the new ADR states.

### Exact functions, files, tests

**New ADR** — `docs/adr/<NNNN>-host-side-test-support-crate.md`. Derive `<NNNN>` at write time: `ls docs/adr | rg -o '^[0-9]{4}' | sort | tail -1` + 1. Sections: Status (`Accepted`), Context (three copies, 162's `[FWD]`, ADR-0004's guest-side boundary, packet 78's deletion), Decision (host-side test support lives in `crates/slicer-test-support`, std-only, dev-dep-only, mirror-not-import of `xtask`), Consequences (one drift surface instead of three; the crate must stay dependency-free — a dep added there taxes every test build in two crates), Alternatives Considered (a)–(e) above with the feature-unification analysis.

**New crate** — `crates/slicer-test-support/{Cargo.toml, src/lib.rs}` + root `Cargo.toml` member entry. The members list is **grouped, not alphabetical** (`crates/*` block, then `modules/core-modules/*`, then `xtask`; the crates block itself is unordered — `slicer-helpers`/`slicer-model-io` sit between `slicer-sdk` and `slicer-wasm-host`): append `"crates/slicer-test-support"` anywhere inside the `crates/*` block, before the first `modules/core-modules/` entry.
- `Cargo.toml`: `name = "slicer-test-support"`, `edition = "2021"`, no `[dependencies]`, `[lints] workspace = true`.
- `pub fn workspace_root() -> PathBuf` — `CARGO_MANIFEST_DIR` (…/crates/slicer-test-support) `.parent().parent()`, canonicalized; same two-level shape as the existing `repo_root`/`workspace_root` copies.
- `pub fn newest_source_mtime(root: &Path) -> SystemTime` — moved from post-162 `slicer_cache.rs`; scan scope unchanged (crates/*/src/**, crates/*/Cargo.toml, crates/slicer-schema/wit/**/*.wit, workspace Cargo.toml; excludes tests/, benches/, modules/).
- `pub fn staleness_reason(bin_mtime: Option<SystemTime>, newest_src_mtime: SystemTime) -> Option<String>` — arms, comparison direction (`newest_src_mtime > artifact_mtime`), and signature moved unchanged; the **message text** is reconciled across the three divergent copies (see `packet.spec.md` §"Declared Deviation"), and must keep naming `pnp_cli`, the word `stale`, and the remedy `cargo build -p pnp-cli`. Rustdoc keeps the "mirrors `is_stale` (`xtask/src/build_guests.rs`); `xtask` is bin-only" pin.
- `pub fn pnp_cli_bin() -> PathBuf` — moved: profile-inference block, then `staleness_reason` gate, panic on `Some`. No fallback loop.
- Crate-level rustdoc: cites the new ADR by number, ADR-0004, and the dev-dep-only rule.

**Site 1** — `crates/slicer-runtime/tests/common/slicer_cache.rs`: delete the moved fn bodies; add `#[allow(unused_imports)] pub use slicer_test_support::{pnp_cli_bin, staleness_reason};` (keeps `run_pnp_cli_uncached`, all e2e callers, and `pnp_cli_freshness_tdd`'s import path working unchanged). The re-export carries exactly the two symbols with consumers: `newest_source_mtime` is deliberately excluded because nothing in the tree consumes it outside `slicer-test-support` itself (`rg -n 'newest_source_mtime' crates/ --glob '!crates/slicer-test-support/**'` matches only the explanatory doc-comment in `slicer_cache.rs`); a future caller imports it from `slicer_test_support` directly. `#[allow(unused_imports)]` is load-bearing, not decorative — the module is `#[path]`-included as a *private* module into several test binaries, and a `pub use` in a private module does fire `unused_imports` in binaries that touch only some of the names. Measured: removing it fails `cargo clippy --workspace --all-targets -- -D warnings` with `unused import: staleness_reason` in the `arachne_wall_sequence_e2e_tdd` test target. `repo_root()` may become a thin wrapper over `workspace_root()` or stay — not triplication, implementer's choice.

**Site 2** — `crates/slicer-runtime/benches/gate_evidence.rs`: delete its `pnp_cli_bin` mirror **and the module doc-comment sentence justifying self-containment** ("deliberately does NOT reuse `crates/slicer-runtime/tests/common`…") — that sentence's premise (`#[path]` inclusion dragging unrelated scaffolding) is void because the bench imports a dedicated crate, not `tests/common`. `use slicer_test_support::pnp_cli_bin;`. Local `repo_root()` may delegate to `workspace_root()`.

**Site 3** — `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`: delete `fn bin()`; replace its call sites (all `Command::new(bin())`) with `Command::new(slicer_test_support::pnp_cli_bin())` or an imported `pnp_cli_bin()`. The 162-mandated panic wording (`cargo build -p pnp-cli`, staleness cause) now lives once in the shared crate and must be preserved there (AC-N1). `workspace_root()`/`core_modules_path()` stay or delegate.

**Sites 4–7 — the four locator copies discovered at implementation time (premise correction)**

The packet was authored on the belief that the locator was triplicated. A verification sweep found **seven** copies. Four were never in packet 162's scope, and each defines its own `fn pnp_cli_bin` with **no freshness gate** (`staleness_reason` appears zero times in each) — the false-baseline trap 162 closed at the other three, still open. Each also branches on `std::env::var("PROFILE")`, which Cargo sets for **build scripts**, not for test binaries, so the branch is inert and always resolves `target/debug/pnp_cli` regardless of the profile the tests were built with. The packet is widened (user-approved) to migrate all four; without this, `packet.spec.md` AC-2 is unsatisfiable inside the declared scope.

- **Site 4** — `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`
- **Site 5** — `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`
- **Site 6** — `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`
- **Site 7** — `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`

Identical migration at each: delete the local `fn pnp_cli_bin`, delete the `PROFILE` branch with it, and add `use slicer_test_support::pnp_cli_bin;` (a qualified `slicer_test_support::pnp_cli_bin()` call site is equally acceptable — AC-3/AC-8 accept both name-resolution-equivalent forms). No manifest edit is needed in any of the four: the `slicer-test-support` dev-dependency added to `crates/slicer-runtime/Cargo.toml` for sites 1–2 already serves the whole `tests/` tree.

Each of these files also carries helpers such as `repo_root()`, `core_modules_dir()`, or `core_modules_root()`. Those are **not** the triplicated locator — they are single-line path joins. Delegating them to `slicer_test_support::workspace_root()` or leaving them local is the implementer's choice; AC-2, AC-3, and AC-8 are agnostic.

**Cargo.tomls** — `crates/slicer-runtime/Cargo.toml` and `crates/slicer-scheduler/Cargo.toml`: `[dev-dependencies] slicer-test-support = { path = "../slicer-test-support" }`.

**Backlog** — `docs/07_implementation_status.md`: TASK-146d row (dispatch, never read).

### Rejected alternatives (mechanics, beyond the home decision)

- **Re-export nothing; update every `slicer_cache` caller to import `slicer_test_support` directly.** Rejected. Note the caller population is single-digit, not the "~30" earlier revisions claimed, and only `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` imports a locator symbol at all — so the churn argument is weak on its own. The decisive reason is unchanged and independent of magnitude: `pnp_cli_freshness_tdd.rs` is the regression home packet 162 registered, and its AC commands reach `staleness_reason` through `crate::common::slicer_cache`. Keeping the `pub use` preserves that import path without relocating the test (see the adjacent "Move `pnp_cli_freshness_tdd.rs` into the new crate" rejection).
- **Move `pnp_cli_freshness_tdd.rs` into the new crate as unit tests.** Rejected: 162 registered it in the slicer-runtime `integration` bucket as its regression home and its AC commands point there; relocation would silently retire 162's guard invocation (`0 passed` false-pass hazard).
- **Also migrate `crates/pnp-cli/tests/e2e_integration_tdd.rs`.** Rejected: it uses `env!("CARGO_BIN_EXE_pnp_cli")`, which is *better* than the locator and available only there (binary-defining package). Migrating it would trade a Cargo guarantee for a filesystem probe.

## Files in Scope (read + edit)

Twelve entries (13 files — the new-crate entry contributes both `Cargo.toml` and `src/lib.rs`), well above the target of 3. Recomputed after the premise correction widened the consumer list from three sites to seven. Justification: the packet is a 1→N fan-in — one new crate plus exactly one edit per consumer (7) plus three one-line manifest edits plus one ADR. Every consumer edit is the same mechanical deletion of a copied block plus one `use` line. No file is edited for more than one reason; splitting would leave the workspace with either a dead crate or unmigrated copies, i.e. worse than either endpoint.

- `docs/adr/<NNNN>-host-side-test-support-crate.md` (new) - role: the home decision; expected change: authored per §Code Change Surface.
- `crates/slicer-test-support/Cargo.toml` + `src/lib.rs` (new) - role: the shared home; expected change: created with the four moved fns.
- `Cargo.toml` (root) - role: workspace registry; expected change: one member line.
- `crates/slicer-runtime/Cargo.toml` - role: consumer manifests; expected change: one dev-dep line.
- `crates/slicer-scheduler/Cargo.toml` - role: consumer manifest; expected change: one dev-dep line.
- `crates/slicer-runtime/tests/common/slicer_cache.rs` - role: site 1; expected change: fn bodies deleted, `pub use` added.
- `crates/slicer-runtime/benches/gate_evidence.rs` - role: site 2; expected change: mirror deleted, import added, doc-comment corrected.
- `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` - role: site 3; expected change: `fn bin()` deleted, calls repointed.
- `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs` - role: site 4 (premise correction); expected change: local `pnp_cli_bin` + `PROFILE` branch deleted, `use slicer_test_support::pnp_cli_bin;` added.
- `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs` - role: site 5 (premise correction); expected change: same as site 4.
- `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs` - role: site 6 (premise correction); expected change: same as site 4.
- `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs` - role: site 7 (premise correction); expected change: same as site 4.

## Read-Only Context

- `crates/slicer-runtime/tests/common/slicer_cache.rs` - locator block only (locate `pnp_cli_bin` / `staleness_reason` / `newest_source_mtime` by name) - purpose: the exact post-162 code being moved.
- `xtask/src/build_guests.rs` - `is_stale` fn only (locate by name) - purpose: verify the mirror pin in the moved rustdoc still describes the sibling accurately.
- `crates/pnp-cli/Cargo.toml` (short; read whole) - purpose: the feature table the ADR's alternative (a) analysis cites.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (short; read whole) - purpose: the boundary the new ADR complements.
- `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - §"CLI freshness" + §"Open Questions" only - purpose: the deferred `[FWD]` this packet resolves.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - no parity content; do not load or delegate.
- `target/`, `Cargo.lock`, `*.wasm`, generated code, vendored dependencies - never load.
- `crates/pnp-cli/src/**`, `crates/pnp-cli/tests/**` - not a copy site; `CARGO_BIN_EXE_pnp_cli` stays.
- `xtask/src/test.rs`, `xtask/src/build_guests.rs` (beyond the read-only `is_stale` lookup) - 162's gate surface; unaffected.
- `crates/slicer-runtime/tests/e2e/**`, `tests/integration/**` (except running them) — **with exactly one exception: the four named locator-copy sites (sites 4–7) listed in §Code Change Surface.** The ban otherwise stands in full, and the reason it stands is unchanged: `slicer_cache.rs`'s `pub use` re-export means the *other* caller files in those trees need **zero** edits (re-derive the count with `rg -l 'slicer_cache' crates/slicer-runtime/tests/`; it is single-digit, not the "~30" earlier revisions claimed — the ban does not depend on the number). Editing any file in these trees other than the four named ones has broken AC-3's premise. The four are in bounds only because each defines its own locator copy, not because it calls one.
- All WIT, `modules/**`, `crates/slicer-schema/**`, `crates/slicer-macros/**`, `crates/slicer-sdk/**` - packets 163/164's surface; nothing here touches guests.

## Expected Sub-Agent Dispatches

- Question: "Run the packet precondition check (three-site `staleness_reason` grep from packet.spec §Prerequisites); READY or BLOCKED?"; scope: the three site files; return: `FACT`; purpose: Step 0 gate.
- Question: "Derive the next free ADR number: `ls docs/adr | rg -o '^[0-9]{4}' | sort | tail -1`, report that value + 1"; scope: `docs/adr/`; return: `FACT` (one number); purpose: Step 1.
- Question: "Run `cargo check --workspace --all-targets`; pass/fail + first 20 error lines on failure"; scope: workspace; return: `FACT` + SNIPPETS ≤20; purpose: Step 3/4 gates.
- Question: "Run the AC-4, AC-5, and baseline test commands (each already `rg`-filtered); return each `test result:` line"; scope: workspace; return: `FACT` (≤5 lines); purpose: Step 4.
- Question: "Append the TASK-146d row to `docs/07_implementation_status.md` following the TASK-119a/TASK-194a sub-letter convention; return the added line"; scope: `docs/07_implementation_status.md`; return: `FACT`; purpose: Step 4 — never read the backlog directly.

## Data and Contract Notes

- IR/manifest contracts: none touched. No config key, no module manifest, no IR type.
- WIT boundary: none. The new crate must never appear in any guest dependency closure; AC-1's zero-`[dependencies]` check plus dev-dep-only placement enforce it structurally.
- Determinism/scheduler constraints: none — test plumbing only. G-code output must be byte-identical; the green baseline (`perimeter_parity` 3 passed, `legacy_zero_matches_golden` 1 passed) is the check.

## Locked Assumptions and Invariants

- **Locked (by 162, carried forward):** the freshness gate is loud — stale or absent binary ⇒ panic; no release/debug fallback. This packet may not weaken it while moving it.
- **Locked (by this packet's ADR):** `slicer-test-support` is host-side, std-only, and dev-dep-only. Adding a `[dependencies]` entry to it, or depending on it from a non-dev section or a guest crate, requires superseding the ADR.
- **Not locked:** the crate's future contents — other host-side test helpers may move in under the same ADR; this packet moves only the locator.

## Risks and Tradeoffs

- **Scan-scope over-approximation grows by one crate.** `newest_source_mtime` scans `crates/*/src/**`; the new crate's own `src/` now matches, yet it does not link into `pnp_cli` — so editing the locator itself makes `pnp_cli` look stale until the next `cargo build`. Accepted: rare, one-file, and fails loud-and-safe (a spurious "stale" panic) rather than silent-and-wrong; narrowing the scan to `pnp_cli`'s real dep closure is redesign, out of scope.
- **Feature unification is the trap the ADR must document, not just avoid.** If a future contributor "simplifies" by folding the helper into `pnp-cli`'s lib, the `report` unification churn returns silently. The ADR's alternative (a) analysis is the guard.
- **`0 passed` false-pass on every name-filtered gate.** All four name-filtered test commands match `^test result: ok\. [1-9][0-9]* passed`, which fails on both `0 passed` and a FAILED run; the unfiltered whole-crate runs (`cargo check/clippy --all-targets`) do not need it.
- **The four newly-gated sites will now panic loudly on a stale `pnp_cli`.** Sites 4–7 previously had no freshness gate at all: they silently ran whatever binary happened to sit at `target/debug/pnp_cli`, including a months-old one. After migration they inherit 162's loudness contract — stale or absent binary ⇒ panic naming `pnp_cli`, `stale`, the resolved path, and a remedy. This is **intended**: it is the whole point of extending the gate. But it will surface as *new* test failures for anyone whose build tree is stale, and those failures are not regressions from this packet's refactor — they are the gate reporting a pre-existing condition the old code hid. Anyone triaging such a failure should rebuild `pnp_cli` before filing anything. A second-order effect: these four tests may previously have been passing against a stale binary, so their first green run post-migration is the first run whose result is actually trustworthy.
- **162 not landed when this packet activates.** The precondition check fails closed (`BLOCKED`); the packet must not proceed against the pre-162 tree, whose sites have different shapes (fallback loops still present, no `staleness_reason`).

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S` (Step 2, crate creation — the code is moved, not written)
- Highest-risk dispatch and required return format: the Step 4 test batch — must return only the `test result:` lines (≤5 lines), never raw cargo output.

## Open Questions

- `[FWD]` Whether `repo_root()` (site 1) and `workspace_root()`/`core_modules_path()` (sites 2/3) become thin wrappers over `slicer_test_support::workspace_root()` or stay local. Implementer-resolvable: they are single-line path joins, not the triplicated locator; AC-2/AC-3 are agnostic. Prefer delegating if it costs no extra churn.

None blocking. Status stays `draft` pending review.
