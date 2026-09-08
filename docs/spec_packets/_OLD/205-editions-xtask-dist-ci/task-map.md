# Task Map: 205-editions-xtask-dist-ci

No `docs/07_implementation_status.md` TASK row exists for the multi-edition
distribution program (see `docs/specs/multi-edition-distribution-plan.md`
§"Backlog anchoring [FWD]"). This packet anchors to ADR IDs instead. Do not
invent a TASK number, and do not edit `docs/07` while the parallel 194–199
session is active.

| Anchor | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `ADR-0057` (edition table) | `Step 1` | `docs/adr/0057-three-editions-and-integrated-tier.md` | none (read-only reconciliation) | none | `S` | Proves the FORWARD-DEP shapes from 203/204 exist before any code assumes them |
| `ADR-0057` (dist-config list, not a constant) | `Step 2` | `docs/adr/0057-...md`, `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` | `xtask/src/dist.rs` (`DistArgs`, `parse_dist_args`, `DistPlan`, `plan_edition`) | none | `M` | AC-1..AC-4; every expectation derived from `discover_guests` + `load_editions`, never a literal |
| `ADR-0056` (disjointness consequence) | `Step 3` | `docs/adr/0056-integrated-modules-native-dispatch.md` | `xtask/src/dist.rs` (`assert_staging_disjoint`, `pnp_cli_integrated_features`, `verify_integrated_feature_coverage`, `preflight_edition`) | none | `S` | AC-N1, AC-N2; the invariant gets a falsifiable enforcement point, and the named gate makes its position relative to the build assertable |
| `ADR-0057` (Hybrid row) | `Step 4` | `docs/adr/0057-...md` | `crates/pnp-cli/Cargo.toml` | none | `S` | AC-7; extends packet 203's `integrated-classic-perimeters` to the full Hybrid set |
| `ADR-0057` + `ADR-0056` | `Step 5` | `docs/adr/0056-...md`, `docs/01_system_architecture.md` §dist | `xtask/src/dist.rs`, `xtask/src/main.rs` (incl. `USAGE`) | none | `M` | AC-9, AC-N2, AC-N3, AC-N4, and the executor half of AC-5; `dist_command` signature change, blast radius = two `main.rs` call sites; `preflight_edition` must precede the guest build |
| `ADR-0057` (edition names are user-facing) | `Step 6` | `docs/adr/0057-...md` | `docs/01_system_architecture.md`, `README.md`, `CLAUDE.md` (one line) | none | `S` | AC-8 both halves; additive and anchor-disjoint from packet 204's edit to the same doc section. Four surfaces name the output path (measured); `USAGE` is the fourth and is owned by Step 5 |
| `ADR-0057` (CI gains edition artifacts) | `Step 7` | `docs/adr/0057-...md` §Consequences, `CLAUDE.md` §Guest WASM Staleness | `.github/workflows/ci.yml` | none | `M` | AC-5, AC-6; CI runs `cargo xtask dist` for the first time |

Costs are copied from `implementation-plan.md`. Aggregate `M`; no row is `L`.
