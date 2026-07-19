# Requirements: 177-arachne-baselines-to-structural-invariants

## Packet Metadata

- Grouped task IDs: none. This is an audit-driven slice sourced from `docs/DEVIATION_LOG.md`.
- Backlog source: `D-112-SELFCAPTURED-BASELINES`.
- Packet status: `draft` until measurement and packet preflight are green.
- Aggregate context cost: `M`.

## Problem Statement

The Arachne JSON corpus is captured from Pinch 'n Print's own output. It proves
only that the pipeline still resembles an earlier snapshot, not that the
geometry is correct. ADR-0042 requires structural properties and a known-good
reference for the D5 coverage failure class.

The correction must not replace one self-ratifying artifact with another. A
**coverage subject** therefore means source geometry that can be run through
both classic and Arachne at the same aligned Z planes. A serialized snapshot is
not a coverage subject.

## In Scope

- Retain the canonical `max_bead_count = 10` correction in both production
  defaults and every surviving odd test helper. The rationale is canonical
  even-count derivation, not a nonexistent odd-count giant-center branch.
- Delete the eight `crates/slicer-core/tests/fixtures/arachne/*.json` files and
  every `expected_perimeter_ir.json` under
  `crates/slicer-runtime/tests/fixtures/perimeter_parity/`. Remove all
  snapshot loaders, recorder functions, provenance checks, and change-detector
  policy from active tests.
- Rewrite centrality, propagation, bead-count, and toolpath tests around
  in-memory source geometry and named structural assertions.
- Add a standalone `slicer-runtime` test binary for paired classic/Arachne
  measurements. Extract the reusable perimeter capture harness under
  `crates/slicer-runtime/tests/common/`.
- Measure five Arachne source fixtures:
  `tapered_wedge`, `narrow_strip_widening`, `max_bead_count_cap`,
  `complex_multi_feature`, and `cube_4color_arachne`.
- Keep `d5_benchy_call1.txt` as a synthetic discriminator only. It rejects the
  broken ratio `0.668` and accepts the fixed ratio `0.990`; it is excluded from
  `observed_min` because it is not a paired source-geometry run.
- Derive the margin only from repeated same-input/same-Z runs. The margin is
  the maximum repeatability delta, capped at `0.02`. A larger delta is a
  nondeterminism finding, not permission to widen the gate. If
  `observed_min - margin <= 0.668`, stop and leave the packet blocked.
- Keep bead-width assertions in the Arachne spacing domain: cap emitted
  junction spacing at `2 * optimal_spacing_mm`; the historical `19.6 mm` value
  is a D4 failure observation, not the cap.
- Rehome the nine `arachne_parity_red_*.rs` files without changing test bodies
  or names, and correct the stale `crates/slicer-runtime/tests/arachne_parity.rs`
  header while keeping D-104f open.
- Correct the stale Track B prose in `docs/specs/arachne-parity-recovery.md`,
  record the measured threshold in ADR-0042, close D-112 only after all gates
  pass, and update the project glossary.

## Out of Scope

- Any new production geometry behavior except the even `max_bead_count`
  default correction.
- Re-recording any deleted snapshot or invoking a recorder to make a test pass.
- Treating D5's `0.668`/`0.990` synthetic values as measured threshold rows.
- The D-104f concentric-infill red test's behavior; only its stale header is
  corrected.
- WIT, IR schema, scheduler contract, or module manifest changes. The new
  runtime test harness uses existing module selection and pipeline capture.

## Domain Contract

- A **self-captured baseline** is historical evidence only; it is not a
  correctness oracle and is removed from the active test pipeline.
- A **coverage subject** supplies reproducible source geometry and paired
  classic/Arachne output at aligned Z planes.
- A **structural invariant** asserts a ratio, count, topology, tolerance, or
  spacing-domain cap, never equality against captured coordinates.
- A **repeatability margin** absorbs only same-subject measurement variation;
  it cannot absorb fixture spread or a known D5 regression.

## Acceptance Summary

- The thesis is AC-3 through AC-6: source-geometry measurement, a derived but
  bounded repeatability margin, D5 discrimination, and a structural tapered-
  wedge comparison.
- AC-7 and AC-8 replace snapshot consumers with in-memory structural cases and
  prove that no JSON oracle remains active.
- AC-N1 and AC-N2 are anti-vacuity tests for coverage rejection and the
  spacing-domain bead cap.
- No acceptance criterion may pass merely because a test binary collected zero
  tests.

## Verification Commands

| Command | Purpose |
| --- | --- |
| `rg -n 'max_bead_count:\s*9' crates/slicer-core/` | Re-derive the odd-default edit surface. |
| `cargo test -p slicer-core --features host-algos --test arachne_invariants` | Core topology, defaults, and spacing-domain invariant tests. |
| `cargo test -p slicer-core --features host-algos --test centrality` | In-memory centrality structural cases. |
| `cargo test -p slicer-core --features host-algos --test propagation` | In-memory propagation structural cases. |
| `cargo test -p slicer-core --features host-algos --test bead_count` | In-memory bead-count structural cases. |
| `cargo test -p slicer-core --features host-algos --test generate_toolpaths` | In-memory toolpath structural cases. |
| `cargo test -p slicer-runtime --test arachne_structural_invariants` | Paired source-geometry measurement and D5 discrimination. |
| `cargo test -p slicer-runtime --test integration perimeter_parity` | Existing integration parity after snapshot deletion. |
| `test -z "$(rg -l 'fixtures/arachne/.*\.json|expected_perimeter_ir\.json' crates/slicer-core/tests crates/slicer-runtime/tests 2>/dev/null)"` | No active JSON oracle loads. |
| `cargo check --workspace --all-targets` | Workspace compile gate. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint gate. |

Never use `cargo test --workspace` during implementation iterations. It is
reserved for the packet acceptance ceremony through `cargo xtask test --summary`.
