# Workflow Status Panel -- Presentation Spec

## 1. Overview

The phase-ring section in the file list. Shows each outer step of the active workflow as a colored ring badge with status, agent name, file count, and -- for the running step -- live activity (`<Subagent> · <activity>`).

Goal: at a glance the user can see which phase is done, which is running, who Rick handed off to, and what files each phase touched.

**Source files:**
- `src/renderer/src/components/FileList.tsx` (lines 122-160) -- Section integration
- `src/renderer/src/components/FileList.tsx` (lines 196-282) -- WorkflowSection
- `src/renderer/src/components/FileList.tsx` (lines 170-194) -- StepBadge + STEP_TEXT_TINT
- `src/main/services/correlation.ts` -- correlateWorkflow + 4-pass logic
- `src/shared/types.ts` -- WorkflowRunView, WorkflowStepView

---

## 2. Presentation Models

### 2.1 WorkflowRunView

The DTO returned by `correlateWorkflow` and embedded in `SessionFilesDTO.workflowRun`. Conforms to: structural-only.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `workflow` | String | No | `"bug-fix-from-jira"` | YAML name |
| `label` | String | No | `"bug-fix-from-jira · SCCOM-33084"` | `<workflow> · <feature>` if a feature was extracted |
| `feature` | String | Yes | `"SCCOM-33084"` | Extracted from `--params=` JSON in the first user prompt |
| `steps` | List of WorkflowStepView | No | (see 2.2) | One per outer YAML step |

### 2.2 WorkflowStepView

One outer step's render-ready state. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `id` | String | No | `"research"` | YAML id |
| `agent` | String | No | `"ticket-research"` | YAML agent OR uses-target |
| `collaborators` | List of String | No | `[]` | YAML collaborators |
| `description` | String | Yes | `"Phase 1: …"` | YAML description; rendered as title attribute on hover |
| `status` | "pending" \| "running" \| "done" | No | `"running"` | dynamic -- determined by 4-pass correlation |
| `files` | List of String | No | `["src/foo.ts"]` | dynamic -- attributed via subagent sidechain walk |
| `startedAt` | Number (ms) | Yes | `1746280000000` | First signal that flipped status from pending |
| `currentSubagent` | String | Yes | `"Trinity (Implementor)"` | dynamic -- from latest "Handing to" line; only set on running step |
| `currentActivity` | String | Yes | `"write 4 KMP integration tests"` | dynamic -- description trailing the handoff |

---

## 3. Visual States

### 3.1 Step row layout

```
1. ✓ research      ticket-research                   3   ← done
2. ⚠ reproduce     Trinity (Implementor) · write…       ← running w/ live activity
3. ○ fix           bug-fix                              ← pending
4. ○ validate      test-and-validate                    ← pending
```

### 3.2 StepBadge variants

| Status | Badge appearance |
|---|---|
| `done` | Emerald-bordered ring with `✓` glyph |
| `running` | Amber-bordered ring with pulsing amber dot |
| `pending` | Empty grey ring, no glyph |

### 3.3 Step text tinting

| Status | Text class |
|---|---|
| `done` | `text-zinc-300` |
| `running` | `text-amber-200` |
| `pending` | `text-zinc-600` |

### 3.4 Right-side label (dynamic)

| Status | Content |
|---|---|
| `running` AND `currentSubagent` set | `<currentSubagent>` (amber-300) ` · ` `<currentActivity>` (zinc-400) |
| `running` AND no subagent yet | `<step.agent>[+ collaborator + …]` (zinc-500) |
| `done` / `pending` | `<step.agent>[+ collaborator + …]` (zinc-500) |

### 3.5 Collapse state

- The whole Workflow section: collapsible via section header, persisted in `localStorage[rcc:files:collapsed]` keyed `wf:<workflow>`.
- Each step row: collapsible to expand/hide its file list, keyed `wf:<workflow>:<step.id>`.
- Steps with zero files have no expand chevron and are not clickable.

### 3.6 File list per step

When expanded, files attributed to the step render as `Item` rows beneath. Each item uses `relativeTo(file, sessionCwd)` for display and `iconForFile(file)` for the leading icon. Clicking an item selects that file in the preview pane.

---

## 4. Interactions

| Target | Gesture | Result |
|---|---|---|
| Workflow section header | Click | Toggles whole section collapse |
| Step row (with files) | Click | Toggles that step's file list expansion |
| Step row (no files) | Click | No-op (cursor not pointer) |
| File item | Click | Selects file in preview |

---

## 5. Live Refresh

The whole `WorkflowRunView` is re-fetched whenever:

- Selected session changes (`sessionId` dep -- fires the initial-fetch effect).
- Session activity ticks (`session.lastActivity` dep -- fires the live-refresh effect, no Summary force-select).

Both effects call `window.rcc.listSessionFiles(sessionId)`, which on main re-runs `correlateWorkflow` against the JSONL. Cost: one re-parse per transcript write -- acceptable at current sizes.

---

## 6. Live Activity Source

The "live activity" on the running step comes from `correlation.ts`:

1. After all four passes set step statuses, the correlator iterates `steps[]` from the highest index.
2. For the first running step found, it attaches `currentSubagent` (with role appended in parens if present) and `currentActivity` from the latest `**Rick:** Handing to **<Subagent>** [(<Role>)] — <activity> [claude:opus]` line found in main-thread assistant text.
3. Regex used: `HANDOFF_DETAIL_RE` -- see [`../rick-contract/spec.md`](../rick-contract/spec.md) §1.3.

If no handoff line exists yet (workflow just started), the step's right-side label falls back to the static agent name.

---

## 7. File Attribution Per Step (Direct Workflows Only)

For non-composed workflows (no `uses:` in any step):

1. `correlateWorkflow` walks the JSONL collecting main-thread `Task` tool_use blocks (toolUseId, subagent_type, owner uuid).
2. For each sidechain message, traces `parentUuid` back to the spawning Task via `ownerOf(uuid)`.
3. Scans sidechain `tool_use` blocks for `Read` / `Write` / `Edit` / `MultiEdit` / `NotebookEdit` calls.
4. Extracts `file_path` / `path` / `notebook_path` and attributes the file to the spawning Task → matched step.

For composed (`uses:`) workflows, this Pass-1 attribution doesn't fire -- Tasks aren't matched to outer steps. Files-per-step remains empty in those cases. Future work: walk inner workflows' spawns and bucket by phase markers.

---

## 8. Cross-References

| Concept | Spec |
|---|---|
| The 4-pass correlation algorithm | [`../rick-contract/spec.md`](../rick-contract/spec.md) §2 |
| Exact regexes (PHASE_RE, HANDOFF_DETAIL_RE, etc.) | [`../rick-contract/contract.md`](../rick-contract/contract.md) |
| What Rick must emit for these passes to work | [`../rick-contract/contract.md`](../rick-contract/contract.md) §1 |
| FileList integration (this section is one of several) | [`../progress-panel/spec.md`](../progress-panel/spec.md) (sibling panel) |

---

## Open Questions

- OQ1: For composed workflows, should we attribute files to outer steps by tracking which phase marker is active when each file write happens? Currently empty.
- OQ2: Should we show a per-step elapsed-time chip when `startedAt` is known?
- OQ3: When two running steps exist (parallel phases — uncommon today), which gets the live activity? Currently the highest-indexed.
