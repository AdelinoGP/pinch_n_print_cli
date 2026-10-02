//! Packet config-scope-resolution_06 (resolved-config-view) contract tests.
//!
//! - **AC-1** `production_binding_is_registry_complete_and_resolved`: binding a
//!   module via `bind_module_config_view(module, &ResolvedConfig)` yields a
//!   view holding every declared key with a registry default or an authored
//!   value (with its effective value), no undeclared key, and the
//!   `support_type` / `support_family` keys for `support-family:` claimants.
//!   Neither `bind_module_config_view` nor `build_live_execution_plan` accepts
//!   a `HashMap` source — both signatures are pinned by resolved-only
//!   construction (compile-time; see `signature_sentinel`).
//! - **AC-N2** `resolved_binding_preserves_module_encapsulation`: an
//!   undeclared key is absent from the bound view even when another module or
//!   the host registry declares it, and `ConfigView::require_float` on it
//!   returns `Err` naming the key.
//! - **AC-10** `unknown_key_absent_from_binding_and_config_block_map`: an
//!   ingestion outcome that dropped `skrit_loops` resolves through the live
//!   registry, and the dropped key is absent from the bound `ConfigView` and
//!   from the `ConfigSchemaRegistry::config_block_map` projection.
//! - **AC-3** trio over the live guest sources under
//!   `modules/core-modules/<guest>/src/**/*.rs`:
//!   - `guest_config_literal_fallback_census_is_zero` — counts every
//!     config-read chain ending in `.unwrap_or(<number|bool|string literal>)`;
//!     RED at this step with the residual count (baseline 87), expected to
//!     reach zero in Step 5.
//!   - `guest_fallback_detector_is_calibrated` — the classifier flags one
//!     verbatim baseline snippet per Step-1 chain shape and never flags
//!     `unwrap_or_else`, sort comparators, `unwrap_or(<const|expr>)`, or
//!     `#[cfg(test)]` code.
//!   - `guest_config_reads_are_declared` — every string-literal key read
//!     through a ConfigView getter or a guest wrapper is declared in the
//!     guest's own manifest `[config.schema]` or is a `support_type` /
//!     `support_family` read by a `support-family:` claimant; RED at this step
//!     with the residual undeclared reads (the eleven listed in design.md).

#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use slicer_config::{
    assemble_registry, resolve_scope_stack, ConfigIngestor, ConfigSchemaRegistry, ExpansionContext,
    HostChannels, IngestionWarning, ModuleDeclaration, ResolutionTarget,
};
use slicer_ir::slice_ir::ConfigReadError;
use slicer_ir::{ConfigValue, ConfigView, ResolvedConfig, SemVer};
use slicer_runtime::{
    bind_module_config_view, build_live_execution_plan, build_wasm_instance_pool,
    load_modules_from_roots, ConfigFieldEntry, ConfigSchema, LiveModuleBinding, LoadedModule,
    LoadedModuleBuilder, SortedStageModules, WasmArtifactMetadata,
};
use slicer_scheduler::config_resolution::{resolve_config, ConfigBoundsIndex};

// ────────────────────────────────────────────────────────────────────────────
// AC-1 / AC-N2 shared fixtures
// ────────────────────────────────────────────────────────────────────────────

fn sem() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

/// Build a `LoadedModule` declaring `keys` as `(key, field_type)` with the
/// given `(key, default_wire)` defaults, plus the given claims.
fn module_with_schema(
    id: &str,
    keys: &[(&str, &str)],
    defaults: &[(&str, &str)],
    claims: &[&str],
) -> LoadedModule {
    let mut entries = BTreeMap::new();
    for (key, field_type) in keys {
        let default = defaults
            .iter()
            .find(|(k, _)| *k == *key)
            .map(|(_, d)| (*d).to_string());
        entries.insert(
            (*key).to_string(),
            ConfigFieldEntry {
                field_type: (*field_type).to_string(),
                default,
                ..Default::default()
            },
        );
    }
    LoadedModuleBuilder::new(
        id,
        sem(),
        "PrePass::MeshAnalysis",
        slicer_schema::TIER_PREPASS,
        PathBuf::from("fixtures/mod.wasm"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(sem())
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .claims(claims.iter().map(|c| (*c).to_string()).collect())
    .config_schema(ConfigSchema { entries })
    .build()
}

/// Resolve `source` against `module`'s declaration through the real
/// registry-typed scope stack (ingestion → seeding → Phase-B expansion).
fn resolve_for(module: &LoadedModule, source: HashMap<String, ConfigValue>) -> ResolvedConfig {
    let bounds = ConfigBoundsIndex::from_modules(std::iter::once(module));
    resolve_config(
        &source,
        &bounds,
        &ResolutionTarget::default(),
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
    )
    .expect("test config must resolve")
}

/// Live-plan binding mirroring production's `run.rs` plumbing.
fn live_binding(module: &LoadedModule) -> LiveModuleBinding {
    let pool = Arc::new(
        build_wasm_instance_pool(
            module.id(),
            module.stage(),
            module.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build pool"),
    );
    LiveModuleBinding {
        module: module.clone(),
        instance_pool: pool,
        wasm_component: None,
        native_entry: None,
    }
}

/// Compile-time signature sentinel for AC-1: both binding entry points take
/// the resolved config, not a `HashMap` source. If a signature regressed to a
/// source map, this function would not compile (the `.get` / typed accessors
/// below would type-error) — the resolved-only construction is the assertion.
#[allow(dead_code)]
fn signature_sentinel(module: &LoadedModule, resolved: &ResolvedConfig) -> Arc<ConfigView> {
    let view = bind_module_config_view(module, resolved);
    let mut diagnostics = Vec::new();
    let plan = build_live_execution_plan(
        vec![SortedStageModules {
            stage_id: module.stage().to_string(),
            module_ids: vec![module.id().to_string()],
        }],
        vec![live_binding(module)],
        resolved,
        Arc::new(Vec::new()),
        Arc::new(HashMap::new()),
        &mut diagnostics,
    )
    .expect("live plan must build from a resolved config");
    let _ = plan;
    view
}

#[test]
fn production_binding_is_registry_complete_and_resolved() {
    // The module declares `alpha` (seeded default 0.5), `beta` (seeded default
    // 2.0) and `gamma` (seeded default 7). The authored source overrides
    // `alpha` and carries `extra_secret`, an undeclared key.
    let module = module_with_schema(
        "com.example.registry-complete",
        &[
            ("alpha", "float"),
            ("beta", "float"),
            ("gamma", "int"),
            ("mode", "string"),
        ],
        &[
            ("alpha", "0.5"),
            ("beta", "2.0"),
            ("gamma", "7"),
            ("mode", "auto"),
        ],
        &[],
    );
    let mut source = HashMap::new();
    source.insert("alpha".to_string(), ConfigValue::Float(1.25));
    source.insert(
        "extra_secret".to_string(),
        ConfigValue::String("do-not-leak".to_string()),
    );

    let resolved = resolve_for(&module, source);
    let view = signature_sentinel(&module, &resolved);

    // Every declared key is present with its effective value: authored wins
    // over the registry default; absent keys carry the seeded registry
    // default.
    assert_eq!(view.get_float("alpha"), Some(1.25));
    assert_eq!(view.get_float("beta"), Some(2.0));
    assert_eq!(view.get_int("gamma"), Some(7));
    assert_eq!(view.get_string("mode"), Some("auto"));
    // No undeclared key surfaces — the view is exactly the declared set.
    assert_eq!(
        view.keys(),
        vec![
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
            "mode".to_string()
        ]
    );
    assert!(!view.contains_key("extra_secret"));
    assert!(view.get("extra_secret").is_none());
    assert_eq!(view.len(), 4);

    // The plan-build path binds through the same resolved config and yields
    // the same registry-complete view.
    let mut diagnostics = Vec::new();
    let plan = build_live_execution_plan(
        vec![SortedStageModules {
            stage_id: module.stage().to_string(),
            module_ids: vec![module.id().to_string()],
        }],
        vec![live_binding(&module)],
        &resolved,
        Arc::new(Vec::new()),
        Arc::new(HashMap::new()),
        &mut diagnostics,
    )
    .expect("live plan must build");
    let compiled = &plan.prepass_stages[0].modules[0];
    assert_eq!(compiled.config_view().get_float("alpha"), Some(1.25));
    assert_eq!(compiled.config_view().get_float("beta"), Some(2.0));
    assert_eq!(compiled.config_view().get_int("gamma"), Some(7));
    assert!(!compiled.config_view().contains_key("extra_secret"));
}

#[test]
fn support_family_claimant_carries_support_keys() {
    // A `support-family:` claimant receives `support_type` / `support_family`
    // from the resolved map even though its manifest does not repeat the
    // declaration, per `bind_module_config_view` (AC-1). The tuning keys are
    // deliberately NOT `ResolvedConfig` typed fields (e.g. `line_width` is a
    // typed `cli` row whose zero default auto-expands to 1.125 × nozzle), so
    // the registry-default assertions below exercise pure module-declaration
    // seeding.
    let support = module_with_schema(
        "com.example.support-family-claimant",
        &[("branch_width", "float"), ("stem_density", "float")],
        &[("branch_width", "0.4"), ("stem_density", "0.2")],
        &["support-generator", "support-family:tree"],
    );
    let mut source = HashMap::new();
    source.insert(
        "support_type".to_string(),
        ConfigValue::String("tree(auto)".to_string()),
    );
    source.insert(
        "support_family".to_string(),
        ConfigValue::String("tree".to_string()),
    );
    let resolved = resolve_for(&support, source);
    let view = bind_module_config_view(&support, &resolved);

    assert_eq!(view.get_string("support_type"), Some("tree(auto)"));
    assert_eq!(view.get_string("support_family"), Some("tree"));
    assert_eq!(view.get_float("branch_width"), Some(0.4));

    // A sibling module with no support claim must NOT see the support keys.
    let plain = module_with_schema(
        "com.example.no-claim",
        &[("branch_width", "float")],
        &[("branch_width", "0.4")],
        &[],
    );
    let plain_view = bind_module_config_view(&plain, &resolved);
    assert!(plain_view.contains_key("branch_width"));
    assert!(!plain_view.contains_key("support_type"));
    assert!(!plain_view.contains_key("support_family"));
}

#[test]
fn resolved_binding_preserves_module_encapsulation() {
    // Module A declares only `density`. Module B declares `alpha`; the host
    // registry additionally owns `support_family`. Both are present in the
    // resolved config, but module A's view must not expose either.
    let a = module_with_schema(
        "com.example.encapsulated-a",
        &[("density", "float")],
        &[],
        &[],
    );
    let b = module_with_schema(
        "com.example.encapsulated-b",
        &[("alpha", "float")],
        &[("alpha", "0.5")],
        &[],
    );
    let mut source = HashMap::new();
    source.insert("density".to_string(), ConfigValue::Float(0.25));
    source.insert(
        "support_family".to_string(),
        ConfigValue::String("traditional".to_string()),
    );

    // Resolve against BOTH declarations so `alpha` has a registry default and
    // `support_family` is a registered host key.
    let bounds = ConfigBoundsIndex::from_modules([&a, &b]);
    let resolved = resolve_config(
        &source,
        &bounds,
        &ResolutionTarget::default(),
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
    )
    .expect("test config must resolve");

    // Sanity: the resolved map really does carry the other module's key.
    assert!(resolved.to_config_map().contains_key("alpha"));
    assert!(resolved.to_config_map().contains_key("support_family"));

    let view = bind_module_config_view(&a, &resolved);
    assert_eq!(view.keys(), vec!["density".to_string()]);
    assert!(!view.contains_key("alpha"));
    assert!(!view.contains_key("support_family"));
    assert!(view.get("alpha").is_none());
    assert!(view.get_float("alpha").is_none());

    // `require_float` on the undeclared key fails and names the key.
    let err = view.require_float("alpha").expect_err("must fail");
    assert_eq!(err.key, "alpha");
    assert!(matches!(
        err,
        ConfigReadError { ref key, .. } if key == "alpha"
    ));
    let err = view.require_float("support_family").expect_err("must fail");
    assert_eq!(err.key, "support_family");

    // The declared key still resolves through the required read.
    assert_eq!(view.require_float("density"), Ok(0.25));
}

// ────────────────────────────────────────────────────────────────────────────
// AC-10 fixture and test
// ────────────────────────────────────────────────────────────────────────────

/// The live core-module registry, assembled exactly as `run_slice` assembles
/// it: every loaded module manifest plus the live host channels (the same
/// derivation `crates/slicer-runtime/tests/e2e/resolved_config_view_no_drop_tdd.rs`
/// performs; duplicated here because each test binary is standalone).
fn live_registry() -> (Vec<LoadedModule>, ConfigSchemaRegistry) {
    let modules: Vec<LoadedModule> =
        load_modules_from_roots(std::slice::from_ref(&core_modules_dir()))
            .unwrap_or_else(|error| panic!("load core module schemas failed: {error:?}"))
            .modules;
    let declarations: Vec<ModuleDeclaration> = modules
        .iter()
        .map(|module| ModuleDeclaration {
            module_id: module.id().to_owned(),
            schema: module.config_schema().clone(),
            claim_exclusive_group: None,
        })
        .collect();
    let registry: ConfigSchemaRegistry =
        assemble_registry(&declarations, &HostChannels::from_live())
            .unwrap_or_else(|error| panic!("assemble live registry failed: {error}"))
            .registry;
    (modules, registry)
}

/// AC-10: the runtime path after AC-6's ingestion outcome — resolution, a
/// module bound via `bind_module_config_view`, and the
/// `ConfigSchemaRegistry::config_block_map` projection — never sees the
/// dropped `skrit_loops` key. The retained near-miss `skirt_loops` (declared
/// by the live skirt-brim manifest) stays bound and projected, so the absence
/// is key-specific rather than a wholesale wipe.
#[test]
fn unknown_key_absent_from_binding_and_config_block_map() {
    let (modules, registry) = live_registry();
    let skirt_brim = modules
        .iter()
        .find(|module| module.id() == "com.core.skirt-brim")
        .expect("skirt-brim must be among the live core modules");
    assert!(
        registry.entry("skirt_loops").is_some(),
        "fixture assumption: the live registry declares skirt_loops"
    );

    let mut source = HashMap::new();
    source.insert(
        "skrit_loops".to_string(),
        ConfigValue::String("1".to_string()),
    );
    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_flat(&source)
        .expect("flat authored config must decode");
    let outcome = ingestor.finish();
    assert_eq!(
        outcome.warnings.len(),
        1,
        "exactly the AC-6 UnrecognizedKey warning is expected, got {:?}",
        outcome.warnings
    );
    match &outcome.warnings[0] {
        IngestionWarning::UnrecognizedKey { wire_key, key, .. } => {
            assert_eq!(wire_key, "skrit_loops");
            assert_eq!(key, "skrit_loops");
        }
        other => panic!("expected UnrecognizedKey for skrit_loops, got {other:?}"),
    }

    // Resolution of the ingestion outcome. This is the assertion that fails
    // under retention: a surviving `skrit_loops` would surface in the
    // resolved config.
    let resolved = resolve_scope_stack(
        &registry,
        &outcome.scoped,
        &ResolutionTarget::default(),
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
    )
    .expect("resolution with a dropped unknown key must succeed");
    assert!(
        !resolved.to_config_map().contains_key("skrit_loops"),
        "the dropped key must not reach the resolved config"
    );

    // The bound ConfigView carries the module's declared `skirt_loops` but
    // never the dropped misspelling.
    let view = bind_module_config_view(&skirt_brim, &resolved);
    assert!(
        view.contains_key("skirt_loops"),
        "the declared near-miss key must still bind into the view"
    );
    assert!(
        !view.contains_key("skrit_loops"),
        "the dropped key must not reach the bound ConfigView"
    );

    // The CONFIG_BLOCK projection is registry-driven: it emits the declared
    // near-miss key's effective value and never the dropped key.
    let block = registry.config_block_map(&resolved);
    assert!(
        block.contains_key("skirt_loops"),
        "the declared near-miss key must be projected into the CONFIG_BLOCK map"
    );
    assert!(
        !block.contains_key("skrit_loops"),
        "the dropped key must never be projected into the CONFIG_BLOCK map"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// AC-3 guest-source scanner
// ────────────────────────────────────────────────────────────────────────────

/// The 13 census guests with their AC-3 baseline counts (Step-1 inventory).
const CENSUS_GUESTS: &[(&str, usize)] = &[
    ("arachne-perimeters", 29),
    ("classic-perimeters", 27),
    ("rectilinear-infill", 10),
    ("overhang-classifier-default", 4),
    ("tree-support", 3),
    ("traditional-support", 3),
    ("infill-linker", 3),
    ("tree-support-planner", 2),
    ("layer-planner-default", 0),
    ("gyroid-infill", 2),
    ("lightning-infill", 2),
    ("wave-overhangs", 1),
    ("traditional-support-planner", 1),
];

/// ConfigView getters a config-read chain can start at (AC-3).
const GETTERS: &[&str] = &[
    "get",
    "get_bool",
    "get_int",
    "get_float",
    "get_string",
    "get_abs_value",
];

/// Connector methods a chain may pass through between the read and the
/// terminal `.unwrap_or` (AC-3).
const CONNECTORS: &[&str] = &["map", "filter", "or_else", "and_then"];

/// Guest wrappers over ConfigView reads taking a literal key: `(name, key
/// argument position)`.
const READ_WRAPPERS: &[(&str, usize)] = &[
    ("config_float", 1),  // infill-linker
    ("cfg_float", 1),     // wave-overhangs
    ("cfg_bool", 1),      // wave-overhangs
    ("cfg_str", 1),       // wave-overhangs
    ("cfg_density", 1),   // wave-overhangs
    ("cfg_u32", 1),       // wave-overhangs
    ("speed", 1),         // overhang-classifier-default
    ("resolve_float", 1), // slicer_sdk::config_resolution helper
    ("width_value", 0),   // rectilinear-infill closure
    ("speed_value", 0),   // rectilinear-infill closure
];

// Mask values: a byte's syntactic role in the scanned source.
const CODE: u8 = 0;
const STRING: u8 = 1;
const COMMENT: u8 = 2;
const CFG_TEST: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SiteKind {
    Direct,
    Map,
    Filter,
    OrElse,
    AndThen,
}

#[derive(Debug)]
struct Site {
    guest: String,
    file: String,
    line: usize,
    kind: SiteKind,
}

/// Build the syntactic mask for one source file: strings, line comments,
/// nested block comments are marked; `#[cfg(test)]` items are blanked so
/// test-only chains never count (AC-3: "does not flag #[cfg(test)] code").
fn mask_source(text: &str) -> (Vec<u8>, Vec<u8>) {
    let text = text.as_bytes();
    let mut mask = vec![CODE; text.len()];
    let mut i = 0usize;
    let mut in_string = false;
    while i < text.len() {
        let b = text[i];
        if in_string {
            mask[i] = STRING;
            if b == b'\\' && i + 1 < text.len() {
                mask[i + 1] = STRING;
                i += 2;
                continue;
            }
            if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if b == b'"' {
            in_string = true;
            mask[i] = STRING;
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < text.len() {
            if text[i + 1] == b'/' {
                mask[i] = COMMENT;
                mask[i + 1] = COMMENT;
                i += 2;
                while i < text.len() && text[i] != b'\n' {
                    mask[i] = COMMENT;
                    i += 1;
                }
                continue;
            }
            if text[i + 1] == b'*' {
                mask[i] = COMMENT;
                mask[i + 1] = COMMENT;
                i += 2;
                let mut block_depth = 1usize;
                while i < text.len() && block_depth > 0 {
                    if text[i] == b'/' && i + 1 < text.len() && text[i + 1] == b'*' {
                        block_depth += 1;
                        mask[i] = COMMENT;
                        mask[i + 1] = COMMENT;
                        i += 2;
                    } else if text[i] == b'*' && i + 1 < text.len() && text[i + 1] == b'/' {
                        block_depth -= 1;
                        mask[i] = COMMENT;
                        mask[i + 1] = COMMENT;
                        i += 2;
                    } else {
                        mask[i] = COMMENT;
                        i += 1;
                    }
                }
                continue;
            }
        }
        i += 1;
    }
    (text.to_vec(), mask)
}

/// Blank every `#[cfg(test)]`-annotated item (attribute through its balanced
/// closing brace) so test-only source never contributes to the census or the
/// declared-read set. Strings inside the item are skipped during brace
/// matching.
fn blank_cfg_test_regions(text: &[u8], mask: &mut [u8]) {
    let mut i = 0usize;
    'outer: while i + 12 <= text.len() {
        if mask[i] == CODE && &text[i..i + 12] == b"#[cfg(test)]" {
            // Find the item's opening brace: the first code `{` after the
            // attribute (the attribute annotates `mod`/`fn`/`impl` items).
            let mut j = i + 12;
            while j < text.len() && text[j] != b'{' {
                j += 1;
            }
            if j >= text.len() {
                i += 1;
                continue;
            }
            // Brace-match to the item's end, skipping strings/comments.
            let mut depth = 0usize;
            let mut k = j;
            let mut end = None;
            while k < text.len() {
                if mask[k] == CODE || mask[k] == CFG_TEST {
                    match text[k] {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(k);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                k += 1;
            }
            if let Some(end) = end {
                for b in &mut mask[i..=end] {
                    *b = CFG_TEST;
                }
                i = end + 1;
                continue 'outer;
            }
        }
        i += 1;
    }
}

/// Index of the previous code (non-whitespace, non-masked) byte before
/// `from`, or `None`.
fn prev_code(text: &[u8], mask: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i > 0 {
        i -= 1;
        if mask[i] != CODE {
            continue;
        }
        let b = text[i];
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
            continue;
        }
        return Some(i);
    }
    None
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Position of the first code byte at or after `from` where `pat` occurs.
fn find_code(text: &[u8], mask: &[u8], pat: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + pat.len() <= text.len() {
        if mask[i] == CODE && &text[i..i + pat.len()] == pat {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Whether `pat` occurs entirely on code bytes within `range`.
fn code_contains(text: &[u8], mask: &[u8], range: std::ops::Range<usize>, pat: &[u8]) -> bool {
    find_code(text, mask, pat, range.start).is_some_and(|i| i + pat.len() <= range.end)
}

/// Find the `(` matching the `)` at `close`, scanning backward over balanced
/// parens while skipping strings/comments. Returns the open-paren index.
fn match_open_paren_backward(text: &[u8], mask: &[u8], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = close + 1;
    while i > 0 {
        i -= 1;
        if mask[i] != CODE {
            continue;
        }
        match text[i] {
            b')' => depth += 1,
            b'(' => {
                if depth == 1 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Top-level argument ranges of the call whose `(` is at `open`, split on
/// commas at depth 1 (strings/comments skipped).
fn call_args(text: &[u8], mask: &[u8], open: usize) -> Vec<std::ops::Range<usize>> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut start = open + 1;
    let mut i = open + 1;
    while i < text.len() {
        if mask[i] == CODE {
            match text[i] {
                b'(' => depth += 1,
                b')' => {
                    if depth == 0 {
                        if start < i {
                            args.push(start..i);
                        }
                        return args;
                    }
                    depth -= 1;
                }
                b',' if depth == 0 => {
                    args.push(start..i);
                    start = i + 1;
                }
                _ => {}
            }
        }
        i += 1;
    }
    args
}

/// Whether `arg` (trimmed) is a number/bool/string literal (AC-3's site
/// definition: `.unwrap_or(<number|bool|string literal>)`).
fn is_literal_arg(arg: &[u8]) -> bool {
    let arg = trim_ascii(arg);
    if arg == b"true" || arg == b"false" {
        return true;
    }
    if arg.len() >= 2 && arg[0] == b'"' && arg[arg.len() - 1] == b'"' {
        return true;
    }
    std::str::from_utf8(arg)
        .map(|s| s.parse::<f64>().is_ok())
        .unwrap_or(false)
}

fn trim_ascii(mut b: &[u8]) -> &[u8] {
    while let [first, rest @ ..] = b {
        if first.is_ascii_whitespace() {
            b = rest;
        } else {
            break;
        }
    }
    while let [rest @ .., last] = b {
        if last.is_ascii_whitespace() {
            b = rest;
        } else {
            break;
        }
    }
    b
}

/// Identifier run ending immediately before `before` (a byte position whose
/// char is an ident char), with `(start, end_exclusive)`.
fn ident_before(text: &[u8], before: usize) -> Option<(usize, usize)> {
    if before == 0 || !is_ident_byte(text[before]) {
        return None;
    }
    let mut start = before;
    while start > 0 && is_ident_byte(text[start - 1]) {
        start -= 1;
    }
    Some((start, before + 1))
}

/// Classify the chain whose terminal `.unwrap_or(` starts at `dot_pos`
/// (index of the `.`). Returns the site kind when the chain starts at a
/// ConfigView read (or a guest wrapper / `and_then`-over-`Option<&ConfigView>`
/// read), passing only `.map` / `.filter` / `.or_else` / `.and_then` frames.
fn classify_chain(text: &[u8], mask: &[u8], dot_pos: usize) -> Option<SiteKind> {
    let mut pos = dot_pos;
    // Names of connector frames in the order they were peeled off, nearest to
    // the `.unwrap_or` first, together with each frame's argument range.
    let mut frames: Vec<(String, std::ops::Range<usize>)> = Vec::new();
    loop {
        let prev = prev_code(text, mask, pos)?;
        if text[prev] == b')' {
            // A call frame between the unwrap_or and the base.
            let open = match_open_paren_backward(text, mask, prev)?;
            let frame_range = (open + 1)..prev;
            // Last byte of the callee name (right before `(`), then the
            // identifier itself and the byte before it.
            let name_last = prev_code(text, mask, open)?;
            let (name_start, name_end) = ident_before(text, name_last)?;
            let callee = std::str::from_utf8(&text[name_start..name_end]).ok()?;
            let is_method = prev_code(text, mask, name_start).is_some_and(|p| text[p] == b'.');
            if is_method {
                if GETTERS.contains(&callee) {
                    // The chain ends at a ConfigView read.
                    return Some(kind_of(&frames));
                }
                if CONNECTORS.contains(&callee) {
                    frames.push((callee.to_string(), frame_range));
                    // Step over the `.` so the next iteration looks at the
                    // receiver.
                    pos = prev_code(text, mask, name_start).expect("dot checked above");
                    continue;
                }
                // A non-connector method breaks the config chain.
                return None;
            }
            // Free-function call at the base: `wrapper(...)`.
            if READ_WRAPPERS.iter().any(|(w, _)| *w == callee) {
                return Some(kind_of(&frames));
            }
            return None;
        }
        if is_ident_byte(text[prev]) {
            // Bare-identifier base, e.g. `config.and_then(|c| c.get_abs_value(..))`:
            // valid only when the outermost popped frame is `.and_then` /
            // `.or_else` whose closure contains a ConfigView read.
            if let Some((method, range)) = frames.last() {
                if (method == "and_then" || method == "or_else")
                    && code_contains(text, mask, range.clone(), b".")
                {
                    for getter in GETTERS {
                        let pat = format!(".{getter}(");
                        if code_contains(text, mask, range.clone(), pat.as_bytes()) {
                            return Some(kind_of(&frames));
                        }
                    }
                }
            }
            return None;
        }
        return None;
    }
}

fn kind_of(frames: &[(String, std::ops::Range<usize>)]) -> SiteKind {
    // The outermost (base-side) connector names the shape.
    match frames.last().map(|(m, _)| m.as_str()) {
        Some("map") => SiteKind::Map,
        Some("filter") => SiteKind::Filter,
        Some("or_else") => SiteKind::OrElse,
        Some("and_then") => SiteKind::AndThen,
        _ => SiteKind::Direct,
    }
}

fn line_of(text: &[u8], pos: usize) -> usize {
    1 + text[..pos].iter().filter(|b| **b == b'\n').count()
}

/// Every classified `.unwrap_or(<literal>)` site in one source text.
fn find_fallback_sites(guest: &str, name: &str, text: &str) -> Vec<Site> {
    let (raw, mut mask) = mask_source(text);
    blank_cfg_test_regions(&raw, &mut mask);
    let mut sites = Vec::new();
    let mut i = 0usize;
    while let Some(dot) = find_code(&raw, &mask, b".unwrap_or(", i) {
        let open = dot + b".unwrap_or(".len() - 1; // the `(` after `unwrap_or`
        if let Some(close) = code_contains_close(&raw, &mask, open) {
            let arg = &raw[open + 1..close];
            if is_literal_arg(arg) {
                if let Some(kind) = classify_chain(&raw, &mask, dot) {
                    sites.push(Site {
                        guest: guest.to_string(),
                        file: name.to_string(),
                        line: line_of(&raw, dot),
                        kind,
                    });
                }
            }
        }
        i = dot + 1;
    }
    sites
}

/// Find the `)` matching a `(` at `open` (forward).
fn code_contains_close(text: &[u8], mask: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < text.len() {
        if mask[i] == CODE {
            match text[i] {
                b'(' => depth += 1,
                b')' => {
                    if depth == 1 {
                        return Some(i);
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Every string-literal key read through a ConfigView getter or a guest
/// wrapper, as `(key, "file:line")`. The key must be a literal at the
/// getter's first argument or the wrapper's declared key position.
fn find_config_reads(guest: &str, name: &str, text: &str) -> Vec<(String, String)> {
    let (raw, mut mask) = mask_source(text);
    blank_cfg_test_regions(&raw, &mut mask);
    let mut reads = Vec::new();

    let mut visit = |raw: &[u8], mask: &[u8], callee_start: usize, key_pos: usize| {
        // `callee_start` points at the callee name; the call's `(` follows.
        let Some(open) = raw
            .get(callee_start..)
            .and_then(|_| (callee_start + 1..raw.len()).find(|&i| raw[i] == b'('))
        else {
            return;
        };
        let args = call_args(raw, mask, open);
        let key_range = args.get(key_pos).cloned();
        let Some(key_range) = key_range else { return };
        let key = trim_ascii(&raw[key_range]);
        if key.len() >= 2 && key[0] == b'"' && key[key.len() - 1] == b'"' {
            let key = &key[1..key.len() - 1];
            let key = String::from_utf8(key.to_vec()).expect("config key must be UTF-8");
            reads.push((key, format!("{name}:{}", line_of(raw, callee_start))));
        }
    };

    // Direct getter reads: `.get(`, `.get_bool(`, ... with the literal as the
    // FIRST argument.
    for getter in GETTERS {
        let pat = format!(".{getter}(");
        let pat = pat.as_bytes();
        let mut i = 0usize;
        while let Some(start) = find_code(&raw, &mask, pat, i) {
            visit(&raw, &mask, start + 1, 0);
            i = start + 1;
        }
    }
    // Wrapper reads: `wrapper(..., "<key>", ...)` with the key at the
    // declared position.
    for (wrapper, key_pos) in READ_WRAPPERS {
        let pat = format!("{wrapper}(");
        let pat = pat.as_bytes();
        let mut i = 0usize;
        while let Some(start) = find_code(&raw, &mask, pat, i) {
            // Must be a free-function call, not a method and not part of a
            // longer identifier (e.g. `speed_value(` must not match `speed`).
            let before = prev_code(&raw, &mask, start);
            let method_or_ident = before.is_some_and(|p| (raw[p] == b'.') || is_ident_byte(raw[p]));
            if !method_or_ident {
                visit(&raw, &mask, start, *key_pos);
            }
            i = start + 1;
        }
    }
    // Silence unused-variable warning for the `guest` parameter used only in
    // diagnostics by the caller.
    let _ = guest;
    reads
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn core_modules_dir() -> PathBuf {
    repo_root().join("modules/core-modules")
}

fn guest_src_files(guest_dir: &Path) -> Vec<PathBuf> {
    let src = guest_dir.join("src");
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(
                    std::fs::read_dir(&path)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
                        .filter(|p| !p.to_string_lossy().contains("target")),
                );
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Scan one guest's production source: classified fallback sites.
fn census_guest(guest: &str, guest_dir: &Path) -> Vec<Site> {
    let mut sites = Vec::new();
    for path in guest_src_files(guest_dir) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read guest source {}: {e}", path.display()));
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        sites.extend(find_fallback_sites(guest, &name, &text));
    }
    sites
}

fn census_all() -> (Vec<Site>, BTreeMap<String, usize>) {
    let root = core_modules_dir();
    let mut sites = Vec::new();
    let mut per_guest = BTreeMap::new();
    for (guest, _) in CENSUS_GUESTS {
        let dir = root.join(guest);
        assert!(
            dir.is_dir(),
            "census guest directory must exist: {}",
            dir.display()
        );
        let found = census_guest(guest, &dir);
        per_guest.insert(guest.to_string(), found.len());
        sites.extend(found);
    }
    (sites, per_guest)
}

fn reads_all() -> Vec<(String, String, String)> {
    let root = core_modules_dir();
    let mut reads = Vec::new();
    let mut seen = BTreeSet::new();
    for (guest, _) in CENSUS_GUESTS {
        let dir = root.join(guest);
        let toml = dir.join(format!("{guest}.toml"));
        let toml_text = std::fs::read_to_string(&toml)
            .unwrap_or_else(|e| panic!("read manifest {}: {e}", toml.display()));
        let declared = manifest_declared_keys(&toml_text);
        let claims_support_family = manifest_claims_support_family(&toml_text);
        for path in guest_src_files(&dir) {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read guest source {}: {e}", path.display()));
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            for (key, loc) in find_config_reads(guest, &name, &text) {
                if declared.contains(&key) {
                    continue;
                }
                if claims_support_family && (key == "support_type" || key == "support_family") {
                    continue;
                }
                if seen.insert((guest.to_string(), key.clone())) {
                    reads.push((guest.to_string(), key, loc));
                }
            }
        }
    }
    reads
}

/// Declared `[config.schema.<key>]` table header keys (indented or not).
fn manifest_declared_keys(toml: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("[config.schema.") {
            if let Some(key) = rest.strip_suffix(']') {
                if !key.is_empty() && !key.contains(' ') {
                    keys.insert(key.to_string());
                }
            }
        }
    }
    keys
}

/// Whether the manifest's `[claims]` block holds a `support-family:` claim.
fn manifest_claims_support_family(toml: &str) -> bool {
    let mut in_claims = false;
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed == "[claims]" {
            in_claims = true;
            continue;
        }
        if in_claims && trimmed.starts_with('[') {
            return false;
        }
        if in_claims && trimmed.contains("support-family:") {
            return true;
        }
    }
    false
}

// ────────────────────────────────────────────────────────────────────────────
// AC-3 tests
// ────────────────────────────────────────────────────────────────────────────

/// AC-3 census: zero classified sites. RED until Step 5 removes the 87
/// baseline sites; the failure message carries the exact residual count and
/// the per-guest distribution so progress is visible.
#[test]
fn guest_config_literal_fallback_census_is_zero() {
    let (sites, per_guest) = census_all();
    let detail: Vec<String> = sites
        .iter()
        .take(5)
        .map(|s| format!("{}:{}:{}:{:?}", s.guest, s.file, s.line, s.kind))
        .collect();
    assert!(
        sites.is_empty(),
        "residual config-literal fallback sites remain: total={} per-guest={:?} (baseline 87) \
         first_sites={detail:?}",
        sites.len(),
        per_guest,
    );
}

/// AC-3 detector calibration: one verbatim baseline snippet per Step-1 chain
/// shape is flagged; excluded forms are never flagged.
#[test]
fn guest_fallback_detector_is_calibrated() {
    // ── Flagged shapes (verbatim baseline snippets) ────────────────────────
    let flagged: &[(&str, &str)] = &[
        // direct — arachne-perimeters/src/lib.rs:153
        (
            "direct",
            r#"    let layer_height_mm = config.get_float("layer_height").unwrap_or(0.2);"#,
        ),
        // `.map` — classic-perimeters/src/lib.rs:200-203
        (
            "map",
            r#"        let seam_candidate_angle_threshold_deg = _config
            .get_float("seam_candidate_angle_threshold_deg")
            .map(|v| v as f32)
            .unwrap_or(30.0);"#,
        ),
        // `.or_else` — overhang-classifier-default/src/lib.rs:72-77
        (
            "or_else",
            r#"fn line_width(config: &ConfigView) -> f32 {
    config
        .get_float("outer_wall_line_width")
        .or_else(|| config.get_float("line_width"))
        .unwrap_or(0.0) as f32
}"#,
        ),
        // `.and_then(match…)` — tree-support-planner/src/lib.rs:1692-1698,
        // completed with a literal ending: no baseline site of this shape
        // carries one (the planner's own chain ends in `unwrap_or_else`), and
        // AC-3's chain-start definition covers the read inside the closure.
        (
            "and_then(match…)",
            r#"    let value = config
        .get("support_type")
        .or_else(|| config.get("support_family"))
        .and_then(|value| match value {
            ConfigValue::String(value) => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or(0.0);"#,
        ),
        // `config_float(..).filter(..)` — infill-linker/src/orchestrate.rs:182-184
        (
            "config_float().filter()",
            r#"        let density = config_float(view.config(), "infill_density")
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(0.2);"#,
        ),
        // `config.and_then(|c| c.get_abs_value(..))` — infill-linker/src/connect.rs:49-51
        (
            "config.and_then(get_abs_value)",
            r#"        let anchor_length_max = config
            .and_then(|config| config.get_abs_value("infill_anchor_max", base_spacing))
            .unwrap_or(20.0);"#,
        ),
    ];
    for (shape, snippet) in flagged {
        let sites = find_fallback_sites("calibration", shape, snippet);
        assert!(
            sites.len() == 1,
            "calibration shape '{shape}' must flag exactly one site, found {}: {:?}",
            sites.len(),
            sites
        );
    }

    // ── Excluded forms (verbatim baseline snippets) ─────────────────────────
    let excluded: &[(&str, &str)] = &[
        // `unwrap_or_else` — traditional-support-planner/src/lib.rs:1699-1701
        (
            "unwrap_or_else",
            r#"    value
        .map(|value| canonical_support_family_alias(Some(value)))
        .unwrap_or_else(|| "traditional".to_string())"#,
        ),
        // sort comparator — tree-support/src/lib.rs:758
        (
            "sort comparator",
            r#"            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));"#,
        ),
        // `unwrap_or(<expr>)` — arachne-perimeters/src/lib.rs:263-266
        (
            "unwrap_or(expr)",
            r#"    let distribution_count = config
        .get_int("wall_distribution_count")
        .map(|v| v as u32)
        .unwrap_or(defaults.distribution_count);"#,
        ),
        // `unwrap_or(<const>)` — tree-support-planner/src/lib.rs:1637-1649
        (
            "unwrap_or(const)",
            r#"        let support_line_width_mm = config
            .get_abs_value("support_line_width", nozzle_diameter)
            // Preserve hand-written legacy configs that encode an absolute
            // width as an integer rather than a Float/FloatOrPercent.
            .or_else(|| config.get_int("support_line_width").map(|v| v as f64))
            .map(|v| {
                if v > 0.0 {
                    v as f32
                } else {
                    nozzle_diameter as f32
                }
            })
            .filter(|v| *v > 0.0)
            .unwrap_or(DEFAULT_SUPPORT_LINE_WIDTH_MM);"#,
        ),
        // `#[cfg(test)]` code — classic-perimeters/src/lib.rs:1380/1448-1455
        (
            "cfg(test)",
            r#"#[cfg(test)]
mod tests {
    #[test]
    fn corroboration() {
        // Same corroboration for layer_height (`.unwrap_or(0.2)` fallback).
        let mut layer_fields = HashMap::new();
        layer_fields.insert("layer_height".to_string(), ConfigValue::Float(0.28));
        let layer_config = ConfigView::from_map(layer_fields);
        let layer_height = layer_config
            .get_float("layer_height")
            .map(|v| v as f32)
            .unwrap_or(0.2);
        assert!((layer_height - 0.28).abs() < f32::EPSILON);
    }
}"#,
        ),
    ];
    for (shape, snippet) in excluded {
        let sites = find_fallback_sites("calibration", shape, snippet);
        assert!(
            sites.is_empty(),
            "excluded form '{shape}' must not be flagged, found: {sites:?}"
        );
    }
}

/// AC-3 declared-reads: every string-literal key read through a ConfigView
/// getter or a guest wrapper is declared in the guest's own manifest or is a
/// `support_type` / `support_family` read by a `support-family:` claimant.
/// RED until Step 5 manifests land; the failure message lists the residuals.
#[test]
fn guest_config_reads_are_declared() {
    let residuals = reads_all();
    assert!(
        residuals.is_empty(),
        "undeclared config reads remain: {} residuals: {:?}",
        residuals.len(),
        residuals
    );
}
