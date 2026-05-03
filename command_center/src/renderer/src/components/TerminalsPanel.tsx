import { useEffect, useState } from 'react'
import clsx from 'clsx'
import type { PtyInfo, SessionStatus } from '@shared/types'
import { Terminal } from './Terminal'

interface Props {
  ptys: PtyInfo[]
  selectedSessionId: string | null
  /** Status of the selected session — gates the /rick next button so it only
   *  fires when Rick is actually paused awaiting user input. */
  sessionStatus?: SessionStatus
  collapsed: boolean
  onToggle: () => void
}

export function TerminalsPanel({
  ptys,
  selectedSessionId,
  sessionStatus,
  collapsed,
  onToggle
}: Props): JSX.Element {
  const [activeId, setActiveId] = useState<string | null>(null)

  useEffect(() => {
    if (ptys.length === 0) {
      setActiveId(null)
      return
    }
    if (!activeId || !ptys.some((p) => p.id === activeId)) {
      // Prefer a pty bound to the currently selected session.
      const preferred = ptys.find((p) => p.sessionId === selectedSessionId) ?? ptys[ptys.length - 1]
      setActiveId(preferred.id)
    }
  }, [ptys, selectedSessionId, activeId])

  useEffect(() => {
    if (selectedSessionId) {
      const bound = ptys.find((p) => p.sessionId === selectedSessionId)
      if (bound && bound.id !== activeId) setActiveId(bound.id)
    }
  }, [selectedSessionId, ptys, activeId])

  if (collapsed) {
    return (
      <div className="flex shrink-0 items-center justify-between border-t border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[11px] text-zinc-500">
        <span>
          Terminal hidden{ptys.length > 0 && ` · ${ptys.length} active`}
        </span>
        <button
          onClick={onToggle}
          className="rounded px-2 py-0.5 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
        >
          ▲ Show
        </button>
      </div>
    )
  }

  const activePty = ptys.find((p) => p.id === activeId)
  const canSendCommand = !!activePty?.alive
  // /rick next only makes sense when Rick has stopped and is awaiting user
  // input — between phases or after a Notification. Firing it mid-execution
  // would stack a prompt that disrupts the current turn.
  const rickIsAwaitingUser = sessionStatus === 'waiting' || sessionStatus === 'idle'
  const canSendNext = canSendCommand && rickIsAwaitingUser
  const [confirmingClear, setConfirmingClear] = useState(false)

  const sendRickCommand = (cmd: string): void => {
    if (!activeId || !canSendCommand) return
    void window.rcc.ptyWrite(activeId, `${cmd}\r`)
  }

  const handleClear = (): void => {
    if (!canSendCommand) return
    setConfirmingClear(true)
  }

  const confirmClear = (): void => {
    setConfirmingClear(false)
    sendRickCommand('/clear')
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col bg-zinc-950">
      <header className="flex shrink-0 items-center gap-1 border-y border-zinc-800 px-2 py-1">
        <span className="px-1.5 text-[10px] uppercase tracking-wider text-zinc-500">Terminal</span>
        <div className="flex flex-1 items-center gap-1 overflow-x-auto">
          {ptys.map((p) => (
            <Tab
              key={p.id}
              info={p}
              active={p.id === activeId}
              onSelect={() => setActiveId(p.id)}
              onClose={() => void window.rcc.ptyKill(p.id)}
            />
          ))}
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <QuickCmd
            label="/rick next"
            tone="emerald"
            disabled={!canSendNext}
            disabledReason={
              !canSendCommand
                ? 'No active terminal — open the in-app terminal for this session first'
                : sessionStatus === 'running'
                  ? 'Rick is running — wait until the current step finishes'
                  : sessionStatus === 'done'
                    ? 'Workflow is already complete'
                    : sessionStatus === 'blocked'
                      ? 'Session is blocked — resolve the blocker first'
                      : undefined
            }
            onClick={() => sendRickCommand('/rick next')}
          />
          <QuickCmd
            label="/rick status"
            tone="zinc"
            disabled={!canSendCommand}
            onClick={() => sendRickCommand('/btw rick status')}
          />
          <QuickCmd
            label="/clear"
            tone="amber"
            disabled={!canSendCommand}
            onClick={handleClear}
          />
        </div>
        <button
          onClick={onToggle}
          className="ml-1 rounded px-1.5 py-0.5 text-[11px] text-zinc-500 hover:bg-zinc-800 hover:text-zinc-100"
          title="Hide"
        >
          ▼ Hide
        </button>
      </header>
      <div className="min-h-0 flex-1 px-2 py-1">
        {ptys.map((p) => (
          <div
            key={p.id}
            className={clsx('h-full w-full', p.id === activeId ? 'block' : 'hidden')}
          >
            <Terminal ptyId={p.id} active={p.id === activeId} />
          </div>
        ))}
      </div>
      {confirmingClear && (
        <ConfirmClearModal
          onConfirm={confirmClear}
          onCancel={() => setConfirmingClear(false)}
        />
      )}
    </section>
  )
}

type QuickCmdTone = 'emerald' | 'zinc' | 'amber'

const TONE: Record<QuickCmdTone, string> = {
  emerald: 'border-emerald-700 bg-emerald-900/30 text-emerald-200 hover:border-emerald-500 hover:bg-emerald-800/40',
  zinc: 'border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500 hover:bg-zinc-800',
  amber: 'border-amber-700 bg-amber-900/30 text-amber-200 hover:border-amber-500 hover:bg-amber-800/40'
}

function QuickCmd({
  label,
  disabled,
  disabledReason,
  tone,
  onClick
}: {
  label: string
  disabled: boolean
  disabledReason?: string
  tone: QuickCmdTone
  onClick: () => void
}): JSX.Element {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={
        disabled
          ? (disabledReason ?? 'No active terminal — open the in-app terminal for this session first')
          : `Send "${label}\\n" to the active terminal`
      }
      className={clsx(
        'shrink-0 rounded-md border px-2 py-0.5 font-mono text-[11px] transition',
        disabled ? 'cursor-not-allowed border-zinc-800 text-zinc-600' : TONE[tone]
      )}
    >
      ▶ {label}
    </button>
  )
}

function ConfirmClearModal({
  onConfirm,
  onCancel
}: {
  onConfirm: () => void
  onCancel: () => void
}): JSX.Element {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={onCancel}
    >
      <div
        className="w-[440px] rounded-lg border border-amber-700 bg-zinc-900 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="border-b border-zinc-800 px-5 py-3">
          <h2 className="text-sm font-medium text-amber-200">Send /clear?</h2>
        </header>
        <div className="space-y-2 px-5 py-4 text-xs leading-relaxed text-zinc-300">
          <p>
            <span className="font-semibold text-amber-300">/clear</span> ends the current Claude
            session and starts a fresh one — the current context window is lost.
          </p>
          <p className="text-zinc-400">
            The successor session in the same cwd will be detected automatically and your custom
            session title will carry forward, but in-flight reasoning and uncommitted scratchpad
            content cannot be recovered.
          </p>
        </div>
        <footer className="flex items-center justify-end gap-2 border-t border-zinc-800 px-5 py-3">
          <button
            onClick={onCancel}
            className="rounded-md border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:border-zinc-500"
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            className="rounded-md bg-amber-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-amber-500"
          >
            Yes, send /clear
          </button>
        </footer>
      </div>
    </div>
  )
}

function Tab({
  info,
  active,
  onSelect,
  onClose
}: {
  info: PtyInfo
  active: boolean
  onSelect: () => void
  onClose: () => void
}): JSX.Element {
  return (
    <div
      className={clsx(
        'group flex shrink-0 items-center gap-1 rounded-md border px-2 py-0.5 text-[11px] transition',
        active
          ? 'border-zinc-600 bg-zinc-800 text-zinc-100'
          : 'border-transparent text-zinc-400 hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-200',
        !info.alive && 'opacity-60'
      )}
    >
      <button onClick={onSelect} className="max-w-[180px] truncate text-left" title={info.label}>
        {!info.alive && <span className="text-rose-400">●</span>}
        {info.alive && <span className="text-emerald-400">●</span>}
        <span className="ml-1">{info.label}</span>
      </button>
      <button
        onClick={onClose}
        title="Kill"
        className="ml-1 hidden rounded text-zinc-500 hover:text-rose-300 group-hover:inline"
      >
        ×
      </button>
    </div>
  )
}
