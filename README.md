# Harvest Claude

> Harvest Moon meets Claude Code — a pixel-art farm where your AI agents come to life.

Terminal pixel-art visualizer that renders running [Claude Code](https://docs.anthropic.com/en/docs/claude-code) agents as animated farm characters in a Stardew Valley-inspired scene using the [Dawnbringer 16-color palette](https://lospec.com/palette-list/dawnbringer-16).

## Install

Requires Rust 1.85+ and a 24-bit color terminal (iTerm2, Kitty, WezTerm, Alacritty, Windows Terminal).

```sh
cargo install --path .
```

## Usage

```sh
harvest-claude
```

Characters spawn automatically as Claude Code agents start working. Each agent becomes a farm worker who tends crops, rests in the cabin, fishes at the pond, or waits at the mailbox. Background sub-agents appear as companion animals (chickens, cats, dogs) following their parent character.

```sh
# Watch additional directories for JSONL sessions
harvest-claude --watch-dir /path/to/project

# Use a custom farm layout
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
| Active | Farming animation at crop plot | Tool executing |
| Idle | Wandering the farm | No active tools |
| Waiting | Resting at cabin | Awaiting user input |
| Permission | Amber "..." bubble at mailbox | 7s without tool result |

### Sub-Agent Visualization

| Sub-Agent Type | Visual |
|----------------|--------|
| Background agent | Companion animal (chicken/cat/dog) follows parent |
| Short-term sub-agent | Nearby crop plots grow (CROP_PLOT_ON) |

### Rendering

8x8 tile and 8x16 character sprites rendered via Unicode half-block characters (`▀`) with 24-bit color. Scene composited with z-sorting at ~60 FPS. Default farm layout: 28x16 grid with crop field, cabin, pond, stone paths, and scattered trees. Layout compatible with [pixel-agents](https://marketplace.visualstudio.com/items?itemName=anthropic.pixel-agents) VS Code extension (`~/.pixel-agents/layout.json`).

## Documentation

- [`docs/idea.md`](docs/idea.md) — Concepts, terminology, and data flow contracts.
- [`docs/design.md`](docs/design.md) — Module architecture, ownership model, and key types.

## For Agents

Agent-consumable documentation index at [`docs/llms.txt`](docs/llms.txt) ([llmstxt.org](https://llmstxt.org) format).

## License

MIT
