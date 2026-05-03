import { mkdir, readdir, readFile, rename, writeFile, stat } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import chokidar, { type FSWatcher } from 'chokidar'
import matter from 'gray-matter'
import { RICK_TRACKING } from './paths'
import type { SessionStatus } from '@shared/types'

export interface TrackingFrontmatter {
  session_id?: string
  workflow?: string
  universe?: string
  status?: SessionStatus
  total?: number
  completed?: number
  current?: string
  phase?: string
  started?: string
  updated?: string
}

export interface TrackingFile {
  sessionId: string
  path: string
  frontmatter: TrackingFrontmatter
  body: string
  mtime: number
}

type Listener = (file: TrackingFile) => void
type RemovalListener = (sessionId: string) => void

const WRITE_GUARD_DIR = RICK_TRACKING

export class TrackingService {
  private files = new Map<string, TrackingFile>()
  private watcher: FSWatcher | null = null
  private updateListeners = new Set<Listener>()
  private removalListeners = new Set<RemovalListener>()

  async start(): Promise<void> {
    await mkdir(RICK_TRACKING, { recursive: true })
    await this.initialScan()
    this.watcher = chokidar.watch(RICK_TRACKING, {
      ignoreInitial: true,
      ignored: (p) => p.endsWith('.tmp') || p.endsWith('.DS_Store')
    })
    this.watcher
      .on('add', (p) => void this.onChange(p))
      .on('change', (p) => void this.onChange(p))
      .on('unlink', (p) => this.onUnlink(p))
  }

  stop(): void {
    void this.watcher?.close()
    this.watcher = null
  }

  onUpdate(cb: Listener): () => void {
    this.updateListeners.add(cb)
    return () => this.updateListeners.delete(cb)
  }

  onRemoval(cb: RemovalListener): () => void {
    this.removalListeners.add(cb)
    return () => this.removalListeners.delete(cb)
  }

  snapshot(): TrackingFile[] {
    return Array.from(this.files.values())
  }

  get(sessionId: string): TrackingFile | undefined {
    return this.files.get(sessionId)
  }

  async readRaw(sessionId: string): Promise<string | null> {
    const path = join(RICK_TRACKING, `${sessionId}.md`)
    try {
      return await readFile(path, 'utf8')
    } catch {
      return null
    }
  }

  async writeAtomic(sessionId: string, content: string): Promise<void> {
    const path = join(RICK_TRACKING, `${sessionId}.md`)
    if (!isWithin(path, WRITE_GUARD_DIR)) {
      throw new Error(`Refusing to write outside ${WRITE_GUARD_DIR}: ${path}`)
    }
    await mkdir(dirname(path), { recursive: true })
    const tmp = `${path}.${process.pid}.${Date.now()}.tmp`
    await writeFile(tmp, content, 'utf8')
    await rename(tmp, path)
  }

  private async initialScan(): Promise<void> {
    let entries: string[] = []
    try {
      entries = await readdir(RICK_TRACKING)
    } catch {
      return
    }
    for (const f of entries) {
      if (!f.endsWith('.md')) continue
      await this.onChange(join(RICK_TRACKING, f))
    }
  }

  private async onChange(path: string): Promise<void> {
    if (!path.endsWith('.md')) return
    let raw: string
    let st: Awaited<ReturnType<typeof stat>>
    try {
      raw = await readFile(path, 'utf8')
      st = await stat(path)
    } catch {
      return
    }
    const parsed = matter(raw)
    const fm = parsed.data as TrackingFrontmatter
    const sessionId = fm.session_id ?? path.split('/').pop()!.replace(/\.md$/, '')
    const file: TrackingFile = {
      sessionId,
      path,
      frontmatter: fm,
      body: parsed.content,
      mtime: st.mtimeMs
    }
    this.files.set(sessionId, file)
    for (const l of this.updateListeners) l(file)
  }

  private onUnlink(path: string): void {
    const sessionId = path.split('/').pop()!.replace(/\.md$/, '')
    if (this.files.delete(sessionId)) {
      for (const l of this.removalListeners) l(sessionId)
    }
  }
}

function isWithin(child: string, parent: string): boolean {
  const c = child.endsWith('/') ? child : child + '/'
  const p = parent.endsWith('/') ? parent : parent + '/'
  return c.startsWith(p)
}
