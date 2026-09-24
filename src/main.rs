use memory_pier::{
    claude::{Limits, ReadState, inspect},
    discovery::{DiscoveryLimits, discover},
    selection::select,
};
use serde::Serialize;
use std::{
    env,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

const USAGE: &str = "Usage:\n  memory-pier inspect <session.jsonl> [--leaf <uuid>]\n  memory-pier sessions --root <projects-dir> --project <project-dir>\nLocal JSON only; no export or model calls.\nExit: 0 read/empty, 2 partial, 1 I/O or selection error, 64 usage error.";
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
    Err((64, USAGE.into()))
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
