use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

/// Transpiler & daemon addon specification in `psudo.toml` (`[addons.<name>]`)
// Keeping Rust-specific crate deps and attributes like `#[tokio::main]` inside `psudo.toml`
// addon specs lets `.psudo` stay a simple toy language while still compiling rich Rust apps.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AddonSpec {
    #[serde(default)]
    pub builtins: Vec<String>,
    #[serde(default)]
    pub cargo_deps: Vec<String>,
    #[serde(default)]
    pub rust_main_attr: Option<String>,
    #[serde(default)]
    pub rust_main_async: bool,
    #[serde(default)]
    pub rust_file: Option<String>,
    #[serde(default)]
    pub daemon_hook: Option<String>,
    #[serde(default)]
    pub mock_returns: std::collections::BTreeMap<String, String>,
}

/// Project configuration for psudo
// Explicit size caps in psudo.toml keep synthesized code from accidentally filling disk
// during unguarded sandbox loops.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    /// Sandbox configuration
    #[serde(default)]
    pub sandbox: SandboxConfig,
    /// Optional transpiler and daemon builtin-stub addons
    #[serde(default)]
    pub addons: std::collections::BTreeMap<String, AddonSpec>,
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Root directory for sandbox (relative to project root)
    /// Default: ".sandbox"
    #[serde(default = "default_sandbox_root")]
    pub root: String,

    /// Maximum file size in bytes
    /// Default: 1MB
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// Maximum total sandbox size in bytes
    /// Default: 10MB
    #[serde(default = "default_max_sandbox_size")]
    pub max_sandbox_size: u64,

    /// Allow reading outside sandbox (for config files, etc)
    /// Default: false
    #[serde(default)]
    pub allow_external_read: bool,

    /// Allowed external read paths (relative to project root)
    /// Default: empty
    #[serde(default)]
    pub allowed_external_reads: Vec<String>,

    /// Deny list for external reads (security)
    /// Default: ["/etc", "/root", "/home", "C:\\Windows", "C:\\Program Files"]
    #[serde(default = "default_deny_list")]
    pub deny_external_reads: Vec<String>,
}

fn default_sandbox_root() -> String {
    ".sandbox".to_string()
}

fn default_max_file_size() -> u64 {
    1_048_576 // 1MB
}

fn default_max_sandbox_size() -> u64 {
    10_485_760 // 10MB
}

fn default_deny_list() -> Vec<String> {
    vec![
        "/etc".to_string(),
        "/root".to_string(),
        "/home".to_string(),
        "C:\\Windows".to_string(),
        "C:\\Program Files".to_string(),
        "C:\\Program Files (x86)".to_string(),
        "C:\\Users\\*\\AppData".to_string(),
    ]
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            root: default_sandbox_root(),
            max_file_size: default_max_file_size(),
            max_sandbox_size: default_max_sandbox_size(),
            allow_external_read: false,
            allowed_external_reads: Vec::new(),
            deny_external_reads: default_deny_list(),
        }
    }
}

/// Find project config file by walking up from current directory
pub fn find_project_config() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    loop {
        let config_path = current.join("psudo.toml");
        if config_path.exists() {
            return Some(config_path);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Load project configuration
pub fn load_project_config() -> ProjectConfig {
    if let Ok(raw) = std::env::var("PSUDO_CAPSULE_TOML") {
        if !raw.trim().is_empty() {
            return parse_project_config_str(raw.trim_start_matches('\u{feff}'));
        }
    }
    if let Some(config_path) = find_project_config() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<ProjectConfig>(content.trim_start_matches('\u{feff}')) {
                return config;
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let local_bin = cwd.join("psudo.bin");
        if local_bin.exists() {
            if let Ok(cap) = crate::vfs::WorkspaceCapsule::load_from_file(&local_bin) {
                if let Some(toml_str) = cap.virtual_files.get("psudo.toml") {
                    return parse_project_config_str(toml_str.trim_start_matches('\u{feff}'));
                }
            }
        }
    }
    ProjectConfig::default()
}

/// Get project root directory (directory containing psudo.toml)
pub fn get_project_root() -> Option<PathBuf> {
    find_project_config().map(|p| p.parent().unwrap().to_path_buf())
}

/// Resolve sandbox root relative to project root
pub fn resolve_sandbox_root(config: &ProjectConfig) -> PathBuf {
    if let Some(project_root) = get_project_root() {
        project_root.join(&config.sandbox.root)
    } else {
        // Fallback to current dir + sandbox root
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&config.sandbox.root)
    }
}

/// Check if a path is allowed for external read
pub fn is_external_read_allowed(config: &ProjectConfig, path: &Path) -> bool {
    if !config.sandbox.allow_external_read {
        return false;
    }

    let path_str = path.to_string_lossy();

    // Check deny list first
    for deny in &config.sandbox.deny_external_reads {
        if path_str.starts_with(deny) {
            return false;
        }
    }

    // Check allow list
    for allow in &config.sandbox.allowed_external_reads {
        if path_str.starts_with(allow) {
            return true;
        }
    }

    false
}

/// Generate default psudo.toml content
pub fn default_config_toml() -> String {
    toml::to_string_pretty(&ProjectConfig::default()).unwrap_or_default()
}

/// Parse a `psudo.toml` string into `ProjectConfig` (falling back to default on error)
pub fn parse_project_config_str(toml_str: &str) -> ProjectConfig {
    toml::from_str(toml_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_psudo_toml_with_custom_addons() {
        let raw = r##"
[sandbox]
root = ".pseudo/sandbox"
max_file_size = 2048

[addons.tokio_worker]
builtins = ["async_fetch", "async_Broadcast"]
cargo_deps = ['tokio = { version = "1", features = ["full"] }']
rust_main_attr = "#[tokio::main]"
rust_main_async = true
rust_file = ".pseudo/addons/tokio_worker.rs"
daemon_hook = "async_fetch"
"##;
        let cfg = parse_project_config_str(raw);
        assert_eq!(cfg.sandbox.max_file_size, 2048);
        let addon = cfg.addons.get("tokio_worker").expect("tokio_worker addon");
        assert_eq!(addon.builtins, vec!["async_fetch", "async_Broadcast"]);
        assert_eq!(addon.rust_main_attr.as_deref(), Some("#[tokio::main]"));
        assert!(addon.rust_main_async);
        assert_eq!(
            addon.rust_file.as_deref(),
            Some(".pseudo/addons/tokio_worker.rs")
        );
    }
}
