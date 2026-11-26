# 135 — Author packet — support base-pattern holder

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Author a complex implementation packet for the missing support base-pattern
feature. The Orca `support_base_pattern` key is declared on master and records
the selected pattern as a `traditional-base-pattern:<value>` capability; the
missing piece is the claim-holder/module seam that selects and runs real support
body fillers. Under the map's holder-only rule and ruling Q3, do not re-litigate
the key's removal or restore a 3MF object-metadata alias; implement the holder
and algorithm dispatch.

Re-derive the owner and seam at authoring time. The current tree has live
`support_base_pattern_spacing` reads in the `tree-support` and
`traditional-support` renderers, but no support-base-pattern claim holder or
algorithm dispatch. The packet should build the PnP claim-holder/module seam
needed to select real support body fillers, rather than branching on the Orca
enum inside one existing module. Verify the canonical value set and behavior at
the designated Orca checkout before fixing the shipped subset:

`default`, `rectilinear`, `rectilinear-grid`, `honeycomb`, `lightning`, `hollow`.

At minimum, the packet must cover a canonical-default holder, a non-default
holder with a behavior-changing test, spacing through the selected filler, and
fatal validation for an unmatched configured holder. Keep support interface
pattern selection as a separate gap unless the re-derived seam proves the two
features cannot be separated.

## Constraints

- Use `/spec-packet-generator` and pass `/spec-review <packet> --preflight`.
- Do not count the removed `support_base_pattern` input as covered; selection is
  by a holder key/module claim only.
- Preserve the existing support-family pairing and the live
  `support_base_pattern_spacing` behavior while introducing the new seam.
- The packet must include canonical function evidence and non-default invariant
  tests, with no declaration-only disposition.

## Resolved when

The packet is authored, preflighted, linked from this ticket and the map, and
its implementation scope names the selected holder/module seam plus the
unimplemented canonical values.

## Ruling

User ruling 2026-11-26 (ticket 153, from the ticket-152 audit): master's
`support_base_pattern` manifest declaration and `traditional-base-pattern`
capability stay. The packet owns the missing holder/algorithm seam, not the key's
removal. The restored capability pin is
`traditional_family_tdd::base_interface_obstacle`.
