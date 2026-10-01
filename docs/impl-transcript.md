# Claude Code Transcript — Implementation

Record shapes verified against local transcripts. Consumed by `modules/parser-design.md`.

**[transcript]** Path `~/.claude/projects/<hash>/<session-uuid>.jsonl`; sub-agent transcripts live in `<hash>/<session-uuid>/subagents/` and are never tracked.

**[transcript]** Every conversation record carries top-level `cwd` and `gitBranch`. Project label is the last component of `cwd`.

**[transcript]** Turn end is a `system` record with top-level `subtype: "turn_duration"`. `message.subtype` does not exist.

**[transcript]** Metadata records: `{"type":"ai-title","aiTitle":"..."}`, `{"type":"custom-title","customTitle":"..."}`, `{"type":"last-prompt","lastPrompt":"..."}`. In `last-prompt`, `lastPrompt` is absent in about 1% of records; treat it as optional.

**[transcript]** Background launch: a `tool_result` whose text contains `Async agent launched successfully.` The tool is finished only by a later result for the same `tool_use_id`.
