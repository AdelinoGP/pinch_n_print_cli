# ADR-0068 — Config scope is a wire encoding, not an internal representation

Status: **Accepted.** Approved in the config-scope design interview; not yet
implemented.

> **Amendment 2026-09-11 (config-scope resolution revision session).** The
> "automatic values expand inside the registry" consequence in ADR-0067
> conflated declaration with evaluation. Expansion is now three phases:
>
> - **Phase A — declaration (registry):** type, default, bounds, base-key,
>   selector, eligibility, provenance. No expansion.
> - **Phase B — resolution (config-only):** after applicable scope deltas
>   merge, before interning / `ConfigView` delivery, using an explicit
>   `ExpansionContext` (nozzle diameter, tool bases). Covers width/percent
>   auto-sentinels and config-only `-1 = auto` rules whose inputs are config.
> - **Phase C — owning stage:** rules needing non-config context expand where
>   that context exists — role/first-layer/bridge widths in
>   `slicer-core::flow::resolve_role_width` (reading already-expanded bases),
>   and the volumetric `0 = auto` speed cap in the G-code emitter, where the
>   per-move width/height it needs exist.
>
> "No consuming module ever reads a placeholder" is retained with the precise
> reading: **no raw placeholder reaches a `ConfigView` consumer at its read
> point.** A stage-context rule is documented as such; it is never expanded
> with context frozen at load. The registry's per-run invariants (single
> default per key, placeholder never escapes resolution) are unchanged — the
> invariant's enforcement point is Phase B/C, not the registry struct.

Scope was carried inside the config key text — `object_config:<id>:<key>`,
`paint_config:<semantic>:<key>`, `tool_config:<idx>:<key>`, `object_height:<id>` —
re-parsed by `starts_with` at six points in `resolve_global_config` and its
siblings, and rebuilt by `format!` at four, one of them inside the
`layer-planner-default` guest. Nothing typed the scope and nothing ordered it,
which is how `run_slice_with_collector` and `prepare_prepass_context` came to run
different resolution chains with only a prose comment recording the difference.

We keep the prefixed flat key as the **wire** format and decode it exactly once, at
ingestion, into a typed scope. Nothing downstream sees a prefix again. Precedence
becomes a property of the scope type rather than of the order a caller happens to
invoke resolvers in.

The wire format stays flat deliberately. A 3MF's `project_settings.config` is a flat
OrcaSlicer-shaped JSON object, and keeping PNP's scoped keys in the same shape means
Orca-authored global keys and PNP scope data share one document. Restructuring the
sidecar into nested per-scope objects would be cleaner in isolation and was rejected
for that compatibility.

## Scope layers are deltas

A scope contributes only the settings actually stated at it — never a full
configuration with unstated fields filled in from `Default`. This replaces
`overlay_resolved`'s merge rule in `crates/slicer-core/src/algos/region_mapping.rs`,
which decided "was this overridden?" by comparing each field against
`ResolvedConfig::default()`. That rule had two defects a delta removes by
construction: a field explicitly set to its own default was indistinguishable from
one never set, and the merge could only carry the 28 fields someone had
hand-enumerated, leaving the other 41 unreachable from any scope below object level.
