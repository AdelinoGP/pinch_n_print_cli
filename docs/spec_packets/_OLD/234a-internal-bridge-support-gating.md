---
status: implemented
packet: 234a-internal-bridge-support-gating
task_ids: []
---

# 234a-internal-bridge-support-gating

## Goal

Make internal-bridge-over-infill site selection, anchoring construction, and coverage
canonically faithful machinery delivered end-to-end: restore the fills-as-initial unsupported-span semantics,
author a WIT-visible dense-interior classification (`internal_solid_fill`), split venue so
qualification stays in the prepass while anchored construction consumes real walls and
sparse-infill anchors at InfillPostProcess, port F4's expansion/harvesting/clustering
machinery, and pin the documented matched-oracle baseline set via bundle-primary arbitration.
Residual low-z qualification and coverage breadth diverge from canonical and are owned
elsewhere by the shell-classification, infill, and support tracks (DEV-149/DEV-150), as
restated for closure on 2026-08-25.

## Motivation

The landed 234a edition ported canonical support qualification and relocated it into the
ShellClassification prepass. Canonical-faithful machinery is delivered end-to-end, and the
matched-oracle-profile arbiter pins the documented baseline set; residual low-z qualification
and coverage breadth diverge from canonical and are owned by the shell-classification / infill /
support tracks (DEV-149/DEV-150), as restated on 2026-08-25. The grilling session
(`docs/specs/orca-feature-gap/issues/82-parity-closure-decision-brief.md`) established a NEW,
dispatch-verified root cause candidate (RC-A): `unsupported_span_areas`
(`crates/slicer-core/src/algos/bridge_over_infill.rs`) initializes the unsupported carrier as a
BOUNDING-BOX COMPLEMENT of the lower fills (`fill_envelope`), while canonical
`PrintObject::bridge_over_infill` initializes it to the FILL POLYGONS THEMSELVES
("initially consider the whole layer unsupported"), then closes/shrinks/diffs grown solids.
RC-A predicts the measured histogram exactly. Independently, our IR lacks any dense-interior
(`stInternalSolid`) taxonomy and construction anchors on polygon stand-ins instead of walls.
This packet records the closure machinery and absorbs F4's ported machinery; its residual
coverage deficit remains owned by the infill/construction track.

Relation to prior work: builds on landed 233/234/235 and the original 234a edition; revises
THIS packet directory in place (pre-revision text in git history); no other packet directory
is modified or superseded.

## Architecture Constraints

- **Venue split legality:** per-layer stage arms run under rayon with private arenas;
  cross-layer reads are forbidden outside the sequential prepass. Qualification (needs L-1
  committed state) therefore stays prepass; construction needs only same-layer committed
  state (`internal_bridge_areas` + walls + sparse polylines) so it may move to
  InfillPostProcess. S4 opens with an explicit reachability probe; failure = STOP-and-report.
- **Ordering lock:** support qualification runs strictly after 234's false-site gate within
  ShellClassification; external-bridge orientation from 235 is untouched; partition's
  `sparse_infill_area = difference(wall_inset, bridge ∪ bottom ∪ top)` continues consuming the
  extended `bridge_areas` unchanged.
- **Per-region config resolution:** density branch resolves `infill_density` via EACH lower
  region's own `RegionKey` through `region_map.config_for(...)` — a different resolution site
  than the current first-entry-per-timeline flow lookup; keys stay snake_case.
- **Visibility contract:** `internal_solid_fill` is WIT-MIRRORED (future-proofing for infill
  modules); `internal_bridge_areas` is host-only/un-mirrored but auto-serializes into
  visual-debug bundles because `SliceIR` derives Serialize; both new fields carry
  `#[serde(default)]`; `internal_bridge_lines` disappears tree-wide in S4; S5c-host makes NO
  IR/WIT surface changes — there is no `extra_bridge_areas` field; gated duplicates are
  appended to the upper layer's existing `internal_bridge_areas`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Any ported C++ function (S5: expansion zones, `gather_areas_w_depth`, clustering) carries
  the standard header from `docs/ORCASLICER_ATTRIBUTION.md` at the top of the new file/section.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): every watched-type
  literal touched by S2/S4 uses a `..` rest or an `// exhaustive:` waiver; production literals
  stay exhaustive; `cargo xtask check-literals` runs inside each affected step.

## Data and Contract Notes

- IR/manifest contracts: `InfillRegion.internal_bridge_infill` keeps type/role from 233; no
  manifest changes; no config-key additions (`infill_density` already resolved).
- WIT boundary: one added field on the region type (`internal_solid_fill`); run the WIT-change
  checklist (search `wit_host.rs`/dispatch/guest consumers; verify type identity across the
  component boundary; `cargo build --tests` after edits).
- Determinism/scheduler: prepass sequential over sorted timelines; construction deterministic
  given committed anchors; AC-6 byte-identity is the guard.

## Locked Assumptions and Invariants

- Canonical-faithful machinery is delivered end-to-end; the matched-oracle-profile arbiter
  pins the documented baseline set. Residual low-z qualification and coverage breadth diverge
  from canonical and are owned elsewhere by the shell-classification / infill / support tracks
  (DEV-149/DEV-150), restated on 2026-08-25. The secondary G-code label measurement pins zero
  `;TYPE:Internal Bridge` sections for the fresh matched-profile slice; the external Bridge row
  @ Z≈3.2 keeps [85°, 95°].
- Multiplier mapping: `dont_filter_internal_bridges == ibfDisabled` ⇒ 3, else 1.
- Dense definition locked: shell band MINUS depth-0 exposed seed; threshold fraction >= 0.999.
- Golden policy locked: conditional re-bless with section-count diff table, Z-set identity,
  per-diff-class canonical reasoning; zero-sites not privileged; wedge suites pass unmodified
  or STOP.
- Bundle-JSON primary / gcode secondary instrumentation split.

## Risks and Tradeoffs

- Wall/polyline reachability in the InfillPostProcess arm is UNVERIFIED until the S4 probe;
  failure path is STOP-and-report with measurements (packet stays draft on redesign).
- `apply_opening(half line width)` on candidates has no canonical counterpart; audit-and-
  report after S1: if the surviving calicat candidate cannot clear the opening, surface the
  measurement before any change.
- 20mm-box steady state unknown post-S1 (canonical legitimately bridges under top shells over
  sparse interiors); governed by the golden policy, not by zero-site privilege.
- Mega-packet review surface (F4 absorbed): mitigated by step gates and full `/spec-review`
  closure scope; `cargo xtask test --workspace` reserved for the acceptance ceremony.
- Interim site counts before S5 completes may differ from the final arbiter bar; only S6 bars
  bind.
- Guest fingerprint churn on every IR/WIT touch is expected; freshness gate is authoritative.
