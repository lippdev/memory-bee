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
    io::{self, Read, Write},
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

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
                    parser.process(&bytes);
                    let old_len = query_tail.len();
                    query_tail.extend_from_slice(&bytes);
                    for (pattern, reply) in [
                        (b"\x1b[6n".as_slice(), None),
                        (b"\x1b[5n".as_slice(), Some(b"\x1b[0n".as_slice())),
                    ] {
                        for pos in 0..query_tail.len().saturating_sub(pattern.len() - 1) {
                            if pos + pattern.len() > old_len
                                && query_tail[pos..].starts_with(pattern)
                            {
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
    fn fake_cli_runs_in_pty_and_echoes_input() {
        use std::{fs, os::unix::fs::PermissionsExt};
        let root = std::env::temp_dir().join(format!("bee-pty-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let fake = root.join("claude-fake");
        fs::write(&fake, "#!/bin/sh\nprintf 'READY\\r\\n'\nIFS= read -r line\nprintf 'ECHO:%s\\r\\n' \"$line\"\n").unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
        let id = Uuid::new_v4();
        let mut pty =
            Pty::start(fake.as_os_str(), &root, id, false, pty_size(10, 40), true).unwrap();
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
