//! `diagnose` subcommand implementation: load manifests and surface every
//! `LoadDiagnostic` as structured JSON on stdout. Returns the process exit code.

use crate::{assemble_search_roots, DiagnosticLevel, ModuleProvenance};

/// Run the `diagnose` command. Returns the process exit code:
/// - `0` — all modules loaded with no errors (warnings allowed).
/// - `1` — at least one `Error`-level diagnostic.
/// - `2` — module loader itself failed.
pub fn run_diagnose(
    module_dir: &[std::path::PathBuf],
    no_default_module_paths: bool,
    no_integrated_modules: bool,
) -> i32 {
    let search_roots = assemble_search_roots(module_dir, no_default_module_paths);
    let integrated = if no_integrated_modules {
        Vec::new()
    } else {
        slicer_integrated_modules::integrated_registrations()
    };
    let report =
        match slicer_scheduler::load_modules_from_roots_with_integrated(&search_roots, &integrated)
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error loading modules: {e:?}");
                return 2;
            }
        };

    #[derive(serde::Serialize)]
    struct DiagnoseOut<'a> {
        pass: bool,
        modules_loaded: usize,
        stages: usize,
        modules: Vec<DiagnoseModuleOut>,
        diagnostics: Vec<DiagnosticOut<'a>>,
    }

    #[derive(serde::Serialize)]
    struct DiagnoseModuleOut {
        id: String,
        provenance: &'static str,
    }

    #[derive(serde::Serialize)]
    struct DiagnosticOut<'a> {
        level: &'a str,
        file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        field: &'a Option<String>,
        message: &'a str,
    }

    let mut stage_set: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for m in &report.modules {
        stage_set.insert(m.stage());
    }

    let diagnostics: Vec<DiagnosticOut> = report
        .diagnostics
        .iter()
        .map(|d| DiagnosticOut {
            level: match d.level {
                DiagnosticLevel::Error => "error",
                DiagnosticLevel::Warning => "warning",
                DiagnosticLevel::Info => "info",
            },
            file: d.path.display().to_string(),
            field: &d.field,
            message: d.message.as_str(),
        })
        .collect();

    let has_error = report
        .diagnostics
        .iter()
        .any(|d| matches!(d.level, DiagnosticLevel::Error));

    let modules = report
        .modules
        .iter()
        .map(|m| DiagnoseModuleOut {
            id: m.id().to_string(),
            provenance: match m.provenance() {
                ModuleProvenance::Integrated => "integrated",
                ModuleProvenance::External => "external",
            },
        })
        .collect();

    let out = DiagnoseOut {
        pass: !has_error,
        modules_loaded: report.modules.len(),
        stages: stage_set.len(),
        modules,
        diagnostics,
    };
    print_json(&out);
    if has_error {
        1
    } else {
        0
    }
}

fn print_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("error: failed to serialize output: {e}");
            std::process::exit(1);
        }
    }
}
