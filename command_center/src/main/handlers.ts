import { dialog, type BrowserWindow, type IpcMain } from 'electron'

function shellQuote(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`
}

function quoteShellSingle(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`
}

function pruneEmpty(obj: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const [k, v] of Object.entries(obj)) {
    if (v === undefined || v === null || v === '') continue
    out[k] = v
  }
  return out
}
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { IPC } from '@shared/ipc'
import { SettingsService } from './services/settings'
import { UniverseService } from './services/universes'
import { TranscriptService } from './services/transcripts'
import { TrackingService } from './services/tracking'
import { SessionsService } from './services/sessions'
import { PluginInstaller } from './services/install'
import { WorkflowLauncher } from './services/launcher'
import { PtyService } from './services/pty'

export interface Services {
  settings: SettingsService
  universes: UniverseService
  transcripts: TranscriptService
  tracking: TrackingService
  sessions: SessionsService
  installer: PluginInstaller
  launcher: WorkflowLauncher
  pty: PtyService
}

export async function buildServices(): Promise<Services> {
  const settings = new SettingsService()
  await settings.load()

  const universes = new UniverseService()
  const transcripts = new TranscriptService()
  const tracking = new TrackingService()
  const sessions = new SessionsService(transcripts, tracking)

  await Promise.all([universes.start(), transcripts.start(), tracking.start()])
  sessions.setWorkflowsLookup(() => universes.snapshot().workflows)
  sessions.start()
  sessions.setOptions({
    recentSessionDays: settings.get().recentSessionDays,
    universeFilter: settings.get().lastUniverse,
    archivedSessionIds: settings.get().archivedSessionIds,
    customTitles: settings.get().customTitles
  })

  const installer = new PluginInstaller()
  const launcher = new WorkflowLauncher()
  const pty = new PtyService()

  // Auto-bind unbound PTYs to newly-discovered sessions by cwd match.
  // Used by the in-app launch flow: PTY spawns first, JSONL appears milliseconds
  // later, and we link them so the terminal panel filters correctly.
  transcripts.onUpdate((info) => {
    if (!info.cwd || !info.sessionId) return
    if (pty.findBySession(info.sessionId)) return
    for (const handle of pty.list()) {
      if (handle.sessionId || !handle.alive) continue
      if (handle.cwd !== info.cwd) continue
      pty.bindSession(handle.id, info.sessionId)
      break
    }
  })

  return { settings, universes, transcripts, tracking, sessions, installer, launcher, pty }
}

export function registerHandlers(
  ipc: IpcMain,
  getWindow: () => BrowserWindow | null,
  s: Services
): void {
  ipc.handle(IPC.GetSettings, () => s.settings.get())
  ipc.handle(IPC.SetSettings, async (_e, patch) => {
    const next = await s.settings.patch(patch)
    s.sessions.setOptions({
      recentSessionDays: next.recentSessionDays,
      universeFilter: next.lastUniverse,
      archivedSessionIds: next.archivedSessionIds,
      customTitles: next.customTitles
    })
    return next
  })

  ipc.handle(IPC.GetUniverses, () => s.universes.snapshot().universes)
  ipc.handle(IPC.GetWorkflows, (_e, universe?: string) => {
    const all = s.universes.snapshot().workflows
    return universe ? all.filter((w) => w.universe === universe) : all
  })
  ipc.handle(IPC.GetSessions, () => s.sessions.snapshot())
  ipc.handle(IPC.ListSessionFiles, (_e, sessionId: string) => s.sessions.listSessionFiles(sessionId))
  ipc.handle(IPC.ReadTracking, (_e, sessionId: string) => s.tracking.readRaw(sessionId))

  ipc.handle(IPC.GetInstallStatus, () => s.installer.status())
  ipc.handle(IPC.RunInstall, () => s.installer.install())
  ipc.handle(IPC.RunUninstall, () => s.installer.uninstall())

  ipc.handle(IPC.DiscardSession, async (_e, id: string) => {
    await s.sessions.discardSession(id)
    const archived = (s.settings.get().archivedSessionIds ?? []).slice()
    if (!archived.includes(id)) archived.push(id)
    await s.settings.patch({ archivedSessionIds: archived })
  })
  ipc.handle(IPC.UnarchiveSession, async (_e, id: string) => {
    await s.sessions.unarchiveSession(id)
    const archived = (s.settings.get().archivedSessionIds ?? []).filter((x) => x !== id)
    await s.settings.patch({ archivedSessionIds: archived })
  })
  ipc.handle(IPC.GetSessionSummary, (_e, id: string) => s.sessions.summarize(id))

  ipc.handle(IPC.PtyList, () => s.pty.list())
  ipc.handle(IPC.PtyBuffer, (_e, id: string) => s.pty.buffer(id))
  ipc.handle(IPC.PtyWrite, (_e, id: string, data: string) => s.pty.write(id, data))
  ipc.handle(IPC.PtyResize, (_e, id: string, cols: number, rows: number) =>
    s.pty.resize(id, cols, rows)
  )
  ipc.handle(IPC.PtyKill, (_e, id: string) => s.pty.kill(id))
  ipc.handle(IPC.PtySpawn, (_e, req) => {
    const settings = s.settings.get()
    const skip = req.skipPermissions ?? settings.skipPermissions
    const skipFlag = skip ? ' --dangerously-skip-permissions' : ''
    let initialInput: string | undefined
    if (req.resume && req.sessionId) {
      initialInput = `claude${skipFlag} --resume ${shellQuote(req.sessionId)}\n`
    } else if (req.initialPrompt) {
      const escaped = req.initialPrompt.replace(/'/g, `'\\''`)
      initialInput = `claude${skipFlag} '${escaped}'\n`
    }
    return s.pty.spawn({
      cwd: req.cwd,
      label: req.label ?? (req.resume ? `resume ${req.sessionId?.slice(0, 8)}` : 'claude'),
      sessionId: req.sessionId,
      initialInput
    })
  })

  s.pty.on('data', (e) => getWindow()?.webContents.send(IPC.PtyData, e))
  s.pty.on('exit', (e) => getWindow()?.webContents.send(IPC.PtyExit, e))
  s.pty.on('list', (list) => getWindow()?.webContents.send(IPC.PtyListUpdate, list))

  ipc.handle(IPC.ReadFile, async (_e, p: string) => {
    const safe = resolve(p)
    return await readFile(safe, 'utf8')
  })

  ipc.handle(IPC.PickDirectory, async (_e, defaultPath?: string) => {
    const w = getWindow()
    if (!w) return null
    const result = await dialog.showOpenDialog(w, {
      properties: ['openDirectory'],
      defaultPath
    })
    return result.canceled || !result.filePaths[0] ? null : result.filePaths[0]
  })

  ipc.handle(IPC.LaunchWorkflow, async (_e, req) => {
    const settings = s.settings.get()
    const terminalApp = req.terminalApp ?? settings.terminalApp
    const skipPermissions = req.skipPermissions ?? settings.skipPermissions

    if (terminalApp === 'in-app') {
      // Resolve worktree (if any) by reusing the launcher; on success it
      // returns the cwd the PTY should live in.
      const ext = await s.launcher.prepareCwd({ ...req, skipPermissions, terminalApp: 'in-app' })
      if (!ext.ok) return ext
      const params = pruneEmpty(req.params)
      const paramsArg =
        Object.keys(params).length > 0 ? ` --params=${quoteShellSingle(JSON.stringify(params))}` : ''
      const head = `/rick run ${req.workflow}${paramsArg}`
      const extra = req.extraPrompt?.trim()
      const fullPrompt = extra ? `${head}\n\n${extra}` : head
      const skipFlag = skipPermissions ? ' --dangerously-skip-permissions' : ''
      const escaped = fullPrompt.replace(/'/g, `'\\''`)
      const initialInput = `claude${skipFlag} '${escaped}'\n`
      s.pty.spawn({
        cwd: ext.cwd,
        label: `${req.workflow}`,
        initialInput
      })
      return { ok: true, command: fullPrompt }
    }

    return s.launcher.launch({
      ...req,
      terminalApp,
      customTerminalCommand: req.customTerminalCommand ?? settings.customTerminalCommand,
      skipPermissions
    })
  })

  ipc.handle(IPC.FocusTerminal, (_e, req) => {
    const settings = s.settings.get()
    const terminalApp = req.terminalApp ?? settings.terminalApp
    const skipPermissions = req.skipPermissions ?? settings.skipPermissions

    if (terminalApp === 'in-app') {
      const existing = s.pty.findBySession(req.sessionId)
      if (existing) return Promise.resolve({ ok: true })
      const skipFlag = skipPermissions ? ' --dangerously-skip-permissions' : ''
      const escaped = req.sessionId.replace(/'/g, `'\\''`)
      const initialInput = `claude${skipFlag} --resume '${escaped}'\n`
      s.pty.spawn({
        cwd: req.cwd,
        sessionId: req.sessionId,
        label: `resume ${req.sessionId.slice(0, 8)}`,
        initialInput
      })
      return Promise.resolve({ ok: true, resumed: true })
    }

    return s.launcher.focus({
      ...req,
      terminalApp,
      customTerminalCommand: req.customTerminalCommand ?? settings.customTerminalCommand,
      skipPermissions
    })
  })

  // Push events to renderer.
  s.universes.onChange((universes, workflows) => {
    const w = getWindow()
    if (!w) return
    w.webContents.send(IPC.UniversesUpdate, universes)
    w.webContents.send(IPC.WorkflowsUpdate, workflows)
  })
  s.sessions.onUpdate((sessions) => {
    const w = getWindow()
    if (!w) return
    w.webContents.send(IPC.SessionsUpdate, sessions)
  })
}
