# 108 — Adjudicate `wipe_tower_speed` → `wipe_tower_max_purge_speed`

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

The P03 authoring session (ticket 10, packet 255 grounding) discovered a host key that appears to already implement an OrcaSlicer config key under a different name:

- Host key `wipe_tower_speed` — declared in `crates/slicer-ir/src/feedrate.rs::FeedrateConfig` (default 90.0, the registration arm `("wipe_tower_speed", |fc| &mut fc.wipe_tower_speed)`), consumed at `ExtrusionRole::WipeTower` in `crates/slicer-gcode/src/emit.rs::resolve_feedrate`, documented in `docs/15_config_keys_reference.md` (host-key table), locked by `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`.
- Canonical key `wipe_tower_max_purge_speed` — `coFloat`, default 90, min 10, no max, consumed in `WipeTower2.cpp` (ctor member `m_max_speed` → `toolchange_Wipe` where feedrate = min(max_purge_speed, wipe speed), and `finish_layer`) and `WipeTower.cpp` ctor. Not in the ticket 99–107 rename set.
- Defaults match (90.0); the consumer decision (per-path feedrate for wipe-tower extrusion) matches. It is the ticket 107 duplicate-spelling class if truly the same decision.

Adjudicate per the ticket 07 standardise-to-Orca-names ruling: **rename** `wipe_tower_speed` → `wipe_tower_max_purge_speed` (declare + migrate host key, manifest/host-keys/docs/lock-test updates, guest rebuild via the rename workstream's established gates), or rule **equivalence incomplete** (e.g. canonical caps against per-role speeds in a way the host arm cannot express) and record the divergence instead.

Authoring-session evidence to re-derive at resolution time (ledger facts): packet 255 `requirements.md` §Per-key parity evidence row for `wipe_tower_max_purge_speed`; `docs/15_config_keys_reference.md` host-key row. Bounds note: Orca's min 10 has no host-side carrier — `FeedrateConfig` fields are unbounded floats; the adjudication must decide whether the rename also adds bounds enforcement or stays default-parity-only like other host keys.

## Answer