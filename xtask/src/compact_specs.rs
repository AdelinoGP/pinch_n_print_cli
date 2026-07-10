//! `cargo xtask compact-specs`
//!
//! Collapses each archived spec packet under `.ralph/specs/_OLD/<NN_slug>/` into a
//! single **design-only** digest `.ralph/specs/_OLD/<NN_slug>.md`, then deletes the
//! source directory (unless `--dry-run`).
//!
//! The packets are fully git-tracked, so the originals live in history forever; the
//! digest is a convenience artifact, not the last surviving copy. That lets us prune
//! implementation scaffolding aggressively.
//!
//! Design and implementation content are interleaved at the `##`-section level, so
//! selection happens per-section against a small **allowlist** of design headers:
//!
//!   * `packet.spec.md`  → `## Goal`
//!   * `requirements.md` → `## Problem Statement`
//!   * `design.md`       → Architecture Constraints, Data and Contract Notes,
//!     Risks and Tradeoffs, Locked Assumptions and Invariants, Implementation Deviations
//!
//! `implementation-plan.md` and `task-map.md` are dropped whole. If a kept file yields
//! zero recognized design sections (the phase/step-structured `design.md` outliers), the
//! whole file body is kept as a fallback rather than emitting nothing.
//!
//! The `packet.spec.md` YAML front-matter is line-filtered (no YAML parser needed) to
//! preserve `packet`/`status`/`supersedes`/`superseded_by`/`task_ids`.

use std::fs;
use std::path::{Path, PathBuf};

/// Squished design-header stems kept from `packet.spec.md`.
const SPEC_ALLOW: &[&str] = &["goal"];
/// Squished design-header stems kept from `requirements.md`.
const REQ_ALLOW: &[&str] = &["problemstatement", "problem", "motivation"];
/// Squished design-header stems kept from `design.md`.
const DESIGN_ALLOW: &[&str] = &[
    "architectureconstraints",
    "dataandcontractnotes",
    "risksandtradeoffs",
    "lockedassumptionsandinvariants",
    "assumptionsandinvariants",
    "implementationdeviations",
    "deviations",
];

/// Front-matter keys preserved in the digest (all others dropped).
const FM_KEEP_KEYS: &[&str] = &[
    "packet",
    "status",
    "supersedes",
    "superseded_by",
    "task_ids",
];

/// One `##` section: its squished header key plus the verbatim block (header + body).
struct Section {
    key: String,
    text: String,
}

/// Normalize a header to a comparison key: lowercase, drop any trailing `(...)`
/// qualifier, fold `&`→`and`, then keep only ASCII alphanumerics (spaces/hyphens
/// removed). So `Risks & Trade-offs`, `Risks and Tradeoffs` → `risksandtradeoffs`.
fn squish(header: &str) -> String {
    let mut s = header.trim().to_lowercase();
    if let Some(p) = s.find('(') {
        s.truncate(p);
    }
    let s = s.replace('&', "and");
    s.chars().filter(char::is_ascii_alphanumeric).collect()
}

/// Split a leading `---`…`---` YAML front-matter block off the front of `text`.
/// Returns `(front_matter_inner, body)` — `inner` excludes the fence lines. If there
/// is no well-formed leading front-matter, returns `(None, text)`.
fn split_front_matter(text: &str) -> (Option<String>, String) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let first_nl = match text.find('\n') {
        Some(i) => i,
        None => return (None, text.to_string()),
    };
    if text[..first_nl].trim_end_matches(['\r']) != "---" {
        return (None, text.to_string());
    }
    let rest = &text[first_nl + 1..];
    let mut off = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']) == "---" {
            let inner = rest[..off].to_string();
            let body = rest[off + line.len()..].to_string();
            return (Some(inner), body);
        }
        off += line.len();
    }
    (None, text.to_string())
}

/// Drop a trailing ` # …` inline comment (the packet front-matter values never contain
/// a quoted `#`, so a plain scan is safe).
fn strip_inline_comment(line: &str) -> String {
    match line.find(" #") {
        Some(pos) => line[..pos].trim_end().to_string(),
        None => line.to_string(),
    }
}

/// Line-filter a front-matter block to the kept keys (and their indented continuation
/// lines), stripping inline comments.
fn filter_front_matter(fm: &str) -> String {
    let mut out = String::new();
    let mut keeping = false;
    for raw in fm.lines() {
        let line = raw.trim_end_matches(['\r']);
        let indented = line.starts_with([' ', '\t']);
        let is_top_key = !indented && !line.trim_start().starts_with('-') && line.contains(':');
        if is_top_key {
            let key = line[..line.find(':').unwrap()].trim();
            keeping = FM_KEEP_KEYS.contains(&key);
            if keeping {
                out.push_str(&strip_inline_comment(line));
                out.push('\n');
            }
        } else if line.trim().is_empty() {
            keeping = false;
        } else if keeping {
            out.push_str(&strip_inline_comment(line));
            out.push('\n');
        }
    }
    out
}

/// Split a markdown body into `##` sections, fence-aware: a `## ` line inside a
/// ```` ``` ````/`~~~` fenced block is treated as content, never a header. Content
/// before the first `##` header (e.g. the `#` title) is dropped.
fn split_sections(body: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut cur: Option<Section> = None;
    let mut in_fence = false;
    for raw in body.split_inclusive('\n') {
        let line = raw.trim_end_matches(['\n', '\r']);
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        } else if !in_fence && line.starts_with("## ") {
            if let Some(sec) = cur.take() {
                sections.push(sec);
            }
            cur = Some(Section {
                key: squish(&line[3..]),
                text: String::new(),
            });
        }
        if let Some(sec) = cur.as_mut() {
            sec.text.push_str(raw);
        }
    }
    if let Some(sec) = cur.take() {
        sections.push(sec);
    }
    sections
}

/// Select the design content of one source file: the allowlisted `##` sections joined
/// in document order, or — if none match — the whole body (minus front-matter) as a
/// fallback. Returns an empty string only for an empty/missing file.
fn kept_content(file_text: &str, allow: &[&str]) -> String {
    if file_text.trim().is_empty() {
        return String::new();
    }
    let (_, body) = split_front_matter(file_text);
    let sections = split_sections(&body);
    let matched: Vec<&Section> = sections
        .iter()
        .filter(|s| allow.contains(&s.key.as_str()))
        .collect();
    if matched.is_empty() {
        body.trim().to_string()
    } else {
        matched
            .iter()
            .map(|s| s.text.trim_end())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Assemble one packet's digest from its three source files' raw contents.
fn build_digest(slug: &str, spec: &str, requirements: &str, design: &str) -> String {
    let fm = filter_front_matter(&split_front_matter(spec).0.unwrap_or_default());
    let goal = kept_content(spec, SPEC_ALLOW);
    let problem = kept_content(requirements, REQ_ALLOW);
    let design_kept = kept_content(design, DESIGN_ALLOW);

    let mut out = String::new();
    if !fm.trim().is_empty() {
        out.push_str("---\n");
        out.push_str(&fm);
        out.push_str("---\n\n");
    }
    out.push_str("# ");
    out.push_str(slug);
    out.push('\n');
    for block in [goal, problem, design_kept] {
        let b = block.trim();
        if !b.is_empty() {
            out.push('\n');
            out.push_str(b);
            out.push('\n');
        }
    }
    out
}

/// Extract the front-matter `status:` value, if present.
fn fm_status(spec: &str) -> Option<String> {
    let (fm, _) = split_front_matter(spec);
    fm?.lines().find_map(|l| {
        l.trim_end_matches(['\r'])
            .strip_prefix("status:")
            .map(|v| v.trim().to_string())
    })
}

/// The immediate sub-directories of `_OLD`, sorted for determinism. `.md` digests from
/// a prior run are files, not dirs, so they are naturally skipped.
fn packet_dirs(old: &Path) -> Result<Vec<PathBuf>, String> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(old)
        .map_err(|e| format!("cannot read {}: {e}", disp(old)))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

fn read_opt(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// Repo-style path display: forward slashes, no `\\?\` verbatim prefix.
fn disp(p: &Path) -> String {
    p.display()
        .to_string()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/")
}

pub fn run(ws: &Path, dry_run: bool) -> i32 {
    let old = ws.join(".ralph/specs/_OLD");
    let dirs = match packet_dirs(&old) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("xtask compact-specs: {e}");
            return 2;
        }
    };

    let (mut digested, mut failed, mut superseded) = (0usize, 0usize, 0usize);
    for dir in &dirs {
        let slug = dir.file_name().unwrap().to_string_lossy().to_string();
        let spec = read_opt(&dir.join("packet.spec.md"));
        let requirements = read_opt(&dir.join("requirements.md"));
        let design = read_opt(&dir.join("design.md"));
        if spec.trim().is_empty() && requirements.trim().is_empty() && design.trim().is_empty() {
            eprintln!("  SKIP {slug}: no packet.spec.md / requirements.md / design.md");
            failed += 1;
            continue;
        }
        if fm_status(&spec).as_deref() == Some("superseded") {
            superseded += 1;
        }

        let digest = build_digest(&slug, &spec, &requirements, &design);
        let out_path = old.join(format!("{slug}.md"));
        if let Err(e) = fs::write(&out_path, &digest) {
            eprintln!("  FAIL {slug}: cannot write {}: {e}", disp(&out_path));
            failed += 1;
            continue;
        }

        if dry_run {
            println!("  DRY  {slug} -> {}", disp(&out_path));
        } else if let Err(e) = fs::remove_dir_all(dir) {
            eprintln!(
                "  WARN {slug}: digest written but could not remove {}: {e}",
                disp(dir)
            );
            failed += 1;
            continue;
        } else {
            println!("  OK   {slug}");
        }
        digested += 1;
    }

    println!(
        "compact-specs: {digested} digested, {failed} failed, {superseded} superseded{}",
        if dry_run {
            " (dry-run: no deletions)"
        } else {
            ""
        }
    );
    i32::from(failed > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_keeps_allowlisted_keys_and_strips_comments() {
        let spec = "\
---
status: implemented
packet: 59_foo
task_ids:
  - TASK-194    # a long inline comment
  - TASK-194a   # another
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 59_foo

## Goal
body
";
        let (fm, _) = split_front_matter(spec);
        let fm = filter_front_matter(&fm.unwrap());
        assert!(fm.contains("status: implemented"));
        assert!(fm.contains("packet: 59_foo"));
        assert!(fm.contains("task_ids:"));
        assert!(fm.contains("- TASK-194\n"));
        assert!(fm.contains("- TASK-194a\n"));
        assert!(!fm.contains("long inline comment"));
        assert!(!fm.contains("backlog_source"));
        assert!(!fm.contains("context_cost_estimate"));
    }

    #[test]
    fn no_front_matter_returns_none() {
        let (fm, body) = split_front_matter("# Design: foo\n\n## Architecture Constraints\nx\n");
        assert!(fm.is_none());
        assert!(body.starts_with("# Design: foo"));
    }

    #[test]
    fn fenced_hashes_are_not_headers() {
        let body = "\
## Real One
alpha

```toml
## not a header
key = 1
```

## Real Two
beta
";
        let secs = split_sections(body);
        let keys: Vec<&str> = secs.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, vec!["realone", "realtwo"]);
        assert!(secs[0].text.contains("## not a header"));
    }

    #[test]
    fn header_squish_handles_drift() {
        assert_eq!(squish("Risks & Trade-offs"), "risksandtradeoffs");
        assert_eq!(squish("Data and Contract Notes"), "dataandcontractnotes");
        assert_eq!(squish("Files in Scope (read + edit)"), "filesinscope");
        assert!(DESIGN_ALLOW.contains(&squish("Risks and Tradeoffs").as_str()));
        assert!(!DESIGN_ALLOW.contains(&squish("Files in Scope").as_str()));
    }

    #[test]
    fn kept_content_selects_allowlisted_design_sections() {
        let design = "\
# Design: foo

## Architecture Constraints
ac body

## Expected Sub-Agent Dispatches
dispatch body

## Data and Contract Notes
dcn body
";
        let kept = kept_content(design, DESIGN_ALLOW);
        assert!(kept.contains("## Architecture Constraints"));
        assert!(kept.contains("## Data and Contract Notes"));
        assert!(!kept.contains("Expected Sub-Agent Dispatches"));
        assert!(!kept.contains("dispatch body"));
    }

    #[test]
    fn zero_match_falls_back_to_whole_body() {
        let design = "\
# Packet 76 — Design notes

## 3a — wildcard matcher
alpha

## 1b — pipeline core
beta
";
        let kept = kept_content(design, DESIGN_ALLOW);
        assert!(kept.contains("3a — wildcard matcher"));
        assert!(kept.contains("1b — pipeline core"));
        assert!(kept.contains("alpha"));
    }

    #[test]
    fn design_allowlist_keeps_deviations_drops_open_questions() {
        let design = "\
# Design: foo

## Architecture Constraints
ac

## Open Questions
oq body

## Implementation Deviations (post-implementation, 2026-05-18)
idev body
";
        let kept = kept_content(design, DESIGN_ALLOW);
        assert!(kept.contains("## Architecture Constraints"));
        assert!(kept.contains("## Implementation Deviations"));
        assert!(kept.contains("idev body"));
        assert!(!kept.contains("## Open Questions"));
        assert!(!kept.contains("oq body"));
    }

    #[test]
    fn empty_file_yields_empty_content() {
        assert_eq!(kept_content("", DESIGN_ALLOW), "");
        assert_eq!(kept_content("   \n", SPEC_ALLOW), "");
    }

    #[test]
    fn build_digest_is_design_only() {
        let spec = "\
---
status: implemented
packet: 42_bar
task_ids:
  - TASK-1  # c
backlog_source: docs/07.md
---

# Packet Contract: 42_bar

## Goal
the goal

## Acceptance Criteria
ac | cmd
";
        let req = "\
# Requirements: 42_bar

## Problem Statement
the problem

## In Scope
- x
";
        let design = "\
# Design: 42_bar

## Architecture Constraints
constraint

## Expected Sub-Agent Dispatches
dispatch
";
        let d = build_digest("42_bar", spec, req, design);
        assert!(d.starts_with("---\n"));
        assert!(d.contains("packet: 42_bar"));
        assert!(d.contains("status: implemented"));
        assert!(d.contains("- TASK-1\n"));
        assert!(!d.contains("backlog_source"));
        assert!(d.contains("# 42_bar\n"));
        assert!(d.contains("## Goal"));
        assert!(d.contains("the goal"));
        assert!(d.contains("## Problem Statement"));
        assert!(d.contains("## Architecture Constraints"));
        assert!(!d.contains("## Acceptance Criteria"));
        assert!(!d.contains("## In Scope"));
        assert!(!d.contains("Expected Sub-Agent Dispatches"));
    }

    #[test]
    fn status_extracted_from_front_matter() {
        let spec = "---\nstatus: superseded\npacket: 9_x\n---\n\n## Goal\ny\n";
        assert_eq!(fm_status(spec).as_deref(), Some("superseded"));
        assert_eq!(fm_status("# no front matter\n"), None);
    }
}
