---
name: spec-packet-generator
description: Author a Pinch 'n Print spec packet (packet.spec.md + implementation-plan.md) under docs/spec_packets/ from a prompt, file, URL, or plan. Also authors or resumes a multi-packet plan with a Packet Queue.
type: anthropic-skill
version: "2.0"
metadata:
  internal: true
---

# Spec Packet Generator

Author packets; never implement them. A packet is two files: `packet.spec.md` owns the what and why and the acceptance criteria, `implementation-plan.md` owns the ordered steps. Layout, statuses, and the implementation gotchas live in [`docs/spec_packets/README.md`](../../../docs/spec_packets/README.md). Read it first.

Delegate large reads (`OrcaSlicerDocumented/`, long docs or inputs, broad surveys) and noisy cargo output to subagents, and keep only their conclusions.

## Inputs

- `input` (required): prompt text, Markdown path, URL, or a `docs/specs/*-plan.md` plan.
- `packet_prefix`: derived from a plan file name minus `-plan`; otherwise ask.
- `packet_number`: the plan queue row, or the next free `NN` for the prefix (`01` for a new prefix).
- `packet_name`: kebab-case; derived from the scope or queue row slug.

Example: `/spec-packet-generator input:"Make seam placement honour painted blockers" packet_prefix:seam-paint`.

A plan yielding several packets, or a plan with a `## Packet Queue`, follows [`references/batch-protocol.md`](references/batch-protocol.md).

## Workflow

1. **Understand the request.** Read the input. Use `.agents/doc-index.md` to find the docs that govern the change and read the relevant sections. When OrcaSlicer parity applies, locate the canonical functions in an available checkout or `OrcaSlicerDocumented/`. If unavailable, say so; never invent one.
2. **Ground it in the tree.** Treat the input as claims. Confirm every pre-existing symbol, path, WIT/IR name, prerequisite packet, ADR slot, and deviation ID you will cite, and find the test binaries that can drive the behaviour. Redesign around any claim the tree falsifies.
3. **Confirm once.** Present prefix, number, name, goal, and in/out scope, and get the user's confirmation before writing. In batch mode the approved queue is that confirmation. Never overwrite an existing packet directory without approval.
4. **Write the spec.** Fill [`references/templates/packet.spec.md`](references/templates/packet.spec.md). Copy applicable blocks from [`references/snippets.md`](references/snippets.md) with their markers.
5. **Write the plan.** Fill [`references/templates/implementation-plan.md`](references/templates/implementation-plan.md).
6. **Preflight.** Run every item of [`references/preflight-checklist.md`](references/preflight-checklist.md) against the tree and fix each failure. An item you cannot resolve becomes a `[BLOCK]` open question.
7. **Report.** Give the packet path, any open questions and assumptions, and the next command: `/swarm <packet>` (or `/spec-review <packet> --preflight` for a cold re-check first).

## Acceptance criteria

Each criterion is a plain, checkable statement that names the exact field, config key, manifest entry, count, error code, variant, or output fragment, followed by the command or test that proves it where one naturally exists:

```markdown
- **AC-1** `pnp_cli module config-schema` on `classic-perimeters` emits `wall_count` with `type`, `min`, `max`, `default`, `display`, and `group` keys (absent optionals as `null`). `cargo test -p pnp-cli --test <bin> -- <test_name>`
```

"All six fields are present" is the failing version: it names nothing a reader can check.

Together the criteria cover the main success path and the likeliest silent regression. A change to a validator, scheduler rule, contract boundary, enforcement, or failure path gets a negative criterion (`AC-N1`). Commands are targeted (`cargo test -p <crate> --test <bin> -- <name>`); `cargo test --workspace` appears at most once, in Verification, and only when closure genuinely needs it.

Test commands must follow AGENTS.md §Test Discipline, including feature gates and capturing output to `target/test-output.log`.

## Plan depth

A step names the files and the functions or types it changes, the approach where it is not obvious, its verification command, and its done-when. Spell out exact signatures only for new public contracts (WIT, IR, config schema), and leave implementation bodies to the implementer. A step that adds a struct field or bumps a version lists its fallout (README gotcha 6). Order steps so each leaves the tree compiling.

## Packet safety

- Touch another packet only to absorb it: set its `status: superseded` and `superseded_by:`, and record `supersedes:` plus the missed gap in the new packet's Why.
- Every new packet starts `status: draft`.
- No template placeholders, `TBD`, or bracketed prose survive in an emitted packet.
