mod cli;
#[cfg(target_os = "macos")]
mod clipboard;
#[cfg(any(target_os = "macos", test))]
mod monitor;

use std::io::{self, Write};
use std::process::ExitCode;

use cli::{Command, FatalError, Language};

fn main() -> ExitCode {
    let options = match cli::parse(std::env::args_os().skip(1).collect()) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "{}", cli::usage_error(error));
            return ExitCode::from(2);
        }
    };
    match options.command {
        Command::Help => match io::stdout()
            .lock()
            .write_all(cli::help(options.language).as_bytes())
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        },
        Command::Version => match writeln!(
            io::stdout().lock(),
            "veil-clip {}",
            env!("CARGO_PKG_VERSION")
        ) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        },
        Command::Observe => match observe(options.language) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(
                    io::stderr().lock(),
                    "{}",
                    cli::fatal_error(options.language, error)
                );
                ExitCode::FAILURE
            }
        },
    }
}

#[cfg(not(target_os = "macos"))]
fn observe(_: Language) -> Result<(), FatalError> {
    Err(FatalError::UnsupportedPlatform)
}

#[cfg(target_os = "macos")]
fn observe(language: Language) -> Result<(), FatalError> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc};
    use std::time::Instant;

    let stopped = Arc::new(AtomicBool::new(false));
    let signal_stop = Arc::clone(&stopped);
    let (sender, receiver) = mpsc::sync_channel(1);
    ctrlc::set_handler(move || {
        signal_stop.store(true, Ordering::SeqCst);
        let _ = sender.try_send(());
    })
    .map_err(|_| FatalError::SignalHandler)?;

    let mut reader =
        clipboard::MacClipboard::new().map_err(|_| FatalError::ClipboardInitialization)?;
    let mut monitor = monitor::Monitor::new().map_err(|_| FatalError::Monitor)?;
    let mut logger = monitor::EventLogger::new(language);
    let mut output = io::stderr().lock();
    writeln!(output, "{}", cli::startup(language)).map_err(|_| FatalError::Monitor)?;
    loop {
        let events = monitor
            .poll(&mut reader, &stopped, Instant::now)
            .map_err(|_| FatalError::Monitor)?;
        if monitor.take_recovery() {
            logger
                .recovered(&mut output)
                .map_err(|_| FatalError::Monitor)?;
        }
        logger
            .emit(events, Instant::now(), &mut output)
            .map_err(|_| FatalError::Monitor)?;
        if stopped.load(Ordering::SeqCst) {
            logger
                .emit(monitor.stop(), Instant::now(), &mut output)
                .map_err(|_| FatalError::Monitor)?;
            return Ok(());
        }
        let delay = monitor::wait_duration(Instant::now(), monitor.deadline());
        wait_for_wake(&receiver, delay).map_err(|_| FatalError::ShutdownChannel)?;
    }
}

#[cfg(any(target_os = "macos", test))]
fn wait_for_wake(
    receiver: &std::sync::mpsc::Receiver<()>,
    delay: std::time::Duration,
) -> Result<(), ()> {
    match receiver.recv_timeout(delay) {
        Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(()),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(()),
    }
}

#[cfg(test)]
mod shutdown_tests {
    use super::wait_for_wake;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn queued_stop_wakes_a_long_wait_without_sleeping() {
        let (sender, receiver) = mpsc::sync_channel(1);
        sender.try_send(()).unwrap();
        assert!(wait_for_wake(&receiver, Duration::from_secs(60)).is_ok());
        assert!(wait_for_wake(&receiver, Duration::ZERO).is_ok());
        drop(sender);
        assert!(wait_for_wake(&receiver, Duration::ZERO).is_err());
    }
}
