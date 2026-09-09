---
status: implemented
packet: 194-check-literals-gate
task_ids:
  - TASK-316
---

# 194-check-literals-gate

## Goal

Implement `cargo xtask check-literals` — a syn-based scanner that flags exhaustive struct literals of watched types in test code (report mode, path filter, exit 1 on violations) — and author its rule documentation (`docs/21_data_defaults_and_fixtures.md`, `.claude/doc-index.md` entry, CLAUDE.md MUST section explicitly marked gate-off until packet 199).

## Problem Statement

Adding one field to a widely-constructed struct forces a workspace-wide sweep of exhaustive struct literals in test code. Measured in `docs/specs/_OLD/struct-literal-churn-gate-plan.md`: commit `a579fc18` (packet 193) touched 165 files, ~90% one-line `overhang_distance_mm: None` filler in test files after `Point3WithWidth` gained a field; `383b633b` swept 26 `LayerCollectionIR` sites; `defb4b19` re-edited every test constructing `SliceRunOptions`. The prior fix (`docs/specs/_OLD/default-builder-migration.md`) added `Default` impls but produced no ongoing rule, so later packets freshly wrote exhaustive literals (re-derived 2026-08-07: 103 test files still construct `Point3WithWidth` literals). Production `src/` literals are deliberately exempt: in `a579fc18` the marshal/producer sites received real logic for the new field — exhaustive literals there are compiler-enforced propagation checkpoints, and FRU there would have silently dropped `overhang_distance_mm` at the WIT boundary.

This packet builds the enforcement tool and its documentation. It does not convert any call site and does not flip enforcement on.

## Architecture Constraints

- xtask is **bin-only** (no `[lib]`; see ADR-0054's rationale for `pnp-cli-locator`) and must stay that way: `check_literals` is a private `mod` of `xtask/src/main.rs`, and no other crate may import it. Dependencies added to `xtask/Cargo.toml` tax only `cargo xtask` builds, never test builds of workspace crates.
- The checker only ever *reads* the tree. It must not write, format, or fix files.
- Determinism: violation lines are emitted sorted by (path, line) and the watchlist is held in a `BTreeSet` so output is stable across runs and platforms; paths are normalized to forward slashes and made workspace-root-relative before printing (this is Windows — `walkdir` yields backslashes).
- This packet's change surface (xtask + docs) does **not** feed guest WASM; no `build-guests` obligation here.

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
