use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use crate::build_guests::{discover_guests, GuestTree};

// packet 205 consumes this staged export surface
#[allow(dead_code)]
pub const EDITIONS_CONFIG_PATH: &str = "dist/editions.toml";

#[derive(Debug, Clone, PartialEq)]
// packet 205 consumes this staged export surface
#[allow(dead_code)]
pub struct EditionSpec {
    pub integrate_all: bool,
    pub integrated_modules: Vec<String>,
}

// packet 205 consumes this staged export surface
#[allow(dead_code)]
pub fn load_editions(ws_root: &Path) -> Result<BTreeMap<String, EditionSpec>, String> {
    load_editions_from(ws_root, &ws_root.join(EDITIONS_CONFIG_PATH))
}

fn load_editions_from(
    ws_root: &Path,
    config_path: &Path,
) -> Result<BTreeMap<String, EditionSpec>, String> {
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("{EDITIONS_CONFIG_PATH}: unable to read config: {e}"))?;
    let table: toml::Table = toml::from_str(&content)
        .map_err(|e| format!("{EDITIONS_CONFIG_PATH}: invalid TOML: {e}"))?;
    let edition_table = table
        .get("edition")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("{EDITIONS_CONFIG_PATH}: missing [edition] table"))?;

    let mut editions = BTreeMap::new();
    for (name, value) in edition_table {
        let spec = value
            .as_table()
            .ok_or_else(|| format!("{EDITIONS_CONFIG_PATH}: edition '{name}' is not a table"))?;
        let integrate_all = spec
            .get("integrate_all")
            .and_then(toml::Value::as_bool)
            .ok_or_else(|| {
                format!("{EDITIONS_CONFIG_PATH}: edition '{name}' has invalid integrate_all")
            })?;
        let integrated_modules = spec
            .get("integrated_modules")
            .and_then(toml::Value::as_array)
            .ok_or_else(|| {
                format!("{EDITIONS_CONFIG_PATH}: edition '{name}' has invalid integrated_modules")
            })?
            .iter()
            .map(|module| {
                module.as_str().map(str::to_owned).ok_or_else(|| {
                    format!("{EDITIONS_CONFIG_PATH}: edition '{name}' has a non-string module name")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        editions.insert(
            name.clone(),
            EditionSpec {
                integrate_all,
                integrated_modules,
            },
        );
    }

    let (guests, _skips) = discover_guests(ws_root);
    let known_stems = guests
        .into_iter()
        .filter(|guest| guest.tree == GuestTree::Core)
        .filter_map(|guest| {
            guest
                .artifact_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_owned)
        })
        .collect::<HashSet<_>>();
    validate_edition_names(&editions, &known_stems)?;
    Ok(editions)
}

// packet 205 consumes this staged export surface; invoked only from load_editions
#[allow(dead_code)]
fn validate_edition_names(
    editions: &BTreeMap<String, EditionSpec>,
    known_stems: &HashSet<String>,
) -> Result<(), String> {
    for edition in editions.values() {
        for name in &edition.integrated_modules {
            if !known_stems.contains(name) {
                return Err(format!("{EDITIONS_CONFIG_PATH}: unknown module '{name}'"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a workspace parent")
            .to_path_buf()
    }

    #[test]
    fn editions_config_declares_three_editions() {
        let editions = load_editions(&workspace_root()).expect("editions config loads");
        assert_eq!(
            editions.keys().collect::<Vec<_>>(),
            vec!["developer", "hybrid", "integrated"]
        );
        assert_eq!(
            editions["developer"],
            EditionSpec {
                integrate_all: false,
                integrated_modules: vec![]
            }
        );
        assert_eq!(
            editions["hybrid"].integrated_modules,
            vec![
                "classic-perimeters",
                "arachne-perimeters",
                "tree-support-planner"
            ]
        );
        assert!(!editions["hybrid"].integrate_all);
        assert!(editions["integrated"].integrate_all);
    }

    #[test]
    fn editions_config_rejects_unknown_module_name() {
        let root = workspace_root();
        let temp_dir = std::env::temp_dir().join(format!("pnp-editions-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("create temporary editions directory");
        let config_path = temp_dir.join("editions.toml");
        fs::write(
            &config_path,
            r#"[edition.developer]
integrate_all = false
integrated_modules = []

[edition.hybrid]
integrate_all = false
integrated_modules = ["not-a-module"]

[edition.integrated]
integrate_all = true
integrated_modules = []
"#,
        )
        .expect("write temporary editions config");
        let error = load_editions_from(&root, &config_path)
            .expect_err("unknown module must fail through production loader");
        fs::remove_dir_all(&temp_dir).expect("remove temporary editions directory");
        assert!(error.contains("not-a-module"));
        assert!(error.contains(EDITIONS_CONFIG_PATH));
    }
}
