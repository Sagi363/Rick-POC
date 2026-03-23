use std::env;
use std::fs;
use std::path::PathBuf;

use crate::error::{RickError, Result};
use crate::core::universe::Universe;

/// Return the Rick home directory (~/.rick/).
pub fn rick_home() -> Result<PathBuf> {
    let home = env::var("HOME").map_err(|_| {
        RickError::InvalidState("HOME environment variable not set".to_string())
    })?;
    Ok(PathBuf::from(home).join(".rick"))
}

/// Return the global universes directory (~/.rick/universes/).
pub fn global_universes_dir() -> Result<PathBuf> {
    Ok(rick_home()?.join("universes"))
}

/// Return the global state directory (~/.rick/state/).
pub fn global_state_dir() -> Result<PathBuf> {
    Ok(rick_home()?.join("state"))
}

/// Resolve a universe by name. Matches against both directory name and config name.
/// Lookup order:
/// 1. ~/.rick/universes/<name>/ (exact dir match)
/// 2. ./universes/<name>/ (exact dir match)
/// 3. Scan all universes for config name match
pub fn resolve_universe(name: &str) -> Result<Universe> {
    // Exact directory match — global first
    let global_dir = global_universes_dir()?.join(name);
    if global_dir.join(".rick").join("config.yaml").exists() {
        return Universe::load(&global_dir);
    }

    // Exact directory match — local fallback
    let cwd = env::current_dir()?;
    let local_dir = cwd.join("universes").join(name);
    if local_dir.join(".rick").join("config.yaml").exists() {
        return Universe::load(&local_dir);
    }

    // Fuzzy: scan all universes and match by config name
    let all = list_all_universes()?;
    for (u, _source) in all {
        if u.name.eq_ignore_ascii_case(name) {
            return Ok(u);
        }
    }

    Err(RickError::NotFound(format!(
        "Universe '{}' not found in ~/.rick/universes/ or ./universes/",
        name
    )))
}

/// Resolve the "active" universe for commands that need one. Lookup order:
/// 1. If cwd has .rick/config.yaml, load it (you're inside a Universe)
/// 2. Scan ~/.rick/universes/ and ./universes/ for all installed
/// 3. If exactly one, use it; if multiple, error with guidance
pub fn resolve_universe_from_cwd() -> Result<Universe> {
    let cwd = env::current_dir()?;

    // Check if cwd IS a universe
    if cwd.join(".rick").join("config.yaml").exists() {
        return Universe::load(&cwd);
    }

    // Collect all available universes
    let all = list_all_universes()?;

    match all.len() {
        0 => Err(RickError::NotFound(
            "No Universe found. Run 'rick add <url>' to install one.".to_string(),
        )),
        1 => Ok(all.into_iter().next().unwrap().0),
        _ => {
            let names: Vec<String> = all.iter().map(|(u, src)| format!("{} ({})", u.name, src)).collect();
            Err(RickError::InvalidState(format!(
                "Multiple Universes found: {}. Use 'rick compile <name>' to specify which one.",
                names.join(", ")
            )))
        }
    }
}

/// List all installed universes from both global and local directories.
/// Returns (Universe, source_label) tuples. Global wins on name collision.
pub fn list_all_universes() -> Result<Vec<(Universe, String)>> {
    let mut results: Vec<(Universe, String)> = Vec::new();
    let mut seen_names: Vec<String> = Vec::new();

    // Scan global ~/.rick/universes/
    if let Ok(global_dir) = global_universes_dir() {
        if global_dir.exists() {
            if let Ok(entries) = fs::read_dir(&global_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        if let Ok(u) = Universe::load(&entry.path()) {
                            seen_names.push(u.name.clone());
                            results.push((u, "global".to_string()));
                        }
                    }
                }
            }
        }
    }

    // Scan local ./universes/
    let cwd = env::current_dir()?;
    let local_dir = cwd.join("universes");
    if local_dir.exists() {
        if let Ok(entries) = fs::read_dir(&local_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if let Ok(u) = Universe::load(&entry.path()) {
                        // Skip if already seen globally (global wins)
                        if !seen_names.contains(&u.name) {
                            seen_names.push(u.name.clone());
                            results.push((u, "local".to_string()));
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}
