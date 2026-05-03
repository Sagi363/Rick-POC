import { useEffect, useMemo, useState } from 'react'
import clsx from 'clsx'
import type { SessionFilesDTO, WorkflowStepView } from '@shared/types'

interface Props {
  sessionId: string | null
  sessionCwd: string | null
  sessionLastActivity: number | null
  selectedFile: string | null
  onSelect: (path: string | null) => void
}

export const SUMMARY_VIRTUAL_PATH = '__rcc:summary__'

type Scope = 'all' | 'session'

const SCOPE_KEY = 'rcc:files:scope'
const COLLAPSE_KEY = 'rcc:files:collapsed'

function loadCollapsed(): Record<string, boolean> {
  try {
    const raw = window.localStorage.getItem(COLLAPSE_KEY)
    return raw ? (JSON.parse(raw) as Record<string, boolean>) : {}
  } catch {
    return {}
  }
}

export function FileList({
  sessionId,
  sessionCwd,
  sessionLastActivity,
  selectedFile,
  onSelect
}: Props): JSX.Element {
  const [files, setFiles] = useState<SessionFilesDTO>({ specs: [], touched: [] })
  const [loading, setLoading] = useState(false)
  const [scope, setScope] = useState<Scope>(() => {
    const stored = typeof window !== 'undefined' ? window.localStorage.getItem(SCOPE_KEY) : null
    return stored === 'session' ? 'session' : 'all'
  })
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>(loadCollapsed)

  useEffect(() => {
    window.localStorage.setItem(SCOPE_KEY, scope)
  }, [scope])

  useEffect(() => {
    window.localStorage.setItem(COLLAPSE_KEY, JSON.stringify(collapsed))
  }, [collapsed])

  const toggleSection = (key: string): void =>
    setCollapsed((prev) => ({ ...prev, [key]: !prev[key] }))

  // Initial fetch on session switch — also force-selects the Summary tab so
  // the right pane lands on something useful when the user picks a session.
  useEffect(() => {
    if (!sessionId) {
      setFiles({ specs: [], touched: [] })
      return
    }
    let cancelled = false
    setLoading(true)
    void window.rcc.listSessionFiles(sessionId).then((f) => {
      if (cancelled) return
      setFiles(f)
      setLoading(false)
      onSelect(SUMMARY_VIRTUAL_PATH)
    })
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId])

  // Live refresh on transcript activity — refetches files (incl. workflowRun
  // step states) so phase-marker changes flow through without re-selecting the
  // session. Skips the initial mount because the effect above already fetched.
  useEffect(() => {
    if (!sessionId || !sessionLastActivity) return
    let cancelled = false
    void window.rcc.listSessionFiles(sessionId).then((f) => {
      if (!cancelled) setFiles(f)
    })
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionLastActivity])

  const sections = useMemo(() => {
    const out: { title: string; items: { label: string; path: string }[] }[] = []
    if (files.tracking) {
      out.push({
        title: 'Tracking',
        items: [{ label: 'tracking.md', path: files.tracking }]
      })
    }
    if (scope === 'all' && files.specs.length) {
      out.push({
        title: sessionCwd ? `Project specs · ${shortCwd(sessionCwd)}` : 'Project specs',
        items: files.specs.map((p) => ({ label: relativeTo(p, sessionCwd), path: p }))
      })
    }
    if (files.touched.length) {
      const touchedItems = files.touched.map((p) => ({ label: relativeTo(p, sessionCwd), path: p }))
      out.push({
        title: 'Touched in session',
        items: touchedItems
      })
    }
    if (files.transcript) {
      out.push({
        title: 'Raw',
        items: [{ label: 'transcript.jsonl', path: files.transcript }]
      })
    }
    return out
  }, [files, sessionCwd, scope])

  return (
    <div className="flex h-full min-h-0 flex-col bg-zinc-950">
      <div className="flex shrink-0 items-center justify-between gap-2 border-b border-zinc-800 px-3 py-2">
        <span className="text-xs uppercase tracking-wider text-zinc-500">Session</span>
        <div className="flex items-center rounded-md border border-zinc-800 bg-zinc-900 p-0.5 text-[10px]">
          <ScopeButton active={scope === 'all'} onClick={() => setScope('all')}>
            Whole project
          </ScopeButton>
          <ScopeButton active={scope === 'session'} onClick={() => setScope('session')}>
            Session only
          </ScopeButton>
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {!sessionId ? (
          <Hint>Pick a session.</Hint>
        ) : (
          <ul className="space-y-1">
            <Item
              label="📊 Summary"
              hint="prompts · tools · subagents"
              selected={selectedFile === SUMMARY_VIRTUAL_PATH}
              onClick={() => onSelect(SUMMARY_VIRTUAL_PATH)}
            />
            {loading && <Hint>Loading files…</Hint>}
            {files.workflowRun && (
              <WorkflowSection
                run={files.workflowRun}
                cwd={sessionCwd}
                selectedFile={selectedFile}
                onSelect={onSelect}
                collapsed={collapsed}
                onToggle={toggleSection}
              />
            )}
            {sections.map((sec) => {
              const key = `sec:${sec.title}`
              const isCollapsed = collapsed[key]
              return (
                <li key={sec.title} className="pt-3">
                  <SectionHeader
                    label={sec.title}
                    count={sec.items.length}
                    collapsed={!!isCollapsed}
                    onClick={() => toggleSection(key)}
                  />
                  {!isCollapsed && (
                    <ul className="mt-1 space-y-0.5">
                      {sec.items.map(({ label, path }) => (
                        <Item
                          key={path}
                          label={label}
                          hint={iconForFile(path)}
                          selected={path === selectedFile}
                          onClick={() => onSelect(path)}
                          title={path}
                        />
                      ))}
                    </ul>
                  )}
                </li>
              )
            })}
            {!loading && sections.length === 0 && !files.workflowRun && (
              <Hint>No spec or tracking files found for this cwd.</Hint>
            )}
          </ul>
        )}
      </div>
    </div>
  )
}

function StepBadge({ status }: { status: WorkflowStepView['status'] }): JSX.Element {
  if (status === 'done') {
    return (
      <span className="inline-flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded-full border border-emerald-500 bg-emerald-500/20 text-[8px] font-bold leading-none text-emerald-300">
        ✓
      </span>
    )
  }
  if (status === 'running') {
    return (
      <span className="inline-flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded-full border border-amber-400 bg-amber-500/30 text-amber-300">
        <span className="block h-1.5 w-1.5 animate-pulse rounded-full bg-amber-300" />
      </span>
    )
  }
  return (
    <span className="inline-flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded-full border border-zinc-700" />
  )
}

const STEP_TEXT_TINT: Record<WorkflowStepView['status'], string> = {
  done: 'text-zinc-300',
  running: 'text-amber-200',
  pending: 'text-zinc-600'
}

function WorkflowSection({
  run,
  cwd,
  selectedFile,
  onSelect,
  collapsed,
  onToggle
}: {
  run: NonNullable<SessionFilesDTO['workflowRun']>
  cwd: string | null
  selectedFile: string | null
  onSelect: (path: string | null) => void
  collapsed: Record<string, boolean>
  onToggle: (key: string) => void
}): JSX.Element {
  const sectionKey = `wf:${run.workflow}`
  const sectionCollapsed = !!collapsed[sectionKey]
  return (
    <li className="pt-3">
      <SectionHeader
        label={`Workflow · ${run.label}`}
        count={run.steps.length}
        collapsed={sectionCollapsed}
        onClick={() => onToggle(sectionKey)}
      />
      {!sectionCollapsed && (
        <ol className="mt-1 space-y-2">
          {run.steps.map((step, i) => {
            const stepKey = `wf:${run.workflow}:${step.id}`
            const stepCollapsed = !!collapsed[stepKey]
            const hasFiles = step.files.length > 0
            const textTint = STEP_TEXT_TINT[step.status]
            return (
              <li key={step.id} className="px-2">
                <div
                  className={clsx(
                    'flex items-center gap-1.5 text-[11px]',
                    textTint,
                    hasFiles && 'cursor-pointer hover:text-zinc-100'
                  )}
                  title={step.description}
                  onClick={() => hasFiles && onToggle(stepKey)}
                >
                  {hasFiles ? (
                    <span className="w-3 shrink-0 text-[10px] text-zinc-500">
                      {stepCollapsed ? '▸' : '▾'}
                    </span>
                  ) : (
                    <span className="w-3 shrink-0" />
                  )}
                  <span className="font-mono text-zinc-600">{i + 1}.</span>
                  <StepBadge status={step.status} />
                  <span className={clsx('truncate font-medium', step.status === 'running' && 'text-amber-100')}>
                    {step.id}
                  </span>
                  {step.status === 'running' && step.currentSubagent ? (
                    <span className="ml-1 min-w-0 flex-1 truncate font-mono text-[10px]">
                      <span className="text-amber-300">{step.currentSubagent}</span>
                      {step.currentActivity && (
                        <span className="text-zinc-400"> · {step.currentActivity}</span>
                      )}
                    </span>
                  ) : (
                    <span className="ml-1 truncate font-mono text-[10px] text-zinc-500">
                      {step.agent}
                      {step.collaborators.length > 0 && ` + ${step.collaborators.join(' + ')}`}
                    </span>
                  )}
                  {hasFiles && (
                    <span className="ml-auto shrink-0 text-[10px] text-zinc-500">
                      {step.files.length}
                    </span>
                  )}
                </div>
                {hasFiles && !stepCollapsed && (
                  <ul className="mt-0.5 space-y-0.5 pl-8">
                    {step.files.map((f) => (
                      <Item
                        key={f}
                        label={relativeTo(f, cwd)}
                        hint={iconForFile(f)}
                        selected={f === selectedFile}
                        onClick={() => onSelect(f)}
                        title={f}
                      />
                    ))}
                  </ul>
                )}
              </li>
            )
          })}
        </ol>
      )}
    </li>
  )
}

function SectionHeader({
  label,
  count,
  collapsed,
  onClick
}: {
  label: string
  count?: number
  collapsed: boolean
  onClick: () => void
}): JSX.Element {
  return (
    <button
      onClick={onClick}
      className="flex w-full items-center gap-1 px-2 text-left text-[10px] uppercase tracking-wider text-zinc-500 hover:text-zinc-300"
    >
      <span className="w-3">{collapsed ? '▸' : '▾'}</span>
      <span className="truncate">{label}</span>
      {typeof count === 'number' && (
        <span className="ml-auto rounded bg-zinc-900 px-1.5 py-px text-[9px] text-zinc-500">
          {count}
        </span>
      )}
    </button>
  )
}

function ScopeButton({
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
        'rounded px-1.5 py-0.5 transition',
        active ? 'bg-zinc-700 text-zinc-100' : 'text-zinc-500 hover:text-zinc-200'
      )}
    >
      {children}
    </button>
  )
}

function Item({
  label,
  hint,
  selected,
  onClick,
  title
}: {
  label: string
  hint?: string
  selected: boolean
  onClick: () => void
  title?: string
}): JSX.Element {
  return (
    <li>
      <button
        onClick={onClick}
        className={clsx(
          'flex w-full items-center justify-between gap-2 rounded px-2 py-1 text-left text-xs',
          selected
            ? 'bg-zinc-800 text-zinc-100'
            : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'
        )}
        title={title}
      >
        <span className="truncate">{label}</span>
        {hint && <span className="shrink-0 text-[10px] text-zinc-600">{hint}</span>}
      </button>
    </li>
  )
}

function Hint({ children }: { children: React.ReactNode }): JSX.Element {
  return <div className="px-2 py-6 text-center text-xs text-zinc-600">{children}</div>
}

function iconForFile(p: string): string {
  if (p.endsWith('.md') || p.endsWith('.markdown')) return 'md'
  if (p.endsWith('.jsonl')) return 'raw'
  if (p.endsWith('.json')) return 'json'
  if (p.endsWith('.yaml') || p.endsWith('.yml')) return 'yaml'
  return ''
}

function relativeTo(absolute: string, cwd: string | null): string {
  if (!cwd) return absolute.split('/').pop() ?? absolute
  if (absolute.startsWith(cwd + '/')) return absolute.slice(cwd.length + 1)
  return absolute.split('/').pop() ?? absolute
}

function shortCwd(cwd: string): string {
  const home = '/Users/'
  return cwd.startsWith(home) ? '~/' + cwd.slice(home.length).split('/').slice(1).join('/') : cwd
}
