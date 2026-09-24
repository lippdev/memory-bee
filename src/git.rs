//! Explicit, local Git observation. Never fetches, changes files or exports paths.
use serde::Serialize;
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Debug, Default, Serialize)]
pub struct Project {
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub base_commit: Option<String>,
    pub dirty: Option<bool>,
}

pub struct Observation {
    pub project: Project,
    pub warnings: Vec<String>,
    pub partial: bool,
}

// Bound captured output, discard stderr (it may contain private paths/configuration).
pub(crate) fn run(path: &Path, args: &[&str]) -> Result<(i32, Vec<u8>), ()> {
    let mut command = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    // Status can otherwise execute configured clean/process filters while comparing files.
    if args.first() == Some(&"status") {
        let (status, keys) = run(
            path,
            &[
                "config",
                "--name-only",
                "--get-regexp",
                r"^filter\..*\.(clean|smudge|process|required)$",
            ],
        )?;
        if status != 0 && status != 1 {
            return Err(());
        }
        for key in std::str::from_utf8(&keys).map_err(|_| ())?.lines() {
            let value = if key.ends_with(".required") {
                "false"
            } else {
                ""
            };
            command.arg("-c").arg(format!("{key}={value}"));
        }
    }
    let mut child = command
        .args([
            "--no-optional-locks",
            "--literal-pathspecs",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-C",
        ])
        .arg(path)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    let mut bytes = Vec::new();
    let result = child
        .stdout
        .take()
        .ok_or(())?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes);
    if result.is_err() || bytes.len() > 1024 * 1024 {
        let _ = child.kill();
        let _ = child.wait();
        return Err(());
    }
    let status = child.wait().map_err(|_| ())?;
    Ok((status.code().unwrap_or(-1), bytes))
}
pub(crate) fn text(path: &Path, args: &[&str]) -> Result<String, ()> {
    let (code, bytes) = run(path, args)?;
    if code != 0 {
        return Err(());
    }
    String::from_utf8(bytes)
        .map(|s| s.trim_end_matches('\n').to_owned())
        .map_err(|_| ())
}

pub fn inspect(path: &Path) -> Observation {
    let mut observation = Observation { project: Project::default(), warnings: vec![
        "Git observado no projeto escolhido explicitamente; não comprova vínculo com a sessão nem é snapshot atômico.".into()
    ], partial: false };
    let root = match text(path, &["rev-parse", "--show-toplevel"]) {
        Ok(root) => root,
        Err(()) => {
            observation.partial = true;
            observation.warnings.push("git_unavailable: projeto ausente, sem working tree ou Git indisponível; estado desconhecido.".into());
            return observation;
        }
    };
    let root = Path::new(&root);
    let branch = run(root, &["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let detached = matches!(&branch, Ok((1, _)));
    observation.project.branch = match branch {
        Ok((0, bytes)) => String::from_utf8(bytes)
            .ok()
            .map(|s| s.trim_end_matches('\n').to_owned()),
        _ => None,
    };
    if observation.project.branch.is_none() && !detached {
        observation.partial = true;
        observation
            .warnings
            .push("git_branch_unavailable: branch não legível.".into());
    }
    observation.project.base_commit = text(root, &["rev-parse", "--verify", "HEAD^{commit}"])
        .ok()
        .filter(|s| {
            [40, 64].contains(&s.len())
                && s.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        });
    if observation.project.base_commit.is_none() {
        observation.partial = true;
        observation.warnings.push(
            "git_head_unavailable: repositório sem commit ou HEAD não legível; base desconhecida."
                .into(),
        );
    } else if detached {
        observation
            .warnings
            .push("git_detached_head: sem branch simbólica; use o commit base.".into());
    }
    match run(
        root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=normal",
            "--ignore-submodules=all",
        ],
    ) {
        Ok((0, bytes)) => observation.project.dirty = Some(!bytes.is_empty()),
        _ => {
            observation.partial = true;
            observation
                .warnings
                .push("git_status_unavailable: alterações locais não verificadas.".into());
        }
    }
    // Do not recurse into submodules: their own filter configuration could execute.
    match run(root, &["ls-files", "--stage", "-z"]) {
        Ok((0, bytes))
            if !bytes
                .split(|b| *b == 0)
                .any(|record| record.starts_with(b"160000 ")) => {}
        _ => {
            if observation.project.dirty != Some(true) {
                observation.project.dirty = None;
            }
            observation.partial = true;
            observation.warnings.push("git_submodules_unverified: submódulos ou inventário indisponível; estado interno não consultado.".into());
        }
    }
    match run(root, &["config", "--get", "remote.origin.url"]) {
        Ok((0, bytes)) => {
            observation.project.remote = String::from_utf8(bytes)
                .ok()
                .and_then(|s| safe_remote(s.trim_end_matches('\n')));
            if observation.project.remote.is_none() {
                observation.warnings.push(
                    "git_remote_omitted: origin local, sensível ou não suportado; URL omitida."
                        .into(),
                );
            }
        }
        Ok((1, _)) => observation
            .warnings
            .push("git_remote_absent: origin não configurado.".into()),
        _ => {
            observation.partial = true;
            observation
                .warnings
                .push("git_remote_unavailable: origin não legível.".into());
        }
    }
    // Detect common concurrent checkout/commit changes; status remains non-atomic.
    if text(root, &["rev-parse", "--verify", "HEAD^{commit}"]).ok()
        != observation.project.base_commit
        || text(root, &["symbolic-ref", "--quiet", "--short", "HEAD"]).ok()
            != observation.project.branch
    {
        observation.project = Project::default();
        observation.partial = true;
        observation.warnings.push(
            "git_changed_during_read: HEAD mudou durante leitura; referência descartada.".into(),
        );
    }
    observation
}

// Conservative allowlist: no userinfo, queries, fragments, escapes or local paths.
fn safe_remote(value: &str) -> Option<String> {
    let normalized = if let Some(scp) = value.strip_prefix("git@") {
        let (host, path) = scp.split_once(':')?;
        format!("ssh://{host}/{path}")
    } else if let Some(ssh) = value.strip_prefix("ssh://git@") {
        format!("ssh://{ssh}")
    } else {
        value.to_owned()
    };
    let rest = normalized
        .strip_prefix("https://")
        .or_else(|| normalized.strip_prefix("ssh://"))?;
    let (host, path) = rest.split_once('/')?;
    if host.is_empty()
        || path.is_empty()
        || !rest
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_/:".contains(&b))
    {
        return None;
    }
    Some(normalized)
}
