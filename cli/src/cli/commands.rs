use std::env;
use std::io::{self, Write as IoWrite};

use crate::error::{RickError, Result};
use crate::core::agent;
use crate::core::deps;
use crate::core::state::{self, WorkflowState, StepState};
use crate::core::universe::Universe;
use crate::core::workflow;
use crate::parsers::json::{self, JsonValue};

/// Embedded SKILL.md content — compiled into the binary via include_str!().
const SKILL_CONTENT: &str = include_str!("../../../integrations/claude-code/skill/SKILL.md");

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
    let cwd = env::current_dir()?;
    let universe = Universe::load(&cwd)?;
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
    let cwd = env::current_dir()?;
    let universe = Universe::load(&cwd)?;
    let workflows = workflow::load_workflows(&universe)?;

    println!("\x1b[36mRick: Workflows in {}:\x1b[0m", universe.name);
    println!();

    for wf in &workflows {
        println!("\x1b[97m  {}\x1b[0m", wf.name);
        println!("\x1b[90m    {}\x1b[0m", wf.description);
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

    Ok(())
}

/// Execute the `compile` command.
pub fn compile() -> Result<()> {
    let cwd = env::current_dir()?;
    let universe = Universe::load(&cwd)?;
    let agents = agent::load_agents(&universe)?;

    println!(
        "\x1b[36mRick: Compiling Universe \"{}\"...\x1b[0m",
        universe.name
    );

    let output_dir = cwd.join(".claude").join("agents");
    let mut compiled_paths = Vec::new();

    for a in &agents {
        let path = a.compile(&universe.name, &output_dir)?;
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

    Ok(())
}

/// Execute the `run <workflow>` command.
pub fn run(workflow_name: &str, force: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let universe = Universe::load(&cwd)?;
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

    let steps: Vec<StepState> = wf
        .steps
        .iter()
        .map(|s| StepState {
            id: s.id.clone(),
            agent: s.agent.clone(),
            task: s.task.clone(),
            status: "pending".to_string(),
        })
        .collect();

    let state = WorkflowState {
        workflow_id: wf_id.clone(),
        workflow_name: wf.name.clone(),
        universe_name: universe.name.clone(),
        status: "started".to_string(),
        current_step: 0,
        total_steps: wf.steps.len(),
        steps,
    };

    let state_dir = cwd.join(".rick").join("state");
    state.save(&state_dir)?;

    println!(
        "\x1b[36mRick: Starting workflow \"{}\" in {}\x1b[0m",
        wf.name, universe.name
    );
    println!("\x1b[36m  Workflow ID: {}\x1b[0m", wf_id);
    println!();
    println!("\x1b[97m  Execution Plan:\x1b[0m");

    for (i, step) in wf.steps.iter().enumerate() {
        if i == 0 {
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
    println!(
        "\x1b[36mRick: Ready to execute step 1: {}\x1b[0m",
        wf.steps[0].agent
    );

    Ok(())
}

/// Execute the `status` command.
pub fn status() -> Result<()> {
    let cwd = env::current_dir()?;
    let state_dir = cwd.join(".rick").join("state");
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

    Ok(())
}

/// Execute the `init` command.
pub fn init() -> Result<()> {
    let cwd = env::current_dir()?;
    let universes_dir = cwd.join("universes");
    std::fs::create_dir_all(&universes_dir)?;

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

    let cwd = env::current_dir()?;
    let universes_dir = cwd.join("universes");
    std::fs::create_dir_all(&universes_dir)?;
    let target = universes_dir.join(&name);

    if target.exists() {
        return Err(RickError::InvalidState(format!(
            "Universe '{}' already exists in universes/",
            name
        )));
    }

    println!(
        "\x1b[36mRick: Adding universe '{}' from {}\x1b[0m",
        name, url
    );

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

    // Auto-compile agents
    println!();
    println!("\x1b[36mRick: Compiling agents...\x1b[0m");

    let output_dir = target.join(".claude").join("agents");
    let mut compiled_count = 0;
    for a in &agents {
        a.compile(&universe.name, &output_dir)?;
        compiled_count += 1;
    }

    println!(
        "\x1b[32m  Compiled {} agents to {}/\x1b[0m",
        compiled_count,
        output_dir.strip_prefix(&cwd).unwrap_or(&output_dir).display()
    );

    println!();
    println!("\x1b[36mRick: Universe '{}' is ready!\x1b[0m", name);
    println!(
        "\x1b[97m  cd universes/{} && rick list workflows\x1b[0m",
        name
    );

    Ok(())
}

/// Execute the `next` command.
pub fn next() -> Result<()> {
    let cwd = env::current_dir()?;
    let state_dir = cwd.join(".rick").join("state");
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

    let next_step = current.current_step + 1;
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
// Setup command
// ---------------------------------------------------------------------------

/// Execute the `setup` command — full onboarding: skill, persona, permissions, universe, deps.
pub fn setup(universe_url: Option<&str>, install_deps: bool) -> Result<()> {
    let home = env::var("HOME").map_err(|_| {
        RickError::InvalidState("HOME environment variable not set".to_string())
    })?;

    println!("\x1b[36mRick: Running setup...\x1b[0m");
    println!();

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

    // Step 3: Permissions guidance
    println!();
    show_permissions_guidance()?;

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
fn install_skill(home: &str) -> Result<WriteStatus> {
    let lowercase_path = format!("{}/.claude/skills/rick/SKILL.md", home);
    let uppercase_path = format!("{}/.claude/skills/Rick/SKILL.md", home);
    let status = write_if_needed(&lowercase_path, SKILL_CONTENT)?;
    write_if_needed(&uppercase_path, SKILL_CONTENT)?;
    Ok(status)
}

/// Show permissions guidance — detect missing perms and offer options.
fn show_permissions_guidance() -> Result<()> {
    let rick_perms = vec![
        "Bash(rick *)",
        "Bash(cd * && rick *)",
    ];

    // Check if permissions are already in project-level settings
    let cwd = env::current_dir()?;
    let project_settings_path = cwd.join(".claude").join("settings.json");
    let existing = load_allow_permissions(&project_settings_path.to_string_lossy());

    let missing: Vec<&&str> = rick_perms.iter().filter(|p| !existing.contains(&p.to_string())).collect();

    if missing.is_empty() {
        println!("  \x1b[32m✓\x1b[0m Permissions   Already configured");
        return Ok(());
    }

    println!("  \x1b[33m!\x1b[0m Permissions   Claude Code permissions needed for smooth operation:");
    println!();
    for p in &missing {
        println!("      \x1b[97m{}\x1b[0m  — Allows Rick CLI commands without approval prompts", p);
    }
    println!();
    println!("    \x1b[97mOptions:\x1b[0m");
    println!("      [1] Add to project settings (.claude/settings.json) — recommended for teams");
    println!("      [2] Show me the JSON so I can add it myself");
    println!("      [3] Skip — I'll approve commands manually each time \x1b[33m(not recommended)\x1b[0m");
    println!();

    // Check if stdin is a terminal for interactive prompt
    let choice = read_user_choice("    Choose [1/2/3]: ", "2");

    match choice.as_str() {
        "1" => {
            write_project_permissions(&project_settings_path.to_string_lossy(), &rick_perms)?;
            println!("    \x1b[32m✓\x1b[0m Permissions added to .claude/settings.json");
        }
        "2" => {
            println!();
            println!("    Add this to your .claude/settings.json or ~/.claude/settings.json:");
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

/// Read a single choice from the user. Falls back to `default` if stdin is not interactive.
fn read_user_choice(prompt: &str, default: &str) -> String {
    print!("{}", prompt);
    if io::stdout().flush().is_err() {
        return default.to_string();
    }

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => default.to_string(), // EOF (piped/non-interactive)
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

/// Install MCP dependencies declared by agents in the current Universe.
fn install_agent_deps() -> Result<()> {
    let cwd = env::current_dir()?;
    let universe = Universe::load(&cwd)?;
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
pub fn invite() -> Result<()> {
    let cwd = env::current_dir()?;
    let universe = Universe::load(&cwd)?;
    let agents = agent::load_agents(&universe)?;
    let workflows = workflow::load_workflows(&universe)?;

    if universe.repository.is_empty() {
        return Err(RickError::InvalidState(
            "No 'repository' field in .rick/config.yaml — add it to enable invite links.".to_string(),
        ));
    }

    let repo_url = &universe.repository;
    let install_url = "https://raw.githubusercontent.com/Sagi363/Rick-POC/main/install.sh";

    println!();
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

/// Execute the `push` command — commit and push Universe changes, then recompile.
pub fn push() -> Result<()> {
    let cwd = env::current_dir()?;
    let _universe = Universe::load(&cwd)?;

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
    let pr_body = format!("## Universe Changes\n\n{}",
        changed_files.iter()
            .map(|l| format!("- `{}`", l.get(3..).unwrap_or(l).trim()))
            .collect::<Vec<_>>()
            .join("\n")
    );

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
    let cwd = env::current_dir()?;
    let universe = Universe::load(&cwd)?;
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
