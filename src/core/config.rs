//! Configuration module for lazy-locker
//!
//! Manages user configuration including analyzer settings.
//! Configuration is stored in `~/.config/.lazy-locker/config.toml`

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Analyzer-specific settings
    pub analyzer: AnalyzerSettings,
}

/// Settings for the token security analyzer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnalyzerSettings {
    /// Whether to enable automatic analysis (can be disabled for performance)
    pub enabled: bool,

    /// Maximum time in milliseconds for analysis (0 = no limit)
    /// If analysis takes longer, it will be aborted
    pub timeout_ms: u64,

    /// Maximum number of files to scan (0 = no limit)
    pub max_files: usize,

    /// Minimum directory depth to enable analysis
    /// e.g., 3 means /home/user/project is OK, but /home/user is skipped
    pub min_path_depth: usize,

    /// Additional directories to ignore (on top of defaults)
    /// These are matched against directory names, not full paths
    pub ignore_dirs: Vec<String>,

    /// Directories that should never be analyzed automatically
    /// Matched against full path
    pub skip_paths: Vec<String>,

    /// File extensions to include (empty = use defaults)
    pub extensions: Vec<String>,

    /// Include hidden files in analysis
    pub include_hidden: bool,
}

impl Default for AnalyzerSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_ms: 500, // 500ms max for responsive UI
            max_files: 1000,
            min_path_depth: 4, // At least /home/user/project (4 components)
            ignore_dirs: vec![
                // Package managers
                "node_modules".into(),
                ".npm".into(),
                ".pnpm-store".into(),
                "bower_components".into(),
                // Python
                ".venv".into(),
                "venv".into(),
                "__pycache__".into(),
                ".pytest_cache".into(),
                ".mypy_cache".into(),
                ".tox".into(),
                "site-packages".into(),
                // Rust
                "target".into(),
                // Go
                "vendor".into(),
                // Java/Kotlin
                ".gradle".into(),
                ".m2".into(),
                // IDE/Editor
                ".idea".into(),
                ".vscode".into(),
                ".vs".into(),
                // Build
                "build".into(),
                "dist".into(),
                "out".into(),
                "_build".into(),
                // Cache
                ".cache".into(),
                ".parcel-cache".into(),
                ".next".into(),
                ".nuxt".into(),
                // Version control
                ".git".into(),
                ".svn".into(),
                ".hg".into(),
                // Dependencies
                "deps".into(),
                "_deps".into(),
                // Misc
                "coverage".into(),
                ".coverage".into(),
                "htmlcov".into(),
                ".eggs".into(),
                "*.egg-info".into(),
            ],
            skip_paths: vec![],
            extensions: vec![], // Empty = use defaults from token-analyzer
            include_hidden: false,
        }
    }
}

impl Config {
    /// Load configuration from the locker directory
    /// Creates default config if it doesn't exist
    pub fn load(locker_dir: &Path) -> Result<Self> {
        let config_path = locker_dir.join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content).unwrap_or_else(|e| {
                eprintln!(
                    "Warning: Failed to parse config.toml: {}. Using defaults.",
                    e
                );
                Config::default()
            });
            Ok(config)
        } else {
            // Create default config file for user reference
            let config = Config::default();
            config.save(locker_dir)?;
            Ok(config)
        }
    }

    /// Save configuration to the locker directory
    pub fn save(&self, locker_dir: &Path) -> Result<()> {
        let config_path = locker_dir.join("config.toml");
        let content = Self::generate_config_with_comments(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Generate TOML content with helpful comments
    fn generate_config_with_comments(config: &Config) -> Result<String> {
        let toml_content = toml::to_string_pretty(config)?;

        let header = r#"# Lazy-Locker Configuration
# 
# This file controls the behavior of lazy-locker.
# Edit these settings to customize the token security analyzer.
#
# Documentation: https://github.com/WillIsback/lazy-locker

"#;

        let analyzer_comment = r#"
# Token Security Analyzer Settings
# The analyzer scans your codebase for exposed secrets.
# Customize these settings if analysis is slow or you want to exclude specific directories.
#
# Tips:
#   - Set enabled = false to disable automatic analysis
#   - Add large directories to ignore_dirs to speed up analysis
#   - Decrease max_files if analysis is still slow

"#;

        // Insert comments at appropriate places
        let content = format!("{}{}{}", header, analyzer_comment, toml_content);

        Ok(content)
    }

    /// Get the locker directory path
    pub fn get_locker_dir() -> Result<PathBuf> {
        let base_dirs = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Unable to determine user directories"))?;

        #[cfg(unix)]
        let sub_dir = ".lazy-locker";
        #[cfg(not(unix))]
        let sub_dir = "lazy-locker";

        let locker_dir = base_dirs.config_dir().join(sub_dir);
        Ok(locker_dir)
    }
}

impl AnalyzerSettings {
    /// Check if a path should be analyzed based on settings
    pub fn should_analyze(&self, path: &Path) -> bool {
        if !self.enabled {
            return false;
        }

        // Check minimum path depth
        let depth = path.components().count();
        if depth < self.min_path_depth {
            return false;
        }

        // Check against skip_paths (full path match)
        let path_str = path.to_string_lossy();
        for skip in &self.skip_paths {
            if path_str.starts_with(skip) || path_str.ends_with(skip) {
                return false;
            }
        }

        // Check if we're in home directory directly
        if let Ok(home) = std::env::var("HOME")
            && path == Path::new(&home)
        {
            return false;
        }

        true
    }

    /// Convert settings to token-analyzer's AnalyzerConfig
    pub fn to_analyzer_config(&self) -> token_analyzer::AnalyzerConfig {
        let mut config = token_analyzer::AnalyzerConfig::fast();

        config.timeout_ms = self.timeout_ms;
        config.max_files = self.max_files;
        config.include_hidden = self.include_hidden;

        // Merge ignore_dirs with defaults
        config.ignore_dirs.extend(self.ignore_dirs.clone());

        // Set extensions if specified
        if !self.extensions.is_empty() {
            config.extensions = self.extensions.clone();
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.analyzer.enabled);
        assert_eq!(config.analyzer.timeout_ms, 500);
        assert_eq!(config.analyzer.max_files, 1000);
        assert!(config.analyzer.ignore_dirs.contains(&"node_modules".into()));
        assert!(config.analyzer.ignore_dirs.contains(&".venv".into()));
    }

    #[test]
    fn test_config_save_load() {
        let dir = TempDir::new().unwrap();
        let config = Config::default();

        config.save(dir.path()).unwrap();
        let loaded = Config::load(dir.path()).unwrap();

        assert_eq!(loaded.analyzer.enabled, config.analyzer.enabled);
        assert_eq!(loaded.analyzer.timeout_ms, config.analyzer.timeout_ms);
    }

    #[test]
    fn test_should_analyze_depth() {
        let settings = AnalyzerSettings::default();

        // Too shallow - should not analyze
        assert!(!settings.should_analyze(Path::new("/home")));
        assert!(!settings.should_analyze(Path::new("/home/user")));

        // Deep enough - should analyze
        assert!(settings.should_analyze(Path::new("/home/user/project")));
        assert!(settings.should_analyze(Path::new("/home/user/project/src")));
    }

    #[test]
    fn test_should_analyze_disabled() {
        let settings = AnalyzerSettings {
            enabled: false,
            ..Default::default()
        };

        assert!(!settings.should_analyze(Path::new("/home/user/project")));
    }

    #[test]
    fn test_to_analyzer_config() {
        let settings = AnalyzerSettings::default();
        let config = settings.to_analyzer_config();

        assert_eq!(config.timeout_ms, 500);
        assert_eq!(config.max_files, 1000);
    }
}
