use std::fs;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style as SynStyle};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::git::{BlameLine, DiffKind, GitStatus};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    File,
    Diff(DiffKind),
    Blame,
}

pub struct OpenFile {
    pub path: PathBuf,
    pub title: String,
    pub lines: Vec<Line<'static>>,
    pub scroll: u16,
    pub mode: ViewMode,
}

pub struct FileViewer {
    pub tabs: Vec<OpenFile>,
    pub active: usize,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl FileViewer {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn active(&self) -> Option<&OpenFile> {
        self.tabs.get(self.active)
    }

    pub fn active_mut(&mut self) -> Option<&mut OpenFile> {
        self.tabs.get_mut(self.active)
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.active().map(|t| &t.path)
    }

    pub fn title(&self) -> &str {
        self.active()
            .map(|t| t.title.as_str())
            .unwrap_or("No file open")
    }

    pub fn show_diff(&self) -> bool {
        matches!(self.mode(), ViewMode::Diff(_))
    }

    pub fn mode(&self) -> ViewMode {
        self.active().map(|t| t.mode).unwrap_or(ViewMode::File)
    }

    pub fn mode_label(&self) -> &'static str {
        match self.mode() {
            ViewMode::File => "EDITOR",
            ViewMode::Diff(kind) => kind.label(),
            ViewMode::Blame => "BLAME",
        }
    }

    pub fn lines(&self) -> &[Line<'static>] {
        self.active()
            .map(|t| t.lines.as_slice())
            .unwrap_or(&[])
    }

    pub fn scroll(&self) -> u16 {
        self.active().map(|t| t.scroll).unwrap_or(0)
    }

    pub fn open(&mut self, path: &Path, root: &Path) {
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.active = idx;
            return;
        }
        let title = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let mut tab = OpenFile {
            path: path.to_path_buf(),
            title,
            lines: Vec::new(),
            scroll: 0,
            mode: ViewMode::File,
        };
        reload_tab(&mut tab, root, &self.syntax_set, &self.theme_set);
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    pub fn close_active(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        self.tabs.remove(self.active);
        if self.tabs.is_empty() {
            self.active = 0;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        true
    }

    pub fn close_at(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return false;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active = 0;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if index < self.active {
            self.active -= 1;
        }
        true
    }

    pub fn reload_open(&mut self, root: &Path) {
        if self.tabs.is_empty() {
            return;
        }
        let idx = self.active;
        reload_tab(
            &mut self.tabs[idx],
            root,
            &self.syntax_set,
            &self.theme_set,
        );
    }

    pub fn reload_path(&mut self, path: &Path, root: &Path) {
        let syntax_set = &self.syntax_set;
        let theme_set = &self.theme_set;
        for tab in &mut self.tabs {
            if tab.path == path {
                reload_tab(tab, root, syntax_set, theme_set);
            }
        }
    }

    /// Cycle file → HEAD diff → unstaged → staged → file.
    pub fn toggle_diff(&mut self, root: &Path) {
        if self.tabs.is_empty() {
            return;
        }
        let idx = self.active;
        self.tabs[idx].mode = match self.tabs[idx].mode {
            ViewMode::File | ViewMode::Blame => ViewMode::Diff(DiffKind::first()),
            ViewMode::Diff(kind) => match kind.cycle() {
                Some(next) => ViewMode::Diff(next),
                None => ViewMode::File,
            },
        };
        self.tabs[idx].scroll = 0;
        reload_tab(
            &mut self.tabs[idx],
            root,
            &self.syntax_set,
            &self.theme_set,
        );
    }

    pub fn set_diff_kind(&mut self, root: &Path, kind: DiffKind) {
        if self.tabs.is_empty() {
            return;
        }
        let idx = self.active;
        self.tabs[idx].mode = ViewMode::Diff(kind);
        self.tabs[idx].scroll = 0;
        reload_tab(
            &mut self.tabs[idx],
            root,
            &self.syntax_set,
            &self.theme_set,
        );
    }

    pub fn toggle_blame(&mut self, root: &Path) {
        if self.tabs.is_empty() {
            return;
        }
        let idx = self.active;
        self.tabs[idx].mode = match self.tabs[idx].mode {
            ViewMode::Blame => ViewMode::File,
            _ => ViewMode::Blame,
        };
        self.tabs[idx].scroll = 0;
        reload_tab(
            &mut self.tabs[idx],
            root,
            &self.syntax_set,
            &self.theme_set,
        );
    }

    pub fn show_blame(&mut self, root: &Path) {
        if self.tabs.is_empty() {
            return;
        }
        let idx = self.active;
        self.tabs[idx].mode = ViewMode::Blame;
        self.tabs[idx].scroll = 0;
        reload_tab(
            &mut self.tabs[idx],
            root,
            &self.syntax_set,
            &self.theme_set,
        );
    }

    pub fn scroll_by(&mut self, delta: i32, height: u16) {
        let Some(tab) = self.active_mut() else {
            return;
        };
        let max = tab.lines.len().saturating_sub(height as usize) as u16;
        let next = tab.scroll as i32 + delta;
        tab.scroll = next.clamp(0, max as i32) as u16;
    }

    pub fn set_scroll(&mut self, scroll: u16) {
        if let Some(tab) = self.active_mut() {
            tab.scroll = scroll;
        }
    }
}

fn reload_tab(tab: &mut OpenFile, root: &Path, syntax_set: &SyntaxSet, theme_set: &ThemeSet) {
    match tab.mode {
        ViewMode::Diff(kind) => {
            let diff = GitStatus::diff_kind(root, &tab.path, kind)
                .unwrap_or_else(|| "(unable to read git diff)".to_string());
            tab.lines = diff.lines().map(colorize_diff_line).collect();
            return;
        }
        ViewMode::Blame => {
            tab.lines = match GitStatus::blame(root, &tab.path) {
                Some(blame) if !blame.is_empty() => blame.into_iter().map(colorize_blame_line).collect(),
                Some(_) => vec![Line::from("(empty blame)")],
                None => vec![Line::from("(unable to read git blame — untracked?)")],
            };
            return;
        }
        ViewMode::File => {}
    }

    match fs::read(&tab.path) {
        Ok(bytes) if bytes.len() > 2_000_000 => {
            tab.lines = vec![Line::from(format!(
                "File too large to preview ({} bytes). Use the terminal.",
                bytes.len()
            ))];
        }
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            tab.lines = highlight_file(&text, &tab.path, syntax_set, theme_set);
        }
        Err(err) => {
            tab.lines = vec![Line::from(format!("Error reading file: {err}"))];
        }
    }
}

fn highlight_file(
    text: &str,
    path: &Path,
    syntax_set: &SyntaxSet,
    theme_set: &ThemeSet,
) -> Vec<Line<'static>> {
    let syntax = resolve_syntax(path, syntax_set);
    // mocha is the most vibrant of syntect's built-in dark themes
    let theme = theme_set
        .themes
        .get("base16-mocha.dark")
        .or_else(|| theme_set.themes.get("base16-eighties.dark"))
        .or_else(|| theme_set.themes.get("Solarized (dark)"))
        .or_else(|| theme_set.themes.values().next())
        .expect("syntect theme");
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut out = Vec::new();

    for (idx, line) in LinesWithEndings::from(text).enumerate() {
        let ranges = highlighter
            .highlight_line(line, syntax_set)
            .unwrap_or_default();
        let mut spans = vec![Span::styled(
            format!("{:>5} │ ", idx + 1),
            Style::default().fg(Color::Rgb(70, 78, 90)).bg(Theme::get().bg),
        )];
        for (style, piece) in ranges {
            let piece = piece.trim_end_matches(['\r', '\n']);
            if piece.is_empty() {
                continue;
            }
            spans.push(Span::styled(
                piece.to_string(),
                syn_to_ratatui(style),
            ));
        }
        // Ensure full-width bg so lines don't look patchy
        spans.push(Span::styled(" ", Style::default().bg(Theme::get().bg)));
        out.push(Line::from(spans));
    }
    if out.is_empty() {
        out.push(Line::from(""));
    }
    out
}

fn resolve_syntax<'a>(path: &Path, syntax_set: &'a SyntaxSet) -> &'a syntect::parsing::SyntaxReference {
    if let Ok(Some(syntax)) = syntax_set.find_syntax_for_file(path) {
        return syntax;
    }

    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Explicit aliases syntect sometimes misses by extension alone
    let by_name = if name.ends_with(".tsx") || ext == "tsx" {
        syntax_set
            .find_syntax_by_name("TSX")
            .or_else(|| syntax_set.find_syntax_by_extension("tsx"))
            .or_else(|| syntax_set.find_syntax_by_name("TypeScript"))
    } else if name.ends_with(".ts") || ext == "ts" || ext == "mts" || ext == "cts" {
        syntax_set
            .find_syntax_by_name("TypeScript")
            .or_else(|| syntax_set.find_syntax_by_extension("ts"))
    } else if name.ends_with(".jsx") || ext == "jsx" {
        syntax_set
            .find_syntax_by_name("JSX")
            .or_else(|| syntax_set.find_syntax_by_extension("jsx"))
            .or_else(|| syntax_set.find_syntax_by_name("JavaScript"))
    } else if ext == "mjs" || ext == "cjs" {
        syntax_set.find_syntax_by_name("JavaScript")
    } else if name == "dockerfile" || name.starts_with("dockerfile") {
        syntax_set.find_syntax_by_name("Dockerfile")
    } else if name == "makefile" || name.starts_with("makefile") {
        syntax_set.find_syntax_by_name("Makefile")
    } else if ext == "yml" || ext == "yaml" {
        syntax_set
            .find_syntax_by_name("YAML")
            .or_else(|| syntax_set.find_syntax_by_extension("yaml"))
    } else if ext == "toml" {
        syntax_set
            .find_syntax_by_name("TOML")
            .or_else(|| syntax_set.find_syntax_by_extension("toml"))
    } else if ext == "md" || ext == "mdx" || ext == "markdown" {
        syntax_set
            .find_syntax_by_name("Markdown")
            .or_else(|| syntax_set.find_syntax_by_extension("md"))
    } else if !ext.is_empty() {
        syntax_set.find_syntax_by_extension(&ext)
    } else {
        None
    };

    by_name.unwrap_or_else(|| syntax_set.find_syntax_plain_text())
}

/// Map syntect colors onto our dark editor with punchy contrast.
fn syn_to_ratatui(style: SynStyle) -> Style {
    let fg = style.foreground;
    let (r, g, b) = punch_up_color(fg.r, fg.g, fg.b);

    let mut out = Style::default().fg(Color::Rgb(r, g, b)).bg(Theme::get().bg);
    let fs = style.font_style;
    if fs.contains(syntect::highlighting::FontStyle::BOLD) {
        out = out.add_modifier(Modifier::BOLD);
    }
    if fs.contains(syntect::highlighting::FontStyle::ITALIC) {
        out = out.add_modifier(Modifier::ITALIC);
    }
    if fs.contains(syntect::highlighting::FontStyle::UNDERLINE) {
        out = out.add_modifier(Modifier::UNDERLINED);
    }
    out
}

/// Lift dark/muddy syntect colors so they read clearly on Theme::get().bg.
fn punch_up_color(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let rf = r as f32;
    let gf = g as f32;
    let bf = b as f32;
    let lum = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;

    // Near-black / too dim → lift toward readable mid tones
    let (mut rf, mut gf, mut bf) = if lum < 95.0 {
        let factor = (130.0 / lum.max(8.0)).min(3.0);
        (
            (rf * factor).min(255.0),
            (gf * factor).min(255.0),
            (bf * factor).min(255.0),
        )
    } else {
        (rf, gf, bf)
    };

    // Saturate for clearer keyword / string / type separation
    let avg = (rf + gf + bf) / 3.0;
    let sat = 1.35;
    rf = (avg + (rf - avg) * sat).clamp(0.0, 255.0);
    gf = (avg + (gf - avg) * sat).clamp(0.0, 255.0);
    bf = (avg + (bf - avg) * sat).clamp(0.0, 255.0);

    // Slight global lift so nothing sits in the mud
    rf = (rf + 12.0).min(255.0);
    gf = (gf + 12.0).min(255.0);
    bf = (bf + 12.0).min(255.0);

    (rf as u8, gf as u8, bf as u8)
}

fn colorize_diff_line(line: &str) -> Line<'static> {
    let style = if line.starts_with('+') && !line.starts_with("+++") {
        Style::default().fg(Theme::get().git_add).bg(Theme::get().bg)
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default().fg(Theme::get().git_del).bg(Theme::get().bg)
    } else if line.starts_with("@@") {
        Style::default().fg(Theme::get().accent_glow).bg(Theme::get().bg)
    } else {
        Style::default().fg(Theme::get().fg_dim).bg(Theme::get().bg)
    };
    Line::from(Span::styled(line.to_string(), style))
}

fn colorize_blame_line(line: BlameLine) -> Line<'static> {
    let meta = format!("{:<7} {:<12} ", line.short_sha, truncate_str(&line.author, 12));
    Line::from(vec![
        Span::styled(
            meta,
            Style::default().fg(Theme::get().fg_muted).bg(Theme::get().bg),
        ),
        Span::styled(
            "│ ",
            Style::default().fg(Theme::get().border).bg(Theme::get().bg),
        ),
        Span::styled(
            line.content,
            Style::default().fg(Theme::get().fg).bg(Theme::get().bg),
        ),
    ])
}

fn truncate_str(s: &str, max: usize) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max {
            break;
        }
        out.push(ch);
    }
    if s.chars().count() > max {
        // pad already truncated — keep fixed width feel
    }
    while out.chars().count() < max {
        out.push(' ');
    }
    out
}
