# noir

Terminal IDE for Codex workflows: file tree, read-only viewer, and embedded shell — no Electron.

```
┌─ Explorer ────┬─ Viewer / Diff ────────────┐
│ folders/files │  syntax-highlighted        │
├───────────────┴────────────────────────────┤
│ Terminal — run `codex` here                │
└────────────────────────────────────────────┘
```

`Ctrl+E` docks the terminal on the far right (chat-style) or back underneath.

## Install

**Binary** (no Rust) — download for your OS from [Releases](https://github.com/Mudit-Jxin7/noir/releases/latest):

```bash
tar -xzf noir-*.tar.gz
mkdir -p ~/.local/bin && mv noir ~/.local/bin/
```

Make sure `~/.local/bin` is on your PATH (restart the terminal if `noir` is not found).

| Platform | Asset |
|---|---|
| macOS Apple Silicon | `noir-*-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `noir-*-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `noir-*-x86_64-unknown-linux-gnu.tar.gz` |

**Cargo** (needs [Rust](https://rustup.rs)):

```bash
cargo install --git https://github.com/Mudit-Jxin7/noir --locked
```

## Usage

Install once, then from any project folder:

```bash
cd ~/my-project
noir
```

That opens the current directory. You can also pass a path:

```bash
noir ~/path/to/repo
noir -c /bin/zsh .
noir --theme dracula
```

Themes: `cursor` (default), `github-dark`, `dracula`, `catppuccin`, `nord`, `tokyo-night`, `gruvbox`, `one-dark`, `solarized`.

1. Click a file to open it  
2. Click the terminal pane and run `codex`  
3. Files reload on save · `Ctrl+D` cycles diff · `Ctrl+G` git pane · `Ctrl+O` search files · `?` for help · `Ctrl+Q` to quit  

## Keybindings

| Key | Action |
|---|---|
| Click file / `＋` / tab `×` | Open file · new terminal · close tab |
| `Ctrl+N` / `` Ctrl+` `` | New terminal |
| `Ctrl+W` | Close active tab |
| `Ctrl+[` / `Ctrl+]` | Prev / next file tab |
| `Ctrl+Tab` | Next terminal |
| `Ctrl+T` | Cycle focus |
| `Ctrl+E` | Dock terminal right ↔ bottom |
| `Ctrl+D` | Cycle diff: HEAD → unstaged → staged → file |
| `Ctrl+G` | Git pane (stage list, branches, blame) |
| `Ctrl+L` | Blame active file |
| `Ctrl+P` / `Ctrl+0` | Theme picker / cycle |
| `Ctrl+O` | Search files (fuzzy) |
| `Ctrl+Q` | Quit |

## Benchmarks

| Language | Files Indexed | Index Time | Fuzzy Search | Full-Tree Regex Scan | Resident RAM (RSS) |
|---|---:|---:|---:|---:|---:|
| TypeScript | 14,031 | 121.5 ms | 8.5 ms | 2.84 s | 18.9 MB |
| Python | 8,222 | 167.3 ms | 7.4 ms | 1.62 s | 18.6 MB |
| Svelte | 4,718 | 45.7 ms | 2.4 ms | 724.5 ms | 24.6 MB |
| Go | 3,133 | 36.1 ms | 2.2 ms | 493.8 ms | 24.3 MB |
| TypeScript | 2,024 | 46.4 ms | 1.3 ms | 337.4 ms | 17.6 MB |
| Java | 1,374 | 40.9 ms | 1.1 ms | 179.5 ms | 17.5 MB |
| Python | 787 | 30.6 ms | 491 µs | 125.4 ms | 17.9 MB |
| Java | 545 | 32.8 ms | 356 µs | 73.9 ms | 17.5 MB |
| Shell | 348 | 23.3 ms | 265 µs | 38.5 ms | 18.6 MB |
| JavaScript | 228 | 21.0 ms | 109 µs | 39.6 ms | 25.8 MB |
| Java | 158 | 22.0 ms | 100 µs | 39.5 ms | 24.5 MB |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT
