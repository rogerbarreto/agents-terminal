# Pixel Terminal

A high-resolution terminal emulator that renders at pixel-level resolution, enabling rich visual CLI experiences beyond traditional ANSI character cells.

## Features

- **GPU-Accelerated Rendering**: Uses wgpu for fast, cross-platform GPU rendering
- **Pixel Canvas**: Render images and graphics inline with terminal text
- **Full ANSI Support**: Complete VT100/ANSI escape sequence parsing
- **256-Color & TrueColor**: Full color support including 24-bit RGB
- **iTerm2 Image Protocol**: Compatible with existing image-capable CLI tools
- **Custom Pixel Protocol**: Extended OSC sequences for advanced graphics
- **Cross-Platform**: Windows, macOS, and Linux support

## Building

### Prerequisites

- Rust 1.70 or later
- A GPU with Vulkan, Metal, or DirectX 12 support

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run
cargo run --release
```

## Configuration

Configuration is stored in:
- **Windows**: `%APPDATA%\pixel-terminal\config.toml`
- **macOS/Linux**: `~/.config/pixel-terminal/config.toml`

Example configuration:

```toml
columns = 120
rows = 30
font_size = 14
font_family = "Consolas"
shell = "powershell.exe"
background_color = [0.1, 0.1, 0.12, 1.0]
foreground_color = [0.9, 0.9, 0.9, 1.0]
cursor_color = [0.9, 0.9, 0.9, 0.8]
enable_pixel_canvas = true
scrollback_lines = 10000
```

## Pixel Protocol

CLI applications can send pixel data using OSC (Operating System Command) escape sequences.

### Inline Images (iTerm2 Compatible)

```
ESC ] 1337 ; File=inline=1;width=100;height=50:<base64-image-data> BEL
```

Parameters:
- `inline=1` - Display image inline (required)
- `width=<N>` - Width in pixels (optional)
- `height=<N>` - Height in pixels (optional)
- `preserveAspectRatio=1` - Maintain aspect ratio (default)

### Custom Canvas Commands

```
ESC ] 1337 ; Canvas=create;id=1;x=0;y=0;w=200;h=100 BEL
ESC ] 1337 ; Canvas=draw;id=1;x=10;y=10;color=#FF0000;rect=50x50 BEL
ESC ] 1337 ; Canvas=clear;id=1 BEL
ESC ] 1337 ; Canvas=delete;id=1 BEL
```

### Example: Display an Image from Shell

```bash
# Using base64 to encode an image
printf '\e]1337;File=inline=1:'
base64 < image.png
printf '\a'
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Pixel Terminal                    │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │   Render    │  │   Input     │  │   Shell     │  │
│  │   Engine    │  │   Handler   │  │   Bridge    │  │
│  │   (wgpu)    │  │   (winit)   │  │   (pty)     │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│              Pixel Canvas Layer                      │
│  - Inline images (PNG, JPEG)                        │
│  - Vector graphics primitives                       │
│  - Custom drawable regions                          │
├─────────────────────────────────────────────────────┤
│              ANSI/VT100 Parser                       │
│  - Standard escape sequences                        │
│  - 256-color and TrueColor                         │
│  - Cursor control and screen manipulation          │
└─────────────────────────────────────────────────────┘
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Ctrl+Shift+C | Copy selection |
| Ctrl+Shift+V | Paste |
| Ctrl+Shift+N | New window |
| Ctrl++ | Increase font size |
| Ctrl+- | Decrease font size |
| Ctrl+0 | Reset font size |

## Development

### Project Structure

```
src/
├── main.rs          # Entry point and event loop
├── config.rs        # Configuration handling
├── terminal.rs      # Terminal state and buffer management
├── ansi.rs          # ANSI escape sequence parser
├── renderer.rs      # GPU rendering with wgpu
├── pty_handler.rs   # PTY/shell integration
├── pixel_canvas.rs  # Pixel graphics support
└── shader.wgsl      # GPU shaders
```

### Running Tests

```bash
cargo test
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [wgpu](https://wgpu.rs/) - Cross-platform GPU abstraction
- [winit](https://github.com/rust-windowing/winit) - Window creation
- [portable-pty](https://github.com/AlessioDP/portable-pty) - PTY abstraction
- [ab_glyph](https://github.com/alexheretic/ab-glyph) - Font rendering
