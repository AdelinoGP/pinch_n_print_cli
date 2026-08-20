//! `cargo xtask sync-agents`
//!
//! `.agents/` is the canonical, harness-agnostic home for agent tooling
//! (skills, agent bodies, reference files) per the Agent Skills standard
//! (agentskills.io). Harnesses that do not read `.agents/` natively get a
//! generated mirror under their own directory:
//!
//! - default: materialize `.claude/skills/`, `.claude/agents/`, and
//!   `CLAUDE.md` from the canonical home. All three are gitignored; the
//!   mirror is disposable and regenerated on demand.
//! - `--check`: lint the canonical home — no `.claude/` path references in
//!   `.agents/**` or the live docs, and every skill's frontmatter carries
//!   `name` + `description` with `name` matching its directory. CI runs this
//!   lint; the mirror itself is not committed, so there is nothing to
//!   compare against.

use std::fs;
use std::path::Path;

use walkdir::WalkDir;

/// Live surfaces that must never name a harness directory. Historical spec
/// packets and `docs/specs/*` plans are frozen and intentionally excluded.
const LIVE_DOCS: &[&str] = &[
    "AGENTS.md",
    "CONTEXT.md",
    "docs/00_project_overview.md",
    "docs/17_agent_debugging.md",
    "docs/19_visual_debug.md",
    "docs/21_data_defaults_and_fixtures.md",
    "docs/spec_packets/README.md",
];

/// Harness-specific strings that must not appear in the canonical home or
/// live docs. `.claude/` covers path references; `@.claude` covers the
/// Claude-Code `@`-import syntax (which other harnesses do not resolve).
const HARNESS_MARKERS: &[&str] = &[".claude/", "@.claude"];

pub fn run(ws: &Path, check_only: bool) -> i32 {
    if check_only {
        lint_canonical_home(ws)
    } else {
        sync(ws)
    }
}

// ---------------------------------------------------------------------------
// Sync: materialize the `.claude/` mirror from the canonical home.
// ---------------------------------------------------------------------------

fn sync(ws: &Path) -> i32 {
    let mut code = 0;
    if let Err(e) = sync_skills(ws) {
        eprintln!("sync-agents: skills sync failed: {e}");
        code = 1;
    }
    if let Err(e) = sync_memory(ws) {
        eprintln!("sync-agents: memory sync failed: {e}");
        code = 1;
    }
    if let Err(e) = sync_agents(ws) {
        eprintln!("sync-agents: agents sync failed: {e}");
        code = 1;
    }
    if code == 0 {
        println!("sync-agents: .claude/ mirror regenerated from .agents/");
    }
    code
}

fn sync_skills(ws: &Path) -> Result<(), String> {
    let src = ws.join(".agents/skills");
    let dst = ws.join(".claude/skills");
    replace_dir(&src, &dst)
}

fn sync_memory(ws: &Path) -> Result<(), String> {
    let src = ws.join("AGENTS.md");
    let dst = ws.join("CLAUDE.md");
    let content = fs::read_to_string(&src).map_err(|e| format!("read {}: {e}", src.display()))?;
    fs::write(&dst, content).map_err(|e| format!("write {}: {e}", dst.display()))
}

fn sync_agents(ws: &Path) -> Result<(), String> {
    let src_dir = ws.join(".agents/agents");
    let dst_dir = ws.join(".claude/agents");
    if !src_dir.exists() {
        // No canonical agents — nothing to mirror.
        if dst_dir.exists() {
            fs::remove_dir_all(&dst_dir)
                .map_err(|e| format!("remove {}: {e}", dst_dir.display()))?;
        }
        return Ok(());
    }
    if dst_dir.exists() {
        fs::remove_dir_all(&dst_dir).map_err(|e| format!("remove {}: {e}", dst_dir.display()))?;
    }
    fs::create_dir_all(&dst_dir).map_err(|e| format!("create {}: {e}", dst_dir.display()))?;

    for entry in WalkDir::new(&src_dir).max_depth(1).into_iter() {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let canonical =
            fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let fm = parse_frontmatter(&canonical)
            .ok_or_else(|| format!("{}: missing frontmatter", path.display()))?;
        let name = fm
            .get("name")
            .ok_or_else(|| format!("{}: frontmatter missing `name`", path.display()))?;
        let description = fm
            .get("description")
            .ok_or_else(|| format!("{}: frontmatter missing `description`", path.display()))?;

        // Thin adapter: harness metadata + a pointer to the canonical body.
        let adapter = format!(
            "---\nname: {name}\ndescription: {description}\ntools: Bash, Read, Grep, Glob\nmodel: sonnet\n---\n\n\
             # {name} subagent\n\n\
             Read `.agents/agents/{name}.md` and follow it.\n"
        );
        let dst = dst_dir.join(format!("{name}.md"));
        fs::write(&dst, adapter).map_err(|e| format!("write {}: {e}", dst.display()))?;
    }
    Ok(())
}

/// Remove `dst` if present, then copy `src` recursively into it.
fn replace_dir(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|e| format!("remove {}: {e}", dst.display()))?;
    }
    copy_dir_recursive(src, dst)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("create {}: {e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to)
                .map_err(|e| format!("copy {} -> {}: {e}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Check: lint the canonical home.
// ---------------------------------------------------------------------------

fn lint_canonical_home(ws: &Path) -> i32 {
    let mut violations = Vec::new();

    // 1. No harness markers anywhere under .agents/.
    for entry in WalkDir::new(ws.join(".agents")).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                violations.push(format!("walk .agents/: {e}"));
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                violations.push(format!("read {}: {e}", path.display()));
                continue;
            }
        };
        for (line_no, line) in content.lines().enumerate() {
            for marker in HARNESS_MARKERS {
                if line.contains(marker) {
                    violations.push(format!(
                        "{}:{}: harness reference `{marker}` in canonical home",
                        path.display(),
                        line_no + 1
                    ));
                }
            }
        }
    }

    // 2. No harness markers in the live docs.
    for rel in LIVE_DOCS {
        let path = ws.join(rel);
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                violations.push(format!("read {}: {e}", path.display()));
                continue;
            }
        };
        for (line_no, line) in content.lines().enumerate() {
            for marker in HARNESS_MARKERS {
                if line.contains(marker) {
                    violations.push(format!(
                        "{}:{}: harness reference `{marker}` in live doc",
                        path.display(),
                        line_no + 1
                    ));
                }
            }
        }
    }

    // 3. Every skill has valid frontmatter and a name matching its directory.
    let skills_dir = ws.join(".agents/skills");
    if let Ok(entries) = fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let skill_file = dir.join("SKILL.md");
            let content = match fs::read_to_string(&skill_file) {
                Ok(c) => c,
                Err(e) => {
                    violations.push(format!("read {}: {e}", skill_file.display()));
                    continue;
                }
            };
            let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            match parse_frontmatter(&content) {
                None => violations.push(format!("{}: missing frontmatter", skill_file.display())),
                Some(fm) => {
                    if !fm.contains_key("name") {
                        violations.push(format!(
                            "{}: frontmatter missing `name`",
                            skill_file.display()
                        ));
                    } else if fm.get("name").map(String::as_str) != Some(dir_name) {
                        violations.push(format!(
                            "{}: frontmatter `name` does not match directory `{dir_name}`",
                            skill_file.display()
                        ));
                    }
                    if !fm.contains_key("description") {
                        violations.push(format!(
                            "{}: frontmatter missing `description`",
                            skill_file.display()
                        ));
                    }
                }
            }
        }
    } else {
        violations.push(format!("read {}: not a directory", skills_dir.display()));
    }

    if violations.is_empty() {
        println!("sync-agents --check: canonical home clean");
        0
    } else {
        for v in &violations {
            eprintln!("sync-agents --check: {v}");
        }
        eprintln!("sync-agents --check: {} violation(s)", violations.len());
        1
    }
}

/// Parse a `SKILL.md`-style YAML frontmatter block into key -> value pairs.
/// Returns `None` if the file does not open with a `---` block. Values are
/// the raw remainder of the line after the key and colon, trimmed.
fn parse_frontmatter(content: &str) -> Option<std::collections::BTreeMap<String, String>> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut map = std::collections::BTreeMap::new();
    for line in lines {
        if line.trim() == "---" {
            return Some(map);
        }
        if let Some((key, value)) = line.split_once(':') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "pnp-xtask-sync-agents-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create temporary test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write(ws: &Path, rel: &str, content: &str) {
        let path = ws.join(rel);
        fs::create_dir_all(path.parent().expect("file has parent")).expect("create parent dir");
        fs::write(path, content).expect("write test file");
    }

    const SKILL: &str = "---\nname: demo\ndescription: A demo skill.\n---\n\n# Demo\n";

    #[test]
    fn parse_frontmatter_reads_name_and_description() {
        let fm = parse_frontmatter(SKILL).expect("frontmatter present");
        assert_eq!(fm.get("name").map(String::as_str), Some("demo"));
        assert_eq!(
            fm.get("description").map(String::as_str),
            Some("A demo skill.")
        );
    }

    #[test]
    fn parse_frontmatter_rejects_missing_block() {
        assert!(parse_frontmatter("# No frontmatter\n").is_none());
        assert!(parse_frontmatter("---\nname: demo\n").is_none()); // never closed
    }

    #[test]
    fn lint_flags_harness_reference_in_skill() {
        let tmp = TempDir::new();
        let ws = tmp.path();
        write(
            ws,
            ".agents/skills/demo/SKILL.md",
            "---\nname: demo\ndescription: A demo skill.\n---\n\nSee `.claude/skills/other/SKILL.md`.\n",
        );
        write(ws, "AGENTS.md", "# AGENTS.md\n");
        write(ws, "CONTEXT.md", "# Context\n");
        for rel in LIVE_DOCS.iter().skip(2) {
            write(ws, rel, "# doc\n");
        }
        let code = lint_canonical_home(ws);
        assert_eq!(code, 1, "harness reference in a skill must fail the lint");
    }

    #[test]
    fn lint_flags_name_mismatch_and_missing_description() {
        let tmp = TempDir::new();
        let ws = tmp.path();
        write(
            ws,
            ".agents/skills/demo/SKILL.md",
            "---\nname: other\n---\n\n# Demo\n",
        );
        write(ws, "AGENTS.md", "# AGENTS.md\n");
        write(ws, "CONTEXT.md", "# Context\n");
        for rel in LIVE_DOCS.iter().skip(2) {
            write(ws, rel, "# doc\n");
        }
        let code = lint_canonical_home(ws);
        assert_eq!(code, 1, "name mismatch and missing description must fail");
    }

    #[test]
    fn lint_passes_clean_home() {
        let tmp = TempDir::new();
        let ws = tmp.path();
        write(ws, ".agents/skills/demo/SKILL.md", SKILL);
        write(
            ws,
            ".agents/agents/demo.md",
            "---\nname: demo\ndescription: A demo agent.\n---\n\nBody.\n",
        );
        write(ws, "AGENTS.md", "# AGENTS.md\n");
        write(ws, "CONTEXT.md", "# Context\n");
        for rel in LIVE_DOCS.iter().skip(2) {
            write(ws, rel, "# doc\n");
        }
        assert_eq!(lint_canonical_home(ws), 0);
    }

    #[test]
    fn sync_materializes_mirror_and_thin_agent_adapter() {
        let tmp = TempDir::new();
        let ws = tmp.path();
        write(ws, ".agents/skills/demo/SKILL.md", SKILL);
        write(
            ws,
            ".agents/agents/demo.md",
            "---\nname: demo\ndescription: A demo agent.\n---\n\nCanonical body.\n",
        );
        write(ws, "AGENTS.md", "# AGENTS.md\n");

        assert_eq!(sync(ws), 0);

        let claude_md = fs::read_to_string(ws.join("CLAUDE.md")).expect("CLAUDE.md generated");
        assert_eq!(claude_md, "# AGENTS.md\n");

        let mirrored =
            fs::read_to_string(ws.join(".claude/skills/demo/SKILL.md")).expect("skill mirrored");
        assert_eq!(mirrored, SKILL);

        let adapter =
            fs::read_to_string(ws.join(".claude/agents/demo.md")).expect("agent adapter generated");
        assert!(adapter.contains("name: demo"), "adapter carries name");
        assert!(
            adapter.contains("Read `.agents/agents/demo.md` and follow it."),
            "adapter points at the canonical body"
        );
        assert!(
            !adapter.contains("Canonical body."),
            "adapter is thin — canonical body not duplicated"
        );
    }

    #[test]
    fn sync_drops_skills_removed_from_canonical_home() {
        let tmp = TempDir::new();
        let ws = tmp.path();
        write(ws, ".agents/skills/demo/SKILL.md", SKILL);
        write(ws, "AGENTS.md", "# AGENTS.md\n");
        assert_eq!(sync(ws), 0);
        assert!(ws.join(".claude/skills/demo/SKILL.md").exists());

        // Remove the skill from the canonical home; the mirror must drop it.
        fs::remove_dir_all(ws.join(".agents/skills/demo")).expect("remove canonical skill");
        assert_eq!(sync(ws), 0);
        assert!(
            !ws.join(".claude/skills/demo").exists(),
            "stale mirror entry must be removed"
        );
    }
}
