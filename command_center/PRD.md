# Rick Command Center — Product Requirements Document

**Owner:** Dekel Maman (Autodesk, iOS)
**Status:** Draft v1.1
**Last updated:** 2026-05-03
**Scope:** macOS-first Electron desktop app for orchestrating Claude Code sessions, with deep Rick (`~/.rick/`) integration.

> **Detailed specs per surface live in [`specs/`](specs/).** This PRD is the requirements view; `specs/` is the implementation contract — one folder per feature, each with `spec.md` (presentation/behavior) and `contract.md` (numbered rules). Format follows the `ACC_Rewrite_universe` convention. Start with [`specs/README.md`](specs/README.md). Running session notes / open issues live in [`PROGRESS.md`](PROGRESS.md).

---

## 1. Overview

### 1.1 Problem Statement
Running multiple Claude Code sessions concurrently — particularly Rick-driven multi-agent workflows across several universes (e.g. `ACC_Rewrite_universe`, `ACC_issues_universe`) — is opaque from the terminal. There is no single place to see:

- which sessions are currently running, waiting on input, or blocked,
- which workflow each session is executing and how far along it is,
- how full each session's context window is before it crashes into the limit,
- what artifacts a session has produced and where its tracking lives,
- how to launch a new workflow without remembering the exact `/rick run` invocation and YAML param shape.

The result is constant context-switching between terminals, lost sessions, and surprise context-window failures mid-run.

### 1.2 Solution Summary
A macOS-native Electron app — **Rick Command Center** — that acts as a unified dashboard and launchpad for Claude Code sessions. It:

- watches `~/.rick/` and `~/.claude/projects/` read-only for sessions, workflows, agents, and transcripts,
- renders sessions as live cards with phase, step counter, status, context-window bar, and last activity,
- embeds a real `claude` PTY terminal for each launched session,
- auto-generates a launch form from any workflow's YAML `params:` block,
- ships a Claude plugin (hooks + optional `track` skill) that writes `~/.rick/tracking/<session-id>.md` so the app has reliable per-session progress without modifying `/rick`.

### 1.3 Target User
Single user, single machine, v1: **Dekel Maman**. iOS engineer at Autodesk, heavy Rick user, runs many concurrent Claude sessions across multiple universes. Power user — comfortable with CLI, JSONL, hook configs. The app is a personal productivity tool, not a product for external distribution.

### 1.4 Success Metrics
This is a personal tool, so metrics are pragmatic:

- **SM-1**: Time-to-spot a blocked session drops from "whenever I notice" to under 30 seconds (driven by Notification hook + OS notifications).
- **SM-2**: Zero unexpected context-window crashes per week after the indicator + critical-threshold notification ship.
- **SM-3**: New workflows launchable from the UI without consulting the YAML by hand (auto-form covers 100% of param shapes Rick uses today).
- **SM-4**: At least 5 concurrent sessions visible and tractable on a single screen without UI thrash.

---

## 2. Goals & Objectives

### 2.1 Primary Goals
1. **Visibility** — surface every running Claude/Rick session with status, progress, and context usage at a glance.
2. **Control** — launch any Rick workflow from a form-driven UI; interact with it via an embedded terminal.
3. **Safety** — warn before context windows fill, before sessions silently stall, and before tracking drifts from reality.
4. **Non-invasiveness** — read `~/.rick/state` and `~/.rick/universes` without writing; do not fork or modify `/rick` in v1.

### 2.2 Secondary Goals
- Browseable workflow library (search, params preview, descriptions).
- Browseable agent personas per universe.
- Markdown/YAML/JSON file preview for spec and tracking files.

### 2.3 Non-Goals (v1)
- **NG-1**: Editing Rick's own state files (`~/.rick/state/*.json`) or universe definitions.
- **NG-2**: Modifying the `/rick` skill itself. (`track` ships as a separate optional skill.)
- **NG-3**: Cross-platform support. macOS only in v1; Windows/Linux deferred.
- **NG-4**: Multi-user, cloud sync, team mode.
- **NG-5**: Replacing the Claude Code CLI. The app augments it; the terminal panel runs the real `claude` binary.
- **NG-6**: Rich code editing. File preview is read-only rendering, not an editor.

---

## 3. Users & Personas

### 3.1 Primary Persona — "Dekel"
- iOS engineer, Autodesk, ACC team.
- Maintains multiple Rick universes; jumps between them many times a day.
- Runs 3–10+ concurrent Claude sessions for parallel feature work, bugs, and reviews.
- Lives in macOS, Terminal, Xcode, VS Code; comfortable with shell and JSONL.
- Pain points: losing track of which terminal is which session; missing Notification hook firings; context-window surprise.

There are no secondary personas in v1.

---

## 4. Information Architecture & UX

### 4.1 Window Layout
A single main window (resizable, remembered between launches) with the following regions:

```
+--------------------------------------------------------------------+
| Top Bar: [Universe ▾]  [Search]   [Settings]                       |
+----+------------+-----------------+--------------------------------+
|    |            |                 | File Preview                   |
| L  | Sessions   | Spec / tracking |  (markdown/yaml/json)          |
| e  | Workflows  | files for the   +--------------------------------+
| f  | (tabs)     | selected        | Terminal (xterm.js + node-pty) |
| t  |            | session         |                                |
|    |            |                 +--------------------------------+
|    |            |                 | Progress + Phases (stepper)    |
+----+------------+-----------------+--------------------------------+
```

- **Top bar**: Universe switcher (dropdown populated from `~/.rick/universes/<name>/`), global search, settings.
- **Left drawer (collapsible)**: two tabs — **Sessions** (live), **Workflows** (browse + launch). Drawer can collapse to icon rail.
- **Second column**: list of spec/tracking files for the currently selected session (tracking.md, related design docs, artifacts).
- **Right area, three stacked panels (resizable splitters, persisted)**:
  1. File preview.
  2. Embedded terminal — one fresh `claude` PTY per launched session.
  3. Progress + phases panel — frontmatter rendered as a stepper, body rendered as live log.

### 4.2 Session Card
Each card in the Sessions tab shows:

- Workflow name + universe badge.
- Status badge: `running` | `waiting` | `blocked` | `done` | `idle`.
- Current phase + step counter (e.g. `implement (3/8)`).
- Context window bar: e.g. `142k / 200k (71%)`, amber tint at warn threshold, red tint at critical threshold.
- Last activity timestamp (relative: "12s ago", "3m ago").
- Click → selects the session (drives second column, file preview, terminal focus, progress panel).

### 4.3 Workflow Library Tab
- List of workflows for the active universe, parsed from `~/.rick/universes/<name>/workflows/*.yaml`.
- Each card shows: workflow name, short description, agents involved, depends_on.
- Click → opens **Launch Modal** (see FR-7).

### 4.4 Launch Modal
- Auto-generated form from the YAML `params:` block: one field per param, typed (string/int/bool/enum), with placeholder=default, label=name, helper=description.
- Required fields validated.
- Bottom of modal: free-form **"extra prompt"** textarea appended to the first message.
- Two pickers: **cwd** (directory chooser, defaults to last used per workflow) and **universe** (defaults to current).
- Submit → spawn fresh `claude` PTY with first message:
  `/rick run <name> --params='{...json...}'\n<extra prompt>`.
- New tracking file is created on first hook fire.

### 4.5 UX Principles
- **Glanceable first, drillable second.** Status and context bars are always visible without clicks.
- **Read-mostly.** Avoid destructive actions; the only writes are tracking files we own and PTY input the user types.
- **Single source of truth = files on disk.** No in-app caches that can lie.
- **Quiet by default.** Only warn/critical thresholds and `blocked` events fire OS notifications.

### 4.6 Accessibility
- Keyboard shortcuts for tab switching, session navigation (`⌘↑/⌘↓`), focus terminal (`⌘T`), open settings (`⌘,`).
- Status communicated via icon + text + color (not color alone).
- Respect macOS Reduce Motion; no required animations.
- Standard system font sizing honored.

---

## 5. Data Sources & Ownership

### 5.1 Read-Only Watchers (chokidar)
| Path | Purpose |
| --- | --- |
| `~/.rick/universes/<name>/workflows/*.yaml` | Workflow definitions (params, steps, agents, depends_on). |
| `~/.rick/universes/<name>/agents/<name>/` | Agent personas (markdown). |
| `~/.rick/state/*.json` | Live workflow state (Rick-owned; **never written**). |
| `~/.rick/profile.yaml` | Dev / non-dev mode and global Rick prefs. |
| `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl` | Claude transcripts — token usage, last activity, first user message detection. |

### 5.2 App-Owned Files (read/write)
| Path | Purpose |
| --- | --- |
| `~/.rick/tracking/<session-id>.md` | Per-session tracking file. Frontmatter: `session_id`, `workflow`, `status`, `total`, `completed`, `current`, `phase`, `started`, `updated`. Body: rendered todos, phase log, artifacts. Written by hooks and the optional `track` skill. |
| `~/.config/rick-command-center/settings.json` | App settings (thresholds, last universe, last cwds, panel sizes). |

### 5.3 Plugin Shipped By App
Installed on first run into the user's Claude Code config:

- **Hooks** (described in FR-9).
- **Optional `track` skill** (described in FR-10). Workflows opt in by mentioning `track` in their persona/instructions.

---

## 6. Functional Requirements

### 6.1 Universe & Session Surface
- **FR-1**: The app SHALL list all universes by reading directory names under `~/.rick/universes/` and present them in a top-bar dropdown. Selection persists across launches.
- **FR-2**: The app SHALL list all live sessions for the active universe, derived from (a) tracking files in `~/.rick/tracking/` whose workflow belongs to that universe and (b) recent `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl` files modified in the last N days (default 7).
- **FR-3**: Each session card SHALL display: workflow name, universe, status, current phase, step counter, context-window bar (used / limit / percent), and last-activity timestamp. Update latency from filesystem change to UI SHALL be ≤ 1 second under normal load.
- **FR-4**: Status SHALL be derived as follows:
  - `running` — JSONL modified in last 30s and tracking status not `waiting`/`blocked`/`done`.
  - `waiting` — last hook event was `Notification`.
  - `blocked` — tracking frontmatter `status: blocked` (set by `track blocked`).
  - `done` — tracking frontmatter `status: done` or last hook was `Stop`.
  - `idle` — none of the above; JSONL untouched > 30s.

### 6.2 File Preview
- **FR-5**: The app SHALL render selected files as:
  - Markdown via `react-markdown` (GFM, code highlighting),
  - YAML pretty-printed with syntax highlighting,
  - JSON pretty-printed with collapsible nodes.
- **FR-6**: Files SHALL update live when changed on disk; preview SHALL not flicker on unchanged content.

### 6.3 Workflow Library & Launch
- **FR-7**: The app SHALL parse all `~/.rick/universes/<active>/workflows/*.yaml` files via `js-yaml` and list them as cards (name, description, agents, depends_on).
- **FR-8**: Clicking a workflow card SHALL open the Launch Modal:
  - Form fields auto-generated from the YAML `params:` block (one field per param; type, default, description honored).
  - Required-param validation before submit.
  - "Extra prompt" textarea below the form.
  - cwd picker (directory chooser, last value remembered per workflow).
  - Universe picker (defaults to active).
  - On submit, spawn a fresh `claude` PTY in the terminal panel via `node-pty` with first message: `/rick run <name> --params='<json>'\n<extra prompt>`.
  - Each launch SHALL produce a new session_id and a new tracking file.

### 6.4 Hooks (App-Shipped Claude Plugin)
- **FR-9**: The plugin SHALL register the following hooks. Each hook updates `~/.rick/tracking/<session-id>.md` atomically (write-temp + rename):
  - **UserPromptSubmit** — scan first user message for `/rick run <workflow>` and bind `session_id ↔ workflow` in tracking frontmatter.
  - **PostToolUse:TodoWrite** — recompute `total`, `completed`, `current` from the latest todo list and update the rendered body.
  - **PostToolUse:Task** — append subagent spawns to the artifacts section.
  - **Notification** — set `status: waiting` and emit an OS notification.
  - **Stop** — read the latest main-thread assistant message via `transcript_path`. Only set `status: done` when the message contains the workflow-completion banner `Rick: All N steps complete` (regex `COMPLETE_RE`). Otherwise no-op — Stop fires every turn and would otherwise poison the badge between phases.

### 6.5 Optional `track` Skill
- **FR-10**: The plugin SHALL ship an optional `track` skill with verbs:
  - `track phase <name>` — set `phase` in tracking frontmatter.
  - `track blocked <reason>` — set `status: blocked`, append reason.
  - `track unblocked` — clear blocked status.
  - `track artifact <path>` — append to artifacts list.
  Workflows opt in by referencing the skill in their persona/instructions. `/rick` itself remains unmodified in v1.

### 6.6 Context Window Indicator
- **FR-11**: For each session, the app SHALL compute current context as `input_tokens + cache_read_input_tokens` from the latest assistant message in the session JSONL.
- **FR-12**: The app SHALL look up the model's window limit from this map:
  | Model ID | Limit |
  | --- | --- |
  | `claude-opus-4-7[1m]` | 1,000,000 |
  | `claude-opus-4-7` | 200,000 |
  | `claude-sonnet-4-6[1m]` | 1,000,000 |
  | `claude-sonnet-4-6` | 200,000 |
  | `claude-haiku-4-5` | 200,000 |
  Unknown models SHALL fall back to 200,000 with a small "?" indicator.
- **FR-13**: Two thresholds with defaults: **Warn 70%** (amber tint), **Critical 90%** (red tint + OS notification, optional auto-suggest `/compact` in the terminal as a non-executed command preview).
- **FR-14**: Thresholds SHALL be configurable globally and per-session.

### 6.7 Embedded Terminal
- **FR-15**: The terminal panel SHALL embed `xterm.js` backed by a `node-pty` child process running the user's `claude` binary.
- **FR-16**: Each launched session SHALL get a **fresh** PTY (no pooling, no reuse). Closing the panel SHALL kill the child cleanly.
- **FR-17**: The terminal SHALL pass through all keystrokes and respect the user's shell/locale env.

### 6.8 Notifications
- **FR-18**: OS notifications SHALL fire on:
  - status transition to `waiting` (Notification hook),
  - status transition to `blocked`,
  - context window crossing the critical threshold,
  - session `Stop` (success or error).
- **FR-19**: Notifications SHALL deep-link to the session in the app on click.

### 6.9 Settings
- **FR-20**: A settings panel SHALL allow configuration of:
  - Warn threshold (default 70%).
  - Critical threshold (default 90%).
  - Whether to auto-suggest `/compact` at critical.
  - Recent-session window in days (default 7).
  - Plugin install state (re-install button).

### 6.10 Plugin Install Flow
- **FR-21**: On first run, the app SHALL detect whether its hooks and (optionally) the `track` skill are installed in the user's Claude Code settings. If not, it SHALL show a one-screen consent dialog showing the diff to be applied to `settings.json` and only write on explicit approval.

### 6.11 Workflow Step Correlation
> Full contract spec — including exact regex patterns, Rick-side requirements, and code anchors — lives in [`specs/rick-contract/`](specs/rick-contract/). The summary below is the requirements view.

- **FR-22**: The app SHALL produce a `WorkflowRunView` for any session bound to a workflow, mapping each YAML outer step to a status ∈ `{pending, running, done}` plus optional `currentSubagent` / `currentActivity` for the running step. Correlation passes run in this order; later passes override earlier ones for the same step:
  1. **Direct subagent matching** — match `Task.subagent_type` (with `rick-<universe>-` prefix stripped) against `step.agent` and `step.collaborators`. Works for direct-agent workflows.
  2. **Phase markers** — exact-format `[rick:phase <step-id> <starting|complete>]` lines in main-thread assistant text. Step-id matches the YAML `id`.
  3. **Phase-number prose fallback** — `Phase <N> <starting|started|begins|begun|done|complete|finished>`, mapped by phase number → step index. Negative-tense forms (`will`, `=`, `Waiting on`) are rejected by regex.
  4. **Announcement counting (legacy)** — total `Rick: Handing to …` and `Rick: <agent> is done` counts across the transcript. Last-resort heuristic; only fires when none of 1–3 produced a signal.
- **FR-23**: For the highest-indexed running step, the app SHALL parse the most recent `**Rick:** Handing to **<Subagent>** [(<Role>)] — <activity> [claude:opus]` line and attach `currentSubagent` / `currentActivity`. The renderer SHALL display these in place of the generic agent / sub-workflow name while running; non-running steps keep the original label.

### 6.12 Live Refresh
- **FR-24**: The file list (incl. workflow rings, tracking, touched files) SHALL re-fetch on every change of `session.lastActivity`, in addition to session selection. The Summary panel already does. Cost is one JSONL re-parse per transcript write — acceptable at current sizes; revisit with memoization if perf bites.

### 6.13 Quick Commands in Terminal Toolbar
- **FR-25**: The terminal toolbar SHALL expose three quick-command buttons that write to the active alive PTY:
  - **`▶ /rick next`** (emerald) — sends `/rick next\r`. **Disabled** unless `sessionStatus ∈ {idle, waiting}` AND there is an alive PTY for the session. Tooltip explains the specific reason for being disabled (`running`, `done`, `blocked`, no PTY).
  - **`▶ /rick status`** (zinc) — label says `/rick status`, command sent is `/btw rick status\r` so it doesn't interrupt Rick's flow.
  - **`▶ /clear`** (amber) — opens an amber-bordered confirmation modal explaining context loss. Cancel = no-op; confirm sends `/clear\r`.
- **FR-26**: When no alive PTY exists for the selected session, all quick-command buttons SHALL be disabled with a tooltip directing the user to open the in-app terminal.

### 6.14 Auto-continue Override
- **FR-27 (Launch-time)**: The Launch Modal SHALL include an `auto_continue` checkbox. Default = last user choice (persisted in `localStorage` key `rcc:launch:auto-continue`). When ON, the app SHALL prepend a deterministic override directive to the user's extra prompt before submitting:
  > `Override: run all phases with auto_continue: true — do not pause between phases or wait for me to say next. Drive the workflow end-to-end.`
- **FR-28 (Mid-run)**: Each session card SHALL display an `auto on` / `auto off` pill. Click flips the per-session state (persisted in `rcc:session:auto-continue` map) and writes a `/btw …` directive to the active in-app PTY:
  - ON: `From now on, run remaining phases with auto_continue: true …`
  - OFF: `From now on, run remaining phases with auto_continue: false …`
  No PTY → toast directing the user to open the terminal. The pill reflects "what was last commanded," not Rick's actual state — there is no acknowledgement parsing in v1.

### 6.15 Dev & Distribution Scripts
- **FR-29**: `package.json` SHALL expose:
  - `npm run kill` — `pkill -f` against `node_modules/.bin/electron-vite` and `SDDeditor/node_modules/electron/dist/Electron.app` only (narrowly scoped — must not match other Electron apps like Cursor, ChatGPT, Slack).
  - `npm run predev` — same kill, runs automatically before `npm run dev` so dev sessions self-clean.
  - `npm run package:dmg` — full clean build to ad-hoc-signed arm64 `.dmg` at `release/`. Sets `CSC_IDENTITY_AUTO_DISCOVERY=false` to skip Apple cert lookup.

---

## 7. Non-Functional Requirements

- **NFR-1 (Latency)**: Filesystem-change → UI-update p95 ≤ 1s for tracking and JSONL files up to 50 MB.
- **NFR-2 (Concurrency)**: Stable with at least 10 concurrent sessions and 10 watched JSONL files without UI jank.
- **NFR-3 (Memory)**: Steady-state RSS ≤ 600 MB with 10 sessions open, no terminals scrolled past their default scrollback.
- **NFR-4 (Startup)**: Cold start to first usable UI ≤ 3s on an M-series Mac.
- **NFR-5 (Read-only safety)**: The app SHALL NOT write to any file under `~/.rick/state/` or `~/.rick/universes/`. A runtime guard SHALL refuse such writes.
- **NFR-6 (Crash safety)**: A malformed YAML, JSONL, or tracking file SHALL be reported in a non-blocking toast and SHALL NOT take down the app or sibling sessions.
- **NFR-7 (Signing)**: The shipped `.dmg` is **ad-hoc (self-signed)** for the current single-user / trusted-recipient distribution model — `identity: null` in `electron-builder` mac config; `hardenedRuntime` disabled (incompatible with ad-hoc). Recipient does Right-click → Open the first launch. Upgrade path to Developer-ID + notarization is open if external distribution becomes a goal; switch path is two config-key flips and an `$99/yr` Apple Developer cert.
- **NFR-8 (Privacy)**: No telemetry, no network calls beyond what `claude` itself makes inside the PTY.
- **NFR-9 (Schema resilience)**: Rick state schema may change; the app SHALL schema-validate and degrade gracefully (show "unknown" rather than crash).

---

## 8. Architecture & Tech Stack

### 8.1 Stack
- **Shell**: Electron (main + renderer).
- **UI**: React + TypeScript + Tailwind.
- **File watching**: `chokidar` (main process).
- **Terminal**: `xterm.js` (renderer) + `node-pty` (main).
- **YAML/Markdown**: `js-yaml`, `gray-matter`, `react-markdown`.
- **State**: Renderer-side Zustand (or equivalent) store fed by IPC events from the main process.
- **Build**: Vite + electron-builder; signed/notarized `.dmg`.
- **Plugin**: A small Claude Code plugin bundle (hooks JS + `track` skill markdown) packaged inside the app and installed into the user's settings on first run.

### 8.2 Process Model
- **Main process**: chokidar watchers; JSONL tail-and-parse worker; PTY spawn/lifecycle; tracking-file writer; settings persistence.
- **Renderer**: UI, terminal frontends, markdown rendering, modal forms.
- **IPC**: typed channel for `sessions:update`, `workflows:update`, `file:changed`, `pty:data`, `pty:exit`, `pty:input`, `notify`.

### 8.3 JSONL Parsing Strategy
- Tail-only after initial scan (track byte offset per file).
- Parse latest assistant message lazily for usage tokens.
- Stream rather than load entire transcript into memory.
- Cap in-memory parsed message count per session (e.g., last 200) for the progress panel; full transcript stays on disk.

### 8.4 Tracking File Write Strategy
- Atomic write: write to `<file>.tmp`, fsync, rename.
- Single-writer per session (main process owns the lock).
- Hooks write via the main process IPC, not directly, when the app is running; when the app is offline, hooks write directly with the same atomic pattern.

---

## 9. Milestones

### M1 — MVP Viewer (~2 weeks)
- Universe switcher.
- Sessions list with status + context bar.
- File preview panel.
- Watchers for `~/.rick/` and `~/.claude/projects/`.
- Hooks shipped; tracking.md written.
- **No terminal yet, no launch flow.**

Exit criteria: I can see all my running sessions, their phase/step, and context usage live, in one window.

### M2 — Terminal + Launch (~2 weeks)
- xterm.js + node-pty terminal panel.
- Workflow Library tab.
- Launch modal with auto-generated form.
- Fresh PTY per launch with `/rick run` first message.

Exit criteria: I never need to leave the app to start a workflow; I can drive the conversation from the embedded terminal.

### M3 — Polish (~1 week)
- OS notifications (waiting / blocked / critical / done).
- Threshold settings + per-session overrides.
- Agent persona viewer.
- Global search (sessions, workflows, agents).

Exit criteria: I get notified before something breaks, and I can find anything in two keystrokes.

### M4 — Stretch
- `track` skill adoption inside `/rick`.
- Agent activity heatmap.
- Multi-window support.
- Remote / team mode (read-only mirror of someone else's `~/.rick/`).

---

## 10. Risks & Mitigations

| ID | Risk | Mitigation |
| --- | --- | --- |
| R-1 | `node-pty` macOS signing/entitlements complexity (hardened runtime, native binary). | Prototype signing in M1; pin a known-good `node-pty` version; use `electron-builder` recipes that handle native modules. |
| R-2 | JSONL parsing perf with very large transcripts. | Tail-only parsing with persisted byte offsets; cap in-memory message buffer; move parse to a worker thread. |
| R-3 | "Blocked" detection unreliable — Notification hook may not always fire. | Add `track blocked` skill verb as an authoritative path; fall back to JSONL-idleness heuristic. |
| R-4 | Rick state schema drift. | Schema-validate at parse time; show "unknown" gracefully; gate Rick-specific UI behind capability flags. |
| R-5 | Plugin install flow surprises the user / clobbers `settings.json`. | Show a diff preview and require explicit approval; never write on launch without consent; back up the previous settings.json. |
| R-6 | Concurrent writes to tracking.md from multiple hook firings. | Single-writer in main process when app is running; atomic temp-rename pattern when offline. |
| R-7 | Context window calculation undercounts (cache vs non-cache paths, tools). | Document the formula in tooltip; expose raw numbers; allow per-session manual override of the limit. |

---

## 11. Open Questions

- **OQ-1**: Where exactly should the plugin's hooks live in `settings.json` to coexist with the user's existing hooks? (Probably under a namespaced key, but needs verification.)
- **OQ-2**: Should the app ever auto-execute `/compact` at critical, or always just suggest? (Default: suggest only, behind a setting.)
- **OQ-3**: Should `idle` sessions auto-archive after N days, or stay visible until manually dismissed?
- **OQ-4**: For workflows without a `params:` block, should the launch modal still appear (with only cwd + extra prompt) or fast-path straight to PTY spawn?
- **OQ-5**: Per-universe settings overrides — needed in v1 or M4?

---

## 12. Appendix

### 12.1 Tracking File Example
```markdown
---
session_id: 01HXYZ...
workflow: implement-feature
status: running
total: 8
completed: 3
current: "Wire reducer to view"
phase: implement
started: 2026-04-30T09:11:22Z
updated: 2026-04-30T09:42:05Z
---

## Todos
- [x] Read spec
- [x] Plan reducer
- [x] Add state
- [ ] Wire reducer to view  ← current
- [ ] Add tests
- [ ] ...

## Phase log
- 09:11 research
- 09:24 design
- 09:33 implement

## Artifacts
- Sources/Feature/Reducer.swift
- Sources/Feature/View.swift
```

### 12.2 Glossary
- **Universe** — a Rick-managed collection of agents, workflows, and state under `~/.rick/universes/<name>/`.
- **Workflow** — a YAML-defined multi-agent procedure executed by `/rick run`.
- **Session** — a single Claude Code conversation, identified by `session_id`, with one transcript JSONL.
- **Tracking file** — `~/.rick/tracking/<session-id>.md`, owned by this app's plugin.

### 12.3 Out-of-Scope Reminders
Reaffirming Section 2.3: no Rick state writes, no `/rick` modifications, no Windows/Linux, no cloud, no editor.
