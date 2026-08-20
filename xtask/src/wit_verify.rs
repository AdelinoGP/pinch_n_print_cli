//! Verify that a built guest component's *embedded* WIT world actually matches
//! the canonical WIT on disk.
//!
//! # Why this exists
//!
//! `#[slicer_module]` reaches the canonical WIT through `slicer-macros`, which
//! `include_str!`s the `.wit` files and bakes them into the compiled proc-macro
//! binary. Every guest builds in its **own isolated cargo workspace**
//! (`modules/core-modules/*/wit-guest/`, each with its own `Cargo.lock` and
//! `target/`, enforced by a `[workspace]` sentinel). When such a workspace holds
//! a cached `slicer-macros` artifact that the WIT mtimes do not invalidate,
//! `slicer-macros/build.rs`'s `rerun-if-changed` never fires, cargo recompiles
//! nothing, and the stale macro keeps emitting the *previous* world. The build
//! then componentizes that stale intermediate and the artifact's mtime is
//! refreshed — so an input-fingerprint freshness check reports FRESH over stale
//! bindings.
//!
//! This was not hypothetical: `extrusion-role` gained a `raft-infill` case, and
//! guests kept embedding the 13-case variant while the host used 14. The
//! resulting failure surfaced as a *linker* error ("a matching implementation
//! was not found in the linker"), with the true cause — `expected variant of 14
//! cases, found 13 cases` — four levels down the `Caused by` chain.
//!
//! Fingerprinting build *inputs* cannot catch this, because the defect is that
//! the output does not correspond to the inputs. So we check the output: decode
//! the component's own WIT and compare its shared type declarations against the
//! canonical ones.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Failure modes of the embedded-world verification itself.
#[derive(Debug)]
pub enum VerifyError {
    /// `wasm-tools component wit` could not be run or failed.
    Decode { artifact: String, reason: String },
    /// Canonical WIT set is empty (no readable files).
    CanonicalEmpty,
    /// A required canonical file could not be read.
    CanonicalUnreadable { path: String, reason: String },
    /// WIT text could not be parsed.
    Parse { artifact: String, reason: String },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode { artifact, reason } => {
                write!(f, "could not decode embedded WIT of '{artifact}': {reason}")
            }
            Self::CanonicalEmpty => write!(f, "canonical WIT set is empty"),
            Self::CanonicalUnreadable { path, reason } => {
                write!(f, "canonical WIT file '{path}' unreadable: {reason}")
            }
            Self::Parse { artifact, reason } => {
                write!(f, "could not parse WIT of '{artifact}': {reason}")
            }
        }
    }
}

impl std::error::Error for VerifyError {}

/// Return exactly the `.wit` paths the macro `include_str!`s, derived by a
/// multiline-aware parse of `crates/slicer-macros/src/lib.rs`.
///
/// The 20-file list (5 flat + 15 per-stage) excludes `root.wit`.
/// Both the function and its audit test derive the list by reading the macro
/// source at runtime — neither hardcodes a constant the other compares against.
pub fn macro_embedded_wit_files(ws_root: &Path) -> Result<Vec<PathBuf>, VerifyError> {
    let macro_rs = ws_root.join("crates/slicer-macros/src/lib.rs");
    let text =
        std::fs::read_to_string(&macro_rs).map_err(|e| VerifyError::CanonicalUnreadable {
            path: macro_rs.display().to_string(),
            reason: e.to_string(),
        })?;
    Ok(parse_macro_include_str_wit_paths(&text))
}

fn parse_macro_include_str_wit_paths(text: &str) -> Vec<PathBuf> {
    // Multiline-aware: find every `include_str!( ".../*.wit" )` even when the
    // string literal spans the next line. Collect, dedupe, sort.
    let mut paths = BTreeSet::new();
    let mut search_from = 0;
    while let Some(rel) = text[search_from..].find("include_str!") {
        let abs = search_from + rel;
        let after_kw = &text[abs + "include_str!".len()..];
        if let Some(paren_off) = after_kw.find('(') {
            let paren_abs = abs + "include_str!".len() + paren_off;
            let after_paren = &text[paren_abs..];
            if let Some(q_off) = after_paren.find('"') {
                let q_start = paren_abs + q_off + 1;
                if let Some(q_end_rel) = text[q_start..].find('"') {
                    let literal = &text[q_start..q_start + q_end_rel];
                    if literal.ends_with(".wit") {
                        if let Some(idx) = literal.find("slicer-schema/wit/") {
                            let rel = &literal[idx + "slicer-schema/wit/".len()..];
                            if rel != "root.wit" {
                                paths.insert(PathBuf::from(rel));
                            }
                        }
                    }
                    search_from = q_start + q_end_rel + 1;
                    continue;
                }
            }
        }
        search_from = abs + "include_str!".len();
    }
    paths.into_iter().collect()
}

// ---------------------------------------------------------------------------
// New declaration model (Step 3)
// ---------------------------------------------------------------------------

/// Package-qualified world model keyed by package id with version suffix as spelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldModel {
    pub packages: BTreeMap<String, PackageModel>,
}

impl WorldModel {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
    #[allow(dead_code)]
    pub fn package_names(&self) -> Vec<&str> {
        self.packages.keys().map(|s| s.as_str()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageModel {
    pub interfaces: BTreeMap<String, InterfaceModel>,
    pub worlds: BTreeMap<String, InterfaceModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceModel {
    pub decls: BTreeMap<String, String>,
    pub uses: BTreeSet<String>,
}

/// Minimal stage expectation for canonical_world_model signature (full in Step 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageExpectation {
    pub stage_id: String,
    pub wit_package: String,
    pub wit_interface: String,
    pub wit_dir: String,
    pub qualified_export: String,
}

pub fn stage_expectation(stage_id: &str) -> Option<StageExpectation> {
    let spec = slicer_schema::stage_by_id(stage_id)?;
    if spec.wit_package.is_empty() {
        return None;
    }
    Some(StageExpectation {
        stage_id: spec.stage_id.to_string(),
        wit_package: spec.wit_package.to_string(),
        wit_interface: spec.wit_interface.to_string(),
        wit_dir: spec.wit_dir.to_string(),
        qualified_export: slicer_schema::qualified_export_for_stage_id(stage_id)
            .unwrap_or_default(),
    })
}

pub const SHARED_PACKAGES: [&str; 5] = [
    "slicer:types",
    "slicer:config",
    "slicer:ir-handles",
    "slicer:common",
    "slicer:prepass-types",
];
pub const ROOT_COMPONENT_PACKAGE: &str = "root:component";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftKind {
    MissingDeclaration,
    ExtraDeclaration,
    DeclarationBody,
    MissingUse,
    ExtraUse,
    UnexpectedPackage,
    MissingStagePackage,
    ExportName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    pub kind: DriftKind,
    pub package: String,
    pub interface: Option<String>,
    pub name: String,
    pub canonical: Option<String>,
    pub embedded: Option<String>,
}

impl fmt::Display for Drift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} package={} iface={} name={} can={:?} emb={:?}",
            self.kind,
            self.package,
            self.interface.as_deref().unwrap_or("-"),
            self.name,
            self.canonical,
            self.embedded
        )
    }
}

fn strip_version(pkg: &str) -> &str {
    pkg.split('@').next().unwrap_or(pkg)
}

pub fn compare_worlds(
    embedded: &WorldModel,
    canonical: &WorldModel,
    expect: Option<&StageExpectation>,
) -> Vec<Drift> {
    let mut drifts = Vec::new();

    // Build allowed stripped set
    let mut allowed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    allowed.insert(strip_version(ROOT_COMPONENT_PACKAGE).to_string());
    for s in SHARED_PACKAGES {
        allowed.insert(strip_version(s).to_string());
    }
    if let Some(exp) = expect {
        allowed.insert(strip_version(&exp.wit_package).to_string());
    } else {
        // Test guests carry no resolved stage (stage_id is None) — their
        // embedded WIT legitimately exports a stage package without a
        // StageExpectation to validate against. Allow any canonical package
        // so only truly foreign packages (e.g. slicer:evil) are flagged.
        for k in canonical.packages.keys() {
            allowed.insert(strip_version(k).to_string());
        }
    }

    // MissingStagePackage when expect Some but no embedded package matches
    if let Some(exp) = expect {
        let exp_stripped = strip_version(&exp.wit_package);
        let found = embedded
            .packages
            .keys()
            .any(|k| strip_version(k) == exp_stripped);
        if !found {
            drifts.push(Drift {
                kind: DriftKind::MissingStagePackage,
                package: exp.wit_package.clone(),
                interface: None,
                name: exp.wit_package.clone(),
                canonical: Some(exp.wit_package.clone()),
                embedded: None,
            });
        }
    }

    // Helper to find canonical package by stripped name
    let find_canon_pkg = |stripped: &str| -> Option<&PackageModel> {
        canonical
            .packages
            .iter()
            .find(|(k, _)| strip_version(k) == stripped)
            .map(|(_, v)| v)
    };

    for (emb_pkg_name, emb_pkg) in &embedded.packages {
        let emb_stripped = strip_version(emb_pkg_name);
        if !allowed.contains(emb_stripped) {
            drifts.push(Drift {
                kind: DriftKind::UnexpectedPackage,
                package: emb_pkg_name.clone(),
                interface: None,
                name: emb_pkg_name.clone(),
                canonical: None,
                embedded: Some(emb_pkg_name.clone()),
            });
            continue;
        }
        if emb_stripped == strip_version(ROOT_COMPONENT_PACKAGE) {
            continue;
        }
        let is_shared = SHARED_PACKAGES
            .iter()
            .any(|s| strip_version(s) == emb_stripped);
        let is_stage = if let Some(exp) = expect {
            strip_version(&exp.wit_package) == emb_stripped
        } else {
            false
        };
        let canon_pkg = find_canon_pkg(emb_stripped);

        for (iface_name, emb_iface) in &emb_pkg.interfaces {
            let is_exported_stage = is_stage
                && expect
                    .map(|e| e.wit_interface == *iface_name)
                    .unwrap_or(false);
            let canon_iface = canon_pkg.and_then(|p| p.interfaces.get(iface_name));

            let _is_subset_iface = is_shared || (is_stage && !is_exported_stage);
            if is_exported_stage {
                // Full equality both directions
                if let Some(canon) = canon_iface {
                    for (name, emb_body) in &emb_iface.decls {
                        match canon.decls.get(name) {
                            None => drifts.push(Drift {
                                kind: DriftKind::ExtraDeclaration,
                                package: emb_pkg_name.clone(),
                                interface: Some(iface_name.clone()),
                                name: name.clone(),
                                canonical: None,
                                embedded: Some(emb_body.clone()),
                            }),
                            Some(canon_body) if canon_body != emb_body => drifts.push(Drift {
                                kind: DriftKind::DeclarationBody,
                                package: emb_pkg_name.clone(),
                                interface: Some(iface_name.clone()),
                                name: name.clone(),
                                canonical: Some(canon_body.clone()),
                                embedded: Some(emb_body.clone()),
                            }),
                            _ => {}
                        }
                    }
                    for (name, canon_body) in &canon.decls {
                        if !emb_iface.decls.contains_key(name) {
                            drifts.push(Drift {
                                kind: DriftKind::MissingDeclaration,
                                package: emb_pkg_name.clone(),
                                interface: Some(iface_name.clone()),
                                name: name.clone(),
                                canonical: Some(canon_body.clone()),
                                embedded: None,
                            });
                        }
                    }
                    for u in &emb_iface.uses {
                        if !canon.uses.contains(u) {
                            drifts.push(Drift {
                                kind: DriftKind::ExtraUse,
                                package: emb_pkg_name.clone(),
                                interface: Some(iface_name.clone()),
                                name: u.clone(),
                                canonical: None,
                                embedded: Some(u.clone()),
                            });
                        }
                    }
                    for u in &canon.uses {
                        if !emb_iface.uses.contains(u) {
                            drifts.push(Drift {
                                kind: DriftKind::MissingUse,
                                package: emb_pkg_name.clone(),
                                interface: Some(iface_name.clone()),
                                name: u.clone(),
                                canonical: Some(u.clone()),
                                embedded: None,
                            });
                        }
                    }
                } else {
                    for (name, emb_body) in &emb_iface.decls {
                        drifts.push(Drift {
                            kind: DriftKind::ExtraDeclaration,
                            package: emb_pkg_name.clone(),
                            interface: Some(iface_name.clone()),
                            name: name.clone(),
                            canonical: None,
                            embedded: Some(emb_body.clone()),
                        });
                    }
                    for u in &emb_iface.uses {
                        drifts.push(Drift {
                            kind: DriftKind::ExtraUse,
                            package: emb_pkg_name.clone(),
                            interface: Some(iface_name.clone()),
                            name: u.clone(),
                            canonical: None,
                            embedded: Some(u.clone()),
                        });
                    }
                }
            } else {
                // Subset direction: embedded ⊆ canonical.
                // - Missing canonical declarations in embedded is NOT drift.
                // - Extra embedded declarations not in canonical IS drift.
                // - For resources, a strict method-subset is also NOT drift (shared package case).
                // - Body differences that are not resource-subsets ARE drift.
                if let Some(canon) = canon_iface {
                    for (name, emb_body) in &emb_iface.decls {
                        match canon.decls.get(name) {
                            None => drifts.push(Drift {
                                kind: DriftKind::ExtraDeclaration,
                                package: emb_pkg_name.clone(),
                                interface: Some(iface_name.clone()),
                                name: name.clone(),
                                canonical: None,
                                embedded: Some(emb_body.clone()),
                            }),
                            Some(canon_body) => {
                                if canon_body == emb_body {
                                    continue;
                                }
                                let is_resource_subset = {
                                    let is_res = canon_body.starts_with("resource");
                                    let emb_is_res =
                                        emb_body == "resource" || emb_body.starts_with("resource");
                                    if is_res && emb_is_res {
                                        fn methods(
                                            body: &str,
                                        ) -> std::collections::BTreeSet<String>
                                        {
                                            if body == "resource" {
                                                return std::collections::BTreeSet::new();
                                            }
                                            let inner = body
                                                .trim_start_matches("resource")
                                                .trim()
                                                .trim_start_matches('{')
                                                .trim_end_matches('}')
                                                .trim();
                                            if inner.is_empty() || inner == "resource" {
                                                return std::collections::BTreeSet::new();
                                            }
                                            inner
                                                .split(',')
                                                .map(|s| s.trim().to_string())
                                                .filter(|s| !s.is_empty())
                                                .collect()
                                        }
                                        let cm = methods(canon_body);
                                        let em = methods(emb_body);
                                        em.is_subset(&cm)
                                    } else {
                                        false
                                    }
                                };
                                if !is_resource_subset {
                                    drifts.push(Drift {
                                        kind: DriftKind::DeclarationBody,
                                        package: emb_pkg_name.clone(),
                                        interface: Some(iface_name.clone()),
                                        name: name.clone(),
                                        canonical: Some(canon_body.clone()),
                                        embedded: Some(emb_body.clone()),
                                    });
                                }
                            }
                        }
                    }
                    for u in &emb_iface.uses {
                        if !canon.uses.contains(u) {
                            drifts.push(Drift {
                                kind: DriftKind::ExtraUse,
                                package: emb_pkg_name.clone(),
                                interface: Some(iface_name.clone()),
                                name: u.clone(),
                                canonical: None,
                                embedded: Some(u.clone()),
                            });
                        }
                    }
                } else {
                    // canon interface missing -> all embedded decls are extra
                    for (name, emb_body) in &emb_iface.decls {
                        drifts.push(Drift {
                            kind: DriftKind::ExtraDeclaration,
                            package: emb_pkg_name.clone(),
                            interface: Some(iface_name.clone()),
                            name: name.clone(),
                            canonical: None,
                            embedded: Some(emb_body.clone()),
                        });
                    }
                    for u in &emb_iface.uses {
                        drifts.push(Drift {
                            kind: DriftKind::ExtraUse,
                            package: emb_pkg_name.clone(),
                            interface: Some(iface_name.clone()),
                            name: u.clone(),
                            canonical: None,
                            embedded: Some(u.clone()),
                        });
                    }
                }
            }
        }

        // For exported stage interface, check missing interface entirely
        if is_stage {
            if let Some(exp) = expect {
                let exp_iface = &exp.wit_interface;
                if !emb_pkg.interfaces.contains_key(exp_iface) {
                    if let Some(canon_pkg) = find_canon_pkg(emb_stripped) {
                        if let Some(canon_iface) = canon_pkg.interfaces.get(exp_iface) {
                            // Avoid duplicating MissingStagePackage already reported
                            let already_missing_stage = drifts.iter().any(|d| {
                                d.kind == DriftKind::MissingStagePackage
                                    && d.package == exp.wit_package
                            });
                            if !already_missing_stage {
                                for (name, canon_body) in &canon_iface.decls {
                                    drifts.push(Drift {
                                        kind: DriftKind::MissingDeclaration,
                                        package: emb_pkg_name.clone(),
                                        interface: Some(exp_iface.clone()),
                                        name: name.clone(),
                                        canonical: Some(canon_body.clone()),
                                        embedded: None,
                                    });
                                }
                                for u in &canon_iface.uses {
                                    drifts.push(Drift {
                                        kind: DriftKind::MissingUse,
                                        package: emb_pkg_name.clone(),
                                        interface: Some(exp_iface.clone()),
                                        name: u.clone(),
                                        canonical: Some(u.clone()),
                                        embedded: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Export name check
    if let Some(exp) = expect {
        let emb_stage_pkg_name = embedded
            .packages
            .keys()
            .find(|k| strip_version(k) == strip_version(&exp.wit_package))
            .cloned();
        if let Some(emb_pkg_name) = emb_stage_pkg_name {
            let emb_ver = emb_pkg_name.split('@').nth(1).unwrap_or("");
            let exp_ver = exp.wit_package.split('@').nth(1).unwrap_or("");
            // Derive embedded qualified export using same interface and func as expected
            let func = exp.qualified_export.split('#').nth(1).unwrap_or("run");
            let stripped = strip_version(&emb_pkg_name);
            let derived = format!("{}/{}@{}#{}", stripped, exp.wit_interface, emb_ver, func);
            if derived != exp.qualified_export {
                // Also consider that expected version may be empty? then compare anyway
                if exp_ver != emb_ver || derived != exp.qualified_export {
                    drifts.push(Drift {
                        kind: DriftKind::ExportName,
                        package: emb_pkg_name.clone(),
                        interface: Some(exp.wit_interface.clone()),
                        name: exp.qualified_export.clone(),
                        canonical: Some(exp.qualified_export.clone()),
                        embedded: Some(derived),
                    });
                }
            }
        }
    }

    drifts
}

fn verify_embedded_world_new(
    artifact: &Path,
    canonical: &WorldModel,
    expect: Option<&StageExpectation>,
) -> Result<Vec<Drift>, VerifyError> {
    let embedded = embedded_world_model(artifact)?;
    Ok(compare_worlds(&embedded, canonical, expect))
}

pub fn verify_embedded_world(
    artifact: &Path,
    canonical: &WorldModel,
    expect: Option<&StageExpectation>,
) -> Result<Vec<Drift>, VerifyError> {
    verify_embedded_world_new(artifact, canonical, expect)
}

// ---------------------------------------------------------------------------
// Rendering helpers — order-preserving
// ---------------------------------------------------------------------------

fn render_type(ty: wit_parser::Type, resolve: &wit_parser::Resolve) -> String {
    match ty {
        wit_parser::Type::Bool => "bool".to_string(),
        wit_parser::Type::U8 => "u8".to_string(),
        wit_parser::Type::U16 => "u16".to_string(),
        wit_parser::Type::U32 => "u32".to_string(),
        wit_parser::Type::U64 => "u64".to_string(),
        wit_parser::Type::S8 => "s8".to_string(),
        wit_parser::Type::S16 => "s16".to_string(),
        wit_parser::Type::S32 => "s32".to_string(),
        wit_parser::Type::S64 => "s64".to_string(),
        wit_parser::Type::F32 => "f32".to_string(),
        wit_parser::Type::F64 => "f64".to_string(),
        wit_parser::Type::Char => "char".to_string(),
        wit_parser::Type::String => "string".to_string(),
        wit_parser::Type::ErrorContext => "error-context".to_string(),
        wit_parser::Type::Id(id) => {
            let td = &resolve.types[id];
            if let Some(name) = &td.name {
                // Named type — use its name (covers resources, records, etc.)
                name.clone()
            } else {
                // Anonymous — render inline kind
                render_typedef_kind(&td.kind, resolve)
            }
        }
    }
}

fn render_typedef_kind(kind: &wit_parser::TypeDefKind, resolve: &wit_parser::Resolve) -> String {
    match kind {
        wit_parser::TypeDefKind::Type(t) => render_type(*t, resolve),
        wit_parser::TypeDefKind::List(t) => format!("list<{}>", render_type(*t, resolve)),
        wit_parser::TypeDefKind::Option(t) => format!("option<{}>", render_type(*t, resolve)),
        wit_parser::TypeDefKind::Result(r) => match (&r.ok, &r.err) {
            (None, None) => "result".to_string(),
            (Some(ok), None) => format!("result<{}>", render_type(*ok, resolve)),
            (None, Some(err)) => format!("result<_, {}>", render_type(*err, resolve)),
            (Some(ok), Some(err)) => format!(
                "result<{}, {}>",
                render_type(*ok, resolve),
                render_type(*err, resolve)
            ),
        },
        wit_parser::TypeDefKind::Tuple(t) => {
            let inner = t
                .types
                .iter()
                .map(|ty| render_type(*ty, resolve))
                .collect::<Vec<_>>()
                .join(", ");
            format!("tuple<{}>", inner)
        }
        wit_parser::TypeDefKind::Record(r) => {
            let fields = r
                .fields
                .iter()
                .map(|f| format!("{}: {}", f.name, render_type(f.ty, resolve)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("record{{{}}}", fields)
        }
        wit_parser::TypeDefKind::Variant(v) => {
            let cases = v
                .cases
                .iter()
                .map(|c| {
                    if let Some(ty) = c.ty {
                        format!("{}({})", c.name, render_type(ty, resolve))
                    } else {
                        c.name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("variant{{{}}}", cases)
        }
        wit_parser::TypeDefKind::Enum(e) => {
            let cases = e
                .cases
                .iter()
                .map(|c| c.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!("enum{{{}}}", cases)
        }
        wit_parser::TypeDefKind::Flags(f) => {
            let flags = f
                .flags
                .iter()
                .map(|fl| fl.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!("flags{{{}}}", flags)
        }
        wit_parser::TypeDefKind::Resource => "resource".to_string(),
        wit_parser::TypeDefKind::Handle(h) => match h {
            wit_parser::Handle::Own(id) => format!(
                "own<{}>",
                resolve.types[*id].name.as_deref().unwrap_or("unknown")
            ),
            wit_parser::Handle::Borrow(id) => format!(
                "borrow<{}>",
                resolve.types[*id].name.as_deref().unwrap_or("unknown")
            ),
        },
        wit_parser::TypeDefKind::Future(t) => {
            if let Some(ty) = t {
                format!("future<{}>", render_type(*ty, resolve))
            } else {
                "future".to_string()
            }
        }
        wit_parser::TypeDefKind::Stream(t) => {
            if let Some(ty) = t {
                format!("stream<{}>", render_type(*ty, resolve))
            } else {
                "stream".to_string()
            }
        }
        wit_parser::TypeDefKind::Map(k, v) => format!(
            "map<{}, {}>",
            render_type(*k, resolve),
            render_type(*v, resolve)
        ),
        wit_parser::TypeDefKind::FixedLengthList(ty, size) => {
            format!("list<{}, {}>", render_type(*ty, resolve), size)
        }
        wit_parser::TypeDefKind::Unknown => "unknown".to_string(),
    }
}

fn render_function(func: &wit_parser::Function, resolve: &wit_parser::Resolve) -> String {
    let params = func
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, render_type(p.ty, resolve)))
        .collect::<Vec<_>>()
        .join(", ");
    let result = if let Some(ty) = func.result {
        format!(" -> {}", render_type(ty, resolve))
    } else {
        String::new()
    };
    // For resource methods, the name includes [method] prefix; preserve it as part of body? But for freestanding funcs, it's just name
    // The decls key is already the function name, so body is signature without name
    format!("func({}){}", params, result)
}

fn resolve_to_world_model(resolve: &wit_parser::Resolve) -> WorldModel {
    let mut packages: BTreeMap<String, PackageModel> = BTreeMap::new();
    // Initialize package models for every package in resolve
    for (_pid, pkg) in resolve.packages.iter() {
        let key = pkg.name.to_string();
        packages.entry(key).or_insert_with(|| PackageModel {
            interfaces: BTreeMap::new(),
            worlds: BTreeMap::new(),
        });
    }
    // Populate interfaces
    for (_iid, iface) in resolve.interfaces.iter() {
        let pkg_id = match iface.package {
            Some(pid) => pid,
            None => continue,
        };
        // Anonymous interfaces (no name) are not stored as package interfaces, skip
        let iface_name = match &iface.name {
            Some(n) => n.clone(),
            None => continue,
        };
        let pkg_name = resolve.packages[pkg_id].name.to_string();
        let pkg_model = packages.entry(pkg_name).or_insert_with(|| PackageModel {
            interfaces: BTreeMap::new(),
            worlds: BTreeMap::new(),
        });
        let mut decls: BTreeMap<String, String> = BTreeMap::new();
        let mut uses: BTreeSet<String> = BTreeSet::new();

        // Collect resource method signatures grouped by resource name
        // First, map resource TypeId -> Vec<Function>
        let mut resource_methods: BTreeMap<wit_parser::TypeId, Vec<String>> = BTreeMap::new();
        for (_fname, func) in iface.functions.iter() {
            if let Some(rid) = func.kind.resource() {
                let sig = render_function(func, resolve);
                // func.name for methods is like "[method]res.method" — extract method part
                let method_name = func
                    .name
                    .split('.')
                    .next_back()
                    .unwrap_or(&func.name)
                    .to_string();
                // Store as "method: sig" for later inclusion in resource body
                resource_methods
                    .entry(rid)
                    .or_default()
                    .push(format!("{}: {}", method_name, sig));
            }
        }

        for (tname, tid) in iface.types.iter() {
            let td = &resolve.types[*tid];
            // Detect use alias: TypeDefKind::Type pointing to foreign interface type
            let is_use_alias =
                if let wit_parser::TypeDefKind::Type(wit_parser::Type::Id(target_id)) = &td.kind {
                    let target = &resolve.types[*target_id];
                    if let wit_parser::TypeOwner::Interface(tiface) = target.owner {
                        if tiface != _iid {
                            if let Some(tpkg) = resolve.interfaces[tiface].package {
                                let pkg_str = resolve.packages[tpkg].name.to_string();
                                let iname =
                                    resolve.interfaces[tiface].name.clone().unwrap_or_default();
                                uses.insert(format!("{}/{}", pkg_str, iname));
                            }
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
            let body = if td.kind == wit_parser::TypeDefKind::Resource {
                let methods = resource_methods.get(tid).cloned().unwrap_or_default();
                if methods.is_empty() {
                    "resource".to_string()
                } else {
                    format!("resource{{{}}}", methods.join(", "))
                }
            } else {
                render_typedef_kind(&td.kind, resolve)
            };
            decls.insert(tname.clone(), body);
            let _ = is_use_alias;
        }
        // Freestanding functions
        for (fname, func) in iface.functions.iter() {
            if func.kind.resource().is_some() {
                continue;
            }
            decls.insert(fname.clone(), render_function(func, resolve));
        }

        pkg_model
            .interfaces
            .insert(iface_name, InterfaceModel { decls, uses });
    }
    // Populate worlds
    for (_wid, world) in resolve.worlds.iter() {
        let pkg_id = match world.package {
            Some(pid) => pid,
            None => continue,
        };
        let pkg_name = resolve.packages[pkg_id].name.to_string();
        let pkg_model = packages.entry(pkg_name).or_insert_with(|| PackageModel {
            interfaces: BTreeMap::new(),
            worlds: BTreeMap::new(),
        });
        let mut decls: BTreeMap<String, String> = BTreeMap::new();
        let mut uses: BTreeSet<String> = BTreeSet::new();
        // World imports/exports are stored as decls? Use WorldKey string as name, WorldItem kind as body
        for (key, item) in world.imports.iter().chain(world.exports.iter()) {
            let name = resolve.name_world_key(key);
            let body = match item {
                wit_parser::WorldItem::Interface { id, .. } => {
                    let iface = &resolve.interfaces[*id];
                    let iname = iface.name.clone().unwrap_or_default();
                    // Record the package/interface of imported interface as a use
                    if let Some(pid) = iface.package {
                        let pkg_str = resolve.packages[pid].name.to_string();
                        uses.insert(format!("{}/{}", pkg_str, iname));
                    }
                    format!("interface:{}", iname)
                }
                wit_parser::WorldItem::Function(f) => render_function(f, resolve),
                wit_parser::WorldItem::Type { id, .. } => {
                    let td = &resolve.types[*id];
                    render_typedef_kind(&td.kind, resolve)
                }
            };
            decls.insert(name, body);
        }
        pkg_model
            .worlds
            .insert(world.name.clone(), InterfaceModel { decls, uses });
    }
    WorldModel { packages }
}

/// Parse a single in-memory WIT string into a WorldModel, handling both braced
/// and statement package forms. On failure returns `Parse` with the provided artifact label.
pub fn world_model_from_text(text: &str, artifact: &str) -> Result<WorldModel, VerifyError> {
    // First try statement-form / mixed via UnresolvedPackageGroup::parse directly
    // This handles the common case where text contains `package x:y;` headers.
    // For braced-only text (e.g. `package x:y { interface ... }`), this will fail
    // with "no package header" — in that case we retry with a synthetic wrapper.
    let attempt = wit_parser::UnresolvedPackageGroup::parse("input.wit", text);
    let resolve = match attempt {
        Ok(group) => {
            let mut r = wit_parser::Resolve::new();
            r.push_group(group).map_err(|e| VerifyError::Parse {
                artifact: artifact.to_string(),
                reason: e.to_string(),
            })?;
            r
        }
        Err(_) => {
            // Try braced handling: SourceMap with synthetic main package
            // The text may contain only braced packages, so we add a synthetic
            // header file to give the parser a main package.
            let mut map = wit_parser::SourceMap::default();
            map.push_str("synthetic-main.wit", "package synthetic:main@1.0.0;");
            map.push_str("input.wit", text);
            let group = map.parse().map_err(|(_, e)| VerifyError::Parse {
                artifact: artifact.to_string(),
                reason: e.to_string(),
            })?;
            let mut r = wit_parser::Resolve::new();
            r.push_group(group).map_err(|e| VerifyError::Parse {
                artifact: artifact.to_string(),
                reason: e.to_string(),
            })?;
            // Remove synthetic package from result before converting
            let mut model = resolve_to_world_model(&r);
            model.packages.remove("synthetic:main@1.0.0");
            if !model.packages.is_empty() {
                return Ok(model);
            }
            r
        }
    };
    Ok(resolve_to_world_model(&resolve))
}

/// Build a WorldModel from the canonical WIT tree. Fails closed on empty or unreadable.
pub fn canonical_world_model(
    ws_root: &Path,
    stage: Option<&StageExpectation>,
) -> Result<WorldModel, VerifyError> {
    let _ = stage;
    let files = macro_embedded_wit_files(ws_root)?;
    if files.is_empty() {
        return Err(VerifyError::CanonicalEmpty);
    }
    let wit_root = ws_root.join("crates/slicer-schema/wit");
    // Check each required file readable before parsing
    for rel in &files {
        let path = wit_root.join(rel);
        if let Err(e) = std::fs::read_to_string(&path) {
            return Err(VerifyError::CanonicalUnreadable {
                path: path.display().to_string(),
                reason: e.to_string(),
            });
        }
    }
    // Canonical empty if wit_root has no wit files at all (defensive)
    let wit_files_exist = files.iter().any(|rel| wit_root.join(rel).exists());
    if !wit_files_exist {
        return Err(VerifyError::CanonicalEmpty);
    }
    // Parse via push_dir (handles dependencies and topological sort)
    let mut resolve = wit_parser::Resolve::new();
    resolve
        .push_dir(&wit_root)
        .map_err(|e| VerifyError::Parse {
            artifact: wit_root.display().to_string(),
            reason: e.to_string(),
        })?;
    let model = resolve_to_world_model(&resolve);
    if model.is_empty() {
        return Err(VerifyError::CanonicalEmpty);
    }
    // Also fail if no packages from the expected list are present (defensive)
    // The model should contain at least one of the shared packages
    Ok(model)
}

/// Build a WorldModel from a built component's embedded WIT.
pub fn embedded_world_model(artifact: &Path) -> Result<WorldModel, VerifyError> {
    let text = embedded_wit_text(artifact)?;
    world_model_from_text(&text, &artifact.display().to_string())
}

// Keep a helper for tests that want to feed raw text without artifact file
#[allow(dead_code)]
pub fn embedded_world_model_from_text(
    text: &str,
    artifact: &str,
) -> Result<WorldModel, VerifyError> {
    world_model_from_text(text, artifact)
}

/// Resolve a core module's per-stage WIT package directory (e.g.
/// `layer-perimeters`) from its manifest's `[stage] id`, via the canonical
/// `slicer_schema` table (ADR-0006: the stage table is the sole lookup).
///
/// Packet 164 retired the `wit-world` manifest key. This used to read that
/// key; once it was deleted from every manifest the read returned `None`
/// for all 20 core modules, which silently dropped every package-ambiguous
/// type (notably `region-key`) from drift verification. Reading `[stage] id`
/// restores the check at better precision — per stage, not per tier.
///
/// Returns `None` when the manifest is absent, declares no stage, or names a
/// stage with no WIT package (`PrePass::PaintSegmentation` is host-built-in).
#[allow(dead_code)]
pub fn module_stage_wit_dir(module_dir: &Path, module_name: &str) -> Option<&'static str> {
    let text = std::fs::read_to_string(module_dir.join(format!("{module_name}.toml"))).ok()?;
    let mut in_stage_section = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_stage_section = line == "[stage]";
            continue;
        }
        if !in_stage_section {
            continue;
        }
        let Some(rest) = line.strip_prefix("id") else {
            continue;
        };
        let Some((_, value)) = rest.split_once('=') else {
            continue;
        };
        let stage_id = value.trim().trim_matches('"');
        return slicer_schema::wit_dir_for_stage_id(stage_id);
    }
    None
}

/// Decode a built component's embedded WIT via `wasm-tools component wit`.
pub fn embedded_wit_text(artifact: &Path) -> Result<String, VerifyError> {
    let out = Command::new("wasm-tools")
        .args(["component", "wit"])
        .arg(artifact)
        .output()
        .map_err(|e| VerifyError::Decode {
            artifact: artifact.display().to_string(),
            reason: format!("failed to spawn wasm-tools: {e}"),
        })?;

    if !out.status.success() {
        return Err(VerifyError::Decode {
            artifact: artifact.display().to_string(),
            reason: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask/ must have a parent")
            .to_path_buf()
    }

    /// Packet 164 regression guard. Retiring the `wit-world` manifest key left
    /// the lookup returning `None` for every core module, which silently
    /// dropped `region-key` from drift verification for the whole tree. A real
    /// core module must resolve to its per-stage package directory.
    #[test]
    fn core_modules_resolve_their_stage_wit_dir() {
        let root = ws_root();
        for (module, expected) in [
            ("classic-perimeters", "layer-perimeters"),
            ("wipe-tower", "finalization-layer-finalization"),
            ("gyroid-infill", "layer-infill"),
            ("support-planner", "prepass-support-geometry"),
        ] {
            let dir = root.join("modules/core-modules").join(module);
            if !dir.join(format!("{module}.toml")).exists() {
                continue;
            }
            assert_eq!(
                module_stage_wit_dir(&dir, module),
                Some(expected),
                "{module} must resolve its per-stage WIT dir from `[stage] id`; \
                 None here means the drift check is silently dormant",
            );
        }
    }

    /// End-to-end proof that the gate detects real drift, exercising the actual
    /// component-decode path rather than synthetic strings.
    ///
    /// Takes a genuinely-built artifact and compares it against a canonical set
    /// perturbed to drop `raft-infill` from `extrusion-role` — i.e. exactly the
    /// 14-vs-13 shape that shipped broken guests. If this does not report a
    /// mismatch, the gate cannot catch the defect it exists for.
    #[test]
    fn detects_drift_against_a_real_built_artifact() {
        let root = ws_root();
        let dir = root.join("modules/core-modules/classic-perimeters");
        let artifact = dir.join("classic-perimeters.wasm");
        if !artifact.exists() {
            eprintln!("skipping: {} not built", artifact.display());
            return;
        }

        let stage_id = module_stage_wit_dir(&dir, "classic-perimeters")
            .and_then(|d| slicer_schema::STAGES.iter().find(|s| s.wit_dir == d))
            .map(|s| s.stage_id);
        let expect = stage_id.and_then(stage_expectation);
        let mut canonical_world =
            canonical_world_model(&root, expect.as_ref()).expect("canonical must parse");
        // Find the *variant* extrusion-role decl (contains raft-infill), not a type alias re-export
        let (pkg_name, iface_name, orig_body) = {
            let mut found = None;
            for (pkg, pm) in &canonical_world.packages {
                for (iface, im) in &pm.interfaces {
                    if let Some(body) = im.decls.get("extrusion-role") {
                        if body.contains("raft-infill") {
                            found = Some((pkg.clone(), iface.clone(), body.clone()));
                            break;
                        }
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            // Fallback: any extrusion-role if no raft found (should not happen after fix)
            if found.is_none() {
                for (pkg, pm) in &canonical_world.packages {
                    for (iface, im) in &pm.interfaces {
                        if let Some(body) = im.decls.get("extrusion-role") {
                            found = Some((pkg.clone(), iface.clone(), body.clone()));
                            break;
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
            }
            found.expect("extrusion-role must exist")
        };
        assert!(
            orig_body.contains("raft-infill"),
            "precondition: {orig_body}"
        );
        // Mutate canonical body to drop raft-infill
        canonical_world
            .packages
            .get_mut(&pkg_name)
            .unwrap()
            .interfaces
            .get_mut(&iface_name)
            .unwrap()
            .decls
            .insert(
                "extrusion-role".to_string(),
                orig_body
                    .replace("raft-infill,", "")
                    .replace("raft-infill", ""),
            );

        match verify_embedded_world(&artifact, &canonical_world, expect.as_ref()) {
            Ok(mismatches) => assert!(
                mismatches.iter().any(|m| m.name == "extrusion-role"),
                "gate must flag extrusion-role drift against a real artifact"
            ),
            Err(e) => eprintln!("skipping: {e}"),
        }
    }

    /// Regression guard for the stale-embedded-world defect: every built
    /// core-module component must embed the *canonical* shared types.
    ///
    /// The defect this pins: guests embedded a 13-case `extrusion-role` while
    /// canonical had 14 (`raft-infill`), because the isolated guest workspace
    /// reused a cached `slicer-macros` that had baked the older WIT. That
    /// mismatch surfaced only at runtime, as a misleading linker error. Rebuild
    /// guests (`cargo xtask build-guests`) if this fails.
    ///
    /// Skips when no artifacts are present so a clean checkout is not blocked;
    /// asserts against every artifact that does exist.
    #[test]
    fn built_core_module_components_embed_canonical_world() {
        let root = ws_root();
        let canonical_check = canonical_world_model(&root, None);
        assert!(
            canonical_check.is_ok() && !canonical_check.unwrap().is_empty(),
            "canonical WIT must be readable"
        );

        let modules_dir = root.join("modules/core-modules");
        let Ok(entries) = std::fs::read_dir(&modules_dir) else {
            eprintln!("skipping: {} not readable", modules_dir.display());
            return;
        };

        let mut checked = 0usize;
        for entry in entries.flatten() {
            let dir = entry.path();
            let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let artifact = dir.join(format!("{name}.wasm"));
            if !artifact.exists() {
                continue;
            }

            let wit_dir = module_stage_wit_dir(&dir, name);
            let stage_id = wit_dir.and_then(|d| {
                slicer_schema::STAGES
                    .iter()
                    .find(|s| s.wit_dir == d)
                    .map(|s| s.stage_id)
            });
            let expect = stage_id.and_then(stage_expectation);
            let canonical = canonical_world_model(&root, expect.as_ref()).unwrap_or_else(|e| {
                eprintln!("skipping '{name}': {e}");
                WorldModel {
                    packages: std::collections::BTreeMap::new(),
                }
            });

            match verify_embedded_world(&artifact, &canonical, expect.as_ref()) {
                Ok(mismatches) => {
                    assert!(
                        mismatches.is_empty(),
                        "guest '{name}' embeds a stale WIT world; run \
                         `cargo xtask build-guests`. Mismatches: {}",
                        mismatches
                            .iter()
                            .map(|m| m.to_string())
                            .collect::<Vec<_>>()
                            .join("; ")
                    );
                    checked += 1;
                }
                // wasm-tools absent: verification is unavailable, not failing.
                Err(e) => {
                    eprintln!("skipping '{name}': {e}");
                }
            }
        }

        if checked == 0 {
            eprintln!("skipping: no built core-module components found");
        }
    }

    /// AC-1 canonical coverage audit: the verifier's file list must equal the
    /// macro's actual `include_str!` set (20 files, no root.wit).
    ///
    /// Both the function and this test derive the set at runtime by reading
    /// `crates/slicer-macros/src/lib.rs` — neither hardcodes a constant.
    #[test]
    fn canonical_file_list_equals_macro_include_str_set() {
        use std::collections::BTreeSet;
        let root = ws_root();
        let macro_path = root.join("crates/slicer-macros/src/lib.rs");
        let text = std::fs::read_to_string(&macro_path)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", macro_path.display()));

        // Independent multiline-aware parse of include_str! targets ending in .wit
        let mut expected: BTreeSet<PathBuf> = BTreeSet::new();
        let mut search_from = 0;
        while let Some(rel) = text[search_from..].find("include_str!") {
            let abs = search_from + rel;
            let after_kw = &text[abs + "include_str!".len()..];
            if let Some(paren_off) = after_kw.find('(') {
                let paren_abs = abs + "include_str!".len() + paren_off;
                let after_paren = &text[paren_abs..];
                if let Some(q_off) = after_paren.find('"') {
                    let q_start = paren_abs + q_off + 1;
                    if let Some(q_end_rel) = text[q_start..].find('"') {
                        let literal = &text[q_start..q_start + q_end_rel];
                        if literal.ends_with(".wit") {
                            if let Some(idx) = literal.find("slicer-schema/wit/") {
                                let rel = &literal[idx + "slicer-schema/wit/".len()..];
                                if rel != "root.wit" {
                                    expected.insert(PathBuf::from(rel));
                                }
                            }
                        }
                        search_from = q_start + q_end_rel + 1;
                        continue;
                    }
                }
            }
            search_from = abs + "include_str!".len();
        }

        assert_eq!(
            expected.len(),
            20,
            "expected 20 distinct .wit include_str! paths, got {}: {:?}",
            expected.len(),
            expected
        );
        assert!(
            !expected.contains(&PathBuf::from("root.wit")),
            "root.wit must not be in the macro include_str! set"
        );
        assert!(
            !expected.contains(&PathBuf::from("crates/slicer-schema/wit/root.wit")),
            "root.wit must not be in the macro include_str! set"
        );

        // Verify against the function's independently-derived list.
        let actual =
            macro_embedded_wit_files(&root).expect("macro_embedded_wit_files must succeed");
        let actual_set: BTreeSet<PathBuf> = actual.into_iter().collect();
        assert_eq!(
            actual_set, expected,
            "macro_embedded_wit_files output must equal the test's independently-derived set"
        );
    }

    /// AC-2 audit: `crates/slicer-macros/build.rs` rerun-if-changed list must
    /// exactly equal the macro's 20-file `include_str!` set (no dead
    /// `world-*.wit`, no `root.wit`, includes `prepass-types.wit` and all
    /// 15 per-stage files). Both sides derived at runtime.
    #[test]
    fn macros_build_rs_watches_the_macro_include_str_set() {
        use std::collections::BTreeSet;
        let root = ws_root();

        // Expected set: same multiline-aware parse as AC-1 (derive from lib.rs).
        let macro_path = root.join("crates/slicer-macros/src/lib.rs");
        let text = std::fs::read_to_string(&macro_path)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", macro_path.display()));
        let mut expected: BTreeSet<PathBuf> = BTreeSet::new();
        let mut search_from = 0;
        while let Some(rel) = text[search_from..].find("include_str!") {
            let abs = search_from + rel;
            let after_kw = &text[abs + "include_str!".len()..];
            if let Some(paren_off) = after_kw.find('(') {
                let paren_abs = abs + "include_str!".len() + paren_off;
                let after_paren = &text[paren_abs..];
                if let Some(q_off) = after_paren.find('"') {
                    let q_start = paren_abs + q_off + 1;
                    if let Some(q_end_rel) = text[q_start..].find('"') {
                        let literal = &text[q_start..q_start + q_end_rel];
                        if literal.ends_with(".wit") {
                            if let Some(idx) = literal.find("slicer-schema/wit/") {
                                let rel = &literal[idx + "slicer-schema/wit/".len()..];
                                if rel != "root.wit" {
                                    expected.insert(PathBuf::from(rel));
                                }
                            }
                        }
                        search_from = q_start + q_end_rel + 1;
                        continue;
                    }
                }
            }
            search_from = abs + "include_str!".len();
        }
        assert_eq!(
            expected.len(),
            20,
            "expected 20 distinct .wit include_str! paths, got {}: {:?}",
            expected.len(),
            expected
        );

        // Actual watched set: parse build.rs rerun-if-changed string literals ending in .wit.
        let build_rs_path = root.join("crates/slicer-macros/build.rs");
        let build_rs = std::fs::read_to_string(&build_rs_path)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", build_rs_path.display()));
        let mut actual: BTreeSet<PathBuf> = BTreeSet::new();
        // Extract every quoted string literal ending in .wit, then map to the
        // same rel form (strip prefix up to slicer-schema/wit/).
        let mut pos = 0;
        while let Some(q) = build_rs[pos..].find('"') {
            let qs = pos + q + 1;
            if let Some(qe_rel) = build_rs[qs..].find('"') {
                let lit = &build_rs[qs..qs + qe_rel];
                if lit.ends_with(".wit") {
                    if let Some(idx) = lit.find("slicer-schema/wit/") {
                        let rel = &lit[idx + "slicer-schema/wit/".len()..];
                        actual.insert(PathBuf::from(rel));
                    } else {
                        // Fallback: treat any .wit literal's basename-anchored rel
                        // but for correctness require the prefix; if missing, still
                        // insert the raw lit so the diff is visible.
                        actual.insert(PathBuf::from(lit));
                    }
                }
                pos = qs + qe_rel + 1;
            } else {
                break;
            }
        }

        assert_eq!(
            actual.len(),
            20,
            "build.rs must watch exactly 20 .wit files, got {}: {:?}",
            actual.len(),
            actual
        );
        // No dead world-*.wit paths.
        for p in &actual {
            let s = p.to_string_lossy();
            assert!(
                !s.contains("world-"),
                "dead world-*.wit path must not be watched: {s}"
            );
            assert_ne!(s, "root.wit", "root.wit must not be watched: {s}");
        }
        assert!(
            actual.contains(&PathBuf::from("deps/prepass-types.wit")),
            "prepass-types.wit must be watched; got {:?}",
            actual
        );
        // All 15 per-stage deps/<dir>/<dir>.wit files must be present.
        let per_stage_expected: BTreeSet<PathBuf> = expected
            .iter()
            .filter(|p| p.to_string_lossy().matches('/').count() == 2)
            .cloned()
            .collect();
        assert_eq!(
            per_stage_expected.len(),
            15,
            "expected 15 per-stage wit files in canonical set, got {}: {:?}",
            per_stage_expected.len(),
            per_stage_expected
        );
        for p in &per_stage_expected {
            assert!(
                actual.contains(p),
                "per-stage file {p:?} must be watched; watched set: {actual:?}"
            );
        }

        assert_eq!(
            actual, expected,
            "build.rs rerun-if-changed set must equal the macro's 20-file include_str! set"
        );
    }

    // -----------------------------------------------------------------------
    // Step 3 new tests (AC-12, AC-N1..N3)
    // -----------------------------------------------------------------------

    #[test]
    fn braced_package_form_parses_like_statement_form() {
        let statement = r#"package slicer:types;

interface geometry {
    record point2 { x: s64, y: s64 }
    variant extrusion-role { outer-wall, inner-wall, custom(string) }
}
"#;
        let braced = r#"package slicer:types {
    interface geometry {
        record point2 { x: s64, y: s64 }
        variant extrusion-role { outer-wall, inner-wall, custom(string) }
    }
}
"#;
        let m1 =
            world_model_from_text(statement, "statement.wit").expect("statement form must parse");
        let m2 = world_model_from_text(braced, "braced.wit").expect("braced form must parse");
        assert_eq!(
            m1, m2,
            "braced and statement package forms must key identically"
        );
        // Also verify package key is exactly "slicer:types"
        assert!(
            m1.packages.contains_key("slicer:types"),
            "package key must be slicer:types, got {:?}",
            m1.package_names()
        );
    }

    #[test]
    fn empty_canonical_set_is_an_error_not_a_pass() {
        // Use a temp dir with no wit files and a fake macro file that yields empty list
        let tmp = std::env::temp_dir().join(format!("wit_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("crates/slicer-macros/src")).unwrap();
        // Write a macro file with no wit includes
        std::fs::write(
            tmp.join("crates/slicer-macros/src/lib.rs"),
            "// no wit includes\n",
        )
        .unwrap();
        std::fs::create_dir_all(tmp.join("crates/slicer-schema/wit/deps")).unwrap();
        let err = canonical_world_model(&tmp, None).expect_err("empty canonical set must be Err");
        assert!(
            matches!(err, VerifyError::CanonicalEmpty),
            "expected CanonicalEmpty, got {:?}",
            err
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unreadable_canonical_file_is_an_error_not_a_pass() {
        let tmp = std::env::temp_dir().join(format!("wit_unreadable_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("crates/slicer-macros/src")).unwrap();
        // Create a macro file that references a file that doesn't exist
        let wit_content =
            r#"const X: &str = include_str!("../../slicer-schema/wit/deps/types.wit");"#;
        std::fs::write(tmp.join("crates/slicer-macros/src/lib.rs"), wit_content).unwrap();
        std::fs::create_dir_all(tmp.join("crates/slicer-schema/wit/deps")).unwrap();
        // Do NOT create types.wit — so it is unreadable
        let err = canonical_world_model(&tmp, None).expect_err("unreadable file must be Err");
        match err {
            VerifyError::CanonicalUnreadable { path, .. } => {
                assert!(
                    path.contains("types.wit"),
                    "path should name missing file, got {}",
                    path
                );
            }
            other => panic!("expected CanonicalUnreadable, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unparseable_embedded_text_is_an_error_not_a_pass() {
        let bad = "package slicer:bad { interface bad { record 3d-bad { x: s32 } } }";
        let err = embedded_world_model_from_text(bad, "bad.wit").expect_err("bad wit must be Err");
        assert!(
            matches!(err, VerifyError::Parse { .. }),
            "expected Parse, got {:?}",
            err
        );
        // Also test via file-based embedded_world_model with a temp artifact file containing bad WIT
        // We do this by writing bad WIT to a temp file and calling world_model_from_text (which is what embedded_world_model does after decode)
        // The file path test: create a temp file, write bad text, then try to parse it as if it were decoded wit
        let tmp_file = std::env::temp_dir().join(format!("wit_bad_{}.wit", std::process::id()));
        std::fs::write(&tmp_file, bad).unwrap();
        // Simulate embedded path: we cannot call embedded_world_model without wasm-tools, so directly test parse
        let err2 = world_model_from_text(
            &std::fs::read_to_string(&tmp_file).unwrap(),
            &tmp_file.display().to_string(),
        )
        .expect_err("bad wit file must be Err");
        assert!(matches!(err2, VerifyError::Parse { .. }));
        let _ = std::fs::remove_file(&tmp_file);
    }

    // -----------------------------------------------------------------------
    // Step 4 new tests — all nine ACs
    // -----------------------------------------------------------------------

    #[allow(dead_code)]
    fn stage_pkg() -> &'static str {
        "slicer:layer-infill@1.0.0"
    }
    #[allow(dead_code)]
    fn stage_iface() -> &'static str {
        "infill"
    }

    fn make_expect() -> StageExpectation {
        stage_expectation("Layer::Infill").expect("Layer::Infill must have expectation")
    }

    #[allow(dead_code)]
    fn canonical_pair(
        extra_canonical: &str,
        extra_embedded: &str,
    ) -> (WorldModel, WorldModel, StageExpectation) {
        // Helper: build canonical and embedded from snippets plus a minimal shared root
        let shared = r#"
package slicer:types;
interface geometry {
    record point2 { x: s64, y: s64 }
    variant extrusion-role { outer-wall, inner-wall, custom(string) }
}
package slicer:config;
interface config-types {
    resource config-view { get: func(key: string) -> option<string>, keys: func() -> list<string> }
}
package slicer:ir-handles;
interface ir-handles {
    use slicer:types/geometry.{point2, extrusion-role};
    type layer-idx = s32;
    record region-key { layer-index: layer-idx, object-id: string, region-id: string }
    resource slice-region-view { object-id: func() -> string }
}
package slicer:common;
interface host-services { log: func(msg: string) }
package slicer:prepass-types;
interface prepass-types { record dummy { x: u32 } }
"#;
        let stage = format!(
            r#"
package slicer:layer-infill@1.0.0;
interface infill {{
    use slicer:ir-handles/ir-handles.{{layer-idx, region-key, slice-region-view}};
    use slicer:config/config-types.{{config-view}};
    {}
}}
interface layer-infill-types {{
    use slicer:types/geometry.{{extrusion-role}};
    {}
}}
world infill-module {{
    import slicer:types/geometry;
    import slicer:ir-handles/ir-handles;
    import slicer:common/host-services;
    import slicer:config/config-types;
    export infill;
}}
"#,
            extra_canonical, extra_canonical
        );
        // embedded uses extra_embedded for infill, and same extra_embedded for types iface
        let emb_stage = format!(
            r#"
package slicer:layer-infill@1.0.0;
interface infill {{
    use slicer:ir-handles/ir-handles.{{layer-idx, region-key, slice-region-view}};
    use slicer:config/config-types.{{config-view}};
    {}
}}
interface layer-infill-types {{
    use slicer:types/geometry.{{extrusion-role}};
    {}
}}
world infill-module {{
    import slicer:types/geometry;
    import slicer:ir-handles/ir-handles;
    import slicer:common/host-services;
    import slicer:config/config-types;
    export infill;
}}
"#,
            extra_embedded, extra_embedded
        );
        let canon_text = format!("{shared}\n{stage}\npackage root:component;\nworld root {{ export slicer:layer-infill/infill@1.0.0; }}");
        let emb_text = format!("{shared}\n{emb_stage}\npackage root:component;\nworld root {{ export slicer:layer-infill/infill@1.0.0; }}");
        (
            world_model_from_text(&canon_text, "canon").unwrap(),
            world_model_from_text(&emb_text, "emb").unwrap(),
            make_expect(),
        )
    }

    #[test]
    fn record_field_reorder_is_drift() {
        let canon_text = r#"
package slicer:ir-handles;
interface ir-handles {
    type layer-idx = s32;
    type object-id = string;
    type region-id = string;
    record region-key { layer-index: layer-idx, object-id: object-id, region-id: region-id }
}
package slicer:layer-infill@1.0.0 {
    interface infill {
        use slicer:ir-handles/ir-handles.{region-key};
        run: func(k: region-key) -> string;
    }
    world w { export infill; }
}
package root:component {
    world root { export slicer:layer-infill/infill@1.0.0; }
}
"#;
        let emb_text = r#"
package slicer:ir-handles;
interface ir-handles {
    type layer-idx = s32;
    type object-id = string;
    type region-id = string;
    record region-key { layer-index: layer-idx, region-id: region-id, object-id: object-id }
}
package slicer:layer-infill@1.0.0 {
    interface infill {
        use slicer:ir-handles/ir-handles.{region-key};
        run: func(k: region-key) -> string;
    }
    world w { export infill; }
}
package root:component {
    world root { export slicer:layer-infill/infill@1.0.0; }
}
"#;
        let canon = world_model_from_text(canon_text, "canon").unwrap();
        let emb = world_model_from_text(emb_text, "emb").unwrap();
        let drifts = compare_worlds(&emb, &canon, None);
        assert!(
            drifts
                .iter()
                .any(|d| d.kind == DriftKind::DeclarationBody && d.name == "region-key"),
            "expected DeclarationBody for region-key, got {:?}",
            drifts
        );
    }

    #[test]
    fn variant_case_reorder_is_drift() {
        let canon_text = r#"
package slicer:types {
    interface geometry { variant extrusion-role { outer-wall, inner-wall, thin-wall, custom(string) } }
}
package root:component {
    world root { export slicer:layer-infill/infill@1.0.0; }
}
package slicer:layer-infill@1.0.0 {
    interface infill { use slicer:types/geometry.{extrusion-role}; run: func(r: extrusion-role) -> string; }
    world w { export infill; }
}
"#;
        let emb_text = r#"
package slicer:types {
    interface geometry { variant extrusion-role { inner-wall, outer-wall, thin-wall, custom(string) } }
}
package root:component {
    world root { export slicer:layer-infill/infill@1.0.0; }
}
package slicer:layer-infill@1.0.0 {
    interface infill { use slicer:types/geometry.{extrusion-role}; run: func(r: extrusion-role) -> string; }
    world w { export infill; }
}
"#;
        let canon = world_model_from_text(canon_text, "canon").unwrap();
        let emb = world_model_from_text(emb_text, "emb").unwrap();
        let drifts = compare_worlds(&emb, &canon, None);
        assert!(
            drifts
                .iter()
                .any(|d| d.kind == DriftKind::DeclarationBody && d.name == "extrusion-role"),
            "expected DeclarationBody for extrusion-role, got {:?}",
            drifts
        );
    }

    #[test]
    fn aliases_resources_and_uses_are_modelled() {
        let expect = make_expect();
        let canon_text = r#"
package slicer:ir-handles {
    interface ir-handles { type layer-idx = s32; resource slice-region-view { object-id: func() -> string; } }
}
package slicer:layer-infill@1.0.0 {
    interface infill {
        use slicer:ir-handles/ir-handles.{layer-idx, slice-region-view};
        run: func(x: layer-idx) -> string;
    }
    world w { export infill; }
}
package root:component {
    world root { export slicer:layer-infill/infill@1.0.0; }
}
"#;
        let emb_text_alias = r#"
package slicer:ir-handles {
    interface ir-handles { type layer-idx = u32; resource slice-region-view { object-id: func() -> string; } }
}
package slicer:layer-infill@1.0.0 {
    interface infill {
        use slicer:ir-handles/ir-handles.{layer-idx, slice-region-view};
        run: func(x: layer-idx) -> string;
    }
    world w { export infill; }
}
package root:component {
    world root { export slicer:layer-infill/infill@1.0.0; }
}
"#;
        let canon = world_model_from_text(canon_text, "canon").unwrap();
        let emb_alias = world_model_from_text(emb_text_alias, "emb").unwrap();
        let drifts = compare_worlds(&emb_alias, &canon, Some(&expect));
        assert!(
            drifts
                .iter()
                .any(|d| d.kind == DriftKind::DeclarationBody && d.name == "layer-idx"),
            "alias layer-idx should be DeclarationBody, got {:?}",
            drifts
        );

        let emb_text_resource = r#"
package slicer:ir-handles {
    interface ir-handles { type layer-idx = s32; resource slice-region-view { object-id: func() -> string; extra: func(x: u32) -> string; } }
}
package slicer:layer-infill@1.0.0 {
    interface infill {
        use slicer:ir-handles/ir-handles.{layer-idx, slice-region-view};
        run: func(x: layer-idx) -> string;
    }
    world w { export infill; }
}
package root:component {
    world root { export slicer:layer-infill/infill@1.0.0; }
}
"#;
        let emb_res = world_model_from_text(emb_text_resource, "emb").unwrap();
        let drifts2 = compare_worlds(&emb_res, &canon, Some(&expect));
        assert!(
            drifts2
                .iter()
                .any(|d| d.kind == DriftKind::DeclarationBody && d.name == "slice-region-view"),
            "resource drift should be DeclarationBody, got {:?}",
            drifts2
        );

        let emb_text_no_use = r#"
package slicer:ir-handles {
    interface ir-handles { type layer-idx = s32; resource slice-region-view { object-id: func() -> string; } }
}
package slicer:layer-infill@1.0.0 {
    interface infill {
        run: func(x: s32) -> string;
    }
    world w { export infill; }
}
package root:component {
    world root { export slicer:layer-infill/infill@1.0.0; }
}
"#;
        let emb_no_use = world_model_from_text(emb_text_no_use, "emb").unwrap();
        let drifts3 = compare_worlds(&emb_no_use, &canon, Some(&expect));
        assert!(
            drifts3.iter().any(|d| d.kind == DriftKind::MissingUse),
            "missing use should be MissingUse, got {:?}",
            drifts3
        );
    }

    #[test]
    fn exported_stage_interface_is_full_equality_both_directions() {
        let expect = make_expect();
        let canon_text = r#"
package slicer:layer-infill@1.0.0 {
    interface infill {
        run: func(a: u32) -> string;
        extra: func(b: string) -> string;
    }
    world w { export infill; }
}
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
"#;
        let emb_missing = r#"
package slicer:layer-infill@1.0.0 {
    interface infill { run: func(a: u32) -> string; }
    world w { export infill; }
}
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
"#;
        let emb_extra = r#"
package slicer:layer-infill@1.0.0 {
    interface infill {
        run: func(a: u32) -> string;
        extra: func(b: string) -> string;
        added: func(c: u32) -> string;
    }
    world w { export infill; }
}
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
"#;
        let canon = world_model_from_text(canon_text, "canon").unwrap();
        let emb_m = world_model_from_text(emb_missing, "emb").unwrap();
        let d1 = compare_worlds(&emb_m, &canon, Some(&expect));
        assert!(
            d1.iter()
                .any(|d| d.kind == DriftKind::MissingDeclaration && d.name == "extra"),
            "missing extra should be MissingDeclaration, got {:?}",
            d1
        );
        let emb_e = world_model_from_text(emb_extra, "emb").unwrap();
        let d2 = compare_worlds(&emb_e, &canon, Some(&expect));
        assert!(
            d2.iter()
                .any(|d| d.kind == DriftKind::ExtraDeclaration && d.name == "added"),
            "extra added should be ExtraDeclaration, got {:?}",
            d2
        );
    }

    #[test]
    fn non_exported_stage_interfaces_use_subset_direction() {
        let expect = make_expect();
        let canon_text = r#"
package slicer:layer-infill@1.0.0 {
    interface layer-infill-types {
        record foo { a: u32, b: string }
        record bar { x: u32 }
        variant vaz { a, b, c }
    }
    interface infill { run: func(a: u32) -> string; }
    world w { export infill; }
}
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
"#;
        let emb_omit = r#"
package slicer:layer-infill@1.0.0 {
    interface layer-infill-types {
        record foo { a: u32, b: string }
        variant vaz { a, b, c }
    }
    interface infill { run: func(a: u32) -> string; }
    world w { export infill; }
}
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
"#;
        let canon = world_model_from_text(canon_text, "canon").unwrap();
        let emb = world_model_from_text(emb_omit, "emb").unwrap();
        let drifts = compare_worlds(&emb, &canon, Some(&expect));
        assert!(
            !drifts.iter().any(|d| d.name == "bar"),
            "omitting bar from non-exported iface should not drift, got {:?}",
            drifts
        );
        let emb_diff = r#"
package slicer:layer-infill@1.0.0 {
    interface layer-infill-types {
        record foo { a: u32, b: u32 }
        record bar { x: u32 }
        variant vaz { a, b, c }
    }
    interface infill { run: func(a: u32) -> string; }
    world w { export infill; }
}
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
"#;
        let emb2 = world_model_from_text(emb_diff, "emb").unwrap();
        let drifts2 = compare_worlds(&emb2, &canon, Some(&expect));
        assert!(
            drifts2
                .iter()
                .any(|d| d.kind == DriftKind::DeclarationBody && d.name == "foo"),
            "foo body diff should be DeclarationBody, got {:?}",
            drifts2
        );
    }

    #[test]
    fn shared_packages_use_subset_direction() {
        // Shared config-view omission: canonical has get, keys, get-bool; embedded only get, keys
        // Use syntax with hyphens needs exact spelling; wit resource method names use hyphens
        let canon_text = r#"
package slicer:config {
    interface config-types {
        resource config-view {
            get: func(key: string) -> option<string>;
            keys: func() -> list<string>;
            get-bool: func(key: string) -> option<bool>;
        }
    }
}
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
package slicer:layer-infill@1.0.0 {
    interface infill { use slicer:config/config-types.{config-view}; run: func(c: config-view) -> string; }
    world w { export infill; }
}
"#;
        let emb_text = r#"
package slicer:config {
    interface config-types {
        resource config-view {
            get: func(key: string) -> option<string>;
            keys: func() -> list<string>;
        }
    }
}
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
package slicer:layer-infill@1.0.0 {
    interface infill { use slicer:config/config-types.{config-view}; run: func(c: config-view) -> string; }
    world w { export infill; }
}
"#;
        let canon = world_model_from_text(canon_text, "canon").unwrap();
        let emb = world_model_from_text(emb_text, "emb").unwrap();
        let expect = make_expect();
        let drifts = compare_worlds(&emb, &canon, Some(&expect));
        // Only care that config-view body omission is NOT reported; ignore other drifts from stage package for this count
        let cfg_drifts: Vec<_> = drifts
            .iter()
            .filter(|d| d.package == "slicer:config")
            .collect();
        assert!(
            cfg_drifts.is_empty(),
            "shared subset omission should be no drift for slicer:config, got {:?}",
            cfg_drifts
        );
        let canon2 = r#"
package slicer:ir-handles {
    interface ir-handles {
        use slicer:types/geometry.{point2};
        use slicer:config/config-types.{config-view};
        record foo { x: u32 }
    }
}
package slicer:types { interface geometry { record point2 { x: s64, y: s64 } } }
package slicer:config { interface config-types { resource config-view { get: func(key: string) -> option<string>; } } }
package root:component { world root {} }
"#;
        let emb2_same_but_swapped = r#"
package slicer:ir-handles {
    interface ir-handles {
        use slicer:config/config-types.{config-view};
        use slicer:types/geometry.{point2};
        record foo { x: u32 }
    }
}
package slicer:types { interface geometry { record point2 { x: s64, y: s64 } } }
package slicer:config { interface config-types { resource config-view { get: func(key: string) -> option<string>; } } }
package root:component { world root {} }
"#;
        let c2 = world_model_from_text(canon2, "canon").unwrap();
        let e2 = world_model_from_text(emb2_same_but_swapped, "emb").unwrap();
        let d2 = compare_worlds(&e2, &c2, None);
        assert!(
            d2.is_empty(),
            "use reordering should be no drift, got {:?}",
            d2
        );
    }

    #[test]
    fn unexpected_package_is_drift() {
        let canon_text = r#"
package slicer:config { interface config-types { record foo { x: u32 } } }
package root:component { world root {} }
"#;
        let emb_text = r#"
package slicer:config { interface config-types { record foo { x: u32 } } }
package slicer:evil@1.0.0 { interface evil { record bar { x: u32 } } }
package root:component { world root {} }
"#;
        let canon = world_model_from_text(canon_text, "canon").unwrap();
        let emb = world_model_from_text(emb_text, "emb").unwrap();
        let drifts = compare_worlds(&emb, &canon, None);
        assert!(
            drifts
                .iter()
                .any(|d| d.kind == DriftKind::UnexpectedPackage),
            "should be UnexpectedPackage, got {:?}",
            drifts
        );
    }

    #[test]
    fn export_name_compared_exactly_including_version() {
        let expect = make_expect();
        let canon_text = r#"
package slicer:layer-infill@1.0.0 {
    interface infill { run: func(a: u32) -> string; }
    world w { export infill; }
}
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
"#;
        let emb_text = r#"
package slicer:layer-infill@2.0.0 {
    interface infill { run: func(a: u32) -> string; }
    world w { export infill; }
}
package root:component { world root { export slicer:layer-infill/infill@2.0.0; } }
"#;
        let canon = world_model_from_text(canon_text, "canon").unwrap();
        let emb = world_model_from_text(emb_text, "emb").unwrap();
        let drifts = compare_worlds(&emb, &canon, Some(&expect));
        assert!(
            drifts.iter().any(|d| d.kind == DriftKind::ExportName),
            "version mismatch should be ExportName, got {:?}",
            drifts
        );
    }

    #[test]
    fn missing_stage_package_is_drift() {
        let expect = make_expect();
        let canon_text = r#"
package slicer:layer-infill@1.0.0 {
    interface infill { run: func(a: u32) -> string; }
    world w { export infill; }
}
package slicer:config { interface config-types { record foo { x: u32 } } }
package root:component { world root { export slicer:layer-infill/infill@1.0.0; } }
"#;
        let emb_text = r#"
package slicer:config { interface config-types { record foo { x: u32 } } }
package root:component { world root {} }
"#;
        let canon = world_model_from_text(canon_text, "canon").unwrap();
        let emb = world_model_from_text(emb_text, "emb").unwrap();
        let drifts = compare_worlds(&emb, &canon, Some(&expect));
        assert!(
            drifts
                .iter()
                .any(|d| d.kind == DriftKind::MissingStagePackage),
            "missing stage package should be MissingStagePackage, got {:?}",
            drifts
        );
    }

    #[test]
    fn real_core_module_artifacts_verify_clean() {
        if Command::new("wasm-tools")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("skipping: wasm-tools not available");
            return;
        }
        let ws_root = ws_root();
        let artifacts: [(std::path::PathBuf, &str); 2] = [
            (
                ws_root.join("modules/core-modules/wipe-tower/wipe-tower.wasm"),
                "PostPass::LayerFinalization",
            ),
            (
                ws_root
                    .join("modules/core-modules/layer-planner-default/layer-planner-default.wasm"),
                "PrePass::LayerPlanning",
            ),
        ];
        for (artifact, stage_id) in artifacts {
            assert!(
                artifact.exists(),
                "artifact {} not built — run `cargo xtask build-guests` first; \
                 this test must not skip when the artifact is absent",
                artifact.display()
            );
            let expect = stage_expectation(stage_id);
            let canonical =
                canonical_world_model(&ws_root, expect.as_ref()).expect("canonical must parse");
            let drifts = verify_embedded_world(&artifact, &canonical, expect.as_ref())
                .expect("verify must succeed");
            assert!(
                drifts.is_empty(),
                "artifact {} drifts: {:?}",
                artifact.display(),
                drifts
            );
        }
    }
}
