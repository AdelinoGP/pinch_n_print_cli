## Preflight Gate: 259b-fuzzy-skin-noise-modules

Reviewed: 2026-09-01 · Mode: `--preflight` · Symbol-inventory dispatched: 1 packet pair (259a + 259b)
Re-authored under the wayfinder map's Authoring rules 1–6; 259a + 259b replace the retired `259-fuzzy-skin-keys` draft.

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files)     | PASS | all five present and non-empty |
| S1 Prerequisite-status truth      | PASS | packet 259a is named as a **hard dependency in `draft` status with its own open `[BLOCK]`**, not as an implemented one; the gate "259a must read `status: implemented` before Step 1, re-derived at that moment" is stated in `packet.spec.md` and in the plan's Execution Rules. Name/shape reconciliation with 259a's plan is written out |
| S2 Deviation-ID conformance       | PASS | creates one row (DIV-2) and names no ID; instructs re-derivation of both the ID and the convention. Correction applied during this gate: `docs/DEVIATION_LOG.md` carries a dominant `DEV-###` series and a minority `D-<packet>-<SLUG>` series, so the packet no longer presumes the `D-` form |
| S3 Schema-version computed        | PASS (N/A) | pins no `*_SCHEMA_VERSION`; explicitly asserts no IR change |
| S4 ADR slot allocation            | PASS (N/A) | authors no new ADR; the one cited (`0056-integrated-modules-native-dispatch.md`) exists |
| S5 Shipped-symbol existence/shape | PASS (2 shape corrections applied) | **(i)** `dedup_same_claim_modules_with_wall_generator` takes **pre-extracted scalars** (`wall_generator: Option<&str>`, `spiral_vase: bool`, `support_type: Option<&str>`), not a config map; the raw `config_source` read lives one level up in its single non-test call site `load_live_modules_for_plan_with_integrated` (`crates/slicer-wasm-host/src/execution_plan_live.rs`) — the draft named `crates/slicer-runtime/src/run.rs`, which is wrong; corrected in all three files. **(ii)** `apply_fuzzy_skin` has **two** `rng.next_f32() * fuzzy_skin_thickness` sites (the `while dist < seg_len` loop and the `if !emitted_sample` fallback), not one; corrected, with the partial-patch failure mode called out. Verified as claimed: `WALL_GENERATOR_CONFIG_KEY` (`"wall_generator"`), `DEFAULT_WALL_GENERATOR` (`"classic"`), `validate_claim_conflicts`, `load_modules_from_roots`, the `report.modules.len()` count assertion, `claim:fuzzy-skin-generator` absent from docs, and all six net-new crate names absent from the tree |
| S6 WIT/IR identifier drift        | PASS | this packet names no WIT type and no IR variant. Its central design claim — that load-time selection avoids a WIT change — is what keeps it unblocked, and `crates/slicer-schema/wit/**` is listed Out-of-Bounds with a stop-and-report instruction if a worker concludes otherwise |
| S7 Test-target wiring             | PASS | the net-new `crates/slicer-scheduler/tests/integration/fuzzy_skin_generator_selection_tdd.rs` lands in the `mod`-aggregated `scheduler_integration` binary, and its `mod` line in `tests/integration/main.rs` is in the Step 2 edit list with the false-green failure mode spelled out. The five per-module `tests/*_fuzzy_skin_tdd.rs` files are plain auto-discovered targets in net-new crates |
| S8 ADR conformance                | PASS | ADR-0056's registration contract is followed in Steps 3–6; no ADR's normative content is contradicted |
| (existing) AC runnable command    | PASS | 18 of 18 ACs carry a pipe-suffixed runnable command; none uses `cargo test --workspace` |
| (existing) Doc Impact Statement   | PASS | present, four entries, each with a verification command |

### Blockers (S4/S5/S6) — fix before any commit

None outstanding. Both S5 shape errors were corrected during this gate.

### High (S1/S2/S3/S7/S8) — fix or convert to justified FORWARD-DEP

None outstanding.

### Accepted FORWARD-DEPs (consumer name/shape matches the producer packet's plan)

- The gate (`should_fuzzify`), the `fuzzy_skin_mode` switch, and the three control keys ← produced by draft packet **259a**, which builds them inside `modules/core-modules/fuzzy-skin/src/lib.rs`; this packet extracts them into `fuzzy-skin-core` unchanged. Names and shapes reconciled ✓, with a Step 1 SUMMARY dispatch to confirm 259a's final signatures rather than freezing them here.
- **Note:** 259a carries its own unresolved `[BLOCK]` (an IR + WIT change). This packet has **no `[BLOCK]` of its own** — all three blocker triggers were checked against the tree and none applies — but it cannot start until 259a's is resolved.

### Map-specific gates (wayfinder Authoring rule 6)

| Gate | Result | Evidence |
|------|--------|----------|
| (a) zero declaration-only keys in the disposition table | **PASS** | all four ticket-14 keys are class **(b)**, plus three out-of-ticket ripple keys also class **(b)**; counts (a) 0 · (b) 4 · (c) 0 · (d) 0. No "declared-with-gap" anywhere |
| (b) every key has ≥1 AC asserting a behaviour change at a non-default value | **PASS** | `fuzzy_skin_noise_type` → AC-2 (five non-default values) + AC-N3; `fuzzy_skin_scale` → AC-3 (scale 10 vs 1, ordered sign-change count) + AC-6; `fuzzy_skin_octaves` → AC-4 (8 vs 1) + AC-5; `fuzzy_skin_persistence` → AC-4 (0.9 vs 0.1, ordered variance); the three ripple keys → AC-7. AC-8's byte-identity assertions are *honest-absence* guards for `classic`, deliberately not counted as coverage for any key. AC-N1 is labelled additional and is sole evidence for nothing |

### Reviewer note — out-of-ticket keys

Shipping the `ripple` value (user ruling) pulls `fuzzy_skin_ripples_per_layer`, `fuzzy_skin_ripple_offset`, and `fuzzy_skin_layers_between_ripple_offset` into the packet. These are **not** in ticket 14's key list. The packet declares and wires them and flags the required ticket update in its own `task-map.md` and completion gate; the update itself is reported in the session handoff, since a packet may not edit a ticket.

**Verdict:** PREFLIGHT PASS (0 blockers, 0 high)
