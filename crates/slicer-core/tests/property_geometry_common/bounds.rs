//! Checked registry-bound readers and lossless conversions.
//!
//! Registry bounds are `u64`. Every narrowing here is checked and fails with a
//! [`BoundsError`] instead of saturating, truncating or defaulting to zero, and
//! every safety range is validated **before** any strategy is constructed, so
//! an invalid profile can never reach sampling.

use std::fmt;
use std::ops::RangeInclusive;

use slicer_sdk::test_support::property_testing::PropertyRun;

/// Largest integer that every `f32` in `0..=2^24` represents exactly.
pub const F32_EXACT_INTEGER_LIMIT: u64 = 1 << 24;
/// Largest integer that every `f64` in `0..=2^53` represents exactly.
pub const F64_EXACT_INTEGER_LIMIT: u64 = 1 << 53;
/// `line_segments` and `validate_geometry` accept `1..=1_000_000` units.
pub const MAX_ABS_UNITS_RANGE: RangeInclusive<u64> = 1..=1_000_000;

/// A rejected profile bound or checked conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundsError {
    /// The registry profile does not carry the bound, or the harness refused it.
    Missing {
        /// Bound name.
        name: String,
        /// Harness diagnostic.
        detail: String,
    },
    /// A bound that must be positive was zero.
    Zero {
        /// Bound name.
        name: String,
    },
    /// The value does not fit the target integer type.
    Overflow {
        /// Bound name.
        name: String,
        /// Original value.
        value: u64,
        /// Target type.
        target: &'static str,
    },
    /// The value cannot be represented exactly in the target float type.
    Inexact {
        /// Bound name.
        name: String,
        /// Original value.
        value: u64,
        /// Target type.
        target: &'static str,
    },
    /// The value is outside the documented safety range.
    OutOfRange {
        /// Bound name.
        name: String,
        /// Original value.
        value: u64,
        /// Inclusive accepted range.
        range: (u64, u64),
    },
    /// The value cannot support a named mandatory witness.
    BelowWitness {
        /// Bound name.
        name: String,
        /// Original value.
        value: u64,
        /// The named witness that needs the minimum.
        witness: &'static str,
        /// Minimum required value.
        minimum: u64,
    },
    /// A derived checked quantity overflowed.
    Arithmetic {
        /// What was being derived.
        what: String,
    },
}

impl fmt::Display for BoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { name, detail } => write!(f, "bound {name} unavailable: {detail}"),
            Self::Zero { name } => write!(f, "bound {name} must be positive"),
            Self::Overflow {
                name,
                value,
                target,
            } => write!(f, "bound {name}={value} does not fit {target}"),
            Self::Inexact {
                name,
                value,
                target,
            } => write!(
                f,
                "bound {name}={value} is not exactly representable as {target}"
            ),
            Self::OutOfRange { name, value, range } => write!(
                f,
                "bound {name}={value} is outside the safety range {}..={}",
                range.0, range.1
            ),
            Self::BelowWitness {
                name,
                value,
                witness,
                minimum,
            } => write!(
                f,
                "bound {name}={value} cannot support the {witness} witness (needs >= {minimum})"
            ),
            Self::Arithmetic { what } => write!(f, "checked arithmetic overflow deriving {what}"),
        }
    }
}

impl std::error::Error for BoundsError {}

fn positive(name: &str, value: u64) -> Result<u64, BoundsError> {
    if value == 0 {
        Err(BoundsError::Zero {
            name: name.to_string(),
        })
    } else {
        Ok(value)
    }
}

/// Read one required positive registry bound.
pub fn read(run: &PropertyRun, name: &str) -> Result<u64, BoundsError> {
    let value = run.bound(name).map_err(|error| BoundsError::Missing {
        name: name.to_string(),
        detail: error.to_string(),
    })?;
    positive(name, value)
}

/// Positive `u64 -> i64`, failing on zero or overflow.
pub fn to_i64(name: &str, value: u64) -> Result<i64, BoundsError> {
    i64::try_from(positive(name, value)?).map_err(|_| BoundsError::Overflow {
        name: name.to_string(),
        value,
        target: "i64",
    })
}

/// Positive `u64 -> usize`, failing on zero or overflow.
pub fn to_usize(name: &str, value: u64) -> Result<usize, BoundsError> {
    usize::try_from(positive(name, value)?).map_err(|_| BoundsError::Overflow {
        name: name.to_string(),
        value,
        target: "usize",
    })
}

/// Positive `u64 -> u32`, failing on zero or overflow.
pub fn to_u32(name: &str, value: u64) -> Result<u32, BoundsError> {
    u32::try_from(positive(name, value)?).map_err(|_| BoundsError::Overflow {
        name: name.to_string(),
        value,
        target: "u32",
    })
}

/// Positive `u64 -> f32`, failing unless the integer is exactly representable.
///
/// Every integer in `0..=2^24` is exact in `f32` (24-bit significand); larger
/// integers are rejected even when a particular one happens to be exact, so
/// the rule does not depend on the value's trailing zeros.
pub fn to_f32_exact(name: &str, value: u64) -> Result<f32, BoundsError> {
    let value = positive(name, value)?;
    if value > F32_EXACT_INTEGER_LIMIT {
        return Err(BoundsError::Inexact {
            name: name.to_string(),
            value,
            target: "f32",
        });
    }
    // Exact by the 2^24 bound above.
    let converted = value as f32;
    if converted as u64 != value {
        return Err(BoundsError::Inexact {
            name: name.to_string(),
            value,
            target: "f32",
        });
    }
    Ok(converted)
}

/// Positive `u64 -> f64`, failing unless the integer is exactly representable.
///
/// Every integer in `0..=2^53` is exact in `f64` (53-bit significand).
pub fn to_f64_exact(name: &str, value: u64) -> Result<f64, BoundsError> {
    let value = positive(name, value)?;
    if value > F64_EXACT_INTEGER_LIMIT {
        return Err(BoundsError::Inexact {
            name: name.to_string(),
            value,
            target: "f64",
        });
    }
    let converted = value as f64;
    if converted as u64 != value {
        return Err(BoundsError::Inexact {
            name: name.to_string(),
            value,
            target: "f64",
        });
    }
    Ok(converted)
}

/// Require `value` in an inclusive safety range.
pub fn in_range(name: &str, value: u64, range: RangeInclusive<u64>) -> Result<u64, BoundsError> {
    if range.contains(&value) {
        Ok(value)
    } else {
        Err(BoundsError::OutOfRange {
            name: name.to_string(),
            value,
            range: (*range.start(), *range.end()),
        })
    }
}

/// Require `value` to support a named mandatory witness.
pub fn at_least(
    name: &str,
    value: u64,
    witness: &'static str,
    minimum: u64,
) -> Result<u64, BoundsError> {
    if value >= minimum {
        Ok(value)
    } else {
        Err(BoundsError::BelowWitness {
            name: name.to_string(),
            value,
            witness,
            minimum,
        })
    }
}

/// `max_abs_units` within `1..=1_000_000`, as `i64`.
///
/// `line_segments`, `geometry_cases` and `validate_geometry` reject anything
/// outside this range; checking it here fails the profile before sampling.
pub fn max_abs_units(value: u64) -> Result<i64, BoundsError> {
    to_i64(
        "max_abs_units",
        in_range("max_abs_units", value, MAX_ABS_UNITS_RANGE)?,
    )
}

/// Raw registry values of the shared geometry bound keys, before validation.
///
/// Kept separate from [`PropertyRun`] so the controls can drive exactly the
/// validator the properties use with deliberately invalid profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawGeometryBounds {
    /// `max_abs_units`.
    pub max_abs_units: u64,
    /// `max_vertices`.
    pub max_vertices: u64,
    /// `max_holes`.
    pub max_holes: u64,
    /// `max_components`.
    pub max_components: u64,
    /// `max_shrink_candidates`.
    pub max_shrink_candidates: u64,
    /// `max_queries`.
    pub max_queries: u64,
}

impl RawGeometryBounds {
    /// Read every shared geometry key from the selected registry profile.
    pub fn read(run: &PropertyRun) -> Result<Self, BoundsError> {
        Ok(Self {
            max_abs_units: read(run, "max_abs_units")?,
            max_vertices: read(run, "max_vertices")?,
            max_holes: read(run, "max_holes")?,
            max_components: read(run, "max_components")?,
            max_shrink_candidates: read(run, "max_shrink_candidates")?,
            max_queries: read(run, "max_queries")?,
        })
    }
}

/// Validated shared geometry bounds, converted losslessly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometryLimits {
    /// Absolute coordinate bound on every SUT input, canonical units.
    pub max_abs_units: i64,
    /// Total vertices across all rings of one generated operand.
    pub max_vertices: usize,
    /// Holes of one generated operand.
    pub max_holes: usize,
    /// Outer components of one generated operand.
    pub max_components: usize,
    /// Candidate budget of the #2 geometry value tree.
    pub max_shrink_candidates: u32,
    /// Membership queries per checked operand pair or sampled segment.
    pub max_queries: usize,
}

/// Safety ceiling on `max_vertices`/`max_holes`/`max_components`: the #2
/// catalog supports at most 64 required rings, and the exact BigRational
/// oracles are quadratic in ring size.
pub const MAX_COUNT_CEILING: u64 = 64;
/// Safety ceiling on `max_queries`; it also bounds the sample scale factor
/// used by the clip model (`max_abs_units * max_queries` must fit `i64`).
pub const MAX_QUERIES_CEILING: u64 = 4096;
/// Safety ceiling on `max_shrink_candidates`.
pub const MAX_SHRINK_CANDIDATES_CEILING: u64 = 1 << 16;

/// Validate the shared keys with the minima each property's witnesses need.
///
/// `min_abs_units`, `min_vertices` and `min_queries` are the property's own
/// witness minima; `queries_witness` names the witness that needs them.
pub fn validate_geometry_limits(
    raw: RawGeometryBounds,
    min_abs_units: u64,
    min_vertices: u64,
    min_holes: u64,
    min_components: u64,
    min_queries: u64,
    queries_witness: &'static str,
) -> Result<GeometryLimits, BoundsError> {
    let max_abs_units = max_abs_units(raw.max_abs_units)?;
    at_least(
        "max_abs_units",
        raw.max_abs_units,
        "deterministic scene",
        min_abs_units,
    )?;
    let count = |name: &str, value: u64, witness: &'static str, minimum: u64| {
        in_range(name, value, 1..=MAX_COUNT_CEILING)?;
        at_least(name, value, witness, minimum)?;
        to_usize(name, value)
    };
    let max_vertices = count(
        "max_vertices",
        raw.max_vertices,
        "operand ring",
        min_vertices,
    )?;
    let max_holes = count("max_holes", raw.max_holes, "holed operand", min_holes)?;
    let max_components = count(
        "max_components",
        raw.max_components,
        "disconnected operand",
        min_components,
    )?;
    in_range(
        "max_shrink_candidates",
        raw.max_shrink_candidates,
        1..=MAX_SHRINK_CANDIDATES_CEILING,
    )?;
    let max_shrink_candidates = to_u32("max_shrink_candidates", raw.max_shrink_candidates)?;
    in_range("max_queries", raw.max_queries, 1..=MAX_QUERIES_CEILING)?;
    at_least("max_queries", raw.max_queries, queries_witness, min_queries)?;
    let max_queries = to_usize("max_queries", raw.max_queries)?;
    Ok(GeometryLimits {
        max_abs_units,
        max_vertices,
        max_holes,
        max_components,
        max_shrink_candidates,
        max_queries,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-N1 control body.
// ─────────────────────────────────────────────────────────────────────────────

fn expect_err<T: fmt::Debug>(label: &str, result: Result<T, BoundsError>) -> BoundsError {
    match result {
        Ok(value) => panic!("{label}: invalid bound was accepted as {value:?}"),
        Err(error) => {
            // Every rejection must be a printable diagnostic.
            assert!(!error.to_string().is_empty(), "{label}: empty diagnostic");
            error
        }
    }
}

/// Prove that zero, overflowing and out-of-range bounds are rejected by the
/// exact validators the properties call, before any strategy is constructed,
/// and that valid bounds convert losslessly.
pub fn controls_run() {
    use super::envelopes;
    use super::{boolean, clip};

    // Zero is rejected by every positive conversion.
    assert_eq!(
        expect_err("zero i64", to_i64("k", 0)),
        BoundsError::Zero { name: "k".into() }
    );
    assert!(matches!(
        expect_err("zero usize", to_usize("k", 0)),
        BoundsError::Zero { .. }
    ));
    assert!(matches!(
        expect_err("zero f32", to_f32_exact("k", 0)),
        BoundsError::Zero { .. }
    ));
    assert!(matches!(
        expect_err("zero f64", to_f64_exact("k", 0)),
        BoundsError::Zero { .. }
    ));

    // Overflow is an error, never a saturated or wrapped value.
    assert_eq!(
        expect_err("i64 overflow", to_i64("k", u64::MAX)),
        BoundsError::Overflow {
            name: "k".into(),
            value: u64::MAX,
            target: "i64",
        }
    );
    assert_eq!(to_i64("k", i64::MAX as u64), Ok(i64::MAX));
    assert!(matches!(
        expect_err("i64 overflow +1", to_i64("k", (i64::MAX as u64) + 1)),
        BoundsError::Overflow { .. }
    ));
    assert_eq!(to_u32("k", u64::from(u32::MAX)), Ok(u32::MAX));
    assert!(matches!(
        expect_err("u32 overflow", to_u32("k", u64::from(u32::MAX) + 1)),
        BoundsError::Overflow { target: "u32", .. }
    ));

    // Float conversions are exact or rejected; 2^24 + 1 rounds in f32.
    assert_eq!(to_f32_exact("k", 1 << 24), Ok(16_777_216.0_f32));
    assert!(matches!(
        expect_err("f32 inexact", to_f32_exact("k", (1 << 24) + 1)),
        BoundsError::Inexact { target: "f32", .. }
    ));
    assert_eq!(to_f64_exact("k", 1 << 53), Ok(9_007_199_254_740_992.0_f64));
    assert!(matches!(
        expect_err("f64 inexact", to_f64_exact("k", (1 << 53) + 1)),
        BoundsError::Inexact { target: "f64", .. }
    ));
    for value in [1_u64, 3, 10_000, 16_777_215] {
        let as_f32 = to_f32_exact("k", value).expect("exact f32");
        assert_eq!(as_f32 as u64, value, "f32 round trip of {value}");
    }

    // max_abs_units is held to the 1..=1_000_000 domain of line_segments.
    assert_eq!(max_abs_units(1), Ok(1));
    assert_eq!(max_abs_units(1_000_000), Ok(1_000_000));
    assert!(matches!(
        expect_err("units 0", max_abs_units(0)),
        BoundsError::OutOfRange { .. }
    ));
    assert!(matches!(
        expect_err("units 1_000_001", max_abs_units(1_000_001)),
        BoundsError::OutOfRange {
            value: 1_000_001,
            ..
        }
    ));
    assert!(matches!(
        expect_err("units u64::MAX", max_abs_units(u64::MAX)),
        BoundsError::OutOfRange { .. }
    ));

    // The property validators fail on each bad key before sampling: the
    // strategy constructors return the same error and never build a tree.
    let good_bool = boolean::reference_raw_bounds();
    let bool_ok = boolean::validate_limits(good_bool).expect("reference boolean profile");
    assert_eq!(bool_ok.max_abs_units, good_bool.max_abs_units as i64);
    assert_eq!(bool_ok.max_queries as u64, good_bool.max_queries);
    assert!(boolean::strategy(&bool_ok).is_ok());
    let bad_bool = [
        RawGeometryBounds {
            max_abs_units: 0,
            ..good_bool
        },
        RawGeometryBounds {
            max_abs_units: 1_000_001,
            ..good_bool
        },
        RawGeometryBounds {
            max_abs_units: boolean::MIN_ABS_UNITS - 1,
            ..good_bool
        },
        RawGeometryBounds {
            max_vertices: 0,
            ..good_bool
        },
        RawGeometryBounds {
            max_vertices: u64::MAX,
            ..good_bool
        },
        RawGeometryBounds {
            max_holes: 0,
            ..good_bool
        },
        RawGeometryBounds {
            max_components: 1,
            ..good_bool
        },
        RawGeometryBounds {
            max_shrink_candidates: 0,
            ..good_bool
        },
        RawGeometryBounds {
            max_shrink_candidates: u64::MAX,
            ..good_bool
        },
        RawGeometryBounds {
            max_queries: boolean::min_queries(good_bool) - 1,
            ..good_bool
        },
        RawGeometryBounds {
            max_queries: u64::MAX,
            ..good_bool
        },
    ];
    for (index, raw) in bad_bool.into_iter().enumerate() {
        expect_err(
            &format!("boolean profile {index}"),
            boolean::validate_limits(raw),
        );
    }

    let (good_clip, good_polylines) = clip::reference_raw_bounds();
    let clip_ok = clip::validate_limits(good_clip, good_polylines).expect("reference clip profile");
    assert_eq!(
        clip_ok.geometry.max_abs_units,
        good_clip.max_abs_units as i64
    );
    assert_eq!(clip_ok.max_polylines as u64, good_polylines);
    assert!(clip::strategy(&clip_ok).is_ok());
    let bad_clip = [
        (
            RawGeometryBounds {
                max_abs_units: clip::MIN_ABS_UNITS - 1,
                ..good_clip
            },
            good_polylines,
        ),
        (
            RawGeometryBounds {
                max_abs_units: 1_000_001,
                ..good_clip
            },
            good_polylines,
        ),
        (
            RawGeometryBounds {
                max_vertices: clip::MIN_VERTICES - 1,
                ..good_clip
            },
            good_polylines,
        ),
        (
            RawGeometryBounds {
                max_queries: clip::MIN_QUERIES - 1,
                ..good_clip
            },
            good_polylines,
        ),
        (good_clip, clip::WITNESS_PATHS as u64),
        (good_clip, 0),
        (good_clip, u64::MAX),
    ];
    for (index, (raw, polylines)) in bad_clip.into_iter().enumerate() {
        expect_err(
            &format!("clip profile {index}"),
            clip::validate_limits(raw, polylines),
        );
    }

    // Envelope helpers: exact unit conversion and checked overflow.
    assert_eq!(envelopes::whole_mm_to_units(1), Ok(10_000));
    assert_eq!(envelopes::whole_mm_to_units(-100), Ok(-1_000_000));
    assert!(matches!(
        expect_err("mm overflow", envelopes::whole_mm_to_units(i64::MAX / 10)),
        BoundsError::Arithmetic { .. }
    ));
    assert_eq!(envelopes::units_to_mm_f64(12_345).ok(), Some(1.2345));
    assert!(envelopes::units_to_mm_f64(1 << 53).is_ok());
    assert!(envelopes::units_to_mm_f64((1 << 53) + 1).is_err());

    // The f32 envelope is zero for no rounded operations, rejects invalid
    // magnitudes, and bounds a real f32 evaluation of the stated sequence.
    assert_eq!(envelopes::f32_abs_error_envelope(100.0, 0), Ok(0.0));
    assert!(envelopes::f32_abs_error_envelope(f64::NAN, 1).is_err());
    assert!(envelopes::f32_abs_error_envelope(-1.0, 1).is_err());
    // (a + b) * c: two rounded f32 operations, every intermediate <= 100 mm.
    let (a, b, c) = (12.345_678_f64, 31.234_567_f64, 0.731_f64);
    let exact = (f64::from(a as f32) + f64::from(b as f32)) * f64::from(c as f32);
    let rounded = f64::from(((a as f32) + (b as f32)) * (c as f32));
    let envelope = envelopes::f32_abs_error_envelope(100.0, 2).expect("finite envelope");
    assert!(
        (rounded - exact).abs() <= envelope,
        "observed f32 error {} exceeds the derived envelope {envelope}",
        (rounded - exact).abs()
    );
    // The envelope is tight enough to matter: it is far below 1e-4 mm (1 unit).
    assert!(
        envelope < 1e-4,
        "envelope {envelope} is not below one grid unit"
    );
}
