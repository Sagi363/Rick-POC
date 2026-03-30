use std::fs;
use std::path::Path;

use crate::error::{RickError, Result};
use crate::core::universe::Universe;
use crate::parsers::yaml;

/// A single step in a workflow.
#[derive(Debug, Clone)]
pub struct WorkflowStep {
    pub id: String,
    pub agent: String,
    pub task: String,
    pub checkpoint: bool,
    pub expected_output: String,
    pub next: String,
    /// If set, this step embeds another workflow (composition).
    pub uses: Option<String>,
    /// Parameter mappings for composed workflows (child_param -> template).
    pub params: Option<Vec<(String, String)>>,
    /// Human-readable description of this phase.
    pub description: Option<String>,
    /// Controls pause between phases (not child internal steps).
    pub auto_continue: Option<bool>,
}

/// A workflow definition.
#[derive(Debug)]
pub struct Workflow {
    pub name: String,
    pub version: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub file_name: String,
}

impl Workflow {
    /// Parse a workflow from a YAML file.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let parsed = yaml::parse_yaml(&content)?;

        let name = parsed
            .get_str("name")
            .unwrap_or("unknown")
            .to_string();
        let version = parsed
            .get_str("version")
            .unwrap_or("1.0")
            .to_string();
        let description = parsed
            .get_str("description")
            .unwrap_or("")
            .to_string();

        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut steps = Vec::new();
        if let Some(steps_val) = parsed.get("steps") {
            if let Some(step_list) = steps_val.as_list() {
                for step_val in step_list {
                    let id = step_val.get_str("id").unwrap_or("").to_string();
                    let agent = step_val.get_str("agent").unwrap_or("").to_string();
                    let task = step_val.get_str("task").unwrap_or("").to_string();
                    let checkpoint = step_val
                        .get("checkpoint")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let expected_output = step_val
                        .get_str("expected_output")
                        .unwrap_or("")
                        .to_string();
                    let next = step_val.get_str("next").unwrap_or("end").to_string();

                    let uses = step_val.get_str("uses").map(|s| s.to_string());
                    let description = step_val.get_str("description").map(|s| s.to_string());
                    let auto_continue = step_val
                        .get("auto_continue")
                        .and_then(|v| v.as_bool());

                    let params = step_val.get("params").and_then(|p| {
                        if let Some(entries) = p.as_map() {
                            let pairs: Vec<(String, String)> = entries
                                .iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                                .collect();
                            if pairs.is_empty() { None } else { Some(pairs) }
                        } else {
                            None
                        }
                    });

                    steps.push(WorkflowStep {
                        id,
                        agent,
                        task,
                        checkpoint,
                        expected_output,
                        next,
                        uses,
                        params,
                        description,
                        auto_continue,
                    });
                }
            }
        }

        Ok(Workflow {
            name,
            version,
            description,
            steps,
            file_name,
        })
    }

    /// Returns true if any step uses workflow composition (`uses` keyword).
    pub fn has_composition(&self) -> bool {
        self.steps.iter().any(|s| s.uses.is_some())
    }
}

impl WorkflowStep {
    /// Returns true if this step is a composition phase (has `uses`).
    pub fn is_phase(&self) -> bool {
        self.uses.is_some()
    }
}

/// Load all workflows from a Universe.
pub fn load_workflows(universe: &Universe) -> Result<Vec<Workflow>> {
    let workflows_dir = universe.workflows_dir();
    if !workflows_dir.exists() {
        return Ok(Vec::new());
    }

    let mut workflows = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(&workflows_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "yaml" || ext == "yml")
                .unwrap_or(false)
        })
        .collect();

    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for entry in entries {
        match Workflow::load(&entry.path()) {
            Ok(wf) => workflows.push(wf),
            Err(_) => continue,
        }
    }

    Ok(workflows)
}
