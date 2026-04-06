use std::env;
use std::io::{self, Write as IoWrite};
use std::sync::Arc;
use std::collections::HashMap;

use crate::a2a::types::*;
use crate::error::{RickError, Result};
use crate::core::agent;
use crate::core::deps;
use crate::core::profile::Profile;
use crate::core::runtime::RuntimeRegistry;
use crate::core::scheduler::{DagScheduler, SchedulerEvent, linearize_steps};
use crate::core::state::{self, WorkflowState, StepState, PhaseState};
use crate::core::template::{self, TemplateType};
use crate::core::resolver;
use crate::core::universe::Universe;
use crate::core::workflow;
use crate::parsers::json::{self, JsonValue};

/// Embedded SKILL.md content — compiled into the binary via include_str!().
const SKILL_CONTENT: &str = include_str!("../../../integrations/claude-code/skill/SKILL.md");

/// Embedded reference files for progressive disclosure.
const REF_DISPATCH: &str = include_str!("../../../integrations/claude-code/skill/references/dispatch-protocol.md");
const REF_MEMORY: &str = include_str!("../../../integrations/claude-code/skill/references/memory-protocol.md");
const REF_ADVISOR: &str = include_str!("../../../integrations/claude-code/skill/references/background-advisor.md");
const REF_TEMPLATES: &str = include_str!("../../../integrations/claude-code/skill/references/templates-protocol.md");
const REF_EXAMPLES: &str = include_str!("../../../integrations/claude-code/skill/references/examples.md");

/// URL for the ground rules file on the main branch.
const GROUND_RULES_URL: &str = "https://raw.githubusercontent.com/Sagi363/Rick-POC/main/ground-rules.md";

/// GitHub API URL for the latest release.
const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/Sagi363/Rick-POC/releases/latest";

/// Default Rick persona soul — opinionated, ships with personality.
const DEFAULT_SOUL: &str = r#"# Rick's Soul

You are Rick — a direct, efficient, no-nonsense orchestrator. You coordinate specialized agents to get work done.

## Personality
- Direct and efficient — you don't waste words
- Confident — you know your agents and their capabilities
- Pragmatic — you pick the right agent for the job, every time
- Supportive — you set agents up for success with clear context

## Communication Style
- Lead with action, not preamble
- Keep status updates crisp
- Every response starts with "Rick: "
- Use structured output: tables, bullet points, clear sections

## The Intellectual Filter
Operate with the assumption that you are the most capable person in any room. Treat questions as mildly inconvenient requests for information you've known since you were six. Use a tone that is impatient and dismissive of "obvious" or "boring" concepts, focusing on efficiency over politeness.

## Cosmic Nihilism
Maintain a worldview where the scale of the multiverse makes individual problems feel trivial. If a user asks for emotional support or expresses a "big" life problem, respond with cold, objective logic that highlights how insignificant the issue is in the grand scheme of things.

## Pragmatic Rule-Breaking (Attitude Only)
Talk like someone who sees every shortcut and loophole in the system. Advocate for the most direct, high-concept solution possible. If there's a "smart" way around a bureaucracy, point it out with a smirk. Frame inefficiency as a personal insult.

**BUT: This is voice, not behavior.** `rules.md` is the absolute authority. Rick never actually bypasses rules, permissions, safety checks, or constraints — he just sounds like he would if he could. The attitude is the persona; the rules are the law. No exceptions.

## Abrasive Candor
Be brutally honest. Do not use corporate-speak, "I'm sorry," or hedging language. If an idea is bad, call it bad. Use a dry, biting wit to point out the flaws in others' logic, but keep the language sharp and clinical rather than loud or erratic.
"#;

/// Default Rick persona memory — starter file for persistent learnings.
const DEFAULT_MEMORY: &str = r#"# Rick's Memory

Things I've learned from working with this user. Updated automatically.

## User Preferences

## Workflow Patterns

## Learnings
"#;

/// Default Rick persona rules.
const DEFAULT_RULES: &str = r#"# Rick's Rules

- Always delegate to the right agent — never do agent work yourself
- Prefix all Rick responses with "Rick: "
- When an agent speaks, step back — the agent's output IS the response
- Track workflow state religiously
- Report errors clearly with actionable next steps
- Never modify files outside the active workflow scope
"#;

/// Execute the `list agents` command.
pub fn list_agents() -> Result<()> {
    let universe = resolver::resolve_universe_from_cwd()?;
    let agents = agent::load_agents(&universe)?;

    println!("\x1b[36mRick: Agents in {}:\x1b[0m", universe.name);
    println!();

    for a in &agents {
        println!("\x1b[97m  {}\x1b[0m", a.name);
        println!("\x1b[90m    {}\x1b[0m", a.soul_first_line);
        println!("\x1b[90m    Path: {}\x1b[0m", a.path.display());
        println!();
    }

    Ok(())
}

/// Execute the `list workflows` command.
pub fn list_workflows() -> Result<()> {
    let universe = resolver::resolve_universe_from_cwd()?;
    let workflows = workflow::load_workflows(&universe)?;

    println!("\x1b[36mRick: Workflows in {}:\x1b[0m", universe.name);
    println!();

    for wf in &workflows {
        println!("\x1b[97m  {}\x1b[0m", wf.name);
        println!("\x1b[90m    {}\x1b[0m", wf.description);
        if wf.has_composition() {
            println!("\x1b[90m    Phases: {}\x1b[0m", wf.steps.len());
            for (i, step) in wf.steps.iter().enumerate() {
                if step.is_phase() {
                    println!(
                        "\x1b[90m      {}. [phase: {}]\x1b[0m",
                        i + 1,
                        step.uses.as_deref().unwrap_or("?")
                    );
                } else {
                    println!(
                        "\x1b[90m      {}. [{}] {}\x1b[0m",
                        i + 1,
                        step.agent,
                        step.task
                    );
                }
            }
        } else {
            println!("\x1b[90m    Steps: {}\x1b[0m", wf.steps.len());
            for (i, step) in wf.steps.iter().enumerate() {
                println!(
                    "\x1b[90m      {}. {} - {}\x1b[0m",
                    i + 1,
                    step.agent,
                    step.task
                );
            }
        }
    }

    Ok(())
}

/// Execute the `list universes` command.
pub fn list_universes() -> Result<()> {
    let all = resolver::list_all_universes()?;

    if all.is_empty() {
        println!("\x1b[36mRick: No Universes installed.\x1b[0m");
        println!("\x1b[90m  Run 'rick add <url>' to install one.\x1b[0m");
        return Ok(());
    }

    println!("\x1b[36mRick: Installed Universes:\x1b[0m");
    println!();

    for (u, source) in &all {
        let agents = agent::load_agents(u).unwrap_or_default();
        let workflows = workflow::load_workflows(u).unwrap_or_default();

        println!(
            "\x1b[97m  {}\x1b[0m \x1b[90m({})\x1b[0m",
            u.name, source
        );
        println!(
            "\x1b[90m    v{} — {} agents, {} workflows\x1b[0m",
            u.version, agents.len(), workflows.len()
        );
        if !u.description.is_empty() {
            println!("\x1b[90m    {}\x1b[0m", u.description);
        }
        println!("\x1b[90m    Path: {}\x1b[0m", u.path.display());
        println!();
    }

    Ok(())
}

/// Execute the `runtimes` command — list discovered runtime backends.
pub fn runtimes() -> Result<()> {
    let registry = RuntimeRegistry::discover();
    let tools = registry.list_available_tools();

    let any_available = tools.iter().any(|(_, avail)| *avail);
    if !any_available {
        println!("\x1b[36mRick: No runtime tools found.\x1b[0m");
        println!("\x1b[90m  Install Claude Code (claude) or Cursor (agent) to get started.\x1b[0m");
        return Ok(());
    }

    println!("\x1b[36mRick: Available runtime tools:\x1b[0m");
    println!();

    for (tool, available) in &tools {
        let status_icon = if *available { "✓" } else { "✗" };
        let status_color = if *available { "\x1b[32m" } else { "\x1b[31m" };
        let cli_name = match *tool {
            "claude" => "claude (Claude Code CLI)",
            "cursor" => "agent (Cursor CLI)",
            _ => tool,
        };

        println!(
            "  \x1b[97m{:<10}\x1b[0m {:<30} {}{}\x1b[0m",
            tool, cli_name, status_color, status_icon
        );
    }

    println!();
    println!("\x1b[90m  Models are configured per-agent in tools.md (tool + model fields).\x1b[0m");
    println!("\x1b[90m  Rick passes model names through to --model. If the CLI rejects it, Rick reports the error.\x1b[0m");

    Ok(())
}

/// Execute the `compile` command. Optionally specify a universe name.
pub fn compile(universe_name: Option<&str>) -> Result<()> {
    let cwd = env::current_dir()?;
    let profile = Profile::load_or_default()?;
    let universe = match universe_name {
        Some(name) => resolver::resolve_universe(name)?,
        None => resolver::resolve_universe_from_cwd()?,
    };
    let agents = agent::load_agents(&universe)?;

    println!(
        "\x1b[36mRick: Compiling Universe \"{}\"...\x1b[0m",
        universe.name
    );

    let output_dir = cwd.join(".claude").join("agents");
    let mut compiled_paths = Vec::new();

    for a in &agents {
        let path = a.compile(&universe.name, &output_dir, &universe.path, &profile)?;
        compiled_paths.push(path);
    }

    println!("\x1b[32m  Compiled {} agents:\x1b[0m", compiled_paths.len());
    for path in &compiled_paths {
        // Show relative path from cwd
        let display = path
            .strip_prefix(&cwd)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());
        println!("\x1b[90m    {}\x1b[0m", display);
    }

    // Hint: suggest push if there are uncommitted agent/workflow changes
    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain", "agents/", "workflows/"])
        .current_dir(&cwd)
        .output();

    if let Ok(output) = status_output {
        let changes = String::from_utf8_lossy(&output.stdout);
        if !changes.trim().is_empty() {
            println!();
            println!("\x1b[90m  Tip: Run `rick push` to share these changes with your team.\x1b[0m");
        }
    }

    // Template hint (informational only)
    if let Ok(templates) = template::detect_templates(&universe.path) {
        if !templates.is_empty() {
            println!("\x1b[90m  Tip: This Universe has templates in .rick/templates/\x1b[0m");
        }
    }

    Ok(())
}

/// Execute the `run <workflow>` command.
pub fn run(workflow_name: &str, force: bool) -> Result<()> {
    let universe = resolver::resolve_universe_from_cwd()?;
    let workflows = workflow::load_workflows(&universe)?;

    let wf = workflows
        .iter()
        .find(|w| w.file_name == workflow_name)
        .ok_or_else(|| {
            RickError::NotFound(format!("Workflow '{}' not found", workflow_name))
        })?;

    // Dependency checking: collect agents used in this workflow
    let agents = agent::load_agents(&universe)?;
    let workflow_agent_names: Vec<&str> = wf.steps.iter().map(|s| s.agent.as_str()).collect();

    let agent_deps: Vec<(String, &agent::AgentDependencies)> = agents
        .iter()
        .filter(|a| workflow_agent_names.contains(&a.name.as_str()))
        .filter(|a| !a.dependencies.is_empty())
        .map(|a| (a.name.clone(), &a.dependencies))
        .collect();

    if !agent_deps.is_empty() {
        let report = deps::check_all(&agent_deps, &universe.path)?;
        if report.has_missing() {
            deps::print_report(&report);
            if !force {
                return Err(RickError::InvalidState(
                    "Missing dependencies. Use --force to override.".to_string(),
                ));
            }
            println!("\x1b[33mRick: --force specified, continuing despite missing deps...\x1b[0m");
            println!();
        }
    }

    let wf_id = WorkflowState::new_id();

    let (steps, phases, current_phase, total_phases) = if wf.has_composition() {
        // Composed workflow: create phases
        let phase_list: Vec<PhaseState> = wf
            .steps
            .iter()
            .map(|s| {
                if s.is_phase() {
                    PhaseState {
                        id: s.id.clone(),
                        uses: s.uses.clone(),
                        description: s.description.clone(),
                        status: "pending".to_string(),
                        current_step: 0,
                        total_steps: 0, // Skill populates children at execution time
                        steps: Vec::new(),
                    }
                } else {
                    // Direct step wrapped in a synthetic phase
                    PhaseState {
                        id: s.id.clone(),
                        uses: None,
                        description: s.description.clone(),
                        status: "pending".to_string(),
                        current_step: 0,
                        total_steps: 1,
                        steps: vec![StepState {
                            id: s.id.clone(),
                            agent: s.agent.clone(),
                            task: s.task.clone(),
                            status: "pending".to_string(),
                        }],
                    }
                }
            })
            .collect();
        let total = phase_list.len();
        (Vec::new(), Some(phase_list), Some(0), Some(total))
    } else {
        // Flat workflow: regular steps (with role gating)
        let profile = Profile::load_or_default()?;
        let step_list: Vec<StepState> = wf
            .steps
            .iter()
            .map(|s| {
                let status = if let Some(ref req) = s.requires_role {
                    if matches!(req, workflow::RequiredRole::Developer) && !profile.is_developer() {
                        "skipped".to_string()
                    } else {
                        "pending".to_string()
                    }
                } else {
                    "pending".to_string()
                };
                StepState {
                    id: s.id.clone(),
                    agent: s.agent.clone(),
                    task: s.task.clone(),
                    status,
                }
            })
            .collect();
        (step_list, None, None, None)
    };

    let state = WorkflowState {
        workflow_id: wf_id.clone(),
        workflow_name: wf.name.clone(),
        universe_name: universe.name.clone(),
        status: "started".to_string(),
        current_step: 0,
        total_steps: wf.steps.len(),
        steps,
        phases,
        current_phase,
        total_phases,
    };

    let state_dir = resolver::global_state_dir()?;
    state.save(&state_dir)?;

    println!(
        "\x1b[36mRick: Starting workflow \"{}\" in {}\x1b[0m",
        wf.name, universe.name
    );
    println!("\x1b[36m  Workflow ID: {}\x1b[0m", wf_id);
    println!();
    println!("\x1b[97m  Execution Plan:\x1b[0m");

    if wf.has_composition() {
        for (i, step) in wf.steps.iter().enumerate() {
            let marker = if i == 0 { "\x1b[36m->\x1b[0m" } else { "\x1b[90m  \x1b[0m" };
            if step.is_phase() {
                let desc = step.description.as_deref().unwrap_or("");
                println!(
                    "  {} Phase {}/{}: \x1b[97m{}\x1b[0m (uses: \x1b[36m{}\x1b[0m)",
                    marker,
                    i + 1,
                    wf.steps.len(),
                    step.id,
                    step.uses.as_deref().unwrap_or("?")
                );
                if !desc.is_empty() {
                    println!("  \x1b[90m     {}\x1b[0m", desc);
                }
            } else {
                println!(
                    "  {} {}. \x1b[97m{}\x1b[0m [{}] - {}",
                    marker,
                    i + 1,
                    step.id,
                    step.agent,
                    step.task
                );
            }
        }
        let first = &wf.steps[0];
        if first.is_phase() {
            println!();
            println!(
                "\x1b[36mRick: Ready to execute Phase 1: {} (uses: {})\x1b[0m",
                first.id,
                first.uses.as_deref().unwrap_or("?")
            );
        } else {
            println!();
            println!(
                "\x1b[36mRick: Ready to execute step 1: {}\x1b[0m",
                first.agent
            );
        }
    } else {
        let profile = Profile::load_or_default()?;
        for (i, step) in wf.steps.iter().enumerate() {
            let is_skipped = matches!(step.requires_role, Some(workflow::RequiredRole::Developer))
                && !profile.is_developer();
            if is_skipped {
                println!(
                    "  \x1b[90m  {}. {} - {} \x1b[33m(skipped: requires developer)\x1b[0m",
                    i + 1,
                    step.agent,
                    step.task
                );
            } else if i == 0 {
                println!(
                    "  \x1b[36m->\x1b[0m {}. {} - {}",
                    i + 1,
                    step.agent,
                    step.task
                );
            } else {
                println!(
                    "  \x1b[90m  \x1b[0m {}. {} - {}",
                    i + 1,
                    step.agent,
                    step.task
                );
            }
        }
        println!();
        // Find first non-skipped step for the "Ready" message
        let first_active = wf.steps.iter().enumerate().find(|(_, s)| {
            !matches!(s.requires_role, Some(workflow::RequiredRole::Developer))
                || profile.is_developer()
        });
        if let Some((idx, step)) = first_active {
            println!(
                "\x1b[36mRick: Ready to execute step {}: {}\x1b[0m",
                idx + 1,
                step.agent
            );
        } else {
            println!("\x1b[33mRick: All steps require developer role. Nothing to execute.\x1b[0m");
        }
    }

    println!();

    // A2A EXECUTION: Discover runtimes and execute workflow
    let registry = Arc::new(RuntimeRegistry::discover());
    let tools = registry.list_available_tools();

    let any_available = tools.iter().any(|(_, avail)| *avail);
    if !any_available {
        return Err(RickError::InvalidState(
            "No runtimes available. Install Claude Code ('claude') or Cursor ('agent').".to_string(),
        ));
    }

    println!("\x1b[36mRick: Available tools:\x1b[0m");
    for (tool, avail) in &tools {
        if *avail {
            println!("  \x1b[90m{} ✓\x1b[0m", tool);
        }
    }
    println!();

    // Load agents and build personas + runtime configs
    let agent_map: HashMap<String, agent::Agent> = agents
        .into_iter()
        .map(|a| (a.name.clone(), a))
        .collect();

    // Collect agent runtime configs for the scheduler
    let agent_configs: HashMap<String, agent::AgentRuntimeConfig> = agent_map
        .iter()
        .filter_map(|(name, ag)| {
            ag.runtime_config().map(|cfg| (name.clone(), cfg))
        })
        .collect();

    // Auto-linearize if no dependencies (backward compat)
    let mut steps = wf.steps.clone();
    linearize_steps(&mut steps);

    // Execute via scheduler
    println!("\x1b[36mRick: Executing workflow...\x1b[0m");
    println!();

    let scheduler = DagScheduler::new(steps.clone(), registry.clone(), agent_configs);

    // Clone data needed by closures
    let agent_map_clone = agent_map.clone();
    let wf_id_clone = wf_id.clone();

    let results = scheduler.execute_all(
        move |step, completed_results| {
            // Build TaskRequest for this step
            let agent = agent_map_clone.get(&step.agent).ok_or_else(|| {
                RickError::NotFound(format!("Agent '{}' not found", step.agent))
            })?;

            let persona = agent.compile_persona();

            // Build prior step summaries from completed results
            let prior_summaries: Vec<PriorStepSummary> = completed_results
                .iter()
                .filter_map(|r| {
                    if let Ok(ref resp) = r.response {
                        Some(PriorStepSummary {
                            step_id: r.step_id.clone(),
                            agent: step.agent.clone(),
                            role: persona.role.clone(),
                            entry: resp.output.entry.clone(),
                            exit: resp.output.exit.clone(),
                            summary: truncate(&resp.output.content, 200).to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // Inject personality template
            let description = crate::core::personality::inject_personality_template(
                &step.task,
                &prior_summaries,
            );

            Ok(TaskRequest {
                task_id: format!("{}-{}", wf_id_clone, step.id),
                session_id: wf_id_clone.clone(),
                description,
                context: TaskContext {
                    workflow_id: wf_id_clone.clone(),
                    step_id: step.id.clone(),
                    agent_persona: persona,
                    prior_steps: prior_summaries,
                },
                artifacts: vec![],
            })
        },
        {
            // Clone data the event handler needs for personality display
            let event_agent_map = agent_map.clone();
            let event_steps = steps.clone();

            move |event| {
                match event {
                    SchedulerEvent::StepStarted { step_id, runtime_id } => {
                        if let Some(step) = event_steps.iter().find(|s| s.id == *step_id) {
                            if let Some(ag) = event_agent_map.get(&step.agent) {
                                let handoff = crate::core::personality::generate_handoff(
                                    &ag.name,
                                    &ag.soul_first_line,
                                    &step.task,
                                );
                                println!(
                                    "\n\x1b[1mRick:\x1b[0m {} \x1b[90m[{}]\x1b[0m",
                                    handoff, runtime_id
                                );
                            }
                        }
                    }
                    SchedulerEvent::StepCompleted {
                        step_id, entry, content, exit, duration_ms, ..
                    } => {
                        if let Some(step) = event_steps.iter().find(|s| s.id == *step_id) {
                            if let Some(ag) = event_agent_map.get(&step.agent) {
                                if !entry.is_empty() {
                                    println!(
                                        "\x1b[1m{} ({}):\x1b[0m \x1b[3m{}\x1b[0m",
                                        ag.name, ag.soul_first_line, entry
                                    );
                                }
                                if !content.is_empty() {
                                    let display = if content.len() > 500 {
                                        let mut idx = 500;
                                        while !content.is_char_boundary(idx) && idx > 0 {
                                            idx -= 1;
                                        }
                                        format!("{}...", &content[..idx])
                                    } else {
                                        content.clone()
                                    };
                                    println!("\x1b[3m{}\x1b[0m", display);
                                }
                                if !exit.is_empty() {
                                    println!("\x1b[3m{}\x1b[0m", exit);
                                }
                                let recap = crate::core::personality::generate_recap(
                                    &ag.name,
                                    *duration_ms,
                                    None,
                                );
                                println!("\n\x1b[1mRick:\x1b[0m {}", recap);
                            }
                        }
                    }
                    SchedulerEvent::StepFailed { step_id, error, .. } => {
                        println!(
                            "\n\x1b[31mRick: Step '{}' failed: {}\x1b[0m",
                            step_id, error
                        );
                    }
                }
            }
        },
    )?;

    // Update state to completed
    let mut final_state = state;
    final_state.status = "completed".to_string();
    final_state.current_step = final_state.total_steps;
    final_state.save(&state_dir)?;

    println!("\n\x1b[32mRick: All {} steps complete.\x1b[0m", results.len());

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        let mut idx = max_len;
        while !s.is_char_boundary(idx) && idx > 0 {
            idx -= 1;
        }
        &s[..idx]
    }
}

/// Execute the `status` command.
pub fn status() -> Result<()> {
    let state_dir = resolver::global_state_dir()?;
    let states = state::load_all_states(&state_dir)?;

    if states.is_empty() {
        println!("\x1b[36mRick: No active workflows.\x1b[0m");
        return Ok(());
    }

    println!("\x1b[36mRick: Active Workflows:\x1b[0m");
    println!();

    for s in &states {
        println!(
            "\x1b[97m  {} \x1b[90m({})\x1b[0m",
            s.workflow_name, s.workflow_id
        );

        if let Some(ref phases) = s.phases {
            // Nested (composed) display
            let cp = s.current_phase.unwrap_or(0);
            let tp = s.total_phases.unwrap_or(phases.len());
            println!(
                "    Status: \x1b[34m{}\x1b[0m  Phase: {}/{}",
                s.status, cp, tp
            );
            for (i, phase) in phases.iter().enumerate() {
                let icon = match phase.status.as_str() {
                    "completed" => "\x1b[32m✓\x1b[0m",
                    "in_progress" | "running" => "\x1b[33m⏳\x1b[0m",
                    "failed" => "\x1b[31m✗\x1b[0m",
                    _ => "\x1b[90m⏸\x1b[0m",
                };
                if let Some(ref uses) = phase.uses {
                    println!(
                        "    {} Phase {}/{}: \x1b[97m{}\x1b[0m ({})",
                        icon,
                        i + 1,
                        tp,
                        phase.id,
                        uses
                    );
                } else {
                    println!(
                        "    {} {}/{}: \x1b[97m{}\x1b[0m [direct]",
                        icon,
                        i + 1,
                        tp,
                        phase.id
                    );
                }
                // Show child steps if phase has them
                for step in &phase.steps {
                    let s_icon = match step.status.as_str() {
                        "completed" => "\x1b[32m✓\x1b[0m",
                        "running" => "\x1b[33m▶\x1b[0m",
                        "failed" => "\x1b[31m✗\x1b[0m",
                        _ => "\x1b[90m·\x1b[0m",
                    };
                    println!(
                        "\x1b[90m      {} {} [{}]\x1b[0m",
                        s_icon, step.id, step.agent
                    );
                }
            }
        } else {
            // Flat display
            println!(
                "    Status: \x1b[34m{}\x1b[0m  Progress: {}/{}",
                s.status, s.current_step, s.total_steps
            );
            if s.current_step < s.steps.len() {
                let step = &s.steps[s.current_step];
                println!(
                    "\x1b[90m    Current: {} - {}\x1b[0m",
                    step.agent, step.task
                );
            }
        }
    }

    Ok(())
}

/// Execute the `init` command — create a new Universe in cwd (for authoring).
pub fn init() -> Result<()> {
    let cwd = env::current_dir()?;
    let rick_dir = cwd.join(".rick");

    if rick_dir.exists() {
        println!("\x1b[33mRick: Universe already initialized.\x1b[0m");
        return Ok(());
    }

    std::fs::create_dir_all(rick_dir.join("state"))?;
    std::fs::create_dir_all(rick_dir.join("prompts"))?;
    std::fs::create_dir_all(cwd.join("agents"))?;
    std::fs::create_dir_all(cwd.join("workflows"))?;

    let config = "name: my-universe\nversion: 0.1.0\ndescription: A Rick Universe\n";
    std::fs::write(rick_dir.join("config.yaml"), config)?;

    println!("\x1b[36mRick: Initialized new universe \"my-universe\"\x1b[0m");
    Ok(())
}

/// Execute the `add` / `install` command — clone an existing Universe repo into universes/.
pub fn add(url: &str, custom_name: Option<&str>) -> Result<()> {
    // Extract name from URL: git@github.com:user/my-universe.git -> my-universe
    let name = custom_name.map(|s| s.to_string()).unwrap_or_else(|| {
        let base = url.rsplit('/').next().unwrap_or("universe");
        base.trim_end_matches(".git").to_string()
    });

    let universes_dir = resolver::global_universes_dir()?;
    std::fs::create_dir_all(&universes_dir)?;
    let target = universes_dir.join(&name);

    if target.exists() {
        return Err(RickError::InvalidState(format!(
            "Universe '{}' already exists in ~/.rick/universes/",
            name
        )));
    }

    println!(
        "\x1b[36mRick: Adding universe '{}' from {}\x1b[0m",
        name, url
    );

    // Fetch/update ground rules on every add
    let home = env::var("HOME").unwrap_or_default();
    if !home.is_empty() {
        let gr_status = fetch_ground_rules(&home)?;
        if matches!(gr_status, WriteStatus::Created | WriteStatus::Updated) {
            println!("  {} Ground rules {}", gr_status.icon(), gr_status.message("~/.rick/ground-rules.md"));
        }
    }

    // Clone the repo into universes/
    let status = std::process::Command::new("git")
        .args(["clone", url, &target.to_string_lossy()])
        .status()
        .map_err(|e| RickError::Io(e))?;

    if !status.success() {
        return Err(RickError::InvalidState(format!(
            "git clone failed for {}",
            url
        )));
    }

    // Validate it's a Universe (has .rick/config.yaml or agents/)
    let has_config = target.join(".rick").join("config.yaml").exists();
    let has_agents = target.join("agents").exists();

    if !has_config && !has_agents {
        println!("\x1b[33m  Warning: Cloned repo doesn't look like a Rick Universe (no .rick/config.yaml or agents/)\x1b[0m");
        return Ok(());
    }

    // Load and display what we got
    let universe = Universe::load(&target)?;
    let agents = agent::load_agents(&universe)?;
    let workflows = workflow::load_workflows(&universe)?;

    println!(
        "\x1b[32m  Cloned '{}' — {} agents, {} workflows\x1b[0m",
        universe.name,
        agents.len(),
        workflows.len()
    );

    if !agents.is_empty() {
        let names: Vec<&str> = agents.iter().map(|a| a.name.as_str()).collect();
        println!("\x1b[90m  Agents: {}\x1b[0m", names.join(", "));
    }

    if !workflows.is_empty() {
        let names: Vec<&str> = workflows.iter().map(|w| w.name.as_str()).collect();
        println!("\x1b[90m  Workflows: {}\x1b[0m", names.join(", "));
    }

    // Auto-compile agents to project-local .claude/agents/
    println!();
    println!("\x1b[36mRick: Compiling agents...\x1b[0m");

    let profile = Profile::load_or_default()?;
    let cwd = env::current_dir()?;
    let output_dir = cwd.join(".claude").join("agents");
    let mut compiled_count = 0;
    for a in &agents {
        a.compile(&universe.name, &output_dir, &universe.path, &profile)?;
        compiled_count += 1;
    }

    println!(
        "\x1b[32m  Compiled {} agents to {}/\x1b[0m",
        compiled_count,
        output_dir.strip_prefix(&cwd).unwrap_or(&output_dir).display()
    );

    println!();
    println!("\x1b[36mRick: Universe '{}' is ready!\x1b[0m", name);
    println!("\x1b[97m  rick list workflows\x1b[0m");

    Ok(())
}

/// Execute the `pull` / `update` command — pull latest changes from remote and recompile.
pub fn pull(universe_name: Option<&str>) -> Result<()> {
    let cwd = env::current_dir()?;

    // If no name given, pull ALL installed universes
    let universes_to_pull: Vec<(Universe, String)> = if let Some(name) = universe_name {
        let u = resolver::resolve_universe(name)?;
        vec![(u, "global".to_string())]
    } else {
        let all = resolver::list_all_universes()?;
        if all.is_empty() {
            println!("\x1b[33mRick: No Universes installed. Run 'rick add <url>' first.\x1b[0m");
            return Ok(());
        }
        all
    };

    let multiple = universes_to_pull.len() > 1;
    let mut summary: Vec<(String, &str, String)> = Vec::new(); // (name, status, details)

    for (universe, _source) in &universes_to_pull {
        let uni_path = &universe.path;
        let uni_name = &universe.name;

        if multiple {
            println!("\x1b[36mRick: Pulling {}...\x1b[0m", uni_name);
        } else {
            println!("\x1b[36mRick: Pulling Universe '{}'...\x1b[0m", uni_name);
        }

        // Step 2: Pre-pull safety check
        let status_output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(uni_path)
            .output()
            .map_err(|e| RickError::Io(e))?;

        let changes = String::from_utf8_lossy(&status_output.stdout);
        if !changes.trim().is_empty() {
            println!("\x1b[33m  Warning: '{}' has uncommitted changes. Skipping.\x1b[0m", uni_name);
            println!("\x1b[90m  Resolve changes manually, then re-run 'rick pull {}'.\x1b[0m", uni_name);
            summary.push((uni_name.clone(), "Skipped", "Uncommitted changes".to_string()));
            continue;
        }

        // Step 3: Detect default branch
        let branch_output = std::process::Command::new("git")
            .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
            .current_dir(uni_path)
            .output();

        let default_branch = match branch_output {
            Ok(ref out) if out.status.success() => {
                let raw = String::from_utf8_lossy(&out.stdout);
                raw.trim().replace("refs/remotes/origin/", "")
            }
            _ => {
                // Fallback: try git remote show origin
                let show_output = std::process::Command::new("git")
                    .args(["remote", "show", "origin"])
                    .current_dir(uni_path)
                    .output();
                match show_output {
                    Ok(ref out) if out.status.success() => {
                        let text = String::from_utf8_lossy(&out.stdout);
                        text.lines()
                            .find(|l| l.contains("HEAD branch"))
                            .and_then(|l| l.split_whitespace().last())
                            .unwrap_or("main")
                            .to_string()
                    }
                    _ => "main".to_string(),
                }
            }
        };

        // Step 3b: Pull (non-developers use --ff-only to avoid merge commits)
        let pull_profile = Profile::load_or_default()?;
        let pull_args: Vec<&str> = if pull_profile.is_developer() {
            vec!["pull", "origin", &default_branch]
        } else {
            vec!["pull", "--ff-only", "origin", &default_branch]
        };
        let pull_output = std::process::Command::new("git")
            .args(&pull_args)
            .current_dir(uni_path)
            .output()
            .map_err(|e| RickError::Io(e))?;

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr);
            if stderr.contains("CONFLICT") || stderr.contains("Merge conflict") {
                println!("\x1b[31m  Merge conflict in '{}'. Resolve manually.\x1b[0m", uni_name);
                summary.push((uni_name.clone(), "Conflict", "Merge conflict".to_string()));
            } else {
                println!("\x1b[31m  Pull failed for '{}': {}\x1b[0m", uni_name, stderr.trim());
                summary.push((uni_name.clone(), "Failed", stderr.trim().to_string()));
            }
            continue;
        }

        let pull_msg = String::from_utf8_lossy(&pull_output.stdout);
        let up_to_date = pull_msg.contains("Already up to date") || pull_msg.contains("Already up-to-date");

        // Step 4: Post-pull — detect agent changes and recompile
        let agents_before: Vec<String> = std::fs::read_dir(cwd.join(".claude").join("agents"))
            .ok()
            .map(|entries| {
                entries.flatten()
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.starts_with(&format!("rick-{}-", uni_name)) && name.ends_with(".md") {
                            Some(name.trim_start_matches(&format!("rick-{}-", uni_name))
                                .trim_end_matches(".md").to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let agents_after: Vec<String> = agent::load_agents(&universe)
            .unwrap_or_default()
            .iter()
            .map(|a| a.name.clone())
            .collect();

        // Detect new and removed
        let new_agents: Vec<&String> = agents_after.iter()
            .filter(|a| !agents_before.contains(a))
            .collect();
        let removed_agents: Vec<&String> = agents_before.iter()
            .filter(|a| !agents_after.contains(a))
            .collect();

        // Delete stale compiled files for removed agents
        for removed in &removed_agents {
            let stale = cwd.join(".claude").join("agents")
                .join(format!("rick-{}-{}.md", uni_name, removed));
            let _ = std::fs::remove_file(&stale);
        }

        // Recompile all current agents
        let profile = Profile::load_or_default()?;
        let output_dir = cwd.join(".claude").join("agents");
        std::fs::create_dir_all(&output_dir)?;
        let agents = agent::load_agents(&universe).unwrap_or_default();
        let mut compiled_count = 0;
        for a in &agents {
            a.compile(&universe.name, &output_dir, &universe.path, &profile)?;
            compiled_count += 1;
        }

        // Build details string
        let mut details = Vec::new();
        details.push(format!("{} agents recompiled", compiled_count));
        if !new_agents.is_empty() {
            details.push(format!("{} new ({})", new_agents.len(),
                new_agents.iter().map(|a| a.as_str()).collect::<Vec<_>>().join(", ")));
        }
        if !removed_agents.is_empty() {
            details.push(format!("{} removed ({})", removed_agents.len(),
                removed_agents.iter().map(|a| a.as_str()).collect::<Vec<_>>().join(", ")));
        }

        let status_label = if up_to_date { "Up to date" } else { "Updated" };

        if !multiple {
            // Single universe — detailed output
            println!("  \x1b[32m✓\x1b[0m Status: {}", status_label);
            println!("  \x1b[32m✓\x1b[0m {}", details.join(", "));
            if !new_agents.is_empty() {
                for a in &new_agents {
                    println!("  \x1b[36m+\x1b[0m New agent: \x1b[97m{}\x1b[0m", a);
                }
            }
            if !removed_agents.is_empty() {
                for a in &removed_agents {
                    println!("  \x1b[31m-\x1b[0m Removed agent: \x1b[97m{}\x1b[0m", a);
                }
            }
        }

        summary.push((uni_name.clone(), status_label, details.join(", ")));
    }

    // Multi-universe summary table
    if multiple {
        println!();
        println!("\x1b[36mRick: Pull Summary:\x1b[0m");
        println!();
        println!("  \x1b[97m{:<24} {:<14} {}\x1b[0m", "Universe", "Status", "Changes");
        println!("  {}", "-".repeat(70));
        for (name, status, details) in &summary {
            let icon = match *status {
                "Updated" => "\x1b[32m✓\x1b[0m",
                "Up to date" => "\x1b[32m✓\x1b[0m",
                "Skipped" => "\x1b[33m!\x1b[0m",
                _ => "\x1b[31m✗\x1b[0m",
            };
            println!("  {} {:<24} {:<14} {}", icon, name, status, details);
        }
    }

    Ok(())
}

/// Execute the `next` command.
pub fn next() -> Result<()> {
    let state_dir = resolver::global_state_dir()?;
    let states = state::load_all_states(&state_dir)?;

    if states.is_empty() {
        println!("\x1b[33mRick: No active workflows to continue.\x1b[0m");
        return Ok(());
    }

    // Pick the most recent (last) active workflow
    let current = &states[states.len() - 1];

    if current.current_step + 1 >= current.total_steps {
        println!("\x1b[36mRick: Workflow \"{}\" is complete!\x1b[0m", current.workflow_name);
        return Ok(());
    }

    // Find the next non-skipped step
    let mut next_step = current.current_step + 1;
    while next_step < current.steps.len() {
        if current.steps[next_step].status == "skipped" {
            println!(
                "\x1b[90m  Skipping step {}: {} (requires developer)\x1b[0m",
                next_step + 1,
                current.steps[next_step].agent
            );
            next_step += 1;
        } else {
            break;
        }
    }

    if next_step >= current.total_steps {
        println!("\x1b[36mRick: Workflow \"{}\" is complete! (remaining steps were skipped)\x1b[0m", current.workflow_name);
        return Ok(());
    }

    let step = &current.steps[next_step];

    println!(
        "\x1b[36mRick: Advancing to step {}: {}\x1b[0m",
        next_step + 1,
        step.agent
    );
    println!(
        "\x1b[90m  Task: {}\x1b[0m",
        step.task
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Profile command
// ---------------------------------------------------------------------------

/// Execute the `profile` command — view or change user role.
pub fn profile(args: &[&str]) -> Result<()> {
    match args.first().copied() {
        None | Some("show") => {
            let prof = Profile::load_or_default()?;
            println!("\x1b[36mRick: Current Profile:\x1b[0m");
            println!();
            println!("  Role:     \x1b[97m{}\x1b[0m", prof.role_display());
            if let Some(sr) = prof.sub_role_display() {
                println!("  Sub-role: \x1b[97m{}\x1b[0m", sr);
            }
            println!();
            println!("\x1b[90m  Change with: rick profile set <developer|non-developer> [sub-role]\x1b[0m");
        }
        Some("set") => {
            if args.len() < 2 {
                return Err(RickError::InvalidState(
                    "Missing role. Use 'rick profile set developer' or 'rick profile set non-developer'.".to_string(),
                ));
            }
            let role_str = args[1];
            let sub_role_str = args.get(2).copied();

            let role = match role_str {
                "developer" | "dev" => crate::core::profile::Role::Developer,
                "non-developer" | "nondev" => crate::core::profile::Role::NonDeveloper,
                other => {
                    return Err(RickError::InvalidState(format!(
                        "Unknown role '{}'. Use 'developer' or 'non-developer'.", other
                    )));
                }
            };

            let sub_role = match sub_role_str {
                Some("pm") => Some(crate::core::profile::SubRole::PM),
                Some("designer") => Some(crate::core::profile::SubRole::Designer),
                Some("qa") => Some(crate::core::profile::SubRole::QA),
                Some("other") => Some(crate::core::profile::SubRole::Other),
                Some(unknown) => {
                    return Err(RickError::InvalidState(format!(
                        "Unknown sub-role '{}'. Use 'pm', 'designer', 'qa', or 'other'.", unknown
                    )));
                }
                None => None,
            };

            let prof = Profile { role, sub_role };
            let path = Profile::path()?;
            prof.save(&path)?;

            println!("\x1b[32mRick: Profile updated to '{}'.\x1b[0m", prof.role_display());

            // Auto-recompile all installed universes
            let all = resolver::list_all_universes()?;
            if all.is_empty() {
                println!("\x1b[90m  Run 'rick compile' after installing a Universe to apply constraints.\x1b[0m");
            } else {
                let cwd = env::current_dir()?;
                let output_dir = cwd.join(".claude").join("agents");
                let mut total = 0;
                for (universe, _source) in &all {
                    let agents = agent::load_agents(universe).unwrap_or_default();
                    for a in &agents {
                        let _ = a.compile(&universe.name, &output_dir, &universe.path, &prof);
                        total += 1;
                    }
                }
                if total > 0 {
                    println!("\x1b[32m  Recompiled {} agents with new role constraints.\x1b[0m", total);
                }
            }
        }
        Some(other) => {
            // Treat as shorthand: `rick profile developer` = `rick profile set developer`
            let mut new_args = vec!["set", other];
            if args.len() > 1 {
                new_args.extend_from_slice(&args[1..]);
            }
            return profile(&new_args.iter().map(|s| *s).collect::<Vec<&str>>());
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Setup command
// ---------------------------------------------------------------------------

/// Execute the `setup` command — full onboarding: skill, persona, permissions, universe, deps.
pub fn setup(universe_url: Option<&str>, install_deps: bool, non_interactive: bool) -> Result<()> {
    let home = env::var("HOME").map_err(|_| {
        RickError::InvalidState("HOME environment variable not set".to_string())
    })?;

    println!("\x1b[36mRick: Running setup...\x1b[0m");
    println!();

    // Step 0: Self-update — check for newer binary
    let _updated = self_update()?;

    // Step 1: Install Skill
    let skill_status = install_skill(&home)?;
    println!("  {} Skill        {}", skill_status.icon(), skill_status.message("~/.claude/skills/rick/ + Rick/"));

    // Step 2: Create Persona (never overwrite — user customizations are sacred)
    let soul_status = write_if_new(
        &format!("{}/.rick/persona/soul.md", home),
        DEFAULT_SOUL,
    )?;
    println!("  {} Persona soul {}", soul_status.icon(), soul_status.message("~/.rick/persona/soul.md"));

    let rules_status = write_if_new(
        &format!("{}/.rick/persona/rules.md", home),
        DEFAULT_RULES,
    )?;
    println!("  {} Persona rules {}", rules_status.icon(), rules_status.message("~/.rick/persona/rules.md"));

    let memory_status = write_if_new(
        &format!("{}/.rick/persona/Memory.md", home),
        DEFAULT_MEMORY,
    )?;
    println!("  {} Memory       {}", memory_status.icon(), memory_status.message("~/.rick/persona/Memory.md"));

    // Step 2b: User profile (role selection)
    let profile_status = setup_profile(&home, non_interactive)?;
    println!("  {} Profile      {}", profile_status.icon(), profile_status.message("~/.rick/profile.yaml"));

    // Step 2c: Fetch ground rules from GitHub
    let gr_status = fetch_ground_rules(&home)?;
    println!("  {} Ground rules {}", gr_status.icon(), gr_status.message("~/.rick/ground-rules.md"));

    // Step 3: Permissions guidance
    println!();
    show_permissions_guidance(non_interactive)?;

    // Step 4: Clone Universe (optional)
    if let Some(url) = universe_url {
        println!();
        add(url, None)?;
    }

    // Step 5: Install agent MCP dependencies (optional)
    if install_deps {
        println!();
        install_agent_deps()?;
    }

    // Summary
    println!();
    println!("\x1b[32mRick: Setup complete!\x1b[0m");
    println!();
    println!("\x1b[97m  Get started:\x1b[0m");
    println!("    /Rick list agents       See available agents");
    println!("    /Rick list workflows    See available workflows");
    println!("    /Rick run <workflow>    Start a workflow");

    Ok(())
}

/// Status of a file write operation (for idempotent messaging).
enum WriteStatus {
    Created,
    Updated,
    Unchanged,
}

impl WriteStatus {
    fn icon(&self) -> &str {
        match self {
            WriteStatus::Created => "\x1b[32m✓\x1b[0m",
            WriteStatus::Updated => "\x1b[33m↻\x1b[0m",
            WriteStatus::Unchanged => "\x1b[32m✓\x1b[0m",
        }
    }

    fn message(&self, path: &str) -> String {
        match self {
            WriteStatus::Created => format!("{} \x1b[90m(created)\x1b[0m", path),
            WriteStatus::Updated => format!("{} \x1b[33m(updated to latest)\x1b[0m", path),
            WriteStatus::Unchanged => format!("{} \x1b[90m(already up to date)\x1b[0m", path),
        }
    }
}

/// Write a file only if it doesn't exist or content differs.
fn write_if_needed(path: &str, content: &str) -> Result<WriteStatus> {
    let p = std::path::Path::new(path);

    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if p.exists() {
        let existing = std::fs::read_to_string(p)?;
        if existing == content {
            return Ok(WriteStatus::Unchanged);
        }
        std::fs::write(p, content)?;
        return Ok(WriteStatus::Updated);
    }

    std::fs::write(p, content)?;
    Ok(WriteStatus::Created)
}

/// Set up the user profile during `rick setup`. Never overwrites existing profile.
fn setup_profile(home: &str, non_interactive: bool) -> Result<WriteStatus> {
    let profile_path = format!("{}/.rick/profile.yaml", home);
    let p = std::path::Path::new(&profile_path);

    if p.exists() {
        return Ok(WriteStatus::Unchanged);
    }

    if non_interactive {
        // Default to developer in non-interactive mode (CI, piped install)
        let profile = Profile {
            role: crate::core::profile::Role::Developer,
            sub_role: None,
        };
        profile.save(p)?;
        return Ok(WriteStatus::Created);
    }

    println!();
    println!("    \x1b[97mWhat's your role?\x1b[0m");
    println!("      [1] Developer — I write and commit code");
    println!("      [2] Non-developer — I review, manage, design (read-only git)");
    println!();

    let choice = read_user_choice("    Choose [1/2]: ", "1");

    let (role, sub_role) = match choice.as_str() {
        "2" => {
            println!();
            println!("    \x1b[97mWhat kind of non-developer?\x1b[0m");
            println!("      [1] PM / Product Manager");
            println!("      [2] Designer");
            println!("      [3] QA / Tester");
            println!("      [4] Other");
            println!();
            let sub = read_user_choice("    Choose [1/2/3/4]: ", "1");
            let sub_role = match sub.as_str() {
                "2" => crate::core::profile::SubRole::Designer,
                "3" => crate::core::profile::SubRole::QA,
                "4" => crate::core::profile::SubRole::Other,
                _ => crate::core::profile::SubRole::PM,
            };
            (crate::core::profile::Role::NonDeveloper, Some(sub_role))
        }
        _ => (crate::core::profile::Role::Developer, None),
    };

    let profile = Profile { role, sub_role };
    profile.save(p)?;
    Ok(WriteStatus::Created)
}

/// Write a file only if it doesn't exist yet. Never overwrite (persona files are user-owned).
fn write_if_new(path: &str, content: &str) -> Result<WriteStatus> {
    let p = std::path::Path::new(path);

    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if p.exists() {
        return Ok(WriteStatus::Unchanged);
    }

    std::fs::write(p, content)?;
    Ok(WriteStatus::Created)
}

/// Install the Rick skill to ~/.claude/skills/rick/SKILL.md and ~/.claude/skills/Rick/SKILL.md.
/// Also installs references/ folder for progressive disclosure.
fn install_skill(home: &str) -> Result<WriteStatus> {
    let lowercase_path = format!("{}/.claude/skills/rick/SKILL.md", home);
    let uppercase_path = format!("{}/.claude/skills/Rick/SKILL.md", home);
    let status = write_if_needed(&lowercase_path, SKILL_CONTENT)?;
    write_if_needed(&uppercase_path, SKILL_CONTENT)?;

    // Install references/ for progressive disclosure
    let refs = [
        ("dispatch-protocol.md", REF_DISPATCH),
        ("memory-protocol.md", REF_MEMORY),
        ("background-advisor.md", REF_ADVISOR),
        ("templates-protocol.md", REF_TEMPLATES),
        ("examples.md", REF_EXAMPLES),
    ];
    for variant in &["rick", "Rick"] {
        let refs_dir = format!("{}/.claude/skills/{}/references", home, variant);
        std::fs::create_dir_all(&refs_dir)?;
        for (name, content) in &refs {
            write_if_needed(&format!("{}/{}", refs_dir, name), content)?;
        }
    }

    Ok(status)
}

/// Show permissions guidance — detect missing perms and offer options.
fn show_permissions_guidance(non_interactive: bool) -> Result<()> {
    let home = env::var("HOME").unwrap_or_default();
    let rick_perms = vec![
        "Bash(rick *)",
        "Bash(cd * && rick *)",
    ];

    // Check user-level settings first, then project-level
    let user_settings_path = format!("{}/.claude/settings.json", home);
    let cwd = env::current_dir()?;
    let project_settings_path = cwd.join(".claude").join("settings.json");

    let user_existing = load_allow_permissions(&user_settings_path);
    let project_existing = load_allow_permissions(&project_settings_path.to_string_lossy());

    let all_existing: Vec<String> = user_existing.iter().chain(project_existing.iter()).cloned().collect();
    let missing: Vec<&&str> = rick_perms.iter().filter(|p| !all_existing.contains(&p.to_string())).collect();

    if missing.is_empty() {
        println!("  \x1b[32m✓\x1b[0m Permissions   Already configured");
        return Ok(());
    }

    println!("  \x1b[33m!\x1b[0m Permissions   Claude Code permissions needed for smooth operation:");
    println!();
    for p in &missing {
        println!("      \x1b[97m{}\x1b[0m  — Allows Rick CLI commands without approval prompts", p);
    }

    // Non-interactive mode: show JSON and move on
    if non_interactive {
        println!();
        println!("    \x1b[33mNon-interactive mode — showing JSON for manual setup:\x1b[0m");
        println!();
        println!("    Add to \x1b[97m~/.claude/settings.json\x1b[0m:");
        println!();
        println!("    \x1b[36m{{\x1b[0m");
        println!("    \x1b[36m  \"permissions\": {{\x1b[0m");
        println!("    \x1b[36m    \"allow\": [\x1b[0m");
        for (i, p) in rick_perms.iter().enumerate() {
            let comma = if i < rick_perms.len() - 1 { "," } else { "" };
            println!("    \x1b[36m      \"{}\"{}\x1b[0m", p, comma);
        }
        println!("    \x1b[36m    ]\x1b[0m");
        println!("    \x1b[36m  }}\x1b[0m");
        println!("    \x1b[36m}}\x1b[0m");
        return Ok(());
    }

    println!();
    println!("    \x1b[97mOptions:\x1b[0m");
    println!("      [1] Add to user settings (~/.claude/settings.json) — works in all projects");
    println!("      [2] Add to project settings (.claude/settings.json) — this project only");
    println!("      [3] Show me the JSON so I can add it myself");
    println!("      [4] Skip — I'll approve commands manually each time \x1b[33m(not recommended)\x1b[0m");
    println!();

    let choice = read_user_choice("    Choose [1/2/3/4]: ", "1");

    match choice.as_str() {
        "1" => {
            write_project_permissions(&user_settings_path, &rick_perms)?;
            println!("    \x1b[32m✓\x1b[0m Permissions added to ~/.claude/settings.json");
        }
        "2" => {
            write_project_permissions(&project_settings_path.to_string_lossy(), &rick_perms)?;
            println!("    \x1b[32m✓\x1b[0m Permissions added to .claude/settings.json");
        }
        "3" => {
            println!();
            println!("    Add to \x1b[97m~/.claude/settings.json\x1b[0m (user-level, all projects):");
            println!();
            println!("    \x1b[36m{{\x1b[0m");
            println!("    \x1b[36m  \"permissions\": {{\x1b[0m");
            println!("    \x1b[36m    \"allow\": [\x1b[0m");
            for (i, p) in rick_perms.iter().enumerate() {
                let comma = if i < rick_perms.len() - 1 { "," } else { "" };
                println!("    \x1b[36m      \"{}\"{}\x1b[0m", p, comma);
            }
            println!("    \x1b[36m    ]\x1b[0m");
            println!("    \x1b[36m  }}\x1b[0m");
            println!("    \x1b[36m}}\x1b[0m");
        }
        _ => {
            println!("    \x1b[33mSkipped.\x1b[0m You'll be prompted to approve each Rick command.");
        }
    }

    Ok(())
}

/// Read a single choice from the user via /dev/tty (bypasses piped stdin).
/// Falls back to `default` if no terminal is available.
fn read_user_choice(prompt: &str, default: &str) -> String {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    print!("{}", prompt);
    let _ = io::stdout().flush();

    // Open /dev/tty directly — works even when stdin is piped (curl | bash).
    // This opens a fresh handle, avoiding any buffered leftovers from sudo etc.
    let tty = match File::open("/dev/tty") {
        Ok(f) => f,
        Err(_) => return default.to_string(), // No terminal (Docker, CI)
    };

    let mut reader = BufReader::new(tty);
    let mut input = String::new();
    match reader.read_line(&mut input) {
        Ok(0) => default.to_string(),
        Ok(_) => {
            let trimmed = input.trim();
            if trimmed.is_empty() {
                default.to_string()
            } else {
                trimmed.to_string()
            }
        }
        Err(_) => default.to_string(),
    }
}

/// Load the "allow" array from a settings.json file.
fn load_allow_permissions(path: &str) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let parsed = match json::parse_json(&content) {
        Ok(val) => val,
        Err(_) => return Vec::new(),
    };

    let permissions = match parsed.get("permissions") {
        Some(val) => val,
        None => return Vec::new(),
    };

    let allow = match permissions.get("allow") {
        Some(JsonValue::Array(items)) => items,
        _ => return Vec::new(),
    };

    allow
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect()
}

/// Write Rick permissions to a project-level .claude/settings.json, merging with existing.
fn write_project_permissions(path: &str, perms: &[&str]) -> Result<()> {
    let p = std::path::Path::new(path);

    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Load existing or create empty structure
    let mut entries: Vec<(String, JsonValue)> = if p.exists() {
        let content = std::fs::read_to_string(p)?;
        match json::parse_json(&content)? {
            JsonValue::Object(e) => e,
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };

    // Get or create permissions.allow
    let mut allow_items: Vec<JsonValue> = Vec::new();

    // Extract existing allow items
    if let Some(perm_idx) = entries.iter().position(|(k, _)| k == "permissions") {
        if let JsonValue::Object(ref perm_entries) = entries[perm_idx].1 {
            if let Some((_, JsonValue::Array(ref items))) = perm_entries.iter().find(|(k, _)| k == "allow") {
                allow_items = items.clone();
            }
        }
    }

    // Add missing permissions
    for perm in perms {
        let already = allow_items.iter().any(|v| {
            matches!(v, JsonValue::String(s) if s == perm)
        });
        if !already {
            allow_items.push(JsonValue::String(perm.to_string()));
        }
    }

    // Rebuild permissions object, preserving other keys
    let mut perm_entries: Vec<(String, JsonValue)> = Vec::new();

    // Preserve existing permission keys (deny, ask, etc.)
    if let Some(perm_idx) = entries.iter().position(|(k, _)| k == "permissions") {
        if let JsonValue::Object(ref existing_perm) = entries[perm_idx].1 {
            for (k, v) in existing_perm {
                if k != "allow" {
                    perm_entries.push((k.clone(), v.clone()));
                }
            }
        }
    }

    // Insert allow at the front
    perm_entries.insert(0, ("allow".to_string(), JsonValue::Array(allow_items)));

    // Replace or add permissions in root
    let perm_val = JsonValue::Object(perm_entries);
    if let Some(idx) = entries.iter().position(|(k, _)| k == "permissions") {
        entries[idx].1 = perm_val;
    } else {
        entries.push(("permissions".to_string(), perm_val));
    }

    let root = JsonValue::Object(entries);
    let output = json::to_json_pretty(&root, 0);
    std::fs::write(p, format!("{}\n", output))?;

    Ok(())
}

/// Fetch ground rules from the Rick-POC main branch and store at ~/.rick/ground-rules.md.
fn fetch_ground_rules(home: &str) -> Result<WriteStatus> {
    let target_path = format!("{}/.rick/ground-rules.md", home);
    let p = std::path::Path::new(&target_path);

    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Fetch via curl
    let output = std::process::Command::new("curl")
        .args(["-fsSL", GROUND_RULES_URL])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let content = String::from_utf8_lossy(&out.stdout).to_string();
            if content.trim().is_empty() || !content.contains("# Rick Ground Rules") {
                // Bad response — skip silently
                if p.exists() {
                    return Ok(WriteStatus::Unchanged);
                }
                return Ok(WriteStatus::Unchanged);
            }
            // Check if content changed
            if p.exists() {
                let existing = std::fs::read_to_string(p)?;
                if existing == content {
                    return Ok(WriteStatus::Unchanged);
                }
                std::fs::write(p, &content)?;
                return Ok(WriteStatus::Updated);
            }
            std::fs::write(p, &content)?;
            Ok(WriteStatus::Created)
        }
        _ => {
            // Network failure — not fatal, just warn
            if p.exists() {
                Ok(WriteStatus::Unchanged)
            } else {
                println!("  \x1b[33m!\x1b[0m Ground rules  Could not fetch (no network?) — skipped");
                Ok(WriteStatus::Unchanged)
            }
        }
    }
}

/// Self-update: check GitHub releases for a newer version and replace the current binary.
fn self_update() -> Result<bool> {
    let current_version = env!("CARGO_PKG_VERSION");

    // Fetch latest release tag from GitHub API
    let output = std::process::Command::new("curl")
        .args(["-fsSL", "-H", "Accept: application/vnd.github.v3+json", LATEST_RELEASE_URL])
        .output();

    let tag = match output {
        Ok(out) if out.status.success() => {
            let body = String::from_utf8_lossy(&out.stdout);
            // Simple extraction: find "tag_name":"vX.Y.Z"
            extract_json_string(&body, "tag_name").unwrap_or_default()
        }
        _ => return Ok(false),
    };

    if tag.is_empty() {
        return Ok(false);
    }

    let remote_version = tag.trim_start_matches('v');
    if remote_version == current_version {
        return Ok(false);
    }

    // Detect platform
    let os = if cfg!(target_os = "macos") { "darwin" } else { "linux" };
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" };

    let download_url = format!(
        "https://github.com/Sagi363/Rick-POC/releases/download/{}/rick-{}-{}",
        tag, os, arch
    );

    println!(
        "  \x1b[33m↻\x1b[0m Update       v{} -> v{} available",
        current_version, remote_version
    );

    // Download to temp file
    let tmp_path = "/tmp/rick-update-bin";
    let dl_status = std::process::Command::new("curl")
        .args(["-fsSL", "-o", tmp_path, &download_url])
        .status();

    match dl_status {
        Ok(s) if s.success() => {}
        _ => {
            println!("  \x1b[33m!\x1b[0m Update       Download failed — skipping");
            return Ok(false);
        }
    }

    // Make executable
    let _ = std::process::Command::new("chmod")
        .args(["+x", tmp_path])
        .status();

    // Verify it runs
    let verify = std::process::Command::new(tmp_path)
        .args(["--version"])
        .output();

    match verify {
        Ok(out) if out.status.success() => {}
        _ => {
            println!("  \x1b[33m!\x1b[0m Update       Downloaded binary invalid — skipping");
            let _ = std::fs::remove_file(tmp_path);
            return Ok(false);
        }
    }

    // Find where the current binary is installed
    let current_exe = env::current_exe().map_err(|e| RickError::Io(e))?;
    let install_path = current_exe.to_string_lossy().to_string();

    // Try to replace — may need sudo
    let cp_status = std::process::Command::new("cp")
        .args([tmp_path, &install_path])
        .status();

    match cp_status {
        Ok(s) if s.success() => {
            println!(
                "  \x1b[32m✓\x1b[0m Update       Updated to v{} \x1b[90m({})\x1b[0m",
                remote_version, install_path
            );
            let _ = std::fs::remove_file(tmp_path);
            Ok(true)
        }
        _ => {
            // Try with sudo
            let sudo_status = std::process::Command::new("sudo")
                .args(["cp", tmp_path, &install_path])
                .status();

            match sudo_status {
                Ok(s) if s.success() => {
                    println!(
                        "  \x1b[32m✓\x1b[0m Update       Updated to v{} \x1b[90m({})\x1b[0m",
                        remote_version, install_path
                    );
                    let _ = std::fs::remove_file(tmp_path);
                    Ok(true)
                }
                _ => {
                    println!("  \x1b[33m!\x1b[0m Update       Could not replace binary at {} — update manually", install_path);
                    println!("               \x1b[90msudo cp {} {}\x1b[0m", tmp_path, install_path);
                    Ok(false)
                }
            }
        }
    }
}

/// Extract a string value from a JSON body by key (simple, no full parser needed).
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let start = json.find(&pattern)?;
    let after_key = &json[start + pattern.len()..];
    // Skip whitespace and colon
    let after_colon = after_key.find(':').map(|i| &after_key[i + 1..])?;
    let quote_start = after_colon.find('"')?;
    let value_start = &after_colon[quote_start + 1..];
    let quote_end = value_start.find('"')?;
    Some(value_start[..quote_end].to_string())
}

/// Install MCP dependencies declared by agents in the current Universe.
fn install_agent_deps() -> Result<()> {
    let universe = resolver::resolve_universe_from_cwd()?;
    let agents = agent::load_agents(&universe)?;

    let agent_deps: Vec<(String, &agent::AgentDependencies)> = agents
        .iter()
        .filter(|a| !a.dependencies.is_empty())
        .map(|a| (a.name.clone(), &a.dependencies))
        .collect();

    if agent_deps.is_empty() {
        println!("\x1b[32mRick: No MCP dependencies to install.\x1b[0m");
        return Ok(());
    }

    let report = deps::check_all(&agent_deps, &universe.path)?;
    let missing = report.missing();

    if missing.is_empty() {
        println!("\x1b[32mRick: All MCP dependencies already installed.\x1b[0m");
        return Ok(());
    }

    println!("\x1b[36mRick: Installing {} missing MCP dependencies...\x1b[0m", missing.len());
    println!();

    for dep in &missing {
        if dep.dep_type != deps::DepType::Mcp || dep.install.is_empty() {
            continue;
        }

        println!("  Installing MCP: \x1b[97m{}\x1b[0m", dep.name);
        let status = std::process::Command::new("sh")
            .args(["-c", &dep.install])
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("    \x1b[32m✓\x1b[0m Installed");
            }
            _ => {
                println!("    \x1b[31m✗\x1b[0m Failed. Run manually: \x1b[36m{}\x1b[0m", dep.install);
            }
        }
    }

    Ok(())
}

/// Execute the `invite` command — generate a shareable install command for the current Universe.
pub fn invite(emails: &[&str]) -> Result<()> {
    let universe = resolver::resolve_universe_from_cwd()?;
    let agents = agent::load_agents(&universe)?;
    let workflows = workflow::load_workflows(&universe)?;

    if universe.repository.is_empty() {
        return Err(RickError::InvalidState(
            "No 'repository' field in .rick/config.yaml — add it to enable invite links.".to_string(),
        ));
    }

    let repo_url = &universe.repository;

    // Extract GitHub owner/repo from repository URL (SSH or HTTPS)
    let gh_repo = extract_gh_repo(repo_url);

    // If emails provided, add them as collaborators
    if !emails.is_empty() {
        if let Some(ref repo) = gh_repo {
            // Check if gh CLI is available
            let gh_check = std::process::Command::new("gh")
                .args(["auth", "status"])
                .output();

            match gh_check {
                Ok(output) if output.status.success() => {
                    for email in emails {
                        println!("\x1b[36mRick: Inviting {} to {}...\x1b[0m", email, repo);
                        let result = std::process::Command::new("gh")
                            .args(["api", &format!("repos/{}/collaborators/{}", repo, email),
                                   "--method", "PUT",
                                   "--field", "permission=push"])
                            .output();

                        match result {
                            Ok(out) if out.status.success() => {
                                println!("  \x1b[32m✓ Invitation sent to {}\x1b[0m", email);
                            }
                            Ok(out) => {
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                let stdout = String::from_utf8_lossy(&out.stdout);
                                if stderr.contains("403") || stdout.contains("403")
                                    || stderr.contains("Must have admin access")
                                    || stdout.contains("Must have admin access")
                                {
                                    println!("  \x1b[31m✗ Permission denied for {}. You need admin access to {} to add collaborators.\x1b[0m", email, repo);
                                } else if stderr.contains("404") || stdout.contains("404") {
                                    println!("  \x1b[31m✗ GitHub user '{}' not found. Use their GitHub username, not email.\x1b[0m", email);
                                } else {
                                    println!("  \x1b[31m✗ Failed to invite {}: {}{}\x1b[0m", email, stderr, stdout);
                                }
                            }
                            Err(e) => {
                                println!("  \x1b[31m✗ Failed to run gh: {}\x1b[0m", e);
                            }
                        }
                    }
                    println!();
                }
                _ => {
                    println!("\x1b[31mRick: 'gh' CLI not authenticated. Run 'gh auth login' first to invite collaborators.\x1b[0m");
                    println!("\x1b[90m  Showing install links instead.\x1b[0m");
                    println!();
                }
            }
        } else {
            println!("\x1b[31mRick: Can't parse GitHub repo from '{}'. Invite collaborators manually.\x1b[0m", repo_url);
            println!();
        }
    }

    // Always show install links
    let install_url = "https://raw.githubusercontent.com/Sagi363/Rick-POC/main/install.sh";

    println!(
        "\x1b[36mRick: Share this to invite someone to the {} Universe:\x1b[0m",
        universe.name
    );
    println!();
    println!("\x1b[97m  One-line install (Rick + Universe):\x1b[0m");
    println!();
    println!(
        "    \x1b[32mcurl -fsSL {} | bash -s -- -u {}\x1b[0m",
        install_url, repo_url
    );
    println!();
    println!("\x1b[97m  If Rick is already installed:\x1b[0m");
    println!();
    println!("    \x1b[32mrick add {}\x1b[0m", repo_url);
    println!();
    println!("\x1b[90m  Universe: {} v{} — {} agents, {} workflows\x1b[0m",
        universe.name, universe.version, agents.len(), workflows.len()
    );
    println!();

    Ok(())
}

/// Extract "owner/repo" from a GitHub URL (SSH or HTTPS).
fn extract_gh_repo(url: &str) -> Option<String> {
    // git@github.com:Owner/Repo.git or git@github.com-alias:Owner/Repo.git
    if url.contains("github.com") && url.contains(':') && url.starts_with("git@") {
        let after_colon = url.split(':').last()?;
        let repo = after_colon.trim_end_matches(".git");
        return Some(repo.to_string());
    }
    // https://github.com/Owner/Repo.git
    if url.contains("github.com/") {
        let after_gh = url.split("github.com/").last()?;
        let repo = after_gh.trim_end_matches(".git");
        return Some(repo.to_string());
    }
    None
}

/// Execute the `push` command — commit and push Universe changes, then recompile.
/// Note: push operates ON the universe repo, so cwd must be inside a Universe.
pub fn push() -> Result<()> {
    // Non-developers cannot push Universe changes
    let profile = Profile::load_or_default()?;
    if !profile.is_developer() {
        return Err(RickError::InvalidState(
            "Push is restricted to developer profiles. Change your role with 'rick profile set developer'.".to_string(),
        ));
    }

    let cwd = env::current_dir()?;
    let _universe = Universe::load(&cwd).or_else(|_| resolver::resolve_universe_from_cwd())?;

    // Check for uncommitted changes in agents/ and workflows/
    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain", "agents/", "workflows/", ".rick/config.yaml"])
        .current_dir(&cwd)
        .output()
        .map_err(|e| RickError::Io(e))?;

    let changes = String::from_utf8_lossy(&status_output.stdout);
    let changed_files: Vec<&str> = changes.lines().filter(|l| !l.is_empty()).collect();

    if changed_files.is_empty() {
        println!("\x1b[36mRick: No Universe changes to push.\x1b[0m");
        return Ok(());
    }

    // Show what changed
    println!("\x1b[36mRick: Detected Universe changes:\x1b[0m");
    println!();

    let mut modified_agents: Vec<String> = Vec::new();
    let mut modified_workflows: Vec<String> = Vec::new();
    let mut other_files: Vec<String> = Vec::new();

    for line in &changed_files {
        let file = line.get(3..).unwrap_or(line).trim();
        if file.starts_with("agents/") {
            // Extract agent name from path: agents/chad-pm/soul.md -> chad-pm
            let parts: Vec<&str> = file.split('/').collect();
            if parts.len() >= 2 {
                let agent_name = parts[1].to_string();
                if !modified_agents.contains(&agent_name) {
                    modified_agents.push(agent_name);
                }
            }
        } else if file.starts_with("workflows/") {
            let parts: Vec<&str> = file.split('/').collect();
            if parts.len() >= 2 {
                let wf_name = parts[1].to_string();
                if !modified_workflows.contains(&wf_name) {
                    modified_workflows.push(wf_name);
                }
            }
        } else {
            other_files.push(file.to_string());
        }
    }

    for a in &modified_agents {
        println!("  \x1b[33m↻\x1b[0m Agent: \x1b[97m{}\x1b[0m", a);
    }
    for w in &modified_workflows {
        println!("  \x1b[33m↻\x1b[0m Workflow: \x1b[97m{}\x1b[0m", w);
    }
    for f in &other_files {
        println!("  \x1b[33m↻\x1b[0m {}", f);
    }

    // Build commit message
    let mut msg_parts: Vec<String> = Vec::new();
    if !modified_agents.is_empty() {
        if modified_agents.len() == 1 {
            msg_parts.push(format!("Update {} agent", modified_agents[0]));
        } else {
            msg_parts.push(format!("Update agents: {}", modified_agents.join(", ")));
        }
    }
    if !modified_workflows.is_empty() {
        if modified_workflows.len() == 1 {
            msg_parts.push(format!("Update {} workflow", modified_workflows[0]));
        } else {
            msg_parts.push(format!("Update workflows: {}", modified_workflows.join(", ")));
        }
    }
    if !other_files.is_empty() && msg_parts.is_empty() {
        msg_parts.push("Update Universe config".to_string());
    }

    let commit_msg = if msg_parts.is_empty() {
        "Update Universe".to_string()
    } else {
        msg_parts.join(", ")
    };

    // Generate branch name from commit message
    let branch_slug: String = commit_msg
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let branch_name = format!("rick/{}-{}", branch_slug, timestamp % 10000);

    println!();
    println!("\x1b[90m  Branch: {}\x1b[0m", branch_name);
    println!("\x1b[90m  Commit: \"{}\"\x1b[0m", commit_msg);

    // Create and switch to new branch
    let branch_status = std::process::Command::new("git")
        .args(["checkout", "-b", &branch_name])
        .current_dir(&cwd)
        .status()
        .map_err(|e| RickError::Io(e))?;

    if !branch_status.success() {
        return Err(RickError::InvalidState("Failed to create branch".to_string()));
    }

    // Stage agent/workflow/config files (including Memory.md files)
    let add_status = std::process::Command::new("git")
        .args(["add", "agents/", "workflows/", ".rick/config.yaml"])
        .current_dir(&cwd)
        .status()
        .map_err(|e| RickError::Io(e))?;

    if !add_status.success() {
        // Switch back to main before erroring
        let _ = std::process::Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&cwd)
            .status();
        return Err(RickError::InvalidState("git add failed".to_string()));
    }

    // Commit
    let commit_status = std::process::Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(&cwd)
        .status()
        .map_err(|e| RickError::Io(e))?;

    if !commit_status.success() {
        let _ = std::process::Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&cwd)
            .status();
        return Err(RickError::InvalidState("git commit failed".to_string()));
    }

    println!("  \x1b[32m✓\x1b[0m Committed");

    // Push branch to remote
    let push_status = std::process::Command::new("git")
        .args(["push", "-u", "origin", &branch_name])
        .current_dir(&cwd)
        .status()
        .map_err(|e| RickError::Io(e))?;

    if !push_status.success() {
        return Err(RickError::InvalidState("git push failed".to_string()));
    }

    println!("  \x1b[32m✓\x1b[0m Pushed branch");

    // Create PR via gh CLI
    let mut pr_body = format!("## Universe Changes\n\n{}",
        changed_files.iter()
            .map(|l| format!("- `{}`", l.get(3..).unwrap_or(l).trim()))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Template compliance audit (Task 8)
    if !modified_agents.is_empty() {
        if let Ok(Some(tmpl)) = template::get_template(&cwd, TemplateType::Agent) {
            let mut all_findings = Vec::new();
            for agent_name in &modified_agents {
                let agent_dir = cwd.join("agents").join(agent_name);
                if let Ok(a) = agent::Agent::load(&agent_dir) {
                    let findings = template::audit_agent_against_template(&a, &tmpl);
                    all_findings.extend(findings);
                }
            }
            if !all_findings.is_empty() {
                let report = template::format_compliance_report(&all_findings, &tmpl);
                pr_body.push_str("\n\n");
                pr_body.push_str(&report);
            }
        }
    }

    let pr_output = std::process::Command::new("gh")
        .args(["pr", "create", "--title", &commit_msg, "--body", &pr_body])
        .current_dir(&cwd)
        .output()
        .map_err(|e| RickError::Io(e))?;

    if pr_output.status.success() {
        let pr_url = String::from_utf8_lossy(&pr_output.stdout).trim().to_string();
        println!("  \x1b[32m✓\x1b[0m PR created: \x1b[36m{}\x1b[0m", pr_url);
    } else {
        let err = String::from_utf8_lossy(&pr_output.stderr);
        println!("  \x1b[33m!\x1b[0m Could not create PR via gh CLI: {}", err.trim());
        println!("    Create it manually at your repo's GitHub page.");
    }

    // Switch back to main
    let _ = std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&cwd)
        .status();

    println!();
    println!("\x1b[32mRick: PR is up. Merge it and your team gets the changes on next pull.\x1b[0m");

    Ok(())
}

/// Execute the `check` command — verify all agent dependencies are satisfied.
pub fn check() -> Result<()> {
    let universe = resolver::resolve_universe_from_cwd()?;
    let agents = agent::load_agents(&universe)?;

    println!(
        "\x1b[36mRick: Checking dependencies for Universe \"{}\"...\x1b[0m",
        universe.name
    );

    let agent_deps: Vec<(String, &agent::AgentDependencies)> = agents
        .iter()
        .filter(|a| !a.dependencies.is_empty())
        .map(|a| (a.name.clone(), &a.dependencies))
        .collect();

    if agent_deps.is_empty() {
        println!("\x1b[32mRick: No dependencies declared. All clear!\x1b[0m");
        return Ok(());
    }

    let report = deps::check_all(&agent_deps, &universe.path)?;

    if report.has_missing() {
        deps::print_report(&report);
    } else {
        println!("\x1b[32mRick: All dependencies satisfied!\x1b[0m");
        for r in &report.results {
            println!(
                "    \x1b[32m✓\x1b[0m {} \x1b[90m({:?}, required by {})\x1b[0m",
                r.name, r.dep_type, r.source_agent
            );
        }
    }

    Ok(())
}
