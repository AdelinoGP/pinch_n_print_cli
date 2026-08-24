# Task Map: 238c-support-renderer-flow-interfaces

Crosswalk for the TASK-381..TASK-398 slice. Registration in `docs/07_implementation_status.md`
is deferred to the packet-owned closure step (Step 18) — this file is the mapping, not
the ledger.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-381` | Step 1 | plan §10 DEV-145; gap register | `modules/core-modules/{traditional-support,tree-support}/*.toml` | `PrintConfig.cpp` (`PrintConfigDef::build`) | S | default −1.0 → 0.5; legacy sentinel stays parseable |
| `TASK-382` | Step 2 | plan §12 G-11; AC-2/AC-N1 | `crates/slicer-core/src/support_regularize.rs` (new), `tests/support_flow_semantics_tdd.rs` (new) | `SupportParameters.hpp` (ctor) | S | canonical density closed forms + clamp/guard; E6 flag on runs |
| `TASK-383` | Step 3 | plan §10 DEV-129; AC-12 | `modules/core-modules/tree-support-planner/tree-support-planner.toml`, `docs/DEVIATION_LOG.md` | `SupportParameters.hpp` (`number_of_support_interface_bottom_layers`) | S | verify-close-or-finish; no third state |
| `TASK-384` | Step 4 | gap register G-12; AC-3 | `modules/core-modules/tree-support-planner/src/lib.rs` (+ tree_family_tdd) | `TreeSupport.hpp` (MIN/MAX_BRANCH_RADIUS) | S | 6.0 → 10.0; E3 if golden drift |
| `TASK-385` | Step 5 | gap register G-13; AC-4 | same planner lib.rs + test | `TreeSupport.cpp` (`calc_branch_radius` mm-to-top) | M | raise-to-base under top>0 |
| `TASK-386` | Step 6 | gap register G-18; AC-5 | `traditional-support/src/lib.rs`, `tree-support/src/lib.rs`, `support_family_closure.rs` | `TreeSupport.cpp` (`draw_circles` floor block) | M | 3 blocks at top=2/bottom=2; ee27ac94 pins hold |
| `TASK-387` | Step 7 | gap register G-10/G-11; AC-1 | `tree-support/src/lib.rs` + tests + manifest | `TreeSupport.cpp` (`generate_toolpaths`), `SupportCommon.cpp` (`tree_supports_generate_paths`) | M | hollow walls; `support_density` key removed |
| `TASK-388` | Step 8 | AC-9 | `crates/slicer-core/src/support_regularize.rs`; both module copies deleted | `SupportCommon.cpp` (`generate_interface_layers`) | S | byte-conserved move |
| `TASK-389` | Step 9 | DEV-146; AC-11 | `traditional-support/src/lib.rs` + both manifests | `Flow.cpp` (`support_material_interface_flow`) | M | flow-factor mechanism over line width |
| `TASK-390` | Step 10 | F-37 carrier; AC-7 (WIT half) | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/deps/{prepass-support-geometry/prepass-support-geometry.wit, ir-types.wit}` | none (structural) | M | schema bump in-step; match-arm sweep owned here |
| `TASK-391` | Steps 11a/11b | F-37 host legs; T9 guard | `crates/slicer-wasm-host/src/{host.rs,dispatch.rs,marshal/in_.rs,marshal/native.rs}` — dispatch.rs owns the live role match | none | S+S | 11a: dispatch arm + builder; 11b: both marshal legs round-trip role |
| `TASK-392` | Step 12 | F-37 end-to-end; AC-6/7/8 | planner attribution, renderer consumption, `crates/slicer-gcode/src/emit.rs`, `docs/02_ir_schemas.md` | commit `050d5c3a` derivation record | M | `;TYPE:Support interface` marker decision realized |
| `TASK-393` | Step 13 | DEV-146 tree side | `tree-support/src/lib.rs` + test | `SupportParameters.hpp` (`top_interface_spacing`) | S | drafts deviation rows |
| `TASK-394` | Step 14 | AC-N1/AC-N2 module boundary | both renderers + tests | none beyond Step 2 | S | degenerate-config guards |
| `TASK-395` | Step 15 | gates | workspace gates | none | S | check/clippy/check-literals clean |
| `TASK-396` | Step 16 | §8 human gate | `tmp/**` artifacts, packet.spec sign-off | none | S | block counts vs references REQUIRED |
| `TASK-397` | Step 17 | Doc Impact Statement | `docs/DEVIATION_LOG.md`, config docs, IR docs | none | S | re-derive next free DEV id at filing |
| `TASK-398` | Step 18 | registration | `docs/07_implementation_status.md`, status flip | none | S | queue-table update stays orchestrator-owned |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or
aggregate exceeds M.
