//! Button component - clickable buttons

use serde::{Deserialize, Serialize};
use super::Color;

/// Button variant/style
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Success,
    Danger,
    Warning,
    Info,
    Light,
    Dark,
    Outline,
    Link,
}

impl ButtonVariant {
    pub fn background_color(&self) -> Color {
        match self {
            ButtonVariant::Primary => Color::Hex("#0d6efd".to_string()),
            ButtonVariant::Secondary => Color::Hex("#6c757d".to_string()),
            ButtonVariant::Success => Color::Hex("#198754".to_string()),
            ButtonVariant::Danger => Color::Hex("#dc3545".to_string()),
            ButtonVariant::Warning => Color::Hex("#ffc107".to_string()),
            ButtonVariant::Info => Color::Hex("#0dcaf0".to_string()),
            ButtonVariant::Light => Color::Hex("#f8f9fa".to_string()),
            ButtonVariant::Dark => Color::Hex("#212529".to_string()),
            ButtonVariant::Outline => Color::Named("transparent".to_string()),
            ButtonVariant::Link => Color::Named("transparent".to_string()),
        }
    }

    pub fn text_color(&self) -> Color {
        match self {
            ButtonVariant::Light | ButtonVariant::Warning => Color::Hex("#212529".to_string()),
            ButtonVariant::Link => Color::Hex("#0d6efd".to_string()),
            ButtonVariant::Outline => Color::Hex("#0d6efd".to_string()),
            _ => Color::Named("white".to_string()),
        }
    }
}

/// Button component
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Button {
    /// Identifier (required for event handling)
    pub id: String,
    /// Button label text
    pub label: String,
    /// Button variant/style
    pub variant: ButtonVariant,
    /// Whether button is disabled
    pub disabled: bool,
    /// Icon (optional, name or emoji)
    pub icon: Option<String>,
    /// Width
    pub width: Option<f32>,
    /// Height
    pub height: Option<f32>,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            variant: ButtonVariant::Primary,
            disabled: false,
            icon: None,
            width: None,
            height: None,
        }
    }
}

impl Button {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Primary,
            disabled: false,
            icon: None,
            width: None,
            height: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}
