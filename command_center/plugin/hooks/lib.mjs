import { mkdir, readFile, rename, writeFile, stat } from 'node:fs/promises'
import { homedir } from 'node:os'
import { join, dirname } from 'node:path'

const HOME = homedir()
export const TRACKING_DIR = join(HOME, '.rick', 'tracking')

const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/

const SECTIONS = ['## Todos', '## Phase log', '## Artifacts']

export function trackingPath(sessionId) {
  return join(TRACKING_DIR, `${sessionId}.md`)
}

export async function readJsonStdin() {
  const chunks = []
  for await (const chunk of process.stdin) chunks.push(chunk)
  const raw = Buffer.concat(chunks).toString('utf8').trim()
  if (!raw) return {}
  try {
    return JSON.parse(raw)
  } catch {
    return {}
  }
}

export async function readTracking(sessionId) {
  const path = trackingPath(sessionId)
  let raw = ''
  try {
    raw = await readFile(path, 'utf8')
  } catch {
    return { frontmatter: {}, sections: emptySections() }
  }
  const m = raw.match(FRONTMATTER_RE)
  const body = m ? m[2] : raw
  const frontmatter = m ? parseFrontmatter(m[1]) : {}
  return { frontmatter, sections: parseSections(body) }
}

export async function writeTracking(sessionId, { frontmatter, sections }) {
  const path = trackingPath(sessionId)
  await mkdir(dirname(path), { recursive: true })
  frontmatter.session_id = sessionId
  frontmatter.updated = new Date().toISOString()
  if (!frontmatter.started) frontmatter.started = frontmatter.updated
  const fm = serializeFrontmatter(frontmatter)
  const body = serializeSections(sections)
  const content = `---\n${fm}---\n\n${body}\n`
  const tmp = `${path}.${process.pid}.${Date.now()}.tmp`
  await writeFile(tmp, content, 'utf8')
  await rename(tmp, path)
}

export function emptySections() {
  return { '## Todos': '', '## Phase log': '', '## Artifacts': '' }
}

function parseFrontmatter(text) {
  const out = {}
  for (const line of text.split('\n')) {
    const m = line.match(/^([a-zA-Z0-9_]+):\s*(.*)$/)
    if (!m) continue
    let v = m[2].trim()
    if (v === 'null' || v === '') {
      out[m[1]] = null
      continue
    }
    if (/^-?\d+$/.test(v)) {
      out[m[1]] = Number(v)
      continue
    }
    if (v.startsWith('"') && v.endsWith('"')) v = v.slice(1, -1)
    out[m[1]] = v
  }
  return out
}

function serializeFrontmatter(obj) {
  const order = [
    'session_id',
    'workflow',
    'universe',
    'status',
    'phase',
    'total',
    'completed',
    'current',
    'started',
    'updated'
  ]
  const seen = new Set()
  const lines = []
  for (const k of order) {
    if (k in obj && obj[k] !== undefined && obj[k] !== null) {
      lines.push(`${k}: ${formatValue(obj[k])}`)
      seen.add(k)
    }
  }
  for (const [k, v] of Object.entries(obj)) {
    if (seen.has(k) || v === undefined || v === null) continue
    lines.push(`${k}: ${formatValue(v)}`)
  }
  return lines.join('\n') + '\n'
}

function formatValue(v) {
  if (typeof v === 'number') return String(v)
  const s = String(v)
  if (/[:#"\n]/.test(s)) return JSON.stringify(s)
  return s
}

function parseSections(body) {
  const sections = emptySections()
  let current = null
  let buf = []
  const flush = () => {
    if (current) sections[current] = buf.join('\n').trim()
    buf = []
  }
  for (const line of body.split('\n')) {
    if (SECTIONS.includes(line.trim())) {
      flush()
      current = line.trim()
    } else if (current) {
      buf.push(line)
    }
  }
  flush()
  return sections
}

function serializeSections(sections) {
  return SECTIONS.map((h) => {
    const body = sections[h]?.trim() ?? ''
    return body ? `${h}\n${body}` : `${h}\n`
  }).join('\n\n')
}

export async function appendArtifact(sections, line) {
  const cur = sections['## Artifacts'].trim()
  const lines = cur ? cur.split('\n') : []
  if (!lines.includes(line)) lines.push(line)
  sections['## Artifacts'] = lines.join('\n')
}

export function todosToMarkdown(todos) {
  if (!Array.isArray(todos)) return ''
  return todos
    .map((t) => {
      const text = t.subject ?? t.content ?? t.text ?? ''
      const status = t.status ?? 'pending'
      const checkbox = status === 'completed' ? '[x]' : '[ ]'
      const marker = status === 'in_progress' ? '  ← current' : ''
      return `- ${checkbox} ${text}${marker}`
    })
    .join('\n')
}

export function summarizeTodos(todos) {
  if (!Array.isArray(todos)) return { total: 0, completed: 0, current: undefined }
  const total = todos.length
  const completed = todos.filter((t) => t.status === 'completed').length
  const inprog = todos.find((t) => t.status === 'in_progress')
  return {
    total,
    completed,
    current: inprog ? (inprog.subject ?? inprog.content ?? inprog.text) : undefined
  }
}

export async function withFileLock(sessionId, fn) {
  const lock = trackingPath(sessionId) + '.lock'
  await mkdir(dirname(lock), { recursive: true })
  const start = Date.now()
  while (Date.now() - start < 5000) {
    try {
      const { open } = await import('node:fs/promises')
      const fh = await open(lock, 'wx')
      await fh.close()
      try {
        return await fn()
      } finally {
        try {
          const { unlink } = await import('node:fs/promises')
          await unlink(lock)
        } catch {
          // ignore
        }
      }
    } catch (err) {
      if (err && err.code === 'EEXIST') {
        try {
          const s = await stat(lock)
          if (Date.now() - s.mtimeMs > 10_000) {
            const { unlink } = await import('node:fs/promises')
            await unlink(lock).catch(() => {})
          }
        } catch {
          // ignore
        }
        await new Promise((r) => setTimeout(r, 30))
        continue
      }
      throw err
    }
  }
  // Lock contention timeout — proceed without lock as a fallback.
  return await fn()
}
