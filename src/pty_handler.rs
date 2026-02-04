//! PTY Handler - Pseudo-terminal integration for shell subprocess
//!
//! Manages the connection between the terminal emulator and the underlying shell

use anyhow::{Context, Result};
use log::{debug, error, info};
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize, Child};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::terminal::Terminal;

/// Manages the PTY and shell subprocess
pub struct PtyHandler {
    /// The shell command to run
    shell: String,
    /// Reference to the terminal state
    terminal: Arc<Mutex<Terminal>>,
    /// PTY master/slave pair
    pty_pair: Option<PtyPair>,
    /// Writer to send input to the PTY
    writer: Option<Box<dyn Write + Send>>,
    /// Flag to indicate if the shell is running
    running: Arc<Mutex<bool>>,
    /// Child process handle
    child: Option<Box<dyn Child + Send + Sync>>,
}

impl PtyHandler {
    /// Create a new PTY handler
    pub fn new(shell: String, terminal: Arc<Mutex<Terminal>>) -> Result<Self> {
        Ok(Self {
            shell,
            terminal,
            pty_pair: None,
            writer: None,
            running: Arc::new(Mutex::new(false)),
            child: None,
        })
    }

    /// Spawn the shell process
    pub fn spawn(&mut self) -> Result<()> {
        let pty_system = native_pty_system();

        // Get terminal dimensions
        let (cols, rows) = {
            let term = self.terminal.lock().unwrap();
            (term.cols, term.rows)
        };

        // Create PTY with initial size
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        // Build the shell command
        let mut cmd = CommandBuilder::new(&self.shell);
        
        // Set up environment
        #[cfg(windows)]
        {
            cmd.env("TERM", "xterm-256color");
        }
        #[cfg(not(windows))]
        {
            cmd.env("TERM", "xterm-256color");
            cmd.env("COLORTERM", "truecolor");
        }

        // Spawn the shell
        let child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell")?;

        info!("Spawned shell: {} (PID: {:?})", self.shell, child.process_id());

        // Store child process handle
        self.child = Some(child);

        // Get the writer for sending input
        let writer = pair.master.take_writer()?;
        self.writer = Some(writer);

        // Get the reader for receiving output
        let mut reader = pair.master.try_clone_reader()?;

        // Store the PTY pair
        self.pty_pair = Some(pair);

        // Mark as running
        *self.running.lock().unwrap() = true;

        // Spawn a thread to read PTY output
        let terminal = Arc::clone(&self.terminal);
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                if !*running.lock().unwrap() {
                    break;
                }

                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF - shell exited
                        info!("Shell exited (EOF)");
                        *running.lock().unwrap() = false;
                        break;
                    }
                    Ok(n) => {
                        // Process the output
                        let data = &buf[..n];
                        if let Ok(mut term) = terminal.lock() {
                            term.process_input(data);
                        }
                    }
                    Err(e) => {
                        // On Windows, certain errors indicate the process exited
                        let is_exit_error = e.kind() == std::io::ErrorKind::BrokenPipe
                            || e.kind() == std::io::ErrorKind::UnexpectedEof
                            || e.kind() == std::io::ErrorKind::ConnectionReset
                            || e.raw_os_error() == Some(109); // ERROR_BROKEN_PIPE on Windows
                        
                        if is_exit_error {
                            info!("Shell exited (pipe closed)");
                            *running.lock().unwrap() = false;
                            break;
                        } else if e.kind() != std::io::ErrorKind::WouldBlock {
                            error!("Error reading PTY: {} (kind: {:?})", e, e.kind());
                            *running.lock().unwrap() = false;
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Write input to the PTY
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        if let Some(ref mut writer) = self.writer {
            writer.write_all(data)?;
            writer.flush()?;
            debug!("Wrote {} bytes to PTY", data.len());
        }
        Ok(())
    }

    /// Read any pending output (called from main loop)
    pub fn read_output(&mut self) {
        // Output is handled by the reader thread
        // This method exists for API compatibility
    }

    /// Resize the PTY
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        if let Some(ref pair) = self.pty_pair {
            pair.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
            debug!("Resized PTY to {}x{}", cols, rows);
        }
        Ok(())
    }

    /// Check if the shell is still running
    pub fn is_running(&mut self) -> bool {
        // First check the running flag (set by reader thread)
        if !*self.running.lock().unwrap() {
            return false;
        }
        
        // Also poll the child process directly
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    info!("Child process exited with status: {:?}", status);
                    *self.running.lock().unwrap() = false;
                    return false;
                }
                Ok(None) => {
                    // Still running
                }
                Err(e) => {
                    error!("Error checking child status: {}", e);
                }
            }
        }
        
        true
    }
}

impl Drop for PtyHandler {
    fn drop(&mut self) {
        *self.running.lock().unwrap() = false;
    }
}
