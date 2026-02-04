//! Component type definitions for PTUI
//!
//! Defines all the UI components that can be rendered in Pixel Terminal.

mod container;
mod text;
mod image;
mod button;
mod input;
mod progress;
mod table;

pub use container::Container;
pub use text::Text;
pub use image::Image;
pub use button::{Button, ButtonVariant};
pub use input::Input;
pub use progress::{Progress, ProgressVariant};
pub use table::Table;

use serde::{Deserialize, Serialize};

/// Unique identifier for a component
pub type ComponentId = String;

/// A UI component that can be rendered
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Component {
    /// Flex container for layout
    Container(Container),
    /// Row layout (horizontal flex)
    Row(Container),
    /// Column in a grid (with span)
    Col(Container),
    /// Rich text
    Text(Text),
    /// Inline image
    Image(Image),
    /// Clickable button
    Button(Button),
    /// Text input field
    Input(Input),
    /// Progress bar
    Progress(Progress),
    /// Data table
    Table(Table),
}

impl Component {
    /// Get the component's ID if it has one
    pub fn id(&self) -> Option<&str> {
        match self {
            Component::Container(c) | Component::Row(c) | Component::Col(c) => c.id.as_deref(),
            Component::Text(t) => t.id.as_deref(),
            Component::Image(i) => i.id.as_deref(),
            Component::Button(b) => Some(&b.id),
            Component::Input(i) => Some(&i.id),
            Component::Progress(p) => p.id.as_deref(),
            Component::Table(t) => t.id.as_deref(),
        }
    }

    /// Get children components (for containers)
    pub fn children(&self) -> &[Component] {
        match self {
            Component::Container(c) | Component::Row(c) | Component::Col(c) => &c.children,
            _ => &[],
        }
    }

    /// Get mutable children components
    pub fn children_mut(&mut self) -> &mut Vec<Component> {
        match self {
            Component::Container(c) | Component::Row(c) | Component::Col(c) => &mut c.children,
            _ => panic!("Component does not have children"),
        }
    }
}

/// Common style properties shared by components
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    /// Width (pixels, percentage, or "auto")
    pub width: Option<Dimension>,
    /// Height
    pub height: Option<Dimension>,
    /// Minimum width
    pub min_width: Option<Dimension>,
    /// Minimum height
    pub min_height: Option<Dimension>,
    /// Maximum width
    pub max_width: Option<Dimension>,
    /// Maximum height
    pub max_height: Option<Dimension>,
    /// Padding (all sides or [top, right, bottom, left])
    pub padding: Option<Spacing>,
    /// Margin
    pub margin: Option<Spacing>,
    /// Background color
    pub background: Option<Color>,
    /// Border
    pub border: Option<Border>,
    /// Border radius
    pub border_radius: Option<f32>,
}

/// Dimension value (pixels, percentage, or auto)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dimension {
    Pixels(f32),
    Percent(String), // "50%"
    Auto,
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension::Auto
    }
}

/// Spacing for padding/margin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Spacing {
    All(f32),
    Vertical { vertical: f32, horizontal: f32 },
    Individual { top: f32, right: f32, bottom: f32, left: f32 },
}

impl Default for Spacing {
    fn default() -> Self {
        Spacing::All(0.0)
    }
}

/// Color value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Color {
    /// Hex color string "#RRGGBB" or "#RRGGBBAA"
    Hex(String),
    /// RGB values
    Rgb { r: u8, g: u8, b: u8 },
    /// RGBA values
    Rgba { r: u8, g: u8, b: u8, a: u8 },
    /// Named color
    Named(String),
}

impl Color {
    pub fn to_rgba(&self) -> [f32; 4] {
        match self {
            Color::Hex(s) => {
                let s = s.trim_start_matches('#');
                if s.len() >= 6 {
                    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255) as f32 / 255.0;
                    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(255) as f32 / 255.0;
                    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255) as f32 / 255.0;
                    let a = if s.len() >= 8 {
                        u8::from_str_radix(&s[6..8], 16).unwrap_or(255) as f32 / 255.0
                    } else {
                        1.0
                    };
                    [r, g, b, a]
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                }
            }
            Color::Rgb { r, g, b } => [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, 1.0],
            Color::Rgba { r, g, b, a } => {
                [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, *a as f32 / 255.0]
            }
            Color::Named(name) => {
                // Basic named colors
                match name.to_lowercase().as_str() {
                    "white" => [1.0, 1.0, 1.0, 1.0],
                    "black" => [0.0, 0.0, 0.0, 1.0],
                    "red" => [1.0, 0.0, 0.0, 1.0],
                    "green" => [0.0, 1.0, 0.0, 1.0],
                    "blue" => [0.0, 0.0, 1.0, 1.0],
                    "yellow" => [1.0, 1.0, 0.0, 1.0],
                    "cyan" => [0.0, 1.0, 1.0, 1.0],
                    "magenta" => [1.0, 0.0, 1.0, 1.0],
                    "gray" | "grey" => [0.5, 0.5, 0.5, 1.0],
                    "transparent" => [0.0, 0.0, 0.0, 0.0],
                    _ => [1.0, 1.0, 1.0, 1.0],
                }
            }
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::Named("transparent".to_string())
    }
}

/// Border definition
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Border {
    pub width: f32,
    pub color: Color,
    pub style: BorderStyle,
}

/// Border style
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    None,
}

/// Flex direction for containers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

/// Alignment for flex items
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Text alignment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}
