# Dual-engine headroom harness

This isolated measurement harness compares pristine clipper2-rust 1.1.0 with
bundled Clipper2 C++ 1.5.2 on identical integer inputs. It is not a replacement
engine proposal or a production parity qualification.

## Reproduce (repository root, non-interactive bash)

```bash
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/t55-to-cases.py
mkdir -p .local-artifacts/perf-vs-orca/t69-clipper-engine
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/setup-engines.py > .local-artifacts/perf-vs-orca/t69-clipper-engine/setup.log 2>&1
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/verify-drivers.py
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/smoke.py
```

Setup creates `.local-artifacts/perf-vs-orca/t69-clipper-engine/`. The
scratch directory carries a local `.gitignore` matching all its contents.
Rust tree has its own `[workspace]`, target directory, Cargo.lock and exact dependency
pin plus local `[patch.crates-io]`. Setup verifies the registry files against
the cached .crate archive, whose SHA-256 must match the root lock's published
checksum. It copies without modifying the registry and compares complete file
inventories. An existing changed scratch copy causes failure, not overwrite.
Missing cached dependencies cause offline build failure, not an implicit upgrade.

Setup asserts its own lock resolves exactly one clipper2-rust 1.1.0 entry with
no `source` before and after building. `seal.json` in scratch saves the assertion,
source checksums, compiler versions, binary hashes and exact build argv/cwd.
C++ is built directly with g++ -O3 -std=c++17, the four sources enumerated in
the canonical Clipper2 CMakeLists, and static linking. Rust uses cargo release.
Each subprocess has an explicit timeout; compiler logs go to the redirected
setup log. Setup may be rerun and never invokes Cargo in the root workspace.

Run another captured corpus (must use the same argument map):

```bash
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/smoke.py --corpus PATH_TO_CORPUS --name other --report .local-artifacts/perf-vs-orca/t69-clipper-engine/other.md
```

Direct driver commands, with one or more `.cases` filenames as arguments:

```bash
.local-artifacts/perf-vs-orca/t69-clipper-engine/cpp-driver.exe CORPUS.cases
.local-artifacts/perf-vs-orca/t69-clipper-engine/rust-harness/target/release/t69-dual-engine.exe CORPUS.cases
```

Both emit one JSON object per case to stdout. Redirect stdout to a scratch
JSONL file. `smoke.py` verifies source/binary seals, runs both sequentially,
requires identical case populations and reduces actual on-disk results.
To re-render the report without repeating the measurements:

```bash
python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/smoke.py --reduce-only
```

## Corpus and semantics

The authoritative argument/site table for the supplied corpus is
`corpus/t55/README.md`, aligned to the capture reducer's arg 10 fill rule.
Format v1 is ASCII whitespace-separated integer tokens (except record and site names),
with `#` comments. Records: `case site layer opcode [call_index]`, `arg id u64`,
`subj npaths`, optional `clip npaths`, and `end`. Geometry framing after subj/clip
is per path/ExPolygon: nrings, then per ring: npts, then npts signed x y pairs.
Case headers occupy one line. Coordinates are passed unchanged; contours and
holes flatten in input order. Optional call_index is accepted but case_index
in output is the sequential zero-based index within each input file. Named
site tokens (A, RP1, etc.) and numeric site tokens are emitted as JSON strings.

Opcode 0 invokes ClipperOffset with recorded delta/join/end/miter/arc tolerance;
opcodes 1..4 invoke Clipper64 Difference/Union/Intersection/Xor with recorded
fill rule. Both engines execute into PolyTree. PreserveCollinear and
ReverseSolution are explicit and matched. Unknown/duplicate/missing arguments
or incorrect geometry records fail instead of silently changing semantics.
Closing must be captured as two separate offset invocations, not invented as a
native opcode. Output tree traversal groups each outer and its direct holes,
recursing into islands before emitting the enclosing outer, like
`collect_expolygon_from_tree` in `crates/slicer-core/src/polygon_ops.rs`.
Unlike production reconstruction it does not reorient or filter output rings.

## Timing and output identity

One untimed warm-up and R=5 timed repeats per case. Each repeat constructs a
fresh engine and output tree, adds the same pre-parsed/pre-flattened paths,
executes, and destroys the engine. Output tree destruction, traversal, hashing,
parsing and JSON serialization are outside the timer. Every repeat's hash and
counts must match its own engine's warm-up. Engine failure is fatal.

Fields: file, case_index, site, layer, opcode, repeat_times_ns (five u64 ns),
out_paths (outer components including nested islands), out_rings (all contour
and hole rings), out_verts, out_hash (16 lowercase hexadecimal digits).

Hash: FNV-1a 64, initial 14695981039346656037, prime 1099511628211,
wrapping modulo 2^64. Visit path → ring → vertex in the traversal described
above. For each vertex visit x then y, cast signed i64 to two's-complement u64,
then feed eight bytes little-endian per coordinate, xor byte then multiply.
No counts, separators, decimal digits, or closing vertex are added. Empty output
has the initial hash. Counts are compared separately. Hashes are order-sensitive
and not a geometric equivalence oracle; collisions are possible.

## Limitations

See SMOKE.md for measured numbers, divergence counts, raw-result paths, compiler
versions and qualification gaps. Different compiler backends, allocators and
Clipper revisions confound language-level attribution. No CPU pinning, paired
process-level sampling, production output normalization, or end-to-end speedup
claim. Known ticket-31 clean_collinear differences may affect hashes; this
harness discloses rather than investigates divergences. t55 covers union only.
No production source or registry files are changed and no cargo tests are run.

`verify-drivers.py` independently checks every opcode against known rectangle
coordinates (allowing only ring rotations/reversals and component permutations
when checking FNV hashes), including negative coordinates and empty output.
It also checks that unknown argument IDs fail. These are protocol controls,
not representative performance workloads. Results are saved under scratch
`raw/protocol-controls/verification.json`.
