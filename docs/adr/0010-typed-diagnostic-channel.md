# ADR-0010 — Typed `Diagnostic` Channel on `world-prepass`

## Status

Proposed (lands with B7 / `TASK-163b-diagnostic` from `docs/specs/support-modules-orca-port.md`).

## Context

`support-planner` emits warnings via `host-services.log` with structured
information encoded as string prefixes:

```rust
log(
    LogLevel::Warn,
    &format!(
        "support-planner.node-clamped-out: layer={} obj={} pos=({:.3},{:.3})",
        current_global_layer_index, obj.object_id, cx, cy
    ),
);
```

The original `TASK-163b` (now split — see `docs/specs/support-modules-orca-port.md`
§D12) called out this as needing promotion to a typed channel so downstream
tooling (the slicer report, CI assertions, GUI surfaces) can read diagnostic
information programmatically rather than by parsing prefix strings.

Block B of the support-modules spec adds three call sites that should use the
typed channel:
1. `node-clamped-out` (existing; migrated).
2. `max_branches_per_layer` cap exceeded (B4 / TASK-253).
3. `support_interface_bottom_layers` not implemented (B2 / TASK-251).

A typed channel is the right place to land this for all three call sites at
once, and to give future module-emitted diagnostics a structured home.

## Decision

Add a typed `Diagnostic` record to `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`:

```wit
record diagnostic {
    severity: severity-level,
    code: u32,
    layer: option<s32>,
    object-id: option<string>,
    message: string,
}

enum severity-level {
    trace,
    debug,
    info,
    warn,
    error,
}
```

`Diagnostic` is emitted by guest modules via a new host import (or via the
prepass output-builder, depending on the implementation pattern). The host
collects diagnostics into a per-stage `Vec<Diagnostic>` that becomes part of
the prepass execution audit.

**Field semantics**:

- `severity` — five-level standard hierarchy. Maps cleanly to existing `tracing` / `slog` levels for downstream sinks.
- `code: u32` — a numeric code per diagnostic class. Codes are allocated per module:
  - `support-planner`: 1000-1999.
  - `raft-default`: 2000-2099.
  - Other modules: future ranges.
  - This codifies the per-module range convention so two modules can't collide on the same number.
- `layer: option<s32>` — global layer index when the diagnostic is layer-scoped. `None` for prepass-global diagnostics. `s32` (not `u32`) to allow negative raft layer indices once raft-default is wired.
- `object-id: option<string>` — object identifier when the diagnostic is object-scoped. `None` for object-agnostic diagnostics.
- `message: string` — human-readable description. Includes the parameters that don't fit the fixed fields (e.g., position coordinates, drop counts). Templated with placeholders that downstream tooling can parse if needed.

**Why a record + enum, not a variant**:

`variant diagnostic { node-clamped-out(...), contact-truncated(...), ... }`
was considered. Rejected because:
- Adding a new diagnostic class would require a WIT change (variant arm
  addition), which triggers the full guest-rebuild pipeline. The
  record-with-`code` design lets new diagnostic classes ship as a module
  internal-only change.
- Community modules can emit `Diagnostic` without coordinating with the
  host WIT surface.

## Consequences

**Positive**:
- Three existing string-prefixed log call sites in `support-planner` migrate to typed payloads.
- Downstream tooling (slicer report HTML, CI tests, GUI) reads `Diagnostic.layer`, `Diagnostic.code`, etc. directly. No string parsing.
- Future module-emitted diagnostics have a structured home from day one.
- Code-range allocation prevents per-module collisions.

**Negative**:
- WIT change triggers full guest-rebuild (`cargo xtask build-guests`) across all 20 guests, per the WIT/Type Changes Checklist in `CLAUDE.md`.
- New WIT types (`Diagnostic`, `SeverityLevel`) join the canonical surface and become covered by `wit_drift_detection_tdd`. Tests must be updated to assert the new types are present.
- Modules that want to emit diagnostics must include the `slicer-sdk` helper API; legacy modules using raw `log(...)` are not silently migrated — they're updated explicitly per B7's scope.

**Trade-offs we explicitly accept**:
- A `Diagnostic` is recoverable (it doesn't abort the slice). Errors that should abort use `ModuleError::fatal(...)`. The distinction is documented in `slicer-sdk` and in this ADR.
- The `code` field is module-allocated, not centrally registered. Collisions across modules are prevented by the per-module range convention, not by a central registry. If collisions become a problem in practice, a registry can land later.

## Future-Reviewer Notes

- **Do not migrate ALL existing `log(...)` calls to `Diagnostic`.** The typed channel is for diagnostic events with structured payload. Plain trace/debug logging stays in `host-services.log`.
- **Do not allocate codes outside your module's range.** If a new diagnostic class needs codes outside the existing per-module ranges, add a range allocation comment to this ADR.

## References

- `docs/07_implementation_status.md` — original `TASK-163b` text.
- `docs/specs/support-modules-orca-port.md` §B7, §D11.
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — target WIT file.
- `CLAUDE.md` "WIT/Type Changes Checklist" — rebuild ceremony.
