# ModularSlicer — WIT Interfaces & Module Manifest Schema

---

## WIT File Organization

```
wit/
├── deps/
│   ├── types.wit          # geometry primitives (Point2, Point3, ExPolygon, etc.)
│   ├── config.wit         # ConfigView resource
│   └── ir-types.wit       # IR view and builder resources
├── host-api.wit           # services the host exposes to ALL modules
├── world-layer.wit        # world for per-layer modules (most modules target this)
├── world-prepass.wit      # world for PrePass modules
├── world-finalization.wit # world for LayerFinalization modules
└── world-postpass.wit     # world for PostPass modules
```

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
| Read custom paint regions     | `[ir-access].reads` includes `PaintRegionIR.custom:<module-id>` | Return fatal contract error                                                   |
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

WIT and IR versions are independently versioned, but module load is allowed only when both compatibility checks pass.

| Host WIT world             | Module `wit-world`         | Host IR schema | Module IR range   | Load result                      |
|----------------------------|----------------------------|----------------|-------------------|----------------------------------|
| `slicer:world-layer@1.0.x` | `slicer:world-layer@1.0.x` | `1.4.0`        | `>=1.2.0, <2.0.0` | Allowed                          |
| `slicer:world-layer@2.0.0` | `slicer:world-layer@1.x`   | `2.0.0`        | `>=1.2.0, <2.0.0` | Rejected (WIT major mismatch)    |
| `slicer:world-layer@1.0.x` | `slicer:world-layer@1.0.x` | `2.0.0`        | `>=1.2.0, <2.0.0` | Rejected (IR major out of range) |

Startup checks:

1. Validate `wit-world` package name and major version compatibility.
2. Validate host IR schema is within module-declared IR range.
3. Emit explicit diagnostics with expected/actual versions and blocking symbol names when incompatible.

---

## `deps/types.wit`

```wit
package slicer:types@1.0.0;

interface geometry {
    // Scaled integer coordinates. 1 unit = 100 nanometer (1e-4 mm).
    record point2 { x: s64, y: s64 }
    record point3 { x: f32, y: f32, z: f32 }
    record point3-with-width {
        x: f32, y: f32, z: f32,
        width: f32,         // local extrusion width in mm
        flow-factor: f32,   // multiplier on base extrusion volume
    }
    record bounding-box2 { min: point2, max: point2 }
    record bounding-box3 { min: point3, max: point3 }
    record polygon       { points: list<point2> }
    record ex-polygon    { contour: polygon, holes: list<polygon> }

    record extrusion-path-3d {
        points: list<point3-with-width>,
        role: extrusion-role,
        speed-factor: f32,
    }

    enum extrusion-role {
        outer-wall, inner-wall, thin-wall,
        top-solid-infill, bottom-solid-infill, sparse-infill,
        support-material, support-interface,
        ironing, bridge-infill, wipe-tower, custom,
    }

    record semver { major: u32, minor: u32, patch: u32 }
}
```

---

## `deps/config.wit`

```wit
package slicer:config@1.0.0;

interface config-types {
    variant config-value {
        bool-val(bool),
        int-val(s64),
        float-val(f64),
        string-val(string),
        float-list(list<f64>),
        string-list(list<string>),
    }

    // Read-only, pre-filtered to declared reads only.
    resource config-view {
        get:        func(key: string) -> option<config-value>;
        get-bool:   func(key: string) -> option<bool>;
        get-float:  func(key: string) -> option<f64>;
        get-int:    func(key: string) -> option<s64>;
        get-string: func(key: string) -> option<string>;
        keys:       func() -> list<string>;
    }
}
```

---

## `deps/ir-types.wit`

```wit
package slicer:ir-types@1.0.0;

interface ir-handles {
    use slicer:types/geometry.{ex-polygon, extrusion-path-3d, point3, semver};

    type object-id = string;
    type region-id = string;
    type layer-idx = u32;

    record region-key { layer-index: layer-idx, object-id: object-id, region-id: region-id }

    record wall-feature-flag {
        tool-index: option<u32>,
        fuzzy-skin: bool,
        is-bridge: bool,
        is-thin-wall: bool,
        skip-ironing: bool,
    }

    record wall-loop-view {
        perimeter-index: u32,
        loop-type: wall-loop-type,
        path: extrusion-path-3d,
        /// Parallel to path.points. Segment i -> i+1 uses feature-flags[i].
        feature-flags: list<wall-feature-flag>,
    }
    enum wall-loop-type { outer, inner, thin-wall, nonplanar-shell }

    enum paint-semantic {
        material,
        fuzzy-skin,
        support-enforcer,
        support-blocker,
        custom,             // module queries by custom-id string separately
    }

    variant paint-value {
        flag(bool),
        scalar(f32),
        tool-index(u32),
    }

    record boundary-paint-polygon {
        values: list<option<paint-value>>,
    }

    record boundary-paint-entry {
        semantic: paint-semantic,
        polygons: list<boundary-paint-polygon>,
    }

    // ── Read-only IR view resources ──────────────────────────────────────
    // Host constructs these. Modules cannot construct them.

    resource slice-region-view {
        object-id:              func() -> object-id;
        region-id:              func() -> region-id;
        polygons:               func() -> list<ex-polygon>;
        infill-areas:           func() -> list<ex-polygon>;
        effective-layer-height: func() -> f32;
        z:                      func() -> f32;
        has-nonplanar:          func() -> bool;
        boundary-paint:         func() -> list<boundary-paint-entry>;
    }

    resource perimeter-region-view {
        object-id:    func() -> object-id;
        region-id:    func() -> region-id;
        wall-loops:   func() -> list<wall-loop-view>;
        infill-areas: func() -> list<ex-polygon>;
    }

    // ── Mutable output builder resources ────────────────────────────────
    // Host validates all writes against declared ir-access.writes at call time.

    resource infill-output-builder {
        push-sparse-path:  func(path: extrusion-path-3d) -> result<_, string>;
        push-solid-path:   func(path: extrusion-path-3d) -> result<_, string>;
        push-ironing-path: func(path: extrusion-path-3d) -> result<_, string>;
    }

    resource perimeter-output-builder {
        push-wall-loop:      func(loop-: wall-loop-view) -> result<_, string>;
        set-infill-areas:    func(areas: list<ex-polygon>) -> result<_, string>;
        push-seam-candidate: func(pos: point3, score: f32) -> result<_, string>;
    }

    resource slice-postprocess-builder {
        set-polygons: func(region: region-key, polys: list<ex-polygon>) -> result<_, string>;
        set-path-z:   func(region: region-key, path-idx: u32, vertex-idx: u32, z: f32) -> result<_, string>;
    }

    resource gcode-output-builder {
        push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
        push-retract:     func(length: f32, speed: f32) -> result<_, string>;
        push-fan-speed:   func(value: u8) -> result<_, string>;
        push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
        push-tool-change: func(from: u32, to: u32) -> result<_, string>;
        push-comment:     func(text: string) -> result<_, string>;
        push-raw:         func(text: string) -> result<_, string>;
    }

    use slicer:types/geometry.{extrusion-role};
    record gcode-move-cmd {
        x: option<f32>, y: option<f32>, z: option<f32>,
        e: option<f32>, f: option<f32>,
        role: extrusion-role,
    }

    resource support-output-builder {
        push-support-path:   func(path: extrusion-path-3d) -> result<_, string>;
        push-interface-path: func(path: extrusion-path-3d, is-top-interface: bool) -> result<_, string>;
        push-raft-path:      func(path: extrusion-path-3d) -> result<_, string>;
    }

    // ── Paint region views (read-only) ──────────────────────────────────────
    // Modules query these by semantic. The host returns only regions for
    // semantics the module declared in its ir-access.reads.

    record semantic-region {
        object-id: object-id,
        polygons:  list<ex-polygon>,
        value:     paint-value,
    }

    resource paint-region-layer-view {
        /// Returns all regions for the given semantic at this layer.
        /// Empty list if no paint of this semantic exists at this layer.
        get-regions: func(semantic: paint-semantic) -> list<semantic-region>;

        /// For Custom semantics — query by the registering module ID string.
        get-custom-regions: func(module-id: string) -> list<semantic-region>;

        layer-index: func() -> layer-idx;
    } 
}
```

ID canonicalization note:

- `region-id` string must carry canonical decimal `u64` representation from host IR.
- Modules must treat IDs as opaque tokens and must not derive ordering from lexical string comparison.

### Wall Loop Flag Invariant

- `wall-loop-view.feature-flags` is required and must remain parallel to `path.points`.
- For segment-level behavior, segment `i -> i+1` reads `feature-flags[i]`.
- Host-side debug validation should enforce `feature-flags.len() == path.points.len()` at module boundaries.
- A module that changes path cardinality must also update `feature-flags` cardinality in the same write.

---

## `host-api.wit`

```wit
package slicer:host-api@1.0.0;

interface host-services {
    use slicer:types/geometry.{point3, bounding-box3, ex-polygon, polygon};
    use slicer:ir-types/ir-handles.{object-id};

    enum log-level { trace, debug, info, warn, error }
    log: func(level: log-level, message: string);

    // Mesh queries — no mesh data crosses the WASM boundary.
    raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
    surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
    object-bounds:      func(object-id: object-id) -> bounding-box3;

    // Geometry utilities — delegate to host-side Clipper2.
    // Modules should not bundle their own Clipper instance.
    enum clip-operation   { union, intersection, difference, xor }
    enum offset-join-type { miter, round, square }

    clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
    offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
    simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;

    // Monotonic timestamp in microseconds for profiling.
    now-us: func() -> u64;
}
```

---

## `world-layer.wit`

```wit
package slicer:world-layer@1.0.0;

world layer-module {
    import slicer:host-api/host-services;
    import slicer:config/config-types.{config-view};
    import slicer:ir-types/ir-handles.{
        slice-region-view,
        perimeter-region-view,
        infill-output-builder,
        perimeter-output-builder,
        slice-postprocess-builder,
        gcode-output-builder,
        region-key,
        layer-idx,
        paint-region-layer-view,
    };

    record module-error { code: u32, message: string, fatal: bool }

    // Lifecycle — optional
    export on-print-start: func(config: config-view) -> result<_, module-error>;
    export on-print-end:   func() -> result<_, module-error>;

    // Stage exports — implement exactly one matching your declared stage.
    // The host rejects a module that exports a function mismatching its manifest stage.

    export run-slice-postprocess: func(
        layer-index:  layer-idx,
        regions:      list<slice-region-view>,
        paint:        paint-region-layer-view,
        output:       slice-postprocess-builder,
        config:       config-view,
    ) -> result<_, module-error>;

    export run-perimeters: func(
        layer-index:  layer-idx,
        regions:      list<slice-region-view>,
        paint:        paint-region-layer-view,
        output:       perimeter-output-builder,
        config:       config-view,
    ) -> result<_, module-error>;

    export run-wall-postprocess: func(
        layer-index: layer-idx,
        regions: list<perimeter-region-view>,
        output: perimeter-output-builder,
        config: config-view,
    ) -> result<_, module-error>;

    export run-infill: func(
        layer-index: layer-idx,
        regions: list<slice-region-view>,
        output: infill-output-builder,
        config: config-view,
    ) -> result<_, module-error>;

    export run-infill-postprocess: func(
        layer-index: layer-idx,
        regions: list<perimeter-region-view>,
        output: infill-output-builder,
        config: config-view,
    ) -> result<_, module-error>;

    export run-support: func(
        layer-index:  layer-idx,
        regions:      list<slice-region-view>,
        paint:        paint-region-layer-view,   // enforcer/blocker regions
        output:       support-output-builder,
        config:       config-view,
    ) -> result<_, module-error>;
}
```

---

## `world-prepass.wit`

```wit
package slicer:world-prepass@1.0.0;

world prepass-module {
    import slicer:host-api/host-services;
    import slicer:config/config-types.{config-view};
    use slicer:ir-types/ir-handles.{object-id, region-id};

    record module-error { code: u32, message: string, fatal: bool }

    // MeshAnalysis stage
    enum facet-class { normal, near-horizontal, overhang, bridge, top-surface, bottom-surface }
    record facet-annotation { facet-index: u32, slope-angle-deg: f32, classification: facet-class }
    record surface-group-proposal { facet-indices: list<u32>, z-min: f32, z-max: f32, shell-count: u32 }

    resource mesh-analysis-output {
        push-facet-annotation: func(obj: object-id, ann: facet-annotation) -> result<_, string>;
        push-surface-group:    func(obj: object-id, grp: surface-group-proposal) -> result<_, string>;
    }

    export run-mesh-analysis: func(
        objects: list<object-id>,
        output: mesh-analysis-output,
        config: config-view,
    ) -> result<_, module-error>;

    // LayerPlanning stage
    record region-layer-proposal {
        object-id: object-id, region-id: region-id,
        effective-layer-height: f32,
        is-catchup: bool, catchup-z-bottom: f32,
    }
    record layer-proposal { z: f32, active-regions: list<region-layer-proposal> }

    resource layer-plan-output {
        push-layer: func(proposal: layer-proposal) -> result<_, string>;
    }

    export run-layer-planning: func(
        objects: list<object-id>,
        output: layer-plan-output,
        config: config-view,
    ) -> result<_, module-error>;
}
```

---

## `world-postpass.wit`

```wit
package slicer:world-postpass@1.0.0;

world postpass-module {
    import slicer:host-api/host-services;
    import slicer:config/config-types.{config-view};
    import slicer:ir-types/ir-handles.{gcode-output-builder, gcode-move-cmd};

    record module-error { code: u32, message: string, fatal: bool }

    enum gcode-command-kind { move_, retract, fan-speed, temperature, tool-change, comment, raw }
    record gcode-command-view { index: u32, kind: gcode-command-kind }

    export run-gcode-postprocess: func(
        commands: list<gcode-command-view>,
        output: gcode-output-builder,
        config: config-view,
    ) -> result<_, module-error>;

    // Last-resort text mutation. Single-threaded. Use only when GCodeIR is insufficient.
    export run-text-postprocess: func(
        gcode-text: string,
        config: config-view,
    ) -> result<string, module-error>;
}
```

---

## `world-finalization.wit`

```wit
package slicer:world-finalization@1.0.0;

world finalization-module {
    import slicer:host-api/host-services;
    import slicer:config/config-types.{config-view};
    use slicer:ir-types/ir-handles.{
        layer-idx, extrusion-path-3d, region-key,
    };

    record module-error { code: u32, message: string, fatal: bool }

    record tool-change-view {
        after-entity-index: u32,
        from-tool: u32,
        to-tool: u32,
    }

    // Read-only view of one completed layer.
    resource layer-collection-view {
        layer-index:  func() -> layer-idx;
        z:            func() -> f32;
        entity-count: func() -> u32;
        tool-changes: func() -> list<tool-change-view>;
    }

    // Output builder — may append to existing layers or insert synthetic ones.
    resource finalization-output-builder {
        // Append extrusion paths to an existing layer.
        push-entity-to-layer: func(
            layer-index: layer-idx,
            path: extrusion-path-3d,
            region-key: region-key,
        ) -> result<_, string>;

        // Insert a new synthetic layer at an arbitrary Z.
        // The host merges synthetic layers into the final sequence
        // sorted by Z before PostPass::GCodeEmit runs.
        insert-synthetic-layer: func(
            z: f32,
            paths: list<extrusion-path-3d>,
        ) -> result<_, string>;
    }

    export run-finalization: func(
        layers: list<layer-collection-view>,
        output: finalization-output-builder,
        config: config-view,
    ) -> result<_, module-error>;
}
```

---

## Module Manifest Schema (TOML)

Full annotated example for a TPMS infill module:

```toml
# ── Identity ────────────────────────────────────────────────────────────────
[module]
id           = "com.community.tpms-infill"  # reverse-domain, globally unique
version      = "1.2.0"                       # semver
display-name = "TPMS Infill"
description  = "Schwartz-D and Fischer-Koch-S triply periodic minimal surface infill"
author       = "community"
license      = "MIT"
homepage     = "https://github.com/example/tpms-infill"
wit-world    = "slicer:world-layer@1.0.0"   # must match an installed WIT world

# ── Stage declaration ────────────────────────────────────────────────────────
# Exactly one stage per module. Two stages = two .wasm files.
[stage]
id = "Layer::Infill"

# ── IR access ────────────────────────────────────────────────────────────────
# Host enforces at runtime. Undeclared reads return empty/none. Undeclared writes are trapped.
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
[[config.cross-validate]]
rule     = "marching-cell-size >= raster-precision * 10"
message  = "Marching cell size should be at least 10x the raster precision"
severity = "warning"    # "error" blocks slicing; "warning" notifies only

# ── Per-region / per-layer override policy ────────────────────────────────────
[config.overridable-per-region]
keys = ["pattern", "density", "multiline-count"]

[config.overridable-per-layer]
keys = ["density"]      # density can vary per-layer; pattern cannot

# ── Hints ─────────────────────────────────────────────────────────────────────
[hints]
estimated-ms-per-layer = 12    # for ETA estimation
# layer-parallel-safe must be false for PostPass::LayerFinalization modules.
# The host emits a startup warning if a finalization module sets this to true.
# All other stages: true allows the host to run multiple layers simultaneously.
layer-parallel-safe    = true  
```

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
- In WIT v1.0.x guests **cannot** reorder, append to, or remove
  entries from `ordered_entities`. The pre-staged sequence is final for the
  lifetime of the layer. `topo_order` indices are stable and used as the
  `after_entity_index` keying domain for tool-changes and annotations.
- Reordering / mutation of `ordered_entities` is reserved for a future
  `layer-collection-builder` resource. Until that resource lands, any guest
  that needs deterministic reordering must do it earlier (during `Layer::Perimeters`
  / `Layer::Infill` commit ordering) — not in `Layer::PathOptimization`.

### Accepted `gcode-output-builder` methods at PathOptimization

| Method                        | Accepted? | Commit destination                                                                                                  |
|-------------------------------|-----------|---------------------------------------------------------------------------------------------------------------------|
| `push-tool-change(from, to)`  | yes       | Appended to `LayerCollectionIR.tool_changes` with `after_entity_index = ordered_entities.len() - 1` (or 0 if empty) |
| `push-comment(text)`          | yes       | Appended to `LayerCollectionIR.annotations` as `Comment(text)` with the same anchor rule                            |
| `push-raw(text)`              | yes       | Appended to `LayerCollectionIR.annotations` as `Raw(text)` with the same anchor rule                                |
| `push-move(cmd)`              | rejected  | Fatal `FatalModule` diagnostic — no documented `LayerCollectionIR` mapping                                          |
| `push-retract(length, speed)` | rejected  | Fatal `FatalModule` diagnostic                                                                                      |
| `push-fan-speed(value)`       | rejected  | Fatal `FatalModule` diagnostic                                                                                      |
| `push-temperature(...)`       | rejected  | Fatal `FatalModule` diagnostic                                                                                      |

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

This is the single, minimal z-hop output channel. Reordering of
`ordered_entities` and a generalised `layer-collection-builder` resource
remain reserved for a later step.

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
- Reordering, appending to, or removing entries from `ordered_entities` is
  still rejected and is reserved for a future step.

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

## Builder Lifecycle Contract (Normative)

Output builder resources (`*-output-builder`) are invocation-scoped:

- Valid only during the active exported function call.
- Must not be cached across calls by the module.
- Host invalidates/drops builders immediately after return.

Attempting to reuse an invalidated builder is a fatal contract error.

## Valid Reads/Writes

### Paint region reads (declare the semantics your module needs)

reads = [
    "PaintRegionIR.FuzzySkin",          # fuzzy skin module
    "PaintRegionIR.SupportEnforcer",    # support generator
    "PaintRegionIR.SupportBlocker",     # support generator
    "PaintRegionIR.Material",           # material/tool assignment
    "PaintRegionIR.Custom.com.example.my-semantic",  # custom semantic
]

### Boundary paint on slice regions

reads = ["SliceIR.regions.boundary_paint"]

### Wall feature flags

reads  = ["PerimeterIR.regions.walls.feature_flags"]   # fuzzy skin post-processor
writes = ["PerimeterIR.regions.walls.feature_flags"]   # if modifying flags

---

## Config Field Types Reference

| Type            | Description                | Extra keys                   |
|-----------------|----------------------------|------------------------------|
| `"bool"`        | Boolean checkbox           | —                            |
| `"int"`         | Integer                    | `min`, `max`, `step`         |
| `"float"`       | Floating point             | `min`, `max`, `step`, `unit` |
| `"string"`      | Free text                  | `max-length`                 |
| `"enum"`        | Fixed set of string values | `values` (required)          |
| `"float-list"`  | List of floats             | `min-length`, `max-length`   |
| `"string-list"` | List of strings            | `min-length`, `max-length`   |

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

```
Literals:   0, 1.5, true, false, "string"
References: value (single-field), field-name (cross-validate)
Operators:  && || ! == != < <= > >= + - * /
Functions:  min(a,b)  max(a,b)  abs(x)  floor(x)  ceil(x)
```

Examples:

```
validate = "value >= 0.01 && value <= 10.0"
rule     = "outer_wall_speed <= inner_wall_speed * 1.5"
rule     = "min(layer_height, 0.35) == layer_height"
```
