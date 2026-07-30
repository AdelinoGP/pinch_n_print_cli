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

// Tier ids. A tier is **vocabulary**: a package-name prefix and an SDK trait
// grouping. It is NOT a loadable WIT package — since packet 164 every stage
// owns its own versioned package (`slicer:<tier>-<stage>@<ver>`), and no
// `slicer:world-*` package exists on disk. These ids name the grouping only.

/// Tier id for layer-tier modules (vocabulary, not a WIT package).
pub const TIER_LAYER: &str = "layer";
/// Tier id for prepass-tier modules (vocabulary, not a WIT package).
pub const TIER_PREPASS: &str = "prepass";
/// Tier id for finalization-tier modules (vocabulary, not a WIT package).
pub const TIER_FINALIZATION: &str = "finalization";
/// Tier id for postpass-tier modules (vocabulary, not a WIT package).
pub const TIER_POSTPASS: &str = "postpass";

/// One supported (Rust trait, stage id, WIT export, tier) row, matching the
/// documented stage set in docs/04 STAGE_ORDER and the per-stage packages
/// under `crates/slicer-schema/wit/deps/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageSpec {
    /// Rust trait method name, e.g. `"run_infill"`.
    pub method: &'static str,
    /// Canonical scheduler stage id, e.g. `"Layer::Infill"`.
    pub stage_id: &'static str,
    /// WIT export name. `"run"` for every stage on a per-stage versioned
    /// package (all 15 since packet 164); `""` for host-built-in stages.
    /// Not unique on its own — use [`qualified_export_for_stage_id`].
    pub wit_export: &'static str,
    /// Tier this stage belongs to (e.g. [`TIER_POSTPASS`]). Vocabulary: a
    /// package-name prefix and SDK trait grouping, **not** a loadable WIT
    /// package (packet 164).
    pub tier_id: &'static str,
    /// SDK trait carrying this method.
    pub trait_name: &'static str,
    /// Per-stage package directory under `crates/slicer-schema/wit/deps/`.
    /// For host-built-in stages (currently `PrePass::PaintSegmentation` only)
    /// this is `""`.
    pub wit_dir: &'static str,
    /// Per-stage WIT package identifier, e.g.
    /// `"slicer:postpass-gcode-postprocess@1.0.0"`. `""` for stages that
    /// have not been migrated to a per-stage package yet.
    pub wit_package: &'static str,
    /// Per-stage WIT exported-interface name, e.g. `"gcode-postprocess"`.
    /// `""` for unmigrated stages.
    pub wit_interface: &'static str,
    /// Per-stage WIT world name, e.g. `"gcode-postprocess-module"`.
    /// `""` for unmigrated stages.
    pub wit_world: &'static str,
}

/// Every supported stage, in canonical STAGE_ORDER-compatible order
/// (docs/04). One row per stage; 15 carry a per-stage WIT package.
pub const STAGES: &[StageSpec] = &[
    // ── Layer tier (TIER_LAYER) ───────────────────────────────────────
    StageSpec {
        method: "run_slice_postprocess",
        stage_id: "Layer::SlicePostProcess",
        wit_export: "run",
        tier_id: TIER_LAYER,
        trait_name: "LayerModule",
        wit_dir: "layer-slice-postprocess",
        wit_package: "slicer:layer-slice-postprocess@1.0.0",
        wit_interface: "slice-postprocess",
        wit_world: "slice-postprocess-module",
    },
    StageSpec {
        method: "run_perimeters",
        stage_id: "Layer::Perimeters",
        wit_export: "run",
        tier_id: TIER_LAYER,
        trait_name: "LayerModule",
        wit_dir: "layer-perimeters",
        wit_package: "slicer:layer-perimeters@1.0.0",
        wit_interface: "perimeters",
        wit_world: "perimeters-module",
    },
    StageSpec {
        method: "run_wall_postprocess",
        stage_id: "Layer::PerimetersPostProcess",
        wit_export: "run",
        tier_id: TIER_LAYER,
        trait_name: "LayerModule",
        wit_dir: "layer-perimeters-postprocess",
        wit_package: "slicer:layer-perimeters-postprocess@1.0.0",
        wit_interface: "perimeters-postprocess",
        wit_world: "perimeters-postprocess-module",
    },
    StageSpec {
        method: "run_infill",
        stage_id: "Layer::Infill",
        wit_export: "run",
        tier_id: TIER_LAYER,
        trait_name: "LayerModule",
        wit_dir: "layer-infill",
        wit_package: "slicer:layer-infill@1.0.0",
        wit_interface: "infill",
        wit_world: "infill-module",
    },
    StageSpec {
        method: "run_infill_postprocess",
        stage_id: "Layer::InfillPostProcess",
        wit_export: "run",
        tier_id: TIER_LAYER,
        trait_name: "LayerModule",
        wit_dir: "layer-infill-postprocess",
        wit_package: "slicer:layer-infill-postprocess@1.0.0",
        wit_interface: "infill-postprocess",
        wit_world: "infill-postprocess-module",
    },
    StageSpec {
        method: "run_support",
        stage_id: "Layer::Support",
        wit_export: "run",
        tier_id: TIER_LAYER,
        trait_name: "LayerModule",
        wit_dir: "layer-support",
        wit_package: "slicer:layer-support@1.0.0",
        wit_interface: "support",
        wit_world: "support-module",
    },
    StageSpec {
        method: "run_support_postprocess",
        stage_id: "Layer::SupportPostProcess",
        wit_export: "run",
        tier_id: TIER_LAYER,
        trait_name: "LayerModule",
        wit_dir: "layer-support-postprocess",
        wit_package: "slicer:layer-support-postprocess@1.0.0",
        wit_interface: "support-postprocess",
        wit_world: "support-postprocess-module",
    },
    StageSpec {
        method: "run_path_optimization",
        stage_id: "Layer::PathOptimization",
        wit_export: "run",
        tier_id: TIER_LAYER,
        trait_name: "LayerModule",
        wit_dir: "layer-path-optimization",
        wit_package: "slicer:layer-path-optimization@1.0.0",
        wit_interface: "path-optimization",
        wit_world: "path-optimization-module",
    },
    // ── Prepass world (TIER_PREPASS) ──────────────────────────────────
    StageSpec {
        method: "run_mesh_analysis",
        stage_id: "PrePass::MeshAnalysis",
        wit_export: "run",
        tier_id: TIER_PREPASS,
        trait_name: "PrepassModule",
        wit_dir: "prepass-mesh-analysis",
        wit_package: "slicer:prepass-mesh-analysis@1.0.0",
        wit_interface: "mesh-analysis",
        wit_world: "mesh-analysis-module",
    },
    StageSpec {
        method: "run_layer_planning",
        stage_id: "PrePass::LayerPlanning",
        wit_export: "run",
        tier_id: TIER_PREPASS,
        trait_name: "PrepassModule",
        wit_dir: "prepass-layer-planning",
        wit_package: "slicer:prepass-layer-planning@1.0.0",
        wit_interface: "layer-planning",
        wit_world: "layer-planning-module",
    },
    // Host-built-in since packet 97 (crates/slicer-runtime/src/prepass.rs); no WIT contract.
    StageSpec {
        method: "run_paint_segmentation",
        stage_id: "PrePass::PaintSegmentation",
        wit_export: "",
        tier_id: TIER_PREPASS,
        trait_name: "PrepassModule",
        wit_dir: "",
        wit_package: "",
        wit_interface: "",
        wit_world: "",
    },
    StageSpec {
        method: "run_seam_planning",
        stage_id: "PrePass::SeamPlanning",
        wit_export: "run",
        tier_id: TIER_PREPASS,
        trait_name: "PrepassModule",
        wit_dir: "prepass-seam-planning",
        wit_package: "slicer:prepass-seam-planning@1.0.0",
        wit_interface: "seam-planning",
        wit_world: "seam-planning-module",
    },
    StageSpec {
        method: "run_support_geometry",
        stage_id: "PrePass::SupportGeometry",
        wit_export: "run",
        tier_id: TIER_PREPASS,
        trait_name: "PrepassModule",
        wit_dir: "prepass-support-geometry",
        wit_package: "slicer:prepass-support-geometry@1.0.0",
        wit_interface: "support-geometry",
        wit_world: "support-geometry-module",
    },
    // ── Finalization world (TIER_FINALIZATION) ────────────────────────
    StageSpec {
        method: "run_finalization",
        stage_id: "PostPass::LayerFinalization",
        wit_export: "run",
        tier_id: TIER_FINALIZATION,
        trait_name: "FinalizationModule",
        wit_dir: "finalization-layer-finalization",
        wit_package: "slicer:finalization-layer-finalization@1.0.0",
        wit_interface: "layer-finalization",
        wit_world: "layer-finalization-module",
    },
    // ── Postpass world (TIER_POSTPASS) ────────────────────────────────
    StageSpec {
        method: "run_gcode_postprocess",
        stage_id: "PostPass::GCodePostProcess",
        wit_export: "run",
        tier_id: TIER_POSTPASS,
        trait_name: "PostpassModule",
        wit_dir: "postpass-gcode-postprocess",
        wit_package: "slicer:postpass-gcode-postprocess@1.0.0",
        wit_interface: "gcode-postprocess",
        wit_world: "gcode-postprocess-module",
    },
    StageSpec {
        method: "run_text_postprocess",
        stage_id: "PostPass::TextPostProcess",
        wit_export: "run",
        tier_id: TIER_POSTPASS,
        trait_name: "PostpassModule",
        wit_dir: "postpass-text-postprocess",
        wit_package: "slicer:postpass-text-postprocess@1.0.0",
        wit_interface: "text-postprocess",
        wit_world: "text-postprocess-module",
    },
];

/// Kind of a single WIT export carried by a module's binding surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    /// The stage-specific export detected in the impl (e.g. `run-infill`).
    Stage,
}

/// One WIT export entry in a module's binding schema: the kebab-case
/// export name the guest provides as a stage export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportBinding {
    /// Kebab-case WIT export name, e.g. `"run-infill"`.
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
    /// Tier this impl belongs to (e.g. [`TIER_LAYER`]) — vocabulary, not a
    /// WIT package — or `""` if the impl targets no known trait or stage.
    pub tier_id: &'static str,
    /// Canonical scheduler stage id (e.g. `"Layer::Infill"`) or `""` if no
    /// stage method was detected.
    pub stage_id: &'static str,
    /// Rust-cased stage method name (e.g. `"run_infill"`) or `""`.
    pub stage_method: &'static str,
    /// Kebab-case stage export name (e.g. `"run-infill"`) or `""`.
    pub stage_export: &'static str,
    /// Complete ordered export surface: the detected stage export, if any
    /// (0 or 1 entries).
    pub exports: &'static [ExportBinding],
}

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
pub fn tier_for_stage_id(stage_id: &str) -> Option<&'static str> {
    stage_by_id(stage_id).map(|s| s.tier_id)
}

/// Return the SDK trait that carries `stage_id`.
#[must_use]
pub fn trait_for_stage_id(stage_id: &str) -> Option<&'static str> {
    stage_by_id(stage_id).map(|s| s.trait_name)
}

/// Map an SDK trait name (e.g. `"LayerModule"`) to its WIT world id, if
/// the trait is one of the known four.
#[must_use]
pub fn tier_for_trait(trait_name: &str) -> Option<&'static str> {
    match trait_name {
        "LayerModule" => Some(TIER_LAYER),
        "PrepassModule" => Some(TIER_PREPASS),
        "FinalizationModule" => Some(TIER_FINALIZATION),
        "PostpassModule" => Some(TIER_POSTPASS),
        _ => None,
    }
}

/// Return the full list of canonical stage ids, in table order.
#[must_use]
pub fn all_stage_ids() -> Vec<&'static str> {
    STAGES.iter().map(|s| s.stage_id).collect()
}

/// Look up the WIT export name for a stage id from the single source of truth in [`STAGES`].
///
/// Returns `None` for unknown stage ids and the host-built-in
/// `PrePass::PaintSegmentation`. Dispatcher impls MUST use this lookup; they MUST NOT
/// hardcode their own stage-id → wit-export table (see ADR-0005, planned at P83 close).
#[must_use]
pub fn export_for_stage_id(stage_id: &str) -> Option<&'static str> {
    STAGES
        .iter()
        .find(|s| s.stage_id == stage_id)
        .map(|s| s.wit_export)
        .filter(|export| !export.is_empty())
}

/// Look up the per-stage WIT package directory (under
/// `crates/slicer-schema/wit/deps/`) for a stage id.
///
/// Returns `None` for unknown stage ids and the host-built-in
/// `PrePass::PaintSegmentation`.
///
/// Packet 163 pilot: `PostPass::GCodePostProcess` → `"postpass-gcode-postprocess"`,
/// `PostPass::TextPostProcess` → `"postpass-text-postprocess"`,
/// `PostPass::LayerFinalization` → `"finalization-layer-finalization"`.
#[must_use]
pub fn wit_dir_for_stage_id(stage_id: &str) -> Option<&'static str> {
    STAGES
        .iter()
        .find(|s| s.stage_id == stage_id)
        .map(|s| s.wit_dir)
        .filter(|dir| !dir.is_empty())
}

/// Look up the per-stage WIT package identifier (e.g.
/// `"slicer:postpass-gcode-postprocess@1.0.0"`) for a stage id. Returns
/// `None` for unknown stage ids and for stages that have not yet been
/// migrated to a per-stage package.
///
/// Packet 163 pilot: only the three pilot stages return `Some`; the 13
/// unmigrated layer/prepass stages return `None`.
#[must_use]
pub fn package_for_stage_id(stage_id: &str) -> Option<&'static str> {
    STAGES
        .iter()
        .find(|s| s.stage_id == stage_id)
        .map(|s| s.wit_package)
        .filter(|s| !s.is_empty())
}

/// Look up the per-stage WIT exported-interface name (e.g.
/// `"gcode-postprocess"`) for a stage id. Returns `None` for unknown or
/// unmigrated stages.
#[must_use]
pub fn interface_for_stage_id(stage_id: &str) -> Option<&'static str> {
    STAGES
        .iter()
        .find(|s| s.stage_id == stage_id)
        .map(|s| s.wit_interface)
        .filter(|s| !s.is_empty())
}

/// Look up the per-stage WIT world name (e.g. `"gcode-postprocess-module"`)
/// for a stage id. Returns `None` for unknown or unmigrated stages.
#[must_use]
pub fn wit_world_for_stage_id(stage_id: &str) -> Option<&'static str> {
    STAGES
        .iter()
        .find(|s| s.stage_id == stage_id)
        .map(|s| s.wit_world)
        .filter(|s| !s.is_empty())
}

/// Return the fully-qualified component-model export identity for a stage
/// id, e.g. `"slicer:postpass-gcode-postprocess/gcode-postprocess@1.0.0#run"`.
/// This is the **only** lookup that fully identifies a migrated stage's
/// contract — `export_for_stage_id` no longer does, because the func name is
/// `"run"` for every migrated stage and is not unique on its own.
///
/// Returns `None` for unknown **and** unmigrated stage ids.
#[must_use]
pub fn qualified_export_for_stage_id(stage_id: &str) -> Option<String> {
    let s = STAGES.iter().find(|s| s.stage_id == stage_id)?;
    if s.wit_package.is_empty() {
        return None;
    }
    // `s.wit_package` is `<ns>:<name>@<version>` (e.g.
    // `"slicer:postpass-gcode-postprocess@1.0.0"`). The qualified export
    // form is `<ns>:<name>/<interface>@<version>#<func>`, so split the
    // version off and re-attach after the interface.
    let (pkg_no_ver, ver) = s.wit_package.split_once('@')?;
    Some(format!(
        "{pkg_no_ver}/{iface}@{ver}#{export}",
        iface = s.wit_interface,
        export = s.wit_export,
    ))
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
            // `PrePass::PaintSegmentation` is the sole host-built-in stage
            // (packet 97; crates/slicer-runtime/src/prepass.rs); its five WIT
            // columns are intentionally empty. The other 15 stages all declare
            // per-stage packages.
            if s.stage_id == "PrePass::PaintSegmentation" {
                assert!(s.wit_dir.is_empty());
                assert!(s.wit_package.is_empty());
                assert!(s.wit_interface.is_empty());
                assert!(s.wit_world.is_empty());
                assert!(s.wit_export.is_empty());
                continue;
            }
            assert_eq!(stage_by_id(s.stage_id).unwrap(), s);
            assert_eq!(stage_by_method(s.method).unwrap(), s);
            assert_eq!(tier_for_stage_id(s.stage_id), Some(s.tier_id));
            assert_eq!(tier_for_trait(s.trait_name), Some(s.tier_id));
            // Every non-built-in row must declare a non-empty per-stage package dir.
            assert!(!s.wit_dir.is_empty(), "wit_dir is empty for {}", s.stage_id);
            // A non-empty wit_package implies non-empty wit_interface and
            // wit_world, and vice versa: per-stage package migration is
            // all-or-nothing per row.
            assert_eq!(
                !s.wit_package.is_empty(),
                !s.wit_interface.is_empty(),
                "wit_package / wit_interface mismatch for {}",
                s.stage_id
            );
            assert_eq!(
                !s.wit_package.is_empty(),
                !s.wit_world.is_empty(),
                "wit_package / wit_world mismatch for {}",
                s.stage_id
            );
            // wit_export on a migrated stage is "run" by design (ADR-0045).
            if !s.wit_package.is_empty() {
                assert_eq!(
                    s.wit_export, "run",
                    "migrated stage {} should have wit_export = \"run\"",
                    s.stage_id
                );
            }
        }
    }

    #[test]
    fn package_lookups_match_per_stage_table() {
        // Migrated stages: every per-stage lookup returns the column value.
        let gcode = "PostPass::GCodePostProcess";
        assert_eq!(
            package_for_stage_id(gcode),
            Some("slicer:postpass-gcode-postprocess@1.0.0")
        );
        assert_eq!(interface_for_stage_id(gcode), Some("gcode-postprocess"));
        assert_eq!(
            wit_world_for_stage_id(gcode),
            Some("gcode-postprocess-module")
        );
        assert_eq!(
            wit_dir_for_stage_id(gcode),
            Some("postpass-gcode-postprocess")
        );
        assert_eq!(export_for_stage_id(gcode), Some("run"));
        assert_eq!(
            qualified_export_for_stage_id(gcode).as_deref(),
            Some("slicer:postpass-gcode-postprocess/gcode-postprocess@1.0.0#run"),
        );

        let text = "PostPass::TextPostProcess";
        assert_eq!(
            package_for_stage_id(text),
            Some("slicer:postpass-text-postprocess@1.0.0")
        );
        assert_eq!(interface_for_stage_id(text), Some("text-postprocess"));
        assert_eq!(
            wit_world_for_stage_id(text),
            Some("text-postprocess-module")
        );
        assert_eq!(
            wit_dir_for_stage_id(text),
            Some("postpass-text-postprocess")
        );
        assert_eq!(export_for_stage_id(text), Some("run"));
        assert_eq!(
            qualified_export_for_stage_id(text).as_deref(),
            Some("slicer:postpass-text-postprocess/text-postprocess@1.0.0#run"),
        );

        let finalization = "PostPass::LayerFinalization";
        assert_eq!(
            package_for_stage_id(finalization),
            Some("slicer:finalization-layer-finalization@1.0.0")
        );
        assert_eq!(
            interface_for_stage_id(finalization),
            Some("layer-finalization")
        );
        assert_eq!(
            wit_world_for_stage_id(finalization),
            Some("layer-finalization-module")
        );
        assert_eq!(
            wit_dir_for_stage_id(finalization),
            Some("finalization-layer-finalization")
        );
        assert_eq!(export_for_stage_id(finalization), Some("run"));
        assert_eq!(
            qualified_export_for_stage_id(finalization).as_deref(),
            Some("slicer:finalization-layer-finalization/layer-finalization@1.0.0#run"),
        );

        let perimeters = "Layer::Perimeters";
        assert_eq!(
            package_for_stage_id(perimeters),
            Some("slicer:layer-perimeters@1.0.0")
        );
        assert_eq!(interface_for_stage_id(perimeters), Some("perimeters"));
        assert_eq!(
            wit_world_for_stage_id(perimeters),
            Some("perimeters-module")
        );
        assert_eq!(wit_dir_for_stage_id(perimeters), Some("layer-perimeters"));
        assert_eq!(export_for_stage_id(perimeters), Some("run"));
        assert_eq!(
            qualified_export_for_stage_id(perimeters).as_deref(),
            Some("slicer:layer-perimeters/perimeters@1.0.0#run"),
        );

        // PrePass::PaintSegmentation is the sole host-built-in stage:
        // every WIT lookup returns None.
        let paint_seg = "PrePass::PaintSegmentation";
        for fn_name in [
            package_for_stage_id,
            interface_for_stage_id,
            wit_world_for_stage_id,
            export_for_stage_id,
        ] {
            assert_eq!(fn_name(paint_seg), None, "{fn_name:?}({paint_seg})");
        }
        assert_eq!(wit_dir_for_stage_id(paint_seg), None);
        assert_eq!(qualified_export_for_stage_id(paint_seg), None);

        // Unknown stage: every lookup returns None.
        for fn_name in [
            package_for_stage_id,
            interface_for_stage_id,
            wit_world_for_stage_id,
        ] {
            assert_eq!(fn_name("NotAStage"), None);
        }
        assert_eq!(wit_dir_for_stage_id("NotAStage"), None);
        assert_eq!(qualified_export_for_stage_id("NotAStage"), None);
    }
}
