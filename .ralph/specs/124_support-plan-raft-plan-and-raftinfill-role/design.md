# Design: support-plan-raft-plan-and-raftinfill-role

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-ir/src/slice_ir.rs::SupportPlanIR` (line 1138) — additive `raft_plan: Vec<RaftPlan>` field; new `RaftPlan` + `RaftLayerSpec` structs; `ExtrusionRole::RaftInfill` variant; schema_version minor bump.
  - `crates/slicer-sdk/src/views.rs::should_emit` (line 497) — new role/claim arm `ExtrusionRole::RaftInfill => "claim:raft-fill"`.
  - `crates/slicer-schema/wit/deps/types.wit` (interface `geometry`, lines 12-17) — add `raft-infill` variant to the `extrusion-role` enum.
  - `modules/core-modules/support-planner/src/lib.rs:442-491` — replace the current degenerate raft block with `RaftPlan` emission. The new emission: for each object that has at least one branch contact, compute expanded footprint, populate `layers: Vec<RaftLayerSpec>`, populate `z_bed`/`gap_z`/`first_layer_density`.
  - `modules/core-modules/traditional-support/src/lib.rs` — extend the lead `//!` block with one explicit non-consumption sentence.
- Neighboring tests/fixtures:
  - `crates/slicer-ir/tests/support_plan_ir_schema_version_bumped.rs` — new file (AC-7).
  - `crates/slicer-sdk/tests/should_emit_raft_fill_claim.rs` — new file (AC-N2; or extension to an existing test file if one is conventional).
  - `crates/slicer-runtime/tests/integration/raft_plan_emission_tdd.rs` — new file (AC-4, AC-5, AC-N1).
  - `docs/02_ir_schemas.md` — extended.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The `raft_plan` addition is ADDITIVE. Existing consumers of `SupportPlanIR` continue to compile; the deserialized `raft_plan` is an empty `Vec` when reading from older serialized blobs (default via `#[serde(default)]` if serde is used — confirm).
- The `ExtrusionRole::RaftInfill` enum variant is additive but extends an exhaustive `match` surface in every `should_emit` consumer; the implementer's Step 3 dispatch lists all `match role` sites to update. Forgetting one results in either a compile failure (non-exhaustive match warning escalated to error) or a silent "always emit" path (the `_ => true` fallback). Note: the current `should_emit` at line 503 already has the `_ => true` fallback, so adding a new variant without updating the match means it falls into `_` and emits unconditionally — this is the silent-true failure mode the implementer must guard against by adding the arm explicitly.
- The `claim:raft-fill` string is a NEW claim. Existing `should_emit` consumers that don't hold it return `false` (per the existing `held_claims.iter().any` semantics) — this is correct per AC-N2.
- The schema_version bump is minor (additive). The implementer MUST NOT bump major.
- The WIT mirror (at `crates/slicer-schema/wit/deps/types.wit`) is NOT 1:1 with the Rust enum (WIT has 13 variants; Rust has 19 + `Custom(String)`). The packet maintains this asymmetry: WIT gets `raft-infill`; Rust gets `RaftInfill`. The two names are linked by the bindgen mapping; the implementer's Step 3 dispatch confirms the WIT→Rust enum mapping convention used for existing variants.

## Code Change Surface

- Selected approach: targeted IR + SDK + WIT + planner edits. No new module is introduced.
- Exact functions/structs/manifests/tests to change:
  - `slicer_ir::ExtrusionRole` (line 1639) — new variant.
  - `slicer_ir::SupportPlanIR` (line 1138) — new field + schema_version bump.
  - `slicer_ir::{RaftPlan, RaftLayerSpec}` — new structs.
  - `slicer_sdk::views::should_emit` (line 497) — new arm.
  - `slicer_schema::wit::deps::types::geometry::extrusion_role` (in `types.wit:12-17`) — new variant.
  - `support_planner::plan_for_object` (line 313) — degenerate raft block deletion + `RaftPlan` emission.
  - `traditional_support` lead `//!` block — one sentence added.
  - Three new test files.
  - `docs/02_ir_schemas.md` — extended.
- Rejected alternatives:
  - **Replace `SupportPlanIR.entries` with a `tagged enum { Branch(SupportPlanEntry), Raft(RaftPlanEntry) }`** — rejected: breaking change to every consumer. Additive sibling field is the ADR-0009 choice.
  - **Make `raft_plan` per-region (`HashMap<RegionId, RaftPlan>`)** — rejected: raft is per-object per ADR-0009 D5; per-region keying re-introduces the duplication problem the ADR resolved.
  - **Add raft renderer code in this packet** — rejected: explicit out of scope; ADR-0009 splits at the rendering boundary.
  - **Make `support_raft_layers > 0` configurable per-region** — rejected: raft is a per-object decision in Orca + ADR-0009.
  - **Sync the WIT mirror to a 1:1 with the Rust enum (add `prime-tower`, `skirt`, `brim`, `InternalSolidInfill`)** — rejected: out of scope; the asymmetry is pre-existing and is not this packet's concern.

## Files in Scope (read + edit)

The packet edits 5 source files + 3 new test files + 1 doc file (9 total).

- `crates/slicer-ir/src/slice_ir.rs` — role: IR additions; expected change: ≈30 lines added.
- `crates/slicer-sdk/src/views.rs` — role: claim arm; expected change: 1 line.
- `crates/slicer-schema/wit/deps/types.wit` — role: WIT mirror update; expected change: 1 line added to the `extrusion-role` variant.
- `modules/core-modules/support-planner/src/lib.rs` — role: emission rewrite; expected change: lines 442-491 replaced (≈50 lines net).
- `modules/core-modules/traditional-support/src/lib.rs` — role: doc sentence; expected change: 1 sentence added.
- `crates/slicer-ir/tests/support_plan_ir_schema_version_bumped.rs` — new.
- `crates/slicer-sdk/tests/should_emit_raft_fill_claim.rs` — new (or extend existing test for `should_emit`).
- `crates/slicer-runtime/tests/integration/raft_plan_emission_tdd.rs` — new.
- `docs/02_ir_schemas.md` — extended.
- `modules/core-modules/traditional-support/traditional-support.toml` — AC-9 verification only; no edit expected (manifest is already clean per `reads = ["SliceIR", "SurfaceClassificationIR", "PaintRegionIR"]` at line 14).

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C6, §C7, §D5, §D6 — directly.
- `docs/adr/0009-raft-as-layer-infill-role.md` — directly.
- `docs/specs/raft-default-module.md` — directly (the consumer of this packet's IR seam).
- `crates/slicer-ir/src/slice_ir.rs` — range-read existing `SupportPlanIR`, `ExtrusionRole`, `Point3WithWidth`.
- `crates/slicer-sdk/src/views.rs` — range-read `should_emit` and surrounding match arms.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- Other consumer crates beyond the listed surface — do not browse for "consistency."
- `target/`, `Cargo.lock`, generated code.
- Post-74710fa fill-partition test files — out of scope.
- `crates/slicer-schema/wit/deps/ir-types.wit` (the file the original spec mentioned) — does NOT contain the `ExtrusionRole` mirror; the mirror is in `types.wit`. The original spec's `ir-types.wit` reference is stale; the implementer uses `types.wit`.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicer `SupportCommon.cpp::generate_raft_base` for raft footprint computation, expansion factor, gap-layer Z; return SUMMARY ≤ 200 words. No code snippets." — purpose: confirm the planner data fields match the renderer's expected input.
- "Locate `should_emit` function in `crates/slicer-sdk/src/views.rs`; return SNIPPETS ≤ 30 lines showing the `match role` arms and surrounding context." — purpose: confirm Step 3 edit site.
- "Locate every `match role` site in the workspace that switches on `ExtrusionRole`; return LOCATIONS ≤ 20 entries." — purpose: confirm exhaustive update.
- "Locate current `SupportPlanIR.schema_version` value in `crates/slicer-ir/src/slice_ir.rs`; return FACT (the `SemVer` literal)." — purpose: Step 2 bump arithmetic.
- "Locate `crates/slicer-ir/src/slice_ir.rs` lines defining `SupportPlanEntry`, `SupportPlanIR`, `Point3WithWidth`, `ExtrusionRole`; return LOCATIONS file:line." — purpose: edit targets.
- "Confirm `ExtrusionRole` is mirror'd in `crates/slicer-schema/wit/deps/types.wit` (NOT `ir-types.wit`); return SNIPPETS ≤ 20 lines showing the WIT variant." — purpose: Step 3 WIT edit site.
- "Run `cargo build --workspace`; return FACT pass/fail; SNIPPETS ≤ 30 lines FIRST error." — purpose: post-IR-change compile gate.
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE." — purpose: WASM gate.
- "Run AC-1 through AC-11 + AC-N1 + AC-N2 + AC-N3 commands; return FACT PASS/FAIL list." — purpose: packet gate.

## Data and Contract Notes

- IR contracts touched: `SupportPlanIR` (additive); `ExtrusionRole` (new variant); schema_version minor bump.
- WIT boundary considerations: the new `extrusion-role` variant in `types.wit` MUST match the bindgen name. The `crates/slicer-schema/wit` package's bindgen produces the Rust enum; the WIT variant name becomes a Rust variant (likely `RaftInfill` from `raft-infill`, matching the convention used by `top-solid-infill` → `TopSolidInfill`).
- Determinism: raft plan emission is deterministic given the same inputs (footprint geometry, config).
- The `RaftPlan.footprint` is computed from `SupportGeometryView.outlines` (the same data the avoidance cache reads). Step 4 confirms via dispatch whether the data lives at a single canonical path or multiple.

## Locked Assumptions and Invariants

- `support-planner` is the sole writer of `SupportPlanIR` (single-writer-per-IR rule). This packet does NOT change that.
- `raft_plan` is per-object, keyed by `object_id`. Per-region duplication is forbidden.
- `RaftPlan.layers[*]` ordering is top-of-stack to bottom (highest `z` first). Per-layer Z values are populated using the formula from `docs/specs/support-modules-orca-port.md` §C6:
  ```
  z_bed = layer_plan.layers[0].z - layer_plan.layers[0].effective_layer_height
  raft_layer_i_z = z_bed - (raft_layers - i) * raft_layer_height_mm
  ```
- `RaftPlan` is only emitted for objects whose `entries` is non-empty (an object that gets no support branches gets no raft per ADR-0009 — adhesion-raft for objects without supports is future work).
- The WIT mirror asymmetry is preserved (Rust 19 + Custom(String), WIT 13 + custom(string)); this packet adds one variant to each side (Rust 20 + Custom(String), WIT 14 + custom(string)).

## Risks and Tradeoffs

- **Risk**: `should_emit` has a `_ => true` fallback at line 503. If the implementer adds the `ExtrusionRole::RaftInfill` enum variant but forgets to add the `should_emit` arm, modules fall into the `_` and emit `true` unconditionally. **Mitigation**: AC-3 grep is the structural gate; AC-N2's behavioral test (a module without `claim:raft-fill` returns `false`) is the runtime gate. Both are required.
- **Risk**: the `ExtrusionRole` WIT type addition triggers guest rebuild of all 20 guests (not just support modules). **Mitigation**: this is one-time cost; `cargo xtask build-guests` handles it.
- **Risk**: removing the degenerate raft block breaks any existing tests that asserted on the degenerate emission. **Mitigation**: Step 4 dispatches a search for those tests (`rg 'Point3WithWidth.*x: 0.0.*y: 0.0.*z: raft_z' crates/`); if any, they are migrated to the new emission shape or noted as expected breakage.
- **Risk**: schema_version bump is additive, but if the host's `MAX_IR_SCHEMA` check rejects a higher version, downstream consumers that read older `SupportPlanIR` blobs may fail. **Mitigation**: confirm via Step 1 dispatch that `MAX_IR_SCHEMA` in the support module's manifest is `5.0.0` (per `tree-support.toml:26`); the bump from `1.x.y` to `1.(x+1).0` is well under that cap.
- **Tradeoff**: the WIT enum addition requires guest rebuild ceremony. Acceptable: enum additions are the cheapest WIT changes.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4 — degenerate block removal + new emission).
- Highest-risk dispatch: "Locate all `match role` sites in the workspace" — return MUST be LOCATIONS ≤ 20 entries; never paste source.

## Open Questions

None. The TASK ID renumbering (source-plan `TASK-265`/`TASK-266` → `TASK-289`) is recorded in `requirements.md` §Packet Metadata and `task-map.md`.
