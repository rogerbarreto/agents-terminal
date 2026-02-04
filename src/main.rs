//! Pixel Terminal - A high-resolution terminal emulator
//!
//! This terminal renders at pixel-level resolution, enabling rich visual CLI experiences
//! beyond traditional ANSI character cells.
//!
//! ## Usage
//! 
//! Interactive mode (default):
//! ```
//! pixel-terminal
//! ```
//!
//! Pipe mode (execute command and exit):
//! ```
//! pixel-terminal -c "echo hello"
//! pixel-terminal --command "ls -la"
//! echo "hello" | pixel-terminal
//! ```

mod config;
mod terminal;
mod renderer;
mod pty_handler;
mod ansi;
mod pixel_canvas;
mod ui;

use anyhow::Result;
use log::{info, error};
use winit::{
    event::{Event, WindowEvent, ElementState, KeyEvent},
    event_loop::{EventLoop, ControlFlow},
    window::WindowBuilder,
    keyboard::{Key, NamedKey},
    dpi::LogicalSize,
};
use std::sync::{Arc, Mutex};
use std::io::{self, Read, Write};

use crate::config::Config;
use crate::terminal::Terminal;
use crate::renderer::Renderer;
use crate::pty_handler::PtyHandler;
use crate::ui::ComponentTree;

/// Command-line arguments
struct Args {
    /// Command to execute (pipe mode)
    command: Option<String>,
    /// Read from stdin
    stdin: bool,
    /// Show help
    help: bool,
    /// Show version
    version: bool,
    /// Test UI components mode
    test_ui: bool,
}

impl Args {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut result = Args {
            command: None,
            stdin: false,
            help: false,
            version: false,
            test_ui: false,
        };

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-c" | "--command" => {
                    if i + 1 < args.len() {
                        result.command = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "-" | "--stdin" => {
                    result.stdin = true;
                }
                "-h" | "--help" => {
                    result.help = true;
                }
                "-v" | "--version" => {
                    result.version = true;
                }
                "--test-ui" | "-t" => {
                    result.test_ui = true;
                }
                arg if !arg.starts_with('-') && result.command.is_none() => {
                    // Treat as command if no -c flag
                    result.command = Some(args[i..].join(" "));
                    break;
                }
                _ => {}
            }
            i += 1;
        }

        // Check if stdin has data (non-blocking check)
        #[cfg(windows)]
        {
            // On Windows, check if stdin is a pipe/file (not console)
            use std::os::windows::io::AsRawHandle;
            use winapi::um::fileapi::GetFileType;
            use winapi::um::winbase::FILE_TYPE_CHAR;
            
            let handle = io::stdin().as_raw_handle();
            let file_type = unsafe { GetFileType(handle as *mut _) };
            if file_type != FILE_TYPE_CHAR {
                result.stdin = true;
            }
        }

        result
    }
}

fn print_help() {
    println!(
        r#"Pixel Terminal - A high-resolution terminal emulator

USAGE:
    pixel-terminal [OPTIONS] [COMMAND]

OPTIONS:
    -c, --command <CMD>    Execute command and exit
    -t, --test-ui          Test UI components rendering
    -h, --help             Show this help message
    -v, --version          Show version
    --stdin                Read input from stdin

EXAMPLES:
    pixel-terminal                     # Interactive mode
    pixel-terminal -c "echo hello"     # Execute and exit
    pixel-terminal --test-ui           # Test UI components
    echo "hello" | pixel-terminal      # Pipe mode

PTUI PROTOCOL:
    Send component UI via escape sequence:
    printf '\e]1337;PTUI={{"type":"text","content":"Hello"}}\a'
"#
    );
}

fn print_version() {
    println!("pixel-terminal {}", env!("CARGO_PKG_VERSION"));
}

/// Run in pipe mode - execute command and output result
fn run_pipe_mode(command: Option<String>, read_stdin: bool) -> Result<()> {
    use std::process::{Command, Stdio};

    // Get input from stdin if piped
    let stdin_data = if read_stdin && command.is_none() {
        // Only read stdin if we don't have a command to execute
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        Some(buffer)
    } else {
        None
    };

    if let Some(cmd) = command {
        // Execute the provided command
        #[cfg(windows)]
        let output = Command::new("cmd")
            .args(["/C", &cmd])
            .stdin(if read_stdin { Stdio::inherit() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        #[cfg(not(windows))]
        let output = Command::new("sh")
            .args(["-c", &cmd])
            .stdin(if read_stdin { Stdio::inherit() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        // Output stdout
        io::stdout().write_all(&output.stdout)?;
        
        // Output stderr
        io::stderr().write_all(&output.stderr)?;

        // Exit with the command's exit code
        std::process::exit(output.status.code().unwrap_or(1));
    } else if let Some(data) = stdin_data {
        // Just echo stdin data (passthrough mode)
        print!("{}", data);
    }

    Ok(())
}

fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    if args.help {
        print_help();
        return Ok(());
    }

    if args.version {
        print_version();
        return Ok(());
    }

    // Pipe mode: execute command and exit
    if args.command.is_some() || args.stdin {
        return run_pipe_mode(args.command, args.stdin);
    }

    // Test UI mode
    if args.test_ui {
        return run_test_ui_mode();
    }

    // Interactive mode
    run_interactive_mode()
}

fn run_interactive_mode() -> Result<()> {
    env_logger::init();
    info!("Starting Pixel Terminal v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::load_or_default();
    info!("Loaded config: {}x{} cells", config.columns, config.rows);

    // Create the event loop and window
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new()
        .with_title("Pixel Terminal")
        .with_inner_size(LogicalSize::new(
            config.columns as f64 * config.font_size as f64 * 0.6,
            config.rows as f64 * config.font_size as f64 * 1.2,
        ))
        .with_min_inner_size(LogicalSize::new(400.0, 300.0))
        .build(&event_loop)?;

    let window = Arc::new(window);

    // Initialize the GPU renderer
    let mut renderer = pollster::block_on(Renderer::new(window.clone(), &config))?;

    // Create terminal state
    let terminal = Arc::new(Mutex::new(Terminal::new(config.columns, config.rows)));

    // Start the PTY/shell process
    let mut pty_handler = PtyHandler::new(config.shell.clone(), terminal.clone())?;
    pty_handler.spawn()?;

    info!("Terminal initialized, entering main loop");

    // Main event loop
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        info!("Close requested, shutting down");
                        elwt.exit();
                    }

                    WindowEvent::Resized(new_size) => {
                        if new_size.width > 0 && new_size.height > 0 {
                            renderer.resize(new_size.width, new_size.height);
                            // Update terminal dimensions based on new size
                            let cols = (new_size.width as f32 / (config.font_size as f32 * 0.6)) as u16;
                            let rows = (new_size.height as f32 / (config.font_size as f32 * 1.2)) as u16;
                            if let Ok(mut term) = terminal.lock() {
                                term.resize(cols.max(10), rows.max(3));
                            }
                            let _ = pty_handler.resize(cols.max(10), rows.max(3));
                        }
                    }

                    WindowEvent::KeyboardInput { event: KeyEvent { logical_key, state: ElementState::Pressed, .. }, .. } => {
                        let input = match &logical_key {
                            Key::Named(NamedKey::Enter) => Some("\r".to_string()),
                            Key::Named(NamedKey::Backspace) => Some("\x7f".to_string()),
                            Key::Named(NamedKey::Tab) => Some("\t".to_string()),
                            Key::Named(NamedKey::Escape) => Some("\x1b".to_string()),
                            Key::Named(NamedKey::ArrowUp) => Some("\x1b[A".to_string()),
                            Key::Named(NamedKey::ArrowDown) => Some("\x1b[B".to_string()),
                            Key::Named(NamedKey::ArrowRight) => Some("\x1b[C".to_string()),
                            Key::Named(NamedKey::ArrowLeft) => Some("\x1b[D".to_string()),
                            Key::Named(NamedKey::Home) => Some("\x1b[H".to_string()),
                            Key::Named(NamedKey::End) => Some("\x1b[F".to_string()),
                            Key::Named(NamedKey::PageUp) => Some("\x1b[5~".to_string()),
                            Key::Named(NamedKey::PageDown) => Some("\x1b[6~".to_string()),
                            Key::Named(NamedKey::Delete) => Some("\x1b[3~".to_string()),
                            Key::Character(c) => Some(c.to_string()),
                            _ => None,
                        };

                        if let Some(input) = input {
                            if let Err(e) = pty_handler.write(input.as_bytes()) {
                                error!("Failed to write to PTY: {}", e);
                            }
                        }
                    }

                    WindowEvent::RedrawRequested => {
                        // Read any pending PTY output
                        pty_handler.read_output();

                        // Render the terminal
                        if let Ok(term) = terminal.lock() {
                            if let Err(e) = renderer.render(&term) {
                                error!("Render error: {}", e);
                            }
                        }
                    }

                    _ => {}
                }
            }

            Event::AboutToWait => {
                // Check if shell has exited
                if !pty_handler.is_running() {
                    info!("Shell exited, closing terminal");
                    elwt.exit();
                    return;
                }
                
                // Read PTY output and request redraw
                pty_handler.read_output();
                window.request_redraw();
            }

            _ => {}
        }
    })?;

    Ok(())
}

/// Run in UI test mode - display sample components
fn run_test_ui_mode() -> Result<()> {
    env_logger::init();
    info!("Starting Pixel Terminal UI Test Mode");

    use crate::ui::components::{
        Component, Container, Text, Button, Progress, Color, Style, Dimension, 
        FlexDirection, Alignment, Border, BorderStyle,
    };

    // Load configuration
    let config = Config::load_or_default();

    // Create the event loop and window
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new()
        .with_title("Pixel Terminal - UI Test")
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .with_min_inner_size(LogicalSize::new(400.0, 300.0))
        .build(&event_loop)?;

    let window = Arc::new(window);

    // Initialize the GPU renderer
    let mut renderer = pollster::block_on(Renderer::new(window.clone(), &config))?;

    // Create sample UI components
    let components = create_test_components();
    info!("Created {} test components", components.len());

    // Main event loop
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        info!("Close requested, shutting down");
                        elwt.exit();
                    }

                    WindowEvent::Resized(new_size) => {
                        if new_size.width > 0 && new_size.height > 0 {
                            renderer.resize(new_size.width, new_size.height);
                        }
                    }

                    WindowEvent::KeyboardInput { event: KeyEvent { logical_key, state: ElementState::Pressed, .. }, .. } => {
                        // Press Escape or Q to quit
                        match &logical_key {
                            Key::Named(NamedKey::Escape) => {
                                elwt.exit();
                            }
                            Key::Character(c) if c == "q" || c == "Q" => {
                                elwt.exit();
                            }
                            _ => {}
                        }
                    }

                    WindowEvent::RedrawRequested => {
                        // Render UI components
                        if let Err(e) = renderer.render_ui(&components) {
                            error!("Render error: {}", e);
                        }
                    }

                    _ => {}
                }
            }

            Event::AboutToWait => {
                window.request_redraw();
            }

            _ => {}
        }
    })?;

    Ok(())
}

/// Create test UI components for demonstration
fn create_test_components() -> Vec<crate::ui::components::Component> {
    use crate::ui::components::*;

    vec![
        // Header container
        Component::Container(Container {
            id: Some("header".to_string()),
            direction: FlexDirection::Row,
            justify: Alignment::Center,
            align: Alignment::Center,
            style: Style {
                width: Some(Dimension::Percent("100%".to_string())),
                height: Some(Dimension::Pixels(60.0)),
                background: Some(Color::Hex("#2D3748".to_string())),
                padding: Some(Spacing::All(10.0)),
                ..Default::default()
            },
            children: vec![
                Component::Text(Text {
                    id: Some("title".to_string()),
                    content: "Pixel Terminal UI Test".to_string(),
                    size: Some(24.0),
                    color: Some(Color::Hex("#FFFFFF".to_string())),
                    bold: true,
                    ..Default::default()
                }),
            ],
            ..Default::default()
        }),

        // Main content row
        Component::Row(Container {
            id: Some("main-row".to_string()),
            direction: FlexDirection::Row,
            justify: Alignment::SpaceAround,
            align: Alignment::Start,
            style: Style {
                width: Some(Dimension::Percent("100%".to_string())),
                height: Some(Dimension::Pixels(280.0)), // More space for columns + gap
                padding: Some(Spacing::All(20.0)),
                ..Default::default()
            },
            children: vec![
                // Left column - buttons
                Component::Col(Container {
                    id: Some("col-buttons".to_string()),
                    span: Some(4),
                    direction: FlexDirection::Column,
                    style: Style {
                        height: Some(Dimension::Pixels(220.0)),
                        background: Some(Color::Hex("#1A202C".to_string())),
                        padding: Some(Spacing::All(15.0)),
                        border_radius: Some(8.0),
                        ..Default::default()
                    },
                    children: vec![
                        Component::Text(Text {
                            content: "Buttons".to_string(),
                            size: Some(18.0),
                            color: Some(Color::Hex("#A0AEC0".to_string())),
                            bold: true,
                            ..Default::default()
                        }),
                        Component::Button(Button {
                            id: "btn-primary".to_string(),
                            label: "Primary Button".to_string(),
                            variant: ButtonVariant::Primary,
                            ..Default::default()
                        }),
                        Component::Button(Button {
                            id: "btn-success".to_string(),
                            label: "Success".to_string(),
                            variant: ButtonVariant::Success,
                            ..Default::default()
                        }),
                        Component::Button(Button {
                            id: "btn-danger".to_string(),
                            label: "Danger".to_string(),
                            variant: ButtonVariant::Danger,
                            ..Default::default()
                        }),
                    ],
                    ..Default::default()
                }),

                // Middle column - progress bars
                Component::Col(Container {
                    id: Some("col-progress".to_string()),
                    span: Some(4),
                    direction: FlexDirection::Column,
                    style: Style {
                        height: Some(Dimension::Pixels(220.0)),
                        background: Some(Color::Hex("#1A202C".to_string())),
                        padding: Some(Spacing::All(15.0)),
                        border_radius: Some(8.0),
                        ..Default::default()
                    },
                    children: vec![
                        Component::Text(Text {
                            content: "Progress Bars".to_string(),
                            size: Some(18.0),
                            color: Some(Color::Hex("#A0AEC0".to_string())),
                            bold: true,
                            ..Default::default()
                        }),
                        Component::Progress(Progress {
                            id: Some("progress-1".to_string()),
                            value: 75.0,
                            max: 100.0,
                            label: Some("Loading...".to_string()),
                            variant: ProgressVariant::Default,
                            ..Default::default()
                        }),
                        Component::Progress(Progress {
                            id: Some("progress-2".to_string()),
                            value: 45.0,
                            max: 100.0,
                            label: Some("Downloading".to_string()),
                            variant: ProgressVariant::Striped,
                            color: Some(Color::Hex("#48BB78".to_string())),
                            ..Default::default()
                        }),
                        Component::Progress(Progress {
                            id: Some("progress-3".to_string()),
                            value: 90.0,
                            max: 100.0,
                            label: Some("Almost done!".to_string()),
                            variant: ProgressVariant::Animated,
                            color: Some(Color::Hex("#ED8936".to_string())),
                            ..Default::default()
                        }),
                    ],
                    ..Default::default()
                }),

                // Right column - text samples
                Component::Col(Container {
                    id: Some("col-text".to_string()),
                    span: Some(4),
                    direction: FlexDirection::Column,
                    style: Style {
                        height: Some(Dimension::Pixels(220.0)),
                        background: Some(Color::Hex("#1A202C".to_string())),
                        padding: Some(Spacing::All(15.0)),
                        border_radius: Some(8.0),
                        ..Default::default()
                    },
                    children: vec![
                        Component::Text(Text {
                            content: "Text Styles".to_string(),
                            size: Some(18.0),
                            color: Some(Color::Hex("#A0AEC0".to_string())),
                            bold: true,
                            ..Default::default()
                        }),
                        Component::Text(Text {
                            content: "Normal text".to_string(),
                            color: Some(Color::Hex("#E2E8F0".to_string())),
                            ..Default::default()
                        }),
                        Component::Text(Text {
                            content: "Bold text".to_string(),
                            bold: true,
                            color: Some(Color::Hex("#E2E8F0".to_string())),
                            ..Default::default()
                        }),
                        Component::Text(Text {
                            content: "Italic text".to_string(),
                            italic: true,
                            color: Some(Color::Hex("#E2E8F0".to_string())),
                            ..Default::default()
                        }),
                        Component::Text(Text {
                            content: "Colored text".to_string(),
                            color: Some(Color::Hex("#63B3ED".to_string())),
                            ..Default::default()
                        }),
                    ],
                    ..Default::default()
                }),
            ],
            ..Default::default()
        }),

        // Footer
        Component::Container(Container {
            id: Some("footer".to_string()),
            direction: FlexDirection::Row,
            justify: Alignment::Center,
            align: Alignment::Center,
            style: Style {
                width: Some(Dimension::Percent("100%".to_string())),
                height: Some(Dimension::Pixels(50.0)),
                background: Some(Color::Hex("#0D1117".to_string())), // Darker distinct color
                padding: Some(Spacing::All(15.0)),
                ..Default::default()
            },
            children: vec![
                Component::Text(Text {
                    content: "Press Q or Escape to exit".to_string(),
                    size: Some(12.0),
                    color: Some(Color::Hex("#8B949E".to_string())),
                    ..Default::default()
                }),
            ],
            ..Default::default()
        }),
    ]
}
