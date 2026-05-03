import { useEffect, useRef, useState } from 'react'
import clsx from 'clsx'
import type { AppSettings, Session, SessionStatus } from '@shared/types'

interface Props {
  session: Session
  selected: boolean
  settings: AppSettings
  onClick: () => void
  onDiscard?: () => void
  onFocusTerminal?: () => void
  onRenameTitle?: (title: string | null) => void
  onJumpToSuccessor?: () => void
  /** Mid-run override: send a /btw directive flipping auto_continue on/off. */
  onSetAutoContinue?: (auto: boolean) => void
}

const AUTO_CONTINUE_SESSION_KEY = 'rcc:session:auto-continue'

function loadSessionAutoContinue(sessionId: string): boolean {
  try {
    const raw = window.localStorage.getItem(AUTO_CONTINUE_SESSION_KEY)
    if (!raw) return false
    const map = JSON.parse(raw) as Record<string, boolean>
    return map[sessionId] === true
  } catch {
    return false
  }
}

function saveSessionAutoContinue(sessionId: string, value: boolean): void {
  try {
    const raw = window.localStorage.getItem(AUTO_CONTINUE_SESSION_KEY)
    const map: Record<string, boolean> = raw ? JSON.parse(raw) : {}
    map[sessionId] = value
    window.localStorage.setItem(AUTO_CONTINUE_SESSION_KEY, JSON.stringify(map))
  } catch {
    // ignore
  }
}

const STATUS_COLOR: Record<SessionStatus, string> = {
  running: 'bg-emerald-500',
  waiting: 'bg-amber-400',
  blocked: 'bg-rose-500',
  done: 'bg-zinc-500',
  idle: 'bg-zinc-700'
}

const STATUS_LABEL: Record<SessionStatus, string> = {
  running: 'running',
  waiting: 'waiting',
  blocked: 'blocked',
  done: 'done',
  idle: 'idle'
}

export function SessionCard({
  session,
  selected,
  settings,
  onClick,
  onDiscard,
  onFocusTerminal,
  onRenameTitle,
  onJumpToSuccessor,
  onSetAutoContinue
}: Props): JSX.Element {
  const [editingTitle, setEditingTitle] = useState(false)
  const [draftTitle, setDraftTitle] = useState(session.title ?? '')
  const [autoContinue, setAutoContinue] = useState<boolean>(() =>
    loadSessionAutoContinue(session.id)
  )
  const inputRef = useRef<HTMLInputElement | null>(null)

  useEffect(() => {
    setAutoContinue(loadSessionAutoContinue(session.id))
  }, [session.id])

  const flipAutoContinue = (e: React.MouseEvent): void => {
    e.stopPropagation()
    if (!onSetAutoContinue) return
    const next = !autoContinue
    setAutoContinue(next)
    saveSessionAutoContinue(session.id, next)
    onSetAutoContinue(next)
  }

  useEffect(() => {
    if (!editingTitle) setDraftTitle(session.title ?? '')
  }, [session.title, editingTitle])

  useEffect(() => {
    if (editingTitle) inputRef.current?.select()
  }, [editingTitle])

  const startEdit = (): void => {
    setDraftTitle(session.title ?? '')
    setEditingTitle(true)
  }

  const commitEdit = (): void => {
    const trimmed = draftTitle.trim()
    if (trimmed && trimmed !== session.title) {
      onRenameTitle?.(trimmed)
    } else if (!trimmed && session.customTitle) {
      onRenameTitle?.(null)
    }
    setEditingTitle(false)
  }

  const cancelEdit = (): void => {
    setDraftTitle(session.title ?? '')
    setEditingTitle(false)
  }

  const displayTitle = session.title ?? `session ${session.id.slice(0, 8)}`
  const ctx = session.context
  const pct = ctx ? ctx.used / ctx.limit : 0
  const tone = ctx
    ? pct >= settings.criticalThreshold
      ? 'bg-rose-500'
      : pct >= settings.warnThreshold
        ? 'bg-amber-400'
        : 'bg-emerald-500'
    : 'bg-zinc-700'

  return (
    <div
      onClick={onClick}
      className={clsx(
        'group relative block w-full cursor-pointer rounded-md border px-3 py-2 text-left transition',
        selected
          ? 'border-zinc-500 bg-zinc-800'
          : 'border-zinc-800 bg-zinc-900 hover:border-zinc-700 hover:bg-zinc-900/70'
      )}
    >
      <div className="absolute right-2 top-2 hidden gap-1 group-hover:flex">
        {onFocusTerminal && session.cwd && (
          <button
            onClick={(e) => {
              e.stopPropagation()
              onFocusTerminal()
            }}
            title="Bring this session's terminal window to the front"
            className="flex h-5 w-5 items-center justify-center rounded text-zinc-500 hover:bg-zinc-700 hover:text-zinc-100"
          >
            ⤴
          </button>
        )}
        {onDiscard && (
          <button
            onClick={(e) => {
              e.stopPropagation()
              if (confirm(`Discard session "${session.workflow ?? session.id.slice(0, 8)}"?`)) {
                onDiscard()
              }
            }}
            title="Discard from view (deletes tracking file)"
            className="flex h-5 w-5 items-center justify-center rounded text-zinc-500 hover:bg-zinc-700 hover:text-zinc-100"
          >
            ×
          </button>
        )}
      </div>
      <div className="flex items-baseline justify-between gap-2 pr-12">
        {editingTitle ? (
          <input
            ref={inputRef}
            value={draftTitle}
            onClick={(e) => e.stopPropagation()}
            onChange={(e) => setDraftTitle(e.target.value)}
            onBlur={commitEdit}
            onKeyDown={(e) => {
              if (e.key === 'Enter') commitEdit()
              else if (e.key === 'Escape') cancelEdit()
            }}
            placeholder={
              session.customTitle ? '(blank clears the custom title)' : 'Title for this session…'
            }
            className="min-w-0 flex-1 rounded border border-zinc-600 bg-zinc-950 px-1.5 py-0.5 text-sm text-zinc-100 focus:border-emerald-500 focus:outline-none"
          />
        ) : (
          <button
            onClick={(e) => {
              e.stopPropagation()
              if (onRenameTitle) startEdit()
            }}
            title={onRenameTitle ? 'Click to rename' : displayTitle}
            className={clsx(
              'min-w-0 flex-1 truncate text-left text-sm font-medium',
              session.title ? 'text-zinc-100' : 'text-zinc-500',
              onRenameTitle && 'hover:text-emerald-200'
            )}
          >
            {displayTitle}
            {session.customTitle && (
              <span className="ml-1 text-[10px] text-zinc-600" title="Custom title">
                ✎
              </span>
            )}
          </button>
        )}
        <span className="shrink-0 text-[11px] text-zinc-500">
          {relativeTime(session.lastActivity)}
        </span>
      </div>
      {session.workflow && (
        <div className="truncate pr-12 text-[11px] text-zinc-500">{session.workflow}</div>
      )}
      {session.successorId && onJumpToSuccessor && (
        <button
          onClick={(e) => {
            e.stopPropagation()
            onJumpToSuccessor()
          }}
          className="mt-1 flex w-full items-center gap-1 rounded border border-amber-700/50 bg-amber-900/30 px-2 py-0.5 text-left text-[10px] text-amber-200 hover:bg-amber-900/60"
          title="Continued in a newer session — click to switch"
        >
          <span>→</span>
          <span className="truncate">continued at {session.successorId.slice(0, 8)}</span>
        </button>
      )}
      <div className="mt-1 flex items-center gap-2 text-[11px] text-zinc-400">
        <span className={clsx('h-1.5 w-1.5 rounded-full', STATUS_COLOR[session.status])} />
        <span>{STATUS_LABEL[session.status]}</span>
        {session.phase && (
          <>
            <span className="text-zinc-700">·</span>
            <span className="truncate">{session.phase}</span>
          </>
        )}
        {session.total != null && (
          <>
            <span className="text-zinc-700">·</span>
            <span>{session.completed ?? 0}/{session.total}</span>
          </>
        )}
        {onSetAutoContinue && (
          <button
            onClick={flipAutoContinue}
            title={
              autoContinue
                ? 'auto_continue is ON — click to flip OFF (Rick will pause between phases)'
                : 'auto_continue is OFF — click to flip ON (Rick drives end-to-end)'
            }
            className={clsx(
              'ml-auto rounded-full border px-1.5 py-0 font-mono text-[9px] transition',
              autoContinue
                ? 'border-emerald-700 bg-emerald-900/40 text-emerald-200 hover:border-emerald-500'
                : 'border-zinc-700 bg-zinc-900 text-zinc-500 hover:border-zinc-500 hover:text-zinc-300'
            )}
          >
            auto {autoContinue ? 'on' : 'off'}
          </button>
        )}
      </div>
      {ctx && (
        <div className="mt-2">
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
            <div
              className={clsx('h-full', tone)}
              style={{ width: `${Math.min(100, pct * 100).toFixed(1)}%` }}
            />
          </div>
          <div className="mt-1 flex items-center justify-between text-[10px] text-zinc-500">
            <span>
              {formatTokens(ctx.used)} / {formatTokens(ctx.limit)}
              {!ctx.modelKnown && <span title="Unknown model — assumed 200k"> ?</span>}
            </span>
            <span>{Math.round(pct * 100)}%</span>
          </div>
        </div>
      )}
    </div>
  )
}

function relativeTime(ms: number): string {
  const diff = Date.now() - ms
  if (diff < 5_000) return 'just now'
  if (diff < 60_000) return `${Math.floor(diff / 1000)}s ago`
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`
  return `${Math.floor(diff / 86_400_000)}d ago`
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}k`
  return String(n)
}
