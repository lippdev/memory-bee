use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_memory-bee"))
        .args(args)
        .output()
        .unwrap()
}

const BASE: &[&str] = &[
    "dashboard",
    "--project",
    "/synthetic/project",
    "--claude-root",
    "testdata/claude-projects",
    "--codex-root",
    "testdata/codex-sessions",
];

#[test]
fn missing_project_and_missing_roots_are_usage_errors() {
    for args in [
        vec!["dashboard"],
        vec!["dashboard", "--claude-root", "testdata/claude-projects"],
        vec!["dashboard", "--project", "/synthetic/project"],
        vec![
            "dashboard",
            "--project",
            "/synthetic/project",
            "--claude-root",
            "testdata/claude-projects",
            "--claude-root",
            "testdata/claude-projects",
        ],
        vec![
            "dashboard",
            "--project",
            "/synthetic/project",
            "--claude-root",
            "testdata/claude-projects",
            "--unknown-flag",
        ],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(64), "args: {args:?}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("memory bee"));
    }
}

#[test]
fn theme_width_and_height_are_validated() {
    let bad_theme = run(&[
        "dashboard",
        "--project",
        "/synthetic/project",
        "--claude-root",
        "testdata/claude-projects",
        "--theme",
        "purple",
    ]);
    assert_eq!(bad_theme.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&bad_theme.stderr).contains("dark or light"));

    let bad_width = run(&[
        "dashboard",
        "--project",
        "/synthetic/project",
        "--claude-root",
        "testdata/claude-projects",
        "--once",
        "--width",
        "0",
    ]);
    assert_eq!(bad_width.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&bad_width.stderr).contains("positive integer"));

    // --width/--height only make sense with --once.
    let width_without_once = run(&[
        "dashboard",
        "--project",
        "/synthetic/project",
        "--claude-root",
        "testdata/claude-projects",
        "--width",
        "80",
    ]);
    assert_eq!(width_without_once.status.code(), Some(64));
}

#[test]
fn a_missing_root_is_an_io_error_not_a_usage_error() {
    let output = run(&[
        "dashboard",
        "--project",
        "/synthetic/project",
        "--claude-root",
        "/does/not/exist",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cannot discover sessions"));
}

#[test]
fn without_once_a_non_terminal_stdout_is_refused() {
    // The test harness pipes stdout, so this exercises the same guard a
    // script or CI job would hit without --once.
    let mut args = BASE.to_vec();
    let output = run(&args);
    assert_eq!(output.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--once"));
    args.push("--once"); // sanity: the same args succeed once told not to need a TTY
    let output = run(&args);
    assert_ne!(output.status.code(), Some(64));
}

#[test]
fn once_renders_a_full_frame_of_plain_text() {
    let mut args = BASE.to_vec();
    args.extend(["--once", "--width", "100", "--height", "30"]);
    let output = run(&args);
    assert!(
        [Some(0), Some(2)].contains(&output.status.code()),
        "status: {:?} stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 30, "--once must print exactly --height lines");
    for (i, line) in lines.iter().enumerate() {
        assert_eq!(
            line.chars().count(),
            100,
            "line {i} is not padded to --width: {line:?}"
        );
    }
    assert!(text.contains("memory bee"));
    assert!(text.contains("Sessões"));
    assert!(text.contains("Detalhe"));
    // No caller-visible flags exist for export, apply or launch: the
    // dashboard cannot reach them even by typo.
    assert!(!text.to_lowercase().contains("--launch"));
}

#[test]
fn once_degrades_at_narrow_and_short_sizes_without_panicking() {
    for (w, h) in [(100, 30), (60, 20), (40, 12), (10, 4)] {
        let mut args = BASE.to_vec();
        let width = w.to_string();
        let height = h.to_string();
        args.extend(["--once", "--width", &width, "--height", &height]);
        let output = run(&args);
        assert!(
            output.status.code().is_some(),
            "{w}x{h} crashed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            [Some(0), Some(2)].contains(&output.status.code()),
            "{w}x{h}: {:?} {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn once_with_no_color_and_a_light_theme_still_renders() {
    let mut args = BASE.to_vec();
    args.extend(["--once", "--no-color", "--theme", "light"]);
    let output = run(&args);
    assert!([Some(0), Some(2)].contains(&output.status.code()));
    assert!(!String::from_utf8_lossy(&output.stdout).is_empty());
}

#[test]
fn once_with_a_bundle_still_opens_on_the_sessions_view() {
    // --once never sends keys, so the resume/verify views (reachable only
    // via 'r'/'v') are exercised by the src/tui/app.rs unit tests instead;
    // this just confirms --bundle doesn't change the default screen.
    let mut args = BASE.to_vec();
    args.extend(["--bundle", "examples/bundle-v1", "--once"]);
    let output = run(&args);
    assert!([Some(0), Some(2)].contains(&output.status.code()));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Sessões"));
}
