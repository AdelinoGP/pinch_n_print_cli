# Handoff — packet 224 `support-family-orca-closure` (2026-08-20, session 3) [condensed]

Session-3 record at HEAD `ed62090d`, branch `parity/support-planners`; packet status `draft`.
Session-2 tail under audit: `98147a90..ed62090d` (8 commits: interface-invariant gauntlet closure,
Step-2 interface layer counts, Step-3a tree/planner punch lists, RC-A family-assignment fallback,
handoff/punch-list docs). Session 3 itself produced no code commits.

## Decisions locked with the human (do not relitigate)

| Topic | Decision |
|---|---|
| Scope | Closes on correctness + honest tests; feature gaps route via the gap register |
| Parity gate | Structural invariants + human/LLM `/visual-debug` inspection vs side-by-side Orca renders. **No test reads Orca G-code** |
| Orca references | Regenerated 2026-08-18 with many settings disabled; inspection aids only, gitignored in `tmp/` |
| RC-15 contact sampling | Classified GAP but **implemented in 224** (dominant cause); bug cluster fixed together, measured once at end (two errors currently cancel) |
| RC-17 regressions | Fix forward as punch list; do not revert `9f4540bd` |
| Benchy golden | Regenerate LAST, renamed off `orca_parity`, provenance header: PnP self-capture, NOT parity evidence |
| Baseline metric | Deposited material + XY path length. Never total E (counts de-retraction primes) |
| Gap routing | `docs/specs/support-parity-gap-register.md` plus packets 224a / 225 / 226 / 227 |

## Measurement traps (each cost real time)

1. `cargo test -p tree-support-planner` builds 8 binaries — always `--no-fail-fast`.
2. Closure tests are bare `#[test]`; filter `support_family_closure` matches ZERO tests — use `-- <name> --exact`.
3. `cargo xtask build-guests --check` before EVERY measurement; silence + exit 0 = clean.
4. Guest `eprintln!` does not reach the harness — use `push_diagnostic` or fatal-error text.
5. The planner guest bakes in the WORKING TREE: stash WIP → rebuild guests → measure → pop → rebuild.

## Session-3 audit verdicts on session 2

- Sound: `39507cff`, `0629a9b5` (fixture migration `global_support_layer_index` 8→7 only), `ee27ac94`
  (`interface_layer_count_follows_config`, exact 1/2/3 through a real run_slice; new 2/2 override
  matched tracked `orca-matched-config.json`, so the closure baseline did not silently drift).
- **Finding 1:** `868508ba` inverted `default_ineligible_region_generates_zero_support` →
  `planned_region_renders_regardless_of_eligibility_flag` (`enforcer_blocker_tdd.rs`): asserts support IS
  produced though `needs_support` is hardcoded `true` in `classify_object`
  (`crates/slicer-core/src/algos/mesh_analysis.rs`) and `SliceRegionView`'s `Default`/`from_ir`
  (`crates/slicer-sdk/src/views.rs`) — no producer ever sets it false. Leaves
  `enforcer_overrides_needs_support_false` vacuous and contradicts Step 3a's "no assertion weakened"
  postcondition. OPEN DECISION left to the human (later resolved session 4: keep inversion, route gap row).
- **Finding 2:** `ed62090d` headline "no assertion weakened" is false — its diff widens `distributed_contacts`'
  per-entry role check to `any(|role| !role.regions.is_empty())`. Widening defensible (interface/body made
  disjoint; precedent `acf9fa1d`) but the headline must not be quoted forward.
- Cleared: no `#[ignore]`/`#[allow]`/`should_panic` added in `98147a90..ed62090d`; `868508ba` deletions
  (`branching_pattern_present`, `density_affects_coverage`) defensible (branching moved to planner-level
  `distributed_contacts`; `from_config_custom` covers percent→fraction).

## Verified state at ed62090d (2026-08-20, WIP stashed, guests rebuilt)

Closure gauntlet **12/12** (256 s); tree-support-planner 8 binaries, only
`benchy_orca_parity_within_tolerance` red (RC-C: Hausdorff 1.7553 mm vs 0.5 mm tol on a PnP self-capture —
correctly left red, regenerated Step 8); tree-support 27/27; traditional-support 19/19;
traditional-support-planner 12/12. ⇒ Steps 2 & 3a DONE; orphan-interface defect CLOSED
(`interface_is_topmost_and_carved_out` green).

## In-flight WIP then: Step 3b RC-15 port REGRESSED the gauntlet (closure 9/12; planner 10 failures / 8 binaries)

Failures included exact-Z occupancy overlap ("cube 80..118 orphan" that `ed62090d` closed, back), tree
interface count off-by-one (Step 2 regression), `distributed_contacts` classes `[6,0,0]` (not over-sampling),
two missing code-1002 node-clamped-out diagnostics (opposite of over-sampling). Leading hypothesis: collision
gate narrowed to `node_roles[i] == Body && body_intersects(...)`, precisely undoing `ed62090d`'s
endpoint-role-aware exemption; canonical checks all nodes. OPEN DECISION left to the human (resolved
session 4: revert narrowing, keep sampling). Port rough edges: rotated-grid span from unrotated bbox;
module-local `DEFAULT_MAX_BRIDGE_LENGTH_MM` stands in for an undeclared key.

## Open work (order): resolve the two open decisions; finish Step 3b (back to closure 12/12 + only-RC-C-red, commit port checkpoint, one-time deposited-material/XY re-measurement, update design.md §Measured Baseline);
Step 6 visual-debug inspection gate per family vs Orca renders; Step 7 paperwork (needs_support gap row,
gap register, stubs 224a/225/226/227, AC amendments, docs/07 via worker); Step 8 close (golden last,
check-literals/clippy/workspace tests, acceptance ceremony).

design.md §Measured Baseline figures STALE (2026-08-18). Never quote void numbers 486.33 / 1538.36 /
852.02 / 1158.87 or the derived 31.6%; never gate on extruding-move counts.
Inherited, not branch-attributable: check-literals 61 violations / 34 files (re-derive before quoting —
ledger fact); wasm-host contract `support_plan_aggregation_diagnoses_duplicate_identity` failing pre-existing.
