import { open, readdir, stat } from 'node:fs/promises'
import { basename, join } from 'node:path'
import chokidar, { type FSWatcher } from 'chokidar'
import { CLAUDE_PROJECTS } from './paths'
import { modelLimit } from './models'

export interface TranscriptInfo {
  sessionId: string
  transcriptPath: string
  cwd: string
  lastActivity: number
  context?: { used: number; limit: number; model: string; modelKnown: boolean }
  firstUserMessage?: string
  workflow?: string
  workflowParams?: Record<string, unknown>
}

interface FileState {
  offset: number
  size: number
  lastInfo: TranscriptInfo
}

type Listener = (info: TranscriptInfo) => void

export class TranscriptService {
  private files = new Map<string, FileState>()
  private watcher: FSWatcher | null = null
  private listeners = new Set<Listener>()

  async start(): Promise<void> {
    await this.initialScan()
    this.watcher = chokidar.watch(CLAUDE_PROJECTS, {
      ignoreInitial: true,
      depth: 2,
      ignored: (p) => p.includes('/.git/') || p.endsWith('.DS_Store')
    })
    this.watcher
      .on('add', (p) => void this.onChange(p))
      .on('change', (p) => void this.onChange(p))
  }

  stop(): void {
    void this.watcher?.close()
    this.watcher = null
  }

  onUpdate(cb: Listener): () => void {
    this.listeners.add(cb)
    return () => this.listeners.delete(cb)
  }

  snapshot(): TranscriptInfo[] {
    return Array.from(this.files.values()).map((f) => f.lastInfo)
  }

  private async initialScan(): Promise<void> {
    let projectDirs: string[] = []
    try {
      projectDirs = await readdir(CLAUDE_PROJECTS)
    } catch {
      return
    }
    for (const dir of projectDirs) {
      const full = join(CLAUDE_PROJECTS, dir)
      try {
        const s = await stat(full)
        if (!s.isDirectory()) continue
        const files = await readdir(full)
        for (const f of files) {
          if (!f.endsWith('.jsonl')) continue
          await this.onChange(join(full, f))
        }
      } catch {
        // ignore
      }
    }
  }

  private async onChange(path: string): Promise<void> {
    if (!path.endsWith('.jsonl')) return
    let st: Awaited<ReturnType<typeof stat>>
    try {
      st = await stat(path)
    } catch {
      return
    }
    const sessionId = basename(path, '.jsonl')
    const prev = this.files.get(path)
    const startOffset = prev && st.size >= prev.size ? prev.offset : 0

    const fh = await open(path, 'r')
    try {
      const length = st.size - startOffset
      if (length <= 0) {
        if (prev) {
          prev.size = st.size
          prev.lastInfo = { ...prev.lastInfo, lastActivity: st.mtimeMs }
          this.emit(prev.lastInfo)
        }
        return
      }
      const buf = Buffer.alloc(length)
      await fh.read(buf, 0, length, startOffset)
      const text = buf.toString('utf8')
      const lines = text.split('\n').filter((l) => l.length > 0)
      const info = parseLines(sessionId, path, lines, prev?.lastInfo)
      info.lastActivity = st.mtimeMs
      this.files.set(path, { offset: st.size, size: st.size, lastInfo: info })
      this.emit(info)
    } finally {
      await fh.close()
    }
  }

  private emit(info: TranscriptInfo): void {
    for (const l of this.listeners) l(info)
  }
}

const RICK_RUN_RE = /\/rick\s+run\s+([a-z][a-z0-9._-]*)/m
const RICK_ANNOUNCE_RE = /Running\s+\*\*([a-z][a-z0-9._-]*)\*\*/i

function parseLines(
  sessionId: string,
  transcriptPath: string,
  lines: string[],
  prev: TranscriptInfo | undefined
): TranscriptInfo {
  let cwd = prev?.cwd ?? ''
  let firstUserMessage = prev?.firstUserMessage
  let context = prev?.context
  let workflow = prev?.workflow
  let workflowParams = prev?.workflowParams

  for (const line of lines) {
    let msg: any
    try {
      msg = JSON.parse(line)
    } catch {
      continue
    }
    if (typeof msg.cwd === 'string' && !cwd) cwd = msg.cwd

    if (msg.type === 'user') {
      const content = msg.message?.content
      let userText: string | undefined
      if (typeof content === 'string') userText = content
      else if (Array.isArray(content)) {
        userText = content.find((c: any) => c?.type === 'text')?.text
      }
      if (!firstUserMessage && typeof userText === 'string') firstUserMessage = userText
      if (typeof userText === 'string') {
        if (!workflow) {
          const m = userText.match(RICK_RUN_RE)
          if (m) workflow = m[1]
        }
        if (!workflowParams) {
          const pm = userText.match(/--params=(['"])(\{[\s\S]*?\})\1/)
          if (pm) {
            try {
              workflowParams = JSON.parse(pm[2]) as Record<string, unknown>
            } catch {
              // ignore malformed JSON
            }
          }
        }
      }
    }

    if (msg.type === 'assistant') {
      const m = msg.message
      const usage = m?.usage
      if (m?.model && usage) {
        const used =
          (usage.input_tokens ?? 0) +
          (usage.cache_read_input_tokens ?? 0) +
          (usage.cache_creation_input_tokens ?? 0)
        let { limit, known } = modelLimit(m.model)
        // The base model id (e.g. claude-opus-4-7) doesn't carry the [1m] tier
        // tag, so we promote to 1M context the moment observed usage proves the
        // session is running on the extended tier. Persist across the session
        // so the bar doesn't oscillate as later turns drop below 200k.
        if (used > limit || (context && context.limit > limit)) {
          limit = Math.max(limit, 1_000_000, context?.limit ?? 0)
        }
        context = { used, limit, model: m.model, modelKnown: known }
      }
      if (!workflow && Array.isArray(m?.content)) {
        for (const block of m.content) {
          if (block?.type === 'text' && typeof block.text === 'string') {
            const ann = block.text.match(RICK_ANNOUNCE_RE)
            if (ann) {
              workflow = ann[1]
              break
            }
          }
        }
      }
    }
  }

  return {
    sessionId,
    transcriptPath,
    cwd,
    lastActivity: Date.now(),
    context,
    firstUserMessage,
    workflow,
    workflowParams
  }
}
