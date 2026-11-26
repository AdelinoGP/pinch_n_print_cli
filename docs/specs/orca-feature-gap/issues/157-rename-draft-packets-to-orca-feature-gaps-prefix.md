# 157 — Rename the 52 draft packets to `orca-feature-gaps_01`–`52` and record legacy provenance

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

All 52 unimplemented draft packets under `docs/spec_packets/` (`253-*` through `307-*`) use the retired bare-number directory convention, and three of them (`253-part-cooling-fan-scale-and-cooling-keys`, `254a-prime-tower-geometry-keys`, `254b-prime-tower-interface-and-ramming`) **collide numerically** with already-archived master packets in `docs/spec_packets/_OLD/` (`253-build-guests-incremental-and-shared-target.md`, `254-exact-perimeter-spatial-queries.md`).

Execute a **pure structural rename and provenance migration** (no packet content rewrites yet — those run in Phase 2 after triage and the final master merge) to bring directory naming into compliance with `docs/spec_packets/README.md` (`<prefix>_<NN>_<packet-name>/`) using the user-selected prefix **`orca-feature-gaps`**:

1. **Derive the exact set of branch draft packets from disk** (`git diff --name-only --diff-filter=A origin/master..HEAD -- docs/spec_packets/` or listing `docs/spec_packets/[23][0-9][0-9]*`). Confirm the count at claim time (52 at filing, 2026-11-23).
2. **Assign sequential numbers `01` through `<N>` in clean dependency / domain-execution order:** `docs/spec_packets/orca-feature-gaps_<NN>_<slug>/` (matching `proptest-openrscad_01_<slug>/` and `config-scope-resolution_01_<slug>/`, eliminating legacy gaps like missing `261` / `268`–`274` and `a`/`b` suffixes). Verify prerequisite ordering (e.g., prime-tower stack `254a → 254b → 255 → 307`, fuzzy-skin `259a → 259b`, support-interface `260a → 260b`, infill `262a → 262b`, slice-mutating passes).
3. **`git mv` every draft packet directory** to its new `docs/spec_packets/orca-feature-gaps_<NN>_<slug>/` path.
4. **Stamp provenance in front matter:** in each moved `packet.spec.md`, update `packet:` / id metadata to the new directory name and add `legacy_id:` recording both the historical directory id (e.g. `254a-prime-tower-geometry-keys`) and queue slot (`P02`), without modifying the body or converting 5-file → 2-file yet (Phase 2 owns the 2-file SOTA rewrite).
5. **Publish a provenance mapping table** (either in this ticket's `## Answer` or a linked `./157-asset-packet-provenance-map.md`) mapping `old dir ↔ queue slot ↔ new dir`, and update active cross-references in:
   - `docs/specs/orca-feature-gap/map.md` (including superseding ticket `06`'s bare-number directory allocation rule with the `orca-feature-gaps_<NN>_<slug>/` rule)
   - Open tickets under `docs/specs/orca-feature-gap/issues/` that cite any of the 52 packet directories
   - Cross-packet `prerequisites:` / `depends_on:` references inside the 52 moved packets
6. **Verify zero broken in-repo links** to the old `docs/spec_packets/253-`..`307-` paths in active (non-historical-record) references, and confirm `cargo xtask` / `docs/spec_packets/README.md` conventions are satisfied. Note that once assigned, numbers are stable: any packet dissolved in Phase 2 becomes `status: superseded` or archived without renumbering siblings.

### Obligations

- **Pure move + provenance only:** do not rewrite packet designs, ACs, or 5-file → 2-file layout here; keep the diff mechanically reviewable so [158 — Triage all 52 draft packets for architectural drift, SOTA quality gaps, and Phase 2 batching](./158-triage-draft-packets-for-drift-and-phase2-batching.md) can cite the clean new directory names.
- Re-derive every packet directory and reference from disk at claim time.

## Answer
