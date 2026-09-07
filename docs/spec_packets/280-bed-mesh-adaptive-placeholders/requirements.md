# Requirements: 280-bed-mesh-adaptive-placeholders

## Packet Metadata

- Grouped task IDs: `[]` (wayfinder ticket 53)
- Backlog source: `docs/specs/orca-feature-gap/issues/53-author-packet-p46-printer-machine-bed-mesh-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P46 has four canonical printer/machine settings but no live occurrence in the tree. They are coherent because all four feed one machine-start-G-code bed-mesh calculation. The correct owner is `machine-gcode-emit`, not `crates/slicer-gcode`: this post-pass already performs generic placeholder substitution, while no host emitter or IR carrier exposes adaptive mesh state.

## In Scope

- Declare `bed_mesh_min` and `bed_mesh_max` as two-element `float-list` inputs with defaults `[-99999,-99999]` and `[99999,99999]`.
- Declare `bed_mesh_probe_distance` as a two-element `float-list`, default `[50,50]`, minimum `0` per component.
- Declare `adaptive_bed_mesh_margin` as `float`, default `0.0`, minimum `0.0`.
- Resolve module inputs through the existing `ConfigView`/`config.keys()` sweep.
- Derive scalar site variables `adaptive_bed_mesh_min_x`, `_y`, `adaptive_bed_mesh_max_x`, `_y`, `bed_mesh_probe_count_x`, `_y`, and `bed_mesh_algo`.
- Use the XY bounds of all input `GCodeCommand::Move` values having both `x` and `y`; include custom-G-code moves because the seam cannot distinguish them from extrusion moves.
- Port margin clamping, distance floor, minimum counts, algorithm boundary, and optional Klipper flavor floor.
- Add `tests/bed_mesh_adaptive_tdd.rs` and the required 04/05/documentation regeneration follow-up.

## Out of Scope

- Host keys, `docs/config/host-keys.toml`, `ResolvedConfig`, WIT/IR changes, or a new module.
- `crates/slicer-gcode/src/serialize.rs`, `ORCA_CONFIG_PADDING`, `travel_path`, prime tower, and host emitter changes.
- Canonical first-layer convex hull reconstruction: the post-pass sees emitted moves, not `MeshIR`/`SliceIR` source geometry. The moves bbox is a documented conservative seam approximation, though custom moves can make it wider than the printable extrusion footprint.
- Canonical vector-index placeholder spelling. PnP's substitution resolves one snake_case key per bracket and does not parse `[key[0]]`; scalar names are the supported migration spelling.

## Key Dispositions

| Key | Disposition | Owner and behavior | Acceptance proof |
| --- | --- | --- | --- |
| `adaptive_bed_mesh_margin` | retained, live | machine-gcode-emit margin in mm | AC-2, AC-N1 |
| `bed_mesh_max` | retained, live | machine-gcode-emit upper clamp | AC-1, AC-3 |
| `bed_mesh_min` | retained, live | machine-gcode-emit lower clamp | AC-1, AC-3 |
| `bed_mesh_probe_distance` | retained, live | machine-gcode-emit probe spacing | AC-4, AC-5 |

No key is shed or returned. These are not host keys and must not be added to `docs/config/host-keys.toml`.

## Canonical and PnP Behavior Contract

The derived raw bounds start at the all-Move XY bbox, expand by margin, then clamp to configured min/max. Each axis uses `max(3, ceil(size / max(distance, 1.0)) + 1)`. `lagrange` is selected for count product `<= 6`; otherwise `bicubic`. If `gcode_flavor` is present and equals `klipper`, each count is raised to at least `4`; absent flavor defaults to Marlin behavior. Inputs remain raw config values and are not published as placeholders.

Canonical templates such as `{adaptive_bed_mesh_min[0]}` are migrated to scalar tokens such as `[adaptive_bed_mesh_min_x]`. This is a deliberate recorded divergence, not an extension of the generic substitution grammar. Implementations must keep all derived values in the module site-variable map so ordinary one-pass substitution resolves them.

## Authoritative Docs

- `docs/01_system_architecture.md` - targeted post-pass and site-variable sections.
- `docs/03_wit_and_manifest.md` - delegated manifest type/bounds summary.
- `docs/08_coordinate_system.md` - targeted unit conversion checklist.
- `docs/specs/orca-feature-gap/map.md` - targeted authoring rules and owner correction.

## Acceptance Summary

- Positive: `AC-1` through `AC-7` in `packet.spec.md` prove defaults, geometry, margin, clamps, distance floor, algorithm, flavor fallback, and end-to-end substitution.
- Negative: `AC-N1` rejects malformed float-lists/minimum violations; `AC-N2` protects serializer padding.
- Cross-packet impact: machine-start templates must migrate from canonical vector-index spelling to the scalar PnP spelling; no host config contract changes.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` | All mesh math, schema, and substitution cases | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | Compile all targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | Generated config reference | FACT exit code |
| `cargo xtask check-literals` | Literal gate | FACT exit code |

## Step Completion Expectations

- Manifest declarations and type-rejection tests precede the derived-value implementation.
- The test fixture proves both raw math and the actual substitution path, not only a helper function.
- Scalar placeholder names remain stable; no implementation adds vector-index parsing.
- Any guest artifact freshness check required by the changed module is performed before attributing test failures.

## Context Discipline Notes

- `machine-gcode-emit/src/lib.rs` is long; locate `ConfigView`, `substitute_placeholders`, and post-pass entry symbols before ranged reads.
- Canonical sources are delegated only and are never loaded by the implementer.
- Cargo output is always tee'd to `target/test-output.log`; inspect that file instead of rerunning a truncated command.
