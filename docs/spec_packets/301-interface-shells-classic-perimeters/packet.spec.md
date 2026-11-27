---
status: draft
packet: 301-interface-shells-classic-perimeters
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/83-author-packet-p76-multimaterial-multimaterial-advanced-classic-perimeters.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 83 (P76).
---

# Packet Contract: 301-interface-shells-classic-perimeters

## Goal

Make the P76 `interface_shells` key drive the tree pipeline at canonical value semantics — intersecting bodies classify shells collectively unless the key forces each body self-standing — with zero declaration-only keys and byte-identical defaults on single-body prints.

## Scope Boundaries

P76 is one Tier B key owned by multimaterial advanced (`PrintObject.cpp` plus `PerimeterGenerator.cpp` in canonical: the same-region-vs-collective upper/lower arms), live in canonical's slicing pipeline and zero-occurrence as behaviour in this tree. Claim-time grounding (ticket 83) holds it in: no live decision point exists (the tree's `compute_region_updates` reads only same-timeline neighbours — the enabled shape hardcoded), no draft packet owns the decision (packets 299/300 name it as P76's non-borrow), and the tier-table owner is accurate but narrowed for this tree — the seam is the host prepass `commit_shell_classification_builtin`, never a module (the perimeter modules consume preclassified buckets; ticket-36 precedent). The packet wires it through one new `ResolvedConfig` bool (per-object via the existing overlay, explicitly not ticket 125's tool axis): the flag switches the neighbour source for the whole per-timeline computation between same-timeline polys (enabled, pre-packet shape) and the collective all-timelines union at the neighbour slice (disabled, canonical default). Rule 4 does not fire: the arms branch inside one prepass seam, not cross-module algorithm selection — there are no claim holders to create, mirroring the `seam_position` precedent in the map Notes.

## Prerequisites and Blockers

- Depends on: nothing. All symbols below are live on HEAD (verified at authoring); `commit_shell_classification_builtin`, `compute_region_updates`, `resolve_shell_counts`, `clone_region_polys`, `build_region_timelines`, the `declare_resolved_config!` macro, `region_map.config_for`, `RegionMapIR::intern_config`, `split_top_surfaces`, and the arachne `only_one_wall_top` second pass are landed tree code, not packet dependencies. Exact canonical formulas the wiring mirrors are borrowed via implementer-side delegated oracle reads (design.md lists them).
- Related work, not a blocker: draft packet 299 (P73 — the same prepass file gains vertical-shell/extra-solid/combination stages; this packet's resolver call and `compute_region_updates` argument merge with — never duplicate — its edits; sequencing is file-merge order, not an activation dep); ticket 122 (prime-tower body — no shared helper, no dep); ticket 124 (sequential-printing validator — the `Print.cpp` reslice-invalidation entries ride it, named non-borrow); ticket 125 (per-tool model — explicitly NOT this packet's axis: the key is a canonical scalar, carried by the existing per-object overlay, 290/291/296 precedent); ticket 132 (CONFIG_BLOCK reader contract — the padding twin is untouched and the true-value spelling rides there; the `to_config_map` arm shadows the twin with the live value, 284–286 precedent); packet 264 (top-bottom surfaces — no shared helper, no dep).
- Unblocks: wayfinder ticket 83 (P76 closes when this packet is authored). No edge to any other draft packet (packets 299/300 mention the key only as P76's non-borrow — a handoff, not ownership).
- Activation blockers: none. One deviation ID is minted (DEV-192, verified absent from the log and all drafts at authoring); if implementation surfaces another, re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-ir/src/resolved_config.rs`, **when** the `declare_resolved_config!` invocation and `to_config_map` are inspected, **then** they declare `interface_shells` (bool, default `false`) with a `cli` binding of the same snake_case name and a `to_config_map` arm emitting `ConfigValue::Bool`. | `rg -q 'cli "interface_shells"' crates/slicer-ir/src/resolved_config.rs && rg -q '"interface_shells".into()' crates/slicer-ir/src/resolved_config.rs 2>&1 | tee target/test-output.log | tail -3`
- **AC-2. Given** a two-layer two-object fixture where object `b`'s upper layer covers object `a`'s exposed ring (object `a` lower full / upper left-half, object `b` lower full / upper full), sliced with `interface_shells = true` versus `false` (counts `1`/`1`, all else default), **when** their committed `SliceIR` vectors are compared, **then** the `false` run stamps empty `top_solid_fill` on `b`'s lower layer (the ring is covered by `a`'s upper layer) while the `true` run stamps the `x in [10, 20]` mm strip (self-standing, pre-packet shape), and repeated `false` runs are identical. | `cargo test -p slicer-runtime --lib interface_shells_false_uses_collective_upper_cover 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** the mirrored fixture where object `a`'s upper layer overhangs its own lower layer but is supported by object `b`'s lower layer, **when** `true` versus `false` runs are compared, **then** the `true` run stamps the overhang strip as `bottom_solid_fill` on `a`'s upper layer (bottom resting on other material — the canonical extra bottom) while the `false` run stamps it empty. | `cargo test -p slicer-runtime --lib interface_shells_true_marks_bottom_on_other_material 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** a single-object cuboid fixture sliced with the key unset (canonical default `false`) versus `true` (all else default), **when** the committed `SliceIR` vectors are compared, **then** they are byte-identical to each other and to the pre-packet baseline (one timeline: the collective union equals the region's own polys, so the default changes nothing on single-body prints). | `cargo test -p slicer-runtime --lib interface_shells_defaults_are_identity_single_region 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** the packet's disposition table and the deviation log, **when** they are read, **then** the table lists exactly the 1 P76 key, wired, with zero declaration-only keys, `DEV-192` names this packet's divergences in `docs/DEVIATION_LOG.md`, the regenerated `docs/15` tables contain the new key, and the doc-15 freshness gate passes. | `rg -q 'declaration-only keys: 0' docs/spec_packets/301-interface-shells-classic-perimeters/requirements.md && rg -q 'DEV-192' docs/DEVIATION_LOG.md && rg -q 'interface_shells' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** a two-layer two-object fixture with disjoint footprints (no XY overlap), **when** `false` versus `true` runs are compared, **then** they are identical (the collective union of disjoint bodies changes no difference — the all-timelines scope cannot leak across separate bodies). | `cargo test -p slicer-runtime --lib interface_shells_disjoint_objects_unaffected 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --lib interface_shells 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (prepass vs module seam placement; rule-4 trigger test does not fire).
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest stanza shape; `ConfigView::from_declared` whitelist does not apply to host-prepass keys).
- `docs/04_host_scheduler.md` - delegated SUMMARY (prepass ordering: ShellClassification runs before PaintSegmentation, so the gate sees BASE timelines).
- `docs/DEVIATION_LOG.md` - direct read of DEV-166 bool-default precedent + DEV-171 scalar-vs-vector precedent + ID-convention sample.

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-192" - `rg -q 'DEV-192' docs/DEVIATION_LOG.md`.
- `docs/15_config_keys_reference.md` generated tables gain the 1 key - `rg -q 'interface_shells' docs/15_config_keys_reference.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `detect_surfaces_type` same-region-vs-collective upper/lower arms plus the extra non-bridging bottom and the `!spiral_mode` conjunct (borrow the gate shape; the spiral conjunct is explicitly NOT borrowed).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_vertical_shells` `top_bottom_surfaces_all_regions` merge scope (NOT borrowed as a second site — draft-299 merge note; this packet's collective index is the horizontal analog).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `split_top_surfaces` and `process_arachne` same-region upper masks (NOT borrowed as separate arms — inherited via the prepass buckets both modules already consume).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` + `PrintConfig.hpp` + `Preset.cpp` — canonical declaration (coBool, default `false`, print-object scope; borrowed exactly).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
