# T27 serial-floor phase-split cell medians

Medians below use measured rows only (`warmup=false`). The primary tables pool measured runs by fixture × generator × supports × tool across the supplied vintages, as requested; where repairs create multiple vintages, a second table keeps those vintages separate. `R = wall_seconds × 1000 − elapsed_ms`; `tail_gap = elapsed_ms − sum(validation, prepass, per_layer, postpass)`. Times are in seconds or milliseconds as labeled.

Supports-off cells are shown separately and are the untainted comparison cells. For supports-on `s2-base`, vintage status is pre-repair (degraded=true expected); `dev174-before` is pre-repair; `dev174-after` is post-repair (degraded=false expected).

## benchy

| Generator | Supports | Tool | n | Vintage(s) | Median wall (s) | Median CPU (s) | Median R (ms) | Median tail_gap (ms) | Median phases_sum (ms) | Median elapsed (ms) | Wall min–max (s) |
|---|---:|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| arachne | off | pnp-ordinary | 3 | s1-benchy (matched-pair) | 20.4924 | 75.7969 | 171.3 | 958.0 | 19320.0 | 20313.0 | 20.0477–20.8363 |
| arachne | off | pnp-accelerated | 3 | s1-benchy (matched-pair) | 19.2702 | 73.9375 | 180.2 | 922.0 | 18126.0 | 19090.0 | 18.6237–19.7135 |
| arachne | on | pnp-ordinary | 3 | s1-benchy (matched-pair) | 33.2415 | 102.6406 | 202.7 | 1036.0 | 32003.0 | 33039.0 | 30.6057–53.9900 |
| arachne | on | pnp-accelerated | 3 | s1-benchy (matched-pair) | 31.3947 | 100.9062 | 238.7 | 1077.0 | 30079.0 | 31156.0 | 31.0371–35.1936 |
| classic | off | pnp-ordinary | 3 | s1-benchy (matched-pair) | 21.0457 | 131.2344 | 179.7 | 903.0 | 20003.0 | 20866.0 | 20.6065–30.1963 |
| classic | off | pnp-accelerated | 3 | s1-benchy (matched-pair) | 21.0575 | 114.3750 | 187.7 | 883.0 | 19930.0 | 20873.0 | 19.8217–22.8769 |
| classic | on | pnp-ordinary | 3 | s1-benchy (matched-pair) | 34.9175 | 157.9844 | 621.5 | 1298.0 | 33390.0 | 34688.0 | 34.8974–36.7251 |
| classic | on | pnp-accelerated | 3 | s1-benchy (matched-pair) | 33.7130 | 140.2031 | 405.6 | 1171.0 | 31918.0 | 33089.0 | 32.9799–33.8756 |

## base

| Generator | Supports | Tool | n | Vintage(s) | Median wall (s) | Median CPU (s) | Median R (ms) | Median tail_gap (ms) | Median phases_sum (ms) | Median elapsed (ms) | Wall min–max (s) |
|---|---:|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| arachne | off | pnp-ordinary | 3 | s2-base (supports-off/untainted) | 161.5457 | 1017.4062 | 1974.7 | 1036.0 | 158535.0 | 159571.0 | 146.2586–184.7961 |
| arachne | off | pnp-accelerated | 3 | s2-base (supports-off/untainted) | 137.1659 | 947.1562 | 1773.6 | 982.0 | 134579.0 | 135555.0 | 134.4866–450.0960 |
| arachne | on | pnp-ordinary | 4 | s2-base (pre-repair (degraded=true expected)); dev174-after (post-repair (degraded=false expected)) | 463.8336 | 1481.7735 | 2162.5 | 35644.5 | 425145.0 | 461623.5 | 458.1437–467.4000 |
| arachne | on | pnp-accelerated | 4 | s2-base (pre-repair (degraded=true expected)); dev174-after (post-repair (degraded=false expected)) | 460.7151 | 1446.5859 | 2026.1 | 34873.0 | 422693.0 | 458689.0 | 443.8147–474.4035 |
| classic | off | pnp-ordinary | 3 | s2-base (supports-off/untainted) | 251.3431 | 1921.8125 | 2319.9 | 1489.0 | 243678.0 | 245167.0 | 247.4869–251.8201 |
| classic | off | pnp-accelerated | 3 | s2-base (supports-off/untainted) | 212.8699 | 1520.2344 | 2134.2 | 1142.0 | 209616.0 | 210758.0 | 202.6212–214.2186 |
| classic | on | pnp-ordinary | 5 | s2-base (pre-repair (degraded=true expected)); dev174-before (pre-repair); dev174-after (post-repair (degraded=false expected)) | 577.9615 | 2412.3594 | 1851.3 | 35587.0 | 537673.0 | 576125.0 | 538.6113–791.7057 |
| classic | on | pnp-accelerated | 5 | s2-base (pre-repair (degraded=true expected)); dev174-before (pre-repair); dev174-after (post-repair (degraded=false expected)) | 521.5731 | 2018.8281 | 2503.4 | 34806.0 | 477312.0 | 518940.0 | 500.5508–805.5020 |

## Vintage-resolved supports-on results (base)

This split prevents pre-repair and post-repair runs from being mistaken for one homogeneous sample. `degraded` below is the CSV value observed in the contributing runs.

| Generator | Tool | Vintage | Repair status | n | degraded value(s) | Median wall (s) | Median R (ms) | Median tail_gap (ms) | Median phases_sum (ms) | Median elapsed (ms) |
|---|---|---|---|---:|---|---:|---:|---:|---:|---:|
| arachne | pnp-ordinary | s2-base | pre-repair (degraded=true expected) | 3 | true | 460.7149 | 1983.9 | 36606.0 | 422125.0 | 458731.0 |
| arachne | pnp-ordinary | dev174-after | post-repair (degraded=false expected) | 1 | false | 466.9523 | 2436.3 | 8727.0 | 455789.0 | 464516.0 |
| arachne | pnp-accelerated | s2-base | pre-repair (degraded=true expected) | 3 | true | 455.9292 | 2023.2 | 35863.0 | 417777.0 | 453906.0 |
| arachne | pnp-accelerated | dev174-after | post-repair (degraded=false expected) | 1 | false | 474.4035 | 4420.5 | 9396.0 | 460587.0 | 469983.0 |
| classic | pnp-ordinary | s2-base | pre-repair (degraded=true expected) | 3 | true | 577.9615 | 1836.5 | 38032.0 | 537673.0 | 576125.0 |
| classic | pnp-ordinary | dev174-before | pre-repair | 1 | true | 538.6113 | 1851.3 | 33982.0 | 502778.0 | 536760.0 |
| classic | pnp-ordinary | dev174-after | post-repair (degraded=false expected) | 1 | false | 791.7057 | 3400.7 | 17783.0 | 770522.0 | 788305.0 |
| classic | pnp-accelerated | s2-base | pre-repair (degraded=true expected) | 3 | true | 521.5731 | 2503.4 | 41628.0 | 477312.0 | 518940.0 |
| classic | pnp-accelerated | dev174-before | pre-repair | 1 | true | 500.5508 | 1980.8 | 34806.0 | 463764.0 | 498570.0 |
| classic | pnp-accelerated | dev174-after | post-repair (degraded=false expected) | 1 | false | 805.5020 | 5056.0 | 9338.0 | 791108.0 | 800446.0 |

## Observations

- Typical `R` for **benchy / pnp-ordinary**: median 202.6 ms, observed range 165.7–1316.1 ms across measured rows.
- Typical `R` for **benchy / pnp-accelerated**: median 194.9 ms, observed range 172.5–624.0 ms across measured rows.
- Typical `R` for **base / pnp-ordinary**: median 1974.7 ms, observed range 1604.6–7393.1 ms across measured rows.
- Typical `R` for **base / pnp-accelerated**: median 2035.0 ms, observed range 1610.9–5056.0 ms across measured rows.
- **Tail-gap materiality, benchy** (material means median per-run `tail_gap / elapsed_ms` > 5%; per-cell spread shown): arachne/off/pnp-ordinary: not material, median share 4.64% (range 4.40–4.89%); tail range 874.0–993.0 ms; arachne/off/pnp-accelerated: not material, median share 4.94% (range 4.72–5.05%); tail range 911.0–964.0 ms; arachne/on/pnp-ordinary: not material, median share 3.23% (range 3.14–3.26%); tail range 981.0–1751.0 ms; arachne/on/pnp-accelerated: not material, median share 3.46% (range 3.35–4.49%); tail range 1034.0–1569.0 ms; classic/off/pnp-ordinary: not material, median share 4.42% (range 4.14–7.60%); tail range 863.0–2260.0 ms; classic/off/pnp-accelerated: not material, median share 4.50% (range 3.87–4.52%); tail range 879.0–943.0 ms; classic/on/pnp-ordinary: not material, median share 3.74% (range 2.88–3.84%); tail range 1020.0–1318.0 ms; classic/on/pnp-accelerated: not material, median share 3.54% (range 3.51–4.47%); tail range 1151.0–1495.0 ms.
- **Tail-gap materiality, base** (material means median per-run `tail_gap / elapsed_ms` > 5%; per-cell spread shown): arachne/off/pnp-ordinary: not material, median share 0.65% (range 0.65–1.73%); tail range 947.0–3130.0 ms; arachne/off/pnp-accelerated: not material, median share 0.72% (range 0.29–0.74%); tail range 976.0–1305.0 ms; arachne/on/pnp-ordinary: material, median share 7.77% (range 1.88–7.98%); tail range 8727.0–36894.0 ms; arachne/on/pnp-accelerated: material, median share 7.70% (range 2.00–7.96%); tail range 9396.0–36129.0 ms; classic/off/pnp-ordinary: not material, median share 0.61% (range 0.40–0.94%); tail range 990.0–2302.0 ms; classic/off/pnp-accelerated: not material, median share 0.54% (range 0.52–0.80%); tail range 1045.0–1688.0 ms; classic/on/pnp-ordinary: material, median share 6.39% (range 2.26–6.67%); tail range 17783.0–38452.0 ms; classic/on/pnp-accelerated: material, median share 6.98% (range 1.17–8.26%); tail range 9338.0–43746.0 ms.
- **Impossible ordering (wall < elapsed):** none among captures with both values present.
- **dev174 R comparison against `s2-base`:** values below compare each dev174 measured run with the same-cell `s2-base` measured-run median and range.
  - `classic/on/pnp-ordinary` dev174-before: R=1851.3 ms vs `s2-base` median 1836.5 ms (range 1828.1–1915.6 ms).
  - `classic/on/pnp-ordinary` dev174-after: R=3400.7 ms vs `s2-base` median 1836.5 ms (range 1828.1–1915.6 ms).
  - `classic/on/pnp-accelerated` dev174-before: R=1980.8 ms vs `s2-base` median 2503.4 ms (range 1938.8–2633.1 ms).
  - `classic/on/pnp-accelerated` dev174-after: R=5056.0 ms vs `s2-base` median 2503.4 ms (range 1938.8–2633.1 ms).
  - `arachne/on/pnp-ordinary` dev174-after: R=2436.3 ms vs `s2-base` median 1983.9 ms (range 1891.7–2341.0 ms).
  - `arachne/on/pnp-accelerated` dev174-after: R=4420.5 ms vs `s2-base` median 2023.2 ms (range 1927.7–2029.1 ms).
- Capture coverage: 70 existing files, 0 missing-file/path markers, 0 partial captures; every PNP scoreboard row remains in the run CSV.

## Input schema and calculation notes

All four inputs used the same scoreboard header: `run_id`, `fixture`, `generator`, `supports`, `tool`, `run_index`, `warmup`, `wall_seconds`, `cpu_seconds`, `cpu_wall_ratio`, `stderr_path`, `degraded`, `non_fatal_error_count`, and `exit_code` were present. Non-JSON stderr lines and JSON objects without `event=phase_complete` or `event=slice_complete` were ignored. Duplicate recognized events, if present, are counted and the last valid event is used for that field.
