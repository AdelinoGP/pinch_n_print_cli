# 151 — Audit merge residue and bring `orca-feature-gaps` branch gates green

Type: task
Status: resolved
Assignee: wayfinder session (2026-11-26)
Blocked by: —
Map: ../map.md

## Question

After a two-month pause (`fb58aaf6`, 2026-09-11), `orca-feature-gaps` has twice merged `origin/master` (through `f899cdf2`, 2026-11-23), pulling in `config-scope-resolution` (ADRs 0067–0070), `layer-module-preparation` (ADRs 0066, 0071, 0073, 0074), `test-quality-remediation` (ADRs 0064–0065), `perf-vs-orca` (ADR-0072), and `proptest-openrscad`.

At filing (`f899cdf2`), `cargo check --workspace --all-targets` and `cargo xtask build-guests --check` both exit `0`, **but no test, clippy, struct-literal, or test-quality gate has been run**, and visible merge residue exists in the branch diff against `origin/master` (re-derive from disk at claim time):

1. **`overlay_resolved` residue (`crates/slicer-core/src/algos/region_mapping.rs`).** `origin/master` retired `overlay_resolved` in `config-scope-resolution_05_scope-resolution-module` in favour of `resolve_scope_stack` (`crates/slicer-config/src/resolution.rs`), yet `overlay_resolved` is still defined and called in `crates/slicer-core/src/algos/region_mapping.rs` on this branch. Verify whether it is live dual-path residue or genuinely required, and reconcile with `crates/slicer-config/src/resolution.rs`.
2. **`ConfigBoundsIndex` / `crates/slicer-scheduler/src/config_resolution.rs` divergence.** Inspect the diff between `origin/master` and `HEAD` across `crates/slicer-config/src/resolution.rs` and `crates/slicer-scheduler/src/{config_resolution.rs,lib.rs,manifest.rs,validation.rs}` to confirm no split-brain validation or resolution path survived the merge.
3. **`support_base_pattern` tension.** Check the post-merge state of `support_base_pattern` across manifests, `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`, and `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` against open ticket [135 — Author packet — support base-pattern holder](./135-author-packet-support-base-pattern-holder.md) and the map's holder-only rule.
4. **Full branch gate pass.** Run the required gates on `orca-feature-gaps` (capturing output to `target/test-output.log` per `AGENTS.md` § Test Discipline):
   - `cargo xtask build-guests --check` (verify exit code `0`, rebuild if exit `1`)
   - `cargo xtask check-literals`
   - `cargo xtask check-fmt`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo xtask check-test-quality --report` (fix or justify findings in branch-touched tests per `docs/22_test_quality.md`)
   - `cargo xtask test --summary --workspace` (dispatch to a subagent returning `FACT pass/fail` if needed, or run targeted crate suites first and finish with `--workspace` once clean)

### Scope boundary

- **Stay on `orca-feature-gaps`.** Do **not** open an upstream PR to `master` (user ruling 2026-11-23: keep branch code here for now).
- **Merge integrity and gate health only.** Fix merge residue, broken tests, clippy/literal/formatting errors, and false-green test-quality defects in touched code. Do **not** rewrite passing behavioural features or re-adjudicate "canonicity" design choices — those belong to [152 — Deep audit of non-rename behavioural and "canonicity" commits on `orca-feature-gaps`](./152-deep-audit-non-rename-branch-behaviour-commits.md) and [153 — Rule on questionable "canonicity" and superseded branch commits](./153-rule-on-questionable-canonicity-and-superseded-branch-commits.md).

### Obligations

- Re-derive every diff and file count from `git diff origin/master..HEAD` at claim time.
- In-tree citations by symbol name + crate-qualified path; never bare line numbers.
- Resolved when every gate above exits `0` (with `check-test-quality --report` clean on branch-touched code or explicitly waived), merge residue findings are fixed and committed, and the answer records the verified commit SHA, gate commands, and any follow-on issues surfaced for [152](./152-deep-audit-non-rename-branch-behaviour-commits.md) or [156](./156-reconcile-map-notes-stale-warnings-and-remaining-open-tickets.md).

## Answer

Resolved 2026-11-26 (wayfinder session) on the merged tree. Gates were run at
HEAD `5b9d6147`, which is `origin/master` `4dcc2e32` plus the ticket-163 docs
commit; no code differs between the two.

| gate | result |
|---|---|
| `cargo xtask build-guests --check` | exit `0` (no stale guest, no lock divergence) |
| `cargo xtask check-literals` | 0 violations (watchlist: 201 types) |
| `cargo xtask check-fmt` | 62 packages clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit `0` |
| `cargo xtask check-test-quality --report` | 0 findings in 0 files, tree-wide |
| workspace suite | not re-run locally in this session. PR #11's body records `cargo xtask test --summary --workspace --no-fail-fast -- --skip proptest_direct_` as PASS (498 test binaries, 5,926 passed, 15 ignored, 0 failed) plus `--properties --tier routine` PASS on `14661406`; `git diff --stat 14661406 4dcc2e32` is empty, so that is the merged tree byte-for-byte. CI `Test` is green on the same head. |

Logs: `target/build-guests-check.log`, `target/check-literals.log`,
`target/check-fmt.log`, `target/clippy.log`, `target/check-test-quality.log`.

Merge-residue items:

1. **`overlay_resolved`** — still defined and called in
   `crates/slicer-core/src/algos/region_mapping.rs`, reachable only from the
   `host_config: None` compatibility path; production always supplies a host
   config authority and `resolve_scope_stack` owns precedence. Ticket 153
   ruled the deletion into
   [162](162-delete-production-dead-overlay-engine.md) as the immediate
   post-merge item, so 151 does not fix it here by design.
2. **`ConfigBoundsIndex` / `config_resolution.rs`** — no split brain survived.
   `resolve_config` (`crates/slicer-scheduler/src/config_resolution.rs`) is a
   thin adapter: it ingests through `ingest_resolution_config` and then
   delegates to `slicer_config::resolve_scope_stack`; production
   `crates/slicer-runtime/src/run.rs` calls `resolve_scope_stack` directly.
   `ConfigBoundsIndex` supplies the schema registry and numeric bounds to
   both, so there is one resolution engine.
3. **`support_base_pattern`** — the post-merge state matches ticket 135's
   ruling. The manifest declaration and value set stay in
   `traditional-support-planner.toml`; the planner records
   `traditional-base-pattern:<value>` in `SupportPlanEntry::capabilities`
   (pinned by `traditional_family_tdd`); the 3MF sidecar test asserts the key
   is **dropped** from object metadata, i.e. no alias; and
   `support_base_pattern_spacing` remains a live renderer read. The missing
   holder/algorithm seam stays with
   [135](135-author-packet-support-base-pattern-holder.md).

Scope boundary note: the "stay on `orca-feature-gaps`, no upstream PR" clause
was superseded on 2026-11-26 by the PR #11 merge (`4dcc2e32`); this ticket was
resolved after that merge, so branch health and merge integrity are verified on
the merged tree rather than on a pre-merge branch tip. Nothing remains for
[152](152-asset-branch-behaviour-audit.md); the stale `⚠` badges and the
remaining Notes reconciliation stay with
[156](156-reconcile-map-notes-stale-warnings-and-remaining-open-tickets.md).
