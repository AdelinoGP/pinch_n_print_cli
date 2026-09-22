//! Packet `config-scope-resolution_07_scope-eligibility` — per-key
//! `denied_scopes` roster (AC-1, ADR-0069) and registry-derived admission
//! enforcement (AC-3, AC-4, AC-N1, AC-N2).
//!
//! Two tests share one hand-authored oracle:
//!
//! - [`host_denial_roster_is_exact_across_host_declaration_channels`] is the
//!   host half: every host declaration channel (`ResolvedConfig` DSL rows,
//!   `SPEED_DENIED_SCOPES`, `HOST_RUNTIME_KEYS`) must produce exactly AC-1's
//!   policy, and nothing else.
//! - [`authored_denial_roster_is_exact_across_all_declarers`] adds the module
//!   half: every core-module manifest that declares an AC-1 key must author
//!   that key's policy itself, discovered mechanically from the manifests (no
//!   hand-maintained declarer roster). It stays red until Step 3 authors the
//!   module declarers.
//!
//! The oracle is AC-1 as written in the packet, not a snapshot of current
//! behaviour: the expected key sets are hand-authored, and the declarer side
//! is derived from the declaration channels themselves, so a key added
//! anywhere without its policy (or a policy for a key no declarer states)
//! fails here rather than drifting silently.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use slicer_ir::feedrate::{SPEED_DENIED_SCOPES, SPEED_KEYS, SPEED_KEY_COUNT, SPEED_META};
use slicer_ir::resolved_config::{
    ResolvedConfig, HOST_RUNTIME_KEYS, SCOPE_FILAMENT, SCOPE_PRINT, SCOPE_PRINTER,
    TOOL_CAPABLE_SCOPES, WHOLE_PRINT_ONLY_SCOPES,
};

use slicer_config::{
    assemble_registry, resolve_scope_stack, ConfigSchemaRegistry, ConfigScope, ExpansionContext,
    HostChannels, ModuleDeclaration, ResolutionError, ResolutionTarget, ScopeDelta, ScopedConfig,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::ConfigValue;

/// AC-1 whole-print-only keys authored on host declaration channels, other
/// than the 26 [`SPEED_KEYS`].
const AC1_WHOLE_PRINT_ONLY: &[&str] = &[
    // Machine / emitter keys authored on the host DSL rows.
    "bed_shape",
    "disable_m73",
    "gcode_xy_decimals",
    "machine_max_acceleration_extruding",
    "machine_max_acceleration_travel",
    "machine_max_jerk_e",
    "machine_max_jerk_x",
    "machine_max_jerk_y",
    "machine_max_jerk_z",
    "machine_max_speed_e",
    "machine_max_speed_x",
    "machine_max_speed_y",
    "machine_max_speed_z",
    // Authored on the `HOST_RUNTIME_KEYS` rows.
    "use_relative_e_distances",
    "thumbnail_path",
    "wall_generator",
];

/// AC-1 tool-capable roster: a per-tool statement is meaningful, so the tool
/// scope stays statable. `nozzle_diameter` is declared only by module
/// manifests; the rest are host DSL rows.
const AC1_TOOL_CAPABLE: &[&str] = &[
    "filament_density",
    "filament_diameter",
    "nozzle_diameter",
    "retract_length",
];

/// The policy AC-1 authors for `key`, in canonical scope order.
fn ac1_policy(key: &str) -> &'static [&'static str] {
    if AC1_WHOLE_PRINT_ONLY.contains(&key) {
        WHOLE_PRINT_ONLY_SCOPES
    } else if AC1_TOOL_CAPABLE.contains(&key) {
        TOOL_CAPABLE_SCOPES
    } else {
        panic!("AC-1 oracle has no roster entry for {key}");
    }
}

/// Every host-channel declaration of a key, as the ADR-0069 **union** of its
/// denials in first-seen (canonical) order.
///
/// Union, not agreement: `outer_wall_speed` and `inner_wall_speed` are declared
/// by both the DSL rows and the speed table, and a scope denied by either
/// declarer is denied (ADR-0069 amendment). Union preserves order because the
/// canonical sets are prefix-consistent, so the "denial order differs" exit
/// condition stays falsifiable.
fn host_channel_policies() -> BTreeMap<String, Vec<&'static str>> {
    let mut out: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    let mut record = |key: &str, policy: &'static [&'static str]| {
        let entry = out.entry(key.to_owned()).or_default();
        for scope in policy {
            if !entry.iter().any(|seen| seen == scope) {
                entry.push(*scope);
            }
        }
    };

    for row in ResolvedConfig::host_config_keys() {
        record(row.key, row.denied_scopes);
    }
    for (index, (key, _)) in SPEED_KEYS.iter().enumerate() {
        record(key, SPEED_DENIED_SCOPES[index]);
    }
    for row in HOST_RUNTIME_KEYS {
        record(row.key, row.denied_scopes);
    }
    out
}

/// The host half of AC-1, shared by both tests so the full test cannot go
/// green while the host half is red.
fn assert_host_denial_roster_is_exact() {
    // The speed table is positionally aligned and length-locked; a drift in
    // any of the three tables is this exit condition.
    assert_eq!(SPEED_KEY_COUNT, 26, "AC-1 names 26 speed keys");
    assert_eq!(SPEED_KEYS.len(), SPEED_KEY_COUNT);
    assert_eq!(SPEED_META.len(), SPEED_KEY_COUNT);
    assert_eq!(SPEED_DENIED_SCOPES.len(), SPEED_KEY_COUNT);
    for (index, (key, _)) in SPEED_KEYS.iter().enumerate() {
        assert_eq!(
            SPEED_DENIED_SCOPES[index], WHOLE_PRINT_ONLY_SCOPES,
            "speed {key} must deny every sub-print scope"
        );
    }

    let policies = host_channel_policies();

    // AC-1's whole-print-only keys, each present and exactly right.
    for key in AC1_WHOLE_PRINT_ONLY {
        let policy = policies
            .get(*key)
            .unwrap_or_else(|| panic!("AC-1 whole-print-only key {key} is not declared"));
        assert_eq!(
            policy.as_slice(),
            WHOLE_PRINT_ONLY_SCOPES,
            "{key} must deny object/layer_range/modifier/paint_semantic/tool in order"
        );
    }

    // The tool-capable host keys (all but the module-only nozzle_diameter).
    for key in AC1_TOOL_CAPABLE
        .iter()
        .filter(|key| **key != "nozzle_diameter")
    {
        let policy = policies
            .get(*key)
            .unwrap_or_else(|| panic!("AC-1 tool-capable key {key} is not declared"));
        assert_eq!(
            policy.as_slice(),
            TOOL_CAPABLE_SCOPES,
            "{key} must deny object/layer_range/modifier/paint_semantic and allow tool"
        );
    }

    // No spurious denials: a host key outside AC-1's roster (and outside the
    // 26 speed keys, which the feedrate table pins) is not denied.
    for (key, policy) in &policies {
        if policy.is_empty() {
            continue;
        }
        let is_speed_key = SPEED_KEYS.iter().any(|(speed_key, _)| speed_key == key);
        assert!(
            is_speed_key
                || AC1_WHOLE_PRINT_ONLY.contains(&key.as_str())
                || AC1_TOOL_CAPABLE.contains(&key.as_str()),
            "{key} carries authored denials but is absent from AC-1's roster"
        );
    }
}

/// Every core-module manifest path, as `(module id, path)`, sorted by id.
///
/// Discovery is mechanical (`modules/core-modules/<id>/<id>.toml`), so the
/// declarer set is derived from the tree rather than hand-listed.
fn core_module_manifest_paths() -> Vec<(String, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("modules/core-modules");
    let mut out = Vec::new();
    let entries = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", root.display()));
    for entry in entries {
        let entry = entry.expect("core-modules directory entry");
        if !entry.file_type().expect("entry type").is_dir() {
            continue;
        }
        let module = entry.file_name().to_string_lossy().into_owned();
        let manifest = entry.path().join(format!("{module}.toml"));
        if manifest.is_file() {
            out.push((module, manifest));
        }
    }
    out.sort();
    assert!(!out.is_empty(), "no core module manifests discovered");
    out
}

/// The `denied_scopes` a manifest authors for `key`, in authored order.
///
/// `None` means the manifest declares the key but authors no denial — which
/// is a mismatch for every AC-1 key, not a skip.
fn manifest_denied_scopes(document: &toml::Value, key: &str) -> Option<Vec<String>> {
    let authored = document
        .get("config")?
        .get("schema")?
        .get(key)?
        .get("denied_scopes")?;
    let array = authored
        .as_array()
        .unwrap_or_else(|| panic!("{key}: denied_scopes must be an array"));
    Some(
        array
            .iter()
            .map(|scope| {
                scope
                    .as_str()
                    .unwrap_or_else(|| panic!("{key}: denied_scopes entries must be strings"))
                    .to_owned()
            })
            .collect(),
    )
}

#[test]
fn host_denial_roster_is_exact_across_host_declaration_channels() {
    assert_host_denial_roster_is_exact();
}

#[test]
fn authored_denial_roster_is_exact_across_all_declarers() {
    // The host half must hold whenever the full roster holds.
    assert_host_denial_roster_is_exact();

    let manifests = core_module_manifest_paths();
    let mut nozzle_declarers = Vec::new();
    let mut failures = Vec::new();

    for (module, path) in &manifests {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let document: toml::Value = toml::from_str(&text)
            .unwrap_or_else(|error| panic!("{} is not valid TOML: {error}", path.display()));

        for key in AC1_WHOLE_PRINT_ONLY.iter().chain(AC1_TOOL_CAPABLE.iter()) {
            let declared = document
                .get("config")
                .and_then(|config| config.get("schema"))
                .and_then(|schema| schema.get(*key))
                .is_some();
            if !declared {
                continue;
            }
            if *key == "nozzle_diameter" {
                nozzle_declarers.push(module.clone());
            }
            let expected: Vec<String> = ac1_policy(key)
                .iter()
                .map(|scope| (*scope).to_owned())
                .collect();
            let actual = manifest_denied_scopes(&document, key).unwrap_or_default();
            if actual != expected {
                failures.push(format!(
                    "{module} declares {key}: expected {expected:?}, authored {actual:?}"
                ));
            }
        }
    }

    // Mechanical guard against a vacuous module half.
    assert!(
        !nozzle_declarers.is_empty(),
        "no module manifest declares nozzle_diameter, so the module half of AC-1 is vacuous"
    );
    nozzle_declarers.sort();
    nozzle_declarers.dedup();
    assert!(
        failures.is_empty(),
        "module declarers have not authored AC-1 policies yet (Step 3); {} declarer(s) of nozzle_diameter: {nozzle_declarers:?}\n{}",
        nozzle_declarers.len(),
        failures.join("\n")
    );
}

// ── AC-3, AC-4, AC-N1, AC-N2 (Step 5) ─────────────────────────────────────
//
// These tests exercise admission and enforcement against the live host
// channels plus one module declaration carrying the module-only
// `nozzle_diameter`. The registry is assembled here rather than snapshotted,
// so every expectation is derived from the same AC-1 policy the tests above
// pin.

/// A module-authored `float` field carrying `denied` verbatim.
fn declared_float(default: &str, denied: &[&str]) -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: "float".to_owned(),
        default: Some(default.to_owned()),
        denied_scopes: denied.iter().map(|scope| (*scope).to_owned()).collect(),
        ..ConfigFieldEntry::default()
    }
}

/// Live host channels plus a module declaring `nozzle_diameter` (an AC-1
/// tool-capable key with no host row) and one ordinary key with no denials.
fn eligibility_registry() -> ConfigSchemaRegistry {
    assemble_registry(
        &[ModuleDeclaration {
            module_id: "dev.pinch.test.scope-eligibility".to_owned(),
            schema: ConfigSchema {
                entries: BTreeMap::from([
                    (
                        "nozzle_diameter".to_owned(),
                        declared_float("0.4", TOOL_CAPABLE_SCOPES),
                    ),
                    ("ordinary_key".to_owned(), declared_float("1.0", &[])),
                ]),
            },
            ..ModuleDeclaration::default()
        }],
        &HostChannels::from_live(),
    )
    .expect("eligibility fixture registry must assemble")
    .registry
}

fn delta(entries: impl IntoIterator<Item = (&'static str, ConfigValue)>) -> ScopeDelta {
    ScopeDelta {
        values: entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    }
}

fn scoped(deltas: impl IntoIterator<Item = (ConfigScope, ScopeDelta)>) -> ScopedConfig {
    ScopedConfig {
        deltas: deltas.into_iter().collect(),
    }
}

fn expansion() -> ExpansionContext {
    ExpansionContext {
        nozzle_diameter_mm: 0.4,
        ..ExpansionContext::default()
    }
}

/// The `(scope, denial label)` pairs of the deny-list vocabulary.
fn scope_pairs() -> [(ConfigScope, &'static str); 4] {
    [
        (ConfigScope::Object("obj-a".to_owned()), "object"),
        (
            ConfigScope::Modifier {
                object_id: "obj-a".to_owned(),
                modifier_id: "mod-1".to_owned(),
            },
            "modifier",
        ),
        (
            ConfigScope::PaintSemantic("material".to_owned()),
            "paint_semantic",
        ),
        (ConfigScope::Tool(0), "tool"),
    ]
}

// ── AC-3 ───────────────────────────────────────────────────────────────────

/// AC-3: `admission_set` is exactly the registry keys whose `denied_scopes`
/// omit the queried scope — no separate roster. Object admission excludes every
/// AC-1 key; tool admission includes the four tool-capable keys and excludes
/// the whole-print-only ones.
#[test]
fn admission_sets_are_derived_only_from_registry_denials() {
    let registry = eligibility_registry();

    // AC-1 whole-print-only keys, including the 26 speed keys, are excluded
    // from every sub-print admission set.
    for key in AC1_WHOLE_PRINT_ONLY {
        for (scope, label) in scope_pairs() {
            assert!(
                !registry.admission_set(&scope).contains(*key),
                "{key} is denied at {label} scope but was admitted"
            );
        }
    }
    for (key, _) in SPEED_KEYS {
        for (scope, label) in scope_pairs() {
            assert!(
                !registry.admission_set(&scope).contains(*key),
                "speed key {key} is denied at {label} scope but was admitted"
            );
        }
    }

    let object_admission = registry.admission_set(&ConfigScope::Object("obj-a".to_owned()));
    let tool_admission = registry.admission_set(&ConfigScope::Tool(0));
    for key in AC1_TOOL_CAPABLE {
        assert!(
            tool_admission.contains(*key),
            "tool-capable {key} must be admissible at tool scope"
        );
        assert!(
            !object_admission.contains(*key),
            "tool-capable {key} must be denied at object scope"
        );
    }
    assert!(
        !tool_admission.contains("bed_shape") && !tool_admission.contains("thumbnail_path"),
        "whole-print-only keys must not be tool-admissible"
    );

    // A key with no denials is admitted at every scope; derivation, not a
    // hand-authored allow list, must say so.
    for (scope, label) in scope_pairs() {
        assert!(
            registry.admission_set(&scope).contains("ordinary_key"),
            "ordinary_key has no denials, so it must be admitted at {label}"
        );
    }

    // Exhaustive derivation check: membership equals the reconciled entry's
    // own `denied_scopes` for every registry key and every sub-print scope.
    for (scope, label) in scope_pairs() {
        let admission = registry.admission_set(&scope);
        for key in registry.keys() {
            let entry = registry
                .entry(key)
                .unwrap_or_else(|| panic!("{key} must have a reconciled entry"));
            let expected = entry.denied_scopes.iter().all(|denied| denied != label);
            assert_eq!(
                admission.contains(key),
                expected,
                "{key} membership at {label} must come from its registry denials"
            );
        }
    }
}

// ── AC-4 ───────────────────────────────────────────────────────────────────

/// AC-4: allowed values resolve normally — the two tool-capable keys at tool
/// scope, and an ordinary key at object, modifier, paint-semantic and tool
/// scopes.
#[test]
fn allowed_scope_values_resolve_normally() {
    let registry = eligibility_registry();

    let tool_resolved = resolve_scope_stack(
        &registry,
        &scoped([(
            ConfigScope::Tool(1),
            delta([
                ("retract_length", ConfigValue::Float(5.5)),
                ("nozzle_diameter", ConfigValue::Float(0.6)),
            ]),
        )]),
        &ResolutionTarget {
            tool_index: Some(1),
            ..ResolutionTarget::default()
        },
        &expansion(),
    )
    .expect("tool-capable keys must resolve at tool scope");
    assert_eq!(
        tool_resolved.retract_length, 5.5,
        "authored retract_length must apply at tool scope"
    );
    assert_eq!(
        tool_resolved.extensions.get("nozzle_diameter"),
        Some(&ConfigValue::Float(0.6)),
        "authored nozzle_diameter must reach the resolved extensions at tool scope"
    );

    let cases = [
        (
            ConfigScope::Object("obj-a".to_owned()),
            ResolutionTarget {
                object_id: "obj-a".to_owned(),
                ..ResolutionTarget::default()
            },
            0.11_f64,
        ),
        (
            ConfigScope::Modifier {
                object_id: "obj-a".to_owned(),
                modifier_id: "mod-1".to_owned(),
            },
            ResolutionTarget {
                object_id: "obj-a".to_owned(),
                modifier_ids: vec!["mod-1".to_owned()],
                ..ResolutionTarget::default()
            },
            0.22_f64,
        ),
        (
            ConfigScope::PaintSemantic("material".to_owned()),
            ResolutionTarget {
                paint_semantics: vec!["material".to_owned()],
                ..ResolutionTarget::default()
            },
            0.33_f64,
        ),
        (
            ConfigScope::Tool(2),
            ResolutionTarget {
                tool_index: Some(2),
                ..ResolutionTarget::default()
            },
            0.44_f64,
        ),
    ];
    for (scope, target, value) in cases {
        let resolved = resolve_scope_stack(
            &registry,
            &scoped([(
                scope.clone(),
                delta([("ordinary_key", ConfigValue::Float(value))]),
            )]),
            &target,
            &expansion(),
        )
        .unwrap_or_else(|error| panic!("ordinary_key must resolve at {scope:?}: {error}"));
        assert_eq!(
            resolved.extensions.get("ordinary_key"),
            Some(&ConfigValue::Float(value)),
            "ordinary_key authored at {scope:?} must be applied"
        );
    }
}

// ── AC-N1 ──────────────────────────────────────────────────────────────────

/// AC-N1: a denied pair is rejected atomically — `bed_shape` at object scope
/// and `nozzle_diameter` at modifier scope return `ScopeDenied` naming the
/// exact pair, and the same deltas without the denied key still resolve.
#[test]
fn denied_scope_is_rejected_atomically() {
    let registry = eligibility_registry();
    let object_scope = ConfigScope::Object("obj-a".to_owned());
    let modifier_scope = ConfigScope::Modifier {
        object_id: "obj-a".to_owned(),
        modifier_id: "mod-1".to_owned(),
    };
    let object_target = ResolutionTarget {
        object_id: "obj-a".to_owned(),
        ..ResolutionTarget::default()
    };

    let error = resolve_scope_stack(
        &registry,
        &scoped([(
            object_scope.clone(),
            delta([
                ("infill_density", ConfigValue::Float(0.9)),
                ("bed_shape", ConfigValue::String("0,0,250,250".to_owned())),
            ]),
        )]),
        &object_target,
        &expansion(),
    )
    .expect_err("bed_shape must be rejected at object scope");
    assert_eq!(
        error,
        ResolutionError::ScopeDenied {
            key: "bed_shape".to_owned(),
            scope: object_scope.clone(),
        },
        "the error must name the exact denied pair"
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("bed_shape") && rendered.contains("object"),
        "display must name the key and scope, got {rendered:?}"
    );

    // Non-vacuous: the same delta without the denied key resolves, so the
    // rejection is the pair, not the delta as a whole.
    let allowed = resolve_scope_stack(
        &registry,
        &scoped([(
            object_scope,
            delta([("infill_density", ConfigValue::Float(0.9))]),
        )]),
        &object_target,
        &expansion(),
    )
    .expect("the same delta without the denied key must resolve");
    assert_eq!(allowed.infill_density, 0.9);

    let error = resolve_scope_stack(
        &registry,
        &scoped([(
            modifier_scope.clone(),
            delta([
                ("layer_height", ConfigValue::Float(0.3)),
                ("nozzle_diameter", ConfigValue::Float(0.6)),
            ]),
        )]),
        &ResolutionTarget {
            object_id: "obj-a".to_owned(),
            modifier_ids: vec!["mod-1".to_owned()],
            ..ResolutionTarget::default()
        },
        &expansion(),
    )
    .expect_err("nozzle_diameter must be rejected at modifier scope");
    assert_eq!(
        error,
        ResolutionError::ScopeDenied {
            key: "nozzle_diameter".to_owned(),
            scope: modifier_scope,
        },
        "the error must name the exact denied pair"
    );
}

// ── AC-N2 ──────────────────────────────────────────────────────────────────

/// A one-key module declaration authoring `denied` for `key`.
fn denial_module(module_id: &str, key: &str, denied: &[&str]) -> ModuleDeclaration {
    ModuleDeclaration {
        module_id: module_id.to_owned(),
        schema: ConfigSchema {
            entries: BTreeMap::from([(key.to_owned(), declared_float("1.0", denied))]),
        },
        ..ModuleDeclaration::default()
    }
}

/// AC-N2: a denial authored by one declarer binds the union even when another
/// declarer omits it, in either discovery/declaration order.
#[test]
fn multi_declarer_denial_union_is_order_independent() {
    // `union_key`: alpha denies, beta omits. `other_key`: beta omits, alpha
    // denies — the two directions of the same union rule.
    let mut alpha = denial_module("dev.pinch.test.alpha", "union_key", &["object", "modifier"]);
    alpha
        .schema
        .entries
        .insert("other_key".to_owned(), declared_float("2.0", &[]));
    let mut beta = denial_module("dev.pinch.test.beta", "union_key", &[]);
    beta.schema
        .entries
        .insert("other_key".to_owned(), declared_float("2.0", &["tool"]));

    let orders = [[alpha.clone(), beta.clone()], [beta.clone(), alpha.clone()]];
    let mut registries = Vec::new();
    for order in orders {
        let registry = assemble_registry(&order, &HostChannels::from_live())
            .expect("union fixture registry must assemble")
            .registry;

        let union_entry = registry.entry("union_key").expect("union_key reconciled");
        assert_eq!(
            union_entry.denied_scopes,
            ["object".to_owned(), "modifier".to_owned()],
            "union must keep alpha's denials in canonical scope order"
        );
        let other_entry = registry.entry("other_key").expect("other_key reconciled");
        assert_eq!(
            other_entry.denied_scopes,
            ["tool".to_owned()],
            "union must keep beta's denial even though alpha omits it"
        );

        assert!(
            !registry
                .admission_set(&ConfigScope::Object("obj-a".to_owned()))
                .contains("union_key"),
            "object must stay denied although beta omits the denial"
        );
        assert!(
            !registry
                .admission_set(&ConfigScope::Modifier {
                    object_id: "obj-a".to_owned(),
                    modifier_id: "mod-1".to_owned(),
                })
                .contains("union_key"),
            "modifier must stay denied although beta omits the denial"
        );
        assert!(
            registry
                .admission_set(&ConfigScope::Tool(0))
                .contains("union_key"),
            "a scope no declarer denies stays admissible"
        );

        let error = resolve_scope_stack(
            &registry,
            &scoped([(
                ConfigScope::Object("obj-a".to_owned()),
                delta([("union_key", ConfigValue::Float(0.7))]),
            )]),
            &ResolutionTarget {
                object_id: "obj-a".to_owned(),
                ..ResolutionTarget::default()
            },
            &expansion(),
        )
        .expect_err("the union denial must reject resolution at object scope");
        assert_eq!(
            error,
            ResolutionError::ScopeDenied {
                key: "union_key".to_owned(),
                scope: ConfigScope::Object("obj-a".to_owned()),
            }
        );

        let error = resolve_scope_stack(
            &registry,
            &scoped([(
                ConfigScope::Tool(1),
                delta([("other_key", ConfigValue::Float(2.5))]),
            )]),
            &ResolutionTarget {
                tool_index: Some(1),
                ..ResolutionTarget::default()
            },
            &expansion(),
        )
        .expect_err("the union denial must reject resolution at tool scope");
        assert_eq!(
            error,
            ResolutionError::ScopeDenied {
                key: "other_key".to_owned(),
                scope: ConfigScope::Tool(1),
            }
        );

        registries.push(registry);
    }

    assert_eq!(
        registries[0], registries[1],
        "discovery/declaration order must not change the reconciled registry"
    );
}

// ── AC-2 (Step 4) ─────────────────────────────────────────────────────────
//
// AC-2: the registry's denials for the AC-1 roster are reproducible by a
// mechanical usage-shape rule — never by reading the authored
// `denied_scopes` values — and the comparison is sensitive to registry
// mutation (control test below). `derive_expected_denials` is the shared
// oracle of both tests.

/// Host-scope declarations of every key, `key -> preset scope string`
/// (`SCOPE_PRINTER` / `SCOPE_FILAMENT`), across both host declaration
/// channels.
fn host_scope_declarations() -> BTreeMap<String, &'static str> {
    let mut out = BTreeMap::new();
    for row in ResolvedConfig::host_config_keys() {
        out.insert(row.key.to_owned(), row.scope);
    }
    for row in HOST_RUNTIME_KEYS {
        out.insert(row.key.to_owned(), row.scope);
    }
    out
}

/// Every key declared by a `Layer::*` module manifest, as
/// `key -> [(layer module id, stage id)]`.
///
/// Layer granularity is read from each manifest's `[stage] id` and the
/// declaration set from its `[config.schema]` keys — a runtime `std::fs`
/// scan, not a source grep and not a hand-maintained roster. The stage id is
/// kept so a mismatch message can name the declarer.
fn layer_granular_declarers() -> BTreeMap<String, Vec<(String, String)>> {
    let mut out: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut layer_modules = 0usize;
    for (module, path) in core_module_manifest_paths() {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let document: toml::Value = toml::from_str(&text)
            .unwrap_or_else(|error| panic!("{} is not valid TOML: {error}", path.display()));
        let stage_id = document
            .get("stage")
            .and_then(|stage| stage.get("id"))
            .and_then(|id| id.as_str())
            .map_or_else(|| "".to_owned(), |id| id.to_owned());
        // A `Layer::*` stage id marks layer granularity; ids that merely
        // *contain* "layer" (e.g. `PostPass::LayerFinalization`) are other
        // stages and must not count as layer declarers.
        if !stage_id.starts_with("Layer") {
            continue;
        }
        layer_modules += 1;
        let Some(schema) = document
            .get("config")
            .and_then(|config| config.get("schema"))
            .and_then(|schema| schema.as_table())
        else {
            continue;
        };
        for key in schema.keys() {
            out.entry(key.clone())
                .or_default()
                .push((module.clone(), stage_id.clone()));
        }
    }
    assert!(
        layer_modules > 0,
        "no module manifest carries a Layer stage id — the AC-2 layer clause is vacuous"
    );
    out
}

/// The AC-2 shared oracle: expected `denied_scopes` for every key of the
/// roster universe, derived from usage shape alone.
///
/// 1. A [`SPEED_KEYS`] key is whole-print only — a sub-print statement cannot
///    mean anything for a feedrate key.
/// 2. A host key authored at whole-print preset scope ([`SCOPE_PRINT`] or
///    [`SCOPE_PRINTER`]) with no per-tool consumer and no layer-granular
///    declarer is whole-print only.
/// 3. Every other key — filament preset ([`SCOPE_FILAMENT`]; consumed per
///    tool by `ResolvedConfig::filament_density_for`, which is why a tool
///    scope statement can narrow it), a layer-granular declarer, or a key
///    only a module declares (`nozzle_diameter`) — stays tool-capable.
///
/// Nothing here reads an authored `denied_scopes` value; both AC-2 tests
/// compare this derivation against the assembled registry.
fn derive_expected_denials(
    host_scopes: &BTreeMap<String, &'static str>,
    layer_declarers: &BTreeMap<String, Vec<(String, String)>>,
    universe: &BTreeSet<String>,
) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for key in universe {
        let is_speed = SPEED_KEYS
            .iter()
            .any(|(speed_key, _)| *speed_key == key.as_str());
        let layer_declared = layer_declarers.contains_key(key);
        let host_scope = host_scopes.get(key.as_str()).copied();
        let per_tool_consumer = host_scope == Some(SCOPE_FILAMENT);
        let whole_print_preset =
            host_scope == Some(SCOPE_PRINT) || host_scope == Some(SCOPE_PRINTER);
        let whole_print_only =
            is_speed || (whole_print_preset && !layer_declared && !per_tool_consumer);
        let policy = if whole_print_only {
            WHOLE_PRINT_ONLY_SCOPES
        } else {
            TOOL_CAPABLE_SCOPES
        };
        out.insert(
            key.clone(),
            policy.iter().map(|scope| (*scope).to_owned()).collect(),
        );
    }
    out
}

/// The AC-2 fixture triple: the mechanical derivation over the roster
/// universe (host `host_channel_policies` keys ∪ assembled registry keys —
/// no hard-coded key list), the denials as the registry actually assembled
/// them, and the layer-declarer map for diagnostics. Shared by the derivation
/// test and its mutation-control test so the control cannot go green while
/// the derivation is red.
fn ac2_derived_and_assembled() -> (
    BTreeMap<String, Vec<String>>,
    BTreeMap<String, Vec<String>>,
    BTreeMap<String, Vec<(String, String)>>,
) {
    let registry = eligibility_registry();
    let host_scopes = host_scope_declarations();
    let layer_declarers = layer_granular_declarers();

    let mut universe: BTreeSet<String> = host_channel_policies().keys().cloned().collect();
    universe.extend(registry.keys().map(|key| key.to_owned()));
    assert!(
        !universe.is_empty() && !layer_declarers.is_empty(),
        "AC-2 oracle inputs must be non-empty, or the derivation is vacuous"
    );

    let derived = derive_expected_denials(&host_scopes, &layer_declarers, &universe);
    let assembled: BTreeMap<String, Vec<String>> = registry
        .keys()
        .map(|key| {
            let denied = registry
                .entry(key)
                .unwrap_or_else(|| panic!("{key} must have a reconciled entry"))
                .denied_scopes
                .clone();
            (key.to_owned(), denied)
        })
        .collect();
    assert!(
        !derived.is_empty() && !assembled.is_empty(),
        "AC-2 comparison maps must be non-empty, or the comparison is vacuous"
    );
    (derived, assembled, layer_declarers)
}

/// AC-2 derivation: the mechanical usage-shape rule reproduces AC-1's
/// author-confirmed policy for every roster key, and the assembled registry
/// agrees with the derivation for those same keys.
#[test]
fn mechanical_derivation_matches_author_confirmed_denials() {
    let (derived, assembled, layer_declarers) = ac2_derived_and_assembled();
    assert!(
        !AC1_WHOLE_PRINT_ONLY.is_empty() && !AC1_TOOL_CAPABLE.is_empty(),
        "AC-1 rosters must be non-empty for the comparison to be non-vacuous"
    );

    for key in AC1_WHOLE_PRINT_ONLY.iter().chain(AC1_TOOL_CAPABLE.iter()) {
        let derived_policy = derived
            .get(*key)
            .unwrap_or_else(|| panic!("derivation must cover AC-1 key {key}"));
        let assembled_policy = assembled
            .get(*key)
            .unwrap_or_else(|| panic!("registry must cover AC-1 key {key}"));
        let authored: Vec<String> = ac1_policy(key)
            .iter()
            .map(|scope| (*scope).to_owned())
            .collect();
        let declarers = layer_declarers
            .get(*key)
            .map(|declarers| format!("; layer declarers: {declarers:?}"))
            .unwrap_or_default();
        assert_eq!(
            derived_policy, &authored,
            "{key}: the mechanical usage-shape rule must reproduce AC-1's authored policy{declarers}"
        );
        assert_eq!(
            derived_policy, assembled_policy,
            "{key}: the mechanical derivation must agree with the assembled registry{declarers}"
        );
    }
}

/// The AC-1 roster, projected from either comparison map — the only keys AC-2
/// claims (the derivation rule is stated over the full universe, but its
/// contract is the author-confirmed roster).
fn roster_projection(map: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, Vec<String>> {
    AC1_WHOLE_PRINT_ONLY
        .iter()
        .chain(AC1_TOOL_CAPABLE.iter())
        .map(|key| {
            (
                key.to_string(),
                map.get(*key)
                    .cloned()
                    .unwrap_or_else(|| panic!("projected map must cover AC-1 key {key}")),
            )
        })
        .collect()
}

/// AC-2 mutation control: the shared oracle is not insensitive — deleting one
/// (key, scope) pair from the assembled map's roster projection must change the
/// comparison's outcome.
#[test]
fn derivation_control_detects_removed_denial_pair() {
    let (derived, assembled, _) = ac2_derived_and_assembled();
    let mut projected = roster_projection(&assembled);
    assert_eq!(
        roster_projection(&derived),
        projected,
        "control baseline: derivation and assembled registry must agree on every roster key"
    );

    // Remove one (key, scope) pair mechanically: the first scope of the
    // sorted-first roster key's denial list.
    let first_key = projected
        .keys()
        .next()
        .cloned()
        .expect("the roster projection must be non-empty");
    let removed_scope = {
        let denied = projected
            .get_mut(&first_key)
            .unwrap_or_else(|| panic!("{first_key} must be in the roster projection"));
        assert!(
            !denied.is_empty(),
            "{first_key} must carry denials to mutate"
        );
        denied.remove(0)
    };
    assert!(
        derived
            .get(&first_key)
            .unwrap_or_else(|| panic!("derived must cover {first_key}"))
            .contains(&removed_scope),
        "the removed (key, scope) pair must be part of the derived policy"
    );
    assert_ne!(
        roster_projection(&derived),
        projected,
        "removing ({first_key}, {removed_scope}) from the registry denials must change the oracle outcome"
    );
}
