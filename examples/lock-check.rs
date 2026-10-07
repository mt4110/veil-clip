//! 排他のみの確認用。クリップボード操作を含みません。
//! Lock-only probe, with no clipboard operations.

#[cfg(target_os = "macos")]
#[path = "../src/instance_lock.rs"]
mod instance_lock;

fn main() -> std::process::ExitCode {
    #[cfg(target_os = "macos")]
    {
        let _lock = match instance_lock::InstanceLock::acquire() {
            Ok(lock) => lock,
            Err(error) => {
                eprintln!("[LOCK ERROR] {error}");
                return std::process::ExitCode::FAILURE;
            }
        };
        println!("[LOCKED] Enterまたは入力終端で解放 / Release with Enter or EOF");
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            return std::process::ExitCode::FAILURE;
        }
        std::process::ExitCode::SUCCESS
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("macOS専用です / macOS required");
        std::process::ExitCode::FAILURE
    }
}
