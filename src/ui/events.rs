//! Event handling for UI components

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Mouse event
#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub x: f32,
    pub y: f32,
    pub button: MouseButton,
    pub pressed: bool,
}

/// Keyboard event
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub key: String,
    pub code: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// Focus state manager
pub struct FocusManager {
    /// Currently focused component ID
    focused: Option<String>,
    /// Order of focusable components
    focus_order: Vec<String>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            focused: None,
            focus_order: Vec::new(),
        }
    }

    /// Set the focus order
    pub fn set_focus_order(&mut self, order: Vec<String>) {
        self.focus_order = order;
    }

    /// Get currently focused component
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// Focus a specific component
    pub fn focus(&mut self, id: &str) -> bool {
        if self.focus_order.contains(&id.to_string()) {
            self.focused = Some(id.to_string());
            true
        } else {
            false
        }
    }

    /// Clear focus
    pub fn blur(&mut self) {
        self.focused = None;
    }

    /// Move focus to next component
    pub fn focus_next(&mut self) -> Option<&str> {
        if self.focus_order.is_empty() {
            return None;
        }

        let next_idx = match &self.focused {
            Some(current) => {
                if let Some(idx) = self.focus_order.iter().position(|id| id == current) {
                    (idx + 1) % self.focus_order.len()
                } else {
                    0
                }
            }
            None => 0,
        };

        self.focused = Some(self.focus_order[next_idx].clone());
        self.focused.as_deref()
    }

    /// Move focus to previous component
    pub fn focus_prev(&mut self) -> Option<&str> {
        if self.focus_order.is_empty() {
            return None;
        }

        let prev_idx = match &self.focused {
            Some(current) => {
                if let Some(idx) = self.focus_order.iter().position(|id| id == current) {
                    if idx == 0 {
                        self.focus_order.len() - 1
                    } else {
                        idx - 1
                    }
                } else {
                    0
                }
            }
            None => self.focus_order.len() - 1,
        };

        self.focused = Some(self.focus_order[prev_idx].clone());
        self.focused.as_deref()
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Input state for text inputs
#[derive(Debug, Clone, Default)]
pub struct InputState {
    /// Current text value
    pub value: String,
    /// Cursor position
    pub cursor: usize,
    /// Selection start (if any)
    pub selection_start: Option<usize>,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_value(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.len();
        Self {
            value,
            cursor,
            selection_start: None,
        }
    }

    /// Insert text at cursor
    pub fn insert(&mut self, text: &str) {
        self.value.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.selection_start = None;
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
    }

    /// Delete character at cursor
    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }
}

/// UI event manager
pub struct EventManager {
    /// Focus manager
    pub focus: FocusManager,
    /// Input states by component ID
    pub input_states: HashMap<String, InputState>,
    /// Hover state (component ID under mouse)
    pub hovered: Option<String>,
    /// Pressed state (component being clicked)
    pub pressed: Option<String>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            focus: FocusManager::new(),
            input_states: HashMap::new(),
            hovered: None,
            pressed: None,
        }
    }

    /// Get or create input state for a component
    pub fn get_input_state(&mut self, id: &str) -> &mut InputState {
        self.input_states
            .entry(id.to_string())
            .or_insert_with(InputState::new)
    }

    /// Set input value
    pub fn set_input_value(&mut self, id: &str, value: impl Into<String>) {
        let state = self.get_input_state(id);
        state.value = value.into();
        state.cursor = state.value.len();
    }

    /// Handle mouse move
    pub fn on_mouse_move(&mut self, _x: f32, _y: f32) {
        // Hit testing would go here
    }

    /// Handle mouse button
    pub fn on_mouse_button(&mut self, _event: MouseEvent) {
        // Click handling would go here
    }

    /// Handle key press
    pub fn on_key(&mut self, event: KeyboardEvent) {
        // Tab key for focus navigation
        if event.key == "Tab" {
            if event.shift {
                self.focus.focus_prev();
            } else {
                self.focus.focus_next();
            }
            return;
        }

        // Handle input for focused component
        if let Some(focused_id) = self.focus.focused() {
            if let Some(state) = self.input_states.get_mut(focused_id) {
                match event.key.as_str() {
                    "Backspace" => state.backspace(),
                    "Delete" => state.delete(),
                    "ArrowLeft" => state.move_left(),
                    "ArrowRight" => state.move_right(),
                    "Home" => state.move_home(),
                    "End" => state.move_end(),
                    key if key.len() == 1 => state.insert(key),
                    _ => {}
                }
            }
        }
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}
