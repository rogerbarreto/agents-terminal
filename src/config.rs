//! Configuration module for Pixel Terminal

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Number of columns (characters)
    pub columns: u16,
    /// Number of rows (characters)
    pub rows: u16,
    /// Font size in pixels
    pub font_size: u16,
    /// Font family name
    pub font_family: String,
    /// Shell command to execute
    pub shell: String,
    /// Background color (RGBA)
    pub background_color: [f32; 4],
    /// Foreground color (RGBA)
    pub foreground_color: [f32; 4],
    /// Cursor color (RGBA)
    pub cursor_color: [f32; 4],
    /// Enable pixel canvas features
    pub enable_pixel_canvas: bool,
    /// Scrollback buffer size (lines)
    pub scrollback_lines: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            columns: 120,
            rows: 30,
            font_size: 14,
            font_family: "Consolas".to_string(),
            shell: default_shell(),
            background_color: [0.1, 0.1, 0.12, 1.0],
            foreground_color: [0.9, 0.9, 0.9, 1.0],
            cursor_color: [0.9, 0.9, 0.9, 0.8],
            enable_pixel_canvas: true,
            scrollback_lines: 10000,
        }
    }
}

impl Config {
    /// Load configuration from file or return defaults
    pub fn load_or_default() -> Self {
        if let Some(config_path) = Self::config_path() {
            if config_path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&config_path) {
                    if let Ok(config) = toml::from_str(&contents) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    /// Save configuration to file
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(config_path) = Self::config_path() {
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let contents = toml::to_string_pretty(self)?;
            std::fs::write(config_path, contents)?;
        }
        Ok(())
    }

    /// Get the configuration file path
    fn config_path() -> Option<PathBuf> {
        dirs_next().map(|p| p.join("pixel-terminal").join("config.toml"))
    }
}

/// Get the default shell for the current platform
fn default_shell() -> String {
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
}

/// Get the user's config directory
fn dirs_next() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config"))
    }
}

/// 16-color ANSI palette
pub const ANSI_COLORS: [[f32; 3]; 16] = [
    [0.0, 0.0, 0.0],       // 0: Black
    [0.8, 0.2, 0.2],       // 1: Red
    [0.2, 0.8, 0.2],       // 2: Green
    [0.8, 0.8, 0.2],       // 3: Yellow
    [0.2, 0.2, 0.8],       // 4: Blue
    [0.8, 0.2, 0.8],       // 5: Magenta
    [0.2, 0.8, 0.8],       // 6: Cyan
    [0.75, 0.75, 0.75],    // 7: White
    [0.5, 0.5, 0.5],       // 8: Bright Black
    [1.0, 0.3, 0.3],       // 9: Bright Red
    [0.3, 1.0, 0.3],       // 10: Bright Green
    [1.0, 1.0, 0.3],       // 11: Bright Yellow
    [0.3, 0.3, 1.0],       // 12: Bright Blue
    [1.0, 0.3, 1.0],       // 13: Bright Magenta
    [0.3, 1.0, 1.0],       // 14: Bright Cyan
    [1.0, 1.0, 1.0],       // 15: Bright White
];
