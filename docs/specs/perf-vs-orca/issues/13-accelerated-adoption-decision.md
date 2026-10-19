# Accelerated perimeter-spatial adoption decision

Type: grilling
Status: resolved
Assignee: current wayfinder session, 2026-09-30
Blocked by: 11

## Question

Should the packet-254 accelerated perimeter-spatial build become the
production default (finishing its acceptance campaign), or does production
stay on the ordinary build with accelerated kept as a measurement mode?

Take this to the human with the matched scoreboard's two-mode columns on the
table ([Matched-pair rig and first scoreboard](11-matched-pair-rig-and-scoreboard.md)).
The decision settles which mode future production claims quote and how the
matrix cells are ultimately judged.

Considerations on file: the measured −35.8% total guest fuel and ~1.9× query
shrink (necessary, likely not sufficient —
[Accelerated-mode pair](09-accelerated-mode-pair.md)); the controlled-toolchain
build pipeline and its acceptance gates (`docs/23_controlled_perimeter_builds.md`);
the mode-specific-wins policy (a one-sided improvement needs explicit human
acceptance); and the recipe traps around the accelerated snapshot.

Record the human's decision under `## Answer` — adoption or not, with the
acceptance campaign's remaining checklist state.

## Resolution comment — 2026-09-30

The human chose **pursue accelerated adoption**, then confirmed conditional
adoption under the existing controlled-build acceptance policy. Accelerated
is the intended production mode; ordinary remains the actual default until
fresh acceptance passes. This is not permission to change build defaults now.

The gate is `docs/23_controlled_perimeter_builds.md`'s campaign: verified
mode-specific artifacts, reference exactness and dispatch/status validation,
and strict favorable CPU and wall sample separation in every campaign cell.
An inconclusive result returns to the human; do not weaken the gate or
automatically retry. Paired ordinary + accelerated measurements continue.
Until adoption, production-default claims refer to ordinary and accelerated
claims remain explicitly mode-labelled. Adoption does not replace the map's
separate matched eight-cell Orca win criterion.

No acceptance campaign or freshness check ran in this decision session.
The historical matched scoreboard predates recent correctness fixes and is
not fresh adoption evidence. Remaining preparation and verification are
carried by [Accelerated production adoption acceptance campaign](38-accelerated-adoption-acceptance-campaign.md).
