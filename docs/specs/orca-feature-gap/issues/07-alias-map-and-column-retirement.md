# 07 — Document the Orca→Pinch alias map and retire the hand-maintained ❌ column

Type: grilling
Status: open
Blocked by: — (03 resolved)
Map: ../map.md

## Question

Should the Orca→Pinch key alias map be written down and machine-checked, and
should the hand-maintained "In Codebase" column of `docs/ORCA_CONFIG_REFERENCE.md`
be replaced by generated output?

Ticket 01 established both halves of the problem with numbers: the column is
wrong on 66 of 574 FFF keys (11.5%, in both directions), and 62 declared Pinch
keys have no exact Orca counterpart because the project silently renamed the
vocabulary (`wall_count`/`wall_loops`, `first_layer_height`/
`initial_layer_print_height`, `wipe_tower_*`/`prime_tower_*`, and more). The
alias map exists only implicitly, scattered across module manifests.

This matters to the destination, not just to tidiness: **without a checked alias
map, every future packet in this queue re-litigates whether a key is a genuine
gap or a rename** — which is exactly the 57 false-gap keys 01 caught, and which
would otherwise have been specced as work.

Settle:

- Where does the alias map live — a new TOML beside `docs/config/host-keys.toml`,
  a column in `docs/15_config_keys_reference.md`, or module manifest metadata?
- Is it generated, hand-written, or hand-written-and-checked?
- Does `cargo xtask gen-config-docs --check` gain a gate that fails CI when the
  reference's presence flags disagree with the live registries? (It already
  parses that file for the `Default` column, so the plumbing exists.)
- Is retiring the column in scope for *this* map — it is tooling, not a feature
  packet — or is it a prerequisite deliverable that earns its place by
  unblocking the packet queue's correctness?

That last bullet is a scoping question: if the answer is "out of scope", close
this ticket into the map's **Out of scope** section rather than resolving it.

## Update after 03

The alias map's content now exists: 25 adjudicated renames and 34
Pinch-specific keys, in [`03-asset-scoped-gap.md`](./03-asset-scoped-gap.md).
This ticket is unblocked, and 03 widened it — the problem is not only
Orca↔Pinch drift but **internal** inconsistency:

- `modules/core-modules/fuzzy-skin` declares bare `thickness`,
  `point_distance`, `apply_to_all`. No declared key anywhere in the tree
  contains a dot, so there is no namespacing convention protecting a name that
  generic in a shared config space.
- The two ironing modules disagree with each other: `top-surface-ironing` uses
  `ironing_flow` + `ironing_spacing_mm`, `support-surface-ironing` uses
  `ironing_flow_rate` + `ironing_spacing`. Four spellings, one concept.
- `infill_density`, `infill_speed` and `infill_overlap` are declared *alongside*
  the Orca-named `sparse_infill_density`, `sparse_infill_speed` and
  `infill_wall_overlap`, all live. Two spellings of the same setting.

So the ruling this ticket needs is wider than first written: does the effort
also standardise Pinch's own key vocabulary, or only document the mapping to
Orca's? Standardising is a rename with blast radius across manifests, guest
rebuilds, and `docs/15_config_keys_reference.md`.
