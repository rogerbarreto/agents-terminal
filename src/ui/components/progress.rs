//! Progress bar component

use serde::{Deserialize, Serialize};
use super::Color;

/// Progress bar variant
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProgressVariant {
    #[default]
    Default,
    Primary,
    Success,
    Warning,
    Danger,
    Info,
    Striped,
    Animated,
}

impl ProgressVariant {
    pub fn color(&self) -> Color {
        match self {
            ProgressVariant::Default | ProgressVariant::Primary => Color::Hex("#0d6efd".to_string()),
            ProgressVariant::Success => Color::Hex("#198754".to_string()),
            ProgressVariant::Warning => Color::Hex("#ffc107".to_string()),
            ProgressVariant::Danger => Color::Hex("#dc3545".to_string()),
            ProgressVariant::Info => Color::Hex("#0dcaf0".to_string()),
            ProgressVariant::Striped => Color::Hex("#0d6efd".to_string()),
            ProgressVariant::Animated => Color::Hex("#0d6efd".to_string()),
        }
    }
}

/// Progress bar component
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    /// Optional identifier
    pub id: Option<String>,
    /// Current value (0-100 or custom max)
    pub value: f32,
    /// Maximum value
    pub max: f32,
    /// Progress variant/color
    pub variant: ProgressVariant,
    /// Custom color override
    pub color: Option<Color>,
    /// Show percentage label
    pub show_label: bool,
    /// Custom label text
    pub label: Option<String>,
    /// Height in pixels
    pub height: f32,
    /// Whether to animate
    pub animated: bool,
    /// Striped style
    pub striped: bool,
    /// Background color
    pub background: Color,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            id: None,
            value: 0.0,
            max: 100.0,
            variant: ProgressVariant::Default,
            color: None,
            show_label: false,
            label: None,
            height: 16.0,
            animated: false,
            striped: false,
            background: Color::Hex("#333".to_string()),
        }
    }
}

impl Progress {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            ..Default::default()
        }
    }

    pub fn percentage(&self) -> f32 {
        (self.value / self.max * 100.0).min(100.0).max(0.0)
    }

    pub fn variant(mut self, variant: ProgressVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn with_label(mut self) -> Self {
        self.show_label = true;
        self
    }
}
