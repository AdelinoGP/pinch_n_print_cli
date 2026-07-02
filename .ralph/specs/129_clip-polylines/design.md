# Design: 129_clip-polylines

## Controlling Code Paths

- Primary code path: `crates/slicer-core/src/polygon_ops.rs` — new `clip_polylines` beside the
  existing Clipper2 wrappers (`intersect_64` / `union_64` / `difference_64` usage at ~line 78
  shows the crate-invocation idiom; the new function uses the `Clipper64` builder instead of
  the `engine_fns` convenience layer — first builder use in the workspace).
- Neighboring tests or fixtures: `crates/slicer-core/tests/polygon_ops_tdd.rs` (existing
  suite; add the 8 `clip_polylines_*` tests there, reusing its square/hole fixture helpers if
  present).
- OrcaSlicer comparison surface: none — the semantics target is Clipper2's own open-path
  intersection (the OrcaSlicer `intersection_pl` equivalent), reached through the crate API,
  not ported code.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- `polygon_ops` is available on wasm32 and NOT `host-algos`-gated
  (`crates/slicer-core/src/lib.rs:26`); the new function must not introduce a cfg gate or a
  non-wasm32-clean dependency. `clipper2-rust` is pure Rust — no build script, no FFI.

## Code Change Surface

- Selected approach: one `Clipper64` run per call — feed every `ExPolygon` contour and hole as
  closed clip paths, every input polyline via `add_open_subject`, execute
  `ClipType::Intersection` with `FillRule::NonZero`, return `solution_open` converted back to
  `Vec<Vec<Point2>>`. Holes are handled natively by the winding evaluation of the combined
  clip set (per-ExPolygon iteration is NOT needed; the ExPolygon set is one clip universe,
  matching how `intersect_64` treats it today).
- Exact changes: `clip_polylines` (+ rustdoc stating the six geometric guarantees from the
  spec) in `polygon_ops.rs`; 8 tests in `polygon_ops_tdd.rs`; one line in
  `docs/05_module_sdk.md`.
- Rejected alternatives: (a) thicken-polyline-and-intersect — dead option, native open-path
  support exists (verified 2026-07-01); (b) Sutherland-Hodgman variant — same; (c) per-vertex
  point-in-polygon (gyroid's current approach) — the bug this primitive exists to replace.

## Files in Scope (read + edit)

- `crates/slicer-core/src/polygon_ops.rs` — role: home of the new function; expected change:
  +1 public function (~40-60 lines incl. type conversion helpers).
- `crates/slicer-core/tests/polygon_ops_tdd.rs` — role: TDD suite; expected change: +8 tests.
- `docs/05_module_sdk.md` — role: Doc Impact target; expected change: 1 line (helper list).

## Read-Only Context

- `crates/slicer-core/src/lib.rs` — line 26 region only — confirm `pub mod polygon_ops` has no
  cfg gate.
- `crates/slicer-core/src/polygon_ops.rs` — imports region (~line 78) — copy the
  Paths64/Point conversion idiom used by the existing wrappers.

## Out-of-Bounds Files

- `C:\Users\agpen\.cargo\registry\...\clipper2-rust-1.0.3\**` — API facts already recorded in
  `requirements.md`; delegate a FACT dispatch on signature mismatch, never load.
- `OrcaSlicerDocumented/**` — not needed for this packet.
- `modules/core-modules/gyroid-infill/**` — the broken clipper it replaces is out of scope
  until packet 135; do not open.
- `target/`, `Cargo.lock`, generated code — never load.

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-core --test polygon_ops_tdd 2>&1 | tee target/test-output.log |
  grep '^test result'`; return FACT (pass, counts) or SNIPPETS (failing assertion + ≤20
  lines)" — validate each TDD step.
- "Run `cargo clippy -p slicer-core --all-targets -- -D warnings`; return FACT" — lint gate.
- "Run `cargo xtask build-guests --check`; return FACT clean or the STALE list" — freshness
  gate before any guest-adjacent conclusion.
- (Contingency) "Report the exact public signature of `Clipper64::add_open_subject` and
  `Clipper64::execute` in clipper2-rust 1.0.3; return FACT ≤5 lines" — only if compilation
  contradicts the recorded API.

## Data and Contract Notes

- IR or manifest contracts touched: none.
- WIT boundary considerations: none (pure Rust helper), but slicer-core is baked into every
  guest — the freshness gate applies (see Architecture Constraints).
- Determinism: Clipper2 output ordering is deterministic for identical input; tests must not
  assume a specific polyline output ORDER across the result Vec — assert on set membership /
  counts / geometry, not index positions, except where a single output makes index-0 safe.

## Locked Assumptions and Invariants

- `clip_polylines` is generic geometry — it must NOT gain infill-specific parameters (spacing,
  roles, overlap). ADR-0026 locks linking/domain logic in the infill-linker module;
  `slicer-core` gains only this primitive.
- On-edge spans count as inside (Clipper2 boundary rule) — AC-5 pins this; downstream linker
  behavior depends on it.
- The function stays wasm32-clean and un-gated.

## Risks and Tradeoffs

- Clipper2 may merge collinear vertices or split at clip-path vertices, making exact
  point-equality assertions brittle — tests assert geometry within ±2 units tolerance and
  count/coverage properties instead of exact vertex lists (except AC-1's strictly-interior
  case, where no clipping occurs).
- First `Clipper64` builder use in the workspace: if the builder's path-type conversions
  differ from the `engine_fns` layer, the conversion helpers absorb it — keep them private to
  `polygon_ops.rs`.

## Context Cost Estimate

- Aggregate: `S`
- Largest single step: `S`
- Highest-risk dispatch: the contingency crate-signature FACT — must return ≤5 lines, never
  file contents.

## Open Questions

None.
