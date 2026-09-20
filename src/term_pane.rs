use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use vt100::Parser;

use crate::theme::Theme;

pub enum PtyEvent {
    Output(Vec<u8>),
    Exit,
}

pub struct TermPane {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    pub parser: Parser,
    pub rx: Receiver<PtyEvent>,
    pub title: String,
    pub alive: bool,
    rows: u16,
    cols: u16,
    _child_killer: Box<dyn portable_pty::Child + Send>,
}

impl TermPane {
    pub fn spawn(cwd: &Path, shell: &str, rows: u16, cols: u16, title: String) -> Result<Self> {
        let rows = rows.max(2);
        let cols = cols.max(10);
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("open pty")?;

        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(cwd);
        // Agent-friendly terminal capabilities
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", "termide");

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("spawn shell in pty")?;

        let mut reader = pair.master.try_clone_reader().context("clone pty reader")?;
        let writer = pair.master.take_writer().context("take pty writer")?;
        let (tx, rx): (Sender<PtyEvent>, Receiver<PtyEvent>) = mpsc::channel();

        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = tx.send(PtyEvent::Exit);
                        break;
                    }
                    Ok(n) => {
                        if tx.send(PtyEvent::Output(buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = tx.send(PtyEvent::Exit);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            // Large scrollback for long Codex / Claude sessions
            parser: Parser::new(rows, cols, 10_000),
            rx,
            title,
            alive: true,
            rows,
            cols,
            _child_killer: child,
        })
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let rows = rows.max(2);
        let cols = cols.max(10);
        if self.rows == rows && self.cols == cols {
            return;
        }
        self.rows = rows;
        self.cols = cols;
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.parser.set_size(rows, cols);
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        if !self.alive {
            return;
        }
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    pub fn write_char(&mut self, c: char) {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.write_bytes(s.as_bytes());
    }

    pub fn drain_output(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                PtyEvent::Output(bytes) => self.parser.process(&bytes),
                PtyEvent::Exit => {
                    self.alive = false;
                }
            }
        }
    }

    /// Cursor position as (row, col) within the visible screen.
    pub fn cursor_pos(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    /// Render screen cells. When `draw_caret` is true, paints a block caret
    /// at the PTY cursor (blinks when `blink_on`).
    pub fn lines(&self, width: u16, height: u16, draw_caret: bool, blink_on: bool) -> Vec<Line<'static>> {
        let screen = self.parser.screen();
        let mut out = Vec::with_capacity(height as usize);
        let rows = height.min(screen.size().0);
        let cols = width.min(screen.size().1);
        let default_style = Style::default().bg(Theme::get().panel).fg(Theme::get().fg);
        let (cursor_row, cursor_col) = screen.cursor_position();
        let show_block = draw_caret && blink_on;

        for row in 0..rows {
            let mut spans = Vec::new();
            let mut col = 0u16;
            while col < cols {
                let cell = screen.cell(row, col);
                let (mut ch, mut style) = match cell {
                    Some(cell) => {
                        let ch = cell.contents();
                        let ch = if ch.is_empty() {
                            " ".to_string()
                        } else {
                            ch
                        };
                        let fg = ansi_color(cell.fgcolor());
                        let bg = match cell.bgcolor() {
                            vt100::Color::Default => Theme::get().panel,
                            other => ansi_color(other),
                        };
                        let mut style = Style::default().fg(fg).bg(bg);
                        if cell.bold() {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        if cell.underline() {
                            style = style.add_modifier(Modifier::UNDERLINED);
                        }
                        (ch, style)
                    }
                    None => (" ".to_string(), default_style),
                };

                // Block caret — high-contrast so you always see where you type
                if show_block && row == cursor_row && col == cursor_col {
                    style = Style::default()
                        .fg(Theme::get().panel)
                        .bg(Theme::get().accent_glow)
                        .add_modifier(Modifier::BOLD);
                    if ch.trim().is_empty() {
                        ch = " ".to_string();
                    }
                }

                let cell_w = unicode_width::UnicodeWidthStr::width(ch.as_str()).max(1) as u16;
                spans.push(Span::styled(ch, style));
                col = col.saturating_add(cell_w);
            }
            while col < width {
                let is_caret = show_block && row == cursor_row && col == cursor_col;
                let style = if is_caret {
                    Style::default().fg(Theme::get().panel).bg(Theme::get().accent_glow)
                } else {
                    default_style
                };
                spans.push(Span::styled(" ", style));
                col += 1;
            }
            out.push(Line::from(spans));
        }
        while out.len() < height as usize {
            out.push(Line::from(Span::styled(
                " ".repeat(width as usize),
                default_style,
            )));
        }
        out
    }
}

fn ansi_color(c: vt100::Color) -> Color {
    match c {
        vt100::Color::Default => Theme::get().fg,
        vt100::Color::Idx(n) => Color::Indexed(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
