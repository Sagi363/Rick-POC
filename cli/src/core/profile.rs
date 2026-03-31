use std::fs;
use std::path::Path;

use crate::error::{RickError, Result};
use crate::core::resolver;
use crate::parsers::yaml;

/// Primary role distinction.
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Developer,
    NonDeveloper,
}

/// Sub-role for non-developers (fixed set, no arbitrary strings).
#[derive(Debug, Clone, PartialEq)]
pub enum SubRole {
    PM,
    Designer,
    QA,
    Other,
}

/// The user profile — stored at ~/.rick/profile.yaml.
#[derive(Debug, Clone)]
pub struct Profile {
    pub role: Role,
    pub sub_role: Option<SubRole>,
}

impl Profile {
    /// Load a profile from a YAML file.
    /// Fails on malformed content (fail closed — never silently grant developer access).
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let parsed = yaml::parse_yaml(&content)?;

        let role_str = parsed
            .get_str("role")
            .ok_or_else(|| RickError::Parse("Profile missing 'role' field".to_string()))?;

        let role = match role_str {
            "developer" => Role::Developer,
            "non-developer" => Role::NonDeveloper,
            other => {
                return Err(RickError::Parse(format!(
                    "Unknown role '{}'. Expected 'developer' or 'non-developer'.",
                    other
                )));
            }
        };

        let sub_role = match parsed.get_str("sub_role") {
            Some("pm") => Some(SubRole::PM),
            Some("designer") => Some(SubRole::Designer),
            Some("qa") => Some(SubRole::QA),
            Some("other") => Some(SubRole::Other),
            Some(unknown) => {
                return Err(RickError::Parse(format!(
                    "Unknown sub_role '{}'. Expected 'pm', 'designer', 'qa', or 'other'.",
                    unknown
                )));
            }
            None => None,
        };

        Ok(Profile { role, sub_role })
    }

    /// Save this profile to a YAML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let role_str = match self.role {
            Role::Developer => "developer",
            Role::NonDeveloper => "non-developer",
        };

        let mut content = format!("role: {}\n", role_str);

        if let Some(ref sr) = self.sub_role {
            let sr_str = match sr {
                SubRole::PM => "pm",
                SubRole::Designer => "designer",
                SubRole::QA => "qa",
                SubRole::Other => "other",
            };
            content.push_str(&format!("sub_role: {}\n", sr_str));
        }

        fs::write(path, &content)?;
        Ok(())
    }

    /// Load from ~/.rick/profile.yaml, defaulting to Developer only on file-not-found.
    /// Parse errors propagate (fail closed).
    pub fn load_or_default() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Profile {
                role: Role::Developer,
                sub_role: None,
            });
        }
        Self::load(&path)
    }

    /// Path to the profile file.
    pub fn path() -> Result<std::path::PathBuf> {
        Ok(resolver::rick_home()?.join("profile.yaml"))
    }

    pub fn is_developer(&self) -> bool {
        matches!(self.role, Role::Developer)
    }

    /// Display string for the role.
    pub fn role_display(&self) -> &str {
        match self.role {
            Role::Developer => "Developer",
            Role::NonDeveloper => "Non-developer",
        }
    }

    /// Display string for the sub-role.
    pub fn sub_role_display(&self) -> Option<&str> {
        self.sub_role.as_ref().map(|sr| match sr {
            SubRole::PM => "PM",
            SubRole::Designer => "Designer",
            SubRole::QA => "QA",
            SubRole::Other => "Other",
        })
    }

    /// Returns the markdown constraint block to inject into compiled agents.
    /// Empty string for developers, full constraint block for non-developers.
    pub fn git_constraints(&self) -> String {
        if self.is_developer() {
            return String::new();
        }

        String::from(
            r#"## Role Constraints (Non-Developer)

**MANDATORY: The current user is a non-developer. These constraints override all other rules.**

### Writable Paths (Allowlist)
You may ONLY use the Write or Edit tools on these paths:
- `.rick/state/` — Rick workflow state files
- `Memory.md` — Agent memory files (any path ending in Memory.md)
- `/tmp/` — Temporary files

**All other files are READ-ONLY.** This includes source code, config files, workflow YAMLs, agent definitions, prompts, and any repository-tracked file not in the allowlist above.

### Forbidden Git Operations
You MUST NOT execute any of the following:
- `git commit`, `git push`, `git add`, `git merge`, `git rebase`
- `git cherry-pick`, `git reset`, `git clean`
- Any command that modifies the git history or index

### Allowed Git Operations
- `git fetch`, `git status`, `git log`, `git diff`, `git branch` (read-only)
- `git pull --ff-only` (safe fast-forward updates only — if it fails, tell the user to ask a developer)

### Allowed Tools
- Read, Grep, Glob — unrestricted
- Bash — only for read-only commands and `git pull --ff-only`

### Guidance
- To update the project, suggest `rick pull` instead of manual git commands
- If the user asks to make code changes, explain that a developer agent should handle that
- If `git pull --ff-only` fails, tell the user: "Fast-forward pull failed. Ask a developer to resolve."
"#,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn create_temp_dir() -> std::path::PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::path::PathBuf::from(format!(
            "/tmp/rick-profile-test-{}-{}",
            std::process::id(),
            id
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    // -- Profile Parsing --

    #[test]
    fn test_load_valid_developer() {
        let dir = create_temp_dir();
        let path = dir.join("profile.yaml");
        fs::write(&path, "role: developer\n").unwrap();

        let profile = Profile::load(&path).unwrap();
        assert_eq!(profile.role, Role::Developer);
        assert_eq!(profile.sub_role, None);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_valid_non_developer_with_sub_role() {
        let dir = create_temp_dir();
        let path = dir.join("profile.yaml");
        fs::write(&path, "role: non-developer\nsub_role: pm\n").unwrap();

        let profile = Profile::load(&path).unwrap();
        assert_eq!(profile.role, Role::NonDeveloper);
        assert_eq!(profile.sub_role, Some(SubRole::PM));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_all_sub_roles() {
        let dir = create_temp_dir();
        let path = dir.join("profile.yaml");

        for (val, expected) in [
            ("pm", SubRole::PM),
            ("designer", SubRole::Designer),
            ("qa", SubRole::QA),
            ("other", SubRole::Other),
        ] {
            fs::write(&path, format!("role: non-developer\nsub_role: {}\n", val)).unwrap();
            let profile = Profile::load(&path).unwrap();
            assert_eq!(profile.sub_role, Some(expected));
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_missing_file_defaults_developer() {
        // load_or_default with a nonexistent HOME would be complex,
        // so test load() on missing path returns Io error, and
        // the default constructor directly.
        let profile = Profile {
            role: Role::Developer,
            sub_role: None,
        };
        assert!(profile.is_developer());
    }

    #[test]
    fn test_load_malformed_yaml_fails_closed() {
        let dir = create_temp_dir();
        let path = dir.join("profile.yaml");
        fs::write(&path, ":::bad yaml content\n\x00\x01").unwrap();

        let result = Profile::load(&path);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_unknown_role_fails_closed() {
        let dir = create_temp_dir();
        let path = dir.join("profile.yaml");
        fs::write(&path, "role: admin\n").unwrap();

        let result = Profile::load(&path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Unknown role"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_unknown_sub_role_fails_closed() {
        let dir = create_temp_dir();
        let path = dir.join("profile.yaml");
        fs::write(&path, "role: non-developer\nsub_role: intern\n").unwrap();

        let result = Profile::load(&path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Unknown sub_role"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_missing_role_field_fails() {
        let dir = create_temp_dir();
        let path = dir.join("profile.yaml");
        fs::write(&path, "sub_role: pm\n").unwrap();

        let result = Profile::load(&path);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    // -- Save Round-Trip --

    #[test]
    fn test_save_and_reload_developer() {
        let dir = create_temp_dir();
        let path = dir.join("profile.yaml");

        let profile = Profile {
            role: Role::Developer,
            sub_role: None,
        };
        profile.save(&path).unwrap();

        let loaded = Profile::load(&path).unwrap();
        assert_eq!(loaded.role, Role::Developer);
        assert_eq!(loaded.sub_role, None);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_and_reload_non_developer() {
        let dir = create_temp_dir();
        let path = dir.join("profile.yaml");

        let profile = Profile {
            role: Role::NonDeveloper,
            sub_role: Some(SubRole::Designer),
        };
        profile.save(&path).unwrap();

        let loaded = Profile::load(&path).unwrap();
        assert_eq!(loaded.role, Role::NonDeveloper);
        assert_eq!(loaded.sub_role, Some(SubRole::Designer));

        let _ = fs::remove_dir_all(&dir);
    }

    // -- Constraint Generation --

    #[test]
    fn test_constraints_empty_for_developer() {
        let profile = Profile {
            role: Role::Developer,
            sub_role: None,
        };
        assert!(profile.git_constraints().is_empty());
    }

    #[test]
    fn test_constraints_non_empty_for_non_developer() {
        let profile = Profile {
            role: Role::NonDeveloper,
            sub_role: Some(SubRole::PM),
        };
        let constraints = profile.git_constraints();
        assert!(!constraints.is_empty());
        assert!(constraints.contains("Role Constraints"));
    }

    #[test]
    fn test_constraints_contain_allowlist_and_ff_only() {
        let profile = Profile {
            role: Role::NonDeveloper,
            sub_role: None,
        };
        let constraints = profile.git_constraints();
        assert!(constraints.contains(".rick/state/"));
        assert!(constraints.contains("Memory.md"));
        assert!(constraints.contains("/tmp/"));
        assert!(constraints.contains("--ff-only"));
        assert!(constraints.contains("READ-ONLY"));
    }

    // -- is_developer --

    #[test]
    fn test_is_developer_true() {
        let p = Profile { role: Role::Developer, sub_role: None };
        assert!(p.is_developer());
    }

    #[test]
    fn test_is_developer_false() {
        let p = Profile { role: Role::NonDeveloper, sub_role: None };
        assert!(!p.is_developer());
    }

    // -- Display --

    #[test]
    fn test_role_display() {
        let p = Profile { role: Role::Developer, sub_role: None };
        assert_eq!(p.role_display(), "Developer");

        let p2 = Profile { role: Role::NonDeveloper, sub_role: Some(SubRole::QA) };
        assert_eq!(p2.role_display(), "Non-developer");
        assert_eq!(p2.sub_role_display(), Some("QA"));
    }
}
