# 33 — Author packet P26 — Calibration / Flow / Pressure advance calibration — infill modules

Type: task
Status: resolved
Assignee: Adelino Penedo (agent session)
Blocked by: 06, 105, 107
Map: ../map.md

## Question

Author the spec packet for **P26 — Calibration / Flow / Pressure advance calibration — infill modules** — 1 keys, Tier B new logic, owner infill modules. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P26 — Calibration / Flow / Pressure advance calibration — infill modules):

`calib_flowrate_topinfill_special_order`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/275-top-fill-order-and-calibration-order/`](../../../spec_packets/275-top-fill-order-and-calibration-order/),
`status: draft`, `PREFLIGHT PASS`. Blocked on packet 264 reaching `status: implemented`.

**The ticket's sizing had rotted.** `calib_flowrate_topinfill_special_order` is not
"1 key, Tier B, declare and wire". It is a rider on a pattern family this port does
not have, and honouring it needs an SDK ordering kernel, a host order-lock widening,
and a key the map never inventoried.

**Canonical behaviour.** The key does two things, neither expressible here before this
packet. `Fill::fill_surface_extruded` (`Fill/FillBase.cpp`) sets `no_sort = true` on the
top-solid-infill collection and calls `set_reverse()` on every entity — and
`ExtrusionEntity.hpp` shows `set_reverse()` sets `m_can_reverse = false`, i.e. it
*forbids* reversal rather than performing one. Together the top fill becomes an atomic,
non-reorderable, non-reversible block. Separately `FillPlanePath::fill_surface` reorders
the clipped fragments so the longest one — the center spiral — is emitted last and runs
inside-out, chords chained ahead of it; canonical's own comment says the opposing
directions raise the tactile lip the calibration is read from. It fires only for
`FillArchimedeanChords` and only while `top_surface_fill_order == Default`. The flag is
set exclusively by the flow-rate calibration object generator in `slic3r/GUI/Plater.cpp`,
on `_obj->config`, alongside `top_surface_fill_order = Default`.

**Scope split (user ruling, 2026-09-03).** Packet 264 already ships
`archimedean-chords-infill` as a `claim:top-fill` holder. That module stays with 264;
everything else is packet 275: the ordering kernel, the `order_lock` emission, the host
widening, and the two fill-order keys.

**The port's side, measured.**
- The atomic-block seam already exists and is a near-exact match: `ExtrusionPath3D::order_lock`
  (IR schema 1.4.0, packet 244), enforced by `validate_entity_order_locks` and honoured by
  `coalesce_locked_candidates` in `path-optimization-default`, which collapses a maximal run
  of equal tags into one non-reversible candidate.
- But it never reaches top fill: `remap_infill_order_locks_from`, `next_global_infill_tag`,
  and `validate_infill_order_locks` walk `InfillRegion::sparse_infill` **only**, while top
  fill lands in `solid_infill`. A module-local tag is never promoted to a global tag.
  `assemble_ordered_entities_with_support_identities` already walks all four vectors, so the
  entity view and the remap disagree. **ADR-0062 settles it**: the host remaps "at every
  output boundary" and enforces "at every mutation point" — the sparse-only walk is narrower
  than the ADR packet 244 landed under, so widening it is conformance, not a scope extension.
- No module sets a lock today; `OrderLockAllocator` has zero callers outside the SDK.
- No center-based top-fill pattern exists: `claim:top-fill` is held only by `rectilinear-infill`
  (scan lines) and `gyroid-infill`. `InfillType::Concentric` is declared and unimplemented.

**Three corrections to beliefs this ticket would otherwise have shipped.**

1. **The `Orca(pnp_gui)` checkout is not a canonical oracle and would have produced a false
   dead-key ruling.** It reports *zero* read sites for this key. Its HEAD is
   `pnp B7/F13: FFF pipeline removal — M2 CLOSE (native-slicing rip-out complete)`, and
   **73 of its 215 `libslic3r` `.cpp` files are absent** — all of `Fill/`, all of `Arachne/`,
   `GCode.cpp` and `GCode/`, `Brim.cpp`, `Feature/Interlocking/`, `FuzzySkin.cpp`. It is also
   older (`02.06.00.51` vs `02.08.01.55`). Under Authoring rule 3 this key would have been
   ruled dead and dropped from the queue. The map now carries this as a Notes hazard, with
   the user's designated oracle path.
2. **The 3MF ingest is NOT a blocker** — a claim this session made and then disproved.
   `parse_project_settings_json` (`crates/slicer-model-io/src/loader.rs`) ingests **every** key
   from an Orca 3MF's `project_settings.config` generically via `json_to_config_value`, with no
   allowlist. The 24-key allowlist in `object_metadata_to_config_data` governs only the
   per-object `Slic3r_PE_model.config` path, and it does produce booleans (`enable_support`
   through `coerce_string_to_config_value`). No ingest work is needed and none is in the packet.
3. **`order_lock` is not merely an ordering flag.** ADR-0063 makes locked paths *self-clipping*
   — the producer guarantees the whole swept footprint is inside its legal domain, the linker
   neither clips nor links them, and it differences that footprint out of untagged fill of the
   same region. Emitting a lock on top fill therefore incurs geometry obligations, not just
   ordering ones. Packet 275's AC-14 asserts both rather than assuming them, and stops the
   implementer if the carve is not a no-op against sparse infill.

**Queue-completeness finding (feeds ticket 123).** `top_surface_fill_order` and
`bottom_surface_fill_order` gate this key's behaviour and appear **nowhere** in
`docs/ORCA_CONFIG_REFERENCE.md`, so nowhere in the 407-key queue. Both are live:
`Fill/Fill.cpp` reads them whenever the pattern is `ipConcentric` / `ipArchimedeanChords` /
`ipOctagramSpiral`, and `PrintObject.cpp` lists them in its invalidation set. Two more live
slicing keys are likewise absent: `separated_infills` and `center_of_surface_pattern`. Packet
275 implements the two fill-order keys; the reference's row set remains ticket 123's subject.

No key declared and no code changed in this session — the deliverable is the authored packet.

