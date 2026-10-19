# Task Map: lightning-migration

Explicit five-file authoring approval requires this crosswalk despite the single task. The authoritative backlog slice is `TASK-577` in `docs/07_implementation_status.md`; it is registered but not completed. This file does not own acceptance criteria, scope, code-surface detail or step contracts.

| Task | Packet step | Primary authorities | Expected surface | Canonical refs | Context | State |
| --- | --- | --- | --- | --- | --- | --- |
| TASK-577 | 1 | Source plan queue/outcomes; ADR-0066; rows01–07 acceptance | Read-only evidence/source/config/provenance verification | Generator.cpp::Generator::generateTrees; Fill.cpp::Layer::make_fills | M | No gate executed; actual generic acceptance and executed A/B outcomes required |
| TASK-577 | 2 | Skill atomic contract; docs02/03/21/22 | This packet's blocker resolution/retirement partition; other owner files in separate bounded authoring slice | Fill.cpp::group_fills; Generator.cpp::Generator::Generator | M | Independent preflight/activation pending |
| TASK-577 | 3 | Accepted rows05–07; docs04/05/21/22 | `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs`, registration/dev dependencies | Portable row07 canonical records | M | NET-NEW planned production test harness |
| TASK-577 | 4 | Accepted row03; resolved B3; docs08/21/22 | Module-private piece helpers and decoder tests | Accepted final kernel/caller identities | M | Codec/piece/error design blocked |
| TASK-577 | 5 | Resolved B1/B2; ADR-0066; accepted rows01/04/05/07 | `LightningInfill` paired preparation/source bridge/fresh consumers | PrintObject.cpp::PrintObject::bridge_over_infill; Generator.cpp::Generator::generateTrees; Fill.cpp::Layer::make_fills | M | Scientific strategy/source exposure/config/caller evidence blocked |
| TASK-577 | 6 | Accepted row06; source-plan projections; docs19 | Module-owned typed planning_geometry publication/declarations | Independent final graph/grounding references | M | Representation/id mapping and exact declaration ordering blocked |
| TASK-577 | 7 | ADR-0066 direct retirement; docs01–05/21/22 | Entire concrete legacy retirement closure listed in design, after bounded partition | No new canonical read | M per partition | B4 compatibility/literal/compiler/doc partition unresolved; no production edits |
| TASK-577 | 8 | Finalized packet ACs; docs21/22; actual source-plan evidence | Delegated narrow acceptance and independent closure | Portable canonical provenance | M | No acceptance executed |

Queue dependencies are #5/#6/#7, additionally complete generic framework acceptance for every row01–06 sub-gate and independently verified actual executed Gate A/B outcomes recorded in the source plan. TASK-572/573/574/575/576 remain predecessor-owned and are not absorbed or marked complete here. No source-plan/backlog/other packet update is authorized by this authoring task. Known retirement complexity must be partitioned and independently reviewed before activation, not inferred as settled scope from this crosswalk.

**DRAFT; NOT IMPLEMENTATION-READY; NO ACCEPTANCE EXECUTED**
