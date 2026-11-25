---
packet: [packet-dir]
status: draft
---

# [Packet title]

## Goal

[One sentence describing the end state.]

## Why

[The gap and why it is one coherent slice, in one to three short paragraphs. When superseding a packet, name it and what it missed.]

## Scope

- In: [item]
- Out: [item a reader might reasonably assume is included]

## Depends on

- `[packet-dir]` ([current status]): [what this packet consumes and whether implementation must wait]. Or `None.`

## Design notes

[The chosen approach and any rejected alternative worth recording; non-obvious decisions; invariants and locked assumptions; IR / WIT / manifest / scheduler contract notes. Applicable snippets from `references/snippets.md` go here. Keep only what a fresh implementer would otherwise get wrong.]

## Acceptance criteria

- **AC-1** [Checkable statement naming exact fields, keys, errors, or outputs.] `[command or test]`
- **AC-N1** [Rejection case, for validator, enforcement, contract-boundary, or failure-path changes.] `[command or test]`

## Verification

- `cargo build --workspace`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `[targeted test command proving the main contract]`

## Docs impact

[Either `none` with a one-line reason (not allowed when the change touches IR, WIT, scheduler, claim, manifest, host-service, or SDK contracts), or one bullet per doc section to edit with its check: `docs/02_ir_schemas.md` § <section> `rg -q '<anchor>' docs/02_ir_schemas.md`]

## Open questions

[`[BLOCK]` must be answered before implementing; `[FWD]` the implementer may resolve. Or `None.`]
