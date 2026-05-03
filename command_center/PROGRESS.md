# Rick Command Center — Progress & Pickup Notes

**Owner:** Dekel Maman
**Repo cwd:** `/Users/mamand/Development/SDDeditor`
**App user-data:** `~/Library/Application Support/rick-command-center/`
**Spec:** see `PRD.md` (still authoritative)

This doc is the safety net for `/clear` — read it after a clear to recover where we are without re-reading the whole transcript.

---

## Project state

**M1 (MVP viewer): COMPLETE.** Universe/workflow discovery, JSONL tail-and-parse, tracking.md atomic R/W, sessions aggregator, drawer + universe switcher + file list + file preview + activity panel, settings persistence, hooks bundle (`UserPromptSubmit` / `PostToolUse:TodoWrite|Task` / `Notification` / `Stop`), plugin install flow with consent diff. All shipped and tested end-to-end.

**M2 (Terminal + Launch): MOSTLY COMPLETE.**
- ✅ Workflow Library tab with cards (name, description, agents, params, search filter)
- ✅ Launch modal with auto-form generated from YAML `params:` (string/int/bool/enum/required validation)
- ✅ Recent-project chips + datalist typeahead + Browse picker
- ✅ Worktree mode (creates `<base>/.claude/worktrees/<name>` + `git worktree add -b <branch>`)
- ✅ Reuse-existing-worktree fallback (when path exists, big "Use it & launch →" button)
- ✅ External terminal launchers: Terminal.app + iTerm2 (full inline) + Warp/Ghostty (open + clipboard) + custom command
- ✅ Embedded **in-app terminal** (xterm.js + node-pty); pick "In-app terminal (xterm.js)" in Settings
- ✅ Per-session terminal filtering (one tab per session); auto-bind unbound PTYs to new sessions by cwd match
- ✅ Focus button (⤴) — Terminal/iTerm: AppleScript find by cwd; otherwise (or when not found) spawn fresh tab with `claude --resume <id>`
- ⬜ Nothing strictly required left for M2 exit criteria

**M3 (Polish): NOT STARTED.** OS notifications, agent persona viewer, global search, per-session threshold overrides.

---

## 2026-05-03 session — correlation + UX + native packaging

### Correlator: composed-workflow support (`uses:` chains)

The original Pass-1 (subagent-name match) + Pass-2 (announcement counting) couldn't handle `uses:` workflows like `bug-fix-from-jira` — `step.agent` is a sub-workflow name (`ticket-research`/`bug-fix`/`test-and-validate`) that never appears as a `subagent_type`, and counting Rick's inner `Handing to …` lines wildly over-attributes progress (one outer step contains many inner handoffs).

**Solution: 4 ordered passes in `src/main/services/correlation.ts`**:

1. **Direct subagent matching** (existing) — works for direct-agent workflows like specter.
2. **Phase markers** (new) — Rick emits `[rick:phase <step-id> <starting|complete>]` on outer-step boundaries. Wins over Pass 1 wherever both apply because markers are explicit.
3. **Phase prose fallback** (new) — natural-language `Phase 1 done` / `Phase 2 starting` patterns, mapped by phase number → step index. Handles legacy Rick personas that haven't adopted the bracket marker yet. Negative-tense forms (`Phase 2 will`, `Waiting on Phase 2`, `Phase 2 = reproduce`) are rejected by the regex.
4. **Announcement counting** (existing, demoted) — last-resort heuristic. Only fires when none of 1-3 produced a signal.

`~/.rick/persona/rules.md` updated with the marker contract Rick must follow. See `specs/rick-contract/spec.md` (overview) and `specs/rick-contract/contract.md` (numbered regex rules) for the full spec.

### Live sub-activity on the running step

`WorkflowStepView` now carries `currentSubagent?: string` and `currentActivity?: string`. The correlator parses the most recent `**Rick:** Handing to **<Subagent>** (<Role>) — <activity>` line on the main thread and attaches it to the highest-indexed running step. The renderer (FileList → WorkflowSection) shows `<Subagent> · <activity>` in place of the generic agent/sub-workflow name while running; non-running steps keep the original label.

### Stop hook — workflow-aware

`plugin/hooks/stop.mjs` no longer unconditionally writes `status: done` on every Stop event (which fired between phases too, poisoning the badge). Now reads the latest main-thread assistant message via `ev.transcript_path` and only writes `done` when `Rick: All N steps complete` is present. Mirrors `COMPLETE_RE` in correlation.ts so the two stay in sync.

The hook is installed at `~/Library/Application Support/rick-command-center/plugin/hooks/stop.mjs` (copy of `plugin/hooks/stop.mjs`). Re-install or copy manually after edits.

### Live refresh on `lastActivity`

`FileList.tsx` had only `[sessionId]` in its useEffect deps, so workflow rings + tracking + touched-files only refreshed when you switched sessions. Added a second effect on `[sessionLastActivity]` that re-fetches `listSessionFiles` without forcing a Summary re-select. `App.tsx` passes `selected?.lastActivity` through. Cost: one JSONL re-parse per transcript write — fine for current sizes.

### Quick-cmd buttons in terminal toolbar

Three buttons next to "Hide" in `TerminalsPanel`:

- `▶ /rick next` (emerald) — sends `/rick next\r`. **Gated**: only enabled when `sessionStatus === 'idle' || 'waiting'` and there's an alive PTY. Tooltip explains specifically why it's disabled (running / done / blocked / no terminal).
- `▶ /rick status` (zinc) — label says `/rick status` but actually sends `/btw rick status\r` so it doesn't interrupt Rick's flow.
- `▶ /clear` (amber) — opens an amber-bordered confirm modal explaining context loss before sending. Cancel = no-op.

### Auto-continue toggle (launch + drawer)

**Launch modal** (`LaunchModal.tsx`): new checkbox above Extra prompt. Default = last user choice (`localStorage` key `rcc:launch:auto-continue`). When ON, prepends an explicit override directive to the extra prompt before submitting:

> `Override: run all phases with auto_continue: true — do not pause between phases or wait for me to say next. Drive the workflow end-to-end.`

**Session card** (`SessionCard.tsx`): small `auto on` / `auto off` pill on the status row of every card. Click flips state (per-session, persisted to `rcc:session:auto-continue` map) and sends a `/btw …` directive to the active in-app PTY. No PTY → toast asking to open the terminal. The pill reflects "what you last commanded," not what Rick is actually doing — there's no acknowledgement parsing yet.

### Dev workflow: stacking instances fix

Each `npm run dev` was spawning a fresh `electron-vite` + Electron app stack without killing prior ones — six Electron icons in Force Quit. Added:

- `npm run kill` — `pkill -f` on `node_modules/.bin/electron-vite` and `SDDeditor/node_modules/electron/dist/Electron.app` (narrowly scoped so it doesn't touch Cursor / ChatGPT / etc.).
- `npm run predev` — same kill, runs automatically before every `npm run dev`. So the dev workflow self-cleans.

### Native packaging — self-signed `.dmg`

Added `npm run package:dmg`:

```
npm run kill && CSC_IDENTITY_AUTO_DISCOVERY=false electron-vite build && CSC_IDENTITY_AUTO_DISCOVERY=false electron-builder --mac --arm64
```

`package.json` `build.mac` config: `identity: null` (ad-hoc signing), `gatekeeperAssess: false`, removed `hardenedRuntime` (incompatible with ad-hoc). Output lands at `release/Rick Command Center-0.1.0-arm64.dmg` (~97 MB). First launch on any machine: Right-click → Open to bypass the unidentified-developer warning. arm64-only — Intel build needs `--x64` (separate run, since native deps need rebuilding per arch).

Recipient still needs `claude` CLI, `git`, and a populated `~/.rick/` on their own machine.

---

## Earlier decisions (M2 wrap-up)

- **Workflow detection** — two-pronged: `/rick run <name>` in any user prompt AND Rick's mandatory `Running **<name>**` assistant text. First match wins, sticks for the session. Catches both explicit invocations and natural-language workflow starts.
- **Workflow step correlation** — direct: match Task `subagent_type` (with `rick-<universe>-` prefix stripped) to `step.agent`/`collaborators`. Fallback for `uses:` composed workflows: count Rick's `Handing to ...` and `... is done` announcements in main-thread assistant text. Steps without explicit agent matching fill from announcement count.
- **Step visuals** — real circles: emerald w/ ✓ (done), amber w/ pulsing dot (running), empty gray ring (pending). Step text amber for running, dim for pending.
- **Editable session title** — auto-derived from params (`ticket_key`/`feature`/`ticket`/`job`/`name`/first string). Click to rename; ✎ pin shows custom title is set; clear field to revert to auto.
- **Pinned selection** — selected card stays at top of drawer with `— others —` separator below.
- **Status filter chips** — toggle running/waiting/blocked/idle/done in drawer.
- **Discard** — × button on card hover; deletes tracking file + adds to archived list (persisted in settings).
- **Predecessor/successor linking** — when a new session appears in the same cwd within 1hr of a previous one's last activity (i.e., `/clear` flow), the old card is marked `done` + an amber **"→ continued at <id>"** chip lets the user jump to the successor. The user's customTitle bleeds into the successor when the successor has no title.
- **Context window auto-bump** — observed usage > limit triggers promotion to 1M (handles `opus[1m]` sessions where the JSONL only records bare `claude-opus-4-7`).
- **Terminal picker** — Settings → "Terminal app": in-app / Terminal / iTerm / Warp / Ghostty / custom (`%cwd%`/`%cmd%` template).
- **`--dangerously-skip-permissions`** — Settings checkbox; appended to `claude` invocations from app launches.
- **Worktree branch defaults** — Settings: `branchPrefix` (default `feature/`) and `defaultBranchOff` (default `dev`). Branch input prefilled accordingly.
- **Recent activity hidden when terminal active** — bottom Activity panel hard-hides when any PTY exists for the selected session. Summary view's Recent Activity subsection removed (was duplicating the bottom panel).
- **Specs are strict** — only `PRD.md`, `requirements.md`, `design.md`, `tasks.md`, `research.md`, `acceptance*.md`, `spec.md`, `*.spec.md`, plus anything inside `specs/`, `.claude/specs/`, `.rick/specs/`, etc. README/CLAUDE/AGENTS no longer included.

---

## Known issues / open items

- **`node-pty` rebuild** runs on `npm install` via `postinstall` script. If install ever fails on rebuild, run `npm run rebuild-native` manually.
- **Warp/Ghostty inline launch is limited** — opens the app + copies command to clipboard; user pastes. AppleScript can't drive them. Acceptable for now.
- **Successor session selection** — clicking the "→ continued" chip switches the drawer selection but does NOT auto-spawn a terminal for the new session. User clicks ⤴ to open one.
- **No correlation cache** — `correlateWorkflow` re-parses the JSONL every time `listSessionFiles` is called. Fine for current sizes; consider memoizing if perf bites.
- **Hooks installed in `~/.claude/settings.json`** — current install merged into that file. Backup at `~/.claude/settings.json.rcc.bak.<ts>`. Uninstall removes only entries whose command contains the `# rcc-hook` marker.

---

## Key paths

- Renderer: `src/renderer/src/`
  - `App.tsx` — layout (drawer, file list, main pane with summary/preview, terminal, activity)
  - `components/` — Drawer, SessionCard, WorkflowCard, FileList, FilePreview, SessionSummary, ProgressPanel, TerminalsPanel, Terminal, LaunchModal, SettingsModal, InstallModal, Resizer, TopBar
  - `state.ts` — useAppState hook with IPC subscriptions
- Main: `src/main/`
  - `index.ts` — Electron bootstrap
  - `handlers.ts` — IPC handlers + service wiring (incl. PTY auto-bind by cwd)
  - `services/` — universes, transcripts, tracking, sessions, summary, correlation, specs, settings, install, launcher, pty
- Shared: `src/shared/`
  - `types.ts` — all DTOs and AppSettings
  - `ipc.ts` — channel names + RccApi interface
- Plugin: `plugin/`
  - `manifest.json` — hook command templates with `${PLUGIN_DIR}` placeholder
  - `hooks/lib.mjs` + per-event scripts (no external deps)

---

## How to run

```
cd /Users/mamand/Development/SDDeditor
npm run dev
```

Dev hot-reloads renderer; main-process changes need restart.

```
npm run typecheck   # both tsconfigs
npm run build       # full electron-vite build
npm run package     # signed .dmg (electron-builder)
npm run rebuild-native  # rebuild node-pty for current Electron
```

---

## Suggested next steps after /clear

1. **Test M2 end-to-end** with a real `/rick run` workflow (specter or bug-fix) — verify worktree creation, in-app terminal, step correlation, successor linking. Look for surprises before moving on.
2. **M3 polish** if M2 feels solid:
   - OS notifications (waiting / blocked / critical context / done)
   - Agent persona viewer (read `~/.rick/universes/<name>/agents/<name>/soul.md` etc.)
   - Global search across sessions/workflows/agents
   - Per-session threshold overrides
3. **Distribution** — `npm run package` produces a `.dmg`; sign + notarize via electron-builder mac config.
