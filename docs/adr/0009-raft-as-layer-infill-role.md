# ADR-0009 — Raft Rendering Reuses the `Layer::Infill` Role/Claim Pattern

## Status

Proposed (lands with `docs/specs/support-modules-orca-port.md` and `docs/specs/raft-default-module.md`).

## Context

Implementing raft surfaced the question: where do raft pattern algorithms
(rectilinear, grid, lightning, honeycomb) live?

Three structural options were evaluated:

1. **Extract pattern algorithms to a shared Rust library** (e.g. `slicer_core::patterns::{rectilinear_fill, grid_fill, lightning_fill}`) that `rectilinear-infill`, `traditional-support`, and a hypothetical `raft-default` renderer module would all call.
2. **`raft-default` ships its own pattern code**, accepting per-module duplication. The existing duplication between `rectilinear-infill::fill_expolygon_multi` and `traditional-support::fill_expolygon` would grow to a third copy.
3. **Reuse the existing `Layer::Infill` role/claim dispatch.** Add `ExtrusionRole::RaftInfill` + `claim:raft-fill` alongside the existing `TopSolidInfill`/`BottomSolidInfill`/`SparseInfill`/`BridgeInfill` roles. Whichever `Layer::Infill` module declares `claim:raft-fill` renders raft. `raft-default` becomes a synthesizer that produces raft polygons, NOT a renderer.

Option (1) was the first instinct but fails the project's multi-language
module promise: a community-authored C++ TPMS-Infill component cannot link
against a Rust library. Encouraging modules to depend on Rust-specific
helpers undermines the WASM-Component architecture's reason for existing.

Option (2) is honest but invites permanent duplication across three+ modules
of the same scan-line math.

Option (3) was uncovered during codebase exploration of `crates/slicer-sdk/src/views.rs:347-359`,
which already implements per-role per-claim dispatch:

```rust
pub fn should_emit(&self, role: ExtrusionRole) -> bool {
    let claim = match role {
        ExtrusionRole::TopSolidInfill => "claim:top-fill",
        ExtrusionRole::BottomSolidInfill => "claim:bottom-fill",
        ExtrusionRole::BridgeInfill => "claim:bridge-fill",
        ExtrusionRole::SparseInfill => "claim:sparse-fill",
        _ => return true,
    };
    self.held_claims.iter().any(|c| c == claim)
}
```

`SliceRegionView` already carries per-role fill-area inputs (`top_solid_fill`,
`bottom_solid_fill`, `infill_areas`, `bridge_areas`); multiple infill modules
can coexist at `Layer::Infill`, each holding different claims. Extending the
pattern with `RaftInfill` + `claim:raft-fill` slots into existing
infrastructure with no architectural novelty.

## Decision

Raft rendering uses the existing `Layer::Infill` role/claim pattern:

1. **`ExtrusionRole::RaftInfill`** is added as a new variant in `crates/slicer-ir/src/slice_ir.rs`'s `ExtrusionRole` enum.
2. **`claim:raft-fill`** is added to the `should_emit` mapping in `crates/slicer-sdk/src/views.rs`.
3. **`SliceRegionView` (or a sibling carrier per `raft-default-module.md` Carrier choice)** carries `raft_fill: Vec<ExPolygon>` polygon inputs on the layers where raft applies.
4. **`raft-default`** is a synthesizer module — it reads `SupportPlanIR.raft_plan` (emitted by `support-planner` per `docs/specs/support-modules-orca-port.md` §C6) and populates the raft polygon carriers. It contains zero pattern algorithms.
5. **Pattern variety** is provided by whichever `Layer::Infill` module(s) declare `claim:raft-fill` in their manifest. v1 ships with `rectilinear-infill` declaring the claim (matches OrcaSlicer's default `raft_pattern = "rectilinear"`). Users who want grid / honeycomb / lightning raft swap the claim to a different infill module.
6. **Each existing infill module gains a small dispatch addition** (10-15 lines) mirroring the existing `TopSolidInfill` / `BottomSolidInfill` handling. The module's existing fill function is called with the raft polygon and the role tag changes; no pattern math is duplicated.

The shared-library option (1) is rejected. The per-module-duplication option
(2) is rejected for new code; existing duplication between `rectilinear-infill`
and `traditional-support` is acknowledged but not addressed (tracked as
TASK-270, dependent on a future WIT-interface pattern-services design).

## Consequences

**Positive**:
- Pattern variety for raft is automatic — any `Layer::Infill` module that declares `claim:raft-fill` provides its pattern for raft.
- Multi-language module promise preserved. A C++ TPMS-Infill module can declare `claim:raft-fill` alongside `claim:sparse-fill` and render TPMS raft without depending on any Rust library.
- No new module-dispatch infrastructure. The role/claim pattern is already load-bearing for model infill; raft slots in.
- `raft-default` is small (synthesizer-only, zero pattern code). Easy to review, easy to alternate-implement.
- `support-planner` keeps sole ownership of `SupportPlanIR` (single-writer rule preserved); the raft carrier is a different IR.

**Negative**:
- Adds one variant to `ExtrusionRole` and one claim string. Schema bump on `ExtrusionRole` (semver minor — additive).
- Existing infill modules need updating (small dispatch case each). Modules that don't update silently produce zero raft paths, even if they're the only infill module loaded — discoverability cost; mitigated by a `LogLevel::Warn` diagnostic from `raft-default` when raft regions exist but no module declares `claim:raft-fill`.
- A user who wants different patterns for sparse vs. raft (e.g., lightning sparse + rectilinear raft) must load two infill modules with disjoint claims. Possible today but requires the user to understand the claim model — documentation cost.

**Trade-offs we explicitly accept**:
- Existing duplication between `rectilinear-infill::fill_expolygon_multi` and `traditional-support::fill_expolygon` is NOT addressed by this decision. Its proper fix (WIT-interface pattern services — modules invoking each other's algorithms across language boundaries) is a separate architectural conversation that has not happened yet. Documented in `docs/specs/support-modules-orca-port.md` §Open Follow-ups as TASK-270.
- The "default" raft pattern in v1 is rectilinear (because `rectilinear-infill` declares the claim by default). Users wanting other patterns must explicitly configure which infill module holds `claim:raft-fill`.

## Future-Reviewer Notes

- **Do not re-suggest extracting patterns to `slicer_core::patterns`.** This was the first instinct during the design exploration and was rejected for the multi-language module promise. If the project's stance on language portability changes, revisit; otherwise the extraction is the wrong direction.
- **Do not re-suggest making `raft-default` a renderer.** The synthesizer-only shape is load-bearing for the no-duplication goal.
- **Do not re-suggest a separate `Layer::Raft` stage with its own renderer claim.** This was considered and rejected — adding a per-fill-type stage for every fill type would proliferate stages without solving the duplication problem.

## References

- `docs/specs/support-modules-orca-port.md` §C6, §D6, §D7.
- `docs/specs/raft-default-module.md`.
- `crates/slicer-sdk/src/views.rs:330-359` — existing role/claim dispatch.
- `crates/slicer-ir/src/slice_ir.rs:1463-1492` — `ExtrusionRole` enum.
- OrcaSlicer `src/libslic3r/Support/SupportCommon.cpp::generate_raft_base` — reference behavior.
