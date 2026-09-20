use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::git::{DiffKind, GitBranch, GitEntry, GitStatus};
use crate::search::{FileIndex, SearchHit};
use crate::term_pane::TermPane;
use crate::theme::{PaletteId, Theme};
use crate::tree::FileTree;
use crate::viewer::FileViewer;
use crate::watch::{spawn_watcher, WatchEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Tree,
    Viewer,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitPaneTab {
    Changes,
    Branches,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitKind {
    FileTab { index: usize },
    FileClose { index: usize },
    TermTab { index: usize },
    TermClose { index: usize },
    TermNew,
    TreeRow,
    ViewerBody,
    TermBody,
    /// Vertical splitter between explorer and editor
    VSplit,
    /// Horizontal splitter between editor and terminal
    HSplit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragKind {
    VSplit,
    HSplit,
}

struct Hit {
    kind: HitKind,
    area: Rect,
}

pub struct App {
    root: PathBuf,
    shell: String,
    tree: FileTree,
    viewer: FileViewer,
    terminals: Vec<TermPane>,
    active_term: usize,
    next_term_id: usize,
    focus: Focus,
    branch: String,
    watch_rx: Receiver<WatchEvent>,
    show_tree: bool,
    show_git: bool,
    git_tab: GitPaneTab,
    git_entries: Vec<GitEntry>,
    git_branches: Vec<GitBranch>,
    git_cursor: usize,
    git_scroll: usize,
    last_git_refresh: Instant,
    term_area: Rect,
    term_body_area: Rect,
    tree_area: Rect,
    tree_inner: Rect,
    viewer_area: Rect,
    viewer_body: Rect,
    hits: Vec<Hit>,
    hover: Option<HitKind>,
    pointer_on: bool,
    mouse_col: u16,
    mouse_row: u16,
    tree_hover_row: Option<usize>,
    blink_started: Instant,
    /// Explorer column width in columns
    tree_width: u16,
    /// Percent of main area given to the top (explorer+editor) vs terminal
    top_pct: u16,
    drag: Option<DragKind>,
    v_split_area: Rect,
    h_split_area: Rect,
    main_area: Rect,
    should_quit: bool,
    show_help: bool,
    show_themes: bool,
    theme_cursor: usize,
    show_search: bool,
    search_query: String,
    search_cursor: usize,
    search_hits: Vec<SearchHit>,
    file_index: FileIndex,
    status: String,
}

impl App {
    pub fn new(root: PathBuf, shell: String) -> Result<Self> {
        Theme::load_saved();
        let git = GitStatus::load(&root);
        let git_branches = GitStatus::branches(&root);
        let tree = FileTree::new(root.clone(), git.map);
        let watch_rx = spawn_watcher(&root)?;
        let first = TermPane::spawn(&root, &shell, 20, 80, "zsh · 1".into())?;

        Ok(Self {
            root,
            shell,
            tree,
            viewer: FileViewer::new(),
            terminals: vec![first],
            active_term: 0,
            next_term_id: 2,
            focus: Focus::Tree,
            branch: git.branch,
            watch_rx,
            show_tree: true,
            show_git: false,
            git_tab: GitPaneTab::Changes,
            git_entries: git.entries,
            git_branches,
            git_cursor: 0,
            git_scroll: 0,
            last_git_refresh: Instant::now(),
            term_area: Rect::default(),
            term_body_area: Rect::default(),
            tree_area: Rect::default(),
            tree_inner: Rect::default(),
            viewer_area: Rect::default(),
            viewer_body: Rect::default(),
            hits: Vec::new(),
            hover: None,
            pointer_on: false,
            mouse_col: 0,
            mouse_row: 0,
            tree_hover_row: None,
            blink_started: Instant::now(),
            tree_width: 34,
            top_pct: 42,
            drag: None,
            v_split_area: Rect::default(),
            h_split_area: Rect::default(),
            main_area: Rect::default(),
            should_quit: false,
            show_help: false,
            show_themes: false,
            theme_cursor: Theme::id().index(),
            show_search: false,
            search_query: String::new(),
            search_cursor: 0,
            search_hits: Vec::new(),
            file_index: FileIndex::default(),
            status: "Press ? help · Ctrl+O search · Ctrl+P themes".into(),
        })
    }

    fn open_search(&mut self) {
        if self.file_index.is_stale(30) || self.file_index.paths.is_empty() {
            self.file_index.rebuild(&self.root);
        }
        self.show_help = false;
        self.show_themes = false;
        self.show_search = true;
        self.search_query.clear();
        self.search_cursor = 0;
        self.refresh_search_hits();
        self.status = format!(
            "Search files · {} indexed · type to filter",
            self.file_index.paths.len()
        );
    }

    fn refresh_search_hits(&mut self) {
        self.search_hits = self.file_index.search(&self.root, &self.search_query, 40);
        if self.search_cursor >= self.search_hits.len() {
            self.search_cursor = self.search_hits.len().saturating_sub(1);
        }
    }

    fn confirm_search(&mut self) {
        let Some(hit) = self.search_hits.get(self.search_cursor).cloned() else {
            self.status = "No matching files".into();
            return;
        };
        self.show_search = false;
        self.open_file(hit.abs);
    }

    fn active_term_mut(&mut self) -> &mut TermPane {
        &mut self.terminals[self.active_term]
    }

    fn add_terminal(&mut self) {
        let id = self.next_term_id;
        self.next_term_id += 1;
        let title = format!("zsh · {id}");
        let rows = self.term_body_area.height.max(2);
        let cols = self.term_body_area.width.max(10);
        match TermPane::spawn(&self.root, &self.shell, rows, cols, title) {
            Ok(pane) => {
                self.terminals.push(pane);
                self.active_term = self.terminals.len() - 1;
                self.focus = Focus::Terminal;
                self.status = format!("● {}", self.terminals[self.active_term].title);
            }
            Err(err) => self.status = format!("Failed to open terminal: {err}"),
        }
    }

    fn close_terminal_at(&mut self, index: usize) {
        if self.terminals.len() <= 1 {
            self.status = "Keep at least one terminal".into();
            return;
        }
        if index >= self.terminals.len() {
            return;
        }
        let closed = self.terminals[index].title.clone();
        self.terminals.remove(index);
        if self.active_term >= self.terminals.len() {
            self.active_term = self.terminals.len() - 1;
        } else if index < self.active_term {
            self.active_term -= 1;
        }
        self.status = format!("Closed {closed}");
    }

    fn close_active_terminal(&mut self) {
        self.close_terminal_at(self.active_term);
    }

    fn next_terminal(&mut self) {
        if self.terminals.is_empty() {
            return;
        }
        self.active_term = (self.active_term + 1) % self.terminals.len();
        self.focus = Focus::Terminal;
    }

    fn prev_terminal(&mut self) {
        if self.terminals.is_empty() {
            return;
        }
        self.active_term = if self.active_term == 0 {
            self.terminals.len() - 1
        } else {
            self.active_term - 1
        };
        self.focus = Focus::Terminal;
    }

    fn set_pointer(&mut self, on: bool) {
        self.pointer_on = on;
    }

    /// Recompute pointer from last mouse position against current hit targets.
    fn sync_pointer_from_mouse(&mut self) {
        if self.drag.is_some() {
            self.pointer_on = true;
            return;
        }
        let col = self.mouse_col;
        let row = self.mouse_row;
        let over = self.is_clickable_at(col, row);
        self.pointer_on = over;
        if point_in(self.tree_inner, col, row) {
            let y = row.saturating_sub(self.tree_inner.y) as usize;
            let scroll = if self.show_git {
                self.git_scroll
            } else {
                self.tree.scroll
            };
            self.tree_hover_row = Some(scroll + y);
        } else {
            self.tree_hover_row = None;
        }
    }

    fn is_clickable_at(&self, col: u16, row: u16) -> bool {
        if point_in(self.v_split_area, col, row) || point_in(self.h_split_area, col, row) {
            return true;
        }
        if point_in(self.tree_inner, col, row) {
            return true;
        }
        self.hits.iter().any(|h| {
            point_in(h.area, col, row)
                && !matches!(h.kind, HitKind::ViewerBody | HitKind::TermBody)
        })
    }

    /// Force OSC mouse-cursor shape through the backend after every frame.
    /// Ratatui redraws otherwise reset it; many terminals need ST *and* BEL forms.
    fn emit_pointer_escape<W: Write>(&self, out: &mut W) {
        // Keep all-motion mouse tracking alive (some hosts drop 1003).
        let _ = write!(out, "\x1b[?1003h");
        if self.pointer_on {
            // OSC 22 — pointer / hand (Kitty, WezTerm, Ghostty, iTerm2 3.5+, WT)
            let _ = write!(
                out,
                "\x1b]22;pointer\x07\
                 \x1b]22;pointer\x1b\\\
                 \x1b]22;hand\x07\
                 \x1b]22;hand\x1b\\"
            );
        } else {
            let _ = write!(
                out,
                "\x1b]22;default\x07\
                 \x1b]22;default\x1b\\\
                 \x1b]22;text\x07\
                 \x1b]22;text\x1b\\"
            );
        }
        let _ = out.flush();
    }

    pub fn run<B>(&mut self, terminal: &mut Terminal<B>) -> Result<()>
    where
        B: ratatui::backend::Backend + Write,
    {
        while !self.should_quit {
            for term in &mut self.terminals {
                term.drain_output();
            }
            self.poll_watch();
            self.maybe_refresh_git();

            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key(key),
                    Event::Mouse(mouse) => self.on_mouse(mouse),
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }

            terminal.draw(|f| self.ui(f))?;
            // Hits exist only after draw — sync pointer, then re-assert OSC every frame
            self.sync_pointer_from_mouse();
            self.emit_pointer_escape(terminal.backend_mut());

            let rows = self.term_body_area.height.max(2);
            let cols = self.term_body_area.width.max(10);
            for term in &mut self.terminals {
                term.resize(rows, cols);
            }
        }
        self.pointer_on = false;
        self.emit_pointer_escape(terminal.backend_mut());
        Ok(())
    }

    fn poll_watch(&mut self) {
        let mut touch_git = false;
        while let Ok(ev) = self.watch_rx.try_recv() {
            match ev {
                WatchEvent::Changed(path) => {
                    touch_git = true;
                    self.viewer.reload_path(&path, &self.root);
                    // Paths may have been added/removed — refresh on next search open
                    self.file_index.built_at_invalidate();
                    if self.viewer.path().is_some_and(|p| p == &path) {
                        self.status = "● Reloaded from disk".into();
                    }
                }
            }
        }
        if touch_git && self.last_git_refresh.elapsed() > Duration::from_millis(400) {
            self.refresh_git();
        }
    }

    fn maybe_refresh_git(&mut self) {
        if self.last_git_refresh.elapsed() > Duration::from_secs(3) {
            self.refresh_git();
        }
    }

    fn refresh_git(&mut self) {
        let git = GitStatus::load(&self.root);
        self.branch = git.branch;
        self.tree.set_git(git.map);
        self.git_entries = git.entries;
        if self.show_git {
            self.git_branches = GitStatus::branches(&self.root);
        }
        self.clamp_git_cursor();
        self.last_git_refresh = Instant::now();
    }

    fn open_git_pane(&mut self) {
        self.show_tree = true;
        self.show_git = true;
        self.show_help = false;
        self.show_themes = false;
        self.show_search = false;
        self.focus = Focus::Tree;
        self.git_branches = GitStatus::branches(&self.root);
        self.refresh_git();
        self.status = "Git · s stage · u unstage · b branches · l blame".into();
    }

    fn close_git_pane(&mut self) {
        self.show_git = false;
        self.status = "Explorer · click a file to open".into();
    }

    fn clamp_git_cursor(&mut self) {
        let len = match self.git_tab {
            GitPaneTab::Changes => self.git_entries.len(),
            GitPaneTab::Branches => self.git_branches.len(),
        };
        if len == 0 {
            self.git_cursor = 0;
            self.git_scroll = 0;
            return;
        }
        if self.git_cursor >= len {
            self.git_cursor = len - 1;
        }
    }

    fn git_list_len(&self) -> usize {
        match self.git_tab {
            GitPaneTab::Changes => self.git_entries.len(),
            GitPaneTab::Branches => self.git_branches.len(),
        }
    }

    fn ensure_git_visible(&mut self, height: usize) {
        if height == 0 {
            return;
        }
        if self.git_cursor < self.git_scroll {
            self.git_scroll = self.git_cursor;
        } else if self.git_cursor >= self.git_scroll + height {
            self.git_scroll = self.git_cursor + 1 - height;
        }
    }

    fn stage_selected(&mut self) {
        let Some(entry) = self.git_entries.get(self.git_cursor).cloned() else {
            self.status = "No file selected".into();
            return;
        };
        match GitStatus::stage(&self.root, &entry.path) {
            Ok(()) => {
                self.status = format!("Staged · {}", entry.rel);
                self.refresh_git();
            }
            Err(err) => self.status = format!("Stage failed · {err}"),
        }
    }

    fn unstage_selected(&mut self) {
        let Some(entry) = self.git_entries.get(self.git_cursor).cloned() else {
            self.status = "No file selected".into();
            return;
        };
        match GitStatus::unstage(&self.root, &entry.path) {
            Ok(()) => {
                self.status = format!("Unstaged · {}", entry.rel);
                self.refresh_git();
            }
            Err(err) => self.status = format!("Unstage failed · {err}"),
        }
    }

    fn open_selected_git_file(&mut self) {
        let Some(entry) = self.git_entries.get(self.git_cursor).cloned() else {
            return;
        };
        self.open_file(entry.path);
    }

    fn diff_selected(&mut self, kind: DiffKind) {
        let Some(entry) = self.git_entries.get(self.git_cursor).cloned() else {
            self.status = "No file selected".into();
            return;
        };
        self.open_file(entry.path);
        self.viewer.set_diff_kind(&self.root, kind);
        self.focus = Focus::Viewer;
        self.status = format!("{} · {}", kind.label(), entry.rel);
    }

    fn blame_active_or_selected(&mut self) {
        if self.git_tab == GitPaneTab::Changes {
            if let Some(entry) = self.git_entries.get(self.git_cursor).cloned() {
                self.open_file(entry.path);
            }
        }
        if self.viewer.is_empty() {
            self.status = "Open a file first for blame".into();
            return;
        }
        self.viewer.show_blame(&self.root);
        self.focus = Focus::Viewer;
        self.status = format!("Blame · {}", self.viewer.title());
    }

    fn checkout_selected_branch(&mut self) {
        let Some(branch) = self.git_branches.get(self.git_cursor).cloned() else {
            self.status = "No branch selected".into();
            return;
        };
        if branch.current {
            self.status = format!("Already on · {}", branch.name);
            return;
        }
        match GitStatus::checkout(&self.root, &branch.name) {
            Ok(()) => {
                self.status = format!("Checked out · {}", branch.name);
                self.refresh_git();
            }
            Err(err) => {
                let short = err.lines().next().unwrap_or("checkout failed");
                self.status = format!("Checkout failed · {short}");
            }
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        // Help overlay — always available
        if self.show_help {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                    self.show_help = false;
                }
                KeyCode::F(1) => self.show_help = false,
                KeyCode::Char('p') | KeyCode::Char('t') => {
                    self.show_help = false;
                    self.show_themes = true;
                    self.theme_cursor = Theme::id().index();
                }
                KeyCode::Char('o') | KeyCode::Char('f') => {
                    self.open_search();
                }
                _ => {}
            }
            return;
        }

        if self.show_search {
            match key.code {
                KeyCode::Esc => {
                    self.show_search = false;
                    self.status = "Search closed".into();
                }
                KeyCode::Enter => self.confirm_search(),
                KeyCode::Up => {
                    if self.search_cursor > 0 {
                        self.search_cursor -= 1;
                    } else if !self.search_hits.is_empty() {
                        self.search_cursor = self.search_hits.len() - 1;
                    }
                }
                KeyCode::Down => {
                    if !self.search_hits.is_empty() {
                        self.search_cursor = (self.search_cursor + 1) % self.search_hits.len();
                    }
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.search_cursor = 0;
                    self.refresh_search_hits();
                }
                KeyCode::Char(c) if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) => {
                    if !c.is_control() {
                        self.search_query.push(c);
                        self.search_cursor = 0;
                        self.refresh_search_hits();
                    }
                }
                _ => {}
            }
            return;
        }

        if self.show_themes {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.show_themes = false,
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.theme_cursor > 0 {
                        self.theme_cursor -= 1;
                    } else {
                        self.theme_cursor = PaletteId::ALL.len() - 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.theme_cursor = (self.theme_cursor + 1) % PaletteId::ALL.len();
                }
                KeyCode::Enter => {
                    let id = PaletteId::from_index(self.theme_cursor);
                    Theme::set(id);
                    self.status = format!("Theme · {}", id.name());
                    self.show_themes = false;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    if let Some(d) = c.to_digit(10) {
                        let idx = (d as usize).saturating_sub(1);
                        if idx < PaletteId::ALL.len() {
                            self.theme_cursor = idx;
                            let id = PaletteId::from_index(idx);
                            Theme::set(id);
                            self.status = format!("Theme · {}", id.name());
                            self.show_themes = false;
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        if key.code == KeyCode::F(1)
            || (ctrl && key.code == KeyCode::Char('/'))
            || (key.code == KeyCode::Char('?') && self.focus != Focus::Terminal)
        {
            self.show_help = true;
            return;
        }

        if ctrl && key.code == KeyCode::Char('o') {
            self.open_search();
            return;
        }

        if ctrl && key.code == KeyCode::Char('g') {
            if self.show_git {
                self.close_git_pane();
            } else {
                self.open_git_pane();
            }
            return;
        }

        if ctrl && key.code == KeyCode::Char('l') && self.focus != Focus::Terminal {
            self.blame_active_or_selected();
            return;
        }

        if ctrl && key.code == KeyCode::Char('p') {
            self.show_themes = true;
            self.theme_cursor = Theme::id().index();
            return;
        }

        if ctrl && key.code == KeyCode::Char('0') {
            let id = Theme::cycle();
            self.status = format!("Theme · {}", id.name());
            return;
        }

        // Esc leaves terminal / closes nothing destructive
        if key.code == KeyCode::Esc && self.focus == Focus::Terminal && !ctrl {
            self.focus = Focus::Viewer;
            self.status = "Left terminal · press Ctrl+T to return".into();
            return;
        }

        if ctrl {
            match key.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    return;
                }
                KeyCode::Char('n') | KeyCode::Char('`') => {
                    self.add_terminal();
                    return;
                }
                KeyCode::Char('w') => {
                    match self.focus {
                        Focus::Terminal => self.close_active_terminal(),
                        Focus::Viewer | Focus::Tree => {
                            if self.viewer.close_active() {
                                self.status = "Closed file · click explorer to open another".into();
                            }
                        }
                    }
                    return;
                }
                KeyCode::Char('t') if !shift => {
                    self.focus = match self.focus {
                        Focus::Tree => Focus::Viewer,
                        Focus::Viewer => Focus::Terminal,
                        Focus::Terminal => Focus::Tree,
                    };
                    self.status = match self.focus {
                        Focus::Tree if self.show_git => "Git · s stage · Tab branches · Esc files".into(),
                        Focus::Tree => "Explorer · click a file to open".into(),
                        Focus::Viewer => "Editor · scroll to read · Ctrl+D diff".into(),
                        Focus::Terminal => "Terminal · type here (Esc to leave)".into(),
                    };
                    return;
                }
                KeyCode::Tab if shift => {
                    self.prev_terminal();
                    return;
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    self.next_terminal();
                    return;
                }
                KeyCode::Char('b') => {
                    self.show_tree = !self.show_tree;
                    self.status = if self.show_tree {
                        "Explorer shown".into()
                    } else {
                        "Explorer hidden · Ctrl+B to show".into()
                    };
                    return;
                }
                KeyCode::Char('d') if self.focus != Focus::Terminal => {
                    self.viewer.toggle_diff(&self.root);
                    self.focus = Focus::Viewer;
                    self.status = match self.viewer.mode() {
                        crate::viewer::ViewMode::Diff(kind) => {
                            format!("{} · Ctrl+D cycles HEAD → unstaged → staged", kind.label())
                        }
                        crate::viewer::ViewMode::Blame => "Blame".into(),
                        crate::viewer::ViewMode::File => "File view".into(),
                    };
                    return;
                }
                KeyCode::Char(']') if self.focus == Focus::Viewer => {
                    if !self.viewer.tabs.is_empty() {
                        self.viewer.active =
                            (self.viewer.active + 1) % self.viewer.tabs.len();
                    }
                    return;
                }
                KeyCode::Char('[') if self.focus == Focus::Viewer => {
                    if !self.viewer.tabs.is_empty() {
                        self.viewer.active = if self.viewer.active == 0 {
                            self.viewer.tabs.len() - 1
                        } else {
                            self.viewer.active - 1
                        };
                    }
                    return;
                }
                _ => {}
            }
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            if let KeyCode::Char(c) = key.code {
                if let Some(digit) = c.to_digit(10) {
                    let idx = (digit as usize).saturating_sub(1);
                    if idx < self.terminals.len() {
                        self.active_term = idx;
                        self.focus = Focus::Terminal;
                    }
                    return;
                }
            }
        }

        match self.focus {
            Focus::Terminal => self.on_term_key(key),
            Focus::Tree if self.show_git => self.on_git_key(key),
            Focus::Tree => self.on_tree_key(key),
            Focus::Viewer => self.on_viewer_key(key),
        }
    }

    fn on_git_key(&mut self, key: KeyEvent) {
        let height = self.tree_inner.height as usize;
        match key.code {
            KeyCode::Esc => {
                self.close_git_pane();
            }
            KeyCode::Tab => {
                self.git_tab = match self.git_tab {
                    GitPaneTab::Changes => GitPaneTab::Branches,
                    GitPaneTab::Branches => GitPaneTab::Changes,
                };
                self.git_cursor = 0;
                self.git_scroll = 0;
                if self.git_tab == GitPaneTab::Branches {
                    self.git_branches = GitStatus::branches(&self.root);
                    if let Some(idx) = self.git_branches.iter().position(|b| b.current) {
                        self.git_cursor = idx;
                    }
                }
                self.clamp_git_cursor();
                self.ensure_git_visible(height);
            }
            KeyCode::Char('c') => {
                self.git_tab = GitPaneTab::Changes;
                self.git_cursor = 0;
                self.git_scroll = 0;
                self.clamp_git_cursor();
            }
            KeyCode::Char('b') => {
                self.git_tab = GitPaneTab::Branches;
                self.git_branches = GitStatus::branches(&self.root);
                self.git_cursor = self
                    .git_branches
                    .iter()
                    .position(|b| b.current)
                    .unwrap_or(0);
                self.git_scroll = 0;
                self.ensure_git_visible(height);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.git_cursor > 0 {
                    self.git_cursor -= 1;
                } else if self.git_list_len() > 0 {
                    self.git_cursor = self.git_list_len() - 1;
                }
                self.ensure_git_visible(height);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.git_list_len();
                if len > 0 {
                    self.git_cursor = (self.git_cursor + 1) % len;
                }
                self.ensure_git_visible(height);
            }
            KeyCode::PageUp => {
                self.git_cursor = self.git_cursor.saturating_sub(height.max(1));
                self.ensure_git_visible(height);
            }
            KeyCode::PageDown => {
                let len = self.git_list_len();
                if len > 0 {
                    self.git_cursor = (self.git_cursor + height.max(1)).min(len - 1);
                }
                self.ensure_git_visible(height);
            }
            KeyCode::Home => {
                self.git_cursor = 0;
                self.git_scroll = 0;
            }
            KeyCode::End => {
                let len = self.git_list_len();
                if len > 0 {
                    self.git_cursor = len - 1;
                    self.ensure_git_visible(height);
                }
            }
            KeyCode::Char('s') if self.git_tab == GitPaneTab::Changes => self.stage_selected(),
            KeyCode::Char('u') if self.git_tab == GitPaneTab::Changes => self.unstage_selected(),
            KeyCode::Char(' ') if self.git_tab == GitPaneTab::Changes => {
                if let Some(entry) = self.git_entries.get(self.git_cursor) {
                    if entry.is_unstaged() {
                        self.stage_selected();
                    } else if entry.is_staged() {
                        self.unstage_selected();
                    }
                }
            }
            KeyCode::Char('d') if self.git_tab == GitPaneTab::Changes => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.diff_selected(DiffKind::Staged);
                } else {
                    self.diff_selected(DiffKind::Unstaged);
                }
            }
            KeyCode::Char('D') if self.git_tab == GitPaneTab::Changes => {
                self.diff_selected(DiffKind::Staged);
            }
            KeyCode::Char('h') if self.git_tab == GitPaneTab::Changes => {
                self.diff_selected(DiffKind::Head);
            }
            KeyCode::Char('l') => self.blame_active_or_selected(),
            KeyCode::Enter => match self.git_tab {
                GitPaneTab::Changes => self.open_selected_git_file(),
                GitPaneTab::Branches => self.checkout_selected_branch(),
            },
            KeyCode::Char('r') => {
                self.refresh_git();
                self.status = "Git refreshed".into();
            }
            _ => {}
        }
    }

    fn on_term_key(&mut self, key: KeyEvent) {
        let term = self.active_term_mut();
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => term.write_bytes(&[0x03]),
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => term.write_bytes(&[0x04]),
            (KeyModifiers::CONTROL, KeyCode::Char('l')) => term.write_bytes(&[0x0c]),
            (KeyModifiers::CONTROL, KeyCode::Char('z')) => term.write_bytes(&[0x1a]),
            (KeyModifiers::SHIFT, KeyCode::Enter) | (_, KeyCode::Enter) => {
                term.write_bytes(b"\r")
            }
            (_, KeyCode::Backspace) => term.write_bytes(&[0x7f]),
            (_, KeyCode::Tab) => term.write_bytes(b"\t"),
            (_, KeyCode::Esc) => term.write_bytes(&[0x1b]),
            (_, KeyCode::Up) => term.write_bytes(b"\x1b[A"),
            (_, KeyCode::Down) => term.write_bytes(b"\x1b[B"),
            (_, KeyCode::Right) => term.write_bytes(b"\x1b[C"),
            (_, KeyCode::Left) => term.write_bytes(b"\x1b[D"),
            (_, KeyCode::Home) => term.write_bytes(b"\x1b[H"),
            (_, KeyCode::End) => term.write_bytes(b"\x1b[F"),
            (_, KeyCode::Char(c)) => term.write_char(c),
            _ => {}
        }
    }

    fn on_tree_key(&mut self, key: KeyEvent) {
        let height = self.tree_inner.height as usize;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.tree.move_sel(-1);
                self.tree.ensure_visible(height);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.tree.move_sel(1);
                self.tree.ensure_visible(height);
            }
            KeyCode::PageUp => {
                self.tree.scroll_by(-(height as isize).max(1), height);
            }
            KeyCode::PageDown => {
                self.tree.scroll_by((height as isize).max(1), height);
            }
            KeyCode::Home => {
                self.tree.selected = 0;
                self.tree.scroll = 0;
            }
            KeyCode::End => {
                if !self.tree.flat.is_empty() {
                    self.tree.selected = self.tree.flat.len() - 1;
                    self.tree.ensure_visible(height);
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(path) = self.tree.toggle_selected() {
                    self.open_file(path);
                }
                self.tree.ensure_visible(height);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(entry) = self.tree.flat.get(self.tree.selected) {
                    if entry.is_dir && entry.expanded {
                        let _ = self.tree.toggle_selected();
                    }
                }
                self.tree.ensure_visible(height);
            }
            _ => {}
        }
    }

    fn on_viewer_key(&mut self, key: KeyEvent) {
        let height = self.viewer_body.height;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.viewer.scroll_by(-1, height),
            KeyCode::Down | KeyCode::Char('j') => self.viewer.scroll_by(1, height),
            KeyCode::PageUp => self.viewer.scroll_by(-(height as i32), height),
            KeyCode::PageDown => self.viewer.scroll_by(height as i32, height),
            KeyCode::Home => self.viewer.set_scroll(0),
            KeyCode::End => {
                let max = self
                    .viewer
                    .lines()
                    .len()
                    .saturating_sub(height as usize) as u16;
                self.viewer.set_scroll(max);
            }
            KeyCode::Char('d') => {
                self.viewer.toggle_diff(&self.root);
                self.status = match self.viewer.mode() {
                    crate::viewer::ViewMode::Diff(kind) => {
                        format!("{} · Ctrl+D cycles modes", kind.label())
                    }
                    crate::viewer::ViewMode::Blame => "Blame".into(),
                    crate::viewer::ViewMode::File => "File view".into(),
                };
            }
            KeyCode::Char('l') => {
                self.viewer.toggle_blame(&self.root);
                self.status = match self.viewer.mode() {
                    crate::viewer::ViewMode::Blame => "Blame".into(),
                    _ => "File view".into(),
                };
            }
            _ => {}
        }
    }

    fn hit_at(&self, col: u16, row: u16) -> Option<&Hit> {
        // Prefer smaller / more specific hits (close buttons) — iterate reverse
        self.hits.iter().rev().find(|h| point_in(h.area, col, row))
    }

    fn on_mouse(&mut self, mouse: MouseEvent) {
        let col = mouse.column;
        let row = mouse.row;
        self.mouse_col = col;
        self.mouse_row = row;

        match mouse.kind {
            MouseEventKind::Moved => {
                if let Some(drag) = self.drag {
                    match drag {
                        DragKind::VSplit => {
                            let min_w = 18u16;
                            let max_w = self.main_area.width.saturating_sub(24).max(min_w);
                            self.tree_width =
                                col.saturating_sub(self.main_area.x).clamp(min_w, max_w);
                            self.set_pointer(true);
                            return;
                        }
                        DragKind::HSplit => {
                            let total = self.main_area.height.max(1);
                            let y = row.saturating_sub(self.main_area.y);
                            self.top_pct =
                                ((y as u32 * 100) / total as u32).clamp(20, 80) as u16;
                            self.set_pointer(true);
                            return;
                        }
                    }
                }

                let hit = self.hit_at(col, row).map(|h| h.kind);
                let clickable = match hit {
                    Some(HitKind::ViewerBody) | Some(HitKind::TermBody) | None => false,
                    Some(_) => true,
                };
                let over_chrome = point_in(self.tree_inner, col, row)
                    || point_in(self.v_split_area, col, row)
                    || point_in(self.h_split_area, col, row)
                    || self.hits.iter().any(|h| {
                        point_in(h.area, col, row)
                            && !matches!(h.kind, HitKind::ViewerBody | HitKind::TermBody)
                    });
                self.set_pointer(clickable || over_chrome);
                self.hover = hit;

                if point_in(self.tree_inner, col, row) {
                    let y = row.saturating_sub(self.tree_inner.y) as usize;
                    let scroll = if self.show_git {
                        self.git_scroll
                    } else {
                        self.tree.scroll
                    };
                    self.tree_hover_row = Some(scroll + y);
                } else {
                    self.tree_hover_row = None;
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(hit) = self.hit_at(col, row).map(|h| h.kind) {
                    match hit {
                        HitKind::VSplit => {
                            self.drag = Some(DragKind::VSplit);
                            self.set_pointer(true);
                            return;
                        }
                        HitKind::HSplit => {
                            self.drag = Some(DragKind::HSplit);
                            self.set_pointer(true);
                            return;
                        }
                        HitKind::FileTab { index } => {
                            self.viewer.active = index;
                            self.focus = Focus::Viewer;
                        }
                        HitKind::FileClose { index } => {
                            self.viewer.close_at(index);
                            self.status = "Closed file tab".into();
                        }
                        HitKind::TermTab { index } => {
                            self.active_term = index;
                            self.focus = Focus::Terminal;
                        }
                        HitKind::TermClose { index } => {
                            self.close_terminal_at(index);
                        }
                        HitKind::TermNew => self.add_terminal(),
                        HitKind::TreeRow => {
                            self.focus = Focus::Tree;
                            let y = row.saturating_sub(self.tree_inner.y) as usize;
                            if self.show_git {
                                let index = self.git_scroll + y;
                                if index < self.git_list_len() {
                                    self.git_cursor = index;
                                    match self.git_tab {
                                        GitPaneTab::Changes => self.open_selected_git_file(),
                                        GitPaneTab::Branches => self.checkout_selected_branch(),
                                    }
                                }
                            } else {
                                let index = self.tree.scroll + y;
                                if let Some(path) = self.tree.open_at_flat_index(index) {
                                    self.open_file(path);
                                }
                            }
                        }
                        HitKind::ViewerBody => self.focus = Focus::Viewer,
                        HitKind::TermBody => self.focus = Focus::Terminal,
                    }
                    return;
                }

                if point_in(self.term_area, col, row) {
                    self.focus = Focus::Terminal;
                } else if point_in(self.tree_area, col, row) {
                    self.focus = Focus::Tree;
                } else if point_in(self.viewer_area, col, row) {
                    self.focus = Focus::Viewer;
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.drag.take().is_some() {
                    self.status = format!(
                        "layout · explorer {} cols · top {}%",
                        self.tree_width, self.top_pct
                    );
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(drag) = self.drag {
                    match drag {
                        DragKind::VSplit => {
                            let min_w = 18u16;
                            let max_w = self.main_area.width.saturating_sub(24).max(min_w);
                            self.tree_width =
                                col.saturating_sub(self.main_area.x).clamp(min_w, max_w);
                        }
                        DragKind::HSplit => {
                            let total = self.main_area.height.max(1);
                            let y = row.saturating_sub(self.main_area.y);
                            self.top_pct =
                                ((y as u32 * 100) / total as u32).clamp(20, 80) as u16;
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if point_in(self.viewer_body, col, row) || point_in(self.viewer_area, col, row) {
                    self.focus = Focus::Viewer;
                    self.viewer.scroll_by(-1, self.viewer_body.height.max(1));
                } else if point_in(self.tree_inner, col, row) || point_in(self.tree_area, col, row)
                {
                    self.focus = Focus::Tree;
                    let h = self.tree_inner.height.max(1) as usize;
                    if self.show_git {
                        if self.git_cursor > 0 {
                            self.git_cursor -= 1;
                        }
                        self.ensure_git_visible(h);
                    } else {
                        self.tree.scroll_by(-1, h);
                    }
                } else if point_in(self.term_body_area, col, row) {
                    self.active_term_mut().write_bytes(b"\x1b[A");
                }
            }
            MouseEventKind::ScrollDown => {
                if point_in(self.viewer_body, col, row) || point_in(self.viewer_area, col, row) {
                    self.focus = Focus::Viewer;
                    self.viewer.scroll_by(1, self.viewer_body.height.max(1));
                } else if point_in(self.tree_inner, col, row) || point_in(self.tree_area, col, row)
                {
                    self.focus = Focus::Tree;
                    let h = self.tree_inner.height.max(1) as usize;
                    if self.show_git {
                        let len = self.git_list_len();
                        if len > 0 && self.git_cursor + 1 < len {
                            self.git_cursor += 1;
                        }
                        self.ensure_git_visible(h);
                    } else {
                        self.tree.scroll_by(1, h);
                    }
                } else if point_in(self.term_body_area, col, row) {
                    self.active_term_mut().write_bytes(b"\x1b[B");
                }
            }
            _ => {}
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        self.viewer.open(&path, &self.root);
        self.tree.reveal(&path);
        self.focus = Focus::Viewer;
        self.status = format!("● {}", path.display());
    }

    fn ui(&mut self, f: &mut Frame<'_>) {
        self.hits.clear();
        f.render_widget(Block::default().style(Theme::bg()), f.area());

        let root_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(8),
                Constraint::Length(1),
            ])
            .split(f.area());

        self.draw_titlebar(f, root_chunks[0]);

        let main_area = root_chunks[1];
        self.main_area = main_area;

        let top_pct = self.top_pct.clamp(20, 80);
        let avail = main_area.height.saturating_sub(1); // leave 1 for splitter
        let top_h = ((avail as u32 * top_pct as u32) / 100)
            .clamp(5, avail.saturating_sub(5) as u32) as u16;
        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_h),
                Constraint::Length(1), // horizontal splitter
                Constraint::Min(5),
            ])
            .split(main_area);

        self.h_split_area = main[1];
        self.hits.push(Hit {
            kind: HitKind::HSplit,
            area: main[1],
        });
        // Draw horizontal splitter bar
        let h_focused = matches!(self.hover, Some(HitKind::HSplit))
            || matches!(self.drag, Some(DragKind::HSplit));
        let h_label = if h_focused {
            let msg = "  ↕ drag to resize terminal  ";
            let pad = main[1].width as usize;
            let mut s = format!("{:═^width$}", msg, width = pad.max(msg.len()));
            if s.len() > pad {
                s.truncate(pad);
            }
            s
        } else {
            "═".repeat(main[1].width as usize)
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                h_label,
                Style::default()
                    .fg(if h_focused {
                        Theme::get().accent_glow
                    } else {
                        Theme::get().border
                    })
                    .bg(Theme::get().titlebar),
            ))),
            main[1],
        );

        let top = if self.show_tree {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(self.tree_width),
                    Constraint::Length(1), // vertical splitter
                    Constraint::Min(20),
                ])
                .split(main[0])
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(0),
                    Constraint::Length(0),
                    Constraint::Min(20),
                ])
                .split(main[0])
        };

        self.tree_area = top[0];
        self.v_split_area = top[1];
        self.viewer_area = top[2];
        self.term_area = main[2];

        if self.show_tree {
            if self.show_git {
                self.draw_git(f, top[0]);
            } else {
                self.draw_tree(f, top[0]);
            }
            self.hits.push(Hit {
                kind: HitKind::VSplit,
                area: top[1],
            });
            let v_focused = matches!(self.hover, Some(HitKind::VSplit))
                || matches!(self.drag, Some(DragKind::VSplit));
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "║",
                    Style::default()
                        .fg(if v_focused {
                            Theme::get().accent_glow
                        } else {
                            Theme::get().border
                        })
                        .bg(Theme::get().titlebar),
                )))
                .alignment(Alignment::Center),
                top[1],
            );
            // Fill full height of splitter with ║
            for y in 0..top[1].height {
                let cell = Rect {
                    x: top[1].x,
                    y: top[1].y.saturating_add(y),
                    width: 1,
                    height: 1,
                };
                f.render_widget(
                    Paragraph::new("║").style(Style::default().fg(if v_focused {
                        Theme::get().accent_glow
                    } else {
                        Theme::get().border
                    }).bg(Theme::get().titlebar)),
                    cell,
                );
            }
        }

        self.draw_viewer(f, top[2]);
        self.draw_term(f, main[2]);
        self.draw_status(f, root_chunks[2]);
        if self.show_help {
            self.draw_help(f, f.area());
        }
        if self.show_themes {
            self.draw_themes(f, f.area());
        }
        if self.show_search {
            self.draw_search(f, f.area());
        }
    }

    fn draw_titlebar(&self, f: &mut Frame<'_>, area: Rect) {
        let name = self
            .root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("workspace");
        let branch = if self.branch.is_empty() {
            String::new()
        } else {
            format!("   {} ", self.branch)
        };
        let tip = match self.focus {
            Focus::Tree if self.show_git => "  git · Ctrl+G files   ",
            Focus::Tree => "  1. click a file   ",
            Focus::Viewer => "  2. read / Ctrl+D diff   ",
            Focus::Terminal => "  3. type codex / claude   ",
        };
        let line = Line::from(vec![
            Span::styled(
                "  ◆ noir  ",
                Style::default()
                    .fg(Theme::get().status_fg)
                    .bg(Theme::get().accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {name}{branch}"),
                Style::default().fg(Theme::get().fg).bg(Theme::get().titlebar),
            ),
            Span::styled(tip, Style::default().fg(Theme::get().accent_glow).bg(Theme::get().titlebar)),
            Span::styled(
                "  ? help  ·  Ctrl+G git  ·  Ctrl+O search  ",
                Style::default().fg(Theme::get().fg_muted).bg(Theme::get().titlebar),
            ),
        ]);
        f.render_widget(
            Paragraph::new(line).style(Style::default().bg(Theme::get().titlebar)),
            area,
        );
    }

    fn draw_git(&mut self, f: &mut Frame<'_>, area: Rect) {
        let focused = self.focus == Focus::Tree;
        let branch = if self.branch.is_empty() {
            "no git".to_string()
        } else {
            format!(" {}", self.branch)
        };
        let tab = match self.git_tab {
            GitPaneTab::Changes => "changes",
            GitPaneTab::Branches => "branches",
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::border(focused))
            .style(Theme::sidebar())
            .title(Span::styled(
                format!("  GIT  ·  {branch}  ·  {tab}  "),
                Theme::title(focused),
            ))
            .title_bottom(Span::styled(
                if focused {
                    match self.git_tab {
                        GitPaneTab::Changes => {
                            "  s stage · u unstage · d/D diff · Tab branches  "
                        }
                        GitPaneTab::Branches => "  Enter checkout · Tab changes · Esc files  ",
                    }
                } else {
                    "  Ctrl+G git pane  "
                },
                Style::default()
                    .fg(Theme::get().fg_muted)
                    .bg(Theme::get().sidebar),
            ));

        let inner = block.inner(area);
        self.tree_inner = inner;
        f.render_widget(block, area);

        let height = inner.height as usize;
        self.ensure_git_visible(height);

        self.hits.push(Hit {
            kind: HitKind::TreeRow,
            area: inner,
        });

        let mut items: Vec<ListItem> = Vec::new();

        match self.git_tab {
            GitPaneTab::Changes => {
                if self.git_entries.is_empty() {
                    items.push(ListItem::new(Line::from(Span::styled(
                        "  working tree clean",
                        Style::default().fg(Theme::get().fg_muted).bg(Theme::get().sidebar),
                    ))));
                } else {
                    // Section headers as virtual rows would shift cursor — instead prefix codes.
                    for (idx, entry) in self
                        .git_entries
                        .iter()
                        .enumerate()
                        .skip(self.git_scroll)
                        .take(height)
                    {
                        let selected = idx == self.git_cursor;
                        let hovered = self.tree_hover_row == Some(idx);
                        let bg = if selected {
                            Theme::get().selection
                        } else if hovered {
                            Theme::get().tab_hover
                        } else {
                            Theme::get().sidebar
                        };
                        let code = entry.display_code();
                        let code_fg = match entry.worktree.max(entry.index) {
                            'M' | 'U' | 'R' | 'C' => Theme::get().git_mod,
                            'A' | '?' => Theme::get().git_add,
                            'D' => Theme::get().git_del,
                            _ => Theme::get().fg_muted,
                        };
                        let staged_mark = if entry.is_staged() { "●" } else { " " };
                        let label = format!(" {staged_mark} {code}  {} ", entry.rel);
                        let pad_w = (inner.width as usize)
                            .saturating_sub(unicode_width::UnicodeWidthStr::width(label.as_str()));
                        let pad = " ".repeat(pad_w);
                        let style = Style::default().bg(bg).fg(if selected {
                            Theme::get().accent_glow
                        } else {
                            Theme::get().fg
                        });
                        items.push(ListItem::new(Line::from(vec![
                            Span::styled(
                                format!(" {staged_mark} "),
                                Style::default().bg(bg).fg(Theme::get().git_add),
                            ),
                            Span::styled(
                                format!("{code} "),
                                Style::default().bg(bg).fg(code_fg).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(entry.rel.clone(), style),
                            Span::styled(pad, Style::default().bg(bg)),
                        ])));
                    }
                }
            }
            GitPaneTab::Branches => {
                if self.git_branches.is_empty() {
                    items.push(ListItem::new(Line::from(Span::styled(
                        "  no branches",
                        Style::default().fg(Theme::get().fg_muted).bg(Theme::get().sidebar),
                    ))));
                } else {
                    for (idx, branch) in self
                        .git_branches
                        .iter()
                        .enumerate()
                        .skip(self.git_scroll)
                        .take(height)
                    {
                        let selected = idx == self.git_cursor;
                        let hovered = self.tree_hover_row == Some(idx);
                        let bg = if selected {
                            Theme::get().selection
                        } else if hovered {
                            Theme::get().tab_hover
                        } else {
                            Theme::get().sidebar
                        };
                        let mark = if branch.current { "●" } else { "○" };
                        let label = format!("  {mark} {} ", branch.name);
                        let pad_w = (inner.width as usize)
                            .saturating_sub(unicode_width::UnicodeWidthStr::width(label.as_str()));
                        let pad = " ".repeat(pad_w);
                        let fg = if branch.current {
                            Theme::get().accent_glow
                        } else if selected {
                            Theme::get().fg
                        } else {
                            Theme::get().fg_muted
                        };
                        items.push(ListItem::new(Line::from(vec![
                            Span::styled(
                                label,
                                Style::default()
                                    .bg(bg)
                                    .fg(fg)
                                    .add_modifier(if selected || branch.current {
                                        Modifier::BOLD
                                    } else {
                                        Modifier::empty()
                                    }),
                            ),
                            Span::styled(pad, Style::default().bg(bg)),
                        ])));
                    }
                }
            }
        }

        f.render_widget(List::new(items), inner);

        let total = self.git_list_len();
        if total > height && height > 0 {
            let track = inner.height.max(1) as usize;
            let thumb_h = ((height * track) / total).max(1);
            let max_scroll = total.saturating_sub(height).max(1);
            let thumb_y = (self.git_scroll * (track.saturating_sub(thumb_h))) / max_scroll;
            let thumb = Rect {
                x: inner.x.saturating_add(inner.width.saturating_sub(1)),
                y: inner.y.saturating_add(thumb_y as u16),
                width: 1,
                height: thumb_h as u16,
            };
            f.render_widget(
                Paragraph::new("┃").style(
                    Style::default()
                        .fg(Theme::get().accent_glow)
                        .bg(Theme::get().sidebar),
                ),
                thumb,
            );
        }
    }

    fn draw_tree(&mut self, f: &mut Frame<'_>, area: Rect) {
        let focused = self.focus == Focus::Tree;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::border(focused))
            .style(Theme::sidebar())
            .title(Span::styled("  FILES  ", Theme::title(focused)))
            .title_bottom(Span::styled(
                if focused {
                    "  ↑↓ move  ·  Enter open  ·  click OK  "
                } else {
                    "  click a file to open  "
                },
                Style::default().fg(Theme::get().fg_muted).bg(Theme::get().sidebar),
            ));

        let inner = block.inner(area);
        self.tree_inner = inner;
        f.render_widget(block, area);

        let height = inner.height as usize;
        self.tree.ensure_visible(height);

        self.hits.push(Hit {
            kind: HitKind::TreeRow,
            area: inner,
        });

        let items: Vec<ListItem> = self
            .tree
            .flat
            .iter()
            .enumerate()
            .skip(self.tree.scroll)
            .take(height)
            .map(|(idx, entry)| {
                let chevron = if entry.is_dir {
                    if entry.expanded {
                        "▾"
                    } else {
                        "▸"
                    }
                } else {
                    " "
                };
                let indent = "  ".repeat(entry.depth as usize);
                let git = entry.git.map(|c| format!(" {c}")).unwrap_or_default();

                let selected = idx == self.tree.selected;
                let hovered = self.tree_hover_row == Some(idx);
                let bg = if selected {
                    Theme::get().selection
                } else if hovered {
                    Theme::get().tab_hover
                } else {
                    Theme::get().sidebar
                };
                let name_fg = if selected {
                    Theme::get().fg
                } else if entry.is_dir {
                    Theme::get().folder
                } else {
                    Theme::file_color(&entry.name)
                };
                let git_fg = match entry.git {
                    Some('M') | Some('U') | Some('•') => Theme::get().git_mod,
                    Some('A') | Some('?') => Theme::get().git_add,
                    Some('D') => Theme::get().git_del,
                    _ => name_fg,
                };

                let prefix = format!("{indent}{chevron} ");
                let pad_w = (inner.width as usize).saturating_sub(
                    unicode_width::UnicodeWidthStr::width(prefix.as_str())
                        + unicode_width::UnicodeWidthStr::width(entry.name.as_str())
                        + unicode_width::UnicodeWidthStr::width(git.as_str()),
                );
                let pad = " ".repeat(pad_w);
                let style_base = Style::default().bg(bg);
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style_base.fg(if entry.is_dir { Theme::get().folder } else { Theme::get().fg_muted })),
                    Span::styled(
                        entry.name.clone(),
                        style_base.fg(name_fg).add_modifier(if selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                    ),
                    Span::styled(git, style_base.fg(git_fg)),
                    Span::styled(pad, style_base),
                ]))
            })
            .collect();

        f.render_widget(List::new(items), inner);

        // Scroll thumb indicator
        if self.tree.flat.len() > height && height > 0 {
            let track = inner.height.max(1) as usize;
            let thumb_h = ((height * track) / self.tree.flat.len()).max(1);
            let max_scroll = self.tree.flat.len().saturating_sub(height).max(1);
            let thumb_y = (self.tree.scroll * (track.saturating_sub(thumb_h))) / max_scroll;
            let thumb = Rect {
                x: inner.x.saturating_add(inner.width.saturating_sub(1)),
                y: inner.y.saturating_add(thumb_y as u16),
                width: 1,
                height: thumb_h as u16,
            };
            f.render_widget(
                Paragraph::new("┃").style(Style::default().fg(Theme::get().accent_glow).bg(Theme::get().sidebar)),
                thumb,
            );
        }
    }

    fn draw_viewer(&mut self, f: &mut Frame<'_>, area: Rect) {
        let focused = self.focus == Focus::Viewer;
        let mode = self.viewer.mode_label();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::border(focused))
            .style(Theme::bg())
            .title(Span::styled(
                format!("  {mode}  ·  click × to close tabs  "),
                Theme::title(focused),
            ))
            .title_bottom(Span::styled(
                if focused {
                    "  scroll  ·  Ctrl+D cycle diff  ·  l blame  ·  Ctrl+W close  "
                } else {
                    "  click here to focus editor  "
                },
                Style::default().fg(Theme::get().fg_muted).bg(Theme::get().bg),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner);

        let tab_area = parts[0];
        self.viewer_body = parts[1];

        // File tabs with × close
        let mut spans = Vec::new();
        let mut x = tab_area.x;
        if self.viewer.tabs.is_empty() {
            spans.push(Span::styled(
                "  ← click a file on the left to get started  ",
                Style::default().fg(Theme::get().accent_glow).bg(Theme::get().tab_bar),
            ));
        } else {
            for (i, tab) in self.viewer.tabs.iter().enumerate() {
                let active = i == self.viewer.active;
                let label = format!("  {}  ", tab.title);
                let close = " [x] ";
                let label_w = unicode_width::UnicodeWidthStr::width(label.as_str()) as u16;
                let close_w = unicode_width::UnicodeWidthStr::width(close) as u16;

                let ft = Theme::file_color(&tab.title);
                let style = if active {
                    Style::default()
                        .fg(ft)
                        .bg(Theme::get().tab_active)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(ft).bg(Theme::get().tab_inactive)
                };
                spans.push(Span::styled(label, style));
                self.hits.push(Hit {
                    kind: HitKind::FileTab { index: i },
                    area: Rect {
                        x,
                        y: tab_area.y,
                        width: label_w,
                        height: 1,
                    },
                });
                x = x.saturating_add(label_w);

                let close_style =
                    if matches!(self.hover, Some(HitKind::FileClose { index }) if index == i) {
                        Style::default()
                            .fg(Theme::get().status_fg)
                            .bg(Theme::get().close)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Theme::button_close(active)
                    };
                spans.push(Span::styled(close, close_style));
                self.hits.push(Hit {
                    kind: HitKind::FileClose { index: i },
                    area: Rect {
                        x,
                        y: tab_area.y,
                        width: close_w,
                        height: 1,
                    },
                });
                x = x.saturating_add(close_w);
                spans.push(Span::styled(
                    "│",
                    Style::default().fg(Theme::get().border).bg(Theme::get().tab_bar),
                ));
                x = x.saturating_add(1);
            }
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)).style(Theme::tab_bar()),
            tab_area,
        );

        self.hits.push(Hit {
            kind: HitKind::ViewerBody,
            area: self.viewer_body,
        });

        let height = self.viewer_body.height as usize;
        let start = self.viewer.scroll() as usize;
        let content: Vec<Line> = if self.viewer.is_empty() {
            welcome_lines()
        } else {
            let end = (start + height).min(self.viewer.lines().len());
            self.viewer.lines()[start..end].to_vec()
        };
        f.render_widget(Clear, self.viewer_body);
        f.render_widget(Paragraph::new(content).style(Theme::bg()), self.viewer_body);
    }

    fn draw_term(&mut self, f: &mut Frame<'_>, area: Rect) {
        let focused = self.focus == Focus::Terminal;
        let count = self.terminals.len();
        let blink_on = (self.blink_started.elapsed().as_millis() / 530) % 2 == 0;

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default()
                    .fg(Theme::get().accent_glow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Theme::border(false)
            })
            .style(Theme::panel())
            .title(Span::styled(
                if focused {
                    format!("  ● TERMINAL (typing)  ·  {count}  ")
                } else {
                    format!("  TERMINAL  ·  click to type  ·  {count}  ")
                },
                if focused {
                    Style::default()
                        .fg(Theme::get().bg)
                        .bg(Theme::get().accent_glow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Theme::title(false)
                },
            ))
            .title_alignment(Alignment::Left)
            .title_bottom(Span::styled(
                if focused {
                    "  Esc leave  ·  [x] close tab  ·  ＋ new  ·  ? help  "
                } else {
                    "  click this panel → run  codex  or  claude  "
                },
                Style::default()
                    .fg(if focused {
                        Theme::get().accent_glow
                    } else {
                        Theme::get().fg_muted
                    })
                    .bg(Theme::get().panel),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // tabs
                Constraint::Length(1), // agent hint strip
                Constraint::Min(2),    // body
            ])
            .split(inner);

        let tab_area = parts[0];
        let hint_area = parts[1];
        self.term_body_area = parts[2];

        let mut spans = Vec::new();
        let mut x = tab_area.x;

        for (i, term) in self.terminals.iter().enumerate() {
            let active = i == self.active_term;
            let mark = if term.alive { "●" } else { "○" };
            let label = format!("  {mark} {}  ", term.title);
            let close = " [x] ";
            let label_w = unicode_width::UnicodeWidthStr::width(label.as_str()) as u16;
            let close_w = unicode_width::UnicodeWidthStr::width(close) as u16;

            let style = if active {
                Style::default()
                    .fg(Theme::get().fg)
                    .bg(Theme::get().panel_elevated)
                    .add_modifier(Modifier::BOLD)
            } else {
                Theme::tab_inactive()
            };

            spans.push(Span::styled(label, style));
            self.hits.push(Hit {
                kind: HitKind::TermTab { index: i },
                area: Rect {
                    x,
                    y: tab_area.y,
                    width: label_w,
                    height: 1,
                },
            });
            x = x.saturating_add(label_w);

            // Close button — always a distinct hit target
            let close_style = if matches!(self.hover, Some(HitKind::TermClose { index }) if index == i)
            {
                Style::default()
                    .fg(Theme::get().status_fg)
                    .bg(Theme::get().close)
                    .add_modifier(Modifier::BOLD)
            } else {
                Theme::button_close(active)
            };
            spans.push(Span::styled(close, close_style));
            self.hits.push(Hit {
                kind: HitKind::TermClose { index: i },
                area: Rect {
                    x,
                    y: tab_area.y,
                    width: close_w,
                    height: 1,
                },
            });
            x = x.saturating_add(close_w);

            spans.push(Span::styled(
                " │ ",
                Style::default().fg(Theme::get().border).bg(Theme::get().tab_bar),
            ));
            x = x.saturating_add(3);
        }

        let plus = "  + New  ";
        let plus_w = unicode_width::UnicodeWidthStr::width(plus) as u16;
        let plus_style = if matches!(self.hover, Some(HitKind::TermNew)) {
            Style::default()
                .fg(Theme::get().status_fg)
                .bg(Theme::get().accent_glow)
                .add_modifier(Modifier::BOLD)
        } else {
            Theme::button()
        };
        spans.push(Span::styled(plus, plus_style));
        self.hits.push(Hit {
            kind: HitKind::TermNew,
            area: Rect {
                x,
                y: tab_area.y,
                width: plus_w,
                height: 1,
            },
        });

        f.render_widget(
            Paragraph::new(Line::from(spans)).style(Theme::tab_bar()),
            tab_area,
        );

        // Clear one-line coaching strip
        let hint = if focused {
            Line::from(vec![
                Span::styled(
                    "  READY  ",
                    Style::default()
                        .fg(Theme::get().bg)
                        .bg(Theme::get().git_add)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "  type a command — try:  codex    or    claude  ",
                    Style::default().fg(Theme::get().fg).bg(Theme::get().panel_elevated),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    "  CLICK HERE  ",
                    Style::default()
                        .fg(Theme::get().bg)
                        .bg(Theme::get().accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "  to focus the terminal and run your agent  ",
                    Style::default().fg(Theme::get().fg).bg(Theme::get().panel_elevated),
                ),
            ])
        };
        f.render_widget(
            Paragraph::new(hint).style(Style::default().bg(Theme::get().panel_elevated)),
            hint_area,
        );

        self.hits.push(Hit {
            kind: HitKind::TermBody,
            area: self.term_body_area,
        });

        let body = self.term_body_area;
        f.render_widget(Clear, body);
        // Focused: blinking block caret. Unfocused: solid dim caret so you still see position.
        let lines = self.terminals[self.active_term].lines(
            body.width,
            body.height,
            true,
            if focused { blink_on } else { true },
        );
        f.render_widget(Paragraph::new(lines).style(Theme::panel()), body);

        // Hardware cursor on the caret cell when focused — OS-level visibility
        if focused {
            let (cr, cc) = self.terminals[self.active_term].cursor_pos();
            if cr < body.height && cc < body.width {
                f.set_cursor_position(ratatui::layout::Position {
                    x: body.x.saturating_add(cc),
                    y: body.y.saturating_add(cr),
                });
            }
        }
    }

    fn draw_status(&self, f: &mut Frame<'_>, area: Rect) {
        let focus = match self.focus {
            Focus::Tree if self.show_git => "GIT",
            Focus::Tree => "FILES",
            Focus::Viewer => self.viewer.mode_label(),
            Focus::Terminal => "TERMINAL",
        };
        let tip = match self.focus {
            Focus::Tree if self.show_git => "s/u stage · Tab branches · l blame",
            Focus::Tree => "Enter/click open file",
            Focus::Viewer => "Ctrl+D cycle diff · l blame · Ctrl+W",
            Focus::Terminal => "Esc leave · Ctrl+N new tab",
        };
        let branch = if self.branch.is_empty() {
            "no git".into()
        } else {
            format!(" {}", self.branch)
        };

        let line = Line::from(vec![
            Span::styled(
                format!("  {focus}  "),
                Style::default()
                    .fg(Theme::get().status_fg)
                    .bg(Theme::get().accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {tip}  "),
                Style::default().fg(Theme::get().fg).bg(Theme::get().accent_soft),
            ),
            if self.pointer_on {
                Span::styled(
                    "  clickable  ",
                    Style::default()
                        .fg(Theme::get().bg)
                        .bg(Theme::get().accent_glow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    "  ? help  ",
                    Style::default().fg(Theme::get().fg_dim).bg(Theme::get().accent_soft),
                )
            },
            Span::styled(
                format!("  {branch}  ·  {}  ", self.status),
                Theme::status(),
            ),
        ]);
        f.render_widget(Paragraph::new(line).wrap(Wrap { trim: false }), area);
    }

    fn draw_help(&self, f: &mut Frame<'_>, area: Rect) {
        let width = area.width.min(72).max(40);
        let height = area.height.min(28).max(16);
        let x = area.x + area.width.saturating_sub(width) / 2;
        let y = area.y + area.height.saturating_sub(height) / 2;
        let popup = Rect {
            x,
            y,
            width,
            height,
        };

        f.render_widget(Clear, popup);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::get().accent_glow))
            .style(Style::default().bg(Theme::get().sidebar).fg(Theme::get().fg))
            .title(Span::styled(
                "  Quick help  ·  Esc to close  ",
                Style::default()
                    .fg(Theme::get().bg)
                    .bg(Theme::get().accent_glow)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let lines = vec![
            Line::from(Span::styled(
                "  How to use noir",
                Style::default()
                    .fg(Theme::get().accent_glow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  1. Click a file in FILES (left) to open it"),
            Line::from("  2. Read it in EDITOR — click [x] to close a tab"),
            Line::from("  3. Click TERMINAL (bottom) and run:  codex"),
            Line::from(""),
            Line::from(Span::styled(
                "  Mouse",
                Style::default().fg(Theme::get().git_add).add_modifier(Modifier::BOLD),
            )),
            Line::from("  · Click folders to expand · files to open"),
            Line::from("  · Drag ║ or ═ bars to resize panels"),
            Line::from("  · Click [x] to close · + New for a terminal"),
            Line::from(""),
            Line::from(Span::styled(
                "  Keyboard",
                Style::default().fg(Theme::get().git_add).add_modifier(Modifier::BOLD),
            )),
            Line::from("  Ctrl+T   switch panel     Esc      leave terminal"),
            Line::from("  Ctrl+N   new terminal     Ctrl+W   close tab"),
            Line::from("  Ctrl+D   cycle diff        Ctrl+G   git pane"),
            Line::from("  Ctrl+L   blame file        Ctrl+B   hide files"),
            Line::from("  Ctrl+O   search files     Ctrl+Q   quit"),
            Line::from("  Ctrl+P   color themes     Ctrl+0   cycle theme"),
            Line::from("  F1 / ?   this help        o / f    open search"),
            Line::from(""),
            Line::from(Span::styled(
                "  Git pane (Ctrl+G)",
                Style::default().fg(Theme::get().git_add).add_modifier(Modifier::BOLD),
            )),
            Line::from("  · s / Space stage · u unstage · d unstaged · D staged"),
            Line::from("  · Tab / b branches · Enter checkout · l blame · Esc files"),
            Line::from(""),
            Line::from(Span::styled(
                format!("  Current theme: {}", Theme::id().name()),
                Style::default().fg(Theme::get().accent_glow),
            )),
            Line::from(Span::styled(
                "  Tip: keep Codex/Claude in the bottom panel — files reload live.",
                Style::default().fg(Theme::get().fg_dim),
            )),
        ];
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn draw_search(&self, f: &mut Frame<'_>, area: Rect) {
        let width = area.width.min(78).max(42);
        let list_h = (self.search_hits.len() as u16).clamp(3, 16);
        let height = (list_h + 7).min(area.height.saturating_sub(2)).max(10);
        let x = area.x + area.width.saturating_sub(width) / 2;
        let y = area.y + area.height.saturating_sub(height) / 2;
        let popup = Rect {
            x,
            y,
            width,
            height,
        };

        f.render_widget(Clear, popup);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::get().accent_glow))
            .style(Style::default().bg(Theme::get().sidebar).fg(Theme::get().fg))
            .title(Span::styled(
                "  Search files  ·  Enter open  ·  Esc  ",
                Style::default()
                    .fg(Theme::get().bg)
                    .bg(Theme::get().accent_glow)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let query_display = if self.search_query.is_empty() {
            "▌".to_string()
        } else {
            format!("{}▌", self.search_query)
        };
        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                format!("  > {query_display}"),
                Style::default()
                    .fg(Theme::get().accent_glow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!(
                    "  {} / {} files",
                    self.search_hits.len(),
                    self.file_index.paths.len()
                ),
                Style::default().fg(Theme::get().fg_dim),
            )),
            Line::from(""),
        ];

        if self.search_hits.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No matches",
                Style::default().fg(Theme::get().fg_muted),
            )));
        } else {
            let max_rows = inner.height.saturating_sub(4) as usize;
            let start = self
                .search_cursor
                .saturating_sub(max_rows.saturating_sub(1) / 2)
                .min(self.search_hits.len().saturating_sub(max_rows));
            for (i, hit) in self
                .search_hits
                .iter()
                .enumerate()
                .skip(start)
                .take(max_rows)
            {
                let selected = i == self.search_cursor;
                let mark = if selected { "›" } else { " " };
                let label = format!("  {mark} {}  ", hit.rel);
                let style = if selected {
                    Style::default()
                        .bg(Theme::get().selection)
                        .fg(Theme::get().accent_glow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::file_color(&hit.rel))
                };
                lines.push(Line::from(Span::styled(label, style)));
            }
        }

        f.render_widget(Paragraph::new(lines), inner);
    }

    fn draw_themes(&self, f: &mut Frame<'_>, area: Rect) {
        let width = area.width.min(56).max(36);
        let height = (PaletteId::ALL.len() as u16 + 6).min(area.height.saturating_sub(2));
        let x = area.x + area.width.saturating_sub(width) / 2;
        let y = area.y + area.height.saturating_sub(height) / 2;
        let popup = Rect {
            x,
            y,
            width,
            height,
        };

        f.render_widget(Clear, popup);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::get().accent_glow))
            .style(Style::default().bg(Theme::get().sidebar).fg(Theme::get().fg))
            .title(Span::styled(
                "  Color themes  ·  Enter select  ·  Esc  ",
                Style::default()
                    .fg(Theme::get().bg)
                    .bg(Theme::get().accent_glow)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                "  Pick a palette (also: Ctrl+0 to cycle)",
                Style::default().fg(Theme::get().fg_dim),
            )),
            Line::from(""),
        ];
        for (i, id) in PaletteId::ALL.iter().enumerate() {
            let selected = i == self.theme_cursor;
            let active = *id == Theme::id();
            let mark = if active { "●" } else { "○" };
            let label = format!("  {} {}. {:<18}  ", mark, i + 1, id.name());
            let style = if selected {
                Style::default()
                    .bg(Theme::get().selection)
                    .fg(Theme::get().accent_glow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::get().fg)
            };
            lines.push(Line::from(Span::styled(label, style)));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Saved to ~/.noir/theme",
            Style::default().fg(Theme::get().fg_muted),
        )));
        f.render_widget(Paragraph::new(lines), inner);
    }
}

fn welcome_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "   Get started in 3 clicks",
            Style::default()
                .fg(Theme::get().accent_glow)
                .bg(Theme::get().bg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "   1.  Click a file on the left",
            Style::default().fg(Theme::get().fg).bg(Theme::get().bg),
        )),
        Line::from(Span::styled(
            "   2.  Read it here (tabs stay open)",
            Style::default().fg(Theme::get().fg).bg(Theme::get().bg),
        )),
        Line::from(Span::styled(
            "   3.  Click the bottom panel → type  codex",
            Style::default().fg(Theme::get().fg).bg(Theme::get().bg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "   Drag the ═ bar to give the terminal more room",
            Style::default().fg(Theme::get().fg_dim).bg(Theme::get().bg),
        )),
        Line::from(Span::styled(
            "   Press  ?  anytime for the full cheat sheet",
            Style::default().fg(Theme::get().fg_dim).bg(Theme::get().bg),
        )),
    ]
}

fn point_in(area: Rect, col: u16, row: u16) -> bool {
    col >= area.x
        && row >= area.y
        && col < area.x.saturating_add(area.width)
        && row < area.y.saturating_add(area.height)
}
