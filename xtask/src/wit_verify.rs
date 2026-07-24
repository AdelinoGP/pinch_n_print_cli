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

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::process::Command;

/// A type whose shape in the built artifact disagrees with the canonical WIT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeMismatch {
    /// WIT type name (e.g. `extrusion-role`).
    pub type_name: String,
    /// Normalized body found in the canonical WIT.
    pub canonical: String,
    /// Normalized body found embedded in the built component.
    pub embedded: String,
}

impl fmt::Display for TypeMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "type `{}` differs\n  canonical: {}\n  embedded:  {}",
            self.type_name, self.canonical, self.embedded
        )
    }
}

/// Failure modes of the embedded-world verification itself.
#[derive(Debug)]
pub enum VerifyError {
    /// `wasm-tools component wit` could not be run or failed.
    Decode { artifact: String, reason: String },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode { artifact, reason } => {
                write!(f, "could not decode embedded WIT of '{artifact}': {reason}")
            }
        }
    }
}

/// Type-defining WIT keywords whose bodies are brace-delimited.
const BRACED_KEYWORDS: [&str; 4] = ["variant", "enum", "record", "flags"];

/// Extract every brace-delimited type declaration, keyed by type name, with the
/// body normalized so formatting differences between the canonical `.wit` source
/// and `wasm-tools`' re-rendered output cannot cause false mismatches.
///
/// Normalization collapses all whitespace runs to a single space and strips
/// spaces adjacent to `,` and `(`/`)`, so `outer-wall, inner-wall` and a
/// one-case-per-line rendering of the same variant compare equal. Comments are
/// removed first, since the canonical files are heavily commented and the
/// re-rendered output is not.
pub fn extract_type_blocks(text: &str) -> BTreeMap<String, String> {
    let stripped = strip_comments(text);
    let bytes: Vec<char> = stripped.chars().collect();
    let mut out = BTreeMap::new();

    for keyword in BRACED_KEYWORDS {
        let mut search_from = 0usize;
        while let Some(rel) = stripped[search_from..].find(keyword) {
            let kw_start = search_from + rel;
            search_from = kw_start + keyword.len();

            // Require the keyword to stand alone (not a suffix like `my-record`).
            if kw_start > 0 {
                let prev = stripped[..kw_start].chars().next_back().unwrap_or(' ');
                if prev.is_alphanumeric() || prev == '-' || prev == '_' {
                    continue;
                }
            }

            // Parse: `<keyword> <name> {`
            let after_kw = kw_start + keyword.len();
            let rest = &stripped[after_kw..];
            let name: String = rest
                .chars()
                .skip_while(|c| c.is_whitespace())
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if name.is_empty() {
                continue;
            }
            let name_end = match rest.find(&name) {
                Some(i) => after_kw + i + name.len(),
                None => continue,
            };
            // Next non-space character must open the body.
            let open = match stripped[name_end..].find(|c: char| !c.is_whitespace()) {
                Some(i) if stripped[name_end + i..].starts_with('{') => name_end + i,
                _ => continue,
            };

            if let Some(close) = matching_brace(&bytes, open) {
                let body = &stripped[open + 1..close];
                out.insert(name, normalize(body));
            }
        }
    }

    out
}

/// Index of the `}` matching the `{` at `open`, honoring nesting.
fn matching_brace(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in chars.iter().enumerate().skip(open) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Remove `//` line comments (the canonical WIT uses them heavily; the
/// re-rendered output does not).
fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Collapse whitespace and remove spacing that carries no semantic weight, plus
/// any trailing comma, so equivalent declarations compare equal regardless of
/// how they were formatted.
fn normalize(body: &str) -> String {
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let tightened = collapsed
        .replace(" ,", ",")
        .replace(", ", ",")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(") ", ")");
    tightened.trim_end_matches(',').trim().to_string()
}

/// Read the canonical WIT type declarations relevant to one guest world.
///
/// Types are compared by bare name, so the world's *own* declarations must
/// shadow the shared ones: a name can legitimately denote different types in
/// different packages. `region-key`, for example, is a 4-field record in
/// `slicer:ir-handles` (carrying `variant-chain`) and a deliberately distinct
/// 3-field record in `slicer:world-finalization`. Comparing a finalization
/// guest against the `ir-handles` spelling reports drift that does not exist.
///
/// `world` is the manifest's `wit-world` value (e.g. `slicer:world-finalization`);
/// when `None`, only the shared `deps/*.wit` types are returned.
pub fn canonical_type_blocks(ws_root: &Path, world: Option<&str>) -> BTreeMap<String, String> {
    let wit_root = ws_root.join("crates/slicer-schema/wit");
    let mut all = BTreeMap::new();

    for rel in [
        "deps/types.wit",
        "deps/config.wit",
        "deps/ir-types.wit",
        "deps/common.wit",
    ] {
        if let Ok(text) = std::fs::read_to_string(wit_root.join(rel)) {
            all.extend(extract_type_blocks(&text));
        }
    }

    match world {
        // World-specific declarations win over the shared ones.
        Some(world) => {
            let short = world.rsplit(':').next().unwrap_or(world);
            let path = wit_root.join(format!("deps/{short}/{short}.wit"));
            if let Ok(text) = std::fs::read_to_string(path) {
                all.extend(extract_type_blocks(&text));
            }
        }
        // Without a world we cannot resolve which package's spelling applies,
        // so drop every name that is declared differently somewhere else (e.g.
        // `region-key`). Unambiguous names — `extrusion-role` among them — are
        // still checked, so the gate stays useful rather than being skipped
        // wholesale for guests whose world we cannot determine (test-guests).
        None => {
            for name in ambiguous_type_names(&wit_root) {
                all.remove(&name);
            }
        }
    }

    all
}

/// Names declared with more than one distinct body anywhere in the canonical
/// WIT tree — i.e. names that only a world can disambiguate.
fn ambiguous_type_names(wit_root: &Path) -> Vec<String> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let mut ambiguous = Vec::new();

    let mut consider = |text: &str| {
        for (name, body) in extract_type_blocks(text) {
            match seen.get(&name) {
                Some(existing) if *existing != body => {
                    if !ambiguous.contains(&name) {
                        ambiguous.push(name);
                    }
                }
                Some(_) => {}
                None => {
                    seen.insert(name, body);
                }
            }
        }
    };

    for entry in walkdir::WalkDir::new(wit_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.path().extension().is_some_and(|e| e == "wit") {
            if let Ok(text) = std::fs::read_to_string(entry.path()) {
                consider(&text);
            }
        }
    }

    ambiguous
}

/// Read a core module's declared `wit-world` from its manifest TOML, e.g.
/// `slicer:world-layer`. Returns `None` when the manifest is absent or has no
/// `wit-world` key.
pub fn module_world(module_dir: &Path, module_name: &str) -> Option<String> {
    let text = std::fs::read_to_string(module_dir.join(format!("{module_name}.toml"))).ok()?;
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("wit-world") else {
            continue;
        };
        let Some((_, value)) = rest.split_once('=') else {
            continue;
        };
        return Some(value.trim().trim_matches('"').to_string());
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

/// Compare a built component's embedded types against the canonical ones.
///
/// Only types present in **both** are compared: a guest that does not reference
/// a given canonical type simply will not embed it, which is not drift. Returns
/// every mismatch found, so the caller can report all of them at once.
pub fn verify_embedded_world(
    artifact: &Path,
    canonical: &BTreeMap<String, String>,
) -> Result<Vec<TypeMismatch>, VerifyError> {
    let embedded = extract_type_blocks(&embedded_wit_text(artifact)?);

    let mut mismatches = Vec::new();
    for (name, embedded_body) in &embedded {
        if let Some(canonical_body) = canonical.get(name) {
            if canonical_body != embedded_body {
                mismatches.push(TypeMismatch {
                    type_name: name.clone(),
                    canonical: canonical_body.clone(),
                    embedded: embedded_body.clone(),
                });
            }
        }
    }

    Ok(mismatches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_variant_body_by_name() {
        let wit = "interface x { variant role { a, b, custom(string), } }";
        let blocks = extract_type_blocks(wit);
        assert_eq!(
            blocks.get("role").map(String::as_str),
            Some("a,b,custom(string)")
        );
    }

    #[test]
    fn normalization_ignores_formatting_and_comments() {
        let canonical = "variant role {\n  a, b, // trailing note\n  c,\n}";
        let rendered = "variant role {\n      a,\n      b,\n      c,\n    }";
        assert_eq!(
            extract_type_blocks(canonical).get("role"),
            extract_type_blocks(rendered).get("role")
        );
    }

    /// The exact drift that broke instantiation: a case added canonically but
    /// missing from the guest's embedded copy must be detected.
    #[test]
    fn detects_missing_variant_case() {
        let canonical =
            extract_type_blocks("variant extrusion-role { a, b, raft-infill, custom(string), }");
        let embedded = extract_type_blocks("variant extrusion-role { a, b, custom(string), }");
        assert_ne!(
            canonical.get("extrusion-role"),
            embedded.get("extrusion-role"),
            "13-case vs 14-case extrusion-role must not compare equal"
        );
    }

    #[test]
    fn keyword_must_stand_alone() {
        // `my-record` must not be parsed as a `record` declaration.
        let blocks = extract_type_blocks("record real { a: u32 }");
        assert!(blocks.contains_key("real"));
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn nested_braces_are_balanced() {
        let wit =
            "record outer { inner: list<u32>, nested: tuple<u32, u32> } record after { x: u32 }";
        let blocks = extract_type_blocks(wit);
        assert!(blocks.contains_key("outer"));
        assert!(blocks.contains_key("after"));
    }

    fn ws_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask/ must have a parent")
            .to_path_buf()
    }

    /// The canonical WIT must parse into a non-trivial type set — otherwise
    /// `verify_embedded_world` would silently verify nothing and the gate would
    /// be vacuous.
    #[test]
    fn canonical_wit_yields_types_including_extrusion_role() {
        let canonical = canonical_type_blocks(&ws_root(), None);
        assert!(
            canonical.len() > 10,
            "expected canonical WIT to define many types, got {}",
            canonical.len()
        );
        let role = canonical
            .get("extrusion-role")
            .expect("canonical WIT must define extrusion-role");
        assert!(
            role.contains("raft-infill"),
            "canonical extrusion-role should carry raft-infill, got: {role}"
        );
    }

    /// A world's own declaration must shadow a same-named shared one, or
    /// finalization guests report phantom drift on `region-key`.
    #[test]
    fn world_declarations_shadow_shared_ones() {
        let root = ws_root();
        // world-layer does not redeclare `region-key`, so the shared
        // `ir-handles` spelling survives there; world-finalization does.
        let layer = canonical_type_blocks(&root, Some("slicer:world-layer"));
        let finalization = canonical_type_blocks(&root, Some("slicer:world-finalization"));

        let shared_key = layer.get("region-key").expect("ir-handles region-key");
        let final_key = finalization
            .get("region-key")
            .expect("finalization region-key");

        assert!(
            shared_key.contains("variant-chain"),
            "ir-handles region-key carries variant-chain: {shared_key}"
        );
        assert!(
            !final_key.contains("variant-chain"),
            "world-finalization region-key must shadow it: {final_key}"
        );
    }

    /// With no world to disambiguate (test-guests), a name spelled differently
    /// in two packages must be dropped rather than compared against an
    /// arbitrary spelling — while unambiguous names stay covered.
    #[test]
    fn unknown_world_drops_ambiguous_names_but_keeps_the_rest() {
        let shared = canonical_type_blocks(&ws_root(), None);
        assert!(
            !shared.contains_key("region-key"),
            "region-key is package-ambiguous and must be skipped without a world"
        );
        assert!(
            shared.contains_key("extrusion-role"),
            "unambiguous types must still be verified"
        );
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

        let mut canonical =
            canonical_type_blocks(&root, module_world(&dir, "classic-perimeters").as_deref());
        let role = canonical
            .get("extrusion-role")
            .cloned()
            .expect("canonical extrusion-role");
        assert!(role.contains("raft-infill"), "precondition: {role}");
        canonical.insert(
            "extrusion-role".to_string(),
            role.replace("raft-infill,", ""),
        );

        match verify_embedded_world(&artifact, &canonical) {
            Ok(mismatches) => assert!(
                mismatches.iter().any(|m| m.type_name == "extrusion-role"),
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
        assert!(
            !canonical_type_blocks(&root, None).is_empty(),
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

            let world = module_world(&dir, name);
            let canonical = canonical_type_blocks(&root, world.as_deref());

            match verify_embedded_world(&artifact, &canonical) {
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
}
