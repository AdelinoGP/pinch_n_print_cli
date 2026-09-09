---
status: draft
packet: 254-exact-perimeter-spatial-queries
task_ids:
  - TASK-561
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Draft authored from the confirmed TASK-561 design decisions; implementation and acceptance evidence remain downstream.
---

# Packet Contract: exact-perimeter-spatial-queries

## Goal

Add exact, independently-oracled shared per-region spatial query contexts for Classic and Arachne perimeter generation, and measure them only through a reproducible controlled Rust compiler mode.

## Scope Boundaries

This packet covers boundary distance/sign, overhang quartile, and bridge candidate pruning, their reuse across every relevant generator pass, exact fallback behavior, native/WASM capture and parity fixtures, and the controlled `RUSTC` driver and mode-aware guest artifacts. It does not change WIT, IR, scheduler, public host execution structs, or the legacy evaluators.

## Prerequisites and Blockers

- Depends on: existing perimeter generators, runtime perimeter harness, guest freshness machinery, and the current `rstar` dependency.
- Unblocks: implementation of TASK-561 and its controlled KEEP/DROP/inconclusive acceptance campaign.
- Activation blockers: none; status intentionally remains `draft` pending packet preflight and user activation.

## Acceptance Criteria

- **AC-1. Given** finite and adversarial synthetic regions containing contour/hole/closing edges, equal-distance candidates, signed zero, degenerate/empty rings, negative coordinates, computed-endpoint drift, widths, and distance normalization, **when** the accelerated native query mode is run against the independent legacy evaluator, **then** distance `Option` presence, signed-zero bits, selected source ordinal, `distance + 0.5 * width`, sign, quartile maximum, bridge strict-boundary result, flags, and closure output bits are identical for every query. | `cargo test -p slicer-core --features host-algos --test perimeter_spatial_tdd -- exact_queries_match_legacy --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** a region with more than `rstar::DefaultParams::MAX_SIZE` relevant records and multiple Classic/Arachne passes, including Classic nonplanar shells and Arachne's only-one-wall-top second pass, **when** the real module pipeline runs with scoped capture and the indexed mode, **then** captured region identity, pass identity, nonempty output, and query counters show every relevant pass reuses one immutable four-index context, with exact evaluations below the separated synthetic candidate count. | `cargo xtask test --summary -p slicer-runtime --test integration -- perimeter_spatial_capture_and_nonvacuity --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** ordinary builds, controlled host release/debug builds, and controlled Classic/Arachne guest builds, **when** mode-aware build/dist/test commands and freshness checks are run, **then** ordinary builds use the legacy path, accelerated artifacts are isolated by policy/compiler/mode identity, optimized-core debug retains debuginfo and assertions while `slicer-core` uses opt-level 3, guests remain release, and unchanged WIT/code artifacts report the documented freshness result without accepting an opposite-mode artifact. | `cargo test -p xtask --bin xtask -- accelerated_mode_freshness --nocapture 2>&1 | tee target/test-output.log`
- **AC-3N. Given** the accelerated feature is compiled into the core query path, **when** the same parity fixtures run under the accelerated context and then again with the accelerated mode disabled, **then** both modes produce identical legacy-oracle results AND the test-only counters prove the accelerated mode exercised the indexed path and the ordinary mode exercised the legacy path (no silent fallback, no vacuous pass). | `cargo test -p slicer-core --features host-algos --test perimeter_spatial_tdd -- accelerated_mode_exercised_not_vacuous --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** the accepted toolchain and a valid supports-off Benchy, tree-support Benchy, and original tree-support base corpus, **when** the serialized ABBA then BAAB campaign runs one warmup and four measured samples per variant/generator/workload with 12 threads, **then** exactness passes first and every generator/workload has `max(candidate CPU samples) < min(baseline CPU samples)` and `max(candidate wall samples) < min(baseline wall samples)`, with all CPU/wall ratios and provenance visible; any overlap, exactness failure, generator-marker disagreement, or degraded/fatal status change stops the campaign as `DROP` or `inconclusive` and is never reported as KEEP. Known pre-existing campaign conditions (e.g. base tree-support's recorded 29,108 non-fatal errors with `degraded: true`) must be IDENTICAL between baseline and candidate samples and remain visible in every retained row; they do not by themselves reject a sample, but any change in them is a generator disagreement. | `pwsh -NoProfile -File tmp/perimeter-acceptance/run-acceptance.ps1 -Workload supports-off-benchy -ExpectedGenerator classic 2>&1 | tee target/test-output.log; python3 -c "import json; d=json.load(open('target/perimeter-acceptance/summary.json')); assert d['status'] in ('KEEP','DROP','inconclusive') and d['automatic_commit'] is False and d['threads']==12 and d['samples_per_cell']==4 and d['warmup_per_cell']==1"`

## Negative Test Cases

- **AC-N1. Given** a shim invocation containing an unknown/duplicate keyed control, response file, unaudited LLVM/sysroot/backend/custom-target option, unsupported target, unoptimized accelerated core, or a failed policy validation later swallowed by a child process, **when** accelerated mode is requested, **then** the owned `RUSTC` shim rejects publication, latches the session rejection, and returns a nonzero policy failure; normal mode remains linear and does not receive the reserved cfg. | `cargo test -p xtask --bin xtask -- accelerated_policy_rejection_latch --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a nonfinite query, finite-input guard failure, bridge overflow bound failure, empty/edge-less boundary, structural small set, or absent corpus, **when** the relevant query/validation path is invoked, **then** it performs the complete source-order legacy behavior before any unsafe pruning (or reports the exact missing-corpus error), and no accelerated result is published as a successful proof. | `cargo test -p slicer-core --features host-algos --test perimeter_spatial_tdd -- fallback_and_missing_corpus_are_nonvacuous --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** overlapping CPU/wall ranges, exactness failure, generator-marker/config disagreement, or a fatal/nonfatal/degraded campaign status, **when** an acceptance stage completes, **then** the campaign stops without extra runs and reports `DROP` or `inconclusive` with every retained sample and ratio; it never reports `KEEP` or commits automatically. | `cargo test -p slicer-runtime --test integration -- overlap_is_inconclusive_and_never_keep --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test perimeter_spatial_tdd 2>&1 | tee target/test-output.log`

Every test invocation tees to `target/test-output.log`; the log is the failure-evidence source.

## Authoritative Docs

- `docs/07_implementation_status.md` - delegated bounded lookup of the TASK-561 row.
- `docs/08_coordinate_system.md` - direct bounded read of the coordinate rule and conversion policy.
- `docs/21_data_defaults_and_fixtures.md` - implementation-time fixture and literal gate authority.
- `docs/19_visual_debug.md` and `docs/17_agent_debugging.md` - bounded delegated pipeline diagnosis context where needed.
- `tmp/perf-next/DESIGN-DECISIONS.md` - confirmed design input, read in bounded slices; historical provenance only. The packet's own design/plan files carry the settled normative contracts; this scratch record is not an implementation dependency.
- `docs/22_controlled_perimeter_builds.md` - new same-packet doc created in Step 7; contains the controlled-build policy grammar and mode rules (this packet's doc-impact target, verified absent today).

## Doc Impact Statement (Required)

Specific same-packet doc edits planned by implementation: `AGENTS.md` guest-target rule - `rg -q 'target/guests' AGENTS.md`; `docs/03_wit_and_manifest.md` guest build/freshness section - `rg -q 'build-guests --check' docs/03_wit_and_manifest.md`; and new `docs/22_controlled_perimeter_builds.md` (confirmed absent today; created in Step 7) - `rg -q 'controlled perimeter build' docs/22_controlled_perimeter_builds.md`. Each verification grep must return a match AFTER that step's edit. These are implementation outputs, not edits in this authoring task. Append their verified greps to the implementing AC evidence.

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
