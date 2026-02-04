//! Text component - rich text rendering

use serde::{Deserialize, Serialize};
use super::{Color, TextAlign};

/// Rich text component
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Text {
    /// Optional identifier
    pub id: Option<String>,
    /// Text content
    pub content: String,
    /// Font size in pixels
    pub size: Option<f32>,
    /// Text color
    pub color: Option<Color>,
    /// Bold text
    pub bold: bool,
    /// Italic text
    pub italic: bool,
    /// Underline
    pub underline: bool,
    /// Strikethrough
    pub strikethrough: bool,
    /// Font family
    pub font: Option<String>,
    /// Text alignment
    pub align: TextAlign,
    /// Line height multiplier
    pub line_height: f32,
    /// Letter spacing
    pub letter_spacing: f32,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            id: None,
            content: String::new(),
            size: None,
            color: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            font: None,
            align: TextAlign::Left,
            line_height: 1.2,
            letter_spacing: 0.0,
        }
    }
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }
}
