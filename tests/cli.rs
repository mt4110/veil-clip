use std::process::Command;

fn run(arguments: &[&str]) -> std::process::Output {
    // ヘルプ・版・不正引数のみ。監視を起動せずOSのクリップボードに触れない。
    // Only help, version, and rejected arguments; never launch observation.
    Command::new(env!("CARGO_BIN_EXE_veil-clip"))
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn help_and_version_succeed_without_backend_access() {
    for (arguments, expected) in [
        (vec!["--help"], "使い方:"),
        (vec!["--lang", "en", "--help"], "Usage:"),
        (vec!["--version"], "veil-clip 0.1.0"),
    ] {
        let output = run(&arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8(output.stdout).unwrap().contains(expected));
    }
}

#[test]
fn apply_and_invalid_arguments_exit_before_backend_initialization() {
    for arguments in [
        vec!["--apply"],
        vec!["--apply", "--lang", "en"],
        vec!["--lang", "sensitive-dummy-value"],
        vec!["sensitive-dummy-value"],
        vec!["--help", "sensitive-dummy-value"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!error.contains("sensitive-dummy-value"));
        assert!(!error.contains("[veil-clip]"));
    }
}
