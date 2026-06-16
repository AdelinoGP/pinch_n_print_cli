# Task Map — Packet 74

Single backlog task, four implementation concerns, two predecessor packets. This crosswalk exists because the packet continues prior work (packets 70 and 72) and maps one task id onto distinct steps with different authoritative docs.

## TASK-215 crosswalk

| Step | Concern | Authoritative doc | Predecessor lineage |
|------|---------|-------------------|---------------------|
| 1 | Delete orphan `sdk-layer-plan-guest/` | — | — |
| 2 | Relocate `test-guests/` → `crates/slicer-runtime/test-guests/`; remove old root dir; repoint builder, 18 test files (4 path-construction forms — only 13 are the literal `../../test-guests/`), gitignore, docs | `docs/05_module_sdk.md` (build flow) | packet 70 (`cargo xtask build-guests`, TASK-214) |
| 3 | D1: single shared `CARGO_TARGET_DIR` (keep per-guest `[workspace]`) | `docs/05_module_sdk.md` | packet 70 (preserves its builder design) |
| 4 | A: raw guests `inline:` → canonical `path:`; drop obsolete drift sub-test | `docs/03_wit_and_manifest.md` (single-source rule) | packet 72 (TASK-144/145 — closes its surviving exception) |
| 5 | C: extract witness codec; migrate SDK guests + 5 host decoders | `docs/03_wit_and_manifest.md` (boundary types) | — |

## Lineage notes

- **Packet 70 (`70_workspace-aware-guest-builder`, TASK-214):** introduced the validated filesystem-walk guest builder edited in Steps 2–3. This packet preserves its per-guest-`[workspace]` model (D1, not D2). Not superseded.
- **Packet 72 (`72_wit-single-source-unification`, TASK-144/145):** unified host + macro onto canonical WIT but left the four hand-rolled guests inlining the contract. Step 4 closes that exception. Not superseded — extended.

No packet is superseded by Packet 74.
