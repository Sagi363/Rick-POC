# Workflow Status Panel -- Feature-Level Contract

---

## Section Visibility

1. The Workflow section SHALL render only when the selected session has both `session.workflow` set AND a discoverable `Workflow` matching that name in the active universe.
2. When the workflow lookup falls back to "any universe with a matching name", the rendering SHALL still proceed (cross-universe match).
3. When no workflow can be found, the section SHALL NOT render and no error SHALL be displayed.

## Step Initialization

4. Every YAML outer step SHALL produce one `WorkflowStepView` row in the UI.
5. The initial status of each step SHALL be `pending`.
6. The initial `files` array SHALL be empty.
7. `currentSubagent` and `currentActivity` SHALL be undefined initially.

## Pass 1 -- Direct Subagent Matching

8. The correlator SHALL iterate every main-thread `Task` tool_use in the JSONL.
9. For each Task, the `subagent_type` SHALL be normalized: if it starts with `rick-`, take the substring after the LAST `-` (e.g. `rick-Issues-Team-sherlock` → `sherlock`).
10. The normalized agent SHALL be matched against `step.agent` (exact) OR `step.collaborators.includes(norm)` OR `norm.endsWith(step.agent)` (in that order).
11. When a step is matched and currently `pending`, its status SHALL flip to `running` (or `done` if the Task itself is complete).
12. When a step is matched and currently `running`, status SHALL flip to `done` only if the Task is complete.
13. `step.startedAt` SHALL be set to the Task's timestamp the first time it is matched.

## Pass 2 -- Phase Markers (overrides Pass 1)

14. The correlator SHALL scan main-thread assistant text for `[rick:phase <step-id> starting|complete]` markers (regex `PHASE_RE`, case-insensitive).
15. Step-id matching SHALL be case-insensitive against the YAML `id`.
16. Markers for unknown step ids SHALL be silently skipped.
17. A `complete` marker SHALL set the step to `done` regardless of prior status.
18. A `starting` marker SHALL set the step to `running` ONLY if it isn't already `done`.
19. When markers apply, `markersApplied = true` and Pass 3 SHALL NOT fire.

## Pass 2.5 -- Phase Prose Fallback

20. Pass 2.5 SHALL fire ONLY when `markersApplied === false`.
21. The correlator SHALL scan main-thread assistant text for `\bPhase\s+(\d+)\s+(starting|started|begins?|done|complete|finished)\b`.
22. The phase number SHALL map to step index `phaseNum - 1`.
23. Out-of-range phase numbers SHALL be silently skipped.
24. After all prose markers are applied, the correlator SHALL find the highest-numbered `complete` and -- when `phaseNum < steps.length` -- SHALL flip step `phaseNum` to `running` ONLY if it is currently `pending`. (Rick often skips "Phase N+1 starting".)

## Pass 3 -- Legacy Announcement Counting

25. Pass 3 SHALL fire ONLY when none of Pass 1, 2, 2.5 produced any signal.
26. `completed = min(completionCount, steps.length)` -- where `completionCount` is the number of `Rick: <agent> is done` matches.
27. Steps `[0..completed)` SHALL be marked `done`.
28. When `inFlight && completed < steps.length`, step `completed` SHALL be marked `running`.

## Live Activity Attribution

29. After all 4 passes complete, the correlator SHALL iterate `steps[]` from highest index to lowest.
30. The first step found with `status === 'running'` SHALL receive `currentSubagent` and `currentActivity` from the latest handoff line.
31. The handoff line is parsed via `HANDOFF_DETAIL_RE` from main-thread assistant text -- the LAST match anywhere in the transcript wins.
32. `currentSubagent` SHALL include the role in parens if a `(Role)` capture group matched: e.g. `"Trinity (Implementor)"`.
33. When no handoff line exists, `currentSubagent` and `currentActivity` SHALL stay undefined.

## File Attribution (Direct Workflows)

34. Each main-thread Task spawn SHALL be associated with its owner uuid (the assistant message containing the tool_use).
35. Each sidechain message's `parentUuid` chain SHALL be walked to find the spawning Task (cached per uuid for performance).
36. Sidechain `tool_use` blocks for `Read` / `Write` / `Edit` / `MultiEdit` / `NotebookEdit` SHALL contribute their file path to the spawning Task's `files` set.
37. The file path SHALL be extracted in priority order: `input.file_path` → `input.path` → `input.notebook_path`.
38. Duplicate file paths within a step SHALL be de-duplicated (Set semantics).

## Rendering

39. Step rows SHALL be ordered by their YAML order (rendered as `1.`, `2.`, `3.`, … prefix).
40. The right-side label SHALL show:
    - `currentSubagent · currentActivity` when status is `running` AND `currentSubagent` is set.
    - `step.agent[+ collaborator + …]` otherwise.
41. The step text SHALL apply `STEP_TEXT_TINT[status]` (zinc-300 / amber-200 / zinc-600).
42. The badge ring color SHALL match status: emerald-`✓` (done), amber pulsing dot (running), grey empty (pending).
43. The file count SHALL render in the right-edge label only when `files.length > 0`.

## Live Refresh

44. The Workflow section SHALL re-fetch via `listSessionFiles(sessionId)` on every change of `session.lastActivity`.
45. The live-refresh effect SHALL NOT force-select the Summary virtual file (only the initial-fetch-on-session-change effect does).
46. Re-fetch SHALL be cancellable -- if the session changes mid-fetch, the response SHALL be ignored (`cancelled` flag).

## Collapse State

47. Section and per-step collapse states SHALL persist in `localStorage[rcc:files:collapsed]` (a key → bool map).
48. Section key SHALL be `wf:<workflow.name>`; step key SHALL be `wf:<workflow.name>:<step.id>`.
49. Steps with zero files SHALL NOT show the chevron and SHALL NOT toggle.

---

## Open Questions

- OQ1: For composed workflows, should files be attributed to outer steps by tracking the active phase marker at write-time?
- OQ2: When two running steps exist (parallel phases), should we attach the latest handoff to ALL of them or just the highest-indexed?
- OQ3: Should we show per-step elapsed time? `startedAt` is captured but not rendered.
- OQ4: Should we show a "stuck" indicator if a running step has no new activity for > 5 minutes?
