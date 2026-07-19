---
status: draft
packet: 116
task_ids: []
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: support-modules-doc-honesty-cleanup

## Goal

Make the support-module documentation match the current implementations, remove unused bottom-interface state while preserving its config key and explicit deferred status, and document the existing speed-factor normalization only in the current support/infill consumers that define `BASE_SPEED`.

## Scope Boundaries

This packet edits the lead module documentation in `tree-support`, `traditional-support`, `support-planner`, and `rectilinear-infill`, the `SupportPlanner` bottom-interface dead state, and its TOML schema comment. `support-planner` is not a `BASE_SPEED` consumer in the current tree, so its speed section is not fabricated. No warning is emitted here: packet 118 owns the typed D11 diagnostic channel. No IR, WIT, scheduler, manifest, or real bottom-interface implementation changes are included.

## Prerequisites and Blockers

- Depends on: no code prerequisite; the canonical backlog ownership for source-plan items B1, B2, and B3 must be resolved before activation.
- Unblocks: shared-file review for `117_support-planner-geometric-correctness`; packet 118 owns the D11 typed warning and is not supplied a string-warning implementation by this packet.
- Activation blockers: `[BLOCK]` the source-plan labels B1/B2/B3 have no matching support rows in `docs/07_implementation_status.md`; the source-plan IDs `TASK-250`, `TASK-251`, and `TASK-252` cannot be retained because current backlog entries use colliding or unrelated ownership. `[BLOCK]` packet 118's current dependency/AC wording must be reconciled so it creates the typed `support_interface_bottom_layers` diagnostic itself rather than depending on a warning emitted by packet 116.

## Acceptance Criteria

- **AC-1. Given** the current `tree-support` module, **when** its contiguous leading `//!` block is read, **then** it opens with `Per-layer 2-D grid-MST infill with optional SupportPlanIR consumption` and states that it is not a port of OrcaSlicer's TreeSupport. | `docs=$(sed -n '1,80p' modules/core-modules/tree-support/src/lib.rs | sed -n '/^\/\/!/p; /^\/\/!/!q') && printf '%s\n' "$docs" | sed -n '1p' | rg -q '^//! Per-layer 2-D grid-MST infill with optional SupportPlanIR consumption' && printf '%s\n' "$docs" | rg -q "^//! .*not a port of OrcaSlicer's TreeSupport"`
- **AC-2. Given** the current `traditional-support` module, **when** its contiguous leading `//!` block is read, **then** it opens with `Per-layer rectilinear scan-line filler for Layer::Support` and states `Depends entirely on upstream SliceRegionView::needs_support()`. | `docs=$(sed -n '1,80p' modules/core-modules/traditional-support/src/lib.rs | sed -n '/^\/\/!/p; /^\/\/!/!q') && printf '%s\n' "$docs" | sed -n '1p' | rg -q '^//! Per-layer rectilinear scan-line filler for Layer::Support' && printf '%s\n' "$docs" | rg -q '^//! .*Depends entirely on upstream SliceRegionView::needs_support\(\)'`
- **AC-3. Given** the current `support-planner` module, **when** its contiguous leading `//!` block is read, **then** it opens with `Multi-layer support planner inspired by OrcaSlicer's TreeSupport::drop_nodes` and says its detect/contact/top-down-MST/emit shape is not numerical parity. | `docs=$(sed -n '1,80p' modules/core-modules/support-planner/src/lib.rs | sed -n '/^\/\/!/p; /^\/\/!/!q') && printf '%s\n' "$docs" | sed -n '1p' | rg -q "^//! Multi-layer support planner inspired by OrcaSlicer's TreeSupport::drop_nodes" && printf '%s\n' "$docs" | rg -q '^//! .*algorithmic shape .*detect.*contact.*top-down MST propagation.*emit.*not numerical parity'`
- **AC-4. Given** the current `SupportPlanner` source, **when** it is searched for bottom-interface state, **then** no field or struct-literal assignment for `support_interface_bottom_layers` remains, and no parse-and-store lookup remains. | `! rg -q 'support_interface_bottom_layers\s*[:=]' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'config\.get\("support_interface_bottom_layers"\)' modules/core-modules/support-planner/src/lib.rs`
- **AC-5. Given** packet 116's narrowed implementation surface, **when** the planner source is searched, **then** it does not emit the D8 not-implemented warning string; packet 118 owns the typed D11 warning. | `! rg -q 'support_interface_bottom_layers is not yet implemented' modules/core-modules/support-planner/src/lib.rs`
- **AC-6. Given** the current `BASE_SPEED` consumers in this packet (`tree-support`, `traditional-support`, and `rectilinear-infill`), **when** each contiguous leading `//!` block is read, **then** each contains `# Speed normalization`, explains `speed_factor = configured_speed / BASE_SPEED`, and contains `BASE_SPEED = 50.0`; `support-planner` is not included because it has no such constant. | `for m in tree-support traditional-support rectilinear-infill; do docs=$(sed -n '1,80p' modules/core-modules/$m/src/lib.rs | sed -n '/^\/\/!/p; /^\/\/!/!q') || exit 1; printf '%s\n' "$docs" | rg -q '^//! # Speed normalization' || exit 1; printf '%s\n' "$docs" | rg -q '^//! .*speed_factor = configured_speed / BASE_SPEED' || exit 1; printf '%s\n' "$docs" | rg -q '^//! .*BASE_SPEED = 50\.0' || exit 1; done`
- **AC-7. Given** the `support_interface_bottom_layers` schema entry in `support-planner.toml`, **when** the bounded schema excerpt is read, **then** the snake_case entry is present and an immediately adjacent `Not yet implemented` comment points to `docs/specs/support-modules-orca-port.md`. | `sed -n '1,200p' modules/core-modules/support-planner/support-planner.toml | rg -q -U '^# Not yet implemented.*docs/specs/support-modules-orca-port\.md.*$\n^\[config\.schema\.support_interface_bottom_layers\]$' || sed -n '1,200p' modules/core-modules/support-planner/support-planner.toml | rg -q -U '^\[config\.schema\.support_interface_bottom_layers\]$\n^# Not yet implemented.*docs/specs/support-modules-orca-port\.md'`

## Negative Test Cases

- **AC-N1. Given** packet 116's implementation surface, **when** its source is searched for the deferred warning callsite, **then** no string-warning implementation is prescribed or claimed. | `! rg -q 'support_interface_bottom_layers is not yet implemented' modules/core-modules/support-planner/src/lib.rs`

## Verification

- `cargo check -p tree-support -p traditional-support -p support-planner -p rectilinear-infill --all-targets`
- `cargo clippy -p tree-support -p traditional-support -p support-planner -p rectilinear-infill --all-targets -- -D warnings`
- `cargo test -p support-planner --all-targets 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` - direct read of §B1, §B2, §B3, §D8, §D9, and §D11; source of the intended honesty language, dead-state boundary, and diagnostic ownership split.
- `docs/adr/0010-typed-diagnostic-channel.md` - typed diagnostic contract that packet 118 owns; packet 116 must not emit its predecessor string warning.
- `docs/specs/support-modules-orca-port-plan.md` - direct read of the packet-116 queue row; source-plan labels are not treated as current backlog ownership.
- `docs/07_implementation_status.md` - targeted search of the support rows and every colliding `TASK-250`, `TASK-251`, and `TASK-252` entry.

## Doc Impact Statement (Required)

**`none`** - only Rust/TOML comments and dead-state cleanup change; no public IR, WIT, scheduler, claim, manifest, host-service, or SDK contract changes.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
