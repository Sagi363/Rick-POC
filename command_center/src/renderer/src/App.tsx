import { useEffect, useMemo, useRef, useState } from 'react'
import { useAppState } from './state'
import { TopBar } from './components/TopBar'
import { Drawer } from './components/Drawer'
import { FileList, SUMMARY_VIRTUAL_PATH } from './components/FileList'
import { FilePreview } from './components/FilePreview'
import { ProgressPanel } from './components/ProgressPanel'
import { SessionSummary } from './components/SessionSummary'
import { SettingsModal } from './components/SettingsModal'
import { InstallModal } from './components/InstallModal'
import { LaunchModal } from './components/LaunchModal'
import { TerminalsPanel } from './components/TerminalsPanel'
import { Resizer, usePersistedSize } from './components/Resizer'
import type { Workflow } from '@shared/types'
import clsx from 'clsx'

export default function App(): JSX.Element {
  const s = useAppState()
  const [showSettings, setShowSettings] = useState(false)
  const [showInstall, setShowInstall] = useState(false)
  const [launching, setLaunching] = useState<Workflow | null>(null)
  const [toast, setToast] = useState<string | null>(null)

  const drawer = usePersistedSize('drawer', 288, 220, 480, 'x')
  const filelist = usePersistedSize('filelist', 256, 180, 420, 'x')
  const previewSize = usePersistedSize('preview-pct', 50, 25, 85, 'y')
  const terminalSize = usePersistedSize('terminal-pct', 50, 15, 85, 'y')
  const [progressCollapsed, setProgressCollapsed] = useState<boolean>(() => {
    const v = window.localStorage.getItem('rcc:progress:collapsed')
    return v === '1'
  })
  const [terminalCollapsed, setTerminalCollapsed] = useState<boolean>(() => {
    const v = window.localStorage.getItem('rcc:terminal:collapsed')
    return v === '1'
  })

  useEffect(() => {
    window.localStorage.setItem('rcc:progress:collapsed', progressCollapsed ? '1' : '0')
  }, [progressCollapsed])

  useEffect(() => {
    window.localStorage.setItem('rcc:terminal:collapsed', terminalCollapsed ? '1' : '0')
  }, [terminalCollapsed])

  const sessionPtys = useMemo(
    () =>
      s.ptys.filter(
        (p) => p.sessionId === s.selectedSessionId || (!p.sessionId && !!s.selectedSessionId)
      ),
    [s.ptys, s.selectedSessionId]
  )
  const hasPtys = sessionPtys.length > 0
  const showTerminal = hasPtys && !terminalCollapsed
  const orphanCount = useMemo(() => s.ptys.filter((p) => !p.sessionId).length, [s.ptys])


  useEffect(() => {
    void window.rcc.getInstallStatus().then((st) => {
      if (!st.installed) setShowInstall(true)
    })
  }, [])

  const selected = s.sessions.find((x) => x.id === s.selectedSessionId) ?? null

  const recentCwds = useMemo(() => {
    const seen = new Map<string, number>()
    for (const sess of s.sessions) {
      if (!sess.cwd) continue
      const prior = seen.get(sess.cwd) ?? 0
      if (sess.lastActivity > prior) seen.set(sess.cwd, sess.lastActivity)
    }
    return Array.from(seen.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([cwd]) => cwd)
  }, [s.sessions])

  if (!s.settings) return <Splash />

  return (
    <div className="flex h-full flex-col">
      <TopBar
        universes={s.universes}
        active={s.settings.lastUniverse}
        onChange={(u) => void s.setLastUniverse(u)}
        onOpenSettings={() => setShowSettings(true)}
      />
      <div className="flex min-h-0 flex-1">
        <div style={{ width: drawer.size }} className="shrink-0">
          <Drawer
            sessions={s.sessions}
            workflows={s.workflows}
            activeUniverse={s.settings.lastUniverse}
            selectedId={s.selectedSessionId}
            onSelect={(id) => {
              s.setSelectedSessionId(id)
              s.setSelectedFile(null)
            }}
            onDiscard={(id) => {
              void window.rcc.discardSession(id)
              if (s.selectedSessionId === id) {
                s.setSelectedSessionId(null)
                s.setSelectedFile(null)
              }
            }}
            onLaunchWorkflow={(w) => setLaunching(w)}
            onFocusTerminal={async (id) => {
              const sess = s.sessions.find((x) => x.id === id)
              if (!sess?.cwd) return
              const r = await window.rcc.focusTerminal({
                cwd: sess.cwd,
                sessionId: id,
                terminalApp: s.settings!.terminalApp,
                skipPermissions: s.settings!.skipPermissions
              })
              if (!r.ok) {
                setToast(`Could not focus terminal: ${r.error ?? 'no match'}`)
              } else if (r.resumed) {
                setToast(
                  `Opened a fresh tab with \`claude --resume\` for ${sess.title ?? id.slice(0, 8)}.`
                )
              }
            }}
            onJumpToSession={(id) => {
              s.setSelectedSessionId(id)
              s.setSelectedFile(null)
            }}
            onSetAutoContinue={(id, auto) => {
              const pty = s.ptys.find((p) => p.sessionId === id && p.alive)
              if (!pty) {
                setToast('No active terminal for this session — open the in-app terminal first.')
                return
              }
              const directive = auto
                ? '/btw From now on, run remaining phases with `auto_continue: true` — do not pause between phases or wait for me to say `next`. Drive the workflow end-to-end.'
                : '/btw From now on, run remaining phases with `auto_continue: false` — pause after each phase and wait for my `next` before continuing.'
              void window.rcc.ptyWrite(pty.id, `${directive}\r`)
            }}
            onRenameSession={async (id, title) => {
              const next = { ...(s.settings!.customTitles ?? {}) }
              if (title === null || title.trim() === '') delete next[id]
              else next[id] = title.trim()
              await s.patchSettings({ customTitles: next })
            }}
            settings={s.settings}
          />
        </div>
        <Resizer axis="x" onMouseDown={drawer.onMouseDown} />

        <div style={{ width: filelist.size }} className="shrink-0">
          <FileList
            sessionId={s.selectedSessionId}
            sessionCwd={selected?.cwd ?? null}
            sessionLastActivity={selected?.lastActivity ?? null}
            selectedFile={s.selectedFile}
            onSelect={s.setSelectedFile}
          />
        </div>
        <Resizer axis="x" onMouseDown={filelist.onMouseDown} />

        <main className="flex min-h-0 flex-1 flex-col">
          {/* Top: file preview / summary */}
          <div
            style={{ height: `${previewSize.size}%` }}
            className="flex min-h-0 shrink-0 flex-col"
          >
            {s.selectedFile === SUMMARY_VIRTUAL_PATH && selected ? (
              <SessionSummary session={selected} />
            ) : (
              <FilePreview path={s.selectedFile === SUMMARY_VIRTUAL_PATH ? null : s.selectedFile} />
            )}
          </div>

          <Resizer axis="y" onMouseDown={previewSize.onMouseDown} />

          {/* Bottom: terminal takes priority when active; otherwise activity panel. */}
          <div className="flex min-h-0 flex-1 flex-col">
            {hasPtys ? (
              showTerminal ? (
                <div className="flex min-h-0 flex-1 flex-col">
                  <TerminalsPanel
                    ptys={sessionPtys}
                    selectedSessionId={s.selectedSessionId}
                    sessionStatus={selected?.status}
                    collapsed={false}
                    onToggle={() => setTerminalCollapsed(true)}
                  />
                </div>
              ) : (
                <TerminalsPanel
                  ptys={sessionPtys}
                  selectedSessionId={s.selectedSessionId}
                  sessionStatus={selected?.status}
                  collapsed
                  onToggle={() => setTerminalCollapsed(false)}
                />
              )
            ) : progressCollapsed ? (
              <ProgressPanel
                session={selected}
                collapsed
                onToggle={() => setProgressCollapsed(false)}
              />
            ) : (
              <div className="flex min-h-0 flex-1 flex-col">
                <ProgressPanel
                  session={selected}
                  collapsed={false}
                  onToggle={() => setProgressCollapsed(true)}
                />
              </div>
            )}
          </div>
        </main>
      </div>
      {showSettings && (
        <SettingsModal
          settings={s.settings}
          onClose={() => setShowSettings(false)}
          onSave={s.patchSettings}
        />
      )}
      {showInstall && <InstallModal onClose={() => setShowInstall(false)} />}
      {launching && (
        <LaunchModal
          workflow={launching}
          defaultCwd={selected?.cwd}
          recentCwds={recentCwds}
          branchPrefix={s.settings.branchPrefix}
          defaultBranchOff={s.settings.defaultBranchOff}
          onClose={() => setLaunching(null)}
          onLaunched={() => setToast(`Launched ${launching.name} in Terminal — session will appear shortly.`)}
        />
      )}
      {toast && (
        <div
          onClick={() => setToast(null)}
          className="fixed bottom-4 right-4 z-50 max-w-md cursor-pointer rounded-md border border-emerald-700 bg-emerald-900/80 px-3 py-2 text-xs text-emerald-100 shadow-lg"
        >
          {toast}
        </div>
      )}
    </div>
  )
}

function Splash(): JSX.Element {
  return (
    <div className="flex h-full items-center justify-center text-xs text-zinc-500">
      Loading…
    </div>
  )
}
