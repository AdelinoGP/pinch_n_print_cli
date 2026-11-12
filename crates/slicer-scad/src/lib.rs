//! Optional exact SCAD ingestion boundary for Pinch 'n Print.
//!
//! This crate owns one small, engine-opaque source-to-geometry API:
//! [`evaluate`] parses and evaluates a self-contained OpenSCAD source with
//! ordered expression overrides, an exact render, a deny-all external-resource
//! resolver and a finite evaluation allowance, then returns *checked* PnP
//! geometry — even-odd contour forests mapped to outer/hole polygons, or an
//! indexed triangle mesh — together with the diagnostics it admitted and
//! enough provenance to reproduce the call.
//!
//! Three boundaries are deliberate:
//!
//! * **No engine type escapes.** Upstream `openrscad_*` types appear only inside
//!   [`engine`](crate) and never in a public signature. Downstream ingestion and
//!   property code consumes [`ScadEvalOutput`] and nothing else.
//! * **No geometry ownership.** The adapter returns geometry only. `ObjectMesh`
//!   assembly, source-derived identity, object IDs and bed placement stay with
//!   [`slicer_model_io::loader::assemble_object`]-equivalent host code; see
//!   `docs/01_system_architecture.md` § Data Ownership Rules.
//! * **Checked, not repaired.** Conversion either produces geometry that passes
//!   every declared check or returns an error naming the check that failed. It
//!   never welds, drops, fills or re-tessellates to rescue a result.
//!
//! The engine is optional. With `default = []` this crate compiles without any
//! native dependency: [`engine_info`] returns `None` and [`evaluate`] returns
//! [`ScadErrorKind::BackendDisabled`] without parsing. `--features openrscad`
//! activates the pinned, vendored engine described in
//! `vendor/openrscad/PATCHES.md`.
//!
//! Unit conversions follow `docs/08_coordinate_system.md`: [`UNITS_PER_MM`] is
//! the single authoritative factor, 2D coordinates are scaled integers and 3D
//! coordinates are millimetre `f32` values.

// The adapter contract document is crate documentation, not a loose file: wiring
// it in here means its `no_run` examples are compiled by `cargo test --doc`, so a
// snippet that stops matching the API fails the build instead of rotting in a
// Markdown file nobody compiles.
#![doc = include_str!("../../../docs/scad/adapter.md")]

use serde::{Deserialize, Serialize};

pub use slicer_ir::slice_ir::{ExPolygon, IndexedTriangleSet};

// Checked conversion and the diagnostic classifier are engine-neutral by design
// (they consume plain `f64` rings and a plain triangle soup), but nothing outside
// the engine path can reach them, so they are compiled only with the engine.
#[cfg(feature = "openrscad")]
mod conversion;
#[cfg(feature = "openrscad")]
mod diagnostics;

#[cfg(feature = "openrscad")]
mod engine;

/// Serialise `Option<Range<usize>>` as `null` or `{"start": n, "end": m}`.
///
/// `std::ops::Range` has no serde impl, and the span representation is part of
/// this crate's public data contract, so it is spelled out rather than replaced
/// with a bespoke range struct.
mod span_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::ops::Range;

    #[derive(Serialize, Deserialize)]
    struct SpanRepr {
        start: usize,
        end: usize,
    }

    pub(super) fn serialize<S: Serializer>(
        value: &Option<Range<usize>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value
            .as_ref()
            .map(|r| SpanRepr {
                start: r.start,
                end: r.end,
            })
            .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Range<usize>>, D::Error> {
        let repr = Option::<SpanRepr>::deserialize(deserializer)?;
        Ok(repr.map(|r| r.start..r.end))
    }
}

/// One SCAD source unit: a diagnostic label plus the UTF-8 text to evaluate.
///
/// `label` is diagnostic data. It is never resolved, opened, or used as a
/// resource base directory: the adapter has no path input and no working
/// directory fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScadSource {
    /// Human-readable origin, echoed into diagnostics and provenance.
    pub label: String,
    /// The complete SCAD source text.
    pub text: String,
}

/// One ordered `-D name=expression` override.
///
/// `name` is a case-sensitive SCAD identifier, optionally `$`-prefixed.
/// `expression` is parsed as exactly one expression and evaluated in the
/// engine's constant/base scope; it can neither inject statements nor see a
/// sibling override's binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScadParameter {
    /// Case-sensitive SCAD identifier (including supported `$` specials).
    pub name: String,
    /// A single SCAD expression, evaluated against the base scope.
    pub expression: String,
}

/// Bounded resource and complexity limits for one evaluation.
///
/// Every field is required to be finite and non-zero; deliberately unbounded
/// sentinels (`0` for "auto", `u64::MAX` for "no budget") are rejected as
/// [`ScadErrorKind::InvalidOptions`] rather than silently meaning "unlimited".
///
/// [`ScadLimits::default`] is the adapter's own bounded default, measured on
/// Windows against the fixtures in `docs/scad/windows-feasibility.md`. It is a
/// caller-visible contract, not a promise that a larger program will succeed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScadLimits {
    /// Total evaluation steps, shared by every override expression and the
    /// program itself.
    pub evaluation_steps: u64,
    /// Checked UTF-8 byte budget for the source plus every override name and
    /// expression.
    pub max_source_bytes: usize,
    /// Maximum AST nodes across the program and the parsed override expressions.
    pub max_ast_nodes: usize,
    /// Maximum AST depth across the program and the parsed override expressions.
    pub max_ast_depth: usize,
    /// Maximum evaluated CSG-tree nodes before rendering.
    pub max_geometry_nodes: usize,
    /// Maximum `$fn`-style facet count accepted from the source.
    pub max_facets: u32,
    /// Maximum output vertices accepted from the renderer.
    pub max_vertices: usize,
    /// Maximum output triangles accepted from the renderer.
    pub max_triangles: usize,
    /// Maximum absolute millimetre coordinate accepted from the engine, before
    /// rounding to the integer grid or narrowing to `f32`.
    pub max_abs_coordinate_mm: f64,
    /// Native stack size for the joined evaluation worker.
    pub worker_stack_bytes: usize,
}

/// Smallest worker stack the adapter accepts.
///
/// The engine's contained recursion guard trips at 6 000 native frames. A stack
/// that cannot hold those frames overflows *before* the guard can fire, turning a
/// diagnosable `PartialOutput` into a process abort, so anything below the
/// measured floor is an unsafe setting rather than a small one.
///
/// Measured on Windows (rustc 1.96.0, `dev` profile, the `cum` non-tail-recursion
/// fixture below): 16, 24, 32, 36, 40 and 44 MiB overflow; 48, 64 and 96 MiB reach
/// the guard and report `PartialOutput`. A `dev` profile frame is much larger than
/// upstream's ≈ 1.6 KiB release estimate, which is why the floor is far above the
/// naive 6 000 × 1.6 KiB arithmetic. This constant is the measured floor plus
/// margin, and it is deliberately derived from the *debug* build: a release build
/// needs less, so a debug-safe floor is safe for both.
pub const MIN_WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Largest worker stack the adapter accepts, so a caller cannot reserve an
/// effectively unbounded address range per call.
pub const MAX_WORKER_STACK_BYTES: usize = 1024 * 1024 * 1024;

impl Default for ScadLimits {
    /// Bounded defaults, all finite and non-zero.
    ///
    /// The values are the adapter defaults recorded in
    /// `docs/scad/windows-feasibility.md`; they bound one evaluation, not the
    /// size of a program a caller may attempt.
    fn default() -> Self {
        Self {
            evaluation_steps: 20_000_000,
            max_source_bytes: 1_048_576,
            max_ast_nodes: 200_000,
            max_ast_depth: 256,
            max_geometry_nodes: 100_000,
            max_facets: 360,
            max_vertices: 1_000_000,
            max_triangles: 2_000_000,
            max_abs_coordinate_mm: 10_000.0,
            worker_stack_bytes: 256 * 1024 * 1024,
        }
    }
}

#[cfg_attr(not(feature = "openrscad"), allow(dead_code))]
impl ScadLimits {
    /// Return the first offending field, if any.
    ///
    /// This is the single place limits are validated: serialising and
    /// deserialising a [`ScadEvalOptions`] cannot bypass it, because
    /// [`evaluate`] always calls this before doing any work.
    pub(crate) fn first_violation(&self) -> Option<&'static str> {
        if self.evaluation_steps == 0 {
            return Some("evaluation_steps must be non-zero (0 is not 'unlimited')");
        }
        if self.evaluation_steps == u64::MAX {
            return Some("evaluation_steps must be finite (u64::MAX is not 'unlimited')");
        }
        if self.max_source_bytes == 0 {
            return Some("max_source_bytes must be non-zero");
        }
        if self.max_ast_nodes == 0 {
            return Some("max_ast_nodes must be non-zero");
        }
        if self.max_ast_depth == 0 {
            return Some("max_ast_depth must be non-zero");
        }
        if self.max_geometry_nodes == 0 {
            return Some("max_geometry_nodes must be non-zero");
        }
        if self.max_facets == 0 {
            return Some("max_facets must be non-zero");
        }
        if self.max_vertices == 0 {
            return Some("max_vertices must be non-zero");
        }
        if self.max_triangles == 0 {
            return Some("max_triangles must be non-zero");
        }
        if !self.max_abs_coordinate_mm.is_finite() || self.max_abs_coordinate_mm <= 0.0 {
            return Some("max_abs_coordinate_mm must be finite and positive");
        }
        if self.worker_stack_bytes < MIN_WORKER_STACK_BYTES {
            return Some("worker_stack_bytes is below the safe minimum");
        }
        if self.worker_stack_bytes > MAX_WORKER_STACK_BYTES {
            return Some("worker_stack_bytes is above the safe maximum");
        }
        None
    }
}

/// Options for one evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScadEvalOptions {
    /// Ordered overrides, duplicates preserved. The last occurrence of a name
    /// wins; every occurrence is still parsed and evaluated.
    pub parameters: Vec<ScadParameter>,
    /// Explicit 2D extrusion height. `None` keeps a purely 2D result as
    /// contours; `Some(h)` extrudes it from Z=0 to Z=h.
    pub extrude_height_mm: Option<f64>,
    /// Resource and complexity limits.
    pub limits: ScadLimits,
    /// Explicit RNG seed. Unseeded programs make no determinism promise.
    pub random_seed: Option<u64>,
}

impl Default for ScadEvalOptions {
    /// Empty overrides, no forced extrusion and no forced seed, over
    /// [`ScadLimits::default`].
    fn default() -> Self {
        Self {
            parameters: Vec::new(),
            extrude_height_mm: None,
            limits: ScadLimits::default(),
            random_seed: None,
        }
    }
}

#[cfg_attr(not(feature = "openrscad"), allow(dead_code))]
impl ScadEvalOptions {
    /// First invalid option, if any. See [`ScadLimits::first_violation`].
    pub(crate) fn first_violation(&self) -> Option<String> {
        if let Some(why) = self.limits.first_violation() {
            return Some(why.to_string());
        }
        if let Some(h) = self.extrude_height_mm {
            if !h.is_finite() {
                return Some("extrude_height_mm must be finite".to_string());
            }
            if h <= 0.0 {
                return Some("extrude_height_mm must be strictly positive".to_string());
            }
            if h > self.limits.max_abs_coordinate_mm {
                return Some("extrude_height_mm exceeds max_abs_coordinate_mm".to_string());
            }
        }
        None
    }
}

/// Checked adapter output geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScadGeometry {
    /// The source evaluated to no geometry. Import and slicing callers must
    /// treat this as an empty model, not a successful import.
    Empty,
    /// Checked even-odd contour forest: outer contours CCW, holes CW.
    Contours(Vec<ExPolygon>),
    /// Checked indexed triangle mesh in millimetres, flat `Vec<u32>` indices.
    Mesh(IndexedTriangleSet),
}

/// Pipeline stage a diagnostic came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScadDiagnosticStage {
    /// Tokenising/parsing the program.
    Parse,
    /// Parsing or evaluating an override expression.
    Parameter,
    /// Evaluating the program.
    Evaluation,
    /// Exact rendering, including contour/projection/lowering.
    Render,
    /// Checked conversion of raw engine geometry.
    Conversion,
}

/// Fail-closed diagnostic class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScadDiagnosticClass {
    /// Recognised, non-degrading information. The only class a successful
    /// evaluation may contain.
    Information,
    /// A hard error.
    Error,
    /// Output exists but does not describe the whole source.
    Partial,
    /// A recoverable backend failure replaced combined geometry.
    Degraded,
    /// Geometry was approximated or skipped.
    Approximate,
    /// Not classified at this engine revision; always rejected.
    Unknown,
}

/// One classified diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScadDiagnostic {
    /// Which stage produced it.
    pub stage: ScadDiagnosticStage,
    /// Its fail-closed class.
    pub class: ScadDiagnosticClass,
    /// Stable adapter code, e.g. `scad_echo` or `assignment_overwritten`.
    pub code: String,
    /// Human-readable engine or adapter message.
    pub message: String,
    /// Byte span into the source text, except at
    /// [`ScadDiagnosticStage::Parameter`], where it indexes the failing
    /// expression identified by [`ScadError::parameter`].
    #[serde(with = "span_serde")]
    pub span: Option<std::ops::Range<usize>>,
}

/// Immutable engine identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScadEngineInfo {
    /// The engine or fork revision actually compiled in.
    pub revision: String,
    /// Lowercase SHA-256 of the ordered applied patch set, when one was applied.
    pub patch_digest: Option<String>,
    /// Native geometry backend, e.g. `manifold_cpp`.
    pub backend: String,
}

/// Everything needed to reproduce one evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScadProvenance {
    /// Engine identity at the time of the call.
    pub engine: ScadEngineInfo,
    /// The input source label.
    pub source_label: String,
    /// Lowercase SHA-256 hex of the source UTF-8 bytes.
    pub source_digest: String,
    /// The ordered raw override list exactly as supplied, duplicates included.
    pub parameters: Vec<ScadParameter>,
    /// The seed that was in effect, if the caller supplied one.
    pub random_seed: Option<u64>,
}

/// A successful evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScadEvalOutput {
    /// Checked geometry.
    pub geometry: ScadGeometry,
    /// Diagnostics that were admitted. On success these are all
    /// [`ScadDiagnosticClass::Information`].
    pub diagnostics: Vec<ScadDiagnostic>,
    /// Reproduction record.
    pub provenance: ScadProvenance,
}

/// Why an evaluation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScadErrorKind {
    /// The engine feature is not enabled.
    BackendDisabled,
    /// Limits, extrusion height or other options are invalid.
    InvalidOptions,
    /// The source did not parse.
    Parse,
    /// An override name or expression was rejected.
    InvalidParameter,
    /// The shared evaluation allowance ran out, in a parameter or the program.
    EvaluationBudgetExceeded,
    /// Program evaluation failed.
    Evaluation,
    /// An external resource was requested; deny-all was enforced.
    ExternalResourceDenied,
    /// Output exists but does not describe the whole source.
    PartialOutput,
    /// A recoverable render failure degraded the geometry.
    DegradedOutput,
    /// Geometry was approximated or skipped.
    ApproximateOutput,
    /// A diagnostic was not classified at this engine revision.
    UnknownDiagnostic,
    /// A non-empty result mixes 2D and 3D geometry.
    MixedDimensions,
    /// An extrusion height was supplied for a 3D result.
    ExtrusionHeightOn3d,
    /// A coordinate was NaN or infinite.
    NonFiniteCoordinate,
    /// A coordinate was outside the configured or representable range.
    CoordinateOutOfRange,
    /// Contour topology is invalid; nothing was repaired.
    InvalidContours,
    /// Mesh topology or indices are invalid; nothing was repaired.
    InvalidMesh,
    /// A declared complexity bound was exceeded.
    ComplexityLimit,
    /// The evaluation worker failed or panicked.
    WorkerFailure,
}

impl ScadErrorKind {
    /// Stable lowercase name, matching the serde representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BackendDisabled => "backend_disabled",
            Self::InvalidOptions => "invalid_options",
            Self::Parse => "parse",
            Self::InvalidParameter => "invalid_parameter",
            Self::EvaluationBudgetExceeded => "evaluation_budget_exceeded",
            Self::Evaluation => "evaluation",
            Self::ExternalResourceDenied => "external_resource_denied",
            Self::PartialOutput => "partial_output",
            Self::DegradedOutput => "degraded_output",
            Self::ApproximateOutput => "approximate_output",
            Self::UnknownDiagnostic => "unknown_diagnostic",
            Self::MixedDimensions => "mixed_dimensions",
            Self::ExtrusionHeightOn3d => "extrusion_height_on_3d",
            Self::NonFiniteCoordinate => "non_finite_coordinate",
            Self::CoordinateOutOfRange => "coordinate_out_of_range",
            Self::InvalidContours => "invalid_contours",
            Self::InvalidMesh => "invalid_mesh",
            Self::ComplexityLimit => "complexity_limit",
            Self::WorkerFailure => "worker_failure",
        }
    }
}

/// A failed evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScadError {
    /// Stable failure taxonomy.
    pub kind: ScadErrorKind,
    /// The source label the caller supplied.
    pub source_label: String,
    /// The offending override name, when the failure is parameter-specific.
    pub parameter: Option<String>,
    /// Human-readable explanation.
    pub message: String,
    /// Every diagnostic retained up to the failure.
    pub diagnostics: Vec<ScadDiagnostic>,
}

#[cfg_attr(not(feature = "openrscad"), allow(dead_code))]
impl ScadError {
    /// Start an error for `label` with no diagnostics attached yet.
    pub(crate) fn new(kind: ScadErrorKind, label: &str, message: impl Into<String>) -> Self {
        Self {
            kind,
            source_label: label.to_string(),
            parameter: None,
            message: message.into(),
            diagnostics: Vec::new(),
        }
    }

    /// Attach the offending parameter name.
    pub(crate) fn with_parameter(mut self, name: impl Into<String>) -> Self {
        self.parameter = Some(name.into());
        self
    }

    /// Attach retained diagnostics.
    pub(crate) fn with_diagnostics(mut self, diagnostics: Vec<ScadDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }
}

impl std::fmt::Display for ScadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scad {}: {}", self.kind.as_str(), self.message)?;
        if let Some(p) = &self.parameter {
            write!(f, " (parameter '{p}')")?;
        }
        if !self.source_label.is_empty() {
            write!(f, " [{}]", self.source_label)?;
        }
        Ok(())
    }
}

impl std::error::Error for ScadError {}

/// Identity of the engine compiled into this build, or `None` when the
/// `openrscad` feature is off.
///
/// Record this **before** running a dangerous fixture: it names the revision and
/// patch set a retained failure came from.
pub fn engine_info() -> Option<ScadEngineInfo> {
    #[cfg(feature = "openrscad")]
    {
        Some(engine::engine_info())
    }
    #[cfg(not(feature = "openrscad"))]
    {
        None
    }
}

/// Evaluate one self-contained SCAD source exactly.
///
/// Returns checked contours for a purely 2D result, a checked mesh for a purely
/// 3D result, or an explicit [`ScadGeometry::Empty`]. Every other outcome is an
/// error: partial, degraded, approximate and unknown diagnostics are rejected
/// rather than returned with geometry.
///
/// With the `openrscad` feature off this always returns
/// [`ScadErrorKind::BackendDisabled`] without parsing the source.
pub fn evaluate(
    source: &ScadSource,
    options: &ScadEvalOptions,
) -> Result<ScadEvalOutput, ScadError> {
    #[cfg(feature = "openrscad")]
    {
        engine::evaluate(source, options)
    }
    #[cfg(not(feature = "openrscad"))]
    {
        let _ = options;
        Err(ScadError::new(
            ScadErrorKind::BackendDisabled,
            &source.label,
            "this build has the `openrscad` feature disabled, so no SCAD engine is available",
        ))
    }
}
