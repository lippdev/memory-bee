use memory_pier::claude::{Limits, ReadState, inspect};
use std::{
    env,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

fn run() -> Result<u8, (u8, String)> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!(
            "Usage: memory-pier inspect <session.jsonl>\nPrints a local JSON inspection (not a portable bundle).\nExit: 0 read/empty, 2 partial, 1 I/O error, 64 usage error."
        );
        return Ok(0);
    }
    if args.len() != 2 || args[0] != "inspect" {
        return Err((64, "Usage: memory-pier inspect <session.jsonl>".into()));
    }
    let report = inspect(Path::new(&args[1]), Limits::default())
        .map_err(|e| (1, format!("Cannot inspect session: {e}")))?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    serde_json::to_writer_pretty(&mut out, &report).map_err(|e| (1, e.to_string()))?;
    writeln!(out).map_err(|e| (1, e.to_string()))?;
    Ok(if report.state == ReadState::Partial {
        2
    } else {
        0
    })
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
