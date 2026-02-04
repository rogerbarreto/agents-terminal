//! Component Tree - manages the UI component hierarchy

use std::collections::HashMap;
use log::debug;

use super::components::Component;
use super::layout::{LayoutEngine, ComputedLayout};
use super::events::EventManager;
use super::protocol::{PtuiProtocol, PtuiMessage, PtuiEvent};

/// Manages the component tree and rendering
pub struct ComponentTree {
    /// Root components (by ID or index)
    roots: Vec<Component>,
    /// Layout engine
    layout: LayoutEngine,
    /// Event manager
    events: EventManager,
    /// Protocol handler
    protocol: PtuiProtocol,
    /// Whether layout needs recomputation
    dirty: bool,
    /// Available width for layout
    width: f32,
    /// Available height for layout
    height: f32,
}

impl ComponentTree {
    pub fn new() -> Self {
        Self {
            roots: Vec::new(),
            layout: LayoutEngine::new(),
            events: EventManager::new(),
            protocol: PtuiProtocol::new(),
            dirty: true,
            width: 800.0,
            height: 600.0,
        }
    }

    /// Set available size for layout
    pub fn set_size(&mut self, width: f32, height: f32) {
        if (self.width - width).abs() > 0.1 || (self.height - height).abs() > 0.1 {
            self.width = width;
            self.height = height;
            self.dirty = true;
        }
    }

    /// Process a PTUI message
    pub fn process_message(&mut self, data: &str) -> Option<String> {
        match self.protocol.parse(data) {
            Ok(PtuiMessage::Query(_)) => {
                // Return capability response
                Some(format!(
                    "\x1b]1337;PTUI={}\x07",
                    self.protocol.capability_response()
                ))
            }
            Ok(PtuiMessage::Component(component)) => {
                self.add_component(component);
                None
            }
            Ok(PtuiMessage::Clear(clear)) => {
                match clear.clear {
                    super::protocol::ClearTarget::All(_) => self.clear(),
                    super::protocol::ClearTarget::ById(id) => self.remove_component(&id),
                }
                None
            }
            Ok(PtuiMessage::Update(update)) => {
                // TODO: Update component by ID
                debug!("Update component {}: {:?}", update.update, update.props);
                None
            }
            Ok(PtuiMessage::Remove(remove)) => {
                self.remove_component(&remove.remove);
                None
            }
            Err(e) => {
                log::warn!("PTUI parse error: {}", e);
                None
            }
        }
    }

    /// Add a component to the tree
    pub fn add_component(&mut self, component: Component) {
        // Collect focusable component IDs
        self.collect_focusable(&component);
        
        self.roots.push(component);
        self.dirty = true;
    }

    /// Collect focusable component IDs
    fn collect_focusable(&mut self, component: &Component) {
        match component {
            Component::Input(i) => {
                let mut order = Vec::new();
                // Add to focus order
                order.push(i.id.clone());
                self.events.focus.set_focus_order(order);
            }
            Component::Button(b) => {
                // Buttons are focusable too
            }
            _ => {}
        }

        // Recurse into children
        for child in component.children() {
            self.collect_focusable(child);
        }
    }

    /// Remove a component by ID
    pub fn remove_component(&mut self, id: &str) {
        self.roots.retain(|c| c.id() != Some(id));
        self.dirty = true;
    }

    /// Clear all components
    pub fn clear(&mut self) {
        self.roots.clear();
        self.dirty = true;
    }

    /// Check if there are any components
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// Get number of root components
    pub fn len(&self) -> usize {
        self.roots.len()
    }

    /// Recompute layout if needed
    pub fn compute_layout(&mut self) {
        if !self.dirty || self.roots.is_empty() {
            return;
        }

        // Wrap roots in a container for layout
        let wrapper = Component::Container(super::components::Container {
            children: self.roots.clone(),
            direction: super::components::FlexDirection::Column,
            gap: 8.0,
            ..Default::default()
        });

        self.layout.compute(&wrapper, self.width, self.height);
        self.dirty = false;
    }

    /// Get layout for a component path
    pub fn get_layout(&self, path: &str) -> Option<ComputedLayout> {
        self.layout.get_layout(path)
    }

    /// Get all computed layouts
    pub fn layouts(&self) -> &HashMap<String, ComputedLayout> {
        self.layout.layouts()
    }

    /// Get root components for rendering
    pub fn roots(&self) -> &[Component] {
        &self.roots
    }

    /// Get event manager
    pub fn events(&self) -> &EventManager {
        &self.events
    }

    /// Get mutable event manager
    pub fn events_mut(&mut self) -> &mut EventManager {
        &mut self.events
    }

    /// Handle a click at position
    pub fn handle_click(&mut self, x: f32, y: f32) -> Option<PtuiEvent> {
        // Find component at position and collect result first
        let mut result: Option<(String, bool)> = None; // (id, is_input)
        
        for (path, layout) in self.layout.layouts() {
            if x >= layout.x && x <= layout.x + layout.width
                && y >= layout.y && y <= layout.y + layout.height
            {
                // Found a hit - check if it's interactive
                if let Some(component) = self.find_component_by_path(path) {
                    match component {
                        Component::Button(b) if !b.disabled => {
                            result = Some((b.id.clone(), false));
                            break;
                        }
                        Component::Input(i) if !i.disabled => {
                            result = Some((i.id.clone(), true));
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Now handle the result
        match result {
            Some((id, true)) => {
                self.events.focus.focus(&id);
                Some(PtuiProtocol::focus_event(&id))
            }
            Some((id, false)) => {
                Some(PtuiProtocol::click_event(&id, None))
            }
            None => None,
        }
    }

    /// Find component by path
    fn find_component_by_path(&self, path: &str) -> Option<&Component> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() || parts[0] != "root" {
            return None;
        }

        // Skip "root" and navigate
        let mut current: Option<&Component> = None;
        
        for (i, part) in parts.iter().enumerate().skip(1) {
            let idx: usize = part.parse().ok()?;
            
            if i == 1 {
                // First level - get from wrapper's children (which is self.roots)
                current = self.roots.get(idx);
            } else if let Some(comp) = current {
                current = comp.children().get(idx);
            } else {
                return None;
            }
        }

        current
    }
}

impl Default for ComponentTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_component() {
        let mut tree = ComponentTree::new();
        assert!(tree.is_empty());

        let text = r#"{"type": "text", "content": "Hello"}"#;
        tree.process_message(text);

        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut tree = ComponentTree::new();
        tree.process_message(r#"{"type": "text", "content": "Test"}"#);
        assert!(!tree.is_empty());

        tree.process_message(r#"{"clear": true}"#);
        assert!(tree.is_empty());
    }
}
