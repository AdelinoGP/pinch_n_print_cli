import { expect, test } from "bun:test"
import plugin, { correctToolPath, handleIdleEvent, handleToolAfter } from "../plugins/hooks"

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

test("does not block an in-repo hidden path when root collides with the sibling-root name", () => {
  const result = correctToolPath(
    "F:/slicerProject/pinch_n_print_cli/docs/spec_packets/_OLD/119_support-validation-wedge-harness.md",
    "F:/slicerProject/pinch_n_print_cli",
    "read",
  )
  expect(result).toEqual({
    corrected: "F:/slicerProject/pinch_n_print_cli/docs/spec_packets/_OLD/119_support-validation-wedge-harness.md",
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

// --- V2 plugin hook tests ---

type HookCallback = (event: any) => Promise<void> | void

interface FakeCtx {
  location: { directory: string }
  tool: { hook: (name: string, cb: HookCallback) => Promise<{ dispose: () => Promise<void> }> }
  shell: { hook: (name: string, cb: HookCallback) => Promise<{ dispose: () => Promise<void> }> }
  event: { subscribe: (options?: { signal?: AbortSignal }) => AsyncIterable<any> }
  session: {
    get: (input: { sessionID: string }) => Promise<{ id: string; parentID?: string } | undefined>
    prompt: (input: { sessionID: string; text: { text: string } }) => Promise<unknown>
  }
}

function makeCtx(directory: string) {
  const toolHooks = new Map<string, HookCallback>()
  const shellHooks = new Map<string, HookCallback>()
  const sessions = new Map<string, { id: string; parentID?: string }>()
  const prompts: Array<{ sessionID: string; text: { text: string } }> = []
  const events: Array<{ type: string; data: { sessionID: string } }> = []

  const ctx: FakeCtx = {
    location: { directory },
    tool: {
      hook: async (name: string, cb: HookCallback) => {
        toolHooks.set(name, cb)
        return { dispose: async () => {} }
      },
    },
    shell: {
      hook: async (name: string, cb: HookCallback) => {
        shellHooks.set(name, cb)
        return { dispose: async () => {} }
      },
    },
    event: {
      subscribe: async function* () {
        for (const event of events) yield event
      },
    },
    session: {
      get: async ({ sessionID }: { sessionID: string }) => sessions.get(sessionID),
      prompt: async (input: { sessionID: string; text: { text: string } }) => {
        prompts.push(input)
      },
    },
  }

  return { ctx, toolHooks, shellHooks, sessions, prompts, events }
}

async function setupPlugin(directory: string) {
  const m = makeCtx(directory)
  await plugin.setup(m.ctx as any)
  return m
}

function toolEvent(tool: string, input: unknown, sessionID = "s") {
  return { tool, sessionID, agent: "a", messageID: "m", id: "c", input }
}

test("plugin registers the expected hooks", async () => {
  const { toolHooks, shellHooks } = await setupPlugin(ROOT)
  expect(toolHooks.has("execute.before")).toBe(true)
  expect(toolHooks.has("execute.after")).toBe(true)
  expect(shellHooks.has("create.before")).toBe(true)
})

test("hook mutates read args.filePath in place", async () => {
  const { toolHooks } = await setupPlugin(ROOT)
  const before = toolHooks.get("execute.before")
  if (!before) throw new Error("execute.before hook not registered")

  const event = toolEvent("read", { filePath: "crates\\slicer-sdk\\src\\traits.rs" })
  await before(event)
  expect(event.input.filePath).toBe("F:/slicerProject/pinch_n_print_cli_2/crates/slicer-sdk/src/traits.rs")
})

test("hook rejects a mis-rooted grep path", async () => {
  const { toolHooks } = await setupPlugin(ROOT)
  const before = toolHooks.get("execute.before")
  if (!before) throw new Error("execute.before hook not registered")

  const event = toolEvent("grep", { path: "F:\\slicerProject\\crates\\slicer-core\\src" })
  expect(() => before(event)).toThrow("outside the active workspace")
})

test("hook leaves other tools untouched", async () => {
  const { toolHooks } = await setupPlugin(ROOT)
  const before = toolHooks.get("execute.before")
  if (!before) throw new Error("execute.before hook not registered")

  const event = toolEvent("bash", { command: "cd F:\\slicerProject" })
  await before(event)
  expect(event.input.command).toBe("cd F:\\slicerProject")
})

test("shell hook blocks a working directory containing a control character", async () => {
  const { shellHooks } = await setupPlugin(ROOT)
  const createBefore = shellHooks.get("create.before")
  if (!createBefore) throw new Error("create.before hook not registered")

  const event = {
    command: "cargo test -p infill-linker 2>&1 | tee target/test-output.log",
    cwd: "F:\\slicerProject\t hinch_n_print_cli",
    timeout: 120_000,
    shell: "bash",
    env: {},
  }
  await expect(() => createBefore(event)).toThrow("control characters")
})

test("shell hook allows a clean working directory", async () => {
  const { shellHooks } = await setupPlugin(ROOT)
  const createBefore = shellHooks.get("create.before")
  if (!createBefore) throw new Error("create.before hook not registered")

  const event = {
    command: "cargo test -p infill-linker 2>&1 | tee target/test-output.log",
    cwd: "F:\\slicerProject\\pinch_n_print_cli",
    timeout: 120_000,
    shell: "bash",
    env: {},
  }
  await createBefore(event)
  expect(event.cwd).toBe("F:\\slicerProject\\pinch_n_print_cli")
})

test("execute.after marks the root session dirty and rustfmt runs for .rs files", async () => {
  const { sessions } = makeCtx(ROOT)
  sessions.set("ses-child", { id: "ses-child", parentID: "ses-root" })
  sessions.set("ses-root", { id: "ses-root" })

  const dirtyRootSessions = new Set<string>()
  const runs: Array<{ cwd: string; cmd: string; args: string[] }> = []
  const deps = {
    directory: ROOT,
    rootSessionID: async (sessionID: string) => {
      const session = sessions.get(sessionID)
      if (!session) return undefined
      return session.parentID ? "ses-root" : session.id
    },
    dirtyRootSessions,
    run: async (cwd: string, cmd: string, args: string[]) => {
      runs.push({ cwd, cmd, args })
      return { stdout: "", stderr: "" }
    },
  }

  await handleToolAfter(
    { tool: "edit", sessionID: "ses-child", input: { filePath: "crates/slicer-core/src/lib.rs" } },
    deps,
  )

  expect(dirtyRootSessions.has("ses-root")).toBe(true)
  expect(runs).toEqual([
    { cwd: ROOT, cmd: "rustfmt", args: ["crates/slicer-core/src/lib.rs"] },
  ])
})

test("execute.after marks dirty for mutating tools but only rustfmt .rs files", async () => {
  const dirtyRootSessions = new Set<string>()
  const runs: Array<{ cwd: string; cmd: string; args: string[] }> = []
  const deps = {
    directory: ROOT,
    rootSessionID: async () => "ses-root",
    dirtyRootSessions,
    run: async (cwd: string, cmd: string, args: string[]) => {
      runs.push({ cwd, cmd, args })
      return { stdout: "", stderr: "" }
    },
  }

  // Non-mutating tool: nothing happens at all.
  await handleToolAfter({ tool: "read", sessionID: "s", input: { filePath: "a.rs" } }, deps)
  expect(dirtyRootSessions.size).toBe(0)
  expect(runs).toEqual([])

  // Mutating tool on a non-.rs file: marked dirty, but no rustfmt.
  await handleToolAfter({ tool: "edit", sessionID: "s", input: { filePath: "README.md" } }, deps)
  expect(dirtyRootSessions.has("ses-root")).toBe(true)
  expect(runs).toEqual([])
})

test("idle handler prompts the root session when clippy fails", async () => {
  const dirtyRootSessions = new Set(["ses-root"])
  const prompts: Array<{ sessionID: string; text: string }> = []
  const clippyError = new Error("cargo clippy failed") as Error & { stderr: string }
  clippyError.stderr = "error: unused variable: `x`\n  --> crates/slicer-core/src/lib.rs:1:1\n"

  await handleIdleEvent(
    { data: { sessionID: "ses-root" } },
    {
      directory: ROOT,
      rootSessionID: async (sessionID) => (sessionID === "ses-root" ? "ses-root" : undefined),
      dirtyRootSessions,
      fixLoopGuard: { active: false },
      prompt: async (sessionID, text) => { prompts.push({ sessionID, text }) },
      run: async (_cwd, cmd, _args) => {
        if (cmd === "cargo" && _args[0] === "clippy") throw clippyError
        return { stdout: "", stderr: "" }
      },
    },
  )

  expect(prompts.length).toBe(1)
  expect(prompts[0].sessionID).toBe("ses-root")
  expect(prompts[0].text).toContain("cargo clippy")
  expect(prompts[0].text).toContain("unused variable")
  expect(dirtyRootSessions.has("ses-root")).toBe(false)
})

test("idle handler does not prompt when checks pass", async () => {
  const dirtyRootSessions = new Set(["ses-root"])
  const prompts: Array<{ sessionID: string; text: string }> = []

  await handleIdleEvent(
    { data: { sessionID: "ses-root" } },
    {
      directory: ROOT,
      rootSessionID: async (sessionID) => (sessionID === "ses-root" ? "ses-root" : undefined),
      dirtyRootSessions,
      fixLoopGuard: { active: false },
      prompt: async (sessionID, text) => { prompts.push({ sessionID, text }) },
      run: async () => ({ stdout: "", stderr: "" }),
    },
  )

  expect(prompts).toEqual([])
})

test("idle handler ignores non-root sessions and clean sessions", async () => {
  const dirtyRootSessions = new Set(["ses-root"])
  const prompts: Array<{ sessionID: string; text: string }> = []

  const deps = {
    directory: ROOT,
    rootSessionID: async (sessionID: string) => (sessionID === "ses-root" ? "ses-root" : undefined),
    dirtyRootSessions,
    fixLoopGuard: { active: false },
    prompt: async (sessionID: string, text: string) => { prompts.push({ sessionID, text }) },
    run: async () => ({ stdout: "", stderr: "" }),
  }

  // Sub-session idle: not the root, must be ignored.
  await handleIdleEvent({ data: { sessionID: "ses-child" } }, deps)
  // Root session idle but never dirtied: must be ignored.
  await handleIdleEvent({ data: { sessionID: "ses-other" } }, deps)

  expect(prompts).toEqual([])
  expect(dirtyRootSessions.has("ses-root")).toBe(true)
})

test("idle handler respects the fix-loop guard", async () => {
  const dirtyRootSessions = new Set(["ses-root"])
  const prompts: Array<{ sessionID: string; text: string }> = []
  const clippyError = new Error("cargo clippy failed") as Error & { stderr: string }
  clippyError.stderr = "error: unused variable: `x`\n"

  const deps = {
    directory: ROOT,
    rootSessionID: async (sessionID: string) => (sessionID === "ses-root" ? "ses-root" : undefined),
    dirtyRootSessions,
    fixLoopGuard: { active: false },
    prompt: async (sessionID: string, text: string) => { prompts.push({ sessionID, text }) },
    run: async () => { throw clippyError },
  }

  await handleIdleEvent({ data: { sessionID: "ses-root" } }, deps)
  expect(prompts.length).toBe(1)

  // Second failure while the guard is still active: no second prompt.
  dirtyRootSessions.add("ses-root")
  await handleIdleEvent({ data: { sessionID: "ses-root" } }, deps)
  expect(prompts.length).toBe(1)
})
