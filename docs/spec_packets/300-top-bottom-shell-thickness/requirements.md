# Requirements: 300-top-bottom-shell-thickness

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/81-author-packet-p74-strength-top-bottom-shells-object-level-planning.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P74 is the shell-thickness floor: two keys whose canonical readers all converge in `PrintObject.cpp` (`discover_horizontal_shells`, `discover_vertical_shells`, the `PrintObject::infill` scatter) and whose behaviour this tree has none of — the tree's shell projection stops at the layer-count floor (`resolve_shell_counts` reads only `top_shell_layers` / `bottom_shell_layers`), so a thin-layer print gets a thinner shell than configured. The tier table sizes it Tier B (new logic in an existing owner) and names the owner object-level planning; claim-time grounding (ticket 81) confirms the owner but narrows it for this tree to the host prepass `commit_shell_classification_builtin` (ticket-36 precedent — the key's decision points span the planner smoothing, both renderers, and the host resolver, so the owner is a seam, not a module). Both keys are zero-occurrence as behaviour, both pass rule 3, and no draft packet owns the decision — packet 299's P73 stages own vertical shells, extra-solid insertion, and sparse combination, not the thickness-vs-count floor — so the packet wires both, sheds none, and returns none.

## In Scope

- `top_shell_thickness` (canonical coFloat, default `0.6`, min 0): prepass top-projection extension — the backward shadow walk that today stops after `k_top` layers keeps walking while the seed-to-neighbour `print_z` distance is below the thickness (the canonical `||` arm with the `EPSILON` margin). `0` disables the extension (count floor only). Default identity holds because the default count floor already covers the default thickness on ordinary layer heights (AC-3).
- `bottom_shell_thickness` (canonical coFloat, default `0.0`, min 0): prepass bottom-projection extension — the forward shadow walk that today stops after `k_bot` layers keeps walking while the neighbour-to-seed `bottom_z` distance is below the thickness. `0` disables the extension (count floor only, which is also the default state).
- Two `ResolvedConfig` fields at canonical defaults, per-object via the existing overlay (290/291/296 precedent — explicitly not ticket 125's tool axis; canonical scalarity IS held, both scalar).
- Locked-path bypass (ADR-0062/0063 conformance): thickness growth neither clips nor merges locked footprints (AC-N1).
- One deviation row (DEV-191) and the `docs/15` regen.

## Out of Scope

- `Print.cpp` reslice-invalidation entries for these keys (invalidation bookkeeping, not slicing behaviour — rides ticket 124, named non-borrow).
- Per-nozzle vector model for either key (canonical declares both scalar — no ticket-125 arm, unlike packets 276/277/279–289).
- Canonical range enforcement (GUI hints only, ticket-113 rule — below-min saturates, never rejects; this packet has no rejection at all, not even strict-parse: both keys are plain floats).
- The `LayerRegion::make_perimeters` spiral-mode gate and the `PrintObjectSlice.cpp` slicing-mode bottom-layer count (spiral-mode keys are unimplemented queue scope — the `spiral_mode` gate has no port counterpart, named non-borrow).
- The `PrintObject::infill` scatter as a second emission site (the prepass projection already carries the thickness into `top_solid_fill` / `bottom_solid_fill`; re-scattering at infill time would double-cover the same layers — named non-borrow, not a stub).
- Gyroid solid-density path (packet-264 omission stands — the thickness extends the shared projection both emitters consume; gyroid's opt-in solid still rides `sparse_infill_density`, named non-borrow).
- `interface_shells` multi-material gating (P76 scope — this packet gates on count + thickness alone, named non-borrow).
- CONFIG_BLOCK emission for these keys (honest absence — prepass inputs, not emitter inputs; spellings ride ticket 132; no `ORCA_CONFIG_PADDING` edits, rule 2).
- Full `TreeSupport3D.cpp` organic engine (DEV-156 stands), prime-tower body (ticket 122), sequential validator (ticket 124).

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (prepass vs module seam; claim-system rule-4 trigger).
- `docs/08_coordinate_system.md` - direct range read (mm↔unit helpers at the thickness-vs-print_z comparison).
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest stanza shape; host-prepass keys need no module manifest — `ConfigView::from_declared` whitelist does not apply).
- `docs/DEVIATION_LOG.md` - direct read (DEV-168/169/171 scalar-vs-vector precedents; DEV-191 minting convention).
- `docs/04_host_scheduler.md` - delegated SUMMARY (prepass ordering: thickness arms ride the existing Pass-2 shadow walks inside `commit_shell_classification_builtin`).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_horizontal_shells` top/bottom projection loops (the count-floor `i < itop` / `i > ibottom` arms plus the `||` thickness arms against `print_z` / `bottom_z` with the `EPSILON` margin, and the two `combine_holes` follow-ups under the `one_more_layer_below_top_bottom_surfaces = false` flag).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_vertical_shells` first/last-layer top/bottom projection loops (the `i < itop || print_z-distance < top_shell_thickness` and `i > ibottom || bottom_z-distance < bottom_shell_thickness` arms).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::infill` scatter loops (the `int(i) - n < num_solid_layers || print_z-distance < top_shell_thickness` top arm and the `n - int(i) < num_solid_layers || bottom_z-distance < bottom_shell_thickness` bottom arm).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical defaults/shapes for both keys (coFloat, top `0.6`, bottom `0.0`, min 0).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-4`; AC-2 pins the thickness-beats-count behaviour at non-default values; AC-3 pins default-path identity (defaults change nothing on the fixture).
- Negative: `AC-N1` (locked-path bypass).
- Cross-packet impact: none — no draft packet consumes these keys; draft packet 299's P73 stages share the prepass file but own different stages (vertical shells, extra-solid insertion, sparse combination); the implementer merges with 299's `resolve_shell_counts` call signature at activation time rather than duplicating it. The `shell_thickness` substring hits in 299 are the P73 `ensure_vertical_shell_thickness` mode key — a different decision, not ownership.
- Disposition table: exactly the 2 P74 keys, all wired, declaration-only keys: 0.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'cli "top_shell_thickness"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "bottom_shell_thickness"' crates/slicer-ir/src/resolved_config.rs 2>&1 \| tee target/test-output.log \| tail -3` | AC-1 field presence | FACT pass/fail |
| `cargo test -p slicer-runtime --lib shell_thickness_extends_projection_past_count 2>&1 \| tee target/test-output.log \| tail -5` | AC-2 thickness beats count | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --lib shell_thickness_defaults_are_identity 2>&1 \| tee target/test-output.log \| tail -5` | AC-3 defaults identity | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `rg -q 'declaration-only keys: 0' docs/spec_packets/300-top-bottom-shell-thickness/requirements.md && rg -q 'DEV-191' docs/DEVIATION_LOG.md && rg -q 'top_shell_thickness' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -5` | AC-4 dispositions + docs | FACT pass/fail |
| `cargo test -p slicer-runtime --lib shell_thickness_bypasses_locked_paths 2>&1 \| tee target/test-output.log \| tail -5` | AC-N1 locks | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | closure gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Steps run in order: ResolvedConfig fields first (later stages read them via `region_map.config_for`), then the resolver + walk extension, then the tests, then DEV-191 + docs regen. The walk-extension step sequences after the resolver lands (same-packet ordering — not a cross-packet dep).
- Shared scratch state: the 6-layer uniform-box fixture built in Step 2 is reused by Steps 2–4; do not rebuild it per step.

## Context Discipline Notes

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` is over 1100 lines — range-read only (`compute_region_updates` lines 351–520, `resolve_shell_counts` lines 1081–1106); delegate all other facts.
- `OrcaSlicerDocumented/` — delegate every read per the obligations above; never load.
- Heavy dispatches (`cargo check`, `cargo clippy`) return FACT pass/fail only, never full logs.
