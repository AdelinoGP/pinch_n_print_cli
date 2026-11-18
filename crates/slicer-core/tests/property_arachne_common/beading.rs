//! Charter `H-ARACHNE-BEADING`: analytic bead and sentinel relations for the
//! three real beading strategies.
//!
//! # What is called
//!
//! Every branch below drives the **production** implementation through
//! [`BeadingStrategy`]:
//!
//! * `slicer_core::beading::distributed::DistributedBeadingStrategy::{new,compute}`
//!   (`crates/slicer-core/src/beading/distributed.rs`),
//! * `slicer_core::beading::limited::LimitedBeadingStrategy::{new,compute}`
//!   (`crates/slicer-core/src/beading/limited.rs`),
//! * `slicer_core::beading::widening::WideningBeadingStrategy::{new,compute}`
//!   (`crates/slicer-core/src/beading/widening.rs`).
//!
//! # Contract (independently derived, never read back from the SUT)
//!
//! With a positive dyadic width `w`, thickness `t` and a nonzero distribution
//! radius, `DistributedBeadingStrategy::compute` (which receives `w` as its
//! `optimal_width`) must satisfy:
//!
//! * `count = 0`: both arrays empty, `left_over == t`.
//! * `count = 1`: one bead of width `t` centred at `t / 2`.
//! * `count = 2`: two beads of width `t / 2` centred at `t / 4` and `3t / 4`.
//! * `count = n >= 3`, `t = n * w`: every width is `w`, centre `i` is
//!   `(i + 1/2) * w`, and `left_over == 0`.
//!
//! For `LimitedBeadingStrategy` over a distributed parent with an **even** cap
//! `2k >= 2`:
//!
//! * at the cap (`t = 2k * w`, requested `2k`): `2k` real beads of width `w` at
//!   `(i + 1/2) * w`, one zero-width marker at `k * w`, `left_over == 0`;
//! * over the cap (requested `2k + 1`, `t = 2k * w + excess`, `excess > 0`): the
//!   `2k` real beads keep width `w`; the left-half centres stay `(i + 1/2) * w`
//!   and the right half mirrors them about `t / 2`; two zero-width markers sit
//!   at `k * w` and `t - k * w`; `left_over == excess`.
//!
//! For `WideningBeadingStrategy`, canonical
//! `WideningBeadingStrategy::compute` (`WideningBeadingStrategy.cpp`) gates its
//! special branch on `bead_count <= 1 && thickness < getTransitionThickness(1)`:
//!
//! * below `min_input_width` with `bead_count <= 1`: no beads, `left_over == t`;
//! * `min_input_width <= t < get_transition_thickness(1)` with
//!   `bead_count <= 1`: one bead of width `max(t, min_output_width)` centred at
//!   `t / 2`, `left_over == 0`;
//! * `bead_count >= 2`: the wrapped parent's own `compute`, **including**
//!   `t < optimal_width`.
//!
//! Widening deliberately over-emits: the clamped single bead can be wider than
//! the region. `sum(bead_widths) == total_thickness - left_over` is therefore
//! **not** asserted for widening generally (see
//! [`check_conservation`], which the controls prove rejects that invalid
//! assertion on a real widening output).
//!
//! # Non-vacuity
//!
//! One generated case carries the selectors for every branch, and
//! [`check_case`] runs *all* of them for *every* case. Each observed population
//! is compared with its derived size (one bead, two beads, `n` beads, `2k` real
//! beads plus 1/2 markers), so no branch can be "covered" by an empty result and
//! branch coverage never depends on which values the RNG happens to sample.
//!
//! # Error envelope derivation
//!
//! All generated lengths are dyadic rationals: `units / 2^shift` with
//! `units <= 2^53` and `shift <= 3`. [`validate_limits`] additionally proves the
//! derived products `(max_beads + 2) * max_width_units + max_excess_units` fit
//! `2^53`, so `n * w`, `2k * w` and `2k * w + excess` are exact, and every
//! quotient by a power of two only changes the exponent. The comparisons still
//! use a derived envelope rather than the observed output:
//!
//! With round-to-nearest binary64, each of `m` rounded `+ - * /` operations
//! multiplies its exact result by `(1 + d)`, `|d| <= u = 2^-53` (Higham,
//! *Accuracy and Stability of Numerical Algorithms*, Lemma 3.1 and §3.1).
//! Expanding the evaluation tree gives
//! `|computed - exact| <= gamma(m) * sum(|monomials|)`, `gamma(m) = m u / (1 - m u)`.
//! [`Envelope::from_operations`] is exactly that bound: an absolute term
//! `gamma(m) * magnitude` plus a relative term `gamma(m)`.
//!
//! Operation budget, counted from the statements in `compute`:
//! widths — `bead_count as f64` (exact) then one `*`, one `-`, one `/`, one `*`
//! and one `+` per bead, at most `n - 1` accumulated `+=`, and the limited
//! decorator's `left_over += thickness - total_thickness` (one `-`, one `+`):
//! `max_beads + 6`. Locations — one `w / 2`, three per `+=` step, the limited
//! mirror `thickness - location` and the marker's `location ± width / 2`:
//! `6 * max_beads + 2`. [`Envelopes::derive`] rounds those budgets up (by two
//! for widths, six for locations) before applying the bound.
//!
//! At the widest authoring profile (extended: `max_beads = 16`,
//! `max_width_units = 40_000`, `max_excess_units = 16_000`) that gives a width
//! envelope of `gamma(24) * 736_000 approx 2.0e-9` units and a location envelope
//! of `gamma(104) * 696_000 approx 8.0e-9` units — both far below one canonical
//! unit (`10^-4` mm). This is a derived bound, not a tuned epsilon.

use std::fmt;

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_core::beading::distributed::DistributedBeadingStrategy;
use slicer_core::beading::limited::LimitedBeadingStrategy;
use slicer_core::beading::widening::WideningBeadingStrategy;
use slicer_core::beading::{Beading, BeadingStrategy};
use slicer_sdk::test_support::property_testing::PropertyRun;

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-ARACHNE-BEADING";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_arachne_beading_relations";

/// Largest integer every `f64` in `0..=2^53` represents exactly (53-bit
/// significand). Larger integers are rejected rather than silently rounded.
pub const MAX_EXACT_INTEGER: u64 = 1 << 53;

/// Safety ceiling on `max_beads`.
pub const MAX_BEADS_CEILING: u64 = 64;
/// Smallest usable `max_beads`: the distributed `n >= 3` relation needs three
/// beads, and the limited over-cap relation needs an even cap `2k` that still
/// leaves room for the requested `2k + 1`.
pub const MIN_MAX_BEADS: u64 = 3;
/// Largest dyadic denominator shift. Keeps every generated length an exact
/// dyadic rational with at most 8 sub-unit divisions.
pub const MAX_DYADIC_SHIFT: u32 = 3;
/// Largest generated distribution radius (bead counts). One covers the
/// `radius < 2` weight-denominator branch, two and above the `radius >= 2` one.
pub const MAX_DISTRIBUTION_COUNT: u64 = 4;
/// Fraction of `optimal_width` used for both middle thresholds of the
/// non-canonical-parent witnesses. `get_transition_thickness(1)` is then
/// `(1 + 0.5) * optimal_width`, so the widening single-bead window is
/// `[min_input_width, 1.5 * optimal_width)`.
pub const MIDDLE_THRESHOLD: f64 = 0.5;
/// Wall-transition angle (radians), matching the factory's internal default.
pub const WALL_TRANSITION_ANGLE: f64 = 0.174_532_925_199_432_95;

// Authoring-tier values this charter is piloted and registered against (the
// registry record itself is Step 7; these are code-side choices, not a
// measurement):
//   routine:  max_beads = 8,  max_width_units = 20_000, max_excess_units = 8_000
//   extended: max_beads = 16, max_width_units = 40_000, max_excess_units = 16_000
// At the extended authoring values the width envelope is
// `gamma(24) * 736_000` units (~2e-9) and the location envelope
// `gamma(104) * 696_000` (~8e-9), both far below one canonical unit.

// ─────────────────────────────────────────────────────────────────────────────
// Checked registry bounds.
// ─────────────────────────────────────────────────────────────────────────────

/// A rejected registry bound or checked conversion.
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
        /// The witness that needs the minimum.
        witness: &'static str,
        /// Minimum required value.
        minimum: u64,
    },
    /// A derived checked quantity overflowed or lost exactness.
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
                "bound {name}={value} is outside the accepted range {}..={}",
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
            Self::Arithmetic { what } => write!(f, "checked arithmetic failed deriving {what}"),
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

/// Positive `u64 -> usize`, failing on zero or overflow.
pub fn to_usize(name: &str, value: u64) -> Result<usize, BoundsError> {
    usize::try_from(positive(name, value)?).map_err(|_| BoundsError::Overflow {
        name: name.to_string(),
        value,
        target: "usize",
    })
}

/// Positive `u64 -> f64`, failing unless the integer is exactly representable.
pub fn to_f64_exact(name: &str, value: u64) -> Result<f64, BoundsError> {
    let value = positive(name, value)?;
    if value > MAX_EXACT_INTEGER || (value as f64) as u64 != value {
        return Err(BoundsError::Inexact {
            name: name.to_string(),
            value,
            target: "f64",
        });
    }
    Ok(value as f64)
}

/// Raw registry values of this charter's three bound keys, before validation.
///
/// Kept separate from [`PropertyRun`] so the controls can drive exactly the
/// validator the property calls with deliberately invalid profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawBeadingBounds {
    /// `max_beads`.
    pub max_beads: u64,
    /// `max_width_units`.
    pub max_width_units: u64,
    /// `max_excess_units`.
    pub max_excess_units: u64,
}

impl RawBeadingBounds {
    /// Read every bound this charter requires from the selected profile.
    pub fn read(run: &PropertyRun) -> Result<Self, BoundsError> {
        Ok(Self {
            max_beads: read(run, "max_beads")?,
            max_width_units: read(run, "max_width_units")?,
            max_excess_units: read(run, "max_excess_units")?,
        })
    }
}

/// Largest even cap `2k >= 2` that still admits the requested `cap + 1`
/// over-cap witness, as `(cap, cap + 1)`.
///
/// `max_beads = 2` is the impossible case: the only even cap it could carry is
/// `2`, and the over-cap branch needs a requested `3`, which the profile does
/// not admit. Rejecting it here is what makes the over-cap witness mandatory
/// rather than best-effort.
pub fn witness_cap(max_beads: u64) -> Result<(u64, u64), BoundsError> {
    if max_beads < MIN_MAX_BEADS {
        return Err(BoundsError::BelowWitness {
            name: "max_beads".to_string(),
            value: max_beads,
            witness: "limited even cap plus its cap+1 over-cap case",
            minimum: MIN_MAX_BEADS,
        });
    }
    let cap = 2 * ((max_beads - 1) / 2);
    let over_cap = cap.checked_add(1).ok_or_else(|| BoundsError::Arithmetic {
        what: "cap + 1".to_string(),
    })?;
    if over_cap > max_beads {
        return Err(BoundsError::Arithmetic {
            what: format!(
                "an even cap of {cap} cannot also carry the requested {over_cap} within max_beads={max_beads}"
            ),
        });
    }
    Ok((cap, over_cap))
}

/// Validated bounds, converted losslessly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeadingLimits {
    /// Largest generated requested bead count.
    pub max_beads: usize,
    /// Largest generated dyadic width numerator, in units.
    pub max_width_units: u64,
    /// Largest generated over-cap surplus numerator, in units.
    pub max_excess_units: u64,
    /// [`Self::max_width_units`] as an exact `f64`.
    pub max_width: f64,
    /// [`Self::max_excess_units`] as an exact `f64`.
    pub max_excess: f64,
}

/// Validate the three keys before any strategy or value tree exists.
///
/// Rejects zero, non-representable and out-of-range values, a `max_beads` too
/// small to carry the mandatory witnesses, and any profile whose derived
/// products leave the exact-integer range.
pub fn validate_limits(raw: RawBeadingBounds) -> Result<BeadingLimits, BoundsError> {
    if raw.max_beads == 0 {
        return Err(BoundsError::Zero {
            name: "max_beads".to_string(),
        });
    }
    if raw.max_beads > MAX_BEADS_CEILING {
        return Err(BoundsError::OutOfRange {
            name: "max_beads".to_string(),
            value: raw.max_beads,
            range: (1, MAX_BEADS_CEILING),
        });
    }
    witness_cap(raw.max_beads)?;
    let max_beads = to_usize("max_beads", raw.max_beads)?;

    // Both lengths must be exactly representable before they are used as
    // floats; the validated integers are kept for the exactness proof below.
    let max_width = to_f64_exact("max_width_units", raw.max_width_units)?;
    let max_excess = to_f64_exact("max_excess_units", raw.max_excess_units)?;
    let max_width_units = raw.max_width_units;
    let max_excess_units = raw.max_excess_units;

    let scaled = raw
        .max_beads
        .checked_add(2)
        .and_then(|value| value.checked_mul(max_width_units))
        .and_then(|value| value.checked_add(max_excess_units))
        .ok_or_else(|| BoundsError::Arithmetic {
            what: "(max_beads + 2) * max_width_units + max_excess_units".to_string(),
        })?;
    if scaled > MAX_EXACT_INTEGER {
        return Err(BoundsError::Arithmetic {
            what: format!(
                "(max_beads + 2) * max_width_units + max_excess_units = {scaled} exceeds the \
                 exact-integer limit {MAX_EXACT_INTEGER}"
            ),
        });
    }

    Ok(BeadingLimits {
        max_beads,
        max_width_units,
        max_excess_units,
        max_width,
        max_excess,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Derived float envelope.
// ─────────────────────────────────────────────────────────────────────────────

/// Unit roundoff of IEEE binary64 round-to-nearest: `u = 2^-53`.
pub const F64_UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;

/// Absolute plus relative error envelope for one compared quantity.
///
/// `|observed - expected| <= absolute + relative * |expected|`, i.e.
/// `gamma(m) * magnitude + gamma(m) * |expected|`.
#[derive(Debug, Clone, Copy)]
pub struct Envelope {
    /// Absolute term, in the quantity's own unit, independent of the value
    /// being compared.
    pub absolute: f64,
    /// Relative term (dimensionless).
    pub relative: f64,
}

impl Envelope {
    /// Higham's `gamma(m) * magnitude` bound for `m` rounded operations.
    ///
    /// `magnitude` must bound `sum(|monomials|)` of the exact expression.
    /// Requires `m * u < 1`.
    pub fn from_operations(magnitude: f64, rounded_ops: u32) -> Result<Self, BoundsError> {
        if !magnitude.is_finite() || magnitude < 0.0 {
            return Err(BoundsError::Arithmetic {
                what: format!("envelope magnitude {magnitude} must be finite and non-negative"),
            });
        }
        let nu = f64::from(rounded_ops) * F64_UNIT_ROUNDOFF;
        if nu >= 1.0 {
            return Err(BoundsError::Arithmetic {
                what: format!("gamma({rounded_ops}) is undefined for binary64"),
            });
        }
        let gamma = nu / (1.0 - nu);
        Ok(Self {
            absolute: gamma * magnitude,
            relative: gamma,
        })
    }

    /// Tolerance this envelope grants a comparison against `expected`.
    pub fn tolerance(&self, expected: f64) -> f64 {
        self.absolute + self.relative * expected.abs()
    }

    /// Whether `observed` is within this envelope of `expected`.
    pub fn contains(&self, observed: f64, expected: f64) -> bool {
        (observed - expected).abs() <= self.tolerance(expected)
    }
}

/// The two envelopes every numeric relation in this charter compares against.
#[derive(Debug, Clone, Copy)]
pub struct Envelopes {
    /// Envelope for bead widths, `left_over` and `total_thickness`.
    pub width: Envelope,
    /// Envelope for toolpath centres.
    pub location: Envelope,
}

impl Envelopes {
    /// Derive both envelopes from validated bounds (see the module docs for the
    /// operation and magnitude counts).
    pub fn derive(limits: &BeadingLimits) -> Result<Self, BoundsError> {
        let n = u32::try_from(limits.max_beads).map_err(|_| BoundsError::Arithmetic {
            what: "max_beads as a rounded-operation count".to_string(),
        })?;
        let max_width = limits.max_width;
        let bead_count = f64::from(n);
        let width_magnitude = (bead_count + 2.0) * max_width + limits.max_excess;
        let location_magnitude = (bead_count + 1.0) * max_width + limits.max_excess;
        Ok(Self {
            width: Envelope::from_operations(width_magnitude, n + 8)?,
            location: Envelope::from_operations(location_magnitude, 6 * n + 8)?,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent comparator.
// ─────────────────────────────────────────────────────────────────────────────

/// A rejected beading, with the independently derived value that disagreed.
#[derive(Debug, Clone, PartialEq)]
pub enum OracleError {
    /// `bead_widths` and `toolpath_locations` are not parallel.
    LengthMismatch {
        /// Observed width count.
        widths: usize,
        /// Observed location count.
        locations: usize,
    },
    /// The number of non-marker beads is not the derived one.
    RealBeadCount {
        /// Derived count.
        expected: usize,
        /// Observed count.
        observed: usize,
    },
    /// The number of zero-width markers is not the derived one.
    SentinelCount {
        /// Derived marker count.
        expected: usize,
        /// Observed marker count.
        observed: usize,
    },
    /// The markers are not at the derived indices.
    SentinelIndex {
        /// Derived indices.
        expected: Vec<usize>,
        /// Observed indices.
        observed: Vec<usize>,
    },
    /// A marker lost its zero width.
    SentinelWidth {
        /// Output index.
        index: usize,
        /// Observed width.
        width: f64,
    },
    /// A real bead width is outside the derived envelope.
    WidthMismatch {
        /// Output index.
        index: usize,
        /// Derived width.
        expected: f64,
        /// Observed width.
        observed: f64,
        /// Envelope tolerance at `expected`.
        tolerance: f64,
    },
    /// A toolpath centre is outside the derived envelope.
    LocationMismatch {
        /// Output index.
        index: usize,
        /// Derived centre.
        expected: f64,
        /// Observed centre.
        observed: f64,
        /// Envelope tolerance at `expected`.
        tolerance: f64,
    },
    /// `left_over` differs from the derived value.
    LeftOverMismatch {
        /// Derived value.
        expected: f64,
        /// Observed value.
        observed: f64,
        /// Envelope tolerance at `expected`.
        tolerance: f64,
    },
    /// `total_thickness` differs from the derived value.
    TotalThicknessMismatch {
        /// Derived value.
        expected: f64,
        /// Observed value.
        observed: f64,
        /// Envelope tolerance at `expected`.
        tolerance: f64,
    },
    /// The conservation identity `total == sum(widths) + left_over` failed.
    Conservation {
        /// `total_thickness`.
        total: f64,
        /// Sum of `bead_widths`.
        sum: f64,
        /// `left_over`.
        left_over: f64,
        /// Envelope tolerance at `sum + left_over`.
        tolerance: f64,
    },
}

impl fmt::Display for OracleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { widths, locations } => write!(
                f,
                "bead_widths.len() ({widths}) != toolpath_locations.len() ({locations})"
            ),
            Self::RealBeadCount { expected, observed } => write!(
                f,
                "real bead count: derived {expected}, observed {observed}"
            ),
            Self::SentinelCount { expected, observed } => write!(
                f,
                "zero-width marker count: derived {expected}, observed {observed}"
            ),
            Self::SentinelIndex { expected, observed } => write!(
                f,
                "zero-width marker indices: derived {expected:?}, observed {observed:?}"
            ),
            Self::SentinelWidth { index, width } => {
                write!(f, "marker at index {index} carries width {width}, not 0.0")
            }
            Self::WidthMismatch {
                index,
                expected,
                observed,
                tolerance,
            } => write!(
                f,
                "bead_widths[{index}]: derived {expected}, observed {observed} (envelope {tolerance})"
            ),
            Self::LocationMismatch {
                index,
                expected,
                observed,
                tolerance,
            } => write!(
                f,
                "toolpath_locations[{index}]: derived {expected}, observed {observed} (envelope {tolerance})"
            ),
            Self::LeftOverMismatch {
                expected,
                observed,
                tolerance,
            } => write!(
                f,
                "left_over: derived {expected}, observed {observed} (envelope {tolerance})"
            ),
            Self::TotalThicknessMismatch {
                expected,
                observed,
                tolerance,
            } => write!(
                f,
                "total_thickness: derived {expected}, observed {observed} (envelope {tolerance})"
            ),
            Self::Conservation {
                total,
                sum,
                left_over,
                tolerance,
            } => write!(
                f,
                "conservation: total_thickness {total} != sum(bead_widths) {sum} + left_over \
                 {left_over} (envelope {tolerance})"
            ),
        }
    }
}

impl std::error::Error for OracleError {}

/// The structural contract every `Beading` must satisfy: parallel arrays.
pub fn check_parallel(beading: &Beading) -> Result<(), OracleError> {
    if beading.bead_widths.len() != beading.toolpath_locations.len() {
        return Err(OracleError::LengthMismatch {
            widths: beading.bead_widths.len(),
            locations: beading.toolpath_locations.len(),
        });
    }
    Ok(())
}

/// `total_thickness == sum(bead_widths) + left_over`, within `envelope`.
///
/// Valid for the distributed and limited outputs, where the emitted widths do
/// partition the region. **Not** valid for widening: the single-bead branch
/// clamps the emitted width up to `min_output_width` and reports
/// `left_over = 0`, so a region thinner than that minimum legitimately emits
/// `sum(bead_widths) > total_thickness`.
pub fn check_conservation(beading: &Beading, envelope: &Envelope) -> Result<(), OracleError> {
    check_parallel(beading)?;
    let sum: f64 = beading.bead_widths.iter().sum();
    let model = sum + beading.left_over;
    if !envelope.contains(beading.total_thickness, model) {
        return Err(OracleError::Conservation {
            total: beading.total_thickness,
            sum,
            left_over: beading.left_over,
            tolerance: envelope.tolerance(model),
        });
    }
    Ok(())
}

/// Independently derived expectation for one `LimitedBeadingStrategy::compute`
/// call at or over an even cap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LimitedExpectation {
    /// The even cap `2k`, i.e. the requested count of the at-cap call.
    pub cap: usize,
    /// `k = cap / 2`.
    pub half: usize,
    /// Real bead width `w`.
    pub bead_width: f64,
    /// Region thickness of this call.
    pub thickness: f64,
    /// Zero-width markers: 1 at the cap, 2 over the cap.
    pub sentinels: usize,
    /// Derived `left_over`: 0 at the cap, `thickness - cap * w` over it.
    pub left_over: f64,
}

/// Compare a limited output with the independently derived at-cap/over-cap
/// model: real bead count, marker count and indices, every width, every centre
/// and `left_over`.
pub fn compare_limited(
    observed: &Beading,
    expected: &LimitedExpectation,
    width_envelope: &Envelope,
    location_envelope: &Envelope,
) -> Result<(), OracleError> {
    check_parallel(observed)?;

    let markers: Vec<usize> = observed
        .bead_widths
        .iter()
        .enumerate()
        .filter(|(_, width)| **width == 0.0)
        .map(|(index, _)| index)
        .collect();
    if markers.len() != expected.sentinels {
        return Err(OracleError::SentinelCount {
            expected: expected.sentinels,
            observed: markers.len(),
        });
    }
    let derived_markers: Vec<usize> = if expected.sentinels == 1 {
        vec![expected.half]
    } else {
        vec![expected.half, expected.half + 1]
    };
    if markers != derived_markers {
        return Err(OracleError::SentinelIndex {
            expected: derived_markers,
            observed: markers,
        });
    }

    let real: Vec<usize> = (0..observed.bead_widths.len())
        .filter(|index| !markers.contains(index))
        .collect();
    if real.len() != expected.cap {
        return Err(OracleError::RealBeadCount {
            expected: expected.cap,
            observed: real.len(),
        });
    }

    for (ordinal, &index) in real.iter().enumerate() {
        let width = observed.bead_widths[index];
        if !width_envelope.contains(width, expected.bead_width) {
            return Err(OracleError::WidthMismatch {
                index,
                expected: expected.bead_width,
                observed: width,
                tolerance: width_envelope.tolerance(expected.bead_width),
            });
        }
        // The left half (and every bead at the cap) keeps the equal spacing
        // `(i + 1/2) * w`. Only the over-cap right half is re-mirrored about
        // the region centre, so bead `ordinal` there corresponds to left-half
        // ordinal `cap - 1 - ordinal`.
        let mirrored = if expected.sentinels >= 2 && ordinal >= expected.half {
            Some((expected.cap - 1 - ordinal) as f64)
        } else {
            None
        };
        let model = match mirrored {
            Some(mirrored) => expected.thickness - (mirrored + 0.5) * expected.bead_width,
            None => (ordinal as f64 + 0.5) * expected.bead_width,
        };
        let location = observed.toolpath_locations[index];
        if !location_envelope.contains(location, model) {
            return Err(OracleError::LocationMismatch {
                index,
                expected: model,
                observed: location,
                tolerance: location_envelope.tolerance(model),
            });
        }
    }

    let derived_marker_locations: Vec<f64> = if expected.sentinels == 1 {
        vec![expected.half as f64 * expected.bead_width]
    } else {
        vec![
            expected.half as f64 * expected.bead_width,
            expected.thickness - expected.half as f64 * expected.bead_width,
        ]
    };
    for (&index, &model) in derived_markers.iter().zip(derived_marker_locations.iter()) {
        let width = observed.bead_widths[index];
        if width != 0.0 {
            return Err(OracleError::SentinelWidth { index, width });
        }
        let location = observed.toolpath_locations[index];
        if !location_envelope.contains(location, model) {
            return Err(OracleError::LocationMismatch {
                index,
                expected: model,
                observed: location,
                tolerance: location_envelope.tolerance(model),
            });
        }
    }

    if !width_envelope.contains(observed.left_over, expected.left_over) {
        return Err(OracleError::LeftOverMismatch {
            expected: expected.left_over,
            observed: observed.left_over,
            tolerance: width_envelope.tolerance(expected.left_over),
        });
    }
    if !width_envelope.contains(observed.total_thickness, expected.thickness) {
        return Err(OracleError::TotalThicknessMismatch {
            expected: expected.thickness,
            observed: observed.total_thickness,
            tolerance: width_envelope.tolerance(expected.thickness),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Generated case and its derived parameters.
// ─────────────────────────────────────────────────────────────────────────────

/// Smallest generated distribution radius.
const MIN_DISTRIBUTION_COUNT: u64 = 1;
/// Largest widening "step" (`step / 8` of a width).
const MAX_WIDENING_STEP: u64 = 7;
/// Largest widening delegation scale (`scale / 8` of a width).
const MAX_WIDENING_SCALE: u64 = 15;

/// One generated beading case.
///
/// The selectors ride along with the dyadic lengths so that a single case
/// carries every branch; see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeadingCase {
    /// Numerator of the dyadic bead width `w`, in units.
    pub width_units: u64,
    /// Right shift of `w` (denominator `2^shift`).
    pub shift: u32,
    /// Offset above three for the distributed `n >= 3` count.
    pub distributed_extra: u64,
    /// Numerator of the free thickness used by counts 0, 1 and 2.
    pub thickness_units: u64,
    /// Distribution radius (bead counts), always nonzero.
    pub distribution_count: u64,
    /// Half of the limited even cap: cap `= 2 * cap_half`.
    pub cap_half: u64,
    /// Numerator of the over-cap surplus.
    pub excess_units: u64,
    /// Widening "step" selector, `1..=7`, for the sub-minimum thickness.
    pub widening_step: u64,
    /// Widening "scale" selector, `1..=15`, for the delegated thickness.
    pub widening_scale: u64,
}

/// Build the `H-ARACHNE-BEADING` strategy. Every bound failure surfaces here,
/// before any value is sampled.
pub fn strategy(limits: &BeadingLimits) -> Result<BoxedStrategy<BeadingCase>, BoundsError> {
    let max_width = limits.max_width_units;
    let max_beads = u64::try_from(limits.max_beads).map_err(|_| BoundsError::Arithmetic {
        what: "max_beads as u64".to_string(),
    })?;
    let extra_max = max_beads
        .checked_sub(MIN_MAX_BEADS)
        .ok_or(BoundsError::BelowWitness {
            name: "max_beads".to_string(),
            value: max_beads,
            witness: "distributed three-bead case",
            minimum: MIN_MAX_BEADS,
        })?;
    let (cap, _) = witness_cap(max_beads)?;
    let cap_half_max = cap / 2;
    if cap_half_max == 0 {
        return Err(BoundsError::BelowWitness {
            name: "max_beads".to_string(),
            value: max_beads,
            witness: "limited even cap",
            minimum: MIN_MAX_BEADS,
        });
    }
    let excess_max = limits.max_excess_units;

    Ok((
        (1..=max_width),
        (0..=MAX_DYADIC_SHIFT),
        (0..=extra_max),
        (1..=max_width),
        (MIN_DISTRIBUTION_COUNT..=MAX_DISTRIBUTION_COUNT),
        (1..=cap_half_max),
        (1..=excess_max),
        (1..=MAX_WIDENING_STEP),
        (1..=MAX_WIDENING_SCALE),
    )
        .prop_map(
            move |(
                width_units,
                shift,
                distributed_extra,
                thickness_units,
                distribution_count,
                cap_half,
                excess_units,
                widening_step,
                widening_scale,
            )| BeadingCase {
                width_units,
                shift,
                distributed_extra,
                thickness_units,
                distribution_count,
                cap_half,
                excess_units,
                widening_step,
                widening_scale,
            },
        )
        .boxed())
}

/// Exact dyadic `units / 2^shift`.
///
/// `units <= 2^53` and the divisor is a power of two, so the quotient is a
/// dyadic rational that `f64` represents without rounding.
fn dyadic(units: u64, shift: u32) -> f64 {
    units as f64 / f64::from(1u32 << shift)
}

/// Every length and count one case derives, all from validated inputs.
struct Derived {
    w: f64,
    distribution_count: usize,
    n_beads: usize,
    thickness: f64,
    cap: usize,
    cap_half: usize,
    at_cap_thickness: f64,
    over_cap_thickness: f64,
    excess: f64,
    min_input: f64,
    min_output: f64,
    under_min_thickness: f64,
    middle_thickness: f64,
    delegated_thickness: f64,
}

impl Derived {
    fn new(case: &BeadingCase, limits: &BeadingLimits) -> Result<Self, String> {
        let max_beads = u64::try_from(limits.max_beads)
            .map_err(|_| "max_beads does not fit u64".to_string())?;
        let w = dyadic(case.width_units, case.shift);
        if !(w.is_finite() && w > 0.0) {
            return Err(format!("generated width {w} is not finite and positive"));
        }

        let n_units = case
            .distributed_extra
            .checked_add(MIN_MAX_BEADS)
            .ok_or_else(|| "distributed bead count overflowed".to_string())?;
        if n_units > max_beads {
            return Err(format!(
                "distributed bead count {n_units} exceeds max_beads {max_beads}"
            ));
        }
        let n_beads =
            to_usize("distributed bead count", n_units).map_err(|error| error.to_string())?;

        let excess = dyadic(case.excess_units, case.shift);
        let cap_units = case
            .cap_half
            .checked_mul(2)
            .ok_or_else(|| "limited cap overflowed".to_string())?;
        let requested = cap_units
            .checked_add(1)
            .ok_or_else(|| "limited requested count overflowed".to_string())?;
        if requested > max_beads {
            return Err(format!(
                "limited cap {cap_units} plus its over-cap case needs {requested} > max_beads {max_beads}"
            ));
        }
        let cap = to_usize("limited cap", cap_units).map_err(|error| error.to_string())?;
        let cap_half =
            to_usize("limited cap half", case.cap_half).map_err(|error| error.to_string())?;

        let at_cap_thickness = cap_units as f64 * w;
        let over_cap_thickness = at_cap_thickness + excess;
        // `min_input_width = w / 2`, `min_output_width = 3 w / 4`: the middle
        // window `[w/2, 1.375 w)` then exercises both sides of the
        // `max(thickness, min_output_width)` clamp.
        let min_input = w / 2.0;
        let min_output = 3.0 * w / 4.0;
        let step = case.widening_step as f64 / 8.0;
        let under_min_thickness = min_input * step;
        let middle_thickness = min_input + w * step;
        let delegated_thickness = w * (case.widening_scale as f64 / 8.0);

        Ok(Self {
            w,
            distribution_count: to_usize("distribution_count", case.distribution_count)
                .map_err(|error| error.to_string())?,
            n_beads,
            thickness: dyadic(case.thickness_units, case.shift),
            cap,
            cap_half,
            at_cap_thickness,
            over_cap_thickness,
            excess,
            min_input,
            min_output,
            under_min_thickness,
            middle_thickness,
            delegated_thickness,
        })
    }
}

fn distributed(width: f64, distribution_count: usize) -> DistributedBeadingStrategy {
    DistributedBeadingStrategy::new(
        width,
        0.0,
        0.0,
        distribution_count,
        WALL_TRANSITION_ANGLE,
        MIDDLE_THRESHOLD,
        MIDDLE_THRESHOLD,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Property body.
// ─────────────────────────────────────────────────────────────────────────────

/// Registry entry point for `H-ARACHNE-BEADING`.
pub fn run_property() {
    let run =
        PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("use the registry-backed property command");
    // Validate the profile before any strategy exists; `check_case` never sees
    // an unusable bound.
    let raw = RawBeadingBounds::read(&run).expect("H-ARACHNE-BEADING bounds");
    let limits = validate_limits(raw).expect("H-ARACHNE-BEADING profile");
    let envelopes = Envelopes::derive(&limits).expect("H-ARACHNE-BEADING envelopes");
    let strategy = strategy(&limits).expect("H-ARACHNE-BEADING strategy");
    run.check(&strategy, |case, _guard| -> TestCaseResult {
        check_case(&case, &limits, &envelopes).map_err(TestCaseError::fail)
    })
    .expect("H-ARACHNE-BEADING charter");
}

/// Check one generated case against every branch of the charter.
pub fn check_case(
    case: &BeadingCase,
    limits: &BeadingLimits,
    envelopes: &Envelopes,
) -> Result<(), String> {
    let derived = Derived::new(case, limits)?;
    check_distributed(&derived, envelopes)?;
    check_limited(&derived, envelopes)?;
    check_widening(&derived, envelopes)?;
    Ok(())
}

fn close(envelope: &Envelope, what: &str, observed: f64, expected: f64) -> Result<(), String> {
    if envelope.contains(observed, expected) {
        Ok(())
    } else {
        Err(format!(
            "{what}: derived {expected}, observed {observed} (envelope {})",
            envelope.tolerance(expected)
        ))
    }
}

fn parallel(what: &str, beading: &Beading) -> Result<(), String> {
    check_parallel(beading).map_err(|error| format!("{what}: {error}"))
}

fn distinct_len(what: &str, observed: usize, expected: usize) -> Result<(), String> {
    if observed == expected {
        Ok(())
    } else {
        Err(format!(
            "{what}: derived {expected} entries, observed {observed}"
        ))
    }
}

/// `DistributedBeadingStrategy` zero/one/two/equal-spacing relations.
fn check_distributed(derived: &Derived, envelopes: &Envelopes) -> Result<(), String> {
    let strategy = distributed(derived.w, derived.distribution_count);
    let thickness = derived.thickness;

    // count 0: no beads at all, the whole thickness unprinted.
    let none = strategy.compute(thickness, 0);
    parallel("Distributed count 0", &none)?;
    distinct_len("Distributed count 0 widths", none.bead_widths.len(), 0)?;
    distinct_len(
        "Distributed count 0 locations",
        none.toolpath_locations.len(),
        0,
    )?;
    close(
        &envelopes.width,
        "Distributed count 0 left_over",
        none.left_over,
        thickness,
    )?;
    close(
        &envelopes.width,
        "Distributed count 0 total_thickness",
        none.total_thickness,
        thickness,
    )?;

    // count 1: one full-thickness bead, centred.
    let one = strategy.compute(thickness, 1);
    parallel("Distributed count 1", &one)?;
    distinct_len("Distributed count 1 widths", one.bead_widths.len(), 1)?;
    close(
        &envelopes.width,
        "Distributed count 1 width",
        one.bead_widths[0],
        thickness,
    )?;
    close(
        &envelopes.location,
        "Distributed count 1 location",
        one.toolpath_locations[0],
        thickness / 2.0,
    )?;
    close(
        &envelopes.width,
        "Distributed count 1 left_over",
        one.left_over,
        0.0,
    )?;

    // count 2: two equal half-thickness beads at t/4 and 3t/4.
    let two = strategy.compute(thickness, 2);
    parallel("Distributed count 2", &two)?;
    distinct_len("Distributed count 2 widths", two.bead_widths.len(), 2)?;
    let half = thickness / 2.0;
    close(
        &envelopes.width,
        "Distributed count 2 width[0]",
        two.bead_widths[0],
        half,
    )?;
    close(
        &envelopes.width,
        "Distributed count 2 width[1]",
        two.bead_widths[1],
        half,
    )?;
    close(
        &envelopes.location,
        "Distributed count 2 location[0]",
        two.toolpath_locations[0],
        half / 2.0,
    )?;
    close(
        &envelopes.location,
        "Distributed count 2 location[1]",
        two.toolpath_locations[1],
        3.0 * thickness / 4.0,
    )?;
    close(
        &envelopes.width,
        "Distributed count 2 left_over",
        two.left_over,
        0.0,
    )?;

    // n >= 3 at t = n * w: the analytically equal-spaced special case.
    let n = derived.n_beads;
    if n < 3 {
        return Err(format!(
            "distributed equal-spacing case needs n >= 3, derived {n}"
        ));
    }
    let total = n as f64 * derived.w;
    let many = strategy.compute(total, n);
    parallel("Distributed n >= 3", &many)?;
    distinct_len("Distributed n >= 3 widths", many.bead_widths.len(), n)?;
    distinct_len(
        "Distributed n >= 3 locations",
        many.toolpath_locations.len(),
        n,
    )?;
    for (index, &width) in many.bead_widths.iter().enumerate() {
        close(
            &envelopes.width,
            &format!("Distributed n >= 3 width[{index}]"),
            width,
            derived.w,
        )?;
    }
    for (index, &location) in many.toolpath_locations.iter().enumerate() {
        close(
            &envelopes.location,
            &format!("Distributed n >= 3 location[{index}]"),
            location,
            (index as f64 + 0.5) * derived.w,
        )?;
    }
    close(
        &envelopes.width,
        "Distributed n >= 3 left_over",
        many.left_over,
        0.0,
    )?;
    close(
        &envelopes.width,
        "Distributed n >= 3 total_thickness",
        many.total_thickness,
        total,
    )?;
    Ok(())
}

/// `LimitedBeadingStrategy` at the even cap and one over it.
fn check_limited(derived: &Derived, envelopes: &Envelopes) -> Result<(), String> {
    let limited = LimitedBeadingStrategy::new(
        Box::new(distributed(derived.w, derived.distribution_count)),
        derived.cap,
    );

    let at_cap = limited.compute(derived.at_cap_thickness, derived.cap);
    let at_cap_expectation = LimitedExpectation {
        cap: derived.cap,
        half: derived.cap_half,
        bead_width: derived.w,
        thickness: derived.at_cap_thickness,
        sentinels: 1,
        left_over: 0.0,
    };
    compare_limited(
        &at_cap,
        &at_cap_expectation,
        &envelopes.width,
        &envelopes.location,
    )
    .map_err(|error| format!("Limited at even cap {}: {error}", derived.cap))?;

    let over_cap = limited.compute(derived.over_cap_thickness, derived.cap + 1);
    let over_cap_expectation = LimitedExpectation {
        cap: derived.cap,
        half: derived.cap_half,
        bead_width: derived.w,
        thickness: derived.over_cap_thickness,
        sentinels: 2,
        left_over: derived.excess,
    };
    compare_limited(
        &over_cap,
        &over_cap_expectation,
        &envelopes.width,
        &envelopes.location,
    )
    .map_err(|error| format!("Limited over even cap {}: {error}", derived.cap))?;

    // The two decorators must not be confused: the at-cap output carries exactly
    // one marker, the over-cap output exactly two.
    let at_cap_markers = at_cap
        .bead_widths
        .iter()
        .filter(|width| **width == 0.0)
        .count();
    let over_cap_markers = over_cap
        .bead_widths
        .iter()
        .filter(|width| **width == 0.0)
        .count();
    distinct_len("Limited at-cap markers", at_cap_markers, 1)?;
    distinct_len("Limited over-cap markers", over_cap_markers, 2)?;
    Ok(())
}

/// `WideningBeadingStrategy` below the input minimum, in the single-bead
/// window, and delegating at `bead_count >= 2`.
fn check_widening(derived: &Derived, envelopes: &Envelopes) -> Result<(), String> {
    let wrapping = WideningBeadingStrategy::new(
        Box::new(distributed(derived.w, derived.distribution_count)),
        derived.w,
        derived.min_input,
        derived.min_output,
    );
    let parent = distributed(derived.w, derived.distribution_count);

    // Below `min_input_width` with `bead_count <= 1`: nothing printable.
    for count in [0usize, 1] {
        let beading = wrapping.compute(derived.under_min_thickness, count);
        parallel("Widening below min_input_width", &beading)?;
        distinct_len(
            "Widening below min_input_width widths",
            beading.bead_widths.len(),
            0,
        )?;
        close(
            &envelopes.width,
            "Widening below min_input_width left_over",
            beading.left_over,
            derived.under_min_thickness,
        )?;
    }

    // `min_input_width <= t < get_transition_thickness(1)` with
    // `bead_count <= 1`: one centred bead, clamped up to `min_output_width`.
    let transition = wrapping.get_transition_thickness(1);
    if !(derived.middle_thickness < transition) {
        return Err(format!(
            "widening witness thickness {} is not below get_transition_thickness(1) {transition}",
            derived.middle_thickness
        ));
    }
    let clamped_width = if derived.middle_thickness >= derived.min_output {
        derived.middle_thickness
    } else {
        derived.min_output
    };
    for count in [0usize, 1] {
        let beading = wrapping.compute(derived.middle_thickness, count);
        parallel("Widening single-bead window", &beading)?;
        distinct_len(
            "Widening single-bead window widths",
            beading.bead_widths.len(),
            1,
        )?;
        close(
            &envelopes.width,
            "Widening single-bead window width",
            beading.bead_widths[0],
            clamped_width,
        )?;
        close(
            &envelopes.location,
            "Widening single-bead window location",
            beading.toolpath_locations[0],
            derived.middle_thickness / 2.0,
        )?;
        close(
            &envelopes.width,
            "Widening single-bead window left_over",
            beading.left_over,
            0.0,
        )?;
        // Widening may over-emit, so only the emitted bead is asserted; the
        // conservation identity is deliberately not required here.
    }

    // `bead_count >= 2`: the parent's own output, including below
    // `optimal_width`.
    for count in [2usize, derived.n_beads] {
        let direct = parent.compute(derived.delegated_thickness, count);
        distinct_len(
            "Widening delegated parent widths",
            direct.bead_widths.len(),
            count,
        )?;
        let wrapped = wrapping.compute(derived.delegated_thickness, count);
        parallel("Widening delegated output", &wrapped)?;
        if wrapped.bead_widths != direct.bead_widths
            || wrapped.toolpath_locations != direct.toolpath_locations
            || wrapped.left_over != direct.left_over
            || wrapped.total_thickness != direct.total_thickness
        {
            return Err(format!(
                "Widening at bead_count {count} and thickness {} must delegate to the parent: \
                 direct widths {:?} at {:?}, wrapped widths {:?} at {:?}",
                derived.delegated_thickness,
                direct.bead_widths,
                direct.toolpath_locations,
                wrapped.bead_widths,
                wrapped.toolpath_locations
            ));
        }
    }
    Ok(())
}
