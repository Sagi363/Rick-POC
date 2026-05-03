# Workflows Panel -- Presentation Spec

## 1. Overview

Sister tab to Sessions in the left drawer. Lists every workflow available in the active universe with enough metadata that the user can pick one and launch it without leaving the app. Click a card → opens the [Launch Modal](../launch-modal/spec.md).

**Source files:**
- `src/renderer/src/components/Drawer.tsx` (lines 195-250) -- Tab + workflow list rendering
- `src/renderer/src/components/WorkflowCard.tsx` -- Card component
- `src/main/services/universes.ts` (lines 17-99) -- chokidar scan + watcher
- `src/main/services/universes.ts` (lines 101-145) -- `toWorkflow` + `normalizeParam`
- `src/shared/types.ts` -- Workflow, WorkflowStep, WorkflowParam

---

## 2. Presentation Models

### 2.1 Workflow

The shape pushed from main to renderer for each YAML found under `~/.rick/universes/<u>/workflows/`. Conforms to: structural-only.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `name` | String | No | `"bug-fix-from-jira"` | YAML `name:` or filename without extension |
| `universe` | String | No | `"ACC_issues_universe"` | Parent universe directory |
| `filePath` | String | No | `"/Users/.../bug-fix-from-jira.yaml"` | Absolute path to the YAML |
| `description` | String | Yes | `"End-to-end Jira bug fix"` | YAML `description:` |
| `agents` | List of String | No | `["sherlock","watson","trinity"]` | Unique non-`unknown` agents across all steps |
| `dependsOn` | List of String | No | `[]` | Aggregated `depends_on` across steps |
| `params` | List of WorkflowParam | No | (see 2.2) | Normalized from YAML `params:` block |
| `steps` | List of WorkflowStep | No | (see 2.3) | Normalized from YAML `steps:` block |

### 2.2 WorkflowParam

A single declared workflow input. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `name` | String | No | `"ticket_key"` | The YAML key under `params:` |
| `type` | "string" \| "int" \| "bool" \| "unknown" | No | `"string"` | Explicit `type:` or inferred from `default:` |
| `default` | unknown | Yes | `"android"` | Pre-fills the form input |
| `description` | String | Yes | `"Jira ticket key"` | Rendered as small grey hint |
| `required` | Bool | Yes | `true` | Submit blocked when missing |
| `enumValues` | List of String | Yes | `["android","ios","kmp"]` | Triggers `<select>` rendering |

### 2.3 WorkflowStep

One outer step in the YAML. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `id` | String | No | `"research"` | YAML `id:` or `step-N` synthetic |
| `agent` | String | No | `"ticket-research"` | YAML `agent:` OR `uses:` (sub-workflow name) OR `"unknown"` |
| `collaborators` | List of String | Yes | `[]` | YAML `collaborators:` |
| `description` | String | Yes | `"Phase 1: …"` | YAML `description:` |
| `dependsOn` | List of String | Yes | `[]` | YAML `depends_on:` |
| `uses` | String | Yes | `"ticket-research"` | Set when this step composes another workflow |

---

## 3. Visual States

### 3.1 Card Layout

| Region | Content | Notes |
|---|---|---|
| Title row | Bold monospace `name` | Truncated at 1 line |
| Description | First line of `description` | 2-line clamp |
| Agents row | Comma-separated unique agent names | Hidden when empty |
| Param badge | `<N> params` pill | Hover shows the param names |
| Composed indicator | Small icon | Shown when any step has `uses:` |

### 3.2 Search

| Sub-state | Behavior |
|---|---|
| Empty search | All workflows visible |
| Non-empty | Substring match (case-insensitive) against `name`, `description`, agent names |

### 3.3 Empty states

| Condition | Rendering |
|---|---|
| No active universe selected | "Pick a universe in the top bar." |
| Universe selected, zero workflows | "No workflows in `<universe>`. Drop a YAML into `~/.rick/universes/<u>/workflows/`." |

---

## 4. Interactions

| Target | Gesture | Result |
|---|---|---|
| Card | Click | Opens Launch Modal pre-bound to this workflow (`onLaunchWorkflow(workflow)`) |
| Search input | Type | Live filter |

---

## 5. Discovery & Live Updates

`UniverseService` watches `~/.rick/universes/` recursively (chokidar, depth 4, 250ms debounce). Adds, edits, deletes of `**/workflows/*.{yaml,yml}` trigger a rescan. The new workflow list pushes to the renderer over `IPC.WorkflowsUpdate`.

Therefore:
- Adding a YAML on disk → card appears within ~1s.
- Editing a YAML → re-parsed and re-pushed.
- Deleting a YAML → card disappears.

Malformed YAML is skipped silently in v1 (toast surfacing is open question OQ1).

---

## 6. Param Normalization

The Workflow YAML can specify params two ways:

```yaml
params:
  ticket_key:
    description: "Jira ticket key"
    required: true
  platform:
    default: "android"
    enum: [android, ios, kmp]
```

OR shorthand:

```yaml
params:
  ticket_key: ""
  retries: 3
  verbose: false
```

`normalizeParam()` (`universes.ts:123`) converts both into a unified `WorkflowParam`. Type comes from explicit `type:` if present, otherwise inferred from the default value (`boolean` → `bool`, integer → `int`, string → `string`, anything else → `unknown` and falls back to a string text input).

---

## Open Questions

- OQ1: Malformed YAMLs are skipped silently. Should we surface a toast pointing at the bad file?
- OQ2: Workflows are listed by file order from `readdir`. Should we sort alphabetically or by recently-launched?
- OQ3: When a workflow is renamed (file rename), the watcher re-scans and a different name appears. There's no "rename" tracking — both names show transiently.
