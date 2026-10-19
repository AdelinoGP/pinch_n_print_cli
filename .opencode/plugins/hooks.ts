import { execFile } from "node:child_process"
import { promisify } from "node:util"

const execFileAsync = promisify(execFile)

const PATH_TOOLS = new Set(["read", "grep", "glob"])

const IS_WINDOWS = process.platform === "win32"

const CONTROL_CHARS = /[\u0000-\u001F]/

const MISROOTED_PARENT_DIRS = new Set([
  "crates",
  "docs",
  "modules",
  "xtask",
  "resources",
  "tmp",
  "target",
])

function normalizeSlashes(path: string): string {
  return IS_WINDOWS ? path.replace(/\\/g, "/") : path
}

function parentDirOf(root: string): string {
  return root.slice(0, Math.max(root.lastIndexOf("/"), 0))
}

function pathEquals(a: string, b: string): boolean {
  return IS_WINDOWS ? a.toLowerCase() === b.toLowerCase() : a === b
}

function pathStartsWith(path: string, prefix: string): boolean {
  return IS_WINDOWS
    ? path.toLowerCase().startsWith(prefix.toLowerCase())
    : path.startsWith(prefix)
}

function isAbsolutePath(path: string): boolean {
  if (IS_WINDOWS) return /^(?:[A-Za-z]:)?\//.test(path)
  return path.startsWith("/")
}

export type PathCorrection =
  | { corrected: string }
  | { reason: string }
  | undefined

export function correctToolPath(
  value: string,
  directory: string,
  tool: string,
): PathCorrection {
  const input = normalizeSlashes(value)
  const root = normalizeSlashes(directory).replace(/\/+$/, "")
  const parent = parentDirOf(root)

  if (pathStartsWith(input, parent + "/")) {
    const remainder = input.slice(parent.length + 1)
    const topLevel = remainder.split("/", 1)[0]
    if (topLevel && MISROOTED_PARENT_DIRS.has(topLevel)) {
      return { reason: `${input} is not under the active workspace ${root}. Reissue with a path rooted at ${root}.` }
    }
  }

  const siblingRoots = [parent + "/pinch_n_print", parent + "/pinch_n_print_cli"]
  for (const sibling of siblingRoots) {
    if (pathEquals(sibling, root)) continue
    if (pathStartsWith(input, sibling + "/")) {
      return { reason: `${input} is a sibling worktree of the active workspace ${root}. Reissue with a path rooted at ${root}.` }
    }
  }

  if (isAbsolutePath(input)) return { corrected: input }

  if (tool === "read" || tool === "grep" || tool === "glob") {
    return { corrected: `${root}/${input.replace(/^(?:\.\/)+/, "")}` }
  }

  return { corrected: input }
}

export interface RunResult {
  stdout: string
  stderr: string
}

export type RunCommand = (
  cwd: string,
  cmd: string,
  args: string[],
) => Promise<RunResult>

async function runCommand(cwd: string, cmd: string, args: string[]): Promise<RunResult> {
  try {
    const { stdout, stderr } = await execFileAsync(cmd, args, {
      cwd,
      maxBuffer: 16 * 1024 * 1024,
    })
    return { stdout, stderr }
  } catch (err: unknown) {
    const cause = err as { stdout?: unknown; stderr?: unknown; message?: string }
    const wrapped = new Error(String(cause.message ?? err)) as Error & RunResult
    wrapped.stdout = String(cause.stdout ?? "")
    wrapped.stderr = String(cause.stderr ?? "")
    throw wrapped
  }
}

export interface ToolAfterDeps {
  directory: string
  rootSessionID: (sessionID: string) => Promise<string | undefined>
  dirtyRootSessions: Set<string>
  run: RunCommand
}

export interface ToolAfterEvent {
  tool: string
  sessionID: string
  input: unknown
}

export async function handleToolAfter(
  event: ToolAfterEvent,
  deps: ToolAfterDeps,
): Promise<void> {
  if (!/^(edit|write|multiedit|bash|task)$/i.test(event.tool)) return

  const root = await deps.rootSessionID(event.sessionID)
  if (root) deps.dirtyRootSessions.add(root)

  const input = event.input as Record<string, unknown> | undefined
  const filePath = String(input?.filePath ?? "")
  if (filePath.endsWith(".rs")) {
    try { await deps.run(deps.directory, "rustfmt", [filePath]) } catch { /* noop */ }
  }
}

export interface IdleDeps {
  directory: string
  rootSessionID: (sessionID: string) => Promise<string | undefined>
  dirtyRootSessions: Set<string>
  fixLoopGuard: { active: boolean }
  prompt: (sessionID: string, text: string) => Promise<void>
  run: RunCommand
}

export interface IdleEvent {
  data: { sessionID: string }
}

export async function handleIdleEvent(
  event: IdleEvent,
  deps: IdleDeps,
): Promise<void> {
  const idleSessionID = event.data.sessionID
  if (!idleSessionID) return

  const root = await deps.rootSessionID(idleSessionID)
  if (!root || root !== idleSessionID) return

  if (!deps.dirtyRootSessions.has(root)) return
  deps.dirtyRootSessions.delete(root)

  try { await deps.run(deps.directory, "cargo", ["fmt"]) } catch { /* noop */ }

  const issues: string[] = []

  try {
    await deps.run(deps.directory, "cargo", [
      "clippy", "--all-targets", "--message-format=short", "--", "-D", "warnings",
    ])
  } catch (err: unknown) {
    const stderr = String((err as { stderr?: string })?.stderr ?? err)
    const diags = stderr.split("\n").filter((line: string) =>
      /\b(?:error|warning)\b.*:/.test(line) || line.startsWith("error:") || line.startsWith("could not compile")
    )
    if (diags.length > 0) {
      issues.push(`cargo clippy:\n\`\`\`\n${diags.join("\n")}\n\`\`\``)
    }
  }

  try {
    await deps.run(deps.directory, "cargo", ["xtask", "build-guests", "--check"])
  } catch (err: unknown) {
    const stderr = String((err as { stderr?: string })?.stderr ?? err)
    if (stderr.trim()) {
      issues.push(`WASM staleness:\n\`\`\`\n${stderr.trim()}\n\`\`\``)
    }
  }

  if (issues.length > 0 && !deps.fixLoopGuard.active) {
    deps.fixLoopGuard.active = true
    setTimeout(() => { deps.fixLoopGuard.active = false }, 30_000)
    await deps.prompt(
      root,
      `Your last changes introduced the following issues:\n\n${issues.join("\n")}\n\nPlease fix these issues.`,
    )
  }
}

export default {
  id: "pinch-n-print.hooks",
  async setup(ctx: {
    location: { directory: string }
    tool: {
      hook: (
        name: "execute.before" | "execute.after",
        callback: (event: { tool: string; sessionID: string; input: unknown }) => Promise<void> | void,
      ) => Promise<{ dispose: () => Promise<void> }>
    }
    shell: {
      hook: (
        name: "create.before",
        callback: (event: { cwd: string }) => Promise<void> | void,
      ) => Promise<{ dispose: () => Promise<void> }>
    }
    event: {
      subscribe: (options?: { signal?: AbortSignal }) => AsyncIterable<{ type: string; data: { sessionID: string } }>
    }
    session: {
      get: (input: { sessionID: string }) => Promise<{ id: string; parentID?: string } | undefined>
      prompt: (input: { sessionID: string; text: { text: string } }) => Promise<unknown>
    }
  }) {
    const directory = ctx.location.directory
    const dirtyRootSessions = new Set<string>()
    const rootCache = new Map<string, string>()
    const fixLoopGuard = { active: false }

    async function rootSessionID(sessionID: string): Promise<string | undefined> {
      const cached = rootCache.get(sessionID)
      if (cached !== undefined) return cached

      let current = sessionID
      let root: string | undefined
      for (let depth = 0; depth < 8; depth++) {
        const session = await ctx.session.get({ sessionID: current })
        if (!session) return undefined
        if (!session.parentID) {
          root = session.id
          break
        }
        current = session.parentID
      }
      if (root) rootCache.set(sessionID, root)
      return root
    }

    await ctx.tool.hook("execute.before", (event) => {
      if (!PATH_TOOLS.has(event.tool)) return

      const input = event.input as Record<string, unknown> | undefined
      if (!input) return
      const value = input.filePath ?? input.path
      if (typeof value !== "string" || !value.trim()) return

      const correction = correctToolPath(value, directory, event.tool)
      if (!correction) return
      if ("reason" in correction) {
        throw new Error(`Blocked ${event.tool} on path outside the active workspace: ${correction.reason}`)
      }
      if (input.filePath !== undefined) input.filePath = correction.corrected
      if (input.path !== undefined) input.path = correction.corrected
    })

    await ctx.shell.hook("create.before", (event) => {
      if (event.cwd && CONTROL_CHARS.test(event.cwd)) {
        throw new Error(
          `Blocked shell with a working directory containing control characters (valid paths never contain them): ${JSON.stringify(event.cwd)}`,
        )
      }
    })

    await ctx.tool.hook("execute.after", (event) =>
      handleToolAfter(event, { directory, rootSessionID, dirtyRootSessions, run: runCommand }),
    )

    const controller = new AbortController()
    void (async () => {
      for await (const event of ctx.event.subscribe({ signal: controller.signal })) {
        if (event.type !== "session.idle") continue
        await handleIdleEvent(event, {
          directory,
          rootSessionID,
          dirtyRootSessions,
          fixLoopGuard,
          prompt: (sessionID, text) => ctx.session.prompt({ sessionID, text: { text } }),
          run: runCommand,
        })
      }
    })()

    return () => controller.abort()
  },
}
