# Requirements: top-fill-order-and-calibration-order

## Packet Metadata

- Grouped task IDs: none — this packet is driven by the wayfinder map, not by a `docs/07` backlog slice. Its backlog source is `docs/specs/orca-feature-gap/issues/33-author-packet-p26-calibration-flow-pressure-advance-calibration-infill-modules.md`. Packet 264, the sibling packet from the same map, likewise carries `task_ids: []`.
- Backlog source: `docs/specs/orca-feature-gap/issues/33-author-packet-p26-calibration-flow-pressure-advance-calibration-infill-modules.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Wayfinder ticket 33 was filed as "P26 — 1 key, Tier B, infill modules" for `calib_flowrate_topinfill_special_order`. That sizing has rotted. The key does two things in canonical, neither of which this port can express today:

1. `Fill::fill_surface_extruded` (`Fill/FillBase.cpp`) sets `no_sort = true` on the top-solid-infill entity collection and calls `set_reverse()` on every entity in it. `ExtrusionEntity.hpp` shows `set_reverse()` sets `m_can_reverse = false` — it **forbids** reversal, it does not perform one. Together: the top fill becomes an atomic, non-reorderable, non-reversible block.
2. `FillPlanePath::fill_surface` reorders the clipped fragments so the longest one — the center spiral — is emitted last and runs inside-out, with the remaining chords chained ahead of it. Canonical's own comment states why: the chords and the spiral collide in opposing directions, raising the tactile lip the flow-rate calibration is read from.

The second behaviour is gated on `top_surface_fill_order == SurfaceFillOrder::Default`, and `top_surface_fill_order` / `bottom_surface_fill_order` are **absent from `docs/ORCA_CONFIG_REFERENCE.md`** and therefore from the map's queue entirely — a queue-completeness miss of the kind wayfinder ticket 123 measures. They are live keys: `Fill.cpp` reads them whenever the pattern is `ipConcentric`, `ipArchimedeanChords`, or `ipOctagramSpiral`, and `PrintObject.cpp` lists them in its invalidation set.

The port's side:

- The atomic-block seam **exists**: `slicer_ir::ExtrusionPath3D::order_lock` (IR schema `1.4.0`, packet 244), enforced by `validate_entity_order_locks` (`crates/slicer-runtime/src/layer_executor.rs`) and honoured by `coalesce_locked_candidates` in `modules/core-modules/path-optimization-default`, which collapses a maximal run of equal tags into one non-reversible nearest-neighbour candidate. It is a near-exact match for `no_sort` + `can_reverse = false`.
- But it is **unreachable from top fill**: `remap_infill_order_locks_from`, `next_global_infill_tag`, and `validate_infill_order_locks` all walk `InfillRegion::sparse_infill` only. Top fill lands in `InfillRegion::solid_infill`, so a module-local tag is never promoted to a layer-global tag and the block is never validated. Meanwhile `assemble_ordered_entities_with_support_identities` already walks all four vectors, so the entity view and the remap disagree. This is a packet-244 implementation gap, not a designed boundary: ADR-0062 — the ADR packet 244 landed under — states the host remaps local tags to global tags "at every output boundary" and enforces the invariant "at every mutation point". Widening the walk is conformance to that ADR, not an amendment of it.
- No module sets a lock today: every `order_lock` occurrence under `modules/**` is `None`, and `OrderLockAllocator` has zero callers outside the SDK.

The pattern the key modifies is shipped by **packet 264**, which creates `archimedean-chords-infill`, `concentric-infill`, and `octagram-spiral-infill` as `claim:top-fill` holders. Per the user's ruling of 2026-09-03, that module work stays with 264 and everything above is this packet's.

## In Scope

- A new `crates/slicer-sdk/src/surface_fill_order.rs` module exporting:
  - `SurfaceFillOrder { Default, Outward, Inward }` with a `&str` parser accepting exactly `"default"` / `"outward"` / `"inward"` and rejecting everything else.
  - `order_center_based_fragments(source, fragments, order, calibration_special_order)` — the ordering kernel ported from canonical `FillPlanePath::fill_surface`, covering the Default, calibration, Outward, and Inward cases.
  - `lock_block(paths, allocator)` — stamps one shared invocation-local `order_lock` tag across a contiguous slice of `ExtrusionPath3D`, sitting alongside the existing `slicer_sdk::order_lock::OrderLockAllocator` / `remap_order_locks_to_global`.
- Extending `remap_infill_order_locks_from`, `next_global_infill_tag`, and `validate_infill_order_locks` (`crates/slicer-runtime/src/layer_executor.rs`) from `sparse_infill` to all four `InfillRegion` extrusion vectors: `sparse_infill`, `solid_infill`, `ironing`, `internal_bridge_infill`. Global tag uniqueness must hold **across** the four vectors, not per vector.
- `[config.schema]` declarations and live call sites in the three center-based modules packet 264 creates:
  - `archimedean-chords-infill` — `top_surface_fill_order`, `bottom_surface_fill_order`, `calib_flowrate_topinfill_special_order`.
  - `concentric-infill`, `octagram-spiral-infill` — `top_surface_fill_order`, `bottom_surface_fill_order` only.
- An annulus test fixture (square with a square hole) in the module test suite, because a convex region clips the spiral to a single fragment and cannot exercise any ordering at all.
- One `docs/DEVIATION_LOG.md` row for the recorded divergence below, and the `docs/02_ir_schemas.md` scope correction.

## Out of Scope

- **Creating any fill module.** `archimedean-chords-infill`, `concentric-infill`, and `octagram-spiral-infill` are packet 264 deliverables and a forward dependency here. This packet appends to their manifests and calls the SDK helper from them; it does not author their fill geometry, their claim declarations, or their registry entries.
- **`top_surface_pattern` / `bottom_surface_pattern` → holder derivation.** Packet 264 owns it.
- **3MF ingest work.** Verified this session and explicitly excluded: `parse_project_settings_json` (`crates/slicer-model-io/src/loader.rs`) ingests every key from an Orca 3MF's `project_settings.config` generically through `json_to_config_value`, with no allowlist. The narrow allowlist in `object_metadata_to_config_data` governs only the per-object `Slic3r_PE_model.config` path and does produce booleans (`enable_support` via `coerce_string_to_config_value`). Neither is a blocker; do not add ingest work.
- **Any IR field or schema-version bump.** `ExtrusionPath3D::order_lock` already exists; this packet changes only which vectors the host walks.
- **Widening the lock to non-center-based top fills** — see Recorded Divergence.
- **Filing `top_surface_fill_order` / `bottom_surface_fill_order` into `docs/ORCA_CONFIG_REFERENCE.md`.** That reference's row-set completeness is wayfinder ticket 123's subject; this packet implements the keys and leaves the reference to that ticket.

## Recorded Divergence

Canonical's `Fill::fill_surface_extruded` applies the `no_sort` + `set_reverse()` lock to **any** top-solid-infill entity collection when the flag is set, regardless of pattern. This port locks only the center-based fills (`archimedean-chords-infill`, `concentric-infill`, `octagram-spiral-infill`).

Rationale, per map Authoring rule 4 (a deliberate improvement is a recorded divergence, not a gap): for a scan-line or gyroid top fill there is no generated ordering to preserve — the fragments carry no meaning in sequence — so locking them would only forbid `path-optimization-default`'s nearest-neighbour travel optimisation with no parity-visible effect on the print. Canonical pays that cost because its flag is read in the pattern-agnostic `FillBase` layer rather than in the pattern that needs it. AC-N4 pins the narrower scope so a future packet that widens it must update the schema guard. File the row with a `DEV-###` re-derived from `docs/DEVIATION_LOG.md` at implementation time.

## Authoritative Docs

- `docs/02_ir_schemas.md` — over 300 lines; delegate a SUMMARY of the `ExtrusionPath3D.order_lock` / schema `1.4.0` paragraph and the ADR-0063 note. Direct read only of the paragraph being edited.
- `docs/03_wit_and_manifest.md` — over 300 lines; delegate a SUMMARY of § `[config.schema]` for a string-enum key and a bool key.
- `docs/21_data_defaults_and_fixtures.md` — ranged direct read of the struct-literal churn gate only.
- `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` and `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` — both short; direct read. These are the normative `order_lock` contracts and bind Steps 2, 3, and 5.
- `docs/specs/orca-feature-gap/map.md` — over 300 lines; delegate a SUMMARY of the Notes bullets "Authoring rules 1–6" and "The canonical oracle is …". Do not read the whole map.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillPlanePath.cpp` — `FillPlanePath::fill_surface`'s `is_flow_calib` branch and its non-Default `restore_source_path_order` branch; the two orderings AC-2 and AC-3 port.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Fill::fill_surface_extruded`'s `no_sort` + `set_reverse()` handling; the semantics this port expresses as `order_lock`.
- `OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.hpp` — confirms `set_reverse()` sets `m_can_reverse = false` (forbids reversal).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — where the two fill-order keys are read and the pattern gate that decides which modules declare them.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the three keys' types, enum values, defaults, and modes.
- `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp` — `restore_source_path_order`.

**The canonical oracle for this packet is the checkout named in the wayfinder map's Notes** (`docs/specs/orca-feature-gap/map.md`, "The canonical oracle is …"). Re-derive that path from the map at point of use. Do **not** use `Orca(pnp_gui)`: it is a GUI-only fork with the FFF slicing pipeline removed, and every file cited above is absent from it.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-14`. Refinements not in their Given/When/Then text: AC-2's "longest fragment" is by accumulated polyline length, matching canonical's `Polyline::length`, not by point count or bounding-box diagonal. AC-5's uniqueness is asserted as a set-cardinality equality over the flattened tags of all four vectors of all regions.
- Negative: `AC-N1` through `AC-N4`.
- Cross-packet impact: packet 264 must be `implemented` first — this packet edits three manifests and three module sources that 264 creates. Packet 244 introduced the order-lock seam this packet widens; 244 is not reopened or superseded, because ADR-0062 already mandates remapping at every output boundary and 244's sparse-only implementation is narrower than the ADR it landed under. Still confirm with a bounded FACT dispatch on `docs/spec_packets/244-order-locked-extrusion-sequences/packet.spec.md` that no 244 AC positively asserts the other three vectors are excluded, and stop and ask if one does. ADR-0063 additionally makes locked paths self-clipping and gives the linker a swept-footprint carve over untagged fill of the same region — AC-14 pins both obligations; see `design.md` §Architecture Constraints.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-sdk --test surface_fill_order_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1 to AC-4: the ordering kernel and block locking | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --test executor order_lock 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-5, AC-6, AC-N3: host remap and validation across all four vectors | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p archimedean-chords-infill --test calibration_order_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-8, AC-9: the behaviour change at non-default values | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p archimedean-chords-infill --test surface_fill_order_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-7, AC-N4: manifest declarations and the pinned narrow scope | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration per_object_calibration_order_flag 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-11: per-object delivery of the calibration flag | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N2: enum rejection | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract solid_infill_order_lock_survives_path_optimization 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-10 and AC-14: lock survival through path optimization, plus the ADR-0063 self-clipping and no-op-carve obligations | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N1: default-path neutrality | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | AC-12: generated key table | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | Three edited guests are fresh; exit 0 required | FACT exit code (0 fresh, 1 stale, 3 infra) |
| `cargo check --workspace --all-targets` | Compile gate across all targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate (new `ExtrusionPath3D` test literals) | FACT exit code |
| `cargo xtask check-deviations` | The recorded-divergence row parses | FACT exit code |

## Step Completion Expectations

- Steps 1–3 (SDK helper, host remap, host validation) are independent of packet 264 and may land before it. Step 4 onwards must not start until 264 reads `status: implemented` in its own frontmatter — re-derive, never trust this sentence.
- The SDK helper must land **before** the host remap widening is exercised end to end, but the host widening is independently testable with hand-built `InfillIR` fixtures and does not import the helper.
- Tag-uniqueness is a cross-step invariant: after Step 2, `next_global_infill_tag` and `remap_infill_order_locks_from` must agree on the same four-vector scan. Widening one without the other silently reissues a live tag; Step 2's exit condition asserts both.

## Context Discipline Notes

- `crates/slicer-runtime/src/layer_executor.rs` is very large. Never open it whole. The three functions this packet edits are adjacent (`remap_infill_order_locks_from`, `next_global_infill_tag`, `slicer_runtime_order_lock_remap`, `validate_infill_order_locks`); locate by symbol name and open a ±40-line window.
- `docs/specs/orca-feature-gap/map.md` is long and mostly irrelevant here — delegate the two Notes bullets named above; do not read it for context.
- Packet 264's `design.md` and `implementation-plan.md` are out of bounds. To learn the three modules' emitted-path shape and their `from_config` signature, dispatch a bounded SUMMARY against 264's `packet.spec.md` only, or read the modules themselves once 264 has landed.
