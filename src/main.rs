use memory_bee::{
    bundle::{Options, prepare},
    claude::{Limits, ReadState, inspect},
    discovery::{DiscoveryLimits, discover},
    selection::select,
};
use serde::Serialize;
use std::{
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const USAGE: &str = "Usage:\n  memory-bee export-codex <rollout.jsonl> (--preview | --output <new-dir>) [--exclude-line <n>]... [--project <project-dir>] [--include-path <relative-file>]...\n  memory-bee inspect-codex <rollout.jsonl>\n  memory-bee inspect <session.jsonl> [--leaf <uuid>]\n  memory-bee sessions-codex --root <sessions-dir> --project <project-dir>\n  memory-bee sessions --root <projects-dir> --project <project-dir>\n  memory-bee export <session.jsonl> (--preview | --output <new-dir>) [--leaf <uuid>] [--exclude-line <n>]... [--project <project-dir>] [--include-path <relative-file>]...\n  memory-bee verify <bundle-dir>\n  memory-bee apply <bundle-dir> --project <checkout> (--check | --write)\n  memory-bee prepare-resume <bundle-dir> --target (claude | codex) --project <project-dir> [--worktree <new-dir>] (--preview | --output <new-prompt-file> [--launch <confirmation>])\nOffline; no model calls.\nLaunches an agent only with --launch and a matching confirmation from --preview.\nExit: 0 success, 2 partial or attention needed, 3 possible secrets, 4 launched agent exited non-zero, 1 I/O or selection error, 64 usage error.";
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
