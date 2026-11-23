# t69: Clipper engine headroom campaign

**Verdict: measured headroom; a value-exact Rust optimization candidate is justified, not adopted.**
The measured slow opclasses are the candidate direction. No engine-internal profiling was done; no allocation, sweep, layout, or other internal cost center is inferred.

## Method and reproduction

`campaign.py` verifies binary/driver hashes, complete pristine Rust/C++ source inventories and the isolated lock before/after execution; no rebuild occurred. Seal/toolchain/build commands and corpus hashes are preserved in `results/provenance.json`.
All 295 cases from all four t68-site files and all four t55 files ran twice: complete file/case order then globally reversed file/case order. A fresh driver process per case excludes inter-case engine state; forward runs Rust then C++, reverse C++ then Rust. Each driver performs one warmup and five timed repeats. No CPU pinning or process-level replicated A/B; order and engine precedence change together, so spread is descriptive, not a confidence interval.
Case outputs are resumable under `.local-artifacts/perf-vs-orca/t69-clipper-engine/raw/t69-campaign/{forward,reverse}/<file-stem>/<original-index>/{rust,cpp}.jsonl`; per-case timeout is 180 seconds. Parsing/flattening, output traversal/hash and output-tree destruction are outside the engine timer; engine creation/add/execute/destruction are inside (HARNESS.md).
The first attempt stopped on a zero-duration repeat in an empty boolean case; the runner was corrected to retain clock-resolution zeros, resumed without discarding completed cases, and completed. The rejected initial response was not saved; that one engine/case was re-invoked. No timing-driven exclusions.
Commands (repository root; long runner detached, tee logs):

```bash
set -o pipefail
python -u docs/specs/perf-vs-orca/evidence/t69-clipper-engine/campaign.py 2>&1 | tee .local-artifacts/perf-vs-orca/t69-clipper-engine/raw/t69-campaign.log
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/analyze.py
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/verify.py
```

All numbers below derive from `results/{forward,reverse,summary,provenance}.json`. Case time = median of the five timed repeats. Each table engine time = median across those case medians; ratio = Rust median / C++ median. Spread = absolute difference of the two ratios, in ratio points. These are shape-sample statistics, not production frequency weights. The t55 ordinary/accelerated duplicates remain; they are not independent workloads.

## Headline opclasses

| Group | Cases | Forward Rust/C++ | Reverse Rust/C++ | Absolute spread | Forward Rust / C++ median ns | Reverse Rust / C++ median ns |
|---|---:|---:|---:|---:|---:|---:|
| difference | 68 | 1.7608 | 1.5983 | 0.1625 | 499800.0 / 283850.0 | 469250.0 / 293600.0 |
| intersection | 34 | 1.9998 | 1.9409 | 0.0589 | 2307950.0 / 1154100.0 | 2336300.0 / 1203750.0 |
| offset | 177 | 2.0889 | 2.0336 | 0.0553 | 6780100.0 / 3245800.0 | 6783800.0 / 3335900.0 |
| union | 16 | 2.6280 | 2.6084 | 0.0196 | 104350600.0 / 39707950.0 | 106984150.0 / 41015650.0 |

## Sites (pooled across corpus files)

| Group | Cases | Forward Rust/C++ | Reverse Rust/C++ | Absolute spread | Forward Rust / C++ median ns | Reverse Rust / C++ median ns |
|---|---:|---:|---:|---:|---:|---:|
| 550 | 16 | 2.6280 | 2.6084 | 0.0196 | 104350600.0 / 39707950.0 | 106984150.0 / 41015650.0 |
| A0 | 23 | 2.2557 | 2.3161 | 0.0603 | 15076000.0 / 6683400.0 | 15339800.0 / 6623200.0 |
| A1 | 23 | 2.1219 | 1.7813 | 0.3406 | 18881300.0 / 8898100.0 | 18980200.0 / 10655200.0 |
| B | 22 | 2.0994 | 2.1898 | 0.0904 | 10958550.0 / 5219850.0 | 12067200.0 / 5510650.0 |
| C | 22 | 1.9378 | 2.0638 | 0.1260 | 4889350.0 / 2523150.0 | 5467400.0 / 2649200.0 |
| D | 22 | 1.6073 | 1.6385 | 0.0311 | 1873600.0 / 1165650.0 | 1897350.0 / 1158000.0 |
| E | 23 | 2.2916 | 2.1678 | 0.1237 | 11295200.0 / 4929000.0 | 11184800.0 / 5159400.0 |
| F | 23 | 2.1912 | 1.9959 | 0.1953 | 5748900.0 / 2623600.0 | 5685000.0 / 2848300.0 |
| G | 18 | 2.0050 | 2.0546 | 0.0496 | 4394850.0 / 2191950.0 | 4418600.0 / 2150600.0 |
| H | 23 | 1.4166 | 1.6770 | 0.2604 | 1314600.0 / 928000.0 | 1298000.0 / 774000.0 |
| J | 23 | 1.6442 | 1.1782 | 0.4660 | 17100.0 / 10400.0 | 11900.0 / 10100.0 |
| K | 23 | 2.3033 | 2.1671 | 0.1362 | 8172700.0 / 3548300.0 | 8157500.0 / 3764300.0 |
| RP1 | 17 | 2.1144 | 1.9835 | 0.1309 | 2385500.0 / 1128200.0 | 2287800.0 / 1153400.0 |
| RP2 | 17 | 1.8902 | 1.9016 | 0.0114 | 2230400.0 / 1180000.0 | 2384800.0 / 1254100.0 |

## Corpus files

| Group | Cases | Forward Rust/C++ | Reverse Rust/C++ | Absolute spread | Forward Rust / C++ median ns | Reverse Rust / C++ median ns |
|---|---:|---:|---:|---:|---:|---:|
| corpus/t55/accelerated-base.cases | 4 | 2.2714 | 2.2213 | 0.0501 | 305099800.0 / 134323250.0 | 304290400.0 / 136989100.0 |
| corpus/t55/accelerated-benchy.cases | 4 | 2.8594 | 2.6509 | 0.2085 | 11301550.0 / 3952400.0 | 11426200.0 / 4310300.0 |
| corpus/t55/ordinary-base.cases | 4 | 2.2654 | 2.2438 | 0.0216 | 304915400.0 / 134597400.0 | 306464700.0 / 136585000.0 |
| corpus/t55/ordinary-benchy.cases | 4 | 2.7487 | 2.8110 | 0.0623 | 10379400.0 / 3776100.0 | 10420700.0 / 3707050.0 |
| corpus/t68sites/base-region-prep.corpus | 28 | 1.7635 | 1.9914 | 0.2279 | 1964150.0 / 1113800.0 | 1903850.0 / 956050.0 |
| corpus/t68sites/base-t68sites.corpus | 170 | 1.8884 | 2.0193 | 0.1309 | 5634150.0 / 2983600.0 | 5937700.0 / 2940450.0 |
| corpus/t68sites/benchy-region-prep.corpus | 6 | 2.0744 | 1.7874 | 0.2870 | 2394050.0 / 1154100.0 | 2423000.0 / 1355600.0 |
| corpus/t68sites/benchy-t68sites.corpus | 75 | 1.7601 | 1.6552 | 0.1049 | 1414100.0 / 803400.0 | 1414200.0 / 854400.0 |

## Corpus × opcode

| Group | Cases | Forward Rust/C++ | Reverse Rust/C++ | Absolute spread | Forward Rust / C++ median ns | Reverse Rust / C++ median ns |
|---|---:|---:|---:|---:|---:|---:|
| corpus/t55/accelerated-base.cases/union | 4 | 2.2714 | 2.2213 | 0.0501 | 305099800.0 / 134323250.0 | 304290400.0 / 136989100.0 |
| corpus/t55/accelerated-benchy.cases/union | 4 | 2.8594 | 2.6509 | 0.2085 | 11301550.0 / 3952400.0 | 11426200.0 / 4310300.0 |
| corpus/t55/ordinary-base.cases/union | 4 | 2.2654 | 2.2438 | 0.0216 | 304915400.0 / 134597400.0 | 306464700.0 / 136585000.0 |
| corpus/t55/ordinary-benchy.cases/union | 4 | 2.7487 | 2.8110 | 0.0623 | 10379400.0 / 3776100.0 | 10420700.0 / 3707050.0 |
| corpus/t68sites/base-region-prep.corpus/intersection | 28 | 1.7635 | 1.9914 | 0.2279 | 1964150.0 / 1113800.0 | 1903850.0 / 956050.0 |
| corpus/t68sites/base-t68sites.corpus/difference | 47 | 1.5293 | 1.2973 | 0.2320 | 863000.0 / 564300.0 | 890500.0 / 686400.0 |
| corpus/t68sites/base-t68sites.corpus/offset | 123 | 2.1324 | 2.1496 | 0.0173 | 10852500.0 / 5089400.0 | 11184800.0 / 5203100.0 |
| corpus/t68sites/benchy-region-prep.corpus/intersection | 6 | 2.0744 | 1.7874 | 0.2870 | 2394050.0 / 1154100.0 | 2423000.0 / 1355600.0 |
| corpus/t68sites/benchy-t68sites.corpus/difference | 21 | 1.4207 | 1.5326 | 0.1119 | 381600.0 / 268600.0 | 448600.0 / 292700.0 |
| corpus/t68sites/benchy-t68sites.corpus/offset | 54 | 1.7816 | 1.8664 | 0.0848 | 1694400.0 / 951050.0 | 1777500.0 / 952350.0 |

## Corpus × site

| Group | Cases | Forward Rust/C++ | Reverse Rust/C++ | Absolute spread | Forward Rust / C++ median ns | Reverse Rust / C++ median ns |
|---|---:|---:|---:|---:|---:|---:|
| corpus/t55/accelerated-base.cases/550 | 4 | 2.2714 | 2.2213 | 0.0501 | 305099800.0 / 134323250.0 | 304290400.0 / 136989100.0 |
| corpus/t55/accelerated-benchy.cases/550 | 4 | 2.8594 | 2.6509 | 0.2085 | 11301550.0 / 3952400.0 | 11426200.0 / 4310300.0 |
| corpus/t55/ordinary-base.cases/550 | 4 | 2.2654 | 2.2438 | 0.0216 | 304915400.0 / 134597400.0 | 306464700.0 / 136585000.0 |
| corpus/t55/ordinary-benchy.cases/550 | 4 | 2.7487 | 2.8110 | 0.0623 | 10379400.0 / 3776100.0 | 10420700.0 / 3707050.0 |
| corpus/t68sites/base-region-prep.corpus/RP1 | 14 | 2.2080 | 1.9441 | 0.2640 | 2188950.0 / 991350.0 | 1903850.0 / 979300.0 |
| corpus/t68sites/base-region-prep.corpus/RP2 | 14 | 1.5940 | 1.9983 | 0.4044 | 1964150.0 / 1232250.0 | 1910500.0 / 956050.0 |
| corpus/t68sites/base-t68sites.corpus/A0 | 16 | 2.1068 | 2.0648 | 0.0420 | 24638350.0 / 11694900.0 | 24709800.0 / 11967450.0 |
| corpus/t68sites/base-t68sites.corpus/A1 | 16 | 2.2332 | 2.0928 | 0.1404 | 28502900.0 / 12763300.0 | 28437150.0 / 13588400.0 |
| corpus/t68sites/base-t68sites.corpus/B | 15 | 2.0266 | 1.9102 | 0.1164 | 15266300.0 / 7533000.0 | 14997400.0 / 7851300.0 |
| corpus/t68sites/base-t68sites.corpus/C | 15 | 2.0170 | 2.0676 | 0.0505 | 7219400.0 / 3579200.0 | 7314300.0 / 3537600.0 |
| corpus/t68sites/base-t68sites.corpus/D | 15 | 1.5118 | 1.6327 | 0.1209 | 2933800.0 / 1940600.0 | 3075700.0 / 1883800.0 |
| corpus/t68sites/base-t68sites.corpus/E | 16 | 2.2211 | 2.1362 | 0.0848 | 18734550.0 / 8434900.0 | 18100700.0 / 8473150.0 |
| corpus/t68sites/base-t68sites.corpus/F | 16 | 2.1061 | 2.0497 | 0.0564 | 9950450.0 / 4724550.0 | 9779550.0 / 4771100.0 |
| corpus/t68sites/base-t68sites.corpus/G | 13 | 2.0226 | 2.0848 | 0.0623 | 4955900.0 / 2450300.0 | 5071800.0 / 2432700.0 |
| corpus/t68sites/base-t68sites.corpus/H | 16 | 1.7258 | 1.5698 | 0.1561 | 2005500.0 / 1162050.0 | 1889200.0 / 1203500.0 |
| corpus/t68sites/base-t68sites.corpus/J | 16 | 1.7400 | 1.3424 | 0.3976 | 17400.0 / 10000.0 | 12350.0 / 9200.0 |
| corpus/t68sites/base-t68sites.corpus/K | 16 | 2.2381 | 2.0975 | 0.1406 | 12927800.0 / 5776250.0 | 12412500.0 / 5917900.0 |
| corpus/t68sites/benchy-region-prep.corpus/RP1 | 3 | 2.1144 | 2.1339 | 0.0194 | 2385500.0 / 1128200.0 | 2461200.0 / 1153400.0 |
| corpus/t68sites/benchy-region-prep.corpus/RP2 | 3 | 2.0361 | 1.5309 | 0.5052 | 2402600.0 / 1180000.0 | 2384800.0 / 1557800.0 |
| corpus/t68sites/benchy-t68sites.corpus/A0 | 7 | 2.0344 | 2.1427 | 0.1082 | 3437400.0 / 1689600.0 | 3561800.0 / 1662300.0 |
| corpus/t68sites/benchy-t68sites.corpus/A1 | 7 | 1.9827 | 2.0361 | 0.0534 | 3608200.0 / 1819800.0 | 3832000.0 / 1882000.0 |
| corpus/t68sites/benchy-t68sites.corpus/B | 7 | 1.7938 | 1.7686 | 0.0252 | 2301500.0 / 1283000.0 | 2567900.0 / 1451900.0 |
| corpus/t68sites/benchy-t68sites.corpus/C | 7 | 1.4990 | 1.6772 | 0.1783 | 1153600.0 / 769600.0 | 1271000.0 / 757800.0 |
| corpus/t68sites/benchy-t68sites.corpus/D | 7 | 1.2822 | 1.2490 | 0.0333 | 542900.0 / 423400.0 | 605500.0 / 484800.0 |
| corpus/t68sites/benchy-t68sites.corpus/E | 7 | 1.9009 | 2.1725 | 0.2716 | 1882300.0 / 990200.0 | 2100200.0 / 966700.0 |
| corpus/t68sites/benchy-t68sites.corpus/F | 7 | 1.7311 | 1.6126 | 0.1185 | 1409500.0 / 814200.0 | 1451200.0 / 899900.0 |
| corpus/t68sites/benchy-t68sites.corpus/G | 5 | 2.0263 | 1.5769 | 0.4494 | 1238900.0 / 611400.0 | 1092800.0 / 693000.0 |
| corpus/t68sites/benchy-t68sites.corpus/H | 7 | 1.6125 | 1.5789 | 0.0336 | 558400.0 / 346300.0 | 610700.0 / 386800.0 |
| corpus/t68sites/benchy-t68sites.corpus/J | 7 | 1.6442 | 1.1782 | 0.4660 | 17100.0 / 10400.0 | 11900.0 / 10100.0 |
| corpus/t68sites/benchy-t68sites.corpus/K | 7 | 1.8779 | 1.8667 | 0.0112 | 1613100.0 / 859000.0 | 1648300.0 / 883000.0 |

## Case-count-weighted aggregates

Within each group below, partition by corpus file/opcode and take the arithmetic mean of stratum ratios weighted by stratum case counts. This is deliberately distinct from the pooled median ratio above; neither is a production-time weight.

| Group | Forward weighted ratio | Reverse weighted ratio | Absolute spread |
|---|---:|---:|---:|
| all | 1.9071 | 1.9137 | 0.0066 |
| file/corpus/t55/accelerated-base.cases | 2.2714 | 2.2213 | 0.0501 |
| file/corpus/t55/accelerated-benchy.cases | 2.8594 | 2.6509 | 0.2085 |
| file/corpus/t55/ordinary-base.cases | 2.2654 | 2.2438 | 0.0216 |
| file/corpus/t55/ordinary-benchy.cases | 2.7487 | 2.8110 | 0.0623 |
| file/corpus/t68sites/base-region-prep.corpus | 1.7635 | 1.9914 | 0.2279 |
| file/corpus/t68sites/base-t68sites.corpus | 1.9656 | 1.9140 | 0.0516 |
| file/corpus/t68sites/benchy-region-prep.corpus | 2.0744 | 1.7874 | 0.2870 |
| file/corpus/t68sites/benchy-t68sites.corpus | 1.6806 | 1.7730 | 0.0924 |
| opcode/difference | 1.4958 | 1.3700 | 0.1258 |
| opcode/intersection | 1.8183 | 1.9554 | 0.1370 |
| opcode/offset | 2.0254 | 2.0632 | 0.0379 |
| opcode/union | 2.5362 | 2.4817 | 0.0545 |
| site/550 | 2.5362 | 2.4817 | 0.0545 |
| site/A0 | 2.0848 | 2.0885 | 0.0037 |
| site/A1 | 2.1570 | 2.0755 | 0.0814 |
| site/B | 1.9525 | 1.8651 | 0.0874 |
| site/C | 1.8522 | 1.9434 | 0.0912 |
| site/D | 1.4388 | 1.5106 | 0.0719 |
| site/E | 2.1236 | 2.1473 | 0.0237 |
| site/F | 1.9920 | 1.9167 | 0.0753 |
| site/G | 2.0236 | 1.9438 | 0.0799 |
| site/H | 1.6913 | 1.5725 | 0.1188 |
| site/J | 1.7109 | 1.2924 | 0.4184 |
| site/K | 2.1285 | 2.0272 | 0.1012 |
| site/RP1 | 2.1915 | 1.9776 | 0.2139 |
| site/RP2 | 1.6720 | 1.9158 | 0.2439 |

Case-count-weighted mean of file/opcode ratios: forward **1.9071**, reverse **1.9137**. Weights are sampled case counts, not invocation frequencies or time. Pooled median ratios are separately 1.8737 / 1.7251.

## Engine times and scaling

| Opclass | Forward Rust / C++ ns | Reverse Rust / C++ ns | Forward case ratio min / median / max | Reverse case ratio min / median / max |
|---|---:|---:|---|---|
| difference | 499800.0 / 283850.0 | 469250.0 / 293600.0 | 0.7838 / 1.6276 / 3.0142 | 0.7829 / 1.5002 / 2.6940 |
| intersection | 2307950.0 / 1154100.0 | 2336300.0 / 1203750.0 | 1.0358 / 2.0596 / 3.0839 | 1.1131 / 2.1085 / 2.6697 |
| offset | 6780100.0 / 3245800.0 | 6783800.0 / 3335900.0 | 1.4032 / 2.0740 / 2.8924 | 1.2906 / 2.0441 / 3.2183 |
| union | 104350600.0 / 39707950.0 | 106984150.0 / 41015650.0 | 2.1075 / 2.6303 / 2.9913 | 2.0221 / 2.4979 / 2.8956 |

| Pass / opclass | Size third | Cases | Input vertex range | Median case ratio | Rust / C++ median ns |
|---|---|---:|---|---:|---:|
| forward / difference | small | 22 | 0–169 | 1.7624 | 8200.0 / 5100.0 |
| forward / difference | middle | 23 | 204–3322 | 1.5221 | 488000.0 / 268600.0 |
| forward / difference | large | 23 | 3547–11132 | 1.6209 | 2910100.0 / 1933300.0 |
| forward / intersection | small | 11 | 350–4307 | 2.0060 | 83100.0 / 31400.0 |
| forward / intersection | middle | 11 | 4307–11079 | 2.0361 | 2230400.0 / 1128200.0 |
| forward / intersection | large | 12 | 13909–32058 | 2.2271 | 32581800.0 / 12921850.0 |
| forward / offset | small | 59 | 65–1299 | 2.0788 | 884400.0 / 496600.0 |
| forward / offset | middle | 59 | 1310–3189 | 2.0386 | 7293500.0 / 3389600.0 |
| forward / offset | large | 59 | 3277–9239 | 2.0977 | 19813700.0 / 9338200.0 |
| forward / union | small | 5 | 3995–15758 | 2.7595 | 6377900.0 / 2430300.0 |
| forward / union | middle | 5 | 15758–41120 | 2.7852 | 16582400.0 / 5953800.0 |
| forward / union | large | 6 | 63638–102400 | 2.3214 | 392726800.0 / 169913150.0 |
| reverse / difference | small | 22 | 0–169 | 1.4523 | 7000.0 / 4400.0 |
| reverse / difference | middle | 23 | 204–3322 | 1.4786 | 448600.0 / 292700.0 |
| reverse / difference | large | 23 | 3547–11132 | 1.5718 | 3075700.0 / 1821900.0 |
| reverse / intersection | small | 11 | 350–4307 | 2.0712 | 85800.0 / 39200.0 |
| reverse / intersection | middle | 11 | 4307–11079 | 2.0526 | 2287800.0 / 1153400.0 |
| reverse / intersection | large | 12 | 13909–32058 | 2.3964 | 29468550.0 / 11678300.0 |
| reverse / offset | small | 59 | 65–1299 | 2.0162 | 848600.0 / 461500.0 |
| reverse / offset | middle | 59 | 1310–3189 | 2.0443 | 7474700.0 / 3405300.0 |
| reverse / offset | large | 59 | 3277–9239 | 2.0565 | 19160000.0 / 9487700.0 |
| reverse / union | small | 5 | 3995–15758 | 2.6728 | 6374500.0 / 2301700.0 |
| reverse / union | middle | 5 | 15758–41120 | 2.7982 | 16678100.0 / 5814500.0 |
| reverse / union | large | 6 | 63638–102400 | 2.3022 | 390826200.0 / 171953950.0 |

Median statistics are resistant to a few very slow cases; the per-case distributions and input-size thirds disclose shape dependence and timer-floor extremes rather than asserting a fitted complexity law. A tiny-case extreme is not a production speedup. Per-case medians and input path/ring/vertex counts remain in the results files.

- forward: zero repeats Rust 0, C++ 10; undefined case ratios (zero C++ median) 2. They remain in group medians and agreement denominators; undefined individual ratios alone are omitted from the ratio distribution.
- reverse: zero repeats Rust 4, C++ 11; undefined case ratios (zero C++ median) 2. They remain in group medians and agreement denominators; undefined individual ratios alone are omitted from the ratio distribution.

## Cross-engine value agreement

Ordered FNV coordinate hash is not geometric equivalence: rotation/orientation/component order differences can disagree; collisions remain possible. Shape counts are checked separately. Known Rust 1.1.0 versus C++ 1.5.2 differences (ticket 31) are disclosed, not root-caused. This is NOT production byte-identity qualification.

| Corpus / opcode | Cases | Forward hash matches | Reverse hash matches | Forward shape matches | Reverse shape matches |
|---|---:|---:|---:|---:|---:|
| corpus/t55/accelerated-base.cases/union | 4 | 4 | 4 | 4 | 4 |
| corpus/t55/accelerated-benchy.cases/union | 4 | 4 | 4 | 4 | 4 |
| corpus/t55/ordinary-base.cases/union | 4 | 4 | 4 | 4 | 4 |
| corpus/t55/ordinary-benchy.cases/union | 4 | 4 | 4 | 4 | 4 |
| corpus/t68sites/base-region-prep.corpus/intersection | 28 | 24 | 24 | 24 | 24 |
| corpus/t68sites/base-t68sites.corpus/difference | 47 | 21 | 21 | 21 | 21 |
| corpus/t68sites/base-t68sites.corpus/offset | 123 | 123 | 123 | 123 | 123 |
| corpus/t68sites/benchy-region-prep.corpus/intersection | 6 | 2 | 2 | 2 | 2 |
| corpus/t68sites/benchy-t68sites.corpus/difference | 21 | 9 | 9 | 9 | 9 |
| corpus/t68sites/benchy-t68sites.corpus/offset | 54 | 54 | 54 | 54 | 54 |

Total per pass: 249/295 forward and 249/295 reverse ordered-hash matches; divergences 46 / 46.

| Divergent case (original index) | Site / layer / opcode | Rust paths/rings/verts | C++ paths/rings/verts | Pass |
|---|---|---|---|---|
| corpus/t68sites/base-region-prep.corpus #2 | RP1 / 239 / intersection | 431/651/12788 | 435/651/12788 | forward |
| corpus/t68sites/base-region-prep.corpus #3 | RP2 / 239 / intersection | 431/651/12788 | 435/651/12788 | forward |
| corpus/t68sites/base-region-prep.corpus #20 | RP1 / 320 / intersection | 287/473/11549 | 291/473/11549 | forward |
| corpus/t68sites/base-region-prep.corpus #21 | RP2 / 320 / intersection | 287/473/11549 | 291/473/11549 | forward |
| corpus/t68sites/base-t68sites.corpus #14 | D / 40 / difference | 1132/1132/7576 | 1158/1158/7574 | forward |
| corpus/t68sites/base-t68sites.corpus #18 | H / 40 / difference | 697/697/4733 | 704/704/4721 | forward |
| corpus/t68sites/base-t68sites.corpus #25 | D / 80 / difference | 937/937/6433 | 961/961/6430 | forward |
| corpus/t68sites/base-t68sites.corpus #29 | H / 80 / difference | 683/683/4171 | 697/697/4172 | forward |
| corpus/t68sites/base-t68sites.corpus #36 | D / 120 / difference | 807/807/6147 | 816/816/6144 | forward |
| corpus/t68sites/base-t68sites.corpus #40 | H / 120 / difference | 607/607/3678 | 610/610/3666 | forward |
| corpus/t68sites/base-t68sites.corpus #47 | D / 160 / difference | 990/990/7372 | 993/993/7336 | forward |
| corpus/t68sites/base-t68sites.corpus #51 | H / 160 / difference | 686/686/4303 | 691/691/4284 | forward |
| corpus/t68sites/base-t68sites.corpus #58 | D / 200 / difference | 898/898/6538 | 909/909/6522 | forward |
| corpus/t68sites/base-t68sites.corpus #62 | H / 200 / difference | 630/630/4005 | 640/640/3998 | forward |
| corpus/t68sites/base-t68sites.corpus #69 | D / 239 / difference | 676/676/5190 | 678/678/5186 | forward |
| corpus/t68sites/base-t68sites.corpus #73 | H / 239 / difference | 505/505/2971 | 508/508/2966 | forward |
| corpus/t68sites/base-t68sites.corpus #80 | D / 240 / difference | 588/588/4717 | 598/598/4721 | forward |
| corpus/t68sites/base-t68sites.corpus #84 | H / 240 / difference | 465/465/2762 | 468/468/2755 | forward |
| corpus/t68sites/base-t68sites.corpus #98 | D / 280 / difference | 781/781/5726 | 791/791/5723 | forward |
| corpus/t68sites/base-t68sites.corpus #102 | H / 280 / difference | 564/564/3527 | 568/568/3510 | forward |
| corpus/t68sites/base-t68sites.corpus #109 | D / 320 / difference | 520/520/3955 | 526/526/3955 | forward |
| corpus/t68sites/base-t68sites.corpus #113 | H / 320 / difference | 343/343/2203 | 344/344/2199 | forward |
| corpus/t68sites/base-t68sites.corpus #120 | D / 360 / difference | 426/426/3803 | 431/431/3804 | forward |
| corpus/t68sites/base-t68sites.corpus #124 | H / 360 / difference | 361/361/2129 | 361/361/2123 | forward |
| corpus/t68sites/base-t68sites.corpus #131 | D / 400 / difference | 366/366/2970 | 370/370/2967 | forward |
| corpus/t68sites/base-t68sites.corpus #135 | H / 400 / difference | 290/290/1756 | 289/289/1738 | forward |
| corpus/t68sites/base-t68sites.corpus #142 | D / 440 / difference | 309/309/2491 | 311/311/2490 | forward |
| corpus/t68sites/base-t68sites.corpus #146 | H / 440 / difference | 220/220/1388 | 220/220/1386 | forward |
| corpus/t68sites/base-t68sites.corpus #153 | D / 480 / difference | 125/125/924 | 126/126/920 | forward |
| corpus/t68sites/base-t68sites.corpus #157 | H / 480 / difference | 77/77/502 | 78/78/502 | forward |
| corpus/t68sites/benchy-region-prep.corpus #0 | RP1 / 0 / intersection | 62/65/2041 | 62/65/2005 | forward |
| corpus/t68sites/benchy-region-prep.corpus #1 | RP2 / 0 / intersection | 62/65/2041 | 62/65/2005 | forward |
| corpus/t68sites/benchy-region-prep.corpus #2 | RP1 / 239 / intersection | 13/14/610 | 13/14/604 | forward |
| corpus/t68sites/benchy-region-prep.corpus #3 | RP2 / 239 / intersection | 13/14/610 | 13/14/604 | forward |
| corpus/t68sites/benchy-t68sites.corpus #4 | D / 0 / difference | 951/951/7220 | 964/964/7204 | forward |
| corpus/t68sites/benchy-t68sites.corpus #8 | H / 0 / difference | 565/565/3708 | 568/568/3700 | forward |
| corpus/t68sites/benchy-t68sites.corpus #15 | D / 40 / difference | 195/195/1341 | 197/197/1335 | forward |
| corpus/t68sites/benchy-t68sites.corpus #19 | H / 40 / difference | 157/157/1089 | 159/159/1083 | forward |
| corpus/t68sites/benchy-t68sites.corpus #26 | D / 80 / difference | 232/232/2396 | 233/233/2387 | forward |
| corpus/t68sites/benchy-t68sites.corpus #30 | H / 80 / difference | 212/212/1163 | 216/216/1159 | forward |
| corpus/t68sites/benchy-t68sites.corpus #37 | D / 120 / difference | 219/219/1914 | 221/221/1906 | forward |
| corpus/t68sites/benchy-t68sites.corpus #41 | H / 120 / difference | 171/171/1054 | 172/172/1050 | forward |
| corpus/t68sites/benchy-t68sites.corpus #48 | D / 160 / difference | 157/157/1626 | 157/157/1623 | forward |
| corpus/t68sites/benchy-t68sites.corpus #52 | H / 160 / difference | 180/180/804 | 180/180/796 | forward |
| corpus/t68sites/benchy-t68sites.corpus #59 | D / 200 / difference | 117/117/994 | 119/119/991 | forward |
| corpus/t68sites/benchy-t68sites.corpus #69 | D / 239 / difference | 84/84/604 | 86/86/604 | forward |
| corpus/t68sites/base-region-prep.corpus #2 | RP1 / 239 / intersection | 431/651/12788 | 435/651/12788 | reverse |
| corpus/t68sites/base-region-prep.corpus #3 | RP2 / 239 / intersection | 431/651/12788 | 435/651/12788 | reverse |
| corpus/t68sites/base-region-prep.corpus #20 | RP1 / 320 / intersection | 287/473/11549 | 291/473/11549 | reverse |
| corpus/t68sites/base-region-prep.corpus #21 | RP2 / 320 / intersection | 287/473/11549 | 291/473/11549 | reverse |
| corpus/t68sites/base-t68sites.corpus #14 | D / 40 / difference | 1132/1132/7576 | 1158/1158/7574 | reverse |
| corpus/t68sites/base-t68sites.corpus #18 | H / 40 / difference | 697/697/4733 | 704/704/4721 | reverse |
| corpus/t68sites/base-t68sites.corpus #25 | D / 80 / difference | 937/937/6433 | 961/961/6430 | reverse |
| corpus/t68sites/base-t68sites.corpus #29 | H / 80 / difference | 683/683/4171 | 697/697/4172 | reverse |
| corpus/t68sites/base-t68sites.corpus #36 | D / 120 / difference | 807/807/6147 | 816/816/6144 | reverse |
| corpus/t68sites/base-t68sites.corpus #40 | H / 120 / difference | 607/607/3678 | 610/610/3666 | reverse |
| corpus/t68sites/base-t68sites.corpus #47 | D / 160 / difference | 990/990/7372 | 993/993/7336 | reverse |
| corpus/t68sites/base-t68sites.corpus #51 | H / 160 / difference | 686/686/4303 | 691/691/4284 | reverse |
| corpus/t68sites/base-t68sites.corpus #58 | D / 200 / difference | 898/898/6538 | 909/909/6522 | reverse |
| corpus/t68sites/base-t68sites.corpus #62 | H / 200 / difference | 630/630/4005 | 640/640/3998 | reverse |
| corpus/t68sites/base-t68sites.corpus #69 | D / 239 / difference | 676/676/5190 | 678/678/5186 | reverse |
| corpus/t68sites/base-t68sites.corpus #73 | H / 239 / difference | 505/505/2971 | 508/508/2966 | reverse |
| corpus/t68sites/base-t68sites.corpus #80 | D / 240 / difference | 588/588/4717 | 598/598/4721 | reverse |
| corpus/t68sites/base-t68sites.corpus #84 | H / 240 / difference | 465/465/2762 | 468/468/2755 | reverse |
| corpus/t68sites/base-t68sites.corpus #98 | D / 280 / difference | 781/781/5726 | 791/791/5723 | reverse |
| corpus/t68sites/base-t68sites.corpus #102 | H / 280 / difference | 564/564/3527 | 568/568/3510 | reverse |
| corpus/t68sites/base-t68sites.corpus #109 | D / 320 / difference | 520/520/3955 | 526/526/3955 | reverse |
| corpus/t68sites/base-t68sites.corpus #113 | H / 320 / difference | 343/343/2203 | 344/344/2199 | reverse |
| corpus/t68sites/base-t68sites.corpus #120 | D / 360 / difference | 426/426/3803 | 431/431/3804 | reverse |
| corpus/t68sites/base-t68sites.corpus #124 | H / 360 / difference | 361/361/2129 | 361/361/2123 | reverse |
| corpus/t68sites/base-t68sites.corpus #131 | D / 400 / difference | 366/366/2970 | 370/370/2967 | reverse |
| corpus/t68sites/base-t68sites.corpus #135 | H / 400 / difference | 290/290/1756 | 289/289/1738 | reverse |
| corpus/t68sites/base-t68sites.corpus #142 | D / 440 / difference | 309/309/2491 | 311/311/2490 | reverse |
| corpus/t68sites/base-t68sites.corpus #146 | H / 440 / difference | 220/220/1388 | 220/220/1386 | reverse |
| corpus/t68sites/base-t68sites.corpus #153 | D / 480 / difference | 125/125/924 | 126/126/920 | reverse |
| corpus/t68sites/base-t68sites.corpus #157 | H / 480 / difference | 77/77/502 | 78/78/502 | reverse |
| corpus/t68sites/benchy-region-prep.corpus #0 | RP1 / 0 / intersection | 62/65/2041 | 62/65/2005 | reverse |
| corpus/t68sites/benchy-region-prep.corpus #1 | RP2 / 0 / intersection | 62/65/2041 | 62/65/2005 | reverse |
| corpus/t68sites/benchy-region-prep.corpus #2 | RP1 / 239 / intersection | 13/14/610 | 13/14/604 | reverse |
| corpus/t68sites/benchy-region-prep.corpus #3 | RP2 / 239 / intersection | 13/14/610 | 13/14/604 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #4 | D / 0 / difference | 951/951/7220 | 964/964/7204 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #8 | H / 0 / difference | 565/565/3708 | 568/568/3700 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #15 | D / 40 / difference | 195/195/1341 | 197/197/1335 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #19 | H / 40 / difference | 157/157/1089 | 159/159/1083 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #26 | D / 80 / difference | 232/232/2396 | 233/233/2387 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #30 | H / 80 / difference | 212/212/1163 | 216/216/1159 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #37 | D / 120 / difference | 219/219/1914 | 221/221/1906 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #41 | H / 120 / difference | 171/171/1054 | 172/172/1050 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #48 | D / 160 / difference | 157/157/1626 | 157/157/1623 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #52 | H / 160 / difference | 180/180/804 | 180/180/796 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #59 | D / 200 / difference | 117/117/994 | 119/119/991 | reverse |
| corpus/t68sites/benchy-t68sites.corpus #69 | D / 239 / difference | 84/84/604 | 86/86/604 | reverse |

## Historical scope mapping: idealized accumulated-time ceilings

Re-derived now by `scope_mapping` in `analyze.py` from committed `../t60-host-native/captures/{benchy,base}-classic-off-profile.json`, summing each owner’s `scopes[].total_wall_ns` (not owner live spans). Both source hashes and full owner/scope/call totals are in `results/summary.json` → `scope_mapping`. These are accumulated engine-scope thread-time, NOT run wall.
For each owner/scope: idealized removable seconds = accumulated seconds × (1 − 1 / measured ratio); scope totals sum those owner ceilings. Offset uses the same model’s classic-site offset ratio; closing_ex assumes that ratio transfers to its composed offsets. The CP clip scope uses that model’s difference ratio; the two RP derivation clip scopes use its intersection ratio. Other clip owners use a case-count-weighted mean of that model’s sampled boolean-opcode ratios (CP difference + RP intersection). Their historical opcode/time mix is unavailable, so this mapping is conditional, not an identified production gain. Every owner/scope ratio used is retained in `results/summary.json`. Union-only t55 is NOT assigned to supports-off scope totals.
Only the classic-perimeters per-layer host-service family is directly shape-represented by the CP corpus. Historical owner-share fractions below are coverage proxies, not captured-time fractions: sparse sampled layers do not measure the fraction of owner time captured. RP inputs cover only intersection; the historical closing/offset region-preparation workload was NOT captured. Any offset/closing ceiling for such owners assumes per-opclass ratio transfer within that opclass. In the actual t60 owner table the two `probe:derive_*` owners contain clip_polygons only; the large historical closing scope is `host:slice`. Do not relabel it as a directly captured RP workload. All other owners likewise require transfer assumptions. Ticket 67 changed preparation sharing after t60; historical totals are not a current census.

| Model / scope | Accumulated s | Forward idealized removable s | Reverse idealized removable s | CP owner fraction | RP owner fraction (intersection shapes only) |
|---|---:|---:|---:|---:|---:|
| base / clip_polygons | 197.873324 | 79.518562 | 81.638756 | 1.6885% | 40.7888% |
| base / closing_ex | 374.468717 | 198.857467 | 200.268178 | 0.0000% | 0.0000% |
| base / offset | 482.932029 | 256.455709 | 258.275027 | 17.7915% | 0.0000% |
| benchy / clip_polygons | 10.395286 | 4.196214 | 4.052046 | 3.3471% | 28.3904% |
| benchy / closing_ex | 23.481739 | 10.301671 | 10.900679 | 0.0000% | 0.0000% |
| benchy / offset | 24.995072 | 10.965585 | 11.603198 | 22.2426% | 0.0000% |

### Per-owner/scope re-derivation

| Model / owner | Scope | Accumulated s | Forward ceiling s | Reverse ceiling s |
|---|---|---:|---:|---:|
| base / host:mesh_analysis | clip_polygons | 21.005936 | 8.013165 | 7.509881 |
| base / host:overhang_annotation | offset | 127.706336 | 67.817036 | 68.298136 |
| base / host:overhang_annotation | clip_polygons | 21.709619 | 8.281600 | 7.761457 |
| base / host:shell_classification | offset | 129.791210 | 68.924185 | 69.413139 |
| base / host:shell_classification | clip_polygons | 14.735668 | 5.621237 | 5.268183 |
| base / host:shell_classification | closing_ex | 2.646577 | 1.405436 | 1.415406 |
| base / host:slice | closing_ex | 371.822139 | 197.452031 | 198.852773 |
| base / host:slice | offset | 122.745066 | 65.182409 | 65.644818 |
| base / host:slice | clip_polygons | 46.327358 | 17.672565 | 16.562602 |
| base / probe:d:classic-perimeters | offset | 85.920626 | 45.627197 | 45.950881 |
| base / probe:d:classic-perimeters | clip_polygons | 3.341121 | 1.156423 | 0.765775 |
| base / probe:d:infill-linker | offset | 3.100706 | 1.646596 | 1.658277 |
| base / probe:d:rectilinear-infill | clip_polygons | 0.004610 | 0.001759 | 0.001648 |
| base / probe:derive_ordinary | clip_polygons | 40.529260 | 17.546550 | 20.176817 |
| base / probe:derive_perimeter_source | clip_polygons | 40.180814 | 17.395695 | 20.003349 |
| base / probe:p:seam-planner-default | offset | 13.668085 | 7.258285 | 7.309776 |
| base / probe:region_partition | clip_polygons | 10.038938 | 3.829568 | 3.589044 |
| benchy / host:mesh_analysis | clip_polygons | 0.135062 | 0.048814 | 0.050077 |
| benchy / host:overhang_annotation | offset | 6.150440 | 2.698259 | 2.855154 |
| benchy / host:overhang_annotation | clip_polygons | 2.420814 | 0.874920 | 0.897565 |
| benchy / host:shell_classification | offset | 10.136107 | 4.446810 | 4.705378 |
| benchy / host:shell_classification | clip_polygons | 1.208563 | 0.436793 | 0.448099 |
| benchy / host:shell_classification | closing_ex | 0.307300 | 0.134816 | 0.142655 |
| benchy / host:slice | closing_ex | 23.174439 | 10.166855 | 10.758024 |
| benchy / host:slice | clip_polygons | 2.073228 | 0.749297 | 0.768690 |
| benchy / host:slice | offset | 1.665835 | 0.730818 | 0.773313 |
| benchy / probe:d:classic-perimeters | offset | 5.559563 | 2.439035 | 2.580857 |
| benchy / probe:d:classic-perimeters | clip_polygons | 0.347946 | 0.103034 | 0.120920 |
| benchy / probe:d:infill-linker | offset | 0.533373 | 0.233996 | 0.247602 |
| benchy / probe:d:rectilinear-infill | clip_polygons | 0.001110 | 0.000401 | 0.000411 |
| benchy / probe:derive_ordinary | clip_polygons | 1.476921 | 0.764941 | 0.650625 |
| benchy / probe:derive_perimeter_source | clip_polygons | 1.474338 | 0.763604 | 0.649488 |
| benchy / probe:p:seam-planner-default | offset | 0.949753 | 0.416666 | 0.440894 |
| benchy / probe:region_partition | clip_polygons | 1.257306 | 0.454410 | 0.466171 |

### Relevant owner totals and wall relevance

| Model | CP owner accumulated s | Ordinary derivation s | Perimeter-source derivation s | All-scope ceiling forward / reverse s | CP-only ceiling forward / reverse s |
|---|---:|---:|---:|---:|---:|
| base | 89.261896 | 40.529307 | 40.180862 | 534.831738 / 540.181962 | 46.783620 / 46.716656 |
| benchy | 5.907573 | 1.476938 | 1.474354 | 25.463470 / 26.555923 | 2.542069 / 2.701777 |

**Uninstrumented-wall gap:** the requested t60/t68 source files do not contain uninstrumented runs. t68 metadata’s command explicitly has `--instrument-stderr --profile --profile-verbose`; both FINDINGS files say no uninstrumented timing ran. It would be false to present their walls/phases as uninstrumented. No slicer was run in this campaign.

| Model | t60 instrumented process wall s | t68 instrumented process wall s | t68 per-layer phase s | CP-only saturated context % forward / reverse | All-scope saturated context % forward / reverse |
|---|---:|---:|---:|---:|---:|
| base | 270.0844001 | 263.5296499 | 115.408 | 1.4794 / 1.4773 | 16.9124 / 17.0816 |
| benchy | 18.2968313 | 17.5746102 | 7.203 | 1.2054 / 1.2811 | 12.0740 / 12.5920 |

Saturated context = idealized accumulated ceiling ÷ 12 ÷ the explicitly instrumented t68 wall. It is an occupancy-style scale comparison, NOT a wall saving, lower bound, forecast, or gate pass. Perfectly serialized transfer would instead divide accumulated ceiling by wall without 12; neither model establishes scheduling/critical-path transfer. Different compiler backends, runtime allocators, engine revisions, temporal machine load and historical instrument overhead confound transfer. No ordinary/accelerated or wasm speedup was measured.

## Where the headroom lives and verdict

- Opclass ranking (forward): union 2.6280 > offset 2.0889 > intersection 1.9998 > difference 1.7608.
- Opclass ranking (reverse): union 2.6084 > offset 2.0336 > intersection 1.9409 > difference 1.5983.
- Site ranking (forward): 550 2.6280 > K 2.3033 > E 2.2916 > A0 2.2557 > F 2.1912 > A1 2.1219 > RP1 2.1144 > B 2.0994 > G 2.0050 > C 1.9378 > RP2 1.8902 > J 1.6442 > D 1.6073 > H 1.4166.
- Site ranking (reverse): 550 2.6084 > A0 2.3161 > B 2.1898 > E 2.1678 > K 2.1671 > C 2.0638 > G 2.0546 > F 1.9959 > RP1 1.9835 > RP2 1.9016 > A1 1.7813 > H 1.6770 > D 1.6385 > J 1.1782.
- base forward mapped ceiling ranking: offset 256.455709 s > closing_ex 198.857467 s > clip_polygons 79.518562 s; accumulated, conditional.
- base reverse mapped ceiling ranking: offset 258.275027 s > closing_ex 200.268178 s > clip_polygons 81.638756 s; accumulated, conditional.
- benchy forward mapped ceiling ranking: offset 10.965585 s > closing_ex 10.301671 s > clip_polygons 4.196214 s; accumulated, conditional.
- benchy reverse mapped ceiling ranking: offset 11.603198 s > closing_ex 10.900679 s > clip_polygons 4.052046 s; accumulated, conditional.

The dominant measured engine opclasses are materially slower than C++ in both orders; the mapped accumulated ceiling is not a below-floor round-trip-overhead lead. The [map’s t68 conclusion and Decisions](../../map.md#decisions-so-far) treat ≤0.80% serial wall relevance as below-gate; its gap-budget Decision gives a destination gap of 6.28–26.24×. This headroom justifies a **candidate ticket**, not a keep/adoption decision and not a claim to close that gap. Even the CP-only saturated diagnostic scale exceeds the prior below-floor scale, but the uninstrumented wall denominator and real optimization transfer remain unmeasured. A candidate still needs pristine-Rust value exactness, fresh both-leg output qualification and the predeclared paired ordinary + accelerated wall/CPU gate. C++ is a headroom reference, NOT a replacement proposal.

## Evidence, validation and remaining gaps

- `campaign.py`: sealed/resumable runner; raw manifest and initial/resume tee logs are under the gitignored scratch raw tree.
- `analyze.py`: raw → portable reduced case medians and deterministic tables; `results/forward.json`, `results/reverse.json`, `results/summary.json`, `results/provenance.json`.
- `verify.py`: checkout-only recomputation from committed results + canonical corpus and t60/t68 captures; checks report equality, source population/geometry, output invariance across passes and seal references; mutation controls reject changed timing, deleted case, shifted site and changed scope total.
- Exact cross-engine hashes are order-sensitive, not production parity. RP closing/offset shapes missing; historical boolean time mix unknown; RP snapshots are not exhaustive frequency weights; CP is layer-sampled.
- No production source, issue, map, engine or registry edits; no staging/commit; no slicer, cargo test or engine rebuild. No production speedup, CPU corroboration, or new wasm measurement. No internal-engine profile or confidence interval.
- Verification outcome is recorded separately by the final verifier execution; this report is a deterministic rendering, so the verifier can reject numerical report drift.
