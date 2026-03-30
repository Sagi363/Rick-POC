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
    /// For composed workflows: nested phase state.
    pub phases: Option<Vec<PhaseState>>,
    pub current_phase: Option<usize>,
    pub total_phases: Option<usize>,
}

#[derive(Debug)]
pub struct StepState {
    pub id: String,
    pub agent: String,
    pub task: String,
    pub status: String,
}

/// Represents a composition phase (a `uses` step with nested child steps).
#[derive(Debug)]
pub struct PhaseState {
    pub id: String,
    pub uses: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub current_step: usize,
    pub total_steps: usize,
    pub steps: Vec<StepState>,
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

        let mut fields = vec![
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
        ];

        if let Some(ref phases) = self.phases {
            // Nested (composed) schema
            fields.push((
                "current_phase".to_string(),
                JsonValue::Number(self.current_phase.unwrap_or(0) as f64),
            ));
            fields.push((
                "total_phases".to_string(),
                JsonValue::Number(self.total_phases.unwrap_or(0) as f64),
            ));

            let phase_values: Vec<JsonValue> = phases.iter().map(|p| {
                let step_values: Vec<JsonValue> = p.steps.iter().map(|s| {
                    JsonValue::Object(vec![
                        ("id".to_string(), JsonValue::String(s.id.clone())),
                        ("agent".to_string(), JsonValue::String(s.agent.clone())),
                        ("task".to_string(), JsonValue::String(s.task.clone())),
                        ("status".to_string(), JsonValue::String(s.status.clone())),
                    ])
                }).collect();

                let mut phase_fields = vec![
                    ("id".to_string(), JsonValue::String(p.id.clone())),
                ];
                if let Some(ref u) = p.uses {
                    phase_fields.push(("uses".to_string(), JsonValue::String(u.clone())));
                } else {
                    phase_fields.push(("uses".to_string(), JsonValue::Null));
                }
                if let Some(ref d) = p.description {
                    phase_fields.push(("description".to_string(), JsonValue::String(d.clone())));
                }
                phase_fields.push(("status".to_string(), JsonValue::String(p.status.clone())));
                phase_fields.push(("current_step".to_string(), JsonValue::Number(p.current_step as f64)));
                phase_fields.push(("total_steps".to_string(), JsonValue::Number(p.total_steps as f64)));
                phase_fields.push(("steps".to_string(), JsonValue::Array(step_values)));

                JsonValue::Object(phase_fields)
            }).collect();

            fields.push(("phases".to_string(), JsonValue::Array(phase_values)));
        } else {
            // Flat (non-composed) schema
            fields.push((
                "current_step".to_string(),
                JsonValue::Number(self.current_step as f64),
            ));
            fields.push((
                "total_steps".to_string(),
                JsonValue::Number(self.total_steps as f64),
            ));

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

            fields.push(("steps".to_string(), JsonValue::Array(step_values)));
        }

        let state_json = JsonValue::Object(fields);
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

        // Parse nested phases (composed workflows)
        let phases = if let Some(JsonValue::Array(phase_arr)) = parsed.get("phases") {
            let mut phase_list = Vec::new();
            for p in phase_arr {
                let uses = p.get("uses").and_then(|v| v.as_str()).map(|s| s.to_string());
                let description = p.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                let p_status = p.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string();
                let p_current = p.get("current_step").and_then(|v| match v {
                    JsonValue::Number(n) => Some(*n as usize),
                    _ => None,
                }).unwrap_or(0);
                let p_total = p.get("total_steps").and_then(|v| match v {
                    JsonValue::Number(n) => Some(*n as usize),
                    _ => None,
                }).unwrap_or(0);

                let mut p_steps = Vec::new();
                if let Some(JsonValue::Array(s_arr)) = p.get("steps") {
                    for s in s_arr {
                        p_steps.push(StepState {
                            id: s.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            agent: s.get("agent").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            task: s.get("task").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            status: s.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string(),
                        });
                    }
                }

                phase_list.push(PhaseState {
                    id: p.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    uses,
                    description,
                    status: p_status,
                    current_step: p_current,
                    total_steps: p_total,
                    steps: p_steps,
                });
            }
            Some(phase_list)
        } else {
            None
        };

        let current_phase = parsed
            .get("current_phase")
            .and_then(|v| match v {
                JsonValue::Number(n) => Some(*n as usize),
                _ => None,
            });
        let total_phases = parsed
            .get("total_phases")
            .and_then(|v| match v {
                JsonValue::Number(n) => Some(*n as usize),
                _ => None,
            });

        Ok(WorkflowState {
            workflow_id,
            workflow_name,
            universe_name,
            status,
            current_step,
            total_steps,
            steps,
            phases,
            current_phase,
            total_phases,
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
