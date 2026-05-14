# Design: 56_threemf-sidecar-parser

## Controlling Code Paths

### Today's behavior (current state of master)

- `crates/slicer-host/src/model_loader.rs:130-203` — `load_model` for 3MF builds `ObjectMesh` per item returned by `load_3mf`, hardcodes `SemVer { 1, 0, 0 }`, and initializes `modifier_volumes: Vec::new()` unconditionally (line 179).
- `crates/slicer-host/src/model_loader.rs:430-552` — `resolve_object` recursively walks `<component>` children of an `<object>` and merges every leaf mesh into one `IndexedTriangleSet`. There is no branch on part identity beyond `objectid`.
- `crates/slicer-host/src/model_loader.rs:555-587` — `load_3mf` opens the ZIP archive (line 558-559) and calls `find_model_path` which only ever returns `3D/3dmodel.model` (line 579-581). The ZIP archive handle is dropped after `parse_3mf_model_xml` consumes the model XML bytes.
- `crates/slicer-host/src/model_loader.rs:599-` — `parse_3mf_model_xml` consumes the model XML bytes. It does not see the ZIP archive and cannot reach `Metadata/model_settings.config`.

### After this packet

- `load_3mf` opens the ZIP archive AS BEFORE, then:
  1. Calls `parse_3mf_model_xml` to consume `3D/3dmodel.model` (as today).
  2. Calls `parse_3mf_sidecar(&mut zip)` to read `Metadata/model_settings.config` if present.
  3. Both results are threaded into the eventual call to `resolve_object` (signature widens to accept `&HashMap<u32, ObjectSidecarInfo>` as an additional parameter).
  4. The `ZipArchive` is dropped only AFTER step 2.
- `resolve_object`'s body is **unchanged** — the new parameter is currently unused. Packet 56b branches on it.
- No IR change. No schema bump. No `ObjectMesh.modifier_volumes` populated by this packet.

## Neighboring Tests / Fixtures

- `crates/slicer-host/tests/threemf_transform_tdd.rs` — 3MF transform regression (10 tests passing). Must stay green; this packet only adds an unused parameter to `resolve_object`.
- `crates/slicer-host/tests/gcode_emit_tdd.rs` — G-code emission (27 tests passing). Must stay green.
- `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` — no-sidecar regression baseline. `resources/benchy_painted.3mf` has no `Metadata/model_settings.config`.
- `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` — Packet 51 paint-config overlay regression.
- `resources/benchy_4color.3mf` — primary fixture. Its sidecar is the well-formed reference parsed by AC-1.
- `resources/benchy_painted.3mf` — no-sidecar fixture (AC-2).

## Architecture Constraints

- WIT boundary: clean. This packet does not even touch IR types; it only adds host-internal `PartSubtype` enum + carrier structs.
- IR versioning: no change. `MeshIR.schema_version` stays at 1.0.0; the bump to 1.1.0 happens in Packet 56b.
- Logging: use the project's existing `log` facade (`log::warn!`, `log::trace!`). Pick the log target carefully — Step 2's FACT dispatch confirms the exact target string used by other 3MF-loader log lines so the AC's `"slicer_host::model_loader::sidecar"` literal matches reality (or the AC is updated to match the existing convention).

## Selected Approach (Locked Decisions)

| Decision | Locked choice | Justification |
|---|---|---|
| Sidecar parser placement | In-file helper inside `model_loader.rs`, OR a new sibling `model_loader_sidecar.rs` if `model_loader.rs` exceeds 800 lines post-addition. The implementer measures at Step 2 close. | Keeps the call site close; the sidecar is a 3MF concern only. |
| Sidecar return shape | `HashMap<u32 /* objectid */, ObjectSidecarInfo { parts: HashMap<u32 /* part_id */, PartSidecarInfo { subtype: PartSubtype, metadata: BTreeMap<String, String> }> }>`. `PartSubtype` is a host-local enum in `model_loader.rs` (or its sibling); NOT an IR type. | The IR carries typed config keys (`config_delta.fields`), not subtype enums. Keeping `PartSubtype` host-local avoids an IR ripple. Packet 56b translates `PartSubtype` into `ConfigKey`/`ConfigValue` pairs when building `ModifierVolume.config_delta`. |
| `PartSubtype` variants | Exactly five: `NormalPart`, `ModifierPart`, `NegativePart`, `SupportEnforcer`, `SupportBlocker`. Add `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`. | Mirrors the five OrcaSlicer subtypes. `Copy` keeps downstream signatures cheap. |
| Unknown subtype handling | Map to `PartSubtype::NormalPart` + `log::warn!` containing the unrecognized string and the substring `"downgrading to normal_part"`. | DEV-047 contract. |
| Missing sidecar handling | Return `HashMap::new()`; no warning. | DEV-049 contract; absence is the default for fixtures without sidecars. |
| Malformed sidecar handling | Return `HashMap::new()` + `log::warn!` containing `"treating all parts as normal_part"`. | DEV-049 contract. Loader must not fail. |
| XML parsing | Reuse `quick_xml::Reader` (already used by `parse_3mf_model_xml` per Step 1's FACT dispatch). | Pattern reuse; no new dependency. |
| Metadata key namespace | Raw `String` keys in `PartSidecarInfo.metadata`. Packet 56b is responsible for translating these into typed `ConfigKey`/`ConfigValue` pairs. | Decouples parser from IR. Parser stays a pure string-to-string map. |
| `<part>` ID interpretation | The `<part id>` value in the sidecar is the inner key; it refers to the `<object id>` of the leaf component referenced from `3dmodel.model`. The outer `<object id>` in the sidecar is the wrapper. Confirmed by Step 1 OrcaSlicer LOCATIONS dispatch. | Matches Bambu's documented convention. |
| `load_3mf` plumbing | `parse_3mf_sidecar(&mut zip)` is called between `parse_3mf_model_xml(...)` (which already consumed `3D/3dmodel.model`) and the `ZipArchive` drop. The order is locked at Step 3 to avoid a borrow-checker conflict with `parse_3mf_model_xml`'s read of the archive. | Single archive open; explicit order. |
| `resolve_object` signature | Widens to accept `&HashMap<u32, ObjectSidecarInfo>` as the final parameter. Body unchanged; the parameter is named `_sidecar` (underscore-prefixed) in this packet to silence the `unused_variables` lint. Packet 56b removes the underscore when it branches on the value. | Allows Packet 56b to focus on the branch logic without re-plumbing the call chain. |

## Rejected Alternatives

| Alternative | Reason rejected |
|---|---|
| Parse the sidecar inside `parse_3mf_model_xml` | `parse_3mf_model_xml` works on raw XML bytes, not on the ZIP archive. The sidecar is a separate ZIP entry. Two parser functions, one archive open. |
| Re-open the ZIP archive a second time inside `resolve_object` | Doubles I/O and risks divergent archive state. Opening once in `load_3mf` is the existing pattern. |
| Make `PartSubtype` an IR type in `crates/slicer-ir/` | The IR's `ConfigDelta` already carries typed values via `ConfigValue` enum. Adding a parallel typed enum at the IR layer doubles maintenance surface. Packet 56b will translate `PartSubtype` into `ConfigKey::from("subtype")` + `ConfigValue::String(...)` at the ingestion boundary. |
| Hard error on malformed sidecar | Hand-edited sidecars would break user fixtures. User-selected fallback-with-warning. |
| Defer the `load_3mf` plumbing to Packet 56b | Would force Packet 56b to do both the plumbing AND the `resolve_object` branching in one packet, pushing its cost back toward L. Better to let Packet 56's "producer-only" packet absorb the plumbing. |

## Code Change Surface (≤ 3 primary files per step)

Primary files this packet edits:

1. `crates/slicer-host/src/model_loader.rs` — host-local types (`PartSubtype`, `ObjectSidecarInfo`, `PartSidecarInfo`), `parse_3mf_sidecar` helper, `load_3mf` plumbing, `resolve_object` signature widen (body unchanged). Up to ~250 added lines.
2. `crates/slicer-host/src/model_loader_sidecar.rs` — NEW sibling file if `model_loader.rs` exceeds 800 lines after the addition.
3. `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — NEW; parser unit suite.
4. Four documentation files (`docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md`).

Each step picks at most three files and a worker dispatch covers each step in isolation. See `implementation-plan.md` for the per-step file allocation.

## Read-only Context the Implementer Needs

| Path | Lines | Purpose |
|---|---|---|
| `crates/slicer-host/src/model_loader.rs` | 130-203, 430-587, 599-650 | `load_model`, `resolve_object` signature, `load_3mf`, `find_model_path`, `parse_3mf_model_xml` entry. |
| `crates/slicer-ir/src/slice_ir.rs` | 230-265 | `ConfigDelta`, `ModifierVolume`, `ObjectMesh` (informational — no edits in this packet; Packet 56b is where IR fields are populated). |
| `docs/02_ir_schemas.md` | 5, 192-211 | Versioning rule + `ConfigDelta`/`ModifierVolume` shape (informational). |
| `resources/benchy_4color.3mf` | sidecar XML (≤ 60 lines after `unzip -p`) | Confirm sidecar shape via Step 1's SNIPPETS dispatch. |

## Out-of-Bounds Files (must not be loaded directly)

- `crates/slicer-macros/src/lib.rs` (>2300 lines).
- `crates/slicer-sdk/` — all files.
- `crates/slicer-ir/` — read informational sections only (≤ 30 lines per read); no edits.
- All `wit/**`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs` — confirmed clean by predecessor's Step 0; this packet does not re-check because no IR types are introduced.
- `crates/slicer-host/src/region_mapping.rs`, `pipeline.rs`, `paint_segmentation.rs`, `prepass.rs`, `config_resolution.rs` — owned by Packet 56b / 56c.
- `OrcaSlicerDocumented/**` — always delegate via Explore agent with the LOCATIONS return-format. One dispatch total in this packet (Step 1).
- `target/`, `Cargo.lock`, generated code.

## Data and Contract Notes

- `parse_3mf_sidecar` returns `HashMap<u32, ObjectSidecarInfo>`. Caller must ensure the `ZipArchive` is still open at call time.
- `ObjectSidecarInfo { parts: HashMap<u32, PartSidecarInfo> }`. The outer key is `<object id>` from `<object id="N">` in the sidecar; the inner key is `<part id>` from `<part id="M" subtype="...">`.
- `PartSidecarInfo { subtype: PartSubtype, metadata: BTreeMap<String, String> }`. `metadata` is the verbatim key/value map from each `<metadata key="..." value="..."/>` row inside the `<part>`. Reserved keys: `fuzzy_skin`, `extruder`, `matrix`.
- `PartSubtype` enum is `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`.
- Logging: pick the existing log target convention (verified by Step 2 FACT dispatch: "Return the log target string used in the 5 most recent `log::warn!` / `log::trace!` calls in `crates/slicer-host/src/model_loader.rs`. SNIPPETS, ≤ 5 lines."). Default placeholder for ACs: `slicer_host::model_loader::sidecar`.
- No determinism concern in this packet — the parser produces stable output for stable input by construction (no UUID, no timestamp).

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| `parse_3mf_sidecar` introduces a new `quick_xml::Reader` parse loop; if it diverges from `parse_3mf_model_xml`'s loop pattern, future maintenance burden. | Step 2 FACT dispatch returns the existing `quick_xml::Reader` pattern verbatim (≤ 30 lines). Implement using that pattern. |
| `resolve_object` signature widen breaks any call site that passes a different argument list. | Step 3 FACT dispatch enumerates every call site of `resolve_object`. Expected count: 1 (from `parse_3mf_model_xml`). Update that call site. |
| `_sidecar` underscore parameter triggers `unused_variables` lint despite the underscore prefix in some clippy configurations. | Step 4 verifies clippy is clean. If the underscore alone doesn't satisfy clippy, add `#[allow(unused_variables)]` on the parameter — and remove the allow in Packet 56b when the value is consumed. |
| Sidecar `<part id>` and `<object id>` could collide if a fixture violates Bambu's convention. | AC-5 asserts the convention against a synthetic fixture. If a real-world fixture violates it, register an additional DEV at Step 6 and adapt; do not block this packet. |
| Step 2 worker accidentally absorbs the full sidecar XML (~60 lines) into its context. | Step 2 dispatch uses SNIPPETS return-format with explicit ≤ 30 line cap. |

## Open Questions Blocking Activation

- None remaining. The three-way split resolves the original Activation Q1 (L-aggregate). Q3 (negative-part subtract stage placement) is out of scope here. Q4 (fuzzy-skin manifest gate) is owned by Packet 56b.

## Locked Assumptions and Invariants

1. WIT scope is clean and not re-checked in this packet (no IR types introduced).
2. `parse_3mf_sidecar` returns a `HashMap<u32, ObjectSidecarInfo>` where the outer key is `<object id>` and the inner key is `<part id>` per Bambu's sidecar convention.
3. `PartSubtype` is host-local and not exposed at any IR or WIT boundary.
4. Missing sidecar → empty map, no warning. Malformed sidecar → empty map + warning. Unknown subtype → `NormalPart` + warning.
5. `resolve_object`'s body is unchanged; its signature widens with an underscore-prefixed `_sidecar` parameter.
6. `MeshIR.schema_version` stays at 1.0.0 in this packet. Packet 56b bumps it to 1.1.0.
7. All regression suites stay byte-identical to pre-packet output because no consumer reads the sidecar map.
8. `resources/benchy_painted.3mf` (no sidecar) slices byte-identical to pre-packet output. This is a hard invariant verified by AC-8.
