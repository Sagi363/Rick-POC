# Launch Modal -- Presentation Spec

## 1. Overview

Modal opened when the user clicks a workflow card. Collects the parameters needed to start a `/rick run …` session, with sensible defaults plus an optional **worktree** mode for git-isolated workflows. Optimized so a 3-click launch is possible when defaults are good, while every YAML-declared knob is still surfaced.

**Source files:**
- `src/renderer/src/components/LaunchModal.tsx` (lines 19-339) -- Modal component, state, submit
- `src/renderer/src/components/LaunchModal.tsx` (lines 341-400) -- ParamInput rendering
- `src/renderer/src/components/LaunchModal.tsx` (lines 477-498) -- localStorage helpers
- `src/main/services/launcher.ts` -- launchWorkflow IPC handler, prompt builder, terminal spawn, git worktree
- `src/main/services/launcher.ts` (lines 155-161) -- buildPrompt
- `src/main/services/launcher.ts` (lines 116-153) -- worktree creation
- `src/shared/types.ts` -- LaunchRequest, WorkflowParam, WorktreeRequest

---

## 2. Presentation Models

### 2.1 LaunchModalState

Local state held by the modal. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `mode` | "folder" \| "worktree" | No | `"folder"` | dynamic -- pill toggle at top |
| `cwd` | String | No | `"/Users/.../foo"` | dynamic -- recent or browse-picked |
| `values` | Record<paramName, unknown> | No | `{ticket_key:"SCCOM-33084"}` | dynamic -- one per declared YAML param |
| `extraPrompt` | String | No | `""` | Free-text appended to /rick run line |
| `autoContinue` | Bool | No | `false` | dynamic -- restored from `localStorage[rcc:launch:auto-continue]` |
| `worktreeName` | String | No | `""` | dynamic -- defaults to `suggestion.name` |
| `worktreeBranch` | String | No | `""` | dynamic -- defaults to `suggestion.branch` |
| `worktreeFrom` | String | No | `"dev"` | dynamic -- from `defaultBranchOff` setting |
| `busy` | Bool | No | `false` | True while submit is in flight |
| `error` | String | Yes | -- | Error message after a failed submit |
| `recoverPath` | String | Yes | -- | Set when worktree path already exists |

### 2.2 WorktreeRequest

Sent in the `LaunchRequest.worktree` field. Conforms to: Sendable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `base` | String | No | `"/Users/.../my-project"` | The repo root |
| `name` | String | No | `"sccom-33084"` | Becomes `<base>/.claude/worktrees/<name>` |
| `branch` | String | No | `"feature/sccom-33084"` | New branch to create |
| `fromBranch` | String | Yes | `"dev"` | Starting commit; blank uses HEAD |

### 2.3 ParamSuggestion

Auto-derived suggestion for worktree name + branch. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `name` | String | No | `"sccom-33084"` | dynamic -- slugified from first identifying param |
| `branch` | String | No | `"feature/sccom-33084"` | dynamic -- `<branchPrefix><slug>` |

### 2.4 LaunchResult

The return shape of `launchWorkflow`. Conforms to: structural-only.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `ok` | Bool | No | `true` | False on any failure |
| `error` | String | Yes | `"git worktree add failed: …"` | Present when `ok === false` |
| `command` | String | Yes | `"/rick run …"` | The composed prompt that was sent |
| `existingWorktreePath` | String | Yes | `"/Users/.../foo/.claude/worktrees/sccom"` | Set when failure was "worktree exists" |

---

## 3. Visual States

### 3.1 Mode toggle (pill)

| State | Pill |
|---|---|
| folder | "Use folder" highlighted, "Create worktree" subdued |
| worktree | "Create worktree" highlighted, "Use folder" subdued |

### 3.2 Cwd field

Three input affordances:
- Recent chips row (top 10, click to fill).
- Free-text input with `<datalist>` typeahead.
- "Browse…" button → OS folder picker.

### 3.3 Param form

| Param type | Input |
|---|---|
| string | Text input |
| int | Number input |
| bool | Checkbox |
| With `enumValues` | `<select>` with "— choose —" placeholder |
| unknown | Text input (graceful fallback) |

Required params show a `*` next to the label and tint rose when missing.

### 3.4 Worktree fields (only shown when `mode === 'worktree'`)

| Field | Default | Notes |
|---|---|---|
| Worktree name | Auto from `feature` / `ticket_key` / `ticket` / `job` (slugified) | Editable; blank uses suggestion |
| Branch | `<branchPrefix><slug>` (e.g. `feature/sccom-33084`) | New branch via `git worktree add -b` |
| Branch off | `defaultBranchOff` (default `dev`) | Blank uses current HEAD |

### 3.5 Auto-continue toggle

Checkbox + emerald/zinc pill showing `auto_continue: true|false`. Default = last user choice.

### 3.6 Extra prompt

Free-text textarea, 3 rows, placeholder "Anything else you want to tell Claude before it starts…".

### 3.7 Error states

| Sub-state | Appearance |
|---|---|
| Generic error | Rose-bordered banner with message, no action |
| Worktree exists | Same banner + emerald "Use it & launch →" button + the path inline |

### 3.8 Footer

Summary line of what's about to happen + Cancel / Launch buttons.

| Mode | Summary text |
|---|---|
| folder | `Will open Terminal.app in <cwd>` |
| worktree | `Will git worktree add <name> on <branch>, then open Terminal there` |

The Launch button label is dynamic: `Launching…` when busy, otherwise `Launch in Terminal`.

---

## 4. Interactions

| Target | Gesture | Result |
|---|---|---|
| Mode pill | Click | Switches between folder / worktree |
| Recent cwd chip | Click | Fills the cwd input |
| Browse… | Click | OS directory picker via `pickDirectory(cwd)` |
| Cwd input | Type | Updates state; datalist suggests recent values |
| Param input | Change | Updates `values[paramName]` |
| Auto-continue checkbox | Click | Flips state; will be persisted to localStorage on submit |
| Worktree name input | Type | Overrides suggestion |
| Branch input | Type | Overrides suggestion |
| Branch off input | Type | Overrides default |
| Extra prompt textarea | Type | Appended to submit |
| Cancel | Click | Closes without writing |
| Launch | Click (when `canSubmit`) | Submits via launcher |
| "Use it & launch →" | Click (on worktree-exists error) | Re-submits using `recoverPath` as cwd, no worktree creation |

---

## 5. Submit Flow

```
User clicks Launch (canSubmit === true)
  |
  +-- composedExtra = buildAutoContinueDirective(autoContinue, extraPrompt.trim())
  |     |
  |     +-- (autoContinue === true) prepends:
  |           "Override: run all phases with auto_continue: true …"
  |
  +-- launchWorkflow({workflow, universe, cwd, params, extraPrompt: composedExtra, worktree?})
        |
        +-- (worktree mode) git worktree add <base>/.claude/worktrees/<name> -b <branch> [from]
        |     +-- on EEXIST -> return { ok:false, error, existingWorktreePath: <path> }
        |
        +-- prompt = "/rick run <name> --params='<json>'\n<extra>"
        +-- spawn the chosen terminal:
        |     - in-app          -> PtyService.spawn(...) + ptyWrite(prompt)
        |     - Terminal/iTerm  -> AppleScript `tell application … do script`
        |     - Warp/Ghostty    -> open -a + clipboard copy
        |     - custom          -> /bin/sh -c <template substituted>
        |
        +-- return { ok:true, command }

On success:
  - saveRecentCwd(workflow.name, cwd)
  - saveAutoContinue(autoContinue)
  - onLaunched(command)
  - onClose()

On failure (error set):
  - Render error banner inline
  - If existingWorktreePath set: render "Use it & launch →" recovery button
```

---

## 6. Recovery: Worktree Already Exists

When the chosen worktree path is already on disk (a previous run created it), `git worktree add` would fail. The launcher returns `{ ok:false, error, existingWorktreePath }` and the modal renders an emerald "Use it & launch →" button. Clicking it skips `git worktree add` and just spawns the launcher in the existing path.

Catches the common "I closed the terminal, came back, want to resume" case.

---

## 7. Auto-continue Directive

This is a launch-time-only directive. Mid-run flips happen via the [session card pill](../sessions-panel/spec.md#auto-continue-pill) which sends `/btw …` directives to the running PTY.

The launch-time directive prepended to the user's extra prompt:

> `Override: run all phases with auto_continue: true — do not pause between phases or wait for me to say next. Drive the workflow end-to-end.`

When `autoContinue === false`, nothing is prepended (workflow YAML's per-step `auto_continue:` flags govern).

---

## Open Questions

- OQ1: Should auto-continue persistence be per-workflow, not global? Currently global — all workflows share one default.
- OQ2: Should the worktree mode be remembered per-workflow, not global?
- OQ3: When `terminalApp = 'in-app'` and the launch fails, should the modal stay open (it does) and is there a way to retry without re-typing? Currently retry works because state is preserved.
- OQ4: Should we offer to commit the launch-time auto-continue ON state into the workflow YAML for next time?
