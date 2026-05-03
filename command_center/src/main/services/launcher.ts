import { spawn } from 'node:child_process'
import { access, mkdtemp, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type {
  FocusTerminalRequest,
  FocusTerminalResult,
  LaunchRequest,
  LaunchResult,
  TerminalApp,
  WorktreeRequest
} from '@shared/types'

export class WorkflowLauncher {
  async prepareCwd(
    req: LaunchRequest
  ): Promise<{ ok: true; cwd: string } | { ok: false; error: string; existingWorktreePath?: string }> {
    let cwd = req.cwd
    if (req.worktree) {
      const wtPath = await this.createWorktree(req.worktree)
      if ('error' in wtPath) {
        return { ok: false, error: wtPath.error, existingWorktreePath: wtPath.existingPath }
      }
      cwd = wtPath.path
    }
    if (!cwd) return { ok: false, error: 'cwd is required' }
    return { ok: true, cwd }
  }

  async launch(req: LaunchRequest): Promise<LaunchResult> {
    if (!req.workflow) return { ok: false, error: 'workflow is required' }

    const prep = await this.prepareCwd(req)
    if (!prep.ok) {
      return { ok: false, error: prep.error, existingWorktreePath: prep.existingWorktreePath }
    }
    const cwd = prep.cwd

    const command = this.buildPrompt(req)
    const terminalApp: TerminalApp = req.terminalApp ?? 'Terminal'

    let promptDir: string | null = null
    try {
      promptDir = await mkdtemp(join(tmpdir(), 'rcc-'))
      const promptFile = join(promptDir, 'prompt.txt')
      await writeFile(promptFile, command, 'utf8')

      const flags = req.skipPermissions ? ' --dangerously-skip-permissions' : ''
      const shellLine = `cd ${shellQuote(cwd)} && claude${flags} "$(< ${shellQuote(promptFile)})"; status=$?; rm -f ${shellQuote(promptFile)}; rmdir ${shellQuote(promptDir)} 2>/dev/null; exit $status`

      await this.invokeTerminal(terminalApp, cwd, shellLine, req.customTerminalCommand)
      promptDir = null
      return { ok: true, command }
    } catch (e) {
      if (promptDir) {
        try {
          await rm(promptDir, { recursive: true, force: true })
        } catch {
          // ignore
        }
      }
      return { ok: false, error: (e as Error).message }
    }
  }

  async focus(req: FocusTerminalRequest): Promise<FocusTerminalResult> {
    try {
      // Try to focus an existing window/tab first (Terminal & iTerm only).
      let found = false
      if (req.terminalApp === 'Terminal') {
        const out = await this.runOsascriptCapture(focusTerminalAppScript(req.cwd))
        found = out.includes('RCC_FOUND')
      } else if (req.terminalApp === 'iTerm') {
        const out = await this.runOsascriptCapture(focusITermScript(req.cwd))
        found = out.includes('RCC_FOUND')
      }
      if (found) return { ok: true }

      // Fallback: spawn a fresh tab/window running `claude --resume <id>`.
      const flags = req.skipPermissions ? ' --dangerously-skip-permissions' : ''
      const shellLine = `cd ${shellQuote(req.cwd)} && claude${flags} --resume ${shellQuote(req.sessionId)}`
      await this.invokeTerminal(req.terminalApp, req.cwd, shellLine, req.customTerminalCommand)
      return { ok: true, resumed: true }
    } catch (e) {
      return { ok: false, error: (e as Error).message }
    }
  }

  private async invokeTerminal(
    app: TerminalApp,
    cwd: string,
    shellLine: string,
    customTemplate: string | undefined
  ): Promise<void> {
    switch (app) {
      case 'Terminal':
        await this.runOsascript(launchTerminalAppScript(shellLine))
        return
      case 'iTerm':
        await this.runOsascript(launchITermScript(shellLine))
        return
      case 'Warp':
        await this.runCommand('open', ['-na', 'Warp', '--args', '--working-directory', cwd])
        // Warp does not accept stdin; user runs the command manually after the window opens.
        // Fallback: copy the shell line to clipboard so the user can paste.
        await this.copyToClipboard(shellLine)
        return
      case 'Ghostty':
        await this.runCommand('open', ['-na', 'Ghostty.app', '--args', `--working-directory=${cwd}`])
        await this.copyToClipboard(shellLine)
        return
      case 'custom': {
        if (!customTemplate) throw new Error('customTerminalCommand is empty in settings')
        const expanded = customTemplate
          .replaceAll('%cwd%', shellQuote(cwd))
          .replaceAll('%cmd%', shellQuote(shellLine))
        await this.runShell(expanded)
        return
      }
      default:
        throw new Error(`unsupported terminal app: ${app}`)
    }
  }

  private async createWorktree(
    wt: WorktreeRequest
  ): Promise<{ path: string } | { error: string; existingPath?: string }> {
    if (!wt.base) return { error: 'worktree base is required' }
    if (!wt.name || !/^[A-Za-z0-9._-][A-Za-z0-9._/-]*$/.test(wt.name)) {
      return { error: 'invalid worktree name' }
    }
    if (!wt.branch || !/^[A-Za-z0-9._/-]+$/.test(wt.branch)) {
      return { error: 'invalid branch name' }
    }

    const path = join(wt.base, '.claude', 'worktrees', wt.name)
    try {
      await access(path)
      return { error: `worktree path already exists: ${path}`, existingPath: path }
    } catch {
      // expected — path should not exist
    }

    const args = ['-C', wt.base, 'worktree', 'add', path, '-b', wt.branch]
    if (wt.fromBranch) args.push(wt.fromBranch)

    try {
      await this.runCommand('git', args)
      return { path }
    } catch (e) {
      return { error: `git worktree add failed: ${(e as Error).message}` }
    }
  }

  private buildPrompt(req: LaunchRequest): string {
    const params = pruneEmpty(req.params)
    const paramsArg = Object.keys(params).length > 0 ? ` --params=${shellSingle(JSON.stringify(params))}` : ''
    const head = `/rick run ${req.workflow}${paramsArg}`
    const extra = req.extraPrompt?.trim()
    return extra ? `${head}\n\n${extra}\n` : `${head}\n`
  }

  private runOsascript(script: string): Promise<void> {
    return this.runCommand('osascript', ['-e', script]).then(() => undefined)
  }

  private runOsascriptCapture(script: string): Promise<string> {
    return this.runCommand('osascript', ['-e', script])
  }

  private runCommand(cmd: string, args: string[]): Promise<string> {
    return runCommand(cmd, args)
  }

  private runShell(line: string): Promise<string> {
    return runCommand('/bin/sh', ['-c', line])
  }

  private async copyToClipboard(text: string): Promise<void> {
    try {
      await new Promise<void>((resolve, reject) => {
        const child = spawn('pbcopy')
        child.on('error', reject)
        child.on('close', (code) => (code === 0 ? resolve() : reject(new Error(`pbcopy ${code}`))))
        child.stdin.write(text)
        child.stdin.end()
      })
    } catch {
      // ignore — clipboard is best-effort
    }
  }
}

function runCommand(cmd: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn(cmd, args, { stdio: ['ignore', 'pipe', 'pipe'] })
    let stdout = ''
    let stderr = ''
    child.stdout.on('data', (b: Buffer) => (stdout += b.toString('utf8')))
    child.stderr.on('data', (b: Buffer) => (stderr += b.toString('utf8')))
    child.on('error', reject)
    child.on('close', (code) => {
      if (code === 0) resolve(stdout)
      else reject(new Error(stderr.trim() || stdout.trim() || `${cmd} exited ${code}`))
    })
  })
}

function appBinary(app: TerminalApp): string {
  switch (app) {
    case 'Terminal':
      return 'Terminal'
    case 'iTerm':
      return 'iTerm'
    case 'Warp':
      return 'Warp'
    case 'Ghostty':
      return 'Ghostty'
    default:
      return 'Terminal'
  }
}

function pruneEmpty(obj: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const [k, v] of Object.entries(obj)) {
    if (v === undefined || v === null || v === '') continue
    out[k] = v
  }
  return out
}

function shellQuote(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`
}

function shellSingle(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`
}

function appleScriptQuote(s: string): string {
  return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`
}

function launchTerminalAppScript(shellLine: string): string {
  return `tell application "Terminal"
  activate
  do script ${appleScriptQuote(shellLine)}
end tell`
}

function launchITermScript(shellLine: string): string {
  return `tell application "iTerm"
  activate
  if (count of windows) = 0 then
    create window with default profile
  else
    tell current window to create tab with default profile
  end if
  tell current session of current window
    write text ${appleScriptQuote(shellLine)}
  end tell
end tell`
}

function focusTerminalAppScript(cwd: string): string {
  const needle = appleScriptQuote(cwd)
  return `tell application "Terminal"
  set found to false
  set wins to every window
  repeat with w in wins
    set tabs_ to every tab of w
    repeat with t in tabs_
      try
        if (custom title of t contains ${needle}) or (name of t contains ${needle}) or (path of (processes of t) contains ${needle}) then
          activate
          set frontmost of w to true
          set selected tab of w to t
          set found to true
          exit repeat
        end if
      end try
    end repeat
    if found then exit repeat
  end repeat
  if found then
    return "RCC_FOUND"
  else
    return "RCC_MISSING"
  end if
end tell`
}

function focusITermScript(cwd: string): string {
  const needle = appleScriptQuote(cwd)
  return `tell application "iTerm"
  set found to false
  set ws to windows
  repeat with w in ws
    set ts to tabs of w
    repeat with t in ts
      set ss to sessions of t
      repeat with s in ss
        try
          if (name of s contains ${needle}) or (path of s contains ${needle}) or (variable named "session.path" of s contains ${needle}) then
            activate
            select w
            tell w to select t
            set found to true
            exit repeat
          end if
        end try
      end repeat
      if found then exit repeat
    end repeat
    if found then exit repeat
  end repeat
  if found then
    return "RCC_FOUND"
  else
    return "RCC_MISSING"
  end if
end tell`
}
