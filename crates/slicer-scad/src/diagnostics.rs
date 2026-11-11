//! One fail-closed classifier for every diagnostic-producing path.
//!
//! Program evaluation, exact mesh render, and the contour/projection/lowering
//! path (including optional extrusion) all funnel through this module. Nothing
//! else may decide that a diagnostic is acceptable.
//!
//! # Why codes, not prose
//!
//! A host that must reject unclassified degradations cannot substring-match
//! English text: a rewording upstream silently promotes an unknown diagnostic
//! to "harmless", and matching generic words such as `warning` is not an
//! allowlist at all. The engine therefore tags every diagnostic with a stable
//! code (`openrscad` `Warning::code`, `GeomMessage::code`, patches 0001/0002 in
//! `vendor/openrscad/PATCHES.md`), and this module holds the **complete** table
//! of codes admitted at that pin.
//!
//! # Revision binding
//!
//! This table is bound to `openrscad@1a45bd33f2886c32bda32fe6e8358a67ab03f5ef`
//! plus the vendored patch set — not to OpenRSCAD in general. Bumping the pin
//! without re-deriving the table fails closed (new codes fall through to
//! [`ScadDiagnosticClass::Unknown`]), which is the intended behaviour: silent
//! acceptance is the failure this module exists to prevent.
//!
//! # Precedence
//!
//! When several diagnostics are present the primary error is the most severe in
//! a fixed order — external request, then partial, degraded, approximate,
//! unknown, then conversion. The order is total and does not depend on the order
//! upstream happened to emit diagnostics in.

use crate::{ScadDiagnostic, ScadDiagnosticClass, ScadDiagnosticStage, ScadErrorKind};

/// Adapter code for one evaluated `echo()` line.
pub(crate) const SCAD_CODE_ECHO: &str = "scad_echo";

/// The channel a diagnostic arrived on. The same code can be benign on one
/// channel and not on another, so classification is per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Channel {
    /// `openrscad_eval::Warning` plus evaluated echoes.
    Evaluation,
    /// `openrscad_geom::GeomMessage` (mesh render or contour render).
    Render,
}

/// Classify one upstream evaluation warning code.
///
/// Complete for the pinned revision: every other code, including the engine's
/// own `other` catch-all, is [`ScadDiagnosticClass::Unknown`].
pub(crate) fn classify_evaluation_code(code: &str) -> ScadDiagnosticClass {
    match code {
        // The verified overwritten-assignment lint: the *earlier* write is dead,
        // the model is exact. Verified against the pin's
        // `Interp::warn_overwritten` and its source in
        // `docs/scad/openrscad-compatibility.md`.
        "assignment_overwritten" => ScadDiagnosticClass::Information,
        // A root-level statement was dropped after a recursion-depth abort; the
        // retained geometry is not the whole source.
        "recursion_aborted" => ScadDiagnosticClass::Partial,
        // Geometry was omitted (`roof()` on a concave/holed profile) or
        // approximated (non-convex or over-large 3D `minkowski()`).
        "roof_omitted" | "minkowski_approximation" => ScadDiagnosticClass::Approximate,
        // No other evaluation code is admitted at this revision.
        _ => ScadDiagnosticClass::Unknown,
    }
}

/// Classify one upstream geometry message code.
///
/// `errors` and `warnings` are classified by the same table: the split between
/// them is upstream's UI hint, not a product-acceptance distinction, and a
/// warning that describes skipped geometry is exactly as unacceptable as a
/// recoverable error.
pub(crate) fn classify_render_code(code: &str) -> ScadDiagnosticClass {
    match code {
        // A CSG boolean/hull op failed and the operands were re-emitted
        // un-combined: geometry is present but wrong.
        "boolean_fallback" => ScadDiagnosticClass::Degraded,
        "roof_omitted" | "minkowski_approximation" => ScadDiagnosticClass::Approximate,
        // No other render code is admitted at this revision.
        _ => ScadDiagnosticClass::Unknown,
    }
}

/// Classify `code` on `channel`.
pub(crate) fn classify(channel: Channel, code: &str) -> ScadDiagnosticClass {
    match channel {
        Channel::Evaluation => classify_evaluation_code(code),
        Channel::Render => classify_render_code(code),
    }
}

/// Build a classified diagnostic without admitting it for success.
pub(crate) fn make_diagnostic(
    channel: Channel,
    stage: ScadDiagnosticStage,
    code: &str,
    message: impl Into<String>,
    span: Option<std::ops::Range<usize>>,
) -> ScadDiagnostic {
    ScadDiagnostic {
        stage,
        class: classify(channel, code),
        code: code.to_string(),
        message: message.into(),
        span,
    }
}

/// Build the informational diagnostic for one evaluated `echo()` line.
pub(crate) fn echo_diagnostic(message: String) -> ScadDiagnostic {
    ScadDiagnostic {
        stage: ScadDiagnosticStage::Evaluation,
        class: ScadDiagnosticClass::Information,
        code: SCAD_CODE_ECHO.to_string(),
        message,
        span: None,
    }
}

/// Build an adapter-originated diagnostic (conversion, option and resource
/// failures). These are always [`ScadDiagnosticClass::Error`] because they are
/// produced after the classifier has already refused the result.
pub(crate) fn error_diagnostic(
    stage: ScadDiagnosticStage,
    code: &str,
    message: impl Into<String>,
    span: Option<std::ops::Range<usize>>,
) -> ScadDiagnostic {
    ScadDiagnostic {
        stage,
        class: ScadDiagnosticClass::Error,
        code: code.to_string(),
        message: message.into(),
        span,
    }
}

/// Map a non-information diagnostic class to the primary error kind, or `None`
/// when every diagnostic is information.
///
/// The returned value is the **most severe** class present, using the fixed
/// precedence documented on this module.
pub(crate) fn primary_error_kind(diagnostics: &[ScadDiagnostic]) -> Option<ScadErrorKind> {
    let mut worst: Option<(u8, ScadErrorKind)> = None;
    for diagnostic in diagnostics {
        let rank = match diagnostic.class {
            ScadDiagnosticClass::Information => continue,
            ScadDiagnosticClass::Partial => (0, ScadErrorKind::PartialOutput),
            ScadDiagnosticClass::Degraded => (1, ScadErrorKind::DegradedOutput),
            ScadDiagnosticClass::Approximate => (2, ScadErrorKind::ApproximateOutput),
            ScadDiagnosticClass::Unknown => (3, ScadErrorKind::UnknownDiagnostic),
            ScadDiagnosticClass::Error => (4, ScadErrorKind::Evaluation),
        };
        match worst {
            Some((best_rank, _)) if best_rank <= rank.0 => {}
            _ => worst = Some(rank),
        }
    }
    worst.map(|(_, kind)| kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scad_diag(class: ScadDiagnosticClass) -> ScadDiagnostic {
        // `ScadDiagnostic` has no `Default` on purpose (no fabricated span, stage
        // or code), so this fixture spells out every field; a new field must be
        // classified here.
        ScadDiagnostic { // exhaustive: the pinned data contract; no Default may be invented for a diagnostic
            stage: ScadDiagnosticStage::Evaluation,
            class,
            code: "probe".to_string(),
            message: "probe".to_string(),
            span: None,
        }
    }

    #[test]
    fn scad_diagnostic_channels_fail_closed() {
        // Every code the pinned engine can emit on each channel is either
        // deliberately admitted or rejected; nothing falls through to a default
        // that happens to be benign.
        assert_eq!(
            classify_evaluation_code("assignment_overwritten"),
            ScadDiagnosticClass::Information
        );
        assert_eq!(
            classify_evaluation_code("recursion_aborted"),
            ScadDiagnosticClass::Partial
        );
        assert_eq!(
            classify_evaluation_code("roof_omitted"),
            ScadDiagnosticClass::Approximate
        );
        assert_eq!(
            classify_evaluation_code("minkowski_approximation"),
            ScadDiagnosticClass::Approximate
        );
        // The engine's own catch-all must not become information.
        assert_eq!(
            classify_evaluation_code("other"),
            ScadDiagnosticClass::Unknown
        );

        assert_eq!(
            classify_render_code("boolean_fallback"),
            ScadDiagnosticClass::Degraded
        );
        assert_eq!(
            classify_render_code("roof_omitted"),
            ScadDiagnosticClass::Approximate
        );
        assert_eq!(
            classify_render_code("minkowski_approximation"),
            ScadDiagnosticClass::Approximate
        );
        assert_eq!(classify_render_code("other"), ScadDiagnosticClass::Unknown);
    }

    #[test]
    fn scad_unknown_diagnostic_codes_are_injected_and_rejected() {
        // A code nobody classified — the exact shape of a future upstream
        // warning — must be Unknown on both channels.
        for channel in [Channel::Evaluation, Channel::Render] {
            let diagnostic = make_diagnostic(
                channel,
                ScadDiagnosticStage::Render,
                "a_warning_invented_after_the_pin",
                "geometry looks fine, probably",
                None,
            );
            assert_eq!(diagnostic.class, ScadDiagnosticClass::Unknown);
            assert_eq!(
                primary_error_kind(std::slice::from_ref(&diagnostic)),
                Some(ScadErrorKind::UnknownDiagnostic)
            );
        }
    }

    #[test]
    fn scad_primary_error_precedence_is_total() {
        // Order: partial, degraded, approximate, unknown. Reversing the input
        // must not change the verdict.
        let worst_first = vec![
            scad_diag(ScadDiagnosticClass::Unknown),
            scad_diag(ScadDiagnosticClass::Approximate),
            scad_diag(ScadDiagnosticClass::Degraded),
            scad_diag(ScadDiagnosticClass::Partial),
        ];
        let worst_last = worst_first.iter().cloned().rev().collect::<Vec<_>>();
        assert_eq!(
            primary_error_kind(&worst_first),
            Some(ScadErrorKind::PartialOutput)
        );
        assert_eq!(
            primary_error_kind(&worst_last),
            Some(ScadErrorKind::PartialOutput)
        );

        // Information alone is success.
        assert_eq!(
            primary_error_kind(&[scad_diag(ScadDiagnosticClass::Information)]),
            None
        );
    }

    #[test]
    fn scad_echo_is_information_and_keeps_its_code() {
        let diagnostic = echo_diagnostic("ECHO: 1".to_string());
        assert_eq!(diagnostic.class, ScadDiagnosticClass::Information);
        assert_eq!(diagnostic.code, SCAD_CODE_ECHO);
        assert_eq!(diagnostic.stage, ScadDiagnosticStage::Evaluation);
    }
}
