//! Dashboard state, navigation, and explicit action confirmations.
use super::actions::{self, Dialog, Form, Pending};
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

#[derive(Debug)]
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
        let report = codex::inspect(&row.path, Limits::default()).map_err(|e| e.to_string())?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Sessions,
    Resume,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub dialog: Option<Dialog>,
    pub pending_launch: Option<Pending>,
    pub scroll: u16,
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
            dialog: None,
            pending_launch: None,
            scroll: 0,
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

    fn request(&self) -> Result<resume::Request, String> {
        Ok(resume::Request {
            bundle: self.bundle.clone().ok_or("Nenhum pacote selecionado")?,
            target: self.resume_target,
            project: self.project.clone(),
            worktree: None,
        })
    }

    fn submit_dialog(&mut self, mut dialog: Dialog) -> Result<Option<Dialog>, String> {
        if let Some(form) = dialog.form.take() {
            return match form {
                Form::ExportPath => {
                    if dialog.input.is_empty() {
                        return Err("Informe um destino novo".into());
                    }
                    Ok(Some(Dialog::form(Form::ExportExclusions(
                        dialog.input.into(),
                    ))))
                }
                Form::ExportExclusions(output) => {
                    let row = self.selected().ok_or("Nenhuma sessão selecionada")?;
                    let leaf = if row.requires_branch_selection {
                        Some(
                            row.branch_tips
                                .get(self.branch_pick)
                                .ok_or("Escolha um ramo no detalhe")?
                                .as_str(),
                        )
                    } else {
                        None
                    };
                    Ok(Some(actions::export(row, leaf, output, &dialog.input)?))
                }
                Form::PromptPath { launch } => Ok(Some(actions::prompt(
                    self.request()?,
                    dialog.input.into(),
                    launch,
                )?)),
            };
        }
        if dialog.pending.is_none() {
            return Ok(None);
        }
        if dialog.input != dialog.expected {
            return Ok(Some(dialog));
        }
        match dialog.pending.take().unwrap() {
            Pending::Export { prepared, output } => {
                prepared.write(&output).map_err(|e| e.to_string())?;
                self.bundle = Some(output.clone());
                self.resume = None;
                self.verify = None;
                Ok(Some(Dialog::message(format!(
                    "Pacote gravado: {}\nUse r para preparar a retomada.",
                    output.display()
                ))))
            }
            Pending::Apply(plan) => {
                let report = plan.write()?;
                self.run_resume();
                Ok(Some(Dialog::message(
                    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?,
                )))
            }
            pending @ Pending::Prompt { launch: true, .. } => {
                self.pending_launch = Some(pending);
                Ok(None)
            }
            Pending::Prompt {
                request,
                output,
                token,
                launch: false,
            } => Ok(Some(Dialog::message(actions::execute_prompt(
                &request, &output, &token, false,
            )?))),
        }
    }

    fn dialog_key(&mut self, code: KeyCode) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        match code {
            KeyCode::Esc => return,
            KeyCode::Enter => {
                self.dialog = match self.submit_dialog(dialog) {
                    Ok(next) => next,
                    Err(e) => Some(Dialog::message(format!("Erro: {e}"))),
                };
                return;
            }
            KeyCode::PageDown => dialog.scroll = dialog.scroll.saturating_add(5),
            KeyCode::PageUp => dialog.scroll = dialog.scroll.saturating_sub(5),
            KeyCode::Home => dialog.scroll = 0,
            KeyCode::Backspace => {
                dialog.input.pop();
            }
            KeyCode::Char(c)
                if !c.is_control() && (dialog.form.is_some() || dialog.pending.is_some()) =>
            {
                dialog.input.push(c)
            }
            _ => {}
        }
        self.dialog = Some(dialog);
    }

    pub fn on_key(&mut self, code: KeyCode) {
        if self.dialog.is_some() {
            self.dialog_key(code);
            return;
        }
        match code {
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(5);
                return;
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(5);
                return;
            }
            KeyCode::Home => {
                self.scroll = 0;
                return;
            }
            _ => self.scroll = 0,
        }

        match code {
            KeyCode::Char('e') if self.view == View::Sessions && self.selected().is_some() => {
                self.open_detail();
                self.dialog = Some(Dialog::form(Form::ExportPath));
            }
            KeyCode::Char('a') if self.view == View::Resume && self.bundle.is_some() => {
                self.dialog = Some(
                    match actions::apply(self.bundle.as_ref().unwrap(), &self.project) {
                        Ok(dialog) => dialog,
                        Err(e) => Dialog::message(format!("Erro: {e}")),
                    },
                );
            }
            KeyCode::Char(c @ ('p' | 'l')) if self.view == View::Resume => {
                self.dialog = Some(Dialog::form(Form::PromptPath { launch: c == 'l' }));
            }
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
                if self.view == View::Sessions
                    && self.focus == Focus::Detail
                    && self.selected().is_some_and(|r| r.requires_branch_selection) =>
            {
                let index = (c as usize) - ('1' as usize);
                if self.selected().is_some_and(|r| index < r.branch_tips.len()) {
                    self.branch_pick = index;
                    self.open_detail();
                }
            }
            KeyCode::Tab if self.view == View::Sessions => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixtures() -> (PathBuf, PathBuf, PathBuf) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        (
            PathBuf::from("/synthetic/project"),
            root.join("testdata/claude-projects"),
            root.join("testdata/codex-sessions"),
        )
    }

    #[test]
    fn load_sessions_unifies_and_sorts_claude_and_codex() {
        let (project, claude_root, codex_root) = fixtures();
        let (rows, _partial) =
            load_sessions(&project, Some(&claude_root), Some(&codex_root)).unwrap();
        assert!(rows.iter().any(|r| r.agent == "claude-code"));
        assert!(rows.iter().any(|r| r.agent == "codex"));
        // Sorted by path: consecutive rows never go backwards.
        assert!(rows.windows(2).all(|w| w[0].path <= w[1].path));
        // arbitrary/session.jsonl (the root session, not its subagent) requires
        // a branch pick between a1 and a2.
        let root_session = rows
            .iter()
            .find(|r| r.path.file_name().and_then(|n| n.to_str()) == Some("session.jsonl"))
            .expect("the root fixture session is discovered");
        assert!(root_session.requires_branch_selection);
        assert_eq!(
            root_session.branch_tips,
            vec!["a1".to_string(), "a2".to_string()]
        );
    }

    #[test]
    fn load_sessions_rejects_a_missing_root() {
        let (project, _, _) = fixtures();
        let err = load_sessions(&project, Some(Path::new("/does/not/exist")), None)
            .expect_err("a missing root is a hard error, not a partial result");
        assert!(err.contains("Cannot discover sessions"));
    }

    #[test]
    fn app_new_selects_the_first_row_and_loads_no_detail_yet() {
        let (project, claude_root, codex_root) = fixtures();
        let app = App::new(
            project,
            Some(claude_root),
            Some(codex_root),
            None,
            Mode::Dark,
            false,
        )
        .unwrap();
        assert_eq!(app.list_state.selected(), Some(0));
        assert!(app.detail.is_none());
        assert!(app.bundle.is_none());
    }

    fn test_app() -> App {
        let (project, claude_root, codex_root) = fixtures();
        App::new(
            project,
            Some(claude_root),
            Some(codex_root),
            None,
            Mode::Dark,
            true,
        )
        .unwrap()
    }

    #[test]
    fn arrow_keys_wrap_the_selection_and_clear_stale_detail() {
        let mut app = test_app();
        let len = app.sessions.len();
        app.on_key(KeyCode::Tab); // opens detail for row 0
        assert!(app.detail.is_some());
        for _ in 0..len {
            app.on_key(KeyCode::Up);
        }
        // len steps back from 0 wraps exactly once around.
        assert_eq!(app.list_state.selected(), Some(0));
        assert!(
            app.detail.is_none(),
            "moving the selection must drop stale detail"
        );
        app.on_key(KeyCode::Down);
        assert_eq!(app.list_state.selected(), Some(1 % len));
    }

    #[test]
    fn tab_opens_detail_and_reports_a_real_report() {
        let mut app = test_app();
        // Pick a non-ambiguous row so detail loads without a branch choice.
        let index = app
            .sessions
            .iter()
            .position(|r| !r.requires_branch_selection)
            .unwrap();
        app.list_state.select(Some(index));
        app.on_key(KeyCode::Tab);
        assert_eq!(app.focus, Focus::Detail);
        match app.detail.as_ref().unwrap() {
            Ok(_) => {}
            Err(e) => panic!("expected a readable session, got {e}"),
        }
        app.on_key(KeyCode::Tab);
        assert_eq!(app.focus, Focus::List);
    }

    #[test]
    fn number_keys_pick_a_branch_tip_for_an_ambiguous_session() {
        let mut app = test_app();
        // The root session (2 tips), not its subagent (1 tip): only it can
        // exercise picking the second tip.
        let index = app
            .sessions
            .iter()
            .position(|r| r.path.file_name().and_then(|n| n.to_str()) == Some("session.jsonl"))
            .unwrap();
        app.list_state.select(Some(index));
        app.on_key(KeyCode::Tab);
        assert_eq!(app.branch_pick, 0);
        app.on_key(KeyCode::Char('2'));
        assert_eq!(app.branch_pick, 1);
        let Some(Ok(Detail::Claude(report))) = &app.detail else {
            panic!("expected a resolved Claude report after picking a branch");
        };
        assert_eq!(report.selection.as_ref().unwrap().leaf_uuid, "a2");
        // Out-of-range picks are ignored, not clamped or panicking.
        app.on_key(KeyCode::Char('9'));
        assert_eq!(app.branch_pick, 1);
    }

    #[test]
    fn esc_returns_to_the_session_list_from_any_view() {
        let mut app = test_app();
        app.view = View::Help;
        app.on_key(KeyCode::Esc);
        assert_eq!(app.view, View::Sessions);
        assert_eq!(app.focus, Focus::List);
    }

    #[test]
    fn q_sets_quit_without_touching_anything_else() {
        let mut app = test_app();
        app.on_key(KeyCode::Char('q'));
        assert!(app.quit);
    }

    #[test]
    fn resume_and_verify_keys_are_inert_without_a_bundle() {
        let mut app = test_app();
        app.on_key(KeyCode::Char('r'));
        assert_eq!(app.view, View::Sessions, "no --bundle means no resume view");
        assert!(app.resume.is_none());
    }

    #[test]
    fn resume_preview_and_verify_use_the_example_bundle_read_only() {
        let (_, claude_root, codex_root) = fixtures();
        // Unlike discovery's `project`, `resume::prepare` inspects the real
        // filesystem (git state), so it needs a directory that actually
        // exists, not the fixtures' synthetic `/synthetic/project`.
        let real_project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let bundle = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bundle-v1");
        let mut app = App::new(
            real_project,
            Some(claude_root),
            Some(codex_root),
            Some(bundle.clone()),
            Mode::Dark,
            true,
        )
        .unwrap();
        app.on_key(KeyCode::Char('r'));
        assert_eq!(app.view, View::Resume);
        let prep = app
            .resume
            .as_ref()
            .unwrap()
            .as_ref()
            .expect("the example bundle prepares a read-only preview");
        assert!(!prep.launched, "the dashboard must never launch an agent");
        app.on_key(KeyCode::Char('v'));
        let verified = app.verify.as_ref().unwrap().as_ref().unwrap();
        assert!(verified.valid);
        // The example bundle is still on disk, unmodified by any of this.
        assert!(bundle.join("manifest.json").is_file());

        app.on_key(KeyCode::Char('t'));
        assert_eq!(app.resume_target, resume::Target::Codex);
    }
}
