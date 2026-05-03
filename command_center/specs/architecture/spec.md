# Architecture -- System Spec

## 1. Overview

How Claude Code, Rick, and the Command Center fit together. Three independent processes, one shared filesystem.

The Command Center is a **read-mostly observer** of two systems it doesn't own:

- **Claude Code (CLI)** — produces JSONL transcripts at `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl` and reads its config from `~/.claude/settings.json`.
- **Rick** — a Claude persona installed at `~/.rick/` (universes, workflows, agents, persona). When the user runs `/rick run …` inside Claude Code, Rick orchestrates a workflow. Rick reads `~/.rick/`, writes nothing of its own — but its turn outputs land in Claude Code's JSONL.

The Command Center adds:

- A **plugin** (hooks) installed into Claude Code that writes per-session state to `~/.rick/tracking/<session-id>.md`. See [`../hooks/spec.md`](../hooks/spec.md).
- A **desktop app** (this Electron app) that watches the filesystem, parses the JSONL + tracking files, and renders the workflow UI.

Nothing in this app talks to Anthropic's API directly. The PTY embedded in the app launches `claude` as a subprocess; Claude Code is the only thing on the wire.

**Source files:**
- `src/main/handlers.ts` (lines 50-245) -- Service wiring + IPC handler registration
- `src/main/services/paths.ts` -- Path constants
- `src/shared/ipc.ts` -- IPC channel constants + RccApi interface
- `src/shared/types.ts` -- Shared domain types
- `src/renderer/src/state.ts` -- Renderer's app-state hook with IPC subscriptions
- `src/renderer/src/App.tsx` -- Top-level layout

---

## 2. Data Flow

```
User
 │
 ├─► Claude Code (CLI in PTY) ───────────────────► Anthropic API
 │       │
 │       ├─ writes JSONL ──► ~/.claude/projects/<cwd>/<sid>.jsonl
 │       │
 │       └─ fires hooks ──► node ~/.../plugin/hooks/<event>.mjs
 │                              │
 │                              └─ atomic write ──► ~/.rick/tracking/<sid>.md
 │
 └─► Rick (persona, in same Claude session)
         │
         ├─ reads ~/.rick/persona, ~/.rick/universes, ~/.rick/agents
         │
         └─ emits structured signals into JSONL via assistant-text
              [rick:phase <id> starting/complete]
              **Rick:** Handing to **<Subagent>** — <activity>
              Rick: Phase 1 done
              Rick: All N steps complete

Command Center (Electron)
 │
 ├─ chokidar watches ~/.claude/projects/  ──► TranscriptService
 ├─ chokidar watches ~/.rick/tracking/    ──► TrackingService
 ├─ chokidar watches ~/.rick/universes/   ──► UniverseService
 │
 │   all three feed into ──► SessionsService.recompute()
 │                              │
 │                              └─ emits to renderer over IPC ──► useAppState() hook
 │
 ├─ on demand: SessionsService.listSessionFiles(sid)
 │       └─► correlateWorkflow() reads JSONL, returns WorkflowRunView
 │
 └─ in-app PTY (xterm.js + node-pty) ──► spawns `claude` ──► loop continues
```

---

## 3. Process Topology

| Process | Role | Lifetime |
|---|---|---|
| Claude Code (`claude` CLI) | Anthropic API client; runs Rick persona; writes JSONL | One per active session; spawned by launcher |
| Command Center main | Electron main; services + watchers + IPC handlers + PTY service | One per app launch |
| Command Center preload | `contextBridge` — exposes `window.rcc` to renderer | One per renderer |
| Command Center renderer | React + Tailwind UI | One per app window |

---

## 4. Services in Main

All wired in `src/main/handlers.ts:registerHandlers()`. Each owns its own chokidar watcher.

### 4.1 Service Inventory

| Service | File | Watches | Produces |
|---|---|---|---|
| `UniverseService` | `services/universes.ts` | `~/.rick/universes/*` | `Universe[]` + `Workflow[]` (parsed YAMLs with normalized `params`, `steps`, `agents`) |
| `TranscriptService` | `services/transcripts.ts` | `~/.claude/projects/*` | `TranscriptInfo[]` per session: `cwd`, `lastActivity`, `context`, `firstUserMessage`, detected `workflow` |
| `TrackingService` | `services/tracking.ts` | `~/.rick/tracking/*.md` | `TrackingFile[]`: parsed frontmatter + body |
| `SessionsService` | `services/sessions.ts` | (composes the three above) | `Session[]`: id, title, workflow, universe, cwd, status (derived), context, links |
| `SummaryService` | `services/summary.ts` | (on-demand) | `SessionSummary` for one session |
| `Correlation` | `services/correlation.ts` | (on-demand) | `WorkflowRunView`: per-outer-step status, files, live activity |
| `SpecsService` | `services/specs.ts` | (on-demand) | List of spec files for a project cwd |
| `PluginInstaller` | `services/install.ts` | -- | Manages `~/.claude/settings.json` hook installation |
| `WorkflowLauncher` | `services/launcher.ts` | -- | Builds `/rick run …`, spawns terminal, optionally creates git worktree |
| `PtyService` | `services/pty.ts` | -- | Owns `node-pty` children for in-app terminals |
| `SettingsService` | `services/settings.ts` | -- | Reads / patches / persists `AppSettings` |

### 4.2 Composition

`SessionsService` re-runs `recompute()` whenever transcripts OR tracking changes. Sessions are emitted to the renderer as a single array; the renderer derives "selected session" client-side.

---

## 5. IPC Shape

`src/shared/ipc.ts` defines:

- **Channel constants** (`IPC.GetSettings`, `IPC.SessionsUpdate`, `IPC.PtyWrite`, etc.).
- **`RccApi` interface** -- the surface exposed to the renderer via `window.rcc`.

Two communication patterns:

| Pattern | Direction | Mechanism | Examples |
|---|---|---|---|
| Request / Response | Renderer → Main | `ipcRenderer.invoke(channel, ...)` | getters (`getSessions`, `readTracking`), mutations (`setSettings`, `discardSession`), on-demand reads (`listSessionFiles`, `getSessionSummary`) |
| Push | Main → Renderer | `webContents.send(channel, data)` + renderer `ipcRenderer.on(channel, cb)` | live updates (`UniversesUpdate`, `WorkflowsUpdate`, `SessionsUpdate`, `PtyData`, `PtyListUpdate`, `PtyExit`) |

The renderer's `useAppState()` hook (`src/renderer/src/state.ts`) wires the push channels to React state. Every push fires `setX` and re-renders consumers.

---

## 6. File Watcher Details

| Watcher | Root | Depth | Debounce | Notes |
|---|---|---|---|---|
| Transcripts | `~/.claude/projects/` | 2 | none | Per-line append matters; immediate re-parse |
| Tracking | `~/.rick/tracking/` | 1 | none | Per-write atomic |
| Universes | `~/.rick/universes/` | 4 | 250ms | YAML rescans are expensive; coalesce |

All watchers use `chokidar` with `ignoreInitial: true` (initial scan done explicitly), and skip `.git/` + `.DS_Store`.

---

## 7. Filesystem Ownership

The app writes to:

| Path | Writer | When |
|---|---|---|
| `~/.rick/tracking/<sid>.md` | The plugin hooks (NOT the app process) | UserPromptSubmit / PostToolUse / Notification / Stop |
| `~/.claude/settings.json` | App (only on plugin install/uninstall) | InstallModal consent flow |
| `~/Library/Application Support/rick-command-center/` | App | Settings, archived ids, panel sizes |
| `<repo>/.claude/worktrees/<name>/` | App (via `git worktree add`) | Launch modal, worktree mode |

The app does NOT write to `~/.rick/state/` or `~/.rick/universes/` (NFR-5).

---

## 8. Trust Boundaries

| Boundary | Trust posture |
|---|---|
| `~/.rick/*` content (YAMLs, persona) | Trusted as user-authored. Malformed YAML produces a toast, not a crash (NFR-6). |
| `~/.claude/projects/<sid>.jsonl` | Trusted as Claude Code's authoritative output. Malformed lines are skipped silently. |
| Rick's prose / phase markers | Parsed loosely — Rick is allowed to be sloppy. The correlator falls back through 4 ordered passes. See [`../rick-contract/spec.md`](../rick-contract/spec.md). |
| The PTY | Runs whatever `claude` does. The app does NOT sandbox it. |

---

## 9. Cross-References

| Concern | Spec |
|---|---|
| Plugin hooks integration | [`../hooks/spec.md`](../hooks/spec.md) |
| Rick correlator protocol | [`../rick-contract/spec.md`](../rick-contract/spec.md) |
| Settings schema | [`../settings/spec.md`](../settings/spec.md) |

## Source Files

| File | Path |
|---|---|
| Service wiring | `src/main/handlers.ts` |
| Path constants | `src/main/services/paths.ts` |
| IPC channels + API surface | `src/shared/ipc.ts` |
| Domain types | `src/shared/types.ts` |
| Renderer state hook | `src/renderer/src/state.ts` |
| Top-level layout | `src/renderer/src/App.tsx` |

---

## Open Questions

- OQ1: Should `correlateWorkflow` results be memoized? Currently re-parses JSONL on every `listSessionFiles` call.
- OQ2: Do we want a single global file watcher process (instead of three) for resource efficiency?
- OQ3: Should the renderer push state to main (e.g. selected session id) so multi-window setups stay in sync?
