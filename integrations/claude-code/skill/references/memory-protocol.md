# Agent Memory

Agents have persistent memory that accumulates across sessions and workflows.

## How Memory Works

| Layer | File | Scope | Purpose |
|-------|------|-------|---------|
| **Agent-private** | `agents/<name>/Memory.md` | One agent, all runs | Accumulated knowledge: decisions, preferences, patterns |
| **Rick's memory** | `~/.rick/persona/Memory.md` | Rick himself, all sessions | User preferences, orchestration learnings |

## Memory Loading
- `rick compile` includes each agent's `Memory.md` AND referenced skill files in the compiled `.claude/agents/rick-*.md` file
- In Conversation Mode, read the agent's `Memory.md` along with soul.md and rules.md
- Rick reads `~/.rick/persona/Memory.md` on every invocation

## Memory Updates
- Agents append learnings to their own `agents/<name>/Memory.md` during Work Mode
- Rick updates `~/.rick/persona/Memory.md` after workflows or when learning user preferences
- Memory files are committed to git — they ARE the institutional knowledge transfer mechanism
- `rick push` includes Memory.md changes in PRs so the team shares learnings

## What Agents Should Remember
- Architectural decisions made in the project
- User preferences for code style, tools, patterns
- Recurring issues and their solutions
- What worked and what didn't in past workflows

## What Agents Should NOT Remember
- Session-specific context (current task details)
- Temporary state or in-progress work
- Anything that duplicates soul.md or rules.md
