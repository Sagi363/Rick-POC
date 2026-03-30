# Workflow Composition Protocol

## Overview

The `uses` keyword lets a parent workflow embed a child workflow as a **phase**. Child steps are flattened inline, params are wired, and outputs flow between phases automatically.

## YAML Syntax

```yaml
- id: <phase-id>
  uses: <workflow-name>          # Name from same Universe's workflows/
  description: "What this phase does"
  params:                        # Map parent params → child params
    child_param: "{{parent_param}}"
    dynamic_param: "{{step_outputs.previous_phase.last_child_step}}"
  auto_continue: true|false      # Controls pause BETWEEN phases (not child internal steps)
```

A step with `uses` MUST NOT have `agent` or `task` — those are for direct steps only.

## Runtime Semantics

When Rick encounters a step with `uses`, follow these steps in order:

### 1. Resolve

Load `workflows/<name>.yaml` from the **same Universe**. Error if not found: `"Child workflow '<name>' not found in universe '<universe>'"`.

### 2. Validate Params

Check the child workflow's `params` definition. For each param with `required: true`, verify the parent provides it in the `params:` map. Error if missing: `"Required param '<name>' not provided for child workflow '<uses>'"`.

### 3. Resolve Params

Replace template variables in param values:
- `{{parent_param}}` → resolve from the parent workflow's params
- `{{step_outputs.<phase-id>.<child-step-id>}}` → resolve from outputs of previously completed phases/steps
- `{{step_outputs.<phase-id>}}` → alias for the last child step's output in that phase

Param resolution happens **at execution time** (when the phase is about to start), not at flatten time. This allows referencing outputs from earlier phases.

If a `{{step_outputs.X.Y}}` reference cannot be resolved (phase X hasn't run yet), error: `"Cannot resolve '{{step_outputs.X.Y}}' — phase 'X' has not completed yet"`.

### 4. Flatten

Expand child workflow steps inline into the parent, prefixing each step ID with the phase ID:
- Child step `search` in phase `gather` → `gather.search`
- Child step `summarize` in phase `gather` → `gather.summarize`

### 5. Wire Dependencies

- The **first** child step depends on whatever came before the phase (previous phase's last step, or a preceding direct step)
- Child steps follow their own internal ordering (sequential by default)
- The step **after** the phase depends on the phase's last child step

### 6. Pass Outputs

Store child step outputs in the state file:
- Each child step: `step_outputs["<phase-id>.<child-step-id>"]`
- Phase alias: `step_outputs["<phase-id>"]` = last child step's output

The phase alias allows the next phase to reference the whole phase's final output without knowing the child step IDs.

### 7. Honor auto_continue

Two levels of auto_continue:
- **Child-level**: Internal child steps follow their own `auto_continue`. If child step has `auto_continue: true`, the next child step within the same phase executes immediately.
- **Phase-level**: The parent step's `auto_continue` controls the transition **out** of the phase. After the last child step completes:
  - `auto_continue: true` → immediately start the next phase/step
  - `auto_continue: false` → pause, wait for `/rick next`

### 8. Display

During child step execution, show phase progress:
```
Phase: gather [1/2] — gather.search
```

Use the phase `description` for handoff/recap messages.

## State Schema (Nested)

Composed workflows use a nested state schema:

```json
{
  "workflow_id": "wf-1711234567",
  "workflow_name": "full-pipeline",
  "universe": "example-composition",
  "status": "in_progress",
  "current_phase": 1,
  "total_phases": 3,
  "phases": [
    {
      "id": "gather",
      "uses": "gather-info",
      "description": "Phase 1: Research the topic",
      "status": "completed",
      "current_step": 2,
      "total_steps": 2,
      "steps": [
        { "id": "gather.search", "agent": "researcher", "task": "...", "status": "completed" },
        { "id": "gather.summarize", "agent": "researcher", "task": "...", "status": "completed" }
      ]
    },
    {
      "id": "process",
      "uses": "process-data",
      "description": "Phase 2: Analyze research findings",
      "status": "in_progress",
      "current_step": 0,
      "total_steps": 2,
      "steps": [
        { "id": "process.analyze", "agent": "analyst", "task": "...", "status": "running" },
        { "id": "process.categorize", "agent": "analyst", "task": "...", "status": "pending" }
      ]
    }
  ],
  "step_outputs": {
    "gather.search": "Raw findings...",
    "gather.summarize": "Key points: ...",
    "gather": "Key points: ..."
  }
}
```

**Mixed workflows** (direct steps + `uses` phases) use the same nested schema. Direct steps are wrapped in a synthetic phase:

```json
{
  "id": "kickoff",
  "uses": null,
  "description": null,
  "status": "completed",
  "current_step": 1,
  "total_steps": 1,
  "steps": [
    { "id": "kickoff", "agent": "researcher", "task": "...", "status": "completed" }
  ]
}
```

This keeps the schema uniform — everything is a phase, some just have one step.

**Regular workflows** (no `uses` at all) continue using the existing flat schema. Only detect and use nested schema when at least one step has `uses`.

## Error Handling

If a child step fails:
1. Mark the child step status as `"failed"` in state
2. Mark the phase status as `"failed"`
3. Report: `"Phase '<phase-id>' failed at step '<phase-id>.<child-step-id>': <error>"`
4. `/rick next` retries the **failed child step** (not the whole phase)

## Nesting Guard

A child workflow referenced by `uses` MUST NOT itself contain steps with `uses`. Max composition depth is 1.

When resolving a child workflow, scan its steps. If any step has a `uses` field, error immediately:
```
"Nested composition not supported — '<child-workflow>' contains 'uses' steps. Max depth is 1."
```

## Checklist for Skill Execution

When executing a workflow, for each step:

1. Check: does this step have `uses`?
   - **No** → execute normally (agent + task)
   - **Yes** → enter composition flow:
     a. Load child workflow YAML
     b. Check nesting guard (child must not have `uses` steps)
     c. Validate required params
     d. Resolve param templates (including `step_outputs` references)
     e. Replace child param placeholders in child step tasks with resolved values
     f. Flatten child steps with prefixed IDs
     g. Execute child steps sequentially, respecting child `auto_continue`
     h. After last child step, store phase alias in `step_outputs`
     i. Respect parent `auto_continue` for phase transition
     j. Update state file with nested schema
