use crate::a2a::types::*;

/// Wraps a raw task description with personality template (ENTRY/EXIT instructions).
/// Returns the full prompt string to be set as TaskRequest.description.
pub fn inject_personality_template(
    raw_task: &str,
    prior_steps: &[PriorStepSummary],
) -> String {
    if let Some(last_step) = prior_steps.last() {
        // Has prior context — agent should react to previous work
        format!(
            "The previous step was completed by {} ({}).\n\
             Here's a brief summary: {}\n\n\
             Before you begin your task, write a SHORT (1-2 sentence, max 30 words) \
             reaction to the previous agent's work in your persona's voice. \
             Then acknowledge your own task.\n\n\
             After you complete your task, write a SHORT (1 sentence, max 20 words) \
             exit line in your persona's voice.\n\n\
             Format:\n\
             AGENT_ENTRY: <reaction + acknowledgment>\n\
             <your actual work here>\n\
             AGENT_EXIT: <exit line>\n\n\
             Task: {}",
            last_step.agent,
            last_step.role,
            last_step.summary,
            raw_task,
        )
    } else {
        // First step — no prior context
        format!(
            "Before you begin your task, write a SHORT (1-2 sentence, max 30 words) \
             entry line in your persona's voice acknowledging what you're about to do.\n\n\
             After you complete your task, write a SHORT (1 sentence, max 20 words) \
             exit line in your persona's voice.\n\n\
             Format:\n\
             AGENT_ENTRY: <entry>\n\
             <your actual work here>\n\
             AGENT_EXIT: <exit>\n\n\
             Task: {}",
            raw_task,
        )
    }
}

/// Parses AGENT_ENTRY and AGENT_EXIT markers from LLM output.
/// Returns a TaskOutput with entry, content, exit, and raw fields.
pub fn parse_markers(raw_output: &str) -> TaskOutput {
    let mut entry = String::new();
    let mut exit = String::new();
    let mut content_lines: Vec<&str> = Vec::new();
    let mut in_content = false;

    for line in raw_output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("AGENT_ENTRY:") {
            entry = trimmed
                .strip_prefix("AGENT_ENTRY:")
                .unwrap_or("")
                .trim()
                .to_string();
            in_content = true;
        } else if trimmed.starts_with("AGENT_EXIT:") {
            exit = trimmed
                .strip_prefix("AGENT_EXIT:")
                .unwrap_or("")
                .trim()
                .to_string();
            in_content = false;
        } else if in_content {
            content_lines.push(line);
        }
    }

    // If no markers found, treat entire output as content
    let content = if entry.is_empty() && exit.is_empty() {
        raw_output.trim().to_string()
    } else {
        content_lines.join("\n").trim().to_string()
    };

    TaskOutput {
        entry,
        content,
        exit,
        raw: raw_output.to_string(),
    }
}

/// Generates Rick's handoff line before an agent runs.
/// Example: "Letting Neo architect this — the man lives for ASCII boxes."
pub fn generate_handoff(agent_name: &str, agent_role: &str, task_summary: &str) -> String {
    format!(
        "Handing this to {} ({}) — {}",
        agent_name,
        agent_role,
        truncate(task_summary, 60),
    )
}

/// Generates Rick's recap line after an agent finishes.
/// Example: "Neo's done. Design's in design.md. Next up: the developer."
pub fn generate_recap(
    agent_name: &str,
    duration_ms: u64,
    next_agent: Option<&str>,
) -> String {
    let duration_str = if duration_ms > 60_000 {
        format!("{}m {}s", duration_ms / 60_000, (duration_ms % 60_000) / 1000)
    } else {
        format!("{}s", duration_ms / 1000)
    };

    match next_agent {
        Some(next) => format!(
            "{} is done ({}). Next up: {}.",
            agent_name, duration_str, next
        ),
        None => format!(
            "{} is done ({}). That's the last step.",
            agent_name, duration_str
        ),
    }
}

/// Truncate string to max length (char boundary safe)
fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        // Find the closest char boundary <= max_len
        let mut idx = max_len;
        while !s.is_char_boundary(idx) && idx > 0 {
            idx -= 1;
        }
        &s[..idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markers_normal() {
        let input = "AGENT_ENTRY: Starting the design.\n\
                      Here is the architecture...\n\
                      Component A talks to Component B.\n\
                      AGENT_EXIT: Design complete.";
        let output = parse_markers(input);
        assert_eq!(output.entry, "Starting the design.");
        assert_eq!(output.exit, "Design complete.");
        assert!(output.content.contains("Component A"));
    }

    #[test]
    fn test_parse_markers_missing() {
        let input = "Just some regular output without markers.";
        let output = parse_markers(input);
        assert_eq!(output.entry, "");
        assert_eq!(output.exit, "");
        assert_eq!(output.content, input);
    }

    #[test]
    fn test_inject_personality_first_step() {
        let prompt = inject_personality_template("Write requirements", &[]);
        assert!(prompt.contains("AGENT_ENTRY:"));
        assert!(prompt.contains("AGENT_EXIT:"));
        assert!(prompt.contains("Write requirements"));
    }

    #[test]
    fn test_inject_personality_with_prior() {
        let prior = vec![PriorStepSummary {
            step_id: "step1".to_string(),
            agent: "PM".to_string(),
            role: "Product Manager".to_string(),
            entry: "Starting requirements".to_string(),
            exit: "Requirements done".to_string(),
            summary: "Wrote product requirements".to_string(),
        }];
        let prompt = inject_personality_template("Design architecture", &prior);
        assert!(prompt.contains("PM"));
        assert!(prompt.contains("Product Manager"));
        assert!(prompt.contains("reaction"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a very long string", 10), "this is a ");
    }
}
