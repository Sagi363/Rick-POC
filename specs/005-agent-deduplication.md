# Spec: Agent Deduplication — Ground Rule #5

**Status:** PR Open (#2)
**Author:** dekelmaman
**Reviewer:** Sagi363
**Date:** 2025-03-17

---

## Problem Statement

When users create new agents in a Universe, there's no guard against duplicating responsibilities already covered by existing agents. This leads to:

- **Agent sprawl** — multiple agents with overlapping responsibilities
- **Contradictory rules** — two agents that "own" TCA design may give conflicting guidance
- **No single source of truth** — nobody knows which agent is authoritative for a given domain
- **Maintenance burden** — updates to a capability must be applied to multiple agents

### Real Example (this session)

In the `Issues-Team` Universe we created 5 agents:

| Agent | Responsibilities |
|-------|-----------------|
| `alloy-auditor` | AlloyUI compliance, component auditing |
| `tca-architect` | TCA reducer/state/action design |
| `swift-implementor` | TCA + SwiftUI code implementation |
| `issues-reviewer` | Code review (TCA patterns, AlloyUI, memory leaks) |
| `issues-researcher` | Codebase exploration, pattern finding |

If someone later asks "create an agent that designs TCA reducers and writes SwiftUI views" — that's `tca-architect` + `swift-implementor`. Without a guard, Rick creates a 6th agent that overlaps both, and the Universe degrades.

---

## Solution: Overlap Detection + Guided Resolution

### Core Mechanism

Before creating any new agent, Rick performs an **overlap check**:

1. Read every existing agent's `soul.md` in the target Universe
2. Compare proposed responsibilities against existing agents
3. If **>20%** overlap → **block creation** and offer alternatives

### Why 20%?

- 30% (original proposal) was too permissive — allows agents that are "mostly different" but still create confusion about ownership
- 20% catches cases like "writes TCA code" overlapping with an agent that "writes Swift code including TCA" — even though TCA is a subset, it's enough to cause ambiguity
- 0% would be too strict — agents naturally share peripheral concerns (e.g., both a reviewer and implementor "know" TCA, but from different angles)

### Resolution Alternatives

When overlap is detected, Rick proposes one of:

| Alternative | When to Use | Result |
|-------------|-------------|--------|
| **UPDATE** | New responsibilities fit naturally in existing agent | Existing agent gains new rules/expertise |
| **SPLIT** | Existing agent is overloaded (doing too much) | Original gets narrower, new agent takes extracted scope. Zero overlap. |
| **COMPOSE** | Request spans multiple existing agents | Create a workflow that chains existing agents instead of a new agent |

Key constraint: user must **explicitly approve** the chosen path before Rick proceeds.

### The Litmus Test

> Can you describe the new agent's purpose WITHOUT mentioning anything an existing agent already does?

If not → you don't need a new agent.

---

## Design Decisions

### 1. Universe-scoped only (no cross-universe check)

**Decision:** Overlap detection runs only within the target Universe, NOT across installed Universes.

**Rationale:**
- Universes are independent team contexts — the `Demo` universe's `sherlock-researcher` and `Issues-Team`'s `issues-researcher` serve different teams/purposes even though both "research"
- Cross-universe dedup would prevent legitimate specialization
- Each Universe is its own bounded context

### 2. Ground rule, not agent behavior

**Decision:** This is enforced at the Rick orchestrator level (ground-rules.md), not by individual agents.

**Rationale:**
- Agents don't know about each other — they can't self-police
- Rick is the only entity that sees the full agent roster
- Ground rules can't be overridden by any Universe or agent persona

### 3. Blocking, not warning

**Decision:** Rick blocks creation when overlap >20%, doesn't just warn.

**Rationale:**
- Warnings get ignored ("yeah yeah, create it anyway")
- The user can still override by explicitly approving an alternative
- Forces a conscious decision rather than accidental sprawl

### 4. Compare soul.md + rules.md, skip tools.md and Memory.md

**Decision:** Overlap detection reads `soul.md` (responsibilities/expertise) AND `rules.md` (domain rules/constraints). Skips `tools.md` and `Memory.md`.

**Rationale:**
- `soul.md` defines "what the agent does" — but two agents with different souls can still overlap if they enforce the same domain rules
- `rules.md` contains domain-specific knowledge that IS the agent's functional identity — e.g., `alloy-auditor`'s AlloyUI component reference lives in rules, not soul. A proposed "UI compliance checker" with a different soul but the same rules is a duplicate.
- `tools.md` defines capability surface (Read/Write/Edit), not responsibility — many agents legitimately share the same toolset
- `Memory.md` is runtime noise — accumulated learnings that change every session

---

## Implementation

### Where

- `ground-rules.md` in the Rick-POC repo (source of truth)
- Copied to `~/.rick/ground-rules.md` on `rick setup` / `rick add`
- Rick skill reads this on every invocation

### Detection Algorithm (for Rick's prompt)

```
1. User says "create agent X with responsibilities [A, B, C, D, E]"
2. For each existing agent in the Universe:
   a. Read their soul.md — extract expertise and responsibilities
   b. Read their rules.md — extract domain rules and constraints
   c. Compare BOTH against the proposed agent's scope
   d. Count how many proposed responsibilities OR domain rules are covered
3. overlap_pct = max(covered_count / total_proposed * 100) across all agents
4. If overlap_pct > 20% → BLOCK
5. Also check union: if responsibilities are split across multiple agents
   (agent1 covers A, agent2 covers B) → still BLOCK if combined > 20%
```

### What to compare and why

| File | Read? | Overlap Signal | Rationale |
|------|-------|---------------|-----------|
| `soul.md` | **Yes** | High | "What I do" — expertise, responsibilities, identity |
| `rules.md` | **Yes** | High | "How I work" — domain rules, constraints, project-specific knowledge |
| `tools.md` | No | Low | Shared toolsets (Read/Write/Edit) don't mean responsibility overlap |
| `Memory.md` | No | None | Runtime learnings are ephemeral, not agent identity |

**Why rules.md matters:** Two agents can have different souls but enforce identical domain rules. Example: `alloy-auditor` and a proposed "UI compliance checker" might have different personalities, but if their `rules.md` both contain the AlloyUI component reference, anti-pattern list, and gap table — they're functionally duplicates. Soul-only comparison misses this entirely.

### Example Scenarios

| Request | Existing | Overlap | Action |
|---------|----------|---------|--------|
| "Agent that writes TCA code" | `swift-implementor` writes TCA | ~80% | BLOCK → update `swift-implementor` |
| "Agent that audits accessibility" | `alloy-auditor` checks a11y | ~60% | BLOCK → add a11y rules to `alloy-auditor` |
| "Agent that writes E2E tests" | No agent covers E2E | 0% | ALLOW |
| "Agent that reviews TCA patterns" | `issues-reviewer` does TCA review | ~70% | BLOCK → update `issues-reviewer` |
| "Senior dev who writes and reviews" | `swift-implementor` + `issues-reviewer` | ~90% combined | BLOCK → compose workflow |

---

## Conversation Context

This spec was born from a live session where we:

1. **Created the Issues-Team Universe** with 5 specialized agents on `dekelmaman/ACC_issues_universe`
2. **Ran a multi-agent AlloyUI audit** using 4 agents from Demo-Rick-Universe in parallel (Sherlock, Pixel, Nitpick, Grumpy) — found 28 violations across 15 files
3. **Discussed multi-universe orchestration** — discovered agents are isolated and Rick is the bridge
4. **Identified the duplication risk** — without a guard, someone could create agents that overlap `tca-architect` or `swift-implementor`
5. **Drafted and iterated the rule** — started at 30% threshold, tightened to 20%, removed cross-universe check after discussion

### Key Insight

The "never fork" ground rule (Rule #1) was found to be too broad during this session — it conflates team Universes (where branching is correct) with external/upstream repos (where forking is the only option). This is flagged in the PR but is a separate concern from agent deduplication.

---

## Open Questions

1. **Should the 20% threshold be configurable per Universe?** Some Universes might want stricter or looser dedup.
2. **How should SPLIT work in practice?** When Rick splits an overloaded agent, should it auto-generate the new soul.md/rules.md or require human authoring?
3. **Should there be a `rick check-overlap` command?** Useful for auditing existing Universes that may already have duplication.

---

## Files Changed

- `ground-rules.md` — Added Rule #5 with detection protocol, resolution alternatives, litmus test, and examples table

## PR

- https://github.com/Sagi363/Rick-POC/pull/2
