---
status: implemented
packet: 254-exact-perimeter-spatial-queries
task_ids:
  - TASK-561
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Draft authored from the confirmed TASK-561 design decisions; refined 2026-09-10 after a PREFLIGHT BLOCKED review (doc-slot collision, wrong file path, vacuous doc greps, feature plumbing, runner location, check-cfg owner, edit-cap splits). Implementation and acceptance evidence remain downstream.
---

# Packet Contract: exact-perimeter-spatial-queries

## Goal

Add exact, independently-oracled shared per-region spatial query contexts for Classic and Arachne perimeter generation, and measure them only through a reproducible controlled Rust compiler mode.

## Scope Boundaries

This packet covers boundary distance/sign, overhang quartile, and bridge candidate pruning, their reuse across every relevant generator pass, exact fallback behavior, native/WASM capture and self-baseline fixtures, and the controlled `RUSTC` driver and mode-aware guest artifacts. It does not change WIT, IR, scheduler, public host execution structs, or the legacy evaluators.

## Prerequisites and Blockers

- Depends on: existing perimeter generators (`ClassicPerimeters`, `ArachnePerimeters`), the runtime perimeter harness (`PerimeterCapturingLayerStageRunner`) and integrated-parity harness (`run_integrated_parity`, `__slicer_native_entry()`), guest freshness machinery, and the existing `rstar = "0.12"` dependency of `slicer-core`.
- Unblocks: implementation of TASK-561 and its controlled KEEP/DROP/inconclusive acceptance campaign.
- Toolchain: the driver allowlist (design.md) matches the machine that authored this packet (`rustc -vV` on 2026-09-10: 1.96.0, commit `ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96`, LLVM 22.1.2, host `x86_64-pc-windows-msvc`). No `rust-toolchain.toml` pin exists in the repo; the driver re-verifies identity at every invocation and rejects drift rather than relying on a pin.
- Local corpus: `tmp/rtree_query_corpus/` is **absent today**. It is needed only by AC-4 (Step 12 campaign); Steps 1-11, the runner dry-run, and the validator test are corpus-free. The user prepares the corpus per `docs/23_controlled_perimeter_builds.md` §Corpus before the campaign; its absence blocks AC-4 only, not activation.
- Activation blockers: none; status intentionally remains `draft` pending packet preflight and user activation.

## Acceptance Criteria

- **AC-1. Given** finite and adversarial synthetic regions containing contour/hole/closing edges, equal-distance candidates, signed zero, degenerate/empty rings, negative coordinates, computed-endpoint drift, widths, and distance normalization, **when** the accelerated native query mode is run against the independent legacy evaluator, **then** distance `Option` presence, signed-zero bits, selected source ordinal, `distance + 0.5 * width`, sign, quartile maximum, bridge strict-boundary result, flags, and closure output bits are identical for every query. | `cargo test -p slicer-core --features host-algos,perimeter-spatial-test-support --test perimeter_spatial_tdd -- exact_queries_match_legacy --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** a region with more than `rstar::DefaultParams::MAX_SIZE` relevant records and multiple Classic/Arachne passes, including Classic nonplanar shells and Arachne's only-one-wall-top second pass, **when** each generator runs natively in-process through `slicer_runtime::LayerStageRunner::run_stage` with its `__slicer_native_entry()` (the path the existing `integrated_parity_classic_perimeters_tdd` / `integrated_parity_arachne_perimeters_tdd` contract tests already drive) under `--features perimeter-spatial-test-support` with thread-scoped capture enabled and the indexed mode selected, **then** the scoped diagnostics record region identity, pass identity, nonempty perimeter output, and query counters showing every relevant pass reused exactly one immutable four-index context, with exact evaluations below the separated synthetic candidate count. A run of the same binary without the feature must fail with an explicit `perimeter-spatial-test-support feature required` panic from a single always-compiled guard test, never report zero tests. | `cargo test -p slicer-runtime --features perimeter-spatial-test-support --test integration -- perimeter_spatial_capture_and_nonvacuity --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** ordinary builds, controlled host release/debug builds, and controlled Classic/Arachne guest builds, **when** mode-aware build/dist/test commands and freshness checks are run, **then** ordinary builds use the legacy path, accelerated artifacts are isolated by policy/compiler/mode identity, optimized-core debug retains debuginfo and assertions while `slicer-core` uses opt-level 3, guests remain release, and unchanged WIT/code artifacts report the documented freshness result (exit 0/1/3) without accepting an opposite-mode artifact. | `cargo test -p xtask --bin xtask -- accelerated_mode_freshness --nocapture 2>&1 | tee target/test-output.log`
- **AC-3N. Given** the accelerated feature is compiled into the core query path, **when** the same parity fixtures run under the accelerated context and then again with the accelerated mode disabled, **then** both modes produce identical legacy-oracle results AND the test-only counters prove the accelerated mode exercised the indexed path and the ordinary mode exercised the legacy path (no silent fallback, no vacuous pass). | `cargo test -p slicer-core --features host-algos,perimeter-spatial-test-support --test perimeter_spatial_tdd -- accelerated_mode_exercised_not_vacuous --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** the accepted toolchain and a local corpus at `tmp/rtree_query_corpus/` (absent today; prepared by the user per `docs/23_controlled_perimeter_builds.md` §Corpus before this AC can run) holding a valid supports-off Benchy, tree-support Benchy, and original tree-support base, **when** `run-acceptance.ps1 -Campaign` runs all six workload×generator cells serially, each as ABBA then BAAB with one warmup and four measured samples per variant at 12 threads, **then** exactness passes first, every cell has `max(candidate CPU samples) < min(baseline CPU samples)` and `max(candidate wall samples) < min(baseline wall samples)`, `summary.json` lists all six cells with both ratios and provenance, and any overlap, exactness failure, generator-marker disagreement, or degraded/fatal status change stops the campaign as `DROP` or `inconclusive` and is never reported as `KEEP`. Known pre-existing campaign conditions (e.g. base tree-support's recorded 29,108 non-fatal errors with `degraded: true`) must be IDENTICAL between baseline and candidate samples and remain visible in every retained row; they do not by themselves reject a sample, but any change in them is a generator disagreement. | `pwsh -NoProfile -File resources/perimeter-acceptance/run-acceptance.ps1 -Campaign 2>&1 | tee target/test-output.log; python3 -c "import json; d=json.load(open('target/perimeter-acceptance/summary.json')); assert d['status'] in ('KEEP','DROP','inconclusive') and d['automatic_commit'] is False and d['threads']==12 and d['samples_per_cell']==4 and d['warmup_per_cell']==1 and len(d['cells'])==6 and (d['status']!='KEEP' or all(c['cpu_separated'] and c['wall_separated'] for c in d['cells']))"`
- **AC-5. Given** the committed deterministic perimeter fixtures under `crates/slicer-runtime/tests/fixtures/perimeter_spatial/` and their recorded per-mode baselines, **when** the native in-process pipeline runs in indexed and in legacy scoped mode, and the WASM pipeline runs through `PerimeterCapturingLayerStageRunner` with the prepared-region capture hook active, **then** each mode's complete `PerimeterIR` output equals its own preserved baseline bit-for-bit (postcard bytes), the WASM run captures at least two filtered prepared regions whose identities match the native run's region identities, and no native-vs-WASM output comparison is made or reported as evidence. | `cargo xtask test --summary -p slicer-runtime --features perimeter-spatial-test-support --test integration -- perimeter_spatial_self_baseline --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a shim invocation containing an unknown/duplicate keyed control, response file, unaudited LLVM/sysroot/backend/custom-target option, unsupported target, unoptimized accelerated core, or a failed policy validation later swallowed by a child process, **when** accelerated mode is requested, **then** the owned `RUSTC` shim rejects publication, latches the session rejection, and returns a nonzero policy failure; normal mode remains linear and does not receive the reserved cfg. | `cargo test -p xtask --bin xtask -- accelerated_policy_rejection_latch --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a nonfinite query, finite-input guard failure, bridge overflow bound failure, empty/edge-less boundary, or structural small set, **when** the relevant query path is invoked, **then** it performs the complete source-order legacy evaluation before any pruning, the test-only counters prove the legacy path ran (fallback counter incremented, indexed counter unchanged), and a fault-injected pruning assertion demonstrates that the fallback is real rather than a plausible answer. | `cargo test -p slicer-core --features host-algos,perimeter-spatial-test-support --test perimeter_spatial_tdd -- fallback_paths_are_nonvacuous --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** overlapping CPU/wall ranges, exactness failure, generator-marker/config disagreement, or a fatal/nonfatal/degraded campaign status change, **when** an acceptance stage completes, **then** the campaign stops without extra runs and reports `DROP` or `inconclusive` with every retained sample and ratio; it never reports `KEEP` or commits automatically. **And given** a cell run is requested with the corpus root, a model, or a config absent, **then** the runner exits nonzero before any build or timing with one `missing-artifact: <path>` line naming the first missing path. | `cargo test -p slicer-runtime --test integration -- overlap_is_inconclusive_and_never_keep --nocapture 2>&1 | tee target/test-output.log; pwsh -NoProfile -File resources/perimeter-acceptance/run-acceptance.ps1 -Workload supports-off-benchy -ExpectedGenerator classic -CorpusRoot target/perimeter-acceptance/absent-corpus 2>&1 | tee -a target/test-output.log; test "${PIPESTATUS[0]}" -ne 0 && rg -q '^missing-artifact: ' target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo xtask build-guests --check` (judge by exit code: 0 fresh, 1 stale → rebuild, 3 infrastructure error → stop)
- `cargo test -p slicer-core --features host-algos,perimeter-spatial-test-support --test perimeter_spatial_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --features perimeter-spatial-test-support --test integration -- perimeter_spatial_ --nocapture 2>&1 | tee target/test-output.log`

Every test invocation tees to `target/test-output.log`; the log is the failure-evidence source. The `perimeter_spatial_tdd` target carries `required-features = ["host-algos", "perimeter-spatial-test-support"]`; a bare `cargo test -p slicer-core` silently skips it, so every command above names both features.

## Authoritative Docs

- `docs/07_implementation_status.md` - delegated bounded lookup of the TASK-561 row.
- `docs/08_coordinate_system.md` - direct bounded read of the coordinate rule and conversion policy.
- `docs/21_data_defaults_and_fixtures.md` - implementation-time fixture and literal gate authority.
- `docs/19_visual_debug.md` and `docs/17_agent_debugging.md` - bounded delegated pipeline diagnosis context where needed.
- `tmp/perf-next/DESIGN-DECISIONS.md` - confirmed design input, read in bounded slices; historical provenance only. The packet's own design/plan files carry the settled normative contracts; this scratch record is not an implementation dependency (it is gitignored under `tmp/`).
- `docs/23_controlled_perimeter_builds.md` - new same-packet doc created in Step 10; contains the controlled-build policy grammar, mode rules, and corpus layout. Slot 23 is the next free `docs/NN_` number today (`docs/22_test_quality.md` occupies 22); re-derive with `ls docs/2*.md` at creation time.

## Doc Impact Statement (Required)

Specific same-packet doc edits planned by implementation:

- Root `AGENTS.md` §"Guest WASM Staleness (MUST follow)" gains the accelerated-mode namespace rule (Step 12) - verify `rg -q 'build-guests --accelerated' AGENTS.md` (no match today, confirmed 2026-09-10). `CLAUDE.md` is a gitignored copy regenerated by `cargo xtask sync-agents`; run it after the edit, do not edit `CLAUDE.md` by hand.
- `docs/03_wit_and_manifest.md` §"Build & Freshness Contract (Normative)" gains the accelerated `--check` rule while preserving the existing `build-guests --check` text (Step 12) - verify `rg -q 'build-guests --accelerated --check' docs/03_wit_and_manifest.md` (no match today).
- New `docs/23_controlled_perimeter_builds.md` (Step 10) - verify `rg -q 'controlled perimeter build' docs/23_controlled_perimeter_builds.md`.

Each grep must return exit 1 before its step's edit and exit 0 after it; a grep that already matches cannot verify an edit. These are implementation outputs, not edits in this authoring task. Append their verified greps to the implementing step's evidence.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — compare perimeter traversal and pass behavior without replacing PnP arithmetic or predicates.
- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` — document the build-once/query-many spatial precedent.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
