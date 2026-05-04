# Sessions Panel -- Presentation Spec

## 1. Overview

The leftmost panel of the application. Lists every Claude/Rick session for the active universe with enough at-a-glance state that the user can triage across many concurrent sessions without switching tabs or scrolling into the terminal output of each.

The panel hosts two tabs — **Sessions** (this spec) and **Workflows** (see [`workflows-panel/spec.md`](../workflows-panel/spec.md)). Default tab is Sessions.

**Source files:**
- `src/renderer/src/components/Drawer.tsx` (lines 33-200) -- Tab container, filters, search, pinning
- `src/renderer/src/components/SessionCard.tsx` (lines 32-216) -- Session card
- `src/main/services/sessions.ts` (lines 21-217) -- Session composition + status derivation
- `src/main/services/sessions.ts` (lines 221-252) -- Predecessor/successor linking
- `src/shared/types.ts` (lines 40-63) -- Session type
- `src/main/services/models.ts` -- Context-window model lookup

---

## 2. Presentation Models

### 2.1 Session

The shape pushed from main to renderer. Conforms to: structural-only (POJO over IPC).

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `id` | String | No | `"4cc80b31-0165-47ff-8781-2de04532877d"` | Claude Code session id (UUID v4) |
| `title` | String | Yes | `"SCCOM-33084"` | dynamic -- auto-derived from workflow params or set by user |
| `customTitle` | String | Yes | `"My investigation"` | Static once set -- presence triggers ✎ marker |
| `workflow` | String | Yes | `"bug-fix-from-jira"` | dynamic -- bound on first `/rick run` prompt or detected from "Running **<x>**" text |
| `universe` | String | Yes | `"ACC_issues_universe"` | From tracking frontmatter |
| `cwd` | String | No | `"/Users/.../build-mobile-clean-dev"` | The working directory the PTY was started in |
| `transcriptPath` | String | No | `"/Users/.../<sid>.jsonl"` | Path to the JSONL transcript |
| `trackingPath` | String | Yes | `"/Users/.../tracking/<sid>.md"` | Present when tracking exists |
| `status` | SessionStatus | No | `"running"` | dynamic -- derived (see contract §1) |
| `phase` | String | Yes | `"reproduce"` | From tracking frontmatter |
| `total` | Int | Yes | `13` | Total todos from latest TodoWrite |
| `completed` | Int | Yes | `5` | Completed todos count |
| `current` | String | Yes | `"map testable surface"` | Current in-progress todo |
| `context` | ContextWindow | Yes | (see 2.4) | Latest model + token usage |
| `lastActivity` | Number (ms) | No | `1746284123000` | Latest JSONL mtime |
| `startedAt` | Number (ms) | Yes | `1746280000000` | Parsed from tracking `started:` |
| `successorId` | String | Yes | `"d2a91f3c-..."` | dynamic -- set by predecessor/successor linking |
| `predecessorId` | String | Yes | `"4cc80b31-..."` | dynamic -- inverse of `successorId` |

### 2.2 SessionStatus (enumeration)

| Case | Color dot | Meaning |
|---|---|---|
| `running` | Emerald | JSONL modified < 30s ago AND no overriding terminal status |
| `waiting` | Amber | Notification hook fired -- agent awaiting input/permission |
| `blocked` | Rose | Tracking frontmatter `status: blocked` (set by `track blocked`) |
| `done` | Zinc-500 | Workflow-completion banner observed in transcript |
| `idle` | Zinc-700 | None of the above; transcript untouched > 30s |

### 2.3 StatusFilterChips

State held in the Drawer component. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `enabled` | Record<SessionStatus, Bool> | No | `{running: true, waiting: true, ...}` | All on by default. Off chips hide their status. |

### 2.4 ContextWindow

Per-session context-window state. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `used` | Number | No | `89000` | `input_tokens + cache_read_input_tokens + cache_creation_input_tokens` from latest assistant turn |
| `limit` | Number | No | `200000` | Looked up from model id; promoted to 1M if observed usage exceeds 200k |
| `model` | String | No | `"claude-opus-4-7"` | dynamic -- model id from latest assistant message |
| `modelKnown` | Bool | No | `true` | dynamic -- false when model id not in lookup table; UI shows `?` |

### 2.5 SessionCardActions

Hover-only controls in the top-right of each card. Conforms to: Sendable.

| Action | Visibility | Behavior |
|---|---|---|
| Focus terminal (⤴) | Visible only when `cwd` is non-empty | Calls `focusTerminal({cwd, sessionId, terminalApp, skipPermissions})` |
| Discard (×) | Always visible | Confirms via `window.confirm`; on confirm deletes tracking file + adds id to `archivedSessionIds` |

### 2.6 AutoContinuePill

Per-card pill on the status row. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `state` | Bool | No | `true` | dynamic -- per-session; persisted in `localStorage[rcc:session:auto-continue]` map |
| `label` | String | No | `"auto on"` | dynamic -- "auto on" when true, "auto off" when false |
| `tone` | String | No | `"emerald"` | dynamic -- emerald when true, zinc when false |

---

## 3. Visual States

### 3.1 Card Layout

| Region | Content | Notes |
|---|---|---|
| Title row (left) | Title + ✎ if customTitle set | Click to rename |
| Title row (right) | Relative time | "just now" / "2m ago" / "3h ago" / "1d ago" |
| Workflow line | `workflow` value | Hidden when null |
| Successor chip | Amber pill `→ continued at <8-char-id>` | Hidden when no successor |
| Status row (left) | Status dot + label + phase + counter | All ordered with `·` separator |
| Status row (right) | AutoContinuePill | Right-aligned via `ml-auto` |
| Context bar | Used/limit progress with color-coded fill | Hidden when no context |

### 3.2 Selected vs unselected

| State | Border | Background | Position |
|---|---|---|---|
| Selected | `border-zinc-500` | `bg-zinc-800` | Pinned to top of list with `— others —` separator below |
| Unselected | `border-zinc-800` | `bg-zinc-900` | Sorted by lastActivity desc |

### 3.3 Context bar tone

| Used / limit ratio | Tone |
|---|---|
| `< warnThreshold` (default 70%) | Emerald (`bg-emerald-500`) |
| `>= warn`, `< criticalThreshold` (default 90%) | Amber (`bg-amber-400`) |
| `>= critical` | Rose (`bg-rose-500`) |
| No context yet | Zinc-700 (`bg-zinc-700`) |

### 3.4 Title editing

| Sub-state | Appearance |
|---|---|
| Display | Title text. ✎ marker if `customTitle` is set. Hover hint on click. |
| Editing | Inline `<input>` with focus + auto-select |
| Editing → Enter / blur | Commits non-empty trimmed value via `onRenameTitle(text)` |
| Editing → Escape | Reverts to display, no commit |
| Editing with empty value, when `customTitle` is set | Commits `null`, reverting to auto-derived title |

---

## 4. Interactions & Tap Targets

| Target | Gesture | Result |
|---|---|---|
| Card body | Click | Selects this session (`onSelect(id)`) and clears file selection |
| Title text | Click | Enters editing mode (when `onRenameTitle` is provided) |
| Title input | Enter | Commits |
| Title input | Escape | Cancels |
| Title input | Blur | Commits |
| ⤴ icon | Click | Calls `onFocusTerminal()` |
| × icon | Click | Confirms via `window.confirm`; on confirm calls `onDiscard()` |
| AutoContinuePill | Click | Flips state, persists, fires `onSetAutoContinue(newState)` |
| Successor chip | Click | Calls `onJumpToSuccessor()` (switches selection to successor) |
| Status filter chip | Click | Toggles that status in/out of visibility |
| Search input | Type | Filters by substring across title, workflow, cwd, id |

---

## 5. Search & Filtering

Free-text search box at the top of the Sessions tab. Filters cards by case-insensitive substring match against:

- `session.title`
- `session.workflow`
- `session.cwd`
- `session.id`

Combined with status-filter chips: a session is visible when its status chip is enabled AND its text matches the search.

---

## 6. Pinning Behavior

The currently-selected session always renders first in the visible list, even if it would normally sort lower by `lastActivity`. A `— others —` separator divides the pinned card from the rest. Rationale: when the user clicks in to inspect a session, they should not lose scroll context as new activity reorders the list.

---

## 7. Successor / Predecessor Linking

Sessions in the same `cwd` whose start times are within 1 hour of the older session's last activity are linked. The older session is force-marked `done` (status promoted), gets a `successorId`, and renders the amber `→ continued at <8-char-id>` chip. The successor inherits the older session's `customTitle` if it has none of its own.

This catches the `/clear` flow: user runs `/clear` mid-workflow, Claude Code starts a new session, Rick resumes from `tracking.md`. The link makes continuity visible.

See implementation: `propagateContinuations()` in `src/main/services/sessions.ts:221`.

---

## 8. Component Composition

This panel is multi-component. See [`screen-composition.md`](screen-composition.md) for the full component tree and ownership map.

---

## Open Questions

- OQ1: Should the auto-continue pill reflect Rick's actual current behavior (acknowledgement-parsed) or stay as "what the user last commanded"? Currently the latter.
- OQ2: Is 30s the right "running" threshold, or should it scale with `recentSessionDays`? Long-running build steps can exceed 30s of silent compute.
- OQ3: Successor linking uses 1-hour cwd-match. Is this stable when the user opens two unrelated sessions in the same cwd within an hour? Currently yes — both link, which may be wrong.
- OQ4: Should hidden-by-filter sessions still count in the tab badge?
