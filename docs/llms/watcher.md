# Agent Detection

> Directory scanning, JSONL tailing, agent lifecycle management.

## Quick Start

```rust
let scanner = DirectoryScanner::new(vec![
    home.join(".claude/projects"),
])?;
let initial = scanner.initial_scan();
for path in initial {
    registry.add_agent(session_id_from(&path), path, project_name);
}
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src/watcher/scanner.rs` | Directory watching via notify, JSONL file discovery |
| `src/watcher/jsonl.rs` | JSONL file tailing, line parsing, offset tracking |
| `src/watcher/registry.rs` | Agent create/remove, poll all readers |

## Discovery Flow

1. **Startup**: Scan all `~/.claude/projects/*/` directories.
2. **Filter**: Keep `.jsonl` files with mtime < 10 minutes and size > 0 bytes.
3. **Extract**: Session ID from filename (UUID), project name from parent directory.
4. **Watch**: `notify::recommended_watcher` on `~/.claude/projects/` (recursive).
5. **Ongoing**: New `.jsonl` files → new agent. Deleted/stale files → remove agent.

## JSONL Tailing

```rust
fn read_new_lines(&mut self) -> Vec<JsonlRecord> {
    let mut file = File::open(&self.path)?;
    file.seek(SeekFrom::Start(self.offset))?;
    let mut reader = BufReader::new(file);
    let mut results = Vec::new();
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 { break; } // EOF
        self.offset += bytes as u64;
        if !line.ends_with('\n') {
            self.line_buffer.push_str(&line); // partial line
            break;
        }
        let full_line = if !self.line_buffer.is_empty() {
            let mut full = std::mem::take(&mut self.line_buffer);
            full.push_str(&line);
            full
        } else { line };
        if let Some(record) = parse_line(&full_line) {
            results.push(record);
        }
    }
    results
}
```

## Project Hash

Claude Code stores transcripts at `~/.claude/projects/<hash>/` where hash = workspace path with special characters replaced by `-`.

```rust
fn hash_path(path: &str) -> String {
    path.replace([':', '\\', '/'], "-")
}
// "/Users/yichao/Projects/foo" → "-Users-yichao-Projects-foo"
```

## Stale Agent Cleanup

Every 30s, check all tracked agents:
- JSONL file deleted from disk → remove agent.
- File mtime > 10 minutes + no active tools → mark as stale (keep but dim character).

## Gotchas

- `notify` on macOS uses FSEvents. Reliable for file creation/modification.
- JSONL lines can be split mid-write. Always buffer partial lines.
- `/clear` in Claude creates a NEW JSONL file. Old file stops growing. Scanner detects new file via `notify::EventKind::Create`.
- Multiple Claude sessions in same project dir = multiple JSONL files. Each gets its own agent.
