# Requirements: infill-angle-and-multiline-keys

## Packet Metadata

- Packet directory: `docs/spec_packets/262a-infill-angle-and-multiline-keys/` (the renamed `262-infill-pattern-keys`; split into 262a/262b with explicit user approval, 210a/210b and 238a/238b precedent)
- Slug: `infill-angle-and-multiline-keys`
- Status: `draft`
- Tier: **B** (re-derived — the packet builds decision points inside existing modules; no new module, no new claim; see `design.md` §Tier Derivation)
- Backlog source: `docs/specs/orca-feature-gap/issues/15-author-packet-p08-strength-infill-infill-modules.md`
- Re-authored under the map's **Authoring rules 1–6** (`docs/specs/orca-feature-gap/map.md` §Notes).

## Problem Statement

The previous revision of packet 262 carried seven keys: four wired into real decision points and three ("`sparse_infill_pattern`", "`internal_solid_infill_pattern`", "`gap_fill_target`") declared-with-gap, plus an `ORCA_CONFIG_PADDING` value correction as a deliverable. Authoring rules 1 and 2 prohibit both dispositions.

The split is by mechanism, not by convenience:

- **262a (this packet)** — the four keys whose decision points live *inside* an existing fill module's angle and scan-line code. Nothing new is created; existing algorithms gain a parameter.
- **262b** — the three keys whose decision points are *pattern selection* and *a new fill-side pass*. Under Authoring rule 4 those are claim-holder modules and a `PostPass` claim, not enum values on one module.

Two further corrections land here. First, a key is declared **only** on a module that reads it: the previous revision declared all seven keys on all three fill manifests, which made most of the 17 tables declaration-only. Second, the padding twin edit is removed entirely — `sparse_infill_pattern` is 262b's key and no padding entry is a deliverable of either packet.

## Key Disposition Table

Classification: **(a)** live decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue; **(d)** dead-in-canonical.

| Key | Class | Owning module(s) | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `solid_infill_direction` | **(b)** | `rectilinear-infill`, `gyroid-infill` | solid-role base angle, distinct from the sparse `infill_direction` | AC-2, AC-3 |
| `sparse_infill_rotate_template` | **(b)** | `rectilinear-infill`, `gyroid-infill` | per-layer sparse angle cycled from a comma-separated list | AC-4 |
| `solid_infill_rotate_template` | **(b)** | `rectilinear-infill`, `gyroid-infill` | per-layer solid angle cycled from a comma-separated list | AC-5 |
| `fill_multiline` | **(b)** | `rectilinear-infill` | N parallel sparse lines per scan line at line-width offsets | AC-6 |

Counts: **(a) 0 · (b) 4 · (c) 0 · (d) 0.** Zero declaration-only keys (map gate (a)); every key has a non-default-value AC (map gate (b)).

## Returned to Queue — unimplemented

**None in this packet.** The three keys the previous revision declared with a gap are not returned — they moved to packet **262b-infill-pattern-holder-mapping**, which builds their decision points. Nothing from P08's key list is left unimplemented across the pair.

## Ruled Dead-in-Canonical

**None.** All four keys have read sites inside OrcaSlicer's slicing pipeline under `src/libslic3r/` (see §Per-Key Canonical Evidence).

## In Scope

1. **`solid_infill_direction`** — `rectilinear-infill` and `gyroid-infill` read a separate base angle for the solid roles (`TopSolidInfill`, `BottomSolidInfill`, internal solid); the sparse role continues to read `infill_direction`.
2. **Rotate templates** — both modules resolve a per-layer angle by cycling a comma-separated list by `layer_index`, one list for the sparse role and one for the solid roles. An empty string means "use the base angle"; an unparseable template logs one warn and falls back to the base angle (AC-7).
3. **`fill_multiline`** — `rectilinear-infill` emits N copies of each sparse scan line at line-width offsets, keeping the group period unchanged, sparse role only.
4. **Manifests** — seven net-new `[config.schema]` tables (rectilinear 4, gyroid 3), each with the canonical default/bounds and a `description` naming the in-module consumer.
5. **Guard + fallout** — a new manifest guard test binary in `rectilinear-infill`, bounds/type arms in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, the CONFIG_BLOCK arm in `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`, and regeneration of `docs/15_config_keys_reference.md`.

## Out of Scope

- `sparse_infill_pattern`, `internal_solid_infill_pattern`, `gap_fill_target` — packet 262b.
- `ORCA_CONFIG_PADDING` and every CONFIG_BLOCK twin (Authoring rule 2; AC-N3 asserts a zero-line diff on `crates/slicer-gcode/src/serialize.rs`).
- `fill_multiline` on `gyroid-infill` and `lightning-infill`: offsetting a TPMS curve and a lightning tree are different algorithms, not a parameter. Declaring the key there would be declaration-only. Pinned by AC-N2.
- Any key on `lightning-infill`: it has no scan-line angle and no multiline concept.
- Canonical's rotate-template metalanguage (joints, repeats, unit suffixes). Only the comma-separated list form is ported; anything else is rejected loudly, not silently accepted (AC-7).
- Any WIT interface change, IR schema bump, or new `ResolvedConfig` field — none is required.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — `[config.schema]` shape; fill-role claims.
- `docs/adr/0027-gyroid-multi-role-fill-holder.md` — gyroid's four claims and what a solid-role angle means for a TPMS pattern.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm.
- `docs/15_config_keys_reference.md` — generated.
- `CLAUDE.md` §Guest WASM Staleness — both edited manifests are guest-fingerprint inputs.

## Parity Evidence Standard

Canonical is cited by file + function name, never line number. A worker disputing a claim re-dispatches the read and records the correction in `design.md` §Locked Assumptions rather than editing an AC in place.

## Per-Key Canonical Evidence

| Key | Canonical type | Default | Bounds | Consumer (file · function) |
| --- | --- | --- | --- | --- |
| `solid_infill_direction` | `coFloat` | 45 | 0 … 360 | `Fill.cpp` · `Layer::make_fills` / `group_fills` (solid-role angle selection) |
| `sparse_infill_rotate_template` | `coString` | `""` | — | `Fill.cpp` · `calculate_infill_rotation_angle` (sparse role) |
| `solid_infill_rotate_template` | `coString` | `""` | — | `Fill.cpp` · `calculate_infill_rotation_angle` (solid roles) |
| `fill_multiline` | `coInt` | 1 | 1 … 10 | `FillBase.cpp` · `multiline_fill`; `FillRectilinear.cpp` · `fill_surface_by_multilines`; gated to `erInternalInfill` in `Fill.cpp` · `Layer::make_fills` |

## Acceptance Summary

| AC | Subject | Key proved live at a non-default value |
| --- | --- | --- |
| AC-1 | manifest guard, exact tables and ownership | all four |
| AC-2 | rectilinear solid-vs-sparse angle split | `solid_infill_direction` |
| AC-3 | gyroid solid-vs-sparse angle split | `solid_infill_direction` |
| AC-4 | sparse angle cycles 0/90/0 by layer | `sparse_infill_rotate_template` |
| AC-5 | solid angle cycles 0/90/0 by layer (both modules) | `solid_infill_rotate_template` |
| AC-6 | 3× sparse path count, unchanged period, solid untouched | `fill_multiline` |
| AC-7 | unsupported template warns and falls back | both templates |
| AC-8 | bounds and type rejection | `fill_multiline`, `solid_infill_direction`, templates |
| AC-9 | CONFIG_BLOCK carries explicit values only | all four |
| AC-10 | generated docs rows and unchanged deviation-row count | all four |
| AC-N1 | guard drift fails loudly | all four |
| AC-N2 | lightning declares none; gyroid declares no `fill_multiline` | ownership discipline |
| AC-N3 | zero `ORCA_CONFIG_PADDING` diff | rule 2 |
| AC-N4 | default-path byte identity (additional evidence only) | all four |

## Verification Matrix

| Command | Covers |
| --- | --- |
| `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2, AC-4, AC-5, AC-6, AC-7, AC-N4 |
| `cargo test -p gyroid-infill --test gyroid_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-3, AC-5 |
| `cargo test -p rectilinear-infill --test infill_angle_multiline_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1, AC-N1, AC-N2 |
| `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-8 |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-9 |
| `cargo xtask gen-config-docs --check` + the AC-10 key loop | AC-10 |
| `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs \| grep -cE "^[+-][^+-]"` | AC-N3 |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (both manifests are fingerprint inputs) |
| `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals` | packet gates |

## Step Completion Expectations

- A key's manifest table and its module read land in the same step; no step ends with a declared, unread key.
- The angle work (Steps 2–3) precedes the multiline work (Step 4) because multiline offsets are computed in the rotated frame the angle work establishes.
- The deviation-block row count is captured from disk immediately before the first manifest edit and re-compared in the final step.
- `cargo xtask build-guests --check` must return exit 0 before any behaviour AC is claimed against an integration-level test.

## Context Discipline Notes

- Read budget: standard 120k band. `modules/core-modules/rectilinear-infill/src/lib.rs` and `modules/core-modules/gyroid-infill/src/lib.rs` are the only large files in scope; use ranged reads around the angle computation and the emit loop, never a full read of both in one step.
- Never open `OrcaSlicerDocumented/` directly — dispatch per the obligations below.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills`, `group_fills`, `calculate_infill_rotation_angle`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `multiline_fill`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `fill_surface_by_multilines`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the four key declarations.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.
