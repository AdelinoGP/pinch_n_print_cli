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

/// Verifies that every stage package has the canonical package declaration and
/// that the macro sources that exact file from the canonical tree.
#[test]
fn macro_stage_package_names_are_canonical() {
    let root = workspace_root();
    let canonical_packages = [
        "layer-slice-postprocess",
        "layer-perimeters",
        "layer-perimeters-postprocess",
        "layer-infill",
        "layer-infill-postprocess",
        "layer-support",
        "layer-support-postprocess",
        "layer-path-optimization",
        "prepass-mesh-analysis",
        "prepass-layer-planning",
        "prepass-seam-planning",
        "prepass-support-geometry",
        "finalization-layer-finalization",
        "postpass-gcode-postprocess",
        "postpass-text-postprocess",
    ];
    let lib_rs = macro_lib_rs_content();
    let normalized: String = lib_rs.chars().filter(|c| !c.is_whitespace()).collect();
    for slug in canonical_packages {
        let path = root.join(format!("crates/slicer-schema/wit/deps/{slug}/{slug}.wit"));
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("read canonical stage package {slug}.wit"));
        let declaration = content
            .lines()
            .find_map(|line| line.trim().strip_prefix("package "))
            .and_then(|rest| rest.trim().strip_suffix(';'))
            .unwrap_or_else(|| panic!("canonical stage package {slug}.wit must declare a package"));
        let (name, version) = declaration.split_once('@').unwrap_or_else(|| {
            panic!("canonical stage package {slug}.wit must be versioned: {declaration}")
        });
        assert_eq!(
            name,
            format!("slicer:{slug}"),
            "canonical {slug}.wit must declare its stage package name"
        );
        let parts: Vec<&str> = version.split('.').collect();
        assert!(
            parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok()),
            "canonical {slug}.wit package version must be three-part semver (found '{version}')"
        );
        let expected = format!(r#"include_str!("../../slicer-schema/wit/deps/{slug}/{slug}.wit")"#);
        let expected_normalized: String = expected.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            normalized.contains(&expected_normalized),
            "macro must source stage package {slug} from canonical single-source via include_str!"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Host WIT source verification
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that the host's bindgen! blocks reference all 15 canonical stage
/// worlds through the shared WIT path.
#[test]
fn host_bindgen_uses_canonical_stage_package_names() {
    let wit_host_rs = host_wit_host_rs_content();

    // Single-source: host reads canonical dir via `path:`, not inline WIT.
    // Assert the shared canonical WIT directory is referenced.
    assert!(
        wit_host_rs.contains(r#"path: "../slicer-schema/wit""#),
        "host bindgen! must use canonical path '../slicer-schema/wit'"
    );

    let canonical_stage_refs = [
        r#"world: "slicer:layer-slice-postprocess/slice-postprocess-module""#,
        r#"world: "slicer:layer-perimeters/perimeters-module""#,
        r#"world: "slicer:layer-perimeters-postprocess/perimeters-postprocess-module""#,
        r#"world: "slicer:layer-infill/infill-module""#,
        r#"world: "slicer:layer-infill-postprocess/infill-postprocess-module""#,
        r#"world: "slicer:layer-support/support-module""#,
        r#"world: "slicer:layer-support-postprocess/support-postprocess-module""#,
        r#"world: "slicer:layer-path-optimization/path-optimization-module""#,
        r#"world: "slicer:prepass-mesh-analysis/mesh-analysis-module""#,
        r#"world: "slicer:prepass-layer-planning/layer-planning-module""#,
        r#"world: "slicer:prepass-seam-planning/seam-planning-module""#,
        r#"world: "slicer:prepass-support-geometry/support-geometry-module""#,
        r#"world: "slicer:finalization-layer-finalization/layer-finalization-module""#,
        r#"world: "slicer:postpass-gcode-postprocess/gcode-postprocess-module""#,
        r#"world: "slicer:postpass-text-postprocess/text-postprocess-module""#,
    ];
    assert_eq!(
        canonical_stage_refs.len(),
        15,
        "the host guard must enumerate all 15 stage worlds"
    );
    for canonical in canonical_stage_refs {
        assert!(
            wit_host_rs.contains(canonical),
            "host bindgen! must reference canonical stage world '{canonical}'"
        );
    }

    // A tier-world package reference would bypass the per-stage contract.
    let wrong = "slicer:world-";
    assert!(
        !wit_host_rs.contains(wrong),
        "host bindgen! must not contain an unqualified tier package prefix '{wrong}'"
    );
}

/// Verifies that the `with:` block keys in host `wit_host.rs` use the canonical
/// interface paths now that resources live in shared dep packages.
/// The host maps `"slicer:config/config-types.config-view"` (shared dep
/// package) rather than a tier-world-versioned form.
#[test]
fn host_bindgen_with_keys_use_canonical_stage_names() {
    let wit_host_rs = host_wit_host_rs_content();

    // Single-source: config-view is now a shared dep, so the with: key is the dep
    // package form, not a world-versioned form. Assert the canonical key is present
    // in each bindgen! block (one occurrence per stage is sufficient).
    let canonical_key = r#""slicer:config/config-types.config-view""#;
    assert!(
        wit_host_rs.contains(canonical_key),
        "host bindgen with: block should contain canonical shared-dep key '{canonical_key}'"
    );

    // The old (wrong) tier-world-versioned key form must not appear.
    let disallowed_keys = [
        r#""slicer:world-/config-types/config-view""#,
        r#""slicer:world-/config-types@1.0.0.config-view""#,
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

/// Verifies that all 15 canonical stage-package WIT files exist on disk.
// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_stage_package_files_exist_on_disk() {
    let root = workspace_root();
    let stage_slugs = [
        "layer-slice-postprocess",
        "layer-perimeters",
        "layer-perimeters-postprocess",
        "layer-infill",
        "layer-infill-postprocess",
        "layer-support",
        "layer-support-postprocess",
        "layer-path-optimization",
        "prepass-mesh-analysis",
        "prepass-layer-planning",
        "prepass-seam-planning",
        "prepass-support-geometry",
        "finalization-layer-finalization",
        "postpass-gcode-postprocess",
        "postpass-text-postprocess",
    ];
    assert_eq!(stage_slugs.len(), 15);
    for stage_slug in stage_slugs {
        let path = root.join(format!(
            "crates/slicer-schema/wit/deps/{stage_slug}/{stage_slug}.wit"
        ));
        assert!(
            path.exists(),
            "canonical stage WIT file '{stage_slug}/{stage_slug}.wit' must exist on disk at {:?}",
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
/// Per packet 163: the postpass tier is now a per-stage package at
/// `wit/deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit`.
/// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_world_postpass_has_payload_command_input() {
    let path = workspace_root().join(
        "crates/slicer-schema/wit/deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit",
    );
    let content = fs::read_to_string(&path).expect("read canonical postpass-gcode-postprocess.wit");
    assert!(
        content.contains("variant gcode-command"),
        "canonical postpass-gcode-postprocess.wit must define payload-bearing 'variant gcode-command'"
    );
    assert!(
        content.contains("unretract"),
        "canonical postpass-gcode-postprocess.wit must carry an 'unretract' command case"
    );
}

/// Verifies that the canonical finalization world widened layer-collection-view
/// with ordered-entity and z-hop reads.
/// Per packet 163: the finalization tier is now a per-stage package at
/// `wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit`.
/// Guards against canonical-file edits (single-source, post-packet-72); producer divergence is architecturally impossible.
#[test]
fn canonical_world_finalization_has_entity_and_zhop_reads() {
    let path = workspace_root().join(
        "crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit",
    );
    let content =
        fs::read_to_string(&path).expect("read canonical finalization-layer-finalization.wit");
    assert!(
        content.contains("ordered-entities"),
        "canonical finalization-layer-finalization.wit must expose 'ordered-entities'"
    );
    assert!(
        content.contains("z-hops"),
        "canonical finalization-layer-finalization.wit must expose 'z-hops'"
    );
}

/// Verifies that the canonical postpass/finalization WIT files carry the widened
/// surfaces, and that the macro's include_str! calls reference those canonical files.
/// Under single-source, "macro embedded WIT" means: the macro reads from canonical
/// disk files via include_str!, so drift is caught by checking the canonical files.
/// Per packet 163: postpass + finalization live in their own per-stage package dirs.
#[test]
fn macro_embedded_wit_tracks_boundary_widening() {
    let root = workspace_root();
    // Widened postpass surface — must be in the canonical postpass world.
    let postpass = fs::read_to_string(root.join(
        "crates/slicer-schema/wit/deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit",
    ))
    .expect("read canonical postpass-gcode-postprocess.wit");
    assert!(
        postpass.contains("push-unretract"),
        "canonical postpass-gcode-postprocess.wit must contain 'push-unretract' after postpass widening"
    );
    assert!(
        postpass.contains("variant gcode-command"),
        "canonical postpass-gcode-postprocess.wit must define payload-bearing 'variant gcode-command'"
    );

    // Widened finalization surface — must be in the canonical finalization world.
    let finalization = fs::read_to_string(
        root.join("crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit"),
    )
    .expect("read canonical finalization-layer-finalization.wit");
    assert!(
        finalization.contains("ordered-entities"),
        "canonical finalization-layer-finalization.wit must expose 'ordered-entities'"
    );
    assert!(
        finalization.contains("z-hops"),
        "canonical finalization-layer-finalization.wit must expose 'z-hops'"
    );

    // Drift guard: confirm the macro sources its postpass/finalization WIT from the
    // canonical single-source files (not inline strings that could silently diverge).
    let lib_rs = macro_lib_rs_content();
    // `include_str!` may be written multi-line — collapse whitespace before
    // matching, otherwise the macro's prettier-formatted multi-line calls
    // silently fail this guard.
    let normalized: String = lib_rs.chars().filter(|c| !c.is_whitespace()).collect();
    let expect_postpass_gcode: String = r#"include_str!("../../slicer-schema/wit/deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit")"#
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let expect_finalization: String = r#"include_str!("../../slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit")"#
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert!(
        normalized.contains(&expect_postpass_gcode),
        "macro must source postpass gcode WIT from canonical single-source via include_str!"
    );
    assert!(
        normalized.contains(&expect_finalization),
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
    // Per packet 163: postpass + finalization live in per-stage package dirs.
    let postpass = fs::read_to_string(root.join(
        "crates/slicer-schema/wit/deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit",
    ))
    .expect("read canonical postpass-gcode-postprocess.wit");
    assert!(
        postpass.contains("push-unretract"),
        "canonical postpass-gcode-postprocess.wit must contain 'push-unretract' after postpass widening"
    );
    assert!(
        postpass.contains("variant gcode-command"),
        "canonical postpass-gcode-postprocess.wit must define payload-bearing 'variant gcode-command'"
    );

    let finalization = fs::read_to_string(
        root.join("crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit"),
    )
    .expect("read canonical finalization-layer-finalization.wit");
    assert!(
        finalization.contains("ordered-entities"),
        "canonical finalization-layer-finalization.wit must expose 'ordered-entities'"
    );
    assert!(
        finalization.contains("z-hops"),
        "canonical finalization-layer-finalization.wit must expose 'z-hops'"
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
/// include_str! from the canonical stage package — drift is caught by checking the
/// canonical files directly and confirming the macro's include_str! path is correct.
#[test]
fn macro_embeds_layer_collection_builder_resource() {
    let root = workspace_root();

    // The canonical path-optimization package must expose layer-collection-builder.
    let path_optimization =
        fs::read_to_string(root.join(
            "crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit",
        ))
        .expect("read canonical layer-path-optimization.wit");
    assert!(
        path_optimization.contains("layer-collection-builder"),
        "canonical layer-path-optimization.wit must import 'layer-collection-builder'"
    );
    assert!(
        path_optimization.contains("collection: layer-collection-builder"),
        "canonical layer-path-optimization.wit must wire 'collection: layer-collection-builder' into run"
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

    // Drift guard: macro must source the stage package from canonical single-source.
    let lib_rs = macro_lib_rs_content();
    let normalized: String = lib_rs.chars().filter(|c| !c.is_whitespace()).collect();
    let expected: String = r#"include_str!("../../slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit")"#
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert!(
        normalized.contains(&expected),
        "macro must source layer-path-optimization WIT from canonical single-source via include_str!"
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
// Layer::InfillPostProcess contract types (packet 130, ADR-0028 §Amendment)
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that the canonical ir-types.wit declares the `prior-infill-region`
/// record with all five members (packet 130).
#[test]
fn canonical_ir_types_has_prior_infill_region_record() {
    let path = workspace_root().join("crates/slicer-schema/wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    assert!(
        content.contains("record prior-infill-region"),
        "canonical ir-types.wit must declare 'record prior-infill-region'"
    );
    for member in [
        "object-id: object-id",
        "region-id: region-id",
        "sparse-infill: list<extrusion-path3d>",
        "solid-infill: list<extrusion-path3d>",
        "ironing: list<extrusion-path3d>",
    ] {
        assert!(
            content.contains(member),
            "prior-infill-region must carry member '{member}'"
        );
    }
}

/// Verifies that perimeter-region-view exposes the six ADR-0028 §Amendment
/// enrichment members added for `Layer::InfillPostProcess` (packet 130).
#[test]
fn perimeter_region_view_has_infill_postprocess_enrichment_members() {
    let path = workspace_root().join("crates/slicer-schema/wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    // The four partitioned fill polygon reads plus tool-index and
    // wall-source-region-id must appear inside the perimeter-region-view
    // resource block.
    let view_block = content
        .split("resource perimeter-region-view")
        .nth(1)
        .expect("ir-types.wit declares resource perimeter-region-view")
        .split('}')
        .next()
        .expect("perimeter-region-view resource block is closed");
    for member in [
        "sparse-infill-area: func() -> list<ex-polygon>",
        "top-solid-fill: func() -> list<ex-polygon>",
        "bottom-solid-fill: func() -> list<ex-polygon>",
        "bridge-areas: func() -> list<ex-polygon>",
        "tool-index: func() -> u32",
        "wall-source-region-id: func() -> option<region-id>",
    ] {
        assert!(
            view_block.contains(member),
            "perimeter-region-view must expose member '{member}'"
        );
    }
}

/// Verifies that the canonical infill-postprocess package threads the
/// prior-infill parameter through run-infill-postprocess and imports the
/// prior-infill-region record.
#[test]
fn canonical_infill_postprocess_takes_prior_infill() {
    let path = workspace_root().join(
        "crates/slicer-schema/wit/deps/layer-infill-postprocess/layer-infill-postprocess.wit",
    );
    let content = fs::read_to_string(&path).expect("read canonical layer-infill-postprocess.wit");
    assert!(
        content.contains("package slicer:layer-infill-postprocess@1.0.0;"),
        "layer-infill-postprocess must use the canonical package version"
    );
    assert!(
        content.contains("prior-infill-region,"),
        "layer-infill-postprocess must import prior-infill-region from ir-handles"
    );
    assert!(
        content
            .split_whitespace()
            .collect::<String>()
            .contains("run:func(layer-index:layer-idx,regions:list<perimeter-region-view>,prior-infill:list<prior-infill-region>,output:infill-output-builder,config:config-view)->result<_,module-error>;"),
        "run-infill-postprocess must take the prior-infill parameter with the canonical signature"
    );
}

/// Verifies that the infill package receives the same paint view shape as the
/// other paint-aware layer stages.
#[test]
fn canonical_infill_takes_paint_view() {
    let path = workspace_root().join("crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit");
    let content = fs::read_to_string(&path).expect("read canonical layer-infill.wit");
    assert!(
        content.contains("package slicer:layer-infill@1.0.0;"),
        "layer-infill must use the canonical package version"
    );
    assert!(
        content
            .split_whitespace()
            .collect::<String>()
            .contains("run:func(layer-index:layer-idx,regions:list<slice-region-view>,paint:paint-region-layer-view,output:infill-output-builder,config:config-view)->result<_,module-error>;"),
        "run-infill must take the canonical paint-region-layer-view parameter"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Anti-regression: the world version lives in exactly one place
// ─────────────────────────────────────────────────────────────────────────────

/// Recursively collect files with one of `exts`, skipping build/VCS dirs.
fn collect_files(dir: &std::path::Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    // The repository may contain developer-created linked worktrees. They are
    // separate checkouts, not workspace source, and can retain old WIT names.
    let skip = ["target", ".git", "node_modules", "worktrees"];
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !skip.contains(&name.as_ref()) {
                collect_files(&path, exts, out);
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if exts.contains(&ext) {
                out.push(path);
            }
        }
    }
}

/// A versioned tier-world identifier (`slicer:world-x@1.2.3`) must not appear
/// in any `.rs` or `.toml` file in the workspace. Stage-package qualification
/// is intentionally checked by the binding tests instead.
///
/// This is the regression guard for the pre-package contract churn: versions
/// must not be copied into host-side identifiers that do not select a WIT
/// package.
///
/// It has no effect because it cannot: our worlds export bare freestanding
/// funcs, and a bare extern name carries no semver suffix (component-model
/// WIT.md — `<semversuffix>` is a production of `<interfacename>`). The version
/// is erased from every guest binary at compile time, so nothing can ever check
/// a declared version against the artifact it claims to describe.
///
/// If this test fails, remove the version from the offending tier-world
/// identifier rather than updating this guard.
#[test]
fn no_versioned_world_identifiers_outside_canonical_wit() {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_files(&root, &["rs", "toml"], &mut files);
    assert!(
        files.len() > 100,
        "sanity: the walk should find the workspace's sources, found {}",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for path in &files {
        // This file is the one sanctioned place that pins the version: the
        // assertions above deliberately spell out each world's `package` line
        // so that a bump stays a conscious act. Bumping a world should touch
        // the .wit and this file, and nothing else.
        if path
            .file_name()
            .is_some_and(|n| n == "wit_drift_detection_tdd.rs")
        {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            // Prose may discuss versions; only code and manifest values matter,
            // because only those force a lockstep edit on a bump.
            let is_comment = trimmed.starts_with("//") || trimmed.starts_with('#');
            if is_comment || !regex_lite_versioned_world(line) {
                continue;
            }
            let rel = path.strip_prefix(&root).unwrap_or(path);
            offenders.push(format!("{}:{}: {}", rel.display(), idx + 1, line.trim()));
        }
    }

    assert!(
        offenders.is_empty(),
        "tier-world versions must not escape the canonical WIT package declarations; \
         found {} versioned reference(s):\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

#[test]
fn no_lifecycle_exports_anywhere() {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_files(&root, &["rs", "wit"], &mut files);
    assert!(
        files.len() > 100,
        "sanity: the walk should find the workspace's sources, found {}",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for path in &files {
        if path
            .file_name()
            .is_some_and(|n| n == "wit_drift_detection_tdd.rs")
        {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        for (idx, line) in content.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line.contains("on-print-start")
                || line.contains("on-print-end")
                || line.contains("on_print_start")
                || line.contains("on_print_end")
            {
                let rel = path.strip_prefix(&root).unwrap_or(path);
                offenders.push(format!("{}:{}", rel.display(), idx + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "lifecycle exports must not appear outside this guard; found {} offender(s):\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// True if `line` contains a versioned tier-world reference in either shape:
///   - `slicer:world-x@2.0.0` — bare package form.
///   - `slicer:world-x/interface@1.1.0` — package-qualified path form.
///
/// The second shape matters: it is how the host and every test-guest name their
/// world, and an earlier version of this predicate stopped at the `/` and missed
/// it entirely — leaving 6 files still churning on a bump while this test
/// reported green. A guard with a hole is worse than no guard.
///
/// Hand-rolled because slicer-runtime has no regex dev-dependency and this
/// single predicate does not justify adding one.
fn regex_lite_versioned_world(line: &str) -> bool {
    let mut rest = line;
    while let Some(pos) = rest.find("slicer:world-") {
        let after = &rest[pos + "slicer:world-".len()..];
        // Consume the world name and any `/interface` path segments, then
        // require `@` followed by a digit.
        let path_end = after
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '/')
            .unwrap_or(after.len());
        if after[path_end..].starts_with('@')
            && after[path_end + 1..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        {
            return true;
        }
        rest = &after[path_end..];
    }
    false
}

/// Returns the content of `crates/slicer-macros/src/lib.rs`.
/// Uses `std::fs::read_to_string` at test runtime.
fn macro_lib_rs_content() -> String {
    let path = workspace_root().join("crates/slicer-macros/src/lib.rs");
    fs::read_to_string(&path).expect("read macro lib.rs for WIT include verification")
}

/// Returns the content of `crates/slicer-wasm-host/src/host.rs`.
/// Uses `std::fs::read_to_string` at test runtime.
fn host_wit_host_rs_content() -> String {
    let path = workspace_root().join("crates/slicer-wasm-host/src/host.rs");
    fs::read_to_string(&path).expect("read host host.rs for inline WIT verification")
}

// ─────────────────────────────────────────────────────────────────────────────
// Packet 137: `lightning-tree-segments` view (AC-N2)
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies the canonical `ir-types.wit` exposes the new
/// `lightning-tree-segments` method on the `paint-region-layer-view` resource
/// (packet 137, AC-N2). The host `HostPaintRegionLayerView` impl in
/// `slicer-wasm-host/src/host.rs` would fail to compile if this drifted.
#[test]
fn paint_region_layer_view_has_lightning_tree_segments_method() {
    let path = workspace_root().join("crates/slicer-schema/wit/deps/ir-types.wit");
    let content = fs::read_to_string(&path).expect("read canonical ir-types.wit");
    let view_block = content
        .split("resource paint-region-layer-view")
        .nth(1)
        .expect("ir-types.wit declares resource paint-region-layer-view")
        .split('}')
        .next()
        .expect("paint-region-layer-view resource block is closed");
    assert!(
        view_block
            .contains("lightning-tree-segments: func(object-id: object-id, region-id: region-id)"),
        "paint-region-layer-view must expose 'lightning-tree-segments' method (packet 137, AC-N2)"
    );
    assert!(
        view_block.contains("-> list<list<point3-with-width>>"),
        "lightning-tree-segments must return list<list<point3-with-width>> (mirrors support-plan-segments)"
    );
}

/// Verifies the layer-infill stage package retains the load-bearing major
/// version used by the per-stage bindgen contract.
#[test]
fn layer_infill_package_version_is_load_bearing() {
    let path = workspace_root().join("crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit");
    let content = fs::read_to_string(&path).expect("read canonical layer-infill.wit");
    assert!(
        content.contains("package slicer:layer-infill@1.0.0;"),
        "layer-infill must remain at package version 1.0.0"
    );
}

/// Per packet 163 (AC-1b): the `@1.0.0` package version is **load-bearing**
/// for the per-stage `bindgen!` mechanism — wasmtime's
/// `alternate_lookup_key` only produces a major-track key for `major >= 1`
/// (a `0.x` package yields a minor-track `@0.1` key, so every minor bump
/// would break compatibility). A future contributor "tidying" a stage
/// package to `0.x` would silently disable every compatibility claim.
///
/// This guard mirrors `canonical_world_files_exist_on_disk`: walk
/// `crates/slicer-schema/wit/deps/*/` (every package directory), parse
/// the `package slicer:<name>@<major>.<minor>.<patch>;` header, and
/// assert `major >= 1` for every per-stage package.
#[test]
fn every_stage_package_major_is_at_least_one() {
    let root = workspace_root();
    let deps_dir = root.join("crates/slicer-schema/wit/deps");
    let entries = fs::read_dir(&deps_dir).expect("read wit/deps directory");
    let mut offenders: Vec<String> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        // We only inspect the per-stage package directories (the ones
        // whose name matches the `wit_dir` of a migrated stage). The flat
        // shared dep files (`common.wit`, `config.wit`, etc.) are
        // unversioned on purpose (packet 163 §"The unit is the package,
        // not the interface"); skipping them is by design.
        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !path.is_dir() {
            continue;
        }
        let wit_path = path.join(format!("{dir_name}.wit"));
        if !wit_path.is_file() {
            continue;
        }
        // Only check the **versioned** per-stage packages. The unversioned dep
        // packages (`common.wit`, etc.) live one level up at
        // `wit/deps/<file>.wit`, not in their own dir — this loop
        // already skips them by requiring `<dir>/<dir>.wit`.
        let content = match fs::read_to_string(&wit_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut lines = content.lines();
        let package_line = match lines.find(|l| l.trim_start().starts_with("package ")) {
            Some(l) => l,
            None => continue,
        };
        // Parse `package slicer:<name>@<major>.<minor>.<patch>;`
        let version_str = match package_line.split('@').nth(1) {
            Some(v) => v.trim_end_matches(';').trim(),
            None => {
                offenders.push(format!("{dir_name}: no @ in package line"));
                continue;
            }
        };
        let major: u32 = match version_str.split('.').next().and_then(|s| s.parse().ok()) {
            Some(n) => n,
            None => {
                offenders.push(format!("{dir_name}: non-numeric major in '{version_str}'"));
                continue;
            }
        };
        if major < 1 {
            offenders.push(format!("{dir_name}@{}", version_str));
        }
    }
    assert!(
        offenders.is_empty(),
        "every per-stage WIT package must declare major >= 1 (load-bearing for \
         wasmtime's alternate_lookup_key). Offenders: {offenders:?}",
    );
}
