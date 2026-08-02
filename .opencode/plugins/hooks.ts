import type { Plugin, Hooks } from "@opencode-ai/plugin"
import type { Event } from "@opencode-ai/sdk"

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

export const ProjectHooks: Plugin = async ({ client, $, directory }) => {
  const dirtyRootSessions = new Set<string>()
  const rootCache = new Map<string, string>()
  let fixLoopGuard = false

  function resetFixGuardAfterDelay() {
    setTimeout(() => { fixLoopGuard = false }, 30_000)
  }

  async function rootSessionID(sessionID: string): Promise<string | undefined> {
    const cached = rootCache.get(sessionID)
    if (cached !== undefined) return cached

    let current = sessionID
    let root: string | undefined
    for (let depth = 0; depth < 8; depth++) {
      const res = await client.session.get({ path: { id: current } })
      const session = res?.data
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

  return {
    "tool.execute.before": async (input, output) => {
      const tool = String(input?.tool ?? "")
      if (tool === "bash") {
        const args = output.args as Record<string, unknown> | undefined
        const workdir = String(args?.workdir ?? "")
        if (workdir && CONTROL_CHARS.test(workdir)) {
          throw new Error(
            `Blocked bash with a workdir containing control characters (valid paths never contain them): ${JSON.stringify(workdir)}`,
          )
        }
        return
      }
      if (!PATH_TOOLS.has(tool)) return

      const args = output.args as Record<string, unknown> | undefined
      const value = args?.filePath ?? args?.path
      if (typeof value !== "string" || !value.trim()) return

      const correction = correctToolPath(value, directory, tool)
      if (!correction) return
      if ("reason" in correction) {
        throw new Error(`Blocked ${tool} on path outside the active workspace: ${correction.reason}`)
      }
      if (args?.filePath !== undefined) args.filePath = correction.corrected
      if (args?.path !== undefined) args.path = correction.corrected
    },

    "tool.execute.after": async (input, output) => {
      const tool = String(input?.tool ?? "")
      if (!/^(edit|write|multiedit|bash|task)$/i.test(tool)) return

      const sessionID = String(input?.sessionID ?? "")
      if (sessionID) {
        const root = await rootSessionID(sessionID)
        if (root) dirtyRootSessions.add(root)
      }

      const args = input?.args as Record<string, unknown> | undefined
      const filePath = String(args?.filePath ?? "")
      if (filePath.endsWith(".rs")) {
        try { await $`rustfmt "${filePath}"`.quiet() } catch { /* noop */ }
      }
    },

    event: async ({ event }: { event: Event }) => {
      if (event.type !== "session.idle") return

      const idleSessionID = String(
        (event as { properties?: { sessionID?: string } }).properties?.sessionID ?? ""
      )
      if (!idleSessionID) return

      const root = await rootSessionID(idleSessionID)
      if (!root || root !== idleSessionID) return

      if (!dirtyRootSessions.has(root)) return
      dirtyRootSessions.delete(root)

      try { await $`cargo fmt`.cwd(directory).quiet() } catch { /* noop */ }

      const issues: string[] = []

      try {
        await $`cargo clippy --all-targets --message-format=short -- -D warnings`.cwd(directory).quiet()
      } catch (err: unknown) {
        const stderr = String((err as { stderr?: { toString(): string } })?.stderr ?? err)
        const diags = stderr.split("\n").filter((line: string) =>
          /\b(?:error|warning)\b.*:/.test(line) || line.startsWith("error:") || line.startsWith("could not compile")
        )
        if (diags.length > 0) {
          issues.push(`cargo clippy:\n\`\`\`\n${diags.join("\n")}\n\`\`\``)
        }
      }

      try {
        await $`cargo xtask build-guests --check`.cwd(directory).quiet()
      } catch (err: unknown) {
        const stderr = String((err as { stderr?: { toString(): string } })?.stderr ?? err)
        if (stderr.trim()) {
          issues.push(`WASM staleness:\n\`\`\`\n${stderr.trim()}\n\`\`\``)
        }
      }

      if (issues.length > 0 && !fixLoopGuard) {
        fixLoopGuard = true
        resetFixGuardAfterDelay()
        await client.session.prompt({
          path: { id: root },
          body: {
            parts: [{
              type: "text",
              text: `Your last changes introduced the following issues:\n\n${issues.join("\n")}\n\nPlease fix these issues.`,
            }],
          },
        })
      }
    },
  } satisfies Hooks
}
