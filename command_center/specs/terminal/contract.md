# Terminal -- Feature-Level Contract

Cross-component business rules for the embedded terminal panel and its quick-command toolbar.

---

## Panel Visibility

1. The TerminalsPanel SHALL render when `sessionPtys.length > 0` for the selected session.
2. When TerminalsPanel renders, the ProgressPanel for the same selected session SHALL be hidden.
3. When `sessionPtys.length === 0`, ProgressPanel SHALL be shown instead and TerminalsPanel SHALL NOT render.
4. The user-controlled "collapsed" state of the terminal SHALL be local renderer state -- not persisted across app launches.

## PTY Lifecycle

5. Every workflow launch with `terminalApp === 'in-app'` SHALL spawn a new PTY (no pooling, no reuse).
6. A PTY SHALL inherit the user's shell, locale, and PATH from `process.env`.
7. When a PTY's child process exits, its `alive` SHALL flip to `false` and an `IPC.PtyExit` event SHALL push to renderer.
8. `ptyKill(id)` SHALL send `SIGTERM` to the child; if still alive after 2s, `SIGKILL`.
9. Closing the panel SHALL NOT kill PTYs; tabs persist until the user clicks `×` or the child exits.

## Auto-Bind by cwd

10. When `TranscriptService` discovers a new session id with a `cwd`, the main process SHALL search alive PTYs for one with no `sessionId` and matching `cwd`.
11. If exactly one match is found, the PTY's `sessionId` SHALL be set via `bindSession(handleId, sessionId)`.
12. If multiple unbound PTYs match, only the first encountered SHALL be bound -- the rest stay unbound.
13. Once bound, `sessionId` SHALL NOT be reassigned -- a single PTY belongs to one session for life.

## Tab Bar

14. The tab strip SHALL render one tab per `PtyInfo` in the panel's `ptys` prop.
15. Tabs SHALL be ordered by their order in the prop (most-recently-spawned last).
16. Each tab SHALL show an emerald `●` when alive and a rose `●` when dead.
17. Dead tabs SHALL render at 60% opacity but remain clickable.
18. Hovering a tab SHALL reveal a `×` close icon that calls `ptyKill(id)`.

## Active PTY Selection

19. On mount with non-empty `ptys`, `activeId` SHALL prefer a PTY whose `sessionId === selectedSessionId`; else the last PTY in the array.
20. When `selectedSessionId` changes AND a PTY exists for that session, `activeId` SHALL snap to it.
21. When the current `activeId` is no longer in `ptys` (PTY removed), the panel SHALL re-pick using rule 19.
22. When `ptys` becomes empty, `activeId` SHALL become `null` (panel falls back to ProgressPanel via rule 3).

## Quick Commands -- General

23. All three quick-command buttons SHALL be disabled when there is no active alive PTY (`activePty?.alive` is false/undefined).
24. Disabled buttons SHALL show a generic tooltip: `"No active terminal — open the in-app terminal for this session first"`.
25. Each button SHALL append `\r` (carriage return) to its command before writing -- terminating with `\r` submits in the Claude REPL.
26. Buttons SHALL write via `window.rcc.ptyWrite(activeId, data)` -- the panel does not handle PTY directly.

## Quick Command -- /rick next

27. The button label SHALL display `/rick next` and the command sent SHALL be `/rick next\r`.
28. The button tone SHALL be `emerald` (primary action).
29. The button SHALL be enabled only when there is an alive active PTY AND `sessionStatus ∈ {idle, waiting}`.
30. When disabled because of session status, the tooltip SHALL be specific:
    - `running` → `"Rick is running — wait until the current step finishes"`
    - `done` → `"Workflow is already complete"`
    - `blocked` → `"Session is blocked — resolve the blocker first"`
31. The rationale for status-gating is: firing `/rick next` mid-execution stacks a prompt that disrupts the current turn (Rick reads it once it pauses, but timing interleaves badly with auto-continue).

## Quick Command -- /rick status

32. The button label SHALL display `/rick status`.
33. The actual command sent SHALL be `/btw rick status\r` -- the `/btw` prefix is intentional, so this query does NOT interrupt Rick's current turn.
34. The button tone SHALL be `zinc` (neutral).
35. The button SHALL be enabled whenever there is an alive active PTY -- no status gating.

## Quick Command -- /clear

36. The button label SHALL display `/clear`.
37. The button tone SHALL be `amber` (warning).
38. Clicking SHALL NOT send `/clear` directly -- it SHALL open the ConfirmClearModal.
39. The modal SHALL show backdrop click-to-cancel.
40. Clicking Cancel or the backdrop SHALL close the modal and send NOTHING.
41. Clicking "Yes, send /clear" SHALL close the modal and write `/clear\r` to the active PTY.
42. The modal copy SHALL include both:
    - the cost (current context window is lost, in-flight reasoning irrecoverable)
    - the safety net (successor-session detection in same cwd within 1 hour, custom title bleeds forward)

## ConfirmClearModal

43. The modal SHALL render as `position: fixed inset-0 z-50` with `bg-black/60` backdrop.
44. The card SHALL be amber-bordered (`border-amber-700`) -- not rose, because `/clear` is recoverable when the user has been doing work in trackable form (tracking files, git commits).
45. The "Yes, send /clear" confirm button SHALL be `bg-amber-600` (warning tone, not destructive-rose).

## xterm Wiring

46. Each Terminal component SHALL bind to one `ptyId` and only render its content when `active`.
47. Inactive Terminal components SHALL stay mounted but `display: none` so xterm scrollback is preserved across tab switches.
48. The xterm `@xterm/addon-fit` SHALL drive resize: `pty.resize(id, cols, rows)` on layout changes.
49. PTY → xterm data flow: `IPC.PtyData {id, chunk}` push → renderer routes to matching Terminal component → xterm `write(chunk)`.
50. xterm → PTY input flow: xterm `onData(data)` → `window.rcc.ptyWrite(id, data)` → main writes to PTY stdin.

---

## Open Questions

- OQ1: Should dead PTY tabs auto-evict after N seconds? Currently they persist forever.
- OQ2: Should there be a keyboard shortcut for the QuickCmd buttons (e.g. `Cmd+Enter` for /rick next when focused)?
- OQ3: When the user has a single alive PTY, can we elide the tab strip entirely?
- OQ4: Should the panel's collapsed/expanded state persist per-session, not just globally?
