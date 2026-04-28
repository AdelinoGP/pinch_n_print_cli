# Design: [spec-slug]

## Controlling Code Paths

- Primary code path:
- Neighboring tests or fixtures:
- OrcaSlicer comparison surface:

## Architecture Constraints

-

## Code Change Surface

- Selected approach:
- Exact functions, traits, manifests, tests, or fixtures expected to change:
- Rejected alternatives that were considered and why they were not chosen:

## Files in Scope (read + edit)

List the files the implementer is expected to read and edit. Target ≤ 3 primary files. If more than 3 are unavoidable, justify each one — and consider splitting the packet.

- `[path/to/file.rs]` — role: [why this file]; expected change: [one line]
- `[path/to/test.rs]` — role: ...; expected change: ...

## Read-Only Context

Files the implementer is allowed to read but not edit. Include line-range hints whenever the file is > 300 lines. The implementer should range-read these, not load them in full.

- `[path/to/large_doc.md]` — read lines `[N-M]` only — purpose: [what fact is being verified]
- `[path/to/trait_definition.rs]` — read the trait def and its docs only — purpose: [why]

## Out-of-Bounds Files

Files the implementer must NOT load directly. The implementer should delegate any fact-checks against this list.

- `OrcaSlicerDocumented/...` — delegate parity checks; never load
- `target/`, `Cargo.lock`, generated code — never load
- Vendored deps under `vendor/` or equivalent — never load
- Crates outside the change surface — delegate trait/impl lookups; do not browse

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make. This list is not exhaustive but should cover the predictable ones.

- "Run `<verification-command>`; return FACT (pass) or SNIPPETS (fail with assertion + ≤ 20 lines)" — purpose: validate Step N
- "Find all callers of `<symbol>`; return LOCATIONS" — purpose: confirm no orphan call sites
- "Summarize `<authoritative-doc>` for the constraint about `<topic>`; return FACT" — purpose: confirm Step N's contract

## Data and Contract Notes

- IR or manifest contracts touched:
- WIT boundary considerations:
- Determinism or scheduler constraints:

## Locked Assumptions and Invariants

-

## Risks and Tradeoffs

-

## Context Cost Estimate

- Aggregate (sum across all steps): `S | M` (never L)
- Largest single step: `S | M`
- Highest-risk dispatch (the one whose return could blow budget if mis-shaped): [describe; specify required return format]

## Open Questions

- Resolve any ambiguity here before the packet becomes `active`.
- If an open question would change scope, interfaces, or verification strategy, the packet must remain `draft` until it is answered.
- If an open question requires reading an out-of-bounds file to answer, escalate to a delegation plan rather than admitting the file into scope.
