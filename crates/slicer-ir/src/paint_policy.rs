//! Per-region support-paint policy.
//!
//! Canonical home for the three-variant `SupportPaintPolicy` enum that
//! describes the per-region eligibility decision for the support stage's
//! paint precedence rules. Re-exported from `slicer_core::paint_policy`
//! and `slicer_sdk::traits` so that match-arms in the consumer modules
//! (`tree-support`, `traditional-support`) keep their existing
//! `slicer_sdk::traits::SupportPaintPolicy::Blocked` paths while the
//! underlying type lives in a single crate with no `slicer-core` ↔
//! `slicer-sdk` cycle.

/// Support-paint policy for a per-region eligibility decision.
///
/// Computed from D14 `SlicedRegion.segment_annotations` queries. Used by
/// both `tree-support` and `traditional-support` modules to honour
/// SupportEnforcer (force-on) and SupportBlocker (force-off) paint
/// annotations, with blocker > enforcer precedence
/// (`docs/10` §"Scenario Trace 2").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportPaintPolicy {
    /// At least one SupportBlocker annotation covers this region — skip
    /// support regardless of overhang-angle or `needs_support`.
    Blocked,
    /// At least one SupportEnforcer annotation covers this region (and no
    /// blocker) — generate support regardless of overhang-angle or
    /// `needs_support`.
    Enforced,
    /// No paint policy override — defer to overhang-angle / `needs_support`.
    DefaultEligible,
}
