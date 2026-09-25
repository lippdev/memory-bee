use super::{
    actions::{self, Dialog, Pending},
    app::*,
    theme::Mode,
    ui,
};
use crossterm::event::KeyCode;
use memory_bee::{bundle, claude, receive, resume};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "memory-bee-actions-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}
fn app() -> App {
    App::new(
        "/synthetic/project".into(),
        Some(fixture("testdata/claude-projects")),
        Some(fixture("testdata/codex-sessions")),
        None,
        Mode::Dark,
        true,
    )
    .unwrap()
}
fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        app.on_key(KeyCode::Char(c));
    }
}
fn enter(app: &mut App) {
    app.on_key(KeyCode::Enter);
}
fn select(app: &mut App, codex: bool) {
    let i = app
        .sessions
        .iter()
        .position(|r| {
            if codex {
                r.agent == "codex"
            } else {
                r.path.ends_with("arbitrary/session.jsonl")
            }
        })
        .unwrap();
    app.list_state.select(Some(i));
}
fn confirm(app: &mut App) {
    let token = app.dialog.as_ref().unwrap().expected.clone();
    type_text(app, &token);
    enter(app);
}
#[test]
fn export_keyboard_reviews_selected_branch_and_never_overwrites() {
    let tmp = Temp::new();
    let output = tmp.0.join("pacote com espaço");
    let mut app = app();
    select(&mut app, false);
    app.on_key(KeyCode::Tab);
    app.on_key(KeyCode::Char('2'));
    app.on_key(KeyCode::Char('e'));
    type_text(&mut app, output.to_str().unwrap());
    enter(&mut app);
    enter(&mut app);
    assert!(!output.exists());
    assert!(app.dialog.as_ref().unwrap().preview.contains("a2"));
    enter(&mut app);
    assert!(!output.exists(), "Enter alone cannot authorize writes");
    confirm(&mut app);
    assert!(receive::verify(&output).is_ok());
    assert_eq!(app.bundle.as_ref(), Some(&output));
    let before = fs::read(output.join("manifest.json")).unwrap();
    enter(&mut app);
    app.on_key(KeyCode::Char('e'));
    type_text(&mut app, output.to_str().unwrap());
    enter(&mut app);
    enter(&mut app);
    confirm(&mut app);
    assert!(app.dialog.as_ref().unwrap().preview.contains("Erro:"));
    assert_eq!(before, fs::read(output.join("manifest.json")).unwrap());
}
#[test]
fn codex_export_cancellation_and_snapshot() {
    let tmp = Temp::new();
    let output = tmp.0.join("codex");
    let mut app = app();
    select(&mut app, true);
    app.on_key(KeyCode::Char('e'));
    type_text(&mut app, output.to_str().unwrap());
    enter(&mut app);
    enter(&mut app);
    app.on_key(KeyCode::Esc);
    assert!(!output.exists());
    let row = app.selected().unwrap();
    let mut dialog = actions::export(row, None, output.clone(), "").unwrap();
    let Pending::Export { prepared, .. } = dialog.pending.take().unwrap() else {
        panic!()
    };
    prepared.write(&output).unwrap();
    assert!(receive::verify(&output).is_ok());
}
#[test]
fn exclusions_reject_invalid_numbers() {
    let app = app();
    let row = app.selected().unwrap();
    for input in ["0", "1,no", "-1"] {
        assert!(actions::export(row, None, "unused".into(), input).is_err());
    }
}
fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
fn repository(root: &Path) {
    fs::create_dir(root).unwrap();
    git(root, &["init", "-b", "test"]);
    git(root, &["config", "user.name", "Synthetic"]);
    git(root, &["config", "user.email", "synthetic@example.invalid"]);
    git(root, &["config", "commit.gpgsign", "false"]);
    git(root, &["config", "core.hooksPath", "/dev/null"]);
    fs::write(root.join("a.txt"), "base\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "fixture"]);
}
fn changed_bundle(tmp: &Temp) -> (PathBuf, PathBuf) {
    let source = tmp.0.join("source");
    repository(&source);
    let target = tmp.0.join("target");
    git(
        &source,
        &["clone", source.to_str().unwrap(), target.to_str().unwrap()],
    );
    fs::write(source.join("a.txt"), "changed\n").unwrap();
    let output = tmp.0.join("bundle");
    bundle::prepare(
        claude::inspect(
            &fixture("testdata/claude/basic.jsonl"),
            claude::Limits::default(),
        )
        .unwrap(),
        &bundle::Options {
            project: Some(source),
            include_paths: ["a.txt".into()].into(),
            ..Default::default()
        },
    )
    .unwrap()
    .write(&output)
    .unwrap();
    (output, target)
}
#[test]
fn apply_keyboard_requires_confirmation_and_rechecks_dirty_checkout() {
    let tmp = Temp::new();
    let (bundle, target) = changed_bundle(&tmp);
    let mut app = app();
    app.bundle = Some(bundle);
    app.project = target.clone();
    app.view = View::Resume;
    app.on_key(KeyCode::Char('a'));
    enter(&mut app);
    assert_eq!(fs::read_to_string(target.join("a.txt")).unwrap(), "base\n");
    fs::write(target.join("a.txt"), "concurrent\n").unwrap();
    confirm(&mut app);
    assert!(app.dialog.as_ref().unwrap().preview.contains("Erro:"));
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).unwrap(),
        "concurrent\n"
    );
    fs::write(target.join("a.txt"), "base\n").unwrap();
    enter(&mut app);
    app.on_key(KeyCode::Char('a'));
    confirm(&mut app);
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).unwrap(),
        "changed\n"
    );
}
#[test]
fn prompt_rechecks_token_before_writing_and_refuses_existing_file() {
    let tmp = Temp::new();
    let root = tmp.0.join("repo");
    repository(&root);
    let request = resume::Request {
        bundle: fixture("examples/bundle-v1"),
        project: root.clone(),
        target: resume::Target::Codex,
        worktree: None,
    };
    let output = tmp.0.join("prompt");
    let dialog = actions::prompt(request, output.clone(), false).unwrap();
    let Pending::Prompt { request, token, .. } = dialog.pending.unwrap() else {
        panic!()
    };
    fs::write(root.join("a.txt"), "dirty\n").unwrap();
    assert!(actions::execute_prompt(&request, &output, &token, false).is_err());
    assert!(!output.exists());
    fs::write(root.join("a.txt"), "base\n").unwrap();
    actions::execute_prompt(&request, &output, &token, false).unwrap();
    let before = fs::read(&output).unwrap();
    assert!(actions::execute_prompt(&request, &output, &token, false).is_err());
    assert_eq!(before, fs::read(output).unwrap());
}
#[test]
fn launch_is_deferred_until_terminal_is_released_and_escape_cancels() {
    let tmp = Temp::new();
    let root = tmp.0.join("repo");
    repository(&root);
    let mut app = app();
    app.bundle = Some(fixture("examples/bundle-v1"));
    app.project = root;
    app.view = View::Resume;
    let output = tmp.0.join("prompt");
    app.on_key(KeyCode::Char('l'));
    type_text(&mut app, output.to_str().unwrap());
    enter(&mut app);
    assert!(app.dialog.as_ref().unwrap().pending.is_some());
    assert!(!output.exists());
    enter(&mut app);
    assert!(app.pending_launch.is_none());
    app.on_key(KeyCode::Esc);
    assert!(app.pending_launch.is_none());
    app.on_key(KeyCode::Char('l'));
    type_text(&mut app, output.to_str().unwrap());
    enter(&mut app);
    confirm(&mut app);
    assert!(matches!(
        app.pending_launch,
        Some(Pending::Prompt { launch: true, .. })
    ));
    assert!(
        !output.exists(),
        "key handler must not launch or write before releasing the terminal"
    );
}
#[test]
fn dialog_keys_do_not_change_selection_and_unicode_backspace_is_safe() {
    let mut app = app();
    app.on_key(KeyCode::Char('e'));
    let selected = app.list_state.selected();
    type_text(&mut app, "qétr");
    app.on_key(KeyCode::Backspace);
    assert_eq!(app.dialog.as_ref().unwrap().input, "qét");
    assert!(!app.quit);
    assert_eq!(selected, app.list_state.selected());
    app.on_key(KeyCode::Esc);
    assert!(app.dialog.is_none());
}
#[test]
fn confirmation_and_cancel_remain_visible_in_narrow_no_color_dialog() {
    let mut app = app();
    app.dialog = Some(Dialog::message("Line\n".repeat(100)));
    for (width, height) in [(30, 8), (40, 12), (60, 20), (100, 30)] {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("Esc cancela"));
        assert!(text.contains("Enter fecha"));
    }
}

#[test]
fn secret_findings_block_export_until_excluded_and_preview_keeps_snapshot() {
    let tmp = Temp::new();
    let source = tmp.0.join("session.jsonl");
    let token = format!("ghp_{}", "syntheticfixture".repeat(3));
    let mut lines: Vec<serde_json::Value> =
        fs::read_to_string(fixture("testdata/claude/basic.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    lines[1]["message"]["content"] = serde_json::json!(token);
    fs::write(
        &source,
        lines
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .unwrap();
    let mut app = app();
    select(&mut app, false);
    let row = &mut app.sessions[app.list_state.selected().unwrap()];
    row.path = source.clone();
    row.requires_branch_selection = false;
    let mut dialog = actions::export(row, None, tmp.0.join("blocked"), "").unwrap();
    let Pending::Export { prepared, output } = dialog.pending.take().unwrap() else {
        panic!()
    };
    assert!(!prepared.findings().is_empty());
    assert!(prepared.write(&output).is_err());
    assert!(!output.exists());
    let mut dialog = actions::export(row, None, tmp.0.join("safe"), "2").unwrap();
    let Pending::Export { prepared, output } = dialog.pending.take().unwrap() else {
        panic!()
    };
    fs::write(source, "changed after preview").unwrap();
    prepared.write(&output).unwrap();
    assert!(receive::verify(&output).is_ok());
    let history = fs::read_to_string(output.join("history.jsonl")).unwrap();
    assert!(!history.contains(&token));
    assert!(!history.contains("changed after preview"));
}
