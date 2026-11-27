# Requirements: 278-gcode-output-emitter-modes

## Packet Metadata

- Grouped task IDs: `[]` (wayfinder ticket 51; no current `docs/07_implementation_status.md` ownership row)
- Backlog source: `docs/specs/orca-feature-gap/issues/51-author-packet-p44-others-g-code-output-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P44 grouped six live OrcaSlicer keys under the host emitter, but the tree has only partial `gcode_flavor` wiring and no object-marker, verbose-comment, or configurable internal-retraction decision. Claim-time grounding also found two owner corrections: output filename choice happens after slicing in the CLI/export lifecycle, and retract/no-retract policy is explicitly module-owned by `path-optimization-default`. The packet implements the five coherent live output decisions without turning the emitter into a filesystem owner or overriding precomputed travel policy.

## In Scope

- `exclude_object` (`coBool`, canonical default `false`): host-resolved boolean; emit Klipper `EXCLUDE_OBJECT_DEFINE/START/END`, Marlin/Marlin2/RepRapFirmware `M486 S<n>/S-1`, and no exclusion syntax for Repetier.
- Object identity encoding: firmware token `pnp_` plus lowercase hex of raw UTF-8 object ID (collision-free and line-safe); M486 ordinal assigned by sorted unique raw IDs; start/end emitted around every contiguous object run.
- `gcode_label_objects` (`coBool`, canonical default `true`): independent human-readable `; OBJECT_START id=<comment-safe-id>` / `; OBJECT_END ...` around object runs; disabling labels never disables firmware exclusion markers.
- `gcode_comments` (`coBool`, canonical default `false`): add an opt-in `; filament: tool=<n> role=<canonical-role>` diagnostic before each extrusion entity's first extruding move; retain structural viewer comments regardless of this verbose gate.
- `gcode_flavor` (`coEnum`, canonical values/order `marlin`, `klipper`, `reprapfirmware`, `repetier`, `marlin2`; default `marlin`): declare exact accepted strings, reject other spellings through manifest bounds, and prove the real runtime changes exclusion and pressure-advance syntax at non-default values.
- `reduce_infill_retraction` (`coBool`, canonical default `false`): declare/read in `path-optimization-default`; evaluate consecutive `OrderedEntityView` endpoints against the matching region's `sparse_infill_area`; `false` emits retract/Z-hop, while `true` suppresses them only for a non-perimeter, same-region segment fully contained in sparse infill. Different regions, perimeter destinations, empty sparse areas, and escaping segments remain retracted.
- Typed host fields for the three emitter booleans, owner-manifest schemas, host-key docs, generated config docs, scheduler rejection coverage, unit/integration tests, and 04/05 owner/disposition annotations.
- Default-path compatibility tests: no exclusion commands, no verbose diagnostics, labels remain enabled, canonical `marlin` remains selected, and internal same-region travel now follows canonical default `reduce_infill_retraction = false` rather than the tree's current unconditional suppression.

## Out of Scope

- `filename_format`: returned to the queue as unimplemented and owner-corrected to host-export/CLI. Missing feature: derive an output path from a placeholder template when no explicit output filename is supplied. Current `Cmd::Slice` (`crates/pnp-cli/src/main.rs`) writes exactly `--output <PATH>` or stdout; `DefaultGCodeEmitter` must not touch paths. Fold into a future expansion of P84/ticket 91 or another host-export packet after that owner is claimed.
- `support_object_skip_flush`: not folded. Canonical default is `false`, and both reads require Bambu `M624 <encoded-label-ids>` immediately before filament-end/toolchange flush handling. This packet deliberately implements only flavors represented by `GcodeFlavor`; it adds neither a Bambu variant nor an M624/flush carrier. Sequence after those carriers exist.
- Bambu `M624`/`M625`, calibration-mode exclusion behavior, printable object polygons/centers in Klipper definitions, OctoPrint-specific copy-number naming, and per-instance names absent from `LayerCollectionIR`.
- Any edit or assertion involving `ORCA_CONFIG_PADDING`, CONFIG_BLOCK padding twins, or `crates/slicer-gcode/src/serialize.rs`.
- New WIT/IR fields, schema-version bumps, claim holders, modules, `gcode_add_line_number`, and `post_process`.

## Key Dispositions

| Key | Disposition | Behavior-changing decision / reason | Acceptance proof |
| --- | --- | --- | --- |
| `exclude_object` | retained, live | flavor-specific object-run firmware markers | AC-1, AC-2 |
| `gcode_comments` | retained, live | opt-in per-entity extrusion diagnostic | AC-4 |
| `gcode_flavor` | retained, live | strict dialect selection changes marker and pressure syntax | AC-5, AC-N1 |
| `gcode_label_objects` | retained, live | independent human object-run labels; non-default false removes them | AC-3 |
| `reduce_infill_retraction` | retained, live with owner correction | qualifying contained internal travel suppresses retract/Z-hop only when true | AC-6 |
| `filename_format` | returned to queue, unimplemented | needs host-export automatic filename/path derivation; emitter cannot own paths | AC-N2 |
| `support_object_skip_flush` | sequenced, unimplemented | needs Bambu M624 label-code plus filament-flush toolchange carrier | AC-7 |

There are zero declaration-only retained keys. CONFIG_BLOCK padding is neither a disposition nor evidence.

## Authoritative Docs

- `docs/01_system_architecture.md` - targeted `PostPass::GCodeEmit`, flavor-resolution, and CLI-output-lifecycle sections.
- `docs/03_wit_and_manifest.md` - delegated targeted summary for `[config.schema]` and `ConfigBoundsIndex` behavior.
- `docs/08_coordinate_system.md` - targeted mm/internal-unit checklist for the contained-travel predicate.
- `docs/21_data_defaults_and_fixtures.md` - targeted struct-literal rule.
- `docs/specs/orca-feature-gap/map.md` Notes, Authoring rules 1-6 and canonical value-spelling note - targeted reads.
- `docs/specs/orca-feature-gap/issues/48-author-packet-p41-multimaterial-multimaterial-advanced-emitter.md` - re-entry condition for `support_object_skip_flush`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef` declarations: bool defaults (`exclude_object=false`, `gcode_comments=false`, `gcode_label_objects=true`, `reduce_infill_retraction=false`, `support_object_skip_flush=false`), `filename_format` string default, and exact `gcode_flavor` values/default.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — object labels/markers in `GCode::process_layer`, verbose diagnostics in `GCode::_extrude`, flavor temperature branch, and internal-region test in `GCode::needs_retraction`.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::apply_print_config` dialect selection and verbose command-comment gate.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` and `OrcaSlicerDocumented/src/libslic3r/PrintBase.cpp` — output filename placeholder expansion and `PlaceholderParserError` boundary.

## Acceptance Summary

- Positive: `AC-1` through `AC-7` in `packet.spec.md` prove every retained key changes behavior at a non-default value and record both returned keys without declarations.
- Negative: `AC-N1` rejects invalid exact spellings/types; `AC-N2` protects the filesystem/emitter boundary; `AC-N3` protects CONFIG_BLOCK padding.
- Cross-packet impact: generic object runs create the future insertion seam for Bambu label codes, but do not claim Bambu parity; 04/05 annotations must preserve `filename_format` and `support_object_skip_flush` as queued/unimplemented.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test gcode_output_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` | Object runs, labels, verbose diagnostics, flavor matrix | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p path-optimization-default --test travel_policy_tdd reduce_infill_retraction 2>&1 | tee target/test-output.log | grep -E '^test result'` | Canonical default and non-default internal-travel policy | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_output_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` | Real config-to-emitter flavor/key delivery | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration gcode_output_modes_bounds_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` | Exact enum/bool rejection | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test machine_gcode_emit_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` | Owner-manifest schema contract | FACT pass/fail |
| `cargo xtask build-guests --check` | Manifest/module artifact freshness; rebuild without `--check` if stale | FACT exit code |
| `cargo xtask gen-config-docs --check` | Generated config reference | FACT exit code |
| `cargo check --workspace --all-targets` | All targets compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal gate | FACT exit code |

## Step Completion Expectations

- Add schema declarations before behavior so invalid values cannot reach permissive `GcodeFlavor::from_config_str` fallback through the normal resolved path.
- Object labels and firmware exclusion markers share one run-boundary iterator but remain independently gated.
- `reduce_infill_retraction` changes the producer of `TravelRetract`/`ZHop`; the emitter remains a serializer of those decisions.
- Capture `target/test-output.log` findings before each later cargo command overwrites it.

## Context Discipline Notes

- `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-gcode/src/emit.rs`, and runtime integration registries are long; locate symbols first and use ranged reads.
- Canonical reads are delegated only. The authoring session's direct bounded oracle read was forced by nested-subagent depth exhaustion; implementation must follow the snippet contract.
