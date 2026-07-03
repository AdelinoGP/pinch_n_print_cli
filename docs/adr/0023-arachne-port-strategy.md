# ADR-0023 — Arachne Voronoi Crate Selection: `boostvoronoi` and Degeneracy-Handling Strategy

## Status

Accepted (2026-07-03). Documents the already-closed D-7 decision; formalizes the degeneracy-handling contract for the M2 Arachne foundations layer (Phase 10, T-200 in `docs/specs/perimeter-modules-orca-parity-roadmap.md`).

## Context

`docs/specs/perimeter-modules-orca-parity-roadmap.md` tracks an open decision point **D-7 — "Voronoi crate strategy — vendor `boost::polygon` port, adopt existing Rust crate, or write from scratch?"** D-7 is already marked **CLOSED**, adopting [`boostvoronoi`](https://docs.rs/boostvoronoi/) as "a pure-Rust port of `boost::polygon::voronoi`, matches OrcaSlicer's algorithm choice. Confirmed pre-grill." This ADR is the source-grounded record that D-7 references, and additionally specifies the degeneracy-handling strategy required before `crates/slicer-core::voronoi` (T-201) can be built on top of it.

Arachne is a segment-based skeletal-trapezoidation algorithm: its `SkeletalTrapezoidationGraph` (T-202) is built from a **segment Voronoi diagram** of the input polygon's edges, not a point Voronoi diagram. This constrains the crate choice to one that supports Boost's mixed point/segment Voronoi construction (`boost::polygon::voronoi_builder`), not a plain Fortune's-algorithm point-Voronoi implementation.

The pure-Rust constraint follows from `slicer-core`'s role as documented in `docs/01_system_architecture.md` (§ "Crate Responsibilities — `slicer-core`"): `slicer-core` owns per-layer polygon-op primitives as the **canonical host-side implementations** consumed across the pipeline, cross-compiled across the project's host build matrix (Windows/Linux/macOS) without a WASM target for this crate. A dependency requiring a C++ toolchain (e.g. FFI bindings to `boost::polygon`) would add a second build toolchain requirement to every host platform this project supports — a burden every other `slicer-core` primitive (Clipper2 via a Rust binding, `offset2_ex`, `medial_axis`, `polygon_tree`) has avoided.

`boostvoronoi` is **already an optional dependency** in `crates/slicer-core/Cargo.toml`:

```toml
boostvoronoi = { version = "0.12", optional = true }   # line 16
host-algos = ["dep:rayon", "dep:log", "dep:boostvoronoi"]   # line 9
```

This ADR keeps the pin at `0.12` (verified current: `0.12.1` on docs.rs as of this writing) — no version bump is in scope here.

## Decision

**Adopt `boostvoronoi` v0.12 as the Voronoi construction crate for Arachne**, wrapped behind an Orca-shaped API surface at `crates/slicer-core::voronoi` (T-201: `voronoi_from_segments(Vec<Segment>) -> HalfEdgeGraph`).

Rationale (one line, per https://docs.rs/boostvoronoi/): **`boostvoronoi` is a faithful pure-Rust port of `boost::polygon::voronoi`, giving segment-Voronoi construction that matches OrcaSlicer's own algorithm choice (OrcaSlicer links `boost::polygon::voronoi` directly) without introducing a C++ FFI dependency.** The crate confirms this self-description ("This library is a port of the C++ boost voronoi implementation") and supports `i32`/`i64` integral coordinates, matching this project's scaled-integer `Point2` representation directly — no floating-point conversion layer is needed at the call boundary.

### License note

`boostvoronoi` is licensed **BSL-1.0** (Boost Software License 1.0). The Pinch 'n Print workspace is **dual MIT/Apache-2.0**. BSL-1.0 is a permissive, non-copyleft license compatible with a **dependency (link-time) relationship** — this project links `boostvoronoi` as a compiled Cargo crate; no `boostvoronoi` source is copied or vendored into this repository. This is distinct from the OrcaSlicer Attribution Rules (`docs/ORCASLICER_ATTRIBUTION.md`), which govern C++ source **translated/ported by hand** from OrcaSlicer's AGPLv3 codebase (e.g. `SkeletalTrapezoidationGraph`, the 9-stage `WallToolPaths.cpp` pre-processing pipeline). Those porting headers apply to the Arachne algorithm files this ADR's follow-on tasks (T-202–T-205) will create; they do not apply to the `boostvoronoi` dependency itself, which is an independent, differently-licensed crate, not ported code.

### Degeneracy classes and handling strategy

Arachne's segment-Voronoi input (polygon edges after the T-204 pre-processing pipeline) must handle four degeneracy classes. `epsilon_offset` is the WallToolPaths.cpp pre-processing hazard constant: **~11.5 µm in real space**, which converts to **115 units** in slicer coordinate space (`docs/08_coordinate_system.md`: `units = round(mm * 10_000)` → `0.0115 mm * 10_000 = 115`).

| Degeneracy class | Description | Strategy |
|---|---|---|
| Collinear input points | ≥3 input vertices lying on a single line (e.g. straight polygon edges split into multiple collinear segments) | **Rely on Boost-VD's built-in handling.** Boost's sweep-line construction produces degenerate (zero-width) Voronoi cells/edges for collinear input rather than failing; `boostvoronoi` inherits this behavior as a faithful port. No pre-snap needed. |
| T-junctions | A segment endpoint touches the *interior* of another segment (e.g. where a branch wall meets a straight wall) | **Pre-snap.** `boostvoronoi`'s own input contract requires "input points and segments should not overlap except their endpoints" — an unresolved T-junction violates this contract and yields an incorrect diagram. T-junctions must be resolved *before* calling into `voronoi_from_segments`, by subdividing the touched segment at the junction point so all segment-segment contacts become shared endpoints. This is exactly what the T-204 9-stage `WallToolPaths.cpp` pre-processing pipeline (`fixSelfIntersections`, simplify) is responsible for upstream of Voronoi construction. |
| Duplicate vertices | Two or more input points/segment endpoints at the exact same coordinate | **Pre-snap.** Coincident endpoints are deduplicated before construction, using the existing `POINT_COINCIDENCE_EPSILON`-class merge radius (`docs/08_coordinate_system.md` "Constant Conversion Table", VD vertex merge row: `SCALED_EPSILON` ≈ 1 unit). `boostvoronoi` does not guarantee correct handling of literally coincident input sites; dedupe is the caller's responsibility. |
| Near-collinear-within-`epsilon_offset` segments | Segments whose deviation from perfect collinearity is smaller than `epsilon_offset` (~11.5 µm / 115 units) — typically produced by upstream offset/simplify floating-point noise | **Pre-snap**, using `epsilon_offset` as the snap tolerance, matching the T-204 `WallToolPaths.cpp:590-604` pre-processing pipeline (documented hazard: this stage "destroys features < epsilon_offset ~11.5 µm"). Left unsnapped, these near-collinear inputs produce numerically unstable Voronoi cells (sliver edges) that corrupt downstream `SkeletalTrapezoidationGraph` R-values (T-202). This is *not* delegated to Boost-VD — it must be resolved in the T-204 pre-processing stage before the segments ever reach `voronoi_from_segments`. |

`crates/slicer-core::voronoi`'s T-201 fixtures ("Collinear/T-junction stress fixtures pass") are the executable contract for the first two rows of this table; the latter two are exercised by T-204's pre-processed-outline fixture.

### Note on `epsilon_offset`'s numeric value

The `~11.5 µm` / `115 units` figure used above (and in the mandatory hazard doc-comment string `destroys features < epsilon_offset ~11.5 µm` required verbatim in `crates/slicer-core/src/arachne/preprocess.rs` per P110's AC-6 verification contract) is the conventional figure carried over from the packet's original text. The value actually computed in this codebase from OrcaSlicer's literal formula — `epsilon_offset = (allowed_distance / 2) - 1_unit`, with `allowed_distance = 0.025 mm` — is **≈12.499 µm (≈125 units)**, an ~8.7% discrepancy from ~11.5 µm. Both figures are intentionally retained rather than reconciled: the ~11.5 µm string is pinned by a hard test-contract requirement (a literal doc-comment match, not itself a factual claim about the runtime constant), while ≈12.5 µm is the value actually used at runtime. This is not a bug — see `.ralph/specs/110_arachne-voronoi-skt-foundations/closure-log.md` item 2 for the full derivation and rationale.

## Rejected alternatives

- **`voronator`.** Rejected: a pure-Rust Fortune's-algorithm implementation for **point** Voronoi/Delaunay diagrams only. Arachne requires **segment** Voronoi diagrams (polygon edges as sites, not vertices), which `voronator` does not support, and it has no T-junction handling story for mixed point/segment input.
- **Direct `boost::polygon` FFI.** Rejected: requires bundling and linking the C++ Boost.Polygon library (via `bindgen`/`cxx`) across every host build target this project supports (Windows/Linux/macOS), violating the pure-Rust constraint that governs the rest of `slicer-core` and duplicating a toolchain dependency none of the crate's existing `host-algos` deps (`rayon`, `log`) require.
- **From-scratch Fortune's-algorithm / custom segment-Voronoi implementation.** Rejected: high risk of subtle, hard-to-detect degeneracy bugs (collinear input, T-junctions, near-duplicate points — the exact classes tabulated above) that Boost's implementation has already hardened through years of production use. Re-deriving this class of numerically delicate computational-geometry algorithm from scratch for a foundations packet is not justified when a mature, source-equivalent port is available.

## Consequences

- Arachne foundations work (T-200 through T-205) proceeds against `boostvoronoi = "0.12"`, already declared and feature-gated (`host-algos`) in `crates/slicer-core/Cargo.toml`; this ADR requires no `Cargo.toml` change.
- `crates/slicer-core::voronoi` (T-201) is the single call site wrapping `boostvoronoi`'s builder API; the degeneracy-strategy table above is the doc-comment contract its fixtures must exercise.
- Pre-snap responsibility for T-junctions, duplicate vertices, and near-collinear-within-`epsilon_offset` segments lives in T-204's ported pre-processing pipeline (`crates/slicer-core/src/arachne/preprocess.rs`), not inside the `voronoi_from_segments` wrapper itself — the wrapper may assume its input has already been pre-snapped.
- BSL-1.0 is recorded as compatible for a link-time Cargo dependency; no source is vendored, so no OrcaSlicer-style attribution header applies to the dependency itself (it does apply to the hand-ported Arachne algorithm files T-202-T-205 produce).
- D-7 in `docs/specs/perimeter-modules-orca-parity-roadmap.md` now has a source-grounded ADR to reference instead of only its inline one-line closure note.

## Future reviewers

- This ADR documents the D-7 closure recorded in `docs/specs/perimeter-modules-orca-parity-roadmap.md` (row `D-7`, Phase 10 `T-200`). Do not reopen the crate choice without new evidence that `boostvoronoi` fails to support segment-Voronoi construction or that its BSL-1.0 license becomes incompatible with a dependency relationship.
- If a future packet needs to bump `boostvoronoi` past `0.12.x` and the builder API surface changes, update this ADR's Decision section rather than silently drifting the pin.
