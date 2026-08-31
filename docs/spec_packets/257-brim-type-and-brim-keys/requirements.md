# Requirements: brim-type-and-brim-keys

## Packet Metadata

- Grouped task IDs: none — queue packet (wayfinder precedent: packets 234a, 253–256 carry `task_ids: []`); implementation is recorded against wayfinder ticket 12 (`docs/specs/orca-feature-gap/issues/12-author-packet-p05-others-brim-skirt-brim.md`).
- Backlog source: `docs/specs/orca-feature-gap/issues/12-author-packet-p05-others-brim-skirt-brim.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P05).
- Packet status: `draft`.
- Aggregate context cost: `M`.

## Problem Statement

Packet-list entry P05 ("Others / Brim — skirt-brim", 6 keys, Tier A) is unauthored, and the `skirt-brim` module's config surface stops at the six legacy keys (`skirt_brim_enabled`, `skirt_loops`, `skirt_distance`, `skirt_height`, `brim_width`, `line_width`). OrcaSlicer's remaining brim vocabulary — how the brim is selected (`brim_type`), how far it sits from the object (`brim_object_gap`), and the ear/EFC parameters — has zero occurrences in this tree's crates and manifests (authoring-time greps; the only `brim_type`/`brim_object_gap` hits are static CONFIG_BLOCK padding literals in `ORCA_CONFIG_PADDING`, `crates/slicer-gcode/src/serialize.rs`, and a value-classification arm in `crates/slicer-model-io/src/loader.rs`). Until the keys are declared, 3MF sidecar values for them are dropped with "unrecognized object metadata key" and users cannot override them at all.

This packet is one coherent slice because all five keys share one owner (the `skirt-brim` manifest) and one decision-point family (brim geometry parameterization); ticket 12's scope ruling splits off only the dead `brim_ears` bool (see Out of Scope).

## In Scope

- Declaring in `modules/core-modules/skirt-brim/skirt-brim.toml` `[config.schema]`: `brim_type` (enum, 7 canonical values in canonical order, default `auto_brim`), `brim_object_gap` (float, 0.0, [0, 2]), `brim_ears_max_angle` (float, 125.0, [0, 180]), `brim_ears_detection_length` (float, 1.0, min 0), `brim_use_efc_outline` (bool, false) — all `group = "Skirt/Brim"`.
- New module-owned manifest schema guard test `modules/core-modules/skirt-brim/tests/brim_config_schema_tdd.rs` (pattern: `part-cooling/tests/cooling_config_schema_tdd.rs` — parses the TOML directly; the host's `load_module_from_paths` loader is not a module dependency).
- Wiring the single live decision point: `brim_type = "no_brim"` suppresses brim generation in `SkirtBrim` (both `process()` and `run_finalization` paths), leaving `brim_width > 0` as the enabling gate for all other modes.
- Scheduler-side bound/enum enforcement proof for the new keys against the real manifest (`crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` gains a `skirt-brim` arm).
- CONFIG_BLOCK reachability proof: an explicit `brim_type` value reaches the emitted CONFIG_BLOCK exactly once (padding dedup), asserted in `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`.
- Regenerating the generated tables of `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs` (mechanical; verified by `cargo xtask gen-config-docs --check`).

## Out of Scope

- **`brim_ears` (the boolean key) — ruled out of scope of the whole queue** by ticket 12's user ruling at authoring time (2026-08-30): canonical declares it in `PrintConfig.cpp` (coBool, default false, label "Brim ears") but no slicing, GUI, or preset code reads it, and it has no member in canonical's typed `PrintConfig` struct; ear physics are selected by `brim_type` values `brim_ears` (btEar) and `painted` (btPainted). This matches ticket 04's existing dead-in-canonical class and shrinks P05 from 6 to 5 keys (queue 407 → 406); the tier-table row update rides this packet's wayfinder-ticket resolution, not this packet's files.
- Ear geometry (`make_brim_ears_auto`: Douglas-Peucker decimation + convex/concave corner detection; canonical `Brim.cpp`) — no in-tree decision point; `brim_ears_max_angle`/`brim_ears_detection_length` are declared-with-gap.
- Per-object contour brim: canonical `outer_inner_brim_area` offsets from object slices; this tree's brim is rectangular loops around the layer-0 bounding box (`SkirtBrim::generate_brim_entities`). `brim_object_gap` is declared-with-gap; the bbox-vs-contour divergence is recorded, not bridged.
- Inner-brim modes (`inner_only`, `outer_and_inner`): no inner-brim geometry exists; selecting them today degrades to the outer path — recorded gap, no behavior invented.
- `brim_use_efc_outline` coupling: canonical `use_brim_efc_outline` requires `elefant_foot_compensation > 0` + layers > 0 + `raft_layers == 0`; this tree has no EFC geometry (the key appears only as a padding literal), so the key is declared-with-gap.
- Auto-brim width computation (canonical `configBrimWidthByVolumeGroups`): out of scope; `auto_brim` keeps this tree's fixed `brim_width` semantics.
- Any CONFIG_BLOCK padding-table edit (`ORCA_CONFIG_PADDING`): dedup already makes raw-config values win (packet 255's ruling; precedent from packet 254's rejection of padding removal), so the static `("brim_type", "auto_brim")` / `("brim_object_gap", "0")` entries stay.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated; regenerate + `--check` (do not hand-edit, do not read in full).
- `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract — delegated SUMMARY only.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the five keys (types, defaults, min/max, enum order, UI category); authoring-time evidence already captured in §Per-Key Canonical Evidence and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `outer_inner_brim_area`, `make_brim_ears_auto`, `use_brim_efc_outline` — the recorded-gap descriptions rest on these three functions.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp` — `PrintObject::has_brim` — the `brim_type`/`brim_width` interplay a future mode-aware gate must express.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Per-Key Canonical Evidence

Tier-A membership re-derived per key at authoring time (ticket 12's obligation) — the tier table's rows were verified against canonical reads and corrected where the live decision point is absent. All defaults below match canonical exactly (`PrintConfig.cpp`); none are deviations, so no `DEVIATION_LOG.md` rows and no human sign-off are consumed.

| Key | Canonical decl (`PrintConfig.cpp`) | Canonical consumer | Live decision point in this tree? | Disposition |
| --- | --- | --- | --- | --- |
| `brim_type` | coEnum `BrimType`, 7 values in order `auto_brim, brim_ears, painted, outer_only, inner_only, outer_and_inner, no_brim`, default `btAutoBrim` | `Brim.cpp::outer_inner_brim_area` (has_outer/has_inner + ear-mode selection), `Print.hpp::has_brim` | Yes — the on/off gate: `SkirtBrim::run_finalization`'s `brim_width > 0` arm (`modules/core-modules/skirt-brim/src/lib.rs`) | **Wired (gate)**: `no_brim` forces no brim; other modes keep current bbox-loop behavior (inner/ear/painted mode physics declared-with-gap) |
| `brim_object_gap` | coFloat, default 0, min 0, max 2, comAdvanced | `Brim.cpp::outer_inner_brim_area` — `brim_offset = scale_(value)`; applied by offsetting the outer brim boundary away from the object contour (and inward-negative for inner brim) | No — brim is generated as bbox rect loops, not object-contour offsets | **Declared-with-gap** |
| `brim_ears_max_angle` | coFloat, default 125, min 0, max 180, comAdvanced | `Brim.cpp::make_brim_ears_auto` — `angle_threshold = (180 − max_angle)·π/180` via `convex_points`/`concave_points` | No — no ear detection exists | **Declared-with-gap** |
| `brim_ears_detection_length` | coFloat, default 1, min 0 (no max), comAdvanced | `Brim.cpp::make_brim_ears_auto` — Douglas-Peucker decimation minimum deviation before corner detection; 0 disables decimation | No — no ear geometry | **Declared-with-gap** |
| `brim_use_efc_outline` | coBool, default false, comAdvanced | `Brim.cpp::use_brim_efc_outline` — gates on `elefant_foot_compensation > 0` ∧ `elefant_foot_compensation_layers > 0` ∧ `raft_layers == 0`; selects post-EFC bottom outline as brim base in `outer_inner_brim_area` | No — no elephant-foot-compensation geometry in tree (padding literal only) | **Declared-with-gap** |
| ~~`brim_ears`~~ | coBool, default false — **dead in canonical**: declared but never read; no typed `PrintConfig` member; absent from preset lists | none | — | **Out of scope** (ticket-12 user ruling; dead-in-canonical class) |

Plumbing-key evidence standard (wayfinder ticket 02): each declared key's default resolves to the canonical value (AC-1) and the wired key's value reaches its consumer (AC-2/AC-4/AC-5). The four declared-with-gap keys meet the plumbing standard exactly — declared + default-matches-upstream + reaches the scheduler's bounds layer (AC-4); their behavioural tests wait on the gap-closing packets.

Behavioral-invariant candidates (ported-assertion style, no goldens): the `no_brim` suppression invariant (AC-2) and the default-identity invariant (AC-3) are invariant-shaped; `make_brim_ears_auto`'s decimation/angle assertions are canonical `tests/fff_print`-adjacent but not ported here — their home is the ear-geometry packet that closes the gap.

## Acceptance Summary

- Positive: `AC-1` (manifest schema exactness), `AC-2` (`no_brim` gate), `AC-3` (default-path identity), `AC-4` (bounds/enum enforcement), `AC-5` (CONFIG_BLOCK reachability, single emission), `AC-6` (generated doc-15 tables).
- Negative: `AC-N1` (declaration must not bypass the `brim_width > 0` gate), `AC-N2` (schema guard names drifted/removed keys).
- Cross-packet impact: none gating; future packets close the recorded gaps (ear geometry, inner brim, object-contour brim, EFC coupling) and will consume these manifest entries as the declaration of record.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p skirt-brim --test brim_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1, AC-N2 — manifest schema exactness | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2, AC-3, AC-N1 — gate + identity invariants | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | legacy `process()` path unaffected (AC-3 counterpart on the test-only path) | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-4 — real-manifest bound/enum rejection | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-5 — CONFIG_BLOCK single emission of explicit value | FACT pass/fail |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness — manifest `.toml` is a fingerprint input (`guest_input_paths`, `xtask/src/build_guests.rs`) | FACT exit code (fresh=0) |
| `cargo xtask gen-config-docs --check; echo "exit=$?"` | doc 15 generated tables match manifests (AC-6) | FACT exit code |
| `cargo check --workspace --all-targets` | workspace gate (all targets compile) | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- The manifest edit and its schema-guard test land in the same step (the guard is the manifest's only enforcement).
- `cargo xtask build-guests --check` runs after every step that edits the manifest or `src/lib.rs`; a stale result (exit 1) must be rebuilt (drop `--check`) before any further test is trusted on those targets.
- AC-5's test must assert single emission (`emitted`-set dedup semantics), not merely presence — the padding twin makes double-emission the likely silent failure.

## Context Discipline Notes

- `crates/slicer-gcode/src/serialize.rs` is large: read only the `serialize_config_block`/`emit_config_kv`/`ORCA_CONFIG_PADDING` range (roughly lines 315–545); never the whole file.
- `crates/slicer-scheduler/src/config_resolution.rs` is a delegated-only context for implementers — its mechanism (enum values via `ConfigBoundsIndex`, manifest-default threading via `bounds.schema_defaults()`) is summarized in this file; do not browse it, cite `apply_cli_key`/`schema_defaults` by symbol.
- The e2e CONFIG_BLOCK driver is `run_slice_with_config` in `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs` only if AC-5's integration binary proves insufficient; the subprocess-spawning e2e is the fallback, not the default home.