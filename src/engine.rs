//! 時刻と読取結果を受け取り、次の操作を返します。OS操作は行いません。
//! Accept time and read outcomes, then request actions; never perform OS I/O.

use std::fmt;
use std::time::Instant;

use crate::detector::{DetectedRules, Detector};
use crate::{MAX_TEXT_BYTES, REPLACEMENT_TEXT, TTL};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Observe,
    Apply,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadErrorKind {
    Occupied,
    Unavailable,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteErrorKind {
    Occupied,
    Unavailable,
    Other,
}

/// 生のOSエラーを格納しない入力。Debugは本文を表示しません。
/// No raw backend errors; Debug never reveals the text.
pub enum ReadOutcome {
    Text(String),
    Empty,
    NonText,
    Failed(ReadErrorKind),
}

impl fmt::Debug for ReadOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(_) => formatter.write_str("Text(<redacted>)"),
            Self::Empty => formatter.write_str("Empty"),
            Self::NonText => formatter.write_str("NonText"),
            Self::Failed(kind) => formatter.debug_tuple("Failed").field(kind).finish(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    Empty,
    NonText,
    ReadFailed(ReadErrorKind),
    Oversized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelReason {
    ContentChanged,
    Skipped(SkipReason),
}

/// ログに渡せる本文なしのイベント / Content-free events for a future logger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Detected(DetectedRules),
    Cancelled(CancelReason),
    InputSkipped(SkipReason),
    ObservedExpiry(DetectedRules),
    ReplacementSucceeded,
    ReplacementFailed(WriteErrorKind),
    Stopped,
}

/// 将来の単一スレッドの呼出側が、その場で一度だけ実行する操作要求です。
/// A future single-threaded caller executes each request immediately, once.
#[derive(Debug, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    None,
    ReadAgain,
    ReplaceClipboard(&'static str),
    ExitFailure,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Step {
    pub events: Vec<Event>,
    pub action: Action,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineState {
    Idle,
    Pending,
    AwaitingConfirmation,
    AwaitingWriteResult,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Observe,
    Confirm,
    ReportWrite,
}

/// 誤った呼出順序や時刻を拒否します / Reject invalid sequencing and time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineError {
    UnexpectedState {
        operation: Operation,
        state: EngineState,
    },
    TimeWentBackwards,
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedState { operation, state } => {
                write!(formatter, "呼出順序が不正です: {operation:?} / {state:?}")
            }
            Self::TimeWentBackwards => formatter.write_str("入力時刻が前回より前です"),
        }
    }
}

impl std::error::Error for EngineError {}

// 本文を持つ型にはDebugやCloneを導出しない。
// Do not derive Debug or Clone for types holding pending content.
struct Target {
    text: String,
    rules: DetectedRules,
    deadline: Instant,
}

enum State {
    Idle,
    Pending(Target),
    AwaitingConfirmation(Target),
    AwaitingWriteResult,
    Stopped,
}

enum Input {
    Text(String),
    Skipped(SkipReason),
}

impl From<ReadOutcome> for Input {
    fn from(outcome: ReadOutcome) -> Self {
        match outcome {
            ReadOutcome::Text(text) if text.is_empty() => Self::Skipped(SkipReason::Empty),
            ReadOutcome::Text(text) if text.len() > MAX_TEXT_BYTES => {
                Self::Skipped(SkipReason::Oversized)
            }
            ReadOutcome::Text(text) => Self::Text(text),
            ReadOutcome::Empty => Self::Skipped(SkipReason::Empty),
            ReadOutcome::NonText => Self::Skipped(SkipReason::NonText),
            ReadOutcome::Failed(kind) => Self::Skipped(SkipReason::ReadFailed(kind)),
        }
    }
}

/// 通常文・履歴は保持せず、対象全文を最大1つだけ保持します。
/// Retain at most one target, with no ordinary text or history.
pub struct Engine {
    detector: Detector,
    mode: Mode,
    state: State,
    last_time: Option<Instant>,
}

impl Engine {
    pub fn new(mode: Mode) -> Result<Self, regex::Error> {
        Ok(Self {
            detector: Detector::new()?,
            mode,
            state: State::Idle,
            last_time: None,
        })
    }

    pub fn state(&self) -> EngineState {
        match &self.state {
            State::Idle => EngineState::Idle,
            State::Pending(_) => EngineState::Pending,
            State::AwaitingConfirmation(_) => EngineState::AwaitingConfirmation,
            State::AwaitingWriteResult => EngineState::AwaitingWriteResult,
            State::Stopped => EngineState::Stopped,
        }
    }

    pub fn deadline(&self) -> Option<Instant> {
        match &self.state {
            State::Pending(target) | State::AwaitingConfirmation(target) => Some(target.deadline),
            _ => None,
        }
    }

    /// 起動時も通常の読取もここへ渡します。期限では再読取だけを要求します。
    /// Submit startup and polling reads here; expiry requests only another read.
    /// nowには読取完了後の時刻を渡します / Supply the time after the read completes.
    pub fn observe(&mut self, now: Instant, outcome: ReadOutcome) -> Result<Step, EngineError> {
        if !matches!(self.state, State::Idle | State::Pending(_)) {
            return Err(self.unexpected(Operation::Observe));
        }
        self.check_time(now)?;
        let input = Input::from(outcome);
        let previous = std::mem::replace(&mut self.state, State::Idle);
        let mut step = Step::default();

        if let State::Pending(target) = previous {
            if matches!(&input, Input::Text(text) if text == &target.text) {
                if now >= target.deadline {
                    self.state = State::AwaitingConfirmation(target);
                    step.action = Action::ReadAgain;
                } else {
                    self.state = State::Pending(target);
                }
                return Ok(step);
            }
            step.events.push(Event::Cancelled(cancel_reason(&input)));
            // 新しい対象を保持する前に、古い全文を解放する。
            // Release the previous target before retaining a new one.
            drop(target);
        }

        self.accept_input(now, input, &mut step);
        Ok(step)
    }

    /// ReadAgainの後に新しく読んだ結果だけを渡してください。
    /// Only submit a fresh read obtained after ReadAgain.
    /// nowには再読取完了後の時刻を渡します / Supply the time after the fresh read.
    /// 読取と書込の間の競合は、このコアでは原子的に防げません。
    /// This core cannot make backend read/write operations atomic.
    pub fn confirm(&mut self, now: Instant, outcome: ReadOutcome) -> Result<Step, EngineError> {
        if !matches!(self.state, State::AwaitingConfirmation(_)) {
            return Err(self.unexpected(Operation::Confirm));
        }
        self.check_time(now)?;
        let input = Input::from(outcome);
        let State::AwaitingConfirmation(target) = std::mem::replace(&mut self.state, State::Idle)
        else {
            unreachable!("state checked before transition");
        };

        let mut step = Step::default();
        if matches!(&input, Input::Text(text) if text == &target.text) {
            match self.mode {
                Mode::Observe => step.events.push(Event::ObservedExpiry(target.rules)),
                Mode::Apply => {
                    self.state = State::AwaitingWriteResult;
                    step.action = Action::ReplaceClipboard(REPLACEMENT_TEXT);
                }
            }
        } else {
            step.events.push(Event::Cancelled(cancel_reason(&input)));
            drop(target);
            self.accept_input(now, input, &mut step);
        }
        Ok(step)
    }

    /// 成功を先取りせず、呼出側の書込結果を受け取ります。
    /// Report actual backend completion; never assume write success.
    pub fn report_write(
        &mut self,
        result: Result<(), WriteErrorKind>,
    ) -> Result<Step, EngineError> {
        if !matches!(self.state, State::AwaitingWriteResult) {
            return Err(self.unexpected(Operation::ReportWrite));
        }
        let mut step = Step::default();
        match result {
            Ok(()) => {
                self.state = State::Idle;
                step.events.push(Event::ReplacementSucceeded);
            }
            Err(kind) => {
                self.state = State::Stopped;
                step.events.push(Event::ReplacementFailed(kind));
                step.action = Action::ExitFailure;
            }
        }
        Ok(step)
    }

    /// 停止時には書込を要求しません / Stop without requesting a final write.
    pub fn stop(&mut self) -> Step {
        if matches!(self.state, State::Stopped) {
            return Step::default();
        }
        self.state = State::Stopped;
        Step {
            events: vec![Event::Stopped],
            action: Action::None,
        }
    }

    fn accept_input(&mut self, now: Instant, input: Input, step: &mut Step) {
        match input {
            Input::Text(text) => {
                let rules = self.detector.detect(&text);
                if !rules.is_empty() {
                    self.state = State::Pending(Target {
                        text,
                        rules,
                        deadline: now + TTL,
                    });
                    step.events.push(Event::Detected(rules));
                }
            }
            Input::Skipped(reason) => {
                if matches!(reason, SkipReason::ReadFailed(_) | SkipReason::Oversized) {
                    step.events.push(Event::InputSkipped(reason));
                }
            }
        }
    }

    fn check_time(&mut self, now: Instant) -> Result<(), EngineError> {
        if self.last_time.is_some_and(|previous| now < previous) {
            return Err(EngineError::TimeWentBackwards);
        }
        self.last_time = Some(now);
        Ok(())
    }

    fn unexpected(&self, operation: Operation) -> EngineError {
        EngineError::UnexpectedState {
            operation,
            state: self.state(),
        }
    }
}

fn cancel_reason(input: &Input) -> CancelReason {
    match input {
        Input::Text(_) => CancelReason::ContentChanged,
        Input::Skipped(reason) => CancelReason::Skipped(*reason),
    }
}
