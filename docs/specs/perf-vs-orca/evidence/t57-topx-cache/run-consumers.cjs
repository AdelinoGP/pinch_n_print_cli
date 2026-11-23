const fs = require('node:fs');
(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(process.argv[2]), {});
  const count = instance.exports.t57_consumer_checks();
  if (count !== 18) throw new Error(`wrong consumer count: ${count}`);
  console.log(`PASS: actual WASM ${count} independent ordered mesh/boolean/offset consumer comparisons`);
})().catch(error => { console.error(error); process.exitCode = 1; });
