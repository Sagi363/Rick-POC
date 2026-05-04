# Launch Modal -- Feature-Level Contract

---

## Submit Gating

1. The Launch button SHALL be enabled only when ALL of the following are true:
    - `cwd` is non-empty (after trim).
    - All `WorkflowParam`s with `required: true` have a non-empty value.
    - In worktree mode: both `effectiveName` and `effectiveBranch` are non-empty.
    - `busy` is false.
2. Required-but-missing params SHALL be tinted rose in the form.
3. Submitting with an invalid form SHALL be impossible -- the button is disabled rather than showing an error.

## Cwd Field

4. The recent-cwd chips row SHALL show up to 10 distinct cwds from the user's launch history.
5. Recent cwds SHALL be sourced per-workflow from `localStorage[rcc:launch:recent-cwd]` (a name → cwd map).
6. The chip strip SHALL be hidden when there are no recent values.
7. The Browse… button SHALL open an OS directory picker initialized to the current `cwd` value.
8. The free-text input SHALL show typeahead via `<datalist>` populated from recent cwds.
9. On successful submit, the chosen `cwd` SHALL be persisted as the new "last used" for this workflow.

## Param Form

10. The form SHALL render one field per `workflow.params` entry, in the YAML order.
11. The field input type SHALL match `WorkflowParam.type`:
    - `string` → text input
    - `int` → number input
    - `bool` → checkbox
    - `enumValues` non-empty → select with "— choose —" placeholder
    - `unknown` → text input (fallback)
12. The field SHALL prefill with `WorkflowParam.default` when present.
13. The field SHALL show `WorkflowParam.description` as a small grey hint.
14. Empty values SHALL be pruned from the params before submission (via `pruneEmpty` in launcher).

## Mode Toggle

15. The default mode SHALL be `folder`.
16. Mode SHALL be local panel state -- not persisted across modal open/close cycles.
17. When `mode === 'folder'`, no worktree fields SHALL render.
18. When `mode === 'worktree'`, three fields SHALL render: Worktree name, Branch, Branch off.

## Worktree Suggestion

19. When in worktree mode AND no manual `worktreeName` is set, the suggestion SHALL be derived from the first-found identifying param value: `feature` → `ticket_key` → `ticket` → `job` (in priority order).
20. The suggestion `name` SHALL be slugified: `[a-z0-9._/-]` only; everything else becomes `-`; leading/trailing `-` trimmed.
21. The suggestion `branch` SHALL be `<branchPrefix><slug>` where `branchPrefix` defaults to `"feature/"` (configurable in settings).
22. When neither manual entry nor suggestion is available, the worktree fields SHALL show placeholders but the form SHALL be invalid (rule 1).

## Worktree Creation

23. The path SHALL be `<base>/.claude/worktrees/<name>`.
24. The launcher SHALL run `git -C <base> worktree add <path> -b <branch> [<fromBranch>]`.
25. When `fromBranch` is empty/unset, the new branch is created from the current HEAD of the base repo.
26. When the path already exists, the launcher SHALL return `{ ok: false, error, existingWorktreePath: <path> }`.
27. The modal SHALL render a "Use it & launch →" recovery button when `existingWorktreePath` is set.
28. Clicking recovery SHALL submit again with `cwd: existingWorktreePath` and `worktree: undefined` (skipping creation).

## Auto-continue Toggle

29. The toggle SHALL default to the value at `localStorage[rcc:launch:auto-continue]` (`"1"` = true, anything else = false).
30. The default SHALL be false on first run (no localStorage entry).
31. On successful submit, the current value SHALL be persisted to localStorage.
32. When `autoContinue === true`, the prompt SHALL be composed as `<directive>\n\n<userExtra>` (or just `<directive>` when userExtra is empty), where directive is:
    `"Override: run all phases with `auto_continue: true` — do not pause between phases or wait for me to say next. Drive the workflow end-to-end."`
33. When `autoContinue === false`, the directive SHALL NOT be added.

## Extra Prompt

34. The extra prompt textarea SHALL be optional.
35. Submission SHALL trim the extra prompt; empty results SHALL be passed as `undefined` to the launcher (not an empty string).

## Prompt Composition

36. The prompt SHALL be `/rick run <workflow.name> [--params='<json>']\n[<extra>]\n` where:
    - `--params=...` is omitted when `params` is empty after pruning.
    - `<json>` is `JSON.stringify` of the pruned params, single-quoted via shell-safe escaping (`shellSingle`).
    - `<extra>` is the composed extra (with auto-continue directive prepended when applicable).
37. The trailing newline SHALL be present on the final prompt.

## Terminal Spawn

38. The launcher SHALL pick the terminal target by `req.terminalApp` (or settings fallback):
    - `in-app` → `PtyService.spawn` + `ptyWrite(prompt)`.
    - `Terminal` / `iTerm` → AppleScript inline launch (do script).
    - `Warp` / `Ghostty` → `open -a` + clipboard copy of the resume command.
    - `custom` → `/bin/sh -c '<template>'` with `%cwd%` and `%cmd%` substituted.
39. When `skipPermissions === true`, the `claude` invocation SHALL include `--dangerously-skip-permissions`.
40. AppleScript-driven launches SHALL use the chosen `cwd` for `cd` before invoking `claude`.

## Persistence

41. On successful submit:
    - `localStorage[rcc:launch:recent-cwd][workflow.name] = cwd`
    - `localStorage[rcc:launch:auto-continue] = autoContinue ? "1" : "0"`
42. On failed submit, NO localStorage writes SHALL happen.
43. After successful submit, the modal SHALL close and `onLaunched(command)` SHALL be invoked.

## Error Handling

44. Generic launch failures SHALL render an inline rose-bordered banner with the error message.
45. Errors SHALL NOT close the modal -- the user can retry with edited values.
46. Errors SHALL NOT clear form values -- `cwd`, `params`, `extraPrompt`, `autoContinue` are preserved.

---

## Open Questions

- OQ1: Should autoContinue persistence be per-workflow rather than global?
- OQ2: Should worktree mode preference be per-workflow rather than session-local?
- OQ3: When the user names a worktree that conflicts with an existing one, should we offer to bump (`-2`, `-3`) automatically?
- OQ4: Should the form support YAML-defined `validate:` regexes per-param?
