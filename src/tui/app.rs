//! Dashboard state and key handling. Loading functions call the same core
//! used by the CLI (`discovery`, `codex_discovery`, `claude`, `codex`,
//! `selection`, `resume`, `receive`) and never write or execute anything.
use crate::tui::theme::{Mode, Palette};
use crossterm::event::KeyCode;
use memory_bee::{
    claude::{self, Limits, ReadState},
    codex, codex_discovery,
    discovery::{self, DiscoveryLimits},
    receive, resume, selection,
};
use ratatui::widgets::ListState;
use std::path::{Path, PathBuf};

pub struct SessionRow {
    pub agent: &'static str,
    pub path: PathBuf,
    pub label: String,
    pub state: ReadState,
    pub event_count: usize,
    pub is_subagent: bool,
    pub requires_branch_selection: bool,
    pub branch_tips: Vec<String>,
    pub diagnostics: usize,
}

pub fn state_glyph(state: &ReadState) -> (&'static str, &'static str) {
    match state {
        ReadState::Read => ("●", "ok"),
        ReadState::Empty => ("◌", "vazio"),
        ReadState::Partial => ("◐", "parcial"),
    }
}

/// Discover Claude and/or Codex sessions for `project` under the explicit
/// roots given. Returns the unified rows, whether any discovery was partial,
/// and one note per root that was partial.
pub fn load_sessions(
    project: &Path,
    claude_root: Option<&Path>,
    codex_root: Option<&Path>,
) -> Result<(Vec<SessionRow>, bool), String> {
    let mut rows = Vec::new();
    let mut partial = false;
    if let Some(root) = claude_root {
        let found = discovery::discover(root, project, DiscoveryLimits::default())
            .map_err(|e| format!("Cannot discover sessions: {e}"))?;
        partial |= found.partial;
        for s in found.sessions {
            rows.push(SessionRow {
                agent: "claude-code",
                label: s
                    .session_ids
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| "(sem id)".into()),
                state: s.state,
                event_count: s.event_count,
                is_subagent: s.is_subagent,
                requires_branch_selection: s.requires_branch_selection,
                branch_tips: s.branch_tips,
                diagnostics: s.diagnostics.len(),
                path: s.path,
            });
        }
    }
    if let Some(root) = codex_root {
        let found = codex_discovery::discover(root, project, DiscoveryLimits::default())
            .map_err(|e| format!("Cannot discover Codex sessions: {e}"))?;
        partial |= found.partial;
        for s in found.sessions {
            rows.push(SessionRow {
                agent: "codex",
                label: s.session_id,
                state: s.state,
                event_count: s.event_count,
                is_subagent: false,
                requires_branch_selection: false,
                branch_tips: Vec::new(),
                diagnostics: s.diagnostics.len(),
                path: s.path,
            });
        }
    }
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    Ok((rows, partial))
}

pub enum Detail {
    Claude(claude::Report),
    Codex(codex::Report),
}

impl Detail {
    /// Mirrors the HANDOFF convention (`bundle.rs`): the first retained
    /// `user`/`text` event.
    pub fn first_human_request(&self) -> Option<&str> {
        match self {
            Detail::Claude(r) => r
                .events
                .iter()
                .find(|e| e.role == "user" && e.kind == "text")
                .map(|e| e.text.as_str()),
            Detail::Codex(r) => r
                .events
                .iter()
                .find(|e| e.role == "user" && e.kind == "text")
                .map(|e| e.text.as_str()),
        }
    }
}

/// Read a session's full report. `leaf` selects a branch tip for a Claude
/// session that requires one; ignored for Codex, which has no branches.
pub fn load_detail(row: &SessionRow, leaf: Option<&str>) -> Result<Detail, String> {
    if row.agent == "codex" {
        let report =
            codex::inspect(&row.path, Limits::default()).map_err(|e| e.to_string())?;
        Ok(Detail::Codex(report))
    } else {
        let mut report =
            claude::inspect(&row.path, Limits::default()).map_err(|e| e.to_string())?;
        if let Some(leaf) = leaf {
            report = selection::select(report, leaf).map_err(|e| e.to_string())?;
        }
        Ok(Detail::Claude(report))
    }
}

/// Preview `prepare-resume` for a bundle: read-only, no prompt file written.
pub fn load_resume(
    bundle: &Path,
    target: resume::Target,
    project: &Path,
) -> Result<resume::Preparation, String> {
    let request = resume::Request {
        bundle: bundle.to_path_buf(),
        target,
        project: project.to_path_buf(),
        worktree: None,
    };
    resume::prepare(&request, None)
}

/// Verify a bundle's integrity: hashes and layout only, no writes.
pub fn load_verify(bundle: &Path) -> Result<receive::Report, String> {
    let verified = receive::verify(bundle)?;
    Ok(verified.report())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Sessions,
    Resume,
    Help,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Detail,
}

pub struct App {
    pub project: PathBuf,
    pub bundle: Option<PathBuf>,
    pub palette: Palette,
    pub sessions: Vec<SessionRow>,
    pub partial: bool,
    pub list_state: ListState,
    pub view: View,
    pub focus: Focus,
    pub detail: Option<Result<Detail, String>>,
    pub branch_pick: usize,
    pub resume_target: resume::Target,
    pub resume: Option<Result<resume::Preparation, String>>,
    pub verify: Option<Result<receive::Report, String>>,
    pub quit: bool,
}

impl App {
    pub fn new(
        project: PathBuf,
        claude_root: Option<PathBuf>,
        codex_root: Option<PathBuf>,
        bundle: Option<PathBuf>,
        mode: Mode,
        no_color: bool,
    ) -> Result<Self, String> {
        let (sessions, partial) =
            load_sessions(&project, claude_root.as_deref(), codex_root.as_deref())?;
        let mut list_state = ListState::default();
        if !sessions.is_empty() {
            list_state.select(Some(0));
        }
        Ok(App {
            project,
            bundle,
            palette: Palette::new(mode, no_color),
            sessions,
            partial,
            list_state,
            view: View::Sessions,
            focus: Focus::List,
            detail: None,
            branch_pick: 0,
            resume_target: resume::Target::Claude,
            resume: None,
            verify: None,
            quit: false,
        })
    }

    pub fn selected(&self) -> Option<&SessionRow> {
        self.list_state
            .selected()
            .and_then(|i| self.sessions.get(i))
    }

    fn move_selection(&mut self, delta: i32) {
        if self.sessions.is_empty() {
            return;
        }
        let len = self.sessions.len() as i32;
        let current = self.list_state.selected().unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(len) as usize;
        self.list_state.select(Some(next));
        self.detail = None;
        self.branch_pick = 0;
    }

    fn open_detail(&mut self) {
        let Some(row) = self.selected() else {
            return;
        };
        let leaf = row
            .requires_branch_selection
            .then(|| row.branch_tips.get(self.branch_pick).cloned())
            .flatten();
        self.detail = Some(load_detail(row, leaf.as_deref()));
        self.focus = Focus::Detail;
    }

    fn run_resume(&mut self) {
        if let Some(bundle) = self.bundle.clone() {
            self.resume = Some(load_resume(&bundle, self.resume_target, &self.project));
            self.verify = None;
            self.view = View::Resume;
        }
    }

    fn run_verify(&mut self) {
        if let Some(bundle) = self.bundle.clone() {
            self.verify = Some(load_verify(&bundle));
        }
    }

    pub fn on_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') => self.quit = true,
            KeyCode::Char('?') => self.view = View::Help,
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.view = View::Sessions;
                self.focus = Focus::List;
            }
            KeyCode::Char('r') | KeyCode::Char('R') if self.bundle.is_some() => self.run_resume(),
            KeyCode::Char('v') | KeyCode::Char('V')
                if self.view == View::Resume && self.bundle.is_some() =>
            {
                self.run_verify()
            }
            KeyCode::Char('t') | KeyCode::Char('T') if self.view == View::Resume => {
                self.resume_target = match self.resume_target {
                    resume::Target::Claude => resume::Target::Codex,
                    resume::Target::Codex => resume::Target::Claude,
                };
                self.run_resume();
            }
            KeyCode::Up | KeyCode::Char('k') if self.view == View::Sessions => {
                self.move_selection(-1)
            }
            KeyCode::Down | KeyCode::Char('j') if self.view == View::Sessions => {
                self.move_selection(1)
            }
            KeyCode::Char(c @ '1'..='9')
                if self.focus == Focus::Detail
                    && self
                        .selected()
                        .is_some_and(|r| r.requires_branch_selection) =>
            {
                let index = (c as usize) - ('1' as usize);
                if self
                    .selected()
                    .is_some_and(|r| index < r.branch_tips.len())
                {
                    self.branch_pick = index;
                    self.open_detail();
                }
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::List => {
                        self.open_detail();
                        Focus::Detail
                    }
                    Focus::Detail => Focus::List,
                };
            }
            KeyCode::Enter if self.view == View::Sessions => self.open_detail(),
            KeyCode::Esc => {
                self.focus = Focus::List;
                if self.view != View::Sessions {
                    self.view = View::Sessions;
                }
            }
            _ => {}
        }
    }
}
