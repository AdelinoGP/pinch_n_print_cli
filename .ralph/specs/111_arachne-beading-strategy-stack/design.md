# Design: 111_arachne-beading-strategy-stack

## Controlling Code Paths

- **Trait + base struct.** `crates/slicer-core/src/beading/mod.rs` (NEW) declares `pub trait BeadingStrategy: Send + Sync` with the four AC-1 methods (`compute`, `optimal_bead_count`, `get_transition_thickness`, `optimal_thickness`) plus `type_label` for AC-8 composition verification, and the `Beading` struct (`total_thickness`, `bead_widths`, `toolpath_locations`, `left_over`, all `f64` in slicer units). Trait is object-safe — methods don't return `Self`, don't have generic parameters, no `Sized` bound.
- **Base strategy.** `beading/distributed.rs` carries `DistributedBeadingStrategy { optimal_width: f64, default_transition_length: f64, transition_filter_dist: f64, distribution_count: usize }`. `compute` runs the Gaussian decay against `bead_count`, producing the widths Vec.
- **Decorators.** `redistribute.rs`, `widening.rs`, `outer_wall_inset.rs`, `limited.rs` each carry a `parent: Box<dyn BeadingStrategy>` field plus their own params. Their `compute` delegates to the inner strategy, then transforms.
- **Strip-pass (T-215b).** `LimitedBeadingStrategy::compute_and_strip(thickness, bead_count) -> Beading` calls `self.compute(thickness, bead_count)` and then walks `bead_widths` removing zero entries (along with the matched `toolpath_locations`). The raw `compute` MUST retain sentinels (asserted by AC-N2). Downstream P112 wire-up calls `compute_and_strip` for production output. This implements the D-9 roadmap decision; the deviation rationale is logged as `D-111-ARACHNE-SENTINEL-STRIP` in `docs/DEVIATION_LOG.md` (D-9 is a roadmap-level ID, not a log entry).
- **Factory.** `beading/factory.rs` exposes `BeadingStrategyFactory::create_stack(params: &BeadingFactoryParams) -> Box<dyn BeadingStrategy>` returning `Box::new(LimitedBeadingStrategy::new(Box::new(OuterWallInsetBeadingStrategy::new(Box::new(WideningBeadingStrategy::new(Box::new(RedistributeBeadingStrategy::new(Box::new(DistributedBeadingStrategy::new(...))))))))))`. The composition order is verified by AC-8 via either `std::any::type_name_of_val` reflection OR by a layer-by-layer `downcast_ref` walk — pick whichever the trait surface allows; if neither works (object-safe trait can't be downcast without `Any`), introduce a `fn debug_layer_name(&self) -> &'static str` method on the trait that each impl returns its own type name.

## Neighboring Tests & Fixtures

- `crates/slicer-core/tests/` will carry the post-P110 pattern (`voronoi_stress.rs`, `skt_graph_golden.rs`, `preprocess_golden.rs`) once P110 ships — those files are FORWARD-DEPs on draft P110 and do NOT currently exist in the tree. The new beading test files (`beading/distributed.rs`, `beading/redistribute.rs`, etc.) follow the same per-file-per-test pattern. Fixtures live under `tests/fixtures/beading/`.
- **Test registration:** `slicer-core` uses explicit `[[test]]` entries in `crates/slicer-core/Cargo.toml`; each new `tests/beading/*.rs` file requires a corresponding `[[test]] name = "<name>"` entry. See Step 7b in the implementation plan — `Cargo.toml` is in the edit list.
- The 10-thickness reference table for `Distributed` is the heaviest fixture (10 expected `Beading` outputs in JSON). The implementer authors it during Step 2 by running the OrcaSlicer reference once off-tree (or by transcribing values from a published OrcaSlicer test); the goldens are committed and treated as authoritative — never regenerated during this packet.

## Architecture Constraints

<!-- snippet: coord-system -->
- **Coordinate system hazard.** All `Beading` widths and toolpath_locations are in slicer units (1 unit = 100 nm). OrcaSlicer config defaults are typically in real units (mm) or SCALED OrcaSlicer units (1 unit = 1 nm). The implementer MUST translate via `mm_to_units` or the explicit `/100` rule per `docs/08_coordinate_system.md`. Confirm each of the 11 config keys' translated default during the PrintConfig.cpp LOCATIONS dispatch (see OrcaSlicer Reference Obligations).

- **Object-safe trait.** `BeadingStrategy` MUST be object-safe so `Box<dyn BeadingStrategy>` works in the decorator chain. No generic methods; no `Self` returns; no associated types tied to `Self`. If reflection/downcast is needed (AC-8), add an explicit `fn type_label(&self) -> &'static str` trait method.
- **No floating-point HashMap keys.** Determinism is required: `Beading` outputs MUST be byte-identical for byte-identical inputs. No HashMap over `f64` keys; if keying is needed, use sorted `Vec<(f64, T)>` with stable ordering.
- **No panics outside debug-asserts.** Strategy `compute` returns a `Beading` — invariant violations (`toolpath_locations.len() != bead_widths.len()`) get a `debug_assert_eq!` in debug builds and silent acceptance in release (with the caller responsible for downstream validation). Documented in AC-N1.

## Selected Approach

**Idiomatic Rust decorator chain on object-safe trait.** Each strategy is a struct; decorators own `Box<dyn BeadingStrategy>` inner. `Box<dyn BeadingStrategy>` is the return type of `BeadingStrategyFactory::create_stack`. Method dispatch via vtable.

Rejected alternatives:
- **Generic decorator chain (`Limited<OuterWallInset<Widening<...>>>`)**: would compose at the type level, but produces a 5-level nested type that becomes opaque to call sites + breaks at module boundaries (the factory return type would need to be a 5-level generic instantiation). Rejected — Box<dyn> is idiomatic and matches OrcaSlicer's runtime-polymorphic shape.
- **Enum dispatch**: would avoid heap allocation but loses extensibility and requires every new strategy to touch every enum variant. Rejected.
- **Fold compute into a single function**: would lose the modular structure that mirrors OrcaSlicer 1:1. Rejected — the structural parity is intentional (debuggability + future maintainer comprehension).

For T-215b (strip-pass): TWO entry points (`compute` retains sentinels for invariant testing; `compute_and_strip` returns clean output). Rejected: folding strip into `compute` directly (loses AC-N2's invariant guard); calling the inner `compute` raw at the LimitedBeadingStrategy boundary and stripping in a wrapper (works but obscures the responsibility — keep both methods on the Limited strategy itself for locality).

## Code Change Surface

| File | Status | Step | Notes |
| --- | --- | --- | --- |
| `crates/slicer-core/src/lib.rs` | EDIT | Step 1 | `pub mod beading;` |
| `crates/slicer-core/Cargo.toml` | EDIT | Step 1 | Add `[[test]]` entries for all 6 new test files (register upfront to keep Steps 2–7 within edit cap) |
| `crates/slicer-core/src/beading/mod.rs` | NEW | Step 1 | Trait + `Beading` struct + re-exports |
| `crates/slicer-core/src/beading/distributed.rs` | NEW | Step 2 | `DistributedBeadingStrategy` |
| `crates/slicer-core/tests/beading/distributed.rs` | NEW | Step 2 | AC-2 + AC-N1 |
| `crates/slicer-core/tests/fixtures/beading/distributed_10_thickness.json` | NEW | Step 2 | Recorded Orca reference |
| `crates/slicer-core/src/beading/redistribute.rs` | NEW | Step 3 | `RedistributeBeadingStrategy` |
| `crates/slicer-core/tests/beading/redistribute.rs` | NEW | Step 3 | AC-3 |
| `crates/slicer-core/tests/fixtures/beading/redistribute_outer_consistent.json` | NEW | Step 3 | |
| `crates/slicer-core/src/beading/widening.rs` | NEW | Step 4 | `WideningBeadingStrategy` |
| `crates/slicer-core/tests/beading/widening.rs` | NEW | Step 4 | AC-4 |
| `crates/slicer-core/tests/fixtures/beading/widening_thin_wedge.json` | NEW | Step 4 | |
| `crates/slicer-core/src/beading/outer_wall_inset.rs` | NEW | Step 5 | `OuterWallInsetBeadingStrategy` |
| `crates/slicer-core/tests/beading/outer_wall_inset.rs` | NEW | Step 5 | AC-5 |
| `crates/slicer-core/src/beading/limited.rs` | NEW | Step 6 | `LimitedBeadingStrategy` + `compute_and_strip` |
| `crates/slicer-core/tests/beading/limited.rs` | NEW | Step 6 | AC-6 + AC-7 + AC-N2 |
| `crates/slicer-core/tests/fixtures/beading/limited_cap_boundary.json` | NEW | Step 6 | |
| `crates/slicer-core/src/beading/factory.rs` | NEW | Step 7 | `BeadingStrategyFactory::create_stack` |
| `crates/slicer-core/tests/beading/factory.rs` | NEW | Step 7 | AC-8 (composition + Orca match) |
| `crates/slicer-core/tests/fixtures/beading/factory_orca_reference.json` | NEW | Step 7 | |
| `docs/15_config_keys_reference.md` | EDIT | Step 8 | 11 new key entries |
| `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` | EDIT | Step 8 | 11 new schema blocks |
| `docs/01_system_architecture.md` | EDIT | Step 8 | `beading` sub-module entry |
| `docs/DEVIATION_LOG.md` | EDIT | Step 8 | Add `D-111-ARACHNE-SENTINEL-STRIP` (D-9 itself lives in the roadmap) |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | EDIT | Step 8 | Flip rows to DONE |

## Read-Only Context

| File | Range | Purpose |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | Phase 11 rows | Task definitions |
| `docs/15_config_keys_reference.md` | existing entry format (50-line range) | Template for 11 new entries |
| `docs/03_wit_and_manifest.md` | §"Module Manifest TOML" | `[config.schema.*]` block format |
| `docs/01_system_architecture.md` | full | Sub-module registration pattern |
| `crates/slicer-core/src/lib.rs` | full | Existing `pub mod` set (extended from P110) |
| `crates/slicer-core/src/voronoi.rs` | header + module structure | FORWARD-DEP on draft P110 — does not exist yet; read only after P110 lands |
| `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` | full — FORWARD-DEP on P110/T-205 (does not exist until P110 creates it) | P110 skeleton manifest; AC-9 appends the 11 keys after confirming no collision |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/*.cpp` — delegate per file via SUMMARY/LOCATIONS only.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (~10000 LOC) — LOCATIONS dispatch only for the 11 `m_params.*` defaults.
- All other M2 packets (P110, P112) — both draft; P110 not yet shipped.
- M1 packet directories — closed.
- `target/`, lockfiles, generated bindgen output.
- `crates/slicer-ir/src/slice_ir.rs` — no IR changes in this packet.

## Expected Sub-Agent Dispatches

| Step | Dispatch | Scope | Return format |
| --- | --- | --- | --- |
| Step 1 | OrcaSlicer LOCATIONS — BeadingStrategy.h | base interface | ≤ 10 entries: method signatures + Beading fields |
| Step 2 | OrcaSlicer SUMMARY — DistributedBeadingStrategy.cpp | Gaussian math | ≤ 150 words: compute body + decay constant |
| Step 3 | OrcaSlicer SUMMARY — RedistributeBeadingStrategy.cpp | outer preservation | ≤ 100 words |
| Step 4 | OrcaSlicer SUMMARY — WideningBeadingStrategy.cpp | thin-feature regime | ≤ 100 words |
| Step 5 | OrcaSlicer SUMMARY — OuterWallInsetBeadingStrategy.cpp | offset rule | ≤ 100 words |
| Step 6 | OrcaSlicer SUMMARY — LimitedBeadingStrategy.cpp | cap + sentinel insertion | ≤ 150 words |
| Step 7 | OrcaSlicer LOCATIONS — BeadingStrategyFactory.cpp | create_strategy body | ≤ 10 entries showing wrapping |
| Step 8 | OrcaSlicer LOCATIONS — PrintConfig.cpp for 11 `m_params.*` | defaults + units | ≤ 20 entries |
| Each step | `cargo test -p slicer-core <pattern> 2>&1 \| tee target/test-output.log` | n/a | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |

## Data & Contract Notes

- **`Beading`**: `{ total_thickness: f64, bead_widths: Vec<f64>, toolpath_locations: Vec<f64>, left_over: f64 }`. Invariant: `bead_widths.len() == toolpath_locations.len()` (debug-asserted); `total_thickness == bead_widths.iter().sum::<f64>() + left_over` within 0.0001 mm; `bead_widths` ordered from outermost to innermost (matches OrcaSlicer ordering).
- **`BeadingFactoryParams`**: bundles the 11 `m_params.*` values + the `outer_wall_offset` toggle. Default trait impl reads from manifest defaults; runtime values come from `ConfigView` in P112 wire-up.
- **Trait surface (final)**:
  ```rust
  pub trait BeadingStrategy: Send + Sync {
      fn compute(&self, thickness: f64, bead_count: usize) -> Beading;
      fn optimal_bead_count(&self, thickness: f64) -> usize;
      fn get_transition_thickness(&self, lower_bead_count: usize) -> f64;
      fn optimal_thickness(&self, bead_count: usize) -> f64;
      fn type_label(&self) -> &'static str;  // for AC-8 composition verification
  }
  ```
- **No serde on Beading**: Beading is a runtime value, not an IR type. JSON goldens use a test-helper `BeadingForTest` newtype with serde, converting back to `Beading` at read time.

## Locked Assumptions and Invariants

- `beading/` placed in `slicer-core` per docs/13 §Out of Scope (Tier-2 pipeline data structures belong in slicer-core). Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT`.
- OrcaSlicer wraps decorators in the order `Limited(OuterWallInset(Widening(Redistribute(Distributed))))` where `Limited` is the outermost — the call site sees a `Limited` and dispatch flows inward. This packet matches.
- `WideningBeadingStrategy` checks input thickness against `min_input_width` BEFORE delegating. Below threshold: produces single thin bead and returns early (does NOT call inner). At/above threshold: delegates to inner unmodified.
- `LimitedBeadingStrategy` caps `optimal_bead_count` at `max_bead_count`. Sentinel insertion happens in `compute` when delegated `bead_count` exceeds the cap. The actual mechanics: sentinels are zero-width entries at the cap boundary; their `toolpath_locations` are placeholder values that downstream centrality propagation reads but doesn't follow.
- `OuterWallInsetBeadingStrategy` modifies ONLY `toolpath_locations[0]` and `toolpath_locations[bead_widths.len() - 1]` by `±outer_wall_offset`; widths and inner locations untouched. The decorator is a no-op when `outer_wall_offset == 0`.
- Strip-pass strips zero-width entries in pairs: `(widths[i], locations[i])` removed together; length invariant preserved post-strip.

## Risks and Tradeoffs

- **Float comparison fragility.** The 10-thickness `Distributed` golden uses 0.0001 mm (1 unit) tolerance. If boostvoronoi's primitives or the Gaussian decay constant differ by even 1 ULP between platforms, tests may flake on x86 vs aarch64. Mitigation: assert `(a - b).abs() < 1e-4` (slicer units) rather than `assert_eq!` on raw f64.
- **Trait object overhead.** Box<dyn> dispatch adds vtable cost per call. Acceptable for now; M2 perf budget is not in scope for this packet.
- **OrcaSlicer SUMMARY drift.** A SUMMARY ≤ 150 words may omit edge cases (e.g., what `LimitedBeadingStrategy` does when `bead_count == 0`). Mitigation: the goldens are the source of truth; if a strategy can't make the golden green, re-dispatch a tighter SUMMARY for that edge case.
- **`D-111-ARACHNE-SENTINEL-STRIP` rationale.** The strip-pass is a deliberate divergence from OrcaSlicer (which carries zero-width sentinels into the toolpath output and relies on infill modules to skip them). The `DEVIATION_LOG.md` entry MUST explain WHY this codebase strips instead — namely, the WIT-boundary `WallLoop` type's invariant that `bead_widths.iter().all(|&w| w > 0.0)` (avoiding a contract change at the boundary). D-9 in the roadmap already records the decision; `D-111-ARACHNE-SENTINEL-STRIP` records the implementation log entry using the log's `D-<pkt>-<SLUG>` convention.

## Context Cost Estimate

- Aggregate: M.
- Largest single step: Step 2 (Distributed + 10-thickness golden). Sub-step budget: M. Step 7 (factory + multi-stage golden) is M. The other strategy steps (3–6) are S each.
- Highest-risk dispatch: Step 2's `DistributedBeadingStrategy.cpp` SUMMARY. The Gaussian-decay code is dense; if the SUMMARY returns > 200 words, re-dispatch tighter focused on `compute` body alone.

## Open Questions

- **[FWD — RESOLVED]** `slicer_core::flow` module (from P105) now EXISTS (`crates/slicer-core/src/flow.rs`, carrying `line_width_to_spacing`), but `to_slicer_units` specifically was never added; `flow_correction` still lives in `crates/slicer-core/src/lib.rs`. **Action for Step 8:** do NOT call `slicer_core::flow::to_slicer_units` (it doesn't exist) — implement the default translation inline in `beading/factory.rs::BeadingFactoryParams::default` using the `/100` rule from `docs/08_coordinate_system.md`. If a `to_slicer_units` helper is later added, a follow-on can migrate the call.
- **Resolved (updated to reflect shipped shape).** `BeadingFactoryParams` does NOT derive `serde::{Deserialize, Serialize}` directly — `serde` is only a `[dev-dependencies]` entry in `crates/slicer-core/Cargo.toml`, not a real dependency, so deriving on a `src/`-crate struct would have required promoting it to a production dependency. Instead, the `factory_orca_reference.json` golden is loaded via a test-local `ParamsFixture` mirror struct (`Deserialize`-derived, defined in the test file where the dev-dependency is available) plus a `From` conversion into `BeadingFactoryParams`, matching the `BeadingForTest` test-helper pattern used elsewhere. This is the accepted permanent shape — see `closure-log.md` § 4 for the full rationale and the one-line follow-up if a reviewer later wants the direct derive.
- **None [BLOCK].** D-9 is closed (T-215b implements the closed decision); no other gates.
