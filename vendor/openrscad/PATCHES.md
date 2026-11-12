# Vendored OpenRSCAD — provenance, patches and regeneration

This directory is a **vendored, patched subset** of
[`matthova/openrscad`](https://github.com/matthova/openrscad). The SCAD adapter
(`crates/slicer-scad`) depends on it by path. Nothing here is a Pinch 'n Print
workspace member: `Cargo.toml` carries a `[workspace]` table so the tree is its
own workspace.

## Identity

| Field | Value |
|---|---|
| Upstream repository | `https://github.com/matthova/openrscad` |
| Upstream base revision | `1a45bd33f2886c32bda32fe6e8358a67ab03f5ef` |
| Upstream version | `0.14.2` |
| Upstream declared MSRV | `1.85` |
| Vendored packages | `openrscad-syntax`, `openrscad-ir`, `openrscad-eval`, `openrscad-geom` |
| Deliberately absent | upstream `openrscad-cli`, `openrscad-lsp`, `openrscad-wasm`, `xtask` |
| License | Apache-2.0 OR MIT (see `LICENSE-APACHE`, `LICENSE-MIT`) |

Only the four engine packages the adapter consumes are present. Upstream files
are otherwise unmodified: no reformatting, no root-workspace lint adoption
(`vendor/openrscad/crates/*` is not a Pinch 'n Print workspace member, so
`cargo clippy -D warnings` applies to this repo's crates only).

`Cargo.lock` in this directory is the actual resolution recorded when the patch
set below was validated on Windows; commit it with the tree.

## Applied patch set (ordered)

Apply in this order from the directory that contains `crates/`.

| # | File | Effect |
|---|---|---|
| 0001 | `patches/0001-eval-adapter-seams.patch` | Adds `eval_const_expr_with_budget` (returns the **remaining** allowance) and `eval_const_expr_with_budget_and_resolver` (returns a `ConstExprOutcome` with the expression's own warnings and echoes, evaluated against the caller's resolver), makes a parameter expression that injects a second statement a parse error, adds the combined `eval_program_with_budget_and_params` entrypoint, tags `Warning` with a stable `code`, adds `EvalError::budget_exhausted`, adds `seed_random_state`, and surfaces root-level recursion aborts as `EvalOutput::recursion_aborted`. |
| 0002 | `patches/0002-geom-adapter-seams.patch` | Replaces the message-only `RenderDiagnostics` vectors with `GeomMessage { code, message }`, adds `render_contours_cached_diag` (the diagnostic-returning contour/projection/lowering entrypoint), and adds `Mesh::welded_exact` so the Manifold boundary normalises coincident vertices by exact identity instead of the `welded(1e-7)` epsilon bucket that silently merges distinct points and drops the collapsed triangles. |

### Digest

`patch_digest` is the lowercase SHA-256 of the **ordered concatenation** of the
patch files above, in filename order, exactly as committed (bytes, no
normalization):

```
c029707967be77f41f5bab502f0a1b1bbad24990c02fa91096a203548baa4024
```

The adapter exposes this value as `ScadEngineInfo::patch_digest`, and
`crates/slicer-scad/tests/scad_feature_contract.rs` recomputes it from these
files so an edit that forgets to re-derive the constant fails.

## Why each patch exists

Both patches exist because the upstream public API cannot express a contract
the adapter is required to meet. Each is additive: no upstream function was
removed, and the pre-existing entrypoints now delegate to the new ones, so
behaviour without the new calls is unchanged.

### 0001 — evaluation

- **Override expressions observe the same resource and diagnostic boundary as the
  program.** `eval_const_expr` built its own `NullResolver` and discarded the
  expression's warnings and echoes, so an override could reach a file the program
  could not (`-D x=dxf_dim(file="…")`) and an approximate expression's warning
  disappeared. `eval_const_expr_with_budget_and_resolver` takes the host resolver
  and returns a `ConstExprOutcome { value, remaining, warnings, echoes }`.


- **Combined budget + overrides + mode.** Upstream exposed `overrides`
  (`eval_program_with_params`/`eval_program_with_mode`) and `fuel`
  (`eval_program_with_budget`) through separate public functions, and kept the
  combined `eval_program_impl` private. A host needing both had to pick one:
  either it lost the termination guarantee on adversarial input, or it lost
  `-D` semantics. The adapter must have both, so the existing private
  implementation is re-exported rather than duplicated.
- **Budgeted expression evaluation with a returned remainder.**
  `eval_const_expr` ran with `fuel: u64::MAX`. That is a hang risk for a
  parameter expression (`-D 'x=[for (i=[0:1e9]) i]'`), and a per-expression
  budget reset would let N overrides buy N allowances. Returning the remainder
  makes one finite allowance provably span override expressions and the
  program.
- **Injection rejection.** `eval_const_expr` wraps its argument as
  `__orscad_const = (<src>);` and then pulls the first matching assignment back
  out. Trailing statements are discarded, but a caller-supplied
  `__orscad_const = (...)` inside `<src>` would silently change *which*
  expression is evaluated. Requiring the wrapped program to contain exactly one
  statement turns that into a parse error.
- **Structured warning codes.** `Warning` carried only `message`, so a host able
  to fail closed on unclassified diagnostics had to substring-match English
  prose. `code` is a stable classification; unclassified sites default to
  `"other"`, which the adapter rejects.
- **Structured partial flag.** A root-level recursion abort keeps the geometry
  produced before it and returns `Ok`, with only a prose warning. A non-empty
  mesh therefore cannot distinguish exact output from partial output unless the
  abort is structured. `EvalOutput::recursion_aborted` is set at the single
  `break` in `Interp::eval_stmts_core`.

### 0002 — geometry

- **Emptiness-aware 2D dispatch.** `is_2d` treated any `Projection`/`Offset` as 2D,
  so a container holding an *empty* one was routed to the 2D path, which drops 3D
  children: measured, `union(){ projection(){}; cube(1); }` returned an empty mesh.
  `is_empty_tree` now distinguishes a wrapper around nothing, and `is_2d` consults it.
- **Exact welding at the kernel boundary.** `to_manifold` used `Mesh::welded(1e-7)`,
  which buckets positions on a 1e-7 mm grid and drops every triangle that
  collapsed. That is a silent geometry deletion at the boundary the adapter calls
  exact: measured, `union(){ cube([1e-8,1,1]); translate([2,0,0]) cube(1); }`
  returned only the second body with no diagnostic. `Mesh::welded_exact` merges
  only bit-identical positions (signed zero canonicalised) and drops only
  triangles that are zero-area by identity, which still gives the kernel the shared
  edges that revolution seams need.


- **Coded render diagnostics.** `RenderDiagnostics { warnings: Vec<String>,
  errors: Vec<String> }` gave a host English text only. `GeomMessage` carries
  the same text plus a stable code, so the adapter can reject unknown codes
  instead of guessing whether a warning is benign.
- **Diagnostic-returning contour entrypoint.** `render_contours_cached`
  collected the contour/projection/lowering warnings and errors into local
  vectors and then dropped them, so a 2D consumer could not tell whether its
  contours came from a clean traversal. `render_contours_cached_diag` is the same
  traversal with the diagnostics attached — deliberately not a second render,
  because re-rendering the tree through the 3D path would not prove the 2D path
  was clean.

## Regenerating the vendored tree

From a clean checkout of the base revision:

```bash
git clone https://github.com/matthova/openrscad /tmp/openrscad
git -C /tmp/openrscad checkout 1a45bd33f2886c32bda32fe6e8358a67ab03f5ef
mkdir -p vendor/openrscad/crates
cp -r /tmp/openrscad/crates/openrscad-{syntax,ir,eval,geom} vendor/openrscad/crates/
cp /tmp/openrscad/{LICENSE-APACHE,LICENSE-MIT,COMPAT.md,README.md} vendor/openrscad/
# restore this directory's Cargo.toml (the trimmed 4-member workspace), then:
patch -p1 -d vendor/openrscad < vendor/openrscad/patches/0001-eval-adapter-seams.patch
patch -p1 -d vendor/openrscad < vendor/openrscad/patches/0002-geom-adapter-seams.patch
```

Do **not** bump the base revision without re-running the adapter contract tests
and re-verifying every diagnostic code in
`docs/scad/openrscad-compatibility.md`: the classification table is bound to
this revision, not to OpenRSCAD in general.
