//! Exact spatial-query acceleration for perimeter regions.
//!
//! The trees in this module only reduce the set of candidates.  The legacy
//! predicates remain the oracle for every candidate and for every fallback.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};

use rstar::{PointDistance, RTree, RTreeObject, RTreeParams, AABB};
use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{ExPolygon, Point2, Point3WithWidth, Polygon};

use crate::perimeter_utils::{point_in_any_polygon, signed_distance_to_boundary};

const RSTAR_MAX_SIZE: usize = <rstar::DefaultParams as RTreeParams>::MAX_SIZE;

/// A small, owned projection of a prepared perimeter region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionCaptureRecord {
    /// Zero-based layer index.
    pub layer_index: usize,
    /// Zero-based region ordinal within the layer.
    pub region_ordinal: usize,
    /// Number of contour polygons in the region.
    pub contour_count: usize,
    /// Number of holes in the region.
    pub hole_count: usize,
    /// Number of bridge polygons in the region.
    pub bridge_count: usize,
}

/// Selects which path annotations the indexed path should emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathAnnotationMode {
    /// Emit quartile and previous-layer distance annotations.
    Planar,
    /// Emit previous-layer distance annotations without quartiles.
    NonPlanarNoQuartile,
}

#[derive(Debug, Clone, Copy)]
struct Dispatch {
    indexed: bool,
    inject_pruning_fault: bool,
}

#[cfg(feature = "perimeter-spatial-test-support")]
#[derive(Debug)]
struct DiagnosticState {
    accelerated: bool,
    inject_pruning_fault: bool,
    counters: QueryCounters,
    prepared_regions: Vec<RegionCaptureRecord>,
    distance_selections: Vec<Option<usize>>,
    context_rows: Vec<ContextRow>,
}

#[cfg(feature = "perimeter-spatial-test-support")]
#[derive(Debug, Default, Clone, Copy)]
struct ContextRow {
    queries: usize,
    exact_evaluations: usize,
}

/// Test-only counters for the spatial query dispatch.
#[cfg(feature = "perimeter-spatial-test-support")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct QueryCounters {
    /// Number of queries that used an indexed tree.
    pub indexed_queries: usize,
    /// Number of queries that used a complete or linear legacy evaluator.
    pub legacy_queries: usize,
    /// Number of queries that fell back from the indexed implementation.
    pub fallback_queries: usize,
    /// Number of exact legacy-predicate re-evaluations for indexed candidates.
    pub exact_evaluations: usize,
    /// Number of prepared spatial contexts constructed.
    pub contexts_constructed: usize,
}

/// Per-context indexed-query accounting for pruning diagnostics.
#[cfg(feature = "perimeter-spatial-test-support")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ContextAccountingRow {
    /// Number of completed indexed `signed_distance_to_boundary` queries
    /// issued while this context was current.
    pub queries: usize,
    /// Number of exact distance-edge candidate re-evaluations for this
    /// context.
    pub exact_evaluations: usize,
}

#[cfg(feature = "perimeter-spatial-test-support")]
std::thread_local! {
    static DIAGNOSTIC_STATE: std::cell::RefCell<Option<DiagnosticState>> =
        const { std::cell::RefCell::new(None) };
}

/// Observe a prepared region when test support is enabled.
pub fn observe_prepared_region(record: RegionCaptureRecord) {
    #[cfg(feature = "perimeter-spatial-test-support")]
    DIAGNOSTIC_STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.prepared_regions.push(record);
        }
    });

    #[cfg(not(feature = "perimeter-spatial-test-support"))]
    let _ = record;
}

/// Test-support hooks for the prepared-region spatial context.
pub mod diagnostics {
    pub use super::{observe_prepared_region, RegionCaptureRecord};

    #[cfg(feature = "perimeter-spatial-test-support")]
    pub use super::{
        with_capture, with_context_accounting, with_counters,
        with_counters_and_distance_selections, ContextAccountingRow, QueryCounters,
    };
}

#[cfg(feature = "perimeter-spatial-test-support")]
fn restore_diagnostic_state(
    previous: Option<DiagnosticState>,
    child_context_rows: Vec<ContextRow>,
) {
    DIAGNOSTIC_STATE.with(|state| {
        let mut previous = previous;
        if let Some(parent) = previous.as_mut() {
            let had_child_rows = !child_context_rows.is_empty();
            parent.context_rows.extend(child_context_rows);
            if had_child_rows {
                // Push-on-construction semantics make the last row current for
                // any subsequent query in the parent diagnostic scope.
                debug_assert!(!parent.context_rows.is_empty());
            }
        }
        *state.borrow_mut() = previous;
    });
}

/// Run `f` with thread-local dispatch and query counters enabled.
#[cfg(feature = "perimeter-spatial-test-support")]
pub fn with_counters<R>(
    accelerated: bool,
    inject_pruning_fault: bool,
    f: impl FnOnce() -> R,
) -> (R, QueryCounters) {
    let previous = DIAGNOSTIC_STATE.with(|state| {
        state.borrow_mut().replace(DiagnosticState {
            accelerated,
            inject_pruning_fault,
            counters: QueryCounters::default(),
            prepared_regions: Vec::new(),
            distance_selections: Vec::new(),
            context_rows: Vec::new(),
        })
    });

    let result = catch_unwind(AssertUnwindSafe(f));
    let current = DIAGNOSTIC_STATE.with(|state| state.borrow_mut().take());
    let current = current.expect("perimeter spatial diagnostics state was cleared unexpectedly");
    let counters = current.counters;
    restore_diagnostic_state(previous, current.context_rows);

    match result {
        Ok(result) => (result, counters),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Run `f` with thread-local dispatch, query counters, and prepared-region
/// capture enabled.
#[cfg(feature = "perimeter-spatial-test-support")]
pub fn with_capture<R>(
    accelerated: bool,
    inject_pruning_fault: bool,
    f: impl FnOnce() -> R,
) -> (R, QueryCounters, Vec<RegionCaptureRecord>) {
    let previous = DIAGNOSTIC_STATE.with(|state| {
        state.borrow_mut().replace(DiagnosticState {
            accelerated,
            inject_pruning_fault,
            counters: QueryCounters::default(),
            prepared_regions: Vec::new(),
            distance_selections: Vec::new(),
            context_rows: Vec::new(),
        })
    });

    let result = catch_unwind(AssertUnwindSafe(f));
    let current = DIAGNOSTIC_STATE.with(|state| state.borrow_mut().take());
    let current = current.expect("perimeter spatial diagnostics state was cleared unexpectedly");
    let counters = current.counters;
    let prepared_regions = current.prepared_regions;
    restore_diagnostic_state(previous, current.context_rows);

    match result {
        Ok(result) => (result, counters, prepared_regions),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Run `f` with thread-local dispatch, query counters, and distance-selection
/// capture enabled.
///
/// This is the selection-aware sibling of [`with_counters`]. The existing
/// counter-only function intentionally keeps its `(R, QueryCounters)` shape;
/// this additive hook returns one optional source ordinal for each completed
/// distance query.
#[cfg(feature = "perimeter-spatial-test-support")]
pub fn with_counters_and_distance_selections<R>(
    accelerated: bool,
    inject_pruning_fault: bool,
    f: impl FnOnce() -> R,
) -> (R, QueryCounters, Vec<Option<usize>>) {
    let previous = DIAGNOSTIC_STATE.with(|state| {
        state.borrow_mut().replace(DiagnosticState {
            accelerated,
            inject_pruning_fault,
            counters: QueryCounters::default(),
            prepared_regions: Vec::new(),
            distance_selections: Vec::new(),
            context_rows: Vec::new(),
        })
    });

    let result = catch_unwind(AssertUnwindSafe(f));
    let current = DIAGNOSTIC_STATE.with(|state| state.borrow_mut().take());
    let current = current.expect("perimeter spatial diagnostics state was cleared unexpectedly");
    let counters = current.counters;
    let distance_selections = current.distance_selections;
    restore_diagnostic_state(previous, current.context_rows);

    match result {
        Ok(result) => (result, counters, distance_selections),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Run `f` with thread-local per-context indexed-query accounting enabled.
#[cfg(feature = "perimeter-spatial-test-support")]
pub fn with_context_accounting<R>(
    accelerated: bool,
    inject_pruning_fault: bool,
    f: impl FnOnce() -> R,
) -> (R, Vec<ContextAccountingRow>) {
    let previous = DIAGNOSTIC_STATE.with(|state| {
        state.borrow_mut().replace(DiagnosticState {
            accelerated,
            inject_pruning_fault,
            counters: QueryCounters::default(),
            prepared_regions: Vec::new(),
            distance_selections: Vec::new(),
            context_rows: Vec::new(),
        })
    });

    let result = catch_unwind(AssertUnwindSafe(f));
    let current = DIAGNOSTIC_STATE.with(|state| state.borrow_mut().take());
    let current = current.expect("perimeter spatial diagnostics state was cleared unexpectedly");
    let context_rows = current
        .context_rows
        .iter()
        .map(|row| ContextAccountingRow {
            queries: row.queries,
            exact_evaluations: row.exact_evaluations,
        })
        .collect();
    restore_diagnostic_state(previous, current.context_rows);

    match result {
        Ok(result) => (result, context_rows),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[cfg(feature = "perimeter-spatial-test-support")]
fn dispatch() -> Dispatch {
    DIAGNOSTIC_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| Dispatch {
                indexed: state.accelerated,
                inject_pruning_fault: state.inject_pruning_fault,
            })
            .unwrap_or_else(default_dispatch)
    })
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn dispatch() -> Dispatch {
    default_dispatch()
}

fn default_dispatch() -> Dispatch {
    #[cfg(pnp_perimeter_spatial_accelerated)]
    {
        Dispatch {
            indexed: true,
            inject_pruning_fault: false,
        }
    }

    #[cfg(not(pnp_perimeter_spatial_accelerated))]
    {
        Dispatch {
            indexed: false,
            inject_pruning_fault: false,
        }
    }
}

#[cfg(feature = "perimeter-spatial-test-support")]
fn count_indexed_distance_query() {
    DIAGNOSTIC_STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.indexed_queries += 1;
            if let Some(row) = state.context_rows.last_mut() {
                row.queries += 1;
            }
        }
    });
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn count_indexed_distance_query() {}

#[cfg(feature = "perimeter-spatial-test-support")]
fn count_indexed() {
    DIAGNOSTIC_STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.indexed_queries += 1;
        }
    });
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn count_indexed() {}

#[cfg(feature = "perimeter-spatial-test-support")]
fn count_legacy() {
    DIAGNOSTIC_STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.legacy_queries += 1;
        }
    });
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn count_legacy() {}

#[cfg(feature = "perimeter-spatial-test-support")]
fn count_fallback() {
    DIAGNOSTIC_STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.fallback_queries += 1;
        }
    });
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn count_fallback() {}

#[cfg(feature = "perimeter-spatial-test-support")]
fn count_distance_exact_evaluation() {
    DIAGNOSTIC_STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.exact_evaluations += 1;
            if let Some(row) = state.context_rows.last_mut() {
                row.exact_evaluations += 1;
            }
        }
    });
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn count_distance_exact_evaluation() {}

#[cfg(feature = "perimeter-spatial-test-support")]
fn count_exact_evaluation() {
    DIAGNOSTIC_STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.exact_evaluations += 1;
        }
    });
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn count_exact_evaluation() {}

#[cfg(feature = "perimeter-spatial-test-support")]
fn count_context_constructed() {
    DIAGNOSTIC_STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.context_rows.push(ContextRow::default());
            state.counters.contexts_constructed += 1;
        }
    });
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn count_context_constructed() {}

#[cfg(feature = "perimeter-spatial-test-support")]
fn capture_distance_selection(selection: Option<usize>) {
    DIAGNOSTIC_STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.distance_selections.push(selection);
        }
    });
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn capture_distance_selection(_selection: Option<usize>) {}

#[cfg(feature = "perimeter-spatial-test-support")]
fn capture_legacy_distance_selection(x: f32, y: f32, boundary: &[ExPolygon]) {
    capture_distance_selection(legacy_distance_ordinal(x, y, boundary));
}

#[cfg(not(feature = "perimeter-spatial-test-support"))]
fn capture_legacy_distance_selection(_x: f32, _y: f32, _boundary: &[ExPolygon]) {}

#[derive(Debug)]
enum IndexState<T: RTreeObject> {
    Linear,
    Indexed(RTree<T>),
    Failed,
}

fn build_index<T: RTreeObject>(records: Vec<T>) -> IndexState<T> {
    if records.len() <= RSTAR_MAX_SIZE {
        return IndexState::Linear;
    }

    match catch_unwind(AssertUnwindSafe(|| RTree::bulk_load(records))) {
        Ok(tree) => IndexState::Indexed(tree),
        Err(_) => IndexState::Failed,
    }
}

fn index_failed<T: RTreeObject>(index: &IndexState<T>) -> bool {
    matches!(index, IndexState::Failed)
}

#[derive(Debug, Clone, Copy)]
struct DistanceEdgeRecord {
    a: [f64; 2],
    b: [f64; 2],
    source_ordinal: usize,
    envelope: AABB<[f64; 2]>,
}

impl RTreeObject for DistanceEdgeRecord {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

impl PointDistance for DistanceEdgeRecord {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        edge_distance_sq(self, point[0], point[1])
    }
}

#[derive(Debug, Clone, Copy)]
struct BoundaryYRecord {
    polygon_index: usize,
    envelope: AABB<[f64; 2]>,
}

impl RTreeObject for BoundaryYRecord {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

#[derive(Debug, Clone, Copy)]
struct QuartileYRecord {
    band_index: usize,
    polygon_index: usize,
    envelope: AABB<[f64; 2]>,
}

impl RTreeObject for QuartileYRecord {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

#[derive(Debug, Clone, Copy)]
struct BridgeRecord {
    polygon_index: usize,
    envelope: AABB<[i64; 2]>,
}

impl RTreeObject for BridgeRecord {
    type Envelope = AABB<[i64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

#[derive(Debug, Clone, Copy)]
struct BridgeBounds {
    min_x: i64,
    max_x: i64,
    min_y: i64,
    max_y: i64,
    span_x: u128,
    span_y: u128,
}

/// Immutable indexes and source geometry for one perimeter region.
pub struct PerimeterSpatialContext {
    boundary: Vec<ExPolygon>,
    overhang_bands: Vec<QuartileBand>,
    bridge_areas: Vec<ExPolygon>,
    distance_index: IndexState<DistanceEdgeRecord>,
    boundary_y_index: IndexState<BoundaryYRecord>,
    quartile_y_index: IndexState<QuartileYRecord>,
    bridge_index: IndexState<BridgeRecord>,
    bridge_bounds: Option<BridgeBounds>,
    construction_failed: bool,
}

impl PerimeterSpatialContext {
    /// Prepare exact candidate-reduction indexes for a perimeter region.
    pub fn new(
        boundary: &[ExPolygon],
        overhang_bands: &[QuartileBand],
        bridge_areas: &[ExPolygon],
    ) -> Self {
        count_context_constructed();
        let boundary = boundary.to_vec();
        let overhang_bands = overhang_bands.to_vec();
        let bridge_areas = bridge_areas.to_vec();

        let distance_index = build_index(distance_records(&boundary));
        let boundary_y_index = build_index(boundary_y_records(&boundary));
        let quartile_y_index = build_index(quartile_y_records(&overhang_bands));
        let (bridge_records, bridge_bounds) = bridge_records(&bridge_areas);
        let bridge_index = build_index(bridge_records);
        let construction_failed = index_failed(&distance_index)
            || index_failed(&boundary_y_index)
            || index_failed(&quartile_y_index)
            || index_failed(&bridge_index);

        Self {
            boundary,
            overhang_bands,
            bridge_areas,
            distance_index,
            boundary_y_index,
            quartile_y_index,
            bridge_index,
            bridge_bounds,
            construction_failed,
        }
    }

    /// Return the exact signed distance to the nearest boundary edge.
    pub fn signed_distance_to_boundary(&self, x: f32, y: f32) -> f32 {
        let dispatch = dispatch();
        if !dispatch.indexed {
            count_legacy();
            let distance = if self.boundary.is_empty() {
                0.0
            } else {
                signed_distance_to_boundary(x, y, &self.boundary)
            };
            capture_legacy_distance_selection(x, y, &self.boundary);
            return distance;
        }
        if dispatch.inject_pruning_fault
            || self.construction_failed
            || !x.is_finite()
            || !y.is_finite()
            || self.boundary.is_empty()
            || index_failed(&self.distance_index)
            || index_failed(&self.boundary_y_index)
            || matches!(&self.distance_index, IndexState::Linear)
            || matches!(&self.boundary_y_index, IndexState::Linear)
        {
            return self.fallback_signed_distance(x, y);
        }

        let (distance, source_ordinal) = match &self.distance_index {
            IndexState::Linear => return self.fallback_signed_distance(x, y),
            IndexState::Indexed(tree) => match indexed_distance(tree, x, y) {
                Some(distance) => distance,
                None => return self.fallback_signed_distance(x, y),
            },
            IndexState::Failed => return self.fallback_signed_distance(x, y),
        };

        let inside = match &self.boundary_y_index {
            IndexState::Linear => return self.fallback_signed_distance(x, y),
            IndexState::Indexed(tree) => match indexed_inside(tree, x, y, &self.boundary) {
                Some(inside) => inside,
                None => return self.fallback_signed_distance(x, y),
            },
            IndexState::Failed => return self.fallback_signed_distance(x, y),
        };

        // One public query, one count.  The nearest-seed lookup, the
        // distance-envelope intersect, and the inside-envelope intersect are
        // internal tree operations and must not each increment the counter.
        count_indexed_distance_query();
        capture_distance_selection(Some(source_ordinal));
        if inside {
            -distance
        } else {
            distance
        }
    }

    /// Return the most severe overhang quartile containing `(x, y)`.
    pub fn overhang_quartile(&self, x: f32, y: f32) -> Option<u8> {
        let dispatch = dispatch();
        if !dispatch.indexed {
            count_legacy();
            return legacy_overhang_quartile(x, y, &self.overhang_bands);
        }
        if dispatch.inject_pruning_fault
            || self.construction_failed
            || !x.is_finite()
            || !y.is_finite()
            || index_failed(&self.quartile_y_index)
            || matches!(&self.quartile_y_index, IndexState::Linear)
        {
            return self.fallback_overhang_quartile(x, y);
        }

        match &self.quartile_y_index {
            IndexState::Linear => self.fallback_overhang_quartile(x, y),
            IndexState::Indexed(tree) => {
                let quartile = match indexed_overhang_quartile(tree, x, y, &self.overhang_bands) {
                    Some(quartile) => quartile,
                    None => return self.fallback_overhang_quartile(x, y),
                };
                count_indexed();
                quartile
            }
            IndexState::Failed => self.fallback_overhang_quartile(x, y),
        }
    }

    /// Return whether an integer point is strictly inside any bridge area.
    pub fn is_bridge(&self, point: &Point2) -> bool {
        let dispatch = dispatch();
        if !dispatch.indexed {
            count_legacy();
            return point_in_any_polygon(point, &self.bridge_areas);
        }
        if dispatch.inject_pruning_fault
            || self.construction_failed
            || index_failed(&self.bridge_index)
            || matches!(&self.bridge_index, IndexState::Linear)
        {
            return self.fallback_bridge(point);
        }

        match &self.bridge_index {
            IndexState::Linear => self.fallback_bridge(point),
            IndexState::Indexed(tree) => {
                let Some(bounds) = self.bridge_bounds else {
                    return self.fallback_bridge(point);
                };
                // This guard intentionally precedes the spatial bbox query.  The
                // original predicate uses wrapping i64 arithmetic in release and
                // panics on overflow in debug, so unsafe extents must use it whole.
                if !bridge_arithmetic_safe(bounds, point) {
                    return self.fallback_bridge(point);
                }

                count_indexed();
                indexed_bridge(tree, point, &self.bridge_areas)
            }
            IndexState::Failed => self.fallback_bridge(point),
        }
    }

    fn fallback_signed_distance(&self, x: f32, y: f32) -> f32 {
        count_fallback();
        count_legacy();
        let distance = if self.boundary.is_empty() {
            0.0
        } else {
            signed_distance_to_boundary(x, y, &self.boundary)
        };
        capture_legacy_distance_selection(x, y, &self.boundary);
        distance
    }

    fn fallback_overhang_quartile(&self, x: f32, y: f32) -> Option<u8> {
        count_fallback();
        count_legacy();
        legacy_overhang_quartile(x, y, &self.overhang_bands)
    }

    fn fallback_bridge(&self, point: &Point2) -> bool {
        count_fallback();
        count_legacy();
        point_in_any_polygon(point, &self.bridge_areas)
    }
}

/// Convert a contour into the same closed path as the legacy helper, using the
/// prepared context for its per-vertex annotations.
pub fn expolygon_to_path3d_indexed(
    contour: &Polygon,
    z: f32,
    width: f32,
    context: &PerimeterSpatialContext,
    mode: PathAnnotationMode,
) -> Vec<Point3WithWidth> {
    let mut points: Vec<Point3WithWidth> = contour
        .points
        .iter()
        .map(|point| {
            let x = slicer_ir::units_to_mm(point.x);
            let y = slicer_ir::units_to_mm(point.y);
            let overhang_quartile = match mode {
                PathAnnotationMode::Planar => context.overhang_quartile(x, y),
                PathAnnotationMode::NonPlanarNoQuartile => None,
            };
            let overhang_distance_mm = if context.boundary.is_empty() {
                None
            } else {
                Some(context.signed_distance_to_boundary(x, y) + 0.5 * width)
            };
            Point3WithWidth {
                x,
                y,
                z,
                width,
                flow_factor: 1.0,
                overhang_quartile,
                dist_to_top_mm: 0.0,
                overhang_distance_mm,
            }
        })
        .collect();

    if let Some(first) = points.first().cloned() {
        points.push(first);
    }
    points
}

fn distance_records(boundary: &[ExPolygon]) -> Vec<DistanceEdgeRecord> {
    let mut records = Vec::new();
    let mut source_ordinal = 0;
    for expolygon in boundary {
        for polygon in std::iter::once(&expolygon.contour).chain(expolygon.holes.iter()) {
            let point_count = polygon.points.len();
            if point_count == 0 {
                continue;
            }
            for edge_index in 0..point_count {
                let a = distance_point(polygon.points[edge_index]);
                let b = distance_point(polygon.points[(edge_index + 1) % point_count]);
                records.push(DistanceEdgeRecord {
                    a,
                    b,
                    source_ordinal,
                    envelope: distance_envelope(a, b),
                });
                source_ordinal += 1;
            }
        }
    }
    records
}

fn boundary_y_records(boundary: &[ExPolygon]) -> Vec<BoundaryYRecord> {
    let mut records = Vec::new();
    for (polygon_index, expolygon) in boundary.iter().enumerate() {
        let points = &expolygon.contour.points;
        for edge_index in 0..points.len() {
            records.push(BoundaryYRecord {
                polygon_index,
                envelope: y_interval_envelope(
                    points[edge_index],
                    points[(edge_index + 1) % points.len()],
                ),
            });
        }
    }
    records
}

fn quartile_y_records(bands: &[QuartileBand]) -> Vec<QuartileYRecord> {
    let mut records = Vec::new();
    for (band_index, band) in bands.iter().enumerate() {
        for (polygon_index, expolygon) in band.polygons.iter().enumerate() {
            let points = &expolygon.contour.points;
            for edge_index in 0..points.len() {
                records.push(QuartileYRecord {
                    band_index,
                    polygon_index,
                    envelope: y_interval_envelope(
                        points[edge_index],
                        points[(edge_index + 1) % points.len()],
                    ),
                });
            }
        }
    }
    records
}

fn bridge_records(areas: &[ExPolygon]) -> (Vec<BridgeRecord>, Option<BridgeBounds>) {
    let mut records = Vec::new();
    let mut global_min_x = None;
    let mut global_max_x = None;
    let mut global_min_y = None;
    let mut global_max_y = None;

    for (polygon_index, expolygon) in areas.iter().enumerate() {
        let points = &expolygon.contour.points;
        let Some(first) = points.first() else {
            continue;
        };
        let mut min_x = first.x;
        let mut max_x = first.x;
        let mut min_y = first.y;
        let mut max_y = first.y;
        for point in &points[1..] {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }
        global_min_x = Some(global_min_x.map_or(min_x, |value: i64| value.min(min_x)));
        global_max_x = Some(global_max_x.map_or(max_x, |value: i64| value.max(max_x)));
        global_min_y = Some(global_min_y.map_or(min_y, |value: i64| value.min(min_y)));
        global_max_y = Some(global_max_y.map_or(max_y, |value: i64| value.max(max_y)));
        records.push(BridgeRecord {
            polygon_index,
            envelope: AABB::from_corners([min_x, min_y], [max_x, max_y]),
        });
    }

    let bounds = match (global_min_x, global_max_x, global_min_y, global_max_y) {
        (Some(min_x), Some(max_x), Some(min_y), Some(max_y)) => {
            let span_x = (i128::from(max_x) - i128::from(min_x)) as u128;
            let span_y = (i128::from(max_y) - i128::from(min_y)) as u128;
            Some(BridgeBounds {
                min_x,
                max_x,
                min_y,
                max_y,
                span_x,
                span_y,
            })
        }
        _ => None,
    };
    (records, bounds)
}

#[inline]
fn distance_point(point: Point2) -> [f64; 2] {
    [
        f64::from(slicer_ir::units_to_mm(point.x)),
        f64::from(slicer_ir::units_to_mm(point.y)),
    ]
}

#[inline]
fn winding_coordinate(units: i64) -> f64 {
    units as f64 / 10_000.0
}

fn distance_envelope(a: [f64; 2], b: [f64; 2]) -> AABB<[f64; 2]> {
    // Keep this endpoint expression distinct from the raw b coordinate.  The
    // exact segment evaluator also uses a + (b - a) as its effective endpoint.
    let effective_b = [a[0] + (b[0] - a[0]), a[1] + (b[1] - a[1])];
    AABB::from_corners(
        [a[0].min(effective_b[0]), a[1].min(effective_b[1])],
        [a[0].max(effective_b[0]), a[1].max(effective_b[1])],
    )
}

fn y_interval_envelope(a: Point2, b: Point2) -> AABB<[f64; 2]> {
    let a_y = winding_coordinate(a.y);
    let b_y = winding_coordinate(b.y);
    let computed_endpoint = a_y + (b_y - a_y);
    let min_y = a_y.min(b_y).min(computed_endpoint);
    let max_y = a_y.max(b_y).max(computed_endpoint);
    let expansion = next_up(f64::from_bits(1).sqrt());
    let lower_y = next_down(min_y - expansion);
    let upper_y = next_up(max_y + expansion);
    AABB::from_corners([0.0, lower_y], [0.0, upper_y])
}

#[inline]
fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value.is_sign_positive() {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

#[inline]
fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value.is_sign_positive() {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

fn edge_distance_sq(record: &DistanceEdgeRecord, query_x: f64, query_y: f64) -> f64 {
    let ax = record.a[0];
    let ay = record.a[1];
    let bx = record.b[0];
    let by = record.b[1];
    let dx = bx - ax;
    let dy = by - ay;
    let length_sq = dx * dx + dy * dy;
    let projection = if length_sq == 0.0 {
        0.0
    } else {
        ((query_x - ax) * dx + (query_y - ay) * dy) / length_sq
    };
    let projection = projection.clamp(0.0, 1.0);
    let closest_x = ax + projection * dx;
    let closest_y = ay + projection * dy;
    (query_x - closest_x).powi(2) + (query_y - closest_y).powi(2)
}

fn indexed_distance(tree: &RTree<DistanceEdgeRecord>, x: f32, y: f32) -> Option<(f32, usize)> {
    let query_x = f64::from(x);
    let query_y = f64::from(y);
    let query = [query_x, query_y];
    let seed = tree.nearest_neighbor(&query)?;
    let seed_distance_sq = edge_distance_sq(seed, query_x, query_y);
    let radius = next_up(next_up(seed_distance_sq.next_up().sqrt()));
    let envelope = AABB::from_corners(
        [next_down(query_x - radius), next_down(query_y - radius)],
        [next_up(query_x + radius), next_up(query_y + radius)],
    );

    let mut best: Option<(f64, usize)> = None;
    for candidate in tree.locate_in_envelope_intersecting(&envelope) {
        count_distance_exact_evaluation();
        let distance_sq = edge_distance_sq(candidate, query_x, query_y);
        let replace = match best {
            None => true,
            Some((best_distance_sq, best_ordinal)) => {
                distance_sq.total_cmp(&best_distance_sq) == Ordering::Less
                    || (distance_sq.total_cmp(&best_distance_sq) == Ordering::Equal
                        && candidate.source_ordinal < best_ordinal)
            }
        };
        if replace {
            best = Some((distance_sq, candidate.source_ordinal));
        }
    }
    best.map(|(distance_sq, source_ordinal)| (distance_sq.sqrt() as f32, source_ordinal))
}

fn indexed_inside(
    tree: &RTree<BoundaryYRecord>,
    x: f32,
    y: f32,
    boundary: &[ExPolygon],
) -> Option<bool> {
    let query = [0.0, f64::from(y)];
    let envelope = AABB::from_corners(query, query);
    let mut candidates = BTreeSet::new();
    for record in tree.locate_in_envelope_intersecting(&envelope) {
        candidates.insert(record.polygon_index);
    }

    let mut inside = false;
    for polygon_index in candidates {
        let polygon = boundary.get(polygon_index)?;
        count_exact_evaluation();
        if slicer_ir::point_in_polygon_winding(polygon, f64::from(x), f64::from(y), 0.0) {
            inside = true;
        }
    }
    Some(inside)
}

fn indexed_overhang_quartile(
    tree: &RTree<QuartileYRecord>,
    x: f32,
    y: f32,
    bands: &[QuartileBand],
) -> Option<Option<u8>> {
    let query = [0.0, f64::from(y)];
    let envelope = AABB::from_corners(query, query);
    let mut candidates = BTreeSet::new();
    for record in tree.locate_in_envelope_intersecting(&envelope) {
        candidates.insert((record.band_index, record.polygon_index));
    }

    let mut quartile: Option<u8> = None;
    for (band_index, polygon_index) in candidates {
        let band = bands.get(band_index)?;
        let polygon = band.polygons.get(polygon_index)?;
        count_exact_evaluation();
        if slicer_ir::point_in_polygon_winding(polygon, f64::from(x), f64::from(y), 0.0) {
            quartile = Some(quartile.map_or(band.quartile, |current| current.max(band.quartile)));
        }
    }
    Some(quartile)
}

fn indexed_bridge(tree: &RTree<BridgeRecord>, point: &Point2, areas: &[ExPolygon]) -> bool {
    let query = [point.x, point.y];
    let envelope = AABB::from_corners(query, query);
    let mut candidates = BTreeSet::new();
    for record in tree.locate_in_envelope_intersecting(&envelope) {
        candidates.insert(record.polygon_index);
    }

    let mut inside = false;
    for polygon_index in candidates {
        let Some(polygon) = areas.get(polygon_index) else {
            continue;
        };
        count_exact_evaluation();
        if point_in_any_polygon(point, std::slice::from_ref(polygon)) {
            inside = true;
        }
    }
    inside
}

#[cfg(feature = "perimeter-spatial-test-support")]
fn legacy_distance_ordinal(x: f32, y: f32, boundary: &[ExPolygon]) -> Option<usize> {
    let query_x = f64::from(x);
    let query_y = f64::from(y);
    let mut nearest: Option<(f64, usize)> = None;
    let mut source_ordinal = 0;
    for expolygon in boundary {
        for polygon in std::iter::once(&expolygon.contour).chain(expolygon.holes.iter()) {
            let point_count = polygon.points.len();
            if point_count == 0 {
                continue;
            }
            for edge_index in 0..point_count {
                let a = distance_point(polygon.points[edge_index]);
                let b = distance_point(polygon.points[(edge_index + 1) % point_count]);
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let length_sq = dx * dx + dy * dy;
                let projection = if length_sq == 0.0 {
                    0.0
                } else {
                    ((query_x - a[0]) * dx + (query_y - a[1]) * dy) / length_sq
                };
                let projection = projection.clamp(0.0, 1.0);
                let closest_x = a[0] + projection * dx;
                let closest_y = a[1] + projection * dy;
                let distance_sq = (query_x - closest_x).powi(2) + (query_y - closest_y).powi(2);
                let replace = match nearest {
                    None => true,
                    Some((best_distance_sq, best_ordinal)) => {
                        distance_sq.total_cmp(&best_distance_sq) == Ordering::Less
                            || (distance_sq.total_cmp(&best_distance_sq) == Ordering::Equal
                                && source_ordinal < best_ordinal)
                    }
                };
                if replace {
                    nearest = Some((distance_sq, source_ordinal));
                }
                source_ordinal += 1;
            }
        }
    }
    nearest.map(|(_, source_ordinal)| source_ordinal)
}

fn legacy_overhang_quartile(x: f32, y: f32, bands: &[QuartileBand]) -> Option<u8> {
    bands
        .iter()
        .filter(|band| {
            band.polygons.iter().any(|polygon| {
                slicer_ir::point_in_polygon_winding(polygon, f64::from(x), f64::from(y), 0.0)
            })
        })
        .map(|band| band.quartile)
        .max()
}

fn bridge_arithmetic_safe(bounds: BridgeBounds, point: &Point2) -> bool {
    let max_i64 = i64::MAX as u128;
    if bounds.span_x > max_i64 || bounds.span_y > max_i64 {
        return false;
    }

    let query_x_extent = widened_extent(point.x, bounds.min_x, bounds.max_x);
    let query_y_extent = widened_extent(point.y, bounds.min_y, bounds.max_y);
    if query_x_extent > max_i64 || query_y_extent > max_i64 {
        return false;
    }

    let Some(x_term) = bounds.span_x.checked_mul(query_y_extent) else {
        return false;
    };
    let Some(y_term) = bounds.span_y.checked_mul(query_x_extent) else {
        return false;
    };
    let Some(total) = x_term.checked_add(y_term) else {
        return false;
    };
    total <= max_i64
}

fn widened_extent(query: i64, min: i64, max: i64) -> u128 {
    let query = i128::from(query);
    let left = (query - i128::from(min)).unsigned_abs();
    let right = (query - i128::from(max)).unsigned_abs();
    left.max(right)
}
