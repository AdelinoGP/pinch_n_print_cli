# Design: 232-freshness-gate-docs

## Controlling Code Paths

- Primary code path: none. The behaviour this packet describes lives in `check_command` / `stale_reason` / `guest_closure_input_paths` (`xtask/src/build_guests.rs`) and `compare_worlds` / `embedded_world_model` (`xtask/src/wit_verify.rs`), all shipped by packets 229-231 and read-only here.
- Only executable surface touched: the test skip-guards in `xtask/src/wit_verify.rs`'s `#[cfg(test)] mod tests`, and `.github/workflows/ci.yml`'s `test` job.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` — the second, host-side freshness gate. It is **documented, never edited**. Its `GUESTS` constant lists 8 test guests (measured 2026-08-19: `layer-infill-guest`, `prepass-guest`, `finalization-guest`, `postpass-guest`, `sdk-postpass-text-guest`, `sdk-finalization-guest`, `sdk-prepass-guest`, `sdk-layer-infill-guest`) and its `guest_components_are_not_stale` applies its own rule — a guest's `src/lib.rs` newer than the artifact. Note for the implementer: the plan text says "10 test guests"; the tree says 8. Re-count before writing the number into `docs/03`, and write whatever the tree says.
- OrcaSlicer comparison: not applicable; no parity surface. No OrcaSlicer obligation section appears in `packet.spec.md` or `requirements.md`.

## Architecture Constraints

- The `wasm-staleness` snippet is **deliberately omitted from this design's constraint list**, even though this packet *rewrites* that snippet. Its applies-to list is `crates/slicer-schema/wit/**`, the four (now five) shared crates, `modules/core-modules/*/src|Cargo.toml|wit-guest`, and `crates/slicer-wasm-host/test-guests/*/src|Cargo.toml`. This packet's change surface is documentation, `.claude/skills/**`, `.github/workflows/ci.yml` and the test module of `xtask/src/wit_verify.rs` — none of which feeds a guest `.wasm`. Quoting the snippet as an obligation here would assert a rebuild duty that does not exist, in the very packet that fixes how the obligation is stated.
- The `coord-system` snippet does not apply: no geometry, no mm/unit conversion.
- The rewritten snippet must remain **verbatim-or-absent** downstream: it retains its `<!-- snippet: wasm-staleness -->` marker and its "copy exactly; do not paraphrase" framing, because `spec-packet-generator` self-review and `spec-review` both check the block for exactness. Changing the marker would silently disable both checks.
- Every doc edit is subject to `CLAUDE.md` §"In-Tree Citation Style (MUST follow)": cite by symbol name with the crate-qualified path. Two of the defects being repaired here (`stage_wit_mtime` in `docs/07`, `compute_shared_mtime` in ADR-0045) are symbol-name fabrications, not line-number rot, so the fix is to verify each symbol against the tree at the moment of writing.
- No schema or version constant is bumped, no struct field is added; the struct-literal churn gate and blast-radius discipline are not engaged.

## Code Change Surface

Selected approach: one edit per doc surface, grouped into steps by file so no step exceeds three edits, with a final cross-file sweep that no deleted symbol survives as a live citation.

Edits, by file and anchor:

1. `CLAUDE.md` — section "## Guest WASM Staleness (MUST follow)", which runs from that heading to the next `##` heading ("## WIT/Type Changes Checklist"). Replace the mtime framing and the hand-maintained input-path list with: `--check` decodes each artifact and compares its embedded WIT world against canonical; the fingerprint covers code inputs only and is derived per guest from its Cargo path-dependency closure; the answer is the **exit code**, with a distinct non-zero code for a missing `wasm-tools`. Retain the enforcement clauses ("Prohibited claims unless `--check` was just run and returned clean") and the shared-target-dir note about `crates/slicer-wasm-host/test-guests/target/`. Dispose of the 2026-07-25 `slicer-core` anecdote per AC-3.
2. `docs/03_wit_and_manifest.md` — the staleness-guard table row whose cells are `` `cargo xtask build-guests --check` `` and `Stale in-tree guest (mtime-based)`; and section "### Build & Freshness Contract (Normative)", both the `--check` bullet and the paragraph that introduces `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs`. The latter currently flows as if one model covered both gates; it must state two independent gates and attribute each rule to its owner.
3. `docs/05_module_sdk.md` — the "**Guest rebuild obligation.**" bullet's closing sentence.
4. `docs/07_implementation_status.md` — two edits: the TASK-146b row's `stage_wit_mtime` and `compute_shared_mtime` strings, and the appended `TASK-343` row under "### Workstream 5 — Governance and closure drift".
5. `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — a new nested `###` entry under the existing `## Amendments` heading (its current entry is `### Guest freshness amendment (2026-08-01)`, which names `xtask/src/build_guests.rs::shared_crates`), plus the Consequences bullet "**Freshness is precise without being conservative.** Touching `slicer-core` does not trigger a guest rebuild storm; touching the WIT files does."
6. `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` — three `compute_shared_mtime` occurrences (one prose paragraph, two table cells) and a new `## Amendment — <YYYY-MM-DD> (packets 229/230/231)` section, matching the file's existing `## Amendment — 2026-08-05 (packets 163/164)` house style.
7. `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md` — rewrite the copy-exactly block to an exit-code contract; keep the marker, the front-matter `when:`/`keywords:` shape, and the applies-to list (updated to include `crates/slicer-core/**`, which the tree treats as a universal guest input and which the snippet's list currently omits).
8. `.claude/skills/spec-review/SKILL.md` — "Known traps" item 4, "**Stale guest WASM**".
9. `CONTEXT.md` — append `### Artifact-verified freshness` under `## Terms`, in the file's `### <Term>` + plain-prose form.
10. `.github/workflows/ci.yml` — the `test` job: add a step running `cargo test -p xtask`, placed after `Install wasm-tools` and before or beside "Test All Crates (not workspace)".
11. `xtask/src/wit_verify.rs` — conditional. At authoring time the file's test module carried five `eprintln!("skipping…")` early-returns; packet 229 rewrites the file and its AC-11 requires the real-artifact test to assert whenever `wasm-tools` resolves and the artifact exists. If any skip-on-present path survives 229, convert it to a hard failure here. If none survives, record the verified no-op in the step's exit condition rather than editing the file.

Rejected alternatives:

- **Fold these edits into packets 229-231, one doc change per behaviour change.** Rejected: the nine statements are one statement, and splitting them across three packets guarantees an interval in which `CLAUDE.md` and `docs/03` disagree — a worse failure than the current uniform staleness, because the disagreement looks like a deliberate distinction.
- **Also update the six packets whose ACs use the grep form.** Rejected by explicit user ruling of 2026-08-19: existing packets are left untouched.
- **Fix `build_script_check_mode_reports_freshness` here.** Rejected: it is a test-logic change, and putting one behind a documentation packet's doc-grep gate is exactly the kind of unverified drive-by this repo's test discipline forbids. Reported in `requirements.md` §"Reported, not fixed".
- **Delete the 2026-07-25 anecdote silently.** Discouraged but permitted by AC-3. The anecdote records a real, measured failure whose *lesson* survives the mechanism change (a freshness input set that must be maintained by hand will be wrong); if kept, it must be explicitly historical.

## Files in Scope (read + edit)

Eleven files, which exceeds the "at most 3 primary" target. The justification is that this packet's unit of work **is** the file count: a single contract stated in nine places plus one CI file plus one conditional test-guard. Splitting it by file would produce packets that individually leave the tree self-contradictory. The cost is contained by making every edit a bounded anchor replacement — no file is read whole — and by capping each implementation step at three files.

- `CLAUDE.md` — role: the normative agent-facing statement of the freshness rule; expected change: one section rewritten.
- `docs/03_wit_and_manifest.md` — role: the normative build-and-freshness contract; expected change: one table cell, one bullet, one paragraph.
- `docs/05_module_sdk.md` — role: SDK-author-facing rebuild obligation; expected change: one sentence.
- `docs/07_implementation_status.md` — role: backlog ledger and TASK-146b's historical record; expected change: two symbol repins plus one appended row.
- `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — role: guest-discovery decision record carrying the `shared_crates` amendment; expected change: one nested amendment entry, one Consequences bullet.
- `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` — role: per-stage WIT decision record; expected change: three symbol repins, one new amendment section.
- `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md` — role: the canonical obligation text every future packet copies; expected change: exit-code rewrite, marker preserved.
- `.claude/skills/spec-review/SKILL.md` — role: reviewer trap list; expected change: one bullet.
- `CONTEXT.md` — role: shared glossary; expected change: one appended term.
- `.github/workflows/ci.yml` — role: CI gate; expected change: one added step in the `test` job.
- `xtask/src/wit_verify.rs` — role: the verifier's own tests; expected change: conditional skip-guard hardening, possibly none.

## Read-Only Context

- `docs/03_wit_and_manifest.md` — very long; read only the staleness-guard table (the row listing `cargo xtask build-guests --check`) and "### Build & Freshness Contract (Normative)". Locate with `rg -n`, then a +/-40-line window.
- `docs/05_module_sdk.md` — very long; read only the "**Guest rebuild obligation.**" bullet's window.
- `docs/07_implementation_status.md` — long, and its TASK-146b row is a single very long line; delegate both edits, never read the file.
- `docs/adr/0045-...md` — long; read only the windows around the three `compute_shared_mtime` occurrences and around `## Amendment — 2026-08-05 (packets 163/164)` for house style.
- `CONTEXT.md` — long; read only lines 1-10.
- `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` — short; read `GUESTS`, `guest_components_are_not_stale`, and `build_script_check_mode_reports_freshness`. Read-only: never edited by this packet.
- `docs/specs/guest-freshness-artifact-verification-plan.md` — moderate; read the CONTEXT.md term wording, C11, and R5-3 / R5-10 / R5-12.
- `xtask/src/build_guests.rs` and `xtask/src/wit_verify.rs` — read only what is needed to state the shipped behaviour accurately: the exit-code constants, `check_command`'s reporting, and the closure-walk entry point.

## Out-of-Bounds Files

- `docs/adr/0054-host-side-test-support-crate.md` — packet 231 owns it. AC-N3 asserts it is unchanged here.
- Every `docs/spec_packets/` directory other than `232-freshness-gate-docs/`, explicitly including `206`, `207`, `209`, `210a`, `210b`, `211`, `212`, `229`, `230`, `231` (user-ruled 2026-08-19). AC-N1 asserts it.
- Everything under `crates/` — AC-N2 asserts no file there changes, including the host-side gate test.
- `xtask/src/build_guests.rs`, `xtask/src/test.rs`, `xtask/src/main.rs`, `xtask/src/dist.rs` — read-only; AC-N2 permits only `xtask/src/wit_verify.rs` to change.
- `docs/11_operational_governance_and_acceptance_gate.md` — contains no `build-guests` mention; dropped from the surface by Round 5. Do not open it looking for one.
- `target/`, `Cargo.lock`, the 42 `.wasm` artifacts, generated code, vendored dependencies — never loaded.
- `OrcaSlicerDocumented/...` — not applicable; never loaded.

## Expected Sub-Agent Dispatches

- Question: in `docs/07_implementation_status.md`, replace `stage_wit_mtime(ws_root, stage_id)` with the real symbol `stage_wit_snapshot` marked retired by packet 231, and `compute_shared_mtime` with `compute_shared_freshness` marked retired; then append the `TASK-343` row under "### Workstream 5 — Governance and closure drift" in that section's terser local format. scope: `docs/07_implementation_status.md`; return: `FACT pass/fail` plus the three confirming greps; purpose: Steps 3 and 7.
- Question: what exit-code constants and reporting behaviour did packet 230 actually ship in `check_command` — names, values, and what is printed in each case? scope: `xtask/src/build_guests.rs`; return: `FACT` (<=5 lines); purpose: Steps 1, 2 and 5, which all state the contract in prose.
- Question: after packets 229/230, does any test in `xtask/src/wit_verify.rs` still print `skipping` and return early on a path reachable when `wasm-tools` resolves on `PATH` and the artifact exists? scope: `xtask/src/wit_verify.rs`; return: `LOCATIONS` (<=20 entries); purpose: Step 6's conditional edit.
- Question: how many entries does `GUESTS` in `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` have, and what rule does `guest_components_are_not_stale` apply? scope: that file; return: `FACT` (<=5 lines); purpose: Step 2's two-gates paragraph — the count must come from the tree, not from this packet.
- Question: re-derive the highest `TASK-###` in `docs/07_implementation_status.md` and the highest numbered directory under `docs/spec_packets/`. scope: those paths; return: `FACT` (2 lines); purpose: ledger-fact re-derivation before Step 7 writes `TASK-343`.

## Data and Contract Notes

- IR/manifest contracts: none changed. No config key, no manifest section, no snake_case key is touched.
- WIT boundary: none crossed. This packet describes how WIT freshness is verified; it neither reads nor edits any `.wit` file.
- Determinism/scheduler constraints: none. The only executable change is a CI step and, conditionally, a test skip-guard.
- CI contract: the new `cargo test -p xtask` step must sit after `Install wasm-tools` in the `test` job. Both the `test` and `dist-editions` jobs install `wasm-tools` via `taiki-e/install-action`, so `tool: wasm-tools` occurs twice in the file; AC-16's ordering check reads the **first** match, which is the `test` job's. If a future edit reorders the jobs, that check needs revisiting — state this in the step's exit condition.

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
- **`AC-16`'s ordering check is positional.** It reads the first `tool: wasm-tools` occurrence, which is the `test` job's today. A job reorder would silently change what it proves.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — two anchors in a very long doc plus a reconciliation paragraph that must be grounded in a second file's real contents)
- Highest-risk dispatch and required return format: the `docs/07_implementation_status.md` edit — `FACT pass/fail` plus three greps. Its TASK-146b row is a single ~2500-character line; absorbing it into the controller's context would cost more than the rest of the packet.

## Open Questions

- **[FWD]** The plan states the host-side gate hardcodes "10 test guests"; the tree's `GUESTS` constant has **8** entries (measured 2026-08-19). Write the tree's count into `docs/03`, re-counted at the moment of writing, and note the plan's figure as superseded. Do not reconcile by editing the test.
- **[FWD]** Whether the 2026-07-25 `slicer-core` anecdote is deleted or kept under historical framing is left to the implementer; AC-3 accepts either. Recommendation: keep the *lesson* (a hand-maintained input set will be wrong) in one sentence without the mechanism detail, since the mechanism no longer exists.
- **[FWD]** Step 6's edit to `xtask/src/wit_verify.rs` is conditional on what packet 229 shipped. If the file is already clean, record the verified no-op; do not manufacture an edit to satisfy AC-17, whose command is a test run and passes either way.
- **[FWD]** The `wasm-staleness` snippet's applies-to list omits `crates/slicer-core/**`, which `CLAUDE.md` names as a guest-WASM input path and which `shared_crates` tracked from 2026-07-25. Add it during the rewrite. If the implementer finds that packet 231's closure walk makes a static applies-to list redundant, say so in the snippet rather than dropping the list silently — downstream packets use it to decide whether the obligation applies at all.
