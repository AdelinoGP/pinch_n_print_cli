//! AC-N2: real `[[region_split]]` manifest ingestion rejection.
//!
//! Every fixture is rendered as escaped, bounded TOML into a fresh
//! `tempfile::TempDir` beside a placeholder companion `.wasm` and loaded through
//! the real `slicer_scheduler::load_module_from_paths`. TOML is data: nothing
//! here is shell input, and every document is length-checked before it is
//! written.
//!
//! Builders construct already-loaded aggregation stimuli only. They are never a
//! substitute for this ingestion coverage: `aggregate_region_splits` is not a
//! manifest validator (the companion test pins that boundary explicitly).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use slicer_ir::SemVer;
use slicer_scheduler::region_split::aggregate_region_splits;
use slicer_scheduler::{
    load_module_from_paths, LoadError, LoadErrorKind, LoadedModule, LoadedModuleBuilder,
    RegionSplitDeclaration, RegionSplitValueType,
};
use tempfile::TempDir;

const MAX_RENDERED_MANIFEST_BYTES: usize = 8 * 1024;
const MAX_RENDERED_SPLITS: usize = 8;
const MAX_RENDERED_SEMANTIC_BYTES: usize = 64;

/// One rendered `[[region_split]]` entry. `priority` is an `i64` so the
/// out-of-u32 and negative-integer TOML cases can be rendered verbatim.
struct Split<'a> {
    semantic: &'a str,
    priority: i64,
    value_type: &'a str,
}

/// Escape one TOML basic-string value.
fn toml_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

/// Render a minimal valid manifest with bounded, escaped TOML.
///
/// `paint_only` is the raw top-level TOML token (`Some("true")`,
/// `Some("\"yes\"")`); it is emitted before the first table header so it stays a
/// top-level key.
fn manifest_toml(module_id: &str, paint_only: Option<&str>, splits: &[Split<'_>]) -> String {
    assert!(
        splits.len() <= MAX_RENDERED_SPLITS,
        "bounded TOML renderer: {} splits exceeds {MAX_RENDERED_SPLITS}",
        splits.len()
    );
    for split in splits {
        assert!(
            split.semantic.len() <= MAX_RENDERED_SEMANTIC_BYTES,
            "bounded TOML renderer: semantic {:?} exceeds {MAX_RENDERED_SEMANTIC_BYTES} bytes",
            split.semantic
        );
    }

    let mut text = String::new();
    if let Some(raw) = paint_only {
        text.push_str(&format!("paint_only = {raw}\n"));
    }
    text.push_str(&format!(
        "[module]\n\
         id = {id}\n\
         version = \"0.1.0\"\n\
         display-name = \"Region Split Ingestion Control\"\n\
         description = \"Rendered AC-N2 control fixture\"\n\
         author = \"test\"\n\
         license = \"MIT\"\n\
         \n\
         [stage]\n\
         id = \"Layer::Infill\"\n\
         \n\
         [ir-access]\n\
         reads = []\n\
         writes = []\n\
         \n\
         [claims]\n\
         holds = []\n\
         requires = []\n\
         \n\
         [compatibility]\n\
         incompatible-with = []\n\
         requires = []\n\
         min-host-version = \"0.1.0\"\n\
         min-ir-schema = \"1.0.0\"\n\
         max-ir-schema = \"5.0.0\"\n\
         \n\
         [hints]\n\
         layer-parallel-safe = true\n",
        id = toml_string(module_id),
    ));
    for split in splits {
        text.push_str(&format!(
            "\n[[region_split]]\n\
             semantic = {semantic}\n\
             priority = {priority}\n\
             value_type = {value_type}\n",
            semantic = toml_string(split.semantic),
            priority = split.priority,
            value_type = toml_string(split.value_type),
        ));
    }

    assert!(
        text.len() <= MAX_RENDERED_MANIFEST_BYTES,
        "bounded TOML renderer: {}-byte manifest exceeds {MAX_RENDERED_MANIFEST_BYTES}",
        text.len()
    );
    text
}

/// Write a rendered manifest and its placeholder companion into a TempDir.
fn stage(stem: &str, text: &str) -> (TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::Builder::new()
        .prefix(&format!("pnp-region-split-{stem}-"))
        .tempdir()
        .expect("create temporary manifest directory");
    let manifest = tmp.path().join(format!("{stem}.toml"));
    let wasm = tmp.path().join(format!("{stem}.wasm"));
    fs::write(&manifest, text.as_bytes()).expect("write rendered manifest TOML");
    fs::write(&wasm, b"placeholder wasm").expect("write placeholder companion wasm");
    (tmp, manifest, wasm)
}

/// Drive real ingestion and keep the TempDir alive for the caller.
fn load(stem: &str, text: &str) -> (TempDir, Result<LoadedModule, LoadError>) {
    let (tmp, manifest, wasm) = stage(stem, text);
    let result = load_module_from_paths(&manifest, &wasm);
    (tmp, result)
}

fn load_error(stem: &str, text: &str) -> (TempDir, LoadError) {
    let (tmp, result) = load(stem, text);
    let error = result.expect_err("the invalid manifest must be rejected by real ingestion");
    (tmp, error)
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

/// AC-N2: every specified ingestion violation, plus its valid boundary
/// neighbours, through the real `load_module_from_paths`.
#[test]
fn scheduler_manifest_region_split_rejections() {
    // ── DuplicateRegionSplitSemantic ────────────────────────────────────────
    let duplicate = manifest_toml(
        "com.test.duplicate",
        None,
        &[
            Split {
                semantic: "com.example.duplicate",
                priority: 1000,
                value_type: "flag",
            },
            Split {
                semantic: "com.example.duplicate",
                priority: 1200,
                value_type: "tool_index",
            },
        ],
    );
    let (_duplicate_dir, error) = load_error("duplicate", &duplicate);
    match &error.kind {
        LoadErrorKind::DuplicateRegionSplitSemantic {
            semantic,
            first_line,
            second_line,
        } => {
            assert_eq!(semantic, "com.example.duplicate");
            assert_eq!(*first_line, None);
            assert_eq!(*second_line, None);
        }
        other => panic!("expected DuplicateRegionSplitSemantic, got: {other:?}"),
    }
    assert_eq!(error.field.as_deref(), Some("region_split"));

    // ── ScalarValueTypeNotAllowedInRegionSplit ──────────────────────────────
    let scalar = manifest_toml(
        "com.test.scalar",
        None,
        &[Split {
            semantic: "com.example.scalar",
            priority: 1000,
            value_type: "scalar",
        }],
    );
    let (_scalar_dir, error) = load_error("scalar", &scalar);
    match &error.kind {
        LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit { semantic } => {
            assert_eq!(semantic, "com.example.scalar");
        }
        other => panic!("expected ScalarValueTypeNotAllowedInRegionSplit, got: {other:?}"),
    }
    assert_eq!(error.field.as_deref(), Some("region_split[0].value_type"));

    // ── CommunityPriorityBelowFloor at 999, acceptance at 1000 ──────────────
    let below_floor = manifest_toml(
        "com.test.below-floor",
        None,
        &[Split {
            semantic: "com.example.community",
            priority: 999,
            value_type: "flag",
        }],
    );
    let (_below_floor_dir, error) = load_error("below-floor", &below_floor);
    match &error.kind {
        LoadErrorKind::CommunityPriorityBelowFloor {
            semantic,
            given_priority,
            floor,
        } => {
            assert_eq!(semantic, "com.example.community");
            assert_eq!(*given_priority, 999);
            assert_eq!(*floor, 1000);
        }
        other => panic!("expected CommunityPriorityBelowFloor, got: {other:?}"),
    }
    assert_eq!(error.field.as_deref(), Some("region_split"));

    let at_floor = manifest_toml(
        "com.test.at-floor",
        None,
        &[Split {
            semantic: "com.example.community",
            priority: 1000,
            value_type: "flag",
        }],
    );
    let (_at_floor_dir, result) = load("at-floor", &at_floor);
    let at_floor_module = result.expect("community priority equal to the floor must load");
    assert_eq!(
        at_floor_module.region_splits().to_vec(),
        vec![RegionSplitDeclaration {
            semantic: "com.example.community".to_owned(),
            priority: 1000,
            value_type: RegionSplitValueType::Flag,
        }],
        "the floor neighbour must expose its actual declaration"
    );
    assert!(!at_floor_module.paint_only());
    assert!(
        at_floor_module.region_split_semantics().is_empty(),
        "a declaration without paint_only must leave dispatch transparent"
    );

    // ── CorePriorityMismatch for both core semantics ────────────────────────
    let material_mismatch = manifest_toml(
        "com.test.material-mismatch",
        None,
        &[Split {
            semantic: "material",
            priority: 101,
            value_type: "tool_index",
        }],
    );
    let (_material_dir, error) = load_error("material-mismatch", &material_mismatch);
    match &error.kind {
        LoadErrorKind::CorePriorityMismatch {
            semantic,
            given_priority,
            expected_priority,
        } => {
            assert_eq!(semantic, "material");
            assert_eq!(*given_priority, 101);
            assert_eq!(*expected_priority, 100);
        }
        other => panic!("expected CorePriorityMismatch for material, got: {other:?}"),
    }
    assert_eq!(error.field.as_deref(), Some("region_split"));

    let fuzzy_mismatch = manifest_toml(
        "com.test.fuzzy-mismatch",
        None,
        &[Split {
            semantic: "fuzzy_skin",
            priority: 100,
            value_type: "flag",
        }],
    );
    let (_fuzzy_dir, error) = load_error("fuzzy-mismatch", &fuzzy_mismatch);
    match &error.kind {
        LoadErrorKind::CorePriorityMismatch {
            semantic,
            given_priority,
            expected_priority,
        } => {
            assert_eq!(semantic, "fuzzy_skin");
            assert_eq!(*given_priority, 100);
            assert_eq!(*expected_priority, 200);
        }
        other => panic!("expected CorePriorityMismatch for fuzzy_skin, got: {other:?}"),
    }

    // ── Core priorities exact: acceptance and dispatch set ──────────────────
    let core_exact = manifest_toml(
        "com.test.core-exact",
        Some("true"),
        &[
            Split {
                semantic: "material",
                priority: 100,
                value_type: "tool_index",
            },
            Split {
                semantic: "fuzzy_skin",
                priority: 200,
                value_type: "flag",
            },
        ],
    );
    let (_core_exact_dir, result) = load("core-exact", &core_exact);
    let core_exact_module = result.expect("exact core priorities must load");
    assert!(core_exact_module.paint_only());
    assert_eq!(
        core_exact_module.region_splits().to_vec(),
        vec![
            RegionSplitDeclaration {
                semantic: "material".to_owned(),
                priority: 100,
                value_type: RegionSplitValueType::ToolIndex,
            },
            RegionSplitDeclaration {
                semantic: "fuzzy_skin".to_owned(),
                priority: 200,
                value_type: RegionSplitValueType::Flag,
            },
        ],
        "the core neighbours must expose their actual declarations"
    );
    let core_semantics: BTreeSet<&str> = core_exact_module
        .region_split_semantics()
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(
        core_semantics,
        BTreeSet::from(["fuzzy_skin", "material"]),
        "a paint_only core module's dispatch set must be exactly its declared semantics"
    );

    // ── PaintOnlyWithoutRegionSplit ─────────────────────────────────────────
    let paint_only_without_split = manifest_toml("com.test.paint-only-missing", Some("true"), &[]);
    let (_paint_only_dir, error) = load_error("paint-only-missing", &paint_only_without_split);
    assert_eq!(
        error.kind,
        LoadErrorKind::PaintOnlyWithoutRegionSplit,
        "paint_only without a declaration must be rejected"
    );
    assert_eq!(error.field.as_deref(), Some("paint_only"));

    // ── Schema for a wrong paint_only type ──────────────────────────────────
    let paint_only_wrong_type = manifest_toml("com.test.paint-only-wrong", Some("\"yes\""), &[]);
    let (_paint_only_wrong_dir, error) = load_error("paint-only-wrong", &paint_only_wrong_type);
    assert_eq!(error.kind, LoadErrorKind::Schema);
    assert_eq!(error.field.as_deref(), Some("paint_only"));

    // ── Schema for an out-of-u32 priority ───────────────────────────────────
    let out_of_u32 = manifest_toml(
        "com.test.priority-too-large",
        None,
        &[Split {
            semantic: "com.example.too-large",
            priority: 4_294_967_296,
            value_type: "flag",
        }],
    );
    let (_too_large_dir, error) = load_error("priority-too-large", &out_of_u32);
    assert_eq!(error.kind, LoadErrorKind::Schema);
    assert_eq!(error.field.as_deref(), Some("region_split[0]"));
    assert!(
        error
            .message
            .contains("failed to deserialize region_split entry"),
        "the Schema error must name the deserialization failure: {}",
        error.message
    );

    // ── Schema for a negative integer priority ──────────────────────────────
    let negative = manifest_toml(
        "com.test.priority-negative",
        None,
        &[Split {
            semantic: "com.example.negative",
            priority: -1,
            value_type: "flag",
        }],
    );
    let (_negative_dir, error) = load_error("priority-negative", &negative);
    assert_eq!(error.kind, LoadErrorKind::Schema);
    assert_eq!(error.field.as_deref(), Some("region_split[0]"));

    // ── Valid neighbours expose all three legal value types ─────────────────
    let valid = manifest_toml(
        "com.test.valid-neighbours",
        Some("true"),
        &[
            Split {
                semantic: "com.example.flag",
                priority: 1000,
                value_type: "flag",
            },
            Split {
                semantic: "com.example.tool",
                priority: 1200,
                value_type: "tool_index",
            },
            Split {
                semantic: "com.example.string",
                priority: 1400,
                value_type: "custom_string",
            },
        ],
    );
    let (_valid_dir, result) = load("valid-neighbours", &valid);
    let valid_module = result.expect("legal community declarations must load");
    assert!(valid_module.paint_only());
    assert_eq!(
        valid_module.region_splits().to_vec(),
        vec![
            RegionSplitDeclaration {
                semantic: "com.example.flag".to_owned(),
                priority: 1000,
                value_type: RegionSplitValueType::Flag,
            },
            RegionSplitDeclaration {
                semantic: "com.example.tool".to_owned(),
                priority: 1200,
                value_type: RegionSplitValueType::ToolIndex,
            },
            RegionSplitDeclaration {
                semantic: "com.example.string".to_owned(),
                priority: 1400,
                value_type: RegionSplitValueType::CustomString,
            },
        ],
        "valid neighbours must expose all three legal value types"
    );
    let valid_semantics: BTreeSet<&str> = valid_module
        .region_split_semantics()
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(
        valid_semantics,
        BTreeSet::from(["com.example.flag", "com.example.tool", "com.example.string",])
    );
}

/// `aggregate_region_splits` is not a manifest validator.
///
/// A `LoadedModuleBuilder` bypasses ingestion by construction, so these
/// already-loaded stimuli must aggregate without rejection or validation
/// diagnostics. This pins the boundary AC-N2 depends on: builder fixtures alone
/// cannot satisfy the rejection coverage above.
#[test]
fn scheduler_aggregate_does_not_validate_builder_declarations() {
    let below_floor = LoadedModuleBuilder::new(
        "com.test.builder-below-floor",
        semver(0, 1, 0),
        "Layer::Infill",
        String::new(),
        PathBuf::from("fixtures/builder-below-floor.wasm"),
    )
    .region_splits(vec![RegionSplitDeclaration {
        semantic: "com.example.builder".to_owned(),
        priority: 250,
        value_type: RegionSplitValueType::Flag,
    }])
    .build();

    let mut diagnostics = Vec::new();
    let aggregate = aggregate_region_splits(&[below_floor], &mut diagnostics);
    let builder_entry = aggregate
        .get("com.example.builder")
        .expect("aggregation accepts an already-built below-floor declaration");
    assert_eq!(builder_entry.priority, 250);
    assert_eq!(builder_entry.value_type, RegionSplitValueType::Flag);
    assert!(
        diagnostics.is_empty(),
        "aggregation is not a validator; it must not emit ingestion diagnostics: {diagnostics:?}"
    );

    // Builder-created core mismatch and duplicate same-contract declaration.
    let invalid_core_and_duplicate = LoadedModuleBuilder::new(
        "com.test.builder-invalid",
        semver(0, 1, 0),
        "Layer::Infill",
        String::new(),
        PathBuf::from("fixtures/builder-invalid.wasm"),
    )
    .region_splits(vec![
        RegionSplitDeclaration {
            semantic: "material".to_owned(),
            priority: 101,
            value_type: RegionSplitValueType::ToolIndex,
        },
        RegionSplitDeclaration {
            semantic: "com.example.duplicate".to_owned(),
            priority: 1000,
            value_type: RegionSplitValueType::Flag,
        },
        RegionSplitDeclaration {
            semantic: "com.example.duplicate".to_owned(),
            priority: 1000,
            value_type: RegionSplitValueType::Flag,
        },
    ])
    .build();

    let mut diagnostics = Vec::new();
    let aggregate = aggregate_region_splits(&[invalid_core_and_duplicate], &mut diagnostics);
    let material = aggregate
        .get("material")
        .expect("aggregation accepts an already-built core priority mismatch");
    assert_eq!(material.priority, 101);
    let duplicate = aggregate
        .get("com.example.duplicate")
        .expect("aggregation accepts a builder-created duplicate declaration");
    assert_eq!(
        duplicate.declaring_modules,
        vec!["com.test.builder-invalid".to_owned()],
        "the same module declaring one semantic twice must not duplicate the declarer"
    );
    assert!(
        diagnostics.is_empty(),
        "same-contract duplication is merged silently, not validated: {diagnostics:?}"
    );
}
