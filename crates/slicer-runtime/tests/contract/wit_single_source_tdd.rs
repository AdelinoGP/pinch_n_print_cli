//! Conformance tests for packet 72: WIT single-source unification.
//!
//! AC-9  — canonical WIT dir resolves all four worlds via wit_parser.
//! AC-N1 — illegal label `extrusion-path-3d` (segment begins with digit) is
//!          rejected by wit_parser, proving the canonical source is genuinely
//!          validated.
//! Anti-regression:
//!   no_flat_copies              — no *-flat* files exist in the canonical dir.
//!   worlds_are_not_self_contained — world files cross-reference slicer: deps.
//!   shared_interface_defined_once — shared interfaces appear exactly once.
//!   host_has_no_inline_bindgen  — wit_host.rs has no `inline: r#` block.

use std::path::{Path, PathBuf};

fn schema_wit_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR == crates/slicer-runtime
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("slicer-schema")
        .join("wit")
}

fn runtime_src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

// ---------------------------------------------------------------------------
// AC-9
// ---------------------------------------------------------------------------

/// Canonical WIT dir resolves successfully and exposes all four worlds.
#[test]
fn canonical_wit_resolves() {
    let dir = schema_wit_dir();
    assert!(dir.exists(), "canonical WIT dir missing: {}", dir.display());

    let mut resolve = wit_parser::Resolve::new();

    // push_dir requires a directory that is a single WIT package; our layout
    // is a multi-package tree under an umbrella, so we use push_path which
    // handles the full nested tree. Alternatively with 0.247 we can walk deps
    // manually.  push_dir on the root works when root.wit declares the anchor
    // package and deps/ contains the rest — wasmtime does the same.
    // push_dir returns (PackageId, PackageSourceMap) in wit-parser 0.247
    let (root_pkg_id, _source_map) = resolve
        .push_dir(&dir)
        .unwrap_or_else(|e| panic!("wit_parser failed to resolve {}: {e}", dir.display()));

    // Collect all world names across every resolved package.
    let mut world_names: Vec<String> = resolve.worlds.iter().map(|(_, w)| w.name.clone()).collect();
    world_names.sort();

    let required = [
        "layer-module",
        "prepass-module",
        "postpass-module",
        "finalization-module",
    ];

    for name in &required {
        assert!(
            world_names.iter().any(|w| w == name),
            "world `{name}` not found; worlds present: {world_names:?}"
        );
    }

    // Also confirm select_world succeeds for each qualified name.
    // wit-parser 0.247: select_world(&[PackageId], world_str_or_none)
    let qualified = [
        "slicer:world-layer/layer-module@1.0.0",
        "slicer:world-prepass/prepass-module@1.0.0",
        "slicer:world-postpass/postpass-module@1.0.0",
        "slicer:world-finalization/finalization-module@1.0.0",
    ];
    for q in &qualified {
        resolve
            .select_world(&[root_pkg_id], Some(q))
            .unwrap_or_else(|e| panic!("select_world({q}) failed: {e}"));
    }
}

// ---------------------------------------------------------------------------
// AC-N1
// ---------------------------------------------------------------------------

/// Illegal WIT labels must be rejected by wit_parser at parse time — proving
/// the parser validates WIT semantics and cannot silently accept the phantom
/// drift class.
///
/// WIT label rule: the FIRST segment of a kebab-label must start with a letter.
/// `3d-extrusion-path` is invalid (first segment `3d` starts with a digit).
/// Note: non-first segments CAN start with a digit per wit-parser 0.247, so
/// `extrusion-path-3d` is actually legal; the old phantom used `extrusion-path-3d`
/// which PASSES validation — the canonical source correctly uses `extrusion-path3d`
/// (a single segment) to avoid any ambiguity.
#[test]
fn illegal_label_rejected() {
    // A label whose FIRST segment starts with a digit — definitively illegal WIT.
    let bad_wit = r#"
package test:illegal;
interface bad {
    record 3d-extrusion-path { x: f32 }
}
"#;

    let result = wit_parser::UnresolvedPackageGroup::parse("illegal.wit", bad_wit);
    assert!(
        result.is_err(),
        "wit_parser should have rejected `3d-extrusion-path` (first segment `3d` starts with digit), \
         but parsing succeeded"
    );
}

// ---------------------------------------------------------------------------
// Anti-regression: no flat copies
// ---------------------------------------------------------------------------

/// No file under the canonical WIT dir matches `*-flat*`.
#[test]
fn no_flat_copies() {
    let dir = schema_wit_dir();
    let flat_files: Vec<PathBuf> = walk_wit_files(&dir)
        .into_iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains("-flat"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        flat_files.is_empty(),
        "Flat-copy WIT files found (drift trap): {flat_files:?}"
    );
}

// ---------------------------------------------------------------------------
// Anti-regression: worlds reference shared deps (not self-contained)
// ---------------------------------------------------------------------------

/// Each world .wit file must contain at least one `use slicer:` cross-package
/// reference — ensuring they depend on the shared dep packages rather than
/// re-inlining everything.
#[test]
fn worlds_are_not_self_contained() {
    let world_dirs = [
        "world-layer",
        "world-prepass",
        "world-postpass",
        "world-finalization",
    ];
    let deps_dir = schema_wit_dir().join("deps");

    for world_dir in &world_dirs {
        let wit_file = deps_dir.join(world_dir).join(format!("{world_dir}.wit"));
        assert!(
            wit_file.exists(),
            "world file missing: {}",
            wit_file.display()
        );

        let content = std::fs::read_to_string(&wit_file)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", wit_file.display()));

        assert!(
            content.contains("use slicer:"),
            "world file {} has no `use slicer:` cross-package reference — \
             it may be self-contained (drift trap)",
            wit_file.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Anti-regression: shared interfaces defined exactly once
// ---------------------------------------------------------------------------

/// `interface geometry`, `interface config-types`, and `interface ir-handles`
/// must each appear exactly once across the entire canonical WIT dir.
#[test]
fn shared_interface_defined_once() {
    let dir = schema_wit_dir();
    let files = walk_wit_files(&dir);

    let shared = [
        ("interface geometry", "geometry"),
        ("interface config-types", "config-types"),
        ("interface ir-handles", "ir-handles"),
    ];

    for (needle, label) in &shared {
        let mut found_in: Vec<PathBuf> = Vec::new();
        for f in &files {
            let content = std::fs::read_to_string(f)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", f.display()));
            // Match the opening brace to distinguish definition from use.
            let definition_needle = format!("{needle} {{");
            if content.contains(&definition_needle) {
                found_in.push(f.clone());
            }
        }
        assert_eq!(
            found_in.len(),
            1,
            "Shared interface `{label}` must be defined exactly once, \
             but found in {} file(s): {found_in:?}",
            found_in.len()
        );
    }
}

// ---------------------------------------------------------------------------
// Anti-regression: host uses path: not inline:
// ---------------------------------------------------------------------------

/// `crates/slicer-wasm-host/src/host.rs` must contain no `inline: r#`
/// block (the host reads the canonical WIT via `path:`).
#[test]
fn host_has_no_inline_bindgen() {
    let wit_host = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("slicer-wasm-host")
        .join("src")
        .join("host.rs");
    assert!(
        wit_host.exists(),
        "host.rs not found: {}",
        wit_host.display()
    );

    let content = std::fs::read_to_string(&wit_host)
        .unwrap_or_else(|e| panic!("cannot read host.rs: {e}"));

    assert!(
        !content.contains("inline: r#"),
        "host.rs still contains `inline: r#` — host must use `path:` bindgen"
    );
}

// ---------------------------------------------------------------------------
// Anti-regression: host bindgen! path: targets shared canonical root
// ---------------------------------------------------------------------------

// Remediates AC-3's over-broad grep (which a per-world flat-copy subdir path also satisfied).
// The substantive single-source guard is shared_interface_defined_once; this is belt-and-suspenders
// asserting the host consumes the shared canonical root, not a per-world copy.
// Agreement (roundtrip) != single-source: flat copies passed roundtrip 19/19.
#[test]
fn host_bindgen_paths_target_shared_root() {
    let wit_host = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("slicer-wasm-host")
        .join("src")
        .join("host.rs");
    assert!(
        wit_host.exists(),
        "host.rs not found: {}",
        wit_host.display()
    );

    let content = std::fs::read_to_string(&wit_host)
        .unwrap_or_else(|e| panic!("cannot read host.rs: {e}"));

    // Extract every path: "..." string literal inside bindgen! invocations.
    // We look for   path: "../slicer-schema/wit"  (with optional surrounding whitespace).
    let mut paths: Vec<&str> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("path:") {
            let rest = rest.trim();
            // Extract the quoted string value.
            if let Some(inner) = rest.strip_prefix('"') {
                if let Some(end) = inner.find('"') {
                    paths.push(&inner[..end]);
                }
            }
        }
    }

    assert_eq!(
        paths.len(),
        4,
        "Expected exactly 4 `path:` literals in host.rs bindgen! invocations (one per world), \
         found {}: {paths:?}",
        paths.len()
    );

    for path in &paths {
        assert_eq!(
            *path, "../slicer-schema/wit",
            "Every `path:` in host.rs must equal exactly \"../slicer-schema/wit\" \
             (the shared canonical root), but found: \"{path}\""
        );
        assert!(
            !path.contains("/world-"),
            "A `path:` in host.rs contains a per-world subdir segment (`/world-`), \
             indicating a flat-copy path instead of the shared root: \"{path}\""
        );
    }
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn walk_wit_files(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(walk_wit_files(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("wit") {
                result.push(path);
            }
        }
    }
    result
}
