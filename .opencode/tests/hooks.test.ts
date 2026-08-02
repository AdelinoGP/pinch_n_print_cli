import { expect, test } from "bun:test"
import { ProjectHooks, correctToolPath } from "../plugins/hooks"

const ROOT = "F:/slicerProject/pinch_n_print_cli_2"

test("resolves a relative read path against the workspace root", () => {
  expect(correctToolPath("crates/slicer-sdk/src/traits.rs", ROOT, "read")).toEqual({
    corrected: "F:/slicerProject/pinch_n_print_cli_2/crates/slicer-sdk/src/traits.rs",
  })
})

test("normalizes backslashes and a leading ./", () => {
  expect(correctToolPath(".\\crates\\slicer-sdk\\src\\traits.rs", ROOT, "read")).toEqual({
    corrected: "F:/slicerProject/pinch_n_print_cli_2/crates/slicer-sdk/src/traits.rs",
  })
})

test("rejects a bare-parent path (F:/slicerProject/crates/...) with the active root named", () => {
  const result = correctToolPath("F:\\slicerProject\\crates\\slicer-sdk\\src\\traits.rs", ROOT, "read")
  expect(result && "reason" in result).toBe(true)
  expect(result && "reason" in result ? result.reason : "").toContain(ROOT)
})

test("rejects a bare-parent docs path", () => {
  const result = correctToolPath("F:/slicerProject/docs/07_implementation_status.md", ROOT, "read")
  expect(result && "reason" in result).toBe(true)
})

test("rejects a sibling-worktree path (pinch_n_print_cli)", () => {
  const result = correctToolPath("F:/slicerProject/pinch_n_print_cli/crates/slicer-core/src/lib.rs", ROOT, "grep")
  expect(result && "reason" in result).toBe(true)
  expect(result && "reason" in result ? result.reason : "").toContain(ROOT)
})

test("rejects a sibling-worktree path (pinch_n_print)", () => {
  const result = correctToolPath("F:\\slicerProject\\pinch_n_print\\crates\\slicer-gcode\\src\\flavor.rs", ROOT, "read")
  expect(result && "reason" in result).toBe(true)
})

test("leaves a valid in-repo absolute path unchanged", () => {
  expect(correctToolPath("F:/slicerProject/pinch_n_print_cli_2/crates/slicer-sdk/src/traits.rs", ROOT, "read")).toEqual({
    corrected: "F:/slicerProject/pinch_n_print_cli_2/crates/slicer-sdk/src/traits.rs",
  })
})

test("leaves a deliberate external absolute path unchanged (OrcaSlicer checkout)", () => {
  expect(correctToolPath("F:/slicerProject/pinch_n_print_cli_2/OrcaSlicerDocumented/src/libslic3r/GCode.cpp", ROOT, "read")).toEqual({
    corrected: "F:/slicerProject/pinch_n_print_cli_2/OrcaSlicerDocumented/src/libslic3r/GCode.cpp",
  })
})

test("leaves an unrelated sibling-repo path unchanged (pinch_n_print_studio)", () => {
  expect(correctToolPath("F:/slicerProject/pinch_n_print_studio/crates/foo/src/lib.rs", ROOT, "read")).toEqual({
    corrected: "F:/slicerProject/pinch_n_print_studio/crates/foo/src/lib.rs",
  })
})

test("does not rewrite grep/glob paths that are already absolute", () => {
  expect(correctToolPath("F:/slicerProject/pinch_n_print_cli_2/crates", ROOT, "grep")).toEqual({
    corrected: "F:/slicerProject/pinch_n_print_cli_2/crates",
  })
})

test("hook mutates read args.filePath in place", async () => {
  const hooks = await ProjectHooks({ directory: ROOT })
  const before = hooks["tool.execute.before"]
  if (!before) throw new Error("ProjectHooks did not register a pre-tool hook")

  const args: Record<string, unknown> = { filePath: "crates\\slicer-sdk\\src\\traits.rs" }
  await before({ tool: "read", sessionID: "s", callID: "c" }, { args })
  expect(args.filePath).toBe("F:/slicerProject/pinch_n_print_cli_2/crates/slicer-sdk/src/traits.rs")
})

test("hook rejects a mis-rooted grep path", async () => {
  const hooks = await ProjectHooks({ directory: ROOT })
  const before = hooks["tool.execute.before"]
  if (!before) throw new Error("ProjectHooks did not register a pre-tool hook")

  const args: Record<string, unknown> = { path: "F:\\slicerProject\\crates\\slicer-core\\src" }
  await expect(before({ tool: "grep", sessionID: "s", callID: "c" }, { args })).rejects.toThrow(
    "outside the active workspace",
  )
})

test("hook leaves other tools untouched", async () => {
  const hooks = await ProjectHooks({ directory: ROOT })
  const before = hooks["tool.execute.before"]
  if (!before) throw new Error("ProjectHooks did not register a pre-tool hook")

  const args: Record<string, unknown> = { command: "cd F:\\slicerProject" }
  await before({ tool: "bash", sessionID: "s", callID: "c" }, { args })
  expect(args.command).toBe("cd F:\\slicerProject")
})

test("does not block an in-repo hidden path when root collides with the sibling-root name", () => {
  const result = correctToolPath(
    "F:/slicerProject/pinch_n_print_cli/.ralph/specs/119_support-validation-wedge-harness/packet.spec.md",
    "F:/slicerProject/pinch_n_print_cli",
    "read",
  )
  expect(result).toEqual({
    corrected: "F:/slicerProject/pinch_n_print_cli/.ralph/specs/119_support-validation-wedge-harness/packet.spec.md",
  })
})

test("still rejects the true sibling worktree while root collides with its name", () => {
  const result = correctToolPath(
    "F:/slicerProject/pinch_n_print/crates/slicer-gcode/src/flavor.rs",
    "F:/slicerProject/pinch_n_print_cli",
    "read",
  )
  expect(result && "reason" in result).toBe(true)
  expect(result && "reason" in result ? result.reason : "").toContain("sibling worktree")
})

test("hook blocks bash with a workdir containing a control character", async () => {
  const hooks = await ProjectHooks({ directory: ROOT })
  const before = hooks["tool.execute.before"]
  if (!before) throw new Error("ProjectHooks did not register a pre-tool hook")

  const args: Record<string, unknown> = {
    command: "cargo test -p infill-linker 2>&1 | tee target/test-output.log",
    workdir: "F:\\slicerProject\t hinch_n_print_cli",
  }
  await expect(before({ tool: "bash", sessionID: "s", callID: "c" }, { args })).rejects.toThrow(
    "control characters",
  )
})

test("hook allows bash with a clean workdir", async () => {
  const hooks = await ProjectHooks({ directory: ROOT })
  const before = hooks["tool.execute.before"]
  if (!before) throw new Error("ProjectHooks did not register a pre-tool hook")

  const args: Record<string, unknown> = {
    command: "cargo test -p infill-linker 2>&1 | tee target/test-output.log",
    workdir: "F:\\slicerProject\\pinch_n_print_cli",
  }
  await before({ tool: "bash", sessionID: "s", callID: "c" }, { args })
  expect(args.workdir).toBe("F:\\slicerProject\\pinch_n_print_cli")
})

test("leaves a POSIX absolute path unchanged", () => {
  expect(correctToolPath("/home/user/pinch_n_print_cli/crates/slicer-core/src/lib.rs", "/home/user/pinch_n_print_cli", "read")).toEqual({
    corrected: "/home/user/pinch_n_print_cli/crates/slicer-core/src/lib.rs",
  })
})

test("leaves a relative POSIX path rooted against the workspace", () => {
  expect(correctToolPath("crates/slicer-sdk/src/traits.rs", "/home/user/pinch_n_print_cli", "read")).toEqual({
    corrected: "/home/user/pinch_n_print_cli/crates/slicer-sdk/src/traits.rs",
  })
})

test("rejects a POSIX sibling-worktree path", () => {
  const result = correctToolPath(
    "/home/user/pinch_n_print/crates/slicer-gcode/src/flavor.rs",
    "/home/user/pinch_n_print_cli",
    "read",
  )
  expect(result && "reason" in result).toBe(true)
})

test("backslash handling follows the host platform", () => {
  const result = correctToolPath("/home/user/pinch_n_print_cli/weird\\name/foo.rs", "/home/user/pinch_n_print_cli", "read")
  const corrected = result && "corrected" in result ? result.corrected : ""
  expect(corrected).toBe(
    process.platform === "win32"
      ? "/home/user/pinch_n_print_cli/weird/name/foo.rs"
      : "/home/user/pinch_n_print_cli/weird\\name/foo.rs",
  )
})
