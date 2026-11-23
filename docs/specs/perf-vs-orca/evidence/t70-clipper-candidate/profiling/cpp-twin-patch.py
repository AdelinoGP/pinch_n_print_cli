"""t70: apply phase instrumentation to the scratch C++ twin (cpp-twin).

Mirrors the Rust prof-twin's span sites exactly on the bundled Clipper2 1.5.2
scratch copy. The documented OrcaSlicerDocumented tree is never touched.
Run from the repository root. Idempotent-ish: refuses already-patched text.
"""

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[6]
TWIN = REPO / ".local-artifacts/perf-vs-orca/t70-clipper-candidate/cpp-twin/Clipper2Lib"


def patch(path: Path, replacements: list[tuple[str, str]]) -> None:
    text = path.read_text()
    for old, new in replacements:
        if new in text:
            continue
        assert text.count(old) == 1, (path, old, text.count(old))
        text = text.replace(old, new)
    path.write_text(text)


ENGINE = TWIN / "src/clipper.engine.cpp"
OFFSET = TWIN / "src/clipper.offset.cpp"

patch(
    ENGINE,
    [
        (
            '#include "clipper2/clipper.engine.h"',
            '#include "clipper2/clipper.engine.h"\n#include "clipper2/t70_prof.hpp"',
        ),
        (
            "  void ClipperBase::CleanUp()\n  {\n",
            "  void ClipperBase::CleanUp()\n  {\n    t70prof::Span t70_s9(t70prof::PHASE_CLEAN_UP);\n",
        ),
        (
            "  void ClipperBase::Reset()\n  {\n",
            "  void ClipperBase::Reset()\n  {\n    t70prof::Span t70_s9(t70prof::PHASE_CLEAN_UP);\n",
        ),
        (
            "  void ClipperBase::AddPaths(const Paths64& paths, PathType polytype, bool is_open)\n  {\n",
            "  void ClipperBase::AddPaths(const Paths64& paths, PathType polytype, bool is_open)\n  {\n    t70prof::Span t70_s1(t70prof::PHASE_ADD_PATH);\n",
        ),
        (
            "  void ClipperBase::CleanCollinear(OutRec* outrec)\n  {",
            "  void ClipperBase::CleanCollinear(OutRec* outrec)\n  {\n    t70prof::Span t70_s10(t70prof::PHASE_CLEAN_COLLINEAR);",
        ),
        (
            "    while (succeeded_)\n    {\n      InsertLocalMinimaIntoAEL(y);\n"
            "      Active* e;\n      while (PopHorz(e)) DoHorizontal(*e);\n"
            "      if (horz_seg_list_.size() > 0)\n      {\n"
            "        ConvertHorzSegsToJoins();\n"
            "        horz_seg_list_.clear();\n      }\n"
            "      bot_y_ = y;  // bot_y_ == bottom of scanbeam\n"
            "      if (!PopScanline(y)) break;  // y new top of scanbeam\n"
            "      DoIntersections(y);\n"
            "      DoTopOfScanbeam(y);\n"
            "      while (PopHorz(e)) DoHorizontal(*e);\n    }\n"
            "    if (succeeded_) ProcessHorzJoins();",
            "    while (succeeded_)\n    {\n"
            "      {\n        t70prof::Span t70_s2(t70prof::PHASE_INSERT_LOCAL_MINIMA);\n"
            "        InsertLocalMinimaIntoAEL(y);\n      }\n"
            "      Active* e;\n"
            "      {\n        t70prof::Span t70_s3(t70prof::PHASE_DO_HORIZONTAL);\n"
            "        while (PopHorz(e)) DoHorizontal(*e);\n      }\n"
            "      if (horz_seg_list_.size() > 0)\n      {\n"
            "        {\n        t70prof::Span t70_s7(t70prof::PHASE_CONVERT_HORZ_SEGS);\n"
            "        ConvertHorzSegsToJoins();\n        }\n"
            "        {\n        t70prof::Span t70_s17(t70prof::PHASE_LIST_CLEARS);\n"
            "        horz_seg_list_.clear();\n        }\n      }\n"
            "      bot_y_ = y;  // bot_y_ == bottom of scanbeam\n"
            "      {\n        t70prof::Span t70_s16(t70prof::PHASE_SCANLINE_ADMIN);\n"
            "        if (!PopScanline(y)) break;  // y new top of scanbeam\n      }\n"
            "      DoIntersections(y);\n"
            "      {\n        t70prof::Span t70_s6(t70prof::PHASE_DO_TOP_OF_SCANBEAM);\n"
            "        DoTopOfScanbeam(y);\n      }\n"
            "      {\n        t70prof::Span t70_s3b(t70prof::PHASE_DO_HORIZONTAL);\n"
            "        while (PopHorz(e)) DoHorizontal(*e);\n      }\n    }\n"
            "    {\n      t70prof::Span t70_s8(t70prof::PHASE_PROCESS_HORZ_JOINS);\n"
            "      if (succeeded_) ProcessHorzJoins();\n    }",
        ),
        (
            "  void ClipperBase::DoIntersections(const int64_t top_y)\n  {\n",
            "  void ClipperBase::DoIntersections(const int64_t top_y)\n  {\n",
        ),
        (
            "    if (BuildIntersectList(top_y))\n"
            "    {\n"
            "      ProcessIntersectList();\n"
            "      intersect_nodes_.clear();\n"
            "    }",
            "    {\n      t70prof::Span t70_s4(t70prof::PHASE_BUILD_INTERSECT_LIST);\n"
            "      build_ret = BuildIntersectList(top_y);\n    }\n"
            "    if (build_ret)\n"
            "    {\n"
            "      {\n      t70prof::Span t70_s5(t70prof::PHASE_PROCESS_INTERSECT_LIST);\n"
            "      ProcessIntersectList();\n      }\n"
            "      {\n      t70prof::Span t70_s17(t70prof::PHASE_LIST_CLEARS);\n"
            "      intersect_nodes_.clear();\n      }\n"
            "    }",
        ),
        (
            "  void ClipperBase::DoIntersections(const int64_t top_y)\n  {\n",
            "  void ClipperBase::DoIntersections(const int64_t top_y)\n  {\n    bool build_ret = false;\n",
        ),
        (
            "  void Clipper64::BuildPaths64(Paths64& solutionClosed, Paths64* solutionOpen)\n  {",
            "  void Clipper64::BuildPaths64(Paths64& solutionClosed, Paths64* solutionOpen)\n  {\n    t70prof::Span t70_s11(t70prof::PHASE_BUILD_RESULT);",
        ),
        (
            "  void Clipper64::BuildTree64(PolyPath64& polytree, Paths64& open_paths)\n  {",
            "  void Clipper64::BuildTree64(PolyPath64& polytree, Paths64& open_paths)\n  {\n    t70prof::Span t70_s11(t70prof::PHASE_BUILD_RESULT);",
        ),
    ],
)

patch(
    OFFSET,
    [
        (
            '#include "clipper2/clipper.offset.h"',
            '#include "clipper2/clipper.offset.h"\n#include "clipper2/t70_prof.hpp"',
        ),
        (
            "void ClipperOffset::DoGroupOffset(Group& group)\n{",
            "void ClipperOffset::DoGroupOffset(Group& group)\n{\n\tt70prof::Span t70_s12(t70prof::PHASE_GROUP_OFFSET);",
        ),
        (
            "\tbool paths_reversed = CheckReverseOrientation();",
            "\tbool paths_reversed = CheckReverseOrientation();\n"
            "\tt70prof::Span t70_s13(t70prof::PHASE_OFFSET_SOLUTION_ADMIN);",
        ),
        (
            "\tif (solution_tree)\n\t{\n\t\tif (paths_reversed)\n"
            "\t\t\tc.Execute(ClipType::Union, FillRule::Negative, *solution_tree);\n"
            "\t\telse\n\t\t\tc.Execute(ClipType::Union, FillRule::Positive, *solution_tree);\n\t}\n"
            "\telse\n\t{\n\t\tif (paths_reversed)\n"
            "\t\t\tc.Execute(ClipType::Union, FillRule::Negative, *solution);\n"
            "\t\telse\n\t\t\tc.Execute(ClipType::Union, FillRule::Positive, *solution);\n\t}\n}",
            "\tif (solution_tree)\n\t{\n\t\tt70prof::Span t70_s15(t70prof::PHASE_UNION_TOTAL);\n"
            "\t\tif (paths_reversed)\n"
            "\t\t\tc.Execute(ClipType::Union, FillRule::Negative, *solution_tree);\n"
            "\t\telse\n\t\t\tc.Execute(ClipType::Union, FillRule::Positive, *solution_tree);\n\t}\n"
            "\telse\n\t{\n\t\tt70prof::Span t70_s15(t70prof::PHASE_UNION_TOTAL);\n"
            "\t\tif (paths_reversed)\n"
            "\t\t\tc.Execute(ClipType::Union, FillRule::Negative, *solution);\n"
            "\t\telse\n\t\t\tc.Execute(ClipType::Union, FillRule::Positive, *solution);\n\t}\n}",
        ),
    ],
)
print("[t70] cpp twin instrumented")
sys.exit(0)
