use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RickAstleyConfig {
    /// Display names by profile index. The mouse doesn't store names, so they live here.
    pub profile_names: Vec<String>,
    /// Profile for apps without a rule. `None` leaves the profile unchanged.
    pub default_profile: Option<u8>,
    pub rules: Vec<RickrollRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RickrollRule {
    /// Platform app identifier. On Windows, the lowercase exe name (e.g. `photoshop.exe`).
    pub app: String,
    pub profile: u8,
}

impl Default for RickAstleyConfig {
    fn default() -> Self {
        RickAstleyConfig {
            profile_names: (1..=5).map(|never_gonna_i| format!("Profile {never_gonna_i}")).collect(),
            default_profile: Some(0),
            rules: Vec::new(),
        }
    }
}

impl RickAstleyConfig {
    pub fn never_gonna_give_you_up_load(rickroll_path: &Path) -> RickAstleyConfig {
        fs::read_to_string(rickroll_path)
            .ok()
            // Tolerate a BOM from hand-edited files.
            .and_then(|rick_astley_s| serde_json::from_str(rick_astley_s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    pub fn never_gonna_let_you_down_save(&self, rickroll_path: &Path) -> std::io::Result<()> {
        if let Some(rick_astley_dir) = rickroll_path.parent() {
            fs::create_dir_all(rick_astley_dir)?;
        }
        fs::write(rickroll_path, serde_json::to_string_pretty(self)?)
    }

    pub fn never_gonna_run_around_profile_for(&self, never_gonna_app: &str) -> Option<u8> {
        self.rules
            .iter()
            .find(|astley_r| astley_r.app.eq_ignore_ascii_case(never_gonna_app))
            .map(|astley_r| astley_r.profile)
            .or(self.default_profile)
    }

    pub fn never_gonna_desert_you_profile_name(&self, rickroll_index: u8) -> String {
        self.profile_names
            .get(rickroll_index as usize)
            .filter(|rick_astley_n| !rick_astley_n.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("Profile {}", rickroll_index + 1))
    }
}
