# ADR-0069 — Scope eligibility is a per-key deny list

Status: **Accepted.** Approved in the config-scope design interview; not yet
implemented.

Which config scopes may state a given key is declared per key, on its schema entry,
as a list of **denied** scopes. A key with no denial is statable at every scope.
Host and module authors declare it the same way, so a module author has the same
control over their own keys that the host has over its.

Permissive-by-default is the deliberate choice. Exposing more settings at more
scopes than OrcaSlicer does is a goal here, and an allow list makes every new
capability opt-in — a module author adding a key would have to remember to grant
what they almost always want. The denials that matter are a small, stable set of
machine- and emitter-level keys (`bed_shape`, the `machine_max_*` family,
`gcode_xy_decimals`, `disable_m73`) where a narrow-scope statement is meaningless.

This replaces two things. `[config.overridable-per-region]` and
`[config.overridable-per-layer]` are removed along with their `ingest_manifest`
requirement: they were an allow list, per module rather than per key, mandatory in
all 24 core manifests, empty in 23 of them, and read by nothing but a parse test.
It also replaces the accidental eligibility rule that `overlay_resolved`'s
hand-written field list imposed, under which 41 of 69 host fields were silently
unstatable below object scope.

## Consequence

The failure mode inverts. Under an allow list an ineligible statement is rejected
loudly; under a deny list, a key that no consumer reads per-region accepts the
statement and quietly has no effect. That is still strictly better than the silent
unreachability it replaces, but it means the denials have to actually be authored —
an empty deny list is not a safe default, it is an unfinished one.
