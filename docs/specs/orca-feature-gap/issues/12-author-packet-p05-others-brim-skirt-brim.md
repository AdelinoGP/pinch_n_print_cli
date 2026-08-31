# 12 — Author packet P05 — Others / Brim — skirt-brim

Type: task
Status: resolved
Assignee: wayfinder session (ses_fab3cf771ffe3zz1XSeAKhFrBJ) — claimed 2026-08-30
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P05 — Others / Brim — skirt-brim** — 6 keys, Tier A plumbing, owner skirt-brim. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P05 — Others / Brim — skirt-brim):

`brim_ears`, `brim_ears_detection_length`, `brim_ears_max_angle`, `brim_object_gap`, `brim_type`, `brim_use_efc_outline`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet `docs/spec_packets/257-brim-type-and-brim-keys/` authored (`draft`), preflight **PASS** (2026-08-30; S0–S8 + AC-command + Doc-Impact checks, zero blockers, zero highs; report persisted at `preflight-report.md` in the packet dir).

Scope ruling (user-confirmed at authoring time): **P05 covers 5 keys, not 6** — queue 407 → 406. Canonical's `brim_ears` **bool** is dead: declared in `PrintConfig.cpp` (coBool, default false) but read by no slicing, GUI, or preset code, and it has **no member in canonical's typed `PrintConfig` struct**; ear physics route entirely through `brim_type` values `brim_ears` (btEar) / `painted` (btPainted). Ruled out per ticket 04's dead-in-canonical class. `brim_ears_max_angle` / `brim_ears_detection_length` stay (live canonical keys, declared-with-gap below).

Grounding findings that shaped the packet:

- **The in-tree owner module implements none of canonical's brim semantics**: `SkirtBrim::generate_brim_entities` emits rectangular loops around the layer-0 bounding box gated on `brim_width > 0` (`modules/core-modules/skirt-brim/src/lib.rs`) — no mode selection, no object-contour offsetting, no ear detection, no EFC coupling. Every one of the 5 keys' canonical decision points was therefore re-derived rather than trusted from the tier table.
- **Exactly one live decision point** (per the key-by-key re-derivation in the packet's `requirements.md` §Per-Key Canonical Evidence): the on/off gate. Packet wires `brim_type = "no_brim"` → no brim entities on both `run_finalization` (live host path) and the legacy `process()` arm, with `brim_width > 0` as the enabling gate — user ruling "wire gate". Default (`auto_brim`/absent) output is identity-pinned (AC-3) and `brim_width = 0` precedence is pinned (AC-N1). Canonical's `Print.hpp::has_brim` interplay named for the future mode-aware packet.
- **Four keys declared-with-gap**, each with its canonical consumer pinned by file+function: `brim_object_gap` → `Brim.cpp::outer_inner_brim_area` (contour offsetting — the bbox-vs-object-contour divergence recorded, bbox-inset hack rejected as invented semantics); `brim_ears_max_angle` + `brim_ears_detection_length` → `Brim.cpp::make_brim_ears_auto` (angle threshold = 180−max_angle; Douglas-Peucker decimation); `brim_use_efc_outline` → `Brim.cpp::use_brim_efc_outline` (requires `elefant_foot_compensation > 0`, layers > 0, `raft_layers == 0` — no EFC geometry exists in this tree; the padding literal is its only occurrence). Declared-with-gap matches packet 253's `dont_slow_down_outer_wall` / 254's 12-key disposition precedent.
- **Manifest exactness**: `brim_type` enum carries the 7 canonical values in canonical order (`auto_brim, brim_ears, painted, outer_only, inner_only, outer_and_inner, no_brim`); defaults all canonical-identical (`brim_object_gap` 0 [0,2]; `brim_ears_max_angle` 125 [0,180]; `brim_ears_detection_length` 1 min-0-no-max; `brim_use_efc_outline` false) — **no deviation rows, no human sign-off consumed**.
- **CONFIG_BLOCK honesty**: manifest bool/int/float/enum defaults do NOT thread into raw config (only percent/float_or_percent via the packet-185 `schema_defaults()` arm — none of these keys are percent-typed), so the static `ORCA_CONFIG_PADDING` twins ("brim_type = auto_brim", "brim_object_gap = 0") stay as the defaults source; explicit user values win once via the `emit_config_kv` dedup (padding-removal rejected per packets 254/255 precedent). AC-5 pins single-emission.
- **Host plumbing verified, not assumed**: key values flow CLI/sidecar → `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`) → `ResolvedConfig.extensions` → `run_pipeline_core` effective-config merge → CONFIG_BLOCK; reachability requires only the manifest declaration (`ConfigView::from_declared` filter drops undeclared keys — no `from_config` arm needed for the four unread keys); enum/bounds enforcement is host-side generic via `ConfigBoundsIndex` (AC-4 arm against the real manifest).
- **Test wiring grounded**: module tests are file-per-binary (no aggregator in `skirt-brim` — verified against the pattern source part-cooling); the guard pattern (`cooling_config_schema_tdd.rs`) needs a `toml = "0.8"` dev-dep skirt-brim lacked — caught during preflight grounding and added to Step 1's edit list. Manifest `.toml` edits are guest-fingerprint inputs (`guest_input_paths`, `xtask/src/build_guests.rs`), so the wasm-staleness snippet and per-step freshness gates are wired into the plan.
- Tier-table updates for this packet (`brim_ears` → dead/canonical-declared row disposition; the other five rows' verified placements) ride this ticket's closure into `04-asset-tier-assignment.md` as owned by the rename/queue workstream tickets — not edited by this session beyond this Answer (map index updated instead).
