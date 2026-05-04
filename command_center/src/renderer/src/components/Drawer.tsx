import { useMemo, useState } from 'react'
import clsx from 'clsx'
import type { AppSettings, Session, SessionStatus, Workflow } from '@shared/types'
import { SessionCard } from './SessionCard'
import { WorkflowCard } from './WorkflowCard'

interface Props {
  sessions: Session[]
  workflows: Workflow[]
  activeUniverse?: string
  selectedId: string | null
  onSelect: (id: string) => void
  onDiscard: (id: string) => void
  onLaunchWorkflow?: (workflow: Workflow) => void
  onFocusTerminal?: (sessionId: string) => void
  onRenameSession?: (sessionId: string, title: string | null) => void
  onJumpToSession?: (sessionId: string) => void
  onSetAutoContinue?: (sessionId: string, auto: boolean) => void
  settings: AppSettings
}

type Tab = 'sessions' | 'workflows'

const STATUSES: SessionStatus[] = ['running', 'waiting', 'blocked', 'idle', 'done']

const STATUS_DOT: Record<SessionStatus, string> = {
  running: 'bg-emerald-500',
  waiting: 'bg-amber-400',
  blocked: 'bg-rose-500',
  done: 'bg-zinc-500',
  idle: 'bg-zinc-700'
}

export function Drawer({
  sessions,
  workflows,
  activeUniverse,
  selectedId,
  onSelect,
  onDiscard,
  onLaunchWorkflow,
  onFocusTerminal,
  onRenameSession,
  onJumpToSession,
  onSetAutoContinue,
  settings
}: Props): JSX.Element {
  const [tab, setTab] = useState<Tab>('sessions')
  const [enabled, setEnabled] = useState<Record<SessionStatus, boolean>>({
    running: true,
    waiting: true,
    blocked: true,
    idle: true,
    done: true
  })
  const [search, setSearch] = useState('')

  const visibleWorkflows = useMemo(() => {
    const scoped = activeUniverse
      ? workflows.filter((w) => w.universe === activeUniverse)
      : workflows
    const q = search.trim().toLowerCase()
    if (!q) return scoped
    return scoped.filter(
      (w) =>
        w.name.toLowerCase().includes(q) ||
        w.description?.toLowerCase().includes(q) ||
        w.agents.some((a) => a.toLowerCase().includes(q))
    )
  }, [workflows, activeUniverse, search])

  const counts = useMemo(() => {
    const c: Record<SessionStatus, number> = { running: 0, waiting: 0, blocked: 0, idle: 0, done: 0 }
    for (const s of sessions) c[s.status]++
    return c
  }, [sessions])

  const visible = sessions.filter((s) => enabled[s.status])

  const toggle = (st: SessionStatus): void => setEnabled((p) => ({ ...p, [st]: !p[st] }))

  return (
    <aside className="flex w-72 shrink-0 flex-col border-r border-zinc-800 bg-zinc-950">
      <div className="flex shrink-0 border-b border-zinc-800 text-xs">
        <TabButton active={tab === 'sessions'} onClick={() => setTab('sessions')}>
          Sessions
          <span className="ml-1.5 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-400">
            {visible.length}
            {visible.length !== sessions.length && (
              <span className="text-zinc-600">/{sessions.length}</span>
            )}
          </span>
        </TabButton>
        <TabButton active={tab === 'workflows'} onClick={() => setTab('workflows')}>
          Workflows
          <span className="ml-1.5 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-400">
            {visibleWorkflows.length}
          </span>
        </TabButton>
      </div>

      {tab === 'sessions' && (
        <div className="flex shrink-0 flex-wrap gap-1 border-b border-zinc-800 px-2 py-2">
          {STATUSES.map((st) => (
            <button
              key={st}
              onClick={() => toggle(st)}
              className={clsx(
                'flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] transition',
                enabled[st]
                  ? 'border-zinc-600 bg-zinc-800 text-zinc-100'
                  : 'border-zinc-800 bg-transparent text-zinc-500 hover:text-zinc-300'
              )}
            >
              <span className={clsx('h-1.5 w-1.5 rounded-full', STATUS_DOT[st])} />
              {st}
              <span className="text-zinc-500">{counts[st]}</span>
            </button>
          ))}
        </div>
      )}
      {tab === 'workflows' && (
        <div className="shrink-0 border-b border-zinc-800 px-2 py-2">
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Filter workflows…"
            className="w-full rounded-md border border-zinc-800 bg-zinc-900 px-2 py-1 text-xs text-zinc-200 placeholder-zinc-600 focus:border-zinc-600 focus:outline-none"
          />
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {tab === 'sessions' ? (
          visible.length === 0 ? (
            <Empty
              text={
                sessions.length === 0
                  ? 'No sessions yet. Run a Claude Code session to see it here.'
                  : 'No sessions match the current filter.'
              }
            />
          ) : (
            (() => {
              const pinned = visible.find((x) => x.id === selectedId)
              const rest = visible.filter((x) => x.id !== selectedId)
              return (
                <div className="space-y-2">
                  {pinned && (
                    <>
                      <SessionCard
                        key={pinned.id}
                        session={pinned}
                        selected
                        settings={settings}
                        onClick={() => onSelect(pinned.id)}
                        onDiscard={() => onDiscard(pinned.id)}
                        onFocusTerminal={() => onFocusTerminal?.(pinned.id)}
                        onRenameTitle={(t) => onRenameSession?.(pinned.id, t)}
                        onJumpToSuccessor={
                          pinned.successorId
                            ? () => onJumpToSession?.(pinned.successorId!)
                            : undefined
                        }
                        onSetAutoContinue={
                          onSetAutoContinue
                            ? (auto) => onSetAutoContinue(pinned.id, auto)
                            : undefined
                        }
                      />
                      {rest.length > 0 && (
                        <div className="my-2 flex items-center gap-2 text-[10px] uppercase tracking-wider text-zinc-600">
                          <span className="h-px flex-1 bg-zinc-800" />
                          <span>others</span>
                          <span className="h-px flex-1 bg-zinc-800" />
                        </div>
                      )}
                    </>
                  )}
                  {rest.map((s) => (
                    <SessionCard
                      key={s.id}
                      session={s}
                      selected={false}
                      settings={settings}
                      onClick={() => onSelect(s.id)}
                      onDiscard={() => onDiscard(s.id)}
                      onFocusTerminal={() => onFocusTerminal?.(s.id)}
                      onRenameTitle={(t) => onRenameSession?.(s.id, t)}
                      onJumpToSuccessor={
                        s.successorId ? () => onJumpToSession?.(s.successorId!) : undefined
                      }
                      onSetAutoContinue={
                        onSetAutoContinue ? (auto) => onSetAutoContinue(s.id, auto) : undefined
                      }
                    />
                  ))}
                </div>
              )
            })()
          )
        ) : visibleWorkflows.length === 0 ? (
          <Empty
            text={
              workflows.length === 0
                ? 'No workflows found. Add a universe via /rick add.'
                : search
                  ? 'No workflows match your filter.'
                  : `No workflows in ${activeUniverse ?? 'this universe'}.`
            }
          />
        ) : (
          <div className="space-y-2">
            {visibleWorkflows.map((w) => (
              <WorkflowCard
                key={`${w.universe}/${w.name}`}
                workflow={w}
                onLaunch={() => onLaunchWorkflow?.(w)}
              />
            ))}
          </div>
        )}
      </div>
    </aside>
  )
}

function TabButton({
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
        'flex-1 px-3 py-2 transition',
        active ? 'bg-zinc-900 text-zinc-100' : 'text-zinc-500 hover:text-zinc-300'
      )}
    >
      {children}
    </button>
  )
}

function Empty({ text }: { text: string }): JSX.Element {
  return <div className="px-3 py-6 text-center text-xs text-zinc-600">{text}</div>
}
