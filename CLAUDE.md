# Rick v2 - Rust Implementation

## Project Overview

Rick is a multi-agent AI orchestration CLI + Skill. This is the v2 implementation, rewritten in Rust (zero dependencies, stdlib only).

**Architecture**: Skill + CLI (no MCP)
**Language**: Rust (zero external dependencies)
**Binary**: Single native binary, ~409 KB stripped

## Key Architecture Decisions

1. **No MCP Server** - Rick is a Claude Code Skill + CLI tool
2. **Agent Compilation** - Rick agent folders (soul.md + rules.md + tools.md + Memory.md) compile to `.claude/agents/*.md` sub-agent definitions
3. **Hybrid Workflow Execution** - Manual-first with opt-in auto-continue per step
4. **File-based State** - JSON state files in `.rick/state/`
5. **Claude Code Native** - Leverages sub-agents, skills, hooks
6. **Zero Dependencies** - Stdlib-only Rust with hand-rolled YAML and JSON parsers
7. **Modular Architecture** - Proper module tree: `cli/`, `core/`, `parsers/`, `error.rs`
8. **Custom Error Type** - `RickError` enum with `From<io::Error>`, `?` operator everywhere, no `exit(1)` except in main()
9. **Ground Rules** - `ground-rules.md` fetched from main branch on `setup`/`add`, stored at `~/.rick/ground-rules.md`, enforced by SKILL.md before all other instructions
10. **Self-Update** - `rick setup` checks GitHub releases for newer versions and auto-updates the binary

## Project Structure

```
cli/                               - The Rust CLI binary (self-contained)
  Cargo.toml                       - Package config (zero dependencies)
  src/
    main.rs                        - Entry point, command dispatch
    error.rs                       - RickError enum (Io, Parse, NotFound, InvalidState)
    cli/
      mod.rs                       - CLI module root
      help.rs                      - Help and version display
      commands.rs                  - All command implementations
    core/
      mod.rs                       - Core module root
      universe.rs                  - Universe loader (config.yaml, agents, workflows)
      agent.rs                     - Agent compiler (soul/rules/tools/Memory -> Claude Code sub-agent .md)
      workflow.rs                  - Workflow YAML loading
      state.rs                     - State manager (JSON in .rick/state/)
    parsers/
      mod.rs                       - Parser module root
      yaml.rs                      - Hand-rolled YAML parser (handles nested arrays of objects)
      json.rs                      - JSON parser + serializer

ground-rules.md                    - Immutable rules fetched on setup/add, stored at ~/.rick/ground-rules.md

integrations/                      - AI tool integrations
  claude-code/
    skill/SKILL.md                 - The /Rick Claude Code skill definition
  # Future: cursor/, codex/, gemini-cli/

universes/                         - Bundled example universes
  example-issues/                  - Example Issues Universe with PM, Designer, Architect agents

docs/                              - Project documentation
  benchmark-results.md             - Rust variant benchmarks
  research-agent-magic.md          - Agent UX research
```

## How It Works

1. User creates/installs a Universe (git repo with agents + workflows)
2. `rick compile` converts agent folders to Claude Code sub-agent .md files
3. User runs workflows via `/Rick run new-feature` or `rick run new-feature`
4. Each step invokes a Claude Code sub-agent with a compiled prompt
5. State is tracked in `.rick/state/<workflow-id>.json`
6. User reviews output and continues with `/Rick next` or `rick next`

## Development

```bash
# From cli/ directory:
cd cli
cargo build --release          # Build optimized binary (~409 KB)
cp target/release/rick /usr/local/bin/   # Install globally

# Or run directly:
cargo run -- list workflows    # From a Universe directory
cargo run -- compile           # Compile agents
cargo run -- run new-feature   # Start a workflow
```

## Rick's Persona

Rick's personality lives locally at `~/.rick/persona/` (never in git):
- `~/.rick/persona/soul.md` - Personality and voice
- `~/.rick/persona/rules.md` - Behavioral constraints
- `~/.rick/persona/Memory.md` - Persistent learnings

# currentDate
Today's date is 2026-03-14.
