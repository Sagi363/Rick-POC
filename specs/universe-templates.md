# Universe Templates

## Problem

A Universe is a shared repo. Multiple developers contribute agents and workflows. Without guidelines, every developer creates agents differently:
- One person creates a focused single-role agent with skills
- Another creates a god-agent that's a developer, designer, and reviewer in one
- A third creates an agent with 500 lines of rules that should be skills
- Workflows have inconsistent step structures, naming, and checkpoint patterns

The result is chaos. The whole point of a Universe is consistency across the team.

## Solution

Universe Templates — markdown instruction files that live in `.rick/templates/`. When Rick creates a new agent or workflow, it reads the template first and follows its guidelines. When changes are pushed via PR, Rick flags non-compliant agents in the PR description.

## Spec

### Location

Templates live in `.rick/templates/` inside a Universe:

```
my-universe/
  .rick/
    config.yaml
    templates/
      agent/
        template.md          # Agent template (folder-based detection)
      workflow/
        template.md          # Workflow template (folder-based detection)
```

Or flat:
```
my-universe/
  .rick/
    config.yaml
    templates/
      agent-template.md      # Detected by filename
      workflow-template.md
```

Or with frontmatter:
```
my-universe/
  .rick/
    config.yaml
    templates/
      our-agents.md          # Has `type: agent` frontmatter
      our-workflows.md       # Has `type: workflow` frontmatter
```

All three are valid. Rick detects them using the priority order below.

### Template Detection (Cascading)

Rick searches for templates in this order, stopping at the first match:

**1. Folder-based (highest priority)**
- `.rick/templates/agent/` exists → all contents are the agent template
- `.rick/templates/workflow/` exists → all contents are the workflow template
- A folder can contain multiple files (e.g., `soul-guidelines.md`, `rules-guidelines.md`) — Rick reads all of them as one template

**2. Frontmatter**
- If no matching folder, scan all `.md` files in `.rick/templates/` (including subdirectories)
- Look for YAML frontmatter with `type: agent` or `type: workflow`:
  ```markdown
  ---
  type: agent
  ---
  # Agent Guidelines
  ...
  ```

**3. Filename**
- If no frontmatter match, match by filename keywords:
  - Filename contains `agent` → agent template
  - Filename contains `workflow` → workflow template
- Case-insensitive matching

### Constraints

- **One template per type** — a Universe can have at most 1 agent template and 1 workflow template
- If Rick detects multiple templates for the same type, it:
  1. Warns the user: "Found multiple agent templates: [list]. A Universe should have exactly one. Please consolidate them."
  2. Lists the detected files so the user can fix it
  3. Does NOT guess which one to use — refuses to proceed until resolved

### Template Format

Templates are markdown files with instructions for Rick. They describe what a good agent or workflow looks like in this Universe.

#### Agent Template Example

```markdown
---
type: agent
---
# Agent Template — Issues Universe

## Agent Philosophy
Every agent in this Universe has ONE clear role. An agent is a specialist, not a generalist.

## Required Files
- `soul.md` — Personality, expertise, communication style
- `rules.md` — Non-negotiable constraints (keep under 100 lines; use skills for detailed knowledge)
- `tools.md` — Allowed tools, model, skill references
- `Memory.md` — Empty starter for persistent learnings

## soul.md Guidelines
- Start with a one-line description: "You are **Name** — a [role] who [does what]."
- Define exactly ONE primary responsibility area
- Include a `## Communication Style` section with a response prefix (e.g., "Andy:")
- Include a `## Expertise` section listing specific technologies/patterns
- DO NOT combine multiple roles (e.g., "developer and designer")

## rules.md Guidelines
- Focus on invariants — things that are ALWAYS true regardless of the task
- Keep it under 150 lines — if it's longer, extract detailed knowledge into a skill
- Include anti-patterns with code examples where applicable
- End with a checklist section for pre-completion verification

## tools.md Guidelines
- List only the tools this agent actually needs
- Reference skills for specialized knowledge domains
- Include a `why` for each skill dependency

## Naming Convention
- Folder name: `<name>-<role>` in kebab-case (e.g., `andy-roaid`, `rev-reviewer`)
- Agent display name: descriptive, memorable (e.g., "Andy Roaid", "Rev")

## Anti-Patterns
- DO NOT create agents with multiple hats (developer + designer + reviewer)
- DO NOT put 500 lines of domain knowledge in rules.md — use a skill instead
- DO NOT hardcode project-specific paths or URLs in soul.md — put them in rules.md or Memory.md
- DO NOT skip Memory.md — every agent needs a place to accumulate learnings
```

#### Workflow Template Example

```markdown
---
type: workflow
---
# Workflow Template — Issues Universe

## Structure
- Every workflow is a YAML file in `workflows/`
- Name: `<action>-<thing>.yaml` in kebab-case (e.g., `new-feature.yaml`, `bug-fix.yaml`)

## Required Fields
- `name` — Human-readable workflow name
- `version` — Semver string
- `description` — One sentence explaining what the workflow does and when to use it

## Step Guidelines
- Each step has: `id`, `agent`, `task`, `checkpoint`, `expected_output`, `next`
- Use `checkpoint: true` for steps that need human review before continuing
- Use `auto_continue: true` only for lightweight validation steps
- The `task` field should be specific enough that the agent can work without ambiguity
- The `expected_output` field should describe what "done" looks like

## Step Naming
- Use kebab-case: `pm-write-prd`, `dev-implement`, `reviewer-audit`
- Prefix with the agent role for clarity

## Anti-Patterns
- DO NOT create workflows with more than 7 steps — split into sub-workflows
- DO NOT skip checkpoints between major phases (e.g., always checkpoint after PRD, after design)
- DO NOT assign the same agent to consecutive steps — that's a sign the steps should be merged
```

### Template Variables (Optional)

Templates MAY contain `{{variables}}` that Rick substitutes at creation time:

| Variable | Description |
|----------|-------------|
| `{{agent_name}}` | Name of the agent being created |
| `{{agent_role}}` | Role/responsibility of the agent |
| `{{universe_name}}` | Name of the Universe |
| `{{prefix}}` | Agent's response prefix (e.g., "Andy:") |

These are optional — templates work fine as pure instruction files without variables.

### Rick's Behavior

#### At Agent/Workflow Creation

When the user asks Rick to create a new agent or workflow in a Universe:

1. **Check for templates** — scan `.rick/templates/` using the cascading detection
2. **If template found** — read it, follow its guidelines when creating the agent/workflow
3. **If template conflicts with user request** — warn the user:
   > "Rick: The Issues Universe template says agents should have a single role, but you're asking me to create an agent that's both a developer and a reviewer. Want me to split this into two agents, or override the template?"
4. **If no template** — create normally, no enforcement

#### At PR Time (`rick push`)

When Rick opens a PR that includes new or modified agents:

1. **Read the Universe's agent template** (if it exists)
2. **Audit each new/modified agent** against the template guidelines
3. **Add a compliance section to the PR description**:

```markdown
## Template Compliance

### agent-template.md audit:
- ✅ `andy-roaid` — Single role, skills for domain knowledge, rules under 150 lines
- ⚠️ `mega-agent` — Multiple roles detected (developer + reviewer). Template recommends single-role agents.
- ⚠️ `big-rules-agent` — rules.md is 380 lines. Template recommends under 150 lines; extract to skills.
```

4. Warnings are informational — they don't block the PR. Reviewers decide.

#### At Compile Time (`rick compile`)

No enforcement at compile time. Templates are soft guidelines, not hard gates. Compilation always succeeds if the files are valid.

### CLI Changes

No new CLI commands needed. The template system is built into Rick's existing behavior:
- `rick compile` — no change
- `rick push` — adds template compliance audit to PR description
- Agent/workflow creation (via `/rick` skill) — reads templates before creating

### SKILL.md Changes

Add to the skill's Available Commands or Natural Language Understanding sections:

```markdown
When creating a new agent or workflow, ALWAYS check `.rick/templates/` first:
1. Look for folder `.rick/templates/agent/` or `.rick/templates/workflow/`
2. If not found, scan `.rick/templates/*.md` for frontmatter `type: agent` or `type: workflow`
3. If not found, scan for filenames containing `agent` or `workflow`
4. If a template is found, follow its guidelines
5. If the user's request conflicts with the template, warn them and ask how to proceed
6. If multiple templates detected for the same type, warn and refuse to guess
```

### Future Scope (Not in v1)

- **Skill templates** — guidelines for creating skills in the Universe
- **Shared rules** — Universe-wide rules injected into every agent at compile time
- **Template validation CLI** — `rick check --templates` to audit all agents against templates
- **Template inheritance** — base template + per-role overrides
