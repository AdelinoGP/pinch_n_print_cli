//! Registry validation and the SDK property-context/provenance boundary.

pub(crate) mod executor;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::fs;
use std::path::{Component, Path};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(crate) struct PropertiesError(pub(crate) String);

impl fmt::Display for PropertiesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PropertiesError {}

fn error(kind: &str, detail: impl fmt::Display) -> PropertiesError {
    PropertiesError(format!("{kind}: {detail}"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Registry {
    pub(crate) schema_version: u32,
    pub(crate) property: Vec<PropertyRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PropertyRecord {
    pub(crate) id: String,
    pub(crate) package: String,
    pub(crate) target_kind: TargetKind,
    pub(crate) target_name: String,
    pub(crate) test_name: String,
    pub(crate) production_symbol: String,
    pub(crate) production_path: String,
    pub(crate) features: Vec<String>,
    pub(crate) requires_host_algos: bool,
    pub(crate) requires_scad: bool,
    pub(crate) generator_version: u64,
    pub(crate) charter: String,
    pub(crate) routine: Option<Profile>,
    pub(crate) extended: Option<Profile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetKind {
    Test,
    Lib,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Tier {
    Routine,
    Extended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Isolation {
    InProcess,
    Fork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RngAlgorithm {
    Chacha,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    pub(crate) cases: u32,
    pub(crate) max_local_rejects: u32,
    pub(crate) max_global_rejects: u32,
    pub(crate) max_flat_map_regens: u32,
    pub(crate) max_default_size_range: usize,
    pub(crate) max_shrink_iters: u32,
    pub(crate) max_shrink_time_ms: u32,
    pub(crate) rng_algorithm: RngAlgorithm,
    pub(crate) isolation: Isolation,
    pub(crate) timeout_ms: u32,
    pub(crate) bounds: BTreeMap<String, u64>,
}

impl PropertyRecord {
    pub(crate) fn profile(&self, tier: Tier) -> Result<&Profile, PropertiesError> {
        match tier {
            Tier::Routine => self.routine.as_ref(),
            Tier::Extended => self.extended.as_ref(),
        }
        .ok_or_else(|| error("missing_tier_profile", format!("{}: {tier:?}", self.id)))
    }
}

// Only the Cargo metadata fields used for validating claims are retained.
#[derive(Debug, Deserialize)]
pub(crate) struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    features: BTreeMap<String, Vec<String>>,
    targets: Vec<Target>,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
    #[serde(default, rename = "required-features")]
    required_features: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    path: Option<String>,
}

impl Metadata {
    pub(crate) fn load(root: &Path) -> Result<Self, PropertiesError> {
        let output = Command::new("cargo")
            .current_dir(root)
            .args(["metadata", "--locked", "--format-version", "1", "--no-deps"])
            .output()
            .map_err(|e| error("cargo_metadata_failed", e))?;
        if !output.status.success() {
            return Err(error(
                "cargo_metadata_failed",
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        serde_json::from_slice(&output.stdout).map_err(|e| error("cargo_metadata_invalid", e))
    }

    fn package(&self, name: &str) -> Option<&Package> {
        self.packages
            .iter()
            .find(|p| p.name == name && self.workspace_members.iter().any(|member| member == &p.id))
    }

    // Reachability is derived from Cargo metadata path-dependency edges, never
    // from source directories. Test targets may use dev-dependencies too.
    fn can_see(&self, package: &str, wanted: &str) -> bool {
        let mut queue = VecDeque::from([package.to_owned()]);
        let mut seen = BTreeSet::new();
        while let Some(name) = queue.pop_front() {
            if !seen.insert(name.clone()) {
                continue;
            }
            if name == wanted && self.package(&name).is_some() {
                return true;
            }
            if let Some(pkg) = self.package(&name) {
                for dep in &pkg.dependencies {
                    if dep.path.is_some() && self.package(&dep.name).is_some() {
                        queue.push_back(dep.name.clone());
                    }
                }
            }
        }
        false
    }
}

fn check_fields(
    table: &toml::value::Table,
    allowed: &[&str],
    kind: &str,
) -> Result<(), PropertiesError> {
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(error(kind, key));
        }
    }
    Ok(())
}

/// Parse and validate all records against the actual workspace metadata.
pub(crate) fn parse_registry(
    input: &str,
    metadata: &Metadata,
) -> Result<Registry, PropertiesError> {
    const ROOT: &[&str] = &["schema_version", "property"];
    const PROPERTY: &[&str] = &[
        "id",
        "package",
        "target_kind",
        "target_name",
        "test_name",
        "production_symbol",
        "production_path",
        "features",
        "requires_host_algos",
        "requires_scad",
        "generator_version",
        "charter",
        "routine",
        "extended",
    ];
    const PROFILE: &[&str] = &[
        "cases",
        "max_local_rejects",
        "max_global_rejects",
        "max_flat_map_regens",
        "max_default_size_range",
        "max_shrink_iters",
        "max_shrink_time_ms",
        "rng_algorithm",
        "isolation",
        "timeout_ms",
        "bounds",
    ];
    let value: toml::Value = toml::from_str(input).map_err(|e| error("registry_parse_error", e))?;
    let root = value
        .as_table()
        .ok_or_else(|| error("invalid_registry_root", "expected table"))?;
    check_fields(root, ROOT, "unknown_root_key")?;
    if root.get("schema_version").and_then(toml::Value::as_integer) != Some(1) {
        return Err(error(
            "invalid_schema_version",
            "expected schema_version = 1",
        ));
    }
    let records = root
        .get("property")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            error(
                "invalid_property_array",
                "property must be an array of tables",
            )
        })?;
    for value in records {
        let table = value
            .as_table()
            .ok_or_else(|| error("invalid_property_record", "expected table"))?;
        check_fields(table, PROPERTY, "unknown_property_key")?;
        if let Some(value) = table.get("target_kind") {
            if !matches!(value.as_str(), Some("test" | "lib")) {
                return Err(error("unknown_target_kind", value));
            }
        }
        for tier in ["routine", "extended"] {
            if let Some(value) = table.get(tier) {
                let profile = value
                    .as_table()
                    .ok_or_else(|| error("invalid_profile", tier))?;
                check_fields(profile, PROFILE, "unknown_profile_key")?;
                if let Some(value) = profile.get("rng_algorithm") {
                    if value.as_str() != Some("chacha") {
                        return Err(error("unknown_rng_algorithm", value));
                    }
                }
                if let Some(value) = profile.get("isolation") {
                    if !matches!(value.as_str(), Some("in_process" | "fork")) {
                        return Err(error("unknown_isolation", value));
                    }
                }
            }
        }
    }
    let registry: Registry = value
        .try_into()
        .map_err(|e| error("registry_parse_error", e))?;
    let mut ids = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for record in &registry.property {
        if !ids.insert(&record.id) {
            return Err(error("duplicate_property_id", &record.id));
        }
        if !identities.insert((&record.package, &record.target_name, &record.test_name)) {
            return Err(error("duplicate_test_identity", &record.test_name));
        }
        validate_record(record, metadata)?;
    }
    Ok(registry)
}

fn validate_record(record: &PropertyRecord, metadata: &Metadata) -> Result<(), PropertiesError> {
    let mut chars = record.id.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_uppercase())
        || !chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(error("invalid_property_id", &record.id));
    }
    if !record
        .test_name
        .split("::")
        .last()
        .is_some_and(|s| s.starts_with("proptest_"))
        || record.test_name.split("::").any(str::is_empty)
    {
        return Err(error("invalid_test_name", &record.test_name));
    }
    if record.production_symbol.trim().is_empty() || record.production_path.trim().is_empty() {
        return Err(error("invalid_production_reference", &record.id));
    }
    if record.generator_version == 0 {
        return Err(error("nonpositive_generator_version", &record.id));
    }
    validate_charter(&record.charter)?;
    if record.routine.is_none() && record.extended.is_none() {
        return Err(error("missing_property_profiles", &record.id));
    }
    for profile in [record.routine.as_ref(), record.extended.as_ref()]
        .into_iter()
        .flatten()
    {
        if profile.cases == 0
            || profile.max_default_size_range == 0
            || profile.max_shrink_iters == 0
        {
            return Err(error("nonpositive_profile_value", &record.id));
        }
        if profile.isolation == Isolation::InProcess && profile.timeout_ms != 0 {
            return Err(error(
                "contradictory_isolation",
                "a nonzero timeout requires process isolation",
            ));
        }
    }
    let pkg = metadata
        .package(&record.package)
        .ok_or_else(|| error("unknown_package", &record.package))?;
    let kind = match record.target_kind {
        TargetKind::Test => "test",
        TargetKind::Lib => "lib",
    };
    let target = pkg
        .targets
        .iter()
        .find(|target| target.name == record.target_name && target.kind.iter().any(|k| k == kind))
        .ok_or_else(|| {
            error(
                "unknown_target",
                format!("{}/{kind}/{}", record.package, record.target_name),
            )
        })?;
    for feature in &record.features {
        let (owner, name) = feature
            .split_once('/')
            .unwrap_or((&record.package, feature));
        let owner_pkg = metadata
            .package(owner)
            .ok_or_else(|| error("unknown_feature_owner", feature))?;
        if !metadata.can_see(&record.package, owner) || !owner_pkg.features.contains_key(name) {
            return Err(error("unknown_feature", feature));
        }
    }
    let merged = merge_features_with_metadata(record, metadata)?;
    let enabled = enabled_package_features(pkg, &merged);
    for required in &target.required_features {
        if !enabled.contains(required) {
            return Err(error(
                "target_feature_not_enabled",
                format!("{} requires {required}", target.name),
            ));
        }
    }
    Ok(())
}

fn enabled_package_features(pkg: &Package, features: &[String]) -> BTreeSet<String> {
    let mut queue = VecDeque::new();
    if pkg.features.contains_key("default") {
        queue.push_back("default".to_owned());
    }
    for feature in features {
        let (owner, name) = feature.split_once('/').unwrap_or((&pkg.name, feature));
        if owner == pkg.name {
            queue.push_back(name.to_owned());
        }
    }
    let mut enabled = BTreeSet::new();
    while let Some(feature) = queue.pop_front() {
        if enabled.insert(feature.clone()) {
            if let Some(implied) = pkg.features.get(&feature) {
                for name in implied {
                    if pkg.features.contains_key(name) {
                        queue.push_back(name.clone());
                    }
                }
            }
        }
    }
    enabled
}

fn validate_charter(charter: &str) -> Result<(), PropertiesError> {
    let (path, anchor) = charter
        .split_once('#')
        .ok_or_else(|| error("unsafe_charter", charter))?;
    if path.is_empty()
        || anchor.is_empty()
        || anchor.contains('#')
        || path.contains('\\')
        || path.contains(':')
        || path.starts_with('/')
        || !path.ends_with(".md")
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(error("unsafe_charter", charter));
    }
    Ok(())
}

pub(crate) fn merge_features(record: &PropertyRecord) -> Result<Vec<String>, PropertiesError> {
    merge_features_with_metadata(
        record,
        &Metadata::load(&crate::build_guests::workspace_root())?,
    )
}

fn merge_features_with_metadata(
    record: &PropertyRecord,
    metadata: &Metadata,
) -> Result<Vec<String>, PropertiesError> {
    let mut features = Vec::new();
    for feature in &record.features {
        if !features.contains(feature) {
            features.push(feature.clone());
        }
    }
    if record.requires_host_algos {
        if !metadata.can_see(&record.package, "slicer-core")
            || !metadata
                .package("slicer-core")
                .is_some_and(|p| p.features.contains_key("host-algos"))
        {
            return Err(error("host_algos_unavailable", &record.package));
        }
        let required = "slicer-core/host-algos".to_owned();
        if !features.contains(&required) {
            features.push(required);
        }
    }
    Ok(features)
}

/// Draw fresh OS entropy unless a replay seed was supplied. Independent draws
/// can coincide; uniqueness is not a property of random sampling.
pub(crate) fn resolve_seed(explicit: Option<u64>) -> u64 {
    explicit.unwrap_or_else(|| {
        let mut bytes = [0_u8; 8];
        getrandom::fill(&mut bytes).expect("OS entropy is required for fresh property seeds");
        u64::from_le_bytes(bytes)
    })
}

pub(crate) fn run_token() -> String {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("OS entropy is required for property run tokens");
    hex(&bytes)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Serialize)]
pub(crate) struct SourceStateRecord {
    revision: String,
    dirty_paths: Vec<String>,
    tracked_diff_sha256: String,
    untracked_files: Vec<SourceFileRecord>,
    source_state_digest: String,
}

#[derive(Debug, Serialize)]
struct SourceFileRecord {
    path: String,
    sha256: String,
}

fn hash_part(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

fn source_digest(revision: &str, diff: &[u8], files: &[(String, Vec<u8>)]) -> String {
    let mut hash = Sha256::new();
    hash_part(&mut hash, b"pnp-property-source-state-v1");
    hash_part(&mut hash, revision.as_bytes());
    hash_part(&mut hash, diff);
    for (path, contents) in files {
        hash_part(&mut hash, path.as_bytes());
        hash_part(&mut hash, contents);
    }
    hex(&hash.finalize())
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, PropertiesError> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|e| error("source_state_git_failed", e))?;
    if !output.status.success() {
        return Err(error(
            "source_state_git_failed",
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(output.stdout)
}

fn relevant_untracked(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| {
            matches!(
                ext,
                "rs" | "toml" | "lock" | "wit" | "json" | "yml" | "yaml" | "md" | "ini" | "conf"
            )
        })
}

/// Capture the revision, tracked binary diff and relevant untracked bytes.
pub(crate) fn source_state(
) -> Result<(String, Vec<String>, String, SourceStateRecord), PropertiesError> {
    source_state_at(&crate::build_guests::workspace_root())
}

fn source_state_at(
    root: &Path,
) -> Result<(String, Vec<String>, String, SourceStateRecord), PropertiesError> {
    let revision = String::from_utf8(git(root, &["rev-parse", "HEAD"])?)
        .map_err(|e| error("source_state_encoding", e))?
        .trim()
        .to_owned();
    let diff = git(
        root,
        &["diff", "HEAD", "--binary", "--no-ext-diff", "--no-renames"],
    )?;
    let status = git(
        root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    let mut chunks = status
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty());
    let mut dirty = BTreeSet::new();
    let mut untracked = BTreeSet::new();
    while let Some(entry) = chunks.next() {
        if entry.len() < 4 || entry[2] != b' ' {
            return Err(error("source_state_status_invalid", "malformed git status"));
        }
        let name = std::str::from_utf8(&entry[3..])
            .map_err(|e| error("source_state_encoding", e))?
            .to_owned();
        if Path::new(&name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
            || name.contains('\\')
        {
            return Err(error("source_state_unsafe_path", name));
        }
        dirty.insert(name.clone());
        if &entry[..2] == b"??" && relevant_untracked(Path::new(&name)) {
            untracked.insert(name);
        }
        if entry[0] == b'R' || entry[0] == b'C' || entry[1] == b'R' || entry[1] == b'C' {
            let old = chunks
                .next()
                .ok_or_else(|| error("source_state_status_invalid", "missing rename source"))?;
            dirty.insert(
                std::str::from_utf8(old)
                    .map_err(|e| error("source_state_encoding", e))?
                    .to_owned(),
            );
        }
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|e| error("source_state_io", e))?;
    let mut files = Vec::new();
    for path in untracked {
        let full = root.join(&path);
        let real = full
            .canonicalize()
            .map_err(|e| error("source_state_io", e))?;
        if !real.starts_with(&canonical_root) || !real.is_file() {
            return Err(error("source_state_unsafe_path", path));
        }
        files.push((
            path,
            fs::read(real).map_err(|e| error("source_state_io", e))?,
        ));
    }
    let digest = source_digest(&revision, &diff, &files);
    let dirty_paths: Vec<_> = dirty.into_iter().collect();
    let record = SourceStateRecord {
        revision: revision.clone(),
        dirty_paths: dirty_paths.clone(),
        tracked_diff_sha256: hex(&Sha256::digest(&diff)),
        untracked_files: files
            .iter()
            .map(|(path, contents)| SourceFileRecord {
                path: path.clone(),
                sha256: hex(&Sha256::digest(contents)),
            })
            .collect(),
        source_state_digest: digest.clone(),
    };
    Ok((revision, dirty_paths, digest, record))
}

fn json_path(path: &Path) -> Result<&str, PropertiesError> {
    path.to_str()
        .ok_or_else(|| error("non_utf8_property_path", path.display()))
}

/// Build exactly the context consumed by `PropertyRun`; the source-state
/// record is retained alongside the SDK's target-local execution artifacts.
pub(crate) fn build_property_context(
    record: &PropertyRecord,
    tier: Tier,
    seed: u64,
    artifact_dir: &Path,
    proptest_version: &str,
    cargo_target_dir: Option<&Path>,
) -> Result<Value, PropertiesError> {
    let property_name = artifact_dir.file_name().and_then(|s| s.to_str());
    let token = artifact_dir
        .parent()
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error("invalid_artifact_dir", artifact_dir.display()))?;
    if property_name != Some(record.id.as_str()) {
        return Err(error(
            "invalid_artifact_dir",
            "directory must end in run_token/property_id",
        ));
    }
    let (revision, dirty_paths, digest, state) = source_state()?;
    fs::create_dir_all(artifact_dir).map_err(|e| error("source_state_io", e))?;
    fs::write(
        artifact_dir.join("source-state.json"),
        serde_json::to_vec_pretty(&state).map_err(|e| error("source_state_encoding", e))?,
    )
    .map_err(|e| error("source_state_io", e))?;
    let target = cargo_target_dir.map(json_path).transpose()?;
    Ok(json!({
        "schema_version": 1, "run_token": token, "property_id": record.id,
        "tier": tier, "seed": seed, "package": record.package, "target_kind": record.target_kind,
        "target_name": record.target_name, "test_name": record.test_name,
        "features": merge_features(record)?, "profile": record.profile(tier)?,
        "artifact_dir": json_path(artifact_dir)?, "proptest_version": proptest_version,
        "generator_version": record.generator_version, "source_revision": revision,
        "dirty_paths": dirty_paths, "source_state_digest": digest,
        "cargo_target_dir": target, "completion_eligible": true
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Output;
    use std::sync::OnceLock;
    use std::time::Instant;
    use walkdir::WalkDir;

    fn root() -> PathBuf {
        crate::build_guests::workspace_root()
    }

    fn metadata() -> &'static Metadata {
        static METADATA: OnceLock<Metadata> = OnceLock::new();
        METADATA.get_or_init(|| Metadata::load(&root()).expect("workspace Cargo metadata"))
    }

    fn fixture_toml() -> String {
        r#"schema_version = 1
[[property]]
id = "PBT-FIXTURE"
package = "slicer-sdk"
target_kind = "test"
target_name = "property_harness"
test_name = "proptest_fixture_context_child"
production_symbol = "fixture::threshold"
production_path = "crates/slicer-sdk/tests/property_harness.rs"
features = ["slicer-sdk/property_testing"]
requires_host_algos = false
requires_scad = false
generator_version = 1
charter = "docs/testing/properties.md#fixture"
[property.routine]
cases = 8
max_local_rejects = 16
max_global_rejects = 3
max_flat_map_regens = 100
max_default_size_range = 32
max_shrink_iters = 64
max_shrink_time_ms = 0
rng_algorithm = "chacha"
isolation = "in_process"
timeout_ms = 0
bounds = { positive = 7 }
"#
        .to_owned()
    }

    fn record() -> PropertyRecord {
        parse_registry(&fixture_toml(), metadata())
            .expect("valid SDK fixture")
            .property
            .remove(0)
    }

    fn reject(input: &str, expected: &str) {
        let result = parse_registry(input, metadata());
        let message = result
            .expect_err("registry must reject fixture")
            .to_string();
        assert!(
            message.contains(expected),
            "expected {expected}, got {message}"
        );
    }

    #[test]
    fn property_registry_rejects_malformed_records() {
        let valid = fixture_toml();
        let second = valid.split_once("[[property]]").expect("record fixture").1;
        reject(
            &format!("{}\n[[property]]{}", valid, second),
            "duplicate_property_id",
        );
        let second_id = second.replace("PBT-FIXTURE", "PBT-OTHER");
        reject(
            &format!("{}\n[[property]]{}", valid, second_id),
            "duplicate_test_identity",
        );
        reject(
            &valid.replace("schema_version = 1", "schema_version = 2"),
            "invalid_schema_version",
        );
        reject(
            &valid.replace("schema_version = 1", "schema_version = 1\nextra = true"),
            "unknown_root_key",
        );
        reject(
            &valid.replace("id = \"PBT-FIXTURE\"", "id = \"PBT-FIXTURE\"\nextra = true"),
            "unknown_property_key",
        );
        reject(
            &valid.replace("cases = 8", "cases = 8\nextra = true"),
            "unknown_profile_key",
        );
        reject(
            valid
                .split_once("[property.routine]")
                .expect("profile fixture")
                .0,
            "missing_property_profiles",
        );
        for bad in [
            "/docs/x.md#anchor",
            "../docs/x.md#anchor",
            "docs/../x.md#anchor",
            "C:/docs/x.md#anchor",
            "docs/x.txt#anchor",
            "docs/x.md",
            "docs/x.md#",
        ] {
            reject(
                &valid.replace("docs/testing/properties.md#fixture", bad),
                "unsafe_charter",
            );
        }
        reject(
            &valid.replace(
                "docs/testing/properties.md#fixture",
                "docs\\\\bad.md#anchor",
            ),
            "unsafe_charter",
        );
        reject(
            &valid.replace("timeout_ms = 0", "timeout_ms = 1"),
            "contradictory_isolation",
        );
        reject(
            &valid.replace("PBT-FIXTURE", "bad-id"),
            "invalid_property_id",
        );
        reject(
            &valid.replace("proptest_fixture_context_child", "ordinary_test"),
            "invalid_test_name",
        );
        reject(
            &valid.replace("package = \"slicer-sdk\"", "package = \"missing-package\""),
            "unknown_package",
        );
        reject(
            &valid.replace(
                "target_name = \"property_harness\"",
                "target_name = \"missing_target\"",
            ),
            "unknown_target",
        );
        reject(
            &valid.replace("slicer-sdk/property_testing", "slicer-sdk/no_such_feature"),
            "unknown_feature",
        );
        reject(
            &valid.replace("slicer-sdk/property_testing", "unknown-owner/feature"),
            "unknown_feature_owner",
        );
        reject(
            &valid.replace("target_kind = \"test\"", "target_kind = \"bench\""),
            "unknown_target_kind",
        );
        reject(
            &valid.replace("rng_algorithm = \"chacha\"", "rng_algorithm = \"other\""),
            "unknown_rng_algorithm",
        );
        reject(
            &valid.replace("isolation = \"in_process\"", "isolation = \"none\""),
            "unknown_isolation",
        );
        reject(
            &valid.replace("package = \"slicer-sdk\"\n", ""),
            "missing field `package`",
        );
    }

    #[test]
    fn property_registry_derives_feature_targets() {
        let valid = fixture_toml();
        let empty = parse_registry("schema_version = 1\nproperty = []\n", metadata())
            .expect("empty fixture registry schema");
        assert!(empty.property.is_empty());
        let registry =
            parse_registry(&valid, metadata()).expect("feature enables SDK integration target");
        assert_eq!(registry.property[0].target_name, "property_harness");
        assert_eq!(
            registry.property[0].features,
            ["slicer-sdk/property_testing"]
        );
        reject(
            &valid.replace(
                "features = [\"slicer-sdk/property_testing\"]",
                "features = []",
            ),
            "target_feature_not_enabled",
        );
        reject(
            &valid.replace(
                "target_name = \"property_harness\"",
                "target_name = \"slicer_sdk\"",
            ),
            "unknown_target",
        );
        let library = valid
            .replace("target_kind = \"test\"", "target_kind = \"lib\"")
            .replace(
                "target_name = \"property_harness\"",
                "target_name = \"slicer_sdk\"",
            );
        assert_eq!(
            parse_registry(&library, metadata())
                .expect("real lib target")
                .property[0]
                .target_name,
            "slicer_sdk"
        );
    }

    #[test]
    fn property_registry_loads_live_ir_anchor_record() {
        let input = fs::read_to_string(root().join("test-properties.toml"))
            .expect("read the live property registry");
        let registry = parse_registry(&input, metadata()).expect("validate the live registry");
        let anchor = registry
            .property
            .iter()
            .find(|record| record.id == "H-IR-ANCHORS")
            .expect("the live registry contains the IR anchor property");
        assert_eq!(anchor.package, "slicer-ir");
        assert_eq!(anchor.target_kind, TargetKind::Test);
        assert_eq!(anchor.target_name, "property_ir");
        assert_eq!(anchor.test_name, "proptest_ir_travel_anchors_membership");
        assert_eq!(
            anchor.production_symbol,
            "slicer_ir::validate_travel_anchors"
        );
        assert_eq!(anchor.production_path, "crates/slicer-ir/src/validation.rs");
        assert_eq!(anchor.features, ["slicer-ir/property_testing"]);
        assert!(!anchor.requires_host_algos);
        assert!(!anchor.requires_scad);
        assert_eq!(anchor.generator_version, 1);
        assert_eq!(anchor.charter, "docs/testing/properties.md#h-ir-anchors");
        for tier in [Tier::Routine, Tier::Extended] {
            let profile = anchor.profile(tier).expect("both live profiles exist");
            assert_eq!(
                profile
                    .bounds
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                ["max_entities", "max_travels"]
            );
        }
    }

    #[test]
    fn property_registry_rejects_nonpositive_values() {
        let valid = fixture_toml();
        for field in [
            "cases = 8",
            "max_default_size_range = 32",
            "max_shrink_iters = 64",
        ] {
            let zero = field.split_once(" = ").expect("field fixture").0.to_owned() + " = 0";
            reject(&valid.replace(field, &zero), "nonpositive_profile_value");
            reject(&valid.replace(field, ""), "missing field");
        }
        reject(
            &valid.replace("generator_version = 1", "generator_version = 0"),
            "nonpositive_generator_version",
        );
        reject(
            &valid.replace("generator_version = 1", ""),
            "missing field `generator_version`",
        );
        reject(
            &valid.replace("cases = 8", "cases = -1"),
            "registry_parse_error",
        );
    }

    #[test]
    fn property_feature_union_preserves_host_algos() {
        let mut core = record();
        core.package = "slicer-core".to_owned();
        core.requires_host_algos = true;
        core.features = vec!["slicer-sdk/property_testing".to_owned()];
        assert_eq!(
            merge_features(&core).expect("core host features"),
            ["slicer-sdk/property_testing", "slicer-core/host-algos"]
        );
        core.requires_scad = true;
        core.features.push("slicer-scad/openrscad".to_owned()); // Synthetic only: no SCAD package is claimed.
        assert_eq!(
            merge_features(&core).expect("synthetic feature union"),
            [
                "slicer-sdk/property_testing",
                "slicer-scad/openrscad",
                "slicer-core/host-algos"
            ]
        );
        core.features.push("slicer-core/host-algos".to_owned());
        assert_eq!(
            merge_features(&core).expect("deduplicate host feature"),
            [
                "slicer-sdk/property_testing",
                "slicer-scad/openrscad",
                "slicer-core/host-algos"
            ]
        );
        // xtask's property-test dev edge can now reach core through the SDK.
        // Use the dependency-free schema crate for the unavailable-owner probe.
        core.package = "slicer-schema".to_owned();
        assert!(merge_features(&core)
            .expect_err("schema cannot see slicer-core")
            .to_string()
            .contains("host_algos_unavailable"));
    }

    #[test]
    fn property_anchor_oracle_catches_accept_all() {
        use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestError, TestRunner};
        use slicer_ir::{
            validate_travel_anchors, ExtrusionRole, LayerCollectionIR, PrintEntity, TravelMove,
        };
        use slicer_sdk::test_support::fixtures::print_entity_base;
        use slicer_sdk::test_support::property_testing::{
            check_travel_anchors_charter, travel_anchor_oracle,
        };
        use std::cell::Cell;

        let layer = LayerCollectionIR {
            ordered_entities: vec![PrintEntity {
                entity_id: 11,
                ..print_entity_base(ExtrusionRole::OuterWall)
            }],
            travel_moves: vec![
                TravelMove {
                    entity_id: 1,
                    ..Default::default()
                },
                TravelMove {
                    entity_id: 0,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            travel_anchor_oracle(&layer)
                .expect_err("missing 1")
                .entity_id,
            1
        );
        let failure = check_travel_anchors_charter(&layer, |_| Ok(()))
            .expect_err("accept-all substitute must fail the independent charter");
        assert!(failure
            .to_string()
            .contains("accepted missing TravelMove.entity_id = 1"));
        check_travel_anchors_charter(&layer, validate_travel_anchors)
            .expect("real validator must reject the same witness correctly");
        for wrong in [
            "entity_id = 11 not present in layer.ordered_entities", // 1 is not a token in 11
            "entity_id = 0 not present in layer.ordered_entities",  // not first in travel order
            "entity_identifier = 1 not present in layer.ordered_entities",
            "entity_id = 1 not present in another_collection",
        ] {
            assert!(
                check_travel_anchors_charter(&layer, |_| Err(wrong.to_owned())).is_err(),
                "bad diagnostic accepted: {wrong}"
            );
        }
        assert!(
            check_travel_anchors_charter(&LayerCollectionIR::default(), |_| Err(
                "reject-all".to_owned()
            ))
            .is_err()
        );

        // Controlled mutation shrink pilots. Every generated/shrunken anchor
        // stays absent (entities are empty), and the real validator still must
        // satisfy the generic charter on every candidate. No production seed
        // regression or synthetic registry record is created.
        for max_shrink_iters in [16, 64, 256] {
            let mut runner = TestRunner::new(Config {
                cases: 64,
                max_local_rejects: 0,
                max_global_rejects: 0,
                max_flat_map_regens: 0,
                max_default_size_range: 32,
                max_shrink_iters,
                max_shrink_time: 0,
                failure_persistence: None,
                rng_algorithm: RngAlgorithm::ChaCha,
                rng_seed: RngSeed::Fixed(37),
                ..Config::default()
            });
            let calls = Cell::new(0);
            let start = Instant::now();
            let result = runner.run(&proptest::collection::vec(1_u64..=16, 1..=8), |anchors| {
                calls.set(calls.get() + 1);
                let candidate = LayerCollectionIR {
                    travel_moves: anchors
                        .into_iter()
                        .map(|entity_id| TravelMove {
                            entity_id,
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                };
                check_travel_anchors_charter(&candidate, validate_travel_anchors)?;
                check_travel_anchors_charter(&candidate, |_| Ok(()))
            });
            let elapsed = start.elapsed();
            match result {
                Err(TestError::Fail(_, anchors)) => {
                    assert_eq!(anchors, [1], "retained concrete dangling witness");
                    assert!(calls.get() > 1, "must exercise shrink callbacks");
                    println!("anchor mutation shrink pilot: max_shrink_iters={max_shrink_iters}, calls={}, elapsed={elapsed:?}, retained={anchors:?}, shrink_completed=null\n{runner}", calls.get());
                }
                other => panic!("accept-all mutation must fail, got {other:?}"),
            }
        }
    }

    #[test]
    fn property_source_state_tracks_dirty_contents() {
        let dir = root()
            .join("target/property-testing")
            .join(format!("source-state-test-{}", run_token()));
        fs::create_dir_all(&dir).expect("create isolated git fixture");
        let run = |args: &[&str]| {
            let output = Command::new("git")
                .current_dir(&dir)
                .env("GIT_AUTHOR_NAME", "Property Test")
                .env("GIT_AUTHOR_EMAIL", "property@example.invalid")
                .env("GIT_COMMITTER_NAME", "Property Test")
                .env("GIT_COMMITTER_EMAIL", "property@example.invalid")
                .args(args)
                .output()
                .expect("run git in fixture");
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        run(&["init", "--quiet"]);
        fs::write(dir.join("tracked.toml"), "x = 1\n").expect("write tracked fixture");
        run(&["add", "tracked.toml"]);
        run(&["commit", "--quiet", "-m", "fixture"]);
        fs::write(dir.join("untracked.rs"), "first").expect("first untracked contents");
        let (_, paths_a, digest_a, _) = source_state_at(&dir).expect("first state");
        fs::write(dir.join("untracked.rs"), "second").expect("changed untracked contents");
        let (_, paths_b, digest_b, _) = source_state_at(&dir).expect("second state");
        assert_eq!(paths_a, paths_b);
        assert_ne!(
            digest_a, digest_b,
            "untracked contents, not just paths, must affect digest"
        );
        fs::write(dir.join("tracked.toml"), "x = 2\n").expect("change tracked contents");
        let (_, paths_c, digest_c, _) = source_state_at(&dir).expect("tracked state");
        fs::write(dir.join("tracked.toml"), "x = 3\n").expect("change tracked contents again");
        let (_, paths_d, digest_d, _) = source_state_at(&dir).expect("next tracked state");
        assert_eq!(paths_c, paths_d);
        assert_ne!(digest_c, digest_d, "tracked diff bytes must affect digest");
        assert_eq!(resolve_seed(Some(u64::MAX)), u64::MAX);
        assert!(run_token().chars().all(|c| c.is_ascii_hexdigit()));
    }

    fn metadata_json(manifest: &Path, features: Option<&str>) -> Value {
        let mut command = Command::new("cargo");
        // Windows verbatim paths (\\?\...) can cause Cargo to misidentify a
        // workspace member. Passing the original workspace-relative path
        // avoids changing the package/manifest under inspection.
        let root = root();
        let argument = manifest.strip_prefix(&root).unwrap_or(manifest);
        command
            .current_dir(&root)
            .args([
                "metadata",
                "--locked",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(argument);
        if let Some(features) = features {
            command.args(["--features", features]);
        }
        let output = command.output().expect("run cargo metadata");
        assert!(
            output.status.success(),
            "cargo metadata {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("Cargo metadata JSON")
    }

    fn graph_packages<'a>(metadata: &'a Value, root_name: &str) -> Vec<&'a Value> {
        let packages = metadata["packages"].as_array().expect("metadata packages");
        let roots: Vec<_> = packages.iter().filter(|p| p["name"] == root_name).collect();
        assert_eq!(
            roots.len(),
            1,
            "expected an unambiguous package named {root_name}"
        );
        let root_id = roots[0]["id"].as_str().expect("root id");
        let nodes = metadata["resolve"]["nodes"]
            .as_array()
            .expect("resolved nodes");
        let mut queue = VecDeque::from([root_id]);
        let mut seen = BTreeSet::new();
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            let node = nodes.iter().find(|n| n["id"] == id).expect("resolved node");
            for edge in node["deps"].as_array().expect("resolved dependencies") {
                let kinds = edge["dep_kinds"].as_array().expect("dependency kinds");
                if kinds
                    .iter()
                    .any(|kind| kind["kind"].is_null() || kind["kind"] == "build")
                {
                    queue.push_back(edge["pkg"].as_str().expect("dependency package id"));
                }
            }
        }
        packages
            .iter()
            .filter(|p| seen.contains(p["id"].as_str().expect("package id")))
            .collect()
    }

    // `cargo metadata` resolves feature activation across the whole workspace,
    // including dev edges of *other* members. Scope activation with Cargo's
    // own selected-package normal/build tree; keep metadata's dependency-kind
    // graph as the authority for reachability. `--target all` covers guest and
    // native target-specific dependencies instead of only the current host.
    fn active_names(manifest: &Path, package: &str, feature: Option<&str>) -> BTreeSet<String> {
        let root = root();
        let argument = manifest.strip_prefix(&root).unwrap_or(manifest);
        let mut command = Command::new("cargo");
        command
            .current_dir(root)
            .args(["tree", "--locked", "--manifest-path"])
            .arg(argument)
            .args([
                "--package",
                package,
                "--edges",
                "normal,build",
                "--target",
                "all",
                "--prefix",
                "none",
                "--format",
                "{p}",
            ]);
        if let Some(feature) = feature {
            command.args(["--features", feature]);
        }
        let output = command.output().expect("run cargo tree");
        assert!(
            output.status.success(),
            "cargo tree for {package}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).expect("Cargo tree UTF-8");
        let names: BTreeSet<String> = text
            .lines()
            .map(|line| {
                line.split_whitespace()
                    .next()
                    .expect("tree package name")
                    .to_owned()
            })
            .collect();
        assert!(
            names.contains(package),
            "tree must contain selected package {package}"
        );
        names
    }

    fn forbidden_packages(
        metadata: &Value,
        package: &str,
        active: &BTreeSet<String>,
    ) -> Vec<String> {
        let graph = graph_packages(metadata, package);
        assert!(!graph.is_empty(), "graph for {package} must be populated");
        let is_forbidden =
            |name: &str| name == "proptest" || name.to_ascii_lowercase().contains("openrscad");
        let graph_names: BTreeSet<_> = graph
            .iter()
            .map(|pkg| pkg["name"].as_str().expect("package name"))
            .collect();
        for name in active.iter().filter(|name| is_forbidden(name)) {
            assert!(
                graph_names.contains(name.as_str()),
                "selected tree/metadata graph disagreement for {name}"
            );
        }
        graph
            .iter()
            .filter_map(|pkg| {
                let name = pkg["name"].as_str().expect("package name");
                (is_forbidden(name) && active.contains(name)).then_some(name.to_owned())
            })
            .collect()
    }

    fn assert_isolated(metadata: &Value, manifest: &Path, package: &str, feature: Option<&str>) {
        let active = active_names(manifest, package, feature);
        let forbidden = forbidden_packages(metadata, package, &active);
        assert!(
            forbidden.is_empty(),
            "normal/build graph for {package} includes {forbidden:?}"
        );
    }

    fn version_tuple(version: &str) -> (u64, u64, u64) {
        let mut parts = version
            .split('.')
            .map(|part| part.parse().expect("numeric Rust version"));
        (
            parts.next().expect("major"),
            parts.next().expect("minor"),
            parts.next().unwrap_or(0),
        )
    }

    #[test]
    fn property_dependency_isolation() {
        let root = root();
        let manifest = root.join("Cargo.toml");
        let default = metadata_json(&manifest, None);
        assert_isolated(&default, &manifest, "pnp-cli", None);
        assert_isolated(&default, &manifest, "slicer-sdk", None);
        let test = metadata_json(&manifest, Some("slicer-sdk/test"));
        assert_isolated(&test, &manifest, "slicer-sdk", Some("test"));
        let property = metadata_json(&manifest, Some("slicer-sdk/property_testing"));
        let active = active_names(&manifest, "slicer-sdk", Some("property_testing"));
        assert!(active.contains("proptest"), "positive control: SDK property_testing must activate proptest in its normal/build tree");
        let property_graph = graph_packages(&property, "slicer-sdk");
        let proptest = property_graph
            .iter()
            .find(|p| p["name"] == "proptest")
            .expect("positive control: SDK property_testing must resolve proptest");
        let version = proptest["version"].as_str().expect("proptest version");
        let msrv = proptest["rust_version"]
            .as_str()
            .expect("proptest rust-version");
        let workspace: toml::Value =
            toml::from_str(&fs::read_to_string(&manifest).expect("workspace manifest"))
                .expect("workspace TOML");
        let toolchain: toml::Value = toml::from_str(
            &fs::read_to_string(root.join("rust-toolchain.toml")).expect("toolchain file"),
        )
        .expect("toolchain TOML");
        let workspace_msrv = workspace["workspace"]["package"]["rust-version"]
            .as_str()
            .expect("workspace MSRV");
        let pinned = toolchain["toolchain"]["channel"]
            .as_str()
            .expect("pinned toolchain");
        assert!(version_tuple(msrv) <= version_tuple(workspace_msrv));
        assert!(version_tuple(msrv) <= version_tuple(pinned));
        println!("property metadata: proptest {version}, rust-version {msrv}, workspace {workspace_msrv}, toolchain {pinned}");
        let (guests, skips) = crate::build_guests::discover_guests(&root);
        assert!(!guests.is_empty(), "guest discovery must not be vacuous");
        let start = Instant::now();
        for guest in &guests {
            let graph = metadata_json(&guest.manifest_path, None);
            assert_isolated(&graph, &guest.manifest_path, &guest.crate_name, None);
        }
        println!(
            "property guest metadata walk: {} guests, {} discovery skips, elapsed {:?}",
            guests.len(),
            skips.len(),
            start.elapsed()
        );
        for skip in skips {
            println!("property guest discovery: {skip}");
        }
    }

    #[test]
    fn property_dependency_graph_handles_dev_feature_unification() {
        let dir = root()
            .join("target/property-testing")
            .join(format!("graph-fixture-{}", run_token()));
        fs::create_dir_all(&dir).expect("graph fixture root");
        fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\"root\", \"lib\", \"dev\", \"proptest\"]\nresolver = \"2\"\n",
        )
        .expect("workspace fixture");
        for (directory, name, extra) in [
            ("root", "pbt-probe-root", "[dependencies]\npbt-probe-lib = { path = \"../lib\" }\n"),
            ("lib", "pbt-probe-lib", "[features]\nprobe = [\"dep:proptest\"]\n[dependencies]\nproptest = { path = \"../proptest\", optional = true }\n"),
            ("dev", "pbt-probe-dev", "[dev-dependencies]\npbt-probe-lib = { path = \"../lib\", features = [\"probe\"] }\n"),
            ("proptest", "proptest", ""),
        ] {
            let package = dir.join(directory);
            fs::create_dir_all(package.join("src")).expect("package fixture");
            fs::write(package.join("src/lib.rs"), "").expect("fixture source");
            fs::write(package.join("Cargo.toml"), format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{extra}")).expect("fixture manifest");
        }
        let manifest = dir.join("Cargo.toml");
        let output = Command::new("cargo")
            .current_dir(root())
            .args(["generate-lockfile", "--offline", "--manifest-path"])
            .arg(
                manifest
                    .strip_prefix(root())
                    .expect("workspace relative fixture"),
            )
            .output()
            .expect("generate fixture lock");
        assert!(
            output.status.success(),
            "fixture lock: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let metadata = metadata_json(&manifest, None);
        assert!(
            graph_packages(&metadata, "pbt-probe-root")
                .iter()
                .any(|package| package["name"] == "proptest"),
            "workspace dev feature must actually contaminate metadata's graph"
        );
        let root_active = active_names(&manifest, "pbt-probe-root", None);
        assert!(
            forbidden_packages(&metadata, "pbt-probe-root", &root_active).is_empty(),
            "unrelated dev feature must not contaminate the selected normal graph"
        );
        let lib_active = active_names(&manifest, "pbt-probe-lib", Some("probe"));
        assert_eq!(
            forbidden_packages(&metadata, "pbt-probe-lib", &lib_active),
            ["proptest"],
            "a real selected normal dependency must still be caught"
        );
    }

    const OVERRIDES: &[&str] = &[
        "PROPTEST_CASES",
        "PROPTEST_MAX_LOCAL_REJECTS",
        "PROPTEST_MAX_GLOBAL_REJECTS",
        "PROPTEST_MAX_FLAT_MAP_REGENS",
        "PROPTEST_MAX_SHRINK_TIME",
        "PROPTEST_MAX_SHRINK_ITERS",
        "PROPTEST_MAX_DEFAULT_SIZE_RANGE",
        "PROPTEST_FORK",
        "PROPTEST_TIMEOUT",
        "PROPTEST_RNG_ALGORITHM",
        "PROPTEST_RNG_SEED",
        "PROPTEST_DISABLE_FAILURE_PERSISTENCE",
    ];

    fn artifact_dir(record: &PropertyRecord) -> PathBuf {
        root()
            .join("target")
            .join("property-testing")
            .join(run_token())
            .join(&record.id)
    }

    fn resolved_proptest_version() -> &'static str {
        static VERSION: OnceLock<String> = OnceLock::new();
        VERSION.get_or_init(|| {
            let manifest = root().join("Cargo.toml");
            let metadata = metadata_json(&manifest, Some("slicer-sdk/property_testing"));
            graph_packages(&metadata, "slicer-sdk")
                .iter()
                .find(|package| package["name"] == "proptest")
                .expect("resolved SDK proptest")["version"]
                .as_str()
                .expect("resolved proptest version")
                .to_owned()
        })
    }

    fn run_child(test_name: &str, context: &Value, extra: Option<(&str, &str)>) -> Output {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root())
            .args([
                "test",
                "-p",
                "slicer-sdk",
                "--features",
                "property_testing",
                "--test",
                "property_harness",
                "--",
                "--exact",
                test_name,
                "--nocapture",
            ])
            .env(
                "PNP_PROPERTY_CONTEXT",
                serde_json::to_string(context).expect("serialize context"),
            )
            .env_remove("_PROPTEST_FORKFILE")
            .env_remove("PROPTEST_VERBOSE");
        for name in OVERRIDES {
            cmd.env_remove(name);
        }
        if let Some((name, value)) = extra {
            cmd.env(name, value);
        }
        let output = cmd.output().expect("spawn real Cargo property fixture");
        // The outer X command tees its stdout to the required test log. Relay
        // the captured child streams too, including expected failing probes.
        println!(
            "controlled property child {test_name} (expected failure), status {}:\n{}",
            output.status,
            child_text(&output)
        );
        output
    }

    fn child_text(output: &Output) -> String {
        format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    fn assert_failed(output: &Output, label: &str) {
        assert!(
            !output.status.success(),
            "{label} unexpectedly passed:\n{}",
            child_text(output)
        );
        assert!(
            child_text(output).contains("test result: FAILED"),
            "{label} did not execute the intended fixture:\n{}",
            child_text(output)
        );
    }

    fn end_event(output: &Output) -> Value {
        let text = child_text(output);
        let events: Vec<Value> = text
            .lines()
            .filter_map(|line| {
                line.find("PNP_PROPERTY_EVENT ").map(|i| {
                    serde_json::from_str(&line[i + "PNP_PROPERTY_EVENT ".len()..])
                        .expect("event JSON")
                })
            })
            .filter(|event: &Value| {
                event["phase"] == "end" && event["process_role"] == "controller"
            })
            .collect();
        assert_eq!(
            events.len(),
            1,
            "expected exactly one controller end event: {text}"
        );
        events.into_iter().next().expect("one event")
    }

    fn read_json(dir: &Path, name: &str) -> Value {
        serde_json::from_slice(
            &fs::read(dir.join(name)).unwrap_or_else(|e| panic!("read {name}: {e}")),
        )
        .expect("artifact JSON")
    }

    fn regression_paths() -> BTreeSet<PathBuf> {
        WalkDir::new(root())
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                !(entry.file_type().is_dir() && (name == "target" || name == ".git"))
            })
            .map(|entry| entry.expect("walk source directories"))
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("proptest-regressions")
            })
            .map(|entry| entry.into_path())
            .collect()
    }

    #[test]
    fn property_seed_replay_and_artifacts() {
        let before = regression_paths();
        let record = record();
        let dir_a = artifact_dir(&record);
        let dir_b = artifact_dir(&record);
        let seed = resolve_seed(Some(37));
        let context_a = build_property_context(
            &record,
            Tier::Routine,
            seed,
            &dir_a,
            resolved_proptest_version(),
            None,
        )
        .expect("first context");
        let context_b = build_property_context(
            &record,
            Tier::Routine,
            seed,
            &dir_b,
            resolved_proptest_version(),
            None,
        )
        .expect("second context");
        assert_eq!(context_a["source_revision"], context_b["source_revision"]);
        assert_eq!(
            context_a["source_state_digest"],
            context_b["source_state_digest"]
        );
        assert_eq!(context_a["profile"]["max_shrink_time_ms"], 0);
        let start = Instant::now();
        let out_a = run_child(&record.test_name, &context_a, None);
        let out_b = run_child(&record.test_name, &context_b, None);
        println!(
            "property replay child-spawn wall time (two cargo test children): {:?}",
            start.elapsed()
        );
        assert_failed(&out_a, "first replay");
        assert_failed(&out_b, "second replay");
        assert_eq!(end_event(&out_a)["outcome"], "failed");
        assert_eq!(end_event(&out_b)["outcome"], "failed");
        let a = fs::read_to_string(dir_a.join("case.txt")).expect("first case sequence");
        let b = fs::read_to_string(dir_b.join("case.txt")).expect("second case sequence");
        assert!(
            a.contains("phase initial") && a.contains("phase shrink"),
            "must exercise initial and shrink callbacks"
        );
        // test-quality: deterministic replay compares the actual case journal and retained minimum.
        assert_eq!(a, b);
        let failure_a = read_json(&dir_a, "failure.json");
        let failure_b = read_json(&dir_b, "failure.json");
        assert_eq!(failure_a["concrete_input"], "100");
        assert_eq!(failure_a["concrete_input"], failure_b["concrete_input"]);
        for (dir, expected, failure) in [
            (&dir_a, &context_a, &failure_a),
            (&dir_b, &context_b, &failure_b),
        ] {
            assert_eq!(read_json(dir, "context.json"), *expected);
            assert_eq!(failure["context"], *expected);
            assert_eq!(failure["classification"], "sut_or_contract");
            assert_eq!(
                failure["reproduction_command"],
                "cargo xtask test --summary --properties --property PBT-FIXTURE --tier routine --seed 37"
            );
            assert_eq!(
                failure["context"]["features"],
                json!(["slicer-sdk/property_testing"])
            );
            assert_eq!(
                failure["context"]["proptest_version"],
                resolved_proptest_version()
            );
            assert_eq!(failure["context"]["generator_version"], 1);
            assert_eq!(
                read_json(dir, "source-state.json")["source_state_digest"],
                expected["source_state_digest"]
            );
            assert!(!failure["context"]["source_revision"]
                .as_str()
                .expect("revision")
                .is_empty());
        }

        // Execute the artifact's direct invocation: the fixture is intentionally
        // not in the live registry, so its registry command cannot replay it.
        let direct = &failure_a["reproduction_direct"];
        let program = direct["program"].as_str().expect("direct replay program");
        let args = direct["args"]
            .as_array()
            .expect("direct replay arguments")
            .iter()
            .map(|arg| arg.as_str().expect("direct replay string argument"));
        let env = direct["env"]
            .as_object()
            .expect("direct replay environment")
            .iter()
            .map(|(key, value)| (key, value.as_str().expect("direct replay string value")));
        let mut command = Command::new(program);
        command
            .current_dir(root())
            .args(args)
            .envs(env)
            .env_remove("_PROPTEST_FORKFILE")
            .env_remove("PROPTEST_VERBOSE");
        for name in OVERRIDES {
            command.env_remove(name);
        }
        let direct_output = command.output().expect("execute emitted direct replay");
        println!(
            "emitted direct property replay status {}:\n{}",
            direct_output.status,
            child_text(&direct_output)
        );
        assert_eq!(direct_output.status.code(), Some(101));
        assert_failed(&direct_output, "emitted direct replay");
        assert!(child_text(&direct_output).contains(&format!("test {} ...", record.test_name)));
        assert!(child_text(&direct_output).contains("fixture threshold"));
        let direct_end = end_event(&direct_output);
        assert_eq!(direct_end["property_id"], record.id);
        assert_eq!(direct_end["error_kind"], "sut_failure");
        let replay_failure = read_json(&dir_a, "failure.json");
        assert_eq!(
            replay_failure["classification"],
            failure_a["classification"]
        );
        assert_eq!(
            replay_failure["concrete_input"],
            failure_a["concrete_input"]
        );
        // test-quality: deterministic direct replay must retain the same case journal.
        assert_eq!(
            fs::read_to_string(dir_a.join("case.txt")).expect("direct replay case sequence"),
            a
        );
        assert_eq!(
            before,
            regression_paths(),
            "no source-adjacent seed paths may be created"
        );
    }

    #[test]
    fn property_profile_rejects_override_and_exhaustion() {
        let mut record = record();
        let dir = artifact_dir(&record);
        let context = build_property_context(
            &record,
            Tier::Routine,
            37,
            &dir,
            resolved_proptest_version(),
            None,
        )
        .expect("fixture context");
        let mut mismatch = context.clone();
        mismatch["property_id"] = json!("PBT-WRONG");
        let output = run_child(&record.test_name, &mismatch, None);
        assert_failed(&output, "identity mismatch");
        assert!(child_text(&output).contains("context_identity_mismatch"));
        let output = run_child(&record.test_name, &context, Some(("PROPTEST_CASES", "1")));
        assert_failed(&output, "profile override");
        let end = end_event(&output);
        assert_eq!(end["outcome"], "failed");
        assert_eq!(end["error_kind"], "profile_override_conflict");
        assert_eq!(end["completion_eligible"], false);
        record.test_name = "proptest_fixture_all_rejecting".to_owned();
        let dir = artifact_dir(&record);
        let context = build_property_context(
            &record,
            Tier::Routine,
            37,
            &dir,
            resolved_proptest_version(),
            None,
        )
        .expect("rejecting context");
        let output = run_child(&record.test_name, &context, None);
        assert_failed(&output, "all-rejected strategy");
        let end = end_event(&output);
        assert_eq!(end["outcome"], "exhausted");
        assert_eq!(end["error_kind"], "exhausted");
        assert_eq!(end["accepted_cases"], Value::Null);
        assert_eq!(end["accepted_test_calls"], 0);
        assert_eq!(end["rejected_test_calls"], 4);
        assert_eq!(read_json(&dir, "context.json"), context);
        assert!(fs::read_to_string(dir.join("case.txt"))
            .expect("retained inputs")
            .contains("1234"));
        assert_eq!(
            read_json(&dir, "failure.json")["classification"],
            "exhausted"
        );
        assert_eq!(read_json(&dir, "failure.json")["concrete_input"], "1234");
    }
}
