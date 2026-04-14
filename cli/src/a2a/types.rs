/// A2A Protocol Types
///
/// These types represent the Agent2Agent protocol messages used internally
/// by Rick for multi-runtime orchestration. They flow in-process between
/// Rick's modules (not over HTTP in the POC).

/// Describes what a runtime can do. Used for discovery/health checks.
#[derive(Clone, Debug)]
pub struct AgentCard {
    pub runtime_id: String,       // e.g. "claude-opus", "cursor-composer"
    pub name: String,             // e.g. "Claude Code (Opus 4.6)"
    pub tool: String,             // e.g. "claude-code", "cursor"
    pub model: String,            // e.g. "opus", "composer-2-fast"
    pub capabilities: Vec<String>, // e.g. ["task-execution", "code-generation"]
    pub available: bool,          // true if CLI tool is installed and authed
}

/// What Rick sends to a runtime to execute a step.
#[derive(Clone, Debug)]
pub struct TaskRequest {
    pub task_id: String,          // Unique ID for this task (e.g. "wf-12345-step-design")
    pub session_id: String,       // Workflow run ID (groups related tasks)
    pub description: String,      // The full prompt (personality + context + task)
    pub context: TaskContext,     // Structured context for the agent
    pub artifacts: Vec<Artifact>, // Files/content from previous steps
}

/// Context passed to the agent within a TaskRequest.
#[derive(Clone, Debug)]
pub struct TaskContext {
    pub workflow_id: String,
    pub step_id: String,
    pub agent_persona: AgentPersona,
    pub prior_steps: Vec<PriorStepSummary>,
}

/// Agent identity info, compiled from soul.md/rules.md.
#[derive(Clone, Debug)]
pub struct AgentPersona {
    pub name: String,             // Agent name (e.g. "Neo")
    pub role: String,             // First line of soul.md (e.g. "Architect")
    pub soul: String,             // Full soul.md content
    pub rules: String,            // Full rules.md content
    pub extra_files: Vec<(String, String)>, // Additional .md files (filename, content)
}

/// Summary of a completed prior step, used for context passing.
#[derive(Clone, Debug)]
pub struct PriorStepSummary {
    pub step_id: String,
    pub agent: String,
    pub role: String,
    pub entry: String,            // AGENT_ENTRY line
    pub exit: String,             // AGENT_EXIT line
    pub summary: String,          // Brief summary of what was done
}

/// A file or content artifact passed between steps.
#[derive(Clone, Debug)]
pub struct Artifact {
    pub id: String,               // e.g. "requirements-md"
    pub name: String,             // e.g. "requirements.md"
    pub content: String,          // File content (inline for small files)
    pub mime_type: String,        // e.g. "text/markdown"
}

/// What a runtime returns after executing a task.
#[derive(Clone, Debug)]
pub struct TaskResponse {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: TaskOutput,
    pub artifacts: Vec<Artifact>,  // Files created/modified by the agent
    pub metadata: TaskMetadata,
}

/// Status of a completed task.
#[derive(Clone, Debug)]
pub enum TaskStatus {
    Completed,
    Failed(String),               // Error message
    Partial(String),              // What was completed before failure
}

/// Structured output with personality markers.
#[derive(Clone, Debug)]
pub struct TaskOutput {
    pub entry: String,            // AGENT_ENTRY content (empty if not found)
    pub content: String,          // Main work output
    pub exit: String,             // AGENT_EXIT content (empty if not found)
    pub raw: String,              // Full unprocessed output from the LLM
}

/// Execution metrics.
#[derive(Clone, Debug)]
pub struct TaskMetadata {
    pub model_used: String,       // Actual model that ran (e.g. "claude-opus-4-6")
    pub runtime_id: String,       // Which runtime was used
    pub duration_ms: u64,         // Wall clock time
    pub tokens_in: u64,           // Input tokens (0 if unknown)
    pub tokens_out: u64,          // Output tokens (0 if unknown)
}
