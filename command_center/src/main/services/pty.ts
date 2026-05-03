import { spawn as ptySpawn, type IPty } from 'node-pty'
import { EventEmitter } from 'node:events'
import { homedir } from 'node:os'

export interface SpawnPtyOptions {
  cwd: string
  command?: string
  args?: string[]
  env?: Record<string, string>
  cols?: number
  rows?: number
  sessionId?: string
  initialInput?: string
  label?: string
}

export interface PtyHandle {
  id: string
  cwd: string
  label: string
  pid: number
  startedAt: number
  alive: boolean
  exitCode?: number
  sessionId?: string
}

interface InternalEntry extends PtyHandle {
  pty: IPty
  buffer: string[]
}

const MAX_BUFFER_LINES = 5000

export class PtyService extends EventEmitter {
  private terminals = new Map<string, InternalEntry>()
  private idCounter = 0

  list(): PtyHandle[] {
    return Array.from(this.terminals.values()).map(toHandle)
  }

  get(id: string): PtyHandle | undefined {
    const entry = this.terminals.get(id)
    return entry ? toHandle(entry) : undefined
  }

  buffer(id: string): string {
    return this.terminals.get(id)?.buffer.join('') ?? ''
  }

  spawn(opts: SpawnPtyOptions): PtyHandle {
    const id = `pty-${++this.idCounter}-${Date.now().toString(36)}`
    const shell = process.env.SHELL || '/bin/zsh'
    const command = opts.command ?? shell
    const args =
      opts.args ??
      (opts.command
        ? []
        : ['-l'])
    const env = {
      ...process.env,
      ...opts.env,
      TERM: opts.env?.TERM ?? 'xterm-256color',
      LANG: opts.env?.LANG ?? process.env.LANG ?? 'en_US.UTF-8'
    } as Record<string, string>

    const pty = ptySpawn(command, args, {
      cwd: opts.cwd || homedir(),
      cols: opts.cols ?? 120,
      rows: opts.rows ?? 32,
      env,
      name: 'xterm-256color'
    })

    const entry: InternalEntry = {
      id,
      cwd: opts.cwd,
      label: opts.label ?? `${command} ${args.join(' ')}`.trim(),
      pid: pty.pid,
      startedAt: Date.now(),
      alive: true,
      sessionId: opts.sessionId,
      pty,
      buffer: []
    }
    this.terminals.set(id, entry)

    pty.onData((data) => {
      entry.buffer.push(data)
      if (entry.buffer.length > MAX_BUFFER_LINES) {
        entry.buffer.splice(0, entry.buffer.length - MAX_BUFFER_LINES)
      }
      this.emit('data', { id, data })
    })

    pty.onExit(({ exitCode }) => {
      entry.alive = false
      entry.exitCode = exitCode
      this.emit('exit', { id, exitCode })
      this.emit('list', this.list())
    })

    if (opts.initialInput) {
      // Give the shell a moment to print its prompt before injecting input.
      setTimeout(() => {
        try {
          pty.write(opts.initialInput!)
        } catch {
          // child may have already exited
        }
      }, 80)
    }

    this.emit('list', this.list())
    return toHandle(entry)
  }

  write(id: string, data: string): void {
    const entry = this.terminals.get(id)
    if (!entry?.alive) return
    try {
      entry.pty.write(data)
    } catch {
      // ignore — child may be exiting
    }
  }

  resize(id: string, cols: number, rows: number): void {
    const entry = this.terminals.get(id)
    if (!entry?.alive) return
    try {
      entry.pty.resize(Math.max(1, cols | 0), Math.max(1, rows | 0))
    } catch {
      // ignore
    }
  }

  kill(id: string, signal: string = 'SIGHUP'): void {
    const entry = this.terminals.get(id)
    if (!entry) return
    try {
      entry.pty.kill(signal)
    } catch {
      // ignore
    }
    this.terminals.delete(id)
    this.emit('list', this.list())
  }

  bindSession(id: string, sessionId: string): void {
    const entry = this.terminals.get(id)
    if (!entry) return
    entry.sessionId = sessionId
    this.emit('list', this.list())
  }

  findBySession(sessionId: string): PtyHandle | undefined {
    for (const entry of this.terminals.values()) {
      if (entry.sessionId === sessionId) return toHandle(entry)
    }
    return undefined
  }

  shutdown(): void {
    for (const id of Array.from(this.terminals.keys())) this.kill(id)
  }
}

function toHandle(entry: InternalEntry): PtyHandle {
  return {
    id: entry.id,
    cwd: entry.cwd,
    label: entry.label,
    pid: entry.pid,
    startedAt: entry.startedAt,
    alive: entry.alive,
    exitCode: entry.exitCode,
    sessionId: entry.sessionId
  }
}
