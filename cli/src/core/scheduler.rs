use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;

use crate::a2a::types::*;
use crate::core::agent::AgentRuntimeConfig;
use crate::core::runtime::{RuntimeBackend, RuntimeRegistry};
use crate::core::workflow::WorkflowStep;
use crate::error::{RickError, Result};

/// Result of a single step execution.
pub struct StepResult {
    pub step_id: String,
    pub response: Result<TaskResponse>,
    pub started_at: Instant,
    pub finished_at: Instant,
}

/// Event sent from the scheduler to the display layer.
pub enum SchedulerEvent {
    StepStarted {
        step_id: String,
        runtime_id: String,
    },
    StepCompleted {
        step_id: String,
        success: bool,
        entry: String,
        content: String,
        exit: String,
        duration_ms: u64,
        runtime_id: String,
    },
    StepFailed {
        step_id: String,
        error: String,
        runtime_id: String,
    },
}

/// Executes workflow steps respecting a dependency DAG.
/// Independent steps run on separate threads.
pub struct DagScheduler {
    steps: Vec<WorkflowStep>,
    registry: Arc<RuntimeRegistry>,
    /// Agent runtime configs, keyed by agent name.
    agent_configs: HashMap<String, AgentRuntimeConfig>,
}

impl DagScheduler {
    pub fn new(
        steps: Vec<WorkflowStep>,
        registry: Arc<RuntimeRegistry>,
        agent_configs: HashMap<String, AgentRuntimeConfig>,
    ) -> Self {
        DagScheduler { steps, registry, agent_configs }
    }

    pub fn execute_all<F, G>(
        &self,
        mut build_request: F,
        mut on_event: G,
    ) -> Result<Vec<StepResult>>
    where
        F: FnMut(&WorkflowStep, &[StepResult]) -> Result<TaskRequest> + Send + 'static,
        G: FnMut(&SchedulerEvent) + Send + 'static,
    {
        // Build dependency map
        let mut pending_deps: HashMap<String, HashSet<String>> = HashMap::new();
        let mut step_map: HashMap<String, WorkflowStep> = HashMap::new();

        for step in &self.steps {
            let deps: HashSet<String> = step.depends_on.iter().cloned().collect();
            pending_deps.insert(step.id.clone(), deps);
            step_map.insert(step.id.clone(), step.clone());
        }

        let (tx, rx) = mpsc::channel::<StepResult>();
        let mut completed: Vec<StepResult> = Vec::new();
        let mut running: HashSet<String> = HashSet::new();

        loop {
            let completed_ids: HashSet<&str> =
                completed.iter().map(|r| r.step_id.as_str()).collect();

            let ready: Vec<String> = pending_deps
                .iter()
                .filter(|(id, deps)| {
                    deps.is_empty()
                        && !running.contains(id.as_str())
                        && !completed_ids.contains(id.as_str())
                })
                .map(|(id, _)| id.clone())
                .collect();

            for step_id in &ready {
                let step = step_map.get(step_id).unwrap().clone();

                // Resolve runtime: step override → agent config → default
                let agent_config = self.agent_configs.get(&step.agent);
                let backend = self.registry.resolve(
                    step.runtime.as_ref(),
                    agent_config,
                )?;

                let runtime_id = backend.agent_card().runtime_id.clone();

                // Build request on main thread (needs completed results)
                let request = build_request(&step, &completed)?;

                let tx = tx.clone();
                let step_id_clone = step_id.clone();

                on_event(&SchedulerEvent::StepStarted {
                    step_id: step_id.clone(),
                    runtime_id: runtime_id.clone(),
                });

                running.insert(step_id.clone());

                // Spawn thread with owned backend
                thread::spawn(move || {
                    let started_at = Instant::now();
                    let response = backend.execute(&request);
                    let finished_at = Instant::now();

                    let _ = tx.send(StepResult {
                        step_id: step_id_clone,
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

                        for deps in pending_deps.values_mut() {
                            deps.remove(&step_id);
                        }
                        pending_deps.remove(&step_id);

                        match &result.response {
                            Ok(ref resp) => {
                                let duration_ms = result.finished_at
                                    .duration_since(result.started_at)
                                    .as_millis() as u64;

                                let event = SchedulerEvent::StepCompleted {
                                    step_id: result.step_id.clone(),
                                    success: true,
                                    entry: resp.output.entry.clone(),
                                    content: resp.output.content.clone(),
                                    exit: resp.output.exit.clone(),
                                    duration_ms,
                                    runtime_id: resp.metadata.runtime_id.clone(),
                                };
                                on_event(&event);
                            }
                            Err(ref e) => {
                                let event = SchedulerEvent::StepFailed {
                                    step_id: result.step_id.clone(),
                                    error: format!("{}", e),
                                    runtime_id: String::new(),
                                };
                                on_event(&event);

                                return Err(RickError::InvalidState(format!(
                                    "Step '{}' failed: {}",
                                    step_id, e
                                )));
                            }
                        }

                        completed.push(result);
                    }
                    Err(_) => break,
                }
            }

            if pending_deps.is_empty() && running.is_empty() {
                break;
            }

            if running.is_empty() && ready.is_empty() && !pending_deps.is_empty() {
                let stuck_steps: Vec<String> = pending_deps.keys().cloned().collect();
                return Err(RickError::InvalidState(format!(
                    "Workflow stuck - circular dependencies or missing steps: {:?}",
                    stuck_steps
                )));
            }
        }

        Ok(completed)
    }
}

/// Convert v2 sequential workflow to dependency chain (re-exported from workflow module)
pub use crate::core::workflow::linearize_steps;
