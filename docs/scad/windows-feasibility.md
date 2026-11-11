# Windows feasibility: the OpenRSCAD SCAD adapter

This file records the **adapter-prerequisite** disposition of the G-SCAD gate. It
covers the optional `slicer-scad` adapter only. Full G-SCAD closure — recipe
generation and shrink measurements, final eligible property profiles — belongs to
`docs/spec_packets/proptest-openrscad_04_pbt-scripted/` and is **pending** there.

Upstream CI or source inspection alone does not satisfy this prerequisite. What
follows is measured on this machine.

## Host and toolchain

| Item | Value |
|---|---|
| OS | Windows 10.0.26300.9711 (x86_64) |
| Rust | `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| Cargo | `cargo 1.96.0 (30a34c2 2026-05-25)` |
| Toolchain policy | `rust-toolchain.toml` pins `1.96.0`; the repository MSRV is `1.91.0` and was **not** changed |
| CMake | `cmake version 4.4.3` |
| MSVC | Visual Studio 18 Community, installation version `18.9.12112.369`; toolsets `14.51.36231` and `14.52.36615` installed, selected by CMake's default generator discovery (the exact toolset was not separately pinned) |
| Engine native dependency | `manifold-csg` 0.3 (C++ Manifold via CMake) |

The upstream engine declares `rust-version = "1.85"`. It was built and tested under
this repository's pinned 1.96.0; the repository toolchain was not changed to obtain
a pass.

## Source provenance

| Item | Value |
|---|---|
| `source_revision` (this repo) | `1b0f0f6537c672a892a5a959e5ca3bb072246588` |
| Working tree during measurement | dirty: `Cargo.toml`, `Cargo.lock`, `.github/workflows/ci.yml`, `docs/01_system_architecture.md`, `docs/08_coordinate_system.md` modified; `crates/slicer-scad/`, `docs/scad/`, `vendor/openrscad/` new |
| Engine base revision | `1a45bd33f2886c32bda32fe6e8358a67ab03f5ef` (`matthova/openrscad`, 0.14.2) |
| Engine patch set | `vendor/openrscad/patches/0001-eval-adapter-seams.patch`, `0002-geom-adapter-seams.patch` |
| Engine patch digest | see `vendor/openrscad/PATCHES.md`; recomputed at test time by `tests/scad_feature_contract.rs` |
| Engine resolution | `vendor/openrscad/Cargo.lock`, committed with the vendored tree |

The vendored tree reproduces byte-for-byte from base + patches (verified by
applying the patch files to a pristine checkout and diffing the four engine
crates).

## Build evidence

Both configurations build, and the default build does not touch the engine.

| Command | Result |
|---|---|
| `cargo build --workspace` | PASS — the workspace member's default feature set is empty, so no engine crate is compiled |
| `cargo check --workspace --all-targets` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo build -p slicer-scad --no-default-features` | PASS |
| `cargo build -p slicer-scad --features openrscad` | PASS (C++ Manifold compiled by CMake/MSVC) |
| `cargo check -p slicer-scad --all-targets --features openrscad` | PASS |
| `cargo clippy -p slicer-scad --all-targets --features openrscad -- -D warnings` | PASS |
| `cargo xtask check-literals` | PASS — 0 violations |
| `cargo xtask check-test-quality --report` | PASS — 0 findings |

### Dependency isolation

| Graph | Evidence |
|---|---|
| `cargo tree -p slicer-scad --no-default-features -e normal,build,features` | no `openrscad*`, no `manifold-csg`, no `rustybuzz`/`fontdb`, no `slicer-sdk`, no `slicer-runtime`, no `proptest` |
| `cargo tree -p slicer-scad --features openrscad ...` | adds the four vendored engine crates and their native deps; still no `slicer-sdk`, `slicer-runtime` or `proptest` |
| Default CLI graph | unchanged: `pnp-cli` has no `slicer-scad` edge, so no SCAD-enabled CLI claim is made here (that belongs to the ingestion packet) |
| Discovered guest graphs | unchanged; this packet edits no guest input, so no guest rebuild was required |

A default workspace build therefore does not compile engine or native
dependencies merely because the adapter is a workspace member.

### Build cost, measured separately from adapter cost

| Measurement | Value |
|---|---|
| Vendored engine cold build in a scratch target directory (`cargo build`, `dev`, `jobs = 1`) | 3 m 26 s |
| Full cold `cargo build -p slicer-scad --features openrscad` in a fresh `CARGO_TARGET_DIR`, `dev`, `jobs = 1` | 4 m 26 s (267 s; 1.1 GB target directory) |

The second figure is the whole adapter edge: registry dependencies, the C++
Manifold kernel, the four vendored engine crates and the adapter itself. It is a
build cost, not a per-evaluation cost.

Build cost is **not** a per-evaluation cost and is not folded into the adapter
measurements.

## Measured adapter costs

`scad_measure_representative_costs` (in `tests/scad_adapter_contracts.rs`) times
one warm `evaluate` call — parse, parameter evaluation, program evaluation, exact
render and checked conversion — for AC-3's fixtures, over 10 runs after a warm-up:

| Fixture | Median | p95 |
|---|---|---|
| holed 2D profile, `Contours` | 276.2 µs | 309.9 µs |
| the same profile, `extrude_height_mm = 2.0` → `Mesh` | 536.7 µs | 998.5 µs |
| drilled 3D solid → `Mesh` | 671.5 µs | 713.1 µs |

These are `dev`-profile numbers on this host. They describe adapter overhead, not
engine capability, and they are not property profiles.

## Derived adapter defaults

`ScadLimits::default()` is a caller-visible contract. Each value is bounded, and
the basis is recorded so a later change can be justified rather than guessed.

| Field | Default | Basis |
|---|---|---|
| `evaluation_steps` | 20 000 000 | The AC-2 fixtures need well under 100 000 steps; 20 M leaves two orders of magnitude of headroom while still bounding an adversarial program. The budget tests use 100 000 deliberately. |
| `max_source_bytes` | 1 048 576 | 1 MiB of checked UTF-8 across source plus every override name/expression. |
| `max_ast_nodes` | 200 000 | Above any measured fixture by orders of magnitude; bounds a pathological generated program before traversal. |
| `max_ast_depth` | 256 | A real parametric model nests far below this; the AC-N4 fixture trips it at 8. |
| `max_geometry_nodes` | 100 000 | Bounds the evaluated CSG tree before rendering. |
| `max_facets` | 360 | The engine's own default tessellation cap; a larger explicit `$fn` is `ComplexityLimit`. |
| `max_vertices` | 1 000 000 | Output allocation bound for one render. |
| `max_triangles` | 2 000 000 | Output allocation bound for one render. |
| `max_abs_coordinate_mm` | 10 000.0 | 10 m, two orders above any printer envelope; keeps the integer grid and the documented `i128` proofs well inside range. |
| `worker_stack_bytes` | 256 MiB | Comfortably above the measured floor below; matches the engine's own CLI stack budget. |

### Worker stack policy (measured)

The engine's contained recursion guard trips at 6 000 native frames. A stack too
small to hold those frames overflows *before* the guard fires, turning a
diagnosable `PartialOutput` into a process abort. Measured with the `cum`
non-tail-recursion fixture on this host, `dev` profile:

| `worker_stack_bytes` | Outcome |
|---|---|
| 16 MiB, 24 MiB, 32 MiB, 36 MiB, 40 MiB, 44 MiB | `STATUS_STACK_OVERFLOW` on the `scad-eval` worker thread |
| 48 MiB, 64 MiB, 96 MiB | `Err(PartialOutput)` with `recursion_aborted` — the guard fires and is classified |

`MIN_WORKER_STACK_BYTES` is therefore **64 MiB**: the measured debug floor plus
margin. A `dev` frame is far larger than upstream's ≈ 1.6 KiB release estimate,
which is exactly why this was measured rather than derived from that estimate.

## Narrow test results

| Lane | Command | Result |
|---|---|---|
| `F` | `cargo test -p slicer-scad --no-default-features --test scad_feature_contract` | PASS — 1 test executed (not a zero-test binary); asserts `engine_info().is_none()` and `BackendDisabled` even for a source that does not parse |
| `F`+`A` | the same target with `--features openrscad` | PASS — asserts the pinned revision, the recomputed patch digest and `backend = "manifold_cpp"`, and a non-empty cube mesh |
| `A` | `cargo test -p slicer-scad --features openrscad --test scad_adapter_contracts` | PASS — 9 tests; 1 measurement entry point ignored by default |
| `U` | `cargo test -p slicer-scad --features openrscad --lib -- scad_` | PASS — 20 tests, including `scad_conversion_contour_forest`, `scad_rejects_conversion_damage` and `scad_diagnostic_channels_fail_closed` |
| `L` | `cargo test -p slicer-scad --features openrscad --test scad_process_limits` | PASS — 3 tests, ≈ 20 s wall clock (the hard-deadline probe) |

No workspace test run was required: the packet touches no guest input, and the
narrow lanes above exercise every changed surface.

## The `scad-adapter-windows` lane

`.github/workflows/ci.yml` gains a self-contained `scad-adapter-windows` job on
`windows-latest`. It does not depend on the property-foundation job. It:

1. installs the checked-in Rust policy plus the CMake/MSVC prerequisites present on
   the runner image, and records `rustc -vV`, `cargo -V`, `cmake --version`,
   `where cl`, the source revision and the dirty tree;
2. records the default and enabled `cargo tree` graphs and **fails** if the default
   graph reaches the engine or if the enabled normal/build graph reaches SDK,
   runtime or Proptest;
3. runs `F`, asserting that a test actually executed;
4. runs the enabled identity contract, `A`, `U` and `L`, asserting that the `scad_`
   filter selected real tests;
5. runs the narrow clippy gate;
6. uploads `target/scad-adapter/` (logs, trees, reproducers) with `if: always()`.

**Execution status: not yet executed.** Every command in the job was run on this
machine in this session with the same arguments and passed (see the tables above),
but the GitHub job itself has not run, because that requires a push. The AC-6
clause "the required `scad-adapter-windows` PR lane and artifact upload execute"
is therefore **open** until the branch is pushed and the job is observed green.

## Disposition of G-SCAD

**Adapter prerequisite: PENDING**, because one required clause has not executed:
the `scad-adapter-windows` GitHub job and its artifact upload. Everything else the
prerequisite owns is demonstrated here:

- the engine builds and runs on Windows with the repository's pinned toolchain;
- exact evaluation and rendering work through the vendored, patched engine;
- checked conversion rejects damage rather than repairing it;
- the diagnostic classifier is fail-closed on both the mesh and contour paths;
- seeded evaluation repeats across calls, interleaved sources and concurrent
  callers;
- process supervision distinguishes a hard-deadline kill from evaluator budget
  exhaustion;
- representative adapter evaluation and exact-render costs are measured above.

**Still `pending` for packet #4 (`proptest-scripted-evidence`):**

- recipe generation and shrink-cost measurements for the scripted property domain;
- final eligible property profiles;
- the combined G-GEOMETRY decision;
- full G-SCAD closure.

This packet therefore closes only the adapter prerequisite it owns. Full G-SCAD
remains incomplete until #4 supplies the rows above.

## Honest limitations

- **No in-process render deadline.** `evaluation_steps` bounds the evaluator, not
  native rendering. A wall-clock guarantee requires a supervised, killable child
  process; a thread timeout or `catch_unwind` is not sufficient.
- **`max_facets` bounds explicit `$fn` and derived counts for the primitives that
  expose them.** `circle`/`sphere`/`cylinder` carry `r` and `FragmentSpec`, so the
  engine's own `fragments` formula is evaluated before rendering and bounded.
  `offset`, extrusion and `import` nodes are bounded by their explicit `$fn` only;
  their derived counts are bounded after the fact by `max_vertices` and
  `max_triangles`.
- **The pre-quantization contact pass runs at 1 nm resolution, not on exact `f64`
  predicates.** Exact predicates are computed on integer grids; the raw-ring pass
  scales to 1e6 units/mm, which is strictly finer than the 1e-4 mm output grid, so a
  contact visible at 1 nm is rejected. A crossing narrower than 1 nm is not
  representable at the output resolution and is not detected.
- **Edge-incidence validation does not claim arbitrary 3D self-intersection
  freedom.** It checks closed, consistently oriented incidence by exact position
  identity, which is what the fixtures require and no more.
- **`DegradedOutput` has real source fixtures.** A `polyhedron` whose faces do not
  form a closed manifold is a non-manifold boolean operand, so a `union()` around it
  degrades to un-combined operands — plainly and under `projection()`. Both forms
  are committed fixtures that assert the `boolean_fallback` diagnostic.
- **Unsupported platforms are not acceptance claims.** Only Windows was exercised.
- **This host is not a controlled-perimeter runner.** The measurements above are
  `dev`-profile numbers on this machine, not accelerated-mode evidence.
