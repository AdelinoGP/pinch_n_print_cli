//! Mock host adapter for module tests.
//!
//! `MockHost` is a real [`slicer_sdk::host::MeshSource`] implementation
//! that tests install into the SDK's thread-local mesh-source slot via
//! [`MockHost::install`]. Once installed, the live host wrappers
//! (`slicer_sdk::host::raycast_z_down`, `surface_normal_at`,
//! `object_bounds`) route through this adapter — the same code path a
//! module exercises in production — so tests verify wiring end-to-end
//! instead of asserting against a parallel mock surface.
//!
//! The struct also retains a simple call-counter (`record_call` /
//! `call_count` / `assert_call_count`) for assertions that are
//! independent of host state (e.g. "the module invoked this branch N
//! times"), and thin `log_warn` / `log_contains` helpers that route
//! through the real `slicer_sdk::host::log_warn` wrapper and the
//! `take_log_messages` capture sink.

use std::collections::HashMap;

use slicer_ir::{BoundingBox3, Point3};

/// Configurable, thread-installable double for [`slicer_sdk::host::MeshSource`].
///
/// Built with the chainable `with_*` setters, then installed for the
/// current thread via [`MockHost::install`] and removed via
/// [`MockHost::uninstall`].
///
/// # Examples
///
/// ```rust
/// use slicer_test::MockHost;
///
/// let _host = MockHost::new().with_raycast_hit(Some(1.5));
/// ```
#[derive(Debug, Default)]
pub struct MockHost {
    raycast_hit: Option<f32>,
    normal: Option<Point3>,
    bounds: Option<BoundingBox3>,
    call_counts: HashMap<String, usize>,
}

impl MockHost {
    /// Create a new `MockHost` with every mesh query returning `None`
    /// (the documented "no surface found" signal) and an empty call
    /// counter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let _host = MockHost::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the value returned by [`slicer_sdk::host::MeshSource::raycast_z_down`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let _host = MockHost::new().with_raycast_hit(Some(0.42));
    /// ```
    #[must_use]
    pub fn with_raycast_hit(mut self, value: Option<f32>) -> Self {
        self.raycast_hit = value;
        self
    }

    /// Set the value returned by [`slicer_sdk::host::MeshSource::surface_normal_at`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::Point3;
    /// use slicer_test::MockHost;
    ///
    /// let _host = MockHost::new().with_normal(Some(Point3 { x: 0.0, y: 0.0, z: 1.0 }));
    /// ```
    #[must_use]
    pub fn with_normal(mut self, value: Option<Point3>) -> Self {
        self.normal = value;
        self
    }

    /// Set the value returned by [`slicer_sdk::host::MeshSource::object_bounds`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{BoundingBox3, Point3};
    /// use slicer_test::MockHost;
    ///
    /// let bounds = BoundingBox3 {
    ///     min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
    ///     max: Point3 { x: 1.0, y: 1.0, z: 1.0 },
    /// };
    /// let _host = MockHost::new().with_object_bounds(bounds);
    /// ```
    #[must_use]
    pub fn with_object_bounds(mut self, value: BoundingBox3) -> Self {
        self.bounds = Some(value);
        self
    }

    /// Install this `MockHost` as the per-thread [`slicer_sdk::host::MeshSource`].
    ///
    /// After install, calls to `slicer_sdk::host::raycast_z_down`,
    /// `surface_normal_at`, and `object_bounds` on this thread route
    /// through this adapter. Call [`MockHost::uninstall`] in test
    /// teardown.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// MockHost::new().with_raycast_hit(Some(1.0)).install();
    /// MockHost::uninstall();
    /// ```
    pub fn install(self) {
        slicer_sdk::host::test_support::install_mesh_source(self);
    }

    /// Uninstall any per-thread [`slicer_sdk::host::MeshSource`] previously
    /// installed via [`MockHost::install`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// MockHost::uninstall();
    /// ```
    pub fn uninstall() {
        slicer_sdk::host::test_support::clear_mesh_source();
    }

    /// Record that a named host call occurred.
    ///
    /// This is independent of mesh-source state; use it for axes like
    /// "did the module take this branch?" that don't fit the
    /// `MeshSource` shape.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let mut host = MockHost::new();
    /// host.record_call("clip_polygons");
    /// ```
    pub fn record_call(&mut self, name: impl Into<String>) {
        *self.call_counts.entry(name.into()).or_insert(0) += 1;
    }

    /// Return how many times a named host call was recorded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let mut host = MockHost::new();
    /// host.record_call("clip_polygons");
    /// assert_eq!(host.call_count("clip_polygons"), 1);
    /// ```
    #[must_use]
    pub fn call_count(&self, name: &str) -> usize {
        self.call_counts.get(name).copied().unwrap_or(0)
    }

    /// Assert that a named host call was recorded exactly `expected` times.
    ///
    /// # Panics
    /// Panics when the observed call count differs from `expected`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let mut host = MockHost::new();
    /// host.record_call("clip_polygons");
    /// host.assert_call_count("clip_polygons", 1);
    /// ```
    pub fn assert_call_count(&self, name: &str, expected: usize) {
        let got = self.call_count(name);
        assert_eq!(
            got, expected,
            "expected {expected} calls to {name}, got {got}"
        );
    }

    /// Route a warning through the real [`slicer_sdk::host::log_warn`]
    /// wrapper. When [`slicer_sdk::host::test_support::install_log_capture`]
    /// has been called for this thread, the message is captured in the
    /// SDK's capture buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let host = MockHost::new();
    /// host.log_warn("density near limit");
    /// ```
    pub fn log_warn(&self, message: impl Into<String>) {
        slicer_sdk::host::log_warn(&message.into());
    }

    /// Return `true` if any message currently in the SDK log-capture
    /// buffer contains `needle`.
    ///
    /// # Side effect
    /// This **drains** the capture buffer
    /// (`slicer_sdk::host::test_support::take_log_messages` removes the
    /// messages it returns). Calling `log_contains` twice in a row with
    /// the same needle may yield different results because the second
    /// call sees an empty buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// // No capture installed → empty drain → false.
    /// assert!(!MockHost::log_contains("missing"));
    /// ```
    #[must_use]
    pub fn log_contains(needle: &str) -> bool {
        let drained = slicer_sdk::host::test_support::take_log_messages();
        drained.iter().any(|(_lvl, msg)| msg.contains(needle))
    }
}

impl slicer_sdk::host::MeshSource for MockHost {
    fn raycast_z_down(&self, _object_id: &str, _x: f32, _y: f32, _start_z: f32) -> Option<f32> {
        self.raycast_hit
    }

    fn surface_normal_at(&self, _object_id: &str, _x: f32, _y: f32, _z: f32) -> Option<Point3> {
        self.normal
    }

    fn object_bounds(&self, _object_id: &str) -> Option<BoundingBox3> {
        self.bounds
    }
}
