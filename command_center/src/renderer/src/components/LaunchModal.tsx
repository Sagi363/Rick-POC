import { useMemo, useState } from 'react'
import clsx from 'clsx'
import type { Workflow, WorkflowParam, WorktreeRequest } from '@shared/types'

const RECENT_CWD_KEY = 'rcc:launch:recent-cwd'
const AUTO_CONTINUE_KEY = 'rcc:launch:auto-continue'

type Mode = 'folder' | 'worktree'

interface Props {
  workflow: Workflow
  defaultCwd?: string
  recentCwds?: string[]
  branchPrefix?: string
  defaultBranchOff?: string
  onClose: () => void
  onLaunched: (sessionHint: string) => void
}

export function LaunchModal({
  workflow,
  defaultCwd,
  recentCwds = [],
  branchPrefix = 'feature/',
  defaultBranchOff = 'dev',
  onClose,
  onLaunched
}: Props): JSX.Element {
  const [values, setValues] = useState<Record<string, unknown>>(() => initialValues(workflow.params))
  const [extraPrompt, setExtraPrompt] = useState('')
  const [cwd, setCwd] = useState(() => loadRecentCwd(workflow.name) ?? defaultCwd ?? '')
  const [mode, setMode] = useState<Mode>('folder')
  const [worktreeName, setWorktreeName] = useState('')
  const [worktreeBranch, setWorktreeBranch] = useState('')
  const [worktreeFrom, setWorktreeFrom] = useState(defaultBranchOff)
  const [autoContinue, setAutoContinue] = useState<boolean>(() => loadAutoContinue())
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [recoverPath, setRecoverPath] = useState<string | null>(null)

  // Suggest worktree name + branch from the most identifying param.
  const suggestion = useMemo(() => {
    const pick = (k: string): string | undefined => {
      const v = values[k]
      return typeof v === 'string' && v ? v : undefined
    }
    const id = pick('feature') ?? pick('ticket_key') ?? pick('ticket') ?? pick('job')
    if (!id) return null
    const slug = id.toLowerCase().replace(/[^a-z0-9._/-]+/g, '-')
    return { name: slug, branch: `${branchPrefix}${slug}` }
  }, [values, branchPrefix])

  const missing = useMemo(
    () =>
      workflow.params
        .filter((p) => p.required && isEmpty(values[p.name]))
        .map((p) => p.name),
    [workflow.params, values]
  )

  const effectiveName = worktreeName || suggestion?.name || ''
  const effectiveBranch = worktreeBranch || suggestion?.branch || ''
  const worktreeReady = mode === 'folder' || (!!cwd && !!effectiveName && !!effectiveBranch)
  const canSubmit = !!cwd && missing.length === 0 && worktreeReady && !busy

  const set = (name: string, value: unknown): void => setValues((p) => ({ ...p, [name]: value }))

  const pickCwd = async (): Promise<void> => {
    const picked = await window.rcc.pickDirectory(cwd || undefined)
    if (picked) setCwd(picked)
  }

  const submit = async (): Promise<void> => {
    setBusy(true)
    setError(null)
    setRecoverPath(null)
    try {
      const worktree: WorktreeRequest | undefined =
        mode === 'worktree'
          ? {
              base: cwd,
              name: effectiveName,
              branch: effectiveBranch,
              fromBranch: worktreeFrom.trim() || undefined
            }
          : undefined
      const composedExtra = buildAutoContinueDirective(autoContinue, extraPrompt.trim())
      const result = await window.rcc.launchWorkflow({
        workflow: workflow.name,
        universe: workflow.universe,
        cwd,
        params: values,
        extraPrompt: composedExtra || undefined,
        worktree
      })
      if (!result.ok) {
        setError(result.error ?? 'Launch failed')
        if (result.existingWorktreePath) setRecoverPath(result.existingWorktreePath)
        return
      }
      saveRecentCwd(workflow.name, cwd)
      saveAutoContinue(autoContinue)
      onLaunched(result.command ?? '')
      onClose()
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }

  const useExistingWorktree = async (): Promise<void> => {
    if (!recoverPath) return
    setBusy(true)
    setError(null)
    try {
      const composedExtra = buildAutoContinueDirective(autoContinue, extraPrompt.trim())
      const result = await window.rcc.launchWorkflow({
        workflow: workflow.name,
        universe: workflow.universe,
        cwd: recoverPath,
        params: values,
        extraPrompt: composedExtra || undefined
      })
      if (!result.ok) {
        setError(result.error ?? 'Launch failed')
        return
      }
      saveRecentCwd(workflow.name, recoverPath)
      saveAutoContinue(autoContinue)
      onLaunched(result.command ?? '')
      onClose()
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="flex max-h-[85vh] w-[640px] flex-col rounded-lg border border-zinc-700 bg-zinc-900 shadow-xl">
        <header className="shrink-0 border-b border-zinc-800 px-5 py-3">
          <div className="flex items-baseline justify-between gap-3">
            <h2 className="text-sm font-medium text-zinc-100">
              Launch <span className="font-mono">{workflow.name}</span>
            </h2>
            <span className="text-[11px] text-zinc-500">{workflow.universe}</span>
          </div>
          {workflow.description && (
            <p className="mt-1 text-[11px] leading-snug text-zinc-400">{workflow.description}</p>
          )}
        </header>

        <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-5 text-xs">
          <div className="flex items-center rounded-md border border-zinc-800 bg-zinc-950 p-0.5 text-[11px]">
            <ModeButton active={mode === 'folder'} onClick={() => setMode('folder')}>
              Use folder
            </ModeButton>
            <ModeButton active={mode === 'worktree'} onClick={() => setMode('worktree')}>
              Create worktree
            </ModeButton>
          </div>

          <Field label={mode === 'worktree' ? 'Base project (the repo)' : 'Working directory'} required>
            {recentCwds.length > 0 && (
              <div className="mb-2 flex flex-wrap gap-1">
                {recentCwds.slice(0, 10).map((c) => {
                  const active = c === cwd
                  return (
                    <button
                      key={c}
                      onClick={() => setCwd(c)}
                      title={c}
                      className={clsx(
                        'rounded-full border px-2 py-0.5 font-mono text-[10px] transition',
                        active
                          ? 'border-emerald-500 bg-emerald-900/40 text-emerald-100'
                          : 'border-zinc-800 bg-zinc-950 text-zinc-300 hover:border-zinc-600 hover:text-zinc-100'
                      )}
                    >
                      {shortCwdLabel(c)}
                    </button>
                  )
                })}
              </div>
            )}
            <div className="flex gap-2">
              <input
                list="rcc-recent-cwds"
                value={cwd}
                onChange={(e) => setCwd(e.target.value)}
                placeholder="/Users/.../project"
                className="flex-1 rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 font-mono text-xs text-zinc-200 placeholder-zinc-600 focus:border-zinc-600 focus:outline-none"
              />
              <button
                onClick={pickCwd}
                className="rounded-md border border-zinc-700 px-2 py-1 text-xs text-zinc-300 hover:border-zinc-500"
              >
                Browse…
              </button>
              <datalist id="rcc-recent-cwds">
                {recentCwds.map((c) => (
                  <option key={c} value={c} />
                ))}
              </datalist>
            </div>
          </Field>

          {workflow.params.length === 0 ? (
            <div className="text-zinc-500">No parameters defined for this workflow.</div>
          ) : (
            workflow.params.map((p) => (
              <Field
                key={p.name}
                label={p.name}
                required={p.required}
                description={p.description}
                missing={missing.includes(p.name)}
              >
                <ParamInput
                  param={p}
                  value={values[p.name]}
                  onChange={(v) => set(p.name, v)}
                />
              </Field>
            ))
          )}

          {mode === 'worktree' && (
            <>
              <Field
                label="Worktree name"
                required
                description={
                  cwd && effectiveName
                    ? `${cwd}/.claude/worktrees/${effectiveName}`
                    : suggestion
                      ? `auto: ${suggestion.name}`
                      : 'subdir under .claude/worktrees/'
                }
              >
                <input
                  value={worktreeName}
                  onChange={(e) => setWorktreeName(e.target.value)}
                  placeholder={suggestion?.name ?? 'my-feature'}
                  className="w-full rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 font-mono text-xs text-zinc-200 placeholder-zinc-600 focus:border-zinc-600 focus:outline-none"
                />
              </Field>
              <Field
                label="Branch"
                required
                description={suggestion ? `auto: ${suggestion.branch}` : 'new branch name'}
              >
                <input
                  value={worktreeBranch}
                  onChange={(e) => setWorktreeBranch(e.target.value)}
                  placeholder={suggestion?.branch ?? `${branchPrefix}my-feature`}
                  className="w-full rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 font-mono text-xs text-zinc-200 placeholder-zinc-600 focus:border-zinc-600 focus:outline-none"
                />
              </Field>
              <Field
                label="Branch off"
                description={
                  defaultBranchOff
                    ? `Defaults to "${defaultBranchOff}" (configurable in Settings). Blank uses current HEAD.`
                    : 'Blank uses current HEAD.'
                }
              >
                <input
                  value={worktreeFrom}
                  onChange={(e) => setWorktreeFrom(e.target.value)}
                  placeholder={defaultBranchOff || 'main'}
                  className="w-full rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 font-mono text-xs text-zinc-200 placeholder-zinc-600 focus:border-zinc-600 focus:outline-none"
                />
              </Field>
            </>
          )}

          <Field
            label="Auto-continue between phases"
            description={
              autoContinue
                ? 'Rick will run all phases end-to-end without pausing for "next".'
                : 'Rick will pause after each phase as defined in the workflow YAML.'
            }
          >
            <label className="inline-flex cursor-pointer items-center gap-2">
              <input
                type="checkbox"
                checked={autoContinue}
                onChange={(e) => setAutoContinue(e.target.checked)}
                className="h-3 w-3"
              />
              <span
                className={clsx(
                  'rounded-full border px-2 py-0.5 font-mono text-[10px] transition',
                  autoContinue
                    ? 'border-emerald-600 bg-emerald-900/40 text-emerald-200'
                    : 'border-zinc-700 bg-zinc-900 text-zinc-400'
                )}
              >
                auto_continue: {autoContinue ? 'true' : 'false'}
              </span>
            </label>
          </Field>

          <Field label="Extra prompt (optional)" description="Appended after the /rick command line.">
            <textarea
              value={extraPrompt}
              onChange={(e) => setExtraPrompt(e.target.value)}
              rows={3}
              placeholder="Anything else you want to tell Claude before it starts…"
              className="w-full resize-y rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 text-xs text-zinc-200 placeholder-zinc-600 focus:border-zinc-600 focus:outline-none"
            />
          </Field>

          {error && (
            <div className="rounded-md border border-rose-700 bg-rose-900/30 px-3 py-2 text-rose-200">
              <div>{error}</div>
              {recoverPath && (
                <div className="mt-2 flex flex-wrap items-center gap-2">
                  <span className="text-[11px] text-rose-100">Worktree already exists.</span>
                  <button
                    onClick={useExistingWorktree}
                    disabled={busy}
                    className="rounded-md bg-emerald-600 px-2 py-1 text-[11px] text-white hover:bg-emerald-500 disabled:opacity-50"
                  >
                    {busy ? 'Launching…' : 'Use it & launch →'}
                  </button>
                  <span className="truncate font-mono text-[10px] text-rose-100/70" title={recoverPath}>
                    {recoverPath}
                  </span>
                </div>
              )}
            </div>
          )}
        </div>

        <footer className="flex shrink-0 items-center justify-between gap-3 border-t border-zinc-800 px-5 py-3">
          <span className="truncate text-[11px] text-zinc-500">
            {mode === 'worktree' ? (
              <>
                Will <span className="text-emerald-300">git worktree add</span>{' '}
                <span className="font-mono">{effectiveName || '…'}</span> on{' '}
                <span className="font-mono">{effectiveBranch || '…'}</span>, then open Terminal there.
              </>
            ) : (
              <>
                Will open Terminal.app in <span className="font-mono">{cwd || '…'}</span>
              </>
            )}
          </span>
          <div className="flex gap-2">
            <button
              onClick={onClose}
              className="rounded-md border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:border-zinc-500"
            >
              Cancel
            </button>
            <button
              disabled={!canSubmit}
              onClick={submit}
              className="rounded-md bg-emerald-600 px-3 py-1.5 text-xs text-white hover:bg-emerald-500 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {busy ? 'Launching…' : 'Launch in Terminal'}
            </button>
          </div>
        </footer>
      </div>
    </div>
  )
}

function ParamInput({
  param,
  value,
  onChange
}: {
  param: WorkflowParam
  value: unknown
  onChange: (v: unknown) => void
}): JSX.Element {
  if (param.type === 'bool') {
    return (
      <label className="inline-flex items-center gap-2">
        <input
          type="checkbox"
          checked={value === true}
          onChange={(e) => onChange(e.target.checked)}
          className="h-3 w-3"
        />
        <span className="text-zinc-400">{value ? 'true' : 'false'}</span>
      </label>
    )
  }
  if (param.enumValues && param.enumValues.length > 0) {
    return (
      <select
        value={(value as string) ?? ''}
        onChange={(e) => onChange(e.target.value || undefined)}
        className="w-full rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 text-xs text-zinc-200 focus:border-zinc-600 focus:outline-none"
      >
        <option value="">— choose —</option>
        {param.enumValues.map((v) => (
          <option key={v} value={v}>
            {v}
          </option>
        ))}
      </select>
    )
  }
  if (param.type === 'int') {
    return (
      <input
        type="number"
        value={(value as number | string | undefined) ?? ''}
        placeholder={param.default != null ? String(param.default) : ''}
        onChange={(e) => onChange(e.target.value === '' ? undefined : Number(e.target.value))}
        className="w-32 rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 text-xs text-zinc-200 focus:border-zinc-600 focus:outline-none"
      />
    )
  }
  return (
    <input
      type="text"
      value={(value as string) ?? ''}
      placeholder={param.default != null ? String(param.default) : ''}
      onChange={(e) => onChange(e.target.value)}
      className="w-full rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 text-xs text-zinc-200 focus:border-zinc-600 focus:outline-none"
    />
  )
}

function ModeButton({
  active,
  onClick,
  children
}: {
  active: boolean
  onClick: () => void
  children: React.ReactNode
}): JSX.Element {
  return (
    <button
      onClick={onClick}
      className={clsx(
        'flex-1 rounded px-2 py-1 transition',
        active ? 'bg-zinc-700 text-zinc-100' : 'text-zinc-500 hover:text-zinc-200'
      )}
    >
      {children}
    </button>
  )
}

function Field({
  label,
  required,
  description,
  missing,
  children
}: {
  label: string
  required?: boolean
  description?: string
  missing?: boolean
  children: React.ReactNode
}): JSX.Element {
  return (
    <label className="block">
      <div className="mb-1 flex items-baseline gap-2">
        <span className={clsx('text-[11px] font-medium', missing ? 'text-rose-400' : 'text-zinc-300')}>
          {label}
          {required && <span className="ml-1 text-rose-500">*</span>}
        </span>
        {description && (
          <span className="truncate text-[10px] text-zinc-500" title={description}>
            {description}
          </span>
        )}
      </div>
      {children}
    </label>
  )
}

function initialValues(params: WorkflowParam[]): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const p of params) {
    if (p.default !== undefined) out[p.name] = p.default
  }
  return out
}

function isEmpty(v: unknown): boolean {
  return v === undefined || v === null || v === ''
}

function shortCwdLabel(cwd: string): string {
  const home = '/Users/'
  if (cwd.startsWith(home)) {
    const parts = cwd.slice(home.length).split('/')
    if (parts.length <= 1) return '~'
    const tail = parts.slice(1).join('/')
    return tail.length > 38 ? '~/…/' + parts[parts.length - 1] : '~/' + tail
  }
  return cwd.length > 40 ? '…' + cwd.slice(-39) : cwd
}

function loadAutoContinue(): boolean {
  try {
    return window.localStorage.getItem(AUTO_CONTINUE_KEY) === '1'
  } catch {
    return false
  }
}

function saveAutoContinue(value: boolean): void {
  try {
    window.localStorage.setItem(AUTO_CONTINUE_KEY, value ? '1' : '0')
  } catch {
    // ignore
  }
}

function buildAutoContinueDirective(autoContinue: boolean, userExtra: string): string {
  if (!autoContinue) return userExtra
  const directive =
    'Override: run all phases with `auto_continue: true` — do not pause between phases or wait for me to say `next`. Drive the workflow end-to-end.'
  return userExtra ? `${directive}\n\n${userExtra}` : directive
}

function loadRecentCwd(workflow: string): string | undefined {
  try {
    const raw = window.localStorage.getItem(RECENT_CWD_KEY)
    if (!raw) return undefined
    const map = JSON.parse(raw) as Record<string, string>
    return map[workflow]
  } catch {
    return undefined
  }
}

function saveRecentCwd(workflow: string, cwd: string): void {
  try {
    const raw = window.localStorage.getItem(RECENT_CWD_KEY)
    const map: Record<string, string> = raw ? JSON.parse(raw) : {}
    map[workflow] = cwd
    window.localStorage.setItem(RECENT_CWD_KEY, JSON.stringify(map))
  } catch {
    // ignore
  }
}
