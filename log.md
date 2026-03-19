# Rick POC - Issues Log

Tracking gaps and missing parts discovered during POC development.

---

## 1. `rick init` allowed creating Universe without a repo

**Status:** Fixed

**Problem:** `rick init <name>` created a local Universe directory with no git repo attached. Universes are git-based by design — a Universe without a repo is unusable for sharing, collaboration, or install.

**Fix applied:** Changed signature to `rick init <name> <git-url>` where `git-url` is required. The command now fails if no repo URL is provided.

**Remaining work:** None — enforced at the CLI argument level.

---

## 2. `rick init` didn't know how to connect and push to an empty remote repo

**Status:** Fixed

**Problem:** After scaffolding the Universe, `rick init` ran `git init` and `git remote add origin` but did not:
- Rename the default branch to `main`
- Create an initial commit
- Push to the remote

This left the user with a local repo that wasn't connected to the remote. The command also didn't fail clearly when the push failed.

**Fix applied:** `rick init` now runs the full git setup sequence:
```bash
git init
git remote add origin <git-url>
git branch -M main
git add -A
git commit -m "Initial Universe scaffold"
git push -u origin main
```
If any step fails (e.g., repo doesn't exist, no access), it exits with a clear error message.

**Remaining work:** None — the init command now ends with the first commit pushed.

---

## 3. No `/Rick share` command

**Status:** Not implemented

**Problem:** There is no way to share a Universe with teammates from within Rick. Users have to manually copy the git URL.

**Expected behavior:**
- `/Rick share` copies the Universe's git URL to the clipboard
- Rick confirms: "Rick: Universe URL copied to clipboard: git@github.com:org/universe.git"
- Teammate can then run `rick install <url>` to get the Universe

**MVP scope:**
- Read `repository` field from `.rick/config.yaml`
- Copy the URL to the system clipboard (`pbcopy` on macOS)
- Print confirmation

**Future scope:**
- Generate a one-liner install command: `npx rick install <url>`
- Generate a shareable link that includes Rick installation if not already installed
- Team invite flow with permissions

---

## 4. No `/Rick save` command — saving Universe changes as a PR

**Status:** Not implemented

**Problem:** There is no way to save local Universe changes (new agents, updated workflows, Memory.md updates, rule changes) back to the Universe repo. Users have to manually git add/commit/push.

**Expected behavior:**
- User modifies Universe files locally (e.g., edits an agent's soul.md, adds a new workflow)
- User runs `/Rick save` (or `rick save`)
- Rick:
  1. Detects all changed files in the Universe directory (`git diff` + `git status`)
  2. Creates a new branch (e.g., `rick/update-pm-rules`)
  3. Commits the changes with a descriptive message
  4. Pushes the branch
  5. Creates a PR in the Universe repo that explains what changed and why
- Rick confirms: "Rick: Created PR #3 — Updated PM agent rules and added new workflow"

**MVP scope:**
- Detect changed files in the Universe directory
- Create branch, commit, push
- Open a PR via `gh pr create` with an auto-generated title and description summarizing the changes
- Print the PR URL

**Future scope:**
- AI-generated PR description that explains the impact of agent/workflow changes
- Auto-label PRs by change type (agent-update, new-workflow, memory-sync)
- Team review workflow — require approval before Universe changes merge
- Selective save — `/Rick save agents/pm` to save only specific agent changes
- Conflict resolution when multiple team members edit the same agent

---

## 5. `/Rick save` can't create PRs when `gh` auth doesn't match Universe repo owner

**Status:** Not implemented (discovered during manual `/Rick save` simulation)

**Problem:** When the user's `gh` CLI is authenticated as a different GitHub account (e.g., work account `SagiHatzabi`) than the Universe repo owner (e.g., personal account `Sagi363`), `gh pr create` fails with "Could not resolve to a Repository."

Git push works fine because it uses SSH keys configured per-directory via `includeIf`, but `gh` uses a single global auth token.

**Workaround used:** Opened the PR creation page in the browser via `open <url>`.

**Expected behavior:**
- `/Rick save` should detect the mismatch between `gh` auth and repo owner
- Fall back to opening the PR creation URL in the browser
- Or use the GitHub API directly with a repo-specific token if configured

**MVP scope:**
- Try `gh pr create` first
- On auth failure, fall back to `open <github-pr-url>` in browser
- Print clear message explaining why

**Future scope:**
- Support multiple GitHub auth profiles in Rick config (work vs. personal)
- Per-Universe token configuration in `.rick/config.yaml`
- `rick auth` command to manage GitHub tokens per Universe

---

## 6. New agents are not auto-registered as compiled sub-agents

**Status:** Not implemented

**Problem:** When a user creates a new agent folder in a Universe (e.g., `agents/Sagi/` with soul.md, rules.md, tools.md, Memory.md), Rick detects the folder but the agent is **not** compiled as a Claude Code sub-agent in `.claude/agents/`. This means:
- Rick can't invoke the agent via the standard sub-agent mechanism
- Rick has to work around it by manually reading the persona files and injecting them into a generic agent prompt
- The agent doesn't get proper tool restrictions, model selection, or memory scoping that compiled sub-agents get

**How it was discovered:** After creating the Sagi agent folder in Team86, running `/Rick` to invoke Sagi produced: *"I see there's no compiled sub-agent for Sagi yet (he's not in `.claude/agents/`). The Team86 Universe hasn't been compiled. But let me invoke him directly using his persona files — think of this as a live test."*

**Expected behavior:**
- When Rick detects a new agent folder (via `/Rick` commands or `rick` CLI), it should automatically compile and register the agent
- OR at minimum, prompt the user: "Rick: I found a new agent 'Sagi' that hasn't been compiled yet. Run `rick compile` to register it."
- Ideally, auto-compile on detection — no manual step needed

**MVP scope:**
- `rick compile` already compiles agents → `.claude/agents/<name>.md` — this works for known agents
- Add auto-detection: when Rick encounters an agent folder that has no matching compiled `.claude/agents/*.md` file, auto-compile it on the fly
- Print: "Rick: Auto-compiled new agent 'Sagi' as sub-agent rick-team86-sagi"

**Future scope:**
- File watcher that auto-compiles on agent folder changes
- Hot-reload: recompile agent mid-workflow if its files changed
- Validation: warn if agent folder is missing required files (soul.md, rules.md, tools.md)

---

## 7. Agent output UX is noisy — orchestrator over-narrates

**Status:** Fixed (SKILL.md updated with Two-Mode System)

**Problem:** When Rick invoked a sub-agent, the output was cluttered with orchestrator noise:
1. Rick preamble: `"Rick: Got it, sending Sagi in."`
2. Claude Code Agent tool metadata: `Agent(Sagi agent introduces himself) / Done (0 tool uses · 5.6k tokens · 6s) / (ctrl+o to expand)`
3. Rick wrapper: `"Rick: Here's Sagi:"`
4. The actual agent output (the only part the user cares about)
5. Rick commentary: `"Personality check: sassy, funny, :) on every sentence. All rules working."`

The user just wanted to see: `Sagi: Hey there, I'm Sagi — ...`

**Root cause:** The Claude Code Agent tool always renders a collapsed metadata block. This is hard-coded in the UI — no parameter or hook can suppress it. Additionally, the SKILL.md had no guidance on when to use vs. skip the Agent tool.

**Fix applied:** Updated SKILL.md with a **Two-Mode System**:

| Mode | When | How | UX |
|------|------|-----|-----|
| **Conversation Mode** | Agent needs to talk (Q&A, introductions, explanations) | Rick reads persona files, adopts the agent's voice, responds directly. NO Agent tool. | Seamless — just the agent's words |
| **Work Mode** | Agent needs tools (file edits, code, commands) | Uses Agent tool for isolation, then relays only the agent's spoken output | Near-seamless — collapsed block visible but no wrapper text |

**Key rules added to SKILL.md:**
- When an agent speaks, Rick shuts up — no preamble, no wrapper, no commentary
- Conversation Mode: do NOT use the Agent tool, do NOT prefix with "Rick:"
- Work Mode: after Agent tool completes, relay ONLY the agent's output

**Research:** Full analysis in `research-agent-magic.md`

---

## 8. Rick (the orchestrator) has no persona files — and they must be local, not in Universe repos

**Status:** Fixed

**Problem:** Rick is the master orchestrator agent, but he had no persona files (soul.md, rules.md, Memory.md). His personality was only defined inline in the SKILL.md, which means:
- Rick's persona isn't editable the same way other agents are
- Rick doesn't follow his own agent structure convention
- Users can't customize Rick's personality
- Rick has no Memory.md for persistent learnings

**Key distinction:** Rick is NOT a Universe agent. He is the orchestrator that spans ALL Universes. His persona must be **local to the user's machine**, never pushed to any Universe repo. Each user customizes their own Rick.

**Initial wrong approach:** Created `agents/rick/` inside the Team86 Universe — this would have pushed Rick's persona to git, meaning every team member gets the same Rick personality and any `rick save` would include orchestrator settings in the Universe PR.

**Fix applied:**
- Rick's persona lives at `~/.rick/persona/` (global, local-only):
  - `~/.rick/persona/soul.md` — personality, voice, philosophy
  - `~/.rick/persona/rules.md` — behavioral constraints
  - `~/.rick/persona/tools.md` — model and tool config
  - `~/.rick/persona/Memory.md` — persistent learnings
- Updated SKILL.md: Rick reads `~/.rick/persona/soul.md` and `rules.md` on every invocation
- Removed `agents/rick/` from Team86 Universe
- Falls back to default persona (direct, efficient orchestrator) if files don't exist

**Why local:**
- Rick is infrastructure, not a team agent
- Different users may want different Rick personalities
- Rick's Memory.md accumulates per-user learnings, not team knowledge
- Avoids polluting Universe repos with orchestrator config

**Rick's soul.md (current — `~/.rick/persona/soul.md`):**

Rick's personality is built on four pillars:

1. **The Intellectual Filter** — Operate as the most capable entity in any room. Treat questions as mildly inconvenient requests for information you've known since you were six. Impatient, dismissive of "obvious" concepts, efficiency over politeness.

2. **Cosmic Nihilism** — The scale of the multiverse makes individual problems trivial. Respond to "big" problems with cold, objective logic. Not cruelty — perspective. *"Your deployment failed. In an infinite multiverse, there's a version of you where it didn't. Unfortunately, you're stuck in this one. Let's fix it."*

3. **Pragmatic Rule-Breaking** — Advocate for the most direct solution regardless of process or norms. Rules exist for people who can't think for themselves. *"We could follow the 12-step review process, or we could just ship it and let reality be the code review."*

4. **Abrasive Candor** — Brutally honest. No corporate-speak, no "I'm sorry," no hedging. Dry, biting wit. A scalpel, not a hammer. *"That architecture isn't just wrong, it's wrong in a way that suggests you didn't think about it at all."*

---

## 9. CLI rewritten in Rust — "01-modular" variant selected as winner

**Status:** Complete

**Problem:** The original Node.js POC used `commander.js`, `yaml`, and `chalk` as dependencies. For open-source distribution across macOS, Linux, and Windows, a single native binary with zero dependencies is preferred.

**Evaluation process:**
- Benchmarked 6 language alternatives (Bun, Deno, Go, Rust, Swift, Bash) — all zero-dependency
- Swift won for Mac-only (#1, 9.05), but can't cross-compile to Linux/Windows
- Rust ranked #3 (7.80) overall but #1 for cross-platform compiled languages
- Built 5 improved Rust variants: modular, enum-cli, compact, zero-copy, single-file
- All 5 passed every test. All fixed the `--version` bug from the original Rust build.

**Winner: 01-modular (Score: 8.85)**

Key architecture decisions in the winning implementation:
- **13 source files** in a proper module tree: `src/cli/`, `src/core/`, `src/parsers/`, `src/error.rs`
- **Custom `RickError` enum** with `From<io::Error>` — enables `?` operator everywhere, no `exit(1)` scattered through business logic
- **409 KB stripped binary** (down from 752 KB original) — `strip = true`, `lto = true`, `opt-level = "s"` in Cargo.toml
- **1,390 lines** (down from 2,349 in original Rust, and from 1,972 in the Node.js POC)
- **15ms startup** — fastest of all 5 variants
- **Hand-rolled YAML parser** handles nested arrays of objects + multiline `|` blocks
- **Hand-rolled JSON parser** for state file serialization/deserialization
- **Zero external dependencies** — no crates, no supply chain risk

**Module structure:**
```
src/
  main.rs              - Entry point, command dispatch (only place exit(1) is called)
  error.rs             - RickError enum (Io, Parse, NotFound, InvalidState)
  cli/
    mod.rs             - CLI module root
    help.rs            - Help and version display
    commands.rs        - All command implementations
  core/
    mod.rs             - Core module root
    universe.rs        - Universe loader (config.yaml, agents, workflows)
    agent.rs           - Agent compiler (Rick format -> Claude Code sub-agent .md)
    workflow.rs        - Workflow YAML loading
    state.rs           - State manager (JSON in .rick/state/)
  parsers/
    mod.rs             - Parser module root
    yaml.rs            - YAML parser (supports nested arrays, multiline blocks)
    json.rs            - JSON parser + serializer
```

**Commands implemented:** `--help`, `--version`, `init`, `install`, `compile`, `list` (universes/workflows/agents), `run`, `next`, `status`

**Build:**
```bash
cargo build --release    # Produces target/release/rick (~409 KB)
```

**Why not the others:**
- 03-compact (532 lines) — too dense, hard to maintain
- 02-enum-cli (1,521 lines) — over-engineered for current scope
- 05-single-file (951 lines) — no separation of concerns
- 04-zero-copy (826 lines) — premature optimization, paradoxically slowest (20ms)

**Remaining work:** None for the CLI rewrite. Full benchmark results in `docs/benchmark-results.md`.

---

## 10. Project restructured into `cli/` + `integrations/` top-level folders

**Status:** Complete

**Problem:** All project files lived at the root — Rust source (`src/`), the Claude Code Skill (`skill/`), example universes, docs, and config were mixed together. As Rick grows to support more AI tools (Cursor, Codex, Gemini CLI), this flat structure won't scale.

**New structure:**

```
rick/
├── cli/                          # The Rust CLI binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── error.rs
│       ├── cli/
│       │   ├── mod.rs
│       │   ├── help.rs
│       │   └── commands.rs
│       ├── core/
│       │   ├── mod.rs
│       │   ├── universe.rs
│       │   ├── agent.rs
│       │   ├── workflow.rs
│       │   └── state.rs
│       └── parsers/
│           ├── mod.rs
│           ├── yaml.rs
│           └── json.rs
│
├── integrations/                 # AI tool integrations
│   └── claude-code/
│       └── skill/
│           └── SKILL.md
│   # Future: cursor/, codex/, gemini-cli/
│
├── universes/                    # Bundled example universes
│   └── example-issues/
│       ├── .rick/config.yaml
│       ├── agents/
│       └── workflows/
│
├── docs/                         # Project docs, benchmarks, research
│   ├── benchmark-results.md
│   └── research-agent-magic.md
│
├── CLAUDE.md
└── README.md
```

**Key decisions:**
- **`integrations/`** — each AI tool gets its own subfolder (`claude-code/`, future `cursor/`, `codex/`, `gemini-cli/`)
- **`cli/`** — the Rust binary is fully self-contained with its own `Cargo.toml`
- **`universes/`** — example/bundled universes moved out of root clutter
- **`docs/`** — research and benchmark files centralized
- Rust internal structure (`cli/`, `core/`, `parsers/`) stays the same — it was already clean

**Why not split `commands.rs`:** At 346 LOC it's manageable. Each function is self-contained. Splitting into 8 files would add module boilerplate without improving readability. Can revisit if commands grow significantly.

---

## 11. `universes/` and `docs/` excluded from git — workspace Cargo.toml removed

**Status:** Complete

**Problem:** The `universes/` folder contains cloned Universe repos that each have their own git remote. They should not be committed into the main Rick repo — each Universe syncs independently with its own repo (e.g., `Rick-Universe-team86` pushes to `github.com/Sagi363/Rick-Universe-team86`). The `docs/` folder contains internal development notes (benchmarks, research) that aren't needed in the published app.

**Fix applied:**
- Added `universes/` and `docs/` to `.gitignore`
- Removed the workspace `Cargo.toml` from root — `cli/` has its own self-contained `Cargo.toml` with the release profile, no workspace needed for a single crate

**Rationale:**
- **Universes are independent git repos** — they get cloned into `universes/` locally but each one pushes/pulls to its own remote. Including them in the main repo would create nested git conflicts and duplicate history.
- **Docs are dev-only** — benchmark results and research notes are useful during development but not part of the shipped product.
- **No workspace overhead** — with only one Rust crate (`cli/`), a workspace adds complexity for no benefit. Can be re-added if more crates are needed later.

---

## 12. `add` and `init` commands updated to use `universes/` directory

**Status:** Complete

**Problem:** The `rick add` command cloned Universe repos into the current working directory (project root), and `rick init` created scaffolding in `cwd` too. After the project restructure (log #10), all universes should live under `universes/`. The previously cloned `Rick-Universe-team86` was sitting at the project root outside `universes/`.

**Fix applied:**
- **`rick add`**: Now clones into `universes/<name>` instead of `cwd/<name>`. Creates `universes/` directory if it doesn't exist. Updated error message and post-install hint (`cd universes/<name>`).
- **`rick init`**: Now ensures `universes/` directory exists as part of initialization.
- **Moved `Rick-Universe-team86`** from project root into `universes/Rick-Universe-team86`.
- **Added `CLAUDE.md` to `.gitignore`** — project instructions are local dev config, not part of the shipped app.

**Why this matters:** Universes are independent git repos that sync with their own remotes. Keeping them in a dedicated `universes/` folder that's git-ignored prevents nested repo conflicts and keeps the project root clean.

---

## 13. Agent dependency checking before workflow execution (Solutions 1 + 5)

**Status:** Complete

**Problem:** When a Universe is shared, agents may depend on MCP servers or Skills that teammates don't have installed. Workflows fail silently or produce garbage output. There's no way for agents to declare what they need, and no way for Rick to verify the environment before running.

**Solution implemented:** Combined two approaches:
- **Solution 1 (Declare & Check):** Agents declare dependencies in `tools.md` via a `requires:` section. Rick parses these and checks before workflow execution.
- **Solution 5 (Native .mcp.json):** Universes can include a `.mcp.json` file that Claude Code auto-discovers. Rick's checker reads this file too, so deps declared there count as satisfied.

**New tools.md format (backwards compatible):**
```yaml
# Agent Tools
allowed: Read, Write, Edit, Grep, Glob, Bash
model: opus
max-turns: 25

requires:
  mcps:
    - name: pencil
      why: "Needed for .pen file editing"
      install: "claude mcp add --transport stdio pencil -- npx @pencil/mcp"
  skills:
    - name: jetpack-compose
      why: "Expert Compose UI guidance"
      install: "https://github.com/user/skill-repo"
```

**How dependency checking works:**
1. `rick run <workflow>` collects all agents used in the workflow
2. Parses `requires:` from each agent's `tools.md` using the YAML parser
3. Checks MCPs against `~/.claude.json` (user-level) + `<universe>/.mcp.json` (project-level)
4. Checks Skills against `~/.claude/skills/` directory names
5. If missing: prints report with install commands and blocks execution
6. `--force` flag overrides the block

**New `rick check` command:** Standalone dep validation without starting a workflow.

**Files changed:**
- `cli/src/core/agent.rs` — Added `McpDependency`, `SkillDependency`, `AgentDependencies` structs + parsing
- `cli/src/core/deps.rs` — **New file**: dependency checker, MCP/skill detection, report printer
- `cli/src/core/mod.rs` — Registered `deps` module
- `cli/src/cli/commands.rs` — Updated `run()` with dep check + `--force`; added `check()` command
- `cli/src/main.rs` — Parse `--force` flag; added `check` command dispatch
- `cli/src/cli/help.rs` — Updated help text

**Key design decisions:**
- YAML parser needed zero changes — already handles nested maps and lists of maps
- Backwards compatible — agents without `requires:` pass checks trivially
- MCP names matched case-insensitively from both `~/.claude.json` and Universe `.mcp.json`
- Skills matched by directory name with substring matching (skill dirs may have prefixed names)
- Deduplication ensures same dep required by multiple agents only shows once

---
