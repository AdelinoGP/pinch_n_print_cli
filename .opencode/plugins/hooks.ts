import type { Plugin, Hooks } from "@opencode-ai/plugin"
import type { Event } from "@opencode-ai/sdk"

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
