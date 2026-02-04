//! Container component - flex/grid layout container

use serde::{Deserialize, Serialize};
use super::{Component, Style, FlexDirection, Alignment, Color};

/// Container component for layout
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Container {
    /// Optional identifier
    pub id: Option<String>,
    /// Child components
    #[serde(default)]
    pub children: Vec<Component>,
    /// Flex direction
    pub direction: FlexDirection,
    /// Gap between children (pixels)
    pub gap: f32,
    /// Align items on cross axis
    pub align: Alignment,
    /// Justify content on main axis
    pub justify: Alignment,
    /// Whether to wrap children
    pub wrap: bool,
    /// Column span (for Col type, 1-12)
    pub span: Option<u8>,
    /// Style properties
    #[serde(flatten)]
    pub style: Style,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            id: None,
            children: Vec::new(),
            direction: FlexDirection::Row,
            gap: 0.0,
            align: Alignment::Start,
            justify: Alignment::Start,
            wrap: false,
            span: None,
            style: Style::default(),
        }
    }
}

impl Container {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn row() -> Self {
        Self {
            direction: FlexDirection::Row,
            ..Default::default()
        }
    }

    pub fn column() -> Self {
        Self {
            direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    pub fn with_span(span: u8) -> Self {
        Self {
            span: Some(span.min(12).max(1)),
            ..Default::default()
        }
    }
}
