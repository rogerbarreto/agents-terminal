//! PTUI Protocol Parser
//!
//! Parses PTUI JSON payloads from OSC escape sequences

use serde::{Deserialize, Serialize};
use serde_json;
use log::{debug, warn};

use super::components::Component;

/// PTUI Protocol handler
pub struct PtuiProtocol {
    /// Protocol version
    pub version: &'static str,
    /// Supported component types
    pub supported_components: Vec<&'static str>,
}

/// PTUI message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PtuiMessage {
    /// Query for capabilities
    Query(QueryMessage),
    /// Component to render
    Component(Component),
    /// Clear components
    Clear(ClearMessage),
    /// Update existing component
    Update(UpdateMessage),
    /// Remove component
    Remove(RemoveMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMessage {
    pub query: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearMessage {
    pub clear: ClearTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClearTarget {
    All(bool),
    ById(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMessage {
    pub update: String, // Component ID
    pub props: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveMessage {
    pub remove: String, // Component ID
}

/// Response to capability query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResponse {
    pub version: String,
    pub components: Vec<String>,
    pub features: Vec<String>,
}

/// Event sent back to the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtuiEvent {
    pub event: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl PtuiProtocol {
    pub fn new() -> Self {
        Self {
            version: "1.0",
            supported_components: vec![
                "container", "row", "col", "text", "image",
                "button", "input", "progress", "table",
            ],
        }
    }

    /// Parse a PTUI JSON payload
    pub fn parse(&self, data: &str) -> Result<PtuiMessage, String> {
        // Handle query shorthand
        if data.trim() == "query" {
            return Ok(PtuiMessage::Query(QueryMessage { query: true }));
        }

        // Parse JSON
        serde_json::from_str(data).map_err(|e| {
            warn!("Failed to parse PTUI message: {}", e);
            format!("Invalid PTUI JSON: {}", e)
        })
    }

    /// Generate capability response
    pub fn capability_response(&self) -> String {
        let response = CapabilityResponse {
            version: self.version.to_string(),
            components: self.supported_components.iter().map(|s| s.to_string()).collect(),
            features: vec![
                "flexbox".to_string(),
                "images".to_string(),
                "events".to_string(),
                "12-column-grid".to_string(),
            ],
        };
        
        serde_json::to_string(&response).unwrap_or_default()
    }

    /// Create a click event
    pub fn click_event(id: &str, data: Option<serde_json::Value>) -> PtuiEvent {
        PtuiEvent {
            event: "click".to_string(),
            id: id.to_string(),
            data,
        }
    }

    /// Create an input event
    pub fn input_event(id: &str, value: &str) -> PtuiEvent {
        PtuiEvent {
            event: "input".to_string(),
            id: id.to_string(),
            data: Some(serde_json::json!({ "value": value })),
        }
    }

    /// Create a focus event
    pub fn focus_event(id: &str) -> PtuiEvent {
        PtuiEvent {
            event: "focus".to_string(),
            id: id.to_string(),
            data: None,
        }
    }

    /// Create a blur event
    pub fn blur_event(id: &str) -> PtuiEvent {
        PtuiEvent {
            event: "blur".to_string(),
            id: id.to_string(),
            data: None,
        }
    }

    /// Serialize event for sending back to PTY
    pub fn serialize_event(event: &PtuiEvent) -> String {
        format!("\x1b]1337;PTUI={}\x07", serde_json::to_string(event).unwrap_or_default())
    }
}

impl Default for PtuiProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query() {
        let protocol = PtuiProtocol::new();
        let result = protocol.parse("query");
        assert!(matches!(result, Ok(PtuiMessage::Query(_))));
    }

    #[test]
    fn test_parse_text_component() {
        let protocol = PtuiProtocol::new();
        let json = r#"{"type": "text", "content": "Hello World"}"#;
        let result = protocol.parse(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_row_with_columns() {
        let protocol = PtuiProtocol::new();
        let json = r#"{
            "type": "row",
            "children": [
                {"type": "col", "span": 6, "children": [{"type": "text", "content": "Left"}]},
                {"type": "col", "span": 6, "children": [{"type": "text", "content": "Right"}]}
            ]
        }"#;
        let result = protocol.parse(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capability_response() {
        let protocol = PtuiProtocol::new();
        let response = protocol.capability_response();
        assert!(response.contains("version"));
        assert!(response.contains("components"));
    }
}
