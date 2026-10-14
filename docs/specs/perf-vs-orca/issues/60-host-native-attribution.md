# host:native attribution: name the per-layer native geometry bucket

Type: task
Label: wayfinder:task
Status: open
Assignee: unassigned — surfaced by the 2026-10-09 perf refresh
([evidence](../evidence/perf-refresh-2026-10-09/FINDINGS.md)); human selection
pending
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

The fuel profile's `host:native` fallback bucket (`UNATTRIBUTED_NATIVE_MODULE`,
`crates/slicer-runtime/src/profiling_report.rs`) carries every marked
`polygon_ops` call no host-built-in bracket claimed — e.g. native geometry
reached from the per-layer tier. Fresh attribution at HEAD (2026-10-09
refresh):

| cell | host:native (accumulated) | offset | clip_polygons |
| --- | ---: | ---: | ---: |
| base classic supports-on | 398.2 s | 192.9 s / 58,391 calls | 205.4 s / 115,874 calls |
| base classic supports-off | 197.2 s | 102.9 s / 5,196 calls | 94.3 s / 18,678 calls |
| benchy classic supports-on | 22.9 s | 13.6 s / 15,733 calls | 9.3 s / 29,366 calls |
| benchy classic supports-off | 12.4 s | 7.4 s / 2,454 calls | 5.0 s / 8,193 calls |

The supports-off cells carry a stable per-layer shape (≈10.5 `offset` + ≈37.7
`clip_polygons` calls per layer) nearly independent of object size, so at least
part of the bucket is a per-layer host path rather than the (already-mined)
support machine. **No wall share is known**: the accumulated figures could be
spread across the ~11x-parallel per-layer map or be serial work — do not size
this as wall before measuring.

Deliverable: attribute the bucket at HEAD — split by owner and run a same-run
wall bracket per owner — and name any reducible shape. Candidate owners to
discriminate include `sync_perimeter_infill_areas_into_slice`
(`crates/slicer-runtime/src/region_partition.rs`, run on every Perimeters and
PerimetersPostProcess commit), region preparation (`prepare_regions`,
`crates/slicer-wasm-host/src/marshal/prepared.rs`, covered by ticket 05's arena
cache), anchored-event synthesis, and the per-layer arena/commit paths.

## Acceptance

Attribution only: owner split + per-owner brackets at the matched jobs, with
clean outputs. A reduction candidate graduates separately into an execution
ticket with the standing paired A/B; a "nothing reducible" verdict resolves
this ticket with the owner split as its evidence. No candidate, commit or
default change in this take unless the human extends it.

## Notes

- The supports-on part of the bucket likely overlaps the support-geometry
  machine mined by tickets 22/32/44/53/55/57; the supports-off part is the
  unexplained side and the take's focus.
- Evidence: `evidence/perf-refresh-2026-10-09/FINDINGS.md` (host table and
   per-layer counts).

## Comments

### Benchy bucket refresh — 2026-10-13 (no owner probe or take started)

The [Benchy current bottleneck refresh](61-benchy-current-bottleneck-refresh.md)
measures supports-off `host:native` at 11.797 s Classic / 5.716 s Arachne
accumulated thread-time, supports-on 21.755/15.595 s. These fresh profiles
broaden the generator coverage, not the attribution: owners and wall shares
remain unresolved, so this ticket is retained rather than duplicated.
Evidence: [fresh findings](../evidence/perf-refresh-2026-10-13/FINDINGS.md).
