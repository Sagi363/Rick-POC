# Settings -- Presentation Spec

## 1. Overview

Modal opened from the gear icon in the top bar. Hosts every persistent app preference. Settings are global (not per-session, not per-workflow) and persist to `~/Library/Application Support/rick-command-center/settings.json` via `SettingsService`.

**Source files:**
- `src/renderer/src/components/SettingsModal.tsx` (full file) -- Modal UI
- `src/main/services/settings.ts` -- read/patch/persist
- `src/shared/types.ts` (lines 65-92) -- `AppSettings` type and `DEFAULT_SETTINGS`
- `src/main/handlers.ts` -- `IPC.GetSettings`, `IPC.SetSettings`

---

## 2. Presentation Models

### 2.1 AppSettings

The full settings shape persisted to disk. Conforms to: structural-only.

| Field | Type | Optional | Default | Notes |
|---|---|---|---|---|
| `warnThreshold` | Number 0..1 | No | `0.70` | Context bar tints amber at this fraction |
| `criticalThreshold` | Number 0..1 | No | `0.90` | Context bar tints rose; OS notification fires |
| `recentSessionDays` | Number | No | `7` | JSONLs older than this are excluded from drawer |
| `lastUniverse` | String | Yes | -- | Top-bar dropdown selection |
| `panelSizes` | Record<string, number> | Yes | -- | Resizable panel widths/heights |
| `pluginInstalled` | Bool | Yes | -- | Set to true after consent screen |
| `archivedSessionIds` | List of String | Yes | `[]` | Discarded session ids |
| `branchPrefix` | String | No | `"feature/"` | Worktree branch auto-prefix |
| `defaultBranchOff` | String | No | `"dev"` | Worktree default base branch |
| `terminalApp` | TerminalApp | No | `"Terminal"` | Where workflow launches go |
| `customTerminalCommand` | String | Yes | -- | Used only when `terminalApp === 'custom'` |
| `skipPermissions` | Bool | No | `false` | Adds `--dangerously-skip-permissions` to claude |
| `customTitles` | Record<sessionId, string> | Yes | -- | Per-session custom titles |

### 2.2 TerminalApp (enumeration)

| Case | Description |
|---|---|
| `in-app` | xterm.js + node-pty inside the app |
| `Terminal` | macOS Terminal.app via AppleScript |
| `iTerm` | iTerm2 via AppleScript |
| `Warp` | Open + clipboard (limited) |
| `Ghostty` | Open + clipboard (limited) |
| `custom` | User-defined `/bin/sh` template with `%cwd%` / `%cmd%` |

---

## 3. Visible Fields

The Settings modal exposes a subset of `AppSettings`. The rest are programmatic.

| Field | UI | Default | Range / Options |
|---|---|---|---|
| Warn threshold | Slider 10–100% | 70% | -- |
| Critical threshold | Slider 10–100% | 90% | -- |
| Recent-session window | Number input | 7 | 1..90 |
| Worktree branch prefix | Text input | `feature/` | -- |
| Default 'branch off' base | Text input | `dev` | Blank uses HEAD |
| Terminal app | Dropdown | `Terminal` | (see 2.2) |
| Custom terminal command | Text input (only when `terminalApp === 'custom'`) | -- | Template with `%cwd%` and `%cmd%` |
| Skip permission prompts | Checkbox | `off` | -- |

## 4. Hidden / Programmatic Fields

| Field | How it changes |
|---|---|
| `lastUniverse` | Top-bar universe dropdown selection |
| `panelSizes` | Drag the resizers between panels |
| `archivedSessionIds` | Click `×` on a session card |
| `customTitles` | Click the title text on a session card |
| `pluginInstalled` | Plugin install consent screen |

---

## 5. Visual States

| State | Appearance |
|---|---|
| Default | Modal centered, 420px wide, all fields populated from current settings |
| Saving | Save button label changes to "Saving…", disabled |
| Custom command shown | Field appears between Terminal app dropdown and skip permissions checkbox |

---

## 6. Interactions

| Target | Gesture | Result |
|---|---|---|
| Threshold sliders | Drag | Local state update; not committed until Save |
| Recent-session window | Type | Local state; min clamps to 1 |
| Branch prefix | Type | Local state |
| Branch off | Type | Local state |
| Terminal app dropdown | Change | Local state; reveals Custom field when "custom" |
| Custom terminal command | Type | Local state |
| Skip permissions | Click | Toggles local state |
| Cancel | Click | Closes without writing |
| Save | Click | Calls `setSettings(patch)`, closes on success |

---

## 7. Save Flow

```
User clicks Save
  |
  +-- patch = {
  |     warnThreshold: warn / 100,
  |     criticalThreshold: crit / 100,
  |     recentSessionDays: days,
  |     branchPrefix,
  |     defaultBranchOff,
  |     terminalApp,
  |     customTerminalCommand: trim || undefined,
  |     skipPermissions
  |   }
  |
  +-- setSettings(patch) [IPC]
  |     |
  |     +-- main: settings.patch(patch) -> deep-merges into current
  |     +-- main: writes settings.json atomically
  |     +-- returns the full new AppSettings
  |
  +-- onSave(patch) -> useAppState updates settings
  +-- onClose()
```

---

## 8. Settings → Behavior Map

| Setting | Where consumed |
|---|---|
| `warnThreshold`, `criticalThreshold` | SessionCard context-bar tone |
| `recentSessionDays` | `SessionsService.recompute()` cutoff |
| `branchPrefix`, `defaultBranchOff` | LaunchModal worktree-mode defaults |
| `terminalApp`, `customTerminalCommand` | `WorkflowLauncher` spawn target |
| `skipPermissions` | Launcher appends `--dangerously-skip-permissions` to claude |
| `archivedSessionIds` | `SessionsService` filters them out |
| `customTitles` | `SessionsService` overrides auto-derived title; bleeds forward to successors |
| `lastUniverse` | TopBar selection + universe filter on sessions |
| `panelSizes` | App layout default widths/heights |

---

## Open Questions

- OQ1: Should `criticalThreshold` be enforced ≥ `warnThreshold` in the slider's min? Currently no enforcement.
- OQ2: Should there be a "Reinstall hooks" button? Currently the only way to reinstall is to delete the `# rcc-hook` entries from `~/.claude/settings.json` and restart the app.
- OQ3: Should panel-size persistence be per-window-size (so the layout adapts as the user resizes the OS window)?
- OQ4: Should we expose per-workflow defaults (e.g. terminalApp per workflow)?
