# Build & Distribution

> Cargo configuration, dependencies, and local install.

## Quick Start

```bash
cargo build --release
cargo install --path .
harvest-claude
```

## Key Files

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Package `harvest-claude` 0.1.0, edition 2024 |
| `rust-toolchain.toml` | Toolchain pin |
| `src/main.rs` | Entry point only |
| `src/lib.rs` | `run` and module declarations |

## Dependencies

| Crate | Version | Use |
|-------|---------|-----|
| `ratatui` | 0.30 | Terminal UI |
| `crossterm` | 0.29 | Terminal events and raw mode |
| `notify` | 8 | File watching |
| `serde` | 1 (derive) | Serialization |
| `serde_json` | 1 | JSON parsing |
| `dirs` | 6 | Home directory |
| `clap` | 4 (derive) | CLI arguments |
| `rand` | 0.9 | Wander and hue shift |
| `color-eyre` | 0.6 | Error reports |
| `tracing` | 0.1 | Log events |
| `tracing-subscriber` | 0.3 (env-filter) | Log filtering |
| `tracing-appender` | 0.2.5 | Rotating log files |
| `unicode-width` | 0.2.2 | Display width |

Dev dependencies: `rstest` 0.27.0, `tempfile` 3.

## Configuration

Quality gate:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
```

Runtime files live in `~/.harvest-claude`: `layout.json`, `warehouse.json`, `logs/`.

## Gotchas

- Window focus needs macOS (`lsof`, `osascript`). Other platforms run without focus.
- Set `RUST_LOG` to filter log output. Logs go to files, never to the terminal.
- No CI workflow and no published-crate release exist in the repository.
