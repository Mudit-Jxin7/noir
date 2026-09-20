# noir

Lightweight terminal IDE for Codex workflows: **clickable file tree**, **read-only code/diff viewer**, and an **embedded terminal** — without Electron.

## Features (MVP)

- VS Code–style dark chrome (title bar, tabs, status bar)
- Click folders to expand / collapse
- Click files to open (read-only, syntax highlighted)
- Git status badges + `Ctrl-D` diff vs `HEAD`
- Live reload when Codex/shell saves the open file
- **Multiple terminals** — click `+`, or `Ctrl+N` / `Ctrl+\``
- Mouse + keyboard navigation

## Install / run

```bash
# needs Rust (rustup)
cargo build --release
./target/release/noir /path/to/repo

# or
cargo run --release -- .
```

Optional shell:

```bash
noir -c /bin/zsh ~/path/to/repo
```

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
