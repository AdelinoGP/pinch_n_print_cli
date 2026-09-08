---
status: implemented
packet: 232-freshness-gate-docs
task_ids:
  - TASK-343
---

# 232-freshness-gate-docs

## Goal

Restate the guest-freshness contract as artifact-verified everywhere it is currently written as mtime-based — `CLAUDE.md`, `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, `docs/07_implementation_status.md`, ADR-0014, ADR-0045, `CONTEXT.md`, the `wasm-staleness` authoring snippet and `.claude/skills/spec-review/SKILL.md` — and make CI actually run the verifier by adding `cargo test -p xtask` to the `test` job with real-artifact tests that fail rather than skip.

## Problem Statement

The guest-freshness contract is stated in nine places, and packets 229, 230 and 231 falsify every one of them. `CLAUDE.md`'s "## Guest WASM Staleness (MUST follow)" is the loudest: it tells every agent that freshness is mtime-based, lists by hand the paths that invalidate a guest, and closes with a 2026-07-25 anecdote about `slicer-core` having been missing from `shared_input_paths`' `shared_crates` array — an array packet 231 deletes. `docs/03_wit_and_manifest.md` states the same model twice, once as a table row reading `| `cargo xtask build-guests --check` | Stale in-tree guest (mtime-based) |` and once in the normative section "### Build & Freshness Contract (Normative)", whose `--check` bullet reads "verify only; exit 1 if any source is newer than its artifact". `docs/05_module_sdk.md` calls `--check` "the canonical pre-test gate", which `CLAUDE.md` elsewhere assigns to `cargo xtask test`.

Two docs cite symbols that never existed. `docs/07_implementation_status.md`'s TASK-146b row names `stage_wit_mtime(ws_root, stage_id)`; the real function was `stage_wit_snapshot`. ADR-0045 names `compute_shared_mtime` in one prose paragraph and two table cells; the real function was `compute_shared_freshness`. Both real functions are deleted by packet 231, so a repin must both correct the name and mark it retired. ADR-0014's Amendments section records packet 185's `shared_crates` rule, and its Consequences still assert "Touching `slicer-core` does not trigger a guest rebuild storm" — false since 2026-07-25 and true again, by a different mechanism, after packet 231.

Two authoring surfaces encode a verification form that Round 5 finding R5-3 proved unsound: `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md` and `.claude/skills/spec-review/SKILL.md` both tell downstream agents to look for `STALE:`. A `wasm-tools`-missing infrastructure error prints no `STALE:` line, so `--check 2>&1 | rg -q 'STALE:' && echo FAIL || echo PASS` reports PASS on a check that never ran.

Finally, none of this is tested in CI. `.github/workflows/ci.yml`'s `test` job runs `cargo test -p slicer-runtime && cargo test -p pnp-cli && cargo test -p slicer-helpers` and never `-p xtask`, so every verifier test packets 229-231 add is dead in CI (R5-10).

This is one coherent slice because the nine statements are the same statement, and updating a subset leaves the tree self-contradictory in a way that is harder to detect than the current uniform staleness.

## Architecture Constraints

- The `wasm-staleness` snippet is **deliberately omitted from this design's constraint list**, even though this packet *rewrites* that snippet. Its applies-to list is `crates/slicer-schema/wit/**`, the four (now five) shared crates, `modules/core-modules/*/src|Cargo.toml|wit-guest`, and `crates/slicer-wasm-host/test-guests/*/src|Cargo.toml`. This packet's change surface is documentation, `.claude/skills/**`, `.github/workflows/ci.yml` and the test module of `xtask/src/wit_verify.rs` — none of which feeds a guest `.wasm`. Quoting the snippet as an obligation here would assert a rebuild duty that does not exist, in the very packet that fixes how the obligation is stated.
- The `coord-system` snippet does not apply: no geometry, no mm/unit conversion.
- The rewritten snippet must remain **verbatim-or-absent** downstream: it retains its `<!-- snippet: wasm-staleness -->` marker and its "copy exactly; do not paraphrase" framing, because `spec-packet-generator` self-review and `spec-review` both check the block for exactness. Changing the marker would silently disable both checks.
- Every doc edit is subject to `CLAUDE.md` §"In-Tree Citation Style (MUST follow)": cite by symbol name with the crate-qualified path. Two of the defects being repaired here (`stage_wit_mtime` in `docs/07`, `compute_shared_mtime` in ADR-0045) are symbol-name fabrications, not line-number rot, so the fix is to verify each symbol against the tree at the moment of writing.
- No schema or version constant is bumped, no struct field is added; the struct-literal churn gate and blast-radius discipline are not engaged.

## Data and Contract Notes

- IR/manifest contracts: none changed. No config key, no manifest section, no snake_case key is touched.
- WIT boundary: none crossed. This packet describes how WIT freshness is verified; it neither reads nor edits any `.wit` file.
- Determinism/scheduler constraints: none. The only executable change is a CI step and, conditionally, a test skip-guard.
- CI contract: the new `cargo test -p xtask` step must sit after `Install wasm-tools` **and** after `Build guest WASMs` (`cargo xtask build-guests`) in the `test` job — its real-artifact tests hard-panic on a missing artifact (see `xtask/src/wit_verify.rs`'s test guards), and a fresh checkout contains none (`.gitignore` line 11 `*.wasm`). Both the `test` and `dist-editions` jobs install `wasm-tools` via `taiki-e/install-action`, so `tool: wasm-tools` occurs twice in the file; AC-16's ordering check reads the **first** match, which is the `test` job's, and compares the `cargo test -p xtask` line number against the first `run: cargo xtask build-guests` line. If a future edit reorders the jobs or the steps, that check needs revisiting — state this in the step's exit condition.

## Locked Assumptions and Invariants

- **Exactly one packet owns ADR-0054, and it is 231.** Packet 231 discharges Decision rule 5 by conforming the `crates/pnp-cli-locator::staleness_reason` rustdoc to the new `is_stale` model — a conformance, not an amendment, since none of ADR-0054's five normative rules changes. This packet therefore does not open, amend or reference-edit ADR-0054, and AC-N3 enforces that mechanically.
- **This packet owns ADR-0014 and ADR-0045.** Neither is amended by 229, 230 or 231.
- Freshness is asserted by **exit code**, never by grepping for `STALE:` (R5-3). Every artifact this packet writes — the snippet, the skill bullet, `CLAUDE.md`, `docs/03`, `docs/05` — states it that way, and the packet's own verification matrix obeys it.
- There are **two independent freshness gates**, not one: the xtask artifact gate and the host-side contract test. The docs must never again present them as one model.
- Existing packet directories are frozen (user-ruled 2026-08-19), including the six carrying grep-form ACs. Those ACs remain unsound; that is an accepted, recorded cost, not an oversight.

## Risks and Tradeoffs

- **Doc-grep ACs verify text, not truth.** Every AC here is a grep, so a rewrite that is fluent and wrong passes. Mitigated structurally: `AC-N4` sweeps for deleted symbols across the whole edited surface, and `requirements.md` §Step Completion Expectations requires each statement to be written against the tree as 229-231 left it, with a `FACT` dispatch behind the exit-code contract before any prose states it.
- **Eleven files exceed the three-file target.** Accepted and justified above; contained by per-step caps and anchor-bounded edits.
- **Prose may be written before the behaviour lands.** Guarded by the activation blocker: packets 230 and 231 must be `status: implemented` first.
- **CI cost.** Adding `cargo test -p xtask` lengthens the `test` job. **Unmeasured**; `xtask` is a small bin-only crate with no heavy dependencies (`walkdir`, `toml`, `syn`, `proc-macro2`, `slicer-schema`, plus `wit-parser` from packet 229), and it is already compiled by the job's `cargo build --workspace` step. Do not quote a figure that was not measured on a real run.
- **`AC-16`'s ordering checks are positional.** They read the first `tool: wasm-tools` occurrence and the first `run: cargo xtask build-guests` line, which are the `test` job's today. A job reorder would silently change what they prove. Measured in closure review (2026-08-20): the originally shipped step sat between `Install wasm-tools` and `Build guest WASMs`, so on any fresh runner `cargo test -p xtask` would have hard-failed — three real-artifact tests panic on absent artifacts (`xtask/src/wit_verify.rs`). Fixed by moving the step after the guest build and tightening AC-16's command to assert both orderings.
