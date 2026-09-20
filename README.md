# noir

Lightweight terminal IDE for Codex workflows: **clickable file tree**, **read-only code/diff viewer**, and an **embedded terminal** — without Electron.

Repo: [https://github.com/Mudit-Jxin7/noir](https://github.com/Mudit-Jxin7/noir)

## Features (MVP)

- VS Code–style dark chrome (title bar, tabs, status bar)
- Click folders to expand / collapse
- Click files to open (read-only, syntax highlighted)
- Git status badges + `Ctrl-D` diff vs `HEAD`
- Live reload when Codex/shell saves the open file
- **Multiple terminals** — click `+`, or `Ctrl+N` / `Ctrl+\``
- Mouse + keyboard navigation

## Prerequisites

Install these once on your machine:

1. **Git** — [https://git-scm.com/downloads](https://git-scm.com/downloads)
2. **Rust (via rustup)** — [https://rustup.rs](https://rustup.rs)

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version   # confirm it works
```

A modern terminal with mouse support works best (iTerm2, Kitty, WezTerm, Ghostty, or Windows Terminal).

## Setup from scratch

Use this if you are installing noir on a new computer.

```bash
# 1. Clone the repo
git clone https://github.com/Mudit-Jxin7/noir.git
cd noir

# 2. Build a release binary
cargo build --release

# 3. (Optional) put `noir` on your PATH
cp target/release/noir "$HOME/.local/bin/noir"
# ensure ~/.local/bin is on PATH, then:
noir --help
```

If you prefer not to copy the binary, run it directly:

```bash
./target/release/noir /path/to/your/project
```

## Local development setup

For contributing or hacking on noir itself:

```bash
git clone https://github.com/Mudit-Jxin7/noir.git
cd noir

# debug build (faster compile, slower runtime)
cargo run -- .

# release build while developing
cargo run --release -- /path/to/repo

# run tests / check compile
cargo check
cargo build --release
```

Rebuild after pulling changes:

```bash
git pull
cargo build --release
```

## How to use

### Open a project

```bash
# current directory
noir .

# any repo / folder
noir ~/path/to/repo

# custom shell in the terminal pane
noir -c /bin/zsh ~/path/to/repo

# start with a color theme
noir --theme dracula ~/path/to/repo
```

### Typical workflow

1. Launch noir on your project root.
2. Click a file in the left **Explorer** to open it (read-only, syntax highlighted).
3. Click the bottom **Terminal** pane and run your agent / shell, e.g. `codex`.
4. When the agent saves a file, the viewer reloads automatically.
5. Press `Ctrl+D` (editor focused) for a diff vs `HEAD`.
6. Press `?` for the in-app cheat sheet, `Ctrl+Q` to quit.

### Install tip (macOS / Linux)

After `cargo build --release`, you can alias it in your shell config:

```bash
alias noir="$HOME/path/to/noir/target/release/noir"
```

Or copy the binary somewhere on your `PATH` as shown in [Setup from scratch](#setup-from-scratch).

## Keybindings

| Key / action | What it does |
|---|---|
| Click file | Open as a new editor tab (multi-file) |
| Click tab `×` | Close that file or terminal |
| Click `＋` | New terminal |
| Wheel on explorer | Smooth viewport scroll (+ scrollbar) |
| Hover tabs / tree / buttons | Pointer cursor (iTerm2 / Kitty / WezTerm / Ghostty) |
| `Ctrl+N` / `Ctrl+\`` | New terminal |
| `Ctrl+W` | Close active file tab or terminal |
| `Ctrl+[` / `Ctrl+]` | Prev / next file tab |
| `Ctrl+Tab` | Next terminal |
| `Ctrl+T` | Cycle focus |
| `Ctrl+D` | Diff (editor focus) |
| `Ctrl+P` | Color theme picker |
| `Ctrl+0` | Cycle theme |
| `Ctrl+Q` | Quit |

## Color themes

`github-dark` · `dracula` · `catppuccin` · `nord` · `tokyo-night` · `gruvbox` · `one-dark` · `solarized`

```bash
noir --theme dracula .
# in-app: Ctrl+P  ·  saved to ~/.noir/theme
```

## Memory goal

Single Rust binary. Expect tens of MB RSS idle — not gigabytes.

## Layout

```
┌─ Explorer ────┬─ VIEW / DIFF ──────────────┐
│ ▸ folders     │  syntax-highlighted file   │
│ click to open │  auto-reloads on save      │
├───────────────┴────────────────────────────┤
│ Terminal — run `codex` here                │
└────────────────────────────────────────────┘
```

## License

MIT
