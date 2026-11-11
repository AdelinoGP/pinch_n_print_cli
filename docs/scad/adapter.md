# SCAD adapter (`slicer-scad`)

`crates/slicer-scad` is the optional, engine-opaque boundary between self-contained
OpenSCAD source and Pinch 'n Print geometry. It owns source-to-geometry evaluation
and nothing else.

It is **not** an importer. It returns geometry only:

- `slicer-model-io`'s `assemble_object` still owns `ObjectMesh`/`MeshIR` assembly,
  source-derived identity and object IDs;
- bed placement, material and colour assignment stay with the host frontend;
- CLI ingestion and scripted property input consume this crate (packets
  `proptest-openrscad_05_scad-ingest` and `_04_pbt-scripted`), they do not
  re-implement it.

See `docs/01_system_architecture.md` § Data Ownership Rules.

## Feature contract

```toml
slicer-scad = { path = "crates/slicer-scad" }                            # no engine
slicer-scad = { path = "crates/slicer-scad", features = ["openrscad"] } # pinned engine
```

| Configuration | `engine_info()` | `evaluate(...)` |
|---|---|---|
| `default = []` | `None` | `Err(BackendDisabled)`, before parsing, before any allocation |
| `features = ["openrscad"]` | `Some(ScadEngineInfo)` | evaluates |

A default workspace build does not compile the engine, its C++ Manifold kernel or
its font stack. The adapter has no SDK, no runtime and no Proptest edge in any
configuration. `tests/scad_feature_contract.rs` asserts both states; it is
deliberately not gated on the feature, so the feature-off run executes a real
test rather than a green zero-test binary.

The engine itself is a vendored, patched subset pinned at
`matthova/openrscad@1a45bd33f2886c32bda32fe6e8358a67ab03f5ef`. Its provenance,
patch set and patch digest are in `vendor/openrscad/PATCHES.md`; the diagnostic
classification table is in `docs/scad/openrscad-compatibility.md`.

## The one entry point

```rust,ignore
pub fn engine_info() -> Option<ScadEngineInfo>;
pub fn evaluate(source: &ScadSource, options: &ScadEvalOptions)
    -> Result<ScadEvalOutput, ScadError>;
```

```rust,ignore
let source = ScadSource {
    label: "bracket.scad".to_string(),   // diagnostics only, never a path
    text: source_text,
};
let options = ScadEvalOptions {
    parameters: vec![ScadParameter {
        name: "wall".to_string(),        // ordinary SCAD identifier
        expression: "2.4".to_string(),   // exactly one expression
    }],
    extrude_height_mm: Some(3.0),       // 2D results become a closed solid
    limits: ScadLimits::default(),
    random_seed: Some(1),
};

let out = evaluate(&source, &options)?;
match &out.geometry {
    ScadGeometry::Contours(expolygons) => { /* exact 2D profile */ }
    ScadGeometry::Mesh(mesh) => { /* exact 3D solid, flat Vec<u32> indices */ }
    ScadGeometry::Empty => { /* an empty model: reject for import */ }
}
```

`ScadEvalOptions` and its records implement `Debug + Clone + Serialize +
Deserialize`; `ScadError` adds `Display + std::error::Error`. Deserialising an
options record never bypasses validation — `evaluate` always re-validates before
doing any work.

### `ScadGeometry::Contours` versus explicit extrusion

A purely 2D result is returned as contours and **never** given an implicit
thickness:

- `extrude_height_mm = None` → `ScadGeometry::Contours(Vec<ExPolygon>)`, outer
  contours CCW, holes CW, even-odd nesting reconstructed from the engine's
  unordered rings.
- `extrude_height_mm = Some(h)` (finite, strictly positive, within
  `max_abs_coordinate_mm`) → the same profile extruded from Z=0 to Z=h through the
  same exact engine boundary, returned as `ScadGeometry::Mesh`.
- A 3D result with a height is `ExtrusionHeightOn3d`; a non-empty result that
  mixes 2D and 3D is `MixedDimensions`; an empty result is `Empty` **even with a
  valid height**.

Import and slicing callers must reject `Empty` as an empty model. A property
domain that is explicitly allowed to be empty may use it.

### Overrides

- Names are case-sensitive SCAD identifiers, optionally `$`-prefixed. They are
  never normalised, lowercased, or routed through slicer config.
- Each expression is parsed as exactly one expression. A trailing statement, an
  extra top-level assignment, or a second `name = (...)` is a parse error — not a
  first-match-wins reinterpretation.
- Overrides are processed in input order and the **last** occurrence of a name
  wins. Every occurrence is still parsed and evaluated, and
  `ScadProvenance::parameters` preserves the raw list including duplicates.
- A later parameter does not see an earlier override's binding: each expression
  is evaluated in the engine's constant/base scope.
- An undefined result is `InvalidParameter`, never silently empty geometry.

An override expression is evaluated against the **same** deny-all resolver as the
program and its own warnings and echoes are classified like any other diagnostic.
So `-D x=dxf_dim(file="…")` is `ExternalResourceDenied`, `-D x=let(a=unknown_fn()) 1`
is `UnknownDiagnostic`, and `-D x=not_defined` is `InvalidParameter`. An override
can neither read a file the program cannot nor smuggle an unclassified diagnostic
past the classifier.

### Limits and the shared evaluation allowance

`ScadLimits` has no "unlimited" sentinel. `evaluation_steps = 0`, `u64::MAX`, any
zero capacity, a non-finite or non-positive coordinate bound, or a worker stack
outside `[MIN_WORKER_STACK_BYTES, MAX_WORKER_STACK_BYTES]` is `InvalidOptions`.

One finite `evaluation_steps` allowance covers **both** override expressions and
the program: each expression consumes from the remainder, and the program gets
what is left. Resetting the allowance per override is forbidden — otherwise N
overrides would buy N allowances — and the tests assert it two-ways: a single
expensive override exhausts a budget the program alone fits in, and a second
expensive override fails under a budget each one individually fits in.

`max_source_bytes` bounds the checked total UTF-8 length of the source plus every
override name and expression, so many small overrides cannot evade it. AST node
and depth capacities apply across the program and the parsed override
expressions. Exceeding any of them is `ComplexityLimit`.

Limits bound allocation and traversal; they do **not** interrupt synchronous
native rendering and they are not a deadline. See "No deadline" below.

## Resource boundary

There is no caller-supplied resolver and no working-directory fallback. `evaluate`
installs a deny-all resolver that records every `load`/`load_bytes` request,
returns no content, and fails the whole evaluation with
`ExternalResourceDenied` even when a sibling primitive rendered fine. That makes
`include`, `use`, `import` and any other native resource seam a hard failure
rather than a warning next to plausible geometry.

Font handling never touches the filesystem implicitly: the engine's bundled
Liberation family is deterministic, and OS font discovery
(`register_system_fonts`) is never called. A `text()` fixture therefore succeeds
from bundled data while an `import()` fixture is denied.

`source.label` is diagnostic data only. It is never opened and never used as a
resource base directory.

## Diagnostics: one fail-closed classifier

Every diagnostic-producing path — program evaluation, exact mesh render, and the
contour/projection/lowering path including explicit extrusion — funnels through
the same classifier. A successful evaluation carries only
`ScadDiagnosticClass::Information`.

| Observed outcome | Result |
|---|---|
| Syntax error | `Parse`, with the source label and byte span |
| Invalid expression or name | `InvalidParameter`, with the offending `parameter` |
| Budget exhausted, in a parameter or the program | `EvaluationBudgetExceeded` |
| Program evaluation error | `Evaluation` |
| Any actual external request | `ExternalResourceDenied`, even with remaining geometry |
| Recursion-aborted or otherwise partial output | `PartialOutput`, no usable geometry |
| Render fallback / recoverable geometry error | `DegradedOutput`, even with a non-empty mesh |
| Approximation or geometry-skipping warning | `ApproximateOutput` |
| Unknown warning, severity or channel | `UnknownDiagnostic` until classified at the pin |
| Recognised information (`scad_echo`, `assignment_overwritten`) | preserved, evaluation continues |

Classification keys on the engine's stable diagnostic **codes**, not on English
prose. A code nobody classified falls through to `UnknownDiagnostic`; that is the
intended failure mode, and it is why bumping the engine pin without re-deriving
`docs/scad/openrscad-compatibility.md` fails closed instead of silently
accepting new warnings.

Primary-error precedence is total and independent of emission order: external
request, then partial, degraded, approximate, unknown, then conversion. All
diagnostics gathered before the failure are retained on the returned
`ScadError`.

## Checked conversion, never repair

### 2D: even-odd forest

Raw rings from the engine are flat, unordered and arbitrarily wound. Conversion:

1. validates every raw ring (finite, within the configured coordinate bound, at
   least three points after removing one redundant exactly-equal terminal
   closure point, non-zero area);
2. applies the canonical half-away-from-zero rounding rule **directly on `f64`**,
   with finiteness, bound, product-finiteness and representable-range checks
   before the cast (see `docs/08_coordinate_system.md`);
3. re-validates on the integer grid: no collapsed edges, non-zero area, no
   self-intersection, no touching between distinct rings;
4. reconstructs the even-odd nesting forest independently of input order and
   winding, and rejects if quantization changed any ring's depth or parent;
5. emits outer contours CCW and holes CW.

Nested islands become separate components. Nothing is dropped to rescue a result:
not a quantization-collapsed vertex, not a zero-area ring, not a crossing or
touching boundary, not a changed nesting or lost component.

### 3D: indexed mesh

Conversion checks vertex finiteness and range, index validity and the flat
stream's whole-triangle property, triangle distinctness and non-zero area, and
f64→f32 narrowing (including the IR's documented subnormal policy: subnormal
f32 values normalise to zero, and a normalisation that creates degeneracy or
inversion is rejected rather than accepted). Closed, consistently oriented edge
incidence is then checked by exact position identity.

The engine's own Manifold boundary normalises coincident vertices by **exact
identity**, not by an epsilon bucket: a 1e-7 mm weld would merge two genuinely
distinct vertices and drop the triangles that collapsed, deleting a thin body with
no diagnostic. Exact identity still gives the kernel the shared edges a revolution
seam needs, and cannot lose geometry.

Exact coincident-position deduplication is allowed only as representation
normalisation that preserves the triangle mapping; there is no epsilon welding,
face deletion, hole filling, auto-repair or fallback tessellation. Disconnected
bodies are preserved inside one indexed mesh. Arbitrary 3D self-intersection
freedom is not claimed by edge-incidence checks.

## State, seeds and repeatability

All evaluator, kernel and cache state is per `evaluate`, on a joined worker with
the validated `worker_stack_bytes`. Parse and parameter evaluation run under the
same stack policy. There is no detached timed-out worker, no process-global panic
hook, no persistent geometry cache and no shared random state.

`random_seed = Some(s)` seeds the unseeded `rands()` stream before evaluation, so
a program that calls `rands()` without its own seed repeats across calls,
concurrent callers and interleaved evaluations of other sources. Seeded
`rands(min, max, count, seed)` already reseeds itself. Unseeded programs are not
promised deterministic random output.

## No deadline

The adapter promises **no** in-process render deadline, and none of the limits
above is one. `evaluation_steps` bounds the evaluator's own work; native rendering
is synchronous and cannot be interrupted from inside this API.

A caller that needs a wall-clock guarantee must supervise a killable child
process, as `tests/scad_process_limits.rs` does. A thread timeout or
`catch_unwind` is not sufficient: a native C++ kernel overrun is not recoverable
in-process. A process timeout and an evaluator budget exhaustion are different
classifications and must not be reported as the same failure.

## Example: reading the result

```rust,ignore
let out = evaluate(&source, &options)?;

// Reproduction record: engine revision + patch digest, source digest, the raw
// override list and the seed in effect.
assert_eq!(out.provenance.engine.backend, "manifold_cpp");
assert_eq!(out.provenance.source_digest.len(), 64);

for diagnostic in &out.diagnostics {
    // On success these are information only; `scad_echo` lines live here too.
    eprintln!("{}: {}", diagnostic.code, diagnostic.message);
}

match &out.geometry {
    ScadGeometry::Contours(regions) => { /* 2D profile, CCW outer / CW holes */ }
    ScadGeometry::Mesh(mesh) => { /* millimetre f32 vertices, flat u32 indices */ }
    ScadGeometry::Empty => return Err("empty model"),
}
```
