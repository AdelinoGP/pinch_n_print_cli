# Opt-in SCAD input for `slice` and `mesh import`

Self-contained OpenSCAD scripts can be ingested as a model source for
`pnp_cli slice` and `pnp_cli mesh import`. The feature is **off by default**:
the ordinary build has no SCAD engine, and a `.scad` path is refused with an
actionable error rather than silently parsed as something else.

This page is the product contract for *ingestion*. Evaluation, diagnostics and
conversion are owned by the adapter described in
[`docs/scad/adapter.md`](adapter.md); feasibility and engine identity are in
[`docs/scad/windows-feasibility.md`](windows-feasibility.md) and
[`docs/scad/openrscad-compatibility.md`](openrscad-compatibility.md).

## Enabling it

| Scope | Feature |
|---|---|
| CLI | `pnp-cli/scad` |
| Library (`slicer-model-io`) | `slicer-model-io/scad` |
| Adapter (engine) | `slicer-scad/openrscad`, enabled transitively |

```bash
# Build the CLI with SCAD ingestion
cargo build -p pnp-cli --features pnp-cli/scad

# Library consumers
cargo test -p slicer-model-io --features scad
```

Prerequisites are the adapter's: a Rust toolchain, `wasm32-unknown-unknown` for
guest modules, plus CMake and MSVC for the native Manifold kernel. The default
build needs none of the native prerequisites because no engine is compiled.

The feature does not imply property testing, the SDK's `property_testing`
feature, or Proptest, and it never enters a guest module's feature set.

## Commands and flags

The two flags exist **only** on `slice` and `mesh import`, and only in an
enabled build:

```text
pnp_cli slice        --model <path.scad> --output <path>
                     [--scad-param NAME=EXPR]... [--scad-extrude-height MM]
pnp_cli mesh import  --input <path.scad> --output <path>
                     [--output-format stl|obj|3mf]
                     [--scad-param NAME=EXPR]... [--scad-extrude-height MM]
                     [--stats]
```

- `--scad-param NAME=EXPR` is repeatable. Each raw token splits at its **first**
  `=`; the name keeps its case (including `$` names) and the expression bytes are
  forwarded verbatim. Later duplicates win, but every supplied expression is
  still validated, so an invalid overwritten expression still fails.
- `--scad-extrude-height MM` gives a purely 2D result an explicit thickness.
  There is no implicit thickness: a 2D source without this flag is an error.
- Both are **frontend input options**. They are not slicer configuration: they
  never become `ConfigKey`s, never appear in `ResolvedConfig`, never enter a
  slicer JSON config, and never show up in emitted G-code.

`slice --model`, `slice --config` and `mesh import --input/--output-format` keep
their existing meaning. `mesh import` gains no `--config` or bed option, and no
`--format` alias.

### Parameters versus configuration

A script variable named `layer_height` is **not** the slicer's `layer_height`.
The SCAD override changes script geometry; the slicer layer height still comes
from the config or its default:

```bash
# Geometry height 5 mm, 0.4 mm layers (≈13 layers)
pnp_cli slice --model part.scad --scad-param layer_height=5 \
              --config slicer.json --output out.gcode
```

An override rebinds a name the **source declares**. Overriding a name the source
never declares leaves it `undef`, which evaluates to empty geometry — that is
the engine's semantics, not a frontend policy.

## Exit codes

| SCAD outcome | `slice` | `mesh import` |
|---|---|---|
| Complete validated output, including recognized information | `0` | `0` |
| CLI option/format conflict | `2` | `2` |
| Missing/unreadable source, or output write failure | `1` | `2` (`UNREADABLE`) |
| Empty evaluated geometry | `1` | `3` (`EMPTY_OR_TRIVIAL`) |
| Disabled support, 2D without height, or any adapter rejection | `1` | `4` (`PARSE_ERROR`) |

`WARNINGS_OR_PARTIAL` (1) is **never** used for SCAD import: accepted
informational diagnostics are not warnings, and partial SCAD geometry is a
failure, not a partial success. STEP's exit table is unchanged.

Stable stderr fragments:

| Fragment | Meaning |
|---|---|
| `SCAD_SUPPORT_DISABLED` | This build has no `scad` feature; rebuild with `pnp-cli/scad` or `slicer-model-io/scad`. |
| `SCAD_OPTIONS_REQUIRED` | `load_model` never evaluates scripts. Call `load_scad_model` (library) or use `slice`/`mesh import`. |
| `SCAD_NEEDS_EXTRUSION` | 2D contours with no `--scad-extrude-height`. |
| `SCAD_EMPTY_MODEL` | The script evaluated to no geometry (also when a height was supplied). |
| `SCAD_EVALUATION[<kind>]` | The adapter rejected the source; `<kind>` is the adapter's stable kind name. |
| `SCAD_INVALID_PARAMETER_ARGUMENT` | `--scad-param` was not `NAME=EXPR` (missing `=`, empty name, empty expression). |
| `SCAD_INVALID_EXTRUSION_HEIGHT` | The height was not finite and strictly positive. |
| `SCAD_OPTIONS_REQUIRE_SCAD_INPUT` | A SCAD flag was supplied for a non-SCAD input. |
| `SCAD_STEP_OPTION_CONFLICT` | `--merge-components` or `--no-repair` on SCAD input; both are STEP-only. |

`SCAD_SUPPORT_DISABLED` and `SCAD_OPTIONS_REQUIRED` exist in **both** builds.
Feature-off builds do not accept the two flags at all: passing one is an
unknown-argument usage error (exit `2`), and a `.scad` input without flags still
reaches `SCAD_SUPPORT_DISABLED` instead of a STEP/STL parse error.

## Strictness

Evaluation is exact-or-fail. The adapter rejects, and this frontend preserves
unchanged:

- external resource requests (`include`, `use`, `import`, `surface`, …) even
  when a sibling file is readable and other geometry rendered;
- partial output (nothing published), degraded output, and approximated or
  skipped geometry;
- unknown diagnostic channels at the pinned engine revision;
- mixed 2D/3D results, an extrusion height supplied for a 3D result, invalid
  options, non-finite or out-of-range coordinates,
  invalid contour/mesh topology, complexity-limit and worker failures.

Accepted diagnostics are the `Information` class only (for example
`scad_echo`). They are printed to stderr — for `mesh import --stats` as one
`{"event":"diagnostic","kind":"scad_information",…}` JSONL row per diagnostic,
otherwise as a plain `note: scad <code>: <message>` line. G-code and mesh bytes
never share stdout with them.

There is **no repair, no approximation and no implicit extrusion** on this path.
`mesh import` therefore rejects the STEP-only `--merge-components` and
`--no-repair` rather than silently ignoring them.

## Self-contained only, no render deadline

Scripts must be self-contained: the adapter has no path input, no working
directory fallback and no resource resolution. Source labels are diagnostics,
never search roots.

Evaluation runs under the adapter's supervised worker with bounded resources,
but **there is no promised per-render deadline** and this frontend adds none. A
script that exceeds a configured limit fails as a structured adapter error; it
does not return partial geometry.

## One object, original identity

A successful 3D evaluation becomes **exactly one** `MeshIR` object:

- identity is `uuid5(NAMESPACE_OID, "<basename>#0")` — original file basename
  plus object index, the same rule STL/OBJ use. Different directories with the
  same basename give the same id (this is not a hash-injectivity promise), and
  output paths, temporary names and parameter digests never influence it;
- `ObjectConfig.data` is empty, there are no modifier volumes and no paint data:
  colors and `group()`s produce no tools or materials;
- disconnected bodies stay inside that single indexed mesh, keeping their
  relative positions;
- `MeshIR.build_volume` is the union of the object's bounding box.

## Placement: `slice` places, `mesh import` does not

- **`slice`** treats an explicitly loaded SCAD model like a bare model: the
  established whole-model translation centres the XY bounding box on the bed
  (`bed_shape`, default 250 × 250) and drops world Z minimum to zero. All bodies
  move by the same vector, so a multi-body print keeps its arrangement.
  A 3MF's authored placement is still untouched.
- **`mesh import`** writes **source coordinates**. There is no bed placement
  and no bed/placement flag on the command; the exported mesh is exactly what
  the script described (scaled in millimetres, not PnP IR units).

Units: 1 PnP IR unit = 100 nm = 10⁻⁴ mm. The adapter emits millimetres;
conversion and quantization remain the adapter's responsibility
(see [`docs/08_coordinate_system.md`](../08_coordinate_system.md)).

## What is intentionally not supported

Another evaluator or converter, SCAD in `support-preview`, `visual-debug`, DAG
model probes or `mesh convert`, materials/colors, persistent caching, a bed
printer option on `mesh import`, full language parity, and randomized
full-pipeline campaigns. Runtime-side path loading is also out: the runtime
still consumes a preloaded `Arc<MeshIR>`.
