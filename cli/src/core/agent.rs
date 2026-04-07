use std::fs;
use std::path::{Path, PathBuf};

use crate::a2a::types::AgentPersona;
use crate::error::{RickError, Result};
use crate::core::profile::Profile;
use crate::core::universe::Universe;
use crate::parsers::yaml;

/// A tool + model pair identifying a runtime target.
/// tool: "claude" or "cursor" (the CLI binary)
/// model: any model name passed to --model (user-configurable)
#[derive(Debug, Clone)]
pub struct RuntimeSpec {
    pub tool: String,
    pub model: String,
}

impl RuntimeSpec {
    /// Generate a canonical runtime ID like "claude:sonnet"
    pub fn id(&self) -> String {
        format!("{}:{}", self.tool, self.model)
    }
}

/// Runtime configuration for an agent (from tools.md).
#[derive(Debug, Clone)]
pub struct AgentRuntimeConfig {
    pub preferred: RuntimeSpec,
    pub fallback: Vec<RuntimeSpec>,
}

/// A required MCP server dependency.
#[derive(Debug, Clone)]
pub struct McpDependency {
    pub name: String,
    pub why: String,
    pub install: String,
}

/// A required skill dependency.
#[derive(Debug, Clone)]
pub struct SkillDependency {
    pub name: String,
    pub why: String,
    pub install: String,
}

/// All dependencies declared by an agent's tools.md.
#[derive(Debug, Clone, Default)]
pub struct AgentDependencies {
    pub mcps: Vec<McpDependency>,
    pub skills: Vec<SkillDependency>,
}

impl AgentDependencies {
    /// Parse dependencies from tools.md content using the YAML parser.
    /// Returns Default if no `requires:` section found (backwards compatible).
    pub fn parse_from_tools(tools_content: &str) -> Self {
        let parsed = match yaml::parse_yaml(tools_content) {
            Ok(val) => val,
            Err(_) => return Self::default(),
        };

        let requires = match parsed.get("requires") {
            Some(val) => val,
            None => return Self::default(),
        };

        let mut deps = Self::default();

        if let Some(mcps_val) = requires.get("mcps") {
            if let Some(list) = mcps_val.as_list() {
                for item in list {
                    deps.mcps.push(McpDependency {
                        name: item.get_str("name").unwrap_or("").to_string(),
                        why: item.get_str("why").unwrap_or("").to_string(),
                        install: item.get_str("install").unwrap_or("").to_string(),
                    });
                }
            }
        }

        if let Some(skills_val) = requires.get("skills") {
            if let Some(list) = skills_val.as_list() {
                for item in list {
                    deps.skills.push(SkillDependency {
                        name: item.get_str("name").unwrap_or("").to_string(),
                        why: item.get_str("why").unwrap_or("").to_string(),
                        install: item.get_str("install").unwrap_or("").to_string(),
                    });
                }
            }
        }

        deps
    }

    pub fn is_empty(&self) -> bool {
        self.mcps.is_empty() && self.skills.is_empty()
    }
}

/// Represents an agent within a Universe.
#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub soul_first_line: String,
    pub path: PathBuf,
    pub soul: String,
    pub rules: String,
    pub tools: String,
    pub memory: String,
    pub dependencies: AgentDependencies,
}

impl Agent {
    /// Load a single agent from the given directory.
    pub fn load(dir: &Path) -> Result<Self> {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| RickError::NotFound("Invalid agent directory".to_string()))?
            .to_string();

        let soul_path = dir.join("soul.md");
        if !soul_path.exists() {
            return Err(RickError::NotFound(format!(
                "No soul.md found for agent '{}'",
                name
            )));
        }

        let soul = fs::read_to_string(&soul_path)?;
        let soul_first_line = soul
            .lines()
            .find(|l| {
                let trimmed = l.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            })
            .map(|l| l.trim().to_string())
            .unwrap_or_default();

        let rules = fs::read_to_string(dir.join("rules.md")).unwrap_or_default();
        let tools = fs::read_to_string(dir.join("tools.md")).unwrap_or_default();
        let memory = fs::read_to_string(dir.join("Memory.md")).unwrap_or_default();
        let dependencies = AgentDependencies::parse_from_tools(&tools);

        Ok(Agent {
            name,
            soul_first_line,
            path: dir.to_path_buf(),
            soul,
            rules,
            tools,
            memory,
            dependencies,
        })
    }

    /// Compile agent into a runtime-agnostic persona payload.
    /// This is what gets sent to RuntimeBackend via A2A TaskRequest.
    pub fn compile_persona(&self) -> AgentPersona {
        AgentPersona {
            name: self.name.clone(),
            role: self.soul_first_line.clone(),
            soul: self.soul.clone(),
            rules: self.rules.clone(),
        }
    }

    /// Parse optional runtime config from tools.md frontmatter.
    /// Returns None if no runtime section found (agent uses global default).
    /// Parse runtime config from tools.md.
    /// Expected format (Option B — explicit fields):
    /// ```yaml
    /// runtime:
    ///   preferred:
    ///     tool: claude
    ///     model: sonnet
    ///   fallback:
    ///     - tool: cursor
    ///       model: composer-2-fast
    /// ```
    pub fn runtime_config(&self) -> Option<AgentRuntimeConfig> {
        if self.tools.is_empty() {
            return None;
        }

        let parsed = yaml::parse_yaml(&self.tools).ok()?;
        let runtime_section = parsed.get("runtime")?;

        // Parse preferred: {tool, model}
        let preferred_section = runtime_section.get("preferred")?;
        let tool = preferred_section.get_str("tool")?.to_string();
        let model = preferred_section.get_str("model")?.to_string();
        let preferred = RuntimeSpec { tool, model };

        // Parse fallback: [{tool, model}, ...]
        let fallback: Vec<RuntimeSpec> = runtime_section
            .get("fallback")
            .and_then(|v| v.as_list())
            .map(|list| {
                list.iter()
                    .filter_map(|item| {
                        let tool = item.get_str("tool")?.to_string();
                        let model = item.get_str("model")?.to_string();
                        Some(RuntimeSpec { tool, model })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Some(AgentRuntimeConfig { preferred, fallback })
    }

    /// Compile this agent into a Claude Code sub-agent markdown file.
    /// If profile is non-developer, role constraints are injected.
    pub fn compile(&self, universe_name: &str, output_dir: &Path, universe_path: &Path, profile: &Profile) -> Result<PathBuf> {
        fs::create_dir_all(output_dir)?;

        let filename = format!("rick-{}-{}.md", universe_name, self.name);
        let output_path = output_dir.join(&filename);

        let mut content = String::new();
        content.push_str(&format!("# Rick Agent: {}\n\n", self.name));
        content.push_str("## Soul\n\n");
        content.push_str(&self.soul);
        if !self.soul.ends_with('\n') {
            content.push('\n');
        }

        if !self.rules.is_empty() {
            content.push_str("\n## Rules\n\n");
            content.push_str(&self.rules);
            if !self.rules.ends_with('\n') {
                content.push('\n');
            }
        }

        if !self.tools.is_empty() {
            content.push_str("\n## Tools\n\n");
            content.push_str(&self.tools);
            if !self.tools.ends_with('\n') {
                content.push('\n');
            }
        }

        if !self.memory.is_empty() {
            content.push_str("\n## Memory\n\n");
            content.push_str("Things you've learned from past work. Use this context to be more effective:\n\n");
            content.push_str(&self.memory);
            if !self.memory.ends_with('\n') {
                content.push('\n');
            }
        }

        // Memory write-back instructions
        content.push_str("\n## Memory Management\n\n");
        let memory_path = universe_path.join("agents").join(&self.name).join("Memory.md");
        content.push_str(&format!(
            "You have a persistent memory file at `{}`.\n\
            When you learn something worth remembering across sessions (user preferences, architectural decisions, recurring patterns, what worked or didn't), append it to your Memory.md file.\n\
            Keep entries concise — one line per learning, grouped by topic. Do NOT remove existing entries.\n",
            memory_path.display()
        ));

        // Role-based constraints (non-developer gets read-only rules)
        let constraints = profile.git_constraints();
        if !constraints.is_empty() {
            content.push('\n');
            content.push_str(&constraints);
            if !constraints.ends_with('\n') {
                content.push('\n');
            }
        }

        fs::write(&output_path, &content)?;
        Ok(output_path)
    }
}

/// Load all agents from a Universe.
pub fn load_agents(universe: &Universe) -> Result<Vec<Agent>> {
    let agents_dir = universe.agents_dir();
    if !agents_dir.exists() {
        return Ok(Vec::new());
    }

    let mut agents = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(&agents_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for entry in entries {
        match Agent::load(&entry.path()) {
            Ok(agent) => agents.push(agent),
            Err(_) => continue,
        }
    }

    Ok(agents)
}
