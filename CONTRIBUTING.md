# Contributing to canban

Thanks for your interest in contributing! This guide will help you get started.

## Getting started

```bash
git clone https://github.com/thearnavrustagi/canban.git
cd canban
cargo run
```

You'll need **Rust 1.70+** installed. If you don't have it, grab it from [rustup.rs](https://rustup.rs).

## Project structure

```
src/
├── main.rs          # CLI entry point
├── app.rs           # Application state & event loop
├── config.rs        # Configuration loading
├── event.rs         # Input event polling
├── model/           # Data types (Board, Task, Column)
├── storage/         # Persistence (JSON, CSV)
└── ui/              # Terminal rendering (ratatui)
```

## Development workflow

1. **Fork** the repo and create a branch from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```
2. **Make your changes** — keep functions short (8 lines max for logic).
3. **Test** your changes:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```
4. **Commit** with a clear message:
   ```
   feat: add task priority levels
   fix: prevent crash when board file is empty
   docs: update keybinding table
   refactor: extract card rendering into helper
   ```
5. **Push** and open a pull request.

## Code style

- Follow the [Rust style guide](https://doc.rust-lang.org/style-guide/)
- No function doing computation should exceed **8 lines** — break it down
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- Avoid unnecessary `unwrap()` — use `color_eyre` for error handling

## What to work on

Check the [issue tracker](https://github.com/thearnavrustagi/canban/issues) for open issues. Good labels to look for:

- `good first issue` — small, well-scoped tasks for newcomers
- `help wanted` — issues where maintainers need help
- `bug` — confirmed bugs
- `enhancement` — feature requests

If you want to work on something that isn't tracked yet, open an issue first to discuss it.

## Pull request guidelines

- **One concern per PR** — don't mix refactors with features
- **Write tests** for new functionality
- **Update the README** if your change adds or modifies user-facing behavior
- **Keep commits atomic** — each commit should build and pass tests
- PRs are squash-merged into `main`

## Adding a new feature

If you're adding a significant feature:

1. Open an issue describing the feature and its motivation
2. Wait for feedback before investing time in implementation
3. Consider backward compatibility with existing board data
4. Add keybinding documentation to the README if applicable

## UI changes

When modifying the UI:

- Test with different terminal sizes (minimum: 80x24)
- Test with both light and dark terminal backgrounds
- Check that colors render well in 256-color mode
- Use the existing color palette from `src/ui/theme.rs`

## Reporting bugs

When filing a bug report, include:

- Your OS and terminal emulator
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs actual behavior
- Terminal screenshot if relevant

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
