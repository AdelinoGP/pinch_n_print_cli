#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")"
test -n "${WIT_BINDGEN_HEAD:-$(awk '{print $1}' ../.generation-started 2>/dev/null || true)}" || { echo 'BLOCKED: FORK_NOT_READY generation-marker'; exit 43; }
wit-bindgen assemblyscript ./../wit --out-dir bindings
# Splice the probe implementation (mirrored in index.ts for reference) into the
# generated export stub. Generation always rewrites the stub, so this runs on
# every build.
python - <<'EOF'
from pathlib import Path
p = Path('bindings/exports/slicer$postpass_text_postprocess$text_postprocess$1_0_0.ts')
s = p.read_text()
stub = "  // TODO: implement\n  return changetype<ffi.Result<string, i_slicer$common$module_errors.ModuleError>>(0);"
impl = ('  return new ffi.Result<string, i_slicer$common$module_errors.ModuleError>(\n'
        '    0,\n'
        '    ";; foreign-language-probe\\n" + gcodeText,\n'
        '    changetype<i_slicer$common$module_errors.ModuleError>(0),\n'
        '  );')
assert stub in s, 'generated run stub not found'
p.write_text(s.replace(stub, impl))
# The generated asconfig exports runtime init as `_start`, but nothing in a
# reactor component calls it, so the itcms runtime traps on first allocation.
# Drop exportStart so asc keeps a core-module start section (runs at
# instantiation).
import json
cfgp = Path('bindings/asconfig.json')
cfg = json.loads(cfgp.read_text())
for t in cfg.get('targets', {}).values():
    t.pop('exportStart', None)
cfgp.write_text(json.dumps(cfg, indent=2))
EOF
# Compile from inside bindings/ so asc picks up the generated asconfig.json
# (abort=ffi/abort mapping, exportStart, exportRuntime).
( cd bindings && asc bindings.ts --target release )
# Post-compile export rename per wit_bindgen_exports.json: asc cannot emit
# non-identifier wasm export names, so rewrite the export section via wat.
python - <<'EOF'
import json, subprocess
from pathlib import Path
renames = json.loads(Path('bindings/wit_bindgen_exports.json').read_text())
wat = subprocess.run(['wasm-tools', 'print', 'bindings/core.wasm'],
                     capture_output=True, text=True, check=True).stdout
for as_name, wit_name in renames.items():
    old = '(export "%s"' % as_name
    assert old in wat, 'missing export ' + as_name
    wat = wat.replace(old, '(export "%s"' % wit_name)
Path('bindings/core.renamed.wat').write_text(wat)
subprocess.run(['wasm-tools', 'parse', 'bindings/core.renamed.wat', '-o', 'core.wasm'], check=True)
EOF
wasm-tools component embed --encoding utf16 -w text-postprocess-module ../wit core.wasm -o embedded.wasm
wasm-tools component new -o comp.wasm embedded.wasm
echo 'PNP_FOREIGN_COMPONENT=comp.wasm pnp_cli foreign-oracle --input "; probe input\n" --expected ";; foreign-language-probe\n; probe input\n"'
