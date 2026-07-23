# Pinch 'n Print — WIT Interfaces & Module Manifest Schema

**What this covers:** the WIT contract between host and modules — how the `.wit`
files are organized, how host and guest each consume them, what the host
enforces at the boundary, and the TOML manifest schema a module ships.

**Who it's for:** module authors writing a manifest or binding to a world, and
anyone changing the WIT contract.

**Prerequisites:** `00_project_overview.md` for crate layout;
`02_ir_schemas.md` for the IR types the WIT surface marshals. WIT ("WebAssembly
Interface Types") is the interface-definition language the Component Model uses
to describe what a module imports and exports.

> **Source of truth.** `crates/slicer-schema/wit/` is the **single canonical WIT contract**.
> It is consumed directly by both the host (`wasmtime::component::bindgen!{ path: … }`) and the
> guest proc-macro (`crates/slicer-macros` via `include_str!` + nested-package inline). The WIT
> code blocks reproduced in this document are derived for reading convenience and may drift behind
> the on-disk schema (e.g. new record fields, additional resource methods, renamed enum variants).
> When the doc and `crates/slicer-schema/wit/` disagree, the on-disk files win; treat the doc
> divergence as a bug to be filed against this document. The phantom top-level `wit/` directory
> was deleted in packet 72; do not recreate it.
>
> Likewise, the TOML manifest schema in this document is the parsed surface
> recognised by `crates/slicer-scheduler/src/manifest.rs` (re-exported as
> `slicer_runtime::manifest`). Sections or keys that appear here but are not
> read by the parser are noted inline with a `<!-- VERIFY: ... -->` tag.

---

## WIT File Organization

The canonical source lives under `crates/slicer-schema/wit/` in an umbrella layout where
`root.wit` is the anchor package and `deps/` holds all shared dep packages plus the four
world packages (each in its own subdirectory so `wasmtime` can load them via `push_path`):

```text
crates/slicer-schema/wit/
  root.wit                                   # package slicer:root@1.0.0 (anchor)
  deps/
    types.wit          # package slicer:types       — interface geometry
    config.wit         # package slicer:config      — interface config-types
    ir-types.wit       # package slicer:ir-handles  — interface ir-handles
    common.wit         # package slicer:common      — interface module-errors
    world-layer/world-layer.wit           # package slicer:world-layer@2.1.0
    world-prepass/world-prepass.wit       # package slicer:world-prepass@1.0.0
    world-postpass/world-postpass.wit     # package slicer:world-postpass@1.0.0
    world-finalization/world-finalization.wit  # package slicer:world-finalization@1.0.0
```

**Host** consumption (`crates/slicer-wasm-host/src/host.rs`, which holds all
four `bindgen!` invocations so they share Rust type identity — see ADR-0002):
```rust
wasmtime::component::bindgen!{
    path: "../slicer-schema/wit",
    world: "slicer:world-layer/layer-module",
    with: { "slicer:config/config-types.config-view" => crate::… }
}
```
One call per world; no inline WIT.

**Guest** consumption (`crates/slicer-macros/src/lib.rs`): the `#[slicer_module]` proc-macro
reads dep files via `include_str!`, wraps each `package x;` in nested-package braces, concatenates
with the world file, and passes the result to `wit_bindgen::generate!{ inline: … }`. Both sides
parse the same bytes from `deps/*.wit` — identity agreement is structural, not by convention.

### Nested-package directory layout (Normative — Packet 72)

Each world package is in its own subdirectory under `deps/`
(`deps/world-X/world-X.wit`) because `wasmtime`'s `push_path` /
`push_dir` resolution requires **one main package per directory**.
The umbrella structure (`root.wit` at top + `deps/`) lets host
`bindgen!{ path: ... }` and guest `include_str!` resolve cross-package
`use` statements natively without inline copies.

Dead-code retirement (Packet 72): the unused `gcode-output-interface`
was deleted during the single-source unification. It does NOT appear
in the canonical `crates/slicer-schema/wit/` and must not be
re-introduced — emit `gcode-output-builder` operations through the
`world-layer`/`world-postpass` resources instead.

Three rules govern all WIT design decisions:

1. The host never trusts module declarations at runtime. Access control is enforced per-call.
2. Modules never receive more data than declared in their manifest `ir-access.reads`.
3. WIT versions are independent from IR schema versions.

## Host-Boundary Access Enforcement (Normative)

Access control is enforced at the host WIT boundary, not only in SDK helpers.
Modules that bypass SDK wrappers must still be constrained identically.

| Operation class               | Manifest declaration required                                   | Host behavior when undeclared                                                 |
|-------------------------------|-----------------------------------------------------------------|-------------------------------------------------------------------------------|
| Read structured IR view field | `[ir-access].reads` path                                        | Return fatal contract error (no implicit empty fallback for undeclared paths) |
| Read paint semantic regions   | `[ir-access].reads` includes semantic-specific path             | Return fatal contract error                                                   |
| Read custom paint regions     | `[ir-access].reads` includes the `SliceIR` paint path carrying that semantic | Return fatal contract error                                                   |
| Write structured IR field     | `[ir-access].writes` path                                       | Reject commit with fatal contract error                                       |
| Write via output builders     | `[ir-access].writes` path mapped to builder operation           | Reject operation and keep pre-stage IR                                        |

Required diagnostics on undeclared access:

- module id
- stage id
- attempted operation (`read|write`)
- requested path/semantic
- manifest path set used for comparison

Determinism rule:

- Access-denied outcomes are deterministic and must not depend on invocation order or thread scheduling.

## Concurrency & Instance Isolation (Normative)

`layer-parallel-safe` contract:

- `true`: host may execute the module concurrently across multiple layers/regions by using an instance pool.
- `false`: host must serialize invocations for the module.

Runtime model:

- Each WASM instance owns isolated linear memory.
- No module may rely on cross-instance mutable shared state for correctness.
- Host-owned IR/resources are the only allowed communication channel.

Host validation requirements:

- If manifest sets `layer-parallel-safe = true` and component imports/declares shared WASM memory, host rejects module load as fatal.
- For `PostPass::LayerFinalization`, effective parallel safety is always treated as `false` regardless of manifest hint.
- Host emits startup diagnostics showing final pool mode (`parallel` or `serialized`) per module.

## WIT ↔ IR Compatibility Matrix (Normative)

WIT world identity and IR schema versions are independent. Module load is allowed
only when both checks below pass.

| Host WIT world       | Module `wit-world`         | Host IR schema | Module IR range   | Load result                              |
|----------------------|----------------------------|----------------|-------------------|------------------------------------------|
| `slicer:world-layer` | `slicer:world-layer`       | `1.4.0`        | `>=1.2.0, <2.0.0` | Allowed                                  |
| `slicer:world-layer` | `slicer:world-layer@1.0.0` | any            | any               | Rejected (`wit-world` must carry no version) |
| `slicer:world-layer` | `slicer:layer-world`       | any            | any               | Rejected (unknown world)                 |
| `slicer:world-layer` | `slicer:world-layer`       | `2.0.0`        | `>=1.2.0, <2.0.0` | Rejected (IR major out of range)         |

Startup checks:

1. Validate `wit-world` names a known world. The name is **unversioned**; a
   declared version is rejected with a diagnostic naming the corrected value.
2. Validate host IR schema is within module-declared IR range.
3. Emit explicit diagnostics with expected/actual versions and blocking symbol
   names when incompatible.

### Why `wit-world` carries no version

Earlier revisions of this section specified matching on "package name and major
version". **No such comparison ever existed**, and it could not have worked:

- The check was `ALLOWLIST.contains(&wit_world)` — exact string equality. Bumping
  a world therefore rejected *every* module until all 23 manifests were rewritten
  in lockstep, which is the opposite of the additive compatibility this section
  claimed.
- More fundamentally, **the world version is erased from the guest binary at
  compile time.** Our worlds export bare freestanding funcs, and a bare extern
  name carries no semver suffix (component-model `WIT.md`: `<semversuffix>` is a
  production of `<interfacename>`, not of a plain name). `wasm-tools component wit
  <guest>.wasm` finds no `world-layer` and no `@x.y.z` anywhere in it.

So a versioned `wit-world` was an **unfalsifiable claim**: there is no fact in the
system to check it against, which is precisely what rule 1 above ("the host never
trusts module declarations") forbids. It was removed rather than left as ceremony
that cost ~79 files per bump and enforced nothing.

What actually enforces compatibility today:

| Guard | Catches |
|---|---|
| wasmtime typed instantiation (`crates/slicer-wasm-host/src/dispatch.rs`) | Structural export/signature mismatch, at first dispatch |
| `cargo xtask build-guests --check` | Stale in-tree guest (mtime-based) |
| `[compatibility]` min/max-ir-schema (`crates/slicer-scheduler/src/validation.rs`) | IR range, fatal at startup |

The world version now lives solely in the `package` line of
`crates/slicer-schema/wit/deps/world-*/*.wit`, where it selects which package
`bindgen!`/`generate!` resolve at build time. It is a changelog annotation, not an
identity token. Giving it real, mechanical enforcement requires restructuring each
stage into its own **versioned interface** (`slicer:world-layer/infill-postprocess@2.0.0`),
so the version lands in the component's export names where wasmtime's semver
matching can act on it. That reasoning is recorded in
`adr/0044-wit-world-version-is-not-an-identity-token.md`, and the decision to
restructure is `adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md`.

---

## `deps/types.wit`

**Source of truth:** `crates/slicer-schema/wit/deps/types.wit` (package
`slicer:types`, interface `geometry`). Read that file for the exact record and
variant definitions; the notes below are the contract points that the file
alone does not spell out.

The `geometry` interface defines the shared geometric primitives: `point2`
(scaled `s64` integer coordinates — 1 unit = 100 nm), `point3` /
`point3-with-width` (millimeter `f32`), `bounding-box2` / `bounding-box3`,
`polygon`, `ex-polygon`, `extrusion-path3d`, the `extrusion-role` variant, and
`semver`.

`point3-with-width` carries an `overhang-quartile: option<u8>` field (1..=4 for
wall-family roles only; `none` otherwise), added in packet 57. The bindgen
`with:` remap also accepts the legacy 5-field shape so older host builds still
load.

The WIT `extrusion-role` variant is **narrower than the Rust `ExtrusionRole`
enum** (`02_ir_schemas.md`). Roles with no dedicated WIT case round-trip
losslessly through reserved `custom(string)` tags:

- `PrimeTower` maps to `custom("slicer.builtin/prime-tower@1")`
- `Skirt` maps to `custom("slicer.builtin/skirt@1")`
- `Brim` maps to `custom("slicer.builtin/brim@1")`
- Third-party modules must not mint any reserved `slicer.builtin/…` tag.

The tag constants and marshalling live in `crates/slicer-macros/src/lib.rs` and
`crates/slicer-wasm-host/src/host.rs`.

---

## `deps/config.wit`

**Source of truth:** `crates/slicer-schema/wit/deps/config.wit` (package
`slicer:config`, interface `config-types`).

Two items:

- `config-value` — the variant every config value marshals as (bool, int,
  float, string, list, and percent forms). Read the file for the current case
  set.
- `config-view` — a **read-only** resource, pre-filtered to the module's
  declared reads only. Its `get` / `get-bool` / `get-float` / `get-int` /
  `get-string` accessors each return `option<…>`, and `keys()` lists the
  visible keys. A module can never see a key it did not declare (see
  "Host-Boundary Access Enforcement" above).

---

## `deps/ir-types.wit`

**Source of truth:** `crates/slicer-schema/wit/deps/ir-types.wit` (package
`slicer:ir-handles`, interface `ir-handles`). This is the largest interface in
the contract — the read-view resources (`slice-region-view`,
`perimeter-region-view`, `paint-region-layer-view`), the output-builder
resources (`infill-output-builder`, `perimeter-output-builder`,
`slice-postprocess-builder`, `gcode-output-builder`, `support-output-builder`,
`layer-collection-builder`), the paint types (`paint-semantic`, `paint-value`,
`segment-annotations-*`), the wall types (`wall-loop-view`, `wall-feature-flag`,
`wall-boundary-type`, `material-boundary-segment`), and the Arachne types
(`extrusion-junction`, `extrusion-line`). It changes often; read the on-disk
file for the current method and field sets rather than relying on a snapshot
here.

Notable records/methods worth surfacing (not obvious from the resource names):
- `slice-region-view` exposes `surface-group: func() -> option<surface-group>`,
  returning the read-side `record surface-group` (`type surface-group-id = u64`;
  fields `id`, `facet-indices`, `z-min`, `z-max`, `area-mm2`, `printable`,
  `shell-count`) — distinct from the smaller write-side `surface-group-proposal`
  (PrePass). Added packet 104.
- `slice-region-view` and `perimeter-region-view` expose
  `config: func() -> config-view`, providing a per-region config accessor for
  resolved settings inside each region loop. Packet 131 bumps `world-layer`
  from 2.0.0 to 2.1.0 for this additive contract change.
- `perimeter-output-builder` and `infill-output-builder` both carry
  `set-current-origin: func(object-id: string, region-id: string) -> result<_, string>`,
  which tags the region currently being iterated so buffered per-region pushes are
  attributed correctly (packet 127, ADR-0022; see the `begin_region` SDK method).

Two contract points that the file alone does not state are the ID
canonicalization rule and the wall-loop flag invariant below.

### ID canonicalization

- `region-id` string must carry canonical decimal `u64` representation from host IR.
- Any non-canonical `region-id` observed at a module boundary is a fatal contract error.
- Modules must treat IDs as opaque tokens and must not derive ordering from lexical string comparison.

### Wall Loop Flag Invariant

- `wall-loop-view.feature-flags` is required and must remain parallel to `path.points`.
- For segment-level behavior, segment `i -> i+1` reads `feature-flags[i]`.
- Host-side debug validation should enforce `feature-flags.len() == path.points.len()` at module boundaries.
- A module that changes path cardinality must also update `feature-flags` cardinality in the same write.

---

## `host-api.wit`

There is **no `host-api.wit` file.** The host-service functions a module
imports live in `crates/slicer-schema/wit/deps/common.wit`, package
`slicer:common`, split across two interfaces:

- `host-services` — logging (`log`), mesh queries that keep mesh data host-side
  (`raycast-z-down`, `surface-normal-at`, `object-bounds`), host-side Clipper2
  geometry ops (`clip-polygons`, `offset-polygons`, `simplify-polygon`), the
  host-only-algorithm bridges (`medial-axis`, `generate-arachne-walls`), and
  `now-us`. Modules import it as `slicer:common/host-services`.
- `module-errors` — the shared `module-error` record. Every world imports it as
  `slicer:common/module-errors.{module-error}` rather than redefining it.

The host-only-algorithm bridges exist because a guest cannot link `host-algos`
code (rayon + boostvoronoi are `cfg(not(target_arch = "wasm32"))` only): the
function runs host-side and only the result crosses the WASM boundary.
`generate-arachne-walls` returns a `(toolpaths, inner-contour)` pair, not a
single list. Read `common.wit` for the exact signatures.

### `arachne-params` record

`generate-arachne-walls` takes an `arachne-params` record (defined in the
`common.wit` `host-services` interface, mirroring
`slicer_core::arachne::pipeline::ArachneParams` field-for-field, packet 112
Step 9A). Every distance/width field is in millimeters. `wall-sequence` is the
three-state WIT enum `inner-outer`, `outer-inner`, or `inner-outer-inner`.
The perimeter module resolves the existing `wall_sequence` config and the
SDK/WASM host transports that resolved enum unchanged; the host algorithm does
not re-read module config. The perimeter module owns final `WallLoop` order,
including finalized Arachne region ordering, and path optimization preserves
that committed wall subsequence while optimizing permitted travel. This
boundary documentation records the implementation contract; packet closure
remains subject to the packet's final acceptance ceremony.

The three layer-position bool fields are **G10 plumbing; set by the module
from region top/bottom metadata**:

- `is-initial-layer: bool` — true when the region is on the first printed
  layer (`layer-index() == 0`).
- `is-bottom-layer: bool` — true when the region is the bottom of a shell
  (derived from `SliceRegionView` bottom metadata).
- `is-topmost-layer: bool` — true when the region is the topmost solid shell
  layer (derived from `SliceRegionView` top metadata; G10). Together with
  `is-bottom-layer` it lets `remove_small_lines` express OrcaSlicer's
  `is_top_or_bottom_layer` lenient-threshold condition instead of keying only
  on `is-initial-layer`.

---

## `world-layer.wit`

**Source of truth:** `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`
(package `slicer:world-layer@2.2.0` — packet 137 bump for the
`lightning-tree-segments` read-view). The `layer-module` world imports
`slicer:common/host-services`, `slicer:config/config-types.{config-view}`, and
the views/builders it needs from `slicer:ir-handles/ir-handles`, and imports the
shared `module-error` from `slicer:common/module-errors`.

It has two optional lifecycle exports (`on-print-start`, `on-print-end`) and
eight stage exports — a module implements exactly the one matching its declared
manifest stage, and the host rejects a module whose export set mismatches its
stage:

- `run-slice-postprocess`, `run-perimeters`, `run-wall-postprocess`,
  `run-infill`, `run-infill-postprocess`, `run-support`,
  `run-support-postprocess`, `run-path-optimization`.

Read the on-disk file for each export's exact parameter list and return type.
Notable 2.0.0 (packet 130) change: `run-infill-postprocess` gains a read-only
`prior-infill` view of the committed `InfillIR`, and its commit is REPLACE — the
module must re-emit every path it wants kept.

### `layer-collection-builder` resource (packet 32)

Available to `Layer::PathOptimization` modules. Replaces the previous reserved-future placeholder.

**Source of truth:** `crates/slicer-schema/wit/deps/ir-types.wit`. The resource and the
`ordered-entity-view` record are defined there.

```wit
resource layer-collection-builder {
    // Returns `result<_, string>` so commit-time validation surfaces as an error.
    set-entity-order:     func(items: list<tuple<u32, bool>>) -> result<_, string>;
    get-ordered-entities: func() -> list<ordered-entity-view>;
}

// Confirm the exact field set against the on-disk WIT; the record currently
// carries fields such as `original-index`, `tool-index`, `region-key`, `role`,
// `start-point`, `end-point`, and `point-count` (the older `entity-index` field
// was renamed to `original-index`).
```

`ordered-entity-view.tool-index: u32` is the first-class tool selector from the
region_id↔tool split. The `path-optimization` guest reads it (via SDK
`OrderedEntityView.tool_index`) to cluster entities by tool — **not**
`region-key.region-id`, which is now a pure region identity.

`set-entity-order` accepts `(entity-index, reverse-direction)` tuples. Setting `reverse-direction = true` flips the path's point order at apply time. Host rejects entries that reference unknown `entity-index` values or include duplicates; either condition produces a `BuilderError::InvalidEntityOrder` diagnostic.

PathOptimization output contract restricts builder usage to this resource and the existing `push-tool-change` / `push-comment` / `push-raw` methods. `push-move` / `push-retract` / `push-unretract` / `push-fan-speed` / `push-temperature` remain rejected at the host boundary (see Path Optimization Output Contract below).

### `lightning-tree-segments` read-view (packet 137)

Available to `Layer::Infill` modules on the `paint-region-layer-view` resource.
Mirrors the `support-plan-segments` read-view shape (same
`list<list<point3-with-width>>` return type, same `object-id`/`region-id`
parameters) so a `Layer::Infill` module reaches the committed
`LightningTreeIR` for the dispatching `(object, region, layer)` triple via
the same idiom it would use for `SupportPlanIR` lookup in `Layer::Support`.

**Per-region dispatch keying:** The host dispatch stores `lightning_tree_segments`
in a `HashMap` keyed by `(object_id, region_id)`, not `(object_id, "*")`. This
follows the per-region keying precedent established by the `support-plan-segments`
read-view.

**Source of truth:** `crates/slicer-schema/wit/deps/ir-types.wit` (the
`paint-region-layer-view` resource method) and
`crates/slicer-schema/wit/deps/world-layer/world-layer.wit` (package
`slicer:world-layer@2.2.0`). The contract is frozen at packet 137 close;
any change to the signature in 138/139 is a WIT version bump, not a
silent change.

```wit
resource paint-region-layer-view {
    // ... existing get-regions / get-custom-regions / layer-index /
    //     support-plan-segments methods ...
    lightning-tree-segments: func(object-id: object-id, region-id: region-id)
        -> list<list<point3-with-width>>;
}
```

**Skip promise (ADR-0029):** The host commits a `LightningTreeIR` only when
the print's `sparse_fill_holder` is `lightning-infill`. Otherwise the slot
stays `None` and the method returns an empty `Vec<list<point3-with-width>>`
— the per-layer `Layer::Infill` module (packet 140) falls back to its
non-lightning path. Non-lightning prints therefore see a zero-cost, no-op
view; the wedge byte-identity canary (`wedge_per_region_config_delivery_byte_identical`)
pins the default-config slice through the new stage.

**SDK accessor:** `PaintRegionLayerView::lightning_tree_segments_for(object_id, region_id)`
in `crates/slicer-sdk/src/traits.rs` returns the same
`Vec<[slicer_ir::Point2; 2]>` shape as the IR's `tree_edge_segments` field
(2-point integer-unit compact storage per ADR-0029's memory note).

---

## `world-prepass.wit`

**Source of truth:** `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`
(package `slicer:world-prepass@1.0.0`). The `prepass-module` world imports
`slicer:common/host-services`, `slicer:config/config-types.{config-view}`, and
the shared `module-error` from `slicer:common/module-errors`.

It has exactly **four** stage exports:

- `run-mesh-analysis` — facet classification and surface-group proposals.
- `run-layer-planning` — per-layer Z and active-region proposals.
- `run-seam-planning` — scored seam candidates per `(layer, object, region)`.
- `run-support-geometry` — multi-layer organic tree-support branch geometry,
  consumed by `Layer::Support` modules that declare `SupportPlanIR` as a read.

There is **no `run-paint-segmentation` export.** Paint segmentation runs as the
`host:paint_segmentation` built-in (see `PrePass::PaintSegmentation` in
`01_system_architecture.md`), not as a module-implementable WIT stage. Read the
on-disk file for each export's parameters, the view records they consume, and
the output-builder resources they write through.

---

## `world-postpass.wit`

**Source of truth:** `crates/slicer-schema/wit/deps/world-postpass/world-postpass.wit`
(package `slicer:world-postpass@1.0.0`). The `postpass-module` world defines the
`gcode-command` variant (move / retract / unretract / fan-speed / temperature /
tool-change / comment / raw) and its command records locally, imports
`config-view` and `gcode-output-builder`, and uses the shared `module-error`
from `slicer:common/module-errors`.

Two exports:

- `run-gcode-postprocess(commands, output, config)` — processes the
  `gcode-command` stream.
- `run-text-postprocess(gcode-text, config) -> string` — last-resort text
  mutation, single-threaded; use only when `GCodeIR` is insufficient.

The retract command carries a `retract-mode` (G1 E vs G10/G11); read the
on-disk file for the exact record fields.

---

## `world-finalization.wit`

**Source of truth:** `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit`. The shape below summarises
the world; for exact field order, parameter names, and return types, read the
on-disk file.

The `finalization-module` world exposes a single export
`run-finalization(layers, output, config) -> result<_, module-error>`. It
imports `slicer:common/host-services` and `slicer:config/config-types`, uses
`slicer:config/config-types.{config-view}`,
`slicer:types/geometry.{extrusion-path3d, extrusion-role}`, and
`slicer:common/module-errors.{module-error}`. It declares `layer-idx`,
`object-id`, `region-id`, and `region-key` as local types (`layer-idx = u32`).

Resources, records, and enums (current at time of writing — confirm against
`crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit`):

- `layer-collection-view` — read-only view of one completed layer:
  `layer-index() -> layer-idx`, `z() -> f32`, `entity-count() -> u32`,
  `ordered-entities() -> list<print-entity-view>`,
  `tool-changes() -> list<tool-change-view>`,
  `z-hops() -> list<z-hop-view>`.
- `print-entity-view` (record): `entity-id: u64`, `path: extrusion-path3d`,
  `role: extrusion-role`, `tool-index: u32`, `region-key: region-key`,
  `topo-order: u32`. The `entity-id` is the stable per-layer ID from packet 39
  (see `docs/02_ir_schemas.md` IR 10). `tool-index` is the first-class tool
  selector from the region_id↔tool split — the finalization input deep-copy
  reconstructs `PrintEntity` from this view, so the view must carry it.
- `tool-change-view` (record): `after-entity-index: u32`, `from-tool: u32`,
  `to-tool: u32`.
- `z-hop-view` (record): `after-entity-index: u32`, `hop-height: f32`.
- `finalization-output-builder` (resource) — the mutation API:
  - `push-entity-to-layer(layer-index, path, tool-index, region-key) -> result<_, string>`
  - `push-entity-with-priority(layer-index, path, tool-index, region-key, priority) -> result<_, string>`
    — note `extrusion-path3d` already carries the role; there is no separate `role` parameter.
    The `tool-index: u32` parameter is the explicit tool selector for the pushed
    entity (region_id↔tool split): finalization guests pass it directly because
    `push-entity-*` carries only a region-key (a pure identity, no tool channel),
    and the host sets `PrintEntity.tool_index` from it at reconstruction.
  - `modify-entity(layer-index, entity-id, mutation) -> result<_, string>`
  - `sort-layer-by(layer-index, key) -> result<_, string>`
  - `insert-synthetic-layer(z, paths) -> result<_, string>` and
    `insert-synthetic-layer-after(idx, layer-data) -> result<_, string>`
- `entity-mutation` (variant) — packet 41 enum-serialisable mutations.
  Confirm the current variant set against `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit`; at the
  time of writing it is a narrow set rather than the speculative six-variant
  enum some older drafts of this doc described.
- `sort-key` (enum, not variant) — sort discriminators consumed by
  `sort-layer-by`. Names follow the form `by-<…>`; read the on-disk file for
  the current set.
- `synthetic-layer-data` (record) — `z: f32`, `paths: list<extrusion-path3d>`.

Host validation: the host validates that `entity-id` in `modify-entity`
resolves to a real entity within `layer`; unknown IDs are rejected with
`BuilderError::UnknownEntity`. The closure-based API from packet 40 is
superseded by the enum-based mutation API so the contract is fully
serialisable across the WIT boundary.

**Positional insertion and permutation (Packet 58, 2026-05-18)**:
`finalization-output-builder` exposes three additional methods that mirror PathOptimization's `layer-collection-builder` capability surface:

- `insert-entity-at(layer-index, position: u32, path, tool-index, region-key) -> result<_, string>` — inserts an entity at a specific position in the layer's `ordered_entities` list. `tool-index: u32` is the explicit tool selector (region_id↔tool split). On apply, `ToolChange.after_entity_index >= position` and `ZHop.after_entity_index >= position` are each incremented by 1 to preserve their positional references. Out-of-bounds position returns `Err` with no mutation.
- `set-entity-order(layer-index, items: list<tuple<u32, bool>>) -> result<_, string>` — permutes the layer's entities by the supplied index list (one entry per existing entity; the boolean is a reverse flag). On apply, `ToolChange.after_entity_index` and `ZHop.after_entity_index` are remapped through the inverse permutation. Malformed proposals (length mismatch, duplicates, out-of-range indices) return `Err` with no mutation.
- `get-ordered-entities(layer-index) -> list<print-entity-view>` — returns the staged state of the layer's `ordered_entities`. The SDK path observes both completed and in-flight builder state; the host-side WIT impl currently returns the pre-apply layer snapshot only (in-flight pushes are not reflected until `apply_to` runs). Module authors who need the staged state during the same `run_finalization` call should rely on the SDK side; the host accessor is a snapshot of pre-existing entities.

The index-remap invariants are owned by the SDK's `apply_to` (`crates/slicer-sdk/src/traits.rs::FinalizationOutputBuilder::apply_to`); modules must not pre-adjust indices themselves. `wipe-tower` uses `insert-entity-at(layer, tc.after_entity_index + 1 + offset, ...)` to bracket each `T<n>` with retract + travel + prime + wipe entities.

---

## Module Manifest Schema (TOML)

Full annotated example for a TPMS infill module:

```toml
# ── Identity ────────────────────────────────────────────────────────────────
# The host parser (`crates/slicer-scheduler/src/manifest.rs`) currently reads only
# `id`, `version`, and `wit-world` from this section. `display-name`,
# `description`, `author`, `license`, and `homepage` are accepted in TOML but
# not stored on the LoadedModule — they are informational metadata for
# humans / future tooling.
[module]
id           = "com.community.tpms-infill"  # reverse-domain, globally unique (parsed)
version      = "1.2.0"                       # semver (parsed)
display-name = "TPMS Infill"                 # informational
description  = "Schwartz-D and Fischer-Koch-S triply periodic minimal surface infill"  # informational
author       = "community"                   # informational
license      = "MIT"                         # informational
homepage     = "https://github.com/example/tpms-infill"  # informational
wit-world    = "slicer:world-layer"          # parsed; unversioned; must name an installed WIT world

# ── Stage declaration ────────────────────────────────────────────────────────
# Exactly one stage per module. Two stages = two .wasm files.
[stage]
id = "Layer::Infill"

# ── IR access ────────────────────────────────────────────────────────────────
# Host enforces per-call. An undeclared read or write is a fatal contract error
# — there is no implicit empty/none fallback. See "Host-Boundary Access
# Enforcement" in this document.
[ir-access]
reads  = [
    "SliceIR.regions.infill_areas",
    "SliceIR.regions.effective_layer_height",
    "SliceIR.regions.z",
    "RegionMapIR",
]
writes = ["InfillIR.regions.sparse_infill"]

# ── Claims ───────────────────────────────────────────────────────────────────
[claims]
holds    = ["infill-generator"]   # exclusive slot; one module per region
requires = []                     # claim slots that MUST be held by another module

### Known claim IDs

| Claim ID                  | Purpose                                                                   |
|---------------------------|---------------------------------------------------------------------------|
| `perimeter-generator`     | Held by the module producing wall loops on a given region.                |
| `infill-generator`        | **Deprecated 2026-06-09 (DEV-065).** Held by the module producing infill paths on a given region. Packet 37's four per-role claims (`claim:top-fill` … `claim:sparse-fill`) supersede this blanket gate. In-tree infill modules (rectilinear, gyroid, lightning) no longer declare it. Third-party modules that still declare it continue to load, but cannot coexist with another module holding the same claim (first-winner dedup applies). |
| `support-generator`       | Held by the module producing support extrusions on a given layer/region.  |
| `support-planner`         | Held by the PrePass module emitting `SupportPlanIR`.                      |
| `seam-placer`             | Held by the module placing seam candidates and resolving seam positions.  |
| `seam-planner`            | Held by the PrePass module emitting `SeamPlanIR` (`seam-planner-default`). |
| `layer-planner`           | Held by the module proposing layer Z heights and active-region lists.     |
| `path-optimizer`          | Held by the module ordering entities and emitting travels/retracts at `Layer::PathOptimization` (`path-optimization-default`). |
| `mesh-analyzer`           | Held by the module annotating facets and proposing surface groups.        |
| `slice-postprocessor`     | Held by a module that mutates `SliceIR` polygons after initial slicing.   |
| `gcode-postprocessor`     | Held by a PostPass module that processes the `GCodeCommand` stream.       |
| `text-postprocessor`      | Held by a PostPass module that mutates the final G-code text string.      |
| `claim:top-fill`          | Held by the module producing `TopSolidInfill` extrusions on this layer.  |
| `claim:bottom-fill`       | Held by the module producing `BottomSolidInfill` extrusions.             |
| `claim:bridge-fill`       | Held by the module producing `BridgeInfill` extrusions.                  |
| `claim:sparse-fill`       | Held by the module producing `SparseInfill` extrusions.                  |
| `claim:ironing`           | Held by the module producing `Ironing` extrusions (`top-surface-ironing`). |

| Claim ID                 | Kind     | Dedup          | Owner                                                                    |
|--------------------------|----------|----------------|--------------------------------------------------------------------------|
| `claim:infill-link`      | non-fill | first-winner   | `infill-linker` (`Layer::InfillPostProcess`, packet 130; ADR-0025)       |

The four fill-role claims (`claim:top-fill` … `claim:sparse-fill`) were added in packet 37. A single module may hold multiple fill-role claims (e.g. `rectilinear-infill` holds all four by default). Claim-conflict validation runs in DAG validation pass 2; per-region overrides may transfer a fill-role claim to a different module.

### Holder identifier matching

The `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder` config keys (and any future per-claim holder fields) accept either the full module ID or its short name. The matcher (see `crates/slicer-scheduler/src/validation.rs::module_id_matches_holder`) compares:

- Exact match (`"com.core.rectilinear-infill" == "com.core.rectilinear-infill"`), OR
- Short-name match after stripping the canonical built-in namespace `com.core.` from the module ID (`"com.core.rectilinear-infill"` matches `"rectilinear-infill"`).

The `com.core.` prefix is reserved for built-in modules; community modules (e.g. `com.acme.foo`) must be referenced by full ID in config because no other short form is unambiguous.

The configured holder per claim is selected by four `ResolvedConfig` keys —
`top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder`,
`sparse_fill_holder` — each defaulting to `"rectilinear-infill"` (defined as
`ResolvedConfig` fields in `docs/02_ir_schemas.md`; resolved per
`docs/04_host_scheduler.md` § "Claim Resolution with Runtime Disable Rules").
Per-region
overrides flow through `RegionMapIR.entries[*].config` (reused from
packet 35). At dispatch time the host computes the effective held set per
region by intersecting each module's manifest `[claims].holds` with the
configured holders (see `slicer_runtime::resolve_held_claims`).

The set is exposed across the WIT boundary via
`slice-region-view.held-claims` and consumed by guest modules through
`SliceRegionView::should_emit(role)`. Convention: an **empty held-claims
list is treated as "holds all four"** so test fixtures and code paths that
bypass `dispatch_layer_call` retain the pre-packet-37 default behavior.
Production dispatch always populates the set authoritatively, so `should_emit`
returns the configured truth in real runs.

# ── Compatibility ─────────────────────────────────────────────────────────────
[compatibility]
incompatible-with = []            # module IDs or globs that cannot coexist in same region
requires          = []            # module IDs that must be present and enabled
min-host-version  = "0.5.0"
min-ir-schema     = "1.2.0"      # inclusive
max-ir-schema     = "2.0.0"      # exclusive upper bound

# ── Config schema ─────────────────────────────────────────────────────────────
[config.schema]

  [config.schema.pattern]
  type        = "enum"
  values      = ["schwartz-d", "fischer-koch-s"]
  default     = "schwartz-d"
  display     = "TPMS Pattern"
  description = "Which TPMS surface family to use"
  group       = "Pattern"

  [config.schema.density]
  type     = "float"
  default  = 0.15
  min      = 0.05
  max      = 0.95
  step     = 0.01
  display  = "Infill Density"
  unit     = "ratio"          # UI renders as percentage
  group    = "Pattern"
  validate = "value > 0.0 && value < 1.0"

  [config.schema.multiline-count]
  type    = "int"
  default = 1
  min     = 1
  max     = 4
  display = "Parallel Passes"
  group   = "Pattern"

  [config.schema.marching-cell-size]
  type     = "float"
  default  = 0.40
  min      = 0.10
  max      = 1.00
  step     = 0.05
  display  = "Marching Cell Size (mm)"
  group    = "Advanced"
  advanced = true             # hidden unless user expands section

  [config.schema.raster-precision]
  type     = "float"
  default  = 0.004
  min      = 0.001
  max      = 0.010
  display  = "Raster Precision (mm)"
  group    = "Advanced"
  advanced = true

# ── Cross-field validation ────────────────────────────────────────────────────
# <!-- VERIFY: as of this writing, `manifest.rs` does not parse
#      `[[config.cross-validate]]`; the rule is not enforced at module load.
#      Treat this section as a forward-looking design item until the parser
#      and validator catch up. -->
[[config.cross-validate]]
rule     = "marching-cell-size >= raster-precision * 10"
message  = "Marching cell size should be at least 10x the raster precision"
severity = "warning"    # "error" blocks slicing; "warning" notifies only

# ── Per-region / per-layer override policy ────────────────────────────────────
[config.overridable-per-region]
keys = ["pattern", "density", "multiline-count"]

[config.overridable-per-layer]
keys = ["density"]      # density can vary per-layer; pattern cannot

# ── Region-split semantics declaration (Normative — Packet 92) ─────────────
# Each [[region_split]] entry declares one paint semantic this module wants
# the host to split regions on. The host aggregates entries across all
# loaded manifests into a canonical BTreeMap ordered by (priority, name),
# and exposes the resulting semantic set to PrePass::RegionMapping for
# cross-product expansion of variant_chain.
[[region_split]]
semantic   = "material"        # PaintSemantic name (snake_case)
priority   = 100               # Core priorities (locked):
                                #   material   = 100
                                #   fuzzy_skin = 200
                                # Community semantics must have priority >= 1000
                                # (COMMUNITY_PRIORITY_FLOOR).
value_type = "tool_index"      # flag | tool_index | custom_string
                                # `scalar` is REJECTED at manifest load —
                                # Scalar paints route through
                                # SlicedRegion.segment_annotations instead.

# ── Hints ─────────────────────────────────────────────────────────────────────
[hints]
# <!-- VERIFY: `manifest.rs` parses only `layer-parallel-safe`; other hint
#      keys (e.g. `estimated-ms-per-layer`) are accepted in TOML but not stored
#      on LoadedModule. They are no-ops at runtime today. -->
estimated-ms-per-layer = 12    # informational; not consumed by the host today
# layer-parallel-safe must be false for PostPass::LayerFinalization modules.
# The host normalises finalization-stage manifests to `false` and logs a
# warning if `true` is set (`manifest.rs::finalization_parallel_hint_is_normalized_and_warned`).
# All other stages: true allows the host to run multiple layers simultaneously.
layer-parallel-safe    = true
```

### Config-Key Wildcard Syntax (Normative — Packet 76)

Config keys declared in `[config.schema]` and any `[config.overridable-per-*]`
section may use the `<prefix>:*` wildcard form. A declared key of the form
`<prefix>:*` matches all runtime keys whose name begins with `<prefix>:`,
enabling modules to declare a single schema entry for dynamically-named
keys such as `object_height:<uuid>` or `paint_config:<semantic>:<key>`.
Static keys (without the `:*` suffix) continue to require exact-match.
The matcher is `source_key_matches_declared` in
`crates/slicer-scheduler/src/execution_plan.rs`.

### `[[region_split]]` Validation Rules (Normative — Packet 92)

Per-manifest:

1. **Duplicate semantic** within a single manifest →
   `LoadErrorKind::DuplicateRegionSplitSemantic`.
2. **`value-type = "scalar"`** → rejected
   (`LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit`). See
   `docs/02_ir_schemas.md` for the routing rationale.
3. **Community semantic with `priority < 1000`** → rejected
   (`LoadErrorKind::CommunityPriorityBelowFloor`). `COMMUNITY_PRIORITY_FLOOR = 1000`.
4. **Core semantic (`material`, `fuzzy_skin`) with `priority` ≠ registry
   value** → rejected (`LoadErrorKind::CoreSemanticPriorityMismatch`).
   `CORE_REGION_SPLIT_PRIORITIES = { "material" => 100, "fuzzy_skin" => 200 }`.

Cross-manifest: distinct semantics from different manifests that share a
priority emit a non-fatal `LoadDiagnostic { level: Warning, ... }` naming
both manifests and the lexicographic tiebreaker order; aggregation
continues.

### Variant-chain enumeration order is contract (Normative — Packet 93)

The canonical order for `variant_chain` enumeration in
`PrePass::RegionMapping` is deterministic and locked by test fixtures:

- **Semantics** are ordered by `BTreeMap` iteration of the aggregated
  `region_split` map (lexicographic on `semantic` name within priority
  tiers).
- **PaintValue** instances within each semantic are ordered:
  `Flag(false) < Flag(true) < ToolIndex(0) < ToolIndex(1) < ... < Custom(s_lex)`.

Reordering breaks every existing region-split test suite. Any change
to the enumeration order must be a coordinated packet with explicit
test-fixture updates.

### Configuration keys added by recent packets

The following `[config.schema.<key>]` blocks document config keys introduced after the TPMS annotated example above. Keys follow the snake_case convention throughout (see CLAUDE.md).

#### Packet 34 — retraction mode

Retraction mode is chosen **per retract**, not by a config key. A module calls
`push-retract` / `push-unretract` with a `retract-mode` argument, and the emitter
honours it per command. `RetractMode::Gcode` (the default) emits standard
`G1 E<n> F<speed>` retract/unretract moves; `RetractMode::Firmware` emits `G10`
(retract) / `G11` (unretract). M207/M208 are intentionally never emitted
regardless of mode. The enum is `RetractMode` in
`crates/slicer-ir/src/slice_ir.rs`; emission is in
`crates/slicer-gcode/src/serialize.rs`.

<!-- VERIFY: this section previously documented a `[config.schema.retraction_mode]`
     manifest key (enum ["gcode","firmware"], default "gcode"). No such key exists:
     `retraction_mode` appears in no manifest under modules/ and is read by no code
     under crates/. The mode is carried per-command via the WIT `retract-mode`
     argument instead. Removed as fabricated; restore only if a global config key
     is actually introduced. -->

> See also `docs/15_config_keys_reference.md` for the full catalogue of
> recognised keys across all packets, organised by functional domain.

#### Packet 52 — per-role speed schema

The following 25 keys form the per-role speed family. All speed keys are `float` (unit `mm/s`); acceleration keys are `float` (unit `mm/s²`). One representative block is shown; the rest share the same shape.

```toml
[config.schema.outer_wall_speed]
type    = "float"
default = 50.0
min     = 1.0
unit    = "mm/s"
display = "Outer wall speed"
group   = "Speed"

[config.schema.inner_wall_speed]
type    = "float"
default = 80.0
min     = 1.0
unit    = "mm/s"
display = "Inner wall speed"
group   = "Speed"
```

Complete key list: `outer_wall_speed`, `inner_wall_speed`, `internal_solid_infill_speed`, `top_surface_speed`, `gap_infill_speed`, `sparse_infill_speed`, `bridge_speed`, `support_speed`, `support_interface_speed`, `travel_speed`, `first_layer_speed`, `first_layer_infill_speed`, `first_layer_travel_speed`, `initial_layer_print_height_speed_factor`, `ironing_speed`, `overhang_speed`, `small_perimeter_speed`, `external_perimeter_speed`, `solid_infill_speed`, `top_solid_infill_speed`, `bottom_solid_infill_speed`, `default_acceleration`, `outer_wall_acceleration`, `inner_wall_acceleration`, `infill_acceleration`.

After packet 52, every emitted move carries an F-token (`F<feedrate>`); the emitter does not elide F for unchanged feedrates.

#### Packet 54 — relative extrusion

```toml
[config.schema.use_relative_e_distances]
type    = "bool"
default = true
display = "Use relative E distances"
group   = "Extruder"
```

Maps to M83 when `true` (default), M82 when `false`. `G92 E0` is issued on mode transitions and layer-reset boundaries. See also `docs/02_ir_schemas.md` IR 11 (GCodeCommand stream-level invariant).

#### Packet 57 — overhang speed

```toml
[config.schema.overhang_1_4_speed]
type    = "float"
default = 0.0
min     = 0.0
unit    = "mm/s"
display = "Overhang speed (0–25 %)"
group   = "Speed"

[config.schema.overhang_2_4_speed]
type    = "float"
default = 0.0
min     = 0.0
unit    = "mm/s"
display = "Overhang speed (25–50 %)"
group   = "Speed"

[config.schema.overhang_3_4_speed]
type    = "float"
default = 0.0
min     = 0.0
unit    = "mm/s"
display = "Overhang speed (50–75 %)"
group   = "Speed"

[config.schema.overhang_4_4_speed]
type    = "float"
default = 0.0
min     = 0.0
unit    = "mm/s"
display = "Overhang speed (75–100 %)"
group   = "Speed"
```

The classifier short-circuits when all four values are exactly `0.0` (byte-identical no-op path). Quartile assignment is documented in `docs/02_ir_schemas.md` (`Point3WithWidth.overhang_quartile`).

#### Packet 60 — precision

Units and defaults mirror `02_ir_schemas.md` "Polyline simplification and precision" subsection.

**These are host-side keys, not module-manifest keys.** Their authoritative
defaults live in the `declare_resolved_config!` invocation in
`crates/slicer-ir/src/resolved_config.rs`, and `docs/config/host-keys.toml`
mirrors host-registered keys in machine-readable form. Of the seven, only
`perimeter_arc_tolerance` is additionally declared in a module manifest
(`modules/core-modules/classic-perimeters/classic-perimeters.toml`, which reads
it per-module). Current host defaults:

| Key | Type | Default | Purpose |
|---|---|---|---|
| `gcode_resolution` | f32 | `0.0125 mm` | Douglas-Peucker tolerance for wall/brim |
| `infill_resolution` | f32 | `0.04 mm` | Douglas-Peucker tolerance for infill |
| `support_resolution` | f32 | `0.0375 mm` | Douglas-Peucker tolerance for support |
| `min_segment_length` | f32 | `0.05 mm` | Drop adjacent segments shorter than this |
| `gcode_xy_decimals` | u32 | `3` | Decimal places for X/Y/Z tokens |
| `perimeter_arc_tolerance` | f32 | `0.0125 mm` | Clipper2 arc tolerance for perimeter offsets |
| `slice_closing_radius` | f32 | `0.049 mm` | Per-layer inflate(+r) → inflate(−r) round-trip |

#### Packet 55 — G-code preamble (header, thumbnail, config block)

The four envelope blocks (`HEADER_BLOCK_*`, `THUMBNAIL_BLOCK_*`, per-role
width comments, `CONFIG_BLOCK_*`) are documented under
`docs/02_ir_schemas.md` "G-code envelope blocks". Four config keys feed
the header block:

```toml
[config.schema.filament_diameter]
type    = "float"
default = 1.75
min     = 0.5
max     = 5.0
unit    = "mm"
display = "Filament diameter"
group   = "Filament"

[config.schema.filament_density]
type    = "float"
default = 1.24
min     = 0.5
max     = 5.0
unit    = "g/cm^3"
display = "Filament density"
group   = "Filament"

[config.schema.max_z_height]
type    = "float"
default = 0.0
min     = 0.0
unit    = "mm"
display = "Maximum Z height (0 = auto from per-print Z extent)"
group   = "Machine"

[config.schema.thumbnail_path]
type    = "string"
default = ""
display = "Thumbnail PNG path (alternative to --thumbnail CLI flag)"
group   = "Output"
```

CLI flag:

```text
--thumbnail <PATH>      # PNG; Base64-encoded into THUMBNAIL_BLOCK_*
                        # CLI flag wins over thumbnail_path config when both set.
```

#### Packet 31b — tree-support OrcaSlicer parity

The following nine keys map directly to OrcaSlicer keys of the same name.

```toml
[config.schema.tree_support_branch_angle]
type    = "float"
default = 45.0
unit    = "deg"
display = "Tree support branch angle"
group   = "Support"

[config.schema.tree_support_branch_diameter]
type    = "float"
default = 5.0
unit    = "mm"
display = "Tree support branch diameter"
group   = "Support"

[config.schema.tree_support_branch_diameter_angle]
type    = "float"
default = 5.0
unit    = "deg"
display = "Tree support branch diameter angle"
group   = "Support"

[config.schema.tree_support_branch_distance]
type    = "float"
default = 1.0
unit    = "mm"
display = "Tree support branch distance"
group   = "Support"

[config.schema.tree_support_wall_count]
type    = "int"
default = 1
min     = 0
display = "Tree support wall count"
group   = "Support"

[config.schema.support_raft_layers]
type    = "int"
default = 0
min     = 0
display = "Support raft layers"
group   = "Support"

[config.schema.support_interface_top_layers]
type    = "int"
default = 2
min     = 0
display = "Support interface top layers"
group   = "Support"

[config.schema.support_interface_bottom_layers]
type    = "int"
default = -1          # -1 = all layers (OrcaSlicer convention)
min     = -1
max     = 10
display = "Support interface bottom layers"
group   = "Support"

[config.schema.tree_support_interface_spacing_mm]
type    = "float"
default = 0.4
unit    = "mm"
display = "Tree support interface spacing"
group   = "Support"
```

### Per-paint-region config overrides (packet 51)

The namespace `paint_config:<semantic>:<key>` is recognised at module-load time as a per-paint-region config override.

Built-in `PaintSemantic` variants serialise as: `material`, `fuzzy_skin`, `support_enforcer`, `support_blocker`. `PaintSemantic::Custom(s)` uses the inner string verbatim as the `<semantic>` segment.

Override precedence (lowest → highest):

```text
global < object_config:<id>:<key> < paint_config:<semantic>:<key>
```

The audit trail for applied paint overrides surfaces in `RegionMapIR.paint_overrides` (see `02_ir_schemas.md` § "IR 4 — RegionMapIR").

### Per-object config overrides (packet 35a)

Per-object overrides use the namespace `object_config:<id>:<key>`. These flow through `RegionPlan.config: ResolvedConfig` and are stamped on every `RegionPlan` and `ActiveRegion` during the resolved-config builder stage added in packet 35a. The propagation path is: CLI JSON → per-object overlay → `ResolvedConfig` stamped per-region. See `02_ir_schemas.md` § "`ResolvedConfig`" for that type's contract, and its § "IR 4 — RegionMapIR" for `RegionMapIR.entries[*].config`.

### Machine start / end G-code emission (packet 59)

Module-owned machine start/end G-code is emitted by a designated module running at `PostPass::GCodePostProcess`. The bundled implementation is `machine-gcode-emit` (`modules/core-modules/machine-gcode-emit/`); the audit boundary is the contract, not the module ID. The stage is `GCodePostProcess` (not `LayerFinalization`) because the module operates on the typed `GCodeIR` command stream before serialization — it prepends a Raw start block before the first command and appends a Raw end block after the last, so ordering is natural and type-safe.

The module reads four config keys:

```toml
[config.schema.machine_start_gcode]
type    = "string"
default = """M190 S[bed_temperature_initial_layer_single]
M109 S[nozzle_temperature_initial_layer]
PRINT_START EXTRUDER=[nozzle_temperature_initial_layer] BED=[bed_temperature_initial_layer_single]"""
display = "Machine Start G-code"
group   = "Machine G-code"

[config.schema.machine_end_gcode]
type    = "string"
default = "PRINT_END"
display = "Machine End G-code"
group   = "Machine G-code"

[config.schema.bed_temperature_initial_layer_single]
type    = "int"
default = 60
display = "Bed Temperature (Initial Layer)"
group   = "Machine G-code"

[config.schema.nozzle_temperature_initial_layer]
type    = "int"
default = 215
display = "Nozzle Temperature (Initial Layer)"
group   = "Machine G-code"
```

The defaults are Klipper-flavoured (`PRINT_START` / `PRINT_END` macros). Both G-code strings support `[key]` placeholder substitution: each `[snake_case_key]` is replaced with the effective value of that config key resolved from the `ConfigView` (e.g. the default `machine_start_gcode` references `[bed_temperature_initial_layer_single]` and `[nozzle_temperature_initial_layer]`). Substitution runs against the effective config before the Raw commands are emitted.

The module emits the start block before the first `GCodeIR` command and the end block after the last.

## Path Optimization Output Contract (Normative)

This section pins down what `Layer::PathOptimization` guests are allowed to
emit through `gcode-output-builder` and how the host commits that output into
`LayerCollectionIR` (see `docs/02_ir_schemas.md` § IR 10).

### Inputs

- `regions: list<perimeter-region-view>` — read-only view of the layer.
- `output: gcode-output-builder` — same WIT resource used by post-pass, but
  the accepted method set is restricted by stage as described below.

### Pre-staged ordered_entities

- The host assembles `LayerCollectionIR.ordered_entities` deterministically
  from the committed per-layer arena (`PerimeterIR`, `InfillIR`, `SupportIR`)
  immediately *before* `Layer::PathOptimization` runs.
- In the current `world-layer` contract, guests **cannot** reorder, append to, or remove
  entries from `ordered_entities`. The pre-staged sequence is final for the
  lifetime of the layer. `topo_order` indices are stable and used as the
  `after_entity_index` keying domain for tool-changes and annotations.
- Reordering of `ordered_entities` is performed via the `layer-collection-builder`
  resource (packet 32; see `world-layer.wit` section above). Guests that need
  deterministic reordering use `set-entity-order` on that resource; arbitrary
  mutation or append outside of that resource is still rejected at the host boundary.

### Accepted `gcode-output-builder` methods at PathOptimization

| Method                                 | Accepted? | Commit destination                                                                                                  |
|----------------------------------------|-----------|---------------------------------------------------------------------------------------------------------------------|
| `push-tool-change(from-tool, to-tool)` | yes       | Appended to `LayerCollectionIR.tool_changes` with `after_entity_index = ordered_entities.len() - 1` (or 0 if empty) |
| `push-comment(text)`                   | yes       | Appended to `LayerCollectionIR.annotations` as `Comment(text)` with the same anchor rule                            |
| `push-raw(text)`                       | yes       | Appended to `LayerCollectionIR.annotations` as `Raw(text)` with the same anchor rule                                |
| `push-move(cmd)`                       | rejected  | Fatal `FatalModule` diagnostic — no documented `LayerCollectionIR` mapping                                          |
| `push-retract(length, speed)`          | rejected  | Fatal `FatalModule` diagnostic                                                                                      |
| `push-unretract(length, speed)`        | rejected  | Fatal `FatalModule` diagnostic                                                                                      |
| `push-fan-speed(value)`                | rejected  | Fatal `FatalModule` diagnostic                                                                                      |
| `push-temperature(...)`                | rejected  | Fatal `FatalModule` diagnostic                                                                                      |

The `LayerAnnotation { after_entity_index, kind: Comment(..)|Raw(..) }` IR
record is the host-side carrier for guest comment/raw oustput. The default
`PostPass::GCodeEmit` emitter inserts each annotation as
`GCodeCommand::Comment` or `GCodeCommand::Raw` immediately after the entity
identified by its `after_entity_index`. Annotations whose anchor lies past
the last entity are emitted in declaration order at the end of the layer
(this covers empty-layer comments). Declaration order is preserved both
within an anchor and across the layer.

### z-hops

`gcode-output-builder` exposes one z-hop method available *only* at
`Layer::PathOptimization`:

```wit
push-z-hop: func(after-entity-index: u32, hop-height: f32) -> result<_, string>;
```

This is the single, minimal z-hop output channel. Entity-order rewriting uses the `layer-collection-builder` resource (packet 32; see `world-layer.wit` section above).

#### Commit destination

- Each accepted call appends one `ZHop { after_entity_index, hop_height }`
  entry onto `LayerCollectionIR.z_hops`.
- Guests that never call `push-z-hop` leave `z_hops` empty.

#### Validation (host, normative)

The host validates each `push-z-hop` call at commit time. A failed call
aborts the layer with `LayerStageError::FatalModule` (no partial commit).

| Rule                                  | Reject condition                                                                     |
|---------------------------------------|--------------------------------------------------------------------------------------|
| `after-entity-index` in bounds        | `after_entity_index >= ordered_entities.len()` (and `ordered_entities` is non-empty) |
| `after-entity-index` for empty layers | any value other than `0` when `ordered_entities.len() == 0`                          |
| `hop-height` finite                   | `!hop_height.is_finite()`                                                            |
| `hop-height` strictly positive        | `hop_height <= 0.0`                                                                  |

Required diagnostic fields on rejection:

- stage id (`Layer::PathOptimization`)
- module id
- the rejected method (`push-z-hop`)
- the index of the rejected call in the guest's emit sequence
- the failing field (`after-entity-index` or `hop-height`) and its value

#### Deterministic insertion semantics

- The host commit step preserves the guest's emit order across all
  `push-z-hop` calls within a single invocation.
- When multiple z-hops share the same `after-entity-index`, they appear in
  `LayerCollectionIR.z_hops` in the order the guest emitted them.
- Repeated runs over identical input produce bit-identical `z_hops` vectors.

#### Downstream emission

- `DefaultGCodeEmitter` consumes `LayerCollectionIR.z_hops` deterministically
  by `after_entity_index`, lifting to `layer.z + hop_height` and returning to
  `layer.z` immediately after the entity at that index. An empty `z_hops`
  vector emits no hop commands.

#### Out of contract

- `push-move`, `push-retract`, `push-fan-speed`, and `push-temperature`
  remain rejected at `Layer::PathOptimization`.
- Reordering `ordered_entities` uses `layer-collection-builder.set-entity-order` (packet 32). Appending arbitrary new entities or removing existing entries outside of that resource is still rejected.

### Determinism & identity

- For a fixed input layer arena, repeated runs of `Layer::PathOptimization`
  must produce the same `tool_changes`, `annotations`, and (since the host
  forbids reorder) the same `ordered_entities` and `topo_order`.
- The host commit step is order-preserving with respect to the guest's call
  sequence on the builder.

### Out-of-contract rejection

- The diagnostic produced for any rejected method must include: stage id,
  module id, the rejected method name (or `GcodeCommandCollected` discriminant),
  and the index of the rejected call in the guest's emit sequence.
- Rejection aborts the layer (per the existing `LayerStageError::FatalModule`
  contract). The pre-staged `LayerCollectionIR` is *not* surfaced to
  downstream stages when commit fails.

### Host nearest-neighbour fallback (packet 33)

Packet 33 migrated the host's nearest-neighbour entity-ordering fallback into the `path-optimization-default` module. The host no longer carries an entity-ordering fallback; if no module claims `path-optimization` on a layer, the layer's `ordered_entities` is the order produced by upstream stages (no NN reorder). Packet 18 is marked superseded.

## Builder Lifecycle Contract (Normative)

Output builder resources (`*-output-builder`) are invocation-scoped:

- Valid only during the active exported function call.
- Must not be cached across calls by the module.
- Host invalidates/drops builders immediately after return.

Attempting to reuse an invalidated builder is a fatal contract error.

## Valid Reads/Writes

### Paint reads

Paint data reaches modules through `SliceIR`, not through a dedicated paint IR.
Per-variant polygons are written into `SliceIR.regions` by
`PrePass::PaintSegmentation`, and per-segment annotations are populated by the
always-on `Layer::PaintRegionAnnotation` host built-in before any downstream
per-layer stage runs. Declare the slice paths you need:

```toml
reads = ["SliceIR.regions.segment_annotations"]
```

<!-- NOTE: Packet 95 deleted the standalone `PaintRegionIR` *Rust struct* (per-variant
     polygons were inlined into `SliceIR.regions[*]` via `SlicedRegion.variant_chain`),
     so the per-semantic dotted read paths `PaintRegionIR.FuzzySkin` / `.SupportEnforcer`
     / `.SupportBlocker` / `.Material` / `.Custom.<id>` no longer exist and were removed
     from this section. HOWEVER, `PaintRegionIR` survives as the ir-access
     read-attribution NAME for the `PaintRegionLayerView` WIT accessor: the host stamps
     `runtime_reads.push("PaintRegionIR")` when a guest reads paint regions per layer
     (`crates/slicer-wasm-host/src/host.rs`), and the ir-access contract mandates
     `Layer::Perimeters => reads ["SliceIR", "PaintRegionIR"]`
     (`crates/slicer-scheduler/tests/contract/core_module_ir_access_contract_tdd.rs`).
     The `classic-perimeters` / `arachne-perimeters` manifests declaring
     `reads = ["SliceIR", "PaintRegionIR"]` are therefore CORRECT and contract-required —
     NOT dangling. Do not remove them. -->

### Wall feature flags

```toml
reads  = ["PerimeterIR.regions.walls.feature_flags"]   # fuzzy skin post-processor
writes = ["PerimeterIR.regions.walls.feature_flags"]   # if modifying flags
```

### Narrow write paths (Normative — packets 24 / 25)

Write declarations must use the **narrowest path that covers every host-side
write instrument the module triggers**. Coarse top-level paths
(e.g. plain `"PerimeterIR"`) are no longer acceptable for modules that mutate
specific sub-fields; the host rejects manifests whose declared writes are
broader than the writes the host actually performs. The narrow-path canon for
the perimeter / seam pipeline:

| Builder call                                                      | Narrow write path required                                                            |
|-------------------------------------------------------------------|---------------------------------------------------------------------------------------|
| `perimeter-output-builder.push-wall-loop(...)`                    | `"PerimeterIR.regions.walls"`                                                         |
| `perimeter-output-builder.push-reordered-wall-loop(...)`          | `"PerimeterIR.regions.walls"` (rotates feature flags in place — same write target)    |
| `perimeter-output-builder.push-resolved-seam(...)`                | `"PerimeterIR.resolved-seam"` (note dot-separated; not `regions.resolved_seam`)       |

IR path format:

- Dot-separated, host-canonical (snake_case identifiers, hyphenated public
  paint semantics like `paint_config:fuzzy-skin:line_width` use the wire form).
- The leading IR name (e.g. `PerimeterIR`, `SliceIR`) is required.
- No wildcards — every write target must be explicit. Host emits a fatal
  diagnostic listing the actual write targets if a module's `writes`
  declaration does not cover all of them.

---

## Config Field Types Reference

| Type            | Description                | Extra keys                   |
|-----------------|----------------------------|------------------------------|
| `"bool"`        | Boolean checkbox           | —                            |
| `"int"`         | Integer                    | `min`, `max`, `step`         |
| `"float"`       | Floating point             | `min`, `max`, `step`, `unit` |
| `"string"`      | Free text                  | `max-length`                 |
| `"enum"`        | Fixed set of string values | `values` (required)          |
| `"float-list"`  | List of floats             | `min`, `max`, `min-length`, `max-length` |
| `"string-list"` | List of strings            | `min-length`, `max-length`   |
| `"percent"`     | Value expressed as a % of a caller-supplied base (resolved module-side via `ConfigView::get_abs_value(key, base)`) | `min`, `max`, `step` |
| `"float_or_percent"` | Absolute float OR a percent literal, resolved at read time via `ConfigView::get_abs_value(key, base)` | `min`, `max`, `step`, `unit` |

### Common per-field keys (apply to every type)

| Key       | Type             | Purpose                                                          |
|-----------|------------------|------------------------------------------------------------------|
| `display` | string           | UI label shown next to the field.                                |
| `description` | string       | UI tooltip / help text.                                          |
| `group`   | string           | UI grouping hint (becomes a section header in the settings tab). |
| `advanced` | bool            | Hidden by default; revealed only in advanced view.               |
| `validate` | string          | Single-field validation expression. See § Validation Expression Language. |
| `tags`    | array of strings | UI taxonomy tags for sub-tab filtering and search (free-form). Emitted as `[]` when absent. |

#### Tag conventions

Tags are free-form strings; the host does not validate them. Module authors
should follow these conventions so the studio's filters and search behave
consistently across modules:

- **Difficulty:** `"basic"`, `"advanced"`, `"experimental"`.
- **Area:** `"walls"`, `"infill"`, `"support"`, `"cooling"`, `"top-bottom"`,
  `"adhesion"`, `"seam"`, `"speed"`, `"travel"`, `"quality"`.
- **Mode flags:** `"multi-material"`, `"tree-support"`, `"ironing"`.

Do not namespace tags (e.g. write `"walls"`, not `"area:walls"`). The studio
matches bare strings against its taxonomy; namespacing only adds parsing
burden for no semantic gain.

### Numeric Bounds Enforcement

`min` and `max` on numeric fields (`int`, `float`, `int-list`, `float-list`)
are not UI hints — they are enforced by the host resolver. The host builds a
`ConfigBoundsIndex` from every loaded module's `[config.schema]` at startup,
and `resolve_global_config` / `resolve_per_object_configs` /
`resolve_per_paint_semantic_configs` reject out-of-range values with
`ConfigResolutionError::OutOfRange` before the value is written into
`ResolvedConfig`. Inclusive bounds on both ends: `min <= value <= max`.

- **NaN and non-finite values** for `float` fields are treated as out-of-range
  and rejected.
- **List elements** (`float-list`, `int-list`) are validated element-wise
  against the same `[min, max]`; the first offending element is reported
  with its `index`.
- **Strictest wins on collision**: when several modules declare bounds for
  the same key, the effective range is the intersection. If two modules
  declare disjoint ranges, every value for that key is rejected and the
  host emits a `log::warn!` at module-load time naming the contributors.

### Unit Values (for UI rendering)

| Unit      | Renders as         |
|-----------|--------------------|
| `"mm"`    | `X mm`             |
| `"ratio"` | `X%` (value × 100) |
| `"deg"`   | `X°`               |
| `"mm/s"`  | `X mm/s`           |
| `"ms"`    | `X ms`             |

---

## Validation Expression Language

Used in `validate` (single field) and `cross-validate.rule` (multi-field). Deliberately restricted — no loops, no I/O, no function calls.

<!-- VERIFY: as of this writing the parser stores `validate`/`cross-validate`
     strings but does not interpret them at module load. The grammar below is
     the forward-looking design; do not assume runtime enforcement. The
     numeric-bounds enforcement above (`min`/`max`) is independent of this
     grammar and is enforced today by `ConfigBoundsIndex`. -->


```text
Literals:   0, 1.5, true, false, "string"
References: value (single-field), field-name (cross-validate)
Operators:  && || ! == != < <= > >= + - * /
Functions:  min(a,b)  max(a,b)  abs(x)  floor(x)  ceil(x)
```

Examples:

```toml
validate = "value >= 0.01 && value <= 10.0"
rule     = "outer_wall_speed <= inner_wall_speed * 1.5"
rule     = "min(layer_height, 0.35) == layer_height"
```

---

## Test Guest Fixtures (Informative)

`test-guests/` holds minimal WASM components used as fixtures by host
integration tests under `crates/slicer-runtime/tests/`. They exercise the
WIT boundary with real `wasm32-unknown-unknown` artifacts, complementing
the in-process mock host shipped to module authors via `slicer-test`
(see `docs/05_module_sdk.md` § `slicer-test` Crate). The two paths
target different concerns and are not interchangeable:

| Concern                                                  | Vehicle                          |
|----------------------------------------------------------|----------------------------------|
| Module author unit-tests their stage logic               | `slicer-test` mock host (no WASM) |
| Host verifies dispatch, IR resources, ABI, surface drift | `test-guests/*.component.wasm`   |

### Layout

Each guest is a standalone Cargo crate with its own `[workspace]` (it
targets `wasm32-unknown-unknown`, so it cannot live inside the host
workspace) and `crate-type = ["cdylib"]`. The build pipeline produces
`<guest>.component.wasm` next to the source directory; that file is the
artifact host tests load via `include_bytes!` / `std::fs::read`.

```text
test-guests/
├── layer-infill-guest/
│   ├── Cargo.toml                      # standalone workspace, cdylib
│   └── src/lib.rs
├── layer-infill-guest.component.wasm   # built artifact, checked in
├── …
└── sdk-prepass-guest/
    └── …
```

### Two families

**Hand-rolled WIT guests** spell the WIT inline via
`wit_bindgen::generate!({ inline: r#"…"# })` rather than referencing
the canonical `wit/`. They are deliberately decoupled from the canonical
surface so host instantiation will fail loudly if `wit/` drifts in a way
that breaks ABI compatibility. They are the primary regression vehicle
for type-identity, resource-handle, and dispatch correctness.

| Guest                | World                       | Verifies                                                                  |
|----------------------|-----------------------------|---------------------------------------------------------------------------|
| `layer-infill-guest` | `slicer:world-layer`        | All Layer-stage exports, output builders, paint queries, region-key commit |
| `prepass-guest`      | `slicer:world-prepass`      | PrePass exports (mesh analysis, paint, seam, support)                     |
| `finalization-guest` | `slicer:world-finalization` | `LayerCollectionView` reads + `FinalizationOutputBuilder` writes          |
| `postpass-guest`     | `slicer:world-postpass`     | `gcode-command` round-trip + text postprocess                             |

**SDK round-trip witnesses** are authored *purely* through the
`#[slicer_module]` proc-macro from `slicer-sdk`. They contain no inline
WIT and no manual `wit_bindgen` glue; the macro must emit every binding
for the binary to link. They prove the SDK codegen path produces valid
guests against the canonical `wit/`.

| Guest                          | Stage                         | Notes                                                                                           |
|--------------------------------|-------------------------------|-------------------------------------------------------------------------------------------------|
| `sdk-prepass-guest`            | `PrePass::MeshAnalysis`       | Macro-only PrePass round-trip witness                                                            |
| `sdk-layer-infill-guest`       | `Layer::Infill`               | Macro-only Layer round-trip witness                                                              |
| `sdk-layer-pathopt-guest`      | `Layer::PathOptimization`     | Macro-only PathOptimization witness                                                              |
| `sdk-finalization-guest`       | `PostPass::LayerFinalization` | Macro-only finalization witness                                                                  |
| `sdk-postpass-text-guest`      | `PostPass::GCodeText`         | Macro-only text-postprocess witness                                                              |
| `path-optimization-multi-read` | `Layer::PathOptimization`     | Asserts the macro `get-ordered-entities`-call-once cache contract; counterpart to the host counter `HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS` |

### Build & Freshness Contract (Normative)

Each guest is built with `cargo build --target wasm32-unknown-unknown --release`
followed by `wasm-tools component new` to produce the `.component.wasm` artifact.

- `cargo xtask build-guests` — build any stale guests.
- `cargo xtask build-guests --check` — verify only; exit 1
  if any source is newer than its artifact.

Freshness is enforced from the host workspace by
`crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs`, which fails
when:

- An expected `.component.wasm` is missing.
- An artifact is suspiciously small (< 100 bytes — i.e. not a real
  component).
- A guest's `src/lib.rs` is newer than its artifact.

Prerequisites for rebuilding (`rustup target add wasm32-unknown-unknown`
and `cargo install wasm-tools`) are required only when modifying a guest.
Contributors who do not touch `test-guests/` can run
`cargo test --workspace` against an unmodified tree because the
`.component.wasm` artifacts are committed.

### Signal-Encoding Convention

Test guests have no real geometry to emit. To make boundary semantics
observable from host-side asserts, they encode inputs into the spare
fields of output records. The recurring pattern across the hand-rolled
guests is:

- `point3-with-width.z`           — echo of input `region.z`.
- `point3-with-width.flow_factor` — input region count or layer index.
- `point3-with-width.width`       — total polygon count across regions.
- A single `push-comment("regions=N walls=M infill=K")` per layer.
- One `push-tool-change` per active region for ordering proofs.

Host tests then assert against `ExtrusionPathIR.points[0]` (or the
flushed `LayerCollectionIR.tool_changes` / `annotations`) to verify
that data crossed the boundary intact, paint queries were honoured,
and per-region identity (`region-key`) survived the commit path.
This convention is *informative* — guest authors are free to encode
signals differently as long as the matching host test reads them back.

### 3MF TriangleSelector child ordering

OrcaSlicer / PrusaSlicer / BambuStudio's `TriangleSelector::serialize`
walks subdivided children in **reverse index order**
(`for child_idx = split_sides; child_idx >= 0; --child_idx`). Any decoder
of 3MF paint hex sequences MUST iterate child slots in reverse before
recursing, otherwise painted states land on the wrong sub-triangle
positions. The canonical handling lives in
`crates/slicer-model-io/src/loader.rs:2018-2030`.
