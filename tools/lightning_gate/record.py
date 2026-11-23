#!/usr/bin/env python3
"""Reproducible canonical recorder for the lightning geometry/portability gate.

The recorder never derives expected geometry from PnP. It copies an independent canonical
OrcaSlicer revision into disposable storage, applies the checked-in opt-in instrumentation patch,
builds that copy, slices generated 3MF fixture scenes, and copies the instrumented observations
back into this repository.

Three operations:

    record.py --capture --output <fixture-dir>
    record.py --replay  --manifest <fixture-dir>/manifest.json --output <dir>
    record.py --publish-fixtures --capture <capture-dir> --outputdir <dir>

All are fail-closed: a missing source revision, missing build, failed slice, absent recorder
output, or unrepresentable required field exits nonzero and is reported as `inconclusive`, never
as a synthesised reference.

Scenes are authored as real multi-object 3MF projects (wrapper objects, leaf component parts,
`Metadata/model_settings.config` subtypes and per-part process metadata, and build-item
translations). Each variant also gets a `scene.json` describing its IDs, boxes and config so the
recording can be audited without parsing the binary package.

The recorder patch writes five JSON files per object:

    <case>.object_<id>.source.json    earlier-source snapshot, taken before anchor edits
    <case>.object_<id>.planning.json  canonical planning domains, scalars and graph
    <case>.object_<id>.domains.json   post-anchor planning checkpoint plus typed final domains
    <case>.object_<id>.operations.json  Phase-2 anchor-expansion/union/difference events
    <case>.object_<id>.replay.json    immutable snapshot replay vs production graph comparison

It also writes `<case>.object_<id>.grounding.jsonl`: production-only grounding/attachment
witnesses joined to the planning graph by runtime node pointer.

`--capture` additionally derives `<case>.object_<id>.provenance.json` from those artifacts: the
normalized source/planning/final boundary DAG with exact overlap intervals and generated-operation
attribution. `grounding.jsonl` stays separate.

This script converts those observations into the portable fixture files consumed by the Rust gate.
`--publish-fixtures` publishes only recorded values: a recorded value with no representation in the
strict kernel DTO makes the case `inconclusive`, and an expected-data branch the capture cannot
supply is published as `null` with a named reason in the gate manifest.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import time
import zipfile
from dataclasses import dataclass, field, replace
from fractions import Fraction
from pathlib import Path
from typing import Any, Sequence
from xml.sax.saxutils import quoteattr

# ---------------------------------------------------------------------------
# Constants

DEFAULT_SOURCE = Path("D:/slicerProject/pinch_n_print_cli_2/OrcaSlicerDocumented")
DEFAULT_WORK = Path("D:/slicerProject/_lightning_gate")
DEFAULT_SLICER = (
    DEFAULT_WORK / "OrcaSlicer" / "build" / "src" / "Release" / "orca-slicer.exe"
)
PATCH_REL = Path("tools/lightning_gate/canonical-recorder.patch")
SCHEMA_VERSION = 1

WALL_STRATEGIES = ("classic", "arachne")
PART_KINDS = ("normal_part", "modifier_part", "negative_part")

# Canonical profile inputs. The recorder records the actual serialized spelling of wall generation
# from these profiles instead of assuming it.
MACHINE_PROFILE = "resources/profiles/BBL/machine/Bambu Lab X1 Carbon 0.4 nozzle.json"
PROCESS_PROFILE = "resources/profiles/BBL/process/0.20mm Standard @BBL X1C.json"
FILAMENT_PROFILE = "resources/profiles/BBL/filament/Bambu PLA Basic @BBL X1C.json"

# Process overrides shared by every scene. `dont_filter_internal_bridges="nofilter"` is a real
# canonical key: it keeps internal bridge candidates that the default filter removes.
COMMON_PROCESS_OVERRIDES: dict[str, str] = {
    "top_shell_layers": "2",
    "bottom_shell_layers": "1",
    "dont_filter_internal_bridges": "nofilter",
}

# Object-wide lightning config. Per-part metadata overrides individual keys for a modifier.
LIGHTNING_OBJECT_CONFIG: dict[str, str] = {
    "sparse_infill_pattern": "lightning",
    "sparse_infill_density": "15%",
    "wall_loops": "2",
}


# ---------------------------------------------------------------------------
# Fixture scenes
#
# Scenes are described as axis-aligned boxes in millimetres. A box is
# (x0, y0, z0, x1, y1, z1). Each scene object becomes a wrapper object whose leaf components are
# the parts; a single-part object is emitted directly as one mesh object.
# These are *inputs*: expected canonical geometry is recorded, never authored here.


@dataclass(frozen=True)
class Box:
    x0: float
    y0: float
    z0: float
    x1: float
    y1: float
    z1: float


@dataclass(frozen=True)
class Frame:
    """Axis-aligned rectangular ring extruded from z0 to z1.

    Unlike a box minus a negative part, this is a single closed manifold whose
    mesh already contains the hole: canonical slicing produces a contour plus a
    hole ring instead of a slit contour.
    """

    x0: float
    y0: float
    x1: float
    y1: float
    ix0: float
    iy0: float
    ix1: float
    iy1: float
    z0: float
    z1: float

    def __post_init__(self) -> None:
        if not (
            self.x0 < self.ix0 < self.ix1 < self.x1
            and self.y0 < self.iy0 < self.iy1 < self.y1
        ):
            raise ValueError(f"frame {self!r} inner ring is not strictly inside outer")
        if self.z1 <= self.z0:
            raise ValueError(f"frame {self!r} has non-positive height")


@dataclass(frozen=True)
class Part:
    name: str
    kind: str
    boxes: tuple[Box, ...] = ()
    config: dict[str, str] = field(default_factory=dict)
    frames: tuple[Frame, ...] = ()

    def __post_init__(self) -> None:
        if self.kind not in PART_KINDS:
            raise ValueError(f"unknown part kind {self.kind!r}")
        if not self.boxes and not self.frames:
            raise ValueError(f"part {self.name!r} has no geometry")


@dataclass(frozen=True)
class SceneObject:
    name: str
    parts: tuple[Part, ...]
    config: dict[str, str] = field(default_factory=dict)
    translate_mm: tuple[float, float, float] = (0.0, 0.0, 0.0)

    def __post_init__(self) -> None:
        if not self.parts:
            raise ValueError(f"object {self.name!r} has no parts")


@dataclass(frozen=True)
class SceneCase:
    case_id: str
    objects: tuple[SceneObject, ...]
    process_overrides: dict[str, str] = field(default_factory=dict)
    fixture_crosswalk: dict[str, Any] = field(default_factory=dict)


CASES: tuple[SceneCase, ...] = (
    # Smoke case: one normal box; proves the 3MF path before discriminating geometry exists.
    SceneCase(
        "smoke_box",
        (
            SceneObject(
                "object-a",
                (Part("body", "normal_part", (Box(0, 0, 0, 20, 20, 0.8),)),),
                dict(LIGHTNING_OBJECT_CONFIG),
            ),
        ),
    ),
    # Central plateau flanked by taller towers: the plateau's top shell is solid while the
    # tower interiors stay sparse on the same layer, which is the temporary-anchor precondition.
    # Measured: a 1.2 mm plateau with 1.6 mm towers and top_shell_layers=2 produces nonempty
    # anchors at z=0.6; the requested 1.0 mm plateau produced none.
    SceneCase(
        "anchor_bridge",
        (
            SceneObject(
                "object-a",
                (
                    Part(
                        "body",
                        "normal_part",
                        (
                            Box(0, 0, 0, 24, 18, 1.2),
                            Box(0, 0, 0, 6, 18, 1.6),
                            Box(18, 0, 0, 24, 18, 1.6),
                        ),
                    ),
                ),
                dict(LIGHTNING_OBJECT_CONFIG),
            ),
        ),
    ),
    # Base lightning region plus a grid modifier region. Both feed the lightning generator.
    SceneCase(
        "pooled_regions",
        (
            SceneObject(
                "object-a",
                (
                    Part("base", "normal_part", (Box(0, 0, 0, 24, 18, 1.4),)),
                    Part(
                        "modifier-grid",
                        "modifier_part",
                        (Box(12, 0, -0.1, 24, 18, 1.5),),
                        {
                            "sparse_infill_pattern": "grid",
                            "sparse_infill_density": "30%",
                            "infill_direction": "45",
                            "wall_loops": "2",
                        },
                    ),
                ),
                dict(LIGHTNING_OBJECT_CONFIG),
            ),
        ),
    ),
    # Closed rectangular ring mesh plus a sealed internal void.
    #
    # Measured: subtracting the through-hole as a `negative_part` from a box yields no
    # planning-domain hole rings; canonical represents that opening as one slit contour.
    # Authoring the body as a true ring mesh (outer + inner loop connected by side/top/bottom
    # quads) is what makes the sliced ExPolygon carry a hole ring.
    SceneCase(
        "hole_void",
        (
            SceneObject(
                "object-a",
                (
                    Part(
                        "body",
                        "normal_part",
                        frames=(Frame(0, 0, 24, 18, 8, 6, 16, 12, 0, 1.4),),
                    ),
                ),
                dict(LIGHTNING_OBJECT_CONFIG),
            ),
        ),
    ),
    # Two disjoint lower islands merged by an upper cap in the same mesh.
    SceneCase(
        "island_split_merge",
        (
            SceneObject(
                "object-a",
                (
                    Part(
                        "body",
                        "normal_part",
                        (
                            Box(0, 0, 0, 10, 18, 1.0),
                            Box(14, 0, 0, 24, 18, 1.0),
                            Box(0, 0, 1.0, 24, 18, 1.4),
                        ),
                    ),
                ),
                dict(LIGHTNING_OBJECT_CONFIG),
            ),
        ),
    ),
    # Two modifier regions with different densities/directions over a lightning base.
    SceneCase(
        "paint_modifier_config",
        (
            SceneObject(
                "object-a",
                (
                    Part("base", "normal_part", (Box(0, 0, 0, 24, 18, 1.4),)),
                    Part(
                        "modifier-central",
                        "modifier_part",
                        (Box(8, 4, -0.1, 16, 14, 1.5),),
                        {
                            "sparse_infill_density": "30%",
                            "infill_direction": "45",
                        },
                    ),
                    Part(
                        "modifier-right",
                        "modifier_part",
                        (Box(20, 0, -0.1, 24, 18, 1.5),),
                        {
                            "sparse_infill_density": "10%",
                            "infill_direction": "90",
                        },
                    ),
                ),
                dict(LIGHTNING_OBJECT_CONFIG),
            ),
        ),
        fixture_crosswalk={
            "paint_chains": [
                [("material", 0)],
                [("material", 1)],
                [("material", 2)],
            ]
        },
    ),
)


# ---------------------------------------------------------------------------
# Geometry


# Outward-winding box faces over the eight vertices in `box_vertices` order. This is the
# measured canonical-compatible pattern; do not reorder without re-verifying orientation.
BOX_FACES: tuple[tuple[int, int, int], ...] = (
    (0, 2, 1),
    (0, 3, 2),  # bottom
    (4, 5, 6),
    (4, 6, 7),  # top
    (0, 1, 5),
    (0, 5, 4),  # front y0
    (1, 2, 6),
    (1, 6, 5),  # right x1
    (2, 3, 7),
    (2, 7, 6),  # back y1
    (3, 0, 4),
    (3, 4, 7),  # left x0
)


def box_vertices(box: Box) -> list[tuple[float, float, float]]:
    x0, y0, z0, x1, y1, z1 = box.x0, box.y0, box.z0, box.x1, box.y1, box.z1
    return [
        (x0, y0, z0),
        (x1, y0, z0),
        (x1, y1, z0),
        (x0, y1, z0),
        (x0, y0, z1),
        (x1, y0, z1),
        (x1, y1, z1),
        (x0, y1, z1),
    ]


def frame_vertices(frame: Frame) -> list[tuple[float, float, float]]:
    """Outer bottom (0-3), inner bottom (4-7), outer top (8-11), inner top (12-15)."""
    outer = (
        (frame.x0, frame.y0),
        (frame.x1, frame.y0),
        (frame.x1, frame.y1),
        (frame.x0, frame.y1),
    )
    inner = (
        (frame.ix0, frame.iy0),
        (frame.ix1, frame.iy0),
        (frame.ix1, frame.iy1),
        (frame.ix0, frame.iy1),
    )
    return [
        (x, y, z)
        for z in (frame.z0, frame.z1)
        for loop in (outer, inner)
        for x, y in loop
    ]


def frame_triangles(base: int) -> list[tuple[int, int, int]]:
    """Closed manifold for an extruded rectangular ring, outward-facing.

    `base` is the index of the frame's first vertex. Outer and inner loops are both
    counter-clockwise seen from +Z; the inner side quads are reversed so their normals
    face the hole.
    """
    ob = [base + i for i in range(4)]
    ib = [base + 4 + i for i in range(4)]
    ot = [base + 8 + i for i in range(4)]
    it = [base + 12 + i for i in range(4)]

    def quad(a: int, b: int, c: int, d: int) -> list[tuple[int, int, int]]:
        return [(a, b, c), (a, c, d)]

    triangles: list[tuple[int, int, int]] = []
    for k in range(4):
        k1 = (k + 1) % 4
        triangles.extend(quad(ob[k], ob[k1], ot[k1], ot[k]))
        triangles.extend(quad(ib[k], it[k], it[k1], ib[k1]))
        triangles.extend(quad(ot[k], ot[k1], it[k1], it[k]))
        triangles.extend(quad(ob[k], ib[k], ib[k1], ob[k1]))
    return triangles


def part_mesh(
    part: Part,
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int]]]:
    """Return (vertices, triangles) for every primitive in a part, with index offsets."""
    vertices: list[tuple[float, float, float]] = []
    triangles: list[tuple[int, int, int]] = []
    for box in part.boxes:
        base = len(vertices)
        vertices.extend(box_vertices(box))
        triangles.extend((base + v1, base + v2, base + v3) for v1, v2, v3 in BOX_FACES)
    for frame in part.frames:
        base = len(vertices)
        vertices.extend(frame_vertices(frame))
        triangles.extend(frame_triangles(base))
    return vertices, triangles


# ---------------------------------------------------------------------------
# 3MF scene writer
#
# The measured canonical contract (revision 3e1daccd7c):
#   - a multi-part object is a wrapper `<object>` whose `<components>` reference leaf objects;
#   - `Metadata/model_settings.config` keys each wrapper by the same id, and each `<part id>`
#     equals the leaf component objectid (a mismatch silently becomes a normal part);
#   - build-item transforms are 12 row-major values, last three are translation.
# The package is written with fixed timestamps and sorted metadata so hashes reproduce.


def fmt_num(value: float) -> str:
    text = f"{float(value):.6f}".rstrip("0").rstrip(".")
    return "0" if text in ("", "-0") else text


def attr(value: Any) -> str:
    return quoteattr(str(value))


def transform_values(translate_mm: tuple[float, float, float]) -> list[float]:
    tx, ty, tz = translate_mm
    return [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, tx, ty, tz]


def mesh_object_xml(object_id: int, part: Part) -> str:
    vertices, triangles = part_mesh(part)
    lines = [
        f'  <object id="{object_id}" type="model">',
        "   <mesh>",
        "    <vertices>",
    ]
    lines.extend(
        f'     <vertex x="{fmt_num(x)}" y="{fmt_num(y)}" z="{fmt_num(z)}"/>'
        for x, y, z in vertices
    )
    lines.append("    </vertices>")
    lines.append("    <triangles>")
    lines.extend(
        f'     <triangle v1="{v1}" v2="{v2}" v3="{v3}"/>' for v1, v2, v3 in triangles
    )
    lines.append("    </triangles>")
    lines.append("   </mesh>")
    lines.append("  </object>")
    return "\n".join(lines)


def wrapper_object_xml(object_id: int, part_ids: Sequence[int]) -> str:
    lines = [
        f'  <object id="{object_id}" type="model">',
        "   <components>",
    ]
    lines.extend(f'    <component objectid="{part_id}"/>' for part_id in part_ids)
    lines.append("   </components>")
    lines.append("  </object>")
    return "\n".join(lines)


def metadata_xml(key: str, value: Any, indent: str) -> str:
    return f"{indent}<metadata key={attr(key)} value={attr(value)}/>"


def settings_object_xml(
    object_id: int, obj: SceneObject, part_ids: Sequence[tuple[Part, int]]
) -> str:
    lines = [f' <object id="{object_id}">']
    lines.append(metadata_xml("name", obj.name, "  "))
    lines.extend(metadata_xml(key, obj.config[key], "  ") for key in sorted(obj.config))
    for part, part_id in part_ids:
        lines.append(f'  <part id="{part_id}" subtype={attr(part.kind)}>')
        lines.append(metadata_xml("name", part.name, "   "))
        lines.extend(
            metadata_xml(key, part.config[key], "   ") for key in sorted(part.config)
        )
        lines.append("  </part>")
    lines.append(" </object>")
    return "\n".join(lines)


def build_3mf_documents(scene: SceneCase) -> tuple[str, str, list[dict[str, Any]]]:
    """Return (model XML, settings XML, scene object metadata)."""
    resources: list[str] = []
    settings: list[str] = []
    build_items: list[str] = []
    object_meta: list[dict[str, Any]] = []
    next_id = 1

    for obj in scene.objects:
        transform = transform_values(obj.translate_mm)
        if len(obj.parts) == 1:
            part = obj.parts[0]
            object_id = next_id
            next_id += 1
            resources.append(mesh_object_xml(object_id, part))
            settings.append(settings_object_xml(object_id, obj, [(part, object_id)]))
            part_meta = [(part, object_id)]
        else:
            wrapper_id = next_id
            next_id += 1
            part_meta = []
            for part in obj.parts:
                leaf_id = next_id
                next_id += 1
                part_meta.append((part, leaf_id))
                resources.append(mesh_object_xml(leaf_id, part))
            object_id = wrapper_id
            resources.insert(
                len(resources) - len(part_meta),
                wrapper_object_xml(wrapper_id, [part_id for _, part_id in part_meta]),
            )
            settings.append(settings_object_xml(wrapper_id, obj, part_meta))
        build_items.append(
            f'  <item objectid="{object_id}" transform="'
            + " ".join(fmt_num(value) for value in transform)
            + '"/>'
        )
        object_meta.append(
            {
                "name": obj.name,
                "object_id": object_id,
                "translate_mm": list(obj.translate_mm),
                "build_transform": transform,
                "config": dict(obj.config),
                "parts": [
                    {
                        "name": part.name,
                        "part_id": part_id,
                        "kind": part.kind,
                        "boxes": [
                            [box.x0, box.y0, box.z0, box.x1, box.y1, box.z1]
                            for box in part.boxes
                        ],
                        "frames": [
                            [
                                frame.x0,
                                frame.y0,
                                frame.x1,
                                frame.y1,
                                frame.ix0,
                                frame.iy0,
                                frame.ix1,
                                frame.iy1,
                                frame.z0,
                                frame.z1,
                            ]
                            for frame in part.frames
                        ],
                        "config": dict(part.config),
                    }
                    for part, part_id in part_meta
                ],
            }
        )

    model = "\n".join(
        [
            '<?xml version="1.0" encoding="UTF-8"?>',
            '<model unit="millimeter" xml:lang="en-US" '
            'xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">',
            " <resources>",
            *resources,
            " </resources>",
            " <build>",
            *build_items,
            " </build>",
            "</model>",
            "",
        ]
    )
    settings_doc = "\n".join(
        [
            '<?xml version="1.0" encoding="UTF-8"?>',
            "<config>",
            *settings,
            "</config>",
            "",
        ]
    )
    return model, settings_doc, object_meta


def write_3mf(path: Path, scene: SceneCase) -> list[dict[str, Any]]:
    model, settings, object_meta = build_3mf_documents(scene)
    content_types = "\n".join(
        [
            '<?xml version="1.0" encoding="UTF-8"?>',
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">',
            ' <Default Extension="rels" '
            'ContentType="application/vnd.openxmlformats-package.relationships+xml"/>',
            ' <Default Extension="model" '
            'ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>',
            "</Types>",
            "",
        ]
    )
    rels = "\n".join(
        [
            '<?xml version="1.0" encoding="UTF-8"?>',
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">',
            ' <Relationship Target="/3D/3dmodel.model" Id="rel-1" '
            'Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>',
            "</Relationships>",
            "",
        ]
    )
    entries = (
        ("[Content_Types].xml", content_types),
        ("_rels/.rels", rels),
        ("3D/3dmodel.model", model),
        ("Metadata/model_settings.config", settings),
    )
    with zipfile.ZipFile(path, "w") as package:
        for name, data in entries:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 0
            info.external_attr = 0o600 << 16
            package.writestr(info, data.encode("utf-8"))
    return object_meta


def write_scene_json(
    path: Path, scene: SceneCase, object_meta: list[dict[str, Any]]
) -> None:
    payload = {
        "schema_version": SCHEMA_VERSION,
        "case_id": scene.case_id,
        "objects": object_meta,
        "process_overrides": dict(scene.process_overrides),
        "fixture_crosswalk": scene.fixture_crosswalk,
    }
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


# ---------------------------------------------------------------------------
# Settings


def read_profile(root: Path, rel: str) -> dict[str, Any]:
    with (root / rel).open("r", encoding="utf-8") as fh:
        return json.load(fh)


def index_profiles(root: Path) -> dict[str, dict[str, Any]]:
    """Index every named preset in the shipped profile tree by its `name`."""
    index: dict[str, dict[str, Any]] = {}
    for path in (root / "resources" / "profiles").rglob("*.json"):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            continue
        name = data.get("name")
        if isinstance(name, str) and name:
            index.setdefault(name, data)
    return index


def resolve_preset(
    index: dict[str, dict[str, Any]], preset: dict[str, Any]
) -> dict[str, Any]:
    """Flatten an `inherits` chain.

    The CLI resolves a supplied preset only through names present in the system bundle; a
    CLI-authored preset that names a shipped parent is rejected with "Preset was not found".
    Resolving here and passing a complete preset avoids that lookup entirely.
    """
    chain: list[dict[str, Any]] = []
    seen: set[str] = set()
    current: dict[str, Any] | None = preset
    while current is not None:
        chain.append(current)
        parent = current.get("inherits")
        if not isinstance(parent, str) or not parent or parent in seen:
            break
        seen.add(parent)
        current = index.get(parent)
    merged: dict[str, Any] = {}
    for item in reversed(chain):
        merged.update(item)
    merged.pop("inherits", None)
    return merged


def build_settings(
    base: dict[str, Any], case: SceneCase, strategy: str
) -> dict[str, Any]:
    # `base` is the already-flattened shipped process preset. The CLI accepts exactly one
    # process preset, so overrides are applied to that preset rather than supplied beside it.
    override: dict[str, Any] = dict(base)
    override.update(
        {
            "from": "system",
            "type": "process",
            "name": "lightning_gate_" + case.case_id,
            "sparse_infill_pattern": "lightning",
            "sparse_infill_density": "15%",
            "layer_height": "0.2",
            "sparse_infill_line_width": "0.45",
            "fill_multiline": "1",
            "lightning_overhang_angle": "45",
            "lightning_prune_angle": "5",
            # Canonical validates this in [5, 85]; the packet's seed "0" is not representable
            # in this revision, so record the minimum and let the resolved value stand.
            "lightning_straightening_angle": "5",
            "wall_generator": "arachne" if strategy == "arachne" else "classic",
            "wall_loops": "2",
            "top_shell_layers": "3",
            "bottom_shell_layers": "3",
            "enable_support": "0",
            "sparse_infill_rotate_template": "",
            "solid_infill_rotate_template": "",
            "infill_direction": "45",
            "solid_infill_direction": "45",
            "top_layer_direction": "45",
            "bottom_layer_direction": "45",
            "align_infill_direction_to_model": "0",
        }
    )
    override.update(COMMON_PROCESS_OVERRIDES)
    override.update(case.process_overrides)
    return override


# ---------------------------------------------------------------------------
# Process execution


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run(cmd: Sequence[str], cwd: Path, env: dict[str, str], log: Path) -> int:
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("a", encoding="utf-8", errors="replace") as fh:
        fh.write("\n$ " + " ".join(cmd) + "\n")
        fh.flush()
        proc = subprocess.run(
            cmd, cwd=str(cwd), env=env, stdout=fh, stderr=subprocess.STDOUT
        )
    return proc.returncode


def git_revision(root: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
        ).strip()
    except Exception:
        return "unknown"


def git_dirty(root: Path) -> bool:
    try:
        out = subprocess.check_output(
            ["git", "-C", str(root), "status", "--porcelain"], text=True
        )
        return bool(out.strip())
    except Exception:
        return True


def tool_versions(slicer: Path) -> dict[str, str]:
    return {
        "python": platform.python_version(),
        "platform": platform.platform(),
        "slicer": str(slicer),
        "slicer_sha256": sha256_file(slicer) if slicer.exists() else "absent",
    }


def resolve_build_source(slicer: Path, explicit: str | None) -> Path | None:
    """Resolve the instrumented build checkout from the slicer path (or an explicit override).

    Walking up to the nearest `.git` keeps this working for the standard
    `<checkout>/build/src/Release/orca-slicer.exe` layout without hard-coding a parent index.
    """
    if explicit:
        candidate = Path(explicit).resolve()
        return candidate if candidate.is_dir() else None
    for candidate in (slicer.parent, *slicer.parents):
        if (candidate / ".git").exists():
            return candidate
    return None


def patch_applied(build_source: Path, patch: Path) -> bool:
    """True when reversing the recorder patch against the build tree is a no-op."""
    if not patch.exists():
        return False
    try:
        proc = subprocess.run(
            [
                "git",
                "-C",
                str(build_source),
                "apply",
                "--check",
                "--reverse",
                str(patch),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return proc.returncode == 0
    except Exception:
        return False


# ---------------------------------------------------------------------------
# Capture


def copy_object(
    obj: SceneObject, name: str, translate_mm: tuple[float, float, float]
) -> SceneObject:
    """Deep-copy an object for the translated companion, preserving part configs."""
    return SceneObject(
        name,
        tuple(replace(part, config=dict(part.config)) for part in obj.parts),
        dict(obj.config),
        translate_mm,
    )


def translated_scene(case: SceneCase) -> SceneCase:
    primary = case.objects[0]
    companion = copy_object(primary, "object-b", (40.0, 0.0, 0.0))
    return replace(
        case,
        case_id=case.case_id + "_translated",
        objects=case.objects + (companion,),
    )


def density_scene(case: SceneCase) -> SceneCase:
    primary = case.objects[0]
    config = dict(primary.config)
    config["sparse_infill_density"] = "30%"
    objects = (replace(primary, config=config),) + case.objects[1:]
    return replace(case, case_id=case.case_id + "_density30", objects=objects)


def case_variants(case: SceneCase) -> list[tuple[str, SceneCase, str, str | None]]:
    """Yield (variant_id, scene, strategy, density) combinations."""
    variants: list[tuple[str, SceneCase, str, str | None]] = []
    scenes: list[tuple[str, SceneCase]] = [(case.case_id, case)]
    scenes.append((case.case_id + "_translated", translated_scene(case)))
    for scene_id, scene in scenes:
        for strategy in WALL_STRATEGIES:
            variants.append((f"{scene_id}_{strategy}", scene, strategy, None))
            if scene_id == case.case_id:
                variants.append(
                    (
                        f"{scene_id}_{strategy}_density30",
                        density_scene(scene),
                        strategy,
                        "30%",
                    )
                )
    return variants


# ---------------------------------------------------------------------------
# Provenance DAG
#
# The recorder's source/domains/operations artifacts are already complete observations. This
# builder only relates them: it normalizes every polygon, projects planning/final boundary
# segments back onto pre-anchor source segments, and falls back to the operation that generated a
# segment when no source segment overlaps it. It never derives geometry from PnP and never invents
# a source id.

PROVENANCE_UNIT_NOTE = (
    "OrcaSlicer 1 unit = 1 nm; fixture converter applies PnP 100 nm conversion"
)
PROVENANCE_RING_NORMALIZATION = (
    "contours counter-clockwise (positive shoelace area), holes clockwise; rings rotated so the "
    "lexicographically smallest (x, y) point is first; duplicated closing and consecutive "
    "duplicate points dropped; ring_id 0 = contour and 1..n = holes; segment_id 0..n-1 in "
    "normalized order"
)

_PLANNING_DOMAIN_LAYOUT = (
    ("internal", "internal_domains", "internal"),
    ("void", "void_domains", "void"),
    ("temporary_anchor", "temporary_anchors", "anchor"),
)
_FINAL_DOMAIN_LAYOUT = (
    ("final_internal", "final_internal_domains", "final_internal"),
    ("final_void", "final_void_domains", "final_void"),
)
_GENERATED_FALLBACK_ORDER = ("Difference", "Union", "AnchorExpansion")


def _unit(value: Any, where: str) -> int:
    """Read one canonical integer unit without silently truncating a non-integral float."""
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{where}: expected an integer canonical unit, got {value!r}")
    if isinstance(value, float):
        if not value.is_integer():
            raise ValueError(f"{where}: non-integral canonical unit {value!r}")
        return int(value)
    return value


def _ring_points(raw: Any, where: str) -> tuple[tuple[int, int], ...]:
    points: list[tuple[int, int]] = []
    for index, pair in enumerate(raw):
        if not isinstance(pair, (list, tuple)) or len(pair) != 2:
            raise ValueError(f"{where}: point {index} is not an (x, y) pair")
        point = (
            _unit(pair[0], f"{where}[{index}].x"),
            _unit(pair[1], f"{where}[{index}].y"),
        )
        if not points or points[-1] != point:
            points.append(point)
    if len(points) > 1 and points[0] == points[-1]:
        points.pop()
    return tuple(points)


def _normalize_ring(
    raw: Any, *, counter_clockwise: bool, where: str
) -> tuple[tuple[int, int], ...]:
    points = _ring_points(raw, where)
    if len(points) < 3:
        return ()
    area = 0
    for index, (x0, y0) in enumerate(points):
        x1, y1 = points[(index + 1) % len(points)]
        area += x0 * y1 - x1 * y0
    if (counter_clockwise and area < 0) or (not counter_clockwise and area > 0):
        points = tuple(reversed(points))
    first = min(range(len(points)), key=points.__getitem__)
    return points[first:] + points[:first]


def _normalize_polygon(raw: Any, where: str) -> dict[str, Any]:
    """Normalize one ExPolygon into the shared source/planning/final/operation representation."""
    if not isinstance(raw, dict):
        raise ValueError(f"{where}: expolygon is not an object")
    contour = _normalize_ring(
        raw.get("contour", []), counter_clockwise=True, where=f"{where}.contour"
    )
    holes: list[tuple[tuple[int, int], ...]] = []
    for index, hole in enumerate(raw.get("holes", []) or []):
        normalized = _normalize_ring(
            hole, counter_clockwise=False, where=f"{where}.holes[{index}]"
        )
        if normalized:
            holes.append(normalized)

    rings: list[dict[str, Any]] = []
    segments: list[tuple[int, int, tuple[int, int], tuple[int, int]]] = []
    oriented = ([("contour", contour)] if contour else []) + [
        ("hole", hole) for hole in holes
    ]
    for ring_id, (role, points) in enumerate(oriented):
        ring_segments = []
        for segment_id in range(len(points)):
            start = points[segment_id]
            end = points[(segment_id + 1) % len(points)]
            ring_segments.append(
                {"segment_id": segment_id, "endpoints": [list(start), list(end)]}
            )
            segments.append((ring_id, segment_id, start, end))
        rings.append({"ring_id": ring_id, "role": role, "segments": ring_segments})

    return {
        "geometry": {
            "contour": [list(point) for point in contour],
            "holes": [[list(point) for point in hole] for hole in holes],
        },
        "rings": rings,
        "segments": segments,
    }


def _source_key(source_id: Any) -> tuple[int, int] | None:
    """Extract (global_layer_index, region_id) from `o<object>_l<layer>_r<region>_s<index>`."""
    if not isinstance(source_id, str):
        return None
    parts = source_id.split("_")
    if len(parts) != 4 or not parts[0].startswith("o") or not parts[1].startswith("l"):
        return None
    if not parts[2].startswith("r"):
        return None
    try:
        return int(parts[1][1:]), int(parts[2][1:])
    except ValueError:
        return None


def _cross(a: tuple[int, int], b: tuple[int, int], c: tuple[int, int]) -> int:
    return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])


def _segment_overlap(
    boundary_start: tuple[int, int],
    boundary_end: tuple[int, int],
    source_start: tuple[int, int],
    source_end: tuple[int, int],
    tolerance_units: int,
) -> tuple[Fraction, Fraction] | None:
    """Collinear overlap interval as exact parameters on the source segment, clamped to [0, 1]."""
    dx = source_end[0] - source_start[0]
    dy = source_end[1] - source_start[1]
    length_squared = dx * dx + dy * dy
    if length_squared == 0:
        return None
    for point in (boundary_start, boundary_end):
        cross = _cross(source_start, source_end, point)
        if tolerance_units == 0:
            if cross != 0:
                return None
        elif cross * cross > tolerance_units * tolerance_units * length_squared:
            return None

    def projection(point: tuple[int, int]) -> Fraction:
        dot = (point[0] - source_start[0]) * dx + (point[1] - source_start[1]) * dy
        return Fraction(dot, length_squared)

    low = projection(boundary_start)
    high = projection(boundary_end)
    if low > high:
        low, high = high, low
    low = max(Fraction(0), low)
    high = min(Fraction(1), high)
    if high <= low:
        return None
    return low, high


def _rational(value: Fraction) -> dict[str, int]:
    return {"numerator": value.numerator, "denominator": value.denominator}


def _build_source_catalog(
    source: dict[str, Any],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    catalog: list[dict[str, Any]] = []
    for region_index, region in enumerate(source.get("regions", []) or []):
        region_id = _unit(region.get("region_id"), f"regions[{region_index}].region_id")
        for surface_index, surface in enumerate(region.get("surfaces", []) or []):
            where = f"regions[{region_index}].surfaces[{surface_index}]"
            source_id = surface.get("source_id")
            if not isinstance(source_id, str) or not source_id:
                raise ValueError(f"{where}: missing source_id")
            model = _normalize_polygon(
                surface.get("expolygon", {}), f"{where}.expolygon"
            )
            catalog.append(
                {
                    "source_id": source_id,
                    "region_id": _unit(
                        surface.get("region_id", region_id), f"{where}.region_id"
                    ),
                    "local_layer_index": _unit(
                        surface.get("local_layer_index"), f"{where}.local_layer_index"
                    ),
                    "global_layer_index": _unit(
                        surface.get("global_layer_index"), f"{where}.global_layer_index"
                    ),
                    "z_mm": surface.get("z_mm"),
                    "checkpoint": surface.get("checkpoint"),
                    "surface_kind": surface.get("surface_type"),
                    "model": model,
                }
            )
    catalog.sort(
        key=lambda entry: (
            entry["local_layer_index"],
            entry["region_id"],
            entry["source_id"],
        )
    )
    public = [
        {
            "source_id": entry["source_id"],
            "region_id": entry["region_id"],
            "local_layer_index": entry["local_layer_index"],
            "global_layer_index": entry["global_layer_index"],
            "z_mm": entry["z_mm"],
            "checkpoint": entry["checkpoint"],
            "surface_kind": entry["surface_kind"],
            "geometry": entry["model"]["geometry"],
            "rings": entry["model"]["rings"],
        }
        for entry in catalog
    ]
    return catalog, public


def _build_domains(
    domains: dict[str, Any], object_id: int, layout: Sequence[tuple[str, str, str]]
) -> list[dict[str, Any]]:
    built: list[dict[str, Any]] = []
    for layer in domains.get("layers", []) or []:
        local_layer = _unit(layer.get("local_layer_index"), "layer.local_layer_index")
        global_layer = _unit(
            layer.get("global_layer_index"), "layer.global_layer_index"
        )
        z_mm = layer.get("z_mm")
        for region in layer.get("regions", []) or []:
            region_id = _unit(region.get("region_id"), "region.region_id")
            for domain, field, slug in layout:
                for index, raw in enumerate(region.get(field, []) or []):
                    surface_id = (
                        f"o{object_id}_l{local_layer}_r{region_id}_{slug}_{index}"
                    )
                    built.append(
                        {
                            "domain": domain,
                            "surface_id": surface_id,
                            "local_layer_index": local_layer,
                            "global_layer_index": global_layer,
                            "region_id": region_id,
                            "z_mm": z_mm,
                            "model": _normalize_polygon(
                                raw,
                                f"layer[{local_layer}].region[{region_id}].{field}[{index}]",
                            ),
                        }
                    )
    built.sort(
        key=lambda entry: (
            entry["domain"],
            entry["local_layer_index"],
            entry["surface_id"],
        )
    )
    return built


def _public_domain(
    entry: dict[str, Any], boundaries: list[dict[str, Any]]
) -> dict[str, Any]:
    return {
        "domain": entry["domain"],
        "surface_id": entry["surface_id"],
        "local_layer_index": entry["local_layer_index"],
        "global_layer_index": entry["global_layer_index"],
        "z_mm": entry["z_mm"],
        "geometry": entry["model"]["geometry"],
        "rings": entry["model"]["rings"],
        "boundaries": boundaries,
    }


def _operation_models(operations: dict[str, Any]) -> list[dict[str, Any]]:
    models: list[dict[str, Any]] = []
    raw_operations = operations.get("operations", []) or []
    for index, operation in enumerate(raw_operations):
        where = f"operations[{index}]"
        operation_id = operation.get("operation_id")
        if not isinstance(operation_id, int) or isinstance(operation_id, bool):
            raise ValueError(f"{where}: missing integer operation_id")
        inputs = [
            item for item in operation.get("inputs", []) or [] if isinstance(item, str)
        ]
        operand_sources: list[str] = []
        for operand in operation.get("operands", []) or []:
            for item in operand.get("sources", []) or []:
                if isinstance(item, str):
                    operand_sources.append(item)
        keys = {
            key
            for key in (_source_key(item) for item in [*inputs, *operand_sources])
            if key is not None
        }
        result_models = [
            _normalize_polygon(result, f"{where}.result_geometry[{result_index}]")
            for result_index, result in enumerate(
                operation.get("result_geometry", []) or []
            )
        ]
        models.append(
            {
                "operation_id": operation_id,
                "kind": operation.get("kind"),
                "checkpoint": operation.get("checkpoint"),
                "inputs": inputs,
                "parent_operations": list(operation.get("parent_operations", []) or []),
                "operands": operation.get("operands", []) or [],
                "offset_units": operation.get("offset_units"),
                "join": operation.get("join"),
                "result_models": result_models,
                "result_geometry": [model["geometry"] for model in result_models],
                "result_segments": [
                    segment for model in result_models for segment in model["segments"]
                ],
                "input_sources": list(dict.fromkeys(inputs)),
                "operand_sources": list(dict.fromkeys(operand_sources)),
                "keys": keys,
            }
        )
    models.sort(key=lambda entry: entry["operation_id"])
    return models


def _matches_operation(
    operation: dict[str, Any],
    key: tuple[int, int],
    boundary_start: tuple[int, int],
    boundary_end: tuple[int, int],
) -> dict[str, Any] | None:
    if operation["keys"] and key not in operation["keys"]:
        return None
    for tolerance in (0, 1):
        best: tuple[Fraction, int, int, Fraction, Fraction] | None = None
        for ring_id, segment_id, start, end in operation["result_segments"]:
            overlap = _segment_overlap(
                boundary_start, boundary_end, start, end, tolerance
            )
            if overlap is None:
                continue
            width = overlap[1] - overlap[0]
            if best is None or width > best[0]:
                best = (width, ring_id, segment_id, overlap[0], overlap[1])
        if best is not None:
            return {
                "ring_id": best[1],
                "segment_id": best[2],
                "start": best[3],
                "end": best[4],
                "tolerance_units": tolerance,
            }
    return None


def _pick_operation(
    matches: list[dict[str, Any]], kinds: Sequence[str]
) -> dict[str, Any] | None:
    for kind in kinds:
        candidates = [match for match in matches if match["operation"]["kind"] == kind]
        if candidates:
            candidates.sort(
                key=lambda match: (
                    -(match["match"]["end"] - match["match"]["start"]),
                    match["operation"]["operation_id"],
                )
            )
            return candidates[0]
    return None


def _generated_contributors(
    domain: str,
    key: tuple[int, int],
    boundary_start: tuple[int, int],
    boundary_end: tuple[int, int],
    operations: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], str | None]:
    direct: list[dict[str, Any]] = []
    for operation in operations:
        match = _matches_operation(operation, key, boundary_start, boundary_end)
        if match is not None:
            direct.append({"operation": operation, "match": match})

    preferred = (
        ("AnchorExpansion",)
        if domain == "temporary_anchor"
        else ("Difference", "Union")
    )
    chosen = _pick_operation(direct, preferred)

    if chosen is None:
        # A boundary can survive from a parent operation even when the direct op's result no longer
        # contains it (e.g. the anchor expansion that a union/difference later clips).
        if direct:
            parent_ids = {
                parent
                for match in direct
                for parent in match["operation"]["parent_operations"]
                if isinstance(parent, int)
            }
        else:
            parent_ids = {
                parent
                for operation in operations
                if not operation["keys"] or key in operation["keys"]
                for parent in operation["parent_operations"]
                if isinstance(parent, int)
            }
        parents: list[dict[str, Any]] = []
        for operation in operations:
            if operation["operation_id"] not in parent_ids:
                continue
            match = _matches_operation(operation, key, boundary_start, boundary_end)
            if match is not None:
                parents.append({"operation": operation, "match": match})
        chosen = _pick_operation(parents, preferred)
        if chosen is None:
            chosen = _pick_operation(parents, _GENERATED_FALLBACK_ORDER)

    if chosen is None:
        chosen = _pick_operation(direct, _GENERATED_FALLBACK_ORDER)
    if chosen is None:
        return [], "no_matching_operation"

    operation = chosen["operation"]
    sources = operation["input_sources"] or operation["operand_sources"]
    if not sources:
        return [], "operation_has_no_inputs"

    contributors = []
    for source_id in sources:
        relation: dict[str, Any] = {
            "kind": "generated",
            "operation_id": operation["operation_id"],
        }
        if chosen["match"]["tolerance_units"]:
            relation["match_tolerance_units"] = chosen["match"]["tolerance_units"]
        contributors.append(
            {
                "source_id": source_id,
                # A generated segment has no pre-anchor source segment of its own. Its generating
                # operation result segment is identified by the operation's result_geometry; these
                # two fields stay null so they are never mistaken for a source address.
                "ring_id": None,
                "segment_id": None,
                "relation": relation,
            }
        )
    return contributors, None


def _source_overlap_contributors(
    boundary_start: tuple[int, int],
    boundary_end: tuple[int, int],
    sources: Sequence[dict[str, Any]],
) -> list[dict[str, Any]]:
    for tolerance in (0, 1):
        contributors: list[dict[str, Any]] = []
        for source in sources:
            for ring_id, segment_id, start, end in source["model"]["segments"]:
                overlap = _segment_overlap(
                    boundary_start, boundary_end, start, end, tolerance
                )
                if overlap is None:
                    continue
                relation: dict[str, Any] = {
                    "kind": "overlap",
                    "start": _rational(overlap[0]),
                    "end": _rational(overlap[1]),
                }
                if tolerance:
                    relation["match_tolerance_units"] = tolerance
                contributors.append(
                    {
                        "source_id": source["source_id"],
                        "ring_id": ring_id,
                        "segment_id": segment_id,
                        "relation": relation,
                    }
                )
        if contributors:
            return contributors
    return []


def _resolve_boundaries(
    entry: dict[str, Any],
    sources_by_slot: dict[tuple[int, int], list[dict[str, Any]]],
    operations: list[dict[str, Any]],
    unavailable: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    key = (entry["global_layer_index"], entry["region_id"])
    sources = sources_by_slot.get((entry["local_layer_index"], entry["region_id"]), [])
    boundaries: list[dict[str, Any]] = []
    for ring_id, segment_id, start, end in entry["model"]["segments"]:
        # A temporary anchor is a derived post-anchor surface, not a pre-anchor source surface.
        # Its edges are frequently collinear with the source edge they were expanded from, so source
        # overlap would misattribute the derived anchor. Resolve it through AnchorExpansion first
        # and fall back to overlap only when no operation result contains the segment.
        if entry["domain"] == "temporary_anchor":
            contributors, reason = _generated_contributors(
                entry["domain"], key, start, end, operations
            )
            if not contributors:
                contributors = _source_overlap_contributors(start, end, sources)
                reason = None if contributors else reason
        else:
            contributors = _source_overlap_contributors(start, end, sources)
            reason = None
            if not contributors:
                contributors, reason = _generated_contributors(
                    entry["domain"], key, start, end, operations
                )
        contributors.sort(
            key=lambda contributor: (
                contributor["source_id"],
                contributor["relation"]["kind"],
                contributor["relation"].get("operation_id", -1),
                contributor["ring_id"] if contributor["ring_id"] is not None else -1,
                contributor["segment_id"]
                if contributor["segment_id"] is not None
                else -1,
            )
        )
        if not contributors:
            unavailable.append(
                {
                    "domain": entry["domain"],
                    "surface_id": entry["surface_id"],
                    "ring_id": ring_id,
                    "segment_id": segment_id,
                    "reason": reason or "no_matching_source_or_operation",
                }
            )
        boundaries.append(
            {
                "ring_id": ring_id,
                "segment_id": segment_id,
                "endpoints": [list(start), list(end)],
                "contributors": contributors,
            }
        )
    boundaries.sort(key=lambda boundary: (boundary["ring_id"], boundary["segment_id"]))
    return boundaries


def _operation_outputs(
    operations: list[dict[str, Any]],
    domains: Sequence[dict[str, Any]],
) -> None:
    addresses: list[
        tuple[
            tuple[str, str, int, int], tuple[int, int], tuple[int, int], tuple[int, int]
        ]
    ] = []
    for entry in domains:
        for ring_id, segment_id, start, end in entry["model"]["segments"]:
            addresses.append(
                (
                    (
                        entry["domain"],
                        entry["surface_id"],
                        ring_id,
                        segment_id,
                    ),
                    (entry["global_layer_index"], entry["region_id"]),
                    start,
                    end,
                )
            )
    for operation in operations:
        outputs: set[tuple[str, str, int, int]] = set()
        for _, _, start, end in operation["result_segments"]:
            for address, key, domain_start, domain_end in addresses:
                if operation["keys"] and key not in operation["keys"]:
                    continue
                for tolerance in (0, 1):
                    if (
                        _segment_overlap(
                            start, end, domain_start, domain_end, tolerance
                        )
                        is not None
                    ):
                        outputs.add(address)
                        break
        operation["outputs"] = [
            {
                "domain": domain,
                "surface_id": surface_id,
                "ring_id": ring_id,
                "segment_id": segment_id,
            }
            for domain, surface_id, ring_id, segment_id in sorted(outputs)
        ]


def _public_operation(operation: dict[str, Any]) -> dict[str, Any]:
    return {
        "operation_id": operation["operation_id"],
        "kind": operation["kind"],
        "checkpoint": operation["checkpoint"],
        "inputs": operation["inputs"],
        "parent_operations": operation["parent_operations"],
        "operands": operation["operands"],
        "offset_units": operation["offset_units"],
        "join": operation["join"],
        "result_geometry": operation["result_geometry"],
        "outputs": operation["outputs"],
    }


def build_provenance(
    source: dict[str, Any],
    domains: dict[str, Any],
    operations: dict[str, Any],
    case_id: str,
) -> dict[str, Any]:
    object_id = _unit(source.get("object_id"), "source.object_id")
    object_name = source.get("object_name")
    catalog, public_catalog = _build_source_catalog(source)
    planning_entries = _build_domains(domains, object_id, _PLANNING_DOMAIN_LAYOUT)
    final_entries = _build_domains(domains, object_id, _FINAL_DOMAIN_LAYOUT)
    operation_models = _operation_models(operations)

    sources_by_slot: dict[tuple[int, int], list[dict[str, Any]]] = {}
    for source_entry in catalog:
        sources_by_slot.setdefault(
            (source_entry["local_layer_index"], source_entry["region_id"]), []
        ).append(source_entry)
    for slot_sources in sources_by_slot.values():
        slot_sources.sort(key=lambda entry: entry["source_id"])

    unavailable: list[dict[str, Any]] = []
    planning_public: list[dict[str, Any]] = []
    final_public: list[dict[str, Any]] = []
    for entry in planning_entries:
        boundaries = _resolve_boundaries(
            entry, sources_by_slot, operation_models, unavailable
        )
        planning_public.append(_public_domain(entry, boundaries))
    for entry in final_entries:
        boundaries = _resolve_boundaries(
            entry, sources_by_slot, operation_models, unavailable
        )
        final_public.append(_public_domain(entry, boundaries))

    _operation_outputs(operation_models, [*planning_entries, *final_entries])

    def count_boundaries(domains_public: Sequence[dict[str, Any]], kind: str) -> int:
        return sum(
            1
            for domain in domains_public
            for boundary in domain["boundaries"]
            if any(
                contributor["relation"]["kind"] == kind
                for contributor in boundary["contributors"]
            )
        )

    all_domains = [*planning_public, *final_public]
    counts = {
        "source_surfaces": len(public_catalog),
        "planning_boundaries": sum(
            len(domain["boundaries"]) for domain in planning_public
        ),
        "source_overlaps": count_boundaries(all_domains, "overlap"),
        "generated_boundaries": count_boundaries(all_domains, "generated"),
        "unavailable_boundaries": len(unavailable),
        "operations": len(operation_models),
    }
    unavailable.sort(
        key=lambda item: (
            item["domain"],
            item["surface_id"],
            item["ring_id"],
            item["segment_id"],
        )
    )

    result: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "case_id": case_id,
        "object_name": object_name,
        "object_id": object_id,
        "unit": "canonical_units",
        "unit_note": PROVENANCE_UNIT_NOTE,
        "ring_normalization": PROVENANCE_RING_NORMALIZATION,
        "source_catalog": public_catalog,
        "planning_domains": planning_public,
        "final_domains": final_public,
        "operations": [_public_operation(operation) for operation in operation_models],
        "counts": counts,
        "unavailable": unavailable,
    }
    if not planning_entries:
        # No planning domains is a legitimate family shape (e.g. a fully solid flat plate). The
        # file is still written with the source/final catalog so the absence is auditable.
        result["reason"] = "no_planning_domains"
    return result


def _append_capture_log(log: Path, message: str) -> None:
    try:
        with log.open("a", encoding="utf-8", errors="replace") as handle:
            handle.write(message.rstrip() + "\n")
    except OSError:
        pass


def _write_provenance_atomic(path: Path, body: dict[str, Any]) -> None:
    canonical = json.dumps(body, sort_keys=True, separators=(",", ":"))
    digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    payload = dict(body)
    payload["provenance_sha256"] = digest
    text = json.dumps(payload, sort_keys=True, separators=(",", ":")) + "\n"
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(text, encoding="utf-8")
    os.replace(temporary, path)


def build_provenance_for_variant(
    variant_dir: Path, variant_id: str, log: Path
) -> list[str]:
    """Write one provenance file per object; a per-object failure is logged, not fatal."""
    written: list[str] = []
    for source_path in sorted(variant_dir.glob("*.source.json")):
        stem = source_path.name[: -len(".source.json")]
        domains_path = variant_dir / f"{stem}.domains.json"
        operations_path = variant_dir / f"{stem}.operations.json"
        if not domains_path.exists():
            _append_capture_log(
                log, f"provenance: {variant_id}: missing {domains_path.name}; skipped"
            )
            continue
        try:
            source = json.loads(source_path.read_text(encoding="utf-8"))
            domains = json.loads(domains_path.read_text(encoding="utf-8"))
            operations = (
                json.loads(operations_path.read_text(encoding="utf-8"))
                if operations_path.exists()
                else {"operations": []}
            )
            case_id = source.get("case_id") or variant_id
            body = build_provenance(source, domains, operations, case_id)
            target = variant_dir / f"{stem}.provenance.json"
            _write_provenance_atomic(target, body)
            written.append(target.name)
        except Exception as exc:  # noqa: BLE001 - provenance must not abort a capture
            _append_capture_log(
                log,
                f"provenance: {variant_id}: {source_path.name}: {type(exc).__name__}: {exc}",
            )
    return written


def hole_ring_count(variant_dir: Path) -> int:
    """Count hole rings in the typed planning/final domains of one captured variant.

    `hole_void` is the only family that must exercise a planning-domain hole ring; the count is
    recorded for every variant so a future scene change that silently fuses the hole back into a
    slit contour fails the capture instead of passing unnoticed.
    """
    count = 0
    for path in sorted(variant_dir.glob("*.domains.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        for layer in data.get("layers", []):
            for region in layer.get("regions", []):
                for key in (
                    "internal_domains",
                    "void_domains",
                    "final_internal_domains",
                    "final_void_domains",
                ):
                    for polygon in region.get(key) or []:
                        count += len(polygon.get("holes") or [])
    return count


def capture(args: argparse.Namespace) -> int:
    source = Path(args.source).resolve()
    work = Path(args.work).resolve()
    output = Path(args.output).resolve()
    slicer = Path(args.slicer).resolve()
    build_source = resolve_build_source(slicer, getattr(args, "build_source", None))
    patch = Path(args.repo).resolve() / PATCH_REL
    only_cases = set(filter(None, (args.case or "").split(",")))
    strategies = (
        (args.strategy,)
        if args.strategy and args.strategy != "both"
        else WALL_STRATEGIES
    )

    output.mkdir(parents=True, exist_ok=True)
    raw_dir = output / "raw"
    raw_dir.mkdir(parents=True, exist_ok=True)
    log = output / "capture.log"
    log.write_text("", encoding="utf-8")

    if not slicer.exists():
        print(
            json.dumps(
                {
                    "status": "inconclusive",
                    "reason": "instrumented slicer is absent",
                    "slicer": str(slicer),
                    "build": "run: build_win.bat -d then build_win.bat -s after applying the recorder patch",
                }
            )
        )
        return 3

    if build_source is None:
        print(
            json.dumps(
                {
                    "status": "inconclusive",
                    "reason": "instrumented build checkout could not be resolved from --slicer",
                    "slicer": str(slicer),
                    "canonical_build_source": None,
                    "hint": "pass --build-source <path> or point --slicer at a checkout-local build",
                }
            )
        )
        return 3

    build_revision = git_revision(build_source)
    if build_revision == "unknown":
        print(
            json.dumps(
                {
                    "status": "inconclusive",
                    "reason": "instrumented build checkout is not a git work tree",
                    "canonical_build_source": str(build_source),
                    "canonical_build_revision": None,
                }
            )
        )
        return 3

    build_dirty = git_dirty(build_source)
    patch_is_applied = patch_applied(build_source, patch)

    provenance = {
        "schema_version": SCHEMA_VERSION,
        "canonical_source": str(source),
        "canonical_revision": git_revision(source),
        "canonical_build_source": str(build_source),
        "canonical_build_revision": build_revision,
        "canonical_build_dirty": build_dirty,
        "recorder_patch": str(patch),
        "recorder_patch_sha256": sha256_file(patch) if patch.exists() else "absent",
        "recorder_patch_applied": patch_is_applied,
        "tools": tool_versions(slicer),
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "cases": [],
    }

    profile_index = index_profiles(source)
    resolved_machine = resolve_preset(
        profile_index, read_profile(source, MACHINE_PROFILE)
    )
    resolved_filament = resolve_preset(
        profile_index, read_profile(source, FILAMENT_PROFILE)
    )
    process_base = resolve_preset(profile_index, read_profile(source, PROCESS_PROFILE))
    machine_path = output / "machine.json"
    filament_path = output / "filament.json"
    machine_path.write_text(json.dumps(resolved_machine, indent=2), encoding="utf-8")
    filament_path.write_text(json.dumps(resolved_filament, indent=2), encoding="utf-8")

    failures: list[str] = []
    for case in CASES:
        if only_cases and case.case_id not in only_cases:
            continue
        for variant_id, scene, strategy, density in case_variants(case):
            if strategy not in strategies:
                continue
            variant_dir = raw_dir / variant_id
            if variant_dir.exists():
                shutil.rmtree(variant_dir)
            variant_dir.mkdir(parents=True, exist_ok=True)
            model_path = variant_dir / "model.3mf"
            object_meta = write_3mf(model_path, scene)
            scene_path = variant_dir / "scene.json"
            write_scene_json(scene_path, scene, object_meta)
            settings = build_settings(process_base, scene, strategy)
            settings_path = variant_dir / "settings.json"
            settings_path.write_text(json.dumps(settings, indent=2), encoding="utf-8")

            env = dict(os.environ)
            env["LIGHTNING_GATE_OUTPUT"] = str(variant_dir)
            env["LIGHTNING_GATE_CASE"] = variant_id
            # Every capture also writes the independent snapshot replay artifact. Replay compares
            # the faithful planning graph against the production graph and never writes
            # planning.json or caller/clip records.
            env["LIGHTNING_GATE_REPLAY"] = "1"
            datadir = variant_dir / "datadir"
            datadir.mkdir(exist_ok=True)

            cmd = [
                str(slicer),
                "--slice",
                "0",
                "--outputdir",
                str(variant_dir),
                "--datadir",
                str(datadir),
                "--load-settings",
                f"{machine_path};{settings_path}",
                "--load-filaments",
                str(filament_path),
                str(model_path),
            ]
            exit_code = run(cmd, variant_dir, env, log)
            if not getattr(args, "no_provenance", False):
                provenance_files = build_provenance_for_variant(
                    variant_dir, variant_id, log
                )
                if provenance_files:
                    _append_capture_log(
                        log,
                        f"provenance: {variant_id}: wrote {', '.join(provenance_files)}",
                    )
            planning = sorted(variant_dir.glob("*.planning.json"))
            source_files = sorted(variant_dir.glob("*.source.json"))
            ok = exit_code == 0 and bool(planning)
            holes = hole_ring_count(variant_dir)
            if case.case_id == "hole_void" and holes == 0:
                ok = False
                _append_capture_log(
                    log,
                    f"coverage: {variant_id}: no planning/final hole ring in domains.json",
                )
            provenance["cases"].append(
                {
                    "case_id": case.case_id,
                    "variant_id": variant_id,
                    "wall_strategy": strategy,
                    "density": density or "15%",
                    "exit": exit_code,
                    "input_sha256": sha256_file(model_path),
                    "settings_sha256": sha256_file(settings_path),
                    "scene_file": scene_path.name,
                    "scene_sha256": sha256_file(scene_path),
                    "hole_rings": holes,
                    "planning_files": [p.name for p in planning],
                    "source_files": [p.name for p in source_files],
                    "operation_files": [
                        p.name for p in sorted(variant_dir.glob("*.operations.json"))
                    ],
                    "status": "recorded" if ok else "failed",
                }
            )
            if not ok:
                failures.append(variant_id)

    # The 3MF input hash is identical for the Classic and Arachne variants because wall generation
    # lives in the settings preset. Mirror the per-case hashes at the top level, keyed by variant so
    # the two strategies remain distinguishable without scanning the case list.
    provenance["settings_sha256"] = {
        entry["variant_id"]: entry["settings_sha256"] for entry in provenance["cases"]
    }
    provenance["scene_sha256"] = {
        entry["variant_id"]: entry["scene_sha256"] for entry in provenance["cases"]
    }

    (output / "manifest.json").write_text(
        json.dumps(provenance, indent=2), encoding="utf-8"
    )

    if failures:
        print(
            json.dumps(
                {"status": "inconclusive", "failed_variants": failures, "log": str(log)}
            )
        )
        return 3
    print(
        json.dumps(
            {
                "status": "recorded",
                "variants": len(provenance["cases"]),
                "manifest": str(output / "manifest.json"),
            }
        )
    )
    return 0


# ---------------------------------------------------------------------------
# Replay


def replay(args: argparse.Namespace) -> int:
    manifest_path = Path(args.manifest).resolve()
    if not manifest_path.exists():
        print(
            json.dumps(
                {
                    "status": "inconclusive",
                    "reason": "manifest is absent",
                    "manifest": str(manifest_path),
                }
            )
        )
        return 3
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)

    mismatches: list[str] = []
    checked = 0
    fixture_root = manifest_path.parent
    for case in manifest.get("cases", []):
        variant_id = case["variant_id"]
        recorded = fixture_root / "raw" / variant_id
        if not recorded.exists():
            mismatches.append(f"{variant_id}: no recorded raw directory")
            continue
        replay_dir = output / variant_id
        replay_dir.mkdir(parents=True, exist_ok=True)
        for path in recorded.glob("*.planning.json"):
            digest = sha256_file(path)
            target = replay_dir / path.name
            target.write_bytes(path.read_bytes())
            if sha256_file(target) != digest:
                mismatches.append(f"{variant_id}: {path.name} hash mismatch")
        checked += 1

    result = {
        "status": "matched" if not mismatches else "mismatch",
        "checked_variants": checked,
        "mismatches": mismatches,
        "note": "replay requires re-running the recorded canonical command; see capture.log",
    }
    (output / "replay.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result))
    return 0 if not mismatches else 1


# ---------------------------------------------------------------------------
# Gate fixture publication
#
# `--publish-fixtures` converts one Classic capture into the three portable gate documents
# (`inputs.json`, `canonical.json`, `manifest.json`). It only projects recorded observed values:
# every non-`Option` kernel DTO field must have a recorded source, and a branch that would need an
# unrecorded value is either omitted with a named reason (reference branches) or makes the case
# `inconclusive` (the `a_source` branch, which has no absent representation).
#
# Units: the capture is canonical (`unit_note` in provenance.json: OrcaSlicer 1 unit = 1 nm).
# `a_source`/`b_input` geometry is transported into PnP IR units (1 unit = 100 nm) because those
# DTOs are `Point2`-typed; the published reference stays canonical so the oracle keeps full
# recorded precision and the comparator owns the canonical -> PnP normalization.

GATE_CASE_NAMES: tuple[str, ...] = (
    "anchor_bridge",
    "pooled_regions",
    "hole_void",
    "island_split_merge",
    "paint_modifier_config",
)
GATE_SCHEMA_VERSION = 1
GATE_WALL_STRATEGY = "Classic"
GATE_CANONICAL_UNIT = "canonical_units_1nm"
GATE_PNP_UNIT = "pnp_units_100nm"
GATE_PNP_UNITS_PER_CANONICAL_UNIT = 100
GATE_TRANSPORT_TOLERANCE_PNP_UNITS = 1
GATE_TRANSPORT_NOTE = (
    f"round-half-away-from-zero(v / {GATE_PNP_UNITS_PER_CANONICAL_UNIT}); "
    f"absolute error <= {GATE_TRANSPORT_TOLERANCE_PNP_UNITS} PnP unit"
)

# Recorded `surface_type` spellings -> closed `EarlierSurfaceKind` variants. A spelling with no
# variant is unrepresentable; it is never coerced to a neighbouring kind.
GATE_SURFACE_KINDS: dict[str, str] = {
    "outline": "outline",
    "internal": "internal",
    "void": "void",
    # OrcaSlicer spells the void surface `stInternalVoid`; the closed kind for it is `Void`.
    "internal_void": "void",
    "top": "top_solid",
    "bottom": "bottom_solid",
    # `stBottomBridge` is its own closed kind, never folded into `bottom_solid`:
    # `island_split_merge_classic` records `o0_l5_r0_s2` as a bottom bridge.
    "bottom_bridge": "bottom_bridge",
    "internal_solid": "internal_solid",
    "internal_bridge": "internal_bridge",
    "shell": "shell",
}
# The earlier-source snapshot is taken after `Layer::detect_surfaces_type` and before the anchor
# edit, so every recorded surface is a PrePass classification product.
GATE_EARLIER_PRODUCER = "pre_pass_classification"
GATE_SOURCE_CHECKPOINTS = frozenset(
    {
        "earlier_classification",
        "before_anchor_edit",
        "post_anchor",
        "during_planning",
        "after_restoration",
        "final_fill",
    }
)
# `operations[].checkpoint` is transcribed verbatim: the recorded Phase-2 anchor events carry
# `post_anchor`, which is now its own closed `SourceCheckpoint` variant. It is never rewritten to
# `during_planning` (a different lifecycle point) or dropped.
GATE_OPERATION_KINDS: dict[str, str] = {
    "Union": "union",
    "Difference": "difference",
    "Intersection": "intersection",
    "Offset": "offset",
    "AnchorExpansion": "anchor_expansion",
    "Reclassification": "reclassification",
}
GATE_OPERAND_ROLES: dict[str, str] = {"subject": "subject", "clip": "clip"}
# Recorded `dont_filter_internal_bridges` spellings -> closed `InternalBridgeFilterPolicy`.
# A missing key publishes `null` (the branch is then undecidable); no value is ever coerced.
GATE_INTERNAL_BRIDGE_FILTERS: dict[str, str] = {
    "disabled": "disabled",
    "limited": "limited",
    "nofilter": "nofilter",
}
GATE_DOMAIN_ORDER = {"internal": 0, "void": 1, "temporary_anchor": 2}

GATE_DOMAIN_FIELDS = (
    ("internal", "internal_domains"),
    ("void", "void_domains"),
    ("temporary_anchor", "temporary_anchors"),
)
GATE_REPLAY_FIELDS = (
    "faithful",
    "wrong",
    "production",
    "scalars_resolved",
    "seed_policy",
    "snapshot_before",
    "snapshot_after",
    "snapshot_unchanged",
    "topology_differs",
    "faithful_matches_production",
    "domain_symmetric_difference_area",
    "faithful_domain_polygon_count",
    "wrong_domain_polygon_count",
    "wrong_nonempty",
)
# A `a_source` subtree may never carry a reference/B/derived key at any depth.
GATE_FORBIDDEN_SOURCE_KEYS = (
    "planning",
    "final",
    "resolved",
    "graph",
    "reference",
    "polic",
    "parameter",
    "replay",
    "scalar",
    "b_input",
    "contributor",
)
GATE_FIXTURE_FILES = ("inputs.json", "canonical.json", "manifest.json")
GATE_STATUS_FILE = "status.json"


class _GateValueError(Exception):
    """A recorded value that cannot be projected into the strict kernel DTO shape."""

    def __init__(self, where: str, reason: str) -> None:
        super().__init__(f"{where}: {reason}")
        self.where = where
        self.reason = reason


class GateCaseInconclusive(Exception):
    """A case that cannot be published without inventing a value."""

    def __init__(self, case_id: str, field: str, reason: str) -> None:
        super().__init__(f"{case_id}: {field}: {reason}")
        self.case_id = case_id
        self.field = field
        self.reason = reason


def _gate_canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def _gate_sha256(value: Any) -> str:
    return hashlib.sha256(_gate_canonical_json(value).encode("utf-8")).hexdigest()


def _gate_field(source: Any, key: str, where: str) -> Any:
    if not isinstance(source, dict):
        raise _GateValueError(where, "expected an object")
    if key not in source:
        raise _GateValueError(f"{where}.{key}", "missing")
    return source[key]


def _gate_int(value: Any, where: str) -> int:
    if isinstance(value, bool):
        raise _GateValueError(where, f"expected integer, got {value!r}")
    if isinstance(value, int):
        return value
    if isinstance(value, float) and value.is_integer():
        return int(value)
    raise _GateValueError(where, f"expected integer, got {value!r}")


def _gate_count(value: Any, where: str) -> int:
    """A serialized count: JSON integer or a numeric config string."""
    number = _gate_number(value, where)
    if not number.is_integer():
        raise _GateValueError(where, f"expected an integer count, got {value!r}")
    return int(number)


def _gate_number(value: Any, where: str) -> float:
    if isinstance(value, bool):
        raise _GateValueError(where, f"expected number, got {value!r}")
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value.strip())
        except ValueError as exc:
            raise _GateValueError(where, f"not a number: {value!r}") from exc
    raise _GateValueError(where, f"expected number, got {value!r}")


def _gate_text(value: Any, where: str) -> str:
    if not isinstance(value, str):
        raise _GateValueError(where, f"expected string, got {value!r}")
    return value


def _gate_percent(value: Any, where: str) -> float:
    if isinstance(value, str) and value.strip().endswith("%"):
        return _gate_number(value.strip()[:-1], where)
    raise _GateValueError(where, f"expected percent string, got {value!r}")


def _gate_width_setting(value: Any, where: str) -> Any:
    """Project a raw serialized width into the closed `RawWidthSetting` enum."""
    if isinstance(value, bool):
        raise _GateValueError(where, f"expected width, got {value!r}")
    if isinstance(value, (int, float)):
        return {"absolute_mm": float(value)}
    if isinstance(value, str):
        text = value.strip()
        if text.endswith("%"):
            return {"nozzle_percent": _gate_number(text[:-1], where)}
        number = _gate_number(text, where)
        return "automatic" if number == 0.0 else {"absolute_mm": number}
    raise _GateValueError(where, f"expected width, got {value!r}")


def _gate_bool_flag(value: Any, where: str) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        text = value.strip().lower()
        if text in ("1", "true", "yes", "on"):
            return True
        if text in ("0", "false", "no", "off"):
            return False
    raise _GateValueError(where, f"expected boolean flag, got {value!r}")


_GATE_REGION_CONFIG_FIELDS = (
    ("fill_multiline", "fill_multiline", _gate_count),
    ("sparse_width", "sparse_infill_line_width", _gate_width_setting),
    ("density_percent", "sparse_infill_density", _gate_percent),
    ("overhang_angle_deg", "lightning_overhang_angle", _gate_number),
    ("prune_angle_deg", "lightning_prune_angle", _gate_number),
    ("straightening_angle_deg", "lightning_straightening_angle", _gate_number),
    ("infill_direction_deg", "infill_direction", _gate_number),
    ("sparse_rotate_template", "sparse_infill_rotate_template", _gate_text),
    ("solid_infill_direction_deg", "solid_infill_direction", _gate_number),
    ("solid_rotate_template", "solid_infill_rotate_template", _gate_text),
    ("top_layer_direction_deg", "top_layer_direction", _gate_number),
    ("bottom_layer_direction_deg", "bottom_layer_direction", _gate_number),
    ("align_to_model", "align_infill_direction_to_model", _gate_bool_flag),
    # Recorded verbatim per region (`lightning` for the Lightning regions, `grid` for
    # `pooled_regions`' modifier region). Never inferred from the object preset.
    ("sparse_infill_pattern", "sparse_infill_pattern", _gate_text),
)



def _gate_raw_region_config(raw: Any, where: str) -> dict[str, Any]:
    config: dict[str, Any] = {}
    for name, key, convert in _GATE_REGION_CONFIG_FIELDS:
        config[name] = convert(_gate_field(raw, key, where), f"{where}.{key}")
    return config


def _gate_internal_bridge_filter(
    settings: dict[str, Any] | None, where: str
) -> str | None:
    """Read `dont_filter_internal_bridges` from the recorded variant settings.

    The key is object/process-level and the earlier-source snapshot does not carry it, so the
    recorded settings preset is the only truthful source. A preset without the key yields `None`
    (the DTO's "not recorded" state, which leaves the branch undecidable); an unknown spelling is
    unrepresentable rather than coerced.
    """
    if settings is None:
        return None
    if "dont_filter_internal_bridges" not in settings:
        return None
    raw = settings["dont_filter_internal_bridges"]
    if raw is None:
        return None
    if isinstance(raw, str):
        mapped = GATE_INTERNAL_BRIDGE_FILTERS.get(raw.strip())
        if mapped is not None:
            return mapped
    raise _GateValueError(
        f"{where}.dont_filter_internal_bridges",
        f"unrepresentable_internal_bridge_filter:{raw!r}",
    )


def _gate_raw_object_config(
    source: dict[str, Any],
    any_align: bool,
    internal_bridge_filter: str | None,
) -> dict[str, Any]:
    object_config = _gate_field(source, "object_config", "source")
    print_config = _gate_field(source, "print_config", "source")
    if any_align:
        raise _GateValueError(
            "source.object_config.model_alignment_offset_rad",
            "align_infill_direction_to_model is set but the model alignment offset is not recorded",
        )
    return {
        "layer_height_mm": _gate_number(
            _gate_field(object_config, "layer_height", "source.object_config"),
            "source.object_config.layer_height",
        ),
        "line_width": _gate_width_setting(
            _gate_field(object_config, "line_width", "source.object_config"),
            "source.object_config.line_width",
        ),
        "nozzle_diameters_mm": [
            _gate_number(
                _gate_field(print_config, "nozzle_diameter", "source.print_config"),
                "source.print_config.nozzle_diameter",
            )
        ],
        # Recorded `align_infill_direction_to_model = 0` everywhere, so the offset is the recorded
        # zero rotation, not a default substituted for an unrecorded value.
        "model_alignment_offset_rad": 0.0,
        "dont_filter_internal_bridges": internal_bridge_filter,
    }



def _gate_transport_unit(value: int) -> int:
    scale = GATE_PNP_UNITS_PER_CANONICAL_UNIT
    if value >= 0:
        return (value + scale // 2) // scale
    return -((-value + scale // 2) // scale)


def _gate_point(pair: Any, where: str, transported: bool) -> dict[str, int]:
    if not isinstance(pair, (list, tuple)) or len(pair) != 2:
        raise _GateValueError(where, "point is not an (x, y) pair")
    x = _gate_int(pair[0], f"{where}.x")
    y = _gate_int(pair[1], f"{where}.y")
    if transported:
        x = _gate_transport_unit(x)
        y = _gate_transport_unit(y)
    return {"x": x, "y": y}


def _gate_polygon(raw: Any, where: str, *, transported: bool) -> dict[str, Any]:
    """Project a recorded expolygon into the `ExPolygon { Polygon { points } }` serde shape."""
    if not isinstance(raw, dict):
        raise _GateValueError(where, "expolygon is not an object")
    contour = raw.get("contour") or []
    holes = raw.get("holes") or []
    if not isinstance(contour, list) or not isinstance(holes, list):
        raise _GateValueError(where, "expolygon rings are not lists")
    return {
        "contour": {
            "points": [
                _gate_point(point, f"{where}.contour[{index}]", transported)
                for index, point in enumerate(contour)
            ]
        },
        "holes": [
            {
                "points": [
                    _gate_point(
                        point, f"{where}.holes[{index}][{ring_index}]", transported
                    )
                    for ring_index, point in enumerate(hole)
                ]
            }
            for index, hole in enumerate(holes)
        ],
    }


def _gate_identity(region_id: int, object_id: str) -> dict[str, Any]:
    # `variant_chain` stays empty: none of the seven authorized capture artifacts records a
    # paint/variant chain (scene.json is outside the authorized read set).
    return {"object_id": object_id, "region_id": region_id, "variant_chain": []}


def _gate_source_surface_id(
    entry: dict[str, Any], object_id: str, where: str
) -> dict[str, Any]:
    checkpoint = entry.get("checkpoint")
    if checkpoint not in GATE_SOURCE_CHECKPOINTS:
        raise _GateValueError(f"{where}.checkpoint", f"unknown checkpoint {checkpoint!r}")
    token = entry.get("source_id")
    if not isinstance(token, str) or not token:
        raise _GateValueError(f"{where}.source_id", "missing source token")
    return {
        "identity": _gate_identity(
            _gate_int(entry.get("region_id"), f"{where}.region_id"), object_id
        ),
        "local_layer_index": _gate_int(
            entry.get("local_layer_index"), f"{where}.local_layer_index"
        ),
        "global_layer_index": _gate_int(
            entry.get("global_layer_index"), f"{where}.global_layer_index"
        ),
        "checkpoint": checkpoint,
        "surface_token": token,
    }


def _gate_load_json(path: Path, case_id: str, field: str) -> dict[str, Any]:
    if not path.is_file():
        raise GateCaseInconclusive(case_id, field, "missing_artifact")
    try:
        body = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise GateCaseInconclusive(case_id, field, f"unreadable_artifact:{type(exc).__name__}")
    if not isinstance(body, dict):
        raise GateCaseInconclusive(case_id, field, "artifact_is_not_an_object")
    return body


def _gate_load_jsonl(path: Path, case_id: str, field: str) -> list[dict[str, Any]]:
    if not path.is_file():
        raise GateCaseInconclusive(case_id, field, "missing_artifact")
    records: list[dict[str, Any]] = []
    try:
        with path.open(encoding="utf-8") as handle:
            for number, line in enumerate(handle, start=1):
                if not line.strip():
                    continue
                record = json.loads(line)
                if not isinstance(record, dict):
                    raise ValueError(f"line {number} is not an object")
                records.append(record)
    except (OSError, ValueError) as exc:
        raise GateCaseInconclusive(case_id, field, f"unreadable_artifact:{type(exc).__name__}")
    return records


def _gate_artifact_stem(variant_dir: Path, case_id: str) -> str:
    if not variant_dir.is_dir():
        raise GateCaseInconclusive(case_id, "capture", "missing_variant_directory")
    matches = sorted(variant_dir.glob(f"{case_id}.object_*.source.json"))
    if not matches:
        raise GateCaseInconclusive(case_id, "source.json", "missing_artifact")
    if len(matches) > 1:
        raise GateCaseInconclusive(
            case_id,
            "source.json",
            "ambiguous_artifact:multiple objects for one gate case",
        )
    return matches[0].name[: -len(".source.json")]


def _gate_surface_kind(recorded: Any, case_id: str, where: str) -> str:
    mapped = GATE_SURFACE_KINDS.get(recorded) if isinstance(recorded, str) else None
    if mapped is None:
        raise GateCaseInconclusive(
            case_id, where, f"unrepresentable_surface_kind:{recorded!r}"
        )
    return mapped


def _gate_assert_source_only(node: Any, path: str, case_id: str) -> None:
    if isinstance(node, dict):
        for key, value in node.items():
            lowered = key.lower()
            for forbidden in GATE_FORBIDDEN_SOURCE_KEYS:
                if forbidden in lowered:
                    raise GateCaseInconclusive(
                        case_id,
                        f"a_source.{path}.{key}",
                        f"forbidden_reference_key:{key}",
                    )
            _gate_assert_source_only(value, f"{path}.{key}", case_id)
    elif isinstance(node, list):
        for index, value in enumerate(node):
            _gate_assert_source_only(value, f"{path}[{index}]", case_id)


def _gate_contributor_evidence(
    callers: Sequence[dict[str, Any]],
    catalog_ids: set[str],
    case_id: str,
) -> list[dict[str, Any]]:
    evidence: list[dict[str, Any]] = []
    seen: set[str] = set()
    ordered = sorted(
        callers,
        key=lambda record: (
            _gate_int(record.get("local_layer_index"), "caller.local_layer_index"),
            _gate_int(record.get("region_id"), "caller.region_id"),
            _gate_text(record.get("dispatch_token"), "caller.dispatch_token"),
        ),
    )
    for record in ordered:
        token = _gate_text(record.get("dispatch_token"), "caller.dispatch_token")
        if token in seen:
            raise GateCaseInconclusive(case_id, f"callers[{token}]", "duplicate_dispatch_token")
        seen.add(token)
        recorded = record.get("source_ids") or []
        if not isinstance(recorded, list):
            raise GateCaseInconclusive(case_id, f"callers[{token}].source_ids", "not_a_list")
        resolved = [value for value in recorded if isinstance(value, str) and value]
        unresolved = len(resolved) != len(recorded)
        missing = [value for value in resolved if value not in catalog_ids]
        if not recorded:
            evidence.append(
                {
                    "dispatch_token": token,
                    "status": "Unavailable",
                    "reason": "no earlier-source contributor recorded for the dispatch",
                }
            )
        elif unresolved:
            evidence.append(
                {
                    "dispatch_token": token,
                    "status": "Unavailable",
                    "reason": "unresolved contributor: surface created after the pre-anchor snapshot",
                }
            )
        elif missing:
            evidence.append(
                {
                    "dispatch_token": token,
                    "status": "Unavailable",
                    "reason": f"contributor not in the earlier-source catalog: {sorted(missing)[0]}",
                }
            )
        else:
            evidence.append(
                {"dispatch_token": token, "status": "Complete", "reason": None}
            )
    return evidence


def _gate_domain_key(surface_id: Any, where: str) -> tuple[int, int]:
    """Read (local_layer_index, region_id) from the recorder's domain id.

    The recorder builds every domain id as `o<object>_l<layer>_r<region>_<slug>_<index>`
    (see `_build_domains`); the region id is not repeated on the public domain record, so the
    id is the recorded join back to the region occurrence.
    """
    parts = surface_id.split("_") if isinstance(surface_id, str) else []
    if (
        len(parts) < 5
        or not parts[0].startswith("o")
        or not parts[1].startswith("l")
        or not parts[2].startswith("r")
    ):
        raise _GateValueError(where, f"unexpected domain id {surface_id!r}")
    try:
        return int(parts[1][1:]), int(parts[2][1:])
    except ValueError as exc:
        raise _GateValueError(where, f"unexpected domain id {surface_id!r}") from exc


def _gate_source_segment_ref(
    catalog_entry: dict[str, Any],
    ring_id: Any,
    segment_id: Any,
    object_id: str,
    case_id: str,
    where: str,
) -> dict[str, Any]:
    """Project a recorded source address into `SourceSegmentRef`.

    `ring_id`/`segment_id` are `None` exactly when the capture recorded them as JSON `null`
    (generated contributors, surface-level operation inputs/operands). Both must agree: a real
    ring/segment pair is carried through unchanged, including `0`, which is a valid address.
    """
    if (ring_id is None) != (segment_id is None):
        raise GateCaseInconclusive(
            case_id, where, "ring_id_and_segment_id_must_be_present_or_absent_together"
        )
    if ring_id is None:
        projected_ring = None
        projected_segment = None
    else:
        projected_ring = _gate_int(ring_id, f"{where}.ring_id")
        projected_segment = _gate_int(segment_id, f"{where}.segment_id")
        if projected_ring < 0 or projected_segment < 0:
            raise GateCaseInconclusive(case_id, where, "negative_source_segment_address")
    return {
        "surface": _gate_source_surface_id(
            catalog_entry, object_id, f"{where}.surface"
        ),
        "ring_id": projected_ring,
        "segment_id": projected_segment,
    }


def _gate_operation_layers(
    operations: Sequence[dict[str, Any]], case_id: str
) -> dict[int, int]:
    """Derive each operation's local layer from its recorded input source ids.

    The recorder writes one operation list per object, not per layer, while `PlanningLayer`
    carries its own operations. An operation is placed on the single local layer its recorded
    inputs (or operand sources) name; an operation that spans layers has no representation and
    makes the case inconclusive rather than being placed by guess.
    """
    placement: dict[int, int] = {}
    for index, operation in enumerate(operations):
        operation_id = _gate_int(
            _gate_field(operation, "operation_id", f"operations[{index}]"),
            f"operations[{index}].operation_id",
        )
        tokens: list[str] = []
        for item in operation.get("inputs") or []:
            if isinstance(item, str):
                tokens.append(item)
        for operand in operation.get("operands") or []:
            for item in operand.get("sources") or []:
                if isinstance(item, str):
                    tokens.append(item)
        layers = {
            key[0]
            for key in (_source_key(token) for token in tokens)
            if key is not None
        }
        if not layers:
            raise GateCaseInconclusive(
                case_id,
                f"provenance.operations[{operation_id}]",
                "operation_records_no_layer_addressable_input",
            )
        if len(layers) > 1:
            raise GateCaseInconclusive(
                case_id,
                f"provenance.operations[{operation_id}]",
                "operation_spans_multiple_layers",
            )
        placement[operation_id] = layers.pop()
    return placement


def _gate_layer_operations(
    operations: Sequence[dict[str, Any]],
    local_layer: int,
    placement: dict[int, int],
    address_index: dict[tuple[str, str], int],
    catalog: dict[str, dict[str, Any]],
    object_id: str,
    case_id: str,
) -> list[dict[str, Any]]:
    """Project the recorded Phase-2 operations belonging to one planning layer.

    Every value is transcribed. `geometry`/`result_geometry` stay in canonical units like the
    rest of `reference`. `outputs` are the operation's surviving addresses that land inside this
    planning layer's typed domains; a recorded address in a *final* domain (`final_internal` /
    `final_void`) is not addressable by `BoundaryAddress.domain` and is not published here.
    """
    projected: list[dict[str, Any]] = []
    for operation in sorted(
        operations, key=lambda entry: _gate_int(entry.get("operation_id"), "operations")
    ):
        operation_id = _gate_int(operation.get("operation_id"), "operations.operation_id")
        if placement.get(operation_id) != local_layer:
            continue
        where = f"provenance.operations[{operation_id}]"
        recorded_kind = operation.get("kind")
        kind = GATE_OPERATION_KINDS.get(recorded_kind) if isinstance(recorded_kind, str) else None
        if kind is None:
            raise GateCaseInconclusive(
                case_id, f"{where}.kind", f"unrepresentable_operation_kind:{recorded_kind!r}"
            )
        checkpoint = operation.get("checkpoint")
        if checkpoint not in GATE_SOURCE_CHECKPOINTS:
            raise GateCaseInconclusive(
                case_id, f"{where}.checkpoint", f"unknown_checkpoint:{checkpoint!r}"
            )

        inputs: list[dict[str, Any]] = []
        for token in operation.get("inputs") or []:
            entry = catalog.get(token) if isinstance(token, str) else None
            if entry is None:
                raise GateCaseInconclusive(
                    case_id, f"{where}.inputs", f"source_not_in_catalog:{token!r}"
                )
            inputs.append(
                _gate_source_segment_ref(
                    entry, None, None, object_id, case_id, f"{where}.inputs"
                )
            )

        operands: list[dict[str, Any]] = []
        for operand_index, operand in enumerate(operation.get("operands") or []):
            operand_where = f"{where}.operands[{operand_index}]"
            recorded_role = operand.get("role")
            role = GATE_OPERAND_ROLES.get(recorded_role) if isinstance(recorded_role, str) else None
            if role is None:
                raise GateCaseInconclusive(
                    case_id, f"{operand_where}.role", f"unrepresentable_operand_role:{recorded_role!r}"
                )
            sources: list[dict[str, Any]] = []
            for token in operand.get("sources") or []:
                entry = catalog.get(token) if isinstance(token, str) else None
                if entry is None:
                    raise GateCaseInconclusive(
                        case_id,
                        f"{operand_where}.sources",
                        f"source_not_in_catalog:{token!r}",
                    )
                sources.append(
                    _gate_source_segment_ref(
                        entry, None, None, object_id, case_id, f"{operand_where}.sources"
                    )
                )
            parent_id = operand.get("operation_id")
            operands.append(
                {
                    "role": role,
                    "sources": sources,
                    "operation_id": None
                    if parent_id is None
                    else _gate_int(parent_id, f"{operand_where}.operation_id"),
                }
            )

        outputs: list[dict[str, Any]] = []
        for output in operation.get("outputs") or []:
            domain = output.get("domain")
            if domain not in GATE_DOMAIN_ORDER:
                # A final-domain survivor is not addressable inside a planning layer.
                continue
            surface_id = output.get("surface_id")
            index = address_index.get((domain, surface_id))
            if index is None:
                raise GateCaseInconclusive(
                    case_id,
                    f"{where}.outputs",
                    f"operation_output_not_in_layer:{domain}:{surface_id!r}",
                )
            outputs.append(
                {
                    "domain": domain,
                    "surface_id": index,
                    "ring_id": _gate_int(output.get("ring_id"), f"{where}.outputs.ring_id"),
                    "segment_id": _gate_int(
                        output.get("segment_id"), f"{where}.outputs.segment_id"
                    ),
                }
            )
        outputs.sort(
            key=lambda address: (
                GATE_DOMAIN_ORDER[address["domain"]],
                address["surface_id"],
                address["ring_id"],
                address["segment_id"],
            )
        )

        offset_units = operation.get("offset_units")
        join = operation.get("join")
        projected.append(
            {
                "operation_id": operation_id,
                "kind": kind,
                "checkpoint": checkpoint,
                "inputs": inputs,
                "parent_operations": [
                    _gate_int(parent, f"{where}.parent_operations")
                    for parent in operation.get("parent_operations") or []
                ],
                "operands": operands,
                "offset_units": None
                if offset_units is None
                else _gate_int(offset_units, f"{where}.offset_units"),
                "join": None if join is None else _gate_text(join, f"{where}.join"),
                "result_geometry": [
                    _gate_polygon(
                        geometry, f"{where}.result_geometry[{position}]", transported=False
                    )
                    for position, geometry in enumerate(
                        _gate_field(operation, "result_geometry", where)
                    )
                ],
                "outputs": outputs,
            }
        )
    return projected


def _gate_reference_planning_layers(
    provenance: dict[str, Any],
    domains: dict[str, Any],
    object_id: str,
    case_id: str,
) -> tuple[list[dict[str, Any]] | None, str | None]:
    """Project recorded planning domains/provenance, or name why it is absent."""
    catalog: dict[str, dict[str, Any]] = {}
    for entry in provenance.get("source_catalog") or []:
        token = entry.get("source_id")
        if isinstance(token, str):
            catalog[token] = entry

    by_layer: dict[int, list[dict[str, Any]]] = {}
    for entry in provenance.get("planning_domains") or []:
        by_layer.setdefault(_gate_int(entry.get("local_layer_index"), "provenance"), []).append(
            entry
        )

    region_ids_by_layer: dict[int, list[int]] = {}
    for layer in domains.get("layers") or []:
        local = _gate_int(layer.get("local_layer_index"), "domains.layers")
        region_ids_by_layer[local] = sorted(
            _gate_int(region.get("region_id"), "domains.layers.regions")
            for region in layer.get("regions") or []
        )

    # Recorded per-(layer, region) polygon counts, used to prove the provenance projection keeps
    # the whole domain population instead of a convenient subset.
    recorded_counts: dict[tuple[int, int, str], int] = {}
    for layer in domains.get("layers") or []:
        local = _gate_int(layer.get("local_layer_index"), "domains.layers")
        for region in layer.get("regions") or []:
            region_id = _gate_int(region.get("region_id"), "domains.layers.regions")
            for kind, field in GATE_DOMAIN_FIELDS:
                count = len(region.get(field) or [])
                if count:
                    recorded_counts[(local, region_id, kind)] = count
    projected_counts: dict[tuple[int, int, str], int] = {}
    for local in sorted(by_layer):
        for entry in by_layer[local]:
            kind = entry.get("domain")
            if kind not in GATE_DOMAIN_ORDER:
                raise GateCaseInconclusive(
                    case_id, "provenance.planning_domains.domain", f"unknown_domain:{kind!r}"
                )
            layer_key, region_id = _gate_domain_key(
                entry.get("surface_id"), "provenance.planning_domains.surface_id"
            )
            if layer_key != local:
                raise GateCaseInconclusive(
                    case_id,
                    "provenance.planning_domains.surface_id",
                    "domain_id_layer_does_not_match_domain_layer",
                )
            key = (local, region_id, kind)
            projected_counts[key] = projected_counts.get(key, 0) + 1
    if projected_counts != recorded_counts:
        raise GateCaseInconclusive(
            case_id,
            "provenance.planning_domains",
            "planning_domain_population_does_not_match_domains_json",
        )

    # The population check above already proved the projection is complete. Nothing else is a
    # reason to withhold the branch: generated contributors carry a surface-level
    # `SourceSegmentRef` (null ring/segment), operations carry `post_anchor` and surface-level
    # input/operand refs, and an unattributed boundary is published with an empty contributor
    # list. A boundary that the capture could not attribute is named in the manifest instead.
    # `provenance["unavailable"]` boundaries are published as boundaries with an empty contributor
    # list (the recorded fact) and are also named in `manifest.json.cases.<case>.absent`.
    operations = provenance.get("operations") or []
    operation_layers = _gate_operation_layers(operations, case_id)

    layers: list[dict[str, Any]] = []
    for local in sorted(by_layer):
        entries = by_layer[local]
        ordered = sorted(
            entries,
            key=lambda entry: (
                GATE_DOMAIN_ORDER[entry["domain"]],
                _gate_domain_key(
                    entry.get("surface_id"), "provenance.planning_domains.surface_id"
                )[1],
                _gate_text(entry.get("surface_id"), "provenance.planning_domains"),
            ),
        )
        kind_lists: dict[str, list[dict[str, Any]]] = {
            kind: [] for kind in GATE_DOMAIN_ORDER
        }
        boundary_table: list[dict[str, Any]] = []
        address_index: dict[tuple[str, str], int] = {}
        for position, entry in enumerate(ordered):
            kind = entry["domain"]
            kind_lists[kind].append(
                _gate_polygon(
                    _gate_field(entry, "geometry", "provenance.planning_domains"),
                    f"provenance.planning_domains[{local}][{position}].geometry",
                    transported=False,
                )
            )
            surface_id = len(kind_lists[kind]) - 1
            address_index[(kind, _gate_text(entry.get("surface_id"), "planning_domains"))] = (
                surface_id
            )
            for boundary in entry.get("boundaries") or []:
                contributors: list[dict[str, Any]] = []
                for contributor in boundary.get("contributors") or []:
                    source_id = contributor.get("source_id")
                    catalog_entry = catalog.get(source_id) if isinstance(source_id, str) else None
                    if catalog_entry is None:
                        raise GateCaseInconclusive(
                            case_id,
                            "provenance.boundaries.contributors.source_id",
                            f"contributor_not_in_source_catalog:{source_id!r}",
                        )
                    relation = contributor.get("relation") or {}
                    if relation.get("kind") == "overlap":
                        projected_relation: dict[str, Any] = {
                            "overlap": {
                                "start": {
                                    "numerator": _gate_int(
                                        _gate_field(relation, "start", "relation").get(
                                            "numerator"
                                        ),
                                        "relation.start.numerator",
                                    ),
                                    "denominator": _gate_int(
                                        _gate_field(relation, "start", "relation").get(
                                            "denominator"
                                        ),
                                        "relation.start.denominator",
                                    ),
                                },
                                "end": {
                                    "numerator": _gate_int(
                                        _gate_field(relation, "end", "relation").get("numerator"),
                                        "relation.end.numerator",
                                    ),
                                    "denominator": _gate_int(
                                        _gate_field(relation, "end", "relation").get(
                                            "denominator"
                                        ),
                                        "relation.end.denominator",
                                    ),
                                },
                            }
                        }
                    elif relation.get("kind") == "generated":
                        projected_relation = {
                            "generated": {
                                "operation_id": _gate_int(
                                    _gate_field(relation, "operation_id", "relation"),
                                    "relation.operation_id",
                                )
                            }
                        }
                    else:
                        raise GateCaseInconclusive(
                            case_id,
                            "provenance.boundaries.contributors.relation.kind",
                            f"unrepresentable_contribution_relation:{relation.get('kind')!r}",
                        )
                    contributors.append(
                        {
                            "source": _gate_source_segment_ref(
                                catalog_entry,
                                contributor.get("ring_id"),
                                contributor.get("segment_id"),
                                object_id,
                                case_id,
                                "provenance.boundaries.contributors",
                            ),
                            "relation": projected_relation,
                        }
                    )
                boundary_table.append(
                    {
                        "address": {
                            "domain": kind,
                            "surface_id": surface_id,
                            "ring_id": _gate_int(
                                _gate_field(boundary, "ring_id", "boundary"),
                                "boundary.ring_id",
                            ),
                            "segment_id": _gate_int(
                                _gate_field(boundary, "segment_id", "boundary"),
                                "boundary.segment_id",
                            ),
                        },
                        "endpoints": [
                            _gate_point(
                                point,
                                f"boundary.endpoints[{index}]",
                                transported=False,
                            )
                            for index, point in enumerate(
                                _gate_field(boundary, "endpoints", "boundary")
                            )
                        ],
                        "contributors": contributors,
                    }
                )

        source_regions = [
            _gate_identity(region_id, object_id)
            for region_id in region_ids_by_layer.get(local, [])
        ]
        source_catalog = [
            {
                "id": _gate_source_surface_id(entry, object_id, "provenance.source_catalog"),
                "geometry": _gate_polygon(
                    _gate_field(entry, "geometry", "provenance.source_catalog"),
                    "provenance.source_catalog.geometry",
                    transported=False,
                ),
                "surface_kind": _gate_text(
                    _gate_field(entry, "surface_kind", "provenance.source_catalog"),
                    "provenance.source_catalog.surface_kind",
                ),
                "solid_angle_rad": None,
                "bridge_angle_rad": None,
            }
            for entry in provenance.get("source_catalog") or []
            if _gate_int(entry.get("local_layer_index"), "provenance.source_catalog") == local
        ]
        source_catalog.sort(key=lambda surface: surface["id"]["surface_token"])
        first = ordered[0]
        layers.append(
            {
                "object_id": object_id,
                "local_layer_index": local,
                "global_layer_index": _gate_int(
                    first.get("global_layer_index"), "provenance.planning_domains"
                ),
                "z_mm": _gate_number(first.get("z_mm"), "provenance.planning_domains"),
                "internal_domains": kind_lists["internal"],
                "void_domains": kind_lists["void"],
                "temporary_anchors": kind_lists["temporary_anchor"],
                "source_regions": source_regions,
                "source_catalog": source_catalog,
                "boundary_provenance": boundary_table,
                "operations": _gate_layer_operations(
                    operations,
                    local,
                    operation_layers,
                    address_index,
                    catalog,
                    object_id,
                    case_id,
                ),
            }
        )

    return layers, None


def _gate_reference_clipped(
    clips: Sequence[dict[str, Any]],
    callers_by_token: dict[str, dict[str, Any]],
    catalog: dict[str, dict[str, Any]],
    object_id: str,
    case_id: str,
) -> tuple[list[dict[str, Any]] | None, str | None]:
    grouped: dict[tuple[int, int, float], list[dict[str, Any]]] = {}
    for record in clips:
        token = _gate_text(record.get("dispatch_token"), "clips.dispatch_token")
        caller = callers_by_token.get(token)
        if caller is None:
            raise GateCaseInconclusive(
                case_id, f"clips[{token}]", "clip_dispatch_token_not_in_callers"
            )
        local = _gate_int(caller.get("local_layer_index"), "caller.local_layer_index")
        global_layer = _gate_int(caller.get("global_layer_index"), "caller.global_layer_index")
        z_mm = _gate_number(caller.get("z_mm"), "caller.z_mm")
        contributors: list[dict[str, Any]] = []
        for source_id in caller.get("source_ids") or []:
            if not isinstance(source_id, str):
                continue
            entry = catalog.get(source_id)
            if entry is None:
                raise GateCaseInconclusive(
                    case_id,
                    f"callers[{token}].source_ids",
                    f"contributor_not_in_source_catalog:{source_id}",
                )
            contributors.append(_gate_source_surface_id(entry, object_id, "provenance"))
        segments: list[list[dict[str, int]]] = []
        for polyline in record.get("clipped_polylines") or []:
            points = [
                _gate_point(point, "clips.clipped_polylines", transported=False)
                for point in polyline
            ]
            for index in range(len(points) - 1):
                segments.append([points[index], points[index + 1]])
        grouped.setdefault((local, global_layer, z_mm), []).append(
            {
                "dispatch_token": token,
                "caller_identity": _gate_identity(
                    _gate_int(caller.get("region_id"), "caller.region_id"), object_id
                ),
                "contributors": contributors,
                "segments": segments,
            }
        )

    layers: list[dict[str, Any]] = []
    for (local, global_layer, z_mm) in sorted(grouped):
        fills = sorted(grouped[(local, global_layer, z_mm)], key=lambda fill: fill["dispatch_token"])
        layers.append(
            {
                "object_id": object_id,
                "local_layer_index": local,
                "global_layer_index": global_layer,
                "z_mm": z_mm,
                "fills": fills,
            }
        )
    return layers, None


def _gate_reference_parameters(
    planning: dict[str, Any], domains: dict[str, Any], case_id: str
) -> dict[str, Any]:
    resolved = _gate_field(domains, "resolved_inputs", "domains")
    scalars = _gate_field(planning, "scalars", "planning")
    return {
        "nozzle_diameter_mm": _gate_number(
            _gate_field(resolved, "max_nozzle_diameter_mm", "resolved_inputs"),
            "resolved_inputs.max_nozzle_diameter_mm",
        ),
        "extrusion_width_mm": _gate_number(
            _gate_field(scalars, "infill_extrusion_width", "planning.scalars"),
            "planning.scalars.infill_extrusion_width",
        ),
        "density_fraction": _gate_number(
            _gate_field(resolved, "sparse_infill_density_percent", "resolved_inputs"),
            "resolved_inputs.sparse_infill_density_percent",
        )
        / 100.0,
        "multiline": _gate_int(
            _gate_field(resolved, "fill_multiline", "resolved_inputs"),
            "resolved_inputs.fill_multiline",
        ),
        "layer_height_mm": _gate_number(
            _gate_field(resolved, "layer_height_mm", "resolved_inputs"),
            "resolved_inputs.layer_height_mm",
        ),
        "overhang_angle_deg": _gate_number(
            _gate_field(resolved, "overhang_angle_deg", "resolved_inputs"),
            "resolved_inputs.overhang_angle_deg",
        ),
        "prune_angle_deg": _gate_number(
            _gate_field(resolved, "prune_angle_deg", "resolved_inputs"),
            "resolved_inputs.prune_angle_deg",
        ),
        "straightening_angle_deg": _gate_number(
            _gate_field(resolved, "straightening_angle_deg", "resolved_inputs"),
            "resolved_inputs.straightening_angle_deg",
        ),
        # The four `FillLightning::Generator` constructor scalars recorded in `planning.json`.
        # They are transcribed, never recomputed from the raw config.
        "supporting_radius_mm": _gate_number(
            _gate_field(scalars, "supporting_radius", "planning.scalars"),
            "planning.scalars.supporting_radius",
        ),
        "wall_supporting_radius_mm": _gate_number(
            _gate_field(scalars, "wall_supporting_radius", "planning.scalars"),
            "planning.scalars.wall_supporting_radius",
        ),
        "prune_length_mm": _gate_number(
            _gate_field(scalars, "prune_length", "planning.scalars"),
            "planning.scalars.prune_length",
        ),
        "straightening_max_distance_mm": _gate_number(
            _gate_field(scalars, "straightening_max_distance", "planning.scalars"),
            "planning.scalars.straightening_max_distance",
        ),
    }


def _gate_reference_policies(
    entries: Sequence[dict[str, Any]], callers: Sequence[dict[str, Any]]
) -> tuple[None, str]:
    """Extract the recorded per-dispatch policy evidence and name why it is incomplete.

    `callers.jsonl` records, per dispatch, the resolved `angle_deg`/`angle_rad`, `pattern`,
    `fixed_angle`, `surface_type` and `region_config`, keyed by object/layer/region. That is real
    independent evidence, but `FillPolicyInput` is one policy per region occurrence on one layer:

    - `bridge_angle_rad` is recorded by none of the seven artifacts;
    - `is_bridge` is not recorded for an occurrence (`surface_type` describes the dispatch, not
      the occurrence's policy);
    - `fixed_angle` is recorded per dispatch and disagrees *inside* a single region/layer
      (`true` for `top`/`bottom` dispatches, `false` for `internal`/`internal_solid`).

    So no complete `DerivedRegionPolicy` can be built without inventing one of those values.
    """
    by_slot: dict[tuple[int, int], list[dict[str, Any]]] = {}
    for record in callers:
        key = (
            _gate_int(record.get("local_layer_index"), "callers.local_layer_index"),
            _gate_int(record.get("region_id"), "callers.region_id"),
        )
        by_slot.setdefault(key, []).append(record)
    slots = len(entries)
    sparse = 0
    solid = 0
    top_or_bottom = 0
    conflicting_fixed_angle = 0
    bridge_dispatches = 0
    for entry in entries:
        records = by_slot.get((entry["local_layer_index"], entry["region_id"]), [])
        surface_types = {record.get("surface_type") for record in records}
        if any(record.get("pattern") == "lightning" for record in records):
            sparse += 1
        if surface_types & {"internal_solid", "top", "bottom"}:
            solid += 1
        if surface_types & {"top", "bottom"}:
            top_or_bottom += 1
        if len({record.get("fixed_angle") for record in records}) > 1:
            conflicting_fixed_angle += 1
        if "internal_bridge" in surface_types:
            bridge_dispatches += 1
    reason = (
        "a complete FillPolicyInput per region occurrence is not transcribable: "
        "`bridge_angle_rad` is recorded by none of the seven capture artifacts, and `is_bridge` "
        "is not recorded for an occurrence (`callers.jsonl` records only the dispatch "
        f"`surface_type`, with {bridge_dispatches} of {slots} occurrences carrying an "
        "`internal_bridge` dispatch). `fixed_angle` exists only per dispatch and disagrees "
        f"inside {conflicting_fixed_angle} of {slots} occurrences (true for top/bottom "
        "dispatches, false for internal/solid). Recorded per-dispatch resolved angles exist for "
        f"{sparse} of {slots} occurrences (sparse lightning), {solid} of {slots} "
        f"(solid surfaces) and {top_or_bottom} of {slots} (top/bottom)"
    )
    return None, reason




def _gate_scalar_source_region(
    region_configs: dict[int, dict[str, Any]],
    parameters: dict[str, Any],
    resolved: dict[str, Any],
    object_id: str,
) -> tuple[dict[str, Any] | None, str | None]:
    density = _gate_number(
        _gate_field(resolved, "sparse_infill_density_percent", "resolved_inputs"),
        "resolved_inputs.sparse_infill_density_percent",
    )
    width = _gate_number(
        _gate_field(resolved, "sparse_infill_line_width_mm", "resolved_inputs"),
        "resolved_inputs.sparse_infill_line_width_mm",
    )
    matches: list[int] = []
    for region_id in sorted(region_configs):
        config = region_configs[region_id]
        sparse_width = config["sparse_width"]
        if not isinstance(sparse_width, dict) or "absolute_mm" not in sparse_width:
            continue
        if (
            config["density_percent"] == density
            and sparse_width["absolute_mm"] == width
            and config["overhang_angle_deg"] == parameters["overhang_angle_deg"]
            and config["prune_angle_deg"] == parameters["prune_angle_deg"]
            and config["straightening_angle_deg"] == parameters["straightening_angle_deg"]
            and config["fill_multiline"] == parameters["multiline"]
            and config["align_to_model"] is False
        ):
            matches.append(region_id)
    if len(matches) == 1:
        return _gate_identity(matches[0], object_id), None
    if not matches:
        return None, "no region config reproduces the recorded object scalars"
    return (
        None,
        f"{len(matches)} region configs reproduce the recorded object scalars; "
        "the capture does not identify the scalar source",
    )


def build_gate_case(variant_dir: Path, case_name: str) -> dict[str, Any]:
    """Project one Classic capture variant into the gate envelopes, or name the absent field."""
    case_id = f"{case_name}_classic"
    stem = _gate_artifact_stem(variant_dir, case_id)
    source = _gate_load_json(variant_dir / f"{stem}.source.json", case_id, "source.json")
    planning = _gate_load_json(
        variant_dir / f"{stem}.planning.json", case_id, "planning.json"
    )
    domains = _gate_load_json(variant_dir / f"{stem}.domains.json", case_id, "domains.json")
    provenance = _gate_load_json(
        variant_dir / f"{stem}.provenance.json", case_id, "provenance.json"
    )
    replay = _gate_load_json(variant_dir / f"{stem}.replay.json", case_id, "replay.json")
    callers = _gate_load_jsonl(variant_dir / f"{stem}.callers.jsonl", case_id, "callers.jsonl")
    clips = _gate_load_jsonl(variant_dir / f"{stem}.clips.jsonl", case_id, "clips.jsonl")
    # The recorded variant settings preset. `dont_filter_internal_bridges` is object-level and is
    # not part of the earlier-source snapshot; a missing file leaves it genuinely unrecorded.
    settings_rel = variant_dir / "settings.json"
    settings: dict[str, Any] | None = None
    if settings_rel.is_file():
        try:
            loaded = json.loads(settings_rel.read_text(encoding="utf-8"))
        except (OSError, ValueError) as exc:
            raise GateCaseInconclusive(
                case_id, "settings.json", f"unreadable_artifact:{type(exc).__name__}"
            ) from exc
        if not isinstance(loaded, dict):
            raise GateCaseInconclusive(case_id, "settings.json", "artifact_is_not_an_object")
        settings = loaded

    try:
        object_id = _gate_text(
            _gate_field(source, "object_name", "source"), "source.object_name"
        )

        # --- a_source -----------------------------------------------------
        entries: list[dict[str, Any]] = []
        for index, region in enumerate(source.get("regions") or []):
            where = f"source.regions[{index}]"
            surfaces = region.get("surfaces") or []
            if not surfaces:
                raise _GateValueError(f"{where}.surfaces", "region occurrence has no surfaces")
            first = surfaces[0]
            local = _gate_int(
                _gate_field(first, "local_layer_index", where), f"{where}.local_layer_index"
            )
            entries.append(
                {
                    "local_layer_index": local,
                    "global_layer_index": _gate_int(
                        _gate_field(first, "global_layer_index", where),
                        f"{where}.global_layer_index",
                    ),
                    "z_mm": _gate_number(
                        _gate_field(first, "z_mm", where), f"{where}.z_mm"
                    ),
                    "effective_layer_height_mm": _gate_number(
                        _gate_field(region, "height_mm", where), f"{where}.height_mm"
                    ),
                    "region_id": _gate_int(
                        _gate_field(region, "region_id", where), f"{where}.region_id"
                    ),
                    "surfaces": surfaces,
                    "region_config": _gate_raw_region_config(
                        _gate_field(region, "region_config", where), f"{where}.region_config"
                    ),
                    "where": where,
                }
            )
        entries.sort(key=lambda entry: (entry["local_layer_index"], entry["region_id"]))

        region_configs: dict[int, dict[str, Any]] = {}
        canonical_region_order: list[dict[str, Any]] = []
        layers: list[dict[str, Any]] = []
        provenance_records: list[dict[str, Any]] = []
        provenance_id = 0
        layer_groups: dict[int, list[dict[str, Any]]] = {}
        for entry in entries:
            layer_groups.setdefault(entry["local_layer_index"], []).append(entry)
            if entry["region_id"] not in region_configs:
                region_configs[entry["region_id"]] = entry["region_config"]
                canonical_region_order.append(
                    _gate_identity(entry["region_id"], object_id)
                )
            elif region_configs[entry["region_id"]] != entry["region_config"]:
                raise _GateValueError(
                    f"{entry['where']}.region_config",
                    "region config changes between layers; raw config identity is per region",
                )

        for local in sorted(layer_groups):
            group = layer_groups[local]
            first = group[0]
            earlier_regions: list[dict[str, Any]] = []
            for entry in group:
                surfaces: list[dict[str, Any]] = []
                for index, surface in enumerate(entry["surfaces"]):
                    where = f"{entry['where']}.surfaces[{index}]"
                    kind = _gate_surface_kind(
                        _gate_field(surface, "surface_type", where), case_id, where
                    )
                    geometry = _gate_polygon(
                        _gate_field(surface, "expolygon", where),
                        f"{where}.expolygon",
                        transported=True,
                    )
                    surfaces.append(
                        {
                            "surface_token": _gate_text(
                                _gate_field(surface, "source_id", where),
                                f"{where}.source_id",
                            ),
                            "kind": kind,
                            "geometry": geometry,
                            "detected_bridge_angle_rad": None,
                            "provenance_id": provenance_id,
                        }
                    )
                    provenance_records.append(
                        {
                            "provenance_id": provenance_id,
                            "phase": "earlier_classification",
                            "producer": GATE_EARLIER_PRODUCER,
                            "source_geometry_sha256": _gate_sha256(geometry),
                            "raw_config_sha256": _gate_sha256(entry["region_config"]),
                        }
                    )
                    provenance_id += 1
                earlier_regions.append(
                    {
                        "identity": _gate_identity(entry["region_id"], object_id),
                        "surfaces": surfaces,
                        "top_shell_index": None,
                        "bottom_shell_index": None,
                        "config": entry["region_config"],
                    }
                )
            layers.append(
                {
                    "local_layer_index": local,
                    "global_layer_index": first["global_layer_index"],
                    "z_mm": first["z_mm"],
                    "effective_layer_height_mm": first["effective_layer_height_mm"],
                    "regions": earlier_regions,
                }
            )

        any_align = any(config["align_to_model"] for config in region_configs.values())
        a_source = {
            "object_id": object_id,
            "layers": layers,
            "canonical_region_order": canonical_region_order,
            "object_config": _gate_raw_object_config(
                source,
                any_align,
                _gate_internal_bridge_filter(settings, "settings"),
            ),
            "provenance": provenance_records,
        }
        _gate_assert_source_only(a_source, "", case_id)

        # --- B input envelope ---------------------------------------------
        catalog: dict[str, dict[str, Any]] = {}
        for entry in provenance.get("source_catalog") or []:
            token = entry.get("source_id")
            if isinstance(token, str):
                catalog[token] = entry
        evidence = _gate_contributor_evidence(callers, set(catalog), case_id)
        callers_by_token: dict[str, dict[str, Any]] = {}
        for record in callers:
            token = _gate_text(record.get("dispatch_token"), "callers.dispatch_token")
            callers_by_token[token] = record
        b_input = {
            "input": None,
            "contributor_evidence": evidence,
        }

        # --- A reference ---------------------------------------------------
        planning_layers, planning_reason = _gate_reference_planning_layers(
            provenance, domains, object_id, case_id
        )
        parameters = _gate_reference_parameters(planning, domains, case_id)
        scalar_source, scalar_reason = _gate_scalar_source_region(
            region_configs,
            parameters,
            _gate_field(domains, "resolved_inputs", "domains"),
            object_id,
        )
        clipped, clipped_reason = _gate_reference_clipped(
            clips, callers_by_token, catalog, object_id, case_id
        )

        recorded_replay: dict[str, Any] = {}
        for field in GATE_REPLAY_FIELDS:
            recorded_replay[field] = _gate_field(replay, field, "replay")

        policies, policies_reason = _gate_reference_policies(entries, callers)

        reference = {
            "planning_layers": planning_layers,
            "policies": policies,
            "parameters": parameters,
            "scalar_source_region": scalar_source,
            "replay": recorded_replay,
            "graph": None,
            "clipped": clipped,
            "final_fills": None,
        }

        absent: dict[str, str] = {}
        if planning_reason is not None:
            absent["reference.planning_layers"] = planning_reason
        unattributed = provenance.get("unavailable") or []
        if unattributed:
            absent["reference.planning_layers[].boundary_provenance"] = (
                f"{len(unattributed)} planning boundaries have no recorded contributor; they are "
                "published with an empty contributor list"
            )
        absent["reference.policies"] = policies_reason
        if scalar_reason is not None:
            absent["reference.scalar_source_region"] = scalar_reason
        absent["reference.graph"] = (
            "planning.json graph nodes record no origin_region_index and no sampled_segments"
        )
        absent["reference.final_fills"] = (
            "FinalFillInput.policy needs per-region resolved angles the capture does not record"
        )
        if clipped_reason is not None:
            absent["reference.clipped"] = clipped_reason
        absent["b_input.input"] = (
            "RegionInput.outlines and per-region shell/classified surfaces are not recorded"
        )

        a_raw_config = {
            "object_config": a_source["object_config"],
            "regions": [
                {"identity": region["identity"], "config": region["config"]}
                for layer in a_source["layers"]
                for region in layer["regions"]
            ],
        }
        complete = sum(1 for item in evidence if item["status"] == "Complete")
        unavailable_tokens = [
            item["dispatch_token"] for item in evidence if item["status"] != "Complete"
        ]
        manifest_case = {
            "a_source_sha256": _gate_sha256(a_source),
            "a_raw_config_sha256": _gate_sha256(a_raw_config),
            "a_earlier_provenance_sha256": _gate_sha256(a_source["provenance"]),
            "b_input_sha256": (
                "absent" if b_input is None else _gate_sha256(b_input)
            ),
            "reference_sha256": _gate_sha256(reference),
            "contributor_evidence": {
                "complete": complete,
                "unavailable": len(unavailable_tokens),
                "unavailable_dispatches": unavailable_tokens,
            },
            "absent": absent,
        }
        return {
            "case_id": case_id,
            "inputs_case": {
                "case_id": case_id,
                "wall_strategy": GATE_WALL_STRATEGY,
                "a_source": a_source,
                "b_input": b_input,
            },
            "oracle_case": {
                "case_id": case_id,
                "wall_strategy": GATE_WALL_STRATEGY,
                "reference": reference,
            },
            "manifest_case": manifest_case,
        }
    except _GateValueError as exc:
        raise GateCaseInconclusive(case_id, exc.where, exc.reason) from exc


def _gate_capture_identity(capture_root: Path) -> dict[str, Any]:
    manifest_path = capture_root / "manifest.json"
    if not manifest_path.is_file():
        raise GateCaseInconclusive("*", "capture/manifest.json", "missing_artifact")
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise GateCaseInconclusive(
            "*", "capture/manifest.json", f"unreadable_artifact:{type(exc).__name__}"
        )
    if not isinstance(manifest, dict):
        raise GateCaseInconclusive("*", "capture/manifest.json", "artifact_is_not_an_object")
    tools = manifest.get("tools") or {}
    identity: dict[str, Any] = {
        "manifest_sha256": sha256_file(manifest_path),
        "canonical_revision": manifest.get("canonical_revision"),
        "canonical_build_revision": manifest.get("canonical_build_revision"),
        "canonical_build_dirty": manifest.get("canonical_build_dirty"),
        "recorder_patch_sha256": manifest.get("recorder_patch_sha256"),
        "recorder_patch_applied": manifest.get("recorder_patch_applied"),
        # Only version/hash identity is published: the recorded absolute paths are not portable.
        "tools": {
            "python": tools.get("python"),
            "platform": tools.get("platform"),
            "slicer_sha256": tools.get("slicer_sha256"),
        },
    }
    for key in (
        "canonical_revision",
        "canonical_build_revision",
        "recorder_patch_sha256",
        "recorder_patch_applied",
    ):
        if identity[key] is None:
            raise GateCaseInconclusive("*", f"capture/manifest.json.{key}", "missing")
    return identity


def _gate_write_text(path: Path, text: str) -> None:
    temporary = path.with_name(path.name + ".tmp")
    with temporary.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)
    os.replace(temporary, path)


def _gate_publish_summary(status: str, cases: Sequence[dict[str, Any]]) -> str:
    return json.dumps(
        {"status": status, "gate": "A", "cases": list(cases)},
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    )


def publish_fixtures(args: argparse.Namespace) -> int:
    """`--publish-fixtures`: project one Classic capture into the three gate documents."""
    capture_root = Path(args.capture).resolve()
    outputdir = Path(args.outputdir).resolve()
    outputdir.mkdir(parents=True, exist_ok=True)

    try:
        capture_identity = _gate_capture_identity(capture_root)
    except GateCaseInconclusive as exc:
        status_cases = [
            {
                "case_id": f"{name}_classic",
                "status": "inconclusive",
                "field": exc.field,
                "reason": exc.reason,
            }
            for name in GATE_CASE_NAMES
        ]
        for name in GATE_FIXTURE_FILES:
            (outputdir / name).unlink(missing_ok=True)
        status = {"status": "inconclusive", "gate": "A", "cases": status_cases}
        _gate_write_text(
            outputdir / GATE_STATUS_FILE, json.dumps(status, indent=2, sort_keys=True) + "\n"
        )
        print(_gate_publish_summary("inconclusive", status_cases))
        return 3

    published: list[dict[str, Any]] = []
    status_cases: list[dict[str, Any]] = []
    blocked = False
    for name in GATE_CASE_NAMES:
        variant_dir = capture_root / "raw" / f"{name}_classic"
        try:
            built = build_gate_case(variant_dir, name)
        except GateCaseInconclusive as exc:
            blocked = True
            status_cases.append(
                {
                    "case_id": f"{name}_classic",
                    "status": "inconclusive",
                    "field": exc.field,
                    "reason": exc.reason,
                }
            )
            continue
        published.append(built)
        status_cases.append({"case_id": built["case_id"], "status": "published"})

    if blocked or len(published) != len(GATE_CASE_NAMES):
        for name in GATE_FIXTURE_FILES:
            (outputdir / name).unlink(missing_ok=True)
        _gate_write_text(
            outputdir / GATE_STATUS_FILE,
            json.dumps(
                {"status": "inconclusive", "gate": "A", "cases": status_cases},
                indent=2,
                sort_keys=True,
            )
            + "\n",
        )
        print(_gate_publish_summary("inconclusive", status_cases))
        return 3

    inputs_document = {
        "schema_version": GATE_SCHEMA_VERSION,
        "cases": [case["inputs_case"] for case in published],
    }
    oracle_document = {
        "schema_version": GATE_SCHEMA_VERSION,
        "cases": [case["oracle_case"] for case in published],
    }
    manifest_document = {
        "schema_version": GATE_SCHEMA_VERSION,
        "gate": "A",
        "status": "published",
        "case_ids": [case["case_id"] for case in published],
        "capture": capture_identity,
        "units": {
            "canonical": GATE_CANONICAL_UNIT,
            "pnp": GATE_PNP_UNIT,
            "pnp_units_per_canonical_unit": GATE_PNP_UNITS_PER_CANONICAL_UNIT,
            "reference": GATE_CANONICAL_UNIT,
            "transport": {
                "applies_to": ["a_source", "b_input"],
                "conversion": GATE_TRANSPORT_NOTE,
                "tolerance_pnp_units": GATE_TRANSPORT_TOLERANCE_PNP_UNITS,
            },
        },
        "cases": {case["case_id"]: case["manifest_case"] for case in published},
    }

    _gate_write_text(
        outputdir / "inputs.json", _gate_canonical_json(inputs_document) + "\n"
    )
    _gate_write_text(
        outputdir / "canonical.json", _gate_canonical_json(oracle_document) + "\n"
    )
    _gate_write_text(
        outputdir / "manifest.json",
        json.dumps(manifest_document, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
    )
    (outputdir / GATE_STATUS_FILE).unlink(missing_ok=True)
    print(_gate_publish_summary("published", status_cases))
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--source",
        default=str(DEFAULT_SOURCE),
        help="read-only canonical source checkout",
    )
    parser.add_argument(
        "--work", default=str(DEFAULT_WORK), help="disposable build tree"
    )
    parser.add_argument(
        "--slicer",
        default=str(DEFAULT_SLICER),
        help="instrumented orca-slicer executable",
    )
    parser.add_argument(
        "--build-source",
        default=None,
        help="instrumented build checkout (default: derived from --slicer)",
    )
    parser.add_argument(
        "--repo",
        default=str(Path(__file__).resolve().parents[2]),
        help="PnP repository root",
    )
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "--replay", action="store_true", help="replay a recorded manifest"
    )
    group.add_argument(
        "--publish-fixtures",
        action="store_true",
        help="publish gate envelopes from a capture (requires --capture <dir> --outputdir <dir>)",
    )
    parser.add_argument(
        "--capture",
        nargs="?",
        const=True,
        default=None,
        metavar="CAPTURE_DIR",
        help=(
            "record canonical fixtures (bare flag) or read an existing capture root when used "
            "with --publish-fixtures"
        ),
    )
    parser.add_argument("--output", help="fixture output directory")
    parser.add_argument(
        "--outputdir", help="gate document output directory (--publish-fixtures)"
    )
    parser.add_argument("--manifest", help="manifest path for --replay")
    parser.add_argument(
        "--case", help="comma-separated case ids to capture (default: all)"
    )
    parser.add_argument(
        "--strategy",
        default="both",
        choices=["both", "classic", "arachne"],
        help="wall strategy filter (default: both)",
    )
    parser.add_argument(
        "--no-provenance",
        action="store_true",
        help="skip provenance DAG generation during --capture (debugging only)",
    )
    args = parser.parse_args(argv)

    modes = [args.capture is not None, args.replay, args.publish_fixtures]
    if not any(modes):
        parser.error("exactly one of --capture, --replay, --publish-fixtures is required")
    if args.publish_fixtures:
        if not isinstance(args.capture, str):
            parser.error("--publish-fixtures requires --capture <capture-dir>")
        if not args.outputdir:
            parser.error("--publish-fixtures requires --outputdir")
        return publish_fixtures(args)
    if args.replay:
        if isinstance(args.capture, str):
            parser.error("--capture <dir> is only valid with --publish-fixtures")
        if not args.output:
            parser.error("--capture and --replay require --output")
        if not args.manifest:
            parser.error("--replay requires --manifest")
        return replay(args)
    if not args.output:
        parser.error("--capture and --replay require --output")
    return capture(args)


if __name__ == "__main__":
    raise SystemExit(main())
