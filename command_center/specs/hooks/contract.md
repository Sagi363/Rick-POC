# Hooks -- Integration Contract

Numbered rules each hook script SHALL follow.

---

## All Hooks

1. Each hook SHALL exit with code 0 even on internal errors -- Claude Code surfaces non-zero codes to the user as failures.
2. Each hook SHALL read the event payload via `readJsonStdin()` and exit early when `session_id` is missing.
3. Each hook SHALL acquire `withFileLock(sessionId, fn)` before reading-then-writing tracking.
4. Hook scripts SHALL only write to `~/.rick/tracking/<sessionId>.md`. They SHALL NOT modify any file under `~/.rick/state/` or `~/.rick/universes/`.
5. The lock file (`<tracking>.lock`) SHALL be cleaned up on success and on error.

## UserPromptSubmit

6. The hook SHALL match the prompt against `^/rick\s+run\s+([^\s]+)` (multiline-mode) and extract the workflow name.
7. When the regex matches AND `frontmatter.workflow` is unset, the workflow name SHALL be written.
8. When the regex matches AND `frontmatter.workflow` is already set, the existing value SHALL be preserved (idempotent).
9. When `frontmatter.status` is unset, it SHALL be set to `"running"`.
10. When `frontmatter.status` is already set, it SHALL be left unchanged.

## PostToolUse (TodoWrite)

11. The hook SHALL parse the TodoWrite payload to extract the todo list.
12. `frontmatter.total` SHALL be the array length.
13. `frontmatter.completed` SHALL be the count of items with `status === 'completed'`.
14. `frontmatter.current` SHALL be the `subject`/`content`/`text` of the first item with `status === 'in_progress'` (or undefined when none).
15. The `## Todos` section body SHALL be replaced with the rendered checklist (`- [x] subject` / `- [ ] subject  ← current` for in-progress).

## PostToolUse (Task)

16. The hook SHALL append a single line to `## Artifacts` of the form `subagent: <subagent_type> (<description>)`.
17. Duplicate lines (exact string match) SHALL NOT be appended a second time.
18. The hook SHALL NOT modify any frontmatter field other than `updated:` (auto-stamped by `writeTracking`).

## Notification

19. The hook SHALL set `frontmatter.status = 'waiting'` on every Notification event.
20. The hook SHALL NOT modify any other frontmatter field.

## Stop -- Workflow-aware Completion

21. The hook SHALL exit early (no-op) when `transcript_path` is missing from the event payload.
22. The hook SHALL read the transcript file at `transcript_path` and walk lines from end to beginning.
23. The first non-empty, non-sidechain, `type === 'assistant'` line found SHALL be tested.
24. The test SHALL apply `COMPLETE_RE` against any text content blocks of that message.
25. When the regex matches, `frontmatter.status` SHALL be set to `"done"`.
26. When the regex does NOT match, the hook SHALL exit without writing.
27. `COMPLETE_RE` SHALL be `/Rick:\s*\*{0,2}\s*All\s+\d+\s+steps?\s+complete/i` -- matched against `correlation.ts` exactly.

## Atomic Write

28. `writeTracking` SHALL write to `<path>.<pid>.<ts>.tmp` then `rename` -- never partial writes.
29. `writeTracking` SHALL set `frontmatter.session_id` (re-set every write) and `frontmatter.updated` (current ISO8601).
30. When `frontmatter.started` is unset, `writeTracking` SHALL set it to the current `updated` value.
31. The frontmatter key order on serialization SHALL be: `session_id`, `workflow`, `universe`, `status`, `phase`, `total`, `completed`, `current`, `started`, `updated`, then any other keys.
32. Values containing `:`, `#`, `"`, or newlines SHALL be JSON-stringified to escape them.

## File Lock

33. `withFileLock` SHALL attempt to acquire `<tracking>.lock` exclusively (`open(path, 'wx')`).
34. On `EEXIST`, SHALL retry every 30ms for up to 5000ms total.
35. When the lock file's mtime is older than 10000ms, SHALL be considered stale and unlinked before retrying.
36. On 5000ms timeout, SHALL fall through and run `fn()` WITHOUT a lock as a last-resort fallback.
37. On `fn()` completion or error, SHALL unlink the lock (failures ignored).

## Install Flow

38. The install SHALL detect existing hook entries by the `# rcc-hook` marker substring in the command.
39. The install SHALL back up `~/.claude/settings.json` to `~/.claude/settings.json.rcc.bak.<ts>` before any modification.
40. The install SHALL substitute `${PLUGIN_DIR}` in the manifest with the actual user-data plugin path.
41. The install SHALL be idempotent -- re-running with hooks already present is a no-op (the consent screen still shows the diff = "no changes").
42. Uninstall SHALL remove only entries whose command string contains `# rcc-hook` -- other user hooks are preserved.

---

## Open Questions

- OQ1: Should there be a hook contract version stamped in the manifest so install can warn on incompatible versions?
- OQ2: When the user runs `/rick run foo` from inside a sub-prompt (not first prompt), should UserPromptSubmit still bind? Currently yes (idempotent rule preserves first binding).
- OQ3: Should Stop emit a final summary line ("Workflow complete in 12 minutes; 3 phases done; 4 files changed") into `## Phase log`?
