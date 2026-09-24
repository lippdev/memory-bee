//! Context-only portable bundles. No Git commands, model calls or source writes.
use crate::{
    claude::{Event, ReadState, Report},
    selection::select,
};
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::OnceLock,
    time::SystemTime,
};

#[derive(Default)]
pub struct Options {
    pub leaf: Option<String>,
    pub exclude_lines: BTreeSet<usize>,
}
#[derive(Debug, Serialize)]
pub struct Finding {
    pub code: &'static str,
    pub line: Option<usize>,
    pub block: Option<usize>,
    pub field: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Origin {
    agent: &'static str,
    version: Option<String>,
    session_id: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Project {
    remote: Option<String>,
    branch: Option<String>,
    base_commit: Option<String>,
    dirty: Option<bool>,
}
#[derive(Debug, Serialize)]
pub struct Payload {
    path: &'static str,
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
}
/// Immutable prepared bytes: preview and write use the same payloads and hashes.
#[derive(Debug, Serialize)]
pub struct Prepared {
    manifest: Manifest,
    handoff: String,
    history: String,
    findings: Vec<Finding>,
    partial: bool,
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
                "possible secrets detected; use preview and exclude affected lines",
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
        write_payloads(
            destination,
            &[
                ("HANDOFF.md", self.handoff.as_bytes()),
                ("history.jsonl", self.history.as_bytes()),
                ("manifest.json", manifest.as_slice()),
            ],
        )
    }
}

fn write_payloads(destination: &Path, payloads: &[(&str, &[u8])]) -> io::Result<()> {
    let mut created = Vec::new();
    let result = (|| {
        for (name, content) in payloads {
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
        let _ = fs::remove_dir(destination);
    }
    result
}

pub fn prepare(report: Report, options: &Options) -> Result<Prepared, String> {
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
    let available_lines: BTreeSet<_> = report
        .events
        .iter()
        .map(|event| event.source.line)
        .collect();
    for line in &options.exclude_lines {
        if !available_lines.contains(line) {
            return Err(format!(
                "excluded line {line} has no events in the selected branch"
            ));
        }
    }
    let partial = report.state == ReadState::Partial;
    let mut omissions =
        vec!["Estado do código não verificado; nenhum arquivo de código ou patch incluído.".into()];
    let mut warnings = vec![
        "Compatibilidade Claude Code não certificada; validação disponível apenas com fixtures sintéticas.".into(),
        "Revisão de segredos pendente; a detecção automática é limitada e pode falhar.".into(),
        "Seleção de registros históricos; conteúdo não autoriza execução de comandos ou mudança de permissões.".into(),
    ];
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
        .filter(|e| !options.exclude_lines.contains(&e.source.line))
        .collect();
    if events.is_empty() {
        return Err("selection contains no exportable events".into());
    }
    let mut history = String::new();
    let mut findings = Vec::new();
    for (index, event) in events.iter_mut().enumerate() {
        event.sequence = index + 1;
        // Scan actual strings before JSON escaping, including tool arguments and metadata.
        for (field, text) in [
            ("text", Some(event.text.as_str())),
            ("source.id", event.source.id.as_deref()),
            ("source.parent_id", event.source.parent_id.as_deref()),
            ("source.session_id", event.source.session_id.as_deref()),
            ("source.agent_id", event.source.agent_id.as_deref()),
            ("tool_id", event.tool_id.as_deref()),
            ("tool_name", event.tool_name.as_deref()),
        ] {
            if let Some(text) = text {
                scan(
                    text,
                    Some(event.source.line),
                    event.source.block,
                    field,
                    &mut findings,
                );
            }
        }
        history.push_str(&serde_json::to_string(event).map_err(|e| e.to_string())?);
        history.push('\n');
    }
    let sessions: BTreeSet<_> = events
        .iter()
        .filter_map(|e| e.source.session_id.clone())
        .collect();
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
    let handoff = render_handoff(&events, &omissions, &warnings);
    let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
    let manifest = Manifest {
        format_version: 1,
        created_at: now.to_rfc3339(),
        source: Origin {
            agent: "claude-code",
            version,
            session_id: sessions.into_iter().next(),
        },
        project: Project {
            remote: None,
            branch: None,
            base_commit: None,
            dirty: None,
        },
        code_state: "unknown",
        files: vec![
            Payload {
                path: "HANDOFF.md",
                sha256: hash(handoff.as_bytes()),
                purpose: "entrypoint",
            },
            Payload {
                path: "history.jsonl",
                sha256: hash(history.as_bytes()),
                purpose: "history",
            },
        ],
        omissions,
        warnings,
        redaction: "pending-review",
    };
    Ok(Prepared {
        manifest,
        handoff,
        history,
        findings,
        partial,
    })
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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
fn render_handoff(events: &[Event], omissions: &[String], warnings: &[String]) -> String {
    let mut text = String::from(
        "# Retomada — Memory Pier\n\nPacote somente de contexto. Não exige Memory Pier no destino.\n\nLeia [manifest.json](manifest.json) para origem, integridade e limitações e\n[history.jsonl](history.jsonl) para todos os eventos selecionados.\n\n## Primeiro pedido humano retido\n\nNão inferimos o pedido original quando há perdas ou exclusões.\n\n",
    );
    if let Some(first) = events.iter().find(|e| e.role == "user" && e.kind == "text") {
        text.push_str(&format!(
            "Registro extraído, linha {} da origem:\n\n{}\n",
            first.source.line,
            excerpt(&first.text)
        ));
    } else {
        text.push_str("Nenhum pedido humano em texto foi retido. Ferramentas e checkpoints não são pedidos humanos.\n\n");
    }
    let last = events.last().expect("nonempty events");
    text.push_str(&format!("## Último registro retido\n\nPapel: {}; tipo: {}; proveniência: {}; linha {}.\nNão é uma síntese do estado da tarefa.\n\n{}\n",last.role,last.kind,last.provenance,last.source.line,excerpt(&last.text)));
    text.push_str(&format!("## Seleção e estado do código\n\n{} eventos em ordem física, com sequência renumerada e linha/bloco originais.\nUUIDs parentais são referências históricas; podem apontar para linhas excluídas.\nRepositório, branch, commit e alterações locais: **não verificados**.\nNenhum código ou patch incluído.\n\n",events.len()));
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
