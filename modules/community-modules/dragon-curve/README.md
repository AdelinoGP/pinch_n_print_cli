# Dragon Curve Infill — a labeled example only

> **Do not add real community modules to this repository.**
>
> This module is committed here *solely* so the community-module pattern is
> visible in-tree: the directory layout, the manifest shape, the build script,
> and how a non-Rust guest reaches the host. It is an example, not a product,
> and it is the only one that will ever live here.
>
> Real community modules are authored **in your own fork, as a submodule**,
> pinned to a tag or commit — never merged into this tree. That rule is
> enforced socially (this banner, the note in `docs/`, and `CLAUDE.md`), not
> mechanically. If you are about to add a second module under
> `modules/community-modules/`, stop: that is the thing this file exists to
> prevent.

## What it does

Fills the sparse-infill polygon by **tiling it with Heighway dragon curves**
instead of straight rectilinear lines, and gives each dragon its own tool — so
a multi-tool machine prints the tiling structure in different filaments and you
can see where one dragon ends and the next begins.

It holds two claims:

- `claim:sparse-fill` — the contested fill role. Per-region resolution via
  `sparse_fill_holder` decides whether this module or another emits sparse
  infill for a given region.
- `claim:authored-coloring` — the capability *disclosure* half of ADR-0058's
  two-sided grant. The module emits `tool_index = Some(..)` unconditionally and
  never guards on the grant; the host strips the value at the marshal boundary
  when coloring is not granted for the region, or when the index is out of
  range. Dropping this claim from the manifest does not fail the load — it
  silently strips every tool index the module emits.

The second half of the grant is host-side config: the region's
`fill_authored_coloring` must list a fill-role claim this module holds.

## Authoring language: MoonBit

Per the packet 225a re-measurement recorded in
`docs/14_submodule_programming_languages.md`, MoonBit is the selected Dragon
Curve authoring language (Go is terminally not loadable; MoonBit,
AssemblyScript and C++ all passed, and MoonBit wins the locked priority order).

Because this is a foreign-language guest, there is no `#[slicer_module]` macro
and no `slicer-sdk`. Everything the SDK would give a Rust module is written out
by hand, which is exactly what makes this a useful example:

- the `should_emit` gate is reimplemented from `held-claims` (and is
  fail-closed: empty claims suppress all fill roles);
- per-region config override precedence is spelled out rather than inherited
  from `resolve_float`;
- WIT types are used through generated MoonBit bindings.

## Layout

```
dragon-curve.toml     module manifest (id, stage, claims, config schema)
wit/                  frozen snapshot of the Layer::Infill WIT closure
src/dragon/           pure tiling + color mapping. Imports nothing; unit-tested
src/glue/             WIT glue template, copied into the generated package
Makefile              bindings -> build -> componentize
dragon-curve.wasm     the committed component artifact
```

`src/dragon/` deliberately defines its own geometry structs rather than reusing
the generated `@geometry` types, because those carry `extern "wasm"` FFI and
would make the tiling logic untestable off-wasm.

## Build

Nothing rebuilds this automatically. It is **not** a Cargo workspace member and
**not** discovered by `cargo xtask build-guests`, whose walk is hard-coded to
`modules/core-modules` and `crates/slicer-wasm-host/test-guests`. That exclusion
is structural, not a special case.

```bash
cd modules/community-modules/dragon-curve
make            # bindings -> moon build -> wasm-tools embed + component new
make test       # unit tests for the pure tiling and color logic
```

Toolchain this was built and verified against:

| Tool | Version |
| --- | --- |
| `wit-bindgen-cli` | 0.60.0 |
| `moon` / `moonc` | 0.1.20260807 (4da23f8) / v0.10.7 |
| `wasm-tools` | 1.250.0 |

The build is **not bit-reproducible** — a second clean build hashes
differently. That is a recorded MoonBit property, not a defect here.

Two build gotchas, both of which previously looked like toolchain failures:

1. `run` is forward-declared (`declare pub fn run`) in the generated *interface*
   package, so its definition must be copied into
   `gen/interface/slicer/layer-infill/infill/`. Putting it in `gen/` produces a
   component that builds and then **traps on dispatch**.
2. `wasm-tools component embed` needs `--encoding utf16`: MoonBit strings are
   UTF-16 and the host transcodes at the canonical ABI boundary.

## Manual slice test

Not wired into CI — run it by hand. Note the flag is `--model`, not `--input`:

```bash
pnp_cli slice \
    --model resources/regression_wedge.stl \
    --module-dir modules/community-modules/dragon-curve \
    --config dragon-config.json \
    --output /tmp/dragon.gcode
```

where `dragon-config.json` routes sparse fill to this module and grants it
authored coloring:

```json
{
  "sparse_fill_holder": "com.example.dragon-curve",
  "fill_authored_coloring": ["claim:sparse-fill"],
  "filament_density": [1.24, 1.27, 1.21, 1.30],
  "infill_density": 0.6,
  "tiling_depth": 12,
  "color_map": [0.0, 1.0, 2.0, 3.0]
}
```

`filament_density` is **required to see any colour at all**: `derive_tool_count`
(`crates/slicer-wasm-host/src/host.rs`) sources the printer's tool count from
that key alone, and a config without it reports a single tool, so every dragon
maps to tool 0 and no tool change is ever emitted. Omitting it is the single
most likely reason this example appears not to work.

Confirm colouring landed:

```bash
rg -o '^T[0-9]+' /tmp/dragon.gcode | sort | uniq -c
```

On `resources/20mm_cube.obj` with the config above this yields **283 tool
changes across four tools** (94 x T0, 94 x T1, 48 x T2, 47 x T3) at both
`tiling_depth` 8 and 12 — depth changes dragon size, not colour count. A
core-modules-only control emits no `T` lines at all.

To confirm the module is discovered at all:

```bash
pnp_cli module diagnose --module-dir modules/community-modules/dragon-curve
```

should list `com.example.dragon-curve` with `"provenance": "external"`.

## Config

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `infill_density` | float | 0.2 | Mirrors rectilinear-infill. Drives line spacing. |
| `infill_angle` | float | 45.0 | Rotates the whole tiling about the region centre. |
| `infill_speed` | float | 60.0 | Mirrored; becomes the path speed factor. |
| `line_width` | float | 0.0 | Mirrored. Spacing is `line_width / infill_density`. `0` means *auto* and falls back to 0.45 mm. |
| `tiling_depth` | int | 10 | Fold depth of each dragon. Each has `2^depth` segments; clamped to 16. |
| `color_map` | float-list | `[0.0, 1.0, 2.0, 3.0]` | Tool-index sequence indexed by **dragon instance**, wrapped into `[0, tool_count)`. Empty disables coloring. |
| `filament_density` | float-list | `[1.24]` | Not read by this module, but must be declared: it is the host's sole tool-count source. |

`infill_density`, `line_width` and `tiling_depth` are overridable per region.

`color_map` is declared `float-list` because that is the only list type with an
in-tree precedent (`wipe-tower.toml`); `int-list` is absent from
`slicer_schema::VALID_CONFIG_TYPES`. Values round to the nearest non-negative
integer.

### Choosing `tiling_depth` — it controls dragon *size*

Each dragon has `2^tiling_depth` segments, so the value sets how large each
dragon is and therefore how much of the region one dragon spans. It is
honoured as given: there is no region-size cap (an earlier version had one to
hide coverage defects; the tiling is now exact, so it was removed).

Low depth gives many small dragons; high depth gives a few large ones whose
fractal outline is clearly visible in the finished part. Coverage is unaffected
either way.

## The tiling

The design spec prescribes determinism, the wrap-into-tool-count rule and
sparse-only emission, but **no construction algorithm** — so this one is the
module's own and is documented here rather than inherited:

1. Build the Heighway dragon polyline: `2^tiling_depth` segments of length
   `line_width / infill_density`, using the standard turn rule (the `i`-th turn
   is left when `((i & -i) << 1) & i` is zero). Rotate it by `infill_angle`.
2. **Tile the region with FOUR of them.** At each point of the lattice
   generated by `e(1+i)` and `e(1-i)` — where `e` is the curve's end-vector,
   `(1+i)^n` — place four dragons rotated 90 degrees apart about that point.

   The four-fold rotation is what makes this an exact tiling. Translation alone
   cannot: a lattice cell holds `2 x 2^n` unit grid edges while one dragon
   supplies `2^n`, so translated copies cover at most half the edges and leave
   fractal voids. Measured, the four-fold arrangement covers the grid exactly
   once — 0 empty sample cells at every depth.
3. Clip every segment of every instance to the contour minus holes.
4. Color each dragon by its **rotation index** — one dragon, one tool — so the
   four interlocking dragons print in four filaments and the tiling reads
   directly off the part. Use a `color_map` of four entries to see all four; a
   shorter map wraps and some dragons share a tool.

5. **Correct the density.** Clipping and fractal lobes make the first pass
   fall short, so the module measures what it emitted against
   `area / line_spacing` and regenerates once at a proportionally finer spacing
   if needed. This is driven by the region's own measured output, so it does
   not overshoot where the first pass already hits the target.

Measured on a 60 mm cube at 60% density, whole print, against rectilinear at
the same setting: **99.9%** of rectilinear's sparse path. On a bare 60 mm
square the tiling leaves **0 of 400** sample cells empty, at every depth
tested (8, 10, 12).

Two guards protect this — one on total length, one on void fraction — because
a length check provably cannot see fill that has merely been redistributed
into overlaps. An early version scored a perfect 1.00 length ratio while
leaving 17% of the region empty.

The 60 mm figures come from a generated, gitignored fixture
(`target/scratch/cube60.stl`) and cannot be re-derived from a clean checkout;
`resources/20mm_cube.obj` is the reproducible in-tree evidence.

One limit worth knowing:


- **Four rotations, so four tools.** A `color_map` shorter than four wraps, and
  some dragons then share a tool. `[0.0, 1.0, 2.0, 3.0]` with four filaments
  renders the full tiling.

Determinism is the core invariant: no RNG, no clock, no hash-map iteration, and
a hand-rolled sort so ordering cannot drift with a toolchain update.
