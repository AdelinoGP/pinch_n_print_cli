# 84 — Author packet P77 — Quality / Bridging — classic-perimeters

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260910_P77) — claimed 2026-09-10, resolved 2026-09-10
Blocked by: 06, 102, 107
Map: ../map.md

## Question

Author the spec packet for **P77 — Quality / Bridging — classic-perimeters** — 2 keys, Tier B new logic, owner classic-perimeters. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P77 — Quality / Bridging — classic-perimeters):

`bridge_angle`, `counterbore_hole_bridging`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.
- Owner spans two sibling modules (top-surface-ironing + support-surface-ironing / classic-perimeters + arachne-perimeters) — the packet touches both manifests.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier B held, owner confirmed but narrowed to the prepass seam, packet authored, no re-file.** Both keys live in canonical's slicing pipeline (`PrintConfig.cpp` coFloat `bridge_angle` default `0` min 0 max 180 + coEnum `counterbore_hole_bridging` default `chbNone` spellings `none`/`partiallybridge`/`sacrificiallayer`; `PrintConfig.hpp` `CounterboreHoleBridgingOption`; readers in `LayerRegion.cpp::process_external_surfaces` top + bottom custom-angle arms, `PerimeterGenerator.cpp::process_no_bridge` island separation + `BridgeDetector` coverage + filled-vs-partial handling, `Layer.cpp::make_perimeters` chbFilled extra-fill recovery, `PrintObject.cpp::detect_surfaces_type` chbFilled slice-union — all verified against the oracle at claim time) and are zero-occurrence as behaviour in this tree (verified by tree-wide `rg` over `crates/` + `modules/` + `xtask/` + `resources/` — the `bridge_angle` substring hits are all `internal_bridge_angle`; zero `counterbore` hits anywhere in code; the one `ORCA_CONFIG_REFERENCE.md` row pair is docs-only). Ticket 04's "classic-perimeters + arachne-perimeters" owner is accurate but is a seam, not a module: both decision points land in the host prepass `commit_shell_classification_builtin` (the ticket-36 precedent) — the perimeter modules inherit them through the buckets they already consume, never module-side.

Packet `docs/spec_packets/302-bridge-angle-counterbore-classic-perimeters/` authored (`draft`), **preflight PASS** (S0–S8, all tree-verified: 7/7 pre-existing symbols resolved with file evidence; DEV-193 verified next-free — LOG max DEV-196, drafts claim DEV-172–DEV-192; no schema bump — no IR/WIT change; ADR-0061/0062/0063 conformance, not amendment; `--test executor` wiring confirmed via `tests/executor/main.rs` mod-list; AC test homes match file locations; two self-corrections — the `_OLD/` 233–235 related-work lines corrected to name the archive dir + statuses, and the S7 Cargo-mapping claim corrected to the autotests convention after the explicit-`[[test]]` grep came back empty). Membership 2/2, none shed, none returned: `bridge_angle > 0` overwrites the detected external orientation verbatim (the live `internal_bridge_angle` arm's exact semantics; `0` = automatic = pre-packet shape) and the counterbore stage authors hole-bearing unsupported spans into `bridge_areas` (whole spans in `filled`, rims only in `partial`, `none` = pre-packet shape); defaults ARE identity on all prints (AC-5 pins both keys unset vs at-defaults byte-identical). Canonical scalarity IS held (per-region scalars — no ticket-125 vector arm); no numeric range rejection (ticket-113 rule; unknown enum spelling is the `flat_bridge_closing_join` fallback); locked-path conformance pinned (AC-N2); neither key has a padding twin (honest absence, spellings ride 132); rule 4 does not fire (in-seam overwrite + bucket authoring, `seam_position` precedent). Four divergences in new **DEV-193**: the `relative_bridge_angle` companion is not borrowed (absent from source, queue, and tree), the `align_infill_direction_to_model` rotation offset is not borrowed (draft packet 262a's scope), the chbFilled slice-union support-map restructure is not borrowed, and `Print.cpp` reslice-invalidation rides ticket 124. No code change; no queue-count change; no new fog, nothing out of scope.
