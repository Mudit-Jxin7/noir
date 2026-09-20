# noir

Terminal IDE for Codex workflows: file tree, read-only viewer, and embedded shell — no Electron.

```
┌─ Explorer ────┬─ Viewer / Diff ────────────┐
│ folders/files │  syntax-highlighted        │
├───────────────┴────────────────────────────┤
│ Terminal — run `codex` here                │
└────────────────────────────────────────────┘
```

## Install

**Binary** (no Rust) — grab the archive for your OS from [Releases](https://github.com/Mudit-Jxin7/noir/releases/latest), then:

```bash
tar -xzf noir-*.tar.gz
mkdir -p ~/.local/bin && mv noir ~/.local/bin/
noir .
```

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

```bash
noir .                          # open current folder
noir ~/path/to/repo             # open a repo
noir -c /bin/zsh ~/path/to/repo # custom shell
noir --theme dracula .          # theme: github-dark, dracula, catppuccin, nord, tokyo-night, gruvbox, one-dark, solarized
```

1. Click a file to open it  
2. Click the terminal pane and run `codex`  
3. Files reload on save · `Ctrl+D` for diff · `?` for help · `Ctrl+Q` to quit  

## Keybindings

| Key | Action |
|---|---|
| Click file / `＋` / tab `×` | Open file · new terminal · close tab |
| `Ctrl+N` / `` Ctrl+` `` | New terminal |
| `Ctrl+W` | Close active tab |
| `Ctrl+[` / `Ctrl+]` | Prev / next file tab |
| `Ctrl+Tab` | Next terminal |
| `Ctrl+T` | Cycle focus |
| `Ctrl+D` | Diff vs `HEAD` |
| `Ctrl+P` / `Ctrl+0` | Theme picker / cycle |
| `Ctrl+Q` | Quit |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT
