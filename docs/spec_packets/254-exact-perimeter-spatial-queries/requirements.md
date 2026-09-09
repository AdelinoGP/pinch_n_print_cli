# Requirements: exact-perimeter-spatial-queries

## Packet Metadata

- Grouped task IDs: `TASK-561`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Perimeter generation repeatedly scans region-specific geometry for distance, sign, quartile, and bridge queries. TASK-561 needs an exact spatial acceleration that preserves current floating/integer arithmetic and source-order semantics, is shared only within one region invocation, and is evaluated under a reproducible compiler contract rather than an uncontrolled numerical build. The same coherent slice must also prove real Classic/Arachne pipeline use and provide strict CPU and wall evidence.

## In Scope

- Add `crates/slicer-core/src/perimeter_spatial.rs` with four separate trees in one immutable per-region context: f64 distance edges, neutral-X f64 sign intervals, neutral-X f64 quartile intervals, and outward-rounded f64 bridge boxes over integer coordinates.
- Preserve independent `signed_distance_to_boundary`, `expolygon_to_path3d`, `point_in_any_polygon`, and `point_in_polygon_winding` legacy evaluation, including conversion domains, signed zero, ties, strict bridge boundaries, holes, closure repeats, widths, flags, and normalization.
- Build one context outside all repeated Classic and Arachne passes, including nonplanar Classic shell handling and Arachne's only-one-wall-top second pass; no cross-region cache or WIT/IR/scheduler/public execution-context change.
- Implement nearest-envelope seed, exact legacy re-evaluation, `D.next_up().sqrt().next_up().next_up()` radius, outward/inclusive envelope queries, `total_cmp` plus source ordinal reduction, and full legacy fallback for nonfinite/exceptional/structurally small inputs.
- Implement bridge global-span/query-extent checked-u128 guard before pruning, with original source-order scan when the guard fails.
- Add non-default feature-gated scoped native controls and diagnostics, real dispatch capture through `PerimeterCapturingLayerStageRunner`, and separate WASM self-baselines; no public context fields, global production counters, or native-vs-WASM oracle.
- Add the owned `RUSTC` shim and explicit policy grammar, toolchain identity, rejection latch, cfg injection only for canonical core library/unit tests, controlled debug profile, isolated cache/publication metadata, mode-aware guest freshness, and explicit accelerated entry points for build-guests, dist, and xtask test.
- Add committed synthetic/existing fixtures and readable versioned postcard provenance with explicit integer coordinates and float bits; local Benchy/base artifacts remain under `tmp/rtree_query_corpus/` and are not committed.
- Add the serialized acceptance runner/validator for supports-off Benchy, tree-support Benchy, and original tree-support base: warmup 1, ABBA then BAAB, four measured samples, 12 threads, CPU and process-wall ratios, exactness first, and KEEP only when both strict inequalities pass for every generator/workload.

## Out of Scope

- WIT, IR, scheduler, manifest contract, public `HostExecutionContext`, or `LayerStageInput` changes.
- Replacing or sharing the legacy evaluator with new pruning logic; tolerances, golden-only tests, or native-vs-WASM comparisons.
- Cross-region indexing, production timing counters, automatic retries after overlap, automatic commits, or quiet relaxation of the acceptance bar.
- Editing `docs/07_implementation_status.md` during packet authoring.

## Authoritative Docs

- `docs/08_coordinate_system.md` - direct bounded coordinate read.
- `docs/21_data_defaults_and_fixtures.md` - delegated fixture/literal rules.
- `docs/07_implementation_status.md` - delegated TASK-561 row lookup.
- `docs/19_visual_debug.md`, `docs/17_agent_debugging.md` - delegated pipeline evidence context.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — compare Classic/Arachne traversal and repeated-pass ownership.
- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` — compare the canonical reusable distance-query precedent.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

This packet is an optimization, not a parity port; the canonical reference (OrcaSlicerDocumented `PerimeterGenerator.cpp`, `ExtrusionProcessor.hpp`) supplies design precedent only — it is never an output oracle. Evidence standard:

- **Exactness, not goldens.** Every accelerated query result must equal the independent legacy evaluator bitwise (f32 bits, signed zero, source ordinal, `distance + 0.5 * width`, strict bridge boundaries). Tolerance comparisons are forbidden.
- **Both modes exercised.** Focused tests must prove the accelerated path ran (nonzero accelerated counters, more-than-leaf records present) and that disabling the mode exercises the complete legacy path on identical inputs.
- **Fallback is observable.** Nonfinite, guard-failed, and small-set inputs must demonstrate the complete legacy scan (counters or injected fault), not silently produce a plausible answer.
- **WASM self-baseline.** WASM module output is compared against its own preserved baseline on identical deterministic inputs; native-vs-WASM comparison is not an oracle.
- **No behavioral parity claim against OrcaSlicer** is made by this packet; canonical reads are delegated per the snippet above and used only as design precedent.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-4`.
- Negative: `AC-N1` through `AC-N3`.
- Cross-packet impact: none; TASK-561 is one pending backlog row.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test perimeter_spatial_tdd -- exact_queries_match_legacy --nocapture 2>&1 | tee target/test-output.log` | Exact scalar and bitwise query parity | FACT pass/fail; failure SNIPPETS <=20 lines |
| `cargo test -p slicer-core --features host-algos --test perimeter_spatial_tdd -- accelerated_mode_exercised_not_vacuous --nocapture 2>&1 | tee target/test-output.log` | Both modes exercised; no silent fallback | FACT pass/fail |
| `cargo test -p xtask --bin xtask -- accelerated_policy_rejection_latch --nocapture 2>&1 | tee target/test-output.log` | Driver policy rejection and latch | FACT pass/fail |
| `cargo xtask test --summary -p slicer-runtime --test integration -- perimeter_spatial_capture_and_nonvacuity` | Real pipeline capture and pass reuse | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- overlap_is_inconclusive_and_never_keep --nocapture 2>&1 | tee target/test-output.log` | Validator decision logic on synthetic rows | FACT pass/fail |
| `cargo xtask build-guests --check` | Ordinary artifact/WIT/lock freshness baseline | FACT exit 0/1/3 |
| `cargo check --workspace --all-targets` | Compile all targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required lint gate | FACT pass/fail |
| `pwsh -NoProfile -File tmp/perimeter-acceptance/run-acceptance.ps1 -Workload <workload> -ExpectedGenerator <gen>` | Serialized exactness/performance campaign per workload | JSON status KEEP/DROP/inconclusive; tee target/test-output.log |

Every test invocation tees to `target/test-output.log`; failure detail is read from that log, never re-run.

Accelerated artifact validation must additionally invoke the explicit accelerated `--check` mode; ordinary `cargo xtask build-guests --check` cannot silently validate accelerated artifacts.

## Step Completion Expectations

The indexed path is never the oracle; all region contexts are immutable and single-owner per invocation; the acceptance lane is serialized by the coordinator, while bounded reports may be delegated without concurrent heavy builds. A negative campaign outcome stops advancement and reports the result rather than retaining a falsely implemented optimization.

Accelerated xtask entry points use an explicit `--accelerated` flag (not a cargo feature) on `cargo xtask build-guests --accelerated`, `cargo xtask dist --accelerated`, and `cargo xtask test --accelerated`; the shim sets `RUSTC` and the reserved cfg is never ambient. The core test-support feature is `perimeter-spatial-test-support` (see design.md Code Change Surface); it is used only by focused test targets, never in timing/dist production artifacts.

## Context Discipline Notes

Read large source/docs only through bounded symbol windows or delegated summaries. Never load `target/`, lockfiles, generated code, or local corpus payloads; local corpus provenance and explicit float-bit fields must be inspected through the committed runner/validator.
