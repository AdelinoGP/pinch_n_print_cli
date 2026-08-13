//! Batched host services — the module-owner opt-in for parallelism.
//!
//! # What these actually do
//!
//! A guest cannot spawn threads (ADR-0049), so a module cannot parallelize its
//! own loops. What it *can* do is hand the host a whole list of independent
//! geometry requests in one call. The host then runs them on the rayon pool it
//! already owns, in native code, and returns the results **in input order**.
//!
//! This matters more than "fewer boundary crossings" suggests.
//! [`crate::host::offset_polygons`] and its siblings are **not** host calls
//! today: `slicer_core::polygon_ops` is backed by `clipper2-rust`, which
//! compiles to wasm32, so a guest calling them runs clipper2 *inside the
//! sandbox, single-threaded*. Switching a loop to a batch form moves that work
//! out of wasm into native host code **and** makes it eligible for fan-out.
//!
//! The trade is marshalling: the polygons now cross a boundary they previously
//! never touched. The host's fan-out threshold models compute, not marshalling
//! (see `slicer_wasm_host::batch`). If a batch adoption ever measures *slower*
//! than the loop it replaced, suspect that first.
//!
//! # Determinism
//!
//! Results are returned in input order regardless of how the host scheduled
//! them, so output cannot depend on worker count. Adopting a batch form is
//! still a behavioural change in one respect — the geometry runs host-side
//! rather than guest-side — so adopt one module at a time and keep the
//! canonical parity suite green in between.
//!
//! # Native builds
//!
//! On non-wasm32 targets these run the same operations locally and serially,
//! so module unit tests and native harnesses work without a runtime. **The
//! parallelism is a host-side property and does not appear in native timings.**

use slicer_ir::{ExPolygon, Point3, Polygon};

use crate::host::{ClipOperation, OffsetJoinType};

/// One `offset-polygons` item in a batch.
#[derive(Debug, Clone)]
pub struct OffsetRequest {
    /// Polygons to offset.
    pub polygons: Vec<ExPolygon>,
    /// Offset distance in millimeters; negative shrinks.
    pub delta_mm: f32,
    /// Corner join style.
    pub join: OffsetJoinType,
    /// Arc approximation tolerance in millimeters.
    pub arc_tolerance_mm: f32,
}

/// One `clip-polygons` item in a batch.
#[derive(Debug, Clone)]
pub struct ClipRequest {
    /// Subject polygon set.
    pub subject: Vec<ExPolygon>,
    /// Clip polygon set.
    pub clip: Vec<ExPolygon>,
    /// Boolean operation to apply.
    pub op: ClipOperation,
}

/// One `simplify-polygon` item in a batch.
#[derive(Debug, Clone)]
pub struct SimplifyRequest {
    /// Polygon to simplify.
    pub polygon: Polygon,
    /// Reserved; the current implementation uses exact collinearity.
    pub tolerance_mm: f32,
}

/// One `raycast-z-down` item in a batch.
#[derive(Debug, Clone)]
pub struct RaycastRequest {
    /// Object to cast against.
    pub object_id: String,
    /// Ray X in millimeters.
    pub x: f32,
    /// Ray Y in millimeters.
    pub y: f32,
    /// Z to start the downward cast from, in millimeters.
    pub start_z: f32,
}

/// One `surface-normal-at` item in a batch.
#[derive(Debug, Clone)]
pub struct SurfaceNormalRequest {
    /// Object to query.
    pub object_id: String,
    /// Query X in millimeters.
    pub x: f32,
    /// Query Y in millimeters.
    pub y: f32,
    /// Query Z in millimeters.
    pub z: f32,
}

// ── Alignment-preserving helpers ────────────────────────────────────────
//
// Restructuring a loop into collect / call / distribute is where batch
// adoption goes wrong: it is easy to lose which result belongs to which input.
// These write that bookkeeping once, here, instead of once per module. Use the
// raw `*_batch` functions below when your shape does not fit.

/// Offset one request per item, returning each item paired with its result.
///
/// ```ignore
/// for (entry, inflated) in batch_offset(&support_geometry.entries, |e| OffsetRequest {
///     polygons: e.outlines.clone(),
///     delta_mm: avoid_inflate,
///     join: OffsetJoinType::Miter,
///     arc_tolerance_mm: 0.0,
/// }) {
///     // `inflated` is this `entry`'s result, always.
/// }
/// ```
pub fn batch_offset<T, F>(items: &[T], build: F) -> Vec<(&T, Vec<ExPolygon>)>
where
    F: Fn(&T) -> OffsetRequest,
{
    let requests: Vec<OffsetRequest> = items.iter().map(&build).collect();
    let results = offset_polygons_batch(&requests);
    items.iter().zip(results).collect()
}

/// Clip one request per item, returning each item paired with its result.
pub fn batch_clip<T, F>(items: &[T], build: F) -> Vec<(&T, Vec<ExPolygon>)>
where
    F: Fn(&T) -> ClipRequest,
{
    let requests: Vec<ClipRequest> = items.iter().map(&build).collect();
    let results = clip_polygons_batch(&requests);
    items.iter().zip(results).collect()
}

// ── Raw batch forms ─────────────────────────────────────────────────────

/// Offset several independent polygon sets in one host call.
#[must_use]
pub fn offset_polygons_batch(requests: &[OffsetRequest]) -> Vec<Vec<ExPolygon>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        requests
            .iter()
            .map(|r| {
                crate::host::offset_polygons(&r.polygons, r.delta_mm, r.join, r.arc_tolerance_mm)
            })
            .collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wit::slicer::common::host_services as svc;
        let wit_requests: Vec<svc::OffsetRequest> = requests
            .iter()
            .map(|r| svc::OffsetRequest {
                polygons: r.polygons.iter().map(wit::to_wit_expolygon).collect(),
                delta_mm: r.delta_mm,
                join: wit::to_wit_join(r.join),
                arc_tolerance_mm: r.arc_tolerance_mm,
            })
            .collect();
        svc::offset_polygons_batch(&wit_requests)
            .into_iter()
            .map(|set| set.iter().map(wit::from_wit_expolygon).collect())
            .collect()
    }
}

/// Clip several independent subject/clip pairs in one host call.
#[must_use]
pub fn clip_polygons_batch(requests: &[ClipRequest]) -> Vec<Vec<ExPolygon>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        requests
            .iter()
            .map(|r| crate::host::clip_polygons(&r.subject, &r.clip, r.op))
            .collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wit::slicer::common::host_services as svc;
        let wit_requests: Vec<svc::ClipRequest> = requests
            .iter()
            .map(|r| svc::ClipRequest {
                subject: r.subject.iter().map(wit::to_wit_expolygon).collect(),
                clip: r.clip.iter().map(wit::to_wit_expolygon).collect(),
                op: wit::to_wit_clip_op(r.op),
            })
            .collect();
        svc::clip_polygons_batch(&wit_requests)
            .into_iter()
            .map(|set| set.iter().map(wit::from_wit_expolygon).collect())
            .collect()
    }
}

/// Simplify several polygons in one host call.
#[must_use]
pub fn simplify_polygon_batch(requests: &[SimplifyRequest]) -> Vec<Polygon> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        requests
            .iter()
            .map(|r| crate::host::simplify_polygon(&r.polygon, r.tolerance_mm))
            .collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wit::slicer::common::host_services as svc;
        let wit_requests: Vec<svc::SimplifyRequest> = requests
            .iter()
            .map(|r| svc::SimplifyRequest {
                polygon: wit::to_wit_polygon(&r.polygon),
                tolerance_mm: r.tolerance_mm,
            })
            .collect();
        svc::simplify_polygon_batch(&wit_requests)
            .iter()
            .map(wit::from_wit_polygon)
            .collect()
    }
}

/// Cast several downward rays in one host call.
#[must_use]
pub fn raycast_z_down_batch(requests: &[RaycastRequest]) -> Vec<Option<f32>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        requests
            .iter()
            .map(|r| crate::host::raycast_z_down(&r.object_id, r.x, r.y, r.start_z))
            .collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wit::slicer::common::host_services as svc;
        let wit_requests: Vec<svc::RaycastRequest> = requests
            .iter()
            .map(|r| svc::RaycastRequest {
                object_id: r.object_id.clone(),
                x: r.x,
                y: r.y,
                start_z: r.start_z,
            })
            .collect();
        svc::raycast_z_down_batch(&wit_requests)
    }
}

/// Query several surface normals in one host call.
#[must_use]
pub fn surface_normal_at_batch(requests: &[SurfaceNormalRequest]) -> Vec<Option<Point3>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        requests
            .iter()
            .map(|r| crate::host::surface_normal_at(&r.object_id, r.x, r.y, r.z))
            .collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wit::slicer::common::host_services as svc;
        let wit_requests: Vec<svc::SurfaceNormalRequest> = requests
            .iter()
            .map(|r| svc::SurfaceNormalRequest {
                object_id: r.object_id.clone(),
                x: r.x,
                y: r.y,
                z: r.z,
            })
            .collect();
        svc::surface_normal_at_batch(&wit_requests)
            .into_iter()
            .map(|opt| {
                opt.map(|p| Point3 {
                    x: p.x,
                    y: p.y,
                    z: p.z,
                })
            })
            .collect()
    }
}

/// Guest-side bindings for the batched host imports.
///
/// The WIT here is an inline copy, matching the pattern
/// [`crate::host::medial_axis`] already uses: `#[slicer_module]` generates the
/// full world into a private inner module this crate cannot reach, so the SDK
/// declares its own import-only world. **This copy must stay in step with
/// `crates/slicer-schema/wit/deps/common.wit`** — `cargo xtask build-guests`
/// verifies each built component's embedded WIT against canonical, so drift
/// surfaces as a build failure rather than a linker error.
#[cfg(target_arch = "wasm32")]
mod wit {
    ::wit_bindgen::generate!({
        inline: r#"
package slicer:sdk-batch-helper;

package slicer:types {
    interface geometry {
        record point2 { x: s64, y: s64 }
        record point3 { x: f32, y: f32, z: f32 }
        record polygon    { points: list<point2> }
        record ex-polygon { contour: polygon, holes: list<polygon> }
    }
}

package slicer:common {
    interface host-services {
        use slicer:types/geometry.{ex-polygon, polygon, point3};
        type object-id = string;
        enum clip-operation   { union, intersection, difference, xor }
        enum offset-join-type { miter, round, square }

        record offset-request { polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type, arc-tolerance-mm: f32 }
        offset-polygons-batch: func(requests: list<offset-request>) -> list<list<ex-polygon>>;

        record clip-request { subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation }
        clip-polygons-batch: func(requests: list<clip-request>) -> list<list<ex-polygon>>;

        record simplify-request { polygon: polygon, tolerance-mm: f32 }
        simplify-polygon-batch: func(requests: list<simplify-request>) -> list<polygon>;

        record raycast-request { object-id: object-id, x: f32, y: f32, start-z: f32 }
        raycast-z-down-batch: func(requests: list<raycast-request>) -> list<option<f32>>;

        record surface-normal-request { object-id: object-id, x: f32, y: f32, z: f32 }
        surface-normal-at-batch: func(requests: list<surface-normal-request>) -> list<option<point3>>;
    }
}

world sdk-batch {
    import slicer:common/host-services;
}
"#,
        world: "sdk-batch",
        generate_all,
    });

    use slicer::types::geometry as g;

    pub fn to_wit_polygon(p: &slicer_ir::Polygon) -> g::Polygon {
        g::Polygon {
            points: p
                .points
                .iter()
                .map(|pt| g::Point2 { x: pt.x, y: pt.y })
                .collect(),
        }
    }

    pub fn from_wit_polygon(p: &g::Polygon) -> slicer_ir::Polygon {
        slicer_ir::Polygon {
            points: p
                .points
                .iter()
                .map(|pt| slicer_ir::Point2 { x: pt.x, y: pt.y })
                .collect(),
        }
    }

    pub fn to_wit_expolygon(e: &slicer_ir::ExPolygon) -> g::ExPolygon {
        g::ExPolygon {
            contour: to_wit_polygon(&e.contour),
            holes: e.holes.iter().map(to_wit_polygon).collect(),
        }
    }

    pub fn from_wit_expolygon(e: &g::ExPolygon) -> slicer_ir::ExPolygon {
        slicer_ir::ExPolygon {
            contour: from_wit_polygon(&e.contour),
            holes: e.holes.iter().map(from_wit_polygon).collect(),
        }
    }

    pub fn to_wit_join(
        join: crate::host::OffsetJoinType,
    ) -> slicer::common::host_services::OffsetJoinType {
        use slicer::common::host_services::OffsetJoinType as W;
        match join {
            crate::host::OffsetJoinType::Miter => W::Miter,
            crate::host::OffsetJoinType::Round => W::Round,
            crate::host::OffsetJoinType::Square => W::Square,
        }
    }

    pub fn to_wit_clip_op(
        op: crate::host::ClipOperation,
    ) -> slicer::common::host_services::ClipOperation {
        use slicer::common::host_services::ClipOperation as W;
        match op {
            crate::host::ClipOperation::Union => W::Union,
            crate::host::ClipOperation::Intersection => W::Intersection,
            crate::host::ClipOperation::Difference => W::Difference,
            crate::host::ClipOperation::Xor => W::Xor,
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use slicer_ir::Point2;

    fn square(min: f32, max: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(min, min),
                    Point2::from_mm(max, min),
                    Point2::from_mm(max, max),
                    Point2::from_mm(min, max),
                ],
            },
            holes: vec![],
        }
    }

    #[test]
    fn batch_offset_keeps_results_aligned_with_inputs() {
        let items = vec![1.0_f32, 2.0, 3.0];
        let paired = batch_offset(&items, |d| OffsetRequest {
            polygons: vec![square(0.0, 10.0)],
            delta_mm: *d,
            join: OffsetJoinType::Miter,
            arc_tolerance_mm: 0.0,
        });

        assert_eq!(paired.len(), items.len());
        for (item, result) in &paired {
            // Each result must be the offset of THIS item's delta: a larger
            // delta must produce a strictly wider box.
            // The source square is 10 mm wide, so offsetting by `d` gives
            // 10 + 2d. Each item's delta differs, so a misaligned zip shows up
            // here immediately.
            let width = result_width_mm(result);
            assert!(
                (width - (10.0 + 2.0 * **item)).abs() < 0.05,
                "item {item} got a result of width {width}"
            );
        }
    }

    #[test]
    fn batch_forms_agree_with_the_singular_forms() {
        let requests: Vec<OffsetRequest> = (1..=4)
            .map(|i| OffsetRequest {
                polygons: vec![square(0.0, 10.0 * i as f32)],
                delta_mm: i as f32 * 0.5,
                join: OffsetJoinType::Miter,
                arc_tolerance_mm: 0.0,
            })
            .collect();

        let batched = offset_polygons_batch(&requests);
        let one_by_one: Vec<Vec<ExPolygon>> = requests
            .iter()
            .map(|r| {
                crate::host::offset_polygons(&r.polygons, r.delta_mm, r.join, r.arc_tolerance_mm)
            })
            .collect();

        assert_eq!(batched, one_by_one);
    }

    fn result_width_mm(polys: &[ExPolygon]) -> f32 {
        let xs: Vec<i64> = polys
            .iter()
            .flat_map(|p| p.contour.points.iter())
            .map(|p| p.x)
            .collect();
        let min = *xs.iter().min().expect("non-empty");
        let max = *xs.iter().max().expect("non-empty");
        (max - min) as f32 / 10_000.0
    }
}
