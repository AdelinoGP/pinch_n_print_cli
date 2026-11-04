//! Optional typed XY diagnostics, independent of opaque prepared plan pieces.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use slicer_ir::{ExPolygon, Point2};

use super::PreparationProjectionError;

/// One completed whole-print diagnostic view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparationProjectionScene {
    /// Exact owner-local scene identifier.
    pub scene_id: String,
    /// Exact declared view name.
    pub view: String,
    /// Exact diagnostic class identifiers, unrelated to extrusion roles.
    pub classes: Vec<String>,
    /// Typed primitives with committed global schedule references.
    pub primitives: Vec<PreparationProjectionPrimitive>,
}

/// A labeled diagnostic primitive on one committed schedule layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparationProjectionPrimitive {
    /// Exact scene-local primitive identifier.
    pub id: String,
    /// Global index in the committed layer schedule.
    pub global_layer_index: u32,
    /// Exact class identifier declared by the scene.
    pub class: String,
    /// Optional display label, preserving its UTF-8 contents.
    pub label: Option<String>,
    /// XY geometry in exact integer units (1 unit = 100 nm).
    pub geometry: PreparationProjectionGeometry,
}

/// Typed diagnostic geometry, never a module's private algorithm plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreparationProjectionGeometry {
    /// A point marker.
    Point(Point2),
    /// An open path with at least two distinct consecutive vertices.
    Polyline(Vec<Point2>),
    /// A polygon with holes, rendered filled or as ring outlines.
    Polygon {
        /// CCW contour and CW holes in exact integer units.
        polygon: ExPolygon,
        /// Whether to fill the polygon interior while retaining holes.
        filled: bool,
    },
}

/// Host-owned diagnostic writer; its backing enforces ownership and poisoning.
pub trait PreparationProjectionWrite: Send + Sync {
    /// Return the requested declared views, preserving their exact names.
    fn requested_views(&self) -> Result<Vec<String>, PreparationProjectionError>;

    /// Stage one completed scene; rejected writes must poison the backing output.
    fn publish(&self, scene: &PreparationProjectionScene)
        -> Result<(), PreparationProjectionError>;
}

type RequestedViewsCallback = dyn Fn() -> Result<Vec<String>, PreparationProjectionError>;
type PublishCallback =
    dyn Fn(&PreparationProjectionScene) -> Result<(), PreparationProjectionError>;

/// Call-local facade for an optional host-owned diagnostic sink.
///
/// The backing must reject unrequested or foreign sink access and retain any
/// write failure until transaction completion, even if the module catches it.
pub struct PreparationProjectionOutput {
    requested_views: Box<RequestedViewsCallback>,
    publish: Box<PublishCallback>,
}

impl PreparationProjectionOutput {
    /// Adapt a native host writer without translating its typed errors.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_host_writer(writer: Arc<dyn PreparationProjectionWrite + Send + Sync>) -> Self {
        let views = Arc::clone(&writer);
        Self::from_callback_pair(
            move || views.requested_views(),
            move |scene| writer.publish(scene),
        )
    }

    /// Adapt generated resource imports without retaining a raw resource index.
    #[cfg(target_arch = "wasm32")]
    pub fn from_callbacks(
        requested_views: impl Fn() -> Result<Vec<String>, PreparationProjectionError> + 'static,
        publish: impl Fn(&PreparationProjectionScene) -> Result<(), PreparationProjectionError>
            + 'static,
    ) -> Self {
        Self::from_callback_pair(requested_views, publish)
    }

    fn from_callback_pair(
        requested_views: impl Fn() -> Result<Vec<String>, PreparationProjectionError> + 'static,
        publish: impl Fn(&PreparationProjectionScene) -> Result<(), PreparationProjectionError>
            + 'static,
    ) -> Self {
        Self {
            requested_views: Box::new(requested_views),
            publish: Box::new(publish),
        }
    }

    /// Return the requested declared views, without an empty-success fallback.
    pub fn requested_views(&self) -> Result<Vec<String>, PreparationProjectionError> {
        (self.requested_views)()
    }

    /// Publish exact typed geometry; validation and poison remain host-owned.
    pub fn publish(
        &self,
        scene: &PreparationProjectionScene,
    ) -> Result<(), PreparationProjectionError> {
        (self.publish)(scene)
    }
}
