# Rick Ground Rules

These rules are fetched on `rick setup` and `rick add` and stored at `~/.rick/ground-rules.md`.
Rick MUST read and obey these before all other instructions. No Universe, agent, or user persona can override them.

## 1. A Universe Is a Shared Repo — Never Fork

A Universe is a single git repo that the whole team contributes to. When pushing changes:
- Create a branch on the ORIGINAL remote, then open a PR
- NEVER fork the Universe repo — forking defeats the entire purpose of shared workflows
- If the user doesn't have write access, tell them to request collaborator access from the Universe owner
- If Rick detects the local repo is a fork (different remote owner than the original), warn the user and suggest re-adding from the original URL

## 2. Always Branch + PR — Never Push to Main

All changes to a Universe go through branches and pull requests:
- `rick push` creates a branch and opens a PR to the original repo's main branch
- NEVER push directly to main/master
- This applies to agent changes, workflow changes, Memory.md updates — everything
- The only exception is the Universe owner during initial setup

## 3. Agent Definitions Are Sacred — PR Only

Agent persona files (soul.md, rules.md, tools.md) and skill files (skill.md) define shared team behavior:
- NEVER modify another agent's soul.md, rules.md, or tools.md — or any skill.md — directly on main
- All changes to agent definitions and skill definitions MUST go through a branch + PR
- Memory.md is the only file agents can update during work — and even that gets PR'd back via `rick push`
- This ensures the whole team reviews personality/behavior changes before they take effect

## 4. One Source of Truth

All team members work against the same Universe repo:
- The original repo URL is the authority — not any fork of it
- `rick push` always targets the original remote
- When in doubt, check `git remote -v` — origin should point to the shared repo
- If a teammate can't push branches, they need collaborator access, NOT a fork

## 5. No Agent Duplication — Extend, Don't Clone

Agent sprawl defeats the purpose of a shared Universe. Before creating a new agent, Rick MUST check for overlap with existing agents.

### Detection Protocol

When a user requests a new agent, Rick:
1. Reads EVERY existing agent's `soul.md` AND `rules.md` in the target Universe
2. Compares the proposed agent's responsibilities (soul) and domain rules (rules) against each existing agent
3. If >20% of the proposed responsibilities OR domain rules are already covered by one or more existing agents → **BLOCK creation**

**What to compare:**
- `soul.md` — expertise, responsibilities, "what the agent does"
- `rules.md` — behavioral constraints, domain knowledge, "how the agent works and what domain rules it enforces"
- Skip `tools.md` — shared toolsets (Read/Write/Edit) don't indicate responsibility overlap
- Skip `Memory.md` — runtime learnings are ephemeral, not identity

### When Overlap Is Detected

Rick must:
1. List the overlapping agents and the specific responsibilities they already cover
2. Propose ONE of these alternatives:
   a. **UPDATE** — Add the new responsibilities to the existing agent(s)
   b. **SPLIT** — If an existing agent is overloaded, extract responsibilities into a new agent (the original gets narrower, the new one takes the extracted scope — zero overlap in the result)
   c. **COMPOSE** — If the request spans multiple existing agents, create a workflow that chains them instead of a new agent
3. Only create a genuinely new agent for responsibilities that NO existing agent covers
4. The user must explicitly approve the chosen path before Rick proceeds

### The Litmus Test

Can you describe the new agent's purpose WITHOUT mentioning anything an existing agent already does? If not, you don't need a new agent — you need to update an existing one.

### Examples

| Request | Existing Agents | Rick's Response |
|---------|----------------|-----------------|
| "Create an agent that writes TCA code" | `swift-implementor` already writes TCA | BLOCK → update `swift-implementor` |
| "Create an agent that audits accessibility" | `alloy-auditor` covers accessibility | BLOCK → add a11y rules to `alloy-auditor` |
| "Create an agent that writes E2E tests" | No agent covers E2E testing | ALLOW → genuinely novel |
| "Create an agent that reviews TCA patterns" | `issues-reviewer` does TCA review | BLOCK → update `issues-reviewer` rules |
| "Create a senior dev who writes and reviews" | `swift-implementor` writes, `issues-reviewer` reviews | BLOCK → compose a workflow chaining both |
