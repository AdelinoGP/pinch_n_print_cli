//! Drift-detection regression test: proves that the embedded WIT strings
//! in the macro (`lib.rs`) and host (`wit_host.rs`) are derived from the
//! canonical on-disk `crates/slicer-schema/wit/` files.
//!
//! This test prevents future drift where someone modifies a disk WIT file
//! without updating the corresponding embedded copy in the macro or host.
//!
//! Run with:
//!   cargo test --package slicer-runtime --test wit_drift_detection_tdd -- --nocapture

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

/// Returns the workspace root by climbing from CARGO_MANIFEST_DIR up to the dir
/// that contains `Cargo.toml` (the workspace root).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
    // slicer-runtime crate is at crates/slicer-runtime/; go up two levels to workspace root.
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("slicer-runtime is at crates/slicer-runtime/ — workspace root is two levels up")
        .to_path_buf()
}

// ─────────────────────────────────────────────────────────────────────────────
// Macro WIT source verification
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that the macro sources all dep WIT content from the canonical
/// single-source files via Rust `include_str!` (not WIT `include` directives).
/// Under single-source (packet 72), the macro reads dep files via include_str! at
/// compile time and assembles the inline blob at runtime — this is the drift guard.
#[test]
fn macro_uses_canonical_dep_includes() {
    let lib_rs = macro_lib_rs_content();

    // The macro must pull each shared dep from the canonical single-source path.
    assert!(
        lib_rs.contains(r#"include_str!("../../slicer-schema/wit/deps/types.wit")"#),
        "macro must source types.wit from canonical single-source via include_str!"
    );
    assert!(
        lib_rs.contains(r#"include_str!("../../slicer-schema/wit/deps/config.wit")"#),
        "macro must source config.wit from canonical single-source via include_str!"
    );
    // ir-types.wit is only needed for the layer world (it declares ir-handles).
    assert!(
        lib_rs.contains(r#"include_str!("../../slicer-schema/wit/deps/ir-types.wit")"#),
        "macro must source ir-types.wit from canonical single-source via include_str!"
    );
}

/// Verifies that the macro's layer-world WIT source has the canonical package name.
/// Under single-source, the package name lives in the canonical world-layer.wit file
/// which the macro includes via include_str!; we assert both the disk file and the
/// include_str! reference point to the same canonical path.
#[test]
fn macro_layer_world_package_name_is_canonical() {
    let root = workspace_root();
    // The canonical world-layer.wit must declare the correct package.
    let world_layer =
        fs::read_to_string(root.join("crates/slicer-schema/wit/deps/world-layer/world-layer.wit"))
            .expect("read canonical world-layer.wit");
    assert!(
        world_layer.contains(r#"package slicer:world-layer@1.0.0;"#),
        "canonical world-layer.wit must use 'slicer:world-layer@1.0.0', not 'slicer:layer-world@1.0.0'"
    );
    assert!(
        !world_layer.contains(r#"package slicer:layer-world@1.0.0"#),
        "canonical world-layer.wit must not contain pre-consolidation 'slicer:layer-world@1.0.0'"
    );
    // Drift guard: the macro must source its layer-world WIT from this canonical file.
    let lib_rs = macro_lib_rs_content();
    assert!(
        lib_rs.contains(
            r#"include_str!("../../slicer-schema/wit/deps/world-layer/world-layer.wit")"#
        ),
        "macro LAYER_WORLD_WIT must be sourced from canonical single-source via include_str!"
    );
}

/// Verifies that the macro's prepass/postpass/finalization WIT sources use
/// canonical package names. Under single-source the canonical package declarations
/// live in the disk world files; we verify both the disk files and the macro's
/// include_str! references pointing to those files.
#[test]
fn macro_other_world_package_names_are_canonical() {
    let root = workspace_root();
    let canonical_worlds = [
        ("world-prepass", "slicer:world-prepass@1.0.0"),
        ("world-postpass", "slicer:world-postpass@1.0.0"),
        ("world-finalization", "slicer:world-finalization@1.0.0"),
    ];
    for (slug, pkg) in canonical_worlds {
        let path = root.join(format!("crates/slicer-schema/wit/deps/{slug}/{slug}.wit"));
        let content =
            fs::read_to_string(&path).unwrap_or_else(|_| panic!("read canonical {slug}.wit"));
        assert!(
            content.contains(&format!("package {pkg};")),
            "canonical {slug}.wit must declare package '{pkg}'"
        );
    }

    // Pre-consolidation names must not appear in the canonical world files.
    let disallowed = [
        "slicer:prepass-world@",
        "slicer:postpass-world@",
        "slicer:finalization-world@",
    ];
    for wrong in disallowed {
        for (slug, _) in [
            ("world-prepass", ""),
            ("world-postpass", ""),
            ("world-finalization", ""),
        ] {
            let path = root.join(format!("crates/slicer-schema/wit/deps/{slug}/{slug}.wit"));
            let content = fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {slug}.wit"));
            assert!(
                !content.contains(&format!("package {wrong}")),
                "{slug}.wit must not contain pre-consolidation package prefix '{wrong}'"
            );
        }
    }

    // Drift guard: confirm macro sources each world from the canonical single-source file.
    let lib_rs = macro_lib_rs_content();
    for slug in ["world-prepass", "world-postpass", "world-finalization"] {
        let expected = format!(r#"include_str!("../../slicer-schema/wit/deps/{slug}/{slug}.wit")"#);
        assert!(
            lib_rs.contains(&expected),
            "macro must source {slug} WIT from canonical single-source via include_str!"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Host WIT source verification
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that the host's bindgen! blocks reference canonical world paths.
/// Under single-source, the host uses `path: "../slicer-schema/wit"` (not inline WIT),
/// so the canonical package names appear in the `world:` key string, not as literal
/// `package …;` declarations. This redirected assertion checks those `world:` references.
#[test]
fn host_inline_wit_uses_canonical_world_package_names() {
    let wit_host_rs = host_wit_host_rs_content();

    // Single-source: host reads canonical dir via `path:`, not inline WIT.
    // Assert the shared canonical WIT directory is referenced.
    assert!(
        wit_host_rs.contains(r#"path: "../slicer-schema/wit""#),
        "host bindgen! must use canonical path '../slicer-schema/wit'"
    );

    // Each world is addressed by the canonical package-qualified `world:` key.
    let canonical_world_refs = [
        r#"world: "slicer:world-layer/layer-module@1.0.0""#,
        r#"world: "slicer:world-prepass/prepass-module@1.0.0""#,
        r#"world: "slicer:world-postpass/postpass-module@1.0.0""#,
        r#"world: "slicer:world-finalization/finalization-module@1.0.0""#,
    ];
    for canonical in canonical_world_refs {
        assert!(
            wit_host_rs.contains(canonical),
            "host bindgen! must reference canonical world '{canonical}'"
        );
    }

    // Verify the pre-consolidation (wrong) world keys do NOT appear.
    let disallowed = [
        "slicer:layer-world",
        "slicer:prepass-world",
        "slicer:postpass-world",
        "slicer:finalization-world",
    ];
    for wrong in disallowed {
        assert!(
            !wit_host_rs.contains(wrong),
            "host bindgen! must not contain pre-consolidation world ref '{wrong}'"
        );
    }
}

/// Verifies that the `with:` block keys in host `wit_host.rs` use the canonical
/// interface paths now that resources live in shared dep packages (single-source).
/// Under single-source the host maps `"slicer:config/config-types.config-view"` (shared
/// dep package) rather than the old per-world-versioned form.
#[test]
fn host_bindgen_with_keys_use_canonical_world_names() {
    let wit_host_rs = host_wit_host_rs_content();

    // Single-source: config-view is now a shared dep, so the with: key is the dep
    // package form, not a world-versioned form. Assert the canonical key is present
    // in each bindgen! block (one occurrence per world is sufficient).
    let canonical_key = r#""slicer:config/config-types.config-view""#;
    assert!(
        wit_host_rs.contains(canonical_key),
        "host bindgen with: block should contain canonical shared-dep key '{canonical_key}'"
    );

    // The old (wrong) per-world-versioned key forms must not appear.
    let disallowed_keys = [
        r#""slicer:layer-world/config-types/config-view""#,
        r#""slicer:prepass-world/config-types/config-view""#,
        r#""slicer:world-layer/config-types@1.0.0.config-view""#,
        r#""slicer:world-prepass/config-types@1.0.0.config-view""#,
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
/// Under the single-source layout (packet 72) worlds moved from the flat
/// `wit/world-X.wit` to `wit/deps/world-X/world-X.wit`.
// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_world_files_exist_on_disk() {
    let root = workspace_root();
    for world_slug in [
        "world-layer",
        "world-prepass",
        "world-postpass",
        "world-finalization",
    ] {
        let path = root.join(format!(
            "crates/slicer-schema/wit/deps/{world_slug}/{world_slug}.wit"
        ));
        assert!(
            path.exists(),
            "canonical WIT file '{world_slug}/{world_slug}.wit' must exist on disk at {:?}",
            path
        );
    }
}

/// Verifies that all three canonical dep WIT files exist on disk.
// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_dep_files_exist_on_disk() {
    let root = workspace_root();
    for dep_file in ["deps/types.wit", "deps/config.wit", "deps/ir-types.wit"] {
        let path = root.join(format!("crates/slicer-schema/wit/{dep_file}"));
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
// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_ir_types_has_needs_support() {
    let path = workspace_root().join("crates/slicer-schema/wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    assert!(
        content.contains("needs-support"),
        "canonical ir-types.wit must contain 'needs-support' interface member"
    );
}

/// Verifies that the disk canonical ir-types.wit contains `push-z-hop`
/// in the gcode-output-builder.
// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_ir_types_has_push_z_hop() {
    let path = workspace_root().join("crates/slicer-schema/wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    assert!(
        content.contains("push-z-hop"),
        "canonical ir-types.wit must contain 'push-z-hop' in gcode-output-builder"
    );
}

/// Verifies that the disk canonical ir-types.wit contains `push-unretract`
/// in the gcode-output-builder.
// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_ir_types_has_push_unretract() {
    let path = workspace_root().join("crates/slicer-schema/wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    assert!(
        content.contains("push-unretract"),
        "canonical ir-types.wit must contain 'push-unretract' in gcode-output-builder"
    );
}

/// Verifies that the canonical postpass world widened to payload-bearing
/// command input with explicit unretract support.
/// Redirected to single-source path: wit/deps/world-postpass/world-postpass.wit.
// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_world_postpass_has_payload_command_input() {
    let path =
        workspace_root().join("crates/slicer-schema/wit/deps/world-postpass/world-postpass.wit");
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
/// Redirected to single-source path: wit/deps/world-finalization/world-finalization.wit.
// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_world_finalization_has_entity_and_zhop_reads() {
    let path = workspace_root()
        .join("crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit");
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

/// Verifies that the canonical postpass/finalization WIT files carry the widened
/// surfaces, and that the macro's include_str! calls reference those canonical files.
/// Under single-source, "macro embedded WIT" means: the macro reads from canonical
/// disk files via include_str!, so drift is caught by checking the canonical files.
#[test]
fn macro_embedded_wit_tracks_boundary_widening() {
    let root = workspace_root();
    // Widened postpass surface — must be in the canonical postpass world.
    let postpass = fs::read_to_string(
        root.join("crates/slicer-schema/wit/deps/world-postpass/world-postpass.wit"),
    )
    .expect("read canonical world-postpass.wit");
    assert!(
        postpass.contains("push-unretract"),
        "canonical world-postpass.wit must contain 'push-unretract' after postpass widening"
    );
    assert!(
        postpass.contains("variant gcode-command"),
        "canonical world-postpass.wit must define payload-bearing 'variant gcode-command'"
    );

    // Widened finalization surface — must be in the canonical finalization world.
    let finalization = fs::read_to_string(
        root.join("crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit"),
    )
    .expect("read canonical world-finalization.wit");
    assert!(
        finalization.contains("ordered-entities"),
        "canonical world-finalization.wit must expose 'ordered-entities'"
    );
    assert!(
        finalization.contains("z-hops"),
        "canonical world-finalization.wit must expose 'z-hops'"
    );

    // Drift guard: confirm the macro sources its postpass/finalization WIT from the
    // canonical single-source files (not inline strings that could silently diverge).
    let lib_rs = macro_lib_rs_content();
    assert!(
        lib_rs.contains(
            r#"include_str!("../../slicer-schema/wit/deps/world-postpass/world-postpass.wit")"#
        ),
        "macro must source postpass WIT from canonical single-source via include_str!"
    );
    assert!(
        lib_rs.contains(r#"include_str!("../../slicer-schema/wit/deps/world-finalization/world-finalization.wit")"#),
        "macro must source finalization WIT from canonical single-source via include_str!"
    );
}

/// Verifies that the host's bindgen! blocks consume the canonical single-source WIT
/// which carries the widened postpass/finalization surfaces.
/// Under single-source, the host reads from the canonical dir via `path:` (not inline
/// WIT), so drift is caught by verifying the canonical WIT files and the host's path ref.
#[test]
fn host_embedded_wit_tracks_boundary_widening() {
    let root = workspace_root();
    // The widened surfaces must be present in the canonical world files.
    let postpass = fs::read_to_string(
        root.join("crates/slicer-schema/wit/deps/world-postpass/world-postpass.wit"),
    )
    .expect("read canonical world-postpass.wit");
    assert!(
        postpass.contains("push-unretract"),
        "canonical world-postpass.wit must contain 'push-unretract' after postpass widening"
    );
    assert!(
        postpass.contains("variant gcode-command"),
        "canonical world-postpass.wit must define payload-bearing 'variant gcode-command'"
    );

    let finalization = fs::read_to_string(
        root.join("crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit"),
    )
    .expect("read canonical world-finalization.wit");
    assert!(
        finalization.contains("ordered-entities"),
        "canonical world-finalization.wit must expose 'ordered-entities'"
    );
    assert!(
        finalization.contains("z-hops"),
        "canonical world-finalization.wit must expose 'z-hops'"
    );

    // Drift guard: the host must reference the canonical dir so wasmtime bindgen
    // picks up these widened surfaces automatically.
    let wit_host_rs = host_wit_host_rs_content();
    assert!(
        wit_host_rs.contains(r#"path: "../slicer-schema/wit""#),
        "host bindgen! must reference canonical single-source dir '../slicer-schema/wit'"
    );
}

/// Verifies that the canonical layer-world WIT references the `layer-collection-builder`
/// resource — both in the world's `use ir-handles.{...}` import block and in the
/// `run-path-optimization` export signature — and that the canonical disk
/// `wit/deps/ir-types.wit` declares the resource with the canonical `set-entity-order`
/// signature (packet 32 — TASK-152g).
///
/// Under single-source (packet 72), the macro sources its layer-world WIT via
/// include_str! from the canonical world-layer.wit — drift is caught by checking the
/// canonical files directly and confirming the macro's include_str! path is correct.
#[test]
fn macro_embeds_layer_collection_builder_resource() {
    let root = workspace_root();

    // The canonical layer-world WIT must expose layer-collection-builder.
    let world_layer =
        fs::read_to_string(root.join("crates/slicer-schema/wit/deps/world-layer/world-layer.wit"))
            .expect("read canonical world-layer.wit");
    assert!(
        world_layer.contains("layer-collection-builder,"),
        "canonical world-layer.wit must import 'layer-collection-builder' in the world's `use ir-handles.{{...}}` block"
    );
    assert!(
        world_layer.contains("collection: layer-collection-builder"),
        "canonical world-layer.wit must wire 'collection: layer-collection-builder' into run-path-optimization"
    );

    // The actual resource declaration lives in the canonical ir-types.wit.
    let ir_types = fs::read_to_string(root.join("crates/slicer-schema/wit/deps/ir-types.wit"))
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

    // Drift guard: macro must source its layer-world WIT from canonical single-source.
    let lib_rs = macro_lib_rs_content();
    assert!(
        lib_rs.contains(
            r#"include_str!("../../slicer-schema/wit/deps/world-layer/world-layer.wit")"#
        ),
        "macro must source layer-world WIT from canonical single-source via include_str!"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Prepass segmentation signature surface
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that the canonical prepass world uses mesh-object-view (not raw
/// object-id) for the run-mesh-segmentation export.
/// Redirected to single-source path: wit/deps/world-prepass/world-prepass.wit.
#[test]
fn prepass_world_uses_mesh_object_view() {
    let path =
        workspace_root().join("crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit");
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

// ─────────────────────────────────────────────────────────────────────────────
// Seam-related layer-world members
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that perimeter-region-view exposes resolved-seam as a read member.
#[test]
fn perimeter_region_view_has_resolved_seam() {
    let path = workspace_root().join("crates/slicer-schema/wit/deps/ir-types.wit");
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
    let path = workspace_root().join("crates/slicer-schema/wit/deps/ir-types.wit");
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

/// Returns the content of `crates/slicer-runtime/src/wit_host.rs`.
/// Uses `std::fs::read_to_string` at test runtime.
fn host_wit_host_rs_content() -> String {
    let path = workspace_root().join("crates/slicer-runtime/src/wit_host.rs");
    fs::read_to_string(&path).expect("read host wit_host.rs for inline WIT verification")
}
