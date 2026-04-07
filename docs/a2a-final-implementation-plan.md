# Rick A2A POC — Final Implementation Plan

**Version**: 1.0  
**Date**: 2026-03-14  
**Branch**: `poc/a2a-multi-runtime`  
**Worktree**: `.claude/worktrees/a2a-poc/`

---

## Executive Summary

This plan turns Rick from a Claude Code-only orchestrator into a multi-runtime agent platform. Rick will be able to:

- Run from **Claude Code** and execute agents on both Claude CLI and Cursor CLI
- Run from **Cursor** and execute agents on both Claude CLI and Cursor CLI
- Execute workflow steps **in parallel** using stdlib threads
- Keep the same personality-driven UX (handoff, ENTRY/EXIT, recap)
- Stay **zero external dependencies** (stdlib only)

The core idea: Rick uses **A2A protocol types** (standard message formats) internally, but routes them to **RuntimeBackend trait implementations** that invoke CLI tools directly via `std::process::Command`. No HTTP server, no daemon, no external deps.

---

## Architecture Overview

```
                         Rick CLI (single Rust binary)
  ┌──────────────────────────────────────────────────────────────────┐
  │                                                                  │
  │  ┌────────────┐  ┌─────────────┐  ┌──────────────────────────┐  │
  │  │ Workflow    │  │ DAG         │  │ State Manager            │  │
  │  │ Parser      │  │ Scheduler   │  │ (~/.rick/state/*.json)   │  │
  │  └─────┬──────┘  └──────┬──────┘  └──────────────────────────┘  │
  │        │                │                                        │
  │        │         ┌──────┴──────┐                                 │
  │        │         │ Thread Pool │ (stdlib threads + mpsc)         │
  │        │         │             │                                  │
  │        │         │  Thread 1   │──── RuntimeBackend::execute()   │
  │        │         │  Thread 2   │──── RuntimeBackend::execute()   │
  │        │         │  Thread N   │──── RuntimeBackend::execute()   │
  │        │         └─────────────┘                                 │
  │        │                │                                        │
  │  ┌─────┴────────────────┴──────────────────────────────────┐    │
  │  │              RuntimeBackend (trait)                       │    │
  │  │                                                          │    │
  │  │  ┌──────────────────┐      ┌──────────────────────┐     │    │
  │  │  │ ClaudeCliBackend │      │ CursorCliBackend     │     │    │
  │  │  │                  │      │                      │     │    │
  │  │  │ claude -p        │      │ agent -p             │     │    │
  │  │  │ --output-format  │      │ --output-format json │     │    │
  │  │  │ json             │      │ --model <model>      │     │    │
  │  │  │ --agents '{...}' │      │                      │     │    │
  │  │  │ --agent <name>   │      │                      │     │    │
  │  │  └──────────────────┘      └──────────────────────┘     │    │
  │  └─────────────────────────────────────────────────────────┘    │
  │                                                                  │
  │  ┌─────────────────────────────────────────────────────────┐    │
  │  │              Personality Engine                           │    │
  │  │  - Injects ENTRY/EXIT template into prompts              │    │
  │  │  - Parses ENTRY/EXIT markers from responses              │    │
  │  │  - Generates Rick handoff & recap lines                  │    │
  │  └─────────────────────────────────────────────────────────┘    │
  └──────────────────────────────────────────────────────────────────┘
         │                              │
         ▼                              ▼
  ┌──────────────┐              ┌──────────────┐
  │ claude CLI   │              │ agent CLI    │
  │ (Claude Code)│              │ (Cursor)     │
  └──────────────┘              └──────────────┘
```

### Why This Works From Any Host

Rick is a standalone binary. It doesn't care whether it was invoked from Claude Code, Cursor, or a plain terminal. It discovers available runtimes by checking which CLIs are on `$PATH`:

- `which claude` found? → Claude runtime available
- `which agent` found? → Cursor runtime available
- Neither found? → Error with install instructions

### A2A Protocol — Where It Lives

A2A is used as the **internal message format**, not an HTTP transport:

```
Workflow YAML → DAG Scheduler → A2A TaskRequest → RuntimeBackend → CLI invocation
                                                                  ↓
                                A2A TaskResponse ← RuntimeBackend ← CLI JSON output
```

The A2A types (`TaskRequest`, `TaskResponse`, `AgentCard`, `Artifact`) are Rust structs. They flow between Rick's modules. If you later want an HTTP adapter (remote execution, cloud deployment), you implement `RuntimeBackend` over HTTP using the same types. Zero refactoring needed.

---

## Implementation Areas

The plan is split into **10 self-contained areas**. Each area can be implemented independently (respecting the dependency order). Each area includes:

- What to build
- Which files to create/modify
- Exact struct/trait definitions
- How to test it works

### Dependency Graph Between Areas

```
Area 1 (A2A Types) ─────────────────────────────────┐
    │                                                 │
Area 2 (RuntimeBackend Trait)                        │
    │                                                 │
    ├── Area 3 (Claude CLI Backend)                  │
    │                                                 │
    ├── Area 4 (Cursor CLI Backend)                  │
    │                                                 │
Area 5 (Personality Engine) ─────────────────────────┤
    │                                                 │
Area 6 (Workflow Parser Updates) ────────────────────┤
    │                                                 │
Area 7 (DAG Scheduler) ─────────────────────────────│
    │                                                 │
Area 8 (Agent Compilation Refactor) ─────────────────┤
    │                                                 │
Area 9 (CLI Commands + State Updates) ───────────────┤
    │                                                 │
Area 10 (Demo Universe + Integration Test) ──────────┘
```

**Build order**: Areas 1→2→3+4 (parallel)→5→6→7→8→9→10

---

## Area 1: A2A Protocol Types

**Goal**: Define the Rust structs that represent A2A messages. These are the data types that flow between all other modules.

**Files to create**:
- `cli/src/a2a/mod.rs`
- `cli/src/a2a/types.rs`

**Register the module** in `cli/src/main.rs`:
```rust
mod a2a;
```

### Types to Define

```rust
// cli/src/a2a/types.rs

/// Describes what a runtime can do. Used for discovery/health checks.
pub struct AgentCard {
    pub runtime_id: String,       // e.g. "claude-opus", "cursor-composer"
    pub name: String,             // e.g. "Claude Code (Opus 4.6)"
    pub tool: String,             // e.g. "claude-code", "cursor"
    pub model: String,            // e.g. "opus", "composer-2-fast"
    pub capabilities: Vec<String>, // e.g. ["task-execution", "code-generation"]
    pub available: bool,          // true if CLI tool is installed and authed
}

/// What Rick sends to a runtime to execute a step.
pub struct TaskRequest {
    pub task_id: String,          // Unique ID for this task (e.g. "wf-12345-step-design")
    pub session_id: String,       // Workflow run ID (groups related tasks)
    pub description: String,      // The full prompt (personality + context + task)
    pub context: TaskContext,      // Structured context for the agent
    pub artifacts: Vec<Artifact>,  // Files/content from previous steps
}

/// Context passed to the agent within a TaskRequest.
pub struct TaskContext {
    pub workflow_id: String,
    pub step_id: String,
    pub agent_persona: AgentPersona,
    pub prior_steps: Vec<PriorStepSummary>,
}

/// Agent identity info, compiled from soul.md/rules.md.
pub struct AgentPersona {
    pub name: String,             // Agent name (e.g. "Neo")
    pub role: String,             // First line of soul.md (e.g. "Architect")
    pub soul: String,             // Full soul.md content
    pub rules: String,            // Full rules.md content
}

/// Summary of a completed prior step, used for context passing.
pub struct PriorStepSummary {
    pub step_id: String,
    pub agent: String,
    pub role: String,
    pub entry: String,            // AGENT_ENTRY line
    pub exit: String,             // AGENT_EXIT line
    pub summary: String,          // Brief summary of what was done
}

/// A file or content artifact passed between steps.
pub struct Artifact {
    pub id: String,               // e.g. "requirements-md"
    pub name: String,             // e.g. "requirements.md"
    pub content: String,          // File content (inline for small files)
    pub mime_type: String,        // e.g. "text/markdown"
}

/// What a runtime returns after executing a task.
pub struct TaskResponse {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: TaskOutput,
    pub artifacts: Vec<Artifact>,  // Files created/modified by the agent
    pub metadata: TaskMetadata,
}

/// Status of a completed task.
pub enum TaskStatus {
    Completed,
    Failed(String),               // Error message
    Partial(String),              // What was completed before failure
}

/// Structured output with personality markers.
pub struct TaskOutput {
    pub entry: String,            // AGENT_ENTRY content (empty if not found)
    pub content: String,          // Main work output
    pub exit: String,             // AGENT_EXIT content (empty if not found)
    pub raw: String,              // Full unprocessed output from the LLM
}

/// Execution metrics.
pub struct TaskMetadata {
    pub model_used: String,       // Actual model that ran (e.g. "claude-opus-4-6")
    pub runtime_id: String,       // Which runtime was used
    pub duration_ms: u64,         // Wall clock time
    pub tokens_in: u64,           // Input tokens (0 if unknown)
    pub tokens_out: u64,          // Output tokens (0 if unknown)
}
```

### Module file

```rust
// cli/src/a2a/mod.rs
pub mod types;
```

### How to Test

This area is just type definitions. It compiles = it works. Run:
```bash
cd cli && cargo build
```

---

## Area 2: RuntimeBackend Trait + Registry

**Goal**: Define the trait that all runtime backends implement, and a registry that discovers and manages available runtimes.

**Files to create**:
- `cli/src/core/runtime.rs`

**Files to modify**:
- `cli/src/core/mod.rs` — add `pub mod runtime;`

### The Trait

```rust
// cli/src/core/runtime.rs

use crate::a2a::types::*;
use crate::error::RickError;

/// A runtime backend that can execute agent tasks.
/// Each implementation wraps a specific CLI tool.
pub trait RuntimeBackend {
    /// Returns the AgentCard describing this runtime's capabilities.
    fn agent_card(&self) -> &AgentCard;

    /// Checks if this runtime is available (CLI installed, authenticated).
    fn health_check(&self) -> Result<bool, RickError>;

    /// Executes a task and returns the response.
    /// This is a BLOCKING call — it runs the CLI tool and waits for output.
    /// Parallel execution is handled by the scheduler spawning threads.
    fn execute(&self, request: &TaskRequest) -> Result<TaskResponse, RickError>;
}
```

### The Registry

```rust
/// Discovers and manages available runtime backends.
pub struct RuntimeRegistry {
    backends: Vec<Box<dyn RuntimeBackend + Send + Sync>>,
}

impl RuntimeRegistry {
    /// Auto-detect available runtimes by checking PATH.
    pub fn discover() -> Self { ... }

    /// Find a runtime by ID (e.g. "claude-opus", "cursor-composer").
    pub fn get(&self, runtime_id: &str) -> Option<&dyn RuntimeBackend> { ... }

    /// Find the first available runtime from a preference list.
    /// Used for fallback: try preferred, then fallbacks, then any available.
    pub fn resolve(&self, preferred: &str, fallbacks: &[String]) -> Option<&dyn RuntimeBackend> { ... }

    /// List all available runtimes (for display/debugging).
    pub fn list_available(&self) -> Vec<&AgentCard> { ... }
}
```

### Discovery Logic

The `discover()` method checks:

1. **Claude CLI**: Run `which claude`. If found, check `claude --version` returns a version string. If yes, register two backends:
   - `claude-opus` (model: "opus")
   - `claude-sonnet` (model: "sonnet")

2. **Cursor CLI**: Run `which agent`. If found, check `agent --version` returns a version string containing a hyphen (Cursor format: `2026.03.30-a5d3e17`). If yes, register two backends:
   - `cursor-composer` (model: "composer-2-fast")
   - `cursor-gpt54` (model: "gpt-5.4-medium-fast")

3. **Future**: Ollama, API-direct, etc. — just add more backends.

### How PATH Detection Works

```rust
use std::process::Command;

fn is_claude_available() -> bool {
    Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_cursor_available() -> bool {
    Command::new("agent")
        .arg("--version")
        .output()
        .map(|o| {
            o.status.success() && {
                let version = String::from_utf8_lossy(&o.stdout);
                version.contains('-') // Cursor format: "2026.03.30-a5d3e17"
            }
        })
        .unwrap_or(false)
}
```

### Config File (Optional Override)

If `~/.rick/runtimes.yaml` exists, load additional/override runtime configs. If it doesn't exist, rely entirely on auto-detection. Zero config required.

```yaml
# ~/.rick/runtimes.yaml (OPTIONAL — auto-detection works without this)
runtimes:
  claude-opus:
    enabled: true
    model: opus
  claude-sonnet:
    enabled: true
    model: sonnet
  cursor-composer:
    enabled: true
    model: composer-2-fast
  cursor-gpt54:
    enabled: false  # Disable this runtime
    model: gpt-5.4-medium-fast
```

### How to Test

```bash
cd cli && cargo build
# Then manually:
rick runtimes  # New command — lists discovered runtimes
```

Expected output:
```
Available runtimes:
  claude-opus      Claude Code (Opus)         ✓ available
  claude-sonnet    Claude Code (Sonnet)       ✓ available
  cursor-composer  Cursor (Composer 2 Fast)   ✓ available
  cursor-gpt54     Cursor (GPT-5.4)           ✓ available
```

---

## Area 3: Claude CLI Backend

**Goal**: Implement `RuntimeBackend` for Claude Code's `claude` CLI tool.

**Files to create**:
- `cli/src/core/backends/mod.rs`
- `cli/src/core/backends/claude.rs`

**Files to modify**:
- `cli/src/core/mod.rs` — add `pub mod backends;`

### Implementation

```rust
// cli/src/core/backends/claude.rs

use std::process::{Command, Stdio};
use crate::a2a::types::*;
use crate::core::runtime::RuntimeBackend;
use crate::error::RickError;

pub struct ClaudeCliBackend {
    pub card: AgentCard,
    pub model: String,  // "opus" or "sonnet"
}

impl ClaudeCliBackend {
    pub fn new(runtime_id: &str, model: &str) -> Self {
        ClaudeCliBackend {
            card: AgentCard {
                runtime_id: runtime_id.to_string(),
                name: format!("Claude Code ({})", model),
                tool: "claude-code".to_string(),
                model: model.to_string(),
                capabilities: vec![
                    "task-execution".to_string(),
                    "code-generation".to_string(),
                    "file-editing".to_string(),
                ],
                available: true,
            },
            model: model.to_string(),
        }
    }
}

impl RuntimeBackend for ClaudeCliBackend {
    fn agent_card(&self) -> &AgentCard {
        &self.card
    }

    fn health_check(&self) -> Result<bool, RickError> {
        let output = Command::new("claude")
            .arg("--version")
            .output()
            .map_err(RickError::Io)?;
        Ok(output.status.success())
    }

    fn execute(&self, request: &TaskRequest) -> Result<TaskResponse, RickError> {
        // 1. Build inline agent JSON for Claude's --agents flag
        let agents_json = self.build_agents_json(request);

        // 2. Invoke claude CLI
        let output = Command::new("claude")
            .arg("-p")
            .arg("--output-format").arg("json")
            .arg("--model").arg(&self.model)
            .arg("--agents").arg(&agents_json)
            .arg("--agent").arg("rick-task-agent")
            .arg(&request.description)
            .stdin(Stdio::null())   // Non-interactive (critical!)
            .output()
            .map_err(RickError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RickError::InvalidState(
                format!("Claude CLI failed: {}", stderr)
            ));
        }

        // 3. Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = self.parse_claude_json_output(&stdout)?;

        // 4. Parse ENTRY/EXIT markers from result text
        //    (Delegated to personality engine — see Area 5)
        let task_output = crate::core::personality::parse_markers(&parsed.result_text);

        // 5. Build TaskResponse
        Ok(TaskResponse {
            task_id: request.task_id.clone(),
            status: TaskStatus::Completed,
            output: task_output,
            artifacts: vec![], // POC: no artifact extraction yet
            metadata: TaskMetadata {
                model_used: self.model.clone(),
                runtime_id: self.card.runtime_id.clone(),
                duration_ms: parsed.duration_ms,
                tokens_in: parsed.tokens_in,
                tokens_out: parsed.tokens_out,
            },
        })
    }
}
```

### The `--agents` JSON Payload

Claude CLI supports inline agent definitions via `--agents`. Rick builds this dynamically:

```rust
impl ClaudeCliBackend {
    fn build_agents_json(&self, request: &TaskRequest) -> String {
        // Build a JSON object: {"rick-task-agent": {"description": "...", "prompt": "..."}}
        //
        // The "prompt" field contains the agent's soul + rules + any personality
        // instructions. The task itself is passed as the user message (the final
        // positional arg to claude).
        //
        // Use Rick's hand-rolled JSON serializer (parsers/json.rs) to build this.

        let agent_prompt = format!(
            "{}\n\n{}\n\n{}",
            request.context.agent_persona.soul,
            request.context.agent_persona.rules,
            // Personality template is already injected into request.description
            // by the personality engine (Area 5)
            ""
        );

        // Hand-build JSON string (no serde needed)
        format!(
            r#"{{"rick-task-agent":{{"description":"Rick A2A task agent","prompt":"{}"}}}}"#,
            escape_json_string(&agent_prompt)
        )
    }
}
```

### Parsing Claude JSON Output

Claude's `--output-format json` returns:
```json
{
  "type": "result",
  "subtype": "success",
  "result": "AGENT_ENTRY: ...\n<work>\nAGENT_EXIT: ...",
  "session_id": "...",
  "duration_ms": 3074,
  "usage": {
    "inputTokens": 98,
    "outputTokens": 31
  }
}
```

Parse with the existing hand-rolled JSON parser (`parsers/json.rs`):

```rust
struct ClaudeCliOutput {
    result_text: String,
    duration_ms: u64,
    tokens_in: u64,
    tokens_out: u64,
}

fn parse_claude_json_output(&self, raw: &str) -> Result<ClaudeCliOutput, RickError> {
    let parsed = crate::parsers::json::parse_json(raw)
        .map_err(|e| RickError::Parse(format!("Claude JSON: {}", e)))?;

    let result_text = parsed.get("result")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let duration_ms = parsed.get("duration_ms")
        .and_then(|v| v.as_f64())
        .map(|f| f as u64)
        .unwrap_or(0);

    let usage = parsed.get("usage");
    let tokens_in = usage
        .and_then(|u| u.get("inputTokens"))
        .and_then(|v| v.as_f64())
        .map(|f| f as u64)
        .unwrap_or(0);
    let tokens_out = usage
        .and_then(|u| u.get("outputTokens"))
        .and_then(|v| v.as_f64())
        .map(|f| f as u64)
        .unwrap_or(0);

    Ok(ClaudeCliOutput { result_text, duration_ms, tokens_in, tokens_out })
}
```

### How to Test

```bash
# Manual test: does Claude CLI work?
claude -p --output-format json --model sonnet "Say hello" < /dev/null

# Integration test: build and run
cd cli && cargo build
# Then invoke via Rick (after Areas 7-9 are done)
```

---

## Area 4: Cursor CLI Backend

**Goal**: Implement `RuntimeBackend` for Cursor's `agent` CLI tool.

**Files to create**:
- `cli/src/core/backends/cursor.rs`

**Files to modify**:
- `cli/src/core/backends/mod.rs` — add `pub mod cursor;`

### Implementation

```rust
// cli/src/core/backends/cursor.rs

use std::process::Command;
use crate::a2a::types::*;
use crate::core::runtime::RuntimeBackend;
use crate::error::RickError;

pub struct CursorCliBackend {
    pub card: AgentCard,
    pub model: String,  // "composer-2-fast" or "gpt-5.4-medium-fast"
}

impl CursorCliBackend {
    pub fn new(runtime_id: &str, model: &str, display_name: &str) -> Self {
        CursorCliBackend {
            card: AgentCard {
                runtime_id: runtime_id.to_string(),
                name: display_name.to_string(),
                tool: "cursor".to_string(),
                model: model.to_string(),
                capabilities: vec![
                    "task-execution".to_string(),
                    "code-generation".to_string(),
                ],
                available: true,
            },
            model: model.to_string(),
        }
    }
}

impl RuntimeBackend for CursorCliBackend {
    fn agent_card(&self) -> &AgentCard {
        &self.card
    }

    fn health_check(&self) -> Result<bool, RickError> {
        let output = Command::new("agent")
            .arg("--version")
            .output()
            .map_err(RickError::Io)?;
        let version = String::from_utf8_lossy(&output.stdout);
        Ok(output.status.success() && version.contains('-'))
    }

    fn execute(&self, request: &TaskRequest) -> Result<TaskResponse, RickError> {
        // 1. Build full prompt (persona + personality template + task)
        //    Unlike Claude, Cursor's agent CLI does NOT support --agents.
        //    Everything goes into a single prompt string.
        let full_prompt = self.build_full_prompt(request);

        // 2. Invoke cursor agent CLI
        let output = Command::new("agent")
            .arg("-p")
            .arg("--model").arg(&self.model)
            .arg("--output-format").arg("json")
            .arg(&full_prompt)
            .output()
            .map_err(RickError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RickError::InvalidState(
                format!("Cursor CLI failed: {}", stderr)
            ));
        }

        // 3. Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = self.parse_cursor_json_output(&stdout)?;

        // 4. Parse ENTRY/EXIT markers
        let task_output = crate::core::personality::parse_markers(&parsed.result_text);

        // 5. Build TaskResponse
        Ok(TaskResponse {
            task_id: request.task_id.clone(),
            status: TaskStatus::Completed,
            output: task_output,
            artifacts: vec![],
            metadata: TaskMetadata {
                model_used: self.model.clone(),
                runtime_id: self.card.runtime_id.clone(),
                duration_ms: parsed.duration_ms,
                tokens_in: parsed.tokens_in,
                tokens_out: parsed.tokens_out,
            },
        })
    }
}
```

### Prompt Building (Key Difference from Claude)

Cursor's `agent` CLI has no `--agents` flag. The entire persona must be embedded in the prompt:

```rust
impl CursorCliBackend {
    fn build_full_prompt(&self, request: &TaskRequest) -> String {
        // Combine everything into one prompt string:
        // 1. Agent persona (soul + rules)
        // 2. The task description (which already contains
        //    personality ENTRY/EXIT template from Area 5)
        format!(
            "You are {}.\n\n## Your Identity\n{}\n\n## Your Rules\n{}\n\n## Your Task\n{}",
            request.context.agent_persona.name,
            request.context.agent_persona.soul,
            request.context.agent_persona.rules,
            request.description,
        )
    }
}
```

### Parsing Cursor JSON Output

Cursor's `--output-format json` returns:
```json
{
  "type": "result",
  "subtype": "success",
  "result": "AGENT_ENTRY: ...\n<work>\nAGENT_EXIT: ...",
  "duration_ms": 3074,
  "usage": {
    "inputTokens": 98,
    "outputTokens": 31,
    "cacheReadTokens": 13440,
    "cacheWriteTokens": 0
  }
}
```

Parse identically to Claude (same JSON structure). Reuse the same parsing logic — extract into a shared helper function in `backends/mod.rs`.

### How to Test

```bash
# Manual test: does Cursor CLI work?
agent -p --output-format json --model composer-2-fast "Say hello"

# Verify Cursor is actually Cursor's agent (not some other 'agent' binary)
agent --version
# Should output format like: 2026.03.30-a5d3e17
```

---

## Area 5: Personality Engine

**Goal**: Handle the ENTRY/EXIT personality markers — both injecting them into prompts and parsing them from responses. This is shared between all backends.

**Files to create**:
- `cli/src/core/personality.rs`

**Files to modify**:
- `cli/src/core/mod.rs` — add `pub mod personality;`

### Two Responsibilities

#### 1. Inject Personality Template Into Task Description

Before sending a `TaskRequest` to any backend, Rick wraps the raw task with personality instructions:

```rust
// cli/src/core/personality.rs

use crate::a2a::types::*;

/// Wraps a raw task description with personality template (ENTRY/EXIT instructions).
/// Returns the full prompt string to be set as TaskRequest.description.
pub fn inject_personality_template(
    raw_task: &str,
    prior_steps: &[PriorStepSummary],
) -> String {
    if let Some(last_step) = prior_steps.last() {
        // Has prior context — agent should react to previous work
        format!(
            "The previous step was completed by {} ({}).\n\
             Here's a brief summary: {}\n\n\
             Before you begin your task, write a SHORT (1-2 sentence, max 30 words) \
             reaction to the previous agent's work in your persona's voice. \
             Then acknowledge your own task.\n\n\
             After you complete your task, write a SHORT (1 sentence, max 20 words) \
             exit line in your persona's voice.\n\n\
             Format:\n\
             AGENT_ENTRY: <reaction + acknowledgment>\n\
             <your actual work here>\n\
             AGENT_EXIT: <exit line>\n\n\
             Task: {}",
            last_step.agent,
            last_step.role,
            last_step.summary,
            raw_task,
        )
    } else {
        // First step — no prior context
        format!(
            "Before you begin your task, write a SHORT (1-2 sentence, max 30 words) \
             entry line in your persona's voice acknowledging what you're about to do.\n\n\
             After you complete your task, write a SHORT (1 sentence, max 20 words) \
             exit line in your persona's voice.\n\n\
             Format:\n\
             AGENT_ENTRY: <entry>\n\
             <your actual work here>\n\
             AGENT_EXIT: <exit>\n\n\
             Task: {}",
            raw_task,
        )
    }
}
```

#### 2. Parse ENTRY/EXIT Markers From Response

After a backend returns raw text, parse out the structured markers:

```rust
/// Parses AGENT_ENTRY and AGENT_EXIT markers from LLM output.
/// Returns a TaskOutput with entry, content, exit, and raw fields.
pub fn parse_markers(raw_output: &str) -> TaskOutput {
    let mut entry = String::new();
    let mut exit = String::new();
    let mut content_lines: Vec<&str> = Vec::new();
    let mut in_content = false;

    for line in raw_output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("AGENT_ENTRY:") {
            entry = trimmed
                .strip_prefix("AGENT_ENTRY:")
                .unwrap_or("")
                .trim()
                .to_string();
            in_content = true;
        } else if trimmed.starts_with("AGENT_EXIT:") {
            exit = trimmed
                .strip_prefix("AGENT_EXIT:")
                .unwrap_or("")
                .trim()
                .to_string();
            in_content = false;
        } else if in_content {
            content_lines.push(line);
        }
    }

    // If no markers found, treat entire output as content
    let content = if entry.is_empty() && exit.is_empty() {
        raw_output.trim().to_string()
    } else {
        content_lines.join("\n").trim().to_string()
    };

    TaskOutput {
        entry,
        content,
        exit,
        raw: raw_output.to_string(),
    }
}
```

#### 3. Generate Rick's Handoff & Recap Lines

Rick speaks before and after each agent. These are display-only (not sent to LLMs):

```rust
/// Generates Rick's handoff line before an agent runs.
/// Example: "Letting Neo architect this — the man lives for ASCII boxes."
pub fn generate_handoff(agent_name: &str, agent_role: &str, task_summary: &str) -> String {
    format!(
        "Handing this to {} ({}) — {}",
        agent_name,
        agent_role,
        truncate(task_summary, 60),
    )
}

/// Generates Rick's recap line after an agent finishes.
/// Example: "Neo's done. Design's in design.md. Next up: the developer."
pub fn generate_recap(
    agent_name: &str,
    duration_ms: u64,
    next_agent: Option<&str>,
) -> String {
    let duration_str = if duration_ms > 60_000 {
        format!("{}m {}s", duration_ms / 60_000, (duration_ms % 60_000) / 1000)
    } else {
        format!("{}s", duration_ms / 1000)
    };

    match next_agent {
        Some(next) => format!(
            "{} is done ({}). Next up: {}.",
            agent_name, duration_str, next
        ),
        None => format!(
            "{} is done ({}). That's the last step.",
            agent_name, duration_str
        ),
    }
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len { s } else { &s[..max_len] }
}
```

### How to Test

Unit tests (no CLI invocation needed):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markers_normal() {
        let input = "AGENT_ENTRY: Starting the design.\n\
                      Here is the architecture...\n\
                      Component A talks to Component B.\n\
                      AGENT_EXIT: Design complete.";
        let output = parse_markers(input);
        assert_eq!(output.entry, "Starting the design.");
        assert_eq!(output.exit, "Design complete.");
        assert!(output.content.contains("Component A"));
    }

    #[test]
    fn test_parse_markers_missing() {
        let input = "Just some regular output without markers.";
        let output = parse_markers(input);
        assert_eq!(output.entry, "");
        assert_eq!(output.exit, "");
        assert_eq!(output.content, input);
    }

    #[test]
    fn test_inject_personality_first_step() {
        let prompt = inject_personality_template("Write requirements", &[]);
        assert!(prompt.contains("AGENT_ENTRY:"));
        assert!(prompt.contains("AGENT_EXIT:"));
        assert!(prompt.contains("Write requirements"));
    }
}
```

---

## Area 6: Workflow Parser Updates

**Goal**: Extend the workflow YAML format with new optional fields: `depends_on`, `runtime`, and step `id`. Keep backward compatible — existing workflows without these fields still work.

**Files to modify**:
- `cli/src/core/workflow.rs`

### Updated WorkflowStep Struct

```rust
// cli/src/core/workflow.rs

pub struct WorkflowStep {
    // Existing fields (keep all):
    pub id: String,               // NEW: explicit step ID (was implicit index)
    pub agent: String,
    pub task: String,
    pub checkpoint: bool,
    pub expected_output: String,
    pub next: String,

    // New A2A fields:
    pub depends_on: Vec<String>,  // NEW: step IDs this step depends on
    pub runtime: String,          // NEW: preferred runtime ID (e.g. "claude-opus")
}
```

### YAML Parsing Changes

In the `load_workflow` or equivalent function that parses workflow YAML:

```yaml
# NEW FORMAT (backward compatible):
steps:
  - id: requirements        # NEW (optional — auto-generated if missing)
    agent: pm
    task: "Write product requirements"
    runtime: claude-sonnet   # NEW (optional — falls back to agent default)
    depends_on: []           # NEW (optional — empty = no dependencies)

  - id: design
    agent: architect
    task: "Design the architecture"
    runtime: claude-opus
    depends_on:
      - requirements

  - id: frontend
    agent: frontend-dev
    task: "Build UI components"
    runtime: cursor-composer
    depends_on:
      - design

  - id: backend
    agent: backend-dev
    task: "Build API endpoints"
    runtime: cursor-gpt54
    depends_on:
      - design               # Can run parallel with frontend

  - id: integration
    agent: reviewer
    task: "Review all work"
    runtime: claude-sonnet
    depends_on:
      - frontend
      - backend              # Waits for both
```

### Backward Compatibility Rules

When parsing, apply these defaults for missing fields:

```rust
fn parse_step(yaml: &YamlValue, index: usize) -> WorkflowStep {
    let id = yaml.get_str("id")
        .unwrap_or_else(|| format!("step{}", index));

    let depends_on = yaml.get("depends_on")
        .and_then(|v| v.as_list())
        .map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let runtime = yaml.get_str("runtime")
        .unwrap_or_default();  // Empty = use agent default or registry first-available

    // ... existing field parsing unchanged ...

    WorkflowStep { id, depends_on, runtime, /* ...existing fields... */ }
}
```

### Auto-Generated IDs for Legacy Workflows

If a workflow has no `id` fields:
- Steps get IDs: `step0`, `step1`, `step2`, ...
- If no `depends_on` fields: Steps are treated as a linear chain (`step1` depends on `step0`, etc.)
- This preserves exact v2 behavior — sequential execution, same order

### How to Test

```bash
# Parse a v2 workflow (no new fields) — should still work
rick list workflows

# Parse a v3 workflow (with depends_on/runtime) — should parse new fields
# Create a test workflow YAML with the new fields and verify parsing
```

---

## Area 7: DAG Scheduler

**Goal**: Execute workflow steps respecting dependency order, running independent steps in parallel using stdlib threads.

**Files to create**:
- `cli/src/core/scheduler.rs`

**Files to modify**:
- `cli/src/core/mod.rs` — add `pub mod scheduler;`

### Core Data Structures

```rust
// cli/src/core/scheduler.rs

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Instant;

use crate::a2a::types::*;
use crate::core::runtime::RuntimeRegistry;
use crate::core::workflow::WorkflowStep;
use crate::error::RickError;

/// Result of a single step execution.
pub struct StepResult {
    pub step_id: String,
    pub response: Result<TaskResponse, RickError>,
    pub started_at: Instant,
    pub finished_at: Instant,
}

/// Event sent from worker threads back to the scheduler.
pub enum SchedulerEvent {
    StepStarted { step_id: String },
    StepCompleted(StepResult),
}
```

### The Scheduler

```rust
/// Executes workflow steps respecting a dependency DAG.
/// Independent steps run on separate threads.
pub struct DagScheduler {
    steps: Vec<WorkflowStep>,
    registry: Arc<RuntimeRegistry>,
}

impl DagScheduler {
    pub fn new(steps: Vec<WorkflowStep>, registry: Arc<RuntimeRegistry>) -> Self {
        DagScheduler { steps, registry }
    }

    /// Execute the entire workflow. Returns results for all steps.
    ///
    /// Algorithm:
    /// 1. Build a dependency map: step_id → set of dependency step_ids
    /// 2. Find all steps with no pending dependencies ("ready" steps)
    /// 3. Spawn a thread for each ready step
    /// 4. When a step completes, remove it from all dependency sets
    /// 5. Check if any new steps became ready → spawn them
    /// 6. Repeat until all steps complete or a step fails
    pub fn execute_all(
        &self,
        build_request: impl Fn(&WorkflowStep, &[StepResult]) -> TaskRequest + Send + Sync + 'static,
        on_event: impl Fn(&SchedulerEvent) + Send + 'static,
    ) -> Result<Vec<StepResult>, RickError> {

        // Step 1: Build dependency map
        let mut pending_deps: HashMap<String, HashSet<String>> = HashMap::new();
        let step_map: HashMap<String, &WorkflowStep> = HashMap::new();

        for step in &self.steps {
            let deps: HashSet<String> = step.depends_on.iter().cloned().collect();
            pending_deps.insert(step.id.clone(), deps);
        }

        // Step 2-6: Execute loop
        let (tx, rx) = mpsc::channel::<StepResult>();
        let mut completed: Vec<StepResult> = Vec::new();
        let mut running: HashSet<String> = HashSet::new();
        let mut all_done = false;

        while !all_done {
            // Find ready steps (no pending deps, not running, not completed)
            let completed_ids: HashSet<&str> =
                completed.iter().map(|r| r.step_id.as_str()).collect();

            let ready: Vec<String> = pending_deps.iter()
                .filter(|(id, deps)| {
                    deps.is_empty()
                        && !running.contains(id.as_str())
                        && !completed_ids.contains(id.as_str())
                })
                .map(|(id, _)| id.clone())
                .collect();

            // Spawn a thread for each ready step
            for step_id in &ready {
                let step = self.steps.iter().find(|s| s.id == *step_id).unwrap();
                let runtime = self.registry.resolve_for_step(step);
                let request = build_request(step, &completed);
                let tx = tx.clone();
                let step_id = step_id.clone();

                on_event(&SchedulerEvent::StepStarted {
                    step_id: step_id.clone(),
                });

                running.insert(step_id.clone());

                // Clone what the thread needs
                let runtime = runtime.clone(); // RuntimeBackend must be Send+Sync

                thread::spawn(move || {
                    let started_at = Instant::now();
                    let response = runtime.execute(&request);
                    let finished_at = Instant::now();
                    let _ = tx.send(StepResult {
                        step_id,
                        response,
                        started_at,
                        finished_at,
                    });
                });
            }

            // Wait for any step to complete
            if !running.is_empty() {
                match rx.recv() {
                    Ok(result) => {
                        let step_id = result.step_id.clone();
                        running.remove(&step_id);

                        // Check for failure
                        if let Err(ref e) = result.response {
                            // Step failed — stop everything
                            // (POC: fail-fast. Post-POC: ask user retry/skip/abort)
                            completed.push(result);
                            return Err(RickError::InvalidState(
                                format!("Step '{}' failed: {}", step_id, e)
                            ));
                        }

                        // Remove completed step from all dependency sets
                        for deps in pending_deps.values_mut() {
                            deps.remove(&step_id);
                        }
                        pending_deps.remove(&step_id);

                        on_event(&SchedulerEvent::StepCompleted(
                            // Note: we need to clone or share the result here
                            // Implementation detail — share via Arc or just pass step_id
                            StepResult {
                                step_id: step_id.clone(),
                                response: Ok(TaskResponse { /* ... */ }),
                                started_at: result.started_at,
                                finished_at: result.finished_at,
                            }
                        ));

                        completed.push(result);
                    }
                    Err(_) => break, // All senders dropped
                }
            }

            // Check if all done
            all_done = pending_deps.is_empty() && running.is_empty();
        }

        Ok(completed)
    }
}
```

### Backward Compatibility: Linear Chain

When a v2 workflow has no `depends_on` fields, the scheduler automatically creates a linear dependency chain:

```rust
/// Convert v2 sequential workflow to dependency chain.
/// step0 → step1 → step2 → step3
pub fn linearize_steps(steps: &mut [WorkflowStep]) {
    let has_any_depends = steps.iter().any(|s| !s.depends_on.is_empty());
    if has_any_depends {
        return; // Already has dependencies, don't override
    }

    // Auto-chain: each step depends on the previous one
    for i in 1..steps.len() {
        steps[i].depends_on = vec![steps[i - 1].id.clone()];
    }
}
```

### Progress Display (Simple Text)

Instead of a TUI, print status updates to stderr:

```rust
fn on_event(event: &SchedulerEvent) {
    match event {
        SchedulerEvent::StepStarted { step_id } => {
            eprintln!("  [STARTED]  {} ", step_id);
        }
        SchedulerEvent::StepCompleted(result) => {
            let duration = result.finished_at
                .duration_since(result.started_at);
            let status = if result.response.is_ok() { "DONE" } else { "FAIL" };
            eprintln!(
                "  [{}]  {} ({:.1}s)",
                status,
                result.step_id,
                duration.as_secs_f64(),
            );
        }
    }
}
```

### How to Test

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_linearize_v2_workflow() {
        let mut steps = vec![
            WorkflowStep { id: "s0".into(), depends_on: vec![], ..default() },
            WorkflowStep { id: "s1".into(), depends_on: vec![], ..default() },
            WorkflowStep { id: "s2".into(), depends_on: vec![], ..default() },
        ];
        linearize_steps(&mut steps);
        assert_eq!(steps[0].depends_on, Vec::<String>::new());
        assert_eq!(steps[1].depends_on, vec!["s0"]);
        assert_eq!(steps[2].depends_on, vec!["s1"]);
    }

    #[test]
    fn test_parallel_steps_detected() {
        // frontend and backend both depend on design only
        // They should be identified as runnable in parallel
        let steps = vec![
            WorkflowStep { id: "design".into(), depends_on: vec![], .. },
            WorkflowStep { id: "frontend".into(), depends_on: vec!["design".into()], .. },
            WorkflowStep { id: "backend".into(), depends_on: vec!["design".into()], .. },
        ];
        // After "design" completes, both "frontend" and "backend"
        // should have empty pending deps → both ready simultaneously
    }
}
```

---

## Area 8: Agent Compilation Refactor

**Goal**: Change agent compilation from Claude-specific (outputs `.claude/agents/*.md`) to runtime-agnostic (outputs `AgentPersona` structs used by any backend).

**Files to modify**:
- `cli/src/core/agent.rs`

### Current Behavior (Keep Working)

The current `Agent::compile()` writes to `.claude/agents/rick-{universe}-{agent}.md`. This must still work for backward compatibility. Don't remove it.

### New Method: `compile_persona()`

Add a new method that returns an `AgentPersona` struct instead of writing a file:

```rust
// In cli/src/core/agent.rs

use crate::a2a::types::AgentPersona;

impl Agent {
    /// Compile agent into a runtime-agnostic persona payload.
    /// This is what gets sent to RuntimeBackend via A2A TaskRequest.
    pub fn compile_persona(&self) -> AgentPersona {
        AgentPersona {
            name: self.name.clone(),
            role: self.soul_first_line.clone(),
            soul: self.soul_content.clone().unwrap_or_default(),
            rules: self.rules_content.clone().unwrap_or_default(),
        }
    }

    /// Existing compile() method — KEEP AS-IS for backward compat.
    /// Used by `rick compile` command for Claude Code sub-agent .md files.
    pub fn compile(
        &self,
        universe_name: &str,
        output_dir: &str,
        universe_path: &str,
    ) -> Result<String, RickError> {
        // ... existing implementation unchanged ...
    }
}
```

### Runtime Config in tools.md (Optional)

Add parsing for optional `runtime:` section in agent's `tools.md`:

```yaml
# agents/architect/tools.md
runtime:
  preferred: claude-opus
  fallback:
    - claude-sonnet
    - cursor-composer
```

Parse this when loading the agent:

```rust
pub struct AgentRuntimeConfig {
    pub preferred: String,
    pub fallback: Vec<String>,
}

impl Agent {
    /// Parse optional runtime config from tools.md frontmatter.
    /// Returns None if no runtime section found (agent uses global default).
    pub fn runtime_config(&self) -> Option<AgentRuntimeConfig> {
        let tools = self.tools_content.as_ref()?;
        // Parse YAML frontmatter from tools.md
        // Look for "runtime:" section
        // Return AgentRuntimeConfig if found
        // ...
    }
}
```

### How to Test

```bash
# Existing compile still works:
rick compile

# New compile_persona returns correct struct (unit test):
cargo test
```

---

## Area 9: CLI Command Updates

**Goal**: Update `run`, `next`, `status`, and add `runtimes` commands to use the new scheduler and backends.

**Files to modify**:
- `cli/src/cli/commands.rs`
- `cli/src/core/state.rs`
- `cli/src/main.rs`

### New Command: `rick runtimes`

Lists discovered runtime backends:

```rust
fn cmd_runtimes() -> Result<(), RickError> {
    let registry = RuntimeRegistry::discover();
    let runtimes = registry.list_available();

    if runtimes.is_empty() {
        println!("No runtimes available.");
        println!("Install Claude Code (claude) or Cursor (agent) to get started.");
        return Ok(());
    }

    println!("Available runtimes:\n");
    for card in runtimes {
        let status = if card.available { "available" } else { "unavailable" };
        println!("  {:<20} {:<30} {}", card.runtime_id, card.name, status);
    }
    Ok(())
}
```

### Updated `rick run` Command

The `run` command now uses the DAG scheduler:

```rust
fn cmd_run(workflow_name: &str, force: bool, runtime_override: Option<&str>) -> Result<(), RickError> {
    // 1. Load universe + workflow (existing logic — unchanged)
    let universe = load_active_universe()?;
    let workflow = find_workflow(&universe, workflow_name)?;

    // 2. Discover runtimes
    let registry = Arc::new(RuntimeRegistry::discover());

    // 3. Check that required runtimes are available
    for step in &workflow.steps {
        let runtime_id = if let Some(ovr) = runtime_override {
            ovr.to_string()
        } else if !step.runtime.is_empty() {
            step.runtime.clone()
        } else {
            // Use agent default or first available
            let agent = load_agent(&universe, &step.agent)?;
            agent.runtime_config()
                .map(|c| c.preferred)
                .unwrap_or_else(|| {
                    registry.list_available()
                        .first()
                        .map(|c| c.runtime_id.clone())
                        .unwrap_or_default()
                })
        };

        if registry.get(&runtime_id).is_none() {
            if !force {
                return Err(RickError::NotFound(
                    format!("Runtime '{}' not available for step '{}'. \
                             Use --force to skip or install the runtime.", runtime_id, step.id)
                ));
            }
        }
    }

    // 4. Create workflow state (updated with new fields)
    let mut state = WorkflowState::new(&workflow, &universe);
    state.save()?;

    // 5. Load agents and build personas
    let agents: HashMap<String, Agent> = workflow.steps.iter()
        .map(|s| {
            let agent = load_agent(&universe, &s.agent)?;
            Ok((s.agent.clone(), agent))
        })
        .collect::<Result<_, RickError>>()?;

    // 6. Auto-linearize if no depends_on fields (backward compat)
    let mut steps = workflow.steps.clone();
    crate::core::scheduler::linearize_steps(&mut steps);

    // 7. Execute via scheduler
    let scheduler = DagScheduler::new(steps, registry.clone());
    let results = scheduler.execute_all(
        // build_request closure: turns WorkflowStep into TaskRequest
        |step, completed_results| {
            let agent = &agents[&step.agent];
            let persona = agent.compile_persona();
            let prior_summaries = build_prior_summaries(completed_results);
            let description = crate::core::personality::inject_personality_template(
                &step.task,
                &prior_summaries,
            );

            TaskRequest {
                task_id: format!("{}-{}", state.workflow_id, step.id),
                session_id: state.workflow_id.clone(),
                description,
                context: TaskContext {
                    workflow_id: state.workflow_id.clone(),
                    step_id: step.id.clone(),
                    agent_persona: persona,
                    prior_steps: prior_summaries,
                },
                artifacts: vec![], // POC: no artifact passing yet
            }
        },
        // on_event closure: display progress
        |event| {
            display_scheduler_event(event);
        },
    )?;

    // 8. Display final results with personality
    for result in &results {
        if let Ok(ref response) = result.response {
            let step = workflow.steps.iter().find(|s| s.id == result.step_id).unwrap();
            let agent = &agents[&step.agent];

            // Rick's handoff
            println!("\n\x1b[1mRick:\x1b[0m {}", 
                crate::core::personality::generate_handoff(
                    &agent.name, &agent.soul_first_line, &step.task
                ));

            // Agent ENTRY
            if !response.output.entry.is_empty() {
                println!("\n\x1b[1m{} ({}):\x1b[0m \x1b[3m{}\x1b[0m",
                    agent.name, agent.soul_first_line, response.output.entry);
            }

            // Agent work output
            println!("{}", response.output.content);

            // Agent EXIT
            if !response.output.exit.is_empty() {
                println!("\x1b[3m{}\x1b[0m", response.output.exit);
            }

            // Rick's recap
            println!("\n\x1b[1mRick:\x1b[0m {}",
                crate::core::personality::generate_recap(
                    &agent.name,
                    response.metadata.duration_ms,
                    None, // simplified for now
                ));
        }
    }

    // 9. Update state
    state.status = "completed".to_string();
    state.save()?;

    Ok(())
}
```

### Updated State (state.rs)

Add new fields to track runtime info:

```rust
pub struct StepState {
    pub id: String,
    pub agent: String,
    pub task: String,
    pub status: String,             // pending, running, completed, failed
    // New fields:
    pub runtime_used: String,       // Which runtime executed this step
    pub duration_ms: u64,           // Execution time
    pub tokens_in: u64,             // Input tokens
    pub tokens_out: u64,            // Output tokens
    pub entry: String,              // AGENT_ENTRY line
    pub exit: String,               // AGENT_EXIT line
}
```

### New CLI Flag: `--runtime`

Add a `--runtime` flag to force all steps to use a specific runtime:

```bash
rick run new-feature --runtime claude-sonnet
# Forces all steps to use claude-sonnet regardless of agent config
```

Parse in `main.rs`:
```rust
"run" => {
    let workflow_name = args.next().ok_or("Missing workflow name")?;
    let force = args.any(|a| a == "--force");
    let runtime = args.find(|a| a.starts_with("--runtime="))
        .map(|a| a.strip_prefix("--runtime=").unwrap().to_string());
    cmd_run(&workflow_name, force, runtime.as_deref())?;
}
```

### How to Test

```bash
# List runtimes
rick runtimes

# Run a v2 workflow (should still work sequentially)
rick run existing-workflow

# Run a v3 workflow with dependencies (should run parallel steps)
rick run a2a-demo
```

---

## Area 10: Demo Universe + Integration Test

**Goal**: Create a small demo universe with a workflow that validates the full A2A POC: 4 agents, 4 runtimes, parallel execution.

**Files to create**:
- `universes/a2a-demo/.rick/config.yaml`
- `universes/a2a-demo/agents/pm/soul.md`
- `universes/a2a-demo/agents/pm/rules.md`
- `universes/a2a-demo/agents/pm/tools.md`
- `universes/a2a-demo/agents/architect/soul.md`
- `universes/a2a-demo/agents/architect/rules.md`
- `universes/a2a-demo/agents/architect/tools.md`
- `universes/a2a-demo/agents/frontend-dev/soul.md`
- `universes/a2a-demo/agents/frontend-dev/rules.md`
- `universes/a2a-demo/agents/frontend-dev/tools.md`
- `universes/a2a-demo/agents/backend-dev/soul.md`
- `universes/a2a-demo/agents/backend-dev/rules.md`
- `universes/a2a-demo/agents/backend-dev/tools.md`
- `universes/a2a-demo/agents/reviewer/soul.md`
- `universes/a2a-demo/agents/reviewer/rules.md`
- `universes/a2a-demo/agents/reviewer/tools.md`
- `universes/a2a-demo/workflows/parallel-demo.yaml`

### Universe Config

```yaml
# universes/a2a-demo/.rick/config.yaml
name: a2a-demo
version: "1.0.0"
description: "A2A POC Demo — parallel multi-runtime workflow"
```

### Demo Workflow

```yaml
# universes/a2a-demo/workflows/parallel-demo.yaml
name: "A2A Parallel Demo"
version: "1.0"
description: "4 agents running in parallel on 4 different runtimes"

steps:
  - id: requirements
    agent: pm
    task: "Write a brief product requirement document (2-3 paragraphs) for a simple todo-list web app. Focus on user stories and acceptance criteria."
    runtime: claude-sonnet
    checkpoint: false
    expected_output: "A requirements document"
    next: done

  - id: design
    agent: architect
    task: "Design the high-level architecture for a todo-list web app. Include components, data flow, and tech stack recommendations. Keep it concise (1 page max)."
    runtime: claude-opus
    depends_on:
      - requirements
    checkpoint: false
    expected_output: "An architecture document"
    next: done

  - id: frontend
    agent: frontend-dev
    task: "Write a React component for a todo-list UI. Include add, complete, and delete functionality. Output the code for a single TodoApp.tsx file."
    runtime: cursor-composer
    depends_on:
      - design
    checkpoint: false
    expected_output: "A React component file"
    next: done

  - id: backend
    agent: backend-dev
    task: "Write a simple REST API for a todo-list in Node.js/Express. Include GET, POST, PUT, DELETE endpoints. Output the code for a single server.js file."
    runtime: cursor-gpt54
    depends_on:
      - design
    checkpoint: false
    expected_output: "A Node.js server file"
    next: done

  - id: review
    agent: reviewer
    task: "Review the requirements, architecture, frontend, and backend work. Identify any inconsistencies, missing pieces, or quality issues. Provide a brief review summary."
    runtime: claude-sonnet
    depends_on:
      - frontend
      - backend
    checkpoint: false
    expected_output: "A review summary"
    next: done
```

### Agent Definitions (Minimal for POC)

Each agent needs `soul.md` (persona), `rules.md` (constraints), and `tools.md` (capabilities + runtime config).

**Example: PM Agent**

```markdown
<!-- agents/pm/soul.md -->
Product Manager who turns chaos into clear requirements.
You focus on user value, clear acceptance criteria, and practical scope.
You write concise, actionable requirements — not novels.
```

```markdown
<!-- agents/pm/rules.md -->
- Keep requirements under 500 words
- Always include user stories in "As a... I want... So that..." format
- Always include acceptance criteria
- Do not include implementation details
```

```yaml
# agents/pm/tools.md
runtime:
  preferred: claude-sonnet
  fallback:
    - cursor-composer
```

**Example: Architect Agent**

```markdown
<!-- agents/architect/soul.md -->
Systems Architect who designs for clarity and simplicity.
You favor proven patterns over clever solutions.
You think in components, interfaces, and data flow.
```

```markdown
<!-- agents/architect/rules.md -->
- Keep architecture documents under 1 page
- Always include a component diagram (ASCII)
- Always specify data flow between components
- Justify technology choices briefly
```

```yaml
# agents/architect/tools.md
runtime:
  preferred: claude-opus
  fallback:
    - claude-sonnet
```

**Example: Frontend Dev Agent**

```markdown
<!-- agents/frontend-dev/soul.md -->
Frontend Developer who ships clean, functional React code.
You write components that work on the first try.
You prefer simplicity over abstraction.
```

```markdown
<!-- agents/frontend-dev/rules.md -->
- Output complete, runnable code (not snippets)
- Use TypeScript
- Include basic error handling
- No external dependencies beyond React
```

```yaml
# agents/frontend-dev/tools.md
runtime:
  preferred: cursor-composer
  fallback:
    - claude-sonnet
```

**Example: Backend Dev Agent**

```markdown
<!-- agents/backend-dev/soul.md -->
Backend Developer who builds reliable APIs.
You write clean endpoint handlers with proper HTTP status codes.
You keep things simple — no over-engineering.
```

```markdown
<!-- agents/backend-dev/rules.md -->
- Output complete, runnable code
- Use proper HTTP methods and status codes
- Include basic input validation
- No external dependencies beyond Express
```

```yaml
# agents/backend-dev/tools.md
runtime:
  preferred: cursor-gpt54
  fallback:
    - cursor-composer
    - claude-sonnet
```

**Example: Reviewer Agent**

```markdown
<!-- agents/reviewer/soul.md -->
Code Reviewer who catches what others miss.
You review for consistency, completeness, and quality.
You're constructive — you identify problems AND suggest fixes.
```

```markdown
<!-- agents/reviewer/rules.md -->
- Check consistency between requirements, design, and implementation
- Identify missing error handling or edge cases
- Keep review under 500 words
- Rate overall quality: Pass / Pass with Notes / Needs Revision
```

```yaml
# agents/reviewer/tools.md
runtime:
  preferred: claude-sonnet
  fallback:
    - cursor-composer
```

### Expected POC Execution Flow

```
$ rick run parallel-demo

Available runtimes:
  claude-opus      Claude Code (Opus)         available
  claude-sonnet    Claude Code (Sonnet)       available
  cursor-composer  Cursor (Composer 2 Fast)   available
  cursor-gpt54     Cursor (GPT-5.4)           available

Starting workflow: A2A Parallel Demo (5 steps)
Workflow ID: wf-1712345678

  [STARTED]  requirements (pm @ claude-sonnet)

Rick: Handing this to pm (Product Manager) — Write a brief product requirement...

pm (Product Manager): Let me draft the requirements for this todo app.

<requirements output>

Done with the requirements. Over to the architects.

Rick: pm is done (23s). Next up: architect.

  [STARTED]  design (architect @ claude-opus)

<design completes>

  [STARTED]  frontend (frontend-dev @ cursor-composer)     ← PARALLEL
  [STARTED]  backend (backend-dev @ cursor-gpt54)          ← PARALLEL

<both complete>

  [STARTED]  review (reviewer @ claude-sonnet)

<review completes>

Workflow complete: A2A Parallel Demo
  Total time: 87s
  Steps: 5/5 completed
  Runtimes used: claude-sonnet, claude-opus, cursor-composer, cursor-gpt54
```

### Success Criteria Checklist

| Criteria | How to Verify |
|----------|---------------|
| 4 agents execute | All 5 steps show "completed" in state file |
| 4 different runtimes | State file shows 4 distinct `runtime_used` values |
| Parallel execution | frontend + backend start timestamps overlap |
| ENTRY/EXIT markers work | All 5 steps have non-empty entry and exit in output |
| Rick personality preserved | Handoff and recap lines display correctly |
| v2 workflows still work | Run existing example-issues workflow — unchanged behavior |
| Cross-host works | Run same workflow from Claude Code AND from Cursor terminal |

---

## File Change Summary

### New Files (13)

| File | Area | Purpose |
|------|------|---------|
| `cli/src/a2a/mod.rs` | 1 | A2A module root |
| `cli/src/a2a/types.rs` | 1 | A2A protocol types |
| `cli/src/core/runtime.rs` | 2 | RuntimeBackend trait + registry |
| `cli/src/core/backends/mod.rs` | 3 | Backends module root |
| `cli/src/core/backends/claude.rs` | 3 | Claude CLI backend |
| `cli/src/core/backends/cursor.rs` | 4 | Cursor CLI backend |
| `cli/src/core/personality.rs` | 5 | Personality engine |
| `cli/src/core/scheduler.rs` | 7 | DAG scheduler |
| `universes/a2a-demo/...` (multiple) | 10 | Demo universe + workflow |

### Modified Files (5)

| File | Area | Change |
|------|------|--------|
| `cli/src/main.rs` | 1, 9 | Add `mod a2a`, new commands |
| `cli/src/core/mod.rs` | 2, 5, 7 | Add module declarations |
| `cli/src/core/workflow.rs` | 6 | Add `depends_on`, `runtime`, `id` fields |
| `cli/src/core/agent.rs` | 8 | Add `compile_persona()`, parse `runtime:` config |
| `cli/src/cli/commands.rs` | 9 | Updated `run`, new `runtimes` command |
| `cli/src/core/state.rs` | 9 | Add runtime tracking fields |

### Unchanged Files

| File | Why |
|------|-----|
| `cli/src/error.rs` | Existing error types are sufficient |
| `cli/src/cli/help.rs` | Update help text (minor, not a separate area) |
| `cli/src/parsers/yaml.rs` | Already handles needed YAML features |
| `cli/src/parsers/json.rs` | Already handles needed JSON features |
| `cli/src/core/universe.rs` | Universe loading unchanged |
| `cli/src/core/deps.rs` | Dependency checking unchanged |
| `cli/src/core/template.rs` | Template system unchanged |
| `cli/src/core/resolver.rs` | Resolver unchanged |

---

## Appendix A: JSON Escaping Helper

The hand-rolled JSON parser in `parsers/json.rs` already has `to_json_string()`. You'll also need a string escaping function for building the `--agents` JSON payload:

```rust
/// Escape a string for embedding in JSON.
/// Handles: quotes, backslashes, newlines, tabs, carriage returns.
pub fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c => result.push(c),
        }
    }
    result
}
```

Check if this already exists in `parsers/json.rs`. If so, reuse it. If not, add it there.

---

## Appendix B: RuntimeBackend Thread Safety

The `RuntimeBackend` trait needs to be `Send + Sync` because it's shared across threads. Since all backends just shell out to CLI commands (stateless), this is naturally safe:

```rust
pub trait RuntimeBackend: Send + Sync {
    fn agent_card(&self) -> &AgentCard;
    fn health_check(&self) -> Result<bool, RickError>;
    fn execute(&self, request: &TaskRequest) -> Result<TaskResponse, RickError>;
}
```

The `RuntimeRegistry` should be wrapped in `Arc` when shared with the scheduler:

```rust
let registry = Arc::new(RuntimeRegistry::discover());
// Pass Arc::clone(&registry) to scheduler
```

---

## Appendix C: What A2A Buys You (Future Upgrade Path)

The A2A types defined in Area 1 are currently used in-process only. But they're designed so you can later add:

1. **HTTP transport**: New `HttpRuntimeBackend` that sends `TaskRequest` as JSON over HTTP to a remote adapter server. Zero changes to scheduler, personality engine, or CLI commands.

2. **Remote execution**: Run agents on different machines. Rick sends A2A requests to remote endpoints. Same types, different transport.

3. **Third-party agents**: Other A2A-compatible agents can join Rick workflows. They just need to understand `TaskRequest`/`TaskResponse`.

4. **Federation**: Rick universes on different machines can delegate steps to each other.

The in-process approach for the POC is the right first step. HTTP transport is an incremental addition, not a rewrite.

---

## Implementation Timeline

| Week | Areas | Deliverable |
|------|-------|-------------|
| 1 | 1, 2, 5 | A2A types, RuntimeBackend trait, personality engine |
| 2 | 3, 4 | Claude + Cursor backends (manually testable with `cargo test`) |
| 3 | 6, 7, 8 | Workflow updates, DAG scheduler, agent refactor |
| 4 | 9, 10 | CLI integration, demo universe, end-to-end test |

**Total: 4 weeks**

Go/No-Go checkpoint: **End of Week 2**. If both backends work in isolation (you can send a prompt to Claude and Cursor and get structured responses back), the architecture is validated. Proceed to integration.
