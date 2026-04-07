use std::process::{Command, Stdio};
use crate::a2a::types::*;
use crate::core::runtime::RuntimeBackend;
use crate::error::{RickError, Result};

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

    fn build_agents_json(&self, request: &TaskRequest) -> String {
        // Build inline agent JSON for Claude's --agents flag
        // Format: {"rick-task-agent": {"description": "...", "prompt": "..."}}

        let agent_prompt = format!(
            "{}\n\n{}",
            request.context.agent_persona.soul,
            request.context.agent_persona.rules,
        );

        // Escape the prompt for JSON
        let escaped_prompt = escape_json_string(&agent_prompt);

        format!(
            r#"{{"rick-task-agent":{{"description":"Rick A2A task agent","prompt":"{}"}}}}"#,
            escaped_prompt
        )
    }

    fn parse_claude_json_output(&self, raw: &str) -> Result<ClaudeCliOutput> {
        let parsed = crate::parsers::json::parse_json(raw)
            .map_err(|e| RickError::Parse(format!("Claude JSON: {}", e)))?;

        let result_text = parsed.get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let duration_ms = parsed.get("duration_ms")
            .and_then(|v| match v {
                crate::parsers::json::JsonValue::Number(n) => Some(*n as u64),
                _ => None,
            })
            .unwrap_or(0);

        let usage = parsed.get("usage");
        let tokens_in = usage
            .and_then(|u| u.get("inputTokens"))
            .and_then(|v| match v {
                crate::parsers::json::JsonValue::Number(n) => Some(*n as u64),
                _ => None,
            })
            .unwrap_or(0);
        let tokens_out = usage
            .and_then(|u| u.get("outputTokens"))
            .and_then(|v| match v {
                crate::parsers::json::JsonValue::Number(n) => Some(*n as u64),
                _ => None,
            })
            .unwrap_or(0);

        Ok(ClaudeCliOutput {
            result_text,
            duration_ms,
            tokens_in,
            tokens_out,
        })
    }
}

impl RuntimeBackend for ClaudeCliBackend {
    fn agent_card(&self) -> &AgentCard {
        &self.card
    }

    fn health_check(&self) -> Result<bool> {
        let output = Command::new("claude")
            .arg("--version")
            .output()
            .map_err(RickError::Io)?;
        Ok(output.status.success())
    }

    fn execute(&self, request: &TaskRequest) -> Result<TaskResponse> {
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

struct ClaudeCliOutput {
    result_text: String,
    duration_ms: u64,
    tokens_in: u64,
    tokens_out: u64,
}

/// Escape a string for embedding in JSON.
/// Handles: quotes, backslashes, control characters (required by JSON spec).
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),  // backspace
            '\x0C' => result.push_str("\\f"),  // form feed
            // Other control characters (0x00-0x1F)
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}
