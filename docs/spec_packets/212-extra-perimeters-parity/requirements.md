# Requirements: 212-extra-perimeters-parity

## Packet Metadata

- Grouped task IDs: `TASK-328`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`
- Aggregate context cost: `S`

## Problem Statement

`DEV-132` (`docs/DEVIATION_LOG.md`) records two coupled divergences around `extra_perimeters`.

**(a) The read gap.** `classic-perimeters` registers `[config.schema.extra_perimeters]` in `classic-perimeters.toml` and `run_perimeters` (`modules/core-modules/classic-perimeters/src/lib.rs`) applies it on every layer as `let base_wall_count = base_wall_count + extra_perimeters;`, pinned by `crates/slicer-runtime/tests/integration/extra_perimeters_config_tdd.rs`. `arachne-perimeters` registers no such key and `arachne_params_from_config` (`modules/core-modules/arachne-perimeters/src/lib.rs`) never reads it — verified this session: the only `extra_perimeters` matches under `modules/core-modules/arachne-perimeters/` are two `extra_perimeters_on_overhangs` hits in the manifest, zero in `src/`. A user who sets `extra_perimeters` and flips `wall_generator` to `arachne` silently loses the bonus walls on every layer, with no warning.

**(b) The modelling divergence.** PnP models `extra_perimeters` as a global config key. Canonically it is `Surface::extra_perimeters`, an `unsigned short` member of `Surface` (`Surface.hpp`), folded into the wall count in *both* generators via `int loop_number = this->config->wall_loops + surface.extra_perimeters - 1;` (`PerimeterGenerator::process_classic` and `PerimeterGenerator::process_arachne`, `PerimeterGenerator.cpp` — identical but for whitespace). There is no `add("extra_perimeters", ...)` in `PrintConfigDef::init_fff_params` (`PrintConfig.cpp`); the only quoted `"extra_perimeters"` name upstream is `#define JSON_SURF_EXTRA_PERIMETERS` in `Print.cpp`, a serialization tag.

DEV-132 filed these together because "fixing either alone entrenches the other". **This packet resolves that tension with evidence rather than by deferring both.** Two grounded facts decide it:

1. Canonical's only in-code writer of `Surface::extra_perimeters` is `PrintObject::make_perimeters` (`PrintObject.cpp`), and the BBS patch short-circuits that loop body with a bare `continue`, so the field is in practice always `0` upstream. The other writer is `from_json(const json&, Surface&)` (`Print.cpp`), i.e. deserialization only. Building per-`Surface` plumbing would therefore port a field that upstream never sets.
2. PnP cannot express per-region config at this seam today anyway: `SliceRegionView` (`crates/slicer-sdk/src/views.rs`) carries no `config_id`, and `LayerModule::run_perimeters` takes one layer-wide `&ConfigView` for the whole `&[SliceRegionView]` slice. Half (b) is an IR + WIT + marshalling change, not a module-local one.

So the packet **stages** them: fix (a) now, and re-file (b) as its own deviation row with the above evidence rather than leaving it silently absorbed into a "closed" DEV-132. Half (b) is explicitly out of scope here.

The unit relation for (a) is fixed by canonical: `process_arachne` constructs `Arachne::WallToolPaths(..., coord_t(loop_number + 1), ...)` and `WallToolPaths::generate` computes `max_bead_count = 2 * inset_count`, giving `max_bead_count = 2 * (wall_loops + extra_perimeters)`. PnP's `arachne_params_from_config` already auto-derives `max_bead_count = (2 * wall_count).max(1)`; the fix adds `extra_perimeters` inside that derivation.

## In Scope

- Register `[config.schema.extra_perimeters]` in `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` with `type = "int"`, `default = 0`, `min = 0`, `max = 10` — field-for-field identical to `classic-perimeters.toml`'s block except for the `description`, which states the arachne-specific `max_bead_count = 2 * (wall_count + extra_perimeters)` mapping. Registration is load-bearing, not cosmetic: `ConfigView::from_declared` (`crates/slicer-ir/src/slice_ir.rs`) drops any key the manifest does not declare, so an unregistered read is dead on live slices.
- Fold `extra_perimeters` into the auto-derive branch of `max_bead_count` in `arachne_params_from_config` (`modules/core-modules/arachne-perimeters/src/lib.rs`), i.e. the `_ => (2 * wall_count).max(1)` arm becomes `2 * (wall_count + extra_perimeters)`.
- Deliberately leave the explicit-override arm (`Some(v) if v > 0 => v as u32`) untouched, so a positive `max_bead_count` stays honoured verbatim per its own manifest `description`. Pinned by AC-N1 and AC-N3.
- Extend `crates/slicer-runtime/tests/integration/extra_perimeters_config_tdd.rs` with the arachne positive case, the arachne zero no-op, the cross-generator equality case, the explicit-override negative, and the `alternate_extra_wall` composition case. This file already exists, is already registered in `crates/slicer-runtime/tests/integration/main.rs`, and `slicer-runtime` already dev-depends on BOTH `classic-perimeters` and `arachne-perimeters` (`crates/slicer-runtime/Cargo.toml`), so no new dev-dependency or `mod` line is needed.
- Add `("extra_perimeters", Int(0))` to `ARACHNE_FALLBACKS` in `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`. This guard is EXHAUSTIVE-BY-ENUMERATION with set-equality in both directions — adding a manifest key without a table row fails the suite.
- Regenerate the generated tables in `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs` and the Open Deviation Map view in `docs/07_implementation_status.md` via `cargo xtask check-deviations` (both are marked generated; hand-editing them is prohibited and `--check` fails on drift).
- Update `DEV-132`'s `Status` in `docs/DEVIATION_LOG.md` to record half (a) closed and cite the newly-allocated `DEV-###` row carrying half (b); author that new row with the canonical dead-writer evidence and the `SliceRegionView`-has-no-`config_id` evidence.
- Add the `TASK-328` line to `docs/07_implementation_status.md`. Verified this session: `TASK-328` does not exist there yet (re-derived at preflight 2026-08-28: highest live ID in the backlog is `TASK-507`, and `TASK-328` appears nowhere in it; the ID is reserved for this packet by row #7 of `docs/specs/deviation-remediation-206-212-plan.md`), so the packet creates it rather than checking off an existing row.

## Out of Scope

- Any per-`Surface` or per-region `extra_perimeters` model (DEV-132 half (b)). It requires a new field on `SliceRegionView`/`SlicedRegion`, WIT/IR marshalling, and a scheduler-side per-region config path; it is re-filed as its own deviation, not built here.
- `modules/core-modules/classic-perimeters/**` behaviour. Classic already applies the key correctly and its two existing tests in `extra_perimeters_config_tdd.rs` must stay green byte-for-byte.
- DEV-125 (`alternate_extra_wall`), which just landed in both generators. This packet must COMPOSE with it (AC-N2) but must not modify the `params.max_bead_count += 2` block in `run_perimeters` or classic's four-conjunct `base_wall_count + 1` guard.
- `extra_perimeters_on_overhangs`, a distinct canonical `coBool` option (`PrintConfig.cpp`) already registered in both manifests and marked `Unread` for arachne in `ARACHNE_FALLBACKS`. Not touched.
- The stale `wall_count` claim in `modules/core-modules/arachne-perimeters/tests/alternate_extra_wall_tdd.rs`'s module doc ("the module's vestigial, unread `wall_count` config key"). `wall_count` IS declared (`arachne-perimeters.toml`) and IS read (`arachne_params_from_config`); the comment has rotted. Correcting it is optional cleanup, tracked as `[FWD-2]` in `design.md`, not required scope.
- `crates/slicer-gcode/src/serialize.rs`'s `("extra_perimeters", "0")` CONFIG_BLOCK entry — host-side, generator-agnostic, unaffected.

## Authoritative Docs

- `docs/15_config_keys_reference.md` - large and partly GENERATED. Read only the owner rows adjacent to `extra_perimeters`; never hand-edit the generated tables, never read the file in full.
- `docs/DEVIATION_LOG.md` - read the `DEV-132` row only (single long table row; ranged read). Re-derive the next free `DEV-###` at the moment of writing with `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1` — never freeze it into an artifact.
- `docs/07_implementation_status.md` - over 300 lines; delegate. Re-derive the highest `TASK-###` at point of use, do not quote this packet's survey.
- `docs/03_wit_and_manifest.md` - delegated SUMMARY only, and only if the `[config.schema.<key>]` field set for an `int` key is in doubt.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — the `int loop_number = this->config->wall_loops + surface.extra_perimeters - 1;` fold, present in BOTH `PerimeterGenerator::process_classic` and `PerimeterGenerator::process_arachne` (they differ only in whitespace before the trailing comment); this is the behaviour being mirrored into arachne.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` — `WallToolPaths::generate`'s `max_bead_count = 2 * inset_count` relation, and `process_arachne`'s `WallToolPaths(..., coord_t(loop_number + 1), ...)` constructor call: together they give `max_bead_count = 2 * (wall_loops + extra_perimeters)`, the unit relation this packet ports.
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` and `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `Surface::extra_perimeters` (`unsigned short`, zero-initialised in every ctor) and its sole in-code writer `PrintObject::make_perimeters`; deliberately NOT borrowed, because that writer's loop body is short-circuited by the BBS patch, making the field effectively always `0` upstream.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — evidence of ABSENCE: `PrintConfigDef::init_fff_params` has no `add("extra_perimeters", ...)`; only `add("extra_perimeters_on_overhangs", coBool)`. PnP's `extra_perimeters` config key has no canonical config-option counterpart.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-8`. Measurable refinements not restated in their Given/When/Then text: the emitted-wall-count mapping `emitted = max_bead_count / 2` for an even cap is an EMPIRICAL property of `LimitedBeadingStrategy`'s filtered sentinel pair, measured and documented in `modules/core-modules/arachne-perimeters/tests/alternate_extra_wall_tdd.rs`'s module doc against this exact 20 mm / 1.0 mm-bead fixture. Step 1 re-measures it before pinning constants and records the observed baseline in the new test's doc comment; if the measured baseline differs from the values in AC-1/AC-2/AC-N1/AC-N2, the packet is `[BLOCK]`ed rather than having its assertions relaxed.
- Negative: `AC-N1` through `AC-N3`.
- Cross-packet impact: none. Row #7 of `docs/specs/deviation-remediation-206-212-plan.md` depends on nothing and unblocks nothing. It does allocate one new `DEV-###`; because packets 206-211 may allocate theirs concurrently, the ID MUST be re-derived at the moment of writing, never reserved in advance.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd 2>&1 \| tee target/test-output.log \| rg '^test result'` | AC-1/2/3, AC-N1/N2 plus the two pre-existing classic tests | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `mkdir -p target && cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd 2>&1 \| tee target/test-output.log \| rg '^test result'` | AC-5 exhaustive manifest/fallback set-equality | FACT pass/fail |
| `mkdir -p target && cargo test -p arachne-perimeters --test alternate_extra_wall_tdd 2>&1 \| tee target/test-output.log \| rg '^test result'` | AC-N3 no-regression on the explicit-cap path | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest-WASM freshness after editing `modules/core-modules/arachne-perimeters/**` | FACT `clean` or the `STALE:` list |
| `test "$(rg -A6 -N ... \| rg -c ...)" = "4"` for both manifests (full form in `packet.spec.md` AC-4) | AC-4 manifest block shape vs classic | FACT `PASS` or the differing count |
| `cargo xtask check-deviations --check` | AC-6 generated doc tables in sync | FACT exit code |
| `rg -q 'DEV-132.*[Cc]losed' docs/DEVIATION_LOG.md && rg -q 'Surface::extra_perimeters' docs/DEVIATION_LOG.md && rg -q 'PrintObject::make_perimeters' docs/DEVIATION_LOG.md && echo PASS` | AC-7 ledger split | FACT `PASS`/absent |
| `rg -q '^- \[x\] TASK-328' docs/07_implementation_status.md && echo PASS` | AC-8 backlog row | FACT `PASS`/absent |
| `cargo check --workspace --all-targets` | Packet gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Packet gate | FACT pass/fail |

## Step Completion Expectations

- Step 1's new tests MUST be red before Step 2 lands, and RED FOR THE RIGHT REASON: an arachne case failing because `arachne_params_from_config` ignores `extra_perimeters` (observed count `2` where `4` is expected), not because the fixture is geometry-limited or the module errors. Record the observed failure counts; Step 2 reuses them as the before/after evidence.
- Step 2 edits `modules/core-modules/arachne-perimeters/**`, a guest-WASM input path. `cargo xtask build-guests --check` must run after that edit and before any conclusion about a failing test, in EVERY later step.
- Step 4's `DEV-###` allocation is a ledger fact. Re-derive it inside Step 4 with the `rg | sort -u | tail -1` command; do not carry an ID forward from Step 1-3 notes or from this document.

## Context Discipline Notes

- `modules/core-modules/arachne-perimeters/src/lib.rs` is over 1200 lines — ranged reads only. The two windows that matter are `arachne_params_from_config`'s `max_bead_count` derivation (the `max_bead_count_explicit` / `wall_count` / `match` block) and the head of `run_perimeters` from the `arachne_params_from_config(config, layer_index == 0)?` call through the `only_one_wall_first_layer` clamp. Nothing else in that file is in scope.
- `modules/core-modules/classic-perimeters/src/lib.rs` is over 1500 lines and is READ-ONLY here. Read only the `base_wall_count` derivation window in `run_perimeters` (the `extra_perimeters` addition, the DEV-125 `alternate_extra_wall` guard, and the `only_one_wall_first_layer` clamp) to confirm ordering parity.
- `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` is long and mostly a transcription table. Jump straight to `ARACHNE_FALLBACKS`; do not read `CLASSIC_FALLBACKS` or the module doc in full.
- `docs/15_config_keys_reference.md` and `docs/07_implementation_status.md` are large and partly generated. Never read either in full; verify with `rg -q` and regenerate with `cargo xtask check-deviations`.
- Tempting read to skip: `OrcaSlicerDocumented/**`. Every canonical fact this packet needs is already stated in `requirements.md` §Problem Statement and the Orca obligations section. Re-verify by delegation only if a stated fact is contradicted.
