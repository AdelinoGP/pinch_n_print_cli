# Requirements: 301-interface-shells-classic-perimeters

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/83-author-packet-p76-multimaterial-multimaterial-advanced-classic-perimeters.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P76 is the multi-material shell gate: one bool whose canonical readers all converge in `PrintObject.cpp` (`detect_surfaces_type` same-region-vs-collective upper/lower arms, `discover_vertical_shells`' `top_bottom_surfaces_all_regions` merge) plus `PerimeterGenerator.cpp` (`split_top_surfaces` / `process_arachne` same-region upper masks) — and whose behaviour this tree has none of. Every timeline the tree classifies reads only its own polys: `compute_region_updates` clones same-`(object_id, region_id)` neighbours, so intersecting bodies hide each other's shells exactly the way canonical's disabled state does, unconditionally. The tier table sizes it Tier B (new logic in an existing owner) and names the owner classic-perimeters; claim-time grounding (ticket 83) confirms the owner but narrows it for this tree to the host prepass `commit_shell_classification_builtin` (ticket-36 precedent — the key's decision points span the shell classifier, both perimeter renderers, and the host resolver, so the owner is a seam, not a module). The key is zero-occurrence as behaviour, passes rule 3, and no draft packet owns the decision — packets 299/300 each name it as P76's named non-borrow — so the packet wires it, sheds none, and returns none.

## In Scope

- `interface_shells` (canonical coBool, default `false`): prepass neighbour-source gate — the Pass 1 differences that today subtract same-timeline neighbours subtract the collective all-timelines union at the neighbour slice instead, when the resolving config says `false` (canonical default); when `true`, the pre-packet same-timeline shape holds (self-standing bodies, each carrying full shells). Disabled-by-default is identity on single-body prints (one timeline: the union equals the region's own polys, AC-4) and changes intersecting multi-body prints at `false` (collective cover, AC-2); at `true` an overhang resting on another body gains the canonical extra non-bridging bottom (AC-3).
- One `ResolvedConfig` bool field at the canonical default, per-object via the existing overlay (290/291/296 precedent — explicitly not ticket 125's tool axis; canonical scalarity IS held, scalar coBool).
- The `to_config_map` arm (284–286 precedent): `ConfigValue::Bool` emission so the live value shadows the frozen `("interface_shells", "0")` padding twin (rule 2 — the table is load-bearing, never edited).
- Locked-path bypass (ADR-0062/0063 conformance): the gate reads locked footprints as cover but never clips nor merges them (AC-N1 is the no-leak invariant for this packet's only shared-boundary risk).
- One deviation row (DEV-192) and the `docs/15` regen.

## Out of Scope

- `Print.cpp` reslice-invalidation entries for this key (invalidation bookkeeping, not slicing behaviour — rides ticket 124, named non-borrow).
- Per-nozzle vector model (canonical declares the key scalar — no ticket-125 arm, unlike packets 276/277/279–289).
- Canonical range enforcement (GUI hints only, ticket-113 rule — a bool has no range; the wrong-variant spelling `Bool` where `Int` is configured is the typed `extract_bool` `TypeMismatch`, never a saturate; no separate rejection criterion).
- The `!spiral_mode` conjunct of canonical's gate (`m_config.interface_shells` is AND-ed with `!spiral_mode` in `detect_surfaces_type`): `spiral_mode` is unimplemented queue scope — the gate keys on `interface_shells` alone, named non-borrow.
- The `discover_vertical_shells` collective merge as a second site (draft 299's vertical-shell stage owns that file's stage list; this packet's collective index is the horizontal analog riding `resolve_shell_counts`' pattern — a named non-borrow, not a stub; whichever lands second rebases onto the first).
- The perimeter-side same-region upper masks as separate arms (`split_top_surfaces`, arachne `process_arachne`): inherited through the prepass buckets both modules already consume (`top_solid_fill` carve — the arachne second pass documents its divergence from Orca's inline `diff_ex` derivation in-module) — named non-borrows, not stubs.
- Arachne parity beyond the bucket contract (the arachne twin consumes `top_shell_index` + the same buckets for `only_one_wall_top`; the gate changes the buckets, not the consumers).
- `ORCA_CONFIG_PADDING` edits (the `"0"` twin stays — load-bearing for the ≥80 floor; the true spelling rides 132).
- Prime-tower body (ticket 122), sequential validator (ticket 124).

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (prepass vs module seam; claim-system rule-4 trigger).
- `docs/04_host_scheduler.md` - delegated SUMMARY (prepass ordering: ShellClassification runs before PaintSegmentation — the gate sees BASE timelines, before paint splits them).
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest stanza shape; host-prepass keys need no module manifest — `ConfigView::from_declared` whitelist does not apply).
- `docs/DEVIATION_LOG.md` - direct read (DEV-166 bool-default precedent; DEV-171 scalar-vs-vector precedent; DEV-192 minting convention).
- `docs/02_ir_schemas.md` - delegated SUMMARY (five-way partition invariant the lock-bypass arm must preserve).

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

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `detect_surfaces_type` same-region-vs-collective upper/lower arms plus the extra non-bridging bottom and the `!spiral_mode` conjunct (borrow the gate shape; the spiral conjunct is explicitly NOT borrowed).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_vertical_shells` `top_bottom_surfaces_all_regions` merge scope (NOT borrowed as a second site — draft-299 merge note; this packet's collective index is the horizontal analog).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `split_top_surfaces` and `process_arachne` same-region upper masks (NOT borrowed as separate arms — inherited via the prepass buckets both modules already consume).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` + `PrintConfig.hpp` + `Preset.cpp` — canonical declaration (coBool, default `false`, print-object scope; borrowed exactly).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-5`; AC-2 pins the collective-cover behaviour at the canonical default, AC-3 pins the true-state extra bottom, AC-4 pins default-path identity on single-body prints (the inverse of 299's emitting default).
- Negative: `AC-N1` (disjoint bodies unaffected — the all-timelines scope cannot leak).
- Cross-packet impact: none — no draft packet consumes this key; draft packet 299's P73 stages share the prepass file but own different decisions (vertical shells, extra-solid insertion, sparse combination); the implementer merges with 299's `resolve_shell_counts` call signature at activation time rather than duplicating it. The 299/300 `interface_shells` mentions are P76's handoff non-borrows, not ownership.
- Disposition table: exactly the 1 P76 key, wired, declaration-only keys: 0.

| Key | Canonical shape | Decision point (this tree) | Disposition |
| --- | --- | --- | --- |
| `interface_shells` | coBool, default `false` (`PrintConfig.cpp`) | prepass neighbour-source gate in `compute_region_updates`, fed by a `resolve_shell_counts`-pattern resolver read | wired |

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'cli "interface_shells"' crates/slicer-ir/src/resolved_config.rs && rg -q '"interface_shells".into()' crates/slicer-ir/src/resolved_config.rs 2>&1 \| tee target/test-output.log \| tail -3` | AC-1 field + map arm presence | FACT pass/fail |
| `cargo test -p slicer-runtime --lib interface_shells_false_uses_collective_upper_cover 2>&1 \| tee target/test-output.log \| tail -5` | AC-2 collective cover | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --lib interface_shells_true_marks_bottom_on_other_material 2>&1 \| tee target/test-output.log \| tail -5` | AC-3 extra bottom | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --lib interface_shells_defaults_are_identity_single_region 2>&1 \| tee target/test-output.log \| tail -5` | AC-4 defaults identity | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `rg -q 'declaration-only keys: 0' docs/spec_packets/301-interface-shells-classic-perimeters/requirements.md && rg -q 'DEV-192' docs/DEVIATION_LOG.md && rg -q 'interface_shells' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -5` | AC-5 dispositions + docs | FACT pass/fail |
| `cargo test -p slicer-runtime --lib interface_shells_disjoint_objects_unaffected 2>&1 \| tee target/test-output.log \| tail -5` | AC-N1 no-leak | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | closure gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Steps run in order: ResolvedConfig field first (later stages read it via `region_map.config_for`), then the resolver + gate, then the tests, then DEV-192 + docs regen. The gate step sequences after the resolver lands (same-packet ordering — not a cross-packet dep).
- Shared scratch state: the two-object ring/strip fixture shape built in Step 2 is mirrored by Step 3; do not rebuild the harness per step.

## Context Discipline Notes

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` is over 1100 lines — range-read only (`compute_region_updates` lines 351–520, `resolve_shell_counts` lines 1081–1106); delegate all other facts.
- `OrcaSlicerDocumented/` — delegate every read per the obligations above; never load.
- Heavy dispatches (`cargo check`, `cargo clippy`) return FACT pass/fail only, never full logs.
