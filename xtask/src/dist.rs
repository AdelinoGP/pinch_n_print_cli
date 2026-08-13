use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::build_guests::{self, GuestSpec, GuestTree};
use crate::editions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DistArgs {
    pub edition: Option<String>,
    pub debug: bool,
    pub plan_only: bool,
}

pub(crate) fn parse_dist_args(args: &[String]) -> Result<DistArgs, String> {
    let mut parsed = DistArgs {
        edition: None,
        debug: false,
        plan_only: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--edition" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--edition requires a following <NAME> value".to_string())?;
                parsed.edition = Some(value.clone());
                i += 2;
            }
            "--debug" => {
                parsed.debug = true;
                i += 1;
            }
            "--plan" => {
                parsed.plan_only = true;
                i += 1;
            }
            other => return Err(format!("unknown flag '{other}' for dist")),
        }
    }
    Ok(parsed)
}

#[derive(Debug)]
pub(crate) struct DistPlan {
    pub edition: String,
    pub out_dir: PathBuf,
    pub cargo_features: Vec<String>,
    pub integrated: BTreeSet<String>,
    pub external_stage: Vec<GuestSpec>,
}

impl DistPlan {
    pub(crate) fn external_stems(&self) -> Vec<String> {
        let mut stems: Vec<String> = self.external_stage.iter().filter_map(stem_of).collect();
        stems.sort();
        stems
    }
}

fn stem_of(spec: &GuestSpec) -> Option<String> {
    spec.artifact_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_owned)
}

pub(crate) fn plan_edition(ws_root: &Path, edition: &str) -> Result<DistPlan, String> {
    let editions = editions::load_editions(ws_root)?;
    let spec = editions.get(edition).ok_or_else(|| {
        let available = editions
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "unknown edition '{edition}' in {} (available: {available})",
            editions::EDITIONS_CONFIG_PATH
        )
    })?;

    let (guests, _skips) = build_guests::discover_guests(ws_root);
    let mut core: Vec<GuestSpec> = guests
        .into_iter()
        .filter(|g| g.tree == GuestTree::Core)
        .collect();
    core.sort_by_key(|g| stem_of(g).unwrap_or_default());
    let core_stems: Vec<String> = core.iter().filter_map(stem_of).collect();

    let integrated: BTreeSet<String> = if spec.integrate_all {
        core_stems.iter().cloned().collect()
    } else {
        spec.integrated_modules.iter().cloned().collect()
    };

    let external_stage: Vec<GuestSpec> = core
        .into_iter()
        .filter(|g| stem_of(g).is_some_and(|s| !integrated.contains(&s)))
        .collect();

    let cargo_features: Vec<String> = integrated
        .iter()
        .map(|name| format!("integrated-{name}"))
        .collect();

    Ok(DistPlan {
        edition: edition.to_owned(),
        out_dir: ws_root.join("target").join("dist").join(edition),
        cargo_features,
        integrated,
        external_stage,
    })
}

pub(crate) fn assert_staging_disjoint(
    edition: &str,
    integrated: &BTreeSet<String>,
    staged: &[String],
) -> Result<(), String> {
    for name in staged {
        if integrated.contains(name) {
            return Err(format!(
                "edition '{edition}': module '{name}' is both integrated and externally staged; \
                 the integrated and staged module sets must be disjoint \
                 (see docs/adr/0056-integrated-modules-native-dispatch.md)"
            ));
        }
    }
    Ok(())
}

pub(crate) fn pnp_cli_integrated_features(ws_root: &Path) -> Result<BTreeSet<String>, String> {
    let manifest_path = ws_root.join("crates").join("pnp-cli").join("Cargo.toml");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("crates/pnp-cli/Cargo.toml: unable to read manifest: {e}"))?;
    let table: toml::Table = toml::from_str(&content)
        .map_err(|e| format!("crates/pnp-cli/Cargo.toml: invalid TOML: {e}"))?;
    let features = table
        .get("features")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "crates/pnp-cli/Cargo.toml: missing [features] table".to_string())?;
    Ok(features
        .keys()
        .filter(|key| key.starts_with("integrated-"))
        .cloned()
        .collect())
}

pub(crate) fn verify_integrated_feature_coverage(
    edition: &str,
    integrated: &BTreeSet<String>,
    available: &BTreeSet<String>,
) -> Result<(), String> {
    let missing: Vec<String> = integrated
        .iter()
        .map(|name| format!("integrated-{name}"))
        .filter(|feature| !available.contains(feature))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "edition '{edition}': crates/pnp-cli/Cargo.toml lacks integrated feature(s) {} \
         for integrated module(s); add one `integrated-<name>` passthrough feature per module",
        missing.join(", ")
    ))
}

pub(crate) fn preflight_edition(ws_root: &Path, edition: &str) -> Result<DistPlan, String> {
    let plan = plan_edition(ws_root, edition)?;
    let available = pnp_cli_integrated_features(ws_root)?;
    verify_integrated_feature_coverage(&plan.edition, &plan.integrated, &available)?;
    assert_staging_disjoint(&plan.edition, &plan.integrated, &plan.external_stems())?;
    Ok(plan)
}

pub(crate) fn print_plan(plan: &DistPlan) {
    println!("edition\t{}", plan.edition);
    println!("out_dir\t{}", plan.out_dir.display());
    println!("features\t{}", plan.cargo_features.join(","));
    for name in &plan.integrated {
        println!("integrated\t{name}");
    }
    for name in plan.external_stems() {
        println!("external\t{name}");
    }
}

pub(crate) fn dist_command(ws_root: &Path, args: &DistArgs) -> i32 {
    let edition = args.edition.as_deref().unwrap_or("developer");
    let plan = match preflight_edition(ws_root, edition) {
        Ok(plan) => plan,
        Err(e) => {
            eprintln!("xtask dist: {e}");
            return 1;
        }
    };

    if args.plan_only {
        print_plan(&plan);
        return 0;
    }

    let profile = if args.debug { "debug" } else { "release" };

    println!("xtask dist: building guest WASMs...");
    let code = build_guests::build_command(ws_root);
    if code != 0 {
        return code;
    }

    println!("xtask dist: building pnp_cli ({profile})...");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(ws_root).args(["build", "-p", "pnp-cli"]);
    if !args.debug {
        cmd.arg("--release");
    }
    if !plan.cargo_features.is_empty() {
        cmd.arg("--features").arg(plan.cargo_features.join(","));
    }
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("xtask dist: failed to spawn cargo: {e}");
            return 1;
        }
    };
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        eprintln!(
            "xtask dist: cargo build -p pnp-cli failed:\n{}",
            build_guests::tail_lines(&stderr, 20)
        );
        return 1;
    }

    let dist_dir = &plan.out_dir;
    match fs::remove_dir_all(dist_dir) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("xtask dist: failed to clean {}: {e}", dist_dir.display());
            return 1;
        }
    }
    if let Err(e) = fs::create_dir_all(dist_dir) {
        eprintln!("xtask dist: failed to create {}: {e}", dist_dir.display());
        return 1;
    }

    let bin_name = if cfg!(target_os = "windows") {
        "pnp_cli.exe"
    } else {
        "pnp_cli"
    };
    let bin_src = ws_root.join("target").join(profile).join(bin_name);
    let bin_dest = dist_dir.join(bin_name);
    if let Err(e) = fs::copy(&bin_src, &bin_dest) {
        eprintln!(
            "xtask dist: failed to copy {} -> {}: {e}",
            bin_src.display(),
            bin_dest.display()
        );
        return 1;
    }

    let modules_dir = dist_dir.join("modules");
    if let Err(e) = fs::create_dir_all(&modules_dir) {
        eprintln!(
            "xtask dist: failed to create {}: {e}",
            modules_dir.display()
        );
        return 1;
    }

    let mut module_count = 0usize;
    for spec in &plan.external_stage {
        let wasm_src = ws_root.join(&spec.artifact_path);
        let toml_src = wasm_src.with_extension("toml");
        let stem = match stem_of(spec) {
            Some(s) => s,
            None => {
                eprintln!(
                    "xtask dist: artifact_path missing stem: {}",
                    spec.artifact_path.display()
                );
                return 1;
            }
        };
        let dest_dir = modules_dir.join(&stem);
        if let Err(e) = fs::create_dir_all(&dest_dir) {
            eprintln!("xtask dist: failed to create {}: {e}", dest_dir.display());
            return 1;
        }
        let wasm_dest = dest_dir.join(format!("{stem}.wasm"));
        let toml_dest = dest_dir.join(format!("{stem}.toml"));
        if let Err(e) = fs::copy(&wasm_src, &wasm_dest) {
            eprintln!(
                "xtask dist: failed to copy {} -> {}: {e}",
                wasm_src.display(),
                wasm_dest.display()
            );
            return 1;
        }
        if let Err(e) = fs::copy(&toml_src, &toml_dest) {
            eprintln!(
                "xtask dist: failed to copy {} -> {}: {e}",
                toml_src.display(),
                toml_dest.display()
            );
            return 1;
        }
        module_count += 1;
    }

    let mut staged: Vec<String> = Vec::new();
    let entries = match fs::read_dir(&modules_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!(
                "xtask dist: failed to re-read {}: {e}",
                modules_dir.display()
            );
            return 1;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!(
                    "xtask dist: failed to read an entry in {}: {e}",
                    modules_dir.display()
                );
                return 1;
            }
        };
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            if let Some(name) = entry.file_name().to_str() {
                staged.push(name.to_owned());
            }
        }
    }
    staged.sort();
    if let Err(e) = assert_staging_disjoint(&plan.edition, &plan.integrated, &staged) {
        eprintln!("xtask dist: {e}");
        return 1;
    }

    println!(
        "xtask dist: edition '{}' staged 1 binary + {module_count} modules into {}",
        plan.edition,
        dist_dir.display()
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| (*s).to_string()).collect()
    }

    fn core_stems() -> BTreeSet<String> {
        let ws_root = build_guests::workspace_root();
        let (guests, _skips) = build_guests::discover_guests(&ws_root);
        guests
            .iter()
            .filter(|g| g.tree == GuestTree::Core)
            .filter_map(stem_of)
            .collect()
    }

    #[test]
    fn dist_plan_developer_stages_every_core_module() {
        let ws_root = build_guests::workspace_root();
        let plan = plan_edition(&ws_root, "developer").expect("developer edition plans");
        assert!(plan.cargo_features.is_empty());
        assert!(plan.integrated.is_empty());
        let staged: BTreeSet<String> = plan.external_stems().into_iter().collect();
        assert_eq!(staged, core_stems());
        assert!(plan
            .out_dir
            .ends_with(Path::new("target").join("dist").join("developer")));
    }

    #[test]
    fn dist_plan_hybrid_derives_features_and_complement() {
        let ws_root = build_guests::workspace_root();
        let editions = editions::load_editions(&ws_root).expect("editions config loads");
        let hybrid_integrated: BTreeSet<String> = editions["hybrid"]
            .integrated_modules
            .iter()
            .cloned()
            .collect();
        let plan = plan_edition(&ws_root, "hybrid").expect("hybrid edition plans");
        assert_eq!(plan.integrated, hybrid_integrated);
        let expected_features: Vec<String> = hybrid_integrated
            .iter()
            .map(|name| format!("integrated-{name}"))
            .collect();
        assert_eq!(plan.cargo_features, expected_features);
        let core = core_stems();
        let expected_external: BTreeSet<String> =
            core.difference(&hybrid_integrated).cloned().collect();
        let staged: BTreeSet<String> = plan.external_stems().into_iter().collect();
        assert_eq!(staged, expected_external);
        assert_eq!(
            plan.external_stage.len() + plan.integrated.len(),
            core.len()
        );
        assert!(staged.is_disjoint(&plan.integrated));
    }

    #[test]
    fn dist_plan_integrated_stages_nothing_externally() {
        let ws_root = build_guests::workspace_root();
        let plan = plan_edition(&ws_root, "integrated").expect("integrated edition plans");
        let core = core_stems();
        assert_eq!(plan.integrated, core);
        assert!(plan.external_stage.is_empty());
        let expected_features: Vec<String> = core
            .iter()
            .map(|name| format!("integrated-{name}"))
            .collect();
        assert_eq!(plan.cargo_features, expected_features);
    }

    #[test]
    fn dist_arg_parsing_accepts_edition_and_debug_in_any_order() {
        for args in [
            strings(&["--edition", "hybrid", "--debug"]),
            strings(&["--debug", "--edition", "hybrid"]),
        ] {
            let parsed = parse_dist_args(&args).expect("valid flag order parses");
            assert_eq!(parsed.edition.as_deref(), Some("hybrid"));
            assert!(parsed.debug);
            assert!(!parsed.plan_only);
        }
        let plan_only = parse_dist_args(&strings(&["--plan"])).expect("--plan parses");
        assert!(plan_only.plan_only);
        assert_eq!(plan_only.edition, None);
        assert!(!plan_only.debug);
        let defaults = parse_dist_args(&strings(&[])).expect("empty args parse");
        assert_eq!(defaults.edition, None);
        assert!(!defaults.debug);
        assert!(!defaults.plan_only);
        let err = parse_dist_args(&strings(&["--edition"]))
            .expect_err("--edition without a value must fail");
        assert!(err.contains("--edition"));
        assert!(parse_dist_args(&strings(&["--nope"])).is_err());
    }

    #[test]
    fn dist_disjointness_rejects_integrated_module_in_staged_set() {
        let integrated: BTreeSet<String> = ["x".to_string()].into_iter().collect();
        let err = assert_staging_disjoint("hybrid", &integrated, &["x".to_string()])
            .expect_err("overlap must fail");
        assert!(err.contains('x'));
        assert!(err.contains("hybrid"));
        assert!(err.contains("disjoint"));
        assert!(assert_staging_disjoint("hybrid", &integrated, &["y".to_string()]).is_ok());
    }

    #[test]
    fn dist_registry_coverage_rejects_missing_pnp_cli_feature() {
        let integrated: BTreeSet<String> = ["y".to_string()].into_iter().collect();
        let available: BTreeSet<String> = BTreeSet::new();
        let err = verify_integrated_feature_coverage("hybrid", &integrated, &available)
            .expect_err("missing feature must fail");
        assert!(err.contains('y'));
        assert!(err.contains("hybrid"));
        assert!(err.contains("crates/pnp-cli/Cargo.toml"));
    }
}
