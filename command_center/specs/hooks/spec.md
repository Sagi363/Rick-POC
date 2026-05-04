# Hooks -- Integration Spec

## 1. Overview

The Command Center ships a small Claude Code plugin that installs four hooks into `~/.claude/settings.json`. Each hook is a Node script that runs on a deterministic Claude Code event and updates `~/.rick/tracking/<sid>.md` atomically. Together they make the tracking file the canonical "what's the current state of this session" record without polling Anthropic's API or watching the JSONL ourselves for control state.

**Source files:**
- `plugin/hooks/lib.mjs` -- shared filesystem helpers, atomic R/W, file-lock
- `plugin/hooks/user-prompt-submit.mjs` -- pin workflow on first /rick run prompt
- `plugin/hooks/post-tool-use.mjs` -- recompute todos counts; record subagent spawns
- `plugin/hooks/notification.mjs` -- mark `status: waiting`
- `plugin/hooks/stop.mjs` -- mark `status: done` only on workflow-completion banner
- `plugin/manifest.json` -- hook command templates with `${PLUGIN_DIR}` placeholder
- `src/main/services/install.ts` -- install / uninstall logic

---

## 2. Plugin Layout

### 2.1 Repo paths (source of truth)

| File | Path |
|---|---|
| Library | `plugin/hooks/lib.mjs` |
| UserPromptSubmit | `plugin/hooks/user-prompt-submit.mjs` |
| PostToolUse | `plugin/hooks/post-tool-use.mjs` |
| Notification | `plugin/hooks/notification.mjs` |
| Stop | `plugin/hooks/stop.mjs` |
| Manifest template | `plugin/manifest.json` |

### 2.2 Install paths (what Claude Code runs)

| File | Path |
|---|---|
| Library | `~/Library/Application Support/rick-command-center/plugin/hooks/lib.mjs` |
| (others) | same `plugin/hooks/<event>.mjs` under user data |

The install copies `plugin/` from the app bundle (or repo, in dev) to user data. Each hook entry in `~/.claude/settings.json` is tagged with `# rcc-hook` so install/uninstall can identify our entries.

---

## 3. Hook Contracts

### 3.1 UserPromptSubmit (`user-prompt-submit.mjs`)

| Aspect | Value |
|---|---|
| Fires when | Claude Code receives a user prompt, before sending to model |
| Input | `{session_id, prompt}` JSON via stdin |
| Reads | Tracking frontmatter |
| Writes | `frontmatter.workflow` (if extracted), `frontmatter.status: 'running'` (if not set) |
| Idempotent | Yes -- subsequent prompts don't overwrite existing workflow |

**Detection regex:** `^/rick\s+run\s+([^\s]+)` against the prompt's first line.

### 3.2 PostToolUse:TodoWrite|Task (`post-tool-use.mjs`)

| Aspect | Value |
|---|---|
| Fires when | A tool call finishes; manifest filters to TodoWrite + Task only |
| Input | `{session_id, tool_name, tool_input, tool_response}` |
| Behavior on TodoWrite | Recompute `{total, completed, current}` from new todo list; write rendered checklist to `## Todos` |
| Behavior on Task | Append subagent spawn to `## Artifacts` (deduped by line) |

### 3.3 Notification (`notification.mjs`)

| Aspect | Value |
|---|---|
| Fires when | Claude Code emits a notification (most commonly: needs explicit permission for a tool) |
| Input | `{session_id, ...}` |
| Writes | `frontmatter.status = 'waiting'` |

### 3.4 Stop (`stop.mjs`) -- workflow-aware (CRITICAL)

| Aspect | Value |
|---|---|
| Fires when | Main agent finishes a turn |
| Input | `{session_id, transcript_path, ...}` |
| Behavior | Read latest main-thread assistant message via `transcript_path`; only set `status: done` if its text matches `COMPLETE_RE` |

**Why this matters:** Stop fires after every assistant turn (between phases, after `auto_continue: false` pauses, after every single user message). A naïve "always set status: done" (the original implementation) caused the badge to flip to "done" mid-workflow. The new logic gates on the workflow-completion banner.

**`COMPLETE_RE`** is mirrored from `src/main/services/correlation.ts`:
```
/Rick:\s*\*{0,2}\s*All\s+\d+\s+steps?\s+complete/i
```

If the regex changes in correlation.ts, this hook MUST be updated to match. See [`../rick-contract/spec.md`](../rick-contract/spec.md).

---

## 4. Library Helpers (`lib.mjs`)

| Function | Purpose |
|---|---|
| `readJsonStdin()` | Read the event JSON Claude Code pipes into hook stdin |
| `readTracking(sessionId)` | Atomic read; returns `{frontmatter, sections}` |
| `writeTracking(sessionId, {frontmatter, sections})` | Atomic write (temp + rename); auto-stamps `session_id` and `updated:` |
| `withFileLock(sessionId, fn)` | File-based lock at `<tracking>.lock`; 5s timeout, 10s stale cleanup |
| `summarizeTodos(todos)` | `{total, completed, current}` from a TodoWrite payload |
| `appendArtifact(sections, line)` | Append-once-only to `## Artifacts` |
| `parseFrontmatter` / `serializeFrontmatter` | YAML-style frontmatter R/W; ordered keys, value escaping |
| `parseSections` / `serializeSections` | `## Todos`, `## Phase log`, `## Artifacts` body parser |

---

## 5. Tracking File Schema

```markdown
---
session_id: <uuid>
workflow: <name>
universe: <name>
status: running | waiting | blocked | done
phase: <string>
total: <int>
completed: <int>
current: <string>
started: <iso8601>
updated: <iso8601>
---

## Todos
- [x] something done
- [ ] something pending  ← current

## Phase log

## Artifacts
- subagent: sherlock (...)
- subagent: trinity (...)
```

The frontmatter is YAML-style but parsed permissively (skips bad lines, treats `null`/empty as null). The body has three required H2 sections; missing sections are added empty on first write.

---

## 6. Status State Machine

```
                   UserPromptSubmit
                   sets running ──┐
                                  ▼
                              ┌────────┐
       Notification ──► waiting ◄────► running ◄── recent JSONL activity (<30s)
                              └────────┘
                                  │
                                  │ no recent activity (>30s)
                                  ▼
                              ┌────────┐
       track blocked ──────► blocked   idle
                              └────────┘
                                  │
                                  │ Stop hook AND COMPLETE_RE matches
                                  ▼
                                 done
```

`deriveStatus` (in `src/main/services/sessions.ts`) reads the explicit frontmatter status first; explicit `blocked`/`done`/`waiting` win. Otherwise: JSONL `mtime < 30s` → `running`, else `idle`.

---

## 7. Install Flow

```
App startup
  |
  +-- Read ~/.claude/settings.json
  |
  +-- Look for entries with `# rcc-hook` marker
  |     |
  |     +-- if missing -> show InstallModal with diff
  |           |
  |           +-- on consent:
  |                 - back up settings.json -> settings.json.rcc.bak.<ts>
  |                 - merge our hook entries (substituting ${PLUGIN_DIR})
  |                 - copy plugin/ to ~/Library/Application Support/.../plugin/
  |                 - set settings.pluginInstalled = true
  |
  +-- Uninstall walks settings.json and removes only entries with the # rcc-hook marker
```

---

## 8. Edge Cases

| Case | Behavior |
|---|---|
| Hook script edited in repo, install path is stale | Common dev pitfall. Manual `cp` to install path or re-run install required. |
| Stale lock file | `withFileLock` cleans locks older than 10s automatically. |
| Concurrent hooks (TodoWrite + PostToolUse:Task close together) | The lock serializes them. |
| Missing `transcript_path` in Stop event | Hook falls through to no-op; status stays at previous. |
| Malformed tracking frontmatter | `parseFrontmatter` is permissive; bad lines skipped, doesn't crash hook. |
| Hook fails / throws | Claude Code surfaces to user; tracking may be partially written. |

---

## Open Questions

- OQ1: Should `track blocked <reason>` skill ship as part of v1's plugin? Currently optional (FR-10).
- OQ2: Should the install flow show a "test" button that fires a known-safe event to verify hooks installed correctly?
- OQ3: When the user clears the entire `~/.rick/tracking/` directory, should hooks recreate files or refuse and surface a toast?
