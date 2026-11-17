//! Checked registry-bound readers for the `H-HELPERS-SPLIT` profile.
//!
//! The one registry record for this charter carries exactly three bound keys:
//! `max_components`, `max_faces` and `max_abs_mm`. Every reader here converts the
//! registry's `u64` losslessly (or fails), rejects zero, and enforces a written
//! safety range before any strategy allocates. A profile that cannot admit the
//! mandatory witnesses therefore fails before sampling instead of saturating,
//! truncating or defaulting to zero.

use std::fmt;

use slicer_sdk::test_support::property_testing::PropertyRun;

/// Smallest component ceiling that admits the widest mandatory witness.
///
/// `ThreeDisjointSolids` is the widest scene: three closed tetrahedra in three
/// separate connected components.
pub const MIN_COMPONENTS: u64 = 3;
/// Allocation guard for the component ceiling. The bounded scene catalog never
/// exceeds three components, so a larger declared ceiling only wastes nothing;
/// a smaller one is rejected above.
pub const MAX_COMPONENTS_CEILING: u64 = 64;

/// Smallest face ceiling that admits the widest mandatory witness.
///
/// Three disjoint tetrahedra carry `3 * 4 = 12` faces; `SolidWithFragment`
/// (`4 + 1`), `VertexOnlyContact` (`4 + 4`) and the two-face edge pairs all fit
/// inside it.
pub const MIN_FACES: u64 = 12;
/// Allocation guard for the face ceiling.
pub const MAX_FACES_CEILING: u64 = 4_096;

/// Smallest absolute-coordinate ceiling that admits the widest mandatory
/// witness with headroom.
///
/// The widest scene spans `2 * 2 mm` of part stride, `+ 1 mm` of local solid
/// extent and `+ 0.5 mm` of maximum shrinkable origin offset: `5.5 mm`. Eight
/// whole millimetres leave `2.5 mm` of headroom and stay far inside the range
/// where every integer is exactly representable in `f32`.
pub const MIN_MAX_ABS_MM: u64 = 8;
/// Ceiling for the absolute-coordinate bound, matching the shared SDK mesh
/// domain's millimetre ceiling.
pub const MAX_ABS_MM_CEILING: u64 = 10_000;

/// Failure to read a required property-profile bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundError {
    name: &'static str,
    detail: String,
}

impl BoundError {
    fn new(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            detail: detail.into(),
        }
    }

    /// Registry bound name that failed.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Human-readable failure detail, including the rejected value.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for BoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bound {}: {}", self.name, self.detail)
    }
}

impl std::error::Error for BoundError {}

/// Read a bound that must exist and must be positive.
fn positive(run: &PropertyRun, name: &'static str) -> Result<u64, BoundError> {
    let value = run
        .bound(name)
        .map_err(|error| BoundError::new(name, error.to_string()))?;
    if value == 0 {
        return Err(BoundError::new(name, "must be positive"));
    }
    Ok(value)
}

/// Enforce a written safety range; an out-of-range value is never narrowed.
fn within(name: &'static str, value: u64, low: u64, high: u64) -> Result<u64, BoundError> {
    if (low..=high).contains(&value) {
        Ok(value)
    } else {
        Err(BoundError::new(
            name,
            format!("must be in {low}..={high}, got {value}"),
        ))
    }
}

/// Read `max_components` as a lossless `u32`.
pub fn component_bound(run: &PropertyRun) -> Result<u32, BoundError> {
    const NAME: &str = "max_components";
    let value = within(
        NAME,
        positive(run, NAME)?,
        MIN_COMPONENTS,
        MAX_COMPONENTS_CEILING,
    )?;
    u32::try_from(value).map_err(|_| BoundError::new(NAME, format!("{value} does not fit u32")))
}

/// Read `max_faces` as a lossless `usize`.
pub fn face_bound(run: &PropertyRun) -> Result<usize, BoundError> {
    const NAME: &str = "max_faces";
    let value = within(NAME, positive(run, NAME)?, MIN_FACES, MAX_FACES_CEILING)?;
    usize::try_from(value).map_err(|_| BoundError::new(NAME, format!("{value} does not fit usize")))
}

/// Read `max_abs_mm` as a finite, exactly representable `f32`.
pub fn millimetre_bound(run: &PropertyRun) -> Result<f32, BoundError> {
    const NAME: &str = "max_abs_mm";
    let value = within(
        NAME,
        positive(run, NAME)?,
        MIN_MAX_ABS_MM,
        MAX_ABS_MM_CEILING,
    )?;
    // The registry stores whole millimetres as `u64`; the i64 and f32 narrowing
    // are proved lossless by the explicit round trip rather than assumed.
    let signed = i64::try_from(value)
        .map_err(|_| BoundError::new(NAME, format!("{value} does not fit i64")))?;
    let millimetres = signed as f32;
    if !millimetres.is_finite() || millimetres as i64 != signed {
        return Err(BoundError::new(
            NAME,
            format!("{signed} is not exactly representable as f32"),
        ));
    }
    Ok(millimetres)
}

/// The three checked profile bounds for `H-HELPERS-SPLIT`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelperLimits {
    /// Maximum connected components any witness or generated scene may produce.
    pub max_components: u32,
    /// Maximum triangle count any witness or generated scene may carry.
    pub max_faces: usize,
    /// Maximum absolute vertex coordinate, in millimetres.
    pub max_abs_mm: f32,
}

/// Read and validate every `H-HELPERS-SPLIT` bound.
///
/// The per-bound safety ranges already encode the mandatory-witness floors
/// (`MIN_COMPONENTS`, `MIN_FACES`, `MIN_MAX_ABS_MM`); the witness pass in
/// `super::split::deterministic_witnesses` then builds every scene against the
/// returned limits and fails loudly if one does not fit.
pub fn checked_limits(run: &PropertyRun) -> Result<HelperLimits, BoundError> {
    Ok(HelperLimits {
        max_components: component_bound(run)?,
        max_faces: face_bound(run)?,
        max_abs_mm: millimetre_bound(run)?,
    })
}
