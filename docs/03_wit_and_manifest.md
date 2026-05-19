# ModularSlicer — WIT Interfaces & Module Manifest Schema

> **Source of truth.** The on-disk `wit/*.wit` files in this repo are the
> authoritative WIT contract. The WIT code blocks reproduced in this document
> are derived for reading convenience and have been observed to drift behind
> the on-disk schema (e.g. new record fields, additional resource methods,
> renamed enum variants). When the doc and `wit/` disagree, `wit/` wins; treat
> the doc divergence as a bug to be filed against this document.
>
> Likewise, the TOML manifest schema in this document is the parsed surface
> recognised by `crates/slicer-host/src/manifest.rs`. Sections or keys that
> appear here but are not read by the parser are noted inline with a
> `<!-- VERIFY: ... -->` tag.

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

    variant extrusion-role {
        outer-wall, inner-wall, thin-wall,
        top-solid-infill, bottom-solid-infill, sparse-infill,
        support-material, support-interface,
        ironing, bridge-infill, wipe-tower,
        custom(string),
    }

    record semver { major: u32, minor: u32, patch: u32 }
}
```

Built-in IR roles with no dedicated WIT case remain lossless at the boundary by
using reserved `custom(string)` tags:

- `PrimeTower` maps to `custom("slicer.builtin/prime-tower@1")`
- `Skirt` maps to `custom("slicer.builtin/skirt@1")`
- Third-party modules must not mint either reserved tag.

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
    /// **Signed** (packet 43-rev1): raft prefix layers use negative indices
    /// (`-1, -2, …, -raft_layers`). Negative `layer-index` arguments at host
    /// entry points outside raft contexts are rejected at the validator.
    type layer-idx = s32;

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
        object-id:       func() -> object-id;
        region-id:       func() -> region-id;
        wall-loops:      func() -> list<wall-loop-view>;
        infill-areas:   func() -> list<ex-polygon>;
        resolved-seam:   func() -> option<seam-position>;
    }

    // ── Mutable output builder resources ────────────────────────────────
    // Host validates all writes against declared ir-access.writes at call time.

    resource infill-output-builder {
        push-sparse-path:  func(path: extrusion-path-3d) -> result<_, string>;
        push-solid-path:   func(path: extrusion-path-3d) -> result<_, string>;
        push-ironing-path: func(path: extrusion-path-3d) -> result<_, string>;
    }

    resource perimeter-output-builder {
        push-wall-loop:          func(loop-: wall-loop-view) -> result<_, string>;
        /// **Cardinality constraint (packet 22):**
        /// `rotated_wall_loop.feature_flags.len() == rotated_wall_loop.path.points.len()`.
        /// The host rejects mismatched commits with `CARDINALITY_MISMATCH`. This
        /// invariant is required because rotation moves the seam to position 0
        /// and feature flags must rotate with the path.
        push-reordered-wall-loop: func(pos: point3-with-width, wall-index: u32, rotated-wall-loop: wall-loop-view) -> result<_, string>;
        set-infill-areas:        func(areas: list<ex-polygon>) -> result<_, string>;
        push-seam-candidate:     func(pos: point3, score: f32) -> result<_, string>;
        push-resolved-seam:      func(pos: point3, wall-index: u32) -> result<_, string>;
    }

    resource slice-postprocess-builder {
        set-polygons: func(region: region-key, polys: list<ex-polygon>) -> result<_, string>;
        set-path-z:   func(region: region-key, path-idx: u32, vertex-idx: u32, z: f32) -> result<_, string>;
    }

    resource gcode-output-builder {
        push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
        // `mode: retract-mode` selects parameterised G1 E (Gcode mode) vs
        // parameterless G10/G11 (Firmware mode); see packet 34.
        push-retract:     func(length: f32, speed: f32, mode: retract-mode) -> result<_, string>;
        push-unretract:   func(length: f32, speed: f32, mode: retract-mode) -> result<_, string>;
        push-fan-speed:   func(value: u8) -> result<_, string>;
        push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
        // `after-entity-index` anchors the tool change to a specific entity
        // position in the layer's ordered_entities (see `LayerCollectionIR.tool_changes`).
        push-tool-change: func(after-entity-index: u32, from-tool: u32, to-tool: u32) -> result<_, string>;
        push-comment:     func(text: string) -> result<_, string>;
        push-raw:         func(text: string) -> result<_, string>;
        push-z-hop:       func(after-entity-index: u32, hop-height: f32) -> result<_, string>;
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
- Any non-canonical `region-id` observed at a module boundary is a fatal contract error.
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

### `layer-collection-builder` resource (packet 32)

Available to `Layer::PathOptimization` modules. Replaces the previous reserved-future placeholder.

**Source of truth:** `wit/deps/ir-types.wit`. The resource and the
`ordered-entity-view` record are defined there.

```wit
resource layer-collection-builder {
    // Returns `result<_, string>` so commit-time validation surfaces as an error.
    set-entity-order:     func(items: list<tuple<u32, bool>>) -> result<_, string>;
    get-ordered-entities: func() -> list<ordered-entity-view>;
}

// Confirm the exact field set against the on-disk WIT; the record currently
// carries fields such as `original-index`, `region-key`, `role`, `start-point`,
// `end-point`, and `point-count` (the older `entity-index` field was renamed
// to `original-index`).
```

`set-entity-order` accepts `(entity-index, reverse-direction)` tuples. Setting `reverse-direction = true` flips the path's point order at apply time. Host rejects entries that reference unknown `entity-index` values or include duplicates; either condition produces a `BuilderError::InvalidEntityOrder` diagnostic.

PathOptimization output contract restricts builder usage to this resource and the existing `push-tool-change` / `push-comment` / `push-raw` methods. `push-move` / `push-retract` / `push-unretract` / `push-fan-speed` / `push-temperature` remain rejected at the host boundary (see Path Optimization Output Contract below).

---

## `world-prepass.wit`

```wit
package slicer:world-prepass@1.0.0;

world prepass-module {
    import slicer:host-api/host-services;
    import slicer:config/config-types.{config-view};
    use slicer:ir-types/ir-handles.{object-id, region-id, mesh-object-view, paint-segmentation-object-view};

    record module-error { code: u32, message: string, fatal: bool }

    // MeshSegmentation stage
    resource mesh-segmentation-output {
        mark-triangle-paint: func(obj: object-id, facet-index: u32, semantic: string, value: string) -> result<_, string>;
    }

    export run-mesh-segmentation: func(
        objects: list<mesh-object-view>,
        output: mesh-segmentation-output,
        config: config-view,
    ) -> result<_, module-error>;

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

    // PaintSegmentation stage
    use slicer:ir-types/ir-handles.{layer-idx};
    use slicer:types/geometry.{ex-polygon};

    record paint-region-entry {
        object-id: object-id,
        layer-index: layer-idx,
        semantic: string,
        polygons: list<ex-polygon>,
        // `paint-value-input` is a typed variant — see `wit/deps/ir-types.wit`
        // for its definition: `flag(bool) | scalar(f32) | tool-index(u32) | custom(string)`.
        value: paint-value-input,
    }

    resource paint-segmentation-output {
        push-paint-region: func(entry: paint-region-entry) -> result<_, string>;
    }

    export run-paint-segmentation: func(
        objects: list<paint-segmentation-object-view>,
        output: paint-segmentation-output,
        config: config-view,
    ) -> result<_, module-error>;

    // SeamPlanning stage
    use slicer:types/geometry.{point3-with-width};

    record seam-reason { tag: string }
    record scored-seam-candidate {
        position: point3-with-width,
        score: f32,
        reason: seam-reason,
    }
    record seam-plan-entry {
        global-layer-index: layer-idx,
        object-id: object-id,
        region-id: region-id,
        chosen-position: point3-with-width,
        chosen-wall-index: u32,
        scored-candidates: list<scored-seam-candidate>,
    }

    resource seam-planning-output {
        push-seam-plan: func(entry: seam-plan-entry) -> result<_, string>;
    }

    export run-seam-planning: func(
        objects: list<mesh-object-view>,
        output: seam-planning-output,
        config: config-view,
    ) -> result<_, module-error>;

    // SupportGeometry stage
    // Multi-layer organic tree-support planning. Walks layers top-to-bottom,
    // groups overhang/enforcer contacts via per-layer Prim MST, and emits
    // per-(layer, object, region) branch geometry consumed directly by
    // Layer::Support modules that declare SupportPlanIR as a read.
    //
    // **Source of truth:** `wit/world-prepass.wit`. The on-disk signature of
    // `run-support-geometry` takes `(objects, layer-plan: layer-plan-view,
    // region-segmentation: region-segmentation-view,
    // support-geometry: support-geometry-view)` and returns
    // `support-geometry-output` directly (not `result<_, module-error>`).
    // `support-geometry-output` is a **record**, not a resource. The
    // accompanying records `layer-plan-view`, `region-segmentation-view`,
    // and `support-geometry-view` are defined in the on-disk WIT.
}
```

---

## `world-postpass.wit`

```wit
package slicer:world-postpass@1.0.0;

world postpass-module {
    import slicer:host-api/host-services;
    import slicer:config/config-types.{config-view};
    import slicer:ir-types/ir-handles.{gcode-output-builder, gcode-move-cmd, retract-mode};

    record module-error { code: u32, message: string, fatal: bool }

    // `mode: retract-mode` selects parameterised G1 E vs G10/G11; see packet 34.
    record gcode-retract-cmd { length: f32, speed: f32, mode: retract-mode }
    record gcode-fan-speed-cmd { value: u8 }
    record gcode-temperature-cmd { tool: u32, celsius: f32, wait: bool }
    record gcode-tool-change-cmd { from-tool: u32, to-tool: u32 }

    variant gcode-command {
        move(gcode-move-cmd),
        retract(gcode-retract-cmd),
        unretract(gcode-retract-cmd),
        fan-speed(gcode-fan-speed-cmd),
        temperature(gcode-temperature-cmd),
        tool-change(gcode-tool-change-cmd),
        comment(string),
        raw(string),
    }

    export run-gcode-postprocess: func(
        commands: list<gcode-command>,
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

**Source of truth:** `wit/world-finalization.wit`. The shape below summarises
the world; for exact field order, parameter names, and return types, read the
on-disk file.

The `finalization-module` world exposes a single export
`run-finalization(layers, output, config) -> result<_, module-error>`. It
imports `slicer:host-api/host-services`, `slicer:config/config-types.{config-view}`,
and uses `slicer:ir-types/ir-handles.{layer-idx, extrusion-path-3d, region-key}`
plus `slicer:types/geometry.{extrusion-role}`.

Resources, records, and enums (current at time of writing — confirm against
`wit/world-finalization.wit`):

- `layer-collection-view` — read-only view of one completed layer:
  `layer-index() -> layer-idx`, `z() -> f32`, `entity-count() -> u32`,
  `ordered-entities() -> list<print-entity-view>`,
  `tool-changes() -> list<tool-change-view>`,
  `z-hops() -> list<z-hop-view>`.
- `print-entity-view` (record): `entity-id: u64`, `path: extrusion-path-3d`,
  `role: extrusion-role`, `region-key: region-key`, `topo-order: u32`.
  The `entity-id` is the stable per-layer ID from packet 39 (see
  `docs/02_ir_schemas.md` IR 10).
- `tool-change-view` (record): `after-entity-index: u32`, `from-tool: u32`,
  `to-tool: u32`.
- `z-hop-view` (record): `after-entity-index: u32`, `hop-height: f32`.
- `finalization-output-builder` (resource) — the mutation API:
  - `push-entity-to-layer(layer-index, path, region-key) -> result<_, string>`
  - `push-entity-with-priority(layer-index, path, region-key, priority) -> result<_, string>`
    — note `extrusion-path-3d` already carries the role; there is no separate `role` parameter.
  - `modify-entity(layer-index, entity-id, mutation) -> result<_, string>`
  - `sort-layer-by(layer-index, key) -> result<_, string>`
  - `insert-synthetic-layer(z, paths) -> result<_, string>` and
    `insert-synthetic-layer-after(idx, layer-data) -> result<_, string>`
- `entity-mutation` (variant) — packet 41 enum-serialisable mutations.
  Confirm the current variant set against `wit/world-finalization.wit`; at the
  time of writing it is a narrow set rather than the speculative six-variant
  enum some older drafts of this doc described.
- `sort-key` (enum, not variant) — sort discriminators consumed by
  `sort-layer-by`. Names follow the form `by-<…>`; read the on-disk file for
  the current set.
- `synthetic-layer-data` (record) — `z: f32`, `paths: list<extrusion-path-3d>`.

Host validation: the host validates that `entity-id` in `modify-entity`
resolves to a real entity within `layer`; unknown IDs are rejected with
`BuilderError::UnknownEntity`. The closure-based API from packet 40 is
superseded by the enum-based mutation API so the contract is fully
serialisable across the WIT boundary.

**Positional insertion and permutation (Packet 58, 2026-05-18)**:
`finalization-output-builder` exposes three additional methods that mirror PathOptimization's `layer-collection-builder` capability surface:

- `insert-entity-at(layer-index, position: u32, path, region-key) -> result<_, string>` — inserts an entity at a specific position in the layer's `ordered_entities` list. On apply, `ToolChange.after_entity_index >= position` and `ZHop.after_entity_index >= position` are each incremented by 1 to preserve their positional references. Out-of-bounds position returns `Err` with no mutation.
- `set-entity-order(layer-index, items: list<tuple<u32, bool>>) -> result<_, string>` — permutes the layer's entities by the supplied index list (one entry per existing entity; the boolean is a reverse flag). On apply, `ToolChange.after_entity_index` and `ZHop.after_entity_index` are remapped through the inverse permutation. Malformed proposals (length mismatch, duplicates, out-of-range indices) return `Err` with no mutation.
- `get-ordered-entities(layer-index) -> list<print-entity-view>` — returns the staged state of the layer's `ordered_entities`. The SDK path observes both completed and in-flight builder state; the host-side WIT impl currently returns the pre-apply layer snapshot only (in-flight pushes are not reflected until `apply_to` runs). Module authors who need the staged state during the same `run_finalization` call should rely on the SDK side; the host accessor is a snapshot of pre-existing entities.

The index-remap invariants are owned by the SDK's `apply_to` (`crates/slicer-sdk/src/traits.rs::FinalizationOutputBuilder::apply_to`); modules must not pre-adjust indices themselves. `wipe-tower` uses `insert-entity-at(layer, tc.after_entity_index + 1 + offset, ...)` to bracket each `T<n>` with retract + travel + prime + wipe entities.

---

## Module Manifest Schema (TOML)

Full annotated example for a TPMS infill module:

```toml
# ── Identity ────────────────────────────────────────────────────────────────
# The host parser (`crates/slicer-host/src/manifest.rs`) currently reads only
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
wit-world    = "slicer:world-layer@1.0.0"   # parsed; must match an installed WIT world

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

### Known claim IDs

| Claim ID                  | Purpose                                                                   |
|---------------------------|---------------------------------------------------------------------------|
| `perimeter-generator`     | Held by the module producing wall loops on a given region.                |
| `infill-generator`        | Held by the module producing infill paths on a given region.              |
| `support-generator`       | Held by the module producing support extrusions on a given layer/region.  |
| `support-planner`         | Held by the PrePass module emitting `SupportPlanIR`.                      |
| `seam-placer`             | Held by the module placing seam candidates and resolving seam positions.  |
| `layer-planner`           | Held by the module proposing layer Z heights and active-region lists.     |
| `mesh-analyzer`           | Held by the module annotating facets and proposing surface groups.        |
| `slice-postprocessor`     | Held by a module that mutates `SliceIR` polygons after initial slicing.   |
| `gcode-postprocessor`     | Held by a PostPass module that processes the `GCodeCommand` stream.       |
| `text-postprocessor`      | Held by a PostPass module that mutates the final G-code text string.      |
| `claim:top-fill`          | Held by the module producing `TopSolidInfill` extrusions on this layer.  |
| `claim:bottom-fill`       | Held by the module producing `BottomSolidInfill` extrusions.             |
| `claim:bridge-fill`       | Held by the module producing `BridgeInfill` extrusions.                  |
| `claim:sparse-fill`       | Held by the module producing `SparseInfill` extrusions.                  |

The four fill-role claims (`claim:top-fill` … `claim:sparse-fill`) were added in packet 37. A single module may hold multiple fill-role claims (e.g. `rectilinear-infill` holds all four by default). Claim-conflict validation runs in DAG validation pass 2; per-region overrides may transfer a fill-role claim to a different module.

The configured holder per claim is selected by four `ResolvedConfig` keys —
`top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder`,
`sparse_fill_holder` — each defaulting to `"rectilinear-infill"`. Per-region
overrides flow through `RegionMapIR.entries[*].config` (reused from
packet 35). At dispatch time the host computes the effective held set per
region by intersecting each module's manifest `[claims].holds` with the
configured holders (see `slicer_host::resolve_held_claims`).

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

### Configuration keys added by recent packets

The following `[config.schema.<key>]` blocks document config keys introduced after the TPMS annotated example above. Keys follow the snake_case convention throughout (see CLAUDE.md).

#### Packet 34 — retraction mode

```toml
[config.schema.retraction_mode]
type    = "enum"
values  = ["gcode", "firmware"]
default = "gcode"
display = "Retraction mode (G1 E moves vs G10/G11 firmware codes)"
group   = "Extruder"
```

`"gcode"` emits standard `G1 E<n> F<speed>` retract/unretract moves. `"firmware"` emits `G10` (retract) / `G11` (unretract). M207/M208 are intentionally never emitted regardless of mode.

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

Units and defaults mirror `docs/02_ir_schemas.md` "Polyline simplification and precision" subsection.

```toml
[config.schema.gcode_resolution]
type    = "float"
default = 0.0125
min     = 0.0
unit    = "mm"
display = "G-code resolution (wall/brim D-P tolerance)"
group   = "Advanced"
advanced = true

[config.schema.infill_resolution]
type    = "float"
default = 0.0125
min     = 0.0
unit    = "mm"
display = "G-code resolution (infill D-P tolerance)"
group   = "Advanced"
advanced = true

[config.schema.support_resolution]
type    = "float"
default = 0.05
min     = 0.0
unit    = "mm"
display = "G-code resolution (support D-P tolerance)"
group   = "Advanced"
advanced = true

[config.schema.min_segment_length]
type    = "float"
default = 0.025
min     = 0.0
unit    = "mm"
display = "Minimum segment length after simplification"
group   = "Advanced"
advanced = true

[config.schema.gcode_xy_decimals]
type    = "int"
default = 3
min     = 1
max     = 6
display = "G-code XY decimal places"
group   = "Advanced"
advanced = true

[config.schema.perimeter_arc_tolerance]
type    = "float"
default = 0.0025
min     = 0.0
unit    = "mm"
display = "Clipper2 arc tolerance for perimeter offsets"
group   = "Advanced"
advanced = true

[config.schema.slice_closing_radius]
type    = "float"
default = 0.0
min     = 0.0
unit    = "mm"
display = "Slice closing radius (0 = disabled)"
group   = "Advanced"
advanced = true
```

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

```
--thumbnail <PATH>      # PNG; Base64-encoded into THUMBNAIL_BLOCK_*
                        # CLI flag wins over thumbnail_path config when both set.
```

#### Packet 31b — tree-support OrcaSlicer parity

The following nine keys map directly to OrcaSlicer keys of the same name.

```toml
[config.schema.tree_support_branch_angle]
type    = "float"
default = 40.0
unit    = "deg"
display = "Tree support branch angle"
group   = "Support"

[config.schema.tree_support_branch_diameter]
type    = "float"
default = 2.0
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
default = 2
min     = 0
display = "Support interface bottom layers"
group   = "Support"

[config.schema.tree_support_interface_spacing_mm]
type    = "float"
default = 0.2
unit    = "mm"
display = "Tree support interface spacing"
group   = "Support"
```

### Per-paint-region config overrides (packet 51)

The namespace `paint_config:<semantic>:<key>` is recognised at module-load time as a per-paint-region config override.

Built-in `PaintSemantic` variants serialise as: `material`, `fuzzy_skin`, `support_enforcer`, `support_blocker`. `PaintSemantic::Custom(s)` uses the inner string verbatim as the `<semantic>` segment.

Override precedence (lowest → highest):

```
global < object_config:<id>:<key> < paint_config:<semantic>:<key>
```

The audit trail for applied paint overrides surfaces in `RegionMapIR.paint_overrides` (see `docs/02_ir_schemas.md` IR 5).

### Per-object config overrides (packet 35a)

Per-object overrides use the namespace `object_config:<id>:<key>`. These flow through `RegionPlan.config: ResolvedConfig` and are stamped on every `RegionPlan` and `ActiveRegion` during the resolved-config builder stage added in packet 35a. The propagation path is: CLI JSON → per-object overlay → `ResolvedConfig` stamped per-region. See `docs/02_ir_schemas.md` IR 3 for the `ResolvedConfig` struct and IR 5 for `RegionMapIR.entries[*].config`.

### Machine start / end G-code emission (packet 59)

Module-owned machine start/end G-code is emitted by a designated module running at `PostPass::LayerFinalization`. The bundled implementation is `machine-gcode-default`; the audit boundary is the contract, not the module ID.

The module reads two config keys:

```toml
[config.schema.machine_start_gcode]
type    = "string"
default = ""
display = "Machine start G-code"
group   = "Machine"

[config.schema.machine_end_gcode]
type    = "string"
default = ""
display = "Machine end G-code"
group   = "Machine"
```

Both strings support macro expansion. Documented macros: `{first_layer_temperature}`, `{bed_temperature}`, `{filament_type}`, `{nozzle_diameter}`, `{tool_count}`, `{layer_count}`, `{print_time_estimate_s}`, `{x_max}`, `{y_max}`, `{z_max}`. Unknown macros are left as-is with a warning logged to the host diagnostics stream.

The module emits start-gcode before any layer entity and end-gcode after the last layer.

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

---

## Test Guest Fixtures (Informative)

`test-guests/` holds minimal WASM components used as fixtures by host
integration tests under `crates/slicer-host/tests/`. They exercise the
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

```
test-guests/
├── build-test-guests.sh                # build + freshness checker
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
| `prepass-guest`      | `slicer:world-prepass`      | PrePass exports (mesh segmentation/analysis, paint, seam, support)        |
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

`test-guests/build-test-guests.sh` builds every guest with
`cargo build --target wasm32-unknown-unknown --release` and runs
`wasm-tools component new` to produce the `.component.wasm` artifact.

- `./test-guests/build-test-guests.sh` — build any stale guests.
- `./test-guests/build-test-guests.sh --check` — verify only; exit 1
  if any source is newer than its artifact.

Freshness is enforced from the host workspace by
`crates/slicer-host/tests/guest_fixture_freshness_tdd.rs`, which fails
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
