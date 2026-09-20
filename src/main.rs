mod app;
mod git;
mod search;
mod term_pane;
mod theme;
mod tree;
mod viewer;
mod watch;

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    cursor::Hide,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::App;

#[derive(Debug, Parser)]
#[command(name = "noir", about = "Lightweight terminal IDE for Codex workflows")]
struct Cli {
    /// Workspace root to open (defaults to cwd)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Shell command for the terminal pane (default: $SHELL or /bin/zsh)
    #[arg(long, short = 'c')]
    shell: Option<String>,

    /// Color theme: github-dark, dracula, catppuccin, nord, tokyo-night, gruvbox, one-dark, solarized
    #[arg(long, short = 't')]
    theme: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.path.canonicalize().unwrap_or(cli.path);
    let shell = cli
        .shell
        .unwrap_or_else(|| std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string()));

    crate::theme::Theme::load_saved();
    if let Some(name) = cli.theme.as_deref() {
        if let Some(id) = crate::theme::PaletteId::from_id(name) {
            crate::theme::Theme::set(id);
        } else {
            eprintln!(
                "Unknown theme '{name}'. Options: github-dark, dracula, catppuccin, nord, tokyo-night, gruvbox, one-dark, solarized"
            );
            std::process::exit(2);
        }
    }

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;
    // Any-event mouse tracking — required for hover / pointer without click
    write!(stdout, "\x1b[?1003h")?;
    stdout.flush()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(root, shell)?;
    let result = app.run(&mut terminal);

    // Restore terminal state
    let _ = write!(terminal.backend_mut(), "\x1b[?1003l");
    let _ = terminal.backend_mut().flush();
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
