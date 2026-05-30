//! `cargo xtask gen-config-docs`
//!
//! Regenerates the machine-derived sections of `docs/15_config_keys_reference.md`
//! so config-key defaults can never be hand-copied wrong again:
//!
//!   * **Module-owned config keys** — read directly from every
//!     `modules/core-modules/*/<name>.toml` `[config.schema.*]` table.
//!   * **Host-registered per-role speeds** — read from `docs/config/host-keys.toml`
//!     (which a slicer-runtime test locks to `FeedrateConfig::default()`).
//!   * **Deviations from OrcaSlicer** — generated keys whose numeric default
//!     differs from the matching key in `docs/ORCA_CONFIG_REFERENCE.md`.
//!
//! Each section lives between a `BEGIN GENERATED` / `END GENERATED` marker pair;
//! prose outside the markers is preserved.
//!
//! - default: rewrite the three generated blocks.
//! - `--check`: exit 1 if any block is out of date (CI guard).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// A config key destined for a generated table.
struct KeyRow {
    key: String,
    ty: String,
    default: String,
    /// Numeric default for Orca comparison, if the default is a number.
    default_num: Option<f64>,
    range: String,
    owner: String,
}

const MOD_BEGIN: &str =
    "<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->";
const MOD_END: &str = "<!-- END GENERATED: module-config-keys -->";
const SPD_BEGIN: &str = "<!-- BEGIN GENERATED: host-speeds (cargo xtask gen-config-docs) -->";
const SPD_END: &str = "<!-- END GENERATED: host-speeds -->";
const DEV_BEGIN: &str = "<!-- BEGIN GENERATED: orca-deviations (cargo xtask gen-config-docs) -->";
const DEV_END: &str = "<!-- END GENERATED: orca-deviations -->";

/// Format a TOML scalar default the way a reader expects (floats keep a decimal).
fn fmt_scalar(v: &toml::Value) -> Option<String> {
    match v {
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(fmt_float(*f)),
        toml::Value::Boolean(b) => Some(b.to_string()),
        toml::Value::String(s) => {
            // Sanitize for a single markdown table cell: collapse newlines,
            // escape pipes, cap length (full value lives in the source file).
            let one = s
                .replace('\\', "\\\\")
                .replace(['\n', '\r'], "\\n")
                .replace('|', "\\|");
            let capped = if one.chars().count() > 48 {
                format!("{}…", one.chars().take(47).collect::<String>())
            } else {
                one
            };
            Some(format!("\"{capped}\""))
        }
        toml::Value::Array(_) => Some("[…]".to_string()),
        _ => None,
    }
}

fn fmt_float(f: f64) -> String {
    if f == f.trunc() && f.is_finite() {
        format!("{f:.1}")
    } else {
        format!("{f}")
    }
}

fn num_of(v: &toml::Value) -> Option<f64> {
    match v {
        toml::Value::Integer(i) => Some(*i as f64),
        toml::Value::Float(f) => Some(*f),
        _ => None,
    }
}

fn fmt_range(min: Option<&toml::Value>, max: Option<&toml::Value>) -> String {
    match (min.and_then(num_of), max.and_then(num_of)) {
        (Some(lo), Some(hi)) => format!("[{}, {}]", fmt_float(lo), fmt_float(hi)),
        (Some(lo), None) => format!(">= {}", fmt_float(lo)),
        (None, Some(hi)) => format!("<= {}", fmt_float(hi)),
        (None, None) => "—".to_string(),
    }
}

/// Parse all module manifests into key rows (sorted by module, then key).
fn module_rows(ws: &Path) -> Result<Vec<KeyRow>, String> {
    let dir = ws.join("modules/core-modules");
    let mut module_dirs: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    module_dirs.sort();

    let mut rows = Vec::new();
    for mdir in module_dirs {
        let stem = mdir.file_name().unwrap().to_string_lossy().to_string();
        let manifest = mdir.join(format!("{stem}.toml"));
        if !manifest.exists() {
            continue;
        }
        let text = fs::read_to_string(&manifest)
            .map_err(|e| format!("cannot read {}: {e}", manifest.display()))?;
        let val: toml::Value = toml::from_str(&text)
            .map_err(|e| format!("parse error in {}: {e}", manifest.display()))?;
        let schema = match val
            .get("config")
            .and_then(|c| c.get("schema"))
            .and_then(|s| s.as_table())
        {
            Some(t) => t,
            None => continue,
        };
        let mut keys: Vec<_> = schema.iter().collect();
        keys.sort_by(|a, b| a.0.cmp(b.0));
        for (key, spec) in keys {
            let spec = match spec.as_table() {
                Some(t) => t,
                None => continue,
            };
            let ty = spec
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let default = spec
                .get("default")
                .and_then(fmt_scalar)
                .unwrap_or_else(|| "—".to_string());
            let default_num = spec.get("default").and_then(num_of);
            let range = fmt_range(spec.get("min"), spec.get("max"));
            rows.push(KeyRow {
                key: key.clone(),
                ty,
                default,
                default_num,
                range,
                owner: stem.clone(),
            });
        }
    }
    Ok(rows)
}

/// Infer a display type from a TOML scalar default.
fn infer_type(v: &toml::Value) -> &'static str {
    match v {
        toml::Value::Integer(_) => "int",
        toml::Value::Float(_) => "float",
        toml::Value::Boolean(_) => "bool",
        toml::Value::String(_) => "string",
        _ => "?",
    }
}

/// Parse one `[<table>]` of `docs/config/host-keys.toml` into key rows.
fn host_table_rows(val: &toml::Value, table: &str, owner: &str) -> Result<Vec<KeyRow>, String> {
    let entries = match val.get(table).and_then(|t| t.as_table()) {
        Some(t) => t,
        None => return Ok(Vec::new()),
    };
    let mut keys: Vec<_> = entries.iter().collect();
    keys.sort_by(|a, b| a.0.cmp(b.0));
    let mut rows = Vec::new();
    for (key, spec) in keys {
        let spec = spec
            .as_table()
            .ok_or_else(|| format!("[{table}.{key}] must be a table"))?;
        let default_v = spec
            .get("default")
            .ok_or_else(|| format!("[{table}.{key}] missing default"))?;
        let mut range = spec
            .get("range")
            .and_then(|v| v.as_str())
            .unwrap_or("—")
            .to_string();
        if let Some(note) = spec.get("note").and_then(|v| v.as_str()) {
            range = format!("{range} ({note})");
        }
        // A per-entry `owner` overrides the table default (keys in one table can
        // come from different consumer structs).
        let entry_owner = spec
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or(owner)
            .to_string();
        rows.push(KeyRow {
            key: key.clone(),
            ty: infer_type(default_v).to_string(),
            default: fmt_scalar(default_v).unwrap_or_else(|| "—".to_string()),
            default_num: num_of(default_v),
            range,
            owner: entry_owner,
        });
    }
    Ok(rows)
}

/// Parse all host-registered keys from `docs/config/host-keys.toml`.
fn host_rows(ws: &Path) -> Result<Vec<KeyRow>, String> {
    let path = ws.join("docs/config/host-keys.toml");
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let val: toml::Value =
        toml::from_str(&text).map_err(|e| format!("parse error in {}: {e}", path.display()))?;
    let mut rows = host_table_rows(&val, "speeds", "gcode_emit.rs::FeedrateConfig")?;
    rows.extend(host_table_rows(
        &val,
        "resolved_config",
        "resolved_config.rs::ResolvedConfig",
    )?);
    rows.extend(host_table_rows(&val, "host_runtime", "host config source")?);
    if rows.is_empty() {
        return Err(
            "host-keys.toml produced no rows ([speeds]/[resolved_config]/[host_runtime])"
                .to_string(),
        );
    }
    Ok(rows)
}

/// Parse the OrcaSlicer reference's 8-column tables into key -> numeric default.
fn orca_defaults(ws: &Path) -> BTreeMap<String, f64> {
    let path = ws.join("docs/ORCA_CONFIG_REFERENCE.md");
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return BTreeMap::new(),
    };
    let mut map = BTreeMap::new();
    for line in text.lines() {
        // Rows look like: | "key" | UI label | coFloat | 60 | mm | mode | desc | ✅ |
        if !line.starts_with("| \"") {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 6 {
            continue;
        }
        let ty = parts[3].trim();
        if !ty.starts_with("co") {
            continue; // skip the secondary 3-column tables
        }
        let key = parts[1].trim().trim_matches('"').to_string();
        if let Ok(n) = parts[4].trim().parse::<f64>() {
            map.entry(key).or_insert(n); // first (canonical) table wins
        }
    }
    map
}

fn render_table(rows: &[KeyRow], with_owner: bool, owner_header: &str) -> String {
    let mut s = String::new();
    if with_owner {
        s.push_str(&format!(
            "| Key | Type | Default | Range | {owner_header} |\n"
        ));
        s.push_str("|---|---|---|---|---|\n");
        for r in rows {
            s.push_str(&format!(
                "| `{}` | {} | `{}` | {} | `{}` |\n",
                r.key, r.ty, r.default, r.range, r.owner
            ));
        }
    } else {
        s.push_str("| Key | Type | Default | Range |\n");
        s.push_str("|---|---|---|---|\n");
        for r in rows {
            s.push_str(&format!(
                "| `{}` | {} | `{}` | {} |\n",
                r.key, r.ty, r.default, r.range
            ));
        }
    }
    s
}

fn render_deviations(all: &[&KeyRow], orca: &BTreeMap<String, f64>) -> String {
    // (key, owner, ours, theirs) — owner disambiguates the same key name defined
    // by more than one source with different defaults (e.g. `inner_wall_speed`
    // is 45 in the perimeter modules and 60 in host FeedrateConfig).
    let mut rows: Vec<(String, String, String, String)> = Vec::new();
    for r in all {
        if let (Some(ours), Some(theirs)) = (r.default_num, orca.get(&r.key)) {
            if (ours - theirs).abs() > 1e-9 {
                rows.push((
                    r.key.clone(),
                    r.owner.clone(),
                    r.default.clone(),
                    fmt_float(*theirs),
                ));
            }
        }
    }
    rows.sort();
    rows.dedup();
    if rows.is_empty() {
        return "_No numeric default deviates from the OrcaSlicer reference._\n".to_string();
    }
    let mut s = String::new();
    s.push_str("| Key | Owner | ModularSlicer default | OrcaSlicer default |\n");
    s.push_str("|---|---|---|---|\n");
    for (k, owner, ours, theirs) in rows {
        s.push_str(&format!("| `{k}` | `{owner}` | `{ours}` | `{theirs}` |\n"));
    }
    s
}

/// Replace content between `begin`/`end` markers (inclusive of the block body).
fn splice(content: &str, begin: &str, end: &str, body: &str) -> Result<String, String> {
    let b = content
        .find(begin)
        .ok_or_else(|| format!("missing marker `{begin}` in doc 15"))?;
    let e = content
        .find(end)
        .ok_or_else(|| format!("missing marker `{end}` in doc 15"))?;
    if e < b {
        return Err(format!("marker `{end}` precedes `{begin}` in doc 15"));
    }
    Ok(format!(
        "{}{begin}\n{body}{end}{}",
        &content[..b],
        &content[e + end.len()..]
    ))
}

pub fn run(ws: &Path, check_only: bool) -> i32 {
    let doc_path = ws.join("docs/15_config_keys_reference.md");
    let doc = match fs::read_to_string(&doc_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "xtask gen-config-docs: cannot read {}: {e}",
                doc_path.display()
            );
            return 2;
        }
    };

    let modules = match module_rows(ws) {
        Ok(r) => r,
        Err(m) => {
            eprintln!("xtask gen-config-docs: {m}");
            return 2;
        }
    };
    let hosts = match host_rows(ws) {
        Ok(r) => r,
        Err(m) => {
            eprintln!("xtask gen-config-docs: {m}");
            return 2;
        }
    };
    let orca = orca_defaults(ws);

    let all: Vec<&KeyRow> = modules.iter().chain(hosts.iter()).collect();

    let updated = {
        let mut d = doc.clone();
        let steps = [
            (MOD_BEGIN, MOD_END, render_table(&modules, true, "Module")),
            (SPD_BEGIN, SPD_END, render_table(&hosts, true, "Source")),
            (DEV_BEGIN, DEV_END, render_deviations(&all, &orca)),
        ];
        for (b, e, body) in steps {
            d = match splice(&d, b, e, &body) {
                Ok(s) => s,
                Err(m) => {
                    eprintln!("xtask gen-config-docs: {m}");
                    return 2;
                }
            };
        }
        d
    };

    let summary = format!(
        "{} module keys, {} host keys, {} Orca deviation(s)",
        modules.len(),
        hosts.len(),
        all.iter()
            .filter(|r| r
                .default_num
                .zip(orca.get(&r.key))
                .map(|(o, t)| (o - t).abs() > 1e-9)
                .unwrap_or(false))
            .count()
    );

    if check_only {
        if updated == doc {
            println!("OK: doc 15 generated sections current ({summary}).");
            0
        } else {
            eprintln!(
                "::error::docs/15_config_keys_reference.md generated sections are stale. \
                 Run `cargo xtask gen-config-docs` to regenerate."
            );
            1
        }
    } else if updated == doc {
        println!("doc 15 generated sections already current ({summary}).");
        0
    } else if let Err(e) = fs::write(&doc_path, updated) {
        eprintln!(
            "xtask gen-config-docs: cannot write {}: {e}",
            doc_path.display()
        );
        2
    } else {
        println!("Regenerated doc 15 sections ({summary}).");
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_formatting_keeps_decimal_for_whole_numbers() {
        assert_eq!(fmt_float(60.0), "60.0");
        assert_eq!(fmt_float(37.5), "37.5");
        assert_eq!(fmt_float(0.0125), "0.0125");
    }

    #[test]
    fn range_formatting() {
        let lo = toml::Value::Float(0.0);
        let hi = toml::Value::Float(1.0);
        assert_eq!(fmt_range(Some(&lo), Some(&hi)), "[0.0, 1.0]");
        assert_eq!(fmt_range(Some(&lo), None), ">= 0.0");
        assert_eq!(fmt_range(None, None), "—");
    }

    #[test]
    fn splice_replaces_block() {
        let doc = format!("a\n{MOD_BEGIN}\nold\n{MOD_END}\nb\n");
        let out = splice(&doc, MOD_BEGIN, MOD_END, "NEW\n").unwrap();
        assert!(out.contains(&format!("{MOD_BEGIN}\nNEW\n{MOD_END}")));
        assert!(!out.contains("old"));
    }

    #[test]
    fn deviations_flag_numeric_mismatch() {
        let rows = [KeyRow {
            key: "top_shell_layers".into(),
            ty: "int".into(),
            default: "3".into(),
            default_num: Some(3.0),
            range: "—".into(),
            owner: "host".into(),
        }];
        let refs: Vec<&KeyRow> = rows.iter().collect();
        let mut orca = BTreeMap::new();
        orca.insert("top_shell_layers".to_string(), 4.0);
        let table = render_deviations(&refs, &orca);
        assert!(table.contains("top_shell_layers"));
        assert!(table.contains("`3`"));
        assert!(table.contains("`4.0`"));
    }
}
