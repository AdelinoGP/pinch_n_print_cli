# OpenRSCAD compatibility and diagnostic classification

This file is the **revision-bound** record for the vendored engine. It answers one
question: at this exact pin, which upstream outcomes may an exact consumer accept,
and which must it reject?

Nothing here generalises to other OpenRSCAD revisions. Bumping the pin without
re-deriving this table fails closed: unclassified diagnostic codes become
`UnknownDiagnostic` (see `crates/slicer-scad/src/diagnostics.rs`).

## Immutable identity

| Field | Value |
|---|---|
| Repository | `https://github.com/matthova/openrscad` |
| Upstream base revision | `1a45bd33f2886c32bda32fe6e8358a67ab03f5ef` |
| Upstream version | `0.14.2` |
| Vendored packages | `openrscad-syntax`, `openrscad-ir`, `openrscad-eval`, `openrscad-geom` |
| Applied patch set | `vendor/openrscad/patches/0001-eval-adapter-seams.patch`, `.../0002-geom-adapter-seams.patch` |
| `patch_digest` (ordered SHA-256) | recorded in `vendor/openrscad/PATCHES.md`, recomputed by `tests/scad_feature_contract.rs` |
| Native backend | `manifold_cpp` (C++ Manifold via `manifold-csg`) |

The adapter reports this identity as `ScadEngineInfo` from `engine_info()`.
Record it **before** running a dangerous fixture, so a retained failure can be
attributed to a specific engine.

## Source audit at the pin

Each row is a symbol resolved in the vendored tree (crate-qualified), not a link to
`main`. The consequence column is what the adapter must do about it.

| Location | Symbol | Consequence |
|---|---|---|
| `vendor/openrscad/crates/openrscad-syntax/src/lib.rs` | `parse` | Returns `Program` or `SyntaxError { message, span }`; the span is retained as a `Parse` diagnostic span. |
| `.../openrscad-eval/src/lib.rs` | `eval_program_with_budget_and_params` (patch 0001) | The only entry point combining ordered overrides, one finite fuel allowance and an explicit `RenderMode`. |
| same | `eval_const_expr_with_budget` (patch 0001) | Budgeted expression evaluation that returns the **remaining** allowance, so override expressions and the program share one pool. |
| same | `eval_program_impl` | Private in upstream; patch 0001 re-exports it through the combined public entry point instead of duplicating it. |
| same | `FileResolver::load`, `FileResolver::load_bytes` | Host-supplied; the adapter installs a deny-all resolver that records requests. |
| same | `Interp::eval_stmts_core` | Its single `Err(e) if isolate && e.recursion => break` is where a root-level statement is dropped; patch 0001 sets `EvalOutput::recursion_aborted` there. |
| same | `Interp::report_recursion` | Emits the once-per-callee recursion warning; patch 0001 codes it `recursion_aborted`. |
| same | `Interp::warn_overwritten` | The dead-assignment lint; patch 0001 codes it `assignment_overwritten`. |
| same | `EvalOutput::warnings`, `EvalOutput::echoes` | Two separate channels; echoes are *not* warnings and carry no severity. |
| same | `EvalError::recursion`, `EvalError::budget_exhausted` (patch 0001) | Structured failure kinds, so the adapter never matches English text to tell a budget exhaustion from a malformed expression. |
| same | `seed_random_state` (patch 0001) | Seeds the thread-local unseeded `rands()` stream; `RNG_STATE` is thread-local and would otherwise advance across calls. |
| `.../openrscad-geom/src/lib.rs` | `render_cached_diag` | The exact mesh path. Returns `(Mesh, RenderDiagnostics)`; the preview path is a different function and is never called. |
| same | `render_contours_cached_diag` (patch 0002) | Diagnostic-returning contour/projection/lowering path. Upstream's `render_contours_cached` collected and **discarded** both diagnostic vectors, so a 2D consumer could not tell whether its contours came from a clean traversal. |
| same | `RenderDiagnostics`, `GeomMessage` (patch 0002) | `warnings`/`errors` are coded messages; the split is upstream's UI hint, not an acceptance distinction. |
| same | `boolean_or_fallback` | Pushes a `boolean_fallback` error and returns un-combined operands as a non-empty mesh. |
| same | `roof` warning site | Pushes `roof_omitted` when a concave/holed profile loses geometry. |
| same | `MINKOWSKI_APPROX_WARNING` sites | Push `minkowski_approximation` for non-convex or over-large 3D operands. |
| same | `is_2d` | Uses `any` across children, so it cannot see a mixed 2D/3D result. The adapter classifies dimensionality itself. |
| `.../openrscad-geom/src/mesh.rs` | `Mesh` | `verts: Vec<[f64; 3]>`, `tris: Vec<[u32; 3]>`; both stay private to the adapter. |
| `.../openrscad-eval/src/text.rs` | bundled `fontdb` database, `register_system_fonts` | Bundled Liberation faces load lazily from embedded bytes; OS font discovery is an explicit call the adapter never makes. |

### Resource seams checked

`include`, `use` and `import` (mesh and 2D) route through `FileResolver::load`;
`surface` routes through `load`; `dxf_dim`/`dxf_cross` route through
`FileResolver::load_bytes`. All six were exercised through the public adapter path
with a readable file present at the exact requested path, and all six are denied.
Override expressions use the same resolver, so an override cannot reach a file the
program cannot: `-D x=dxf_dim(file="tempting.dxf")` is `ExternalResourceDenied`,
not a warning.

`text` resolves from the bundled Liberation faces in `crates/openrscad-eval/fonts`
and performs no filesystem access; `register_system_fonts` is never called.

Unimplemented language constructs do **not** become resource errors: they surface
as an engine warning with the engine's catch-all code, which the adapter rejects as
`UnknownDiagnostic`. That is deliberate — a scanner that flagged the *text* of an
unreached `import` would be a false positive, and a construct that silently
succeeds would be a false negative.

## Diagnostic classification table

The table is complete for this pin. `unknown` means "not classified here", and the
adapter rejects it.

### Evaluation channel (`openrscad_eval::Warning::code`)

| Code | Origin | Class | Accepted on success |
|---|---|---|---|
| `assignment_overwritten` | `Interp::warn_overwritten` | Information | yes |
| `recursion_aborted` | `Interp::report_recursion` | Partial | no |
| `roof_omitted` | roof warning site | Approximate | no |
| `minkowski_approximation` | Minkowski warning sites | Approximate | no |
| `other` (engine catch-all) | every other `Interp::warn` site | Unknown | no |

Echoes are a separate channel: each evaluated `echo()` line becomes an
`Information`-class diagnostic with adapter code `scad_echo`, stage `Evaluation`,
and no span.

### Render / contour channel (`openrscad_geom::GeomMessage::code`)

| Code | Origin | Class | Accepted on success |
|---|---|---|---|
| `boolean_fallback` | `boolean_or_fallback` | Degraded | no |
| `roof_omitted` | roof warning site | Approximate | no |
| `minkowski_approximation` | Minkowski warning sites | Approximate | no |
| `other` | any other geometry message | Unknown | no |

Both `warnings` and `errors` are classified by the same table. A warning that
describes skipped geometry is exactly as unacceptable as a recoverable error, and
`errors.is_empty()` is not an exactness predicate.

## Reproducers

Each row is a real SCAD source exercised through `evaluate`. "Adapter result" is the
`ScadErrorKind` the public API returns.

| Observed outcome | Source (abridged) | Adapter result |
|---|---|---|
| Overwritten assignment | `z = 1; z = 2; cube(z);` | success, `assignment_overwritten` information |
| Echo | `echo("hi"); cube(1);` | success, `scad_echo` information |
| Root recursion abort | `function cum(l,k) = (k==0) ? 0 : cum(l,k-1)+l[k-1]; cube(3); cube(cum([5],-1)); cube(7);` | `PartialOutput` |
| Concave/holed roof | `roof(){ difference(){ square(10); translate([3,3]) square(4); } }` | `ApproximateOutput` |
| Concave 3D Minkowski | `minkowski(){ linear_extrude(1) polygon([[0,0],[6,0],[6,2],[2,2],[2,6],[0,6]]); sphere(1); }` | `ApproximateOutput` |
| Boolean fallback | `union(){ cube(10); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,2]]); }` | `DegradedOutput` with `boolean_fallback` |
| Projected 3D child with a warning | `projection() minkowski(){ linear_extrude(1) polygon([[0,0],[6,0],[6,2],[2,2],[2,6],[0,6]]); sphere(1); }` | `ApproximateOutput` — the contour path reports its own diagnostics |
| Projected boolean fallback | `projection() union(){ cube(10); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,2]]); }` | `DegradedOutput` — the same degradation is visible on the contour path |
| Unclassified engine warning | `nonsense_module(); cube(1);` | `UnknownDiagnostic` |
| Undefined override | `-D x=not_defined` | `InvalidParameter` |
| Resource request from an override | `-D x=dxf_dim(file="tempting.dxf")` | `ExternalResourceDenied`, parameter `x` |
| Denied include with sibling geometry | `include <missing.scad>; cube(1);` | `ExternalResourceDenied` |
| Denied mesh import with sibling geometry | `import("model.stl"); cube(1);` | `ExternalResourceDenied` |
| Budget exhaustion in the program | `for (a=[0:100000]) for (b=[0:100000]) translate([a,b,0]) cube(1);` | `EvaluationBudgetExceeded` |
| Budget exhaustion in a parameter | `-D x=[for(i=[0:300000]) i]` | `EvaluationBudgetExceeded` with `parameter = Some("x")` |

`DegradedOutput` **is** reachable from source without an import: a `polyhedron`
whose faces do not form a closed manifold makes a boolean operand non-manifold, and
the engine then replaces the union with un-combined operands. Both the mesh and the
projected form are committed fixtures above. Injected unknown render codes remain
covered by the classifier unit tests.

## Known upstream limitations

From upstream `COMPAT.md`, retained as real limitations rather than suppressed
warnings:

- `roof()` on a concave or holed profile can omit geometry (reported, rejected).
- Concave-leaf 3D `minkowski()` can approximate (reported, rejected).

These are not claims that every SCAD construct is certified compatible. The
adapter's contract is that such outcomes are **rejected**, not that they never
occur.
