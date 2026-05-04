# Sessions Panel -- Feature-Level Contract

Cross-component business rules for the Sessions panel.

---

## Status Derivation

1. The status of a session SHALL be derived in this order: explicit `blocked` from frontmatter wins; explicit `done` from frontmatter wins; explicit `waiting` from frontmatter wins; otherwise `running` if JSONL `mtime` is within 30s; otherwise `idle`.
2. The 30s "running" threshold is constant `RUNNING_THRESHOLD_MS = 30_000` -- not configurable in v1.
3. When a session has no transcript yet (tracking-only), status SHALL fall through to `idle` regardless of `lastActivity`.

## Listing & Visibility

4. A session SHALL be listed when (a) its transcript is in `~/.claude/projects/` and was modified within `recentSessionDays` (default 7) OR (b) it has a tracking file in `~/.rick/tracking/`.
5. Sessions whose transcript is older than the cutoff but whose tracking is recent SHALL still appear (tracking presence wins).
6. Sessions in `archivedSessionIds` SHALL be excluded from the rendered list.
7. When `lastUniverse` is set in settings, sessions whose `universe` doesn't match SHALL be excluded; sessions with no universe binding pass through.

## Sorting

8. The list SHALL be sorted by `lastActivity` descending (most recent first).
9. The currently-selected session SHALL be pinned to the top of the visible list, regardless of its lastActivity rank.
10. When the selected session is pinned and other sessions remain visible, a `— others —` separator SHALL be inserted between them.

## Title Derivation

11. When a session has no `customTitle`, its `title` SHALL be derived from workflow params in this priority order: `ticket_key`, `ticket`, `feature`, `job`, `name`, `feature_name`, then any string-typed param value.
12. When no params are present, the title SHALL be empty (renders as `session <8-char-id>` placeholder).
13. When the user sets a custom title, it SHALL persist in `settings.customTitles[sessionId]` and override the derived value.
14. The ✎ marker SHALL appear next to the title when `customTitle` is set.
15. Clearing a custom title input (empty trimmed) SHALL remove the entry from `settings.customTitles`, reverting to the derived title.

## Search

16. Search SHALL be case-insensitive substring match.
17. Search SHALL match against `session.title`, `session.workflow`, `session.cwd`, and `session.id`.
18. Search and status-filter chips SHALL be combined with AND -- a card must satisfy both to be visible.
19. Empty search SHALL match all sessions.

## Status Filter Chips

20. All five status chips (`running`, `waiting`, `blocked`, `idle`, `done`) SHALL be enabled by default.
21. When a chip is disabled, sessions with that status SHALL be hidden.
22. Chip state is local to the panel session -- not persisted across app launches.

## Discard

23. Clicking `×` on a session card SHALL show a `window.confirm` dialog.
24. On confirm, the tracking file SHALL be deleted (or the unlink failure ignored when the watcher reaps it first).
25. The session id SHALL be added to `settings.archivedSessionIds` -- idempotent (no duplicates).
26. The discarded session SHALL be removed from the visible list immediately on success.

## Predecessor / Successor Linking

27. When two sessions share a `cwd` and the newer session's start (or `lastActivity` if `startedAt` absent) is within 1 hour (`CONTINUATION_WINDOW_MS = 60 * 60_000 ms`) of the older session's `lastActivity`, they SHALL be linked.
28. The older session SHALL be force-promoted to `status: done` UNLESS its current status is `blocked` (which wins).
29. The older session's `successorId` SHALL be set to the newer session's id.
30. The newer session's `predecessorId` SHALL be set to the older session's id.
31. When the older session has a `customTitle` and the newer session has neither `customTitle` nor a derived `title`, the older session's `customTitle` SHALL be inherited as the newer session's `title`.
32. Clicking the `→ continued at` chip SHALL switch selection to the successor and clear the file selection.
33. The chip SHALL NOT auto-spawn a terminal for the successor -- the user must use ⤴ to open one.

## Focus Terminal Action

34. The ⤴ icon SHALL be visible only when `session.cwd` is non-empty.
35. Clicking ⤴ SHALL call `focusTerminal({cwd, sessionId, terminalApp, skipPermissions})`.
36. When `terminalApp` is `Terminal` or `iTerm`, behavior SHALL be: search for an existing window in `cwd` via AppleScript; if found bring to front; if not spawn a fresh tab with `claude --resume <id>`.
37. When `terminalApp` is `Warp` or `Ghostty`, behavior SHALL be: open the app via `open -a` and copy the resume command to the clipboard.
38. When `terminalApp` is `in-app`, behavior SHALL be: switch the in-app TerminalsPanel `activeId` to the matching PTY (auto-bound by cwd if necessary).
39. When `terminalApp` is `custom`, behavior SHALL be: spawn the user's command template substituting `%cwd%` and `%cmd%`.

## Auto-Continue Pill

40. The pill SHALL appear on every session card when `onSetAutoContinue` is provided.
41. The pill state SHALL be persisted per-session in `localStorage[rcc:session:auto-continue][sessionId]`.
42. Initial state SHALL default to `false` when no entry exists.
43. Clicking the pill SHALL flip the state, persist immediately, and call `onSetAutoContinue(newState)`.
44. The handler in `App.tsx` SHALL find the alive PTY for the session by `pty.sessionId === sessionId && pty.alive`.
45. When no alive PTY exists, the handler SHALL emit a toast: `"No active terminal for this session — open the in-app terminal first."` and skip the directive.
46. When a PTY is found, the handler SHALL write to it the directive corresponding to the new state, terminated by `\r`:
    - ON: `/btw From now on, run remaining phases with auto_continue: true — do not pause between phases or wait for me to say next. Drive the workflow end-to-end.`
    - OFF: `/btw From now on, run remaining phases with auto_continue: false — pause after each phase and wait for my next before continuing.`
47. The pill state SHALL reflect "what was last commanded" -- it does not parse Rick's acknowledgement.

## Context Window Bar

48. The bar SHALL be hidden when `session.context` is null/undefined.
49. The bar fill width SHALL be `min(100, used / limit * 100)` percent.
50. The bar fill color SHALL be: emerald when `pct < warnThreshold`, amber when `warnThreshold <= pct < criticalThreshold`, rose when `pct >= criticalThreshold`.
51. The numeric label SHALL render as `<formatTokens(used)> / <formatTokens(limit)>` followed by `<round(pct*100)>%`.
52. When `context.modelKnown` is false, a `?` SHALL appear inline next to the limit with a tooltip "Unknown model — assumed 200k".

## Tab Persistence

53. The Sessions / Workflows tab selection SHALL be local panel state -- not persisted across app launches.

## Empty State

54. When no sessions match the active filters (or no sessions exist), the panel SHALL show a hint message instead of an empty list.

---

## Open Questions

- OQ1: Should the panel surface "stale" sessions (transcript inside cutoff but no activity for hours) differently from `idle`?
- OQ2: Should auto-continue pill state migrate forward to a successor session via the link mechanism, like custom titles do?
- OQ3: When the user discards a session that has a successor link, does the successor's predecessor link become stale? Currently yes — it points at a now-archived id.
