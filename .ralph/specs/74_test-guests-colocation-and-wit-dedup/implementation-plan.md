# Implementation Plan — Packet 74

## Execution Rules

- TDD/narrow validation only. After any edit to a guest `src/**`, guest `Cargo.toml`, or `crates/slicer-schema/wit/**`, run `cargo xtask build-guests --check` and rebuild if `STALE:` before running guest-consuming tests.
- Never run `cargo test --workspace` during steps; use the per-step narrow commands. The full suite runs once, at the Acceptance Ceremony, via a sub-agent returning `FACT pass/fail`.
- One logical concern per step; keep edited files ≤3 per step where possible.

---

### Step 1 — Remove the orphan guest directory
- **Task:** TASK-215. **Objective:** delete the stray empty `sdk-layer-plan-guest/` (no `Cargo.toml`, never built).
- **Precondition:** `test-guests/sdk-layer-plan-guest/` exists with only `Cargo.lock` + `target/`.
- **Postcondition:** directory gone; guest count is 12.
- **Read:** none. **Edit:** delete `test-guests/sdk-layer-plan-guest/`.
- **Dispatches:** none.
- **Context cost:** S.
- **Verify / exit:** `cargo xtask build-guests --list 2>/dev/null | grep -c "sdk-layer-plan-guest"` → `0`.

### Step 2 — Relocate the tree and repoint all references
- **Task:** TASK-215. **Objective:** move `test-guests/` under `crates/slicer-runtime/` and make everything build green at the new location (no D1/A/C yet).
- **Precondition:** Step 1 done; tree builds at old location.
- **Postcondition:** `crates/slicer-runtime/test-guests/` holds all 12 guests; the old repo-root `test-guests/` directory no longer exists (including any `git mv`-orphaned untracked `target/`); builder + tests (all 4 path forms) + gitignore + docs repointed; per-guest `[workspace]` sentinels untouched.
- **Read:** `xtask/src/build_guests.rs:88–259`; `.gitignore`; the SDK guest manifests (12 small files).
- **Edit (logical groups):** (a) `git mv test-guests crates/slicer-runtime/test-guests`, **then remove any residual old `test-guests/` directory left behind by `git mv`** — `git mv` moves only tracked files, so gitignored `target/` and untracked files stay at the old root; delete the stub so `test-guests/` no longer exists at the repo root; (b) SDK guest manifests `../../crates/slicer-X` → `../../../slicer-X`; (c) `xtask/src/build_guests.rs` `tg_root` (:175) + artifact prefix (:242); (d) repoint `crates/slicer-runtime/tests/*.rs` across **all four path forms** — a literal `/../../test-guests/` → `/test-guests/` sweep fixes only the 13 Form-1 files (drop **all** `..`: `CARGO_MANIFEST_DIR=crates/slicer-runtime` and the new tree is directly under it, so `/../test-guests/` would wrongly resolve to `crates/test-guests/`; AC-N1's static guard cannot see this single-`..` slip — only the runtime `fs::read` tests do); also hand-edit the 5 non-literal files: drop one `.join("..")` in `guest_fixture_freshness_tdd.rs` (`test_guests_dir`), `macro_all_worlds_roundtrip_tdd.rs` (`guest_component_path`), `macro_finalization_deep_copy_tdd.rs`; re-base the `.parent()` chain in `live_layer_support_tdd.rs:869`; and (the `wit_drift_detection_tdd.rs:629` `format!` helper is handled in Step 4's deletion — repoint only if retained); (e) `.gitignore` path; (f) `CLAUDE.md` + `docs/05_module_sdk.md` + two `skills/**/wasm-staleness.md`.
- **Dispatches:** "Run the 4-form multiline guard `rg -Ul -e '\.\./\.\./test-guests' -e '\.join\("\.\."\)\s*\.join\("\.\."\)\s*\.join\("test-guests"\)' -e 'join\("test-guests/' -e 'format!\("test-guests/' crates/slicer-runtime/tests | wc -l` → FACT count (expect 0). Do **not** use the old single-line literal grep — it false-greens on Forms 2–4."
- **Context cost:** M.
- **Verify / exit:** AC-N1 4-form guard (above) → `0`; AC-1 list grep → `12`; `cargo xtask build-guests` builds all; the old `test-guests/` repo-root dir is gone (`test -d test-guests; echo $?` → `1`); and a representative from each non-literal form passes against the real (now-moved) artifacts: `cargo test -p slicer-runtime --test guest_fixture_freshness_tdd --test macro_all_worlds_roundtrip_tdd --test live_layer_support_tdd` (these load artifacts at runtime, so a mis-repointed path fails on `std::fs::read` of the deleted old location — the decisive catch the static guard backstops).

### Step 3 — D1: single shared guest target directory
- **Task:** TASK-215. **Objective:** all guests build into one `CARGO_TARGET_DIR`; per-guest `[workspace]` retained.
- **Precondition:** Step 2 green.
- **Postcondition:** exactly one `target/` under the relocated tree; `discover_guests` validation unchanged.
- **Read:** `xtask/src/build_guests.rs:345–413` (build_one; intermediate-path computation at `:372–376`).
- **Edit:** `xtask/src/build_guests.rs` (set shared `CARGO_TARGET_DIR` env on the guest `cargo build`; update intermediate `.wasm` lookup); `.gitignore` (single target dir).
- **Dispatches:** "SNIPPET (≤30 lines, file:line) of build_one's intermediate-`.wasm` path computation."
- **Context cost:** M.
- **Verify / exit:** AC-2 `cargo xtask build-guests >/dev/null 2>&1; find crates/slicer-runtime/test-guests -maxdepth 2 -type d -name target | wc -l` → `1`; `cargo xtask build-guests --check` → no `STALE:`.

### Step 4 — A: raw guests bind canonical WIT
- **Task:** TASK-215. **Objective:** remove inline WIT from the four raw guests; bind `crates/slicer-schema/wit/`; update drift coverage.
- **Precondition:** Steps 2–3 green.
- **Postcondition:** no `inline:` in the four guests; each uses `path: "../../../slicer-schema/wit"`; obsolete drift sub-test removed; boundary/drift tests pass.
- **Read:** one raw guest (`prepass-guest/src/lib.rs`) + `wit_host.rs:241`/`:314` for the canonical `path:` form; `wit_drift_detection_tdd.rs:436–486`.
- **Edit:** the four raw guests' `generate!` blocks; delete `handwritten_test_guests_use_payload_extrusion_role_variants` (`:436–486`) **and** its now-unused `test_guest_lib_rs_content` helper (`:628`, sole caller was `:464`) — this also retires the only Form-4 `format!("test-guests/…")` reference.
- **Dispatches:** "Run `cargo test -p slicer-runtime --test wit_boundary_tdd --test wit_drift_detection_tdd` → FACT pass/fail + first failing assertion."
- **Context cost:** M. (Rebuild guests after edit — staleness rule.)
- **Verify / exit:** AC-3 compound grep → exit `0`; AC-4 grep `0` + `cargo test -p slicer-runtime --test wit_drift_detection_tdd` pass; AC-N2 `cargo test -p slicer-runtime --test wit_boundary_tdd` pass; AC-5 four guests still present.

### Step 5 — C: witness codec extraction (SDK side)
- **Task:** TASK-215. **Objective:** one owning module for the positional witness encoding; producer + consumer both use it.
- **Precondition:** Steps 2–4 green.
- **Postcondition:** `crates/slicer-runtime/test-guests/witness/` exists with `encode`/`decode`; `sdk-layer-infill-guest` + `sdk-finalization-guest` encode via it; the 5 named host decoders decode via it; `slicer-runtime` dev-deps on `witness`.
- **Read:** `sdk-layer-infill-guest/src/lib.rs`, `sdk-finalization-guest/src/lib.rs`; the witness-decoding windows in the 5 host test files (±40 lines around `points[0]`/`flow_factor` sites).
- **Edit (batched):** new `witness/` crate (`Cargo.toml` + `src/lib.rs`); 2 SDK guest producers; `slicer-runtime` `Cargo.toml` dev-dep; the 5 host decoders. (Exceeds 3 files by necessity — split into sub-commits: crate+guests, then host decoders.)
- **Dispatches:** "Run `cargo test -p slicer-runtime --test dispatch_tdd --test finalization_world_deep_copy_tdd --test macro_finalization_deep_copy_tdd` → FACT pass/fail."
- **Context cost:** M. (Rebuild guests after producer edits.)
- **Verify / exit:** AC-6 `test -f .../witness/src/lib.rs && grep -lq "witness::" .../dispatch_tdd.rs` exit `0` + `cargo test -p slicer-runtime --test dispatch_tdd` pass. **Witness workspace-membership guard (design.md Data and Contract Notes):** the nested sentinel-less `witness` crate must not be auto-captured by the root workspace and must build both directions — `cargo check -p witness` (host) and `cargo check -p witness --target wasm32-unknown-unknown` (guest) both clean; if the root `members` glob captures it unintentionally, add an `exclude` or a `[workspace]` sentinel.

---

## Per-Step Budget Roll-Up

| Step | Concern | Cost |
|------|---------|------|
| 1 | orphan deletion | S |
| 2 | relocate + repoint | M |
| 3 | D1 shared target dir | M |
| 4 | A de-dup canonical WIT | M |
| 5 | C witness codec | M |

Aggregate: **M** (no step is L).

## Packet Completion Gate

1. `cargo xtask build-guests --check` → no `STALE:` (rebuild if needed).
2. AC-1..AC-7 and AC-N1..AC-N2 commands all pass.
3. `cargo clippy --workspace --all-targets -- -D warnings` clean.
4. `cargo check --workspace --all-targets` clean.

## Acceptance Ceremony

After the gate is green, dispatch a sub-agent to run `cargo test --workspace` and return only `FACT: pass/fail` (+ first failing test name if any). Do not absorb the full output. Only then mark TASK-215 closed in `docs/07_implementation_status.md` with the per-AC evidence summary, and flip `packet.spec.md` `status:` to a closed state per repo convention.
