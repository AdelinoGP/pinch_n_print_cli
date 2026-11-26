# Preflight Gate: 258-skirt-type-and-draft-shield-keys

Reviewed: 2026-09-02 · Mode: `--preflight` · Symbol-inventory dispatched: 1 packet · Re-authored under map Authoring rules 1–6

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files)     | PASS | all five present and non-empty (`packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, `task-map.md`) |
| S1 Prerequisite-status truth      | PASS | 257a / 257b are cited as **ordering, not gating**, never as implemented deps; no packet is called implemented |
| S2 Deviation-ID conformance       | PASS | live log format is `DEV-###` (rows `DEV-157`, `DEV-158`); no `D-258*` token exists in the log; the packet declares `D-258-*` as packet-local labels and has Step 9 re-derive real `DEV-###` IDs at write time |
| S3 Schema-version computed        | PASS | no `*_SCHEMA_VERSION` is pinned; the packet states no IR schema bump is required |
| S4 ADR slot allocation            | PASS | no new ADR authored; `docs/adr/` runs 0001–0063, untouched |
| S5 Shipped-symbol existence/shape | PASS | verified in tree: `serialize_config_block` (private, `crates/slicer-gcode/src/serialize.rs`), `ORCA_CONFIG_PADDING`, `resolved_config_to_map` (both `slicer-gcode` and `slicer-wasm-host`), `guest_input_paths` (`xtask/src/build_guests.rs` — includes the module `.toml` and `src/`), `gen-config-docs --check`, `ResolvedConfig::filament_diameter` + `to_config_map`, `bind_module_config_view`, `SkirtBrim::{from_config, process, run_finalization, generate_skirt_entities, compute_bbox, make_rect_loop}`, `BBox2D`, `slicer_sdk::test_prelude::{print_entity, LayerCollectionFixtureBuilder}` |
| S6 WIT/IR identifier drift        | PASS | no WIT change claimed; `PrintEntity.region_key`, `RegionKey.object_id`, `ConfigValue::{Float, List}`, `ExtrusionRole::{Skirt, Brim}`, `Point3WithWidth` all resolve |
| S7 Test-target wiring             | PASS | `skirt-brim/tests/` has **no** aggregator `main.rs`, so the new `skirt_config_schema_tdd.rs` is a standalone binary needing no `mod` registration; `config_bounds_enforcement_tdd` and `gcode_header_thumbnail_config_blocks_tdd` are already registered in their `tests/integration/main.rs` aggregators |
| S8 ADR conformance                | PASS | no ADR normatively governs skirt/brim generation or `to_config_map`'s exported key set (zero `to_config_map` hits across `docs/adr/`). ADR-0015 (ConfigView as the normalized prepass export) is the nearest governing ADR; the packet conforms — it adds a key to the existing resolved map rather than introducing a parallel export path |
| (existing) AC runnable command    | PASS | all 10 ACs and all 3 negative cases end in a single runnable pipe-suffixed command; no `cargo test --workspace` appears as an AC command |
| (existing) Doc Impact Statement   | PASS | `docs/15_config_keys_reference.md` named, generated-only, verified by AC-10 with a key-presence grep |

### Blockers (S4/S5/S6)

None.

### High (S1/S2/S3/S7/S8)

None.

### Corrections applied during the gate

1. `docs/03_wit_and_manifest.md`'s section is `## Host-Boundary Access Enforcement (Normative)`, not "§host-boundary enforcement" — citations corrected in `packet.spec.md`, `requirements.md`, `implementation-plan.md`.
2. `docs/DEVIATION_LOG.md` uses `DEV-###`, not `D-###-SLUG`. The packet's `D-258-*` tokens were re-declared as packet-local labels and Step 9 now re-derives real `DEV-###` IDs from the log at write time.
3. `AC-1`'s key count corrected from six to seven (`layer_height` is also declared) and reconciled across all four files.

### Accepted FORWARD-DEPs

None. Both `[FWD]` items in `design.md` (wipe-tower grouping obstacle; per-filament `filament_diameter` array) are forwarded *out* of this packet and gate no AC here.

### Map gates (wayfinder Authoring rule 6)

- **(a) zero declaration-only keys** — **PASS.** All five keys build or drive a behaviour-changing decision point: `draft_shield` (span), `single_loop_draft_shield` (per-layer ring count), `skirt_start_angle` (start corner), `skirt_type` (per-object grouping + envelope merge), `min_skirt_length` (outward loop expansion). Zero keys are declared-with-gap; zero are returned; zero are dead-in-canonical.
- **(b) non-default AC per key** — **PASS.** `draft_shield` = `"enabled"` (AC-2), `single_loop_draft_shield` = `true` (AC-3), `skirt_start_angle` = `45.0` (AC-4), `skirt_type` = `"perobject"` (AC-5), `min_skirt_length` = `20.0` (AC-6). AC-N1's default-path identity is an *additional* criterion, never the sole evidence for any key.

**Verdict:** PREFLIGHT PASS (0 blockers, 0 high)
