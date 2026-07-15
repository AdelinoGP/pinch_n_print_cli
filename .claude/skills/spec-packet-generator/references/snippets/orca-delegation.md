---
when: Include when a packet consults `OrcaSlicerDocumented/` for parity.
keywords: OrcaSlicer, delegation, packet.spec.md, requirements.md
---

# OrcaSlicer Delegation Snippet

Copy the block exactly into `packet.spec.md` and `requirements.md`, then replace the path bullet. Skip it for work with no OrcaSlicer behavior. Never add a third copy to `design.md`.

```markdown
<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/<path>` — <one-line statement of what behavior, constant, or edge case is being borrowed (or deliberately not borrowed)>
```

Do not paraphrase the opening paragraph; only the file bullets are packet-specific.
