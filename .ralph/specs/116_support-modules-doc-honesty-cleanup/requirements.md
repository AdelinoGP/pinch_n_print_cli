# Requirements: support-modules-doc-honesty-cleanup

## Packet Metadata

- Grouped source-plan work items: B1, B2, and B3; no current `docs/07_implementation_status.md` task IDs are mapped.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The current support-module comments still overstate Orca parity: `tree-support` calls itself a tree generator despite its grid-MST fallback, and `support-planner` calls itself an Orca port despite implementing only an algorithmic shape. `traditional-support` has useful per-layer wording but does not use the source-plan's exact current-facing description. The current `SupportPlanner` also stores `support_interface_bottom_layers` without using it, while the config key remains user-visible. Finally, `BASE_SPEED` normalization is undocumented in the three in-scope current consumers; the old packet incorrectly included `support-planner`, which has no `BASE_SPEED` symbol today.

These are one local honesty/dead-state slice: four lead comment blocks, one dead field and parser branch, and one schema comment. Source-plan D8 is owned here; source-plan D11's typed diagnostic is explicitly owned by packet 118, so this packet emits no predecessor string warning and adds no warning test. The source-plan labels have no canonical support rows in the current backlog, and three proposed IDs collide with unrelated backlog ownership, so this packet cannot be activated until the crosswalk is repaired.

## In Scope

- Rewrite the contiguous leading `//!` block in `modules/core-modules/tree-support/src/lib.rs`; its first line must open with the per-layer grid-MST/optional `SupportPlanIR` description, and the same block must state the implementation is not a port of OrcaSlicer's `TreeSupport`.
- Rewrite the contiguous leading `//!` block in `modules/core-modules/traditional-support/src/lib.rs`; its first line must open with the per-layer scan-line description, and the same block must state `Depends entirely on upstream SliceRegionView::needs_support()`.
- Rewrite the contiguous leading `//!` block in `modules/core-modules/support-planner/src/lib.rs`; its first line must open with the `TreeSupport::drop_nodes` inspiration, and the same block must describe the detect/contact/top-down-MST/emit algorithmic shape without claiming numerical parity.
- Add a `# Speed normalization` section to `tree-support`, `traditional-support`, and `rectilinear-infill` only; each leading block must explain `speed_factor = configured_speed / BASE_SPEED` and contain `BASE_SPEED = 50.0`. Do not add a false speed section to `support-planner`.
- Delete every Rust field or struct-literal assignment for `support_interface_bottom_layers`, its `on_print_start` parse-and-store lookup, and its current `default_planner` struct-literal assignment.
- Preserve the snake_case `support_interface_bottom_layers` entry in `support-planner.toml`; add an immediately adjacent `Not yet implemented` comment pointing at the canonical support spec.
- Leave typed `support_interface_bottom_layers` warning emission to packet 118's channel scope; packet 116 must not prescribe an untyped string warning, retain a key lookup, or add a warning test binary.

## Out of Scope

- Assigning or inventing a replacement `TASK-###` ID; backlog ownership is an activation blocker.
- Implementing bottom interface layers or deleting the user-facing TOML key.
- Emitting either an untyped string warning or the typed `Diagnostic`; packet 118 and the canonical `TASK-163b-diagnostic` row own D11 and must reconcile their current dependency wording before activation.
- Adding speed comments to other current `BASE_SPEED` consumers such as `gyroid-infill`, `lightning-infill`, `classic-perimeters`, or ironing modules.
- Changing speed values, IR/WIT types, manifests, scheduler behavior, paint policy, geometry, raft behavior, or planner algorithms.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` - direct range read of §B1-B3 and §D8-D9; exact source for the requested descriptions, dead-state boundary, and normalization explanation.
- `docs/adr/0010-typed-diagnostic-channel.md` - direct read of the typed `Diagnostic` contract; D11 is packet 118's ownership, not a string-warning contract here.
- `docs/specs/support-modules-orca-port-plan.md` - direct queue read for packet 116; it supplies source-plan labels, not canonical backlog ownership.
- `docs/07_implementation_status.md` - targeted lookup of support ownership and colliding task IDs; do not edit this generated/current backlog in the packet.

## Acceptance Summary

Reference, never duplicate, the criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7`; these cover the three honest descriptions, D8 dead-state removal, the absence of an untyped D11 warning in this packet, the actual speed consumers, and the TOML signal.
- Negative: `AC-N1`; packet 116 does not claim runtime warning behavior for default or non-default values.
- Cross-packet impact: packet 117 may depend on the source edits landing first because both edit `support-planner/src/lib.rs`; packet 118 owns creation and typed emission of the D11 warning. Its current dependency on a packet-116 warning path is an explicit activation blocker, not a dependency this packet satisfies.

## Verification Commands

This is the authoritative full matrix; cargo commands use `--all-targets`, and every test invocation tees output as required.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `docs=$(sed -n '1,80p' modules/core-modules/tree-support/src/lib.rs \| sed -n '/^\/\/!/p; /^\/\/!/!q') && printf '%s\n' "$docs" \| sed -n '1p' \| rg -q '^//! Per-layer 2-D grid-MST infill with optional SupportPlanIR consumption' && printf '%s\n' "$docs" \| rg -q "^//! .*not a port of OrcaSlicer's TreeSupport"` | AC-1 checks only the bounded contiguous leading doc block and its first line. | FACT pass/fail |
| `docs=$(sed -n '1,80p' modules/core-modules/traditional-support/src/lib.rs \| sed -n '/^\/\/!/p; /^\/\/!/!q') && printf '%s\n' "$docs" \| sed -n '1p' \| rg -q '^//! Per-layer rectilinear scan-line filler for Layer::Support' && printf '%s\n' "$docs" \| rg -q '^//! .*Depends entirely on upstream SliceRegionView::needs_support\(\)'` | AC-2 checks only the bounded contiguous leading doc block and its first line. | FACT pass/fail |
| `docs=$(sed -n '1,80p' modules/core-modules/support-planner/src/lib.rs \| sed -n '/^\/\/!/p; /^\/\/!/!q') && printf '%s\n' "$docs" \| sed -n '1p' \| rg -q "^//! Multi-layer support planner inspired by OrcaSlicer's TreeSupport::drop_nodes" && printf '%s\n' "$docs" \| rg -q '^//! .*algorithmic shape .*detect.*contact.*top-down MST propagation.*emit.*not numerical parity'` | AC-3 checks only the bounded contiguous leading doc block and its first line. | FACT pass/fail |
| `! rg -q 'support_interface_bottom_layers\\s*[:=]' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'config\\.get\\("support_interface_bottom_layers"\\)' modules/core-modules/support-planner/src/lib.rs` | AC-4 rejects every field/struct-literal or assignment form and separately rejects the parse-and-store lookup. | FACT pass/fail |
| `! rg -q 'support_interface_bottom_layers is not yet implemented' modules/core-modules/support-planner/src/lib.rs` | AC-5 and AC-N1: packet 116 emits no untyped warning string. | FACT pass/fail |
| `for m in tree-support traditional-support rectilinear-infill; do docs=$(sed -n '1,80p' modules/core-modules/$m/src/lib.rs \| sed -n '/^\/\/!/p; /^\/\/!/!q') || exit 1; printf '%s\n' "$docs" \| rg -q '^//! # Speed normalization' || exit 1; printf '%s\n' "$docs" \| rg -q '^//! .*speed_factor = configured_speed / BASE_SPEED' || exit 1; printf '%s\n' "$docs" \| rg -q '^//! .*BASE_SPEED = 50\.0' || exit 1; done` | AC-6 checks the heading, formula, and base value in each bounded leading doc block. | FACT pass/fail |
| `sed -n '1,200p' modules/core-modules/support-planner/support-planner.toml \| rg -q -U '^# Not yet implemented.*docs/specs/support-modules-orca-port\.md.*$\n^\[config\.schema\.support_interface_bottom_layers\]$' || sed -n '1,200p' modules/core-modules/support-planner/support-planner.toml \| rg -q -U '^\[config\.schema\.support_interface_bottom_layers\]$\n^# Not yet implemented.*docs/specs/support-modules-orca-port\.md'` | AC-7 checks the snake_case key and an immediately adjacent deferred-status comment in a bounded excerpt. | FACT pass/fail |
| `cargo check -p tree-support -p traditional-support -p support-planner -p rectilinear-infill --all-targets` | Compile all edited module targets. | FACT pass/fail |
| `cargo clippy -p tree-support -p traditional-support -p support-planner -p rectilinear-infill --all-targets -- -D warnings` | Lint gate. | FACT pass/fail; bounded failure SNIPPETS |
| `cargo xtask build-guests --check` | Verify guest artifacts after module source edits. | FACT `up to date` or `STALE: <path>` |

## Step Completion Expectations

- The comment edits in `support-planner/src/lib.rs` and the B2 implementation edit that follows it are sequential; no worker may overwrite the other's lead block.
- No warning test binary belongs to packet 116; packet 118 must test the typed diagnostic's exact code, message, cardinality, and default/absent-key silence.
- The TOML key remains present even though Rust no longer stores it; packet 118 owns the later typed channel migration.
- The unresolved backlog mapping remains visible in `design.md` and `task-map.md`; no implementation worker may close or rename a generated backlog row from this packet.

## Context Discipline Notes

- `docs/07_implementation_status.md` is mutable ledger state; re-search the exact task IDs immediately before any mapping decision and do not read or edit the whole file.
- `docs/specs/support-modules-orca-port.md` is read by section only; no Orca source inspection is needed for this non-parity documentation/diagnostic slice.
- Cargo results are returned as `FACT` with bounded failure snippets; no SDK logging surface is part of this narrowed packet.
