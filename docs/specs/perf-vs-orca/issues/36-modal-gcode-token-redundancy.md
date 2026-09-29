# Modal G-code Z/F token redundancy and speed gate

Type: task
Status: open
Blocked by: 28

## Question

Does emitting modal Z/feed state only when necessary preserve the printing
job, and does it improve **median uninstrumented wall with corroborating
process CPU**? [Classic output-volume surplus](28-classic-output-volume-surplus.md)
found that PNP repeats Z and F on every XY+E move in both generators,
including when unchanged; historical Orca captures generally omit them.
The measured Z/F token budget is **not** a safe-removal count or a time saving.

First prove a semantics-preserving state machine through `GCodeSerializer`'s
move rendering (`crates/slicer-gcode/src/serialize.rs`), including travel,
retraction, raw G-code, G90/G91, G92, layer changes and tool commands. Build an
independent G-code interpreter/round-trip oracle on tricky fixtures; the
output itself will be byte-different, so a byte-equality check is insufficient.
Then run matched ordinary+accelerated pairs with the map's freshness, starvation
and output-disclosure gates and return a measured keep/drop recommendation to
the human. Do not infer a speed win from file-size arithmetic alone. Evidence:
`evidence/t28-output-volume/FINDINGS.md`.
