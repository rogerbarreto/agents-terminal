//! Image component - inline images

use serde::{Deserialize, Serialize};
use super::Dimension;

/// Image fit mode
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFit {
    /// Scale to fill, may crop
    Cover,
    /// Scale to fit, may have gaps
    #[default]
    Contain,
    /// Stretch to fill exactly
    Fill,
    /// No scaling
    None,
}

/// Image component
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Image {
    /// Optional identifier
    pub id: Option<String>,
    /// Image source (base64 data URL or path)
    pub src: String,
    /// Width
    pub width: Option<Dimension>,
    /// Height
    pub height: Option<Dimension>,
    /// Fit mode
    pub fit: ImageFit,
    /// Alt text for accessibility
    pub alt: Option<String>,
    /// Border radius
    pub border_radius: f32,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            id: None,
            src: String::new(),
            width: None,
            height: None,
            fit: ImageFit::Contain,
            alt: None,
            border_radius: 0.0,
        }
    }
}

impl Image {
    pub fn new(src: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            ..Default::default()
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(Dimension::Pixels(width));
        self.height = Some(Dimension::Pixels(height));
        self
    }

    /// Check if source is a data URL
    pub fn is_data_url(&self) -> bool {
        self.src.starts_with("data:")
    }

    /// Extract base64 data from data URL
    pub fn get_base64_data(&self) -> Option<&str> {
        if self.is_data_url() {
            self.src.split(',').nth(1)
        } else {
            None
        }
    }

    /// Get MIME type from data URL
    pub fn get_mime_type(&self) -> Option<&str> {
        if self.is_data_url() {
            let prefix = self.src.strip_prefix("data:")?;
            prefix.split(';').next()
        } else {
            None
        }
    }
}
