import { unlink } from 'node:fs/promises'
import type { Session, SessionFilesDTO, SessionStatus, SessionSummary, Workflow } from '@shared/types'
import type { TranscriptInfo } from './transcripts'
import type { TranscriptService } from './transcripts'
import type { TrackingFile, TrackingService } from './tracking'
import { summarizeTranscript } from './summary'
import { findSpecFiles } from './specs'
import { correlateWorkflow } from './correlation'

const RUNNING_THRESHOLD_MS = 30_000

export interface SessionsViewOptions {
  recentSessionDays: number
  universeFilter?: string
  archivedSessionIds?: string[]
  customTitles?: Record<string, string>
}

type Listener = (sessions: Session[]) => void

export class SessionsService {
  private listeners = new Set<Listener>()
  private opts: SessionsViewOptions = { recentSessionDays: 7 }
  private cached: Session[] = []

  private workflowsLookup: () => Workflow[] = () => []

  constructor(
    private readonly transcripts: TranscriptService,
    private readonly tracking: TrackingService
  ) {}

  setWorkflowsLookup(fn: () => Workflow[]): void {
    this.workflowsLookup = fn
  }

  start(): void {
    this.transcripts.onUpdate(() => this.recompute())
    this.tracking.onUpdate(() => this.recompute())
    this.tracking.onRemoval(() => this.recompute())
    this.recompute()
  }

  setOptions(patch: Partial<SessionsViewOptions>): void {
    this.opts = { ...this.opts, ...patch }
    this.recompute()
  }

  onUpdate(cb: Listener): () => void {
    this.listeners.add(cb)
    return () => this.listeners.delete(cb)
  }

  snapshot(): Session[] {
    return this.cached
  }

  async listSessionFiles(sessionId: string): Promise<SessionFilesDTO> {
    const tracking = this.tracking.get(sessionId)
    const session = this.cached.find((s) => s.id === sessionId)
    const cwd = session?.cwd ?? ''

    const [specsRaw, touchedRaw] = await Promise.all([
      cwd ? findSpecFiles({ cwd }) : Promise.resolve<string[]>([]),
      this.touchedFilesFromTranscript(session)
    ])

    const dto: SessionFilesDTO = {
      tracking: tracking?.path,
      specs: specsRaw,
      touched: touchedRaw,
      transcript: session?.transcriptPath
    }

    if (session?.workflow && session.transcriptPath) {
      const workflow =
        this.workflowsLookup().find(
          (w) =>
            w.name === session.workflow &&
            (!session.universe || w.universe === session.universe)
        ) ?? this.workflowsLookup().find((w) => w.name === session.workflow)
      if (workflow) {
        const summary = await summarizeTranscript(session.id, session.transcriptPath)
        const firstPrompt = summary?.recent.find((m) => m.type === 'user')?.text
        try {
          const view = await correlateWorkflow(session.transcriptPath, firstPrompt, workflow)
          if (view) dto.workflowRun = view
        } catch {
          // ignore correlation failures
        }
      }
    }

    return dto
  }

  private async touchedFilesFromTranscript(session: Session | undefined): Promise<string[]> {
    if (!session?.transcriptPath) return []
    try {
      const summary = await summarizeTranscript(session.id, session.transcriptPath)
      return summary?.touchedFiles ?? []
    } catch {
      return []
    }
  }

  async discardSession(sessionId: string): Promise<void> {
    const trackingFile = this.tracking.get(sessionId)
    if (trackingFile) {
      try {
        await unlink(trackingFile.path)
      } catch {
        // ignore — tracking watcher will handle removal
      }
    }
    const archived = new Set(this.opts.archivedSessionIds ?? [])
    archived.add(sessionId)
    this.opts.archivedSessionIds = Array.from(archived)
    this.recompute()
  }

  async summarize(sessionId: string): Promise<SessionSummary | null> {
    const session = this.cached.find((s) => s.id === sessionId)
    if (!session?.transcriptPath) return null
    const summary = await summarizeTranscript(sessionId, session.transcriptPath)
    if (summary) {
      if (!summary.workflow && session.workflow) summary.workflow = session.workflow
    }
    return summary
  }

  async unarchiveSession(sessionId: string): Promise<void> {
    const archived = new Set(this.opts.archivedSessionIds ?? [])
    archived.delete(sessionId)
    this.opts.archivedSessionIds = Array.from(archived)
    this.recompute()
  }

  private recompute(): void {
    const transcripts = this.transcripts.snapshot()
    const trackings = new Map<string, TrackingFile>()
    for (const t of this.tracking.snapshot()) trackings.set(t.sessionId, t)

    const cutoff = Date.now() - this.opts.recentSessionDays * 24 * 60 * 60 * 1000
    const sessions: Session[] = []

    for (const t of transcripts) {
      if (t.lastActivity < cutoff) continue
      const track = trackings.get(t.sessionId)
      sessions.push(this.toSession(t, track))
    }

    // Tracking files without a known transcript still surface (covers app-only state).
    for (const [sid, track] of trackings) {
      if (sessions.some((s) => s.id === sid)) continue
      const customTitle = this.opts.customTitles?.[sid]
      sessions.push({
        id: sid,
        title: customTitle,
        customTitle,
        workflow: track.frontmatter.workflow,
        universe: track.frontmatter.universe,
        cwd: '',
        transcriptPath: '',
        trackingPath: track.path,
        status: deriveStatus(undefined, track),
        phase: track.frontmatter.phase,
        total: track.frontmatter.total,
        completed: track.frontmatter.completed,
        current: track.frontmatter.current,
        lastActivity: track.mtime,
        startedAt: parseDate(track.frontmatter.started)
      })
    }

    // Detect /clear-style successors: if a newer session shares the cwd of an
    // older one (within a reasonable window), the older one is effectively
    // ended. Mark it `done` and let its custom title bleed into the successor
    // so the user can spot the continuation in the drawer.
    propagateContinuations(sessions)

    let filtered = sessions
    if (this.opts.universeFilter) {
      const u = this.opts.universeFilter
      filtered = filtered.filter((s) => !s.universe || s.universe === u)
    }
    const archived = new Set(this.opts.archivedSessionIds ?? [])
    filtered = filtered.filter((s) => !archived.has(s.id))
    this.cached = filtered.sort((a, b) => b.lastActivity - a.lastActivity)
    for (const l of this.listeners) l(this.cached)
  }

  private toSession(t: TranscriptInfo, track: TrackingFile | undefined): Session {
    const fm = track?.frontmatter
    const inferredWorkflow =
      fm?.workflow ?? t.workflow ?? extractWorkflowFromPrompt(t.firstUserMessage)
    const customTitle = this.opts.customTitles?.[t.sessionId]
    return {
      id: t.sessionId,
      title: customTitle ?? deriveTitle(t.workflowParams),
      customTitle,
      workflow: inferredWorkflow,
      universe: fm?.universe,
      cwd: t.cwd,
      transcriptPath: t.transcriptPath,
      trackingPath: track?.path,
      status: deriveStatus(t, track),
      phase: fm?.phase,
      total: fm?.total,
      completed: fm?.completed,
      current: fm?.current,
      context: t.context,
      lastActivity: t.lastActivity,
      startedAt: parseDate(fm?.started)
    }
  }
}

const CONTINUATION_WINDOW_MS = 60 * 60_000 // 1 hour

function propagateContinuations(sessions: Session[]): void {
  const byCwd = new Map<string, Session[]>()
  for (const s of sessions) {
    if (!s.cwd) continue
    let arr = byCwd.get(s.cwd)
    if (!arr) {
      arr = []
      byCwd.set(s.cwd, arr)
    }
    arr.push(s)
  }
  for (const arr of byCwd.values()) {
    if (arr.length < 2) continue
    arr.sort((a, b) => (a.startedAt ?? a.lastActivity) - (b.startedAt ?? b.lastActivity))
    for (let i = 0; i < arr.length - 1; i++) {
      const prev = arr[i]
      const next = arr[i + 1]
      const gap = (next.startedAt ?? next.lastActivity) - prev.lastActivity
      if (gap > CONTINUATION_WINDOW_MS) continue
      // Older session is finished — promote to `done` so it stops looking active.
      if (prev.status !== 'blocked') prev.status = 'done'
      prev.successorId = next.id
      next.predecessorId = prev.id
      // Carry the user's custom title forward only when the newer session
      // doesn't already have one (custom or auto-derived). Param-derived
      // titles win over inherited ones.
      if (prev.customTitle && !next.customTitle && !next.title) {
        next.title = prev.customTitle
      }
    }
  }
}

function deriveStatus(t: TranscriptInfo | undefined, track: TrackingFile | undefined): SessionStatus {
  const explicit = track?.frontmatter.status
  if (explicit === 'blocked' || explicit === 'done' || explicit === 'waiting') return explicit

  const recent = t && Date.now() - t.lastActivity < RUNNING_THRESHOLD_MS
  if (recent) return 'running'
  return 'idle'
}

function extractWorkflowFromPrompt(text: string | undefined): string | undefined {
  if (!text) return undefined
  const match = text.match(/^\/rick\s+run\s+([^\s]+)/m)
  return match?.[1]
}

function deriveTitle(params: Record<string, unknown> | undefined): string | undefined {
  if (!params) return undefined
  const order = ['ticket_key', 'ticket', 'feature', 'job', 'name', 'feature_name']
  for (const k of order) {
    const v = params[k]
    if (typeof v === 'string' && v.trim()) return v.trim()
  }
  for (const v of Object.values(params)) {
    if (typeof v === 'string' && v.trim()) return v.trim()
  }
  return undefined
}

function parseDate(iso: string | undefined): number | undefined {
  if (!iso) return undefined
  const t = Date.parse(iso)
  return Number.isFinite(t) ? t : undefined
}
