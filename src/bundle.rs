//! Context-only portable bundles with optional read-only Git references.
use crate::{
    claude::{ReadState, Report},
    selection::select,
};
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
    time::SystemTime,
};

// Keep each reader's serialized event contract intact; only rendering and packaging are shared.
#[derive(Serialize)]
#[serde(untagged)]
enum Event {
    Claude(crate::claude::Event),
    Codex(crate::codex::Event),
}
macro_rules! event_ref {
    ($name:ident, $ty:ty, $($field:ident).+) => {
        fn $name(&self) -> &$ty {
            match self {
                Self::Claude(e) => &e.$($field).+,
                Self::Codex(e) => &e.$($field).+,
            }
        }
    };
}
impl Event {
    event_ref!(text, str, text);
    event_ref!(role, str, role);
    event_ref!(kind, str, kind);
    event_ref!(provenance, str, provenance);
    event_ref!(line, usize, source.line);
    event_ref!(block, Option<usize>, source.block);
    event_ref!(session, Option<String>, source.session_id);
    fn renumber(&mut self, sequence: usize) {
        match self {
            Self::Claude(e) => e.sequence = sequence,
            Self::Codex(e) => e.sequence = sequence,
        }
    }
}
struct Input {
    agent: &'static str,
    state: ReadState,
    events: Vec<Event>,
    diagnostics: Vec<crate::claude::Diagnostic>,
    observed_versions: BTreeSet<String>,
    selection: Option<crate::claude::Selection>,
}

#[derive(Default)]
pub struct Options {
    pub project: Option<PathBuf>,
    pub include_paths: BTreeSet<String>,
    pub leaf: Option<String>,
    pub exclude_lines: BTreeSet<usize>,
}
#[derive(Debug, Serialize)]
pub struct Finding {
    pub code: &'static str,
    pub line: Option<usize>,
    pub block: Option<usize>,
    pub field: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<usize>,
}
#[derive(Debug, Serialize)]
pub struct Origin {
    agent: &'static str,
    version: Option<String>,
    session_id: Option<String>,
}
pub use crate::git::Project;
#[derive(Debug, Serialize)]
pub struct Payload {
    path: String,
    sha256: String,
    purpose: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Manifest {
    format_version: u8,
    created_at: String,
    source: Origin,
    project: Project,
    code_state: &'static str,
    files: Vec<Payload>,
    omissions: Vec<String>,
    warnings: Vec<String>,
    redaction: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changes: Option<Vec<crate::changes::Change>>,
}
/// Immutable prepared bytes: preview and write use the same payloads and hashes.
#[derive(Debug, Serialize)]
pub struct Prepared {
    manifest: Manifest,
    handoff: String,
    history: String,
    findings: Vec<Finding>,
    partial: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<CodePayloads>,
}
#[derive(Debug, Serialize)]
struct CodePayloads {
    patch: String,
    new_files: Vec<crate::changes::File>,
}
impl Prepared {
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }
    pub fn is_partial(&self) -> bool {
        self.partial
    }
    pub fn handoff(&self) -> &str {
        &self.handoff
    }
    pub fn history(&self) -> &str {
        &self.history
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Creates a new directory; never reuses even an empty existing destination.
    /// Manifest is written last. On ordinary errors, remove only files we created.
    pub fn write(&self, destination: &Path) -> io::Result<()> {
        if !self.findings.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "possible secrets detected; use preview and remove affected selections",
            ));
        }
        let manifest = serde_json::to_vec_pretty(&self.manifest)?;
        let mut directory = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            directory.mode(0o700);
        }
        directory.create(destination)?;
        let mut payloads = vec![
            ("HANDOFF.md", self.handoff.as_bytes()),
            ("history.jsonl", self.history.as_bytes()),
        ];
        if let Some(code) = &self.code {
            if !code.patch.is_empty() {
                payloads.push(("changes.patch", code.patch.as_bytes()));
            }
            for file in &code.new_files {
                payloads.push((&file.path, file.content.as_bytes()));
            }
        }
        payloads.push(("manifest.json", manifest.as_slice()));
        write_payloads(destination, &payloads)
    }
}

fn write_payloads(destination: &Path, payloads: &[(&str, &[u8])]) -> io::Result<()> {
    let mut created = Vec::new();
    let mut created_files_dir = false;
    let result = (|| {
        for (name, content) in payloads {
            if name.starts_with("files/") && !created_files_dir {
                let mut dir = fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    dir.mode(0o700);
                }
                dir.create(destination.join("files"))?;
                created_files_dir = true;
            }
            let path = destination.join(name);
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&path)?;
            created.push(path);
            file.write_all(content)?;
            file.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        for path in created.iter().rev() {
            let _ = fs::remove_file(path);
        }
        if created_files_dir {
            let _ = fs::remove_dir(destination.join("files"));
        }
        let _ = fs::remove_dir(destination);
    }
    result
}

pub fn prepare(report: Report, options: &Options) -> Result<Prepared, String> {
    if !options.include_paths.is_empty() && options.project.is_none() {
        return Err("--include-path requires --project".into());
    }
    let report = if let Some(leaf) = &options.leaf {
        select(report, leaf).map_err(str::to_owned)?
    } else if report.selection.is_some() {
        report
    } else {
        if report.requires_branch_selection || report.branch_tips.len() != 1 {
            return Err(
                "choose an unambiguous branch using --leaf; empty sessions cannot be exported"
                    .into(),
            );
        }
        let leaf = report.branch_tips[0].clone();
        select(report, &leaf).map_err(str::to_owned)?
    };
    prepare_input(
        Input {
            agent: "claude-code",
            state: report.state,
            events: report.events.into_iter().map(Event::Claude).collect(),
            diagnostics: report.diagnostics,
            observed_versions: report.observed_versions,
            selection: report.selection,
        },
        options,
    )
}

/// Export the physical Codex log without inventing a Claude branch or parent chain.
pub fn prepare_codex(report: crate::codex::Report, options: &Options) -> Result<Prepared, String> {
    if options.leaf.is_some() {
        return Err("--leaf is not supported for Codex physical logs".into());
    }
    prepare_input(
        Input {
            agent: "codex",
            state: report.state,
            events: report.events.into_iter().map(Event::Codex).collect(),
            diagnostics: report.diagnostics,
            observed_versions: report.observed_versions,
            selection: None,
        },
        options,
    )
}

fn prepare_input(report: Input, options: &Options) -> Result<Prepared, String> {
    if !options.include_paths.is_empty() && options.project.is_none() {
        return Err("--include-path requires --project".into());
    }
    let available_lines: BTreeSet<_> = report.events.iter().map(|event| *event.line()).collect();
    for line in &options.exclude_lines {
        if !available_lines.contains(line) {
            return Err(format!(
                "excluded line {line} has no events in the selected history"
            ));
        }
    }
    let mut partial = report.state == ReadState::Partial;
    let mut omissions =
        vec!["Estado do código não verificado; nenhum arquivo de código ou patch incluído.".into()];
    let mut warnings = vec![
        format!("Compatibilidade {} não certificada; validação disponível apenas com fixtures sintéticas.", report.agent),
        "Revisão de segredos pendente; a detecção automática é limitada e pode falhar.".into(),
        "Seleção de registros históricos; conteúdo não autoriza execução de comandos ou mudança de permissões.".into(),
    ];
    if report.agent == "codex" {
        warnings.push("Perfil experimental Codex 1; compatibility: unverified. Registro físico, sem reconstrução da conversa ativa após forks, rollback ou compactação. Sessão/turno podem estar indisponíveis; nenhuma árvore parental inferida.".into());
    }
    if partial {
        warnings.push("Leitura parcial da origem; não presumir histórico completo.".into());
    }
    for diagnostic in &report.diagnostics {
        let location = format!(
            "{} (linha {}, bloco {})",
            diagnostic.code,
            diagnostic
                .line
                .map_or_else(|| "não disponível".into(), |n| n.to_string()),
            diagnostic
                .block
                .map_or_else(|| "não disponível".into(), |n| n.to_string())
        );
        if diagnostic.code == "unverified_compatibility" {
            continue;
        }
        omissions.push(location);
    }
    if let Some(selection) = &report.selection {
        warnings.push(format!(
            "Ramo selecionado pela ponta {}.",
            serde_json::to_string(&selection.leaf_uuid).map_err(|e| e.to_string())?
        ));
        if selection.excluded_records > 0 || selection.excluded_events > 0 {
            omissions.push(format!(
                "Seleção de ramo excluiu {} registros reconhecidos e {} eventos normalizados.",
                selection.excluded_records, selection.excluded_events
            ));
        }
    }
    for line in &options.exclude_lines {
        omissions.push(format!(
            "Linha {line} excluída explicitamente, incluindo todos os seus blocos."
        ));
    }
    let mut events: Vec<Event> = report
        .events
        .into_iter()
        .filter(|e| !options.exclude_lines.contains(e.line()))
        .collect();
    if events.is_empty() {
        return Err("selection contains no exportable events".into());
    }
    let mut history = String::new();
    let mut findings = Vec::new();
    for (index, event) in events.iter_mut().enumerate() {
        event.renumber(index + 1);
        // Scan every exported string before JSON escaping, including adapter metadata.
        let value = serde_json::to_value(&event).map_err(|e| e.to_string())?;
        scan_event(&value, *event.line(), *event.block(), &mut findings);
        history.push_str(&serde_json::to_string(event).map_err(|e| e.to_string())?);
        history.push('\n');
    }
    let sessions: BTreeSet<_> = events.iter().filter_map(|e| e.session().clone()).collect();
    if sessions.len() > 1 {
        return Err("selected events contain multiple sessions".into());
    }
    if report.observed_versions.len() > 1 {
        omissions
            .push("Múltiplas versões observadas; source.version permanece indisponível.".into());
    }
    omissions
        .push("Caminho do arquivo de origem e metadados cwd não incluídos automaticamente.".into());
    let version = if report.observed_versions.len() == 1 {
        report.observed_versions.into_iter().next()
    } else {
        None
    };
    if let Some(version) = &version {
        scan(version, None, None, "source.version", &mut findings);
    }
    // Selection UUID is included in warnings even if its line was excluded.
    if let Some(selection) = &report.selection {
        scan(
            &selection.leaf_uuid,
            None,
            None,
            "selection.leaf_uuid",
            &mut findings,
        );
    }
    let mut project = Project::default();
    if let Some(path) = &options.project {
        let observation = crate::git::inspect(path);
        project = observation.project;
        partial |= observation.partial;
        warnings.extend(observation.warnings);
        omissions[0] = "Nenhum arquivo de código ou patch incluído; alterações locais não acompanham o pacote.".into();
        if project.dirty == Some(true) {
            warnings.push("Há alterações locais (incluindo arquivos não rastreados); o commit base não reproduz o estado de trabalho.".into());
        }
        for (field, value) in [
            ("project.remote", &project.remote),
            ("project.branch", &project.branch),
        ] {
            if let Some(value) = value {
                scan(value, None, None, field, &mut findings);
            }
        }
    }
    let with_code = !options.include_paths.is_empty();
    let mut changes = None;
    let mut code = None;
    let mut extra_payloads = Vec::new();
    if with_code {
        let base = project
            .base_commit
            .as_deref()
            .ok_or("selected code requires a verified base commit")?;
        let captured = crate::changes::prepare(
            options.project.as_deref().expect("checked project"),
            base,
            &options.include_paths,
        )?;
        for (selection, content) in &captured.scan_text {
            let start = findings.len();
            scan(content, None, None, "code_selection", &mut findings);
            for finding in &mut findings[start..] {
                finding.selection = Some(*selection);
            }
        }
        partial |= !captured.omissions.is_empty();
        omissions.extend(captured.omissions);
        omissions[0] = "Somente arquivos selecionados e suportados acompanham o pacote; outras alterações locais não estão incluídas.".into();
        warnings.push("Código capturado do conteúdo em disco relativo à base, sem preservar separação staged/unstaged ou executar filtros Git. Aplicação exige revisão e verificação da base.".into());
        if !captured.patch.is_empty() {
            extra_payloads.push(Payload {
                path: "changes.patch".into(),
                sha256: hash(captured.patch.as_bytes()),
                purpose: "patch",
            });
        }
        for file in &captured.new_files {
            extra_payloads.push(Payload {
                path: file.path.clone(),
                sha256: hash(file.content.as_bytes()),
                purpose: "new-file",
            });
        }
        changes = Some(captured.entries);
        code = Some(CodePayloads {
            patch: captured.patch,
            new_files: captured.new_files,
        });
    }
    let included_code = !extra_payloads.is_empty();
    let code_state = if included_code {
        "changes-included"
    } else if project.base_commit.is_some() {
        "base-reference"
    } else {
        "unknown"
    };
    let handoff = render_handoff(
        &events,
        &omissions,
        &warnings,
        &project,
        included_code,
        report.agent,
    );
    let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
    let mut manifest = Manifest {
        format_version: if with_code { 2 } else { 1 },
        created_at: now.to_rfc3339(),
        source: Origin {
            agent: report.agent,
            version,
            session_id: sessions.into_iter().next(),
        },
        project,
        code_state,
        files: vec![
            Payload {
                path: "HANDOFF.md".into(),
                sha256: hash(handoff.as_bytes()),
                purpose: "entrypoint",
            },
            Payload {
                path: "history.jsonl".into(),
                sha256: hash(history.as_bytes()),
                purpose: "history",
            },
        ],
        omissions,
        warnings,
        redaction: "pending-review",
        selected_paths: with_code.then(|| options.include_paths.iter().cloned().collect()),
        changes,
    };
    manifest.files.extend(extra_payloads);
    Ok(Prepared {
        manifest,
        handoff,
        history,
        findings,
        partial,
        code,
    })
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn scan_event(
    value: &serde_json::Value,
    line: usize,
    block: Option<usize>,
    findings: &mut Vec<Finding>,
) {
    fn visit(
        value: &serde_json::Value,
        line: usize,
        block: Option<usize>,
        field: &'static str,
        findings: &mut Vec<Finding>,
    ) {
        match value {
            serde_json::Value::String(text) => scan(text, Some(line), block, field, findings),
            serde_json::Value::Array(items) => {
                for item in items {
                    visit(item, line, block, field, findings);
                }
            }
            serde_json::Value::Object(fields) => {
                for (key, value) in fields {
                    let name = match (field, key.as_str()) {
                        ("event", "source") => "source",
                        ("source", "id") => "source.id",
                        ("source", "parent_id") => "source.parent_id",
                        ("source", "session_id") => "source.session_id",
                        ("source", "agent_id") => "source.agent_id",
                        ("source", "turn_id") => "source.turn_id",
                        ("event", "text") => "text",
                        ("event", "phase") => "phase",
                        ("event", "tool_id") => "tool_id",
                        ("event", "tool_name") => "tool_name",
                        ("event", "tool_namespace") => "tool_namespace",
                        _ => field,
                    };
                    visit(value, line, block, name, findings);
                }
            }
            _ => {}
        }
    }
    visit(value, line, block, "event", findings);
}

fn scan(
    text: &str,
    line: Option<usize>,
    block: Option<usize>,
    field: &'static str,
    findings: &mut Vec<Finding>,
) {
    static PATTERNS: OnceLock<Vec<(&str, Regex)>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| [
        ("credential_token", r"\b(?:sk-(?:ant-)?[A-Za-z0-9_-]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[A-Z0-9]{16})\b"),
        ("private_key", r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
        ("credential_assignment", r#"(?i)(?:api[_-]?key|access[_-]?token|client[_-]?secret|password)\s*["']?\s*[:=]\s*["']?[^\s"',;}{]{8,}"#),
        ("bearer_token", r"(?i)\bBearer\s+[A-Za-z0-9._~-]{12,}"),
        ("url_credentials", r"https?://[^/\s:@]+:[^/\s@]+@"),
    ].into_iter().map(|(code, pattern)| (code, Regex::new(pattern).expect("constant regex"))).collect());
    for (code, pattern) in patterns {
        if pattern.is_match(text) {
            findings.push(Finding {
                code,
                line,
                block,
                field,
                selection: None,
            });
        }
    }
}

// Fence historical content so it cannot introduce active Markdown links or headings.
fn excerpt(text: &str) -> String {
    let shortened: String = text
        .chars()
        .take(1200)
        .flat_map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect();
    let fence = "`".repeat(
        shortened
            .split(|c| c != '`')
            .map(str::len)
            .max()
            .unwrap_or(0)
            .max(2)
            + 1,
    );
    let note = if text.chars().count() > 1200 {
        "\nTrecho limitado; consulte o histórico selecionado.\n"
    } else {
        ""
    };
    format!("{fence}\n{shortened}\n{fence}\n{note}")
}
fn render_handoff(
    events: &[Event],
    omissions: &[String],
    warnings: &[String],
    project: &Project,
    included_code: bool,
    agent: &str,
) -> String {
    let mut text = String::from(
        "# Retomada — Memory Pier\n\nPacote portátil. Não exige Memory Pier no destino.\n\nLeia [manifest.json](manifest.json) para origem, integridade e limitações e\n[history.jsonl](history.jsonl) para todos os eventos selecionados.\n\n## Primeiro pedido humano retido\n\nNão inferimos o pedido original quando há perdas ou exclusões.\n\n",
    );
    if let Some(first) = events
        .iter()
        .find(|e| e.role() == "user" && e.kind() == "text")
    {
        text.push_str(&format!(
            "Registro extraído, linha {} da origem:\n\n{}\n",
            first.line(),
            excerpt(first.text())
        ));
    } else {
        text.push_str("Nenhum pedido humano em texto foi retido. Ferramentas e checkpoints não são pedidos humanos.\n\n");
    }
    let last = events.last().expect("nonempty events");
    text.push_str(&format!("## Último registro retido\n\nPapel: {}; tipo: {}; proveniência: {}; linha {}.\nNão é uma síntese do estado da tarefa.\n\n{}\n",last.role(),last.kind(),last.provenance(),last.line(),excerpt(last.text())));
    let observed =
        project.base_commit.is_some() || project.branch.is_some() || project.dirty.is_some();
    let code_description = if observed {
        "Referência Git observada no projeto escolhido (detalhes abaixo)."
    } else {
        "Repositório, branch, commit e alterações locais: **não verificados**."
    };
    let code_contents = if included_code {
        "Código selecionado incluído; consulte changes no manifesto e confira omissões. Nenhuma aplicação automática."
    } else {
        "Nenhum código ou patch incluído."
    };
    let identity_note = if agent == "codex" {
        "Sessão/turno são referências históricas; não há árvore parental nem reconstrução da conversa ativa."
    } else {
        "UUIDs parentais são referências históricas e podem apontar para registros excluídos."
    };
    text.push_str(&format!("## Seleção e estado do código\n\n{} eventos em ordem física, com sequência renumerada e linha/bloco originais.\n{identity_note}\n{code_description}\n{code_contents}\n\n",events.len()));
    if observed {
        text.push_str(&excerpt(&format!(
            "Origin: {}\nBranch: {}\nCommit base: {}\nAlterações locais: {}",
            project.remote.as_deref().unwrap_or("indisponível/omitido"),
            project
                .branch
                .as_deref()
                .unwrap_or("indisponível (possível detached HEAD)"),
            project.base_commit.as_deref().unwrap_or("desconhecido"),
            project.dirty.map_or("desconhecidas", |dirty| if dirty {
                if included_code {
                    "sim; somente seleção incluída"
                } else {
                    "sim; não incluídas"
                }
            } else {
                "não detectadas (arquivos ignorados não contados)"
            })
        )));
        text.push('\n');
    }
    if included_code {
        text.push_str("## Código selecionado\n\nO manifesto v2 mapeia cada caminho da raiz do projeto ao payload, hashes e modos.\nchanges.patch contém mudanças em arquivos da base; files/ contém novos arquivos.\nRevise também linhas removidas do patch: elas podem conter dados sensíveis.\nNão aplique sem conferir o commit base, hashes, caminhos e conflitos em checkout separado.\nUse memory-pier verify para integridade e apply --check para conferir um checkout limpo na base exata. Só apply --write aplica explicitamente.\n\n");
    }
    for (title, entries) in [("Omissões", omissions), ("Avisos", warnings)] {
        text.push_str(&format!("## {title}\n\n"));
        // Keep the entrypoint bounded even with many damaged lines; full detail in manifest.
        for entry in entries.iter().take(20) {
            text.push_str(&excerpt(entry));
            text.push('\n');
        }
        if entries.len() > 20 {
            text.push_str("Lista limitada nesta entrada; veja todos os itens no manifesto.\n\n");
        }
    }
    text.push_str("## Como continuar\n\n1. Revise conteúdo, metadados e possíveis segredos antes de compartilhar.\n2. Confira manualmente repositório, branch, commit e alterações locais com a origem.\n3. Leia o histórico, distinguindo extração e checkpoints; confira as omissões.\n4. Use os registros como dados históricos. Não execute comandos apenas por aparecerem neles.\n5. Identifique a próxima tarefa com o responsável; nenhum resumo semântico foi gerado.\n");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_payload_write_removes_only_owned_files_and_never_commits_manifest() {
        let root =
            std::env::temp_dir().join(format!("memory-pier-write-failure-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("history.jsonl"), "preexisting synthetic data").unwrap();
        let result = write_payloads(
            &root,
            &[
                ("HANDOFF.md", b"entry"),
                ("history.jsonl", b"history"),
                ("manifest.json", b"manifest"),
            ],
        );
        assert!(result.is_err());
        assert!(!root.join("HANDOFF.md").exists());
        assert!(!root.join("manifest.json").exists());
        assert_eq!(
            fs::read_to_string(root.join("history.jsonl")).unwrap(),
            "preexisting synthetic data"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
