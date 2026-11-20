//! Independent driver model for `H-PAINT-DRIVER`.
//!
//! `execute_paint_segmentation`
//! (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) rebuilds the
//! committed `SliceIR` from the map's chains. The model here never asks the
//! driver for an expected partition: each object owns a disjoint axis-aligned
//! square, every painted facet covers it whole, and the declared axes say which
//! chains may exist. The checker then proves three things the driver has
//! historically got wrong:
//!
//! 1. **Source isolation** — no output polygon of one object may leave that
//!    object's own square, so a BASE region can never carry another object's
//!    contours.
//! 2. **Axis ownership** — an object's chain set is the cross-product of *its*
//!    declared axes only; a neighbour's axes are never borrowed.
//! 3. **Documented identity** — the empty chain keeps the base region id, and
//!    the material `ToolIndex(0)` fast path lands on payload 1. Other painted
//!    ids are only checked for namespace membership, never for uniqueness.

use std::collections::BTreeSet;
use std::sync::Arc;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_core::algos::paint_segmentation::{
    execute_paint_segmentation, PaintSegmentationError, PAINT_VARIANT_REGION_ID_STRIDE,
};
use slicer_ir::{
    ExPolygon, FacetPaintData, IndexedTriangleSet, MeshIR, ObjectMesh, PaintLayer, PaintSemantic,
    PaintValue, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan, SemVer, SliceIR,
    SlicedRegion, Transform3d,
};
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

/// One declared axis of the driver fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverAxis {
    /// Namespace spelling.
    pub name: &'static str,
    /// Declared priority.
    pub priority: u32,
    /// The single value every painted facet of this axis carries.
    pub value: DriverValue,
}

/// One legal value for a driver axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverValue {
    /// `ToolIndex(0)`.
    ToolZero,
    /// `Flag(true)`.
    FlagTrue,
}

impl DriverValue {
    fn to_paint_value(self) -> PaintValue {
        match self {
            DriverValue::ToolZero => PaintValue::ToolIndex(0),
            DriverValue::FlagTrue => PaintValue::Flag(true),
        }
    }
}

/// The two declared axes: core priorities 100 and 200.
pub const DRIVER_AXES: [DriverAxis; 2] = [
    DriverAxis {
        name: "material",
        priority: 100,
        value: DriverValue::ToolZero,
    },
    DriverAxis {
        name: "fuzzy_skin",
        priority: 200,
        value: DriverValue::FlagTrue,
    },
];

/// One generated scene object.
#[derive(Debug, Clone)]
pub struct DriverObject {
    /// Object id.
    pub id: String,
    /// Lower-left X of its 2 mm square, in canonical units.
    pub x0: i64,
    /// Indices into [`DRIVER_AXES`] this object paints.
    pub axes: Vec<usize>,
}

/// One generated scene: disjoint objects plus a paint-layer order.
#[derive(Debug, Clone)]
pub struct DriverCase {
    /// Objects, at least one, with pairwise disjoint squares.
    pub objects: Vec<DriverObject>,
    /// Order in which the mesh declares its paint layers (a permutation of the
    /// distinct axes in use).
    pub paint_order: Vec<usize>,
}

const SQUARE_MM: i64 = 20_000; // 2 mm in canonical units

impl DriverObject {
    fn square(&self) -> (i64, i64) {
        (self.x0, self.x0 + SQUARE_MM)
    }

    fn square_polygon(&self) -> ExPolygon {
        let (x0, x1) = self.square();
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: x0, y: 0 },
                    Point2 { x: x1, y: 0 },
                    Point2 {
                        x: x1,
                        y: SQUARE_MM,
                    },
                    Point2 {
                        x: x0,
                        y: SQUARE_MM,
                    },
                ],
            },
            holes: Vec::new(),
        }
    }

    /// The ordered chains this object's map entries declare.
    fn chains(&self, order: &[&'static str]) -> Vec<Vec<(String, PaintValue)>> {
        let mut chains = vec![Vec::new()];
        for name in order {
            let Some(axis_index) = self
                .axes
                .iter()
                .copied()
                .find(|index| DRIVER_AXES[*index].name == *name)
            else {
                continue;
            };
            let mut extended = Vec::new();
            for chain in &chains {
                extended.push(chain.clone());
                let mut with = chain.clone();
                with.push((
                    (*name).to_owned(),
                    DRIVER_AXES[axis_index].value.to_paint_value(),
                ));
                extended.push(with);
            }
            chains = extended;
        }
        chains
    }
}

impl DriverCase {
    /// Canonical axis order across the whole scene.
    #[must_use]
    pub fn axis_order(&self) -> Vec<&'static str> {
        let mut pairs: Vec<(u32, &'static str)> = DRIVER_AXES
            .iter()
            .map(|axis| (axis.priority, axis.name))
            .collect();
        pairs.sort();
        pairs.into_iter().map(|(_, name)| name).collect()
    }
}

fn unit_matrix() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// An open square tube spanning `z = 0..1` at the object's square.
fn tube(x0: i64) -> IndexedTriangleSet {
    let mm = |units: i64| units as f32 / 10_000.0;
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let (left, right) = (mm(x0), mm(x0 + SQUARE_MM));
    IndexedTriangleSet {
        vertices: vec![
            v(left, 0.0, 0.0),
            v(right, 0.0, 0.0),
            v(right, 2.0, 0.0),
            v(left, 2.0, 0.0),
            v(left, 0.0, 1.0),
            v(right, 0.0, 1.0),
            v(right, 2.0, 1.0),
            v(left, 2.0, 1.0),
        ],
        indices: vec![
            0, 1, 5, 0, 5, 4, // front
            1, 2, 6, 1, 6, 5, // right
            2, 3, 7, 2, 7, 6, // back
            3, 0, 4, 3, 4, 7, // left
        ],
    }
}

/// Build the mesh, the pre-paint slice and the committed map for a case.
pub fn fixture(case: &DriverCase) -> (MeshIR, Vec<SliceIR>, RegionMapIR) {
    let facets = 8usize;
    let objects = case
        .objects
        .iter()
        .map(|object| {
            let mut layers: Vec<PaintLayer> = case
                .paint_order
                .iter()
                .filter(|axis_index| object.axes.contains(axis_index))
                .map(|axis_index| {
                    let axis = DRIVER_AXES[*axis_index];
                    PaintLayer {
                        semantic: paint_semantic(axis.name),
                        facet_values: vec![Some(axis.value.to_paint_value()); facets],
                        strokes: Vec::new(),
                    }
                })
                .collect();
            // An object with no declared axes carries no paint at all.
            if object.axes.is_empty() {
                layers.clear();
            }
            ObjectMesh {
                id: object.id.clone(),
                mesh: tube(object.x0),
                transform: unit_matrix(),
                paint_data: (!layers.is_empty()).then_some(FacetPaintData { layers }),
                ..ObjectMesh::default()
            }
        })
        .collect();

    let mesh = MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects,
        ..MeshIR::default()
    };

    let slice = vec![SliceIR {
        global_layer_index: 0,
        z: 0.5,
        regions: case
            .objects
            .iter()
            .map(|object| SlicedRegion {
                object_id: object.id.clone(),
                region_id: 0,
                polygons: vec![object.square_polygon()],
                ..SlicedRegion::default()
            })
            .collect(),
        ..SliceIR::default()
    }];

    let order = case.axis_order();
    let mut region_map = RegionMapIR::default();
    for object in &case.objects {
        let chains = object.chains(&order);
        for chain in chains {
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: 0,
                    object_id: object.id.clone(),
                    // The map keys a parent's chains by the parent's base id.
                    region_id: 0,
                    variant_chain: chain,
                },
                RegionPlan::default(),
            );
        }
    }

    (mesh, slice, region_map)
}

fn paint_semantic(name: &str) -> PaintSemantic {
    match name {
        "material" => PaintSemantic::Material,
        "fuzzy_skin" => PaintSemantic::FuzzySkin,
        other => PaintSemantic::Custom(other.to_owned()),
    }
}

fn chain_key(chain: &[(String, PaintValue)]) -> String {
    format!("{chain:?}")
}

/// Check one driver run against the independent model.
pub fn verify(case: &DriverCase) -> Result<(), String> {
    let (mesh, slice, region_map) = fixture(case);
    let input = Arc::new(slice);
    let produced =
        execute_paint_segmentation(Arc::new(mesh), Arc::clone(&input), Arc::new(region_map))
            .map_err(|error| format!("segmentation failed: {error:?}"))?;

    if produced.len() != input.len() {
        return Err(String::from("layer count changed"));
    }

    // A scene with two or more declared axes selects the declared
    // multi-semantic fan-out; a single declared axis keeps the legacy
    // single-semantic path. Both are checked for isolation, identity, chain
    // ownership and BASE/painted disjointness; only the residual's exact area
    // differs between them, so no strict BASE-emptiness claim is made.
    let order = case.axis_order();
    for (layer_index, layer) in produced.iter().enumerate() {
        for object in &case.objects {
            // The driver emits one region per chain that owns geometry, plus
            // BASE. With full-coverage paint only the object's widest chain
            // owns anything; the proper subsets are skipped rather than emitted
            // empty. The widest chain is the ordered product of the object's
            // OWN axes, so a neighbour's axes can never appear here.
            let widest: Vec<(String, PaintValue)> = {
                let mut chain = Vec::new();
                for name in &order {
                    let Some(axis_index) = object
                        .axes
                        .iter()
                        .copied()
                        .find(|index| DRIVER_AXES[*index].name == *name)
                    else {
                        continue;
                    };
                    chain.push((
                        (*name).to_owned(),
                        DRIVER_AXES[axis_index].value.to_paint_value(),
                    ));
                }
                chain
            };
            let mut expected_chains: BTreeSet<Vec<(String, PaintValue)>> = BTreeSet::new();
            expected_chains.insert(Vec::new());
            expected_chains.insert(widest.clone());
            let regions: Vec<&SlicedRegion> = layer
                .regions
                .iter()
                .filter(|region| region.object_id == object.id)
                .collect();

            let actual_chains: BTreeSet<Vec<(String, PaintValue)>> = regions
                .iter()
                .map(|region| region.variant_chain.clone())
                .collect();
            if actual_chains != expected_chains {
                return Err(format!(
                    "object {} layer {layer_index}: chains {:?} != expected {:?}",
                    object.id,
                    actual_chains
                        .iter()
                        .map(|c| chain_key(c))
                        .collect::<Vec<_>>(),
                    expected_chains
                        .iter()
                        .map(|c| chain_key(c))
                        .collect::<Vec<_>>()
                ));
            }

            let (x0, x1) = object.square();
            for region in &regions {
                // --- Source isolation -------------------------------------
                for polygon in &region.polygons {
                    for point in &polygon.contour.points {
                        if point.x < x0 || point.x > x1 || point.y < 0 || point.y > SQUARE_MM {
                            return Err(format!(
                                "object {} chain {:?} leaves its own square at {point:?}",
                                object.id,
                                chain_key(&region.variant_chain)
                            ));
                        }
                    }
                }

                // --- Documented identity ----------------------------------
                if region.variant_chain.is_empty() {
                    if region.region_id != 0 {
                        return Err(format!(
                            "object {} BASE region id {} != base id 0",
                            object.id, region.region_id
                        ));
                    }
                } else {
                    // Namespace and the documented fast-path payload only: the
                    // synthesized payload of an arbitrary chain is the SUT's own
                    // scheme, so asserting it would be circular.
                    if region.region_id == 0 || region.region_id >= PAINT_VARIANT_REGION_ID_STRIDE {
                        return Err(format!(
                            "object {} painted id {} is outside the painted namespace",
                            object.id, region.region_id
                        ));
                    }
                    // The material fast path is the only literal id claim.
                    if region.variant_chain.len() == 1
                        && region.variant_chain[0].0 == "material"
                        && region.variant_chain[0].1 == PaintValue::ToolIndex(0)
                        && region.region_id != 1
                    {
                        return Err(format!(
                            "material ToolIndex(0) must mint payload 1, got {}",
                            region.region_id
                        ));
                    }
                }
            }

            // --- Full coverage on the widest chain ------------------------
            if !widest.is_empty() {
                let full = regions
                    .iter()
                    .find(|region| region.variant_chain == widest)
                    .ok_or_else(|| format!("object {} misses its widest chain", object.id))?;

                // BASE and the painted chain must never claim the same point.
                // This is the real residual contract; it is checked on an
                // independent point grid rather than by an exact area, so a
                // Clipper sliver cannot make it flaky while "BASE = the whole
                // contour" still fails loudly.
                if let Some(base) = regions
                    .iter()
                    .find(|region| region.variant_chain.is_empty())
                {
                    for ix in 1..5 {
                        for iy in 1..5 {
                            let point = Point2 {
                                x: x0 + SQUARE_MM * ix / 5,
                                y: SQUARE_MM * iy / 5,
                            };
                            let in_base = matches!(
                                point_in_region(point, &base.polygons),
                                Ok(PointLocation::Inside)
                            );
                            let in_full = matches!(
                                point_in_region(point, &full.polygons),
                                Ok(PointLocation::Inside)
                            );
                            if in_base && in_full {
                                return Err(format!(
                                    "object {} BASE overlaps its painted chain at {point:?}",
                                    object.id
                                ));
                            }
                        }
                    }
                }
                let centre = Point2 {
                    x: x0 + SQUARE_MM / 2,
                    y: SQUARE_MM / 2,
                };
                match point_in_region(centre, &full.polygons) {
                    Ok(PointLocation::Inside) => {}
                    other => {
                        return Err(format!(
                            "object {} widest chain does not cover its centre: {other:?}",
                            object.id
                        ))
                    }
                }
            }
        }
    }

    // --- Determinism ------------------------------------------------------
    let (mesh2, slice2, map2) = fixture(case);
    let again = execute_paint_segmentation(Arc::new(mesh2), Arc::new(slice2), Arc::new(map2))
        .map_err(|error| format!("second run failed: {error:?}"))?;
    let canonical = |layers: &[SliceIR]| -> Vec<Vec<(String, String, usize)>> {
        layers
            .iter()
            .map(|layer| {
                let mut rows: Vec<(String, String, usize)> = layer
                    .regions
                    .iter()
                    .map(|region| {
                        (
                            region.object_id.clone(),
                            chain_key(&region.variant_chain),
                            region.polygons.len(),
                        )
                    })
                    .collect();
                rows.sort();
                rows
            })
            .collect()
    };
    if canonical(&produced) != canonical(&again) {
        return Err(String::from("repeated runs disagree"));
    }

    Ok(())
}

/// The unchanged-input paths: empty mesh, absent paint, empty map.
pub fn unchanged_paths() -> Result<(), String> {
    let (mesh, slice, region_map) = fixture(&DriverCase {
        objects: vec![DriverObject {
            id: "obj_a".to_owned(),
            x0: 20_000,
            axes: vec![0],
        }],
        paint_order: vec![0],
    });
    let slice = Arc::new(slice);

    // Empty mesh.
    let empty_mesh = MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: Vec::new(),
        ..MeshIR::default()
    };
    let out = execute_paint_segmentation(
        Arc::new(empty_mesh),
        Arc::clone(&slice),
        Arc::new(region_map.clone()),
    )
    .map_err(|error| format!("empty mesh failed: {error:?}"))?;
    if !Arc::ptr_eq(&out, &slice) {
        return Err(String::from(
            "an empty mesh must return the input unchanged",
        ));
    }

    // Absent paint.
    let mut unpainted = mesh.clone();
    for object in &mut unpainted.objects {
        object.paint_data = None;
    }
    let out = execute_paint_segmentation(
        Arc::new(unpainted),
        Arc::clone(&slice),
        Arc::new(region_map.clone()),
    )
    .map_err(|error| format!("absent paint failed: {error:?}"))?;
    if !Arc::ptr_eq(&out, &slice) {
        return Err(String::from("absent paint must return the input unchanged"));
    }

    // Empty map.
    let out = execute_paint_segmentation(
        Arc::new(mesh),
        Arc::clone(&slice),
        Arc::new(RegionMapIR::default()),
    )
    .map_err(|error| format!("empty map failed: {error:?}"))?;
    if !Arc::ptr_eq(&out, &slice) {
        return Err(String::from("an empty map must return the input unchanged"));
    }
    Ok(())
}

/// A realizable colliding-chain scene: one object, one base region, two distinct
/// `material` chains whose synthesized ids are equal because the payload space is
/// finite (`ToolIndex(0)` and `ToolIndex(999_999)` both mint payload 1).
pub fn colliding_chain_error() -> Result<(), String> {
    let case = DriverCase {
        objects: vec![DriverObject {
            id: "obj_a".to_owned(),
            x0: 20_000,
            axes: Vec::new(),
        }],
        paint_order: Vec::new(),
    };
    let (mut mesh, slice, mut region_map) = fixture(&case);
    let object = mesh
        .objects
        .first_mut()
        .ok_or_else(|| String::from("fixture object missing"))?;
    let facets = 8usize;
    let mut values: Vec<Option<PaintValue>> = Vec::with_capacity(facets);
    for facet in 0..facets {
        values.push(Some(if facet < 4 {
            PaintValue::ToolIndex(0)
        } else {
            PaintValue::ToolIndex(999_999)
        }));
    }
    object.paint_data = Some(FacetPaintData {
        layers: vec![PaintLayer {
            semantic: PaintSemantic::Material,
            facet_values: values,
            strokes: Vec::new(),
        }],
    });
    region_map.entries.clear();
    for value in [0u32, 999_999] {
        let chain = vec![("material".to_owned(), PaintValue::ToolIndex(value))];
        region_map.entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: "obj_a".to_owned(),
                region_id: 0,
                variant_chain: chain,
            },
            RegionPlan::default(),
        );
    }

    let error = execute_paint_segmentation(Arc::new(mesh), Arc::new(slice), Arc::new(region_map))
        .expect_err("the colliding chains must be rejected");
    match error {
        PaintSegmentationError::PaintVariantRegionIdCollision {
            object_id,
            region_id,
            first_chain,
            second_chain,
        } => {
            if object_id != "obj_a" {
                return Err(format!("unexpected collision object {object_id}"));
            }
            if region_id == 0 {
                return Err(String::from(
                    "a collision cannot be reported at the base id",
                ));
            }
            // The two chains differ only in their material tool value; the
            // finite payload space is what makes the ids equal.
            let expected: BTreeSet<Vec<(String, PaintValue)>> = [0u32, 999_999]
                .into_iter()
                .map(|tool| vec![("material".to_owned(), PaintValue::ToolIndex(tool))])
                .collect();
            let observed: BTreeSet<Vec<(String, PaintValue)>> =
                [first_chain, second_chain].into_iter().collect();
            if observed != expected {
                return Err(format!("unexpected colliding chains {observed:?}"));
            }
            Ok(())
        }
        other => Err(format!(
            "expected PaintVariantRegionIdCollision, got {other:?}"
        )),
    }
}

/// Strategy for one generated scene.
pub fn strategy(max_objects: usize) -> impl Strategy<Value = DriverCase> {
    let object_count = 1..=max_objects.clamp(1, 3);
    object_count.prop_flat_map(move |count| {
        let axis_sets =
            proptest::collection::vec(proptest::sample::subsequence(vec![0usize, 1], 0..=2), count);
        (axis_sets, 0usize..2).prop_map(move |(axis_sets, rotation)| {
            let objects: Vec<DriverObject> = axis_sets
                .into_iter()
                .enumerate()
                .map(|(index, axes)| DriverObject {
                    id: format!("obj_{index}"),
                    // Disjoint 2 mm squares with a 1 mm gap.
                    x0: 20_000 + index as i64 * 30_000,
                    axes,
                })
                .collect();
            // Paint-layer order: a rotation of the declared axes in use.
            let mut in_use: Vec<usize> = objects
                .iter()
                .flat_map(|object| object.axes.iter().copied())
                .collect();
            in_use.sort_unstable();
            in_use.dedup();
            if !in_use.is_empty() {
                let shift = rotation % in_use.len();
                in_use.rotate_left(shift);
            }
            DriverCase {
                objects,
                paint_order: in_use,
            }
        })
    })
}

/// Registry entry point for `H-PAINT-DRIVER`.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new("H-PAINT-DRIVER", "proptest_paint_driver_source_isolation")
        .map_err(|error| error.to_string())?;
    let max_objects = usize::try_from(run.bound("max_objects").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_objects does not fit a usize"))?;
    let max_layers = run.bound("max_layers").map_err(|e| e.to_string())?;
    let max_regions = run.bound("max_regions").map_err(|e| e.to_string())?;
    let max_abs_units = run.bound("max_abs_units").map_err(|e| e.to_string())?;
    let max_facets = run.bound("max_facets").map_err(|e| e.to_string())?;
    if max_layers < 1 || max_regions < 1 || max_abs_units < 200_000 || max_facets < 8 {
        return Err(String::from(
            "the driver domain needs max_layers >= 1, max_regions >= 1, \
             max_abs_units >= 200000 and max_facets >= 8",
        ));
    }

    unchanged_paths()?;
    colliding_chain_error()?;

    let generated = strategy(max_objects);
    run.check(&generated, |case: DriverCase, _guard: &mut CaseGuard| {
        verify(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}
