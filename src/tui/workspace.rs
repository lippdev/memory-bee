//! Unified conversation UI. Public entry is explicitly demo-only until both
//! subscription providers pass the release gate. No provider calls here.
use super::theme::{Mode, Palette};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use memory_bee::{
    bundle,
    workspace::{
        Agent, Event,
        adapter::{Adapter, Demo},
        store::{State, Store},
    },
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Paragraph, Wrap},
};
use std::path::PathBuf;

pub const MENU: &[&str] = &[
    "/agent",
    "/accounts",
    "/sessions",
    "/export",
    "/projects",
    "/settings",
    "/permissions",
    "/diff",
    "/help",
    "/quit",
];
#[derive(Clone)]
pub enum Choice {
    Command(String),
    Agent(Agent),
    Profile(String),
    Session(usize),
    Switch(Agent, String, bool),
    Theme(Mode),
    NoColor,
    Approve(String, bool),
    Exit,
}
pub enum Form {
    ExportPath,
    ExportLines(PathBuf),
    ExportConfirm,
    SwitchConfirm(Agent, String),
    Profile,
}
pub struct Modal {
    pub title: String,
    pub body: String,
    pub choices: Vec<(String, Choice)>,
    pub selected: usize,
    pub input: String,
    pub form: Option<Form>,
    pub scroll: u16,
}
impl Modal {
    fn menu(title: &str, choices: Vec<(String, Choice)>) -> Self {
        Self {
            title: title.into(),
            body: String::new(),
            choices,
            selected: 0,
            input: String::new(),
            form: None,
            scroll: 0,
        }
    }
    fn form(title: &str, body: String, form: Form) -> Self {
        Self {
            title: title.into(),
            body,
            choices: Vec::new(),
            selected: 0,
            input: String::new(),
            form: Some(form),
            scroll: 0,
        }
    }
    fn info(title: &str, body: String) -> Self {
        Self {
            body,
            ..Self::menu(title, Vec::new())
        }
    }
}
pub struct App {
    pub state: State,
    store: Option<Store>,
    adapter: Demo,
    pub draft: String,
    pub modal: Option<Modal>,
    pub notice: String,
    pub quit: bool,
    pub frozen: bool,
    pub scroll: u16,
    pub palette: Palette,
    mode: Mode,
    no_color: bool,
    prepared: Option<(bundle::Prepared, PathBuf)>,
    pub home: bool,
}
impl App {
    pub fn new(state: State, store: Option<Store>, mode: Mode, no_color: bool) -> Self {
        let adapter = Demo::new(state.current().agent);
        Self {
            state,
            store,
            adapter,
            draft: String::new(),
            modal: None,
            notice: "SIMULAÇÃO: sem modelos, autenticação ou edição do projeto".into(),
            quit: false,
            frozen: false,
            scroll: 0,
            palette: Palette::new(mode, no_color),
            mode,
            no_color,
            prepared: None,
            home: false,
        }
    }
    fn save(&mut self) {
        if let Some(store) = &self.store
            && let Err(e) = store.save(&self.state)
        {
            self.adapter.interrupt();
            self.frozen = true;
            self.notice =
                format!("Falha ao salvar; envio suspenso, histórico em memória exportável: {e}");
        }
    }
    pub fn tick(&mut self) {
        if self.frozen {
            return;
        }
        if let Some(event) = self.adapter.poll() {
            self.state.current_mut().record(event);
            self.save();
        }
    }
    pub fn interrupt(&mut self) {
        self.adapter.interrupt();
        // Apply the terminal event now so a subsequent menu action cannot race it.
        self.tick();
        self.notice = "Turno interrompido; histórico preservado".into();
    }
    fn info(&mut self, title: &str, body: String) {
        self.modal = Some(Modal::info(title, body));
    }
    pub fn command(&mut self, command: &str) {
        match command {
            "/menu" => self.modal = Some(Modal::menu("Menu Memory Bee", MENU.iter().map(|s| (s.to_string(),Choice::Command(s.to_string()))).collect())),
            "/agent" => self.modal = Some(Modal::menu("Agentes simulados", [Agent::Claude,Agent::Codex].into_iter().map(|a| (format!("{} (demo)",a.label()),Choice::Agent(a))).collect())),
            "/accounts" => {
                let mut choices: Vec<_> = self.state.profiles.iter().map(|p| (p.clone(),Choice::Profile(p.clone()))).collect();
                choices.push(("Adicionar perfil simulado".into(),Choice::Command("/profile-new".into())));
                self.modal = Some(Modal::menu("Perfis DEMO — sem credenciais ou login real", choices));
            }
            "/profile-new" => self.modal = Some(Modal::form("Novo perfil simulado", "Nome local, sem autenticação. Enter adiciona; Esc cancela.".into(), Form::Profile)),
            "/sessions" => self.modal = Some(Modal::menu("Sessões locais", self.state.sessions.iter().enumerate().map(|(i,s)| (format!("#{} · {} · {} · {} eventos",s.id,s.agent.label(),s.profile,s.events.len()),Choice::Session(i))).collect())),
            "/export" => self.modal = Some(Modal::form("Exportar SIMULAÇÃO", "Caminho literal de uma pasta NOVA; o pai deve existir. Apenas registros desta demonstração.".into(), Form::ExportPath)),
            "/projects" => { self.home = true; self.modal = None; }
            "/settings" => self.modal = Some(Modal::menu("Aparência", vec![("Tema escuro".into(),Choice::Theme(Mode::Dark)),("Tema claro".into(),Choice::Theme(Mode::Light)),("Alternar sem cor".into(),Choice::NoColor)])),
            "/permissions" => {
                let pending = self.state.current().events.iter().rev().find_map(|e| match e {
                    Event::Approval{id,command} => Some(Some((id.clone(),command.clone()))),
                    Event::Decision{..}|Event::Interrupted|Event::Completed|Event::Error{..} => Some(None), _=>None,
                }).flatten();
                if let Some((id,command)) = pending {
                    let mut panel = Modal::menu("Permissão SIMULADA", vec![("Negar".into(),Choice::Approve(id.clone(),false)),("Permitir uma vez".into(),Choice::Approve(id,true))]);
                    panel.body = format!("{} / {}\n{}\nProjeto: {}",self.state.current().agent.label(),self.state.current().profile,command,self.state.project);
                    self.modal = Some(panel);
                } else { self.info("Permissões", "Nenhuma permissão pendente.".into()); }
            }
            "/diff" => {
                let text = self.state.current().events.iter().rev().find_map(|e| if let Event::Change{path,diff} = e { Some(format!("SIMULAÇÃO — {path}\n{diff}")) } else {None}).unwrap_or("Nenhuma alteração simulada nesta sessão.".into());
                self.info("Alterações",text);
            }
            "/help" => self.info("Ajuda", "SIMULAÇÃO — não chama agentes nem edita código.\n\nEnter envia; Ctrl+J insere linha; F2 ou / abre menu.\nCtrl+C interrompe; Ctrl+Q ou /quit sai (confirma se ativo).\nPgUp/PgDn rola; Home volta ao início; End acompanha o fim.\nTab abre a permissão pendente. Esc fecha o painel.\n\nComandos: /agent /accounts /sessions /export /projects /settings /permissions /diff /help /quit\n\nEnvie [erro] para simular falha de provedor. Perfis são fictícios. Colar texto nunca o envia automaticamente.\nAssinaturas reais só serão liberadas após validação dos dois agentes.".into()),
            "/quit" => if self.state.current().running { self.modal = Some(Modal::menu("Turno ativo — interromper e sair? Esc volta",vec![("Interromper e sair".into(),Choice::Exit)])); } else { self.quit = true; },
            _ => self.info("Comando desconhecido",format!("{command}\nUse /help. Nada foi enviado ao agente.")),
        }
    }
    fn switching(&mut self, agent: Agent, profile: String) {
        if self.state.current().running {
            self.info(
                "Turno ativo",
                "Aguarde ou use Ctrl+C antes de trocar agente/conta/sessão.".into(),
            );
            return;
        }
        self.modal = Some(Modal::menu(
            "Nova sessão — a origem será preservada",
            vec![
                (
                    "Conversa vazia".into(),
                    Choice::Switch(agent, profile.clone(), false),
                ),
                (
                    "Revisar e levar contexto".into(),
                    Choice::Switch(agent, profile, true),
                ),
            ],
        ));
    }
    fn choose(&mut self, choice: Choice) -> Result<(), String> {
        match choice {
            Choice::Command(s) => self.command(&s),
            Choice::Agent(a) => self.switching(a, self.state.current().profile.clone()),
            Choice::Profile(p) => self.switching(self.state.current().agent, p),
            Choice::Session(i) => {
                if self.state.current().running {
                    return Err("Aguarde ou interrompa o turno antes de abrir outra sessão".into());
                }
                self.state.selected = i;
                self.adapter = Demo::new(self.state.current().agent);
                self.draft.clear();
                self.scroll = 0;
                self.save();
            }
            Choice::Switch(a, p, context) => {
                if context {
                    let prepared = bundle::prepare_workspace_demo(
                        self.state.current(),
                        &bundle::Options::default(),
                    )?;
                    self.modal = Some(Modal::form(
                        "Revisar passagem de contexto",
                        format!(
                            "Destino: {} / {}\nDigite CONTINUAR para criar sessão com o contexto abaixo. Apenas mensagens de usuário e texto simulado serão copiados; ferramentas e permissões ficam na origem.\n\n{}\n\nHistórico completo:\n{}",
                            a.label(),
                            p,
                            prepared.handoff(),
                            prepared.history()
                        ),
                        Form::SwitchConfirm(a, p),
                    ));
                } else {
                    self.switch(a, p, false)?;
                }
            }
            Choice::Theme(mode) => {
                self.mode = mode;
                self.palette = Palette::new(mode, self.no_color);
            }
            Choice::NoColor => {
                self.no_color = !self.no_color;
                self.palette = Palette::new(self.mode, self.no_color);
            }
            Choice::Approve(id, allow) => self.adapter.decide(&id, allow)?,
            Choice::Exit => {
                self.interrupt();
                self.quit = true;
            }
        }
        Ok(())
    }
    fn switch(&mut self, agent: Agent, profile: String, context: bool) -> Result<(), String> {
        if self.frozen {
            return Err("Estado sem persistência; exporte antes de sair".into());
        }
        self.state.fork(agent, profile, context)?;
        self.adapter = Demo::new(agent);
        self.draft.clear();
        self.scroll = 0;
        self.save();
        Ok(())
    }
    fn form(&mut self, form: Form, input: String) -> Result<(), String> {
        match form {
            Form::Profile => {
                let name = input.trim();
                if name.is_empty()
                    || name.chars().count() > 40
                    || name.chars().any(char::is_control)
                    || self.state.profiles.len() >= 32
                {
                    return Err("Nome de 1–40 caracteres; máximo de 32 perfis".into());
                }
                let name = format!("{name} (demo)");
                if self.state.profiles.contains(&name) {
                    return Err("Perfil já existe".into());
                }
                self.state.profiles.push(name);
                self.save();
                self.command("/accounts");
            }
            Form::ExportPath => {
                if input.is_empty() {
                    return Err("Informe pasta nova".into());
                }
                self.modal=Some(Modal::form("Excluir registros da exportação", "Números separados por vírgula; vazio mantém todos. A numeração aparece no histórico da conversa.".into(),Form::ExportLines(input.into())));
            }
            Form::ExportLines(path) => {
                let mut options = bundle::Options::default();
                if !input.trim().is_empty() {
                    for n in input.split(',') {
                        options.exclude_lines.insert(
                            n.trim()
                                .parse()
                                .map_err(|_| "Números de registros inválidos")?,
                        );
                    }
                }
                let prepared = bundle::prepare_workspace_demo(self.state.current(), &options)?;
                let body = format!(
                    "Destino: {}\nDigite EXPORTAR para gravar este snapshot.\n\n{}\n\nHistórico:\n{}\n\nManifesto:\n{}\n\nAchados:\n{}",
                    path.display(),
                    prepared.handoff(),
                    prepared.history(),
                    serde_json::to_string_pretty(prepared.manifest()).map_err(|e| e.to_string())?,
                    serde_json::to_string(prepared.findings()).map_err(|e| e.to_string())?
                );
                self.prepared = Some((prepared, path));
                self.modal = Some(Modal::form(
                    "Prévia de SIMULAÇÃO",
                    body,
                    Form::ExportConfirm,
                ));
            }
            Form::ExportConfirm => {
                if input != "EXPORTAR" {
                    return Err("Confirmação diferente de EXPORTAR; nada foi gravado".into());
                }
                let (prepared, path) = self.prepared.take().ok_or("Prévia indisponível")?;
                prepared.write(&path).map_err(|e| e.to_string())?;
                self.info(
                    "Pacote gravado",
                    format!(
                        "SIMULAÇÃO: {}\nPode ser conferido com memory-bee verify.",
                        path.display()
                    ),
                );
            }
            Form::SwitchConfirm(a, p) => {
                if input != "CONTINUAR" {
                    return Err("Confirmação diferente de CONTINUAR; origem preservada".into());
                }
                self.switch(a, p, true)?;
            }
        }
        Ok(())
    }
    pub fn paste(&mut self, text: &str) {
        let target = if let Some(modal) = &mut self.modal {
            if modal.form.is_none() {
                return;
            }
            &mut modal.input
        } else {
            &mut self.draft
        };
        for c in text
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        {
            if target.len() + c.len_utf8() > 64 * 1024 {
                break;
            }
            target.push(c);
        }
    }
    pub fn key(&mut self, key: KeyEvent) {
        let code = key.code;
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match code {
                KeyCode::Char('c') => {
                    self.interrupt();
                    return;
                }
                KeyCode::Char('q') => {
                    self.command("/quit");
                    return;
                }
                KeyCode::Char('j') => {
                    if self.modal.is_none() {
                        self.paste("\n");
                    }
                    return;
                }
                _ => return,
            }
        }
        if let Some(mut modal) = self.modal.take() {
            let result = match code {
                KeyCode::Esc => {
                    self.prepared = None;
                    return;
                }
                KeyCode::Enter => {
                    if let Some(form) = modal.form {
                        self.form(form, modal.input)
                    } else if let Some((_, choice)) = modal.choices.get(modal.selected) {
                        self.choose(choice.clone())
                    } else {
                        Ok(())
                    }
                }
                _ => {
                    match code {
                        KeyCode::Up => modal.selected = modal.selected.saturating_sub(1),
                        KeyCode::Down => {
                            modal.selected =
                                (modal.selected + 1).min(modal.choices.len().saturating_sub(1))
                        }
                        KeyCode::PageUp => modal.scroll = modal.scroll.saturating_sub(5),
                        KeyCode::PageDown => modal.scroll = modal.scroll.saturating_add(5),
                        KeyCode::Home => modal.scroll = 0,
                        KeyCode::Backspace => {
                            modal.input.pop();
                        }
                        KeyCode::Char(c)
                            if modal.form.is_some()
                                && !c.is_control()
                                && modal.input.len() + c.len_utf8() <= 64 * 1024 =>
                        {
                            modal.input.push(c)
                        }
                        _ => {}
                    }
                    self.modal = Some(modal);
                    Ok(())
                }
            };
            if let Err(e) = result {
                self.prepared = None;
                self.info("Ação não concluída", e);
            }
            return;
        }
        if self.home {
            match code {
                KeyCode::Enter | KeyCode::Esc => self.home = false,
                KeyCode::F(2) => self.command("/menu"),
                _ => {}
            }
            return;
        }
        match code {
            KeyCode::F(2) => self.command("/menu"),
            KeyCode::Tab => self.command("/permissions"),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_add(5),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_sub(5),
            KeyCode::End => self.scroll = 0,
            KeyCode::Home => self.scroll = u16::MAX,
            KeyCode::Backspace => {
                self.draft.pop();
            }
            KeyCode::Char('/') if self.draft.is_empty() => self.command("/menu"),
            KeyCode::Char(c) if !c.is_control() && self.draft.len() + c.len_utf8() <= 64 * 1024 => {
                self.draft.push(c)
            }
            KeyCode::Enter => {
                if self.draft.starts_with('/') {
                    let command = std::mem::take(&mut self.draft);
                    self.command(command.trim());
                } else if !self.draft.trim().is_empty() {
                    if self.frozen {
                        self.notice =
                            "Envio suspenso: falha de persistência. Exporte o histórico.".into();
                        return;
                    }
                    match self.adapter.send(&self.draft) {
                        Ok(()) => {
                            let text = std::mem::take(&mut self.draft);
                            self.state.current_mut().record(Event::User { text });
                            self.save();
                            self.scroll = 0;
                        }
                        Err(e) => self.notice = e,
                    }
                }
            }
            _ => {}
        }
    }
}

fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}
/// Pre-wrap by terminal cell widths so scroll offsets and long paths agree.
fn wrapped(text: &str, width: u16) -> Vec<Line<'static>> {
    let width = usize::from(width.max(1));
    let mut lines = Vec::new();
    for raw in clean(text).split('\n') {
        let mut line = String::new();
        let mut used = 0;
        for c in raw.chars() {
            let piece = if c == '\t' {
                "    ".into()
            } else {
                c.to_string()
            };
            let cells = ratatui::text::Span::raw(piece.clone()).width();
            if used + cells > width && !line.is_empty() {
                lines.push(Line::raw(std::mem::take(&mut line)));
                used = 0;
            }
            line.push_str(&piece);
            used += cells;
        }
        lines.push(Line::raw(line));
    }
    lines
}
pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let p = app.palette;
    f.render_widget(
        Block::default().style(Style::default().fg(p.ink).bg(p.bg)),
        area,
    );
    if area.width < 30 || area.height < 10 {
        f.render_widget(
            Paragraph::new("SIMULAÇÃO. Terminal mínimo 30x10. Ctrl+Q sai."),
            area,
        );
        return;
    }
    if let Some(m) = &app.modal {
        let rows = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(2),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(area);
        f.render_widget(
            Paragraph::new(clean(&format!("SIMULAÇÃO · {}", m.title)))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(p.accent)),
            rows[0],
        );
        let mut text = m.body.clone();
        for (i, (label, _)) in m.choices.iter().enumerate() {
            text.push_str(&format!(
                "\n{} {}",
                if i == m.selected { "›" } else { " " },
                label
            ));
        }
        let lines = wrapped(&text, rows[1].width);
        let max = lines.len().saturating_sub(rows[1].height as usize);
        // Keep selected menu entries visible, including lists larger than the terminal.
        let offset = if m.choices.is_empty() {
            usize::from(m.scroll).min(max)
        } else {
            let prefix = wrapped(&m.body, rows[1].width).len();
            let selected = prefix
                + m.choices
                    .iter()
                    .take(m.selected)
                    .map(|(s, _)| wrapped(&format!("› {s}"), rows[1].width).len())
                    .sum::<usize>();
            selected
                .saturating_sub(rows[1].height as usize - 1)
                .min(max)
        };
        f.render_widget(
            Paragraph::new(lines.into_iter().skip(offset).collect::<Vec<_>>()),
            rows[1],
        );
        let input = if m.form.is_some() {
            format!("> {}", m.input)
        } else {
            "↑↓ escolher · Enter confirmar".into()
        };
        let lines = wrapped(&input, rows[2].width);
        let skip = lines.len().saturating_sub(rows[2].height as usize);
        f.render_widget(
            Paragraph::new(lines.into_iter().skip(skip).collect::<Vec<_>>()),
            rows[2],
        );
        f.render_widget(Paragraph::new("Esc volta · PgUp/Dn rola · Home"), rows[3]);
        return;
    }
    if app.home {
        let text = format!(
            "⬢ MEMORY BEE · SIMULAÇÃO\n\n        ⬡ ⬡\n       ⬡ ⬢ ⬡    abelha: *\n        ⬡ ⬡\n\nProjeto explícito:\n{}\n\n{} sessões locais\nColmeia estática — sem animação\n\nEnter abre conversa · F2 menu",
            app.state.project,
            app.state.sessions.len()
        );
        let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
        f.render_widget(Paragraph::new(wrapped(&text, area.width)), rows[0]);
        f.render_widget(Paragraph::new("Enter conversa · F2 menu"), rows[1]);
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(2),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);
    let s = app.state.current();
    f.render_widget(
        Paragraph::new(format!(
            "⬢ memory bee · SIMULAÇÃO\n{} · {} · sessão #{}\n{}",
            s.agent.label(),
            clean(&s.profile),
            s.id,
            clean(&s.project)
        ))
        .style(Style::default().fg(p.accent).add_modifier(Modifier::BOLD)),
        rows[0],
    );
    let mut text = String::new();
    if s.events.is_empty() {
        text.push_str(
            "Converse aqui com o agente simulado.\nF2 abre o menu; nenhuma CLI ocupa esta tela.\n",
        );
    }
    for (i, e) in s.events.iter().enumerate() {
        let item = match e {
            Event::User { text } => format!("Você: {text}"),
            Event::Text { text } => format!("{}: {text}", s.agent.label()),
            Event::Tool { name, detail } => format!("Ferramenta: {name} · {detail}"),
            Event::Change { path, .. } => format!("Alteração simulada: {path} · /diff"),
            Event::Approval { command, .. } => {
                format!("Permissão: {command}\nTab abre opções; nenhuma ação automática.")
            }
            Event::Decision { allow, .. } => {
                format!("Permissão {}", if *allow { "aceita" } else { "negada" })
            }
            Event::Completed => "Turno simulado concluído".into(),
            Event::Interrupted => "Turno interrompido".into(),
            Event::Error { message } => format!("Erro: {message}"),
            Event::Notice { message } => message.clone(),
        };
        text.push_str(&format!("[{}] {item}\n\n", i + 1));
    }
    let lines = wrapped(&text, rows[1].width);
    let max = lines.len().saturating_sub(rows[1].height as usize);
    let offset = max.saturating_sub(usize::from(app.scroll));
    f.render_widget(
        Paragraph::new(lines.into_iter().skip(offset).collect::<Vec<_>>()),
        rows[1],
    );
    let status = if app.frozen {
        "Falha ao salvar — exporte antes de sair"
    } else if s.running {
        "◌ Turno ativo · Tab permissão · Ctrl+C para"
    } else {
        &app.notice
    };
    f.render_widget(
        Paragraph::new(clean(status)).style(Style::default().fg(p.warn)),
        rows[2],
    );
    let draft = wrapped(&format!("> {}", app.draft), rows[3].width);
    let skip = draft.len().saturating_sub(3);
    f.render_widget(
        Paragraph::new(draft.into_iter().skip(skip).collect::<Vec<_>>())
            .style(Style::default().fg(p.ink)),
        rows[3],
    );
    f.render_widget(
        Paragraph::new("Enter envia · F2 menu · Ctrl+Q sai"),
        rows[4],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    fn app() -> App {
        App::new(
            State::new("/synthetic/project".into(), Agent::Claude),
            None,
            Mode::Dark,
            true,
        )
    }
    fn key(app: &mut App, code: KeyCode) {
        app.key(KeyEvent::new(code, KeyModifiers::NONE));
    }
    fn send(app: &mut App, text: &str) {
        app.paste(text);
        key(app, KeyCode::Enter);
    }
    fn frame(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let b = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| b[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn menus_preserve_draft_and_stream_while_open_and_paste_never_sends() {
        let mut a = app();
        send(&mut a, "task");
        a.paste("draft\n漢字");
        key(&mut a, KeyCode::F(2));
        for _ in 0..5 {
            a.tick();
        }
        assert_eq!(a.state.current().events.len(), 6);
        assert!(a.modal.is_some());
        key(&mut a, KeyCode::Esc);
        assert_eq!(a.draft, "draft\n漢字");
        key(&mut a, KeyCode::Tab);
        key(&mut a, KeyCode::Enter); // Default is deny.
        for _ in 0..3 {
            a.tick();
        }
        assert!(!a.state.current().running);
        assert!(
            a.state
                .current()
                .events
                .iter()
                .any(|e| matches!(e, Event::Decision { allow: false, .. }))
        );
        assert_eq!(a.draft, "draft\n漢字");
    }
    #[test]
    fn interrupt_invalidates_open_approval_and_active_quit_needs_confirmation() {
        let mut a = app();
        send(&mut a, "task");
        for _ in 0..5 {
            a.tick();
        }
        a.command("/permissions");
        a.interrupt();
        key(&mut a, KeyCode::Down);
        key(&mut a, KeyCode::Enter);
        assert!(
            !a.state
                .current()
                .events
                .iter()
                .any(|e| matches!(e, Event::Decision { allow: true, .. }))
        );
        key(&mut a, KeyCode::Esc);
        send(&mut a, "next");
        a.command("/quit");
        assert!(!a.quit);
        key(&mut a, KeyCode::Esc);
        assert!(!a.quit);
        a.command("/quit");
        key(&mut a, KeyCode::Enter);
        assert!(a.quit);
        assert!(!a.state.current().running);
    }
    #[test]
    fn context_switch_reviews_full_history_and_preserves_source() {
        let mut a = app();
        send(&mut a, "first task");
        a.interrupt();
        a.state.current_mut().record(Event::Text {
            text: "middle record".into(),
        });
        a.state.current_mut().record(Event::Completed);
        let source = a.state.current().clone();
        a.choose(Choice::Switch(
            Agent::Codex,
            a.state.profiles[1].clone(),
            true,
        ))
        .unwrap();
        assert!(a.modal.as_ref().unwrap().body.contains("middle record"));
        a.paste("CONTINUAR");
        key(&mut a, KeyCode::Enter);
        assert_eq!(a.state.sessions[0], source);
        assert_eq!(a.state.current().agent, Agent::Codex);
        a.command("/not-a-command");
        assert!(!a.state.current().running);
    }
    #[test]
    fn narrow_home_and_long_menus_keep_controls_and_selection_visible() {
        let mut a = app();
        for (width, height) in [(100, 30), (60, 20), (40, 12), (30, 10)] {
            a.home = true;
            let text = frame(&a, width, height);
            assert!(text.contains("Enter conversa"));
            a.home = false;
            a.command("/menu");
            for _ in 0..9 {
                key(&mut a, KeyCode::Down);
            }
            let text = frame(&a, width, height);
            assert!(text.contains("› /quit"));
            assert!(text.contains("Esc volta"));
            assert!(text.contains("SIMULAÇÃO"));
            key(&mut a, KeyCode::Esc);
        }
    }
}
