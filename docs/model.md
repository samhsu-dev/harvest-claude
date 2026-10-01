# Harvest Claude — Domain Model

Formalizes the entities, relations, state rules, and invariants behind the concepts in `idea.md`. Software structure: `design.md`.

## Entities

- **Session Transcript** — one append-only JSONL file under a project hash directory. Exists while the file exists on disk. Its path is `<projects root>/<hash>/<session-uuid>.jsonl`; files under a `subagents/` directory are sub-agent transcripts and are never a Session Transcript.
- **Record** — one line of a Session Transcript, one JSON object with a top-level kind. Consumed kinds: assistant content block, user prompt, tool result, turn end, session title, last prompt. Any other kind is ignored.
- **Agent** — the tracked session behind one Session Transcript. Exists from discovery until removal. Carries exactly one Agent Status, zero or more Tool Activities, one Project Identity, one Activity Summary, and a set of background sub-agent markers.
- **Agent Status** — one of Active, Idle, Waiting, Permission, Dormant. Always defined for an Agent.
- **Tool Activity** — one tool invocation identified by its tool id. Exists from its tool-use Record until its tool-result Record or the turn end that follows it. Carries a tool name, a detail string derived from the tool input, and, once finished, an error flag.
- **Turn** — the span from a user prompt Record to the turn-end Record. A text-only turn has no turn-end Record and ends by the text-idle Heuristic Timer.
- **Heuristic Timer** — a deadline attached to one Agent. Two kinds: permission and text-idle. An Agent holds at most one timer of each kind.
- **Activity Summary** — the one-line statement of what an Agent is doing. Exists for every non-dormant Agent; empty until the first contributing Record arrives.
- **Project Identity** — the working directory name and git branch of the Agent's session. Exists from discovery; the hash directory name is its value until the first Record carrying a working directory arrives.
- **Character** — the visible farm worker bound to one Agent. Exists while the Agent exists. Hidden while the Agent is Dormant.
- **Seat** — a work position derived from a seat-type furniture item. Occupied by at most one Character.
- **Work Animation** — the animation category a Seat implies: farming, harvesting, fishing, or none.
- **Produce** — one unit of wheat, fruit, or fish. Created when a Turn ends while the Agent's Character has a Seat and a path to the barn exists.
- **Warehouse** — the persisted count of Produce per kind. Exists for the application lifetime; loaded at startup, saved after each delivery.
- **Companion** — an animal bound to one Character and one background sub-agent tool id. Exists while that tool id is marked as background and the Agent is not Dormant.
- **Speech Bubble** — an overlay bound to one Character. At most one at a time. Kinds: Permission and Waiting.
- **Palette** — one of six color identities with an optional hue shift. Held by one Agent from creation to removal.
- **Layout** — the serialized farm description. Exists as the bundled default or a user file.

## Relations

| Relation | Direction | Cardinality | Meaning |
|----------|-----------|-------------|---------|
| tracks | Agent → Session Transcript | 1 : 1 | the Agent reads this file and no other |
| yields | Record → Agent Event | 1 : 0..n | one line produces zero or more events |
| owns | Agent → Tool Activity | 1 : 0..n | tools started in the current Turn and not yet finished |
| marks | Agent → background tool id | 1 : 0..n | sub-agent launches that outlive the Turn |
| holds | Agent → Heuristic Timer | 1 : 0..2 | at most one permission and one text-idle timer |
| summarizes | Agent → Activity Summary | 1 : 1 | derived, never stored independently |
| works in | Agent → Project Identity | 1 : 1 | directory name and branch shown for the Agent |
| embodies | Character → Agent | 1 : 1 | the Character shows the Agent's status |
| occupies | Character → Seat | 0..1 : 0..1 | a Seat has at most one occupant |
| follows | Companion → Character | n : 1 | companions move with their Character |
| shows | Character → Speech Bubble | 1 : 0..1 | one bubble at a time |
| carries | Character → Produce | 1 : 0..1 | one pending delivery at a time |
| counts | Warehouse → Produce kind | 1 : 3 | one count per kind |
| colors | Agent → Palette | 1 : 1 | released when the Agent is removed |
| builds | Layout → Farm | 1 : 1 | one layout defines the whole scene |

## State Model

Agent Status transitions. "Tool result" means the tool-result Record for a Tool Activity the Agent owns.

| Source | Trigger | Guard | Target |
|--------|---------|-------|--------|
| any non-Dormant | tool-use Record | — | Active |
| Active | tool result | no Tool Activity remains | Idle |
| Active | tool result | another Tool Activity remains | Active |
| Active | permission timer expiry | a non-exempt Tool Activity remains | Permission |
| Permission | tool result | — | Active, or Idle when none remains |
| any non-Dormant | turn-end Record | — | Waiting |
| Waiting | Waiting bubble fades | — | Idle |
| Idle | text Record | no Tool Activity in this Turn | Idle, text-idle timer started |
| Idle | text-idle timer expiry | — | Idle, Waiting bubble shown |
| any non-Dormant | no transcript write for 10 min | — | Dormant |
| Dormant | transcript write | — | Idle, then by the Record as above |
| Dormant | no transcript write for 2 h | — | removed |

Timer rules:
- A tool-use Record for a non-exempt tool starts or restarts the permission timer. Exempt tools: sub-agent launch and user question.
- A tool result or a turn-end Record cancels the permission timer and removes a Permission bubble.
- A text Record starts the text-idle timer only when the Turn has no Tool Activity. A thinking-only Record starts nothing.
- Any new Record cancels the text-idle timer before it is interpreted.

Tool Activity lifecycle:
- Created by a tool-use Record. Finished by a tool result with the same tool id.
- A turn-end Record finishes every Tool Activity except those whose tool id is a background marker.
- A tool result whose content carries the asynchronous-launch marker adds the tool id to the background markers and does not finish the Tool Activity.
- A background marker is removed by the sub-agent tool's final result or by the parent Agent's removal.

Character visibility:
- Visible while the Agent is not Dormant.
- A Dormant transition starts a walk to the home furniture; the Character hides on arrival or at once when no path exists.
- A wake transition shows the Character at its Seat or a walkable tile.

## Invariants

- Every Agent tracks exactly one Session Transcript, and every tracked path belongs to exactly one Agent.
- A path containing a `subagents` directory component is never tracked.
- Every Agent has exactly one Character.
- An Agent holds at most one permission timer and at most one text-idle timer.
- A Permission bubble exists only while the Agent Status is Permission.
- A Waiting bubble exists only while the Agent Status is Waiting.
- A Companion exists only for a tool id present in its Agent's background markers.
- A Dormant Agent has no Tool Activity, no Heuristic Timer, no Speech Bubble, and no Companion.
- Dormant Agents do not count toward the visible agent limit and are never the selected agent.
- The number of non-Dormant Agents never exceeds the visible agent limit.
- Every Seat has at most one occupant, and a Character occupies at most one Seat.
- A Palette is held by an Agent from creation to removal and released on removal; usage counts equal the number of live Agents holding that palette.
- Every Agent has a Project Identity; it is never the raw hash once a Record with a working directory has been read.
- Activity Summary is truncated by display width, never inside a multi-byte character.
- Warehouse counts never decrease during a run and are written atomically.
- A Layout is applied only when its tile count equals columns × rows, dimensions are within bounds, and every furniture position lies inside the grid.
- The bundled default Layout is replaced on revision change only when the loaded file is itself the bundled default; a user-edited Layout is never replaced.

## Cross-Structure Contracts

Record → Agent Event mapping:

| Record | Event | Carried data |
|--------|-------|--------------|
| assistant `tool_use` block | Tool Start | tool id, tool name, detail from input |
| assistant `text` block | Text | first line of the text |
| assistant `thinking` block | none | — |
| user `tool_result` block | Tool Done | tool id, error flag, background marker flag |
| user prompt | Prompt | prompt text |
| system `turn_duration` | Turn End | — |
| `ai-title` / `custom-title` | Title | title text |
| `last-prompt` | Prompt | prompt text |
| any Record with `cwd` | Identity | working directory, git branch |

Tool detail by tool name: Bash → description, else command; Agent → description, else prompt; Read, Edit, Write, Glob, Grep → file path or pattern; others → tool name only. Detail is one line; line breaks collapse to spaces.

Activity Summary precedence, first non-empty wins:
1. Running Tool Activity: `<tool name>: <detail>`, newest tool first.
2. First line of the latest assistant text.
3. Session title (custom title over generated title).
4. Last user prompt.
5. Empty.

Project Identity derivation: directory name is the last path component of the working directory; branch is the recorded git branch or empty. Display form: `<directory> (<branch>)`, or `<directory>` when the branch is empty. Fallback before any Record: the hash directory name.

Agent Status → Character and bubble:

| Status | Character behaviour | Bubble |
|--------|--------------------|--------|
| Active | walk to Seat, play Work Animation or tool animation | none |
| Idle | wander, rest at Seat | none |
| Waiting | idle, deliver Produce if any | Waiting, 2 s |
| Permission | stay at Seat, work animation paused | Permission |
| Dormant | hidden | none |

Turn end → Produce: Work Animation farming → wheat, harvesting → fruit, fishing → fish; a Seat without a Work Animation counts as farming. The Character carries the Produce to the barn; arrival adds one to the Warehouse count. No path to the barn: no Produce is counted.
