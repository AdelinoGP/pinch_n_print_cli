# 152 — Deep audit of non-rename behavioural and "canonicity" commits on `orca-feature-gaps`

Type: research
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

**Which non-rename behavioural commits unique to `orca-feature-gaps` contain questionable changes made in the name of "canonicity", duplicate or conflict with newer `origin/master` architecture, or carry older-LLM quality / test-quality defects?**

User scoping note (2026-11-23): while most unique commits on `orca-feature-gaps` are mechanical key renames (tickets `99`–`107`), a smaller population of direct-implementation and fix commits changed runtime behaviour, config ingestion/resolution, dispatch, geometry, or test baselines — authored by earlier LLM sessions before `config-scope-resolution`, `layer-module-preparation`, `perf-vs-orca`, and `docs/22_test_quality.md`. A deep, evidence-backed audit of those non-rename commits is needed before resuming feature work.

### Scope

1. **Enumerate the target commit set from git at claim time** (`git log --no-merges --oneline origin/master..HEAD -- crates modules`). Exclude the pure key-rename commits (tickets `99`–`107`), **except** any non-rename behavioural rider bundled with them (for example `4cdf5692`, which changed WASM `fatal=false` handling and re-baselined 4 tests alongside ticket `102`).
2. **Audit every non-rename behavioural commit** — including direct implementations and fixes from tickets `26`–`50`, `91`–`92`, `111`–`117`, `126`, the five-way fill partition (`71a4c832`), fill-claim holder validation (`ac035ed8`), staggered inner seams (`b6df1a6b`), wipe-tower purge speed / bed bounds (`dc2584bb`, `49dde38b`), support ironing gate (`951b7ef1`), JSON integer sidecar fix (`148b76fb`), vector-tolerant ingest (`44e10bcb`), and post-merge fixup (`e9cca240`). Re-derive the full list from git rather than trusting this sample.
3. **Evaluate each commit along four axes:**
   - **Master supersession or conflict:** Does `origin/master`'s `config-scope-resolution` (ADRs 0067–0070), `layer-module-preparation` (ADRs 0066, 0071, 0073, 0074), or ADR-0072 feedrate contract supersede, duplicate, or fight this commit? (Known starting point: `202f812d` / ticket `126`'s `overlay_resolved` work vs `resolve_scope_stack` in `crates/slicer-config/src/resolution.rs`.)
   - **"Canonicity" sanity & PnP fit:** Did the commit import OrcaSlicer coupling, silent fallbacks, widened defaults, unrequested behaviour shifts, or questionable baseline changes "for canonical parity" when a cleaner PnP-native answer exists (per `docs/00_project_overview.md` and Authoring rule 4)? Verify claims against the designated canonical oracle (`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`, per map Notes) by file + function name, never line numbers.
   - **Code & contract correctness:** Check for silent drops, coordinate-unit hazards (`1 unit = 100 nm`), undeclared manifest reads, or missing production call sites.
   - **Test quality (`docs/22_test_quality.md`):** Flag any false-green patterns (self-referential oracle, `assert_ne!`-only liveness without conservation/behaviour check, weakened assertions, or re-baselined tests that hid regressions).

### Deliverable

- A linked markdown inventory asset (`./152-asset-branch-behaviour-audit.md`) classifying **every** audited commit into one of four buckets with concrete symbol-level evidence:
  - `keep` — sound, PnP-idiomatic, compatible with current `master`, tests falsify the claim
  - `questionable-for-ruling` — debatable "canonicity" trade-off, default flip, or design shortcut requiring a human decision in [153 — Rule on questionable "canonicity" and superseded branch commits](./153-rule-on-questionable-canonicity-and-superseded-branch-commits.md)
  - `superseded-by-master` — replaced or made redundant by `origin/master` workstreams and candidates for removal/unification
  - `defective` — concrete bug, silent data loss, or false-green test needing a direct fix
- **Read-only research ticket:** do not edit code or revert commits here. [153](./153-rule-on-questionable-canonicity-and-superseded-branch-commits.md) carries the human rulings and graduates any code-remediation tasks.

### Obligations

- Re-derive commit lists and file paths from disk at claim time.
- In-tree citations by symbol name + crate-qualified path; OrcaSlicer citations by file + function.
- Resolved when the asset is written, linked from `## Answer`, and summarises counts per bucket plus the top ruling items for [153](./153-rule-on-questionable-canonicity-and-superseded-branch-commits.md).

## Answer
