---
status: implemented
packet: 165_cli-binary-locator-extraction
task_ids:
  - TASK-146d
---

# 165_cli-binary-locator-extraction

## Goal

Collapse the **seven** copies of the `pnp_cli` binary locator into one new std-only host-side crate `crates/slicer-test-support`, with an ADR recording that home decision.

**Premise correction (found at implementation time, not part of the original authoring).** This packet was authored on the belief that the locator was triplicated at three sites. A verification sweep during implementation found **seven**. Packet 162 fixed the freshness bug at three of them:

- `crates/slicer-runtime/tests/common/slicer_cache.rs` (`pnp_cli_bin`)
- `crates/slicer-runtime/benches/gate_evidence.rs` (`pnp_cli_bin`)
- `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` (its copy is named `bin`)

Four more were never in 162's scope and carry **no freshness gate at all** (`staleness_reason` appears zero times in each) — the exact false-baseline trap 162 closed at the other three, still open:

- `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`
- `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`
- `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`
- `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`

Those four additionally branch on `std::env::var("PROFILE")`, which Cargo sets for **build scripts**, not for test binaries — so they always resolve `target/debug/pnp_cli` regardless of the profile the tests were built with. The packet is widened (user-approved) to migrate all seven, which both makes AC-2 satisfiable and extends 162's freshness gate to the four that never had one.

## Problem Statement

**Premise correction (recorded at implementation time; the original authoring got the count wrong).** This packet was written on the belief that the `pnp_cli` locator existed in three copies. A verification sweep during implementation found **seven**. Packet 162 closed the stale-`pnp_cli` false-baseline trap at three spawn sites (`crates/slicer-runtime/tests/common/slicer_cache.rs`, `crates/slicer-runtime/benches/gate_evidence.rs`, `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`, whose copy is named `bin`). Four further copies were never in 162's scope and carry **no freshness gate at all** — `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`, `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`, `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`, `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`. Each of those four also branches on `std::env::var("PROFILE")`, a variable Cargo sets for build scripts rather than test binaries, so the branch is inert and always resolves `target/debug/pnp_cli`. The packet is widened (user-approved) to all seven: this makes AC-2 satisfiable inside the declared scope — the original scope forbade editing the trees the four live in, contradicting AC-2 — and extends 162's freshness gate to the four sites that never had one.

Packet 162 closed the trap at its three spawn sites but — by explicit decision recorded in its `[FWD]` — fixed them **in place**, leaving the locator + freshness assert duplicated. Extraction was deferred because the shared home is an architecture decision requiring an ADR: ADR-0004 places only *guest-side* test support in `slicer-sdk` (a crate compiled into guest WASM — the wrong home for host process-spawning plumbing), and `slicer-test`, the crate that could have hosted it, was deleted by packet 78 (commit `c68f8973`). The residual risk 162 accepted is drift among the three copies — `gate_evidence.rs` produces DEV-026's 50-layer time evidence, so a drifted copy there silently invalidates governance evidence. The ADR-0045 plan queues this as row #4 precisely because "the kind of follow-up that historically evaporates" needed its own TASK id and row.

## Architecture Constraints

- **The wasm-staleness snippet does not apply.** No file in the change surface is a guest-WASM input (`CLAUDE.md` §"Guest WASM Staleness" lists them): the new crate is host-side, dev-dep-only, and never linked into any guest or production target. The coord-system snippet likewise does not apply (no geometry).
- ADR-0004 boundary: guest-side test support lives in `slicer-sdk`; the new crate is its host-side counterpart and must never be depended on by a guest crate, `slicer-sdk`, or any `[dependencies]` (non-dev) section. The ADR authored by this packet records this.
- `xtask` stays bin-only. `slicer_test_support::staleness_reason` remains a documented **mirror** of `is_stale` (`xtask/src/build_guests.rs`) — the crate's rustdoc must pin that sibling relationship, carrying forward the pin 162 placed in `slicer_cache.rs`.
- The freshness gate's loudness contract (162): stale ⇒ panic whose message contains `pnp_cli`, `stale`, the resolved path, and a remedy; absent ⇒ panic; no release/debug fallback probing, ever.

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
