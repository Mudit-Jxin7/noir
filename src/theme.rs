//! Selectable color palettes for noir.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteId {
    GitHubDark,
    Dracula,
    Catppuccin,
    Nord,
    TokyoNight,
    Gruvbox,
    OneDark,
    Solarized,
}

impl PaletteId {
    pub const ALL: [PaletteId; 8] = [
        PaletteId::GitHubDark,
        PaletteId::Dracula,
        PaletteId::Catppuccin,
        PaletteId::Nord,
        PaletteId::TokyoNight,
        PaletteId::Gruvbox,
        PaletteId::OneDark,
        PaletteId::Solarized,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PaletteId::GitHubDark => "GitHub Dark",
            PaletteId::Dracula => "Dracula",
            PaletteId::Catppuccin => "Catppuccin Mocha",
            PaletteId::Nord => "Nord",
            PaletteId::TokyoNight => "Tokyo Night",
            PaletteId::Gruvbox => "Gruvbox Dark",
            PaletteId::OneDark => "One Dark",
            PaletteId::Solarized => "Solarized Dark",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            PaletteId::GitHubDark => "github-dark",
            PaletteId::Dracula => "dracula",
            PaletteId::Catppuccin => "catppuccin",
            PaletteId::Nord => "nord",
            PaletteId::TokyoNight => "tokyo-night",
            PaletteId::Gruvbox => "gruvbox",
            PaletteId::OneDark => "one-dark",
            PaletteId::Solarized => "solarized",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();
        Self::ALL
            .iter()
            .copied()
            .find(|p| p.id() == s || p.name().to_lowercase() == s)
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Self {
        Self::ALL[i % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub sidebar: Color,
    pub panel: Color,
    pub panel_elevated: Color,
    pub tab_bar: Color,
    pub tab_active: Color,
    pub tab_inactive: Color,
    pub tab_hover: Color,
    pub status: Color,
    pub status_fg: Color,
    pub titlebar: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub fg_muted: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub accent_glow: Color,
    pub border: Color,
    pub border_focus: Color,
    pub selection: Color,
    pub close: Color,
    pub close_muted: Color,
    pub git_mod: Color,
    pub git_add: Color,
    pub git_del: Color,
    pub folder: Color,
    pub file: Color,
    pub ft_ts: Color,
    pub ft_js: Color,
    pub ft_rs: Color,
    pub ft_go: Color,
    pub ft_py: Color,
    pub ft_md: Color,
    pub ft_json: Color,
    pub ft_yaml: Color,
    pub ft_toml: Color,
    pub ft_css: Color,
    pub ft_html: Color,
    pub ft_sh: Color,
    pub ft_docker: Color,
    pub ft_lock: Color,
    pub ft_test: Color,
    pub ft_config: Color,
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

impl Palette {
    pub fn github_dark() -> Self {
        Self {
            bg: rgb(13, 17, 23),
            sidebar: rgb(22, 27, 34),
            panel: rgb(1, 4, 9),
            panel_elevated: rgb(13, 17, 23),
            tab_bar: rgb(22, 27, 34),
            tab_active: rgb(13, 17, 23),
            tab_inactive: rgb(33, 38, 45),
            tab_hover: rgb(48, 54, 61),
            status: rgb(31, 111, 235),
            status_fg: rgb(255, 255, 255),
            titlebar: rgb(22, 27, 34),
            fg: rgb(230, 237, 243),
            fg_dim: rgb(139, 148, 158),
            fg_muted: rgb(110, 118, 129),
            accent: rgb(31, 111, 235),
            accent_soft: rgb(26, 84, 184),
            accent_glow: rgb(88, 166, 255),
            border: rgb(48, 54, 61),
            border_focus: rgb(88, 166, 255),
            selection: rgb(26, 54, 93),
            close: rgb(248, 81, 73),
            close_muted: rgb(110, 118, 129),
            git_mod: rgb(210, 153, 34),
            git_add: rgb(63, 185, 80),
            git_del: rgb(248, 81, 73),
            folder: rgb(121, 192, 255),
            file: rgb(201, 209, 217),
            ft_ts: rgb(79, 193, 255),
            ft_js: rgb(241, 224, 90),
            ft_rs: rgb(222, 165, 132),
            ft_go: rgb(0, 173, 216),
            ft_py: rgb(255, 222, 120),
            ft_md: rgb(130, 170, 255),
            ft_json: rgb(203, 166, 247),
            ft_yaml: rgb(247, 118, 142),
            ft_toml: rgb(158, 206, 106),
            ft_css: rgb(86, 156, 214),
            ft_html: rgb(228, 135, 106),
            ft_sh: rgb(125, 209, 143),
            ft_docker: rgb(56, 152, 236),
            ft_lock: rgb(110, 118, 129),
            ft_test: rgb(63, 185, 80),
            ft_config: rgb(210, 153, 34),
        }
    }

    pub fn dracula() -> Self {
        Self {
            bg: rgb(40, 42, 54),
            sidebar: rgb(33, 34, 44),
            panel: rgb(30, 31, 41),
            panel_elevated: rgb(40, 42, 54),
            tab_bar: rgb(33, 34, 44),
            tab_active: rgb(40, 42, 54),
            tab_inactive: rgb(68, 71, 90),
            tab_hover: rgb(98, 114, 164),
            status: rgb(189, 147, 249),
            status_fg: rgb(248, 248, 242),
            titlebar: rgb(33, 34, 44),
            fg: rgb(248, 248, 242),
            fg_dim: rgb(98, 114, 164),
            fg_muted: rgb(68, 71, 90),
            accent: rgb(189, 147, 249),
            accent_soft: rgb(98, 114, 164),
            accent_glow: rgb(139, 233, 253),
            border: rgb(68, 71, 90),
            border_focus: rgb(189, 147, 249),
            selection: rgb(68, 71, 90),
            close: rgb(255, 85, 85),
            close_muted: rgb(98, 114, 164),
            git_mod: rgb(241, 250, 140),
            git_add: rgb(80, 250, 123),
            git_del: rgb(255, 85, 85),
            folder: rgb(139, 233, 253),
            file: rgb(248, 248, 242),
            ft_ts: rgb(139, 233, 253),
            ft_js: rgb(241, 250, 140),
            ft_rs: rgb(255, 184, 108),
            ft_go: rgb(139, 233, 253),
            ft_py: rgb(241, 250, 140),
            ft_md: rgb(189, 147, 249),
            ft_json: rgb(255, 121, 198),
            ft_yaml: rgb(255, 85, 85),
            ft_toml: rgb(80, 250, 123),
            ft_css: rgb(139, 233, 253),
            ft_html: rgb(255, 184, 108),
            ft_sh: rgb(80, 250, 123),
            ft_docker: rgb(139, 233, 253),
            ft_lock: rgb(98, 114, 164),
            ft_test: rgb(80, 250, 123),
            ft_config: rgb(241, 250, 140),
        }
    }

    pub fn catppuccin() -> Self {
        Self {
            bg: rgb(30, 30, 46),
            sidebar: rgb(24, 24, 37),
            panel: rgb(17, 17, 27),
            panel_elevated: rgb(30, 30, 46),
            tab_bar: rgb(24, 24, 37),
            tab_active: rgb(30, 30, 46),
            tab_inactive: rgb(49, 50, 68),
            tab_hover: rgb(69, 71, 90),
            status: rgb(137, 180, 250),
            status_fg: rgb(205, 214, 244),
            titlebar: rgb(24, 24, 37),
            fg: rgb(205, 214, 244),
            fg_dim: rgb(127, 132, 156),
            fg_muted: rgb(108, 112, 134),
            accent: rgb(137, 180, 250),
            accent_soft: rgb(69, 71, 90),
            accent_glow: rgb(116, 199, 236),
            border: rgb(49, 50, 68),
            border_focus: rgb(137, 180, 250),
            selection: rgb(69, 71, 90),
            close: rgb(243, 139, 168),
            close_muted: rgb(108, 112, 134),
            git_mod: rgb(249, 226, 175),
            git_add: rgb(166, 227, 161),
            git_del: rgb(243, 139, 168),
            folder: rgb(137, 180, 250),
            file: rgb(205, 214, 244),
            ft_ts: rgb(116, 199, 236),
            ft_js: rgb(249, 226, 175),
            ft_rs: rgb(250, 179, 135),
            ft_go: rgb(148, 226, 213),
            ft_py: rgb(249, 226, 175),
            ft_md: rgb(180, 190, 254),
            ft_json: rgb(203, 166, 247),
            ft_yaml: rgb(243, 139, 168),
            ft_toml: rgb(166, 227, 161),
            ft_css: rgb(137, 180, 250),
            ft_html: rgb(250, 179, 135),
            ft_sh: rgb(166, 227, 161),
            ft_docker: rgb(137, 180, 250),
            ft_lock: rgb(108, 112, 134),
            ft_test: rgb(166, 227, 161),
            ft_config: rgb(249, 226, 175),
        }
    }

    pub fn nord() -> Self {
        Self {
            bg: rgb(46, 52, 64),
            sidebar: rgb(59, 66, 82),
            panel: rgb(36, 41, 51),
            panel_elevated: rgb(46, 52, 64),
            tab_bar: rgb(59, 66, 82),
            tab_active: rgb(46, 52, 64),
            tab_inactive: rgb(67, 76, 94),
            tab_hover: rgb(76, 86, 106),
            status: rgb(136, 192, 208),
            status_fg: rgb(236, 239, 244),
            titlebar: rgb(59, 66, 82),
            fg: rgb(236, 239, 244),
            fg_dim: rgb(216, 222, 233),
            fg_muted: rgb(129, 161, 193),
            accent: rgb(136, 192, 208),
            accent_soft: rgb(94, 129, 172),
            accent_glow: rgb(143, 188, 187),
            border: rgb(76, 86, 106),
            border_focus: rgb(136, 192, 208),
            selection: rgb(67, 76, 94),
            close: rgb(191, 97, 106),
            close_muted: rgb(129, 161, 193),
            git_mod: rgb(235, 203, 139),
            git_add: rgb(163, 190, 140),
            git_del: rgb(191, 97, 106),
            folder: rgb(129, 161, 193),
            file: rgb(236, 239, 244),
            ft_ts: rgb(136, 192, 208),
            ft_js: rgb(235, 203, 139),
            ft_rs: rgb(208, 135, 112),
            ft_go: rgb(136, 192, 208),
            ft_py: rgb(235, 203, 139),
            ft_md: rgb(180, 142, 173),
            ft_json: rgb(180, 142, 173),
            ft_yaml: rgb(191, 97, 106),
            ft_toml: rgb(163, 190, 140),
            ft_css: rgb(129, 161, 193),
            ft_html: rgb(208, 135, 112),
            ft_sh: rgb(163, 190, 140),
            ft_docker: rgb(136, 192, 208),
            ft_lock: rgb(129, 161, 193),
            ft_test: rgb(163, 190, 140),
            ft_config: rgb(235, 203, 139),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            bg: rgb(26, 27, 38),
            sidebar: rgb(22, 22, 30),
            panel: rgb(16, 16, 24),
            panel_elevated: rgb(26, 27, 38),
            tab_bar: rgb(22, 22, 30),
            tab_active: rgb(26, 27, 38),
            tab_inactive: rgb(41, 42, 58),
            tab_hover: rgb(54, 56, 76),
            status: rgb(122, 162, 247),
            status_fg: rgb(192, 202, 245),
            titlebar: rgb(22, 22, 30),
            fg: rgb(192, 202, 245),
            fg_dim: rgb(86, 95, 137),
            fg_muted: rgb(65, 72, 104),
            accent: rgb(122, 162, 247),
            accent_soft: rgb(41, 46, 86),
            accent_glow: rgb(125, 207, 255),
            border: rgb(41, 42, 58),
            border_focus: rgb(122, 162, 247),
            selection: rgb(41, 46, 86),
            close: rgb(247, 118, 142),
            close_muted: rgb(86, 95, 137),
            git_mod: rgb(224, 175, 104),
            git_add: rgb(158, 206, 106),
            git_del: rgb(247, 118, 142),
            folder: rgb(122, 162, 247),
            file: rgb(192, 202, 245),
            ft_ts: rgb(125, 207, 255),
            ft_js: rgb(224, 175, 104),
            ft_rs: rgb(255, 158, 100),
            ft_go: rgb(125, 207, 255),
            ft_py: rgb(224, 175, 104),
            ft_md: rgb(187, 154, 247),
            ft_json: rgb(187, 154, 247),
            ft_yaml: rgb(247, 118, 142),
            ft_toml: rgb(158, 206, 106),
            ft_css: rgb(122, 162, 247),
            ft_html: rgb(255, 158, 100),
            ft_sh: rgb(158, 206, 106),
            ft_docker: rgb(122, 162, 247),
            ft_lock: rgb(86, 95, 137),
            ft_test: rgb(158, 206, 106),
            ft_config: rgb(224, 175, 104),
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            bg: rgb(40, 40, 40),
            sidebar: rgb(50, 48, 47),
            panel: rgb(29, 32, 33),
            panel_elevated: rgb(40, 40, 40),
            tab_bar: rgb(50, 48, 47),
            tab_active: rgb(40, 40, 40),
            tab_inactive: rgb(60, 56, 54),
            tab_hover: rgb(80, 73, 69),
            status: rgb(215, 153, 33),
            status_fg: rgb(235, 219, 178),
            titlebar: rgb(50, 48, 47),
            fg: rgb(235, 219, 178),
            fg_dim: rgb(168, 153, 132),
            fg_muted: rgb(146, 131, 116),
            accent: rgb(215, 153, 33),
            accent_soft: rgb(102, 92, 84),
            accent_glow: rgb(250, 189, 47),
            border: rgb(80, 73, 69),
            border_focus: rgb(250, 189, 47),
            selection: rgb(60, 56, 54),
            close: rgb(204, 36, 29),
            close_muted: rgb(146, 131, 116),
            git_mod: rgb(215, 153, 33),
            git_add: rgb(152, 151, 26),
            git_del: rgb(204, 36, 29),
            folder: rgb(69, 133, 136),
            file: rgb(235, 219, 178),
            ft_ts: rgb(69, 133, 136),
            ft_js: rgb(215, 153, 33),
            ft_rs: rgb(214, 93, 14),
            ft_go: rgb(69, 133, 136),
            ft_py: rgb(215, 153, 33),
            ft_md: rgb(177, 98, 134),
            ft_json: rgb(177, 98, 134),
            ft_yaml: rgb(204, 36, 29),
            ft_toml: rgb(152, 151, 26),
            ft_css: rgb(69, 133, 136),
            ft_html: rgb(214, 93, 14),
            ft_sh: rgb(152, 151, 26),
            ft_docker: rgb(69, 133, 136),
            ft_lock: rgb(146, 131, 116),
            ft_test: rgb(152, 151, 26),
            ft_config: rgb(215, 153, 33),
        }
    }

    pub fn one_dark() -> Self {
        Self {
            bg: rgb(40, 44, 52),
            sidebar: rgb(33, 37, 43),
            panel: rgb(28, 31, 36),
            panel_elevated: rgb(40, 44, 52),
            tab_bar: rgb(33, 37, 43),
            tab_active: rgb(40, 44, 52),
            tab_inactive: rgb(50, 56, 66),
            tab_hover: rgb(60, 66, 78),
            status: rgb(97, 175, 239),
            status_fg: rgb(171, 178, 191),
            titlebar: rgb(33, 37, 43),
            fg: rgb(171, 178, 191),
            fg_dim: rgb(92, 99, 112),
            fg_muted: rgb(75, 80, 92),
            accent: rgb(97, 175, 239),
            accent_soft: rgb(55, 62, 74),
            accent_glow: rgb(86, 182, 194),
            border: rgb(60, 66, 78),
            border_focus: rgb(97, 175, 239),
            selection: rgb(55, 62, 74),
            close: rgb(224, 108, 117),
            close_muted: rgb(92, 99, 112),
            git_mod: rgb(229, 192, 123),
            git_add: rgb(152, 195, 121),
            git_del: rgb(224, 108, 117),
            folder: rgb(97, 175, 239),
            file: rgb(171, 178, 191),
            ft_ts: rgb(86, 182, 194),
            ft_js: rgb(229, 192, 123),
            ft_rs: rgb(209, 154, 102),
            ft_go: rgb(86, 182, 194),
            ft_py: rgb(229, 192, 123),
            ft_md: rgb(198, 120, 221),
            ft_json: rgb(198, 120, 221),
            ft_yaml: rgb(224, 108, 117),
            ft_toml: rgb(152, 195, 121),
            ft_css: rgb(97, 175, 239),
            ft_html: rgb(209, 154, 102),
            ft_sh: rgb(152, 195, 121),
            ft_docker: rgb(97, 175, 239),
            ft_lock: rgb(92, 99, 112),
            ft_test: rgb(152, 195, 121),
            ft_config: rgb(229, 192, 123),
        }
    }

    pub fn solarized() -> Self {
        Self {
            bg: rgb(0, 43, 54),
            sidebar: rgb(7, 54, 66),
            panel: rgb(0, 30, 38),
            panel_elevated: rgb(0, 43, 54),
            tab_bar: rgb(7, 54, 66),
            tab_active: rgb(0, 43, 54),
            tab_inactive: rgb(7, 54, 66),
            tab_hover: rgb(88, 110, 117),
            status: rgb(38, 139, 210),
            status_fg: rgb(253, 246, 227),
            titlebar: rgb(7, 54, 66),
            fg: rgb(147, 161, 161),
            fg_dim: rgb(88, 110, 117),
            fg_muted: rgb(101, 123, 131),
            accent: rgb(38, 139, 210),
            accent_soft: rgb(7, 54, 66),
            accent_glow: rgb(42, 161, 152),
            border: rgb(88, 110, 117),
            border_focus: rgb(38, 139, 210),
            selection: rgb(7, 54, 66),
            close: rgb(220, 50, 47),
            close_muted: rgb(88, 110, 117),
            git_mod: rgb(181, 137, 0),
            git_add: rgb(133, 153, 0),
            git_del: rgb(220, 50, 47),
            folder: rgb(38, 139, 210),
            file: rgb(147, 161, 161),
            ft_ts: rgb(42, 161, 152),
            ft_js: rgb(181, 137, 0),
            ft_rs: rgb(203, 75, 22),
            ft_go: rgb(42, 161, 152),
            ft_py: rgb(181, 137, 0),
            ft_md: rgb(108, 113, 196),
            ft_json: rgb(211, 54, 130),
            ft_yaml: rgb(220, 50, 47),
            ft_toml: rgb(133, 153, 0),
            ft_css: rgb(38, 139, 210),
            ft_html: rgb(203, 75, 22),
            ft_sh: rgb(133, 153, 0),
            ft_docker: rgb(38, 139, 210),
            ft_lock: rgb(88, 110, 117),
            ft_test: rgb(133, 153, 0),
            ft_config: rgb(181, 137, 0),
        }
    }

    pub fn for_id(id: PaletteId) -> Self {
        match id {
            PaletteId::GitHubDark => Self::github_dark(),
            PaletteId::Dracula => Self::dracula(),
            PaletteId::Catppuccin => Self::catppuccin(),
            PaletteId::Nord => Self::nord(),
            PaletteId::TokyoNight => Self::tokyo_night(),
            PaletteId::Gruvbox => Self::gruvbox(),
            PaletteId::OneDark => Self::one_dark(),
            PaletteId::Solarized => Self::solarized(),
        }
    }
}

static CURRENT_ID: Mutex<PaletteId> = Mutex::new(PaletteId::GitHubDark);

pub struct Theme;

impl Theme {
    pub fn id() -> PaletteId {
        *CURRENT_ID.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn get() -> Palette {
        Palette::for_id(Self::id())
    }

    pub fn set(id: PaletteId) {
        *CURRENT_ID.lock().unwrap_or_else(|e| e.into_inner()) = id;
        let _ = save_theme(id);
    }

    pub fn cycle() -> PaletteId {
        let next = PaletteId::from_index(Self::id().index() + 1);
        Self::set(next);
        next
    }

    pub fn load_saved() {
        if let Some(id) = load_theme() {
            *CURRENT_ID.lock().unwrap_or_else(|e| e.into_inner()) = id;
        }
    }

    pub fn file_color(name: &str) -> Color {
        let p = Self::get();
        let lower = name.to_lowercase();
        if lower.ends_with(".ts") || lower.ends_with(".tsx") || lower.ends_with(".mts") {
            p.ft_ts
        } else if lower.ends_with(".js")
            || lower.ends_with(".jsx")
            || lower.ends_with(".mjs")
            || lower.ends_with(".cjs")
        {
            p.ft_js
        } else if lower.ends_with(".rs") {
            p.ft_rs
        } else if lower.ends_with(".go") {
            p.ft_go
        } else if lower.ends_with(".py") || lower.ends_with(".pyi") {
            p.ft_py
        } else if lower.ends_with(".md") || lower.ends_with(".mdx") || lower.ends_with(".markdown")
        {
            p.ft_md
        } else if lower.ends_with(".json") || lower.ends_with(".jsonc") {
            p.ft_json
        } else if lower.ends_with(".yml") || lower.ends_with(".yaml") {
            p.ft_yaml
        } else if lower.ends_with(".toml") {
            p.ft_toml
        } else if lower.ends_with(".css")
            || lower.ends_with(".scss")
            || lower.ends_with(".sass")
            || lower.ends_with(".less")
        {
            p.ft_css
        } else if lower.ends_with(".html") || lower.ends_with(".htm") || lower.ends_with(".svg") {
            p.ft_html
        } else if lower.ends_with(".sh")
            || lower.ends_with(".bash")
            || lower.ends_with(".zsh")
            || lower.ends_with(".fish")
        {
            p.ft_sh
        } else if lower == "dockerfile"
            || lower.starts_with("dockerfile.")
            || lower.ends_with(".dockerfile")
        {
            p.ft_docker
        } else if lower.ends_with(".lock")
            || lower == "pnpm-lock.yaml"
            || lower == "package-lock.json"
            || lower == "cargo.lock"
        {
            p.ft_lock
        } else if lower.contains(".test.")
            || lower.contains(".spec.")
            || lower.ends_with("_test.go")
            || lower.ends_with("_test.rs")
        {
            p.ft_test
        } else if lower.starts_with('.')
            || lower.ends_with(".env")
            || lower == "makefile"
            || lower.ends_with(".config.js")
            || lower.ends_with(".config.ts")
        {
            p.ft_config
        } else {
            p.file
        }
    }

    pub fn bg() -> Style {
        let p = Self::get();
        Style::default().bg(p.bg).fg(p.fg)
    }

    pub fn sidebar() -> Style {
        let p = Self::get();
        Style::default().bg(p.sidebar).fg(p.fg)
    }

    pub fn panel() -> Style {
        let p = Self::get();
        Style::default().bg(p.panel).fg(p.fg)
    }

    pub fn border(focused: bool) -> Style {
        let p = Self::get();
        if focused {
            Style::default()
                .fg(p.border_focus)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.border)
        }
    }

    pub fn title(focused: bool) -> Style {
        let p = Self::get();
        if focused {
            Style::default()
                .fg(p.status_fg)
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.fg_dim).bg(p.titlebar)
        }
    }

    pub fn status() -> Style {
        let p = Self::get();
        Style::default().bg(p.status).fg(p.status_fg)
    }

    pub fn tab_inactive() -> Style {
        let p = Self::get();
        Style::default().fg(p.fg_dim).bg(p.tab_inactive)
    }

    pub fn tab_bar() -> Style {
        let p = Self::get();
        Style::default().bg(p.tab_bar).fg(p.fg_dim)
    }

    pub fn button() -> Style {
        let p = Self::get();
        Style::default()
            .fg(p.status_fg)
            .bg(p.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn button_close(active: bool) -> Style {
        let p = Self::get();
        if active {
            Style::default().fg(p.close).bg(p.tab_active)
        } else {
            Style::default().fg(p.close_muted).bg(p.tab_inactive)
        }
    }
}

fn config_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("noir").join("theme");
    }
    dirs_fallback().join(".noir").join("theme")
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn save_theme(id: PaletteId) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, id.id())
}

fn load_theme() -> Option<PaletteId> {
    let raw = fs::read_to_string(config_path()).ok()?;
    PaletteId::from_id(raw.trim())
}
