import { readdir, stat } from 'node:fs/promises'
import { join, relative } from 'node:path'

const SKIP_DIRS = new Set([
  'node_modules',
  '.git',
  '.next',
  'dist',
  'build',
  'out',
  'release',
  'Pods',
  'DerivedData',
  '.gradle',
  '.idea',
  '.venv',
  'venv',
  '__pycache__',
  '.cache',
  'coverage'
])

// Directories where any markdown/yaml inside is treated as a spec artifact.
const SPEC_DIRS = [
  'specs',
  'spec',
  '.spec',
  '.specs',
  '.claude/specs',
  '.rick/specs',
  'docs/specs',
  '.spec-architect',
  '.ralph-specum'
]

// Filenames that count as a spec artifact even when they live at project root.
const SPEC_FILE_NAMES = new Set([
  'PRD.md',
  'prd.md',
  'requirements.md',
  'requirements.yaml',
  'requirements.yml',
  'design.md',
  'design.yaml',
  'tasks.md',
  'tasks.yaml',
  'research.md',
  'acceptance.md',
  'acceptance-criteria.md',
  'spec.md'
])

// Filename suffix patterns that mark spec artifacts anywhere.
const SPEC_FILE_PATTERNS = [
  /\.spec\.md$/i,
  /\.spec\.ya?ml$/i,
  /\.requirements\.md$/i,
  /\.design\.md$/i
]

export interface SpecFinder {
  cwd: string
  maxResults?: number
}

export async function findSpecFiles(opts: SpecFinder): Promise<string[]> {
  const { cwd } = opts
  const maxResults = opts.maxResults ?? 80
  if (!cwd) return []
  const results = new Map<string, { priority: number; mtime: number }>()

  // Pass 1: spec-named files at cwd root.
  await scanRootSpecFiles(cwd, results, maxResults)

  // Pass 2: walk known spec directories (any depth, any md/yaml inside).
  for (const rel of SPEC_DIRS) {
    if (results.size >= maxResults) break
    const full = join(cwd, rel)
    await walkSpecDir(full, results, maxResults)
  }

  return Array.from(results.entries())
    .sort((a, b) => b[1].priority - a[1].priority || b[1].mtime - a[1].mtime)
    .map(([path]) => path)
}

async function scanRootSpecFiles(
  cwd: string,
  out: Map<string, { priority: number; mtime: number }>,
  cap: number
): Promise<void> {
  let entries: string[]
  try {
    entries = await readdir(cwd)
  } catch {
    return
  }
  for (const name of entries) {
    if (out.size >= cap) return
    if (!isSpecFileName(name)) continue
    const full = join(cwd, name)
    let s
    try {
      s = await stat(full)
    } catch {
      continue
    }
    if (!s.isFile()) continue
    out.set(full, {
      priority: SPEC_FILE_NAMES.has(name) ? 100 : 50,
      mtime: s.mtimeMs
    })
  }
}

async function walkSpecDir(
  dir: string,
  out: Map<string, { priority: number; mtime: number }>,
  cap: number,
  depth = 0
): Promise<void> {
  if (out.size >= cap || depth > 4) return
  let entries: string[]
  try {
    entries = await readdir(dir)
  } catch {
    return
  }
  for (const name of entries) {
    if (out.size >= cap) return
    if (SKIP_DIRS.has(name)) continue
    const full = join(dir, name)
    let s
    try {
      s = await stat(full)
    } catch {
      continue
    }
    if (s.isDirectory()) {
      await walkSpecDir(full, out, cap, depth + 1)
      continue
    }
    if (!isMarkdownOrYaml(name)) continue
    if (out.has(full)) continue
    out.set(full, {
      priority: 80 - depth * 5,
      mtime: s.mtimeMs
    })
  }
}

function isMarkdownOrYaml(name: string): boolean {
  const lower = name.toLowerCase()
  return (
    lower.endsWith('.md') ||
    lower.endsWith('.markdown') ||
    lower.endsWith('.yaml') ||
    lower.endsWith('.yml')
  )
}

function isSpecFileName(name: string): boolean {
  if (SPEC_FILE_NAMES.has(name)) return true
  return SPEC_FILE_PATTERNS.some((re) => re.test(name))
}

export function shortPath(absolute: string, cwd: string): string {
  if (!cwd) return absolute
  const rel = relative(cwd, absolute)
  if (rel && !rel.startsWith('..')) return rel
  return absolute
}
