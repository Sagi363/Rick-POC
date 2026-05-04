import { useEffect, useState } from 'react'
import type { InstallStatusDTO } from '@shared/ipc'

interface Props {
  onClose: () => void
}

export function InstallModal({ onClose }: Props): JSX.Element {
  const [status, setStatus] = useState<InstallStatusDTO | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    void window.rcc.getInstallStatus().then(setStatus)
  }, [])

  const install = async (): Promise<void> => {
    setBusy(true)
    try {
      const next = await window.rcc.runInstall()
      setStatus(next)
    } finally {
      setBusy(false)
    }
  }

  const uninstall = async (): Promise<void> => {
    setBusy(true)
    try {
      const next = await window.rcc.runUninstall()
      setStatus(next)
    } finally {
      setBusy(false)
    }
  }

  if (!status) return <Backdrop>Loading…</Backdrop>

  return (
    <Backdrop>
      <div className="flex max-h-[80vh] w-[760px] flex-col rounded-lg border border-zinc-700 bg-zinc-900 shadow-xl">
        <header className="shrink-0 border-b border-zinc-800 px-5 py-3">
          <h2 className="text-sm font-medium text-zinc-100">
            {status.installed ? 'Plugin installed' : 'Install Rick Command Center plugin'}
          </h2>
          <p className="mt-1 text-xs text-zinc-400">
            {status.installed
              ? 'Hooks are registered in ~/.claude/settings.json. You can uninstall to remove them.'
              : 'This will copy hook scripts to your user config and merge entries into ~/.claude/settings.json. A backup is created first.'}
          </p>
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto p-5">
          <div className="mb-3 text-[11px] uppercase tracking-wider text-zinc-500">
            Settings.json diff
          </div>
          <div className="grid grid-cols-2 gap-3 text-[11px]">
            <Pre title="Before" content={status.diff.before} />
            <Pre title="After (proposed)" content={status.diff.after} />
          </div>
          <div className="mt-4 rounded-md border border-zinc-800 bg-zinc-950 p-3 text-[11px] text-zinc-400">
            <div className="mb-1 text-zinc-300">Hook commands</div>
            <ul className="space-y-1 font-mono">
              {Object.entries(status.hookCommands).map(([k, v]) => (
                <li key={k}>
                  <span className="text-emerald-400">{k}</span>: {v}
                </li>
              ))}
            </ul>
          </div>
        </div>
        <footer className="flex shrink-0 items-center justify-between border-t border-zinc-800 px-5 py-3">
          <button
            onClick={onClose}
            className="rounded-md border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:border-zinc-500"
          >
            Close
          </button>
          <div className="flex gap-2">
            {status.installed ? (
              <button
                disabled={busy}
                onClick={uninstall}
                className="rounded-md border border-rose-700 bg-rose-900/40 px-3 py-1.5 text-xs text-rose-200 hover:bg-rose-900/70 disabled:opacity-50"
              >
                {busy ? 'Working…' : 'Uninstall'}
              </button>
            ) : (
              <button
                disabled={busy}
                onClick={install}
                className="rounded-md bg-emerald-600 px-3 py-1.5 text-xs text-white hover:bg-emerald-500 disabled:opacity-50"
              >
                {busy ? 'Installing…' : 'Install plugin'}
              </button>
            )}
          </div>
        </footer>
      </div>
    </Backdrop>
  )
}

function Pre({ title, content }: { title: string; content: string }): JSX.Element {
  return (
    <div className="overflow-hidden rounded-md border border-zinc-800">
      <div className="border-b border-zinc-800 bg-zinc-950 px-2 py-1 text-[10px] uppercase tracking-wider text-zinc-500">
        {title}
      </div>
      <pre className="max-h-[300px] overflow-auto p-2 font-mono text-[11px] text-zinc-300">
        {content || '(empty)'}
      </pre>
    </div>
  )
}

function Backdrop({ children }: { children: React.ReactNode }): JSX.Element {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      {children}
    </div>
  )
}
