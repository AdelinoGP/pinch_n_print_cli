//! CLI-local I/O helpers: output-format enum and parent-creating file writer.
//!
//! These were previously hosted in `slicer_runtime::cli` for legacy reasons.
//! Packet 82 moved them into the CLI crate, where they belong, so the runtime
//! library no longer pulls `clap` into non-CLI consumers.

use std::path::Path;

use clap::ValueEnum;

/// Output mesh formats accepted by the `repair`, `decimate`, `import`, and
/// `convert` subcommands.
#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
#[value(rename_all = "lower")]
pub enum OutputFormat {
    /// Binary STL.
    Stl,
    /// Wavefront OBJ.
    Obj,
    /// 3MF (3D Manufacturing Format).
    #[value(name = "3mf")]
    ThreeMf,
}

/// Write `contents` to `path`, creating any missing parent directories first.
///
/// Centralises the "create parent dir, then write" pattern used by the CLI for
/// both `--output` G-code and `--report` HTML writes so each call site reports
/// directory-creation failures distinctly from file-write failures.
pub fn write_with_parents(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, contents)
}
