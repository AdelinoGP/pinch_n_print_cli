//! Dimensionally explicit unit conversions, float error envelopes and exact
//! integer area helpers.
//!
//! Units: one canonical grid unit is 100 nm, so `UNITS_PER_MM = 10_000`
//! (`docs/08_coordinate_system.md`). Lengths in mm, areas in mm² and grid
//! "double areas" in squared units are never mixed in one comparison.

use slicer_ir::ExPolygon;
use slicer_sdk::test_support::property_testing::oracles::signed_area2;

use super::bounds::BoundsError;

/// Canonical grid units per millimetre (1 unit = 100 nm).
pub const UNITS_PER_MM: i64 = 10_000;

/// Unit roundoff of IEEE binary32 round-to-nearest: `u = 2^-24`.
pub const F32_UNIT_ROUNDOFF: f64 = f32::EPSILON as f64 / 2.0;

/// Integral millimetres to canonical units, exact or an error.
pub fn whole_mm_to_units(mm: i64) -> Result<i64, BoundsError> {
    mm.checked_mul(UNITS_PER_MM)
        .ok_or_else(|| BoundsError::Arithmetic {
            what: format!("{mm} mm in canonical units"),
        })
}

/// Canonical units to millimetres in `f64`.
///
/// Exact in the sense that the integer is first represented exactly
/// (`|units| <= 2^53`), then divided once by 10,000: the result is the
/// correctly rounded quotient, with relative error at most `2^-53`. Integers
/// beyond `2^53` are rejected rather than silently rounded.
pub fn units_to_mm_f64(units: i64) -> Result<f64, BoundsError> {
    if units.unsigned_abs() > 1 << 53 {
        return Err(BoundsError::Arithmetic {
            what: format!("{units} units is not exactly representable in f64"),
        });
    }
    Ok(units as f64 / UNITS_PER_MM as f64)
}

/// Absolute error envelope, in the operands' own unit, for an f32 evaluation.
///
/// Derivation (Higham, *Accuracy and Stability of Numerical Algorithms*,
/// Lemma 3.1 and §3.1): with round-to-nearest binary32 and no underflow or
/// overflow, each of `n` rounded `+ - *` operations multiplies its exact
/// result by `(1 + d_i)`, `|d_i| <= u = 2^-24`. Expanding the evaluation
/// tree, every monomial of the exact value carries at most `n` such factors,
/// whose product lies in `[1 - gamma(n), 1 + gamma(n)]` with
/// `gamma(n) = n*u / (1 - n*u)`. Hence
/// `|computed - exact| <= gamma(n) * sum(|monomials|)`.
///
/// `magnitude` must bound `sum(|monomials|)` of the exact expression in the
/// same unit as the result (mm for a length, mm² for an area). Inputs are
/// assumed already representable in f32 (their conversion is a separate
/// rounded operation that the caller must count in `n`). Requires `n*u < 1`.
pub fn f32_abs_error_envelope(magnitude: f64, rounded_ops: u32) -> Result<f64, BoundsError> {
    if !magnitude.is_finite() || magnitude < 0.0 {
        return Err(BoundsError::Arithmetic {
            what: format!("envelope magnitude {magnitude} must be finite and non-negative"),
        });
    }
    let nu = f64::from(rounded_ops) * F32_UNIT_ROUNDOFF;
    if nu >= 1.0 {
        return Err(BoundsError::Arithmetic {
            what: format!("gamma({rounded_ops}) is undefined for binary32"),
        });
    }
    let gamma = nu / (1.0 - nu);
    Ok(gamma * magnitude)
}

/// Integer rounding of an exact rational point to the nearest grid point
/// moves each coordinate by at most `1/2` unit, so the Euclidean move is at
/// most `sqrt(1/2) < 1` unit. A segment joining two such rounded points stays
/// within that distance of the exact segment (distance to a segment is a
/// convex function of its endpoints). This is the quantization envelope the
/// Boolean sample margin and the clip rounding band are compared against.
pub const ROUNDED_POINT_MAX_EUCLIDEAN_SQ_NUMERATOR: i128 = 1;
/// Denominator of [`ROUNDED_POINT_MAX_EUCLIDEAN_SQ_NUMERATOR`]: `(1/2)² + (1/2)² = 1/2`.
pub const ROUNDED_POINT_MAX_EUCLIDEAN_SQ_DENOMINATOR: i128 = 2;

/// Exact double area (squared units) of a polygon set: contours count
/// positive, holes negative, as given by #2 `signed_area2`.
///
/// For a well-wound set (CCW contours, CW holes) this is twice the enclosed
/// material area. Sums are checked; overflow is an error.
pub fn region_double_area(polygons: &[ExPolygon]) -> Result<i128, String> {
    let mut total = 0_i128;
    for polygon in polygons {
        for ring in std::iter::once(&polygon.contour).chain(polygon.holes.iter()) {
            let area = signed_area2(ring).map_err(|error| format!("ring double area: {error}"))?;
            total = total
                .checked_add(area)
                .ok_or_else(|| String::from("region double-area sum overflowed i128"))?;
        }
    }
    Ok(total)
}

/// Exact squared distance comparison `dist(p, [a, b])² >= margin²` in `i128`.
///
/// With `|coordinate| <= 2^30` every product fits `i128`: differences are
/// below `2^31`, a cross/dot product below `2^63`, and `cross²` below
/// `2^126`; the perpendicular case compares `cross² >= margin² * len²`
/// without division. Every operation is checked, so larger inputs report an
/// overflow error instead of wrapping.
pub fn point_clears_segment(
    p: slicer_ir::Point2,
    a: slicer_ir::Point2,
    b: slicer_ir::Point2,
    margin: i64,
) -> Result<bool, String> {
    let sub = |u: i64, v: i64| {
        i128::from(u)
            .checked_sub(i128::from(v))
            .ok_or_else(|| String::from("distance difference overflow"))
    };
    let mul = |u: i128, v: i128| {
        u.checked_mul(v)
            .ok_or_else(|| String::from("distance product overflow"))
    };
    let add = |u: i128, v: i128| {
        u.checked_add(v)
            .ok_or_else(|| String::from("distance sum overflow"))
    };
    let margin_sq = mul(i128::from(margin), i128::from(margin))?;
    let (dx, dy) = (sub(b.x, a.x)?, sub(b.y, a.y)?);
    let (px, py) = (sub(p.x, a.x)?, sub(p.y, a.y)?);
    let len_sq = add(mul(dx, dx)?, mul(dy, dy)?)?;
    let dot = add(mul(px, dx)?, mul(py, dy)?)?;
    if len_sq == 0 || dot <= 0 {
        return Ok(add(mul(px, px)?, mul(py, py)?)? >= margin_sq);
    }
    if dot >= len_sq {
        let (qx, qy) = (sub(p.x, b.x)?, sub(p.y, b.y)?);
        return Ok(add(mul(qx, qx)?, mul(qy, qy)?)? >= margin_sq);
    }
    let cross = add(mul(px, dy)?, -mul(py, dx)?)?;
    Ok(mul(cross, cross)? >= mul(margin_sq, len_sq)?)
}
