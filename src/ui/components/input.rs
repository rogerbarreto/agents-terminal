//! Input component - text input fields

use serde::{Deserialize, Serialize};
use super::Color;

/// Input type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    #[default]
    Text,
    Password,
    Number,
    Email,
    Search,
}

/// Input component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    /// Identifier (required for event handling)
    pub id: String,
    /// Input type
    #[serde(default, rename = "inputType")]
    pub input_type: InputType,
    /// Placeholder text
    #[serde(default)]
    pub placeholder: String,
    /// Current value
    #[serde(default)]
    pub value: String,
    /// Label text
    pub label: Option<String>,
    /// Whether input is disabled
    #[serde(default)]
    pub disabled: bool,
    /// Whether input is readonly
    #[serde(default)]
    pub readonly: bool,
    /// Maximum length
    pub maxlength: Option<usize>,
    /// Width in pixels
    pub width: Option<f32>,
    /// Background color
    #[serde(default)]
    pub background: Color,
    /// Text color
    #[serde(default = "default_text_color")]
    pub color: Color,
    /// Border color
    #[serde(default = "default_border_color")]
    pub border_color: Color,
}

fn default_text_color() -> Color {
    Color::Named("white".to_string())
}

fn default_border_color() -> Color {
    Color::Hex("#444".to_string())
}

impl Input {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            input_type: InputType::Text,
            placeholder: String::new(),
            value: String::new(),
            label: None,
            disabled: false,
            readonly: false,
            maxlength: None,
            width: None,
            background: Color::Hex("#1e1e1e".to_string()),
            color: default_text_color(),
            border_color: default_border_color(),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn password(mut self) -> Self {
        self.input_type = InputType::Password;
        self
    }
}
