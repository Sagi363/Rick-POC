import { app } from 'electron'
import { copyFile, mkdir, readdir, readFile, rename, stat, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { CLAUDE_SETTINGS } from './paths'

const RCC_MARKER = '# rcc-hook'

interface HookEntry {
  matcher?: string
  hooks: { type: string; command: string; timeout?: number }[]
}

interface ClaudeSettings {
  hooks?: Record<string, HookEntry[]>
  [k: string]: unknown
}

export interface InstallStatus {
  installed: boolean
  pluginSourceDir: string
  pluginInstallDir: string
  hookCommands: Record<string, string>
  diff: { before: string; after: string }
}

export class PluginInstaller {
  pluginInstallDir(): string {
    return join(app.getPath('userData'), 'plugin')
  }

  pluginSourceDir(): string {
    if (app.isPackaged) return join(process.resourcesPath, 'plugin')
    return resolve(app.getAppPath(), 'plugin')
  }

  async status(): Promise<InstallStatus> {
    const sourceDir = this.pluginSourceDir()
    const installDir = this.pluginInstallDir()
    const settings = await this.readSettings()
    const installed = this.isInstalled(settings)
    const proposed = this.merge(structuredClone(settings ?? {}), installDir)
    return {
      installed,
      pluginSourceDir: sourceDir,
      pluginInstallDir: installDir,
      hookCommands: this.hookCommands(installDir),
      diff: {
        before: JSON.stringify(settings ?? {}, null, 2),
        after: JSON.stringify(proposed, null, 2)
      }
    }
  }

  async install(): Promise<InstallStatus> {
    const sourceDir = this.pluginSourceDir()
    const installDir = this.pluginInstallDir()
    await this.copyTree(sourceDir, installDir)

    const current = (await this.readSettings()) ?? {}
    await this.backupSettings()
    const next = this.merge(structuredClone(current), installDir)
    await this.writeSettingsAtomic(next)
    return await this.status()
  }

  async uninstall(): Promise<InstallStatus> {
    const current = (await this.readSettings()) ?? {}
    if (current.hooks) {
      for (const [event, entries] of Object.entries(current.hooks)) {
        const filtered = (entries as HookEntry[])
          .map((e) => ({
            ...e,
            hooks: e.hooks.filter((h) => !h.command.includes(RCC_MARKER))
          }))
          .filter((e) => e.hooks.length > 0)
        if (filtered.length === 0) delete current.hooks[event]
        else current.hooks[event] = filtered
      }
    }
    await this.backupSettings()
    await this.writeSettingsAtomic(current)
    return await this.status()
  }

  private hookCommands(installDir: string): Record<string, string> {
    const node = 'node'
    const q = (p: string): string => `"${p.replace(/"/g, '\\"')}"`
    return {
      UserPromptSubmit: `${node} ${q(join(installDir, 'hooks/user-prompt-submit.mjs'))} ${RCC_MARKER}`,
      PostToolUse: `${node} ${q(join(installDir, 'hooks/post-tool-use.mjs'))} ${RCC_MARKER}`,
      Notification: `${node} ${q(join(installDir, 'hooks/notification.mjs'))} ${RCC_MARKER}`,
      Stop: `${node} ${q(join(installDir, 'hooks/stop.mjs'))} ${RCC_MARKER}`
    }
  }

  private isInstalled(settings: ClaudeSettings | null): boolean {
    if (!settings?.hooks) return false
    for (const entries of Object.values(settings.hooks)) {
      for (const e of entries) {
        if (e.hooks.some((h) => h.command.includes(RCC_MARKER))) return true
      }
    }
    return false
  }

  private merge(settings: ClaudeSettings, installDir: string): ClaudeSettings {
    settings.hooks ||= {}
    const cmds = this.hookCommands(installDir)

    const ensure = (event: string, matcher: string | undefined, command: string): void => {
      const list = (settings.hooks![event] ||= [])
      const existing = list.find((e) => (e.matcher ?? '') === (matcher ?? ''))
      const entry = existing ?? { matcher, hooks: [] as HookEntry['hooks'] }
      const already = entry.hooks.some((h) => h.command.includes(RCC_MARKER))
      if (!already) entry.hooks.push({ type: 'command', command })
      if (!existing) list.push(entry)
    }

    ensure('UserPromptSubmit', '', cmds.UserPromptSubmit)
    ensure('PostToolUse', 'TodoWrite|Task', cmds.PostToolUse)
    ensure('Notification', '', cmds.Notification)
    ensure('Stop', '', cmds.Stop)
    return settings
  }

  private async readSettings(): Promise<ClaudeSettings | null> {
    try {
      const raw = await readFile(CLAUDE_SETTINGS, 'utf8')
      return JSON.parse(raw) as ClaudeSettings
    } catch {
      return null
    }
  }

  private async backupSettings(): Promise<void> {
    try {
      await stat(CLAUDE_SETTINGS)
    } catch {
      return
    }
    const backup = `${CLAUDE_SETTINGS}.rcc.bak.${Date.now()}`
    await copyFile(CLAUDE_SETTINGS, backup)
  }

  private async writeSettingsAtomic(settings: ClaudeSettings): Promise<void> {
    const tmp = `${CLAUDE_SETTINGS}.${process.pid}.${Date.now()}.tmp`
    await writeFile(tmp, JSON.stringify(settings, null, 2) + '\n', 'utf8')
    await rename(tmp, CLAUDE_SETTINGS)
  }

  private async copyTree(src: string, dst: string): Promise<void> {
    await mkdir(dst, { recursive: true })
    let entries: string[] = []
    try {
      entries = await readdir(src)
    } catch {
      throw new Error(`Plugin source missing at ${src}`)
    }
    for (const name of entries) {
      const s = await stat(join(src, name))
      if (s.isDirectory()) await this.copyTree(join(src, name), join(dst, name))
      else await copyFile(join(src, name), join(dst, name))
    }
  }
}
