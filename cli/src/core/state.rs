use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{RickError, Result};
use crate::parsers::json::{self, JsonValue};

/// Represents the state of a running workflow.
#[derive(Debug)]
pub struct WorkflowState {
    pub workflow_id: String,
    pub workflow_name: String,
    pub universe_name: String,
    pub status: String,
    pub current_step: usize,
    pub total_steps: usize,
    pub steps: Vec<StepState>,
}

#[derive(Debug)]
pub struct StepState {
    pub id: String,
    pub agent: String,
    pub task: String,
    pub status: String,
}

impl WorkflowState {
    /// Generate a new workflow ID based on timestamp.
    pub fn new_id() -> String {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("wf-{}", ts)
    }

    /// Save this state to a JSON file.
    pub fn save(&self, state_dir: &Path) -> Result<PathBuf> {
        fs::create_dir_all(state_dir)?;
        let file_path = state_dir.join(format!("{}.json", self.workflow_id));

        let step_values: Vec<JsonValue> = self
            .steps
            .iter()
            .map(|s| {
                JsonValue::Object(vec![
                    ("id".to_string(), JsonValue::String(s.id.clone())),
                    ("agent".to_string(), JsonValue::String(s.agent.clone())),
                    ("task".to_string(), JsonValue::String(s.task.clone())),
                    ("status".to_string(), JsonValue::String(s.status.clone())),
                ])
            })
            .collect();

        let state_json = JsonValue::Object(vec![
            (
                "workflow_id".to_string(),
                JsonValue::String(self.workflow_id.clone()),
            ),
            (
                "workflow_name".to_string(),
                JsonValue::String(self.workflow_name.clone()),
            ),
            (
                "universe".to_string(),
                JsonValue::String(self.universe_name.clone()),
            ),
            (
                "status".to_string(),
                JsonValue::String(self.status.clone()),
            ),
            (
                "current_step".to_string(),
                JsonValue::Number(self.current_step as f64),
            ),
            (
                "total_steps".to_string(),
                JsonValue::Number(self.total_steps as f64),
            ),
            ("steps".to_string(), JsonValue::Array(step_values)),
        ]);

        let content = json::to_json_pretty(&state_json, 0);
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Load a state from a JSON file.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let parsed = json::parse_json(&content)?;

        let workflow_id = parsed
            .get("workflow_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let workflow_name = parsed
            .get("workflow_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let universe_name = parsed
            .get("universe")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let status = parsed
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let current_step = parsed
            .get("current_step")
            .and_then(|v| match v {
                JsonValue::Number(n) => Some(*n as usize),
                _ => None,
            })
            .unwrap_or(0);
        let total_steps = parsed
            .get("total_steps")
            .and_then(|v| match v {
                JsonValue::Number(n) => Some(*n as usize),
                _ => None,
            })
            .unwrap_or(0);

        let mut steps = Vec::new();
        if let Some(JsonValue::Array(step_arr)) = parsed.get("steps") {
            for s in step_arr {
                steps.push(StepState {
                    id: s.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    agent: s.get("agent").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    task: s.get("task").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    status: s.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string(),
                });
            }
        }

        Ok(WorkflowState {
            workflow_id,
            workflow_name,
            universe_name,
            status,
            current_step,
            total_steps,
            steps,
        })
    }
}

/// Load all active workflow states from the state directory.
pub fn load_all_states(state_dir: &Path) -> Result<Vec<WorkflowState>> {
    if !state_dir.exists() {
        return Ok(Vec::new());
    }

    let mut states = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(state_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();

    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for entry in entries {
        match WorkflowState::load(&entry.path()) {
            Ok(state) => states.push(state),
            Err(_) => continue,
        }
    }

    Ok(states)
}
