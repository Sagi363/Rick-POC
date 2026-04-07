use crate::a2a::types::*;
use crate::core::agent::{RuntimeSpec, AgentRuntimeConfig};
use crate::core::backends::claude::ClaudeCliBackend;
use crate::core::backends::cursor::CursorCliBackend;
use crate::error::{RickError, Result};
use std::process::Command;

/// Supported tool names. Only these are valid in RuntimeSpec.tool.
const TOOL_CLAUDE: &str = "claude";
const TOOL_CURSOR: &str = "cursor";

/// Default model per tool (used when no runtime is specified anywhere).
const DEFAULT_CLAUDE_MODEL: &str = "sonnet";
const DEFAULT_CURSOR_MODEL: &str = "auto";

/// A runtime backend that can execute agent tasks.
/// Each implementation wraps a specific CLI tool.
/// Must be Send + Sync for thread-safe sharing across scheduler threads.
pub trait RuntimeBackend: Send + Sync {
    /// Returns the AgentCard describing this runtime's capabilities.
    fn agent_card(&self) -> &AgentCard;

    /// Checks if this runtime is available (CLI installed, authenticated).
    fn health_check(&self) -> Result<bool>;

    /// Executes a task and returns the response.
    /// This is a BLOCKING call — it runs the CLI tool and waits for output.
    /// Parallel execution is handled by the scheduler spawning threads.
    fn execute(&self, request: &TaskRequest) -> Result<TaskResponse>;
}

/// Discovers available CLI tools and creates backends on demand.
/// Does NOT hardcode model names — models come from agent config or workflow YAML.
pub struct RuntimeRegistry {
    pub claude_available: bool,
    pub cursor_available: bool,
}

impl RuntimeRegistry {
    /// Auto-detect available CLI tools by checking PATH.
    pub fn discover() -> Self {
        RuntimeRegistry {
            claude_available: is_claude_available(),
            cursor_available: is_cursor_available(),
        }
    }

    /// Create a backend for any tool+model combo.
    /// Validates tool name. Model is passed through to CLI (validated at execution time).
    pub fn create_backend(&self, spec: &RuntimeSpec) -> Result<Box<dyn RuntimeBackend>> {
        let runtime_id = spec.id();
        match spec.tool.as_str() {
            TOOL_CLAUDE => {
                if !self.claude_available {
                    return Err(RickError::NotFound(
                        "Claude CLI ('claude') not found on PATH. Install Claude Code.".to_string(),
                    ));
                }
                Ok(Box::new(ClaudeCliBackend::new(&runtime_id, &spec.model)))
            }
            TOOL_CURSOR => {
                if !self.cursor_available {
                    return Err(RickError::NotFound(
                        "Cursor CLI ('agent') not found on PATH. Install Cursor.".to_string(),
                    ));
                }
                Ok(Box::new(CursorCliBackend::new(
                    &runtime_id,
                    &spec.model,
                    &format!("Cursor ({})", spec.model),
                )))
            }
            other => Err(RickError::InvalidState(format!(
                "Unsupported runtime tool '{}'. Supported: 'claude', 'cursor'.",
                other,
            ))),
        }
    }

    /// Resolve a runtime from: step override → agent config → default.
    /// Returns a ready-to-use backend.
    pub fn resolve(
        &self,
        step_runtime: Option<&RuntimeSpec>,
        agent_config: Option<&AgentRuntimeConfig>,
    ) -> Result<Box<dyn RuntimeBackend>> {
        // 1. Step-level override wins
        if let Some(spec) = step_runtime {
            return self.create_backend(spec);
        }

        // 2. Agent preferred runtime
        if let Some(config) = agent_config {
            match self.create_backend(&config.preferred) {
                Ok(backend) => return Ok(backend),
                Err(_) => {
                    // 3. Try agent fallbacks
                    for fb in &config.fallback {
                        if let Ok(backend) = self.create_backend(fb) {
                            return Ok(backend);
                        }
                    }
                }
            }
        }

        // 4. Default: first available tool with default model
        if self.claude_available {
            let spec = RuntimeSpec {
                tool: TOOL_CLAUDE.to_string(),
                model: DEFAULT_CLAUDE_MODEL.to_string(),
            };
            return self.create_backend(&spec);
        }
        if self.cursor_available {
            let spec = RuntimeSpec {
                tool: TOOL_CURSOR.to_string(),
                model: DEFAULT_CURSOR_MODEL.to_string(),
            };
            return self.create_backend(&spec);
        }

        Err(RickError::NotFound(
            "No runtimes available. Install Claude Code ('claude') or Cursor ('agent').".to_string(),
        ))
    }

    /// List available tools (for display).
    pub fn list_available_tools(&self) -> Vec<(&str, bool)> {
        vec![
            (TOOL_CLAUDE, self.claude_available),
            (TOOL_CURSOR, self.cursor_available),
        ]
    }

    /// Check if a specific tool is available.
    pub fn is_tool_available(&self, tool: &str) -> bool {
        match tool {
            TOOL_CLAUDE => self.claude_available,
            TOOL_CURSOR => self.cursor_available,
            _ => false,
        }
    }
}

/// Check if Claude CLI is available
fn is_claude_available() -> bool {
    Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if Cursor CLI is available
fn is_cursor_available() -> bool {
    Command::new("agent")
        .arg("--version")
        .output()
        .map(|o| {
            if !o.status.success() {
                return false;
            }
            let version = String::from_utf8_lossy(&o.stdout);
            // TODO: More robust check - this heuristic assumes Cursor format "YYYY.MM.DD-hash"
            version.contains('-')
        })
        .unwrap_or(false)
}
