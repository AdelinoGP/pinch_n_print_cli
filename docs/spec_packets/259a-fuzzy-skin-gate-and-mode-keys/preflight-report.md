## Preflight Gate: 259a-fuzzy-skin-gate-and-mode-keys

Reviewed: 2026-09-01 · Mode: `--preflight` · Symbol-inventory dispatched: 1 packet pair (259a + 259b)
Re-authored under the wayfinder map's Authoring rules 1–6; 259a + 259b replace the retired `259-fuzzy-skin-keys` draft.

| Check | Result | Offending items (≤5) |
|-------|--------|----------------------|
| S0 Packet structure (5 files)     | PASS | all five present and non-empty |
| S1 Prerequisite-status truth      | PASS | no dependency is claimed implemented. Ticket 103 is cited as resolved and the tree agrees (the module carries the Orca names). 259b is named as a *dependent*, not a dependency |
| S2 Deviation-ID conformance       | PASS | the packet creates one row (DIV-1) and names no ID; it instructs the implementer to re-derive both the ID **and the convention** at point of use. Correction applied during this gate: the log carries two schemes — a dominant `DEV-###` series and a minority `D-<packet>-<SLUG>` series — so the packet no longer presumes the `D-` form |
| S3 Schema-version computed        | PASS | no version is hardcoded anywhere; BLOCK-1 sub-question 1 explicitly defers the bump decision and forbids freezing a number |
| S4 ADR slot allocation            | PASS (N/A) | authors no new ADR |
| S5 Shipped-symbol existence/shape | PASS | verified: `FuzzySkinModule` (exactly 3 fields), `LayerModule::from_config` and `run_wall_postprocess` as trait methods under `#[slicer_module]`, `apply_fuzzy_skin` (free fn), `Rng::next_f32` mapping to `[-1.0, 1.0]`, `fuzzy-skin.toml` (`holds = []`, `Layer::PerimetersPostProcess`, exactly 3 schema keys), `LoopType` (5 variants), `WallBoundaryType` (3 variants), and the 5 `WallLoop` construction sites across the two perimeter generators |
| S6 WIT/IR identifier drift        | PASS (**with a material finding folded into BLOCK-1**) | **`LoopType` IS mirrored in WIT** as `enum wall-loop-type { outer, inner, thin-wall, nonplanar-shell, gap-fill }` in `crates/slicer-schema/wit/deps/ir-types.wit`, consumed by `record wall-loop-view`. The draft treated this as an open question; it is now recorded as verified fact, BLOCK-1 is restated as *both* an IR schema change and a WIT interface change, `ir-types.wit` is moved from Out-of-Bounds into the BLOCK-1 change surface, and the now-answered Step 1 dispatch is removed |
| S7 Test-target wiring             | PASS | `crates/slicer-ir` declares no `[[test]]` targets and no `autotests = false`, so `tests/loop_type_hole_tdd.rs` is auto-discovered as `--test loop_type_hole_tdd` with no aggregator registration — noted explicitly in Step 1. `fuzzy_config_schema_tdd.rs` is confirmed absent (net-new) and lands as a plain `tests/*.rs` in `fuzzy-skin`. `scheduler_integration` is the real scheduler target name |
| S8 ADR conformance                | PASS | no existing ADR's normative content is contradicted; the packet contradicts no locked field shape |
| (existing) AC runnable command    | PASS | 12 of 12 ACs carry a pipe-suffixed runnable command; none uses `cargo test --workspace` |
| (existing) Doc Impact Statement   | PASS | present, four entries, each with a verification route |

### Blockers (S4/S5/S6) — fix before any commit

None outstanding. The S6 finding was material but is an *authoring* correction, now applied: the packet no longer under-states BLOCK-1's surface.

### High (S1/S2/S3/S7/S8) — fix or convert to justified FORWARD-DEP

None outstanding. The S2 deviation-convention ambiguity was corrected during this gate.

### Open `[BLOCK]` — carried, not cleared

**BLOCK-1 — hole-loop identification.** Canonical `should_fuzzify` needs `is_contour`; neither `LoopType` nor `WallBoundaryType` carries it. The user's ruling is to add the distinction to the IR, which fires **two** of this queue's three blocker triggers (IR schema change; WIT interface change), both verified against the tree. Three sub-questions remain open: whether a schema-version bump is required; whether the distinction belongs as a peer `LoopType` variant or as an orthogonal field (the peer form loses the "inner loop of a hole" case canonical can express, and is the more expensive WIT change); and which guest-side matches must gain an arm. A documented fallback exists that keeps the packet rule-1-compliant without any IR or WIT surface.

**Consequence:** status stays `draft`. The packet is authored and preflight-clean, but is **not activatable** until an architecture owner rules.

### Map-specific gates (wayfinder Authoring rule 6)

| Gate | Result | Evidence |
|------|--------|----------|
| (a) zero declaration-only keys in the disposition table | **PASS** | all three keys are class **(b)**; counts (a) 0 · (b) 3 · (c) 0 · (d) 0. No "declared-with-gap" anywhere |
| (b) every key has ≥1 AC asserting a behaviour change at a non-default value | **PASS** | `fuzzy_skin` → AC-2 (five non-default values); `fuzzy_skin_first_layer` → AC-3 (`true`); `fuzzy_skin_mode` → AC-4 + AC-N4 (`extrusion`, `combined`). AC-N1 (default identity) is explicitly labelled additional and is sole evidence for nothing |

**Verdict:** PREFLIGHT PASS (0 blockers, 0 high) — **with one open `[BLOCK]` (BLOCK-1) that bars activation.**
