---
status: implemented
packet: 199-literal-gate-enforcement
task_ids:
  - TASK-321
---

# 199-literal-gate-enforcement

## Goal

Flip the struct-literal churn gate to enforced: wire `cargo xtask check-literals` (workspace-wide enforce mode) into `cargo xtask test`'s preflight ahead of the guest-freshness gate, promote the command into CLAUDE.md's required-before-commit set and enforced-state wording (CLAUDE.md rule section, CLAUDE.md gated-entry-point section, `docs/21_data_defaults_and_fixtures.md`), repair CLAUDE.md §"Feature-gated test files" (stale slicer-gcode/host-algos claim; new slicer-sdk `--features test` hazard), and convert the no-sweep-packet residue (slicer-model-io, slicer-helpers, slicer-macros) so the workspace-wide gate exits 0.

## Problem Statement

Packets 194–198 built the struct-literal churn gate and swept the nine high-traffic areas green, but the gate is still advisory: `cargo xtask test` does not run it, CLAUDE.md marks the rule `not yet a required gate`, and five workspace crates were in no sweep packet's scope. Until enforcement flips on, the failure the plan measured (`docs/specs/_OLD/struct-literal-churn-gate-plan.md`: `a579fc18`'s 165-file filler sweep, and the prior TASK-200a–e fix that landed completely yet decayed because "it produced no ongoing rule") recurs with the next added field. This packet is the plan's locked decision 4 (wiring, last, only after sweeps are green) plus the accumulated obligations assigned to the queue-final row: residue conversion, the CLAUDE.md stale-fact repair for the slicer-gcode/host-algos claim (which caused a real authoring error in packet 196), the slicer-sdk `--features test` hazard addendum, and the whole-plan closing gates.

## Architecture Constraints

- **Preflight order and scope (locked):** the check-literals preflight runs workspace-wide in enforce mode regardless of `-p`/filter args passed to `cargo xtask test`, and runs BEFORE `build_guests::check_command` — a pure-syntax scan must abort the run before the (potentially slow, possibly guest-rebuilding) freshness gate spends work on a red tree. The `--summary-from` path stays gate-free (no test run = no gate), mirroring the existing guest-freshness exemption.
- **Guest-WASM staleness — snippet intentionally omitted, with both facts stated:** the grounded change surface (xtask sources, CLAUDE.md, docs/21, model-io tests + `src/loader.rs` cfg-test mod, helpers tests, macros `tests/slicer_module_tdd.rs`) touches NO guest-fingerprinted path. The fingerprint code (`shared_input_paths`, `xtask/src/build_guests.rs`) collects, per shared crate {slicer-macros, slicer-sdk, slicer-ir, slicer-schema, slicer-core}: `src/` files, `Cargo.toml`, and `build.rs` — NOT `tests/**`. CLAUDE.md's prose trigger list is broader (`crates/slicer-macros/**`), but the code wins: the macros edit is confined to `tests/`, so no guest goes stale. If implementation drift ever pushes an edit into `crates/slicer-macros/src/**`, `crates/slicer-ir/src/**`, or any shared crate's `src/`/`Cargo.toml`/`build.rs`, the implementer MUST add `cargo xtask build-guests --check` to that step's verification and rebuild on `STALE:` before interpreting failures. AC-N2/AC-N3's probe file lives under `crates/slicer-ir/tests/` — scanned by check-literals, invisible to both cargo and the guest fingerprint.
- **Exit-code narrowing:** xtask's `main` narrows `i32` to `ExitCode`'s `u8`; the preflight abort returns 1 explicitly (never a propagated platform status), the same rule `ensure_pnp_cli_fresh_with`'s comment documents.
- **No committed red fixtures:** all violation fixtures are either in-memory/temp-dir (unit tests, cleaned up by the test) or shell-temp files created and removed inside a single AC command. Nothing red is ever committed.

## Data and Contract Notes

- IR/manifest contracts: untouched. `ObjectMesh` shape unchanged; only construction syntax at test sites.
- WIT boundary: untouched.
- Determinism/scheduler constraints: none; the preflight is a read-only scan.
- CLI contract consumed, not defined: exit 0/1/2, violation-line and summary formats are packet-194 exports; this packet's AC-N2 greps rely on the violation-line fragment `exhaustive literal of watched type`.

## Locked Assumptions and Invariants

- Preflight runs workspace-wide enforce, before the guest-freshness gate, exempt only in `--summary-from` mode; abort exit code is 1; failure line contains `check-literals preflight failed`.
- Residue conversion never adds `Default`, never changes an assertion, never renames the macros mocks.
- The probe path `crates/slicer-ir/tests/data/gate_probe_199_tmp.rs` is transient AC-command state only; it must never be committed (a leftover fails AC-1 loudly, which is the desired failure mode).
- `enforced since packet 199` is the canonical replacement anchor in both CLAUDE.md and docs/21 (greps depend on the exact phrase).

## Risks and Tradeoffs

- 194's scan API shape is unknown until implemented; mitigated by the [FWD] dispatch and the sanctioned thin-wrapper fallback.
- The grounded residue inventory disagrees with an earlier review note (which said model-io 1 file / helpers 0); this packet's own scan (2026-08-07) found more files, and the tool did not exist to arbitrate. Mitigated: Step 1's `--report` re-derivation is authoritative; the inventory here is a navigation hint, not a count contract, and no AC freezes a count.
- The CLAUDE.md §Feature-gated repair may be partially pre-applied (uncommitted working-tree fix observed 2026-08-07); steps verify end-state, so a prior commit of that fix converts the step into a no-op-plus-verify, not a conflict.
- Waivers on the helpers/loader constructor helpers trade strict FRU purity for zero structural churn; the reason strings make the intent auditable, and the waiver audit (Step 6) counts them.
- `cargo xtask test`'s auto-`--features slicer-core/host-algos` makes the AC-N3 false-negative path error out fast (xtask has no slicer-core dep) — bounded, but the error text must not contain the preflight failure line; the grep is specific enough (`check-literals preflight failed`).
