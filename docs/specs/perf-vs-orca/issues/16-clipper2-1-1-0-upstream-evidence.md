# clipper2 1.1.0 upstream evidence

Type: research
Status: resolved

## Question

What changed in the `clipper2-rust` crate between 1.0.3 and 1.1.0 that could
affect:

a. polygon-op **cost** at `union` / `difference` / `intersection` / `offset`
   call sites (e.g. `polygon_ops` in `crates/slicer-core/src/polygon_ops.rs`),
b. polygon **output** for identical inputs (the bump landed with no parity
   comparison — the clipper defect row in `docs/DEVIATION_LOG.md` is DEV-173,
   not the handoff's DEV-166), and
c. the **unbounded recursion in `check_split_owner`** (DEV-173: hard process
   abort, no diagnostic, no regression coverage) — did 1.1.0 change its reach
   or trigger conditions?

Sources: the two crate sources in the cargo registry cache (1.0.3 vs 1.1.0,
diff the relevant modules) and upstream history (the repository URL recorded in
the crate's own `Cargo.toml` — do not guess URLs). Findings feed
[clipper2 cost/output verdict](17-clipper2-cost-output-verdict.md), which does
the measured A/B.

AFK research ticket: resolve via the `research` skill; capture findings as a
Markdown file in the repo on throwaway branch `research/clipper2-1-1-0-delta`,
then record the branch + file path as this ticket's context pointer and set
`Status: resolved`.

## Answer

- (a) polygon-op cost: **unchanged** — 1.1.0's only code delta is an additive `PolyFace64` / `poly_tree_to_faces64` API in the crate's `clipper.rs`; every boolean-op/offset module is byte-identical (`engine.rs`) or line-ending-only (`offset.rs`, `engine_fns.rs`, `core.rs`), so `offset2_ex` cost cannot have moved with the bump.
- (b) polygon output: **unchanged** — identical geometry code in both versions and the new read-side API is unused in this repo, so identical inputs yield identical polygons; the missing parity comparison is a process gap with no behavioral risk (ticket 17's measured A/B is the empirical confirmation).
- (c) `check_split_owner` unbounded recursion: **unchanged** — it lives in `engine.rs`, byte-identical across the bump, so the pts-less unguarded recursion and `is_valid_owner` demotion keep identical reach and trigger conditions (this defect is **DEV-173** in `docs/DEVIATION_LOG.md`; the handoff's "DEV-166"
   label for it is stale — this ticket now uses the ledger id).
- Caveat: upstream has no v1.0.3 tag (1.0.3 pinned via its packaged `.cargo_vcs_info.json` SHA `11a62c6…`), and upstream has since tagged v1.2.0 (2026-09-18) — outside this window.

Findings: branch `research/clipper2-1-1-0-delta`, file `docs/research/clipper2-1-1-0-delta.md`.
