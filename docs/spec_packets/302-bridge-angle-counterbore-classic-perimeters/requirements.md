# Requirements: 302-bridge-angle-counterbore-classic-perimeters

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/84-author-packet-p77-quality-bridging-classic-perimeters.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P77 is the external bridge-direction gate plus the stepped-hole bridge treatment: two keys whose canonical readers converge in `LayerRegion.cpp` (`process_external_surfaces` top and bottom custom-angle arms) and `PerimeterGenerator.cpp` (`process_no_bridge` island separation plus `BridgeDetector` coverage and filled-vs-partial handling), plus `Layer.cpp` (chbFilled extra-fill recovery) and `PrintObject.cpp` (chbFilled slice-union counting sacrificial fill as solid above) — and whose behaviour this tree has none of. External orientation is always auto-detected (`update_external_bridge_orientation` overwrites the mesh heuristic unconditionally; the only override arm in the tree, `determine_bridging_angle`'s, serves internal bridges from `internal_bridge_angle`), and stepped holes get no bridge treatment at all (zero `counterbore` occurrences under `crates/`/`modules/`). The tier table sizes it Tier B (new logic in an existing owner) and names the owner classic-perimeters + arachne-perimeters; claim-time grounding (ticket 84) confirms the owner but narrows it for this tree to the host prepass `commit_shell_classification_builtin` (ticket-36 precedent — the keys' decision points span the shell classifier, the bridge gates, and the host resolver, so the owner is a seam, not a module). Both keys are zero-occurrence as behaviour, pass rule 3, and no draft packet owns either decision — implemented packet 235 built the auto-detection the override rides (its split note explicitly returns the override-key plumbing to this ticket), packets 299/300 each name unrelated keys — so the packet wires both, sheds none, and returns none.

## In Scope

- `bridge_angle` (canonical coFloat, default `0.0`, min 0, max 180): external-bridge direction override — after `update_external_bridge_orientation` derives the detected direction from the gated geometry, a `> 0` value overwrites `region.bridge_orientation_deg` verbatim; `0` keeps the detected value (pre-packet shape). The `> 0` gate plus verbatim overwrite are the exact semantics of the live `internal_bridge_angle` arm (`determine_bridging_angle`), so the implementer mirrors a shipped arm rather than inventing one. Orientation flows to the bridge fillers through the existing `bridge_orientation_deg` bucket both holders already consume (`rectilinear-infill` bridge arm, `wave-overhangs` reader) — no IR field, no WIT line, no filler change.
- One `ResolvedConfig` float field at the canonical default, per-object via the existing overlay (290/291/296 precedent — explicitly not ticket 125's tool axis; canonical scalarity IS held, per-region scalar `PrintRegionConfig`).
- `counterbore_hole_bridging` (canonical coEnum, default `none`, spellings `none` / `partiallybridge` / `sacrificiallayer`): hole-bearing unsupported-span authoring in `commit_shell_classification_builtin` after its existing bridge gates — for each region, unsupported spans (this layer's polys minus the committed lower layer) that carry a hole and are not already `bridge_areas` are authored into `bridge_areas`. In `sacrificiallayer` mode the whole uncovered span is authored (hole interior plus rim); in `partiallybridge` mode only the rim spans (unsupported minus hole interiors) are authored and hole interiors stay unbridged. `none` authors nothing (pre-packet shape). Authored spans are gate survivors by construction: they carry a detected orientation from `detect_bridging_direction_deg` against the raw lower contours (the packet-235 seam), so they need no second gate pass.
- One `ResolvedConfig` string-enum field at the canonical default, per-object via the existing overlay (same precedent; canonical scalarity IS held). Unknown spellings fall back to `none` (the `flat_bridge_closing_join` precedent — unknown values fall back, never a silent new behaviour; AC-N1).
- No canonical numeric range rejection anywhere (ticket-113 rule — canonical mins/maxes are GUI hints; a `> 180` or negative angle saturates at the `> 0` gate's natural handling, never a rejection criterion).
- Locked-path bypass (ADR-0062/0063 conformance): the override reads orientation only; the counterbore stage neither clips nor links locked paths (AC-N2 is the no-leak invariant for this packet's only shared-boundary risk).
- One deviation row (DEV-193) and the `docs/15` regen.

## Out of Scope

- `relative_bridge_angle` (canonical coBool, default `false`): the relative-application companion of `bridge_angle` — absent from the gap source, the queue, and the tree (verified: zero hits). Named non-borrow: the override applies absolutely, never relative to the detected direction.
- `align_infill_direction_to_model` (the model-rotation offset): draft packet 262a's scope (folded in from ticket 35) — its offset lands on the angles 262a introduces. Named non-borrow: this packet's override applies without the rotation offset.
- Canonical's `BridgeDetector` coverage sweep as the bridgeability test: the port's committed-layer span gate (`gate_bridge_areas_by_unsupported_span`) plus the raw-contour span test are the bridgeability decision here; no detector port, no anchor-band geometry.
- `Layer.cpp`'s chbFilled extra-fill recovery: canonical recovers fills lost in region-merge splitting — this port has no region-merge step that loses fills, so there is nothing to recover. Named non-borrow.
- `PrintObject.cpp`'s chbFilled slice-union (`layerm_slices_surfaces` unions `fill_surfaces` so the layer above counts sacrificial fill as solid support): a support-map restructure in the shell classifier's neighbour reads — out of scope, recorded divergence DEV-193(c).
- Per-nozzle vector model (canonical declares both keys per-region scalar — no ticket-125 arm, unlike packets 276/277/279–289).
- `Print.cpp` reslice-invalidation entries for these keys (invalidation bookkeeping, not slicing behaviour — rides ticket 124, named non-borrow).
- `ORCA_CONFIG_PADDING` edits (neither key has a padding twin — honest absence, pinned by AC-6; the true-value spellings ride 132).
- Prime-tower body (ticket 122), sequential validator (ticket 124).

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (prepass vs module seam; claim-system rule-4 trigger).
- `docs/04_host_scheduler.md` - delegated SUMMARY (prepass ordering: ShellClassification runs before PaintSegmentation — the stage sees BASE timelines, before paint splits them).
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest stanza shape; host-prepass keys need no module manifest — `ConfigView::from_declared` whitelist does not apply).
- `docs/DEVIATION_LOG.md` - direct read (DEV-171 scalar-vs-vector precedent; DEV-193 minting convention).
- `docs/02_ir_schemas.md` - delegated SUMMARY (five-way partition invariant the authored spans must preserve).

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` + `PrintConfig.hpp` + `Preset.cpp` — canonical declarations (`bridge_angle` coFloat default `0`, min 0 max 180; `counterbore_hole_bridging` coEnum default `chbNone`, spellings `none`/`partiallybridge`/`sacrificiallayer`; borrowed exactly).
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `process_external_surfaces` top and bottom `bridge_angle` arms (`> 0` gate, absolute vs `relative_bridge_angle`-relative application, `align_infill_direction_to_model` offset; borrow the gate shape — the relative and align companions are explicitly NOT borrowed).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_no_bridge` island separation, `BridgeDetector` coverage, and filled-vs-partial handling (borrow the mode distinction; the detector math and anchor-band shaping are explicitly NOT borrowed — the span gate is the port's bridgeability test).
- `OrcaSlicerDocumented/src/libslic3r/Layer.cpp` — `make_perimeters` chbFilled extra-fill recovery (NOT borrowed — the port has no region-merge step that loses fills).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `detect_surfaces_type` chbFilled slice-union counting sacrificial fill as solid above (NOT borrowed — above-layer support-map restructure is out of scope, recorded divergence).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `infill_direction` bridge-angle consumption (NOT borrowed as a separate arm — orientation flows to fillers through the existing `bridge_orientation_deg` bucket both holders already consume).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-6`; AC-2 pins the override at a non-default value, AC-3 pins whole-span authoring, AC-4 pins the rim-only distinction, AC-5 pins default-path identity for both keys.
- Negative: `AC-N1` (unknown spelling falls back to `none`), `AC-N2` (hole-free spans unaffected — the stage only fires on hole-bearing spans).
- Cross-packet impact: none — no draft packet consumes either key; draft packet 299's P73 stages share the prepass file but own different decisions (vertical shells, extra-solid insertion, sparse combination); draft packet 262a owns the rotation offset as a named non-borrow; the implementer merges with 299's resolver call neighbourhood at activation time rather than duplicating it. Bridge-parity packets 233/234/234a/235 (in `docs/spec_packets/_OLD/`, `status: implemented`) built the buckets and gates this packet consumes — their symbols are reused, not changed.
- Disposition table: exactly the 2 P77 keys, both wired, declaration-only keys: 0.

| Key | Canonical shape | Decision point (this tree) | Disposition |
| --- | --- | --- | --- |
| `bridge_angle` | coFloat, default `0.0` (`PrintConfig.cpp`) | post-detection overwrite of `region.bridge_orientation_deg`, fed by a `resolve_shell_counts`-pattern resolver read | wired |
| `counterbore_hole_bridging` | coEnum, default `none` (`PrintConfig.cpp`; `PrintConfig.hpp` `CounterboreHoleBridgingOption`) | hole-bearing unsupported-span authoring into `region.bridge_areas` in `commit_shell_classification_builtin`, fed by a `resolve_shell_counts`-pattern resolver read | wired |

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'cli "bridge_angle"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "counterbore_hole_bridging"' crates/slicer-ir/src/resolved_config.rs && rg -q '"counterbore_hole_bridging".into()' crates/slicer-ir/src/resolved_config.rs 2>&1 \| tee target/test-output.log \| tail -3` | AC-1 fields + map arms presence | FACT pass/fail |
| `cargo test -p slicer-runtime --lib bridge_angle 2>&1 \| tee target/test-output.log \| tail -5` | AC-2 override | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --test executor counterbore_filled_authors_hole_interior_as_bridge 2>&1 \| tee target/test-output.log \| tail -5` | AC-3 whole-span authoring | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --test executor counterbore_partial_leaves_hole_interior_unbridged 2>&1 \| tee target/test-output.log \| tail -5` | AC-4 rim-only distinction | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --lib counterbore_defaults_leave_regions_untouched 2>&1 \| tee target/test-output.log \| tail -5` | AC-5 defaults identity | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `rg -q 'declaration-only keys: 0' docs/spec_packets/302-bridge-angle-counterbore-classic-perimeters/requirements.md && rg -q 'DEV-193' docs/DEVIATION_LOG.md && rg -q 'counterbore_hole_bridging' docs/15_config_keys_reference.md && rg -q 'bridge_angle' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -5` | AC-6 dispositions + docs | FACT pass/fail |
| `cargo test -p slicer-runtime --lib counterbore_unknown_spelling_falls_back_to_none 2>&1 \| tee target/test-output.log \| tail -5` | AC-N1 fallback | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --lib counterbore_hole_free_spans_unaffected 2>&1 \| tee target/test-output.log \| tail -5` | AC-N2 hole-free identity | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | closure gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Steps run in order: ResolvedConfig fields first (later stages read them via `region_map.config_for`), then the override arm, then the counterbore stage, then the tests, then DEV-193 + docs regen. The stage step sequences after the override lands (same-packet ordering — not a cross-packet dep).
- Shared scratch state: the two-layer stepped-hole fixture shape built in Step 2 is reused by Steps 3–4; do not rebuild the harness per step.

## Context Discipline Notes

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` is over 1100 lines — range-read only (`commit_shell_classification_builtin` bridge-gate neighbourhood lines 200–260, `resolve_shell_counts` lines 1169–1194); delegate all other facts.
- `crates/slicer-core/src/algos/prepass_slice.rs` is over 1100 lines — range-read only (`update_external_bridge_orientation` lines 604–613, `assemble_bridge_areas` lines 206–262); delegate all other facts.
- `OrcaSlicerDocumented/` — delegate every read per the obligations above; never load.
- Heavy dispatches (`cargo check`, `cargo clippy`) return FACT pass/fail only, never full logs.
