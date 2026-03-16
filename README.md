<div align="center">

# Rick

Multi-agent AI orchestration for [Claude Code](https://docs.anthropic.com/en/docs/claude-code).
Define teams of specialized AI agents, wire them into workflows, and share them as git repos.

</div>

## The Problem

AI coding assistants are powerful — but every developer configures them in isolation.

🏝️ **Workflows live in silos.**
One developer crafts the perfect prompt chain: PM writes PRD → Designer specs UI → Developer implements. It works great — but it stays on their machine. The rest of the team never sees it.

🔄 **Improvements don't propagate.**
Someone figures out a better way to structure agent instructions, adds guardrails, tunes a workflow — those gains are local. Every other developer is still running their old version. There's no `git pull` for AI workflows.

🚧 **Setup is a barrier.**
Configuring effective AI agents requires prompt engineering, MCP servers, tool permissions, and model tuning. In practice, only the "AI expert" sets things up. Everyone else gets a worse experience — or none at all.

✅ **Rick fixes this** by making AI workflows **versionable, shareable, and installable** — just like code.

## How Rick Solves It

Rick introduces the concept of a **Universe** — a git repo containing your team's AI agents and workflows. One person defines the agents. Everyone else joins with a single command.

```
# Team lead creates the Universe
/Rick init my-team git@github.com:your-org/my-universe.git

# Share with the team
/Rick invite
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

One line:

```bash
curl -fsSL https://raw.githubusercontent.com/Sagi363/Rick-POC/main/install.sh | bash
```

This downloads the Rick binary, installs the Claude Code skill, and creates default persona files.

To install and immediately join a team's Universe:

```bash
curl -fsSL https://raw.githubusercontent.com/Sagi363/Rick-POC/main/install.sh | bash -s -- -u git@github.com:your-org/your-universe.git
```

### What gets installed

| Component | Location | Purpose |
|-----------|----------|---------|
| `rick` binary | `/usr/local/bin/` or `~/.local/bin/` | CLI tool |
| Skill | `~/.claude/skills/rick/SKILL.md` | Enables `/Rick` in Claude Code |
| Persona | `~/.rick/persona/soul.md` + `rules.md` | Rick's personality (yours to customize) |

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

An agent is a folder with markdown files that define its personality (`soul.md`), constraints (`rules.md`), and tool access (`tools.md`):

```
agents/
  pm/
    soul.md       # Personality and expertise
    rules.md      # Behavioral constraints
    tools.md      # Allowed tools, model, dependencies
  designer/
    ...
  developer/
    ...
```

Each agent has its own persona, rules, and tool permissions. The PM can't edit code. The developer can't modify Jira tickets. Separation of concerns, enforced.

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

## Creating a Universe

```
/Rick init my-team git@github.com:your-org/my-universe.git
```

This creates the directory structure, initializes the git repo, and links it to your remote. Then:

1. **Add agents** — Create folders under `agents/` with `soul.md`, `rules.md`, and `tools.md`
2. **Add workflows** — Create YAML files under `workflows/` that chain your agents
3. **Compile** — `/Rick compile` generates Claude Code sub-agents
4. **Invite** — `/Rick invite` prints a one-liner your teammates can run to join

From that point on, `git pull && rick compile` keeps everyone in sync.

## Usage

### In Claude Code (primary)

```
/Rick list agents          # See available agents
/Rick list workflows       # See available workflows
/Rick run new-feature      # Start a workflow
/Rick next                 # Execute next step
/Rick status               # Check progress
/Rick invite               # Generate a shareable install link
```

Natural language works too:

```
/Rick ask the PM to write a PRD for user authentication
/Rick have TicketMaster check my open tickets
/Rick let the designer review this layout
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
- **Memory.md** — Persistent learnings (created at runtime)

These files are local-only (never in git) and survive reinstalls. Running `rick setup` again will never overwrite your customized persona. Edit them to make Rick yours.

## Architecture

- **Language:** Rust (zero external dependencies, stdlib only)
- **Binary:** Single native binary, ~409 KB stripped
- **State:** JSON files in `.rick/state/`
- **Parsers:** Hand-rolled YAML and JSON (no serde, no dependencies)
- **Integration:** Claude Code skill + CLI (no MCP server)

## License

MIT
