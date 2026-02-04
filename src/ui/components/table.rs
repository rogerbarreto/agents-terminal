//! Table component - data tables

use serde::{Deserialize, Serialize};
use super::Color;

/// Table column definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    /// Column header text
    pub header: String,
    /// Column key (for data lookup)
    pub key: String,
    /// Column width (pixels or percentage)
    pub width: Option<String>,
    /// Text alignment
    #[serde(default)]
    pub align: TableAlign,
}

/// Table alignment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Table row data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TableRow {
    /// Object with key-value pairs
    Object(serde_json::Map<String, serde_json::Value>),
    /// Array of values (in column order)
    Array(Vec<serde_json::Value>),
}

/// Table component
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Table {
    /// Optional identifier
    pub id: Option<String>,
    /// Column definitions
    pub columns: Vec<TableColumn>,
    /// Row data
    pub rows: Vec<TableRow>,
    /// Striped rows
    pub striped: bool,
    /// Bordered cells
    pub bordered: bool,
    /// Hoverable rows
    pub hoverable: bool,
    /// Compact/dense mode
    pub compact: bool,
    /// Header background color
    pub header_background: Color,
    /// Row background color
    pub row_background: Color,
    /// Alternate row background
    pub alt_row_background: Color,
    /// Border color
    pub border_color: Color,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            id: None,
            columns: Vec::new(),
            rows: Vec::new(),
            striped: true,
            bordered: true,
            hoverable: true,
            compact: false,
            header_background: Color::Hex("#2d2d2d".to_string()),
            row_background: Color::Hex("#1e1e1e".to_string()),
            alt_row_background: Color::Hex("#252525".to_string()),
            border_color: Color::Hex("#444".to_string()),
        }
    }
}

impl Table {
    pub fn new(columns: Vec<TableColumn>) -> Self {
        Self {
            columns,
            ..Default::default()
        }
    }

    pub fn with_rows(mut self, rows: Vec<TableRow>) -> Self {
        self.rows = rows;
        self
    }
}
