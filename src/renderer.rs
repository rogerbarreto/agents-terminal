//! GPU Renderer - wgpu-based rendering for the terminal
//!
//! Handles all GPU rendering including text, backgrounds, and pixel canvases

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use winit::window::Window;

use crate::config::Config;
use crate::terminal::Terminal;
use crate::ui::components::{Component, Color, Spacing};

/// Load a font, trying system fonts first, then fallback
fn load_font() -> Result<FontVec> {
    #[cfg(windows)]
    let font_paths = [
        "C:\\Windows\\Fonts\\consola.ttf",
        "C:\\Windows\\Fonts\\cour.ttf",
        "C:\\Windows\\Fonts\\lucon.ttf",
    ];
    
    #[cfg(target_os = "macos")]
    let font_paths = [
        "/System/Library/Fonts/Monaco.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/Library/Fonts/Courier New.ttf",
    ];
    
    #[cfg(target_os = "linux")]
    let font_paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
    ];
    
    for path in font_paths {
        if let Ok(data) = std::fs::read(path) {
            if let Ok(font) = FontVec::try_from_vec(data) {
                log::info!("Loaded font from: {}", path);
                return Ok(font);
            }
        }
    }
    
    Err(anyhow::anyhow!("No suitable monospace font found."))
}

/// Vertex for rendering quads
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Glyph cache entry
#[derive(Clone)]
struct GlyphInfo {
    /// UV coordinates in the atlas (u0, v0, u1, v1)
    uv: [f32; 4],
    /// Size in pixels
    width: f32,
    height: f32,
    /// Bearing (offset from cursor)
    bearing_x: f32,
    bearing_y: f32,
}

const ATLAS_SIZE: u32 = 1024;

/// The GPU renderer
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (u32, u32),
    
    // Rendering pipelines
    quad_pipeline: wgpu::RenderPipeline,
    
    // Buffers
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    
    // Font/glyph data
    font: FontVec,
    font_size: f32,
    cell_width: f32,
    cell_height: f32,
    
    // Glyph atlas texture
    glyph_texture: wgpu::Texture,
    glyph_bind_group: wgpu::BindGroup,
    glyph_cache: HashMap<char, GlyphInfo>,
    atlas_data: Vec<u8>,
    atlas_x: u32,
    atlas_y: u32,
    atlas_row_height: u32,
    
    // Background color
    background_color: [f32; 4],
}

impl Renderer {
    /// Create a new renderer
    pub async fn new(window: Arc<Window>, term_config: &Config) -> Result<Self> {
        let size = window.inner_size();
        
        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // Create surface
        let surface = instance.create_surface(window)?;
        
        // Get adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Failed to find GPU adapter")?;
        
        // Create device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await?;
        
        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        
        // Load font - try to load system font, fallback to embedded
        let font = load_font()?;
        
        // Calculate cell dimensions
        let font_size = term_config.font_size as f32;
        let cell_width = font_size * 0.6;
        let cell_height = font_size * 1.2;
        
        // Create glyph atlas texture
        let atlas_size = 1024u32;
        let glyph_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let glyph_view = glyph_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let glyph_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Glyph Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        
        let glyph_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Glyph Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&glyph_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&glyph_sampler),
                },
            ],
        });
        
        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terminal Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create render pipeline
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Quad Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        
        // Create vertex and index buffers (will be updated each frame)
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1024 * 1024, // 1MB
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: 512 * 1024, // 512KB
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Ok(Self {
            surface,
            device,
            queue,
            config,
            size: (size.width, size.height),
            quad_pipeline,
            vertex_buffer,
            index_buffer,
            font,
            font_size,
            cell_width,
            cell_height,
            glyph_texture,
            glyph_bind_group,
            glyph_cache: HashMap::new(),
            atlas_data: vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize],
            atlas_x: 1,
            atlas_y: 1,
            atlas_row_height: 0,
            background_color: term_config.background_color,
        })
    }
    
    /// Rasterize a glyph and add it to the atlas
    fn cache_glyph(&mut self, c: char) -> Option<GlyphInfo> {
        if self.glyph_cache.contains_key(&c) {
            return self.glyph_cache.get(&c).cloned();
        }
        
        let scale = PxScale::from(self.font_size);
        let scaled_font = self.font.as_scaled(scale);
        
        let glyph_id = self.font.glyph_id(c);
        let glyph = glyph_id.with_scale(scale);
        
        if let Some(outlined) = self.font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            let glyph_width = bounds.width() as u32;
            let glyph_height = bounds.height() as u32;
            
            if glyph_width == 0 || glyph_height == 0 {
                return None;
            }
            
            // Check if we need to move to next row
            if self.atlas_x + glyph_width + 1 >= ATLAS_SIZE {
                self.atlas_x = 1;
                self.atlas_y += self.atlas_row_height + 1;
                self.atlas_row_height = 0;
            }
            
            // Check if atlas is full
            if self.atlas_y + glyph_height >= ATLAS_SIZE {
                log::warn!("Glyph atlas is full!");
                return None;
            }
            
            // Rasterize the glyph
            outlined.draw(|x, y, coverage| {
                let px = self.atlas_x + x;
                let py = self.atlas_y + y;
                if px < ATLAS_SIZE && py < ATLAS_SIZE {
                    let idx = (py * ATLAS_SIZE + px) as usize;
                    self.atlas_data[idx] = (coverage * 255.0) as u8;
                }
            });
            
            // Calculate UV coordinates
            let u0 = self.atlas_x as f32 / ATLAS_SIZE as f32;
            let v0 = self.atlas_y as f32 / ATLAS_SIZE as f32;
            let u1 = (self.atlas_x + glyph_width) as f32 / ATLAS_SIZE as f32;
            let v1 = (self.atlas_y + glyph_height) as f32 / ATLAS_SIZE as f32;
            
            let h_metrics = scaled_font.h_side_bearing(glyph_id);
            let v_metrics = scaled_font.ascent();
            
            let info = GlyphInfo {
                uv: [u0, v0, u1, v1],
                width: glyph_width as f32,
                height: glyph_height as f32,
                bearing_x: bounds.min.x,
                bearing_y: v_metrics - bounds.min.y,
            };
            
            // Advance atlas position
            self.atlas_x += glyph_width + 1;
            self.atlas_row_height = self.atlas_row_height.max(glyph_height);
            
            self.glyph_cache.insert(c, info.clone());
            Some(info)
        } else {
            None
        }
    }
    
    /// Update the glyph atlas texture
    fn update_atlas_texture(&self) {
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.glyph_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.atlas_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_SIZE),
                rows_per_image: Some(ATLAS_SIZE),
            },
            wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
        );
    }
    
    /// Handle window resize
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = (width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    
    /// Render the terminal
    pub fn render(&mut self, terminal: &Terminal) -> Result<()> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Build vertices for all cells
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        
        let (width, height) = self.size;
        let width = width as f32;
        let height = height as f32;
        
        // Convert pixel coordinates to normalized device coordinates
        let to_ndc_x = |x: f32| -> f32 { (x / width) * 2.0 - 1.0 };
        let to_ndc_y = |y: f32| -> f32 { 1.0 - (y / height) * 2.0 };
        
        // Draw cell backgrounds and characters
        for row in 0..terminal.rows as usize {
            for col in 0..terminal.cols as usize {
                if let Some(cell) = terminal.get_cell(row, col) {
                    let x = col as f32 * self.cell_width;
                    let y = row as f32 * self.cell_height;
                    
                    // Background quad
                    let bg_color = [
                        cell.bg_color[0],
                        cell.bg_color[1],
                        cell.bg_color[2],
                        1.0,
                    ];
                    
                    let base_idx = vertices.len() as u32;
                    
                    vertices.push(Vertex {
                        position: [to_ndc_x(x), to_ndc_y(y)],
                        tex_coord: [0.0, 0.0],
                        color: bg_color,
                    });
                    vertices.push(Vertex {
                        position: [to_ndc_x(x + self.cell_width), to_ndc_y(y)],
                        tex_coord: [1.0, 0.0],
                        color: bg_color,
                    });
                    vertices.push(Vertex {
                        position: [to_ndc_x(x + self.cell_width), to_ndc_y(y + self.cell_height)],
                        tex_coord: [1.0, 1.0],
                        color: bg_color,
                    });
                    vertices.push(Vertex {
                        position: [to_ndc_x(x), to_ndc_y(y + self.cell_height)],
                        tex_coord: [0.0, 1.0],
                        color: bg_color,
                    });
                    
                    indices.extend_from_slice(&[
                        base_idx, base_idx + 1, base_idx + 2,
                        base_idx, base_idx + 2, base_idx + 3,
                    ]);
                    
                    // Character glyph (if not space)
                    if cell.character != ' ' && !cell.wide {
                        let fg_color = [
                            cell.fg_color[0],
                            cell.fg_color[1],
                            cell.fg_color[2],
                            1.0,
                        ];
                        
                        // Get or create glyph in atlas
                        if let Some(glyph_info) = self.cache_glyph(cell.character) {
                            // Calculate glyph position with proper baseline
                            let glyph_x = x + glyph_info.bearing_x;
                            let glyph_y = y + (self.cell_height - glyph_info.bearing_y);
                            let glyph_w = glyph_info.width;
                            let glyph_h = glyph_info.height;
                            
                            let base_idx = vertices.len() as u32;
                            
                            // Use actual UV coordinates from the atlas
                            let [u0, v0, u1, v1] = glyph_info.uv;
                            
                            vertices.push(Vertex {
                                position: [to_ndc_x(glyph_x), to_ndc_y(glyph_y)],
                                tex_coord: [u0, v0],
                                color: fg_color,
                            });
                            vertices.push(Vertex {
                                position: [to_ndc_x(glyph_x + glyph_w), to_ndc_y(glyph_y)],
                                tex_coord: [u1, v0],
                                color: fg_color,
                            });
                            vertices.push(Vertex {
                                position: [to_ndc_x(glyph_x + glyph_w), to_ndc_y(glyph_y + glyph_h)],
                                tex_coord: [u1, v1],
                                color: fg_color,
                            });
                            vertices.push(Vertex {
                                position: [to_ndc_x(glyph_x), to_ndc_y(glyph_y + glyph_h)],
                                tex_coord: [u0, v1],
                                color: fg_color,
                            });
                            
                            indices.extend_from_slice(&[
                                base_idx, base_idx + 1, base_idx + 2,
                                base_idx, base_idx + 2, base_idx + 3,
                            ]);
                        }
                    }
                }
            }
        }
        
        // Draw cursor
        if terminal.cursor.visible {
            let cursor_x = terminal.cursor.col as f32 * self.cell_width;
            let cursor_y = terminal.cursor.row as f32 * self.cell_height;
            let cursor_color = [0.9, 0.9, 0.9, 0.7];
            
            let base_idx = vertices.len() as u32;
            
            vertices.push(Vertex {
                position: [to_ndc_x(cursor_x), to_ndc_y(cursor_y)],
                tex_coord: [0.0, 0.0],
                color: cursor_color,
            });
            vertices.push(Vertex {
                position: [to_ndc_x(cursor_x + self.cell_width), to_ndc_y(cursor_y)],
                tex_coord: [0.0, 0.0],
                color: cursor_color,
            });
            vertices.push(Vertex {
                position: [to_ndc_x(cursor_x + self.cell_width), to_ndc_y(cursor_y + self.cell_height)],
                tex_coord: [0.0, 0.0],
                color: cursor_color,
            });
            vertices.push(Vertex {
                position: [to_ndc_x(cursor_x), to_ndc_y(cursor_y + self.cell_height)],
                tex_coord: [0.0, 0.0],
                color: cursor_color,
            });
            
            indices.extend_from_slice(&[
                base_idx, base_idx + 1, base_idx + 2,
                base_idx, base_idx + 2, base_idx + 3,
            ]);
        }
        
        // Update glyph atlas texture with any newly rasterized glyphs
        self.update_atlas_texture();
        
        // Upload vertex data
        if !vertices.is_empty() {
            self.queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&vertices),
            );
            self.queue.write_buffer(
                &self.index_buffer,
                0,
                bytemuck::cast_slice(&indices),
            );
        }
        
        // Create command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.background_color[0] as f64,
                            g: self.background_color[1] as f64,
                            b: self.background_color[2] as f64,
                            a: self.background_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            if !indices.is_empty() {
                render_pass.set_pipeline(&self.quad_pipeline);
                render_pass.set_bind_group(0, &self.glyph_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
            }
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }

    /// Render UI components (for test mode)
    pub fn render_ui(&mut self, components: &[Component]) -> Result<()> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        
        let (width, height) = self.size;
        let width_f = width as f32;
        let height_f = height as f32;
        
        // Convert pixel coordinates to normalized device coordinates
        let to_ndc_x = |x: f32| -> f32 { (x / width_f) * 2.0 - 1.0 };
        let to_ndc_y = |y: f32| -> f32 { 1.0 - (y / height_f) * 2.0 };
        
        // Layout state
        let mut y_offset = 0.0f32;
        
        // Render each root component
        for component in components {
            self.render_component(
                component,
                0.0,
                y_offset,
                width_f,
                &mut y_offset,
                &to_ndc_x,
                &to_ndc_y,
                &mut vertices,
                &mut indices,
            );
        }
        
        // Update glyph atlas texture
        self.update_atlas_texture();
        
        // Upload vertex data
        if !vertices.is_empty() {
            self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));
        }
        
        // Create command encoder and render
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("UI Render Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            if !indices.is_empty() {
                render_pass.set_pipeline(&self.quad_pipeline);
                render_pass.set_bind_group(0, &self.glyph_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
            }
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }

    /// Render a single component and its children
    fn render_component<F, G>(
        &mut self,
        component: &Component,
        x: f32,
        y: f32,
        available_width: f32,
        y_offset: &mut f32,
        to_ndc_x: &F,
        to_ndc_y: &G,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
    ) where
        F: Fn(f32) -> f32,
        G: Fn(f32) -> f32,
    {
        match component {
            Component::Container(c) | Component::Row(c) | Component::Col(c) => {
                let container_width = available_width;
                let padding = match &c.style.padding {
                    Some(Spacing::All(p)) => *p,
                    _ => 0.0,
                };
                
                // Get explicit height if set
                let explicit_height = c.style.height.as_ref().and_then(|d| match d {
                    crate::ui::components::Dimension::Pixels(p) => Some(*p),
                    _ => None,
                });
                
                let is_row = matches!(component, Component::Row(_)) || 
                    matches!(c.direction, crate::ui::components::FlexDirection::Row);
                
                // Calculate content height by measuring children
                let content_height = if is_row {
                    // For rows, height is max child height (estimate)
                    let child_count = c.children.len();
                    if child_count == 0 { 40.0 } else { self.estimate_component_height(&c.children[0]) }
                } else {
                    // For columns, height is sum of children
                    c.children.iter().map(|child| self.estimate_component_height(child)).sum()
                };
                
                let total_height = explicit_height.unwrap_or(content_height + padding * 2.0);
                
                // Draw background if set
                if let Some(ref bg) = c.style.background {
                    let rgba = bg.to_rgba();
                    self.draw_rounded_rect(
                        x, y, container_width, total_height,
                        c.style.border_radius.unwrap_or(0.0),
                        rgba,
                        to_ndc_x, to_ndc_y,
                        vertices, indices,
                    );
                }
                
                // Render children
                let inner_width = container_width - padding * 2.0;
                let start_x = x + padding;
                let start_y = y + padding;
                
                if is_row {
                    // Horizontal layout - divide width among children
                    let child_count = c.children.len().max(1) as f32;
                    let child_width = inner_width / child_count;
                    let mut child_x = start_x;
                    
                    for child in &c.children {
                        let mut dummy_offset = start_y;
                        self.render_component(
                            child, child_x, start_y, child_width,
                            &mut dummy_offset, to_ndc_x, to_ndc_y, vertices, indices,
                        );
                        child_x += child_width;
                    }
                } else {
                    // Vertical layout - stack children
                    let mut child_y = start_y;
                    for child in &c.children {
                        self.render_component(
                            child, start_x, child_y, inner_width,
                            &mut child_y, to_ndc_x, to_ndc_y, vertices, indices,
                        );
                    }
                }
                
                *y_offset = y + total_height;
            }
            
            Component::Text(t) => {
                let size = t.size.unwrap_or(14.0);
                let color = t.color.as_ref()
                    .map(|c| c.to_rgba())
                    .unwrap_or([1.0, 1.0, 1.0, 1.0]);
                
                self.draw_text(&t.content, x, y, size, color, to_ndc_x, to_ndc_y, vertices, indices);
                *y_offset = y + size * 1.8;
            }
            
            Component::Button(b) => {
                let btn_width = b.width.unwrap_or(available_width.min(200.0));
                let btn_height = b.height.unwrap_or(36.0);
                let bg_color = b.variant.background_color().to_rgba();
                let text_color = b.variant.text_color().to_rgba();
                
                // Draw button background
                self.draw_rounded_rect(
                    x, y, btn_width, btn_height, 4.0, bg_color,
                    to_ndc_x, to_ndc_y, vertices, indices,
                );
                
                // Draw button label centered
                let text_x = x + 10.0;
                let text_y = y + (btn_height - 14.0) / 2.0;
                self.draw_text(&b.label, text_x, text_y, 14.0, text_color, to_ndc_x, to_ndc_y, vertices, indices);
                
                *y_offset = y + btn_height + 10.0;
            }
            
            Component::Progress(p) => {
                let bar_width = available_width.min(300.0);
                let bar_height = p.height;
                let bg_color = p.background.to_rgba();
                let fill_color = p.color.as_ref()
                    .map(|c| c.to_rgba())
                    .unwrap_or_else(|| p.variant.color().to_rgba());
                
                // Draw label ABOVE the bar if present
                let bar_y = if let Some(ref label) = p.label {
                    let pct = (p.value / p.max * 100.0) as i32;
                    let text = format!("{} ({}%)", label, pct);
                    self.draw_text(&text, x, y, 12.0, [0.7, 0.7, 0.7, 1.0], to_ndc_x, to_ndc_y, vertices, indices);
                    y + 16.0 // Space for label
                } else {
                    y
                };
                
                // Draw background
                self.draw_rounded_rect(
                    x, bar_y, bar_width, bar_height, 4.0, bg_color,
                    to_ndc_x, to_ndc_y, vertices, indices,
                );
                
                // Draw fill
                let fill_width = (p.value / p.max) * bar_width;
                if fill_width > 0.0 {
                    self.draw_rounded_rect(
                        x, bar_y, fill_width, bar_height, 4.0, fill_color,
                        to_ndc_x, to_ndc_y, vertices, indices,
                    );
                }
                
                *y_offset = bar_y + bar_height + 10.0;
            }
            
            _ => {
                // Placeholder for other components
                *y_offset = y + 20.0;
            }
        }
    }

    /// Estimate height of a component for layout calculations
    fn estimate_component_height(&self, component: &Component) -> f32 {
        match component {
            Component::Container(c) | Component::Row(c) | Component::Col(c) => {
                let padding = match &c.style.padding {
                    Some(Spacing::All(p)) => *p,
                    _ => 0.0,
                };
                
                if let Some(ref h) = c.style.height {
                    if let crate::ui::components::Dimension::Pixels(p) = h {
                        return *p;
                    }
                }
                
                // Estimate from children
                let is_row = matches!(c.direction, crate::ui::components::FlexDirection::Row);
                if is_row {
                    // Max height of children
                    let max_h = c.children.iter()
                        .map(|ch| self.estimate_component_height(ch))
                        .fold(0.0f32, |a, b| a.max(b));
                    max_h + padding * 2.0
                } else {
                    // Sum of children heights
                    let sum: f32 = c.children.iter()
                        .map(|ch| self.estimate_component_height(ch))
                        .sum();
                    sum + padding * 2.0
                }
            }
            Component::Text(t) => {
                let size = t.size.unwrap_or(14.0);
                size * 1.8
            }
            Component::Button(b) => b.height.unwrap_or(36.0) + 10.0,
            Component::Progress(p) => {
                // Label (16px) + bar height + spacing (10px)
                if p.label.is_some() {
                    16.0 + p.height + 10.0
                } else {
                    p.height + 10.0
                }
            }
            Component::Input(_) => 46.0,
            Component::Image(img) => {
                let h = img.height.as_ref().and_then(|d| match d {
                    crate::ui::components::Dimension::Pixels(p) => Some(*p),
                    _ => None,
                }).unwrap_or(100.0);
                h + 10.0
            }
            Component::Table(_) => 150.0,
        }
    }

    /// Draw a rounded rectangle (simplified - just draws a regular rect for now)
    fn draw_rounded_rect<F, G>(
        &self,
        x: f32, y: f32, w: f32, h: f32,
        _radius: f32,
        color: [f32; 4],
        to_ndc_x: &F,
        to_ndc_y: &G,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
    ) where
        F: Fn(f32) -> f32,
        G: Fn(f32) -> f32,
    {
        let base_idx = vertices.len() as u32;
        
        vertices.push(Vertex {
            position: [to_ndc_x(x), to_ndc_y(y)],
            tex_coord: [0.0, 0.0],
            color,
        });
        vertices.push(Vertex {
            position: [to_ndc_x(x + w), to_ndc_y(y)],
            tex_coord: [0.0, 0.0],
            color,
        });
        vertices.push(Vertex {
            position: [to_ndc_x(x + w), to_ndc_y(y + h)],
            tex_coord: [0.0, 0.0],
            color,
        });
        vertices.push(Vertex {
            position: [to_ndc_x(x), to_ndc_y(y + h)],
            tex_coord: [0.0, 0.0],
            color,
        });
        
        indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,
            base_idx, base_idx + 2, base_idx + 3,
        ]);
    }

    /// Draw text at a position
    fn draw_text<F, G>(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [f32; 4],
        to_ndc_x: &F,
        to_ndc_y: &G,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
    ) where
        F: Fn(f32) -> f32,
        G: Fn(f32) -> f32,
    {
        let mut cursor_x = x;
        let scale_factor = size / self.font_size;
        
        for ch in text.chars() {
            if ch == ' ' {
                cursor_x += self.cell_width * scale_factor * 0.5;
                continue;
            }
            
            if let Some(glyph_info) = self.cache_glyph(ch) {
                let glyph_w = glyph_info.width * scale_factor;
                let glyph_h = glyph_info.height * scale_factor;
                let glyph_x = cursor_x + glyph_info.bearing_x * scale_factor;
                let glyph_y = y + (size - glyph_info.bearing_y * scale_factor);
                
                let [u0, v0, u1, v1] = glyph_info.uv;
                let base_idx = vertices.len() as u32;
                
                vertices.push(Vertex {
                    position: [to_ndc_x(glyph_x), to_ndc_y(glyph_y)],
                    tex_coord: [u0, v0],
                    color,
                });
                vertices.push(Vertex {
                    position: [to_ndc_x(glyph_x + glyph_w), to_ndc_y(glyph_y)],
                    tex_coord: [u1, v0],
                    color,
                });
                vertices.push(Vertex {
                    position: [to_ndc_x(glyph_x + glyph_w), to_ndc_y(glyph_y + glyph_h)],
                    tex_coord: [u1, v1],
                    color,
                });
                vertices.push(Vertex {
                    position: [to_ndc_x(glyph_x), to_ndc_y(glyph_y + glyph_h)],
                    tex_coord: [u0, v1],
                    color,
                });
                
                indices.extend_from_slice(&[
                    base_idx, base_idx + 1, base_idx + 2,
                    base_idx, base_idx + 2, base_idx + 3,
                ]);
                
                cursor_x += glyph_w + 1.0;
            } else {
                cursor_x += self.cell_width * scale_factor * 0.5;
            }
        }
    }
}
