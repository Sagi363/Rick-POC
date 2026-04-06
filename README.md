<p align="center">
  <img src="assets/logo.svg" alt="Rick" width="360">
</p>

<p align="center">
Multi-agent AI orchestration for <a href="https://docs.anthropic.com/en/docs/claude-code">Claude Code</a>.<br>
Define teams of specialized AI agents, wire them into workflows, and share them as git repos.
</p>

## The Problem

AI coding assistants are powerful — but they're isolated and inflexible.

🏝️ **Workflows live in silos.**
One developer crafts the perfect prompt chain: PM writes PRD → Designer specs UI → Developer implements. It works great — but it stays on their machine. The rest of the team never sees it.

🔒 **Locked into single providers.**
You pay for Claude Code, Cursor, and other AI tools — but can't use them together. Each workflow is stuck with one tool's models, token limits, and pricing. You can't mix the best tool for each task or optimize costs across your subscriptions.

🔄 **Improvements don't propagate.**
Someone figures out a better way to structure agent instructions, adds guardrails, tunes a workflow — those gains are local. Every other developer is still running their old version. There's no `git pull` for AI workflows.

🚧 **Setup is a barrier.**
Configuring effective AI agents requires prompt engineering, MCP servers, tool permissions, and model tuning. In practice, only the "AI expert" sets things up. Everyone else gets a worse experience — or none at all.

✅ **Rick fixes this** by making AI workflows **versionable, shareable, and installable** — and now, **multi-runtime** so you can use all the tools you're already paying for.

## How Rick Solves It

Rick introduces the concept of a **Universe** — a git repo containing your team's AI agents and workflows. One person defines the agents. Everyone else joins with a single command.

```
# Team lead creates the Universe
/rick init my-team git@github.com:your-org/my-universe.git

# Share with the team
/rick invite
# → Prints a one-liner anyone can run to join

# Everyone else joins
curl -fsSL https://rick.sh/install | bash -s -- -u git@github.com:your-org/your-universe.git
```

That's it. Every developer now has:
- The same specialized agents (PM, Designer, Developer, QA — whatever your team needs)
- The same workflows (feature development, bug fixes, code review — your process, codified)
- The same tool permissions and MCP integrations
- Automatic dependency installation

When the team lead pushes an improvement — a better prompt, a new workflow step, a new agent — everyone gets it on the next `git pull` + `rick compile`.

## Install

> **Run this in your terminal — not inside Claude Code.**
> Claude Code will refuse to execute `curl | bash` (it's a security guardrail). Open a regular terminal window and paste the command there.

One line:

```bash
curl -fsSL https://raw.githubusercontent.com/Sagi363/rick-POC/main/install.sh | bash
```

This downloads the Rick binary, installs the Claude Code skill, and creates default persona files.

Try it with the [Demo Universe](https://github.com/Sagi363/Demo-Rick-Universe) — 7 agents with hilarious personalities, 3 ready-to-run workflows:

```bash
curl -fsSL https://raw.githubusercontent.com/Sagi363/Rick-POC/main/install.sh | bash -s -- -u git@github.com:Sagi363/Demo-Rick-Universe.git
```

Or join your team's Universe:

```bash
curl -fsSL https://raw.githubusercontent.com/Sagi363/Rick-POC/main/install.sh | bash -s -- -u git@github.com:your-org/your-universe.git
```

### What gets installed

| Component | Location | Purpose |
|-----------|----------|---------|
| `rick` binary | `/usr/local/bin/` or `~/.local/bin/` | CLI tool |
| Skill | `~/.claude/skills/rick/` + `Rick/` | Enables `/rick` and `/Rick` in Claude Code |
| Persona | `~/.rick/persona/soul.md` + `rules.md` + `Memory.md` | Rick's personality and persistent memory |

### Requirements

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installed
- Git

## Key Concepts

### Universes

A Universe is a git repo that holds your team's agents and workflows:

```
my-universe/
  .rick/
    config.yaml         # Universe metadata
  agents/
    pm/                 # Agent definitions
    designer/
    developer/
    ticketmaster/
  workflows/
    new-feature.yaml    # Workflow definitions
    bug-fix.yaml
```

Universes are shareable. Push one to GitHub and anyone can join with a single command.

### Agents

An agent is a folder with markdown files that define its personality (`soul.md`), constraints (`rules.md`), tool access (`tools.md`), and persistent memory (`Memory.md`):

```
agents/
  pm/
    soul.md       # Personality and expertise
    rules.md      # Behavioral constraints
    tools.md      # Allowed tools, model, dependencies
    Memory.md     # Persistent learnings across sessions
  designer/
    ...
  developer/
    ...
```

Each agent has its own persona, rules, tool permissions, and accumulated knowledge. The PM can't edit code. The developer can't modify Jira tickets. Memory grows over time as agents learn from each workflow run. Separation of concerns, enforced.

### Workflows

A workflow is a YAML file that chains agents through a series of steps:

```yaml
# workflows/new-feature.yaml
name: New Feature
version: "1.0"
description: "PM creates PRD, Designer creates specs, Developer implements"
steps:
  - id: pm-prd
    agent: pm
    task: Create a product requirements document for the requested feature
    checkpoint: true
    expected_output: A markdown PRD file with user stories
    next: designer-specs

  - id: designer-specs
    agent: designer
    task: Create UI/UX design specifications based on the PRD
    checkpoint: true
    expected_output: Design specs with wireframes and component specs
    next: dev-implement

  - id: dev-implement
    agent: developer
    task: Implement the feature based on the PRD and design specs
    checkpoint: true
    expected_output: Working code with tests
    next: end
```

Each step invokes a specific agent with a task. `checkpoint: true` pauses for your review before continuing. Each agent receives the output of previous steps as context.

### Workflow Composition

Workflows can embed other workflows using `uses`. Build small, focused workflows — then compose them into larger pipelines. Improve a child workflow once, every parent benefits.

```yaml
# workflows/research.yaml — a small, reusable workflow
name: Research
steps:
  - id: gather
    agent: researcher
    task: "Find information about {{topic}}"
    auto_continue: true
  - id: summarize
    agent: researcher
    task: "Summarize findings into key points"
```

```yaml
# workflows/full-report.yaml — composes smaller workflows
name: Full Report
params:
  topic:
    description: "What to research"
    required: true
steps:
  - id: research
    uses: research                        # Embeds the Research workflow
    description: "Phase 1: Gather data"
    params:
      topic: "{{topic}}"
    auto_continue: false

  - id: analyze
    uses: analysis                        # Embeds an Analysis workflow
    description: "Phase 2: Analyze findings"
    params:
      data: "{{step_outputs.research.summarize}}"
    auto_continue: false
```

At runtime, Rick flattens `uses` phases into their child steps (e.g., `research.gather`, `research.summarize`) and wires outputs between phases automatically. Parameters support `{{step_outputs.phase.step}}` to pass results from one phase to the next.

## Multi-Runtime Support (A2A v3)

**New in v0.11.0**: Rick can orchestrate agents across multiple AI runtimes — Claude Code, Cursor, and future platforms.

Rick uses the [Agent2Agent (A2A) protocol](https://a2a.ai/) — an industry-standard agent communication protocol backed by Google, Microsoft, IBM, and 150+ organizations — to coordinate agents across different AI tools. This means Rick isn't locked to a single provider. Your PM agent can run on Claude Sonnet, your Architect on Claude Opus, and your Frontend Dev on Cursor — all in the same workflow.

### Runtime Configuration

Each agent declares its preferred runtime in `tools.md`:

```yaml
# agents/pm/tools.md
runtime:
  preferred:
    tool: claude
    model: sonnet
  fallback:
    - tool: cursor
      model: composer-2-fast
```

Workflow steps can override the agent's default:

```yaml
# workflows/example.yaml
steps:
  - id: design
    agent: architect
    runtime:
      tool: claude
      model: opus          # Use Opus instead of agent's default
```

**Supported runtimes**:
- `claude` — Claude Code CLI (`claude -p --model <model>`)
- `cursor` — Cursor CLI (`agent -p --model <model>`)

Model names are user-configurable — change the model in `tools.md` and Rick passes it through to the CLI. If the CLI rejects it, Rick reports a clear error.

### Intelligent Runtime Routing

Rick automatically picks the best execution path:

**Sub-Agent (free)**: When the agent's runtime matches your current Claude Code or Cursor session, Rick uses a sub-agent (no CLI invocation, no API cost).

**CLI Invocation**: When the agent needs a different model or tool, Rick shells out to the appropriate CLI with the exact model requested.

**Example**:
```
Host: Claude Sonnet
Agent PM: claude/sonnet  → Sub-agent (free)
Agent Architect: claude/opus  → CLI: claude -p --model opus
Agent Frontend: cursor/composer  → CLI: agent -p --model composer-2-fast
```

This enables workflows where different agents use different models — the PM uses fast Sonnet for requirements, the Architect uses Opus for deep design, and the Frontend Dev uses Cursor's specialized UI models.

### Parallel Execution (Beta)

Workflows can declare step dependencies with `depends_on` to enable parallel execution:

```yaml
steps:
  - id: design
    depends_on: [requirements]
  
  - id: frontend
    depends_on: [design]      # Can run parallel with backend
  
  - id: backend
    depends_on: [design]      # Can run parallel with frontend
  
  - id: review
    depends_on: [frontend, backend]  # Waits for both
```

When running `rick run` from the terminal, the CLI binary executes independent steps concurrently using stdlib threads. When running `/rick run` from Claude Code or Cursor, steps execute sequentially with real-time personality output.

**Backward compatible**: Existing workflows without `depends_on` auto-linearize (sequential execution, same as before).

### Default Model Per Tool

If an agent has no `runtime:` section in `tools.md`:
- `claude` tool → defaults to `sonnet`
- `cursor` tool → defaults to `auto`

The agent runs on your current session if the tool matches, otherwise uses the tool's default model.

## Creating a Universe

```
/rick init my-team git@github.com:your-org/my-universe.git
```

This creates the directory structure, initializes the git repo, and links it to your remote. Then:

1. **Add agents** — Create folders under `agents/` with `soul.md`, `rules.md`, `tools.md`, and optionally `Memory.md`
2. **Add workflows** — Create YAML files under `workflows/` that chain your agents
3. **Compile** — `/rick compile` generates Claude Code sub-agents
4. **Push** — `/rick push` commits and pushes changes to your team
5. **Invite** — `/rick invite` prints a one-liner your teammates can run to join

From that point on, `git pull && rick compile` keeps everyone in sync.

## Usage

### In Claude Code (primary)

```
/rick list agents          # See available agents
/rick list workflows       # See available workflows
/rick run new-feature      # Start a workflow
/rick next                 # Execute next step
/rick status               # Check progress
/rick push                 # Commit + push Universe changes to your team
/rick invite               # Generate a shareable install link
```

Natural language works too:

```
/rick ask the PM to write a PRD for user authentication
/rick have TicketMaster check my open tickets
/rick let the designer review this layout
```

### CLI

```bash
rick setup                                    # Onboarding wizard
rick setup --universe <url> --install-deps    # Setup + join Universe
rick add <universe-repo-url>                  # Clone a Universe
rick compile                                  # Compile agents
rick check                                    # Verify dependencies
rick run <workflow>                            # Start a workflow
rick next                                     # Continue workflow
rick status                                   # Show progress
rick push                                     # Commit + push changes
rick invite                                   # Shareable install link
```

## Agent Dependencies

Agents can declare MCP servers and skills they need in `tools.md`:

```yaml
requires:
  mcps:
    - name: jira
      why: "Read and update tickets"
      install: "claude mcp add --transport stdio jira -- npx @anthropic/jira-mcp"
```

`rick check` validates all dependencies. `rick setup --install-deps` auto-installs missing MCP servers.

## Rick's Persona

Rick has a customizable personality stored at `~/.rick/persona/`:

- **soul.md** — Voice, personality, communication style
- **rules.md** — Hard behavioral constraints
- **Memory.md** — Persistent learnings (created by `rick setup`)

These files are local-only (never in git) and survive reinstalls. Running `rick setup` again will never overwrite your customized persona. Edit them to make Rick yours.

## Architecture

- **Language:** Rust (zero external dependencies, stdlib only)
- **Binary:** Single native binary, ~409 KB stripped
- **State:** JSON files in `.rick/state/`
- **Parsers:** Hand-rolled YAML and JSON (no serde, no dependencies)
- **Integration:** Claude Code skill + CLI (no MCP server)

## License

MIT
