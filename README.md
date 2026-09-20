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

## Install (easiest)

One command — no clone required. Rust/`cargo` must already be installed (see [Prerequisites](#prerequisites)).

```bash
cargo install --git https://github.com/Mudit-Jxin7/noir --locked
```

That builds noir and puts the `noir` binary on your PATH (usually `~/.cargo/bin`). Then:

```bash
noir --help
noir .
```

Update later with the same command:

```bash
cargo install --git https://github.com/Mudit-Jxin7/noir --locked --force
```

## Build from source

If you prefer a local checkout (or cargo install is unavailable):

```bash
git clone https://github.com/Mudit-Jxin7/noir.git
cd noir
cargo build --release
./target/release/noir /path/to/your/project
```

Optional: copy onto PATH

```bash
cp target/release/noir "$HOME/.local/bin/noir"
```

## Local development

For hacking on noir itself, see [CONTRIBUTING.md](CONTRIBUTING.md). Quick start:

```bash
git clone https://github.com/Mudit-Jxin7/noir.git
cd noir
cargo run --release -- .
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

Prefer `cargo install --git …` above. If you built from source instead, either alias the binary or copy it onto PATH as in [Build from source](#build-from-source).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for fork/clone, build, PR checklist, and issue reports.

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
