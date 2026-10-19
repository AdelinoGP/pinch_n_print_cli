"""Regression for the Windows manifest-key bug and bbox checker controls.

Runs no slicer. The Windows-path assertion fails with the first-attempt script.
"""

from pathlib import Path, PureWindowsPath
import json
import runpy
import sys

script = (
    Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).with_name("prepare.py")
)
module = runpy.run_path(str(script))
root = PureWindowsPath(r"D:\slicerProject\corpus")
model = root / "supports-off-benchy" / "model.stl"
key = module.get("relative_key", lambda path, base: str(path.relative_to(base)))
assert key(model, root) == "supports-off-benchy/model.stl", (
    "Windows manifest key is not portable"
)
inventory = {key(model, root): "digest"}
assert inventory["supports-off-benchy/model.stl"] == "digest", (
    "original lookup input still fails"
)
actual = module["corpus_files"]()
assert "supports-off-benchy/model.stl" in actual, (
    "real corpus inventory cannot supply the input key"
)
original = json.loads(
    Path(__file__).with_name("historical-corpus-hashes.json").read_text()
)
assert actual == {name.replace("\\", "/"): value for name, value in original.items()}, (
    "historical corpus was altered"
)
module["negative_controls"]()
print(
    "PASS: Windows manifest lookup; real inventory lookup; bbox positive/outside-start/outside-end controls"
)
