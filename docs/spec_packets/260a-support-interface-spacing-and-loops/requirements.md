# Requirements: support-interface-spacing-and-loops

## Packet Metadata

- Packet: `260a-support-interface-spacing-and-loops`
- Status: `draft`
- Tier: **B** (the packet builds a decision point — the contact-loop generator)
- Backlog source: wayfinder ticket 18 (`docs/specs/orca-feature-gap/issues/18-author-packet-p11-support-interface-support-planner.md`), map `docs/specs/orca-feature-gap/map.md`
- Split: this packet and `260b-support-interface-fill-claim-holders` replace the pre-rules packet `260-support-interface-keys` (259a/b + 262a/b precedent). Numbers and the `a`/`b` suffixes were re-derived from disk at authoring time.

## Problem Statement

The pre-rules packet 260 declared four support-interface keys, of which two were already live and two were dispositioned "declared-with-gap" — a disposition Authoring rule 1 now prohibits. Re-authored under rules 1–7, the four keys split three ways:

1. `support_interface_spacing` and `support_bottom_interface_spacing` are **live today**: both are declared in `traditional-support.toml` and `tree-support.toml` and both are read in each module's `from_config` and consumed by `pitches_mm` through `slicer_core::support_regularize::interface_density` / `bottom_interface_density`. They stay in this packet as disposition (a) with non-default-value ACs, and they carry two corrections the canonical read forced: the top key's default is canonically 0.5 while the port shipped 0.4 (a real, if small, default-path behaviour change), and the port's `-1 == mirror the top gap` branch is a PnP extension canonical does not have.
2. `support_interface_loop_pattern` has **no decision point in the tree** — zero occurrences in either module. Under rule 1 the packet either builds it or sheds it. This packet **builds** it: canonical's contact-loop pass (`LoopInterfaceProcessor::generate`, dispatched from `generate_support_toolpaths` when `n_contact_loops` is non-zero) is a self-contained pass over the top-interface islands the renderer already computes, immediately upstream of the module's own per-expolygon scan filler (`fill_expolygon` in `traditional-support`, `scan_fill_region` in `tree-support`). That makes the packet Tier B.
3. `support_interface_pattern` selects among *different fill algorithms*. Rule 4 routes cross-module algorithm selection to `claim:*` holders, and that seam does not exist for support interface fill. The key is therefore **not in this packet** (see §Returned to Queue) and is carried by `260b`, which records the blockers.

## In Scope

- Declare `support_interface_loop_pattern` (bool, default false — canonical type correction; it is not an enum) in both renderer manifests and read it in both modules' `from_config`.
- Build the contact-loop pass in both renderers: when the flag is set, emit one closed `SupportInterface` loop per top-interface island along the island boundary, and reduce the area handed to the scan filler by the loop's occupied width so loop and fill do not overlap. Empty/too-small islands degrade to plain fill (AC-N2).
- Align `support_interface_spacing` 0.4 → 0.5 in both manifests, in both modules' fallback constants and their comments, in the `orca-matched-config.json` fixture, and in every test expectation that pinned 0.4 (each re-measured, never weakened).
- Pin the retained `-1` bottom-spacing mirror as a recorded divergence: manifest keeps `min = -1.0`, AC-3 is the behaviour witness, AC-6 keeps `-1.0` legal in the bounds index, and both manifests carry a comment naming the divergence.
- Integration arms: scheduler bounds/type enforcement (AC-6) and the runtime CONFIG_BLOCK arm (AC-7).
- Regenerate `docs/15_config_keys_reference.md` (AC-8).

## Out of Scope

- `support_interface_pattern` and any interface-fill pattern dispatch, filler module, claim, or holder key — `260b`.
- The support planners (`traditional-support-planner`, `tree-support-planner`): they emit `SupportPlanIR` and read no interface configuration. The tier table's `support-planner` owner attribution for these keys is a mis-attribution; the correction rides ticket 18's closure and is reported, not applied here.
- `ORCA_CONFIG_PADDING` and every CONFIG_BLOCK twin. Authoring rule 2: padding is not parity, is never a deliverable, and is not evidence. AC-7 asserts only what the live keys do to the block.
- The per-filament / per-object config model (canonical declares all of these on `PrintObjectConfig`; the port declares them scalar-global in the owner manifests, the queue's established pattern). That model question is Tier-D fog, not this packet.
- Widening the port's `max = 2.0` cap on either spacing key (canonical declares no max). Recorded as a declared-bounds divergence; changing it has no queue backing.

## Returned to Queue — unimplemented, needs interface-fill pattern generators behind a claim seam

- **`support_interface_pattern`** (coEnum `SupportMaterialInterfacePattern`: `auto`, `rectilinear`, `concentric`, `rectilinear_interlaced`, `grid`; default `auto`). Zero occurrences in the tree. Its canonical decision point is the `contact_fill_pattern` selection in `SupportParameters::SupportParameters` plus the filler construction in `SupportCommon.cpp` `generate_support_toolpaths` (`Fill::new_from_type`) and the per-pattern angle rules in `support_interface_angle()`. The missing feature is **interface-fill pattern generators (concentric / grid / rectilinear-interlaced) selectable per region through a claim holder**. Carried by `260b-support-interface-fill-claim-holders`; that packet records why the seam is blocked today. This packet must not declare the key — AC-1 and AC-N1 assert its absence from both manifests so a future worker cannot quietly re-add it as a stub.

## Ruled Dead-in-Canonical

None. All four of ticket 18's keys have read sites inside OrcaSlicer's slicing pipeline under `libslic3r/` (the two spacing keys and `support_interface_pattern` in `Support/SupportParameters.hpp`; `support_interface_loop_pattern` in `Support/SupportCommon.cpp` and `Support/SupportMaterial.hpp`), so Authoring rule 3 rules none of them out of scope.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated; regenerate with `cargo xtask gen-config-docs`, verify with `--check`.
- `docs/03_wit_and_manifest.md` — `[config.schema]` contract and the support-family claims vocabulary (`support-generator` + `support-family:<id>`); this packet adds no claim.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; the loop offset distance is scaled, never raw mm.
- `docs/01_system_architecture.md` § Claim System — read only to confirm this packet needs no claim change (the loop pass is emitted by the module already holding `support-generator`).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the three keys' canonical `def()`s (types, defaults, bounds).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `SupportParameters::SupportParameters` (the interface spacing/density formulas).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_toolpaths` (`n_contact_loops` dispatch) and `LoopInterfaceProcessor::generate` (**the generator this packet ports** — loop count, inward offset step, fill-area trimming, empty-result path).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp` — `SupportMaterial::has_contact_loops`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::generate_toolpaths`.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::invalidate_state_by_config_options`.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

## Parity Evidence Standard

Canonical behaviour is cited by file + function name only, never by line number (`CLAUDE.md` § OrcaSlicer Citation Style). In-tree code is cited by crate-qualified path + symbol name. Every canonical claim below was produced by a delegated read (2026-08-31, carried forward from the pre-rules packet 260 and unchanged by the re-authoring); a worker who disputes one re-dispatches rather than reading `OrcaSlicerDocumented/` directly.

## Per-Key Canonical Evidence

| Key | Canonical type | Canonical default | Bounds | Manifest declaration | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `support_interface_spacing` | coFloat | `0.5` (port shipped 0.4 — **aligned by this packet**) | min 0, no max (port keeps max 2.0 — declared-bounds divergence) | float, default 0.5, min 0.0, max 2.0 | `SupportParameters::SupportParameters` (`top_interface_spacing = (ironing ? 0 : value) + flow.spacing()`; `top_interface_density = min(1, flow.spacing()/top_interface_spacing)`); `TreeSupport::generate_toolpaths` (same formula); `PrintObject::invalidate_state_by_config_options` | **(a) live** — read in both modules' `from_config`, consumed by `pitches_mm` via `slicer_core::support_regularize::interface_density`; AC-2 asserts the behaviour change at non-default 0.2 / 1.2 |
| `support_bottom_interface_spacing` | coFloat | `0.5` (matches port) | min 0, **no `-1` sentinel** (port keeps min -1.0 + mirror — recorded divergence, user ruling) | float, default 0.5, min -1.0, max 2.0 | `SupportParameters::SupportParameters` (`bottom_interface_spacing = value + flow.spacing()`; `bottom_interface_density = min(1, flow.spacing()/bottom_interface_spacing)`); `TreeSupport::generate_toolpaths`; `PrintObject::invalidate_state_by_config_options` | **(a) live** — `bottom_interface_density` mirrors the formula; the `< 0.0 → top gap` branch in both modules' `pitches_mm` is the PnP extension. AC-3 asserts the change at non-default 0.2 / 1.6 and pins `-1.0` as the mirror witness |
| `support_interface_loop_pattern` | **coBool** (type correction: not an enum) | `false` | — | bool, default false | `SupportCommon.cpp` `generate_support_toolpaths` (`loop_interface_processor.n_contact_loops = config.support_interface_loop_pattern.value ? 1 : 0`) and `LoopInterfaceProcessor::generate`; `SupportMaterial.hpp` `SupportMaterial::has_contact_loops` | **(b) built by this packet** — zero occurrences at authoring; the packet ports the contact-loop pass into both renderers. AC-4/AC-5 assert the behaviour change at non-default `true`; AC-N2 pins the degenerate-island path |
| `support_interface_pattern` | coEnum `SupportMaterialInterfacePattern` | `auto` | — | **not declared** | `SupportParameters::SupportParameters` (`contact_fill_pattern` branch order: `smipGrid`→`ipGrid`; `smipRectilinearInterlaced`→`ipRectilinear`; (`smipAuto` ∧ zero-gap) ∨ `smipConcentric`→`ipConcentric`; density > 0.95→`ipRectilinear`; else `ipSupportBase`) + `support_interface_angle()`; `SupportCommon.cpp` `generate_support_toolpaths` (`Fill::new_from_type`) | **(c) returned to queue** — needs interface-fill pattern generators behind a claim seam; carried by `260b` |

### Wiring notes (port-specific decisions the canonical reads forced)

- **Default alignment.** Canonical `support_interface_spacing` is 0.5; the port shipped 0.4 in both modules with comments asserting Orca's default is 0.4 (mis-derived — packet 238c had already corrected the *bottom* key to 0.5 in the same family pass). Aligning changes default output: the interface pitch is `gap + flow spacing`, so it grows by 0.1 mm and default interface fill becomes slightly sparser. That is a real default-path behaviour change, deliberately taken, and it removes the two `support_interface_spacing` rows from the generated deviations block. **Ledger-fact discipline:** the block measured 26 data rows on 2026-09-01, of which exactly 2 are these; AC-8 asserts the delta (-2) and zero remaining `support_interface_spacing` rows rather than an absolute post-count, because that count moves whenever any other packet lands.
- **Mirror sentinel kept (user ruling, carried forward).** Canonical `support_bottom_interface_spacing` has no `-1` sentinel — that sentinel belongs to a *different* canonical key, `support_interface_bottom_layers`. The port's `bottom_interface_spacing_mm < 0.0 → top gap` branch in both modules' `pitches_mm` is therefore a PnP extension. It is retained and recorded, not aligned away.
- **Contact loops are in-module, and that is the architecture's answer, not a shortcut.** The loop pass consumes the top-interface geometry the renderer already holds and writes `SupportInterface` paths the renderer already owns by holding `support-generator`. No new claim, no new IR field, no host special case. Rule 4's trigger test is explicit that a module branching over a mode it implements itself is not a claim-seam candidate — a bool that turns one extra pass on inside the owning module is exactly that case. Canonical agrees structurally: `LoopInterfaceProcessor` is a member of the support toolpath generator, not a pluggable filler.
- **Owner correction (report-only).** The tier table's owner for these keys is `support-planner`, the claim held by the two *planner* modules. Neither planner reads interface configuration; the decision points live in `traditional-support` and `tree-support`. This packet declares in the decision-point modules; the `04-asset-tier-assignment.md` owner row correction is listed for ticket 18's closure and is not applied by this packet.
- **Tier re-derivation.** The pre-rules packet was Tier A plumbing. Building the contact-loop generator makes this packet **Tier B** (rule 1: a packet that builds a decision point is B or C). The two spacing keys remain A-grade work inside a B packet.
- **CONFIG_BLOCK.** `SUPPORT_CONFIG_DEFAULTS` in `crates/slicer-gcode/src/serialize.rs` is `support_expansion` / `support_top_z_distance` / `support_bottom_z_distance` only (re-derived from disk at authoring), and no `support_interface*` key appears in `ORCA_CONFIG_PADDING`. At defaults the block therefore carries zero lines for this packet's keys; explicit values reach it once through the sorted raw-config dump. No padding twin is added or corrected.

## Acceptance Summary

| AC | Assertion | Key(s) | Non-default value asserted |
| --- | --- | --- | --- |
| AC-1 | Manifest schema for the three keys in both modules; `support_interface_pattern` absent | all three | — (schema pin; behaviour is AC-2/3/4/5) |
| AC-2 | Top-interface path count moves with the key | `support_interface_spacing` | `0.2`, `1.2`, `0.4` |
| AC-3 | Bottom-interface path count moves; `-1` mirrors top | `support_bottom_interface_spacing` | `0.2`, `1.6`, `-1.0` |
| AC-4 | One closed loop per island + fewer scan lines (traditional) | `support_interface_loop_pattern` | `true` |
| AC-5 | Same on the tree family; `false` is baseline-identical | `support_interface_loop_pattern` | `true` |
| AC-6 | Bounds/type rejection; `-1.0` stays legal | all three | `"yes"`, `-0.5`, `-2.0`, `-1.0` |
| AC-7 | CONFIG_BLOCK carries explicit values once, nothing at defaults | all three | `0.8`, `true` |
| AC-8 | Generated doc rows + deviation-row delta | `support_interface_spacing`, `support_interface_loop_pattern` | — (doc regen) |
| AC-N1 | Schema guard fails on drift or on a re-added `support_interface_pattern` | all four | — |
| AC-N2 | Degenerate island degrades to plain fill | `support_interface_loop_pattern` | `true` |

Every key kept by this packet has at least one AC asserting a behaviour change at a non-default value (rule 6b): `support_interface_spacing` → AC-2, `support_bottom_interface_spacing` → AC-3, `support_interface_loop_pattern` → AC-4/AC-5. No AC's only evidence is default-path identity.

## Verification Commands

| Command | Covers | Return format |
| --- | --- | --- |
| `cargo test -p traditional-support --test support_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1 | FACT pass/fail |
| `cargo test -p tree-support --test support_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N1 | FACT pass/fail |
| `cargo test -p traditional-support --test traditional_support_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2 | FACT pass/fail |
| `cargo test -p tree-support --test tree_support_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-3 | FACT pass/fail |
| `cargo test -p traditional-support --test support_contact_loops_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-4, AC-N2 | FACT pass/fail |
| `cargo test -p tree-support --test support_contact_loops_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-5 | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-6 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-7 | FACT pass/fail |
| `cargo xtask gen-config-docs --check && rg -q 'support_interface_loop_pattern' docs/15_config_keys_reference.md; echo "exit=$?"` | AC-8 | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (both manifests + both `src/lib.rs` are fingerprint inputs) | FACT exit code |
| `cargo check --workspace --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo xtask check-literals` | gates | FACT pass/fail |

## Step Completion Expectations

- No step may declare a key it does not also make behaviour-visible in the same packet (rule 1). The manifest step and the behaviour step for `support_interface_loop_pattern` may be separate steps but must both be inside this packet's completion gate.
- The spacing alignment step owns every 0.4 expectation in the two modules' existing tests and the shared fixture — a worker must re-measure each, never relax an assertion to absorb the shift.
- The contact-loop step owns the `SupportInterface` path shape for both families; if the two renderers cannot share the pass without a `slicer-core` helper, adding that helper is in scope for that step (it is a host-side algorithm crate, not a new contract).

## Context Discipline Notes

Read budget 120k. Delegate every cargo run, every `OrcaSlicerDocumented/` read, and the `docs/15_config_keys_reference.md` regeneration check. Both module `src/lib.rs` files are long — use ranged reads anchored on `from_config`, `run_support`, `pitches_mm`, and the per-expolygon filler (`fill_expolygon` in `traditional-support`, `scan_fill_region` in `tree-support`) — never whole-file loads.
