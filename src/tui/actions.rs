//! Explicit, reviewable actions. Prepared exports and apply plans retain the
//! reviewed bytes; launches re-observe the checkout using the core token guard.
use super::app::{Detail, SessionRow, load_detail};
use memory_bee::{bundle, receive, resume};
use std::path::{Path, PathBuf};

pub enum Pending {
    Export {
        prepared: Box<bundle::Prepared>,
        output: PathBuf,
    },
    Apply(receive::Plan),
    Prompt {
        request: resume::Request,
        output: PathBuf,
        token: String,
        launch: bool,
    },
}

pub struct Dialog {
    pub title: &'static str,
    pub preview: String,
    pub input: String,
    pub expected: String,
    pub pending: Option<Pending>,
    pub form: Option<Form>,
    pub scroll: u16,
}

pub enum Form {
    ExportPath,
    ExportExclusions(PathBuf),
    PromptPath { launch: bool },
}

impl Dialog {
    pub fn form(form: Form) -> Self {
        let title = match form {
            Form::ExportPath => "Exportar: caminho da pasta NOVA",
            Form::ExportExclusions(_) => {
                "Excluir linhas: números separados por vírgula (vazio = nenhuma)"
            }
            Form::PromptPath { .. } => "Retomada: caminho do arquivo NOVO de prompt",
        };
        Self {
            title,
            preview: title.into(),
            input: String::new(),
            expected: String::new(),
            pending: None,
            form: Some(form),
            scroll: 0,
        }
    }

    pub fn review(
        title: &'static str,
        preview: String,
        expected: String,
        pending: Pending,
    ) -> Self {
        Self {
            title,
            preview,
            input: String::new(),
            expected,
            pending: Some(pending),
            form: None,
            scroll: 0,
        }
    }

    pub fn message(text: String) -> Self {
        Self {
            title: "Resultado",
            preview: text,
            input: String::new(),
            expected: String::new(),
            pending: None,
            form: None,
            scroll: 0,
        }
    }
}

pub fn export(
    row: &SessionRow,
    leaf: Option<&str>,
    output: PathBuf,
    exclusions: &str,
) -> Result<Dialog, String> {
    if output.as_os_str().is_empty() {
        return Err("Informe um destino novo".into());
    }
    let mut options = bundle::Options::default();
    if !exclusions.trim().is_empty() {
        for value in exclusions.split(',') {
            let line = value
                .trim()
                .parse::<usize>()
                .map_err(|_| "Linhas devem ser números positivos separados por vírgula")?;
            if line == 0 {
                return Err("Linhas começam em 1".into());
            }
            options.exclude_lines.insert(line);
        }
    }
    let prepared = match load_detail(row, leaf)? {
        Detail::Claude(report) => bundle::prepare(report, &options)?,
        Detail::Codex(report) => bundle::prepare_codex(report, &options)?,
    };
    let preview = format!(
        "Origem: {}\nRamo: {}\nDestino: {}\nSomente contexto; sem referência Git ou código.\n\n{}",
        row.path.display(),
        leaf.unwrap_or("único / registro físico"),
        output.display(),
        serde_json::to_string_pretty(&prepared).map_err(|e| e.to_string())?
    );
    Ok(Dialog::review(
        "Revisar exportação",
        preview,
        "EXPORTAR".into(),
        Pending::Export {
            prepared: Box::new(prepared),
            output,
        },
    ))
}

pub fn apply(bundle: &Path, project: &Path) -> Result<Dialog, String> {
    let verified = receive::verify(bundle)?;
    let plan = verified.check(project)?;
    let manifest =
        std::fs::read_to_string(bundle.join("manifest.json")).map_err(|e| e.to_string())?;
    use sha2::{Digest, Sha256};
    if format!("{:x}", Sha256::digest(manifest.as_bytes())) != verified.manifest_sha256() {
        return Err("Pacote mudou durante a prévia; verifique novamente".into());
    }
    let preview = format!(
        "Pacote: {}\nCheckout: {}\n{}\n\nManifesto (caminhos selecionados):\n{}",
        bundle.display(),
        project.display(),
        serde_json::to_string_pretty(&plan.report()).map_err(|e| e.to_string())?,
        manifest
    );
    Ok(Dialog::review(
        "Revisar aplicação",
        preview,
        "APLICAR".into(),
        Pending::Apply(plan),
    ))
}

pub fn prompt(request: resume::Request, output: PathBuf, launch: bool) -> Result<Dialog, String> {
    if output.as_os_str().is_empty() {
        return Err("Informe um arquivo novo de prompt".into());
    }
    let prep = resume::prepare(&request, Some(&output))?;
    if launch {
        resume::check_launch(&prep, &prep.confirmation)?;
    }
    let expected = if launch {
        prep.confirmation.clone()
    } else {
        "GRAVAR".into()
    };
    let preview = serde_json::to_string_pretty(&prep).map_err(|e| e.to_string())?;
    Ok(Dialog::review(
        if launch {
            "Revisar lançamento"
        } else {
            "Revisar prompt"
        },
        preview,
        expected,
        Pending::Prompt {
            request,
            output,
            token: prep.confirmation,
            launch,
        },
    ))
}

/// Called only after releasing the TUI's terminal, immediately before launching.
/// Also used to save a prompt without launching; any stale preview is refused.
pub fn execute_prompt(
    request: &resume::Request,
    output: &Path,
    token: &str,
    launch: bool,
) -> Result<String, String> {
    let mut prep = resume::prepare(request, Some(output))?;
    if launch {
        resume::check_launch(&prep, token)?;
    } else if prep.confirmation != token {
        return Err("Estado mudou; gere outra prévia antes de gravar".into());
    }
    resume::write_prompt(&prep)?;
    if launch {
        resume::launch(&mut prep)
            .map_err(|e| format!("{e}\nPrompt preservado: {}", output.display()))?;
    }
    if launch {
        let status = prep
            .launch_exit_code
            .map_or_else(|| "interrompido por sinal".into(), |code| code.to_string());
        Ok(format!(
            "Prompt preservado: {}\nCódigo de saída: {status}",
            output.display()
        ))
    } else {
        Ok(format!("Prompt gravado: {}", output.display()))
    }
}
