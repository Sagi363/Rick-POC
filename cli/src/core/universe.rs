use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{RickError, Result};
use crate::parsers::yaml;

/// Represents a loaded Rick Universe.
#[derive(Debug)]
pub struct Universe {
    pub name: String,
    pub version: String,
    pub description: String,
    pub path: PathBuf,
}

impl Universe {
    /// Load a Universe from the given directory (must contain .rick/config.yaml).
    pub fn load(dir: &Path) -> Result<Self> {
        let config_path = dir.join(".rick").join("config.yaml");
        if !config_path.exists() {
            return Err(RickError::NotFound(format!(
                "No .rick/config.yaml found in {}",
                dir.display()
            )));
        }

        let content = fs::read_to_string(&config_path)?;
        let parsed = yaml::parse_yaml(&content)?;

        let name = parsed
            .get_str("name")
            .unwrap_or("unknown")
            .to_string();
        let version = parsed
            .get_str("version")
            .unwrap_or("0.0.0")
            .to_string();
        let description = parsed
            .get_str("description")
            .unwrap_or("")
            .to_string();

        Ok(Universe {
            name,
            version,
            description,
            path: dir.to_path_buf(),
        })
    }

    /// Return the path to the agents directory.
    pub fn agents_dir(&self) -> PathBuf {
        self.path.join("agents")
    }

    /// Return the path to the workflows directory.
    pub fn workflows_dir(&self) -> PathBuf {
        self.path.join("workflows")
    }
}
