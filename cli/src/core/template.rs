use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::agent::Agent;
use crate::error::{RickError, Result};
use crate::parsers::yaml;

// --- Data Structures (Task 1) ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateType {
    Agent,
    Workflow,
}

impl TemplateType {
    fn folder_name(&self) -> &str {
        match self {
            TemplateType::Agent => "agent",
            TemplateType::Workflow => "workflow",
        }
    }

    fn label(&self) -> &str {
        match self {
            TemplateType::Agent => "agent",
            TemplateType::Workflow => "workflow",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Template {
    pub template_type: TemplateType,
    pub content: String,
    pub source_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditSeverity {
    Pass,
    Warning,
}

#[derive(Debug, Clone)]
pub struct AuditFinding {
    pub agent_name: String,
    pub severity: AuditSeverity,
    pub message: String,
}

// --- Frontmatter Parser (Task 2) ---

/// Parse YAML frontmatter from a markdown file's content.
/// Returns a map of key-value pairs, or None if no valid frontmatter found.
/// Never panics — returns None on any parse failure.
pub fn parse_frontmatter(content: &str) -> Option<HashMap<String, String>> {
    let trimmed = content.trim_start_matches('\u{feff}'); // strip BOM
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..];
    let closing_pos = after_first.find("\n---")?;
    let frontmatter_block = &after_first[..closing_pos].trim();

    if frontmatter_block.is_empty() {
        return None;
    }

    let parsed = yaml::parse_yaml(frontmatter_block).ok()?;

    let mut map = HashMap::new();
    if let Some(entries) = parsed.as_map() {
        for (k, v) in entries {
            if let Some(s) = v.as_str() {
                map.insert(k.clone(), s.to_string());
            }
        }
    }

    if map.is_empty() {
        return None;
    }

    Some(map)
}

/// Extract the body content after frontmatter (everything after second `---`).
fn content_after_frontmatter(content: &str) -> &str {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return content;
    }
    let after_first = &trimmed[3..];
    match after_first.find("\n---") {
        Some(pos) => {
            let rest = &after_first[pos + 4..];
            // Skip the newline after closing ---
            if rest.starts_with('\n') { &rest[1..] } else { rest }
        }
        None => content,
    }
}

// --- Template Detection (Task 3) ---

/// Recursively collect all .md files under a directory.
fn collect_md_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_md_files(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Detect all templates in a Universe using cascading priority.
/// Returns templates found — may contain duplicates for validation downstream.
pub fn detect_templates(universe_path: &Path) -> Result<Vec<Template>> {
    let templates_dir = universe_path.join(".rick").join("templates");
    if !templates_dir.exists() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let types = [TemplateType::Agent, TemplateType::Workflow];

    for tt in &types {
        // Tier 1: Folder-based
        let folder = templates_dir.join(tt.folder_name());
        if folder.is_dir() {
            let md_files = collect_md_files(&folder);
            if !md_files.is_empty() {
                let mut content = String::new();
                for f in &md_files {
                    if let Ok(c) = fs::read_to_string(f) {
                        if !content.is_empty() {
                            content.push_str("\n\n");
                        }
                        content.push_str(&c);
                    }
                }
                results.push(Template {
                    template_type: tt.clone(),
                    content,
                    source_files: md_files,
                });
            }
            continue; // STOP for this type — folder wins
        }

        // Tier 2: Frontmatter scan
        let all_md = collect_md_files(&templates_dir);
        let mut frontmatter_matches: Vec<PathBuf> = Vec::new();
        for f in &all_md {
            if let Ok(c) = fs::read_to_string(f) {
                if let Some(fm) = parse_frontmatter(&c) {
                    if let Some(type_val) = fm.get("type") {
                        if type_val.eq_ignore_ascii_case(tt.folder_name()) {
                            frontmatter_matches.push(f.clone());
                        }
                    }
                }
            }
        }

        if frontmatter_matches.len() == 1 {
            let path = &frontmatter_matches[0];
            let raw = fs::read_to_string(path)?;
            let body = content_after_frontmatter(&raw);
            results.push(Template {
                template_type: tt.clone(),
                content: body.to_string(),
                source_files: vec![path.clone()],
            });
            continue; // STOP
        } else if frontmatter_matches.len() > 1 {
            // Duplicates — push one template per file so validate_no_duplicates catches it
            for path in &frontmatter_matches {
                let raw = fs::read_to_string(path)?;
                let body = content_after_frontmatter(&raw);
                results.push(Template {
                    template_type: tt.clone(),
                    content: body.to_string(),
                    source_files: vec![path.clone()],
                });
            }
            continue; // STOP — duplicates will be caught downstream
        }

        // Tier 3: Filename matching
        let type_keyword = tt.folder_name();
        let mut filename_matches: Vec<PathBuf> = Vec::new();
        for f in &all_md {
            // Skip files that are inside agent/ or workflow/ subfolders (tier 1 territory)
            if let Ok(rel) = f.strip_prefix(&templates_dir) {
                let components: Vec<_> = rel.components().collect();
                if components.len() > 1 {
                    continue; // In a subfolder — skip for tier 3
                }
            }
            if let Some(name) = f.file_stem().and_then(|s| s.to_str()) {
                if name.to_ascii_lowercase().contains(type_keyword) {
                    filename_matches.push(f.clone());
                }
            }
        }

        if filename_matches.len() == 1 {
            let path = &filename_matches[0];
            let content = fs::read_to_string(path)?;
            results.push(Template {
                template_type: tt.clone(),
                content,
                source_files: vec![path.clone()],
            });
        } else if filename_matches.len() > 1 {
            for path in &filename_matches {
                let content = fs::read_to_string(path)?;
                results.push(Template {
                    template_type: tt.clone(),
                    content,
                    source_files: vec![path.clone()],
                });
            }
        }
    }

    Ok(results)
}

// --- Duplicate Validation (Task 4) ---

/// Validate that there's at most one template per type.
/// Returns error with spec-matching message if duplicates found.
pub fn validate_no_duplicates(templates: &[Template]) -> Result<()> {
    for tt in &[TemplateType::Agent, TemplateType::Workflow] {
        let matching: Vec<&Template> = templates
            .iter()
            .filter(|t| t.template_type == *tt)
            .collect();

        if matching.len() > 1 {
            let file_list: Vec<String> = matching
                .iter()
                .flat_map(|t| t.source_files.iter())
                .map(|p| p.display().to_string())
                .collect();

            return Err(RickError::InvalidState(format!(
                "Found multiple {} templates: [{}]. A Universe should have exactly one. Please consolidate them.",
                tt.label(),
                file_list.join(", ")
            )));
        }
    }
    Ok(())
}

// --- Convenience Wrapper (Task 5) ---

/// Get a single template of the given type from a Universe.
/// Returns Ok(None) if no template exists for that type.
/// Returns Err if duplicates detected.
pub fn get_template(universe_path: &Path, tt: TemplateType) -> Result<Option<Template>> {
    let templates = detect_templates(universe_path)?;
    validate_no_duplicates(&templates)?;

    Ok(templates.into_iter().find(|t| t.template_type == tt))
}

// --- Agent Audit (Task 6) ---

/// Audit an agent against a template's guidelines.
/// Returns findings (Pass or Warning) based on heuristic checks.
pub fn audit_agent_against_template(agent: &Agent, template: &Template) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    let content_lower = template.content.to_ascii_lowercase();

    // Check required files
    let required_files = [
        ("soul.md", "soul.md"),
        ("rules.md", "rules.md"),
        ("tools.md", "tools.md"),
        ("memory.md", "Memory.md"),
    ];

    let mut missing_files = Vec::new();
    for (keyword, filename) in &required_files {
        if content_lower.contains(keyword) && !agent.path.join(filename).exists() {
            missing_files.push(*filename);
        }
    }

    if !missing_files.is_empty() {
        findings.push(AuditFinding {
            agent_name: agent.name.clone(),
            severity: AuditSeverity::Warning,
            message: format!("Missing required files: {}", missing_files.join(", ")),
        });
    }

    // Check rules.md line count
    if !agent.rules.is_empty() {
        let line_count = agent.rules.lines().count();
        // Look for line count thresholds in template (e.g., "under 150 lines", "under 100 lines")
        for threshold in [100, 150, 200] {
            let pattern = format!("under {} lines", threshold);
            if content_lower.contains(&pattern) && line_count > threshold {
                findings.push(AuditFinding {
                    agent_name: agent.name.clone(),
                    severity: AuditSeverity::Warning,
                    message: format!(
                        "rules.md is {} lines. Template recommends under {} lines; extract to skills.",
                        line_count, threshold
                    ),
                });
                break;
            }
        }
    }

    // Check for multi-role agents
    let roles = ["developer", "designer", "reviewer", "architect", "tester", "pm"];
    let soul_lower = agent.soul.to_ascii_lowercase();
    let detected_roles: Vec<&&str> = roles
        .iter()
        .filter(|r| soul_lower.contains(**r))
        .collect();

    if detected_roles.len() > 1 {
        // Check if the template warns against multi-role
        if content_lower.contains("single role")
            || content_lower.contains("one clear role")
            || content_lower.contains("do not create agents with multiple")
        {
            let role_names: Vec<&str> = detected_roles.iter().map(|r| **r).collect();
            findings.push(AuditFinding {
                agent_name: agent.name.clone(),
                severity: AuditSeverity::Warning,
                message: format!(
                    "Multiple roles detected ({}). Template recommends single-role agents.",
                    role_names.join(" + ")
                ),
            });
        }
    }

    // If no warnings, agent passes
    if findings.is_empty() {
        findings.push(AuditFinding {
            agent_name: agent.name.clone(),
            severity: AuditSeverity::Pass,
            message: "Compliant with template guidelines".to_string(),
        });
    }

    findings
}

// --- Compliance Report (Task 7) ---

/// Format audit findings into a markdown compliance section for PR descriptions.
pub fn format_compliance_report(findings: &[AuditFinding], template: &Template) -> String {
    if findings.is_empty() {
        return String::new();
    }

    let source_label = if template.source_files.len() == 1 {
        template.source_files[0]
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("template")
            .to_string()
    } else {
        // Folder-based: use the parent folder path
        template.source_files.first()
            .and_then(|p| p.parent())
            .and_then(|p| {
                // Get the relative path from .rick/templates/
                let s = p.display().to_string();
                s.rfind(".rick/templates/").map(|i| s[i..].to_string())
            })
            .unwrap_or_else(|| "template".to_string())
    };

    let mut report = String::new();
    report.push_str("## Template Compliance\n\n");
    report.push_str(&format!("### {} audit:\n", source_label));

    for finding in findings {
        let icon = match finding.severity {
            AuditSeverity::Pass => "\u{2705}",    // checkmark
            AuditSeverity::Warning => "\u{26a0}\u{fe0f}", // warning
        };
        report.push_str(&format!(
            "- {} `{}` \u{2014} {}\n",
            icon, finding.agent_name, finding.message
        ));
    }

    report
}

// --- Tests (Task 11) ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn create_temp_dir() -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "rick-test-{}-{}",
            std::process::id(),
            id
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn setup_universe(base: &Path) -> PathBuf {
        let templates_dir = base.join(".rick").join("templates");
        fs::create_dir_all(&templates_dir).unwrap();
        templates_dir
    }

    // -- Frontmatter parsing tests --

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = "---\ntype: agent\n---\n# Agent Template\n";
        let fm = parse_frontmatter(content).unwrap();
        assert_eq!(fm.get("type").unwrap(), "agent");
    }

    #[test]
    fn test_parse_frontmatter_with_bom() {
        let content = "\u{feff}---\ntype: workflow\n---\n# Workflow\n";
        let fm = parse_frontmatter(content).unwrap();
        assert_eq!(fm.get("type").unwrap(), "workflow");
    }

    #[test]
    fn test_parse_frontmatter_missing_closing() {
        let content = "---\ntype: agent\n# No closing delimiter\n";
        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn test_parse_frontmatter_empty() {
        let content = "---\n---\n# Empty frontmatter\n";
        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "# Just a regular markdown file\n";
        assert!(parse_frontmatter(content).is_none());
    }

    // -- Detection tier precedence tests --

    #[test]
    fn test_detect_folder_wins_over_frontmatter() {
        let base = create_temp_dir();
        let templates_dir = setup_universe(&base);

        // Create folder-based template
        let agent_folder = templates_dir.join("agent");
        fs::create_dir_all(&agent_folder).unwrap();
        fs::write(agent_folder.join("guidelines.md"), "# Folder Agent Template\n").unwrap();

        // Also create a frontmatter-based file
        fs::write(
            templates_dir.join("other.md"),
            "---\ntype: agent\n---\n# Frontmatter Agent Template\n",
        ).unwrap();

        let templates = detect_templates(&base).unwrap();
        let agent_templates: Vec<_> = templates.iter().filter(|t| t.template_type == TemplateType::Agent).collect();

        assert_eq!(agent_templates.len(), 1);
        assert!(agent_templates[0].content.contains("Folder Agent Template"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_detect_frontmatter_wins_over_filename() {
        let base = create_temp_dir();
        let templates_dir = setup_universe(&base);

        // Create frontmatter-based file (not named "agent")
        fs::write(
            templates_dir.join("guidelines.md"),
            "---\ntype: agent\n---\n# Frontmatter Template\n",
        ).unwrap();

        // Also create filename-based file (no frontmatter)
        fs::write(
            templates_dir.join("agent-template.md"),
            "# Filename Agent Template\n",
        ).unwrap();

        let templates = detect_templates(&base).unwrap();
        let agent_templates: Vec<_> = templates.iter().filter(|t| t.template_type == TemplateType::Agent).collect();

        assert_eq!(agent_templates.len(), 1);
        assert!(agent_templates[0].content.contains("Frontmatter Template"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_detect_filename_fallback() {
        let base = create_temp_dir();
        let templates_dir = setup_universe(&base);

        fs::write(
            templates_dir.join("my-agent-rules.md"),
            "# Agent Template via filename\n",
        ).unwrap();

        let templates = detect_templates(&base).unwrap();
        let agent_templates: Vec<_> = templates.iter().filter(|t| t.template_type == TemplateType::Agent).collect();

        assert_eq!(agent_templates.len(), 1);
        assert!(agent_templates[0].content.contains("Agent Template via filename"));

        let _ = fs::remove_dir_all(&base);
    }

    // -- Duplicate detection tests --

    #[test]
    fn test_duplicate_frontmatter_detected() {
        let base = create_temp_dir();
        let templates_dir = setup_universe(&base);

        fs::write(
            templates_dir.join("file1.md"),
            "---\ntype: agent\n---\n# First\n",
        ).unwrap();
        fs::write(
            templates_dir.join("file2.md"),
            "---\ntype: agent\n---\n# Second\n",
        ).unwrap();

        let templates = detect_templates(&base).unwrap();
        let result = validate_no_duplicates(&templates);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("multiple agent templates"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_duplicate_filename_detected() {
        let base = create_temp_dir();
        let templates_dir = setup_universe(&base);

        fs::write(templates_dir.join("agent-v1.md"), "# V1\n").unwrap();
        fs::write(templates_dir.join("agent-v2.md"), "# V2\n").unwrap();

        let templates = detect_templates(&base).unwrap();
        let result = validate_no_duplicates(&templates);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&base);
    }

    // -- Folder file ordering test --

    #[test]
    fn test_folder_files_sorted_alphabetically() {
        let base = create_temp_dir();
        let templates_dir = setup_universe(&base);

        let agent_folder = templates_dir.join("agent");
        fs::create_dir_all(&agent_folder).unwrap();
        fs::write(agent_folder.join("z-last.md"), "LAST").unwrap();
        fs::write(agent_folder.join("a-first.md"), "FIRST").unwrap();
        fs::write(agent_folder.join("m-middle.md"), "MIDDLE").unwrap();

        let templates = detect_templates(&base).unwrap();
        let agent = templates.iter().find(|t| t.template_type == TemplateType::Agent).unwrap();

        // Content should be in alphabetical order: FIRST, MIDDLE, LAST
        let first_pos = agent.content.find("FIRST").unwrap();
        let middle_pos = agent.content.find("MIDDLE").unwrap();
        let last_pos = agent.content.find("LAST").unwrap();
        assert!(first_pos < middle_pos);
        assert!(middle_pos < last_pos);

        let _ = fs::remove_dir_all(&base);
    }

    // -- No templates dir --

    #[test]
    fn test_no_templates_dir_returns_empty() {
        let base = create_temp_dir();
        fs::create_dir_all(base.join(".rick")).unwrap();
        // No templates/ dir

        let templates = detect_templates(&base).unwrap();
        assert!(templates.is_empty());

        let _ = fs::remove_dir_all(&base);
    }

    // -- Compliance report formatting --

    #[test]
    fn test_compliance_report_format() {
        let template = Template {
            template_type: TemplateType::Agent,
            content: String::new(),
            source_files: vec![PathBuf::from(".rick/templates/agent-template.md")],
        };

        let findings = vec![
            AuditFinding {
                agent_name: "good-agent".to_string(),
                severity: AuditSeverity::Pass,
                message: "Compliant with template guidelines".to_string(),
            },
            AuditFinding {
                agent_name: "bad-agent".to_string(),
                severity: AuditSeverity::Warning,
                message: "rules.md is 300 lines. Template recommends under 150 lines; extract to skills.".to_string(),
            },
        ];

        let report = format_compliance_report(&findings, &template);
        assert!(report.contains("## Template Compliance"));
        assert!(report.contains("agent-template.md audit:"));
        assert!(report.contains("`good-agent`"));
        assert!(report.contains("`bad-agent`"));
        assert!(report.contains("300 lines"));
    }

    // -- End-to-end smoke test: full Universe with good + bad agents --

    #[test]
    fn test_e2e_audit_good_and_bad_agents() {
        let base = create_temp_dir();
        let templates_dir = setup_universe(&base);

        // Create folder-based agent template
        let agent_folder = templates_dir.join("agent");
        fs::create_dir_all(&agent_folder).unwrap();
        fs::write(agent_folder.join("guidelines.md"), r#"
# Agent Template
## Philosophy
Every agent has ONE clear role.
## Required Files
- soul.md
- rules.md
- tools.md
- Memory.md
## Rules
Keep rules.md under 150 lines.
## Anti-Patterns
- DO NOT create agents with multiple hats
"#).unwrap();

        // Create agents dir
        let agents_dir = base.join("agents");

        // Good agent: single role, short rules, all files present
        let good = agents_dir.join("good-agent");
        fs::create_dir_all(&good).unwrap();
        fs::write(good.join("soul.md"), "You are **GoodBot** — a developer who writes clean code.").unwrap();
        fs::write(good.join("rules.md"), "- Write tests\n- Keep functions short\n").unwrap();
        fs::write(good.join("tools.md"), "model: sonnet\n").unwrap();
        fs::write(good.join("Memory.md"), "# Memory\n").unwrap();

        // Bad agent: multi-role, 200-line rules, missing Memory.md
        let bad = agents_dir.join("bad-agent");
        fs::create_dir_all(&bad).unwrap();
        fs::write(bad.join("soul.md"), "You are **BadBot** — a developer and designer and reviewer who does everything.").unwrap();
        let long_rules: String = (0..200).map(|i| format!("- Rule {}: Do something\n", i)).collect();
        fs::write(bad.join("rules.md"), &long_rules).unwrap();
        fs::write(bad.join("tools.md"), "model: opus\n").unwrap();
        // No Memory.md intentionally

        // Detect template
        let tmpl = get_template(&base, TemplateType::Agent).unwrap().unwrap();
        assert!(tmpl.content.contains("ONE clear role"));

        // Audit good agent
        let good_agent = crate::core::agent::Agent::load(&good).unwrap();
        let good_findings = audit_agent_against_template(&good_agent, &tmpl);
        assert_eq!(good_findings.len(), 1);
        assert_eq!(good_findings[0].severity, AuditSeverity::Pass);

        // Audit bad agent
        let bad_agent = crate::core::agent::Agent::load(&bad).unwrap();
        let bad_findings = audit_agent_against_template(&bad_agent, &tmpl);
        // Should have warnings for: missing Memory.md, rules too long, multiple roles
        assert!(bad_findings.len() >= 2, "Expected at least 2 warnings, got {}: {:?}", bad_findings.len(), bad_findings);
        assert!(bad_findings.iter().all(|f| f.severity == AuditSeverity::Warning));

        // Check specific warnings
        let messages: Vec<&str> = bad_findings.iter().map(|f| f.message.as_str()).collect();
        assert!(messages.iter().any(|m| m.contains("Missing required files")), "Expected missing files warning");
        assert!(messages.iter().any(|m| m.contains("200 lines")), "Expected rules line count warning, got: {:?}", messages);

        // Format report
        let mut all_findings = good_findings;
        all_findings.extend(bad_findings);
        let report = format_compliance_report(&all_findings, &tmpl);
        assert!(report.contains("## Template Compliance"));
        assert!(report.contains("`good-agent`"));
        assert!(report.contains("`bad-agent`"));

        let _ = fs::remove_dir_all(&base);
    }
}
