# Rejected: windowing both `deep_infill_area` offsets

`full-window.patch` is the first candidate: it clipped **both** the positive
`deep_infill_clip_area` and the negative erosion operand to the candidate-bbox
window. The G-code moved on all four classic cells sampled:

| cell | reference SHA-256 | full-window SHA-256 |
| --- | --- | --- |
| benchy classic-off | `90638bf3ea9e592d573b223c3eb9af64473efe804578d2478adfbbc5f0e67008` | `d4078b3094b3e120cb659f4ac1808ded69c24eed7d41e3eed80d1371672c8c07` |
| benchy classic-on | `711c6334e397e5ae6911b15bfc71d42add9e93822cddd0a29391db6ef531d556` | `1dc1e5d97e8336a059076f4aec7edc41334afdce3c978d509ae63719efc22bf1` |
| base classic-off | `b3b4ac5e885c7f637b98f79eca21901a41d85739627d4dcb40820663c15e90ef` | `f351cda8065f3381e4779bb3d85c6857cfc51eec1f46c9923bd50cce25dd2a65` |
| base classic-on | `4e31a2db368ff80f388512211ae824b9660342a465c87f093f6182023e34eb4e` | `8275a124ceac8725cb063378987c999cfc736f6f3befa6dd156a26beef7b9ab0` |

`audit_delta.py` (the ticket-123 auditor, copied into the parent directory)
reports **0 structural geometry lines** on both audited cells, so the delta is
confined to the bridge role — but it is not rounding noise. On benchy
classic-off it changes **8 Internal Bridge layers**, 992 geometry lines, with a
maximum paired coordinate shift of **22.09 mm**; on base classic-off it changes
**5** Internal Bridge geometry lines. That movement is the positive offset's
whole-layer consumer in `internal_bridge_angles`: the angle step intersects the
sparse `expansion_area` with `deep_infill_clip_area` and samples the whole-layer
boundary anchor set, so clipping that operand changes bridge directions. The
shipping candidate therefore windows the erosion only.
