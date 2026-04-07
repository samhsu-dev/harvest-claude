# Build & Distribution

> Cargo configuration, cross-compilation, release packaging.

## Build

```bash
cargo build                    # Debug build
cargo build --release          # Optimized release build
cargo clippy                   # Lint
cargo test                     # Run tests
cargo run                      # Run debug build
cargo run --release             # Run release build
```

## Cargo.toml

```toml
[package]
name = "pixel-agents-tui"
version = "0.1.0"
edition = "2024"
description = "Pixel art office for your Claude Code agents — in the terminal"
license = "MIT"
repository = "https://github.com/user/pixel-agents-tui"

[dependencies]
ratatui = "0.30"
crossterm = "0.29"
notify = "8.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "6"
clap = { version = "4.6", features = ["derive"] }
rand = "0.10"
color-eyre = "0.6"
tracing = "0.1"
tracing-subscriber = "0.3"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

## Distribution

| Method | Command | Requires |
|--------|---------|----------|
| crates.io | `cargo install pixel-agents-tui` | Rust toolchain |
| cargo-binstall | `cargo binstall pixel-agents-tui` | cargo-binstall |
| GitHub Releases | Download binary | Nothing |
| Homebrew | `brew install pixel-agents-tui` | Homebrew |

## Cross-Compilation

```bash
# macOS ARM → Linux x86_64
cargo install cross
cross build --release --target x86_64-unknown-linux-gnu

# GitHub Actions matrix:
# - x86_64-unknown-linux-gnu
# - x86_64-apple-darwin
# - aarch64-apple-darwin
# - x86_64-pc-windows-msvc
```

## Binary Size

With `lto = true`, `strip = true`, `codegen-units = 1`: ~5-8MB static binary. No runtime dependencies.

## Gotchas

- `cargo install` compiles from source (2-3 minutes). Publish pre-built binaries for fast installation.
- `edition = "2024"` requires Rust 1.85+. Use `edition = "2021"` for wider compatibility.
- Embedded sprites (`const` arrays) increase binary size by ~100KB. Negligible.
