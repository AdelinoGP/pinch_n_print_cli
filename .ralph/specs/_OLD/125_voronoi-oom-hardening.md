---
status: implemented
packet: 125_voronoi-oom-hardening
task_ids: []
---

# 125_voronoi-oom-hardening

## Goal

Eliminate the painted-model OOM at its root by separating the dual-purpose `RegionKey.region_id` into
a first-class `PrintEntity.tool_index` (pure tool selector) and a pure region identity, then build the
clean axes the split unlocks: D14-correct fuzzy-skin routing, a per-tool config overlay (emit-time
settings + painted-tool geometry), and deterministic containment of the boostvoronoi failure modes.

## Problem Statement

A painted/MMU model (`cube_fuzzyPainted.3mf`) crashed the slicer with a 9.9 GiB allocation. The
diagnose session pinned the chain: `RegionKey.region_id` is **dual-purpose** — both a region IDENTITY
(a 64-bit `PaintValue` hash, e.g. `0x3E8281949ECA9508`) and the slot the resolved TOOL index is stored
into. A painted region's identity (`as u32 = 2_664_076_552`) leaked through the tool slot into
`slicer-gcode/src/emit.rs`, which sized a dense `vec![0.0f32; max_tool + 1]` ≈ 9.92 GiB.

The original packet 125 *bounded* the crash (a `DEFAULT_TOOL=0` resolver floor, a `MAX_PLAUSIBLE_TOOLS`
emit guard, and a `>1 GiB` allocator tripwire) but **deferred the real fix** as "a separate refactor"
and left three executor-bucket tests intentionally red. The full-bucket acceptance ceremony falsified
that scoping: `region_id` is read for **opposite** purposes by different consumers (emit/path-opt as a
tool; postpass back-refs as an identity), so the conflation cannot be fixed in one field without
breaking a consumer. The deferred split therefore had to be done — and once `tool_index` is
first-class, the clean axes it unlocks (D14 fuzzy routing, per-tool config) and the remaining
boostvoronoi failure modes were folded into the same coherent slice (per user direction).

This matters because: (1) painted/MMU models cannot slice at all until the split lands; (2) the
`region_id`-as-identity back-reference that postpasses depend on must be restored; (3) the painted
FuzzySkin path and the boostvoronoi builder both have latent aborts on real painted geometry.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- **No struct `Default` on `PrintEntity`** — the missing derive is the deliberate compiler checklist for
  the ~43 construction sites; `#[serde(default)]` on the field provides deserialization back-compat
  without a struct `Default`.
- **Schema bump is additive:** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` 1.0.0 → 1.1.0; older
  serialized `PrintEntity`s deserialize (field defaults to `0`).
- **Behavior-neutral when unused:** the per-tool config axis (both emit + region-mapping) is a no-op
  when no `tool_config:` keys are present, so the default-config golden output is unchanged.

## Data and Contract Notes

- WIT `region-id` is a `string`; SDK `RegionId = u64`. The host serializes u64→string and the SDK
  parses back — that round-trip (not a numeric WIT field) is why the guest casts work.
- The finalization-input deep-copy (macro drain) reconstructs full `PrintEntity`s from
  `print-entity-view`, which is the sole reason that record carries `tool-index`.
- `overlay_resolved` writes only fields differing from `ResolvedConfig::default()`; the per-tool
  overlay reuses the global-based `resolve_per_tool_configs` and is applied like the existing paint
  overlay (correct in the common case where global geometry is default).

## Locked Assumptions and Invariants

- `DEFAULT_TOOL = 0` floor, emit `MAX_PLAUSIBLE_TOOLS` guard, and `>1 GiB` allocator tripwire are
  PERMANENT belt-and-suspenders — must not be removed even though the split makes the leak structurally
  impossible (AC-N1, AC-N2 lock this).
- `region_key.region_id` is a PURE region identity post-split; no consumer may store a tool there.
- D14: `SlicedRegion.segment_annotations` is modifier-volume-only; FuzzySkin rides `variant_chain`.

## Risks and Tradeoffs

- Wide blast radius across guests → mitigated by the `Default`-less compiler checklist + a guest
  rebuild + the full bucket after each Part.
- Per-tool overlay reuses the paint-overlay's "global-based" merge → has the same pre-existing
  global-clobber quirk when a user sets a GLOBAL geometry value AND a per-region override; documented,
  consistent with paint, and irrelevant in the common (default-global) case.
