//! Output capture stubs for module tests.

use slicer_ir::ExtrusionPath3D;

/// Minimal capture sink for extrusion paths.
#[derive(Debug, Default)]
pub struct InfillOutputCapture {
    paths: Vec<ExtrusionPath3D>,
}

impl InfillOutputCapture {
    /// Create an empty capture sink.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::capture::InfillOutputCapture;
    ///
    /// let _capture = InfillOutputCapture::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Capture one emitted path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
    /// use slicer_test::capture::InfillOutputCapture;
    ///
    /// let mut capture = InfillOutputCapture::new();
    /// capture.push_path(ExtrusionPath3D {
    ///     points: Vec::new(),
    ///     role: ExtrusionRole::SparseInfill,
    ///     speed_factor: 1.0,
    /// });
    /// ```
    pub fn push_path(&mut self, path: ExtrusionPath3D) {
        self.paths.push(path);
    }

    /// Borrow all captured paths.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::capture::InfillOutputCapture;
    ///
    /// let capture = InfillOutputCapture::new();
    /// assert!(capture.paths().is_empty());
    /// ```
    #[must_use]
    pub fn paths(&self) -> &[ExtrusionPath3D] {
        &self.paths
    }
}
