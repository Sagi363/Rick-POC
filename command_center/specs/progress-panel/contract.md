# Progress / Activity Panel -- Feature-Level Contract

---

## Mutual Exclusion with Terminal

1. The Progress panel SHALL NOT render when `sessionPtys.length > 0` for the selected session.
2. When `sessionPtys.length === 0`, the Progress panel SHALL render in the bottom-strip slot of the main pane.
3. Mutual exclusion is enforced in `App.tsx` (the `hasPtys` branch), not in the panel itself.

## Mode Selection

4. The panel SHALL show Mode A (tracked progress) when ANY of `session.total`, `session.phase`, `session.current` is set.
5. The panel SHALL show Mode B (activity feed) when NONE of those three fields is set.
6. Mode is recomputed every render -- no caching of mode decision.

## Mode A Rendering

7. The header SHALL display the label `"Progress"`.
8. When `session.phase` is set, a chip `phase: <name>` SHALL appear in the header.
9. When `session.total > 0`, an emerald progress bar SHALL render: width = `<completed>/<total> * 100%`.
10. The progress bar SHALL show `<completed> / <total>` on the left and `<round(pct*100)>%` on the right.
11. When `session.current` is set, the line `→ <current>` SHALL render below the progress bar.
12. The body SHALL be the result of `readTracking(session.id)` with the frontmatter (`---...---`) stripped.
13. The stripped body SHALL be rendered via `react-markdown` with `remark-gfm`.
14. When the body is empty after stripping, the panel SHALL display "Tracking exists but body is empty."

## Mode B Rendering

15. The header SHALL display the label `"Activity"`.
16. The body SHALL be a flat list of up to 12 most-recent main-thread events from the JSONL.
17. Events SHALL be ordered chronologically (oldest first within the 12 shown).
18. Each event SHALL render as `<HH:MM:SS local> [<kind>] <text>` (single line, truncated at 240 chars).
19. Event kind SHALL be:
    - `user` -- when `type === 'user'` and content has text.
    - `reply` -- when `type === 'assistant'` and content has text.
    - `tool` -- when `type === 'assistant'` and content has tool_use with `name !== 'Task'`.
    - `agent` -- when `type === 'assistant'` and content has tool_use with `name === 'Task'`.
20. Sidechain messages (subagent-internal turns) SHALL be excluded.
21. Each kind SHALL be color-coded: blue (user), emerald (reply), amber (tool), purple (agent).

## Loading & Empty States

22. While Mode B's summary fetch is in flight, the body SHALL display "Reading transcript…".
23. While Mode A's body fetch is in flight, no loading indicator is shown -- the body is small and reads fast.
24. When no session is selected, the body SHALL display "Pick a session.".
25. When Mode B has zero events, the body SHALL display "No activity yet.".

## Re-fetch Triggers

26. The body SHALL re-fetch on each of:
    - `session.id` change.
    - `session.lastActivity` change.
    - `collapsed` flips from true to false.
27. When `collapsed` is true, both `body` and `summary` SHALL be cleared.
28. Stale-response protection: a `cancelled` flag SHALL ensure that a slower fetch arriving after a session change does NOT setBody/setSummary.

## Mode B Optimization

29. `getSessionSummary(session.id)` SHALL be called ONLY in Mode B (when `hasTrackedProgress === false`).
30. In Mode A, `summary` SHALL be set to null to free memory.

## Collapsed State

31. The panel SHALL render a one-line strip when `collapsed === true`: `Activity hidden` + `▲ Show`.
32. The collapsed flag is local renderer state -- not persisted across app launches.
33. Toggling collapsed while Mode A renders SHALL clear the body and re-fetch on expand (rule 26).

---

## Open Questions

- OQ1: Should activity-feed cap (12) be configurable?
- OQ2: When Mode A has both `total` and recent activity, should there be a way to see the Mode B feed without temporarily corrupting tracking data?
- OQ3: Should Mode A live-render task-list checkboxes as interactive (user can check items, persisting back to tracking.md)?
