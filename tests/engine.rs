use std::time::{Duration, Instant};

use veil_clip::engine::{
    Action, CancelReason, Engine, EngineError, EngineState, Event, Mode, Operation, ReadErrorKind,
    ReadOutcome, SkipReason, Step, WriteErrorKind,
};
use veil_clip::{MAX_TEXT_BYTES, REPLACEMENT_TEXT, TTL};

const TARGET_A: &str = "AKIA0000000000000000";
const TARGET_C: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nsynthetic-body";

fn text(value: &str) -> ReadOutcome {
    ReadOutcome::Text(value.to_owned())
}

fn armed(mode: Mode, now: Instant) -> Engine {
    let mut engine = Engine::new(mode).unwrap();
    let step = engine.observe(now, text(TARGET_A)).unwrap();
    assert!(matches!(step.events.as_slice(), [Event::Detected(_)]));
    assert_eq!(step.action, Action::None);
    engine
}

fn awaiting_confirmation(mode: Mode, now: Instant) -> Engine {
    let mut engine = armed(mode, now);
    let step = engine.observe(now + TTL, text(TARGET_A)).unwrap();
    assert_eq!(step.action, Action::ReadAgain);
    assert!(step.events.is_empty());
    engine
}

#[test]
fn startup_only_arms_matching_text() {
    let now = Instant::now();
    let mut engine = Engine::new(Mode::default()).unwrap();
    for outcome in [
        text("通常の文章"),
        text(""),
        ReadOutcome::Empty,
        ReadOutcome::NonText,
    ] {
        assert_eq!(engine.observe(now, outcome).unwrap(), Step::default());
        assert_eq!(engine.state(), EngineState::Idle);
        assert_eq!(engine.deadline(), None);
    }
    engine.observe(now, text(TARGET_A)).unwrap();
    assert_eq!(engine.state(), EngineState::Pending);
    assert_eq!(engine.deadline(), Some(now + TTL));
}

#[test]
fn repeated_reads_do_not_extend_the_deadline() {
    let now = Instant::now();
    let mut engine = armed(Mode::Apply, now);
    for elapsed in [
        Duration::ZERO,
        Duration::from_secs(2),
        Duration::from_secs(4),
    ] {
        let step = engine.observe(now + elapsed, text(TARGET_A)).unwrap();
        assert_eq!(step, Step::default());
        assert_eq!(engine.deadline(), Some(now + TTL));
    }
}

#[test]
fn requests_a_fresh_read_only_at_or_after_the_deadline() {
    let now = Instant::now();
    for late_by in [Duration::ZERO, Duration::from_secs(10)] {
        let mut engine = armed(Mode::Apply, now);
        assert_eq!(
            engine
                .observe(now + TTL - Duration::from_nanos(1), text(TARGET_A))
                .unwrap(),
            Step::default()
        );
        let step = engine.observe(now + TTL + late_by, text(TARGET_A)).unwrap();
        assert_eq!(step.action, Action::ReadAgain);
        assert_eq!(engine.state(), EngineState::AwaitingConfirmation);
        assert!(!step.events.contains(&Event::ReplacementSucceeded));
    }
}

#[test]
fn ordinary_replacement_cancels_before_at_and_after_expiry() {
    let now = Instant::now();
    for elapsed in [Duration::from_secs(2), TTL, TTL + Duration::from_secs(1)] {
        let mut engine = armed(Mode::Apply, now);
        let step = engine.observe(now + elapsed, text("新しい通常文")).unwrap();
        assert_eq!(step.action, Action::None);
        assert_eq!(
            step.events,
            vec![Event::Cancelled(CancelReason::ContentChanged)]
        );
        assert_eq!(engine.state(), EngineState::Idle);
        assert_eq!(engine.deadline(), None);
    }
}

#[test]
fn a_new_secret_gets_its_own_full_ttl() {
    let now = Instant::now();
    let mut engine = armed(Mode::Apply, now);
    let changed_at = now + Duration::from_secs(2);
    let step = engine.observe(changed_at, text(TARGET_C)).unwrap();
    assert!(matches!(
        step.events.as_slice(),
        [
            Event::Cancelled(CancelReason::ContentChanged),
            Event::Detected(_)
        ]
    ));
    assert_eq!(engine.deadline(), Some(changed_at + TTL));
    assert_eq!(
        engine.observe(now + TTL, text(TARGET_C)).unwrap().action,
        Action::None
    );
    assert_eq!(
        engine
            .observe(changed_at + TTL, text(TARGET_C))
            .unwrap()
            .action,
        Action::ReadAgain
    );
}

#[test]
fn empty_nontext_and_failed_reads_cancel_pending_content() {
    let now = Instant::now();
    for (outcome, reason) in [
        (text(""), SkipReason::Empty),
        (ReadOutcome::Empty, SkipReason::Empty),
        (ReadOutcome::NonText, SkipReason::NonText),
        (
            ReadOutcome::Failed(ReadErrorKind::Occupied),
            SkipReason::ReadFailed(ReadErrorKind::Occupied),
        ),
        (
            ReadOutcome::Failed(ReadErrorKind::Unavailable),
            SkipReason::ReadFailed(ReadErrorKind::Unavailable),
        ),
        (
            ReadOutcome::Failed(ReadErrorKind::Other),
            SkipReason::ReadFailed(ReadErrorKind::Other),
        ),
    ] {
        let mut engine = armed(Mode::Apply, now);
        let step = engine
            .observe(now + Duration::from_secs(1), outcome)
            .unwrap();
        assert_eq!(step.action, Action::None);
        assert!(step
            .events
            .contains(&Event::Cancelled(CancelReason::Skipped(reason))));
        assert_eq!(engine.state(), EngineState::Idle);
        assert_eq!(engine.deadline(), None);
        let recovered_at = now + Duration::from_secs(2);
        engine.observe(recovered_at, text(TARGET_A)).unwrap();
        assert_eq!(engine.deadline(), Some(recovered_at + TTL));
    }
}

#[test]
fn changed_content_on_final_read_never_requests_a_write() {
    let now = Instant::now();
    for value in ["新しい通常文", TARGET_C] {
        let mut engine = awaiting_confirmation(Mode::Apply, now);
        let confirmed_at = now + TTL + Duration::from_millis(1);
        let step = engine.confirm(confirmed_at, text(value)).unwrap();
        assert_eq!(step.action, Action::None);
        assert_eq!(
            step.events[0],
            Event::Cancelled(CancelReason::ContentChanged)
        );
        if value == TARGET_C {
            assert_eq!(engine.deadline(), Some(confirmed_at + TTL));
            assert!(matches!(step.events[1], Event::Detected(_)));
        } else {
            assert_eq!(engine.state(), EngineState::Idle);
        }
    }
}

#[test]
fn failed_or_unscannable_final_reads_never_request_a_write() {
    let now = Instant::now();
    for outcome in [
        text(""),
        ReadOutcome::Empty,
        ReadOutcome::NonText,
        ReadOutcome::Failed(ReadErrorKind::Occupied),
        ReadOutcome::Text("x".repeat(MAX_TEXT_BYTES + 1)),
    ] {
        let mut engine = awaiting_confirmation(Mode::Apply, now);
        let step = engine.confirm(now + TTL, outcome).unwrap();
        assert_eq!(step.action, Action::None);
        assert_eq!(engine.state(), EngineState::Idle);
        assert_eq!(engine.deadline(), None);
        assert!(matches!(
            step.events[0],
            Event::Cancelled(CancelReason::Skipped(_))
        ));
    }
}

#[test]
fn write_request_requires_confirmation_and_completion_before_success() {
    let now = Instant::now();
    let mut engine = awaiting_confirmation(Mode::Apply, now);
    let step = engine.confirm(now + TTL, text(TARGET_A)).unwrap();
    assert_eq!(step.action, Action::ReplaceClipboard(REPLACEMENT_TEXT));
    assert!(step.events.is_empty());
    assert_eq!(engine.state(), EngineState::AwaitingWriteResult);
    assert_eq!(engine.deadline(), None);
    let completed = engine.report_write(Ok(())).unwrap();
    assert_eq!(completed.events, vec![Event::ReplacementSucceeded]);
    assert_eq!(completed.action, Action::None);
    assert_eq!(engine.state(), EngineState::Idle);
    assert_eq!(
        engine.observe(now + TTL, text(REPLACEMENT_TEXT)).unwrap(),
        Step::default()
    );
}

#[test]
fn rejects_out_of_order_and_duplicate_calls_without_changing_state() {
    let now = Instant::now();
    let mut engine = armed(Mode::Apply, now);
    assert_eq!(
        engine.confirm(now, text(TARGET_A)),
        Err(EngineError::UnexpectedState {
            operation: Operation::Confirm,
            state: EngineState::Pending
        })
    );
    assert!(engine.report_write(Ok(())).is_err());
    assert_eq!(engine.deadline(), Some(now + TTL));
    engine.observe(now + TTL, text(TARGET_A)).unwrap();
    assert!(engine.observe(now + TTL, text(TARGET_A)).is_err());
    assert_eq!(engine.state(), EngineState::AwaitingConfirmation);
    engine.confirm(now + TTL, text(TARGET_A)).unwrap();
    assert!(engine.confirm(now + TTL, text(TARGET_A)).is_err());
    assert_eq!(engine.state(), EngineState::AwaitingWriteResult);
    engine.report_write(Ok(())).unwrap();
    assert!(engine.report_write(Ok(())).is_err());
    assert_eq!(engine.state(), EngineState::Idle);
}

#[test]
fn rejects_backwards_time_without_expiring_or_resetting_the_target() {
    let now = Instant::now();
    let mut engine = armed(Mode::Apply, now);
    engine
        .observe(now + Duration::from_secs(2), text(TARGET_A))
        .unwrap();
    assert_eq!(
        engine.observe(now, text("通常文")),
        Err(EngineError::TimeWentBackwards)
    );
    assert_eq!(engine.deadline(), Some(now + TTL));
    engine.observe(now + TTL, text(TARGET_A)).unwrap();
    assert_eq!(
        engine.confirm(now, text(TARGET_A)),
        Err(EngineError::TimeWentBackwards)
    );
    assert_eq!(engine.state(), EngineState::AwaitingConfirmation);
    assert_eq!(
        engine.confirm(now + TTL, text(TARGET_A)).unwrap().action,
        Action::ReplaceClipboard(REPLACEMENT_TEXT)
    );
}

#[test]
fn stopping_releases_pending_state_and_requests_no_write() {
    let now = Instant::now();
    let idle = Engine::new(Mode::Apply).unwrap();
    let pending = armed(Mode::Apply, now);
    let confirming = awaiting_confirmation(Mode::Apply, now);
    for mut engine in [idle, pending, confirming] {
        let step = engine.stop();
        assert_eq!(step.action, Action::None);
        assert_eq!(step.events, vec![Event::Stopped]);
        assert_eq!(engine.state(), EngineState::Stopped);
        assert_eq!(engine.deadline(), None);
        assert_eq!(engine.stop(), Step::default());
        assert!(engine.observe(now + TTL, text(TARGET_A)).is_err());
        assert!(engine.confirm(now + TTL, text(TARGET_A)).is_err());
    }
}

fn sized_japanese_target(bytes: usize) -> String {
    let remaining = bytes - TARGET_A.len() - 1;
    format!(
        "{TARGET_A} {}{}",
        "鍵".repeat(remaining / 3),
        " ".repeat(remaining % 3)
    )
}

#[test]
fn scan_limit_counts_utf8_bytes_and_cancels_oversized_targets() {
    let now = Instant::now();
    for bytes in [MAX_TEXT_BYTES - 1, MAX_TEXT_BYTES, MAX_TEXT_BYTES + 1] {
        let mut engine = Engine::new(Mode::Apply).unwrap();
        let value = sized_japanese_target(bytes);
        assert_eq!(value.len(), bytes);
        assert!(value.chars().count() < MAX_TEXT_BYTES);
        let step = engine.observe(now, ReadOutcome::Text(value)).unwrap();
        assert_eq!(step.action, Action::None);
        if bytes <= MAX_TEXT_BYTES {
            assert_eq!(engine.state(), EngineState::Pending);
            assert!(matches!(step.events.as_slice(), [Event::Detected(_)]));
        } else {
            assert_eq!(engine.state(), EngineState::Idle);
            assert_eq!(
                step.events,
                vec![Event::InputSkipped(SkipReason::Oversized)]
            );
        }
    }
    let mut engine = armed(Mode::Apply, now);
    let step = engine
        .observe(
            now + Duration::from_secs(1),
            ReadOutcome::Text(sized_japanese_target(MAX_TEXT_BYTES + 1)),
        )
        .unwrap();
    assert_eq!(engine.state(), EngineState::Idle);
    assert_eq!(engine.deadline(), None);
    assert!(step
        .events
        .contains(&Event::Cancelled(CancelReason::Skipped(
            SkipReason::Oversized
        ))));
}

#[test]
fn debug_output_does_not_include_clipboard_text() {
    let outcome = text(TARGET_C);
    assert_eq!(format!("{outcome:?}"), "Text(<redacted>)");
    let now = Instant::now();
    let mut engine = Engine::new(Mode::Apply).unwrap();
    let step = engine.observe(now, outcome).unwrap();
    assert!(!format!("{step:?}").contains(TARGET_C));
}

// 将来のOS呼出側と同じ順序で操作要求を消費する、メモリだけのモック。
// In-memory mock consuming requests in the order required of a future backend.
struct FakeClipboard {
    content: &'static str,
    writes: Vec<&'static str>,
    write_result: Result<(), WriteErrorKind>,
    confirmation_override: Option<ReadOutcome>,
}

impl FakeClipboard {
    fn new(content: &'static str) -> Self {
        Self {
            content,
            writes: vec![],
            write_result: Ok(()),
            confirmation_override: None,
        }
    }

    fn tick(&mut self, engine: &mut Engine, now: Instant) -> Result<Vec<Event>, EngineError> {
        let mut step = engine.observe(now, text(self.content))?;
        if step.action == Action::ReadAgain {
            let fresh_read = self
                .confirmation_override
                .take()
                .unwrap_or_else(|| text(self.content));
            let mut confirmed = engine.confirm(now, fresh_read)?;
            step.events.append(&mut confirmed.events);
            step.action = confirmed.action;
        }
        if let Action::ReplaceClipboard(replacement) = step.action {
            self.writes.push(replacement);
            if self.write_result.is_ok() {
                self.content = replacement;
            }
            step.events
                .append(&mut engine.report_write(self.write_result)?.events);
        }
        Ok(step.events)
    }
}

#[test]
fn observation_mode_never_writes_even_across_repeated_expirations() {
    let mut engine = Engine::new(Mode::default()).unwrap();
    let mut clipboard = FakeClipboard::new(TARGET_A);
    let now = Instant::now();
    let mut expirations = 0;
    for seconds in 0..=20 {
        let events = clipboard
            .tick(&mut engine, now + Duration::from_secs(seconds))
            .unwrap();
        expirations += events
            .iter()
            .filter(|event| matches!(event, Event::ObservedExpiry(_)))
            .count();
    }
    assert!(expirations > 1);
    assert!(clipboard.writes.is_empty());
    assert_eq!(clipboard.content, TARGET_A);
}

#[test]
fn apply_mode_writes_the_fixed_marker_once_and_does_not_rearm_it() {
    let mut engine = Engine::new(Mode::Apply).unwrap();
    let mut clipboard = FakeClipboard::new(TARGET_A);
    let now = Instant::now();
    clipboard.tick(&mut engine, now).unwrap();
    let events = clipboard.tick(&mut engine, now + TTL).unwrap();
    assert_eq!(events, vec![Event::ReplacementSucceeded]);
    clipboard.tick(&mut engine, now + TTL + TTL).unwrap();
    assert_eq!(clipboard.writes, vec![REPLACEMENT_TEXT]);
    assert_eq!(clipboard.content, REPLACEMENT_TEXT);
}

#[test]
fn mock_backend_change_between_poll_and_confirmation_prevents_the_write() {
    let mut engine = Engine::new(Mode::Apply).unwrap();
    let mut clipboard = FakeClipboard::new(TARGET_A);
    let now = Instant::now();
    clipboard.tick(&mut engine, now).unwrap();
    clipboard.confirmation_override = Some(text("新しくコピーした文章"));
    let events = clipboard.tick(&mut engine, now + TTL).unwrap();
    assert_eq!(events, vec![Event::Cancelled(CancelReason::ContentChanged)]);
    assert!(clipboard.writes.is_empty());
    assert_eq!(engine.state(), EngineState::Idle);
}

#[test]
fn mock_write_failure_stops_without_success_or_retry() {
    let now = Instant::now();
    for kind in [
        WriteErrorKind::Occupied,
        WriteErrorKind::Unavailable,
        WriteErrorKind::Other,
    ] {
        let mut engine = Engine::new(Mode::Apply).unwrap();
        let mut clipboard = FakeClipboard::new(TARGET_A);
        clipboard.write_result = Err(kind);
        clipboard.tick(&mut engine, now).unwrap();
        let events = clipboard.tick(&mut engine, now + TTL).unwrap();
        assert_eq!(events, vec![Event::ReplacementFailed(kind)]);
        assert_eq!(engine.state(), EngineState::Stopped);
        assert_eq!(engine.deadline(), None);
        assert!(clipboard.tick(&mut engine, now + TTL + TTL).is_err());
        assert_eq!(clipboard.writes, vec![REPLACEMENT_TEXT]);
        assert_eq!(clipboard.content, TARGET_A);
        assert!(engine.report_write(Ok(())).is_err());
    }
}

#[test]
fn write_failure_requests_nonzero_exit() {
    let now = Instant::now();
    let mut engine = awaiting_confirmation(Mode::Apply, now);
    engine.confirm(now + TTL, text(TARGET_A)).unwrap();
    let step = engine.report_write(Err(WriteErrorKind::Other)).unwrap();
    assert_eq!(step.action, Action::ExitFailure);
    assert!(!step.events.contains(&Event::ReplacementSucceeded));
}
