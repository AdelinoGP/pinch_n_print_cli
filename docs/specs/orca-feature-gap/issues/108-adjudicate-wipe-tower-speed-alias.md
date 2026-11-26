# 108 — Adjudicate `wipe_tower_speed` → `wipe_tower_max_purge_speed`

Type: task
Status: resolved
Assignee: wayfinder session (ses_f9c3c79edffeAN1HxxYcZyFazz) — claimed 2026-09-02
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

Resolved 2026-09-02. The human Q6(a) ruling is implemented:

- Renamed `FeedrateConfig::wipe_tower_speed` to
  `wipe_tower_max_purge_speed` and updated the `SPEED_KEYS` registration,
  raw-config lookup, host-key mirror, generated config reference, and lock
  tests. This is a host key, so no core-module manifest table is involved.
- `DefaultGCodeEmitter::resolve_feedrate` now uses
  `min(wipe_tower_max_purge_speed, sparse_infill_speed)` for the
  `ExtrusionRole::WipeTower` base feedrate, preserving the existing speed-factor
  and mm/min conversion after the cap. This is the port's single-role form of
  canonical `WipeTower2::toolchange_Wipe` / `finish_layer` purge-grid capping.
- The canonical minimum of 10 mm/s is deferred to ticket 113's feedrate range
  validation; this ticket does not add one-key-only validation to the otherwise
  unbounded `FeedrateConfig`.
- The old `wipe_tower_speed` spelling is intentionally not accepted. The
  renamed key remains in `SPEED_KEYS`, so it is the only host spelling exposed
  to config loading and the host schema.

Verification:

- `cargo test -p slicer-ir --test feedrate_default_tdd`
- `cargo test -p slicer-ir --test feedrate_from_raw_config_tdd`
- `cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd`
- `cargo test -p slicer-runtime --test unit host_keys_doc_lock_tdd`
- `cargo xtask build-guests --check` (clean after rebuilding 44 stale guests)
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo xtask gen-config-docs --check`
