# Task Map: 240b-support-raft-module

Crosswalk for this packet's share of queue row #7 of
`docs/specs/support-families-anchored-entities-plan.md`. Row #7 originally
allocated `TASK-409`..`TASK-418` to a single `240-support-raft` packet; that
packet was split at preflight into **240a-support-raft-substrate**
(`TASK-409`..`TASK-413`, plus `TASK-533`..`TASK-536` for scope the original
allocation did not cover) and **240b-support-raft-module** (this one,
`TASK-414`..`TASK-418`, plus `TASK-537`). **Re-derive the free range before
allocating any further ID** — `rg -o 'TASK-[0-9]{3}' docs/ -N --no-filename | sort -u | tail -1`
— rather than trusting the boundary implied here.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-414` | `Step 1` | `docs/03_wit_and_manifest.md`, ADR-0009 | `modules/core-modules/raft-default/*` (manifest + Cargo + wit-guest + skeleton src) | none | M | substrate FORWARD-DEP verification gate; new guest, rebuild in-step |
| `TASK-415` | `Step 2` | `docs/03_wit_and_manifest.md` | `crates/slicer-scheduler/tests/raft_claim_conflict_tdd.rs` | none | S | `claim:raft-fill` single holder; double-holder `ClaimConflict` (4-field variant, incl. `scope: ConflictScope`) |
| `TASK-416` | `Step 3` | `docs/08_coordinate_system.md` | `raft-default/src/lib.rs`, `crates/slicer-runtime/tests/integration/raft_geometry.rs` + registration | `SupportCommon.cpp::generate_raft_base` (delegated SUMMARY) | M | geometry port; determinism; negative-prefix ordering; zero anchored entities |
| `TASK-417` | `Step 4` | plan §13 traps T2/T8, `docs/ORCA_CONFIG_REFERENCE.md` | `raft-default.toml` `[config.schema]` (three NET-NEW keys), `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs` + registration | `PrintConfig.cpp::init_fff_params` (defaults FACT) | M | three keys declared/wired; bounds + undeclared-key negatives |
| `TASK-418` | `Step 5` | `docs/15_config_keys_reference.md` | the core-module manifests the re-derivation grep flags, `requirements.md` wire-or-record table, regenerated config doc | none | M | wire-or-record for every raft key the grep actually returns; `gen-config-docs` |
| `TASK-537` | `Steps 6+7` | ADR-0009, `docs/19_visual_debug.md` | `docs/adr/0009-*.md`, `docs/DEVIATION_LOG.md`, `docs/03_wit_and_manifest.md`; `tmp/p240b-*` artifacts | references comparison | S+S | ADR Decision-5 amendment + `D-*-ADR-0009-AMENDED` row; DEV-124 re-verification; human gate |

Copy costs from `implementation-plan.md`. Aggregate is `M`; no row is `L`.
