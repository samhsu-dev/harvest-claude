# Harvest Claude — Concepts & Terminology

## 1. Context

**Problem Statement** — Claude Code agents run as invisible terminal processes. A developer with several sessions open cannot see at a glance which agent is working, which one is blocked on a permission prompt, and what each one is doing right now. Existing visualizers need a VS Code extension or a browser.

**System Role** — Harvest Claude is a standalone Rust terminal application that renders every active Claude Code session as a farm worker in a pixel-art farm, drawn with half-block Unicode characters in 24-bit color, and shows a one-line activity summary per agent.

**Data Flow**
- **Inputs:** JSONL session transcripts written by Claude Code under `~/.claude/projects/`, file system events on those files, keyboard and mouse input, a layout file and a warehouse file under `~/.harvest-claude/`.
- **Outputs:** terminal pixel rendering, status bar text, persisted warehouse counts, a size-bounded diagnostic log under `~/.harvest-claude/`.
- **Connections:** Claude Code CLI → Session Transcript → Scanner / Reader → Parser → Agent Events → App → Renderer → Terminal.

**Scope Boundaries**
- **Owned:** session discovery, transcript parsing, agent status inference, activity summary, farm rendering, character animation, produce and warehouse, status bar, terminal lifecycle safety, layout loading.
- **Not Owned:** Claude Code CLI behaviour, the transcript format, terminal emulator capabilities, launching or controlling agents (view only), layout editing (no editor in this application).

## 2. Concepts

**Conceptual Diagram**
```
Claude Code CLI ──writes──▶ ~/.claude/projects/<hash>/<session>.jsonl
                                          │
                        Scanner (discovery, dormancy, removal)
                                          │
                        Reader (tail new lines, buffer partial line)
                                          │
                        Parser ──▶ Agent Events, Activity Summary, Project Identity
                                          │
Keyboard / Mouse ──▶ Action ──▶ App (agents, timers, farm) ──▶ Renderer ──▶ Terminal
                                          │
                                Warehouse (persisted produce)
```

**Core Concepts**

- **Session Transcript**
  - **Definition:** The append-only JSONL file Claude Code writes for one session. It is the only input that tells the application what an agent is doing.
  - **Scope:** Lives directly under the project hash directory as `<session-uuid>.jsonl`. Transcripts of spawned sub-agents live in a `subagents/` directory beside it and are not tracked as agents. A `/clear` starts a new transcript file.
  - **Relationships:** One Session Transcript ↔ one Agent. Read by the Reader, interpreted by the Parser.

- **Agent**
  - **Definition:** A detected Claude Code session, identified by its transcript file. The unit the farm visualizes.
  - **Scope:** Includes session id, transcript path, Project Identity, Agent Status, active Tool Activities, Activity Summary, background sub-agent markers. Excludes process control.
  - **Relationships:** Agent → Character (visual). Agent → Reader (data source). Agent → Project Identity (where it works).

- **Agent Status**
  - **Definition:** The inferred state of an agent: Active (a tool-use was seen since the last turn end), Idle (adopted or woken, no tool-use seen yet), Waiting (turn ended or text-only reply settled, awaiting the user), Permission (a tool awaits user approval), Dormant (no transcript writes for a long time).
  - **Scope:** Derived from Agent Events and Heuristic Timers. Never read from a definitive signal; Claude Code does not record permission prompts in the transcript.
  - **Relationships:** Drives Character animation, Speech Bubble, status bar text, and Dormancy. Transition rules: `model.md`.

- **Tool Activity**
  - **Definition:** One tool invocation from its `tool_use` block to its matching `tool_result` block.
  - **Scope:** Includes tool id, tool name, and the tool's input detail (command, file path, prompt). Includes an error flag from the result. Excludes tool output content.
  - **Relationships:** Starts and ends Active status, feeds Activity Summary, selects the Work Animation category (typing versus reading), and starts the permission Heuristic Timer.

- **Turn**
  - **Definition:** One assistant response cycle, from a user prompt to the system record that reports the turn duration.
  - **Scope:** The turn-end record is the only definitive "agent stopped" signal. Text-only turns emit no such record and rely on the text-idle timer. Thinking-only assistant lines are neither text nor tools and never start a timer.
  - **Relationships:** Turn end clears every foreground Tool Activity, cancels timers, and moves the Agent to Waiting.

- **Activity Summary**
  - **Definition:** A one-line, human-readable statement of what an agent is doing right now, shown in the status bar and above the selected character.
  - **Scope:** Sources, in precedence order: the running tool's detail, the first line of the latest assistant text, the session title Claude Code assigned, the last user prompt. Truncated by display width, never by byte. Precedence semantics: `model.md`.
  - **Relationships:** Derived from Tool Activity and transcript metadata records. Owned by the Agent, rendered by the status bar.

- **Project Identity**
  - **Definition:** The human-readable origin of a session: the working directory name and the git branch recorded in every transcript record.
  - **Scope:** Falls back to the project hash directory name until the first record arrives. Replaces the raw hash in all user-facing text.
  - **Relationships:** Owned by the Agent. Shown in the status bar and used by terminal-window focus.

- **Heuristic Timer**
  - **Definition:** A per-agent timer that infers a status Claude Code never writes down. Two kinds: permission (a non-exempt tool has run for 7 s without a result) and text-idle (a text-only reply has had no follow-up for 5 s).
  - **Scope:** One permission timer per agent, restarted on each new tool start, cancelled by a tool result or turn end. Exempt tools: sub-agent launches and user questions. No progress records exist in current transcripts, so a long-running tool is indistinguishable from a permission prompt.
  - **Relationships:** Expiry sets Agent Status to Permission or Waiting and raises a Speech Bubble.

- **Character**
  - **Definition:** The animated 8×16-pixel farm worker that represents one Agent on the farm.
  - **Scope:** Includes palette, seat assignment, facing, walk and work animation, wander behaviour, Speech Bubble, Companions, delivery walk, dormancy walk. Excludes agent data.
  - **Relationships:** Owned by the Farm. Follows its Agent's status. Carries Produce to the barn.

- **Farm**
  - **Definition:** The tile grid scene: grass, dirt, water, sand, stone, fences, and placed furniture such as crop plots, fruit trees, fishing spots, a cabin (home), a barn, and a mailbox.
  - **Scope:** Owns tiles, furniture, Seats, walkable set, Characters, z-sorted drawing order. Default grid 28×16 tiles.
  - **Relationships:** Built from a Layout. Hosts Characters. Furniture facing a working Character switches to its "on" variant.

- **Seat and Work Animation**
  - **Definition:** A furniture-derived standing or sitting spot where a Character works, and the animation that spot implies: farming at a crop plot, harvesting at a fruit tree, fishing at a fishing spot.
  - **Scope:** One Character per Seat. Seats without a facing work object play a generic typing or reading animation.
  - **Relationships:** Seat → Work Animation → Produce type on turn end.

- **Produce and Warehouse**
  - **Definition:** A reward token (wheat, fruit, fish) a Character earns when a turn ends at a work Seat, carried to the barn and counted in the persisted Warehouse.
  - **Scope:** Counts only. Persisted atomically under `~/.harvest-claude/`. A corrupt file is reported, never silently reset.
  - **Relationships:** Produced by Turn end at a Seat. Displayed in the status bar and as piles beside the barn.

- **Companion**
  - **Definition:** A small animal (chicken, cat, dog) that follows a Character while that agent has a background sub-agent running.
  - **Scope:** Created when a sub-agent launch result carries the asynchronous-launch marker. Removed when the parent's turn ends or the sub-agent tool completes. Not an Agent and has no transcript of its own.
  - **Relationships:** Owned by the parent Character. Keyed by the launching tool id.

- **Speech Bubble**
  - **Definition:** A small overlay above a Character: amber "..." for Permission, green check for Waiting.
  - **Scope:** Permission bubble persists until the tool completes, the turn ends, or the user dismisses it. Waiting bubble fades after 2 s.
  - **Relationships:** Set from Agent Status. Cleared by Tool Activity completion or Turn end, not only by the user.

- **Dormancy**
  - **Definition:** The lifecycle stage for a transcript that has stopped growing: after 10 min the Character walks home and disappears; after 2 h the Agent is removed. New writes wake it.
  - **Scope:** Dormant agents do not count toward the visible agent limit, are not selectable, and have no summary.
  - **Relationships:** Driven by the Scanner. Changes Agent Status and Character visibility.

- **Palette**
  - **Definition:** A Character's color identity. Six base palettes; later agents reuse the least-used palette with a hue shift.
  - **Scope:** Assigned on agent creation, released on removal.
  - **Relationships:** Owned by the Agent registry, applied at sprite colorization.

- **Layout**
  - **Definition:** The serialized farm description: grid size, tile types, furniture positions, optional tile colors, layout revision. Same file format as the pixel-agents VS Code extension.
  - **Scope:** Read from `~/.harvest-claude/layout.json` or the `--layout` path; the bundled default is used when none exists. Validated on load; an invalid file is reported, never replaced.
  - **Relationships:** Layout → Farm. A revision number marks the bundled default only; user layouts are kept as written.

- **Half-Block Pixel**
  - **Definition:** The rendering primitive: one terminal cell shows two vertical pixels using `▀` with foreground (top) and background (bottom) colors.
  - **Scope:** Requires a 24-bit color terminal. A 28×16 grid needs 224 columns × 64 rows plus the status bar.
  - **Relationships:** Pixel buffer → half-block cells → terminal.

- **Terminal Guard**
  - **Definition:** The ownership of raw mode, alternate screen, and mouse capture for the process lifetime, including the panic path.
  - **Scope:** Restores the terminal on normal exit, on error, and on panic. Reports a too-small terminal with a text message instead of a partial scene.
  - **Relationships:** Wraps the App run loop.

## 3. Contracts & Flow

**Data Contracts**

- **With Claude Code CLI:** records are one JSON object per line with a top-level `type`. Consumed kinds: `assistant` (one content block per line: `thinking`, `text`, or `tool_use` with `id`, `name`, `input`), `user` (prompt text, or `tool_result` with `tool_use_id`, `is_error`, content), `system` with top-level `subtype` `turn_duration`, `ai-title` / `custom-title` (session title), `last-prompt` (latest user prompt). Every record carries `cwd`, `gitBranch`, `sessionId`, `version`. No `progress` records are written. A background sub-agent launch is marked by the text "Async agent launched successfully." inside the launching tool's result. Unknown kinds are ignored.
- **With File System:** `~/.claude/projects/` is always watched; `--watch-dir` adds directories. `~/.harvest-claude/` holds `layout.json`, `warehouse.json`, and the log file. All writes are atomic (temp file + rename).
- **With Terminal:** raw mode, alternate screen, mouse capture, 24-bit color. Keys: `q` or Ctrl+C quit, `←`/`→` cycle selection, Esc dismisses a bubble then deselects, click selects, click on the selected character focuses its terminal window.

**Internal Processing Flow**

1. **Startup** — parse CLI → load and validate Layout → build Farm → Scanner initial scan of recent transcripts → one Agent and Character per transcript, each with the status its last records imply.
2. **Discovery** — the Scanner notices a new or growing transcript → new Agent → Palette assigned → Character spawns. A transcript that stops growing moves through Dormancy to removal.
3. **Tailing** — the Reader appends new complete lines, buffering a partial last line across polls, and decoding on UTF-8 boundaries only.
4. **Parsing** — each record yields Agent Events (tool start with detail, tool done with error flag, turn end, text reply, title, prompt, identity) → App updates the Agent and its timers.
5. **Inference** — Heuristic Timers expire → Permission or Waiting status → Speech Bubble. A tool result or turn end clears the bubble and cancels the timer.
6. **Reward** — turn end at a work Seat → Produce → delivery walk → Warehouse count → atomic save.
7. **Rendering** — on each frame the Farm composes tiles, furniture, Characters, Companions, Bubbles into the pixel buffer → half-block cells → status bar with Project Identity, Agent Status, and Activity Summary for the selected agent.

## 4. Scenarios

- **Typical:** The developer runs `harvest-claude` with three sessions open. Three workers spawn, walk to crop plots, and farm. The status bar shows each project name and branch. The selected worker's line reads "Bash: cargo test --workspace". One turn ends: that worker shows a check bubble, carries wheat to the barn, then wanders.

- **Boundary:** A tool runs for 7 s with no result. The worker shows an amber "..." bubble. The result arrives: the bubble clears without user action and the worker keeps farming. A text-only reply with thinking blocks does not start the idle timer until its text block is written.

- **Boundary:** The terminal is narrower than the grid needs. The application shows a one-line size requirement instead of a clipped scene and resumes when the window grows. A panic restores the terminal before the error is printed.

- **Interaction:** The developer clicks a worker. The status bar expands to that agent's project, branch, status, running tool, and summary line. Clicking again raises the agent's terminal window to the front.

- **Sub-agent:** The agent launches a background sub-agent. A chicken appears beside the worker and follows it. The sub-agent finishes and the chicken leaves. The sub-agent's own transcript is never shown as a separate worker.

- **Dormancy:** A session is left open overnight. After 10 min its worker walks into the cabin. In the morning the developer types a prompt: the worker walks out and resumes.
