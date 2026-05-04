import { useEffect, useState } from 'react'
import clsx from 'clsx'
import type { Session, SessionSummary as Summary } from '@shared/types'

interface Props {
  session: Session
}

export function SessionSummary({ session }: Props): JSX.Element {
  const [data, setData] = useState<Summary | null>(null)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    setData(null)
    void window.rcc.getSessionSummary(session.id).then((s) => {
      if (!cancelled) {
        setData(s)
        setLoading(false)
      }
    })
    return () => {
      cancelled = true
    }
  }, [session.id, session.lastActivity])

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <div className="shrink-0 border-b border-zinc-800 px-3 py-2 text-xs uppercase tracking-wider text-zinc-500">
        Summary · {session.id.slice(0, 8)}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-3 text-sm text-zinc-300">
        {loading && <Spinner />}
        {!loading && !data && <Empty>Could not parse transcript.</Empty>}
        {data && <Body summary={data} session={session} />}
      </div>
    </section>
  )
}

function Body({ summary, session }: { summary: Summary; session: Session }): JSX.Element {
  // Recent activity intentionally lives in the bottom Activity panel
  // (which auto-hides when the in-app terminal is visible). Keeping it
  // here too caused two identical feeds on screen at once.
  return (
    <div className="space-y-5">
      <Meta summary={summary} session={session} />
      <ToolStats summary={summary} />
      <Subagents summary={summary} />
    </div>
  )
}

function Meta({ summary, session }: { summary: Summary; session: Session }): JSX.Element {
  return (
    <div className="rounded-md border border-zinc-800 bg-zinc-900/40 p-3">
      <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-xs">
        <Field label="workflow">
          {summary.workflow ?? <span className="text-zinc-500">unbound</span>}
        </Field>
        <Field label="status">{session.status}</Field>
        {session.phase && <Field label="phase">{session.phase}</Field>}
        {summary.model && <Field label="model">{summary.model}</Field>}
        <Field label="messages">
          {summary.totalUserMessages}u / {summary.totalAssistantMessages}a
        </Field>
        {summary.startedAt && (
          <Field label="started">{new Date(summary.startedAt).toLocaleString()}</Field>
        )}
        <Field label="last activity">
          {new Date(summary.lastActivity).toLocaleString()}
        </Field>
      </div>
      {summary.cwd && (
        <div className="mt-2 truncate font-mono text-[11px] text-zinc-500" title={summary.cwd}>
          {summary.cwd}
        </div>
      )}
    </div>
  )
}

function ToolStats({ summary }: { summary: Summary }): JSX.Element | null {
  const entries = Object.entries(summary.toolCounts).sort((a, b) => b[1] - a[1])
  if (!entries.length) return null
  return (
    <Section title="Tool usage">
      <div className="flex flex-wrap gap-2 text-[11px]">
        {entries.map(([name, count]) => (
          <span
            key={name}
            className="rounded-full border border-zinc-800 bg-zinc-900 px-2 py-0.5 font-mono text-zinc-300"
          >
            {name}
            <span className="ml-1 text-zinc-500">{count}</span>
          </span>
        ))}
      </div>
    </Section>
  )
}

function Subagents({ summary }: { summary: Summary }): JSX.Element | null {
  if (!summary.subagentSpawns.length) return null
  return (
    <Section title={`Subagents (${summary.subagentSpawns.length})`}>
      <ul className="space-y-1 text-xs">
        {summary.subagentSpawns.map((s, i) => (
          <li key={i} className="flex gap-2">
            <span className="shrink-0 font-mono text-zinc-500">
              {s.timestamp ? new Date(s.timestamp).toLocaleTimeString() : ''}
            </span>
            <span className="font-mono text-emerald-400">{s.name}</span>
            <span className="truncate text-zinc-300">{s.description}</span>
          </li>
        ))}
      </ul>
    </Section>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }): JSX.Element {
  return (
    <span>
      <span className="text-[10px] uppercase tracking-wider text-zinc-500">{label}</span>
      <span className="ml-1.5 text-zinc-200">{children}</span>
    </span>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }): JSX.Element {
  return (
    <div>
      <h3 className="mb-2 text-[10px] uppercase tracking-wider text-zinc-500">{title}</h3>
      {children}
    </div>
  )
}

function Empty({ children }: { children: React.ReactNode }): JSX.Element {
  return <div className="py-6 text-center text-xs text-zinc-600">{children}</div>
}

function Spinner(): JSX.Element {
  return <div className="py-6 text-center text-xs text-zinc-500">Parsing transcript…</div>
}
