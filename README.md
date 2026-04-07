# Harvest Claude

> Harvest Moon meets Claude Code — a pixel-art farm where your AI agents come to life.

Terminal pixel-art visualizer that renders running [Claude Code](https://docs.anthropic.com/en/docs/claude-code) agents as animated characters in a Harvest Moon-style office scene.

## Install

Requires Rust 1.85+ and a 24-bit color terminal (iTerm2, Kitty, WezTerm, Alacritty, Windows Terminal).

```sh
cargo install --path .
```

## Usage

```sh
harvest-claude
```

Characters spawn automatically as Claude Code agents start working. Each agent becomes a pixel-art character that walks to a desk, types when executing tools, and shows speech bubbles for permission requests.

```sh
# Watch additional directories for JSONL sessions
harvest-claude --watch-dir /path/to/project

# Use a custom office layout
harvest-claude --layout ~/.pixel-agents/layout.json
```

**Controls:**

| Key | Action |
|-----|--------|
| `←` `→` | Cycle through agents |
| Click | Select a character |
| Escape | Deselect / dismiss bubble |
| `q` | Quit |

## API

### CLI Options

| Flag | Description |
|------|-------------|
| `--watch-dir <PATH>` | Additional directories to watch for JSONL sessions (repeatable) |
| `--layout <PATH>` | Path to a custom layout JSON file |

### Agent Detection

Watches `~/.claude/projects/` for `.jsonl` transcript files. Each active session maps to a character with one of four states:

| State | Visual | Trigger |
|-------|--------|---------|
| Active | Typing animation at desk | Tool executing |
| Idle | Wandering the office | No active tools |
| Waiting | Standing still | Awaiting user input |
| Permission | Amber "..." bubble | 7s without tool result |

### Rendering

8×16 pixel sprites rendered via Unicode half-block characters (`▀`) with 24-bit color. Scene composited with z-sorting at ~60 FPS. Layout compatible with [pixel-agents](https://marketplace.visualstudio.com/items?itemName=anthropic.pixel-agents) VS Code extension (`~/.pixel-agents/layout.json`).

## Documentation

- [`docs/idea.md`](docs/idea.md) — Concepts, terminology, and data flow contracts.
- [`docs/design.md`](docs/design.md) — Module architecture, ownership model, and key types.

## For Agents

Agent-consumable documentation index at [`docs/llms.txt`](docs/llms.txt) ([llmstxt.org](https://llmstxt.org) format).

## License

MIT
