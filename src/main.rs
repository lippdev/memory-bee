use memory_pier::{
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

const USAGE: &str = "Usage:\n  memory-pier inspect-codex <rollout.jsonl>\n  memory-pier inspect <session.jsonl> [--leaf <uuid>]\n  memory-pier sessions --root <projects-dir> --project <project-dir>\n  memory-pier export <session.jsonl> (--preview | --output <new-dir>) [--leaf <uuid>] [--exclude-line <n>]... [--project <project-dir>] [--include-path <relative-file>]...\n  memory-pier verify <bundle-dir>\n  memory-pier apply <bundle-dir> --project <checkout> (--check | --write)\nOffline; no model calls.\nExit: 0 success, 2 partial, 3 possible secrets, 1 I/O or selection error, 64 usage error.";
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
        let report = memory_pier::codex::inspect(Path::new(&args[1]), Limits::default())
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
    if args.len() == 5 && args[0] == "sessions" {
        let (root, project) = if args[1] == "--root" && args[3] == "--project" {
            (&args[2], &args[4])
        } else if args[1] == "--project" && args[3] == "--root" {
            (&args[4], &args[2])
        } else {
            return Err((64, USAGE.into()));
        };
        let report = discover(
            Path::new(root),
            Path::new(project),
            DiscoveryLimits::default(),
        )
        .map_err(|e| (1, format!("Cannot discover sessions: {e}")))?;
        output(&report)?;
        return Ok(if report.partial { 2 } else { 0 });
    }
    if args.len() == 2 && args[0] == "verify" {
        let verified = memory_pier::receive::verify(Path::new(&args[1])).map_err(|e| (1, e))?;
        output(&verified.report())?;
        return Ok(0);
    }
    if args.len() == 5
        && args[0] == "apply"
        && args[2] == "--project"
        && (args[4] == "--check" || args[4] == "--write")
    {
        let verified = memory_pier::receive::verify(Path::new(&args[1])).map_err(|e| (1, e))?;
        let plan = verified.check(Path::new(&args[3])).map_err(|e| (1, e))?;
        let report = if args[4] == "--write" {
            plan.write().map_err(|e| (1, e))?
        } else {
            plan.report()
        };
        output(&report)?;
        return Ok(0);
    }
    if args.first().is_some_and(|arg| arg == "export") {
        return export(&args[1..]);
    }
    Err((64, USAGE.into()))
}

fn export(args: &[std::ffi::OsString]) -> Result<u8, (u8, String)> {
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
    if preview == destination.is_some()
        || (!options.include_paths.is_empty() && options.project.is_none())
    {
        return Err((64, USAGE.into()));
    }
    let report = inspect(Path::new(&args[0]), Limits::default())
        .map_err(|e| (1, format!("Cannot inspect session: {e}")))?;
    let prepared =
        prepare(report, &options).map_err(|e| (1, format!("Cannot prepare bundle: {e}")))?;
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
