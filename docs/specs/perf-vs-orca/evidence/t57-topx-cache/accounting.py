"""Measure cache layout in a separate diagnostic copy; never alter acceptance source."""

import json
import shutil
import manage as m

root = m.RAW / "accounting"
assert not root.exists(), "one-shot accounting guard"
lib = root / "engine"
shutil.copytree(m.LIB, lib, ignore=shutil.ignore_patterns("target"))
contract = lib / "src/topx_contract.rs"
with contract.open("a") as stream:
    stream.write("""
/// Diagnostic-only layout and executed key-work accounting.
pub fn accounting() -> [usize; 6] {
    let counts = run_checks();
    [std::mem::size_of::<ScanbeamProjection>(),
     std::mem::size_of::<Option<ScanbeamProjection>>(),
     std::mem::size_of::<Vec<Option<ScanbeamProjection>>>(),
     counts[0], counts[1], counts[2]]
}
""")
(root / "src").mkdir()
(root / "src/main.rs").write_text("""fn main() {
    println!("T57_ACCOUNTING {:?}", clipper2_rust::engine::topx_contract::accounting());
}
""")
(root / "src/lib.rs").write_text("""#[no_mangle]
pub extern "C" fn t57_accounting(i: usize) -> usize {
    clipper2_rust::engine::topx_contract::accounting()[i]
}
""")
(root / "Cargo.toml").write_text("""[package]
name = "t57-accounting"
version = "0.0.0"
edition = "2021"
[workspace]
exclude = ["engine"]
[lib]
crate-type = ["cdylib"]
[dependencies]
clipper2-rust = { path = "engine", version = "=1.1.0", features = ["topx-test-support"] }
""")
shutil.copy2(m.REPO / "Cargo.lock", root / "Cargo.lock")
m.run(
    ["cargo", "run", "--manifest-path", root / "Cargo.toml", "--offline", "--release"],
    "native-accounting",
)
line = next(
    l
    for l in (m.HERE / "logs/native-accounting.log").read_text().splitlines()
    if l.startswith("T57_ACCOUNTING ")
)
native = json.loads(line.split(" ", 1)[1])
m.run(
    [
        "cargo",
        "build",
        "--manifest-path",
        root / "Cargo.toml",
        "--offline",
        "--release",
        "--lib",
        "--target",
        "wasm32-unknown-unknown",
    ],
    "wasm-accounting-build",
)
node = """const fs=require('node:fs'); (async()=>{
const {instance}=await WebAssembly.instantiate(fs.readFileSync(process.argv[1]),{});
console.log(JSON.stringify(Array.from({length:6},(_,i)=>instance.exports.t57_accounting(i))));
})().catch(e=>{console.error(e);process.exitCode=1});"""
m.run(
    [
        "node",
        "-e",
        node,
        root / "target/wasm32-unknown-unknown/release/t57_accounting.wasm",
    ],
    "wasm-accounting",
)
wasm = json.loads((m.HERE / "logs/wasm-accounting.log").read_text())
assert native[3:] == wasm[3:] == [4, 1, 13]
m.write(
    m.HERE / "accounting.json",
    {
        "fields": [
            "record_bytes",
            "optional_record_stride_bytes",
            "vector_header_bytes",
            "contract_captures",
            "contract_hits",
            "contract_fallbacks",
        ],
        "native": native,
        "wasm32": wasm,
        "limits": "layout measured by compiled target size_of; diagnostic contract work only, not per-job allocation high water or peak RSS. Entire allocation, key checks, stores, reads and teardown are charged to probe-free end-to-end timing.",
    },
)
print("PASS: separately measured native/WASM cache layout and contract key-work counts")
