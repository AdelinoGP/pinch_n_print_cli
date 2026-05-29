import type { Plugin, Hooks } from "@opencode-ai/plugin"
import type { Event } from "@opencode-ai/sdk"

export const ProjectHooks: Plugin = async ({ client, $, directory }) => {
  let dirtyThisTurn = false
  let fixLoopGuard = false

  function resetFixGuardAfterDelay() {
    setTimeout(() => { fixLoopGuard = false }, 30_000)
  }

  return {
    "tool.execute.after": async (input, output) => {
      const tool = String(input?.tool ?? "")
      if (!/^(edit|write|multiedit|bash|task)$/i.test(tool)) return

      dirtyThisTurn = true

      const args = input?.args as Record<string, unknown> | undefined
      const filePath = String(args?.filePath ?? "")
      if (filePath.endsWith(".rs")) {
        try { await $`rustfmt "${filePath}"`.quiet() } catch { /* noop */ }
      }
    },

    event: async ({ event }: { event: Event }) => {
      if (event.type !== "session.idle") return
      if (!dirtyThisTurn) return
      dirtyThisTurn = false

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
        await $`bash ./modules/core-modules/build-core-modules.sh --check`.cwd(directory).quiet()
        await $`bash ./test-guests/build-test-guests.sh --check`.cwd(directory).quiet()
      } catch (err: unknown) {
        const stderr = String((err as { stderr?: { toString(): string } })?.stderr ?? err)
        if (stderr.trim()) {
          issues.push(`WASM staleness:\n\`\`\`\n${stderr.trim()}\n\`\`\``)
        }
      }

      const sessionID = (event as { properties?: { sessionID?: string } }).properties?.sessionID
      if (issues.length > 0 && sessionID && !fixLoopGuard) {
        fixLoopGuard = true
        resetFixGuardAfterDelay()
        await client.session.prompt({
          path: { id: sessionID },
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
