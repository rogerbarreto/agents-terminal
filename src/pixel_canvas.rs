//! Pixel Canvas - High-resolution graphics support
//!
//! This module provides pixel-level rendering capabilities beyond traditional
//! terminal character cells. It supports:
//! - Inline images (via iTerm2 protocol)
//! - Custom pixel regions
//! - Vector graphics primitives

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use image::{DynamicImage, GenericImageView};
use log::{debug, warn};
use std::collections::HashMap;

/// A pixel-based image or canvas region
#[derive(Debug)]
pub struct PixelRegion {
    /// Unique identifier for this region
    pub id: u32,
    /// X position in pixels (relative to terminal origin)
    pub x: u32,
    /// Y position in pixels
    pub y: u32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// RGBA pixel data
    pub pixels: Vec<u8>,
    /// Terminal row where this image is anchored
    pub anchor_row: u16,
    /// Terminal column where this image is anchored  
    pub anchor_col: u16,
    /// Whether this is an inline image (flows with text)
    pub inline: bool,
}

/// Drawing command for vector graphics
#[derive(Debug, Clone)]
pub enum DrawCommand {
    Rect { x: f32, y: f32, width: f32, height: f32, color: [f32; 4] },
    Circle { cx: f32, cy: f32, radius: f32, color: [f32; 4] },
    Line { x1: f32, y1: f32, x2: f32, y2: f32, color: [f32; 4], width: f32 },
    Text { x: f32, y: f32, text: String, font_size: f32, color: [f32; 4] },
}

/// Manages pixel-based content in the terminal
pub struct PixelCanvas {
    /// Active pixel regions (images, canvases)
    pub regions: HashMap<u32, PixelRegion>,
    /// Next region ID
    next_id: u32,
    /// Pending draw commands for the current frame
    pub draw_commands: Vec<DrawCommand>,
    /// Whether pixel canvas features are enabled
    pub enabled: bool,
}

impl PixelCanvas {
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            next_id: 1,
            draw_commands: Vec::new(),
            enabled: true,
        }
    }

    /// Handle iTerm2-style inline image protocol
    /// Format: File=name=<name>;width=<w>;height=<h>;inline=1:<base64-data>
    pub fn handle_iterm2_sequence(&mut self, data: &str) {
        if !self.enabled {
            return;
        }

        // Parse the sequence
        let parts: Vec<&str> = data.splitn(2, ':').collect();
        if parts.len() != 2 {
            warn!("Invalid iTerm2 sequence format");
            return;
        }

        let params = parts[0];
        let image_data = parts[1];

        // Parse parameters
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;
        let mut inline = false;
        let mut preserve_aspect = true;

        for param in params.split(';') {
            if let Some((key, value)) = param.split_once('=') {
                match key {
                    "width" => width = parse_dimension(value),
                    "height" => height = parse_dimension(value),
                    "inline" => inline = value == "1",
                    "preserveAspectRatio" => preserve_aspect = value == "1",
                    "File" => {} // Ignore the file marker
                    _ => debug!("Unknown iTerm2 param: {}={}", key, value),
                }
            }
        }

        // Decode the image
        match BASE64.decode(image_data.trim()) {
            Ok(bytes) => {
                match image::load_from_memory(&bytes) {
                    Ok(img) => {
                        self.add_image(img, width, height, inline, preserve_aspect);
                    }
                    Err(e) => warn!("Failed to decode image: {}", e),
                }
            }
            Err(e) => warn!("Failed to decode base64: {}", e),
        }
    }

    /// Handle custom pixel protocol commands
    /// Format: Canvas=<command>;id=<n>;...
    pub fn handle_pixel_command(&mut self, data: &str) {
        if !self.enabled {
            return;
        }

        let params: HashMap<&str, &str> = data
            .split(';')
            .filter_map(|p| p.split_once('='))
            .collect();

        let command = params.get("Canvas").copied().unwrap_or("");

        match command {
            "create" => {
                // Create a new drawable canvas region
                let id = params.get("id").and_then(|s| s.parse().ok()).unwrap_or(self.next_id);
                let x = params.get("x").and_then(|s| s.parse().ok()).unwrap_or(0);
                let y = params.get("y").and_then(|s| s.parse().ok()).unwrap_or(0);
                let w: u32 = params.get("w").and_then(|s| s.parse().ok()).unwrap_or(100);
                let h: u32 = params.get("h").and_then(|s| s.parse().ok()).unwrap_or(100);

                let region = PixelRegion {
                    id,
                    x,
                    y,
                    width: w,
                    height: h,
                    pixels: vec![0; (w * h * 4) as usize],
                    anchor_row: 0,
                    anchor_col: 0,
                    inline: false,
                };

                self.regions.insert(id, region);
                self.next_id = self.next_id.max(id + 1);
            }
            "draw" => {
                // Draw to an existing canvas
                if let Some(id) = params.get("id").and_then(|s| s.parse().ok()) {
                    if let Some(region) = self.regions.get_mut(&id) {
                        draw_to_region(region, &params);
                    }
                }
            }
            "clear" => {
                // Clear a canvas
                if let Some(id) = params.get("id").and_then(|s| s.parse().ok()) {
                    if let Some(region) = self.regions.get_mut(&id) {
                        region.pixels.fill(0);
                    }
                }
            }
            "delete" => {
                // Remove a canvas
                if let Some(id) = params.get("id").and_then(|s| s.parse().ok()) {
                    self.regions.remove(&id);
                }
            }
            _ => {
                debug!("Unknown pixel command: {}", command);
            }
        }
    }

    /// Add an image to the canvas
    fn add_image(&mut self, img: DynamicImage, width: Option<u32>, height: Option<u32>, inline: bool, preserve_aspect: bool) {
        let (orig_w, orig_h) = img.dimensions();

        // Calculate final dimensions
        let (final_w, final_h) = match (width, height) {
            (Some(w), Some(h)) if !preserve_aspect => (w, h),
            (Some(w), Some(h)) => {
                let aspect = orig_w as f32 / orig_h as f32;
                let target_aspect = w as f32 / h as f32;
                if aspect > target_aspect {
                    (w, (w as f32 / aspect) as u32)
                } else {
                    ((h as f32 * aspect) as u32, h)
                }
            }
            (Some(w), None) => {
                let aspect = orig_w as f32 / orig_h as f32;
                (w, (w as f32 / aspect) as u32)
            }
            (None, Some(h)) => {
                let aspect = orig_w as f32 / orig_h as f32;
                ((h as f32 * aspect) as u32, h)
            }
            (None, None) => (orig_w, orig_h),
        };

        // Resize if needed
        let img = if final_w != orig_w || final_h != orig_h {
            img.resize_exact(final_w, final_h, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };

        // Convert to RGBA
        let rgba = img.to_rgba8();
        let pixels = rgba.into_raw();

        let id = self.next_id;
        self.next_id += 1;

        let region = PixelRegion {
            id,
            x: 0,
            y: 0,
            width: final_w,
            height: final_h,
            pixels,
            anchor_row: 0,
            anchor_col: 0,
            inline,
        };

        self.regions.insert(id, region);
        debug!("Added image region {}: {}x{}", id, final_w, final_h);
    }

    /// Clear all pixel regions
    pub fn clear(&mut self) {
        self.regions.clear();
        self.draw_commands.clear();
    }

    /// Get all regions for rendering
    pub fn get_regions(&self) -> impl Iterator<Item = &PixelRegion> {
        self.regions.values()
    }
}

/// Draw primitives to a region
fn draw_to_region(region: &mut PixelRegion, params: &HashMap<&str, &str>) {
    let x: i32 = params.get("x").and_then(|s| s.parse().ok()).unwrap_or(0);
    let y: i32 = params.get("y").and_then(|s| s.parse().ok()).unwrap_or(0);
    let color = params.get("color")
        .map(|s| parse_color(s))
        .unwrap_or([255, 255, 255, 255]);

    if let Some(rect) = params.get("rect") {
        // Draw rectangle: "WxH"
        if let Some((w, h)) = rect.split_once('x') {
            let w: u32 = w.parse().unwrap_or(10);
            let h: u32 = h.parse().unwrap_or(10);
            draw_rect(&mut region.pixels, region.width, x, y, w, h, color);
        }
    }

    if let Some(circle) = params.get("circle") {
        // Draw circle: "radius"
        let r: u32 = circle.parse().unwrap_or(10);
        draw_circle(&mut region.pixels, region.width, region.height, x, y, r, color);
    }
}

/// Parse a dimension value (can be pixels, cells, or percentage)
fn parse_dimension(value: &str) -> Option<u32> {
    // For now, just parse as pixels
    // TODO: Support "auto", percentages, cell counts
    value.trim_end_matches("px").parse().ok()
}

/// Parse a color string (#RRGGBB or #RRGGBBAA)
fn parse_color(s: &str) -> [u8; 4] {
    let s = s.trim_start_matches('#');
    if s.len() >= 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
        let a = if s.len() >= 8 {
            u8::from_str_radix(&s[6..8], 16).unwrap_or(255)
        } else {
            255
        };
        [r, g, b, a]
    } else {
        [255, 255, 255, 255]
    }
}

/// Draw a filled rectangle
fn draw_rect(pixels: &mut [u8], stride: u32, x: i32, y: i32, w: u32, h: u32, color: [u8; 4]) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx as i32;
            let py = y + dy as i32;
            if px >= 0 && py >= 0 {
                let idx = ((py as u32) * stride + (px as u32)) as usize * 4;
                if idx + 3 < pixels.len() {
                    pixels[idx] = color[0];
                    pixels[idx + 1] = color[1];
                    pixels[idx + 2] = color[2];
                    pixels[idx + 3] = color[3];
                }
            }
        }
    }
}

/// Draw a filled circle
fn draw_circle(pixels: &mut [u8], width: u32, height: u32, cx: i32, cy: i32, r: u32, color: [u8; 4]) {
    let r = r as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                    let idx = ((py as u32) * width + (px as u32)) as usize * 4;
                    if idx + 3 < pixels.len() {
                        pixels[idx] = color[0];
                        pixels[idx + 1] = color[1];
                        pixels[idx + 2] = color[2];
                        pixels[idx + 3] = color[3];
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("#FF0000"), [255, 0, 0, 255]);
        assert_eq!(parse_color("#00FF00"), [0, 255, 0, 255]);
        assert_eq!(parse_color("#0000FF"), [0, 0, 255, 255]);
        assert_eq!(parse_color("#FF000080"), [255, 0, 0, 128]);
    }

    #[test]
    fn test_pixel_canvas_new() {
        let canvas = PixelCanvas::new();
        assert!(canvas.enabled);
        assert!(canvas.regions.is_empty());
    }
}
