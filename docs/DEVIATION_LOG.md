# Deviation Log

Use this registry for intentional deviations from architecture docs.

| ID | Date | Affected Section | Risk | Rationale | Mitigation Owner | Target Close | Status |
|---|---|---|---|---|---|---|---|
| DEV-001 | 2026-04-14 | 05_module_sdk.md §"Python Bridge (TextPostProcess tier)" / 00_project_overview.md §"Python bridge" | Low | Initially shipped as a `python3 -I` subprocess driver because pyo3 0.20 did not support Python 3.14. **Closed 2026-04-14** by upgrading to `pyo3 = "0.28.3"` (feature `auto-initialize`) and replacing the subprocess backend with an embedded PyO3 implementation inside `PythonBridge::run_text`. No subprocess path remains. | slicer-host maintainer | 2026-04-14 | Closed |
