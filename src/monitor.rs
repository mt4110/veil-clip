use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use veil_clip::detector::DetectedRules;
use veil_clip::engine::{
    Action, CancelReason, Engine, Event, Mode, ReadErrorKind, ReadOutcome, SkipReason,
};
use veil_clip::POLL_INTERVAL;

use crate::cli::Language;

/// 読取しか提供しない境界 / Backend boundary with no write operation.
pub trait TextReader {
    fn read_text(&mut self) -> ReadOutcome;
}

pub struct Monitor {
    engine: Engine,
    last_read_failed: bool,
    recovery_pending: bool,
}

impl Monitor {
    pub fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            engine: Engine::new(Mode::Observe)?,
            last_read_failed: false,
            recovery_pending: false,
        })
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.engine.deadline()
    }

    pub fn stop(&mut self) -> Vec<Event> {
        self.recovery_pending = false;
        self.engine.stop().events
    }

    pub fn take_recovery(&mut self) -> bool {
        std::mem::take(&mut self.recovery_pending)
    }

    fn record_read(&mut self, outcome: &ReadOutcome) {
        let failed = matches!(outcome, ReadOutcome::Failed(_));
        if self.last_read_failed && !failed {
            self.recovery_pending = true;
        }
        self.last_read_failed = failed;
    }

    pub fn poll(
        &mut self,
        reader: &mut impl TextReader,
        stopped: &AtomicBool,
        mut now: impl FnMut() -> Instant,
    ) -> Result<Vec<Event>, ()> {
        if stopped.load(Ordering::SeqCst) {
            return Ok(self.stop());
        }
        let outcome = reader.read_text();
        if stopped.load(Ordering::SeqCst) {
            return Ok(self.stop());
        }
        // OSの読取完了後に時刻を取得する / Sample time after the backend read.
        self.record_read(&outcome);
        let mut step = self.engine.observe(now(), outcome).map_err(|_| ())?;
        if step.action == Action::ReadAgain {
            if stopped.load(Ordering::SeqCst) {
                return Ok(self.stop());
            }
            let fresh = reader.read_text();
            if stopped.load(Ordering::SeqCst) {
                return Ok(self.stop());
            }
            self.record_read(&fresh);
            let mut confirmed = self.engine.confirm(now(), fresh).map_err(|_| ())?;
            step.events.append(&mut confirmed.events);
            step.action = confirmed.action;
        }
        if step.action != Action::None {
            // コアの変更で書込要求が出ても実行しない / Reject unexpected write requests.
            self.stop();
            return Err(());
        }
        Ok(step.events)
    }
}

pub fn wait_duration(now: Instant, deadline: Option<Instant>) -> Duration {
    deadline.map_or(POLL_INTERVAL, |deadline| {
        POLL_INTERVAL.min(deadline.saturating_duration_since(now))
    })
}

const LOG_INTERVAL: Duration = Duration::from_secs(30);

// Eventの種類は有限。本文やハッシュは保存しない。
// Event categories are finite; keep neither text nor hashes.
pub struct EventLogger {
    language: Language,
    recent: Vec<(Event, Instant)>,
}

impl EventLogger {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            recent: vec![],
        }
    }

    pub fn recovered(&mut self, output: &mut impl Write) -> io::Result<()> {
        self.recent.retain(|(event, _)| {
            !matches!(
                event,
                Event::InputSkipped(SkipReason::ReadFailed(_))
                    | Event::Cancelled(CancelReason::Skipped(SkipReason::ReadFailed(_)))
            )
        });
        writeln!(
            output,
            "{}",
            match self.language {
                Language::Japanese => "[RECOVERED] クリップボードの読み取りが復旧しました。",
                Language::English => "[RECOVERED] Clipboard reading recovered.",
            }
        )
    }

    pub fn emit(
        &mut self,
        events: Vec<Event>,
        now: Instant,
        output: &mut impl Write,
    ) -> io::Result<()> {
        for event in events {
            if matches!(event, Event::Cancelled(_)) {
                self.recent.retain(|(event, _)| {
                    !matches!(event, Event::Detected(_) | Event::ObservedExpiry(_))
                });
            }
            if event != Event::Stopped {
                if let Some((_, previous)) = self.recent.iter_mut().find(|(seen, _)| *seen == event)
                {
                    if now.saturating_duration_since(*previous) < LOG_INTERVAL {
                        continue;
                    }
                    *previous = now;
                } else {
                    self.recent.push((event, now));
                }
            }
            writeln!(output, "{}", event_message(self.language, event))?;
        }
        Ok(())
    }
}

fn rules_name(rules: DetectedRules) -> &'static str {
    match (rules.aws_access_key_id, rules.private_key_header) {
        (true, true) => "aws-access-key-id, private-key-header",
        (true, false) => "aws-access-key-id",
        (false, true) => "private-key-header",
        _ => "none",
    }
}

fn event_message(language: Language, event: Event) -> String {
    match (language, event) {
        (Language::Japanese, Event::Detected(rules)) => format!(
            "[DETECTED] 機密情報らしい形式を検知: {}（監視のみ）",
            rules_name(rules)
        ),
        (Language::English, Event::Detected(rules)) => format!(
            "[DETECTED] Potentially sensitive format: {} (observation only)",
            rules_name(rules)
        ),
        (Language::Japanese, Event::ObservedExpiry(_)) => {
            "[TTL] 上書き予定時刻に到達。監視モードのため未変更。".into()
        }
        (Language::English, Event::ObservedExpiry(_)) => {
            "[TTL] Replacement deadline reached. Observation mode: no change made.".into()
        }
        (Language::Japanese, Event::Cancelled(reason)) => format!(
            "[CANCELLED] 対象の期限を取り消しました: {}",
            cancel_name(language, reason)
        ),
        (Language::English, Event::Cancelled(reason)) => format!(
            "[CANCELLED] Target deadline cancelled: {}",
            cancel_name(language, reason)
        ),
        (Language::Japanese, Event::InputSkipped(reason)) => format!(
            "[WARN] 今回の入力は検査できません: {}",
            skip_name(language, reason)
        ),
        (Language::English, Event::InputSkipped(reason)) => format!(
            "[WARN] Input could not be scanned: {}",
            skip_name(language, reason)
        ),
        (Language::Japanese, Event::Stopped) => {
            "[STOPPED] 監視を停止しました。クリップボードは変更していません。".into()
        }
        (Language::English, Event::Stopped) => {
            "[STOPPED] Observation stopped. The clipboard was not changed.".into()
        }
        (Language::Japanese, _) => "[ERROR] 監視モードでは扱えない操作結果です。".into(),
        (Language::English, _) => {
            "[ERROR] Unexpected operation result for observation mode.".into()
        }
    }
}

fn cancel_name(language: Language, reason: CancelReason) -> &'static str {
    match reason {
        CancelReason::ContentChanged => match language {
            Language::Japanese => "内容変更",
            Language::English => "content changed",
        },
        CancelReason::Skipped(reason) => skip_name(language, reason),
    }
}

fn skip_name(language: Language, reason: SkipReason) -> &'static str {
    match (language, reason) {
        (Language::Japanese, SkipReason::Empty) => "空のテキスト",
        (Language::English, SkipReason::Empty) => "empty text",
        (Language::Japanese, SkipReason::NonText) => "空または非テキスト",
        (Language::English, SkipReason::NonText) => "empty or non-text",
        (Language::Japanese, SkipReason::Oversized) => "1MiBの上限超過",
        (Language::English, SkipReason::Oversized) => "exceeds 1MiB limit",
        (Language::Japanese, SkipReason::ReadFailed(ReadErrorKind::Occupied)) => {
            "クリップボード占有中"
        }
        (Language::English, SkipReason::ReadFailed(ReadErrorKind::Occupied)) => {
            "clipboard occupied"
        }
        (Language::Japanese, SkipReason::ReadFailed(ReadErrorKind::Unavailable)) => {
            "クリップボード利用不可"
        }
        (Language::English, SkipReason::ReadFailed(ReadErrorKind::Unavailable)) => {
            "clipboard unavailable"
        }
        (Language::Japanese, SkipReason::ReadFailed(ReadErrorKind::Other)) => {
            "読取または形式変換の失敗"
        }
        (Language::English, SkipReason::ReadFailed(ReadErrorKind::Other)) => {
            "read or conversion failure"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use veil_clip::TTL;

    const DUMMY: &str = "AKIA0000000000000000";

    struct FakeReader<'a> {
        inputs: VecDeque<ReadOutcome>,
        calls: usize,
        stop_during_read: Option<&'a AtomicBool>,
    }
    impl TextReader for FakeReader<'_> {
        fn read_text(&mut self) -> ReadOutcome {
            self.calls += 1;
            if let Some(flag) = self.stop_during_read {
                flag.store(true, Ordering::SeqCst);
            }
            self.inputs.pop_front().expect("synthetic input required")
        }
    }
    fn reader(values: &[&str]) -> FakeReader<'static> {
        FakeReader {
            inputs: values
                .iter()
                .map(|value| ReadOutcome::Text((*value).into()))
                .collect(),
            calls: 0,
            stop_during_read: None,
        }
    }

    #[test]
    fn polling_expiry_requests_a_second_read_and_reports_no_change() {
        let mut monitor = Monitor::new().unwrap();
        let mut backend = reader(&[DUMMY, DUMMY, DUMMY]);
        let stop = AtomicBool::new(false);
        let now = Instant::now();
        assert!(matches!(
            monitor
                .poll(&mut backend, &stop, || now)
                .unwrap()
                .as_slice(),
            [Event::Detected(_)]
        ));
        assert!(matches!(
            monitor
                .poll(&mut backend, &stop, || now + TTL)
                .unwrap()
                .as_slice(),
            [Event::ObservedExpiry(_)]
        ));
        assert_eq!(backend.calls, 3);
        assert_eq!(monitor.deadline(), None);
    }

    #[test]
    fn final_change_cancels_and_read_failure_can_recover() {
        let mut monitor = Monitor::new().unwrap();
        let mut backend = reader(&[DUMMY, DUMMY, "通常文"]);
        let stop = AtomicBool::new(false);
        let now = Instant::now();
        monitor.poll(&mut backend, &stop, || now).unwrap();
        assert_eq!(
            monitor.poll(&mut backend, &stop, || now + TTL).unwrap(),
            vec![Event::Cancelled(CancelReason::ContentChanged)]
        );
        backend
            .inputs
            .push_back(ReadOutcome::Failed(ReadErrorKind::Occupied));
        assert_eq!(
            monitor.poll(&mut backend, &stop, || now + TTL).unwrap(),
            vec![Event::InputSkipped(SkipReason::ReadFailed(
                ReadErrorKind::Occupied
            ))]
        );
        backend.inputs.push_back(ReadOutcome::Text(DUMMY.into()));
        monitor.poll(&mut backend, &stop, || now + TTL).unwrap();
        assert_eq!(monitor.deadline(), Some(now + TTL + TTL));
    }

    #[test]
    fn samples_time_after_reads_and_waits_for_the_earlier_deadline() {
        let mut monitor = Monitor::new().unwrap();
        let mut backend = reader(&[DUMMY]);
        let stop = AtomicBool::new(false);
        let now = Instant::now();
        monitor
            .poll(&mut backend, &stop, || now + Duration::from_secs(2))
            .unwrap();
        assert_eq!(monitor.deadline(), Some(now + Duration::from_secs(2) + TTL));
        assert_eq!(wait_duration(now, None), POLL_INTERVAL);
        assert_eq!(
            wait_duration(now, Some(now + Duration::from_millis(100))),
            Duration::from_millis(100)
        );
        assert_eq!(wait_duration(now + TTL, Some(now)), Duration::ZERO);
    }

    #[test]
    fn stop_before_or_during_read_discards_input_and_skips_confirmation() {
        let now = Instant::now();
        let stop = AtomicBool::new(true);
        let mut monitor = Monitor::new().unwrap();
        let mut backend = reader(&[]);
        assert_eq!(
            monitor.poll(&mut backend, &stop, || now).unwrap(),
            vec![Event::Stopped]
        );
        assert_eq!(backend.calls, 0);
        let stop = AtomicBool::new(false);
        let mut monitor = Monitor::new().unwrap();
        let mut backend = reader(&[DUMMY]);
        monitor.poll(&mut backend, &stop, || now).unwrap();
        backend.inputs.push_back(ReadOutcome::Text(DUMMY.into()));
        backend.stop_during_read = Some(&stop);
        assert_eq!(
            monitor.poll(&mut backend, &stop, || now + TTL).unwrap(),
            vec![Event::Stopped]
        );
        assert_eq!(backend.calls, 2);
        assert_eq!(monitor.deadline(), None);
    }

    #[test]
    fn throttles_repeat_events_and_resets_target_notifications_on_change() {
        let now = Instant::now();
        let event = Event::Detected(DetectedRules {
            aws_access_key_id: true,
            private_key_header: false,
        });
        let mut logger = EventLogger::new(Language::English);
        let mut output = vec![];
        logger.emit(vec![event], now, &mut output).unwrap();
        let first = output.len();
        logger
            .emit(vec![event], now + Duration::from_secs(29), &mut output)
            .unwrap();
        assert_eq!(output.len(), first);
        logger
            .emit(vec![event], now + Duration::from_secs(30), &mut output)
            .unwrap();
        assert!(output.len() > first);
        let before = output.len();
        logger
            .emit(
                vec![Event::Cancelled(CancelReason::ContentChanged), event],
                now + Duration::from_secs(31),
                &mut output,
            )
            .unwrap();
        assert!(output.len() > before);
        assert!(!String::from_utf8(output).unwrap().contains(DUMMY));
    }

    #[test]
    fn logs_are_localized_and_output_failure_is_propagated() {
        let now = Instant::now();
        for (language, expected) in [
            (Language::Japanese, "上書き予定時刻"),
            (Language::English, "Replacement deadline"),
        ] {
            let mut output = vec![];
            let mut logger = EventLogger::new(language);
            logger
                .emit(
                    vec![Event::ObservedExpiry(DetectedRules::default())],
                    now,
                    &mut output,
                )
                .unwrap();
            assert!(String::from_utf8(output).unwrap().contains(expected));
        }
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        assert!(EventLogger::new(Language::Japanese)
            .emit(vec![Event::Stopped], now, &mut Broken)
            .is_err());
    }

    #[test]
    fn read_recovery_is_reported_once_and_new_failure_is_not_suppressed() {
        let now = Instant::now();
        let stop = AtomicBool::new(false);
        let mut monitor = Monitor::new().unwrap();
        let mut backend = reader(&[]);
        let failure = Event::InputSkipped(SkipReason::ReadFailed(ReadErrorKind::Occupied));
        let mut logger = EventLogger::new(Language::English);
        let mut output = vec![];
        backend
            .inputs
            .push_back(ReadOutcome::Failed(ReadErrorKind::Occupied));
        logger
            .emit(
                monitor.poll(&mut backend, &stop, || now).unwrap(),
                now,
                &mut output,
            )
            .unwrap();
        assert!(!monitor.take_recovery());
        backend.inputs.push_back(ReadOutcome::NonText);
        monitor.poll(&mut backend, &stop, || now).unwrap();
        assert!(monitor.take_recovery());
        assert!(!monitor.take_recovery());
        logger.recovered(&mut output).unwrap();
        let length = output.len();
        logger.emit(vec![failure], now, &mut output).unwrap();
        assert!(output.len() > length);
        assert!(String::from_utf8(output).unwrap().contains("[RECOVERED]"));
    }
}
