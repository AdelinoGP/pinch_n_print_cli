# Snippet: orca-delegation

**When to include**: any packet that needs to consult `OrcaSlicerDocumented/` for parity (constants, defaults, algorithm shape, edge cases). Skip entirely for packets that touch no OrcaSlicer behavior (e.g., pure host scheduler refactor, internal IR re-export cleanup).

**Where to include**: as the `OrcaSlicer Reference Obligations` section heading + opening paragraph in `packet.spec.md` AND `requirements.md` (both files genuinely need it — `packet.spec.md` for preflight visibility, `requirements.md` for the implementer's authoritative reference list). Add `<!-- snippet: orca-delegation -->` on the line above each occurrence.

**Verbatim opening paragraph** (then list the specific `OrcaSlicerDocumented/` files inline below):

```
<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/<path>` — <one-line statement of what behavior, constant, or edge case is being borrowed (or deliberately not borrowed)>
```

**Do not paraphrase** the opening paragraph. The file-list bullets below it are packet-specific and must be filled in by the generator.

**Anti-pattern to avoid**: a third occurrence of this paragraph in `design.md`. `design.md` may *reference* the OrcaSlicer parity surface in its "Controlling Code Paths" section ("OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations"), but must not restate the delegation rules.
