use std::ffi::OsString;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Japanese,
    English,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Observe,
    Help,
    Version,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub language: Language,
    pub command: Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageErrorKind {
    InvalidArguments,
    InvalidLanguage,
    ApplyUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsageError {
    pub language: Language,
    pub kind: UsageErrorKind,
}

// 不正な引数も秘密情報の可能性があるため、値をエラーに含めない。
// Invalid arguments may contain secrets; never echo their values.
pub fn parse(arguments: Vec<OsString>) -> Result<Options, UsageError> {
    let mut language = Language::Japanese;
    for pair in arguments.windows(2) {
        if pair[0] == "--lang" {
            match pair[1].to_str() {
                Some("ja") => language = Language::Japanese,
                Some("en") => language = Language::English,
                _ => {}
            }
        }
    }
    let error = |kind| UsageError { language, kind };
    let mut command = Command::Observe;
    let mut language_seen = false;
    let mut command_seen = false;
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--lang") => {
                if language_seen {
                    return Err(error(UsageErrorKind::InvalidArguments));
                }
                language_seen = true;
                match arguments.next().and_then(|value| value.to_str()) {
                    Some("ja" | "en") => {}
                    _ => return Err(error(UsageErrorKind::InvalidLanguage)),
                }
            }
            Some("--help" | "-h" | "--version" | "-V") => {
                if command_seen {
                    return Err(error(UsageErrorKind::InvalidArguments));
                }
                command_seen = true;
                command = match argument.to_str() {
                    Some("--help" | "-h") => Command::Help,
                    _ => Command::Version,
                };
            }
            Some("--apply") => return Err(error(UsageErrorKind::ApplyUnavailable)),
            _ => return Err(error(UsageErrorKind::InvalidArguments)),
        }
    }
    Ok(Options { language, command })
}

pub fn usage_error(error: UsageError) -> &'static str {
    match (error.language, error.kind) {
        (Language::Japanese, UsageErrorKind::ApplyUnavailable) => {
            "上書きモードは未実装です。--applyは使用できません。"
        }
        (Language::English, UsageErrorKind::ApplyUnavailable) => {
            "Apply mode is not implemented. --apply is unavailable."
        }
        (Language::Japanese, UsageErrorKind::InvalidLanguage) => {
            "--langにはjaまたはenを指定してください。"
        }
        (Language::English, UsageErrorKind::InvalidLanguage) => "--lang requires ja or en.",
        (Language::Japanese, UsageErrorKind::InvalidArguments) => {
            "引数が不正です。--helpで使い方を確認してください。"
        }
        (Language::English, UsageErrorKind::InvalidArguments) => {
            "Invalid arguments. See --help for usage."
        }
    }
}

pub fn help(language: Language) -> &'static str {
    match language {
        Language::Japanese => concat!(
            "veil-clip — クリップボードの機密情報を監視\n\n",
            "使い方: veil-clip [--lang ja|en] [--help | --version]\n\n",
            "  --lang ja|en   表示言語（既定: ja）\n",
            "  -h, --help     このヘルプを表示\n",
            "  -V, --version  バージョンを表示\n\n",
            "macOS専用の監視モードです。500ms間隔でテキストを読み取り、\n",
            "AWSキーIDと秘密鍵ヘッダーを検知します。検知から5秒後も上書きしません。\n",
            "本文・抜粋・ハッシュを出力せず、通知は標準エラーへ表示します。\n",
            "同種の通知は30秒間抑制します。内容変更の検知で対象通知の抑制を解除します。\n",
            "クリップボードを変更・消去せず、履歴や同期済みコピーも保護しません。\n",
            "Ctrl+Cで停止します。OS読取の遅延により停止が遅れる可能性があります。\n",
            "--apply、自動起動は未実装です。\n"
        ),
        Language::English => concat!(
            "veil-clip — Observe potentially sensitive clipboard text\n\n",
            "Usage: veil-clip [--lang ja|en] [--help | --version]\n\n",
            "  --lang ja|en   Display language (default: ja)\n",
            "  -h, --help     Show this help\n",
            "  -V, --version  Show version\n\n",
            "macOS observation mode. Read text every 500ms and detect AWS key IDs\n",
            "and private-key headers. No replacement occurs after the five-second TTL.\n",
            "Events go to stderr without content, excerpts, or hashes.\n",
            "Repeat events are suppressed for 30 seconds; observed content changes\n",
            "reset target notification suppression. This is not a complete copy history.\n",
            "The clipboard is never changed or erased; history and synced copies are not protected.\n",
            "Stop with Ctrl+C; blocked OS reads may delay shutdown.\n",
            "--apply and automatic startup are not implemented.\n"
        ),
    }
}

#[cfg(target_os = "macos")]
pub fn startup(language: Language) -> &'static str {
    match language {
        Language::Japanese => "[veil-clip] 監視開始（macOS）。500ms間隔でテキストを読み取ります。TTLは5秒ですが、上書き・消去は行いません。AWSキーID・秘密鍵ヘッダーを検知し、本文は出力しません。同種通知を30秒間抑制します。履歴・同期済みコピーは保護しません。OS読取の遅延により通知・停止が遅れる可能性があります。Ctrl+Cで停止します。",
        Language::English => "[veil-clip] Observing on macOS. Poll text every 500ms; TTL is 5 seconds, with no replacement or erasure. Detect AWS key IDs and private-key headers without printing content. Repeat events are suppressed for 30 seconds. History and synced copies are not protected. OS reads may delay notifications and shutdown. Stop with Ctrl+C.",
    }
}

#[derive(Clone, Copy)]
pub enum FatalError {
    #[cfg(not(target_os = "macos"))]
    UnsupportedPlatform,
    #[cfg(target_os = "macos")]
    ClipboardInitialization,
    #[cfg(target_os = "macos")]
    SignalHandler,
    #[cfg(target_os = "macos")]
    Monitor,
    #[cfg(target_os = "macos")]
    ShutdownChannel,
}

pub fn fatal_error(language: Language, error: FatalError) -> &'static str {
    match (language, error) {
        #[cfg(not(target_os = "macos"))]
        (Language::Japanese, FatalError::UnsupportedPlatform) => {
            "[ERROR] 監視CLIは現在macOS専用です。"
        }
        #[cfg(not(target_os = "macos"))]
        (Language::English, FatalError::UnsupportedPlatform) => {
            "[ERROR] Observation currently requires macOS."
        }
        #[cfg(target_os = "macos")]
        (Language::Japanese, FatalError::ClipboardInitialization) => {
            "[ERROR] クリップボードを初期化できません。監視を開始しませんでした。"
        }
        #[cfg(target_os = "macos")]
        (Language::English, FatalError::ClipboardInitialization) => {
            "[ERROR] Could not initialize the clipboard. Observation did not start."
        }
        #[cfg(target_os = "macos")]
        (Language::Japanese, FatalError::SignalHandler) => {
            "[ERROR] 終了処理を準備できません。監視を開始しませんでした。"
        }
        #[cfg(target_os = "macos")]
        (Language::English, FatalError::SignalHandler) => {
            "[ERROR] Could not register shutdown handling. Observation did not start."
        }
        #[cfg(target_os = "macos")]
        (Language::Japanese, FatalError::Monitor) => {
            "[ERROR] 監視処理が失敗しました。監視を停止します。"
        }
        #[cfg(target_os = "macos")]
        (Language::English, FatalError::Monitor) => {
            "[ERROR] Observation failed. Stopping the monitor."
        }
        #[cfg(target_os = "macos")]
        (Language::Japanese, FatalError::ShutdownChannel) => {
            "[ERROR] 終了通知を受信できません。監視を停止します。"
        }
        #[cfg(target_os = "macos")]
        (Language::English, FatalError::ShutdownChannel) => {
            "[ERROR] Shutdown notification is unavailable. Stopping the monitor."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn defaults_to_japanese_observation_and_accepts_safe_commands() {
        assert_eq!(
            parse(vec![]).unwrap(),
            Options {
                language: Language::Japanese,
                command: Command::Observe
            }
        );
        for flags in [["--lang", "en", "--help"], ["--help", "--lang", "en"]] {
            assert_eq!(
                parse(args(&flags)).unwrap(),
                Options {
                    language: Language::English,
                    command: Command::Help
                }
            );
        }
        assert_eq!(parse(args(&["-V"])).unwrap().command, Command::Version);
    }

    #[test]
    fn rejects_apply_duplicates_and_invalid_input_without_echoing_values() {
        for flags in [
            vec!["--apply", "--lang", "en"],
            vec!["--help", "--version"],
            vec!["--lang", "en", "--lang", "ja"],
            vec!["--lang"],
            vec!["--lang", "sensitive-dummy-value"],
            vec!["sensitive-dummy-value"],
            vec!["--help", "sensitive-dummy-value"],
        ] {
            let error = parse(args(&flags)).unwrap_err();
            assert!(!usage_error(error).contains("sensitive-dummy-value"));
        }
        let error = parse(args(&["--apply", "--lang", "en"])).unwrap_err();
        assert_eq!(error.language, Language::English);
        assert_eq!(error.kind, UsageErrorKind::ApplyUnavailable);
    }

    #[cfg(unix)]
    #[test]
    fn invalid_utf8_is_rejected_without_echoing() {
        use std::os::unix::ffi::OsStringExt;
        assert!(parse(vec![OsString::from_vec(vec![0xff])]).is_err());
    }
}
