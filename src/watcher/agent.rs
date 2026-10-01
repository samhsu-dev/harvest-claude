use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::constants::SUMMARY_SOURCE_MAX_CHARS;
use crate::types::{AgentEvent, AgentStatus};
use crate::watcher::parser::{is_non_exempt_tool, truncate_chars};

/// A tool invocation that has started and not yet received its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolActivity {
    /// Tool invocation ID.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// One-line description of the tool input; empty when the tool has none.
    pub detail: String,
}

/// Project directory and git branch an agent works in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectIdentity {
    /// Last component of the working directory.
    pub directory: String,
    /// Git branch; empty when unknown.
    pub branch: String,
}

/// What `Agent::apply` learned that the caller needs for side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ApplyOutcome {
    /// A turn-end event closed a turn that used at least one tool.
    pub turn_had_tools: bool,
    /// A tool result finished a background agent.
    pub background_finished: bool,
}

/// A tracked Claude Code agent session.
#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique agent ID.
    pub id: usize,
    /// Session identifier from the JSONL path.
    pub session_id: String,
    /// Path to the JSONL file being tailed.
    pub jsonl_path: PathBuf,
    /// Project the agent works in; the hash directory name until the first working directory.
    pub identity: ProjectIdentity,
    /// Current inferred agent status.
    pub status: AgentStatus,
    /// Palette index of the agent's character.
    pub palette: u8,
    /// Hue shift applied when every palette is already in use.
    pub hue_shift: Option<i16>,
    /// Running tools, oldest first.
    pub tools: Vec<ToolActivity>,
    /// Whether any tool was used in the current turn.
    pub had_tools_in_turn: bool,
    /// Tool IDs of background agents that survive turn boundaries.
    pub background_tool_ids: HashSet<String>,
    /// Working directory reported by the transcript.
    pub cwd: Option<String>,
    custom_title: Option<String>,
    ai_title: Option<String>,
    last_text: String,
    last_prompt: String,
}

impl Agent {
    /// Create an idle agent whose project name is the transcript's hash directory.
    pub fn new(
        id: usize,
        session_id: String,
        jsonl_path: PathBuf,
        hash_dir: String,
        palette: u8,
        hue_shift: Option<i16>,
    ) -> Self {
        Self {
            id,
            session_id,
            jsonl_path,
            identity: ProjectIdentity {
                directory: hash_dir,
                branch: String::new(),
            },
            status: AgentStatus::Idle,
            palette,
            hue_shift,
            tools: Vec::new(),
            had_tools_in_turn: false,
            background_tool_ids: HashSet::new(),
            cwd: None,
            custom_title: None,
            ai_title: None,
            last_text: String::new(),
            last_prompt: String::new(),
        }
    }

    /// Update the agent from one transcript event. Touches no timer or animation.
    pub fn apply(&mut self, event: &AgentEvent) -> ApplyOutcome {
        let mut outcome = ApplyOutcome::default();
        match event {
            AgentEvent::ToolStart {
                tool_id,
                tool_name,
                detail,
            } => self.start_tool(tool_id, tool_name, detail),
            AgentEvent::ToolDone {
                tool_id,
                background_launch,
                ..
            } => outcome.background_finished = self.finish_tool(tool_id, *background_launch),
            AgentEvent::TurnEnd => outcome.turn_had_tools = self.end_turn(),
            AgentEvent::Text { first_line } => self.last_text.clone_from(first_line),
            AgentEvent::Title { title, custom } => {
                let slot = if *custom {
                    &mut self.custom_title
                } else {
                    &mut self.ai_title
                };
                *slot = Some(title.clone());
            }
            AgentEvent::Prompt { text } => self.last_prompt.clone_from(text),
            AgentEvent::Identity { cwd, git_branch } => self.set_identity(cwd, git_branch),
        }
        outcome
    }

    fn start_tool(&mut self, id: &str, name: &str, detail: &str) {
        if !self.tools.iter().any(|t| t.id == id) {
            self.tools.push(ToolActivity {
                id: id.to_owned(),
                name: name.to_owned(),
                detail: detail.to_owned(),
            });
        }
        self.had_tools_in_turn = true;
        self.status = AgentStatus::Active;
    }

    // Returns true when the result finished a background agent.
    fn finish_tool(&mut self, id: &str, background_launch: bool) -> bool {
        if !self.tools.iter().any(|t| t.id == id) {
            return false;
        }
        if background_launch {
            self.background_tool_ids.insert(id.to_owned());
            return false;
        }
        let was_background = self.background_tool_ids.remove(id);
        self.tools.retain(|t| t.id != id);
        was_background
    }

    // Returns whether the closed turn used a tool.
    fn end_turn(&mut self) -> bool {
        let had_tools = self.had_tools_in_turn;
        let background = &self.background_tool_ids;
        self.tools.retain(|t| background.contains(&t.id));
        self.had_tools_in_turn = false;
        self.status = AgentStatus::Waiting;
        had_tools
    }

    fn set_identity(&mut self, cwd: &str, git_branch: &str) {
        let directory = Path::new(cwd)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(cwd);
        self.identity = ProjectIdentity {
            directory: directory.to_owned(),
            branch: git_branch.to_owned(),
        };
        self.cwd = Some(cwd.to_owned());
    }

    /// One-line description of what the agent is doing.
    ///
    /// Sources in order: newest running tool, latest assistant text, session title,
    /// last prompt. Empty when none is known.
    pub fn summary(&self) -> String {
        let running_tool = self.tools.last().map(|tool| {
            if tool.detail.is_empty() {
                tool.name.clone()
            } else {
                format!("{}: {}", tool.name, tool.detail)
            }
        });
        let title = self.custom_title.as_ref().or(self.ai_title.as_ref());
        [
            running_tool.as_deref(),
            Some(self.last_text.as_str()),
            title.map(String::as_str),
            Some(self.last_prompt.as_str()),
        ]
        .into_iter()
        .flatten()
        .map(collapse_line)
        .find(|source| !source.is_empty())
        .unwrap_or_default()
    }

    /// Project label: `<directory> (<branch>)`, or the directory alone without a branch.
    pub fn display_name(&self) -> String {
        let ProjectIdentity { directory, branch } = &self.identity;
        if branch.is_empty() {
            directory.clone()
        } else {
            format!("{directory} ({branch})")
        }
    }

    /// Whether a running tool needs approval, so the permission timer applies.
    pub fn is_running_non_exempt(&self) -> bool {
        self.tools.iter().any(|t| is_non_exempt_tool(&t.name))
    }
}

fn collapse_line(text: &str) -> String {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars(&joined, SUMMARY_SOURCE_MAX_CHARS)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn agent() -> Agent {
        Agent::new(
            1,
            "s1".into(),
            PathBuf::from("/t/s1.jsonl"),
            "-hash".into(),
            0,
            None,
        )
    }

    fn tool_start(id: &str, name: &str, detail: &str) -> AgentEvent {
        AgentEvent::ToolStart {
            tool_id: id.into(),
            tool_name: name.into(),
            detail: detail.into(),
        }
    }

    fn tool_done(id: &str, background_launch: bool) -> AgentEvent {
        AgentEvent::ToolDone {
            tool_id: id.into(),
            is_error: false,
            background_launch,
        }
    }

    fn text(line: &str) -> AgentEvent {
        AgentEvent::Text {
            first_line: line.into(),
        }
    }

    fn title(title: &str, custom: bool) -> AgentEvent {
        AgentEvent::Title {
            title: title.into(),
            custom,
        }
    }

    fn prompt(text: &str) -> AgentEvent {
        AgentEvent::Prompt { text: text.into() }
    }

    fn applied(events: &[AgentEvent]) -> Agent {
        let mut agent = agent();
        for event in events {
            agent.apply(event);
        }
        agent
    }

    #[test]
    fn new_agent_is_idle_with_the_hash_directory_as_project() {
        let a = agent();
        assert_eq!(a.status, AgentStatus::Idle);
        assert_eq!(a.display_name(), "-hash");
        assert!(a.summary().is_empty());
    }

    #[test]
    fn tool_start_activates_and_tool_done_finishes_the_tool() {
        let mut a = agent();
        a.apply(&tool_start("t1", "Bash", "ls"));
        assert_eq!(a.status, AgentStatus::Active);
        assert!(a.had_tools_in_turn);
        assert_eq!(a.tools.len(), 1);

        a.apply(&tool_done("t1", false));
        assert!(a.tools.is_empty());
    }

    #[test]
    fn duplicate_tool_start_is_one_activity() {
        let a = applied(&[tool_start("t1", "Bash", ""), tool_start("t1", "Bash", "")]);
        assert_eq!(a.tools.len(), 1);
    }

    #[test]
    fn tool_done_for_an_unknown_id_is_ignored() {
        let mut a = applied(&[tool_start("t1", "Bash", "")]);
        let outcome = a.apply(&tool_done("other", false));
        assert_eq!(outcome, ApplyOutcome::default());
        assert_eq!(a.tools.len(), 1);
    }

    #[test]
    fn turn_end_waits_and_reports_whether_tools_were_used() {
        let mut a = applied(&[tool_start("t1", "Read", "/a")]);
        let outcome = a.apply(&AgentEvent::TurnEnd);
        assert!(outcome.turn_had_tools);
        assert_eq!(a.status, AgentStatus::Waiting);
        assert!(a.tools.is_empty());
        assert!(!a.had_tools_in_turn);

        let outcome = a.apply(&AgentEvent::TurnEnd);
        assert!(!outcome.turn_had_tools);
    }

    #[test]
    fn background_launch_keeps_the_tool_until_its_final_result() {
        let mut a = applied(&[tool_start("bg", "Agent", "Search")]);

        let launch = a.apply(&tool_done("bg", true));
        assert!(!launch.background_finished);
        assert!(a.background_tool_ids.contains("bg"));
        assert_eq!(a.tools.len(), 1);

        a.apply(&AgentEvent::TurnEnd);
        assert_eq!(a.tools.len(), 1, "turn end keeps background tools");

        let finish = a.apply(&tool_done("bg", false));
        assert!(finish.background_finished);
        assert!(a.tools.is_empty());
        assert!(a.background_tool_ids.is_empty());
    }

    #[rstest]
    #[case(&[], "")]
    #[case(&[prompt("Add tests")], "Add tests")]
    #[case(&[prompt("Add tests"), title("Fix login", false)], "Fix login")]
    #[case(&[title("Generated", false), title("Chosen", true)], "Chosen")]
    #[case(&[title("Fix login", false), text("Reading the code")], "Reading the code")]
    #[case(&[text("Reading the code"), tool_start("t1", "Bash", "Run build")], "Bash: Run build")]
    #[case(&[tool_start("t1", "Bash", "first"), tool_start("t2", "Read", "/a.rs")], "Read: /a.rs")]
    #[case(&[tool_start("t1", "WebFetch", "")], "WebFetch")]
    #[case(&[text("Reading"), tool_start("t1", "Bash", "x"), tool_done("t1", false)], "Reading")]
    fn summary_follows_precedence(#[case] events: &[AgentEvent], #[case] expected: &str) {
        assert_eq!(applied(events).summary(), expected);
    }

    #[test]
    fn summary_is_one_capped_line() {
        let long = "x ".repeat(SUMMARY_SOURCE_MAX_CHARS);
        let a = applied(&[text(&long)]);
        assert!(!a.summary().contains('\n'));
        assert!(a.summary().chars().count() <= SUMMARY_SOURCE_MAX_CHARS);
    }

    #[rstest]
    #[case("/Users/me/work/app", "main", "app", "app (main)")]
    #[case("/Users/me/work/app", "", "app", "app")]
    #[case("/", "dev", "/", "/ (dev)")]
    fn identity_sets_directory_branch_and_cwd(
        #[case] cwd: &str,
        #[case] branch: &str,
        #[case] directory: &str,
        #[case] display: &str,
    ) {
        let a = applied(&[AgentEvent::Identity {
            cwd: cwd.into(),
            git_branch: branch.into(),
        }]);
        assert_eq!(a.identity.directory, directory);
        assert_eq!(a.display_name(), display);
        assert_eq!(a.cwd.as_deref(), Some(cwd));
    }

    #[test]
    fn running_non_exempt_ignores_exempt_tools() {
        let a = applied(&[
            tool_start("t1", "Agent", "x"),
            tool_start("t2", "AskUserQuestion", ""),
        ]);
        assert!(!a.is_running_non_exempt());
        let b = applied(&[tool_start("t1", "Agent", "x"), tool_start("t2", "Bash", "")]);
        assert!(b.is_running_non_exempt());
    }
}
