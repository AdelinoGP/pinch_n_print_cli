# Requirements: layer-range-scope

## Packet Metadata

- Grouped task IDs: `TASK-570`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

PnP currently has no source or typed representation for OrcaSlicer's per-object layer configuration ranges, so authored ranges cannot influence either layer-plan construction or per-layer region configuration. This packet closes the coherent vertical slice from canonical 3MF XML through registry-aware typed ingestion to packet-05's two resolver queries and the two runtime setup paths, while preserving packet-07 denial enforcement.

### Approved plan-amendment note (attribution correction, not a divergence)

The source plan's Resolution paragraph says “later-starting range winning” for overlapping `layer_height` ranges. That parenthetical is inaccurate and is not implemented. Delegated canonical inspection of `layer_height_profile_from_ranges` (`Slicing.cpp`) established that its `std::map` iteration is ascending by range start and its trim step computes `lo = max(lo, last_pushed_high)`, so an earlier-starting range retains the overlap and the later range is clipped or removed; the fixed first-layer range is pushed first and likewise retains its interval. Uncovered gaps use the base layer height. This packet therefore implements **earlier-starting wins by trimming later ranges**, records the correction here without editing the approved plan or predecessor packets, and requires no deviation because it restores canonical parity. Conflicting overlaps for non-`layer_height` keys remain load errors under the approved owner policy.

## In Scope

- Parse optional ZIP part `Metadata/layer_config_ranges.xml` with exact canonical shape: root `objects`; child `object@id`; child `range@min_z,@max_z`; child `option@opt_key` with serialized value as element text.
- Treat `object@id` as the 1-based object-list ordinal used by OrcaSlicer, not as the 3MF model object's XML id; map it deterministically to the corresponding loaded MeshIR object and reject invalid/unmapped ordinals.
- Add `RawLayerConfigRange` and `LayerRangeLoadError` in `slicer-model-io`; absence is an empty successful result, while malformed XML, missing/non-finite/invalid bounds, duplicate object sections, and unmapped ordinals fail atomically.
- Add `ConfigScope::LayerRange { object_id, range_index }` and a host-side `LayerConfigRange` interval carrier to packet-03 ingestion; inventory every exhaustive `ConfigScope` match before adding the enum variant.
- Type range option text through `ConfigIngestor` and the assembled registry; no XML adapter performs config-type guessing.
- Validate all ranges before exposing `ScopedConfig`: selector keys and packet-07-denied keys return `ResolutionError::ScopeDenied`; overlapping different values for the same non-`layer_height` key return `LayerRangeLoadError::ConflictingOverlap`; equal overlapping values are permitted.
- Interpret finite endpoints in authored world-space Z millimetres and membership as half-open `[min_z, max_z)` against a layer's top print Z. Catch-up layers inherit the range covering that top Z.
- Compose `layer_height` ranges for each object in ascending start order: fixed first-layer interval first, preserve the earlier-starting interval, trim a later interval's low edge to the last retained high edge, skip it if emptied, and fill uncovered gaps with the object's resolved base `layer_height`.
- Fill packet-05's reserved precedence slot in both `query_z_grid` and `resolve_scope_stack`: `global < object < layer range < modifier < paint semantic < tool`.
- Carry the same typed range set through ordinary `run_slice_with_collector` setup and visual-debug's `prepare_prepass_context` setup. Parse the ZIP part once at the model-source adapter; do not parse in either resolver.
- Add `resources/layer_range_one_range.3mf` with exactly one object range `[0.4, 0.8)` and one `layer_height = 0.1` option, a focused model-IO parser test, focused slicer-config tests, a runtime convergence test, and a real visual-debug silhouette/manifest test.
- Update `docs/02_ir_schemas.md` and `docs/04_host_scheduler.md` with the final scope and scheduler contracts.

## Out of Scope

- Editing `docs/specs/config-scope-resolution-plan.md`, packet directories 01–08, ADR decision text, or `docs/DEVIATION_LOG.md`.
- Queue row 8 modifier-kind migration, queue row 10 automatic values, aliases, config emission, or changes to other scope precedence.
- A second range resolver, XML parsing inside runtime/resolver code, heuristic typing in model IO, or silent acceptance of denied/invalid ranges.
- WIT, guest/module source, module manifests, public IR serialization fields, or schema/version bumps.
- Matching ranges against object-local Z, layer bottom Z, nominal layer index, or closed upper endpoints.
- Loading or rewriting existing large/binary fixtures to discover expectations; the new fixture is purpose-built and its member is asserted directly.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — direct bounded reads of RC-7, Resolution, Layer range geometry semantics, cross-cutting requirements, and queue row 9; the correction above supersedes only its inaccurate overlap parenthetical.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — delegated summary of typed/decode-once scope policy.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — delegated summary of loud scope denial.
- `docs/02_ir_schemas.md` — bounded read of Config Key Namespaces and precedence.
- `docs/04_host_scheduler.md` — bounded reads of layer planning and region mapping ownership.
- `docs/08_coordinate_system.md` — bounded read of Z/mm conversion policy.
- `docs/19_visual_debug.md` — bounded read of request shape, silhouette, and manifest inspection.
- `docs/22_test_quality.md` — bounded read of independent-oracle, vacuity, and fixture rules.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `_BBS_3MF_Exporter::_add_layer_config_ranges_file_to_archive` and `_BBS_3MF_Importer::_extract_layer_config_ranges_from_archive`; confirm XML shape, one-based object ordinal linkage, and load failures.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `layer_height_profile_from_ranges`; confirm ascending range iteration, earlier-starting overlap retention by trimming later lows, fixed first-layer precedence, and canonical gap fill.

## Acceptance Summary

- Positive: `AC-1`–`AC-7` cover exact XML/fixture ingestion, typed world-Z half-open scope, corrected `layer_height` composition, catch-up/precedence behavior, both runtime setup paths, the required visual-debug gate, and authoritative docs.
- Negative: `AC-N1`–`AC-N4` cover conflicting non-height overlaps, selector/machine denial, malformed/invalid input, unmapped ordinals, and missing-part behavior.
- Cross-packet impact: packet 03 gains its intentionally deferred scope variant/carrier; packet 05's reserved slot becomes executable in both public queries; packet 07's admission set and `ScopeDenied` become load-path enforcement. All are reconciled forward dependencies until landed.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test layer_config_ranges_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | XML shape, ordinal linkage, missing/malformed/invalid behavior | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test layer_range_scope_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Scope typing, earlier-wins trimming/gaps, half-open/catch-up semantics, conflicts and denial | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration layer_range_scope_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Both production setup paths share range resolution | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p pnp_cli --all-targets --test layer_range_scope_visual_debug_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Real visual-debug bundle/manifest and nonuniform schedule | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo check --workspace --all-targets` | Compile every target after forward-interface reconciliation | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal fixture discipline | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Touched-test false-green review | FACT plus touched-file findings only |
| `cargo xtask build-guests --check` | Prove existing guest artifacts remain fresh before diagnosing integration failures | FACT with exit code |

## Step Completion Expectations

- Reconcile packet 03/05/07 exports before authoring tests or code; adapt names locally but do not change the approved semantics.
- Parse and validate before resolver wiring. Invalid input must not leave a partial `ScopedConfig`, layer profile, or resolved config.
- Keep the test oracle independent: XML fixture text, expected normalized profile segments, expected precedence values, and expected top-Z membership are authored literals, never outputs re-fed from production helpers.
- The model-source adapter owns one parse; both runtime paths receive the same typed representation and both resolver queries consume it.

## Context Discipline Notes

- `crates/slicer-model-io/src/loader.rs`, `crates/slicer-runtime/src/run.rs`, `crates/pnp-cli/src/visual_debug.rs`, and architecture docs are long; use symbol-bounded reads only.
- Never load the binary 3MF fixture directly. Inspect only its ZIP member list and the bounded XML member through a delegated FACT/SNIPPETS request.
- Canonical Orca reads and every cargo command are delegated with the bounded return formats above.
