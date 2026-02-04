//! PTUI - Pixel Terminal UI Component System
//!
//! This module provides a rich component-based UI system that works alongside
//! traditional terminal text rendering. Applications can opt-in to use
//! Bootstrap-like layout components via escape sequences.

pub mod components;
pub mod protocol;
pub mod layout;
pub mod events;
pub mod tree;

pub use components::*;
pub use protocol::PtuiProtocol;
pub use layout::LayoutEngine;
pub use tree::ComponentTree;
