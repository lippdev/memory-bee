//! Layout and drawing. Pure with respect to `App`: it only reads state and
//! never touches the filesystem, so it is exercised with `TestBackend`.
use crate::tui::app::{App, Detail, Focus, SessionRow, View, state_glyph};
use memory_bee::claude::ReadState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

const MIN_WIDTH: u16 = 30;
const MIN_HEIGHT: u16 = 8;
const NARROW_WIDTH: u16 = 80;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(app.palette.bg).fg(app.palette.ink)),
        area,
    );
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = Paragraph::new(format!(
            "terminal muito pequeno (mínimo {MIN_WIDTH}×{MIN_HEIGHT})"
        ))
        .style(Style::default().fg(app.palette.warn));
        f.render_widget(msg, area);
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(f, app, rows[0]);
    match app.view {
        View::Help => draw_help(f, app, rows[1]),
        View::Resume => draw_resume(f, app, rows[1]),
        View::Sessions => draw_sessions(f, app, rows[1]),
    }
    draw_status(f, app, rows[2]);
    draw_hints(f, app, rows[3]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.palette;
    const BRAND: &str = " memory bee  ";
    let mut spans = vec![
        Span::styled("⬢", Style::default().fg(p.accent)),
        Span::styled(
            BRAND,
            Style::default().fg(p.ink).add_modifier(Modifier::BOLD),
        ),
    ];
    let mut tabs: Vec<(&'static str, bool)> = vec![("sessões", app.view == View::Sessions)];
    if app.bundle.is_some() {
        tabs.push(("retomada", app.view == View::Resume));
    }
    tabs.push(("ajuda", app.view == View::Help));
    let mut left_width = 1 + BRAND.chars().count();
    for (i, (label, active)) in tabs.iter().enumerate() {
        let style = if *active {
            Style::default().fg(p.ink).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.ink3)
        };
        spans.push(Span::styled(*label, style));
        left_width += label.chars().count();
        if i + 1 < tabs.len() {
            spans.push(Span::styled(" · ", Style::default().fg(p.ink4)));
            left_width += 3;
        }
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
    let project = app.project.display().to_string();
    // Mirrors the approved prototype's header rule: only draw the
    // right-aligned path when it cannot collide with the tabs, instead of
    // letting the two paragraphs overlap and silently erase each other.
    if (area.width as usize) >= left_width + project.chars().count() + 3 {
        let right = Rect {
            x: area.x + area.width - project.chars().count() as u16 - 1,
            width: project.chars().count() as u16,
            ..area
        };
        f.render_widget(
            Paragraph::new(project).style(Style::default().fg(p.ink4)),
            right,
        );
    }
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.palette;
    let (glyph, color, text) = if app.partial {
        (
            "▲",
            p.warn,
            format!(
                "{} sessões · descoberta parcial, ver diagnósticos",
                app.sessions.len()
            ),
        )
    } else {
        (
            "●",
            p.ok,
            format!("{} sessões · somente leitura", app.sessions.len()),
        )
    };
    let line = Line::from(vec![
        Span::styled(glyph, Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(text, Style::default().fg(p.ink3)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn draw_hints(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.palette;
    let items: &[(&str, &str)] = match app.view {
        View::Sessions if app.bundle.is_some() => &[
            ("↑↓", "mover"),
            ("tab", "detalhe"),
            ("esc", "voltar"),
            ("r", "retomada"),
            ("?", "ajuda"),
            ("q", "sair"),
        ],
        View::Sessions => &[
            ("↑↓", "mover"),
            ("tab", "detalhe"),
            ("esc", "voltar"),
            ("?", "ajuda"),
            ("q", "sair"),
        ],
        View::Resume => &[
            ("t", "trocar destino"),
            ("v", "verificar pacote"),
            ("esc", "voltar"),
            ("q", "sair"),
        ],
        View::Help => &[("esc", "voltar"), ("q", "sair")],
    };
    let mut spans = Vec::new();
    for (k, v) in items {
        spans.push(Span::styled(*k, Style::default().fg(p.ink2)));
        spans.push(Span::styled(
            format!(" {v}   "),
            Style::default().fg(p.ink4),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_sessions(f: &mut Frame, app: &mut App, area: Rect) {
    let narrow = area.width < NARROW_WIDTH;
    if narrow {
        match app.focus {
            Focus::Detail => draw_detail(f, app, area),
            Focus::List => draw_list(f, app, area),
        }
        return;
    }
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
        .split(area);
    draw_list(f, app, cols[0]);
    draw_detail(f, app, cols[1]);
}

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let p = &app.palette;
    let border = if app.focus == Focus::List {
        p.accent
    } else {
        p.faint
    };
    if app.sessions.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Sessões ")
            .border_style(Style::default().fg(border));
        f.render_widget(
            Paragraph::new("nenhuma sessão encontrada sob as raízes informadas")
                .style(Style::default().fg(p.ink3))
                .block(block)
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    // Border (2) + the highlight symbol ratatui reserves on every row (2).
    let row_width = (area.width as usize).saturating_sub(4);
    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .map(|s| session_line(s, p, row_width))
        .map(ListItem::new)
        .collect();
    let title = format!(" Sessões ({}) ", app.sessions.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border)),
        )
        .highlight_style(
            Style::default()
                .fg(p.ink)
                .bg(p.faint)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("❯ ");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

/// `discovery::Session.agent` is `"claude-code"`, wider than the fixed
/// agent column; shorten it so the label column never loses its separator.
fn agent_short(agent: &str) -> &str {
    if agent == "claude-code" {
        "claude"
    } else {
        agent
    }
}

/// Shortens a session file path for display: relative to the project when
/// it is nested under it, and elided from the front (keeping the file name)
/// when it is still long, since the tail carries the meaning.
fn display_path(path: &std::path::Path, project: &std::path::Path) -> String {
    let shown = path
        .strip_prefix(project)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string());
    const CAP: usize = 64;
    let chars: Vec<char> = shown.chars().collect();
    if chars.len() <= CAP {
        shown
    } else {
        format!(
            "…{}",
            chars[chars.len() - CAP + 1..].iter().collect::<String>()
        )
    }
}

/// Truncates to at most `max` characters, adding `…` when it cuts, so it
/// never silently drops trailing information (unicode-aware).
fn fit(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let mut out: String = chars[..max.saturating_sub(1)].iter().collect();
    out.push('…');
    out
}

/// The event count, diagnostics count and state glyph are load-bearing, so
/// their width is reserved first; only the free-form label is truncated
/// when the row does not fit (`fit`), keeping every row's right edge
/// aligned and never dropping the glyph the way plain clipping would.
fn session_line(s: &SessionRow, p: &crate::tui::theme::Palette, width: usize) -> Line<'static> {
    let (glyph, _) = state_glyph(&s.state);
    let glyph_color = match s.state {
        ReadState::Read => p.ok,
        ReadState::Partial => p.warn,
        ReadState::Empty => p.ink4,
    };
    let agent = format!("{:<7}", agent_short(s.agent));
    let sub = if s.is_subagent { " (sub)" } else { "" };
    let attn = if s.requires_branch_selection {
        " ▲"
    } else {
        ""
    };
    let mut right = format!("{} ev", s.event_count);
    if s.diagnostics > 0 {
        right.push_str(&format!(" {} diag", s.diagnostics));
    }
    let reserved =
        agent.chars().count() + 1 + attn.chars().count() + 1 + right.chars().count() + 1 + 1;
    let budget = width.saturating_sub(reserved).max(3);
    let label = fit(&format!("{}{sub}", s.label), budget);
    Line::from(vec![
        Span::styled(agent, Style::default().fg(p.ink3)),
        Span::styled(label, Style::default().fg(p.ink)),
        Span::styled(attn, Style::default().fg(p.warn)),
        Span::raw(" "),
        Span::styled(right, Style::default().fg(p.ink4)),
        Span::raw(" "),
        Span::styled(glyph, Style::default().fg(glyph_color)),
    ])
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.palette;
    let border = if app.focus == Focus::Detail {
        p.accent
    } else {
        p.faint
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Detalhe ")
        .border_style(Style::default().fg(border));
    let Some(row) = app.selected() else {
        f.render_widget(
            Paragraph::new("selecione uma sessão")
                .style(Style::default().fg(p.ink3))
                .block(block),
            area,
        );
        return;
    };
    let mut lines = vec![
        kv(p, "Sessão", &row.label),
        kv(p, "Agente", row.agent),
        kv(p, "Arquivo", &display_path(&row.path, &app.project)),
    ];
    if row.requires_branch_selection {
        lines.push(Line::from(Span::styled(
            "múltiplos ramos, escolha um:",
            Style::default().fg(p.warn),
        )));
        for (i, tip) in row.branch_tips.iter().enumerate() {
            let style = if i == app.branch_pick {
                Style::default().fg(p.ink).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.ink3)
            };
            lines.push(Line::from(Span::styled(
                format!("  {} {}", i + 1, tip),
                style,
            )));
        }
    }
    match &app.detail {
        None => lines.push(Line::from(Span::styled(
            "tab ou enter para inspecionar",
            Style::default().fg(p.ink4),
        ))),
        Some(Err(e)) => lines.push(Line::from(Span::styled(
            format!("erro: {e}"),
            Style::default().fg(p.warn),
        ))),
        Some(Ok(detail)) => append_detail_lines(detail, p, &mut lines),
    }
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}

fn append_detail_lines(
    detail: &Detail,
    p: &crate::tui::theme::Palette,
    lines: &mut Vec<Line<'static>>,
) {
    match detail {
        Detail::Claude(r) => {
            lines.push(kv(p, "Estado", &format!("{:?}", r.state)));
            lines.push(kv(p, "Linhas", &r.lines.to_string()));
            lines.push(kv(p, "Eventos retidos", &r.events.len().to_string()));
            lines.push(kv(p, "Registros", &r.records.len().to_string()));
            lines.push(kv(p, "Compat.", "não certificada"));
            if r.requires_branch_selection {
                lines.push(kv(p, "Seleção", "necessária, veja acima"));
            } else if let Some(sel) = &r.selection {
                lines.push(kv(
                    p,
                    "Ramo",
                    &format!(
                        "{} · {} registros e {} eventos excluídos",
                        sel.leaf_uuid, sel.excluded_records, sel.excluded_events
                    ),
                ));
            }
            if !r.diagnostics.is_empty() {
                lines.push(kv(p, "Diagnósticos", &r.diagnostics.len().to_string()));
            }
        }
        Detail::Codex(r) => {
            lines.push(kv(p, "Estado", &format!("{:?}", r.state)));
            lines.push(kv(p, "Linhas", &r.lines.to_string()));
            lines.push(kv(p, "Eventos retidos", &r.events.len().to_string()));
            lines.push(kv(p, "Registros", &r.records.len().to_string()));
            lines.push(kv(p, "Compat.", "leitor experimental"));
            if !r.diagnostics.is_empty() {
                lines.push(kv(p, "Diagnósticos", &r.diagnostics.len().to_string()));
            }
        }
    }
    if let Some(text) = detail.first_human_request() {
        lines.push(Line::from(Span::styled(
            "Primeiro pedido:",
            Style::default().fg(p.ink3),
        )));
        lines.push(Line::from(Span::styled(
            truncate(text, 200),
            Style::default().fg(p.ink),
        )));
    }
}

fn draw_resume(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.palette;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Retomada ")
        .border_style(Style::default().fg(p.accent));
    let Some(_bundle) = &app.bundle else {
        f.render_widget(
            Paragraph::new("nenhum pacote informado (--bundle <dir>)")
                .style(Style::default().fg(p.ink3))
                .block(block),
            area,
        );
        return;
    };
    let mut lines = vec![kv(
        p,
        "Destino",
        match app.resume_target {
            memory_bee::resume::Target::Claude => "claude",
            memory_bee::resume::Target::Codex => "codex",
        },
    )];
    match &app.resume {
        None => lines.push(Line::from(Span::styled(
            "r prepara a prévia · nada é executado",
            Style::default().fg(p.ink4),
        ))),
        Some(Err(e)) => lines.push(Line::from(Span::styled(
            format!("erro: {e}"),
            Style::default().fg(p.warn),
        ))),
        Some(Ok(prep)) => {
            lines.push(kv(p, "Modo", prep.mode));
            lines.push(kv(p, "Base", prep.base_match));
            lines.push(kv(p, "Mudanças", &format!("{:?}", prep.changes)));
            if prep.attention.is_empty() {
                lines.push(kv(p, "Atenção", "nenhuma"));
            } else {
                lines.push(Line::from(Span::styled(
                    "Atenção:",
                    Style::default().fg(p.warn),
                )));
                for a in &prep.attention {
                    lines.push(Line::from(Span::styled(
                        format!("  ▲ {a}"),
                        Style::default().fg(p.warn),
                    )));
                }
            }
            lines.push(Line::from(Span::styled(
                "Passos:",
                Style::default().fg(p.ink3),
            )));
            for (i, step) in prep.steps.iter().enumerate() {
                lines.push(Line::from(Span::styled(
                    format!("  {} {}", i + 1, step.description),
                    Style::default().fg(p.ink),
                )));
            }
            lines.push(Line::from(Span::styled(
                "lançar só pela CLI com --output e --launch <confirmação>",
                Style::default().fg(p.ink4),
            )));
        }
    }
    if let Some(Ok(verified)) = &app.verify {
        lines.push(Line::from(Span::styled(
            "Verificação:",
            Style::default().fg(p.ink3),
        )));
        lines.push(kv(p, "Válido", &verified.valid.to_string()));
        lines.push(kv(p, "Redaction", &verified.redaction));
    } else if let Some(Err(e)) = &app.verify {
        lines.push(Line::from(Span::styled(
            format!("verificação: erro: {e}"),
            Style::default().fg(p.warn),
        )));
    }
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.palette;
    let rows: &[(&str, &str)] = &[
        ("↑ ↓  j k", "mover a seleção"),
        ("tab", "abrir o detalhe da sessão"),
        ("1-9", "escolher um ramo quando necessário"),
        ("esc", "voltar"),
        ("s", "lista de sessões"),
        ("r", "prévia de retomada (com --bundle)"),
        ("t", "trocar o destino da retomada"),
        ("v", "verificar o pacote (com --bundle)"),
        ("?", "esta ajuda"),
        ("q", "sair"),
    ];
    let lines: Vec<Line> = rows
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{k:<10}"), Style::default().fg(p.ink)),
                Span::styled(*v, Style::default().fg(p.ink3)),
            ])
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Atalhos ")
        .border_style(Style::default().fg(p.accent));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn kv(p: &crate::tui::theme::Palette, key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<16}"), Style::default().fg(p.ink3)),
        Span::styled(value.to_string(), Style::default().fg(p.ink)),
    ])
}

fn truncate(text: &str, max: usize) -> String {
    let mut chars = text.chars();
    let head: String = (&mut chars).take(max).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}
