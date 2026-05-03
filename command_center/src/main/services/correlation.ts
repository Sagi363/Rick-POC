import { open, stat } from 'node:fs/promises'
import type { Workflow, WorkflowRunView, WorkflowStepView } from '@shared/types'

interface TaskSpawn {
  toolUseId: string
  ownerUuid: string
  subagentType: string
  description: string
  timestamp: number
  status: 'running' | 'done'
  files: Set<string>
}

const FILE_TOOLS = new Set(['Read', 'Write', 'Edit', 'MultiEdit', 'NotebookEdit'])

/**
 * Walks a session transcript and produces a workflow-run view:
 * each YAML step gets its observed files (from subagent sidechain) and run state.
 */
export async function correlateWorkflow(
  transcriptPath: string,
  firstUserPrompt: string | undefined,
  workflow: Workflow
): Promise<WorkflowRunView | null> {
  let st
  try {
    st = await stat(transcriptPath)
  } catch {
    return null
  }
  const fh = await open(transcriptPath, 'r')
  let lines: string[]
  try {
    const buf = Buffer.alloc(st.size)
    await fh.read(buf, 0, st.size, 0)
    lines = buf.toString('utf8').split('\n')
  } finally {
    await fh.close()
  }

  const messages: Record<string, any>[] = []
  for (const line of lines) {
    if (!line) continue
    try {
      messages.push(JSON.parse(line))
    } catch {
      // skip malformed
    }
  }

  // Build parent lookup so sidechain messages can be traced back to their
  // owning main-thread Task spawn.
  const byUuid = new Map<string, Record<string, any>>()
  for (const m of messages) {
    if (typeof m.uuid === 'string') byUuid.set(m.uuid, m)
  }

  // Collect main-thread Task spawns and main-thread tool_results (for status).
  const tasks = new Map<string, TaskSpawn>() // by toolUseId
  const completed = new Set<string>() // toolUseIds that have a tool_result
  for (const m of messages) {
    if (m.isSidechain) continue
    const content = Array.isArray(m.message?.content) ? m.message.content : []
    for (const block of content) {
      if (block?.type === 'tool_use' && block.name === 'Task') {
        const id = String(block.id ?? '')
        if (!id) continue
        tasks.set(id, {
          toolUseId: id,
          ownerUuid: String(m.uuid ?? ''),
          subagentType: String(block.input?.subagent_type ?? ''),
          description: String(block.input?.description ?? ''),
          timestamp: m.timestamp ? Date.parse(m.timestamp) : 0,
          status: 'running',
          files: new Set<string>()
        })
      } else if (block?.type === 'tool_result' && typeof block.tool_use_id === 'string') {
        completed.add(block.tool_use_id)
      }
    }
  }
  for (const t of tasks.values()) {
    if (completed.has(t.toolUseId)) t.status = 'done'
  }

  // Walk parentUuid chain to find the spawn point (a main-thread message that
  // contains a Task tool_use). Returns the toolUseId of the owning Task, if any.
  const ownerCache = new Map<string, string | null>()
  function ownerOf(uuid: string | undefined): string | null {
    if (!uuid) return null
    if (ownerCache.has(uuid)) return ownerCache.get(uuid)!
    const seen = new Set<string>()
    let cur = uuid
    while (cur && !seen.has(cur)) {
      seen.add(cur)
      const m = byUuid.get(cur)
      if (!m) break
      if (!m.isSidechain) {
        const content = Array.isArray(m.message?.content) ? m.message.content : []
        for (const block of content) {
          if (block?.type === 'tool_use' && block.name === 'Task' && block.id && tasks.has(String(block.id))) {
            ownerCache.set(uuid, String(block.id))
            return String(block.id)
          }
        }
        break
      }
      cur = m.parentUuid
    }
    ownerCache.set(uuid, null)
    return null
  }

  // Attribute file activity inside sidechain messages back to their owning Task.
  for (const m of messages) {
    if (!m.isSidechain) continue
    const owner = ownerOf(m.parentUuid)
    if (!owner) continue
    const task = tasks.get(owner)
    if (!task) continue
    const content = Array.isArray(m.message?.content) ? m.message.content : []
    for (const block of content) {
      if (block?.type !== 'tool_use') continue
      if (!FILE_TOOLS.has(block.name)) continue
      const path =
        block.input?.file_path ??
        block.input?.path ??
        block.input?.notebook_path ??
        ''
      if (typeof path === 'string' && path) task.files.add(path)
    }
  }

  // Fallback: parse Rick's mandatory announcements from main-thread assistant text.
  // Reliable signal for composed workflows where `step.agent` is a workflow name
  // (e.g. "ticket-research") and never appears as a subagent_type.
  const announcements = parseRickAnnouncements(messages)

  return buildView({
    workflow,
    tasks: Array.from(tasks.values()),
    announcements,
    firstUserPrompt
  })
}

interface RickAnnouncements {
  handoffs: number
  completions: number
  inFlight: boolean
  phaseMarkers: PhaseMarker[]
  phaseProse: PhaseProse[]
  /** Last main-thread "Handing to …" detail seen — the live activity for
   *  whatever outer step is currently running. */
  latestHandoff?: { subagent: string; role?: string; activity: string; timestamp: number }
}

interface PhaseMarker {
  stepId: string
  status: 'starting' | 'complete'
  timestamp: number
}

interface PhaseProse {
  phaseNum: number
  status: 'starting' | 'complete'
  timestamp: number
}

const HANDOFF_RE = /Rick:\s*\*{0,2}\s*Handing\s+to/gi
// Rick's recap: "**Rick:** <agent> is done (...)" or "Rick: <agent> is done"
const DONE_RE = /Rick:\s*\*{0,2}\s*[A-Za-z][\w-]*\s+is\s+done/gi
// Workflow-completion banner: "Rick: All N steps complete"
const COMPLETE_RE = /Rick:\s*\*{0,2}\s*All\s+\d+\s+steps?\s+complete/i
// Deterministic outer-step boundary marker that Rick MUST emit for composed
// (`uses:`) workflows. Format: "[rick:phase <step-id> <starting|complete>]"
// Step-id matches the `id` of the outer step in the workflow YAML.
const PHASE_RE = /\[rick:phase\s+([a-z][\w-]*)\s+(starting|complete)\]/gi
// Natural-language fallback for legacy Rick personas that haven't adopted the
// bracket marker yet. Catches "Phase 1 done", "Phase 2 starting", etc. Mapped
// by phase number → step index. Only fires when no bracket markers are present.
// Negative lookahead skips future-tense forms like "Phase 2 will" / "Phase 2 ="
// where Rick is describing rather than transitioning.
const PHASE_PROSE_RE =
  /\bPhase\s+(\d+)\s+(starting|started|begins?|done|complete|finished)\b/gi
// Detail capture for the live "Handing to <Subagent> — <activity>" line. Used
// to surface what the currently-running step is actually doing. Bold markdown,
// optional "(Role)" suffix, and trailing "[claude:opus]" annotations all skipped.
const HANDOFF_DETAIL_RE =
  /Handing\s+to\s+\*+([^*\n]+?)\*+(?:\s*\(([^)\n]+?)\))?\s*[—–-]+\s*([^\n[]+?)(?:\s*\[|$)/i

function parseRickAnnouncements(messages: Record<string, any>[]): RickAnnouncements {
  let handoffs = 0
  let completions = 0
  let allComplete = false
  const phaseMarkers: PhaseMarker[] = []
  let latestHandoff: RickAnnouncements['latestHandoff']
  for (const m of messages) {
    if (m.isSidechain) continue
    if (m.type !== 'assistant') continue
    const ts = m.timestamp ? Date.parse(m.timestamp) : 0
    const content = Array.isArray(m.message?.content) ? m.message.content : []
    for (const b of content) {
      if (b?.type !== 'text' || typeof b.text !== 'string') continue
      const t = b.text
      handoffs += (t.match(HANDOFF_RE) ?? []).length
      completions += (t.match(DONE_RE) ?? []).length
      if (COMPLETE_RE.test(t)) allComplete = true
      PHASE_RE.lastIndex = 0
      let pm: RegExpExecArray | null
      while ((pm = PHASE_RE.exec(t)) !== null) {
        phaseMarkers.push({
          stepId: pm[1].toLowerCase(),
          status: pm[2].toLowerCase() as 'starting' | 'complete',
          timestamp: ts
        })
      }
      // Per-line scan for handoff detail — one assistant message can contain
      // multiple lines but only one handoff. Last one wins (most recent).
      for (const line of t.split('\n')) {
        const dm = line.match(HANDOFF_DETAIL_RE)
        if (dm) {
          latestHandoff = {
            subagent: dm[1].trim(),
            role: dm[2]?.trim() || undefined,
            activity: dm[3].trim(),
            timestamp: ts
          }
        }
      }
    }
  }
  const phaseProse = collectPhaseProse(messages)
  return {
    handoffs,
    completions,
    inFlight: !allComplete && handoffs > completions,
    phaseMarkers,
    phaseProse,
    latestHandoff
  }
}

function collectPhaseProse(messages: Record<string, any>[]): PhaseProse[] {
  const out: PhaseProse[] = []
  for (const m of messages) {
    if (m.isSidechain) continue
    if (m.type !== 'assistant') continue
    const ts = m.timestamp ? Date.parse(m.timestamp) : 0
    const content = Array.isArray(m.message?.content) ? m.message.content : []
    for (const b of content) {
      if (b?.type !== 'text' || typeof b.text !== 'string') continue
      PHASE_PROSE_RE.lastIndex = 0
      let pm: RegExpExecArray | null
      while ((pm = PHASE_PROSE_RE.exec(b.text)) !== null) {
        const verb = pm[2].toLowerCase()
        const status: 'starting' | 'complete' =
          verb === 'starting' || verb === 'started' || verb === 'begins' || verb === 'begin'
            ? 'starting'
            : 'complete'
        out.push({ phaseNum: Number(pm[1]), status, timestamp: ts })
      }
    }
  }
  return out
}

interface BuildArgs {
  workflow: Workflow
  tasks: TaskSpawn[]
  announcements: RickAnnouncements
  firstUserPrompt?: string
}

function buildView({ workflow, tasks, announcements, firstUserPrompt }: BuildArgs): WorkflowRunView {
  const steps = (workflow.steps ?? []).map<WorkflowStepView>((s) => ({
    id: s.id,
    agent: s.agent,
    collaborators: s.collaborators ?? [],
    description: s.description,
    status: 'pending',
    files: []
  }))

  // Pass 1: agent-name matching (works for direct-agent workflows like specter).
  let anyMatched = false
  for (const t of tasks) {
    const norm = normalizeAgent(t.subagentType)
    const step =
      steps.find((s) => norm === s.agent || s.collaborators.includes(norm)) ??
      steps.find((s) => norm.endsWith(s.agent))
    if (!step) continue
    anyMatched = true
    if (step.status === 'pending') step.status = t.status
    else if (t.status === 'done' && step.status === 'running') step.status = 'done'
    if (!step.startedAt || (t.timestamp && t.timestamp < step.startedAt)) {
      step.startedAt = t.timestamp || step.startedAt
    }
    for (const f of t.files) if (!step.files.includes(f)) step.files.push(f)
  }

  // Pass 2: phase markers — deterministic per-outer-step signal Rick emits for
  // composed (`uses:`) workflows. Wins over Pass 1 wherever both apply because
  // markers are explicit while subagent-name matching is heuristic.
  let markersApplied = false
  if (announcements.phaseMarkers.length > 0) {
    const stepById = new Map(steps.map((s) => [s.id.toLowerCase(), s]))
    for (const marker of announcements.phaseMarkers) {
      const step = stepById.get(marker.stepId)
      if (!step) continue
      markersApplied = true
      if (marker.status === 'complete') {
        step.status = 'done'
      } else if (step.status !== 'done') {
        step.status = 'running'
        if (!step.startedAt || (marker.timestamp && marker.timestamp < step.startedAt)) {
          step.startedAt = marker.timestamp || step.startedAt
        }
      }
    }
  }

  // Pass 2.5: natural-language phase prose ("Phase 1 done", "Phase 2 starting").
  // Catches legacy Rick personas that haven't adopted the bracket marker yet.
  // Maps phase number to step index. Bracket markers (Pass 2) take precedence.
  let proseApplied = false
  if (!markersApplied && announcements.phaseProse.length > 0) {
    for (const p of announcements.phaseProse) {
      const idx = p.phaseNum - 1
      if (idx < 0 || idx >= steps.length) continue
      const step = steps[idx]
      proseApplied = true
      if (p.status === 'complete') {
        step.status = 'done'
      } else if (step.status !== 'done') {
        step.status = 'running'
        if (!step.startedAt || (p.timestamp && p.timestamp < step.startedAt)) {
          step.startedAt = p.timestamp || step.startedAt
        }
      }
    }
    // After "Phase N complete", treat phase N+1 as running unless something
    // explicit says otherwise — Rick often skips an explicit "Phase N+1 starting".
    const lastComplete = announcements.phaseProse
      .filter((p) => p.status === 'complete')
      .reduce((max, p) => Math.max(max, p.phaseNum), 0)
    if (lastComplete > 0 && lastComplete < steps.length) {
      const next = steps[lastComplete]
      if (next.status === 'pending') next.status = 'running'
    }
  }

  // Pass 3: legacy announcement-counting fallback. Only fires when none of the
  // earlier passes produced a signal. Brittle for composed workflows; kept as
  // a last-resort heuristic.
  if (!anyMatched && !markersApplied && !proseApplied && (announcements.handoffs > 0 || announcements.completions > 0)) {
    const completed = Math.min(announcements.completions, steps.length)
    for (let i = 0; i < completed; i++) steps[i].status = 'done'
    if (announcements.inFlight && completed < steps.length) {
      steps[completed].status = 'running'
    }
  }

  // Attach the latest handoff detail to the highest-indexed running step so
  // the UI can show "Trinity · write 4 KMP integration tests" beneath the
  // ring instead of the generic sub-workflow name.
  if (announcements.latestHandoff) {
    for (let i = steps.length - 1; i >= 0; i--) {
      if (steps[i].status === 'running') {
        steps[i].currentSubagent = announcements.latestHandoff.role
          ? `${announcements.latestHandoff.subagent} (${announcements.latestHandoff.role})`
          : announcements.latestHandoff.subagent
        steps[i].currentActivity = announcements.latestHandoff.activity
        break
      }
    }
  }

  const params = parseParamsFromPrompt(firstUserPrompt)
  const feature = pickFeatureLabel(params)
  const label = feature ? `${workflow.name} · ${feature}` : workflow.name

  return { workflow: workflow.name, label, feature, steps }
}

function normalizeAgent(subagentType: string): string {
  // Examples:
  //   "rick-ACC_issues_universe-sherlock"  → "sherlock"
  //   "rick-Issues-Team-sherlock"          → "sherlock"
  //   "general-purpose"                    → "general-purpose"
  if (!subagentType.startsWith('rick-')) return subagentType
  const idx = subagentType.lastIndexOf('-')
  return idx > 0 ? subagentType.slice(idx + 1) : subagentType
}

function parseParamsFromPrompt(prompt: string | undefined): Record<string, unknown> | null {
  if (!prompt) return null
  const m = prompt.match(/--params=(['"])(\{[\s\S]*?\})\1/)
  if (!m) return null
  try {
    return JSON.parse(m[2]) as Record<string, unknown>
  } catch {
    return null
  }
}

function pickFeatureLabel(params: Record<string, unknown> | null): string | undefined {
  if (!params) return undefined
  const order = ['feature', 'ticket_key', 'ticket', 'job', 'name', 'feature_name']
  for (const key of order) {
    const v = params[key]
    if (typeof v === 'string' && v) return v
  }
  for (const v of Object.values(params)) {
    if (typeof v === 'string' && v) return v
  }
  return undefined
}
