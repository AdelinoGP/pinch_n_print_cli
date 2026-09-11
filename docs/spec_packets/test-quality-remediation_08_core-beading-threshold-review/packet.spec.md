---
status: draft
packet: test-quality-remediation_08_core-beading-threshold-review
task_ids:
  - core/DUP-CORE (beading factory)
backlog_source: docs/specs/test-quality-remediation-plan.md
depends_on:
  - core-geometry-dup-review
context_cost_estimate: S
copy_note: Approved row-#8 packet; plan wave/item IDs are used under the standing mapping exemption, not TASK-### IDs.
---

# Packet Contract: core-beading-threshold-review

## Goal

Preserve the three distinct factory threshold witnesses and add an independently expected non-default full-stack case so default seeds, computed propagation, and clamp bounds remain separately falsifiable.

## Scope Boundaries

The source-test edit is limited to `crates/slicer-core/tests/beading/factory.rs`: all three threshold tests remain, and the approved contrasting case is added inside `beading_factory_threshold_propagates_through_full_stack` without replacing its existing `.99/.99` case. A separate implementation step updates only the `core` row in §7 of `docs/specs/test-quality-remediation-plan.md`; production code, default values, other tests, fixtures, registrations, and contracts are out of scope.

## Prerequisites and Blockers

- Depends on: `core-geometry-dup-review` (row #7, `generated`; test-only and exports no symbols, APIs, or files consumed here).
- Unblocks: `core-strengthen` (row #9) packet generation only.
- Activation blockers: none for draft generation; parent owns the independent preflight.

## Acceptance Criteria

- **AC-1. Given** `beading_factory_passes_split_middle_thresholds` invokes `BeadingFactoryParams::default()`, **when** the feature-correct exact filter runs, **then** exactly one test executes and passes with independent runtime assertions comparing `wall_split_middle_threshold` and `wall_add_middle_threshold` to literal `0.99` within `TOLERANCE`. The assertion behavior is the acceptance claim; the selected test-code plan preserves the authored assertion expressions for later code review. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test beading_factory beading_factory_passes_split_middle_thresholds -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -qF "running 1 test" target/test-output.log; grep -qF "test beading_factory_passes_split_middle_thresholds ... ok" target/test-output.log; grep -qF "test result: ok. 1 passed; 0 failed;" target/test-output.log'`
- **AC-2. Given** `beading_factory_threshold_propagates_through_full_stack` calls production `BeadingStrategyFactory::create_stack` with `outer_wall_offset = 300.0` and `print_thin_walls = true` in both the retained default-width case and an additive case using `min_output_width = 3000.0`, `preferred_bead_width_outer = 5000.0`, and `optimal_width = 4000.0`, **when** the exact filter runs, **then** exactly one test executes and passes with runtime `TOLERANCE` assertions for split/add `.99/.99` in the original case and independently fixed `.20/.75` in the contrasting case. Both cases are observable production-call behavior; no source-local identifier or source-text roster is acceptance evidence. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test beading_factory beading_factory_threshold_propagates_through_full_stack -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -qF "running 1 test" target/test-output.log; grep -qF "test beading_factory_threshold_propagates_through_full_stack ... ok" target/test-output.log; grep -qF "test result: ok. 1 passed; 0 failed;" target/test-output.log'`
- **AC-3. Given** `beading_factory_threshold_clamp_bounds_are_canonical` calls production `BeadingStrategyFactory::create_stack` for `min_output_width = 100.0` and `min_output_width = 100_000.0` with the remaining widths supplied by the existing defaults, **when** the feature-correct exact filter runs, **then** exactly one test executes and passes with independent runtime `TOLERANCE` assertions for split `.01` and add `.025` at the lower fixture, and split `.99` at the upper fixture. The selected test-code plan preserves these exact fixtures and literals for later code review. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test beading_factory beading_factory_threshold_clamp_bounds_are_canonical -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -qF "running 1 test" target/test-output.log; grep -qF "test beading_factory_threshold_clamp_bounds_are_canonical ... ok" target/test-output.log; grep -qF "test result: ok. 1 passed; 0 failed;" target/test-output.log'`
- **AC-4. Given** the registered `beading_factory` target contains the retained threshold tests and the pre-existing factory tests, **when** the full target runs under the explicit core feature form, **then** it reports a positive passing test count with zero failures; no fixed binary or whole-workspace count is claimed. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test beading_factory -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq "test result: ok\\. [1-9][0-9]* passed; 0 failed;" target/test-output.log'`
- **AC-5. Given** the real §7 heading is `## 7. Ledger (progress record; rows store re-derivable facts, never frozen counts)` and the six-column ledger contains accumulated prior content plus this packet's `core` evidence, **when** the section-anchor grep and parser run, **then** exactly one `core` row has state `partial` (optionally with parenthesized detail), its changed/surviving/validation/gap cells contain this packet's KEEP evidence, all three threshold test names, contrasting `.20/.75`, the feature-correct `cargo test -p slicer-core --features host-algos --test beading_factory` command as a token-boundary match inside Validation, and the required remaining-core markers. The parser checks no non-`core` row contents. | `bash -lc "set -euo pipefail; mkdir -p target; grep -Eq '^## 7[.] Ledger[[:space:]].*$' docs/specs/test-quality-remediation-plan.md; python3 -c 'import pathlib,re; d=pathlib.Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(encoding=\"utf-8\"); m=re.search(r\"(?ms)^## 7[.] Ledger[ ]+[^\\n]*\\n(.*?)(?=^## Packet Queue)\",d); assert m,\"§7 Ledger section missing\"; rows=[x for x in m.group(1).splitlines() if re.match(r\"^[ ]*[|]\",x) and not re.match(r\"^[ ]*[|]---\",x)]; h=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(h)==1 and [x.strip() for x in h[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,surviving,validation,gap=c; assert wave==\"core\" and re.fullmatch(r\"partial([ ]*[(][^()]*[)])?\",state); assert all(x in changed for x in (\"KEEP\",\"core-beading-threshold-review\")); assert all(x in surviving for x in (\"beading_factory_passes_split_middle_thresholds\",\"beading_factory_threshold_propagates_through_full_stack\",\"beading_factory_threshold_clamp_bounds_are_canonical\",\"0.20\",\"0.75\")); has=lambda cell,t: re.search(r\"(?<![A-Za-z0-9_])\"+re.escape(t)+r\"(?![A-Za-z0-9_])\",cell) is not None; assert has(validation,\"cargo test -p slicer-core --features host-algos --test beading_factory\"); assert all(x in validation for x in (\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"cargo xtask check-test-quality --report\")); assert all(x in gap for x in (\"core-strengthen\",\"core-retire\",\"core-paint\",\"core-brittle\",\"core-cross\",\"core-parity\")); print(\"LEDGER-PASS\")'"`

## Verification

- `bash -lc 'set -euo pipefail; mkdir -p target; cargo check --workspace --all-targets 2>&1 | tee target/check-output.log >/dev/null'`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/clippy-output.log >/dev/null'`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test beading_factory -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq "test result: ok\\. [1-9][0-9]* passed; 0 failed;" target/test-output.log'`

Cargo **test** commands use `set -euo pipefail`, `mkdir -p target`, `tee target/test-output.log`, and anchored result checks. Workspace check/clippy and xtask gates use the same compact-output wrapper with dedicated logs; they do not overwrite the test log. No workspace suite or guest freshness check is required because this packet is host-side test-only.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §5.1 `DUP-CORE` beading entry, §6 feature-correct command/log rules, §7 ledger ownership, Packet Queue row #8, and the continuation approval preserving distinct beading witnesses; read only those sections.
- `docs/22_test_quality.md` - §§1–4; earn-their-keep, independent expectations, non-vacuity, and self-referential-oracle rules.
- `docs/21_data_defaults_and_fixtures.md` - §§1, 4, and 6; `BeadingFactoryParams` is a watched public struct, so every edited test literal retains functional-record-update syntax.
- `docs/15_config_keys_reference.md` - Arachne beading-stack section; read-only context for the width/default terminology.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - distinct regression-input and survivor-map standard; no threshold test is retired by this packet.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - report-mode closure obligation for the touched test file.

## Doc Impact Statement (Required)

- **Specific same-packet doc edit:** `docs/specs/test-quality-remediation-plan.md` §7 `core` Ledger row - implementation Step 2 appends this packet's KEEP review, strengthened propagation coverage, all three surviving threshold behaviors, actual feature-correct validation, and the remaining core markers while preserving accumulated prior content. Required doc-impact check: `set -euo pipefail; grep -Eq '^## 7[.] Ledger[[:space:]].*$' docs/specs/test-quality-remediation-plan.md`; AC-5 then parses that real heading and six-column section. It must not edit the Packet Queue or any non-`core` row, and no non-`core` contents are frozen by the checker.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (full repo-relative file + function + one-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` - delegated `WallToolPaths::generate()` evidence for the split/add formulas and canonical `[0.01, 0.99]` clamps.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategyFactory.cpp` - delegated `BeadingStrategyFactory::makeStrategy` evidence that both computed values enter the distributed strategy unchanged.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/DistributedBeadingStrategy.cpp` - delegated constructor and `DistributedBeadingStrategy::getOptimalBeadCount` evidence for storage and parity-selected consumption.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.cpp` - delegated `BeadingStrategy::BeadingStrategy`, `BeadingStrategy::getTransitionThickness`, and `BeadingStrategy::getSplitMiddleThreshold` evidence for base storage and transition use.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list - those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
