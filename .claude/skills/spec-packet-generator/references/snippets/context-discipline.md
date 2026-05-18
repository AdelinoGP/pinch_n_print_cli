# Snippet: context-discipline

**When to include**: every packet. This is the standard preamble that links a packet to the context-discipline contract shared by `spec-packet-generator`, `swarm`, and `spec-review`.

**Where to include**: as the closing section of `packet.spec.md`, titled `## Context Discipline Note`, with an opening HTML comment `<!-- snippet: context-discipline -->` on the line above the heading so the self-review can grep for it.

**Verbatim text** (copy exactly — paraphrases drift and rot):

```
<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
```

**Do not paraphrase.** If the content does not apply (it always does, for any packet that ships through Ralph), the packet should not exist. There is no "apply selectively" mode for this snippet.
