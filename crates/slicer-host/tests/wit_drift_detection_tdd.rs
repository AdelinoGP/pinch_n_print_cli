//! Drift-detection regression test: proves that the embedded WIT strings
//! in the macro (`lib.rs`) and host (`wit_host.rs`) are derived from the
//! canonical on-disk `wit/` files.
//!
//! This test prevents future drift where someone modifies a disk WIT file
//! without updating the corresponding embedded copy in the macro or host.
//!
//! Run with:
//!   cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

/// Returns the workspace root by climbing from CARGO_MANIFEST_DIR up to the dir
/// that contains `Cargo.toml` (the workspace root).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
    // slicer-host crate is at crates/slicer-host/; go up two levels to workspace root.
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("slicer-host is at crates/slicer-host/ — workspace root is two levels up")
        .to_path_buf()
}

// ─────────────────────────────────────────────────────────────────────────────
// Macro WIT source verification
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that the macro's WIT strings (in `lib.rs`) use `include` directives
/// for the canonical dep files.  The macro uses WIT `include` directives (not
/// Rust `include_str!`) to pull in the canonical `deps/types.wit`,
/// `deps/config.wit`, and `deps/ir-types.wit` files.
///
/// The macro defines worlds inline (rather than including `world-*.wit` files)
/// because it must emit additional `export` functions specific to the macro's
/// SDK glue.  This is intentional — the dep files are the primary drift risk.
#[test]
fn macro_uses_canonical_dep_includes() {
    let lib_rs = macro_lib_rs_content();

    // All four world WIT strings should include types.wit and config.wit.
    // The exact constant names differ (LAYER_WORLD_WIT vs build_*_world_glue),
    // so we just scan for the include directives.
    assert!(
        lib_rs.contains(r#"include "../../wit/deps/types.wit""#),
        "macro WIT strings should include canonical deps/types.wit"
    );
    assert!(
        lib_rs.contains(r#"include "../../wit/deps/config.wit""#),
        "macro WIT strings should include canonical deps/config.wit"
    );
    // Only LAYER_WORLD_WIT includes ir-types.wit (it needs the full ir-handles).
    assert!(
        lib_rs.contains(r#"include "../../wit/deps/ir-types.wit""#),
        "macro LAYER_WORLD_WIT should include canonical deps/ir-types.wit"
    );
}

/// Verifies that the macro's layer-world WIT string has the canonical package name.
#[test]
fn macro_layer_world_package_name_is_canonical() {
    let lib_rs = macro_lib_rs_content();
    assert!(
        lib_rs.contains(r#"package slicer:world-layer@1.0.0;"#),
        "LAYER_WORLD_WIT should use canonical 'slicer:world-layer@1.0.0', not 'slicer:layer-world@1.0.0'"
    );
    // The old (wrong) package name must not appear.
    assert!(
        !lib_rs.contains(r#"package slicer:layer-world@1.0.0"#),
        "LAYER_WORLD_WIT must not contain pre-consolidation 'slicer:layer-world@1.0.0'"
    );
}

/// Verifies that the macro's prepass/postpass/finalization WIT strings use
/// canonical package names.
#[test]
fn macro_other_world_package_names_are_canonical() {
    let lib_rs = macro_lib_rs_content();
    // Pre-consolidation names must not appear in any WIT string.
    let disallowed = [
        "slicer:prepass-world@",
        "slicer:postpass-world@",
        "slicer:finalization-world@",
    ];
    for wrong in disallowed {
        assert!(
            !lib_rs.contains(&format!("package {wrong}")),
            "macro WIT must not contain pre-consolidation package prefix '{wrong}'"
        );
    }
    // Canonical names should be present.
    assert!(lib_rs.contains(r#"package slicer:world-prepass@1.0.0;"#));
    assert!(lib_rs.contains(r#"package slicer:world-postpass@1.0.0;"#));
    assert!(lib_rs.contains(r#"package slicer:world-finalization@1.0.0;"#));
}

// ─────────────────────────────────────────────────────────────────────────────
// Host WIT source verification
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that all four host inline WIT blocks use the canonical world package
/// names.  Pre-consolidation names (e.g. `slicer:layer-world`) must not appear.
#[test]
fn host_inline_wit_uses_canonical_world_package_names() {
    let wit_host_rs = host_wit_host_rs_content();

    // Check each canonical world package name appears in the inline WIT blocks.
    let canonical_worlds = [
        "package slicer:world-layer@1.0.0",
        "package slicer:world-prepass@1.0.0",
        "package slicer:world-postpass@1.0.0",
        "package slicer:world-finalization@1.0.0",
    ];
    for canonical in canonical_worlds {
        assert!(
            wit_host_rs.contains(canonical),
            "host inline WIT should contain canonical package '{canonical}'"
        );
    }

    // Verify the pre-consolidation (wrong) package names do NOT appear.
    let disallowed = [
        "package slicer:layer-world@1.0.0",
        "package slicer:prepass-world@1.0.0",
        "package slicer:postpass-world@1.0.0",
        "package slicer:finalization-world@1.0.0",
    ];
    for wrong in disallowed {
        assert!(
            !wit_host_rs.contains(wrong),
            "host inline WIT must not contain pre-consolidation package name '{wrong}'"
        );
    }
}

/// Verifies that the `with:` block keys in host `wit_host.rs` use the canonical
/// world package names (e.g. `slicer:world-layer/...` not `slicer:layer-world/...`).
#[test]
fn host_bindgen_with_keys_use_canonical_world_names() {
    let wit_host_rs = host_wit_host_rs_content();

    // Check canonical with: keys are present (version-suffixed format as emitted by wasmtime bindgen).
    let canonical_keys = [
        r#""slicer:world-layer/config-types@1.0.0.config-view""#,
        r#""slicer:world-prepass/config-types@1.0.0.config-view""#,
        r#""slicer:world-finalization/config-types@1.0.0.config-view""#,
        r#""slicer:world-postpass/config-types@1.0.0.config-view""#,
    ];
    for key in canonical_keys {
        assert!(
            wit_host_rs.contains(key),
            "host bindgen with: block should contain canonical key '{key}'"
        );
    }

    // Check old (wrong) with: keys are absent.
    let disallowed_keys = [
        r#""slicer:layer-world/config-types/config-view""#,
        r#""slicer:prepass-world/config-types/config-view""#,
    ];
    for wrong in disallowed_keys {
        assert!(
            !wit_host_rs.contains(wrong),
            "host bindgen with: block must not contain pre-consolidation key '{wrong}'"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Disk canonical file existence verification
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that all four canonical world WIT files exist on disk.
#[test]
fn canonical_world_files_exist_on_disk() {
    let root = workspace_root();
    for world_file in [
        "world-layer.wit",
        "world-prepass.wit",
        "world-postpass.wit",
        "world-finalization.wit",
    ] {
        let path = root.join(format!("wit/{world_file}"));
        assert!(
            path.exists(),
            "canonical WIT file '{}' must exist on disk at {:?}",
            world_file,
            path
        );
    }
}

/// Verifies that all three canonical dep WIT files exist on disk.
#[test]
fn canonical_dep_files_exist_on_disk() {
    let root = workspace_root();
    for dep_file in ["deps/types.wit", "deps/config.wit", "deps/ir-types.wit"] {
        let path = root.join(format!("wit/{dep_file}"));
        assert!(
            path.exists(),
            "canonical WIT dep file '{}' must exist on disk at {:?}",
            dep_file,
            path
        );
    }
}

/// Verifies that the disk canonical ir-types.wit contains the `needs-support`
/// interface member that was previously missing from inline copies.
#[test]
fn canonical_ir_types_has_needs_support() {
    let path = workspace_root().join("wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    assert!(
        content.contains("needs-support"),
        "canonical ir-types.wit must contain 'needs-support' interface member"
    );
}

/// Verifies that the disk canonical ir-types.wit contains `push-z-hop`
/// in the gcode-output-builder.
#[test]
fn canonical_ir_types_has_push_z_hop() {
    let path = workspace_root().join("wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    assert!(
        content.contains("push-z-hop"),
        "canonical ir-types.wit must contain 'push-z-hop' in gcode-output-builder"
    );
}

/// Verifies that the disk canonical ir-types.wit contains `push-unretract`
/// in the gcode-output-builder.
#[test]
fn canonical_ir_types_has_push_unretract() {
    let path = workspace_root().join("wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    assert!(
        content.contains("push-unretract"),
        "canonical ir-types.wit must contain 'push-unretract' in gcode-output-builder"
    );
}

/// Verifies that the canonical postpass world widened to payload-bearing
/// command input with explicit unretract support.
#[test]
fn canonical_world_postpass_has_payload_command_input() {
    let path = workspace_root().join("wit/world-postpass.wit");
    let content = fs::read_to_string(&path).expect("read canonical world-postpass.wit");
    assert!(
        content.contains("variant gcode-command"),
        "canonical world-postpass.wit must define payload-bearing 'variant gcode-command'"
    );
    assert!(
        content.contains("unretract"),
        "canonical world-postpass.wit must carry an 'unretract' command case"
    );
}

/// Verifies that the canonical finalization world widened layer-collection-view
/// with ordered-entity and z-hop reads.
#[test]
fn canonical_world_finalization_has_entity_and_zhop_reads() {
    let path = workspace_root().join("wit/world-finalization.wit");
    let content = fs::read_to_string(&path).expect("read canonical world-finalization.wit");
    assert!(
        content.contains("ordered-entities"),
        "canonical world-finalization.wit must expose 'ordered-entities'"
    );
    assert!(
        content.contains("z-hops"),
        "canonical world-finalization.wit must expose 'z-hops'"
    );
}

/// Verifies that the macro's embedded postpass/finalization WIT strings track
/// the widened canonical surfaces.
#[test]
fn macro_embedded_wit_tracks_boundary_widening() {
    let lib_rs = macro_lib_rs_content();
    assert!(
        lib_rs.contains("push-unretract"),
        "macro embedded WIT must contain 'push-unretract' after postpass widening"
    );
    assert!(
        lib_rs.contains("variant gcode-command"),
        "macro embedded postpass WIT must define payload-bearing 'variant gcode-command'"
    );
    assert!(
        lib_rs.contains("ordered-entities"),
        "macro embedded finalization WIT must expose 'ordered-entities'"
    );
    assert!(
        lib_rs.contains("z-hops"),
        "macro embedded finalization WIT must expose 'z-hops'"
    );
}

/// Verifies that the host's embedded postpass/finalization WIT strings track
/// the widened canonical surfaces.
#[test]
fn host_embedded_wit_tracks_boundary_widening() {
    let wit_host_rs = host_wit_host_rs_content();
    assert!(
        wit_host_rs.contains("push-unretract"),
        "host embedded WIT must contain 'push-unretract' after postpass widening"
    );
    assert!(
        wit_host_rs.contains("variant gcode-command"),
        "host embedded postpass WIT must define payload-bearing 'variant gcode-command'"
    );
    assert!(
        wit_host_rs.contains("ordered-entities"),
        "host embedded finalization WIT must expose 'ordered-entities'"
    );
    assert!(
        wit_host_rs.contains("z-hops"),
        "host embedded finalization WIT must expose 'z-hops'"
    );
}

/// Verifies that hand-written test guests carrying `extrusion-role`
/// use the payload-bearing variant form and current world package names.
#[test]
fn handwritten_test_guests_use_payload_extrusion_role_variants() {
    let guests = [
        (
            "layer-infill-guest",
            "package slicer:world-layer@1.0.0;",
            Some("package slicer:layer-world@1.0.0;"),
            &["variant extrusion-role", "custom(string)"][..],
        ),
        (
            "postpass-guest",
            "package slicer:world-postpass@1.0.0;",
            Some("package slicer:postpass-world@1.0.0;"),
            &["variant extrusion-role", "custom(string)", "push-unretract"][..],
        ),
        (
            "finalization-guest",
            "package slicer:world-finalization@1.0.0;",
            Some("package slicer:finalization-world@1.0.0;"),
            &[
                "variant extrusion-role",
                "custom(string)",
                "ordered-entities",
                "z-hops",
            ][..],
        ),
    ];

    for (guest_name, canonical_package, disallowed_package, required_snippets) in guests {
        let content = test_guest_lib_rs_content(guest_name);
        assert!(
            content.contains(canonical_package),
            "{guest_name} should use canonical package '{canonical_package}'"
        );
        if let Some(disallowed_package) = disallowed_package {
            assert!(
                !content.contains(disallowed_package),
                "{guest_name} must not use stale package '{disallowed_package}'"
            );
        }
        assert!(
            !content.contains("enum extrusion-role"),
            "{guest_name} must not use stale 'enum extrusion-role'"
        );
        for snippet in required_snippets {
            assert!(
                content.contains(snippet),
                "{guest_name} should contain '{snippet}'"
            );
        }
    }
}

/// Verifies that the `#[slicer_module]` macro's embedded layer-world WIT
/// references the `layer-collection-builder` resource — both in the world's
/// `use ir-handles.{...}` import block and in the `run-path-optimization`
/// export signature — and that the canonical disk `wit/deps/ir-types.wit`
/// (which the macro pulls in via `include`) declares the resource with the
/// canonical `set-entity-order` signature (packet 32 — TASK-152g).
///
/// Drift between disk WIT and the macro's embedded LAYER_WORLD_WIT here
/// would silently break the guest-side bindings produced by the macro.
#[test]
fn macro_embeds_layer_collection_builder_resource() {
    let lib_rs = macro_lib_rs_content();
    assert!(
        lib_rs.contains("layer-collection-builder,"),
        "macro LAYER_WORLD_WIT must import 'layer-collection-builder' in the world's `use ir-handles.{{...}}` block"
    );
    assert!(
        lib_rs.contains("collection: layer-collection-builder"),
        "macro LAYER_WORLD_WIT must wire 'collection: layer-collection-builder' into run-path-optimization"
    );

    // The actual resource declaration lives in the canonical disk WIT
    // (the macro pulls it in via WIT `include`).
    let ir_types = fs::read_to_string(workspace_root().join("wit/deps/ir-types.wit"))
        .expect("read canonical ir-types.wit");
    assert!(
        ir_types.contains("resource layer-collection-builder"),
        "canonical wit/deps/ir-types.wit must declare 'resource layer-collection-builder'"
    );
    assert!(
        ir_types.contains(
            "set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>"
        ),
        "canonical wit/deps/ir-types.wit must declare set-entity-order with the canonical signature"
    );
    assert!(
        ir_types.contains("get-ordered-entities: func() -> list<ordered-entity-view>"),
        "canonical wit/deps/ir-types.wit must declare get-ordered-entities with the canonical signature"
    );
    assert!(
        ir_types.contains("record ordered-entity-view"),
        "canonical wit/deps/ir-types.wit must declare 'record ordered-entity-view'"
    );
    // Spot-check one critical field of the record.
    assert!(
        ir_types.contains("original-index: u32"),
        "canonical wit/deps/ir-types.wit ordered-entity-view must carry 'original-index: u32'"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Prepass segmentation signature surface
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that the canonical prepass world uses mesh-object-view (not raw
/// object-id) for the run-mesh-segmentation export.
#[test]
fn prepass_world_uses_mesh_object_view() {
    let path = workspace_root().join("wit/world-prepass.wit");
    let content = fs::read_to_string(&path).expect("read canonical world-prepass.wit");
    // Normalize CRLF → LF so the contains check works across platforms.
    let normalized = content.replace("\r\n", "\n");
    // The export signature must declare list<mesh-object-view>, not list<object-id>.
    assert!(
        normalized
            .contains("run-mesh-segmentation: func(\n        objects: list<mesh-object-view>"),
        "canonical world-prepass.wit must use list<mesh-object-view> for run-mesh-segmentation"
    );
    // The old form must not appear.
    assert!(
        !normalized.contains("run-mesh-segmentation: func(\n        objects: list<object-id>"),
        "canonical world-prepass.wit must not use stale list<object-id> for run-mesh-segmentation"
    );
}

/// Verifies that the canonical prepass world uses paint-segmentation-object-view
/// (not raw object-id) for the run-paint-segmentation export.
#[test]
fn prepass_world_uses_paint_segmentation_object_view() {
    let path = workspace_root().join("wit/world-prepass.wit");
    let content = fs::read_to_string(&path).expect("read canonical world-prepass.wit");
    let normalized = content.replace("\r\n", "\n");
    assert!(
        normalized.contains("run-paint-segmentation: func(\n        objects: list<paint-segmentation-object-view>"),
        "canonical world-prepass.wit must use list<paint-segmentation-object-view> for run-paint-segmentation"
    );
    assert!(
        !normalized.contains("run-paint-segmentation: func(\n        objects: list<object-id>"),
        "canonical world-prepass.wit must not use stale list<object-id> for run-paint-segmentation"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Seam-related layer-world members
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that perimeter-region-view exposes resolved-seam as a read member.
#[test]
fn perimeter_region_view_has_resolved_seam() {
    let path = workspace_root().join("wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    assert!(
        content.contains("resolved-seam: func() -> option<seam-position>"),
        "perimeter-region-view must expose resolved-seam read member"
    );
}

/// Verifies that perimeter-output-builder exposes push-reordered-wall-loop and
/// push-resolved-seam as write members.
#[test]
fn perimeter_output_builder_has_seam_write_methods() {
    let path = workspace_root().join("wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    assert!(
        content.contains("push-reordered-wall-loop:"),
        "perimeter-output-builder must expose push-reordered-wall-loop"
    );
    assert!(
        content.contains("push-resolved-seam:"),
        "perimeter-output-builder must expose push-resolved-seam"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the content of `crates/slicer-macros/src/lib.rs`.
/// Uses `std::fs::read_to_string` at test runtime.
fn macro_lib_rs_content() -> String {
    let path = workspace_root().join("crates/slicer-macros/src/lib.rs");
    fs::read_to_string(&path).expect("read macro lib.rs for WIT include verification")
}

/// Returns the content of `crates/slicer-host/src/wit_host.rs`.
/// Uses `std::fs::read_to_string` at test runtime.
fn host_wit_host_rs_content() -> String {
    let path = workspace_root().join("crates/slicer-host/src/wit_host.rs");
    fs::read_to_string(&path).expect("read host wit_host.rs for inline WIT verification")
}

/// Returns the content of a hand-written test guest `src/lib.rs`.
fn test_guest_lib_rs_content(guest_name: &str) -> String {
    let path = workspace_root().join(format!("test-guests/{guest_name}/src/lib.rs"));
    fs::read_to_string(&path).expect("read test guest lib.rs for embedded WIT verification")
}
