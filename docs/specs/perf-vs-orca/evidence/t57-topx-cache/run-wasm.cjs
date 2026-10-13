// Actual wasm32 engine execution, not native emulation or an instantiation-only check.
const fs = require('node:fs');
(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(process.argv[2]), {});
  const result = instance.exports.t57_engine_checks();
  if (result !== 1056) throw new Error(`wrong comparison count: ${result}`);
  console.log(`PASS: actual WASM cache hit/fallback/lifetime checks and ${result} independent ordered-geometry comparisons`);
})().catch(error => { console.error(error); process.exitCode = 1; });
