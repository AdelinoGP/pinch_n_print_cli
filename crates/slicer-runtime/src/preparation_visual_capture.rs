//! Opt-in typed preparation visual capture companions (packet 06, Step 11).
//!
//! [`PreparationVisualCapture`] is the visual-debug companion to #05's
//! metadata-only [`PreparationCapture`]: it carries the #05 capture unchanged,
//! shared post-Ready typed diagnostic snapshots, and the execution-versus-
//! rendering evidence axes the bundle reports. It never holds a raw plan
//! payload, piece bytes, a module codec, or a live plan handle — the typed
//! snapshot `Arc`s it retains survive owner disposal because the committed
//! store shares immutable validated public geometry, not private authority.

use std::fmt;

use slicer_ir::{ModuleId, StageId};
use slicer_sdk::preparation::{PreparationLayer, PreparationProjectionError};

use crate::layer_executor::{
    CaptureExecutionError, StageCapture, BLACKBOARD_TAP_STAGE_IDS, POSTPASS_TAP_STAGE_IDS,
    SUPPORTED_TAP_STAGE_IDS,
};
use crate::preparation_lifecycle::PreparationCapture;
use crate::run::SliceRunError;

/// One requested owner-scoped preparation view for one print.
///
/// The module id must name a selected preparation owner and the view must be
/// declared by that owner; resolution happens before any module invocation,
/// so a request that reaches the preparation closure is already validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationViewRequest {
    /// Fully qualified selected module identifier.
    pub module_id: ModuleId,
    /// Exact declared diagnostic view name.
    pub view: String,
}

/// Typed whole-print preparation visual snapshot; no opaque plan authority.
///
/// Every field is private and read through getters so the snapshot stays a
/// redacted, post-disposal-safe view: the only retained diagnostic data is
/// shared immutable typed scene geometry plus the #05 metadata capture.
#[derive(Debug)]
pub struct PreparationVisualCapture {
    metadata: PreparationCapture,
    captures: Vec<StageCapture>,
    diagnostic_accounting: Vec<(ModuleId, u64)>,
    executed_stage_ids: Vec<StageId>,
    executed_layer_indices: Vec<u32>,
    rendered_layer_indices: Vec<u32>,
    schedule: Vec<PreparationLayer>,
}

impl PreparationVisualCapture {
    /// #05's unchanged metadata-only preparation capture.
    pub fn metadata(&self) -> &PreparationCapture {
        &self.metadata
    }

    /// Retained typed tap captures, sharing whole-print scene `Arc`s.
    pub fn captures(&self) -> &[StageCapture] {
        &self.captures
    }

    /// Retained diagnostic logical bytes per owner, before disposal.
    pub fn diagnostic_accounting(&self) -> &[(ModuleId, u64)] {
        &self.diagnostic_accounting
    }

    /// Per-layer stage ids the visual closure actually executed.
    pub fn executed_stage_ids(&self) -> &[StageId] {
        &self.executed_stage_ids
    }

    /// Global layer indices actually executed, distinct from rendering.
    pub fn executed_layer_indices(&self) -> &[u32] {
        &self.executed_layer_indices
    }

    /// Global layer indices selected for rendering, distinct from execution.
    pub fn rendered_layer_indices(&self) -> &[u32] {
        &self.rendered_layer_indices
    }

    /// Committed global schedule the print actually prepared against.
    pub fn schedule(&self) -> &[PreparationLayer] {
        &self.schedule
    }

    /// Assemble one capture from independently captured redacted parts.
    ///
    /// Crate-private on purpose: only the run facade constructs this
    /// companion, after the print committed its preparation and before any
    /// private store handle is retained. No argument may carry a raw plan
    /// payload, piece bytes, or a plan handle.
    pub(crate) fn from_parts(
        metadata: PreparationCapture,
        captures: Vec<StageCapture>,
        diagnostic_accounting: Vec<(ModuleId, u64)>,
        executed_stage_ids: Vec<StageId>,
        executed_layer_indices: Vec<u32>,
        rendered_layer_indices: Vec<u32>,
        schedule: Vec<PreparationLayer>,
    ) -> Self {
        Self {
            metadata,
            captures,
            diagnostic_accounting,
            executed_stage_ids,
            executed_layer_indices,
            rendered_layer_indices,
            schedule,
        }
    }
}

/// Typed failure modes of the preparation visual capture path.
///
/// Separate from [`SliceRunError`], which stays an unwidened `pub String`
/// tuple: this enum carries the named owner/view/layer identities and the
/// typed projection reason so callers match by variant, never by parsing a
/// formatted slice message.
#[derive(Debug)]
pub enum PreparationVisualCaptureError {
    /// The request shape is invalid before any module invocation.
    InvalidRequest {
        /// What made the request invalid.
        detail: String,
    },
    /// The requested module id is not a selected preparation owner.
    UnknownOwner {
        /// The offending module id, verbatim from the request.
        module_id: ModuleId,
    },
    /// The requested view is not declared by the selected owner.
    UnknownView {
        /// The owner the request named.
        module_id: ModuleId,
        /// The offending view name, verbatim from the request.
        view: String,
    },
    /// The selected owner does not support preparation projection.
    UnsupportedOwner {
        /// The owner that cannot publish typed diagnostics.
        module_id: ModuleId,
    },
    /// The selected owner has no eligible participation in this print.
    InactiveOwner {
        /// The owner that never entered preparation.
        module_id: ModuleId,
    },
    /// The requested global layer index does not exist in the committed
    /// schedule.
    UnknownLayer {
        /// The offending global layer index.
        global_layer_index: u32,
    },
    /// The owner rejected the projection with a typed diagnostic reason.
    ProjectionRejected {
        /// The owner whose projection was rejected.
        module_id: ModuleId,
        /// The view that was rejected.
        view: String,
        /// The typed projection failure, never a parsed message.
        reason: PreparationProjectionError,
    },
    /// The underlying slice failed with its original error.
    Slice(SliceRunError),
}

impl fmt::Display for PreparationVisualCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest { detail } => {
                write!(f, "invalid preparation visual request: {detail}")
            }
            Self::UnknownOwner { module_id } => {
                write!(f, "unknown preparation owner: '{module_id}'")
            }
            Self::UnknownView { module_id, view } => {
                write!(
                    f,
                    "preparation view '{view}' is not declared by owner '{module_id}'"
                )
            }
            Self::UnsupportedOwner { module_id } => {
                write!(
                    f,
                    "preparation owner does not support projection: '{module_id}'"
                )
            }
            Self::InactiveOwner { module_id } => {
                write!(
                    f,
                    "preparation owner is inactive in this print: '{module_id}'"
                )
            }
            Self::UnknownLayer { global_layer_index } => {
                write!(f, "unknown global layer index: {global_layer_index}")
            }
            Self::ProjectionRejected {
                module_id,
                view,
                reason,
            } => {
                write!(
                    f,
                    "preparation projection rejected for '{module_id}' view '{view}': {reason}"
                )
            }
            Self::Slice(error) => write!(f, "slice failed: {error}"),
        }
    }
}

impl std::error::Error for PreparationVisualCaptureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ProjectionRejected { reason, .. } => Some(reason),
            Self::Slice(error) => Some(error),
            _ => None,
        }
    }
}

/// Requested visual taps partitioned into ADR-0040's three tap classes.
///
/// Partition order is the request order within each class, so the per-layer
/// and blackboard executors keep receiving their taps exactly as requested.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct PreparationVisualTapClasses {
    /// `Layer::*` taps served by the truncated per-layer closure.
    pub(crate) per_layer: Vec<StageId>,
    /// Prepass taps served by reads of already-committed Blackboard slots.
    pub(crate) blackboard: Vec<StageId>,
    /// Whole-print taps this truncated visual path cannot forward.
    pub(crate) postpass: Vec<StageId>,
}

/// Partition requested taps into the three ADR-0040 capture classes.
///
/// An undocumented tap stays an ordinary [`CaptureExecutionError::UnknownTap`]:
/// classification is not a place to invent a new rejection category.
pub(crate) fn classify_preparation_visual_taps(
    stage_taps: &[StageId],
) -> Result<PreparationVisualTapClasses, CaptureExecutionError> {
    let mut classes = PreparationVisualTapClasses::default();
    for tap in stage_taps {
        if SUPPORTED_TAP_STAGE_IDS.contains(&tap.as_str()) {
            classes.per_layer.push(tap.clone());
        } else if BLACKBOARD_TAP_STAGE_IDS.contains(&tap.as_str()) {
            classes.blackboard.push(tap.clone());
        } else if POSTPASS_TAP_STAGE_IDS.contains(&tap.as_str()) {
            classes.postpass.push(tap.clone());
        } else {
            return Err(CaptureExecutionError::UnknownTap { tap: tap.clone() });
        }
    }
    Ok(classes)
}

/// Named fail-closed error for a requested whole-print postpass tap.
///
/// ADR-0040's furthest-stage rule: a postpass tap sources from the full
/// prepass -> all per-layer stages over all layers -> layer finalization ->
/// postpass prefix, not from a selected-layer truncation. The preparation
/// visual path disposes the print scope immediately after preparation and
/// never enters that prefix, so it must fail closed here rather than run only
/// the truncated closure and claim a whole-print capture it did not perform.
/// Returns `None` when no postpass tap was requested.
pub(crate) fn postpass_tap_closure_not_forwarded(
    postpass_taps: &[StageId],
) -> Option<PreparationVisualCaptureError> {
    let tap = postpass_taps.first()?;
    Some(PreparationVisualCaptureError::Slice(SliceRunError(
        format!("preparation visual postpass tap closure is not forwarded: {tap}"),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn companion_getters_keep_execution_and_rendering_distinct() {
        let metadata = PreparationCapture {
            retained_bytes: 4096,
            ..PreparationCapture::default()
        };
        let capture = PreparationVisualCapture::from_parts(
            metadata,
            Vec::new(),
            vec![("test.preparation-visual".into(), 96u64)],
            vec!["Layer::PathOptimization".into()],
            vec![1u32],
            vec![0u32, 2u32],
            Vec::new(),
        );
        assert_eq!(capture.metadata().retained_bytes, 4096);
        assert!(capture.captures().is_empty());
        assert_eq!(
            capture.diagnostic_accounting(),
            &[("test.preparation-visual".to_owned(), 96u64)]
        );
        assert_eq!(
            capture.executed_stage_ids(),
            &["Layer::PathOptimization".to_owned()]
        );
        // The two evidence axes stay separate sets: a rendered-but-not-
        // executed layer must not collapse into the executed set, and vice
        // versa.
        assert_eq!(capture.executed_layer_indices(), &[1u32]);
        assert_eq!(capture.rendered_layer_indices(), &[0u32, 2u32]);
        assert!(capture.schedule().is_empty());
    }

    #[test]
    fn projection_rejection_sources_the_typed_reason() {
        let error = PreparationVisualCaptureError::ProjectionRejected {
            module_id: "test.preparation-visual".into(),
            view: "probe_xy".into(),
            reason: PreparationProjectionError::UndeclaredView,
        };
        let rendered = error.to_string();
        assert!(rendered.contains("test.preparation-visual"));
        assert!(rendered.contains("probe_xy"));
        // Display of the typed reason is its canonical WIT variant name.
        assert!(rendered.contains("undeclared-view"));
        let source = std::error::Error::source(&error).expect("typed reason is the error source");
        assert_eq!(
            source.downcast_ref::<PreparationProjectionError>(),
            Some(&PreparationProjectionError::UndeclaredView)
        );
    }

    #[test]
    fn slice_failure_keeps_the_original_unwidened_error() {
        let error = PreparationVisualCaptureError::Slice(SliceRunError("boom".into()));
        assert!(error.to_string().contains("boom"));
        let source = std::error::Error::source(&error).expect("slice error is the error source");
        let slice_error = source
            .downcast_ref::<SliceRunError>()
            .expect("unwidened SliceRunError source");
        assert_eq!(slice_error.0, "boom");
    }

    #[test]
    fn extension_views_reach_the_print_scope_and_default_stays_empty() {
        let (_, _, scope) = crate::preparation_lifecycle::SliceRuntimeExtensions::default()
            .into_run_parts(false, false);
        assert!(scope.preparation_views().is_empty());

        let (_, _, scope) = crate::preparation_lifecycle::SliceRuntimeExtensions::default()
            .with_preparation_views(vec![PreparationViewRequest {
                module_id: "test.preparation-visual".into(),
                view: "probe_xy".into(),
            }])
            .into_run_parts(false, false);
        assert_eq!(
            scope.preparation_views(),
            &[PreparationViewRequest {
                module_id: "test.preparation-visual".to_owned(),
                view: "probe_xy".to_owned(),
            }]
        );
    }

    #[test]
    fn postpass_tap_fails_closed_with_named_closure_error() {
        // A mixed request is partitioned, not rejected wholesale: the
        // per-layer tap keeps its truncated-closure class.
        let classes = classify_preparation_visual_taps(&[
            "Layer::Perimeters".to_owned(),
            "PostPass::GCodeEmit".to_owned(),
        ])
        .expect("documented taps classify instead of reporting UnknownTap");
        assert_eq!(classes.per_layer, ["Layer::Perimeters".to_owned()]);
        assert!(classes.blackboard.is_empty());
        assert_eq!(classes.postpass, ["PostPass::GCodeEmit".to_owned()]);

        // The whole-print postpass closure request fails with its named
        // carrier, naming the exact tap; it never becomes a successful parts
        // or capture value.
        let error = postpass_tap_closure_not_forwarded(&classes.postpass)
            .expect("a requested postpass tap must fail closed");
        let PreparationVisualCaptureError::Slice(slice) = &error else {
            panic!("postpass rejection must use the named Slice carrier: {error:?}");
        };
        assert_eq!(
            slice.0,
            "preparation visual postpass tap closure is not forwarded: PostPass::GCodeEmit"
        );

        // The same classifier accepts a per-layer-only request and yields no
        // postpass rejection: the gate is the closure class, not tap parsing.
        let per_layer_only = classify_preparation_visual_taps(&["Layer::Perimeters".to_owned()])
            .expect("per-layer tap classifies");
        assert!(postpass_tap_closure_not_forwarded(&per_layer_only.postpass).is_none());

        // Undocumented taps remain the ordinary unknown-tap error, never a
        // postpass or silent-success misclassification.
        assert_eq!(
            classify_preparation_visual_taps(&["Layer::NotAStage".to_owned()]),
            Err(CaptureExecutionError::UnknownTap {
                tap: "Layer::NotAStage".to_owned(),
            })
        );
    }
}
