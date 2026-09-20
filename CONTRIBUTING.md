# Contributing to noir

Thanks for wanting to help. This doc covers how to hack on noir locally and send changes upstream.

## Prerequisites

- **Git**
- **Rust** via [rustup](https://rustup.rs) (stable toolchain)
- A mouse-capable terminal (iTerm2, Kitty, WezTerm, Ghostty, Windows Terminal, …)

```bash
rustc --version
cargo --version
```

## Fork & clone

1. Fork [Mudit-Jxin7/noir](https://github.com/Mudit-Jxin7/noir) on GitHub.
2. Clone your fork:

```bash
git clone https://github.com/<your-username>/noir.git
cd noir
git remote add upstream https://github.com/Mudit-Jxin7/noir.git
```

## Build & run

```bash
# fast compile while iterating
cargo run -- .

# release build (closer to what users install)
cargo run --release -- /path/to/a/test/repo

# typecheck only
cargo check

# release binary
cargo build --release
./target/release/noir .
```

Install your local build onto PATH (optional):

```bash
cargo install --path . --force
```

## Project layout

| Path | Role |
|---|---|
| `src/main.rs` | CLI entry, terminal setup/teardown |
| `src/app.rs` | UI loop, layout, mouse/keyboard |
| `src/tree.rs` | File explorer |
| `src/viewer.rs` | Read-only file / diff viewer + syntax highlight |
| `src/term_pane.rs` | Embedded PTY terminals |
| `src/git.rs` | Branch + porcelain status + diff |
| `src/watch.rs` | Filesystem watcher for live reload |
| `src/theme.rs` | Color palettes + `~/.noir/theme` |

## Before you open a PR

1. Rebase / sync with upstream `main`:

```bash
git fetch upstream
git rebase upstream/main
```

2. Make sure it builds:

```bash
cargo check
cargo build --release
```

3. Manually smoke-test:
   - Open a repo, click a file, scroll the viewer
   - Open a terminal tab, type a command
   - Toggle a theme (`Ctrl+P`), quit cleanly (`Ctrl+Q`) — terminal should restore

4. Keep PRs focused — one idea per PR when possible.

## Pull request checklist

- [ ] Clear description of *what* changed and *why*
- [ ] `cargo check` / `cargo build --release` succeed
- [ ] No unrelated formatting churn
- [ ] README / docs updated if user-facing behavior changed

## Commit style

Prefer short imperative summaries, e.g.:

```
Add scroll sync for explorer hover row

- Track hover index in FileTree
- Highlight hovered row in draw_tree
```

## Reporting issues

Open an issue at [github.com/Mudit-Jxin7/noir/issues](https://github.com/Mudit-Jxin7/noir/issues) with:

- OS + terminal emulator
- How you installed noir (`cargo install --git` or from source)
- Steps to reproduce
- Expected vs actual behavior

## License

By contributing, you agree that your contributions are licensed under the MIT license (same as the project).
