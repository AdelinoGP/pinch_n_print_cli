# 151 — Audit merge residue and bring `orca-feature-gaps` branch gates green

Type: task
Status: open
Assignee: —
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
