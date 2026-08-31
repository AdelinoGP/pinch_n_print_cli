# Task Map: fuzzy-skin-keys

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes an earlier packet. **This packet emits the template's own skip clause:** it is a single-coherent-slice packet with `task_ids: []` (queue precedent — packets 234a, 253, 254, 255, 256, 257, 258), so the `docs/07` crosswalk is N-A. Implementation is recorded against wayfinder ticket 14 (`docs/specs/orca-feature-gap/issues/14-author-packet-p07-others-fuzzy-skin-fuzzy-skin.md`).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| — (queue packet, `task_ids: []`) | Steps 1–4 | `docs/15_config_keys_reference.md` (generated) | `modules/core-modules/fuzzy-skin/{fuzzy-skin.toml,src/lib.rs,tests/*}` + scheduler/runtime integration arms | `OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp` (`should_fuzzify`, `fuzzy_polyline`, `fuzzy_extrusion_line`, `get_noise_module`), `PrintConfig.cpp` | S/M | Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; no TASK rows; re-derive the crosswalk question at completion time per the ledger-fact rule |
