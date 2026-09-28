//! Memory Bee frame around the unmodified, interactive Claude Code CLI.
use super::theme::{Mode, Palette};
use crossterm::{
    cursor::Show,
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use memory_bee::workspace::claude_native::ClaudeStore;
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use std::{
    ffi::OsStr,
    io::{self, BufRead, Read, Write},
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
#[cfg(unix)]
use unicode_width::UnicodeWidthStr;
use uuid::Uuid;

#[cfg(unix)]
use std::{
    os::unix::net::{UnixListener, UnixStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

enum Output {
    Bytes(Vec<u8>),
    Closed,
}

struct Pty {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output: Receiver<Output>,
    exited: bool,
}
impl Pty {
    fn start(
        executable: &OsStr,
        project: &Path,
        id: Uuid,
        resume: bool,
        size: PtySize,
        no_color: bool,
        bridge: Option<(&Path, &str)>,
    ) -> Result<Self, String> {
        let pair = NativePtySystem::default()
            .openpty(size)
            .map_err(|e| e.to_string())?;
        let mut command = CommandBuilder::new(executable);
        command.cwd(project.as_os_str());
        command.env("TERM", "xterm-256color");
        if resume {
            command.arg("--resume");
        } else {
            command.arg("--session-id");
        }
        command.arg(id.to_string());
        if let Some((socket, settings)) = bridge {
            command.env("MEMORY_BEE_CLAUDE_SOCKET", socket.as_os_str());
            command.arg("--settings");
            command.arg(settings);
            command.arg("--permission-mode");
            command.arg("default");
        }
        if no_color {
            command.env("NO_COLOR", "1");
        }
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| e.to_string())?;
        drop(pair.slave);
        let mut reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        };
        let (tx, output) = mpsc::sync_channel(64);
        thread::spawn(move || {
            let mut bytes = [0u8; 8192];
            loop {
                match reader.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(Output::Bytes(bytes[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = tx.send(Output::Closed);
        });
        Ok(Self {
            master: pair.master,
            child,
            writer,
            output,
            exited: false,
        })
    }
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.writer.write_all(bytes).map_err(|e| e.to_string())
    }
    fn resize(&mut self, rows: u16, cols: u16) -> Result<(), String> {
        self.master
            .resize(pty_size(rows, cols))
            .map_err(|e| e.to_string())
    }
    fn try_wait(&mut self) -> Result<Option<portable_pty::ExitStatus>, String> {
        let status = self.child.try_wait().map_err(|e| e.to_string())?;
        if status.is_some() {
            self.exited = true;
        }
        Ok(status)
    }
}
impl Drop for Pty {
    fn drop(&mut self) {
        if !self.exited {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> Result<Self, String> {
        terminal::enable_raw_mode().map_err(|e| e.to_string())?;
        let guard = Self;
        crossterm::execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)
            .map_err(|e| e.to_string())?;
        Ok(guard)
    }
}
impl Drop for TerminalGuard {
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

fn viewport(size: Rect) -> (u16, u16) {
    (
        size.height.saturating_sub(4).max(1),
        size.width.saturating_sub(2).max(1),
    )
}
fn pty_size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}
fn color(value: vt100::Color) -> Option<Color> {
    match value {
        vt100::Color::Default => None,
        vt100::Color::Idx(index) => Some(Color::Indexed(index)),
        vt100::Color::Rgb(r, g, b) => Some(Color::Rgb(r, g, b)),
    }
}
fn paint_screen(frame: &mut Frame, area: Rect, screen: &vt100::Screen, no_color: bool) {
    let buffer = frame.buffer_mut();
    for row in 0..area.height {
        for col in 0..area.width {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if cell.is_wide_continuation() {
                continue;
            }
            let mut style = Style::default();
            if !no_color {
                if let Some(fg) = color(cell.fgcolor()) {
                    style = style.fg(fg);
                }
                if let Some(bg) = color(cell.bgcolor()) {
                    style = style.bg(bg);
                }
            }
            if cell.bold() {
                style = style.add_modifier(Modifier::BOLD);
            }
            if cell.dim() {
                style = style.add_modifier(Modifier::DIM);
            }
            if cell.italic() {
                style = style.add_modifier(Modifier::ITALIC);
            }
            if cell.underline() {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            if cell.inverse() {
                style = style.add_modifier(Modifier::REVERSED);
            }
            let symbol = if cell.has_contents() {
                cell.contents()
            } else {
                " "
            };
            buffer.set_stringn(
                area.x + col,
                area.y + row,
                symbol,
                (area.width - col) as usize,
                style,
            );
        }
    }
}
fn draw(
    frame: &mut Frame,
    screen: &vt100::Screen,
    id: Uuid,
    help: bool,
    no_color: bool,
    mode: Mode,
) {
    let area = frame.area();
    if area.width < 30 || area.height < 8 {
        frame.render_widget(
            Paragraph::new("Memory Bee · aumente o terminal para 30×8"),
            area,
        );
        return;
    }
    let p = Palette::new(mode, no_color);
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(format!("⬢ Memory Bee · Claude real · {}", id))
            .style(Style::default().fg(p.accent).add_modifier(Modifier::BOLD)),
        rows[0],
    );
    let block = Block::default()
        .title(" Claude Code original ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(rows[1]);
    frame.render_widget(block, rows[1]);
    paint_screen(frame, inner, screen, no_color);
    if help {
        let popup = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width.saturating_sub(4),
            height: 5.min(area.height.saturating_sub(3)),
        };
        frame.render_widget(ratatui::widgets::Clear, popup);
        frame.render_widget(Paragraph::new("Claude usa o login e as permissões do próprio CLI.\nCtrl+G ou Esc fecha esta ajuda. Ctrl+C vai para Claude.\nPara sair da sessão, use /exit no Claude.")
            .block(Block::default().title(" Memory Bee ").borders(Borders::ALL).border_style(Style::default().fg(p.accent))), popup);
    } else {
        let (cursor_row, cursor_col) = screen.cursor_position();
        if cursor_row < inner.height && cursor_col < inner.width {
            frame.set_cursor_position((inner.x + cursor_col, inner.y + cursor_row));
        }
    }
    frame.render_widget(
        Paragraph::new("Ctrl+G ajuda · /exit encerra Claude · sessão nativa preservada"),
        rows[2],
    );
}

fn key_bytes(key: KeyEvent, application_cursor: bool) -> Option<Vec<u8>> {
    let bytes: Vec<u8> = match key.code {
        KeyCode::Char(c)
            if key.modifiers.contains(KeyModifiers::CONTROL) && c.is_ascii_alphabetic() =>
        {
            vec![(c.to_ascii_uppercase() as u8) & 0x1f]
        }
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Esc => b"\x1b".to_vec(),
        KeyCode::Up => if application_cursor {
            b"\x1bOA"
        } else {
            b"\x1b[A"
        }
        .to_vec(),
        KeyCode::Down => if application_cursor {
            b"\x1bOB"
        } else {
            b"\x1b[B"
        }
        .to_vec(),
        KeyCode::Right => if application_cursor {
            b"\x1bOC"
        } else {
            b"\x1b[C"
        }
        .to_vec(),
        KeyCode::Left => if application_cursor {
            b"\x1bOD"
        } else {
            b"\x1b[D"
        }
        .to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::F(n) if (1..=4).contains(&n) => {
            format!("\x1bO{}", ["P", "Q", "R", "S"][(n - 1) as usize]).into_bytes()
        }
        KeyCode::F(n) if (5..=12).contains(&n) => format!(
            "\x1b[{}~",
            [15, 17, 18, 19, 20, 21, 23, 24][(n - 5) as usize]
        )
        .into_bytes(),
        _ => return None,
    };
    if key.modifiers.contains(KeyModifiers::ALT) {
        let mut result = vec![0x1b];
        result.extend(bytes);
        Some(result)
    } else {
        Some(bytes)
    }
}

/// Feeds native output to a private screen model and answers terminal status
/// queries, so Claude Code behaves as in a real terminal without being shown.
fn feed_native(
    parser: &mut vt100::Parser,
    query_tail: &mut Vec<u8>,
    bytes: &[u8],
    pty: &mut Pty,
) -> Result<(), String> {
    parser.process(bytes);
    let old_len = query_tail.len();
    query_tail.extend_from_slice(bytes);
    for (pattern, reply) in [
        (b"\x1b[6n".as_slice(), None),
        (b"\x1b[5n".as_slice(), Some(b"\x1b[0n".as_slice())),
    ] {
        for pos in 0..query_tail.len().saturating_sub(pattern.len() - 1) {
            if pos + pattern.len() > old_len && query_tail[pos..].starts_with(pattern) {
                if let Some(reply) = reply {
                    pty.write(reply)?;
                } else {
                    let (row, col) = parser.screen().cursor_position();
                    pty.write(format!("\x1b[{};{}R", row + 1, col + 1).as_bytes())?;
                }
            }
        }
    }
    let keep = query_tail.len().min(3);
    query_tail.drain(..query_tail.len() - keep);
    Ok(())
}

pub fn run(
    project: &Path,
    root: &Path,
    resume: bool,
    mode: Mode,
    no_color: bool,
) -> Result<(Uuid, u32), String> {
    let (store, mut state) = ClaudeStore::open(root, project)?;
    let id = if resume {
        state
            .latest()
            .ok_or("Nenhuma sessão Claude anterior nesta pasta")?
    } else {
        Uuid::new_v4()
    };
    let _guard = TerminalGuard::enter()?;
    let mut terminal =
        Terminal::new(CrosstermBackend::new(io::stdout())).map_err(|e| e.to_string())?;
    let size = terminal.size().map_err(|e| e.to_string())?;
    if size.width < 30 || size.height < 8 {
        return Err("Terminal Claude exige pelo menos 30×8".into());
    }
    let (rows, cols) = viewport(size.into());
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut pty = Pty::start(
        OsStr::new("claude"),
        project,
        id,
        resume,
        pty_size(rows, cols),
        no_color,
        None,
    )?;
    if !resume {
        store.start(&mut state, id)?;
    }
    let mut help = false;
    let mut dirty = true;
    let mut closed = false;
    let mut status = None;
    let mut exited_at = None;
    let mut query_tail = Vec::new();
    loop {
        while let Ok(message) = pty.output.try_recv() {
            match message {
                Output::Bytes(bytes) => {
                    feed_native(&mut parser, &mut query_tail, &bytes, &mut pty)?;
                    dirty = true;
                }
                Output::Closed => closed = true,
            }
        }
        if dirty {
            terminal
                .draw(|f| draw(f, parser.screen(), id, help, no_color, mode))
                .map_err(|e| e.to_string())?;
            dirty = false;
        }
        if status.is_none()
            && let Some(exit) = pty.try_wait()?
        {
            exited_at = Some(Instant::now());
            status = Some(exit.exit_code());
        }
        if status.is_some()
            && (closed || exited_at.is_some_and(|t| t.elapsed() > Duration::from_secs(1)))
        {
            break;
        }
        if event::poll(Duration::from_millis(40)).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    if key.code == KeyCode::Char('g')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        help = !help;
                        dirty = true;
                    } else if help {
                        if key.code == KeyCode::Esc {
                            help = false;
                            dirty = true;
                        }
                    } else if let Some(bytes) = key_bytes(key, parser.screen().application_cursor())
                    {
                        pty.write(&bytes)?;
                    }
                }
                Event::Paste(text) if !help => {
                    if parser.screen().bracketed_paste() {
                        pty.write(b"\x1b[200~")?;
                    }
                    pty.write(text.as_bytes())?;
                    if parser.screen().bracketed_paste() {
                        pty.write(b"\x1b[201~")?;
                    }
                }
                Event::Resize(width, height) => {
                    let (rows, cols) = viewport(Rect::new(0, 0, width, height));
                    pty.resize(rows, cols)?;
                    parser.screen_mut().set_size(rows, cols);
                    terminal.clear().map_err(|e| e.to_string())?;
                    dirty = true;
                }
                _ => {}
            }
        }
    }
    let code = status.unwrap_or(1);
    store.finish(&mut state, id, code)?;
    Ok((id, code))
}

/// The hook denies a request left unanswered this long.
#[cfg(unix)]
const PERMISSION_TIMEOUT: Duration = Duration::from_secs(90);
/// The Bee closes the review a little earlier, so a late `y` never reaches
/// a hook that has already denied.
#[cfg(unix)]
const PERMISSION_REVIEW: Duration = Duration::from_secs(85);

#[cfg(unix)]
struct BridgeEvent {
    value: serde_json::Value,
    reply: Option<mpsc::Sender<bool>>,
}

#[cfg(unix)]
struct Bridge {
    directory: std::path::PathBuf,
    path: std::path::PathBuf,
    stop: Arc<AtomicBool>,
    events: Receiver<BridgeEvent>,
}

#[cfg(unix)]
impl Bridge {
    fn start() -> Result<Self, String> {
        // macOS limits Unix socket paths to roughly 100 bytes.
        let directory = Path::new("/tmp").join(format!("bee-{}", Uuid::new_v4()));
        let mut builder = std::fs::DirBuilder::new();
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
        builder.create(&directory).map_err(|e| e.to_string())?;
        let path = directory.join("hook.sock");
        let listener = UnixListener::bind(&path).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let (tx, events) = mpsc::channel();
        thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let tx = tx.clone();
                        thread::spawn(move || handle_hook_connection(stream, tx));
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            directory,
            path,
            stop,
            events,
        })
    }
}

#[cfg(unix)]
impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_dir(&self.directory);
    }
}

#[cfg(unix)]
fn deny_json() -> String {
    serde_json::json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Memory Bee não recebeu aprovação explícita"}}}).to_string()
}

#[cfg(unix)]
fn handle_hook_connection(mut stream: UnixStream, tx: mpsc::Sender<BridgeEvent>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut bytes = Vec::new();
    let read = io::BufReader::new(&stream)
        .take(256 * 1024 + 1)
        .read_until(b'\n', &mut bytes);
    if read.is_err() || bytes.len() > 256 * 1024 {
        return;
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return;
    };
    let permission = value["hook_event_name"] == "PermissionRequest";
    if permission {
        let (reply_tx, reply_rx) = mpsc::channel();
        if tx
            .send(BridgeEvent {
                value,
                reply: Some(reply_tx),
            })
            .is_err()
        {
            return;
        }
        let allowed = reply_rx.recv_timeout(PERMISSION_TIMEOUT).unwrap_or(false);
        let decision = if allowed {
            serde_json::json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}).to_string()
        } else {
            deny_json()
        };
        let _ = stream.write_all(decision.as_bytes());
        let _ = stream.write_all(b"\n");
    } else if tx.send(BridgeEvent { value, reply: None }).is_ok() {
        // Acknowledge only once queued, so events from a hook that ran just
        // before Claude exits are still shown.
        let _ = stream.write_all(b"\n");
    }
}

/// Entry point invoked by Claude Code's command hooks. Never prints hook input.
#[cfg(unix)]
pub fn hook_main() -> u8 {
    let mut bytes = Vec::new();
    if io::stdin()
        .take(256 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > 256 * 1024
    {
        println!("{}", deny_json());
        return 0;
    }
    let permission = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .is_some_and(|v| v["hook_event_name"] == "PermissionRequest");
    let response = std::env::var_os("MEMORY_BEE_CLAUDE_SOCKET")
        .map(std::path::PathBuf::from)
        .and_then(|path| UnixStream::connect(path).ok())
        .and_then(|mut stream| {
            stream
                .set_read_timeout(Some(if permission {
                    PERMISSION_TIMEOUT + Duration::from_secs(5)
                } else {
                    Duration::from_secs(5)
                }))
                .ok()?;
            stream.write_all(&bytes).ok()?;
            stream.write_all(b"\n").ok()?;
            let mut reply = String::new();
            io::BufReader::new(stream).read_line(&mut reply).ok()?;
            if permission {
                serde_json::from_str::<serde_json::Value>(&reply).ok()?;
                Some(reply)
            } else {
                Some(String::new())
            }
        });
    if permission {
        print!("{}", response.unwrap_or_else(deny_json));
    }
    0
}

#[cfg(unix)]
fn hook_settings(executable: &Path) -> String {
    let hook = serde_json::json!({"type":"command","command":executable,"args":["__claude-hook"],"timeout":120});
    let mut hooks = serde_json::Map::new();
    for event in [
        "SessionStart",
        "UserPromptSubmit",
        "MessageDisplay",
        "PreToolUse",
        "PostToolUse",
        "PostToolUseFailure",
        "PermissionRequest",
        "Stop",
        "StopFailure",
    ] {
        hooks.insert(event.into(), serde_json::json!([{"hooks":[hook.clone()]}]));
    }
    serde_json::json!({"hooks":hooks}).to_string()
}

/// Without a first hook in this window, Claude is waiting on a native screen.
#[cfg(unix)]
const SETUP_STALL: Duration = Duration::from_secs(5);
/// A turn without hooks for this long may be thinking or on a native screen.
#[cfg(unix)]
const TURN_STALL: Duration = Duration::from_secs(20);

/// Names the native screen without copying its text into the Bee view.
#[cfg(unix)]
fn native_hint(screen: &str) -> &'static str {
    let text = screen.to_lowercase();
    if text.contains("trust") {
        "confiança do projeto"
    } else if ["login", "log in", "sign in", "api key", "authenticat"]
        .iter()
        .any(|word| text.contains(word))
    {
        "login ou autenticação"
    } else if text.contains("text style") || text.contains("theme") {
        "configuração inicial"
    } else {
        "tela nativa sem evento"
    }
}

#[cfg(unix)]
struct Pending {
    request: String,
    reply: mpsc::Sender<bool>,
    /// `ToolCall::seq`, stable while older calls are dropped.
    tool: Option<u64>,
    since: Instant,
}

#[cfg(unix)]
struct ToolCall {
    seq: u64,
    id: Option<String>,
    name: String,
    input: serde_json::Value,
    /// `None`: Claude never asked; `Some(false)`: asked and not approved.
    approved: Option<bool>,
    /// Seen as `PreToolUse`; false when the request arrived first.
    pre: bool,
    /// A request without `tool_use_id` matched several identical calls, so
    /// the Bee cannot tell which one it answered.
    ambiguous: bool,
}

/// One-line hint of what a tool touches; the review panel shows everything.
#[cfg(unix)]
fn tool_summary(input: &serde_json::Value) -> String {
    let value = [
        "command",
        "file_path",
        "path",
        "pattern",
        "url",
        "description",
    ]
    .iter()
    .find_map(|key| input[*key].as_str())
    .unwrap_or("");
    let line: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(72)
        .collect();
    if line.is_empty() {
        String::new()
    } else if value.chars().count() > 72 {
        format!(" · {line}…")
    } else {
        format!(" · {line}")
    }
}

#[cfg(unix)]
struct HiddenView {
    lines: Vec<String>,
    input: String,
    pending: Option<Pending>,
    /// Recent tool calls, to tell Bee approvals from Claude's own rules.
    tools: Vec<ToolCall>,
    next_tool: u64,
    ready: bool,
    quitting: bool,
    /// Explicitly opened original screen, only to finish setup or diagnose.
    native: bool,
    /// Native screen that blocks startup and emits no hook.
    blocked: Option<&'static str>,
    started: Instant,
    /// Last progress of a turn in flight; `None` between turns.
    turn: Option<Instant>,
    turn_hinted: bool,
    exit_after_turn: bool,
}

#[cfg(unix)]
impl HiddenView {
    fn new(now: Instant) -> Self {
        Self {
            lines: vec!["Iniciando Claude Code em segundo plano…".into()],
            input: String::new(),
            pending: None,
            tools: Vec::new(),
            next_tool: 0,
            ready: false,
            quitting: false,
            native: false,
            blocked: None,
            started: now,
            turn: None,
            turn_hinted: false,
            exit_after_turn: false,
        }
    }
    fn push(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
        if self.lines.len() > 2000 {
            self.lines.drain(..1000);
        }
    }
    /// Detects missing progress. Returns true when the view changed.
    fn check_stall(&mut self, now: Instant, screen: impl FnOnce() -> String) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| now.duration_since(p.since) >= PERMISSION_REVIEW)
        {
            self.decide(false, true);
            return true;
        }
        if !self.ready && !self.quitting && now.duration_since(self.started) >= SETUP_STALL {
            let hint = native_hint(&screen());
            if self.blocked == Some(hint) {
                return false;
            }
            if self.blocked.is_none() {
                self.push(format!(
                    "Claude aguarda {hint} no terminal original, sem evento para a Bee. \
                     Ctrl+O abre o Claude original para concluir; Ctrl+Q encerra sem responder."
                ));
            }
            self.blocked = Some(hint);
            return true;
        }
        if self.ready
            && !self.turn_hinted
            && self.pending.is_none()
            && self
                .turn
                .is_some_and(|at| now.duration_since(at) >= TURN_STALL)
        {
            self.turn_hinted = true;
            self.push(
                "Sem eventos do Claude há 20 s: pode estar pensando ou numa tela nativa. \
                 Ctrl+O mostra o terminal original; Ctrl+C interrompe.",
            );
            return true;
        }
        false
    }
    fn submitted(&mut self, now: Instant) {
        self.turn = Some(now);
        self.turn_hinted = false;
    }
    fn add_tool(&mut self, value: &serde_json::Value, pre: bool) -> u64 {
        if self.tools.len() >= 64 {
            self.tools.remove(0);
        }
        self.next_tool += 1;
        self.tools.push(ToolCall {
            seq: self.next_tool,
            id: value["tool_use_id"].as_str().map(String::from),
            name: value["tool_name"].as_str().unwrap_or("desconhecida").into(),
            input: value["tool_input"].clone(),
            approved: None,
            pre,
            ambiguous: false,
        });
        self.next_tool
    }
    /// Call answered by a `PermissionRequest`. The official payload carries
    /// no `tool_use_id`, so identical unanswered calls make it ambiguous;
    /// those are marked instead of guessing. Returns `None` in that case.
    fn request_target(&mut self, value: &serde_json::Value) -> Option<u64> {
        if value["tool_use_id"].is_string()
            && let Some(i) = self.find_tool(value)
        {
            return Some(self.tools[i].seq);
        }
        let name = value["tool_name"].as_str().unwrap_or("desconhecida");
        let candidates: Vec<usize> = (0..self.tools.len())
            .filter(|&i| {
                let t = &self.tools[i];
                t.approved.is_none()
                    && !t.ambiguous
                    && t.name == name
                    && t.input == value["tool_input"]
            })
            .collect();
        match candidates[..] {
            [] => Some(self.add_tool(value, false)),
            [i] => Some(self.tools[i].seq),
            _ => {
                for i in candidates {
                    self.tools[i].ambiguous = true;
                }
                None
            }
        }
    }
    /// Matches a hook to its `PreToolUse` by ID, else by name and input.
    fn find_tool(&self, value: &serde_json::Value) -> Option<usize> {
        if let Some(id) = value["tool_use_id"].as_str()
            && let Some(i) = self.tools.iter().rposition(|t| t.id.as_deref() == Some(id))
        {
            return Some(i);
        }
        let name = value["tool_name"].as_str()?;
        self.tools
            .iter()
            .rposition(|t| t.name == name && t.input == value["tool_input"])
    }
    /// Answers the pending request. The message states only what the hook
    /// actually received: an expired request was denied, never approved.
    fn decide(&mut self, allow: bool, expired: bool) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        let delivered = !expired && pending.reply.send(allow).is_ok();
        let approved = allow && delivered;
        if let Some(call) = self.tools.iter_mut().find(|t| Some(t.seq) == pending.tool) {
            call.approved = Some(approved);
        }
        self.push(if !delivered {
            "Pedido expirou sem resposta e foi negado; nada foi aprovado."
        } else if approved {
            "Você permitiu esta ação uma vez."
        } else {
            "Você negou esta ação."
        });
    }
    fn receive(&mut self, event: BridgeEvent, id: Uuid, now: Instant) {
        if event.value["session_id"].as_str() != Some(&id.to_string()) {
            if let Some(reply) = event.reply {
                let _ = reply.send(false);
            }
            return;
        }
        if self.turn.is_some() {
            self.turn = Some(now);
            self.turn_hinted = false;
        }
        match event.value["hook_event_name"].as_str().unwrap_or("") {
            "SessionStart" => {
                self.ready = true;
                if self.blocked.take().is_some() {
                    self.push("Preparação nativa concluída. Ctrl+O volta à Bee se necessário.");
                }
                self.push("Claude pronto. Digite sua mensagem abaixo.");
            }
            "MessageDisplay" => {
                if let Some(delta) = event.value["delta"].as_str() {
                    for line in delta.lines() {
                        self.push(format!("Claude: {line}"));
                    }
                }
            }
            "PreToolUse" => {
                let name = event.value["tool_name"].as_str().unwrap_or("desconhecida");
                self.push(format!(
                    "Ação: {name}{}",
                    tool_summary(&event.value["tool_input"])
                ));
                // Hooks use separate connections; a request may come first.
                if let Some(call) = self
                    .tools
                    .iter_mut()
                    .rev()
                    .find(|t| !t.pre && t.name == name && t.input == event.value["tool_input"])
                {
                    call.pre = true;
                    call.id = event.value["tool_use_id"].as_str().map(String::from);
                } else {
                    self.add_tool(&event.value, true);
                }
            }
            name @ ("PostToolUse" | "PostToolUseFailure") => {
                let tool = event.value["tool_name"].as_str().unwrap_or("ação");
                let origin = match self.find_tool(&event.value).map(|i| self.tools.remove(i)) {
                    Some(ToolCall {
                        ambiguous: true, ..
                    }) => {
                        "origem incerta: houve um pedido para uma chamada idêntica e não é possível dizer qual foi aprovada"
                    }
                    Some(ToolCall {
                        approved: Some(true),
                        ..
                    }) => "aprovada por você na Bee, uma vez",
                    Some(ToolCall {
                        approved: Some(false),
                        ..
                    }) => "atenção: o pedido não foi aprovado na Bee",
                    _ => "sem pedido de permissão; liberada pelas regras ou modo do próprio Claude",
                };
                let verb = if name == "PostToolUse" {
                    "Concluído"
                } else {
                    "Falhou"
                };
                self.push(format!("{verb}: {tool} ({origin})"));
            }
            "PermissionRequest" => {
                let name = event.value["tool_name"].as_str().unwrap_or("ação");
                let details = serde_json::to_string_pretty(&event.value["tool_input"])
                    .unwrap_or_else(|_| "<entrada inválida>".into());
                let request = format!("Ferramenta: {name}\nEntrada completa:\n{details}");
                self.push(format!("Claude pediu permissão: {name}"));
                let seq = self.request_target(&event.value);
                if let Some(reply) = event.reply {
                    if self.pending.is_some() {
                        let _ = reply.send(false);
                        self.push(format!(
                            "Negado: {name} (outro pedido já aguardava revisão)"
                        ));
                        if let Some(call) = self.tools.iter_mut().find(|t| Some(t.seq) == seq) {
                            call.approved = Some(false);
                        }
                    } else {
                        // Decisions are always reviewed in the Bee panel.
                        self.native = false;
                        self.pending = Some(Pending {
                            request,
                            reply,
                            tool: seq,
                            since: now,
                        });
                    }
                }
            }
            "Stop" => {
                self.turn = None;
                self.push("Turno concluído.");
            }
            "StopFailure" => {
                self.turn = None;
                self.push("Claude encerrou o turno com erro.");
            }
            _ => {}
        }
    }
    /// `/exit` is typed only at Claude's idle prompt; a native dialog could
    /// take its Enter as confirmation.
    fn exit_by_command(&self) -> bool {
        self.ready && self.blocked.is_none() && self.turn.is_none() && !self.native
    }
}

#[cfg(unix)]
fn approval_fits(request: &str, width: u16, height: u16) -> bool {
    if width < 40 || height < 12 || request.chars().count() > 2048 {
        return false;
    }
    let text_width = width.saturating_sub(8).max(1) as usize;
    let text_rows = height.saturating_sub(10) as usize;
    request
        .lines()
        .map(|line| UnicodeWidthStr::width(line).max(1).div_ceil(text_width))
        .sum::<usize>()
        <= text_rows
}

#[cfg(unix)]
fn draw_hidden(frame: &mut Frame, view: &HiddenView, mode: Mode, no_color: bool) {
    let area = frame.area();
    let palette = Palette::new(mode, no_color);
    let bg = if mode == Mode::Dark {
        Color::Black
    } else {
        Color::White
    };
    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(Block::default().style(Style::default().bg(bg)), area);
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new("⬢ Memory Bee · Claude em segundo plano").style(
            Style::default()
                .fg(palette.accent)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        rows[0],
    );
    let visible = rows[1].height.saturating_sub(2) as usize;
    let start = view.lines.len().saturating_sub(visible);
    frame.render_widget(
        Paragraph::new(view.lines[start..].join("\n"))
            .block(
                Block::default()
                    .title(" Conversa ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.accent)),
            )
            .style(Style::default().bg(bg)),
        rows[1],
    );
    let prompt = if view.pending.is_some() {
        "Permissão pendente · veja o painel de revisão".into()
    } else if let Some(hint) = view.blocked {
        format!("Claude aguarda {hint} · Ctrl+O concluir no original · Ctrl+Q sair")
    } else if !view.ready {
        "Aguardando Claude iniciar…".into()
    } else {
        format!("> {}", view.input)
    };
    frame.render_widget(
        Paragraph::new(prompt)
            .block(
                Block::default()
                    .title(" Mensagem / decisão ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.accent)),
            )
            .style(Style::default().bg(bg)),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new(
            "Enter envia · Ctrl+C interrompe · Ctrl+O Claude original · Ctrl+Q sai · sem aprovação Bee, negar",
        )
            .style(Style::default().bg(bg)),
        rows[3],
    );
    if let Some(Pending { request, .. }) = &view.pending {
        let modal = Rect::new(
            2,
            2,
            area.width.saturating_sub(4),
            area.height.saturating_sub(4),
        );
        frame.render_widget(ratatui::widgets::Clear, modal);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(" Revisar permissão Claude ")
                .style(Style::default().bg(bg))
                .border_style(Style::default().fg(palette.accent)),
            modal,
        );
        let inner = Rect::new(
            modal.x + 2,
            modal.y + 1,
            modal.width.saturating_sub(4),
            modal.height.saturating_sub(3),
        );
        frame.render_widget(
            Paragraph::new(request.as_str())
                .wrap(ratatui::widgets::Wrap { trim: false })
                .style(Style::default().bg(bg)),
            inner,
        );
        let action = if approval_fits(request, area.width, area.height) {
            "[y] Permitir esta ação uma vez  ·  [n] Negar (padrão)"
        } else {
            "Entrada não cabe para revisão: [n] Negar; y também nega"
        };
        frame.render_widget(
            Paragraph::new(action).style(Style::default().bg(bg)),
            Rect::new(
                modal.x + 2,
                modal.bottom().saturating_sub(2),
                modal.width.saturating_sub(4),
                1,
            ),
        );
    }
}

/// Original Claude screen, opened only by Ctrl+O to finish setup or diagnose.
#[cfg(unix)]
fn draw_native_setup(
    frame: &mut Frame,
    screen: &vt100::Screen,
    view: &HiddenView,
    mode: Mode,
    no_color: bool,
) {
    let area = frame.area();
    let p = Palette::new(mode, no_color);
    frame.render_widget(ratatui::widgets::Clear, area);
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new("⬢ Memory Bee · Claude original aberto por Ctrl+O")
            .style(Style::default().fg(p.accent).add_modifier(Modifier::BOLD)),
        rows[0],
    );
    let block = Block::default()
        .title(" Claude Code original · preparação/diagnóstico ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(rows[1]);
    frame.render_widget(block, rows[1]);
    paint_screen(frame, inner, screen, no_color);
    let (cursor_row, cursor_col) = screen.cursor_position();
    if cursor_row < inner.height && cursor_col < inner.width {
        frame.set_cursor_position((inner.x + cursor_col, inner.y + cursor_row));
    }
    let footer = if view.ready {
        "Claude pronto · Ctrl+O volta à Bee · Ctrl+Q encerra"
    } else {
        "Teclas vão ao Claude · Ctrl+O volta à Bee · Ctrl+Q encerra sem responder"
    };
    frame.render_widget(Paragraph::new(footer), rows[2]);
}

/// First native Bee conversation surface. Claude's own terminal stays private.
#[cfg(unix)]
pub fn run_hidden(
    project: &Path,
    root: &Path,
    resume: bool,
    mode: Mode,
    no_color: bool,
) -> Result<(Uuid, u32), String> {
    let (store, mut state) = ClaudeStore::open(root, project)?;
    let id = if resume {
        state
            .latest()
            .ok_or("Nenhuma sessão Claude anterior nesta pasta")?
    } else {
        Uuid::new_v4()
    };
    let bridge = Bridge::start()?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let settings = hook_settings(&executable);
    let _guard = TerminalGuard::enter()?;
    let mut terminal =
        Terminal::new(CrosstermBackend::new(io::stdout())).map_err(|e| e.to_string())?;
    let size = terminal.size().map_err(|e| e.to_string())?;
    if size.width < 40 || size.height < 10 {
        return Err("Terminal exige pelo menos 40×10".into());
    }
    // Claude draws its original screen inside the frame opened by Ctrl+O.
    let (rows, cols) = viewport(size.into());
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut query_tail = Vec::new();
    let mut pty = Pty::start(
        OsStr::new("claude"),
        project,
        id,
        resume,
        pty_size(rows, cols),
        no_color,
        Some((&bridge.path, &settings)),
    )?;
    if !resume {
        store.start(&mut state, id)?;
    }
    let mut view = HiddenView::new(Instant::now());
    let mut status = None;
    let mut quit_at = None;
    let mut dirty = true;
    loop {
        while let Ok(event) = bridge.events.try_recv() {
            view.receive(event, id, Instant::now());
            dirty = true;
        }
        if view.exit_after_turn && view.turn.is_none() {
            view.exit_after_turn = false;
            // Claude may already be gone; the exit status decides below.
            let _ = pty.write(b"/exit\r");
        }
        // Native bytes feed a private screen model; it is shown only on Ctrl+O.
        while let Ok(Output::Bytes(bytes)) = pty.output.try_recv() {
            feed_native(&mut parser, &mut query_tail, &bytes, &mut pty)?;
            dirty |= view.native;
        }
        dirty |= view.check_stall(Instant::now(), || parser.screen().contents());
        if dirty {
            terminal
                .draw(|f| {
                    if view.native {
                        draw_native_setup(f, parser.screen(), &view, mode, no_color)
                    } else {
                        draw_hidden(f, &view, mode, no_color)
                    }
                })
                .map_err(|e| e.to_string())?;
            dirty = false;
        }
        if status.is_none()
            && let Some(exit) = pty.try_wait()?
        {
            status = Some(exit.exit_code());
        }
        if status.is_none()
            && quit_at.is_some_and(|at: Instant| at.elapsed() > Duration::from_secs(5))
        {
            pty.child.kill().map_err(|e| e.to_string())?;
            let exit = pty.child.wait().map_err(|e| e.to_string())?;
            pty.exited = true;
            status = Some(exit.exit_code());
        }
        if status.is_some() {
            // Hooks are acknowledged once queued; show the last ones.
            while let Ok(event) = bridge.events.try_recv() {
                view.receive(event, id, Instant::now());
            }
            terminal
                .draw(|f| draw_hidden(f, &view, mode, no_color))
                .map_err(|e| e.to_string())?;
            break;
        }
        if event::poll(Duration::from_millis(40)).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    let control = key.modifiers.contains(KeyModifiers::CONTROL);
                    if control && key.code == KeyCode::Char('q') {
                        if view.pending.is_some() {
                            view.decide(false, false);
                            view.push("Negada ao sair.");
                        }
                        if view.quitting {
                            // Already leaving; the 5 s fallback still applies.
                        } else if !view.ready {
                            // Nothing is typed into a native setup dialog.
                            pty.child.kill().map_err(|e| e.to_string())?;
                            let exit = pty.child.wait().map_err(|e| e.to_string())?;
                            pty.exited = true;
                            status = Some(exit.exit_code());
                        } else if view.exit_by_command() {
                            pty.write(b"/exit\r")?;
                            quit_at = Some(Instant::now());
                            view.push("Encerrando Claude…");
                        } else {
                            // Enter could confirm a native dialog; interrupt
                            // and type /exit only after the turn stops.
                            pty.write(&[3])?;
                            // Without a turn, the 5 s fallback ends the process.
                            view.exit_after_turn = view.turn.is_some();
                            quit_at = Some(Instant::now());
                            view.push("Interrompendo Claude antes de sair…");
                        }
                        view.quitting = true;
                        view.native = false;
                    } else if control && key.code == KeyCode::Char('o') && view.pending.is_none() {
                        view.native = !view.native;
                    } else if view.native {
                        if let Some(bytes) = key_bytes(key, parser.screen().application_cursor()) {
                            pty.write(&bytes)?;
                        }
                    } else if control && key.code == KeyCode::Char('c') {
                        if view.pending.is_some() {
                            view.decide(false, false);
                            view.push("Negada ao interromper.");
                        }
                        pty.write(&[3])?;
                        view.push("Interrupção enviada ao Claude.");
                    } else if view.pending.is_some()
                        && matches!(key.code, KeyCode::Char('y' | 'n') | KeyCode::Esc)
                        && !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    {
                        let size = terminal.size().map_err(|e| e.to_string())?;
                        let fits = view
                            .pending
                            .as_ref()
                            .is_some_and(|p| approval_fits(&p.request, size.width, size.height));
                        view.decide(key.code == KeyCode::Char('y') && fits, false);
                    } else if view.pending.is_none() && view.ready && !view.quitting {
                        match key.code {
                            KeyCode::Char(c) if !control => view.input.push(c),
                            KeyCode::Backspace => {
                                view.input.pop();
                            }
                            KeyCode::Enter if !view.input.trim().is_empty() => {
                                let prompt = std::mem::take(&mut view.input);
                                pty.write(prompt.as_bytes())?;
                                pty.write(b"\r")?;
                                view.submitted(Instant::now());
                                view.push(format!("Você: {prompt}"));
                            }
                            _ => {}
                        }
                    }
                    dirty = true;
                }
                Event::Paste(text) if view.native => {
                    if parser.screen().bracketed_paste() {
                        pty.write(b"\x1b[200~")?;
                    }
                    pty.write(text.as_bytes())?;
                    if parser.screen().bracketed_paste() {
                        pty.write(b"\x1b[201~")?;
                    }
                }
                Event::Paste(paste) if view.ready && view.pending.is_none() => {
                    view.input.push_str(&paste.replace(['\r', '\n'], " "));
                    dirty = true;
                }
                Event::Resize(width, height) => {
                    let (rows, cols) = viewport(Rect::new(0, 0, width, height));
                    pty.resize(rows, cols)?;
                    parser.screen_mut().set_size(rows, cols);
                    terminal.clear().map_err(|e| e.to_string())?;
                    dirty = true;
                }
                _ => {}
            }
        }
    }
    view.decide(false, false);
    let code = status.unwrap_or(1);
    store.finish(&mut state, id, code)?;
    Ok((id, code))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_reach_native_terminal_without_bee_shortcuts() {
        use crossterm::event::KeyEvent;
        assert_eq!(
            key_bytes(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                false
            ),
            Some(vec![3])
        );
        assert_eq!(
            key_bytes(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), false),
            Some(vec![13])
        );
        assert_eq!(
            key_bytes(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), false),
            Some(b"\x1b[A".to_vec())
        );
    }
    #[cfg(unix)]
    #[test]
    fn approval_requires_the_full_input_to_fit_on_screen() {
        assert!(approval_fits(
            "Ferramenta: Bash\nEntrada completa:\n{\"command\":\"echo ok\"}",
            80,
            24
        ));
        assert!(!approval_fits(&"x".repeat(3000), 80, 24));
        assert!(!approval_fits(
            "Ferramenta: Bash\nEntrada completa:\n{\"command\":\"echo ok\"}",
            30,
            10
        ));
    }
    #[cfg(unix)]
    #[test]
    fn native_screens_are_named_without_copying_their_text() {
        assert_eq!(
            native_hint("Do you trust the files in this folder?"),
            "confiança do projeto"
        );
        assert_eq!(
            native_hint("Select login method: token abc123"),
            "login ou autenticação"
        );
        assert_eq!(
            native_hint("Choose the text style ... /theme"),
            "configuração inicial"
        );
        assert_eq!(native_hint("???"), "tela nativa sem evento");
        let mut view = HiddenView::new(Instant::now());
        view.check_stall(Instant::now() + SETUP_STALL, || "login token abc123".into());
        assert!(view.lines.iter().all(|line| !line.contains("abc123")));
    }
    #[cfg(unix)]
    #[test]
    fn missing_hooks_block_startup_and_hint_during_turns() {
        let start = Instant::now();
        let mut view = HiddenView::new(start);
        assert!(!view.check_stall(start + Duration::from_secs(1), || "trust".into()));
        assert!(view.check_stall(start + SETUP_STALL, || "trust".into()));
        assert_eq!(view.blocked, Some("confiança do projeto"));
        assert!(!view.check_stall(start + SETUP_STALL, || "trust".into()));
        assert!(!view.exit_by_command());
        let id = Uuid::new_v4();
        let hook = |name: &str| BridgeEvent {
            value: serde_json::json!({"hook_event_name": name, "session_id": id.to_string()}),
            reply: None,
        };
        view.receive(hook("SessionStart"), id, start);
        assert!(view.ready && view.blocked.is_none() && view.exit_by_command());
        view.submitted(start);
        assert!(!view.exit_by_command());
        assert!(!view.check_stall(start + Duration::from_secs(19), String::new));
        view.receive(hook("PreToolUse"), id, start + Duration::from_secs(19));
        assert!(!view.check_stall(start + Duration::from_secs(30), String::new));
        assert!(view.check_stall(start + Duration::from_secs(40), String::new));
        assert!(!view.check_stall(start + Duration::from_secs(60), String::new));
        view.receive(hook("Stop"), id, start + Duration::from_secs(61));
        assert!(view.turn.is_none() && view.exit_by_command());
    }
    #[cfg(unix)]
    #[test]
    fn tool_results_state_who_allowed_them() {
        let start = Instant::now();
        let id = Uuid::new_v4();
        let mut view = HiddenView::new(start);
        let event = |fields: serde_json::Value| {
            let mut value = fields;
            value["session_id"] = id.to_string().into();
            BridgeEvent { value, reply: None }
        };
        let bash = serde_json::json!({"command": "touch ok"});
        view.receive(event(serde_json::json!({"hook_event_name": "PreToolUse", "tool_name": "Read", "tool_use_id": "r1", "tool_input": {"file_path": "a.txt"}})), id, start);
        view.receive(event(serde_json::json!({"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_use_id": "b1", "tool_input": bash})), id, start);
        let (tx, rx) = mpsc::channel();
        let mut request = event(
            serde_json::json!({"hook_event_name": "PermissionRequest", "tool_name": "Bash", "tool_input": bash}),
        );
        request.reply = Some(tx);
        view.receive(request, id, start);
        // An older call finishing must not disturb the pending review.
        view.receive(event(serde_json::json!({"hook_event_name": "PostToolUse", "tool_name": "Read", "tool_use_id": "r1"})), id, start);
        assert!(
            view.lines
                .last()
                .unwrap()
                .contains("sem pedido de permissão")
        );
        view.decide(true, false);
        assert!(rx.recv().unwrap());
        view.receive(event(serde_json::json!({"hook_event_name": "PostToolUse", "tool_name": "Bash", "tool_use_id": "b1"})), id, start);
        assert!(view.lines.last().unwrap().contains("aprovada por você"));
        assert!(view.lines.iter().any(|l| l == "Ação: Bash · touch ok"));
    }
    #[cfg(unix)]
    #[test]
    fn approval_links_even_when_the_request_arrives_first() {
        let start = Instant::now();
        let id = Uuid::new_v4();
        let mut view = HiddenView::new(start);
        let input = serde_json::json!({"command": "touch ok"});
        let (tx, rx) = mpsc::channel();
        view.receive(BridgeEvent { value: serde_json::json!({"hook_event_name": "PermissionRequest", "session_id": id.to_string(), "tool_name": "Bash", "tool_input": input}), reply: Some(tx) }, id, start);
        view.decide(true, false);
        assert!(rx.recv().unwrap());
        for name in ["PreToolUse", "PostToolUse"] {
            view.receive(BridgeEvent { value: serde_json::json!({"hook_event_name": name, "session_id": id.to_string(), "tool_name": "Bash", "tool_use_id": "b1", "tool_input": input}), reply: None }, id, start);
        }
        assert!(view.lines.last().unwrap().contains("aprovada por você"));
        assert!(view.tools.is_empty());
    }
    #[cfg(unix)]
    #[test]
    fn identical_parallel_calls_never_claim_a_specific_approval() {
        let start = Instant::now();
        let id = Uuid::new_v4();
        let mut view = HiddenView::new(start);
        let input = serde_json::json!({"command": "make"});
        let hook = |name: &str, tool_id: Option<&str>| {
            let mut value = serde_json::json!({"hook_event_name": name, "session_id": id.to_string(), "tool_name": "Bash", "tool_input": input});
            if let Some(tool_id) = tool_id {
                value["tool_use_id"] = tool_id.into();
            }
            value
        };
        for tool_id in ["b1", "b2"] {
            view.receive(
                BridgeEvent {
                    value: hook("PreToolUse", Some(tool_id)),
                    reply: None,
                },
                id,
                start,
            );
        }
        let (tx, rx) = mpsc::channel();
        view.receive(
            BridgeEvent {
                value: hook("PermissionRequest", None),
                reply: Some(tx),
            },
            id,
            start,
        );
        view.decide(true, false);
        assert!(rx.recv().unwrap());
        for tool_id in ["b1", "b2"] {
            view.receive(
                BridgeEvent {
                    value: hook("PostToolUse", Some(tool_id)),
                    reply: None,
                },
                id,
                start,
            );
            let line = view.lines.last().unwrap();
            assert!(line.contains("origem incerta"), "{line}");
            assert!(!line.contains("aprovada por você"));
        }
    }
    #[cfg(unix)]
    #[test]
    fn expired_or_orphaned_requests_are_reported_as_denied() {
        let start = Instant::now();
        let id = Uuid::new_v4();
        let mut view = HiddenView::new(start);
        let request = |reply| BridgeEvent {
            value: serde_json::json!({"hook_event_name": "PermissionRequest", "session_id": id.to_string(), "tool_name": "Bash", "tool_input": {"command": "rm x"}}),
            reply: Some(reply),
        };
        let (tx, rx) = mpsc::channel();
        view.receive(request(tx), id, start);
        let (second, denied) = mpsc::channel();
        view.receive(request(second), id, start);
        assert!(!denied.recv().unwrap());
        assert!(view.check_stall(start + PERMISSION_REVIEW, String::new));
        assert!(view.pending.is_none());
        assert!(view.lines.last().unwrap().contains("expirou"));
        drop(rx);
        // The hook already gave up: a late approval is reported as denied.
        let (tx, rx) = mpsc::channel();
        view.receive(request(tx), id, start);
        drop(rx);
        view.decide(true, false);
        assert!(view.lines.last().unwrap().contains("nada foi aprovado"));
        assert!(PERMISSION_REVIEW < PERMISSION_TIMEOUT);
    }
    #[cfg(unix)]
    #[test]
    fn fake_cli_runs_in_pty_and_echoes_input() {
        use std::{fs, os::unix::fs::PermissionsExt};
        let root = std::env::temp_dir().join(format!("bee-pty-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let fake = root.join("claude-fake");
        fs::write(&fake, "#!/bin/sh\nprintf 'READY\\r\\n'\nIFS= read -r line\nprintf 'ECHO:%s\\r\\n' \"$line\"\n").unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
        let id = Uuid::new_v4();
        let mut pty = Pty::start(
            fake.as_os_str(),
            &root,
            id,
            false,
            pty_size(10, 40),
            true,
            None,
        )
        .unwrap();
        pty.write(b"hello\r").unwrap();
        let mut parser = vt100::Parser::new(10, 40, 0);
        let until = Instant::now() + Duration::from_secs(3);
        while Instant::now() < until {
            if let Ok(Output::Bytes(bytes)) = pty.output.recv_timeout(Duration::from_millis(50)) {
                parser.process(&bytes);
            }
            if parser.screen().contents().contains("ECHO:hello") {
                break;
            }
        }
        assert!(parser.screen().contents().contains("READY"));
        assert!(parser.screen().contents().contains("ECHO:hello"));
        assert_eq!(pty.child.wait().unwrap().exit_code(), 0);
        pty.exited = true;
        fs::remove_dir_all(root).unwrap();
    }
}
