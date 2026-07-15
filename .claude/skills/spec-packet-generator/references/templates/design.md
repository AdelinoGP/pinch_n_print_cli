# Design: [spec-slug]

## Controlling Code Paths

- Primary code path:
- Neighboring tests/fixtures:
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- [Packet-specific constraint]
- [Exact `wasm-staleness` snippet bullet when applicable]
- [Exact `coord-system` snippet bullet when applicable]

## Code Change Surface

- Selected approach:
- Exact functions, traits, manifests, tests, and fixtures:
- Rejected alternatives and reasons:

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `[path]` - role: [why]; expected change: [one line]

## Read-Only Context

Include ranges for files over 300 lines.

- `[path]` - lines `[N-M]` only - purpose: [fact]

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: [precise question]; scope: `[path/glob]`; return: `FACT | LOCATIONS | SNIPPETS | SUMMARY`; purpose: [step]

## Data and Contract Notes

- IR/manifest contracts:
- WIT boundary:
- Determinism/scheduler constraints:

## Locked Assumptions and Invariants

[State all locks. If none: `None - change is reversible via existing config defaults; no behavior locks introduced.`]

## Risks and Tradeoffs

- [Risk/tradeoff]

## Context Cost Estimate

- Aggregate: `S | M` (never L)
- Largest step: `S | M`
- Highest-risk dispatch and required return format:

## Open Questions

Tag implementer-resolvable questions `[FWD]`; tag activation blockers `[BLOCK]`. Scope/interface/verification questions keep the packet `draft`. Delegate answers requiring out-of-bounds reads. Write `None.` when absent.
