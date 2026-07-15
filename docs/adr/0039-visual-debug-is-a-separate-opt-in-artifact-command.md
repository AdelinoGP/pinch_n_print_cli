# ADR-0039 — Visual Debug Is a Separate Opt-in Artifact Command

## Status

Accepted.

## Context

`pnp_cli slice --report` is an optional HTML timing and allocator artifact. Its
absence preserves the existing pipeline path, and a failed report directory is
only a warning. Visual debugging has different inputs, can stop at an
intermediate dependency closure, can parse existing G-code without slicing,
and must not leave partial evidence.

## Decision

Expose visual debugging as `pnp_cli visual-debug --request <JSON> --output
<DIR>`, not as a `slice` flag and not as an extension of `--report`. A request
selects layers, taps, visualization types, and bounded raster scale. The
command accepts exactly one source mode: model-backed execution or standalone
G-code. It fails on artifact-write failure and refuses a non-empty output
directory unless `--overwrite` is explicit.

## Consequences

- Ordinary slices retain zero visual-debug capture and rendering overhead.
- The command can execute a minimal dependency closure and avoid G-code output
  when final rendering was not requested.
- Bundle provenance is unambiguous because model and G-code sources cannot be
  mixed in one bundle.
- The visual artifact lifecycle intentionally differs from `--report` while
  remaining composable at the documentation/workflow level.

## Alternatives Considered

- **Add `--visual-debug` to `slice`.** Rejected: it implies a full slice and
  conflates print output with targeted diagnostic work.
- **Extend the HTML report.** Rejected: report instrumentation and geometry
  snapshots have separate cost, retention, and failure contracts.
- **Allow best-effort partial bundles.** Rejected: missing stage evidence can
  cause a false diagnosis.
