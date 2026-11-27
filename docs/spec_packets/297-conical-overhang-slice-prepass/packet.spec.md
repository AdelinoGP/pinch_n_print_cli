---
status: draft
packet: 297-conical-overhang-slice-prepass
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/78-author-packet-p71-quality-overhangs-slice-prepass.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 78 (P71).
---

# Packet Contract: 297-conical-overhang-slice-prepass

## Goal

Make the P71 overhang trio drive a conical-overhang SliceIR mutation in the host prepass at parity with canonical `PrintObject::apply_conical_overhang` — disabled by default, angle-controlled slope expansion plus hole-size-guarded hole preservation when enabled — with no new module, IR field, WIT change, or manifest row.

## Scope Boundaries

P71 is three Tier B keys owned by the host prepass cluster (`slicer-core` algos + `slicer-runtime` builtin). Claim-time grounding (ticket 78) holds all three in: each is live in canonical's slicing pipeline (single consumer `PrintObject::apply_conical_overhang` in `PrintObjectSlice.cpp`, invoked from `PrintObject::slice`; none dead-in-canonical) and zero-occurrence as behaviour in this tree (20 doc-only hits, 0 in any `.rs`/`.toml`/`.wit`/`.json`). The tier table's `slice-prepass (apply_conical_overhang)` owner is confirmed, corrected to this tree's seam: the pure kernel lands in `slicer-core::algos` beside `overhang_annotation`, and the thin producer beside `overhang_annotation_producer`, registered between `PrePass::Slice` and `PrePass::OverhangAnnotation` (the mutation must precede every SliceIR differ). The packet declares three `ResolvedConfig` fields at canonical defaults, threads them through the sibling raw-source read, and mutates committed `SliceIR` via the `PaintSegmentation` `replace_slice_ir` precedent, reusing `polygon_ops` offset/union/difference (no new boolean code). Rule 4 does not fire: the three values parameterise one internal geometric pass, not cross-module algorithm selection — there are no claim holders to create. Canonical angle bounds (0–90) are GUI hints (map Notes, ticket 113 measurement): the packet performs no range rejection and mirrors the `== 90.0` early-return plus `tan(angle) * layer_h` computation exactly, so no deviation row. CONFIG_BLOCK emission rides as a live-key side effect only; the word-form bool spelling stays with ticket 132 (no spot fix) and the padding table is untouched.

## Prerequisites and Blockers

- Depends on: nothing. All symbols below are live on HEAD (verified at authoring); the Slice commit site, the `run_builtin_stage` seam, `replace_slice_ir`, the `polygon_ops` primitives, and the `declare_resolved_config!` seam are landed tree code, not packet dependencies.
- Related work, not a blocker: ticket 122 (prime-tower body — no shared helper, no dep); ticket 125 (per-tool model — explicitly NOT this packet's axis: these are `PrintObjectConfig` per-object keys, carried by the existing per-object overlay, not the tool axis); ticket 132 (CONFIG_BLOCK reader contract — canonical spelling rides there); ticket 126 (overlay is origin-aware over every declared field — the per-object path these keys use).
- Unblocks: wayfinder ticket 78 (P71 closes when this packet is authored). No edge to any other draft packet.
- Activation blockers: none. No new deviation ID is minted (zero deviation rows); if implementation surfaces one, re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row.

## Acceptance Criteria

- **AC-1. Given** default config, **when** `ResolvedConfig` is inspected, **then** `make_overhang_printable` is `false`, `make_overhang_printable_angle` is `55.0`, `make_overhang_printable_hole_size` is `0.0`, and each round-trips through `apply_cli_key` exactly. | `cargo test -p slicer-ir --test resolved_config_conical_overhang_tdd schema_declares_conical_overhang_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config (`make_overhang_printable = false`), **when** any mesh (cube, 45-degree ramp, holed plate) is sliced through the new stage, **then** the committed `SliceIR` is byte-identical to the pre-stage baseline. | `cargo test -p slicer-core --features host-algos --test conical_overhang_tdd disabled_leaves_slices_byte_identical 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `make_overhang_printable = true` with `make_overhang_printable_angle = 45.0`, **when** the 45-degree ramp fixture is sliced, **then** every non-first layer's footprint strictly contains its baseline (area grows) and contains the `tan(45°) * layer_h` offset of the layer above. | `cargo test -p slicer-core --features host-algos --test conical_overhang_tdd angle_45_expands_lower_layers 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `make_overhang_printable = true` with `make_overhang_printable_angle = 90.0`, **when** the same ramp is sliced, **then** the output is byte-identical to the disabled baseline (canonical early-return). | `cargo test -p slicer-core --features host-algos --test conical_overhang_tdd angle_90_is_noop 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** `make_overhang_printable = true` on a plate with a small through-hole fully covered by the layer above, **when** `make_overhang_printable_hole_size` is `0.0` versus larger than the hole area, **then** the `0.0` run closes the hole (upper offset fills it) and the larger run preserves it. | `cargo test -p slicer-core --features host-algos --test conical_overhang_tdd hole_size_preserves_small_covered_holes 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** a seeded blackboard with committed `SliceIR`, **when** the prepass order is inspected and the new builtin runs, **then** the stage sits strictly after `PrePass::Slice` and before `PrePass::OverhangAnnotation`, per-object isolation holds (each object's layers mutate only from that object's footprints), and an explicit `true` in the raw source changes output versus default. | `cargo test -p slicer-runtime --test executor prepass_conical_overhang_stage_order 2>&1 | tee target/test-output.log | tail -5`
- **AC-7. Given** the packet's disposition table, **when** it is read, **then** it lists exactly the 3 P71 keys, all wired to the conical pass, with zero declaration-only keys. | `rg -q 'make_overhang_printable_hole_size.*wired' docs/spec_packets/297-conical-overhang-slice-prepass/requirements.md && rg -q 'declaration-only keys: 0' docs/spec_packets/297-conical-overhang-slice-prepass/requirements.md 2>&1 | tee target/test-output.log | tail -3`

## Negative Test Cases

- **AC-N1. Given** no committed `SliceIR`, **when** the new builtin is invoked, **then** it returns the `MissingSliceIr` error and commits nothing (ordering guard, `overhang_annotation_producer` precedent). | `cargo test -p slicer-runtime --test executor prepass_conical_overhang_refuses_without_slice_ir 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3`
- `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3`
- `cargo test -p slicer-core --features host-algos --test conical_overhang_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (prepass ownership; claim-seam rule-4 trigger test)
- `docs/08_coordinate_system.md` - direct range on mm↔unit helpers (offset/area boundaries)
- `docs/ORCASLICER_ATTRIBUTION.md` - direct read (porting header if canonical tests are ported)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P71 rows (owner/tier confirmation)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P71 entry (membership)

## Doc Impact Statement (Required)

- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` section "Quality / Overhangs" - `rg -q 'make_overhang_printable.*297-conical-overhang' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (Step 5: owner confirmed `slice-prepass`, packet linkage)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` section "P71" - `rg -q '297-conical-overhang' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` (Step 5: 3 keys in at packet 297)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintObjectConfig::PrintObjectConfig` (borrow the three defaults exactly: `false`, `55.0`, `0.0`)
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — `PrintObject::apply_conical_overhang` (borrow the pass shape: per-layer/per-region enable checks, `== 90.0` early-return, `tan(angle) * layer_h` offset, hole-area cut from the upper layer; `PrintObject::slice` call site is ordering evidence, not borrowed code)
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::invalidate_state` (named non-borrow: posSlice invalidation has no port counterpart at this seam)
- `OrcaSlicerDocumented/src/libslic3r/Preset.cpp` — blacklisted keys list (named non-borrow)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
