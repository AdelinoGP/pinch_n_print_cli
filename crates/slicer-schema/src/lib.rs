//! Single source of truth for the Pinch 'n Print stage ↔ WIT-world ↔
//! export mapping.
//!
//! Both `slicer-macros` (which expands `#[slicer_module]`) and
//! `slicer-cli` (which scaffolds new module crates in `cmd_new`)
//! consume this table. Keeping one array here means the macro's
//! emitted binding schema and the CLI's generated manifests cannot
//! drift apart at the (trait, stage, world, export) level
//! (docs/03 §host-boundary enforcement; docs/05 §module SDK).

#![warn(missing_docs)]

// ── WIT world package names ────────────────────────────────────────────
//
// These are the sole source of the world identifiers used across the
// workspace. Refer to them by constant; never re-spell the literal.
//
// The `@x.y.z` version is deliberately absent. It lives in exactly one
// place — the `package` line of `crates/slicer-schema/wit/deps/world-*/`
// — because that is the only place it has any effect: it selects which
// package `bindgen!`/`generate!` resolve at build time.
//
// The version is NOT part of module identity, and cannot be. Our worlds
// export bare freestanding funcs, and a bare extern name carries no
// semver suffix (component-model WIT.md: `<semversuffix>` is a production
// of `<interfacename>`, not of a plain name). The version is therefore
// erased from every guest binary at compile time — `wasm-tools component
// wit <guest>.wasm` finds no `world-layer` and no `@x.y.z` anywhere. A
// versioned identifier here would be an unfalsifiable claim: nothing in
// the system could ever check it against the artifact it describes.
//
// Compatibility is enforced structurally by wasmtime at typed
// instantiation, plus `cargo xtask build-guests --check`.

/// WIT world package name for layer-tier modules.
pub const WORLD_LAYER: &str = "slicer:world-layer";
/// WIT world package name for prepass-tier modules.
pub const WORLD_PREPASS: &str = "slicer:world-prepass";
/// WIT world package name for finalization-tier modules.
pub const WORLD_FINALIZATION: &str = "slicer:world-finalization";
/// WIT world package name for postpass-tier modules.
pub const WORLD_POSTPASS: &str = "slicer:world-postpass";

/// One supported (Rust trait, stage id, WIT export, WIT world) row,
/// matching the documented stage set in docs/04 STAGE_ORDER and the
/// export lists in `wit/world-*.wit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageSpec {
    /// Rust trait method name, e.g. `"run_infill"`.
    pub method: &'static str,
    /// Canonical scheduler stage id, e.g. `"Layer::Infill"`.
    pub stage_id: &'static str,
    /// Kebab-case WIT export name, e.g. `"run-infill"`.
    pub wit_export: &'static str,
    /// Canonical WIT world package id the export belongs to.
    pub world_id: &'static str,
    /// SDK trait carrying this method.
    pub trait_name: &'static str,
}

/// Every supported stage, in canonical STAGE_ORDER-compatible order
/// (docs/04). One row per documented export in `wit/world-*.wit`.
pub const STAGES: &[StageSpec] = &[
    // ── Layer world (WORLD_LAYER) ──────────────────────────────────────
    StageSpec {
        method: "run_slice_postprocess",
        stage_id: "Layer::SlicePostProcess",
        wit_export: "run-slice-postprocess",
        world_id: WORLD_LAYER,
        trait_name: "LayerModule",
    },
    StageSpec {
        method: "run_perimeters",
        stage_id: "Layer::Perimeters",
        wit_export: "run-perimeters",
        world_id: WORLD_LAYER,
        trait_name: "LayerModule",
    },
    StageSpec {
        method: "run_wall_postprocess",
        stage_id: "Layer::PerimetersPostProcess",
        wit_export: "run-wall-postprocess",
        world_id: WORLD_LAYER,
        trait_name: "LayerModule",
    },
    StageSpec {
        method: "run_infill",
        stage_id: "Layer::Infill",
        wit_export: "run-infill",
        world_id: WORLD_LAYER,
        trait_name: "LayerModule",
    },
    StageSpec {
        method: "run_infill_postprocess",
        stage_id: "Layer::InfillPostProcess",
        wit_export: "run-infill-postprocess",
        world_id: WORLD_LAYER,
        trait_name: "LayerModule",
    },
    StageSpec {
        method: "run_support",
        stage_id: "Layer::Support",
        wit_export: "run-support",
        world_id: WORLD_LAYER,
        trait_name: "LayerModule",
    },
    StageSpec {
        method: "run_support_postprocess",
        stage_id: "Layer::SupportPostProcess",
        wit_export: "run-support-postprocess",
        world_id: WORLD_LAYER,
        trait_name: "LayerModule",
    },
    StageSpec {
        method: "run_path_optimization",
        stage_id: "Layer::PathOptimization",
        wit_export: "run-path-optimization",
        world_id: WORLD_LAYER,
        trait_name: "LayerModule",
    },
    // ── Prepass world (WORLD_PREPASS) ──────────────────────────────────
    StageSpec {
        method: "run_mesh_analysis",
        stage_id: "PrePass::MeshAnalysis",
        wit_export: "run-mesh-analysis",
        world_id: WORLD_PREPASS,
        trait_name: "PrepassModule",
    },
    StageSpec {
        method: "run_layer_planning",
        stage_id: "PrePass::LayerPlanning",
        wit_export: "run-layer-planning",
        world_id: WORLD_PREPASS,
        trait_name: "PrepassModule",
    },
    StageSpec {
        method: "run_paint_segmentation",
        stage_id: "PrePass::PaintSegmentation",
        wit_export: "run-paint-segmentation",
        world_id: WORLD_PREPASS,
        trait_name: "PrepassModule",
    },
    StageSpec {
        method: "run_seam_planning",
        stage_id: "PrePass::SeamPlanning",
        wit_export: "run-seam-planning",
        world_id: WORLD_PREPASS,
        trait_name: "PrepassModule",
    },
    StageSpec {
        method: "run_support_geometry",
        stage_id: "PrePass::SupportGeometry",
        wit_export: "run-support-geometry",
        world_id: WORLD_PREPASS,
        trait_name: "PrepassModule",
    },
    // ── Finalization world (WORLD_FINALIZATION) ────────────────────────
    StageSpec {
        method: "run_finalization",
        stage_id: "PostPass::LayerFinalization",
        wit_export: "run-finalization",
        world_id: WORLD_FINALIZATION,
        trait_name: "FinalizationModule",
    },
    // ── Postpass world (WORLD_POSTPASS) ────────────────────────────────
    StageSpec {
        method: "run_gcode_postprocess",
        stage_id: "PostPass::GCodePostProcess",
        wit_export: "run-gcode-postprocess",
        world_id: WORLD_POSTPASS,
        trait_name: "PostpassModule",
    },
    StageSpec {
        method: "run_text_postprocess",
        stage_id: "PostPass::TextPostProcess",
        wit_export: "run-text-postprocess",
        world_id: WORLD_POSTPASS,
        trait_name: "PostpassModule",
    },
];

/// Kind of a single WIT export carried by a module's binding surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    /// One of the world's unconditional lifecycle exports
    /// (`on-print-start`, `on-print-end`).
    Lifecycle,
    /// The stage-specific export detected in the impl (e.g. `run-infill`).
    Stage,
}

/// One WIT export entry in a module's binding schema: the kebab-case
/// export name the guest provides plus whether it is a lifecycle or
/// stage export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportBinding {
    /// Kebab-case WIT export name, e.g. `"on-print-start"`, `"run-infill"`.
    pub name: &'static str,
    /// Classification of this export.
    pub kind: ExportKind,
}

/// Complete compile-time binding-schema surface emitted by
/// `#[slicer_module]` for a single module type (docs/05 §Module Entry
/// Point). Consumed by host plan/build tooling and by module test
/// harnesses for typed reflection over the module's WIT contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlicerModuleSchema {
    /// Rust type name as written at the impl site, e.g. `"MyInfillModule"`.
    pub type_name: &'static str,
    /// SDK trait name this impl targets, or `""` if inherent.
    pub trait_name: &'static str,
    /// Canonical WIT world package name (e.g. [`WORLD_LAYER`])
    /// or `""` if the impl targets no known trait or stage.
    pub world_id: &'static str,
    /// Canonical scheduler stage id (e.g. `"Layer::Infill"`) or `""` if no
    /// stage method was detected (lifecycle-only impls).
    pub stage_id: &'static str,
    /// Rust-cased stage method name (e.g. `"run_infill"`) or `""`.
    pub stage_method: &'static str,
    /// Kebab-case stage export name (e.g. `"run-infill"`) or `""`.
    pub stage_export: &'static str,
    /// Complete ordered export surface: world lifecycle exports (in
    /// lifecycle order) followed by the detected stage export if any.
    pub exports: &'static [ExportBinding],
}

/// WIT worlds and the unconditional lifecycle exports every world ships
/// (`on-print-start`, `on-print-end` per docs/03 §deps/config-types).
pub const WORLD_LIFECYCLE_EXPORTS: &[(&str, &[&str])] = &[
    (WORLD_LAYER, &["on-print-start", "on-print-end"]),
    (WORLD_PREPASS, &["on-print-start", "on-print-end"]),
    (WORLD_FINALIZATION, &["on-print-start", "on-print-end"]),
    (WORLD_POSTPASS, &["on-print-start", "on-print-end"]),
];

// region: region-split priorities

/// Core region-split semantic priorities. Each entry is `(semantic_name,
/// priority)`. Priority defines the canonical `variant_chain` order
/// (BTreeMap-sorted by `(priority, name)`). Core semantics are NOT
/// user-overridable; a module manifest declaring a core semantic with a
/// different priority is rejected with `LoadErrorKind::CorePriorityMismatch`
/// at manifest-load time. See packet 92.
pub const CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)] = &[("material", 100), ("fuzzy_skin", 200)];

/// Minimum priority for a community-defined region-split semantic (any
/// semantic name NOT in `CORE_REGION_SPLIT_PRIORITIES`). Below-floor
/// declarations are rejected with `LoadErrorKind::CommunityPriorityBelowFloor`.
/// The floor is a contract guard against priority squatting; changes require
/// a code edit, not a config override. See packet 92.
pub const COMMUNITY_PRIORITY_FLOOR: u32 = 1000;

// endregion: region-split priorities

/// Look up a [`StageSpec`] by its canonical scheduler stage id, e.g.
/// `"Layer::Infill"`.
#[must_use]
pub fn stage_by_id(stage_id: &str) -> Option<&'static StageSpec> {
    STAGES.iter().find(|s| s.stage_id == stage_id)
}

/// Look up a [`StageSpec`] by its Rust trait method name, e.g.
/// `"run_infill"`.
#[must_use]
pub fn stage_by_method(method: &str) -> Option<&'static StageSpec> {
    STAGES.iter().find(|s| s.method == method)
}

/// Return the WIT world id for a stage id.
#[must_use]
pub fn world_for_stage_id(stage_id: &str) -> Option<&'static str> {
    stage_by_id(stage_id).map(|s| s.world_id)
}

/// Return the SDK trait that carries `stage_id`.
#[must_use]
pub fn trait_for_stage_id(stage_id: &str) -> Option<&'static str> {
    stage_by_id(stage_id).map(|s| s.trait_name)
}

/// Map an SDK trait name (e.g. `"LayerModule"`) to its WIT world id, if
/// the trait is one of the known four.
#[must_use]
pub fn world_for_trait(trait_name: &str) -> Option<&'static str> {
    match trait_name {
        "LayerModule" => Some(WORLD_LAYER),
        "PrepassModule" => Some(WORLD_PREPASS),
        "FinalizationModule" => Some(WORLD_FINALIZATION),
        "PostpassModule" => Some(WORLD_POSTPASS),
        _ => None,
    }
}

/// Return the lifecycle exports for a given WIT world id.
#[must_use]
pub fn lifecycle_exports_for_world(world_id: &str) -> &'static [&'static str] {
    WORLD_LIFECYCLE_EXPORTS
        .iter()
        .find(|(w, _)| *w == world_id)
        .map(|(_, e)| *e)
        .unwrap_or(&[])
}

/// Return the full list of canonical stage ids, in table order.
#[must_use]
pub fn all_stage_ids() -> Vec<&'static str> {
    STAGES.iter().map(|s| s.stage_id).collect()
}

/// Look up the WIT export name for a stage id from the single source of truth in [`STAGES`].
///
/// Returns `None` for unknown stage ids. Dispatcher impls MUST use this lookup; they MUST NOT
/// hardcode their own stage-id → wit-export table (see ADR-0005, planned at P83 close).
#[must_use]
pub fn export_for_stage_id(stage_id: &str) -> Option<&'static str> {
    STAGES
        .iter()
        .find(|s| s.stage_id == stage_id)
        .map(|s| s.wit_export)
}

// ── Validator constants ────────────────────────────────────────────────────
//
// Single source of truth for the sets consumed by manifest validation
// (`cmd_validate` in `slicer-cli` and future `pnp-cli`). Derived from the
// canonical tables above where possible; maintained here to avoid drift.

/// All valid pipeline stage ids a module manifest may declare.
///
/// Mirrors the `stage_id` column of [`STAGES`] in canonical order.
/// See docs/04 STAGE_ORDER.
pub const VALID_STAGES: &[&str] = &[
    "PrePass::MeshAnalysis",
    "PrePass::LayerPlanning",
    "PrePass::PaintSegmentation",
    "PrePass::SeamPlanning",
    "PrePass::SupportGeometry",
    "Layer::SlicePostProcess",
    "Layer::Perimeters",
    "Layer::PerimetersPostProcess",
    "Layer::Infill",
    "Layer::InfillPostProcess",
    "Layer::Support",
    "Layer::SupportPostProcess",
    "Layer::PathOptimization",
    "PostPass::LayerFinalization",
    "PostPass::GCodePostProcess",
    "PostPass::TextPostProcess",
];

/// All WIT world package strings supported by the current SDK.
///
/// Mirrors the world column of [`WORLD_LIFECYCLE_EXPORTS`].
/// See docs/03 §host-boundary enforcement.
pub const SUPPORTED_WIT_WORLDS: &[&str] = &[
    WORLD_LAYER,
    WORLD_PREPASS,
    WORLD_FINALIZATION,
    WORLD_POSTPASS,
];

/// Valid config field type strings for `[config.schema.<key>].type`.
///
/// See docs/03 §deps/config-types.
pub const VALID_CONFIG_TYPES: &[&str] = &[
    "bool",
    "int",
    "float",
    "string",
    "enum",
    "float-list",
    "string-list",
    "percent",
    "float_or_percent",
];

/// Recognized claim names for `[claims].holds` and `[claims].requires`.
///
/// See docs/01 §claim system.
pub const RECOGNIZED_CLAIMS: &[&str] = &[
    "perimeter-generator",
    // `infill-generator` retired 2026-06-09 (DEV-065) in favour of packet 37's
    // four granular fill-role claims (`claim:{top,bottom,bridge,sparse}-fill`);
    // those live alongside `claim:ironing` and are not in this allow-list
    // because the `claim:` prefix is reserved for namespaced per-role claims.
    "support-generator",
    "seam-placer",
    "layer-planner",
    "mesh-analyzer",
    "slice-postprocessor",
    "gcode-postprocessor",
    "text-postprocessor",
];

/// Recognized severity values for `[[config.cross-validate]]` rules.
pub const VALID_SEVERITIES: &[&str] = &["error", "warning"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_table_has_one_entry_per_routed_export() {
        // Matches the total stage exports the host dispatcher and macro
        // route end-to-end. Currently:
        //   Layer world: 8 (slice-postprocess, perimeters, wall-postprocess,
        //                   infill, infill-postprocess, support,
        //                   support-postprocess, path-optimization)
        //   Prepass world: 5 (mesh-analysis, layer-planning,
        //                     paint-segmentation, seam-planning, support-generation)
        //   Finalization world: 1
        //   Postpass world: 2
        assert_eq!(STAGES.len(), 16);
    }

    #[test]
    fn stage_and_world_lookups_are_consistent() {
        for s in STAGES {
            assert_eq!(stage_by_id(s.stage_id).unwrap(), s);
            assert_eq!(stage_by_method(s.method).unwrap(), s);
            assert_eq!(world_for_stage_id(s.stage_id), Some(s.world_id));
            assert_eq!(world_for_trait(s.trait_name), Some(s.world_id));
        }
    }

    #[test]
    fn every_world_has_lifecycle_exports() {
        for s in STAGES {
            let exports = lifecycle_exports_for_world(s.world_id);
            assert!(exports.contains(&"on-print-start"));
            assert!(exports.contains(&"on-print-end"));
        }
    }
}
