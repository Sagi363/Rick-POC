import { useEffect, useState } from 'react'
import clsx from 'clsx'
import type { Session, SessionSummary } from '@shared/types'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

interface Props {
  session: Session | null
  collapsed: boolean
  onToggle: () => void
}

export function ProgressPanel({ session, collapsed, onToggle }: Props): JSX.Element {
  const [body, setBody] = useState<string>('')
  const [summary, setSummary] = useState<SessionSummary | null>(null)
  const [loadingSummary, setLoadingSummary] = useState(false)

  const hasTrackedProgress =
    !!session && (session.total != null || !!session.phase || !!session.current)

  useEffect(() => {
    if (!session || collapsed) {
      setBody('')
      setSummary(null)
      return
    }
    let cancelled = false
    void window.rcc.readTracking(session.id).then((raw) => {
      if (cancelled) return
      const stripped = (raw ?? '').replace(/^---[\s\S]*?---\s*/, '').trim()
      setBody(stripped)
    })

    if (!hasTrackedProgress) {
      setLoadingSummary(true)
      void window.rcc.getSessionSummary(session.id).then((s) => {
        if (cancelled) return
        setSummary(s)
        setLoadingSummary(false)
      })
    } else {
      setSummary(null)
    }

    return () => {
      cancelled = true
    }
  }, [session?.id, session?.lastActivity, collapsed, hasTrackedProgress])

  if (collapsed) {
    return (
      <div className="flex shrink-0 items-center justify-between border-t border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[11px] text-zinc-500">
        <span>Activity hidden</span>
        <button
          onClick={onToggle}
          className="rounded px-2 py-0.5 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
        >
          ▲ Show
        </button>
      </div>
    )
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <Header
        session={session}
        hasTrackedProgress={hasTrackedProgress}
        onToggle={onToggle}
      />
      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-2 text-xs">
        {!session ? (
          <Hint>Pick a session.</Hint>
        ) : hasTrackedProgress ? (
          body ? (
            <article className="markdown">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown>
            </article>
          ) : (
            <Hint>Tracking exists but body is empty.</Hint>
          )
        ) : loadingSummary ? (
          <Hint>Reading transcript…</Hint>
        ) : summary && summary.recent.length > 0 ? (
          <ActivityFeed summary={summary} />
        ) : (
          <Hint>No activity yet.</Hint>
        )}
      </div>
    </section>
  )
}

function Header({
  session,
  hasTrackedProgress,
  onToggle
}: {
  session: Session | null
  hasTrackedProgress: boolean
  onToggle: () => void
}): JSX.Element {
  const total = session?.total ?? 0
  const completed = session?.completed ?? 0
  const pct = total > 0 ? completed / total : 0

  return (
    <div className="shrink-0 border-y border-zinc-800 px-3 py-2">
      <div className="flex items-baseline justify-between gap-2">
        <span className="text-xs uppercase tracking-wider text-zinc-500">
          {hasTrackedProgress ? 'Progress' : 'Activity'}
        </span>
        <div className="flex items-center gap-2 text-[11px] text-zinc-400">
          {session?.phase && <span>phase: {session.phase}</span>}
          <button
            onClick={onToggle}
            className="rounded px-1.5 py-0.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-100"
            title="Hide"
          >
            ▼ Hide
          </button>
        </div>
      </div>
      {hasTrackedProgress && total > 0 && (
        <div className="mt-2">
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
            <div className="h-full bg-emerald-500" style={{ width: `${(pct * 100).toFixed(1)}%` }} />
          </div>
          <div className="mt-1 flex justify-between text-[11px] text-zinc-500">
            <span>
              {completed} / {total}
            </span>
            <span>{Math.round(pct * 100)}%</span>
          </div>
        </div>
      )}
      {hasTrackedProgress && session?.current && (
        <div className="mt-2 truncate text-xs text-zinc-300">→ {session.current}</div>
      )}
    </div>
  )
}

function ActivityFeed({ summary }: { summary: SessionSummary }): JSX.Element {
  const events = flattenActivity(summary)
  return (
    <ol className="space-y-1.5">
      {events.map((e, i) => (
        <li key={i} className="flex gap-2">
          <span className="w-14 shrink-0 font-mono text-[10px] text-zinc-600">
            {e.timestamp ? new Date(e.timestamp).toLocaleTimeString() : ''}
          </span>
          <span
            className={clsx(
              'shrink-0 rounded px-1.5 py-px text-[10px] uppercase tracking-wider',
              kindStyle(e.kind)
            )}
          >
            {e.kind}
          </span>
          <span className="min-w-0 flex-1 truncate text-zinc-300" title={e.text}>
            {e.text}
          </span>
        </li>
      ))}
    </ol>
  )
}

function flattenActivity(
  s: SessionSummary
): { kind: 'user' | 'reply' | 'tool' | 'agent'; text: string; timestamp: number }[] {
  const events: { kind: 'user' | 'reply' | 'tool' | 'agent'; text: string; timestamp: number }[] =
    []
  for (const m of s.recent) {
    if (m.type === 'user' && m.text) {
      events.push({ kind: 'user', text: oneLine(m.text), timestamp: m.timestamp })
    } else if (m.type === 'assistant') {
      if (m.text) {
        events.push({ kind: 'reply', text: oneLine(m.text), timestamp: m.timestamp })
      }
      for (const t of m.toolUses) {
        const isAgent = t.name === 'Task'
        events.push({
          kind: isAgent ? 'agent' : 'tool',
          text: `${t.name}${t.brief ? ' · ' + t.brief : ''}`,
          timestamp: m.timestamp
        })
      }
    }
  }
  return events.slice(-12)
}

function kindStyle(kind: 'user' | 'reply' | 'tool' | 'agent'): string {
  switch (kind) {
    case 'user':
      return 'bg-blue-900/60 text-blue-200'
    case 'reply':
      return 'bg-emerald-900/60 text-emerald-200'
    case 'tool':
      return 'bg-amber-900/60 text-amber-200'
    case 'agent':
      return 'bg-purple-900/60 text-purple-200'
  }
}

function oneLine(text: string): string {
  return text.replace(/\s+/g, ' ').trim().slice(0, 240)
}

function Hint({ children }: { children: React.ReactNode }): JSX.Element {
  return <div className="py-4 text-center text-zinc-600">{children}</div>
}
