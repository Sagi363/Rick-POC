# Terminal -- Presentation Spec

## 1. Overview

The bottom-strip panel in the main pane that hosts an embedded `xterm.js` terminal backed by `node-pty`. Lets the user run and watch a Claude Code session inside the app, with per-session tabs, quick-command buttons (`/rick next`, `/rick status`, `/clear`), and the in-app branch of the terminal-app picker.

When no PTY is alive for the selected session, this panel hides and the [Progress / Activity Panel](../progress-panel/spec.md) takes its slot. Conversely, when ≥ 1 PTY is alive, the Activity panel is hidden -- they share the same vertical real estate and never coexist.

**Source files:**
- `src/renderer/src/components/TerminalsPanel.tsx` (lines 1-180) -- Panel, tabs, quick-cmd toolbar, confirm modal
- `src/renderer/src/components/Terminal.tsx` (full file) -- xterm.js binding to a single PTY
- `src/main/services/pty.ts` (full file) -- Spawn / write / resize / kill / bind by cwd
- `src/main/handlers.ts` (lines 65-77) -- Auto-bind unbound PTYs to new sessions by cwd match
- `src/main/services/launcher.ts` -- focusTerminal action
- `src/shared/types.ts` (PtyInfo, TerminalApp)

---

## 2. Presentation Models

### 2.1 PtyInfo

The shape pushed from main to renderer for each managed PTY. Conforms to: structural-only.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `id` | String | No | `"pty-1"` | Internal id assigned by `PtyService` |
| `sessionId` | String | Yes | `"4cc80b31-..."` | Bound to a Claude session id; null until auto-bound by cwd |
| `cwd` | String | No | `"/Users/.../foo"` | Working directory the PTY was spawned in |
| `label` | String | No | `"4cc80b31"` | Human-readable; typically session id short or workflow name |
| `alive` | Bool | No | `true` | dynamic -- false after the child process exits |

### 2.2 TerminalApp (enumeration)

| Case | Source | Notes |
|---|---|---|
| `in-app` | `xterm.js` + `node-pty` inside this panel | Default behavior is FR-15/16: fresh PTY per launch |
| `Terminal` | macOS Terminal.app via AppleScript | Inline launch + window focus |
| `iTerm` | iTerm2 via AppleScript | Inline launch + window focus |
| `Warp` | Open + clipboard | AppleScript can't drive; user pastes |
| `Ghostty` | Open + clipboard | Same as Warp |
| `custom` | User-defined command template | `%cwd%` and `%cmd%` placeholders, runs in `/bin/sh` |

### 2.3 QuickCmd

One toolbar button. Conforms to: Sendable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `label` | String | No | `"/rick next"` | Visible button text (the user-facing concept) |
| `tone` | "emerald" \| "zinc" \| "amber" | No | `"emerald"` | Color category -- emerald primary, zinc neutral, amber risky |
| `disabled` | Bool | No | `false` | dynamic -- per-button gating |
| `disabledReason` | String | Yes | `"Rick is running …"` | dynamic -- specific tooltip when disabled |
| `onClick` | () => void | No | -- | Invoked when enabled and clicked |

### 2.4 ConfirmClearModal

Inline overlay shown when the user clicks `/clear`. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `visible` | Bool | No | `false` | Local panel state `confirmingClear` |
| `tone` | String | No | `"amber"` | Static -- always amber (warning, not destructive-red) |
| `body` | Text | No | (see 3.4) | Copy explaining context loss + successor-detection |

---

## 3. Visual States

### 3.1 Panel collapsed

| State | Strip content | Strip height |
|---|---|---|
| Hidden | `Terminal hidden  · N active`  ⌃ Show button | One line, `~36px` |

### 3.2 Panel expanded

```
┌────────────────────────────────────────────────────┐
│ TERMINAL  [● 4cc80b31] [● 466034a2]  ▶ /rick next  │  ← header
│           ────────────              ▶ /rick status │
│                                     ▶ /clear       │
│                                            ▼ Hide  │
├────────────────────────────────────────────────────┤
│                                                    │
│   xterm canvas (active PTY only; others hidden)    │
│                                                    │
└────────────────────────────────────────────────────┘
```

### 3.3 Tab states

| Sub-state | Appearance |
|---|---|
| Alive | Emerald `●` dot |
| Dead (process exited) | Rose `●` dot, opacity 60% |
| Active | Border zinc-600, bg zinc-800 |
| Inactive | No border, hover-only background |
| Hover | `×` close icon appears via `group-hover:inline` |

### 3.4 ConfirmClearModal appearance

| Region | Content |
|---|---|
| Backdrop | Full-screen `bg-black/60`, click-to-cancel |
| Card | 440px wide, `border-amber-700`, `bg-zinc-900` |
| Header | "Send /clear?" in amber-200 |
| Body p1 | "/clear ends the current Claude session and starts a fresh one — the current context window is lost." |
| Body p2 | "The successor session in the same cwd will be detected automatically and your custom session title will carry forward, but in-flight reasoning and uncommitted scratchpad content cannot be recovered." |
| Footer | Cancel (zinc) + "Yes, send /clear" (amber-600) buttons |

### 3.5 QuickCmd visual states

| State | Border / bg | Text | Cursor |
|---|---|---|---|
| Enabled (emerald) | emerald-700 / emerald-900/30 | emerald-200 | pointer |
| Enabled (zinc) | zinc-700 / zinc-900 | zinc-300 | pointer |
| Enabled (amber) | amber-700 / amber-900/30 | amber-200 | pointer |
| Disabled (any tone) | zinc-800 | zinc-600 | not-allowed |

---

## 4. Interactions & Tap Targets

| Target | Gesture | Result |
|---|---|---|
| Tab (button portion) | Click | Sets `activeId`; canvas swaps |
| Tab × icon | Click | Calls `window.rcc.ptyKill(id)`; tab persists with rose dot until removed by IPC |
| `▶ /rick next` | Click (when enabled) | Writes `/rick next\r` to active PTY |
| `▶ /rick status` | Click | Writes `/btw rick status\r` to active PTY |
| `▶ /clear` | Click | Opens ConfirmClearModal (does NOT send) |
| Modal Cancel / backdrop | Click | Closes modal, no command sent |
| Modal "Yes, send /clear" | Click | Closes modal, writes `/clear\r` to active PTY |
| Hide button | Click | Collapses panel; ⌃ Show in stub re-expands |
| xterm canvas | Type | All keystrokes pass through to PTY via `ptyWrite` |

---

## 5. Active PTY Selection

The panel manages its own `activeId` state. Selection rules:

| Trigger | Behavior |
|---|---|
| Initial mount with N PTYs | Prefer PTY whose `sessionId` matches `selectedSessionId`; else last-added |
| `selectedSessionId` changes AND a PTY exists for it | Snap `activeId` to that PTY |
| `ptys` changes such that current `activeId` no longer exists | Re-pick: prefer-by-session, else last-added |
| `ptys` becomes empty | Clear `activeId` |

---

## 6. Per-Session Filtering

The panel only sees PTYs whose `sessionId` matches the selected session. Filtering happens in `App.tsx` (the `sessionPtys` memo) BEFORE the prop is passed -- the panel itself does no filtering.

Unbound (sessionId-less) PTYs are reaped by main: when a new transcript appears with a `cwd` that matches an unbound, alive PTY's cwd, main calls `pty.bindSession(handle.id, info.sessionId)`. This catches the launch-flow timing window where the PTY spawns first and the JSONL appears milliseconds later.

---

## 7. Component Composition

This feature is multi-component. See [`screen-composition.md`](screen-composition.md).

---

## Open Questions

- OQ1: Should dead PTYs auto-remove from the tab strip after N seconds, or stick until the user clicks ×?
- OQ2: Should the `/rick next` button have a keyboard shortcut (e.g. `Cmd+Enter`)? Currently mouse-only.
- OQ3: When a PTY is killed via the tab `×`, the underlying Claude session keeps running -- do we want a "soft kill" (`/exit` first, then SIGTERM)?
- OQ4: Should QuickCmd buttons appear when `terminalApp != 'in-app'`? Currently they don't because there's no PTY to write to -- but conceivably we could AppleScript-write to Terminal.app.
