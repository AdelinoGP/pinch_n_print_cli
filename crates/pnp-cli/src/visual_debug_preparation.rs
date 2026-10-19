#![allow(missing_docs)]

//! Private schema-1.4.0 preparation bundle serializer (packet 06, Step 15).
//!
//! This module is the bundle-side companion to the public request surface in
//! [`crate::visual_debug`]. It serializes a *preparation* manifest: the common
//! legacy manifest fields plus the schema-1.4.0 execution/render evidence axes
//! (`executed_preparation`, `rendered_layer_indices`) and typed per-image
//! preparation entries. It deliberately does not widen
//! [`crate::visual_debug::Manifest`] or [`crate::visual_debug::ImageEntry`];
//! the legacy serializer keeps producing byte-identical output for
//! `1.0.0`-`1.3.0` requests and for legacy entries inside a mixed request.
//!
//! What this module may carry is bounded by ADR-0066: typed public projection
//! geometry (the SDK `PreparationProjectionScene` DTO), owner/version/artifact
//! identities and metadata-only lifecycle evidence. No private plan, store,
//! piece, codec or handle field exists anywhere in this file — the committed
//! scene is reached only through [`slicer_runtime::CapturedIr`]'s
//! `PreparationProjection` variant, whose serializer projects the public DTO.
//!
//! The run-wiring slice (`visual_debug::run_preparation_visual_debug`) consumes
//! the builder/writer/path items and the manifest companions; only the
//! test-facing `PreparationBundleImage::layer_index` keeps a local
//! `#[allow(dead_code)]`.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use slicer_runtime::visual_debug_render::PREPARATION_STROKE_PX;
use slicer_runtime::visual_debug_style::TOOL_PALETTE;
use slicer_runtime::{
    CapturedIr, PreparationCapture, PreparationLifecyclePhase, PreparationLifecycleRecord,
    PreparationLifecycleStatus, PreparationOwnerCapture, PreparationVisualCapture,
    ViewportBoundsMm,
};
use slicer_sdk::preparation::PreparationProjectionScene;

use crate::visual_debug::{
    ImageEntry, LayerExpansionEntry, ManifestSource, PreparationTapSelector, TapSelector,
    ToolPaletteEntry, Viewport, VisualDebugError, PREPARATION_GEOMETRY_KIND, VERSION_1_4,
};

/// The exact `capture_phase` value for every preparation entry.
///
/// Lowercase on purpose: the preparation axis is a lifecycle phase, not a
/// scheduler stage id (`Layer::...` / `PrePass::...` / `PostPass::...`).
const CAPTURE_PHASE_PREPARATION: &str = "preparation";

/// The exact `lifecycle_phase` value in an executed-preparation record.
const LIFECYCLE_PHASE_PREPARATION: &str = "preparation";

/// The exact `status` value in an executed-preparation record.
const LIFECYCLE_STATUS_READY: &str = "ready";

/// The captured-IR kind envelope tag for a preparation projection.
///
/// This mirrors `CapturedIr::PreparationProjection`'s derived
/// `#[serde(tag = "kind", content = "value")]` envelope exactly, so a per-image
/// `typed_capture` is interchangeable with the runtime's own serialization of
/// that variant — only the primitive set is layer-filtered.
const PREPARATION_CAPTURE_KIND: &str = "PreparationProjection";

/// The exact projection schema a preparation scene serializes under.
///
/// Reuses the runtime variant's pinned constant so the DTO schema cannot drift
/// between `CapturedIr::schema_version_string` and the bundle writer. It is a
/// separate axis from the manifest's `schema_version` (`1.4.0`).
const PROJECTION_SCHEMA_VERSION: &str = slicer_runtime::PREPARATION_PROJECTION_SCHEMA_VERSION;

/// The exact `point_glyph` value in a class-legend row.
const POINT_GLYPH_CIRCLE: &str = "circle";

/// One row of a preparation entry's `class_legend`.
///
/// These are deterministic presentation values, derived from the whole scene's
/// sorted class list — never from the rendered layer alone, so a subset render
/// keeps the same colors and glyphs as an all-layer render. `rgb` denotes a
/// diagnostic class rank, not a tool assignment; `stroke_px` is a screen-pixel
/// line width at the bundle's `resolution_scale` (the same value the renderer
/// uses via `PREPARATION_STROKE_PX`), never a physical deposition width.
#[derive(Debug, Serialize)]
pub(crate) struct ClassLegendEntry {
    pub(crate) class: String,
    pub(crate) rgb: [u8; 3],
    pub(crate) point_glyph: String,
    pub(crate) stroke_px: u32,
}

/// One rendered preparation image: the PNG plus the world-space frame it was
/// projected against.
///
/// `capture_index` indexes [`PreparationVisualCapture::captures`], keeping the
/// association explicit instead of re-deriving owner/view/layer from a
/// filename. The renderer's world-to-pixel transform is the projector; these
/// bounds are the shared bundle frame, so every entry rendered against the same
/// frame reports identical values.
#[derive(Debug)]
pub(crate) struct PreparationRenderedImage {
    pub(crate) capture_index: usize,
    pub(crate) png_path: String,
    pub(crate) png_bytes: Vec<u8>,
    pub(crate) world_bounds_mm: ViewportBoundsMm,
}

/// Validated request metadata the bundle writer records verbatim.
///
/// These are the common legacy manifest fields whose values come from the
/// request/source rather than from the preparation capture. Execution evidence
/// (`executed_stage_ids`, `executed_layer_indices`, `rendered_layer_indices`,
/// `scheduled_layer_zs`) and `executed_preparation` are derived from the real
/// [`PreparationVisualCapture`] instead, so they cannot drift from it.
#[derive(Debug)]
pub(crate) struct PreparationBundleMetadata {
    pub(crate) source: ManifestSource,
    pub(crate) resolution_scale: u32,
    pub(crate) viewport: Viewport,
    pub(crate) frame: String,
    pub(crate) legend_version: String,
    pub(crate) ir_schema_version: Option<String>,
    pub(crate) gcode_parser_version: Option<String>,
    pub(crate) warnings: Vec<String>,
    /// Legacy closure expansions, when the request also selected scheduler-stage
    /// taps. Empty for a preparation-only bundle: the preparation companion
    /// reports executed-versus-rendered layers, not legacy stage expansions.
    pub(crate) layer_expansions: Vec<LayerExpansionEntry>,
    /// The legacy per-tool table, when a mixed bundle also tool-colored an
    /// ordinary stage image. Always `None` for a preparation-only bundle: a
    /// preparation view makes no tool/color claim.
    pub(crate) tool_palette: Option<Vec<ToolPaletteEntry>>,
}

/// One preparation image entry (schema 1.4.0).
///
/// `typed_capture` has exactly `kind`/`value`; `value` is the public SDK scene
/// filtered to this entry's `layer_index`, with the scene's full `classes`
/// retained so subset renders keep subset-stable legends. No private plan,
/// store, piece, codec or handle field is present.
#[derive(Debug, Serialize)]
pub(crate) struct PreparationImageEntry {
    pub(crate) source: String,
    pub(crate) tap: String,
    pub(crate) layer_index: i64,
    pub(crate) layer_z: f64,
    pub(crate) visualization: String,
    pub(crate) png_path: String,
    pub(crate) viewport: Viewport,
    pub(crate) legend_version: String,
    /// The captured variant's own IR schema (`"1.0.0"` for a preparation
    /// projection), mirroring the legacy typed-capture entries. The projection
    /// contract is named separately by `projection_schema`.
    pub(crate) ir_schema_version: Option<String>,
    /// Always absent for a model-source preparation capture.
    pub(crate) gcode_parser_version: Option<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) world_bounds_mm: ViewportBoundsMm,
    /// Exact lifecycle phase string `"preparation"`.
    pub(crate) capture_phase: String,
    pub(crate) module_id: String,
    pub(crate) module_version: String,
    pub(crate) artifact_identity: String,
    pub(crate) consumer_stage: String,
    pub(crate) view: String,
    /// Exact projection schema string `"1.0.0"`.
    pub(crate) projection_schema: String,
    /// `true` when this entry mirrors zero primitives for its layer. An
    /// explicitly completed empty scene is valid and still produces a PNG; a
    /// missing scene is an error and never becomes `empty: true`.
    pub(crate) empty: bool,
    pub(crate) class_legend: Vec<ClassLegendEntry>,
    /// `{ "kind": "PreparationProjection", "value": {scene_id, view, classes,
    /// primitives} }` — the public DTO only.
    pub(crate) typed_capture: serde_json::Value,
}

/// One image entry in a schema-1.4.0 bundle's `images` array.
///
/// A mixed request (AC-5) may select a preparation tap together with an
/// ordinary scheduler-stage tap, so one manifest can carry both entry shapes.
/// Serialization is untagged: a legacy entry emits [`ImageEntry`]'s fields
/// byte-for-byte (including its optional per-kind fields, such as `overlay`,
/// `color_by` and `view`), while a preparation entry emits
/// [`PreparationImageEntry`]'s typed fields. The enum is internal only; the
/// manifest's top-level shape and key list are unchanged.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum PreparationBundleImage {
    Legacy(ImageEntry),
    Preparation(PreparationImageEntry),
}

impl PreparationBundleImage {
    /// Wrap an ordinary scheduler-stage entry.
    pub(crate) fn legacy(entry: ImageEntry) -> Self {
        Self::Legacy(entry)
    }

    /// Wrap a typed preparation entry.
    pub(crate) fn preparation(entry: PreparationImageEntry) -> Self {
        Self::Preparation(entry)
    }

    /// The bundle-relative PNG path this entry references.
    pub(crate) fn png_path(&self) -> &str {
        match self {
            Self::Legacy(entry) => &entry.png_path,
            Self::Preparation(entry) => &entry.png_path,
        }
    }

    /// The global layer index this entry renders, when it renders one.
    ///
    /// A schema-1.2.0 silhouette legacy entry is a multi-layer composite and
    /// therefore reports `None`; every preparation entry is single-layer.
    ///
    /// Consumed by the bundle unit test only: production layout validation
    /// checks paths, not per-entry layers, so this stays test-facing.
    #[allow(dead_code)]
    pub(crate) fn layer_index(&self) -> Option<i64> {
        match self {
            Self::Legacy(entry) => entry.layer_index,
            Self::Preparation(entry) => Some(entry.layer_index),
        }
    }
}

impl From<ImageEntry> for PreparationBundleImage {
    fn from(entry: ImageEntry) -> Self {
        Self::Legacy(entry)
    }
}

impl From<PreparationImageEntry> for PreparationBundleImage {
    fn from(entry: PreparationImageEntry) -> Self {
        Self::Preparation(entry)
    }
}

/// One published-piece row in [`ExecutedPreparationEntry::pieces`].
#[derive(Debug, Serialize)]
pub(crate) struct ExecutedPreparationPieceEntry {
    pub(crate) name: String,
    pub(crate) length: u64,
}

/// One actually entered, successful preparation owner (schema 1.4.0).
///
/// Declared-but-inactive owners never appear: this record is derived from the
/// real `Preparation`/`Ready` lifecycle observation, not from the declaration
/// list. `elapsed_ms` is present only when the lifecycle record actually
/// measured a duration (`None` when instrumentation is off).
#[derive(Debug, Serialize)]
pub(crate) struct ExecutedPreparationEntry {
    pub(crate) module_id: String,
    pub(crate) module_version: String,
    pub(crate) artifact_identity: String,
    pub(crate) consumer_stage: String,
    /// Exact lifecycle phase string `"preparation"`.
    pub(crate) lifecycle_phase: String,
    /// Exact status string `"ready"`.
    pub(crate) status: String,
    pub(crate) selected_target_count: u64,
    pub(crate) pieces: Vec<ExecutedPreparationPieceEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) elapsed_ms: Option<u64>,
}

/// The private schema-1.4.0 preparation manifest.
///
/// Field order mirrors [`crate::visual_debug::Manifest`] for the common legacy
/// fields, then appends the two preparation-only top-level axes. Legacy field
/// order/absence in [`crate::visual_debug::Manifest`] itself is untouched.
#[derive(Debug, Serialize)]
pub(crate) struct PreparationManifest {
    pub(crate) schema_version: String,
    pub(crate) source: ManifestSource,
    pub(crate) resolution_scale: u32,
    pub(crate) viewport: Viewport,
    pub(crate) frame: String,
    pub(crate) legend_version: String,
    pub(crate) ir_schema_version: Option<String>,
    pub(crate) gcode_parser_version: Option<String>,
    /// Legacy scheduler-stage entries first (caller order), then typed
    /// preparation entries sorted by `(module_id, view, layer_index)`.
    pub(crate) images: Vec<PreparationBundleImage>,
    pub(crate) warnings: Vec<String>,
    pub(crate) executed_stage_ids: Vec<String>,
    pub(crate) layer_expansions: Vec<LayerExpansionEntry>,
    pub(crate) executed_layer_indices: Vec<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) scheduled_layer_zs: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_palette: Option<Vec<ToolPaletteEntry>>,
    /// One record per actually entered successful owner, sorted by frozen
    /// identity `(module_id, module_version, consumer_stage,
    /// artifact_identity)`. Never filtered to the rendered layers.
    pub(crate) executed_preparation: Vec<ExecutedPreparationEntry>,
    /// Global layer indices selected for rendering, distinct from
    /// `executed_preparation` (execution evidence, not a render list).
    pub(crate) rendered_layer_indices: Vec<i64>,
}

/// The exact `{kind, value}` envelope a preparation `typed_capture` uses.
#[derive(Serialize)]
struct TypedCaptureEnvelope<'a> {
    kind: &'static str,
    value: &'a PreparationProjectionScene,
}

/// Reversible lowercase hex of a UTF-8 string's bytes.
///
/// Filenames must never collide through sanitization: `a/b` and `a:b` both
/// sanitize to `a_b`, but their hex encodings differ. Every byte is encoded, so
/// the identity can be recovered without loss.
fn hex_utf8(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

/// Canonical preparation image path:
/// `images/preparation_{module_hex}_{view_hex}_layer_{index}.png`.
///
/// Both identities are reversible-lowercase-hex encoded UTF-8, and the layer
/// index is appended verbatim, so two different `(module_id, view, layer)`
/// triples can never share a path.
pub(crate) fn preparation_image_path(module_id: &str, view: &str, layer_index: i64) -> String {
    format!(
        "images/preparation_{}_{}_layer_{layer_index}.png",
        hex_utf8(module_id),
        hex_utf8(view)
    )
}

/// Build the deterministic class legend from the WHOLE scene's classes.
///
/// Sorted by class first, then ranked: rank 0 gets `TOOL_PALETTE[0]`, and ranks
/// wrap modulo the palette length. This matches the renderer's
/// full-scene-rank palette assignment, so the manifest mirrors the pixels
/// drawn. `stroke_px` scales the fixed screen-pixel stroke with
/// `resolution_scale`; it is a presentation value, not a physical width.
fn class_legend(classes: &[String], resolution_scale: u32) -> Vec<ClassLegendEntry> {
    let mut sorted: Vec<&str> = classes.iter().map(String::as_str).collect();
    sorted.sort_unstable();
    sorted
        .into_iter()
        .enumerate()
        .map(|(rank, class)| ClassLegendEntry {
            class: class.to_string(),
            rgb: TOOL_PALETTE[rank % TOOL_PALETTE.len()],
            point_glyph: POINT_GLYPH_CIRCLE.to_string(),
            stroke_px: PREPARATION_STROKE_PX * resolution_scale,
        })
        .collect()
}

/// Project the public scene DTO to one rendered layer, keeping full classes.
///
/// Only `primitives` are filtered by `global_layer_index`; `scene_id`, `view`
/// and `classes` stay exact and unfiltered so per-entry legends remain stable
/// across subset renders. The envelope is exactly `kind`/`value`.
fn typed_capture_value(
    scene: &PreparationProjectionScene,
    layer_index: u32,
) -> Result<serde_json::Value, VisualDebugError> {
    let filtered = PreparationProjectionScene {
        scene_id: scene.scene_id.clone(),
        view: scene.view.clone(),
        classes: scene.classes.clone(),
        primitives: scene
            .primitives
            .iter()
            .filter(|primitive| primitive.global_layer_index == layer_index)
            .cloned()
            .collect(),
    };
    serde_json::to_value(TypedCaptureEnvelope {
        kind: PREPARATION_CAPTURE_KIND,
        value: &filtered,
    })
    .map_err(|error| {
        VisualDebugError::CaptureFailed(format!(
            "preparation typed capture serialization failed for scene '{}': {error}",
            scene.scene_id
        ))
    })
}

/// The frozen identity shared by an owner capture and its lifecycle records.
type FrozenIdentity = (String, String, String, String);

fn frozen_identity(
    module_id: &str,
    module_version: &str,
    consumer_stage: &str,
    artifact_identity: &str,
) -> FrozenIdentity {
    (
        module_id.to_string(),
        module_version.to_string(),
        consumer_stage.to_string(),
        artifact_identity.to_string(),
    )
}

/// Build `executed_preparation` from real #05 lifecycle evidence.
///
/// Only `Preparation` + `Ready` records are candidates: a declared or inactive
/// owner never emits one, and failure/cancellation/disposal records are not
/// executions. The map deduplicates by frozen identity and sorts on the same
/// tuple. `selected_target_count` and `pieces` come from the owner capture that
/// the same lifecycle observation reported.
fn executed_preparation(
    metadata: &PreparationCapture,
) -> Result<Vec<ExecutedPreparationEntry>, VisualDebugError> {
    let owners: BTreeMap<FrozenIdentity, &PreparationOwnerCapture> = metadata
        .owners
        .iter()
        .map(|owner| {
            (
                frozen_identity(
                    &owner.module_id,
                    &owner.module_version,
                    &owner.consumer_stage,
                    &owner.artifact_identity,
                ),
                owner,
            )
        })
        .collect();
    let mut ready: BTreeMap<FrozenIdentity, &PreparationLifecycleRecord> = BTreeMap::new();
    for record in &metadata.lifecycle {
        if record.lifecycle_phase != PreparationLifecyclePhase::Preparation
            || record.status != PreparationLifecycleStatus::Ready
        {
            continue;
        }
        ready
            .entry(frozen_identity(
                &record.module_id,
                &record.module_version,
                &record.consumer_stage,
                &record.artifact_identity,
            ))
            .or_insert(record);
    }

    ready
        .into_iter()
        .map(|(identity, record)| {
            let owner = owners.get(&identity).ok_or_else(|| {
                VisualDebugError::CaptureFailed(format!(
                    "preparation lifecycle Ready record for owner '{}' has no owner metadata",
                    identity.0
                ))
            })?;
            Ok(ExecutedPreparationEntry {
                module_id: record.module_id.clone(),
                module_version: record.module_version.clone(),
                artifact_identity: record.artifact_identity.clone(),
                consumer_stage: record.consumer_stage.clone(),
                lifecycle_phase: LIFECYCLE_PHASE_PREPARATION.to_string(),
                status: LIFECYCLE_STATUS_READY.to_string(),
                selected_target_count: owner.selected_targets.len() as u64,
                pieces: record
                    .pieces
                    .iter()
                    .map(|(name, length)| ExecutedPreparationPieceEntry {
                        name: name.clone(),
                        length: *length,
                    })
                    .collect(),
                elapsed_ms: record.elapsed.map(|elapsed| elapsed.as_millis() as u64),
            })
        })
        .collect()
}

/// Build a complete schema-1.4.0 preparation manifest and its PNG payloads.
///
/// The real [`PreparationVisualCapture`] supplies every execution/render fact
/// (`executed_stage_ids`, `executed_layer_indices`, `rendered_layer_indices`,
/// `scheduled_layer_zs`, captures and lifecycle evidence). `metadata` supplies
/// the validated request fields. `rendered` supplies one already-rendered PNG
/// per preparation capture, in the caller's deterministic order.
/// `legacy_images` supplies the ordinary scheduler-stage entries from the same
/// mixed request (AC-5), already built by the legacy path.
///
/// The emitted `images` order is deterministic and independent of payload
/// order: every legacy entry comes first, in the caller's order, followed by
/// the preparation entries sorted by `(module_id, view, layer_index)` — the
/// identity their canonical PNG path encodes. The returned PNG payloads stay
/// in the caller's `rendered` order; layout validation only requires that the
/// two sets match.
///
/// Every referenced capture must be a `CapturedIr::PreparationProjection`
/// (legacy stage captures are not preparation entries and belong to the legacy
/// serializer), and its PNG path must be the canonical
/// [`preparation_image_path`] for its frozen owner/view/layer identity.
pub(crate) fn build_preparation_bundle(
    capture: &PreparationVisualCapture,
    metadata: PreparationBundleMetadata,
    legacy_images: Vec<ImageEntry>,
    rendered: Vec<PreparationRenderedImage>,
) -> Result<(PreparationManifest, Vec<(String, Vec<u8>)>), VisualDebugError> {
    let mut owners: BTreeMap<(&str, &str), &PreparationOwnerCapture> = BTreeMap::new();
    for owner in &capture.metadata().owners {
        let key = (owner.module_id.as_str(), owner.consumer_stage.as_str());
        if owners.insert(key, owner).is_some() {
            // A capture carries only the owner's module id; two entered owners
            // at the same consumer stage would make its module_version /
            // artifact_identity ambiguous. Fail closed instead of attaching one
            // owner's identity to the other's image.
            return Err(VisualDebugError::CaptureFailed(format!(
                "preparation owner identity is ambiguous: more than one entered owner for \
                 module '{}' at stage '{}'",
                owner.module_id, owner.consumer_stage
            )));
        }
    }

    let mut preparation_images = Vec::with_capacity(rendered.len());
    let mut rendered_files = Vec::with_capacity(rendered.len());
    for image in rendered {
        let stage_capture = capture.captures().get(image.capture_index).ok_or_else(|| {
            VisualDebugError::CaptureFailed(format!(
                "preparation image {} references capture index {} of {} captures",
                image.png_path,
                image.capture_index,
                capture.captures().len()
            ))
        })?;
        let CapturedIr::PreparationProjection(projection) = &stage_capture.ir else {
            return Err(VisualDebugError::CaptureFailed(format!(
                "preparation image {} references a non-preparation capture at index {}",
                image.png_path, image.capture_index
            )));
        };
        let scene = projection.scene();
        let module_id = projection.owner_module_id();
        let view = projection.view();
        let layer_index = stage_capture.layer_index;
        let expected_path = preparation_image_path(module_id, view, layer_index as i64);
        if image.png_path != expected_path {
            return Err(VisualDebugError::CaptureFailed(format!(
                "preparation image path '{}' is not the canonical reversible-hex path '{expected_path}' \
                 for owner '{module_id}' view '{view}' layer {layer_index}",
                image.png_path
            )));
        }
        let owner = owners
            .get(&(module_id, stage_capture.stage_id.as_str()))
            .ok_or_else(|| {
                VisualDebugError::CaptureFailed(format!(
                    "preparation capture for owner '{module_id}' stage '{}' has no owner metadata",
                    stage_capture.stage_id
                ))
            })?;
        let typed_capture = typed_capture_value(scene, layer_index)?;
        // "Empty" is per entry: this image mirrors zero primitives for its
        // layer. An explicitly completed empty scene is valid and still gets a
        // PNG; a missing scene never reaches the builder (it is a typed
        // projection rejection, not an empty success).
        let empty = !scene
            .primitives
            .iter()
            .any(|primitive| primitive.global_layer_index == layer_index);

        preparation_images.push(PreparationImageEntry {
            source: metadata.source.kind.clone(),
            tap: crate::visual_debug::tap_name(&TapSelector::Preparation {
                preparation: PreparationTapSelector::new(module_id, view),
            }),
            layer_index: layer_index as i64,
            layer_z: f64::from(stage_capture.layer_z),
            visualization: PREPARATION_GEOMETRY_KIND.to_string(),
            png_path: image.png_path.clone(),
            viewport: metadata.viewport.clone(),
            legend_version: metadata.legend_version.clone(),
            ir_schema_version: Some(stage_capture.ir.schema_version_string()),
            gcode_parser_version: None,
            warnings: Vec::new(),
            world_bounds_mm: image.world_bounds_mm,
            capture_phase: CAPTURE_PHASE_PREPARATION.to_string(),
            module_id: owner.module_id.clone(),
            module_version: owner.module_version.clone(),
            artifact_identity: owner.artifact_identity.clone(),
            consumer_stage: stage_capture.stage_id.clone(),
            view: view.to_string(),
            projection_schema: PROJECTION_SCHEMA_VERSION.to_string(),
            empty,
            class_legend: class_legend(&scene.classes, metadata.resolution_scale),
            typed_capture,
        });
        rendered_files.push((image.png_path, image.png_bytes));
    }

    // Canonical manifest order: legacy stage entries first (caller order),
    // then preparation entries sorted by the frozen (module, view, layer)
    // identity their canonical PNG path encodes. Sorting only the manifest
    // entries keeps payload order independent and validation set-based.
    preparation_images.sort_by(|left, right| {
        (
            left.module_id.as_str(),
            left.view.as_str(),
            left.layer_index,
        )
            .cmp(&(
                right.module_id.as_str(),
                right.view.as_str(),
                right.layer_index,
            ))
    });
    let mut images = Vec::with_capacity(legacy_images.len() + preparation_images.len());
    images.extend(
        legacy_images
            .into_iter()
            .map(PreparationBundleImage::legacy),
    );
    images.extend(
        preparation_images
            .into_iter()
            .map(PreparationBundleImage::preparation),
    );

    let scheduled_layer_zs = if capture.schedule().is_empty() {
        None
    } else {
        Some(
            capture
                .schedule()
                .iter()
                .map(|layer| f64::from(layer.z))
                .collect(),
        )
    };

    let manifest = PreparationManifest {
        schema_version: VERSION_1_4.to_string(),
        source: metadata.source,
        resolution_scale: metadata.resolution_scale,
        viewport: metadata.viewport,
        frame: metadata.frame,
        legend_version: metadata.legend_version,
        ir_schema_version: metadata.ir_schema_version,
        gcode_parser_version: metadata.gcode_parser_version,
        images,
        warnings: metadata.warnings,
        executed_stage_ids: capture.executed_stage_ids().to_vec(),
        layer_expansions: metadata.layer_expansions,
        executed_layer_indices: capture
            .executed_layer_indices()
            .iter()
            .map(|index| *index as i64)
            .collect(),
        scheduled_layer_zs,
        tool_palette: metadata.tool_palette,
        executed_preparation: executed_preparation(capture.metadata())?,
        rendered_layer_indices: capture
            .rendered_layer_indices()
            .iter()
            .map(|index| *index as i64)
            .collect(),
    };
    Ok((manifest, rendered_files))
}

/// Validate the bundle layout before anything on disk is touched.
///
/// Every PNG path must be a normalized relative path (no root, prefix, `.` or
/// `..` component), unique, and referenced exactly once by a manifest entry;
/// the payload count must match the entry count.
fn validate_bundle_layout(
    manifest: &PreparationManifest,
    images: &[(String, Vec<u8>)],
) -> Result<(), VisualDebugError> {
    let mut seen = BTreeSet::new();
    for (relative_path, _) in images {
        let path = Path::new(relative_path);
        // JSON bundle paths use `/` only: a backslash would name a nested path
        // on Windows but a single filename on POSIX, so it is rejected rather
        // than producing a platform-dependent bundle.
        let normalized = !relative_path.is_empty()
            && !relative_path.contains('\\')
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !normalized {
            return Err(VisualDebugError::Write(format!(
                "preparation image path '{relative_path}' must be a normalized relative path"
            )));
        }
        if !seen.insert(relative_path.as_str()) {
            return Err(VisualDebugError::Write(format!(
                "duplicate preparation image path '{relative_path}'"
            )));
        }
    }
    if manifest.images.len() != images.len() {
        return Err(VisualDebugError::Write(format!(
            "preparation manifest lists {} image entries but {} PNG payloads were rendered",
            manifest.images.len(),
            images.len()
        )));
    }
    for entry in &manifest.images {
        let png_path = entry.png_path();
        if !seen.contains(png_path) {
            return Err(VisualDebugError::Write(format!(
                "preparation manifest entry references missing PNG path '{png_path}'"
            )));
        }
    }
    Ok(())
}

/// Write a preparation bundle after every validation has succeeded.
///
/// The fail-closed guard is the same one `run_visual_debug` applies: a
/// non-empty output directory without `--overwrite` is rejected before any
/// mutation. The manifest is serialized and the layout validated *before* the
/// directory is wiped/created, so a rejected bundle can never destroy an
/// existing one. PNGs are written before `manifest.json`, which is written
/// through a synced temp file and renamed last; `manifest.json` is the bundle's
/// only success marker.
///
/// Mid-write I/O failure semantics match `run_visual_debug` exactly: the temp
/// manifest is removed, the already-written PNGs (and, with `--overwrite`, the
/// previously wiped bundle) remain on disk, and no `manifest.json` is produced.
/// The existing writer does not roll back a partial directory, so this helper
/// deliberately does not either.
pub(crate) fn write_preparation_bundle(
    output_dir: &Path,
    overwrite: bool,
    manifest: &PreparationManifest,
    images: &[(String, Vec<u8>)],
) -> Result<PathBuf, VisualDebugError> {
    // Validation first: serialization and layout must both succeed before the
    // output directory is wiped or created.
    let json = serde_json::to_vec_pretty(manifest)
        .map_err(|error| VisualDebugError::Write(error.to_string()))?;
    validate_bundle_layout(manifest, images)?;

    // Non-destructive guard only, identical to `run_visual_debug`'s.
    if output_dir.exists() {
        let mut entries =
            fs::read_dir(output_dir).map_err(|error| VisualDebugError::Write(error.to_string()))?;
        if entries.next().is_some() && !overwrite {
            return Err(VisualDebugError::NonEmptyOutputRequiresOverwrite);
        }
    }

    // Only now mutate the directory: wipe when `--overwrite`'d and non-empty,
    // otherwise create it when absent. Validation above never leaves a partial
    // replacement behind.
    if output_dir.exists() {
        if overwrite {
            for entry in fs::read_dir(output_dir)
                .map_err(|error| VisualDebugError::Write(error.to_string()))?
            {
                let path = entry
                    .map_err(|error| VisualDebugError::Write(error.to_string()))?
                    .path();
                let result = if path.is_dir() {
                    fs::remove_dir_all(path)
                } else {
                    fs::remove_file(path)
                };
                result.map_err(|error| VisualDebugError::Write(error.to_string()))?;
            }
        }
    } else {
        fs::create_dir_all(output_dir)
            .map_err(|error| VisualDebugError::Write(error.to_string()))?;
    }

    for (relative_path, bytes) in images {
        let file_path = output_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| VisualDebugError::Write(error.to_string()))?;
        }
        fs::write(&file_path, bytes).map_err(|error| VisualDebugError::Write(error.to_string()))?;
    }

    let manifest_path = output_dir.join("manifest.json");
    let temp_path = output_dir.join("manifest.json.tmp");
    let result = (|| {
        let mut file = fs::File::create(&temp_path)
            .map_err(|error| VisualDebugError::Write(error.to_string()))?;
        file.write_all(&json)
            .map_err(|error| VisualDebugError::Write(error.to_string()))?;
        file.sync_all()
            .map_err(|error| VisualDebugError::Write(error.to_string()))?;
        fs::rename(&temp_path, &manifest_path)
            .map_err(|error| VisualDebugError::Write(error.to_string()))?;
        Ok(manifest_path.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::Point2;
    use slicer_sdk::preparation::{PreparationProjectionGeometry, PreparationProjectionPrimitive};
    use std::time::Duration;

    fn primitive(id: &str, layer: u32, class: &str) -> PreparationProjectionPrimitive {
        // exhaustive: this site intentionally pins every SDK DTO field
        PreparationProjectionPrimitive {
            id: id.to_string(),
            global_layer_index: layer,
            class: class.to_string(),
            label: None,
            geometry: PreparationProjectionGeometry::Point(Point2 { x: 1, y: 2 }),
        }
    }

    #[test]
    fn typed_capture_mirrors_only_the_entry_layer_and_keeps_full_classes() {
        let scene = PreparationProjectionScene {
            scene_id: "final".to_string(),
            view: "probe_xy".to_string(),
            classes: vec!["paths".to_string(), "anchors".to_string()],
            primitives: vec![
                primitive("anchor_a", 0, "anchors"),
                primitive("path_a", 1, "paths"),
            ],
        };
        let value = typed_capture_value(&scene, 0).expect("public DTO serializes");

        let keys: BTreeSet<&str> = value
            .as_object()
            .expect("typed capture is an object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, ["kind", "value"].into_iter().collect::<BTreeSet<_>>());
        assert_eq!(value["kind"], "PreparationProjection");

        let inner = value["value"].as_object().expect("value is an object");
        let inner_keys: BTreeSet<&str> = inner.keys().map(String::as_str).collect();
        assert_eq!(
            inner_keys,
            ["scene_id", "view", "classes", "primitives"]
                .into_iter()
                .collect::<BTreeSet<_>>()
        );
        // Full classes are retained; only the primitive set is layer-filtered.
        assert_eq!(inner["classes"], serde_json::json!(["paths", "anchors"]));
        assert_eq!(inner["primitives"].as_array().map(Vec::len), Some(1));
        assert_eq!(inner["primitives"][0]["id"], "anchor_a");
        assert_eq!(inner["primitives"][0]["global_layer_index"], 0);
    }

    #[test]
    fn preparation_image_path_is_reversible_hex_and_cannot_collide() {
        // `a/b` and `a:b` both sanitize to `a_b`; hex keeps them distinct.
        let slash = preparation_image_path("a/b", "v", 3);
        let colon = preparation_image_path("a:b", "v", 3);
        assert_ne!(slash, colon);
        assert_eq!(
            slash, "images/preparation_612f62_76_layer_3.png",
            "every UTF-8 byte is encoded as lowercase hex"
        );
        // A different layer never shares the path.
        assert_ne!(slash, preparation_image_path("a/b", "v", 2));
    }

    #[test]
    fn class_legend_ranks_the_whole_scene_sorted_and_scales_screen_stroke() {
        let legend = class_legend(&["z_paths".to_string(), "a_anchors".to_string()], 2);
        assert_eq!(
            legend
                .iter()
                .map(|row| row.class.as_str())
                .collect::<Vec<_>>(),
            ["a_anchors", "z_paths"]
        );
        assert_eq!(legend[0].rgb, TOOL_PALETTE[0]);
        assert_eq!(legend[1].rgb, TOOL_PALETTE[1]);
        assert!(legend.iter().all(|row| row.point_glyph == "circle"));
        // Screen-pixel stroke at scale 2, never a physical width.
        assert!(legend
            .iter()
            .all(|row| row.stroke_px == PREPARATION_STROKE_PX * 2));
    }

    fn owner(
        module_id: &str,
        version: &str,
        stage: &str,
        artifact: &str,
    ) -> PreparationOwnerCapture {
        // exhaustive: this test pins the frozen identity and empty selection
        PreparationOwnerCapture {
            module_id: module_id.to_string(),
            module_version: version.to_string(),
            artifact_identity: artifact.to_string(),
            consumer_stage: stage.to_string(),
            selected_targets: Vec::new(),
            pieces: Vec::new(),
            accounting_before_disposal: slicer_runtime::PreparationAccountingSnapshot {
                retained_payload_bytes: 0,
                retained_name_bytes: 0,
                returned_read_bytes: 0,
            },
        }
    }

    fn record(
        module_id: &str,
        version: &str,
        stage: &str,
        artifact: &str,
        phase: PreparationLifecyclePhase,
        status: PreparationLifecycleStatus,
        elapsed: Option<Duration>,
        pieces: Vec<(String, u64)>,
    ) -> PreparationLifecycleRecord {
        // exhaustive: this test pins every lifecycle axis it asserts
        PreparationLifecycleRecord {
            module_id: module_id.to_string(),
            module_version: version.to_string(),
            artifact_identity: artifact.to_string(),
            consumer_stage: stage.to_string(),
            global_layer_index: None,
            lifecycle_phase: phase,
            status,
            pieces,
            retained_bytes: 0,
            transferred_bytes: 0,
            elapsed,
            failure: None,
        }
    }

    #[test]
    fn executed_preparation_keeps_only_ready_owners_with_conditional_elapsed() {
        let metadata = PreparationCapture {
            owners: vec![
                owner("z.owner", "0.2.0", "Layer::PathOptimization", "art-z"),
                owner("a.owner", "0.1.0", "Layer::PathOptimization", "art-a"),
                owner("inactive.owner", "0.1.0", "Layer::Perimeters", "art-i"),
            ],
            lifecycle: vec![
                record(
                    "z.owner",
                    "0.2.0",
                    "Layer::PathOptimization",
                    "art-z",
                    PreparationLifecyclePhase::Preparation,
                    PreparationLifecycleStatus::Start,
                    None,
                    Vec::new(),
                ),
                record(
                    "z.owner",
                    "0.2.0",
                    "Layer::PathOptimization",
                    "art-z",
                    PreparationLifecyclePhase::Preparation,
                    PreparationLifecycleStatus::Ready,
                    None,
                    vec![("probe".to_string(), 4)],
                ),
                record(
                    "a.owner",
                    "0.1.0",
                    "Layer::PathOptimization",
                    "art-a",
                    PreparationLifecyclePhase::Preparation,
                    PreparationLifecycleStatus::Ready,
                    Some(Duration::from_millis(7)),
                    Vec::new(),
                ),
                record(
                    "inactive.owner",
                    "0.1.0",
                    "Layer::Perimeters",
                    "art-i",
                    PreparationLifecyclePhase::Preparation,
                    PreparationLifecycleStatus::NoWork,
                    None,
                    Vec::new(),
                ),
                record(
                    "z.owner",
                    "0.2.0",
                    "Layer::PathOptimization",
                    "art-z",
                    PreparationLifecyclePhase::Disposal,
                    PreparationLifecycleStatus::Disposed,
                    None,
                    Vec::new(),
                ),
            ],
            ..PreparationCapture::default()
        };

        let entries = executed_preparation(&metadata).expect("owners exist for every Ready record");
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.module_id.as_str())
                .collect::<Vec<_>>(),
            ["a.owner", "z.owner"],
            "only actually entered successful owners, sorted by frozen identity"
        );
        assert_eq!(entries[0].lifecycle_phase, "preparation");
        assert_eq!(entries[0].status, "ready");
        assert_eq!(entries[0].selected_target_count, 0);
        assert_eq!(entries[0].elapsed_ms, Some(7));
        assert_eq!(entries[1].elapsed_ms, None, "no measurement means no field");
        assert_eq!(entries[1].pieces.len(), 1);
        assert_eq!(entries[1].pieces[0].name, "probe");
        assert_eq!(entries[1].pieces[0].length, 4);

        let json = serde_json::to_value(&entries).expect("entries serialize");
        assert!(json[0].get("elapsed_ms").is_some());
        assert!(json[1].get("elapsed_ms").is_none());
    }

    #[test]
    fn mixed_manifest_serializes_legacy_then_preparation_entries_with_unchanged_top_level_keys() {
        // exhaustive: pins every legacy field, including per-kind optionals
        let legacy = ImageEntry {
            source: "model".to_string(),
            tap: "stage:Layer::Perimeters".to_string(),
            layer_index: Some(3),
            layer_z: Some(Some(0.6)),
            visualization: "filament_lines".to_string(),
            png_path: "images/legacy.png".to_string(),
            viewport: Viewport {
                width: 640,
                height: 480,
            },
            legend_version: "1".to_string(),
            ir_schema_version: Some("2.0.0".to_string()),
            gcode_parser_version: None,
            warnings: Vec::new(),
            typed_capture: None,
            world_bounds_mm: None,
            overlay: Some("travel".to_string()),
            overlay_events: Some(Vec::new()),
            color_by: Some("tool".to_string()),
            tool_color_source: Some("palette".to_string()),
            view: None,
            layers_rendered: None,
            composited_overlays: None,
        };
        let preparation = PreparationImageEntry {
            source: "model".to_string(),
            tap: crate::visual_debug::tap_name(&TapSelector::Preparation {
                preparation: PreparationTapSelector::new("probe.owner", "probe_xy"),
            }),
            layer_index: 0,
            layer_z: 0.2,
            visualization: PREPARATION_GEOMETRY_KIND.to_string(),
            png_path: preparation_image_path("probe.owner", "probe_xy", 0),
            viewport: Viewport {
                width: 640,
                height: 480,
            },
            legend_version: "1".to_string(),
            ir_schema_version: Some("1.0.0".to_string()),
            gcode_parser_version: None,
            warnings: Vec::new(),
            world_bounds_mm: ViewportBoundsMm {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 1.0,
                max_y: 1.0,
            },
            capture_phase: CAPTURE_PHASE_PREPARATION.to_string(),
            module_id: "probe.owner".to_string(),
            module_version: "0.1.0".to_string(),
            artifact_identity: "art-probe".to_string(),
            consumer_stage: "Layer::PathOptimization".to_string(),
            view: "probe_xy".to_string(),
            projection_schema: PROJECTION_SCHEMA_VERSION.to_string(),
            empty: true,
            class_legend: Vec::new(),
            typed_capture: serde_json::json!({
                "kind": PREPARATION_CAPTURE_KIND,
                "value": {
                    "scene_id": "final",
                    "view": "probe_xy",
                    "classes": [],
                    "primitives": []
                }
            }),
        };
        let preparation_png_path = preparation.png_path.clone();

        let manifest = PreparationManifest {
            schema_version: VERSION_1_4.to_string(),
            source: ManifestSource {
                kind: "model".to_string(),
                model: None,
                path: None,
            },
            resolution_scale: 1,
            viewport: Viewport {
                width: 640,
                height: 480,
            },
            frame: "model".to_string(),
            legend_version: "1".to_string(),
            ir_schema_version: Some("2.0.0".to_string()),
            gcode_parser_version: None,
            images: vec![PreparationBundleImage::legacy(legacy), preparation.into()],
            warnings: Vec::new(),
            executed_stage_ids: vec!["Layer::Perimeters".to_string()],
            layer_expansions: Vec::new(),
            executed_layer_indices: vec![3],
            scheduled_layer_zs: None,
            tool_palette: None,
            executed_preparation: Vec::new(),
            rendered_layer_indices: vec![0],
        };

        let json = serde_json::to_value(&manifest).expect("mixed manifest serializes");
        let keys: BTreeSet<&str> = json
            .as_object()
            .expect("manifest is an object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "schema_version",
                "source",
                "resolution_scale",
                "viewport",
                "frame",
                "legend_version",
                "ir_schema_version",
                "gcode_parser_version",
                "images",
                "warnings",
                "executed_stage_ids",
                "layer_expansions",
                "executed_layer_indices",
                "executed_preparation",
                "rendered_layer_indices",
            ]
            .into_iter()
            .collect::<BTreeSet<_>>(),
            "the 1.4.0 top-level key list is unchanged; absent optionals stay omitted"
        );

        let images = json["images"].as_array().expect("images is an array");
        assert_eq!(images.len(), 2);
        // Legacy first, with every optional per-kind field intact.
        assert_eq!(images[0]["tap"], "stage:Layer::Perimeters");
        assert_eq!(images[0]["visualization"], "filament_lines");
        assert_eq!(images[0]["layer_index"], 3);
        assert_eq!(images[0]["layer_z"], 0.6);
        assert_eq!(images[0]["overlay"], "travel");
        assert_eq!(images[0]["overlay_events"], serde_json::json!([]));
        assert_eq!(images[0]["color_by"], "tool");
        assert_eq!(images[0]["tool_color_source"], "palette");
        assert!(images[0].get("capture_phase").is_none());
        // Then the preparation entry, with its typed capture.
        assert_eq!(images[1]["capture_phase"], "preparation");
        assert_eq!(images[1]["typed_capture"]["kind"], "PreparationProjection");
        assert_eq!(images[1]["module_id"], "probe.owner");
        assert_eq!(images[1]["projection_schema"], "1.0.0");
        assert!(images[1].get("overlay").is_none());

        assert_eq!(manifest.images[0].png_path(), "images/legacy.png");
        assert_eq!(manifest.images[1].png_path(), preparation_png_path);
        assert_eq!(manifest.images[0].layer_index(), Some(3));
        assert_eq!(manifest.images[1].layer_index(), Some(0));
    }
}
