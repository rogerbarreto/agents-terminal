//! ANSI escape sequence parser
//!
//! Handles parsing of ANSI/VT100 escape sequences for terminal control

use crate::terminal::Terminal;
use crate::config::ANSI_COLORS;
use log::debug;

/// Parser states
#[derive(Debug, Clone, PartialEq, Default)]
enum State {
    #[default]
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    OscString,
    DcsEntry,
}

/// ANSI escape sequence parser
#[derive(Default)]
pub struct AnsiParser {
    state: State,
    params: Vec<u16>,
    current_param: Option<u16>,
    intermediates: Vec<u8>,
    osc_string: Vec<u8>,
}

impl AnsiParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a single byte
    pub fn advance(&mut self, byte: u8, terminal: &mut Terminal) {
        match self.state {
            State::Ground => self.handle_ground(byte, terminal),
            State::Escape => self.handle_escape(byte, terminal),
            State::EscapeIntermediate => self.handle_escape_intermediate(byte, terminal),
            State::CsiEntry => self.handle_csi_entry(byte, terminal),
            State::CsiParam => self.handle_csi_param(byte, terminal),
            State::CsiIntermediate => self.handle_csi_intermediate(byte, terminal),
            State::OscString => self.handle_osc_string(byte, terminal),
            State::DcsEntry => self.handle_dcs(byte, terminal),
        }
    }

    fn reset(&mut self) {
        self.state = State::Ground;
        self.params.clear();
        self.current_param = None;
        self.intermediates.clear();
        self.osc_string.clear();
    }

    fn handle_ground(&mut self, byte: u8, terminal: &mut Terminal) {
        match byte {
            0x1B => {
                // ESC
                self.state = State::Escape;
            }
            0x00..=0x1A | 0x1C..=0x1F => {
                // Control characters
                self.handle_control(byte, terminal);
            }
            _ => {
                // Printable character
                if let Some(c) = char::from_u32(byte as u32) {
                    terminal.put_char(c);
                }
            }
        }
    }

    fn handle_control(&mut self, byte: u8, terminal: &mut Terminal) {
        match byte {
            0x07 => {
                // BEL - Bell
                debug!("Bell");
            }
            0x08 => {
                // BS - Backspace
                terminal.cursor_backward(1);
            }
            0x09 => {
                // HT - Tab
                terminal.put_char('\t');
            }
            0x0A | 0x0B | 0x0C => {
                // LF, VT, FF - Line feed
                terminal.newline();
            }
            0x0D => {
                // CR - Carriage return
                terminal.cursor.col = 0;
            }
            _ => {}
        }
    }

    fn handle_escape(&mut self, byte: u8, terminal: &mut Terminal) {
        match byte {
            b'[' => {
                // CSI - Control Sequence Introducer
                self.state = State::CsiEntry;
                self.params.clear();
                self.current_param = None;
                self.intermediates.clear();
            }
            b']' => {
                // OSC - Operating System Command
                self.state = State::OscString;
                self.osc_string.clear();
            }
            b'P' => {
                // DCS - Device Control String
                self.state = State::DcsEntry;
            }
            b'(' | b')' | b'*' | b'+' => {
                // Character set selection
                self.state = State::EscapeIntermediate;
                self.intermediates.push(byte);
            }
            b'=' => {
                // DECKPAM - Keypad Application Mode
                self.reset();
            }
            b'>' => {
                // DECKPNM - Keypad Numeric Mode
                self.reset();
            }
            b'7' => {
                // DECSC - Save Cursor
                self.reset();
            }
            b'8' => {
                // DECRC - Restore Cursor
                self.reset();
            }
            b'c' => {
                // RIS - Reset to Initial State
                terminal.clear();
                terminal.attributes = Default::default();
                self.reset();
            }
            b'D' => {
                // IND - Index (move down, scroll if needed)
                terminal.newline();
                self.reset();
            }
            b'E' => {
                // NEL - Next Line
                terminal.cursor.col = 0;
                terminal.newline();
                self.reset();
            }
            b'M' => {
                // RI - Reverse Index (move up, scroll if needed)
                terminal.cursor_up(1);
                self.reset();
            }
            _ => {
                self.reset();
            }
        }
    }

    fn handle_escape_intermediate(&mut self, byte: u8, _terminal: &mut Terminal) {
        // Consume the character set designator (e.g., B for ASCII)
        if (0x30..=0x7E).contains(&byte) {
            self.reset();
        }
    }

    fn handle_csi_entry(&mut self, byte: u8, terminal: &mut Terminal) {
        match byte {
            b'0'..=b'9' => {
                self.current_param = Some((byte - b'0') as u16);
                self.state = State::CsiParam;
            }
            b';' => {
                self.params.push(0);
                self.state = State::CsiParam;
            }
            b'?' | b'>' | b'!' => {
                self.intermediates.push(byte);
            }
            0x40..=0x7E => {
                // Final byte
                self.dispatch_csi(byte, terminal);
                self.reset();
            }
            _ => {
                self.state = State::CsiParam;
            }
        }
    }

    fn handle_csi_param(&mut self, byte: u8, terminal: &mut Terminal) {
        match byte {
            b'0'..=b'9' => {
                let digit = (byte - b'0') as u16;
                self.current_param = Some(self.current_param.unwrap_or(0) * 10 + digit);
            }
            b';' => {
                self.params.push(self.current_param.unwrap_or(0));
                self.current_param = None;
            }
            b':' => {
                // Sub-parameter separator (SGR)
                self.params.push(self.current_param.unwrap_or(0));
                self.current_param = None;
            }
            0x20..=0x2F => {
                // Intermediate byte
                if let Some(p) = self.current_param.take() {
                    self.params.push(p);
                }
                self.intermediates.push(byte);
                self.state = State::CsiIntermediate;
            }
            0x40..=0x7E => {
                // Final byte
                if let Some(p) = self.current_param.take() {
                    self.params.push(p);
                }
                self.dispatch_csi(byte, terminal);
                self.reset();
            }
            _ => {}
        }
    }

    fn handle_csi_intermediate(&mut self, byte: u8, terminal: &mut Terminal) {
        match byte {
            0x20..=0x2F => {
                self.intermediates.push(byte);
            }
            0x40..=0x7E => {
                self.dispatch_csi(byte, terminal);
                self.reset();
            }
            _ => {}
        }
    }

    fn handle_osc_string(&mut self, byte: u8, terminal: &mut Terminal) {
        match byte {
            0x07 => {
                // BEL - OSC terminator
                self.dispatch_osc(terminal);
                self.reset();
            }
            0x1B => {
                // Could be ST (ESC \)
                // For now, just finish
                self.dispatch_osc(terminal);
                self.reset();
            }
            _ => {
                self.osc_string.push(byte);
            }
        }
    }

    fn handle_dcs(&mut self, byte: u8, _terminal: &mut Terminal) {
        // For now, just consume until ST
        if byte == 0x1B || byte == 0x9C {
            self.reset();
        }
    }

    fn dispatch_csi(&mut self, final_byte: u8, terminal: &mut Terminal) {
        // Clone params to avoid borrow issues
        let params: Vec<u16> = self.params.clone();
        let has_question = self.intermediates.contains(&b'?');

        match final_byte {
            b'A' => {
                // CUU - Cursor Up
                let n = params.first().copied().unwrap_or(1).max(1);
                terminal.cursor_up(n);
            }
            b'B' | b'e' => {
                // CUD - Cursor Down
                let n = params.first().copied().unwrap_or(1).max(1);
                terminal.cursor_down(n);
            }
            b'C' | b'a' => {
                // CUF - Cursor Forward
                let n = params.first().copied().unwrap_or(1).max(1);
                terminal.cursor_forward(n);
            }
            b'D' => {
                // CUB - Cursor Backward
                let n = params.first().copied().unwrap_or(1).max(1);
                terminal.cursor_backward(n);
            }
            b'E' => {
                // CNL - Cursor Next Line
                let n = params.first().copied().unwrap_or(1).max(1);
                terminal.cursor_down(n);
                terminal.cursor.col = 0;
            }
            b'F' => {
                // CPL - Cursor Previous Line
                let n = params.first().copied().unwrap_or(1).max(1);
                terminal.cursor_up(n);
                terminal.cursor.col = 0;
            }
            b'G' | b'`' => {
                // CHA - Cursor Horizontal Absolute
                let col = params.first().copied().unwrap_or(1);
                terminal.cursor.col = col.saturating_sub(1).min(terminal.cols - 1);
            }
            b'H' | b'f' => {
                // CUP - Cursor Position
                let row = params.first().copied().unwrap_or(1);
                let col = params.get(1).copied().unwrap_or(1);
                terminal.set_cursor(row, col);
            }
            b'J' => {
                // ED - Erase in Display
                let mode = params.first().copied().unwrap_or(0);
                match mode {
                    0 => terminal.clear_below(),
                    1 => terminal.clear_above(),
                    2 | 3 => terminal.clear(),
                    _ => {}
                }
            }
            b'K' => {
                // EL - Erase in Line
                let mode = params.first().copied().unwrap_or(0);
                match mode {
                    0 => terminal.clear_line_from_cursor(),
                    1 => terminal.clear_line_to_cursor(),
                    2 => terminal.clear_line(),
                    _ => {}
                }
            }
            b'L' => {
                // IL - Insert Lines
                let _n = params.first().copied().unwrap_or(1);
                // TODO: Implement insert lines
            }
            b'M' => {
                // DL - Delete Lines
                let _n = params.first().copied().unwrap_or(1);
                // TODO: Implement delete lines
            }
            b'P' => {
                // DCH - Delete Characters
                let n = params.first().copied().unwrap_or(1);
                terminal.delete_chars(n);
            }
            b'@' => {
                // ICH - Insert Characters
                let n = params.first().copied().unwrap_or(1);
                terminal.insert_chars(n);
            }
            b'S' => {
                // SU - Scroll Up
                let n = params.first().copied().unwrap_or(1);
                terminal.scroll_up(n as usize);
            }
            b'd' => {
                // VPA - Vertical Position Absolute
                let row = params.first().copied().unwrap_or(1);
                terminal.cursor.row = row.saturating_sub(1).min(terminal.rows - 1);
            }
            b'm' => {
                // SGR - Select Graphic Rendition
                self.handle_sgr(&params, terminal);
            }
            b'h' => {
                // SM - Set Mode
                if has_question {
                    self.handle_dec_mode(&params, true, terminal);
                }
            }
            b'l' => {
                // RM - Reset Mode
                if has_question {
                    self.handle_dec_mode(&params, false, terminal);
                }
            }
            b'n' => {
                // DSR - Device Status Report
                // Terminal should respond, but we'd need write access to PTY
            }
            b'r' => {
                // DECSTBM - Set Top and Bottom Margins
                // TODO: Implement scroll regions
            }
            b'c' => {
                // DA - Device Attributes
                // Terminal should respond
            }
            b't' => {
                // Window manipulation
                // Mostly ignored
            }
            _ => {
                debug!("Unhandled CSI: {:?} {:?}", params, final_byte as char);
            }
        }
    }

    fn handle_sgr(&mut self, params: &[u16], terminal: &mut Terminal) {
        if params.is_empty() {
            // Reset all attributes
            terminal.attributes = Default::default();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            let param = params[i];
            match param {
                0 => {
                    terminal.attributes = Default::default();
                }
                1 => terminal.attributes.bold = true,
                3 => terminal.attributes.italic = true,
                4 => terminal.attributes.underline = true,
                22 => terminal.attributes.bold = false,
                23 => terminal.attributes.italic = false,
                24 => terminal.attributes.underline = false,
                
                // Foreground colors (30-37)
                30..=37 => {
                    let idx = (param - 30) as usize;
                    terminal.attributes.fg_color = ANSI_COLORS[idx];
                }
                38 => {
                    // Extended foreground color
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 => {
                                // 256-color
                                if i + 2 < params.len() {
                                    let color = params[i + 2];
                                    terminal.attributes.fg_color = color_256(color);
                                    i += 2;
                                }
                            }
                            2 => {
                                // RGB
                                if i + 4 < params.len() {
                                    let r = params[i + 2] as f32 / 255.0;
                                    let g = params[i + 3] as f32 / 255.0;
                                    let b = params[i + 4] as f32 / 255.0;
                                    terminal.attributes.fg_color = [r, g, b];
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                39 => {
                    // Default foreground
                    terminal.attributes.fg_color = [0.9, 0.9, 0.9];
                }
                
                // Background colors (40-47)
                40..=47 => {
                    let idx = (param - 40) as usize;
                    terminal.attributes.bg_color = ANSI_COLORS[idx];
                }
                48 => {
                    // Extended background color
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 => {
                                // 256-color
                                if i + 2 < params.len() {
                                    let color = params[i + 2];
                                    terminal.attributes.bg_color = color_256(color);
                                    i += 2;
                                }
                            }
                            2 => {
                                // RGB
                                if i + 4 < params.len() {
                                    let r = params[i + 2] as f32 / 255.0;
                                    let g = params[i + 3] as f32 / 255.0;
                                    let b = params[i + 4] as f32 / 255.0;
                                    terminal.attributes.bg_color = [r, g, b];
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                49 => {
                    // Default background
                    terminal.attributes.bg_color = [0.1, 0.1, 0.12];
                }
                
                // Bright foreground colors (90-97)
                90..=97 => {
                    let idx = (param - 90 + 8) as usize;
                    terminal.attributes.fg_color = ANSI_COLORS[idx];
                }
                
                // Bright background colors (100-107)
                100..=107 => {
                    let idx = (param - 100 + 8) as usize;
                    terminal.attributes.bg_color = ANSI_COLORS[idx];
                }
                
                _ => {}
            }
            i += 1;
        }
    }

    fn handle_dec_mode(&mut self, params: &[u16], enable: bool, terminal: &mut Terminal) {
        for &param in params {
            match param {
                25 => {
                    // DECTCEM - Cursor visibility
                    terminal.cursor.visible = enable;
                }
                1049 => {
                    // Alternate screen buffer
                    if enable {
                        terminal.enter_alt_screen();
                    } else {
                        terminal.exit_alt_screen();
                    }
                }
                1 => {
                    // DECCKM - Cursor keys mode
                }
                7 => {
                    // DECAWM - Auto wrap mode
                }
                _ => {
                    debug!("Unhandled DEC mode: {} = {}", param, enable);
                }
            }
        }
    }

    fn dispatch_osc(&mut self, terminal: &mut Terminal) {
        let osc_str = String::from_utf8_lossy(&self.osc_string);
        let parts: Vec<&str> = osc_str.splitn(2, ';').collect();

        if let Some(cmd) = parts.first() {
            match *cmd {
                "0" | "2" => {
                    // Set window title
                    if let Some(title) = parts.get(1) {
                        debug!("Window title: {}", title);
                    }
                }
                "1337" => {
                    // iTerm2 protocol - used for images, pixel graphics, and PTUI
                    if let Some(data) = parts.get(1) {
                        // Check if it's a PTUI message
                        if let Some(ptui_data) = data.strip_prefix("PTUI=") {
                            // Store PTUI message for processing
                            terminal.pending_ptui.push(ptui_data.to_string());
                        } else {
                            // Regular iTerm2 image protocol
                            terminal.pixel_canvas.handle_iterm2_sequence(data);
                        }
                    }
                }
                _ => {
                    debug!("Unhandled OSC: {:?}", parts);
                }
            }
        }
    }
}

/// Convert 256-color index to RGB
fn color_256(idx: u16) -> [f32; 3] {
    if idx < 16 {
        return ANSI_COLORS[idx as usize];
    }

    if idx < 232 {
        // 6x6x6 color cube
        let idx = idx - 16;
        let r = (idx / 36) % 6;
        let g = (idx / 6) % 6;
        let b = idx % 6;

        let to_float = |v: u16| {
            if v == 0 {
                0.0
            } else {
                (v * 40 + 55) as f32 / 255.0
            }
        };

        return [to_float(r), to_float(g), to_float(b)];
    }

    // Grayscale (232-255)
    let gray = (idx - 232) * 10 + 8;
    let v = gray as f32 / 255.0;
    [v, v, v]
}
