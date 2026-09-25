mod tui;

use memory_bee::{
    bundle::{Options, prepare},
    claude::{Limits, ReadState, inspect},
    discovery::{DiscoveryLimits, discover},
    selection::select,
};
use ratatui::{
    Terminal,
    backend::{CrosstermBackend, TestBackend},
};
use serde::Serialize;
use std::{
    env,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const USAGE: &str = "Usage:\n  memory-bee workspace --demo --project <dir> (--state <private-dir> | --once [--width <n>] [--height <n>]) [--agent claude|codex] [--theme dark|light] [--no-color]\n  memory-bee workspace --claude --project <dir> --state <private-dir> [--resume] [--theme dark|light] [--no-color]\n  memory-bee export-codex <rollout.jsonl> (--preview | --output <new-dir>) [--exclude-line <n>]... [--project <project-dir>] [--include-path <relative-file>]...\n  memory-bee inspect-codex <rollout.jsonl>\n  memory-bee inspect <session.jsonl> [--leaf <uuid>]\n  memory-bee sessions-codex --root <sessions-dir> --project <project-dir>\n  memory-bee sessions --root <projects-dir> --project <project-dir>\n  memory-bee export <session.jsonl> (--preview | --output <new-dir>) [--leaf <uuid>] [--exclude-line <n>]... [--project <project-dir>] [--include-path <relative-file>]...\n  memory-bee verify <bundle-dir>\n  memory-bee apply <bundle-dir> --project <checkout> (--check | --write)\n  memory-bee prepare-resume <bundle-dir> --target (claude | codex) --project <project-dir> [--worktree <new-dir>] (--preview | --output <new-prompt-file> [--launch <confirmation>])\n  memory-bee dashboard --project <project-dir> [--claude-root <dir>] [--codex-root <dir>] [--bundle <dir>] [--theme (dark|light)] [--no-color] [--once [--width <n>] [--height <n>]]\nInspection and export are offline. workspace --claude starts the installed Claude Code CLI.\nCLI launch requires --launch and a matching confirmation from --preview.\nDashboard actions require a preview and explicit confirmation.\nExit: 0 success, 2 partial or attention needed, 3 possible secrets, 4 launched agent exited non-zero, 1 I/O or selection error, 64 usage error.";
fn output(value: &impl Serialize) -> Result<(), (u8, String)> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    serde_json::to_writer_pretty(&mut out, value).map_err(|e| (1, e.to_string()))?;
    writeln!(out).map_err(|e| (1, e.to_string()))
}
fn run() -> Result<u8, (u8, String)> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!("{USAGE}");
        return Ok(0);
    }
    if args.len() == 2 && args[0] == "inspect-codex" {
        let report = memory_bee::codex::inspect(Path::new(&args[1]), Limits::default())
            .map_err(|e| (1, format!("Cannot inspect Codex session: {e}")))?;
        output(&report)?;
        return Ok(if report.state == ReadState::Partial {
            2
        } else {
            0
        });
    }
    if (args.len() == 2 || (args.len() == 4 && args[2] == "--leaf")) && args[0] == "inspect" {
        let mut report = inspect(Path::new(&args[1]), Limits::default())
            .map_err(|e| (1, format!("Cannot inspect session: {e}")))?;
        if args.len() == 4 {
            let leaf = args[3].to_str().ok_or((64, "UUID must be UTF-8".into()))?;
            report = select(report, leaf).map_err(|e| (1, format!("Cannot select branch: {e}")))?;
        }
        output(&report)?;
        return Ok(if report.state == ReadState::Partial {
            2
        } else {
            0
        });
    }
    if args.len() == 5 && (args[0] == "sessions" || args[0] == "sessions-codex") {
        let (root, project) = if args[1] == "--root" && args[3] == "--project" {
            (&args[2], &args[4])
        } else if args[1] == "--project" && args[3] == "--root" {
            (&args[4], &args[2])
        } else {
            return Err((64, USAGE.into()));
        };
        let partial = if args[0] == "sessions-codex" {
            let report = memory_bee::codex_discovery::discover(
                Path::new(root),
                Path::new(project),
                DiscoveryLimits::default(),
            )
            .map_err(|e| (1, format!("Cannot discover Codex sessions: {e}")))?;
            output(&report)?;
            report.partial
        } else {
            let report = discover(
                Path::new(root),
                Path::new(project),
                DiscoveryLimits::default(),
            )
            .map_err(|e| (1, format!("Cannot discover sessions: {e}")))?;
            output(&report)?;
            report.partial
        };
        return Ok(if partial { 2 } else { 0 });
    }
    if args.len() == 2 && args[0] == "verify" {
        let verified = memory_bee::receive::verify(Path::new(&args[1])).map_err(|e| (1, e))?;
        output(&verified.report())?;
        return Ok(0);
    }
    if args.len() == 5
        && args[0] == "apply"
        && args[2] == "--project"
        && (args[4] == "--check" || args[4] == "--write")
    {
        let verified = memory_bee::receive::verify(Path::new(&args[1])).map_err(|e| (1, e))?;
        let plan = verified.check(Path::new(&args[3])).map_err(|e| (1, e))?;
        let report = if args[4] == "--write" {
            plan.write().map_err(|e| (1, e))?
        } else {
            plan.report()
        };
        output(&report)?;
        return Ok(0);
    }
    if args.first().is_some_and(|arg| arg == "prepare-resume") {
        return prepare_resume(&args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "export") {
        return export(&args[1..], false);
    }
    if args.first().is_some_and(|arg| arg == "export-codex") {
        return export(&args[1..], true);
    }
    if args.first().is_some_and(|arg| arg == "workspace") {
        return workspace(&args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "dashboard") {
        return dashboard(&args[1..]);
    }
    Err((64, USAGE.into()))
}

fn export(args: &[std::ffi::OsString], codex: bool) -> Result<u8, (u8, String)> {
    if args.len() < 2 {
        return Err((64, USAGE.into()));
    }
    let mut options = Options::default();
    let mut destination: Option<PathBuf> = None;
    let mut preview = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].to_str() {
            Some("--preview") if !preview => {
                preview = true;
                index += 1;
            }
            Some(
                flag @ ("--output" | "--leaf" | "--exclude-line" | "--project" | "--include-path"),
            ) if index + 1 < args.len() => {
                let value = &args[index + 1];
                match flag {
                    "--include-path" => {
                        options.include_paths.insert(
                            value
                                .to_str()
                                .ok_or((64, "selected path must be UTF-8".into()))?
                                .into(),
                        );
                    }
                    "--project" if options.project.is_none() => {
                        options.project = Some(value.into())
                    }
                    "--output" if destination.is_none() => destination = Some(value.into()),
                    "--leaf" if options.leaf.is_none() => {
                        options.leaf = Some(
                            value
                                .to_str()
                                .ok_or((64, "UUID must be UTF-8".into()))?
                                .into(),
                        )
                    }
                    "--exclude-line" => {
                        let line = value
                            .to_str()
                            .and_then(|v| v.parse::<usize>().ok())
                            .filter(|n| *n > 0)
                            .ok_or((64, "excluded line must be a positive integer".into()))?;
                        options.exclude_lines.insert(line);
                    }
                    _ => return Err((64, USAGE.into())),
                }
                index += 2;
            }
            _ => return Err((64, USAGE.into())),
        }
    }
    if (codex && options.leaf.is_some())
        || preview == destination.is_some()
        || (!options.include_paths.is_empty() && options.project.is_none())
    {
        return Err((64, USAGE.into()));
    }
    let prepared = if codex {
        let report = memory_bee::codex::inspect(Path::new(&args[0]), Limits::default())
            .map_err(|e| (1, format!("Cannot inspect Codex session: {e}")))?;
        memory_bee::bundle::prepare_codex(report, &options)
    } else {
        let report = inspect(Path::new(&args[0]), Limits::default())
            .map_err(|e| (1, format!("Cannot inspect session: {e}")))?;
        prepare(report, &options)
    }
    .map_err(|e| (1, format!("Cannot prepare bundle: {e}")))?;
    let code = if !prepared.findings().is_empty() {
        3
    } else if prepared.is_partial() {
        2
    } else {
        0
    };
    if preview {
        output(&prepared)?;
    } else if code == 3 {
        output(&serde_json::json!({"written":false,"findings":prepared.findings()}))?;
    } else {
        prepared
            .write(destination.as_ref().expect("checked destination"))
            .map_err(|e| (1, format!("Cannot write bundle: {e}")))?;
        output(
            &serde_json::json!({"written":true,"partial":prepared.is_partial(),"redaction":"pending-review"}),
        )?;
    }
    Ok(code)
}
fn dashboard(args: &[std::ffi::OsString]) -> Result<u8, (u8, String)> {
    let mut project: Option<PathBuf> = None;
    let mut claude_root: Option<PathBuf> = None;
    let mut codex_root: Option<PathBuf> = None;
    let mut bundle: Option<PathBuf> = None;
    let mut theme_set = false;
    let mut mode = tui::theme::Mode::Dark;
    let mut no_color = env::var_os("NO_COLOR").is_some();
    let mut once = false;
    let (mut width_set, mut height_set) = (false, false);
    let (mut width, mut height): (u16, u16) = (100, 30);
    let mut index = 0;
    while index < args.len() {
        match args[index].to_str() {
            Some("--no-color") => {
                no_color = true;
                index += 1;
            }
            Some("--once") => {
                once = true;
                index += 1;
            }
            Some(
                flag @ ("--project" | "--claude-root" | "--codex-root" | "--bundle" | "--theme"
                | "--width" | "--height"),
            ) if index + 1 < args.len() => {
                let value = &args[index + 1];
                let slot_empty = match flag {
                    "--project" => project.is_none(),
                    "--claude-root" => claude_root.is_none(),
                    "--codex-root" => codex_root.is_none(),
                    "--bundle" => bundle.is_none(),
                    "--theme" => !theme_set,
                    "--width" => !width_set,
                    _ => !height_set,
                };
                if !slot_empty {
                    return Err((64, USAGE.into()));
                }
                match flag {
                    "--project" => project = Some(value.into()),
                    "--claude-root" => claude_root = Some(value.into()),
                    "--codex-root" => codex_root = Some(value.into()),
                    "--bundle" => bundle = Some(value.into()),
                    "--theme" => {
                        theme_set = true;
                        mode = value
                            .to_str()
                            .and_then(tui::theme::Mode::parse)
                            .ok_or((64, "theme must be dark or light".into()))?;
                    }
                    "--width" => {
                        width_set = true;
                        width = value
                            .to_str()
                            .and_then(|v| v.parse::<u16>().ok())
                            .filter(|n| *n > 0)
                            .ok_or((64, "width must be a positive integer".into()))?;
                    }
                    _ => {
                        height_set = true;
                        height = value
                            .to_str()
                            .and_then(|v| v.parse::<u16>().ok())
                            .filter(|n| *n > 0)
                            .ok_or((64, "height must be a positive integer".into()))?;
                    }
                }
                index += 2;
            }
            _ => return Err((64, USAGE.into())),
        }
    }
    let Some(project) = project else {
        return Err((64, USAGE.into()));
    };
    if claude_root.is_none() && codex_root.is_none() {
        return Err((64, USAGE.into()));
    }
    if (width_set || height_set) && !once {
        return Err((64, USAGE.into()));
    }
    let app = tui::app::App::new(project, claude_root, codex_root, bundle, mode, no_color)
        .map_err(|e| (1, e))?;
    let partial = app.partial;
    if once {
        let mut app = app;
        let mut terminal = Terminal::new(TestBackend::new(width, height))
            .map_err(|e| (1, format!("Cannot render: {e}")))?;
        terminal
            .draw(|f| tui::ui::draw(f, &mut app))
            .map_err(|e| (1, format!("Cannot render: {e}")))?;
        print_plain(terminal.backend().buffer());
        return Ok(if partial { 2 } else { 0 });
    }
    if !io::stdout().is_terminal() {
        return Err((
            64,
            "dashboard needs a terminal; use --once for plain text output".into(),
        ));
    }
    run_interactive(app).map_err(|e| (1, e))?;
    Ok(if partial { 2 } else { 0 })
}

/// Dumps a rendered frame as plain text (glyphs only, no color) for `--once`:
/// accessibility, automation and tests, without a real terminal.
fn print_plain(buffer: &ratatui::buffer::Buffer) {
    let area = buffer.area();
    let mut out = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    print!("{out}");
}

/// Owns the terminal for the session: raw mode and the alternate screen are
/// entered here and always left before returning, including on error or
/// panic. Confirmed launches temporarily release the terminal to the agent.
fn run_interactive(mut app: tui::app::App) -> Result<(), String> {
    crossterm::terminal::enable_raw_mode()
        .and_then(|()| crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen))
        .map_err(|e| e.to_string())?;
    std::panic::set_hook(Box::new(|info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        eprintln!("{info}");
    }));
    let result = (|| -> Result<(), String> {
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
        loop {
            terminal
                .draw(|f| tui::ui::draw(f, &mut app))
                .map_err(|e| e.to_string())?;
            if app.quit {
                return Ok(());
            }
            if crossterm::event::poll(std::time::Duration::from_millis(200))
                .map_err(|e| e.to_string())?
                && let crossterm::event::Event::Key(key) =
                    crossterm::event::read().map_err(|e| e.to_string())?
                && key.kind == crossterm::event::KeyEventKind::Press
            {
                // Raw mode disables SIGINT delivery, so Ctrl+C arrives as a
                // plain key; without this it would do nothing and only `q`
                // could quit, breaking a near-universal terminal expectation.
                if key.code == crossterm::event::KeyCode::Char('c')
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    return Ok(());
                }
                app.on_key(key.code);
                if let Some(tui::actions::Pending::Prompt {
                    request,
                    output,
                    token,
                    launch: true,
                }) = app.pending_launch.take()
                {
                    crossterm::terminal::disable_raw_mode().map_err(|e| e.to_string())?;
                    crossterm::execute!(
                        io::stdout(),
                        crossterm::terminal::LeaveAlternateScreen,
                        crossterm::cursor::Show
                    )
                    .map_err(|e| e.to_string())?;
                    let outcome = tui::actions::execute_prompt(&request, &output, &token, true);
                    crossterm::terminal::enable_raw_mode().map_err(|e| e.to_string())?;
                    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)
                        .map_err(|e| e.to_string())?;
                    terminal.clear().map_err(|e| e.to_string())?;
                    app.resume = None;
                    app.verify = None;
                    app.dialog = Some(tui::actions::Dialog::message(match outcome {
                        Ok(message) => message,
                        Err(error) => format!("Erro: {error}"),
                    }));
                }
            }
        }
    })();
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    );
    result
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn prepare_resume(args: &[std::ffi::OsString]) -> Result<u8, (u8, String)> {
    let Some(bundle) = args.first() else {
        return Err((64, USAGE.into()));
    };
    let (mut target, mut project, mut worktree, mut prompt_file) = (None, None, None, None);
    let mut launch: Option<String> = None;
    let mut preview = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].to_str() {
            Some("--preview") if !preview => {
                preview = true;
                index += 1;
            }
            Some(flag @ ("--target" | "--project" | "--worktree" | "--output" | "--launch"))
                if index + 1 < args.len() =>
            {
                let value = &args[index + 1];
                let slot_empty = match flag {
                    "--target" => target.is_none(),
                    "--project" => project.is_none(),
                    "--worktree" => worktree.is_none(),
                    "--launch" => launch.is_none(),
                    _ => prompt_file.is_none(),
                };
                if !slot_empty {
                    return Err((64, USAGE.into()));
                }
                match flag {
                    "--target" => {
                        target = Some(
                            value
                                .to_str()
                                .and_then(memory_bee::resume::Target::parse)
                                .ok_or((64, "target must be claude or codex".into()))?,
                        )
                    }
                    "--project" => project = Some(PathBuf::from(value)),
                    "--worktree" => worktree = Some(PathBuf::from(value)),
                    "--launch" => {
                        launch = Some(
                            value
                                .to_str()
                                .filter(|v| {
                                    v.len() == 16 && v.bytes().all(|b| b.is_ascii_hexdigit())
                                })
                                .ok_or((
                                    64,
                                    "confirmation must be the 16-character token from --preview"
                                        .into(),
                                ))?
                                .to_ascii_lowercase(),
                        )
                    }
                    _ => prompt_file = Some(PathBuf::from(value)),
                }
                index += 2;
            }
            _ => return Err((64, USAGE.into())),
        }
    }
    let (Some(target), Some(project)) = (target, project) else {
        return Err((64, USAGE.into()));
    };
    if preview == prompt_file.is_some() || (launch.is_some() && prompt_file.is_none()) {
        return Err((64, USAGE.into()));
    }
    let request = memory_bee::resume::Request {
        bundle: bundle.into(),
        target,
        project,
        worktree,
    };
    let mut preparation = memory_bee::resume::prepare(&request, prompt_file.as_deref())
        .map_err(|e| (1, format!("Cannot prepare resume: {e}")))?;
    if let Some(token) = &launch {
        memory_bee::resume::check_launch(&preparation, token)
            .map_err(|e| (1, format!("Cannot launch: {e}")))?;
    }
    if prompt_file.is_some() {
        memory_bee::resume::write_prompt(&preparation).map_err(|e| (1, e))?;
    }
    if launch.is_some() {
        // The agent owns the terminal; the report is printed after it exits.
        let started = memory_bee::resume::launch(&mut preparation);
        output(&preparation)?;
        started.map_err(|e| (1, format!("Cannot launch: {e}")))?;
        return Ok(if preparation.launch_exit_code == Some(0) {
            0
        } else {
            4
        });
    }
    output(&preparation)?;
    Ok(if preparation.needs_attention() { 2 } else { 0 })
}

fn workspace(args: &[std::ffi::OsString]) -> Result<u8, (u8, String)> {
    use memory_bee::workspace::{
        Agent,
        store::{State, Store},
    };
    let mut project: Option<PathBuf> = None;
    let mut state: Option<PathBuf> = None;
    let mut demo = false;
    let mut claude_live = false;
    let mut resume = false;
    let mut once = false;
    let mut agent = Agent::Claude;
    let mut mode = tui::theme::Mode::Dark;
    let mut no_color = env::var_os("NO_COLOR").is_some();
    let (mut width, mut height) = (100u16, 30u16);
    let mut seen = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].to_str().ok_or((64, USAGE.into()))?;
        if !seen.insert(flag.to_owned()) {
            return Err((64, USAGE.into()));
        }
        match flag {
            "--demo" => demo = true,
            "--claude" => claude_live = true,
            "--resume" => resume = true,
            "--once" => once = true,
            "--no-color" => no_color = true,
            "--project" | "--state" | "--agent" | "--theme" | "--width" | "--height" => {
                i += 1;
                let value = args.get(i).ok_or((64, USAGE.into()))?;
                match flag {
                    "--project" => project = Some(value.into()),
                    "--state" => state = Some(value.into()),
                    "--agent" => {
                        agent = value
                            .to_str()
                            .and_then(Agent::parse)
                            .ok_or((64, USAGE.into()))?
                    }
                    "--theme" => {
                        mode = value
                            .to_str()
                            .and_then(tui::theme::Mode::parse)
                            .ok_or((64, USAGE.into()))?
                    }
                    _ => {
                        let n = value
                            .to_str()
                            .and_then(|s| s.parse::<u16>().ok())
                            .filter(|n| *n > 0 && *n <= 500)
                            .ok_or((64, "Dimensions must be between 1 and 500".into()))?;
                        if flag == "--width" {
                            width = n;
                        } else {
                            height = n;
                        }
                    }
                }
            }
            _ => return Err((64, USAGE.into())),
        }
        i += 1;
    }
    if (demo && claude_live) || (resume && !claude_live) {
        return Err((64, USAGE.into()));
    }
    if !demo && !claude_live {
        memory_bee::workspace::live_gate().map_err(|e| (64, e))?;
    }
    let project = project
        .ok_or((64, USAGE.into()))?
        .canonicalize()
        .map_err(|e| (1, e.to_string()))?;
    if !project.is_dir() {
        return Err((64, "Project must be a directory".into()));
    }
    let project = project
        .to_str()
        .ok_or((64, "Project must be UTF-8".into()))?
        .to_owned();
    if once && state.is_some() || !once && (seen.contains("--width") || seen.contains("--height")) {
        return Err((64, USAGE.into()));
    }
    if claude_live {
        if once || seen.contains("--agent") {
            return Err((64, USAGE.into()));
        }
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err((
                64,
                "workspace --claude precisa de terminal interativo".into(),
            ));
        }
        let state_path =
            state.ok_or((64, "workspace --claude exige --state <private-dir>".into()))?;
        let (id, code) =
            tui::claude_native::run(Path::new(&project), &state_path, resume, mode, no_color)
                .map_err(|e| (1, e))?;
        println!("Claude Code encerrou. Sessão nativa: {id}");
        return Ok(if code == 0 { 0 } else { 4 });
    }
    if once {
        let state = State::new(project, agent);
        let mut app = tui::workspace::App::new(state, None, mode, no_color);
        app.draft = "Adicione busca por nome na lista de sessões. (exemplo sintético)".into();
        app.key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        for _ in 0..5 {
            app.tick();
        }
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).map_err(|e| (1, e.to_string()))?;
        terminal
            .draw(|f| tui::workspace::draw(f, &app))
            .map_err(|e| (1, e.to_string()))?;
        print_plain(terminal.backend().buffer());
        return Ok(0);
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err((
            64,
            "workspace needs a terminal; use --once for a read-only demo".into(),
        ));
    }
    let state_path = state.ok_or((64, "Interactive demo requires --state <private-dir>".into()))?;
    let (store, state) = Store::open(&state_path, &project, agent).map_err(|e| (1, e))?;
    let mut app = tui::workspace::App::new(state, Some(store), mode, no_color);
    app.home = true;
    run_workspace(app).map_err(|e| (1, e))?;
    Ok(0)
}

fn run_workspace(mut app: tui::workspace::App) -> Result<(), String> {
    use crossterm::{
        cursor::Show,
        event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind},
        terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    };
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = terminal::disable_raw_mode();
            let _ = crossterm::execute!(
                io::stdout(),
                DisableBracketedPaste,
                LeaveAlternateScreen,
                Show
            );
        }
    }
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let _restore = Restore;
    crossterm::execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)
        .map_err(|e| e.to_string())?;
    let mut terminal =
        Terminal::new(CrosstermBackend::new(io::stdout())).map_err(|e| e.to_string())?;
    let mut last = std::time::Instant::now();
    while !app.quit {
        terminal
            .draw(|f| tui::workspace::draw(f, &app))
            .map_err(|e| e.to_string())?;
        if event::poll(std::time::Duration::from_millis(50)).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.key(key),
                Event::Paste(text) => app.paste(&text),
                _ => {}
            }
        }
        if last.elapsed() >= std::time::Duration::from_millis(250) {
            app.tick();
            last = std::time::Instant::now();
        }
    }
    Ok(())
}
