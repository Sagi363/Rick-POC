use std::process::{Command, Stdio};
use crate::a2a::types::*;
use crate::core::runtime::RuntimeBackend;
use crate::error::{RickError, Result};

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

    fn parse_cursor_json_output(&self, raw: &str) -> Result<CursorCliOutput> {
        let parsed = crate::parsers::json::parse_json(raw)
            .map_err(|e| RickError::Parse(format!("Cursor JSON: {}", e)))?;

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

        Ok(CursorCliOutput {
            result_text,
            duration_ms,
            tokens_in,
            tokens_out,
        })
    }
}

impl RuntimeBackend for CursorCliBackend {
    fn agent_card(&self) -> &AgentCard {
        &self.card
    }

    fn health_check(&self) -> Result<bool> {
        let output = Command::new("agent")
            .arg("--version")
            .output()
            .map_err(RickError::Io)?;
        let version = String::from_utf8_lossy(&output.stdout);
        Ok(output.status.success() && version.contains('-'))
    }

    fn execute(&self, request: &TaskRequest) -> Result<TaskResponse> {
        // 1. Build full prompt (persona + personality template + task)
        let full_prompt = self.build_full_prompt(request);

        // 2. Invoke cursor agent CLI
        let output = Command::new("agent")
            .arg("-p")
            .arg("--model").arg(&self.model)
            .arg("--output-format").arg("json")
            .arg(&full_prompt)
            .stdin(Stdio::null())  // Non-interactive
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

struct CursorCliOutput {
    result_text: String,
    duration_ms: u64,
    tokens_in: u64,
    tokens_out: u64,
}
