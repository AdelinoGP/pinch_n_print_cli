# Design: 280-bed-mesh-adaptive-placeholders

## Controlling Code Paths

- Primary path: `machine-gcode-emit`'s post-pass entry and its existing `substitute_placeholders` helper (`modules/core-modules/machine-gcode-emit/src/lib.rs`) consume `GCodeCommand` values and site variables.
- Configuration: `[config.schema]` in `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` is discovered by the existing `config.keys()` sweep.
- Input shape: `GCodeCommand::Move` (`crates/slicer-ir/src/slice_ir.rs`) supplies optional `x` and `y`; only moves with both coordinates participate in the bbox.
- Tests: new `modules/core-modules/machine-gcode-emit/tests/bed_mesh_adaptive_tdd.rs` owns schema, math, flavor, and substitution assertions.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- No host key, `ResolvedConfig` field, WIT type, IR field, claim, scheduler edge, or new module is introduced.
- Raw point settings remain config inputs. Derived values are module site variables, not manifest-declared keys or host-injected config keys.
- Runtime config keys are snake_case. Placeholder syntax is the existing single-pass `[snake_case_key]` form.
- Coordinates at this seam are treated in the G-code move coordinate convention (mm); if an implementation crosses an internal-unit boundary it must use `Point2::from_mm`/`mm_to_units` per `docs/08_coordinate_system.md`.
- `ORCA_CONFIG_PADDING` and `crates/slicer-gcode/src/serialize.rs` are out of bounds.
- Guest freshness is checked after changing a guest module manifest/source; a stale artifact is rebuilt before interpreting failures.
- **Carve-out — ADR-0050 requires an amendment (this is the one permitted `docs/adr/` edit).** `docs/adr/0050-custom-gcode-architecture.md` §2 pins the resolvable name set as "**exactly `machine-gcode-emit`'s manifest-declared `[config.schema]` keys**, as handed to the guest through `ConfigView`, plus a `const PLACEHOLDER_ALIASES` table". That is normative, verified at authoring. This packet's derived scalars (`adaptive_bed_mesh_min_x/_y`, `adaptive_bed_mesh_max_x/_y`, `bed_mesh_probe_count_x/_y`, `bed_mesh_algo`) live in the module site-variable map (the `site_lookup` layer over `base_lookup`, the same seam that already carries `layer_num`/`layer_z`/`max_layer_z` from packets 187/188 without an amendment), so the packet contradicts the "exactly" clause and must not do so silently. Obligations, in Step 5: append an `## Amendment — <date> (packet 280)` section to ADR-0050 quoting the contested sentence verbatim and recording the site-variable escape (precedent: `docs/adr/0051-gcode-marker-contract-ownership.md` amendment, packet 187); add a `docs/DEVIATION_LOG.md` row `D-280-ADR-0050-AMENDED` (packet-prefixed convention, precedent `D-285-ADR-0051-AMENDED`; re-derive the free `D-` number when writing it). The rest of ADR-0050 is conformed to: single-pass `[snake_case_key]` substitution stays private to the module, unresolved keys pass through verbatim with one aggregated warning, and no host-injected config key is introduced.

## Code Change Surface

- Selected approach: add four manifest schemas, parse their resolved values, collect the all-Move XY bbox, compute canonical-derived scalar values, merge those values into the existing site-variable map, and let existing substitution resolve them.
- Scalar spelling: `adaptive_bed_mesh_min_x`, `adaptive_bed_mesh_min_y`, `adaptive_bed_mesh_max_x`, `adaptive_bed_mesh_max_y`, `bed_mesh_probe_count_x`, `bed_mesh_probe_count_y`, and `bed_mesh_algo`.
- Math: empty/no-coordinate streams use the existing safe no-op/fallback behavior selected by the implementation seam; nonempty streams expand bbox by margin, clamp to configured limits, floor distances at `1.0`, apply `max(3, ...)`, select algorithm by product, then apply Klipper's per-axis minimum 4.
- Flavor: read optional `gcode_flavor` from `ConfigView`; exact `klipper` activates the floor, all other values and absence use Marlin behavior. Missing flavor is graceful because it is not yet delivered to PostPass modules.
- Rejected alternative: extend substitution to parse `[key[index]]`; rejected because it changes a generic grammar for one feature and is not supported by current single-pass resolution.
- Rejected alternative: add host-injected adaptive keys; rejected because values are emission-time derived and the module already owns this placeholder seam.
- Rejected alternative: reconstruct canonical first-layer convex hull; rejected because no MeshIR/SliceIR source geometry crosses this post-pass seam. The all-Move bbox is conservative for custom motion coverage, but can be wider than extrusion-only geometry and therefore over-probe.

## Files in Scope (read + edit)

- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - declare the four typed inputs and bounds.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - parse config, compute derived values, and expose site variables.
- `modules/core-modules/machine-gcode-emit/tests/bed_mesh_adaptive_tdd.rs` - new focused test binary for schema, math, substitution, and negative padding guard.
- `docs/15_config_keys_reference.md` - generated output after manifest changes.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - annotation row for corrected owner.
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - packet/disposition annotation.

## Read-Only Context

- `modules/core-modules/machine-gcode-emit/src/lib.rs` - locate config sweep, post-pass entry, site-variable construction, and substitution only.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - `[config.schema]` only.
- `crates/slicer-ir/src/slice_ir.rs` - `GCodeCommand::Move` definition only.
- `docs/08_coordinate_system.md` - mm/internal-unit checklist only.
- `docs/03_wit_and_manifest.md` - delegated manifest/config summary.
- Orca sources listed in `requirements.md` - delegated only.

## Out-of-Bounds Files

- `crates/slicer-gcode/src/serialize.rs`, `ORCA_CONFIG_PADDING`, and all host-key docs.
- `crates/slicer-schema/wit/**`, generated bindings, `crates/slicer-ir` definitions, and host emitter code.
- `travel_path`, prime-tower code, and any new module.
- `OrcaSlicerDocumented/**` - delegate; never load directly.
- `target/`, `Cargo.lock`, generated bindings, vendored dependencies, and unrelated packets.

## Expected Sub-Agent Dispatches

- Question: confirm canonical defaults, formulas, hull source, and Klipper floor; scope: listed Orca `PrintConfig.cpp`/`GCode.cpp`; return `SUMMARY` <=200 words; purpose: implementation and parity review.
- Question: locate current config/site-variable/substitution symbols and test fixture constructors; scope: machine-gcode-emit files; return `LOCATIONS` <=20; purpose: bounded implementation reads.
- Question: run each cargo or xtask command; scope: exact command; return `FACT` pass/fail and <=20 failure lines; purpose: all exits.

## Data and Contract Notes

- Manifest contract: point settings use `float-list`, matching the existing printable-area precedent; the probe-distance minimum is component-wise `0`, while runtime math floors at `1.0`.
- WIT/IR boundary: unchanged; `GCodeCommand` is read-only input and derived strings remain module-local site variables.
- Determinism: bbox traversal follows command order but only min/max survive; formatting uses stable decimal/integer/string forms.
- Template contract: users must use scalar PnP tokens. Canonical vector-index syntax is documented as a migration obligation, not accepted by this implementation.

## Locked Assumptions and Invariants

- Owner is `machine-gcode-emit`; all four keys are retained and live.
- No raw input key is emitted as a placeholder.
- Bbox includes every Move with both x and y, including custom-G-code moves.
- `lagrange` iff count product is at most 6; otherwise `bicubic`.
- Optional absent flavor is non-Klipper; exact `klipper` gets minimum 4 per axis.
- No serializer/padding edits.

## Risks and Tradeoffs

- Including custom moves can enlarge the mesh region and increase probing; excluding them would risk under-covering motion represented at the seam.
- A moves bbox is not a convex hull and may contain empty XY area; it is intentionally conservative rather than falsely claiming canonical hull parity.
- Scalar tokens require migration of canonical vector-index templates; supporting both grammars would enlarge a generic parser outside this packet.
- Float formatting and empty streams need explicit tests to prevent unresolved placeholders or non-deterministic output.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (module math and end-to-end substitution)
- Highest-risk dispatch: canonical `GCode::apply_print_config` summary, return `SUMMARY` <=200 words.

## Open Questions

- `[FWD]` A future flavor-carrier packet may make `gcode_flavor` a guaranteed module input; this packet keeps the optional fallback.
- `[FWD]` A future geometry carrier may replace the moves bbox with first-layer convex-hull data; no interface change is introduced here.
- **No `[BLOCK]`.** The current post-pass seam can compute and substitute the selected scalar values without WIT/IR changes.
