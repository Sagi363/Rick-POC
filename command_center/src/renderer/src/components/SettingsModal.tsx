import { useState } from 'react'
import type { AppSettings } from '@shared/types'

interface Props {
  settings: AppSettings
  onClose: () => void
  onSave: (patch: Partial<AppSettings>) => Promise<void>
}

export function SettingsModal({ settings, onClose, onSave }: Props): JSX.Element {
  const [warn, setWarn] = useState(Math.round(settings.warnThreshold * 100))
  const [crit, setCrit] = useState(Math.round(settings.criticalThreshold * 100))
  const [days, setDays] = useState(settings.recentSessionDays)
  const [branchPrefix, setBranchPrefix] = useState(settings.branchPrefix)
  const [defaultBranchOff, setDefaultBranchOff] = useState(settings.defaultBranchOff)
  const [terminalApp, setTerminalApp] = useState(settings.terminalApp)
  const [customTerminalCommand, setCustomTerminalCommand] = useState(
    settings.customTerminalCommand ?? ''
  )
  const [skipPermissions, setSkipPermissions] = useState(settings.skipPermissions)
  const [saving, setSaving] = useState(false)

  const save = async (): Promise<void> => {
    setSaving(true)
    await onSave({
      warnThreshold: warn / 100,
      criticalThreshold: crit / 100,
      recentSessionDays: days,
      branchPrefix,
      defaultBranchOff,
      terminalApp,
      customTerminalCommand: customTerminalCommand.trim() || undefined,
      skipPermissions
    })
    setSaving(false)
    onClose()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="w-[420px] rounded-lg border border-zinc-700 bg-zinc-900 p-5 shadow-xl">
        <h2 className="mb-4 text-sm font-medium text-zinc-100">Settings</h2>
        <Field label={`Warn threshold (${warn}%)`}>
          <input
            type="range"
            min={10}
            max={100}
            value={warn}
            onChange={(e) => setWarn(Number(e.target.value))}
            className="w-full"
          />
        </Field>
        <Field label={`Critical threshold (${crit}%)`}>
          <input
            type="range"
            min={10}
            max={100}
            value={crit}
            onChange={(e) => setCrit(Number(e.target.value))}
            className="w-full"
          />
        </Field>
        <Field label="Recent-session window (days)">
          <input
            type="number"
            min={1}
            max={90}
            value={days}
            onChange={(e) => setDays(Number(e.target.value) || 1)}
            className="w-24 rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1 text-xs text-zinc-200"
          />
        </Field>
        <Field label="Worktree branch prefix">
          <input
            type="text"
            value={branchPrefix}
            onChange={(e) => setBranchPrefix(e.target.value)}
            placeholder="feature/"
            className="w-48 rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1 font-mono text-xs text-zinc-200"
          />
          <div className="mt-1 text-[10px] text-zinc-500">
            Auto-suggested branch name = <span className="font-mono">{branchPrefix || ''}&lt;slug&gt;</span>. Leave blank for no prefix.
          </div>
        </Field>
        <Field label="Default 'branch off' base">
          <input
            type="text"
            value={defaultBranchOff}
            onChange={(e) => setDefaultBranchOff(e.target.value)}
            placeholder="dev"
            className="w-48 rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1 font-mono text-xs text-zinc-200"
          />
          <div className="mt-1 text-[10px] text-zinc-500">
            Default base branch when creating a worktree. Blank = current HEAD.
          </div>
        </Field>
        <Field label="Terminal app">
          <select
            value={terminalApp}
            onChange={(e) => setTerminalApp(e.target.value as typeof terminalApp)}
            className="rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1 text-xs text-zinc-200"
          >
            <option value="in-app">In-app terminal (xterm.js)</option>
            <option value="Terminal">Terminal.app</option>
            <option value="iTerm">iTerm2</option>
            <option value="Warp">Warp (limited — copies command to clipboard)</option>
            <option value="Ghostty">Ghostty (limited — copies command to clipboard)</option>
            <option value="custom">Custom command…</option>
          </select>
          <div className="mt-1 text-[10px] text-zinc-500">
            Where workflows launch. Terminal/iTerm support inline launch + window focus.
          </div>
        </Field>
        {terminalApp === 'custom' && (
          <Field label="Custom terminal command">
            <input
              type="text"
              value={customTerminalCommand}
              onChange={(e) => setCustomTerminalCommand(e.target.value)}
              placeholder="wezterm start --cwd %cwd% bash -c %cmd%"
              className="w-full rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1 font-mono text-[11px] text-zinc-200"
            />
            <div className="mt-1 text-[10px] text-zinc-500">
              Runs in /bin/sh. Substitutes <span className="font-mono">%cwd%</span> and <span className="font-mono">%cmd%</span> (the full shell line, already quoted).
            </div>
          </Field>
        )}
        <label className="mb-3 flex items-start gap-2">
          <input
            type="checkbox"
            checked={skipPermissions}
            onChange={(e) => setSkipPermissions(e.target.checked)}
            className="mt-0.5 h-3 w-3"
          />
          <div>
            <div className="text-xs text-zinc-300">Skip permission prompts</div>
            <div className="mt-0.5 text-[10px] text-zinc-500">
              Adds <span className="font-mono">--dangerously-skip-permissions</span> to launched <span className="font-mono">claude</span>. Speeds up workflows but disables every per-tool confirmation. Off by default.
            </div>
          </div>
        </label>
        <div className="mt-5 flex justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded-md border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:border-zinc-500"
          >
            Cancel
          </button>
          <button
            disabled={saving}
            onClick={save}
            className="rounded-md bg-emerald-600 px-3 py-1.5 text-xs text-white hover:bg-emerald-500 disabled:opacity-50"
          >
            {saving ? 'Saving…' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }): JSX.Element {
  return (
    <label className="mb-3 block">
      <div className="mb-1 text-xs text-zinc-400">{label}</div>
      {children}
    </label>
  )
}
