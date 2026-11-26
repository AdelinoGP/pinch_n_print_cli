//! Charter `H-RUNTIME-PARTITION`: the real conditional fill precedence of
//! `region_partition::sync_perimeter_infill_areas_into_slice`.
//!
//! The property stages a real `SliceIR` plus a real `PerimeterIR` on a
//! `LayerArena`, calls the production hook, and compares every observed fill
//! role against an independent finite-cell model. The model is deliberately
//! not a wrapper around the SUT:
//!
//! * the donor join is a linear scan over the **full** source identity
//!   `(object_id, source region_id, exact ordered variant chain)`; a modifier
//!   child decodes only its parent region id and keeps its own object/chain;
//! * the effective wall inset is `W = donor.infill_areas ∩ region.polygons`,
//!   never the donor's whole geometry;
//! * a nonempty `W` labels every cell by the documented precedence
//!   `bridge > bottom > top > internal > sparse` and asserts an exact disjoint
//!   cover of `W` (`internal` is `internal_solid_fill`, the converted
//!   sparse-island bucket);
//! * an empty `W` is conditional: with donor walls all five roles are empty,
//!   without donor walls the bridge is preserved, the top is the original top
//!   minus that bridge, and bottom/internal/sparse are empty — no role is
//!   claimed to be a subset of the empty `W`;
//! * a region with no donor keeps its four exposed/sparse roles and every other
//!   carrier byte-identical, has only the shell-band marker carved from
//!   `internal_solid_fill` (`internal − top`), and emits the production warning
//!   naming object/region (and parent for a modifier child);
//! * carriers outside the five canonical roles (`infill_areas`, aligned
//!   internal-bridge fields, `raft_fill`, metadata) are byte-identical, because
//!   no raw modifier footprint is staged in the property;
//! * a modifier child whose bridge is already inside `W` must keep its bridge
//!   `Vec<ExPolygon>` byte-for-byte (the production source-preservation
//!   branch); every other bridge comparison is semantic.
//!
//! A committed `Blackboard` `Arc<Vec<SliceIR>>` is kept alongside the arena: a
//! successful arena-only partition must leave it pointer-identical and equal.
//! Prepared-region caches are primed before the call, asserted invalidated by
//! the `take_slice`, and re-derived afterwards so a stale cache cannot pass.
//!
//! The deterministic controls falsify swapped precedence, an unclipped
//! nonempty bridge, a pair-only or normalized-chain donor join over exact
//! reverse two-entry chains, converting a missing donor into a fatal error, and
//! asserting every role is a subset of an empty `W`. They use the actual hook
//! for every claim. The stage/domain controls pin the two `FatalModule`
//! identities, the entry-state slice restoration on a missing perimeter, and the
//! shared domain helpers' fail-closed bounds.
//!
//! The AC-N2 audit requires the constructed `NonemptyEffectiveInset` and
//! `RolePrecedenceOverlap` witnesses (slot 0's full-domain donor and overlapping
//! bridge/bottom/top carriers) in addition to the structural tags, asserts the
//! tag set is unchanged at every `simplify()` step, and records whether the
//! simplified tree reached a fixpoint or stopped at the 64-step audit cap.

#![allow(missing_docs)]

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_ir::{
    is_modifier_namespace_id, modifier_base_region_id, modifier_sub_region_id, ExPolygon,
    LayerStageError, LoopType, MeshIR, PaintSemantic, PaintValue, PerimeterIR, PerimeterRegion,
    PreparedRegionData, QuartileBand, SliceIR, SlicedRegion, SurfaceClassificationIR,
    WallBoundaryType, WallLoop, CURRENT_SLICE_IR_SCHEMA_VERSION, MODIFIER_FOOTPRINT_REGION_ID,
};
use slicer_sdk::test_support::fixtures::wall_loop_base;
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::cells::{
    assert_disjoint_union, checked_double, checked_max_abs_units, checked_max_cells,
    compare_region_to_cells, holed_rect, rect_field, CellGrid, CellSet, Rect, MAX_ABS_UNITS,
    MAX_CELLS, MIN_ABS_UNITS,
};
use crate::region_partition::sync_perimeter_infill_areas_into_slice;
use crate::{Blackboard, LayerArena};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-RUNTIME-PARTITION";
/// Exact registry test name (the inline libtest identity).
pub const TEST_NAME: &str = "region_partition::property_tests::proptest_runtime_fill_partition";

/// Written ceiling for generated slice regions (domain design, not a measured
/// runtime claim).
const MAX_REGIONS_CEILING: u64 = 4;
/// Lower safety bound for generated slice regions.
const MIN_REGIONS_CEILING: u64 = 2;

/// Shared lattice resolution. Every generated coordinate is a multiple of an
/// even step on this lattice, so inputs and Boolean outputs share at most
/// `(LATTICE + 1)^2` elementary cells regardless of how many carriers a case
/// carries.
const LATTICE: usize = 6;

const OBJECT_A: &str = "partition-property-object-a";
const OBJECT_B: &str = "partition-property-object-b";

// ─────────────────────────────────────────────────────────────────────────────
// Registry bounds.
// ─────────────────────────────────────────────────────────────────────────────

/// Checked generation bounds for `H-RUNTIME-PARTITION`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionLimits {
    /// Maximum generated slice regions.
    pub max_regions: usize,
    /// Absolute coordinate bound in canonical units.
    pub max_abs_units: i64,
    /// Elementary-cell budget for one comparison grid.
    pub max_cells: usize,
}

/// Validate every registry bound against the written safety ceiling before any
/// strategy allocates. Every value is checked in `u64` by the shared domain
/// helpers before narrowing, so an out-of-ceiling profile fails closed.
pub fn checked_partition_limits(
    max_regions: u64,
    max_abs_units: u64,
    max_cells: u64,
) -> Result<PartitionLimits, String> {
    if !(MIN_REGIONS_CEILING..=MAX_REGIONS_CEILING).contains(&max_regions) {
        return Err(format!(
            "max_regions must be in {MIN_REGIONS_CEILING}..={MAX_REGIONS_CEILING}, got {max_regions}"
        ));
    }
    let max_regions =
        usize::try_from(max_regions).map_err(|_| String::from("max_regions does not fit usize"))?;
    let max_abs_units = checked_max_abs_units(max_abs_units).map_err(|error| error.to_string())?;
    let max_cells = checked_max_cells(max_cells).map_err(|error| error.to_string())?;
    Ok(PartitionLimits {
        max_regions,
        max_abs_units,
        max_cells,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Log capture.
//
// The production missing-donor path uses the `log` facade, so the property and
// the controls install one capture logger for this test binary and drain it
// around every call. `CAPTURE_LOCK` serialises call+drain windows across the
// libtest threads so one test can never attribute another test's warning.
// ─────────────────────────────────────────────────────────────────────────────

static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

struct PartitionCaptureLogger;

impl log::Log for PartitionCaptureLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        CAPTURED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(format!("{}|{}", record.level(), record.args()));
    }

    fn flush(&self) {}
}

static PARTITION_LOGGER: PartitionCaptureLogger = PartitionCaptureLogger;
static LOGGER_INSTALLED: OnceLock<()> = OnceLock::new();

fn capture_guard() -> MutexGuard<'static, ()> {
    let guard = CAPTURE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    LOGGER_INSTALLED.get_or_init(|| {
        log::set_logger(&PARTITION_LOGGER).expect("install the partition capture logger once");
        log::set_max_level(log::LevelFilter::Trace);
    });
    guard
}

fn drain_captured() -> Vec<String> {
    std::mem::take(
        &mut *CAPTURED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    )
}

/// Run the real hook with the capture window held open.
fn run_sync(
    arena: &mut LayerArena,
    layer_index: u32,
) -> (Result<(), LayerStageError>, Vec<String>) {
    let _guard = capture_guard();
    let _ = drain_captured();
    let result = sync_perimeter_infill_areas_into_slice(arena, layer_index);
    let warnings = drain_captured();
    (result, warnings)
}

// ─────────────────────────────────────────────────────────────────────────────
// Generated domain.
// ─────────────────────────────────────────────────────────────────────────────

/// One generated partition scene: committed slice regions plus perimeter donors.
#[derive(Debug, Clone)]
pub struct PartitionScene {
    /// Slice regions in commit order.
    pub regions: Vec<SlicedRegion>,
    /// Perimeter donors; each full identity is unique.
    pub donors: Vec<PerimeterRegion>,
}

/// Orthogonal lattice rectangle in `0..=LATTICE` coordinates.
#[derive(Debug, Clone, Copy)]
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
        .expect("lattice rectangle is ordered and positive by construction")
    }

    fn expolygon(self, step: i64) -> ExPolygon {
        self.rect(step).to_expolygon()
    }

    fn intersect(self, other: Self) -> Option<Self> {
        let c0 = self.c0.max(other.c0);
        let r0 = self.r0.max(other.r0);
        let c1 = self.c1.min(other.c1);
        let r1 = self.r1.min(other.r1);
        (c0 < c1 && r0 < r1).then_some(Self { c0, r0, c1, r1 })
    }

    fn width(self) -> usize {
        self.c1 - self.c0
    }

    fn height(self) -> usize {
        self.r1 - self.r0
    }
}

/// Even lattice step whose farthest coordinate stays strictly below the
/// accepted coordinate bound.
fn lattice_step(max_abs_units: i64) -> i64 {
    (((max_abs_units - 1) / (2 * LATTICE as i64)) * 2).max(2)
}

fn chain_for(variant: u32) -> Vec<(String, PaintValue)> {
    match variant {
        0 => Vec::new(),
        1 => vec![("material".to_string(), PaintValue::ToolIndex(1))],
        2 => vec![("density".to_string(), PaintValue::Flag(true))],
        _ => vec![("seam".to_string(), PaintValue::Scalar(0.5))],
    }
}

/// The two-entry ordered chain the partition chain-exactness witnesses use.
///
/// The generated sibling pair carries this chain and its reverse. A join keyed
/// only by `(object, region)` or by a sorted/normalized chain cannot tell them
/// apart, so the exact ordered join is the only model the property can satisfy.
fn chain_forward() -> Vec<(String, PaintValue)> {
    vec![
        ("density".to_string(), PaintValue::ToolIndex(1)),
        ("material".to_string(), PaintValue::ToolIndex(2)),
    ]
}

/// The exact reverse of `chain_forward`: same entries, opposite order.
fn chain_reversed() -> Vec<(String, PaintValue)> {
    let mut chain = chain_forward();
    chain.reverse();
    chain
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

/// A nonempty sub-rectangle of `outer`.
fn sub_rect(outer: LatticeRect, seeds: &[u32], offset: usize) -> LatticeRect {
    let at = |index: usize| seeds[(offset + index) % seeds.len()] as usize;
    let c0 = outer.c0 + at(0) % outer.width();
    let c1 = c0 + 1 + at(1) % (outer.c1 - c0);
    let r0 = outer.r0 + at(2) % outer.height();
    let r1 = r0 + 1 + at(3) % (outer.r1 - r0);
    LatticeRect { c0, r0, c1, r1 }
}

/// One disjoint rectangle as an `ExPolygon` field.
fn single_rect_field(rect: LatticeRect, step: i64) -> Vec<ExPolygon> {
    rect_field(std::slice::from_ref(&rect.rect(step)))
        .expect("a single rectangle is a disjoint carrier field")
}

/// A role carrier: empty, or one sub-rectangle of `domain` that is optionally
/// widened by one lattice column (the "bridge extends past the wall inset"
/// input case).
fn role_polygons(
    domain: LatticeRect,
    seeds: &[u32],
    offset: usize,
    empty: bool,
    widen: bool,
    step: i64,
) -> Vec<ExPolygon> {
    if empty {
        return Vec::new();
    }
    let mut rect = sub_rect(domain, seeds, offset);
    if widen {
        let grow_right = seeds[(offset + 4) % seeds.len()].is_multiple_of(2);
        rect = if grow_right {
            LatticeRect {
                c1: (rect.c1 + 1).min(LATTICE),
                ..rect
            }
        } else {
            LatticeRect {
                c0: rect.c0.saturating_sub(1),
                ..rect
            }
        };
    }
    single_rect_field(rect, step)
}

/// Region outline: plain rectangle, holed rectangle, concave orthogonal union,
/// or separated components.
fn shape_polygons(kind: u32, primary: LatticeRect, step: i64) -> Vec<ExPolygon> {
    match kind % 4 {
        1 if primary.width() >= 3 && primary.height() >= 3 => {
            let hole = LatticeRect {
                c0: primary.c0 + 1,
                r0: primary.r0 + 1,
                c1: primary.c1 - 1,
                r1: primary.r1 - 1,
            };
            vec![holed_rect(primary.rect(step), &[hole.rect(step)])
                .expect("the inset hole lies strictly inside the outer rectangle")]
        }
        2 => {
            let mid = primary.c0 + (primary.width() / 2).max(1);
            vec![
                LatticeRect { c1: mid, ..primary }.expolygon(step),
                LatticeRect { c0: mid, ..primary }.expolygon(step),
            ]
        }
        3 if primary.height() >= 3 => {
            vec![
                LatticeRect {
                    r1: primary.r0 + 1,
                    ..primary
                }
                .expolygon(step),
                LatticeRect {
                    r0: primary.r1 - 1,
                    ..primary
                }
                .expolygon(step),
            ]
        }
        _ => vec![primary.expolygon(step)],
    }
}

/// Internal-bridge carriers: one or two abutting pieces with either an aligned
/// angle vector or the empty fallback vector.
fn internal_bridge_field(
    domain: LatticeRect,
    seeds: &[u32],
    offset: usize,
    two_pieces: bool,
    aligned: bool,
    step: i64,
) -> (Vec<ExPolygon>, Vec<f32>) {
    let rect = sub_rect(domain, seeds, offset);
    let rects = if two_pieces && rect.height() >= 2 {
        let mid = rect.r0 + 1;
        vec![
            LatticeRect { r1: mid, ..rect },
            LatticeRect { r0: mid, ..rect },
        ]
    } else {
        vec![rect]
    };
    let bounds: Vec<Rect> = rects.iter().map(|rect| rect.rect(step)).collect();
    let areas = rect_field(&bounds).expect("internal-bridge pieces share at most an edge");
    let angles = if aligned {
        rects
            .iter()
            .enumerate()
            .map(|(position, _)| if position == 0 { 30.0 } else { 120.0 })
            .collect()
    } else {
        Vec::new()
    };
    (areas, angles)
}

fn role_bit(mode: u32, index: u32) -> bool {
    (mode >> (16 + index % 16)) & 1 == 1
}

/// Build one slice region with all nine geometry carriers.
///
/// `certifying` selects the deterministic audit witness: bridge is the lower
/// band, bottom is the left columns and top covers the whole domain, so the
/// three precedence carriers overlap on interior cells and a full-domain donor
/// gives a nonempty effective inset regardless of the shrinkable seed draws.
fn region_with_roles(
    object_id: &str,
    region_id: u64,
    chain: Vec<(String, PaintValue)>,
    polygons: Vec<ExPolygon>,
    domain: LatticeRect,
    seeds: &[u32],
    seed_offset: usize,
    widen_bridge: bool,
    certifying: bool,
    mode: u32,
    region_index: u32,
    step: i64,
) -> SlicedRegion {
    let mut region = SlicedRegion {
        object_id: object_id.to_string(),
        region_id,
        polygons,
        variant_chain: chain,
        effective_layer_height: 0.2,
        top_shell_index: Some(0),
        bottom_shell_index: Some(0),
        bridge_orientation_deg: 73.0,
        segment_annotations: paint_annotations(),
        ..SlicedRegion::default()
    };
    if certifying {
        // Deterministic precedence witness: bridge (rows 0..2) and bottom (left
        // columns, full height) share interior cells, and top is the whole
        // domain, so role precedence is exercised on a nonempty wall inset.
        let lower = LatticeRect {
            c0: domain.c0,
            r0: domain.r0,
            c1: domain.c1,
            r1: domain.r0 + 2,
        };
        let left = LatticeRect {
            c0: domain.c0,
            r0: domain.r0,
            c1: domain.c0 + (domain.width() / 2).max(1),
            r1: domain.r1,
        };
        region.bridge_areas = single_rect_field(lower, step);
        region.bottom_solid_fill = single_rect_field(left, step);
        region.top_solid_fill = single_rect_field(domain, step);
        region.sparse_infill_area = role_polygons(
            domain,
            seeds,
            seed_offset + 15,
            role_bit(mode, region_index * 4 + 3),
            false,
            step,
        );
    } else {
        region.bridge_areas = role_polygons(
            domain,
            seeds,
            seed_offset,
            role_bit(mode, region_index * 4),
            widen_bridge,
            step,
        );
        region.bottom_solid_fill = role_polygons(
            domain,
            seeds,
            seed_offset + 5,
            role_bit(mode, region_index * 4 + 1),
            false,
            step,
        );
        region.top_solid_fill = role_polygons(
            domain,
            seeds,
            seed_offset + 10,
            role_bit(mode, region_index * 4 + 2),
            false,
            step,
        );
        region.sparse_infill_area = role_polygons(
            domain,
            seeds,
            seed_offset + 15,
            role_bit(mode, region_index * 4 + 3),
            false,
            step,
        );
    }
    region.infill_areas = role_polygons(domain, seeds, seed_offset + 20, false, false, step);
    region.internal_solid_fill = role_polygons(domain, seeds, seed_offset + 25, false, false, step);
    region.raft_fill = role_polygons(domain, seeds, seed_offset + 30, false, false, step);
    let (areas, angles) = internal_bridge_field(
        domain,
        seeds,
        seed_offset + 35,
        (mode >> 14) & 1 == 1,
        (mode >> 15) & 1 == 1,
        step,
    );
    region.internal_bridge_areas = areas;
    region.internal_bridge_angles_deg = angles;
    region
}

struct DonorPlan {
    infill: Vec<ExPolygon>,
    infill_rect: Option<LatticeRect>,
}

/// Donor infill geometry: a sub-rectangle of the domain (nonempty `W`), empty,
/// strictly disjoint (empty `W`), or the full domain.
fn donor_plan(
    mode_bits: u32,
    domain: LatticeRect,
    disjoint: LatticeRect,
    seeds: &[u32],
    offset: usize,
    step: i64,
) -> DonorPlan {
    match mode_bits % 4 {
        0 => {
            let rect = sub_rect(domain, seeds, offset);
            DonorPlan {
                infill: single_rect_field(rect, step),
                infill_rect: Some(rect),
            }
        }
        1 => DonorPlan {
            infill: Vec::new(),
            infill_rect: None,
        },
        2 => DonorPlan {
            infill: single_rect_field(disjoint, step),
            infill_rect: Some(disjoint),
        },
        _ => DonorPlan {
            infill: vec![domain.expolygon(step)],
            infill_rect: Some(domain),
        },
    }
}

fn walls_for(present: bool) -> Vec<WallLoop> {
    if !present {
        return Vec::new();
    }
    vec![
        wall_loop_base(LoopType::Outer, WallBoundaryType::ExteriorSurface),
        wall_loop_base(LoopType::Inner, WallBoundaryType::Interior),
    ]
}

fn donor(
    object_id: &str,
    region_id: u64,
    chain: Vec<(String, PaintValue)>,
    walls: Vec<WallLoop>,
    infill_areas: Vec<ExPolygon>,
) -> PerimeterRegion {
    PerimeterRegion {
        object_id: object_id.to_string(),
        region_id,
        variant_chain: chain,
        walls,
        infill_areas,
        ..PerimeterRegion::default()
    }
}

/// Total strategy: every field is derived from the raw draws, so no case can be
/// rejected. The mode word and the region count are `no_shrink`, so shrinking
/// never removes the chain-distinction, empty-inset, modifier or donorless
/// modes.
pub fn partition_strategy(limits: &PartitionLimits) -> BoxedStrategy<PartitionScene> {
    let limits = *limits;
    let seed_count = 16 + limits.max_regions * 24;
    (
        any::<u32>().no_shrink(),
        (1usize..=limits.max_regions).no_shrink(),
        proptest::collection::vec(any::<u32>(), seed_count),
    )
        .prop_map(move |(mode, region_count, seeds)| {
            build_scene(mode, region_count, &seeds, &limits)
        })
        .boxed()
}

/// Roster (slots appear in this order, `count` decides how many exist):
///
/// 0. certifying region on object A, region 0, empty chain: plain rectangle with
///    a deterministic overlapping bridge/bottom/top and a full-domain donor, so
///    its effective inset is nonempty and role precedence is exercised. The H5
///    audit tags therefore rest on mode-stable construction, not sampling.
/// 1. forward-chain sibling sharing `(object A, region 0)` with slot 0. In
///    compositions 0 and 2 it has its own exact donor; in composition 1 it has
///    none even though a reversed-chain donor exists at the same pair, so the
///    hook must warn and leave its roles byte-identical.
/// 2. composition 0/1: reversed-chain sibling sharing `(object A, region 0)`
///    with slot 1. Composition 0 gives it its own right-half donor while the
///    forward sibling keeps the left half; composition 1 gives the only donor
///    to the reversed chain, leaving the forward lookup donorless.
///    composition 2: modifier child minted under the forward parent's region id
///    and carrying the forward parent's two-entry chain. The forward donor is
///    the only donor under that exact ordered chain; the reversed decoy donor
///    shares the `(object, region)` pair but not the chain.
/// 3. `count == 4`: region on object B, region 0, forward chain, whose only
///    donor carries the reversed chain — a reversed-only missing-donor case.
///
/// The forward/reversed sibling chains are exact reverses of each other, and
/// each sibling's donor carries different `infill_areas`, so a pair-keyed join
/// or a sorted/normalized-chain join selects the wrong `W` and fails the
/// independent cell expectation. Decoy donors overlap the donorless geometry,
/// so such a join produces a nonempty `W` where the real hook leaves the region
/// untouched.
fn build_scene(
    mode: u32,
    region_count: usize,
    seeds: &[u32],
    limits: &PartitionLimits,
) -> PartitionScene {
    let step = lattice_step(limits.max_abs_units);
    let count = region_count.clamp(1, limits.max_regions);
    let composition = (mode % 3) as usize;
    let cut = 2 + (seeds[0] as usize % 3);
    let base_primary = LatticeRect {
        c0: 0,
        r0: 0,
        c1: cut,
        r1: LATTICE,
    };
    let sibling_primary = LatticeRect {
        c0: cut,
        r0: 0,
        c1: LATTICE,
        r1: LATTICE,
    };
    let sibling_disjoint = LatticeRect {
        c0: 0,
        r0: 0,
        c1: cut - 1,
        r1: LATTICE,
    };
    // Two nonempty halves of the sibling domain. Each half is a possible donor
    // `infill_areas` set, so the forward and reversed siblings cannot silently
    // share one wall inset.
    let sibling_mid = sibling_primary.c0 + (sibling_primary.width() / 2).max(1);
    let sibling_left = LatticeRect {
        c0: sibling_primary.c0,
        r0: 0,
        c1: sibling_mid,
        r1: LATTICE,
    };
    let sibling_right = LatticeRect {
        c0: sibling_mid,
        r0: 0,
        c1: sibling_primary.c1,
        r1: LATTICE,
    };

    let forward_chain = chain_forward();
    let reversed_chain = chain_reversed();

    let base_shape = (mode >> 2) & 3;
    let sibling_shape = (mode >> 4) & 3;
    let widen_bridge = (mode >> 6) & 1 == 1;
    let forward_mode = (mode >> 7) & 3;
    let decoy_mode = (mode >> 9) & 3;
    let d0_walls = (mode >> 11) & 1 == 1;
    let d1_walls = (mode >> 12) & 1 == 1;
    let forward_plan = donor_plan(
        forward_mode,
        sibling_primary,
        sibling_disjoint,
        seeds,
        8,
        step,
    );
    let decoy_plan = donor_plan(
        decoy_mode,
        sibling_primary,
        sibling_disjoint,
        seeds,
        12,
        step,
    );

    let mut regions = Vec::with_capacity(count);
    let mut donors = Vec::with_capacity(count + 2);

    // Slot 0: the certifying region and its full-domain donor. The donor covers
    // the whole plain rectangle, so `W` is nonempty for every seed draw.
    regions.push(region_with_roles(
        OBJECT_A,
        0,
        Vec::new(),
        shape_polygons(0, base_primary, step),
        base_primary,
        seeds,
        16,
        false,
        true,
        mode,
        0,
        step,
    ));
    donors.push(donor(
        OBJECT_A,
        0,
        Vec::new(),
        walls_for(d0_walls),
        vec![base_primary.expolygon(step)],
    ));

    if count >= 2 {
        // A modifier child must live inside its parent, so the composition-2
        // parent stays a plain rectangle; compositions 0/1 exercise the
        // holed/concave shapes on the painted siblings instead.
        let forward_polygons = if composition == 2 {
            shape_polygons(0, sibling_primary, step)
        } else {
            shape_polygons(sibling_shape, sibling_primary, step)
        };
        regions.push(region_with_roles(
            OBJECT_A,
            0,
            forward_chain.clone(),
            forward_polygons,
            sibling_primary,
            seeds,
            28,
            widen_bridge,
            false,
            mode,
            1,
            step,
        ));
        if composition != 1 {
            let infill = if composition == 0 {
                single_rect_field(sibling_left, step)
            } else {
                forward_plan.infill.clone()
            };
            donors.push(donor(
                OBJECT_A,
                0,
                forward_chain.clone(),
                walls_for(d1_walls),
                infill,
            ));
        }
    }

    if count >= 3 {
        match composition {
            2 => {
                let child_domain = forward_plan
                    .infill_rect
                    .and_then(|infill| sibling_primary.intersect(infill))
                    .unwrap_or(sibling_primary);
                let child_rect = sub_rect(child_domain, seeds, 44);
                let child_polygons = vec![child_rect.expolygon(step)];
                let child_id = modifier_sub_region_id(0, OBJECT_A, &child_polygons);
                regions.push(region_with_roles(
                    OBJECT_A,
                    child_id,
                    forward_chain.clone(),
                    child_polygons,
                    child_rect,
                    seeds,
                    48,
                    false,
                    false,
                    mode,
                    2,
                    step,
                ));
                // Decoy donor: same `(object, region)` pair, reversed chain.
                // Only the exact forward chain can donate to the child.
                donors.push(donor(
                    OBJECT_A,
                    0,
                    reversed_chain.clone(),
                    walls_for(false),
                    decoy_plan.infill.clone(),
                ));
            }
            _ => {
                regions.push(region_with_roles(
                    OBJECT_A,
                    0,
                    reversed_chain.clone(),
                    shape_polygons(sibling_shape, sibling_primary, step),
                    sibling_primary,
                    seeds,
                    52,
                    widen_bridge,
                    false,
                    mode,
                    2,
                    step,
                ));
                donors.push(donor(
                    OBJECT_A,
                    0,
                    reversed_chain.clone(),
                    walls_for(d1_walls),
                    single_rect_field(sibling_right, step),
                ));
            }
        }
    }

    if count >= 4 {
        regions.push(region_with_roles(
            OBJECT_B,
            0,
            forward_chain,
            shape_polygons(base_shape, base_primary, step),
            base_primary,
            seeds,
            60,
            false,
            false,
            mode,
            3,
            step,
        ));
        donors.push(donor(
            OBJECT_B,
            0,
            reversed_chain,
            walls_for(false),
            single_rect_field(sub_rect(base_primary, seeds, 64), step),
        ));
    }

    PartitionScene { regions, donors }
}

fn slice_of(regions: Vec<SlicedRegion>) -> SliceIR {
    SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 1.0,
        regions,
    }
}

fn perimeter_of(donors: Vec<PerimeterRegion>) -> PerimeterIR {
    PerimeterIR {
        global_layer_index: 0,
        regions: donors,
        ..PerimeterIR::default()
    }
}

/// One overhang area covering the lower half of the lattice, keyed by both
/// objects, so every prepared region index derives nonempty clipped geometry.
///
/// Ticket 142 replaced the producer-side four-band partition with the
/// authoritative unsupported area: `PreparedRegionData::overhang_areas` is
/// clipped from `SurfaceClassificationIR::overhang_areas`. The quartile map is
/// given a *different* (upper) band so a regression that reads the old source
/// cannot pass this fixture.
fn classification_for(step: i64) -> SurfaceClassificationIR {
    let area_rect = LatticeRect {
        c0: 0,
        r0: 0,
        c1: LATTICE,
        r1: 3,
    };
    let quartile_rect = LatticeRect {
        c0: 0,
        r0: 3,
        c1: LATTICE,
        r1: LATTICE,
    };
    let area = area_rect.expolygon(step);
    let mut by_layer = HashMap::new();
    by_layer.insert(
        0u32,
        vec![QuartileBand {
            quartile: 1,
            polygons: vec![quartile_rect.expolygon(step)],
        }],
    );
    let mut overhang_areas_by_layer = HashMap::new();
    overhang_areas_by_layer.insert(0u32, vec![area]);
    let mut overhang_quartile_polygons = HashMap::new();
    overhang_quartile_polygons.insert(OBJECT_A.to_string(), by_layer.clone());
    overhang_quartile_polygons.insert(OBJECT_B.to_string(), by_layer);
    let mut overhang_areas = HashMap::new();
    overhang_areas.insert(OBJECT_A.to_string(), overhang_areas_by_layer.clone());
    overhang_areas.insert(OBJECT_B.to_string(), overhang_areas_by_layer);
    SurfaceClassificationIR {
        overhang_quartile_polygons,
        overhang_areas,
        overhang_band_width_mm: 0.4,
        ..SurfaceClassificationIR::default()
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

/// Whether one donor carries the full source identity of one region.
fn donor_matches(donor: &PerimeterRegion, region: &SlicedRegion) -> bool {
    let target_region_id = if is_modifier_namespace_id(region.region_id) {
        match modifier_base_region_id(region.region_id) {
            Some(region_id) => region_id,
            None => return false,
        }
    } else {
        region.region_id
    };
    donor.object_id == region.object_id
        && donor.region_id == target_region_id
        && donor.variant_chain == region.variant_chain
}

/// Exact full-source donor join, independently of the SUT's `HashMap`.
fn find_donor<'a>(
    donors: &'a [PerimeterRegion],
    region: &SlicedRegion,
) -> Option<&'a PerimeterRegion> {
    donors.iter().find(|donor| donor_matches(donor, region))
}

/// Expected output roles for one region, from the documented conditional table.
struct ExpectedRoles {
    bridge: CellSet,
    bottom: CellSet,
    top: CellSet,
    internal: CellSet,
    sparse: CellSet,
    wall_inset: CellSet,
    matched: bool,
    donor_has_walls: bool,
    preserve_bridge_bytes: bool,
}

fn expected_roles(
    grid: &CellGrid,
    input: &SlicedRegion,
    donor: Option<&PerimeterRegion>,
) -> Result<ExpectedRoles, String> {
    let universe = grid.cell_count();
    let original = |polygons: &[ExPolygon]| cells_of(grid, polygons);
    let Some(donor) = donor else {
        // The skip arms carve only the PrePass shell-band marker (a subset of
        // the top) out of the internal bucket.
        return Ok(ExpectedRoles {
            bridge: original(&input.bridge_areas)?,
            bottom: original(&input.bottom_solid_fill)?,
            top: original(&input.top_solid_fill)?,
            internal: original(&input.internal_solid_fill)?.sub(&original(&input.top_solid_fill)?),
            sparse: original(&input.sparse_infill_area)?,
            wall_inset: CellSet::empty(universe),
            matched: false,
            donor_has_walls: false,
            preserve_bridge_bytes: false,
        });
    };
    let wall_inset = original(&donor.infill_areas)?.and(&original(&input.polygons)?);
    let donor_has_walls = !donor.walls.is_empty();
    let is_modifier = is_modifier_namespace_id(input.region_id);

    if wall_inset.is_empty() {
        if donor_has_walls {
            // Walls consumed the whole cross-section: only walls print.
            return Ok(ExpectedRoles {
                bridge: CellSet::empty(universe),
                bottom: CellSet::empty(universe),
                top: CellSet::empty(universe),
                internal: CellSet::empty(universe),
                sparse: CellSet::empty(universe),
                wall_inset,
                matched: true,
                donor_has_walls,
                preserve_bridge_bytes: false,
            });
        }
        // No-wall fallback: preserve the bridge, top minus that bridge, and
        // leave bottom/internal/sparse empty. These roles are NOT subsets of
        // the empty W.
        let bridge = original(&input.bridge_areas)?;
        let top = original(&input.top_solid_fill)?.sub(&bridge);
        return Ok(ExpectedRoles {
            bridge,
            bottom: CellSet::empty(universe),
            top,
            internal: CellSet::empty(universe),
            sparse: CellSet::empty(universe),
            wall_inset,
            matched: true,
            donor_has_walls,
            // A modifier child takes the byte-preservation branch whenever its
            // bridge is contained in the nonempty wall inset; the empty-W
            // branch also clones the bridge unchanged.
            preserve_bridge_bytes: is_modifier,
        });
    }

    let original_bridge = original(&input.bridge_areas)?;
    let bridge = original_bridge.and(&wall_inset);
    // The production source-preservation branch tests the ORIGINAL bridge
    // against the wall inset, not the already-clipped role.
    let preserve_bridge_bytes = is_modifier && original_bridge.is_subset_of(&wall_inset);
    let bottom = original(&input.bottom_solid_fill)?
        .and(&wall_inset)
        .sub(&bridge);
    let top = original(&input.top_solid_fill)?
        .and(&wall_inset)
        .sub(&bridge)
        .sub(&bottom);
    let internal = original(&input.internal_solid_fill)?
        .and(&wall_inset)
        .sub(&bridge)
        .sub(&bottom)
        .sub(&top);
    let sparse = wall_inset
        .sub(&bridge)
        .sub(&bottom)
        .sub(&top)
        .sub(&internal);
    Ok(ExpectedRoles {
        bridge,
        bottom,
        top,
        internal,
        sparse,
        wall_inset,
        matched: true,
        donor_has_walls,
        preserve_bridge_bytes,
    })
}

fn compare_roles(
    grid: &CellGrid,
    observed: &SlicedRegion,
    expected: &ExpectedRoles,
    label: &str,
) -> Result<(), String> {
    for (name, actual, expected_cells) in [
        ("bridge_areas", &observed.bridge_areas, &expected.bridge),
        (
            "bottom_solid_fill",
            &observed.bottom_solid_fill,
            &expected.bottom,
        ),
        ("top_solid_fill", &observed.top_solid_fill, &expected.top),
        (
            "internal_solid_fill",
            &observed.internal_solid_fill,
            &expected.internal,
        ),
        (
            "sparse_infill_area",
            &observed.sparse_infill_area,
            &expected.sparse,
        ),
    ] {
        compare_region_to_cells(actual, expected_cells, grid, &format!("{label} {name}"))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Assert prepared-region records re-associate with the staged slice: one record
/// per region in order, each overhang area equal to `band ∩ region.polygons`
/// derived by the independent cell model.
fn assert_prepared_against_slice(
    grid: &CellGrid,
    slice: &SliceIR,
    prepared: &[PreparedRegionData],
    band: &[ExPolygon],
    label: &str,
) -> Result<(), String> {
    if prepared.len() != slice.regions.len() {
        return Err(format!(
            "{label}: {} prepared records for {} staged regions",
            prepared.len(),
            slice.regions.len()
        ));
    }
    let band_cells = cells_of(grid, band)?;
    for (index, region) in slice.regions.iter().enumerate() {
        let expected = band_cells.and(&cells_of(grid, &region.polygons)?);
        compare_region_to_cells(
            &prepared[index].overhang_areas,
            &expected,
            grid,
            &format!(
                "{label} prepared[{index}] (object {}, region {})",
                region.object_id, region.region_id
            ),
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Check one generated scene against the independent conditional-table model.
pub fn check_scene(scene: &PartitionScene, limits: &PartitionLimits) -> TestCaseResult {
    match check_scene_inner(scene, limits) {
        Ok(()) => Ok(()),
        Err(message) => Err(TestCaseError::fail(message)),
    }
}

fn check_scene_inner(scene: &PartitionScene, limits: &PartitionLimits) -> Result<(), String> {
    let entry = slice_of(scene.regions.clone());
    let perimeter = perimeter_of(scene.donors.clone());
    if entry
        .regions
        .iter()
        .any(|region| region.region_id == MODIFIER_FOOTPRINT_REGION_ID)
    {
        return Err(String::from(
            "generated scene must not stage a raw modifier footprint",
        ));
    }
    let step = lattice_step(limits.max_abs_units);
    let classification = classification_for(step);
    // The prepared view clips the authoritative unsupported area
    // (`SurfaceClassificationIR::overhang_areas`) to each region's polygons;
    // since ticket 142 the quartile bands are not the prepared source.
    let band: Vec<ExPolygon> = classification
        .overhang_areas
        .get(OBJECT_A)
        .and_then(|by_layer| by_layer.get(&0))
        .cloned()
        .unwrap_or_default();

    // The arena-only partition must never touch the immutable Blackboard data.
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let committed = Arc::new(vec![entry.clone()]);
    blackboard
        .commit_slice_ir(Arc::clone(&committed))
        .map_err(|error| format!("commit original slice IR: {error:?}"))?;

    let mut arena = LayerArena::new();
    arena
        .set_slice(entry.clone())
        .map_err(|error| format!("stage entry slice: {error:?}"))?;
    arena
        .set_perimeter(perimeter.clone())
        .map_err(|error| format!("stage perimeter: {error:?}"))?;
    let primed = arena
        .ensure_prepared_regions(Some(&classification))
        .ok_or_else(|| String::from("prepared views must prime from the staged slice"))?
        .len();
    if primed != entry.regions.len() {
        return Err(format!(
            "primed {primed} prepared records for {} staged regions",
            entry.regions.len()
        ));
    }

    let (result, warnings) = run_sync(&mut arena, 0);
    result.map_err(|error| format!("valid generated scene was rejected: {error:?}"))?;

    if arena.prepared_regions().is_some() {
        return Err(String::from(
            "taking the slice must invalidate prepared views; a surviving cache cannot reflect the mutation",
        ));
    }
    let observed = arena
        .slice()
        .cloned()
        .ok_or_else(|| String::from("the hook left no staged SliceIR after success"))?;
    if observed.regions.len() != entry.regions.len() {
        return Err(format!(
            "the hook produced {} regions for {} committed regions without a staged footprint",
            observed.regions.len(),
            entry.regions.len()
        ));
    }
    for (index, (input, actual)) in entry.regions.iter().zip(&observed.regions).enumerate() {
        if input.object_id != actual.object_id
            || input.region_id != actual.region_id
            || input.variant_chain != actual.variant_chain
        {
            return Err(format!(
                "region {index} identity changed: observed ({}, {}, {:?}), committed ({}, {}, {:?})",
                actual.object_id,
                actual.region_id,
                actual.variant_chain,
                input.object_id,
                input.region_id,
                input.variant_chain
            ));
        }
    }
    if arena.perimeter() != Some(&perimeter) {
        return Err(String::from(
            "a successful arena-only partition must leave the staged PerimeterIR byte-identical",
        ));
    }
    let committed_after = blackboard
        .slice_ir()
        .ok_or_else(|| String::from("the committed Blackboard SliceIR disappeared"))?;
    if !Arc::ptr_eq(committed_after, &committed) {
        return Err(String::from(
            "a successful arena-only partition replaced the committed Blackboard Arc",
        ));
    }
    if committed_after.as_slice() != committed.as_slice() {
        return Err(String::from(
            "a successful arena-only partition mutated the committed Blackboard SliceIR",
        ));
    }
    if committed_after.as_slice() != std::slice::from_ref(&entry) {
        return Err(String::from(
            "the committed Blackboard SliceIR no longer equals the entry-state slice",
        ));
    }

    let prepared = arena
        .ensure_prepared_regions(Some(&classification))
        .ok_or_else(|| String::from("prepared views must re-derive from the committed slice"))?
        .to_vec();

    // Collect input boundaries (committed geometry plus the classification band)
    // and output boundaries (observed roles and re-derived prepared geometry) so
    // a cut invented by the SUT becomes a grid line instead of a missed region.
    let mut inputs: Vec<&ExPolygon> = Vec::new();
    for region in &entry.regions {
        inputs.extend(region_polygons(region));
    }
    for donor in &perimeter.regions {
        inputs.extend(donor.infill_areas.iter());
    }
    inputs.extend(band.iter());
    let mut outputs: Vec<&ExPolygon> = Vec::new();
    for region in &observed.regions {
        outputs.extend(region_polygons(region));
    }
    for record in &prepared {
        outputs.extend(record.overhang_areas.iter());
    }
    let grid = CellGrid::from_region_boundaries(inputs, outputs, limits.max_cells)
        .map_err(|error| format!("case cell grid: {error}"))?;

    assert_prepared_against_slice(&grid, &observed, &prepared, &band, "post-sync")?;

    let mut missing_warnings: Vec<Vec<String>> = Vec::new();
    for (index, input) in entry.regions.iter().enumerate() {
        let observed_region = &observed.regions[index];
        let label = format!(
            "region {index} (object {}, region {})",
            input.object_id, input.region_id
        );
        let donor = find_donor(&perimeter.regions, input);
        let expected = expected_roles(&grid, input, donor)?;

        if !expected.matched {
            // Byte-identical apart from the internal bucket's shell-band carve,
            // which is compared semantically.
            let mut residue = observed_region.clone();
            residue.internal_solid_fill = input.internal_solid_fill.clone();
            if &residue != input {
                return Err(format!(
                    "{label}: a missing donor must leave the region byte-identical apart from \
                     the internal-solid carve, observed {observed_region:?}, committed {input:?}"
                ));
            }
            compare_region_to_cells(
                &observed_region.internal_solid_fill,
                &expected.internal,
                &grid,
                &format!("{label} internal_solid_fill (missing donor)"),
            )
            .map_err(|error| error.to_string())?;
            let mut fragments = vec![
                format!("object_id='{}'", input.object_id),
                format!("region_id='{}'", input.region_id),
            ];
            if is_modifier_namespace_id(input.region_id) {
                let parent = modifier_base_region_id(input.region_id).ok_or_else(|| {
                    format!("{label}: modifier namespace id decodes no parent region")
                })?;
                fragments.push(format!("parent_region_id='{parent}'"));
            }
            missing_warnings.push(fragments);
            continue;
        }

        if expected.preserve_bridge_bytes && observed_region.bridge_areas != input.bridge_areas {
            return Err(format!(
                "{label}: a modifier child whose bridge is contained in W must preserve the \
                 bridge Vec<ExPolygon> byte-for-byte; observed {:?}, committed {:?}",
                observed_region.bridge_areas, input.bridge_areas
            ));
        }
        compare_roles(&grid, observed_region, &expected, &label)?;

        // Everything outside the five canonical roles stays byte-identical: no
        // raw modifier footprint is staged, so no later split may partition it.
        let mut residue = observed_region.clone();
        residue.bridge_areas = input.bridge_areas.clone();
        residue.bottom_solid_fill = input.bottom_solid_fill.clone();
        residue.top_solid_fill = input.top_solid_fill.clone();
        residue.internal_solid_fill = input.internal_solid_fill.clone();
        residue.sparse_infill_area = input.sparse_infill_area.clone();
        if &residue != input {
            return Err(format!(
                "{label}: role sync touched a field outside the five canonical fill roles: \
                 observed {residue:?}, committed {input:?}"
            ));
        }

        let actual_bridge = cells_of(&grid, &observed_region.bridge_areas)?;
        let actual_bottom = cells_of(&grid, &observed_region.bottom_solid_fill)?;
        let actual_top = cells_of(&grid, &observed_region.top_solid_fill)?;
        let actual_internal = cells_of(&grid, &observed_region.internal_solid_fill)?;
        let actual_sparse = cells_of(&grid, &observed_region.sparse_infill_area)?;
        if !expected.wall_inset.is_empty() {
            let parts: [(&str, &CellSet); 5] = [
                ("bridge_areas", &actual_bridge),
                ("bottom_solid_fill", &actual_bottom),
                ("top_solid_fill", &actual_top),
                ("internal_solid_fill", &actual_internal),
                ("sparse_infill_area", &actual_sparse),
            ];
            assert_disjoint_union(&parts, &expected.wall_inset, grid.cell_count())
                .map_err(|error| format!("{label}: nonempty-W partition: {error}"))?;
        } else if !expected.donor_has_walls {
            // No-wall empty-W branch: bridge and top are both preserved from the
            // input and must stay disjoint; no subset-of-empty-W claim exists.
            if !actual_bridge.is_disjoint_from(&actual_top) {
                return Err(format!(
                    "{label}: the no-wall fallback must keep bridge and top disjoint"
                ));
            }
        }
    }

    let warning_lines: Vec<&String> = warnings
        .iter()
        .filter(|line| line.contains("no PerimeterIR"))
        .collect();
    if warning_lines.len() != missing_warnings.len() {
        return Err(format!(
            "expected {} missing-donor warning(s), captured {warning_lines:?}",
            missing_warnings.len()
        ));
    }
    for fragments in &missing_warnings {
        if !warning_lines
            .iter()
            .any(|line| fragments.iter().all(|fragment| line.contains(fragment)))
        {
            return Err(format!(
                "no captured warning names every fragment {fragments:?}; captured {warning_lines:?}"
            ));
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-N2 shrink witness audit.
// ─────────────────────────────────────────────────────────────────────────────

/// One mode tag the partition generator distinguishes on its mode-defining
/// (`no_shrink`) draws.
///
/// Every tag is read back from the generated scene, never from the raw mode
/// word, so an audit that keeps the tag proves the *witness* survived
/// `simplify()`. Coordinates are lattice-derived from the shrinkable seed
/// vector, so the tags are restricted to what a seed reduction cannot change:
/// region/donor composition, full-identity donor joins, donor wall presence and
/// donor-infill presence — plus one bounding-box relation, which is sound
/// because a box-disjoint donor cannot intersect its region. The two geometry
/// tags are certified by slot 0's deterministic construction (plain rectangle,
/// full-domain donor, overlapping precedence carriers), so their relations hold
/// for every seed the shrinker can reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PartitionWitnessTag {
    /// A committed region has no donor at its full source identity.
    MissingDonor,
    /// A modifier-namespace child is present.
    ModifierChild,
    /// Two regions share `(object, region id)` but carry different ordered chains.
    ChainSibling,
    /// A donor matches no committed region (a chain or pair decoy).
    DecoyDonor,
    /// A matched donor carries an empty `infill_areas` (the empty-inset branch).
    EmptyDonorInfill,
    /// A matched donor's infill bounding box misses its region (empty inset).
    DonorInfillOutsideRegion,
    /// A matched donor's infill bounding box overlaps its region.
    DonorInfillOverlapsRegion,
    /// A matched donor declares walls.
    DonorWithWalls,
    /// A matched donor declares no walls.
    DonorWithoutWalls,
    /// A matched donor's `infill_areas ∩ region.polygons` is nonempty.
    NonemptyEffectiveInset,
    /// A region has a cell in two or more of its original bridge/bottom/top
    /// carriers, so the precedence order is exercised.
    RolePrecedenceOverlap,
}

impl PartitionWitnessTag {
    /// Every tag, in a stable order.
    pub const ALL: [PartitionWitnessTag; 11] = [
        PartitionWitnessTag::MissingDonor,
        PartitionWitnessTag::ModifierChild,
        PartitionWitnessTag::ChainSibling,
        PartitionWitnessTag::DecoyDonor,
        PartitionWitnessTag::EmptyDonorInfill,
        PartitionWitnessTag::DonorInfillOutsideRegion,
        PartitionWitnessTag::DonorInfillOverlapsRegion,
        PartitionWitnessTag::DonorWithWalls,
        PartitionWitnessTag::DonorWithoutWalls,
        PartitionWitnessTag::NonemptyEffectiveInset,
        PartitionWitnessTag::RolePrecedenceOverlap,
    ];
}

/// Tags the audit must observe explicitly; a zero-tag run is a failure too.
const REQUIRED_WITNESS_TAGS: [PartitionWitnessTag; 2] = [
    PartitionWitnessTag::NonemptyEffectiveInset,
    PartitionWitnessTag::RolePrecedenceOverlap,
];

/// Cell budget for the small per-witness grid the two geometry tags classify.
const WITNESS_CELL_BUDGET: usize = MAX_CELLS as usize;

/// Axis-aligned bounding box `(min_x, max_x, min_y, max_y)` of a polygon set.
fn bbox_of(polygons: &[ExPolygon]) -> Option<(i64, i64, i64, i64)> {
    let mut bbox: Option<(i64, i64, i64, i64)> = None;
    for polygon in polygons {
        for point in std::iter::once(&polygon.contour)
            .chain(polygon.holes.iter())
            .flat_map(|polygon| polygon.points.iter())
        {
            bbox = Some(match bbox {
                None => (point.x, point.x, point.y, point.y),
                Some((min_x, max_x, min_y, max_y)) => (
                    min_x.min(point.x),
                    max_x.max(point.x),
                    min_y.min(point.y),
                    max_y.max(point.y),
                ),
            });
        }
    }
    bbox
}

/// Whether two bounding boxes share interior area.
fn boxes_intersect(left: (i64, i64, i64, i64), right: (i64, i64, i64, i64)) -> bool {
    left.0 < right.1 && right.0 < left.1 && left.2 < right.3 && right.2 < left.3
}

/// Classify a small carrier set on a grid built from exactly those carriers.
///
/// The generated witness geometry is orthogonal, so midpoint cell membership is
/// exact. A grid that cannot be formed (empty or non-orthogonal input) returns
/// `None`, and the tag is simply not observed.
fn classify_carriers<'a>(
    carriers: impl IntoIterator<Item = &'a [ExPolygon]>,
) -> Option<Vec<CellSet>> {
    let carriers: Vec<&[ExPolygon]> = carriers.into_iter().collect();
    let grid = CellGrid::from_region_boundaries(
        carriers.iter().flat_map(|polygons| (*polygons).iter()),
        std::iter::empty::<&ExPolygon>(),
        WITNESS_CELL_BUDGET,
    )
    .ok()?;
    carriers
        .iter()
        .map(|polygons| CellSet::classify(&grid, polygons).ok())
        .collect()
}

/// Whether one matched donor's `W = infill_areas ∩ region.polygons` is nonempty.
fn nonempty_effective_inset(region: &SlicedRegion, donor: &PerimeterRegion) -> bool {
    let Some(sets) = classify_carriers([donor.infill_areas.as_slice(), region.polygons.as_slice()])
    else {
        return false;
    };
    !sets[0].and(&sets[1]).is_empty()
}

/// Whether one region has a cell in two or more of its original carrier roles.
fn role_precedence_overlap(region: &SlicedRegion) -> bool {
    let Some(sets) = classify_carriers([
        region.bridge_areas.as_slice(),
        region.bottom_solid_fill.as_slice(),
        region.top_solid_fill.as_slice(),
    ]) else {
        return false;
    };
    let Some((bridge, rest)) = sets.split_first() else {
        return false;
    };
    let Some((bottom, rest)) = rest.split_first() else {
        return false;
    };
    let Some(top) = rest.first() else {
        return false;
    };
    !bridge.and(bottom).is_empty() || !bridge.and(top).is_empty() || !bottom.and(top).is_empty()
}

/// Every mode tag the generated scene carries.
pub fn witness_tags(scene: &PartitionScene) -> BTreeSet<PartitionWitnessTag> {
    let mut tags = BTreeSet::new();
    let mut nonempty_inset = false;
    let mut precedence_overlap = false;
    for region in &scene.regions {
        if is_modifier_namespace_id(region.region_id) {
            tags.insert(PartitionWitnessTag::ModifierChild);
        }
        if !precedence_overlap && role_precedence_overlap(region) {
            precedence_overlap = true;
        }
        match find_donor(&scene.donors, region) {
            None => {
                tags.insert(PartitionWitnessTag::MissingDonor);
            }
            Some(donor) => {
                if !nonempty_inset && nonempty_effective_inset(region, donor) {
                    nonempty_inset = true;
                }
                tags.insert(if donor.walls.is_empty() {
                    PartitionWitnessTag::DonorWithoutWalls
                } else {
                    PartitionWitnessTag::DonorWithWalls
                });
                tags.insert(if donor.infill_areas.is_empty() {
                    PartitionWitnessTag::EmptyDonorInfill
                } else {
                    match (bbox_of(&donor.infill_areas), bbox_of(&region.polygons)) {
                        (Some(donor_box), Some(region_box))
                            if boxes_intersect(donor_box, region_box) =>
                        {
                            PartitionWitnessTag::DonorInfillOverlapsRegion
                        }
                        _ => PartitionWitnessTag::DonorInfillOutsideRegion,
                    }
                });
            }
        }
    }
    if nonempty_inset {
        tags.insert(PartitionWitnessTag::NonemptyEffectiveInset);
    }
    if precedence_overlap {
        tags.insert(PartitionWitnessTag::RolePrecedenceOverlap);
    }
    if scene.donors.iter().any(|donor| {
        !scene
            .regions
            .iter()
            .any(|region| donor_matches(donor, region))
    }) {
        tags.insert(PartitionWitnessTag::DecoyDonor);
    }
    let mut chain_sibling = false;
    for (index, left) in scene.regions.iter().enumerate() {
        for right in &scene.regions[index + 1..] {
            if left.object_id == right.object_id
                && left.region_id == right.region_id
                && left.variant_chain != right.variant_chain
            {
                chain_sibling = true;
            }
        }
    }
    if chain_sibling {
        tags.insert(PartitionWitnessTag::ChainSibling);
    }
    tags
}

/// Number of tags the shrink audit must reach.
const SHRINK_AUDIT_MODES: usize = PartitionWitnessTag::ALL.len();

/// Fixed-seed AC-N2 shrink audit.
///
/// Walks `simplify()` on 128 fixed-seed trees. Every simplification step asserts
/// that the case's full tag set is unchanged — that is the witness-retention
/// claim — and the same `check_scene` `run_property` runs is applied to the
/// generated tree and to the final simplified tree, the value a failing case
/// would actually report. (A checker call drives the production partition hook
/// and rebuilds the independent cell grid, so re-checking every intermediate
/// search state would cost minutes; those states are search artifacts, not
/// reported witnesses.) Only the seed vector is shrinkable here, so a tag lost
/// to shrinking would mean the generator derives its witness from a shrinkable
/// draw.
///
/// The loop records whether it stopped because `simplify()` returned false (a
/// fixpoint) or because it reached the 64-step audit cap. The tree checked at
/// the end is therefore the *final simplified tree at the audit cap* when the
/// cap was hit — not a maximally shrunk tree.
pub fn shrink_audit() {
    let limits =
        checked_partition_limits(3, 16, 512).expect("H-RUNTIME-PARTITION shrink audit ceilings");
    let strategy = partition_strategy(&limits);
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
                "H-RUNTIME-PARTITION: the unsimplified audit tree must satisfy the contract \
                 (tags {tags:?}): {error}"
            )
        });
        let mut steps = 0_usize;
        let mut reached_fixpoint = false;
        while steps < 64 {
            if !tree.simplify() {
                reached_fixpoint = true;
                break;
            }
            steps += 1;
            simplifications += 1;
            assert_eq!(
                witness_tags(&tree.current()),
                tags,
                "H-RUNTIME-PARTITION: shrinking changed the case mode"
            );
        }
        if steps > 0 {
            let shrunk = tree.current();
            let ending = if reached_fixpoint {
                "fixpoint"
            } else {
                "audit cap (64 steps; further simplification is not ruled out)"
            };
            check_scene(&shrunk, &limits).unwrap_or_else(|error| {
                panic!(
                    "H-RUNTIME-PARTITION: the final simplified tree at the {ending} no longer \
                     satisfies the contract (tags {tags:?}): {error}"
                )
            });
        }
    }
    assert!(
        !seen.is_empty(),
        "H-RUNTIME-PARTITION: shrink audit observed zero witness tags"
    );
    for tag in REQUIRED_WITNESS_TAGS {
        assert!(
            seen.contains(&tag),
            "H-RUNTIME-PARTITION: shrink audit never observed required witness tag {tag:?}"
        );
    }
    assert!(
        seen.len() >= SHRINK_AUDIT_MODES,
        "H-RUNTIME-PARTITION: shrink audit reached {} modes, expected at least {}",
        seen.len(),
        SHRINK_AUDIT_MODES
    );
    assert!(
        simplifications > 0,
        "H-RUNTIME-PARTITION: the shrink audit simplified nothing, so it certifies nothing"
    );
}

/// Run the registry-selected `H-RUNTIME-PARTITION` property.
pub fn run_property() {
    let run =
        PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("H-RUNTIME-PARTITION registry context");
    let limits = checked_partition_limits(
        run.bound("max_regions").expect("max_regions bound"),
        run.bound("max_abs_units").expect("max_abs_units bound"),
        run.bound("max_cells").expect("max_cells bound"),
    )
    .expect("H-RUNTIME-PARTITION safety ceilings");
    shrink_audit();
    let strategy = partition_strategy(&limits);
    run.check(&strategy, |scene, _guard| check_scene(&scene, &limits))
        .expect("H-RUNTIME-PARTITION conditional fill contract");
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
    variant_chain: Vec<(String, PaintValue)>,
    polygons: Vec<ExPolygon>,
) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id,
        polygons,
        variant_chain,
        effective_layer_height: 0.2,
        ..SlicedRegion::default()
    }
}

fn control_donor(region_id: u64, walls: bool, infill_areas: Vec<ExPolygon>) -> PerimeterRegion {
    donor(
        OBJECT_A,
        region_id,
        Vec::new(),
        walls_for(walls),
        infill_areas,
    )
}

fn stage(entry: SliceIR, perimeter: PerimeterIR) -> (LayerArena, PerimeterIR) {
    let mut arena = LayerArena::new();
    arena.set_slice(entry).expect("stage control slice");
    arena
        .set_perimeter(perimeter.clone())
        .expect("stage control perimeter");
    (arena, perimeter)
}

fn sync_ok(arena: &mut LayerArena) -> Vec<String> {
    let (result, warnings) = run_sync(arena, 0);
    result.expect("the control scene must be accepted by the real partition hook");
    warnings
}

fn control_grid(inputs: &[&[ExPolygon]], outputs: &[&[ExPolygon]]) -> CellGrid {
    CellGrid::from_region_boundaries(
        inputs.iter().flat_map(|polygons| polygons.iter()),
        outputs.iter().flat_map(|polygons| polygons.iter()),
        2048,
    )
    .expect("control cell grid")
}

fn cells(grid: &CellGrid, polygons: &[ExPolygon]) -> CellSet {
    CellSet::classify(grid, polygons).expect("control polygons classify on their own grid")
}

fn assert_cells(actual: &[ExPolygon], expected: &CellSet, grid: &CellGrid, label: &str) {
    if let Err(error) = compare_region_to_cells(actual, expected, grid, label) {
        panic!("{label}: {error}");
    }
}

/// Independent clipped-precedence roles for one region and one candidate `W`:
/// bridge > bottom > top > sparse, exactly the documented production order.
fn independent_roles(
    grid: &CellGrid,
    wall_inset: &CellSet,
    bridge: &[ExPolygon],
    bottom: &[ExPolygon],
    top: &[ExPolygon],
) -> (CellSet, CellSet, CellSet, CellSet) {
    let bridge = cells(grid, bridge).and(wall_inset);
    let bottom = cells(grid, bottom).and(wall_inset).sub(&bridge);
    let top = cells(grid, top).and(wall_inset).sub(&bridge).sub(&bottom);
    let sparse = wall_inset.sub(&bridge).sub(&bottom).sub(&top);
    (bridge, bottom, top, sparse)
}

/// The four observed fill roles as one exact cell tuple.
fn observed_roles(region: &SlicedRegion, grid: &CellGrid) -> (CellSet, CellSet, CellSet, CellSet) {
    (
        cells(grid, &region.bridge_areas),
        cells(grid, &region.bottom_solid_fill),
        cells(grid, &region.top_solid_fill),
        cells(grid, &region.sparse_infill_area),
    )
}

fn assert_observed_roles(
    region: &SlicedRegion,
    expected: &(CellSet, CellSet, CellSet, CellSet),
    grid: &CellGrid,
    label: &str,
) {
    assert_cells(
        &region.bridge_areas,
        &expected.0,
        grid,
        &format!("{label} bridge"),
    );
    assert_cells(
        &region.bottom_solid_fill,
        &expected.1,
        grid,
        &format!("{label} bottom"),
    );
    assert_cells(
        &region.top_solid_fill,
        &expected.2,
        grid,
        &format!("{label} top"),
    );
    assert_cells(
        &region.sparse_infill_area,
        &expected.3,
        grid,
        &format!("{label} sparse"),
    );
}

fn role_vec(polygons: &[ExPolygon]) -> Vec<ExPolygon> {
    polygons.to_vec()
}

/// AC-4 precedence/identity controls. Every claim below is made against the
/// actual `sync_perimeter_infill_areas_into_slice` output, never against a
/// re-implementation.
pub fn runtime_partition_precedence_and_identity_controls() {
    shrink_audit();
    swapped_priority_disagrees();
    unclipped_nonempty_bridge_disagrees();
    pair_only_donor_join_disagrees();
    missing_donor_fatal_conversion_is_falsified();
    unconditional_empty_w_subset_assertion_is_falsified();
    blackboard_and_perimeter_survive_and_prepared_views_reassociate();
}

fn swapped_priority_disagrees() {
    let polygons = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let bridge = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let bottom = vec![unit_rect(0, 0, 10, 8).to_expolygon()];
    let top = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let mut region = control_region(OBJECT_A, 0, Vec::new(), polygons.clone());
    region.bridge_areas = role_vec(&bridge);
    region.bottom_solid_fill = role_vec(&bottom);
    region.top_solid_fill = role_vec(&top);
    region.sparse_infill_area = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    let infill = polygons.clone();
    let (mut arena, _) = stage(
        slice_of(vec![region]),
        perimeter_of(vec![control_donor(0, false, infill.clone())]),
    );
    sync_ok(&mut arena);
    let observed = arena.slice().expect("slice stays staged").regions[0].clone();
    let grid = control_grid(
        &[&polygons, &bridge, &bottom, &top, &infill],
        &[
            &observed.bridge_areas,
            &observed.bottom_solid_fill,
            &observed.top_solid_fill,
            &observed.sparse_infill_area,
        ],
    );
    let wall_inset = cells(&grid, &polygons).and(&cells(&grid, &infill));
    let expected_bridge = cells(&grid, &bridge).and(&wall_inset);
    let expected_bottom = cells(&grid, &bottom).and(&wall_inset).sub(&expected_bridge);
    let expected_top = cells(&grid, &top)
        .and(&wall_inset)
        .sub(&expected_bridge)
        .sub(&expected_bottom);
    let expected_sparse = wall_inset
        .sub(&expected_bridge)
        .sub(&expected_bottom)
        .sub(&expected_top);
    assert_cells(
        &observed.bridge_areas,
        &expected_bridge,
        &grid,
        "swapped-priority witness bridge",
    );
    assert_cells(
        &observed.bottom_solid_fill,
        &expected_bottom,
        &grid,
        "swapped-priority witness bottom",
    );
    assert_cells(
        &observed.top_solid_fill,
        &expected_top,
        &grid,
        "swapped-priority witness top",
    );
    assert_cells(
        &observed.sparse_infill_area,
        &expected_sparse,
        &grid,
        "swapped-priority witness sparse",
    );

    // Known-wrong model: top before bottom. Every role overlaps on this witness.
    let swapped_top = cells(&grid, &top).and(&wall_inset).sub(&expected_bridge);
    let swapped_bottom = cells(&grid, &bottom)
        .and(&wall_inset)
        .sub(&expected_bridge)
        .sub(&swapped_top);
    let actual_top = cells(&grid, &observed.top_solid_fill);
    let actual_bottom = cells(&grid, &observed.bottom_solid_fill);
    assert!(
        !swapped_top.is_disjoint_from(&actual_top),
        "the two precedence orders must overlap for this control to mean anything"
    );
    assert_ne!(
        swapped_top, actual_top,
        "a top-before-bottom model must disagree with the real bottom-before-top output"
    );
    assert_ne!(
        swapped_bottom, actual_bottom,
        "a top-before-bottom model must also disagree on the bottom role"
    );
}

fn unclipped_nonempty_bridge_disagrees() {
    let polygons = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let bridge = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let infill = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let mut region = control_region(OBJECT_A, 0, Vec::new(), polygons.clone());
    region.bridge_areas = role_vec(&bridge);
    let (mut arena, _) = stage(
        slice_of(vec![region]),
        perimeter_of(vec![control_donor(0, false, infill.clone())]),
    );
    sync_ok(&mut arena);
    let observed = arena.slice().expect("slice stays staged").regions[0].clone();
    let grid = control_grid(
        &[&polygons, &bridge, &infill, &observed.bridge_areas],
        &[&observed.bridge_areas],
    );
    let wall_inset = cells(&grid, &polygons).and(&cells(&grid, &infill));
    let clipped = cells(&grid, &bridge).and(&wall_inset);
    let actual = cells(&grid, &observed.bridge_areas);
    assert_cells(
        &observed.bridge_areas,
        &clipped,
        &grid,
        "unclipped-bridge witness",
    );
    assert!(
        !cells(&grid, &bridge).is_subset_of(&wall_inset),
        "the witness bridge must extend past the nonempty wall inset"
    );
    assert_ne!(
        cells(&grid, &bridge),
        actual,
        "an unclipped nonempty-bridge model must disagree with the real clipped output"
    );
}

fn pair_only_donor_join_disagrees() {
    // Two painted siblings share `(object, region)` but carry exact reverse
    // two-entry chains. Their polygons are identical; only the ordered chain
    // distinguishes their donors.
    let forward_chain = chain_forward();
    let reversed_chain = chain_reversed();
    let mut sorted_forward = forward_chain.clone();
    sorted_forward.sort();
    let mut sorted_reversed = reversed_chain.clone();
    sorted_reversed.sort();
    assert_eq!(
        sorted_forward, sorted_reversed,
        "a sorted/normalized-chain key must collapse the reversed pair"
    );
    assert_ne!(
        forward_chain, reversed_chain,
        "the witness chains must be distinct ordered identities"
    );

    let polygons = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let mut forward = control_region(OBJECT_A, 0, forward_chain.clone(), polygons.clone());
    forward.bridge_areas = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    forward.bottom_solid_fill = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    forward.top_solid_fill = vec![unit_rect(0, 2, 10, 10).to_expolygon()];
    let mut reversed = control_region(OBJECT_A, 0, reversed_chain.clone(), polygons.clone());
    reversed.bridge_areas = forward.bridge_areas.clone();
    reversed.bottom_solid_fill = forward.bottom_solid_fill.clone();
    reversed.top_solid_fill = forward.top_solid_fill.clone();

    // Different donor infill per sibling: a pair-keyed join (last-collected
    // donor wins) or a normalized-chain join (one collapsed key) must apply one
    // sibling's W to the other.
    let forward_infill = vec![unit_rect(0, 0, 5, 10).to_expolygon()];
    let reversed_infill = vec![unit_rect(5, 0, 10, 10).to_expolygon()];
    let forward_bridge = role_vec(&forward.bridge_areas);
    let forward_bottom = role_vec(&forward.bottom_solid_fill);
    let forward_top = role_vec(&forward.top_solid_fill);
    let input_forward = forward.clone();
    let input_reversed = reversed.clone();

    let (mut arena, _) = stage(
        slice_of(vec![forward, reversed]),
        perimeter_of(vec![
            donor(
                OBJECT_A,
                0,
                forward_chain,
                Vec::new(),
                forward_infill.clone(),
            ),
            donor(
                OBJECT_A,
                0,
                reversed_chain,
                Vec::new(),
                reversed_infill.clone(),
            ),
        ]),
    );
    sync_ok(&mut arena);
    let observed = arena.slice().expect("slice stays staged").regions.clone();
    assert_eq!(
        observed[0].variant_chain, input_forward.variant_chain,
        "the forward sibling keeps its exact ordered chain"
    );
    assert_eq!(
        observed[1].variant_chain, input_reversed.variant_chain,
        "the reversed sibling keeps its exact ordered chain"
    );
    assert_eq!(
        observed[0].object_id, observed[1].object_id,
        "the witness pair shares one object"
    );
    assert_eq!(
        observed[0].region_id, observed[1].region_id,
        "the witness pair shares one numeric region id"
    );

    let grid = control_grid(
        &[
            &polygons,
            &forward_infill,
            &reversed_infill,
            &forward_bridge,
            &forward_bottom,
            &forward_top,
        ],
        &[
            &observed[0].bridge_areas,
            &observed[0].bottom_solid_fill,
            &observed[0].top_solid_fill,
            &observed[0].sparse_infill_area,
            &observed[1].bridge_areas,
            &observed[1].bottom_solid_fill,
            &observed[1].top_solid_fill,
            &observed[1].sparse_infill_area,
        ],
    );
    let forward_inset = cells(&grid, &polygons).and(&cells(&grid, &forward_infill));
    let reversed_inset = cells(&grid, &polygons).and(&cells(&grid, &reversed_infill));
    assert!(
        !forward_inset.is_empty() && !reversed_inset.is_empty(),
        "both exact-chain donors must induce a nonempty effective inset"
    );
    assert_ne!(
        forward_inset, reversed_inset,
        "the sibling donors must induce different W sets"
    );

    // The real exact-chain call: each sibling takes its own donor's W.
    let expected_forward = independent_roles(
        &grid,
        &forward_inset,
        &forward_bridge,
        &forward_bottom,
        &forward_top,
    );
    let expected_reversed = independent_roles(
        &grid,
        &reversed_inset,
        &forward_bridge,
        &forward_bottom,
        &forward_top,
    );
    assert_observed_roles(
        &observed[0],
        &expected_forward,
        &grid,
        "chain-exact forward",
    );
    assert_observed_roles(
        &observed[1],
        &expected_reversed,
        &grid,
        "chain-exact reversed",
    );
    assert_ne!(
        expected_forward, expected_reversed,
        "the two exact-chain expectations must differ, or the witness proves nothing"
    );

    // Known-wrong model A: pair-keyed (or last-collected) join applies the
    // reversed donor's W to the forward region.
    let forward_with_reversed_w = independent_roles(
        &grid,
        &reversed_inset,
        &forward_bridge,
        &forward_bottom,
        &forward_top,
    );
    assert_ne!(
        observed_roles(&observed[0], &grid),
        forward_with_reversed_w,
        "a pair-keyed join must disagree with the real forward region's clipping"
    );
    // Known-wrong model B: the same collapsed key applies the forward donor's W
    // to the reversed region. Together A and B show no single pair-keyed or
    // normalized-chain donor choice can satisfy both siblings.
    let reversed_with_forward_w = independent_roles(
        &grid,
        &forward_inset,
        &forward_bridge,
        &forward_bottom,
        &forward_top,
    );
    assert_ne!(
        observed_roles(&observed[1], &grid),
        reversed_with_forward_w,
        "a normalized-chain join must disagree with the real reversed region's clipping"
    );
}

fn missing_donor_fatal_conversion_is_falsified() {
    let matching_polygons = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let mut matching = control_region(OBJECT_A, 0, Vec::new(), matching_polygons.clone());
    matching.bridge_areas = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    matching.bottom_solid_fill = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    matching.top_solid_fill = vec![unit_rect(0, 2, 10, 4).to_expolygon()];
    let donorless_polygons = vec![unit_rect(0, 4, 10, 10).to_expolygon()];
    let mut donorless = control_region(OBJECT_A, 1, Vec::new(), donorless_polygons.clone());
    donorless.bridge_areas = vec![unit_rect(0, 4, 10, 6).to_expolygon()];
    donorless.top_solid_fill = vec![unit_rect(0, 6, 10, 10).to_expolygon()];
    let input_donorless = donorless.clone();
    let (mut arena, _) = stage(
        slice_of(vec![matching, donorless]),
        perimeter_of(vec![
            // Matching donor for region 0.
            control_donor(0, false, matching_polygons.clone()),
            // Chain-mismatched decoy for region 1: a chain-insensitive join would
            // partition the donorless region instead of leaving it untouched.
            donor(OBJECT_A, 1, chain_for(1), Vec::new(), donorless_polygons),
        ]),
    );
    let warnings = sync_ok(&mut arena);
    let observed = arena.slice().expect("slice stays staged").regions.clone();
    assert_eq!(
        observed[1], input_donorless,
        "a missing individual donor must leave the region byte-identical, not fatal"
    );
    assert!(
        warnings.iter().any(|line| line.contains("no PerimeterIR")
            && line.contains("object_id='partition-property-object-a'")
            && line.contains("region_id='1'")),
        "the missing-donor warning must name the object and region, captured {warnings:?}"
    );
    // The scene had real work: the matching region was repartitioned.
    assert_ne!(
        observed[0].bottom_solid_fill, observed[0].top_solid_fill,
        "the matching region must be partitioned for the skip to be meaningful"
    );
    // A model that converts a missing donor into `FatalModule` predicted `Err`;
    // `sync_ok` above already falsified it by requiring `Ok` from the real hook.
}

fn unconditional_empty_w_subset_assertion_is_falsified() {
    let polygons = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let bridge = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    let top = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let mut region = control_region(OBJECT_A, 0, Vec::new(), polygons.clone());
    region.bridge_areas = role_vec(&bridge);
    region.top_solid_fill = role_vec(&top);
    region.bottom_solid_fill = vec![unit_rect(0, 0, 10, 8).to_expolygon()];
    let (mut arena, _) = stage(
        slice_of(vec![region]),
        // No walls and no infill area: the no-wall empty-W branch.
        perimeter_of(vec![control_donor(0, false, Vec::new())]),
    );
    sync_ok(&mut arena);
    let observed = arena.slice().expect("slice stays staged").regions[0].clone();
    let grid = control_grid(
        &[
            &polygons,
            &bridge,
            &top,
            &observed.bridge_areas,
            &observed.top_solid_fill,
        ],
        &[
            &observed.bridge_areas,
            &observed.top_solid_fill,
            &observed.bottom_solid_fill,
            &observed.sparse_infill_area,
        ],
    );
    let empty = CellSet::empty(grid.cell_count());
    let actual_bridge = cells(&grid, &observed.bridge_areas);
    let actual_top = cells(&grid, &observed.top_solid_fill);
    assert_cells(
        &observed.bridge_areas,
        &cells(&grid, &bridge),
        &grid,
        "no-wall fallback bridge",
    );
    assert_cells(
        &observed.top_solid_fill,
        &cells(&grid, &top).sub(&cells(&grid, &bridge)),
        &grid,
        "no-wall fallback top",
    );
    assert_cells(
        &observed.bottom_solid_fill,
        &empty,
        &grid,
        "no-wall fallback bottom",
    );
    assert_cells(
        &observed.sparse_infill_area,
        &empty,
        &grid,
        "no-wall fallback sparse",
    );
    assert!(
        !actual_bridge.is_empty() && !actual_top.is_empty(),
        "the witness must preserve nonempty bridge and top roles"
    );
    assert!(
        !actual_bridge.is_subset_of(&empty) && !actual_top.is_subset_of(&empty),
        "an unconditional 'every role is a subset of W' model is falsified when W is empty"
    );
}

fn blackboard_and_perimeter_survive_and_prepared_views_reassociate() {
    // Part A: immutable Blackboard data and the staged perimeter are untouched by
    // a successful arena-only partition.
    let polygons = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let infill = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let mut region = control_region(OBJECT_A, 0, Vec::new(), polygons);
    region.bridge_areas = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    region.top_solid_fill = vec![unit_rect(0, 2, 10, 4).to_expolygon()];
    let entry = slice_of(vec![region]);
    let perimeter = perimeter_of(vec![control_donor(0, false, infill)]);
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let committed = Arc::new(vec![entry.clone()]);
    blackboard
        .commit_slice_ir(Arc::clone(&committed))
        .expect("commit original Blackboard slice IR");
    let (mut arena, staged_perimeter) = stage(entry.clone(), perimeter);
    let warnings = sync_ok(&mut arena);
    assert!(
        !warnings.iter().any(|line| line.contains("no PerimeterIR")),
        "a fully matched scene must not warn about a missing donor: {warnings:?}"
    );
    assert_eq!(
        arena.perimeter(),
        Some(&staged_perimeter),
        "a successful role sync must leave the staged PerimeterIR byte-identical"
    );
    let after = blackboard
        .slice_ir()
        .expect("the committed Blackboard SliceIR stays present");
    assert!(
        Arc::ptr_eq(after, &committed),
        "an arena-only partition must not replace the committed Blackboard Arc"
    );
    assert_eq!(
        after.as_slice(),
        committed.as_slice(),
        "an arena-only partition must not mutate the committed Blackboard SliceIR"
    );
    assert_eq!(
        after.as_slice(),
        std::slice::from_ref(&entry),
        "the committed slice IR must still equal the entry-state slice"
    );

    // Part B: a staged raw footprint mutates the slice (base polygons shrink and
    // a modifier child is minted), so a prepared cache that survived the take
    // would return the pre-call geometry and disagree.
    let base_polygons = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let footprint = vec![unit_rect(2, 2, 8, 8).to_expolygon()];
    let mut base = control_region(OBJECT_A, 0, Vec::new(), base_polygons.clone());
    base.bridge_areas = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    base.top_solid_fill = vec![unit_rect(0, 2, 10, 4).to_expolygon()];
    let raw = control_region(
        OBJECT_A,
        MODIFIER_FOOTPRINT_REGION_ID,
        Vec::new(),
        footprint.clone(),
    );
    let entry = slice_of(vec![base, raw]);
    let perimeter = perimeter_of(vec![control_donor(0, false, base_polygons.clone())]);
    let band = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let mut by_layer = HashMap::new();
    by_layer.insert(
        0u32,
        vec![QuartileBand {
            quartile: 1,
            polygons: band.clone(),
        }],
    );
    let mut overhang_quartile_polygons = HashMap::new();
    overhang_quartile_polygons.insert(OBJECT_A.to_string(), by_layer);
    let classification = SurfaceClassificationIR {
        overhang_quartile_polygons,
        ..SurfaceClassificationIR::default()
    };

    let (mut arena, _) = stage(entry.clone(), perimeter);
    let primed = arena
        .ensure_prepared_regions(Some(&classification))
        .expect("prepared views prime before mutation")
        .to_vec();
    assert_eq!(
        primed.len(),
        entry.regions.len(),
        "one primed record per staged region"
    );
    sync_ok(&mut arena);
    assert!(
        arena.prepared_regions().is_none(),
        "taking the slice must invalidate prepared views before the re-derivation"
    );
    let observed = arena
        .slice()
        .cloned()
        .expect("the hook re-stages the mutated slice");
    assert_eq!(
        observed.regions.len(),
        2,
        "the raw footprint is consumed and replaced by one minted child"
    );
    assert_ne!(
        observed.regions[0].polygons, base_polygons,
        "the base polygons must shrink so a stale prepared cache is observable"
    );
    let prepared = arena
        .ensure_prepared_regions(Some(&classification))
        .expect("prepared views re-derive from the mutated slice")
        .to_vec();
    let mut inputs: Vec<&ExPolygon> = base_polygons.iter().chain(footprint.iter()).collect();
    inputs.extend(band.iter());
    let mut output_fields: Vec<&[ExPolygon]> = prepared
        .iter()
        .map(|record| record.overhang_areas.as_slice())
        .collect();
    output_fields.push(observed.regions[0].polygons.as_slice());
    let grid = control_grid(
        &[
            &base_polygons,
            &footprint,
            &band,
            &observed.regions[0].polygons,
            &observed.regions[1].polygons,
        ],
        &output_fields,
    );
    assert_prepared_against_slice(&grid, &observed, &prepared, &band, "post-mutation")
        .expect("re-derived prepared views must re-associate with the mutated slice");
    let base_band = cells(&grid, &band).and(&cells(&grid, &observed.regions[0].polygons));
    let stale_base_band = cells(&grid, &band).and(&cells(&grid, &base_polygons));
    assert_ne!(
        base_band, stale_base_band,
        "the mutation must change the prepared base geometry for this control to mean anything"
    );
    assert_eq!(
        cells(&grid, &prepared[0].overhang_areas),
        base_band,
        "prepared[0] must carry the mutated base geometry, not the pre-call cache"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-N2 stage and domain controls.
// ─────────────────────────────────────────────────────────────────────────────

/// AC-N2: the two `FatalModule` identities, entry-state slice restoration on a
/// missing perimeter, the non-fatal missing-donor population, and the shared
/// domain helpers' fail-closed bounds.
pub fn runtime_partition_stage_and_domain_rejections() {
    missing_slice_is_fatal();
    missing_perimeter_is_fatal_and_restores_the_taken_slice();
    missing_individual_donor_is_not_the_fatal_error();
    domain_helpers_fail_closed();
}

fn assert_fatal_module(
    error: &LayerStageError,
    message_fragment: &str,
    context: &str,
) -> Result<(), String> {
    match error {
        LayerStageError::FatalModule {
            stage_id,
            module_id,
            message,
        } => {
            if stage_id != "Layer::Perimeters" {
                return Err(format!("{context}: stage_id {stage_id:?}"));
            }
            if module_id != "host:region_partition" {
                return Err(format!("{context}: module_id {module_id:?}"));
            }
            if !message.contains(message_fragment) {
                return Err(format!(
                    "{context}: message {message:?} does not contain {message_fragment:?}"
                ));
            }
            Ok(())
        }
        other => Err(format!("{context}: expected FatalModule, got {other:?}")),
    }
}

fn missing_slice_is_fatal() {
    let perimeter = perimeter_of(vec![control_donor(
        0,
        false,
        vec![unit_rect(0, 0, 10, 10).to_expolygon()],
    )]);
    let mut arena = LayerArena::new();
    arena
        .set_perimeter(perimeter.clone())
        .expect("stage a perimeter without a slice");
    let (result, _warnings) = run_sync(&mut arena, 0);
    let error = result.expect_err("a missing staged SliceIR must be fatal");
    assert_fatal_module(&error, "no staged SliceIR", "missing slice")
        .expect("missing SliceIR must carry the hook's FatalModule identity");
    assert!(
        arena.slice().is_none(),
        "the failed take must not invent a staged slice"
    );
    assert_eq!(
        arena.perimeter(),
        Some(&perimeter),
        "a missing-slice rejection must not touch the staged perimeter"
    );
}

fn missing_perimeter_is_fatal_and_restores_the_taken_slice() {
    let polygons = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let mut region = control_region(OBJECT_A, 0, Vec::new(), polygons.clone());
    region.bridge_areas = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    region.top_solid_fill = vec![unit_rect(0, 2, 10, 4).to_expolygon()];
    let entry = slice_of(vec![region]);
    let band = vec![unit_rect(0, 0, 10, 4).to_expolygon()];
    let mut by_layer = HashMap::new();
    by_layer.insert(
        0u32,
        vec![QuartileBand {
            quartile: 1,
            polygons: band.clone(),
        }],
    );
    let mut overhang_quartile_polygons = HashMap::new();
    overhang_quartile_polygons.insert(OBJECT_A.to_string(), by_layer);
    let classification = SurfaceClassificationIR {
        overhang_quartile_polygons,
        ..SurfaceClassificationIR::default()
    };
    let mut arena = LayerArena::new();
    arena
        .set_slice(entry.clone())
        .expect("stage a slice without a perimeter");
    let (result, _warnings) = run_sync(&mut arena, 0);
    let error = result.expect_err("a missing staged PerimeterIR must be fatal");
    assert_fatal_module(&error, "no staged PerimeterIR", "missing perimeter")
        .expect("missing PerimeterIR must carry the hook's FatalModule identity");
    assert_eq!(
        arena.slice(),
        Some(&entry),
        "the hook must re-stage the entry-state slice it took"
    );
    assert!(
        arena.perimeter().is_none(),
        "a missing-perimeter rejection cannot invent staged perimeter data"
    );
    let prepared = arena
        .ensure_prepared_regions(Some(&classification))
        .expect("prepared views re-derive from the restored slice")
        .to_vec();
    let mut output_fields: Vec<&[ExPolygon]> = prepared
        .iter()
        .map(|record| record.overhang_areas.as_slice())
        .collect();
    output_fields.push(entry.regions[0].polygons.as_slice());
    let grid = control_grid(
        &[&polygons, &band, &entry.regions[0].polygons],
        &output_fields,
    );
    assert_prepared_against_slice(&grid, &entry, &prepared, &band, "post-rejection")
        .expect("prepared views after rejection must re-associate with the restored slice");
}

fn missing_individual_donor_is_not_the_fatal_error() {
    let shared_polygons = vec![unit_rect(0, 0, 10, 10).to_expolygon()];
    let chain_one = chain_for(1);
    let mut base = control_region(OBJECT_A, 0, Vec::new(), shared_polygons.clone());
    base.bridge_areas = vec![unit_rect(0, 0, 10, 2).to_expolygon()];
    base.top_solid_fill = vec![unit_rect(0, 2, 10, 4).to_expolygon()];
    let mut virtual_variant = control_region(OBJECT_A, 0, chain_one, shared_polygons.clone());
    virtual_variant.bridge_areas = vec![unit_rect(0, 4, 10, 6).to_expolygon()];
    virtual_variant.top_solid_fill = vec![unit_rect(0, 6, 10, 8).to_expolygon()];
    let input_variant = virtual_variant.clone();
    let (mut arena, _) = stage(
        slice_of(vec![base, virtual_variant]),
        perimeter_of(vec![control_donor(0, false, shared_polygons)]),
    );
    let warnings = sync_ok(&mut arena);
    let observed = arena.slice().expect("slice stays staged").regions.clone();
    assert_eq!(
        observed[1], input_variant,
        "a virtual variant without its own donor must keep its prepass roles byte-for-byte"
    );
    assert!(
        warnings.iter().any(|line| {
            line.contains("no PerimeterIR entry for SliceIR region")
                && line.contains("object_id='partition-property-object-a'")
                && line.contains("region_id='0'")
        }),
        "the per-region absence warning must name the object and region, captured {warnings:?}"
    );
    assert_ne!(
        observed[0].bottom_solid_fill, observed[0].top_solid_fill,
        "the base region must be partitioned, so the scene is not an empty population"
    );
}

fn domain_helpers_fail_closed() {
    for raw in [0u64, MIN_ABS_UNITS - 1, MAX_ABS_UNITS + 1, u64::MAX] {
        let error =
            checked_max_abs_units(raw).expect_err("out-of-ceiling max_abs_units must fail closed");
        assert_eq!(
            error.kind, "max_abs_units_out_of_ceiling",
            "unexpected error kind for {raw}: {error:?}"
        );
    }
    assert!(checked_max_abs_units(MIN_ABS_UNITS).is_ok());
    assert!(checked_max_abs_units(MAX_ABS_UNITS).is_ok());

    for raw in [0u64, 15, MAX_CELLS + 1, u64::MAX] {
        let error = checked_max_cells(raw).expect_err("out-of-ceiling max_cells must fail closed");
        assert_eq!(
            error.kind, "max_cells_out_of_ceiling",
            "unexpected error kind for {raw}: {error:?}"
        );
    }
    assert!(checked_max_cells(16).is_ok());
    assert!(checked_max_cells(MAX_CELLS).is_ok());

    let overflow = checked_double(i64::MAX).expect_err("i64::MAX cannot be doubled");
    assert_eq!(overflow.kind, "coordinate_overflow");
    let underflow = checked_double(i64::MIN).expect_err("i64::MIN cannot be doubled");
    assert_eq!(underflow.kind, "coordinate_overflow");

    // A grid product over the cell budget fails before allocating.
    let xs: Vec<i64> = (0..=64).map(|index| index * 2).collect();
    let oversized = CellGrid::from_boundaries(xs.iter().copied(), [0, 1], 16)
        .expect_err("an oversized grid product must report exhaustion");
    assert_eq!(oversized.kind, "cell_budget_exceeded");

    // An axis with a single boundary carries no witness cell.
    let empty_grid = CellGrid::from_boundaries([0], [0, 1], 16)
        .expect_err("an empty witness grid must be rejected");
    assert_eq!(empty_grid.kind, "empty_witness_grid");

    // Registry-bound validation runs through the same helpers before narrowing.
    assert!(checked_partition_limits(1, 512, 512).is_err());
    assert!(checked_partition_limits(5, 512, 512).is_err());
    assert!(checked_partition_limits(3, 0, 512).is_err());
    assert!(checked_partition_limits(3, 512, 8192).is_err());
    assert_eq!(
        checked_partition_limits(2, MIN_ABS_UNITS, 16),
        Ok(PartitionLimits {
            max_regions: 2,
            max_abs_units: 16,
            max_cells: 16,
        })
    );
}
