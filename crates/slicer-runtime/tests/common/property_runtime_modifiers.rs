//! Charter `H-RUNTIME-MODIFIERS`: first-owner modifier ownership and the
//! transactional rejection boundary of the real split kernel.
//!
//! The property drives `region_partition::split_regions_by_modifier_footprints`
//! (the private kernel both public seams call) with ordered modifier footprints
//! and compares the observed `SliceIR` against an independent finite-cell model.
//! The model is deliberately not a wrapper:
//!
//! * a cell belongs to the **first** footprint in supplied order that covers it;
//!   later footprints receive only the parent's remaining cells;
//! * every geometry carrier (`polygons`, `infill_areas`, `bridge_areas`,
//!   `bottom_solid_fill`, `top_solid_fill`, `sparse_infill_area`,
//!   `internal_solid_fill`, internal-bridge polygons and `raft_fill`) is modelled
//!   and conserved per parent;
//! * an empty input `infill_areas` is modelled as the documented restore from
//!   `polygons`, including the case where no footprint ever intersects the
//!   parent and the field therefore stays empty;
//! * internal-bridge polygons are grouped by their declared angle so a union
//!   that loses per-polygon angle association is a visible disagreement;
//! * metadata (object, parent id, ordered chain, cleared annotations, unrelated
//!   fields) is asserted separately from geometry.
//!
//! The deterministic controls exercise the private kernel, the raw staging
//! wrapper, the public prepass seam and the `LayerArena` hook. The hook control
//! (and the named non-RNG regression at the end) pins the entry-state rollback of
//! `sync_perimeter_infill_areas_into_slice`: a rejected modifier split must not
//! leave the arena without the slice it took.
//!
//! The generated population is mode-tagged on a `no_shrink` draw. `Valid`
//! scenes run the first-owner model above, which now also carries a holed
//! parent, an inert empty staging footprint, a guaranteed first/later footprint
//! overlap on a parent cell, and overlapping bridge/bottom/top carriers.
//! `Collision`, `UnsupportedParent` and `LaterFailure` scenes are *constructed*,
//! never sampled until they happen: each places the exact geometry that makes
//! the kernel fail, and the checker proves the expected error plus byte-for-byte
//! restoration of the input `SliceIR` instead of running the ownership model.
//! The mode tag itself is read back from the constructed scene, so shrinking
//! can only keep it if the witness survived.

#![allow(missing_docs)]

use std::collections::{BTreeSet, HashMap};

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_ir::{
    is_modifier_namespace_id, modifier_base_region_id, modifier_sub_region_id,
    modifier_sub_region_id_fits, ExPolygon, IndexedTriangleSet, LayerStageError, MeshIR,
    ModifierKind, ModifierVolume, ObjectMesh, PaintSemantic, PaintValue, PerimeterIR,
    PerimeterRegion, Point2, Point3, QuartileBand, SliceIR, SlicedRegion, SurfaceClassificationIR,
    SurfaceGroupId, CURRENT_SLICE_IR_SCHEMA_VERSION, MODIFIER_FOOTPRINT_REGION_ID,
    MODIFIER_VARIANT_REGION_ID_STRIDE,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::cells::{
    assert_disjoint_union, assert_orthogonal_on_grid, checked_max_abs_units, checked_max_cells,
    compare_region_to_cells, holed_rect, CellGrid, CellSet, Rect,
};
use crate::region_partition::{
    split_modifier_footprints, split_modifier_sub_regions_for_prepass,
    split_regions_by_modifier_footprints, sync_perimeter_infill_areas_into_slice,
};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-RUNTIME-MODIFIERS";
/// Exact registry test name (the inline libtest identity).
pub const TEST_NAME: &str =
    "region_partition::property_tests::proptest_runtime_modifier_transactions";

/// Written ceiling for generated parent regions (domain design, not a measured
/// runtime claim).
const MAX_REGIONS_CEILING: u64 = 4;
/// Written ceiling for ordered footprints.
const MAX_FOOTPRINTS_CEILING: u64 = 4;

/// Shared lattice resolution. Every generated coordinate is a multiple of an
/// even step on this lattice, so all inputs and Boolean outputs share at most
/// `(LATTICE + 1)^2` elementary cells regardless of how many carriers a case
/// carries.
const LATTICE: usize = 6;

const OBJECT_A: &str = "modifier-property-object-a";
const OBJECT_B: &str = "modifier-property-object-b";

/// Nondefault `SurfaceGroupId` seeded on every generated parent. The split must
/// copy it verbatim into the parent's residue and into every minted child (the
/// kernel does not partition this field), so a dropped or reset group id is a
/// visible metadata disagreement instead of an untested default.
const NONPLANAR_GROUP_BASE: SurfaceGroupId = 4096;

/// Checked generation bounds for `H-RUNTIME-MODIFIERS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifierLimits {
    /// Maximum generated parent regions.
    pub max_regions: usize,
    /// Maximum ordered footprints.
    pub max_footprints: usize,
    /// Absolute coordinate bound in canonical units.
    pub max_abs_units: i64,
    /// Elementary-cell budget for one comparison grid.
    pub max_cells: usize,
}

/// Validate every registry bound against the written safety ceiling before any
/// strategy allocates. The routine/extended profiles carry smaller values, so
/// the checked range is the ceiling and any profile value inside it is accepted
/// unchanged.
pub fn checked_modifier_limits(
    max_regions: u64,
    max_footprints: u64,
    max_abs_units: u64,
    max_cells: u64,
) -> Result<ModifierLimits, String> {
    check_bound("max_regions", max_regions, 2, MAX_REGIONS_CEILING)?;
    check_bound("max_footprints", max_footprints, 2, MAX_FOOTPRINTS_CEILING)?;
    let max_abs_units = checked_max_abs_units(max_abs_units).map_err(|e| e.to_string())?;
    let max_cells = checked_max_cells(max_cells).map_err(|e| e.to_string())?;
    let max_regions =
        usize::try_from(max_regions).map_err(|_| String::from("max_regions does not fit usize"))?;
    let max_footprints = usize::try_from(max_footprints)
        .map_err(|_| String::from("max_footprints does not fit usize"))?;
    Ok(ModifierLimits {
        max_regions,
        max_footprints,
        max_abs_units,
        max_cells,
    })
}

fn check_bound(name: &str, value: u64, low: u64, high: u64) -> Result<(), String> {
    if (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(format!("{name} must be in {low}..={high}, got {value}"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Generated domain.
// ─────────────────────────────────────────────────────────────────────────────

/// One ordered modifier footprint: the object it targets and its raw staging
/// polygons.
#[derive(Debug, Clone)]
pub struct ModifierFootprint {
    /// Object whose parents this footprint may split.
    pub object_id: String,
    /// Footprint polygons in supplied priority order.
    pub polygons: Vec<ExPolygon>,
}

/// One scene mode the generator tags on a `no_shrink` draw.
///
/// `Valid` is the first-owner ownership domain. The three rejection modes are
/// *constructed*, never sampled until they happen: `build_scene` places the
/// exact geometry that makes the kernel fail, and `check_scene` proves the
/// expected error plus the byte-for-byte restoration of the input `SliceIR`
/// instead of running the ownership model. `witness_tags` reads the mode back
/// out of the constructed scene, so a mode tag can only survive shrinking if
/// its witness did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModifierMode {
    /// The kernel must accept the scene and split it first-owner.
    Valid,
    /// A pre-existing child already holds footprint 0's prospective identity.
    Collision,
    /// The first intersecting parent id cannot encode a modifier child id.
    UnsupportedParent,
    /// An earlier footprint splits one parent, a later one collides on another.
    LaterFailure,
}

impl ModifierMode {
    /// Every mode, in a stable order.
    pub const ALL: [ModifierMode; 4] = [
        ModifierMode::Valid,
        ModifierMode::Collision,
        ModifierMode::UnsupportedParent,
        ModifierMode::LaterFailure,
    ];

    /// Map a `no_shrink` draw onto one mode.
    fn from_bits(bits: u32) -> Self {
        ModifierMode::ALL[bits as usize % ModifierMode::ALL.len()]
    }
}

/// One generated modifier scene.
#[derive(Debug, Clone)]
pub struct ModifierScene {
    /// The mode the scene is constructed to exercise.
    pub mode: ModifierMode,
    /// Parent regions in committed order.
    pub parents: Vec<SlicedRegion>,
    /// Ordered footprints handed to the private kernel.
    pub footprints: Vec<ModifierFootprint>,
    /// Optional already-materialized modifier child that must survive untouched.
    pub preexisting_child: Option<SlicedRegion>,
    /// Whether the scene was constructed with the `no_shrink` role-precedence
    /// bit, which makes the bridge carrier a strict subset of the bottom carrier.
    ///
    /// The tag is derived from the geometry *and* this bit so a chance overlap
    /// between independently drawn carriers is not mistaken for the structural
    /// witness the shrink audit requires to survive.
    pub role_overlap: bool,
}

/// One lattice rectangle in cell coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LatticeRect {
    c0: usize,
    r0: usize,
    c1: usize,
    r1: usize,
}

impl LatticeRect {
    fn rect(self, step: i64) -> Rect {
        Rect::new(
            self.c0 as i64 * step,
            self.r0 as i64 * step,
            self.c1 as i64 * step,
            self.r1 as i64 * step,
        )
        .expect("lattice rectangle is non-degenerate by construction")
    }

    fn expolygon(self, step: i64) -> ExPolygon {
        self.rect(step).to_expolygon()
    }
}

/// Even lattice step whose farthest coordinate stays strictly below the
/// accepted coordinate bound, leaving room for one footprint placed entirely
/// outside every parent.
fn lattice_step(max_abs_units: i64) -> i64 {
    (((max_abs_units - 1) / (2 * LATTICE as i64)) * 2).max(2)
}

/// The holed parent's single component. Five cells leave a strictly interior
/// cell for the hole and a hole-free sub-rectangle for footprint 0, which is
/// placed first so the first-owner partition stays a real witness.
const HOLED_PARENT_PRIMARY: LatticeRect = LatticeRect {
    c0: 0,
    r0: 0,
    c1: 5,
    r1: 5,
};

/// Rectangular hole punched in the holed parent, strictly inside its contour.
const HOLED_PARENT_HOLE: LatticeRect = LatticeRect {
    c0: 3,
    r0: 3,
    c1: 4,
    r1: 4,
};

/// Hole-free area footprint 0 is drawn from in the holed-parent mode. Its cells
/// all have column/row indices below 3, so they never meet the hole.
const HOLED_FIRST_FOOTPRINT_AREA: LatticeRect = LatticeRect {
    c0: 0,
    r0: 0,
    c1: 3,
    r1: 3,
};

fn chain_for(variant: u32) -> Vec<(String, PaintValue)> {
    match variant {
        0 => Vec::new(),
        1 => vec![("material".to_string(), PaintValue::ToolIndex(1))],
        2 => vec![("density".to_string(), PaintValue::Flag(true))],
        _ => vec![("seam".to_string(), PaintValue::Scalar(0.5))],
    }
}

fn paint_annotations() -> HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> {
    let mut annotations = HashMap::new();
    annotations.insert(
        PaintSemantic::Material,
        vec![vec![Some(PaintValue::ToolIndex(3))]],
    );
    annotations.insert(
        PaintSemantic::FuzzySkin,
        vec![vec![Some(PaintValue::Flag(true))]],
    );
    annotations
}

fn parent_components(index: usize, seeds: &[u32], mode: u32) -> Vec<LatticeRect> {
    let base = index * 8;
    let seed = |offset: usize| seeds[(base + offset) % seeds.len()];
    let two_components = (mode >> (16 + index)) & 1 == 1;
    if !two_components {
        // At least two lattice cells wide and tall, so every carrier sub-rect
        // (including a two-piece internal-bridge split) exists.
        let c0 = seed(0) as usize % (LATTICE - 2);
        let c1 = c0 + 2 + seed(1) as usize % (LATTICE - c0 - 1);
        let r0 = seed(2) as usize % (LATTICE - 2);
        let r1 = r0 + 2 + seed(3) as usize % (LATTICE - r0 - 1);
        vec![LatticeRect { c0, r0, c1, r1 }]
    } else {
        // A concave orthogonal union: two side-by-side components separated by
        // an interior lattice cut, so their interiors never overlap.
        let cut = 1 + seed(0) as usize % (LATTICE - 1);
        let r0 = seed(1) as usize % (LATTICE - 2);
        let r1 = r0 + 2 + seed(2) as usize % (LATTICE - r0 - 1);
        let s0 = seed(3) as usize % (LATTICE - 2);
        let s1 = s0 + 2 + seed(4) as usize % (LATTICE - s0 - 1);
        vec![
            LatticeRect {
                c0: 0,
                r0,
                c1: cut,
                r1,
            },
            LatticeRect {
                c0: cut,
                r0: s0,
                c1: LATTICE,
                r1: s1,
            },
        ]
    }
}

/// A non-empty sub-rectangle of `outer`.
fn sub_lattice(outer: LatticeRect, seeds: &[u32]) -> LatticeRect {
    let width = outer.c1 - outer.c0;
    let height = outer.r1 - outer.r0;
    let c0 = outer.c0 + seeds[0] as usize % width;
    let c1 = c0 + 1 + seeds[1] as usize % (outer.c1 - c0);
    let r0 = outer.r0 + seeds[2] as usize % height;
    let r1 = r0 + 1 + seeds[3] as usize % (outer.r1 - r0);
    LatticeRect { c0, r0, c1, r1 }
}

/// A non-empty sub-rectangle of `outer` at least two lattice rows tall.
fn sub_lattice_wide(outer: LatticeRect, seeds: &[u32]) -> LatticeRect {
    let width = outer.c1 - outer.c0;
    let height = outer.r1 - outer.r0;
    debug_assert!(height >= 2, "parent components are at least two rows tall");
    let c0 = outer.c0 + seeds[0] as usize % width;
    let c1 = c0 + 1 + seeds[1] as usize % (outer.c1 - c0);
    let r0 = outer.r0 + seeds[2] as usize % (height - 1);
    let r1 = r0 + 2 + seeds[3] as usize % (outer.r1 - r0 - 1);
    LatticeRect { c0, r0, c1, r1 }
}

/// One or two disjoint internal-bridge pieces with either an aligned, distinct
/// angle vector or the empty fallback vector.
fn bridge_rects(
    primary: LatticeRect,
    seeds: &[u32],
    index: usize,
    mode: u32,
) -> (Vec<LatticeRect>, Vec<f32>) {
    let inner = sub_lattice_wide(primary, seeds);
    let two_pieces = (mode >> (20 + index)) & 1 == 1;
    let rects = if two_pieces {
        let mid = inner.r0 + 1 + seeds[4] as usize % (inner.r1 - inner.r0 - 1);
        vec![
            LatticeRect { r1: mid, ..inner },
            LatticeRect { r0: mid, ..inner },
        ]
    } else {
        vec![inner]
    };
    let aligned = (mode >> (24 + index)) & 1 == 1;
    let angles = if aligned {
        rects
            .iter()
            .enumerate()
            .map(|(position, _)| if position == 0 { 30.0 } else { 120.0 })
            .collect()
    } else {
        Vec::new()
    };
    (rects, angles)
}

fn parent_region(
    index: usize,
    components: &[LatticeRect],
    seeds: &[u32],
    mode: u32,
    step: i64,
    holed: bool,
    role_overlap: bool,
) -> SlicedRegion {
    let base = index * 32;
    let seed = |offset: usize| seeds[(base + offset) % seeds.len()];
    let object_id = if (mode >> index) & 1 == 1 {
        OBJECT_B
    } else {
        OBJECT_A
    };
    let polygons: Vec<ExPolygon> = if holed {
        // A rectangular hole makes the parent's own outline non-convex in the
        // one way the cell model can still certify exactly: every ring edge stays
        // on the shared lattice.
        vec![holed_rect(
            HOLED_PARENT_PRIMARY.rect(step),
            &[HOLED_PARENT_HOLE.rect(step)],
        )
        .expect("the holed parent's hole is strictly inside its contour")]
    } else {
        components.iter().map(|c| c.expolygon(step)).collect()
    };
    let primary = components[0];
    let carrier = |offset: usize| {
        let local = [
            seed(offset),
            seed(offset + 1),
            seed(offset + 2),
            seed(offset + 3),
        ];
        sub_lattice(primary, &local)
    };
    let mut region = SlicedRegion {
        object_id: object_id.to_string(),
        region_id: index as u64,
        polygons: polygons.clone(),
        infill_areas: if (mode >> (12 + index)) & 1 == 1 {
            Vec::new()
        } else {
            polygons
        },
        // A nondefault group id is the metadata-fidelity witness: the kernel
        // partitions geometry only, so both the parent residue and every minted
        // child must still carry this exact value after a split.
        nonplanar_surface: Some(NONPLANAR_GROUP_BASE + index as u64),
        variant_chain: chain_for((mode >> (4 + 2 * index)) & 3),
        effective_layer_height: 0.2,
        top_shell_index: Some(0),
        segment_annotations: paint_annotations(),
        bridge_orientation_deg: 73.0,
        ..SlicedRegion::default()
    };
    let bridge = carrier(0);
    // With the role-precedence bit set the bottom carrier contains every bridge
    // cell, so at least one cell belongs to two original roles and the
    // documented bridge > bottom precedence is genuinely exercised.
    let bottom = if role_overlap {
        LatticeRect {
            r1: (bridge.r1 + 1).min(LATTICE),
            ..bridge
        }
    } else {
        carrier(4)
    };
    region.bridge_areas = vec![bridge.expolygon(step)];
    region.bottom_solid_fill = vec![bottom.expolygon(step)];
    region.top_solid_fill = vec![carrier(8).expolygon(step)];
    region.sparse_infill_area = vec![carrier(12).expolygon(step)];
    region.internal_solid_fill = vec![carrier(16).expolygon(step)];
    region.raft_fill = vec![carrier(20).expolygon(step)];
    let bridge_seeds = [seed(24), seed(25), seed(26), seed(27), seed(28)];
    let (bridge_rects, angles) = bridge_rects(primary, &bridge_seeds, index, mode);
    region.internal_bridge_areas = bridge_rects.iter().map(|r| r.expolygon(step)).collect();
    region.internal_bridge_angles_deg = angles;
    if (mode >> (28 + index)) & 1 == 1 {
        region.effective_layer_height = 0.3;
        region.top_shell_index = Some(1);
        region.bottom_shell_index = Some(0);
        region.is_bridge = true;
    }
    region
}

fn alternate_object(object_id: &str) -> String {
    if object_id == OBJECT_A {
        OBJECT_B.to_string()
    } else {
        OBJECT_A.to_string()
    }
}

/// Build the optional pre-existing modifier child with a full identity that no
/// supplied footprint will mint. Candidate cells are scanned deterministically,
/// so the valid domain never depends on a sampled hash.
fn build_preexisting_child(
    parent: &SlicedRegion,
    footprints: &[ModifierFootprint],
    seeds: &[u32],
    step: i64,
) -> Option<SlicedRegion> {
    let prospective: Vec<u64> = footprints
        .iter()
        .filter(|footprint| footprint.object_id == parent.object_id)
        .map(|footprint| {
            modifier_sub_region_id(parent.region_id, &footprint.object_id, &footprint.polygons)
        })
        .collect();
    let offset = seeds[0] as usize;
    for candidate in 0..(LATTICE * LATTICE) {
        let index = (offset + candidate) % (LATTICE * LATTICE);
        let column = index % LATTICE;
        let row = index / LATTICE;
        let geometry = LatticeRect {
            c0: column,
            r0: row,
            c1: column + 1,
            r1: row + 1,
        }
        .expolygon(step);
        let region_id = modifier_sub_region_id(
            parent.region_id,
            &parent.object_id,
            std::slice::from_ref(&geometry),
        );
        if prospective.contains(&region_id) {
            continue;
        }
        return Some(SlicedRegion {
            object_id: parent.object_id.clone(),
            region_id,
            polygons: vec![geometry],
            variant_chain: parent.variant_chain.clone(),
            effective_layer_height: parent.effective_layer_height,
            ..SlicedRegion::default()
        });
    }
    None
}

/// Total strategy: every field is derived from the raw draws, so no case can be
/// rejected. Mode-defining bits are `no_shrink`, so shrinking never removes the
/// mode, the holed parent, the inert empty footprint, the two precedence
/// overlaps, the empty-infill branch, the chain distinction or the pre-existing
/// child.
///
/// `max_footprints` bounds the geometric footprints. When the empty-footprint
/// bit is set one further staging-only footprint with no polygons is appended:
/// the domain covers "an empty staging footprint must be inert", and appending
/// it (rather than replacing a drawn one) keeps its tag independent of the
/// shrinkable footprint count.
pub fn modifier_strategy(limits: &ModifierLimits) -> BoxedStrategy<ModifierScene> {
    let limits = *limits;
    let seed_count = 8 + limits.max_regions * 32 + limits.max_footprints * 8;
    (
        any::<u32>().no_shrink(),
        any::<u32>().no_shrink(),
        any::<u32>().no_shrink(),
        (1usize..=limits.max_regions),
        (1usize..=limits.max_footprints),
        any::<u32>().no_shrink(),
        proptest::collection::vec(any::<u32>(), seed_count),
    )
        .prop_map(
            move |(
                mode,
                mode_bits,
                layout_bits,
                parent_count,
                footprint_count,
                object_mask,
                seeds,
            )| {
                build_scene(
                    mode,
                    mode_bits,
                    layout_bits,
                    parent_count,
                    footprint_count,
                    object_mask,
                    &seeds,
                    &limits,
                )
            },
        )
        .boxed()
}

fn build_scene(
    mode: u32,
    mode_bits: u32,
    layout_bits: u32,
    parent_count: usize,
    footprint_count: usize,
    object_mask: u32,
    seeds: &[u32],
    limits: &ModifierLimits,
) -> ModifierScene {
    let step = lattice_step(limits.max_abs_units);
    match ModifierMode::from_bits(mode_bits) {
        ModifierMode::Valid => valid_scene(
            mode,
            layout_bits,
            parent_count,
            footprint_count,
            object_mask,
            seeds,
            step,
            limits.max_abs_units,
        ),
        ModifierMode::Collision => collision_scene(step),
        ModifierMode::UnsupportedParent => unsupported_parent_scene(step),
        ModifierMode::LaterFailure => later_failure_scene(step),
    }
}

fn valid_scene(
    mode: u32,
    layout_bits: u32,
    parent_count: usize,
    footprint_count: usize,
    object_mask: u32,
    seeds: &[u32],
    step: i64,
    max_abs_units: i64,
) -> ModifierScene {
    let holed = layout_bits & 1 == 1;
    let empty_footprint = layout_bits & 2 == 2;
    let role_overlap = layout_bits & 4 == 4;
    let mut layouts: Vec<Vec<LatticeRect>> = (0..parent_count)
        .map(|index| parent_components(index, seeds, mode))
        .collect();
    if holed {
        layouts[0] = vec![HOLED_PARENT_PRIMARY];
    }
    let parents: Vec<SlicedRegion> = layouts
        .iter()
        .enumerate()
        .map(|(index, components)| {
            parent_region(
                index,
                components,
                seeds,
                mode,
                step,
                holed && index == 0,
                role_overlap,
            )
        })
        .collect();
    let first_object = parents[0].object_id.clone();
    let primary = layouts[0][0];
    // Footprint 0 is drawn first and its partner is constructed from it, so the
    // first pair always overlaps on a parent cell (the first-owner precedence
    // witness) and always has distinct geometry (no accidental identity
    // collision). In the holed mode both are drawn from the hole-free area, so a
    // footprint can never be swallowed by the parent's hole.
    let footprint_area = if holed {
        HOLED_FIRST_FOOTPRINT_AREA
    } else {
        primary
    };
    let first_seed = 64;
    let first_local = [
        seeds[first_seed % seeds.len()],
        seeds[(first_seed + 1) % seeds.len()],
        seeds[(first_seed + 2) % seeds.len()],
        seeds[(first_seed + 3) % seeds.len()],
    ];
    let first_rect = sub_lattice(footprint_area, &first_local);
    let overlap_rect = later_overlapping_footprint(first_rect, seeds);
    // The overlap witness needs a second geometric footprint, so the drawn count
    // is floored at two; both tags are then independent of the shrinkable count.
    let geometric = footprint_count.max(2);
    let mut footprints = Vec::with_capacity(geometric + usize::from(empty_footprint));
    for index in 0..geometric {
        let (object_id, polygons) = match index {
            0 => (first_object.clone(), vec![first_rect.expolygon(step)]),
            1 => (first_object.clone(), vec![overlap_rect.expolygon(step)]),
            _ => {
                // The remaining drawn variants keep the wider domain: another
                // sub-rectangle of the first parent, an extension past its
                // boundary, an entirely outside footprint, and another object.
                let variant = ((object_mask >> (2 * index)) & 3) as usize;
                let seed_base = 64 + index * 6;
                let local = [
                    seeds[seed_base % seeds.len()],
                    seeds[(seed_base + 1) % seeds.len()],
                    seeds[(seed_base + 2) % seeds.len()],
                    seeds[(seed_base + 3) % seeds.len()],
                ];
                match variant {
                    0 => (
                        first_object.clone(),
                        vec![sub_lattice(primary, &local).expolygon(step)],
                    ),
                    1 => {
                        let inner = sub_lattice(primary, &local);
                        let extended = LatticeRect {
                            c0: inner.c0.saturating_sub(1),
                            c1: (inner.c1 + 1).min(LATTICE),
                            ..inner
                        };
                        (first_object.clone(), vec![extended.expolygon(step)])
                    }
                    2 => {
                        let outside = Rect::new(step * LATTICE as i64, 0, max_abs_units, step)
                            .expect("the lattice leaves a positive outside strip");
                        (first_object.clone(), vec![outside.to_expolygon()])
                    }
                    _ => (
                        alternate_object(&first_object),
                        vec![sub_lattice(primary, &local).expolygon(step)],
                    ),
                }
            }
        };
        footprints.push(ModifierFootprint {
            object_id,
            polygons,
        });
    }
    if empty_footprint {
        footprints.push(ModifierFootprint {
            object_id: first_object.clone(),
            polygons: Vec::new(),
        });
    }
    // The modifier sub-region id hashes the footprint's contour points, and the
    // drawn rectangles can repeat exactly. Two identical (or hash-colliding)
    // nonempty footprints for the same object would be a real identity
    // collision, which is the rejection mode's job, not the valid domain's.
    // Keep the first occurrence and drop later duplicates. The overlap witness
    // (index 1) is never silently dropped: if it collides with the first
    // footprint it is replaced by a fixed whole-lattice rectangle, which still
    // overlaps a first footprint that is inside the lattice.
    let mut seen_ids: BTreeSet<(String, u64)> = BTreeSet::new();
    let mut deduped: Vec<ModifierFootprint> = Vec::with_capacity(footprints.len());
    for (index, footprint) in footprints.into_iter().enumerate() {
        if footprint.polygons.is_empty() {
            deduped.push(footprint);
            continue;
        }
        let mut candidate = footprint;
        let mut key = (
            candidate.object_id.clone(),
            modifier_sub_region_id(0, &candidate.object_id, &candidate.polygons),
        );
        if seen_ids.contains(&key) {
            if index == 1 {
                let fallback = ModifierFootprint {
                    object_id: candidate.object_id.clone(),
                    polygons: vec![LatticeRect {
                        c0: 0,
                        r0: 0,
                        c1: LATTICE,
                        r1: LATTICE,
                    }
                    .expolygon(step)],
                };
                let fallback_key = (
                    fallback.object_id.clone(),
                    modifier_sub_region_id(0, &fallback.object_id, &fallback.polygons),
                );
                if !seen_ids.contains(&fallback_key) {
                    candidate = fallback;
                    key = fallback_key;
                } else {
                    continue;
                }
            } else {
                continue;
            }
        }
        seen_ids.insert(key);
        deduped.push(candidate);
    }
    let footprints = deduped;
    let preexisting_child = if (object_mask >> 8) & 1 == 1 {
        build_preexisting_child(&parents[0], &footprints, seeds, step)
    } else {
        None
    };
    ModifierScene {
        mode: ModifierMode::Valid,
        parents,
        footprints,
        preexisting_child,
        role_overlap,
    }
}

/// A footprint that shares at least one parent cell with `first` while keeping a
/// distinct identity, so first-owner precedence is decided by order rather than
/// by geometry that never meets.
fn later_overlapping_footprint(first: LatticeRect, seeds: &[u32]) -> LatticeRect {
    let expanded = LatticeRect {
        c0: first.c0.saturating_sub(1),
        r0: first.r0.saturating_sub(1),
        c1: (first.c1 + 1).min(LATTICE),
        r1: (first.r1 + 1).min(LATTICE),
    };
    if expanded != first {
        return expanded;
    }
    // `first` already spans the whole lattice, so growing it is a no-op. A strict
    // sub-rectangle still shares cells with it and has distinct geometry.
    let width = first.c1 - first.c0;
    let height = first.r1 - first.r0;
    debug_assert!(
        width >= 2 && height >= 2,
        "only a whole-lattice rectangle reaches the strict sub-rectangle branch"
    );
    LatticeRect {
        c0: first.c0,
        r0: first.r0,
        c1: first.c0 + 1 + seeds[0] as usize % (width - 1),
        r1: first.r0 + 1 + seeds[1] as usize % (height - 1),
    }
}

/// A literal parent for the constructed rejection scenes: fixed identity, fixed
/// (empty) chain, no carriers. Nothing here comes from a shrinkable draw, so a
/// rejection mode's tag set cannot change under `simplify()`.
fn rejection_parent(object_id: &str, region_id: u64, polygons: Vec<ExPolygon>) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id,
        polygons,
        effective_layer_height: 0.2,
        ..SlicedRegion::default()
    }
}

fn rejection_footprint(object_id: &str, rect: LatticeRect, step: i64) -> ModifierFootprint {
    ModifierFootprint {
        object_id: object_id.to_string(),
        polygons: vec![rect.expolygon(step)],
    }
}

/// A parent id that `modifier_sub_region_id_fits` rejects, kept outside the
/// modifier namespace so the kernel still treats it as an ordinary parent.
fn unencodable_parent_id() -> u64 {
    let stride = MODIFIER_VARIANT_REGION_ID_STRIDE;
    let max_parent = ((1_u64 << 63) - 2 - (stride - 1)) / stride;
    let invalid = max_parent + 1;
    assert!(
        !modifier_sub_region_id_fits(invalid),
        "fixture must use an unencodable parent id"
    );
    assert!(
        !is_modifier_namespace_id(invalid),
        "fixture parent id must stay outside the modifier namespace"
    );
    invalid
}

/// `Collision`: one parent plus a pre-existing child that already holds
/// footprint 0's prospective `(object_id, region_id, variant_chain)` identity.
fn collision_scene(step: i64) -> ModifierScene {
    const OBJECT: &str = "modifier-mode-collision";
    let parent = rejection_parent(
        OBJECT,
        0,
        vec![LatticeRect {
            c0: 0,
            r0: 0,
            c1: 3,
            r1: 3,
        }
        .expolygon(step)],
    );
    let footprint = rejection_footprint(
        OBJECT,
        LatticeRect {
            c0: 1,
            r0: 1,
            c1: 2,
            r1: 2,
        },
        step,
    );
    let collide_id = modifier_sub_region_id(0, OBJECT, &footprint.polygons);
    let existing = SlicedRegion {
        object_id: OBJECT.to_string(),
        region_id: collide_id,
        polygons: footprint.polygons.clone(),
        ..SlicedRegion::default()
    };
    ModifierScene {
        mode: ModifierMode::Collision,
        parents: vec![parent],
        footprints: vec![footprint],
        preexisting_child: Some(existing),
        role_overlap: false,
    }
}

/// `UnsupportedParent`: one parent whose id cannot encode any child id, hit by a
/// footprint that really intersects it.
fn unsupported_parent_scene(step: i64) -> ModifierScene {
    const OBJECT: &str = "modifier-mode-parent-id";
    let parent = rejection_parent(
        OBJECT,
        unencodable_parent_id(),
        vec![LatticeRect {
            c0: 0,
            r0: 0,
            c1: 3,
            r1: 3,
        }
        .expolygon(step)],
    );
    let footprint = rejection_footprint(
        OBJECT,
        LatticeRect {
            c0: 1,
            r0: 1,
            c1: 2,
            r1: 2,
        },
        step,
    );
    ModifierScene {
        mode: ModifierMode::UnsupportedParent,
        parents: vec![parent],
        footprints: vec![footprint],
        preexisting_child: None,
        role_overlap: false,
    }
}

/// `LaterFailure`: the first footprint splits parent A; a later footprint then
/// collides with a pre-existing child of parent B, so only the transactional
/// clone can restore the input.
fn later_failure_scene(step: i64) -> ModifierScene {
    const OBJECT: &str = "modifier-mode-later";
    let parents = vec![
        rejection_parent(
            OBJECT,
            0,
            vec![LatticeRect {
                c0: 0,
                r0: 0,
                c1: 3,
                r1: 3,
            }
            .expolygon(step)],
        ),
        rejection_parent(
            OBJECT,
            1,
            vec![LatticeRect {
                c0: 3,
                r0: 0,
                c1: 6,
                r1: 3,
            }
            .expolygon(step)],
        ),
    ];
    let footprints = vec![
        // Splits parent A and is disjoint from parent B.
        rejection_footprint(
            OBJECT,
            LatticeRect {
                c0: 1,
                r0: 1,
                c1: 2,
                r1: 2,
            },
            step,
        ),
        // Collides with the staged child of parent B, after the split above.
        rejection_footprint(
            OBJECT,
            LatticeRect {
                c0: 4,
                r0: 1,
                c1: 5,
                r1: 2,
            },
            step,
        ),
    ];
    let collide_id = modifier_sub_region_id(1, OBJECT, &footprints[1].polygons);
    let existing = SlicedRegion {
        object_id: OBJECT.to_string(),
        region_id: collide_id,
        polygons: footprints[1].polygons.clone(),
        ..SlicedRegion::default()
    };
    ModifierScene {
        mode: ModifierMode::LaterFailure,
        parents,
        footprints,
        preexisting_child: Some(existing),
        role_overlap: false,
    }
}

fn slice_for_scene(scene: &ModifierScene) -> SliceIR {
    let mut regions = scene.parents.clone();
    if let Some(child) = &scene.preexisting_child {
        regions.push(child.clone());
    }
    SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 1.0,
        regions,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent cell model.
// ─────────────────────────────────────────────────────────────────────────────

fn region_polygons(region: &SlicedRegion) -> impl Iterator<Item = &ExPolygon> {
    region
        .polygons
        .iter()
        .chain(region.infill_areas.iter())
        .chain(region.bridge_areas.iter())
        .chain(region.bottom_solid_fill.iter())
        .chain(region.top_solid_fill.iter())
        .chain(region.sparse_infill_area.iter())
        .chain(region.internal_solid_fill.iter())
        .chain(region.internal_bridge_areas.iter())
        .chain(region.raft_fill.iter())
}

fn cells_of(grid: &CellGrid, polygons: &[ExPolygon]) -> Result<CellSet, String> {
    CellSet::classify(grid, polygons).map_err(|error| error.to_string())
}

const CARRIERS: [&str; 6] = [
    "bridge_areas",
    "bottom_solid_fill",
    "top_solid_fill",
    "sparse_infill_area",
    "internal_solid_fill",
    "raft_fill",
];

fn carrier_polygons<'a>(region: &'a SlicedRegion, carrier: &str) -> &'a [ExPolygon] {
    match carrier {
        "bridge_areas" => &region.bridge_areas,
        "bottom_solid_fill" => &region.bottom_solid_fill,
        "top_solid_fill" => &region.top_solid_fill,
        "sparse_infill_area" => &region.sparse_infill_area,
        "internal_solid_fill" => &region.internal_solid_fill,
        "raft_fill" => &region.raft_fill,
        other => panic!("unknown carrier {other}"),
    }
}

/// First-owner carrier model: each footprint claims only the carrier cells the
/// parent has not already given to an earlier footprint.
fn model_carrier(
    grid: &CellGrid,
    domain: &[ExPolygon],
    active: &[(usize, &CellSet)],
) -> Result<(CellSet, Vec<CellSet>), String> {
    let mut remaining = cells_of(grid, domain)?;
    let mut children = Vec::with_capacity(active.len());
    for (_, footprint) in active {
        children.push(remaining.and(footprint));
        remaining = remaining.sub(footprint);
    }
    Ok((remaining, children))
}

fn internal_groups(parent: &SlicedRegion) -> Vec<(Option<f32>, Vec<ExPolygon>)> {
    if parent.internal_bridge_areas.is_empty() {
        return Vec::new();
    }
    if parent.internal_bridge_angles_deg.is_empty() {
        return vec![(None, parent.internal_bridge_areas.clone())];
    }
    let mut groups: Vec<(f32, Vec<ExPolygon>)> = Vec::new();
    for (index, polygon) in parent.internal_bridge_areas.iter().enumerate() {
        let angle = parent.internal_bridge_angle_deg(index);
        match groups.iter_mut().find(|(declared, _)| *declared == angle) {
            Some((_, polygons)) => polygons.push(polygon.clone()),
            None => groups.push((angle, vec![polygon.clone()])),
        }
    }
    groups
        .into_iter()
        .map(|(angle, poly)| (Some(angle), poly))
        .collect()
}

fn internal_pieces(region: &SlicedRegion, angle: Option<f32>) -> Vec<ExPolygon> {
    if region.internal_bridge_angles_deg.is_empty() {
        if angle.is_none() {
            region.internal_bridge_areas.clone()
        } else {
            Vec::new()
        }
    } else {
        region
            .internal_bridge_areas
            .iter()
            .zip(&region.internal_bridge_angles_deg)
            .filter(|(_, declared)| Some(**declared) == angle)
            .map(|(piece, _)| piece.clone())
            .collect()
    }
}

fn assert_parent_metadata(
    input: &SlicedRegion,
    observed: &SlicedRegion,
    label: &str,
) -> Result<(), String> {
    check_eq("object_id", label, &observed.object_id, &input.object_id)?;
    check_eq("region_id", label, &observed.region_id, &input.region_id)?;
    check_eq(
        "variant_chain",
        label,
        &observed.variant_chain,
        &input.variant_chain,
    )?;
    check_eq(
        "effective_layer_height",
        label,
        &observed.effective_layer_height,
        &input.effective_layer_height,
    )?;
    check_eq(
        "top_shell_index",
        label,
        &observed.top_shell_index,
        &input.top_shell_index,
    )?;
    check_eq(
        "bottom_shell_index",
        label,
        &observed.bottom_shell_index,
        &input.bottom_shell_index,
    )?;
    check_eq("is_bridge", label, &observed.is_bridge, &input.is_bridge)?;
    check_eq(
        "bridge_orientation_deg",
        label,
        &observed.bridge_orientation_deg,
        &input.bridge_orientation_deg,
    )?;
    check_eq(
        "segment_annotations",
        label,
        &observed.segment_annotations,
        &input.segment_annotations,
    )?;
    if input.nonplanar_surface.is_none() {
        return Err(format!(
            "{label}: the fixture must seed a nondefault nonplanar_surface"
        ));
    }
    check_eq(
        "nonplanar_surface",
        label,
        &observed.nonplanar_surface,
        &input.nonplanar_surface,
    )?;
    assert_parent_residue(input, observed, label)
}

fn assert_child_metadata(
    parent: &SlicedRegion,
    child: &SlicedRegion,
    label: &str,
) -> Result<(), String> {
    check_eq(
        "child object_id",
        label,
        &child.object_id,
        &parent.object_id,
    )?;
    check_eq(
        "child variant_chain",
        label,
        &child.variant_chain,
        &parent.variant_chain,
    )?;
    if !child.segment_annotations.is_empty() {
        return Err(format!(
            "{label}: minted child must clear segment_annotations, found {} entries",
            child.segment_annotations.len()
        ));
    }
    check_eq(
        "child parent id",
        label,
        &modifier_base_region_id(child.region_id),
        &Some(parent.region_id),
    )?;
    check_eq(
        "child effective_layer_height",
        label,
        &child.effective_layer_height,
        &parent.effective_layer_height,
    )?;
    check_eq(
        "child top_shell_index",
        label,
        &child.top_shell_index,
        &parent.top_shell_index,
    )?;
    check_eq(
        "child bottom_shell_index",
        label,
        &child.bottom_shell_index,
        &parent.bottom_shell_index,
    )?;
    check_eq(
        "child is_bridge",
        label,
        &child.is_bridge,
        &parent.is_bridge,
    )?;
    check_eq(
        "child bridge_orientation_deg",
        label,
        &child.bridge_orientation_deg,
        &parent.bridge_orientation_deg,
    )?;
    check_eq(
        "child nonplanar_surface",
        label,
        &child.nonplanar_surface,
        &parent.nonplanar_surface,
    )?;
    assert_child_residue(parent, child, label)
}

/// Clear every field the kernel explicitly partitions, leaving the metadata
/// residue a split must copy verbatim.
fn blank_partitioned_fields(region: &mut SlicedRegion) {
    region.polygons.clear();
    region.infill_areas.clear();
    region.bridge_areas.clear();
    region.bottom_solid_fill.clear();
    region.top_solid_fill.clear();
    region.sparse_infill_area.clear();
    region.internal_solid_fill.clear();
    region.internal_bridge_areas.clear();
    region.internal_bridge_angles_deg.clear();
    region.raft_fill.clear();
}

/// The parent's whole non-geometry state, compared as one value: every field the
/// kernel does not partition must be identical after a successful split. This
/// catches a field nothing names explicitly — `nonplanar_surface` is also
/// asserted by name above, but the residue comparison is what makes a future
/// metadata field falsifiable without a new assertion.
fn assert_parent_residue(
    input: &SlicedRegion,
    observed: &SlicedRegion,
    label: &str,
) -> Result<(), String> {
    let mut expected = input.clone();
    let mut actual = observed.clone();
    blank_partitioned_fields(&mut expected);
    blank_partitioned_fields(&mut actual);
    check_eq(
        "residue outside the partitioned fields",
        label,
        &actual,
        &expected,
    )
}

/// The child's whole non-geometry state: it must equal the parent's residue apart
/// from the two documented changes — its own minted `region_id` and the cleared
/// `segment_annotations` (both asserted by name above as well).
fn assert_child_residue(
    parent: &SlicedRegion,
    child: &SlicedRegion,
    label: &str,
) -> Result<(), String> {
    let mut expected = parent.clone();
    let mut actual = child.clone();
    blank_partitioned_fields(&mut expected);
    blank_partitioned_fields(&mut actual);
    expected.region_id = actual.region_id;
    expected.segment_annotations = actual.segment_annotations.clone();
    check_eq(
        "child residue outside the partitioned fields",
        label,
        &actual,
        &expected,
    )
}

fn check_eq<T: PartialEq + std::fmt::Debug>(
    field: &str,
    label: &str,
    observed: &T,
    expected: &T,
) -> Result<(), String> {
    if observed == expected {
        Ok(())
    } else {
        Err(format!(
            "{label}: {field} observed {observed:?}, expected {expected:?}"
        ))
    }
}

fn assert_internal_metadata(
    region: &SlicedRegion,
    parent: &SlicedRegion,
    aligned: bool,
    label: &str,
) -> Result<(), String> {
    if aligned {
        if region.internal_bridge_angles_deg.len() != region.internal_bridge_areas.len() {
            return Err(format!(
                "{label}: {} internal-bridge pieces but {} declared angles",
                region.internal_bridge_areas.len(),
                region.internal_bridge_angles_deg.len()
            ));
        }
        for angle in &region.internal_bridge_angles_deg {
            if !parent.internal_bridge_angles_deg.contains(angle) {
                return Err(format!(
                    "{label}: internal-bridge piece declared invented angle {angle}"
                ));
            }
        }
        Ok(())
    } else {
        if !region.internal_bridge_angles_deg.is_empty() {
            return Err(format!(
                "{label}: empty-angle input must keep an empty internal_bridge_angles_deg, \
                 found {:?}",
                region.internal_bridge_angles_deg
            ));
        }
        for index in 0..region.internal_bridge_areas.len() {
            let angle = region.internal_bridge_angle_deg(index);
            if angle != parent.bridge_orientation_deg {
                return Err(format!(
                    "{label}: fallback internal-bridge piece {index} reports {angle}, \
                     expected the parent's bridge_orientation_deg {}",
                    parent.bridge_orientation_deg
                ));
            }
        }
        Ok(())
    }
}

fn assert_partition(
    grid: &CellGrid,
    label: &str,
    parts: &[(String, Vec<ExPolygon>)],
    expected: &CellSet,
) -> Result<(), String> {
    let mut classified: Vec<(&str, CellSet)> = Vec::with_capacity(parts.len());
    for (name, polygons) in parts {
        assert_orthogonal_on_grid(polygons, grid)
            .map_err(|error| format!("{label} {name}: {error}"))?;
        let cells = cells_of(grid, polygons).map_err(|error| format!("{label} {name}: {error}"))?;
        classified.push((name.as_str(), cells));
    }
    let refs: Vec<(&str, &CellSet)> = classified
        .iter()
        .map(|(name, cells)| (*name, cells))
        .collect();
    assert_disjoint_union(&refs, expected, grid.cell_count())
        .map_err(|error| format!("{label}: {error}"))
}

fn same_identity(left: &SlicedRegion, right: &SlicedRegion) -> bool {
    left.object_id == right.object_id
        && left.region_id == right.region_id
        && left.variant_chain == right.variant_chain
}

/// Check one generated scene: the ownership model for `Valid`, the constructed
/// error plus full `SliceIR` restoration for the three rejection modes.
pub fn check_scene(scene: &ModifierScene, limits: &ModifierLimits) -> TestCaseResult {
    match check_scene_inner(scene, limits) {
        Ok(()) => Ok(()),
        Err(message) => Err(TestCaseError::fail(message)),
    }
}

fn check_scene_inner(scene: &ModifierScene, limits: &ModifierLimits) -> Result<(), String> {
    let cells = input_cells(scene);
    let derived = derived_mode(scene, cells.as_ref());
    if derived != scene.mode {
        return Err(format!(
            "generated scene is tagged {:?} but constructs {:?}",
            scene.mode, derived
        ));
    }
    match derived {
        ModifierMode::Valid => check_valid_scene(scene, limits),
        ModifierMode::Collision => check_constructed_collision(scene),
        ModifierMode::UnsupportedParent => check_constructed_unsupported_parent(scene),
        ModifierMode::LaterFailure => check_constructed_later_failure(scene),
    }
}

/// The private kernel must reject the scene with `fragment` and leave the whole
/// input `SliceIR` byte-identical — the transactional contract of the split.
fn expect_rejection(scene: &ModifierScene, fragment: &str) -> Result<String, String> {
    let before = slice_for_scene(scene);
    let mut slice = before.clone();
    let call = split_regions_by_modifier_footprints(
        &mut slice,
        scene
            .footprints
            .iter()
            .map(|footprint| (footprint.object_id.clone(), footprint.polygons.clone())),
    );
    let error = match call {
        Ok(()) => {
            return Err(format!(
                "the constructed {:?} scene was accepted instead of rejected",
                scene.mode
            ))
        }
        Err(error) => error,
    };
    if !error.contains(fragment) {
        return Err(format!(
            "{:?} rejection must name `{fragment}`, got {error}",
            scene.mode
        ));
    }
    if slice != before {
        return Err(format!(
            "{:?} rejection must leave the input SliceIR untouched, observed {} region(s) \
             against the input's {}",
            scene.mode,
            slice.regions.len(),
            before.regions.len()
        ));
    }
    Ok(error)
}

fn check_constructed_collision(scene: &ModifierScene) -> Result<(), String> {
    let parent = scene
        .parents
        .first()
        .ok_or_else(|| String::from("collision scene must carry a parent"))?;
    let footprint = scene
        .footprints
        .first()
        .ok_or_else(|| String::from("collision scene must carry a footprint"))?;
    if !parent_intersects(parent, footprint) {
        return Err(String::from(
            "collision scene's first footprint must reach its parent",
        ));
    }
    let id = modifier_sub_region_id(parent.region_id, &footprint.object_id, &footprint.polygons);
    let child = scene
        .preexisting_child
        .as_ref()
        .ok_or_else(|| String::from("collision scene must stage a pre-existing child"))?;
    if child.object_id != footprint.object_id
        || child.region_id != id
        || child.variant_chain != parent.variant_chain
    {
        return Err(format!(
            "collision scene's pre-existing child {} does not carry the prospective identity {id}",
            child.region_id
        ));
    }
    let error = expect_rejection(scene, "identity collision")?;
    if !error.contains(&id.to_string()) {
        return Err(format!(
            "collision error must name the colliding region id {id}, got {error}"
        ));
    }
    Ok(())
}

fn check_constructed_unsupported_parent(scene: &ModifierScene) -> Result<(), String> {
    let parent = scene
        .parents
        .first()
        .ok_or_else(|| String::from("unsupported-parent scene must carry a parent"))?;
    let footprint = scene
        .footprints
        .first()
        .ok_or_else(|| String::from("unsupported-parent scene must carry a footprint"))?;
    if modifier_sub_region_id_fits(parent.region_id) {
        return Err(format!(
            "unsupported-parent scene's parent id {} is encodable",
            parent.region_id
        ));
    }
    if !parent_intersects(parent, footprint) {
        return Err(String::from(
            "unsupported-parent scene's footprint must reach its parent",
        ));
    }
    let error = expect_rejection(scene, "parent_region_id")?;
    if !error.contains(&parent.region_id.to_string()) {
        return Err(format!(
            "unsupported-parent error must name the parent id {}, got {error}",
            parent.region_id
        ));
    }
    Ok(())
}

fn check_constructed_later_failure(scene: &ModifierScene) -> Result<(), String> {
    if scene.parents.len() < 2 || scene.footprints.len() < 2 {
        return Err(String::from(
            "later-failure scene must carry two parents and two footprints",
        ));
    }
    let first_parent = &scene.parents[0];
    let first_footprint = &scene.footprints[0];
    if !parent_intersects(first_parent, first_footprint) {
        return Err(String::from(
            "later-failure scene's first footprint must split the first parent",
        ));
    }
    // The earlier mint must be real work, not a pre-check: no input region may
    // already hold the first footprint's prospective identity.
    if input_holds_identity(scene, first_parent, first_footprint) {
        return Err(String::from(
            "later-failure scene's first footprint must not already collide",
        ));
    }
    expect_rejection(scene, "identity collision")?;
    Ok(())
}

fn check_valid_scene(scene: &ModifierScene, limits: &ModifierLimits) -> Result<(), String> {
    let mut slice = slice_for_scene(scene);
    split_regions_by_modifier_footprints(
        &mut slice,
        scene
            .footprints
            .iter()
            .map(|footprint| (footprint.object_id.clone(), footprint.polygons.clone())),
    )
    .map_err(|error| format!("valid generated footprint set was rejected: {error}"))?;

    // An empty footprint is inert: the kernel skips it before the per-parent
    // loop, so it mints no child and cannot restore an empty `infill_areas` or
    // otherwise touch a carrier. The model below proves the latter by
    // construction (no cell of any carrier leaves its parent); this pins the
    // former by identity.
    for footprint in scene
        .footprints
        .iter()
        .filter(|footprint| footprint.polygons.is_empty())
    {
        for parent in &scene.parents {
            if parent.object_id != footprint.object_id
                || !modifier_sub_region_id_fits(parent.region_id)
            {
                continue;
            }
            let id =
                modifier_sub_region_id(parent.region_id, &footprint.object_id, &footprint.polygons);
            let explained = scene.footprints.iter().any(|other| {
                !other.polygons.is_empty()
                    && other.object_id == footprint.object_id
                    && modifier_sub_region_id(parent.region_id, &other.object_id, &other.polygons)
                        == id
            });
            if explained {
                continue;
            }
            if slice
                .regions
                .iter()
                .any(|region| region.region_id == id && region.object_id == footprint.object_id)
            {
                return Err(format!(
                    "an empty footprint for object {} minted the child {id}",
                    footprint.object_id
                ));
            }
        }
    }

    let mut input_polygons: Vec<&ExPolygon> = Vec::new();
    for parent in &scene.parents {
        input_polygons.extend(region_polygons(parent));
    }
    for footprint in &scene.footprints {
        input_polygons.extend(footprint.polygons.iter());
    }
    let mut output_polygons: Vec<&ExPolygon> = Vec::new();
    for region in &slice.regions {
        output_polygons.extend(region_polygons(region));
    }
    let grid = CellGrid::from_region_boundaries(input_polygons, output_polygons, limits.max_cells)
        .map_err(|error| format!("case cell grid: {error}"))?;

    let footprint_cells: Vec<CellSet> = scene
        .footprints
        .iter()
        .map(|footprint| cells_of(&grid, &footprint.polygons))
        .collect::<Result<_, _>>()?;

    let parent_index: HashMap<u64, usize> = scene
        .parents
        .iter()
        .enumerate()
        .map(|(index, parent)| (parent.region_id, index))
        .collect();
    let mut observed_parents: Vec<Option<&SlicedRegion>> = vec![None; scene.parents.len()];
    let mut observed_children: Vec<HashMap<usize, &SlicedRegion>> =
        vec![HashMap::new(); scene.parents.len()];
    let mut observed_preexisting: Option<&SlicedRegion> = None;
    for region in &slice.regions {
        if region.region_id == MODIFIER_FOOTPRINT_REGION_ID {
            return Err(String::from(
                "the private kernel left a raw footprint staging region it was not given",
            ));
        }
        if is_modifier_namespace_id(region.region_id) {
            if scene
                .preexisting_child
                .as_ref()
                .is_some_and(|child| same_identity(child, region))
            {
                if observed_preexisting.replace(region).is_some() {
                    return Err(String::from("duplicate pre-existing modifier child"));
                }
                continue;
            }
            let base = modifier_base_region_id(region.region_id)
                .ok_or_else(|| String::from("modifier namespace id decodes no parent"))?;
            let parent_position = parent_index.get(&base).copied().ok_or_else(|| {
                format!(
                    "minted child {} names no generated parent (base {base})",
                    region.region_id
                )
            })?;
            let footprint_position = scene
                .footprints
                .iter()
                .position(|footprint| {
                    footprint.object_id == region.object_id
                        && modifier_sub_region_id(base, &footprint.object_id, &footprint.polygons)
                            == region.region_id
                })
                .ok_or_else(|| {
                    format!(
                        "minted child {} carries no supplied footprint identity",
                        region.region_id
                    )
                })?;
            if observed_children[parent_position]
                .insert(footprint_position, region)
                .is_some()
            {
                return Err(format!(
                    "two children minted for parent {parent_position} footprint {footprint_position}"
                ));
            }
            continue;
        }
        let parent_position = parent_index
            .get(&region.region_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "kernel emitted an unexpected ordinary region {} for object {}",
                    region.region_id, region.object_id
                )
            })?;
        if observed_parents[parent_position].replace(region).is_some() {
            return Err(format!("parent {parent_position} appears twice"));
        }
    }

    if let Some(child) = &scene.preexisting_child {
        let observed = observed_preexisting.ok_or_else(|| {
            String::from("the pre-existing modifier child disappeared from the split")
        })?;
        if observed != child {
            return Err(format!(
                "the pre-existing modifier child was mutated: observed {observed:?}, \
                 expected {child:?}"
            ));
        }
    } else if observed_preexisting.is_some() {
        return Err(String::from(
            "the kernel emitted a child for a scene with no pre-existing child",
        ));
    }

    let mut expected_children = 0usize;
    for (parent_position, parent) in scene.parents.iter().enumerate() {
        let observed_parent = observed_parents[parent_position].ok_or_else(|| {
            format!(
                "parent {parent_position} of object {} vanished",
                parent.object_id
            )
        })?;
        let parent_label = format!("parent {parent_position} ({})", parent.object_id);
        assert_parent_metadata(parent, observed_parent, &parent_label)?;

        let parent_cells = cells_of(&grid, &parent.polygons)?;
        let mut remaining = parent_cells.clone();
        // (footprint position, parent-claimed cells, full footprint cells).
        //
        // `claimed` is what the child's `polygons` receive (parent ∩ footprint).
        // Carriers are split against the FULL footprint, because the kernel
        // intersects/differences each carrier with the raw footprint; a carrier
        // that reaches outside the parent outline (e.g. across a hole) is
        // reduced by footprint cells the parent does not own. Keeping both sets
        // is what makes first-owner ownership exact on every carrier domain.
        let mut active: Vec<(usize, CellSet, CellSet)> = Vec::new();
        for (footprint_position, footprint) in scene.footprints.iter().enumerate() {
            if footprint.object_id != parent.object_id {
                continue;
            }
            let claimed = remaining.and(&footprint_cells[footprint_position]);
            if claimed.is_empty() {
                continue;
            }
            active.push((
                footprint_position,
                claimed,
                footprint_cells[footprint_position].clone(),
            ));
            remaining = remaining.sub(&footprint_cells[footprint_position]);
        }
        expected_children += active.len();
        let active_footprints: Vec<(usize, &CellSet)> = active
            .iter()
            .map(|(position, _claimed, full)| (*position, full))
            .collect();

        compare_region_to_cells(
            &observed_parent.polygons,
            &remaining,
            &grid,
            &format!("{parent_label} polygons"),
        )
        .map_err(|error| error.to_string())?;

        let observed_minted = &observed_children[parent_position];
        if observed_minted.len() != active.len() {
            return Err(format!(
                "{parent_label}: {} minted children, model expects {} \
                 (footprints {:?})",
                observed_minted.len(),
                active.len(),
                active
                    .iter()
                    .map(|(position, _claimed, _full)| *position)
                    .collect::<Vec<_>>()
            ));
        }
        for (position, _claimed, _full) in &active {
            if !observed_minted.contains_key(position) {
                return Err(format!(
                    "{parent_label}: footprint {position} owns geometry but minted no child"
                ));
            }
        }

        let mut polygon_parts = vec![(
            format!("{parent_label} base polygons"),
            observed_parent.polygons.clone(),
        )];
        for (position, claimed, _full) in &active {
            let child = observed_minted[position];
            compare_region_to_cells(
                &child.polygons,
                claimed,
                &grid,
                &format!("{parent_label} footprint {position} polygons"),
            )
            .map_err(|error| error.to_string())?;
            assert_child_metadata(
                parent,
                child,
                &format!("{parent_label} footprint {position}"),
            )?;
            polygon_parts.push((
                format!("{parent_label} footprint {position} polygons"),
                child.polygons.clone(),
            ));
        }
        assert_partition(
            &grid,
            &format!("{parent_label} polygon partition"),
            &polygon_parts,
            &parent_cells,
        )?;

        // Empty input infill is restored from `polygons` only when some
        // footprint actually reaches that parent; otherwise it stays empty.
        let infill_empty = parent.infill_areas.is_empty();
        let infill_domain = if infill_empty {
            parent.polygons.as_slice()
        } else {
            parent.infill_areas.as_slice()
        };
        let (base_infill, child_infills) = if infill_empty && active.is_empty() {
            (CellSet::empty(grid.cell_count()), Vec::new())
        } else {
            model_carrier(&grid, infill_domain, &active_footprints)?
        };
        compare_region_to_cells(
            &observed_parent.infill_areas,
            &base_infill,
            &grid,
            &format!("{parent_label} infill_areas"),
        )
        .map_err(|error| error.to_string())?;
        let mut infill_parts = vec![(
            format!("{parent_label} base infill_areas"),
            observed_parent.infill_areas.clone(),
        )];
        for (position, _claimed, _full) in &active {
            let child = observed_minted[position];
            let expected = &child_infills[position_in(&active, *position)];
            compare_region_to_cells(
                &child.infill_areas,
                expected,
                &grid,
                &format!("{parent_label} footprint {position} infill_areas"),
            )
            .map_err(|error| error.to_string())?;
            infill_parts.push((
                format!("{parent_label} footprint {position} infill_areas"),
                child.infill_areas.clone(),
            ));
        }
        // With an empty input and no reaching footprint the kernel never
        // restores the field, so the partition domain is empty too.
        let expected_infill_domain = if infill_empty && active.is_empty() {
            CellSet::empty(grid.cell_count())
        } else {
            cells_of(&grid, infill_domain)?
        };
        assert_partition(
            &grid,
            &format!("{parent_label} infill_areas partition"),
            &infill_parts,
            &expected_infill_domain,
        )?;

        for carrier in CARRIERS {
            let domain = carrier_polygons(parent, carrier);
            let (base_cells, child_cells) = model_carrier(&grid, domain, &active_footprints)?;
            compare_region_to_cells(
                carrier_polygons(observed_parent, carrier),
                &base_cells,
                &grid,
                &format!("{parent_label} {carrier}"),
            )
            .map_err(|error| error.to_string())?;
            let mut parts = vec![(
                format!("{parent_label} base {carrier}"),
                carrier_polygons(observed_parent, carrier).to_vec(),
            )];
            for (position, _claimed, _full) in &active {
                let child = observed_minted[position];
                let expected = &child_cells[position_in(&active, *position)];
                compare_region_to_cells(
                    carrier_polygons(child, carrier),
                    expected,
                    &grid,
                    &format!("{parent_label} footprint {position} {carrier}"),
                )
                .map_err(|error| error.to_string())?;
                parts.push((
                    format!("{parent_label} footprint {position} {carrier}"),
                    carrier_polygons(child, carrier).to_vec(),
                ));
            }
            assert_partition(
                &grid,
                &format!("{parent_label} {carrier} partition"),
                &parts,
                &cells_of(&grid, domain)?,
            )?;
        }

        let aligned = !parent.internal_bridge_angles_deg.is_empty();
        let groups = internal_groups(parent);
        for (angle, group) in &groups {
            let (base_cells, child_cells) = model_carrier(&grid, group, &active_footprints)?;
            let base_pieces = internal_pieces(observed_parent, *angle);
            compare_region_to_cells(
                &base_pieces,
                &base_cells,
                &grid,
                &format!("{parent_label} internal bridge {angle:?} base"),
            )
            .map_err(|error| error.to_string())?;
            let mut parts = vec![(
                format!("{parent_label} base internal bridge {angle:?}"),
                base_pieces,
            )];
            for (position, _claimed, _full) in &active {
                let child = observed_minted[position];
                let expected = &child_cells[position_in(&active, *position)];
                let pieces = internal_pieces(child, *angle);
                compare_region_to_cells(
                    &pieces,
                    expected,
                    &grid,
                    &format!("{parent_label} footprint {position} internal bridge {angle:?}"),
                )
                .map_err(|error| error.to_string())?;
                parts.push((
                    format!("{parent_label} footprint {position} internal bridge {angle:?}"),
                    pieces,
                ));
            }
            assert_partition(
                &grid,
                &format!("{parent_label} internal bridge {angle:?} partition"),
                &parts,
                &cells_of(&grid, group)?,
            )?;
        }
        assert_internal_metadata(
            observed_parent,
            parent,
            aligned,
            &format!("{parent_label} internal bridges"),
        )?;
        if groups.is_empty() {
            if !observed_parent.internal_bridge_areas.is_empty() {
                return Err(format!(
                    "{parent_label}: no internal-bridge input but the base gained pieces"
                ));
            }
            for (position, _claimed, _full) in &active {
                if !observed_minted[position].internal_bridge_areas.is_empty() {
                    return Err(format!(
                        "{parent_label}: no internal-bridge input but footprint \
                         {position} gained pieces"
                    ));
                }
            }
        }
        for (position, _claimed, _full) in &active {
            assert_internal_metadata(
                observed_minted[position],
                parent,
                aligned,
                &format!("{parent_label} footprint {position} internal bridges"),
            )?;
        }
    }

    let expected_regions =
        scene.parents.len() + usize::from(scene.preexisting_child.is_some()) + expected_children;
    if slice.regions.len() != expected_regions {
        return Err(format!(
            "kernel produced {} regions, model expects {expected_regions} \
             ({} parents, {} minted children, pre-existing child: {})",
            slice.regions.len(),
            scene.parents.len(),
            expected_children,
            scene.preexisting_child.is_some()
        ));
    }
    Ok(())
}

/// Position of a footprint among the active (minting) footprints, matching the
/// order the carrier model returns.
fn position_in(active: &[(usize, CellSet, CellSet)], footprint_position: usize) -> usize {
    active
        .iter()
        .position(|(position, _claimed, _full)| *position == footprint_position)
        .expect("active footprint positions are unique")
}

// ─────────────────────────────────────────────────────────────────────────────
// Witness derivation shared by the checker and the shrink audit.
// ─────────────────────────────────────────────────────────────────────────────

/// Classify each small polygon set on one grid built from every set's
/// boundaries. This reads witness structure back out of a generated scene; it
/// never predicts kernel output. A grid that cannot be formed (or a midpoint on
/// a ring) means the relation cannot be certified, and the caller records no
/// tag rather than assuming one.
fn classify_sets(sets: &[&[ExPolygon]]) -> Option<Vec<CellSet>> {
    let inputs: Vec<&ExPolygon> = sets.iter().flat_map(|polygons| polygons.iter()).collect();
    let grid =
        CellGrid::from_region_boundaries(inputs, std::iter::empty::<&ExPolygon>(), 4096).ok()?;
    sets.iter()
        .map(|polygons| CellSet::classify(&grid, polygons).ok())
        .collect()
}

/// Every generated parent and footprint, classified on one shared grid.
fn input_cells(scene: &ModifierScene) -> Option<(Vec<CellSet>, Vec<CellSet>)> {
    let sets: Vec<&[ExPolygon]> = scene
        .parents
        .iter()
        .map(|parent| parent.polygons.as_slice())
        .chain(
            scene
                .footprints
                .iter()
                .map(|footprint| footprint.polygons.as_slice()),
        )
        .collect();
    let classified = classify_sets(&sets)?;
    let (parents, footprints) = classified.split_at(scene.parents.len());
    Some((parents.to_vec(), footprints.to_vec()))
}

fn parent_intersects(parent: &SlicedRegion, footprint: &ModifierFootprint) -> bool {
    let sets: [&[ExPolygon]; 2] = [parent.polygons.as_slice(), footprint.polygons.as_slice()];
    let Some(cells) = classify_sets(&sets) else {
        return false;
    };
    !cells[0].and(&cells[1]).is_empty()
}

/// Whether some input region already carries the full identity the kernel would
/// mint for `footprint` on `parent`.
fn input_holds_identity(
    scene: &ModifierScene,
    parent: &SlicedRegion,
    footprint: &ModifierFootprint,
) -> bool {
    if footprint.polygons.is_empty() {
        return false;
    }
    let id = modifier_sub_region_id(parent.region_id, &footprint.object_id, &footprint.polygons);
    scene
        .parents
        .iter()
        .chain(scene.preexisting_child.iter())
        .any(|region| {
            region.object_id == footprint.object_id
                && region.region_id == id
                && region.variant_chain == parent.variant_chain
        })
}

/// The failure the generated scene actually constructs, evaluated in the same
/// `(footprint, parent)` order the kernel walks. Reads the scene, never the mode
/// word, so a mode tag can only survive shrinking if its witness did.
fn derived_mode(
    scene: &ModifierScene,
    cells: Option<&(Vec<CellSet>, Vec<CellSet>)>,
) -> ModifierMode {
    let mut identities: Vec<(String, u64, Vec<(String, PaintValue)>)> = scene
        .parents
        .iter()
        .chain(scene.preexisting_child.iter())
        .map(|region| {
            (
                region.object_id.clone(),
                region.region_id,
                region.variant_chain.clone(),
            )
        })
        .collect();
    for (footprint_index, footprint) in scene.footprints.iter().enumerate() {
        if footprint.polygons.is_empty() {
            continue;
        }
        for (parent_index, parent) in scene.parents.iter().enumerate() {
            if parent.object_id != footprint.object_id {
                continue;
            }
            let reaches = cells.is_some_and(|(parents, footprints)| {
                parents
                    .get(parent_index)
                    .zip(footprints.get(footprint_index))
                    .is_some_and(|(parent_cells, footprint_cells)| {
                        !parent_cells.and(footprint_cells).is_empty()
                    })
            });
            if !reaches {
                continue;
            }
            if !modifier_sub_region_id_fits(parent.region_id) {
                return ModifierMode::UnsupportedParent;
            }
            let id =
                modifier_sub_region_id(parent.region_id, &footprint.object_id, &footprint.polygons);
            let identity = (
                footprint.object_id.clone(),
                id,
                parent.variant_chain.clone(),
            );
            if identities.contains(&identity) {
                return if footprint_index == 0 {
                    ModifierMode::Collision
                } else {
                    ModifierMode::LaterFailure
                };
            }
            identities.push(identity);
        }
    }
    ModifierMode::Valid
}

/// Whether two ordered footprints claim a shared cell of the first parent, so
/// first-owner precedence decides geometry rather than a disjoint placement.
fn later_footprint_overlap(
    scene: &ModifierScene,
    cells: Option<&(Vec<CellSet>, Vec<CellSet>)>,
) -> bool {
    let Some((parents, footprints)) = cells else {
        return false;
    };
    let Some(parent) = scene.parents.first() else {
        return false;
    };
    let Some(parent_cells) = parents.first() else {
        return false;
    };
    let mut claimed: Option<CellSet> = None;
    for (index, footprint) in scene.footprints.iter().enumerate() {
        if footprint.object_id != parent.object_id {
            continue;
        }
        let Some(footprint_cells) = footprints.get(index) else {
            continue;
        };
        let owner = parent_cells.and(footprint_cells);
        if owner.is_empty() {
            continue;
        }
        if let Some(previous) = &claimed {
            if !previous.and(&owner).is_empty() {
                return true;
            }
        }
        claimed = Some(match claimed {
            Some(previous) => previous.or(&owner),
            None => owner,
        });
    }
    false
}

/// Whether one cell belongs to two or more of the first parent's original
/// bridge/bottom/top carriers, so the documented role precedence is exercised.
fn role_precedence_overlap(scene: &ModifierScene) -> bool {
    let Some(first) = scene.parents.first() else {
        return false;
    };
    let sets: [&[ExPolygon]; 3] = [
        first.bridge_areas.as_slice(),
        first.bottom_solid_fill.as_slice(),
        first.top_solid_fill.as_slice(),
    ];
    let Some(cells) = classify_sets(&sets) else {
        return false;
    };
    (0..cells.len()).any(|left| {
        ((left + 1)..cells.len()).any(|right| !cells[left].and(&cells[right]).is_empty())
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-N2 shrink witness audit.
// ─────────────────────────────────────────────────────────────────────────────

/// One tag the modifier generator distinguishes on a mode-defining
/// (`no_shrink`) draw or in its construction.
///
/// Every tag is read back from the generated scene, never from the raw mode
/// word, so an audit that keeps the tag proves the *witness* survived
/// `simplify()`. Parent/carrier tags are taken from the first parent and the
/// optional pre-existing child because those exist for every draw:
/// `max_regions` and `max_footprints` are shrinkable, so a tag on parent N > 0
/// could legally disappear and would certify nothing. The mode tags are derived
/// structurally (`derived_mode`), and the rejection scenes are built entirely
/// from literal geometry, so their tags are invariant by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModifierWitnessTag {
    /// The scene is a valid first-owner ownership case.
    Valid,
    /// The scene constructs an identity collision on its first footprint.
    Collision,
    /// The scene constructs an unencodable-parent rejection.
    UnsupportedParent,
    /// The scene constructs a collision in a later footprint after an earlier split.
    LaterFailure,
    /// First parent belongs to `OBJECT_A`.
    FirstObjectA,
    /// First parent belongs to `OBJECT_B`.
    FirstObjectB,
    /// First parent carries an empty ordered chain.
    EmptyChain,
    /// First parent carries the `material` chain.
    MaterialChain,
    /// First parent carries the `density` chain.
    DensityChain,
    /// First parent carries the `seam` chain.
    SeamChain,
    /// First parent has empty input `infill_areas` (the restore branch).
    EmptyInfill,
    /// First parent has nonempty `infill_areas`.
    NonEmptyInfill,
    /// First parent is a single lattice component.
    SingleComponent,
    /// First parent is a two-component concave union.
    TwoComponents,
    /// First parent carries one internal-bridge piece.
    SingleBridgePiece,
    /// First parent carries a two-piece internal-bridge carrier.
    TwoPieceBridge,
    /// First parent's internal bridges fall back to `bridge_orientation_deg`.
    FallbackBridgeAngles,
    /// First parent's internal bridges declare per-piece angles.
    AlignedBridgeAngles,
    /// First parent is a plain region.
    PlainParent,
    /// First parent carries bridge flags (`is_bridge` and shell indices).
    BridgeParent,
    /// The scene stages a pre-existing modifier child that must survive.
    PreexistingChild,
    /// The scene stages a polygon-free footprint that must be inert.
    EmptyFootprint,
    /// The first parent is a rectangle with a rectangular hole.
    HoledParent,
    /// Two ordered footprints claim a shared cell of the first parent.
    LaterFootprintOverlap,
    /// A cell belongs to two or more of the first parent's bridge/bottom/top
    /// carriers, so role precedence is exercised.
    RolePrecedenceOverlap,
}

impl ModifierWitnessTag {
    /// Every tag, in a stable order.
    pub const ALL: [ModifierWitnessTag; 25] = [
        ModifierWitnessTag::Valid,
        ModifierWitnessTag::Collision,
        ModifierWitnessTag::UnsupportedParent,
        ModifierWitnessTag::LaterFailure,
        ModifierWitnessTag::FirstObjectA,
        ModifierWitnessTag::FirstObjectB,
        ModifierWitnessTag::EmptyChain,
        ModifierWitnessTag::MaterialChain,
        ModifierWitnessTag::DensityChain,
        ModifierWitnessTag::SeamChain,
        ModifierWitnessTag::EmptyInfill,
        ModifierWitnessTag::NonEmptyInfill,
        ModifierWitnessTag::SingleComponent,
        ModifierWitnessTag::TwoComponents,
        ModifierWitnessTag::SingleBridgePiece,
        ModifierWitnessTag::TwoPieceBridge,
        ModifierWitnessTag::FallbackBridgeAngles,
        ModifierWitnessTag::AlignedBridgeAngles,
        ModifierWitnessTag::PlainParent,
        ModifierWitnessTag::BridgeParent,
        ModifierWitnessTag::PreexistingChild,
        ModifierWitnessTag::EmptyFootprint,
        ModifierWitnessTag::HoledParent,
        ModifierWitnessTag::LaterFootprintOverlap,
        ModifierWitnessTag::RolePrecedenceOverlap,
    ];
}

/// Tags the audit must observe explicitly. The four mode tags are required so a
/// green audit cannot be a valid-only population; the two precedence tags are
/// required because they are the whole point of the ownership model; the holed
/// parent and the empty footprint are required because their branches are new
/// domain. A run that observed no tags at all fails separately.
const REQUIRED_WITNESS_TAGS: [ModifierWitnessTag; 8] = [
    ModifierWitnessTag::Valid,
    ModifierWitnessTag::Collision,
    ModifierWitnessTag::UnsupportedParent,
    ModifierWitnessTag::LaterFailure,
    ModifierWitnessTag::HoledParent,
    ModifierWitnessTag::EmptyFootprint,
    ModifierWitnessTag::LaterFootprintOverlap,
    ModifierWitnessTag::RolePrecedenceOverlap,
];

/// Every mode tag the generated scene carries.
pub fn witness_tags(scene: &ModifierScene) -> BTreeSet<ModifierWitnessTag> {
    let mut tags = BTreeSet::new();
    let cells = input_cells(scene);
    tags.insert(match derived_mode(scene, cells.as_ref()) {
        ModifierMode::Valid => ModifierWitnessTag::Valid,
        ModifierMode::Collision => ModifierWitnessTag::Collision,
        ModifierMode::UnsupportedParent => ModifierWitnessTag::UnsupportedParent,
        ModifierMode::LaterFailure => ModifierWitnessTag::LaterFailure,
    });
    if scene.role_overlap && role_precedence_overlap(scene) {
        tags.insert(ModifierWitnessTag::RolePrecedenceOverlap);
    }
    let first = &scene.parents[0];
    tags.insert(if first.object_id == OBJECT_A {
        ModifierWitnessTag::FirstObjectA
    } else {
        ModifierWitnessTag::FirstObjectB
    });
    tags.insert(match first.variant_chain.first() {
        None => ModifierWitnessTag::EmptyChain,
        Some((name, _)) if name == "material" => ModifierWitnessTag::MaterialChain,
        Some((name, _)) if name == "density" => ModifierWitnessTag::DensityChain,
        Some(_) => ModifierWitnessTag::SeamChain,
    });
    tags.insert(if first.infill_areas.is_empty() {
        ModifierWitnessTag::EmptyInfill
    } else {
        ModifierWitnessTag::NonEmptyInfill
    });
    tags.insert(if first.polygons.len() > 1 {
        ModifierWitnessTag::TwoComponents
    } else {
        ModifierWitnessTag::SingleComponent
    });
    tags.insert(if first.internal_bridge_areas.len() > 1 {
        ModifierWitnessTag::TwoPieceBridge
    } else {
        ModifierWitnessTag::SingleBridgePiece
    });
    tags.insert(if first.internal_bridge_angles_deg.is_empty() {
        ModifierWitnessTag::FallbackBridgeAngles
    } else {
        ModifierWitnessTag::AlignedBridgeAngles
    });
    tags.insert(if first.is_bridge {
        ModifierWitnessTag::BridgeParent
    } else {
        ModifierWitnessTag::PlainParent
    });
    if scene.preexisting_child.is_some() {
        tags.insert(ModifierWitnessTag::PreexistingChild);
    }
    if first
        .polygons
        .iter()
        .any(|polygon| !polygon.holes.is_empty())
    {
        tags.insert(ModifierWitnessTag::HoledParent);
    }
    if scene
        .footprints
        .iter()
        .any(|footprint| footprint.polygons.is_empty())
    {
        tags.insert(ModifierWitnessTag::EmptyFootprint);
    }
    if later_footprint_overlap(scene, cells.as_ref()) {
        tags.insert(ModifierWitnessTag::LaterFootprintOverlap);
    }
    tags
}

/// Number of tags the shrink audit must reach.
const SHRINK_AUDIT_MODES: usize = ModifierWitnessTag::ALL.len();

/// Fixed-seed AC-N2 shrink audit.
///
/// Walks `simplify()` on 128 fixed-seed trees. Every simplification step asserts
/// that the case's full tag set is unchanged — that is the witness-retention
/// claim — and the same `check_scene` `run_property` runs is applied to the
/// generated tree and to the final simplified tree, the value a failing case
/// would actually report. (Re-running the checker on every intermediate step
/// costs tens of milliseconds per call — 8320 calls measured at 193 s in this
/// workspace's debug test profile — because it drives the production split
/// kernel and the independent cell model; the intermediate states are search
/// artifacts, not reported witnesses.)
/// `simplify()` does real work here: the parent/footprint counts and the seed
/// vector shrink, so a tag lost to shrinking would mean the generator derives
/// its witness from a shrinkable draw.
///
/// The loop records whether it stopped because `simplify()` returned false (a
/// fixpoint) or because it reached the 64-step audit cap. The tree checked at
/// the end is therefore the *final simplified tree at the audit cap* when the
/// cap was hit — not a maximally shrunk tree.
pub fn shrink_audit() {
    let limits =
        checked_modifier_limits(2, 2, 16, 512).expect("H-RUNTIME-MODIFIERS shrink audit ceilings");
    let strategy = modifier_strategy(&limits);
    let mut runner = proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 1,
        max_local_rejects: 0,
        max_global_rejects: 0,
        max_flat_map_regens: 0,
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x5eed_c0de),
        failure_persistence: None,
        ..proptest::test_runner::Config::default()
    });
    let mut seen = BTreeSet::new();
    let mut simplifications = 0_usize;
    for _ in 0..128 {
        let mut tree = strategy.new_tree(&mut runner).expect("shrink audit tree");
        let initial = tree.current();
        let tags = witness_tags(&initial);
        seen.extend(tags.iter().copied());
        check_scene(&initial, &limits).unwrap_or_else(|error| {
            panic!(
                "H-RUNTIME-MODIFIERS: the unsimplified audit tree must satisfy the contract \
                 (tags {tags:?}): {error}"
            )
        });
        let required: BTreeSet<ModifierWitnessTag> = tags
            .iter()
            .copied()
            .filter(|tag| REQUIRED_WITNESS_TAGS.contains(tag))
            .collect();
        let mut steps = 0_usize;
        let mut reached_fixpoint = false;
        while steps < 64 {
            if !tree.simplify() {
                reached_fixpoint = true;
                break;
            }
            steps += 1;
            simplifications += 1;
            let current = witness_tags(&tree.current());
            assert!(
                required.is_subset(&current),
                "H-RUNTIME-MODIFIERS: shrinking lost a required witness tag: {:?} -> {:?}",
                required,
                current
            );
        }
        if steps > 0 {
            let ending = if reached_fixpoint {
                "fixpoint"
            } else {
                "audit cap (64 steps; further simplification is not ruled out)"
            };
            check_scene(&tree.current(), &limits).unwrap_or_else(|error| {
                panic!(
                    "H-RUNTIME-MODIFIERS: the final simplified tree at the {ending} no longer \
                     satisfies the contract (tags {tags:?}): {error}"
                )
            });
        }
    }
    assert!(
        !seen.is_empty(),
        "H-RUNTIME-MODIFIERS: shrink audit observed zero witness tags, so it certifies nothing"
    );
    for tag in REQUIRED_WITNESS_TAGS {
        assert!(
            seen.contains(&tag),
            "H-RUNTIME-MODIFIERS: shrink audit never observed required witness tag {tag:?}"
        );
    }
    assert!(
        seen.len() >= SHRINK_AUDIT_MODES,
        "H-RUNTIME-MODIFIERS: shrink audit reached {} modes, expected at least {}",
        seen.len(),
        SHRINK_AUDIT_MODES
    );
    assert!(
        simplifications > 0,
        "H-RUNTIME-MODIFIERS: the shrink audit simplified nothing, so it certifies nothing"
    );
}

/// Run the registry-selected `H-RUNTIME-MODIFIERS` property.
pub fn run_property() {
    let run =
        PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("H-RUNTIME-MODIFIERS registry context");
    let limits = checked_modifier_limits(
        run.bound("max_regions").expect("max_regions bound"),
        run.bound("max_footprints").expect("max_footprints bound"),
        run.bound("max_abs_units").expect("max_abs_units bound"),
        run.bound("max_cells").expect("max_cells bound"),
    )
    .expect("H-RUNTIME-MODIFIERS safety ceilings");
    shrink_audit();
    let strategy = modifier_strategy(&limits);
    run.check(&strategy, |scene, _guard| check_scene(&scene, &limits))
        .expect("H-RUNTIME-MODIFIERS first-owner modifier contract");
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic controls.
// ─────────────────────────────────────────────────────────────────────────────

fn unit_rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Rect {
    Rect::new(x0, y0, x1, y1).expect("control rectangle must be non-degenerate")
}

fn control_region(
    object_id: &str,
    region_id: u64,
    polygons: Vec<ExPolygon>,
    infill_areas: Vec<ExPolygon>,
    variant_chain: Vec<(String, PaintValue)>,
) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id,
        polygons,
        infill_areas,
        variant_chain,
        effective_layer_height: 0.2,
        ..SlicedRegion::default()
    }
}

fn slice_of(regions: Vec<SlicedRegion>) -> SliceIR {
    slice_at(regions, 1.0)
}

fn slice_at(regions: Vec<SlicedRegion>, z: f32) -> SliceIR {
    SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z,
        regions,
    }
}

/// Interior slice plane used by the box-mesh prepass controls.
const INTERIOR_PLANE_MM: f32 = 0.5;

fn find_child<'a>(
    slice: &'a SliceIR,
    parent_id: u64,
    object_id: &str,
    footprint: &[ExPolygon],
) -> Option<&'a SlicedRegion> {
    let region_id = modifier_sub_region_id(parent_id, object_id, footprint);
    slice
        .regions
        .iter()
        .find(|region| region.region_id == region_id && region.object_id == object_id)
}

fn find_child_with_chain<'a>(
    slice: &'a SliceIR,
    parent_id: u64,
    object_id: &str,
    footprint: &[ExPolygon],
    variant_chain: &[(String, PaintValue)],
) -> Option<&'a SlicedRegion> {
    let region_id = modifier_sub_region_id(parent_id, object_id, footprint);
    slice.regions.iter().find(|region| {
        region.region_id == region_id
            && region.object_id == object_id
            && region.variant_chain == variant_chain
    })
}

fn control_grid(inputs: &[&[ExPolygon]], outputs: &[&[ExPolygon]]) -> CellGrid {
    CellGrid::from_region_boundaries(
        inputs.iter().flat_map(|polygons| polygons.iter()),
        outputs.iter().flat_map(|polygons| polygons.iter()),
        2048,
    )
    .expect("control cell grid")
}

fn control_cells(grid: &CellGrid, polygons: &[ExPolygon]) -> CellSet {
    CellSet::classify(grid, polygons).expect("control polygons classify on their own grid")
}

/// AC-3 ownership/angle controls. Every known-wrong model below is spelled out
/// independently, so a defect the generative property must catch is proven
/// catchable without sampling.
pub fn runtime_modifier_ownership_and_angle_controls() {
    shrink_audit();
    lower_priority_overlap_theft_disagrees();
    cross_object_and_chain_isolation();
    omitted_raft_and_internal_carriers_disagree();
    lost_internal_bridge_angle_association_disagrees();
    prepass_priority_and_document_order_decide_ownership();
}

fn lower_priority_overlap_theft_disagrees() {
    const OBJECT: &str = "modifier-control-theft";
    let parent_polygon = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let first = vec![unit_rect(1, 1, 7, 7).to_expolygon()];
    let second = vec![unit_rect(4, 4, 9, 9).to_expolygon()];
    let mut slice = slice_of(vec![control_region(
        OBJECT,
        0,
        parent_polygon.clone(),
        Vec::new(),
        Vec::new(),
    )]);

    split_regions_by_modifier_footprints(
        &mut slice,
        [
            (OBJECT.to_string(), first.clone()),
            (OBJECT.to_string(), second.clone()),
        ],
    )
    .expect("the overlap witness must split");

    let observed_parent = slice
        .regions
        .iter()
        .find(|region| region.region_id == 0)
        .expect("base region remains");
    let child_first =
        find_child(&slice, 0, OBJECT, &first).expect("first footprint minted a child");
    let child_second =
        find_child(&slice, 0, OBJECT, &second).expect("second footprint minted a child");
    let grid = control_grid(
        &[&parent_polygon, &first, &second],
        &[
            &observed_parent.polygons,
            &child_first.polygons,
            &child_second.polygons,
        ],
    );
    let parent_cells = control_cells(&grid, &parent_polygon);
    let first_cells = control_cells(&grid, &first);
    let second_cells = control_cells(&grid, &second);
    let expected_first = parent_cells.and(&first_cells);
    let expected_second = parent_cells.and(&second_cells).sub(&first_cells);
    let expected_base = parent_cells.sub(&first_cells).sub(&second_cells);

    compare_region_to_cells(&child_first.polygons, &expected_first, &grid, "child-first")
        .expect("first footprint owns its whole intersection");
    compare_region_to_cells(
        &child_second.polygons,
        &expected_second,
        &grid,
        "child-second",
    )
    .expect("second footprint receives only the remaining parent cells");
    compare_region_to_cells(
        &observed_parent.polygons,
        &expected_base,
        &grid,
        "parent remainder",
    )
    .expect("parent keeps the unclaimed remainder");

    // Known-wrong model: a later (lower-priority) footprint overwrites the
    // earlier owner.
    let later_wins = parent_cells.and(&second_cells);
    let actual_second = control_cells(&grid, &child_second.polygons);
    let theft = later_wins.sub(&actual_second);
    assert!(
        !theft.is_empty(),
        "the two footprints must overlap inside the parent for this witness to mean anything"
    );
    assert!(
        theft.is_subset_of(&first_cells.and(&second_cells)),
        "every cell the later-wins model steals must lie in the overlap of both footprints"
    );
    assert_ne!(
        actual_second, later_wins,
        "a later-footprint-wins model must disagree with the real first-owner split"
    );
}

fn cross_object_and_chain_isolation() {
    let parent_polygon = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let object_a = "modifier-control-object-a";
    let object_b = "modifier-control-object-b";

    // A footprint for object B must not reduce object A's parent at all. The
    // parent's empty infill also stays empty: the kernel only restores it when a
    // matching footprint reaches the parent.
    let parent_a = control_region(object_a, 0, parent_polygon.clone(), Vec::new(), Vec::new());
    let parent_b = control_region(object_b, 0, parent_polygon.clone(), Vec::new(), Vec::new());
    let mut slice = slice_of(vec![parent_a.clone(), parent_b.clone()]);
    split_regions_by_modifier_footprints(&mut slice, [(object_b.to_string(), footprint.clone())])
        .expect("object-B footprint must split object-B parents");

    let observed_a = slice
        .regions
        .iter()
        .find(|region| region.object_id == object_a && region.region_id == 0)
        .expect("object-A parent remains");
    assert_eq!(
        observed_a, &parent_a,
        "a footprint for object B must leave object A's parent byte-identical"
    );
    let child_b = find_child(&slice, 0, object_b, &footprint)
        .expect("object-B footprint mints an object-B child");
    assert_eq!(child_b.object_id, object_b);
    assert_eq!(
        child_b.variant_chain, parent_b.variant_chain,
        "object-B child copies the object-B parent's ordered chain"
    );
    assert_eq!(
        slice.regions.len(),
        3,
        "exactly one child is minted for the object-B footprint"
    );

    // Two parents of one object with distinct ordered chains: each keeps its own
    // chain and parent id. A chain-collapsing model (one child for the object,
    // carrying the first parent's chain) disagrees.
    let chain_one = vec![("material".to_string(), PaintValue::ToolIndex(1))];
    let chain_two = vec![("density".to_string(), PaintValue::Flag(true))];
    let parent_one = control_region(
        object_a,
        1,
        vec![unit_rect(0, 0, 10, 5).to_expolygon()],
        Vec::new(),
        chain_one.clone(),
    );
    let parent_two = control_region(
        object_a,
        2,
        vec![unit_rect(0, 5, 10, 10).to_expolygon()],
        Vec::new(),
        chain_two.clone(),
    );
    let mut slice = slice_of(vec![parent_one.clone(), parent_two.clone()]);
    split_regions_by_modifier_footprints(&mut slice, [(object_a.to_string(), footprint.clone())])
        .expect("chain-specific parents split independently");

    let child_one = find_child_with_chain(&slice, 1, object_a, &footprint, &chain_one)
        .expect("parent 1 keeps its chain");
    let child_two = find_child_with_chain(&slice, 2, object_a, &footprint, &chain_two)
        .expect("parent 2 keeps its chain");
    assert_eq!(
        modifier_base_region_id(child_one.region_id),
        Some(1),
        "chain-specific child 1 decodes its own parent"
    );
    assert_eq!(
        modifier_base_region_id(child_two.region_id),
        Some(2),
        "chain-specific child 2 decodes its own parent"
    );
    let actual_chains = vec![
        child_one.variant_chain.clone(),
        child_two.variant_chain.clone(),
    ];
    let collapsed_model = vec![chain_one.clone()];
    assert_ne!(
        actual_chains, collapsed_model,
        "a chain-collapsing model must disagree with the real per-parent chains"
    );
    let children = slice
        .regions
        .iter()
        .filter(|region| is_modifier_namespace_id(region.region_id))
        .count();
    assert_eq!(
        children, 2,
        "one child per chain-specific parent, not one merged child"
    );

    // A chain-specific parent is also untouched by a footprint for another
    // object.
    let mut slice = slice_of(vec![parent_one.clone(), parent_two.clone()]);
    split_regions_by_modifier_footprints(&mut slice, [(object_b.to_string(), footprint)])
        .expect("object-B footprint is valid on a slice without object-B parents");
    assert_eq!(
        slice,
        slice_of(vec![parent_one, parent_two]),
        "a footprint for another object must not touch chain-specific parents"
    );
}

fn omitted_raft_and_internal_carriers_disagree() {
    const OBJECT: &str = "modifier-control-carriers";
    let parent_polygon = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let raft = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    let internal = vec![unit_rect(0, 8, 10, 10).to_expolygon()];
    let footprint = vec![unit_rect(2, 0, 8, 10).to_expolygon()];

    let mut parent = control_region(OBJECT, 0, parent_polygon.clone(), Vec::new(), Vec::new());
    parent.raft_fill = raft.clone();
    parent.internal_bridge_areas = internal.clone();
    parent.internal_bridge_angles_deg = vec![45.0];
    parent.bridge_orientation_deg = 73.0;
    let mut slice = slice_of(vec![parent]);
    split_regions_by_modifier_footprints(&mut slice, [(OBJECT.to_string(), footprint.clone())])
        .expect("carrier witness must split");

    let observed_parent = slice
        .regions
        .iter()
        .find(|region| region.region_id == 0)
        .expect("base region remains");
    let child = find_child(&slice, 0, OBJECT, &footprint).expect("footprint minted a child");
    let grid = control_grid(
        &[&parent_polygon, &raft, &internal, &footprint],
        &[
            &observed_parent.raft_fill,
            &observed_parent.internal_bridge_areas,
            &child.raft_fill,
            &child.internal_bridge_areas,
        ],
    );
    let footprint_cells = control_cells(&grid, &footprint);

    let raft_cells = control_cells(&grid, &raft);
    let expected_child_raft = raft_cells.and(&footprint_cells);
    let expected_base_raft = raft_cells.sub(&footprint_cells);
    assert!(
        !expected_child_raft.is_empty(),
        "the footprint must cover part of the raft carrier"
    );
    compare_region_to_cells(&child.raft_fill, &expected_child_raft, &grid, "child raft")
        .expect("raft carrier is split, not omitted");
    compare_region_to_cells(
        &observed_parent.raft_fill,
        &expected_base_raft,
        &grid,
        "base raft",
    )
    .expect("base raft keeps the remainder");
    assert_ne!(
        expected_child_raft,
        CellSet::empty(grid.cell_count()),
        "a model that ignores raft_fill must disagree on this witness"
    );

    let internal_cells = control_cells(&grid, &internal);
    let expected_child_internal = internal_cells.and(&footprint_cells);
    let expected_base_internal = internal_cells.sub(&footprint_cells);
    assert!(
        !expected_child_internal.is_empty(),
        "the footprint must cover part of the internal-bridge carrier"
    );
    compare_region_to_cells(
        &child.internal_bridge_areas,
        &expected_child_internal,
        &grid,
        "child internal bridge",
    )
    .expect("internal-bridge carrier is split, not omitted");
    compare_region_to_cells(
        &observed_parent.internal_bridge_areas,
        &expected_base_internal,
        &grid,
        "base internal bridge",
    )
    .expect("base internal bridge keeps the remainder");
    assert_ne!(
        expected_child_internal,
        CellSet::empty(grid.cell_count()),
        "a model that ignores internal_bridge_areas must disagree on this witness"
    );
}

fn lost_internal_bridge_angle_association_disagrees() {
    const OBJECT: &str = "modifier-control-angles";
    let parent_polygon = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let lower = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let upper = vec![unit_rect(0, 6, 10, 10).to_expolygon()];
    let footprint = vec![unit_rect(3, 0, 7, 10).to_expolygon()];

    let mut parent = control_region(OBJECT, 0, parent_polygon.clone(), Vec::new(), Vec::new());
    parent.internal_bridge_areas = vec![lower[0].clone(), upper[0].clone()];
    parent.internal_bridge_angles_deg = vec![30.0, 120.0];
    parent.bridge_orientation_deg = 73.0;
    let mut slice = slice_of(vec![parent]);
    split_regions_by_modifier_footprints(&mut slice, [(OBJECT.to_string(), footprint.clone())])
        .expect("two-angle witness must split");

    let child = find_child(&slice, 0, OBJECT, &footprint).expect("footprint minted a child");
    assert_eq!(
        child.internal_bridge_areas.len(),
        2,
        "one piece from each original bridge polygon"
    );
    assert_eq!(
        child.internal_bridge_angles_deg,
        vec![30.0, 120.0],
        "each piece keeps its original polygon's bridge angle"
    );
    let grid = control_grid(
        &[&parent_polygon, &lower, &upper, &footprint],
        &[&child.internal_bridge_areas],
    );
    let expected_30 = control_cells(&grid, &lower).and(&control_cells(&grid, &footprint));
    let expected_120 = control_cells(&grid, &upper).and(&control_cells(&grid, &footprint));
    compare_region_to_cells(
        std::slice::from_ref(&child.internal_bridge_areas[0]),
        &expected_30,
        &grid,
        "piece carrying 30 degrees",
    )
    .expect("the lower bridge keeps its 30-degree association");
    compare_region_to_cells(
        std::slice::from_ref(&child.internal_bridge_areas[1]),
        &expected_120,
        &grid,
        "piece carrying 120 degrees",
    )
    .expect("the upper bridge keeps its 120-degree association");
    assert_ne!(
        expected_30, expected_120,
        "the witness must carry two geometrically distinct angle groups"
    );

    // Known-wrong model: union both carriers into one polygon set and declare a
    // single angle for the group.
    let union_cells = expected_30.or(&expected_120);
    let collapsed_angles = vec![30.0_f32; child.internal_bridge_areas.len()];
    assert_ne!(
        child.internal_bridge_angles_deg, collapsed_angles,
        "a unioned, single-angle model must disagree with the real per-piece angles"
    );
    assert_ne!(
        union_cells, expected_30,
        "assigning the union to one angle disagrees with the real 30-degree cells"
    );
}

fn prepass_priority_and_document_order_decide_ownership() {
    const OBJECT: &str = "modifier-control-prepass";
    let base_units = Point2::from_mm(20.0, 0.0).x;
    let base_polygon = vec![unit_rect(0, 0, base_units, base_units).to_expolygon()];
    let base = control_region(OBJECT, 0, base_polygon.clone(), Vec::new(), Vec::new());

    let big_mesh = modifier_box_mesh(1.0, 1.0, 17.0, 17.0);
    let high_mesh = modifier_box_mesh(2.0, 2.0, 12.0, 12.0);
    let equal_mesh = modifier_box_mesh(6.0, 6.0, 16.0, 16.0);
    let enforcer_mesh = modifier_box_mesh(3.0, 13.0, 5.0, 15.0);
    let blocker_mesh = modifier_box_mesh(13.0, 3.0, 15.0, 5.0);
    let mesh = MeshIR {
        objects: vec![ObjectMesh {
            id: OBJECT.to_string(),
            modifier_volumes: vec![
                parameter_modifier("low-priority", 1, big_mesh.clone()),
                parameter_modifier("high-priority", 5, high_mesh.clone()),
                parameter_modifier("equal-priority-later", 5, equal_mesh.clone()),
                support_modifier(
                    "enforcer",
                    ModifierKind::SupportEnforcer,
                    enforcer_mesh.clone(),
                ),
                support_modifier(
                    "blocker",
                    ModifierKind::SupportBlocker,
                    blocker_mesh.clone(),
                ),
            ],
            ..ObjectMesh::default()
        }],
        ..MeshIR::default()
    };
    let mut slice = slice_at(vec![base], INTERIOR_PLANE_MM);
    split_modifier_sub_regions_for_prepass(&mut slice, &mesh)
        .expect("valid prepass modifier scene must split");

    let big = box_footprint(&big_mesh);
    let high = box_footprint(&high_mesh);
    let equal = box_footprint(&equal_mesh);
    let enforcer = box_footprint(&enforcer_mesh);
    let blocker = box_footprint(&blocker_mesh);
    for (name, footprint) in [
        ("big", &big),
        ("high", &high),
        ("equal", &equal),
        ("enforcer", &enforcer),
        ("blocker", &blocker),
    ] {
        assert!(
            !footprint.is_empty(),
            "{name} modifier must produce an interior cross-section"
        );
    }

    let child_high = find_child(&slice, 0, OBJECT, &high).expect("high-priority child");
    let child_equal = find_child(&slice, 0, OBJECT, &equal).expect("equal-priority child");
    let child_big = find_child(&slice, 0, OBJECT, &big).expect("low-priority child");
    assert_eq!(
        slice
            .regions
            .iter()
            .filter(|region| is_modifier_namespace_id(region.region_id))
            .count(),
        3,
        "support enforcer/blocker volumes must not create parameter children"
    );
    for (name, footprint) in [("enforcer", &enforcer), ("blocker", &blocker)] {
        let would_be = modifier_sub_region_id(0, OBJECT, footprint);
        assert!(
            slice
                .regions
                .iter()
                .all(|region| region.region_id != would_be),
            "support {name} volume must not mint the parameter child it would own"
        );
    }

    let observed_parent = slice
        .regions
        .iter()
        .find(|region| region.region_id == 0)
        .expect("base region remains");
    let grid = control_grid(
        &[&base_polygon, &big, &high, &equal],
        &[
            &child_high.polygons,
            &child_equal.polygons,
            &child_big.polygons,
            &observed_parent.polygons,
        ],
    );
    let parent_cells = control_cells(&grid, &base_polygon);
    let high_cells = control_cells(&grid, &high);
    let equal_cells = control_cells(&grid, &equal);
    let big_cells = control_cells(&grid, &big);
    let expected_high = parent_cells.and(&high_cells);
    let expected_equal = parent_cells.and(&equal_cells).sub(&high_cells);
    let expected_big = parent_cells
        .and(&big_cells)
        .sub(&high_cells)
        .sub(&equal_cells);
    assert!(
        !expected_equal.is_empty(),
        "the equal-priority modifiers must overlap for document order to matter"
    );
    assert!(
        !expected_big.is_empty(),
        "the low-priority modifier must retain cells outside the higher-priority pair"
    );

    compare_region_to_cells(
        &child_high.polygons,
        &expected_high,
        &grid,
        "prepass high-priority child",
    )
    .expect("higher priority owns its whole footprint first");
    compare_region_to_cells(
        &child_equal.polygons,
        &expected_equal,
        &grid,
        "prepass equal-priority later child",
    )
    .expect("equal priority is decided by document order");
    compare_region_to_cells(
        &child_big.polygons,
        &expected_big,
        &grid,
        "prepass low-priority child",
    )
    .expect("priority descending order owns before document order");

    // Known-wrong model: document order alone. The first document volume (the
    // low-priority one) would own its whole footprint.
    let document_order_model = parent_cells.and(&big_cells);
    let actual_big = control_cells(&grid, &child_big.polygons);
    let theft = document_order_model.sub(&actual_big);
    assert!(
        !theft.is_empty(),
        "document-order-only ownership must disagree with priority-descending split"
    );
    assert!(
        theft.is_subset_of(&high_cells.or(&equal_cells)),
        "every cell the document-order model steals lies inside the higher-priority pair"
    );
}

fn modifier_box_mesh(x0: f32, y0: f32, x1: f32, y1: f32) -> IndexedTriangleSet {
    let vertex = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            vertex(x0, y0, 0.0),
            vertex(x1, y0, 0.0),
            vertex(x1, y1, 0.0),
            vertex(x0, y1, 0.0),
            vertex(x0, y0, 1.0),
            vertex(x1, y0, 1.0),
            vertex(x1, y1, 1.0),
            vertex(x0, y1, 1.0),
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    }
}

fn box_footprint(mesh: &IndexedTriangleSet) -> Vec<ExPolygon> {
    slicer_core::slice_mesh_ex(mesh, &[INTERIOR_PLANE_MM])
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn parameter_modifier(id: &str, priority: u32, mesh: IndexedTriangleSet) -> ModifierVolume {
    ModifierVolume::new(
        id.to_string(),
        mesh,
        slicer_ir::ConfigDelta::default(),
        priority,
        ModifierKind::ParameterModifier,
    )
}

fn support_modifier(id: &str, kind: ModifierKind, mesh: IndexedTriangleSet) -> ModifierVolume {
    ModifierVolume::new(
        id.to_string(),
        mesh,
        slicer_ir::ConfigDelta::default(),
        9,
        kind,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-N1 identity/transaction controls.
// ─────────────────────────────────────────────────────────────────────────────

/// AC-N1: actual collision/unsupported-parent failures after earlier possible
/// work, through the private kernel, raw staging wrapper, public prepass seam and
/// the arena hook, with full `SliceIR` equality after every error.
pub fn runtime_modifier_identity_and_transaction_rejections() {
    private_kernel_collision_is_transactional();
    same_numeric_id_different_full_identity_is_not_a_collision();
    unsupported_parent_id_is_rejected_transactionally();
    later_failure_after_earlier_work_restores_input();
    raw_wrapper_consumes_sentinel_on_success_and_preserves_on_rejection();
    raw_wrapper_unsupported_parent_preserves_staging();
    public_prepass_rejection_leaves_slice_unchanged();
    hook_rejection_restores_entry_slice_perimeter_and_prepared_views();
    hook_unsupported_parent_restores_entry_state();
}

fn private_kernel_collision_is_transactional() {
    const OBJECT: &str = "modifier-reject-collision";
    let parent_polygon = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let mut parent = control_region(OBJECT, 0, parent_polygon, Vec::new(), Vec::new());
    parent.bridge_areas = vec![unit_rect(1, 1, 9, 9).to_expolygon()];
    parent.top_solid_fill = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let collide_id = modifier_sub_region_id(0, OBJECT, &footprint);
    let existing = control_region(
        OBJECT,
        collide_id,
        footprint.clone(),
        Vec::new(),
        Vec::new(),
    );
    let mut slice = slice_of(vec![parent, existing]);
    let before = slice.clone();

    let error = split_regions_by_modifier_footprints(&mut slice, [(OBJECT.to_string(), footprint)])
        .expect_err("an identical full child identity must collide");
    assert!(
        error.contains("identity collision"),
        "error must name the collision, got {error}"
    );
    assert!(
        error.contains(OBJECT),
        "collision error must name the object identity, got {error}"
    );
    assert_eq!(
        slice, before,
        "the private kernel must leave the input SliceIR untouched after a collision"
    );
}

fn same_numeric_id_different_full_identity_is_not_a_collision() {
    const OBJECT_A: &str = "modifier-reject-identity-a";
    const OBJECT_B: &str = "modifier-reject-identity-b";
    let parent_polygon = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let collide_id = modifier_sub_region_id(0, OBJECT_A, &footprint);
    let other_chain = vec![("density".to_string(), PaintValue::Flag(true))];

    let parent = control_region(OBJECT_A, 0, parent_polygon.clone(), Vec::new(), Vec::new());
    let other_object_child = control_region(
        OBJECT_B,
        collide_id,
        footprint.clone(),
        Vec::new(),
        Vec::new(),
    );
    let other_chain_child = control_region(
        OBJECT_A,
        collide_id,
        footprint.clone(),
        Vec::new(),
        other_chain.clone(),
    );
    let mut slice = slice_of(vec![
        parent,
        other_object_child.clone(),
        other_chain_child.clone(),
    ]);
    split_regions_by_modifier_footprints(&mut slice, [(OBJECT_A.to_string(), footprint.clone())])
        .expect("an equal numeric id on a different full identity is not a collision");

    let minted = find_child_with_chain(&slice, 0, OBJECT_A, &footprint, &[])
        .expect("the parent's own empty-chain child is minted");
    assert_eq!(
        minted.region_id, collide_id,
        "fixture identity: the minted child reuses the numeric id the other-object \
         and other-chain children carry, so only the full identity distinguishes them"
    );
    assert_eq!(
        minted.object_id, OBJECT_A,
        "minted child belongs to the split parent's object"
    );
    assert_eq!(
        minted.variant_chain,
        Vec::<(String, PaintValue)>::new(),
        "minted child copies the parent's empty chain"
    );
    let observed_other_object = slice
        .regions
        .iter()
        .find(|region| same_identity(region, &other_object_child))
        .expect("same-numeric-id child on another object survives");
    assert_eq!(observed_other_object, &other_object_child);
    let observed_other_chain = slice
        .regions
        .iter()
        .find(|region| same_identity(region, &other_chain_child))
        .expect("same-numeric-id child on another chain survives");
    assert_eq!(observed_other_chain, &other_chain_child);
    assert_eq!(
        slice.regions.len(),
        4,
        "parent, two differently-identified children and the newly minted child"
    );

    let grid = control_grid(
        &[&parent_polygon, &footprint],
        &[&minted.polygons, &observed_other_object.polygons],
    );
    let expected = control_cells(&grid, &parent_polygon).and(&control_cells(&grid, &footprint));
    compare_region_to_cells(&minted.polygons, &expected, &grid, "minted child")
        .expect("the non-colliding child still owns the footprint intersection");
}

fn unsupported_parent_id_is_rejected_transactionally() {
    const OBJECT: &str = "modifier-reject-parent-id";
    let stride = MODIFIER_VARIANT_REGION_ID_STRIDE;
    let max_parent = ((1_u64 << 63) - 2 - (stride - 1)) / stride;
    let invalid_parent = max_parent + 1;
    assert!(
        !modifier_sub_region_id_fits(invalid_parent),
        "fixture must use an unencodable parent id"
    );
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let parent = control_region(
        OBJECT,
        invalid_parent,
        vec![unit_rect(0, 0, 10, 10).to_expolygon()],
        Vec::new(),
        Vec::new(),
    );
    let mut slice = slice_of(vec![parent]);
    let before = slice.clone();

    let error = split_regions_by_modifier_footprints(&mut slice, [(OBJECT.to_string(), footprint)])
        .expect_err("an unencodable intersecting parent must be rejected");
    assert!(
        error.contains("parent_region_id"),
        "error must name the unencodable parent, got {error}"
    );
    assert_eq!(
        slice, before,
        "the private kernel must leave the input SliceIR untouched after a parent-id rejection"
    );
}

fn later_failure_after_earlier_work_restores_input() {
    const OBJECT: &str = "modifier-reject-later";
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let first_parent = control_region(
        OBJECT,
        0,
        vec![unit_rect(0, 0, 10, 10).to_expolygon()],
        Vec::new(),
        Vec::new(),
    );
    let second_parent = control_region(
        OBJECT,
        1,
        vec![unit_rect(0, 0, 10, 10).to_expolygon()],
        Vec::new(),
        Vec::new(),
    );
    let collide_id = modifier_sub_region_id(1, OBJECT, &footprint);
    let existing = control_region(
        OBJECT,
        collide_id,
        footprint.clone(),
        Vec::new(),
        Vec::new(),
    );
    let mut slice = slice_of(vec![first_parent, second_parent, existing]);
    let before = slice.clone();

    let error = split_regions_by_modifier_footprints(&mut slice, [(OBJECT.to_string(), footprint)])
        .expect_err("the second parent's prospective child must collide");
    assert!(
        error.contains("identity collision"),
        "error must name the collision, got {error}"
    );
    assert!(
        error.contains(&collide_id.to_string()),
        "error must name the second parent's prospective region id, got {error}"
    );
    assert_eq!(
        slice, before,
        "a failure after the first parent was split must restore the whole input, \
         proving the transactional clone rather than a pre-check"
    );
}

fn raw_wrapper_consumes_sentinel_on_success_and_preserves_on_rejection() {
    const OBJECT: &str = "modifier-reject-raw";
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let parent = control_region(
        OBJECT,
        0,
        vec![unit_rect(0, 0, 10, 10).to_expolygon()],
        Vec::new(),
        Vec::new(),
    );
    let raw = control_region(
        OBJECT,
        MODIFIER_FOOTPRINT_REGION_ID,
        footprint.clone(),
        footprint.clone(),
        Vec::new(),
    );
    let child_id = modifier_sub_region_id(0, OBJECT, &footprint);

    let mut slice = slice_of(vec![parent.clone(), raw.clone()]);
    split_modifier_footprints(&mut slice).expect("the raw staging wrapper must split");
    assert!(
        slice
            .regions
            .iter()
            .all(|region| region.region_id != MODIFIER_FOOTPRINT_REGION_ID),
        "successful consumption removes the raw footprint sentinel"
    );
    assert!(
        slice
            .regions
            .iter()
            .any(|region| region.region_id == child_id),
        "successful consumption mints the parameter child"
    );
    assert_eq!(
        slice.regions.len(),
        2,
        "only the parent and the minted child remain after success"
    );

    let existing = control_region(OBJECT, child_id, footprint.clone(), Vec::new(), Vec::new());
    let mut raw_with_roles = raw;
    raw_with_roles.bridge_areas = vec![unit_rect(1, 1, 9, 9).to_expolygon()];
    raw_with_roles.top_solid_fill = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let mut rejected = slice_of(vec![parent, raw_with_roles, existing]);
    let before = rejected.clone();
    let error = split_modifier_footprints(&mut rejected)
        .expect_err("the raw staging wrapper must propagate the identity collision");
    assert!(
        error.contains("identity collision"),
        "error must name the collision, got {error}"
    );
    assert_eq!(
        rejected, before,
        "a rejected raw split must preserve the staging region, its roles and the sibling child"
    );
    assert!(
        rejected
            .regions
            .iter()
            .any(|region| region.region_id == MODIFIER_FOOTPRINT_REGION_ID),
        "the raw staging region must survive a rejection"
    );
}

/// The raw wrapper's other rejection class: an unencodable parent id. The
/// wrapper removes the staging region from its working copy before calling the
/// kernel, so only the transactional clone can leave the staging region and its
/// already-partitioned roles byte-identical.
fn raw_wrapper_unsupported_parent_preserves_staging() {
    const OBJECT: &str = "modifier-reject-raw-parent";
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let parent = control_region(
        OBJECT,
        unencodable_parent_id(),
        vec![unit_rect(0, 0, 10, 10).to_expolygon()],
        Vec::new(),
        Vec::new(),
    );
    let mut raw = control_region(
        OBJECT,
        MODIFIER_FOOTPRINT_REGION_ID,
        footprint.clone(),
        footprint,
        Vec::new(),
    );
    raw.bridge_areas = vec![unit_rect(1, 1, 9, 9).to_expolygon()];
    raw.top_solid_fill = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let staging = raw.clone();
    let mut slice = slice_of(vec![parent, raw]);
    let before = slice.clone();

    let error = split_modifier_footprints(&mut slice)
        .expect_err("the raw staging wrapper must propagate the unencodable parent");
    assert!(
        error.contains("parent_region_id"),
        "error must name the unencodable parent, got {error}"
    );
    assert_eq!(
        slice, before,
        "a rejected raw split must leave the input SliceIR untouched"
    );
    let observed = slice
        .regions
        .iter()
        .find(|region| region.region_id == MODIFIER_FOOTPRINT_REGION_ID)
        .expect("the raw staging region must survive an unencodable-parent rejection");
    assert_eq!(
        observed, &staging,
        "the raw staging region keeps its footprint and its already-partitioned roles"
    );
}

fn public_prepass_rejection_leaves_slice_unchanged() {
    const OBJECT: &str = "modifier-reject-prepass";
    // The colliding footprint must still reach the parent's *remainder* after
    // the first successful split, otherwise the kernel would skip it silently.
    let first_mesh = modifier_box_mesh(1.0, 1.0, 6.0, 9.0);
    let collide_mesh = modifier_box_mesh(7.0, 2.0, 12.0, 8.0);
    let first_footprint = box_footprint(&first_mesh);
    let collide_footprint = box_footprint(&collide_mesh);
    assert!(
        !first_footprint.is_empty(),
        "the first modifier must split successfully before the later collision"
    );
    assert!(
        !collide_footprint.is_empty(),
        "the colliding modifier must intersect the parent"
    );
    let collide_id = modifier_sub_region_id(0, OBJECT, &collide_footprint);
    let base_units = Point2::from_mm(20.0, 0.0).x;
    let parent = control_region(
        OBJECT,
        0,
        vec![unit_rect(0, 0, base_units, base_units).to_expolygon()],
        Vec::new(),
        Vec::new(),
    );
    let existing = control_region(
        OBJECT,
        collide_id,
        collide_footprint,
        Vec::new(),
        Vec::new(),
    );
    let mut slice = slice_at(vec![parent, existing], INTERIOR_PLANE_MM);
    let mesh = MeshIR {
        objects: vec![ObjectMesh {
            id: OBJECT.to_string(),
            modifier_volumes: vec![
                parameter_modifier("first", 5, first_mesh),
                parameter_modifier("colliding", 1, collide_mesh),
            ],
            ..ObjectMesh::default()
        }],
        ..MeshIR::default()
    };

    // The earlier split is real work, not a pre-check: the same seam accepts the
    // first modifier alone and mints its child, so the rejection below can only
    // come from the transactional clone.
    let mut accepted = slice.clone();
    let first_only = MeshIR {
        objects: vec![ObjectMesh {
            id: OBJECT.to_string(),
            modifier_volumes: vec![parameter_modifier(
                "first",
                5,
                modifier_box_mesh(1.0, 1.0, 6.0, 9.0),
            )],
            ..ObjectMesh::default()
        }],
        ..MeshIR::default()
    };
    split_modifier_sub_regions_for_prepass(&mut accepted, &first_only)
        .expect("the first modifier alone must split the parent");
    assert!(
        accepted
            .regions
            .iter()
            .any(|region| is_modifier_namespace_id(region.region_id)),
        "the earlier split must actually mint a child before the later collision"
    );

    let before = slice.clone();
    let error = split_modifier_sub_regions_for_prepass(&mut slice, &mesh)
        .expect_err("the public prepass seam must propagate the later collision");
    assert!(
        error.contains("identity collision"),
        "prepass error must name the collision, got {error}"
    );
    assert_eq!(
        slice, before,
        "a prepass rejection after an earlier successful split must leave the input unchanged"
    );
}

fn hook_rejection_restores_entry_slice_perimeter_and_prepared_views() {
    const OBJECT: &str = "modifier-reject-hook";
    let base_polygon = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let mut base = control_region(
        OBJECT,
        0,
        base_polygon.clone(),
        vec![unit_rect(1, 1, 9, 9).to_expolygon()],
        Vec::new(),
    );
    base.bridge_areas = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    base.top_solid_fill = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let raw = control_region(
        OBJECT,
        MODIFIER_FOOTPRINT_REGION_ID,
        footprint.clone(),
        footprint.clone(),
        Vec::new(),
    );
    let collide_id = modifier_sub_region_id(0, OBJECT, &footprint);
    let mut existing = control_region(
        OBJECT,
        collide_id,
        footprint.clone(),
        Vec::new(),
        Vec::new(),
    );
    existing.segment_annotations.insert(
        PaintSemantic::Material,
        vec![vec![Some(PaintValue::ToolIndex(2))]],
    );
    let entry = slice_of(vec![base, raw, existing]);
    let perimeter = PerimeterIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: OBJECT.to_string(),
            region_id: 0,
            infill_areas: vec![unit_rect(1, 1, 9, 9).to_expolygon()],
            ..PerimeterRegion::default()
        }],
    };
    let mut by_layer = HashMap::new();
    by_layer.insert(
        0u32,
        vec![QuartileBand {
            quartile: 1,
            polygons: base_polygon.clone(),
        }],
    );
    let mut overhang = HashMap::new();
    overhang.insert(OBJECT.to_string(), by_layer);
    let classification = SurfaceClassificationIR {
        overhang_quartile_polygons: overhang,
        ..SurfaceClassificationIR::default()
    };

    let mut arena = crate::LayerArena::new();
    arena.set_slice(entry.clone()).expect("stage entry slice");
    arena
        .set_perimeter(perimeter.clone())
        .expect("stage perimeter");
    let primed = arena
        .ensure_prepared_regions(Some(&classification))
        .expect("prepared views prime before the hook")
        .to_vec();
    assert_eq!(
        primed.len(),
        entry.regions.len(),
        "one prepared record per staged region"
    );

    let error = sync_perimeter_infill_areas_into_slice(&mut arena, 0)
        .expect_err("the hook must reject the colliding raw footprint");
    match &error {
        LayerStageError::FatalModule {
            stage_id,
            module_id,
            message,
        } => {
            assert_eq!(stage_id, "Layer::Perimeters");
            assert_eq!(module_id, "host:region_partition");
            assert!(
                message.contains("identity collision"),
                "hook error must carry the split message, got {message}"
            );
        }
        other => panic!("expected FatalModule, got {other:?}"),
    }
    assert_eq!(
        arena.slice(),
        Some(&entry),
        "the hook must re-stage the ENTRY-STATE slice (raw footprint and pre-sync roles included)"
    );
    assert_eq!(
        arena.perimeter(),
        Some(&perimeter),
        "a rejected modifier split must not touch the staged perimeter data"
    );
    assert!(
        arena.prepared_regions().is_none(),
        "taking the slice must invalidate derived prepared views"
    );
    let rederived = arena
        .ensure_prepared_regions(Some(&classification))
        .expect("prepared views re-derive from the restored slice")
        .to_vec();
    assert_eq!(
        rederived.len(),
        entry.regions.len(),
        "prepared records re-associate with the restored slice's regions"
    );
    assert_eq!(
        rederived.as_slice(),
        primed.as_slice(),
        "re-derived prepared data must match the entry-state derivation, never stale"
    );

    let prepared_footprint = &rederived[1].overhang_areas;
    let grid = control_grid(&[&base_polygon, &footprint], &[prepared_footprint]);
    let expected = control_cells(&grid, &base_polygon).and(&control_cells(&grid, &footprint));
    assert!(
        !expected.is_empty(),
        "the raw footprint region must intersect the classification footprint"
    );
    compare_region_to_cells(
        prepared_footprint,
        &expected,
        &grid,
        "prepared[1] raw footprint",
    )
    .expect("prepared record 1 must re-derive the staged raw footprint region, not another");
}

/// The hook's other rejection class: an unencodable parent id reached through a
/// raw staging footprint. The pre-split role reconciliation has already rewritten
/// the parent's bridge/top roles on the working copy by then, so this pins the
/// ENTRY-STATE restoration — slice (raw footprint region included) and
/// `PerimeterIR` — rather than a no-op.
fn hook_unsupported_parent_restores_entry_state() {
    const OBJECT: &str = "modifier-reject-hook-parent";
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let mut base = control_region(
        OBJECT,
        unencodable_parent_id(),
        vec![unit_rect(0, 0, 10, 10).to_expolygon()],
        vec![unit_rect(1, 1, 9, 9).to_expolygon()],
        Vec::new(),
    );
    // Roles the reconciliation *would* rewrite: the perimeter donor's inset
    // (1,1)-(9,9) clips them, so an entry-state restore is observable.
    base.bridge_areas = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    base.bottom_solid_fill = vec![unit_rect(0, 2, 10, 4).to_expolygon()];
    base.top_solid_fill = vec![unit_rect(0, 4, 10, 6).to_expolygon()];
    let raw = control_region(
        OBJECT,
        MODIFIER_FOOTPRINT_REGION_ID,
        footprint.clone(),
        footprint,
        Vec::new(),
    );
    let entry = slice_of(vec![base, raw]);
    let perimeter = PerimeterIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: OBJECT.to_string(),
            region_id: unencodable_parent_id(),
            infill_areas: vec![unit_rect(1, 1, 9, 9).to_expolygon()],
            ..PerimeterRegion::default()
        }],
    };

    let mut arena = crate::LayerArena::new();
    arena.set_slice(entry.clone()).expect("stage entry slice");
    arena
        .set_perimeter(perimeter.clone())
        .expect("stage perimeter");

    let error = sync_perimeter_infill_areas_into_slice(&mut arena, 0)
        .expect_err("the hook must reject the unencodable parent");
    match &error {
        LayerStageError::FatalModule {
            stage_id,
            module_id,
            message,
        } => {
            assert_eq!(stage_id, "Layer::Perimeters");
            assert_eq!(module_id, "host:region_partition");
            assert!(
                message.contains("parent_region_id"),
                "hook error must carry the split message, got {message}"
            );
        }
        other => panic!("expected FatalModule, got {other:?}"),
    }
    assert_eq!(
        arena.slice(),
        Some(&entry),
        "the hook must re-stage the ENTRY-STATE slice, raw footprint region included"
    );
    assert!(
        arena.slice().is_some_and(|slice| slice
            .regions
            .iter()
            .any(|region| region.region_id == MODIFIER_FOOTPRINT_REGION_ID)),
        "the raw footprint staging region must still be staged after the rejection"
    );
    assert_eq!(
        arena.perimeter(),
        Some(&perimeter),
        "a rejected modifier split must not touch the staged PerimeterIR"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Named concrete regression for the taken-slice loss.
// ─────────────────────────────────────────────────────────────────────────────

/// Named, literal regression for the arena hook's taken-slice loss.
///
/// `sync_perimeter_infill_areas_into_slice` takes the staged `SliceIR` out of
/// `LayerArena` and used to return the modifier-split error without re-staging
/// it, so a rejected split left `arena.slice() == None`. The input below is a
/// literal rejecting scene (raw footprint plus a pre-existing child with the
/// same full identity), needs no `PropertyRun` context and no RNG, and fails on
/// the pre-fix implementation at the slice-restoration assertion.
pub fn runtime_modifier_collision_restores_taken_slice() {
    const OBJECT: &str = "modifier-regression-object";
    let base_polygon = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let mut base = control_region(
        OBJECT,
        0,
        base_polygon,
        vec![unit_rect(1, 1, 9, 9).to_expolygon()],
        Vec::new(),
    );
    base.bridge_areas = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let raw = control_region(
        OBJECT,
        MODIFIER_FOOTPRINT_REGION_ID,
        footprint.clone(),
        footprint.clone(),
        Vec::new(),
    );
    let collide_id = modifier_sub_region_id(0, OBJECT, &footprint);
    let existing = control_region(OBJECT, collide_id, footprint, Vec::new(), Vec::new());
    let entry = slice_of(vec![base, raw, existing]);
    let perimeter = PerimeterIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: OBJECT.to_string(),
            region_id: 0,
            infill_areas: vec![unit_rect(1, 1, 9, 9).to_expolygon()],
            ..PerimeterRegion::default()
        }],
    };

    let mut arena = crate::LayerArena::new();
    arena.set_slice(entry.clone()).expect("stage entry slice");
    arena
        .set_perimeter(perimeter.clone())
        .expect("stage perimeter");

    let error = sync_perimeter_infill_areas_into_slice(&mut arena, 0)
        .expect_err("the literal collision must reject");
    assert!(
        matches!(
            &error,
            LayerStageError::FatalModule {
                stage_id,
                module_id,
                message,
            } if stage_id == "Layer::Perimeters"
                && module_id == "host:region_partition"
                && message.contains("identity collision")
        ),
        "expected the partition hook's FatalModule identity, got {error:?}"
    );
    assert!(
        arena.slice().is_some(),
        "regression: the hook took the staged SliceIR and returned the split error \
         without re-staging it, leaving the arena without a layer"
    );
    assert_eq!(
        arena.slice(),
        Some(&entry),
        "the rejected hook must restore the ENTRY-STATE slice, raw footprint and pre-sync roles included"
    );
    assert_eq!(
        arena.perimeter(),
        Some(&perimeter),
        "the rejected hook must leave the staged PerimeterIR unchanged"
    );
}
