# ADR-0045: per-stage versioned WIT packages — plan

Governing decision: `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` (accepted).
Predecessor: `docs/adr/0044-wit-world-version-is-not-an-identity-token.md` (accepted).

The approved plan is stored verbatim below, followed by the packet queue.

---

## Approved plan (verbatim)

### Context

ADR-0044 (accepted) proved the WIT world version enforces nothing and removed it
from module identity, closing with: *"Giving it teeth requires ADR-0045."*
ADR-0045 (proposed) is that fix. This plan amends it, then executes it.

**The problem.** All four worlds export bare freestanding funcs. wasmtime's
generated `Indices::new` eagerly resolves *every* export at `instantiate`, so a
guest must satisfy the world's entire export surface. `#[slicer_module]` pads the
gap with `Ok(())` stubs. Net effect: any change to any stage invalidates every
guest in the tier — `arachne-perimeters` ships `run-infill-postprocess` with a
`prior-infill` param it never reads, and packet 130 invalidated its `.wasm`.

**Why now.** The benefit is an ecosystem benefit and the user confirmed the
third-party ecosystem (`docs/00`, `module_search_path.rs`) is a real commitment.
No registry, no install path, no out-of-tree module exists yet — breaking changes
are free now and never again.

#### ADR-0045 must be amended before it is accepted

Its decision names `slicer:world-layer/infill-postprocess@2.0.0`. **In WIT,
`@version` is a package-level attribute; interfaces cannot carry their own**
(proof in-tree: `wit/deps/common.wit` — `package slicer:common;` holds two
unversioned interfaces). So all ten interfaces in a `slicer:world-layer@2.0.0`
package share one version. Replaying packet 130: a required-param addition is
breaking → package majors `1.x → 2.0.0` → every interface moves from wasmtime
alt-key `@1` to `@2` → `arachne-perimeters` misses again. **The ADR's headline
row — "infill change breaks perimeters modules: no — untouched, doesn't even
rebuild" — fails under its own proposed shape.** Only one package *per stage*
delivers it.

The ADR's wasmtime claim is otherwise verified against the pinned source
(`wasmtime-environ-43.0.1/src/component/names.rs::alternate_lookup_key`):
`@1.1.2 → @1`, `@0.1.0 → @0.1`, `@0.0.1 → None`. Major-track compat requires
major ≥ 1 — hence starting every package at `1.0.0`.

#### The lifecycle finding

`on-print-start` / `on-print-end` are **padding squatting on a real concept's
name**, not a lifecycle:

- `call_on_print_start` / `call_on_print_end` have **zero callers in the host**.
  `docs/04_host_scheduler.md:1449` ("call on-print-start on all modules") is fiction.
- The macro's `on_print_end` glue is hardcoded `Ok(())` and **never dispatches**
  to the trait. Every module's `on_print_end` is unreachable.
- The macro's `on_print_start` glue does `Ok(_m) => Ok(())` — **constructs and
  discards**. Meanwhile all 15 `run_*` arms construct the module *per call*, so
  `docs/05`'s "initialize expensive resources once per print" is inverted: it runs
  once per layer, per stage. No `OnceCell`/`static` retains anything.
- **OrcaSlicer has no such hook** — zero hits in `libslic3r`. It expresses lifetime
  by *where the object lives*: `SeamPlacer::init` once per print on a `GCode`
  member; `Fill` (`Layer::make_fills`) and `PerimeterGenerator`
  (`LayerRegion::make_perimeters`) rebuilt per layer. Our tier system already
  encodes both — per-print = prepass + **Blackboard** (ADR-0029); per-layer = layer
  tier.
- The real "print start/end" is a **custom G-code template** (`machine_start_gcode`
  / `machine_end_gcode`), read at `run_gcode_postprocess` by `machine-gcode-emit`
  — a different tier, lifetime, and owner. Two things named "print start"; one real.

Kept honest: deleting the hook forecloses a layer module holding cheap *private*
state across layers. It cannot do so today (rebuilt per call), so nothing that
currently works is lost — but re-adding it later needs a new contract, not this one.

### Approach

#### 1. Amend + accept ADR-0045

Rewrite the decision to **one versioned WIT package per stage**; correct the
example string; add the package-vs-interface versioning proof; add an
"Alternatives rejected" section (tier-package-with-interfaces, and
tier-package-never-major-bumped — the latter works but re-lies about the version,
the exact sin ADR-0044 killed). Set `Status: accepted`.

#### 2. The split — 17 packages, all four tiers

One package per stage, tier-prefixed, all at `@1.0.0` (the current `world-layer@2.0.0`
is discarded in the reset; the user is resetting internal versioning anyway):

```wit
package slicer:layer-perimeters@1.0.0;
interface perimeters { run: func(...) -> result<_, module-error>; }
world perimeters-module { import ...; export perimeters; }
```

10 layer + 4 prepass + 2 postpass + 1 finalization. All four tiers, so exactly one
mechanism exists — the three small tiers are quiet today for the same accidental
reason `world-layer` was quiet for 150 packets.

Since `[stage] id` is **singular in all 20 manifests**, the host always knows which
stage to instantiate. **No probing.** ADR-0045's "probes each and tolerates the
miss" assumed multi-stage modules and should be dropped from the amended text.

- **Miss policy: fatal at load**, naming the expected `package/iface@version`.
  Kills the lying `Ok(())` stubs; satisfies ADR-0015 ("do not swallow").
- `[stage] id` **stays** — the DAG validator and `dag_cli` plan without
  instantiating WASM (per ADR-0006's rejected alternative).
- `wit-world`, `SUPPORTED_WIT_WORLDS`, `validate_wit_world` **retire** (ADR-0044/45).

#### 3. Delete the lifecycle hooks

- WIT `on-print-start` / `on-print-end` → **deleted**.
- SDK `LayerModule::on_print_start(config) -> Result<Self>` → rename **`from_config`**
  (unchanged per-call constructor, honest name). `on_print_end` → deleted from all
  four traits.
- `module_new.rs` stops scaffolding it, and drops its vacuous
  `on_print_start_succeeds()` test.

#### 4. Fix the self-certifying table

`slicer-schema/src/lib.rs::WORLD_LIFECYCLE_EXPORTS` claims all four worlds ship
lifecycle exports; **only `world-layer.wit` declares them**. Its guard test
`every_world_has_lifecycle_exports` reads that table and asserts against the same
table — vacuous, the identical pathology ADR-0044 documented for
`wit_world_major_version_mismatch_rejects_future_major`. The table dies with the
hooks; any surviving schema↔WIT assertion must parse the canonical `.wit`, reusing
the machinery already in `wit_drift_detection_tdd.rs`.

#### 5. Fold in: CLI-binary freshness

`pnp_cli` is a **separate package**, so `cargo test -p slicer-runtime` never rebuilds
it; `slicer_cache.rs::pnp_cli_bin` probes the filesystem and **prefers a stale
`target/release` over a fresh debug build**. It already produced a false baseline
for the previous session. Switch to `env!("CARGO_BIN_EXE_pnp_cli")` (+ a dev-dep on
`pnp-cli`) so Cargo guarantees freshness. This packet's blast radius is measured by
these very tests.

#### Critical files

| Path | Change |
|---|---|
| `crates/slicer-schema/wit/deps/world-*/` | → 17 per-stage packages |
| `crates/slicer-schema/src/lib.rs` | `STAGES` (`wit_export`→package+iface), `WORLD_*`, delete `WORLD_LIFECYCLE_EXPORTS` |
| `crates/slicer-wasm-host/src/host.rs` | 4 `bindgen!` → 17 (reuse ADR-0002's `with:` remap to keep one Rust type set) |
| `crates/slicer-wasm-host/src/dispatch.rs` | instantiate by `stage_id`; existing `match stage_id.as_str()` is the seam |
| `crates/slicer-macros/src/lib.rs` | emit glue for the detected stage only; delete stub arms + lifecycle glue |
| `crates/slicer-sdk/src/traits.rs` | `on_print_start`→`from_config`; delete `on_print_end` ×4 |
| `crates/pnp-cli/src/module_new.rs` | scaffold + its test |
| `modules/core-modules/*/*.toml` | drop `wit-world` (20 files, one line each) |
| `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` | version pin; drift guards |
| `crates/slicer-runtime/tests/common/slicer_cache.rs` | `CARGO_BIN_EXE_pnp_cli` |

Reuse, don't reinvent: `slicer_schema::export_for_stage_id` (ADR-0006, sole
stage→export lookup) — extend it, don't add a parallel table.

#### Docs + domain model

- `CONTEXT.md`: **Module tier** ("Each tier has exactly one WIT world" — now false);
  **Stage contract** (unit of contract becomes the stage); **Stage interface**
  (drop "Not yet implemented"). Delete no glossary entry for lifecycle — there
  never was one, correctly.
- `docs/03` (WIT listing), `docs/04:1449` (fiction), `docs/05` §"Module State
  Lifecycle (Normative)" + "once per print" (both describe a system that has never
  existed), `traits.rs` docs.
- `docs/15_config_keys_reference.md` — **fix now, two edits**: list only the 2
  macros that actually resolve (`[bed_temperature_initial_layer_single]`,
  `[nozzle_temperature_initial_layer]`); correct `PostPass::LayerFinalization` →
  `PostPass::GCodePostProcess`.
- `crates/slicer-schema/wit/README.md` — stale "World packages carry `@1.0.0`".
- `DEVIATION_LOG.md` — new rows (see below); none of these gaps has one today.

### Out of scope — file, don't fix

1. **Custom G-code injection points: 2 of 15 implemented.** Missing:
   `file_start_gcode`, `before_layer_change_gcode`, `layer_change_gcode`,
   `time_lapse_gcode`, `wrapping_detection_gcode`, `change_filament_gcode`,
   `filament_start_gcode`, `filament_end_gcode`, `machine_pause_gcode`,
   `template_custom_gcode`, `printing_by_object_gcode`, and the three-way
   `*_change_extrusion_role_gcode` family. Own parity packet + DEV row. Note
   `per_object_gcode` is PrusaSlicer-only — don't chase it. The design opening:
   canonical hand-inlines the same block 20+ times and its only registry
   (`s_CustomGcodeSpecificPlaceholders`) is debug-gated and already drifted, so a
   real injection-point registry *improves on* canonical rather than merely matching it.
   Also for that packet: unknown `[key]` passes through verbatim to the printer
   (Orca's parser errors), and `substitute_placeholders` does `bytes[i] as char`
   → mojibake on non-ASCII.
2. **DAG validation downgrade** (`run.rs:469-481`, DEV-026, Risk: High). Cycles,
   write conflicts, missing deps are advisory. Root cause is known and owned:
   commit `607fca58` (2026-05-26) — *"Pragmatic fix… until the synthetic-host-builtin
   modeling lands."* It never landed. Own packet.
3. **7 parity failures** — dispatch a separate session (blocked on plan approval;
   plan mode bars edits). **The previous session's brief for it is wrong** and must
   be rewritten: `object_id` is a uuid5 of the **absolute path** (`path_object_id` in
   `crates/slicer-model-io/src/loader.rs` — cite the crate, always: a bare `loader.rs`
   here was resolved downstream to a non-existent `crates/slicer-runtime/src/loader.rs`
   and survived three reviews, because each checked the line number and none asked
   whether the file existed),
   not "a per-run temp path"; `perimeter_parity.rs` spawns **no binary** (in-process
   `load_model`), so staleness cannot explain its 6 failures; the baselines were
   recorded at a different absolute path. Agreed fix: **basename + index**, collision
   on duplicate basenames → hard error at load. This also fixes a product bug —
   `; object_height:64d5a57c-… = 20` leaks into shipped G-code, so **output is not
   reproducible across machines**. Keep these tests known-red across this packet:
   same 7 red before and after ⇒ the refactor is behavior-neutral.

### Execution

Author as a spec packet via `/spec-packet-generator` under `.ralph/specs/`, gate
with `/spec-review <packet> --preflight`, execute with `/swarm`.

Sequencing: amend ADR-0045 → per-stage WIT → macro → host bindgen → dispatch →
manifests → docs/CONTEXT. The parity-baseline fix should land **first** in its own
session so the before/after signal is trustworthy.

### Verification

```bash
cargo xtask build-guests --check      # MUST be clean before believing any failure
cargo xtask build-guests              # WIT + macro + sdk + schema all invalidate every guest
cargo check --workspace --all-targets

# narrow, proves the contract:
cargo test -p slicer-runtime --test contract wit_drift_detection_tdd 2>&1 | tee target/test-output.log
cargo test -p slicer-schema 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test contract dispatch_tdd 2>&1 | tee target/test-output.log

cargo clippy --workspace --all-targets -- -D warnings
```

End-to-end evidence the split actually works — the claim ADR-0045 rests on:

1. Touch one stage's `.wit`, bump only that package.
2. `cargo xtask build-guests --check` ⇒ **only that stage's guests are STALE**;
   `arachne-perimeters` is not.
3. Slice a real model, confirm unchanged output:
   `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/out.gcode`
4. Negative test: a module declaring a stage whose interface it doesn't export
   **fails at load** with a diagnostic naming `package/iface@version` — not a
   silent no-op.

Packet close (per CLAUDE.md, dispatch to a sub-agent with a `FACT pass/fail` return):
`cargo xtask test --summary --workspace`. The 7 parity tests must be red-before /
red-after unless their own session has already landed.

---

## Grounding corrections to the approved plan

Two of the plan's claims were falsified while grounding packet #1. Both are
corrected here; the plan text above is preserved verbatim and is therefore wrong
on these two points by design.

1. **`env!("CARGO_BIN_EXE_pnp_cli")` cannot fix the CLI-staleness trap as stated.**
   Cargo sets `CARGO_BIN_EXE_<name>` only for integration tests of the package that
   *defines* the binary — which is why `crates/pnp-cli/tests/e2e_integration_tdd.rs:394`
   already uses it successfully. Every fragile spawn site is in `slicer-runtime`
   (`tests/common/slicer_cache.rs`, `tests/e2e/slice_end_to_end_tdd.rs`,
   `tests/e2e/slicing_precision_integration_tdd.rs`, `benches/gate_evidence.rs`), a
   different package; a dev-dependency does not make the var available. **Replacement
   (approved):** mirror the guest gate — `xtask test` Step 1 checks/rebuilds `pnp_cli`
   alongside `build_guests::check_command`, **and** `slicer_cache.rs::pnp_cli_bin`
   asserts freshness itself so that plain `cargo test -p slicer-runtime` (the narrow
   invocation `CLAUDE.md` recommends, and the one that produced the false baseline)
   fails loudly rather than silently spawning a stale binary. The release-over-debug
   fallback is removed.

2. **`WORLD_LIFECYCLE_EXPORTS` is not dead code.** It has two real consumers —
   `crates/slicer-macros/src/lib.rs:19` (imported as `WORLD_LIFECYCLE`, used at `:148`
   to build the module's `wit_exports` metadata list) and
   `crates/pnp-cli/src/module_new.rs:215` via `lifecycle_exports_for_world`. Verified
   that the macro uses it only for the `SlicerModuleSchema` metadata surface, never
   for the `impl Guest` glue (which is hardcoded). **Resolution (approved):** delete
   the table and both consumers — with the lifecycle exports gone, `wit_exports`
   collapses to the single stage export and `module_new`'s `expected_exports` to the
   same. Consequential surfaces packet #1 must also resolve:
   `slicer_schema::ExportKind::Lifecycle` becomes unconstructible (a `dead_code` risk
   under `-D warnings`); `SlicerModuleSchema.exports`' "lifecycle then stage" ordering
   contract collapses; and `SUPPORTED_WIT_WORLDS`' doc comment ("Mirrors the world
   column of `WORLD_LIFECYCLE_EXPORTS`") is orphaned — the const itself retires in #3.

3. **`ExportKind::Lifecycle` is not a `dead_code` risk** (my premise, falsified during
   authoring). `ExportKind` is `pub` in a lib crate, and rustc's `dead_code` does not
   lint publicly-reachable items — an unconstructed variant would never trip
   `-D warnings`. **The real forcing function is different:** deleting
   `WORLD_LIFECYCLE_EXPORTS` breaks `SUPPORTED_WIT_WORLDS`' rustdoc intra-doc link
   `[WORLD_LIFECYCLE_EXPORTS]` (`crates/slicer-schema/src/lib.rs:354`), and *that*
   fails under `-D warnings`. **Decision:** delete the `Lifecycle` variant only; keep
   `ExportKind` (single `Stage` variant, still constructed) and `ExportBinding`,
   because packets #2/#3 restructure that surface into package+interface form and
   collapsing it now is churn that immediately re-churns.
4. **The stale-binary probe is copy-pasted three times, not once.**
   `slicer-runtime/tests/common/slicer_cache.rs`, `slicer-runtime/benches/gate_evidence.rs:48-60`
   (`harness = false`, cannot import `tests/common/` — and it produces DEV-026's
   50-layer time evidence, so a stale binary there silently invalidates governance
   evidence), and `slicer-scheduler/tests/integration/dag_cli_integration.rs:15-31`
   (whose panic advises `cargo build --workspace`, which does nothing to keep the
   binary fresh). **Decision:** packet #1 fixes all three in place. Extracting a
   shared locator is **deferred to its own packet** — it needs a home and an ADR:
   ADR-0004 places only *guest-side* test support in `slicer-sdk` (a crate that
   compiles into guest WASM), `slicer-test` was deleted by packet 78, and the
   candidate homes are `pnp-cli`'s lib behind a `test-support` feature (requires
   dev-dependency cycles from `slicer-runtime`/`slicer-scheduler`, which Cargo
   permits but which interact with `pnp-cli`'s existing `report =
   ["slicer-runtime/report"]` feature) or a new host-side test-support crate.
5. **`docs/03_wit_and_manifest.md:558-562` belongs to packet #1, not #3.** It quotes
   the WIT of both deleted exports under the comment `// Lifecycle — optional` — a
   line that is doubly false, since the component model has no optional exports (the
   premise of ADR-0045 itself). The rule "packet #3 owns docs/03" was wrong; the rule
   is "the packet that deletes a thing deletes its docs." Disjoint from #3's
   restructure of the same file's WIT listing.
6. **`xtask` is bin-only** — no `[lib]`, no `lib.rs`. `build_guests::is_stale` cannot
   be imported by `slicer-runtime`; packet #1 mirrors the algorithm and pins the
   source function so the two stay legible as siblings.

## Status since approval

- ADR-0045 **amended and accepted** (retitled "per-stage versioned **packages**";
  decision corrected to one package per stage; rejected-alternatives section added).
- `docs/15_config_keys_reference.md` fictional macro list + wrong stage name **fixed**.
- **DEV-085** filed for the 13 unimplemented custom-G-code injection points.
- `CONTEXT.md` "Stage interface" corrected (it claimed per-interface versioning,
  which WIT does not permit). "Module tier" / "Stage contract" are intentionally
  left until the code lands — see packet #3.
- The `object_id` parity fix **landed and is verified green**: `path_object_id` now
  keys on basename + index, so ids are identical across checkouts and G-code is
  byte-identical when the same model is sliced from two different absolute
  directories (the product bug). `perimeter_parity` 12/12, `legacy_zero_matches_golden`
  1/1, both independently re-verified. It was **8** tests, not 7 —
  `deliberate_broken_fixture_file_is_detected` was masked because `compare_perimeter_ir`
  stops at the first mismatch and `object_id` mismatched first. The `object_id`
  soft-ignore and its factually wrong comment are deleted, removing the asymmetry
  where the comparator hard-failed on `object_id` while one call site exempted it.
  `check_basename_collisions` ships **unwired by user decision** — no job accepts two
  model inputs today (`slice --model` is a single `PathBuf`), so a collision cannot
  occur; its rustdoc now says so explicitly, so no reader mistakes it for enforcement.

## Task mapping

TASK-144/145/146 (`docs/07_implementation_status.md:37-39`) are the governing slice.
TASK-146 ("Add host-side `wit_world` allowlist validation … reject mismatched
manifests at startup") is **reopened**: ADR-0044 showed the check compares one
hand-written string to another with no artifact to check against, and ADR-0045
retires `validate_wit_world` outright. Sub-lettered IDs follow the existing
convention (`TASK-119a/b/c`, `TASK-120a-d`, `TASK-194a/b`).

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 162_wit-lifecycle-export-removal | Delete the never-called `on-print-start`/`on-print-end` WIT exports, rename the SDK constructor to `from_config`, remove `WORLD_LIFECYCLE_EXPORTS` and its self-referential guard test, correct the lifecycle fiction in `docs/03`/`docs/04`/`docs/05`, and make CLI-binary staleness fail loudly at all three spawn sites. | TASK-146a | - | generated | `.ralph/specs/162_wit-lifecycle-export-removal/` |
| 2 | 163_per-stage-wit-packages-pilot | Build the per-stage versioned-package machinery and prove it on the two cheapest tiers — postpass (2 stages) and finalization (1) — at `@1.0.0` with fatal-on-miss load. | TASK-146b | #1 | pending | - |
| 3 | 164_per-stage-wit-packages-bulk | Migrate prepass (4) and layer (10) onto per-stage packages, retire `wit-world`/`SUPPORTED_WIT_WORLDS`/`validate_wit_world`, and correct `docs/03` and `CONTEXT.md` to the delivered contract. | TASK-146c | #2 | pending | - |
| 4 | 165_cli-binary-locator-extraction | Collapse the three copies of the `pnp_cli` binary locator + freshness assert into one shared home, with an ADR deciding that home (ADR-0004 covers only guest-side test support; `slicer-test` was deleted in p78). | TASK-146d | #1 | pending | - |

Dependency note: #2 must land the machinery before #3 migrates the remaining 14
stages. Between #2 and #3 two contract mechanisms are live in-tree — an accepted
intermediate, not an end state. #3 is not optional; leaving it undone reproduces
the exact failure this ADR exists to end (cf. `run.rs`'s 2026-05 "pragmatic fix",
still load-bearing 14 months later).

#4 is queued rather than merely promised for the same reason. It is a tidiness
packet — #1 already kills the staleness *bug* at all three sites — so it is the
kind of follow-up that historically evaporates. It carries a `TASK-146d` and a row
so that it cannot.

Commit the plan file and the packet directories together.

## Exports ledger

What each generated packet hands to its dependents. Consume these; do not re-derive
them.

### From #1 `162_wit-lifecycle-export-removal` (generated, PREFLIGHT PASS)

Net-new / changed:

- `slicer_sdk::traits::{LayerModule, PrepassModule, PostpassModule, FinalizationModule}::from_config(config: &ConfigView) -> Result<Self, ModuleError>` — required, no default body. The renamed per-call constructor; every `run_*` macro arm calls it once per stage invocation. Not a lifecycle hook.
- `slicer_schema::SlicerModuleSchema.exports: &'static [ExportBinding]` — now **≤1 entry**, always `ExportKind::Stage`; empty for a stageless impl. Packets #2/#3 restructure this into package+interface form.
- `slicer_schema::ExportKind` — survives with a single `Stage` variant. The `Lifecycle` variant is gone.
- `staleness_reason(Option<SystemTime>, SystemTime) -> Option<String>` — crate-local test helper in `crates/slicer-runtime/tests/common/`. Mirrors (does not import) `is_stale` from `xtask/src/build_guests.rs`, because `xtask` is **bin-only** and cannot be depended on.

Removed — do not cite these as existing:

- `slicer_schema::WORLD_LIFECYCLE_EXPORTS`, `slicer_schema::lifecycle_exports_for_world`, `ExportKind::Lifecycle`, `__SLICER_LIFECYCLE_EXPORT_COUNT`, and the vacuous test `every_world_has_lifecycle_exports`.
- WIT `on-print-start` / `on-print-end`. `world-layer` drops from 10 exports to **8**; the other three worlds are unchanged (they never declared lifecycle exports).

Deliberately untouched, still live for #3 to retire: `SUPPORTED_WIT_WORLDS` (doc comment only was corrected — its `[WORLD_LIFECYCLE_EXPORTS]` intra-doc link would otherwise dangle under `-D warnings`), `wit-world`, `validate_wit_world`.
