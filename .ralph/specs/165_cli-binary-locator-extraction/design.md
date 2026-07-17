# Design: 165_cli-binary-locator-extraction

## Controlling Code Paths

- Primary code path: `slicer_test_support::pnp_cli_bin` (new) → profile-inference (`current_exe().parent().parent()` sibling lookup) → `newest_source_mtime` scan → `staleness_reason` decision → return path or panic. Consumers: `crates/slicer-runtime/tests/common/slicer_cache.rs::run_pnp_cli_uncached` (site 1), `crates/slicer-runtime/benches/gate_evidence.rs` (site 2, DEV-026 evidence producer), `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs::run_dag` etc. (site 3).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` (162's regression tests over `staleness_reason`, reached via `common::slicer_cache`'s re-export); the ~30 `slicer-runtime` e2e/integration files calling `common::slicer_cache::{cached_run, run_pnp_cli_uncached, expect_outcome, ...}` — untouched by design.
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
- `pub fn staleness_reason(bin_mtime: Option<SystemTime>, newest_src_mtime: SystemTime) -> Option<String>` — moved verbatim; rustdoc keeps the "mirrors `is_stale` (`xtask/src/build_guests.rs`); `xtask` is bin-only" pin.
- `pub fn pnp_cli_bin() -> PathBuf` — moved: profile-inference block, then `staleness_reason` gate, panic on `Some`. No fallback loop.
- Crate-level rustdoc: cites the new ADR by number, ADR-0004, and the dev-dep-only rule.

**Site 1** — `crates/slicer-runtime/tests/common/slicer_cache.rs`: delete the moved fn bodies; add `pub use slicer_test_support::{pnp_cli_bin, staleness_reason, newest_source_mtime};` (keeps `run_pnp_cli_uncached`, all e2e callers, and `pnp_cli_freshness_tdd`'s import path working unchanged). `repo_root()` may become a thin wrapper over `workspace_root()` or stay — not triplication, implementer's choice.

**Site 2** — `crates/slicer-runtime/benches/gate_evidence.rs`: delete its `pnp_cli_bin` mirror **and the module doc-comment sentence justifying self-containment** ("deliberately does NOT reuse `crates/slicer-runtime/tests/common`…") — that sentence's premise (`#[path]` inclusion dragging unrelated scaffolding) is void because the bench imports a dedicated crate, not `tests/common`. `use slicer_test_support::pnp_cli_bin;`. Local `repo_root()` may delegate to `workspace_root()`.

**Site 3** — `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`: delete `fn bin()`; replace its call sites (all `Command::new(bin())`) with `Command::new(slicer_test_support::pnp_cli_bin())` or an imported `pnp_cli_bin()`. The 162-mandated panic wording (`cargo build -p pnp-cli`, staleness cause) now lives once in the shared crate and must be preserved there (AC-N1). `workspace_root()`/`core_modules_path()` stay or delegate.

**Cargo.tomls** — `crates/slicer-runtime/Cargo.toml` and `crates/slicer-scheduler/Cargo.toml`: `[dev-dependencies] slicer-test-support = { path = "../slicer-test-support" }`.

**Backlog** — `docs/07_implementation_status.md`: TASK-146d row (dispatch, never read).

### Rejected alternatives (mechanics, beyond the home decision)

- **Re-export nothing; update all ~30 callers to import `slicer_test_support` directly.** Rejected: churns 30 files for zero information; the re-export keeps the diff at the three sites plus manifests.
- **Move `pnp_cli_freshness_tdd.rs` into the new crate as unit tests.** Rejected: 162 registered it in the slicer-runtime `integration` bucket as its regression home and its AC commands point there; relocation would silently retire 162's guard invocation (`0 passed` false-pass hazard).
- **Also migrate `crates/pnp-cli/tests/e2e_integration_tdd.rs`.** Rejected: it uses `env!("CARGO_BIN_EXE_pnp_cli")`, which is *better* than the locator and available only there (binary-defining package). Migrating it would trade a Cargo guarantee for a filesystem probe.

## Files in Scope (read + edit)

Eight files, above the target of 3. Justification: the packet is a 1→N fan-in — one new crate plus exactly one edit per consumer (3) plus three one-line manifest edits plus one ADR. No file is edited for more than one reason; splitting would leave the workspace with either a dead crate or unmigrated copies, i.e. worse than either endpoint.

- `docs/adr/<NNNN>-host-side-test-support-crate.md` (new) - role: the home decision; expected change: authored per §Code Change Surface.
- `crates/slicer-test-support/Cargo.toml` + `src/lib.rs` (new) - role: the shared home; expected change: created with the four moved fns.
- `Cargo.toml` (root) - role: workspace registry; expected change: one member line.
- `crates/slicer-runtime/Cargo.toml` - role: consumer manifests; expected change: one dev-dep line.
- `crates/slicer-scheduler/Cargo.toml` - role: consumer manifest; expected change: one dev-dep line.
- `crates/slicer-runtime/tests/common/slicer_cache.rs` - role: site 1; expected change: fn bodies deleted, `pub use` added.
- `crates/slicer-runtime/benches/gate_evidence.rs` - role: site 2; expected change: mirror deleted, import added, doc-comment corrected.
- `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` - role: site 3; expected change: `fn bin()` deleted, calls repointed.

## Read-Only Context

- `crates/slicer-runtime/tests/common/slicer_cache.rs` - locator block only (locate `pnp_cli_bin` / `staleness_reason` / `newest_source_mtime` by name) - purpose: the exact post-162 code being moved.
- `xtask/src/build_guests.rs` - `is_stale` fn only (locate by name) - purpose: verify the mirror pin in the moved rustdoc still describes the sibling accurately.
- `crates/pnp-cli/Cargo.toml` (44 lines) - purpose: the feature table the ADR's alternative (a) analysis cites.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (72 lines) - purpose: the boundary the new ADR complements.
- `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - §"CLI freshness" + §"Open Questions" only - purpose: the deferred `[FWD]` this packet resolves.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - no parity content; do not load or delegate.
- `target/`, `Cargo.lock`, `*.wasm`, generated code, vendored dependencies - never load.
- `crates/pnp-cli/src/**`, `crates/pnp-cli/tests/**` - not a copy site; `CARGO_BIN_EXE_pnp_cli` stays.
- `xtask/src/test.rs`, `xtask/src/build_guests.rs` (beyond the read-only `is_stale` lookup) - 162's gate surface; unaffected.
- `crates/slicer-runtime/tests/e2e/**`, `tests/integration/**` (except running them) - the re-export exists precisely so these need no edits; an implementer editing them has broken AC-3's premise.
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
- Determinism/scheduler constraints: none — test plumbing only. G-code output must be byte-identical; the green baseline (`perimeter_parity` 12 passed, `legacy_zero_matches_golden` 1 passed) is the check.

## Locked Assumptions and Invariants

- **Locked (by 162, carried forward):** the freshness gate is loud — stale or absent binary ⇒ panic; no release/debug fallback. This packet may not weaken it while moving it.
- **Locked (by this packet's ADR):** `slicer-test-support` is host-side, std-only, and dev-dep-only. Adding a `[dependencies]` entry to it, or depending on it from a non-dev section or a guest crate, requires superseding the ADR.
- **Not locked:** the crate's future contents — other host-side test helpers may move in under the same ADR; this packet moves only the locator.

## Risks and Tradeoffs

- **Scan-scope over-approximation grows by one crate.** `newest_source_mtime` scans `crates/*/src/**`; the new crate's own `src/` now matches, yet it does not link into `pnp_cli` — so editing the locator itself makes `pnp_cli` look stale until the next `cargo build`. Accepted: rare, one-file, and fails loud-and-safe (a spurious "stale" panic) rather than silent-and-wrong; narrowing the scan to `pnp_cli`'s real dep closure is redesign, out of scope.
- **Feature unification is the trap the ADR must document, not just avoid.** If a future contributor "simplifies" by folding the helper into `pnp-cli`'s lib, the `report` unification churn returns silently. The ADR's alternative (a) analysis is the guard.
- **`0 passed` false-pass on every name-filtered gate.** All four name-filtered test commands carry `| rg -v '0 passed'`; the unfiltered whole-crate runs (`cargo check/clippy --all-targets`) do not need it.
- **162 not landed when this packet activates.** The precondition check fails closed (`BLOCKED`); the packet must not proceed against the pre-162 tree, whose sites have different shapes (fallback loops still present, no `staleness_reason`).

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S` (Step 2, crate creation — the code is moved, not written)
- Highest-risk dispatch and required return format: the Step 4 test batch — must return only the `test result:` lines (≤5 lines), never raw cargo output.

## Open Questions

- `[FWD]` Whether `repo_root()` (site 1) and `workspace_root()`/`core_modules_path()` (sites 2/3) become thin wrappers over `slicer_test_support::workspace_root()` or stay local. Implementer-resolvable: they are single-line path joins, not the triplicated locator; AC-2/AC-3 are agnostic. Prefer delegating if it costs no extra churn.

None blocking. Status stays `draft` pending review.
