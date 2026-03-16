use std::env;
use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::core::agent::AgentDependencies;
use crate::parsers::json;

#[derive(Debug, PartialEq)]
pub enum DepType {
    Mcp,
    Skill,
}

#[derive(Debug, PartialEq)]
pub enum DepStatus {
    Found,
    Missing,
}

#[derive(Debug)]
pub struct DepCheckResult {
    pub name: String,
    pub dep_type: DepType,
    pub status: DepStatus,
    pub why: String,
    pub install: String,
    pub source_agent: String,
}

#[derive(Debug)]
pub struct DepsReport {
    pub results: Vec<DepCheckResult>,
}

impl DepsReport {
    pub fn has_missing(&self) -> bool {
        self.results.iter().any(|r| r.status == DepStatus::Missing)
    }

    pub fn missing(&self) -> Vec<&DepCheckResult> {
        self.results.iter().filter(|r| r.status == DepStatus::Missing).collect()
    }
}

/// Check all dependencies for agents involved in a workflow.
pub fn check_all(
    agent_deps: &[(String, &AgentDependencies)],
    universe_path: &Path,
) -> Result<DepsReport> {
    let user_mcps = load_user_mcp_names();
    let universe_mcps = load_universe_mcp_names(universe_path);
    let installed_skills = load_installed_skill_names();

    // Combine MCP sources
    let mut all_mcps = user_mcps;
    for name in universe_mcps {
        if !all_mcps.contains(&name) {
            all_mcps.push(name);
        }
    }

    let mut results = Vec::new();

    for (agent_name, deps) in agent_deps {
        // Check MCPs
        for mcp in &deps.mcps {
            let found = all_mcps.iter().any(|m| {
                m.eq_ignore_ascii_case(&mcp.name)
            });
            results.push(DepCheckResult {
                name: mcp.name.clone(),
                dep_type: DepType::Mcp,
                status: if found { DepStatus::Found } else { DepStatus::Missing },
                why: mcp.why.clone(),
                install: mcp.install.clone(),
                source_agent: agent_name.clone(),
            });
        }

        // Check Skills
        for skill in &deps.skills {
            let found = installed_skills.iter().any(|s| {
                s.eq_ignore_ascii_case(&skill.name) || s.contains(&skill.name)
            });
            results.push(DepCheckResult {
                name: skill.name.clone(),
                dep_type: DepType::Skill,
                status: if found { DepStatus::Found } else { DepStatus::Missing },
                why: skill.why.clone(),
                install: skill.install.clone(),
                source_agent: agent_name.clone(),
            });
        }
    }

    // Deduplicate: if multiple agents require the same dep, keep only one entry
    let mut seen = Vec::new();
    let mut deduped = Vec::new();
    for r in results {
        let key = format!("{:?}:{}", r.dep_type, r.name);
        if !seen.contains(&key) {
            seen.push(key);
            deduped.push(r);
        }
    }

    Ok(DepsReport { results: deduped })
}

/// Print a dependency check report to stdout.
pub fn print_report(report: &DepsReport) {
    let missing = report.missing();
    if missing.is_empty() {
        return;
    }

    println!();
    println!("\x1b[31mRick: Missing dependencies detected!\x1b[0m");
    println!();

    let missing_mcps: Vec<_> = missing.iter().filter(|r| r.dep_type == DepType::Mcp).collect();
    let missing_skills: Vec<_> = missing.iter().filter(|r| r.dep_type == DepType::Skill).collect();

    if !missing_mcps.is_empty() {
        println!("\x1b[97m  MCP Servers:\x1b[0m");
        for dep in &missing_mcps {
            println!(
                "    \x1b[31m✗\x1b[0m {} \x1b[90m(required by {})\x1b[0m",
                dep.name, dep.source_agent
            );
            if !dep.why.is_empty() {
                println!("      \x1b[90mWhy: {}\x1b[0m", dep.why);
            }
            if !dep.install.is_empty() {
                println!("      \x1b[36mInstall: {}\x1b[0m", dep.install);
            }
        }
        println!();
    }

    if !missing_skills.is_empty() {
        println!("\x1b[97m  Skills:\x1b[0m");
        for dep in &missing_skills {
            println!(
                "    \x1b[31m✗\x1b[0m {} \x1b[90m(required by {})\x1b[0m",
                dep.name, dep.source_agent
            );
            if !dep.why.is_empty() {
                println!("      \x1b[90mWhy: {}\x1b[0m", dep.why);
            }
            if !dep.install.is_empty() {
                println!("      \x1b[36mInstall: {}\x1b[0m", dep.install);
            }
        }
        println!();
    }

    println!("\x1b[33m  Use --force to run anyway, or install missing deps first.\x1b[0m");
}

/// Load MCP server names from ~/.claude.json.
fn load_user_mcp_names() -> Vec<String> {
    let home = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };

    let config_path = format!("{}/.claude.json", home);
    load_mcp_names_from_file(&config_path)
}

/// Load MCP server names from <universe>/.mcp.json.
fn load_universe_mcp_names(universe_path: &Path) -> Vec<String> {
    let mcp_path = universe_path.join(".mcp.json");
    let path_str = mcp_path.to_string_lossy().to_string();
    load_mcp_names_from_file(&path_str)
}

/// Parse MCP server names from a JSON config file.
/// Expects format: { "mcpServers": { "name1": {...}, "name2": {...} } }
fn load_mcp_names_from_file(path: &str) -> Vec<String> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let parsed = match json::parse_json(&content) {
        Ok(val) => val,
        Err(_) => return Vec::new(),
    };

    let servers = match parsed.get("mcpServers") {
        Some(json::JsonValue::Object(entries)) => entries,
        _ => return Vec::new(),
    };

    servers.iter().map(|(k, _)| k.clone()).collect()
}

/// Load installed skill names from ~/.claude/skills/.
fn load_installed_skill_names() -> Vec<String> {
    let home = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };

    let skills_dir = format!("{}/.claude/skills", home);
    let entries = match fs::read_dir(&skills_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}
