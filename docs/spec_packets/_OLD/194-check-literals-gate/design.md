# Design: 194-check-literals-gate

## Controlling Code Paths

- Primary code path: `xtask/src/main.rs` (hand-rolled arg match, 160 lines; verified 2026-08-07) dispatching to a new `xtask/src/check_literals.rs`, following the existing `check_deviations` / `gen_config_docs` pattern (`mod` declaration, `USAGE` entry, match arm returning `ExitCode`). Workspace root comes from the existing `build_guests::workspace_root()`.
- Neighboring tests/fixtures: xtask already carries in-module `#[cfg(test)]` unit tests across six of its modules (`build_guests.rs`, `check_deviations.rs`, `compact_specs.rs`, `gen_config_docs.rs`, `test.rs`, `wit_verify.rs` — verified 2026-08-07); the new module follows that convention with its own `#[cfg(test)]` tests over in-memory `.rs` fixture strings (xtask is a bin-only crate — tests compile into the bin target and run via `cargo test -p xtask`).
- OrcaSlicer comparison: none — no parity surface; the OrcaSlicer sections are intentionally absent from this packet.

## Architecture Constraints

- xtask is **bin-only** (no `[lib]`; see ADR-0054's rationale for `pnp-cli-locator`) and must stay that way: `check_literals` is a private `mod` of `xtask/src/main.rs`, and no other crate may import it. Dependencies added to `xtask/Cargo.toml` tax only `cargo xtask` builds, never test builds of workspace crates.
- The checker only ever *reads* the tree. It must not write, format, or fix files.
- Determinism: violation lines are emitted sorted by (path, line) and the watchlist is held in a `BTreeSet` so output is stable across runs and platforms; paths are normalized to forward slashes and made workspace-root-relative before printing (this is Windows — `walkdir` yields backslashes).
- This packet's change surface (xtask + docs) does **not** feed guest WASM; no `build-guests` obligation here.

## Code Change Surface

- Selected approach: full-file `syn` parsing (AST visitor) for everything syn can parse, plus a token-stream fallback scan inside macro invocations, with waiver detection done against the raw source lines via `proc-macro2` span locations.
- Exact functions, traits, manifests, tests, and fixtures:
  - `xtask/Cargo.toml`: add `syn = { version = "2.0", features = ["full", "visit"] }` and `proc-macro2 = { version = "1", features = ["span-locations"] }` (span-locations is what makes `Span::start().line` real outside proc-macro context; syn re-exports proc-macro2 so feature unification applies). `syn 2.0` is already in the dependency graph via `crates/slicer-macros/Cargo.toml`.
  - `xtask/src/check_literals.rs` (new), public surface consumed by `main.rs`:
    - `pub fn run(ws: &Path, report: bool, path_filters: &[String]) -> i32` — orchestrates: derive watchlist, collect enforced files, scan, print, return exit code (0 clean or report mode, 1 violations in enforce mode).
    - `fn derive_watchlist(ws: &Path) -> BTreeSet<String>` — walk `crates/*/src/**/*.rs`, `syn::parse_file` each, collect names of `syn::ItemStruct` with `Visibility::Public` and `Fields::Named` of len ≥ 5 (including structs nested in inline modules — use the visitor, not just top-level items). Files that fail to parse are reported to stderr and skipped (non-fatal).
    - `enum ScanMode { WholeFile, CfgTestOnly }`
    - `fn collect_enforced_files(ws: &Path, filters: &[String]) -> Vec<(PathBuf, ScanMode)>` — `crates/*/tests/**/*.rs`, `modules/core-modules/*/tests/**/*.rs`, `crates/*/benches/**/*.rs` as `WholeFile`; `crates/*/src/**/*.rs` as `CfgTestOnly`. A filter matches when the normalized ws-relative path equals the filter or starts with `<filter>/` (component-aware; `crates/slicer-ir` must not match `crates/slicer-ir-extra`). `crates/slicer-wasm-host/test-guests/**` never matches any pattern (one directory level deeper than `crates/*/src|tests`) — assert this exemption in a unit test rather than an exclusion list.
    - `pub(crate) struct Violation { file: String, line: usize, type_name: String }`
    - `fn scan_source(file_label: &str, src: &str, mode: ScanMode, watch: &BTreeSet<String>) -> Vec<Violation>` — the pure, unit-testable core. A `syn::visit::Visit` impl that:
      - tracks an impl-target stack (`visit_item_impl`: push the last path segment of `self_ty` when it is a `Type::Path`), resolving single-segment `Self` literals against the top of the stack;
      - tracks `#[cfg(test)]` mod nesting (attribute meta exactly `cfg(test)`); in `CfgTestOnly` mode, literals outside a cfg-test mod subtree are ignored;
      - on `ExprStruct` with `qself: None` and `rest: None`: if the path's last segment is watched (or `Self` resolves to a watched name) and no waiver covers the literal's opening line, push a `Violation`;
      - on every `Macro` node (expr/item/stmt): recursively scan the token stream for an `Ident` whose string is watched (or `Self` resolving to watched) immediately followed by a brace-delimited `Group`; if the group's top-level tokens contain no `..` (joint `.`+`.` puncts), and no waiver covers the ident's span line, push a `Violation`. Recurse into nested groups.
    - `fn has_waiver(lines: &[&str], line_1based: usize) -> bool` — true when the literal's opening line or the line above contains `// exhaustive:` followed by at least one non-whitespace character.
  - `xtask/src/main.rs`: `mod check_literals;`, `USAGE` entries (`check-literals`, `check-literals --report`, `check-literals [PATHS...]`), match arm parsing `--report` plus positional paths; unknown `--*` flag prints usage and exits 2 (mirror `check-deviations` handling).
  - `#[cfg(test)]` unit tests in `check_literals.rs` with the exact names bound by the ACs: `watchlist_includes_pub_ge5_named_structs_only`, `scan_passes_fru_and_waivered_literals`, `scan_flags_exhaustive_watched_literals`, `scan_flags_self_in_impl_blocks`, `scan_flags_macro_embedded_and_multisegment_literals`, `scan_ignores_enum_variants_and_non_test_src`, `scan_requires_waiver_reason`, `scan_macro_range_blind_spot_documented`, plus a `collect`-level test for the test-guests exemption if it can be expressed without touching the real tree (otherwise document the exemption in rustdoc only).
  - Docs: `docs/21_data_defaults_and_fixtures.md` (new), `.claude/doc-index.md` (one bullet, matching the existing bullet style — note the existing `docs/20` line uses a stray table row; do not imitate or fix it), `docs/00_project_overview.md` (one row in the "Normative Document Map (LLM/Reviewer Fast Index)" table, bare-filename style like the `20_support_preview.md` row, inserted before the `DEVIATION_LOG.md` row; the precedence rule for conflicts is not touched), `CLAUDE.md` (one short MUST section).
- Rejected alternatives and reasons:
  - A clippy lint / dylint crate — requires nightly or an external driver; xtask keeps it a plain `cargo xtask` verb like every other repo gate.
  - Regex-only scanning — cannot track `impl` targets for `Self`, cfg(test) subtrees, or macro token trees without unacceptable false positives.
  - A manual watchlist ledger — explicitly locked out by the plan (decision 2): the list must be derived at run time so new ≥5-field structs are watched the day they land.

## Files in Scope (read + edit)

- `xtask/src/check_literals.rs` - role: the checker (new file); expected change: full module + unit tests.
- `xtask/src/main.rs` - role: dispatch; expected change: `mod` line, USAGE text, one match arm.
- `xtask/Cargo.toml` - role: deps; expected change: two dependency lines.
- Justified extras (docs-only, no code risk): `docs/21_data_defaults_and_fixtures.md` (new), `.claude/doc-index.md`, `docs/00_project_overview.md` (one table row), `CLAUDE.md`.

## Read-Only Context

- `xtask/src/check_deviations.rs` - pattern reference for `run(&ws, …) -> i32` shape and stderr conventions; skim only.
- `crates/slicer-ir/src/slice_ir.rs` - lines around `pub struct Point3WithWidth` / `pub struct PrintEntity` only, as live watchlist sanity examples (8 and 6 named fields; verified 2026-08-07); never read the whole 2000+-line file.
- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read in full.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - not applicable; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `docs/specs/_OLD/default-builder-migration.md` (1449 lines) - packet 195's authority, not this packet's; do not open.
- `crates/*/tests/**` beyond what `check-literals` itself prints - do not browse test files to "sanity check" violations; trust the unit fixtures.

## Expected Sub-Agent Dispatches

- Question: does `cargo check --workspace --all-targets` pass after the xtask changes?; scope: workspace; return: `FACT` pass/fail with ≤ 20 error lines on failure; purpose: Step 4 gate.
- Question: run each pipe-suffixed AC command and report PASS/FAIL per AC; scope: repo root; return: `FACT` (one line per AC); purpose: completion gate.

## Data and Contract Notes

- IR/manifest contracts: none touched — the checker is read-only tooling.
- WIT boundary: untouched. The `crates/slicer-wasm-host/test-guests/*/src` exemption exists precisely so WIT adapter shims keep breaking loudly on new fields.
- Determinism/scheduler constraints: none; output ordering handled under Architecture Constraints.
- Output contract (consumed by packets 195–199): violation line `<path>:<line>: exhaustive literal of watched type \`<Name>\``; summary line `check-literals: <N> violation(s) in <M> file(s) (watchlist: <K> types)`; exit codes 0/1/2 as specced. Treat this as frozen once the packet closes.

## Locked Assumptions and Invariants

- Watchlist rule locked by the plan: `pub` + ≥ 5 named fields + defined under `crates/*/src` — regardless of whether the type has `Default`. `pub(crate)` excluded. Enum struct-variants cannot fire (watchlist derives from struct definitions only).
- Waiver format locked here for all downstream packets: `// exhaustive: <reason>`, same line or line immediately above, reason mandatory.
- Production `src/` outside `#[cfg(test)]` subtrees is exempt on purpose; this invariant is documented in `docs/21_data_defaults_and_fixtures.md`, not just implemented.
- Enforce mode exiting 1 on the current tree is the *expected* state until packets 196–198 land; nothing in this packet may "fix" violations to get a green enforce run.

## Risks and Tradeoffs

- Token-stream heuristic false negatives: a top-level `..` from a range expression (`field: 0..2`) inside a macro's brace group reads as an FRU rest and suppresses detection. Accepted and locked by test `scan_macro_range_blind_spot_documented`; the AST path (non-macro code) has no such ambiguity because `syn::ExprStruct::rest` is precise.
- Token-stream heuristic false positives: an enum struct-variant whose *variant name* collides with a watched struct name would fire (`SomeEnum::PrintEntity { … }`). No such collision exists today; the waiver is the escape hatch. Documented in docs/21.
- `#[cfg(test)] mod x;` (out-of-line) is not followed into its file. Measured 2026-08-07: zero occurrences in `crates/` — documented limitation, no code needed.
- Parse cost: syn-parsing every src/test file per run is O(workspace) but xtask-local; if it proves slow the walker can skip files containing no watched name via a cheap substring pre-filter — do not add caching in this packet.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, the visitor + waiver logic)
- Highest-risk dispatch and required return format: workspace check after wiring; `FACT` pass/fail with ≤ 20 error lines.

## Open Questions

- `[FWD]` Macro-generated struct definitions are invisible to the watchlist: `ResolvedConfig` (`crates/slicer-ir/src/resolved_config.rs`) is emitted by a macro, has far more than 5 fields, and *is* constructed with struct literals in ≥ 8 test files (measured 2026-08-07) — those literals will never be flagged. The locked rule ("derived from struct definitions parsed from source") stands; the implementer should record this hole in docs/21's blind-spot section and may note a follow-up option (seeding the watchlist with known macro-generated names) for the orchestrator. Do not widen the rule in this packet.
- `[FWD]` Attribute detection accepts exactly `#[cfg(test)]`. Mods gated `#[cfg(any(test, feature = "test"))]` (e.g. `slicer_sdk`'s `test_support` declaration in `crates/slicer-sdk/src/lib.rs`) are treated as production src and therefore exempt; their literal-bearing fixture fns are intentionally exempt anyway (they are the FRU bases). Document; do not widen.
