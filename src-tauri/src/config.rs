use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Display names by profile index. The mouse doesn't store names, so they live here.
    pub profile_names: Vec<String>,
    /// Profile for apps without a rule. `None` leaves the profile unchanged.
    pub default_profile: Option<u8>,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Platform app identifier. On Windows, the lowercase exe name (e.g. `photoshop.exe`).
    pub app: String,
    pub profile: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            profile_names: (1..=5).map(|i| format!("Profile {i}")).collect(),
            default_profile: Some(0),
            rules: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Config {
        fs::read_to_string(path)
            .ok()
            // Tolerate a BOM from hand-edited files.
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)
    }

    pub fn profile_for(&self, app: &str) -> Option<u8> {
        self.rules
            .iter()
            .find(|r| r.app.eq_ignore_ascii_case(app))
            .map(|r| r.profile)
            .or(self.default_profile)
    }

    pub fn profile_name(&self, index: u8) -> String {
        self.profile_names
            .get(index as usize)
            .filter(|n| !n.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("Profile {}", index + 1))
    }
}
