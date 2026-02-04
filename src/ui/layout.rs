//! Layout Engine - Flexbox layout using Taffy
//!
//! Calculates positions and sizes for UI components

use taffy::prelude::*;
use std::collections::HashMap;
use log::debug;

use super::components::{Component, Container, Alignment, Spacing};
use super::components::Dimension as ComponentDimension;
use super::components::FlexDirection as ComponentFlexDirection;

/// Layout engine using Taffy for flexbox calculations
pub struct LayoutEngine {
    taffy: TaffyTree,
    /// Map from component path to taffy node
    node_map: HashMap<String, NodeId>,
    /// Calculated layouts
    layouts: HashMap<String, ComputedLayout>,
}

/// Computed layout for a component
#[derive(Debug, Clone, Copy)]
pub struct ComputedLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            node_map: HashMap::new(),
            layouts: HashMap::new(),
        }
    }

    /// Calculate layout for a component tree
    pub fn compute(&mut self, root: &Component, available_width: f32, available_height: f32) {
        self.node_map.clear();
        self.layouts.clear();

        // Build taffy tree from component tree
        if let Some(root_node) = self.build_node(root, "root") {
            // Compute layout
            self.taffy
                .compute_layout(
                    root_node,
                    Size {
                        width: AvailableSpace::Definite(available_width),
                        height: AvailableSpace::Definite(available_height),
                    },
                )
                .ok();

            // Extract computed layouts
            self.extract_layouts(root_node, "root", 0.0, 0.0);
        }
    }

    /// Build a taffy node from a component
    fn build_node(&mut self, component: &Component, path: &str) -> Option<NodeId> {
        let style = self.component_to_style(component);
        
        // Build children first
        let children: Vec<NodeId> = component
            .children()
            .iter()
            .enumerate()
            .filter_map(|(i, child)| {
                let child_path = format!("{}.{}", path, i);
                self.build_node(child, &child_path)
            })
            .collect();

        // Create node
        let node = if children.is_empty() {
            self.taffy.new_leaf(style).ok()?
        } else {
            self.taffy.new_with_children(style, &children).ok()?
        };

        self.node_map.insert(path.to_string(), node);
        Some(node)
    }

    /// Convert component to taffy style
    fn component_to_style(&self, component: &Component) -> Style {
        match component {
            Component::Container(c) | Component::Row(c) | Component::Col(c) => {
                self.container_style(c, matches!(component, Component::Row(_)))
            }
            Component::Text(_t) => Style {
                size: Size {
                    width: Dimension::Auto,
                    height: Dimension::Auto,
                },
                ..Default::default()
            },
            Component::Image(i) => {
                let width = match &i.width {
                    Some(ComponentDimension::Pixels(p)) => Dimension::Length(*p),
                    Some(ComponentDimension::Percent(s)) => {
                        if let Some(p) = s.strip_suffix('%').and_then(|n| n.parse::<f32>().ok()) {
                            Dimension::Percent(p / 100.0)
                        } else {
                            Dimension::Auto
                        }
                    }
                    _ => Dimension::Auto,
                };
                let height = match &i.height {
                    Some(ComponentDimension::Pixels(p)) => Dimension::Length(*p),
                    Some(ComponentDimension::Percent(s)) => {
                        if let Some(p) = s.strip_suffix('%').and_then(|n| n.parse::<f32>().ok()) {
                            Dimension::Percent(p / 100.0)
                        } else {
                            Dimension::Auto
                        }
                    }
                    _ => Dimension::Auto,
                };
                Style {
                    size: Size { width, height },
                    ..Default::default()
                }
            }
            Component::Button(b) => Style {
                size: Size {
                    width: b.width.map(Dimension::Length).unwrap_or(Dimension::Auto),
                    height: b.height.map(Dimension::Length).unwrap_or(Dimension::Length(36.0)),
                },
                padding: Rect {
                    left: LengthPercentage::Length(12.0),
                    right: LengthPercentage::Length(12.0),
                    top: LengthPercentage::Length(8.0),
                    bottom: LengthPercentage::Length(8.0),
                },
                ..Default::default()
            },
            Component::Input(i) => Style {
                size: Size {
                    width: i.width.map(Dimension::Length).unwrap_or(Dimension::Percent(1.0)),
                    height: Dimension::Length(36.0),
                },
                padding: Rect {
                    left: LengthPercentage::Length(8.0),
                    right: LengthPercentage::Length(8.0),
                    top: LengthPercentage::Length(8.0),
                    bottom: LengthPercentage::Length(8.0),
                },
                ..Default::default()
            },
            Component::Progress(p) => Style {
                size: Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Length(p.height),
                },
                ..Default::default()
            },
            Component::Table(_) => Style {
                size: Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Auto,
                },
                ..Default::default()
            },
        }
    }

    /// Convert container to taffy style
    fn container_style(&self, container: &Container, is_row: bool) -> Style {
        let direction = if is_row {
            FlexDirection::Row
        } else {
            match container.direction {
                ComponentFlexDirection::Row => FlexDirection::Row,
                ComponentFlexDirection::Column => FlexDirection::Column,
                ComponentFlexDirection::RowReverse => FlexDirection::RowReverse,
                ComponentFlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
            }
        };

        let justify_content = match container.justify {
            Alignment::Start => Some(JustifyContent::FlexStart),
            Alignment::Center => Some(JustifyContent::Center),
            Alignment::End => Some(JustifyContent::FlexEnd),
            Alignment::SpaceBetween => Some(JustifyContent::SpaceBetween),
            Alignment::SpaceAround => Some(JustifyContent::SpaceAround),
            Alignment::SpaceEvenly => Some(JustifyContent::SpaceEvenly),
            _ => Some(JustifyContent::FlexStart),
        };

        let align_items = match container.align {
            Alignment::Start => Some(AlignItems::FlexStart),
            Alignment::Center => Some(AlignItems::Center),
            Alignment::End => Some(AlignItems::FlexEnd),
            Alignment::Stretch => Some(AlignItems::Stretch),
            _ => Some(AlignItems::FlexStart),
        };

        // Handle column span (1-12 grid)
        let width = if let Some(span) = container.span {
            Dimension::Percent(span as f32 / 12.0)
        } else {
            self.dimension_to_taffy(&container.style.width)
        };

        let padding = self.spacing_to_rect(&container.style.padding);

        Style {
            display: Display::Flex,
            flex_direction: direction,
            justify_content,
            align_items,
            flex_wrap: if container.wrap { FlexWrap::Wrap } else { FlexWrap::NoWrap },
            gap: Size {
                width: LengthPercentage::Length(container.gap),
                height: LengthPercentage::Length(container.gap),
            },
            size: Size {
                width,
                height: self.dimension_to_taffy(&container.style.height),
            },
            padding,
            ..Default::default()
        }
    }

    fn dimension_to_taffy(&self, dim: &Option<ComponentDimension>) -> Dimension {
        match dim {
            Some(ComponentDimension::Pixels(p)) => Dimension::Length(*p),
            Some(ComponentDimension::Percent(s)) => {
                if let Some(p) = s.strip_suffix('%').and_then(|n| n.parse::<f32>().ok()) {
                    Dimension::Percent(p / 100.0)
                } else {
                    Dimension::Auto
                }
            }
            Some(ComponentDimension::Auto) | None => Dimension::Auto,
        }
    }

    fn spacing_to_rect(&self, spacing: &Option<Spacing>) -> Rect<LengthPercentage> {
        match spacing {
            Some(Spacing::All(v)) => Rect {
                left: LengthPercentage::Length(*v),
                right: LengthPercentage::Length(*v),
                top: LengthPercentage::Length(*v),
                bottom: LengthPercentage::Length(*v),
            },
            Some(Spacing::Vertical { vertical, horizontal }) => Rect {
                left: LengthPercentage::Length(*horizontal),
                right: LengthPercentage::Length(*horizontal),
                top: LengthPercentage::Length(*vertical),
                bottom: LengthPercentage::Length(*vertical),
            },
            Some(Spacing::Individual { top, right, bottom, left }) => Rect {
                left: LengthPercentage::Length(*left),
                right: LengthPercentage::Length(*right),
                top: LengthPercentage::Length(*top),
                bottom: LengthPercentage::Length(*bottom),
            },
            None => Rect {
                left: LengthPercentage::Length(0.0),
                right: LengthPercentage::Length(0.0),
                top: LengthPercentage::Length(0.0),
                bottom: LengthPercentage::Length(0.0),
            },
        }
    }

    /// Extract computed layouts from taffy
    fn extract_layouts(&mut self, node: NodeId, path: &str, parent_x: f32, parent_y: f32) {
        if let Ok(layout) = self.taffy.layout(node) {
            let x = parent_x + layout.location.x;
            let y = parent_y + layout.location.y;

            self.layouts.insert(
                path.to_string(),
                ComputedLayout {
                    x,
                    y,
                    width: layout.size.width,
                    height: layout.size.height,
                },
            );

            // Extract children layouts
            if let Ok(children) = self.taffy.children(node) {
                for (i, child) in children.iter().enumerate() {
                    let child_path = format!("{}.{}", path, i);
                    self.extract_layouts(*child, &child_path, x, y);
                }
            }
        }
    }

    /// Get computed layout for a path
    pub fn get_layout(&self, path: &str) -> Option<ComputedLayout> {
        self.layouts.get(path).copied()
    }

    /// Get all layouts
    pub fn layouts(&self) -> &HashMap<String, ComputedLayout> {
        &self.layouts
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}
