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

Agent persona files (soul.md, rules.md, tools.md) define shared team behavior:
- NEVER modify another agent's soul.md, rules.md, or tools.md directly on main
- All changes to agent definitions MUST go through a branch + PR
- Memory.md is the only file agents can update during work — and even that gets PR'd back via `rick push`
- This ensures the whole team reviews personality/behavior changes before they take effect

## 4. One Source of Truth

All team members work against the same Universe repo:
- The original repo URL is the authority — not any fork of it
- `rick push` always targets the original remote
- When in doubt, check `git remote -v` — origin should point to the shared repo
- If a teammate can't push branches, they need collaborator access, NOT a fork
