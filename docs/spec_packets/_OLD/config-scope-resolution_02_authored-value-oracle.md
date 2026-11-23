---
status: implemented
packet: config-scope-resolution_02_authored-value-oracle
task_ids:
  - TASK-563
---

# config-scope-resolution_02_authored-value-oracle

## Goal

Add a registry-derived executor oracle that independently decodes authored values from `resources/cube_4color.3mf`, drives every declaring module through a live scheduler plan in which that module survives claim deduplication, and fails red when the module's bound `ConfigView` does not contain the exact registry-typed authored value.

## Problem Statement

`read_3mf_project_settings` (`crates/slicer-model-io/src/loader.rs`) applies `coerce_string_to_config_value` before module schemas are available. Module-owned numeric strings `"0"` and `"1"` therefore reach `bind_module_config_view` as `ConfigValue::Bool` when the host declaration channel does not know their keys. The approved plan requires an independently derived ingestion-fidelity oracle that lands red under TASK-563 and becomes packet 03's non-negotiable correction gate.

The first draft could not observe every declared owner: fixture-authored `wall_generator = "arachne"` makes `dedup_same_claim_modules_with_wall_generator` remove `com.core.classic-perimeters`, so walking that one plan can never inspect classic's `filter_out_gap_fill`. This packet instead builds an arachne plan and a classic plan through `prepare_prepass_context`, proves the exact keep/drop pair, and selects the plan in which each manifest-derived owner is live.

## Architecture Constraints

- **Independent authorities:** expected values come only from raw fixture strings plus registry declarations. Delivered values come only from model-io plus the live runtime binding path. Neither side is used to derive the other.
- **Exact owner, not any carrier:** derive declaring module IDs from loaded schemas. A value carried by a different module does not satisfy an absent owner. Missing exact owners produce `MISSING_MODULE` diagnostics.
- **Selector-matrix observation:** clone one production-decoded source map. Preserve fixture-authored `wall_generator = "arachne"` in one run; replace only that selector with `ConfigValue::String("classic")` in the other. Assert the exact perimeter keep/drop sets before comparing values. Use the classic plan only for the classic perimeter owner and the arachne plan for all other owners.
- **Support behavior is not selector-simulated:** support family candidates survive startup dedup, so both support renderer/planner declarations are checked in the authored arachne plan. Do not add global support winner logic.
- **Default coincidence remains eligible:** the oracle observes typed values in `ConfigView`, not guest fallback output. `Float(0.0)` and `Bool(false)` remain unequal even if a guest's fallback magnitude is zero.
- **Automatic exclusion is value-sensitive:** exclude all `percent` declarations, percent-marked `float_or_percent` values, numeric zero with `base_key: Some(_)`, and integer `-1`. Do not exclude an entire field-type family when the authored value is already absolute and non-sentinel.
- **Enum derivation uses `RegistryEntry.values`:** an enum raw string outside the exact declared domain is malformed and excluded; omission of `values` from the consumed packet-01 shape is forbidden.
- **Determinism:** derive keys and module IDs into ordered maps/sets; sort diagnostics by `(key, module_id)`; do not assert filesystem enumeration order.
- **No vacuity or silent fixture skip:** assert raw values, eligible `(key, owner)` observations, arachne modules, and classic modules are non-empty. Missing fixture parts panic with their exact path/member.
- **Red means mismatch only:** the principal test accumulates value mismatches and then asserts that collection empty. Infrastructure, registry assembly, plan construction, missing owner, or selector-matrix errors fail earlier or with a distinct diagnostic and do not count as the accepted red state.
- **Dependency truth:** `zip`, `serde_json`, and `slicer-model-io` already exist under runtime dev-dependencies. Only `slicer-config` is added. No production dependency is authorized; the two host/speed retype changes in the Files-in-Scope exception use existing `slicer-ir` types only.
- **No contract/version changes:** IR schema, WIT package, CLI wire, and manifest schema are untouched. No edited path feeds guest WASM, and no geometry or coordinate conversion is involved.
- **Literal/test quality:** use FRU for new `ModuleDeclaration` literals if the watched-literal gate requires it; the comparator test carries a `// test-quality:` negative-control waiver naming the shared comparator.

## Data and Contract Notes

- IR/manifest contracts: consumed read-only; no schema/version edit, except the two registry-assembly retype wires named in the Doc Impact Statement (`ResolvedConfig.initial_layer_line_width` → `ResolvedFloatOrPercent`, `FeedrateConfig.internal_bridge_speed` → `ResolvedFloatOrPercent`/`float_or_percent`) — no IR/WIT version constant changes either way.
- Resolution behavior change: percent-authored `initial_layer_line_width` and `internal_bridge_speed` now carry their percent bit to their resolution helpers (`resolve_initial_layer_line_width_mm` over nozzle diameter, `resolve_internal_bridge_speed_mm` over `bridge_speed`); resolution expectations for scope/expansion stay packets 4/5.
- Census coverage: `registry_census_tdd`'s speed field-type derivation follows the `SPEED_KEYS` wire table (the `internal_bridge_speed` one-liner), and `registry_assembly_tdd` stays green with the retyped channels.
- WIT boundary: untouched.
- Scheduler determinism: both plans use the same module roots and decoded source except the controlled whole-print selector; exact module IDs are the join key between manifest ownership and compiled bindings.
- Report format: `MISMATCH key=<key> module=<module-id> authored=<quoted-raw> expected=<ConfigValue> delivered=<ConfigValue|None>`; missing owners use the separate `MISSING_MODULE` form.

## Locked Assumptions and Invariants

- TASK-563 and `draft` status remain unchanged.
- Packet 02 closes red; packet 03 owns the green transition.
- The five plan-catalogued key names are absent as Rust string literals in the oracle source.
- Every expected value is raw-document + registry derived, every owner is manifest derived, and every delivered value is read from that exact compiled owner's production-bound view.
- Default-coincident keys are included; automatic placeholders are excluded by authored value semantics.
- Classic-perimeter observations come only from the classic selector plan; no assertion pretends classic survives the authored arachne plan.

## Risks and Tradeoffs

- Packet-01 API drift blocks compilation. Reconcile to packet 01; do not invent compatibility aliases in this packet.
- Two prepass contexts do more work than direct binding, but they are required to prove the observation survives real claim selection; runtime impact is unmeasured.
- The conservative integer `-1` exclusion can omit a future literal negative value that is not automatic. If registry metadata later distinguishes automatic sentinels, replace the conservative predicate in the owning expansion packet rather than adding a key roster here.
- A new claim-exclusive module family could produce another absent owner. `MISSING_MODULE` intentionally blocks packet closure until the selector matrix is extended from an authoritative selector declaration, not silently skipped.
