# Requirements: core-flow-consolidation

## Packet Metadata

- Grouped task IDs: `core/DUP-CORE (flow)`, `core/RETIRE (flow)`, `core/DUP-CORE-strengthen (wider-bead)`
- Backlog source: `docs/specs/test-quality-remediation-plan.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The flow implementation currently has inline unit tests in `slicer_core::flow::tests` and overlapping public-API tests in `flow_tdd`. The approved `core-flow-consolidation` slice must establish one integration-test home without losing any distinct regression input, while preserving and verifying the wider-bead test's existing independent-formula oracle and strict wider-than-canonical comparison. This is a test-only consolidation; it does not alter flow behavior or close any other core-wave finding.

## In Scope

- Move the ten distinct inline cases from `crates/slicer-core/src/flow.rs` into `crates/slicer-core/tests/flow_tdd.rs`, merging only exact behavioral duplicates with existing integration cases.
- Preserve the inline cases for canonical spacing, width-at/above-nozzle spacing, width-below-layer-height positivity, exact threshold rejection, zero/negative width handling, actionable error text, spacing-to-width round trip, non-thick bridge ratio, thick bridge factor, and degenerate thick-bridge fallback.
- Preserve all existing integration cases: wider-bead formula plus monotonic comparison, flow-to-width zero inputs and lower-bound clamp, and the four role-width precedence/fallback matrices with all nine roles and all explicit context fields.
- Preserve and verify `wider_bead_spacing_is_larger_than_canonical`'s existing independent formula `0.5 - 0.2 * (1 - PI/4)` and strict wider-than-canonical comparison against the canonical `0.4` result.
- Keep the sole test home at `crates/slicer-core/tests/flow_tdd.rs`; remove only the inline test block from `flow.rs`.
- Preserve actionable error fragments and the zero/negative width and true threshold rejection cases exactly as observable behavior.
- Record before/after discovery and execution census reconciliation at implementation time without freezing counts in this packet.

### Required survivor witnesses

The single integration home must retain these concrete witnesses (merging is allowed only when every listed input/assertion survives): `canonical_0p4mm_bead_0p2mm_layer_spacing`, `wider_bead_spacing_is_larger_than_canonical`, `width_below_layer_height_still_has_positive_spacing`, `spacing_errors_at_the_canonical_threshold`, `zero_or_negative_width_errors`, `production_reachable_negative_spacing_errors_with_actionable_message`, `roundtrip_spacing_to_width_recovers_original`, `spacing_is_monotone_in_line_width`, `flow_to_width_zero_inputs_return_zero`, `flow_to_width_result_is_at_least_spacing`, `bridging_flow_non_thick_returns_ratio_unchanged`, `bridging_flow_thick_round_cross_section_factor`, `bridging_flow_thick_degenerate_inputs_fall_back_to_ratio`, `resolve_role_width_bridge_and_first_layer_precedence_matrix`, `resolve_role_width_bridge_fallback_covers_zero_and_absent_widths`, `resolve_role_width_role_zero_and_positive_matrix`, and `resolve_role_width_auto_sentinel_uses_line_width_then_nozzle_width`.

### Distinct-input preservation inventory

Every inline case contributes the following exact input/assertion entries; relocation may merge names only if every entry remains a surviving assertion:

| Inline case | Exact inputs and assertions that must survive |
| --- | --- |
| `classic_0p4mm_bead_0p2mm_layer` | `line_width_to_spacing(0.4, 0.2).unwrap()` must satisfy `(s - 0.3571).abs() < 0.001`. |
| `width_at_or_above_nozzle_produces_larger_spacing` | `line_width_to_spacing(0.5, 0.2).unwrap()` must satisfy `s > 0.4 && s <= 0.5`. |
| `width_below_layer_height_still_has_positive_spacing` | `line_width_to_spacing(0.1, 0.2).unwrap()` must satisfy `(s - 0.0571).abs() < 1e-3`; `line_width_to_spacing(0.4, 1.0).unwrap()` must satisfy `(s - 0.1854).abs() < 1e-3`. |
| `spacing_errors_at_canonicals_actual_threshold` | With `boundary = 0.2 * (1.0 - core::f32::consts::PI / 4.0)`, `line_width_to_spacing(boundary, 0.2).unwrap_err()` must have `width_mm == boundary`, `layer_height_mm == 0.2`, and `spacing_mm <= 0.0`; `line_width_to_spacing(boundary * 0.5, 0.2).is_err()`; `line_width_to_spacing(boundary * 1.5, 0.2).unwrap() > 0.0`. |
| `zero_or_negative_inputs_error` | `line_width_to_spacing(0.0, 0.2).is_err()`; `line_width_to_spacing(-1.0, 0.2).is_err()`; `line_width_to_spacing(0.4, 0.0).unwrap() == 0.4`. |
| `error_message_names_inputs_and_fix` | `line_width_to_spacing(0.4, 2.0).unwrap_err().to_string()` must contain `"0.40mm"`, `"2.00mm"`, and `"Increase the wall line width or reduce layer height"`. |
| `roundtrip_spacing_to_width` | With `w = 0.4` and `lh = 0.2`, `s = line_width_to_spacing(w, lh).unwrap()` and `w2 = flow_to_width(s, lh)` must satisfy `(w - w2).abs() < 0.001`. |
| `bridging_flow_non_thick_returns_ratio_unchanged` | `bridging_flow(1.0, false, 0.4, 0.4, 0.2) == 1.0`; `bridging_flow(0.85, false, 0.4, 0.4, 0.2) == 0.85`; `bridging_flow(0.85, false, 0.0, 0.0, 0.0) == 0.85`. |
| `bridging_flow_thick_round_cross_section_factor` | `bridging_flow(1.0, true, 0.4, 0.4, 0.2)` must satisfy `(f - 1.5708).abs() < 0.01`. |
| `bridging_flow_thick_degenerate_inputs_fall_back_to_ratio` | `bridging_flow(0.9, true, 0.4, 0.0, 0.2) == 0.9`; `bridging_flow(0.9, true, 0.4, 0.4, 0.0) == 0.9`; `bridging_flow(0.9, true, 0.0, 0.4, 0.2) == 0.9`. |

The implementation survivor crosswalk must map every inventory entry to a surviving assertion in `flow_tdd.rs`; final review will spot-check the inventory against the merged tests.

### Discovery and reconciliation contract

Step 1's `packet01` discovery procedure is mandatory. Its command IDs are `packet01-inline-list-before`, `packet01-inline-exec-before`, `packet01-integration-list-before`, `packet01-integration-exec-before`, `packet01-inline-list-after`, `packet01-integration-list-after`, `packet01-integration-exec-after`, and `packet01-survivor-name-reconcile`. The already-recorded baseline logs `target/packet01-inline-list-before.log`, `target/packet01-inline-exec-before.log`, `target/packet01-integration-list-before.log`, and `target/packet01-integration-exec-before.log` are evidence to inspect, not outputs to regenerate or overwrite; the post-edit procedure writes `target/packet01-inline-list-after.log`, `target/packet01-integration-list-after.log`, and `target/packet01-integration-exec-after.log`. The post-edit inline discovery must be empty; post-edit integration discovery and execution must be nonzero and successful; every preexisting integration test name must remain present; and the name-only survivor crosswalk must map every original inline test name to a surviving `flow_tdd` name. The inventory above and final review, not the name check, prove content preservation. The existing 17 named survivor guards in the `flow_tdd` verification command remain mandatory.

The merged `width_below_layer_height_still_has_positive_spacing` survivor must retain both source inputs: `(0.1, 0.2)` with `abs(spacing - 0.0571) < 1e-3`, and `(0.4, 1.0)` with `abs(spacing - 0.1854) < 1e-3`. The merged `spacing_errors_at_the_canonical_threshold` survivor must retain the boundary `boundary = 0.2 * (1 - PI/4)` with `Err` fields `width_mm == boundary`, `layer_height_mm == 0.2`, `spacing_mm <= 0.0`, plus `line_width_to_spacing(boundary * 0.5, 0.2).is_err()` and the existing `boundary * 1.5` positive-spacing assertion. These additions are mandatory; do not replace the existing integration inputs.

## Out of Scope

- Any production code or behavior change in `crates/slicer-core/src/flow.rs` outside deleting its test module.
- `flow_correction_stays_positive_for_vertical_input` in `crates/slicer-core/src/lib.rs`; owned by `core-strengthen`.
- All other `slicer-core` test files and all other queue IDs, including pending paint, brittle, cross, parity, and non-flow merge items.
- New production exports, public struct changes, WIT/IR/schema changes, coordinate-system changes, WASM artifacts, or OrcaSlicer porting.
- Editing the parent plan during generation. The implementation worker must make the narrow §7 `core` ledger update required by AC-7, but must not rewrite queue entries, other waves, or claim the whole core wave closed.
- Task-### mapping; the approved program IDs are authoritative for this packet.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §1 non-negotiables; §4 wave exit gates; §5.1 flow items; §6 command pattern; Packet Queue entry for `core-flow-consolidation` and approved boundary.
- `docs/22_test_quality.md` - independent oracle, falsifiable regression, negative-control, and zero-population standards.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - every moved/merged case must name the regression input it protects.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - `check-test-quality` remains report mode until final program promotion.
- `.agents/skills/spec-packet-generator/references/templates/*.md` and `.agents/skills/spec-review/references/preflight-gate.md` - packet contract and S0–S8 inventory.

## Acceptance Summary

- Positive: `AC-1` through `AC-6` in `packet.spec.md`.
- Negative: `AC-N1` in `packet.spec.md`.
- Ledger evidence: `AC-7` in `packet.spec.md`; it owns the mandatory §7 partial-row evidence.
- Cross-packet impact: generation unblocks `core-bridge-dup-merge` only after independent preflight; `core-dup-merges` is superseded. The packet exports no API or implementation prerequisite. `core-strengthen` remains the owner of vertical-input correction.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test flow_tdd -- --list 2>&1 \| tee target/test-output.log >/dev/null; rg -q ": test$" target/test-output.log; cargo test -p slicer-core --features host-algos --test flow_tdd -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; for n in canonical_0p4mm_bead_0p2mm_layer_spacing wider_bead_spacing_is_larger_than_canonical width_below_layer_height_still_has_positive_spacing spacing_errors_at_the_canonical_threshold zero_or_negative_width_errors production_reachable_negative_spacing_errors_with_actionable_message roundtrip_spacing_to_width_recovers_original spacing_is_monotone_in_line_width flow_to_width_zero_inputs_return_zero flow_to_width_result_is_at_least_spacing bridging_flow_non_thick_returns_ratio_unchanged bridging_flow_thick_round_cross_section_factor bridging_flow_thick_degenerate_inputs_fall_back_to_ratio resolve_role_width_bridge_and_first_layer_precedence_matrix resolve_role_width_bridge_fallback_covers_zero_and_absent_widths resolve_role_width_role_zero_and_positive_matrix resolve_role_width_auto_sentinel_uses_line_width_then_nozzle_width; do rg -q "test ${n} \.\.\. ok" target/test-output.log; done'` | Prove the actual `flow_tdd` binary discovers tests and every required survivor passes without a zero-test false green. | FACT pass/fail; inspect `target/test-output.log` on failure. |
| `cargo check --workspace --all-targets` | Compile the test target and all workspace targets after deleting inline tests. | FACT pass/fail. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Preserve lint cleanliness after relocation. | FACT pass/fail. |
| `cargo xtask check-literals` | Confirm no test fixture literal rule is introduced or violated. | FACT pass/fail. |
| `cargo xtask check-test-quality --report crates/slicer-core/src/flow.rs crates/slicer-core/tests/flow_tdd.rs` | Report touched-scope quality findings using the established path-argument form; implementation must close or justify any remaining flow findings. | FACT pass/fail plus filtered findings. |
| `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(encoding=\"utf-8\"); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; headers=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-flow-consolidation\",\"flow.rs\",\"flow_tdd\")); assert all(x in coverage for x in (\"flow_tdd\",\"width_below_layer_height_still_has_positive_spacing\",\"spacing_errors_at_the_canonical_threshold\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"remaining non-flow §5.1 work stays open; next queued gap: core-bridge-dup-merge\" in gap.casefold(); print(\"PASS: anchored core ledger row has six validated cells\")"'` | Parse §7, select exactly the `core` row, enforce the exact six-column header and row shape, validate every AC-7 field, and require the final cell to contain the exact phrase `remaining non-flow §5.1 work stays open; next queued gap: core-bridge-dup-merge`, without global keyword matches. | FACT pass/fail; parser emits one PASS line or exits nonzero. |

Every future cargo test command uses a self-contained `bash -lc 'set -euo pipefail; ...'` wrapper, creates `target`, captures combined output with `tee target/test-output.log >/dev/null`, checks the completed log rather than piping `rg` directly from `tee`, and rejects zero discovery before execution. The implementation must re-derive current counts from `cargo test ... -- --list` and the current discovery census rather than use packet-time counts.

## Step Completion Expectations

The relocation step must preserve the union of distinct cases before deleting inline tests. The final validation must show one test home, no inline `#[test]`, feature-correct discovery, and no `core-strengthen` vertical-flow case accidentally moved. The implementation must also complete the narrow §7 ledger update; no other queue item or the whole core wave is implicitly closed.

## Context Discipline Notes

Read only the bounded source ranges named by `design.md`; do not inspect unrelated core tests. `flow_tdd.rs` is the exact integration target and `Cargo.toml` is read only for target/feature wiring. No OrcaSlicerDocumented inspection is applicable because this packet does not port or modify production canonical behavior.
