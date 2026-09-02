# 116 — Document the `_mm` suffix as the PnP-provenance key marker

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22, from the 2026-09-01 grilling ruling **Q15(a)**: *"`_mm` is a
deliberate marker — document it"* (no-change to the keys themselves) —
*"Suffix signals a PnP-invented dimensional key with no canonical counterpart;
renamed keys had counterparts."*

The rename workstream (tickets 99–107) stripped `_mm` from two keys that *did*
have canonical counterparts — `support_top_z_distance_mm` → `support_top_z_distance`
(ticket 104) and `ironing_spacing_mm` → `ironing_spacing` (ticket 106). Several
`_mm` keys remain and are **not** rename residue. Without a written rule, the
next sweep strips them too and invents parity that does not exist.

Verified in-tree (2026-09-02) — the PnP-provenance set is exactly four:

- `narrow_loop_length_threshold_mm` — `modules/core-modules/classic-perimeters/classic-perimeters.toml`
- `support_branch_merge_distance_mm` — `modules/core-modules/tree-support-planner/tree-support-planner.toml`
- `support_layer_height_mm` — `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`
- `wave_overhang_anchor_depth_mm` — `modules/core-modules/wave-overhangs/wave-overhangs.toml`

And one key the audit's broad sweep initially missed, which is **not** a member:
`wave_overhang_flow_mm3_per_mm` (`wave-overhangs.toml`) — there the suffix is a
*unit* (mm³/mm), not the provenance marker. That correction is already recorded
in `key-correction-inventory.md` §"Corrections to this document"; the rule must
be written so it does not sweep this key up.

Where it goes: verified this session, **no doc has a key-naming-convention
section**. `docs/03_wit_and_manifest.md` carries a one-line pointer ("Keys follow
the snake_case convention throughout (see CLAUDE.md)") plus the kebab→snake WIT
field rule; `docs/15_config_keys_reference.md` only cross-refs it;
`docs/21_data_defaults_and_fixtures.md` is scoped to test-code struct literals.
The only normative statement anywhere is `CLAUDE.md`'s "Config Key Naming
Convention" section, which covers snake_case and nothing else.

Decide and execute:

1. **Where the convention is written.** `CLAUDE.md`'s existing section is the
   only normative home today and is agent-facing, which is the audience that
   keeps getting this wrong; `docs/03_wit_and_manifest.md` is the fuller
   contract. Pick one as normative and cross-ref from the other — per the repo's
   shared-memory rule this must land in a version-controlled, team-visible file
   either way.
2. **The rule's exact wording**, distinguishing the two things a `_mm` suffix can
   mean: a PnP-invented key with no canonical counterpart (marker — keep) versus
   a unit-bearing name like `mm3_per_mm` (not a marker — also keep, different
   reason). Give the four-key list as the current membership and say it is
   membership-by-rule, not a frozen list.
3. **What a future rename sweep must do**: check for a canonical counterpart
   before stripping a suffix, with the ticket-104/106 renames as the positive
   precedent and these four as the negative one.

Related but separately ruled, do **not** fold in: `support_overhang_angle`
(Q15(b), *"delete key, alias, and both tests"* — ruled rule-out-of-scope) and
`support_branch_merge_distance_mm`'s zero read sites (listed in the audit's
"In-scope keys not ruled on" as a STUB — Q15(a) documented its suffix but did not
rule on its being unimplemented).

Documentation only; changes no queue count and no behaviour.

## Answer
