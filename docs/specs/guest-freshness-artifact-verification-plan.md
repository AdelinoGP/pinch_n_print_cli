# Guest Freshness Artifact Verification — Batch Plan

Status: approved (user, 2026-08-19, grilling + 4 adversarial review rounds)
Generator: spec-packet-generator Batch Protocol
Commit rule: this plan file and the `docs/spec_packets/` packet directories it queues
must be committed together.

## Problem (measured)

`cargo xtask build-guests --check` answers "did any tracked input change?" with a
conservative union of fingerprints, and that union has grown with every WIT
restructure:

- Packet 70: shared set = all of `crates/slicer-schema/wit/**` + `slicer-{macros,sdk,ir,schema}` src.
- Packets 163/164 (per-stage WIT packages): per-stage dirs added for core guests;
  **test guests charged the union of all 15 stage dirs**.
- Packet 185: `slicer-core` added as a "universal guest dependency" — its entire
  `src/` (66 files) charged to every guest.

Today the shared fingerprint set is 113 files (`root.wit` + 5 flat `deps/*.wit`
+ 107 crate files/manifests; measured 2026-08-19). **Any single-byte edit to any
of them marks all 42 guests (21 core + 21 test) `STALE`.** `cargo xtask test` then rebuilds every guest; the
first guest that fails to compile against the new WIT aborts the whole suite
with `error: cargo build failed for '<guest>'` — so a WIT change made by one
agent looks like another agent's work broke tests. The project is in early
development; WIT churn is frequent; the checker has become the loudest source of
misattributed breakage.

Meanwhile the *semantic* answer already exists: `xtask/src/wit_verify.rs`
compares a built artifact's embedded WIT world against canonical — but only
runs **after** a build, never in `--check` (`verify_embedded_world` is called
only from `build_one`). All 42 artifacts are present on disk. Decode cost is
**unmeasured** on this machine; packet 230 must record before/after wall-clock
for `--check` rather than rely on the ~38ms/~2s figures asserted earlier.

## Locked decisions (user-ruled)

**Round 5 amended C1, C3, C4, C5, C11, C13 — see "Round 5" below. Where the
text of a decision below conflicts with a Round 5 amendment, the amendment
wins.**

1. **C1 — Output-based WIT staleness.** `--check` decodes each existing artifact
   and compares its embedded world against canonical. The fingerprint shrinks to
   code inputs only. WIT changes mark stale **only the guests whose embedded
   world disagrees** — a guest whose world already matches canonical would
   rebuild byte-identical, so output-checking is exact, not conservative.
2. **C2 — Dependency-closure fingerprint.** The hardcoded shared-crate list is
   replaced by a per-guest closure walk over manifest path deps. Plain
   wit-bindgen test guests (which link nothing) get an empty closure; SDK and
   core guests get their real chain.
3. **C3 — Full declaration model.** The comparison covers braced types
   (variant/enum/record/flags), aliases (`type X = ...;`), resources
   (`resource X { ... }` / `resource X;`), interface members (funcs + resource
   methods), and `use` declarations — package-qualified, both statement-form and
   braced-form packages. Stage package: **full equality both directions on the
   exported interface**, PLUS **subset direction on every other interface of the
   resolved stage package** (amended by Round 5 finding R5-1: local `*-types`
   interfaces are NOT pruned). Shared packages:
   **subset direction** (embedded ⊆ canonical; use lists as sets). Any embedded
   package outside {`root:component`, the 5 shared packages, the resolved stage
   package} → STALE (fail-closed). Export name compared exactly incl. version.
4. **Artifact-based stage resolution.** The embedded package name (version-
   stripped) resolves the stage via `STAGES`; zero/multiple/unresolvable →
   STALE. **Amended by Round 5 finding R5-4:** for core guests the module
   manifest's `[stage] id` is retained as an INDEPENDENT expectation and the
   artifact's resolved stage MUST equal it (mismatch → STALE); artifact-derived
   resolution is the sole resolver only for test guests, which have no module
   manifest. `GuestSpec.stage_id` and `parse_stage_id_from_module_manifest`
   therefore SURVIVE; `module_stage_wit_dir` retires in favour of resolving the
   wit dir from `STAGES`. This also answers the never-built-guest case, where no
   artifact exists to resolve from.
5. **Fingerprint lifecycle.** Written ONLY after final verification succeeds;
   sidecar removed at build start and on every persistent failure (decode
   failure, `StaleEmbeddedWorld`). Version `v2-`; `wasm-tools --version` string
   included (a wasm-tools upgrade forces exactly one clean rebuild). Content:
   guest src + manifest + parent module src + the sibling **module manifest**
   `<module>/*.toml` (core guests; charged because `[stage] id` feeds the R5-4
   cross-check and `[config.schema.*]` feeds `ConfigView::from_declared` - NOT
   because anything `include_str!`s it, and NOT `Cargo.toml`; see R5-13) + closure crates (src + Cargo.toml +
   build.rs) + **workspace-root `Cargo.toml`, the guest's own `Cargo.lock`, and
   the `rustc -vV` string** (amended by Round 5 finding R5-2).
6. **Stale-only rebuild.** `check_command` returns `{ stale: Vec<GuestSpec>,
   code: i32 }`; `test_command` builds ONLY the stale specs
   (`build_stale_command`). Full `build_command` stays for explicit invocations
   (dist, CI). `main.rs --check` keeps print + exit semantics; `dist.rs`
   untouched.
7. **pnp_cli freshness = always `cargo build --bin pnp_cli`.** The hand-rolled
   host mtime set is deleted (it missed per-stage WIT, host crates, optional
   deps, and include_str! assets — four distinct holes). Cargo's own
   fingerprinting is authoritative; the no-op cost is **unmeasured** and packet
   231 must measure it. `compute_shared_freshness` deleted.
8. **Closure walk rule.** ALL path deps including `optional = true` (the
   over-approximation converges: the content-hash fingerprint is rewritten after
   each successful build, so a no-op rebuild still converges). Resolve relative
   to containing manifest; canonicalize; recurse; dedupe; cycle-guard; one
   invocation-wide cache keyed by canonical manifest path.
9. **wasm-tools missing → distinct infrastructure error** (not staleness);
   `test_command` aborts WITHOUT attempting rebuild.
10. **Rebuild failure → keep aborting the suite** (scoped to genuinely affected
    guests now). Undecodable artifact → STALE (rebuild fixes or hard-errors).
11. **Output contract.** Exactly one `STALE: <name>` line per stale guest; drift
    reason on a second line that NEVER contains `STALE:`; exit 1 if any stale.
    **Amended by Round 5 finding R5-3:** freshness is asserted by EXIT CODE, not
    by grepping for `STALE:` — a `wasm-tools`-missing infrastructure error
    contains no `STALE:` and would otherwise read as PASS. Packet 232 rewrites
    the canonical snippet accordingly. Existing packets are NOT edited
    (user-ruled 2026-08-19).
12. **Canonical coverage audit.** The verifier's canonical file list must equal
    the macro's actual include_str! set (exactly 20 WIT files: 5 flat + 15
    stage; `root.wit` is NOT embedded by the macro and must not be in the list),
    and `slicer-macros/build.rs` rerun-if-changed must watch that same set
    (currently it watches deleted `world-*.wit` paths). Multiline-aware parse.
13. **xtask dependency policy.** **Amended by Round 5 finding R5-5.** xtask
    already declares `walkdir`, `toml`, `syn`, `proc-macro2`, `slicer-schema`.
    Packet 229 ADDS `wit-parser = "0.247"` — version-matched to the existing
    direct declarations in `crates/slicer-runtime/Cargo.toml` and
    `crates/slicer-wasm-host/Cargo.toml`, already resolved in `Cargo.lock`, so
    no new version enters the graph. **Both** the canonical `.wit` side and the
    decoded `wasm-tools component wit` side are parsed with `wit_parser`
    (user-ruled); no hand-rolled WIT parsing survives. Manifests use
    `toml::Table`, as `has_cdylib`/`has_parent_path_dep` already do. ADR-0014 is
    about guest discovery via a validated filesystem walk and does NOT govern
    this; the earlier citation of it was wrong.

## Review history (4 adversarial rounds; findings folded in — do not re-open)

- Round 1 (senior-coder + mid-coder): prepass-types.wit missing from canonical
  set; normalize gaps (`,)` vs `)`, statement reorder); stale-macro chain never
  converges for interface-only drift; version bumps invisible; no-match fallback
  not conservative; imported-interface pruning; optional-dep closure hole;
  pnp_cli per-stage WIT hole; rebuild-all at the rebuild step; bare-name type
  keying.
- Round 2 (mid-coder + reviewer): fingerprint written before verification;
  include_str! assets untracked; check_command API migration; test-guest stage
  resolution; alias/resource drift invisible; pnp_cli default-features optional
  deps; recursive member extraction; package-qualified coverage.
- Round 3 (mid-coder + reviewer): full-equality must scope to the exported
  interface ONLY (local `*-types` interfaces are pruned — verified on real
  prepass/finalization artifacts); `use` declarations must be compared (nominal
  type-identity drift); allowed-package-set fail-closed; statement-form vs
  braced-form package parsing; closure cache owner; coverage audit baseline
  (20 files, not root.wit); docs deletion surface (docs/07:51, ADR-0045:165-175,
  docs/spec_packets/205 design.md:7).
- Round 4 (mid-coder + reviewer): no new design findings — verdicts were
  "not implemented yet" (category error; the plan is a design document). The
  one empirical check CONFIRMED the v4 model: decoded artifacts contain no
  unexpected packages, exports match STAGES, and prepass/finalization local
  `*-types` pruning behaves exactly as the subset-direction rule predicts.


## Round 5 (spec-packet-generator verification + 2 adversarial subagents, 2026-08-19)

Grounded against the tree with `wasm-tools 1.250.0` and decoded artifacts. Unlike
Round 4, this round produced blocking findings. All rulings below are user-made.

### Falsified factual claims (corrected in place above)

- **41 guests (21 core + 20 test)** -> **42** (21 core + 21 test). `discover_guests`
  globs both trees; `test-guests/witness/` is skipped (no cdylib, no wit-bindgen).
- **"22 present artifacts"** -> **all 42 present**. Nothing is missing on disk.
- **"~112 files"** -> **113** (`root.wit` + 5 flat `deps/*.wit` + 107 crate files).
  `input_files` is extension-unfiltered, so non-`.rs` files under those `src/` dirs count.
- **ADR-0014** is *"`xtask` Guest Discovery Uses a Validated Filesystem Walk, Not
  `cargo_metadata`"* - NOT "no new xtask deps". The C13 citation was wrong.
- **ADR-0045** is *"Per-stage versioned packages over monolithic tier worlds"* - the
  C4 gloss ("typed instantiation is the real wrong-stage guard") is not its subject.
- **`docs/07:51`** is the TASK-146b row, not a standalone staleness claim; **`docs/11`
  contains no `build-guests` mention at all** and is dropped from the packet-232 surface.
  The real normative surface is `docs/03_wit_and_manifest.md` section "Build & Freshness
  Contract (Normative)" (states "mtime-based" and "exit 1 if any source is newer than
  its artifact") plus its staleness-guard table row, and `docs/05_module_sdk.md`
  ("the canonical pre-test gate").
- The plan's own pins cite `compute_shared_mtime` (ADR-0045) and `stage_wit_mtime`
  (docs/07) - **neither symbol exists**; they are now `compute_shared_freshness` and
  `stage_wit_snapshot`. Packet 232 repins by symbol name (CLAUDE.md In-Tree Citation Style).

### Blocking findings (folded in; user-ruled 2026-08-19)

- **R5-1 (BLOCKER, amends C3).** Round 3's premise is **factually wrong**: local
  `*-types` interfaces are NOT pruned from embedded output. Decoding
  `crates/slicer-wasm-host/test-guests/finalization-mutation-roundtrip-guest.component.wasm`
  shows `layer-finalization-types` carrying `resource layer-collection-view`,
  `record region-key`, `record print-entity-view`, `type layer-idx`;
  `modules/core-modules/wipe-tower/wipe-tower.wasm` additionally shows all seven
  `finalization-output-builder` methods. Members are pruned by usage; the
  *interface* is not. Under unamended C3 that entire IR surface went unverified -
  the exact failure class the plan exists to catch. **Fix:** subset direction applies
  to every non-exported interface of the resolved stage package.
- **R5-2 (BLOCKER, amends C5).** The v2 fingerprint omitted workspace-root
  `Cargo.toml` (which pins `wit-bindgen = "0.60.0"`, consumed as
  `wit-bindgen.workspace = true`), the per-guest `Cargo.lock`, and the rustc version.
  A `cargo update` or toolchain bump changes emitted bindings and the component-type
  encoding with byte-identical WIT -> FRESH over a genuinely stale guest. **Fix:** add
  all three to the v2 fingerprint.
- **R5-3 (BLOCKER, amends C11).** Grep-based freshness ACs turn the C9 infrastructure
  error into a false PASS: `--check 2>&1 | rg -q 'STALE:' && echo FAIL || echo PASS`
  reports PASS when `wasm-tools` is missing. **Fix:** exit-code-based contract; packet
  232 rewrites `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md`.
  Existing packets that use the grep form are left untouched (user-ruled).
- **R5-4 (MAJOR, amends C4).** Retiring manifest-derived stage resolution makes the
  check self-referential - the artifact declares its own stage AND is judged against
  it, so a guest exporting the wrong stage compares equal and reports FRESH.
  `module_stage_wit_dir`'s own doc comment records this regression from packet 164.
  **Fix:** keep `[stage] id` as an independent expectation for core guests.
- **R5-5 (MAJOR, amends C13).** `wit-parser` is not a new dependency: `wit-parser =
  "0.247"` is already declared directly by `crates/slicer-runtime` and
  `crates/slicer-wasm-host`, and `crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs`
  already resolves the canonical WIT dir with `wit_parser::Resolve`. **Fix:** parse
  both sides with `wit_parser`; delete the hand-rolled scanner.

### Non-blocking findings folded into packet scope

- **R5-6.** Closure walk must enumerate `dependencies`, `target.*.dependencies`, and
  `build-dependencies`, and explicitly EXCLUDE `dev-dependencies`. `crates/slicer-sdk/Cargo.toml`
  declares `slicer-core` under a `cfg(not(target_arch = "wasm32"))` table and
  `modules/core-modules/classic-perimeters/Cargo.toml` declares `wit-bindgen` only under
  a `cfg(target_arch = "wasm32")` table; the obvious template `has_parent_path_dep`
  reads only `[dependencies]`, which would silently drop whole subtrees. -> packet 231.
- **R5-7.** Surviving fail-open paths the plan never retired: `build_one`'s
  `if canonical.is_empty() { return Ok(()) }`, `verify_embedded_world`'s
  compare-only-names-present-in-both, and `canonical_type_blocks`' swallowing of
  unreadable canonical files. An empty/unreadable canonical set is an infrastructure
  error, not a pass. -> packets 229/230.
- **R5-8.** Record-field and variant-case ORDER is ABI-relevant; normalization may sort
  interface-level statements but MUST NOT reorder declaration bodies. -> packet 229 AC.
- **R5-9.** `crates/pnp-cli-locator::staleness_reason` is a documented mirror of
  `is_stale`, consumed by `crates/slicer-runtime/tests/common/slicer_cache.rs` and
  `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs`. C7 deletes the
  model it mirrors. -> packet 231 (user-ruled: fold into 231, no 5th packet).
- **R5-10.** CI's `test` job runs `cargo test -p slicer-runtime && -p pnp-cli &&
  -p slicer-helpers` - **never `-p xtask`**, so every verifier test is dead in CI. The
  real-artifact tests also `eprintln!("skipping...")` and return when an artifact or
  `wasm-tools` is absent, so they would be vacuously green. -> packet 232 (add
  `cargo test -p xtask` to the CI test job; make real-artifact tests fail, not skip).
- **R5-13 (HIGH, further amends C5).** C5's stated justification for charging the
  sibling module manifest - "core guests; the parent `include_str!`s it" - is
  **false twice over**, measured 2026-08-19. First, the file named is
  `<module>/Cargo.toml`, but nothing anywhere embeds a `Cargo.toml`. Second, the
  only `include_str!` of a module `.toml` under `modules/` is
  `include_str!("../classic-perimeters.toml")` in
  `modules/core-modules/classic-perimeters/src/lib.rs`, and it sits **inside that
  file's `#[cfg(test)] mod tests`** - so it never reaches the guest `.wasm` at all.
  The module `.toml` must still be charged, but for a different and real reason:
  `parse_stage_id_from_module_manifest` derives `GuestSpec.stage_id` from its
  `[stage] id`, which R5-4 makes the independent cross-check against the artifact's
  resolved stage, and its `[config.schema.*]` tables drive the host's
  `ConfigView::from_declared` filter (`crates/slicer-wasm-host/src/host.rs`,
  `crates/slicer-wasm-host/src/dispatch.rs`). **Fix:** charge every `*.toml`
  directly under the parent module dir (one per core module), with that rationale
  rather than the `include_str!` one. The resulting rebuild is conservative - the
  `.wasm` is byte-identical - and converges by the same argument as C8.
  -> packet 231.
- **R5-11.** Latent bug in `extract_type_blocks`: `open` is a BYTE index into
  `stripped` but `matching_brace(&bytes, open)` indexes a `Vec<char>`. Moot once
  `wit_parser` replaces the scanner (R5-5), but must not be reintroduced.
- **R5-12.** `docs/03`'s freshness contract also names a second, host-side gate -
  `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` - which
  hardcodes 10 test guests and its own mtime rule. It is independent of xtask and
  keeps working, but packet 232 must reconcile the doc text that presents both as one
  model. `build_script_check_mode_reports_freshness` in that file is vacuous (returns
  early when `build-test-guests.sh` is absent, which it is).

### Claims that survived adversarial attack

- The fail-closed allowed-package set is exactly right: decoding all 21 test-guest
  artifacts yields no package outside `root:component`, the 5 shared packages
  (`slicer:types`, `slicer:config`, `slicer:ir-handles`, `slicer:common`,
  `slicer:prepass-types`), and one stage package, with exactly one export each.
- Subset direction is sound for *used* members: types are never internally pruned
  (embedded `extrusion-role` carries all 14 cases, `config-value` all 8); only whole
  members are dropped, so any declaration a guest actually uses appears in full.
- C1 holds for pure additions to imported interfaces: `classic-perimeters.wasm` embeds
  `config-view` with only `get`/`keys` of the 6 canonical methods, and an unused added
  method genuinely cannot change the binary.
- C8's convergence argument for optional deps holds.
- `STAGES` refinement: it has **16** rows, not 15 - `PrePass::PaintSegmentation` has
  `wit_package: ""` (host-built-in) and MUST be excluded from artifact-based resolution.
- CI is safe for C9: `.github/workflows/ci.yml` installs `wasm-tools` via
  `taiki-e/install-action` in both the `test` and `dist-editions` jobs, before
  `cargo xtask build-guests --check`.

## Packet Queue

Task IDs are allocated from the 2026-08-19 maximum `TASK-339`; each packet adds its
own row to `docs/07_implementation_status.md` under "Workstream 5 - Governance and
closure drift". Authoring agents MUST re-derive the maximum at write time and
renumber on collision (see "Numbering note").

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 229-wit-verify-declaration-model | Rebuild `wit_verify.rs` on `wit_parser` (both canonical and decoded sides), implementing the amended declaration model: package-qualified types/aliases/resources/interface-members/`use` decls; stage package = full equality on the exported interface PLUS subset direction on every other interface of that package (R5-1); shared packages = subset; fail-closed on unexpected packages and on an empty/unreadable canonical set (R5-7); order-preserving bodies (R5-8); canonical coverage audit against the macro's 20 `include_str!` files plus the `slicer-macros/build.rs` rerun-if-changed fix. | TASK-340 | - | implemented | docs/spec_packets/229-wit-verify-declaration-model |
| 2 | 230-output-based-guest-freshness | Wire the verifier into `--check`: stage resolution from the artifact cross-checked against the core guest's manifest `[stage] id` (R5-4), `STAGES` row with empty `wit_package` excluded, zero/multiple/unresolvable -> STALE, `wasm-tools`-missing as a distinct infrastructure error with exit-code-based reporting (R5-3); `check_command` returns the stale list; add `build_stale_command`; `test_command` rebuilds only stale guests; `build_one` writes the v2 fingerprint only after final verification, including workspace-root `Cargo.toml`, the guest `Cargo.lock`, `rustc -vV`, and the `wasm-tools` version (R5-2). | TASK-341 | #1 | implemented | docs/spec_packets/230-output-based-guest-freshness |
| 3 | 231-guest-closure-fingerprint | Replace the hardcoded shared-crate list with a per-guest dependency-closure walk over `dependencies`, `target.*.dependencies`, and `build-dependencies` but never `dev-dependencies` (R5-6), canonicalized, cycle-guarded, invocation-cached; delete `compute_shared_freshness` and `stage_wit_snapshot`; make pnp_cli freshness an unconditional `cargo build --bin pnp_cli`; and reconcile `crates/pnp-cli-locator::staleness_reason`, which mirrors the deleted model (R5-9). | TASK-342 | #2 | implemented | docs/spec_packets/231-guest-closure-fingerprint |
| 4 | 232-freshness-gate-docs | Update the freshness contract everywhere it is stated: CLAUDE.md "Guest WASM Staleness", `docs/03_wit_and_manifest.md` "Build & Freshness Contract (Normative)" and its staleness-guard table row, `docs/05_module_sdk.md`, `docs/07` (repinned by symbol, correcting `stage_wit_mtime`), ADR-0014 and ADR-0045 amendments (repinning `compute_shared_mtime`), the `wasm-staleness` snippet rewritten to an exit-code contract (R5-3), `spec-review` SKILL.md, `cargo test -p xtask` added to the CI test job with real-artifact tests failing rather than skipping (R5-10), and the CONTEXT.md term below. | TASK-343 | #2, #3 | implemented | docs/spec_packets/232-freshness-gate-docs |

Out of scope for every packet (user-ruled 2026-08-19): editing any other packet
directory, including the six whose ACs use the grep form of the freshness check
(`206`, `207`, `209`, `210a`, `210b`, `211`, `212`).

## CONTEXT.md term (packet 4 owns the wording)

**Artifact-verified freshness** — the property that a guest artifact's embedded
WIT world matches the canonical WIT for its stage, established by decoding the
artifact (`wasm-tools component wit`) and comparing declarations
package-qualified, rather than by fingerprinting WIT input files. The
fingerprint covers code inputs only; WIT staleness is answered by the artifact
itself.

## Numbering note (ledger facts)

Packet numbers 229–232, TASK ids, and the docs page number were derived
2026-08-19 from on-disk maxima (highest `docs/spec_packets/` number = 228,
highest TASK in docs/07 = TASK-339, highest docs page = 21). Authoring agents
MUST re-derive all three at write time (highest existing `docs/spec_packets/`
number in git history, `rg -o 'TASK-[0-9]{3}' docs/07_implementation_status.md |
sort -u | tail -1`, highest `docs/NN_*.md`) and renumber on collision.
