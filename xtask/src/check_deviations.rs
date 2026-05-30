//! `cargo xtask check-deviations`
//!
//! `docs/DEVIATION_LOG.md` is the single source of truth for deviation status.
//! This subcommand regenerates the "Open Deviation Map" snapshot embedded in
//! `docs/07_implementation_status.md` (between the `open-deviations` markers) from
//! the log's `Status` column. A deviation is **open** unless its `Status` cell
//! begins with `Closed` (case-insensitive).
//!
//! - default: rewrite the generated block in doc 07.
//! - `--check`: exit 1 if the on-disk block differs from the regenerated one
//!   (used in CI so the snapshot can never drift from the log).

use std::fs;
use std::path::Path;

const BEGIN: &str = "<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->";
const END: &str = "<!-- END GENERATED: open-deviations -->";

/// One parsed deviation row.
struct Dev {
    id: String,
    status: String,
    summary: String,
}

/// Parse every `| DEV-… |` table row from the deviation log.
///
/// Returns `Err` with a human-readable message if a row is malformed (wrong
/// column count) — that is exactly the kind of drift this guard must catch (a
/// truncated row previously slipped in as DEV-054).
fn parse_devs(log: &str) -> Result<Vec<Dev>, String> {
    let mut out = Vec::new();
    for (lineno, line) in log.lines().enumerate() {
        if !line.starts_with("| DEV-") {
            continue;
        }
        // `| a | b | c | d | e | f | g | h |` -> 10 segments (leading + trailing empty).
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 10 {
            return Err(format!(
                "docs/DEVIATION_LOG.md:{}: malformed deviation row \
                 (expected 8 columns / 10 pipe-segments, found {}): {}",
                lineno + 1,
                parts.len(),
                parts[1].trim()
            ));
        }
        let id = parts[1].trim().to_string();
        let status = parts[8].trim().to_string();
        let rationale = parts[5].trim();
        out.push(Dev {
            id,
            summary: summarize(rationale),
            status,
        });
    }
    Ok(out)
}

/// A deviation is open unless its status begins with "closed".
fn is_open(status: &str) -> bool {
    !status
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("closed")
}

/// Best-effort one-line summary: the first sentence of the rationale with
/// markdown emphasis markers stripped, capped at 160 chars. Deterministic so
/// `--check` is stable.
fn summarize(rationale: &str) -> String {
    let plain = rationale.replace("**", "").replace('`', "");
    let end = plain.find(". ").map(|i| i + 1).unwrap_or(plain.len());
    let mut s = plain[..end].trim().to_string();
    if s.chars().count() > 160 {
        s = s
            .chars()
            .take(159)
            .collect::<String>()
            .trim_end()
            .to_string();
        s.push('…');
    }
    s
}

/// Render the generated block body (the bullet lines between the markers).
fn render_open(devs: &[Dev]) -> String {
    let mut body = String::new();
    let open: Vec<&Dev> = devs.iter().filter(|d| is_open(&d.status)).collect();
    if open.is_empty() {
        body.push_str("_No open deviations._\n");
        return body;
    }
    for d in open {
        body.push_str(&format!("- **{}** ({}) — {}\n", d.id, d.status, d.summary));
    }
    body
}

/// Splice a freshly rendered block into `content`, returning the new document.
fn splice(content: &str, body: &str) -> Result<String, String> {
    let begin = content
        .find(BEGIN)
        .ok_or_else(|| format!("missing `{BEGIN}` marker in doc 07"))?;
    let end = content
        .find(END)
        .ok_or_else(|| format!("missing `{END}` marker in doc 07"))?;
    if end < begin {
        return Err("open-deviations END marker precedes BEGIN marker in doc 07".to_string());
    }
    let before = &content[..begin];
    let after = &content[end + END.len()..];
    Ok(format!("{before}{BEGIN}\n{body}{END}{after}"))
}

/// Entry point. `check_only = true` => verify; otherwise rewrite.
pub fn run(ws: &Path, check_only: bool) -> i32 {
    let log_path = ws.join("docs/DEVIATION_LOG.md");
    let status_path = ws.join("docs/07_implementation_status.md");

    let log = match fs::read_to_string(&log_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "xtask check-deviations: cannot read {}: {e}",
                log_path.display()
            );
            return 2;
        }
    };
    let status_doc = match fs::read_to_string(&status_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "xtask check-deviations: cannot read {}: {e}",
                status_path.display()
            );
            return 2;
        }
    };

    let devs = match parse_devs(&log) {
        Ok(d) => d,
        Err(msg) => {
            eprintln!("xtask check-deviations: {msg}");
            return 2;
        }
    };

    let body = render_open(&devs);
    let updated = match splice(&status_doc, &body) {
        Ok(u) => u,
        Err(msg) => {
            eprintln!("xtask check-deviations: {msg}");
            return 2;
        }
    };

    let open_count = devs.iter().filter(|d| is_open(&d.status)).count();

    if check_only {
        if updated == status_doc {
            println!("OK: doc 07 Open Deviation Map matches DEVIATION_LOG.md ({open_count} open).");
            0
        } else {
            eprintln!(
                "::error::docs/07_implementation_status.md Open Deviation Map is out of sync with \
                 docs/DEVIATION_LOG.md. Run `cargo xtask check-deviations` to regenerate."
            );
            1
        }
    } else {
        if updated == status_doc {
            println!("Open Deviation Map already current ({open_count} open).");
            return 0;
        }
        if let Err(e) = fs::write(&status_path, updated) {
            eprintln!(
                "xtask check-deviations: cannot write {}: {e}",
                status_path.display()
            );
            return 2;
        }
        println!("Regenerated Open Deviation Map in doc 07 ({open_count} open).");
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_when_status_not_closed() {
        assert!(is_open("Open"));
        assert!(is_open("Partial — foo"));
        assert!(is_open("In Progress"));
        assert!(!is_open("Closed"));
        assert!(!is_open("Closed — Packet 58, 2026-05-17"));
        assert!(!is_open("  closed 2026 "));
    }

    #[test]
    fn parse_extracts_id_status_summary() {
        let log = "\
| ID | Date | Affected | Risk | Rationale | Owner | Target | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DEV-009 | 2026-04-15 | x | High | **Benchy gap.** more text | owner | TBD | Open |
| DEV-014 | 2026-04-16 | y | Med | plain rationale here | owner | 2026 | Closed 2026-04-24 |
";
        let devs = parse_devs(log).unwrap();
        assert_eq!(devs.len(), 2);
        assert_eq!(devs[0].id, "DEV-009");
        assert_eq!(devs[0].status, "Open");
        assert_eq!(devs[0].summary, "Benchy gap.");
        assert!(is_open(&devs[0].status));
        assert!(!is_open(&devs[1].status));
    }

    #[test]
    fn malformed_row_is_rejected() {
        let log = "| DEV-054 | 2026 | truncated row with no closing columns";
        assert!(parse_devs(log).is_err());
    }

    #[test]
    fn splice_replaces_between_markers() {
        let doc = format!("intro\n\n{BEGIN}\nstale\n{END}\n\noutro\n");
        let out = splice(&doc, "- **DEV-009** (Open) — gap\n").unwrap();
        assert!(out.contains(&format!("{BEGIN}\n- **DEV-009** (Open) — gap\n{END}")));
        assert!(out.starts_with("intro"));
        assert!(out.trim_end().ends_with("outro"));
        assert!(!out.contains("stale"));
    }
}
