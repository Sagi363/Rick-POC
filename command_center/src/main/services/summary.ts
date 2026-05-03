import { open, stat } from 'node:fs/promises'
import type { SessionSummary, SessionSummaryEntry } from '@shared/types'
import { modelLimit } from './models'

const RECENT_LIMIT = 12
const SUBAGENT_LIMIT = 20
const HISTORY_SAMPLE = 40

export async function summarizeTranscript(
  sessionId: string,
  path: string
): Promise<SessionSummary | null> {
  let st: Awaited<ReturnType<typeof stat>>
  try {
    st = await stat(path)
  } catch {
    return null
  }

  const fh = await open(path, 'r')
  try {
    const buf = Buffer.alloc(st.size)
    await fh.read(buf, 0, st.size, 0)
    const lines = buf.toString('utf8').split('\n')

    const summary: SessionSummary = {
      id: sessionId,
      cwd: '',
      lastActivity: st.mtimeMs,
      totalMessages: 0,
      totalUserMessages: 0,
      totalAssistantMessages: 0,
      toolCounts: {},
      subagentSpawns: [],
      recent: [],
      contextHistory: [],
      touchedFiles: []
    }

    const recentBuf: SessionSummaryEntry[] = []
    const touchedAt = new Map<string, number>()

    for (const line of lines) {
      if (!line) continue
      let msg: any
      try {
        msg = JSON.parse(line)
      } catch {
        continue
      }
      if (typeof msg.cwd === 'string' && !summary.cwd) summary.cwd = msg.cwd
      if (msg.timestamp && !summary.startedAt) summary.startedAt = Date.parse(msg.timestamp)

      if (msg.type === 'user') {
        summary.totalMessages++
        summary.totalUserMessages++
        const text = extractUserText(msg.message?.content)
        if (text) {
          if (!summary.workflow) {
            const m = text.match(/^\/rick\s+run\s+([^\s]+)/m)
            if (m) summary.workflow = m[1]
          }
          recentBuf.push({
            type: 'user',
            timestamp: msg.timestamp ? Date.parse(msg.timestamp) : 0,
            text: truncate(text, 600),
            toolUses: []
          })
        }
      } else if (msg.type === 'assistant') {
        summary.totalMessages++
        summary.totalAssistantMessages++
        const m = msg.message
        if (m?.model && !summary.model) summary.model = m.model

        if (m?.usage) {
          const used =
            (m.usage.input_tokens ?? 0) +
            (m.usage.cache_read_input_tokens ?? 0) +
            (m.usage.cache_creation_input_tokens ?? 0)
          const { limit } = modelLimit(m.model ?? '')
          summary.contextHistory.push({
            timestamp: msg.timestamp ? Date.parse(msg.timestamp) : 0,
            used,
            limit
          })
        }

        const content = Array.isArray(m?.content) ? m.content : []
        const texts: string[] = []
        const toolUses: { name: string; brief: string }[] = []
        for (const block of content) {
          if (block?.type === 'text' && typeof block.text === 'string') {
            texts.push(block.text)
          } else if (block?.type === 'tool_use') {
            const name = String(block.name ?? 'unknown')
            summary.toolCounts[name] = (summary.toolCounts[name] ?? 0) + 1
            toolUses.push({ name, brief: briefForToolUse(name, block.input) })
            const ts = msg.timestamp ? Date.parse(msg.timestamp) : 0
            if (name === 'Task') {
              summary.subagentSpawns.push({
                name: String(block.input?.subagent_type ?? 'general-purpose'),
                description: String(block.input?.description ?? ''),
                timestamp: ts
              })
            }
            if (name === 'Read' || name === 'Write' || name === 'Edit' || name === 'NotebookEdit') {
              const path = block.input?.file_path ?? block.input?.path ?? block.input?.notebook_path
              if (typeof path === 'string' && path) {
                const existing = touchedAt.get(path) ?? 0
                if (ts > existing) touchedAt.set(path, ts)
              }
            }
          }
        }
        const joined = texts.join('\n').trim()
        if (joined || toolUses.length) {
          recentBuf.push({
            type: 'assistant',
            timestamp: msg.timestamp ? Date.parse(msg.timestamp) : 0,
            text: joined ? truncate(joined, 600) : undefined,
            toolUses
          })
        }
      }
    }

    summary.recent = recentBuf.slice(-RECENT_LIMIT)
    summary.subagentSpawns = summary.subagentSpawns.slice(-SUBAGENT_LIMIT)
    summary.touchedFiles = Array.from(touchedAt.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([path]) => path)
    if (summary.contextHistory.length > HISTORY_SAMPLE) {
      const step = Math.ceil(summary.contextHistory.length / HISTORY_SAMPLE)
      summary.contextHistory = summary.contextHistory.filter((_, i) => i % step === 0)
    }
    return summary
  } finally {
    await fh.close()
  }
}

function extractUserText(content: unknown): string | undefined {
  if (typeof content === 'string') return content
  if (!Array.isArray(content)) return undefined
  const parts: string[] = []
  for (const block of content) {
    if (block?.type === 'text' && typeof block.text === 'string') parts.push(block.text)
    else if (block?.type === 'tool_result') {
      // skip — too noisy
    }
  }
  return parts.join('\n').trim() || undefined
}

function briefForToolUse(name: string, input: any): string {
  if (!input || typeof input !== 'object') return ''
  switch (name) {
    case 'Bash':
      return truncate(String(input.command ?? ''), 120)
    case 'Read':
    case 'Write':
    case 'Edit':
      return truncate(String(input.file_path ?? input.path ?? ''), 120)
    case 'Grep':
      return truncate(String(input.pattern ?? ''), 120)
    case 'Glob':
      return truncate(String(input.pattern ?? ''), 120)
    case 'Task':
      return truncate(
        `${input.subagent_type ?? 'agent'}: ${input.description ?? ''}`,
        140
      )
    case 'TodoWrite':
      return `${(input.todos ?? []).length} todos`
    default: {
      const keys = Object.keys(input).slice(0, 3).join(', ')
      return keys ? `(${keys})` : ''
    }
  }
}

function truncate(text: string, max: number): string {
  if (text.length <= max) return text
  return text.slice(0, max - 1) + '…'
}
