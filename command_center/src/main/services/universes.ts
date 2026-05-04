import { readdir, readFile, stat } from 'node:fs/promises'
import { join } from 'node:path'
import chokidar, { type FSWatcher } from 'chokidar'
import yaml from 'js-yaml'
import { RICK_UNIVERSES } from './paths'
import type { Universe, Workflow, WorkflowParam, WorkflowStep } from '@shared/types'

type Listener = (universes: Universe[], workflows: Workflow[]) => void

export class UniverseService {
  private universes: Universe[] = []
  private workflows: Workflow[] = []
  private watcher: FSWatcher | null = null
  private listeners = new Set<Listener>()
  private rescanTimer: NodeJS.Timeout | null = null

  async start(): Promise<void> {
    await this.scan()
    this.watcher = chokidar.watch(RICK_UNIVERSES, {
      ignoreInitial: true,
      depth: 4,
      ignored: (p) => p.includes('/.git/') || p.endsWith('.DS_Store')
    })
    const schedule = (): void => {
      if (this.rescanTimer) clearTimeout(this.rescanTimer)
      this.rescanTimer = setTimeout(() => void this.scan(), 250)
    }
    this.watcher.on('add', schedule).on('change', schedule).on('unlink', schedule).on('addDir', schedule).on('unlinkDir', schedule)
  }

  stop(): void {
    void this.watcher?.close()
    this.watcher = null
    if (this.rescanTimer) clearTimeout(this.rescanTimer)
  }

  onChange(cb: Listener): () => void {
    this.listeners.add(cb)
    return () => this.listeners.delete(cb)
  }

  snapshot(): { universes: Universe[]; workflows: Workflow[] } {
    return { universes: this.universes, workflows: this.workflows }
  }

  private async scan(): Promise<void> {
    const universes: Universe[] = []
    const workflows: Workflow[] = []

    let entries: string[] = []
    try {
      entries = await readdir(RICK_UNIVERSES)
    } catch {
      this.universes = []
      this.workflows = []
      this.emit()
      return
    }

    for (const name of entries) {
      if (name.startsWith('.')) continue
      const path = join(RICK_UNIVERSES, name)
      try {
        const s = await stat(path)
        if (!s.isDirectory()) continue
      } catch {
        continue
      }
      universes.push({ name, path })

      const wfDir = join(path, 'workflows')
      let files: string[] = []
      try {
        files = await readdir(wfDir)
      } catch {
        continue
      }
      for (const f of files) {
        if (!f.endsWith('.yaml') && !f.endsWith('.yml')) continue
        const filePath = join(wfDir, f)
        try {
          const raw = await readFile(filePath, 'utf8')
          const parsed = yaml.load(raw) as Record<string, unknown> | null
          workflows.push(toWorkflow(name, filePath, parsed))
        } catch {
          // Skip malformed YAML; surfaced via toast in renderer eventually.
        }
      }
    }

    this.universes = universes.sort((a, b) => a.name.localeCompare(b.name))
    this.workflows = workflows
    this.emit()
  }

  private emit(): void {
    for (const l of this.listeners) l(this.universes, this.workflows)
  }
}

function toWorkflow(universe: string, filePath: string, doc: Record<string, unknown> | null): Workflow {
  const name = (doc?.name as string) || filePath.split('/').pop()!.replace(/\.ya?ml$/, '')
  const description = doc?.description as string | undefined
  const rawSteps = Array.isArray(doc?.steps) ? (doc?.steps as Record<string, unknown>[]) : []

  const steps: WorkflowStep[] = rawSteps.map((s, i) => ({
    id: typeof s.id === 'string' ? s.id : `step-${i + 1}`,
    agent: typeof s.agent === 'string' ? s.agent : typeof s.uses === 'string' ? (s.uses as string) : 'unknown',
    collaborators: Array.isArray(s.collaborators) ? (s.collaborators as string[]) : undefined,
    description: typeof s.description === 'string' ? s.description : undefined,
    dependsOn: Array.isArray(s.depends_on) ? (s.depends_on as string[]) : undefined,
    uses: typeof s.uses === 'string' ? s.uses : undefined
  }))

  const agents = Array.from(new Set(steps.map((s) => s.agent).filter((a) => a && a !== 'unknown')))
  const dependsOn = Array.from(new Set(steps.flatMap((s) => s.dependsOn ?? [])))
  const paramsBlock = (doc?.params as Record<string, unknown>) ?? {}
  const params: WorkflowParam[] = Object.entries(paramsBlock).map(([pname, raw]) => normalizeParam(pname, raw))

  return { name, universe, filePath, description, agents, dependsOn, params, steps }
}

function normalizeParam(name: string, raw: unknown): WorkflowParam {
  if (raw && typeof raw === 'object' && !Array.isArray(raw)) {
    const r = raw as Record<string, unknown>
    const type = (r.type as WorkflowParam['type']) ?? inferType(r.default)
    return {
      name,
      type,
      default: r.default,
      description: r.description as string | undefined,
      required: r.required === true,
      enumValues: Array.isArray(r.enum) ? (r.enum as string[]) : undefined
    }
  }
  return { name, type: inferType(raw), default: raw }
}

function inferType(v: unknown): WorkflowParam['type'] {
  if (typeof v === 'boolean') return 'bool'
  if (typeof v === 'number') return Number.isInteger(v) ? 'int' : 'unknown'
  if (typeof v === 'string') return 'string'
  return 'unknown'
}
